//! Token definitions for Axon using logos and lasso

use axon_span::ByteSpan;
use lasso::Spur;
use logos::Logos;

#[derive(Logos, Debug, Clone, Copy, PartialEq)]
#[logos(skip r"[ \t\r\n\f]+")]
#[logos(skip r"//[^\n]*")]
#[logos(skip r"/\*([^*]|\*[^/])*\*/")]
pub enum RawToken {
    // Keywords
    #[token("fn")]
    Fn,
    #[token("struct")]
    Struct,
    #[token("Table")]
    Table,
    #[token("let")]
    Let,
    #[token("mut")]
    Mut,
    #[token("move")]
    Move,
    #[token("isolated")]
    Isolated,
    #[token("for")]
    For,
    #[token("in")]
    In,
    #[token("if")]
    If,
    #[token("else")]
    Else,
    #[token("while")]
    While,
    #[token("return")]
    Return,
    #[token("spawn")]
    Spawn,
    #[token("join")]
    Join,
    #[token("where")]
    Where,
    #[token("true")]
    True,
    #[token("false")]
    False,

    // Identifiers
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*")]
    Ident,

    // Numbers
    #[regex(r"[0-9][0-9_]*\.[0-9][0-9_]*")]
    Float,
    #[regex(r"[0-9][0-9_]*")]
    Int,

    // Multi-character Operators
    #[token("->")]
    Arrow,
    #[token("::")]
    ColonColon,
    #[token("==")]
    EqEq,
    #[token("!=")]
    NotEq,
    #[token("<=")]
    LtEq,
    #[token(">=")]
    GtEq,
    #[token("&&")]
    AndAnd,
    #[token("||")]
    OrOr,
    #[token("+=")]
    PlusEq,
    #[token("-=")]
    MinusEq,
    #[token("*=")]
    StarEq,
    #[token("/=")]
    SlashEq,

    // Zero-UB Overflow Operators
    #[token("+%")]
    PlusWrap,
    #[token("-%")]
    MinusWrap,
    #[token("*%")]
    StarWrap,
    #[token("+!")]
    PlusSat,
    #[token("-!")]
    MinusSat,
    #[token("*!")]
    StarSat,

    // Single-character Punctuation
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("%")]
    Percent,
    #[token("=")]
    Eq,
    #[token("<")]
    Lt,
    #[token(">")]
    Gt,
    #[token("!")]
    Bang,
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token(":")]
    Colon,
    #[token(";")]
    Semi,
    #[token(",")]
    Comma,
    #[token(".")]
    Dot,
    #[token("&")]
    Amp,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TokenKind {
    // Keywords
    Fn,
    Struct,
    Table,
    Let,
    Mut,
    Move,
    Isolated,
    For,
    In,
    If,
    Else,
    While,
    Return,
    Spawn,
    Join,
    Where,
    True,
    False,

    // Values
    Ident(Spur),
    Int(i64),
    Float(f64),

    // Symbols
    Arrow,
    ColonColon,
    EqEq,
    NotEq,
    LtEq,
    GtEq,
    AndAnd,
    OrOr,
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    PlusWrap,
    MinusWrap,
    StarWrap,
    PlusSat,
    MinusSat,
    StarSat,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Eq,
    Lt,
    Gt,
    Bang,
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Colon,
    Semi,
    Comma,
    Dot,
    Amp,

    Eof,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: ByteSpan,
}
