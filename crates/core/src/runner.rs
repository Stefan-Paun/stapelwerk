//! Test runs and metrics (`docs/LEVELS.md`, "Test runs and metrics").
//!
//! Every test runs on a fresh [`Machine`] built directly from the [`Program`] (no debugger
//! session involved). The first failing expectation becomes the test's `check.*` [`Mismatch`].
//! Order of the checks: run status (a fault is reported as `fault`, any other non-`HALTED` status
//! as `check.not_halted`), callee-saved registers of function tests (`check.abi_clobbered`),
//! `out`, `out_text`, `expect`/`expectb` (in file order), `expect_reg`, `expect_screen`,
//! `expect_heap_empty`.
//!
//! Output comparison: `out` compares the **number** entries of the output tape, `out_text` the
//! **byte** entries, each in order; entries of the other kind are ignored.

use crate::asm::{MnemonicFilter, assemble};
use crate::consts::{OUT_KIND_BYTE, OUT_KIND_INT, SCREEN_BYTES, SCREEN_START, SCREEN_W};
use crate::diag::{Arg, Args, Diagnostic, FILE_PLAYER, write_args};
use crate::fault::{Fault, FaultKind};
use crate::isa::Reg;
use crate::json::JsonWriter;
use crate::level::pack::{pack_source, ref_image_name};
use crate::level::{AddrRef, Level, MemValues, ScreenExpect, TestCase};
use crate::machine::{Machine, MachineConfig, Status};
use crate::program::Program;
use crate::session::{
    CALLEE_SAVED, CallInfo, LineCounter, call_registers, prepare_call, run_to_end,
};

/// Why a test that ran to completion did not pass (`check.*` key).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mismatch {
    pub key: &'static str,
    pub args: Args,
}

impl Mismatch {
    fn new(key: &'static str, args: Args) -> Self {
        Mismatch { key, args }
    }
}

/// Result of one test.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TestResult {
    /// Index in the level's test list.
    pub index: u32,
    pub name: String,
    pub hidden: bool,
    pub passed: bool,
    pub status: Status,
    pub cycles: u64,
    pub steps: u64,
    pub fault: Option<Fault>,
    pub mismatch: Option<Mismatch>,
}

/// Metrics over all tests.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Metrics {
    /// Average cycles per test, rounded up.
    pub cycles_avg: u64,
    pub cycles_max: u64,
    /// Average steps per test, rounded up.
    pub steps_avg: u64,
    /// Instruction words from the player's file.
    pub size: u32,
}

/// Result of `run_tests`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TestReport {
    pub passed: bool,
    pub tests: Vec<TestResult>,
    pub metrics: Metrics,
    /// `(ref_cycles, ref_size)` of the reference solution, if known.
    pub reference: Option<(u64, u32)>,
}

/// Computes the metrics from test results (averages rounded up; 0 for no tests).
pub fn metrics(tests: &[TestResult], size: u32) -> Metrics {
    let n = tests.len() as u64;
    if n == 0 {
        return Metrics {
            size,
            ..Metrics::default()
        };
    }
    let cycles: u64 = tests.iter().map(|t| t.cycles).sum();
    let steps: u64 = tests.iter().map(|t| t.steps).sum();
    Metrics {
        cycles_avg: cycles.div_ceil(n),
        cycles_max: tests.iter().map(|t| t.cycles).max().unwrap_or(0),
        steps_avg: steps.div_ceil(n),
        size,
    }
}

// ---------------------------------------------------------------------------------------------
// Assembling level code
// ---------------------------------------------------------------------------------------------

/// Assembles player code for `level` (with its `[library]` and `allow`/`forbid` filter).
pub fn assemble_level(level: &Level, source: &str) -> Result<Program, Vec<Diagnostic>> {
    assemble(source, level.section("library"), &level.filter)
}

/// Assembles the level's `[reference]` (with the library and the level's filter); `None` if the
/// level has no reference (packed levels).
pub fn assemble_reference(level: &Level) -> Option<Result<Program, Vec<Diagnostic>>> {
    level
        .section("reference")
        .map(|src| assemble_level(level, src))
}

/// Same as [`assemble_level`] but ignores `allow`/`forbid` (used where only the library matters).
pub fn assemble_unfiltered(level: &Level, source: &str) -> Result<Program, Vec<Diagnostic>> {
    assemble(source, level.section("library"), &MnemonicFilter::All)
}

// ---------------------------------------------------------------------------------------------
// One test
// ---------------------------------------------------------------------------------------------

/// Values of r4, r5, r6, r7 and fp before a function call, drawn from the test seed with
/// SplitMix64 (`fp` is never 0). Shared with the debugger session so both set up function tests
/// identically.
pub fn function_test_regs(seed: u32) -> [u32; 5] {
    call_registers(seed)
}

/// Callee-saved registers checked after a function test, in report order.
pub const CALLEE_CHECKED: [Reg; 6] = CALLEE_SAVED;

/// Resolves a test address: a number, a symbol (label or constant of `program`, or a built-in
/// constant) or `SYMBOL ± n`, in 0…0xFFFF ([`AddrRef::resolve`]).
pub fn resolve_addr(program: &Program, addr: &AddrRef) -> Option<u32> {
    addr.resolve(program)
}

/// Where `expect_screen = reference` takes its screen from.
#[derive(Clone, Copy, Debug)]
pub enum ScreenRef<'a> {
    /// No reference screen available (compared against a black screen).
    None,
    /// This run *is* the reference run: the expectation holds trivially.
    SelfRun,
    /// The reference solution's screen after the same test.
    Screen(&'a [u8]),
}

/// A machine prepared for one test (see [`prepare_test`]).
#[derive(Clone, Debug)]
pub struct PreparedTest {
    pub machine: Machine,
    /// For function tests: trap address, target and the callee-saved values before the call.
    pub call: Option<CallInfo>,
    /// Executions per player line (for `hottest_line` after a budget overrun).
    pub lines: LineCounter,
    /// Set when the run cannot start (function label missing): the result is this fault.
    pub start_fault: Option<Fault>,
}

/// Builds the fresh machine for test `test` of `level`: budget, seed, heap check, input tapes,
/// `screen_init`, `set`/`setb` (written directly into memory; addresses beyond 0xFFFF are cut off,
/// unresolved symbols are skipped), `reg` lines, and for function tests the random callee-saved
/// registers, the pushed `__trap` return address and `pc = LABEL`.
pub fn prepare_test(level: &Level, program: &Program, test: &TestCase) -> PreparedTest {
    let seed = level.test_seed(test);
    let config = MachineConfig {
        heap_check: level.heap_check,
        seed,
        budget: level.test_budget(test),
        interactive: false,
    };
    let mut m = Machine::new(program, config);
    m.tapes.in_ints = test.input.clone();
    m.tapes.in_text = test.in_text.clone();
    if let Some(img) = test.screen_init.as_deref().and_then(|n| level.image(n)) {
        write_bytes(&mut m, SCREEN_START, &img.pixels);
    }
    for set in &test.sets {
        if let Some(addr) = resolve_addr(program, &set.addr) {
            write_bytes(&mut m, addr, &set.values.to_bytes());
        }
    }
    let mut call = None;
    let mut start_fault = None;
    match &test.call {
        // Order (shared with the session): screen, sets, random callee-saved registers, `reg`
        // lines, push `__trap`, `pc = LABEL`.
        Some(label) => {
            let target = program
                .symbol(label)
                .map(|s| s.value)
                .filter(|v| (0..0x1_0000).contains(v));
            match target {
                Some(t) => {
                    call = Some(prepare_call(
                        &mut m.state,
                        program.trap_addr,
                        t as u32,
                        seed,
                        &test.regs,
                    ));
                }
                None => {
                    for (r, v) in [Reg::R4, Reg::R5, Reg::R6, Reg::R7, Reg::Fp]
                        .iter()
                        .zip(call_registers(seed))
                    {
                        m.state.set_reg(*r, v);
                    }
                    for &(r, v) in &test.regs {
                        m.state.set_reg(r, v);
                    }
                    let f = FaultKind::BadJumpTarget { target: 0 }.at(m.state.pc, 0);
                    start_fault = Some(f);
                }
            }
        }
        None => {
            for &(r, v) in &test.regs {
                m.state.set_reg(r, v);
            }
        }
    }
    PreparedTest {
        machine: m,
        call,
        lines: LineCounter::new(program),
        start_fault,
    }
}

fn write_bytes(m: &mut Machine, addr: u32, bytes: &[u8]) {
    let mem = m.state.mem.bytes_mut();
    for (i, b) in bytes.iter().enumerate() {
        if let Some(slot) = (addr as usize).checked_add(i).and_then(|a| mem.get_mut(a)) {
            *slot = *b;
        }
    }
}

/// Runs a prepared test to its end (halt, fault, budget; function tests: `pc` at `__trap`, which
/// counts as `HALTED` without executing the trap word). Uses [`run_to_end`], the same loop as the
/// debugger session, so cycles and steps agree.
pub fn run_prepared(p: &mut PreparedTest) -> Status {
    if let Some(f) = p.start_fault {
        p.machine.state.status = Status::Fault;
        p.machine.state.fault = Some(f);
        return Status::Fault;
    }
    run_to_end(&mut p.machine, p.call.map(|c| c.trap), &mut p.lines)
}

/// Runs test `index` and returns the result together with the final machine (for screens and
/// debugging). `index` out of range → a failed result with status `NOT_ASSEMBLED`.
pub fn run_test_machine(
    level: &Level,
    program: &Program,
    index: u32,
    screen_ref: ScreenRef<'_>,
) -> (TestResult, Option<Machine>) {
    let Some(test) = level.tests.get(index as usize) else {
        let r = TestResult {
            index,
            name: String::new(),
            hidden: false,
            passed: false,
            status: Status::NotAssembled,
            cycles: 0,
            steps: 0,
            fault: None,
            mismatch: None,
        };
        return (r, None);
    };
    let mut p = prepare_test(level, program, test);
    let status = run_prepared(&mut p);
    let r = finish_test(level, program, index, &p, status, screen_ref);
    (r, Some(p.machine))
}

/// Builds the result of test `index` after its prepared machine `p` stopped with `status`
/// ([`run_prepared`], or a single-stepping loop with the same end rule as the CLI's `--trace`).
/// A run that ended at the budget reports `check.not_halted` with the extra arguments `cycles`,
/// `hottest_line` (most executed line of the player's file, 0 if none) and `hottest_count`.
pub fn finish_test(
    level: &Level,
    program: &Program,
    index: u32,
    p: &PreparedTest,
    status: Status,
    screen_ref: ScreenRef<'_>,
) -> TestResult {
    let test = level.tests.get(index as usize);
    let m = &p.machine;
    let fault = if status == Status::Fault {
        m.state.fault
    } else {
        None
    };
    let mismatch = match (status, test) {
        (Status::Fault, _) => None,
        (Status::Halted, Some(test)) => evaluate(level, program, test, p, screen_ref),
        (Status::BudgetExceeded, _) => {
            let line = p.lines.hottest_line();
            Some(Mismatch::new(
                "check.not_halted",
                vec![
                    ("status", status.name().into()),
                    ("cycles", Arg::Int(m.state.cycles as i64)),
                    ("hottest_line", line.into()),
                    ("hottest_count", Arg::Int(p.lines.count_of(line) as i64)),
                ],
            ))
        }
        (other, _) => Some(Mismatch::new(
            "check.not_halted",
            vec![("status", other.name().into())],
        )),
    };
    TestResult {
        index,
        name: test.map(|t| t.name.clone()).unwrap_or_default(),
        hidden: test.is_some_and(|t| t.hidden),
        passed: status == Status::Halted && mismatch.is_none(),
        status,
        cycles: m.state.cycles,
        steps: m.state.steps,
        fault,
        mismatch,
    }
}

/// Checks every expectation of a halted run; returns the first failing one.
fn evaluate(
    level: &Level,
    program: &Program,
    test: &TestCase,
    p: &PreparedTest,
    screen_ref: ScreenRef<'_>,
) -> Option<Mismatch> {
    let m = &p.machine;
    if let Some(c) = p.call.and_then(|c| c.clobbered(&m.state)) {
        return Some(Mismatch::new(
            "check.abi_clobbered",
            vec![
                ("reg", c.reg.name().into()),
                ("before", c.before.into()),
                ("after", c.after.into()),
            ],
        ));
    }
    let len = m.state.out_len as usize;
    let tape = |kind: u8| -> Vec<i32> {
        m.tapes
            .out_kinds
            .iter()
            .zip(&m.tapes.out_vals)
            .take(len)
            .filter(|(k, _)| **k == kind)
            .map(|(_, v)| *v)
            .collect()
    };
    if let Some(expected) = &test.out
        && let Some(mm) = compare_numbers(expected, &tape(OUT_KIND_INT))
    {
        return Some(mm);
    }
    if let Some(expected) = &test.out_text
        && let Some(mm) = compare_text(expected, &tape(OUT_KIND_BYTE))
    {
        return Some(mm);
    }
    for e in &test.expects {
        let mem = m.state.mem.bytes();
        let Some(base) = resolve_addr(program, &e.addr) else {
            let expected: i64 = match &e.values {
                MemValues::Words(w) => w.first().map_or(0, |v| *v as i32 as i64),
                MemValues::Bytes(b) => b.first().map_or(0, |v| *v as i64),
            };
            return Some(mem_mismatch(0, expected, 0));
        };
        let byte = |a: u32| -> u8 { mem.get(a as usize).copied().unwrap_or(0) };
        match &e.values {
            MemValues::Words(words) => {
                for (i, &w) in words.iter().enumerate() {
                    let a = base.wrapping_add(4 * i as u32);
                    let actual = u32::from_le_bytes([
                        byte(a),
                        byte(a.wrapping_add(1)),
                        byte(a.wrapping_add(2)),
                        byte(a.wrapping_add(3)),
                    ]);
                    if actual != w {
                        return Some(mem_mismatch(a, w as i32 as i64, actual as i32 as i64));
                    }
                }
            }
            MemValues::Bytes(bytes) => {
                for (i, &b) in bytes.iter().enumerate() {
                    let a = base.wrapping_add(i as u32);
                    let actual = byte(a);
                    if actual != b {
                        return Some(mem_mismatch(a, b as i64, actual as i64));
                    }
                }
            }
        }
    }
    for &(r, v) in &test.expect_regs {
        let actual = m.state.reg(r);
        if actual != v {
            return Some(Mismatch::new(
                "check.reg_mismatch",
                vec![
                    ("reg", r.name().into()),
                    ("expected", (v as i32).into()),
                    ("actual", (actual as i32).into()),
                ],
            ));
        }
    }
    if let Some(exp) = &test.expect_screen {
        let black = [0u8; SCREEN_BYTES as usize];
        let expected: Option<&[u8]> = match (exp, screen_ref) {
            (ScreenExpect::Reference, ScreenRef::SelfRun) => None,
            (ScreenExpect::Reference, ScreenRef::Screen(s)) => Some(s),
            (ScreenExpect::Reference, ScreenRef::None) => Some(&black),
            (ScreenExpect::Image(name), _) => {
                Some(level.image(name).map_or(&black[..], |i| &i.pixels))
            }
        };
        if let Some(expected) = expected
            && let Some(mm) = compare_screen(expected, m.state.mem.screen())
        {
            return Some(mm);
        }
    }
    if test.expect_heap_empty {
        let live: Vec<_> = m.state.heap.blocks().iter().filter(|b| b.live).collect();
        if let Some(first) = live.first() {
            let line = program
                .line_of(first.alloc_pc)
                .filter(|e| e.file == FILE_PLAYER)
                .map_or(0, |e| e.line);
            return Some(Mismatch::new(
                "check.heap_not_empty",
                vec![
                    ("count", (live.len() as u32).into()),
                    ("alloc_line", line.into()),
                ],
            ));
        }
    }
    None
}

fn mem_mismatch(addr: u32, expected: i64, actual: i64) -> Mismatch {
    Mismatch::new(
        "check.mem_mismatch",
        vec![
            ("addr", addr.into()),
            ("expected", Arg::Int(expected)),
            ("actual", Arg::Int(actual)),
        ],
    )
}

fn compare_numbers(expected: &[i32], actual: &[i32]) -> Option<Mismatch> {
    let n = expected.len().max(actual.len());
    for i in 0..n {
        let idx = Arg::from(i as u32);
        match (expected.get(i), actual.get(i)) {
            (Some(e), Some(a)) if e != a => {
                return Some(Mismatch::new(
                    "check.out_mismatch",
                    vec![
                        ("index", idx),
                        ("expected", (*e).into()),
                        ("actual", (*a).into()),
                    ],
                ));
            }
            (Some(e), None) => {
                return Some(Mismatch::new(
                    "check.out_missing",
                    vec![("index", idx), ("expected", (*e).into())],
                ));
            }
            (None, Some(a)) => {
                return Some(Mismatch::new(
                    "check.out_extra",
                    vec![("index", idx), ("actual", (*a).into())],
                ));
            }
            _ => {}
        }
    }
    None
}

fn compare_text(expected: &[u8], actual: &[i32]) -> Option<Mismatch> {
    let n = expected.len().max(actual.len());
    for i in 0..n {
        let e = expected.get(i).map_or(-1, |b| *b as i32);
        let a = actual.get(i).copied().unwrap_or(-1);
        if e != a {
            return Some(Mismatch::new(
                "check.text_mismatch",
                vec![
                    ("offset", (i as u32).into()),
                    ("expected", e.into()),
                    ("actual", a.into()),
                ],
            ));
        }
    }
    None
}

/// Compares the low four bits of two screens.
fn compare_screen(expected: &[u8], actual: &[u8]) -> Option<Mismatch> {
    let mut first = None;
    let mut count = 0u32;
    for i in 0..SCREEN_BYTES as usize {
        let e = expected.get(i).copied().unwrap_or(0) & 0x0F;
        let a = actual.get(i).copied().unwrap_or(0) & 0x0F;
        if e != a {
            count += 1;
            if first.is_none() {
                first = Some((i as u32, e, a));
            }
        }
    }
    first.map(|(i, e, a)| {
        Mismatch::new(
            "check.screen_mismatch",
            vec![
                ("x", (i % SCREEN_W).into()),
                ("y", (i / SCREEN_W).into()),
                ("expected", (e as u32).into()),
                ("actual", (a as u32).into()),
                ("count", count.into()),
            ],
        )
    })
}

/// Runs test `index` of `level` against `program` on a fresh machine. For
/// `expect_screen = reference` the reference solution is run on the same test first.
pub fn run_test(level: &Level, program: &Program, index: u32) -> TestResult {
    let needs_ref = level
        .tests
        .get(index as usize)
        .is_some_and(|t| t.expect_screen == Some(ScreenExpect::Reference));
    let screen = if needs_ref {
        reference_screen(level, index)
    } else {
        None
    };
    let sref = match &screen {
        Some(s) => ScreenRef::Screen(s),
        None => ScreenRef::None,
    };
    run_test_machine(level, program, index, sref).0
}

// ---------------------------------------------------------------------------------------------
// Reference and all tests
// ---------------------------------------------------------------------------------------------

/// The reference solution run on every test.
#[derive(Clone, Debug)]
pub struct Reference {
    pub program: Program,
    pub results: Vec<TestResult>,
    /// Screen after each test (4096 bytes; empty if the test index had no machine).
    pub screens: Vec<Vec<u8>>,
    /// `cycles_avg` over all tests (= `ref_cycles`).
    pub cycles: u64,
    /// Size of the reference (= `ref_size`).
    pub size: u32,
}

impl Reference {
    /// `true` if the reference passes every test.
    pub fn passes(&self) -> bool {
        !self.results.is_empty() && self.results.iter().all(|r| r.passed)
    }
}

/// Assembles and runs the level's `[reference]` on every test. `Ok(None)` if the level has no
/// reference; `Err` with the assembler diagnostics if it does not assemble.
pub fn run_reference(level: &Level) -> Result<Option<Reference>, Vec<Diagnostic>> {
    let program = match assemble_reference(level) {
        None => return Ok(None),
        Some(r) => r?,
    };
    let mut results = Vec::with_capacity(level.tests.len());
    let mut screens = Vec::with_capacity(level.tests.len());
    for i in 0..level.tests.len() as u32 {
        let (r, m) = run_test_machine(level, &program, i, ScreenRef::SelfRun);
        results.push(r);
        screens.push(m.map(|m| m.state.mem.screen().to_vec()).unwrap_or_default());
    }
    let met = metrics(&results, program.size);
    Ok(Some(Reference {
        cycles: met.cycles_avg,
        size: program.size,
        program,
        results,
        screens,
    }))
}

/// The reference solution's screen after test `index` (unpacked levels only; ABI
/// `sess_reference_screen`).
pub fn reference_screen(level: &Level, index: u32) -> Option<Vec<u8>> {
    let program = assemble_reference(level)?.ok()?;
    let (_, m) = run_test_machine(level, &program, index, ScreenRef::SelfRun);
    m.map(|m| m.state.mem.screen().to_vec())
}

/// Runs all tests. The reference (for `expect_screen = reference` and the reference metrics) is
/// computed from `[reference]` if present, else `ref_cycles`/`ref_size` of a packed file are used.
pub fn run_tests(level: &Level, program: &Program) -> TestReport {
    let reference = run_reference(level).ok().flatten();
    run_tests_with(level, program, reference.as_ref())
}

/// [`run_tests`] with an already computed reference (e.g. when checking many mutants).
pub fn run_tests_with(
    level: &Level,
    program: &Program,
    reference: Option<&Reference>,
) -> TestReport {
    let tests: Vec<TestResult> = (0..level.tests.len() as u32)
        .map(|i| {
            let sref = match reference.and_then(|r| r.screens.get(i as usize)) {
                Some(s) if !s.is_empty() => ScreenRef::Screen(s),
                _ => ScreenRef::None,
            };
            run_test_machine(level, program, i, sref).0
        })
        .collect();
    let metrics = metrics(&tests, program.size);
    let packed = level.ref_cycles.zip(level.ref_size);
    TestReport {
        passed: !tests.is_empty() && tests.iter().all(|t| t.passed),
        tests,
        metrics,
        reference: packed.or(reference.map(|r| (r.cycles, r.size))),
    }
}

// ---------------------------------------------------------------------------------------------
// JSON
// ---------------------------------------------------------------------------------------------

impl TestResult {
    /// Writes one entry of the `tests` array (without the optional `input`, which needs the level).
    pub fn write_json(&self, w: &mut JsonWriter) {
        w.begin_object();
        self.write_fields(w);
        w.end_object();
    }

    fn write_fields(&self, w: &mut JsonWriter) {
        w.field_uint("index", self.index as u64);
        w.field_str("name", &self.name);
        w.field_bool("hidden", self.hidden);
        w.field_bool("passed", self.passed);
        w.field_uint("status", self.status.code() as u64);
        w.field_uint("cycles", self.cycles);
        w.field_uint("steps", self.steps);
        w.key("fault");
        match &self.fault {
            Some(f) => f.write_json(w),
            None => w.null(),
        }
        w.key("mismatch");
        match &self.mismatch {
            Some(m) => {
                w.begin_object();
                w.field_str("key", m.key);
                w.key("args");
                write_args(w, &m.args);
                w.end_object();
            }
            None => w.null(),
        }
    }

    /// Writes one entry including `input` (`{in, in_text, out, out_text}`): always for visible
    /// tests, for hidden tests only when they failed (ABI `sess_check` / `sess_run_tests`).
    pub fn write_json_with_input(&self, level: &Level, w: &mut JsonWriter) {
        w.begin_object();
        self.write_fields(w);
        let test = level.tests.get(self.index as usize);
        if let Some(t) = test
            && (!t.hidden || !self.passed)
        {
            w.key("input");
            write_input(t, w);
        }
        w.end_object();
    }
}

fn write_input(t: &TestCase, w: &mut JsonWriter) {
    w.begin_object();
    w.key("in");
    w.begin_array();
    for v in &t.input {
        w.int(*v as i64);
    }
    w.end_array();
    w.field_str("in_text", &String::from_utf8_lossy(&t.in_text));
    w.key("out");
    match &t.out {
        Some(out) => {
            w.begin_array();
            for v in out {
                w.int(*v as i64);
            }
            w.end_array();
        }
        None => w.null(),
    }
    w.key("out_text");
    match &t.out_text {
        Some(b) => w.str(&String::from_utf8_lossy(b)),
        None => w.null(),
    }
    w.end_object();
}

impl TestReport {
    /// The `sess_run_tests` JSON (`docs/ABI.md`).
    pub fn to_json(&self, level: &Level) -> String {
        let mut w = JsonWriter::new();
        w.begin_object();
        w.field_bool("passed", self.passed);
        w.key("tests");
        w.begin_array();
        for t in &self.tests {
            t.write_json_with_input(level, &mut w);
        }
        w.end_array();
        w.key("metrics");
        w.begin_object();
        w.field_uint("cycles_avg", self.metrics.cycles_avg);
        w.field_uint("cycles_max", self.metrics.cycles_max);
        w.field_uint("steps_avg", self.metrics.steps_avg);
        w.field_uint("size", self.metrics.size as u64);
        w.end_object();
        w.key("reference");
        match self.reference {
            Some((cycles, size)) => {
                w.begin_object();
                w.field_uint("cycles", cycles);
                w.field_uint("size", size as u64);
                w.end_object();
            }
            None => w.null(),
        }
        w.end_object();
        w.finish()
    }
}

// ---------------------------------------------------------------------------------------------
// Packing
// ---------------------------------------------------------------------------------------------

/// Why a level cannot be packed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PackError {
    /// The level file does not parse.
    Parse(Vec<Diagnostic>),
    /// The level has no `[reference]` section.
    NoReference,
    /// The reference does not assemble.
    Reference(Vec<Diagnostic>),
    /// The packed text does not parse again (internal error).
    Repack(Vec<Diagnostic>),
}

/// A packed level: the text for the web build and its parsed form.
#[derive(Clone, Debug)]
pub struct PackedLevel {
    pub text: String,
    pub level: Level,
}

/// Packs one level source (`stapelwerk pack-levels`): runs the reference on every test, then
/// rewrites the text with [`pack_source`].
pub fn pack_level(src: &str) -> Result<PackedLevel, PackError> {
    let level = crate::level::parse(src).map_err(PackError::Parse)?;
    let reference = run_reference(&level)
        .map_err(PackError::Reference)?
        .ok_or(PackError::NoReference)?;
    let screens: Vec<(String, Vec<u8>)> = level
        .tests
        .iter()
        .zip(&reference.screens)
        .filter(|(t, _)| t.expect_screen == Some(ScreenExpect::Reference))
        .map(|(t, s)| (t.name.clone(), s.clone()))
        .collect();
    let text = pack_source(src, reference.cycles, reference.size, &screens);
    let packed = crate::level::parse(&text).map_err(PackError::Repack)?;
    debug_assert!(
        screens
            .iter()
            .all(|(n, _)| packed.image(&ref_image_name(n)).is_some())
    );
    Ok(PackedLevel {
        text,
        level: packed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn result(cycles: u64, steps: u64) -> TestResult {
        TestResult {
            index: 0,
            name: "t".into(),
            hidden: false,
            passed: true,
            status: Status::Halted,
            cycles,
            steps,
            fault: None,
            mismatch: None,
        }
    }

    #[test]
    fn metrics_round_up() {
        let m = metrics(&[result(10, 3), result(11, 4)], 7);
        assert_eq!(m.cycles_avg, 11);
        assert_eq!(m.cycles_max, 11);
        assert_eq!(m.steps_avg, 4);
        assert_eq!(m.size, 7);
        assert_eq!(metrics(&[], 2).cycles_avg, 0);
    }

    #[test]
    fn json_entry() {
        let mut w = JsonWriter::new();
        result(5, 2).write_json(&mut w);
        assert_eq!(
            w.finish(),
            r#"{"index":0,"name":"t","hidden":false,"passed":true,"status":1,"cycles":5,"steps":2,"fault":null,"mismatch":null}"#
        );
    }

    #[test]
    fn function_regs_are_deterministic() {
        let a = function_test_regs(7);
        assert_eq!(a, function_test_regs(7));
        assert_ne!(a, function_test_regs(8));
        assert_ne!(a[4], 0);
    }

    #[test]
    fn output_comparison() {
        assert_eq!(compare_numbers(&[1, 2], &[1, 2]), None);
        assert_eq!(
            compare_numbers(&[1, 2], &[1]).unwrap().key,
            "check.out_missing"
        );
        assert_eq!(
            compare_numbers(&[1], &[1, 5]).unwrap().key,
            "check.out_extra"
        );
        assert_eq!(
            compare_numbers(&[1], &[2]).unwrap().key,
            "check.out_mismatch"
        );
        let t = compare_text(b"ab", &[97]).unwrap();
        assert_eq!(t.args[2], ("actual", Arg::Int(-1)));
    }
}
