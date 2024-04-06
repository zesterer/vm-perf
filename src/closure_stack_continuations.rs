use super::*;
use core::marker::PhantomData;

pub struct Stack(*mut i64);

impl Stack {
    #[inline(always)]
    unsafe fn push(&mut self, x: i64) {
        self.0.write(x);
        self.0 = self.0.add(1);
    }

    #[inline(always)]
    unsafe fn pop(&mut self) -> i64 {
        self.0 = self.0.sub(1);
        self.0.read()
    }

    unsafe fn set_offset(&mut self, offset: usize, x: i64) {
        self.0.sub(offset).write(x);
    }

    unsafe fn get_offset(&self, offset: usize) -> i64 {
        self.0.sub(offset).read()
    }
}

pub struct ClosureStackContinuations;

impl Vm for ClosureStackContinuations {
    type Program<'a> = Func<'a>;

    fn compile(expr: &Expr) -> Self::Program<'_> {
        unsafe { Self::compile(expr, ()) }
    }

    unsafe fn execute(prog: &Self::Program<'_>, args: &[i64]) -> i64 {
        let mut v = vec![0; 64];
        let stack_raw = Box::into_raw(vec![0i64; 1024].into_boxed_slice());
        let stack = Stack(stack_raw as _);
        prog.invoke(args.as_ptr(), v.as_mut_ptr(), stack);
        (*stack_raw)[0]
    }
}

trait MaybeCont<'a> {
    #[inline(always)]
    fn cont(&self, args: *const i64, locals: *mut i64, stack: Stack) -> Stack {
        stack
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
    fn cont(&self, args: *const i64, locals: *mut i64, stack: Stack) -> Stack {
        self.invoke(args, locals, stack)
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
    f: unsafe fn(*const (), *const i64, *mut i64, Stack) -> Stack,
    data: *const (),
    phantom: PhantomData<&'a ()>,
}

impl<'a> Func<'a> {
    #[inline(always)]
    pub fn invoke(&self, args: *const i64, locals: *mut i64, stack: Stack) -> Stack {
        unsafe { (self.f)(self.data, args, locals, stack) }
    }
}

pub fn make_func<'a, F: Fn(*const i64, *mut i64, Stack) -> Stack + 'a>(f: F) -> Func<'a> {
    #[inline(always)]
    unsafe fn invoke<F: Fn(*const i64, *mut i64, Stack) -> Stack>(
        data: *const (),
        args: *const i64,
        locals: *mut i64,
        stack: Stack,
    ) -> Stack {
        let f = std::mem::transmute::<_, &F>(data);
        f(args, locals, stack)
    }

    Func {
        f: invoke::<F>,
        data: Box::into_raw(Box::new(f)) as _,
        phantom: PhantomData,
    }
}

impl ClosureStackContinuations {
    unsafe fn compile<'a>(
        expr: &'a Expr,
        cont: impl MaybeCont<'a> + 'a,
    ) -> <Self as Vm>::Program<'a> {
        // A stand-in for expressions that don't return anything
        const UNIT: i64 = 0;

        fn returns(expr: &Expr) -> bool {
            match expr {
                Expr::Litr(_) | Expr::Arg(_) | Expr::Get(_) | Expr::Add(_, _) => true,
                Expr::Let(_, expr) => returns(expr),
                Expr::Set(_, _) | Expr::While(_, _) => false,
                Expr::Then(_, b) => returns(b),
            }
        }

        match expr {
            Expr::Litr(x) => {
                let x = *x;
                make_func(move |args, locals, mut stack| {
                    stack.push(x);
                    cont.cont(args, locals, stack)
                })
            }
            Expr::Arg(idx) => match idx {
                0 => make_func(move |args, locals, mut stack| {
                    stack.push(unsafe { *args.add(0) });
                    cont.cont(args, locals, stack)
                }),
                1 => make_func(move |args, locals, mut stack| {
                    stack.push(unsafe { *args.add(1) });
                    cont.cont(args, locals, stack)
                }),
                _ => {
                    let idx = *idx;
                    make_func(move |args, locals, mut stack| {
                        stack.push(unsafe { *args.add(idx) });
                        cont.cont(args, locals, stack)
                    })
                }
            },
            Expr::Get(local) => match local {
                0 => make_func(move |args, locals, mut stack| {
                    stack.push(unsafe { *locals.offset(-1) });
                    cont.cont(args, locals, stack)
                }),
                1 => make_func(move |args, locals, mut stack| {
                    stack.push(unsafe { *locals.offset(-2) });
                    cont.cont(args, locals, stack)
                }),
                _ => {
                    let offset = -1 - *local as isize;
                    make_func(move |args, locals, mut stack| {
                        stack.push(unsafe { *locals.offset(offset) });
                        cont.cont(args, locals, stack)
                    })
                }
            },
            Expr::Add(x, y) => match &**y {
                Expr::Litr(1) => Self::compile(
                    x,
                    make_func(move |args, locals, mut stack| {
                        let x = stack.pop();
                        stack.push(x + 1);
                        cont.cont(args, locals, stack)
                    }),
                ),
                Expr::Litr(-1) => Self::compile(
                    x,
                    make_func(move |args, locals, mut stack| {
                        let x = stack.pop();
                        stack.push(x - 1);
                        cont.cont(args, locals, stack)
                    }),
                ),

                Expr::Litr(y) => {
                    let y = *y;
                    Self::compile(
                        x,
                        make_func(move |args, locals, mut stack| {
                            let x = stack.pop();
                            stack.push(x + y);
                            cont.cont(args, locals, stack)
                        }),
                    )
                }
                Expr::Arg(1) => Self::compile(
                    x,
                    make_func(move |args, locals, mut stack| {
                        let x = stack.pop();
                        stack.push(x + unsafe { *args.add(1) });
                        cont.cont(args, locals, stack)
                    }),
                ),
                _ => Self::compile(
                    x,
                    Self::compile(
                        y,
                        make_func(move |args, locals, mut stack| {
                            let y = stack.pop();
                            let x = stack.pop();
                            stack.push(x + y);
                            cont.cont(args, locals, stack)
                        }),
                    ),
                ),
            },
            Expr::Let(rhs, then) => {
                let then = Self::compile(
                    then,
                    cont.map(|cont| {
                        make_func(move |args, locals: *mut i64, stack| {
                            cont.cont(args, unsafe { locals.offset(-1) }, stack)
                        })
                    }),
                );
                Self::compile(
                    rhs,
                    make_func(move |args, locals, mut stack| {
                        unsafe {
                            locals.write(stack.pop());
                        }
                        then.invoke(args, unsafe { locals.add(1) }, stack)
                    }),
                )
            }
            Expr::Set(local, rhs) => match local {
                0 => Self::compile(
                    rhs,
                    make_func(move |args, locals, mut stack| {
                        unsafe {
                            locals.offset(-1).write(stack.pop());
                        }
                        cont.cont(args, locals, stack)
                    }),
                ),
                1 => Self::compile(
                    rhs,
                    make_func(move |args, locals, mut stack| {
                        unsafe {
                            locals.offset(-2).write(stack.pop());
                        }
                        cont.cont(args, locals, stack)
                    }),
                ),
                _ => {
                    let offset = -1 - *local as isize;
                    Self::compile(
                        rhs,
                        make_func(move |args, locals, mut stack| {
                            unsafe {
                                locals.offset(offset).write(stack.pop());
                            }
                            cont.cont(args, locals, stack)
                        }),
                    )
                }
            },
            Expr::While(pred, body) => {
                let pred = Self::compile(pred, ());
                let body_returns = returns(body);
                let body = Self::compile(body, ());
                make_func(move |args, locals, mut stack| {
                    loop {
                        stack = pred.invoke(args, locals, stack);
                        if stack.pop() <= 0 {
                            break;
                        } else {
                            stack = body.invoke(args, locals, stack);
                            if body_returns {
                                stack.pop();
                            }
                        }
                    }
                    cont.cont(args, locals, stack)
                })
            }
            Expr::Then(a, b) => {
                let b = Self::compile(b, cont);
                let a_returns = returns(a);
                // TODO: Check if a returns, pop from stack if so
                Self::compile(
                    a,
                    make_func(move |args, locals, mut stack| {
                        if a_returns {
                            stack.pop();
                        }
                        b.invoke(args, locals, stack)
                    }),
                )
            }
        }
    }
}
