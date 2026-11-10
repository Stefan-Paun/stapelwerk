//! Native unit tests of the export functions. Pointers are truncated to 32 bits on 64-bit hosts;
//! the buffer registry compares truncated addresses consistently, so `(ptr, len)` round trips
//! still work. Memory behind returned pointers is not dereferenced here (that is tested under
//! Node against the real module, `test/wasm/`).

use super::*;
use stapelwerk_core::machine::Status;

const LEVEL: &str = include_str!("../../../levels/2-2-bis-zur-null.lvl");

fn out() -> String {
    with(|st| String::from_utf8_lossy(&st.out).into_owned())
}

/// Allocates an input buffer holding `bytes`; returns `(ptr, len)`.
fn put(bytes: &[u8]) -> (u32, u32) {
    let len = bytes.len() as u32;
    let p = buf_alloc(len);
    with(|st| {
        let b = st
            .bufs
            .iter_mut()
            .find(|b| addr(b.as_ptr()) == p && b.len() == bytes.len())
            .expect("buffer registered");
        b.copy_from_slice(bytes);
    });
    (p, len)
}

fn call_text<R>(text: &str, f: impl FnOnce(u32, u32) -> R) -> R {
    let (p, l) = put(text.as_bytes());
    let r = f(p, l);
    buf_free(p, l);
    r
}

fn load_level(text: &str) -> u32 {
    call_text(text, |p, l| level_load(p, l))
}

fn assemble(s: u32, src: &str) -> u32 {
    call_text(src, |p, l| sess_assemble(s, p, l))
}

fn reference_source() -> String {
    level::parse(LEVEL)
        .unwrap()
        .section("reference")
        .unwrap()
        .to_string()
}

#[test]
fn module_basics() {
    assert_eq!(abi_version(), 1);
    let n = segments();
    assert_eq!(n, out_len());
    assert!(out().starts_with(r#"[{"name":"null""#));
    assert_eq!(buf_alloc(0), 0);
    let live = with(|st| st.bufs.len());
    let (p, l) = put(b"abc");
    buf_free(p, l + 1); // wrong length: ignored
    assert_eq!(with(|st| st.bufs.len()), live + 1);
    buf_free(p, l);
    assert_eq!(with(|st| st.bufs.len()), live);
}

#[test]
fn level_load_and_queries() {
    assert_eq!(load_level("[level]\nid = x\n"), 0);
    let diag = out();
    assert!(
        diag.starts_with(r#"{"ok":false,"diagnostics":[{"key":"lvl."#),
        "{diag}"
    );

    let lv = load_level(LEVEL);
    assert_ne!(lv, 0);
    level_info(lv);
    let info = out();
    assert!(info.starts_with(r#"{"id":"2-2-bis-zur-null","version":1,"chapter":2"#));
    assert!(info.contains(r#""hidden_tests":11"#));
    let n = call_text("start", |p, l| level_section(lv, p, l));
    assert!(n > 0);
    assert!(out().contains("halt"));
    assert_eq!(call_text("reference", |p, l| level_section(lv, p, l)), -1);
    assert_eq!(call_text("nope", |p, l| level_section(lv, p, l)), -1);
    // A session handle is not a level handle.
    let s = sess_new(0);
    level_info(s);
    assert_eq!(out(), r#"{"error":"abi.bad_handle"}"#);
    sess_free(s);
    level_free(lv);
    level_info(lv);
    assert_eq!(out(), r#"{"error":"abi.bad_handle"}"#);
    assert_eq!(sess_new(lv), ERR);
}

#[test]
fn reference_passes_and_empty_program_fails() {
    let lv = load_level(LEVEL);
    let s = sess_new(lv);
    // Freeing the level does not break the session.
    level_free(lv);
    assert_eq!(sess_run_tests(s), out_len());
    assert_eq!(out(), r#"{"error":"abi.not_assembled"}"#);
    assert_eq!(sess_reset(s, 0), Status::NotAssembled.code());

    assert_eq!(assemble(s, &reference_source()), 0);
    assert!(out().starts_with(r#"{"ok":true,"size":9,"#), "{}", out());
    sess_run_tests(s);
    let report = out();
    assert!(report.starts_with(r#"{"passed":true,"tests":[{"index":0,"name":"grundfall""#));
    assert!(report.contains(r#""reference":{"cycles":"#));

    assert_eq!(sess_reset(s, 0), Status::Paused.code());
    sess_check(s);
    assert!(out().contains(r#""passed":true"#));
    assert_eq!(sess_run(s, 1_000_000), Status::Halted.code());
    assert_eq!(sess_reset(s, 15), ERR, "only 15 tests");
    assert_eq!(sess_reset(s, -2), ERR);
    assert_eq!(sess_reset(s, -1), Status::Paused.code());
    sess_check(s);
    assert_eq!(out(), "null");
    assert_ne!(sess_reference_screen(s, 0), 0);
    assert_eq!(sess_reference_screen(s, 99), ERR);

    assert_eq!(assemble(s, "        halt\n"), 0);
    sess_run_tests(s);
    assert!(out().starts_with(r#"{"passed":false,"#));
    assert!(out().contains(r#""mismatch":{"key":"check.out_missing""#));
    sess_free(s);
    assert_eq!(sess_step(s, 1), ERR);
}

#[test]
fn diagnostics_have_positions() {
    let s = sess_new(0);
    assert_eq!(assemble(s, "        mov r1, 0\n        ad r1, 2\n"), 1);
    let json = out();
    assert!(
        json.contains(r#""key":"asm.unknown_mnemonic","file":0,"line":2,"col":9"#),
        "{json}"
    );
    assert_eq!(sess_step(s, 1), Status::NotAssembled.code());
    sess_free(s);
}

#[test]
fn sandbox_input_and_errors() {
    let s = sess_new(0);
    assert_eq!(
        assemble(
            s,
            "        sys READ_INT\n        sys WRITE_INT\n        halt\n"
        ),
        0
    );
    assert_eq!(sess_run(s, 1000), Status::WaitingInput.code());
    assert_eq!(sess_in_int(s, 41), Status::WaitingInput.code());
    assert_eq!(sess_run(s, 1000), Status::Halted.code());
    assert_eq!(with(|st| st.sess(s).unwrap().view.out_len), 1);
    assert_eq!(
        call_text("x", |p, l| sess_in_text(s, p, l)),
        Status::Halted.code()
    );
    assert_eq!(sess_in_text(s, 12, 3), ERR, "unknown buffer");
    assert_eq!(sess_set_reg(s, 12, 0), ERR, "bad register");
    assert_eq!(sess_set_reg(s, 0, 1), ERR, "halted");
    assert_eq!(sess_back_to_change(s, 3, 0), ERR, "bad kind");
    assert_eq!(sess_back_to_change(s, 0, 0), Status::Paused.code());
    assert_eq!(sess_bp(s, 0x100, 2), ERR);
    assert_eq!(sess_bp(s, 0x104, 1), 1);
    assert_eq!(sess_bp(s, 0x4000, 1), 0);
    sess_disasm(s, 0x10000, 1);
    assert_eq!(out(), r#"{"error":"abi.bad_arg"}"#);
    sess_disasm(s, 0x100, 2);
    assert!(out().starts_with(r#"[{"addr":256,"#));
    assert_eq!(sess_reference_screen(s, 0), 0);
    sess_run_tests(s);
    assert!(out().starts_with(r#"{"passed":false,"tests":[],"#));
    sess_digest(s);
    assert_eq!(out().len(), 16);
    assert_eq!(call_text("ab", |_, l| sess_assemble(s, 7, l)), ERR);
    assert_eq!(out(), r#"{"error":"abi.bad_arg"}"#);
    sess_free(s);
    sess_fault(s);
    assert_eq!(out(), r#"{"error":"abi.bad_handle"}"#);
    assert_eq!(sess_view(s), ERR);
}
