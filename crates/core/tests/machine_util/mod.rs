//! Helpers shared by the machine tests: hand-built programs (no assembler needed).
#![allow(dead_code)]

use stapelwerk_core::consts::{SYS_ALLOC, SYS_FREE};
use stapelwerk_core::fault::FaultKind;
use stapelwerk_core::heap::HeapCheck;
use stapelwerk_core::isa::{AluOp, Cond, Instr, Mem, Reg, encode};
use stapelwerk_core::machine::{Machine, MachineConfig, MachineState, Status, Tapes};
use stapelwerk_core::program::Program;

pub use Reg::*;

/// Program from instructions (from 0x0100) and initial data (from 0x4000).
pub fn program(code: &[Instr], data: &[u8]) -> Program {
    let words: Vec<u32> = code.iter().map(encode).collect();
    raw_program(&words, data)
}

/// Program from raw words.
pub fn raw_program(words: &[u32], data: &[u8]) -> Program {
    Program {
        text_len: words.len() as u32 * 4,
        words: words.to_vec(),
        data: data.to_vec(),
        trap_addr: 0x100 + words.len() as u32 * 4,
        ..Program::default()
    }
}

pub fn config() -> MachineConfig {
    MachineConfig::default()
}

pub fn config_off() -> MachineConfig {
    MachineConfig {
        heap_check: HeapCheck::Off,
        ..MachineConfig::default()
    }
}

/// Machine for `code` with the default configuration (test mode, strict heap).
pub fn machine(code: &[Instr]) -> Machine {
    Machine::new(&program(code, &[]), config())
}

/// Runs `code` to completion and returns the machine.
pub fn run(code: &[Instr]) -> Machine {
    run_cfg(code, &[], config())
}

pub fn run_cfg(code: &[Instr], data: &[u8], cfg: MachineConfig) -> Machine {
    let mut m = Machine::new(&program(code, data), cfg);
    m.run(u64::MAX);
    m
}

/// Runs `code` and returns the fault kind (panics if the run did not fault).
pub fn fault_of(code: &[Instr]) -> FaultKind {
    let m = run(code);
    fault_kind(&m)
}

pub fn fault_kind(m: &Machine) -> FaultKind {
    assert_eq!(m.state.status, Status::Fault, "expected a fault");
    m.state.fault.expect("fault set").kind
}

/// Digest ignoring status and fault (for the precise-fault property).
pub fn digest_without_status(state: &MachineState, tapes: &Tapes) -> u64 {
    let mut s = state.clone();
    s.status = Status::Paused;
    s.fault = None;
    s.digest(tapes)
}

// --- instruction builders ----------------------------------------------------------------------

pub fn movi(rd: Reg, imm: i16) -> Instr {
    Instr::MovI { rd, imm }
}

pub fn movhi(rd: Reg, imm: u16) -> Instr {
    Instr::MovHi { rd, imm }
}

/// `li rd, v` as two words.
pub fn li(rd: Reg, v: u32) -> [Instr; 2] {
    [movi(rd, v as u16 as i16), movhi(rd, (v >> 16) as u16)]
}

pub fn mov(rd: Reg, rs: Reg) -> Instr {
    Instr::Mov { rd, rs }
}

pub fn alu(op: AluOp, rd: Reg, rs1: Reg, rs2: Reg) -> Instr {
    Instr::Alu { op, rd, rs1, rs2 }
}

pub fn alui(op: AluOp, rd: Reg, rs1: Reg, imm: i32) -> Instr {
    Instr::AluI { op, rd, rs1, imm }
}

pub fn cmpi(rs1: Reg, imm: i32) -> Instr {
    Instr::AluI {
        op: AluOp::Cmp,
        rd: R0,
        rs1,
        imm,
    }
}

pub fn based(base: Reg, off: i16) -> Mem {
    Mem::Based { base, off }
}

pub fn abs(addr: u16) -> Mem {
    Mem::Abs { addr }
}

pub fn ld(rd: Reg, m: Mem) -> Instr {
    Instr::Ld { rd, m }
}

pub fn st(rs: Reg, m: Mem) -> Instr {
    Instr::St { rs, m }
}

pub fn sys(n: u16) -> Instr {
    Instr::Sys { n }
}

pub fn jump(cond: Cond, off: i32) -> Instr {
    Instr::Jump { cond, off }
}

pub const HALT: Instr = Instr::Halt;
pub const NOP: Instr = Instr::Nop;
pub const RET: Instr = Instr::Ret;

/// `r0 = ALLOC(size)`.
pub fn alloc(size: i16) -> [Instr; 2] {
    [movi(R0, size), sys(SYS_ALLOC)]
}

/// `FREE(r0)`.
pub fn free() -> Instr {
    sys(SYS_FREE)
}

/// Concatenates instruction groups.
pub fn code(parts: &[&[Instr]]) -> Vec<Instr> {
    parts.iter().flat_map(|p| p.iter().copied()).collect()
}
