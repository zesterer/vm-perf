use super::*;
use std::marker::PhantomData;

pub struct TapeClosures;

type OpFn = unsafe fn(&[i64], &mut Tape, &mut Vec<i64>) -> i64;

#[derive(Copy, Clone)]
struct Tape<'a>(*const usize, PhantomData<&'a ()>);

impl<'a> Tape<'a> {
    //unsafe fn next_fn(&mut self) -> OpFn { let res = std::mem::transmute(self.0.read()); self.0 = self.0.add(1); res }
    unsafe fn next_eval(&mut self, args: &[i64], locals: &mut Vec<i64>) -> i64 {
        let f = std::mem::transmute::<_, OpFn>(self.0.read());
        self.0 = self.0.add(1);
        f(args, self, locals)
    }
    unsafe fn next_int(&mut self) -> i64 { let res = std::mem::transmute(self.0.read()); self.0 = self.0.add(1); res }
    unsafe fn next_usize(&mut self) -> usize { let res = self.0.read(); self.0 = self.0.add(1); res }
    unsafe fn skip(&mut self, n: usize) { self.0 = self.0.add(n); }
}

impl Vm for TapeClosures {
    type Program<'a> = Vec<usize>;

    fn compile(expr: &Expr) -> Self::Program<'_> {
        fn compile_inner(ops: &mut Vec<usize>, expr: &Expr) {
            // A stand-in for expressions that don't return anything
            const UNIT: i64 = 0;

            match expr {
                Expr::Litr(x) => {
                    unsafe fn f(_: &[i64], tape: &mut Tape, _: &mut Vec<i64>) -> i64 {
                        tape.next_int()
                    }
                    ops.push(unsafe { std::mem::transmute(f as OpFn) });
                    ops.push(*x as usize);
                },
                Expr::Arg(idx) => {
                    unsafe fn f(args: &[i64], tape: &mut Tape, _: &mut Vec<i64>) -> i64 {
                        let idx = tape.next_usize();
                        *args.get_unchecked(idx)
                    }
                    ops.push(unsafe { std::mem::transmute(f as OpFn) });
                    ops.push(*idx);
                },
                Expr::Get(local) => {
                    unsafe fn f(_: &[i64], tape: &mut Tape, locals: &mut Vec<i64>) -> i64 {
                        let local = tape.next_usize();
                        *locals.get_unchecked(locals.len() - local - 1)
                    }
                    ops.push(unsafe { std::mem::transmute(f as OpFn) });
                    ops.push(*local);
                },
                Expr::Add(x, y) => {
                    unsafe fn f(args: &[i64], tape: &mut Tape, locals: &mut Vec<i64>) -> i64 {
                        let x = tape.next_eval(args, locals);
                        let y = tape.next_eval(args, locals);
                        x + y
                    }
                    ops.push(unsafe { std::mem::transmute(f as OpFn) });
                    compile_inner(ops, x);
                    compile_inner(ops, y);
                },
                Expr::Let(rhs, then) => {
                    unsafe fn f(args: &[i64], tape: &mut Tape, locals: &mut Vec<i64>) -> i64 {
                        let rhs = tape.next_eval(args, locals);
                        locals.push(rhs);
                        let then = tape.next_eval(args, locals);
                        locals.pop().unwrap_unchecked();
                        then
                    }
                    ops.push(unsafe { std::mem::transmute(f as OpFn) });
                    compile_inner(ops, rhs);
                    compile_inner(ops, then);
                },
                Expr::Set(local, rhs) => {
                    unsafe fn f(args: &[i64], tape: &mut Tape, locals: &mut Vec<i64>) -> i64 {
                        let rhs = tape.next_eval(args, locals);
                        let local = tape.next_usize();
                        let local_offs = locals.len() - local - 1;
                        *locals.get_unchecked_mut(local_offs) = rhs;
                        UNIT
                    }
                    ops.push(unsafe { std::mem::transmute(f as OpFn) });
                    compile_inner(ops, rhs);
                    ops.push(*local);
                },
                Expr::While(pred, body) => {
                    unsafe fn f(args: &[i64], tape: &mut Tape, locals: &mut Vec<i64>) -> i64 {
                        let end_skip = tape.next_usize();
                        let old_tape = *tape;
                        while tape.next_eval(args, locals) > 0 {
                            tape.next_eval(args, locals); // body
                            *tape = old_tape;
                        }
                        tape.skip(end_skip);
                        UNIT
                    }
                    ops.push(unsafe { std::mem::transmute(f as OpFn) });
                    let end_fixup = ops.len();
                    ops.push(0);
                    compile_inner(ops, pred);
                    let body_start = ops.len();
                    compile_inner(ops, body);
                    ops[end_fixup] = ops.len() - body_start;
                },
                Expr::Then(a, b) => {
                    unsafe fn f(args: &[i64], tape: &mut Tape, locals: &mut Vec<i64>) -> i64 {
                        tape.next_eval(args, locals);
                        tape.next_eval(args, locals)
                    }
                    ops.push(unsafe { std::mem::transmute(f as OpFn) });
                    compile_inner(ops, a);
                    compile_inner(ops, b);
                },
            }
        }

        let mut ops = Vec::new();

        compile_inner(&mut ops, expr);

        ops
    }

    unsafe fn execute(prog: &Self::Program<'_>, args: &[i64]) -> i64 {
        Tape(prog.as_ptr(), PhantomData)
            .next_eval(args, &mut Vec::new())
    }
}
