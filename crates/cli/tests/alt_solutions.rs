//! Every file in `tests/solutions/<level-id>.s` is a second, independently written solution to
//! that level: written from the task text and hints only, without looking at the `[reference]`.
//! Each one must pass all tests of its level through the real binary. This keeps the claim
//! "every level is solvable from its text" honest when levels or the machine change.

use std::path::{Path, PathBuf};
use std::process::Command;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn second_solutions_pass_their_levels() {
    let dir = repo().join("tests/solutions");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("tests/solutions exists")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "s"))
        .collect();
    files.sort();
    assert!(
        files.len() >= 28,
        "expected at least 28 second solutions, found {}",
        files.len()
    );

    let mut failures = Vec::new();
    for file in &files {
        let id = file.file_stem().unwrap().to_string_lossy().into_owned();
        let level = repo().join("levels").join(format!("{id}.lvl"));
        assert!(level.exists(), "no level for solution {id}");
        let out = Command::new(env!("CARGO_BIN_EXE_stapelwerk"))
            .arg("run")
            .arg(file)
            .arg("--level")
            .arg(&level)
            .output()
            .expect("run stapelwerk");
        if !out.status.success() {
            failures.push(format!(
                "{id}:\n{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "second solutions failed:\n{}",
        failures.join("\n")
    );
}
