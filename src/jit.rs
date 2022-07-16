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
use inkwell::values::{IntValue, PointerValue, PhiValue};
// use inkwell::types::{IntType, };//PointerType};
use inkwell::basic_block::BasicBlock;
use inkwell::builder::Builder;
use inkwell::module::Module;
use crate::code::{EvmOp, IndexedEvmCode};
use crate::constants::{EVM_STACK_SIZE, EVM_STACK_ELEMENT_SIZE};


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


#[derive(Debug, Copy, Clone)]
pub struct JitEvmEngineBookkeeping<'ctx> {
    pub stackbase: IntValue<'ctx>,
    pub sp: IntValue<'ctx>,
    pub retval: IntValue<'ctx>,
}

impl<'ctx> JitEvmEngineBookkeeping<'ctx> {
    pub fn update_sp(&self, sp: IntValue<'ctx>) -> Self {
        Self { sp, stackbase: self.stackbase, retval: self.retval }
    }
}


#[derive(Debug, Copy, Clone)]
pub struct JitEvmEngineSimpleBlock<'ctx> {
    pub block: BasicBlock<'ctx>,
    pub phi_stackbase: PhiValue<'ctx>,
    pub phi_sp: PhiValue<'ctx>,
    pub phi_retval: PhiValue<'ctx>,
    // pub label: String,
    // pub suffix: String,
}

impl<'ctx> JitEvmEngineSimpleBlock<'ctx> {
    pub fn new(engine: &JitEvmEngine<'ctx>, block_before: BasicBlock<'ctx>, name: &str, suffix: &str) -> Self {
        let i64_type = engine.context.i64_type();

        let block = engine.context.insert_basic_block_after(block_before, name);
        engine.builder.position_at_end(block);
        let phi_stackbase = engine.builder.build_phi(i64_type, &format!("stackbase{}", suffix));
        let phi_sp = engine.builder.build_phi(i64_type, &format!("sp{}", suffix));
        let phi_retval = engine.builder.build_phi(i64_type, &format!("retval{}", suffix));

        Self { block, phi_stackbase, phi_sp, phi_retval } //, label: name.to_string(), suffix: suffix.to_string() }
    }

    pub fn add_incoming(&self, book: &JitEvmEngineBookkeeping<'ctx>, prev: &JitEvmEngineSimpleBlock<'ctx>) {
        self.phi_stackbase.add_incoming(&[(&book.stackbase, prev.block)]);
        self.phi_sp.add_incoming(&[(&book.sp, prev.block)]);
        self.phi_retval.add_incoming(&[(&book.retval, prev.block)]);
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
        book: JitEvmEngineBookkeeping<'a>,
        val: IntValue<'a>) -> JitEvmEngineBookkeeping<'a>
    {
        let i64_type = self.context.i64_type();
        let sp_offset = i64_type.const_int(EVM_STACK_ELEMENT_SIZE, false);

        let sp_ptr = self.builder.build_int_to_ptr(book.sp, i64_type.ptr_type(AddressSpace::Generic), "");
        self.builder.build_store(sp_ptr, val);
        let sp = self.builder.build_int_add(book.sp, sp_offset, "");

        book.update_sp(sp)
    }
    
    fn build_stack_pop<'a>(
        &'a self,
        book: JitEvmEngineBookkeeping<'a>) -> (JitEvmEngineBookkeeping<'a>, IntValue<'a>)
    {
        let i64_type = self.context.i64_type();
        let sp_offset = i64_type.const_int(EVM_STACK_ELEMENT_SIZE, false);

        let sp = self.builder.build_int_sub(book.sp, sp_offset, "");
        let sp_ptr = self.builder.build_int_to_ptr(sp, i64_type.ptr_type(AddressSpace::Generic), "");
        let val = self.builder.build_load(sp_ptr, "").into_int_value();

        (book.update_sp(sp), val)
    }
    
    fn build_stack_write<'a>(
        &'a self,
        book: JitEvmEngineBookkeeping<'a>,
        idx: usize,
        val: IntValue<'a>) -> JitEvmEngineBookkeeping<'a>
    {
        let i64_type = self.context.i64_type();
        let idx = i64_type.const_int((idx as u64)*EVM_STACK_ELEMENT_SIZE, false);

        let sp_int = self.builder.build_int_sub(book.sp, idx, "");
        let sp_ptr = self.builder.build_int_to_ptr(sp_int, i64_type.ptr_type(AddressSpace::Generic), "");
        self.builder.build_store(sp_ptr, val);

        book
    }
    
    fn build_stack_read<'a>(
        &'a self,
        book: JitEvmEngineBookkeeping<'a>,
        idx: usize) -> (JitEvmEngineBookkeeping<'a>, IntValue<'a>)
    {
        let i64_type = self.context.i64_type();
        let idx = i64_type.const_int((idx as u64)*EVM_STACK_ELEMENT_SIZE, false);

        let sp_int = self.builder.build_int_sub(book.sp, idx, "");
        let sp_ptr = self.builder.build_int_to_ptr(sp_int, i64_type.ptr_type(AddressSpace::Generic), "");
        let val = self.builder.build_load(sp_ptr, "").into_int_value();

        (book, val)
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


        // SETUP HANDLER

        let setup_block = self.context.append_basic_block(function, "setup");
        self.builder.position_at_end(setup_block);

        // let stackbase = self.builder.build_int_to_ptr(function.get_nth_param(0).unwrap().into_int_value(), i64_type.ptr_type(AddressSpace::Generic), "stackbase");
        let stackbase = function.get_nth_param(0).unwrap().into_int_value();
        let retval = i64_type.const_int(0, false);
        let setup_book = JitEvmEngineBookkeeping { stackbase: stackbase, sp: stackbase, retval: retval };


        // INSTRUCTIONS

        let ops_len = code.code.ops.len();
        assert!(ops_len > 0);

        let mut instructions: Vec<JitEvmEngineSimpleBlock<'_>> = Vec::new();
        for i in 0..ops_len {
            let block_before = if i == 0 {
                setup_block
            } else {
                instructions[i-1].block
            };
            let label = format!("Instruction #{}: {:?}", i, code.code.ops[i]);
            instructions.push(JitEvmEngineSimpleBlock::new(self, block_before, &label, &format!("-{}", i)));
        }

        self.builder.position_at_end(setup_block);
        self.builder.build_unconditional_branch(instructions[0].block);
        instructions[0].phi_stackbase.add_incoming(&[(&setup_book.stackbase, setup_block)]);
        instructions[0].phi_sp.add_incoming(&[(&setup_book.sp, setup_block)]);
        instructions[0].phi_retval.add_incoming(&[(&setup_book.retval, setup_block)]);


        // END HANDLER

        let end = JitEvmEngineSimpleBlock::new(self, instructions[ops_len-1].block, &"end", &"-end");
        self.builder.build_return(Some(&end.phi_retval.as_basic_value().into_int_value()));


        // RENDER INSTRUCTIONS

        for (i, op) in code.code.ops.iter().enumerate() {
            use EvmOp::*;

            let this = instructions[i];

            self.builder.position_at_end(this.block);
            let book = JitEvmEngineBookkeeping {
                stackbase: this.phi_stackbase.as_basic_value().into_int_value(),
                sp: this.phi_sp.as_basic_value().into_int_value(),
                retval: this.phi_retval.as_basic_value().into_int_value(),
            };

            let next = if i+1 == ops_len { end } else { instructions[i+1] };

            match op {
                Stop => {
                    let val = i64_type.const_int(0, false);
                    self.builder.build_return(Some(&val));
                },
                Push(_, val) => {
                    let val = i64_type.const_int(val.as_u64(), false);
                    let book = self.build_stack_push(book, val);

                    self.builder.build_unconditional_branch(next.block);
                    next.add_incoming(&book, &this);
                },
                Pop => {
                    let (book, _) = self.build_stack_pop(book);

                    self.builder.build_unconditional_branch(next.block);
                    next.add_incoming(&book, &this);
                },
                Jumpdest => {
                    self.builder.build_unconditional_branch(next.block);
                    next.add_incoming(&book, &this);
                },
                // Jump => {  /// TODO
                //     let (book, target) = self.build_stack_pop(book);

                //     if code.jumpdests.is_empty() {
                //         // there are no valid jump targets, this Jump has to fail!
                //         self.builder.build_unconditional_branch(end.block);

                //     } else {
                //         let mut jump_table = Vec::new();
                //         for (j, jmp_i) in code.jumpdests.iter().enumerate() {
                //             let jmp_target = code.opidx2target[jmp_i];
                //             jump_table.push(self.context.insert_basic_block_after(if j == 0 { instructions_block[i] } else { jump_table[j-1] }, &format!("instruction #{}: {:?} / to Jumpdest #{} at op #{} to byte #{}", i, code.code.ops[i], j, jmp_i, jmp_target)));
                //         }

                //         self.builder.build_unconditional_branch(jump_table[0]);

                //         for (j, jmp_i) in code.jumpdests.iter().enumerate() {
                //             let jmp_target = code.opidx2target[jmp_i];
                //             let jmp_target = jmp_target.as_u64();
                //             self.builder.position_at_end(jump_table[j]);
                //             let cmp = self.builder.build_int_compare(IntPredicate::EQ, i64_type.const_int(jmp_target, false), target, "");
                //             self.builder.build_conditional_branch(cmp, instructions_block[*jmp_i], if j+1 == code.jumpdests.len() { end.block } else { jump_table[j+1] });
                //         }
                //     }
                // },
                // Jumpi => { /// TODO
                //     let (book, target) = self.build_stack_pop(book);
                //     let (book, val) = self.build_stack_pop(book);

                //     if code.jumpdests.is_empty() {
                //         // there are no valid jump targets, this Jumpi has to fail!
                //         self.builder.build_unconditional_branch(end.block);

                //     } else {
                //         let block_jump_no = self.context.insert_basic_block_after(instructions_block[i], &format!("instruction #{}: {:?} / jump no", i, code.code.ops[i]));
                //         let block_jump_yes = self.context.insert_basic_block_after(block_jump_no, &format!("instruction #{}: {:?} / jump yes", i, code.code.ops[i]));

                //         let mut jump_table = Vec::new();
                //         for (j, jmp_i) in code.jumpdests.iter().enumerate() {
                //             let jmp_target = code.opidx2target[jmp_i];
                //             jump_table.push(self.context.insert_basic_block_after(if j == 0 { block_jump_yes } else { jump_table[j-1] }, &format!("instruction #{}: {:?} / to Jumpdest #{} at op #{} to byte #{}", i, code.code.ops[i], j, jmp_i, jmp_target)));
                //         }

                //         let cmp = self.builder.build_int_compare(IntPredicate::EQ, i64_type.const_int(0, false), val, "");
                //         self.builder.build_conditional_branch(cmp, block_jump_no, block_jump_yes);

                //         self.builder.position_at_end(block_jump_no);
                //         self.builder.build_unconditional_branch(next.block);

                //         self.builder.position_at_end(block_jump_yes);
                //         self.builder.build_unconditional_branch(jump_table[0]);

                //         for (j, jmp_i) in code.jumpdests.iter().enumerate() {
                //             let jmp_target = code.opidx2target[jmp_i];
                //             let jmp_target = jmp_target.as_u64();
                //             self.builder.position_at_end(jump_table[j]);
                //             let cmp = self.builder.build_int_compare(IntPredicate::EQ, i64_type.const_int(jmp_target, false), target, "");
                //             self.builder.build_conditional_branch(cmp, instructions_block[*jmp_i], if j+1 == code.jumpdests.len() { end.block } else { jump_table[j+1] });
                //         }
                //     }
                // },
                Swap1 => {
                    let (book, a) = self.build_stack_read(book, 1);
                    let (book, b) = self.build_stack_read(book, 2);
                    let book = self.build_stack_write(book, 1, b);
                    let book = self.build_stack_write(book, 2, a);

                    self.builder.build_unconditional_branch(next.block);
                    next.add_incoming(&book, &this);
                },
                Swap2 => {
                    let (book, a) = self.build_stack_read(book, 1);
                    let (book, b) = self.build_stack_read(book, 3);
                    let book = self.build_stack_write(book, 1, b);
                    let book = self.build_stack_write(book, 3, a);

                    self.builder.build_unconditional_branch(next.block);
                    next.add_incoming(&book, &this);
                },
                Dup2 => {
                    let (book, val) = self.build_stack_read(book, 2);
                    let book = self.build_stack_push(book, val);

                    self.builder.build_unconditional_branch(next.block);
                    next.add_incoming(&book, &this);
                },
                Dup3 => {
                    let (book, val) = self.build_stack_read(book, 3);
                    let book = self.build_stack_push(book, val);

                    self.builder.build_unconditional_branch(next.block);
                    next.add_incoming(&book, &this);
                },
                Dup4 => {
                    let (book, val) = self.build_stack_read(book, 4);
                    let book = self.build_stack_push(book, val);

                    self.builder.build_unconditional_branch(next.block);
                    next.add_incoming(&book, &this);
                },
                Iszero => {
                    let (book, val) = self.build_stack_pop(book);
                    let cmp = self.builder.build_int_compare(IntPredicate::EQ, i64_type.const_int(0, false), val, "");

                    let push_0 = JitEvmEngineSimpleBlock::new(self, instructions[i].block, &format!("Instruction #{}: {:?} / push 0", i, op), &format!("-{}-0", i));
                    let push_1 = JitEvmEngineSimpleBlock::new(self, push_0.block, &format!("Instruction #{}: {:?} / push 1", i, op), &format!("-{}-1", i));
                    self.builder.position_at_end(this.block);
                    self.builder.build_conditional_branch(cmp, push_1.block, push_0.block);
                    push_0.add_incoming(&book, &this);
                    push_1.add_incoming(&book, &this);

                    self.builder.position_at_end(push_0.block);
                    let book_0 = self.build_stack_push(book, i64_type.const_int(0, false));
                    self.builder.build_unconditional_branch(next.block);
                    next.add_incoming(&book_0, &push_0);

                    self.builder.position_at_end(push_1.block);
                    let book_1 = self.build_stack_push(book, i64_type.const_int(1, false));
                    self.builder.build_unconditional_branch(next.block);
                    next.add_incoming(&book_1, &push_1);
                },
                Add => {
                    let (book, a) = self.build_stack_pop(book);
                    let (book, b) = self.build_stack_pop(book);
                    let c = self.builder.build_int_add(a, b, "");
                    let book = self.build_stack_push(book, c);

                    self.builder.build_unconditional_branch(next.block);
                    next.add_incoming(&book, &this);
                },
                Sub => {
                    let (book, a) = self.build_stack_pop(book);
                    let (book, b) = self.build_stack_pop(book);
                    let c = self.builder.build_int_sub(a, b, "");
                    let book = self.build_stack_push(book, c);

                    self.builder.build_unconditional_branch(next.block);
                    next.add_incoming(&book, &this);
                },


                AugmentedPushJump(_, val) => {
                    if code.jumpdests.is_empty() {
                        // there are no valid jump targets, this Jump has to fail!
                        self.builder.build_unconditional_branch(end.block);
                        end.add_incoming(&book, &this);
                    } else {
                        // retrieve the corresponding jump target (panic if not a valid jump target) ...
                        let jmp_i = code.target2opidx[val];
                        // ... and jump to there!
                        self.builder.build_unconditional_branch(instructions[jmp_i].block);
                        instructions[jmp_i].add_incoming(&book, &this);
                    }
                },
                AugmentedPushJumpi(_, val) => {
                    let (book, condition) = self.build_stack_pop(book);

                    if code.jumpdests.is_empty() {
                        // there are no valid jump targets, this Jumpi has to fail!
                        self.builder.build_unconditional_branch(end.block);
                        end.add_incoming(&book, &this);

                    } else {
                        // retrieve the corresponding jump target (panic if not a valid jump target) ...
                        let jmp_i = code.target2opidx[val];
                        // ... and jump to there (conditionally)!
                        let cmp = self.builder.build_int_compare(IntPredicate::EQ, i64_type.const_int(0, false), condition, "");
                        self.builder.build_conditional_branch(cmp, next.block, instructions[jmp_i].block);
                        next.add_incoming(&book, &this);
                        instructions[jmp_i].add_incoming(&book, &this);


                        // let block_jump_no = self.context.insert_basic_block_after(instructions_block[i], &format!("instruction #{}: {:?} / jump no", i, code.code.ops[i]));
                        // let block_jump_yes = self.context.insert_basic_block_after(block_jump_no, &format!("instruction #{}: {:?} / jump yes", i, code.code.ops[i]));

                        // let cmp = self.builder.build_int_compare(IntPredicate::EQ, i64_type.const_int(0, false), condition, "");
                        // self.builder.build_conditional_branch(cmp, block_jump_no, block_jump_yes);

                        // self.builder.position_at_end(block_jump_no);
                        // self.builder.build_unconditional_branch(next.block);
                        // next_phi_stackbase.add_incoming(&[(&book.stackbase, instructions_block[i])]);
                        // next_phi_sp.add_incoming(&[(&book.sp, instructions_block[i])]);

                        // self.builder.position_at_end(block_jump_yes);
                        // // retrieve the corresponding jump target (panic if not a valid jump target) ...
                        // let jmp_i = code.target2opidx[val];
                        // // ... and jump to there!
                        // self.builder.build_unconditional_branch(instructions_block[jmp_i]);
                        // instructions_phi_stackbase[jmp_i].add_incoming(&[(&book.stackbase, instructions_block[i])]);
                        // instructions_phi_sp[jmp_i].add_incoming(&[(&book.sp, instructions_block[i])]);
                        // instructions_phi_retval[jmp_i].add_incoming(&[(&book.retval, instructions_block[i])]);
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
