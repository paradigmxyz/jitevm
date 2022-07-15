# JIT-EVM Experiments

## Goal

Evaluate performance gains to expect when EVM were to compile hot contracts into machine code (rather than interpreting the EVM bytecode)


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
