//! System calls (`sys n`). They change only r0 (and the heap, tapes and RNG state); flags are
//! unchanged; each costs 4 cycles (charged by [`crate::cpu::step`]).
//!
//! Semantics: `docs/ISA.md` ("System calls").

use crate::consts::{
    OUT_KIND_BYTE, OUT_KIND_INT, OUTPUT_LIMIT, STRING_LIMIT, SYS_ALLOC, SYS_FREE, SYS_RAND,
    SYS_READ_CHAR, SYS_READ_INT, SYS_WRITE_CHAR, SYS_WRITE_INT, SYS_WRITE_STR,
};
use crate::cpu::check_data;
use crate::fault::{Access, FaultKind};
use crate::isa::Reg;
use crate::machine::{MachineConfig, MachineState, Tapes};
use crate::rng::xorshift32;

/// Result of a successful system call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SysOutcome {
    /// Completed; the caller advances `pc` and the counters.
    Done,
    /// Input is empty but still open (interactive mode): nothing changed, retry later.
    WaitInput,
}

/// `true` when no more input can arrive: always in test mode, interactively after closing.
fn input_closed(tapes: &Tapes, cfg: &MachineConfig) -> bool {
    tapes.input_closed || !cfg.interactive
}

/// Appends one output entry (room checked by the caller).
fn write_out(state: &mut MachineState, tapes: &mut Tapes, kind: u8, value: i32) {
    state.out_len = tapes.push_output(state.out_len, kind, value);
}

fn check_room(state: &MachineState, entries: u32) -> Result<(), FaultKind> {
    if state.out_len as u64 + entries as u64 > OUTPUT_LIMIT as u64 {
        Err(FaultKind::OutputLimit)
    } else {
        Ok(())
    }
}

/// Performs system call `n`. On `Err` nothing may have changed (precise faults).
/// Unknown numbers → `fault.unknown_syscall {n}`.
pub fn syscall(
    n: u16,
    state: &mut MachineState,
    tapes: &mut Tapes,
    cfg: &MachineConfig,
) -> Result<SysOutcome, FaultKind> {
    let r0 = state.regs[Reg::R0.index()];
    match n {
        SYS_READ_INT => match tapes.in_ints.get(state.in_int_pos as usize) {
            Some(&v) => {
                state.regs[Reg::R0.index()] = v as u32;
                state.in_int_pos += 1;
            }
            None if input_closed(tapes, cfg) => return Err(FaultKind::InputEnd),
            None => return Ok(SysOutcome::WaitInput),
        },
        SYS_WRITE_INT => {
            check_room(state, 1)?;
            write_out(state, tapes, OUT_KIND_INT, r0 as i32);
        }
        SYS_READ_CHAR => match tapes.in_text.get(state.in_text_pos as usize) {
            Some(&b) => {
                state.regs[Reg::R0.index()] = b as u32;
                state.in_text_pos += 1;
            }
            None if input_closed(tapes, cfg) => state.regs[Reg::R0.index()] = u32::MAX,
            None => return Ok(SysOutcome::WaitInput),
        },
        SYS_WRITE_CHAR => {
            check_room(state, 1)?;
            write_out(state, tapes, OUT_KIND_BYTE, (r0 & 0xFF) as i32);
        }
        SYS_WRITE_STR => {
            // Check the whole string (every byte like `ldb`) before writing anything.
            let mut len = None;
            for i in 0..=STRING_LIMIT {
                let a = r0.wrapping_add(i);
                check_data(&state.mem, &state.heap, cfg.heap_check, a, 1, Access::Read)?;
                if state.mem.bytes().get(a as usize).copied().unwrap_or(0) == 0 {
                    len = Some(i);
                    break;
                }
            }
            let len = len.ok_or(FaultKind::StringTooLong { addr: r0 })?;
            check_room(state, len)?;
            for i in 0..len {
                let b = state
                    .mem
                    .bytes()
                    .get(r0.wrapping_add(i) as usize)
                    .copied()
                    .unwrap_or(0);
                write_out(state, tapes, OUT_KIND_BYTE, b as i32);
            }
        }
        SYS_ALLOC => {
            let addr = state.heap.alloc(&mut state.mem, r0, state.pc);
            state.regs[Reg::R0.index()] = addr;
        }
        SYS_FREE => state.heap.free(&mut state.mem, r0)?,
        SYS_RAND => {
            state.rng = xorshift32(state.rng);
            state.regs[Reg::R0.index()] = state.rng;
        }
        _ => return Err(FaultKind::UnknownSyscall { n: n as u32 }),
    }
    Ok(SysOutcome::Done)
}
