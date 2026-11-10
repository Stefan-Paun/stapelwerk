//! Minimal JSON writer (output only, no parser). Produces compact JSON with deterministic key
//! order (the order of the calls). Used by the WASM ABI and the CLI.
//!
//! ```
//! use stapelwerk_core::json::JsonWriter;
//! let mut w = JsonWriter::new();
//! w.begin_object();
//! w.field_str("key", "fault.div_zero");
//! w.key("args");
//! w.begin_object();
//! w.end_object();
//! w.field_null("mismatch");
//! w.end_object();
//! assert_eq!(w.finish(), r#"{"key":"fault.div_zero","args":{},"mismatch":null}"#);
//! ```

/// Streaming JSON writer. Misuse (e.g. a value without a key inside an object) produces invalid
/// JSON but never panics.
#[derive(Clone, Debug, Default)]
pub struct JsonWriter {
    out: String,
    /// One entry per open container: `true` while it is still empty.
    first: Vec<bool>,
    after_key: bool,
}

impl JsonWriter {
    /// Empty writer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the JSON text written so far.
    pub fn finish(self) -> String {
        self.out
    }

    /// Current text (for inspection).
    pub fn as_str(&self) -> &str {
        &self.out
    }

    fn pre_value(&mut self) {
        if self.after_key {
            self.after_key = false;
            return;
        }
        if let Some(first) = self.first.last_mut() {
            if !*first {
                self.out.push(',');
            }
            *first = false;
        }
    }

    /// Writes an object key; the next call must write its value.
    pub fn key(&mut self, k: &str) {
        self.pre_value();
        write_escaped(&mut self.out, k);
        self.out.push(':');
        self.after_key = true;
    }

    pub fn begin_object(&mut self) {
        self.pre_value();
        self.out.push('{');
        self.first.push(true);
    }

    pub fn end_object(&mut self) {
        self.first.pop();
        self.out.push('}');
    }

    pub fn begin_array(&mut self) {
        self.pre_value();
        self.out.push('[');
        self.first.push(true);
    }

    pub fn end_array(&mut self) {
        self.first.pop();
        self.out.push(']');
    }

    pub fn str(&mut self, v: &str) {
        self.pre_value();
        write_escaped(&mut self.out, v);
    }

    pub fn int(&mut self, v: i64) {
        self.pre_value();
        self.out.push_str(&v.to_string());
    }

    pub fn uint(&mut self, v: u64) {
        self.pre_value();
        self.out.push_str(&v.to_string());
    }

    pub fn bool(&mut self, v: bool) {
        self.pre_value();
        self.out.push_str(if v { "true" } else { "false" });
    }

    pub fn null(&mut self) {
        self.pre_value();
        self.out.push_str("null");
    }

    pub fn field_str(&mut self, k: &str, v: &str) {
        self.key(k);
        self.str(v);
    }

    pub fn field_int(&mut self, k: &str, v: i64) {
        self.key(k);
        self.int(v);
    }

    pub fn field_uint(&mut self, k: &str, v: u64) {
        self.key(k);
        self.uint(v);
    }

    pub fn field_bool(&mut self, k: &str, v: bool) {
        self.key(k);
        self.bool(v);
    }

    pub fn field_null(&mut self, k: &str) {
        self.key(k);
        self.null();
    }
}

/// Appends `s` as a JSON string literal. Escapes `"`, `\`, control characters, and U+2028/U+2029
/// (so the output is also safe inside JavaScript source).
pub fn write_escaped(out: &mut String, s: &str) {
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 || c == '\u{2028}' || c == '\u{2029}' => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_structures() {
        let mut w = JsonWriter::new();
        w.begin_object();
        w.field_bool("ok", true);
        w.field_int("size", -3);
        w.key("list");
        w.begin_array();
        w.uint(1);
        w.begin_object();
        w.field_str("a", "x");
        w.end_object();
        w.begin_array();
        w.end_array();
        w.null();
        w.end_array();
        w.end_object();
        assert_eq!(
            w.finish(),
            r#"{"ok":true,"size":-3,"list":[1,{"a":"x"},[],null]}"#
        );
    }

    #[test]
    fn escaping() {
        let mut s = String::new();
        write_escaped(&mut s, "a\"b\\c\n\u{1}\u{2028}ä<");
        assert_eq!(s, "\"a\\\"b\\\\c\\n\\u0001\\u2028ä<\"");
    }
}
