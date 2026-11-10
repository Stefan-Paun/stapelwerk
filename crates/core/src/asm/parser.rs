//! Statement parser: labels, instructions, directives. PHASE 1B (internal; may be restructured).

use super::expr::Expr;
use super::lexer::Token;
use crate::diag::Diagnostic;
use crate::isa::Reg;

/// An instruction operand as written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Operand {
    Reg(Reg),
    /// Immediate, jump target or syscall name/number.
    Expr(Expr),
    /// `[reg]`, `[reg ± expr]` or `[expr]` (`base = None`).
    Mem {
        base: Option<Reg>,
        off: Expr,
    },
}

/// What a line contains after its labels.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Stmt {
    Empty,
    Instr {
        mnemonic: String,
        operands: Vec<Operand>,
    },
    Directive {
        name: String,
        args: Vec<Token>,
    },
}

/// One parsed source line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    /// 1-based line number.
    pub line_no: u32,
    /// Labels defined on this line (`global.local` already resolved).
    pub labels: Vec<String>,
    pub stmt: Stmt,
}

/// Parses all lines of one file. PHASE 1B.
pub fn parse_file(_src: &str, _file: u8) -> (Vec<Line>, Vec<Diagnostic>) {
    (Vec::new(), Vec::new())
}
