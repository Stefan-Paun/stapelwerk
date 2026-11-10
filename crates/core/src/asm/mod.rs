//! Assembler: player source + optional level library → [`Program`].
//!
//! The **contract** with the rest of the crate is only [`assemble`], [`MnemonicFilter`] and the
//! types in [`crate::program`] and [`crate::diag`]. Internally: [`lexer`] (tokens per line),
//! [`expr`] (constant expressions), [`parser`] (labels, instructions, directives) and [`layout`]
//! (pre-pass, two passes, encoding). Language reference: `docs/ISA.md` ("Assembly language").

pub mod expr;
pub mod layout;
pub mod lexer;
pub mod parser;

use crate::diag::{Diagnostic, FILE_LIBRARY, FILE_PLAYER};
use crate::isa::Cond;
use crate::program::Program;

/// Level options `allow` / `forbid`: which mnemonics the **player** may use (the library is never
/// filtered). Names are compared in canonical lower-case form; condition aliases count as their
/// canonical jump (`je` ≡ `jz`), and the pseudo-instruction `li` is filtered by its own name.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum MnemonicFilter {
    /// No restriction.
    #[default]
    All,
    /// Only these mnemonics.
    Allow(Vec<String>),
    /// All except these mnemonics.
    Forbid(Vec<String>),
}

/// Canonical lower-case spelling used by [`MnemonicFilter`].
pub fn canonical_mnemonic(name: &str) -> String {
    match Cond::from_mnemonic(name) {
        Some(c) => c.mnemonic().to_string(),
        None => name.to_ascii_lowercase(),
    }
}

impl MnemonicFilter {
    /// `true` if the player may use `mnemonic` (any case, aliases allowed).
    pub fn permits(&self, mnemonic: &str) -> bool {
        let m = canonical_mnemonic(mnemonic);
        match self {
            MnemonicFilter::All => true,
            MnemonicFilter::Allow(list) => list.iter().any(|x| canonical_mnemonic(x) == m),
            MnemonicFilter::Forbid(list) => !list.iter().any(|x| canonical_mnemonic(x) == m),
        }
    }
}

/// Assembles the player's source (file 0) together with an optional library (file 1).
///
/// Binding rules (spec): a pre-pass collects the library's `.equ` constants (they may only use
/// built-in and earlier library constants) and makes them visible to the player; text = player
/// code from 0x0100, then library code, then one `halt` word labelled `__trap`; data = player
/// `.data` from 0x4000, then library `.data`; labels of both files share one namespace.
///
/// *Clarification:* one zero word separates the player's code from the library (or from the trap
/// word if there is no library), so a program that runs past its end without `halt` stops with
/// `fault.zero_instruction`. That word has no line-map entry and does not count towards `size`.
///
/// Returns all diagnostics (sorted by file, line, column; at most 50 plus `asm.too_many_errors`)
/// if there is at least one.
pub fn assemble(
    player: &str,
    library: Option<&str>,
    filter: &MnemonicFilter,
) -> Result<Program, Vec<Diagnostic>> {
    let (mut p_lines, mut diags) = parser::parse_file(player, FILE_PLAYER, filter);
    let mut l_lines = match library {
        Some(src) => {
            let (lines, d) = parser::parse_file(src, FILE_LIBRARY, &MnemonicFilter::All);
            diags.extend(d);
            lines
        }
        None => Vec::new(),
    };
    layout::assemble_lines(&mut p_lines, &mut l_lines, diags)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_uses_canonical_names() {
        let allow = MnemonicFilter::Allow(vec!["mov".into(), "JZ".into(), "sys".into()]);
        assert!(allow.permits("MOV"));
        assert!(allow.permits("je"));
        assert!(!allow.permits("add"));
        let forbid = MnemonicFilter::Forbid(vec!["mul".into(), "jne".into()]);
        assert!(!forbid.permits("Mul"));
        assert!(!forbid.permits("jnz"));
        assert!(forbid.permits("add"));
        assert!(MnemonicFilter::All.permits("anything"));
    }
}
