//! Disassembler: canonical text form of instructions (`docs/ISA.md`, "Canonical text form").
//! The assembler must accept every line this module prints and produce the same word again
//! (`asm(dis(w)) == w`).

use crate::consts::syscall_name;
use crate::isa::{AluOp, ImmKind, Instr, Mem, decode};
use crate::program::{Program, SymbolKind};

/// One disassembled word.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DisasmLine {
    pub addr: u32,
    pub word: u32,
    /// Canonical text, or `.word 0x…` for words that do not decode.
    pub text: String,
    /// Label defined at `addr`, if any.
    pub label: Option<String>,
}

/// Absolute target of a J-format instruction at `addr` with word offset `off`
/// (may lie outside 0…0xFFFF; printed in decimal then).
pub const fn jump_target(addr: u32, off: i32) -> i64 {
    addr as i64 + 4 * off as i64
}

/// Jump target text: `0x` and four upper-case hex digits inside 0…0xFFFF, else signed decimal.
fn target_text(t: i64) -> String {
    if (0..=0xFFFF).contains(&t) {
        format!("0x{t:04X}")
    } else {
        t.to_string()
    }
}

/// Memory operand text: `[fp]`, `[fp + 8]`, `[fp - 8]`, `[0x4000]`.
fn mem_text(m: Mem) -> String {
    match m {
        Mem::Based { base, off: 0 } => format!("[{base}]"),
        Mem::Based { base, off } if off < 0 => format!("[{base} - {}]", -(off as i32)),
        Mem::Based { base, off } => format!("[{base} + {off}]"),
        Mem::Abs { addr } => format!("[0x{addr:04X}]"),
    }
}

/// Immediate text of an ALU operation: decimal for s16 and shift widths, `0x…` for u16.
fn alu_imm_text(op: AluOp, imm: i32) -> String {
    match op.imm_kind() {
        ImmKind::U16 => format!("0x{imm:X}"),
        _ => imm.to_string(),
    }
}

/// Canonical text of `instr` located at `addr` (needed for jump targets).
pub fn format_instr(instr: &Instr, addr: u32) -> String {
    format_with(instr, addr, &|t| target_text(t))
}

/// Like [`format_instr`], but jump and call targets that a code label of `program` points at are
/// printed as that label (for debugger views). The text still assembles to the same word.
pub fn format_instr_labeled(instr: &Instr, addr: u32, program: &Program) -> String {
    format_with(instr, addr, &|t| match label_at(program, t) {
        Some(name) => name.to_string(),
        None => target_text(t),
    })
}

fn format_with(instr: &Instr, addr: u32, target: &dyn Fn(i64) -> String) -> String {
    let m = instr.mnemonic();
    match *instr {
        Instr::Halt | Instr::Nop | Instr::Ret => m.to_string(),
        Instr::Sys { n } => match syscall_name(n) {
            Some(name) => format!("sys {name}"),
            None => format!("sys {n}"),
        },
        Instr::Mov { rd, rs } | Instr::Neg { rd, rs } | Instr::Not { rd, rs } => {
            format!("{m} {rd}, {rs}")
        }
        Instr::MovI { rd, imm } => format!("{m} {rd}, {imm}"),
        Instr::MovHi { rd, imm } => format!("{m} {rd}, 0x{imm:X}"),
        Instr::Lea { rd: r, m: mm }
        | Instr::Ld { rd: r, m: mm }
        | Instr::Ldb { rd: r, m: mm }
        | Instr::Ldbs { rd: r, m: mm }
        | Instr::St { rs: r, m: mm }
        | Instr::Stb { rs: r, m: mm } => format!("{m} {r}, {}", mem_text(mm)),
        Instr::Alu { op, rd, rs1, rs2 } => {
            if op.writes_rd() {
                format!("{m} {rd}, {rs1}, {rs2}")
            } else {
                format!("{m} {rs1}, {rs2}")
            }
        }
        Instr::AluI { op, rd, rs1, imm } => {
            let i = alu_imm_text(op, imm);
            if op.writes_rd() {
                format!("{m} {rd}, {rs1}, {i}")
            } else {
                format!("{m} {rs1}, {i}")
            }
        }
        Instr::Push { rs: r }
        | Instr::Pop { rd: r }
        | Instr::JumpReg { rs: r }
        | Instr::CallReg { rs: r } => format!("{m} {r}"),
        Instr::Jump { off, .. } | Instr::Call { off } => {
            format!("{m} {}", target(jump_target(addr, off)))
        }
    }
}

/// Name of a label (code or data, not a constant) at `addr`; global labels win over local ones.
fn label_at(program: &Program, addr: i64) -> Option<&str> {
    let mut found: Option<&str> = None;
    for s in &program.symbols {
        if s.kind != SymbolKind::Const && s.value == addr {
            if !s.is_local() {
                return Some(&s.name);
            }
            found.get_or_insert(&s.name);
        }
    }
    found
}

/// Canonical text of `word` at `addr`, or `None` if the word does not decode.
pub fn disasm_word(word: u32, addr: u32) -> Option<String> {
    decode(word).ok().map(|i| format_instr(&i, addr))
}

/// Disassembles `count` words from `addr` (rounded down to a multiple of 4) out of raw memory,
/// naming labels from `program`. Stops at the end of `mem`.
pub fn disasm_range(
    mem: &[u8],
    program: Option<&Program>,
    addr: u32,
    count: u32,
) -> Vec<DisasmLine> {
    let start = addr & !3;
    let mut out = Vec::with_capacity(count.min(4096) as usize);
    for i in 0..count {
        let Some(a) = start.checked_add(i.saturating_mul(4)) else {
            break;
        };
        let Some(bytes) = mem.get(a as usize..a as usize + 4) else {
            break;
        };
        let word = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let text = match decode(word) {
            Ok(instr) => format_instr(&instr, a),
            Err(_) => format!(".word 0x{word:08X}"),
        };
        let label = program
            .and_then(|p| label_at(p, a as i64))
            .map(str::to_string);
        out.push(DisasmLine {
            addr: a,
            word,
            text,
            label,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asm::{MnemonicFilter, assemble};
    use crate::isa::tests::random_instr;
    use crate::isa::{Cond, OPCODES, REL20_MAX, REL20_MIN, Reg, encode};
    use crate::rng::SplitMix64;

    #[test]
    fn jump_targets() {
        assert_eq!(jump_target(0x10C, -3), 0x100);
        assert_eq!(jump_target(0x100, -(1 << 19)), 0x100 - (1 << 21));
    }

    fn dis(w: u32, addr: u32) -> String {
        disasm_word(w, addr).unwrap()
    }

    #[test]
    fn canonical_forms() {
        assert_eq!(dis(0x2012_3000, 0x100), "add r1, r2, r3");
        assert_eq!(dis(0x3012_FFFF, 0x100), "add r1, r2, -1");
        assert_eq!(dis(0x5008_FFF8, 0x100), "ld r0, [fp - 8]");
        assert_eq!(dis(0x500F_4000, 0x100), "ld r0, [0x4000]");
        assert_eq!(dis(0x5880_0000, 0x100), "push fp");
        assert_eq!(dis(0x601F_FFFD, 0x10C), "jz 0x0100");
        assert_eq!(dis(0x0400_0002, 0x100), "sys WRITE_INT");
        assert_eq!(dis(0x0400_0063, 0x100), "sys 99");
        assert_eq!(dis(0x0100_0000, 0x100), "halt");
        assert_eq!(dis(0x3712_00FF, 0x100), "and r1, r2, 0xFF");
        assert_eq!(dis(0x3E01_0000, 0x100), "tst r1, 0x0");
        assert_eq!(dis(0x3D01_FFFB, 0x100), "cmp r1, -5");
        assert_eq!(dis(0x3A11_0003, 0x100), "shl r1, r1, 3");
        assert_eq!(dis(0x5008_0000, 0x100), "ld r0, [fp]");
        assert_eq!(dis(0x5008_0008, 0x100), "ld r0, [fp + 8]");
        assert_eq!(dis(0x5008_8000, 0x100), "ld r0, [fp - 32768]");
        assert_eq!(dis(0x1210_ABCD, 0x100), "movhi r1, 0xABCD");
        assert_eq!(dis(0x6008_0000, 0x100), "jmp -2096896");
        assert_eq!(dis(0x6200_0000, 0x100), "jmp r0");
    }

    /// Spec test 5: `asm(dis(w)) == w` for random valid instructions (in batches of 1000 lines,
    /// each disassembled at the address it will be assembled to).
    #[test]
    fn text_roundtrip_random() {
        let mut rng = SplitMix64::new(0x5EED_1B00);
        let filter = MnemonicFilter::All;
        for _ in 0..100 {
            let mut src = String::new();
            let mut expect = Vec::new();
            for k in 0..1000u32 {
                let i = random_instr(&mut rng);
                src.push_str(&format_instr(&i, 0x100 + 4 * k));
                src.push('\n');
                expect.push(encode(&i));
            }
            let p = assemble(&src, None, &filter).unwrap_or_else(|d| panic!("{:?}", &d[..1]));
            assert_eq!(&p.words[..1000], &expect[..]);
        }
    }

    /// Every immediate kind at both limits, every condition at both jump limits.
    #[test]
    fn text_roundtrip_limits() {
        let mut instrs = Vec::new();
        for row in OPCODES {
            if let Some(op) = AluOp::from_nibble((row.opcode & 0xF) as u32)
                && row.opcode >> 4 == 3
            {
                let (lo, hi) = op.imm_kind().range().unwrap();
                for imm in [lo, hi] {
                    let rd = if op.writes_rd() { Reg::R7 } else { Reg::R0 };
                    instrs.push(Instr::AluI {
                        op,
                        rd,
                        rs1: Reg::Sp,
                        imm: imm as i32,
                    });
                }
            }
        }
        for imm in [i16::MIN, i16::MAX, 0] {
            instrs.push(Instr::MovI { rd: Reg::Fp, imm });
            instrs.push(Instr::Ld {
                rd: Reg::R1,
                m: Mem::Based {
                    base: Reg::Sp,
                    off: imm,
                },
            });
        }
        for imm in [0, u16::MAX] {
            instrs.push(Instr::MovHi { rd: Reg::R3, imm });
            instrs.push(Instr::Sys { n: imm });
            instrs.push(Instr::Stb {
                rs: Reg::R2,
                m: Mem::Abs { addr: imm },
            });
        }
        for off in [REL20_MIN, REL20_MAX, 0, -1, 1] {
            for cond in Cond::ALL {
                instrs.push(Instr::Jump { cond, off });
            }
            instrs.push(Instr::Call { off });
        }
        let mut src = String::new();
        for (k, i) in instrs.iter().enumerate() {
            src.push_str(&format_instr(i, 0x100 + 4 * k as u32));
            src.push('\n');
        }
        let p = assemble(&src, None, &MnemonicFilter::All).unwrap_or_else(|d| panic!("{d:?}"));
        for (k, i) in instrs.iter().enumerate() {
            assert_eq!(
                p.words[k],
                encode(i),
                "{}",
                format_instr(i, 0x100 + 4 * k as u32)
            );
        }
    }

    #[test]
    fn range_with_labels() {
        let src = "main: mov r1, 1\nloop: sub r1, 1\n jnz loop\n halt\n.data\nv: .word 7\n";
        let p = assemble(src, None, &MnemonicFilter::All).unwrap();
        let mut mem = vec![0u8; 0x10000];
        for (i, w) in p.words.iter().enumerate() {
            mem[0x100 + 4 * i..0x104 + 4 * i].copy_from_slice(&w.to_le_bytes());
        }
        let lines = disasm_range(&mem, Some(&p), 0x100, 6);
        assert_eq!(lines[0].label.as_deref(), Some("main"));
        assert_eq!(lines[1].label.as_deref(), Some("loop"));
        assert_eq!(lines[2].text, "jnz 0x0104");
        assert_eq!(lines[4].text, ".word 0x00000000");
        assert_eq!(lines[5].label.as_deref(), Some("__trap"));
        let i = decode(p.words[2]).unwrap();
        assert_eq!(format_instr_labeled(&i, 0x108, &p), "jnz loop");
        assert!(disasm_range(&mem, None, 0xFFFC, 3).len() == 1);
    }
}
