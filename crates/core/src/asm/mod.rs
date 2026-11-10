//! Assembler: player source + optional level library → [`Program`].
//!
//! PHASE 1B owns this directory. The **contract** with the rest of the crate is only
//! [`assemble`], [`MnemonicFilter`] and the types in [`crate::program`] and [`crate::diag`]; the
//! internal split into [`lexer`], [`expr`], [`parser`] and [`layout`] is a suggestion and may be
//! restructured freely. Language reference: `docs/ISA.md` ("Assembly language").

pub mod expr;
pub mod layout;
pub mod lexer;
pub mod parser;

use crate::diag::Diagnostic;
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
/// Returns all diagnostics (sorted by file, line, column; at most 50 plus `asm.too_many_errors`)
/// if there is at least one. PHASE 1B.
pub fn assemble(
    _player: &str,
    _library: Option<&str>,
    _filter: &MnemonicFilter,
) -> Result<Program, Vec<Diagnostic>> {
    Err(Vec::new())
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
