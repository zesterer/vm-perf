use super::*;
use core::marker::PhantomData;

pub struct ClosureContinuations;

impl Vm for ClosureContinuations {
    type Program<'a> = Func<'a>;

    fn compile(expr: &Expr) -> Self::Program<'_> {
        unsafe { Self::compile(expr, ()) }
    }

    unsafe fn execute(prog: &Self::Program<'_>, args: &[i64]) -> i64 {
        let mut v = vec![0; 64];
        prog.invoke(args.as_ptr(), v.as_mut_ptr(), 0)
    }
}

trait MaybeCont<'a> {
    #[inline(always)]
    fn cont(&self, args: *const i64, locals: *mut i64, result: i64) -> i64 {
        result
    }
    fn map(self, f: impl FnOnce(Self) -> Func<'a>) -> Self
    where
        Self: Sized,
    {
        self
    }
}

impl<'a> MaybeCont<'a> for () {}
impl<'a> MaybeCont<'a> for Func<'a> {
    #[inline(always)]
    fn cont(&self, args: *const i64, locals: *mut i64, result: i64) -> i64 {
        self.invoke(args, locals, result)
    }
    fn map(self, f: impl FnOnce(Self) -> Func<'a>) -> Self
    where
        Self: Sized,
    {
        f(self)
    }
}

// Sadly, rustc currently does a poor job of generating good vtable dispatch code for functions.
// This is the solution: a custom wide pointer that uses a combination of inlining and transmutation to do the right
// thing.
pub struct Func<'a> {
    f: unsafe fn(*const (), *const i64, *mut i64, i64) -> i64,
    data: *const (),
    phantom: PhantomData<&'a ()>,
}

impl<'a> Func<'a> {
    #[inline(always)]
    pub fn invoke(&self, args: *const i64, locals: *mut i64, ret: i64) -> i64 {
        unsafe { (self.f)(self.data, args, locals, ret) }
    }
}

pub fn make_func<'a, F: Fn(*const i64, *mut i64, i64) -> i64 + 'a>(f: F) -> Func<'a> {
    #[inline(always)]
    unsafe fn invoke<F: Fn(*const i64, *mut i64, i64) -> i64>(
        data: *const (),
        args: *const i64,
        locals: *mut i64,
        ret: i64,
    ) -> i64 {
        let f = std::mem::transmute::<_, &F>(data);
        f(args, locals, ret)
    }

    Func {
        f: invoke::<F>,
        data: Box::into_raw(Box::new(f)) as _,
        phantom: PhantomData,
    }
}

impl ClosureContinuations {
    unsafe fn compile<'a>(
        expr: &'a Expr,
        cont: impl MaybeCont<'a> + 'a,
    ) -> <Self as Vm>::Program<'a> {
        // A stand-in for expressions that don't return anything
        const UNIT: i64 = 0;

        match expr {
            Expr::Litr(x) => {
                let x = *x;
                make_func(move |args, locals, _| cont.cont(args, locals, x))
            }
            Expr::Arg(idx) => match idx {
                0 => make_func(move |args, locals, _| {
                    cont.cont(args, locals, unsafe { *args.add(0) })
                }),
                1 => make_func(move |args, locals, _| {
                    cont.cont(args, locals, unsafe { *args.add(1) })
                }),
                _ => {
                    let idx = *idx;
                    make_func(move |args, locals, _| {
                        cont.cont(args, locals, unsafe { *args.add(idx) })
                    })
                }
            },
            Expr::Get(local) => match local {
                0 => make_func(move |args, locals, _| {
                    cont.cont(args, locals, unsafe { *locals.offset(-1) })
                }),
                1 => make_func(move |args, locals, _| {
                    cont.cont(args, locals, unsafe { *locals.offset(-2) })
                }),
                _ => {
                    let offset = -1 - *local as isize;
                    make_func(move |args, locals, _| {
                        cont.cont(args, locals, unsafe { *locals.offset(offset) })
                    })
                }
            },
            Expr::Add(x, y) => match &**y {
                Expr::Litr(1) => Self::compile(
                    x,
                    make_func(move |args, locals, r| cont.cont(args, locals, r + 1)),
                ),
                Expr::Litr(-1) => Self::compile(
                    x,
                    make_func(move |args, locals, r| cont.cont(args, locals, r - 1)),
                ),

                Expr::Litr(y) => {
                    let y = *y;
                    Self::compile(
                        x,
                        make_func(move |args, locals, r| cont.cont(args, locals, r + y)),
                    )
                }
                Expr::Arg(1) => Self::compile(
                    x,
                    make_func(move |args, locals, r| {
                        cont.cont(args, locals, r + unsafe { *args.add(1) })
                    }),
                ),
                _ => {
                    let y = Self::compile(y, ());
                    Self::compile(
                        x,
                        make_func(move |args, locals, r| {
                            let y = y.invoke(args, locals, 0);
                            cont.cont(args, locals, r + y)
                        }),
                    )
                }
            },
            Expr::Let(rhs, then) => {
                let then = Self::compile(
                    then,
                    cont.map(|cont| {
                        make_func(move |args, locals: *mut i64, r| {
                            cont.cont(args, unsafe { locals.offset(-1) }, r)
                        })
                    }),
                );
                Self::compile(
                    rhs,
                    make_func(move |args, locals, r| {
                        unsafe {
                            locals.write(r);
                        }
                        then.invoke(args, unsafe { locals.add(1) }, 0)
                    }),
                )
            }
            Expr::Set(local, rhs) => match local {
                0 => Self::compile(
                    rhs,
                    make_func(move |args, locals, r| {
                        unsafe {
                            locals.offset(-1).write(r);
                        }
                        cont.cont(args, locals, UNIT)
                    }),
                ),
                1 => Self::compile(
                    rhs,
                    make_func(move |args, locals, r| {
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
                        make_func(move |args, locals, r| {
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
                make_func(move |args, locals, _| {
                    while pred.invoke(args, locals, 0) > 0 {
                        body.invoke(args, locals, 0);
                    }
                    cont.cont(args, locals, UNIT)
                })
            }
            Expr::Then(a, b) => {
                let b = Self::compile(b, cont);
                Self::compile(
                    a,
                    make_func(move |args, locals, _b| b.invoke(args, locals, 0)),
                )
            }
        }
    }
}
