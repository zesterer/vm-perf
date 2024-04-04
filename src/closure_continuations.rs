use super::*;

pub struct ClosureContinuations;

impl Vm for ClosureContinuations {
    type Program<'a> = Box<dyn Fn(*const i64, *mut i64, i64) -> i64 + 'a>;

    fn compile(expr: &Expr) -> Self::Program<'_> {
        Self::compile(expr, ())
    }

    unsafe fn execute(prog: &Self::Program<'_>, args: &[i64]) -> i64 {
        let mut v = vec![0; 64];
        prog(args.as_ptr(), v.as_mut_ptr(), 0)
    }
}

trait MaybeCont<'a> {
    #[inline(always)]
    fn cont(&self, args: *const i64, locals: *mut i64, result: i64) -> i64 {
        result
    }
    fn map(self, f: impl FnOnce(Self) -> Box<dyn Fn(*const i64, *mut i64, i64) -> i64 + 'a>) -> Self
    where
        Self: Sized,
    {
        self
    }
}

impl<'a> MaybeCont<'a> for () {}
impl<'a> MaybeCont<'a> for Box<dyn Fn(*const i64, *mut i64, i64) -> i64 + 'a> {
    #[inline(always)]
    fn cont(&self, args: *const i64, locals: *mut i64, result: i64) -> i64 {
        (*self)(args, locals, result)
    }
    fn map(self, f: impl FnOnce(Self) -> Box<dyn Fn(*const i64, *mut i64, i64) -> i64 + 'a>) -> Self
    where
        Self: Sized,
    {
        f(self)
    }
}

fn make_cont<'a, F: Fn(*const i64, *mut i64, i64) -> i64 + 'a>(
    f: F,
) -> Box<dyn Fn(*const i64, *mut i64, i64) -> i64 + 'a> {
    Box::new(f)
}

impl ClosureContinuations {
    fn compile<'a>(expr: &'a Expr, cont: impl MaybeCont<'a> + 'a) -> <Self as Vm>::Program<'a> {
        // A stand-in for expressions that don't return anything
        const UNIT: i64 = 0;

        match expr {
            Expr::Litr(x) => {
                let x = *x;
                Box::new(move |args, locals, _| cont.cont(args, locals, x))
            }
            Expr::Arg(idx) => match idx {
                0 => Box::new(move |args, locals, _| {
                    cont.cont(args, locals, unsafe { *args.add(0) })
                }),
                1 => Box::new(move |args, locals, _| {
                    cont.cont(args, locals, unsafe { *args.add(1) })
                }),
                _ => {
                    let idx = *idx;
                    Box::new(move |args, locals, _| {
                        cont.cont(args, locals, unsafe { *args.add(idx) })
                    })
                }
            },
            Expr::Get(local) => match local {
                0 => Box::new(move |args, locals, _| {
                    cont.cont(args, locals, unsafe { *locals.offset(-1) })
                }),
                1 => Box::new(move |args, locals, _| {
                    cont.cont(args, locals, unsafe { *locals.offset(-2) })
                }),
                _ => {
                    let offset = -1 - *local as isize;
                    Box::new(move |args, locals, _| {
                        cont.cont(args, locals, unsafe { *locals.offset(offset) })
                    })
                }
            },
            Expr::Add(x, y) => match &**y {
                Expr::Litr(1) => Self::compile(
                    x,
                    make_cont(move |args, locals, r| cont.cont(args, locals, r + 1)),
                ),
                Expr::Litr(-1) => Self::compile(
                    x,
                    make_cont(move |args, locals, r| cont.cont(args, locals, r - 1)),
                ),

                Expr::Litr(y) => {
                    let y = *y;
                    Self::compile(
                        x,
                        make_cont(move |args, locals, r| cont.cont(args, locals, r + y)),
                    )
                }
                Expr::Arg(1) => Self::compile(
                    x,
                    make_cont(move |args, locals, r| {
                        cont.cont(args, locals, r + unsafe { *args.add(1) })
                    }),
                ),
                _ => {
                    let y = Self::compile(y, ());
                    Self::compile(
                        x,
                        make_cont(move |args, locals, r| {
                            let y = y(args, locals, 0);
                            cont.cont(args, locals, r + y)
                        }),
                    )
                }
            },
            Expr::Let(rhs, then) => {
                let then = Self::compile(
                    then,
                    cont.map(|cont| {
                        make_cont(move |args, locals: *mut i64, r| {
                            cont.cont(args, unsafe { locals.offset(-1) }, r)
                        })
                    }),
                );
                Self::compile(
                    rhs,
                    make_cont(move |args, locals, r| {
                        unsafe {
                            locals.write(r);
                        }
                        then(args, unsafe { locals.add(1) }, 0)
                    }),
                )
            }
            Expr::Set(local, rhs) => match local {
                0 => Self::compile(
                    rhs,
                    make_cont(move |args, locals, r| {
                        unsafe {
                            locals.offset(-1).write(r);
                        }
                        cont.cont(args, locals, UNIT)
                    }),
                ),
                1 => Self::compile(
                    rhs,
                    make_cont(move |args, locals, r| {
                        unsafe {
                            locals.offset(-2).write(r);
                        }
                        cont.cont(args, locals, UNIT)
                    }),
                ),
                _ => {
                    let offset = -1 - *local as isize;
                    Self::compile(
                        rhs,
                        make_cont(move |args, locals, r| {
                            unsafe {
                                locals.offset(offset).write(r);
                            }
                            cont.cont(args, locals, UNIT)
                        }),
                    )
                }
            },
            Expr::While(pred, body) => {
                let pred = Self::compile(pred, ());
                let body = Self::compile(body, ());
                Box::new(move |args, locals, _| {
                    while pred(args, locals, 0) > 0 {
                        body(args, locals, 0);
                    }
                    cont.cont(args, locals, UNIT)
                })
            }
            Expr::Then(a, b) => {
                let b = Self::compile(b, cont);
                Self::compile(a, make_cont(move |args, locals, _b| b(args, locals, 0)))
            }
        }
    }
}
