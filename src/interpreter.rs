use thiserror::Error;
use primitive_types::U256;
use crate::code::{EvmOp, IndexedEvmCode};
use crate::constants::{EVM_STACK_SIZE};


#[derive(Error, Debug)]
pub enum EvmInterpreterError {
    #[error("interpreter error: stack full")]
    StackFull,
    #[error("interpreter error: stack empty")]
    StackEmpty,
    #[error("interpreter error: stack too small")]
    StackTooSmall,
    #[error("interpreter error: Jump destination invalid")]
    JumpDestinationInvalid,
    #[error("interpreter error: Jump destination not Jumpdest")]
    JumpDestinationNotJumpdest,
    #[error("unknown/unimplemented instruction: {0:?}")]
    UnknownInstruction(EvmOp),
}


#[derive(Debug, Clone)]
pub struct EvmOuterContext {
    pub memory: Vec<u8>,
    pub calldata: Vec<u8>,
    pub returndata: Vec<u8>,
}


#[derive(Debug, Clone)]
pub struct EvmInnerContext<'a> {
    pub code: &'a IndexedEvmCode,
    pub stack: [U256; EVM_STACK_SIZE],
    pub pc: usize,
    pub sp: usize,
    pub gas: usize,
}

impl EvmInnerContext<'_> {
    pub fn push(&mut self, val: U256) -> Result<(), EvmInterpreterError> {
        if self.sp == EVM_STACK_SIZE {
            Err(EvmInterpreterError::StackFull)
        } else {
            self.stack[self.sp] = val;
            self.sp += 1;
            Ok(())
        }
    }

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
    pub fn tick(&mut self) -> Result<bool, EvmInterpreterError> {
        use EvmOp::*;

        let op = &self.inner.code.code.ops[self.inner.pc];
        self.inner.pc += 1;

        // println!("Op: {:?}", op);

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
            Swap1 => {
                if self.inner.sp <= 1 {
                    return Err(EvmInterpreterError::StackTooSmall);
                }
                let a = self.inner.stack[self.inner.sp - 1];
                let b = self.inner.stack[self.inner.sp - 2];
                self.inner.stack[self.inner.sp - 1] = b;
                self.inner.stack[self.inner.sp - 2] = a;
            },
            Swap2 => {
                if self.inner.sp <= 2 {
                    return Err(EvmInterpreterError::StackTooSmall);
                }
                let a = self.inner.stack[self.inner.sp - 1];
                let b = self.inner.stack[self.inner.sp - 3];
                self.inner.stack[self.inner.sp - 1] = b;
                self.inner.stack[self.inner.sp - 3] = a;
            },
            Dup2 => {
                if self.inner.sp < 2 {
                    return Err(EvmInterpreterError::StackTooSmall);
                }
                self.inner.push(self.inner.stack[self.inner.sp - 2])?;
            },
            Dup3 => {
                if self.inner.sp < 3 {
                    return Err(EvmInterpreterError::StackTooSmall);
                }
                self.inner.push(self.inner.stack[self.inner.sp - 3])?;
            },
            Dup4 => {
                if self.inner.sp < 4 {
                    return Err(EvmInterpreterError::StackTooSmall);
                }
                self.inner.push(self.inner.stack[self.inner.sp - 4])?;
            },
            Iszero => {
                let val = self.inner.pop()?;
                if val == U256::zero() {
                    self.inner.push(U256::one())?;
                } else {
                    self.inner.push(U256::zero())?;
                }
            },
            Add => {
                let a = self.inner.pop()?;
                let b = self.inner.pop()?;
                self.inner.push(a + b)?;
            },
            Sub => {
                let a = self.inner.pop()?;
                let b = self.inner.pop()?;
                self.inner.push(a - b)?;
            },
            _ => {
                return Err(EvmInterpreterError::UnknownInstruction(op.clone()));
            },
            // 0x52 => {
            //     // MSTORE
            //     let offset = self.inner.pop().as_usize();
            //     let value = self.inner.pop();

            //     if self.outer.memory.len() < offset + 32 {
            //         self.outer.memory.resize(offset + 32, 0);
            //     }
            //     value.to_big_endian(&mut self.outer.memory[offset .. offset+32]);

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
}


