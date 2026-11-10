//! JSON shapes of the ABI (`docs/ABI.md`, "JSON shapes") that the core does not write itself.

use stapelwerk_core::consts::SEGMENTS;
use stapelwerk_core::diag::Diagnostic;
use stapelwerk_core::disasm::DisasmLine;
use stapelwerk_core::json::JsonWriter;
use stapelwerk_core::program::{LineEntry, Symbol};
use stapelwerk_core::session::{Frame, HeapBlockInfo};

/// `{"error":"abi.…"}`
pub fn error(key: &str) -> String {
    let mut w = JsonWriter::new();
    w.begin_object();
    w.field_str("error", key);
    w.end_object();
    w.finish()
}

/// `segments`: `[{name, start, end, r, w, x}]`.
pub fn segments() -> String {
    let mut w = JsonWriter::new();
    w.begin_array();
    for s in &SEGMENTS {
        w.begin_object();
        w.field_str("name", s.name);
        w.field_uint("start", s.start as u64);
        w.field_uint("end", s.end as u64);
        w.field_bool("r", s.read);
        w.field_bool("w", s.write);
        w.field_bool("x", s.exec);
        w.end_object();
    }
    w.end_array();
    w.finish()
}

fn write_diagnostics(w: &mut JsonWriter, diags: &[Diagnostic]) {
    w.key("diagnostics");
    w.begin_array();
    for d in diags {
        d.write_json(w);
    }
    w.end_array();
}

/// `sess_assemble`: `{ok, size, diagnostics}`.
pub fn assembled(ok: bool, size: u32, diags: &[Diagnostic]) -> String {
    let mut w = JsonWriter::new();
    w.begin_object();
    w.field_bool("ok", ok);
    w.field_uint("size", size as u64);
    write_diagnostics(&mut w, diags);
    w.end_object();
    w.finish()
}

/// `level_load` on error: `{ok: false, diagnostics}`.
pub fn level_error(diags: &[Diagnostic]) -> String {
    let mut w = JsonWriter::new();
    w.begin_object();
    w.field_bool("ok", false);
    write_diagnostics(&mut w, diags);
    w.end_object();
    w.finish()
}

/// `sess_symbols`: `[{name, kind, value, file, line}]`.
pub fn symbols(symbols: &[Symbol]) -> String {
    let mut w = JsonWriter::new();
    w.begin_array();
    for s in symbols {
        w.begin_object();
        w.field_str("name", &s.name);
        w.field_str("kind", s.kind.as_str());
        w.field_int("value", s.value);
        w.field_uint("file", s.file as u64);
        w.field_uint("line", s.line as u64);
        w.end_object();
    }
    w.end_array();
    w.finish()
}

/// `sess_linemap`: `[{a, f, l}]`.
pub fn linemap(entries: &[LineEntry]) -> String {
    let mut w = JsonWriter::new();
    w.begin_array();
    for e in entries {
        w.begin_object();
        w.field_uint("a", e.addr as u64);
        w.field_uint("f", e.file as u64);
        w.field_uint("l", e.line as u64);
        w.end_object();
    }
    w.end_array();
    w.finish()
}

/// `sess_line_counts`: `[[line, count], …]`.
pub fn line_counts(counts: &[(u32, u64)]) -> String {
    let mut w = JsonWriter::new();
    w.begin_array();
    for &(line, count) in counts {
        w.begin_array();
        w.uint(line as u64);
        w.uint(count);
        w.end_array();
    }
    w.end_array();
    w.finish()
}

/// `sess_frames`: `[{fp, ret, func, size}]`.
pub fn frames(frames: &[Frame]) -> String {
    let mut w = JsonWriter::new();
    w.begin_array();
    for f in frames {
        w.begin_object();
        w.field_uint("fp", f.fp as u64);
        w.field_uint("ret", f.ret as u64);
        w.key("func");
        match &f.func {
            Some(name) => w.str(name),
            None => w.null(),
        }
        w.field_uint("size", f.size as u64);
        w.end_object();
    }
    w.end_array();
    w.finish()
}

/// `sess_heap`: `[{addr, size, live, line}]`.
pub fn heap(blocks: &[HeapBlockInfo]) -> String {
    let mut w = JsonWriter::new();
    w.begin_array();
    for b in blocks {
        w.begin_object();
        w.field_uint("addr", b.addr as u64);
        w.field_uint("size", b.size as u64);
        w.field_bool("live", b.live);
        w.field_uint("line", b.line as u64);
        w.end_object();
    }
    w.end_array();
    w.finish()
}

/// `sess_disasm`: `[{addr, word, text, label}]`.
pub fn disasm(lines: &[DisasmLine]) -> String {
    let mut w = JsonWriter::new();
    w.begin_array();
    for l in lines {
        w.begin_object();
        w.field_uint("addr", l.addr as u64);
        w.field_uint("word", l.word as u64);
        w.field_str("text", &l.text);
        w.key("label");
        match &l.label {
            Some(name) => w.str(name),
            None => w.null(),
        }
        w.end_object();
    }
    w.end_array();
    w.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shapes() {
        assert_eq!(error("abi.bad_handle"), r#"{"error":"abi.bad_handle"}"#);
        assert!(
            segments().starts_with(
                r#"[{"name":"null","start":0,"end":256,"r":false,"w":false,"x":false},"#
            )
        );
        assert_eq!(line_counts(&[(2, 1), (3, 5)]), "[[2,1],[3,5]]");
        assert_eq!(
            assembled(true, 3, &[]),
            r#"{"ok":true,"size":3,"diagnostics":[]}"#
        );
    }
}
