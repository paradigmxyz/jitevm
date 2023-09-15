use eyre::Result;
use jitevm::code::{EvmCode, EvmOpParserMode, IndexedEvmCode};
use jitevm::constants::EVM_STACK_SIZE;
use jitevm::interpreter::{EvmContext, EvmInnerContext, EvmOuterContext};
use jitevm::jit::{JitEvmEngine, JitEvmExecutionContext};
use jitevm::test_data;
use primitive_types::U256;

use std::collections::HashMap;
use std::error::Error;
use std::time::Instant;

fn main() -> Result<(), Box<dyn Error>> {
    let ops = test_data::get_code_ops_fibonacci();
    // let ops = test_data::get_code_ops_fibonacci_repetitions();
    // let ops = test_data::get_code_ops_supersimple1();
    // let ops = test_data::get_code_ops_supersimple2();
    // let ops = test_data::get_code_ops_storage1();
    // let ops = test_data::get_code_ops_mstore_mload();
    // let ops = test_data::get_code_bin_revm_test1();

    // TESTING BASIC OPERATIONS WITH EVMOP AND EVMCODE

    let code = EvmCode { ops: ops.clone() };
    let augmented_code = code.augment();
    let indexed_code = IndexedEvmCode::new_from_evmcode(augmented_code.clone());

    println!("Code: {:?}", code);
    println!("Augmented code: {:?}", augmented_code);
    println!("Indexed code: {:?}", indexed_code);
    println!("Serialized code: {:?}", code.to_bytes());
    println!("Serialized code (hex): {:?}", hex::encode(code.to_bytes()));

    assert!(code.to_bytes() == augmented_code.to_bytes());
    assert!(code == EvmCode::new_from_bytes(&augmented_code.to_bytes(), EvmOpParserMode::Strict)?);

    let bcode = test_data::get_code_bin_revm_test1();
    let code = EvmCode::new_from_bytes(&bcode, EvmOpParserMode::Lax)?;
    // println!("Deserialized code: {:?}", code);
    // let ops = code.clone().ops;    
    assert!(code.to_bytes() == bcode);

    use itertools::Itertools;
    println!(
        "Unique instructions: {:?}",
        code.ops
            .iter()
            .unique()
            .sorted()
            .collect::<Vec<&jitevm::code::EvmOp>>()
    );

    // TESTING EVMINTERPRETER

    let mut ctx = EvmContext {
        outer: EvmOuterContext {
            calldata: hex::decode("30627b7c").unwrap().into(),
            // returndata: vec![],
            storage: HashMap::new(),
            callvalue: U256::zero(),
        },
        inner: EvmInnerContext {
            code: &EvmCode { ops: ops.clone() }.index(),
            stack: [0.into(); EVM_STACK_SIZE],
            pc: 0,
            sp: 0,
            // gas: 0,
            memory: vec![],
        },
    };

    println!("Benchmarking interpreted execution ...");
    let mut t = 0;
    // println!("t={}: Context: {:?}", t, ctx);
    let measurement_now = Instant::now();
    loop {
        // let ctx_pre = ctx.clone();
        match ctx.tick() {
            Ok(false) => {
                break;
            }
            Ok(true) => {}
            Err(e) => {
                println!("Interpreter error at t={}: {:?}: {}", t, e, e);
                // println!("Pre-context: {:?}", ctx_pre);
                println!("Context: {:?}", ctx);
                break;
            }
        }
        // if !ctx.tick()? {
        //     break;
        // };
        t += 1;
        // println!("t={}: Context: {:?}", t, ctx);
    }
    let measurement_revm_interpreter = measurement_now.elapsed();
    println!("t={}: Context: {:?}", t, ctx);
    println!("Ret: {:?}", ctx.inner.stack[0]);
    println!("Runtime: {:.2?}", measurement_revm_interpreter);

    // TESTING JIT

    use inkwell::context::Context;
    let context = Context::create();
    let engine = JitEvmEngine::new_from_context(&context)?;
    // let fn_contract = engine.jit_compile_contract(&EvmCode { ops: ops.clone() }.augment().index())?;
    let fn_contract = engine.jit_compile_contract(&EvmCode { ops: ops.clone() }.augment().index(), Some("jit_main.ll".to_string()), Some("jit_main.asm".to_string()))?;

    println!("Benchmark compiled execution ...");
    let mut execution_context_stack = [U256::zero(); 1024];
    // TODO: at maximum block size of 30M gas, max memory size is 123169 words = ~128000 words = 4096000 bytes
    let mut execution_context_memory = [0u8; 4096000];
    let mut execution_context_storage = HashMap::<U256, U256>::new();

    let mut execution_context = JitEvmExecutionContext {
        stack: &mut execution_context_stack as *mut _ as usize,
        memory: &mut execution_context_memory as *mut _ as usize,
        storage: &mut execution_context_storage as *mut _ as usize,
    };
    println!("INPUT: {:?}", execution_context.clone());

    let measurement_now = Instant::now();
    let context_ptr = &mut execution_context as *mut _ as usize;
    println!("Context ptr: {:x}", context_ptr);
    println!("Stack ptr: {:x}", execution_context.stack);
    println!("Memory ptr: {:x}", execution_context.memory);

    let _ret = unsafe { fn_contract.call(context_ptr) };
    let measurement_llvm_execution = measurement_now.elapsed();

    println!("Ret: {:?}", execution_context_stack[0]);
    println!("Stack: {:?}", execution_context_stack);
    println!("Runtime: {:.2?}", measurement_llvm_execution);

    println!("Speedup: {:.2}x", measurement_revm_interpreter.as_secs_f64() / measurement_llvm_execution.as_secs_f64());

    // TESTING AOT-COMPILED EVM

    let ctx_raw = EvmContext {
        outer: EvmOuterContext {
            calldata: hex::decode("30627b7c").unwrap().into(),
            // returndata: vec![],
            storage: HashMap::new(),
            callvalue: U256::zero(),
        },
        inner: EvmInnerContext {
            code: &EvmCode { ops: ops.clone() }.index(),
            stack: [0.into(); EVM_STACK_SIZE],
            pc: 0,
            sp: 0,
            // gas: 0,
            memory: vec![],
        },
    };

    // println!("Benchmarking interpreted execution ...");
    // let mut t = 0;
    // println!("t={}: Context: {:?}", t, ctx);

    macro_rules! exec_aot_evmop {
        ($ctx:expr, $_aot_ret:ident, $pc:expr, $op:expr) => {
            $ctx.inner.pc = $pc;
            match $ctx.tick_inner_simplified($op) {
                Ok(true) => {},
                Ok(false) => { $_aot_ret = 0; break; },
                Err(_) => { $_aot_ret = 1; break; },
            }
        };
    }

    let mut ctx = ctx_raw.clone();

    enum AotJumpDest {
        JumpDest0,
        JumpDest1,
        JumpDest2,
    }
    let mut _jumpdest = AotJumpDest::JumpDest0;
    let mut _aot_ret: usize = 0;

    let measurement_now = Instant::now();
    loop {
        use jitevm::code::EvmOp::*;
        match _jumpdest {
            AotJumpDest::JumpDest0 => {
                exec_aot_evmop!(ctx, _aot_ret, 0, Push(2, U256::zero() + 10000));
                exec_aot_evmop!(ctx, _aot_ret, 3, Push(1, U256::zero()));
                exec_aot_evmop!(ctx, _aot_ret, 5, Push(1, U256::one()));
                _jumpdest = AotJumpDest::JumpDest1;
                continue;
            },
            AotJumpDest::JumpDest1 => {
                // exec_aot_evmop!(ctx, _aot_ret, 7, Jumpdest);   // op 3 code 7
                exec_aot_evmop!(ctx, _aot_ret, 8, Dup3);
                exec_aot_evmop!(ctx, _aot_ret, 9, Iszero);
                exec_aot_evmop!(ctx, _aot_ret, 10, Push(1, U256::zero() + 28));
                exec_aot_evmop!(ctx, _aot_ret, 12, Jumpi);
                if ctx.inner.pc == 7 {
                    _jumpdest = AotJumpDest::JumpDest1;
                    continue;
                } else if ctx.inner.pc == 28 {
                    _jumpdest = AotJumpDest::JumpDest2;
                    continue;
                } else if ctx.inner.pc == 12 {
                    // no jump occurred - TODO: this needs fixing
                } else {
                    _aot_ret = 2;
                    break;
                }
                exec_aot_evmop!(ctx, _aot_ret, 13, Dup2);
                exec_aot_evmop!(ctx, _aot_ret, 14, Dup2);
                exec_aot_evmop!(ctx, _aot_ret, 15, Add);
                exec_aot_evmop!(ctx, _aot_ret, 16, Swap2);
                exec_aot_evmop!(ctx, _aot_ret, 17, Pop);
                exec_aot_evmop!(ctx, _aot_ret, 18, Swap1);
                exec_aot_evmop!(ctx, _aot_ret, 19, Swap2);
                exec_aot_evmop!(ctx, _aot_ret, 20, Push(1, U256::one()));
                exec_aot_evmop!(ctx, _aot_ret, 22, Swap1);
                exec_aot_evmop!(ctx, _aot_ret, 23, Sub);
                exec_aot_evmop!(ctx, _aot_ret, 24, Swap2);
                exec_aot_evmop!(ctx, _aot_ret, 25, Push(1, U256::zero() + 7));
                exec_aot_evmop!(ctx, _aot_ret, 27, Jump);
                if ctx.inner.pc == 7 {
                    _jumpdest = AotJumpDest::JumpDest1;
                    continue;
                } else if ctx.inner.pc == 28 {
                    _jumpdest = AotJumpDest::JumpDest2;
                    continue;
                } else {
                    _aot_ret = 2;
                    break;
                }
            },
            AotJumpDest::JumpDest2 => {
                // exec_aot_evmop!(ctx, _aot_ret, 28, Jumpdest);   // op 21 code 28
                exec_aot_evmop!(ctx, _aot_ret, 29, Swap2);
                exec_aot_evmop!(ctx, _aot_ret, 30, Pop);
                exec_aot_evmop!(ctx, _aot_ret, 31, Pop);
                exec_aot_evmop!(ctx, _aot_ret, 32, Stop);
            },
        }
    }

    let measurement_runtime = measurement_now.elapsed();
    println!("{} -> Context: {:?}", _aot_ret, ctx);
    println!("Runtime: {:.2?}", measurement_runtime);

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

    // // // BenchmarkDB is dummy state that implements Database trait.
    // // let mut evm = revm::new();
    // // evm.database(BenchmarkDB(contract_data));

    // // // execution globals block hash/gas_limit/coinbase/timestamp..
    // // evm.env.tx.caller = H160::from_str("0x1000000000000000000000000000000000000000").unwrap();
    // // evm.env.tx.transact_to =
    // //     TransactTo::Call(H160::from_str("0x0000000000000000000000000000000000000000").unwrap());
    // // evm.env.tx.data = Bytes::from(hex::decode("30627b7c").unwrap());

    Ok(())
}
