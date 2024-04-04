use super::*;

pub struct Walker;

impl Vm for Walker {
    type Program<'a> = &'a Expr;

    fn compile(expr: &Expr) -> Self::Program<'_> {
        expr
    }

    unsafe fn execute(expr: &Self::Program<'_>, args: &[i64]) -> i64 {
        unsafe fn execute_inner(expr: &Expr, args: &[i64], locals: &mut Vec<i64>) -> i64 {
            // A stand-in for expressions that don't return anything
            const UNIT: i64 = 0;

            match expr {
                Expr::Litr(x) => *x,
                Expr::Arg(idx) => *args.get_unchecked(*idx),
                Expr::Get(local) => *locals.get_unchecked(locals.len() - local - 1),
                Expr::Add(x, y) => execute_inner(x, args, locals) + execute_inner(y, args, locals),
                Expr::Let(rhs, then) => {
                    let rhs = execute_inner(rhs, args, locals);
                    locals.push(rhs);
                    let res = execute_inner(then, args, locals);
                    locals.pop().unwrap_unchecked();
                    res
                }
                Expr::Set(local, rhs) => {
                    let rhs = execute_inner(rhs, args, locals);
                    let local_offs = locals.len() - local - 1;
                    *locals.get_unchecked_mut(local_offs) = rhs;
                    UNIT
                }
                Expr::While(pred, body) => {
                    while execute_inner(pred, args, locals) > 0 {
                        execute_inner(body, args, locals);
                    }
                    UNIT
                }
                Expr::Then(a, b) => {
                    execute_inner(a, args, locals);
                    execute_inner(b, args, locals)
                }
            }
        }

        execute_inner(expr, args, &mut Vec::new())
    }
}
