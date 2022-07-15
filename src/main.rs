use eyre::Result;
use std::time::Instant;
use jitevm::code::{EvmCode, IndexedEvmCode, EvmOpParserMode};
use jitevm::interpreter::{EvmContext, EvmInnerContext, EvmOuterContext, EVM_STACK_SIZE};
use jitevm::test_data;


fn main() -> Result<()> {


    let ops = test_data::get_code_ops_fibonacci();

    // // TESTING BASIC OPERATIONS WITH EVMOP AND EVMCODE

    // let code = EvmCode { ops: ops.clone() };
    // let augmented_code = code.augment();
    // let indexed_code = IndexedEvmCode::new_from_evmcode(augmented_code.clone());

    // // println!("Code: {:?}", code);
    // // println!("Augmented code: {:?}", augmented_code);
    // // println!("Indexed code: {:?}", indexed_code);
    // // println!("Serialized code: {:?}", code.to_bytes());
    
    // assert!(code.to_bytes() == augmented_code.to_bytes());
    // assert!(code == EvmCode::new_from_bytes(&augmented_code.to_bytes(), EvmOpParserMode::Strict)?);


    // let bcode = test_data::get_code_bin_revm_test1();
    // let code = EvmCode::new_from_bytes(&bcode, EvmOpParserMode::Lax)?;
    // // println!("Deserialized code: {:?}", code);

    // // use itertools::Itertools;
    // // println!("Unique instructions: {:?}", code.ops.iter().unique().sorted().collect::<Vec<&jitevm::code::EvmOp>>());



    // TESTING EVMINTERPRETER

    let mut ctx = EvmContext {
        outer: EvmOuterContext {
            memory: vec![],
            calldata: hex::decode("30627b7c").unwrap().into(),
            returndata: vec![],
        },
        inner: EvmInnerContext {
            code: &EvmCode { ops: ops.clone() }.index(),
            stack: [0.into(); EVM_STACK_SIZE],
            pc: 0,
            sp: 0,
            gas: 0,
        },
    };

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

    Ok(())




    


    // // // BenchmarkDB is dummy state that implements Database trait.
    // // let mut evm = revm::new();
    // // evm.database(BenchmarkDB(contract_data));

    // // // execution globals block hash/gas_limit/coinbase/timestamp..
    // // evm.env.tx.caller = H160::from_str("0x1000000000000000000000000000000000000000").unwrap();
    // // evm.env.tx.transact_to =
    // //     TransactTo::Call(H160::from_str("0x0000000000000000000000000000000000000000").unwrap());
    // // evm.env.tx.data = Bytes::from(hex::decode("30627b7c").unwrap());

}
