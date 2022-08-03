use thiserror::Error;
use primitive_types::U256;
use std::collections::HashMap;
use crate::code::{EvmOp, IndexedEvmCode};
use crate::constants::{EVM_STACK_SIZE, EVM_STACK_ELEMENT_SIZE};
use crate::operations;


macro_rules! op1_u256_operation {
    ($self:ident, $fname:expr) => {{
        let a = $self.inner.pop()?;
        $self.inner.push($fname(a))?;
    }};
}

macro_rules! op2_u256_operation {
    ($self:ident, $fname:expr) => {{
        let a = $self.inner.pop()?;
        let b = $self.inner.pop()?;
        $self.inner.push($fname(a, b))?;
    }};
}

// macro_rules! op3_u256_operation {
//     ($self:ident, $fname:expr) => {{
//         let a = $self.inner.pop()?;
//         let b = $self.inner.pop()?;
//         let c = $self.inner.pop()?;
//         $self.inner.push($fname(a, b, c))?;
//     }};
// }


#[derive(Error, Debug)]
pub enum EvmInterpreterError {
    #[error("interpreter error: stack full")]
    StackFull,
    #[error("interpreter error: stack empty")]
    StackEmpty,
    #[error("interpreter error: stack too small")]
    StackTooSmall,
    #[error("interpreter error: Sload key not found")]
    SloadKeyNotFound,
    #[error("interpreter error: Jump destination invalid")]
    JumpDestinationInvalid,
    #[error("interpreter error: Jump destination not Jumpdest")]
    JumpDestinationNotJumpdest,
    #[error("unknown/unimplemented instruction: {0:?}")]
    UnknownInstruction(EvmOp),
}


#[derive(Debug, Clone)]
pub struct EvmOuterContext {
    pub calldata: Vec<u8>,
    // pub returndata: Vec<u8>,
    pub storage: HashMap<U256, U256>,
    pub callvalue: U256,
}


#[derive(Debug, Clone)]
pub struct EvmInnerContext<'a> {
    pub code: &'a IndexedEvmCode,
    pub stack: [U256; EVM_STACK_SIZE],
    pub pc: usize,
    pub sp: usize,
    pub memory: Vec<u8>,
    // pub gas: usize,
}

impl EvmInnerContext<'_> {
    #[inline(always)]
    pub fn push(&mut self, val: U256) -> Result<(), EvmInterpreterError> {
        if self.sp == EVM_STACK_SIZE {
            Err(EvmInterpreterError::StackFull)
        } else {
            self.stack[self.sp] = val;
            self.sp += 1;
            Ok(())
        }
    }

    #[inline(always)]
    pub fn pop(&mut self) -> Result<U256, EvmInterpreterError> {
        if self.sp == 0 {
            Err(EvmInterpreterError::StackEmpty)
        } else {
            self.sp -= 1;
            Ok(self.stack[self.sp])
        }
    }
}


#[derive(Debug, Clone)]
pub struct EvmContext<'a> {
    pub inner: EvmInnerContext<'a>,
    pub outer: EvmOuterContext,
}

impl EvmContext<'_> {
    pub fn _do_swap(&mut self, idx: usize) -> Result<(), EvmInterpreterError> {
        if self.inner.sp <= idx {
            return Err(EvmInterpreterError::StackTooSmall);
        }
        let a = self.inner.stack[self.inner.sp - 1];
        let b = self.inner.stack[self.inner.sp - (idx+1)];
        self.inner.stack[self.inner.sp - 1] = b;
        self.inner.stack[self.inner.sp - (idx+1)] = a;
        Ok(())
    }

    pub fn _do_dup(&mut self, idx: usize) -> Result<(), EvmInterpreterError> {
        if self.inner.sp < idx {
            return Err(EvmInterpreterError::StackTooSmall);
        }
        self.inner.push(self.inner.stack[self.inner.sp - idx])?;
        Ok(())
    }
    
    pub fn tick(&mut self) -> Result<bool, EvmInterpreterError> {
        // use EvmOp::*;

        if self.inner.pc == self.inner.code.code.ops.len() {
            return Ok(false)
        }

        let op = &self.inner.code.code.ops[self.inner.pc];
        self.inner.pc += 1;

        // println!("Op: {:?}", op);
        self.tick_inner(op)
    }

    pub fn tick_inner(&mut self, op: &EvmOp) -> Result<bool, EvmInterpreterError> {
        use EvmOp::*;
        
        match op {
            Stop => {
                return Ok(false);
            },
            Push(_, val) => {
                self.inner.push(*val)?;
            },
            Pop => {
                self.inner.pop()?;
            },
            Jumpdest => {},
            Mload => {
                let offset = self.inner.pop()?;
                let offset = offset.as_u64();
                let min_len = (offset + EVM_STACK_ELEMENT_SIZE) as usize;

                if self.inner.memory.len() < min_len {
                    self.inner.memory.resize(min_len, 0u8);
                }

                self.inner.push(U256::from_big_endian(&self.inner.memory[offset as usize..32+offset as usize]))?;
            },
            Mstore => {
                let offset = self.inner.pop()?;
                let value = self.inner.pop()?;

                let offset = offset.as_u64();
                let min_len = (offset + EVM_STACK_ELEMENT_SIZE) as usize;

                if self.inner.memory.len() < min_len {
                    self.inner.memory.resize(min_len, 0u8);
                }

                value.to_big_endian(&mut self.inner.memory[offset as usize..32+offset as usize]);
            },
            // Mstore8 => {

            // },
            Sload => {
                let key = self.inner.pop()?;
                let val = self.outer.storage.get(&key);//.ok_or(EvmInterpreterError::SloadKeyNotFound)?;
                let val = match val {
                    None => U256::zero(),
                    Some(v) => *v,
                };
                self.inner.push(val)?;
            },
            Sstore => {
                let key = self.inner.pop()?;
                let val = self.inner.pop()?;
                self.outer.storage.insert(key, val);
            },
            Jump => {
                let target = self.inner.pop()?;
                let opidx = self.inner.code.target2opidx.get(&target).ok_or(EvmInterpreterError::JumpDestinationInvalid)?;
                if !self.inner.code.jumpdests.contains(opidx) {
                    return Err(EvmInterpreterError::JumpDestinationNotJumpdest);
                }
                self.inner.pc = *opidx;
            },
            Jumpi => {
                let target = self.inner.pop()?;
                let cond = self.inner.pop()?;
                if cond != U256::zero() {
                    let opidx = self.inner.code.target2opidx.get(&target).ok_or(EvmInterpreterError::JumpDestinationInvalid)?;
                    if !self.inner.code.jumpdests.contains(opidx) {
                        return Err(EvmInterpreterError::JumpDestinationNotJumpdest);
                    }
                    self.inner.pc = *opidx;
                }
            },
            Swap1 => { self._do_swap(1)? },
            Swap2 => { self._do_swap(2)? },
            Swap3 => { self._do_swap(3)? },
            Swap4 => { self._do_swap(4)? },
            Swap5 => { self._do_swap(5)? },
            Swap6 => { self._do_swap(6)? },
            Swap7 => { self._do_swap(7)? },
            Swap8 => { self._do_swap(8)? },
            Swap9 => { self._do_swap(9)? },
            Swap10 => { self._do_swap(10)? },
            Swap11 => { self._do_swap(11)? },
            Swap12 => { self._do_swap(12)? },
            Swap13 => { self._do_swap(13)? },
            Swap14 => { self._do_swap(14)? },
            Swap15 => { self._do_swap(15)? },
            Swap16 => { self._do_swap(16)? },
            Dup1 => { self._do_dup(1)? },
            Dup2 => { self._do_dup(2)? },
            Dup3 => { self._do_dup(3)? },
            Dup4 => { self._do_dup(4)? },
            Dup5 => { self._do_dup(5)? },
            Dup6 => { self._do_dup(6)? },
            Dup7 => { self._do_dup(7)? },
            Dup8 => { self._do_dup(8)? },
            Dup9 => { self._do_dup(9)? },
            Dup10 => { self._do_dup(10)? },
            Dup11 => { self._do_dup(11)? },
            Dup12 => { self._do_dup(12)? },
            Dup13 => { self._do_dup(13)? },
            Dup14 => { self._do_dup(14)? },
            Dup15 => { self._do_dup(15)? },
            Dup16 => { self._do_dup(16)? },
            Add => op2_u256_operation!(self, operations::Add),
            Mul => op2_u256_operation!(self, operations::Mul),
            Sub => op2_u256_operation!(self, operations::Sub),
            Exp => op2_u256_operation!(self, operations::Exp),
            Div => op2_u256_operation!(self, operations::Div),
            Sdiv => op2_u256_operation!(self, operations::Sdiv),
            Mod => op2_u256_operation!(self, operations::Mod),
            // Smod => op2_u256_operation!(self, operations::Smod),
            // Addmod => op3_u256_operation!(self, operations::Addmod),
            // Mulmod => op3_u256_operation!(self, operations::Mulmod),
            Slt => op2_u256_operation!(self, operations::Slt),
            Sgt => op2_u256_operation!(self, operations::Sgt),
            Iszero => op1_u256_operation!(self, operations::Iszero),
            Not => op1_u256_operation!(self, operations::Not),
            // Byte => op2_u256_operation!(self, operations::Byte),
            Shl => op2_u256_operation!(self, operations::Shl),
            Shr => op2_u256_operation!(self, operations::Shr),
            // Sar => op2_u256_operation!(self, operations::Sar),
            And => op2_u256_operation!(self, operations::And),
            Or => op2_u256_operation!(self, operations::Or),
            // Xor => op2_u256_operation!(self, operations::Xor),
            // Signextend => op2_u256_operation!(self, operations::Signextend),
            Lt => op2_u256_operation!(self, operations::Lt),
            Gt => op2_u256_operation!(self, operations::Gt),
            Eq => op2_u256_operation!(self, operations::Eq),
            Callvalue => {
                self.inner.push(self.outer.callvalue)?;
            },
            Calldatasize => {
                self.inner.push(U256::zero() + self.outer.calldata.len())?;
            },
            Calldataload => {
                let offset = self.inner.pop()?.as_usize();
                if offset >= self.outer.calldata.len() {
                    self.inner.push(U256::zero())?;
                } else {
                    let mut read_from = self.outer.calldata.clone();
                    read_from.extend_from_slice(&[0u8; 32]);
                    self.inner.push(U256::from_big_endian(&read_from[offset..offset+(EVM_STACK_ELEMENT_SIZE as usize)]))?;
                }
            },
            _ => {
                return Err(EvmInterpreterError::UnknownInstruction(op.clone()));
            },
            // 0x52 => {
            //     // MSTORE
            //     let offset = self.inner.pop().as_usize();
            //     let value = self.inner.pop();

            //     if self.inner.memory.len() < offset + 32 {
            //         self.inner.memory.resize(offset + 32, 0);
            //     }
            //     value.to_big_endian(&mut self.inner.memory[offset .. offset+32]);

            //     // account for gas:
            //     // ...
            // },
            // 0x60 => {
            //     // PUSH1
            //     let val = U256::from_big_endian(&self.inner.code[self.inner.pc .. self.inner.pc+1]);
            //     self.inner.pc += 1;
                
            //     self.inner.push(val);

            //     // account for gas:
            //     // ...
            // },
            // _ => {
            //     panic!("Unsupported opcode: {:#04x}", opcode);
            // }
        }

        Ok(true)
    }

    #[inline(always)]
    pub fn tick_inner_simplified(&mut self, op: EvmOp) -> Result<bool, EvmInterpreterError> {
        use EvmOp::*;
        
        match op {
            Stop => {
                return Ok(false);
            },
            Push(_, val) => {
                self.inner.push(val)?;
            },
            Pop => {
                self.inner.pop()?;
            },
            Jumpdest => {},
            Jump => {
                let target = self.inner.pop()?;
                self.inner.pc = target.as_usize();
            },
            Jumpi => {
                let target = self.inner.pop()?;
                let cond = self.inner.pop()?;
                if cond != U256::zero() {
                    self.inner.pc = target.as_usize();
                }
            },
            Swap1 => { self._do_swap(1)? },
            Swap2 => { self._do_swap(2)? },
            Swap3 => { self._do_swap(3)? },
            Swap4 => { self._do_swap(4)? },
            Swap5 => { self._do_swap(5)? },
            Swap6 => { self._do_swap(6)? },
            Swap7 => { self._do_swap(7)? },
            Swap8 => { self._do_swap(8)? },
            Swap9 => { self._do_swap(9)? },
            Swap10 => { self._do_swap(10)? },
            Swap11 => { self._do_swap(11)? },
            Swap12 => { self._do_swap(12)? },
            Swap13 => { self._do_swap(13)? },
            Swap14 => { self._do_swap(14)? },
            Swap15 => { self._do_swap(15)? },
            Swap16 => { self._do_swap(16)? },
            Dup1 => { self._do_dup(1)? },
            Dup2 => { self._do_dup(2)? },
            Dup3 => { self._do_dup(3)? },
            Dup4 => { self._do_dup(4)? },
            Dup5 => { self._do_dup(5)? },
            Dup6 => { self._do_dup(6)? },
            Dup7 => { self._do_dup(7)? },
            Dup8 => { self._do_dup(8)? },
            Dup9 => { self._do_dup(9)? },
            Dup10 => { self._do_dup(10)? },
            Dup11 => { self._do_dup(11)? },
            Dup12 => { self._do_dup(12)? },
            Dup13 => { self._do_dup(13)? },
            Dup14 => { self._do_dup(14)? },
            Dup15 => { self._do_dup(15)? },
            Dup16 => { self._do_dup(16)? },
            Add => op2_u256_operation!(self, operations::Add),
            Mul => op2_u256_operation!(self, operations::Mul),
            Sub => op2_u256_operation!(self, operations::Sub),
            Exp => op2_u256_operation!(self, operations::Exp),
            Div => op2_u256_operation!(self, operations::Div),
            Sdiv => op2_u256_operation!(self, operations::Sdiv),
            Mod => op2_u256_operation!(self, operations::Mod),
            // Smod => op2_u256_operation!(self, operations::Smod),
            // Addmod => op3_u256_operation!(self, operations::Addmod),
            // Mulmod => op3_u256_operation!(self, operations::Mulmod),
            Slt => op2_u256_operation!(self, operations::Slt),
            Sgt => op2_u256_operation!(self, operations::Sgt),
            Iszero => op1_u256_operation!(self, operations::Iszero),
            Not => op1_u256_operation!(self, operations::Not),
            // Byte => op2_u256_operation!(self, operations::Byte),
            Shl => op2_u256_operation!(self, operations::Shl),
            Shr => op2_u256_operation!(self, operations::Shr),
            // Sar => op2_u256_operation!(self, operations::Sar),
            And => op2_u256_operation!(self, operations::And),
            Or => op2_u256_operation!(self, operations::Or),
            // Xor => op2_u256_operation!(self, operations::Xor),
            // Signextend => op2_u256_operation!(self, operations::Signextend),
            Lt => op2_u256_operation!(self, operations::Lt),
            Gt => op2_u256_operation!(self, operations::Gt),
            Eq => op2_u256_operation!(self, operations::Eq),
            _ => {
                return Err(EvmInterpreterError::UnknownInstruction(op.clone()));
            },
        }

        Ok(true)
    }
}


