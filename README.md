# VM Performance Comparison

This repository exists as an accessible benchmark comparison between various strategies for implementing interpreters.

The benchmarks are not particularly scientific. Take them with a pinch of salt.

## Benchmarks

Benchmarks were performed on a 16 core AMD Ryzen 7 3700X.

```
test walker_compile            ... bench:           1 ns/iter (+/- 0)
test walker_execute            ... bench:     220,482 ns/iter (+/- 8,050)

test bytecode_compile          ... bench:         180 ns/iter (+/- 10)
test bytecode_execute          ... bench:     148,318 ns/iter (+/- 27,830)

test closures_compile          ... bench:         275 ns/iter (+/- 27)
test closures_execute          ... bench:     146,074 ns/iter (+/- 4,168)

test stack_closures_compile    ... bench:         417 ns/iter (+/- 27)
test stack_closures_execute    ... bench:     289,795 ns/iter (+/- 9,075)

test tape_closures_compile     ... bench:         170 ns/iter (+/- 7)
test tape_closures_execute     ... bench:     204,454 ns/iter (+/- 3,410)

test register_closures_compile ... bench:         196 ns/iter (+/- 11)
test register_closures_execute ... bench:     146,056 ns/iter (+/- 8,119)

test rust_execute              ... bench:      17,445 ns/iter (+/- 499)
test rust_opt_execute          ... bench:           1 ns/iter (+/- 0)
```

`rust_execute` and `rust_opt_execute` are 'standard candles', implemented in native Rust code. The former has very few
optimisations applied, whereas the latter is permitted to take advantage of the full optimising power of LLVM.

## Setup

Each technique has two stages:

- Compilation: The technique is given an expression AST and is permitted to generate whatever program it needs from it

- Execution: The technique is given the program and told to run the program to completion

For the sake of a fair comparison, I've tried to avoid any techniques taking advantage of the structure of the AST to
improve performance.

The AST provided to the techniques is conceptually simple. The only data types are integers, the only arithmetic
instruction is addition, and the only control flow is `while`. Locals exist and can be created and mutated. Programs
also get provided a series of arguments at execution time to parameterise their execution.

## Techniques

### `walker`

A simple AST walker. Compilation is an identity function. AST evaluation is done by recursively matching on AST nodes.

### `bytecode`

A naive stack 'bytecode' interpreter. Compilation takes the AST and translates it into a list of instructions. Execution
operates upon the stack, pushing and popping values.

### `closures`

Uses simple indirect threading, 'compiling' the entire program into a deeply nested closure. Execution simply evaluates
the closure.

### `stack_closures`

Like `closures`, except intermediate values are maintained on a `Vec` stack rather than the hardware stack of the
closures.

### `tape_closures`

Like `closures`, except each closure is permitted no environment at compilation time, and instead fetches it from a tape
of static data at execution time.

### `register_closures`

Like `closures`, except the 2 highest most recently created locals are passed through the closures as arguments, rather
than being maintained on the locals stack.
