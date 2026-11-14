//! axon_lexer: Ultra-fast lexer with logos and lasso string interning

pub mod token;

use axon_span::{ByteSpan, FileId};
use lasso::Rodeo;
use logos::Logos;
use thiserror::Error;

pub use token::{RawToken, Token, TokenKind};

#[derive(Error, Debug, Clone, PartialEq)]
#[error("Syntax error at {span:?}: {msg}")]
pub struct LexError {
    pub span: ByteSpan,
    pub msg: String,
}

pub struct Lexer<'a> {
    file_id: FileId,
    inner: logos::Lexer<'a, RawToken>,
    interner: &'a mut Rodeo,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str, file_id: FileId, interner: &'a mut Rodeo) -> Self {
        Self {
            file_id,
            inner: RawToken::lexer(src),
            interner,
        }
    }

    pub fn tokenize_all(mut self) -> Result<Vec<Token>, LexError> {
        let mut tokens = Vec::new();
        while let Some(raw_res) = self.inner.next() {
            let span = self.inner.span();
            let byte_span = ByteSpan::new(span.start as u32, span.end as u32, self.file_id);
            let slice = self.inner.slice();

            let raw = match raw_res {
                Ok(t) => t,
                Err(_) => {
                    return Err(LexError {
                        span: byte_span,
                        msg: format!("Unexpected token '{}'", slice),
                    });
                }
            };

            let kind = match raw {
                RawToken::Fn => TokenKind::Fn,
                RawToken::Struct => TokenKind::Struct,
                RawToken::Table => TokenKind::Table,
                RawToken::Let => TokenKind::Let,
                RawToken::Mut => TokenKind::Mut,
                RawToken::Move => TokenKind::Move,
                RawToken::Isolated => TokenKind::Isolated,
                RawToken::For => TokenKind::For,
                RawToken::In => TokenKind::In,
                RawToken::If => TokenKind::If,
                RawToken::Else => TokenKind::Else,
                RawToken::While => TokenKind::While,
                RawToken::Return => TokenKind::Return,
                RawToken::Spawn => TokenKind::Spawn,
                RawToken::Join => TokenKind::Join,
                RawToken::Where => TokenKind::Where,
                RawToken::True => TokenKind::True,
                RawToken::False => TokenKind::False,

                RawToken::Ident => {
                    let spur = self.interner.get_or_intern(slice);
                    TokenKind::Ident(spur)
                }

                RawToken::Int => {
                    let clean = slice.replace('_', "");
                    let val = clean.parse::<i64>().map_err(|_| LexError {
                        span: byte_span,
                        msg: format!("Invalid integer literal '{}'", slice),
                    })?;
                    TokenKind::Int(val)
                }

                RawToken::Float => {
                    let clean = slice.replace('_', "");
                    let val = clean.parse::<f64>().map_err(|_| LexError {
                        span: byte_span,
                        msg: format!("Invalid float literal '{}'", slice),
                    })?;
                    TokenKind::Float(val)
                }

                RawToken::Arrow => TokenKind::Arrow,
                RawToken::ColonColon => TokenKind::ColonColon,
                RawToken::EqEq => TokenKind::EqEq,
                RawToken::NotEq => TokenKind::NotEq,
                RawToken::LtEq => TokenKind::LtEq,
                RawToken::GtEq => TokenKind::GtEq,
                RawToken::AndAnd => TokenKind::AndAnd,
                RawToken::OrOr => TokenKind::OrOr,
                RawToken::PlusEq => TokenKind::PlusEq,
                RawToken::MinusEq => TokenKind::MinusEq,
                RawToken::StarEq => TokenKind::StarEq,
                RawToken::SlashEq => TokenKind::SlashEq,

                RawToken::PlusWrap => TokenKind::PlusWrap,
                RawToken::MinusWrap => TokenKind::MinusWrap,
                RawToken::StarWrap => TokenKind::StarWrap,
                RawToken::PlusSat => TokenKind::PlusSat,
                RawToken::MinusSat => TokenKind::MinusSat,
                RawToken::StarSat => TokenKind::StarSat,

                RawToken::Plus => TokenKind::Plus,
                RawToken::Minus => TokenKind::Minus,
                RawToken::Star => TokenKind::Star,
                RawToken::Slash => TokenKind::Slash,
                RawToken::Percent => TokenKind::Percent,
                RawToken::Eq => TokenKind::Eq,
                RawToken::Lt => TokenKind::Lt,
                RawToken::Gt => TokenKind::Gt,
                RawToken::Bang => TokenKind::Bang,
                RawToken::LParen => TokenKind::LParen,
                RawToken::RParen => TokenKind::RParen,
                RawToken::LBrace => TokenKind::LBrace,
                RawToken::RBrace => TokenKind::RBrace,
                RawToken::LBracket => TokenKind::LBracket,
                RawToken::RBracket => TokenKind::RBracket,
                RawToken::Colon => TokenKind::Colon,
                RawToken::Semi => TokenKind::Semi,
                RawToken::Comma => TokenKind::Comma,
                RawToken::Dot => TokenKind::Dot,
                RawToken::Amp => TokenKind::Amp,
            };

            tokens.push(Token {
                kind,
                span: byte_span,
            });
        }

        let eof_span = if let Some(last) = tokens.last() {
            ByteSpan::new(last.span.end, last.span.end, self.file_id)
        } else {
            ByteSpan::new(0, 0, self.file_id)
        };

        tokens.push(Token {
            kind: TokenKind::Eof,
            span: eof_span,
        });

        Ok(tokens)
    }
}
