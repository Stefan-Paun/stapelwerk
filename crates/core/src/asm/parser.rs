//! Statement parser: labels, instructions, directives.
//!
//! Every line is parsed independently (the only state carried between lines is the current
//! global label, which scopes `.local` labels). A line with an error still yields a [`Line`], so
//! the layout can reserve the words an instruction would have used and later lines keep their
//! addresses.

use super::MnemonicFilter;
use super::expr::{Expr, ExprParser};
use super::lexer::{Token, TokenKind, tokenize_line};
use crate::consts::{DIRECTIVES, PSEUDO_MNEMONICS, SUGGESTION_DISTANCE, is_reserved_name};
use crate::diag::{Diagnostic, FILE_PLAYER};
use crate::isa::{Cond, OPCODES, Reg, is_mnemonic};

/// Source span (1-based UTF-16 column and length).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Span {
    pub col: u32,
    pub len: u32,
}

impl Span {
    fn of(t: &Token) -> Span {
        Span {
            col: t.col,
            len: t.len,
        }
    }

    fn between(a: &Token, b: &Token) -> Span {
        Span {
            col: a.col,
            len: b.end().saturating_sub(a.col),
        }
    }
}

/// An instruction operand as written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Operand {
    Reg(Reg, Span),
    /// Immediate, jump target or syscall name/number.
    Expr(Expr),
    /// `[reg]`, `[reg ± expr]` (`base = Some`) or `[expr]` (`base = None`); `off = None` means 0.
    Mem {
        base: Option<Reg>,
        off: Option<Expr>,
        span: Span,
    },
    /// Nothing between two commas (or after the last one).
    Empty(Span),
}

impl Operand {
    pub fn span(&self) -> Span {
        match self {
            Operand::Reg(_, s) | Operand::Mem { span: s, .. } | Operand::Empty(s) => *s,
            Operand::Expr(e) => Span {
                col: e.col,
                len: e.len,
            },
        }
    }
}

/// Argument of a data directive.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DirArg {
    Str(Vec<u8>, Span),
    Expr(Expr),
    Empty(Span),
}

impl DirArg {
    pub fn span(&self) -> Span {
        match self {
            DirArg::Str(_, s) | DirArg::Empty(s) => *s,
            DirArg::Expr(e) => Span {
                col: e.col,
                len: e.len,
            },
        }
    }
}

/// Directives (without `.equ`, which is [`Stmt::Equ`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    Text,
    Data,
    Word,
    Byte,
    String,
    Ascii,
    Space,
    Align,
}

/// What a line contains after its labels.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Stmt {
    Empty,
    /// An instruction or `li`; `mnemonic` is lower-case.
    Instr {
        mnemonic: String,
        mspan: Span,
        operands: Vec<Operand>,
        /// From the mnemonic to the end of the line.
        span: Span,
    },
    /// A statement that has already been reported but would occupy `words` instruction words in
    /// `.text` (unknown mnemonic, forbidden instruction, operand syntax error).
    BadInstr {
        words: u32,
        mspan: Span,
    },
    Directive {
        dir: Dir,
        dspan: Span,
        args: Vec<DirArg>,
        span: Span,
    },
    /// `.equ NAME, expr`; `value = None` if the expression had an error (already reported).
    Equ {
        name: String,
        nspan: Span,
        value: Option<Expr>,
    },
    /// A line whose statement could not be parsed (already reported); it emits nothing.
    Bad,
}

/// A label definition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LabelDef {
    /// Full name (`global.local` for local labels).
    pub name: String,
    /// `true` for `.local` definitions.
    pub local: bool,
    pub span: Span,
}

/// One parsed source line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    /// 1-based line number.
    pub line_no: u32,
    /// Labels defined on this line (`global.local` already resolved).
    pub labels: Vec<LabelDef>,
    pub stmt: Stmt,
    /// `true` once a diagnostic was reported for this line (later passes stay quiet).
    pub has_error: bool,
}

/// Damerau-Levenshtein distance in the optimal-string-alignment variant (ASCII case ignored):
/// insertion, deletion, substitution and a swap of two neighbouring characters each cost 1.
/// Returns `limit + 1` early when the lengths alone already differ by more than `limit`.
pub fn edit_distance(a: &str, b: &str, limit: usize) -> usize {
    let a: Vec<u8> = a.bytes().map(|c| c.to_ascii_lowercase()).collect();
    let b: Vec<u8> = b.bytes().map(|c| c.to_ascii_lowercase()).collect();
    if a.len().abs_diff(b.len()) > limit {
        return limit + 1;
    }
    // Three rows: i-2, i-1 and i (the transposition looks two rows back).
    let mut prev2: Vec<usize> = vec![0; b.len() + 1];
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0; b.len() + 1];
    for i in 1..=a.len() {
        cur[0] = i;
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            let mut d = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                d = d.min(prev2[j - 2] + 1);
            }
            cur[j] = d;
        }
        std::mem::swap(&mut prev2, &mut prev);
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

/// Closest known mnemonic, pseudo-instruction, condition alias or directive within
/// [`SUGGESTION_DISTANCE`] (and strictly closer than the length of `name`), measured with
/// [`edit_distance`]. Among equally close candidates one with the same first letter wins, then one
/// with the same length; remaining ties go to the first candidate in table order.
pub fn suggest_mnemonic(name: &str) -> Option<&'static str> {
    let first = name.bytes().next().map(|c| c.to_ascii_lowercase());
    let rank = |cand: &str, d: usize| {
        let other_first = cand.bytes().next().map(|c| c.to_ascii_lowercase()) != first;
        let other_len = cand.len() != name.len();
        (d, other_first, other_len)
    };
    let mut best: Option<((usize, bool, bool), &'static str)> = None;
    let mut consider = |cand: &'static str| {
        let d = edit_distance(name, cand, SUGGESTION_DISTANCE);
        if d <= SUGGESTION_DISTANCE && d < name.len() {
            let r = rank(cand, d);
            if best.is_none_or(|(br, _)| r < br) {
                best = Some((r, cand));
            }
        }
    };
    if name.starts_with('.') {
        DIRECTIVES.iter().for_each(|d| consider(d));
    } else {
        OPCODES.iter().for_each(|r| consider(r.mnemonic));
        PSEUDO_MNEMONICS.iter().for_each(|m| consider(m));
        for c in Cond::ALL {
            consider(c.mnemonic());
            c.aliases().iter().for_each(|a| consider(a));
        }
    }
    best.map(|(_, c)| c)
}

/// Lower-case directive.
fn directive(name: &str) -> Option<Dir> {
    Some(match name.to_ascii_lowercase().as_str() {
        ".text" => Dir::Text,
        ".data" => Dir::Data,
        ".word" => Dir::Word,
        ".byte" => Dir::Byte,
        ".string" => Dir::String,
        ".ascii" => Dir::Ascii,
        ".space" => Dir::Space,
        ".align" => Dir::Align,
        _ => return None,
    })
}

/// Splits `toks` at top-level commas (commas inside `[]` or `()` do not split).
fn split_commas(toks: &[Token]) -> Vec<&[Token]> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut start = 0;
    for (i, t) in toks.iter().enumerate() {
        match t.kind {
            TokenKind::Punct("[") | TokenKind::Punct("(") => depth += 1,
            TokenKind::Punct("]") | TokenKind::Punct(")") => depth -= 1,
            TokenKind::Punct(",") if depth <= 0 => {
                out.push(&toks[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push(&toks[start..]);
    out
}

struct LineParser<'a> {
    file: u8,
    line: u32,
    text: &'a str,
    scope: Option<&'a str>,
}

impl LineParser<'_> {
    fn diag(&self, key: &'static str, span: Span) -> Diagnostic {
        Diagnostic::new(key, self.file, self.line, span.col, span.len)
    }

    /// Parses a whole operand slice as one expression.
    fn expr(&self, toks: &[Token], end_col: u32) -> Result<Expr, Diagnostic> {
        let mut p = ExprParser::new(toks, self.scope, self.file, self.line, self.text, end_col);
        let e = p.parse()?;
        if let Some(t) = toks.get(p.pos) {
            return Err(self
                .diag("asm.unexpected_char", Span::of(t))
                .arg("ch", t.first_char(self.text)));
        }
        Ok(e)
    }

    /// One instruction operand. `end_col` is the column right after the slice (for empty ones).
    fn operand(&self, toks: &[Token], end_col: u32) -> Result<Operand, Diagnostic> {
        let (Some(first), Some(last)) = (toks.first(), toks.last()) else {
            return Ok(Operand::Empty(Span {
                col: end_col,
                len: 0,
            }));
        };
        if let Some(r) = first.ident().and_then(Reg::from_name) {
            match toks.get(1) {
                None => return Ok(Operand::Reg(r, Span::of(first))),
                // `r1 r2`, `r1 5`: probably a missing comma.
                Some(t) if !matches!(t.kind, TokenKind::Punct(_)) => {
                    return Err(self
                        .diag("asm.unexpected_char", Span::of(t))
                        .arg("ch", t.first_char(self.text)));
                }
                // `r2 + 1`: a register inside an expression (reported by the expression parser).
                Some(_) => {}
            }
        }
        if !first.is("[") {
            return Ok(Operand::Expr(self.expr(toks, end_col)?));
        }
        let span = Span::between(first, last);
        if !last.is("]") || toks.len() < 2 {
            return Err(self
                .diag("asm.operand_type", span)
                .arg("expected", "memory"));
        }
        let inner = &toks[1..toks.len() - 1];
        let close = last.col;
        let base = inner
            .first()
            .and_then(|t| t.ident())
            .and_then(Reg::from_name);
        match base {
            None => {
                if inner.is_empty() {
                    return Err(self
                        .diag("asm.operand_type", span)
                        .arg("expected", "memory"));
                }
                Ok(Operand::Mem {
                    base: None,
                    off: Some(self.expr(inner, close)?),
                    span,
                })
            }
            Some(r) => {
                let off = match inner.get(1) {
                    None => None,
                    // `[r + e]`: the expression after `+`.
                    Some(t) if t.is("+") => Some(self.expr(&inner[2..], close)?),
                    // `[r - e]`: the expression including the `-`, so it binds like C.
                    Some(t) if t.is("-") => Some(self.expr(&inner[1..], close)?),
                    Some(t) => {
                        return Err(self
                            .diag("asm.unexpected_char", Span::of(t))
                            .arg("ch", t.first_char(self.text)));
                    }
                };
                Ok(Operand::Mem {
                    base: Some(r),
                    off,
                    span,
                })
            }
        }
    }

    fn dir_arg(&self, toks: &[Token], end_col: u32) -> Result<DirArg, Diagnostic> {
        match toks {
            [] => Ok(DirArg::Empty(Span {
                col: end_col,
                len: 0,
            })),
            [t, rest @ ..] if matches!(t.kind, TokenKind::Str(_)) => {
                if let Some(x) = rest.first() {
                    return Err(self
                        .diag("asm.unexpected_char", Span::of(x))
                        .arg("ch", x.first_char(self.text)));
                }
                match &t.kind {
                    TokenKind::Str(s) => Ok(DirArg::Str(s.clone(), Span::of(t))),
                    _ => Ok(DirArg::Empty(Span::of(t))),
                }
            }
            _ => Ok(DirArg::Expr(self.expr(toks, end_col)?)),
        }
    }
}

/// Splits operand tokens and returns each slice with the column right after it.
fn slices_with_end(toks: &[Token], line_end: u32) -> Vec<(&[Token], u32)> {
    let parts = split_commas(toks);
    let n = parts.len();
    let mut out = Vec::with_capacity(n);
    let mut consumed = 0usize;
    for (i, part) in parts.into_iter().enumerate() {
        consumed += part.len();
        // The comma following this slice (if any) marks its end.
        let end = if i + 1 < n {
            toks[consumed].col
        } else {
            line_end
        };
        consumed += 1;
        out.push((part, end));
    }
    out
}

/// Parses all lines of one file. `filter` is applied to instruction mnemonics (player only).
pub fn parse_file(src: &str, file: u8, filter: &MnemonicFilter) -> (Vec<Line>, Vec<Diagnostic>) {
    let mut lines = Vec::new();
    let mut diags = Vec::new();
    let mut scope: Option<String> = None;
    for (i, text) in src.lines().enumerate() {
        let line_no = (i + 1) as u32;
        let (line, d) = parse_line(text, file, line_no, &mut scope, filter);
        if let Some(d) = d {
            diags.extend(d);
        }
        lines.push(line);
    }
    (lines, diags)
}

fn parse_line(
    text: &str,
    file: u8,
    line_no: u32,
    scope: &mut Option<String>,
    filter: &MnemonicFilter,
) -> (Line, Option<Vec<Diagnostic>>) {
    let mut line = Line {
        line_no,
        labels: Vec::new(),
        stmt: Stmt::Empty,
        has_error: false,
    };
    let toks = match tokenize_line(text, file, line_no) {
        Ok(t) => t,
        Err(d) => {
            line.stmt = Stmt::Bad;
            line.has_error = true;
            return (line, Some(vec![d]));
        }
    };
    let mut diags = Vec::new();
    let mut i = 0;
    // Labels.
    while i + 1 < toks.len() && toks[i + 1].is(":") {
        let t = &toks[i];
        i += 2;
        let Some(name) = t.ident() else {
            diags.push(
                Diagnostic::new("asm.unexpected_char", file, line_no, t.col, t.len)
                    .arg("ch", t.first_char(text)),
            );
            continue;
        };
        if let Some(rest) = name.strip_prefix('.') {
            match scope.as_deref() {
                Some(g) => line.labels.push(LabelDef {
                    name: format!("{g}.{rest}"),
                    local: true,
                    span: Span::of(t),
                }),
                None => diags.push(Diagnostic::new(
                    "asm.local_label_without_global",
                    file,
                    line_no,
                    t.col,
                    t.len,
                )),
            }
        } else if let Some(dot) = name.find('.') {
            let col = t.col + dot as u32;
            diags
                .push(Diagnostic::new("asm.unexpected_char", file, line_no, col, 1).arg("ch", "."));
        } else {
            // A global label opens a new scope even if it is rejected, to avoid follow-up errors.
            *scope = Some(name.to_string());
            if is_reserved_name(name) {
                diags.push(
                    Diagnostic::new("asm.reserved_name", file, line_no, t.col, t.len)
                        .arg("name", name),
                );
            } else {
                line.labels.push(LabelDef {
                    name: name.to_string(),
                    local: false,
                    span: Span::of(t),
                });
            }
        }
    }
    let rest = &toks[i..];
    let p = LineParser {
        file,
        line: line_no,
        text,
        scope: scope.as_deref(),
    };
    let (stmt, d) = statement(&p, rest, filter);
    line.stmt = stmt;
    diags.extend(d);
    line.has_error = !diags.is_empty();
    (line, (!diags.is_empty()).then_some(diags))
}

/// The statement and the first error of the line, if any.
type StmtResult = (Stmt, Option<Diagnostic>);

fn statement(p: &LineParser<'_>, toks: &[Token], filter: &MnemonicFilter) -> StmtResult {
    let Some(head) = toks.first() else {
        return (Stmt::Empty, None);
    };
    let line_end = toks.last().map(|t| t.end()).unwrap_or(head.end());
    let span = Span::between(head, toks.last().unwrap_or(head));
    let Some(name) = head.ident() else {
        let d = p
            .diag("asm.unexpected_char", Span::of(head))
            .arg("ch", head.first_char(p.text));
        return (Stmt::Bad, Some(d));
    };
    let mspan = Span::of(head);
    if name.starts_with('.') {
        if name.eq_ignore_ascii_case(".equ") {
            return equ(p, &toks[1..], line_end, span);
        }
        let Some(dir) = directive(name) else {
            let mut d = p.diag("asm.unknown_mnemonic", mspan).arg("name", name);
            if let Some(s) = suggest_mnemonic(name) {
                d = d.arg("suggestion", s);
            }
            return (Stmt::Bad, Some(d));
        };
        let mut args = Vec::new();
        if toks.len() > 1 {
            for (slice, end) in slices_with_end(&toks[1..], line_end) {
                match p.dir_arg(slice, end) {
                    Ok(a) => args.push(a),
                    Err(d) => return (Stmt::Bad, Some(d)),
                }
            }
        }
        let stmt = Stmt::Directive {
            dir,
            dspan: mspan,
            args,
            span,
        };
        return (stmt, None);
    }
    let lower = name.to_ascii_lowercase();
    let words = if lower == "li" { 2 } else { 1 };
    let bad = Stmt::BadInstr { words, mspan };
    if lower != "li" && !is_mnemonic(&lower) {
        let mut d = p.diag("asm.unknown_mnemonic", mspan).arg("name", name);
        if let Some(s) = suggest_mnemonic(name) {
            d = d.arg("suggestion", s);
        }
        return (bad, Some(d));
    }
    if p.file == FILE_PLAYER && !filter.permits(&lower) {
        let d = p
            .diag("asm.forbidden_instruction", mspan)
            .arg("mnemonic", super::canonical_mnemonic(&lower));
        return (bad, Some(d));
    }
    let mut operands = Vec::new();
    if toks.len() > 1 {
        for (slice, end) in slices_with_end(&toks[1..], line_end) {
            match p.operand(slice, end) {
                Ok(o) => operands.push(o),
                Err(d) => return (bad, Some(d)),
            }
        }
    }
    let stmt = Stmt::Instr {
        mnemonic: lower,
        mspan,
        operands,
        span,
    };
    (stmt, None)
}

fn equ(p: &LineParser<'_>, toks: &[Token], line_end: u32, span: Span) -> StmtResult {
    let parts = slices_with_end(toks, line_end);
    let got = if toks.is_empty() { 0 } else { parts.len() };
    if got != 2 {
        let d = p
            .diag("asm.operand_count", span)
            .arg("expected", "2")
            .arg("got", got as i64);
        return (Stmt::Bad, Some(d));
    }
    let (name_toks, _) = parts[0];
    let name = match name_toks {
        [t] => match t.ident() {
            Some(n) if !n.contains('.') => (n.to_string(), Span::of(t)),
            _ => {
                let d = p
                    .diag("asm.operand_type", Span::of(t))
                    .arg("expected", "label");
                return (Stmt::Bad, Some(d));
            }
        },
        _ => {
            let s = name_toks
                .first()
                .zip(name_toks.last())
                .map(|(a, b)| Span::between(a, b))
                .unwrap_or(Span {
                    col: parts[1].1,
                    len: 0,
                });
            let d = p.diag("asm.operand_type", s).arg("expected", "label");
            return (Stmt::Bad, Some(d));
        }
    };
    if is_reserved_name(&name.0) {
        let d = p
            .diag("asm.reserved_name", name.1)
            .arg("name", name.0.as_str());
        return (Stmt::Bad, Some(d));
    }
    let (vt, vend) = parts[1];
    let (value, d) = match p.expr(vt, vend) {
        Ok(e) => (Some(e), None),
        Err(d) => (None, Some(d)),
    };
    let stmt = Stmt::Equ {
        name: name.0,
        nspan: name.1,
        value,
    };
    (stmt, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suggestions() {
        assert_eq!(suggest_mnemonic("ad"), Some("add"));
        assert_eq!(suggest_mnemonic("MVO"), Some("mov"));
        assert_eq!(suggest_mnemonic("jeq"), Some("je"));
        assert_eq!(suggest_mnemonic("lii"), Some("li"));
        assert_eq!(suggest_mnemonic(".wrod"), Some(".word"));
        assert_eq!(suggest_mnemonic("xyzzy"), None);
        assert_eq!(edit_distance("kitten", "sitting", 5), 3);
    }

    #[test]
    fn suggestions_with_swapped_letters() {
        assert_eq!(suggest_mnemonic("jpm"), Some("jmp"));
        assert_eq!(suggest_mnemonic("mvo"), Some("mov"));
        assert_eq!(suggest_mnemonic("sbu"), Some("sub"));
        assert_eq!(suggest_mnemonic("pussh"), Some("push"));
        assert_eq!(suggest_mnemonic("hlat"), Some("halt"));
        assert_eq!(edit_distance("ab", "ba", 2), 1);
        assert_eq!(edit_distance("HLAT", "halt", 2), 1);
        // Optimal string alignment: a swapped pair is not edited again.
        assert_eq!(edit_distance("ca", "abc", 5), 3);
        assert_eq!(edit_distance("abcdef", "a", 2), 3);
    }

    #[test]
    fn labels_and_operands() {
        let (lines, d) = parse_file(
            "main: .x: ld r1, [fp - 8] ; c\n  st r2, [0x4000]\n",
            0,
            &MnemonicFilter::All,
        );
        assert!(d.is_empty(), "{d:?}");
        assert_eq!(lines[0].labels.len(), 2);
        assert_eq!(lines[0].labels[1].name, "main.x");
        match &lines[0].stmt {
            Stmt::Instr { operands, .. } => {
                assert!(matches!(
                    operands[1],
                    Operand::Mem {
                        base: Some(Reg::Fp),
                        ..
                    }
                ));
            }
            s => panic!("{s:?}"),
        }
    }
}
