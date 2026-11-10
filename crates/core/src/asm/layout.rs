//! Two-pass layout: addresses, symbol table, encoding, line map. PHASE 1B (internal; may be
//! restructured).

use super::MnemonicFilter;
use super::parser::Line;
use crate::diag::Diagnostic;
use crate::program::Program;

/// Lays out player and library lines and encodes them. PHASE 1B.
pub fn layout(
    _player: &[Line],
    _library: &[Line],
    _filter: &MnemonicFilter,
) -> Result<Program, Vec<Diagnostic>> {
    Err(Vec::new())
}
