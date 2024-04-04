use super::*;

pub struct RegisterClosures;

const REG_COUNT: usize = 2;

impl Vm for RegisterClosures {
    type Program<'a> = Box<
        dyn Fn(
                *const i64,
                *mut i64,
                &mut [i64; REG_COUNT], // r1
            ) -> i64
            + 'a,
    >;

    fn compile(expr: &Expr) -> Self::Program<'_> {
        // A stand-in for expressions that don't return anything
        const UNIT: i64 = 0;

        match expr {
            Expr::Litr(x) => {
                let x = *x;
                Box::new(move |_, _, _| x)
            }
            Expr::Arg(idx) => {
                let idx = *idx;
                Box::new(move |args, _, _| unsafe { *args.add(idx) })
            }
            Expr::Get(local) => match local {
                0 => Box::new(move |_, _, r| r[0]),
                1 => Box::new(move |_, _, r| r[1]),
                _ => {
                    let offset = -1 - *local as isize + REG_COUNT as isize;
                    Box::new(move |_, locals, _| unsafe { *locals.offset(offset) })
                }
            },
            Expr::Add(x, y) => match &**y {
                Expr::Litr(1) => {
                    let x = Self::compile(x);
                    Box::new(move |args, locals, r| x(args, locals, r) + 1)
                }
                Expr::Litr(y) => {
                    let x = Self::compile(x);
                    let y = *y;
                    Box::new(move |args, locals, r| x(args, locals, r) + y)
                }
                Expr::Arg(1) => {
                    let x = Self::compile(x);
                    Box::new(move |args, locals, r| x(args, locals, r) + unsafe { *args.add(1) })
                }
                _ => {
                    let x = Self::compile(x);
                    let y = Self::compile(y);
                    Box::new(move |args, locals, r| x(args, locals, r) + y(args, locals, r))
                }
            },
            Expr::Let(rhs, then) => {
                let rhs = Self::compile(rhs);
                let then = Self::compile(then);
                Box::new(move |args, locals, r| {
                    let rhs = rhs(args, locals, r);
                    unsafe {
                        locals.write(r[1]);
                    }
                    r[1] = r[0];
                    r[0] = rhs;
                    let res = then(args, unsafe { locals.add(1) }, r);
                    r[0] = r[1];
                    unsafe {
                        r[1] = locals.read();
                    }
                    res
                })
            }
            Expr::Set(local, rhs) => {
                let rhs = Self::compile(rhs);
                match local {
                    0 => Box::new(move |args, locals, r| {
                        r[0] = rhs(args, locals, r);
                        UNIT
                    }),
                    1 => Box::new(move |args, locals, r| {
                        r[1] = rhs(args, locals, r);
                        UNIT
                    }),
                    _ => {
                        let offset = -1 - *local as isize + REG_COUNT as isize;
                        Box::new(move |args, locals, r| {
                            let rhs = rhs(args, locals, r);
                            unsafe {
                                *locals.offset(offset) = rhs;
                            }
                            UNIT
                        })
                    }
                }
            }
            Expr::While(pred, body) => {
                let pred = Self::compile(pred);
                let body = Self::compile(body);
                Box::new(move |args, locals, r| {
                    while pred(args, locals, r) > 0 {
                        body(args, locals, r);
                    }
                    UNIT
                })
            }
            Expr::Then(a, b) => {
                let a = Self::compile(a);
                let b = Self::compile(b);
                Box::new(move |args, locals, r| {
                    a(args, locals, r);
                    b(args, locals, r)
                })
            }
            _ => todo!(),
        }
    }

    unsafe fn execute(prog: &Self::Program<'_>, args: &[i64]) -> i64 {
        let mut v = vec![0; 64];
        prog(args.as_ptr(), v.as_mut_ptr(), &mut [0; REG_COUNT])
    }
}
