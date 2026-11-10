//! Execution of single instructions.
//!
//! Semantics: `docs/ISA.md`. The ALU vectors in `tests/vectors/alu.txt` are the independent
//! yardstick for [`alu`].
//!
//! Every instruction is executed in two phases: first everything that can fail is checked
//! (fetch, decode, operands, memory and stack rights, system call preconditions), then the state is
//! updated. A fault therefore never leaves a partial change behind (precise faults).

use crate::consts::{STACK_START, STACK_TOP};
use crate::fault::{Access, FaultKind};
use crate::heap::{Heap, HeapCheck};
use crate::isa::{self, AluOp, DecodeError, FLAG_C, FLAG_N, FLAG_V, FLAG_Z, Instr, Mem, Reg};
use crate::machine::{MachineConfig, MachineState, Status, Tapes};
use crate::mem::Memory;
use crate::sys::{self, SysOutcome};

#[inline]
fn zn(r: u32) -> u32 {
    (if r == 0 { FLAG_Z } else { 0 }) | (if r & 0x8000_0000 != 0 { FLAG_N } else { 0 })
}

#[inline]
fn bit(cond: bool, flag: u32) -> u32 {
    if cond { flag } else { 0 }
}

/// Pure ALU: computes `a op b` and the **complete new flag word** (Z, N, C, V as defined by
/// [`AluOp::flag_effect`]). For `cmp` the result is `a − b`, for `tst` `a & b` (the caller does not
/// write it back). Shift widths are taken `& 31`. Division or remainder by 0 returns
/// `Err(FaultKind::DivZero)`. `neg` is `alu(Sub, 0, x)`; `not` is computed by the caller with
/// [`crate::isa::FlagEffect::Logic`].
#[inline]
pub fn alu(op: AluOp, a: u32, b: u32) -> Result<(u32, u32), FaultKind> {
    let (r, c, v) = match op {
        AluOp::Add => {
            let (r, carry) = a.overflowing_add(b);
            (r, carry, ((a ^ r) & (b ^ r)) >> 31 != 0)
        }
        AluOp::Sub | AluOp::Cmp => {
            let r = a.wrapping_sub(b);
            (r, a < b, ((a ^ b) & (a ^ r)) >> 31 != 0)
        }
        AluOp::Mul => {
            let r = a.wrapping_mul(b);
            let wide_u = a as u64 * b as u64;
            let wide_s = a as i32 as i64 * b as i32 as i64;
            (r, wide_u > u32::MAX as u64, wide_s != wide_s as i32 as i64)
        }
        AluOp::Div | AluOp::Rem => {
            if b == 0 {
                return Err(FaultKind::DivZero);
            }
            let (sa, sb) = (a as i32, b as i32);
            let overflow = sa == i32::MIN && sb == -1;
            let r = match op {
                AluOp::Div => sa.wrapping_div(sb),
                _ => sa.wrapping_rem(sb),
            };
            (r as u32, false, overflow)
        }
        AluOp::Divu | AluOp::Remu => {
            if b == 0 {
                return Err(FaultKind::DivZero);
            }
            let r = if op == AluOp::Divu { a / b } else { a % b };
            (r, false, false)
        }
        AluOp::And | AluOp::Tst => (a & b, false, false),
        AluOp::Or => (a | b, false, false),
        AluOp::Xor => (a ^ b, false, false),
        AluOp::Shl | AluOp::Shr | AluOp::Sar => {
            let k = b & 31;
            if k == 0 {
                (a, false, false)
            } else {
                let (r, out) = match op {
                    AluOp::Shl => (a << k, (a >> (32 - k)) & 1),
                    AluOp::Shr => (a >> k, (a >> (k - 1)) & 1),
                    _ => (((a as i32) >> k) as u32, (a >> (k - 1)) & 1),
                };
                (r, out != 0, false)
            }
        }
    };
    Ok((r, zn(r) | bit(c, FLAG_C) | bit(v, FLAG_V)))
}

/// Combined data-access check: segment rights and alignment ([`Memory::check`]), then the heap
/// shadow ([`Heap::check_access`], only for `heap_check = strict`).
#[inline]
pub fn check_data(
    mem: &Memory,
    heap: &Heap,
    mode: HeapCheck,
    addr: u32,
    size: u32,
    access: Access,
) -> Result<(), FaultKind> {
    mem.check(addr, size, access)?;
    heap.check_access(addr, size, access, mode)
}

/// Stack check for `push` and `call` (value of `sp` before the instruction).
#[inline]
fn check_push(sp: u32) -> Result<(), FaultKind> {
    if !sp.is_multiple_of(4) {
        Err(FaultKind::StackMisaligned { sp })
    } else if sp < STACK_START + 4 {
        Err(FaultKind::StackOverflow { sp })
    } else if sp > STACK_TOP {
        Err(FaultKind::StackUnderflow { sp })
    } else {
        Ok(())
    }
}

/// Stack check for `pop` and `ret` (value of `sp` before the instruction).
#[inline]
fn check_pop(sp: u32) -> Result<(), FaultKind> {
    if !sp.is_multiple_of(4) {
        Err(FaultKind::StackMisaligned { sp })
    } else if sp >= STACK_TOP {
        Err(FaultKind::StackUnderflow { sp })
    } else if sp < STACK_START {
        Err(FaultKind::StackOverflow { sp })
    } else {
        Ok(())
    }
}

#[inline]
fn jump_target(mem: &Memory, target: u32) -> Result<u32, FaultKind> {
    if mem.is_code_target(target) {
        Ok(target)
    } else {
        Err(FaultKind::BadJumpTarget { target })
    }
}

/// What a successfully checked instruction does after its checks; applied by [`step`].
enum Effect {
    /// Fall through to the next instruction.
    Next,
    /// Continue at this address.
    Goto(u32),
    /// `halt`: pc stays.
    Halt,
    /// `sys` found empty open input.
    Wait,
}

#[inline]
fn set(state: &mut MachineState, r: Reg, v: u32) {
    state.regs[r.index()] = v;
}

/// Loads the value for `ld`/`ldb`/`ldbs` after all checks.
#[inline]
fn load(state: &MachineState, cfg: &MachineConfig, m: Mem, size: u32) -> Result<u32, FaultKind> {
    let addr = m.effective_address(&state.regs);
    check_data(
        &state.mem,
        &state.heap,
        cfg.heap_check,
        addr,
        size,
        Access::Read,
    )?;
    Ok(if size == 4 {
        state.mem.peek_u32(addr)
    } else {
        state.mem.bytes().get(addr as usize).copied().unwrap_or(0) as u32
    })
}

/// Checks and executes the decoded instruction. On `Err` nothing has changed.
#[inline]
fn execute(
    state: &mut MachineState,
    tapes: &mut Tapes,
    cfg: &MachineConfig,
    instr: Instr,
) -> Result<Effect, FaultKind> {
    let pc = state.pc;
    let r = |state: &MachineState, reg: Reg| state.regs[reg.index()];
    match instr {
        Instr::Halt => Ok(Effect::Halt),
        Instr::Nop => Ok(Effect::Next),
        Instr::Sys { n } => match sys::syscall(n, state, tapes, cfg)? {
            SysOutcome::Done => Ok(Effect::Next),
            SysOutcome::WaitInput => Ok(Effect::Wait),
        },
        Instr::Ret => {
            let sp = r(state, Reg::Sp);
            check_pop(sp)?;
            let target = state.mem.read_u32(sp)?;
            if !state.mem.is_code_target(target) {
                return Err(FaultKind::BadReturnAddress { target });
            }
            set(state, Reg::Sp, sp + 4);
            Ok(Effect::Goto(target))
        }
        Instr::Mov { rd, rs } => {
            let v = r(state, rs);
            set(state, rd, v);
            Ok(Effect::Next)
        }
        Instr::MovI { rd, imm } => {
            set(state, rd, imm as i32 as u32);
            Ok(Effect::Next)
        }
        Instr::MovHi { rd, imm } => {
            let v = ((imm as u32) << 16) | (r(state, rd) & 0xFFFF);
            set(state, rd, v);
            Ok(Effect::Next)
        }
        Instr::Lea { rd, m } => {
            let v = m.effective_address(&state.regs);
            set(state, rd, v);
            Ok(Effect::Next)
        }
        Instr::Alu { op, rd, rs1, rs2 } => {
            let (v, flags) = alu(op, r(state, rs1), r(state, rs2))?;
            if op.writes_rd() {
                set(state, rd, v);
            }
            state.flags = flags;
            Ok(Effect::Next)
        }
        Instr::AluI { op, rd, rs1, imm } => {
            let (v, flags) = alu(op, r(state, rs1), imm as u32)?;
            if op.writes_rd() {
                set(state, rd, v);
            }
            state.flags = flags;
            Ok(Effect::Next)
        }
        Instr::Neg { rd, rs } => {
            let (v, flags) = alu(AluOp::Sub, 0, r(state, rs))?;
            set(state, rd, v);
            state.flags = flags;
            Ok(Effect::Next)
        }
        Instr::Not { rd, rs } => {
            let v = !r(state, rs);
            set(state, rd, v);
            state.flags = zn(v);
            Ok(Effect::Next)
        }
        Instr::Ld { rd, m } => {
            let v = load(state, cfg, m, 4)?;
            set(state, rd, v);
            Ok(Effect::Next)
        }
        Instr::Ldb { rd, m } => {
            let v = load(state, cfg, m, 1)?;
            set(state, rd, v);
            Ok(Effect::Next)
        }
        Instr::Ldbs { rd, m } => {
            let v = load(state, cfg, m, 1)? as u8 as i8 as i32 as u32;
            set(state, rd, v);
            Ok(Effect::Next)
        }
        Instr::St { rs, m } => {
            let addr = m.effective_address(&state.regs);
            check_data(
                &state.mem,
                &state.heap,
                cfg.heap_check,
                addr,
                4,
                Access::Write,
            )?;
            let v = r(state, rs);
            state.mem.write_u32(addr, v)?;
            Ok(Effect::Next)
        }
        Instr::Stb { rs, m } => {
            let addr = m.effective_address(&state.regs);
            check_data(
                &state.mem,
                &state.heap,
                cfg.heap_check,
                addr,
                1,
                Access::Write,
            )?;
            let v = r(state, rs) as u8;
            state.mem.write_u8(addr, v)?;
            Ok(Effect::Next)
        }
        Instr::Push { rs } => {
            let sp = r(state, Reg::Sp);
            check_push(sp)?;
            // `push sp` stores the old value.
            let v = r(state, rs);
            state.mem.write_u32(sp - 4, v)?;
            set(state, Reg::Sp, sp - 4);
            Ok(Effect::Next)
        }
        Instr::Pop { rd } => {
            let sp = r(state, Reg::Sp);
            check_pop(sp)?;
            let v = state.mem.read_u32(sp)?;
            set(state, Reg::Sp, sp + 4);
            // `pop sp` ends with sp = v.
            set(state, rd, v);
            Ok(Effect::Next)
        }
        Instr::Jump { cond, off } => {
            if cond.holds(state.flags) {
                let target = jump_target(&state.mem, pc.wrapping_add((off as u32) << 2))?;
                Ok(Effect::Goto(target))
            } else {
                Ok(Effect::Next)
            }
        }
        Instr::Call { off } => {
            let target = jump_target(&state.mem, pc.wrapping_add((off as u32) << 2))?;
            call(state, target)
        }
        Instr::JumpReg { rs } => {
            let target = jump_target(&state.mem, r(state, rs))?;
            Ok(Effect::Goto(target))
        }
        Instr::CallReg { rs } => {
            let target = jump_target(&state.mem, r(state, rs))?;
            call(state, target)
        }
    }
}

/// Second half of `call` / `call rs` once the target is known to be valid.
#[inline]
fn call(state: &mut MachineState, target: u32) -> Result<Effect, FaultKind> {
    let sp = state.regs[Reg::Sp.index()];
    check_push(sp)?;
    state.mem.write_u32(sp - 4, state.pc.wrapping_add(4))?;
    set(state, Reg::Sp, sp - 4);
    Ok(Effect::Goto(target))
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
pub fn step(state: &mut MachineState, tapes: &mut Tapes, cfg: &MachineConfig) -> Status {
    let pc = state.pc;
    let decoded = state.mem.fetch(pc).and_then(|word| {
        isa::decode(word).map_err(|e| match e {
            DecodeError::Zero => FaultKind::ZeroInstruction,
            DecodeError::Illegal => FaultKind::IllegalInstruction { word },
        })
    });
    let result = decoded.and_then(|instr| execute(state, tapes, cfg, instr).map(|e| (instr, e)));
    match result {
        Ok((instr, effect)) => {
            let status = match effect {
                Effect::Next => {
                    state.pc = pc.wrapping_add(4);
                    Status::Paused
                }
                Effect::Goto(target) => {
                    state.pc = target;
                    Status::Paused
                }
                Effect::Halt => Status::Halted,
                Effect::Wait => {
                    state.status = Status::WaitingInput;
                    return Status::WaitingInput;
                }
            };
            state.steps += 1;
            state.cycles += instr.cycles() as u64;
            state.status = status;
            status
        }
        Err(kind) => {
            state.status = Status::Fault;
            state.fault = Some(kind.at(pc, state.steps));
            Status::Fault
        }
    }
}
