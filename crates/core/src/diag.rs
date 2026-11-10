//! Diagnostics shared by the assembler (`asm.*`), the level parser (`lvl.*`), the test comparison
//! (`check.*`), the ABI (`abi.*`) and run status (`run.*`).
//!
//! Only keys and arguments live in the core; all human-readable texts live in the UI's i18n files.
//! The complete list of keys with their argument names is [`ALL_KEYS`]; `docs/ERRORS.md` documents
//! exactly the same set (checked by a test).

use crate::json::JsonWriter;

/// File index of the player's source.
pub const FILE_PLAYER: u8 = 0;
/// File index of the level library (`[library]`).
pub const FILE_LIBRARY: u8 = 1;
/// File index used for `lvl.*` diagnostics (the `.lvl` file itself).
pub const FILE_LEVEL: u8 = 2;

/// Argument value of a diagnostic or fault.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Arg {
    Int(i64),
    Str(String),
}

impl From<i64> for Arg {
    fn from(v: i64) -> Self {
        Arg::Int(v)
    }
}

impl From<u32> for Arg {
    fn from(v: u32) -> Self {
        Arg::Int(v as i64)
    }
}

impl From<i32> for Arg {
    fn from(v: i32) -> Self {
        Arg::Int(v as i64)
    }
}

impl From<u64> for Arg {
    fn from(v: u64) -> Self {
        Arg::Int(v.min(i64::MAX as u64) as i64)
    }
}

impl From<&str> for Arg {
    fn from(v: &str) -> Self {
        Arg::Str(v.to_string())
    }
}

impl From<String> for Arg {
    fn from(v: String) -> Self {
        Arg::Str(v)
    }
}

/// Named arguments in a fixed order.
pub type Args = Vec<(&'static str, Arg)>;

/// Writes `args` as a JSON object (`{}` when empty).
pub fn write_args(w: &mut JsonWriter, args: &[(&'static str, Arg)]) {
    w.begin_object();
    for (name, value) in args {
        w.key(name);
        match value {
            Arg::Int(v) => w.int(*v),
            Arg::Str(s) => w.str(s),
        }
    }
    w.end_object();
}

/// An error found in source text (assembler or level file).
///
/// Positions: `line` and `col` are 1-based, `col` and `len` count **UTF-16 code units** (so the
/// browser editor can use them directly). `len` may be 0 for "at this position".
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    /// One of the keys in [`ALL_KEYS`], e.g. `asm.unknown_mnemonic`.
    pub key: &'static str,
    /// [`FILE_PLAYER`], [`FILE_LIBRARY`] or [`FILE_LEVEL`].
    pub file: u8,
    pub line: u32,
    pub col: u32,
    pub len: u32,
    pub args: Args,
}

impl Diagnostic {
    /// New diagnostic without arguments.
    pub fn new(key: &'static str, file: u8, line: u32, col: u32, len: u32) -> Self {
        Diagnostic {
            key,
            file,
            line,
            col,
            len,
            args: Vec::new(),
        }
    }

    /// Adds an argument (builder style).
    pub fn arg(mut self, name: &'static str, value: impl Into<Arg>) -> Self {
        self.args.push((name, value.into()));
        self
    }

    /// Writes `{key,file,line,col,len,args}`.
    pub fn write_json(&self, w: &mut JsonWriter) {
        w.begin_object();
        w.field_str("key", self.key);
        w.field_uint("file", self.file as u64);
        w.field_uint("line", self.line as u64);
        w.field_uint("col", self.col as u64);
        w.field_uint("len", self.len as u64);
        w.key("args");
        write_args(w, &self.args);
        w.end_object();
    }
}

/// Every key with its argument names (in output order). A trailing `?` marks an optional argument.
/// `fault.*` faults additionally always carry `pc` and `step` outside `args`.
pub const ALL_KEYS: &[(&str, &[&str])] = &[
    // Runtime faults (see fault.rs; order = fault code 1…23).
    ("fault.illegal_instruction", &["word"]),
    ("fault.zero_instruction", &[]),
    ("fault.exec_outside_code", &[]),
    ("fault.bad_jump_target", &["target"]),
    ("fault.bad_return_address", &["target"]),
    ("fault.null_access", &["addr", "access"]),
    ("fault.out_of_range", &["addr", "access"]),
    ("fault.guard_page", &["addr", "access"]),
    ("fault.write_to_code", &["addr"]),
    ("fault.misaligned", &["addr", "size"]),
    ("fault.stack_overflow", &["sp"]),
    ("fault.stack_underflow", &["sp"]),
    ("fault.stack_misaligned", &["sp"]),
    ("fault.div_zero", &[]),
    ("fault.input_end", &[]),
    ("fault.heap_unallocated", &["addr"]),
    ("fault.heap_overflow", &["addr"]),
    ("fault.use_after_free", &["addr"]),
    ("fault.invalid_free", &["addr"]),
    ("fault.double_free", &["addr"]),
    ("fault.unknown_syscall", &["n"]),
    ("fault.string_too_long", &["addr"]),
    ("fault.output_limit", &[]),
    // Run status (not a fault).
    ("run.budget_exceeded", &["cycles", "hottest_line"]),
    // Assembler.
    ("asm.unknown_mnemonic", &["name", "suggestion?"]),
    ("asm.unknown_register", &["name"]),
    ("asm.operand_count", &["expected", "got"]),
    ("asm.operand_type", &["expected"]),
    ("asm.imm_range", &["value", "min", "max"]),
    ("asm.undefined_symbol", &["name"]),
    ("asm.duplicate_label", &["name", "first_line"]),
    ("asm.reserved_name", &["name"]),
    ("asm.bad_number", &[]),
    ("asm.bad_string", &[]),
    ("asm.unexpected_char", &["ch"]),
    ("asm.local_label_without_global", &[]),
    ("asm.needs_constant", &[]),
    ("asm.expr_div_zero", &[]),
    ("asm.expr_overflow", &[]),
    ("asm.data_in_text", &[]),
    ("asm.code_in_data", &[]),
    ("asm.section_overflow", &["section", "size", "max"]),
    ("asm.forbidden_instruction", &["mnemonic"]),
    ("asm.too_many_errors", &[]),
    // Level files.
    ("lvl.unknown_section", &["name"]),
    ("lvl.unknown_key", &["key"]),
    ("lvl.missing_key", &["key"]),
    ("lvl.bad_value", &["key"]),
    ("lvl.duplicate_section", &[]),
    ("lvl.no_tests", &[]),
    ("lvl.allow_and_forbid", &[]),
    ("lvl.bad_image", &["name", "line"]),
    // Test comparison.
    ("check.out_mismatch", &["index", "expected", "actual"]),
    ("check.out_missing", &["index", "expected"]),
    ("check.out_extra", &["index", "actual"]),
    ("check.text_mismatch", &["offset", "expected", "actual"]),
    ("check.mem_mismatch", &["addr", "expected", "actual"]),
    ("check.reg_mismatch", &["reg", "expected", "actual"]),
    (
        "check.screen_mismatch",
        &["x", "y", "expected", "actual", "count"],
    ),
    ("check.heap_not_empty", &["count", "alloc_line"]),
    ("check.abi_clobbered", &["reg", "before", "after"]),
    ("check.not_halted", &["status"]),
    // WASM ABI.
    ("abi.bad_handle", &[]),
    ("abi.bad_arg", &[]),
    ("abi.not_assembled", &[]),
];

/// Argument names of a key, or `None` if the key is unknown.
pub fn key_args(key: &str) -> Option<&'static [&'static str]> {
    ALL_KEYS.iter().find(|(k, _)| *k == key).map(|(_, a)| *a)
}

/// `true` for every key in [`ALL_KEYS`].
pub fn is_known_key(key: &str) -> bool {
    key_args(key).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_unique_and_well_formed() {
        for (i, (k, _)) in ALL_KEYS.iter().enumerate() {
            let (family, name) = k.split_once('.').expect("family.name");
            assert!(
                ["fault", "run", "asm", "lvl", "check", "abi"].contains(&family),
                "{k}"
            );
            assert!(
                name.bytes().all(|b| b.is_ascii_lowercase() || b == b'_'),
                "{k}"
            );
            assert!(
                !ALL_KEYS[i + 1..].iter().any(|(o, _)| o == k),
                "duplicate {k}"
            );
        }
    }

    /// `docs/ERRORS.md` lists exactly the keys of [`ALL_KEYS`] with the same arguments.
    /// Line format: ``- `key` {arg1, arg2?} – text`` (braces omitted when there are no arguments).
    #[test]
    fn errors_md_matches_key_list() {
        let doc = include_str!("../../../docs/ERRORS.md");
        let mut documented: Vec<(String, Vec<String>)> = Vec::new();
        let mut in_fence = false;
        for line in doc.lines() {
            if line.starts_with("```") {
                in_fence = !in_fence;
                continue;
            }
            if in_fence {
                continue;
            }
            let Some(rest) = line.strip_prefix("- `") else {
                continue;
            };
            let Some((key, rest)) = rest.split_once('`') else {
                continue;
            };
            if !key.contains('.') {
                continue;
            }
            let rest = rest.trim_start();
            let args: Vec<String> = match rest.strip_prefix('{') {
                Some(r) => r
                    .split_once('}')
                    .map(|(inside, _)| inside)
                    .unwrap_or("")
                    .split(',')
                    .map(|a| a.trim().to_string())
                    .filter(|a| !a.is_empty())
                    .collect(),
                None => Vec::new(),
            };
            documented.push((key.to_string(), args));
        }
        let expected: Vec<(String, Vec<String>)> = ALL_KEYS
            .iter()
            .map(|(k, a)| (k.to_string(), a.iter().map(|s| s.to_string()).collect()))
            .collect();
        assert_eq!(documented, expected);
    }

    #[test]
    fn diagnostic_json() {
        let d = Diagnostic::new("asm.unknown_mnemonic", FILE_PLAYER, 3, 9, 4)
            .arg("name", "ad")
            .arg("suggestion", "add");
        let mut w = JsonWriter::new();
        d.write_json(&mut w);
        assert_eq!(
            w.finish(),
            r#"{"key":"asm.unknown_mnemonic","file":0,"line":3,"col":9,"len":4,"args":{"name":"ad","suggestion":"add"}}"#
        );
    }
}
