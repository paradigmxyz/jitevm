use thiserror::Error;
use primitive_types::U256;
use std::collections::HashMap;


#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EvmOp {
    Stop,
    Push(usize, U256),
    Pop,
    Jumpdest,
    Jump,
    Jumpi,
    Swap1,
    Swap2,
    Dup2,
    Dup3,
    Dup4,
    Iszero,
    Add,
    Sub,

    AugmentedPushJump(usize, U256),
    AugmentedPushJumpi(usize, U256),
}

#[derive(Error, Debug)]
pub enum EvmOpError {
    #[error("parser error: incomplete instruction")]
    ParserErrorIncompleteInstruction,
    #[error("parser error: unknown instruction")]
    ParserErrorUnknownInstruction,
}

impl EvmOp {
    pub fn len(&self) -> usize {
        use EvmOp::*;

        match self {
            Stop => 1,
            Push(len, _) => 1 + len,
            Pop => 1,
            Jumpdest => 1,
            Jump => 1,
            Jumpi => 1,
            Swap1 => 1,
            Swap2 => 1,
            Dup2 => 1,
            Dup3 => 1,
            Dup4 => 1,
            Iszero => 1,
            Add => 1,
            Sub => 1,

            AugmentedPushJump(len, _) => 1 + len + 1,
            AugmentedPushJumpi(len, _) => 1 + len + 1,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        use EvmOp::*;

        match self {
            Stop => vec![0x00],
            Push(len, val) => {
                assert!(*len >= 1);
                assert!(*len <= 32);

                let mut v = [0u8; 32];
                val.to_big_endian(&mut v);
                let mut w = vec![0x60];
                w.append(&mut v[32-len..32].to_vec());
                w
            },
            Pop => vec![0x50],
            Jumpdest => vec![0x5B],
            Jump => vec![0x56],
            Jumpi => vec![0x57],
            Swap1 => vec![0x90],
            Swap2 => vec![0x91],
            Dup2 => vec![0x81],
            Dup3 => vec![0x82],
            Dup4 => vec![0x83],
            Iszero => vec![0x15],
            Add => vec![0x01],
            Sub => vec![0x03],

            AugmentedPushJump(len, val) => Push(*len, *val).to_bytes().into_iter().chain(Jump.to_bytes().into_iter()).collect(),
            AugmentedPushJumpi(len, val) => Push(*len, *val).to_bytes().into_iter().chain(Jumpi.to_bytes().into_iter()).collect(),
        }
    }

    pub fn new_from_bytes(b: &[u8]) -> Result<(Self, usize), EvmOpError> {
        use EvmOp::*;

        if b.len() == 0 {
            return Err(EvmOpError::ParserErrorIncompleteInstruction);
        }

        let opcode = b[0] as usize;
        if 0x60 <= opcode && opcode <= 0x7F {
            // PUSH (read operand from code)
            let len = opcode - 0x60 + 1;

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
                0x50 => Ok((Pop, 1)),
                0x5B => Ok((Jumpdest, 1)),
                0x56 => Ok((Jump, 1)),
                0x57 => Ok((Jumpi, 1)),
                0x90 => Ok((Swap1, 1)),
                0x91 => Ok((Swap2, 1)),
                0x81 => Ok((Dup2, 1)),
                0x82 => Ok((Dup3, 1)),
                0x83 => Ok((Dup4, 1)),
                0x15 => Ok((Iszero, 1)),
                0x01 => Ok((Add, 1)),
                0x03 => Ok((Sub, 1)),
                _ => {
                    return Err(EvmOpError::ParserErrorUnknownInstruction);
                },
            }
        }
    }
}


#[derive(Debug)]
pub struct EvmCode {
    pub ops: Vec<EvmOp>,
}

#[derive(Error, Debug)]
pub enum EvmCodeError {
    #[error("parser error: incomplete instruction at offset {0}")]
    ParserErrorIncompleteInstruction(usize),
    #[error("parser error: unknown instruction at offset {0}")]
    ParserErrorUnknownInstruction(usize),
}

impl EvmCode {
    pub fn new_from_bytes(b: &[u8]) -> Result<Self, EvmCodeError> {
        let mut idx = 0;
        let mut ops = Vec::new();

        while idx < b.len() {
            match EvmOp::new_from_bytes(&b[idx..]) {
                Ok((op, offset)) => {
                    ops.push(op);
                    idx += offset;
                },
                Err(EvmOpError::ParserErrorIncompleteInstruction) => {
                    return Err(EvmCodeError::ParserErrorIncompleteInstruction(idx));
                },
                Err(EvmOpError::ParserErrorUnknownInstruction) => {
                    return Err(EvmCodeError::ParserErrorUnknownInstruction(idx));
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
}


#[derive(Debug)]
pub struct IndexedEvmCode {
    pub code: EvmCode,
    pub opidx2target: HashMap<usize, U256>,
    pub target2opidx: HashMap<U256, usize>,
    pub jumpdests: Vec<usize>,
}

impl IndexedEvmCode {
    pub fn new_from_evmcode(code: EvmCode) -> Self {
        let mut opidx2target = HashMap::new();
        let mut target2opidx = HashMap::new();
        let mut jumpdests = Vec::new();

        let mut target = 0;
        for opidx in 0..code.ops.len() {
            opidx2target.insert(opidx, U256::zero() + target);
            target2opidx.insert(U256::zero() + target, opidx);
            target += code.ops[opidx].len();

            if code.ops[opidx] == EvmOp::Jumpdest {
                jumpdests.push(opidx);
            }
        }

        Self { code, opidx2target, target2opidx, jumpdests }
    }
}
