//! Level files (`.lvl`): data model and parser. Format: `docs/LEVELS.md`.
//!
//! [`parse`] never panics; every problem becomes a `lvl.*` [`Diagnostic`] with `file` =
//! [`FILE_LEVEL`] and the 1-based line of the `.lvl` file. Packing for the web build, the
//! manifest and `index.json` live in [`pack`].

pub mod pack;

use crate::asm::MnemonicFilter;
use crate::asm::lexer::{TokenKind, parse_number, tokenize_line, utf16_col};
use crate::consts::{DEFAULT_BUDGET, MAX_BUDGET, MEM_SIZE, SCREEN_BYTES, SCREEN_H, SCREEN_W};
use crate::diag::{Diagnostic, FILE_LEVEL};
use crate::heap::HeapCheck;
use crate::isa::{Reg, is_mnemonic};
use crate::json::JsonWriter;
use crate::session::TestSetup;

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

impl MemValues {
    /// Number of bytes covered.
    pub fn byte_len(&self) -> u32 {
        match self {
            MemValues::Words(w) => (w.len() as u32).saturating_mul(4),
            MemValues::Bytes(b) => b.len() as u32,
        }
    }

    /// The values as little-endian bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            MemValues::Words(w) => w.iter().flat_map(|v| v.to_le_bytes()).collect(),
            MemValues::Bytes(b) => b.clone(),
        }
    }
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

impl TestCase {
    /// The debugger session's view of this test (`session::Session::reset`): input, registers,
    /// memory, the `screen_init` image's pixels, `call`, and the seed/budget overrides. `index` is
    /// the test's position in the level.
    pub fn to_setup(&self, level: &Level, index: u32) -> TestSetup {
        TestSetup {
            index,
            input: self.input.clone(),
            in_text: self.in_text.clone(),
            regs: self.regs.clone(),
            sets: self.sets.clone(),
            screen: self
                .screen_init
                .as_deref()
                .and_then(|n| level.image(n))
                .map(|i| i.pixels.clone()),
            call: self.call.clone(),
            seed: self.seed,
            budget: self.budget,
        }
    }
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

    /// The hints of `lang` (`"de"` or `"en"`), split at lines `---`; empty if the section is
    /// missing.
    pub fn hints(&self, lang: &str) -> Vec<String> {
        match self.section(&format!("hint.{lang}")) {
            Some(text) => split_hints(text),
            None => Vec::new(),
        }
    }

    /// Effective budget of a test.
    pub fn test_budget(&self, t: &TestCase) -> u64 {
        t.budget.unwrap_or(self.budget)
    }

    /// Effective seed of a test.
    pub fn test_seed(&self, t: &TestCase) -> u32 {
        t.seed.unwrap_or(self.seed)
    }

    /// JSON for the ABI export `level_info` (`docs/ABI.md`).
    pub fn info_json(&self) -> String {
        let mut w = JsonWriter::new();
        self.write_info_json(&mut w);
        w.finish()
    }

    /// Writes the `level_info` object.
    pub fn write_info_json(&self, w: &mut JsonWriter) {
        w.begin_object();
        w.field_str("id", &self.id);
        w.field_uint("version", self.version as u64);
        w.field_uint("chapter", self.chapter as u64);
        w.field_uint("order", self.order as u64);
        w.field_str("kind", self.kind.as_str());
        write_lang_pair(w, "title", &self.title_de, &self.title_en);
        write_lang_pair(w, "card", &self.card_de, &self.card_en);
        w.field_uint("budget", self.budget);
        w.field_uint("seed", self.seed as u64);
        w.field_bool("mobile", self.mobile);
        w.field_str("heap_check", self.heap_check.as_str());
        for (key, list) in [
            (
                "allow",
                match &self.filter {
                    MnemonicFilter::Allow(l) => Some(l),
                    _ => None,
                },
            ),
            (
                "forbid",
                match &self.filter {
                    MnemonicFilter::Forbid(l) => Some(l),
                    _ => None,
                },
            ),
        ] {
            w.key(key);
            match list {
                Some(l) => {
                    w.begin_array();
                    for m in l {
                        w.str(m);
                    }
                    w.end_array();
                }
                None => w.null(),
            }
        }
        w.key("tests");
        w.begin_array();
        for (i, t) in self.tests.iter().enumerate().filter(|(_, t)| !t.hidden) {
            w.begin_object();
            w.field_uint("index", i as u64);
            w.field_str("name", &t.name);
            w.end_object();
        }
        w.end_array();
        w.field_uint(
            "hidden_tests",
            self.tests.iter().filter(|t| t.hidden).count() as u64,
        );
        w.key("ref_cycles");
        match self.ref_cycles {
            Some(v) => w.uint(v),
            None => w.null(),
        }
        w.key("ref_size");
        match self.ref_size {
            Some(v) => w.uint(v as u64),
            None => w.null(),
        }
        w.key("sections");
        w.begin_array();
        for (name, _) in self.sections.iter().filter(|(n, _)| n != "reference") {
            w.str(name);
        }
        for img in &self.images {
            w.str(&format!("{IMAGE_SECTION_PREFIX}{}", img.name));
        }
        w.end_array();
        w.end_object();
    }
}

fn write_lang_pair(w: &mut JsonWriter, key: &str, de: &str, en: &str) {
    w.key(key);
    w.begin_object();
    w.field_str("de", de);
    w.field_str("en", en);
    w.end_object();
}

/// Splits a hint section at lines that are exactly `---` (trailing whitespace ignored); every
/// part is trimmed of surrounding blank lines.
pub fn split_hints(text: &str) -> Vec<String> {
    let mut parts = vec![String::new()];
    for line in text.split('\n') {
        if line.trim_end() == "---" {
            parts.push(String::new());
        } else if let Some(cur) = parts.last_mut() {
            cur.push_str(line);
            cur.push('\n');
        }
    }
    parts
        .into_iter()
        .map(|p| trim_blank_lines(&p).to_string())
        .collect()
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

/// Number of hints per language.
pub const HINT_COUNT: usize = 3;

/// Maximum number of `lvl.*` diagnostics reported for one file.
const MAX_LEVEL_ERRORS: usize = 50;

/// `true` for names allowed in `[image.NAME]`, test names and level ids: ASCII letters, digits,
/// `_`, `-` and `.`, not empty, at most 96 characters (room for the `__ref_` prefix).
pub fn is_valid_name(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 96
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-' || b == b'.')
}

/// What a header line names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Header<'a> {
    /// A known section (`level`, `test`, …, `image.NAME`).
    Known(&'a str),
    /// Looks like a header but names no known section.
    Unknown(&'a str),
}

/// Header recognition shared by the parser and the packer: a line that is `[name]` (surrounding
/// whitespace ignored).
pub(crate) fn header_of(line: &str) -> Option<Header<'_>> {
    let t = line.trim();
    let inner = t.strip_prefix('[')?.strip_suffix(']')?;
    if inner.is_empty() || inner.contains('[') || inner.contains(']') {
        return None;
    }
    if SECTION_NAMES.contains(&inner) {
        return Some(Header::Known(inner));
    }
    if let Some(name) = inner.strip_prefix(IMAGE_SECTION_PREFIX)
        && is_valid_name(name)
    {
        return Some(Header::Known(inner));
    }
    Some(Header::Unknown(inner))
}

/// Removes leading and trailing blank lines (lines containing only whitespace) but keeps the
/// indentation of the first non-blank line.
fn trim_blank_lines(s: &str) -> &str {
    let mut start = 0;
    for line in s.split_inclusive('\n') {
        if line.trim().is_empty() {
            start += line.len();
        } else {
            break;
        }
    }
    let rest = s.get(start..).unwrap_or("");
    rest.trim_end()
}

/// Byte offset of the first `;` outside a double-quoted string.
fn comment_start(line: &str) -> usize {
    let bytes = line.as_bytes();
    let mut in_str = false;
    let mut i = 0;
    while let Some(&b) = bytes.get(i) {
        match b {
            b'\\' if in_str => i += 1,
            b'"' => in_str = !in_str,
            b';' if !in_str => return i,
            _ => {}
        }
        i += 1;
    }
    line.len()
}

fn utf16_len(s: &str) -> u32 {
    s.encode_utf16().count() as u32
}

/// One `key [argument] = value` line.
struct Kv<'a> {
    key: &'a str,
    key_col: u32,
    arg: Option<&'a str>,
    value: &'a str,
    /// Column and length of the value (or of the end of the line if the value is empty).
    val_col: u32,
    val_len: u32,
}

/// Why a line in a key-value section is malformed.
enum KvError<'a> {
    /// No `=`, or more than two words left of it; carries the first word.
    Shape(&'a str, u32),
}

/// Splits a key-value line. `Ok(None)` for blank and comment-only lines.
fn split_kv(line: &str) -> Result<Option<Kv<'_>>, KvError<'_>> {
    let body = line.get(..comment_start(line)).unwrap_or("");
    if body.trim().is_empty() {
        return Ok(None);
    }
    // Words left of `=` with their byte offsets.
    let eq = body.find('=');
    let left = body.get(..eq.unwrap_or(body.len())).unwrap_or("");
    let mut words: Vec<(usize, &str)> = Vec::new();
    let mut pos = 0;
    for w in left.split_whitespace() {
        let off = left
            .get(pos..)
            .and_then(|r| r.find(w))
            .map_or(pos, |o| o + pos);
        words.push((off, w));
        pos = off + w.len();
    }
    let Some(&(key_off, key)) = words.first() else {
        let col = utf16_col(line, eq.unwrap_or(0));
        return Err(KvError::Shape("", col));
    };
    let key_col = utf16_col(line, key_off);
    let Some(eq) = eq else {
        return Err(KvError::Shape(key, key_col));
    };
    if words.len() > 2 {
        return Err(KvError::Shape(key, key_col));
    }
    let right = body.get(eq + 1..).unwrap_or("");
    let value = right.trim();
    let lead = right.len() - right.trim_start().len();
    let val_off = eq + 1 + lead;
    Ok(Some(Kv {
        key,
        key_col,
        arg: words.get(1).map(|&(_, a)| a),
        value,
        val_col: utf16_col(line, val_off),
        val_len: utf16_len(value),
    }))
}

/// Integer with optional sign: decimal, `0x…`, `0b…`, `_` between digits.
pub fn parse_int(s: &str) -> Option<i64> {
    let s = s.trim();
    if let Some(r) = s.strip_prefix('-') {
        parse_number(r).map(|v| -v)
    } else {
        parse_number(s.strip_prefix('+').unwrap_or(s))
    }
}

fn parse_in_range(s: &str, lo: i64, hi: i64) -> Option<i64> {
    parse_int(s).filter(|v| (lo..=hi).contains(v))
}

/// Space-separated integers in `lo..=hi`.
fn parse_list(s: &str, lo: i64, hi: i64) -> Option<Vec<i64>> {
    s.split_whitespace()
        .map(|w| parse_in_range(w, lo, hi))
        .collect()
}

/// A double-quoted string with the assembler's escapes (`\n \t \0 \\ \" \' \xHH`) → bytes.
pub fn parse_string(s: &str) -> Option<Vec<u8>> {
    if !s.starts_with('"') {
        return None;
    }
    let toks = tokenize_line(s, FILE_LEVEL, 0).ok()?;
    match toks.as_slice() {
        [t] => match &t.kind {
            TokenKind::Str(b) => Some(b.clone()),
            _ => None,
        },
        _ => None,
    }
}

/// Address argument: a number 0…0xFFFF or a symbol name.
fn parse_addr(s: &str) -> Option<AddrRef> {
    let first = s.bytes().next()?;
    if first.is_ascii_digit() {
        return parse_in_range(s, 0, MEM_SIZE as i64 - 1).map(|v| AddrRef::Num(v as u32));
    }
    let ok = (first.is_ascii_alphabetic() || first == b'_' || first == b'.')
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'.');
    ok.then(|| AddrRef::Symbol(s.to_string()))
}

/// Label name for `call`.
fn is_label(s: &str) -> bool {
    let mut b = s.bytes();
    matches!(b.next(), Some(c) if c.is_ascii_alphabetic() || c == b'_')
        && s.bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'.')
}

fn yes_no(s: &str) -> Option<bool> {
    match s {
        "yes" => Some(true),
        "no" => Some(false),
        _ => None,
    }
}

/// Register value: −2³¹…2³²−1, stored as u32.
fn parse_word(s: &str) -> Option<u32> {
    parse_in_range(s, i32::MIN as i64, u32::MAX as i64).map(|v| v as u32)
}

fn parse_words(s: &str) -> Option<Vec<u32>> {
    let v = parse_list(s, i32::MIN as i64, u32::MAX as i64)?;
    (!v.is_empty()).then(|| v.into_iter().map(|x| x as u32).collect())
}

fn parse_bytes(s: &str) -> Option<Vec<u8>> {
    if s.starts_with('"') {
        return parse_string(s);
    }
    let v = parse_list(s, -128, 255)?;
    (!v.is_empty()).then(|| v.into_iter().map(|x| x as u8).collect())
}

const LEVEL_KEYS: [&str; 17] = [
    "id",
    "version",
    "chapter",
    "order",
    "title.de",
    "title.en",
    "card.de",
    "card.en",
    "kind",
    "budget",
    "seed",
    "allow",
    "forbid",
    "heap_check",
    "mobile",
    "ref_cycles",
    "ref_size",
];

const TEST_KEYS: [&str; 18] = [
    "name",
    "hidden",
    "in",
    "in_text",
    "reg",
    "set",
    "setb",
    "screen_init",
    "call",
    "out",
    "out_text",
    "expect",
    "expectb",
    "expect_reg",
    "expect_screen",
    "expect_heap_empty",
    "seed",
    "budget",
];

/// Test keys that take an argument (`key ARG = value`); they may also repeat.
const ARG_KEYS: [&str; 6] = ["reg", "set", "setb", "expect", "expectb", "expect_reg"];

/// Where the parser currently is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Cur {
    /// Before the first header.
    Start,
    Level,
    Test(usize),
    Text(usize),
    Image(usize),
    /// Content of a duplicate section (ignored).
    Skip,
}

/// Bookkeeping for one `[test]` section.
#[derive(Default)]
struct TestMeta {
    header_line: u32,
    seen: Vec<&'static str>,
    /// (line, col, len) of `reg` lines with registers outside r0–r3 (only an error for function
    /// tests, which is known at the end of the section).
    high_regs: Vec<(u32, u32, u32)>,
    /// (line, col, len) of `screen_init` and `expect_screen` values, checked at the end.
    screen_init_at: (u32, u32, u32),
    expect_screen_at: (u32, u32, u32),
}

/// An image section before parsing: (name, header line, (line number, row) list).
type RawImage<'a> = (String, u32, Vec<(u32, &'a str)>);

struct Parser<'a> {
    level: Level,
    diags: Vec<Diagnostic>,
    level_header: Option<u32>,
    level_seen: Vec<&'static str>,
    allow_or_forbid: Option<&'static str>,
    tests: Vec<TestMeta>,
    /// Text sections: (name, header line, content lines).
    texts: Vec<(String, u32, Vec<&'a str>)>,
    /// Images: (name, header line, (line number, row) list).
    images: Vec<RawImage<'a>>,
}

/// Parses a `.lvl` file. All `lvl.*` diagnostics use [`FILE_LEVEL`]; they are sorted by line and
/// limited to 50. Never panics.
pub fn parse(src: &str) -> Result<Level, Vec<Diagnostic>> {
    let src = src.strip_prefix('\u{FEFF}').unwrap_or(src);
    let mut p = Parser {
        level: Level::default(),
        diags: Vec::new(),
        level_header: None,
        level_seen: Vec::new(),
        allow_or_forbid: None,
        tests: Vec::new(),
        texts: Vec::new(),
        images: Vec::new(),
    };
    let mut cur = Cur::Start;
    for (i, raw) in src.split('\n').enumerate() {
        let no = (i as u32).saturating_add(1);
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        let kv_context = matches!(cur, Cur::Start | Cur::Level | Cur::Test(_));
        match header_of(line) {
            Some(Header::Known(name)) => {
                cur = p.open_section(name, no, line);
                continue;
            }
            Some(Header::Unknown(name)) if kv_context => {
                p.diags.push(
                    Diagnostic::new("lvl.unknown_section", FILE_LEVEL, no, 1, utf16_len(line))
                        .arg("name", name),
                );
                cur = Cur::Skip;
                continue;
            }
            _ => {}
        }
        match cur {
            Cur::Start => p.stray_line(no, line),
            Cur::Level => p.level_line(no, line),
            Cur::Test(t) => p.test_line(t, no, line),
            Cur::Text(t) => {
                if let Some(s) = p.texts.get_mut(t) {
                    s.2.push(line);
                }
            }
            Cur::Image(t) => {
                if let Some(s) = p.images.get_mut(t) {
                    s.2.push((no, line));
                }
            }
            Cur::Skip => {}
        }
    }
    p.finish()
}

impl<'a> Parser<'a> {
    fn err(&mut self, key: &'static str, line: u32, col: u32, len: u32) -> &mut Diagnostic {
        self.diags
            .push(Diagnostic::new(key, FILE_LEVEL, line, col, len));
        let n = self.diags.len() - 1;
        &mut self.diags[n]
    }

    fn bad_value(&mut self, key: &str, line: u32, col: u32, len: u32) {
        let d = Diagnostic::new("lvl.bad_value", FILE_LEVEL, line, col, len).arg("key", key);
        self.diags.push(d);
    }

    fn open_section(&mut self, name: &str, no: u32, line: &str) -> Cur {
        let dup = |p: &mut Parser<'a>| {
            p.err("lvl.duplicate_section", no, 1, utf16_len(line));
            Cur::Skip
        };
        match name {
            "level" => {
                if self.level_header.is_some() {
                    return dup(self);
                }
                self.level_header = Some(no);
                Cur::Level
            }
            "test" => {
                self.level.tests.push(TestCase::default());
                self.tests.push(TestMeta {
                    header_line: no,
                    ..TestMeta::default()
                });
                Cur::Test(self.level.tests.len() - 1)
            }
            _ => {
                if let Some(img) = name.strip_prefix(IMAGE_SECTION_PREFIX) {
                    if self.images.iter().any(|(n, _, _)| n == img) {
                        return dup(self);
                    }
                    self.images.push((img.to_string(), no, Vec::new()));
                    return Cur::Image(self.images.len() - 1);
                }
                if self.texts.iter().any(|(n, _, _)| n == name) {
                    return dup(self);
                }
                self.texts.push((name.to_string(), no, Vec::new()));
                Cur::Text(self.texts.len() - 1)
            }
        }
    }

    /// A line before the first section header: only blank lines and comments are allowed.
    fn stray_line(&mut self, no: u32, line: &str) {
        match split_kv(line) {
            Ok(None) => {}
            Ok(Some(kv)) => {
                let len = utf16_len(kv.key);
                self.err("lvl.unknown_key", no, kv.key_col, len)
                    .args
                    .push(("key", kv.key.into()));
            }
            Err(KvError::Shape(word, col)) => {
                let len = utf16_len(word);
                self.err("lvl.unknown_key", no, col, len)
                    .args
                    .push(("key", word.into()));
            }
        }
    }

    /// Splits a key-value line and checks the key against `known`. Returns the canonical key.
    fn kv<'l>(
        &mut self,
        no: u32,
        line: &'l str,
        known: &[&'static str],
    ) -> Option<(&'static str, Kv<'l>)> {
        let kv = match split_kv(line) {
            Ok(None) => return None,
            Ok(Some(kv)) => kv,
            Err(KvError::Shape(word, col)) => {
                let len = utf16_len(word);
                if known.contains(&word) {
                    self.bad_value(word, no, col, len);
                } else {
                    self.err("lvl.unknown_key", no, col, len)
                        .args
                        .push(("key", word.into()));
                }
                return None;
            }
        };
        let Some(&key) = known.iter().find(|k| **k == kv.key) else {
            let len = utf16_len(kv.key);
            self.err("lvl.unknown_key", no, kv.key_col, len)
                .args
                .push(("key", kv.key.into()));
            return None;
        };
        // An argument only for the argument keys, and always for them.
        if kv.arg.is_some() != ARG_KEYS.contains(&key) {
            self.bad_value(key, no, kv.key_col, utf16_len(kv.key));
            return None;
        }
        Some((key, kv))
    }

    fn level_line(&mut self, no: u32, line: &str) {
        let Some((key, kv)) = self.kv(no, line, &LEVEL_KEYS) else {
            return;
        };
        let (vc, vl) = (kv.val_col, kv.val_len);
        if self.level_seen.contains(&key) {
            self.bad_value(key, no, kv.key_col, utf16_len(kv.key));
            return;
        }
        self.level_seen.push(key);
        let v = kv.value;
        if key == "allow" || key == "forbid" {
            if self.allow_or_forbid.is_some() {
                self.err("lvl.allow_and_forbid", no, kv.key_col, utf16_len(kv.key));
                return;
            }
            self.allow_or_forbid = Some(key);
            let list: Vec<String> = v.split_whitespace().map(str::to_ascii_lowercase).collect();
            let ok = !list.is_empty() && list.iter().all(|m| is_mnemonic(m) || m == "li");
            self.level.filter = if key == "allow" {
                MnemonicFilter::Allow(list)
            } else {
                MnemonicFilter::Forbid(list)
            };
            if !ok {
                self.bad_value(key, no, vc, vl);
            }
            return;
        }
        let l = &mut self.level;
        let ok = match key {
            "id" => {
                let ok = is_valid_name(v);
                l.id = v.to_string();
                ok
            }
            "version" => {
                l.version = 1;
                v == "1"
            }
            "chapter" => set(&mut l.chapter, parse_in_range(v, 0, 7)),
            "order" => set(&mut l.order, parse_in_range(v, 1, 999)),
            "title.de" => text(&mut l.title_de, v),
            "title.en" => text(&mut l.title_en, v),
            "card.de" => text(&mut l.card_de, v),
            "card.en" => text(&mut l.card_en, v),
            "kind" => LevelKind::parse(v).map(|k| l.kind = k).is_some(),
            "budget" => set(&mut l.budget, parse_in_range(v, 1, MAX_BUDGET as i64)),
            "seed" => set(&mut l.seed, parse_in_range(v, 0, u32::MAX as i64)),
            "heap_check" => HeapCheck::parse(v).map(|h| l.heap_check = h).is_some(),
            "mobile" => yes_no(v).map(|m| l.mobile = m).is_some(),
            "ref_cycles" => parse_in_range(v, 0, i64::MAX)
                .map(|c| l.ref_cycles = Some(c as u64))
                .is_some(),
            "ref_size" => parse_in_range(v, 0, u32::MAX as i64)
                .map(|c| l.ref_size = Some(c as u32))
                .is_some(),
            _ => true,
        };
        if !ok {
            self.bad_value(key, no, vc, vl);
        }
    }

    fn test_line(&mut self, t: usize, no: u32, line: &str) {
        let Some((key, kv)) = self.kv(no, line, &TEST_KEYS) else {
            return;
        };
        let (vc, vl) = (kv.val_col, kv.val_len);
        let Some(meta) = self.tests.get_mut(t) else {
            return;
        };
        if !ARG_KEYS.contains(&key) {
            if meta.seen.contains(&key) {
                self.bad_value(key, no, kv.key_col, utf16_len(kv.key));
                return;
            }
            meta.seen.push(key);
        }
        let Some(tc) = self.level.tests.get_mut(t) else {
            return;
        };
        let v = kv.value;
        let arg = kv.arg.unwrap_or("");
        let ok = match key {
            "name" => {
                tc.name = v.to_string();
                // At most 64 characters, so `__ref_NAME` is still a valid image name.
                is_valid_name(v) && v.len() <= 64
            }
            "hidden" => yes_no(v).map(|h| tc.hidden = h).is_some(),
            "in" => parse_list(v, i32::MIN as i64, i32::MAX as i64)
                .map(|l| tc.input = l.into_iter().map(|x| x as i32).collect())
                .is_some(),
            "in_text" => parse_string(v).map(|b| tc.in_text = b).is_some(),
            "out" => parse_list(v, i32::MIN as i64, i32::MAX as i64)
                .map(|l| tc.out = Some(l.into_iter().map(|x| x as i32).collect()))
                .is_some(),
            "out_text" => parse_string(v).map(|b| tc.out_text = Some(b)).is_some(),
            "reg" | "expect_reg" => match (Reg::from_name(arg), parse_word(v)) {
                (Some(r), Some(val)) => {
                    if key == "reg" {
                        tc.regs.push((r, val));
                        if r.index() > 3 {
                            meta.high_regs.push((no, kv.key_col, utf16_len(kv.key)));
                        }
                    } else {
                        tc.expect_regs.push((r, val));
                    }
                    true
                }
                _ => false,
            },
            "set" | "setb" | "expect" | "expectb" => {
                let values = if key.ends_with('b') {
                    parse_bytes(v).map(MemValues::Bytes)
                } else {
                    parse_words(v).map(MemValues::Words)
                };
                match (parse_addr(arg), values) {
                    (Some(addr), Some(values)) => {
                        let spec = MemSpec { addr, values };
                        if key.starts_with("set") {
                            tc.sets.push(spec);
                        } else {
                            tc.expects.push(spec);
                        }
                        true
                    }
                    _ => false,
                }
            }
            "screen_init" => {
                meta.screen_init_at = (no, vc, vl);
                tc.screen_init = Some(v.to_string());
                is_valid_name(v)
            }
            "expect_screen" => {
                meta.expect_screen_at = (no, vc, vl);
                tc.expect_screen = Some(if v == "reference" {
                    ScreenExpect::Reference
                } else {
                    ScreenExpect::Image(v.to_string())
                });
                is_valid_name(v)
            }
            "call" => {
                tc.call = Some(v.to_string());
                is_label(v)
            }
            "expect_heap_empty" => yes_no(v).map(|h| tc.expect_heap_empty = h).is_some(),
            "seed" => parse_in_range(v, 0, u32::MAX as i64)
                .map(|s| tc.seed = Some(s as u32))
                .is_some(),
            "budget" => parse_in_range(v, 1, MAX_BUDGET as i64)
                .map(|b| tc.budget = Some(b as u64))
                .is_some(),
            _ => true,
        };
        if !ok {
            self.bad_value(key, no, vc, vl);
        }
    }

    fn finish(mut self) -> Result<Level, Vec<Diagnostic>> {
        // [level]
        match self.level_header {
            Some(line) if !self.level_seen.contains(&"id") => {
                self.err("lvl.missing_key", line, 1, 7)
                    .args
                    .push(("key", "id".into()));
            }
            None => {
                self.err("lvl.missing_key", 1, 1, 0)
                    .args
                    .push(("key", "id".into()));
            }
            _ => {}
        }
        // Images.
        for (name, header, rows) in std::mem::take(&mut self.images) {
            match parse_image(&name, &rows) {
                Ok(img) => self.level.images.push(img),
                Err((row, line)) => {
                    let line = line.unwrap_or(header);
                    self.err("lvl.bad_image", line, 1, 0)
                        .args
                        .extend([("name", name.as_str().into()), ("line", row.into())]);
                }
            }
        }
        // Text sections.
        for (name, header, lines) in std::mem::take(&mut self.texts) {
            let content = trim_blank_lines(&lines.join("\n")).to_string();
            if name.starts_with("hint.") {
                let parts = split_hints(&content);
                if parts.len() != HINT_COUNT || parts.iter().any(|p| p.is_empty()) {
                    let len = utf16_len(&name) + 2;
                    self.bad_value(&name, header, 1, len);
                }
            }
            self.level.sections.push((name, content));
        }
        // Tests.
        if self.level.tests.is_empty() {
            self.err("lvl.no_tests", 1, 1, 0);
        }
        let has_reference = self.level.section("reference").is_some();
        let tests = std::mem::take(&mut self.tests);
        for (t, meta) in tests.iter().enumerate() {
            let Some(tc) = self.level.tests.get(t).cloned() else {
                continue;
            };
            if !meta.seen.contains(&"name") {
                self.err("lvl.missing_key", meta.header_line, 1, 6)
                    .args
                    .push(("key", "name".into()));
            }
            if tc.call.is_some() {
                for &(line, col, len) in &meta.high_regs {
                    self.bad_value("reg", line, col, len);
                }
            }
            if let Some(name) = &tc.screen_init
                && is_valid_name(name)
                && self.level.image(name).is_none()
            {
                let (l, c, n) = meta.screen_init_at;
                self.bad_value("screen_init", l, c, n);
            }
            let screen_ok = match &tc.expect_screen {
                Some(ScreenExpect::Reference) => has_reference,
                Some(ScreenExpect::Image(name)) => {
                    !is_valid_name(name) || self.level.image(name).is_some()
                }
                None => true,
            };
            if !screen_ok {
                let (l, c, n) = meta.expect_screen_at;
                self.bad_value("expect_screen", l, c, n);
            }
        }
        if self.diags.is_empty() {
            return Ok(self.level);
        }
        self.diags.sort_by_key(|d| (d.line, d.col));
        self.diags.truncate(MAX_LEVEL_ERRORS);
        Err(self.diags)
    }
}

fn set<T: TryFrom<i64>>(slot: &mut T, v: Option<i64>) -> bool {
    match v.and_then(|v| T::try_from(v).ok()) {
        Some(v) => {
            *slot = v;
            true
        }
        None => false,
    }
}

fn text(slot: &mut String, v: &str) -> bool {
    *slot = v.to_string();
    !v.is_empty()
}

/// Parses image rows. Error: (1-based row, file line of that row).
fn parse_image(name: &str, rows: &[(u32, &str)]) -> Result<Image, (u32, Option<u32>)> {
    // Trailing blank rows do not count.
    let used = rows
        .iter()
        .rposition(|(_, r)| !r.trim().is_empty())
        .map_or(0, |i| i + 1);
    let mut pixels = vec![0u8; SCREEN_BYTES as usize];
    for (y, &(line, row)) in rows.iter().take(used).enumerate() {
        let row_no = y as u32 + 1;
        if y as u32 >= SCREEN_H {
            return Err((row_no, Some(line)));
        }
        let row = row.trim_end();
        if row.chars().count() as u32 > SCREEN_W {
            return Err((row_no, Some(line)));
        }
        for (x, ch) in row.chars().enumerate() {
            let v = match ch {
                '.' => 0,
                '0'..='9' => ch as u8 - b'0',
                'A'..='F' => ch as u8 - b'A' + 10,
                _ => return Err((row_no, Some(line))),
            };
            if let Some(p) = pixels.get_mut(y * SCREEN_W as usize + x) {
                *p = v;
            }
        }
    }
    Ok(Image {
        name: name.to_string(),
        pixels,
    })
}

/// Writes an image as section rows: `0`–`9`/`A`–`F`, `.` for 0, trailing `.` and trailing empty
/// rows omitted (an empty row inside is written as a single `.`).
pub fn image_rows(pixels: &[u8]) -> Vec<String> {
    let mut rows: Vec<String> = pixels
        .chunks(SCREEN_W as usize)
        .take(SCREEN_H as usize)
        .map(|row| {
            let s: String = row
                .iter()
                .map(|&p| match p & 0x0F {
                    0 => '.',
                    v @ 0..=9 => (b'0' + v) as char,
                    v => (b'A' + v - 10) as char,
                })
                .collect();
            let t = s.trim_end_matches('.');
            if t.is_empty() {
                ".".to_string()
            } else {
                t.to_string()
            }
        })
        .collect();
    while rows.last().is_some_and(|r| r == ".") {
        rows.pop();
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kv_lines() {
        let kv = split_kv("  in_text = \"a;b\"  ; comment")
            .ok()
            .flatten()
            .unwrap();
        assert_eq!(kv.key, "in_text");
        assert_eq!(kv.value, "\"a;b\"");
        assert_eq!(kv.val_col, 13);
        let kv = split_kv("reg r1 = -5").ok().flatten().unwrap();
        assert_eq!((kv.key, kv.arg, kv.value), ("reg", Some("r1"), "-5"));
        assert!(split_kv("; only a comment").ok().unwrap().is_none());
        assert!(split_kv("hidden").is_err());
        assert!(split_kv("a b c = 1").is_err());
    }

    #[test]
    fn values() {
        assert_eq!(parse_int("-0x10"), Some(-16));
        assert_eq!(parse_string("\"a\\n\\x41\""), Some(b"a\nA".to_vec()));
        assert_eq!(parse_string("\"open"), None);
        assert_eq!(parse_addr("0x4000"), Some(AddrRef::Num(0x4000)));
        assert_eq!(parse_addr("0x10000"), None);
        assert_eq!(parse_addr("MAX"), Some(AddrRef::Symbol("MAX".into())));
        assert_eq!(parse_words("-1 2"), Some(vec![u32::MAX, 2]));
        assert_eq!(parse_bytes("300"), None);
        assert_eq!(trim_blank_lines("\n\n  a\nb\n\n"), "  a\nb");
    }

    #[test]
    fn headers() {
        assert_eq!(header_of("[level]"), Some(Header::Known("level")));
        assert_eq!(header_of(" [image.x-1] "), Some(Header::Known("image.x-1")));
        assert_eq!(header_of("[image.]"), Some(Header::Unknown("image.")));
        assert_eq!(header_of("[foo]"), Some(Header::Unknown("foo")));
        assert_eq!(header_of("ld r0, [r1]"), None);
        assert_eq!(header_of("[]"), None);
    }

    #[test]
    fn image_round_trip() {
        let mut px = vec![0u8; 4096];
        px[0] = 15;
        px[64 * 3 + 5] = 10;
        let rows = image_rows(&px);
        assert_eq!(rows, vec!["F", ".", ".", ".....A"]);
        let lines: Vec<(u32, &str)> = rows.iter().map(|r| (1, r.as_str())).collect();
        assert_eq!(parse_image("x", &lines).unwrap().pixels, px);
    }
}
