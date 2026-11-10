//! Assembler golden tests (spec test 6): every `tests/asm/*.s` file is assembled and checked
//! against the expectations in its comments:
//!
//! - `; expect-word: 0x… [0x…]` – the instruction words of this line (via the line map);
//! - `; expect-error: KEY LINE:COL [arg=value …]` – the file must fail with exactly these
//!   diagnostics (key, line and column; listed arguments must match, others are not checked);
//! - `; expect-bytes: ADDR hh hh …` – bytes of the data segment;
//! - `; expect-symbol: NAME VALUE` – a symbol's value;
//! - `; golden-filter: allow|forbid m1 m2 …` – the mnemonic filter to assemble with.

use std::fs;
use std::path::PathBuf;

use stapelwerk_core::asm::{MnemonicFilter, assemble};
use stapelwerk_core::diag::{Arg, Diagnostic, FILE_PLAYER};
use stapelwerk_core::program::Program;

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/asm")
}

fn parse_int(s: &str) -> i64 {
    let s = s.trim();
    let (neg, s) = match s.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, s),
    };
    let v = match s.strip_prefix("0x") {
        Some(h) => i64::from_str_radix(h, 16).expect(s),
        None => s.parse::<i64>().expect(s),
    };
    if neg { -v } else { v }
}

struct ErrExpect {
    key: String,
    line: u32,
    col: u32,
    args: Vec<(String, String)>,
}

fn arg_text(a: &Arg) -> String {
    match a {
        Arg::Int(v) => v.to_string(),
        Arg::Str(s) => s.clone(),
    }
}

fn show(d: &[Diagnostic]) -> String {
    d.iter()
        .map(|d| {
            let args: Vec<String> = d
                .args
                .iter()
                .map(|(k, v)| format!("{k}={}", arg_text(v)))
                .collect();
            format!(
                "  {} {}:{} len={} {}",
                d.key,
                d.line,
                d.col,
                d.len,
                args.join(" ")
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn words_of_line(p: &Program, line: u32) -> Vec<u32> {
    p.linemap
        .iter()
        .filter(|e| e.file == FILE_PLAYER && e.line == line)
        .map(|e| p.words[((e.addr - 0x100) / 4) as usize])
        .collect()
}

fn check_file(name: &str, src: &str) -> Vec<String> {
    let mut failures = Vec::new();
    let mut filter = MnemonicFilter::All;
    let mut errors: Vec<ErrExpect> = Vec::new();
    let mut words: Vec<(u32, Vec<u32>)> = Vec::new();
    let mut bytes: Vec<(u32, i64, Vec<u8>)> = Vec::new();
    let mut symbols: Vec<(u32, String, i64)> = Vec::new();
    for (i, text) in src.lines().enumerate() {
        let ln = (i + 1) as u32;
        let Some((_, comment)) = text.split_once(';') else {
            continue;
        };
        let comment = comment.trim();
        if let Some(rest) = comment.strip_prefix("golden-filter:") {
            let mut it = rest.split_whitespace();
            let mode = it.next().unwrap_or("");
            let list: Vec<String> = it.map(str::to_string).collect();
            filter = match mode {
                "allow" => MnemonicFilter::Allow(list),
                _ => MnemonicFilter::Forbid(list),
            };
        } else if let Some(rest) = comment.strip_prefix("expect-word:") {
            let ws = rest
                .split_whitespace()
                .map(|w| parse_int(w) as u32)
                .collect();
            words.push((ln, ws));
        } else if let Some(rest) = comment.strip_prefix("expect-error:") {
            let mut it = rest.split_whitespace();
            let key = it.next().expect("key").to_string();
            let pos = it.next().expect("line:col");
            let (l, c) = pos.split_once(':').expect("line:col");
            let args = it
                .map(|a| {
                    let (k, v) = a.split_once('=').expect("arg=value");
                    (k.to_string(), v.to_string())
                })
                .collect();
            errors.push(ErrExpect {
                key,
                line: l.parse().unwrap(),
                col: c.parse().unwrap(),
                args,
            });
        } else if let Some(rest) = comment.strip_prefix("expect-bytes:") {
            let mut it = rest.split_whitespace();
            let addr = parse_int(it.next().unwrap());
            let bs = it.map(|b| u8::from_str_radix(b, 16).unwrap()).collect();
            bytes.push((ln, addr, bs));
        } else if let Some(rest) = comment.strip_prefix("expect-symbol:") {
            let mut it = rest.split_whitespace();
            let n = it.next().unwrap().to_string();
            symbols.push((ln, n, parse_int(it.next().unwrap())));
        }
    }

    match assemble(src, None, &filter) {
        Ok(p) => {
            if !errors.is_empty() {
                failures.push(format!("{name}: expected errors, but it assembled"));
            }
            for (ln, exp) in &words {
                let got = words_of_line(&p, *ln);
                if &got != exp {
                    failures.push(format!(
                        "{name}:{ln}: words {:08X?}, expected {:08X?}",
                        got, exp
                    ));
                }
            }
            for (ln, addr, exp) in &bytes {
                let off = (*addr - 0x4000) as usize;
                let got = p.data.get(off..off + exp.len());
                if got != Some(exp.as_slice()) {
                    failures.push(format!(
                        "{name}:{ln}: bytes {got:02X?}, expected {exp:02X?}"
                    ));
                }
            }
            for (ln, n, v) in &symbols {
                let got = p.symbol(n).map(|s| s.value);
                if got != Some(*v) {
                    failures.push(format!(
                        "{name}:{ln}: symbol {n} = {got:?}, expected {v:#x}"
                    ));
                }
            }
        }
        Err(diags) => {
            if errors.is_empty() {
                failures.push(format!("{name}: unexpected errors:\n{}", show(&diags)));
                return failures;
            }
            let mut ok = diags.len() == errors.len();
            for (d, e) in diags.iter().zip(&errors) {
                ok &= d.key == e.key && d.line == e.line && d.col == e.col && d.file == FILE_PLAYER;
                for (k, v) in &e.args {
                    let got = d
                        .args
                        .iter()
                        .find(|(n, _)| n == k)
                        .map(|(_, a)| arg_text(a));
                    ok &= got.as_deref() == Some(v.as_str());
                }
            }
            if !ok {
                failures.push(format!(
                    "{name}: diagnostics differ; got:\n{}",
                    show(&diags)
                ));
            }
        }
    }
    failures
}

#[test]
fn golden_files() {
    let mut entries: Vec<PathBuf> = fs::read_dir(golden_dir())
        .expect("tests/asm exists")
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "s"))
        .collect();
    entries.sort();
    assert!(entries.len() >= 5, "golden files missing");
    let mut failures = Vec::new();
    for path in &entries {
        let src = fs::read_to_string(path).unwrap();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        failures.extend(check_file(&name, &src));
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Every `asm.*` key has at least one golden error case.
#[test]
fn every_asm_key_has_a_golden_case() {
    let mut all = String::new();
    for e in fs::read_dir(golden_dir()).unwrap() {
        all.push_str(&fs::read_to_string(e.unwrap().path()).unwrap());
    }
    for (key, _) in stapelwerk_core::diag::ALL_KEYS {
        if key.starts_with("asm.") {
            assert!(
                all.contains(&format!("expect-error: {key} ")),
                "no golden case for {key}"
            );
        }
    }
}

/// Every opcode table row has at least one golden `expect-word` line.
#[test]
fn every_opcode_has_a_golden_word() {
    let src = fs::read_to_string(golden_dir().join("instructions.s")).unwrap();
    let mut seen = [false; 256];
    for line in src.lines() {
        if let Some((_, c)) = line.split_once("; expect-word:") {
            for w in c.split_whitespace() {
                seen[(parse_int(w) as u32 >> 24) as usize] = true;
            }
        }
    }
    for r in stapelwerk_core::isa::OPCODES {
        assert!(
            seen[r.opcode as usize],
            "opcode {:#04x} has no golden word",
            r.opcode
        );
    }
}
