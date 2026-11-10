//! English messages for diagnostics, faults and test mismatches (the CLI's own texts; the game
//! UI has its own translations in `web/js/texts/`).
//!
//! Templates use `{name}` for an argument as is and `{name:x}` for a number in hexadecimal
//! (`0x` + at least four upper-case digits).

use stapelwerk_core::diag::{Arg, Diagnostic};
use stapelwerk_core::fault::Fault;
use stapelwerk_core::runner::{Mismatch, TestResult};

/// (key, template). Optional arguments are handled in [`describe`].
const TEMPLATES: &[(&str, &str)] = &[
    // Faults.
    (
        "fault.illegal_instruction",
        "illegal instruction word {word:x}",
    ),
    (
        "fault.zero_instruction",
        "executed a zero word (forgot halt?)",
    ),
    ("fault.exec_outside_code", "pc left the code"),
    (
        "fault.bad_jump_target",
        "invalid jump or call target {target:x}",
    ),
    (
        "fault.bad_return_address",
        "ret to invalid address {target:x} (unbalanced stack?)",
    ),
    (
        "fault.null_access",
        "access to the null segment at {addr:x} ({access})",
    ),
    (
        "fault.out_of_range",
        "address {addr:x} is out of range ({access})",
    ),
    (
        "fault.guard_page",
        "access to the guard segment at {addr:x} ({access})",
    ),
    (
        "fault.write_to_code",
        "write into the code segment at {addr:x}",
    ),
    ("fault.misaligned", "misaligned word access at {addr:x}"),
    ("fault.stack_overflow", "stack overflow (sp = {sp:x})"),
    ("fault.stack_underflow", "stack underflow (sp = {sp:x})"),
    (
        "fault.stack_misaligned",
        "misaligned stack pointer (sp = {sp:x})",
    ),
    ("fault.div_zero", "division by zero"),
    (
        "fault.input_end",
        "READ_INT found no number left in the input",
    ),
    (
        "fault.heap_unallocated",
        "access to unallocated heap memory at {addr:x}",
    ),
    (
        "fault.heap_overflow",
        "access past the end of a heap block at {addr:x}",
    ),
    (
        "fault.use_after_free",
        "use of freed heap memory at {addr:x}",
    ),
    (
        "fault.invalid_free",
        "FREE of {addr:x}, which is not the start of a heap block",
    ),
    (
        "fault.double_free",
        "FREE of {addr:x}, which is already freed",
    ),
    ("fault.unknown_syscall", "unknown system call {n}"),
    (
        "fault.string_too_long",
        "no terminating 0 within 4096 bytes of the string at {addr:x}",
    ),
    (
        "fault.output_limit",
        "the output already holds 8192 entries",
    ),
    // Run status.
    (
        "run.budget_exceeded",
        "cycle budget used up after {cycles} cycles (most executed: line {hottest_line})",
    ),
    // Assembler.
    ("asm.unknown_mnemonic", "unknown mnemonic '{name}'"),
    (
        "asm.unknown_register",
        "'{name}' is not a register (r0-r7, fp, sp)",
    ),
    (
        "asm.operand_count",
        "wrong number of operands: expected {expected}, got {got}",
    ),
    ("asm.operand_type", "wrong operand: expected {expected}"),
    ("asm.imm_range", "value {value} is outside {min}..{max}"),
    ("asm.undefined_symbol", "undefined symbol '{name}'"),
    (
        "asm.duplicate_label",
        "'{name}' is already defined in line {first_line}",
    ),
    ("asm.reserved_name", "'{name}' is a reserved name"),
    ("asm.bad_number", "malformed number"),
    ("asm.bad_string", "malformed string or character literal"),
    ("asm.unexpected_char", "unexpected character '{ch}'"),
    (
        "asm.local_label_without_global",
        "local label before any global label",
    ),
    (
        "asm.needs_constant",
        "the expression must be a constant known at this point",
    ),
    (
        "asm.expr_div_zero",
        "division by zero in a constant expression",
    ),
    ("asm.expr_overflow", "constant expression overflows"),
    (
        "asm.data_in_text",
        "data directive in the .text section (put .data before it)",
    ),
    (
        "asm.code_in_data",
        "instruction in the .data section (put .text before it)",
    ),
    (
        "asm.section_overflow",
        "section {section} is too large ({size} > {max} bytes)",
    ),
    (
        "asm.forbidden_instruction",
        "'{mnemonic}' is not allowed in this level",
    ),
    ("asm.too_many_errors", "too many errors, stopped here"),
    // Level files.
    ("lvl.unknown_section", "unknown section [{name}]"),
    ("lvl.unknown_key", "unknown key '{key}'"),
    ("lvl.missing_key", "missing key '{key}'"),
    ("lvl.bad_value", "invalid value for '{key}'"),
    ("lvl.duplicate_section", "this section may appear only once"),
    ("lvl.no_tests", "the level has no [test] section"),
    (
        "lvl.allow_and_forbid",
        "'allow' and 'forbid' cannot both be given",
    ),
    (
        "lvl.bad_image",
        "image '{name}': more than 64 rows or columns, or an invalid character in row {line}",
    ),
    // Test comparison.
    (
        "check.out_mismatch",
        "output #{index}: expected {expected}, got {actual}",
    ),
    (
        "check.out_missing",
        "output #{index} is missing (expected {expected})",
    ),
    (
        "check.out_extra",
        "unexpected extra output #{index}: {actual}",
    ),
    (
        "check.text_mismatch",
        "text output at byte {offset}: expected {expected}, got {actual} (-1 = none)",
    ),
    (
        "check.mem_mismatch",
        "memory at {addr:x}: expected {expected}, got {actual}",
    ),
    (
        "check.reg_mismatch",
        "register {reg}: expected {expected}, got {actual}",
    ),
    (
        "check.screen_mismatch",
        "screen differs at ({x}, {y}): expected colour {expected}, got {actual} ({count} pixels wrong)",
    ),
    (
        "check.heap_not_empty",
        "{count} heap block(s) not freed (first allocated in line {alloc_line})",
    ),
    (
        "check.abi_clobbered",
        "{reg} changed from {before:x} to {after:x}; r4-r7, fp and sp must be restored",
    ),
    ("check.not_halted", "the run ended with status {status}"),
];

fn arg<'a>(args: &'a [(&'static str, Arg)], name: &str) -> Option<&'a Arg> {
    args.iter().find(|(n, _)| *n == name).map(|(_, v)| v)
}

/// Fills a template.
fn fill(template: &str, args: &[(&'static str, Arg)]) -> String {
    let mut out = String::new();
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let Some(close) = rest[open..].find('}') else {
            out.push_str(&rest[open..]);
            return out;
        };
        let spec = &rest[open + 1..open + close];
        let (name, hex) = match spec.strip_suffix(":x") {
            Some(n) => (n, true),
            None => (spec, false),
        };
        match arg(args, name) {
            Some(Arg::Int(v)) if hex => out.push_str(&format!("0x{:04X}", *v as u32)),
            Some(Arg::Int(v)) => out.push_str(&v.to_string()),
            Some(Arg::Str(s)) => out.push_str(s),
            None => out.push('?'),
        }
        rest = &rest[open + close + 1..];
    }
    out.push_str(rest);
    out
}

/// English text for `key` with `args`, followed by ` [key]`.
pub fn describe(key: &str, args: &[(&'static str, Arg)]) -> String {
    let mut text = match TEMPLATES.iter().find(|(k, _)| *k == key) {
        Some((_, t)) => fill(t, args),
        None => key.to_string(),
    };
    if key == "asm.unknown_mnemonic"
        && let Some(Arg::Str(s)) = arg(args, "suggestion")
    {
        text.push_str(&format!(" (did you mean '{s}'?)"));
    }
    if key == "check.not_halted"
        && let (Some(Arg::Int(cycles)), Some(Arg::Int(line)), Some(Arg::Int(count))) = (
            arg(args, "cycles"),
            arg(args, "hottest_line"),
            arg(args, "hottest_count"),
        )
    {
        text.push_str(&format!(
            ": cycle budget used up after {cycles} cycles, line {line} ran {count} times"
        ));
    }
    if key == "asm.imm_range" && matches!(arg(args, "hint"), Some(Arg::Str(h)) if h == "li") {
        text.push_str(" (use 'li' for large constants)");
    }
    format!("{text} [{key}]")
}

/// `name:line:col: message [key]`.
pub fn diagnostic(name: &str, d: &Diagnostic) -> String {
    format!("{name}:{}:{}: {}", d.line, d.col, describe(d.key, &d.args))
}

pub fn fault(f: &Fault) -> String {
    describe(f.key(), &f.kind.args())
}

pub fn mismatch(m: &Mismatch) -> String {
    describe(m.key, &m.args)
}

/// Why a test failed, in one line.
pub fn test_failure(r: &TestResult) -> String {
    if let Some(f) = &r.fault {
        return format!("{} (pc 0x{:04X}, after {} steps)", fault(f), f.pc, f.step);
    }
    match &r.mismatch {
        Some(m) => mismatch(m),
        None => format!("status {}", r.status.name()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stapelwerk_core::diag::ALL_KEYS;

    #[test]
    fn every_key_has_a_message() {
        for (key, args) in ALL_KEYS {
            if key.starts_with("abi.") {
                continue;
            }
            let t = TEMPLATES
                .iter()
                .find(|(k, _)| k == key)
                .unwrap_or_else(|| panic!("no message for {key}"));
            // Every placeholder names a documented argument.
            let mut rest = t.1;
            while let Some(open) = rest.find('{') {
                let close = rest[open..].find('}').unwrap() + open;
                let name = rest[open + 1..close].trim_end_matches(":x");
                assert!(
                    args.iter().any(|a| a.trim_end_matches('?') == name),
                    "{key}: {name}"
                );
                rest = &rest[close + 1..];
            }
        }
    }

    #[test]
    fn fills_arguments() {
        let s = describe(
            "asm.unknown_mnemonic",
            &[("name", "ad".into()), ("suggestion", "add".into())],
        );
        assert_eq!(
            s,
            "unknown mnemonic 'ad' (did you mean 'add'?) [asm.unknown_mnemonic]"
        );
        let s = describe(
            "check.not_halted",
            &[
                ("status", "BUDGET_EXCEEDED".into()),
                ("cycles", Arg::Int(10000)),
                ("hottest_line", Arg::Int(9)),
                ("hottest_count", Arg::Int(3334)),
            ],
        );
        assert_eq!(
            s,
            "the run ended with status BUDGET_EXCEEDED: cycle budget used up after 10000 cycles, line 9 ran 3334 times [check.not_halted]"
        );
        let s = describe("check.mem_mismatch", &[("addr", Arg::Int(0x4000))]);
        assert!(s.starts_with("memory at 0x4000: expected ?"));
    }
}
