//! Fixed numbers of the machine: memory map, built-in assembler constants, colour names,
//! system call numbers and limits. Normative description: `docs/ISA.md`.

use crate::isa;

// ---------------------------------------------------------------------------------------------
// Memory map (64 KiB, 256 pages of 256 bytes)
// ---------------------------------------------------------------------------------------------

/// Total addressable memory in bytes.
pub const MEM_SIZE: u32 = 0x1_0000;
/// Page size of the page table.
pub const PAGE_SIZE: u32 = 256;
/// Number of pages.
pub const PAGE_COUNT: u32 = 256;

/// Start of the null segment (no access).
pub const NULL_START: u32 = 0x0000;
/// Start of the text segment (code: player, then library, then the trap word).
pub const TEXT_START: u32 = 0x0100;
/// Start of the data segment (`.data`: player, then library).
pub const DATA_START: u32 = 0x4000;
/// Start of the input segment (level input data, constant `INPUT`).
pub const INPUT_START: u32 = 0x6000;
/// Start of the heap segment (only through `ALLOC`).
pub const HEAP_START: u32 = 0x8000;
/// Start of the guard segment (no access).
pub const GUARD_START: u32 = 0xC000;
/// Start of the stack segment.
pub const STACK_START: u32 = 0xD000;
/// Start of the screen segment; also the initial stack pointer (stack grows down from here).
pub const SCREEN_START: u32 = 0xF000;

/// Program entry point.
pub const ENTRY: u32 = TEXT_START;
/// Initial value of `sp`.
pub const STACK_TOP: u32 = SCREEN_START;

/// One memory segment.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Segment {
    Null,
    Text,
    Data,
    Input,
    Heap,
    Guard,
    Stack,
    Screen,
}

/// Static description of a segment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SegmentInfo {
    pub segment: Segment,
    /// Lower-case name used in JSON and docs.
    pub name: &'static str,
    /// First address.
    pub start: u32,
    /// One past the last address.
    pub end: u32,
    pub read: bool,
    pub write: bool,
    pub exec: bool,
}

/// The memory map in address order. Covers 0x0000…0xFFFF without gaps.
pub const SEGMENTS: [SegmentInfo; 8] = [
    seg(
        Segment::Null,
        "null",
        NULL_START,
        TEXT_START,
        false,
        false,
        false,
    ),
    seg(
        Segment::Text,
        "text",
        TEXT_START,
        DATA_START,
        true,
        false,
        true,
    ),
    seg(
        Segment::Data,
        "data",
        DATA_START,
        INPUT_START,
        true,
        true,
        false,
    ),
    seg(
        Segment::Input,
        "input",
        INPUT_START,
        HEAP_START,
        true,
        true,
        false,
    ),
    seg(
        Segment::Heap,
        "heap",
        HEAP_START,
        GUARD_START,
        true,
        true,
        false,
    ),
    seg(
        Segment::Guard,
        "guard",
        GUARD_START,
        STACK_START,
        false,
        false,
        false,
    ),
    seg(
        Segment::Stack,
        "stack",
        STACK_START,
        SCREEN_START,
        true,
        true,
        false,
    ),
    seg(
        Segment::Screen,
        "screen",
        SCREEN_START,
        MEM_SIZE,
        true,
        true,
        false,
    ),
];

const fn seg(
    segment: Segment,
    name: &'static str,
    start: u32,
    end: u32,
    read: bool,
    write: bool,
    exec: bool,
) -> SegmentInfo {
    SegmentInfo {
        segment,
        name,
        start,
        end,
        read,
        write,
        exec,
    }
}

/// Segment containing `addr`, or `None` for addresses ≥ 0x10000.
pub const fn segment_of(addr: u32) -> Option<&'static SegmentInfo> {
    let mut i = 0;
    while i < SEGMENTS.len() {
        if addr >= SEGMENTS[i].start && addr < SEGMENTS[i].end {
            return Some(&SEGMENTS[i]);
        }
        i += 1;
    }
    None
}

// ---------------------------------------------------------------------------------------------
// Screen
// ---------------------------------------------------------------------------------------------

/// Screen width in pixels.
pub const SCREEN_W: u32 = 64;
/// Screen height in pixels.
pub const SCREEN_H: u32 = 64;
/// Screen size in bytes (one byte per pixel).
pub const SCREEN_BYTES: u32 = SCREEN_W * SCREEN_H;
/// Only the low four bits of a pixel byte select the colour.
pub const COLOR_MASK: u8 = 0x0F;

/// The 16 colour names; index = colour value. RGB values live only in the web UI.
pub const COLOR_NAMES: [&str; 16] = [
    "BLACK",
    "BLUE",
    "GREEN",
    "CYAN",
    "RED",
    "MAGENTA",
    "BROWN",
    "LIGHTGRAY",
    "DARKGRAY",
    "LIGHTBLUE",
    "LIGHTGREEN",
    "LIGHTCYAN",
    "LIGHTRED",
    "PINK",
    "YELLOW",
    "WHITE",
];

// ---------------------------------------------------------------------------------------------
// System calls
// ---------------------------------------------------------------------------------------------

pub const SYS_READ_INT: u16 = 1;
pub const SYS_WRITE_INT: u16 = 2;
pub const SYS_READ_CHAR: u16 = 3;
pub const SYS_WRITE_CHAR: u16 = 4;
pub const SYS_WRITE_STR: u16 = 5;
pub const SYS_ALLOC: u16 = 6;
pub const SYS_FREE: u16 = 7;
pub const SYS_RAND: u16 = 8;

/// (number, name) of every system call. Names are matched ignoring ASCII case in `sys`.
pub const SYSCALLS: [(u16, &str); 8] = [
    (SYS_READ_INT, "READ_INT"),
    (SYS_WRITE_INT, "WRITE_INT"),
    (SYS_READ_CHAR, "READ_CHAR"),
    (SYS_WRITE_CHAR, "WRITE_CHAR"),
    (SYS_WRITE_STR, "WRITE_STR"),
    (SYS_ALLOC, "ALLOC"),
    (SYS_FREE, "FREE"),
    (SYS_RAND, "RAND"),
];

/// Number of a system call name (ASCII case ignored).
pub fn syscall_number(name: &str) -> Option<u16> {
    SYSCALLS
        .iter()
        .find(|(_, n)| n.eq_ignore_ascii_case(name))
        .map(|(num, _)| *num)
}

/// Canonical name of a system call number.
pub fn syscall_name(number: u16) -> Option<&'static str> {
    SYSCALLS
        .iter()
        .find(|(num, _)| *num == number)
        .map(|(_, n)| *n)
}

// ---------------------------------------------------------------------------------------------
// Built-in assembler constants
// ---------------------------------------------------------------------------------------------

/// Built-in constants visible in every program (case-sensitive), without the colour names.
pub const BUILTIN_CONSTANTS: [(&str, i64); 6] = [
    ("SCREEN", SCREEN_START as i64),
    ("SCREEN_W", SCREEN_W as i64),
    ("SCREEN_H", SCREEN_H as i64),
    ("INPUT", INPUT_START as i64),
    ("HEAP", HEAP_START as i64),
    ("STACK_TOP", STACK_TOP as i64),
];

/// Value of a built-in constant or colour name (case-sensitive). Syscall names are not constants;
/// they are only valid as the operand of `sys`.
pub fn builtin_constant(name: &str) -> Option<i64> {
    if let Some((_, v)) = BUILTIN_CONSTANTS.iter().find(|(n, _)| *n == name) {
        return Some(*v);
    }
    COLOR_NAMES
        .iter()
        .position(|c| *c == name)
        .map(|i| i as i64)
}

/// Assembler directives (ASCII case ignored).
pub const DIRECTIVES: [&str; 9] = [
    ".text", ".data", ".word", ".byte", ".string", ".ascii", ".space", ".align", ".equ",
];

/// Pseudo-instructions (ASCII case ignored).
pub const PSEUDO_MNEMONICS: [&str; 1] = ["li"];

/// Name of the invisible `halt` word appended after the library (function tests return here).
pub const TRAP_LABEL: &str = "__trap";

/// `true` if `name` may not be used as a label or `.equ` name (`asm.reserved_name`):
/// register names, mnemonics (including condition aliases and `li`) and syscall names, all with
/// ASCII case ignored; built-in constants and colour names exactly; and names starting with `__`.
pub fn is_reserved_name(name: &str) -> bool {
    isa::Reg::from_name(name).is_some()
        || isa::is_mnemonic(name)
        || PSEUDO_MNEMONICS
            .iter()
            .any(|m| m.eq_ignore_ascii_case(name))
        || syscall_number(name).is_some()
        || builtin_constant(name).is_some()
        || name.starts_with("__")
}

// ---------------------------------------------------------------------------------------------
// Limits
// ---------------------------------------------------------------------------------------------

/// Maximum number of output tape entries (`fault.output_limit` beyond).
pub const OUTPUT_LIMIT: u32 = 8192;
/// Output tape entry kind: signed decimal number.
pub const OUT_KIND_INT: u8 = 0;
/// Output tape entry kind: byte (character).
pub const OUT_KIND_BYTE: u8 = 1;
/// Maximum string length for `WRITE_STR`, excluding the terminating 0.
pub const STRING_LIMIT: u32 = 4096;
/// Default cycle budget per test.
pub const DEFAULT_BUDGET: u64 = 200_000;
/// Maximum cycle budget per test.
pub const MAX_BUDGET: u64 = 10_000_000;
/// Maximum number of frames reported by the frame chain.
pub const MAX_FRAMES: u32 = 64;
/// The assembler stops after this many diagnostics and adds `asm.too_many_errors`.
pub const MAX_ASM_ERRORS: usize = 50;
/// Maximum Levenshtein distance for "did you mean" suggestions.
pub const SUGGESTION_DISTANCE: usize = 2;

/// Heap block alignment.
pub const HEAP_ALIGN: u32 = 8;
/// Red zone bytes after every heap block.
pub const HEAP_REDZONE: u32 = 8;
/// Heap shadow granule size in bytes.
pub const HEAP_GRANULE: u32 = 4;
/// Size of the heap shadow map in bytes (one byte per granule).
pub const HEAP_SHADOW_BYTES: u32 = (GUARD_START - HEAP_START) / HEAP_GRANULE;
/// Fill byte for freshly allocated heap memory.
pub const HEAP_ALLOC_FILL: u8 = 0xCD;
/// Fill byte for freed heap memory.
pub const HEAP_FREE_FILL: u8 = 0xDD;

/// Initial snapshot interval of the time-travel debugger (in steps).
pub const SNAPSHOT_INTERVAL: u64 = 1024;
/// Maximum number of snapshots before thinning.
pub const MAX_SNAPSHOTS: usize = 64;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segments_cover_memory_without_gaps() {
        assert_eq!(SEGMENTS[0].start, 0);
        for pair in SEGMENTS.windows(2) {
            assert_eq!(pair[0].end, pair[1].start);
        }
        assert_eq!(SEGMENTS[7].end, MEM_SIZE);
        for s in SEGMENTS {
            assert_eq!(s.start % PAGE_SIZE, 0);
            assert_eq!(s.end % PAGE_SIZE, 0);
        }
        assert_eq!(segment_of(0xFFFF).map(|s| s.segment), Some(Segment::Screen));
        assert_eq!(segment_of(0x0100).map(|s| s.segment), Some(Segment::Text));
        assert_eq!(segment_of(0x10000), None);
        assert_eq!(HEAP_SHADOW_BYTES, 4096);
    }

    #[test]
    fn names() {
        assert_eq!(syscall_number("write_int"), Some(2));
        assert_eq!(syscall_name(8), Some("RAND"));
        assert_eq!(syscall_name(9), None);
        assert_eq!(builtin_constant("SCREEN"), Some(0xF000));
        assert_eq!(builtin_constant("screen"), None);
        assert_eq!(builtin_constant("WHITE"), Some(15));
        assert!(is_reserved_name("ADD"));
        assert!(is_reserved_name("je"));
        assert!(is_reserved_name("Sp"));
        assert!(is_reserved_name("read_int"));
        assert!(is_reserved_name("YELLOW"));
        assert!(is_reserved_name("__trap"));
        assert!(!is_reserved_name("yellow"));
        assert!(!is_reserved_name("schleife"));
    }
}
