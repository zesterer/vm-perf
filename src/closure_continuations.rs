use super::*;

pub struct ClosureContinuations;

type Func<'a, E> = Box<dyn Fn(*const i64, *mut i64, i64) -> Result<i64, E> + 'a>;

impl Vm for ClosureContinuations {
    type Program<'a> = Func<'a, core::convert::Infallible>;

    fn compile(expr: &Expr) -> Self::Program<'_> {
        Self::compile(expr, ())
    }

    unsafe fn execute(prog: &Self::Program<'_>, args: &[i64]) -> i64 {
        let mut v = vec![0; 64];
        prog(args.as_ptr(), v.as_mut_ptr(), 0).unwrap()
    }
}

trait MaybeCont<'a, E> {
    #[inline(always)]
    fn cont(&self, args: *const i64, locals: *mut i64, result: i64) -> Result<i64, E> {
        Ok(result)
    }
    fn map(self, f: impl FnOnce(Self) -> Func<'a, E>) -> Self
    where
        Self: Sized,
    {
        self
    }
}

impl<'a, E> MaybeCont<'a, E> for () {}
impl<'a, E> MaybeCont<'a, E> for Func<'a, E> {
    #[inline(always)]
    fn cont(&self, args: *const i64, locals: *mut i64, result: i64) -> Result<i64, E> {
        (*self)(args, locals, result)
    }
    fn map(self, f: impl FnOnce(Self) -> Func<'a, E>) -> Self
    where
        Self: Sized,
    {
        f(self)
    }
}

fn make_cont<'a, E, F: Fn(*const i64, *mut i64, i64) -> Result<i64, E> + 'a>(f: F) -> Func<'a, E> {
    Box::new(f)
}

trait Throw<'a>: Sized {
    fn throw(args: *const i64, locals: *mut i64, payload: i64) -> Self {
        panic!("Not a throwable error!");
    }
}
impl<'a> Throw<'a> for core::convert::Infallible {}

struct ResumeState {
    args: *const i64,
    locals: *mut i64,
    payload: i64,
}
impl<'a> Throw<'a> for Box<ResumeState> {
    fn throw(args: *const i64, locals: *mut i64, payload: i64) -> Self {
        Box::new(ResumeState { args, locals, payload })
    }
}

impl ClosureContinuations {
    fn compile<'a, E: Throw<'a> + 'a>(expr: &'a Expr, cont: impl MaybeCont<'a, E> + 'a) -> Func<'a, E> {
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
                            cont.cont(args, locals, r + y?)
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
                    while pred(args, locals, 0)? > 0 {
                        body(args, locals, 0)?;
                    }
                    cont.cont(args, locals, UNIT)
                })
            }
            Expr::Then(a, b) => {
                let b = Self::compile(b, cont);
                Self::compile(a, make_cont(move |args, locals, _b| b(args, locals, 0)))
            },
            Expr::Throw(a) => {
                Self::compile(a, make_cont(move |args, locals, x| Err(E::throw(args, locals, x))))
            },
            Expr::Catch(a) => {
                let a = Self::compile::<Box<_>>(a, ());
                make_cont(move |args, locals, _| {
                    match a(args, locals, 0) {
                        Ok(x) => Ok(x),
                        Err(resume) => panic!("Caught {}!", resume.payload),
                    }
                })
            },
        }
    }
}

#[test]
fn throw_catch() {
    fn create_expr() -> Expr {
        // let mut total = 0;
        // let mut count = args[0];
        // while count > 0 {
        //     total = total + args[1];
        //     count = count - 1;
        // }
        // total
        Expr::Let(
            Box::new(Expr::Litr(0)), // total
            Box::new(Expr::Then(
                Box::new(Expr::Catch(
                    Box::new(Expr::Let(
                        Box::new(Expr::Arg(0)), // counter
                        Box::new(Expr::While(
                            Box::new(Expr::Get(0)),
                            Box::new(Expr::Then(
                                Box::new(Expr::Set(
                                    1,
                                    Box::new(Expr::Add(Box::new(Expr::Get(1)), Box::new(Expr::Arg(1)))),
                                )),
                                Box::new(Expr::Then(
                                    Box::new(Expr::Throw(Box::new(Expr::Litr(42)))),
                                    Box::new(Expr::Set(
                                        0,
                                        Box::new(Expr::Add(Box::new(Expr::Get(0)), Box::new(Expr::Litr(-1)))),
                                    )),
                                )),
                            )),
                        )),
                    )),
                )),
                Box::new(Expr::Get(0)), // total
            )),
        )
    }
    
    let expr = create_expr();
    let prog = <ClosureContinuations as Vm>::compile(&expr);
    unsafe { ClosureContinuations::execute(&prog, &[1000, 13]); }
}
