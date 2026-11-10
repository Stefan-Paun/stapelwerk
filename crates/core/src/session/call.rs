//! Function tests (`call = LABEL`, SPEC §12): preparing the call, running to the trap word and
//! checking the callee-saved registers. Shared by the session and the test runner, so both count
//! the same cycles and steps.

use crate::isa::Reg;
use crate::machine::{Machine, MachineState, RunObserver, Status};
use crate::rng::SplitMix64;

use super::lines::LineCounter;

/// Registers that must survive a function call, in reporting order (`check.abi_clobbered`).
pub const CALLEE_SAVED: [Reg; 6] = [Reg::R4, Reg::R5, Reg::R6, Reg::R7, Reg::Fp, Reg::Sp];

/// A prepared function call: where it returns to and the callee-saved values before the call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CallInfo {
    /// Address of the trap `halt` word (`__trap`); reaching it ends the run (HALTED).
    pub trap: u32,
    /// Address of the called label.
    pub target: u32,
    /// Values of [`CALLEE_SAVED`] before the call (`sp` before the return address was pushed).
    pub before: [u32; 6],
}

/// The first callee-saved register that a function test changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Clobbered {
    pub reg: Reg,
    pub before: u32,
    pub after: u32,
}

/// The random values for r4, r5, r6, r7 and fp of a function test: SplitMix64(`seed`) in that
/// order, fp re-drawn while 0. Must stay equal to `runner::function_test_regs`.
pub fn call_registers(seed: u32) -> [u32; 5] {
    let mut rng = SplitMix64::new(seed as u64);
    let mut v = [0u32; 5];
    for slot in &mut v {
        *slot = rng.next_u32();
    }
    while v[4] == 0 {
        v[4] = rng.next_u32();
    }
    v
}

/// Prepares a function test on a fresh `state`: r4–r7 and fp get [`call_registers`], then the
/// test's `reg` lines are applied, the callee-saved values are recorded, the trap address is pushed
/// as the return address and `pc = target`.
pub fn prepare_call(
    state: &mut MachineState,
    trap: u32,
    target: u32,
    seed: u32,
    regs: &[(Reg, u32)],
) -> CallInfo {
    let rnd = call_registers(seed);
    for (r, v) in [Reg::R4, Reg::R5, Reg::R6, Reg::R7, Reg::Fp]
        .iter()
        .zip(rnd)
    {
        state.set_reg(*r, v);
    }
    for &(r, v) in regs {
        state.set_reg(r, v);
    }
    let before = CALLEE_SAVED.map(|r| state.reg(r));
    let sp = state.reg(Reg::Sp).wrapping_sub(4);
    if let Some(dst) = state
        .mem
        .bytes_mut()
        .get_mut(sp as usize..(sp as usize).saturating_add(4))
    {
        dst.copy_from_slice(&trap.to_le_bytes());
    }
    state.set_reg(Reg::Sp, sp);
    state.pc = target;
    CallInfo {
        trap,
        target,
        before,
    }
}

impl CallInfo {
    /// The first of `r4`–`r7`, `fp`, `sp` whose value differs from before the call, if any.
    pub fn clobbered(&self, state: &MachineState) -> Option<Clobbered> {
        CALLEE_SAVED
            .iter()
            .zip(self.before)
            .find(|(r, before)| state.reg(**r) != *before)
            .map(|(r, before)| Clobbered {
                reg: *r,
                before,
                after: state.reg(*r),
            })
    }
}

/// Observer that stops before the trap word and counts lines.
struct TrapStop<'a> {
    trap: Option<u32>,
    lines: &'a mut LineCounter,
}

impl RunObserver for TrapStop<'_> {
    #[inline]
    fn should_break(&mut self, pc: u32) -> bool {
        Some(pc) == self.trap
    }

    #[inline]
    fn executed(&mut self, pc: u32) {
        self.lines.count(pc);
    }
}

/// Runs `machine` until it stops for good (halt, fault, budget, or waiting for input), counting
/// lines. With `trap = Some(addr)` (function test) reaching `addr` ends the run with HALTED
/// **before** the trap's `halt` word executes — the same rule the session uses, so `cycles` and
/// `steps` agree between `run_tests` and the debugger.
pub fn run_to_end(machine: &mut Machine, trap: Option<u32>, lines: &mut LineCounter) -> Status {
    let mut obs = TrapStop { trap, lines };
    loop {
        let status = machine.run_observed(u64::MAX, &mut obs);
        let at_trap = trap == Some(machine.state.pc);
        match status {
            Status::Breakpoint | Status::BudgetExceeded | Status::Paused if at_trap => {
                machine.state.status = Status::Halted;
                return Status::Halted;
            }
            Status::Paused => continue,
            s => return s,
        }
    }
}
