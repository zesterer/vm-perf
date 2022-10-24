use super::*;

pub struct RegisterClosures;

const REG_COUNT: usize = 2;

impl Vm for RegisterClosures {
    type Program<'a> = Box<dyn Fn(
        &[i64],
        &mut Vec<i64>,
        &mut i64, // r0
        &mut i64, // r1
    ) -> i64 + 'a>;

    fn compile(expr: &Expr) -> Self::Program<'_> {
        // A stand-in for expressions that don't return anything
        const UNIT: i64 = 0;

        match expr {
            Expr::Litr(x) => Box::new(move |_, _, _, _| *x),
            Expr::Arg(idx) => Box::new(move |args, _, _, _| unsafe { *args.get_unchecked(*idx) }),
            Expr::Get(local) => match local {
                0 => Box::new(move |_, _, r0, _| *r0),
                1 => Box::new(move |_, _, _, r1| *r1),
                _ => Box::new(move |_, locals, _, _| unsafe { *locals.get_unchecked(locals.len() - local + (REG_COUNT + 1)) }),
            },
            Expr::Add(x, y) => {
                let x = Self::compile(x);
                let y = Self::compile(y);
                Box::new(move |args, locals, r0, r1| x(args, locals, r0, r1) + y(args, locals, r0, r1))
            },
            Expr::Let(rhs, then) => {
                let rhs = Self::compile(rhs);
                let then = Self::compile(then);
                Box::new(move |args, locals, r0, r1| {
                    let rhs = rhs(args, locals, r0, r1);
                    locals.push(*r1);
                    *r1 = *r0;
                    *r0 = rhs;
                    let res = then(args, locals, r0, r1);
                    *r0 = *r1;
                    unsafe { *r1 = locals.pop().unwrap_unchecked(); }
                    res
                })
            },
            Expr::Set(local, rhs) => {
                let rhs = Self::compile(rhs);
                    match local {
                    0 => Box::new(move |args, locals, r0, r1| { *r0 = rhs(args, locals, r0, r1); UNIT }),
                    1 => Box::new(move |args, locals, r0, r1| { *r1 = rhs(args, locals, r0, r1); UNIT }),
                    _ => Box::new(move |args, locals, r0, r1| {
                        let rhs = rhs(args, locals, r0, r1);
                        let local_offs = locals.len() - local + (REG_COUNT + 1);
                        unsafe { *locals.get_unchecked_mut(local_offs) = rhs; }
                        UNIT
                    }),
                }
            },
            Expr::While(pred, body) => {
                let pred = Self::compile(pred);
                let body = Self::compile(body);
                Box::new(move |args, locals, r0, r1| {
                    while pred(args, locals, r0, r1) > 0 {
                        body(args, locals, r0, r1);
                    }
                    UNIT
                })
            },
            Expr::Then(a, b) => {
                let a = Self::compile(a);
                let b = Self::compile(b);
                Box::new(move |args, locals, r0, r1| {
                    a(args, locals, r0, r1);
                    b(args, locals, r0, r1)
                })
            },
        }
    }

    unsafe fn execute(prog: &Self::Program<'_>, args: &[i64]) -> i64 {
        prog(args, &mut Vec::new(), &mut 0, &mut 0)
    }
}
