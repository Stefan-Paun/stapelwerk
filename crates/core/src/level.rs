//! Level files (`.lvl`): data model and parser. Format: `docs/LEVELS.md`.
//!
//! The data model is part of the phase-0 contract (the runner, the session and the CLI build on
//! it); [`parse`] is a placeholder for a later phase.

use crate::asm::MnemonicFilter;
use crate::consts::DEFAULT_BUDGET;
use crate::diag::{Diagnostic, FILE_LEVEL};
use crate::heap::HeapCheck;
use crate::isa::Reg;

/// `kind` in `[level]`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LevelKind {
    #[default]
    Normal,
    Tutorial,
    Debug,
    Final,
    Bonus,
}

impl LevelKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            LevelKind::Normal => "normal",
            LevelKind::Tutorial => "tutorial",
            LevelKind::Debug => "debug",
            LevelKind::Final => "final",
            LevelKind::Bonus => "bonus",
        }
    }

    pub fn parse(s: &str) -> Option<LevelKind> {
        Some(match s {
            "normal" => LevelKind::Normal,
            "tutorial" => LevelKind::Tutorial,
            "debug" => LevelKind::Debug,
            "final" => LevelKind::Final,
            "bonus" => LevelKind::Bonus,
            _ => return None,
        })
    }
}

/// Address in a test line: a number or a symbol of the assembled program.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AddrRef {
    Num(u32),
    Symbol(String),
}

/// `set ADR = …` (words) or `setb ADR = …` (bytes or a string), and the `expect`/`expectb`
/// counterparts. Words are stored little-endian when converted to bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MemValues {
    Words(Vec<u32>),
    Bytes(Vec<u8>),
}

/// One memory initialisation or expectation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemSpec {
    pub addr: AddrRef,
    pub values: MemValues,
}

/// `expect_screen = reference | NAME`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScreenExpect {
    /// Same screen as the reference solution after the same test (unpacked levels only).
    Reference,
    /// Named `[image.NAME]`.
    Image(String),
}

/// One `[test]` section.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TestCase {
    pub name: String,
    pub hidden: bool,
    /// `in = …`
    pub input: Vec<i32>,
    /// `in_text = "…"`
    pub in_text: Vec<u8>,
    /// `reg rX = v` (initial registers; for function tests r0–r3).
    pub regs: Vec<(Reg, u32)>,
    /// `set` / `setb`.
    pub sets: Vec<MemSpec>,
    /// `screen_init = NAME`
    pub screen_init: Option<String>,
    /// `call = LABEL` (function test).
    pub call: Option<String>,
    /// `out = …`; `None` = output not checked, `Some(vec![])` = output must be empty.
    pub out: Option<Vec<i32>>,
    /// `out_text = "…"` (output compared as bytes).
    pub out_text: Option<Vec<u8>>,
    /// `expect` / `expectb`.
    pub expects: Vec<MemSpec>,
    /// `expect_reg rX = v`.
    pub expect_regs: Vec<(Reg, u32)>,
    pub expect_screen: Option<ScreenExpect>,
    pub expect_heap_empty: bool,
    /// Overrides the level seed.
    pub seed: Option<u32>,
    /// Overrides the level budget.
    pub budget: Option<u64>,
}

/// A `[image.NAME]` section: 64×64 colour indices (missing pixels are 0).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Image {
    pub name: String,
    /// 4096 bytes, row-major, values 0…15.
    pub pixels: Vec<u8>,
}

/// A parsed level.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Level {
    pub id: String,
    pub version: u32,
    pub chapter: u32,
    pub order: u32,
    pub title_de: String,
    pub title_en: String,
    pub card_de: String,
    pub card_en: String,
    pub kind: LevelKind,
    /// Cycles per test (default 200000, max 10000000).
    pub budget: u64,
    pub seed: u32,
    pub filter: MnemonicFilter,
    pub heap_check: HeapCheck,
    pub mobile: bool,
    /// Only in packed files.
    pub ref_cycles: Option<u64>,
    /// Only in packed files.
    pub ref_size: Option<u32>,
    pub tests: Vec<TestCase>,
    pub images: Vec<Image>,
    /// Raw text sections in file order: `start`, `library`, `reference`, `text.de`, `text.en`,
    /// `hint.de`, `hint.en`, `learned.de`, `learned.en` (name, content).
    pub sections: Vec<(String, String)>,
}

impl Default for Level {
    fn default() -> Self {
        Level {
            id: String::new(),
            version: 1,
            chapter: 0,
            order: 1,
            title_de: String::new(),
            title_en: String::new(),
            card_de: String::new(),
            card_en: String::new(),
            kind: LevelKind::Normal,
            budget: DEFAULT_BUDGET,
            seed: 0,
            filter: MnemonicFilter::All,
            heap_check: HeapCheck::Strict,
            mobile: false,
            ref_cycles: None,
            ref_size: None,
            tests: Vec::new(),
            images: Vec::new(),
            sections: Vec::new(),
        }
    }
}

impl Level {
    /// Raw content of a text section (e.g. `"library"`).
    pub fn section(&self, name: &str) -> Option<&str> {
        self.sections
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, c)| c.as_str())
    }

    /// Image by name.
    pub fn image(&self, name: &str) -> Option<&Image> {
        self.images.iter().find(|i| i.name == name)
    }
}

/// Section names that may appear in `[…]` headers, besides `image.NAME`.
pub const SECTION_NAMES: [&str; 11] = [
    "level",
    "test",
    "start",
    "library",
    "reference",
    "text.de",
    "text.en",
    "hint.de",
    "hint.en",
    "learned.de",
    "learned.en",
];

/// Prefix of image section headers (`[image.NAME]`).
pub const IMAGE_SECTION_PREFIX: &str = "image.";

/// Parses a `.lvl` file. All `lvl.*` diagnostics use [`FILE_LEVEL`]. Later phase; returns
/// `lvl.no_tests` until implemented.
pub fn parse(_src: &str) -> Result<Level, Vec<Diagnostic>> {
    Err(vec![Diagnostic::new("lvl.no_tests", FILE_LEVEL, 1, 1, 0)])
}
