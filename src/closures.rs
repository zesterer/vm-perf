use super::*;
use core::marker::PhantomData;

// Sadly, rustc currently does a poor job of generating good vtable dispatch code for functions.
// This is the solution: a custom wide pointer that uses a combination of inlining and transmutation to do the right
// thing.
pub struct Func<'a> {
    f: unsafe fn(*const (), *const i64, *mut i64) -> i64,
    data: *const (),
    phantom: PhantomData<&'a ()>,
}

impl<'a> Func<'a> {
    #[inline(always)]
    pub fn invoke(&self, args: *const i64, locals: *mut i64) -> i64 {
        unsafe { (self.f)(self.data, args, locals) }
    }
}

pub fn make_func<'a, F: Fn(*const i64, *mut i64) -> i64 + 'a>(f: F) -> Func<'a> {
    #[inline(always)]
    unsafe fn invoke<F: Fn(*const i64, *mut i64) -> i64>(
        data: *const (),
        args: *const i64,
        locals: *mut i64,
    ) -> i64 {
        let f = std::mem::transmute::<_, &F>(data);
        f(args, locals)
    }

    Func {
        f: invoke::<F>,
        data: Box::into_raw(Box::new(f)) as _,
        phantom: PhantomData,
    }
}

pub struct Closures;

impl Vm for Closures {
    type Program<'a> = Func<'a>;

    fn compile(expr: &Expr) -> Self::Program<'_> {
        // A stand-in for expressions that don't return anything
        const UNIT: i64 = 0;

        match expr {
            Expr::Litr(x) => {
                let x = *x;
                make_func(move |_, _| x)
            }
            Expr::Arg(idx) => match idx {
                0 => make_func(move |args, _| unsafe { *args.add(0) }),
                1 => make_func(move |args, _| unsafe { *args.add(1) }),
                _ => {
                    let idx = *idx;
                    make_func(move |args, _| unsafe { *args.add(idx) })
                }
            },
            Expr::Get(local) => match local {
                0 => make_func(move |_, locals| unsafe { *locals.offset(-1) }),
                1 => make_func(move |_, locals| unsafe { *locals.offset(-2) }),
                _ => {
                    let offset = -1 - *local as isize;
                    make_func(move |_, locals| unsafe { *locals.offset(offset) })
                }
            },
            Expr::Add(x, y) => match &**y {
                Expr::Litr(1) => {
                    let x = Self::compile(x);
                    make_func(move |args, locals| x.invoke(args, locals) + 1)
                }
                Expr::Litr(-1) => {
                    let x = Self::compile(x);
                    make_func(move |args, locals| x.invoke(args, locals) - 1)
                }
                Expr::Litr(y) => {
                    let x = Self::compile(x);
                    let y = *y;
                    make_func(move |args, locals| x.invoke(args, locals) + y)
                }
                Expr::Arg(1) => {
                    let x = Self::compile(x);
                    make_func(move |args, locals| x.invoke(args, locals) + unsafe { *args.add(1) })
                }
                _ => {
                    let x = Self::compile(x);
                    let y = Self::compile(y);
                    make_func(move |args, locals| x.invoke(args, locals) + y.invoke(args, locals))
                }
            },
            Expr::Let(rhs, then) => {
                let rhs = Self::compile(rhs);
                let then = Self::compile(then);
                make_func(move |args, locals| {
                    let rhs = rhs.invoke(args, locals);
                    unsafe {
                        locals.write(rhs);
                    }
                    let res = then.invoke(args, unsafe { locals.offset(1) });
                    res
                })
            }
            Expr::Set(local, rhs) => match local {
                0 => {
                    let rhs = Self::compile(rhs);
                    make_func(move |args, locals| {
                        let rhs = rhs.invoke(args, locals);
                        unsafe {
                            locals.offset(-1).write(rhs);
                        }
                        UNIT
                    })
                }
                1 => {
                    let rhs = Self::compile(rhs);
                    make_func(move |args, locals| {
                        let rhs = rhs.invoke(args, locals);
                        unsafe {
                            locals.offset(-2).write(rhs);
                        }
                        UNIT
                    })
                }
                _ => {
                    let rhs = Self::compile(rhs);
                    let offset = -1 - *local as isize;
                    make_func(move |args, locals| {
                        let rhs = rhs.invoke(args, locals);
                        unsafe {
                            locals.offset(offset).write(rhs);
                        }
                        UNIT
                    })
                }
            },
            Expr::While(pred, body) => {
                let pred = Self::compile(pred);
                let body = Self::compile(body);
                make_func(move |args, locals| {
                    while pred.invoke(args, locals) > 0 {
                        body.invoke(args, locals);
                    }
                    UNIT
                })
            }
            Expr::Then(a, b) => {
                let a = Self::compile(a);
                let b = Self::compile(b);
                make_func(move |args, locals| {
                    a.invoke(args, locals);
                    b.invoke(args, locals)
                })
            }
        }
    }

    unsafe fn execute(prog: &Self::Program<'_>, args: &[i64]) -> i64 {
        let mut v = vec![0; 64];
        prog.invoke(args.as_ptr(), v.as_mut_ptr())
    }
}
