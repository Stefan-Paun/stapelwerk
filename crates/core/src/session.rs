//! Debugger session with time travel (SPEC §14, `docs/ABI.md` "Session semantics").
//!
//! A [`Session`] owns the assembled program, one [`Machine`] (state + tapes) and the time-travel
//! bookkeeping: snapshots, the edit log, breakpoints and line counters. The WASM layer maps the
//! `sess_*` exports onto these methods one to one.
//!
//! # Timeline model
//!
//! "State `t`" is the machine after exactly `t` completed instructions **with the edits logged at
//! step `t` applied**. Moving forward to step `s` ("arriving") first takes a snapshot if `s` is a
//! multiple of the interval K and newer than the last snapshot (so a snapshot holds the state
//! *before* the edits at its step), then applies the edits logged at `s`. `goto(t)` restores the
//! newest usable snapshot ≤ `t`, applies its step's edits and replays to `t` without breakpoints;
//! replay is deterministic because the tapes only grow (input is only appended, a closed input
//! stays closed) and the edit log is replayed at its steps.
//!
//! A state reached by `goto` is PAUSED unless it is HALTED; faults, breakpoints, waiting for input
//! and an exhausted budget are not states of their own but happen when trying the next
//! instruction, so they reappear on the next `step`/`run`.
//!
//! # Stable buffers
//!
//! The machine (memory, heap shadow, output tapes) is created once per session and reused by every
//! `reset`, `assemble` and `goto`: restoring copies into the existing buffers, and the output
//! vectors reserve [`OUTPUT_LIMIT`] entries up front and are only truncated, so their addresses
//! never change. Only the first `out_len` output entries are visible; entries beyond belong to
//! the (deterministic) future and are overwritten when output is produced again.

mod call;
mod inspect;
mod lines;

pub use call::{
    CALLEE_SAVED, CallInfo, Clobbered, call_registers, prepare_call, run_to_end, start_test,
};
pub use inspect::{Frame, HeapBlockInfo, StateView, frame_chain, heap_blocks};
pub use lines::LineCounter;

use crate::asm::{MnemonicFilter, assemble};
use crate::consts::{
    DEFAULT_BUDGET, MAX_BUDGET, MAX_SNAPSHOTS, MEM_SIZE, OUTPUT_LIMIT, SCREEN_BYTES, SCREEN_START,
    SNAPSHOT_INTERVAL,
};
use crate::diag::Diagnostic;
use crate::disasm::{DisasmLine, disasm_range};
use crate::fault::Fault;
use crate::heap::HeapCheck;
use crate::isa::Reg;
use crate::level::MemSpec;
use crate::machine::{Machine, MachineConfig, MachineState, RunObserver, Status, Tapes};
use crate::program::{LineEntry, Program, Symbol};

/// `kind` argument of `back_to_change`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChangeKind {
    /// Register by index ([`REG_FLAGS`] and [`REG_PC`] included).
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

/// Register index of the flag word for `set_reg` / `back_to_change` (0–9 are r0–r7, fp, sp).
/// Only bits 0–3 (Z, N, C, V) are kept on `set_reg`.
pub const REG_FLAGS: u32 = 10;
/// Register index of `pc` for `set_reg` / `back_to_change`.
pub const REG_PC: u32 = 11;

/// Level settings a session needs (filled by the level code; [`Default`] = sandbox).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionConfig {
    /// `[library]` source (file 1), assembled together with the player's code.
    pub library: Option<String>,
    /// `allow` / `forbid`.
    pub filter: MnemonicFilter,
    pub heap_check: HeapCheck,
    /// Level seed (`RAND`, function-test registers); a test may override it.
    pub seed: u32,
    /// Cycle budget per run; a test may override it.
    pub budget: u64,
}

impl Default for SessionConfig {
    /// The sandbox: no library, no filter, strict heap, seed 0, budget [`MAX_BUDGET`].
    fn default() -> Self {
        SessionConfig {
            library: None,
            filter: MnemonicFilter::All,
            heap_check: HeapCheck::Strict,
            seed: 0,
            budget: MAX_BUDGET,
        }
    }
}

impl SessionConfig {
    /// Settings with the level default budget (200000) instead of the sandbox budget.
    pub fn level_defaults() -> Self {
        SessionConfig {
            budget: DEFAULT_BUDGET,
            ..SessionConfig::default()
        }
    }
}

/// Everything a `[test]` section contributes to the start of a run. Built by the level code from
/// a `level::TestCase` (plus the level's images); applied by [`Session::reset`] in this order on a
/// fresh machine:
///
/// 1. `screen` (`screen_init`), 2. `sets` (`set`/`setb`; words little-endian, symbols resolved
///    against the assembled program, unresolved ones skipped), 3. function tests only: random
///    r4–r7 and fp, 4. `regs` (`reg rX = v`), 5. function tests only: push `__trap`,
///    `pc = LABEL` (see [`prepare_call`]); a missing label gives a start fault instead.
///
/// Steps 2–5 are [`start_test`], shared with `runner::prepare_test`, so a test reset in the
/// debugger starts in exactly the state `run_tests` uses.
///
/// The input tapes get `input` (`in`) and `in_text` (`in_text`) and are closed (test mode).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TestSetup {
    /// Index of the test in the level (reported as `test_index`).
    pub index: u32,
    /// `in = …`
    pub input: Vec<i32>,
    /// `in_text = "…"`
    pub in_text: Vec<u8>,
    /// `reg rX = v`
    pub regs: Vec<(Reg, u32)>,
    /// `set` / `setb` (same type as in `level::TestCase::sets`).
    pub sets: Vec<MemSpec>,
    /// `screen_init`: the image's pixels (at most 4096 bytes, row-major), written to 0xF000.
    pub screen: Option<Vec<u8>>,
    /// `call = LABEL`: function test.
    pub call: Option<String>,
    /// `seed` override (else the level seed).
    pub seed: Option<u32>,
    /// `budget` override (else the level budget).
    pub budget: Option<u64>,
}

/// Errors of session calls (ABI: status 0xFFFFFFFF / `abi.bad_arg`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionError {
    /// Argument out of range (register index, unaligned word, address ≥ 64 KiB).
    BadArg,
    /// `set_reg` while the run is over (HALTED, FAULT, BUDGET_EXCEEDED).
    NotPaused,
}

#[derive(Clone, Debug)]
struct Snapshot {
    /// The state before the edits of its step (`state.steps` is the step).
    state: MachineState,
    /// Line counters at that step.
    counts: Vec<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Edit {
    step: u64,
    reg: u32,
    value: u32,
}

/// A debugger session for one level (or the sandbox).
#[derive(Clone, Debug)]
pub struct Session {
    config: SessionConfig,
    program: Option<Program>,
    /// Created once; its buffers are reused for the whole session (stable addresses).
    machine: Machine,
    /// Instruction addresses with a breakpoint, sorted.
    breakpoints: Vec<u32>,
    /// Current test index, −1 = interactive.
    test_index: i32,
    /// Set for function tests.
    call: Option<CallInfo>,
    lines: LineCounter,
    /// Sorted by step; the first one is always step 0.
    snapshots: Vec<Snapshot>,
    /// Snapshot interval K.
    interval: u64,
    /// How often the snapshots were thinned since the reset.
    thinned: u32,
    /// Sorted by step (stable for edits at the same step).
    edits: Vec<Edit>,
}

impl Default for Session {
    fn default() -> Self {
        Session::new(SessionConfig::default())
    }
}

/// Observer for one chunk of execution: step limit, trap, breakpoints, line counters.
struct Chunk<'a> {
    remaining: u64,
    trap: Option<u32>,
    breakpoints: &'a [u32],
    lines: &'a mut LineCounter,
}

impl RunObserver for Chunk<'_> {
    #[inline]
    fn should_break(&mut self, pc: u32) -> bool {
        self.remaining == 0
            || Some(pc) == self.trap
            || (!self.breakpoints.is_empty() && self.breakpoints.binary_search(&pc).is_ok())
    }

    #[inline]
    fn executed(&mut self, pc: u32) {
        self.remaining -= 1;
        self.lines.count(pc);
    }
}

/// `true` if the machine can execute another instruction.
fn resumable(status: Status) -> bool {
    matches!(
        status,
        Status::Paused | Status::Breakpoint | Status::WaitingInput
    )
}

fn read_reg(state: &MachineState, reg: u32) -> u32 {
    match reg {
        REG_FLAGS => state.flags,
        REG_PC => state.pc,
        r => state.regs.get(r as usize).copied().unwrap_or(0),
    }
}

fn write_reg(state: &mut MachineState, reg: u32, value: u32) {
    match reg {
        REG_FLAGS => state.flags = value & 0xF,
        REG_PC => state.pc = value,
        r => {
            if let Some(slot) = state.regs.get_mut(r as usize) {
                *slot = value;
            }
        }
    }
}

impl Session {
    /// New session; nothing assembled yet (status NOT_ASSEMBLED).
    pub fn new(config: SessionConfig) -> Self {
        let mut machine = Machine::new(&Program::default(), MachineConfig::default());
        machine.state.status = Status::NotAssembled;
        let t = &mut machine.tapes;
        t.out_kinds.reserve_exact(OUTPUT_LIMIT as usize);
        t.out_vals.reserve_exact(OUTPUT_LIMIT as usize);
        Session {
            config,
            program: None,
            machine,
            breakpoints: Vec::new(),
            test_index: -1,
            call: None,
            lines: LineCounter::default(),
            snapshots: Vec::new(),
            interval: SNAPSHOT_INTERVAL,
            thinned: 0,
            edits: Vec::new(),
        }
    }

    /// The sandbox: `Session::new(SessionConfig::default())`.
    pub fn sandbox() -> Self {
        Session::default()
    }

    // -----------------------------------------------------------------------------------------
    // Program and reset
    // -----------------------------------------------------------------------------------------

    /// Assembles the player's code with the level's library and filter. On success the program
    /// is loaded, breakpoints are cleared and an interactive run is reset (status PAUSED); on
    /// failure the old program is dropped (status NOT_ASSEMBLED) and all diagnostics returned.
    pub fn assemble(&mut self, source: &str) -> Result<(), Vec<Diagnostic>> {
        match assemble(source, self.config.library.as_deref(), &self.config.filter) {
            Ok(program) => {
                self.load_program(program);
                Ok(())
            }
            Err(diags) => {
                self.unload();
                Err(diags)
            }
        }
    }

    /// Loads an already assembled program (as [`Session::assemble`] does on success).
    pub fn load_program(&mut self, program: Program) {
        self.lines = LineCounter::new(&program);
        self.program = Some(program);
        self.breakpoints.clear();
        self.reset(None);
    }

    fn unload(&mut self) {
        self.program = None;
        self.call = None;
        self.breakpoints.clear();
        self.snapshots.clear();
        self.edits.clear();
        self.lines = LineCounter::default();
        self.test_index = -1;
        self.machine.state.status = Status::NotAssembled;
        self.machine.state.fault = None;
    }

    /// Fresh machine for a test (`Some`) or an interactive run (`None`: open, empty input,
    /// level seed and budget). Clears snapshots, edits and line counters; breakpoints stay.
    ///
    /// A test always starts exactly as in `runner::prepare_test` (see [`start_test`]): unresolved
    /// `set` addresses are skipped, bytes beyond 0xFFFF cut off, and a `call` label the program
    /// does not define gives a start fault (status FAULT, `fault.bad_jump_target`, step 0).
    pub fn reset(&mut self, setup: Option<TestSetup>) -> Status {
        let Some(program) = &self.program else {
            return Status::NotAssembled;
        };
        let interactive = setup.is_none();
        let setup = setup.unwrap_or_default();
        let cfg = MachineConfig {
            heap_check: self.config.heap_check,
            seed: setup.seed.unwrap_or(self.config.seed),
            budget: setup.budget.unwrap_or(self.config.budget),
            interactive,
        };
        let mut fresh = MachineState::new(program, &cfg);
        if let Some(img) = &setup.screen {
            let n = img.len().min(SCREEN_BYTES as usize);
            let start = SCREEN_START as usize;
            if let (Some(dst), Some(src)) = (
                fresh.mem.bytes_mut().get_mut(start..start + n),
                img.get(..n),
            ) {
                dst.copy_from_slice(src);
            }
        }
        let (call, start_fault) = start_test(
            &mut fresh,
            program,
            &setup.sets,
            &setup.regs,
            setup.call.as_deref(),
            cfg.seed,
        );
        if let Some(f) = start_fault {
            // Same result as `runner::run_prepared`: the run is over before its first step.
            fresh.status = Status::Fault;
            fresh.fault = Some(f);
        }

        let m = &mut self.machine;
        m.state.restore_from(&fresh);
        m.config = cfg;
        m.tapes.out_kinds.clear();
        m.tapes.out_vals.clear();
        m.tapes.in_ints.clear();
        m.tapes.in_ints.extend_from_slice(&setup.input);
        m.tapes.in_text.clear();
        m.tapes.in_text.extend_from_slice(&setup.in_text);
        m.tapes.input_closed = !interactive;
        self.call = call;
        self.test_index = if interactive {
            -1
        } else {
            setup.index.min(i32::MAX as u32) as i32
        };
        self.lines.clear();
        self.snapshots.clear();
        self.edits.clear();
        self.interval = SNAPSHOT_INTERVAL;
        self.thinned = 0;
        self.arrive();
        self.status()
    }

    // -----------------------------------------------------------------------------------------
    // Execution
    // -----------------------------------------------------------------------------------------

    /// Executes up to `n` instructions, ignoring breakpoints (stops early on halt, fault, budget,
    /// waiting for input, or the trap of a function test).
    pub fn step(&mut self, n: u64) -> Status {
        if self.program.is_none() {
            return Status::NotAssembled;
        }
        if n == 0 {
            return self.status();
        }
        let target = self.machine.state.steps.saturating_add(n);
        self.advance(target, u64::MAX, false)
    }

    /// Runs one time slice: until something stops the run, a breakpoint is reached (never before
    /// the first instruction of the call), or at least `max_cycles` cycles were used (PAUSED;
    /// `max_cycles = 0` executes one instruction). A slice that ends right before a breakpoint
    /// reports BREAKPOINT, so a breakpoint is never skipped at a slice boundary.
    pub fn run(&mut self, max_cycles: u64) -> Status {
        if self.program.is_none() {
            return Status::NotAssembled;
        }
        self.advance(u64::MAX, max_cycles, true)
    }

    /// `goto(steps − n)`, not below 0.
    pub fn back(&mut self, n: u64) -> Status {
        let t = self.machine.state.steps.saturating_sub(n);
        self.goto(t)
    }

    /// Moves to state `t` (see the module docs). `t` equal to the current step does nothing; a
    /// `t` beyond the end of the run stops where the run stops.
    pub fn goto(&mut self, t: u64) -> Status {
        if self.program.is_none() {
            return Status::NotAssembled;
        }
        let cur = self.machine.state.steps;
        if t == cur {
            return self.status();
        }
        // Newest snapshot ≤ t (the step-0 snapshot always exists).
        let idx = self
            .snapshots
            .partition_point(|s| s.state.steps <= t)
            .saturating_sub(1);
        if let Some(snap) = self.snapshots.get(idx) {
            let s = snap.state.steps;
            // A snapshot ahead of `cur` is only usable while the output tape still holds its
            // entries (the tape is always a prefix of this timeline's output).
            let usable_ahead =
                s > cur && self.machine.tapes.out_kinds.len() >= snap.state.out_len as usize;
            if t < cur || usable_ahead {
                self.restore(idx);
            }
        }
        self.advance(t, u64::MAX, false)
    }

    /// Goes to the state directly before the last step < now that changed a register
    /// (`kind = Register`, index 0–9, [`REG_FLAGS`], [`REG_PC`]), a memory word (4-aligned
    /// address) or a byte; step 0 if there is none. Edits count as changes of the step that
    /// leads into their state.
    pub fn back_to_change(
        &mut self,
        kind: ChangeKind,
        target: u32,
    ) -> Result<Status, SessionError> {
        let valid = match kind {
            ChangeKind::Register => target <= REG_PC,
            ChangeKind::Word => target.is_multiple_of(4) && target <= MEM_SIZE - 4,
            ChangeKind::Byte => target < MEM_SIZE,
        };
        if !valid {
            return Err(SessionError::BadArg);
        }
        if self.program.is_none() {
            return Ok(Status::NotAssembled);
        }
        let now = self.machine.state.steps;
        let mut seg_end = now;
        let mut j = self.snapshots.partition_point(|s| s.state.steps < now);
        // Scan the snapshot segments from the newest backwards; within a segment replay step by
        // step and remember the last step that changed the value.
        while j > 0 {
            j -= 1;
            let start = self.snapshots[j].state.steps;
            self.restore(j);
            let mut prev = self.watch(kind, target);
            let mut found = None;
            while self.machine.state.steps < seg_end {
                let before = self.machine.state.steps;
                self.advance(before + 1, u64::MAX, false);
                if self.machine.state.steps != before + 1 {
                    break;
                }
                let v = self.watch(kind, target);
                if v != prev {
                    found = Some(before);
                }
                prev = v;
            }
            if let Some(step) = found {
                return Ok(self.goto(step));
            }
            seg_end = start;
        }
        Ok(self.goto(0))
    }

    fn watch(&self, kind: ChangeKind, target: u32) -> u32 {
        let s = &self.machine.state;
        match kind {
            ChangeKind::Register => read_reg(s, target),
            ChangeKind::Word => s.mem.peek_u32(target),
            ChangeKind::Byte => s.mem.bytes().get(target as usize).copied().unwrap_or(0) as u32,
        }
    }

    /// Edits a register (0–9 = r0–r7, fp, sp; [`REG_FLAGS`]; [`REG_PC`]) while the run can
    /// continue (PAUSED, BREAKPOINT, WAITING_INPUT). Discards the future (snapshots and edits
    /// after now, output beyond `out_len`) and logs the edit at the current step.
    pub fn set_reg(&mut self, reg: u32, value: u32) -> Result<Status, SessionError> {
        if reg > REG_PC {
            return Err(SessionError::BadArg);
        }
        if self.program.is_none() {
            return Ok(Status::NotAssembled);
        }
        let status = self.status();
        if !resumable(status) {
            return Err(SessionError::NotPaused);
        }
        let t = self.machine.state.steps;
        self.snapshots.retain(|s| s.state.steps <= t);
        self.edits.retain(|e| e.step <= t);
        let n = self.machine.state.out_len as usize;
        self.machine.tapes.out_kinds.truncate(n);
        self.machine.tapes.out_vals.truncate(n);
        let value = if reg == REG_FLAGS { value & 0xF } else { value };
        self.edits.push(Edit {
            step: t,
            reg,
            value,
        });
        write_reg(&mut self.machine.state, reg, value);
        Ok(status)
    }

    /// Sets (`on`) or clears a breakpoint. `false` if `addr` is not the address of an
    /// instruction of the program (player or library; not the trap word).
    pub fn set_breakpoint(&mut self, addr: u32, on: bool) -> bool {
        if self
            .program
            .as_ref()
            .and_then(|p| p.line_of(addr))
            .is_none()
        {
            return false;
        }
        match (self.breakpoints.binary_search(&addr), on) {
            (Err(i), true) => self.breakpoints.insert(i, addr),
            (Ok(i), false) => {
                self.breakpoints.remove(i);
            }
            _ => {}
        }
        true
    }

    /// Removes all breakpoints.
    pub fn clear_breakpoints(&mut self) {
        self.breakpoints.clear();
    }

    /// Breakpoint addresses, sorted.
    pub fn breakpoints(&self) -> &[u32] {
        &self.breakpoints
    }

    /// Appends a number to the interactive input (ignored in test mode or once closed).
    pub fn push_int(&mut self, v: i32) -> Status {
        if self.input_open() {
            self.machine.tapes.in_ints.push(v);
        }
        self.status()
    }

    /// Appends bytes to the interactive text input (ignored in test mode or once closed).
    pub fn push_text(&mut self, bytes: &[u8]) -> Status {
        if self.input_open() {
            self.machine.tapes.in_text.extend_from_slice(bytes);
        }
        self.status()
    }

    /// Closes the interactive input for good.
    pub fn close_input(&mut self) -> Status {
        if self.program.is_some() {
            self.machine.tapes.input_closed = true;
        }
        self.status()
    }

    fn input_open(&self) -> bool {
        self.program.is_some()
            && self.machine.config.interactive
            && !self.machine.tapes.input_closed
    }

    // -----------------------------------------------------------------------------------------
    // Time-travel engine
    // -----------------------------------------------------------------------------------------

    /// Executes until step `target`, a stop (halt, fault, budget, input, trap), a breakpoint
    /// (`use_bp`) or the end of the cycle slice. Splits the work into chunks that end at snapshot
    /// boundaries and edit steps so both are handled exactly at their step.
    fn advance(&mut self, target: u64, max_cycles: u64, use_bp: bool) -> Status {
        let start_cycles = self.machine.state.cycles;
        let trap = self.call.map(|c| c.trap);
        let mut first = true;
        loop {
            let st = &self.machine.state;
            if st.steps >= target || !resumable(st.status) {
                return st.status;
            }
            let used = st.cycles - start_cycles;
            if !first {
                if use_bp && self.breakpoints.binary_search(&st.pc).is_ok() {
                    self.machine.state.status = Status::Breakpoint;
                    return Status::Breakpoint;
                }
                if used >= max_cycles {
                    return st.status;
                }
            }
            let steps = st.steps;
            let next_snap = (steps / self.interval + 1) * self.interval;
            let next_edit = self
                .edits
                .iter()
                .find(|e| e.step > steps)
                .map_or(u64::MAX, |e| e.step);
            let stop = target.min(next_snap).min(next_edit);
            let mut chunk = Chunk {
                remaining: stop - steps,
                trap,
                breakpoints: if use_bp { &self.breakpoints } else { &[] },
                lines: &mut self.lines,
            };
            self.machine
                .run_observed(max_cycles.saturating_sub(used), &mut chunk);
            first = false;
            let limit_reached = chunk.remaining == 0;
            let st = &mut self.machine.state;
            if trap == Some(st.pc)
                && matches!(
                    st.status,
                    Status::Paused | Status::Breakpoint | Status::BudgetExceeded
                )
            {
                // Function test: reaching the trap ends the run before its `halt` executes.
                st.status = Status::Halted;
            } else if limit_reached
                && matches!(st.status, Status::Breakpoint | Status::BudgetExceeded)
            {
                // Stopped for our own step limit; breakpoints and the budget are checked again
                // before the next instruction.
                st.status = Status::Paused;
            }
            if st.steps != steps {
                self.arrive();
            }
            let status = self.machine.state.status;
            if status != Status::Paused {
                return status;
            }
        }
    }

    /// Bookkeeping on reaching a new step: snapshot (before edits), then the step's edits.
    fn arrive(&mut self) {
        let s = self.machine.state.steps;
        let newer = self.snapshots.last().is_none_or(|l| l.state.steps < s);
        if s.is_multiple_of(self.interval) && newer {
            self.snapshots.push(Snapshot {
                state: self.machine.state.clone(),
                counts: self.lines.raw().to_vec(),
            });
            if self.snapshots.len() > MAX_SNAPSHOTS {
                let k2 = self.interval * 2;
                self.snapshots.retain(|s| s.state.steps % k2 == 0);
                self.interval = k2;
                self.thinned += 1;
            }
        }
        self.apply_edits(s);
    }

    fn apply_edits(&mut self, step: u64) {
        let from = self.edits.partition_point(|e| e.step < step);
        for e in self.edits.get(from..).unwrap_or(&[]) {
            if e.step != step {
                break;
            }
            write_reg(&mut self.machine.state, e.reg, e.value);
        }
    }

    /// Restores snapshot `idx` in place (stable buffers) and applies its step's edits.
    fn restore(&mut self, idx: usize) {
        let Some(snap) = self.snapshots.get(idx) else {
            return;
        };
        self.machine.state.restore_from(&snap.state);
        self.lines.restore_raw(&snap.counts);
        let step = snap.state.steps;
        self.apply_edits(step);
    }

    // -----------------------------------------------------------------------------------------
    // Queries
    // -----------------------------------------------------------------------------------------

    /// Current status (NOT_ASSEMBLED without a program).
    pub fn status(&self) -> Status {
        if self.program.is_none() {
            Status::NotAssembled
        } else {
            self.machine.state.status
        }
    }

    /// Completed instructions of the current state.
    pub fn steps(&self) -> u64 {
        self.machine.state.steps
    }

    /// The configuration the session was created with.
    pub fn config(&self) -> &SessionConfig {
        &self.config
    }

    /// The assembled program.
    pub fn program(&self) -> Option<&Program> {
        self.program.as_ref()
    }

    /// The current machine state.
    pub fn state(&self) -> &MachineState {
        &self.machine.state
    }

    /// The tapes (whole vectors; only the first `out_len` output entries are visible).
    pub fn tapes(&self) -> &Tapes {
        &self.machine.tapes
    }

    /// The machine configuration of the current run (seed, budget, interactive).
    pub fn machine_config(&self) -> &MachineConfig {
        &self.machine.config
    }

    /// Current test index, −1 = interactive.
    pub fn test_index(&self) -> i32 {
        self.test_index
    }

    /// The `StateView` record of the WASM ABI.
    pub fn view(&self) -> StateView {
        StateView::new(
            &self.machine.state,
            &self.machine.tapes,
            self.status(),
            self.test_index,
            self.machine.config.budget,
        )
    }

    /// The 65536 memory bytes (stable address for the whole session).
    pub fn memory(&self) -> &[u8] {
        self.machine.state.mem.bytes()
    }

    /// The 4096-byte heap shadow (stable address for the whole session).
    pub fn shadow(&self) -> &[u8] {
        self.machine.state.heap.shadow()
    }

    /// Visible output: kinds and values of the first `out_len` entries. The vectors behind them
    /// have capacity ≥ 8192 and never move.
    pub fn output(&self) -> (&[u8], &[i32]) {
        let n = self.machine.state.out_len as usize;
        let t = &self.machine.tapes;
        (
            t.out_kinds.get(..n).unwrap_or(&t.out_kinds),
            t.out_vals.get(..n).unwrap_or(&t.out_vals),
        )
    }

    /// The fault, if the status is FAULT.
    pub fn fault(&self) -> Option<Fault> {
        match self.status() {
            Status::Fault => self.machine.state.fault,
            _ => None,
        }
    }

    /// All symbols of the program (empty if not assembled).
    pub fn symbols(&self) -> &[Symbol] {
        self.program.as_ref().map_or(&[], |p| &p.symbols)
    }

    /// The line map of the program (empty if not assembled).
    pub fn linemap(&self) -> &[LineEntry] {
        self.program.as_ref().map_or(&[], |p| &p.linemap)
    }

    /// `[(line, count)]` executions per line of the player's file since the reset (lines with
    /// count 0 are omitted).
    pub fn line_counts(&self) -> Vec<(u32, u64)> {
        self.lines.line_counts()
    }

    /// The most executed line of the player's file, 0 if none (`run.budget_exceeded`).
    pub fn hottest_line(&self) -> u32 {
        self.lines.hottest_line()
    }

    /// The fp frame chain, innermost first.
    pub fn frames(&self) -> Vec<Frame> {
        frame_chain(&self.machine.state, self.program.as_ref())
    }

    /// Heap blocks with the line of their `ALLOC`.
    pub fn heap_blocks(&self) -> Vec<HeapBlockInfo> {
        heap_blocks(&self.machine.state, self.program.as_ref())
    }

    /// Disassembles `count` words (at most 4096) from `addr` out of the current memory.
    pub fn disasm(&self, addr: u32, count: u32) -> Vec<DisasmLine> {
        disasm_range(self.memory(), self.program.as_ref(), addr, count.min(4096))
    }

    /// FNV-1a-64 of the whole state.
    pub fn digest_u64(&self) -> u64 {
        self.machine.state.digest(&self.machine.tapes)
    }

    /// `digest()` as 16 lower-case hex characters.
    pub fn digest(&self) -> String {
        format!("{:016x}", self.digest_u64())
    }

    /// The prepared call of a function test.
    pub fn call_info(&self) -> Option<&CallInfo> {
        self.call.as_ref()
    }

    /// Function tests: the first callee-saved register (r4–r7, fp, sp) that differs from its
    /// value before the call, with before/after; `None` if all match or this is no function test.
    pub fn call_clobbered(&self) -> Option<Clobbered> {
        self.call
            .as_ref()
            .and_then(|c| c.clobbered(&self.machine.state))
    }

    /// Number of snapshots held.
    pub fn snapshot_count(&self) -> usize {
        self.snapshots.len()
    }

    /// Current snapshot interval K.
    pub fn snapshot_interval(&self) -> u64 {
        self.interval
    }

    /// How often the snapshots were thinned since the reset.
    pub fn thin_count(&self) -> u32 {
        self.thinned
    }
}
