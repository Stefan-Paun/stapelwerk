//! System calls (`sys n`). They change only r0 (and the heap, tapes and RNG state); flags are
//! unchanged; each costs 4 cycles (charged by [`crate::cpu::step`]).
//!
//! PHASE 1A owns this file. Placeholders are marked `PHASE 1A`; semantics: `docs/ISA.md`
//! ("System calls").

use crate::fault::FaultKind;
use crate::machine::{MachineConfig, MachineState, Tapes};

/// Result of a successful system call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SysOutcome {
    /// Completed; the caller advances `pc` and the counters.
    Done,
    /// Input is empty but still open (interactive mode): nothing changed, retry later.
    WaitInput,
}

/// Performs system call `n`. On `Err` nothing may have changed (precise faults).
/// Unknown numbers → `fault.unknown_syscall {n}`. PHASE 1A.
pub fn syscall(
    n: u16,
    _state: &mut MachineState,
    _tapes: &mut Tapes,
    _cfg: &MachineConfig,
) -> Result<SysOutcome, FaultKind> {
    Err(FaultKind::UnknownSyscall { n: n as u32 })
}
