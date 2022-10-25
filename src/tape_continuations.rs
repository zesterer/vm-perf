use super::*;
use std::marker::PhantomData;

pub struct TapeContinuations;

#[derive(Default)]
struct Reg {
    //r0: i64,
    //r1: i64,
}

type OpFn = unsafe fn(reg: Reg, &[i64], &mut Tape, Stack, Stack);

pub struct Stack(*mut i64);

impl Stack {
    #[inline(always)]
    unsafe fn push(&mut self, x: i64) {
        self.0.write(x);
        self.0 = self.0.add(1);
    }

    #[inline(always)]
    unsafe fn pop(&mut self) -> i64 {
        self.0 = self.0.sub(1);
        self.0.read()
    }

    unsafe fn set_offset(&mut self, offset: usize, x: i64) {
        self.0.sub(offset + 1).write(x);
    }

    unsafe fn get_offset(&self, offset: usize) -> i64 {
        self.0.sub(offset + 1).read()
    }
}

#[derive(Copy, Clone)]
struct Tape<'a>(*const usize, PhantomData<&'a ()>);

impl<'a> Tape<'a> {
    //unsafe fn next_fn(&mut self) -> OpFn { let res = std::mem::transmute(self.0.read()); self.0 = self.0.add(1); res }
    #[inline(always)]
    unsafe fn next_eval(&mut self, reg: Reg, args: &[i64], mut stack: Stack, locals: Stack) {
        let f = std::mem::transmute::<_, OpFn>(self.0.read());
        self.0 = self.0.add(1);
        f(reg, args, self, stack, locals)
    }
    #[inline(always)]
    unsafe fn next_int(&mut self) -> i64 { let res = std::mem::transmute(self.0.read()); self.0 = self.0.add(1); res }
    unsafe fn next_usize(&mut self) -> usize { let res = self.0.read(); self.0 = self.0.add(1); res }
    unsafe fn skip(&mut self, n: usize) { self.0 = self.0.add(n); }
    unsafe fn unskip(&mut self, n: usize) { self.0 = self.0.sub(n); }
}

impl Vm for TapeContinuations {
    type Program<'a> = Vec<usize>;

    fn compile(expr: &Expr) -> Self::Program<'_> {
        fn compile_inner(ops: &mut Vec<usize>, expr: &Expr) {
            fn returns(expr: &Expr) -> bool {
                match expr {
                    Expr::Litr(_)
                    | Expr::Arg(_)
                    | Expr::Get(_)
                    | Expr::Add(_, _) => true,
                    Expr::Let(_, expr) => returns(expr),
                    Expr::Set(_, _)
                    | Expr::While(_, _) => false,
                    Expr::Then(_, b) => returns(b),
                }
            }

            match expr {
                Expr::Litr(x) => {
                    unsafe fn litr(reg: Reg, args: &[i64], tape: &mut Tape, mut stack: Stack, locals: Stack) {
                        let x = tape.next_int();
                        stack.push(x);
                        tape.next_eval(reg, args, stack, locals);
                    }
                    ops.push(unsafe { std::mem::transmute(litr as OpFn) });
                    ops.push(*x as usize);
                },
                Expr::Arg(idx) => {
                    unsafe fn arg(reg: Reg, args: &[i64], tape: &mut Tape, mut stack: Stack, locals: Stack) {
                        let idx = tape.next_usize();
                        stack.push(*args.get_unchecked(idx));
                        tape.next_eval(reg, args, stack, locals)
                    }
                    ops.push(unsafe { std::mem::transmute(arg as OpFn) });
                    ops.push(*idx);
                },
                Expr::Get(local) => {
                    unsafe fn get(reg: Reg, args: &[i64], tape: &mut Tape, mut stack: Stack, locals: Stack) {
                        let local = tape.next_usize();
                        stack.push(locals.get_offset(local));
                        tape.next_eval(reg, args, stack, locals)
                    }
                    ops.push(unsafe { std::mem::transmute(get as OpFn) });
                    ops.push(*local);
                },
                Expr::Add(x, y) => {
                    unsafe fn add(reg: Reg, args: &[i64], tape: &mut Tape, mut stack: Stack, locals: Stack) {
                        let x = stack.pop();
                        let y = stack.pop();
                        stack.push(x + y);
                        tape.next_eval(reg, args, stack, locals)
                    }
                    compile_inner(ops, x);
                    compile_inner(ops, y);
                    ops.push(unsafe { std::mem::transmute(add as OpFn) });
                },
                Expr::Let(rhs, then) => {
                    unsafe fn let_push(reg: Reg, args: &[i64], tape: &mut Tape, mut stack: Stack, mut locals: Stack) {
                        locals.push(stack.pop());
                        tape.next_eval(reg, args, stack, locals)
                    }
                    compile_inner(ops, rhs);
                    ops.push(unsafe { std::mem::transmute(let_push as OpFn) });
                    compile_inner(ops, then);
                    unsafe fn let_pop(reg: Reg, args: &[i64], tape: &mut Tape, mut stack: Stack, mut locals: Stack) {
                        locals.pop();
                        tape.next_eval(reg, args, stack, locals)
                    }
                    ops.push(unsafe { std::mem::transmute(let_pop as OpFn) });
                },
                Expr::Set(local, rhs) => {
                    unsafe fn set(reg: Reg, args: &[i64], tape: &mut Tape, mut stack: Stack, mut locals: Stack) {
                        let local = tape.next_usize();
                        locals.set_offset(local, stack.pop());
                        tape.next_eval(reg, args, stack, locals)
                    }
                    compile_inner(ops, rhs);
                    ops.push(unsafe { std::mem::transmute(set as OpFn) });
                    ops.push(*local);
                },
                Expr::While(pred, body) => {
                    // Pred
                    let start = ops.len();
                    compile_inner(ops, pred);
                    // Check
                    unsafe fn while_pred(reg: Reg, args: &[i64], tape: &mut Tape, mut stack: Stack, locals: Stack) {
                        let end_skip = tape.next_usize();
                        let pred = stack.pop();
                        if pred <= 0 {
                            tape.skip(end_skip);
                        }
                        tape.next_eval(reg, args, stack, locals)
                    }
                    ops.push(unsafe { std::mem::transmute(while_pred as OpFn) });
                    let end_fixup = ops.len();
                    ops.push(0);
                    let body_start = ops.len();
                    // Body
                    compile_inner(ops, body);
                    if returns(body) {
                        unsafe fn while_pop(reg: Reg, args: &[i64], tape: &mut Tape, mut stack: Stack, locals: Stack) {
                            stack.pop();
                            tape.next_eval(reg, args, stack, locals)
                        }
                        ops.push(unsafe { std::mem::transmute(while_pop as OpFn) });
                    }
                    // Loop
                    unsafe fn while_loop(reg: Reg, args: &[i64], tape: &mut Tape, mut stack: Stack, locals: Stack) {
                        let unskip = tape.next_usize();
                        tape.unskip(unskip);
                        tape.next_eval(reg, args, stack, locals)
                    }
                    ops.push(unsafe { std::mem::transmute(while_loop as OpFn) });
                    ops.push(ops.len() - start + 1);
                    // Fixup
                    ops[end_fixup] = ops.len() - body_start;
                },
                Expr::Then(a, b) => {
                    compile_inner(ops, a);
                    if returns(a) {
                        unsafe fn then_pop(reg: Reg, args: &[i64], tape: &mut Tape, mut stack: Stack, locals: Stack) {
                            stack.pop();
                            tape.next_eval(reg, args, stack, locals)
                        }
                        ops.push(unsafe { std::mem::transmute(then_pop as OpFn) });
                    }
                    compile_inner(ops, b);
                },
            }
        }

        let mut ops = Vec::new();

        compile_inner(&mut ops, expr);

        unsafe fn ret(reg: Reg, args: &[i64], tape: &mut Tape, mut stack: Stack, locals: Stack) {}
        ops.push(unsafe { std::mem::transmute(ret as OpFn) });

        ops
    }

    unsafe fn execute(prog: &Self::Program<'_>, args: &[i64]) -> i64 {
        let stack_raw = Box::into_raw(vec![0i64; 1024].into_boxed_slice());
        let mut stack = Stack(stack_raw as _);
        let locals_raw = Box::into_raw(vec![0i64; 1024].into_boxed_slice());
        let mut locals = Stack(locals_raw as _);
        Tape(prog.as_ptr(), PhantomData)
            .next_eval(Reg::default(), args, stack, locals);
        (*stack_raw)[0]
    }
}
