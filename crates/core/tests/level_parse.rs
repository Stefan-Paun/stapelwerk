//! Level parser: the sample level, every `lvl.*` diagnostic with its line, `level_info` JSON and
//! robustness against corrupted files.

use stapelwerk_core::asm::MnemonicFilter;
use stapelwerk_core::heap::HeapCheck;
use stapelwerk_core::isa::Reg;
use stapelwerk_core::level::pack::pack_source;
use stapelwerk_core::level::{AddrRef, LevelKind, MemValues, ScreenExpect, parse, split_hints};
use stapelwerk_core::rng::SplitMix64;
use stapelwerk_core::runner::{assemble_level, run_tests_with};

const SAMPLE: &str = include_str!("../../../levels/2-2-bis-zur-null.lvl");

/// A small valid level; `extra_level` goes into `[level]`, `tests` replaces the test sections.
fn level(extra_level: &str, tests: &str) -> String {
    format!("[level]\nid = t\n{extra_level}\n[start]\n  halt\n\n[reference]\n  halt\n\n{tests}")
}

fn keys(src: &str) -> Vec<(&'static str, u32)> {
    match parse(src) {
        Ok(_) => Vec::new(),
        Err(d) => d.iter().map(|d| (d.key, d.line)).collect(),
    }
}

#[test]
fn sample_level_parses() {
    let l = parse(SAMPLE).expect("sample level");
    assert_eq!(l.id, "2-2-bis-zur-null");
    assert_eq!((l.chapter, l.order, l.version), (2, 2, 1));
    assert_eq!(l.kind, LevelKind::Normal);
    assert_eq!(l.title_de, "Bis zur Null");
    assert_eq!(l.title_en, "Until Zero");
    assert_eq!(l.budget, 100_000);
    assert_eq!(l.seed, 2202);
    assert!(!l.mobile);
    assert_eq!(l.heap_check, HeapCheck::Strict);
    assert_eq!(l.filter, MnemonicFilter::All);
    assert_eq!(l.tests.len(), 15);
    assert_eq!(l.tests.iter().filter(|t| t.hidden).count(), 11);
    assert_eq!(l.tests[0].name, "grundfall");
    assert_eq!(l.tests[0].input, vec![3, 4, 5, 0]);
    assert_eq!(l.tests[0].out, Some(vec![12]));
    for lang in ["de", "en"] {
        let hints = l.hints(lang);
        assert_eq!(hints.len(), 3, "{lang}");
        assert!(hints[2].starts_with("```"), "{lang}");
        assert!(l.section(&format!("text.{lang}")).is_some());
        assert!(l.section(&format!("learned.{lang}")).is_some());
    }
    assert!(l.section("reference").unwrap().contains("sys WRITE_INT"));
    assert!(l.section("start").unwrap().starts_with("; Bis zur Null"));
    // Code fences and Markdown in text sections stay content.
    assert!(l.section("learned.en").unwrap().contains("```c"));
}

#[test]
fn info_json_shape() {
    let l = parse(SAMPLE).unwrap();
    let j = l.info_json();
    assert!(j.starts_with(
        r#"{"id":"2-2-bis-zur-null","version":1,"chapter":2,"order":2,"kind":"normal","title":{"de":"Bis zur Null","en":"Until Zero"},"card":{"de":"#
    ));
    let details = |input: &str, out: &str| {
        format!(
            r#""in":[{input}],"in_text":"","out":[{out}],"out_text":null,"regs":[],"set":[],"call":null,"expect_reg":[],"expect":[],"screen_init":null,"expect_screen":null,"heap_empty":false"#
        )
    };
    let tests = format!(
        r#""tests":[{{"index":0,"name":"grundfall",{}}},{{"index":1,"name":"leere-folge",{}}},{{"index":2,"name":"negativ",{}}},{{"index":3,"name":"zwischensumme-null",{}}}]"#,
        details("3,4,5,0", "12"),
        details("0", "0"),
        details("-7,10,0", "3"),
        details("100,-100,42,0", "42"),
    );
    assert!(
        j.contains(&format!(
            r#""budget":100000,"seed":2202,"mobile":false,"heap_check":"strict","allow":null,"forbid":null,{tests},"hidden_tests":11,"ref_cycles":null,"ref_size":null,"sections":["text.de","text.en","hint.de","hint.en","learned.de","learned.en","start"]}}"#
        )),
        "{j}"
    );
    assert!(!j.contains("verdeckt"), "hidden tests stay hidden");
}

#[test]
fn info_json_test_details() {
    let src = level(
        "",
        "[test]\nname = a\nin = -1 0x10\nin_text = \"x;y\\n\"\n\
         reg r1 = -2\nset 0x4000 = 1 -1\nsetb DATA = \"ab\"\n\
         screen_init = bild\nout =\nout_text = \"ok\"\nexpect 0x4000 = 7\nexpectb ZIEL = 9 255\n\
         expect_reg sp = 0xF000\nexpect_screen = bild\nexpect_heap_empty = yes\n\n\
         [test]\nname = f\ncall = fn\nreg r3 = 4\nexpect_screen = reference\n\n\
         [test]\nname = h\nhidden = yes\nin = 99\n\n\
         [image.bild]\nF.\n",
    );
    let l = parse(&src).unwrap_or_else(|d| panic!("{d:?}"));
    let j = l.info_json();
    assert!(
        j.contains(
            r#"{"index":0,"name":"a","in":[-1,16],"in_text":"x;y\n","out":[],"out_text":"ok","regs":[{"reg":"r1","value":-2}],"set":[{"addr":16384,"words":[1,-1]},{"addr":"DATA","bytes":[97,98]}],"call":null,"expect_reg":[{"reg":"sp","value":61440}],"expect":[{"addr":16384,"words":[7]},{"addr":"ZIEL","bytes":[9,255]}],"screen_init":"bild","expect_screen":"bild","heap_empty":true}"#
        ),
        "{j}"
    );
    assert!(
        j.contains(
            r#"{"index":1,"name":"f","in":[],"in_text":"","out":null,"out_text":null,"regs":[{"reg":"r3","value":4}],"set":[],"call":"fn","expect_reg":[],"expect":[],"screen_init":null,"expect_screen":"reference","heap_empty":false}],"hidden_tests":1"#
        ),
        "{j}"
    );
    assert!(!j.contains("99"));
}

#[test]
fn addresses_with_constants_and_offsets() {
    let src = level(
        "",
        "[test]\nname = a\nset INPUT = 1\nset INPUT + 8 = 2\nsetb SCREEN-1 = 3\n\
         set tabelle + 0x10 = 4\nexpect STACK_TOP - 4 = 5\nexpectb ZIEL - 2 = 6\nexpect HEAP = 7\n",
    );
    let l = parse(&src).unwrap_or_else(|d| panic!("{d:?}"));
    let t = &l.tests[0];
    let addrs: Vec<&AddrRef> = t.sets.iter().chain(&t.expects).map(|s| &s.addr).collect();
    assert_eq!(
        addrs,
        vec![
            &AddrRef::Num(0x6000),
            &AddrRef::Num(0x6008),
            &AddrRef::Num(0xEFFF),
            &AddrRef::Offset("tabelle".into(), 16),
            &AddrRef::Num(0xEFFC),
            &AddrRef::Offset("ZIEL".into(), -2),
            &AddrRef::Num(0x8000),
        ]
    );
    // level_info: built-in constants are resolved, program symbols stay text.
    let j = l.info_json();
    assert!(
        j.contains(r#""set":[{"addr":24576,"words":[1]},{"addr":24584,"words":[2]},{"addr":61439,"bytes":[3]},{"addr":"tabelle + 16","words":[4]}]"#),
        "{j}"
    );
    assert!(
        j.contains(r#""expect":[{"addr":61436,"words":[5]},{"addr":"ZIEL - 2","bytes":[6]},{"addr":32768,"words":[7]}]"#),
        "{j}"
    );
    // Out of range or malformed → lvl.bad_value on that line.
    for bad in [
        "set STACK_TOP + 0x1000 = 1",
        "set INPUT - 0x6001 = 1",
        "set tabelle + = 1",
        "set tabelle * 2 = 1",
        "expect 0x4000 + 4 = 1",
        "setb tabelle + -1 = 1",
    ] {
        let src = level("", &format!("[test]\nname = a\n{bad}\n"));
        let d = parse(&src).expect_err(bad);
        assert_eq!(d.len(), 1, "{bad}: {d:?}");
        assert_eq!(d[0].key, "lvl.bad_value", "{bad}");
    }
}

#[test]
fn every_test_key() {
    let src = level(
        "kind = debug\nbudget = 5000\nseed = 0x10\nallow = mov halt JE li\nheap_check = off\nmobile = yes\n",
        "[test]\nname = a ; comment\nhidden = yes\nin = -1 0x10 0b11\nin_text = \"x;y\\n\"\n\
         reg r1 = -2\nset 0x4000 = 1 -1\nsetb DATA = \"ab\"\nsetb 0x4010 = 1 255 -128\n\
         screen_init = bild\nout =\nout_text = \"ok\"\nexpect 0x4000 = 7\nexpectb 0x4004 = 9\n\
         expect_reg sp = 0xF000\nexpect_screen = bild\nexpect_heap_empty = yes\nseed = 3\nbudget = 77\n\n\
         [test]\nname = f\ncall = fn\nreg r3 = 4\nexpect_screen = reference\n\n\
         [image.bild]\nF.\n\n.A\n\n",
    );
    let l = parse(&src).unwrap_or_else(|d| panic!("{d:?}"));
    assert_eq!(l.kind, LevelKind::Debug);
    assert_eq!((l.budget, l.seed, l.mobile), (5000, 16, true));
    assert_eq!(l.heap_check, HeapCheck::Off);
    assert_eq!(
        l.filter,
        MnemonicFilter::Allow(vec!["mov".into(), "halt".into(), "je".into(), "li".into()])
    );
    let t = &l.tests[0];
    assert!(t.hidden);
    assert_eq!(t.input, vec![-1, 16, 3]);
    assert_eq!(t.in_text, b"x;y\n");
    assert_eq!(t.regs, vec![(Reg::R1, (-2i32) as u32)]);
    assert_eq!(t.sets[0].addr, AddrRef::Num(0x4000));
    assert_eq!(t.sets[0].values, MemValues::Words(vec![1, u32::MAX]));
    assert_eq!(t.sets[1].addr, AddrRef::Symbol("DATA".into()));
    assert_eq!(t.sets[1].values, MemValues::Bytes(b"ab".to_vec()));
    assert_eq!(t.sets[2].values, MemValues::Bytes(vec![1, 255, 128]));
    assert_eq!(t.screen_init.as_deref(), Some("bild"));
    assert_eq!(t.out, Some(vec![]));
    assert_eq!(t.out_text.as_deref(), Some(&b"ok"[..]));
    assert_eq!(t.expects.len(), 2);
    assert_eq!(t.expect_regs, vec![(Reg::Sp, 0xF000)]);
    assert_eq!(t.expect_screen, Some(ScreenExpect::Image("bild".into())));
    assert!(t.expect_heap_empty);
    assert_eq!((t.seed, t.budget), (Some(3), Some(77)));
    assert_eq!(l.tests[1].call.as_deref(), Some("fn"));
    assert_eq!(l.tests[1].expect_screen, Some(ScreenExpect::Reference));
    let img = l.image("bild").unwrap();
    assert_eq!(img.pixels[0], 15);
    assert_eq!(img.pixels[2 * 64 + 1], 10);
    assert_eq!(img.pixels.iter().filter(|p| **p != 0).count(), 2);
}

#[test]
fn every_lvl_error_with_its_line() {
    let t = "[test]\nname = a\n";
    // unknown_section (in a key-value section), but plain content in text sections.
    assert_eq!(keys(&level("[foo]", t)), vec![("lvl.unknown_section", 3)]);
    assert!(
        keys("[level]\nid = t\n[text.de]\n[foo]\n[x](handbuch:stack)\n[test]\nname = a\n")
            .is_empty()
    );
    assert_eq!(
        keys(&level("colour = red", t)),
        vec![("lvl.unknown_key", 3)]
    );
    assert_eq!(
        keys(&format!("stray\n{}", level("", t))),
        vec![("lvl.unknown_key", 1)]
    );
    assert_eq!(
        keys(&level("", "[test]\nhidden = yes\nbogus = 1\n")),
        vec![("lvl.missing_key", 10), ("lvl.unknown_key", 12)]
    );
    assert_eq!(
        keys("[level]\nversion = 1\n[test]\nname = a\n"),
        vec![("lvl.missing_key", 1)]
    );
    assert_eq!(keys("[test]\nname = a\n"), vec![("lvl.missing_key", 1)]);
    for (extra, line) in [
        ("chapter = 8", 3),
        ("version = 2", 3),
        ("order = 0", 3),
        ("kind = boss", 3),
        ("budget = 10000001", 3),
        ("seed = -1", 3),
        ("mobile = maybe", 3),
        ("heap_check = loose", 3),
        ("allow = mov frobnicate", 3),
        ("title.de =", 3),
        ("id = x\n", 3),
        ("kind", 3),
    ] {
        assert_eq!(
            keys(&level(extra, t)),
            vec![("lvl.bad_value", line)],
            "{extra}"
        );
    }
    for bad in [
        "in = 1 x",
        "in = 2147483648",
        "in_text = abc",
        "in_text = \"\\q\"",
        "reg r9 = 1",
        "reg r1 = 4294967296",
        "set 0x10000 = 1",
        "set 0x4000 =",
        "setb 0x4000 = 256",
        "expect 1x = 1",
        "expect_reg = 5",
        "hidden = ja",
        "name = zwei worte",
        "screen_init = fehlt",
        "expect_screen = fehlt",
        "call = 1abc",
        "budget = 0",
        "name = b",
        "out = 1 2 x",
        "expect_heap_empty = 1",
    ] {
        let src = level("", &format!("[test]\nname = a\n{bad}\n"));
        assert_eq!(keys(&src), vec![("lvl.bad_value", 12)], "{bad}");
    }
    // reg r4 is fine in a normal test but not in a function test.
    assert!(keys(&level("", "[test]\nname = a\nreg r4 = 1\n")).is_empty());
    assert_eq!(
        keys(&level("", "[test]\nname = a\nreg r4 = 1\ncall = f\n")),
        vec![("lvl.bad_value", 12)]
    );
    // expect_screen = reference needs a [reference].
    assert_eq!(
        keys("[level]\nid = t\n[test]\nname = a\nexpect_screen = reference\n"),
        vec![("lvl.bad_value", 5)]
    );
    assert_eq!(
        keys(&format!("{}[level]\n", level("", t))),
        vec![("lvl.duplicate_section", 12)]
    );
    assert_eq!(
        keys(&format!("{}[start]\n", level("", t))),
        vec![("lvl.duplicate_section", 12)]
    );
    assert_eq!(
        keys(&format!("{}[image.a]\n1\n[image.a]\n2\n", level("", t))),
        vec![("lvl.duplicate_section", 14)]
    );
    assert_eq!(keys(&level("", "")), vec![("lvl.no_tests", 1)]);
    assert_eq!(
        keys(&level("allow = mov\nforbid = add", t)),
        vec![("lvl.allow_and_forbid", 4)]
    );
    // Images: invalid character, too many columns, too many rows.
    let img = |rows: &str| format!("{}[image.x]\n{rows}", level("", t));
    assert_eq!(keys(&img("12\n1G\n")), vec![("lvl.bad_image", 14)]);
    assert_eq!(keys(&img("ab\n")), vec![("lvl.bad_image", 13)]);
    assert_eq!(
        keys(&img(&format!("{}\n", "1".repeat(65)))),
        vec![("lvl.bad_image", 13)]
    );
    assert_eq!(keys(&img(&"1\n".repeat(65))), vec![("lvl.bad_image", 77)]);
    assert!(keys(&img(&format!("{}\n\n\n", "1\n".repeat(64)))).is_empty());
    // Hints: exactly three non-empty parts.
    let hints = |body: &str| format!("{}[hint.de]\n{body}\n", level("", t));
    assert!(keys(&hints("a\n---\nb\n---\nc")).is_empty());
    assert_eq!(keys(&hints("a\n---\nb")), vec![("lvl.bad_value", 12)]);
    assert_eq!(keys(&hints("a\n---\n---\nc")), vec![("lvl.bad_value", 12)]);
}

#[test]
fn diagnostic_args_and_columns() {
    let src = level("", "[test]\nname = a\nin = 1 x\n");
    let d = parse(&src).unwrap_err();
    assert_eq!(d[0].file, 2);
    assert_eq!((d[0].line, d[0].col, d[0].len), (12, 6, 3));
    assert_eq!(d[0].args, vec![("key", "in".into())]);
    let d = parse(&format!(
        "{}[image.bild]\nZ\n",
        level("", "[test]\nname = a\n")
    ))
    .unwrap_err();
    assert_eq!(
        d[0].args,
        vec![("name", "bild".into()), ("line", 1u32.into())]
    );
}

#[test]
fn crlf_and_bom_are_accepted() {
    let src = format!("\u{FEFF}{}", SAMPLE.replace('\n', "\r\n"));
    let l = parse(&src).unwrap();
    assert_eq!(l.tests.len(), 15);
    assert!(!l.section("start").unwrap().contains('\r'));
}

#[test]
fn hint_split() {
    assert_eq!(split_hints("a\n---\n\nb\n---  \nc\n"), vec!["a", "b", "c"]);
    assert_eq!(split_hints("only"), vec!["only"]);
}

/// Spec test 7 (level parser): corrupted level files never panic in the parser, the packer or
/// the runner.
#[test]
fn corrupted_levels_do_not_panic() {
    let mut rng = SplitMix64::new(0x1E7E1);
    let lines: Vec<&str> = SAMPLE.lines().collect();
    let tokens = [
        "[",
        "]",
        "=",
        ";",
        "\"",
        "\\",
        "---",
        "[test]",
        "[level]",
        "[image.x]",
        "[reference]",
        "reg",
        "set",
        "expect",
        "0x",
        "-",
        "ä",
        "\u{1F600}",
        "\t",
        "call = ",
        "in = ",
    ];
    for _ in 0..2000 {
        let mut ls: Vec<String> = lines.iter().map(|s| s.to_string()).collect();
        for _ in 0..1 + rng.below(6) {
            let i = rng.below(ls.len() as u64) as usize;
            match rng.below(5) {
                0 => {
                    ls.remove(i);
                }
                1 => {
                    let t = tokens[rng.below(tokens.len() as u64) as usize];
                    let line = &mut ls[i];
                    let at = line
                        .char_indices()
                        .map(|(k, _)| k)
                        .nth(rng.below(line.chars().count() as u64 + 1) as usize)
                        .unwrap_or(line.len());
                    line.insert_str(at, t);
                }
                2 => {
                    let j = rng.below(ls.len() as u64) as usize;
                    ls.swap(i, j);
                }
                3 => ls[i] = tokens[rng.below(tokens.len() as u64) as usize].to_string(),
                _ => {
                    let dup = ls[i].clone();
                    ls.insert(i, dup);
                }
            }
        }
        let text = ls.join("\n");
        let _ = pack_source(&text, 1, 2, &[("t".into(), vec![3; 4096])]);
        if let Ok(l) = parse(&text)
            && let Ok(p) = assemble_level(&l, "  halt\n")
        {
            let r = run_tests_with(&l, &p, None);
            let _ = r.to_json(&l);
            let _ = l.info_json();
        }
    }
    for _ in 0..2000 {
        let n = rng.below(300) as usize;
        let bytes: Vec<u8> = (0..n).map(|_| rng.next_u32() as u8).collect();
        let _ = parse(&String::from_utf8_lossy(&bytes));
    }
}
