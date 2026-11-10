//! Two-pass layout: addresses, symbol table, encoding, line map.
//!
//! Order of work (see [`assemble_lines`]):
//! 1. library pre-pass: the library's `.equ` constants (visible to the player);
//! 2. pass 1 over the player, then over the library: addresses of every item, labels, the
//!    player's `.equ`, `.space` and `.align`;
//! 3. pass 2 over both files: instruction encoding and data values (labels may be used before
//!    their definition, constants may not).
//!
//! Text layout: player code from 0x0100, one zero word, library code, the trap `halt` word.

use std::collections::{BTreeMap, BTreeSet};

use super::expr::{EvalError, Expr, Resolve, eval};
use super::parser::{Dir, DirArg, LabelDef, Line, Operand, Span, Stmt};
use crate::consts::{
    DATA_START, INPUT_START, TEXT_START, TRAP_LABEL, builtin_constant, syscall_number,
};
use crate::diag::{Diagnostic, FILE_LIBRARY, FILE_PLAYER};
use crate::isa::{
    AluOp, Cond, ImmKind, Instr, Mem, REL20_MAX, REL20_MIN, Reg, Shape, encode, rows_for_mnemonic,
};
use crate::program::{LineEntry, Program, Symbol, SymbolKind};

/// Maximum size of the text segment in bytes (player + zero word + library + trap word).
pub const TEXT_MAX: u64 = (DATA_START - TEXT_START) as u64;
/// Maximum size of the data segment in bytes.
pub const DATA_MAX: u64 = (INPUT_START - DATA_START) as u64;

const NOP_WORD: u32 = 0x0200_0000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Section {
    Text,
    Data,
}

/// Where a symbol may be used.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Use {
    /// Library `.equ` pre-pass: only built-ins and earlier library constants.
    Prepass { line: u32 },
    /// Pass 1 (`.equ`, `.space`, `.align`): labels only if already placed.
    Early { file: u8, line: u32 },
    /// Pass 2: all labels.
    Late { file: u8, line: u32 },
}

#[derive(Clone, Debug)]
struct SymInfo {
    kind: SymbolKind,
    value: i64,
    file: u8,
    line: u32,
}

/// Placement of one line's item, decided in pass 1.
#[derive(Clone, Copy, Debug, Default)]
struct Place {
    /// Absolute address of the item (after alignment).
    addr: u32,
    /// Bytes (data) or bytes (text; padding of `.align` in text).
    size: u32,
    placed: bool,
}

struct FileLayout {
    places: Vec<Place>,
    text_bytes: u64,
    data_bytes: u64,
}

struct Asm {
    diags: Vec<Diagnostic>,
    syms: BTreeMap<String, SymInfo>,
    /// Order in which symbols were defined (for `Program::symbols`).
    order: Vec<String>,
    /// Names defined anywhere (labels and constants of both files).
    all_names: BTreeSet<String>,
    /// First line whose item made a section too large: (file, line, span).
    text_overflow: Option<(u8, u32, Span)>,
    data_overflow: Option<(u8, u32, Span)>,
}

/// What an evaluation failure turns into.
fn eval_diag(err: EvalError, file: u8, line: u32, whole: Span) -> Diagnostic {
    match err {
        EvalError::Undefined { name, col, len } => {
            if looks_like_register(&name) {
                Diagnostic::new("asm.unknown_register", file, line, col, len).arg("name", name)
            } else {
                Diagnostic::new("asm.undefined_symbol", file, line, col, len).arg("name", name)
            }
        }
        EvalError::Later { col, len } => {
            Diagnostic::new("asm.needs_constant", file, line, col, len)
        }
        EvalError::DivZero => {
            Diagnostic::new("asm.expr_div_zero", file, line, whole.col, whole.len)
        }
        EvalError::Overflow => {
            Diagnostic::new("asm.expr_overflow", file, line, whole.col, whole.len)
        }
    }
}

/// `r8`, `R12`, `pc`: names a beginner may take for registers.
fn looks_like_register(name: &str) -> bool {
    let b = name.as_bytes();
    name.eq_ignore_ascii_case("pc")
        || (b.len() >= 2 && (b[0] == b'r' || b[0] == b'R') && b[1..].iter().all(u8::is_ascii_digit))
}

fn span_of(e: &Expr) -> Span {
    Span {
        col: e.col,
        len: e.len,
    }
}

impl Asm {
    fn diag(&mut self, d: Diagnostic) {
        self.diags.push(d);
    }

    fn resolve(&self, name: &str, at: Use) -> Resolve {
        if let Some(v) = builtin_constant(name) {
            return Resolve::Value(v);
        }
        match self.syms.get(name) {
            Some(s) => {
                let visible = match (s.kind, at) {
                    (SymbolKind::Const, Use::Prepass { line }) => {
                        s.file == FILE_LIBRARY && s.line < line
                    }
                    (SymbolKind::Const, Use::Early { file, line } | Use::Late { file, line }) => {
                        s.file != file || s.line < line
                    }
                    (_, Use::Prepass { .. }) => false,
                    _ => true,
                };
                if visible {
                    Resolve::Value(s.value)
                } else {
                    Resolve::Later
                }
            }
            None if self.all_names.contains(name) => Resolve::Later,
            None => Resolve::Missing,
        }
    }

    fn eval(&self, e: &Expr, at: Use, file: u8, line: u32) -> Result<i64, Diagnostic> {
        eval(e, &mut |n| self.resolve(n, at)).map_err(|err| eval_diag(err, file, line, span_of(e)))
    }

    /// Defines a label or constant; duplicates are reported and the first definition is kept.
    fn define(&mut self, name: &str, shown: &str, span: Span, info: SymInfo) -> bool {
        if let Some(first) = self.syms.get(name) {
            let d = Diagnostic::new(
                "asm.duplicate_label",
                info.file,
                info.line,
                span.col,
                span.len,
            )
            .arg("name", shown)
            .arg("first_line", first.line);
            self.diag(d);
            return false;
        }
        self.order.push(name.to_string());
        self.syms.insert(name.to_string(), info);
        true
    }

    fn define_label(&mut self, l: &LabelDef, kind: SymbolKind, addr: u64, file: u8, line: u32) {
        let shown = super::expr::shown_name(&l.name, l.local);
        self.define(
            &l.name,
            &shown,
            l.span,
            SymInfo {
                kind,
                value: addr as i64,
                file,
                line,
            },
        );
    }

    /// Library pre-pass: evaluates the library's `.equ` lines in order.
    fn prepass(&mut self, lines: &mut [Line]) {
        for line in lines.iter_mut() {
            let Line {
                line_no,
                stmt,
                has_error,
                ..
            } = line;
            if let Stmt::Equ { name, nspan, value } = stmt {
                let at = Use::Prepass { line: *line_no };
                let v = match value {
                    Some(e) if !*has_error => match self.eval(e, at, FILE_LIBRARY, *line_no) {
                        Ok(v) => v,
                        Err(d) => {
                            self.diag(d);
                            *has_error = true;
                            0
                        }
                    },
                    _ => 0,
                };
                let info = SymInfo {
                    kind: SymbolKind::Const,
                    value: v,
                    file: FILE_LIBRARY,
                    line: *line_no,
                };
                if !self.define(name, name, *nspan, info) {
                    *has_error = true;
                }
            }
        }
    }

    fn flush(&mut self, pending: &mut Vec<(LabelDef, u32)>, sec: Section, pos: u64, file: u8) {
        let kind = match sec {
            Section::Text => SymbolKind::Code,
            Section::Data => SymbolKind::Data,
        };
        for (l, lno) in pending.drain(..) {
            self.define_label(&l, kind, pos, file, lno);
        }
    }

    fn check_text(&mut self, text_pos: u64, tail: u64, file: u8, line: u32, span: Span) {
        if text_pos - TEXT_START as u64 + tail > TEXT_MAX && self.text_overflow.is_none() {
            self.text_overflow = Some((file, line, span));
        }
    }

    fn check_data(&mut self, data_pos: u64, file: u8, line: u32, span: Span) {
        if data_pos - DATA_START as u64 > DATA_MAX && self.data_overflow.is_none() {
            self.data_overflow = Some((file, line, span));
        }
    }

    /// Reports `asm.operand_count` once per line.
    fn count_error(&mut self, err: &mut bool, d: (u8, u32, Span), expected: &str, got: usize) {
        if !*err {
            let (file, line, span) = d;
            self.diag(
                Diagnostic::new("asm.operand_count", file, line, span.col, span.len)
                    .arg("expected", expected)
                    .arg("got", got as i64),
            );
            *err = true;
        }
    }

    /// Reports a diagnostic once per line.
    fn once(&mut self, err: &mut bool, d: Diagnostic) {
        if !*err {
            self.diag(d);
            *err = true;
        }
    }

    /// Pass 1 over one file.
    fn pass1(
        &mut self,
        lines: &mut [Line],
        file: u8,
        text_base: u32,
        data_base: u32,
    ) -> FileLayout {
        let mut places = vec![Place::default(); lines.len()];
        let mut section = Section::Text;
        let mut text_pos: u64 = text_base as u64;
        let mut data_pos: u64 = data_base as u64;
        // Labels waiting for the next item: (label, line number).
        let mut pending: Vec<(LabelDef, u32)> = Vec::new();
        // Bytes the text segment needs after this file (zero word and/or trap word).
        let tail: u64 = if file == FILE_PLAYER { 8 } else { 4 };

        for (idx, line) in lines.iter_mut().enumerate() {
            let Line {
                line_no,
                labels,
                stmt,
                has_error: err,
            } = line;
            let ln = *line_no;
            for l in labels.drain(..) {
                pending.push((l, ln));
            }
            match stmt {
                Stmt::Empty | Stmt::Bad => {}
                Stmt::Instr { .. } | Stmt::BadInstr { .. } => {
                    let (words, mspan): (u32, Span) = match &*stmt {
                        Stmt::Instr {
                            mnemonic, mspan, ..
                        } => (if mnemonic == "li" { 2 } else { 1 }, *mspan),
                        Stmt::BadInstr { words, mspan } => (*words, *mspan),
                        _ => (0, Span::default()),
                    };
                    if section == Section::Data {
                        let d = Diagnostic::new("asm.code_in_data", file, ln, mspan.col, mspan.len);
                        self.once(err, d);
                        continue;
                    }
                    self.flush(&mut pending, section, text_pos, file);
                    places[idx] = Place {
                        addr: text_pos as u32,
                        size: words * 4,
                        placed: true,
                    };
                    text_pos += words as u64 * 4;
                    self.check_text(text_pos, tail, file, ln, mspan);
                }
                Stmt::Equ { name, nspan, value } => {
                    if file == FILE_LIBRARY {
                        continue; // defined in the pre-pass
                    }
                    let v = match value {
                        Some(e) if !*err => {
                            match self.eval(e, Use::Early { file, line: ln }, file, ln) {
                                Ok(v) => v,
                                Err(d) => {
                                    self.once(err, d);
                                    0
                                }
                            }
                        }
                        _ => 0,
                    };
                    let info = SymInfo {
                        kind: SymbolKind::Const,
                        value: v,
                        file,
                        line: ln,
                    };
                    if !self.define(name, name, *nspan, info) {
                        *err = true;
                    }
                }
                Stmt::Directive {
                    dir,
                    dspan,
                    args,
                    span,
                } => {
                    let (dir, dspan, span) = (*dir, *dspan, *span);
                    match dir {
                        Dir::Text | Dir::Data => {
                            if !args.is_empty() {
                                self.count_error(err, (file, ln, span), "0", args.len());
                            }
                            let pos = if section == Section::Text {
                                text_pos
                            } else {
                                data_pos
                            };
                            self.flush(&mut pending, section, pos, file);
                            section = if dir == Dir::Text {
                                Section::Text
                            } else {
                                Section::Data
                            };
                        }
                        Dir::Align => {
                            let Some(n) = self.single_const(err, dir, (file, ln, span), args)
                            else {
                                continue;
                            };
                            if section == Section::Text {
                                let aligned = text_pos.next_multiple_of(n);
                                places[idx] = Place {
                                    addr: text_pos as u32,
                                    size: (aligned - text_pos) as u32,
                                    placed: true,
                                };
                                text_pos = aligned;
                                self.check_text(text_pos, tail, file, ln, dspan);
                            } else {
                                data_pos = data_pos.next_multiple_of(n);
                                self.check_data(data_pos, file, ln, dspan);
                            }
                        }
                        Dir::Word | Dir::Byte | Dir::String | Dir::Ascii | Dir::Space => {
                            if section == Section::Text {
                                let d = Diagnostic::new(
                                    "asm.data_in_text",
                                    file,
                                    ln,
                                    dspan.col,
                                    dspan.len,
                                );
                                self.once(err, d);
                                continue;
                            }
                            let size = match dir {
                                Dir::Word | Dir::Byte => {
                                    if args.is_empty() {
                                        self.count_error(err, (file, ln, span), "1+", 0);
                                        continue;
                                    }
                                    if dir == Dir::Word {
                                        data_pos = data_pos.next_multiple_of(4);
                                        args.len() as u64 * 4
                                    } else {
                                        args.len() as u64
                                    }
                                }
                                Dir::String | Dir::Ascii => match args.as_slice() {
                                    [DirArg::Str(s, _)] => {
                                        s.len() as u64 + u64::from(dir == Dir::String)
                                    }
                                    [other] => {
                                        let s = other.span();
                                        let d = Diagnostic::new(
                                            "asm.operand_type",
                                            file,
                                            ln,
                                            s.col,
                                            s.len,
                                        )
                                        .arg("expected", "string");
                                        self.once(err, d);
                                        continue;
                                    }
                                    _ => {
                                        self.count_error(err, (file, ln, span), "1", args.len());
                                        continue;
                                    }
                                },
                                _ => {
                                    // .space n
                                    let Some(n) =
                                        self.single_const(err, dir, (file, ln, span), args)
                                    else {
                                        continue;
                                    };
                                    n
                                }
                            };
                            self.flush(&mut pending, section, data_pos, file);
                            places[idx] = Place {
                                addr: data_pos as u32,
                                size: size as u32,
                                placed: true,
                            };
                            data_pos += size;
                            self.check_data(data_pos, file, ln, dspan);
                        }
                    }
                }
            }
        }
        let pos = if section == Section::Text {
            text_pos
        } else {
            data_pos
        };
        self.flush(&mut pending, section, pos, file);
        FileLayout {
            places,
            text_bytes: text_pos - text_base as u64,
            data_bytes: data_pos - data_base as u64,
        }
    }

    /// Argument of `.space` (0…8192) or `.align` (power of two 1…256), evaluated immediately.
    fn single_const(
        &mut self,
        err: &mut bool,
        dir: Dir,
        at: (u8, u32, Span),
        args: &[DirArg],
    ) -> Option<u64> {
        let (file, ln, _) = at;
        if *err {
            return None;
        }
        let e = match args {
            [DirArg::Expr(e)] => e,
            [other] => {
                let s = other.span();
                let d = Diagnostic::new("asm.operand_type", file, ln, s.col, s.len)
                    .arg("expected", "immediate");
                self.once(err, d);
                return None;
            }
            _ => {
                self.count_error(err, at, "1", args.len());
                return None;
            }
        };
        let v = match self.eval(e, Use::Early { file, line: ln }, file, ln) {
            Ok(v) => v,
            Err(d) => {
                self.once(err, d);
                return None;
            }
        };
        let (min, max) = if dir == Dir::Align {
            (1, 256)
        } else {
            (0, DATA_MAX as i64)
        };
        let ok = (min..=max).contains(&v) && (dir != Dir::Align || (v & (v - 1)) == 0);
        if !ok {
            let s = span_of(e);
            let d = Diagnostic::new("asm.imm_range", file, ln, s.col, s.len)
                .arg("value", v)
                .arg("min", min)
                .arg("max", max);
            self.once(err, d);
            return None;
        }
        Some(v as u64)
    }
}

// -------------------------------------------------------------------------------------------------
// Instruction encoding (pass 2)
// -------------------------------------------------------------------------------------------------

/// Operand checker for one instruction line.
struct Enc<'a> {
    asm: &'a Asm,
    file: u8,
    line: u32,
    at: Use,
}

type R<T> = Result<T, Diagnostic>;

impl Enc<'_> {
    fn d(&self, key: &'static str, s: Span) -> Diagnostic {
        Diagnostic::new(key, self.file, self.line, s.col, s.len)
    }

    fn wrong(&self, op: &Operand, expected: &str) -> Diagnostic {
        self.d("asm.operand_type", op.span())
            .arg("expected", expected)
    }

    fn eval(&self, e: &Expr) -> R<i64> {
        self.asm.eval(e, self.at, self.file, self.line)
    }

    fn reg(&self, op: &Operand) -> R<Reg> {
        match op {
            Operand::Reg(r, _) => Ok(*r),
            Operand::Expr(e) => match e.single_sym() {
                Some(_) => {
                    let s = span_of(e);
                    let name = match &e.nodes[0] {
                        super::expr::Node::Sym { name, local, .. } => {
                            super::expr::shown_name(name, *local)
                        }
                        _ => String::new(),
                    };
                    Err(self.d("asm.unknown_register", s).arg("name", name))
                }
                None => Err(self.wrong(op, "register")),
            },
            _ => Err(self.wrong(op, "register")),
        }
    }

    fn range(&self, v: i64, lo: i64, hi: i64, s: Span) -> R<i64> {
        if (lo..=hi).contains(&v) {
            Ok(v)
        } else {
            Err(self
                .d("asm.imm_range", s)
                .arg("value", v)
                .arg("min", lo)
                .arg("max", hi))
        }
    }

    fn imm(&self, op: &Operand, kind: ImmKind) -> R<i64> {
        let Operand::Expr(e) = op else {
            return Err(self.wrong(op, "immediate"));
        };
        let v = self.eval(e)?;
        let (lo, hi) = kind.range().unwrap_or((i64::MIN, i64::MAX));
        self.range(v, lo, hi, span_of(e))
    }

    fn mem(&self, op: &Operand) -> R<Mem> {
        let Operand::Mem { base, off, .. } = op else {
            return Err(self.wrong(op, "memory"));
        };
        let v = match off {
            Some(e) => self.eval(e)?,
            None => 0,
        };
        let s = off.as_ref().map(span_of).unwrap_or(op.span());
        Ok(match base {
            Some(b) => Mem::Based {
                base: *b,
                off: self.range(v, -32768, 32767, s)? as i16,
            },
            None => Mem::Abs {
                addr: self.range(v, 0, 0xFFFF, s)? as u16,
            },
        })
    }

    /// Word offset of a jump/call target.
    fn target(&self, op: &Operand, addr: u32) -> R<i32> {
        let Operand::Expr(e) = op else {
            return Err(self.wrong(op, "label"));
        };
        let t = self.eval(e)?;
        let s = span_of(e);
        let dist = t.checked_sub(addr as i64).unwrap_or(i64::MAX);
        let lo = addr as i64 + 4 * REL20_MIN as i64;
        let hi = addr as i64 + 4 * REL20_MAX as i64;
        if !(lo..=hi).contains(&t) {
            return Err(self
                .d("asm.imm_range", s)
                .arg("value", t)
                .arg("min", lo)
                .arg("max", hi));
        }
        if dist % 4 != 0 {
            return Err(self.d("asm.operand_type", s).arg("expected", "label"));
        }
        Ok((dist / 4) as i32)
    }

    fn count(&self, span: Span, expected: &str, got: usize) -> Diagnostic {
        self.d("asm.operand_count", span)
            .arg("expected", expected)
            .arg("got", got as i64)
    }

    /// Encodes one instruction line into one or two words.
    fn instr(
        &self,
        mnemonic: &str,
        ops: &[Operand],
        addr: u32,
        span: Span,
    ) -> R<(u32, Option<u32>)> {
        let n = ops.len();
        let need = |k: usize, exp: &str| -> R<()> {
            if n == k {
                Ok(())
            } else {
                Err(self.count(span, exp, n))
            }
        };
        let one = |i: Instr| Ok((encode(&i), None));
        if mnemonic == "li" {
            need(2, "2")?;
            let rd = self.reg(&ops[0])?;
            let Operand::Expr(e) = &ops[1] else {
                return Err(self.wrong(&ops[1], "immediate"));
            };
            let v = self.range(self.eval(e)?, -(1i64 << 31), (1i64 << 32) - 1, span_of(e))? as u32;
            let lo = Instr::MovI {
                rd,
                imm: v as u16 as i16,
            };
            let hi = Instr::MovHi {
                rd,
                imm: (v >> 16) as u16,
            };
            return Ok((encode(&lo), Some(encode(&hi))));
        }
        if let Some(cond) = Cond::from_mnemonic(mnemonic) {
            need(1, "1")?;
            if cond == Cond::Always
                && let Operand::Reg(rs, _) = ops[0]
            {
                return one(Instr::JumpReg { rs });
            }
            let off = self.target(&ops[0], addr)?;
            return one(Instr::Jump { cond, off });
        }
        let Some(row) = rows_for_mnemonic(mnemonic).next() else {
            return Err(self.d("asm.unknown_mnemonic", span).arg("name", mnemonic));
        };
        match row.shape {
            Shape::None => {
                need(0, "0")?;
                one(match row.opcode {
                    0x01 => Instr::Halt,
                    0x02 => Instr::Nop,
                    _ => Instr::Ret,
                })
            }
            Shape::Sys => {
                need(1, "1")?;
                if let Operand::Expr(e) = &ops[0]
                    && let Some(num) = e.single_sym().and_then(syscall_number)
                {
                    return one(Instr::Sys { n: num });
                }
                let v = self.imm(&ops[0], ImmKind::Sys)?;
                one(Instr::Sys { n: v as u16 })
            }
            Shape::RdRs if row.opcode == 0x10 => {
                // mov rd, rs | mov rd, simm16
                need(2, "2")?;
                let rd = self.reg(&ops[0])?;
                match &ops[1] {
                    Operand::Reg(rs, _) => one(Instr::Mov { rd, rs: *rs }),
                    Operand::Expr(e) => {
                        let v = self.eval(e)?;
                        match self.range(v, -32768, 32767, span_of(e)) {
                            Ok(v) => one(Instr::MovI { rd, imm: v as i16 }),
                            Err(d) => Err(d.arg("hint", "li")),
                        }
                    }
                    other => Err(self.wrong(other, "register")),
                }
            }
            Shape::RdRs => {
                // neg / not: 1 or 2 operands
                if !(1..=2).contains(&n) {
                    return Err(self.count(span, "1-2", n));
                }
                let rd = self.reg(&ops[0])?;
                let rs = if n == 2 { self.reg(&ops[1])? } else { rd };
                one(if row.opcode == 0x40 {
                    Instr::Neg { rd, rs }
                } else {
                    Instr::Not { rd, rs }
                })
            }
            Shape::RdImm => {
                need(2, "2")?;
                let rd = self.reg(&ops[0])?;
                let v = self.imm(&ops[1], row.imm)?;
                one(Instr::MovHi { rd, imm: v as u16 })
            }
            Shape::RegMem => {
                need(2, "2")?;
                let r = self.reg(&ops[0])?;
                let m = self.mem(&ops[1])?;
                one(match row.opcode {
                    0x13 => Instr::Lea { rd: r, m },
                    0x50 => Instr::Ld { rd: r, m },
                    0x51 => Instr::Ldb { rd: r, m },
                    0x52 => Instr::Ldbs { rd: r, m },
                    0x54 => Instr::St { rs: r, m },
                    _ => Instr::Stb { rs: r, m },
                })
            }
            Shape::RdRsRs | Shape::RsRs => {
                let op = AluOp::from_nibble((row.opcode & 0xF) as u32).unwrap_or(AluOp::Add);
                let (rd, rs1, src) = if op.writes_rd() {
                    if !(2..=3).contains(&n) {
                        return Err(self.count(span, "2-3", n));
                    }
                    let rd = self.reg(&ops[0])?;
                    if n == 3 {
                        (rd, self.reg(&ops[1])?, &ops[2])
                    } else {
                        (rd, rd, &ops[1])
                    }
                } else {
                    need(2, "2")?;
                    (Reg::R0, self.reg(&ops[0])?, &ops[1])
                };
                match src {
                    Operand::Reg(rs2, _) => one(Instr::Alu {
                        op,
                        rd,
                        rs1,
                        rs2: *rs2,
                    }),
                    Operand::Expr(_) => {
                        let imm = self.imm(src, op.imm_kind())? as i32;
                        one(Instr::AluI { op, rd, rs1, imm })
                    }
                    other => Err(self.wrong(other, "register")),
                }
            }
            Shape::Reg => {
                // push / pop
                need(1, "1")?;
                let r = self.reg(&ops[0])?;
                one(if row.opcode == 0x58 {
                    Instr::Push { rs: r }
                } else {
                    Instr::Pop { rd: r }
                })
            }
            Shape::Target => {
                // call L | call rs (jmp is handled through Cond above)
                need(1, "1")?;
                if let Operand::Reg(rs, _) = ops[0] {
                    return one(Instr::CallReg { rs });
                }
                let off = self.target(&ops[0], addr)?;
                one(Instr::Call { off })
            }
            Shape::RdRsImm | Shape::RsImm => {
                // Never the first row of a mnemonic.
                Err(self.d("asm.unknown_mnemonic", span).arg("name", mnemonic))
            }
        }
    }
}

impl Asm {
    /// Pass 2 over one file: writes words and data, extends the line map.
    fn pass2(&mut self, lines: &mut [Line], layout: &FileLayout, file: u8, out: &mut Output) {
        for (idx, line) in lines.iter_mut().enumerate() {
            let place = layout.places[idx];
            if !place.placed {
                continue;
            }
            let Line {
                line_no,
                stmt,
                has_error: err,
                ..
            } = line;
            let ln = *line_no;
            let at = Use::Late { file, line: ln };
            let wi = (place.addr.wrapping_sub(TEXT_START) / 4) as usize;
            let base = place.addr.wrapping_sub(DATA_START) as usize;
            match stmt {
                Stmt::Instr {
                    mnemonic,
                    operands,
                    span,
                    ..
                } => {
                    out.code_lines(place, file, ln);
                    if *err {
                        continue;
                    }
                    let enc = Enc {
                        asm: self,
                        file,
                        line: ln,
                        at,
                    };
                    match enc.instr(mnemonic, operands, place.addr, *span) {
                        Ok((w0, w1)) => {
                            out.put_word(wi, w0);
                            if let Some(w1) = w1 {
                                out.put_word(wi + 1, w1);
                            }
                        }
                        Err(d) => self.once(err, d),
                    }
                }
                Stmt::BadInstr { .. } => out.code_lines(place, file, ln),
                Stmt::Directive { dir, args, .. } => match dir {
                    Dir::Align => {
                        // Padding inside .text: nop words (no size, but a line-map entry).
                        if place.addr < DATA_START {
                            for k in 0..(place.size / 4) as usize {
                                out.put_word(wi + k, NOP_WORD);
                                out.linemap.push(LineEntry {
                                    addr: place.addr + 4 * k as u32,
                                    file,
                                    line: ln,
                                });
                            }
                        }
                    }
                    Dir::String | Dir::Ascii => {
                        if let [DirArg::Str(s, _)] = args.as_slice() {
                            for (k, b) in s.iter().enumerate() {
                                out.put_byte(base + k, *b);
                            }
                        }
                    }
                    Dir::Word | Dir::Byte => {
                        if *err {
                            continue;
                        }
                        let (w, lo, hi) = if *dir == Dir::Word {
                            (4usize, -(1i64 << 31), (1i64 << 32) - 1)
                        } else {
                            (1usize, -128, 255)
                        };
                        for (k, a) in args.iter().enumerate() {
                            let r = match a {
                                DirArg::Expr(e) => self.eval(e, at, file, ln).and_then(|v| {
                                    if (lo..=hi).contains(&v) {
                                        Ok(v)
                                    } else {
                                        let s = span_of(e);
                                        Err(Diagnostic::new(
                                            "asm.imm_range",
                                            file,
                                            ln,
                                            s.col,
                                            s.len,
                                        )
                                        .arg("value", v)
                                        .arg("min", lo)
                                        .arg("max", hi))
                                    }
                                }),
                                other => {
                                    let s = other.span();
                                    Err(Diagnostic::new("asm.operand_type", file, ln, s.col, s.len)
                                        .arg("expected", "immediate"))
                                }
                            };
                            match r {
                                Ok(v) => {
                                    for (j, b) in
                                        (v as u32).to_le_bytes().iter().take(w).enumerate()
                                    {
                                        out.put_byte(base + k * w + j, *b);
                                    }
                                }
                                Err(d) => {
                                    self.once(err, d);
                                    break;
                                }
                            }
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }
}

/// Output buffers of pass 2 (capped at the segment sizes; writes beyond are dropped because
/// the overflow is already reported).
struct Output {
    words: Vec<u32>,
    data: Vec<u8>,
    linemap: Vec<LineEntry>,
    size: u32,
}

impl Output {
    fn put_word(&mut self, i: usize, w: u32) {
        if let Some(slot) = self.words.get_mut(i) {
            *slot = w;
        }
    }

    fn put_byte(&mut self, i: usize, b: u8) {
        if let Some(slot) = self.data.get_mut(i) {
            *slot = b;
        }
    }

    /// Line-map entries (and size) for an instruction line.
    fn code_lines(&mut self, place: Place, file: u8, line: u32) {
        let n = place.size / 4;
        for k in 0..n {
            self.linemap.push(LineEntry {
                addr: place.addr + 4 * k,
                file,
                line,
            });
        }
        if file == FILE_PLAYER {
            self.size += n;
        }
    }
}

/// Collects every name defined in `lines` (labels and constants).
fn collect_names(lines: &[Line], out: &mut BTreeSet<String>) {
    for l in lines {
        for lab in &l.labels {
            out.insert(lab.name.clone());
        }
        if let Stmt::Equ { name, .. } = &l.stmt {
            out.insert(name.clone());
        }
    }
}

/// Lays out player and library lines and encodes them. `diags` holds the parser's diagnostics.
pub fn assemble_lines(
    player: &mut [Line],
    library: &mut [Line],
    mut diags: Vec<Diagnostic>,
) -> Result<Program, Vec<Diagnostic>> {
    let mut all_names = BTreeSet::new();
    collect_names(player, &mut all_names);
    collect_names(library, &mut all_names);
    let mut asm = Asm {
        diags: Vec::new(),
        syms: BTreeMap::new(),
        order: Vec::new(),
        all_names,
        text_overflow: None,
        data_overflow: None,
    };
    asm.diags.append(&mut diags);

    asm.prepass(library);
    let pl = asm.pass1(player, FILE_PLAYER, TEXT_START, DATA_START);
    let lib_text = TEXT_START as u64 + pl.text_bytes + 4;
    let lib_data = (DATA_START as u64 + pl.data_bytes).next_multiple_of(4);
    // Clamp bases so absurdly large (already overflowing) programs cannot wrap addresses.
    let ll = asm.pass1(
        library,
        FILE_LIBRARY,
        lib_text.min(u32::MAX as u64 / 2) as u32,
        lib_data.min(u32::MAX as u64 / 2) as u32,
    );
    let trap = lib_text + ll.text_bytes;
    let text_total = trap + 4 - TEXT_START as u64;
    // The library's data starts at the next multiple of 4 (only counted if it has data).
    let data_end = if ll.data_bytes > 0 {
        lib_data + ll.data_bytes
    } else {
        DATA_START as u64 + pl.data_bytes
    };
    let data_total = data_end - DATA_START as u64;

    if text_total > TEXT_MAX {
        let (f, l, s) = asm
            .text_overflow
            .unwrap_or((FILE_PLAYER, 1, Span { col: 1, len: 0 }));
        asm.diag(
            Diagnostic::new("asm.section_overflow", f, l, s.col, s.len)
                .arg("section", "text")
                .arg("size", text_total as i64)
                .arg("max", TEXT_MAX as i64),
        );
    }
    if data_total > DATA_MAX {
        let (f, l, s) = asm
            .data_overflow
            .unwrap_or((FILE_PLAYER, 1, Span { col: 1, len: 0 }));
        asm.diag(
            Diagnostic::new("asm.section_overflow", f, l, s.col, s.len)
                .arg("section", "data")
                .arg("size", data_total as i64)
                .arg("max", DATA_MAX as i64),
        );
    }

    let mut out = Output {
        words: vec![0u32; (text_total.min(TEXT_MAX) / 4) as usize],
        data: vec![0u8; data_total.min(DATA_MAX) as usize],
        linemap: Vec::new(),
        size: 0,
    };
    asm.pass2(player, &pl, FILE_PLAYER, &mut out);
    asm.pass2(library, &ll, FILE_LIBRARY, &mut out);
    let Output {
        mut words,
        data,
        linemap,
        size,
    } = out;

    if !asm.diags.is_empty() {
        return Err(finish_diags(asm.diags));
    }

    let trap_addr = trap as u32;
    if let Some(slot) = words.last_mut() {
        *slot = encode(&Instr::Halt);
    }
    let mut symbols: Vec<Symbol> = asm
        .order
        .iter()
        .filter_map(|name| {
            asm.syms.get(name).map(|s| Symbol {
                name: name.clone(),
                kind: s.kind,
                value: s.value,
                file: s.file,
                line: s.line,
            })
        })
        .collect();
    symbols.sort_by_key(|s| (s.file, s.line));
    symbols.push(Symbol {
        name: TRAP_LABEL.to_string(),
        kind: SymbolKind::Code,
        value: trap_addr as i64,
        file: FILE_LIBRARY,
        line: 0,
    });
    Ok(Program {
        text_len: words.len() as u32 * 4,
        words,
        data,
        symbols,
        linemap,
        trap_addr,
        size,
    })
}

/// Sorts diagnostics by position and caps them at [`crate::consts::MAX_ASM_ERRORS`].
pub fn finish_diags(mut diags: Vec<Diagnostic>) -> Vec<Diagnostic> {
    diags.sort_by_key(|d| (d.file, d.line, d.col));
    let max = crate::consts::MAX_ASM_ERRORS;
    if diags.len() > max {
        let next = &diags[max];
        let stop = Diagnostic::new("asm.too_many_errors", next.file, next.line, next.col, 0);
        diags.truncate(max);
        diags.push(stop);
    }
    diags
}
