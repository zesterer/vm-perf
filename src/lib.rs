#![feature(let_chains)]

pub mod bytecode;
pub mod bytecode_closures;
pub mod closure_continuations;
pub mod closures;
pub mod register_closures;
pub mod stack_closures;
pub mod tape_closures;
pub mod tape_continuations;
pub mod walker;

pub use crate::{
    bytecode::Bytecode, bytecode_closures::BytecodeClosures,
    closure_continuations::ClosureContinuations, closures::Closures,
    register_closures::RegisterClosures, stack_closures::StackClosures,
    tape_closures::TapeClosures, tape_continuations::TapeContinuations, walker::Walker,
};

// Relative to the top of the locals stack
type LocalOffset = usize;

pub enum Expr {
    Litr(i64),                   // i64
    Arg(usize),                  // i64
    Get(LocalOffset),            // i64
    Add(Box<Expr>, Box<Expr>),   // i64 -> i64 -> i64
    Let(Box<Expr>, Box<Expr>),   // i64 -> i64 -> i64
    Set(LocalOffset, Box<Expr>), // i64 -> ()
    While(Box<Expr>, Box<Expr>), // i64 -> ? -> ()
    Then(Box<Expr>, Box<Expr>),  // ? -> ?
    Throw(Box<Expr>),
    Catch(Box<Expr>),
}

pub trait Vm {
    type Program<'a>;

    fn compile(expr: &Expr) -> Self::Program<'_>;

    // SAFETY: Program must be well-formed.
    unsafe fn execute(prog: &Self::Program<'_>, args: &[i64]) -> i64;
}
