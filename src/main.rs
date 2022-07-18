use eyre::Result;
use std::time::Instant;
use primitive_types::U256;
use jitevm::code::{EvmCode, IndexedEvmCode, EvmOpParserMode};
use jitevm::interpreter::{EvmContext, EvmInnerContext, EvmOuterContext};
use jitevm::jit::{JitEvmEngine, JitEvmExecutionContext};
use jitevm::constants::EVM_STACK_SIZE;
use jitevm::test_data;
use std::error::Error;
use std::collections::HashMap;


fn main() -> Result<(), Box<dyn Error>> {


    let ops = test_data::get_code_ops_fibonacci();
    // let ops = test_data::get_code_ops_fibonacci_repetitions();
    // let ops = test_data::get_code_ops_supersimple1();
    // let ops = test_data::get_code_ops_supersimple2();
    // let ops = test_data::get_code_ops_storage1();
    // let ops = test_data::get_code_bin_revm_test1();

    // TESTING BASIC OPERATIONS WITH EVMOP AND EVMCODE

    let code = EvmCode { ops: ops.clone() };
    let augmented_code = code.augment();
    let _indexed_code = IndexedEvmCode::new_from_evmcode(augmented_code.clone());

    // println!("Code: {:?}", code);
    // println!("Augmented code: {:?}", augmented_code);
    // println!("Indexed code: {:?}", indexed_code);
    println!("Serialized code: {:?}", code.to_bytes());
    
    assert!(code.to_bytes() == augmented_code.to_bytes());
    assert!(code == EvmCode::new_from_bytes(&augmented_code.to_bytes(), EvmOpParserMode::Strict)?);


    let bcode = test_data::get_code_bin_revm_test1();
    let code = EvmCode::new_from_bytes(&bcode, EvmOpParserMode::Lax)?;
    // println!("Deserialized code: {:?}", code);
    let ops = code.clone().ops;

    use itertools::Itertools;
    println!("Unique instructions: {:?}", code.ops.iter().unique().sorted().collect::<Vec<&jitevm::code::EvmOp>>());



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
    println!("t={}: Context: {:?}", t, ctx);
    let measurement_now = Instant::now();
    loop {
        if !ctx.tick()? {
            break;
        };
        t += 1;
        // println!("t={}: Context: {:?}", t, ctx);
    }
    let measurement_runtime = measurement_now.elapsed();
    println!("t={}: Context: {:?}", t, ctx);
    println!("Runtime: {:.2?}", measurement_runtime);



    // TESTING JIT
    
    use inkwell::context::Context;
    let context = Context::create();
    let engine = JitEvmEngine::new_from_context(&context)?;
    // let fn_contract = engine.jit_compile_contract(&EvmCode { ops: ops.clone() }.augment().index())?;
    let fn_contract = engine.jit_compile_contract(&EvmCode { ops: ops.clone() }.index())?;

    println!("Benchmark compiled execution ...");
    for _i in 0..3 {
        let measurement_now = Instant::now();

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

        let ret = unsafe { fn_contract.call(&mut execution_context as *mut _ as usize) };
        let measurement_runtime = measurement_now.elapsed();

        println!("Ret: {:?}", ret);
        println!("Stack: {:?}", execution_context_stack);
        println!("Storage: {:?}", execution_context_storage);
        println!("Runtime: {:.2?}", measurement_runtime);
    }




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
