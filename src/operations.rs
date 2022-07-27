use primitive_types::U256;
use revm::instructions::{arithmetic, bitwise};
use std::ops::{BitAnd, BitOr, BitXor};

macro_rules! op1_u256_fn {
    ($fname:ident, $fn:expr) => {
        #[allow(non_snake_case)]
        pub fn $fname(a: U256) -> U256 {
            let ret = $fn(a);
            ret
        }
    };
}

macro_rules! op2_u256_method {
    ($fname:ident, $method:ident) => {
        #[allow(non_snake_case)]
        pub fn $fname(a: U256, b: U256) -> U256 {
            let ret = a.$method(b);
            ret
        }
    };
}

macro_rules! op2_u256_method_ret_tuple {
    ($fname:ident, $method:ident) => {
        #[allow(non_snake_case)]
        pub fn $fname(a: U256, b: U256) -> U256 {
            let (ret, _) = a.$method(b);
            ret
        }
    };
}

macro_rules! op2_u256_method_ref_ret_bool {
    ($fname:ident, $method:ident) => {
        #[allow(non_snake_case)]
        pub fn $fname(a: U256, b: U256) -> U256 {
            let ret = a.$method(&b);
            let ret = if ret { U256::one() } else { U256::zero() };
            ret
        }
    };
}

macro_rules! op2_u256_fn {
    ($fname:ident, $fn:expr) => {
        #[allow(non_snake_case)]
        pub fn $fname(a: U256, b: U256) -> U256 {
            let ret = $fn(a, b);
            ret
        }
    };
}

macro_rules! op3_u256_fn {
    ($fname:ident, $fn:expr) => {
        #[allow(non_snake_case)]
        pub fn $fname(a: U256, b: U256, c: U256) -> U256 {
            let ret = $fn(a, b, c);
            ret
        }
    };
}

op2_u256_method_ret_tuple!(Add, overflowing_add);
op2_u256_method_ret_tuple!(Mul, overflowing_mul);
op2_u256_method_ret_tuple!(Sub, overflowing_sub);
op2_u256_fn!(Exp, arithmetic::exp);
op2_u256_fn!(Div, arithmetic::div);
op2_u256_fn!(Sdiv, arithmetic::sdiv);
op2_u256_fn!(Mod, arithmetic::rem);
op2_u256_fn!(Smod, arithmetic::smod);
op3_u256_fn!(Addmod, arithmetic::addmod);
op3_u256_fn!(Mulmod, arithmetic::mulmod);
op2_u256_fn!(Slt, bitwise::slt);
op2_u256_fn!(Sgt, bitwise::sgt);
op1_u256_fn!(Iszero, bitwise::iszero);
op1_u256_fn!(Not, bitwise::not);
op2_u256_fn!(Byte, bitwise::byte);
op2_u256_fn!(Shl, bitwise::shl);
op2_u256_fn!(Shr, bitwise::shr);
op2_u256_fn!(Sar, bitwise::sar);
op2_u256_method!(And, bitand);
op2_u256_method!(Or, bitor);
op2_u256_method!(Xor, bitxor);
op2_u256_fn!(Signextend, arithmetic::signextend);
op2_u256_method_ref_ret_bool!(Lt, lt);
op2_u256_method_ref_ret_bool!(Gt, gt);
op2_u256_method_ref_ret_bool!(Eq, eq);
