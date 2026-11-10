//! Packing levels for the web build (`stapelwerk pack-levels`), `levels/manifest.txt` and
//! `index.json`.
//!
//! Packing is a text transformation of the source file, so comments and layout survive:
//! `[reference]` is removed, `ref_cycles`/`ref_size` are written at the end of `[level]`,
//! every `expect_screen = reference` becomes `expect_screen = __ref_TESTNAME`, and the reference
//! screens are appended as `[image.__ref_TESTNAME]` sections. The reference values themselves are
//! computed by [`crate::runner::pack_level`].

use super::{Header, IMAGE_SECTION_PREFIX, Level, header_of, image_rows, split_kv};
use crate::json::JsonWriter;

/// Image name for the reference screen of test `test_name`.
pub fn ref_image_name(test_name: &str) -> String {
    format!("__ref_{test_name}")
}

/// Rewrites a level source into its packed form (see the module documentation).
/// `ref_screens` maps test names to reference screens (4096 bytes each); only tests with
/// `expect_screen = reference` are rewritten.
pub fn pack_source(
    src: &str,
    ref_cycles: u64,
    ref_size: u32,
    ref_screens: &[(String, Vec<u8>)],
) -> String {
    let src = src.strip_prefix('\u{FEFF}').unwrap_or(src);
    let mut out: Vec<String> = Vec::new();
    #[derive(PartialEq)]
    enum Sec {
        Other,
        Level,
        Test,
        Reference,
    }
    let mut sec = Sec::Other;
    // Index in `out` after the last non-blank line of `[level]`.
    let mut level_end: Option<usize> = None;
    // Lines of the current test (rewritten once the test's name is known).
    let mut test_lines: Vec<String> = Vec::new();
    let flush_test = |out: &mut Vec<String>, lines: &mut Vec<String>| {
        let name = lines.iter().find_map(|l| match split_kv(l) {
            Ok(Some(kv)) if kv.key == "name" => Some(kv.value.to_string()),
            _ => None,
        });
        for l in lines.drain(..) {
            let rewrite = match split_kv(&l) {
                Ok(Some(kv)) => kv.key == "expect_screen" && kv.value == "reference",
                _ => false,
            };
            match (&name, rewrite) {
                (Some(n), true) => out.push(format!("expect_screen = {}", ref_image_name(n))),
                _ => out.push(l),
            }
        }
    };
    let mut lines: Vec<&str> = src.split('\n').collect();
    if lines.last() == Some(&"") {
        lines.pop();
    }
    for raw in lines {
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        if let Some(Header::Known(name)) = header_of(line) {
            if sec == Sec::Test {
                flush_test(&mut out, &mut test_lines);
            }
            sec = match name {
                "level" => Sec::Level,
                "test" => Sec::Test,
                "reference" => Sec::Reference,
                _ => Sec::Other,
            };
            if sec == Sec::Reference {
                // Drop the blank lines that separated the previous section from it.
                while out.last().is_some_and(|l| l.trim().is_empty()) {
                    out.pop();
                }
                out.push(String::new());
                continue;
            }
            out.push(line.to_string());
            if sec == Sec::Level {
                level_end = Some(out.len());
            }
            continue;
        }
        match sec {
            Sec::Reference => {}
            Sec::Test => test_lines.push(line.to_string()),
            Sec::Level => {
                let is_ref_key = matches!(split_kv(line), Ok(Some(kv))
                    if kv.key == "ref_cycles" || kv.key == "ref_size");
                if is_ref_key {
                    continue;
                }
                out.push(line.to_string());
                if !line.trim().is_empty() {
                    level_end = Some(out.len());
                }
            }
            Sec::Other => out.push(line.to_string()),
        }
    }
    if sec == Sec::Test {
        flush_test(&mut out, &mut test_lines);
    }
    let at = level_end.unwrap_or(0).min(out.len());
    out.insert(at, format!("ref_size = {ref_size}"));
    out.insert(at, format!("ref_cycles = {ref_cycles}"));
    while out.last().is_some_and(|l| l.trim().is_empty()) {
        out.pop();
    }
    for (test, pixels) in ref_screens {
        out.push(String::new());
        out.push(format!("[{IMAGE_SECTION_PREFIX}{}]", ref_image_name(test)));
        out.extend(image_rows(pixels));
    }
    let mut s = out.join("\n");
    s.push('\n');
    s
}

/// A chapter line of the manifest: `# chapter N: German title | English title`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chapter {
    pub number: u32,
    pub title_de: String,
    pub title_en: String,
}

/// `levels/manifest.txt`: chapters and level file names in playing order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Manifest {
    pub chapters: Vec<Chapter>,
    /// (file name, 1-based line).
    pub files: Vec<(String, u32)>,
}

/// Parses a manifest. Blank lines and other `#` lines are ignored. Errors are English messages
/// with line numbers (the manifest is a build-tool file, not shown in the game).
pub fn parse_manifest(src: &str) -> Result<Manifest, Vec<String>> {
    let mut m = Manifest::default();
    let mut errors = Vec::new();
    for (i, raw) in src.lines().enumerate() {
        let no = i as u32 + 1;
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix('#') {
            let rest = rest.trim_start();
            let Some(rest) = rest.strip_prefix("chapter") else {
                continue;
            };
            let parsed = rest.trim_start().split_once(':').and_then(|(n, titles)| {
                let number = n.trim().parse::<u32>().ok().filter(|n| *n <= 7)?;
                let (de, en) = titles.split_once('|')?;
                let (de, en) = (de.trim(), en.trim());
                (!de.is_empty() && !en.is_empty()).then(|| Chapter {
                    number,
                    title_de: de.to_string(),
                    title_en: en.to_string(),
                })
            });
            match parsed {
                Some(c) if m.chapters.iter().any(|o| o.number == c.number) => {
                    errors.push(format!("line {no}: chapter {} listed twice", c.number));
                }
                Some(c) => m.chapters.push(c),
                None => errors.push(format!(
                    "line {no}: expected '# chapter N: German title | English title' with N in 0..7"
                )),
            }
            continue;
        }
        let valid = line.ends_with(".lvl")
            && !line.contains('/')
            && !line.contains('\\')
            && !line.starts_with('.')
            && !line.contains(char::is_whitespace);
        if !valid {
            errors.push(format!(
                "line {no}: '{line}' is not a level file name (expected NAME.lvl in the same folder)"
            ));
        } else if m.files.iter().any(|(f, _)| f == line) {
            errors.push(format!("line {no}: '{line}' listed twice"));
        } else {
            m.files.push((line.to_string(), no));
        }
    }
    if errors.is_empty() {
        Ok(m)
    } else {
        Err(errors)
    }
}

/// `index.json` for the web build:
/// `{"version":1,"chapters":[{"chapter","title":{de,en}}],"levels":[{"id","chapter","order","kind","title":{de,en},"card":{de,en},"mobile"}]}`.
/// Levels keep the given order (manifest order).
pub fn index_json(chapters: &[Chapter], levels: &[Level]) -> String {
    let mut w = JsonWriter::new();
    w.begin_object();
    w.field_uint("version", 1);
    w.key("chapters");
    w.begin_array();
    for c in chapters {
        w.begin_object();
        w.field_uint("chapter", c.number as u64);
        super::write_lang_pair(&mut w, "title", &c.title_de, &c.title_en);
        w.end_object();
    }
    w.end_array();
    w.key("levels");
    w.begin_array();
    for l in levels {
        w.begin_object();
        w.field_str("id", &l.id);
        w.field_uint("chapter", l.chapter as u64);
        w.field_uint("order", l.order as u64);
        w.field_str("kind", l.kind.as_str());
        super::write_lang_pair(&mut w, "title", &l.title_de, &l.title_en);
        super::write_lang_pair(&mut w, "card", &l.card_de, &l.card_en);
        w.field_bool("mobile", l.mobile);
        w.end_object();
    }
    w.end_array();
    w.end_object();
    let mut s = w.finish();
    s.push('\n');
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest() {
        let m = parse_manifest(
            "# chapter 0: Einschalten | Power On\n\n# a comment\n0-1-a.lvl\n# chapter 1: A | B\n",
        )
        .unwrap();
        assert_eq!(m.chapters.len(), 2);
        assert_eq!(m.chapters[0].title_en, "Power On");
        assert_eq!(m.files, vec![("0-1-a.lvl".to_string(), 4)]);
        assert!(parse_manifest("# chapter 9: A | B\n").is_err());
        assert!(parse_manifest("# chapter 1: A\n").is_err());
        assert!(parse_manifest("../x.lvl\n").is_err());
        assert!(parse_manifest("a.lvl\na.lvl\n").is_err());
    }

    #[test]
    fn pack_rewrites_sections() {
        let src = "[level]\nid = x\nref_size = 99\n\n[reference]\n  halt\n\n[test]\nexpect_screen = reference\nname = t1\n";
        let packed = pack_source(src, 7, 1, &[("t1".into(), vec![1u8; 4096])]);
        assert!(!packed.contains("[reference]"));
        assert!(!packed.contains("99"));
        assert!(packed.contains("[level]\nid = x\nref_cycles = 7\nref_size = 1\n"));
        assert!(packed.contains("expect_screen = __ref_t1\nname = t1\n"));
        assert!(packed.contains("[image.__ref_t1]\n1111"));
        assert!(packed.ends_with('\n'));
    }
}
