# VM Performance Comparison

This repository exists as an accessible benchmark comparison between various strategies for implementing interpreters.

The benchmarks are not particularly scientific. Take them with a pinch of salt.

## Benchmarks

Benchmarks were performed on a 16 core AMD Ryzen 7 3700X.

```
test bytecode_closures_compile           ... bench:         440 ns/iter (+/- 5)
test bytecode_closures_execute           ... bench:     272,928 ns/iter (+/- 6,565)

test bytecode_compile                    ... bench:         170 ns/iter (+/- 4)
test bytecode_execute                    ... bench:     133,028 ns/iter (+/- 18,037)

test closure_continuations_compile       ... bench:         400 ns/iter (+/- 12)
test closure_continuations_execute       ... bench:      38,501 ns/iter (+/- 1,162)

test closure_stack_continuations_compile ... bench:         407 ns/iter (+/- 13)
test closure_stack_continuations_execute ... bench:      55,571 ns/iter (+/- 800)

test closures_compile                    ... bench:         348 ns/iter (+/- 41)
test closures_execute                    ... bench:      80,409 ns/iter (+/- 547)

test register_closures_compile           ... bench:         158 ns/iter (+/- 3)
test register_closures_execute           ... bench:      83,567 ns/iter (+/- 3,280)

test stack_closures_compile              ... bench:         501 ns/iter (+/- 14)
test stack_closures_execute              ... bench:     274,446 ns/iter (+/- 3,623)

test tape_closures_compile               ... bench:         146 ns/iter (+/- 8)
test tape_closures_execute               ... bench:     199,621 ns/iter (+/- 1,932)

test tape_continuations_compile          ... bench:         148 ns/iter (+/- 2)
test tape_continuations_execute          ... bench:      42,476 ns/iter (+/- 1,298)

test walker_compile                      ... bench:           0 ns/iter (+/- 0)
test walker_execute                      ... bench:     242,722 ns/iter (+/- 6,891)



test rust_execute                        ... bench:      17,104 ns/iter (+/- 4,848)
test rust_opt_execute                    ... bench:           1 ns/iter (+/- 0)
```

`rust_execute` and `rust_opt_execute` are 'standard candles', implemented in native Rust code. The former has very few
optimisations applied, whereas the latter is permitted to take advantage of the full optimising power of LLVM.

The fastest technique appears to be [`closure_continuations`](#closure_continuations). It manages to achieve very
respectable performance, coming within spitting difference of (deoptimised) native code.

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

### `closure_continuations`

Shares much of the simplicity of `closures`, but passes the next instruction to be performed - if any - as a continuation,
allowing for tail-call optimisation (TCO) to occur in a substantial number of cases.

### `closure_stack_continuations`

Just like `closure_continuations`, except it uses a stack to pass values around. This can improve the ability to perform
tail-call optimisations (TCO), at the cost of needing to touch memory when manipulating values. It's possible that some
combination of both approaches might hit an even nicer sweet spot.

### `bytecode_closures`

A mix between `bytecode` and `closures`. The AST is compiled down to a series of instruction-like closures, which are
then executed in a loop and indexed via an instruction pointer.

### `stack_closures`

Like `closures`, except intermediate values are maintained on a `Vec` stack rather than the hardware stack of the
closures.

### `tape_closures`

Like `closures`, except each closure is permitted no environment at compilation time, and instead fetches it from a tape
of static data at execution time.

### `register_closures`

Like `closures`, except the 2 highest most recently created locals are passed through the closures as arguments, rather
than being maintained on the locals stack.

### `tape_continuations`

Similar to `tape_closures`, except the next function to be executed is called from within the previous, allowing the
compiler to perform TCO (Tail Call Optimisation) on the function. This significantly reduces the stack-bashing that
needs to occur to set up each function, resulting in a very significant performance boost: at the cost of complexity.
