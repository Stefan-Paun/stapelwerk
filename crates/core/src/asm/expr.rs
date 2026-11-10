//! Constant expressions, evaluated in i64 with overflow checks; C precedence:
//! unary `- ~`, then `* / %`, `+ -`, `<< >>`, `&`, `^`, `|`; parentheses.
//! PHASE 1B (internal; may be restructured).

/// Unary operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnOp {
    Neg,
    Not,
}

/// Binary operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinOp {
    Mul,
    Div,
    Rem,
    Add,
    Sub,
    Shl,
    Shr,
    And,
    Xor,
    Or,
}

/// Expression tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expr {
    Num(i64),
    /// Label or constant (local labels already qualified as `global.local`).
    Sym(String),
    Unary(UnOp, Box<Expr>),
    Binary(BinOp, Box<Expr>, Box<Expr>),
}

/// Why evaluation failed; maps to the diagnostic keys in the comments.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EvalError {
    /// `asm.undefined_symbol {name}`
    Undefined(String),
    /// `asm.expr_div_zero`
    DivZero,
    /// `asm.expr_overflow`
    Overflow,
}

/// Evaluates `expr`, resolving symbols through `lookup`. PHASE 1B.
pub fn eval(_expr: &Expr, _lookup: &dyn Fn(&str) -> Option<i64>) -> Result<i64, EvalError> {
    Err(EvalError::Overflow)
}
