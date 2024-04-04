use super::*;

pub struct Closures;

impl Vm for Closures {
    type Program<'a> = Box<dyn Fn(&[i64], &mut Vec<i64>) -> i64 + 'a>;

    fn compile(expr: &Expr) -> Self::Program<'_> {
        // A stand-in for expressions that don't return anything
        const UNIT: i64 = 0;

        match expr {
            Expr::Litr(x) => Box::new(move |_, _| *x),
            Expr::Arg(idx) => Box::new(move |args, _| unsafe { *args.get_unchecked(*idx) }),
            Expr::Get(local) => Box::new(move |_, locals| unsafe {
                *locals.get_unchecked(locals.len() - local - 1)
            }),
            Expr::Add(x, y) => {
                let x = Self::compile(x);
                let y = Self::compile(y);
                Box::new(move |args, locals| x(args, locals) + y(args, locals))
            }
            Expr::Let(rhs, then) => {
                let rhs = Self::compile(rhs);
                let then = Self::compile(then);
                Box::new(move |args, locals| {
                    let rhs = rhs(args, locals);
                    locals.push(rhs);
                    let res = then(args, locals);
                    unsafe {
                        locals.pop().unwrap_unchecked();
                    }
                    res
                })
            }
            Expr::Set(local, rhs) => {
                let rhs = Self::compile(rhs);
                Box::new(move |args, locals| {
                    let rhs = rhs(args, locals);
                    let local_offs = locals.len() - local - 1;
                    unsafe {
                        *locals.get_unchecked_mut(local_offs) = rhs;
                    }
                    UNIT
                })
            }
            Expr::While(pred, body) => {
                let pred = Self::compile(pred);
                let body = Self::compile(body);
                Box::new(move |args, locals| {
                    while pred(args, locals) > 0 {
                        body(args, locals);
                    }
                    UNIT
                })
            }
            Expr::Then(a, b) => {
                let a = Self::compile(a);
                let b = Self::compile(b);
                Box::new(move |args, locals| {
                    a(args, locals);
                    b(args, locals)
                })
            }
        }
    }

    unsafe fn execute(prog: &Self::Program<'_>, args: &[i64]) -> i64 {
        prog(args, &mut Vec::new())
    }
}
