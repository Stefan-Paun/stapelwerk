//! Test runner: every kind of test passes and fails with the right `check.*` key, function tests,
//! heap leaks, budget, JSON shape and packing round trip.

use stapelwerk_core::diag::Arg;
use stapelwerk_core::level::{Level, parse};
use stapelwerk_core::machine::Status;
use stapelwerk_core::runner::{
    TestReport, TestResult, assemble_level, function_test_regs, pack_level, reference_screen,
    run_test, run_tests,
};

const SAMPLE: &str = include_str!("../../../levels/2-2-bis-zur-null.lvl");

/// A level with a small library (`WERT` at 0x4000) and the given extra sections.
fn level(body: &str) -> Level {
    let src = format!(
        "[level]\nid = r\nbudget = 2000\nseed = 5\n\n[library]\n.data\nWERT:   .word 0\n\n{body}"
    );
    parse(&src).unwrap_or_else(|d| panic!("{d:?}\n{src}"))
}

fn report(l: &Level, code: &str) -> TestReport {
    let p = assemble_level(l, code).unwrap_or_else(|d| panic!("{d:?}\n{code}"));
    run_tests(l, &p)
}

/// Runs a one-test level and returns its result.
fn one(test: &str, code: &str) -> TestResult {
    let l = level(&format!("[test]\nname = t\n{test}\n"));
    let r = report(&l, code);
    r.tests.into_iter().next().unwrap()
}

fn mismatch(r: &TestResult) -> (&'static str, Vec<(&'static str, Arg)>) {
    let m = r
        .mismatch
        .as_ref()
        .unwrap_or_else(|| panic!("no mismatch: {r:?}"));
    (m.key, m.args.clone())
}

fn int(v: i64) -> Arg {
    Arg::Int(v)
}

const PRINT_SUM: &str =
    "  sys READ_INT\n  mov r1, r0\n  sys READ_INT\n  add r0, r1\n  sys WRITE_INT\n  halt\n";

#[test]
fn io_tests() {
    let r = one("in = 3 4\nout = 7", PRINT_SUM);
    assert!(r.passed, "{r:?}");
    assert_eq!(r.status, Status::Halted);
    assert!(r.cycles > 0 && r.steps == 6);
    let r = one("in = 3 4\nout = 8", PRINT_SUM);
    assert_eq!(
        mismatch(&r),
        (
            "check.out_mismatch",
            vec![("index", int(0)), ("expected", int(8)), ("actual", int(7))]
        )
    );
    let r = one("in = 3 4\nout = 7 1", PRINT_SUM);
    assert_eq!(
        mismatch(&r),
        (
            "check.out_missing",
            vec![("index", int(1)), ("expected", int(1))]
        )
    );
    let r = one("in = 3 4\nout =", PRINT_SUM);
    assert_eq!(
        mismatch(&r),
        (
            "check.out_extra",
            vec![("index", int(0)), ("actual", int(7))]
        )
    );
    // No `out` line: output not checked.
    assert!(one("in = 3 4", PRINT_SUM).passed);
    // Running out of input is a fault, not a mismatch.
    let r = one("in = 3\nout = 3", PRINT_SUM);
    assert!(!r.passed);
    assert_eq!(r.status, Status::Fault);
    assert_eq!(r.fault.map(|f| f.key()), Some("fault.input_end"));
    assert!(r.mismatch.is_none());
}

#[test]
fn text_tests() {
    let code = "  li r0, 0x4100\n  sys WRITE_STR\n  halt\n";
    assert!(one("setb 0x4100 = \"hi\"\nout_text = \"hi\"", code).passed);
    let r = one("setb 0x4100 = \"hi\"\nout_text = \"ho\"", code);
    assert_eq!(
        mismatch(&r),
        (
            "check.text_mismatch",
            vec![
                ("offset", int(1)),
                ("expected", int(111)),
                ("actual", int(105))
            ]
        )
    );
    let r = one("setb 0x4100 = \"hi\"\nout_text = \"h\"", code);
    assert_eq!(
        mismatch(&r),
        (
            "check.text_mismatch",
            vec![
                ("offset", int(1)),
                ("expected", int(-1)),
                ("actual", int(105))
            ]
        )
    );
    // Numbers and bytes are compared separately.
    let mixed = "  mov r0, 5\n  sys WRITE_INT\n  mov r0, 'A'\n  sys WRITE_CHAR\n  halt\n";
    assert!(one("out = 5\nout_text = \"A\"", mixed).passed);
    // in_text feeds READ_CHAR.
    let echo = "  sys READ_CHAR\n  sys WRITE_INT\n  sys READ_CHAR\n  sys WRITE_INT\n  halt\n";
    assert!(one("in_text = \"z\"\nout = 122 -1", echo).passed);
}

#[test]
fn memory_and_register_tests() {
    let store = "  li r1, WERT\n  mov r2, 42\n  st r2, [r1]\n  mov r3, 2\n  halt\n";
    assert!(one("expect WERT = 42\nexpectb 0x4000 = 42 0", store).passed);
    let r = one("expect WERT = -41", store);
    assert_eq!(
        mismatch(&r),
        (
            "check.mem_mismatch",
            vec![
                ("addr", int(0x4000)),
                ("expected", int(-41)),
                ("actual", int(42))
            ]
        )
    );
    let r = one("expectb 0x4001 = 1", store);
    assert_eq!(
        mismatch(&r),
        (
            "check.mem_mismatch",
            vec![
                ("addr", int(0x4001)),
                ("expected", int(1)),
                ("actual", int(0))
            ]
        )
    );
    assert!(one("expect_reg r3 = 2\nexpect_reg sp = 0xF000", store).passed);
    let r = one("expect_reg r3 = -3", store);
    assert_eq!(
        mismatch(&r),
        (
            "check.reg_mismatch",
            vec![
                ("reg", "r3".into()),
                ("expected", int(-3)),
                ("actual", int(2))
            ]
        )
    );
    // set (by symbol) and reg initialise the machine.
    let load =
        "  li r1, WERT\n  ld r0, [r1]\n  sys WRITE_INT\n  mov r0, r3\n  sys WRITE_INT\n  halt\n";
    assert!(one("set WERT = 77\nreg r3 = -9\nout = 77 -9", load).passed);
    // Addresses with built-in constants and `SYMBOL ± n`.
    let store2 = "  li r1, WERT\n  mov r2, 7\n  st r2, [r1 + 4]\n  li r1, INPUT\n  ld r0, [r1 + 8]\n  sys WRITE_INT\n  halt\n";
    let l = parse(
        "[level]\nid = r\n\n[library]\n.data\nWERT:   .word 0, 0, 0\n\n\
         [test]\nname = t\nset INPUT + 8 = 31\nout = 31\nexpect WERT + 4 = 7\nexpectb WERT+4 = 7 0\nexpect WERT + 8 = 0\n",
    )
    .unwrap();
    assert!(report(&l, store2).passed, "{:?}", report(&l, store2));
    let l = parse(
        "[level]\nid = r\n\n[library]\n.data\nWERT:   .word 0, 0, 0\n\n\
         [test]\nname = t\nexpect WERT + 4 = 8\n",
    )
    .unwrap();
    assert_eq!(
        report(&l, store2).tests[0]
            .mismatch
            .as_ref()
            .map(|m| m.args.clone()),
        Some(vec![
            ("addr", int(0x4004)),
            ("expected", int(8)),
            ("actual", int(7))
        ])
    );
    // Output is checked before memory.
    let r = one("out = 1\nexpect WERT = 1", store);
    assert_eq!(mismatch(&r).0, "check.out_missing");
}

#[test]
fn screen_tests() {
    let draw = "  li r1, SCREEN\n  mov r2, 0x1F\n  stb r2, [r1]\n  halt\n";
    let l = level(
        "[test]\nname = a\nexpect_screen = punkt\n\n[test]\nname = b\nexpect_screen = zwei\n\n[image.punkt]\nF\n\n[image.zwei]\nFA\n",
    );
    let r = report(&l, draw);
    assert!(r.tests[0].passed, "low 4 bits only: {:?}", r.tests[0]);
    assert_eq!(
        mismatch(&r.tests[1]),
        (
            "check.screen_mismatch",
            vec![
                ("x", int(1)),
                ("y", int(0)),
                ("expected", int(10)),
                ("actual", int(0)),
                ("count", int(1))
            ]
        )
    );
    // screen_init loads an image before the run.
    let read = "  li r1, SCREEN\n  ldb r0, [r1 + 65]\n  sys WRITE_INT\n  halt\n";
    let l = level("[test]\nname = a\nscreen_init = bild\nout = 7\n\n[image.bild]\n.\n.7\n");
    assert!(report(&l, read).passed);
}

#[test]
fn reference_screen_tests() {
    let l = level(
        "[reference]\n  li r1, SCREEN\n  mov r2, 4\n  stb r2, [r1 + 66]\n  halt\n\n\
         [test]\nname = bild\nexpect_screen = reference\n",
    );
    let screen = reference_screen(&l, 0).unwrap();
    assert_eq!(screen[66], 4);
    let ok = "  li r1, SCREEN\n  mov r2, 0x14\n  stb r2, [r1 + 66]\n  halt\n";
    assert!(report(&l, ok).passed);
    let p = assemble_level(&l, "  halt\n").unwrap();
    let r = run_test(&l, &p, 0);
    assert_eq!(
        mismatch(&r),
        (
            "check.screen_mismatch",
            vec![
                ("x", int(2)),
                ("y", int(1)),
                ("expected", int(4)),
                ("actual", int(0)),
                ("count", int(1))
            ]
        )
    );
}

#[test]
fn heap_tests() {
    let leak = "  mov r0, 8\n  sys ALLOC\n  halt\n";
    let r = one("expect_heap_empty = yes", leak);
    assert_eq!(
        mismatch(&r),
        (
            "check.heap_not_empty",
            vec![("count", int(1)), ("alloc_line", int(2))]
        )
    );
    assert!(one("", leak).passed);
    let clean = "  mov r0, 8\n  sys ALLOC\n  sys FREE\n  halt\n";
    assert!(one("expect_heap_empty = yes", clean).passed);
}

#[test]
fn budget_and_faults() {
    let r = one(
        "budget = 100",
        "  mov r1, 0\nschleife:\n  add r1, 1\n  jmp schleife\n",
    );
    assert_eq!(r.status, Status::BudgetExceeded);
    assert!(r.cycles >= 100);
    // The budget end names cycles, the most executed line of file 0 and its count.
    let (key, args) = mismatch(&r);
    assert_eq!(key, "check.not_halted");
    assert_eq!(args[0], ("status", "BUDGET_EXCEEDED".into()));
    assert_eq!(args[1], ("cycles", int(r.cycles as i64)));
    assert_eq!(args[2], ("hottest_line", int(3)));
    let Arg::Int(count) = args[3].1.clone() else {
        panic!("{args:?}")
    };
    assert_eq!(args[3].0, "hottest_count");
    assert!(count >= 30, "{count}");
    // JSON: status 5 and the extra arguments.
    let l = level("[test]\nname = t\nbudget = 100\n");
    let rep = report(&l, "  mov r1, 0\nschleife:\n  add r1, 1\n  jmp schleife\n");
    let j = rep.to_json(&l);
    assert!(
        j.contains(&format!(
            r#""passed":false,"status":5,"cycles":{},"steps":{},"fault":null,"mismatch":{{"key":"check.not_halted","args":{{"status":"BUDGET_EXCEEDED","cycles":{},"hottest_line":3,"hottest_count":{count}}}}}"#,
            r.cycles, r.steps, r.cycles
        )),
        "{j}"
    );
    // Other non-halted endings keep the plain form.
    let r = one("", "  sys READ_INT\n  halt\n");
    assert_eq!(r.status, Status::Fault);
    assert_eq!(r.mismatch, None);
    // The level budget applies without an override.
    let r = one("", "schleife:\n  jmp schleife\n");
    assert!(r.cycles >= 2000 && r.cycles < 2100);
    let r = one("", "  mov r1, 0\n  div r0, r1\n  halt\n");
    assert_eq!(r.status, Status::Fault);
    assert_eq!(
        r.fault.map(|f| (f.key(), f.pc)),
        Some(("fault.div_zero", 0x104))
    );
}

const SQUARE: &str = "  halt\nquadrat:\n  mul r0, r0, r0\n  ret\n";

#[test]
fn function_tests() {
    let r = one("call = quadrat\nreg r0 = 7\nexpect_reg r0 = 49", SQUARE);
    assert!(r.passed, "{r:?}");
    // `call` + `ret` = the cost of the function; the trap word is not executed.
    assert_eq!(r.steps, 2);
    let r = one("call = quadrat\nreg r0 = 7\nexpect_reg r0 = 48", SQUARE);
    assert_eq!(mismatch(&r).0, "check.reg_mismatch");

    // Destroying a callee-saved register.
    let clobber = "  halt\nquadrat:\n  mov r4, r0\n  mul r0, r4, r4\n  ret\n";
    let r = one("call = quadrat\nreg r0 = 7\nexpect_reg r0 = 49", clobber);
    let before = function_test_regs(5)[0];
    assert_eq!(
        mismatch(&r),
        (
            "check.abi_clobbered",
            vec![
                ("reg", "r4".into()),
                ("before", int(before as i64)),
                ("after", int(7))
            ]
        )
    );
    // Test seed overrides the level seed.
    let r = one("call = quadrat\nreg r0 = 7\nseed = 9", clobber);
    assert_eq!(
        mismatch(&r).1[1],
        ("before", int(function_test_regs(9)[0] as i64))
    );

    // Returning with an unbalanced stack.
    let sp = "  halt\nf:\n  pop r1\n  push r2\n  push r2\n  jmp r1\n";
    let r = one("call = f", sp);
    assert_eq!(
        mismatch(&r),
        (
            "check.abi_clobbered",
            vec![
                ("reg", "sp".into()),
                ("before", int(0xF000)),
                ("after", int(0xEFF8))
            ]
        )
    );
    // A function that never returns runs into the budget; a missing label is a bad jump target.
    let r = one("call = quadrat", "  halt\nquadrat:\n  jmp quadrat\n");
    assert_eq!(r.status, Status::BudgetExceeded);
    let r = one("call = fehlt", SQUARE);
    assert_eq!(r.fault.map(|f| f.key()), Some("fault.bad_jump_target"));
}

#[test]
fn report_json_and_hidden_inputs() {
    let l = level(
        "[reference]\n  sys READ_INT\n  sys WRITE_INT\n  halt\n\n\
         [test]\nname = sichtbar\nin = 1\nout = 1\n\n\
         [test]\nname = geheim-ok\nhidden = yes\nin = 2\nout = 2\n\n\
         [test]\nname = geheim-falsch\nhidden = yes\nin = 3\nin_text = \"a\"\nout = 4\nout_text = \"b\"\n",
    );
    let p = assemble_level(&l, "  sys READ_INT\n  sys WRITE_INT\n  halt\n").unwrap();
    let r = run_tests(&l, &p);
    assert!(!r.passed);
    let j = r.to_json(&l);
    assert!(j.starts_with(r#"{"passed":false,"tests":[{"index":0,"name":"sichtbar","hidden":false,"passed":true,"status":1,"cycles":"#), "{j}");
    assert!(j.contains(r#""fault":null,"mismatch":null,"input":{"in":[1],"in_text":"","out":[1],"out_text":null}},{"index":1"#), "{j}");
    // Passed hidden test: no input.
    assert!(j.contains(r#""name":"geheim-ok","hidden":true,"passed":true"#));
    assert!(!j.contains(r#""in":[2]"#), "{j}");
    // Failed hidden test: input and expectation for reproduction.
    assert!(j.contains(r#""mismatch":{"key":"check.out_mismatch","args":{"index":0,"expected":4,"actual":3}},"input":{"in":[3],"in_text":"a","out":[4],"out_text":"b"}}"#), "{j}");
    let m = r.metrics;
    assert!(j.contains(&format!(
        r#""metrics":{{"cycles_avg":{},"cycles_max":{},"steps_avg":3,"size":3}},"reference":{{"cycles":{},"size":3}}}}"#,
        m.cycles_avg, m.cycles_max, m.cycles_avg
    )), "{j}");
    // Without reference and packed values: null.
    let l2 = level("[test]\nname = a\n");
    let r2 = run_tests(&l2, &p);
    assert!(r2.to_json(&l2).ends_with(r#""reference":null}"#));
}

#[test]
fn sample_level_reference_and_metrics() {
    let l = parse(SAMPLE).unwrap();
    let reference = l.section("reference").unwrap();
    let p = assemble_level(&l, reference).unwrap();
    let r = run_tests(&l, &p);
    assert!(r.passed);
    assert_eq!(r.metrics.size, 9);
    assert_eq!(r.reference, Some((r.metrics.cycles_avg, 9)));
    // The start code and the empty program fail.
    for code in [l.section("start").unwrap(), ""] {
        let p = assemble_level(&l, code).unwrap();
        assert!(!run_tests(&l, &p).passed);
    }
    // Loop inversion: more instructions, fewer cycles.
    let inverted = "  mov r1, 0\n  sys READ_INT\n  cmp r0, 0\n  jz fertig\nschleife:\n  add r1, r0\n  sys READ_INT\n  cmp r0, 0\n  jnz schleife\nfertig:\n  mov r0, r1\n  sys WRITE_INT\n  halt\n";
    let p = assemble_level(&l, inverted).unwrap();
    let r2 = run_tests(&l, &p);
    assert!(r2.passed);
    assert!(r2.metrics.size > r.metrics.size);
    assert!(r2.metrics.cycles_avg < r.metrics.cycles_avg);
}

/// Packing keeps results and metrics, and the packed level carries the reference values.
#[test]
fn pack_round_trip() {
    let screen_level = "[level]\nid = bild\nbudget = 5000\n\n\
        [reference]\n  li r1, SCREEN\n  sys READ_INT\n  stb r0, [r1 + 3]\n  halt\n\n\
        [test]\nname = rot\nin = 4\nexpect_screen = reference\n\n\
        [test]\nname = gelb\nhidden = yes\nin = 14\nexpect_screen = reference\n";
    let players = [
        "  li r1, SCREEN\n  sys READ_INT\n  stb r0, [r1 + 3]\n  halt\n",
        "  li r1, SCREEN\n  mov r0, 4\n  stb r0, [r1 + 3]\n  halt\n",
        "  halt\n",
    ];
    for (src, codes) in [
        (screen_level, &players[..]),
        (SAMPLE, &["  halt\n", PRINT_SUM][..]),
    ] {
        let original = parse(src).unwrap();
        let packed = pack_level(src).unwrap();
        assert!(packed.level.section("reference").is_none());
        assert!(!packed.text.contains("[reference]"));
        let again = parse(&packed.text).unwrap();
        assert_eq!(again, packed.level);
        let ref_prog = assemble_level(&original, original.section("reference").unwrap()).unwrap();
        let ref_report = run_tests(&original, &ref_prog);
        assert_eq!(
            (packed.level.ref_cycles, packed.level.ref_size),
            (
                Some(ref_report.metrics.cycles_avg),
                Some(ref_report.metrics.size)
            )
        );
        let mut codes: Vec<&str> = codes.to_vec();
        codes.push(original.section("reference").unwrap());
        for code in codes {
            let a = run_tests(&original, &assemble_level(&original, code).unwrap());
            let b = run_tests(&packed.level, &assemble_level(&packed.level, code).unwrap());
            assert_eq!(a, b, "{code}");
        }
    }
    let packed = pack_level(screen_level).unwrap();
    assert!(packed.text.contains("expect_screen = __ref_rot"));
    assert_eq!(packed.level.image("__ref_gelb").unwrap().pixels[3], 14);
    // Packing an already packed level is refused (no reference any more).
    assert!(pack_level(&packed.text).is_err());
}

/// `TestCase::to_setup` gives the debugger session exactly the runner's start and end states,
/// including function tests, `screen_init`, `set` by symbol and the budget.
#[test]
fn session_setup_matches_runner() {
    use stapelwerk_core::runner::{prepare_test, run_prepared};
    use stapelwerk_core::session::{Session, SessionConfig};
    let l = level(
        "[test]\nname = f\ncall = quadrat\nreg r0 = 7\nseed = 3\n\n\
         [test]\nname = s\nset WERT = 9\nscreen_init = bild\nin = 1 2\nreg r5 = 4\n\n\
         [test]\nname = b\nbudget = 30\n\n[image.bild]\n.5\n",
    );
    let p = assemble_level(&l, SQUARE).unwrap();
    let cfg = SessionConfig {
        library: l.section("library").map(str::to_string),
        seed: l.seed,
        budget: l.budget,
        heap_check: l.heap_check,
        ..SessionConfig::default()
    };
    let mut s = Session::new(cfg);
    s.load_program(p.clone());
    for (i, t) in l.tests.iter().enumerate() {
        let setup = t.to_setup(&l, i as u32);
        assert_eq!(setup.index, i as u32);
        let mut prep = prepare_test(&l, &p, t);
        s.reset(Some(setup)).unwrap();
        assert_eq!(
            s.digest_u64(),
            prep.machine.state.digest(&prep.machine.tapes),
            "start {i}"
        );
        let status = run_prepared(&mut prep);
        assert_eq!(s.run(u64::MAX), status, "{i}");
        assert_eq!(
            s.digest_u64(),
            prep.machine.state.digest(&prep.machine.tapes),
            "end {i}"
        );
    }
}
