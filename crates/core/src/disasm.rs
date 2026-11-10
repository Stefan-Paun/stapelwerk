//! Disassembler: canonical text form of instructions (`docs/ISA.md`, "Canonical text form").
//! The assembler must accept every line this module prints and produce the same word again
//! (`asm(dis(w)) == w`).
//!
//! PHASE 1B owns this file. Placeholders are marked `PHASE 1B`.

use crate::isa::{Instr, decode};
use crate::program::Program;

/// One disassembled word.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DisasmLine {
    pub addr: u32,
    pub word: u32,
    /// Canonical text, or `.word 0x…` for words that do not decode.
    pub text: String,
    /// Label defined at `addr`, if any.
    pub label: Option<String>,
}

/// Absolute target of a J-format instruction at `addr` with word offset `off`
/// (may lie outside 0…0xFFFF; printed in decimal then).
pub const fn jump_target(addr: u32, off: i32) -> i64 {
    addr as i64 + 4 * off as i64
}

/// Canonical text of `instr` located at `addr` (needed for jump targets). PHASE 1B.
pub fn format_instr(_instr: &Instr, _addr: u32) -> String {
    String::new()
}

/// Canonical text of `word` at `addr`, or `None` if the word does not decode.
pub fn disasm_word(word: u32, addr: u32) -> Option<String> {
    decode(word).ok().map(|i| format_instr(&i, addr))
}

/// Disassembles `count` words from `addr` out of raw memory, naming labels from `program`.
/// PHASE 1B.
pub fn disasm_range(
    _mem: &[u8],
    _program: Option<&Program>,
    _addr: u32,
    _count: u32,
) -> Vec<DisasmLine> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jump_targets() {
        assert_eq!(jump_target(0x10C, -3), 0x100);
        assert_eq!(jump_target(0x100, -(1 << 19)), 0x100 - (1 << 21));
    }
}
