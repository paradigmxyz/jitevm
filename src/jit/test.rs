use paste::paste;
use rand::Rng;
use primitive_types::U256;
use crate::{code::EvmOp, jit::JitEvmExecutionContext};

fn run_jit_ops(len: usize, ops: Vec<EvmOp>) -> Vec<U256> {
    use crate::jit::{JitEvmExecutionContextHolder, JitEvmEngine};
    use crate::code::{EvmCode};
    use inkwell::context::Context;

    let context = Context::create();
    let engine = JitEvmEngine::new_from_context(&context).unwrap();

    let mut holder = JitEvmExecutionContextHolder::new_from_empty();
    let mut ctx = JitEvmExecutionContext::new_from_holder(&mut holder);
    let fn_contract = engine.jit_compile_contract(&EvmCode { ops: ops.clone() }.index(), Some("jit_test.ll".to_string()), Some("jit_test.asm".to_string())).unwrap();
    let ret = unsafe { fn_contract.call(&mut ctx as *mut _ as usize) };

    holder.stack[..len].to_vec()
}

macro_rules! test_op1 {
    ($fname:ident, $evmop:expr, $opname:expr) => {
        paste! {
            #[test]
            fn [<operations_jit_equivalence_ $fname>]() {
                use crate::code::EvmOp::*;
                use crate::operations;

                fn _test(a: U256) {
                    let d = run_jit_ops(1, vec![
                        Push(32, a),
                        $evmop,
                    ]);
                    let d = d[0];
                    let d_ = $opname(a);
                    if d != d_ {
                        println!("a = {:?} / d = {:?} / d' = {:?}", a, d, d_);
                    }
                    assert_eq!(d, d_);
                }

                _test(U256::zero());
                _test(U256::one());

                for _i in 0..1000 {
                    let a = rand::thread_rng().gen::<[u8; 32]>();
                    let a = U256::from_big_endian(&a);
                    _test(a);
                }
            }
        }
    };
}

macro_rules! test_op2 {
    ($fname:ident, $evmop:expr, $opname:expr) => {
        paste! {
            #[test]
            fn [<operations_jit_equivalence_ $fname>]() {
                use crate::code::EvmOp::*;
                use crate::operations;

                fn _test(a: U256, b: U256) {
                    let d = run_jit_ops(1, vec![
                        Push(32, b),
                        Push(32, a),
                        $evmop,
                    ]);
                    let d = d[0];
                    let d_ = $opname(a, b);
                    if d != d_ {
                        println!("a = {:?} / b = {:?} / d = {:?} / d' = {:?}", a, b, d, d_);
                    }
                    assert_eq!(d, d_);
                }

                _test(U256::zero(), U256::zero());
                _test(U256::zero(), U256::one());
                _test(U256::one(), U256::zero());
                _test(U256::one(), U256::one());

                for _i in 0..1000 {
                    let a = rand::thread_rng().gen::<[u8; 32]>();
                    let b = rand::thread_rng().gen::<[u8; 32]>();
                    let a = U256::from_big_endian(&a);
                    let b = U256::from_big_endian(&b);
                    _test(a, b);
                }
            }
        }
    };
}


test_op1!(iszero, EvmOp::Iszero, operations::Iszero);
test_op2!(add, EvmOp::Add, operations::Add);
test_op2!(sub, EvmOp::Sub, operations::Sub);
test_op2!(mul, EvmOp::Mul, operations::Mul);
test_op2!(div, EvmOp::Div, operations::Div);
test_op2!(sdiv, EvmOp::Sdiv, operations::Sdiv);
test_op2!(mod, EvmOp::Mod, operations::Mod);
test_op2!(eq, EvmOp::Eq, operations::Eq);
test_op2!(lt, EvmOp::Lt, operations::Lt);
test_op2!(gt, EvmOp::Gt, operations::Gt);
test_op2!(slt, EvmOp::Slt, operations::Slt);
test_op2!(sgt, EvmOp::Sgt, operations::Sgt);
test_op2!(and, EvmOp::And, operations::And);
test_op2!(or, EvmOp::Or, operations::Or);
// test_op2!(xor, EvmOp::Xor, operations::Xor);
test_op1!(not, EvmOp::Not, operations::Not);
