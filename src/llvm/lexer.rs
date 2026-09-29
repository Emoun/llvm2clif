//! Tokenizer for LLVM textual IR (`.ll` files).
//!
//! The lexer understands the token classes of the LLVM IR grammar that the
//! parser needs: identifiers/keywords, local (`%`) and global (`@`) names,
//! labels, integer and floating point literals, strings, metadata
//! references, attribute group references and punctuation. Comments are
//! discarded. Every token records the line it starts on for diagnostics.

use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub enum Tok {
    /// A bare identifier or keyword, e.g. `define`, `i32`, `add`.
    Ident(String),
    /// `%name`, `%12` or `%"quoted"`.
    Local(String),
    /// `@name`, `@12` or `@"quoted"`.
    Global(String),
    /// `name:` at the start of a basic block.
    Label(String),
    /// An integer literal (decimal, possibly negative).
    Int(i128),
    /// A floating point literal, kept as its source text.
    Float(String),
    /// `"..."` with escapes resolved.
    Str(Vec<u8>),
    /// `c"..."` with escapes resolved.
    CStr(Vec<u8>),
    /// `!name` or `!12` (metadata reference) — the text after `!`.
    MetaRef(String),
    /// A bare `!` (starts `!{`, `!"..."` or `!DIxxx(` forms).
    Bang,
    /// `#12` attribute group reference.
    AttrGroup(u32),
    /// `#dbg_value` and friends: the name of a debug record (LLVM 19+).
    DbgRecord(String),
    /// `|`, used between debug-info flags inside `!DIxxx(...)` nodes.
    Pipe,
    /// `$name` comdat reference.
    Comdat(String),
    /// `...`
    Ellipsis,
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Less,
    Greater,
    Comma,
    Equal,
    Star,
    Colon,
    Eof,
}

impl fmt::Display for Tok {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Tok::Ident(s) => write!(f, "{s}"),
            Tok::Local(s) => write!(f, "%{s}"),
            Tok::Global(s) => write!(f, "@{s}"),
            Tok::Label(s) => write!(f, "{s}:"),
            Tok::Int(i) => write!(f, "{i}"),
            Tok::Float(s) => write!(f, "{s}"),
            Tok::Str(s) => write!(f, "\"{}\"", String::from_utf8_lossy(s)),
            Tok::CStr(s) => write!(f, "c\"{}\"", String::from_utf8_lossy(s)),
            Tok::MetaRef(s) => write!(f, "!{s}"),
            Tok::Bang => write!(f, "!"),
            Tok::AttrGroup(n) => write!(f, "#{n}"),
            Tok::DbgRecord(s) => write!(f, "#{s}"),
            Tok::Pipe => write!(f, "|"),
            Tok::Comdat(s) => write!(f, "${s}"),
            Tok::Ellipsis => write!(f, "..."),
            Tok::LParen => write!(f, "("),
            Tok::RParen => write!(f, ")"),
            Tok::LBracket => write!(f, "["),
            Tok::RBracket => write!(f, "]"),
            Tok::LBrace => write!(f, "{{"),
            Tok::RBrace => write!(f, "}}"),
            Tok::Less => write!(f, "<"),
            Tok::Greater => write!(f, ">"),
            Tok::Comma => write!(f, ","),
            Tok::Equal => write!(f, "="),
            Tok::Star => write!(f, "*"),
            Tok::Colon => write!(f, ":"),
            Tok::Eof => write!(f, "<eof>"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Token {
    pub tok: Tok,
    pub line: u32,
}

#[derive(Debug)]
pub struct LexError {
    pub line: u32,
    pub message: String,
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for LexError {}

fn is_ident_start(c: u8) -> bool {
    c.is_ascii_alphabetic() || c == b'_' || c == b'$' || c == b'.' || c == b'-'
}

fn is_ident_char(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_' || c == b'$' || c == b'.' || c == b'-'
}

/// Splits `src` into tokens. The returned vector always ends with `Tok::Eof`.
pub fn tokenize(src: &str) -> Result<Vec<Token>, LexError> {
    let bytes = src.as_bytes();
    let mut toks = Vec::with_capacity(src.len() / 4);
    let mut i = 0usize;
    let mut line = 1u32;
    // Block labels (`name:`) are only recognized as the first token on a
    // line, which is where LLVM prints them; this keeps `key: value` pairs
    // inside metadata nodes from being mistaken for labels.
    let mut at_line_start = true;

    while i < bytes.len() {
        let c = bytes[i];
        let starts_line = at_line_start;
        at_line_start = false;
        match c {
            b'\n' => {
                line += 1;
                i += 1;
                at_line_start = true;
            }
            b' ' | b'\t' | b'\r' => {
                i += 1;
                at_line_start = starts_line;
            }
            b';' => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            b'(' => {
                toks.push(Token {
                    tok: Tok::LParen,
                    line,
                });
                i += 1;
            }
            b')' => {
                toks.push(Token {
                    tok: Tok::RParen,
                    line,
                });
                i += 1;
            }
            b'[' => {
                toks.push(Token {
                    tok: Tok::LBracket,
                    line,
                });
                i += 1;
            }
            b']' => {
                toks.push(Token {
                    tok: Tok::RBracket,
                    line,
                });
                i += 1;
            }
            b'{' => {
                toks.push(Token {
                    tok: Tok::LBrace,
                    line,
                });
                i += 1;
            }
            b'}' => {
                toks.push(Token {
                    tok: Tok::RBrace,
                    line,
                });
                i += 1;
            }
            b'<' => {
                toks.push(Token {
                    tok: Tok::Less,
                    line,
                });
                i += 1;
            }
            b'>' => {
                toks.push(Token {
                    tok: Tok::Greater,
                    line,
                });
                i += 1;
            }
            b',' => {
                toks.push(Token {
                    tok: Tok::Comma,
                    line,
                });
                i += 1;
            }
            b'=' => {
                toks.push(Token {
                    tok: Tok::Equal,
                    line,
                });
                i += 1;
            }
            b'*' => {
                toks.push(Token {
                    tok: Tok::Star,
                    line,
                });
                i += 1;
            }
            b':' => {
                toks.push(Token {
                    tok: Tok::Colon,
                    line,
                });
                i += 1;
            }
            b'.' if bytes.get(i + 1) == Some(&b'.') && bytes.get(i + 2) == Some(&b'.') => {
                toks.push(Token {
                    tok: Tok::Ellipsis,
                    line,
                });
                i += 3;
            }
            b'"' => {
                let (s, next) = lex_string(bytes, i + 1, line)?;
                toks.push(Token {
                    tok: Tok::Str(s),
                    line,
                });
                i = next;
            }
            b'c' if bytes.get(i + 1) == Some(&b'"') => {
                let (s, next) = lex_string(bytes, i + 2, line)?;
                toks.push(Token {
                    tok: Tok::CStr(s),
                    line,
                });
                i = next;
            }
            b'%' | b'@' | b'$' => {
                let (name, next) = lex_name(bytes, i + 1, line)?;
                let tok = match c {
                    b'%' => Tok::Local(name),
                    b'@' => Tok::Global(name),
                    _ => Tok::Comdat(name),
                };
                toks.push(Token { tok, line });
                i = next;
            }
            b'!' => {
                // `!name`, `!123` are metadata references; a bare `!` starts
                // an inline node (`!{`), a metadata string (`!"..."`) or a
                // specialized node (`!DILocation(`).
                let start = i + 1;
                let mut j = start;
                while j < bytes.len() && is_ident_char(bytes[j]) {
                    j += 1;
                }
                if j > start && bytes.get(j) != Some(&b'(') {
                    let name = std::str::from_utf8(&bytes[start..j]).unwrap().to_string();
                    toks.push(Token {
                        tok: Tok::MetaRef(name),
                        line,
                    });
                    i = j;
                } else {
                    toks.push(Token {
                        tok: Tok::Bang,
                        line,
                    });
                    i += 1;
                }
            }
            b'#' => {
                let start = i + 1;
                if bytes.get(start).is_some_and(|&b| is_ident_start(b)) {
                    // `#dbg_value(...)`: a debug record (LLVM 19 and newer).
                    let (name, next) = lex_ident(bytes, start);
                    toks.push(Token {
                        tok: Tok::DbgRecord(name),
                        line,
                    });
                    i = next;
                    continue;
                }
                let mut j = start;
                while j < bytes.len() && bytes[j].is_ascii_digit() {
                    j += 1;
                }
                if j == start {
                    return Err(LexError {
                        line,
                        message: "expected attribute group number after '#'".into(),
                    });
                }
                let n: u32 = std::str::from_utf8(&bytes[start..j])
                    .unwrap()
                    .parse()
                    .map_err(|_| LexError {
                        line,
                        message: "attribute group number out of range".into(),
                    })?;
                toks.push(Token {
                    tok: Tok::AttrGroup(n),
                    line,
                });
                i = j;
            }
            b'|' => {
                toks.push(Token {
                    tok: Tok::Pipe,
                    line,
                });
                i += 1;
            }
            b'0'..=b'9' | b'-' | b'+' => {
                // Numbers: integers, decimal floats (1.5e+3), hex floats
                // (0x3FF0...), and the special hex forms 0xK/0xL/0xM/0xH/0xR.
                let start = i;
                let mut j = i;
                if bytes[j] == b'-' || bytes[j] == b'+' {
                    j += 1;
                }
                if j < bytes.len() && bytes[j] == b'0' && bytes.get(j + 1) == Some(&b'x') {
                    j += 2;
                    while j < bytes.len() && bytes[j].is_ascii_hexdigit() {
                        j += 1;
                    }
                    let text = std::str::from_utf8(&bytes[start..j]).unwrap().to_string();
                    toks.push(Token {
                        tok: Tok::Float(text),
                        line,
                    });
                    i = j;
                    continue;
                }
                let digits_start = j;
                while j < bytes.len() && bytes[j].is_ascii_digit() {
                    j += 1;
                }
                if j == digits_start {
                    // A lone '-' or '+': treat as the start of an identifier
                    // (e.g. `-1` handled above; `-` alone should not happen).
                    if is_ident_start(c) {
                        let (name, next) = lex_ident(bytes, i);
                        toks.push(Token {
                            tok: Tok::Ident(name),
                            line,
                        });
                        i = next;
                        continue;
                    }
                    return Err(LexError {
                        line,
                        message: format!("unexpected character '{}'", c as char),
                    });
                }
                let mut is_float = false;
                if j < bytes.len()
                    && bytes[j] == b'.'
                    && bytes.get(j + 1).is_some_and(|d| d.is_ascii_digit())
                {
                    is_float = true;
                    j += 1;
                    while j < bytes.len() && bytes[j].is_ascii_digit() {
                        j += 1;
                    }
                }
                if j < bytes.len() && (bytes[j] == b'e' || bytes[j] == b'E') {
                    let mut k = j + 1;
                    if k < bytes.len() && (bytes[k] == b'+' || bytes[k] == b'-') {
                        k += 1;
                    }
                    if k < bytes.len() && bytes[k].is_ascii_digit() {
                        is_float = true;
                        while k < bytes.len() && bytes[k].is_ascii_digit() {
                            k += 1;
                        }
                        j = k;
                    }
                }
                let text = std::str::from_utf8(&bytes[start..j]).unwrap();
                if is_float {
                    toks.push(Token {
                        tok: Tok::Float(text.to_string()),
                        line,
                    });
                } else if starts_line
                    && bytes.get(j) == Some(&b':')
                    && bytes[start] != b'-'
                    && bytes[start] != b'+'
                {
                    // A numbered basic block label, e.g. `12:`.
                    toks.push(Token {
                        tok: Tok::Label(text.to_string()),
                        line,
                    });
                    j += 1;
                } else {
                    let v: i128 = text.parse().map_err(|_| LexError {
                        line,
                        message: format!("integer literal out of range: {text}"),
                    })?;
                    toks.push(Token {
                        tok: Tok::Int(v),
                        line,
                    });
                }
                i = j;
            }
            _ if is_ident_start(c) => {
                let (name, next) = lex_ident(bytes, i);
                if starts_line && bytes.get(next) == Some(&b':') {
                    toks.push(Token {
                        tok: Tok::Label(name),
                        line,
                    });
                    i = next + 1;
                } else {
                    toks.push(Token {
                        tok: Tok::Ident(name),
                        line,
                    });
                    i = next;
                }
            }
            _ => {
                return Err(LexError {
                    line,
                    message: format!("unexpected character '{}'", c as char),
                });
            }
        }
    }
    toks.push(Token {
        tok: Tok::Eof,
        line,
    });
    Ok(toks)
}

fn lex_ident(bytes: &[u8], start: usize) -> (String, usize) {
    let mut j = start;
    while j < bytes.len() && is_ident_char(bytes[j]) {
        j += 1;
    }
    (
        std::str::from_utf8(&bytes[start..j]).unwrap().to_string(),
        j,
    )
}

/// Lexes the name part after `%`, `@` or `$`: an identifier, a number, or a
/// quoted string.
fn lex_name(bytes: &[u8], start: usize, line: u32) -> Result<(String, usize), LexError> {
    if bytes.get(start) == Some(&b'"') {
        let (s, next) = lex_string(bytes, start + 1, line)?;
        return Ok((String::from_utf8_lossy(&s).into_owned(), next));
    }
    let mut j = start;
    while j < bytes.len() && is_ident_char(bytes[j]) {
        j += 1;
    }
    if j == start {
        return Err(LexError {
            line,
            message: "expected a name".into(),
        });
    }
    Ok((
        std::str::from_utf8(&bytes[start..j]).unwrap().to_string(),
        j,
    ))
}

/// Lexes the body of a string literal starting right after the opening
/// quote. Resolves `\xx` hex escapes and `\\`.
fn lex_string(bytes: &[u8], start: usize, line: u32) -> Result<(Vec<u8>, usize), LexError> {
    let mut out = Vec::new();
    let mut j = start;
    loop {
        match bytes.get(j) {
            None => {
                return Err(LexError {
                    line,
                    message: "unterminated string literal".into(),
                })
            }
            Some(b'"') => return Ok((out, j + 1)),
            Some(b'\\') => {
                if bytes.get(j + 1) == Some(&b'\\') {
                    out.push(b'\\');
                    j += 2;
                } else {
                    let hex = bytes.get(j + 1..j + 3).ok_or_else(|| LexError {
                        line,
                        message: "bad string escape".into(),
                    })?;
                    let s = std::str::from_utf8(hex).map_err(|_| LexError {
                        line,
                        message: "bad string escape".into(),
                    })?;
                    let v = u8::from_str_radix(s, 16).map_err(|_| LexError {
                        line,
                        message: format!("bad string escape \\{s}"),
                    })?;
                    out.push(v);
                    j += 3;
                }
            }
            Some(&c) => {
                out.push(c);
                j += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toks(s: &str) -> Vec<Tok> {
        tokenize(s).unwrap().into_iter().map(|t| t.tok).collect()
    }

    #[test]
    fn basic_tokens() {
        assert_eq!(
            toks("%1 = add nsw i32 %0, -7 ; comment"),
            vec![
                Tok::Local("1".into()),
                Tok::Equal,
                Tok::Ident("add".into()),
                Tok::Ident("nsw".into()),
                Tok::Ident("i32".into()),
                Tok::Local("0".into()),
                Tok::Comma,
                Tok::Int(-7),
                Tok::Eof
            ]
        );
    }

    #[test]
    fn labels_strings_metadata() {
        assert_eq!(
            toks("entry:\n2:\n@\"a b\" = c\"hi\\00\" !dbg !12 #0 $c ..."),
            vec![
                Tok::Label("entry".into()),
                Tok::Label("2".into()),
                Tok::Global("a b".into()),
                Tok::Equal,
                Tok::CStr(b"hi\0".to_vec()),
                Tok::MetaRef("dbg".into()),
                Tok::MetaRef("12".into()),
                Tok::AttrGroup(0),
                Tok::Comdat("c".into()),
                Tok::Ellipsis,
                Tok::Eof
            ]
        );
    }

    #[test]
    fn floats_and_bang() {
        assert_eq!(
            toks("1.5e+00 0x3FF0000000000000 !{ !\"x\" } !DILocation(line: 1)"),
            vec![
                Tok::Float("1.5e+00".into()),
                Tok::Float("0x3FF0000000000000".into()),
                Tok::Bang,
                Tok::LBrace,
                Tok::Bang,
                Tok::Str(b"x".to_vec()),
                Tok::RBrace,
                Tok::Bang,
                Tok::Ident("DILocation".into()),
                Tok::LParen,
                Tok::Ident("line".into()),
                Tok::Colon,
                Tok::Int(1),
                Tok::RParen,
                Tok::Eof
            ]
        );
    }

    #[test]
    fn labels_only_at_line_start() {
        assert_eq!(
            toks("  entry:\nx: y:\n  br label %entry"),
            vec![
                Tok::Label("entry".into()),
                Tok::Label("x".into()),
                Tok::Ident("y".into()),
                Tok::Colon,
                Tok::Ident("br".into()),
                Tok::Ident("label".into()),
                Tok::Local("entry".into()),
                Tok::Eof
            ]
        );
    }

    #[test]
    fn debug_records_and_flag_separators() {
        assert_eq!(
            toks("  #dbg_value(i32 %a, !9, !DIExpression(), !10)"),
            vec![
                Tok::DbgRecord("dbg_value".into()),
                Tok::LParen,
                Tok::Ident("i32".into()),
                Tok::Local("a".into()),
                Tok::Comma,
                Tok::MetaRef("9".into()),
                Tok::Comma,
                Tok::Bang,
                Tok::Ident("DIExpression".into()),
                Tok::LParen,
                Tok::RParen,
                Tok::Comma,
                Tok::MetaRef("10".into()),
                Tok::RParen,
                Tok::Eof
            ]
        );
        assert_eq!(
            toks("!DISubprogram(flags: DIFlagPrototyped | DIFlagAllCallsDescribed)"),
            vec![
                Tok::Bang,
                Tok::Ident("DISubprogram".into()),
                Tok::LParen,
                Tok::Ident("flags".into()),
                Tok::Colon,
                Tok::Ident("DIFlagPrototyped".into()),
                Tok::Pipe,
                Tok::Ident("DIFlagAllCallsDescribed".into()),
                Tok::RParen,
                Tok::Eof
            ]
        );
        assert_eq!(toks("#3"), vec![Tok::AttrGroup(3), Tok::Eof]);
    }
}
