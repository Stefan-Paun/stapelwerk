//! Machine state, tapes and the run loop.
//!
//! The split between [`MachineState`] and [`Tapes`] is fixed by the time-travel debugger:
//! `MachineState` is cloned into snapshots (it holds only the *length* of the output tape), while
//! the tapes only ever grow and are shared by all snapshots. Rewinding truncates the visible output
//! to `state.out_len`.
//!
//! PHASE 1A owns this file. Placeholders are marked `PHASE 1A`.

use crate::consts::{DEFAULT_BUDGET, ENTRY, STACK_TOP};
use crate::cpu;
use crate::fault::Fault;
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

    /// FNV-1a-64 over the whole state (registers, flags, pc, memory, heap blocks and shadow,
    /// RNG, counters, status, output up to `out_len`, read positions). The field order is part of
    /// the differential test between native and WASM and must not change once defined. PHASE 1A.
    pub fn digest(&self, _tapes: &Tapes) -> u64 {
        0
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

    /// Executes one instruction (ignores breakpoints, respects the budget). PHASE 1A.
    pub fn step(&mut self) -> Status {
        cpu::step(&mut self.state, &mut self.tapes, &self.config)
    }

    /// Runs until the status is no longer `Paused`, the budget is exhausted, or at least
    /// `max_cycles` more cycles were consumed (time slice; returns `Paused` then). PHASE 1A.
    pub fn run(&mut self, _max_cycles: u64) -> Status {
        self.state.status
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
