//! The assembled program: the contract between the assembler (phase 1B) and the machine (phase 1A).
//!
//! The assembler produces a [`Program`]; the machine loads it with
//! [`crate::machine::MachineState::new`]. Neither side needs the other's internals.

use crate::consts::{DATA_START, TEXT_START};
use crate::diag::FILE_PLAYER;

/// What a symbol names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SymbolKind {
    /// Label in `.text` (address of an instruction).
    Code,
    /// Label in `.data`.
    Data,
    /// `.equ` constant.
    Const,
}

impl SymbolKind {
    /// `"code"`, `"data"` or `"const"` (JSON).
    pub const fn as_str(self) -> &'static str {
        match self {
            SymbolKind::Code => "code",
            SymbolKind::Data => "data",
            SymbolKind::Const => "const",
        }
    }
}

/// A label or constant defined by the player or the library.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Symbol {
    /// Full name; local labels are stored as `global.local`.
    pub name: String,
    pub kind: SymbolKind,
    /// Address for labels; value for constants (expressions are evaluated in i64).
    pub value: i64,
    /// [`crate::diag::FILE_PLAYER`] or [`crate::diag::FILE_LIBRARY`].
    pub file: u8,
    /// 1-based source line of the definition.
    pub line: u32,
}

impl Symbol {
    /// `true` for local labels (`global.local`).
    pub fn is_local(&self) -> bool {
        self.name.contains('.')
    }
}

/// Source location of one instruction word.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LineEntry {
    /// Address of the instruction word.
    pub addr: u32,
    /// [`crate::diag::FILE_PLAYER`] or [`crate::diag::FILE_LIBRARY`].
    pub file: u8,
    /// 1-based source line.
    pub line: u32,
}

/// Result of assembling player code plus optional library.
///
/// Layout (fixed by the spec): `words[0]` is at [`TEXT_START`] (0x0100), first the player's
/// instructions, then the library's, then one `halt` word at `trap_addr` (label `__trap`).
/// `data[0]` is at [`DATA_START`] (0x4000): the player's `.data`, then the library's.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Program {
    /// Encoded instruction words from 0x0100 (player, library, trap).
    pub words: Vec<u32>,
    /// Occupied length of the text segment in bytes (`words.len() * 4`). The code region for
    /// jump-target checks and execution is `TEXT_START .. TEXT_START + text_len`.
    pub text_len: u32,
    /// Initial bytes of the data segment from 0x4000.
    pub data: Vec<u8>,
    /// All symbols (player, library, `__trap`), in definition order.
    pub symbols: Vec<Symbol>,
    /// One entry per instruction word except the trap word, sorted by address.
    pub linemap: Vec<LineEntry>,
    /// Address of the trap `halt` word (`__trap`).
    pub trap_addr: u32,
    /// Size metric: number of instruction words from the player's file (`li` counts 2).
    pub size: u32,
}

impl Program {
    /// Address one past the last instruction word.
    pub const fn code_end(&self) -> u32 {
        TEXT_START + self.text_len
    }

    /// Address of `data[0]`.
    pub const fn data_start(&self) -> u32 {
        DATA_START
    }

    /// Looks up a symbol by exact name.
    pub fn symbol(&self, name: &str) -> Option<&Symbol> {
        self.symbols.iter().find(|s| s.name == name)
    }

    /// Source location of the instruction at `addr`.
    pub fn line_of(&self, addr: u32) -> Option<LineEntry> {
        self.linemap
            .binary_search_by_key(&addr, |e| e.addr)
            .ok()
            .and_then(|i| self.linemap.get(i).copied())
    }

    /// Nearest global (non-local) code label at or below `addr`, for frame and disassembly names.
    pub fn function_at(&self, addr: u32) -> Option<&Symbol> {
        self.symbols
            .iter()
            .filter(|s| s.kind == SymbolKind::Code && !s.is_local() && s.value <= addr as i64)
            .max_by_key(|s| s.value)
    }

    /// Lines of the player's file that produced instruction words (for line counters).
    pub fn player_lines(&self) -> impl Iterator<Item = &LineEntry> {
        self.linemap.iter().filter(|e| e.file == FILE_PLAYER)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Program {
        Program {
            words: vec![0x1100_0001, 0x0100_0000, 0x0100_0000],
            text_len: 12,
            data: vec![],
            symbols: vec![
                Symbol {
                    name: "main".into(),
                    kind: SymbolKind::Code,
                    value: 0x100,
                    file: 0,
                    line: 1,
                },
                Symbol {
                    name: "main.end".into(),
                    kind: SymbolKind::Code,
                    value: 0x104,
                    file: 0,
                    line: 3,
                },
                Symbol {
                    name: "__trap".into(),
                    kind: SymbolKind::Code,
                    value: 0x108,
                    file: 1,
                    line: 0,
                },
            ],
            linemap: vec![
                LineEntry {
                    addr: 0x100,
                    file: 0,
                    line: 2,
                },
                LineEntry {
                    addr: 0x104,
                    file: 0,
                    line: 3,
                },
            ],
            trap_addr: 0x108,
            size: 2,
        }
    }

    #[test]
    fn lookups() {
        let p = sample();
        assert_eq!(p.code_end(), 0x10C);
        assert_eq!(p.line_of(0x104).map(|e| e.line), Some(3));
        assert_eq!(p.line_of(0x102), None);
        assert_eq!(p.function_at(0x104).map(|s| s.name.as_str()), Some("main"));
        assert_eq!(
            p.function_at(0x108).map(|s| s.name.as_str()),
            Some("__trap")
        );
        assert_eq!(p.player_lines().count(), 2);
    }
}
