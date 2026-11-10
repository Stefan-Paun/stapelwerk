//! Heap manager for 0x8000…0xBFFF, kept in Rust (not in VM memory): first fit, 8-byte aligned
//! blocks, an 8-byte red zone after every block, and a shadow map with one byte per 4-byte granule.
//!
//! PHASE 1A owns this file. Placeholders are marked `PHASE 1A`; semantics: `docs/ISA.md` ("Heap").

use crate::consts::{HEAP_SHADOW_BYTES, HEAP_START};
use crate::fault::{Access, FaultKind};
use crate::mem::Memory;

/// Shadow state: never allocated.
pub const SHADOW_FREE: u8 = 0;
/// Shadow state: part of a live block.
pub const SHADOW_ALLOCATED: u8 = 1;
/// Shadow state: part of a freed block.
pub const SHADOW_FREED: u8 = 2;
/// Shadow state: red zone or alignment padding after a block.
pub const SHADOW_REDZONE: u8 = 3;

/// Level switch `heap_check`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HeapCheck {
    /// Every access to free/freed/red-zone heap memory faults (default).
    #[default]
    Strict,
    /// Those three checks are skipped; `invalid_free` and `double_free` still apply.
    Off,
}

impl HeapCheck {
    /// `"strict"` or `"off"`.
    pub const fn as_str(self) -> &'static str {
        match self {
            HeapCheck::Strict => "strict",
            HeapCheck::Off => "off",
        }
    }

    /// Parses `strict` / `off`.
    pub fn parse(s: &str) -> Option<HeapCheck> {
        match s {
            "strict" => Some(HeapCheck::Strict),
            "off" => Some(HeapCheck::Off),
            _ => None,
        }
    }
}

/// One allocation (live or freed), in allocation order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Block {
    /// Start address (8-aligned).
    pub addr: u32,
    /// Requested size in bytes.
    pub size: u32,
    /// `false` after `FREE`.
    pub live: bool,
    /// `pc` of the `sys ALLOC` instruction (for "allocated in line …").
    pub alloc_pc: u32,
}

/// Heap state. Part of [`crate::machine::MachineState`]; cloned into snapshots.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Heap {
    /// One byte per 4-byte granule of 0x8000…0xBFFF (4096 bytes), values `SHADOW_*`.
    /// Allocated once; the buffer address is stable (exported to JavaScript).
    shadow: Box<[u8]>,
    blocks: Vec<Block>,
}

impl Default for Heap {
    fn default() -> Self {
        Self::new()
    }
}

impl Heap {
    /// Empty heap, all granules free.
    pub fn new() -> Self {
        Heap {
            shadow: vec![SHADOW_FREE; HEAP_SHADOW_BYTES as usize].into_boxed_slice(),
            blocks: Vec::new(),
        }
    }

    /// The shadow map (4096 bytes).
    pub fn shadow(&self) -> &[u8] {
        &self.shadow
    }

    /// All blocks, live and freed.
    pub fn blocks(&self) -> &[Block] {
        &self.blocks
    }

    /// Number of live blocks (`expect_heap_empty`).
    pub fn live_count(&self) -> usize {
        self.blocks.iter().filter(|b| b.live).count()
    }

    /// Shadow state of the granule containing `addr` (`None` outside the heap).
    pub fn shadow_at(&self, addr: u32) -> Option<u8> {
        let off = addr.checked_sub(HEAP_START)?;
        self.shadow.get((off / 4) as usize).copied()
    }

    /// Copies `other` into this value in place (keeps the shadow buffer address stable).
    pub fn copy_from(&mut self, other: &Heap) {
        self.shadow.copy_from_slice(&other.shadow);
        self.blocks.clone_from(&other.blocks);
    }

    /// `ALLOC`: first fit; returns the 8-aligned address or 0 (size 0 or no space). Fills the
    /// block with 0xCD in `mem` and updates the shadow. PHASE 1A.
    pub fn alloc(&mut self, _mem: &mut Memory, _size: u32, _pc: u32) -> u32 {
        0
    }

    /// `FREE`: 0 is a no-op; otherwise `addr` must start a live block (`invalid_free`,
    /// `double_free`). Fills the block with 0xDD and marks it freed. PHASE 1A.
    pub fn free(&mut self, _mem: &mut Memory, addr: u32) -> Result<(), FaultKind> {
        if addr == 0 {
            return Ok(());
        }
        Err(FaultKind::InvalidFree { addr })
    }

    /// Strict-mode check of a data access that touches the heap segment: free →
    /// `heap_unallocated`, freed → `use_after_free`, red zone → `heap_overflow` (each with the
    /// first offending address). Always `Ok` for [`HeapCheck::Off`] and outside the heap. PHASE 1A.
    pub fn check_access(
        &self,
        _addr: u32,
        _size: u32,
        _access: Access,
        _mode: HeapCheck,
    ) -> Result<(), FaultKind> {
        Ok(())
    }
}
