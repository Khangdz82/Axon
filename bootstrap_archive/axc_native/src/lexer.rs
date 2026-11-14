//! Axon v5.0 Zero-Ceremony Lexer
//! Supports Python/Go-clean syntax (optional `;`, `:=`, `f"..."`, `29.99d` decimals, `|>`, `and`/`or`/`not`)
//! as well as full systems/security annotations (`@shield`, `@secret`, `+%`, `isolated`, `&`).

#[derive(Debug, Clone, PartialEq)]
pub enum FStrSegment {
    Lit(String),
    Expr(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Ident(String),
    Int(i64),
    Float(f64),
    Dec(i64), // Fixed-point scale 10_000 (4 decimal digits, exact financial math)
    Str(String),
    FStr(Vec<FStrSegment>),
    Attr(String), // @shield, @secret, @simd, @route, etc.

    // Keywords
    Fn,
    Struct,
    Enum,
    Interface,
    Impl,
    Use,
    Let,
    Mut,
    Isolated,
    Move,
    Spawn,
    If,
    Else,
    While,
    For,
    In,
    Return,
    Break,
    Continue,
    Match,
    Where,
    And,
    Or,
    Not,
    Is,
    With,
    As,
    True,
    False,
    AsmBang,

    // Operators & Punctuation
    ColonEq,    // :=
    PlusEq,     // +=
    MinusEq,    // -=
    StarEq,     // *=
    SlashEq,    // /=
    WrapAdd,    // +%
    WrapSub,    // -%
    WrapMul,    // *%
    PipeGt,     // |>
    Arrow,      // ->
    FatArrow,   // =>
    ColonColon, // ::
    DotDot,     // ..
    EqEq,       // ==
    NotEq,      // !=
    LtEq,       // <=
    GtEq,       // >=
    Shl,        // <<
    Shr,        // >>
    Lt,         // <
    Gt,         // >
    Plus,       // +
    Minus,      // -
    Star,       // *
    Slash,      // /
    Percent,    // %
    Amp,        // &
    Pipe,       // |
    Caret,      // ^
    Bang,       // !
    Question,   // ?
    Eq,         // =
    Colon,      // :
    Semi,       // ;
    Comma,      // ,
    Dot,        // .
    LParen,     // (
    RParen,     // )
    LBracket,   // [
    RBracket,   // ]
    LBrace,     // {
    RBrace,     // }
    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, Clone)]
pub struct LexError {
    pub msg: String,
    pub line: usize,
    pub col: usize,
}

pub fn tokenize(src: &str) -> Result<Vec<Token>, LexError> {
    let bytes = src.as_bytes();
    let len = bytes.len();
    let mut i = if len >= 3 && bytes[0] == 0xEF && bytes[1] == 0xBB && bytes[2] == 0xBF {
        3
    } else {
        0
    };
    let mut line = 1usize;
    let mut col = 1usize;
    let mut tokens = Vec::with_capacity(len / 3 + 16);

    while i < len {
        let b = bytes[i];
        // Whitespace
        if b == b' ' || b == b'\t' || b == b'\r' {
            i += 1;
            col += 1;
            continue;
        }
        if b == b'\n' {
            i += 1;
            line += 1;
            col = 1;
            continue;
        }

        // Line comments
        if b == b'/' && i + 1 < len && bytes[i + 1] == b'/' {
            while i < len && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        // Block comments
        if b == b'/' && i + 1 < len && bytes[i + 1] == b'*' {
            let start_line = line;
            let start_col = col;
            i += 2;
            col += 2;
            let mut closed = false;
            while i + 1 < len {
                if bytes[i] == b'*' && bytes[i + 1] == b'/' {
                    i += 2;
                    col += 2;
                    closed = true;
                    break;
                }
                if bytes[i] == b'\n' {
                    line += 1;
                    col = 1;
                } else {
                    col += 1;
                }
                i += 1;
            }
            if !closed {
                return Err(LexError {
                    msg: "Unterminated block comment `/* ... */`".into(),
                    line: start_line,
                    col: start_col,
                });
            }
            continue;
        }

        let tok_line = line;
        let tok_col = col;

        // Interpolated f-string: f"..."
        if b == b'f' && i + 1 < len && bytes[i + 1] == b'"' {
            i += 2;
            col += 2;
            let mut segments = Vec::new();
            let mut cur_lit = String::new();
            let mut closed = false;
            while i < len {
                let c = bytes[i];
                if c == b'"' {
                    i += 1;
                    col += 1;
                    closed = true;
                    break;
                }
                if c == b'\\' && i + 1 < len {
                    i += 1;
                    col += 1;
                    let esc = match bytes[i] {
                        b'n' => '\n',
                        b'r' => '\r',
                        b't' => '\t',
                        b'"' => '"',
                        b'\\' => '\\',
                        b'{' => '{',
                        b'}' => '}',
                        other => other as char,
                    };
                    cur_lit.push(esc);
                    i += 1;
                    col += 1;
                    continue;
                }
                if c == b'{' {
                    if i + 1 < len && bytes[i + 1] == b'{' {
                        cur_lit.push('{');
                        i += 2;
                        col += 2;
                        continue;
                    }
                    if !cur_lit.is_empty() {
                        segments.push(FStrSegment::Lit(std::mem::take(&mut cur_lit)));
                    }
                    i += 1;
                    col += 1;
                    let expr_start = i;
                    let mut depth = 1usize;
                    while i < len && depth > 0 {
                        if bytes[i] == b'{' {
                            depth += 1;
                        } else if bytes[i] == b'}' {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        if bytes[i] == b'\n' {
                            line += 1;
                            col = 1;
                        } else {
                            col += 1;
                        }
                        i += 1;
                    }
                    if depth != 0 {
                        return Err(LexError {
                            msg: "Unclosed `{...}` interpolation inside f-string".into(),
                            line: tok_line,
                            col: tok_col,
                        });
                    }
                    let expr_str = std::str::from_utf8(&bytes[expr_start..i])
                        .unwrap_or("")
                        .trim()
                        .to_string();
                    segments.push(FStrSegment::Expr(expr_str));
                    i += 1; // skip '}'
                    col += 1;
                    continue;
                }
                let ch_str = std::str::from_utf8(&bytes[i..]).unwrap_or("");
                if let Some(ch) = ch_str.chars().next() {
                    cur_lit.push(ch);
                    let clen = ch.len_utf8();
                    i += clen;
                    if ch == '\n' {
                        line += 1;
                        col = 1;
                    } else {
                        col += 1;
                    }
                } else {
                    i += 1;
                    col += 1;
                }
            }
            if !closed {
                return Err(LexError {
                    msg: "Unterminated interpolated string `f\"...\"`".into(),
                    line: tok_line,
                    col: tok_col,
                });
            }
            if !cur_lit.is_empty() || segments.is_empty() {
                segments.push(FStrSegment::Lit(cur_lit));
            }
            tokens.push(Token {
                kind: TokenKind::FStr(segments),
                line: tok_line,
                col: tok_col,
            });
            continue;
        }

        // Standard string literal: "..."
        if b == b'"' {
            i += 1;
            col += 1;
            let mut s = String::new();
            let mut closed = false;
            while i < len {
                let c = bytes[i];
                if c == b'"' {
                    i += 1;
                    col += 1;
                    closed = true;
                    break;
                }
                if c == b'\\' && i + 1 < len {
                    i += 1;
                    col += 1;
                    let esc = match bytes[i] {
                        b'n' => '\n',
                        b'r' => '\r',
                        b't' => '\t',
                        b'0' => '\0',
                        b'"' => '"',
                        b'\\' => '\\',
                        other => other as char,
                    };
                    s.push(esc);
                    i += 1;
                    col += 1;
                    continue;
                }
                let ch_str = std::str::from_utf8(&bytes[i..]).unwrap_or("");
                if let Some(ch) = ch_str.chars().next() {
                    s.push(ch);
                    let clen = ch.len_utf8();
                    i += clen;
                    if ch == '\n' {
                        line += 1;
                        col = 1;
                    } else {
                        col += 1;
                    }
                } else {
                    i += 1;
                    col += 1;
                }
            }
            if !closed {
                return Err(LexError {
                    msg: "Unterminated string literal `\"...\"`".into(),
                    line: tok_line,
                    col: tok_col,
                });
            }
            tokens.push(Token {
                kind: TokenKind::Str(s),
                line: tok_line,
                col: tok_col,
            });
            continue;
        }

        // Attributes / Annotations: @shield, @secret, @simd, @route, etc.
        if b == b'@' {
            i += 1;
            col += 1;
            let start = i;
            while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
                col += 1;
            }
            let name = std::str::from_utf8(&bytes[start..i]).unwrap_or("").to_string();
            tokens.push(Token {
                kind: TokenKind::Attr(name),
                line: tok_line,
                col: tok_col,
            });
            continue;
        }

        // Numbers: integers, floats, hex, binary, and fixed-point `d` decimals (e.g. 29.99d)
        if b.is_ascii_digit() {
            let start = i;
            if b == b'0' && i + 1 < len && (bytes[i + 1] == b'x' || bytes[i + 1] == b'X') {
                i += 2;
                col += 2;
                let hex_start = i;
                while i < len && (bytes[i].is_ascii_hexdigit() || bytes[i] == b'_') {
                    i += 1;
                    col += 1;
                }
                let clean: String = std::str::from_utf8(&bytes[hex_start..i])
                    .unwrap_or("0")
                    .chars()
                    .filter(|&c| c != '_')
                    .collect();
                let val = u64::from_str_radix(&clean, 16).unwrap_or(0) as i64;
                tokens.push(Token {
                    kind: TokenKind::Int(val),
                    line: tok_line,
                    col: tok_col,
                });
                continue;
            }
            while i < len && (bytes[i].is_ascii_digit() || bytes[i] == b'_') {
                i += 1;
                col += 1;
            }
            let mut is_float = false;
            if i + 1 < len && bytes[i] == b'.' && bytes[i + 1].is_ascii_digit() {
                is_float = true;
                i += 1;
                col += 1;
                while i < len && (bytes[i].is_ascii_digit() || bytes[i] == b'_') {
                    i += 1;
                    col += 1;
                }
            }
            let raw_num: String = std::str::from_utf8(&bytes[start..i])
                .unwrap_or("0")
                .chars()
                .filter(|&c| c != '_')
                .collect();

            // Fixed-point financial decimal suffix `d` (e.g. `29.99d` -> 299900)
            if i < len && bytes[i] == b'd' && (i + 1 == len || !bytes[i + 1].is_ascii_alphanumeric()) {
                i += 1;
                col += 1;
                let scaled = parse_fixed_decimal(&raw_num);
                tokens.push(Token {
                    kind: TokenKind::Dec(scaled),
                    line: tok_line,
                    col: tok_col,
                });
                continue;
            }

            // Optional type suffixes on int/float literals like `10i64`, `1.0f32`
            if i < len && (bytes[i] == b'i' || bytes[i] == b'u' || bytes[i] == b'f') {
                let sfx_start = i;
                i += 1;
                while i < len && bytes[i].is_ascii_digit() {
                    i += 1;
                }
                let sfx = &bytes[sfx_start..i];
                if sfx == b"f32" || sfx == b"f64" {
                    is_float = true;
                }
                col += i - sfx_start;
            }

            if is_float {
                let fv = raw_num.parse::<f64>().unwrap_or(0.0);
                tokens.push(Token {
                    kind: TokenKind::Float(fv),
                    line: tok_line,
                    col: tok_col,
                });
            } else {
                let iv = raw_num
                    .parse::<i64>()
                    .or_else(|_| raw_num.parse::<u64>().map(|u| u as i64))
                    .unwrap_or(0);
                tokens.push(Token {
                    kind: TokenKind::Int(iv),
                    line: tok_line,
                    col: tok_col,
                });
            }
            continue;
        }

        // Identifiers & Keywords
        if b.is_ascii_alphabetic() || b == b'_' {
            let start = i;
            while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
                col += 1;
            }
            let word = std::str::from_utf8(&bytes[start..i]).unwrap_or("");
            if word == "asm" && i < len && bytes[i] == b'!' {
                i += 1;
                col += 1;
                tokens.push(Token {
                    kind: TokenKind::AsmBang,
                    line: tok_line,
                    col: tok_col,
                });
                continue;
            }
            let kind = match word {
                "fn" | "def" => TokenKind::Fn,
                "struct" | "record" => TokenKind::Struct,
                "enum" => TokenKind::Enum,
                "interface" | "trait" => TokenKind::Interface,
                "impl" => TokenKind::Impl,
                "use" | "import" => TokenKind::Use,
                "let" => TokenKind::Let,
                "mut" | "var" => TokenKind::Mut,
                "isolated" => TokenKind::Isolated,
                "move" => TokenKind::Move,
                "spawn" | "go" => TokenKind::Spawn,
                "if" => TokenKind::If,
                "else" => TokenKind::Else,
                "while" => TokenKind::While,
                "for" => TokenKind::For,
                "in" => TokenKind::In,
                "return" => TokenKind::Return,
                "break" => TokenKind::Break,
                "continue" => TokenKind::Continue,
                "match" => TokenKind::Match,
                "where" => TokenKind::Where,
                "and" => TokenKind::And,
                "or" => TokenKind::Or,
                "not" => TokenKind::Not,
                "is" => TokenKind::Is,
                "with" => TokenKind::With,
                "as" => TokenKind::As,
                "true" => TokenKind::True,
                "false" => TokenKind::False,
                _ => TokenKind::Ident(word.to_string()),
            };
            tokens.push(Token {
                kind,
                line: tok_line,
                col: tok_col,
            });
            continue;
        }

        // Multi-character operators
        let two = if i + 1 < len {
            Some((bytes[i], bytes[i + 1]))
        } else {
            None
        };
        if let Some(pair) = two {
            let m = match pair {
                (b':', b'=') => Some(TokenKind::ColonEq),
                (b'+', b'=') => Some(TokenKind::PlusEq),
                (b'-', b'=') => Some(TokenKind::MinusEq),
                (b'*', b'=') => Some(TokenKind::StarEq),
                (b'/', b'=') => Some(TokenKind::SlashEq),
                (b'+', b'%') => Some(TokenKind::WrapAdd),
                (b'-', b'%') => Some(TokenKind::WrapSub),
                (b'*', b'%') => Some(TokenKind::WrapMul),
                (b'|', b'>') => Some(TokenKind::PipeGt),
                (b'-', b'>') => Some(TokenKind::Arrow),
                (b'=', b'>') => Some(TokenKind::FatArrow),
                (b':', b':') => Some(TokenKind::ColonColon),
                (b'.', b'.') => Some(TokenKind::DotDot),
                (b'=', b'=') => Some(TokenKind::EqEq),
                (b'!', b'=') => Some(TokenKind::NotEq),
                (b'<', b'=') => Some(TokenKind::LtEq),
                (b'>', b'=') => Some(TokenKind::GtEq),
                (b'<', b'<') => Some(TokenKind::Shl),
                (b'>', b'>') => Some(TokenKind::Shr),
                (b'&', b'&') => Some(TokenKind::And),
                (b'|', b'|') => Some(TokenKind::Or),
                _ => None,
            };
            if let Some(kind) = m {
                i += 2;
                col += 2;
                tokens.push(Token {
                    kind,
                    line: tok_line,
                    col: tok_col,
                });
                continue;
            }
        }

        // Single-character tokens
        let single = match b {
            b'<' => Some(TokenKind::Lt),
            b'>' => Some(TokenKind::Gt),
            b'+' => Some(TokenKind::Plus),
            b'-' => Some(TokenKind::Minus),
            b'*' => Some(TokenKind::Star),
            b'/' => Some(TokenKind::Slash),
            b'%' => Some(TokenKind::Percent),
            b'&' => Some(TokenKind::Amp),
            b'|' => Some(TokenKind::Pipe),
            b'^' => Some(TokenKind::Caret),
            b'!' => Some(TokenKind::Bang),
            b'?' => Some(TokenKind::Question),
            b'=' => Some(TokenKind::Eq),
            b':' => Some(TokenKind::Colon),
            b';' => Some(TokenKind::Semi),
            b',' => Some(TokenKind::Comma),
            b'.' => Some(TokenKind::Dot),
            b'(' => Some(TokenKind::LParen),
            b')' => Some(TokenKind::RParen),
            b'[' => Some(TokenKind::LBracket),
            b']' => Some(TokenKind::RBracket),
            b'{' => Some(TokenKind::LBrace),
            b'}' => Some(TokenKind::RBrace),
            _ => None,
        };

        if let Some(kind) = single {
            i += 1;
            col += 1;
            tokens.push(Token {
                kind,
                line: tok_line,
                col: tok_col,
            });
        } else {
            return Err(LexError {
                msg: format!("Unexpected character `{}`", b as char),
                line: tok_line,
                col: tok_col,
            });
        }
    }

    tokens.push(Token {
        kind: TokenKind::Eof,
        line,
        col,
    });
    Ok(tokens)
}

fn parse_fixed_decimal(s: &str) -> i64 {
    if let Some((int_part, frac_part)) = s.split_once('.') {
        let hi = int_part.parse::<i64>().unwrap_or(0) * 10_000;
        let mut frac_digits = [b'0'; 4];
        for (idx, ch) in frac_part.bytes().take(4).enumerate() {
            frac_digits[idx] = ch;
        }
        let lo_str = std::str::from_utf8(&frac_digits).unwrap_or("0000");
        let lo = lo_str.parse::<i64>().unwrap_or(0);
        if int_part.starts_with('-') {
            hi - lo
        } else {
            hi + lo
        }
    } else {
        s.parse::<i64>().unwrap_or(0) * 10_000
    }
}
