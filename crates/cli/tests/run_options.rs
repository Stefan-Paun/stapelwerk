//! `stapelwerk run` with `--level`/`--test`, `--budget`, `--heap-check` and `--library`, run
//! through the real binary against the repository's levels.

use std::path::{Path, PathBuf};
use std::process::Command;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn level_path(id: &str) -> PathBuf {
    repo().join("levels").join(format!("{id}.lvl"))
}

/// Raw content of section `[name]` of a level file (up to the next section header).
fn section(id: &str, name: &str) -> String {
    let text = std::fs::read_to_string(level_path(id)).unwrap();
    let header = format!("[{name}]");
    let mut out = String::new();
    let mut inside = false;
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with('[') && t.ends_with(']') && !t.contains(' ') && !t.contains(',') {
            inside = t == header;
            continue;
        }
        if inside {
            out.push_str(line);
            out.push('\n');
        }
    }
    assert!(!out.is_empty(), "{id}: no [{name}]");
    out
}

/// A fresh temporary folder, removed on drop.
struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let p = std::env::temp_dir().join(format!("stapelwerk-run-{name}-{}", std::process::id()));
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

/// Runs the binary; returns (exit code, stdout, stderr).
fn run(args: &[&std::ffi::OsStr]) -> (i32, String, String) {
    let o = Command::new(env!("CARGO_BIN_EXE_stapelwerk"))
        .args(args)
        .output()
        .unwrap();
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).into_owned(),
        String::from_utf8_lossy(&o.stderr).into_owned(),
    )
}

fn os<S: AsRef<std::ffi::OsStr> + ?Sized>(s: &S) -> &std::ffi::OsStr {
    s.as_ref()
}

/// `run <code> --level <id> [extra…]` with `code` written to a temp file.
fn run_level(d: &TempDir, id: &str, code: &str, extra: &[&str]) -> (i32, String, String) {
    let f = d.file(&format!("{id}.s"), code);
    let lvl = level_path(id);
    let mut args = vec![os("run"), f.as_os_str(), os("--level"), lvl.as_os_str()];
    args.extend(extra.iter().map(os));
    run(&args)
}

#[test]
fn endless_loop_start_code_reports_budget_with_line() {
    let d = TempDir::new("budget");
    let id = "2-4-endlosschleife";
    let start = section(id, "start");
    let (code, out, err) = run_level(&d, id, &start, &["--test", "ungerade"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(out.starts_with("7\n5\n3\n1\n-1\n"), "{out}");
    assert!(
        err.contains("-- test #1 'ungerade' of 2-4-endlosschleife"),
        "{err}"
    );
    assert!(
        err.contains("cycle budget used up after 10000 cycles (most executed: line 8)"),
        "{err}"
    );
    assert!(
        err.contains("-- check: the run ended with status BUDGET_EXCEEDED: cycle budget used up after 10000 cycles, line 8 ran "),
        "{err}"
    );
    assert!(err.contains("[check.not_halted]"), "{err}");
    assert!(err.ends_with("-- FAILED\n"), "{err}");

    // The same test by index; an even start passes.
    let (code, _, err) = run_level(&d, id, &start, &["--test", "0"]);
    assert_eq!(code, 0, "{err}");
    assert!(err.contains("-- halted after"), "{err}");
    assert!(err.ends_with("-- PASSED\n"), "{err}");

    // All tests: a readable summary, exit 1.
    let (code, out, _) = run_level(&d, id, &start, &[]);
    assert_eq!(code, 1, "{out}");
    assert!(
        out.starts_with(
            "level 2-4-endlosschleife: 16 tests (12 hidden), budget 10000, heap check strict\n"
        ),
        "{out}"
    );
    assert!(out.contains("ok    #0 gerade"), "{out}");
    assert!(out.contains("FAIL  #1 ungerade"), "{out}");
    assert!(out.contains("#5 verdeckt-2 (hidden)"), "{out}");
    assert!(out.contains("line 8 ran"), "{out}");
    assert!(out.contains("10/16 passed; cycles avg"), "{out}");
    assert!(out.contains("; reference 429 cycles avg, size 7"), "{out}");

    // The reference passes everything; with a tiny --budget it no longer does.
    let reference = section(id, "reference");
    let (code, out, _) = run_level(&d, id, &reference, &[]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("16/16 passed"), "{out}");
    let (code, out, _) = run_level(&d, id, &reference, &["--budget", "20"]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("budget 20,"), "{out}");
    assert!(out.contains("BUDGET_EXCEEDED"), "{out}");
}

#[test]
fn after_free_level_faults_like_the_game() {
    let d = TempDir::new("free");
    let id = "7-2-nach-dem-freigeben";
    let start = section(id, "start");
    // heap_check = off in the level: reading freed memory yields 0xDDDDDDDD, the crash comes
    // later as an out-of-range access, exactly as in the game.
    let (code, out, err) = run_level(&d, id, &start, &["--test", "beispiel"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(err.contains("[fault.out_of_range]"), "{err}");
    assert!(err.contains("(line 24)"), "{err}");
    assert!(!err.contains("use_after_free"), "{err}");
    assert!(err.ends_with("-- FAILED\n"), "{err}");
    // --heap-check strict overrides the level and finds the real cause.
    let (code, _, err) = run_level(
        &d,
        id,
        &start,
        &["--test", "beispiel", "--heap-check", "strict"],
    );
    assert_eq!(code, 1);
    assert!(err.contains("[fault.use_after_free]"), "{err}");
    // The reference passes all tests.
    let (code, out, _) = run_level(&d, id, &section(id, "reference"), &[]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("15/15 passed"), "{out}");
}

#[test]
fn function_tests_with_trace() {
    let d = TempDir::new("trace");
    let id = "6-1-fakultaet";
    let reference = section(id, "reference");
    let (code, plain_out, plain_err) = run_level(&d, id, &reference, &["--test", "fak-5"]);
    assert_eq!(code, 0, "{plain_err}");
    let (code, out, err) = run_level(&d, id, &reference, &["--test", "fak-5", "--trace"]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(err, plain_err, "trace and plain run end identically");
    let trace: Vec<&str> = out.lines().collect();
    assert!(trace.len() > 5, "{out}");
    assert!(trace[0].starts_with("     1  0x"), "{out}");
    assert!(out.ends_with(&plain_out), "{out}");
    // Tracing every test gives the same summary as the plain run.
    let summary = |s: &str| -> Vec<String> {
        s.lines()
            .filter(|l| {
                l.starts_with("ok    ") || l.starts_with("FAIL  ") || l.contains(" passed;")
            })
            .map(str::to_string)
            .collect()
    };
    let (_, all_plain, _) = run_level(&d, id, &reference, &[]);
    let (code, all_traced, _) = run_level(&d, id, &reference, &["--trace"]);
    assert_eq!(code, 0);
    assert!(all_traced.contains("== test #0 'fak-5'"), "{all_traced}");
    assert_eq!(summary(&all_plain), summary(&all_traced));
}

#[test]
fn level_run_errors() {
    let d = TempDir::new("errors");
    let id = "2-2-bis-zur-null";
    // Unknown test: usage error listing the tests.
    let (code, _, err) = run_level(&d, id, "  halt\n", &["--test", "gibtsnicht"]);
    assert_eq!(code, 2);
    assert!(err.contains("0:grundfall"), "{err}");
    let (code, _, _) = run_level(&d, id, "  halt\n", &["--test", "99"]);
    assert_eq!(code, 2);
    // Assembler errors name the player's file and line.
    let (code, _, err) = run_level(&d, id, "  halt\n  ad r1, r2\n", &[]);
    assert_eq!(code, 1);
    assert!(err.contains(".s:2:3: unknown mnemonic 'ad'"), "{err}");
    // Options that make no sense with --level, or without it.
    for extra in [
        &["--in", "1"][..],
        &["--in-text", "x"],
        &["--library", "x.s"],
    ] {
        let (code, _, err) = run_level(&d, id, "  halt\n", extra);
        assert_eq!(code, 2, "{extra:?}");
        assert!(err.contains("cannot be combined with --level"), "{err}");
    }
    let f = d.file("h.s", "  halt\n");
    let (code, _, err) = run(&[os("run"), f.as_os_str(), os("--test"), os("1")]);
    assert_eq!(code, 2);
    assert!(err.contains("--test needs --level"), "{err}");
    for (opt, v) in [
        ("--budget", "0"),
        ("--budget", "10000001"),
        ("--heap-check", "an"),
    ] {
        let (code, _, _) = run(&[os("run"), f.as_os_str(), os(opt), os(v)]);
        assert_eq!(code, 2, "{opt} {v}");
    }
}

#[test]
fn addresses_with_constants_in_a_level() {
    let d = TempDir::new("addr");
    let lvl = d.file(
        "a.lvl",
        "[level]\nid = a\nbudget = 1000\n\n[library]\n.data\nwerte: .word 0, 0, 0\n\n\
         [test]\nname = t\nset INPUT + 4 = 21\nexpect werte + 8 = 42\nexpectb STACK_TOP - 4 = 0\n",
    );
    let prog = d.file(
        "p.s",
        "  li r1, INPUT\n  ld r0, [r1 + 4]\n  add r0, r0\n  li r2, werte\n  st r0, [r2 + 8]\n  halt\n",
    );
    let (code, _, err) = run(&[
        os("run"),
        prog.as_os_str(),
        os("--level"),
        lvl.as_os_str(),
        os("--test"),
        os("t"),
    ]);
    assert_eq!(code, 0, "{err}");
    let wrong = d.file(
        "w.s",
        "  li r2, werte\n  mov r0, 1\n  st r0, [r2 + 8]\n  halt\n",
    );
    let (code, _, err) = run(&[
        os("run"),
        wrong.as_os_str(),
        os("--level"),
        lvl.as_os_str(),
        os("--test"),
        os("0"),
    ]);
    assert_eq!(code, 1);
    assert!(
        err.contains("-- check: memory at 0x4008: expected 42, got 1 [check.mem_mismatch]"),
        "{err}"
    );
}

#[test]
fn plain_run_with_library_budget_and_heap_check() {
    let d = TempDir::new("plain");
    let lib = d.file("lib.s", "doppelt:\n  add r0, r0\n  ret\n");
    let main = d.file(
        "main.s",
        "  sys READ_INT\n  call doppelt\n  sys WRITE_INT\n  halt\n",
    );
    let (code, out, err) = run(&[
        os("run"),
        main.as_os_str(),
        os("--library"),
        lib.as_os_str(),
        os("--in"),
        os("21"),
    ]);
    assert_eq!((code, out.as_str()), (0, "42\n"), "{err}");
    // Library errors name the library file.
    let bad_lib = d.file("bad.s", "doppelt:\n  addd r0, r0\n  ret\n");
    let (code, _, err) = run(&[
        os("run"),
        main.as_os_str(),
        os("--library"),
        bad_lib.as_os_str(),
    ]);
    assert_eq!(code, 1);
    assert!(err.contains("bad.s:2:3: unknown mnemonic 'addd'"), "{err}");
    assert!(err.contains("[library]"), "{err}");

    // --budget
    let lp = d.file(
        "loop.s",
        "  mov r0, 1\nschleife:\n  add r1, r0\n  jmp schleife\n",
    );
    let (code, _, err) = run(&[os("run"), lp.as_os_str(), os("--budget"), os("50")]);
    assert_eq!(code, 1);
    assert!(
        err.contains("cycle budget used up after 50 cycles"),
        "{err}"
    );
    assert!(err.contains("ran "), "{err}");

    // --heap-check: reading freed memory faults only with the check on.
    let uaf = d.file(
        "uaf.s",
        "  mov r0, 8\n  sys ALLOC\n  mov r1, r0\n  sys FREE\n  ld r0, [r1]\n  sys WRITE_INT\n  halt\n",
    );
    let (code, _, err) = run(&[os("run"), uaf.as_os_str()]);
    assert_eq!(code, 1);
    assert!(err.contains("[fault.use_after_free]"), "{err}");
    let (code, out, err) = run(&[os("run"), uaf.as_os_str(), os("--heap-check"), os("off")]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(out, format!("{}\n", 0xDDDD_DDDDu32 as i32));
}
