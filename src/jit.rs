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
use inkwell::types::{IntType, };//PointerType};
use inkwell::basic_block::BasicBlock;
use inkwell::builder::Builder;
use inkwell::module::Module;
use crate::code::{EvmOp, IndexedEvmCode};
use crate::constants::{EVM_STACK_SIZE, EVM_STACK_ELEMENT_SIZE};


pub type JitEvmCompiledContract = unsafe extern "C" fn(usize) -> u64;
const _EVM_JIT_STACK_ALIGN: u32 = 16;


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

impl From<&str> for JitEvmEngineError {
    fn from(e: &str) -> Self {
        Self::UnknownStringError(e.to_string())
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
}

impl<'ctx> JitEvmEngineSimpleBlock<'ctx> {
    pub fn new(engine: &JitEvmEngine<'ctx>, block_before: BasicBlock<'ctx>, name: &str, suffix: &str) -> Self {
        let i64_type = engine.context.i64_type();

        let block = engine.context.insert_basic_block_after(block_before, name);
        engine.builder.position_at_end(block);
        let phi_stackbase = engine.builder.build_phi(i64_type, &format!("stackbase{}", suffix));
        let phi_sp = engine.builder.build_phi(i64_type, &format!("sp{}", suffix));
        let phi_retval = engine.builder.build_phi(i64_type, &format!("retval{}", suffix));

        Self { block, phi_stackbase, phi_sp, phi_retval }
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
    pub type_ptrint: IntType<'ctx>,
    pub type_stackel: IntType<'ctx>,
    pub type_retval: IntType<'ctx>,
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

        let target_data = execution_engine.get_target_data();
        let type_ptrint = context.ptr_sized_int_type(&target_data, None);   // type for pointers (stack pointer, host interaction)
        // ensure consistency btw Rust/LLVM definition of compiled contract function
        assert_eq!(type_ptrint.get_bit_width(), 64);
        assert_eq!(usize::BITS, 64);
        // TODO: the above assumes that pointers address memory byte-wise!

        let type_stackel = context.custom_width_int_type(256);   // type for stack elements
        assert_eq!(type_stackel.get_bit_width(), 256);
        assert_eq!(type_stackel.get_bit_width() as u64, EVM_STACK_ELEMENT_SIZE * 8);

        let type_retval = context.i64_type();   // type for return value
        // ensure consistency btw Rust/LLVM definition of compiled contract function
        assert_eq!(type_retval.get_bit_width(), 64);
        assert_eq!(u64::BITS, 64);

        Ok(Self {
            context: &context,
            module,
            builder,
            execution_engine,
            type_ptrint,
            type_stackel,
            type_retval,
        })
    }


    // HELPER FUNCTIONS

    fn build_stack_push<'a>(
        &'a self,
        book: JitEvmEngineBookkeeping<'a>,
        val: IntValue<'a>) -> JitEvmEngineBookkeeping<'a>
    {
        let sp_offset = self.type_ptrint.const_int(EVM_STACK_ELEMENT_SIZE, false);

        let sp_ptr = self.builder.build_int_to_ptr(book.sp, self.type_stackel.ptr_type(AddressSpace::Generic), "");
        self.builder.build_store(sp_ptr, val);
        let sp = self.builder.build_int_add(book.sp, sp_offset, "");

        book.update_sp(sp)
    }
    
    fn build_stack_pop<'a>(
        &'a self,
        book: JitEvmEngineBookkeeping<'a>) -> (JitEvmEngineBookkeeping<'a>, IntValue<'a>)
    {
        let sp_offset = self.type_ptrint.const_int(EVM_STACK_ELEMENT_SIZE, false);

        let sp = self.builder.build_int_sub(book.sp, sp_offset, "");
        let sp_ptr = self.builder.build_int_to_ptr(sp, self.type_stackel.ptr_type(AddressSpace::Generic), "");
        let val = self.builder.build_load(sp_ptr, "").into_int_value();

        (book.update_sp(sp), val)
    }
    
    fn build_stack_write<'a>(
        &'a self,
        book: JitEvmEngineBookkeeping<'a>,
        idx: usize,
        val: IntValue<'a>) -> JitEvmEngineBookkeeping<'a>
    {
        let idx = self.type_ptrint.const_int((idx as u64)*EVM_STACK_ELEMENT_SIZE, false);

        let sp_int = self.builder.build_int_sub(book.sp, idx, "");
        let sp_ptr = self.builder.build_int_to_ptr(sp_int, self.type_stackel.ptr_type(AddressSpace::Generic), "");
        self.builder.build_store(sp_ptr, val);

        book
    }
    
    fn build_stack_read<'a>(
        &'a self,
        book: JitEvmEngineBookkeeping<'a>,
        idx: u64) -> (JitEvmEngineBookkeeping<'a>, IntValue<'a>)
    {
        let idx = self.type_ptrint.const_int((idx as u64)*EVM_STACK_ELEMENT_SIZE, false);

        let sp_int = self.builder.build_int_sub(book.sp, idx, "");
        let sp_ptr = self.builder.build_int_to_ptr(sp_int, self.type_stackel.ptr_type(AddressSpace::Generic), "");
        let val = self.builder.build_load(sp_ptr, "").into_int_value();

        (book, val)
    }

    fn build_dup<'a>(
        &'a self,
        book: JitEvmEngineBookkeeping<'a>,
        idx: u64) -> Result<JitEvmEngineBookkeeping<'a>, JitEvmEngineError>
    {
        let len_stackel = self.type_ptrint.const_int(EVM_STACK_ELEMENT_SIZE, false);
        let sp_src_offset = self.type_ptrint.const_int(idx*EVM_STACK_ELEMENT_SIZE, false);
        let src_int = self.builder.build_int_sub(book.sp, sp_src_offset, "");
        let src_ptr = self.builder.build_int_to_ptr(src_int, self.type_stackel.ptr_type(AddressSpace::Generic), "");
        let dst_ptr = self.builder.build_int_to_ptr(book.sp, self.type_stackel.ptr_type(AddressSpace::Generic), "");
        self.builder.build_memcpy(dst_ptr, _EVM_JIT_STACK_ALIGN, src_ptr, _EVM_JIT_STACK_ALIGN, len_stackel)?;
        let sp = self.builder.build_int_add(book.sp, len_stackel, "");
        let book = book.update_sp(sp);

        Ok(book)
    }


    pub fn jit_compile_contract(&self, code: &IndexedEvmCode) -> Result<JitFunction<JitEvmCompiledContract>, JitEvmEngineError> {
        // let void_type = self.context.void_type();
        // let i64_type = self.context.i64_type();


        // //  Install our global callback into the system <------ later! code fragment from github repo above, will be useful to integrate with "outer" context of EVM
        // let i1_type = context.custom_width_int_type(1);
        // let cb_type = i1_type.fn_type(
        //     &[i64_type.array_type(6).ptr_type(AddressSpace::Generic).into()], false);
        // let cb_func = module.add_function("cb", cb_type, None);
        // execution_engine.add_global_mapping(&cb_func, callback as usize);


        let executecontract_fn_type = self.type_retval.fn_type(&[self.type_ptrint.into()], false);
        let function = self.module.add_function("executecontract", executecontract_fn_type, None);


        // SETUP HANDLER

        let setup_block = self.context.append_basic_block(function, "setup");
        self.builder.position_at_end(setup_block);

        // let stackbase = self.builder.build_int_to_ptr(function.get_nth_param(0).unwrap().into_int_value(), i64_type.ptr_type(AddressSpace::Generic), "stackbase");
        let stackbase = function.get_nth_param(0).unwrap().into_int_value();
        let retval = self.type_retval.const_int(0, false);
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


        // ERROR-JUMPDEST HANDLER

        let error_jumpdest = JitEvmEngineSimpleBlock::new(self, end.block, &"error-jumpdest", &"-error-jumpdest");
        self.builder.build_return(Some(&self.type_retval.const_int(1, false)));


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

            let book = match op {
                Stop => {
                    let val = self.type_retval.const_int(0, false);
                    self.builder.build_return(Some(&val));
                    continue;   // skip auto-generated jump to next instruction
                },
                Push(_, val) => {
                    let val = self.type_stackel.const_int_arbitrary_precision(&val.0);
                    let book = self.build_stack_push(book, val);
                    book
                },
                Pop => {
                    let (book, _) = self.build_stack_pop(book);
                    book
                },
                Jumpdest => {
                    book
                },
                Jump => {
                    let (book, target) = self.build_stack_pop(book);

                    if code.jumpdests.is_empty() {
                        // there are no valid jump targets, this Jump has to fail!
                        self.builder.build_unconditional_branch(end.block);
                        end.add_incoming(&book, &this);

                    } else {
                        let mut jump_table: Vec<JitEvmEngineSimpleBlock<'_>> = Vec::new();
                        for (j, jmp_i) in code.jumpdests.iter().enumerate() {
                            let jmp_target = code.opidx2target[jmp_i];
                            jump_table.push(JitEvmEngineSimpleBlock::new(
                                self,
                                if j == 0 { this.block } else { jump_table[j-1].block },
                                &format!("instruction #{}: {:?} / to Jumpdest #{} at op #{} to byte #{}", i, op, j, jmp_i, jmp_target),
                                &format!("-{}-{}", i, j),
                            ));
                        }

                        self.builder.position_at_end(this.block);
                        self.builder.build_unconditional_branch(jump_table[0].block);
                        jump_table[0].add_incoming(&book, &this);

                        for (j, jmp_i) in code.jumpdests.iter().enumerate() {
                            let jmp_target = code.opidx2target[jmp_i];
                            let jmp_target = jmp_target.as_u64();   // REMARK: assumes that code cannot exceed 2^64 instructions, probably ok ;)
                            self.builder.position_at_end(jump_table[j].block);
                            let cmp = self.builder.build_int_compare(IntPredicate::EQ, self.type_stackel.const_int(jmp_target, false), target, "");
                            if j+1 == code.jumpdests.len() {
                                self.builder.build_conditional_branch(cmp, instructions[*jmp_i].block, error_jumpdest.block);
                                instructions[*jmp_i].add_incoming(&book, &jump_table[j]);
                                error_jumpdest.add_incoming(&book, &jump_table[j]);
                            } else {
                                self.builder.build_conditional_branch(cmp, instructions[*jmp_i].block, jump_table[j+1].block);
                                instructions[*jmp_i].add_incoming(&book, &jump_table[j]);
                                jump_table[j+1].add_incoming(&book, &jump_table[j]);
                            }
                        }
                    }

                    continue;   // skip auto-generated jump to next instruction
                },
                Jumpi => {
                    let (book, target) = self.build_stack_pop(book);
                    let (book, val) = self.build_stack_pop(book);

                    if code.jumpdests.is_empty() {
                        // there are no valid jump targets, this Jumpi has to fail!
                        self.builder.build_unconditional_branch(end.block);
                        end.add_incoming(&book, &this);

                    } else {
                        let mut jump_table: Vec<JitEvmEngineSimpleBlock<'_>> = Vec::new();
                        for (j, jmp_i) in code.jumpdests.iter().enumerate() {
                            let jmp_target = code.opidx2target[jmp_i];
                            jump_table.push(JitEvmEngineSimpleBlock::new(
                                self,
                                if j == 0 { this.block } else { jump_table[j-1].block },
                                &format!("instruction #{}: {:?} / to Jumpdest #{} at op #{} to byte #{}", i, op, j, jmp_i, jmp_target),
                                &format!("-{}-{}", i, j),
                            ));
                        }

                        self.builder.position_at_end(this.block);
                        let cmp = self.builder.build_int_compare(IntPredicate::EQ, self.type_stackel.const_int(0, false), val, "");
                        self.builder.build_conditional_branch(cmp, next.block, jump_table[0].block);
                        next.add_incoming(&book, &this);
                        jump_table[0].add_incoming(&book, &this);

                        for (j, jmp_i) in code.jumpdests.iter().enumerate() {
                            let jmp_target = code.opidx2target[jmp_i];
                            let jmp_target = jmp_target.as_u64();   // REMARK: assumes that code cannot exceed 2^64 instructions, probably ok ;)
                            self.builder.position_at_end(jump_table[j].block);
                            let cmp = self.builder.build_int_compare(IntPredicate::EQ, self.type_stackel.const_int(jmp_target, false), target, "");
                            if j+1 == code.jumpdests.len() {
                                self.builder.build_conditional_branch(cmp, instructions[*jmp_i].block, error_jumpdest.block);
                                instructions[*jmp_i].add_incoming(&book, &jump_table[j]);
                                error_jumpdest.add_incoming(&book, &jump_table[j]);
                            } else {
                                self.builder.build_conditional_branch(cmp, instructions[*jmp_i].block, jump_table[j+1].block);
                                instructions[*jmp_i].add_incoming(&book, &jump_table[j]);
                                jump_table[j+1].add_incoming(&book, &jump_table[j]);
                            }
                        }
                    }

                    continue;   // skip auto-generated jump to next instruction
                },
                Swap1 => {
                    let (book, a) = self.build_stack_read(book, 1);
                    let (book, b) = self.build_stack_read(book, 2);
                    let book = self.build_stack_write(book, 1, b);
                    let book = self.build_stack_write(book, 2, a);
                    book
                },
                Swap2 => {
                    let (book, a) = self.build_stack_read(book, 1);
                    let (book, b) = self.build_stack_read(book, 3);
                    let book = self.build_stack_write(book, 1, b);
                    let book = self.build_stack_write(book, 3, a);
                    book
                },
                Dup1 => {
                    let book = self.build_dup(book, 1)?;
                    book
                },
                Dup2 => {
                    let book = self.build_dup(book, 2)?;
                    book
                },
                Dup3 => {
                    let book = self.build_dup(book, 3)?;
                    book
                },
                Dup4 => {
                    let book = self.build_dup(book, 4)?;
                    book
                },
                Iszero => {
                    let (book, val) = self.build_stack_pop(book);
                    let cmp = self.builder.build_int_compare(IntPredicate::EQ, self.type_stackel.const_int(0, false), val, "");

                    let push_0 = JitEvmEngineSimpleBlock::new(self, instructions[i].block, &format!("Instruction #{}: {:?} / push 0", i, op), &format!("-{}-0", i));
                    let push_1 = JitEvmEngineSimpleBlock::new(self, push_0.block, &format!("Instruction #{}: {:?} / push 1", i, op), &format!("-{}-1", i));
                    
                    self.builder.position_at_end(this.block);
                    self.builder.build_conditional_branch(cmp, push_1.block, push_0.block);
                    push_0.add_incoming(&book, &this);
                    push_1.add_incoming(&book, &this);

                    self.builder.position_at_end(push_0.block);
                    let book_0 = self.build_stack_push(book, self.type_stackel.const_int(0, false));
                    self.builder.build_unconditional_branch(next.block);
                    next.add_incoming(&book_0, &push_0);

                    self.builder.position_at_end(push_1.block);
                    let book_1 = self.build_stack_push(book, self.type_stackel.const_int(1, false));
                    self.builder.build_unconditional_branch(next.block);
                    next.add_incoming(&book_1, &push_1);

                    continue;   // skip auto-generated jump to next instruction
                },
                Add => {
                    let (book, a) = self.build_stack_pop(book);
                    let (book, b) = self.build_stack_pop(book);
                    let c = self.builder.build_int_add(a, b, "");
                    let book = self.build_stack_push(book, c);
                    book
                },
                Sub => {
                    let (book, a) = self.build_stack_pop(book);
                    let (book, b) = self.build_stack_pop(book);
                    let c = self.builder.build_int_sub(a, b, "");
                    let book = self.build_stack_push(book, c);
                    book
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
                    
                    continue;   // skip auto-generated jump to next instruction
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
                        let cmp = self.builder.build_int_compare(IntPredicate::EQ, self.type_stackel.const_int(0, false), condition, "");
                        self.builder.build_conditional_branch(cmp, next.block, instructions[jmp_i].block);
                        next.add_incoming(&book, &this);
                        instructions[jmp_i].add_incoming(&book, &this);
                    }

                    continue;   // skip auto-generated jump to next instruction
                },

                _ => {
                    panic!("Op not implemented: {:?}", op);
                },
            };

            self.builder.build_unconditional_branch(next.block);
            next.add_incoming(&book, &this);
        }


        // OUTPUT LLVM
        self.module.print_to_stderr();


        // COMPILE
        let run_fn: JitFunction<JitEvmCompiledContract> = unsafe { self.execution_engine.get_function("executecontract")? };
        Ok(run_fn)
    }
}
