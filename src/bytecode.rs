use super::*;

pub struct Bytecode;

#[derive(Debug)]
pub enum Op {
    Litr(i64),
    Arg(usize),
    Get(usize),
    Add,
    PushLocal,
    PopLocal,
    SetLocal(usize),
    Pop,
    JmpZN(usize),
    Jmp(usize),
    Ret,
}

impl Vm for Bytecode {
    type Program<'a> = Vec<Op>;

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

        fn compile_inner(ops: &mut Vec<Op>, expr: &Expr) {
            match expr {
                Expr::Litr(x) => ops.push(Op::Litr(*x)),
                Expr::Arg(idx) => ops.push(Op::Arg(*idx)),
                Expr::Get(local) => ops.push(Op::Get(*local)),
                Expr::Add(x, y) => {
                    compile_inner(ops, x);
                    compile_inner(ops, y);
                    ops.push(Op::Add);
                },
                Expr::Let(rhs, then) => {
                    compile_inner(ops, rhs);
                    ops.push(Op::PushLocal);
                    compile_inner(ops, then);
                    ops.push(Op::PopLocal);
                },
                Expr::Set(local, rhs) => {
                    compile_inner(ops, rhs);
                    ops.push(Op::SetLocal(*local));
                },
                Expr::While(pred, body) => {
                    let start = ops.len();
                    compile_inner(ops, pred);
                    let branch_fixup = ops.len();
                    ops.push(Op::JmpZN(0)); // Will be fixed up
                    compile_inner(ops, body);
                    if returns(body) {
                        ops.push(Op::Pop);
                    }
                    ops.push(Op::Jmp(start));
                    ops[branch_fixup] = Op::JmpZN(ops.len());
                },
                Expr::Then(a, b) => {
                    compile_inner(ops, a);
                    if returns(a) {
                        ops.push(Op::Pop);
                    }
                    compile_inner(ops, b);
                },
            }
        }

        let mut ops = Vec::new();

        compile_inner(&mut ops, expr);

        ops.push(Op::Ret);

        ops
    }

    unsafe fn execute(prog: &Self::Program<'_>, args: &[i64]) -> i64 {
        let mut ip = 0;
        let mut stack = Vec::new();
        let mut locals = Vec::new();
        loop {
            let op = prog.get_unchecked(ip);
            ip += 1;
            match op {
                Op::Litr(x) => stack.push(*x),
                Op::Arg(idx) => stack.push(*args.get_unchecked(*idx)),
                Op::Get(local) => stack.push(*locals.get_unchecked(locals.len() - local - 1)),
                Op::Add => {
                    let x = stack.pop().unwrap_unchecked();
                    let y = stack.pop().unwrap_unchecked();
                    stack.push(x + y);
                },
                Op::PushLocal => locals.push(stack.pop().unwrap_unchecked()),
                Op::PopLocal => unsafe { locals.pop().unwrap_unchecked(); },
                Op::SetLocal(local) => {
                    let rhs = stack.pop().unwrap_unchecked();
                    let local_offs = locals.len() - local - 1;
                    unsafe { *locals.get_unchecked_mut(local_offs) = rhs; }
                },
                Op::Pop => unsafe { stack.pop().unwrap_unchecked(); },
                Op::JmpZN(goto) => {
                    if stack.pop().unwrap_unchecked() <= 0 {
                        ip = *goto;
                    }
                },
                Op::Jmp(goto) => ip = *goto,
                Op::Ret => break stack.pop().unwrap_unchecked(),
            }
        }
    }
}
