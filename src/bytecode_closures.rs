use super::*;

pub struct BytecodeClosures;

// #[derive(Debug)]
// pub enum Op {
//     Litr(i64),
//     Arg(usize),
//     Get(usize),
//     Add,
//     PushLocal,
//     PopLocal,
//     SetLocal(usize),
//     Pop,
//     JmpZN(usize),
//     Jmp(usize),
//     Ret,
// }

type OpFn<'a> = Box<dyn Fn(
    &mut usize,
    &[i64], // args
    &mut Vec<i64>, // stack
    &mut Vec<i64>, // locals
) -> bool + 'a>;

impl Vm for BytecodeClosures {
    type Program<'a> = Vec<OpFn<'a>>;

    fn compile(expr: &Expr) -> Self::Program<'_> {
        fn returns(expr: &Expr) -> bool {
            match expr {
                Expr::Litr(_)
                | Expr::Arg(_)
                | Expr::Get(_)
                | Expr::Add(_, _) => true,
                Expr::Let(_, expr) => returns(expr),
                Expr::Set(_, _)
                | Expr::While(_, _)=> false,
                Expr::Then(_, b) => returns(b),
            }
        }

        unsafe fn compile_inner<'a>(ops: &mut Vec<OpFn<'a>>, expr: &'a Expr) {
            match expr {
                Expr::Litr(x) => ops.push(Box::new(move |_, _, stack, _| { stack.push(*x); false })),
                Expr::Arg(idx) => ops.push(Box::new(move |_, args, stack, _| { stack.push(*args.get_unchecked(*idx)); false })),
                Expr::Get(local) => ops.push(Box::new(move |_, _, stack, locals| {
                    stack.push(*locals.get_unchecked(locals.len() - local - 1));
                    false
                })),
                Expr::Add(x, y) => {
                    compile_inner(ops, x);
                    compile_inner(ops, y);
                    ops.push(Box::new(move |_, _, stack, _| {
                        let x = stack.pop().unwrap_unchecked();
                        let y = stack.pop().unwrap_unchecked();
                        stack.push(x + y);
                        false
                    }));
                },
                Expr::Let(rhs, then) => {
                    compile_inner(ops, rhs);
                    ops.push(Box::new(move |_, _, stack, locals| {
                        locals.push(stack.pop().unwrap_unchecked());
                        false
                    }));
                    compile_inner(ops, then);
                    ops.push(Box::new(move |_, _, _, locals| {
                        locals.pop().unwrap_unchecked();
                        false
                    }));
                },
                Expr::Set(local, rhs) => {
                    compile_inner(ops, rhs);
                    ops.push(Box::new(move |_, _, stack, locals| {
                        let rhs = stack.pop().unwrap_unchecked();
                        let local_offs = locals.len() - local - 1;
                        *locals.get_unchecked_mut(local_offs) = rhs;
                        false
                    }));
                },
                Expr::While(pred, body) => {
                    let start = ops.len();
                    compile_inner(ops, pred);
                    let branch_fixup = ops.len();
                    ops.push(Box::new(|_, _, _, _| false)); // Will be fixed up
                    compile_inner(ops, body);
                    if returns(body) {
                        ops.push(Box::new(move |_, _, stack, _| {
                            stack.pop().unwrap_unchecked();
                            false
                        }));
                    }
                    ops.push(Box::new(move |ip, _, _, _| {
                        *ip = start;
                        false
                    }));
                    let end = ops.len();
                    ops[branch_fixup] = Box::new(move |ip, _, stack, _| {
                        if stack.pop().unwrap_unchecked() <= 0 {
                            *ip = end;
                        }
                        false
                    });
                },
                Expr::Then(a, b) => {
                    compile_inner(ops, a);
                    if returns(a) {
                        ops.push(Box::new(move |_, _, stack, _| {
                            stack.pop().unwrap_unchecked();
                            false
                        }));
                    }
                    compile_inner(ops, b);
                },
            }
        }

        let mut ops = Vec::new();

        unsafe { compile_inner(&mut ops, expr); }

        ops.push(Box::new(move |_, _, _, _| true));

        ops
    }

    unsafe fn execute(prog: &Self::Program<'_>, args: &[i64]) -> i64 {
        let mut ip = 0;
        let mut stack = Vec::new();
        let mut locals = Vec::new();
        loop {
            let f = prog.get_unchecked(ip);
            ip += 1;
            if f(&mut ip, args, &mut stack, &mut locals) {
                break stack.pop().unwrap_unchecked();
            }
        }
    }
}
