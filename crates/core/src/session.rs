//! Debugger session with time travel (`docs/ABI.md`, "Session semantics"). Later phase; the
//! public signatures below mirror the WASM exports `sess_*` and are part of the phase-0 contract.

use crate::asm::MnemonicFilter;
use crate::diag::Diagnostic;
use crate::level::Level;
use crate::machine::{Machine, MachineState, Status};
use crate::program::Program;
use crate::runner::{TestReport, TestResult};

/// `kind` argument of `back_to_change`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChangeKind {
    /// Register by index (0–9).
    Register = 0,
    /// Memory word at a 4-aligned address.
    Word = 1,
    /// Memory byte.
    Byte = 2,
}

impl ChangeKind {
    pub const fn from_code(code: u32) -> Option<ChangeKind> {
        match code {
            0 => Some(ChangeKind::Register),
            1 => Some(ChangeKind::Word),
            2 => Some(ChangeKind::Byte),
            _ => None,
        }
    }
}

/// Result of `assemble`: `{ok, size, diagnostics}`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AsmReport {
    pub ok: bool,
    pub size: u32,
    pub diagnostics: Vec<Diagnostic>,
}

/// One frame of the fp chain: `{fp, ret, func, size}`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    pub fp: u32,
    /// Return address stored at `[fp + 4]`.
    pub ret: u32,
    /// Function name (nearest global label), if known.
    pub func: Option<String>,
    /// Frame size in bytes.
    pub size: u32,
}

/// A debugger session for one level (or the sandbox when `level` is `None`).
#[derive(Clone, Debug, Default)]
pub struct Session {
    pub level: Option<Level>,
    pub program: Option<Program>,
    pub machine: Option<Machine>,
    /// Instruction addresses with a breakpoint, sorted.
    pub breakpoints: Vec<u32>,
    /// Current test index, −1 = interactive.
    pub test_index: i32,
}

impl Session {
    /// New session; nothing assembled yet.
    pub fn new(level: Option<Level>) -> Self {
        Session {
            level,
            test_index: -1,
            ..Session::default()
        }
    }

    /// Assembles the player's code with the level's library and filter. Later phase.
    pub fn assemble(&mut self, _source: &str) -> AsmReport {
        let _filter = self
            .level
            .as_ref()
            .map(|l| l.filter.clone())
            .unwrap_or(MnemonicFilter::All);
        AsmReport::default()
    }

    /// Fresh machine for test `test` (−1 = interactive). Later phase.
    pub fn reset(&mut self, _test: i32) -> Status {
        Status::NotAssembled
    }

    /// `n` single steps (breakpoints ignored). Later phase.
    pub fn step(&mut self, _n: u32) -> Status {
        Status::NotAssembled
    }

    /// Runs a time slice of at most `max_cycles` cycles. Later phase.
    pub fn run(&mut self, _max_cycles: u32) -> Status {
        Status::NotAssembled
    }

    /// `goto(steps − n)`, not below 0. Later phase.
    pub fn back(&mut self, _n: u64) -> Status {
        Status::NotAssembled
    }

    /// Restores the state after exactly `step` steps. Later phase.
    pub fn goto(&mut self, _step: u64) -> Status {
        Status::NotAssembled
    }

    /// Jumps to the state directly before the last step that changed the target. Later phase.
    pub fn back_to_change(&mut self, _kind: ChangeKind, _target: u32) -> Status {
        Status::NotAssembled
    }

    /// Edits a register while paused (discards the future). Later phase.
    pub fn set_reg(&mut self, _reg: u32, _value: u32) -> Status {
        Status::NotAssembled
    }

    /// Sets or clears a breakpoint; `false` if `addr` is not an instruction address. Later phase.
    pub fn set_breakpoint(&mut self, _addr: u32, _on: bool) -> bool {
        false
    }

    /// Removes all breakpoints.
    pub fn clear_breakpoints(&mut self) {
        self.breakpoints.clear();
    }

    /// Appends a number to the interactive input. Later phase.
    pub fn push_int(&mut self, _v: i32) {}

    /// Appends bytes to the interactive text input. Later phase.
    pub fn push_text(&mut self, _bytes: &[u8]) {}

    /// Closes the interactive input. Later phase.
    pub fn close_input(&mut self) {}

    /// Current machine state, if a machine exists.
    pub fn state(&self) -> Option<&MachineState> {
        self.machine.as_ref().map(|m| &m.state)
    }

    /// `digest()` as 16 lower-case hex characters. Later phase.
    pub fn digest(&self) -> String {
        String::new()
    }

    /// Executions per source line of the player's file: `[(line, count)]`. Later phase.
    pub fn line_counts(&self) -> Vec<(u32, u64)> {
        Vec::new()
    }

    /// The fp frame chain. Later phase.
    pub fn frames(&self) -> Vec<Frame> {
        Vec::new()
    }

    /// Evaluates the current test like one entry of `run_tests`. Later phase.
    pub fn check(&self) -> Option<TestResult> {
        None
    }

    /// Runs all tests of the level. Later phase.
    pub fn run_tests(&self) -> Option<TestReport> {
        None
    }
}
