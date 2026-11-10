//! Instruction set architecture: registers, flags, conditions, the opcode table and the
//! binary encoding.
//!
//! [`OPCODES`] is the **single source of truth** for mnemonics, opcodes, formats, immediate kinds,
//! cycle costs and flag effects. The assembler, the disassembler, the CPU and `stapelwerk isa-table`
//! all read from it. Normative prose: `docs/ISA.md`.
//!
//! Encoding (fixed 32 bit, nibble aligned, stored little-endian):
//!
//! | Format | 31–24 | 23–20 | 19–16 | 15–12 | 11–0 |
//! |---|---|---|---|---|---|
//! | N | op | 0 | 0 | 0 | 0 |
//! | R | op | A | B | C | 0 |
//! | I | op | A | B | imm16 (bits 15–0) ||
//! | J | op | A (condition) | imm20 (bits 19–0, signed, in words relative to this instruction) |||
//!
//! Decoding is strict: every valid word has exactly one meaning and one canonical text form, so
//! `encode(decode(w)?) == w` holds for all 2³² words.

use core::fmt;

/// Flag bit Z (zero) in the flag word.
pub const FLAG_Z: u32 = 1 << 0;
/// Flag bit N (negative, bit 31 of the result).
pub const FLAG_N: u32 = 1 << 1;
/// Flag bit C (carry; borrow for subtraction, x86 convention).
pub const FLAG_C: u32 = 1 << 2;
/// Flag bit V (signed overflow).
pub const FLAG_V: u32 = 1 << 3;
/// All defined flag bits.
pub const FLAG_MASK: u32 = FLAG_Z | FLAG_N | FLAG_C | FLAG_V;

/// Register field value 15 in field B of memory instructions and `lea`: base 0, unsigned offset.
pub const ABS_BASE_CODE: u32 = 15;

/// Smallest signed 20-bit jump offset (in words).
pub const REL20_MIN: i32 = -(1 << 19);
/// Largest signed 20-bit jump offset (in words).
pub const REL20_MAX: i32 = (1 << 19) - 1;

// ---------------------------------------------------------------------------------------------
// Registers
// ---------------------------------------------------------------------------------------------

/// One of the ten architectural registers. The program counter is not a register operand.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Reg {
    R0 = 0,
    R1 = 1,
    R2 = 2,
    R3 = 3,
    R4 = 4,
    R5 = 5,
    R6 = 6,
    R7 = 7,
    /// Frame pointer (code 8), callee-saved.
    Fp = 8,
    /// Stack pointer (code 9).
    Sp = 9,
}

impl Reg {
    /// Number of registers.
    pub const COUNT: usize = 10;
    /// All registers in code order.
    pub const ALL: [Reg; 10] = [
        Reg::R0,
        Reg::R1,
        Reg::R2,
        Reg::R3,
        Reg::R4,
        Reg::R5,
        Reg::R6,
        Reg::R7,
        Reg::Fp,
        Reg::Sp,
    ];

    /// Register for a 4-bit field value; codes 10–15 are not registers.
    pub const fn from_code(code: u32) -> Option<Reg> {
        if code < 10 {
            Some(Reg::ALL[code as usize])
        } else {
            None
        }
    }

    /// 4-bit field value.
    pub const fn code(self) -> u32 {
        self as u32
    }

    /// Index into a `[u32; 10]` register file.
    pub const fn index(self) -> usize {
        self as usize
    }

    /// Canonical lower-case name (`r0`…`r7`, `fp`, `sp`).
    pub const fn name(self) -> &'static str {
        match self {
            Reg::R0 => "r0",
            Reg::R1 => "r1",
            Reg::R2 => "r2",
            Reg::R3 => "r3",
            Reg::R4 => "r4",
            Reg::R5 => "r5",
            Reg::R6 => "r6",
            Reg::R7 => "r7",
            Reg::Fp => "fp",
            Reg::Sp => "sp",
        }
    }

    /// Parses a register name, ignoring ASCII case.
    pub fn from_name(name: &str) -> Option<Reg> {
        Reg::ALL
            .iter()
            .copied()
            .find(|r| r.name().eq_ignore_ascii_case(name))
    }

    /// Callee-saved registers (r4–r7, fp) must hold the same value after `ret`.
    pub const fn is_callee_saved(self) -> bool {
        matches!(self, Reg::R4 | Reg::R5 | Reg::R6 | Reg::R7 | Reg::Fp)
    }
}

impl fmt::Display for Reg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

// ---------------------------------------------------------------------------------------------
// Conditions
// ---------------------------------------------------------------------------------------------

/// Jump condition (field A of opcode 0x60). Code 15 is invalid.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Cond {
    Always = 0,
    Z = 1,
    Nz = 2,
    Lt = 3,
    Ge = 4,
    Le = 5,
    Gt = 6,
    B = 7,
    Ae = 8,
    Be = 9,
    A = 10,
    S = 11,
    Ns = 12,
    O = 13,
    No = 14,
}

impl Cond {
    /// All conditions in code order.
    pub const ALL: [Cond; 15] = [
        Cond::Always,
        Cond::Z,
        Cond::Nz,
        Cond::Lt,
        Cond::Ge,
        Cond::Le,
        Cond::Gt,
        Cond::B,
        Cond::Ae,
        Cond::Be,
        Cond::A,
        Cond::S,
        Cond::Ns,
        Cond::O,
        Cond::No,
    ];

    /// Condition for a 4-bit field value (15 is invalid).
    pub const fn from_code(code: u32) -> Option<Cond> {
        if code < 15 {
            Some(Cond::ALL[code as usize])
        } else {
            None
        }
    }

    /// 4-bit field value.
    pub const fn code(self) -> u32 {
        self as u32
    }

    /// Canonical mnemonic (the disassembler always prints this one).
    pub const fn mnemonic(self) -> &'static str {
        match self {
            Cond::Always => "jmp",
            Cond::Z => "jz",
            Cond::Nz => "jnz",
            Cond::Lt => "jlt",
            Cond::Ge => "jge",
            Cond::Le => "jle",
            Cond::Gt => "jgt",
            Cond::B => "jb",
            Cond::Ae => "jae",
            Cond::Be => "jbe",
            Cond::A => "ja",
            Cond::S => "js",
            Cond::Ns => "jns",
            Cond::O => "jo",
            Cond::No => "jno",
        }
    }

    /// Accepted alternative spellings.
    pub const fn aliases(self) -> &'static [&'static str] {
        match self {
            Cond::Z => &["je"],
            Cond::Nz => &["jne"],
            _ => &[],
        }
    }

    /// Looks up a jump mnemonic or alias (ASCII case ignored). `jmp` yields [`Cond::Always`].
    pub fn from_mnemonic(name: &str) -> Option<Cond> {
        Cond::ALL.iter().copied().find(|c| {
            c.mnemonic().eq_ignore_ascii_case(name)
                || c.aliases().iter().any(|a| a.eq_ignore_ascii_case(name))
        })
    }

    /// The flag predicate as text, e.g. `N ≠ V`.
    pub const fn predicate(self) -> &'static str {
        match self {
            Cond::Always => "always",
            Cond::Z => "Z",
            Cond::Nz => "!Z",
            Cond::Lt => "N ≠ V",
            Cond::Ge => "N = V",
            Cond::Le => "Z or N ≠ V",
            Cond::Gt => "!Z and N = V",
            Cond::B => "C",
            Cond::Ae => "!C",
            Cond::Be => "C or Z",
            Cond::A => "!C and !Z",
            Cond::S => "N",
            Cond::Ns => "!N",
            Cond::O => "V",
            Cond::No => "!V",
        }
    }

    /// Meaning after `cmp a, b`.
    pub const fn meaning(self) -> &'static str {
        match self {
            Cond::Always => "always jumps",
            Cond::Z => "a = b",
            Cond::Nz => "a ≠ b",
            Cond::Lt => "a < b (signed)",
            Cond::Ge => "a ≥ b (signed)",
            Cond::Le => "a ≤ b (signed)",
            Cond::Gt => "a > b (signed)",
            Cond::B => "a < b (unsigned)",
            Cond::Ae => "a ≥ b (unsigned)",
            Cond::Be => "a ≤ b (unsigned)",
            Cond::A => "a > b (unsigned)",
            Cond::S => "result negative",
            Cond::Ns => "result not negative",
            Cond::O => "signed overflow",
            Cond::No => "no signed overflow",
        }
    }

    /// Evaluates the condition on a flag word (bits [`FLAG_Z`], [`FLAG_N`], [`FLAG_C`], [`FLAG_V`]).
    pub const fn holds(self, flags: u32) -> bool {
        let z = flags & FLAG_Z != 0;
        let n = flags & FLAG_N != 0;
        let c = flags & FLAG_C != 0;
        let v = flags & FLAG_V != 0;
        match self {
            Cond::Always => true,
            Cond::Z => z,
            Cond::Nz => !z,
            Cond::Lt => n != v,
            Cond::Ge => n == v,
            Cond::Le => z || n != v,
            Cond::Gt => !z && n == v,
            Cond::B => c,
            Cond::Ae => !c,
            Cond::Be => c || z,
            Cond::A => !c && !z,
            Cond::S => n,
            Cond::Ns => !n,
            Cond::O => v,
            Cond::No => !v,
        }
    }
}

// ---------------------------------------------------------------------------------------------
// ALU operations
// ---------------------------------------------------------------------------------------------

/// ALU operation, identified by the low nibble of opcodes 0x20–0x2E (register form) and
/// 0x30–0x3E (immediate form).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AluOp {
    Add = 0,
    Sub = 1,
    Mul = 2,
    Div = 3,
    Divu = 4,
    Rem = 5,
    Remu = 6,
    And = 7,
    Or = 8,
    Xor = 9,
    Shl = 10,
    Shr = 11,
    Sar = 12,
    /// Flags of `a - b`, no result register (field A = 0).
    Cmp = 13,
    /// Flags of `a & b`, no result register (field A = 0).
    Tst = 14,
}

impl AluOp {
    /// All ALU operations in nibble order.
    pub const ALL: [AluOp; 15] = [
        AluOp::Add,
        AluOp::Sub,
        AluOp::Mul,
        AluOp::Div,
        AluOp::Divu,
        AluOp::Rem,
        AluOp::Remu,
        AluOp::And,
        AluOp::Or,
        AluOp::Xor,
        AluOp::Shl,
        AluOp::Shr,
        AluOp::Sar,
        AluOp::Cmp,
        AluOp::Tst,
    ];

    /// Operation for a low nibble (15 is invalid).
    pub const fn from_nibble(n: u32) -> Option<AluOp> {
        if n < 15 {
            Some(AluOp::ALL[n as usize])
        } else {
            None
        }
    }

    /// Low nibble of the opcode.
    pub const fn nibble(self) -> u32 {
        self as u32
    }

    /// `false` for `cmp` and `tst`, which only set flags.
    pub const fn writes_rd(self) -> bool {
        !matches!(self, AluOp::Cmp | AluOp::Tst)
    }

    /// Table row of the register form (0x20 + nibble).
    pub fn info_reg(self) -> &'static OpInfo {
        &OPCODES_ARR[OPCODE_INDEX[0x20 + self as usize] as usize]
    }

    /// Table row of the immediate form (0x30 + nibble).
    pub fn info_imm(self) -> &'static OpInfo {
        &OPCODES_ARR[OPCODE_INDEX[0x30 + self as usize] as usize]
    }

    /// Mnemonic (from the table).
    pub fn mnemonic(self) -> &'static str {
        self.info_reg().mnemonic
    }

    /// Immediate kind of the immediate form (from the table).
    pub fn imm_kind(self) -> ImmKind {
        self.info_imm().imm
    }

    /// Cycle cost (from the table; equal for both forms).
    pub fn cycles(self) -> u32 {
        self.info_reg().cycles
    }

    /// Flag effect (from the table).
    pub fn flag_effect(self) -> FlagEffect {
        self.info_reg().flags
    }
}

// ---------------------------------------------------------------------------------------------
// Opcode table
// ---------------------------------------------------------------------------------------------

/// Instruction word format.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    /// Only the opcode; all other bits 0.
    N,
    /// Register fields A, B, C; bits 11–0 are 0.
    R,
    /// Register fields A, B and a 16-bit immediate.
    I,
    /// Condition field A and a signed 20-bit word offset.
    J,
}

impl Format {
    /// Single-letter name.
    pub const fn name(self) -> &'static str {
        match self {
            Format::N => "N",
            Format::R => "R",
            Format::I => "I",
            Format::J => "J",
        }
    }
}

/// Assembly operand shape of a table row (what the assembler parses, what the disassembler prints).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    /// No operands (`halt`, `nop`, `ret`).
    None,
    /// `sys NAME|n` (A = B = 0, imm16 = n).
    Sys,
    /// `op rd, rs` → A = rd, B = rs (`mov`, `neg`, `not`).
    RdRs,
    /// `op rd, imm` → A = rd, B = 0 (`mov` immediate, `movhi`).
    RdImm,
    /// `op reg, [m]` → A = reg, B/imm16 = memory operand (`lea`, `ld*`, `st*`).
    RegMem,
    /// `op rd, rs1, rs2` → A, B, C (ALU register form).
    RdRsRs,
    /// `op rs1, rs2` → A = 0, B, C (`cmp`, `tst` register form).
    RsRs,
    /// `op rd, rs1, imm` → A, B, imm16 (ALU immediate form).
    RdRsImm,
    /// `op rs1, imm` → A = 0, B, imm16 (`cmp`, `tst` immediate form).
    RsImm,
    /// `op r` → A only (`push`, `pop`, `jmp rs`, `call rs`).
    Reg,
    /// `op target` → J format (`jmp`/`j<cc>`, `call`).
    Target,
}

/// How a 16- or 20-bit immediate field is interpreted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImmKind {
    /// No immediate.
    None,
    /// Signed 16 bit, −32768…32767, sign-extended.
    S16,
    /// Unsigned 16 bit, 0…65535, zero-extended.
    U16,
    /// Shift width 0…31 (values 32…65535 are invalid encodings).
    Shift,
    /// System call number 0…65535.
    Sys,
    /// Memory operand: signed offset −32768…32767 with a base register, or absolute 0…0xFFFF with B = 15.
    Mem,
    /// Signed 20-bit offset in words relative to the address of the instruction.
    Rel20,
}

impl ImmKind {
    /// Short name used in the generated table.
    pub const fn name(self) -> &'static str {
        match self {
            ImmKind::None => "–",
            ImmKind::S16 => "s16",
            ImmKind::U16 => "u16",
            ImmKind::Shift => "sh",
            ImmKind::Sys => "sys",
            ImmKind::Mem => "mem",
            ImmKind::Rel20 => "rel20",
        }
    }

    /// Inclusive value range of the operand as written in assembly, if the kind has a fixed range.
    /// `Mem` depends on the base and `Rel20` on the address, so they return `None`.
    pub const fn range(self) -> Option<(i64, i64)> {
        match self {
            ImmKind::S16 => Some((-32768, 32767)),
            ImmKind::U16 | ImmKind::Sys => Some((0, 65535)),
            ImmKind::Shift => Some((0, 31)),
            ImmKind::None | ImmKind::Mem | ImmKind::Rel20 => None,
        }
    }

    /// Meaning, for the generated table.
    pub const fn description(self) -> &'static str {
        match self {
            ImmKind::None => "no immediate",
            ImmKind::S16 => "signed, −32768…32767, sign-extended",
            ImmKind::U16 => "unsigned, 0…65535, zero-extended",
            ImmKind::Shift => "shift width 0…31",
            ImmKind::Sys => "system call number 0…65535",
            ImmKind::Mem => {
                "memory operand: base register + signed offset −32768…32767, or absolute 0…0xFFFF (B = 15)"
            }
            ImmKind::Rel20 => "signed 20-bit offset in words, relative to this instruction",
        }
    }
}

/// Which flags an instruction writes and how.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlagEffect {
    /// Flags unchanged.
    None,
    /// Z, N from result; C = carry out of bit 31; V = signed overflow.
    Add,
    /// Z, N from result; C = borrow (a <ᵤ b); V = signed overflow. Also `cmp` and `neg` (0 − x).
    Sub,
    /// Z, N from result; C = product does not fit u32; V = product does not fit i32.
    Mul,
    /// Z, N from result; C = 0; V = 1 exactly for INT_MIN / −1 (`div`, `rem`).
    Div,
    /// Z, N from result; C = 0; V = 0 (`divu`, `remu`, `and`, `or`, `xor`, `tst`, `not`).
    Logic,
    /// Z, N from result; C = last bit shifted out (0 for width 0); V = 0.
    Shift,
}

impl FlagEffect {
    /// All effects, for the generated table.
    pub const ALL: [FlagEffect; 7] = [
        FlagEffect::None,
        FlagEffect::Add,
        FlagEffect::Sub,
        FlagEffect::Mul,
        FlagEffect::Div,
        FlagEffect::Logic,
        FlagEffect::Shift,
    ];

    /// Short name used in the instruction table.
    pub const fn name(self) -> &'static str {
        match self {
            FlagEffect::None => "–",
            FlagEffect::Add => "add",
            FlagEffect::Sub => "sub",
            FlagEffect::Mul => "mul",
            FlagEffect::Div => "div",
            FlagEffect::Logic => "logic",
            FlagEffect::Shift => "shift",
        }
    }

    /// (Z and N, C, V) descriptions.
    pub const fn describe(self) -> (&'static str, &'static str, &'static str) {
        match self {
            FlagEffect::None => ("unchanged", "unchanged", "unchanged"),
            FlagEffect::Add => ("from result", "carry out of bit 31", "signed overflow"),
            FlagEffect::Sub => (
                "from result",
                "borrow: a < b unsigned (x86 convention)",
                "signed overflow",
            ),
            FlagEffect::Mul => (
                "from result",
                "product does not fit in u32",
                "product does not fit in i32",
            ),
            FlagEffect::Div => ("from result", "0", "1 exactly for INT_MIN / −1"),
            FlagEffect::Logic => ("from result", "0", "0"),
            FlagEffect::Shift => ("from result", "last bit shifted out (0 for width 0)", "0"),
        }
    }
}

/// One row of the opcode table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OpInfo {
    /// Canonical mnemonic. Opcode 0x60 is listed as `jmp`; its conditional forms are named by [`Cond`].
    pub mnemonic: &'static str,
    /// Bits 31–24.
    pub opcode: u8,
    /// Word format.
    pub format: Format,
    /// Assembly operand shape.
    pub shape: Shape,
    /// Immediate interpretation.
    pub imm: ImmKind,
    /// Cycle cost.
    pub cycles: u32,
    /// Flag effect.
    pub flags: FlagEffect,
    /// Assembly syntax as shown in the documentation.
    pub syntax: &'static str,
    /// One-line description (English).
    pub summary: &'static str,
}

macro_rules! row {
    ($op:expr, $m:expr, $fmt:ident, $shape:ident, $imm:ident, $cyc:expr, $fl:ident, $syn:expr, $sum:expr) => {
        OpInfo {
            mnemonic: $m,
            opcode: $op,
            format: Format::$fmt,
            shape: Shape::$shape,
            imm: ImmKind::$imm,
            cycles: $cyc,
            flags: FlagEffect::$fl,
            syntax: $syn,
            summary: $sum,
        }
    };
}

#[rustfmt::skip]
const OPCODES_ARR: [OpInfo; 51] = [
    row!(0x01, "halt", N, None, None, 1, None, "halt", "Stop the program (status HALTED)."),
    row!(0x02, "nop", N, None, None, 1, None, "nop", "Do nothing."),
    row!(0x04, "sys", I, Sys, Sys, 4, None, "sys NAME|n", "System call n; changes only r0."),
    row!(0x08, "ret", N, None, None, 2, None, "ret", "pc = pop() (target checked)."),
    row!(0x10, "mov", R, RdRs, None, 1, None, "mov rd, rs", "rd = rs"),
    row!(0x11, "mov", I, RdImm, S16, 1, None, "mov rd, simm16", "rd = sext(imm)"),
    row!(0x12, "movhi", I, RdImm, U16, 1, None, "movhi rd, uimm16", "rd = (imm << 16) | (rd & 0xFFFF)"),
    row!(0x13, "lea", I, RegMem, Mem, 1, None, "lea rd, [m]", "rd = effective address of [m]"),
    row!(0x20, "add", R, RdRsRs, None, 1, Add, "add rd, rs1, rs2", "rd = rs1 + rs2"),
    row!(0x21, "sub", R, RdRsRs, None, 1, Sub, "sub rd, rs1, rs2", "rd = rs1 − rs2"),
    row!(0x22, "mul", R, RdRsRs, None, 3, Mul, "mul rd, rs1, rs2", "rd = rs1 × rs2 (low 32 bits)"),
    row!(0x23, "div", R, RdRsRs, None, 8, Div, "div rd, rs1, rs2", "rd = rs1 / rs2 (signed, rounds toward 0)"),
    row!(0x24, "divu", R, RdRsRs, None, 8, Logic, "divu rd, rs1, rs2", "rd = rs1 / rs2 (unsigned)"),
    row!(0x25, "rem", R, RdRsRs, None, 8, Div, "rem rd, rs1, rs2", "rd = rs1 rem rs2 (signed, sign of dividend)"),
    row!(0x26, "remu", R, RdRsRs, None, 8, Logic, "remu rd, rs1, rs2", "rd = rs1 rem rs2 (unsigned)"),
    row!(0x27, "and", R, RdRsRs, None, 1, Logic, "and rd, rs1, rs2", "rd = rs1 & rs2"),
    row!(0x28, "or", R, RdRsRs, None, 1, Logic, "or rd, rs1, rs2", "rd = rs1 | rs2"),
    row!(0x29, "xor", R, RdRsRs, None, 1, Logic, "xor rd, rs1, rs2", "rd = rs1 ^ rs2"),
    row!(0x2A, "shl", R, RdRsRs, None, 1, Shift, "shl rd, rs1, rs2", "rd = rs1 << (rs2 & 31)"),
    row!(0x2B, "shr", R, RdRsRs, None, 1, Shift, "shr rd, rs1, rs2", "rd = rs1 >> (rs2 & 31), logical"),
    row!(0x2C, "sar", R, RdRsRs, None, 1, Shift, "sar rd, rs1, rs2", "rd = rs1 >> (rs2 & 31), arithmetic"),
    row!(0x2D, "cmp", R, RsRs, None, 1, Sub, "cmp rs1, rs2", "flags of rs1 − rs2"),
    row!(0x2E, "tst", R, RsRs, None, 1, Logic, "tst rs1, rs2", "flags of rs1 & rs2"),
    row!(0x30, "add", I, RdRsImm, S16, 1, Add, "add rd, rs1, simm16", "rd = rs1 + imm"),
    row!(0x31, "sub", I, RdRsImm, S16, 1, Sub, "sub rd, rs1, simm16", "rd = rs1 − imm"),
    row!(0x32, "mul", I, RdRsImm, S16, 3, Mul, "mul rd, rs1, simm16", "rd = rs1 × imm (low 32 bits)"),
    row!(0x33, "div", I, RdRsImm, S16, 8, Div, "div rd, rs1, simm16", "rd = rs1 / imm (signed, rounds toward 0)"),
    row!(0x34, "divu", I, RdRsImm, U16, 8, Logic, "divu rd, rs1, uimm16", "rd = rs1 / imm (unsigned)"),
    row!(0x35, "rem", I, RdRsImm, S16, 8, Div, "rem rd, rs1, simm16", "rd = rs1 rem imm (signed, sign of dividend)"),
    row!(0x36, "remu", I, RdRsImm, U16, 8, Logic, "remu rd, rs1, uimm16", "rd = rs1 rem imm (unsigned)"),
    row!(0x37, "and", I, RdRsImm, U16, 1, Logic, "and rd, rs1, uimm16", "rd = rs1 & imm"),
    row!(0x38, "or", I, RdRsImm, U16, 1, Logic, "or rd, rs1, uimm16", "rd = rs1 | imm"),
    row!(0x39, "xor", I, RdRsImm, U16, 1, Logic, "xor rd, rs1, uimm16", "rd = rs1 ^ imm"),
    row!(0x3A, "shl", I, RdRsImm, Shift, 1, Shift, "shl rd, rs1, sh", "rd = rs1 << sh"),
    row!(0x3B, "shr", I, RdRsImm, Shift, 1, Shift, "shr rd, rs1, sh", "rd = rs1 >> sh, logical"),
    row!(0x3C, "sar", I, RdRsImm, Shift, 1, Shift, "sar rd, rs1, sh", "rd = rs1 >> sh, arithmetic"),
    row!(0x3D, "cmp", I, RsImm, S16, 1, Sub, "cmp rs1, simm16", "flags of rs1 − imm"),
    row!(0x3E, "tst", I, RsImm, U16, 1, Logic, "tst rs1, uimm16", "flags of rs1 & imm"),
    row!(0x40, "neg", R, RdRs, None, 1, Sub, "neg rd, rs", "rd = 0 − rs"),
    row!(0x41, "not", R, RdRs, None, 1, Logic, "not rd, rs", "rd = ~rs"),
    row!(0x50, "ld", I, RegMem, Mem, 2, None, "ld rd, [m]", "Load 32-bit word (address multiple of 4)."),
    row!(0x51, "ldb", I, RegMem, Mem, 2, None, "ldb rd, [m]", "Load byte, zero-extended."),
    row!(0x52, "ldbs", I, RegMem, Mem, 2, None, "ldbs rd, [m]", "Load byte, sign-extended."),
    row!(0x54, "st", I, RegMem, Mem, 2, None, "st rs, [m]", "Store 32-bit word (address multiple of 4)."),
    row!(0x55, "stb", I, RegMem, Mem, 2, None, "stb rs, [m]", "Store lowest byte of rs."),
    row!(0x58, "push", R, Reg, None, 2, None, "push rs", "sp −= 4; mem[sp] = rs (push sp stores the old value)."),
    row!(0x59, "pop", R, Reg, None, 2, None, "pop rd", "v = mem[sp]; sp += 4; rd = v"),
    row!(0x60, "jmp", J, Target, Rel20, 1, None, "jmp L / j<cc> L", "If the condition holds: pc = this + 4 × sext(imm20)."),
    row!(0x61, "call", J, Target, Rel20, 2, None, "call L", "push(pc + 4); pc = this + 4 × sext(imm20)"),
    row!(0x62, "jmp", R, Reg, None, 1, None, "jmp rs", "pc = rs (target checked)"),
    row!(0x63, "call", R, Reg, None, 2, None, "call rs", "push(pc + 4); pc = rs (target checked)"),
];

/// The opcode table, sorted by opcode. Single source of truth for the instruction set.
pub static OPCODES: &[OpInfo] = &OPCODES_ARR;

const NO_ROW: u8 = 0xFF;

const fn build_index() -> [u8; 256] {
    let mut idx = [NO_ROW; 256];
    let mut i = 0;
    while i < OPCODES_ARR.len() {
        idx[OPCODES_ARR[i].opcode as usize] = i as u8;
        i += 1;
    }
    idx
}

const OPCODE_INDEX: [u8; 256] = build_index();

/// Table row for an opcode byte, or `None` for unassigned opcodes.
pub fn op_info(opcode: u8) -> Option<&'static OpInfo> {
    match OPCODE_INDEX[opcode as usize] {
        NO_ROW => None,
        i => OPCODES_ARR.get(i as usize),
    }
}

/// All table rows with this mnemonic (ASCII case ignored). `mov`, `jmp` and `call` have two rows;
/// the conditional jumps are not listed here but via [`Cond::from_mnemonic`].
pub fn rows_for_mnemonic(name: &str) -> impl Iterator<Item = &'static OpInfo> + '_ {
    OPCODES_ARR
        .iter()
        .filter(move |r| r.mnemonic.eq_ignore_ascii_case(name))
}

/// `true` if `name` is a real instruction mnemonic, a jump-condition mnemonic or alias (case ignored).
/// Pseudo-instructions such as `li` are not included.
pub fn is_mnemonic(name: &str) -> bool {
    rows_for_mnemonic(name).next().is_some() || Cond::from_mnemonic(name).is_some()
}

// ---------------------------------------------------------------------------------------------
// Instructions
// ---------------------------------------------------------------------------------------------

/// Memory operand `[m]` of `lea`, `ld`, `ldb`, `ldbs`, `st`, `stb`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mem {
    /// `[base]`, `[base + off]`, `[base - off]`: field B = base, imm16 = signed offset.
    Based { base: Reg, off: i16 },
    /// `[addr]`: field B = 15, imm16 = unsigned absolute address 0…0xFFFF.
    Abs { addr: u16 },
}

impl Mem {
    /// Effective address before the range check: `base + sext(off) mod 2³²`, or `addr`.
    /// Results ≥ 0x10000 must be rejected by the machine with `fault.out_of_range`.
    pub const fn effective_address(self, regs: &[u32; 10]) -> u32 {
        match self {
            Mem::Based { base, off } => regs[base.index()].wrapping_add(off as i32 as u32),
            Mem::Abs { addr } => addr as u32,
        }
    }
}

/// A decoded instruction. Every value for which [`Instr::is_valid`] holds encodes to exactly one
/// word, and that word decodes back to the same value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Instr {
    /// 0x01
    Halt,
    /// 0x02
    Nop,
    /// 0x04 `sys n`
    Sys { n: u16 },
    /// 0x08
    Ret,
    /// 0x10 `mov rd, rs`
    Mov { rd: Reg, rs: Reg },
    /// 0x11 `mov rd, simm16`
    MovI { rd: Reg, imm: i16 },
    /// 0x12 `movhi rd, uimm16`
    MovHi { rd: Reg, imm: u16 },
    /// 0x13 `lea rd, [m]`
    Lea { rd: Reg, m: Mem },
    /// 0x20–0x2E `op rd, rs1, rs2`. For `cmp`/`tst`, `rd` is always [`Reg::R0`] (field A = 0).
    Alu {
        op: AluOp,
        rd: Reg,
        rs1: Reg,
        rs2: Reg,
    },
    /// 0x30–0x3E `op rd, rs1, imm`. `imm` holds the operand value as interpreted by
    /// [`AluOp::imm_kind`] (−32768…32767, 0…65535 or 0…31). For `cmp`/`tst`, `rd` is [`Reg::R0`].
    AluI {
        op: AluOp,
        rd: Reg,
        rs1: Reg,
        imm: i32,
    },
    /// 0x40 `neg rd, rs`
    Neg { rd: Reg, rs: Reg },
    /// 0x41 `not rd, rs`
    Not { rd: Reg, rs: Reg },
    /// 0x50 `ld rd, [m]`
    Ld { rd: Reg, m: Mem },
    /// 0x51 `ldb rd, [m]`
    Ldb { rd: Reg, m: Mem },
    /// 0x52 `ldbs rd, [m]`
    Ldbs { rd: Reg, m: Mem },
    /// 0x54 `st rs, [m]`
    St { rs: Reg, m: Mem },
    /// 0x55 `stb rs, [m]`
    Stb { rs: Reg, m: Mem },
    /// 0x58 `push rs`
    Push { rs: Reg },
    /// 0x59 `pop rd`
    Pop { rd: Reg },
    /// 0x60 `jmp L` / `j<cc> L`; `off` in words relative to this instruction, [`REL20_MIN`]…[`REL20_MAX`].
    Jump { cond: Cond, off: i32 },
    /// 0x61 `call L`; `off` as for [`Instr::Jump`].
    Call { off: i32 },
    /// 0x62 `jmp rs`
    JumpReg { rs: Reg },
    /// 0x63 `call rs`
    CallReg { rs: Reg },
}

/// Why a word is not a valid instruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecodeError {
    /// The word is 0 (`fault.zero_instruction`, "forgot halt?").
    Zero,
    /// Unassigned opcode or a field that violates strict decoding (`fault.illegal_instruction`).
    Illegal,
}

impl Instr {
    /// Opcode byte.
    pub const fn opcode(&self) -> u8 {
        match *self {
            Instr::Halt => 0x01,
            Instr::Nop => 0x02,
            Instr::Sys { .. } => 0x04,
            Instr::Ret => 0x08,
            Instr::Mov { .. } => 0x10,
            Instr::MovI { .. } => 0x11,
            Instr::MovHi { .. } => 0x12,
            Instr::Lea { .. } => 0x13,
            Instr::Alu { op, .. } => 0x20 + op as u8,
            Instr::AluI { op, .. } => 0x30 + op as u8,
            Instr::Neg { .. } => 0x40,
            Instr::Not { .. } => 0x41,
            Instr::Ld { .. } => 0x50,
            Instr::Ldb { .. } => 0x51,
            Instr::Ldbs { .. } => 0x52,
            Instr::St { .. } => 0x54,
            Instr::Stb { .. } => 0x55,
            Instr::Push { .. } => 0x58,
            Instr::Pop { .. } => 0x59,
            Instr::Jump { .. } => 0x60,
            Instr::Call { .. } => 0x61,
            Instr::JumpReg { .. } => 0x62,
            Instr::CallReg { .. } => 0x63,
        }
    }

    /// Table row of this instruction.
    pub fn info(&self) -> &'static OpInfo {
        &OPCODES_ARR[OPCODE_INDEX[self.opcode() as usize] as usize]
    }

    /// Mnemonic as printed by the disassembler (conditional jumps use the condition name).
    pub fn mnemonic(&self) -> &'static str {
        match *self {
            Instr::Jump { cond, .. } => cond.mnemonic(),
            _ => self.info().mnemonic,
        }
    }

    /// Cycle cost from the table.
    pub fn cycles(&self) -> u32 {
        self.info().cycles
    }

    /// Flag effect from the table.
    pub fn flag_effect(&self) -> FlagEffect {
        self.info().flags
    }

    /// Checks the invariants that the type system does not express: immediate ranges per kind,
    /// jump offset range and `rd = r0` for `cmp`/`tst`.
    pub fn is_valid(&self) -> bool {
        match *self {
            Instr::Alu { op, rd, .. } => op.writes_rd() || rd == Reg::R0,
            Instr::AluI { op, rd, imm, .. } => {
                let range_ok = match op.imm_kind().range() {
                    Some((lo, hi)) => (lo..=hi).contains(&(imm as i64)),
                    None => false,
                };
                range_ok && (op.writes_rd() || rd == Reg::R0)
            }
            Instr::Jump { off, .. } | Instr::Call { off } => (REL20_MIN..=REL20_MAX).contains(&off),
            _ => true,
        }
    }
}

const fn word_r(op: u8, a: u32, b: u32, c: u32) -> u32 {
    ((op as u32) << 24) | (a << 20) | (b << 16) | (c << 12)
}

const fn word_i(op: u8, a: u32, b: u32, imm16: u32) -> u32 {
    ((op as u32) << 24) | (a << 20) | (b << 16) | (imm16 & 0xFFFF)
}

const fn word_j(op: u8, a: u32, off: i32) -> u32 {
    ((op as u32) << 24) | (a << 20) | ((off as u32) & 0xF_FFFF)
}

const fn mem_fields(m: Mem) -> (u32, u32) {
    match m {
        Mem::Based { base, off } => (base.code(), off as u16 as u32),
        Mem::Abs { addr } => (ABS_BASE_CODE, addr as u32),
    }
}

/// Encodes an instruction. For values that violate [`Instr::is_valid`] the immediate is truncated
/// to its field and `cmp`/`tst` always get field A = 0; the result is still a well-formed word for
/// all valid inputs only.
pub fn encode(instr: &Instr) -> u32 {
    let op = instr.opcode();
    match *instr {
        Instr::Halt | Instr::Nop | Instr::Ret => word_r(op, 0, 0, 0),
        Instr::Sys { n } => word_i(op, 0, 0, n as u32),
        Instr::Mov { rd, rs } | Instr::Neg { rd, rs } | Instr::Not { rd, rs } => {
            word_r(op, rd.code(), rs.code(), 0)
        }
        Instr::MovI { rd, imm } => word_i(op, rd.code(), 0, imm as u16 as u32),
        Instr::MovHi { rd, imm } => word_i(op, rd.code(), 0, imm as u32),
        Instr::Lea { rd: r, m }
        | Instr::Ld { rd: r, m }
        | Instr::Ldb { rd: r, m }
        | Instr::Ldbs { rd: r, m }
        | Instr::St { rs: r, m }
        | Instr::Stb { rs: r, m } => {
            let (b, imm) = mem_fields(m);
            word_i(op, r.code(), b, imm)
        }
        Instr::Alu {
            op: a,
            rd,
            rs1,
            rs2,
        } => {
            let rd = if a.writes_rd() { rd.code() } else { 0 };
            word_r(op, rd, rs1.code(), rs2.code())
        }
        Instr::AluI {
            op: a,
            rd,
            rs1,
            imm,
        } => {
            let rd = if a.writes_rd() { rd.code() } else { 0 };
            word_i(op, rd, rs1.code(), imm as u32)
        }
        Instr::Push { rs: r }
        | Instr::Pop { rd: r }
        | Instr::JumpReg { rs: r }
        | Instr::CallReg { rs: r } => word_r(op, r.code(), 0, 0),
        Instr::Jump { cond, off } => word_j(op, cond.code(), off),
        Instr::Call { off } => word_j(op, 0, off),
    }
}

fn reg(code: u32) -> Result<Reg, DecodeError> {
    Reg::from_code(code).ok_or(DecodeError::Illegal)
}

fn mem(b: u32, imm16: u32) -> Result<Mem, DecodeError> {
    if b == ABS_BASE_CODE {
        Ok(Mem::Abs { addr: imm16 as u16 })
    } else {
        Ok(Mem::Based {
            base: reg(b)?,
            off: imm16 as u16 as i16,
        })
    }
}

fn require(ok: bool) -> Result<(), DecodeError> {
    if ok {
        Ok(())
    } else {
        Err(DecodeError::Illegal)
    }
}

/// Strictly decodes a word (see `docs/ISA.md`, "Strict decoding"). Never panics.
pub fn decode(word: u32) -> Result<Instr, DecodeError> {
    if word == 0 {
        return Err(DecodeError::Zero);
    }
    let op = (word >> 24) as u8;
    let a = (word >> 20) & 0xF;
    let b = (word >> 16) & 0xF;
    let c = (word >> 12) & 0xF;
    let low12 = word & 0xFFF;
    let imm16 = word & 0xFFFF;
    let low20 = word & 0xF_FFFF;
    let low24 = word & 0xFF_FFFF;

    match op {
        0x01 | 0x02 | 0x08 => {
            require(low24 == 0)?;
            Ok(match op {
                0x01 => Instr::Halt,
                0x02 => Instr::Nop,
                _ => Instr::Ret,
            })
        }
        0x04 => {
            require(a == 0 && b == 0)?;
            Ok(Instr::Sys { n: imm16 as u16 })
        }
        0x10 | 0x40 | 0x41 => {
            require(c == 0 && low12 == 0)?;
            let rd = reg(a)?;
            let rs = reg(b)?;
            Ok(match op {
                0x10 => Instr::Mov { rd, rs },
                0x40 => Instr::Neg { rd, rs },
                _ => Instr::Not { rd, rs },
            })
        }
        0x11 => {
            require(b == 0)?;
            Ok(Instr::MovI {
                rd: reg(a)?,
                imm: imm16 as u16 as i16,
            })
        }
        0x12 => {
            require(b == 0)?;
            Ok(Instr::MovHi {
                rd: reg(a)?,
                imm: imm16 as u16,
            })
        }
        0x13 | 0x50 | 0x51 | 0x52 | 0x54 | 0x55 => {
            let r = reg(a)?;
            let m = mem(b, imm16)?;
            Ok(match op {
                0x13 => Instr::Lea { rd: r, m },
                0x50 => Instr::Ld { rd: r, m },
                0x51 => Instr::Ldb { rd: r, m },
                0x52 => Instr::Ldbs { rd: r, m },
                0x54 => Instr::St { rs: r, m },
                _ => Instr::Stb { rs: r, m },
            })
        }
        0x20..=0x2E => {
            let alu = AluOp::from_nibble((op & 0xF) as u32).ok_or(DecodeError::Illegal)?;
            require(low12 == 0)?;
            require(alu.writes_rd() || a == 0)?;
            Ok(Instr::Alu {
                op: alu,
                rd: reg(a)?,
                rs1: reg(b)?,
                rs2: reg(c)?,
            })
        }
        0x30..=0x3E => {
            let alu = AluOp::from_nibble((op & 0xF) as u32).ok_or(DecodeError::Illegal)?;
            require(alu.writes_rd() || a == 0)?;
            let imm = match alu.imm_kind() {
                ImmKind::S16 => imm16 as u16 as i16 as i32,
                ImmKind::U16 => imm16 as i32,
                ImmKind::Shift => {
                    require(imm16 <= 31)?;
                    imm16 as i32
                }
                _ => return Err(DecodeError::Illegal),
            };
            Ok(Instr::AluI {
                op: alu,
                rd: reg(a)?,
                rs1: reg(b)?,
                imm,
            })
        }
        0x58 | 0x59 | 0x62 | 0x63 => {
            require(low20 == 0)?;
            let r = reg(a)?;
            Ok(match op {
                0x58 => Instr::Push { rs: r },
                0x59 => Instr::Pop { rd: r },
                0x62 => Instr::JumpReg { rs: r },
                _ => Instr::CallReg { rs: r },
            })
        }
        0x60 | 0x61 => {
            // Sign-extend the 20-bit offset.
            let off = ((low20 << 12) as i32) >> 12;
            if op == 0x60 {
                let cond = Cond::from_code(a).ok_or(DecodeError::Illegal)?;
                Ok(Instr::Jump { cond, off })
            } else {
                require(a == 0)?;
                Ok(Instr::Call { off })
            }
        }
        _ => Err(DecodeError::Illegal),
    }
}

// ---------------------------------------------------------------------------------------------
// Generated documentation
// ---------------------------------------------------------------------------------------------

/// The Markdown instruction table printed by `stapelwerk isa-table` and checked into
/// `docs/isa-table.md` (CI compares both byte for byte).
pub fn isa_table_markdown() -> String {
    let mut s = String::new();
    s.push_str("<!-- Generated by `stapelwerk isa-table` from crates/core/src/isa.rs. Do not edit by hand. -->\n\n");
    s.push_str("# Stapelwerk instruction table\n\n");
    s.push_str("Generated from the opcode table in `crates/core/src/isa.rs`, the single source of truth.\n");
    s.push_str("The prose specification is [ISA.md](ISA.md).\n\n");

    s.push_str("## Instructions\n\n");
    s.push_str(
        "| Opcode | Mnemonic | Syntax | Format | Immediate | Cycles | Flags | Description |\n",
    );
    s.push_str("|---|---|---|---|---|---|---|---|\n");
    for r in OPCODES {
        s.push_str(&format!(
            "| 0x{:02X} | `{}` | `{}` | {} | {} | {} | {} | {} |\n",
            r.opcode,
            r.mnemonic,
            r.syntax.replace('|', "\\|"),
            r.format.name(),
            r.imm.name(),
            r.cycles,
            r.flags.name(),
            r.summary.replace('|', "\\|"),
        ));
    }

    s.push_str("\n## Immediate kinds\n\n");
    s.push_str("| Kind | Meaning |\n|---|---|\n");
    for k in [
        ImmKind::S16,
        ImmKind::U16,
        ImmKind::Shift,
        ImmKind::Sys,
        ImmKind::Mem,
        ImmKind::Rel20,
    ] {
        s.push_str(&format!("| {} | {} |\n", k.name(), k.description()));
    }

    s.push_str("\n## Flag effects\n\n");
    s.push_str("| Effect | Z, N | C | V |\n|---|---|---|---|\n");
    for e in FlagEffect::ALL {
        let (zn, c, v) = e.describe();
        s.push_str(&format!("| {} | {} | {} | {} |\n", e.name(), zn, c, v));
    }

    s.push_str("\n## Jump conditions (opcode 0x60, field A)\n\n");
    s.push_str("| Code | Mnemonic | Aliases | Condition | Meaning after `cmp a, b` |\n");
    s.push_str("|---|---|---|---|---|\n");
    for c in Cond::ALL {
        let aliases: Vec<String> = c.aliases().iter().map(|a| format!("`{a}`")).collect();
        let aliases = if aliases.is_empty() {
            "–".to_string()
        } else {
            aliases.join(", ")
        };
        s.push_str(&format!(
            "| 0x{:X} | `{}` | {} | {} | {} |\n",
            c.code(),
            c.mnemonic(),
            aliases,
            c.predicate(),
            c.meaning(),
        ));
    }
    s.push_str("\nCode 0xF is invalid.\n");
    s
}

// ---------------------------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------------------------

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    #[test]
    fn mandatory_encoding_vectors() {
        let cases: [(Instr, u32); 8] = [
            (
                Instr::Alu {
                    op: AluOp::Add,
                    rd: Reg::R1,
                    rs1: Reg::R2,
                    rs2: Reg::R3,
                },
                0x2012_3000,
            ),
            (
                Instr::AluI {
                    op: AluOp::Add,
                    rd: Reg::R1,
                    rs1: Reg::R2,
                    imm: -1,
                },
                0x3012_FFFF,
            ),
            (
                Instr::Ld {
                    rd: Reg::R0,
                    m: Mem::Based {
                        base: Reg::Fp,
                        off: -8,
                    },
                },
                0x5008_FFF8,
            ),
            (
                Instr::Ld {
                    rd: Reg::R0,
                    m: Mem::Abs { addr: 0x4000 },
                },
                0x500F_4000,
            ),
            (Instr::Push { rs: Reg::Fp }, 0x5880_0000),
            (
                Instr::Jump {
                    cond: Cond::Z,
                    off: -3,
                },
                0x601F_FFFD,
            ),
            (Instr::Sys { n: 2 }, 0x0400_0002),
            (Instr::Halt, 0x0100_0000),
        ];
        for (instr, word) in cases {
            assert!(instr.is_valid(), "{instr:?}");
            assert_eq!(encode(&instr), word, "encode {instr:?}");
            assert_eq!(decode(word), Ok(instr), "decode {word:#010x}");
        }
    }

    #[test]
    fn table_is_sorted_unique_and_indexed() {
        for pair in OPCODES.windows(2) {
            assert!(pair[0].opcode < pair[1].opcode);
        }
        for r in OPCODES {
            assert_eq!(op_info(r.opcode), Some(r));
        }
        assert_eq!(op_info(0x00), None);
        assert_eq!(op_info(0x2F), None);
        assert_eq!(op_info(0x3F), None);
        assert_eq!(op_info(0xFF), None);
    }

    #[test]
    fn alu_rows_agree_between_forms() {
        for op in AluOp::ALL {
            let r = op.info_reg();
            let i = op.info_imm();
            assert_eq!(r.opcode, 0x20 + op as u8);
            assert_eq!(i.opcode, 0x30 + op as u8);
            assert_eq!(r.mnemonic, i.mnemonic);
            assert_eq!(r.cycles, i.cycles);
            assert_eq!(r.flags, i.flags);
            assert_eq!(r.format, Format::R);
            assert_eq!(i.format, Format::I);
            let (rs, is) = if op.writes_rd() {
                (Shape::RdRsRs, Shape::RdRsImm)
            } else {
                (Shape::RsRs, Shape::RsImm)
            };
            assert_eq!(r.shape, rs);
            assert_eq!(i.shape, is);
        }
    }

    #[test]
    fn zero_and_illegal_words() {
        assert_eq!(decode(0), Err(DecodeError::Zero));
        let illegal = [
            0x0000_0001, // opcode 0, non-zero
            0x0100_0001, // halt with junk
            0x0300_0000, // unassigned
            0x0410_0001, // sys with A != 0
            0x10A0_0000, // mov r10
            0x1012_1000, // mov with C != 0
            0x1101_0005, // mov imm with B != 0
            0x2F12_3000, // opcode 0x2F
            0x2012_3001, // low 12 bits set
            0x2D11_2000, // cmp with A != 0
            0x3A12_0020, // shl by 32
            0x50AF_0000, // ld into r10
            0x500E_0000, // ld with base 14
            0x5880_0001, // push with junk
            0x6F00_0000, // condition 15
            0x6110_0000, // call with A != 0
            0x2012_F000, // rs2 = 15
            0x13F0_0000, // lea into register 15
        ];
        for w in illegal {
            assert_eq!(decode(w), Err(DecodeError::Illegal), "{w:#010x}");
        }
    }

    #[test]
    fn conditions_and_aliases() {
        assert_eq!(Cond::from_mnemonic("JE"), Some(Cond::Z));
        assert_eq!(Cond::from_mnemonic("jne"), Some(Cond::Nz));
        assert_eq!(Cond::from_mnemonic("jmp"), Some(Cond::Always));
        assert_eq!(Cond::from_mnemonic("jxx"), None);
        // cmp 3, 5: 3 - 5 = -2 -> N=1, V=0, C=1 (borrow), Z=0
        let f = FLAG_N | FLAG_C;
        assert!(Cond::Lt.holds(f));
        assert!(Cond::B.holds(f));
        assert!(Cond::Be.holds(f));
        assert!(!Cond::Ge.holds(f));
        assert!(!Cond::A.holds(f));
        assert!(Cond::Le.holds(FLAG_Z));
        assert!(!Cond::Gt.holds(FLAG_Z));
    }

    #[test]
    fn register_names() {
        assert_eq!(Reg::from_name("FP"), Some(Reg::Fp));
        assert_eq!(Reg::from_name("r7"), Some(Reg::R7));
        assert_eq!(Reg::from_name("r8"), None);
        assert_eq!(Reg::from_code(10), None);
        assert!(Reg::Fp.is_callee_saved() && !Reg::Sp.is_callee_saved());
    }

    fn random_reg(rng: &mut SplitMix64) -> Reg {
        Reg::ALL[rng.below(10) as usize]
    }

    fn random_mem(rng: &mut SplitMix64) -> Mem {
        if rng.below(4) == 0 {
            Mem::Abs {
                addr: rng.next_u32() as u16,
            }
        } else {
            Mem::Based {
                base: random_reg(rng),
                off: rng.next_u32() as u16 as i16,
            }
        }
    }

    /// Generates a random valid instruction, biased towards immediate boundaries.
    pub(crate) fn random_instr(rng: &mut SplitMix64) -> Instr {
        let row = OPCODES[rng.below(OPCODES.len() as u64) as usize];
        let edge = |rng: &mut SplitMix64, lo: i64, hi: i64| -> i64 {
            match rng.below(4) {
                0 => lo,
                1 => hi,
                _ => rng.range_i64(lo, hi),
            }
        };
        match row.opcode {
            0x01 => Instr::Halt,
            0x02 => Instr::Nop,
            0x04 => Instr::Sys {
                n: edge(rng, 0, 65535) as u16,
            },
            0x08 => Instr::Ret,
            0x10 => Instr::Mov {
                rd: random_reg(rng),
                rs: random_reg(rng),
            },
            0x11 => Instr::MovI {
                rd: random_reg(rng),
                imm: edge(rng, -32768, 32767) as i16,
            },
            0x12 => Instr::MovHi {
                rd: random_reg(rng),
                imm: edge(rng, 0, 65535) as u16,
            },
            0x13 => Instr::Lea {
                rd: random_reg(rng),
                m: random_mem(rng),
            },
            0x20..=0x2E => {
                let op = AluOp::from_nibble((row.opcode & 0xF) as u32).unwrap();
                Instr::Alu {
                    op,
                    rd: if op.writes_rd() {
                        random_reg(rng)
                    } else {
                        Reg::R0
                    },
                    rs1: random_reg(rng),
                    rs2: random_reg(rng),
                }
            }
            0x30..=0x3E => {
                let op = AluOp::from_nibble((row.opcode & 0xF) as u32).unwrap();
                let (lo, hi) = op.imm_kind().range().unwrap();
                Instr::AluI {
                    op,
                    rd: if op.writes_rd() {
                        random_reg(rng)
                    } else {
                        Reg::R0
                    },
                    rs1: random_reg(rng),
                    imm: edge(rng, lo, hi) as i32,
                }
            }
            0x40 => Instr::Neg {
                rd: random_reg(rng),
                rs: random_reg(rng),
            },
            0x41 => Instr::Not {
                rd: random_reg(rng),
                rs: random_reg(rng),
            },
            0x50 => Instr::Ld {
                rd: random_reg(rng),
                m: random_mem(rng),
            },
            0x51 => Instr::Ldb {
                rd: random_reg(rng),
                m: random_mem(rng),
            },
            0x52 => Instr::Ldbs {
                rd: random_reg(rng),
                m: random_mem(rng),
            },
            0x54 => Instr::St {
                rs: random_reg(rng),
                m: random_mem(rng),
            },
            0x55 => Instr::Stb {
                rs: random_reg(rng),
                m: random_mem(rng),
            },
            0x58 => Instr::Push {
                rs: random_reg(rng),
            },
            0x59 => Instr::Pop {
                rd: random_reg(rng),
            },
            0x60 => Instr::Jump {
                cond: Cond::ALL[rng.below(15) as usize],
                off: edge(rng, REL20_MIN as i64, REL20_MAX as i64) as i32,
            },
            0x61 => Instr::Call {
                off: edge(rng, REL20_MIN as i64, REL20_MAX as i64) as i32,
            },
            0x62 => Instr::JumpReg {
                rs: random_reg(rng),
            },
            0x63 => Instr::CallReg {
                rs: random_reg(rng),
            },
            other => panic!("table row {other:#04x} not covered by the generator"),
        }
    }

    #[test]
    fn roundtrip_random_instructions() {
        let mut rng = SplitMix64::new(0x5EED_0001);
        let mut seen = [false; 256];
        for _ in 0..200_000 {
            let instr = random_instr(&mut rng);
            assert!(instr.is_valid(), "{instr:?}");
            let w = encode(&instr);
            assert_eq!(decode(w), Ok(instr), "word {w:#010x}");
            assert_eq!(instr.opcode() as u32, w >> 24);
            seen[instr.opcode() as usize] = true;
        }
        for r in OPCODES {
            assert!(
                seen[r.opcode as usize],
                "opcode {:#04x} never generated",
                r.opcode
            );
        }
    }

    #[test]
    fn random_words_never_panic_and_reencode() {
        let mut rng = SplitMix64::new(42);
        let mut valid = 0u32;
        for _ in 0..500_000 {
            // Mix fully random words with words that have a valid opcode byte.
            let mut w = rng.next_u32();
            if rng.below(2) == 0 {
                let row = OPCODES[rng.below(OPCODES.len() as u64) as usize];
                w = (w & 0x00FF_FFFF) | ((row.opcode as u32) << 24);
            }
            if rng.below(2) == 0 {
                w &= !0xFFF;
            }
            if rng.below(2) == 0 {
                let a = rng.below(10) as u32;
                let b = rng.below(10) as u32;
                let c = rng.below(10) as u32;
                w = (w & 0xFF00_0FFF) | (a << 20) | (b << 16) | (c << 12);
            }
            if let Ok(instr) = decode(w) {
                valid += 1;
                assert!(instr.is_valid(), "{w:#010x} -> {instr:?}");
                assert_eq!(encode(&instr), w, "{w:#010x} -> {instr:?}");
            }
        }
        assert!(valid > 1000);
    }

    /// Spec test 4: all 2³² words. Run with `cargo test --release -- --ignored` (CI only).
    #[test]
    #[ignore = "exhaustive over 2^32 words; run in release mode (CI)"]
    fn exhaustive_decode() {
        let mut valid: u64 = 0;
        let mut w: u32 = 0;
        loop {
            if let Ok(instr) = decode(w) {
                valid += 1;
                assert_eq!(encode(&instr), w, "{w:#010x} -> {instr:?}");
            }
            if w == u32::MAX {
                break;
            }
            w += 1;
        }
        assert!(valid > 0);
    }

    #[test]
    fn effective_address_wraps() {
        let mut regs = [0u32; 10];
        regs[Reg::Fp.index()] = 0xEFF0;
        let m = Mem::Based {
            base: Reg::Fp,
            off: -8,
        };
        assert_eq!(m.effective_address(&regs), 0xEFE8);
        regs[Reg::R1.index()] = 4;
        let m = Mem::Based {
            base: Reg::R1,
            off: -8,
        };
        assert_eq!(m.effective_address(&regs), 0xFFFF_FFFC);
        assert_eq!(Mem::Abs { addr: 0x4000 }.effective_address(&regs), 0x4000);
    }

    #[test]
    fn isa_table_mentions_every_row() {
        let t = isa_table_markdown();
        for r in OPCODES {
            assert!(t.contains(&format!("| 0x{:02X} |", r.opcode)));
        }
        assert!(t.ends_with('\n'));
    }
}
