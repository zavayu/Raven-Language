use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use inkwell::AddressSpace;
use inkwell::basic_block::BasicBlock;
use inkwell::types::{BasicType, BasicTypeEnum};
use inkwell::values::{BasicValueEnum, FunctionValue};
use compilers::compiling::UnsafeFn;
use syntax::function::Function;
use syntax::ParsingError;
use syntax::syntax::Syntax;
use syntax::types::Types;
use crate::compiler::CompilerImpl;
use crate::function_compiler::{compile_block, instance_function, instance_struct};
use crate::internal::structs::get_internal_struct;

pub struct CompilerTypeGetter<'ctx> {
    pub syntax: Arc<Mutex<Syntax>>,
    pub compiler: Rc<CompilerImpl<'ctx>>,
    pub compiling: Rc<Vec<(FunctionValue<'ctx>, Arc<Function>)>>,
    pub blocks: HashMap<String, BasicBlock<'ctx>>,
    pub variables: HashMap<String, BasicValueEnum<'ctx>>,
}

impl<'ctx> CompilerTypeGetter<'ctx> {
    pub fn new(compiler: Rc<CompilerImpl<'ctx>>, syntax: Arc<Mutex<Syntax>>) -> Self {
        return Self {
            compiler,
            syntax,
            compiling: Rc::new(Vec::new()),
            blocks: HashMap::new(),
            variables: HashMap::new(),
        };
    }

    pub fn for_function(&self, function: Function, llvm_function: FunctionValue<'ctx>) -> Self {
        let mut variables = self.variables.clone();
        let offset = function.fields.len() != llvm_function.count_params() as usize;
        for i in 0..llvm_function.count_params() as usize {
            variables.insert(function.fields.get(i + offset as usize).unwrap().field.name.clone(),
                             llvm_function.get_nth_param(i as u32).unwrap());
        }
        return Self {
            syntax: self.syntax.clone(),
            compiler: self.compiler.clone(),
            compiling: self.compiling.clone(),
            blocks: self.blocks.clone(),
            variables,
        };
    }

    pub fn get_function(&mut self, function: &Arc<Function>) -> FunctionValue<'ctx> {
        match self.compiler.module.get_function(&function.name) {
            Some(found) => found,
            None => {
                instance_function(function.clone(), self)
            }
        }
    }

    pub fn get_type(&mut self, types: &Types) -> BasicTypeEnum<'ctx> {
        let found = match self.compiler.module.get_struct_type(&types.name()) {
            Some(found) => found.as_basic_type_enum(),
            None => get_internal_struct(self.compiler.context, &types.name()).unwrap_or(
                instance_struct(types.clone_struct(), self)
                    .as_basic_type_enum())
        }.as_basic_type_enum();
        return match types {
            Types::Struct(_) => found,
            Types::Reference(_) => found.ptr_type(AddressSpace::default()).as_basic_type_enum(),
            Types::Generic(_, _) => panic!("Can't compile a generic!")
        };
    }

    pub fn compile<T, A>(&mut self) -> Result<UnsafeFn<T, A>, Vec<ParsingError>> {
        let locked = self.syntax.lock().unwrap();
        let function = match locked.functions.get("main::main") {
            Some(main) => main,
            None => return Err(vec!(ParsingError::new((0, 0), 0, (0, 0), 0, "No main!".to_string())))
        }.clone();
        drop(locked);

        instance_function(function, self);

        while !self.compiling.is_empty() {
            let (function_type, function) = unsafe {
                Rc::get_mut_unchecked(&mut self.compiling)
            }.remove(0);
            compile_block(&function.code, function_type, self, &mut 0);
        }

        return Ok(self.compiler.get_main().unwrap());
    }
}