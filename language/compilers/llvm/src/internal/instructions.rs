use inkwell::builder::Builder;
use inkwell::{AddressSpace, IntPredicate};
use inkwell::types::{BasicType, BasicTypeEnum};
use inkwell::values::{BasicMetadataValueEnum, BasicValue, BasicValueEnum, FunctionValue, PointerValue};
use crate::compiler::CompilerImpl;
use crate::internal::intrinsics::compile_llvm_intrinsics;
use crate::type_getter::CompilerTypeGetter;

pub fn compile_internal<'ctx>(type_getter: &CompilerTypeGetter<'ctx>, compiler: &CompilerImpl<'ctx>, name: &String, value: FunctionValue<'ctx>) {
    let block = compiler.context.append_basic_block(value, "0");
    compiler.builder.position_at_end(block);
    let params = value.get_params();
    if name.starts_with("numbers::Cast") {
        build_cast(value.get_params().get(0).unwrap(), value.get_type().get_return_type().unwrap(), compiler);
        return;
    } else if name.starts_with("math::Add") {
        let pointer_type = params.get(0).unwrap().into_pointer_value();
        let malloc = malloc_type(type_getter, pointer_type.get_type().const_zero(), &mut 0);

        let returning = compiler.builder.build_int_add(compiler.builder.build_load(pointer_type, "2").into_int_value(),
                                                       compiler.builder.build_load(params.get(1).unwrap().into_pointer_value(), "3").into_int_value(), "1");
        compiler.builder.build_store(malloc, returning);
        compiler.builder.build_return(Some(&malloc));
    } else if name.starts_with("math::Subtract") {
        let pointer_type = params.get(0).unwrap().into_pointer_value();
        let malloc = malloc_type(type_getter, pointer_type.get_type().const_zero(), &mut 0);
        let returning = compiler.builder.build_int_sub(compiler.builder.build_load(params.get(0).unwrap().into_pointer_value(), "2").into_int_value(),
                                                       compiler.builder.build_load(params.get(1).unwrap().into_pointer_value(), "3").into_int_value(), "1");
        compiler.builder.build_store(malloc, returning);
        compiler.builder.build_return(Some(&malloc));
    } else if name.starts_with("math::Multiply") {
        let pointer_type = params.get(0).unwrap().into_pointer_value();
        let malloc = malloc_type(type_getter, pointer_type.get_type().const_zero(), &mut 0);
        let returning = compiler.builder.build_int_mul(compiler.builder.build_load(params.get(0).unwrap().into_pointer_value(), "2").into_int_value(),
                                                       compiler.builder.build_load(params.get(1).unwrap().into_pointer_value(), "3").into_int_value(), "1");
        compiler.builder.build_store(malloc, returning);
        compiler.builder.build_return(Some(&malloc));
    } else if name.starts_with("math::Divide") {
        let pointer_type = params.get(0).unwrap().into_pointer_value();
        let malloc = malloc_type(type_getter, pointer_type.get_type().const_zero(), &mut 0);
        let returning = if name.ends_with("u64") {
            compiler.builder.build_int_unsigned_div(compiler.builder.build_load(params.get(0).unwrap().into_pointer_value(), "2").into_int_value(),
                                                    compiler.builder.build_load(params.get(1).unwrap().into_pointer_value(), "3").into_int_value(), "1")
        } else {
            compiler.builder.build_int_signed_div(compiler.builder.build_load(params.get(0).unwrap().into_pointer_value(), "2").into_int_value(),
                                                  compiler.builder.build_load(params.get(1).unwrap().into_pointer_value(), "3").into_int_value(), "1")
        };
        compiler.builder.build_store(malloc, returning);
        compiler.builder.build_return(Some(&malloc));
    } else if name.starts_with("math::Remainder") {
        let pointer_type = params.get(0).unwrap().into_pointer_value();
        let malloc = malloc_type(type_getter, pointer_type.get_type().const_zero(), &mut 0);
        let returning = if name.ends_with("u64") {
            compiler.builder.build_int_unsigned_rem(compiler.builder.build_load(params.get(0).unwrap().into_pointer_value(), "2").into_int_value(),
                                                    compiler.builder.build_load(params.get(1).unwrap().into_pointer_value(), "3").into_int_value(), "1")
        } else {
            compiler.builder.build_int_signed_rem(compiler.builder.build_load(params.get(0).unwrap().into_pointer_value(), "2").into_int_value(),
                                                  compiler.builder.build_load(params.get(1).unwrap().into_pointer_value(), "3").into_int_value(), "1")
        };
        compiler.builder.build_store(malloc, returning);
        compiler.builder.build_return(Some(&malloc));
    } else if name.starts_with("math::Equal") {
        let malloc = malloc_type(type_getter,
                                 type_getter.compiler.context.bool_type().ptr_type(AddressSpace::default()).const_zero(), &mut 0);
        let returning = compiler.builder
            .build_int_compare(IntPredicate::EQ, compiler.builder.build_load(params.get(0).unwrap().into_pointer_value(), "2").into_int_value(),
                               compiler.builder.build_load(params.get(1).unwrap().into_pointer_value(), "3").into_int_value(), "1");
        compiler.builder.build_store(malloc, returning);
        compiler.builder.build_return(Some(&malloc));
    } else if name.starts_with("array::Index") {
        let offset = get_loaded(&compiler.builder, params.get(1).unwrap()).into_int_value();
        let offset = compiler.builder.build_int_add(offset, compiler.context.i64_type().const_int(1, false), "3");

        let gep;
        unsafe {
            gep = compiler.builder
                .build_in_bounds_gep(params.get(0).unwrap().into_pointer_value(),
                                     &[offset], "1");
        }

        let gep = compiler.builder.build_load(gep, "2");
        compiler.builder.build_return(Some(&gep));
    } else if name.starts_with("array::Empty") {
        let size = unsafe {
            type_getter.compiler.builder.build_gep(value.get_type().get_return_type().unwrap()
                                                       .ptr_type(AddressSpace::default()).const_zero(),
                                                   &[type_getter.compiler.context.i64_type()
                                                       .const_int(1, false)], "0")
        };

        let malloc = compiler.builder.build_call(compiler.module.get_function("malloc")
                                                     .unwrap_or(compile_llvm_intrinsics("malloc", type_getter)),
                                                 &[BasicMetadataValueEnum::PointerValue(size)], "1")
            .try_as_basic_value().unwrap_left().into_pointer_value();

        compiler.builder.build_store(malloc, compiler.context.i64_type().const_zero());
        compiler.builder.build_return(Some(&malloc.as_basic_value_enum()));
    } else if name.starts_with("math::Not") {
        let malloc = malloc_type(type_getter,
                                 type_getter.compiler.context.bool_type().ptr_type(AddressSpace::default()).const_zero(), &mut 0);
        let returning = compiler.builder
            .build_not(compiler.builder.build_load(params.get(0).unwrap().into_pointer_value(), "1").into_int_value(), "0");
        compiler.builder.build_store(malloc, returning);
        compiler.builder.build_return(Some(&malloc));
    } else if name.starts_with("math::BitXOR") {
        let pointer_type = params.get(0).unwrap().into_pointer_value();
        let malloc = malloc_type(type_getter, pointer_type.get_type().const_zero(), &mut 0);

        let returning = compiler.builder.build_xor(compiler.builder.build_load(pointer_type, "2").into_int_value(),
                                                   compiler.builder.build_load(params.get(1).unwrap().into_pointer_value(), "3").into_int_value(), "1");
        compiler.builder.build_store(malloc, returning);
        compiler.builder.build_return(Some(&malloc));
    }  else if name.starts_with("math::BitOr") {
        let pointer_type = params.get(0).unwrap().into_pointer_value();
        let malloc = malloc_type(type_getter, pointer_type.get_type().const_zero(), &mut 0);

        let returning = compiler.builder.build_or(compiler.builder.build_load(pointer_type, "2").into_int_value(),
                                                   compiler.builder.build_load(params.get(1).unwrap().into_pointer_value(), "3").into_int_value(), "1");
        compiler.builder.build_store(malloc, returning);
        compiler.builder.build_return(Some(&malloc));
    }   else if name.starts_with("math::BitAnd") {
        let pointer_type = params.get(0).unwrap().into_pointer_value();
        let malloc = malloc_type(type_getter, pointer_type.get_type().const_zero(), &mut 0);

        let returning = compiler.builder.build_and(compiler.builder.build_load(pointer_type, "2").into_int_value(),
                                                   compiler.builder.build_load(params.get(1).unwrap().into_pointer_value(), "3").into_int_value(), "1");
        compiler.builder.build_store(malloc, returning);
        compiler.builder.build_return(Some(&malloc));
    } else {
        panic!("Unknown internal operation: {}", name)
    }
}

pub fn malloc_type<'a>(type_getter: &CompilerTypeGetter<'a>, pointer_type: PointerValue<'a>, id: &mut u64) -> PointerValue<'a> {
    let size = unsafe {
        type_getter.compiler.builder.build_gep(pointer_type,
                                               &[type_getter.compiler.context.i64_type().const_int(1, false)], &id.to_string())
    };
    *id += 1;
    let size = type_getter.compiler.builder.build_bitcast(size,
                                                          type_getter.compiler.context.i64_type().ptr_type(AddressSpace::default()), &id.to_string()).into_pointer_value();
    *id += 1;

    let malloc = type_getter.compiler.builder.build_call(type_getter.compiler.module.get_function("malloc")
                                                             .unwrap_or(compile_llvm_intrinsics("malloc", type_getter)),
                                                         &[BasicMetadataValueEnum::PointerValue(size)], &id.to_string()).try_as_basic_value().unwrap_left().into_pointer_value();
    *id += 1;
    let malloc = type_getter.compiler.builder.build_bitcast(malloc.as_basic_value_enum(), pointer_type.as_basic_value_enum().get_type(), &id.to_string());
    *id += 1;
    return malloc.into_pointer_value();
}

fn get_loaded<'ctx>(compiler: &Builder<'ctx>, value: &BasicValueEnum<'ctx>) -> BasicValueEnum<'ctx> {
    if value.is_pointer_value() {
        return compiler.build_load(value.into_pointer_value(), "0");
    }
    return value.clone();
}

fn build_cast(first: &BasicValueEnum, _second: BasicTypeEnum, compiler: &CompilerImpl) {
    //TODO float casting
    compiler.builder.build_return(Some(&compiler.builder.build_load(first.into_pointer_value(), "1")));
}