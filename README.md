# VM Performance Comparison

This repository exists as an accessible benchmark comparison between various strategies for implementing interpreters.

The benchmarks are not particularly scientific. Take them with a pinch of salt.

## Benchmarks

Benchmarks were performed on a 16 core AMD Ryzen 7 3700X.

```
test bytecode_closures_compile  ... bench:         449 ns/iter (+/- 15)
test bytecode_closures_execute  ... bench:     291,392 ns/iter (+/- 19,659)

test bytecode_compile           ... bench:         155 ns/iter (+/- 3)
test bytecode_execute           ... bench:     147,675 ns/iter (+/- 28,828)

test closures_compile           ... bench:         245 ns/iter (+/- 21)
test closures_execute           ... bench:     160,676 ns/iter (+/- 17,143)

test register_closures_compile  ... bench:         200 ns/iter (+/- 5)
test register_closures_execute  ... bench:     133,603 ns/iter (+/- 4,014)

test stack_closures_compile     ... bench:         498 ns/iter (+/- 8)
test stack_closures_execute     ... bench:     290,284 ns/iter (+/- 36,228)

test tape_closures_compile      ... bench:         133 ns/iter (+/- 11)
test tape_closures_execute      ... bench:     214,571 ns/iter (+/- 7,067)

test tape_continuations_compile ... bench:         169 ns/iter (+/- 1)
test tape_continuations_execute ... bench:      88,928 ns/iter (+/- 10,945)

test walker_compile             ... bench:           1 ns/iter (+/- 0)
test walker_execute             ... bench:     242,589 ns/iter (+/- 11,180)

test rust_execute               ... bench:      17,530 ns/iter (+/- 899)
test rust_opt_execute           ... bench:           1 ns/iter (+/- 0)
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

### `bytecode_closures`

A mix between `bytecode` and `closures`. The AST is compiled down to a series of instruction-like closures, which are
then executed in a loop and indexed via an instruction pointer.

### `tape_continuations`

Similar to `tape_closures`, except the next function to be executed is called from within the previous, allowing the
compiler to perform TCO (Tail Call Optimisation) on the function. This significantly reduces the stack-bashing that
needs to occur to set up each function, resulting in a very significant performance boost: at the cost of complexity.
