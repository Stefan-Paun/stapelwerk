//! Read-only views of a machine state for the debugger: `StateView`, the frame chain and heap
//! blocks with source lines.

use crate::consts::{MAX_FRAMES, STACK_START, STACK_TOP};
use crate::diag::FILE_PLAYER;
use crate::disasm::jump_target;
use crate::isa::{Instr, Reg, decode};
use crate::machine::{MachineState, Status, Tapes};
use crate::program::Program;

/// The fixed 96-byte state record of the WASM ABI (`sess_view`, docs/ABI.md "StateView").
/// Counters saturate at 2³²−1.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StateView {
    pub status: u32,
    pub pc: u32,
    /// r0–r7, fp, sp.
    pub regs: [u32; Reg::COUNT],
    pub flags: u32,
    pub steps: u32,
    pub cycles: u32,
    /// 0 = no fault, else the 1-based fault code.
    pub fault_code: u32,
    /// First numeric fault argument, else 0.
    pub fault_addr: u32,
    pub out_len: u32,
    pub in_int_pos: u32,
    pub in_int_len: u32,
    pub in_text_pos: u32,
    pub in_text_len: u32,
    /// Current test, −1 = interactive.
    pub test_index: i32,
    pub budget: u32,
}

const _: () = assert!(core::mem::size_of::<StateView>() == 96);

fn sat(v: u64) -> u32 {
    v.min(u32::MAX as u64) as u32
}

impl StateView {
    /// Builds the view. `status` overrides the state's status (NOT_ASSEMBLED sessions).
    pub fn new(
        state: &MachineState,
        tapes: &Tapes,
        status: Status,
        test_index: i32,
        budget: u64,
    ) -> Self {
        let (fault_code, fault_addr) = match &state.fault {
            Some(f) if status == Status::Fault => (f.kind.code(), f.kind.addr()),
            _ => (0, 0),
        };
        StateView {
            status: status.code(),
            pc: state.pc,
            regs: state.regs,
            flags: state.flags,
            steps: sat(state.steps),
            cycles: sat(state.cycles),
            fault_code,
            fault_addr,
            out_len: state.out_len,
            in_int_pos: state.in_int_pos,
            in_int_len: sat(tapes.in_ints.len() as u64),
            in_text_pos: state.in_text_pos,
            in_text_len: sat(tapes.in_text.len() as u64),
            test_index,
            budget: sat(budget),
        }
    }
}

/// One frame of the fp chain: `{fp, ret, func, size}` (`sess_frames`), innermost first.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    pub fp: u32,
    /// Return address stored at `[fp + 4]`.
    pub ret: u32,
    /// Function name (nearest global code label), if known.
    pub func: Option<String>,
    /// Bytes from the frame's lowest address up to and including its return address: for the
    /// innermost frame `fp + 8 − sp`, for outer frames `fp + 8 − (child fp + 8)`.
    pub size: u32,
}

/// Target of a direct `call` that returns to `ret`, if the word before `ret` is one.
fn call_target(state: &MachineState, ret: u32) -> Option<u32> {
    if !ret.is_multiple_of(4) || ret < 4 {
        return None;
    }
    let site = ret - 4;
    match decode(state.mem.peek_u32(site)) {
        Ok(Instr::Call { off }) => {
            let t = jump_target(site, off);
            (0..=0xFFFF).contains(&t).then_some(t as u32)
        }
        _ => None,
    }
}

/// The debugger frame chain (SPEC §9): follows fp until fp = 0, fp leaves the stack segment
/// (`[fp]` and `[fp + 4]` must lie in 0xD000…0xEFFF), fp does not increase, or 64 frames.
///
/// Function name: nearest global label ≤ the target of the direct `call` before the return
/// address; if that is unknown (`call rs`, trap, garbage), the label ≤ `pc` for the innermost frame
/// and ≤ the child's return address for the others.
pub fn frame_chain(state: &MachineState, program: Option<&Program>) -> Vec<Frame> {
    let mut out = Vec::new();
    let mut fp = state.reg(Reg::Fp);
    let mut low = state.reg(Reg::Sp);
    let mut code_addr = state.pc; // an address inside the current frame's function
    let mut prev: Option<u32> = None;
    while out.len() < MAX_FRAMES as usize {
        if !(STACK_START..=STACK_TOP - 8).contains(&fp) || prev.is_some_and(|p| fp <= p) {
            break;
        }
        let ret = state.mem.peek_u32(fp + 4);
        let old_fp = state.mem.peek_u32(fp);
        let named_at = call_target(state, ret).unwrap_or(code_addr);
        let func = program
            .and_then(|p| p.function_at(named_at))
            .map(|s| s.name.clone());
        out.push(Frame {
            fp,
            ret,
            func,
            size: (fp + 8).saturating_sub(low),
        });
        low = fp + 8;
        code_addr = ret;
        prev = Some(fp);
        fp = old_fp;
    }
    out
}

/// A heap block with the source line of its `ALLOC` (`sess_heap`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HeapBlockInfo {
    pub addr: u32,
    pub size: u32,
    pub live: bool,
    /// Line of the `sys ALLOC` in the player's file, 0 if unknown (e.g. inside the library).
    pub line: u32,
}

/// All heap blocks (live and freed, allocation order) with their `ALLOC` lines.
pub fn heap_blocks(state: &MachineState, program: Option<&Program>) -> Vec<HeapBlockInfo> {
    state
        .heap
        .blocks()
        .iter()
        .map(|b| HeapBlockInfo {
            addr: b.addr,
            size: b.size,
            live: b.live,
            line: program
                .and_then(|p| p.line_of(b.alloc_pc))
                .filter(|e| e.file == FILE_PLAYER)
                .map_or(0, |e| e.line),
        })
        .collect()
}
