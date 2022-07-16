use thiserror::Error;
use std::convert::From;
use inkwell::OptimizationLevel;
use inkwell::AddressSpace;
use inkwell::context::Context;
// use inkwell::execution_engine::JitFunction;
use inkwell::execution_engine::{ExecutionEngine, JitFunction};
use inkwell::targets::{InitializationConfig, Target};
use inkwell::IntPredicate;
// use inkwell::values::{FunctionValue, PointerValue, PhiValue, IntValue, BasicValue};
use inkwell::values::{IntValue, PointerValue};
// use inkwell::types::{IntType, };//PointerType};
// use inkwell::basic_block::BasicBlock;
use inkwell::builder::Builder;
use inkwell::module::Module;
use crate::code::{EvmOp, IndexedEvmCode};
use crate::constants::{EVM_STACK_SIZE};


// TODO: this currently assumes that usize (on host) = i64_type (within LLVM)
pub type JitEvmCompiledContract = unsafe extern "C" fn(usize) -> u64;   // TODO TODO TODO


#[derive(Error, Debug)]
pub enum JitEvmEngineError {
    #[error("FunctionLookupError: {0:?}")]
    FunctionLookupError(#[from] inkwell::execution_engine::FunctionLookupError),
    #[error("LlvmStringError: {0:?}")]
    UnknownLlvmStringError(#[from] inkwell::support::LLVMString),
    #[error("StringError: {0:?}")]
    UnknownStringError(String),
}

impl From<String> for JitEvmEngineError {
    fn from(e: String) -> Self {
        Self::UnknownStringError(e)
    }
}


pub struct JitEvmEngine<'ctx> {
    pub context: &'ctx Context,
    pub module: Module<'ctx>,
    pub builder: Builder<'ctx>,
    pub execution_engine: ExecutionEngine<'ctx>,
}

impl<'ctx> JitEvmEngine<'ctx> {
    // TODO: THIS NEEDS FIXING

    // pub fn new() -> Result<Self, JitEvmEngineError> {
    //     Target::initialize_native(&InitializationConfig::default())?;

    //     let context = Context::create();
    //     let module = context.create_module("jitevm");
    //     let builder = context.create_builder();
    //     let execution_engine = module.create_jit_execution_engine(OptimizationLevel::Aggressive)?;

    //     // Ok(Self { context: &context, module, builder, execution_engine })
    //     Ok(Self { context: context, module, builder, execution_engine })
    // }


    // TODO: TOY EXAMPLE TO FIX IT WITH

    // pub struct Foo<'a> {
    //     pub string: Vec<u8>,
    //     pub view: &'a [u8],
    // }

    // impl<'a> Foo<'a> {
    //     pub fn new() -> Self {
    //         let string = vec![1,2,3,4,5];
    //         let view = &string[2..4];
    //         Self { string: string, view: view }
    //     }
    // }


    // TODO: STOP GAP INITIALIZATION METHOD

    pub fn new_from_context(context: &'ctx Context) -> Result<Self, JitEvmEngineError> {
        Target::initialize_native(&InitializationConfig::default())?;
        let module = context.create_module("jitevm");
        let builder = context.create_builder();
        let execution_engine = module.create_jit_execution_engine(OptimizationLevel::Aggressive)?;
        Ok(Self { context: &context, module, builder, execution_engine })
    }


    // HELPER FUNCTIONS

    fn build_stack_push<'a>(
        &'a self,
        inner_context_sp: PointerValue<'a>,
        inner_context_sp_offset: IntValue<'a>,
        val: IntValue<'a>)
    {
        let i64_type = self.context.i64_type();
        let sp_int = self.builder.build_load(inner_context_sp, "").into_int_value();
        let sp_ptr = self.builder.build_int_to_ptr(sp_int, i64_type.ptr_type(AddressSpace::Generic), "");
        self.builder.build_store(sp_ptr, val);
        self.builder.build_store(inner_context_sp, self.builder.build_int_add(sp_int, inner_context_sp_offset, ""));
    }
    
    fn build_stack_pop<'a>(
        &'a self,
        inner_context_sp: PointerValue<'a>,
        inner_context_sp_offset: IntValue<'a>) -> IntValue<'a>
    {
        let i64_type = self.context.i64_type();
        let sp_int = self.builder.build_load(inner_context_sp, "").into_int_value();
        let sp_int = self.builder.build_int_sub(sp_int, inner_context_sp_offset, "");
        self.builder.build_store(inner_context_sp, sp_int);
        let sp_ptr = self.builder.build_int_to_ptr(sp_int, i64_type.ptr_type(AddressSpace::Generic), "");
        let val = self.builder.build_load(sp_ptr, "").into_int_value();
        val
    }
    
    fn build_stack_write<'a>(
        &'a self,
        inner_context_sp: PointerValue<'a>,
        idx: IntValue<'a>,
        val: IntValue<'a>)
    {
        let i64_type = self.context.i64_type();
        let sp_int = self.builder.build_load(inner_context_sp, "").into_int_value();
        let sp_int = self.builder.build_int_sub(sp_int, idx, "");
        let sp_ptr = self.builder.build_int_to_ptr(sp_int, i64_type.ptr_type(AddressSpace::Generic), "");
        self.builder.build_store(sp_ptr, val);
    }
    
    fn build_stack_read<'a>(
        &'a self,
        inner_context_sp: PointerValue<'a>,
        idx: IntValue<'a>) -> IntValue<'a>
    {
        let i64_type = self.context.i64_type();
        let sp_int = self.builder.build_load(inner_context_sp, "").into_int_value();
        let sp_int = self.builder.build_int_sub(sp_int, idx, "");
        let sp_ptr = self.builder.build_int_to_ptr(sp_int, i64_type.ptr_type(AddressSpace::Generic), "");
        let val = self.builder.build_load(sp_ptr, "").into_int_value();
        val
    }


    pub fn jit_compile_contract(&self, code: &IndexedEvmCode) -> Result<JitFunction<JitEvmCompiledContract>, JitEvmEngineError> {
        // let void_type = self.context.void_type();
        let i64_type = self.context.i64_type();


        // //  Install our global callback into the system <------ later! code fragment from github repo above, will be useful to integrate with "outer" context of EVM
        // let i1_type = context.custom_width_int_type(1);
        // let cb_type = i1_type.fn_type(
        //     &[i64_type.array_type(6).ptr_type(AddressSpace::Generic).into()], false);
        // let cb_func = module.add_function("cb", cb_type, None);
        // execution_engine.add_global_mapping(&cb_func, callback as usize);


        let executecontract_fn_type = i64_type.fn_type(&[i64_type.into()], false);
        let function = self.module.add_function("executecontract", executecontract_fn_type, None);


        // SETUP

        let block_setup = self.context.append_basic_block(function, "setup");
        self.builder.position_at_end(block_setup);

        // let inner_context_stack = self.builder.build_alloca(i64_type.array_type(1024), "stack");
        let inner_context_stack = self.builder.build_int_to_ptr(function.get_nth_param(0).unwrap().into_int_value(), i64_type.ptr_type(AddressSpace::Generic), "");
        let inner_context_sp = self.builder.build_alloca(i64_type, "sp");
        let inner_context_sp_offset = i64_type.const_int(8, false);   // stack elements are 8 bytes for now
        self.builder.build_store(inner_context_sp, self.builder.build_ptr_to_int(inner_context_stack, i64_type, ""));


        // fn jit_compile_sum(&self) -> Option<JitFunction<SumFunc>> {
    //     let i64_type = self.context.i64_type();
    //     let fn_type = i64_type.fn_type(&[i64_type.into(), i64_type.into(), i64_type.into()], false);
    //     let function = self.module.add_function("sum", fn_type, None);
    //     let basic_block = self.context.append_basic_block(function, "entry");

    //     self.builder.position_at_end(basic_block);

    //     let x = function.get_nth_param(0)?.into_int_value();
    //     let y = function.get_nth_param(1)?.into_int_value();
    //     let z = function.get_nth_param(2)?.into_int_value();

    //     let sum = self.builder.build_int_add(x, y, "sum");
    //     let sum = self.builder.build_int_add(sum, z, "sum");

    //     self.builder.build_return(Some(&sum));

    //     unsafe { self.execution_engine.get_function("sum").ok() }
    // }


        // INSTRUCTIONS

        assert!(code.code.ops.len() > 0);

        let mut block_instructions = Vec::new();
        for i in 0..code.code.ops.len() {
            block_instructions.push(self.context.insert_basic_block_after(if i == 0 { block_setup } else { block_instructions[i-1] }, &format!("instruction #{}: {:?}", i, code.code.ops[i])));
        }

        self.builder.position_at_end(block_setup);
        self.builder.build_unconditional_branch(block_instructions[0]);


        // ERROR HANDLER

        let block_error = self.context.append_basic_block(function, "error");
        self.builder.position_at_end(block_error);

        let val = i64_type.const_int(u64::MAX, false);
        self.builder.build_return(Some(&val));


        // RENDER INSTRUCTIONS

        for (i, op) in code.code.ops.iter().enumerate() {
            use EvmOp::*;
            self.builder.position_at_end(block_instructions[i]);

            match op {
                Stop => {
                    let val = self.build_stack_pop(inner_context_sp, inner_context_sp_offset);
                    self.builder.build_return(Some(&val));
                },
                Push(_, val) => {
                    let val = i64_type.const_int(val.as_u64(), false);
                    self.build_stack_push(inner_context_sp, inner_context_sp_offset, val);
                    self.builder.build_unconditional_branch(block_instructions[i+1]);
                },
                Pop => {
                    self.build_stack_pop(inner_context_sp, inner_context_sp_offset);
                    self.builder.build_unconditional_branch(block_instructions[i+1]);
                },
                Jumpdest => {
                    self.builder.build_unconditional_branch(block_instructions[i+1]);
                },
                Jump => {
                    let target = self.build_stack_pop(inner_context_sp, inner_context_sp_offset);

                    if code.jumpdests.is_empty() {
                        // there are no valid jump targets, this Jump has to fail!
                        self.builder.build_unconditional_branch(block_error);

                    } else {
                        let mut jump_table = Vec::new();
                        for (j, jmp_i) in code.jumpdests.iter().enumerate() {
                            let jmp_target = code.opidx2target[jmp_i];
                            jump_table.push(self.context.insert_basic_block_after(if j == 0 { block_instructions[i] } else { jump_table[j-1] }, &format!("instruction #{}: {:?} / to Jumpdest #{} at op #{} to byte #{}", i, code.code.ops[i], j, jmp_i, jmp_target)));
                        }

                        self.builder.build_unconditional_branch(jump_table[0]);

                        for (j, jmp_i) in code.jumpdests.iter().enumerate() {
                            let jmp_target = code.opidx2target[jmp_i];
                            let jmp_target = jmp_target.as_u64();
                            self.builder.position_at_end(jump_table[j]);
                            let cmp = self.builder.build_int_compare(IntPredicate::EQ, i64_type.const_int(jmp_target, false), target, "");
                            self.builder.build_conditional_branch(cmp, block_instructions[*jmp_i], if j+1 == code.jumpdests.len() { block_error } else { jump_table[j+1] });
                        }
                    }
                },
                Jumpi => {
                    let target = self.build_stack_pop(inner_context_sp, inner_context_sp_offset);
                    let val = self.build_stack_pop(inner_context_sp, inner_context_sp_offset);

                    if code.jumpdests.is_empty() {
                        // there are no valid jump targets, this Jumpi has to fail!
                        self.builder.build_unconditional_branch(block_error);

                    } else {
                        let block_jump_no = self.context.insert_basic_block_after(block_instructions[i], &format!("instruction #{}: {:?} / jump no", i, code.code.ops[i]));
                        let block_jump_yes = self.context.insert_basic_block_after(block_jump_no, &format!("instruction #{}: {:?} / jump yes", i, code.code.ops[i]));

                        let mut jump_table = Vec::new();
                        for (j, jmp_i) in code.jumpdests.iter().enumerate() {
                            let jmp_target = code.opidx2target[jmp_i];
                            jump_table.push(self.context.insert_basic_block_after(if j == 0 { block_jump_yes } else { jump_table[j-1] }, &format!("instruction #{}: {:?} / to Jumpdest #{} at op #{} to byte #{}", i, code.code.ops[i], j, jmp_i, jmp_target)));
                        }

                        let cmp = self.builder.build_int_compare(IntPredicate::EQ, i64_type.const_int(0, false), val, "");
                        self.builder.build_conditional_branch(cmp, block_jump_no, block_jump_yes);

                        self.builder.position_at_end(block_jump_no);
                        self.builder.build_unconditional_branch(block_instructions[i+1]);

                        self.builder.position_at_end(block_jump_yes);
                        self.builder.build_unconditional_branch(jump_table[0]);

                        for (j, jmp_i) in code.jumpdests.iter().enumerate() {
                            let jmp_target = code.opidx2target[jmp_i];
                            let jmp_target = jmp_target.as_u64();
                            self.builder.position_at_end(jump_table[j]);
                            let cmp = self.builder.build_int_compare(IntPredicate::EQ, i64_type.const_int(jmp_target, false), target, "");
                            self.builder.build_conditional_branch(cmp, block_instructions[*jmp_i], if j+1 == code.jumpdests.len() { block_error } else { jump_table[j+1] });
                        }
                    }
                },
                Swap1 => {
                    let idx_a = i64_type.const_int(1*8, false);
                    let idx_b = i64_type.const_int(2*8, false);
                    let a = self.build_stack_read(inner_context_sp, idx_a);
                    let b = self.build_stack_read(inner_context_sp, idx_b);
                    self.build_stack_write(inner_context_sp, idx_a, b);
                    self.build_stack_write(inner_context_sp, idx_b, a);
                    self.builder.build_unconditional_branch(block_instructions[i+1]);
                },
                Swap2 => {
                    let idx_a = i64_type.const_int(1*8, false);
                    let idx_b = i64_type.const_int(3*8, false);
                    let a = self.build_stack_read(inner_context_sp, idx_a);
                    let b = self.build_stack_read(inner_context_sp, idx_b);
                    self.build_stack_write(inner_context_sp, idx_a, b);
                    self.build_stack_write(inner_context_sp, idx_b, a);
                    self.builder.build_unconditional_branch(block_instructions[i+1]);
                },
                Dup2 => {
                    let idx = i64_type.const_int(2*8, false);
                    let val = self.build_stack_read(inner_context_sp, idx);
                    self.build_stack_push(inner_context_sp, inner_context_sp_offset, val);
                    self.builder.build_unconditional_branch(block_instructions[i+1]);
                },
                Dup3 => {
                    let idx = i64_type.const_int(3*8, false);
                    let val = self.build_stack_read(inner_context_sp, idx);
                    self.build_stack_push(inner_context_sp, inner_context_sp_offset, val);
                    self.builder.build_unconditional_branch(block_instructions[i+1]);
                },
                Dup4 => {
                    let idx = i64_type.const_int(4*8, false);
                    let val = self.build_stack_read(inner_context_sp, idx);
                    self.build_stack_push(inner_context_sp, inner_context_sp_offset, val);
                    self.builder.build_unconditional_branch(block_instructions[i+1]);
                },
                Iszero => {
                    let val = self.build_stack_pop(inner_context_sp, inner_context_sp_offset);

                    let block_push_0 = self.context.insert_basic_block_after(block_instructions[i], &format!("instruction #{}: {:?} / push 0", i, code.code.ops[i]));
                    let block_push_1 = self.context.insert_basic_block_after(block_push_0, &format!("instruction #{}: {:?} / push 1", i, code.code.ops[i]));

                    let cmp = self.builder.build_int_compare(IntPredicate::EQ, i64_type.const_int(0, false), val, "");
                    self.builder.build_conditional_branch(cmp, block_push_1, block_push_0);

                    self.builder.position_at_end(block_push_0);
                    self.build_stack_push(inner_context_sp, inner_context_sp_offset, i64_type.const_int(0, false));
                    self.builder.build_unconditional_branch(block_instructions[i+1]);

                    self.builder.position_at_end(block_push_1);
                    self.build_stack_push(inner_context_sp, inner_context_sp_offset, i64_type.const_int(1, false));
                    self.builder.build_unconditional_branch(block_instructions[i+1]);
                },
                Add => {
                    let a = self.build_stack_pop(inner_context_sp, inner_context_sp_offset);
                    let b = self.build_stack_pop(inner_context_sp, inner_context_sp_offset);
                    let c = self.builder.build_int_add(a, b, "");
                    self.build_stack_push(inner_context_sp, inner_context_sp_offset, c);
                    self.builder.build_unconditional_branch(block_instructions[i+1]);
                },
                Sub => {
                    let a = self.build_stack_pop(inner_context_sp, inner_context_sp_offset);
                    let b = self.build_stack_pop(inner_context_sp, inner_context_sp_offset);
                    let c = self.builder.build_int_sub(a, b, "");
                    self.build_stack_push(inner_context_sp, inner_context_sp_offset, c);
                    self.builder.build_unconditional_branch(block_instructions[i+1]);
                },


                AugmentedPushJump(_, val) => {
                    if code.jumpdests.is_empty() {
                        // there are no valid jump targets, this Jump has to fail!
                        self.builder.build_unconditional_branch(block_error);
                    } else {
                        // retrieve the corresponding jump target (panic if not a valid jump target) ...
                        let jmp_i = code.target2opidx[val];
                        // ... and jump to there!
                        self.builder.build_unconditional_branch(block_instructions[jmp_i]);
                    }
                },
                AugmentedPushJumpi(_, val) => {
                    let condition = self.build_stack_pop(inner_context_sp, inner_context_sp_offset);

                    if code.jumpdests.is_empty() {
                        // there are no valid jump targets, this Jumpi has to fail!
                        self.builder.build_unconditional_branch(block_error);

                    } else {
                        let block_jump_no = self.context.insert_basic_block_after(block_instructions[i], &format!("instruction #{}: {:?} / jump no", i, code.code.ops[i]));
                        let block_jump_yes = self.context.insert_basic_block_after(block_jump_no, &format!("instruction #{}: {:?} / jump yes", i, code.code.ops[i]));

                        let cmp = self.builder.build_int_compare(IntPredicate::EQ, i64_type.const_int(0, false), condition, "");
                        self.builder.build_conditional_branch(cmp, block_jump_no, block_jump_yes);

                        self.builder.position_at_end(block_jump_no);
                        self.builder.build_unconditional_branch(block_instructions[i+1]);

                        self.builder.position_at_end(block_jump_yes);
                        // retrieve the corresponding jump target (panic if not a valid jump target) ...
                        let jmp_i = code.target2opidx[val];
                        // ... and jump to there!
                        self.builder.build_unconditional_branch(block_instructions[jmp_i]);
                    }
                },


                _ => {
                    panic!("Op not implemented: {:?}", op);
                },
            }
        }


        // OUTPUT LLVM
        self.module.print_to_stderr();


        // COMPILE
        let run_fn: JitFunction<JitEvmCompiledContract> = unsafe { self.execution_engine.get_function("executecontract")? };
        Ok(run_fn)
    }
}
