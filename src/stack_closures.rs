use super::*;

pub struct StackClosures;

type OpFn<'a> = Box<
    dyn Fn(
            &[i64],        // args
            &mut usize,    // ip
            &mut Vec<i64>, // stack
            &mut Vec<i64>, // locals
        ) -> Option<i64>
        + 'a,
>;

impl Vm for StackClosures {
    type Program<'a> = Vec<OpFn<'a>>;

    fn compile(expr: &Expr) -> Self::Program<'_> {
        fn returns(expr: &Expr) -> bool {
            match expr {
                Expr::Litr(_) | Expr::Arg(_) | Expr::Get(_) | Expr::Add(_, _) => true,
                Expr::Let(_, expr) => returns(expr),
                Expr::Set(_, _) | Expr::While(_, _) => false,
                Expr::Then(_, b) => returns(b),
                _ => todo!(),
            }
        }

        fn compile_inner<'a>(ops: &mut Vec<OpFn<'a>>, expr: &'a Expr) {
            match expr {
                Expr::Litr(x) => ops.push(Box::new(move |_, _, stack, _| {
                    stack.push(*x);
                    None
                })),
                Expr::Arg(idx) => ops.push(Box::new(move |args, _, stack, _| {
                    unsafe {
                        stack.push(*args.get_unchecked(*idx));
                    }
                    None
                })),
                Expr::Get(local) => ops.push(Box::new(move |_, _, stack, locals| {
                    unsafe {
                        stack.push(*locals.get_unchecked(locals.len() - local - 1));
                    }
                    None
                })),
                Expr::Add(x, y) => {
                    compile_inner(ops, x);
                    compile_inner(ops, y);
                    ops.push(Box::new(move |_, _, stack, _| {
                        unsafe {
                            let x = stack.pop().unwrap_unchecked();
                            let y = stack.pop().unwrap_unchecked();
                            stack.push(x + y);
                        }
                        None
                    }))
                }
                Expr::Let(rhs, then) => {
                    compile_inner(ops, rhs);
                    ops.push(Box::new(move |_, _, stack, locals| {
                        unsafe {
                            let rhs = stack.pop().unwrap_unchecked();
                            locals.push(rhs);
                        }
                        None
                    }));
                    compile_inner(ops, then);
                    ops.push(Box::new(move |_, _, _, locals| {
                        unsafe {
                            locals.pop().unwrap_unchecked();
                        }
                        None
                    }));
                }
                Expr::Set(local, rhs) => {
                    compile_inner(ops, rhs);
                    ops.push(Box::new(move |_, _, stack, locals| {
                        unsafe {
                            let rhs = stack.pop().unwrap_unchecked();
                            let local_offs = locals.len() - local - 1;
                            *locals.get_unchecked_mut(local_offs) = rhs;
                        }
                        None
                    }));
                }
                Expr::While(pred, body) => {
                    let start = ops.len();
                    compile_inner(ops, pred);
                    let branch_fixup = ops.len();
                    ops.push(Box::new(move |_, _, _, _| None));
                    compile_inner(ops, body);
                    if returns(body) {
                        ops.push(Box::new(move |_, _, stack, _| {
                            unsafe {
                                stack.pop().unwrap_unchecked();
                            }
                            None
                        }));
                    }
                    ops.push(Box::new(move |_, ip, _, _| {
                        *ip = start;
                        None
                    }));
                    let end = ops.len();
                    ops[branch_fixup] = Box::new(move |_, ip, stack, _| {
                        unsafe {
                            let pred = stack.pop().unwrap_unchecked();
                            if pred <= 0 {
                                *ip = end;
                            }
                        }
                        None
                    });
                }
                Expr::Then(a, b) => {
                    compile_inner(ops, a);
                    if returns(a) {
                        ops.push(Box::new(move |_, _, stack, _| {
                            unsafe {
                                stack.pop().unwrap_unchecked();
                            }
                            None
                        }));
                    }
                    compile_inner(ops, b);
                }
                _ => todo!(),
            }
        }

        let mut ops = Vec::new();

        compile_inner(&mut ops, expr);

        ops.push(Box::new(move |_, _, stack, _| unsafe {
            Some(stack.pop().unwrap_unchecked())
        }));

        ops
    }

    unsafe fn execute(prog: &Self::Program<'_>, args: &[i64]) -> i64 {
        let mut ip = 0;
        let mut stack = Vec::new();
        let mut locals = Vec::new();
        loop {
            let f = prog.get_unchecked(ip);
            ip += 1;
            if let Some(res) = f(args, &mut ip, &mut stack, &mut locals) {
                break res;
            }
        }
    }
}
