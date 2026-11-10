//! Line tokenizer.
//!
//! Columns in diagnostics are 1-based and counted in UTF-16 code units; [`utf16_col`] converts a
//! byte offset of a line into such a column. The tokenizer tracks the column itself while it walks
//! the line, so every token carries its UTF-16 column and length.

use crate::diag::Diagnostic;

/// Kind of token.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TokenKind {
    /// Identifier, mnemonic, register, directive (with leading `.`), local label (`.name`) or
    /// qualified local label (`global.local`).
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
    /// Byte offset of the token in its line.
    pub start: u32,
}

impl Token {
    /// Column one past the end of the token.
    pub fn end(&self) -> u32 {
        self.col + self.len
    }

    /// `true` if the token is the punctuation `p`.
    pub fn is(&self, p: &str) -> bool {
        matches!(self.kind, TokenKind::Punct(q) if q == p)
    }

    /// The identifier text, if the token is an identifier.
    pub fn ident(&self) -> Option<&str> {
        match &self.kind {
            TokenKind::Ident(s) => Some(s),
            _ => None,
        }
    }

    /// First source character of the token in `line` (for `asm.unexpected_char {ch}`).
    pub fn first_char(&self, line: &str) -> String {
        line.get(self.start as usize..)
            .and_then(|s| s.chars().next())
            .map(String::from)
            .unwrap_or_default()
    }
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

const fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_'
}

const fn is_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Walks a line by bytes while keeping the UTF-16 column.
struct Cursor<'a> {
    src: &'a str,
    bytes: &'a [u8],
    pos: usize,
    col: u32,
}

impl<'a> Cursor<'a> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn peek_at(&self, k: usize) -> Option<u8> {
        self.bytes.get(self.pos + k).copied()
    }

    /// Advances by one character and returns it.
    fn bump(&mut self) -> Option<char> {
        let b = self.peek()?;
        if b < 0x80 {
            self.pos += 1;
            self.col += 1;
            return Some(b as char);
        }
        let ch = self.src.get(self.pos..)?.chars().next()?;
        self.pos += ch.len_utf8();
        self.col += ch.len_utf16() as u32;
        Some(ch)
    }

    /// Advances over ASCII identifier characters.
    fn eat_ident_chars(&mut self) {
        while let Some(b) = self.peek() {
            if !is_ident_char(b) {
                break;
            }
            self.pos += 1;
            self.col += 1;
        }
    }
}

/// Splits one source line into tokens; a `;` outside a string or character literal starts a
/// comment that runs to the end of the line. Returns the first lexical error of the line.
pub fn tokenize_line(line: &str, file: u8, line_no: u32) -> Result<Vec<Token>, Diagnostic> {
    let mut c = Cursor {
        src: line,
        bytes: line.as_bytes(),
        pos: 0,
        col: 1,
    };
    let mut out = Vec::new();
    let err = |key: &'static str, col: u32, end: u32| {
        Diagnostic::new(key, file, line_no, col, end.saturating_sub(col).max(1))
    };
    loop {
        while matches!(c.peek(), Some(b' ' | b'\t' | b'\r' | 0x0B | 0x0C)) {
            c.bump();
        }
        let Some(b) = c.peek() else { break };
        let start = c.pos;
        let col = c.col;
        let kind = match b {
            b';' => break,
            _ if is_ident_start(b) => {
                c.eat_ident_chars();
                // Qualified local label `global.local`.
                if c.peek() == Some(b'.') && c.peek_at(1).is_some_and(is_ident_start) {
                    c.bump();
                    c.eat_ident_chars();
                }
                TokenKind::Ident(line[start..c.pos].to_string())
            }
            b'.' => {
                if c.peek_at(1).is_some_and(is_ident_start) {
                    c.bump();
                    c.eat_ident_chars();
                    TokenKind::Ident(line[start..c.pos].to_string())
                } else {
                    c.bump();
                    return Err(err("asm.unexpected_char", col, c.col).arg("ch", "."));
                }
            }
            b'0'..=b'9' => {
                c.eat_ident_chars();
                match parse_number(&line[start..c.pos]) {
                    Some(v) => TokenKind::Number(v),
                    None => return Err(err("asm.bad_number", col, c.col)),
                }
            }
            b'\'' => {
                c.bump();
                match lex_char(&mut c) {
                    Some(v) => TokenKind::Number(v),
                    None => {
                        // Skip to the closing quote (if any) so the whole literal is marked.
                        while let Some(ch) = c.bump() {
                            if ch == '\'' {
                                break;
                            }
                        }
                        return Err(err("asm.bad_string", col, c.col));
                    }
                }
            }
            b'"' => {
                c.bump();
                match lex_string(&mut c) {
                    Some(bytes) => TokenKind::Str(bytes),
                    None => {
                        while let Some(ch) = c.bump() {
                            if ch == '"' {
                                break;
                            }
                        }
                        return Err(err("asm.bad_string", col, c.col));
                    }
                }
            }
            b'<' | b'>' => {
                c.bump();
                if c.peek() == Some(b) {
                    c.bump();
                    TokenKind::Punct(if b == b'<' { "<<" } else { ">>" })
                } else {
                    let ch = if b == b'<' { "<" } else { ">" };
                    return Err(err("asm.unexpected_char", col, c.col).arg("ch", ch));
                }
            }
            _ => {
                let p = match b {
                    b',' => Some(","),
                    b':' => Some(":"),
                    b'[' => Some("["),
                    b']' => Some("]"),
                    b'(' => Some("("),
                    b')' => Some(")"),
                    b'+' => Some("+"),
                    b'-' => Some("-"),
                    b'*' => Some("*"),
                    b'/' => Some("/"),
                    b'%' => Some("%"),
                    b'&' => Some("&"),
                    b'^' => Some("^"),
                    b'|' => Some("|"),
                    b'~' => Some("~"),
                    _ => None,
                };
                let ch = c.bump().unwrap_or('?');
                match p {
                    Some(p) => TokenKind::Punct(p),
                    None => {
                        return Err(
                            err("asm.unexpected_char", col, c.col).arg("ch", ch.to_string())
                        );
                    }
                }
            }
        };
        out.push(Token {
            kind,
            col,
            len: c.col - col,
            start: start as u32,
        });
    }
    Ok(out)
}

/// Parses `42`, `0x2A`, `0b1010`, `1_000` (underscores only between digits). `None` if malformed
/// or beyond `i64::MAX`.
pub fn parse_number(s: &str) -> Option<i64> {
    let (digits, radix) = if let Some(r) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        (r, 16)
    } else if let Some(r) = s.strip_prefix("0b").or_else(|| s.strip_prefix("0B")) {
        (r, 2)
    } else {
        (s, 10)
    };
    if digits.is_empty()
        || digits.starts_with('_')
        || digits.ends_with('_')
        || digits.contains("__")
    {
        return None;
    }
    let mut v: u64 = 0;
    for ch in digits.chars() {
        if ch == '_' {
            continue;
        }
        let d = ch.to_digit(radix)? as u64;
        v = v.checked_mul(radix as u64)?.checked_add(d)?;
    }
    i64::try_from(v).ok()
}

/// One escape sequence after the backslash; returns the byte value.
fn lex_escape(c: &mut Cursor<'_>) -> Option<u8> {
    let e = c.bump()?;
    Some(match e {
        'n' => b'\n',
        't' => b'\t',
        '0' => 0,
        '\\' => b'\\',
        '"' => b'"',
        '\'' => b'\'',
        'x' => {
            let h = (c.bump()?.to_digit(16)?, c.bump()?.to_digit(16)?);
            (h.0 * 16 + h.1) as u8
        }
        _ => return None,
    })
}

/// Character literal after the opening quote: exactly one character or escape, then `'`.
fn lex_char(c: &mut Cursor<'_>) -> Option<i64> {
    let ch = c.bump()?;
    let v = match ch {
        '\'' => return None,
        '\\' => lex_escape(c)? as i64,
        _ => ch as i64,
    };
    if c.bump()? != '\'' {
        return None;
    }
    Some(v)
}

/// String literal after the opening quote, up to and including the closing `"`.
fn lex_string(c: &mut Cursor<'_>) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    loop {
        match c.bump()? {
            '"' => return Some(out),
            '\\' => out.push(lex_escape(c)?),
            ch => {
                let mut buf = [0u8; 4];
                out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
            }
        }
    }
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

    fn kinds(s: &str) -> Vec<TokenKind> {
        tokenize_line(s, 0, 1)
            .unwrap()
            .into_iter()
            .map(|t| t.kind)
            .collect()
    }

    #[test]
    fn tokens() {
        use TokenKind::*;
        assert_eq!(
            kinds("loop: add r1, 0x2A ; hi"),
            vec![
                Ident("loop".into()),
                Punct(":"),
                Ident("add".into()),
                Ident("r1".into()),
                Punct(","),
                Number(42)
            ]
        );
        assert_eq!(
            kinds("'\\n' ';' \"a;b\\x41\" 1_000 0b101 a.b .c << >>"),
            vec![
                Number(10),
                Number(59),
                Str(b"a;bA".to_vec()),
                Number(1000),
                Number(5),
                Ident("a.b".into()),
                Ident(".c".into()),
                Punct("<<"),
                Punct(">>")
            ]
        );
    }

    #[test]
    fn errors_and_columns() {
        let e = tokenize_line("  0x", 0, 3).unwrap_err();
        assert_eq!((e.key, e.line, e.col, e.len), ("asm.bad_number", 3, 3, 2));
        for bad in ["1__0", "1_", "0b2", "12ab", "99999999999999999999"] {
            assert_eq!(tokenize_line(bad, 0, 1).unwrap_err().key, "asm.bad_number");
        }
        for bad in ["\"abc", "'ab'", "''", "'\\q'", "\"\\x4\""] {
            assert_eq!(
                tokenize_line(bad, 0, 1).unwrap_err().key,
                "asm.bad_string",
                "{bad}"
            );
        }
        let e = tokenize_line("ä😀 @", 0, 1).unwrap_err();
        assert_eq!(e.key, "asm.unexpected_char");
        // 'ä' itself cannot start a token; it is reported at column 1.
        assert_eq!(e.col, 1);
        let t = tokenize_line("\"ä😀\" x", 0, 1).unwrap();
        assert_eq!((t[1].col, t[1].len), (7, 1));
    }
}
