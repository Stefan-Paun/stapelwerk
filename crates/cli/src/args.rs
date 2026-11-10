//! Tiny argument parser (no third-party crates).
//!
//! Supports `--name value`, `--name=value`, `-o value`, boolean flags, `--` to end options, and a
//! fixed range of positional arguments per command.

/// One option of a command.
#[derive(Clone, Copy, Debug)]
pub struct Opt {
    /// Spelling including dashes, e.g. `--in` or `-o`.
    pub name: &'static str,
    /// `true` if the option takes a value.
    pub takes_value: bool,
    /// `true` if the option must be given.
    pub required: bool,
}

/// Static description of a subcommand.
#[derive(Clone, Copy, Debug)]
pub struct CommandSpec {
    pub name: &'static str,
    /// Usage line without the program name.
    pub usage: &'static str,
    /// One-line description.
    pub about: &'static str,
    pub options: &'static [Opt],
    pub min_positional: usize,
    pub max_positional: usize,
}

/// Parsed arguments of one command.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Parsed {
    /// Options in the order given: (name, value).
    pub options: Vec<(&'static str, Option<String>)>,
    pub positional: Vec<String>,
}

impl Parsed {
    /// `true` if a flag was given.
    pub fn flag(&self, name: &str) -> bool {
        self.options.iter().any(|(n, _)| *n == name)
    }

    /// Value of an option.
    pub fn value(&self, name: &str) -> Option<&str> {
        self.options
            .iter()
            .find(|(n, _)| *n == name)
            .and_then(|(_, v)| v.as_deref())
    }
}

/// Parses the arguments after the command name.
pub fn parse(spec: &CommandSpec, args: &[String]) -> Result<Parsed, String> {
    let mut parsed = Parsed::default();
    let mut only_positional = false;
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        i += 1;
        if only_positional || !arg.starts_with('-') || arg == "-" {
            parsed.positional.push(arg.clone());
            continue;
        }
        if arg == "--" {
            only_positional = true;
            continue;
        }
        let (name, inline) = match arg.split_once('=') {
            Some((n, v)) if n.starts_with("--") => (n, Some(v.to_string())),
            _ => (arg.as_str(), None),
        };
        let Some(opt) = spec.options.iter().find(|o| o.name == name) else {
            return Err(format!("unknown option '{name}' for '{}'", spec.name));
        };
        if parsed.flag(opt.name) {
            return Err(format!("option '{}' given more than once", opt.name));
        }
        let value = if opt.takes_value {
            match inline {
                Some(v) => Some(v),
                None => match args.get(i) {
                    Some(v) => {
                        i += 1;
                        Some(v.clone())
                    }
                    None => return Err(format!("option '{}' needs a value", opt.name)),
                },
            }
        } else {
            if inline.is_some() {
                return Err(format!("option '{}' takes no value", opt.name));
            }
            None
        };
        parsed.options.push((opt.name, value));
    }
    for opt in spec.options.iter().filter(|o| o.required) {
        if !parsed.flag(opt.name) {
            return Err(format!("missing required option '{}'", opt.name));
        }
    }
    let n = parsed.positional.len();
    if n < spec.min_positional || n > spec.max_positional {
        return Err(format!(
            "wrong number of arguments for '{}' (got {n})",
            spec.name
        ));
    }
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPEC: CommandSpec = CommandSpec {
        name: "run",
        usage: "",
        about: "",
        options: &[
            Opt {
                name: "--in",
                takes_value: true,
                required: false,
            },
            Opt {
                name: "--trace",
                takes_value: false,
                required: false,
            },
            Opt {
                name: "-o",
                takes_value: true,
                required: false,
            },
        ],
        min_positional: 1,
        max_positional: 1,
    };

    fn strings(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn options_and_positionals() {
        let p = parse(&SPEC, &strings(&["a.s", "--in", "3 4", "--trace"])).unwrap();
        assert_eq!(p.positional, vec!["a.s"]);
        assert_eq!(p.value("--in"), Some("3 4"));
        assert!(p.flag("--trace"));
        let p = parse(&SPEC, &strings(&["--in=-5", "-o", "x", "--", "-file.s"])).unwrap();
        assert_eq!(p.value("--in"), Some("-5"));
        assert_eq!(p.value("-o"), Some("x"));
        assert_eq!(p.positional, vec!["-file.s"]);
    }

    #[test]
    fn errors() {
        assert!(parse(&SPEC, &strings(&["a.s", "--bogus"])).is_err());
        assert!(parse(&SPEC, &strings(&["a.s", "--in"])).is_err());
        assert!(parse(&SPEC, &strings(&["a.s", "--trace=1"])).is_err());
        assert!(parse(&SPEC, &strings(&["a.s", "--trace", "--trace"])).is_err());
        assert!(parse(&SPEC, &strings(&[])).is_err());
        assert!(parse(&SPEC, &strings(&["a.s", "b.s"])).is_err());
    }
}
