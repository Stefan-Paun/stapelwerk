//! `stapelwerk` command-line tool (`docs/LEVELS.md`, "Command-line tool").
//!
//! Exit codes: 0 success, 1 failure (diagnostics, failed checks, faulting programs), 2 usage
//! error.

mod args;
mod fuzz;
mod levels_cmds;
mod messages;
mod program_cmds;

use args::{CommandSpec, Opt, parse};
use std::io::Write;
use std::process::ExitCode;

const fn opt(name: &'static str, takes_value: bool, required: bool) -> Opt {
    Opt {
        name,
        takes_value,
        required,
    }
}

const COMMANDS: &[CommandSpec] = &[
    CommandSpec {
        name: "asm",
        usage: "asm <file.s> [-o out.img]",
        about: "Assemble a source file and write the memory image.",
        options: &[opt("-o", true, false)],
        min_positional: 1,
        max_positional: 1,
    },
    CommandSpec {
        name: "dis",
        usage: "dis <file.img|file.s>",
        about: "Disassemble an image or an assembled source file.",
        options: &[],
        min_positional: 1,
        max_positional: 1,
    },
    CommandSpec {
        name: "run",
        usage: "run <file.s> [--in \"3 4\"] [--in-text \"…\"] [--trace]",
        about: "Assemble and run a program, printing its output.",
        options: &[
            opt("--in", true, false),
            opt("--in-text", true, false),
            opt("--trace", false, false),
        ],
        min_positional: 1,
        max_positional: 1,
    },
    CommandSpec {
        name: "check-levels",
        usage: "check-levels [--strict] [--mutate] <dir|file>",
        about: "Parse and verify level files (reference passes, tests are strong enough).",
        options: &[opt("--strict", false, false), opt("--mutate", false, false)],
        min_positional: 1,
        max_positional: 1,
    },
    CommandSpec {
        name: "pack-levels",
        usage: "pack-levels <dir> <target-dir>",
        about: "Pack levels for the web (strip references, write index.json).",
        options: &[],
        min_positional: 2,
        max_positional: 2,
    },
    CommandSpec {
        name: "isa-table",
        usage: "isa-table",
        about: "Print the Markdown instruction table (docs/isa-table.md).",
        options: &[],
        min_positional: 0,
        max_positional: 0,
    },
    CommandSpec {
        name: "fuzz-gen",
        usage: "fuzz-gen --seed S --count N --out <dir>",
        about: "Generate random programs and digests for the differential test.",
        options: &[
            opt("--seed", true, true),
            opt("--count", true, true),
            opt("--out", true, true),
        ],
        min_positional: 0,
        max_positional: 0,
    },
];

fn usage() -> String {
    let mut s = String::from("Usage: stapelwerk <command> [options]\n\nCommands:\n");
    for c in COMMANDS {
        s.push_str(&format!("  {:<52} {}\n", c.usage, c.about));
    }
    s.push_str("\n  help | --help | -h      Show this help.\n  --version | -V          Show the version.\n");
    s
}

/// Runs the tool with the arguments after the program name. Returns the exit code.
fn run(args: &[String], out: &mut dyn Write, err: &mut dyn Write) -> u8 {
    let Some(cmd) = args.first() else {
        let _ = err.write_all(usage().as_bytes());
        return 2;
    };
    match cmd.as_str() {
        "help" | "--help" | "-h" => {
            let _ = out.write_all(usage().as_bytes());
            return 0;
        }
        "--version" | "-V" => {
            let _ = writeln!(out, "stapelwerk {}", env!("CARGO_PKG_VERSION"));
            return 0;
        }
        _ => {}
    }
    let Some(spec) = COMMANDS.iter().find(|c| c.name == cmd) else {
        let _ = writeln!(err, "stapelwerk: unknown command '{cmd}'\n");
        let _ = err.write_all(usage().as_bytes());
        return 2;
    };
    let parsed = match parse(spec, &args[1..]) {
        Ok(p) => p,
        Err(e) => {
            let _ = writeln!(err, "stapelwerk: {e}\nUsage: stapelwerk {}", spec.usage);
            return 2;
        }
    };
    match spec.name {
        "isa-table" => match out.write_all(stapelwerk_core::isa::isa_table_markdown().as_bytes()) {
            Ok(()) => 0,
            Err(_) => 1,
        },
        "asm" => program_cmds::cmd_asm(&parsed, out, err),
        "dis" => program_cmds::cmd_dis(&parsed, out, err),
        "run" => program_cmds::cmd_run(&parsed, out, err),
        "check-levels" => levels_cmds::cmd_check_levels(&parsed, out, err),
        "pack-levels" => levels_cmds::cmd_pack_levels(&parsed, out, err),
        "fuzz-gen" => {
            let seed = parsed.value("--seed").and_then(|s| s.parse::<u64>().ok());
            let count = parsed.value("--count").and_then(|s| s.parse::<u32>().ok());
            let (Some(seed), Some(count)) = (seed, count) else {
                let _ = writeln!(
                    err,
                    "stapelwerk: --seed must be a number 0…2^64-1 and --count a number 0…2^32-1"
                );
                return 2;
            };
            let dir = parsed.value("--out").unwrap_or(".");
            match fuzz::generate(seed, count, std::path::Path::new(dir)) {
                Ok(msg) => {
                    let _ = writeln!(out, "{msg}");
                    0
                }
                Err(e) => {
                    let _ = writeln!(err, "stapelwerk: {e}");
                    1
                }
            }
        }
        other => {
            let _ = writeln!(err, "stapelwerk: unknown command '{other}'");
            2
        }
    }
}

fn main() -> ExitCode {
    let mut args = Vec::new();
    for a in std::env::args_os().skip(1) {
        match a.into_string() {
            Ok(s) => args.push(s),
            Err(_) => {
                eprintln!("stapelwerk: arguments must be valid UTF-8");
                return ExitCode::from(2);
            }
        }
    }
    let code = run(
        &args,
        &mut std::io::stdout().lock(),
        &mut std::io::stderr().lock(),
    );
    ExitCode::from(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(args: &[&str]) -> (u8, String, String) {
        let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        let mut out = Vec::new();
        let mut err = Vec::new();
        let code = run(&args, &mut out, &mut err);
        (
            code,
            String::from_utf8(out).unwrap(),
            String::from_utf8(err).unwrap(),
        )
    }

    #[test]
    fn isa_table_works() {
        let (code, out, _) = call(&["isa-table"]);
        assert_eq!(code, 0);
        assert_eq!(out, stapelwerk_core::isa::isa_table_markdown());
    }

    #[test]
    fn missing_files_fail_cleanly() {
        for args in [
            &["asm", "does-not-exist.s"][..],
            &["dis", "does-not-exist.img"],
            &["run", "does-not-exist.s"],
        ] {
            let (code, _, err) = call(args);
            assert_eq!(code, 1, "{args:?}");
            assert!(err.contains("cannot read"), "{args:?}: {err}");
        }
        assert_eq!(call(&["check-levels", "does-not-exist"]).0, 2);
        assert_eq!(call(&["run", "x.s", "--in", "1 zwei"]).0, 2);
    }

    #[test]
    fn usage_errors() {
        assert_eq!(call(&[]).0, 2);
        assert_eq!(call(&["bogus"]).0, 2);
        assert_eq!(call(&["asm"]).0, 2);
        assert_eq!(call(&["isa-table", "extra"]).0, 2);
        assert_eq!(
            call(&["fuzz-gen", "--seed", "x", "--count", "1", "--out", "d"]).0,
            2
        );
        let (code, out, _) = call(&["--help"]);
        assert_eq!(code, 0);
        assert!(out.contains("check-levels"));
        assert_eq!(call(&["--version"]).0, 0);
    }
}
