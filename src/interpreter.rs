#[derive(Debug)]
struct EvmOuterContext {
    memory: Vec<u8>,
    calldata: Vec<u8>,
    returndata: Vec<u8>,
}


#[derive(Debug)]
struct EvmInnerContext<'a> {
    code: &'a EvmCode,
    stack: [U256; 10],//1024],
    pc: usize,
    sp: usize,
    gas: usize,
}

impl EvmInnerContext<'_> {
    // TODO: make sure this can fail!
    fn push(&mut self, val: U256) {
        self.stack[self.sp] = val;
        self.sp += 1;
    }

    // TODO: handle empty stack
    fn pop(&mut self) -> U256 {
        let val = if self.sp == 0 {
            // error!
            0.into()
        } else {
            self.stack[self.sp-1]
        };
        self.sp -= 1;
        val
    }
}


#[derive(Debug)]
struct EvmContext<'a> {
    inner: EvmInnerContext<'a>,
    outer: EvmOuterContext,
}

impl EvmContext<'_> {
    fn tick(&mut self) -> bool {
        use EvmOp::*;

        let op = &self.inner.code.ops[self.inner.pc];
        self.inner.pc += 1;

        // println!("Op: {:?}", op);

        match op {
            Stop => {
                return false;
            },
            Push(_, val) => {
                self.inner.push(*val);
            },
            Pop => {
                self.inner.pop();
            },
            Jumpdest => {},
            Jump => {
                let target = self.inner.pop();
                let opidx = self.inner.code.opidx_for_target(target);
                if self.inner.code.ops[opidx] != Jumpdest {
                    panic!("jump-ing to not jumpdest, aaaah!");
                }
                self.inner.pc = opidx;
            },
            Jumpi => {
                let target = self.inner.pop();
                let cond = self.inner.pop();
                if cond != U256::zero() {
                    let opidx = self.inner.code.opidx_for_target(target);
                    if self.inner.code.ops[opidx] != Jumpdest {
                        panic!("jumpi-ing to not jumpdest, aaaah!");
                    }
                    self.inner.pc = opidx;
                }
            },
            Swap1 => {
                let a = self.inner.stack[self.inner.sp - 1];
                let b = self.inner.stack[self.inner.sp - 2];
                self.inner.stack[self.inner.sp - 1] = b;
                self.inner.stack[self.inner.sp - 2] = a;
            },
            Swap2 => {
                let a = self.inner.stack[self.inner.sp - 1];
                let b = self.inner.stack[self.inner.sp - 3];
                self.inner.stack[self.inner.sp - 1] = b;
                self.inner.stack[self.inner.sp - 3] = a;
            },
            Dup2 => {
                self.inner.push(self.inner.stack[self.inner.sp - 2]);
            },
            Dup3 => {
                self.inner.push(self.inner.stack[self.inner.sp - 3]);
            },
            Dup4 => {
                self.inner.push(self.inner.stack[self.inner.sp - 4]);
            },
            Iszero => {
                let val = self.inner.pop();
                if val == U256::zero() {
                    self.inner.push(U256::one());
                } else {
                    self.inner.push(U256::zero());
                }
            },
            Add => {
                let a = self.inner.pop();
                let b = self.inner.pop();
                self.inner.push(a + b);
            },
            Sub => {
                let a = self.inner.pop();
                let b = self.inner.pop();
                self.inner.push(a - b);
            },
            _ => {
                panic!("Op not implemented: {:?}", op);
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

        return true;
    }
}


