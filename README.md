# JIT-EVM Experiments

## Goal

Evaluate performance gains to expect when EVM were to compile hot contracts into machine code (rather than interpreting the EVM bytecode)


## Todos

* Use `memcpy`/`memset`/... for stack operations
* Inject execution context from "outside" into the JIT-compiled contract
* Callbacks into the host environment (memory/storage/calldata/returndata access, gas calculations, ...)
* Error handling
* Gas accounting


## Experiment

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


## References

* https://github.com/TheDan64/inkwell
* https://www.mattkeeter.com/projects/elfjit/
* https://github.com/mkeeter/advent-of-code/blob/master/2018/day21-jit/src/main.rs
* https://github.com/bluealloy/revm
* https://github.com/ethereum/evmone/pull/320
* https://github.com/axic/snailtracer
