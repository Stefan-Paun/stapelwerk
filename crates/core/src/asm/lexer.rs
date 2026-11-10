//! Line tokenizer. PHASE 1B (internal; may be restructured).
//!
//! Columns in diagnostics are 1-based and counted in UTF-16 code units; [`utf16_col`] converts a
//! byte offset of a line into such a column.

use crate::diag::Diagnostic;

/// Kind of token.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TokenKind {
    /// Identifier, mnemonic, register, directive (with leading `.`) or local label (`.name`).
    Ident(String),
    /// Number or character literal, already converted.
    Number(i64),
    /// String literal, already unescaped (UTF-8 bytes).
    Str(Vec<u8>),
    /// Punctuation or operator: `, : [ ] ( ) + - * / % << >> & ^ | ~`.
    Punct(&'static str),
}

/// A token with its position in the line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    /// 1-based column in UTF-16 code units.
    pub col: u32,
    /// Length in UTF-16 code units.
    pub len: u32,
}

/// 1-based UTF-16 column of byte offset `byte` in `line` (offsets inside a character round down).
pub fn utf16_col(line: &str, byte: usize) -> u32 {
    let mut col = 1u32;
    for (i, ch) in line.char_indices() {
        if i >= byte {
            break;
        }
        col += ch.len_utf16() as u32;
    }
    col
}

/// Splits one source line (without the comment) into tokens. PHASE 1B.
pub fn tokenize_line(_line: &str, _file: u8, _line_no: u32) -> Result<Vec<Token>, Diagnostic> {
    Ok(Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf16_columns() {
        assert_eq!(utf16_col("abc", 0), 1);
        assert_eq!(utf16_col("abc", 2), 3);
        // 'ä' is 2 bytes / 1 unit, '😀' is 4 bytes / 2 units.
        assert_eq!(utf16_col("ä😀x", 2), 2);
        assert_eq!(utf16_col("ä😀x", 6), 4);
    }
}
