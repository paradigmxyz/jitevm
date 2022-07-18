use thiserror::Error;
use primitive_types::U256;
use std::collections::{HashMap, HashSet};


#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EvmOp {
    Stop,
    Add,
    Mul,
    Sub,
    Div,
    Sdiv,
    Mod,
    // Smod,
    // Addmod,
    // Mulmod,
    Exp,
    // Signextend,
    Lt,
    Gt,
    Slt,
    Sgt,
    Eq,
    Iszero,
    And,
    Or,
    // Xor,
    Not,   // 0x19 = 25
    // Byte,
    Shl,
    Shr,
    // Sar,
    Sha3,   // 0x20 = 32
    // Address,
    // Balance,
    Origin,

    Caller,
    Callvalue,
    Calldataload,
    Calldatasize,

    Pop,

    Mload,
    Mstore,
    Mstore8,
    Sload,
    Sstore,
    Jump,
    Jumpi,
    Jumpdest,

    Push(usize, U256),
    Dup1,
    Dup2,
    Dup3,
    Dup4,
    Dup5,
    Dup6,
    Dup7,
    Dup8,
    Dup9,
    Dup10,
    Dup11,
    Dup12,
    Dup13,
    Dup14,
    Dup15,
    Dup16,
    Swap1,
    Swap2,
    Swap3,
    Swap4,
    Swap5,
    Swap6,
    Swap7,
    Swap8,
    Swap9,
    Swap10,
    Swap11,
    Swap12,
    Swap13,
    Swap14,
    Swap15,
    Swap16,

    Log2,
    Return,

    Revert,
    Invalid,
    // Selfdestruct,

    AugmentedPushJump(usize, U256),
    AugmentedPushJumpi(usize, U256),

    Unknown(u8),
}

#[derive(Error, Debug)]
pub enum EvmOpError {
    #[error("parser error: incomplete instruction")]
    ParserErrorIncompleteInstruction,
    #[error("parser error: unknown instruction")]
    ParserErrorUnknownInstruction(u8),
}

#[derive(Debug, Clone, Copy)]
pub enum EvmOpParserMode {
    Lax,
    Strict,
}

impl EvmOp {
    pub fn len(&self) -> usize {
        use EvmOp::*;

        match self {
            Push(len, _) => 1 + len,
            AugmentedPushJump(len, _) => 1 + len + 1,
            AugmentedPushJumpi(len, _) => 1 + len + 1,
            Unknown(_) => 1,
            _ => 1,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        use EvmOp::*;

        match self {
            Stop => vec![0x00],
            Add => vec![0x01],
            Mul => vec![0x02],
            Sub => vec![0x03],
            Div => vec![0x04],
            Sdiv => vec![0x05],
            Mod => vec![0x06],
            Exp => vec![0x0a],
            Lt => vec![0x10],
            Gt => vec![0x11],
            Slt => vec![0x12],
            Sgt => vec![0x13],
            Eq => vec![0x14],
            Iszero => vec![0x15],
            And => vec![0x16],
            Or => vec![0x17],
            Not => vec![0x19],
            Shl => vec![0x1b],
            Shr => vec![0x1c],
            // Sar => vec![0x1d],
            Sha3 => vec![0x20],

            // Address => vec![0x30],
            Origin => vec![0x32],
            Caller => vec![0x33],
            Callvalue => vec![0x34],
            Calldataload => vec![0x35],
            Calldatasize => vec![0x36],

            Pop => vec![0x50],
            Mload => vec![0x51],
            Mstore => vec![0x52],
            Mstore8 => vec![0x53],
            Sload => vec![0x54],
            Sstore => vec![0x55],
            Jump => vec![0x56],
            Jumpi => vec![0x57],
            Jumpdest => vec![0x5b],

            Push(len, val) => {
                assert!(*len >= 1);
                assert!(*len <= 32);

                let mut v = [0u8; 32];
                val.to_big_endian(&mut v);
                let mut w = vec![0x60 + (len - 1) as u8];
                w.append(&mut v[32-len..32].to_vec());
                w
            },
            Dup1 => vec![0x80],
            Dup2 => vec![0x81],
            Dup3 => vec![0x82],
            Dup4 => vec![0x83],
            Dup5 => vec![0x84],
            Dup6 => vec![0x85],
            Dup7 => vec![0x86],
            Dup8 => vec![0x87],
            Dup9 => vec![0x88],
            Dup10 => vec![0x89],
            Dup11 => vec![0x8a],
            Dup12 => vec![0x8b],
            Dup13 => vec![0x8c],
            Dup14 => vec![0x8d],
            Dup15 => vec![0x8e],
            Dup16 => vec![0x8f],
            Swap1 => vec![0x90],
            Swap2 => vec![0x91],
            Swap3 => vec![0x92],
            Swap4 => vec![0x93],
            Swap5 => vec![0x94],
            Swap6 => vec![0x95],
            Swap7 => vec![0x96],
            Swap8 => vec![0x97],
            Swap9 => vec![0x98],
            Swap10 => vec![0x99],
            Swap11 => vec![0x9a],
            Swap12 => vec![0x9b],
            Swap13 => vec![0x9c],
            Swap14 => vec![0x9d],
            Swap15 => vec![0x9e],
            Swap16 => vec![0x9f],

            // Log0 => vec![0xa0],
            Log2 => vec![0xa2],

            Return => vec![0xf3],

            Revert => vec![0xfd],
            Invalid => vec![0xfe],

            AugmentedPushJump(len, val) => Push(*len, *val).to_bytes().into_iter().chain(Jump.to_bytes().into_iter()).collect(),
            AugmentedPushJumpi(len, val) => Push(*len, *val).to_bytes().into_iter().chain(Jumpi.to_bytes().into_iter()).collect(),

            Unknown(opcode) => vec![*opcode],
        }
    }

    pub fn new_from_bytes(b: &[u8], mode: EvmOpParserMode) -> Result<(Self, usize), EvmOpError> {
        use EvmOp::*;

        if b.len() == 0 {
            return Err(EvmOpError::ParserErrorIncompleteInstruction);
        }

        let opcode = b[0];
        if 0x60u8 <= opcode && opcode <= 0x7Fu8 {
            // PUSH (read operand from code)
            let len = (opcode - 0x60 + 1) as usize;

            if 1 + len > b.len() {
                return Err(EvmOpError::ParserErrorIncompleteInstruction);
            } else {
                let val = U256::from_big_endian(&b[1 .. 1+len]);
                return Ok((Push(len, val), 1+len));
            }

        } else {
            // other opcodes
            match opcode {
                0x00 => Ok((Stop, 1)),
                0x01 => Ok((Add, 1)),
                0x02 => Ok((Mul, 1)),
                0x03 => Ok((Sub, 1)),
                0x04 => Ok((Div, 1)),
                0x05 => Ok((Sdiv, 1)),
                0x06 => Ok((Mod, 1)),
                0x0a => Ok((Exp, 1)),
                0x10 => Ok((Lt, 1)),
                0x11 => Ok((Gt, 1)),
                0x12 => Ok((Slt, 1)),
                0x13 => Ok((Sgt, 1)),
                0x14 => Ok((Eq, 1)),
                0x15 => Ok((Iszero, 1)),
                0x16 => Ok((And, 1)),
                0x17 => Ok((Or, 1)),
                0x19 => Ok((Not, 1)),
                0x1b => Ok((Shl, 1)),
                0x1c => Ok((Shr, 1)),
                0x20 => Ok((Sha3, 1)),

                0x32 => Ok((Origin, 1)),
                0x33 => Ok((Caller, 1)),
                0x34 => Ok((Callvalue, 1)),
                0x35 => Ok((Calldataload, 1)),
                0x36 => Ok((Calldatasize, 1)),

                0x50 => Ok((Pop, 1)),
                0x51 => Ok((Mload, 1)),
                0x52 => Ok((Mstore, 1)),
                0x53 => Ok((Mstore8, 1)),
                0x54 => Ok((Sload, 1)),
                0x55 => Ok((Sstore, 1)),
                0x56 => Ok((Jump, 1)),
                0x57 => Ok((Jumpi, 1)),
                0x5b => Ok((Jumpdest, 1)),
                
                0x80 => Ok((Dup1, 1)),
                0x81 => Ok((Dup2, 1)),
                0x82 => Ok((Dup3, 1)),
                0x83 => Ok((Dup4, 1)),
                0x84 => Ok((Dup5, 1)),
                0x85 => Ok((Dup6, 1)),
                0x86 => Ok((Dup7, 1)),
                0x87 => Ok((Dup8, 1)),
                0x88 => Ok((Dup9, 1)),
                0x89 => Ok((Dup10, 1)),
                0x8a => Ok((Dup11, 1)),
                0x8b => Ok((Dup12, 1)),
                0x8c => Ok((Dup13, 1)),
                0x8d => Ok((Dup14, 1)),
                0x8e => Ok((Dup15, 1)),
                0x8f => Ok((Dup16, 1)),

                0x90 => Ok((Swap1, 1)),
                0x91 => Ok((Swap2, 1)),
                0x92 => Ok((Swap3, 1)),
                0x93 => Ok((Swap4, 1)),
                0x94 => Ok((Swap5, 1)),
                0x95 => Ok((Swap6, 1)),
                0x96 => Ok((Swap7, 1)),
                0x97 => Ok((Swap8, 1)),
                0x98 => Ok((Swap9, 1)),
                0x99 => Ok((Swap10, 1)),
                0x9a => Ok((Swap11, 1)),
                0x9b => Ok((Swap12, 1)),
                0x9c => Ok((Swap13, 1)),
                0x9d => Ok((Swap14, 1)),
                0x9e => Ok((Swap15, 1)),
                0x9f => Ok((Swap16, 1)),

                0xa2 => Ok((Log2, 1)),
                0xf3 => Ok((Return, 1)),
                0xfd => Ok((Revert, 1)),
                0xfe => Ok((Invalid, 1)),

                _ => {
                    match mode {
                        EvmOpParserMode::Lax => Ok((Unknown(opcode), 1)),
                        EvmOpParserMode::Strict => {
                            return Err(EvmOpError::ParserErrorUnknownInstruction(opcode));
                        },
                    }
                },
            }
        }
    }
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvmCode {
    pub ops: Vec<EvmOp>,
}

#[derive(Error, Debug)]
pub enum EvmCodeError {
    #[error("parser error: incomplete instruction (PUSH) at offset {0}")]
    ParserErrorIncompleteInstruction(usize),
    #[error("parser error: unknown instruction at offset {0}: {1:#04x}")]
    ParserErrorUnknownInstruction(usize, u8),
}

impl EvmCode {
    pub fn new_from_bytes(b: &[u8], mode: EvmOpParserMode) -> Result<Self, EvmCodeError> {
        let mut idx = 0;
        let mut ops = Vec::new();

        while idx < b.len() {
            match EvmOp::new_from_bytes(&b[idx..], mode) {
                Ok((op, offset)) => {
                    ops.push(op);
                    idx += offset;
                },
                Err(EvmOpError::ParserErrorIncompleteInstruction) => {
                    return Err(EvmCodeError::ParserErrorIncompleteInstruction(idx));
                },
                Err(EvmOpError::ParserErrorUnknownInstruction(opcode)) => {
                    return Err(EvmCodeError::ParserErrorUnknownInstruction(idx, opcode));
                },
            }
        }

        Ok(Self { ops })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut ret = Vec::new();

        for op in &self.ops {
            ret.append(&mut op.to_bytes());
        }

        ret
    }

    pub fn augment(&self) -> Self {
        use EvmOp::*;

        let mut ops = Vec::new();
        let mut idx = 0;

        while idx < self.ops.len() {
            if idx < self.ops.len() - 1 {
                if let Push(len, val) = self.ops[idx] {
                    if self.ops[idx+1] == Jump {
                        ops.push(AugmentedPushJump(len, val));
                        idx += 2;
                        continue;
                    } else if self.ops[idx+1] == Jumpi {
                        ops.push(AugmentedPushJumpi(len, val));
                        idx += 2;
                        continue;
                    }
                }
            }
            
            ops.push(self.ops[idx].clone());
            idx += 1;
        }

        Self { ops }
    }

    pub fn index(&self) -> IndexedEvmCode {
        IndexedEvmCode::new_from_evmcode(self.clone())
    }
}


#[derive(Debug, Clone)]
pub struct IndexedEvmCode {
    pub code: EvmCode,
    pub opidx2target: HashMap<usize, U256>,
    pub target2opidx: HashMap<U256, usize>,
    pub jumpdests: HashSet<usize>,
}

impl IndexedEvmCode {
    pub fn new_from_evmcode(code: EvmCode) -> Self {
        let mut opidx2target = HashMap::new();
        let mut target2opidx = HashMap::new();
        let mut jumpdests = HashSet::new();

        let mut target = 0;
        for opidx in 0..code.ops.len() {
            opidx2target.insert(opidx, U256::zero() + target);
            target2opidx.insert(U256::zero() + target, opidx);
            target += code.ops[opidx].len();

            if code.ops[opidx] == EvmOp::Jumpdest {
                jumpdests.insert(opidx);
            }
        }

        Self { code, opidx2target, target2opidx, jumpdests }
    }
}
