//! Heap manager for 0x8000…0xBFFF, kept in Rust (not in VM memory): first fit, 8-byte aligned
//! blocks, an 8-byte red zone after every block, and a shadow map with one byte per 4-byte granule.
//!
//! Semantics: `docs/ISA.md` ("Heap").

use crate::consts::{
    HEAP_ALIGN, HEAP_ALLOC_FILL, HEAP_FREE_FILL, HEAP_GRANULE, HEAP_REDZONE, HEAP_SHADOW_BYTES,
    HEAP_START,
};
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
    /// allocated part of the block with 0xCD in `mem` and updates the shadow.
    ///
    /// Layout (ISA.md clarification): the span is round_up(n, 8) + 8 bytes from an 8-aligned
    /// address; the first round_up(n, 4) bytes are allocated, the rest is red zone. First fit picks
    /// the lowest address whose span consists only of free or freed granules. Freed blocks whose
    /// span is reused are dropped from the block list (their memory now belongs to the new block).
    pub fn alloc(&mut self, mem: &mut Memory, size: u32, pc: u32) -> u32 {
        if size == 0 {
            return 0;
        }
        let heap_bytes = HEAP_SHADOW_BYTES * HEAP_GRANULE;
        let span =
            (size as u64).div_ceil(HEAP_ALIGN as u64) * HEAP_ALIGN as u64 + HEAP_REDZONE as u64;
        if span > heap_bytes as u64 {
            return 0;
        }
        let span_granules = (span / HEAP_GRANULE as u64) as usize;
        let align_granules = (HEAP_ALIGN / HEAP_GRANULE) as usize;
        let Some(start) = first_fit(&self.shadow, span_granules, align_granules) else {
            return 0;
        };
        let alloc_granules = size.div_ceil(HEAP_GRANULE) as usize;
        if let Some(g) = self.shadow.get_mut(start..start + span_granules) {
            let (used, red) = g.split_at_mut(alloc_granules.min(span_granules));
            used.fill(SHADOW_ALLOCATED);
            red.fill(SHADOW_REDZONE);
        }
        let addr = HEAP_START + start as u32 * HEAP_GRANULE;
        let span = span as u32;
        // Freed blocks overlapping the new span are gone for good.
        self.blocks
            .retain(|b| b.live || b.addr + span_of(b.size) <= addr || b.addr >= addr + span);
        mem.fill(addr, alloc_granules as u32 * HEAP_GRANULE, HEAP_ALLOC_FILL);
        self.blocks.push(Block {
            addr,
            size,
            live: true,
            alloc_pc: pc,
        });
        addr
    }

    /// `FREE`: 0 is a no-op; otherwise `addr` must start a live block (`invalid_free`,
    /// `double_free`). Fills the whole span (block and red zone) with 0xDD and marks it freed.
    /// On `Err` nothing has changed.
    pub fn free(&mut self, mem: &mut Memory, addr: u32) -> Result<(), FaultKind> {
        if addr == 0 {
            return Ok(());
        }
        let Some(block) = self.blocks.iter_mut().rev().find(|b| b.addr == addr) else {
            return Err(FaultKind::InvalidFree { addr });
        };
        if !block.live {
            return Err(FaultKind::DoubleFree { addr });
        }
        block.live = false;
        let span = span_of(block.size);
        let g = ((addr - HEAP_START) / HEAP_GRANULE) as usize;
        if let Some(s) = self.shadow.get_mut(g..g + (span / HEAP_GRANULE) as usize) {
            s.fill(SHADOW_FREED);
        }
        mem.fill(addr, span, HEAP_FREE_FILL);
        Ok(())
    }

    /// Strict-mode check of a data access that touches the heap segment: free →
    /// `heap_unallocated`, freed → `use_after_free`, red zone → `heap_overflow` (each with the
    /// first offending address). Always `Ok` for [`HeapCheck::Off`] and outside the heap.
    #[inline]
    pub fn check_access(
        &self,
        addr: u32,
        size: u32,
        _access: Access,
        mode: HeapCheck,
    ) -> Result<(), FaultKind> {
        if mode == HeapCheck::Off {
            return Ok(());
        }
        let end = addr.saturating_add(size.max(1));
        if end <= HEAP_START || addr >= HEAP_END {
            return Ok(());
        }
        let mut a = addr.max(HEAP_START);
        while a < end.min(HEAP_END) {
            let g = ((a - HEAP_START) / HEAP_GRANULE) as usize;
            match self.shadow.get(g).copied().unwrap_or(SHADOW_ALLOCATED) {
                SHADOW_FREE => return Err(FaultKind::HeapUnallocated { addr: a }),
                SHADOW_FREED => return Err(FaultKind::UseAfterFree { addr: a }),
                SHADOW_REDZONE => return Err(FaultKind::HeapOverflow { addr: a }),
                _ => {}
            }
            // Next granule boundary.
            a = (a - HEAP_START) / HEAP_GRANULE * HEAP_GRANULE + HEAP_START + HEAP_GRANULE;
        }
        Ok(())
    }
}

/// End of the heap segment (exclusive).
const HEAP_END: u32 = HEAP_START + HEAP_SHADOW_BYTES * HEAP_GRANULE;

/// Span in bytes of a block of `size` bytes: round_up(size, 8) + red zone.
fn span_of(size: u32) -> u32 {
    size.div_ceil(HEAP_ALIGN) * HEAP_ALIGN + HEAP_REDZONE
}

/// Lowest granule index, a multiple of `align`, that starts `len` consecutive free or freed
/// granules. One linear pass.
fn first_fit(shadow: &[u8], len: usize, align: usize) -> Option<usize> {
    let mut cand = 0usize;
    for (g, &state) in shadow.iter().enumerate() {
        if state == SHADOW_FREE || state == SHADOW_FREED {
            if g >= cand && g + 1 - cand >= len {
                return Some(cand);
            }
        } else {
            cand = (g + 1).div_ceil(align) * align;
        }
    }
    None
}
