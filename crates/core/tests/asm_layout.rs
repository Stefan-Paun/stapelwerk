//! Assembler layout and binding rules: player + library, zero word, trap, size, line map,
//! symbols, filters, diagnostics across both files, UTF-16 columns, section limits, speed.

use std::time::Instant;

use stapelwerk_core::asm::{MnemonicFilter, assemble};
use stapelwerk_core::diag::{Arg, Diagnostic, FILE_LIBRARY, FILE_PLAYER};
use stapelwerk_core::program::{Program, SymbolKind};

fn ok(src: &str, lib: Option<&str>) -> Program {
    assemble(src, lib, &MnemonicFilter::All).unwrap_or_else(|d| panic!("{d:#?}"))
}

fn errs(src: &str, lib: Option<&str>) -> Vec<Diagnostic> {
    match assemble(src, lib, &MnemonicFilter::All) {
        Ok(_) => panic!("expected errors"),
        Err(d) => d,
    }
}

fn arg<'a>(d: &'a Diagnostic, name: &str) -> Option<&'a Arg> {
    d.args.iter().find(|(n, _)| *n == name).map(|(_, a)| a)
}

const HALT: u32 = 0x0100_0000;

#[test]
fn layout_player_zero_word_library_trap() {
    let src = "main: li r1, 5\n      call lib_f\n      halt\n.data\npd: .byte 1\n";
    let lib = "lib_f: ret\n.data\nld_: .word 7\n";
    let p = ok(src, Some(lib));
    // player: li (2) + call + halt = 4 words at 0x100..0x110, zero word at 0x110,
    // library ret at 0x114, trap at 0x118.
    assert_eq!(p.words.len(), 7);
    assert_eq!(p.words[4], 0);
    assert_eq!(p.words[5], 0x0800_0000);
    assert_eq!(p.words[6], HALT);
    assert_eq!(p.trap_addr, 0x118);
    assert_eq!(p.text_len, 28);
    assert_eq!(p.code_end(), 0x11C);
    assert_eq!(p.symbol("lib_f").unwrap().value, 0x114);
    assert_eq!(p.symbol("__trap").unwrap().value, 0x118);
    // data: player byte at 0x4000, library word aligned to 0x4004.
    assert_eq!(p.symbol("pd").unwrap().value, 0x4000);
    assert_eq!(p.symbol("ld_").unwrap().value, 0x4004);
    assert_eq!(p.data, vec![1, 0, 0, 0, 7, 0, 0, 0]);
    // size counts the player's words only (li = 2); no line-map entry for zero and trap words.
    assert_eq!(p.size, 4);
    let addrs: Vec<u32> = p.linemap.iter().map(|e| e.addr).collect();
    assert_eq!(addrs, vec![0x100, 0x104, 0x108, 0x10C, 0x114]);
    assert_eq!(p.line_of(0x104).map(|e| e.line), Some(1));
    assert_eq!(p.line_of(0x114).map(|e| e.file), Some(FILE_LIBRARY));
    assert_eq!(p.line_of(0x110), None);
    assert_eq!(p.line_of(0x118), None);
    // Symbols: player first, then library, then __trap.
    let names: Vec<&str> = p.symbols.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, vec!["main", "pd", "lib_f", "ld_", "__trap"]);
    assert_eq!(p.symbol("pd").unwrap().kind, SymbolKind::Data);
}

#[test]
fn layout_without_library_and_empty_program() {
    let p = ok("halt\n", None);
    assert_eq!(p.words, vec![HALT, 0, HALT]);
    assert_eq!(p.trap_addr, 0x108);
    let p = ok("; nichts\n", None);
    assert_eq!(p.words, vec![0, HALT]);
    assert_eq!(p.size, 0);
    assert!(p.linemap.is_empty());
}

#[test]
fn align_padding_is_not_size() {
    let p = ok("halt\n.align 16\nend: halt\n", None);
    assert_eq!(p.symbol("end").unwrap().value, 0x110);
    assert_eq!(
        &p.words[..5],
        &[HALT, 0x0200_0000, 0x0200_0000, 0x0200_0000, HALT]
    );
    assert_eq!(p.size, 2);
}

#[test]
fn library_constants_visible_to_player_and_ordered() {
    let lib = ".equ A, 1\n.equ B, A + 1\nf: ret\n";
    let p = ok("mov r0, B\nhalt\n", Some(lib));
    assert_eq!(p.words[0], 0x1100_0002);
    let b = p.symbol("B").unwrap();
    assert_eq!(
        (b.kind, b.value, b.file, b.line),
        (SymbolKind::Const, 2, FILE_LIBRARY, 2)
    );

    // Later library constant, library label and player constant are not usable in the pre-pass.
    let lib = ".equ A, B\n.equ B, 1\n.equ C, f\n.equ D, P\nf: ret\n";
    let d = errs(".equ P, 3\nhalt\n", Some(lib));
    let got: Vec<(&str, u8, u32)> = d.iter().map(|d| (d.key, d.file, d.line)).collect();
    assert_eq!(
        got,
        vec![
            ("asm.needs_constant", FILE_LIBRARY, 1),
            ("asm.needs_constant", FILE_LIBRARY, 3),
            ("asm.needs_constant", FILE_LIBRARY, 4),
        ]
    );
}

#[test]
fn player_equ_rules() {
    // Earlier labels and constants are fine; labels may be used before their definition.
    let p = ok(
        "a: nop\nb: nop\n.equ LEN, b - a\n.equ TWO, LEN / 2\nmov r0, TWO\njmp c\nc: halt\n",
        None,
    );
    assert_eq!(p.symbol("LEN").unwrap().value, 4);
    assert_eq!(p.words[2], 0x1100_0002);
    // A constant defined later cannot be used, not even in pass-2 contexts.
    let d = errs(".data\n.word K\n.text\nmov r0, K\n.equ K, 1\n", None);
    assert_eq!(d.len(), 2);
    assert!(d.iter().all(|d| d.key == "asm.needs_constant"));
    // A constant defined twice, also across files.
    let d = errs(".equ K, 1\nK: halt\n", Some(".equ Q, 2\n"));
    assert_eq!(d[0].key, "asm.duplicate_label");
    let d = errs("Q: halt\n", Some("\n.equ Q, 2\n"));
    assert_eq!(
        (d[0].key, d[0].file, d[0].line),
        ("asm.duplicate_label", FILE_PLAYER, 1)
    );
    assert_eq!(arg(&d[0], "first_line"), Some(&Arg::Int(2)));
}

#[test]
fn shared_namespace_and_library_errors() {
    let d = errs("f: halt\n", Some("nop\nf: ret\nbogus r1\n"));
    let got: Vec<(&str, u8, u32, u32)> = d.iter().map(|d| (d.key, d.file, d.line, d.col)).collect();
    assert_eq!(
        got,
        vec![
            ("asm.duplicate_label", FILE_LIBRARY, 2, 1),
            ("asm.unknown_mnemonic", FILE_LIBRARY, 3, 1),
        ]
    );
    assert_eq!(arg(&d[0], "first_line"), Some(&Arg::Int(1)));
    // The library may call player labels (one namespace).
    let p = ok("main: call g\nhalt\ncb: ret\n", Some("g: call cb\nret\n"));
    assert_eq!(p.symbol("g").unwrap().value, 0x110);
}

#[test]
fn filter_applies_to_player_only() {
    let filter = MnemonicFilter::Allow(vec!["call".into(), "halt".into(), "jz".into()]);
    let lib = "f: mul r0, r0\nret\n";
    assert!(assemble("call f\nje f\nhalt\n", Some(lib), &filter).is_ok());
    let d = assemble("call f\nli r0, 1\nhalt\n", Some(lib), &filter).unwrap_err();
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].key, "asm.forbidden_instruction");
    assert_eq!(arg(&d[0], "mnemonic"), Some(&Arg::Str("li".into())));
}

#[test]
fn beginner_mistakes() {
    let d = errs("mov r1, 70000\n", None);
    assert_eq!(d[0].key, "asm.imm_range");
    assert_eq!(arg(&d[0], "hint"), Some(&Arg::Str("li".into())));
    assert_eq!((d[0].col, d[0].len), (9, 5));
    assert_eq!(errs("ld r1, r2\n", None)[0].key, "asm.operand_type");
    let d = errs("add r1\n", None);
    assert_eq!(d[0].key, "asm.operand_count");
    assert_eq!(arg(&d[0], "expected"), Some(&Arg::Str("2-3".into())));
    assert_eq!(arg(&d[0], "got"), Some(&Arg::Int(1)));
    let d = errs("jmp ende\nhalt\n", None);
    assert_eq!(
        (d[0].key, d[0].col, d[0].len),
        ("asm.undefined_symbol", 5, 4)
    );
    let d = errs("loop: nop\nloop: halt\n", None);
    assert_eq!(d[0].key, "asm.duplicate_label");
    assert_eq!((d[0].line, d[0].col, d[0].len), (2, 1, 4));
    assert_eq!(arg(&d[0], "first_line"), Some(&Arg::Int(1)));
    // Missing colon after a label: the label is taken as a mnemonic.
    let d = errs("lop add r1, 1\n", None);
    assert_eq!(d[0].key, "asm.unknown_mnemonic");
    // Missing comma.
    assert_eq!(errs("add r1 r2\n", None)[0].key, "asm.unexpected_char");
    // Error recovery: one diagnostic per bad line, all lines reported, sorted.
    let d = errs("ad r1, 1\nhalt\nmov r9, 1\njmp x\n", None);
    let lines: Vec<u32> = d.iter().map(|d| d.line).collect();
    assert_eq!(lines, vec![1, 3, 4]);
}

#[test]
fn utf16_columns_after_umlauts_and_emoji() {
    // 'ä' is one UTF-16 unit (two UTF-8 bytes), '😀' two units (four bytes).
    let d = errs(".data\n.word 'ä', '😀', nope ; Größe\n", None);
    assert_eq!(d[0].key, "asm.undefined_symbol");
    assert_eq!((d[0].line, d[0].col, d[0].len), (2, 18, 4));
    let d = errs("; Zähler für Schleifen 😀\n  mov r1, 99999\n", None);
    assert_eq!((d[0].line, d[0].col), (2, 11));
    let d = errs(".data\ns: .ascii \"😀😀\" x\n", None);
    assert_eq!((d[0].key, d[0].col), ("asm.unexpected_char", 18));
}

#[test]
fn text_overflow() {
    let mut src = String::from("main:\n");
    for _ in 0..4030 {
        src.push_str("nop\n");
    }
    // 4030 + zero word + trap = 4032 words = 16128 bytes: fits exactly.
    assert!(assemble(&src, None, &MnemonicFilter::All).is_ok());
    src.push_str("halt\n");
    let d = errs(&src, None);
    assert_eq!(d.len(), 1);
    assert_eq!((d[0].key, d[0].line), ("asm.section_overflow", 4032));
    assert_eq!(arg(&d[0], "section"), Some(&Arg::Str("text".into())));
    assert_eq!(arg(&d[0], "size"), Some(&Arg::Int(16132)));
    assert_eq!(arg(&d[0], "max"), Some(&Arg::Int(16128)));
}

#[test]
fn data_fits_exactly() {
    assert!(assemble("halt\n.data\n.space 8192\n", None, &MnemonicFilter::All).is_ok());
    let d = errs("halt\n.data\n.space 8190\n", Some(".data\n.byte 1\n"));
    assert_eq!(d[0].key, "asm.section_overflow");
    assert_eq!(d[0].file, FILE_LIBRARY);
}

/// A 500-line program assembled repeatedly; prints the time per run. Must stay well below 5 ms
/// in release builds (the bound here is generous for unoptimised test builds).
#[test]
fn assembles_quickly() {
    let mut src = String::new();
    src.push_str(".equ N, 10\nmain:   mov r4, 0\n");
    for i in 0..120 {
        src.push_str(&format!(
            "f{i}:    ld r0, [fp - 8]     ; Zeile {i}\n        add r0, r0, N * 2\n        jnz f{}\n        li r1, SCREEN + {i}\n",
            (i + 1) % 120
        ));
    }
    src.push_str("        halt\n.data\n");
    for i in 0..18 {
        src.push_str(&format!("d{i}: .word {i}, d{i}, f{i}\n"));
    }
    assert!(src.lines().count() >= 500);
    let p = ok(&src, None);
    assert!(p.size > 500);
    let runs = 20u32;
    let t = Instant::now();
    for _ in 0..runs {
        let _ = assemble(&src, None, &MnemonicFilter::All);
    }
    let per = t.elapsed() / runs;
    println!("assemble {} lines: {:?} per run", src.lines().count(), per);
    assert!(per.as_millis() < 100, "too slow: {per:?}");
}
