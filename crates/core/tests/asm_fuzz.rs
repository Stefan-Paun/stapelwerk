//! Robustness (spec test 7 for the assembler) and the text round trip over random words
//! (spec test 5, second generator: random words that decode).

use std::fs;
use std::path::PathBuf;

use stapelwerk_core::asm::{MnemonicFilter, assemble};
use stapelwerk_core::consts::MAX_ASM_ERRORS;
use stapelwerk_core::diag::is_known_key;
use stapelwerk_core::disasm::format_instr;
use stapelwerk_core::isa::{OPCODES, decode};
use stapelwerk_core::rng::SplitMix64;

fn sources() -> Vec<String> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/asm");
    let mut v: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|e| fs::read_to_string(e.unwrap().path()).unwrap())
        .collect();
    v.sort();
    v.push(
        "main: push fp\n mov fp, sp\n .l: ld r0, [fp - 8]\n jz .l\n li r1, 'x'\n sys WRITE_CHAR\n\
         .data\n s: .string \"ä\\n\"\n w: .word s, main.l\n"
            .into(),
    );
    v
}

const PIECES: &[&str] = &[
    " ",
    ",",
    ":",
    ";",
    "[",
    "]",
    "(",
    ")",
    "+",
    "-",
    "*",
    "/",
    "%",
    "<<",
    ">>",
    "&",
    "|",
    "^",
    "~",
    "'",
    "\"",
    "\\",
    ".",
    "0x",
    "0b",
    "_",
    "r1",
    "fp",
    "sp",
    "r9",
    "li",
    "mov",
    "jmp",
    ".word",
    ".data",
    ".text",
    ".equ",
    ".align",
    ".space",
    "main",
    ".l",
    "ä",
    "😀",
    "\t",
    "\n",
    "99999999999999999999",
    "-0x8000",
    "SCREEN",
    "__x",
    "\u{0}",
    "\r\n",
];

fn mutate(rng: &mut SplitMix64, src: &str) -> String {
    let mut s: Vec<char> = src.chars().collect();
    for _ in 0..=rng.below(4) {
        let n = s.len().max(1) as u64;
        match rng.below(6) {
            0 if !s.is_empty() => {
                let i = rng.below(n) as usize;
                let j = (i + 1 + rng.below(8) as usize).min(s.len());
                s.drain(i..j);
            }
            1 | 2 => {
                let i = rng.below(n + 1).min(s.len() as u64) as usize;
                let p = PIECES[rng.below(PIECES.len() as u64) as usize];
                for (k, c) in p.chars().enumerate() {
                    s.insert(i + k, c);
                }
            }
            3 if !s.is_empty() => {
                let i = rng.below(n) as usize;
                s[i] = char::from_u32(rng.below(0x80) as u32).unwrap_or('?');
            }
            4 => {
                let i = rng.below(n + 1).min(s.len() as u64) as usize;
                s.truncate(i);
            }
            _ => {
                // Duplicate a chunk (creates duplicate labels, long lines).
                if !s.is_empty() {
                    let i = rng.below(n) as usize;
                    let j = (i + rng.below(40) as usize).min(s.len());
                    let chunk: Vec<char> = s[i..j].to_vec();
                    s.splice(i..i, chunk);
                }
            }
        }
    }
    s.into_iter().collect()
}

fn check(src: &str, lib: Option<&str>, filter: &MnemonicFilter) {
    match assemble(src, lib, filter) {
        Ok(p) => {
            assert_eq!(p.text_len as usize, p.words.len() * 4);
            assert!(p.text_len <= 16128 && p.data.len() <= 8192);
            assert!(p.linemap.windows(2).all(|w| w[0].addr < w[1].addr));
            assert_eq!(p.words.last(), Some(&0x0100_0000));
            assert_eq!(p.trap_addr, p.code_end() - 4);
        }
        Err(d) => {
            assert!(!d.is_empty());
            assert!(d.len() <= MAX_ASM_ERRORS + 1);
            for x in &d {
                assert!(
                    x.key.starts_with("asm.") && is_known_key(x.key),
                    "{}",
                    x.key
                );
                assert!(x.line >= 1 && x.col >= 1, "{x:?}");
                assert!(x.file <= 1);
            }
            assert!(
                d.windows(2)
                    .all(|w| (w[0].file, w[0].line, w[0].col) <= (w[1].file, w[1].line, w[1].col))
            );
        }
    }
}

#[test]
fn corrupted_sources_never_panic() {
    let srcs = sources();
    let mut rng = SplitMix64::new(0x00F0_221B);
    let forbid = MnemonicFilter::Forbid(vec!["mul".into(), "li".into()]);
    for i in 0..12_000 {
        let base = &srcs[rng.below(srcs.len() as u64) as usize];
        let m = mutate(&mut rng, base);
        if i % 3 == 0 {
            let k = rng.below(srcs.len() as u64) as usize;
            let lib = mutate(&mut rng, &srcs[k]);
            check(&m, Some(&lib), &forbid);
        } else {
            check(&m, None, &MnemonicFilter::All);
        }
    }
}

#[test]
fn random_bytes_never_panic() {
    let mut rng = SplitMix64::new(0xB17E5);
    for _ in 0..12_000 {
        let len = rng.below(160) as usize;
        let bytes: Vec<u8> = (0..len)
            .map(|_| match rng.below(4) {
                0 => rng.next_u32() as u8,
                1 => b"\n,:[]+-'\".;"[rng.below(11) as usize],
                _ => b'a' + rng.below(26) as u8,
            })
            .collect();
        let s = String::from_utf8_lossy(&bytes);
        check(&s, None, &MnemonicFilter::All);
        check("halt\n", Some(&s), &MnemonicFilter::All);
    }
}

#[test]
fn hostile_inputs() {
    let deep = format!("mov r0, {}1{}\n", "(".repeat(5000), ")".repeat(5000));
    check(&deep, None, &MnemonicFilter::All);
    let unary = format!("mov r0, {}1\n", "-".repeat(5000));
    check(&unary, None, &MnemonicFilter::All);
    let long = format!("mov r0, {}\n", vec!["1"; 20000].join("+"));
    check(&long, None, &MnemonicFilter::All);
    check(
        &".data\n.space 8192\n".repeat(2000),
        None,
        &MnemonicFilter::All,
    );
    check(&"x\n".repeat(10_000), None, &MnemonicFilter::All);
    check(&"l: ".repeat(3000), None, &MnemonicFilter::All);
    check("\u{feff}halt\n\r\n\r", None, &MnemonicFilter::All);
}

/// `asm(dis(w)) == w` for random words that decode (independent of the instruction generator
/// used by the unit test in `disasm.rs`).
#[test]
fn text_roundtrip_random_words() {
    let mut rng = SplitMix64::new(0x000D_15A5);
    let mut total = 0;
    while total < 100_000 {
        let mut src = String::new();
        let mut expect = Vec::new();
        while expect.len() < 1000 {
            let mut w = rng.next_u32();
            let row = OPCODES[rng.below(OPCODES.len() as u64) as usize];
            w = (w & 0x00FF_FFFF) | ((row.opcode as u32) << 24);
            if rng.below(2) == 0 {
                w &= !0xFFF;
            }
            if rng.below(2) == 0 {
                let (a, b, c) = (
                    rng.below(10) as u32,
                    rng.below(10) as u32,
                    rng.below(10) as u32,
                );
                w = (w & 0xFF00_0FFF) | (a << 20) | (b << 16) | (c << 12);
            }
            if let Ok(i) = decode(w) {
                let addr = 0x100 + 4 * expect.len() as u32;
                src.push_str(&format_instr(&i, addr));
                src.push('\n');
                expect.push(w);
            }
        }
        let p = assemble(&src, None, &MnemonicFilter::All).unwrap_or_else(|d| panic!("{:?}", d[0]));
        assert_eq!(&p.words[..expect.len()], &expect[..]);
        total += expect.len();
    }
}
