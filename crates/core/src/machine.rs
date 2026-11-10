//! Machine state, tapes and the run loop.
//!
//! The split between [`MachineState`] and [`Tapes`] is fixed by the time-travel debugger:
//! `MachineState` is cloned into snapshots (it holds only the *length* of the output tape), while
//! the tapes only ever grow and are shared by all snapshots. Rewinding truncates the visible output
//! to `state.out_len`.
//!
use crate::consts::{DEFAULT_BUDGET, ENTRY, STACK_TOP};
use crate::cpu;
use crate::fault::{Access, Fault, FaultKind};
use crate::heap::{Heap, HeapCheck};
use crate::isa::Reg;
use crate::mem::Memory;
use crate::program::Program;
use crate::rng::rand_state_from_seed;

/// Run status. The numeric values are the ABI status codes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Status {
    /// Ready to execute the instruction at `pc`.
    #[default]
    Paused = 0,
    /// `halt` executed (or a function test reached `__trap`).
    Halted = 1,
    /// Stopped by a fault (see `MachineState::fault`).
    Fault = 2,
    /// Stopped before an instruction with a breakpoint.
    Breakpoint = 3,
    /// Waiting for interactive input.
    WaitingInput = 4,
    /// The cycle budget is used up.
    BudgetExceeded = 5,
    /// No successfully assembled program.
    NotAssembled = 6,
}

/// ABI value for a handle or argument error.
pub const STATUS_ERROR: u32 = 0xFFFF_FFFF;

impl Status {
    /// ABI status code.
    pub const fn code(self) -> u32 {
        self as u32
    }

    /// Status for an ABI code.
    pub const fn from_code(code: u32) -> Option<Status> {
        Some(match code {
            0 => Status::Paused,
            1 => Status::Halted,
            2 => Status::Fault,
            3 => Status::Breakpoint,
            4 => Status::WaitingInput,
            5 => Status::BudgetExceeded,
            6 => Status::NotAssembled,
            _ => return None,
        })
    }

    /// Upper-case name as in the docs (`PAUSED`, …), used as `check.not_halted {status}`.
    pub const fn name(self) -> &'static str {
        match self {
            Status::Paused => "PAUSED",
            Status::Halted => "HALTED",
            Status::Fault => "FAULT",
            Status::Breakpoint => "BREAKPOINT",
            Status::WaitingInput => "WAITING_INPUT",
            Status::BudgetExceeded => "BUDGET_EXCEEDED",
            Status::NotAssembled => "NOT_ASSEMBLED",
        }
    }
}

/// Per-run configuration (from the level and the test).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MachineConfig {
    /// Level switch `heap_check`.
    pub heap_check: HeapCheck,
    /// Seed for `RAND` (0 → 0x9E3779B9).
    pub seed: u32,
    /// Cycle budget; the run stops with [`Status::BudgetExceeded`] when `cycles >= budget` before
    /// an instruction.
    pub budget: u64,
    /// `true` for interactive runs (sandbox, test index −1): empty open input → `WAITING_INPUT`.
    /// `false` in test mode: input is closed from the start.
    pub interactive: bool,
}

impl Default for MachineConfig {
    fn default() -> Self {
        MachineConfig {
            heap_check: HeapCheck::Strict,
            seed: 0,
            budget: DEFAULT_BUDGET,
            interactive: false,
        }
    }
}

/// Input and output tapes. They only grow; snapshots store read positions and the output length.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Tapes {
    /// Output entry kinds ([`crate::consts::OUT_KIND_INT`] / [`crate::consts::OUT_KIND_BYTE`]).
    pub out_kinds: Vec<u8>,
    /// Output entry values (numbers, or bytes 0…255).
    pub out_vals: Vec<i32>,
    /// Number input channel (`READ_INT`).
    pub in_ints: Vec<i32>,
    /// Text input channel (`READ_CHAR`).
    pub in_text: Vec<u8>,
    /// `true` once no more input can arrive (always in test mode).
    pub input_closed: bool,
}

impl Tapes {
    /// Empty tapes.
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends an output entry at position `out_len`, dropping any entries beyond it first
    /// (they belong to an abandoned future after time travel). Returns the new length.
    pub fn push_output(&mut self, out_len: u32, kind: u8, value: i32) -> u32 {
        let n = out_len as usize;
        self.out_kinds.truncate(n);
        self.out_vals.truncate(n);
        self.out_kinds.push(kind);
        self.out_vals.push(value);
        out_len + 1
    }
}

/// Everything that a snapshot must contain. `Clone` produces a snapshot; [`MachineState::restore_from`]
/// restores one in place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MachineState {
    /// r0–r7, fp, sp (index = [`Reg::index`]).
    pub regs: [u32; Reg::COUNT],
    pub pc: u32,
    /// Flag word (bits [`crate::isa::FLAG_Z`] …).
    pub flags: u32,
    pub mem: Memory,
    pub heap: Heap,
    /// xorshift32 state of `RAND`.
    pub rng: u32,
    /// Completed instructions.
    pub steps: u64,
    /// Consumed cycles.
    pub cycles: u64,
    /// Next index into `Tapes::in_ints`.
    pub in_int_pos: u32,
    /// Next index into `Tapes::in_text`.
    pub in_text_pos: u32,
    /// Visible length of the output tape.
    pub out_len: u32,
    pub status: Status,
    /// Set exactly when `status == Status::Fault`.
    pub fault: Option<Fault>,
}

impl MachineState {
    /// Fresh machine with the program loaded: registers 0, `sp = 0xF000`, `fp = 0`,
    /// `pc = 0x0100`, flags 0, `RAND` seeded from `cfg.seed`, status `Paused`.
    pub fn new(program: &Program, cfg: &MachineConfig) -> Self {
        let mut mem = Memory::new();
        mem.load_program(program);
        let mut regs = [0u32; Reg::COUNT];
        regs[Reg::Sp.index()] = STACK_TOP;
        MachineState {
            regs,
            pc: ENTRY,
            flags: 0,
            mem,
            heap: Heap::new(),
            rng: rand_state_from_seed(cfg.seed),
            steps: 0,
            cycles: 0,
            in_int_pos: 0,
            in_text_pos: 0,
            out_len: 0,
            status: Status::Paused,
            fault: None,
        }
    }

    /// Register value.
    pub fn reg(&self, r: Reg) -> u32 {
        self.regs[r.index()]
    }

    /// Sets a register.
    pub fn set_reg(&mut self, r: Reg, value: u32) {
        self.regs[r.index()] = value;
    }

    /// Restores a snapshot in place: memory and shadow buffers keep their addresses.
    pub fn restore_from(&mut self, other: &MachineState) {
        self.regs = other.regs;
        self.pc = other.pc;
        self.flags = other.flags;
        self.mem.copy_from(&other.mem);
        self.heap.copy_from(&other.heap);
        self.rng = other.rng;
        self.steps = other.steps;
        self.cycles = other.cycles;
        self.in_int_pos = other.in_int_pos;
        self.in_text_pos = other.in_text_pos;
        self.out_len = other.out_len;
        self.status = other.status;
        self.fault = other.fault;
    }

    /// FNV-1a-64 over the whole state. The field order is part of the differential test between
    /// native and WASM and must not change once defined:
    ///
    /// 1. registers r0…r7, fp, sp (u32 each), pc, flags
    /// 2. memory (65536 bytes), code end (u32)
    /// 3. heap shadow (4096 bytes), number of blocks (u32), per block addr, size, live (u8),
    ///    alloc_pc
    /// 4. RNG state (u32), steps (u64), cycles (u64), in_int_pos, in_text_pos, out_len (u32 each)
    /// 5. status code (u32); fault: 0 (u8) or 1 (u8) + code, first argument, access/size, pc
    ///    (u32 each) and step (u64)
    /// 6. output tape entries `0..out_len`: kind (u8), value (i32)
    ///
    /// All integers little-endian.
    pub fn digest(&self, tapes: &Tapes) -> u64 {
        let mut h = Fnv::new();
        for r in self.regs {
            h.u32(r);
        }
        h.u32(self.pc);
        h.u32(self.flags);
        h.bytes(self.mem.bytes());
        h.u32(self.mem.code_end());
        h.bytes(self.heap.shadow());
        h.u32(self.heap.blocks().len() as u32);
        for b in self.heap.blocks() {
            h.u32(b.addr);
            h.u32(b.size);
            h.bytes(&[b.live as u8]);
            h.u32(b.alloc_pc);
        }
        h.u32(self.rng);
        h.u64(self.steps);
        h.u64(self.cycles);
        h.u32(self.in_int_pos);
        h.u32(self.in_text_pos);
        h.u32(self.out_len);
        h.u32(self.status.code());
        match self.fault {
            None => h.bytes(&[0]),
            Some(f) => {
                h.bytes(&[1]);
                h.u32(f.kind.code());
                h.u32(f.kind.addr());
                h.u32(fault_extra(&f.kind));
                h.u32(f.pc);
                h.u64(f.step);
            }
        }
        for i in 0..self.out_len as usize {
            h.bytes(&[tapes.out_kinds.get(i).copied().unwrap_or(0)]);
            h.u32(tapes.out_vals.get(i).copied().unwrap_or(0) as u32);
        }
        h.finish()
    }
}

/// Second fault argument for the digest: access (0 = r, 1 = w) or size, else 0.
fn fault_extra(kind: &FaultKind) -> u32 {
    match *kind {
        FaultKind::NullAccess { access, .. }
        | FaultKind::OutOfRange { access, .. }
        | FaultKind::GuardPage { access, .. } => match access {
            Access::Read => 0,
            Access::Write => 1,
        },
        FaultKind::Misaligned { size, .. } => size,
        _ => 0,
    }
}

/// FNV-1a, 64 bit.
struct Fnv(u64);

impl Fnv {
    const OFFSET: u64 = 0xCBF2_9CE4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01B3;

    fn new() -> Self {
        Fnv(Self::OFFSET)
    }

    fn bytes(&mut self, data: &[u8]) {
        let mut h = self.0;
        for &b in data {
            h ^= b as u64;
            h = h.wrapping_mul(Self::PRIME);
        }
        self.0 = h;
    }

    fn u32(&mut self, v: u32) {
        self.bytes(&v.to_le_bytes());
    }

    fn u64(&mut self, v: u64) {
        self.bytes(&v.to_le_bytes());
    }

    fn finish(&self) -> u64 {
        self.0
    }
}

/// Hook into [`Machine::run_observed`]: breakpoints and per-instruction bookkeeping (line
/// counters, `hottest_line`). Both methods default to doing nothing, so the plain run loop pays
/// nothing for them.
pub trait RunObserver {
    /// Called before every instruction except the first one of the call; `true` stops the run
    /// with [`Status::Breakpoint`] before the instruction at `pc` executes.
    #[inline]
    fn should_break(&mut self, _pc: u32) -> bool {
        false
    }

    /// Called after every completed instruction (including `halt`) with its address.
    #[inline]
    fn executed(&mut self, _pc: u32) {}
}

/// Observer that does nothing (plain `run`).
#[derive(Clone, Copy, Debug, Default)]
pub struct NoObserver;

impl RunObserver for NoObserver {}

/// Observer that stops at the addresses of a sorted breakpoint list.
#[derive(Clone, Copy, Debug)]
pub struct Breakpoints<'a>(pub &'a [u32]);

impl RunObserver for Breakpoints<'_> {
    fn should_break(&mut self, pc: u32) -> bool {
        self.0.binary_search(&pc).is_ok()
    }
}

/// A machine with its tapes and configuration: the unit used by tests and the CLI.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Machine {
    pub state: MachineState,
    pub tapes: Tapes,
    pub config: MachineConfig,
}

impl Machine {
    /// Fresh machine for `program`. In test mode (`!config.interactive`) the input is closed.
    pub fn new(program: &Program, config: MachineConfig) -> Self {
        let tapes = Tapes {
            input_closed: !config.interactive,
            ..Tapes::default()
        };
        Machine {
            state: MachineState::new(program, &config),
            tapes,
            config,
        }
    }

    /// Executes one instruction (ignores breakpoints, respects the budget). A machine that is
    /// halted, faulted or out of budget does nothing and returns its status; `Breakpoint` and
    /// `WaitingInput` resume (a still-empty input just yields `WaitingInput` again).
    pub fn step(&mut self) -> Status {
        if !self.resume() {
            return self.state.status;
        }
        if self.state.cycles >= self.config.budget {
            self.state.status = Status::BudgetExceeded;
            return Status::BudgetExceeded;
        }
        cpu::step(&mut self.state, &mut self.tapes, &self.config)
    }

    /// Runs until the status is no longer `Paused`, the budget is exhausted, or at least
    /// `max_cycles` more cycles were consumed (time slice; returns `Paused` then). At least one
    /// instruction is attempted per call, so `max_cycles = 0` behaves like a single step.
    pub fn run(&mut self, max_cycles: u64) -> Status {
        self.run_observed(max_cycles, &mut NoObserver)
    }

    /// [`Machine::run`] with a hook for breakpoints and per-instruction bookkeeping (phase 2:
    /// sessions). The observer is never asked about the first instruction of the call, so a run
    /// started on a breakpoint moves on.
    pub fn run_observed<O: RunObserver>(&mut self, max_cycles: u64, obs: &mut O) -> Status {
        if !self.resume() {
            return self.state.status;
        }
        let start = self.state.cycles;
        let budget = self.config.budget;
        let mut first = true;
        loop {
            if self.state.cycles >= budget {
                self.state.status = Status::BudgetExceeded;
                return Status::BudgetExceeded;
            }
            let pc = self.state.pc;
            if !first && obs.should_break(pc) {
                self.state.status = Status::Breakpoint;
                return Status::Breakpoint;
            }
            first = false;
            let steps = self.state.steps;
            let status = cpu::step(&mut self.state, &mut self.tapes, &self.config);
            if self.state.steps != steps {
                obs.executed(pc);
            }
            if status != Status::Paused {
                return status;
            }
            if self.state.cycles - start >= max_cycles {
                return Status::Paused;
            }
        }
    }

    /// Prepares a stopped-but-resumable machine for the next instruction. `false` if the run is
    /// over (halted, faulted, budget exceeded, not assembled).
    fn resume(&mut self) -> bool {
        match self.state.status {
            Status::Paused => true,
            Status::Breakpoint | Status::WaitingInput => {
                self.state.status = Status::Paused;
                true
            }
            Status::Halted | Status::Fault | Status::BudgetExceeded | Status::NotAssembled => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_state() {
        let m = Machine::new(&Program::default(), MachineConfig::default());
        assert_eq!(m.state.pc, 0x0100);
        assert_eq!(m.state.reg(Reg::Sp), 0xF000);
        assert_eq!(m.state.reg(Reg::Fp), 0);
        assert_eq!(m.state.status, Status::Paused);
        assert!(m.tapes.input_closed);
        assert_eq!(m.state.rng, 0x9E37_79B9);
    }

    #[test]
    fn restore_keeps_buffer_addresses() {
        let mut a = MachineState::new(&Program::default(), &MachineConfig::default());
        let mut b = a.clone();
        b.mem.bytes_mut()[0x4000] = 7;
        b.pc = 0x104;
        let ptr = a.mem.bytes().as_ptr();
        let shadow = a.heap.shadow().as_ptr();
        a.restore_from(&b);
        assert_eq!(a, b);
        assert_eq!(a.mem.bytes().as_ptr(), ptr);
        assert_eq!(a.heap.shadow().as_ptr(), shadow);
    }

    #[test]
    fn output_truncates_abandoned_future() {
        let mut t = Tapes::new();
        let mut len = t.push_output(0, 0, 1);
        len = t.push_output(len, 0, 2);
        assert_eq!(len, 2);
        let len = t.push_output(1, 1, 65);
        assert_eq!(len, 2);
        assert_eq!(t.out_vals, vec![1, 65]);
        assert_eq!(t.out_kinds, vec![0, 1]);
    }

    #[test]
    fn status_codes() {
        for c in 0..7 {
            assert_eq!(Status::from_code(c).map(Status::code), Some(c));
        }
        assert_eq!(Status::from_code(7), None);
    }
}
