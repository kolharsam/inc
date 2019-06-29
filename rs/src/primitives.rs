//! Scheme primitives implemented directly in the compiler
//!
//! Several scheme functions like `(add ...` are implemented by the compiler in
//! assembly rather than in scheme. All of them live in this module.

use crate::{
    compiler::emit,
    compiler::state::State,
    core::*,
    immediate,
    x86::{Ins::*, Operand::*, Register::*, *},
};

// Unary Primitives

/// Increment number by 1
pub fn inc(s: &mut State, x: &AST) -> ASM {
    emit::eval(s, x) + Add { r: RAX, v: Const(immediate::n(1)) }
}

/// Decrement by 1
pub fn dec(s: &mut State, x: &AST) -> ASM {
    emit::eval(s, x) + Sub { r: RAX, v: Const(immediate::n(1)) }
}

/// Is the expression a fixnum?
///
/// # Examples
///
/// ```scheme
/// (fixnum? 42) => #t
/// (fixnum? "hello") => #f
/// ```
pub fn fixnump(s: &mut State, expr: &AST) -> ASM {
    emit::eval(s, expr)
        + emit::mask()
        + compare(Reg(RAX), Const(immediate::NUM), "sete")
}

/// Is the expression a boolean?
pub fn booleanp(s: &mut State, expr: &AST) -> ASM {
    emit::eval(s, expr)
        + emit::mask()
        + compare(Reg(RAX), Const(immediate::BOOL), "sete")
}

/// Is the expression a char?
pub fn charp(s: &mut State, expr: &AST) -> ASM {
    emit::eval(s, expr)
        + emit::mask()
        + compare(Reg(RAX), Const(immediate::CHAR), "sete")
}

/// Is the expression null?
pub fn nullp(s: &mut State, expr: &AST) -> ASM {
    emit::eval(s, expr) + compare(Reg(RAX), Const(immediate::NIL), "sete")
}

/// Is the expression zero?
pub fn zerop(s: &mut State, expr: &AST) -> ASM {
    emit::eval(s, expr) + compare(Reg(RAX), Const(immediate::NUM), "sete")
}

/// Logical not
pub fn not(s: &mut State, expr: &AST) -> ASM {
    emit::eval(s, expr) + compare(Reg(RAX), Const(immediate::FALSE), "sete")
}

// Binary Primitives

/// Evaluate arguments and store the first argument in stack and second in `RAX`
fn binop(s: &mut State, x: &AST, y: &AST) -> ASM {
    emit::eval(s, x) + Save { r: RAX, si: s.si } + emit::eval(s, y)
}

/// Add `x` and `y` and move result to register RAX
pub fn plus(s: &mut State, x: &AST, y: &AST) -> ASM {
    binop(s, &x, &y) + Add { r: RAX, v: Stack(s.si) }
}

/// Subtract `x` from `y` and move result to register RAX
//
// `sub` Subtracts the 2nd op from the first and stores the result in the
// 1st. This is pretty inefficient to update result in stack and load it
// back. Reverse the order and fix it up.
pub fn minus(s: &mut State, x: &AST, y: &AST) -> ASM {
    binop(s, &x, &y)
        + Sub { r: RAX, v: Stack(s.si) }
        + Load { r: RAX, si: s.si }
}

/// Multiply `x` and `y` and move result to register RAX
// The destination operand is of `mul` is an implied operand located in
// register AX. GCC throws `Error: ambiguous operand size for `mul'` without
// size quantifier
pub fn mul(s: &mut State, x: &AST, y: &AST) -> ASM {
    binop(s, &x, &y)
        + Sar { r: RAX, v: immediate::SHIFT }
        + Mul { v: Stack(s.si) }
}

/// Divide `x` by `y` and move result to register RAX
// Division turned out to be much more trickier than I expected it to be.
// Unlike @namin's code, I'm using a shift arithmetic right (SAR) instead of
// shift logical right (SHR) and I don't know how the original examples
// worked at all for negative numbers. I also had to use the CQO instruction
// to Sign-Extend RAX which the 32 bit version is obviously not concerned
// with. I got the idea from GCC disassembly.
//
// Dividend is passed in RDX:RAX and IDIV instruction takes the divisor as the
// argument. the quotient is stored in RAX and the remainder in RDX.
fn div(s: &mut State, x: &AST, y: &AST) -> ASM {
    let mut ctx = String::new();

    ctx.push_str(&(emit::eval(s, y).to_string()));
    ctx.push_str(&Ins::Sar { r: RAX, v: immediate::SHIFT }.to_string());
    ctx.push_str("    mov rcx, rax \n");
    ctx.push_str(&emit::eval(s, x).to_string());
    ctx.push_str(&Ins::Sar { r: RAX, v: immediate::SHIFT }.to_string());
    ctx.push_str("    mov rdx, 0 \n");
    ctx.push_str("    cqo \n");
    ctx.push_str("    idiv rcx \n");
    ctx.into()
}

pub fn quotient(s: &mut State, x: &AST, y: &AST) -> ASM {
    div(s, x, y) + Sal { r: RAX, v: immediate::SHIFT }
}

pub fn remainder(s: &mut State, x: &AST, y: &AST) -> ASM {
    div(s, x, y)
        + Mov { to: Reg(RAX), from: Reg(RDX) }
        + Sal { r: RAX, v: immediate::SHIFT }
}

/// Compares the first operand with the second with `SETcc`
// See `Ins::Cmp` to see how the compare instruction works.
//
// `SETcc` sets the destination operand to 0 or 1 depending on the settings of
// the status flags (CF, SF, OF, ZF, and PF) in the EFLAGS register.
//
// `MOVZX` copies the contents of the source operand (register or memory
// location) to the destination operand (register) and zero extends the value.
fn compare(a: Operand, b: Operand, setcc: &str) -> ASM {
    Cmp { a, b }
        + Slice(format!("    {} al\n", setcc))
        + Slice("    movzx rax, al\n".to_string())
        + Slice(format!("    sal al, {}\n", immediate::SHIFT))
        + Slice(format!("    or al, {}\n", immediate::BOOL))
}

/// Logical eq
pub fn eq(s: &mut State, x: &AST, y: &AST) -> ASM {
    binop(s, x, y) + compare(Stack(s.si), Reg(RAX), "sete")
}

/// Logical <
pub fn lt(s: &mut State, x: &AST, y: &AST) -> ASM {
    binop(s, x, y) + compare(Stack(s.si), Reg(RAX), "setl")
}

/// Logical >
pub fn gt(s: &mut State, x: &AST, y: &AST) -> ASM {
    binop(s, x, y) + compare(Stack(s.si), Reg(RAX), "setg")
}

/// Logical <=
pub fn lte(s: &mut State, x: &AST, y: &AST) -> ASM {
    binop(s, x, y) + compare(Stack(s.si), Reg(RAX), "setle")
}

/// Logical >=
pub fn gte(s: &mut State, x: &AST, y: &AST) -> ASM {
    binop(s, x, y) + compare(Stack(s.si), Reg(RAX), "setge")
}
