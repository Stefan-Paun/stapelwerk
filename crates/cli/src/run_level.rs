//! `run <file.s> --level <file.lvl> [--test NAME|INDEX]`: runs a player program against a level's
//! tests exactly like the test runner (`runner::prepare_test`, `run_prepared`, `finish_test`).
//!
//! With `--test` one test runs: the program's output goes to stdout, status, fault (with source
//! line), the `check.*` result and the verdict to stderr. Without `--test` every test runs and a
//! summary in the spirit of `run_tests` is printed on stdout. Exit code 0 only if every test that
//! ran passed.

use crate::levels_cmds::section_first_line;
use crate::messages;
use crate::program_cmds::{
    LibraryName, RunOptions, format_output, print_diags, read_text, report_status, trace,
};
use stapelwerk_core::level::{Level, ScreenExpect, parse};
use stapelwerk_core::machine::Status;
use stapelwerk_core::program::Program;
use stapelwerk_core::runner::{
    Reference, ScreenRef, TestResult, assemble_level, finish_test, metrics, prepare_test,
    run_prepared, run_reference,
};
use std::io::Write;

/// Finds a test by name, or else by 0-based index.
fn find_test(level: &Level, sel: &str) -> Option<u32> {
    if let Some(i) = level.tests.iter().position(|t| t.name == sel) {
        return Some(i as u32);
    }
    sel.parse::<u32>()
        .ok()
        .filter(|i| (*i as usize) < level.tests.len())
}

/// Runs one test the way the runner does; with `trace_out` every instruction is printed.
fn run_one(
    level: &Level,
    program: &Program,
    index: u32,
    reference: Option<&Reference>,
    trace_out: Option<&mut dyn Write>,
) -> (TestResult, stapelwerk_core::runner::PreparedTest, Status) {
    let test = &level.tests[index as usize];
    let mut p = prepare_test(level, program, test);
    let status = match trace_out {
        Some(out) if p.start_fault.is_none() => {
            let trap = p.call.map(|c| c.trap);
            trace(&mut p.machine, program, trap, &mut p.lines, out)
        }
        _ => run_prepared(&mut p),
    };
    let sref = match reference.and_then(|r| r.screens.get(index as usize)) {
        Some(s) if !s.is_empty() => ScreenRef::Screen(s),
        _ => ScreenRef::None,
    };
    let r = finish_test(level, program, index, &p, status, sref);
    (r, p, status)
}

/// One line: why a test failed (fault with source line, or the `check.*` mismatch).
fn failure_text(program: &Program, r: &TestResult) -> String {
    if let Some(f) = &r.fault {
        return format!(
            "fault at 0x{:04X}{}: {}",
            f.pc,
            crate::program_cmds::line_note(program, f.pc),
            messages::fault(f)
        );
    }
    match &r.mismatch {
        Some(m) => messages::mismatch(m),
        None => format!("status {}", r.status.name()),
    }
}

pub fn run_level(
    path: &str,
    level_path: &str,
    test_sel: Option<&str>,
    opts: &RunOptions,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    let Some(level_src) = read_text(level_path, err) else {
        return 1;
    };
    let mut level = match parse(&level_src) {
        Ok(l) => l,
        Err(diags) => {
            for d in &diags {
                let _ = writeln!(err, "{}", messages::diagnostic(level_path, d));
            }
            let _ = writeln!(
                err,
                "stapelwerk: {} error(s) in '{level_path}'",
                diags.len()
            );
            return 1;
        }
    };
    // The reference (for `expect_screen = reference` and the reference metrics) is computed with
    // the level as written, before any overrides.
    let needs_reference = level
        .tests
        .iter()
        .any(|t| t.expect_screen == Some(ScreenExpect::Reference))
        || test_sel.is_none();
    let reference = if needs_reference {
        run_reference(&level).ok().flatten()
    } else {
        None
    };
    if let Some(b) = opts.budget {
        level.budget = b;
        for t in &mut level.tests {
            t.budget = None;
        }
    }
    if let Some(h) = opts.heap_check {
        level.heap_check = h;
    }
    let indices: Vec<u32> = match test_sel {
        Some(sel) => match find_test(&level, sel) {
            Some(i) => vec![i],
            None => {
                let names: Vec<String> = level
                    .tests
                    .iter()
                    .enumerate()
                    .map(|(i, t)| format!("{i}:{}", t.name))
                    .collect();
                let _ = writeln!(
                    err,
                    "stapelwerk: --test: no test '{sel}' in '{level_path}' (tests: {})",
                    names.join(", ")
                );
                return 2;
            }
        },
        None => (0..level.tests.len() as u32).collect(),
    };
    let Some(src) = read_text(path, err) else {
        return 1;
    };
    let program = match assemble_level(&level, &src) {
        Ok(p) => p,
        Err(diags) => {
            let lib = LibraryName {
                name: level_path,
                first_line: section_first_line(&level_src, "library").unwrap_or(1),
            };
            print_diags(path, Some(&lib), &diags, err);
            return 1;
        }
    };

    if let [index] = indices[..]
        && test_sel.is_some()
    {
        let trace_out: Option<&mut dyn Write> = if opts.trace { Some(&mut *out) } else { None };
        let (r, p, status) = run_one(&level, &program, index, reference.as_ref(), trace_out);
        let _ = out.write_all(&format_output(&p.machine));
        let _ = writeln!(
            err,
            "-- test #{index} '{}'{} of {}",
            r.name,
            if r.hidden { " (hidden)" } else { "" },
            level.id
        );
        if p.start_fault.is_some() {
            let _ = writeln!(
                err,
                "-- call = {}: not a code label of the program",
                level.tests[index as usize].call.as_deref().unwrap_or("")
            );
        } else {
            report_status(&p.machine, &program, status, &p.lines, err);
        }
        if let Some(m) = &r.mismatch {
            let _ = writeln!(err, "-- check: {}", messages::mismatch(m));
        }
        let _ = writeln!(err, "-- {}", if r.passed { "PASSED" } else { "FAILED" });
        return if r.passed { 0 } else { 1 };
    }

    let _ = writeln!(
        out,
        "level {}: {} tests ({} hidden), budget {}, heap check {}",
        level.id,
        level.tests.len(),
        level.tests.iter().filter(|t| t.hidden).count(),
        level.budget,
        level.heap_check.as_str()
    );
    let mut results = Vec::with_capacity(indices.len());
    for &i in &indices {
        let trace_out: Option<&mut dyn Write> = if opts.trace {
            let t = &level.tests[i as usize];
            let _ = writeln!(out, "== test #{i} '{}'", t.name);
            Some(&mut *out)
        } else {
            None
        };
        let (r, _, _) = run_one(&level, &program, i, reference.as_ref(), trace_out);
        let name = format!("#{i} {}{}", r.name, if r.hidden { " (hidden)" } else { "" });
        let _ = if r.passed {
            writeln!(
                out,
                "ok    {name:<32} {} cycles, {} steps",
                r.cycles, r.steps
            )
        } else {
            writeln!(out, "FAIL  {name:<32} {}", failure_text(&program, &r))
        };
        results.push(r);
    }
    let met = metrics(&results, program.size);
    let passed = results.iter().filter(|r| r.passed).count();
    let ref_text = match level
        .ref_cycles
        .zip(level.ref_size)
        .or(reference.as_ref().map(|r| (r.cycles, r.size)))
    {
        Some((c, s)) => format!("; reference {c} cycles avg, size {s}"),
        None => String::new(),
    };
    let _ = writeln!(
        out,
        "{passed}/{} passed; cycles avg {}, max {}; steps avg {}; size {}{ref_text}",
        results.len(),
        met.cycles_avg,
        met.cycles_max,
        met.steps_avg,
        met.size
    );
    if !results.is_empty() && passed == results.len() {
        0
    } else {
        1
    }
}
