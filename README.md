# VM Performance Comparison

This repository exists as an accessible benchmark comparison between various strategies for implementing interpreters.

The benchmarks are not particularly scientific. Take them with a pinch of salt.

## Benchmarks

Benchmarks were performed on a 16 core AMD Ryzen 7 3700X.

```
test bytecode_closures_compile           ... bench:         442 ns/iter (+/- 9)
test bytecode_closures_execute           ... bench:     278,186 ns/iter (+/- 3,722)

test bytecode_compile                    ... bench:         170 ns/iter (+/- 4)
test bytecode_execute                    ... bench:     135,355 ns/iter (+/- 17,387)

test closure_continuations_compile       ... bench:         397 ns/iter (+/- 12)
test closure_continuations_execute       ... bench:      38,407 ns/iter (+/- 420)

test closure_stack_continuations_compile ... bench:         408 ns/iter (+/- 11)
test closure_stack_continuations_execute ... bench:      55,867 ns/iter (+/- 974)

test closures_compile                    ... bench:         230 ns/iter (+/- 1)
test closures_execute                    ... bench:     137,376 ns/iter (+/- 1,448)

test register_closures_compile           ... bench:         154 ns/iter (+/- 4)
test register_closures_execute           ... bench:      79,356 ns/iter (+/- 2,885)

test stack_closures_compile              ... bench:         502 ns/iter (+/- 9)
test stack_closures_execute              ... bench:     269,112 ns/iter (+/- 4,937)

test tape_closures_compile               ... bench:         192 ns/iter (+/- 5)
test tape_closures_execute               ... bench:     206,273 ns/iter (+/- 10,550)

test tape_continuations_compile          ... bench:         198 ns/iter (+/- 2)
test tape_continuations_execute          ... bench:      42,758 ns/iter (+/- 899)

test walker_compile                      ... bench:           0 ns/iter (+/- 0)
test walker_execute                      ... bench:     219,535 ns/iter (+/- 9,767)

test rust_execute                        ... bench:      16,755 ns/iter (+/- 222)
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
