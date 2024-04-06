#![feature(test)]

extern crate test;
use test::{black_box, Bencher};
use vm_perf::{
    Bytecode, BytecodeClosures, ClosureContinuations, ClosureStackContinuations, Closures, Expr,
    RegisterClosures, StackClosures, TapeClosures, TapeContinuations, Vm, Walker,
};

fn create_expr() -> Expr {
    // let mut total = 0;
    // let mut count = args[0];
    // while count > 0 {
    //     total = total + args[1];
    //     count = count - 1;
    // }
    // total
    Expr::Let(
        Box::new(Expr::Litr(0)), // total
        Box::new(Expr::Then(
            Box::new(Expr::Let(
                Box::new(Expr::Arg(0)), // counter
                Box::new(Expr::While(
                    Box::new(Expr::Get(0)),
                    Box::new(Expr::Then(
                        Box::new(Expr::Set(
                            1,
                            Box::new(Expr::Add(Box::new(Expr::Get(1)), Box::new(Expr::Arg(1)))),
                        )),
                        Box::new(Expr::Set(
                            0,
                            Box::new(Expr::Add(Box::new(Expr::Get(0)), Box::new(Expr::Litr(-1)))),
                        )),
                    )),
                )),
            )),
            Box::new(Expr::Get(0)), // total
        )),
    )
}

#[inline(never)]
unsafe fn rust_impl(args: &[i64]) -> i64 {
    // Lots of silly stuff to force the compiler skip basically all attempts at optimisation
    let mut total = black_box(0);
    let mut count = black_box(*args.get_unchecked(0));
    while black_box(count) > 0 {
        total = black_box(total) + black_box(*args.get_unchecked(1));
        count = black_box(count) + black_box(-1);
    }
    black_box(total)
}

#[inline(never)]
unsafe fn rust_impl_opt(args: &[i64]) -> i64 {
    let mut total = 0;
    let mut count = *args.get_unchecked(0);
    while count > 0 {
        total = total + *args.get_unchecked(1);
        count = count + -1;
    }
    total
}

fn create_args() -> &'static [i64] {
    &[10000, 13]
}

fn answer() -> i64 {
    10000 * 13
}

fn bench_compile<V: Vm>(b: &mut Bencher) {
    let expr = black_box(create_expr());

    b.iter(move || {
        black_box(V::compile(&expr));
    });
}

fn bench_execute<V: Vm>(b: &mut Bencher) {
    let expr = create_expr();

    let program = black_box(V::compile(&expr));

    let args = black_box(create_args());

    b.iter(move || {
        let res = unsafe { black_box(V::execute(&program, args)) };
        assert_eq!(res, answer());
    });
}

// AST walker
#[bench]
fn walker_compile(b: &mut Bencher) {
    bench_compile::<Walker>(b)
}
#[bench]
fn walker_execute(b: &mut Bencher) {
    bench_execute::<Walker>(b)
}
// Bytecode
#[bench]
fn bytecode_compile(b: &mut Bencher) {
    bench_compile::<Bytecode>(b)
}
#[bench]
fn bytecode_execute(b: &mut Bencher) {
    bench_execute::<Bytecode>(b)
}
// Closures
#[bench]
fn closures_compile(b: &mut Bencher) {
    bench_compile::<Closures>(b)
}
#[bench]
fn closures_execute(b: &mut Bencher) {
    bench_execute::<Closures>(b)
}
// Stack closures
#[bench]
fn stack_closures_compile(b: &mut Bencher) {
    bench_compile::<StackClosures>(b)
}
#[bench]
fn stack_closures_execute(b: &mut Bencher) {
    bench_execute::<StackClosures>(b)
}
// Tape closures
#[bench]
fn tape_closures_compile(b: &mut Bencher) {
    bench_compile::<TapeClosures>(b)
}
#[bench]
fn tape_closures_execute(b: &mut Bencher) {
    bench_execute::<TapeClosures>(b)
}
// Register closures
#[bench]
fn register_closures_compile(b: &mut Bencher) {
    bench_compile::<RegisterClosures>(b)
}
#[bench]
fn register_closures_execute(b: &mut Bencher) {
    bench_execute::<RegisterClosures>(b)
}
// Bytecode closures
#[bench]
fn bytecode_closures_compile(b: &mut Bencher) {
    bench_compile::<BytecodeClosures>(b)
}
#[bench]
fn bytecode_closures_execute(b: &mut Bencher) {
    bench_execute::<BytecodeClosures>(b)
}
// Tape closures
#[bench]
fn tape_continuations_compile(b: &mut Bencher) {
    bench_compile::<TapeContinuations>(b)
}
#[bench]
fn tape_continuations_execute(b: &mut Bencher) {
    bench_execute::<TapeContinuations>(b)
}
// Closure continuations
#[bench]
fn closure_continuations_compile(b: &mut Bencher) {
    bench_compile::<ClosureContinuations>(b)
}
#[bench]
fn closure_continuations_execute(b: &mut Bencher) {
    bench_execute::<ClosureContinuations>(b)
}
// Closure stack continuations
#[bench]
fn closure_stack_continuations_compile(b: &mut Bencher) {
    bench_compile::<ClosureStackContinuations>(b)
}
#[bench]
fn closure_stack_continuations_execute(b: &mut Bencher) {
    bench_execute::<ClosureStackContinuations>(b)
}

// Pure Rust controls
#[bench]
fn rust_execute(b: &mut Bencher) {
    let args = black_box(create_args());
    b.iter(move || {
        let res = unsafe { rust_impl(args) };
        assert_eq!(res, answer());
    });
}
#[bench]
fn rust_opt_execute(b: &mut Bencher) {
    let args = black_box(create_args());
    b.iter(move || {
        let res = unsafe { rust_impl_opt(args) };
        assert_eq!(res, answer());
    });
}
