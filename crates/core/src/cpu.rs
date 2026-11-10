//! Execution of single instructions.
//!
//! PHASE 1A owns this file. Placeholders are marked `PHASE 1A`; semantics: `docs/ISA.md`.
//! The ALU vectors in `tests/vectors/alu.txt` are the independent yardstick for [`alu`].

use crate::fault::FaultKind;
use crate::isa::AluOp;
use crate::machine::{MachineConfig, MachineState, Status, Tapes};

/// Pure ALU: computes `a op b` and the **complete new flag word** (Z, N, C, V as defined by
/// [`AluOp::flag_effect`]). For `cmp` the result is `a − b`, for `tst` `a & b` (the caller does not
/// write it back). Shift widths are taken `& 31`. Division or remainder by 0 returns
/// `Err(FaultKind::DivZero)`. `neg` is `alu(Sub, 0, x)`; `not` is computed by the caller with
/// [`crate::isa::FlagEffect::Logic`]. PHASE 1A.
pub fn alu(_op: AluOp, _a: u32, _b: u32) -> Result<(u32, u32), FaultKind> {
    Err(FaultKind::IllegalInstruction { word: 0 })
}

/// Executes one instruction at `state.pc`.
///
/// Contract:
/// - Only called while `state.status` is [`Status::Paused`] (the caller handles breakpoints and
///   the cycle budget).
/// - On success: registers/memory/flags/output updated, `pc` advanced, `steps += 1`,
///   `cycles += cost`; returns [`Status::Paused`], or [`Status::Halted`] after `halt` (pc stays on
///   the `halt` word, which counts as a step and costs 1 cycle).
/// - `READ_INT`/`READ_CHAR` on an empty but open input: nothing changes (no step, no cycles) and
///   [`Status::WaitingInput`] is returned; the instruction is retried after more input.
/// - On a fault: nothing changes except `state.status = Fault` and `state.fault = Some(..)` with
///   `pc` of the faulting instruction and `step = state.steps`; returns [`Status::Fault`].
///
/// PHASE 1A.
pub fn step(state: &mut MachineState, _tapes: &mut Tapes, _cfg: &MachineConfig) -> Status {
    state.status
}
