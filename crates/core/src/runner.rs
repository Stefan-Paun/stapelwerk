//! Test runs and metrics (`docs/LEVELS.md`, "Test runs and metrics"). Later phase; the result
//! types are part of the phase-0 contract.

use crate::diag::{Args, write_args};
use crate::fault::Fault;
use crate::json::JsonWriter;
use crate::level::Level;
use crate::machine::Status;
use crate::program::Program;

/// Why a test that ran to completion did not pass (`check.*` key).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mismatch {
    pub key: &'static str,
    pub args: Args,
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

/// Runs test `index` of `level` against `program` on a fresh machine. Later phase.
pub fn run_test(_level: &Level, _program: &Program, index: u32) -> TestResult {
    TestResult {
        index,
        name: String::new(),
        hidden: false,
        passed: false,
        status: Status::NotAssembled,
        cycles: 0,
        steps: 0,
        fault: None,
        mismatch: None,
    }
}

/// Runs all tests. Later phase.
pub fn run_tests(level: &Level, program: &Program) -> TestReport {
    let tests: Vec<TestResult> = (0..level.tests.len() as u32)
        .map(|i| run_test(level, program, i))
        .collect();
    let metrics = metrics(&tests, program.size);
    TestReport {
        passed: !tests.is_empty() && tests.iter().all(|t| t.passed),
        tests,
        metrics,
        reference: level.ref_cycles.zip(level.ref_size),
    }
}

impl TestResult {
    /// Writes one entry of the `tests` array (without the optional `input`, which needs the level).
    pub fn write_json(&self, w: &mut JsonWriter) {
        w.begin_object();
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
        w.end_object();
    }
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
}
