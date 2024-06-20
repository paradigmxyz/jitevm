# JIT-EVM Experiments

_This repository has been superseded by https://github.com/paradigmxyz/revmc.
_
## Goal

Evaluate performance gains to expect when EVM were to compile hot contracts into machine code (rather than interpreting the EVM bytecode)


## Todos

* Support all instructions
* Performance evaluation
* Error handling
* Gas accounting


## Experiment (MacOS, see Ubuntu below)

Setup:
* LLVM 14 installed with `brew` in `/opt/homebrew/opt/llvm`

Run:
```
LLVM_SYS_140_PREFIX=/opt/homebrew/opt/llvm cargo run
```

What it does:
* Fibonacci sequence calculator hand-implemented in EVM bytecode
* Runs it in EVM interpreter
* Compiles it to machine code and executes it
* You compare that the outputs correspond to the 15th Fibonacci number (which is 377)

How to interpret the output:
* Modulo indexing convention, 15th Fibonacci number is 377 (says Google)
* Last line of demo output (this is from JIT VM): return value is 377
* Scrolling up past the LLVM IR: last state of interpreter has 377 in the 0th stack entry


## Tests (MacOS, see Ubuntu below)

```
RUST_BACKTRACE=1 LLVM_SYS_140_PREFIX=/opt/homebrew/opt/llvm RUST_MIN_STACK=8388608 cargo test -- --nocapture
```


## Ubuntu

Install:
```
sudo apt-get install llvm-14 llvm-14-dev libllvm14
```

Adjust paths above accordingly (leaving `LLVM_SYS_140_PREFIX=/opt/homebrew/opt/llvm` away entirely "works for me").


## References

* https://github.com/TheDan64/inkwell
* https://www.mattkeeter.com/projects/elfjit/
* https://github.com/mkeeter/advent-of-code/blob/master/2018/day21-jit/src/main.rs
* https://github.com/bluealloy/revm
* https://github.com/ethereum/evmone/pull/320
* https://github.com/axic/snailtracer
* https://doc.rust-lang.org/reference/type-layout.html
