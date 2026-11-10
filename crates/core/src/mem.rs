//! 64 KiB memory with segment rights (page table: 256 pages of 256 bytes, see
//! [`crate::consts::SEGMENTS`]). Heap shadow checks are done separately by [`crate::heap::Heap`].
//!
//! PHASE 1A owns this file. Everything marked `PHASE 1A` is a placeholder that returns a
//! harmless default and must be implemented according to `docs/ISA.md` ("Memory map").

use crate::consts::{MEM_SIZE, SCREEN_BYTES, SCREEN_START, TEXT_START};
use crate::fault::{Access, FaultKind};
use crate::program::Program;

/// The machine's memory. The byte buffer is allocated once and never reallocated, so a pointer to
/// it stays valid for the lifetime of the value (the WASM ABI hands it to JavaScript).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Memory {
    bytes: Box<[u8]>,
    /// One past the last instruction word (`TEXT_START + program.text_len`).
    code_end: u32,
}

impl Default for Memory {
    fn default() -> Self {
        Self::new()
    }
}

impl Memory {
    /// All-zero memory with an empty code region.
    pub fn new() -> Self {
        Memory {
            bytes: vec![0u8; MEM_SIZE as usize].into_boxed_slice(),
            code_end: TEXT_START,
        }
    }

    /// Raw bytes (65536), no checks. For the debugger, the hexdump and tests.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Raw mutable bytes, no checks. For loaders (`set` in tests, `screen_init`) only, never for
    /// instruction execution.
    pub fn bytes_mut(&mut self) -> &mut [u8] {
        &mut self.bytes
    }

    /// The 4096 screen bytes at 0xF000.
    pub fn screen(&self) -> &[u8] {
        let start = SCREEN_START as usize;
        self.bytes
            .get(start..start + SCREEN_BYTES as usize)
            .unwrap_or(&[])
    }

    /// One past the last instruction word.
    pub fn code_end(&self) -> u32 {
        self.code_end
    }

    /// Copies `other` into this value **in place** (keeps the buffer address stable).
    pub fn copy_from(&mut self, other: &Memory) {
        self.bytes.copy_from_slice(&other.bytes);
        self.code_end = other.code_end;
    }

    /// Writes the program's words (little-endian) from 0x0100 and its data from 0x4000 and sets
    /// the code region. PHASE 1A.
    pub fn load_program(&mut self, _program: &Program) {}

    /// `true` if `addr` is a valid jump/call/return target: 4-aligned and inside
    /// `TEXT_START..code_end`. PHASE 1A.
    pub fn is_code_target(&self, _addr: u32) -> bool {
        false
    }

    /// Checks segment rights and alignment for a data access of `size` bytes (1 or 4) at `addr`
    /// (already computed modulo 2³²). Order of checks: range (≥ 0x10000 → `out_of_range`),
    /// alignment for words, then segment rights (`null_access`, `guard_page`, `write_to_code`).
    /// PHASE 1A.
    pub fn check(&self, addr: u32, _size: u32, access: Access) -> Result<(), FaultKind> {
        Err(FaultKind::OutOfRange { addr, access })
    }

    /// Checked word read (little-endian). PHASE 1A.
    pub fn read_u32(&self, addr: u32) -> Result<u32, FaultKind> {
        Err(FaultKind::OutOfRange {
            addr,
            access: Access::Read,
        })
    }

    /// Checked byte read. PHASE 1A.
    pub fn read_u8(&self, addr: u32) -> Result<u8, FaultKind> {
        Err(FaultKind::OutOfRange {
            addr,
            access: Access::Read,
        })
    }

    /// Checked word write (little-endian). PHASE 1A.
    pub fn write_u32(&mut self, addr: u32, _value: u32) -> Result<(), FaultKind> {
        Err(FaultKind::OutOfRange {
            addr,
            access: Access::Write,
        })
    }

    /// Checked byte write. PHASE 1A.
    pub fn write_u8(&mut self, addr: u32, _value: u8) -> Result<(), FaultKind> {
        Err(FaultKind::OutOfRange {
            addr,
            access: Access::Write,
        })
    }

    /// Instruction fetch at `pc`: must lie in `TEXT_START..code_end`, else `exec_outside_code`.
    /// PHASE 1A.
    pub fn fetch(&self, _pc: u32) -> Result<u32, FaultKind> {
        Err(FaultKind::ExecOutsideCode)
    }

    /// Unchecked little-endian word at `addr` (0 outside memory). For the debugger.
    pub fn peek_u32(&self, addr: u32) -> u32 {
        let a = addr as usize;
        match self.bytes.get(a..a.saturating_add(4)) {
            Some(b) if b.len() == 4 => u32::from_le_bytes([b[0], b[1], b[2], b[3]]),
            _ => 0,
        }
    }
}
