//! `asm`, `dis` and `run`.

use crate::args::Parsed;
use crate::messages;
use stapelwerk_core::asm::{MnemonicFilter, assemble};
use stapelwerk_core::consts::{OUT_KIND_INT, TEXT_START};
use stapelwerk_core::diag::FILE_PLAYER;
use stapelwerk_core::disasm::{disasm_word, format_instr_labeled};
use stapelwerk_core::isa::decode;
use stapelwerk_core::isa::{FLAG_C, FLAG_N, FLAG_V, FLAG_Z, Reg};
use stapelwerk_core::level::parse_int;
use stapelwerk_core::machine::{Machine, MachineConfig, Status};
use stapelwerk_core::program::Program;
use stapelwerk_core::session::{LineCounter, run_to_end};
use std::io::Write;
use std::path::Path;

/// Magic bytes of a memory image written by `asm -o`.
pub const IMAGE_MAGIC: &[u8; 8] = b"STPWIMG1";

/// Image layout: magic (8 bytes), text length and data length in bytes (u32 little-endian
/// each), the text words from 0x0100 (little-endian), then the data bytes from 0x4000.
pub fn write_image(p: &Program) -> Vec<u8> {
    let mut out = Vec::with_capacity(16 + p.words.len() * 4 + p.data.len());
    out.extend_from_slice(IMAGE_MAGIC);
    out.extend_from_slice(&(p.words.len() as u32 * 4).to_le_bytes());
    out.extend_from_slice(&(p.data.len() as u32).to_le_bytes());
    for w in &p.words {
        out.extend_from_slice(&w.to_le_bytes());
    }
    out.extend_from_slice(&p.data);
    out
}

/// Reads an image written by [`write_image`]: (text words, data bytes).
pub fn read_image(bytes: &[u8]) -> Result<(Vec<u32>, Vec<u8>), String> {
    let bad = || "not a Stapelwerk image (expected the format written by 'asm -o')".to_string();
    let rest = bytes.strip_prefix(IMAGE_MAGIC.as_slice()).ok_or_else(bad)?;
    let u32_at = |i: usize| -> Option<u32> {
        rest.get(i..i + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    };
    let (text, data) = (u32_at(0).ok_or_else(bad)?, u32_at(4).ok_or_else(bad)?);
    if text % 4 != 0 || 16 + text as u64 + data as u64 != bytes.len() as u64 {
        return Err(bad());
    }
    let body = &rest[8..];
    let words = body[..text as usize]
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| u32::from_le_bytes(*b))
        .collect();
    Ok((words, body[text as usize..].to_vec()))
}

fn read_text(path: &str, err: &mut dyn Write) -> Option<String> {
    match std::fs::read(path) {
        Ok(b) => Some(String::from_utf8_lossy(&b).into_owned()),
        Err(e) => {
            let _ = writeln!(err, "stapelwerk: cannot read '{path}': {e}");
            None
        }
    }
}

/// Assembles a file without library; prints diagnostics on failure.
fn assemble_file(path: &str, err: &mut dyn Write) -> Result<Program, u8> {
    let src = read_text(path, err).ok_or(1u8)?;
    match assemble(&src, None, &MnemonicFilter::All) {
        Ok(p) => Ok(p),
        Err(diags) => {
            for d in &diags {
                let _ = writeln!(err, "{}", messages::diagnostic(path, d));
            }
            let _ = writeln!(err, "stapelwerk: {} error(s) in '{path}'", diags.len());
            Err(1)
        }
    }
}

pub fn cmd_asm(p: &Parsed, out: &mut dyn Write, err: &mut dyn Write) -> u8 {
    let path = &p.positional[0];
    let program = match assemble_file(path, err) {
        Ok(prog) => prog,
        Err(code) => return code,
    };
    match p.value("-o") {
        Some(target) => match std::fs::write(target, write_image(&program)) {
            Ok(()) => {
                let _ = writeln!(
                    out,
                    "wrote {target}: {} code words (size {}), {} data bytes",
                    program.words.len(),
                    program.size,
                    program.data.len()
                );
                0
            }
            Err(e) => {
                let _ = writeln!(err, "stapelwerk: cannot write '{target}': {e}");
                1
            }
        },
        None => {
            let _ = writeln!(
                out,
                "ok: {} code words (size {}), {} data bytes",
                program.words.len(),
                program.size,
                program.data.len()
            );
            0
        }
    }
}

pub fn cmd_dis(p: &Parsed, out: &mut dyn Write, err: &mut dyn Write) -> u8 {
    let path = &p.positional[0];
    let is_source = Path::new(path)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("s"));
    let (words, data, program) = if is_source {
        match assemble_file(path, err) {
            Ok(prog) => (prog.words.clone(), prog.data.clone(), Some(prog)),
            Err(code) => return code,
        }
    } else {
        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) => {
                let _ = writeln!(err, "stapelwerk: cannot read '{path}': {e}");
                return 1;
            }
        };
        match read_image(&bytes) {
            Ok((w, d)) => (w, d, None),
            Err(e) => {
                let _ = writeln!(err, "stapelwerk: '{path}': {e}");
                return 1;
            }
        }
    };
    let mut text = String::new();
    for (i, &w) in words.iter().enumerate() {
        let addr = TEXT_START + 4 * i as u32;
        if let Some(prog) = &program {
            for s in prog.symbols.iter().filter(|s| {
                s.value == addr as i64 && s.kind == stapelwerk_core::program::SymbolKind::Code
            }) {
                text.push_str(&format!("{}:\n", s.name));
            }
        }
        let ins = instr_text(w, addr, program.as_ref());
        let line = program
            .as_ref()
            .and_then(|pr| pr.line_of(addr))
            .map(|e| {
                if e.file == FILE_PLAYER {
                    format!("  ; line {}", e.line)
                } else {
                    format!("  ; library line {}", e.line)
                }
            })
            .unwrap_or_default();
        text.push_str(&format!("    0x{addr:04X}  {w:08X}  {ins}{line}\n"));
    }
    if !data.is_empty() {
        text.push_str(&format!("; .data: {} bytes from 0x4000\n", data.len()));
    }
    match out.write_all(text.as_bytes()) {
        Ok(()) => 0,
        Err(_) => 1,
    }
}

/// Disassembly of one word; jump targets as labels when the program is known.
fn instr_text(word: u32, addr: u32, program: Option<&Program>) -> String {
    match (decode(word), program) {
        (Ok(i), Some(p)) => format_instr_labeled(&i, addr, p),
        _ => disasm_word(word, addr).unwrap_or_else(|| format!(".word 0x{word:08X}")),
    }
}

fn flags_text(f: u32) -> String {
    [(FLAG_Z, 'Z'), (FLAG_N, 'N'), (FLAG_C, 'C'), (FLAG_V, 'V')]
        .iter()
        .map(|&(bit, c)| if f & bit != 0 { c } else { '-' })
        .collect()
}

/// The visible output: numbers in decimal (a newline between two consecutive numbers), bytes as
/// they are; a final newline is added if missing.
pub fn format_output(m: &Machine) -> Vec<u8> {
    let n = m.state.out_len as usize;
    let mut out = Vec::new();
    let mut prev_num = false;
    for (k, v) in m.tapes.out_kinds.iter().zip(&m.tapes.out_vals).take(n) {
        if *k == OUT_KIND_INT {
            if prev_num {
                out.push(b'\n');
            }
            out.extend_from_slice(v.to_string().as_bytes());
            prev_num = true;
        } else {
            out.push(*v as u8);
            prev_num = false;
        }
    }
    if !out.is_empty() && out.last() != Some(&b'\n') {
        out.push(b'\n');
    }
    out
}

pub fn cmd_run(p: &Parsed, out: &mut dyn Write, err: &mut dyn Write) -> u8 {
    let path = &p.positional[0];
    let mut input = Vec::new();
    if let Some(s) = p.value("--in") {
        for w in s
            .split(|c: char| c.is_whitespace() || c == ',')
            .filter(|w| !w.is_empty())
        {
            match parse_int(w).and_then(|v| i32::try_from(v).ok()) {
                Some(v) => input.push(v),
                None => {
                    let _ = writeln!(
                        err,
                        "stapelwerk: --in: '{w}' is not a 32-bit integer (example: --in \"3 4 -5 0\")"
                    );
                    return 2;
                }
            }
        }
    }
    let program = match assemble_file(path, err) {
        Ok(prog) => prog,
        Err(code) => return code,
    };
    let mut m = Machine::new(&program, MachineConfig::default());
    m.tapes.in_ints = input;
    m.tapes.in_text = p.value("--in-text").unwrap_or("").as_bytes().to_vec();
    let mut lines = LineCounter::new(&program);
    let status = if p.flag("--trace") {
        trace(&mut m, &program, &mut lines, out)
    } else {
        run_to_end(&mut m, None, &mut lines)
    };
    let _ = out.write_all(&format_output(&m));
    let s = &m.state;
    let line_of = |pc: u32| {
        program
            .line_of(pc)
            .filter(|e| e.file == FILE_PLAYER)
            .map_or(String::new(), |e| format!(" (line {})", e.line))
    };
    match status {
        Status::Halted => {
            let _ = writeln!(
                err,
                "-- halted after {} steps, {} cycles",
                s.steps, s.cycles
            );
            0
        }
        Status::Fault => {
            if let Some(f) = &s.fault {
                let _ = writeln!(
                    err,
                    "-- fault at 0x{:04X}{} after {} steps: {}",
                    f.pc,
                    line_of(f.pc),
                    f.step,
                    messages::fault(f)
                );
            }
            1
        }
        Status::BudgetExceeded => {
            let hottest = lines.hottest_line();
            let args = vec![
                ("cycles", s.cycles.into()),
                ("hottest_line", hottest.into()),
            ];
            let _ = writeln!(
                err,
                "-- {}",
                messages::describe("run.budget_exceeded", &args)
            );
            1
        }
        other => {
            let _ = writeln!(err, "-- stopped with status {}", other.name());
            1
        }
    }
}

/// Single-steps the machine and prints one line per executed instruction: step number, pc,
/// instruction and the registers, flags and output it changed.
fn trace(
    m: &mut Machine,
    program: &Program,
    lines: &mut LineCounter,
    out: &mut dyn Write,
) -> Status {
    loop {
        let before = m.state.clone();
        let word = before.mem.peek_u32(before.pc);
        let text = instr_text(word, before.pc, Some(program));
        let status = m.step();
        if m.state.steps != before.steps {
            lines.count(before.pc);
            let mut changes = Vec::new();
            for i in 0..Reg::COUNT {
                let (a, b) = (before.regs[i], m.state.regs[i]);
                if a != b {
                    let r = Reg::from_code(i as u32).map_or("?", Reg::name);
                    if i >= Reg::Fp.index() {
                        changes.push(format!("{r}=0x{b:04X}"));
                    } else {
                        changes.push(format!("{r}={}", b as i32));
                    }
                }
            }
            if before.flags != m.state.flags {
                changes.push(format!("flags={}", flags_text(m.state.flags)));
            }
            if m.state.out_len > before.out_len {
                let i = before.out_len as usize;
                let (k, v) = (m.tapes.out_kinds.get(i), m.tapes.out_vals.get(i));
                if let (Some(&k), Some(&v)) = (k, v) {
                    if k == OUT_KIND_INT {
                        changes.push(format!("out {v}"));
                    } else {
                        changes.push(format!("out byte {v}"));
                    }
                }
            }
            let line = format!(
                "{:>6}  0x{:04X}  {:<24}  {}",
                m.state.steps,
                before.pc,
                text,
                changes.join(" ")
            );
            let _ = writeln!(out, "{}", line.trim_end());
        }
        if status != Status::Paused {
            return status;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_round_trip() {
        let p = assemble(
            "  mov r1, 5\n  halt\n.data\n  .byte 1, 2, 3\n",
            None,
            &MnemonicFilter::All,
        )
        .unwrap();
        let img = write_image(&p);
        let (w, d) = read_image(&img).unwrap();
        assert_eq!(w, p.words);
        assert_eq!(d, p.data);
        assert!(read_image(b"STPWIMG1\x04\0\0\0\0\0\0\0").is_err());
        assert!(read_image(b"junk").is_err());
    }
}
