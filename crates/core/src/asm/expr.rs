//! Constant expressions, evaluated in i64 with overflow checks; C precedence:
//! unary `- ~`, then `* / %`, `+ -`, `<< >>`, `&`, `^`, `|`; parentheses.
//!
//! An expression is stored in postfix order ([`Node`]s), so evaluation and dropping never recurse,
//! however long the expression is. Parsing recurses only for parentheses and unary operators and is
//! limited to [`MAX_DEPTH`] levels.

use super::lexer::{Token, TokenKind};
use crate::diag::Diagnostic;
use crate::isa::Reg;

/// Maximum nesting of parentheses and unary operators (deeper → `asm.expr_overflow`).
pub const MAX_DEPTH: u32 = 64;

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

impl BinOp {
    /// Operator and binding strength (higher binds tighter) of a punctuation token.
    fn from_punct(p: &str) -> Option<(BinOp, u8)> {
        Some(match p {
            "|" => (BinOp::Or, 1),
            "^" => (BinOp::Xor, 2),
            "&" => (BinOp::And, 3),
            "<<" => (BinOp::Shl, 4),
            ">>" => (BinOp::Shr, 4),
            "+" => (BinOp::Add, 5),
            "-" => (BinOp::Sub, 5),
            "*" => (BinOp::Mul, 6),
            "/" => (BinOp::Div, 6),
            "%" => (BinOp::Rem, 6),
            _ => return None,
        })
    }
}

/// One postfix node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Node {
    Num(i64),
    /// Label or constant; local labels are already qualified as `global.local`.
    Sym {
        name: String,
        /// Written as `.local` (for messages, the name is shown as written).
        local: bool,
        col: u32,
        len: u32,
    },
    Unary(UnOp),
    Binary(BinOp),
}

/// A parsed expression with its source span.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Expr {
    pub nodes: Vec<Node>,
    pub col: u32,
    pub len: u32,
}

impl Expr {
    /// The symbol name if the expression is a single symbol.
    pub fn single_sym(&self) -> Option<&str> {
        match self.nodes.as_slice() {
            [Node::Sym { name, .. }] => Some(name),
            _ => None,
        }
    }
}

/// Name as the user wrote it (`.local` for qualified local labels).
pub fn shown_name(name: &str, local: bool) -> String {
    if local {
        match name.find('.') {
            Some(i) => name[i..].to_string(),
            None => name.to_string(),
        }
    } else {
        name.to_string()
    }
}

/// Result of looking up a symbol.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resolve {
    Value(i64),
    /// Defined, but not usable at this point (`asm.needs_constant`).
    Later,
    /// Not defined anywhere (`asm.undefined_symbol`).
    Missing,
}

/// Why evaluation failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EvalError {
    /// `asm.undefined_symbol {name}` at the symbol.
    Undefined { name: String, col: u32, len: u32 },
    /// `asm.needs_constant` at the symbol.
    Later { col: u32, len: u32 },
    /// `asm.expr_div_zero`
    DivZero,
    /// `asm.expr_overflow`
    Overflow,
}

/// Evaluates `expr`, resolving symbols through `lookup`.
pub fn eval(expr: &Expr, lookup: &mut dyn FnMut(&str) -> Resolve) -> Result<i64, EvalError> {
    let mut stack: Vec<i64> = Vec::with_capacity(4);
    for node in &expr.nodes {
        let v = match node {
            Node::Num(n) => *n,
            Node::Sym {
                name,
                local,
                col,
                len,
            } => match lookup(name) {
                Resolve::Value(v) => v,
                Resolve::Later => {
                    return Err(EvalError::Later {
                        col: *col,
                        len: *len,
                    });
                }
                Resolve::Missing => {
                    return Err(EvalError::Undefined {
                        name: shown_name(name, *local),
                        col: *col,
                        len: *len,
                    });
                }
            },
            Node::Unary(op) => {
                let a = stack.pop().ok_or(EvalError::Overflow)?;
                match op {
                    UnOp::Neg => a.checked_neg().ok_or(EvalError::Overflow)?,
                    UnOp::Not => !a,
                }
            }
            Node::Binary(op) => {
                let b = stack.pop().ok_or(EvalError::Overflow)?;
                let a = stack.pop().ok_or(EvalError::Overflow)?;
                binary(*op, a, b)?
            }
        };
        stack.push(v);
    }
    match stack.as_slice() {
        [v] => Ok(*v),
        _ => Err(EvalError::Overflow),
    }
}

fn binary(op: BinOp, a: i64, b: i64) -> Result<i64, EvalError> {
    let r = match op {
        BinOp::Add => a.checked_add(b),
        BinOp::Sub => a.checked_sub(b),
        BinOp::Mul => a.checked_mul(b),
        BinOp::Div | BinOp::Rem if b == 0 => return Err(EvalError::DivZero),
        BinOp::Div => a.checked_div(b),
        BinOp::Rem if b == -1 => Some(0),
        BinOp::Rem => a.checked_rem(b),
        BinOp::Shl => {
            if !(0..64).contains(&b) {
                None
            } else {
                let r = a << b;
                (r >> b == a).then_some(r)
            }
        }
        BinOp::Shr => (0..64).contains(&b).then(|| a >> b),
        BinOp::And => Some(a & b),
        BinOp::Xor => Some(a ^ b),
        BinOp::Or => Some(a | b),
    };
    r.ok_or(EvalError::Overflow)
}

/// Parser over a token slice.
pub struct ExprParser<'a> {
    pub toks: &'a [Token],
    pub pos: usize,
    /// Current global label for `.local` references.
    pub scope: Option<&'a str>,
    pub file: u8,
    pub line: u32,
    /// Source line (for `asm.unexpected_char {ch}`).
    pub text: &'a str,
    /// Column just after the operand (for "missing operand" errors).
    pub end_col: u32,
    nodes: Vec<Node>,
}

impl<'a> ExprParser<'a> {
    pub fn new(
        toks: &'a [Token],
        scope: Option<&'a str>,
        file: u8,
        line: u32,
        text: &'a str,
        end_col: u32,
    ) -> Self {
        ExprParser {
            toks,
            pos: 0,
            scope,
            file,
            line,
            text,
            end_col,
            nodes: Vec::new(),
        }
    }

    fn diag(&self, key: &'static str, col: u32, len: u32) -> Diagnostic {
        Diagnostic::new(key, self.file, self.line, col, len)
    }

    /// Parses one expression starting at `self.pos` and stops at the first token that cannot
    /// continue it.
    pub fn parse(&mut self) -> Result<Expr, Diagnostic> {
        let col = self
            .toks
            .get(self.pos)
            .map(|t| t.col)
            .unwrap_or(self.end_col);
        self.nodes.clear();
        self.binary(1, 0)?;
        let end = if self.pos > 0 {
            self.toks[self.pos - 1].end()
        } else {
            col
        };
        Ok(Expr {
            nodes: std::mem::take(&mut self.nodes),
            col,
            len: end.saturating_sub(col),
        })
    }

    fn binary(&mut self, min_prec: u8, depth: u32) -> Result<(), Diagnostic> {
        self.unary(depth)?;
        loop {
            let Some((op, prec)) = self.toks.get(self.pos).and_then(|t| match t.kind {
                TokenKind::Punct(p) => BinOp::from_punct(p),
                _ => None,
            }) else {
                return Ok(());
            };
            if prec < min_prec {
                return Ok(());
            }
            self.pos += 1;
            self.binary(prec + 1, depth)?;
            self.nodes.push(Node::Binary(op));
        }
    }

    fn unary(&mut self, depth: u32) -> Result<(), Diagnostic> {
        let Some(t) = self.toks.get(self.pos) else {
            return Err(self
                .diag("asm.operand_type", self.end_col, 0)
                .arg("expected", "immediate"));
        };
        if depth >= MAX_DEPTH {
            return Err(self.diag("asm.expr_overflow", t.col, t.len));
        }
        match &t.kind {
            TokenKind::Number(n) => {
                self.nodes.push(Node::Num(*n));
                self.pos += 1;
            }
            TokenKind::Ident(name) => {
                if Reg::from_name(name).is_some() {
                    return Err(self
                        .diag("asm.operand_type", t.col, t.len)
                        .arg("expected", "immediate"));
                }
                let (name, local) = match name.strip_prefix('.') {
                    Some(rest) => match self.scope {
                        Some(g) => (format!("{g}.{rest}"), true),
                        None => {
                            return Err(self.diag("asm.local_label_without_global", t.col, t.len));
                        }
                    },
                    None => (name.clone(), false),
                };
                self.nodes.push(Node::Sym {
                    name,
                    local,
                    col: t.col,
                    len: t.len,
                });
                self.pos += 1;
            }
            TokenKind::Punct("-") | TokenKind::Punct("~") => {
                let op = if t.is("-") { UnOp::Neg } else { UnOp::Not };
                self.pos += 1;
                self.unary(depth + 1)?;
                self.nodes.push(Node::Unary(op));
            }
            TokenKind::Punct("(") => {
                self.pos += 1;
                self.binary(1, depth + 1)?;
                match self.toks.get(self.pos) {
                    Some(c) if c.is(")") => self.pos += 1,
                    Some(c) => {
                        return Err(self
                            .diag("asm.unexpected_char", c.col, c.len)
                            .arg("ch", c.first_char(self.text)));
                    }
                    None => {
                        return Err(self
                            .diag("asm.unexpected_char", self.end_col, 0)
                            .arg("ch", ")"));
                    }
                }
            }
            TokenKind::Str(_) => {
                return Err(self
                    .diag("asm.operand_type", t.col, t.len)
                    .arg("expected", "immediate"));
            }
            TokenKind::Punct(_) => {
                return Err(self
                    .diag("asm.unexpected_char", t.col, t.len)
                    .arg("ch", t.first_char(self.text)));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::lexer::tokenize_line;
    use super::*;

    fn ev(s: &str) -> Result<i64, EvalError> {
        let toks = tokenize_line(s, 0, 1).unwrap();
        let mut p = ExprParser::new(&toks, Some("g"), 0, 1, s, 99);
        let e = p.parse().unwrap();
        assert_eq!(p.pos, toks.len(), "{s}");
        eval(&e, &mut |n| match n {
            "x" => Resolve::Value(10),
            "g.l" => Resolve::Value(7),
            "later" => Resolve::Later,
            _ => Resolve::Missing,
        })
    }

    #[test]
    fn precedence_and_values() {
        assert_eq!(ev("1 + 2 * 3"), Ok(7));
        assert_eq!(ev("(1 + 2) * 3"), Ok(9));
        // 1 | (6 ^ (3 & 5)) = 1 | 7
        assert_eq!(ev("1 | 6 ^ 3 & 5"), Ok(7));
        assert_eq!(ev("1 << 2 + 1"), Ok(8));
        assert_eq!(ev("-7 / 2"), Ok(-3));
        assert_eq!(ev("-7 % 2"), Ok(-1));
        assert_eq!(ev("~0"), Ok(-1));
        assert_eq!(ev("- -5"), Ok(5));
        assert_eq!(ev("10 - 2 - 3"), Ok(5));
        assert_eq!(ev("x * 2 + .l"), Ok(27));
        assert_eq!(ev("-8 >> 1"), Ok(-4));
    }

    #[test]
    fn errors() {
        assert_eq!(ev("1 / 0"), Err(EvalError::DivZero));
        assert_eq!(ev("5 % (2 - 2)"), Err(EvalError::DivZero));
        assert_eq!(ev("0x7FFFFFFFFFFFFFFF + 1"), Err(EvalError::Overflow));
        assert_eq!(ev("1 << 63"), Err(EvalError::Overflow));
        assert_eq!(ev("1 << 64"), Err(EvalError::Overflow));
        assert_eq!(
            ev("(-0x7FFFFFFFFFFFFFFF - 1) / -1"),
            Err(EvalError::Overflow)
        );
        assert_eq!(ev("(-0x7FFFFFFFFFFFFFFF - 1) % -1"), Ok(0));
        assert!(matches!(ev("nope"), Err(EvalError::Undefined { .. })));
        assert!(matches!(ev("later + 1"), Err(EvalError::Later { .. })));
    }

    #[test]
    fn deep_nesting_is_limited() {
        let s = format!("{}1{}", "(".repeat(200), ")".repeat(200));
        let toks = tokenize_line(&s, 0, 1).unwrap();
        let mut p = ExprParser::new(&toks, None, 0, 1, &s, 1);
        assert_eq!(p.parse().unwrap_err().key, "asm.expr_overflow");
        // Long flat chains are fine (no recursion in evaluation).
        let s = vec!["1"; 5000].join(" + ");
        assert_eq!(ev(&s), Ok(5000));
    }
}
