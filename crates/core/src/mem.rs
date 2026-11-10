//! 64 KiB memory with segment rights (page table: 256 pages of 256 bytes, see
//! [`crate::consts::SEGMENTS`]). Heap shadow checks are done separately by [`crate::heap::Heap`].
//!
//! Semantics: `docs/ISA.md` ("Memory map", "Memory access checks").

use crate::consts::{
    DATA_START, MEM_SIZE, PAGE_COUNT, PAGE_SIZE, SCREEN_BYTES, SCREEN_START, SEGMENTS, Segment,
    TEXT_START,
};
use crate::fault::{Access, FaultKind};
use crate::program::Program;

/// Size of the data segment in bytes.
const DATA_BYTES: usize = 0x2000;

/// Page right bits.
const R: u8 = 1;
const W: u8 = 2;
const X: u8 = 4;

/// Segment index for every page (index into [`SEGMENTS`]).
const fn build_page_segments() -> [u8; PAGE_COUNT as usize] {
    let mut table = [0u8; PAGE_COUNT as usize];
    let mut page = 0;
    while page < PAGE_COUNT as usize {
        let addr = page as u32 * PAGE_SIZE;
        let mut i = 0;
        while i < SEGMENTS.len() {
            if addr >= SEGMENTS[i].start && addr < SEGMENTS[i].end {
                table[page] = i as u8;
            }
            i += 1;
        }
        page += 1;
    }
    table
}

/// Rights (`R`, `W`, `X` bits) for every page, derived from the segment table.
const fn build_page_rights() -> [u8; PAGE_COUNT as usize] {
    let segs = build_page_segments();
    let mut table = [0u8; PAGE_COUNT as usize];
    let mut page = 0;
    while page < PAGE_COUNT as usize {
        let s = &SEGMENTS[segs[page] as usize];
        table[page] = (if s.read { R } else { 0 })
            | (if s.write { W } else { 0 })
            | (if s.exec { X } else { 0 });
        page += 1;
    }
    table
}

/// The page table: segment of every 256-byte page.
pub const PAGE_SEGMENTS: [u8; PAGE_COUNT as usize] = build_page_segments();
/// The page table: rights of every 256-byte page.
const PAGE_RIGHTS: [u8; PAGE_COUNT as usize] = build_page_rights();

/// Segment of an address below 0x10000 (via the page table).
pub fn page_segment(addr: u32) -> Option<Segment> {
    PAGE_SEGMENTS
        .get((addr / PAGE_SIZE) as usize)
        .and_then(|&i| SEGMENTS.get(i as usize))
        .map(|s| s.segment)
}

/// Fault for an access that the page rights deny (address already known to be < 0x10000).
fn denied(addr: u32, access: Access) -> FaultKind {
    match page_segment(addr) {
        Some(Segment::Null) => FaultKind::NullAccess { addr, access },
        Some(Segment::Guard) => FaultKind::GuardPage { addr, access },
        Some(Segment::Text) => FaultKind::WriteToCode { addr },
        // Not reachable with the fixed memory map; keep a sensible key anyway.
        _ => FaultKind::OutOfRange { addr, access },
    }
}

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
    /// the code region. Words beyond the text segment and data beyond the data segment are
    /// ignored (the assembler never produces them); the code region is clamped accordingly.
    pub fn load_program(&mut self, program: &Program) {
        let max_words = ((DATA_START - TEXT_START) / 4) as usize;
        let mut addr = TEXT_START as usize;
        for &w in program.words.iter().take(max_words) {
            if let Some(dst) = self.bytes.get_mut(addr..addr + 4) {
                dst.copy_from_slice(&w.to_le_bytes());
            }
            addr += 4;
        }
        let loaded = (program.words.len().min(max_words) as u32) * 4;
        let text_len = program.text_len.min(loaded) & !3;
        self.code_end = TEXT_START + text_len;
        let data_len = program.data.len().min(DATA_BYTES);
        let start = DATA_START as usize;
        if let (Some(dst), Some(src)) = (
            self.bytes.get_mut(start..start + data_len),
            program.data.get(..data_len),
        ) {
            dst.copy_from_slice(src);
        }
    }

    /// `true` if `addr` is a valid jump/call/return target: 4-aligned and inside
    /// `TEXT_START..code_end`.
    #[inline]
    pub fn is_code_target(&self, addr: u32) -> bool {
        addr.is_multiple_of(4) && addr >= TEXT_START && addr < self.code_end
    }

    /// Checks segment rights and alignment for a data access of `size` bytes (1 or 4) at `addr`
    /// (already computed modulo 2³²). Order of checks: range (≥ 0x10000 → `out_of_range`),
    /// alignment for words, then segment rights (`null_access`, `guard_page`, `write_to_code`).
    /// Heap shadow checks are separate ([`crate::heap::Heap::check_access`]).
    #[inline]
    pub fn check(&self, addr: u32, size: u32, access: Access) -> Result<(), FaultKind> {
        if addr >= MEM_SIZE {
            return Err(FaultKind::OutOfRange { addr, access });
        }
        if size == 4 && !addr.is_multiple_of(4) {
            return Err(FaultKind::Misaligned { addr, size: 4 });
        }
        // Aligned words never cross a page, so the page of `addr` decides for the whole access.
        let rights = PAGE_RIGHTS[(addr / PAGE_SIZE) as usize];
        match access {
            Access::Read if rights & R == 0 => Err(denied(addr, access)),
            Access::Write if rights & W == 0 => Err(denied(addr, access)),
            _ => Ok(()),
        }
    }

    /// Checked word read (little-endian).
    #[inline]
    pub fn read_u32(&self, addr: u32) -> Result<u32, FaultKind> {
        self.check(addr, 4, Access::Read)?;
        Ok(self.peek_u32(addr))
    }

    /// Checked byte read.
    #[inline]
    pub fn read_u8(&self, addr: u32) -> Result<u8, FaultKind> {
        self.check(addr, 1, Access::Read)?;
        Ok(self.bytes.get(addr as usize).copied().unwrap_or(0))
    }

    /// Checked word write (little-endian).
    #[inline]
    pub fn write_u32(&mut self, addr: u32, value: u32) -> Result<(), FaultKind> {
        self.check(addr, 4, Access::Write)?;
        let a = addr as usize;
        if let Some(dst) = self.bytes.get_mut(a..a + 4) {
            dst.copy_from_slice(&value.to_le_bytes());
        }
        Ok(())
    }

    /// Checked byte write.
    #[inline]
    pub fn write_u8(&mut self, addr: u32, value: u8) -> Result<(), FaultKind> {
        self.check(addr, 1, Access::Write)?;
        if let Some(b) = self.bytes.get_mut(addr as usize) {
            *b = value;
        }
        Ok(())
    }

    /// Instruction fetch at `pc`: must be 4-aligned and lie in `TEXT_START..code_end`, else
    /// `exec_outside_code`.
    #[inline]
    pub fn fetch(&self, pc: u32) -> Result<u32, FaultKind> {
        if self.is_code_target(pc) {
            Ok(self.peek_u32(pc))
        } else {
            Err(FaultKind::ExecOutsideCode)
        }
    }

    /// Fills `len` bytes from `addr` with `value`, without checks (heap fill patterns). Bytes
    /// outside memory are ignored.
    pub(crate) fn fill(&mut self, addr: u32, len: u32, value: u8) {
        let a = addr as usize;
        let end = (a + len as usize).min(self.bytes.len());
        if let Some(dst) = self.bytes.get_mut(a..end) {
            dst.fill(value);
        }
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
