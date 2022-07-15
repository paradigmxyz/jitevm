use inkwell::OptimizationLevel;
use inkwell::AddressSpace;
use inkwell::context::Context;
use inkwell::execution_engine::JitFunction;
use inkwell::targets::{InitializationConfig, Target};
use inkwell::IntPredicate;
use inkwell::values::{FunctionValue, PointerValue, PhiValue, IntValue, BasicValue};
use inkwell::types::{IntType, PointerType};
use inkwell::basic_block::BasicBlock;
use inkwell::builder::Builder;
use inkwell::module::Module;


// LLVM HELPERS

fn build_stack_push(
    context: &Context,
    module: &Module,
    builder: &Builder,
    i64_type: IntType,
    inner_context_sp: PointerValue,
    inner_context_sp_offset: IntValue,
    val: IntValue)
{
    let sp_int = builder.build_load(inner_context_sp, "").into_int_value();
    let sp_ptr = builder.build_int_to_ptr(sp_int, i64_type.ptr_type(AddressSpace::Generic), "");
    builder.build_store(sp_ptr, val);
    builder.build_store(inner_context_sp, builder.build_int_add(sp_int, inner_context_sp_offset, ""));
}

fn build_stack_pop<'a>(
    context: &'a Context,
    module: &'a Module,
    builder: &'a Builder,
    i64_type: IntType<'a>,
    inner_context_sp: PointerValue<'a>,
    inner_context_sp_offset: IntValue<'a>) -> IntValue<'a>
{
    let sp_int = builder.build_load(inner_context_sp, "").into_int_value();
    let sp_int = builder.build_int_sub(sp_int, inner_context_sp_offset, "");
    builder.build_store(inner_context_sp, sp_int);
    let sp_ptr = builder.build_int_to_ptr(sp_int, i64_type.ptr_type(AddressSpace::Generic), "");
    let val = builder.build_load(sp_ptr, "").into_int_value();
    val
}

fn build_stack_write(
    context: &Context,
    module: &Module,
    builder: &Builder,
    i64_type: IntType,
    inner_context_sp: PointerValue,
    idx: IntValue,
    val: IntValue)
{
    let sp_int = builder.build_load(inner_context_sp, "").into_int_value();
    let sp_int = builder.build_int_sub(sp_int, idx, "");
    let sp_ptr = builder.build_int_to_ptr(sp_int, i64_type.ptr_type(AddressSpace::Generic), "");
    builder.build_store(sp_ptr, val);
}

fn build_stack_read<'a>(
    context: &'a Context,
    module: &'a Module,
    builder: &'a Builder,
    i64_type: IntType<'a>,
    inner_context_sp: PointerValue<'a>,
    idx: IntValue<'a>) -> IntValue<'a>
{
    let sp_int = builder.build_load(inner_context_sp, "").into_int_value();
    let sp_int = builder.build_int_sub(sp_int, idx, "");
    let sp_ptr = builder.build_int_to_ptr(sp_int, i64_type.ptr_type(AddressSpace::Generic), "");
    let val = builder.build_load(sp_ptr, "").into_int_value();
    val
}






    // LLVM code inspired by: https://github.com/mkeeter/advent-of-code/blob/master/2018/day21-jit/src/main.rs

    Target::initialize_native(&InitializationConfig::default())?;

    let context = Context::create();
    let module = context.create_module("jitevm-module");
    let builder = context.create_builder();
    let execution_engine = module.create_jit_execution_engine(OptimizationLevel::Aggressive)?;

    let void_type = context.void_type();
    let i64_type = context.i64_type();


    // //  Install our global callback into the system <------ later! code fragment from github repo above, will be useful to integrate with "outer" context of EVM
    // let i1_type = context.custom_width_int_type(1);
    // let cb_type = i1_type.fn_type(
    //     &[i64_type.array_type(6).ptr_type(AddressSpace::Generic).into()], false);
    // let cb_func = module.add_function("cb", cb_type, None);
    // execution_engine.add_global_mapping(&cb_func, callback as usize);


    let jitevm_fn_type = i64_type.fn_type(&[], false);
    let function = module.add_function("jitevm", jitevm_fn_type, None);


    // SETUP

    let block_setup = context.append_basic_block(function, "setup");
    builder.position_at_end(block_setup);

    let inner_context_stack = builder.build_alloca(i64_type.array_type(1024), "stack");
    let inner_context_sp = builder.build_alloca(i64_type, "sp");
    let inner_context_sp_offset = i64_type.const_int(8, false);   // stack elements are 8 bytes for now
    builder.build_store(inner_context_sp, builder.build_ptr_to_int(inner_context_stack, i64_type, ""));


    // INSTRUCTIONS

    assert!(ops.len() > 0);

    let mut block_instructions = Vec::new();
    for i in 0..ops.len() {
        block_instructions.push(context.insert_basic_block_after(if i == 0 { block_setup } else { block_instructions[i-1] }, &format!("instruction #{}: {:?}", i, ops[i])));
    }

    builder.position_at_end(block_setup);
    builder.build_unconditional_branch(block_instructions[0]);


    // ERROR HANDLER

    let block_error = context.append_basic_block(function, "error");
    builder.position_at_end(block_error);

    let val = i64_type.const_int(u64::MAX, false);
    builder.build_return(Some(&val));


    // ANALYZE JUMP TARGETS

    let mut jump_targets = Vec::new();
    for (i, op) in ops.iter().enumerate() {
        use EvmOp::*;

        match op {
            Jumpdest => {
                let code = EvmCode { ops: ops.clone() };
                jump_targets.push((i, code.target_for_opidx(i)));
            },
            _ => {},
        }
    }
    println!("Jump targets: {:?}", jump_targets);


    // RENDER INSTRUCTIONS

    for (i, op) in ops.iter().enumerate() {
        use EvmOp::*;
        builder.position_at_end(block_instructions[i]);

        match op {
            Stop => {
                let val = build_stack_pop(&context, &module, &builder, i64_type, inner_context_sp, inner_context_sp_offset);
                builder.build_return(Some(&val));
            },
            Push(_, val) => {
                let val = i64_type.const_int(val.as_u64(), false);
                build_stack_push(&context, &module, &builder, i64_type, inner_context_sp, inner_context_sp_offset, val);
                builder.build_unconditional_branch(block_instructions[i+1]);
            },
            Pop => {
                build_stack_pop(&context, &module, &builder, i64_type, inner_context_sp, inner_context_sp_offset);
                builder.build_unconditional_branch(block_instructions[i+1]);
            },
            Jumpdest => {
                builder.build_unconditional_branch(block_instructions[i+1]);
            },
            Jump => {
                let target = build_stack_pop(&context, &module, &builder, i64_type, inner_context_sp, inner_context_sp_offset);

                if jump_targets.is_empty() {
                    // there are no valid jump targets, this Jump has to fail!
                    builder.build_unconditional_branch(block_error);

                } else {
                    let mut jump_table = Vec::new();
                    for (j, (jmp_i, jmp_target)) in jump_targets.iter().enumerate() {
                        jump_table.push(context.insert_basic_block_after(if j == 0 { block_instructions[i] } else { jump_table[j-1] }, &format!("instruction #{}: {:?} / to Jumpdest #{} at op #{} to byte #{}", i, ops[i], j, jmp_i, jmp_target)));
                    }

                    builder.build_unconditional_branch(jump_table[0]);

                    for (j, (jmp_i, jmp_target)) in jump_targets.iter().enumerate() {
                        let jmp_target = jmp_target.as_u64();
                        builder.position_at_end(jump_table[j]);
                        let cmp = builder.build_int_compare(IntPredicate::EQ, i64_type.const_int(jmp_target, false), target, "");
                        builder.build_conditional_branch(cmp, block_instructions[*jmp_i], if j+1 == jump_targets.len() { block_error } else { jump_table[j+1] });
                    }
                }
            },
            Jumpi => {
                let target = build_stack_pop(&context, &module, &builder, i64_type, inner_context_sp, inner_context_sp_offset);
                let val = build_stack_pop(&context, &module, &builder, i64_type, inner_context_sp, inner_context_sp_offset);

                if jump_targets.is_empty() {
                    // there are no valid jump targets, this Jumpi has to fail!
                    builder.build_unconditional_branch(block_error);

                } else {
                    let block_jump_no = context.insert_basic_block_after(block_instructions[i], &format!("instruction #{}: {:?} / jump no", i, ops[i]));
                    let block_jump_yes = context.insert_basic_block_after(block_jump_no, &format!("instruction #{}: {:?} / jump yes", i, ops[i]));

                    let mut jump_table = Vec::new();
                    for (j, (jmp_i, jmp_target)) in jump_targets.iter().enumerate() {
                        jump_table.push(context.insert_basic_block_after(if j == 0 { block_jump_yes } else { jump_table[j-1] }, &format!("instruction #{}: {:?} / to Jumpdest #{} at op #{} to byte #{}", i, ops[i], j, jmp_i, jmp_target)));
                    }

                    let cmp = builder.build_int_compare(IntPredicate::EQ, i64_type.const_int(0, false), val, "");
                    builder.build_conditional_branch(cmp, block_jump_no, block_jump_yes);

                    builder.position_at_end(block_jump_no);
                    builder.build_unconditional_branch(block_instructions[i+1]);

                    builder.position_at_end(block_jump_yes);
                    builder.build_unconditional_branch(jump_table[0]);

                    for (j, (jmp_i, jmp_target)) in jump_targets.iter().enumerate() {
                        let jmp_target = jmp_target.as_u64();
                        builder.position_at_end(jump_table[j]);
                        let cmp = builder.build_int_compare(IntPredicate::EQ, i64_type.const_int(jmp_target, false), target, "");
                        builder.build_conditional_branch(cmp, block_instructions[*jmp_i], if j+1 == jump_targets.len() { block_error } else { jump_table[j+1] });
                    }
                }
            },
            Swap1 => {
                let idx_a = i64_type.const_int(1*8, false);
                let idx_b = i64_type.const_int(2*8, false);
                let a = build_stack_read(&context, &module, &builder, i64_type, inner_context_sp, idx_a);
                let b = build_stack_read(&context, &module, &builder, i64_type, inner_context_sp, idx_b);
                build_stack_write(&context, &module, &builder, i64_type, inner_context_sp, idx_a, b);
                build_stack_write(&context, &module, &builder, i64_type, inner_context_sp, idx_b, a);
                builder.build_unconditional_branch(block_instructions[i+1]);
            },
            Swap2 => {
                let idx_a = i64_type.const_int(1*8, false);
                let idx_b = i64_type.const_int(3*8, false);
                let a = build_stack_read(&context, &module, &builder, i64_type, inner_context_sp, idx_a);
                let b = build_stack_read(&context, &module, &builder, i64_type, inner_context_sp, idx_b);
                build_stack_write(&context, &module, &builder, i64_type, inner_context_sp, idx_a, b);
                build_stack_write(&context, &module, &builder, i64_type, inner_context_sp, idx_b, a);
                builder.build_unconditional_branch(block_instructions[i+1]);
            },
            Dup2 => {
                let idx = i64_type.const_int(2*8, false);
                let val = build_stack_read(&context, &module, &builder, i64_type, inner_context_sp, idx);
                build_stack_push(&context, &module, &builder, i64_type, inner_context_sp, inner_context_sp_offset, val);
                builder.build_unconditional_branch(block_instructions[i+1]);
            },
            Dup3 => {
                let idx = i64_type.const_int(3*8, false);
                let val = build_stack_read(&context, &module, &builder, i64_type, inner_context_sp, idx);
                build_stack_push(&context, &module, &builder, i64_type, inner_context_sp, inner_context_sp_offset, val);
                builder.build_unconditional_branch(block_instructions[i+1]);
            },
            Dup4 => {
                let idx = i64_type.const_int(4*8, false);
                let val = build_stack_read(&context, &module, &builder, i64_type, inner_context_sp, idx);
                build_stack_push(&context, &module, &builder, i64_type, inner_context_sp, inner_context_sp_offset, val);
                builder.build_unconditional_branch(block_instructions[i+1]);
            },
            Iszero => {
                let val = build_stack_pop(&context, &module, &builder, i64_type, inner_context_sp, inner_context_sp_offset);

                let block_push_0 = context.insert_basic_block_after(block_instructions[i], &format!("instruction #{}: {:?} / push 0", i, ops[i]));
                let block_push_1 = context.insert_basic_block_after(block_push_0, &format!("instruction #{}: {:?} / push 1", i, ops[i]));

                let cmp = builder.build_int_compare(IntPredicate::EQ, i64_type.const_int(0, false), val, "");
                builder.build_conditional_branch(cmp, block_push_1, block_push_0);

                builder.position_at_end(block_push_0);
                build_stack_push(&context, &module, &builder, i64_type, inner_context_sp, inner_context_sp_offset, i64_type.const_int(0, false));
                builder.build_unconditional_branch(block_instructions[i+1]);

                builder.position_at_end(block_push_1);
                build_stack_push(&context, &module, &builder, i64_type, inner_context_sp, inner_context_sp_offset, i64_type.const_int(1, false));
                builder.build_unconditional_branch(block_instructions[i+1]);
            },
            Add => {
                let a = build_stack_pop(&context, &module, &builder, i64_type, inner_context_sp, inner_context_sp_offset);
                let b = build_stack_pop(&context, &module, &builder, i64_type, inner_context_sp, inner_context_sp_offset);
                let c = builder.build_int_add(a, b, "");
                build_stack_push(&context, &module, &builder, i64_type, inner_context_sp, inner_context_sp_offset, c);
                builder.build_unconditional_branch(block_instructions[i+1]);
            },
            Sub => {
                let a = build_stack_pop(&context, &module, &builder, i64_type, inner_context_sp, inner_context_sp_offset);
                let b = build_stack_pop(&context, &module, &builder, i64_type, inner_context_sp, inner_context_sp_offset);
                let c = builder.build_int_sub(a, b, "");
                build_stack_push(&context, &module, &builder, i64_type, inner_context_sp, inner_context_sp_offset, c);
                builder.build_unconditional_branch(block_instructions[i+1]);
            },


            DetectedPushJump(_, val) => {
                if jump_targets.is_empty() {
                    // there are no valid jump targets, this Jump has to fail!
                    builder.build_unconditional_branch(block_error);
                } else {
                    // retrieve the corresponding jump target (panic if not a valid jump target) ...
                    let (jmp_i, _) = jump_targets.iter().find(|e| e.1 == *val).unwrap();
                    // ... and jump to there!
                    builder.build_unconditional_branch(block_instructions[*jmp_i]);
                }
            },
            DetectedPushJumpi(_, val) => {
                let condition = build_stack_pop(&context, &module, &builder, i64_type, inner_context_sp, inner_context_sp_offset);

                if jump_targets.is_empty() {
                    // there are no valid jump targets, this Jumpi has to fail!
                    builder.build_unconditional_branch(block_error);

                } else {
                    let block_jump_no = context.insert_basic_block_after(block_instructions[i], &format!("instruction #{}: {:?} / jump no", i, ops[i]));
                    let block_jump_yes = context.insert_basic_block_after(block_jump_no, &format!("instruction #{}: {:?} / jump yes", i, ops[i]));

                    let cmp = builder.build_int_compare(IntPredicate::EQ, i64_type.const_int(0, false), condition, "");
                    builder.build_conditional_branch(cmp, block_jump_no, block_jump_yes);

                    builder.position_at_end(block_jump_no);
                    builder.build_unconditional_branch(block_instructions[i+1]);

                    builder.position_at_end(block_jump_yes);
                    // retrieve the corresponding jump target (panic if not a valid jump target) ...
                    let (jmp_i, _) = jump_targets.iter().find(|e| e.1 == *val).unwrap();
                    // ... and jump to there!
                    builder.build_unconditional_branch(block_instructions[*jmp_i]);
                }
            },


            _ => {
                panic!("Op not implemented: {:?}", op);
            },
        }
    }


    // OUTPUT LLVM
    module.print_to_stderr();

    println!("Compiling...");
    type RunFunction = unsafe extern "C" fn() -> i64;
    let run_fn: JitFunction<RunFunction> = unsafe { execution_engine.get_function("jitevm")? };

    println!("Running...");
    for i in 0..10 {
        let measurement_now = Instant::now();
        let ret = unsafe { run_fn.call() };
        let measurement_runtime = measurement_now.elapsed();
        println!("Ret: {:?}", ret);
        println!("Runtime: {:.2?}", measurement_runtime);
    }

    return Ok(());