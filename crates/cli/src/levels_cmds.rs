//! `check-levels` and `pack-levels` (`docs/LEVELS.md`, "Checking and packing").

use crate::args::Parsed;
use crate::messages;
use stapelwerk_core::consts::{SEGMENTS, Segment, segment_of};
use stapelwerk_core::diag::{Diagnostic, FILE_LIBRARY, FILE_PLAYER};
use stapelwerk_core::isa::{Instr, decode};
use stapelwerk_core::level::pack::{Manifest, index_json, parse_manifest};
use stapelwerk_core::level::{HINT_COUNT, Level, LevelKind, MemValues, parse};
use stapelwerk_core::program::{Program, SymbolKind};
use stapelwerk_core::runner::{
    PackError, Reference, ScreenRef, assemble_level, pack_level, resolve_addr, run_reference,
    run_test_machine, run_tests_with,
};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Maximum number of `; mutate:skip` lines per level.
const MAX_MUTATE_SKIPS: usize = 2;

/// Marker in a reference line's comment that exempts it from the mutation test.
const MUTATE_SKIP: &str = "mutate:skip";

/// First content line (1-based) of section `name` in `src`, if the section exists. Matches the
/// parser, which drops leading blank lines of text sections.
fn section_first_line(src: &str, name: &str) -> Option<u32> {
    let header = format!("[{name}]");
    let mut lines = src.lines().enumerate();
    lines.find(|(_, l)| l.trim() == header)?;
    lines
        .find(|(_, l)| !l.trim().is_empty())
        .map(|(i, _)| i as u32 + 1)
}

/// Formats assembler diagnostics of level code with file lines of the `.lvl` file.
fn code_diags(file: &str, src: &str, section: &str, diags: &[Diagnostic]) -> Vec<String> {
    let player = section_first_line(src, section).unwrap_or(1);
    let library = section_first_line(src, "library").unwrap_or(1);
    diags
        .iter()
        .map(|d| {
            let (base, what) = match d.file {
                FILE_PLAYER => (player, section),
                FILE_LIBRARY => (library, "library"),
                _ => (1, "level"),
            };
            format!(
                "{file}:{}:{}: [{what}] {}",
                base + d.line.saturating_sub(1),
                d.col,
                messages::describe(d.key, &d.args)
            )
        })
        .collect()
}

/// Comment part of an assembly line (after the first `;` outside quotes), if any.
fn comment_of(line: &str) -> Option<&str> {
    let mut in_str = false;
    let mut in_chr = false;
    let mut prev = '\0';
    for (i, c) in line.char_indices() {
        match c {
            '"' if !in_chr && prev != '\\' => in_str = !in_str,
            '\'' if !in_str && prev != '\\' => in_chr = !in_chr,
            ';' if !in_str && !in_chr => return line.get(i + 1..),
            _ => {}
        }
        prev = c;
    }
    None
}

/// The label prefix(es) of an assembly line (`schleife:` in `schleife: add r1, r0`), with the
/// original indentation; empty if there is none.
fn label_prefix(line: &str) -> &str {
    let mut end = 0;
    loop {
        let rest = line.get(end..).unwrap_or("");
        let lead = rest.len() - rest.trim_start().len();
        let t = rest.trim_start();
        let ident = t
            .bytes()
            .take_while(|b| b.is_ascii_alphanumeric() || *b == b'_' || *b == b'.')
            .count();
        if ident > 0 && t.as_bytes().get(ident) == Some(&b':') {
            end += lead + ident + 1;
        } else {
            return line.get(..end).unwrap_or("");
        }
    }
}

/// Result of checking one level.
struct LevelCheck {
    problems: Vec<String>,
    level: Option<Level>,
    summary: String,
}

/// Checks one level file (`strict`, `mutate` as on the command line).
fn check_level(file: &str, src: &str, strict: bool, mutate: bool) -> LevelCheck {
    let mut problems = Vec::new();
    let level = match parse(src) {
        Ok(l) => l,
        Err(diags) => {
            let problems = diags
                .iter()
                .map(|d| messages::diagnostic(file, d))
                .collect();
            return LevelCheck {
                problems,
                level: None,
                summary: String::new(),
            };
        }
    };
    macro_rules! p {
        ($msg:expr) => {
            problems.push(format!("{file}: {}", $msg))
        };
    }

    if strict {
        for (key, value) in [
            ("title.de", &level.title_de),
            ("title.en", &level.title_en),
            ("card.de", &level.card_de),
            ("card.en", &level.card_en),
        ] {
            if value.trim().is_empty() {
                p!(format!("[level] needs '{key}'"));
            }
        }
        for lang in ["de", "en"] {
            for sec in ["text", "learned"] {
                let name = format!("{sec}.{lang}");
                if level.section(&name).is_none_or(|s| s.trim().is_empty()) {
                    p!(format!("section [{name}] is missing or empty"));
                }
            }
            if level.section(&format!("hint.{lang}")).is_none() {
                p!(format!(
                    "section [hint.{lang}] is missing (exactly {HINT_COUNT} hints separated by '---')"
                ));
            }
        }
        if level.ref_cycles.is_some() || level.ref_size.is_some() {
            p!("ref_cycles/ref_size belong only in packed files (written by pack-levels)");
        }
        let mut names: Vec<&str> = level.tests.iter().map(|t| t.name.as_str()).collect();
        names.sort_unstable();
        for pair in names.windows(2) {
            if pair[0] == pair[1] {
                p!(format!("test name '{}' is used more than once", pair[0]));
            }
        }
    }

    // Reference solution.
    let reference = match run_reference(&level) {
        Ok(Some(r)) => Some(r),
        Ok(None) => {
            if strict {
                p!("section [reference] is missing");
            }
            None
        }
        Err(diags) => {
            problems.extend(code_diags(file, src, "reference", &diags));
            None
        }
    };
    if let Some(r) = &reference {
        for (res, t) in r.results.iter().zip(&level.tests) {
            if !res.passed {
                p!(format!(
                    "the reference fails test '{}': {}",
                    t.name,
                    messages::test_failure(res)
                ));
            } else if res.cycles.saturating_mul(2) > level.test_budget(t) {
                p!(format!(
                    "the reference needs {} cycles in test '{}', more than 50 % of the budget {}",
                    res.cycles,
                    t.name,
                    level.test_budget(t)
                ));
            }
        }
    }

    if strict {
        // Addresses and labels used by tests must exist and be allowed.
        let fallback;
        let program: Option<&Program> = match &reference {
            Some(r) => Some(&r.program),
            None => {
                fallback = assemble_level(&level, "").ok();
                fallback.as_ref()
            }
        };
        if let Some(prog) = program {
            for msg in address_problems(&level, prog) {
                p!(msg);
            }
        }
        // Empty program and start code must fail (except tutorials).
        let start = level.section("start").unwrap_or("");
        match assemble_level(&level, start) {
            Err(diags) => problems.extend(code_diags(file, src, "start", &diags)),
            Ok(prog) => {
                if level.kind != LevelKind::Tutorial
                    && passes_all(&level, &prog, reference.as_ref())
                {
                    problems.push(format!("{file}: the start code passes every test"));
                }
            }
        }
        if level.kind != LevelKind::Tutorial
            && let Ok(prog) = assemble_level(&level, "")
            && passes_all(&level, &prog, reference.as_ref())
        {
            problems.push(format!("{file}: the empty program passes every test"));
        }
    }

    let mut mutation_note = String::new();
    if mutate && let (Some(r), Some(ref_src)) = (&reference, level.section("reference")) {
        let (msgs, note) = mutation_check(file, src, &level, ref_src, r);
        problems.extend(msgs);
        mutation_note = note;
    }

    let summary = match &reference {
        Some(r) => format!(
            "{} tests ({} hidden), reference {} words, {} cycles avg{}",
            level.tests.len(),
            level.tests.iter().filter(|t| t.hidden).count(),
            r.size,
            r.cycles,
            mutation_note
        ),
        None => format!("{} tests", level.tests.len()),
    };
    LevelCheck {
        problems,
        level: Some(level),
        summary,
    }
}

fn passes_all(level: &Level, program: &Program, reference: Option<&Reference>) -> bool {
    run_tests_with(level, program, reference).passed
}

/// `set`/`setb` must lie in writable segments other than the heap (which only `ALLOC` hands out);
/// `expect` addresses must exist; `call` labels must be code labels.
fn address_problems(level: &Level, program: &Program) -> Vec<String> {
    let mut out = Vec::new();
    let writable: Vec<_> = SEGMENTS
        .iter()
        .filter(|s| s.write && s.segment != Segment::Heap)
        .map(|s| s.name)
        .collect();
    for t in &level.tests {
        for (spec, what) in t
            .sets
            .iter()
            .map(|s| (s, "set"))
            .chain(t.expects.iter().map(|s| (s, "expect")))
        {
            let Some(addr) = resolve_addr(program, &spec.addr) else {
                out.push(format!(
                    "test '{}': {what} address {:?} is not a symbol of the reference program",
                    t.name, spec.addr
                ));
                continue;
            };
            let len = spec.values.byte_len().max(1);
            let last = addr as u64 + len as u64 - 1;
            if matches!(spec.values, MemValues::Words(_)) && addr % 4 != 0 {
                out.push(format!(
                    "test '{}': {what} 0x{addr:04X}: words need an address that is a multiple of 4",
                    t.name
                ));
            }
            let (Some(a), Some(b)) = (
                segment_of(addr),
                u32::try_from(last).ok().and_then(segment_of),
            ) else {
                out.push(format!(
                    "test '{}': {what} 0x{addr:04X}..0x{last:04X} leaves the memory",
                    t.name
                ));
                continue;
            };
            if what == "set" && (a.segment != b.segment || !writable.contains(&a.name)) {
                out.push(format!(
                    "test '{}': set 0x{addr:04X}..0x{last:04X} must lie inside one of the segments {}",
                    t.name,
                    writable.join(", ")
                ));
            }
        }
        if let Some(label) = &t.call
            && !program
                .symbol(label)
                .is_some_and(|s| s.kind == SymbolKind::Code)
        {
            out.push(format!(
                "test '{}': call = {label} is not a code label of the reference program",
                t.name
            ));
        }
    }
    out
}

/// Deletes each reference instruction in turn; every mutant must fail at least one test.
/// Returns problems and a summary note.
fn mutation_check(
    file: &str,
    src: &str,
    level: &Level,
    ref_src: &str,
    reference: &Reference,
) -> (Vec<String>, String) {
    let mut problems = Vec::new();
    let base = section_first_line(src, "reference").unwrap_or(1);
    let lines: Vec<&str> = ref_src.split('\n').collect();
    let mut instr_lines: Vec<u32> = reference.program.player_lines().map(|e| e.line).collect();
    instr_lines.dedup();
    let (mut killed, mut skipped, mut total) = (0usize, 0usize, 0usize);
    for line_no in instr_lines {
        let Some(text) = lines.get(line_no as usize - 1) else {
            continue;
        };
        let all_nop = reference
            .program
            .player_lines()
            .filter(|e| e.line == line_no)
            .all(|e| {
                let i = ((e.addr - stapelwerk_core::consts::TEXT_START) / 4) as usize;
                reference
                    .program
                    .words
                    .get(i)
                    .is_some_and(|w| decode(*w) == Ok(Instr::Nop))
            });
        if all_nop {
            continue;
        }
        if comment_of(text).is_some_and(|c| c.contains(MUTATE_SKIP)) {
            skipped += 1;
            continue;
        }
        total += 1;
        let mutant: String = lines
            .iter()
            .enumerate()
            .map(|(i, l)| {
                if i + 1 == line_no as usize {
                    label_prefix(l)
                } else {
                    l
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        if mutant_is_killed(level, &mutant, reference) {
            killed += 1;
        } else {
            problems.push(format!(
                "{file}:{}: tests too weak: the reference without `{}` still passes every test",
                base + line_no - 1,
                text.trim()
            ));
        }
    }
    if skipped > MAX_MUTATE_SKIPS {
        problems.push(format!(
            "{file}: {skipped} lines carry '; {MUTATE_SKIP}' (at most {MAX_MUTATE_SKIPS} allowed)"
        ));
    }
    let note = format!(", {killed}/{total} deletions caught, {skipped} skipped");
    (problems, note)
}

fn mutant_is_killed(level: &Level, src: &str, reference: &Reference) -> bool {
    let Ok(program) = assemble_level(level, src) else {
        return true;
    };
    (0..level.tests.len() as u32).any(|i| {
        let sref = match reference.screens.get(i as usize) {
            Some(s) if !s.is_empty() => ScreenRef::Screen(s),
            _ => ScreenRef::None,
        };
        !run_test_machine(level, &program, i, sref).0.passed
    })
}

/// Level files of a directory: from `manifest.txt` if present, else every `*.lvl` (sorted).
fn level_files(
    dir: &Path,
    problems: &mut Vec<String>,
    strict: bool,
) -> (Vec<PathBuf>, Option<Manifest>) {
    let mut on_disk: Vec<String> = match std::fs::read_dir(dir) {
        Ok(rd) => rd
            .filter_map(|e| e.ok())
            .filter_map(|e| e.file_name().into_string().ok())
            .filter(|n| n.ends_with(".lvl"))
            .collect(),
        Err(e) => {
            problems.push(format!("{}: cannot read the folder: {e}", dir.display()));
            return (Vec::new(), None);
        }
    };
    on_disk.sort();
    let manifest_path = dir.join("manifest.txt");
    let Ok(text) = std::fs::read_to_string(&manifest_path) else {
        if strict {
            problems.push(format!("{}: missing", manifest_path.display()));
        }
        return (on_disk.iter().map(|n| dir.join(n)).collect(), None);
    };
    let manifest = match parse_manifest(&text) {
        Ok(m) => m,
        Err(errs) => {
            for e in errs {
                problems.push(format!("{}: {e}", manifest_path.display()));
            }
            return (Vec::new(), None);
        }
    };
    for (f, line) in &manifest.files {
        if !on_disk.contains(f) {
            problems.push(format!(
                "{}:{line}: '{f}' does not exist",
                manifest_path.display()
            ));
        }
    }
    if strict {
        for f in &on_disk {
            if !manifest.files.iter().any(|(m, _)| m == f) {
                problems.push(format!(
                    "{}: '{f}' is not listed in the manifest",
                    manifest_path.display()
                ));
            }
        }
    }
    let files = manifest
        .files
        .iter()
        .filter(|(f, _)| on_disk.contains(f))
        .map(|(f, _)| dir.join(f))
        .collect();
    (files, Some(manifest))
}

/// Chapter of each manifest file: the last chapter line above it.
fn manifest_chapters(text: &str) -> Vec<(String, Option<u32>)> {
    let mut chapter = None;
    let mut out = Vec::new();
    for line in text.lines().map(str::trim) {
        if let Some(rest) = line.strip_prefix('#') {
            if let Some(n) = rest
                .trim_start()
                .strip_prefix("chapter")
                .and_then(|r| r.split_once(':'))
                .and_then(|(n, _)| n.trim().parse::<u32>().ok())
            {
                chapter = Some(n);
            }
        } else if !line.is_empty() {
            out.push((line.to_string(), chapter));
        }
    }
    out
}

pub fn cmd_check_levels(p: &Parsed, out: &mut dyn Write, err: &mut dyn Write) -> u8 {
    let strict = p.flag("--strict");
    let mutate = p.flag("--mutate");
    let target = Path::new(&p.positional[0]);
    let mut problems: Vec<String> = Vec::new();
    let (files, manifest) = if target.is_dir() {
        level_files(target, &mut problems, strict)
    } else if target.is_file() {
        (vec![target.to_path_buf()], None)
    } else {
        let _ = writeln!(err, "stapelwerk: '{}' does not exist", target.display());
        return 2;
    };
    let chapters = if manifest.is_some() {
        std::fs::read_to_string(target.join("manifest.txt"))
            .map(|t| manifest_chapters(&t))
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    let mut seen: Vec<(String, u32, u32, String)> = Vec::new();
    let mut failed = 0usize;
    for path in &files {
        let name = path.display().to_string();
        let src = match std::fs::read(path) {
            Ok(b) => String::from_utf8_lossy(&b).into_owned(),
            Err(e) => {
                problems.push(format!("{name}: cannot read: {e}"));
                continue;
            }
        };
        let mut c = check_level(&name, &src, strict, mutate);
        if let Some(l) = &c.level {
            let file_name = path
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or_default()
                .to_string();
            if strict {
                if file_name != format!("{}.lvl", l.id) {
                    c.problems.push(format!(
                        "{name}: the file must be named after its id: {}.lvl",
                        l.id
                    ));
                }
                if let Some((_, ch)) = chapters.iter().find(|(f, _)| *f == file_name)
                    && *ch != Some(l.chapter)
                {
                    c.problems.push(format!(
                        "{name}: chapter = {} but the manifest lists it under chapter {}",
                        l.chapter,
                        ch.map_or("none".into(), |n| n.to_string())
                    ));
                }
                if let Some(m) = &manifest
                    && !m.chapters.iter().any(|ch| ch.number == l.chapter)
                {
                    c.problems.push(format!(
                        "{name}: the manifest has no chapter line for chapter {}",
                        l.chapter
                    ));
                }
            }
            for (id, ch, ord, other) in &seen {
                if *id == l.id {
                    c.problems
                        .push(format!("{name}: id '{id}' is also used by {other}"));
                }
                if strict && *ch == l.chapter && *ord == l.order {
                    c.problems.push(format!(
                        "{name}: chapter {ch} order {ord} is also used by {other}"
                    ));
                }
            }
            seen.push((l.id.clone(), l.chapter, l.order, name.clone()));
        }
        if c.problems.is_empty() {
            let _ = writeln!(out, "ok    {name}: {}", c.summary);
        } else {
            failed += 1;
            let _ = writeln!(out, "FAIL  {name}");
            for msg in &c.problems {
                let _ = writeln!(out, "      {msg}");
            }
        }
    }
    for msg in &problems {
        let _ = writeln!(out, "FAIL  {msg}");
    }
    let _ = writeln!(
        out,
        "{} level(s) checked{}{}: {}",
        files.len(),
        if strict { ", strict" } else { "" },
        if mutate { ", mutation test" } else { "" },
        if failed == 0 && problems.is_empty() {
            "all good".to_string()
        } else {
            format!("{} problem(s)", failed + problems.len())
        }
    );
    if files.is_empty() && problems.is_empty() {
        let _ = writeln!(
            err,
            "stapelwerk: no level files found in '{}'",
            target.display()
        );
        return 1;
    }
    if failed == 0 && problems.is_empty() {
        0
    } else {
        1
    }
}

pub fn cmd_pack_levels(p: &Parsed, out: &mut dyn Write, err: &mut dyn Write) -> u8 {
    let dir = Path::new(&p.positional[0]);
    let target = Path::new(&p.positional[1]);
    let manifest_path = dir.join("manifest.txt");
    let manifest = match std::fs::read_to_string(&manifest_path) {
        Ok(t) => match parse_manifest(&t) {
            Ok(m) => m,
            Err(errs) => {
                for e in errs {
                    let _ = writeln!(err, "{}: {e}", manifest_path.display());
                }
                return 1;
            }
        },
        Err(e) => {
            let _ = writeln!(
                err,
                "stapelwerk: cannot read '{}': {e}",
                manifest_path.display()
            );
            return 1;
        }
    };
    let mut packed = Vec::new();
    let mut failed = false;
    for (f, _) in &manifest.files {
        let path = dir.join(f);
        let name = path.display().to_string();
        let src = match std::fs::read(&path) {
            Ok(b) => String::from_utf8_lossy(&b).into_owned(),
            Err(e) => {
                let _ = writeln!(err, "{name}: cannot read: {e}");
                failed = true;
                continue;
            }
        };
        match pack_level(&src) {
            Ok(pl) => {
                let reference_ok = run_reference(&parse(&src).unwrap_or_default())
                    .ok()
                    .flatten()
                    .is_some_and(|r| r.passes());
                if !reference_ok {
                    let _ = writeln!(
                        err,
                        "{name}: the reference does not pass every test (run check-levels)"
                    );
                    failed = true;
                    continue;
                }
                packed.push(pl);
            }
            Err(e) => {
                failed = true;
                match e {
                    PackError::Parse(diags) | PackError::Repack(diags) => {
                        for d in &diags {
                            let _ = writeln!(err, "{}", messages::diagnostic(&name, d));
                        }
                    }
                    PackError::NoReference => {
                        let _ = writeln!(err, "{name}: section [reference] is missing");
                    }
                    PackError::Reference(diags) => {
                        for l in code_diags(&name, &src, "reference", &diags) {
                            let _ = writeln!(err, "{l}");
                        }
                    }
                }
            }
        }
    }
    if failed {
        let _ = writeln!(err, "stapelwerk: nothing written");
        return 1;
    }
    if let Err(e) = std::fs::create_dir_all(target) {
        let _ = writeln!(err, "stapelwerk: cannot create '{}': {e}", target.display());
        return 1;
    }
    for pl in &packed {
        let path = target.join(format!("{}.lvl", pl.level.id));
        if let Err(e) = std::fs::write(&path, &pl.text) {
            let _ = writeln!(err, "stapelwerk: cannot write '{}': {e}", path.display());
            return 1;
        }
    }
    let levels: Vec<Level> = packed.into_iter().map(|p| p.level).collect();
    let index = target.join("index.json");
    if let Err(e) = std::fs::write(&index, index_json(&manifest.chapters, &levels)) {
        let _ = writeln!(err, "stapelwerk: cannot write '{}': {e}", index.display());
        return 1;
    }
    let _ = writeln!(
        out,
        "packed {} level(s) into {}",
        levels.len(),
        target.display()
    );
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_and_comments() {
        assert_eq!(label_prefix("schleife: add r1, r0"), "schleife:");
        assert_eq!(label_prefix("  a: .b:  halt"), "  a: .b:");
        assert_eq!(label_prefix("        add r1, r0"), "");
        assert_eq!(label_prefix("  mov r0, ':'"), "");
        assert_eq!(
            comment_of("  mov r1, 0 ; x ; mutate:skip"),
            Some(" x ; mutate:skip")
        );
        assert_eq!(comment_of("  mov r0, ';'"), None);
        assert_eq!(comment_of("  .string \"a;b\""), None);
    }

    #[test]
    fn first_line_of_section() {
        let src = "[level]\nid = x\n\n[reference]\n\n  halt\n";
        assert_eq!(section_first_line(src, "reference"), Some(6));
        assert_eq!(section_first_line(src, "start"), None);
    }
}
