use super::*;
use std::marker::PhantomData;

pub struct TapeContinuations;

#[derive(Default)]
struct Reg {
    r0: i64, // Return value
    r1: i64, // Scratch register (currently unused)
}

type OpFn = unsafe fn(reg: Reg, *const i64, Tape, Stack);

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
        self.0.sub(offset).write(x);
    }

    unsafe fn get_offset(&self, offset: usize) -> i64 {
        self.0.sub(offset).read()
    }
}

#[derive(Copy, Clone)]
struct Tape<'a>(*const usize, PhantomData<&'a ()>);

impl<'a> Tape<'a> {
    unsafe fn this_eval(self, reg: Reg, args: *const i64, mut stack: Stack) {
        let f = std::mem::transmute::<_, OpFn>(self.0.read());
        f(reg, args, self, stack)
    }
    //unsafe fn next_fn(&mut self) -> OpFn { let res = std::mem::transmute(self.0.read()); self.0 = self.0.add(1); res }
    #[inline(always)]
    unsafe fn next_eval(mut self, reg: Reg, args: *const i64, mut stack: Stack) {
        self.0 = self.0.add(1);
        let f = std::mem::transmute::<_, OpFn>(self.0.read());
        f(reg, args, self, stack)
    }
    #[inline(always)]
    unsafe fn next_int(&mut self) -> i64 {
        self.0 = self.0.add(1);
        std::mem::transmute(self.0.read())
    }
    unsafe fn next_usize(&mut self) -> usize {
        self.0 = self.0.add(1);
        self.0.read()
    }
    unsafe fn skip(&mut self, n: usize) {
        self.0 = self.0.add(n);
    }
    unsafe fn unskip(&mut self, n: usize) {
        self.0 = self.0.sub(n);
    }
}

impl Vm for TapeContinuations {
    type Program<'a> = Vec<usize>;

    fn compile(expr: &Expr) -> Self::Program<'_> {
        enum Scope<'a> {
            None,
            Intermediate(&'a Self),
            Local(&'a Self),
        }

        impl<'a> Scope<'a> {
            // Convert the offset of a local to an offset on the stack
            // i.e: given a stack like [x, #1, y, #0, z] we'd compute 4 as the offset of #1
            fn local_offset_to_stack_offset(&self, offset: usize) -> usize {
                match self {
                    Self::None => unreachable!("local not in stack"),
                    Self::Intermediate(parent) => parent.local_offset_to_stack_offset(offset) + 1,
                    Self::Local(parent) if offset == 0 => 0,
                    Self::Local(parent) => parent.local_offset_to_stack_offset(offset - 1) + 1,
                }
            }
        }

        fn compile_inner(ops: &mut Vec<usize>, expr: &Expr, scope: &Scope) {
            fn returns(expr: &Expr) -> bool {
                match expr {
                    Expr::Litr(_) | Expr::Arg(_) | Expr::Get(_) | Expr::Add(_, _) => true,
                    Expr::Let(_, expr) => returns(expr),
                    Expr::Set(_, _) | Expr::While(_, _) => false,
                    Expr::Then(_, b) => returns(b),
                }
            }

            match expr {
                Expr::Litr(x) => {
                    unsafe fn litr(mut reg: Reg, args: *const i64, mut tape: Tape, mut stack: Stack) {
                        let x = tape.next_int();
                        reg.r0 = x;
                        tape.next_eval(reg, args, stack);
                    }
                    ops.push(unsafe { std::mem::transmute(litr as OpFn) });
                    ops.push(*x as usize);
                },
                Expr::Arg(idx) => {
                    unsafe fn arg(mut reg: Reg, args: *const i64, mut tape: Tape, mut stack: Stack) {
                        let idx = tape.next_usize();
                        let x = args.add(idx).read();
                        reg.r0 = x;
                        tape.next_eval(reg, args, stack)
                    }
                    ops.push(unsafe { std::mem::transmute(arg as OpFn) });
                    ops.push(*idx);
                },
                Expr::Get(local) => {
                    unsafe fn get(mut reg: Reg, args: *const i64, mut tape: Tape, mut stack: Stack) {
                        let local = tape.next_usize();
                        let x = stack.get_offset(local);
                        reg.r0 = x;
                        tape.next_eval(reg, args, stack)
                    }
                    ops.push(unsafe { std::mem::transmute(get as OpFn) });
                    ops.push(scope.local_offset_to_stack_offset(*local) + 1);
                },
                Expr::Add(x, y) => match &**y {
                    Expr::Litr(1) => {
                        unsafe fn add_one(mut reg: Reg, args: *const i64, tape: Tape, mut stack: Stack) {
                            reg.r0 += 1;
                            tape.next_eval(reg, args, stack)
                        }
                        compile_inner(ops, x, scope);
                        ops.push(unsafe { std::mem::transmute(add_one as OpFn) });
                    },
                    Expr::Litr(y) => {
                        unsafe fn add_litr(mut reg: Reg, args: *const i64, mut tape: Tape, mut stack: Stack) {
                            let y = tape.next_int();
                            reg.r0 += y;
                            tape.next_eval(reg, args, stack)
                        }
                        compile_inner(ops, x, scope);
                        ops.push(unsafe { std::mem::transmute(add_litr as OpFn) });
                        ops.push(*y as usize);
                    },
                    Expr::Arg(1) => {
                        unsafe fn add_arg1(mut reg: Reg, args: *const i64, mut tape: Tape, mut stack: Stack) {
                            let y = args.add(1).read();
                            reg.r0 += y;
                            tape.next_eval(reg, args, stack)
                        }
                        compile_inner(ops, x, scope);
                        ops.push(unsafe { std::mem::transmute(add_arg1 as OpFn) });
                    },
                    _ => {
                        unsafe fn add_swap(mut reg: Reg, args: *const i64, tape: Tape, mut stack: Stack) {
                            stack.push(reg.r0);
                            tape.next_eval(reg, args, stack)
                        }
                        unsafe fn add(mut reg: Reg, args: *const i64, tape: Tape, mut stack: Stack) {
                            let y = stack.pop();
                            reg.r0 += y;
                            tape.next_eval(reg, args, stack)
                        }
                        compile_inner(ops, x, scope);
                        ops.push(unsafe { std::mem::transmute(add_swap as OpFn) });
                        compile_inner(ops, y, &Scope::Intermediate(scope));
                        ops.push(unsafe { std::mem::transmute(add as OpFn) });
                    },
                },
                Expr::Let(rhs, then) => {
                    compile_inner(ops, rhs, scope);
                    unsafe fn let_push(reg: Reg, args: *const i64, tape: Tape, mut stack: Stack) {
                        stack.push(reg.r0);
                        tape.next_eval(reg, args, stack)
                    }
                    ops.push(unsafe { std::mem::transmute(let_push as OpFn) });
                    compile_inner(ops, then, &Scope::Local(scope));
                    unsafe fn let_pop(reg: Reg, args: *const i64, tape: Tape, mut stack: Stack) {
                        stack.pop();
                        tape.next_eval(reg, args, stack)
                    }
                    ops.push(unsafe { std::mem::transmute(let_pop as OpFn) });
                },
                Expr::Set(local, rhs) => if let Expr::Add(a, b) = &**rhs
                    && let Expr::Get(y) = &**a
                    && local == y
                {
                    compile_inner(ops, b, scope);
                    let local_offset = scope.local_offset_to_stack_offset(*local);
                    unsafe fn add_assign_at<const N: usize>(reg: Reg, args: *const i64, tape: Tape, mut stack: Stack) {
                        let b = reg.r0;
                        let a = stack.get_offset(N + 1);
                        stack.set_offset(N + 1, a + b);
                        tape.next_eval(reg, args, stack)
                    }
                    match local_offset {
                        0 => ops.push(unsafe { std::mem::transmute(add_assign_at::<0> as OpFn) }),
                        1 => ops.push(unsafe { std::mem::transmute(add_assign_at::<1> as OpFn) }),
                        _ => {
                            unsafe fn add_assign(reg: Reg, args: *const i64, mut tape: Tape, mut stack: Stack) {
                                let local = tape.next_usize();
                                // let b = stack.pop();
                                let b = reg.r0;
                                let a = stack.get_offset(local);
                                stack.set_offset(local, a + b);
                                tape.next_eval(reg, args, stack)
                            }
                            ops.push(unsafe { std::mem::transmute(add_assign as OpFn) });
                            ops.push(local_offset + 1);
                        },
                    }
                } else {
                    unsafe fn set(reg: Reg, args: *const i64, mut tape: Tape, mut stack: Stack) {
                        let local = tape.next_usize();
                        let x = reg.r0;
                        stack.set_offset(local, x);
                        tape.next_eval(reg, args, stack)
                    }
                    compile_inner(ops, rhs, scope);
                    ops.push(unsafe { std::mem::transmute(set as OpFn) });
                    ops.push(scope.local_offset_to_stack_offset(*local) + 1);
                },
                Expr::While(pred, body) => {
                    // Pred
                    let start = ops.len();
                    compile_inner(ops, pred, scope);
                    // Check
                    unsafe fn while_pred(reg: Reg, args: *const i64, mut tape: Tape, mut stack: Stack) {
                        let end_skip = tape.next_usize();
                        let pred = reg.r0;
                        if pred <= 0 {
                            tape.skip(end_skip);
                        }
                        tape.next_eval(reg, args, stack)
                    }
                    ops.push(unsafe { std::mem::transmute(while_pred as OpFn) });
                    let end_fixup = ops.len();
                    ops.push(0);
                    let body_start = ops.len();
                    // Body
                    compile_inner(ops, body, scope);
                    // Loop
                    unsafe fn while_loop(reg: Reg, args: *const i64, mut tape: Tape, mut stack: Stack) {
                        let unskip = tape.next_usize();
                        tape.unskip(unskip);
                        tape.next_eval(reg, args, stack)
                    }
                    ops.push(unsafe { std::mem::transmute(while_loop as OpFn) });
                    ops.push(ops.len() - start + 1);
                    // Fixup
                    ops[end_fixup] = ops.len() - body_start;
                },
                Expr::Then(a, b) => {
                    compile_inner(ops, a, scope);
                    compile_inner(ops, b, scope);
                },
            }
        }

        let mut ops = Vec::new();

        compile_inner(&mut ops, expr, &Scope::None);

        unsafe fn ret(reg: Reg, args: *const i64, tape: Tape, mut stack: Stack) {}
        ops.push(unsafe { std::mem::transmute(ret as OpFn) });

        ops
    }

    unsafe fn execute(prog: &Self::Program<'_>, args: &[i64]) -> i64 {
        let stack_raw = Box::into_raw(vec![0i64; 1024].into_boxed_slice());
        let mut stack = Stack(stack_raw as _);
        Tape(prog.as_ptr(), PhantomData).this_eval(Reg::default(), args.as_ptr(), stack);
        (*stack_raw)[0]
    }
}
