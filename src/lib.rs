#![feature(generic_associated_types)]

pub mod walker;
pub mod bytecode;
pub mod closures;
pub mod stack_closures;
pub mod tape_closures;
pub mod register_closures;
pub mod bytecode_closures;
pub mod tape_continuations;

pub use crate::{
    walker::Walker,
    bytecode::Bytecode,
    closures::Closures,
    stack_closures::StackClosures,
    tape_closures::TapeClosures,
    register_closures::RegisterClosures,
    bytecode_closures::BytecodeClosures,
    tape_continuations::TapeContinuations,
};

// Relative to the top of the locals stack
type LocalOffset = usize;

pub enum Expr {
    Litr(i64), // i64
    Arg(usize), // i64
    Get(LocalOffset), // i64
    Add(Box<Expr>, Box<Expr>), // i64 -> i64 -> i64
    Let(Box<Expr>, Box<Expr>), // i64 -> i64 -> i64
    Set(LocalOffset, Box<Expr>), // i64 -> ()
    While(Box<Expr>, Box<Expr>), // i64 -> ? -> ()
    Then(Box<Expr>, Box<Expr>), // ? -> ?
}

pub trait Vm {
    type Program<'a>;

    fn compile(expr: &Expr) -> Self::Program<'_>;

    // SAFETY: Program must be well-formed.
    unsafe fn execute(prog: &Self::Program<'_>, args: &[i64]) -> i64;
}
