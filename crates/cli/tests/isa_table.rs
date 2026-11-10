//! `stapelwerk isa-table` must reproduce `docs/isa-table.md` byte for byte
//! (regenerate with `cargo run -q -p stapelwerk-cli -- isa-table > docs/isa-table.md`).

use std::process::Command;

#[test]
fn isa_table_matches_docs() {
    let out = Command::new(env!("CARGO_BIN_EXE_stapelwerk"))
        .arg("isa-table")
        .output()
        .expect("run stapelwerk");
    assert!(out.status.success());
    let expected = include_str!("../../../docs/isa-table.md");
    assert!(
        out.stdout == expected.as_bytes(),
        "docs/isa-table.md is stale; regenerate it with `stapelwerk isa-table`"
    );
}

#[test]
fn unknown_command_exits_with_2() {
    let out = Command::new(env!("CARGO_BIN_EXE_stapelwerk"))
        .arg("no-such-command")
        .output()
        .expect("run stapelwerk");
    assert_eq!(out.status.code(), Some(2));
}
