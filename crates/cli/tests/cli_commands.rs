//! End-to-end tests of the `stapelwerk` binary via `std::process::Command`.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const SAMPLE: &str = include_str!("../../../levels/2-2-bis-zur-null.lvl");

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_stapelwerk"))
}

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A fresh temporary folder (removed again by [`TempDir::drop`]).
struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let p = std::env::temp_dir().join(format!("stapelwerk-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        TempDir(p)
    }

    fn file(&self, name: &str, content: &str) -> PathBuf {
        let p = self.0.join(name);
        std::fs::write(&p, content).unwrap();
        p
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn text(o: &Output) -> (i32, String, String) {
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).into_owned(),
        String::from_utf8_lossy(&o.stderr).into_owned(),
    )
}

const SUM: &str = "        mov r1, 0\nschleife:\n        sys READ_INT\n        cmp r0, 0\n        jz fertig\n        add r1, r0\n        jmp schleife\nfertig:\n        mov r0, r1\n        sys WRITE_INT\n        halt\n";

#[test]
fn run_prints_output_and_status() {
    let d = TempDir::new("run");
    let f = d.file("sum.s", SUM);
    let (code, out, err) = text(
        &bin()
            .arg("run")
            .arg(&f)
            .args(["--in", "3 4 5 0"])
            .output()
            .unwrap(),
    );
    assert_eq!(code, 0, "{err}");
    assert_eq!(out, "12\n");
    assert_eq!(err, "-- halted after 22 steps, 37 cycles\n");

    let (code, out, _) = text(
        &bin()
            .arg("run")
            .arg(&f)
            .args(["--in", "3 0", "--trace"])
            .output()
            .unwrap(),
    );
    assert_eq!(code, 0);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 13, "{out}");
    assert_eq!(lines[1], "     2  0x0104  sys READ_INT              r0=3");
    assert_eq!(lines[3], "     4  0x010C  jz fertig");
    assert_eq!(
        lines[7],
        "     8  0x0108  cmp r0, 0                 flags=Z---"
    );
    assert_eq!(lines[10], "    11  0x011C  sys WRITE_INT             out 3");
    assert_eq!(lines[12], "3");

    // A fault: exit code 1 and a readable message with the line.
    let (code, _, err) = text(
        &bin()
            .arg("run")
            .arg(&f)
            .args(["--in", "3"])
            .output()
            .unwrap(),
    );
    assert_eq!(code, 1);
    assert!(err.contains("fault at 0x0104 (line 3)"), "{err}");
    assert!(err.contains("[fault.input_end]"), "{err}");

    // Text input and output.
    let echo = d.file(
        "echo.s",
        "  sys READ_CHAR\n  sys WRITE_CHAR\n  sys READ_CHAR\n  sys WRITE_CHAR\n  halt\n",
    );
    let (code, out, _) = text(
        &bin()
            .arg("run")
            .arg(&echo)
            .args(["--in-text", "ok"])
            .output()
            .unwrap(),
    );
    assert_eq!((code, out.as_str()), (0, "ok\n"));

    // An endless loop names the hottest line.
    let lp = d.file(
        "loop.s",
        "  mov r0, 1\nschleife:\n  add r1, r0\n  jmp schleife\n",
    );
    let (code, _, err) = text(&bin().arg("run").arg(&lp).output().unwrap());
    assert_eq!(code, 1);
    assert!(
        err.contains("most executed: line 3") || err.contains("most executed: line 4"),
        "{err}"
    );
}

#[test]
fn asm_and_dis() {
    let d = TempDir::new("asm");
    let f = d.file("sum.s", SUM);
    let img = d.0.join("sum.img");
    let (code, out, _) = text(
        &bin()
            .arg("asm")
            .arg(&f)
            .arg("-o")
            .arg(&img)
            .output()
            .unwrap(),
    );
    assert_eq!(code, 0);
    assert!(out.contains("11 code words (size 9)"), "{out}");
    let (code, out, _) = text(&bin().arg("dis").arg(&img).output().unwrap());
    assert_eq!(code, 0);
    assert!(
        out.contains("    0x0104  04000001  sys READ_INT\n"),
        "{out}"
    );
    let (code, out, _) = text(&bin().arg("dis").arg(&f).output().unwrap());
    assert_eq!(code, 0);
    assert!(
        out.contains("schleife:\n    0x0104  04000001  sys READ_INT  ; line 3\n"),
        "{out}"
    );
    assert!(out.contains("jz fertig"), "{out}");

    let bad = d.file("bad.s", "  ad r1, r2\n  mov r1, 99999\n");
    let (code, _, err) = text(&bin().arg("asm").arg(&bad).output().unwrap());
    assert_eq!(code, 1);
    assert!(
        err.contains(
            "bad.s:1:3: unknown mnemonic 'ad' (did you mean 'add'?) [asm.unknown_mnemonic]"
        ),
        "{err}"
    );
    assert!(err.contains("(use 'li' for large constants)"), "{err}");
    let junk = d.file("junk.img", "nope");
    assert_eq!(
        bin().arg("dis").arg(&junk).output().unwrap().status.code(),
        Some(1)
    );
}

#[test]
fn check_levels_passes_on_the_repository() {
    let o = bin()
        .args(["check-levels", "--strict", "--mutate"])
        .arg(repo().join("levels"))
        .output()
        .unwrap();
    let (code, out, err) = text(&o);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("ok    "), "{out}");
    assert!(out.contains("all good"), "{out}");
}

/// Writes the sample level (under its own file name) plus a manifest into a temp folder, with
/// `edit` applied to the level text.
fn broken(name: &str, edit: impl Fn(&str) -> String) -> (TempDir, String) {
    let d = TempDir::new(name);
    d.file(
        "manifest.txt",
        "# chapter 2: Sprünge | Jumps\n2-2-bis-zur-null.lvl\n",
    );
    d.file("2-2-bis-zur-null.lvl", &edit(SAMPLE));
    let o = bin()
        .args(["check-levels", "--strict", "--mutate"])
        .arg(&d.0)
        .output()
        .unwrap();
    let (code, out, err) = text(&o);
    assert_eq!(code, 1, "{name}: {out}{err}");
    (d, out)
}

#[test]
fn check_levels_finds_broken_levels() {
    // Wrong expectation: the reference fails a hidden test.
    let (_d, out) = broken("wrong", |s| s.replace("out = 2500000", "out = 2500001"));
    assert!(
        out.contains("the reference fails test 'verdeckt-6'"),
        "{out}"
    );
    // A weak test set: without the hidden tests and with only one-number inputs, deleting
    // `jmp schleife` is not noticed.
    let (_d, out) = broken("weak", |s| {
        let cut = s.find("[test]").unwrap();
        format!(
            "{}[test]\nname = a\nin = 5 0\nout = 5\n\n[test]\nname = b\nin = 0\nout = 0\n",
            &s[..cut]
        )
    });
    assert!(out.contains("tests too weak"), "{out}");
    assert!(out.contains("`jmp schleife`"), "{out}");
    // Missing English hints, duplicate test names, parse errors.
    let (_d, out) = broken("lang", |s| {
        let a = s.find("[hint.en]").unwrap();
        let b = s.find("[learned.de]").unwrap();
        format!("{}{}", &s[..a], &s[b..]).replace("name = negativ", "name = grundfall")
    });
    assert!(out.contains("[hint.en] is missing"), "{out}");
    assert!(
        out.contains("test name 'grundfall' is used more than once"),
        "{out}"
    );
    let (_d, out) = broken("parse", |s| s.replace("in = -7 10 0", "in = -7 zehn 0"));
    assert!(
        out.contains(":32:6: invalid value for 'in' [lvl.bad_value]")
            || out.contains("[lvl.bad_value]"),
        "{out}"
    );
    // Start code that already solves the level.
    let (_d, out) = broken("start", |s| {
        let reference = &s[s.find("[reference]\n").unwrap() + 12..s.find("\n[test]").unwrap()];
        s.replacen(
            "; Lies Zahlen bis zur 0 und gib ihre Summe aus.\n\n        halt\n",
            reference,
            1,
        )
    });
    assert!(out.contains("the start code passes every test"), "{out}");
    // Too many mutate:skip markers and a set outside writable memory.
    let (_d, out) = broken("skip", |s| {
        s.replace(
            "        cmp r0, 0          ; Endmarke?",
            "        cmp r0, 0          ; mutate:skip",
        )
        .replace(
            "        add r1, r0         ; Summe += Zahl",
            "        add r1, r0         ; mutate:skip",
        )
        .replace("name = grundfall\n", "name = grundfall\nset 0x0200 = 1\n")
    });
    assert!(out.contains("3 lines carry '; mutate:skip'"), "{out}");
    assert!(
        out.contains("set 0x0200..0x0203 must lie inside one of the segments"),
        "{out}"
    );
}

#[test]
fn pack_levels_writes_index_and_packed_files() {
    let d = TempDir::new("pack");
    let (code, out, err) = text(
        &bin()
            .arg("pack-levels")
            .arg(repo().join("levels"))
            .arg(&d.0)
            .output()
            .unwrap(),
    );
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("packed 1 level(s)"));
    let index = std::fs::read_to_string(d.0.join("index.json")).unwrap();
    assert!(index.contains(r#"{"id":"2-2-bis-zur-null","chapter":2,"order":2,"kind":"normal","title":{"de":"Bis zur Null","en":"Until Zero"}"#), "{index}");
    assert!(
        index.contains(r#"{"chapter":7,"title":{"de":"Heap und Listen","en":"Heap & Lists"}}"#),
        "{index}"
    );
    let packed = std::fs::read_to_string(d.0.join("2-2-bis-zur-null.lvl")).unwrap();
    assert!(!packed.contains("[reference]"));
    assert!(packed.contains("ref_size = 9\n"));
    // A packed file still checks (without --strict: no reference needed).
    let (code, out, _) = text(
        &bin()
            .arg("check-levels")
            .arg(d.0.join("2-2-bis-zur-null.lvl"))
            .output()
            .unwrap(),
    );
    assert_eq!(code, 0, "{out}");
}

#[test]
fn fuzz_gen_writes_programs_and_digests() {
    let d = TempDir::new("fuzz");
    let out_dir = d.0.join("f");
    let run = || {
        bin()
            .args(["fuzz-gen", "--seed", "7", "--count", "5", "--out"])
            .arg(&out_dir)
            .output()
            .unwrap()
    };
    let (code, _, err) = text(&run());
    assert_eq!(code, 0, "{err}");
    let digests = std::fs::read_to_string(out_dir.join("digests.txt")).unwrap();
    let lines: Vec<&str> = digests.lines().filter(|l| !l.starts_with('#')).collect();
    assert_eq!(lines.len(), 5);
    assert!(lines[0].starts_with("fuzz-00000.s "), "{digests}");
    assert!(lines.iter().all(|l| l.contains(" 0:")), "{digests}");
    // Every program assembles with the CLI as well, and generation is deterministic.
    let (code, _, err) = text(
        &bin()
            .arg("asm")
            .arg(out_dir.join("fuzz-00004.s"))
            .output()
            .unwrap(),
    );
    assert_eq!(code, 0, "{err}");
    let (code, _, _) = text(&run());
    assert_eq!(code, 0);
    assert_eq!(
        std::fs::read_to_string(out_dir.join("digests.txt")).unwrap(),
        digests
    );
}
