use axon_lexer::{Lexer, TokenKind};
use axon_span::{FileId, SourceMap};
use lasso::Rodeo;

fn tokenize(src: &str) -> Vec<TokenKind> {
    let mut interner = Rodeo::default();
    let lexer = Lexer::new(src, FileId(0), &mut interner);
    lexer
        .tokenize_all()
        .unwrap()
        .into_iter()
        .map(|t| t.kind)
        .collect()
}

#[test]
fn test_keywords() {
    let src = "fn struct Table let mut move isolated for in if else while return spawn join where true false";
    let tokens = tokenize(src);
    assert_eq!(
        tokens,
        vec![
            TokenKind::Fn,
            TokenKind::Struct,
            TokenKind::Table,
            TokenKind::Let,
            TokenKind::Mut,
            TokenKind::Move,
            TokenKind::Isolated,
            TokenKind::For,
            TokenKind::In,
            TokenKind::If,
            TokenKind::Else,
            TokenKind::While,
            TokenKind::Return,
            TokenKind::Spawn,
            TokenKind::Join,
            TokenKind::Where,
            TokenKind::True,
            TokenKind::False,
            TokenKind::Eof
        ]
    );
}

#[test]
fn test_overflow_operators() {
    let src = "+% -% *% +! -! *!";
    let tokens = tokenize(src);
    assert_eq!(
        tokens,
        vec![
            TokenKind::PlusWrap,
            TokenKind::MinusWrap,
            TokenKind::StarWrap,
            TokenKind::PlusSat,
            TokenKind::MinusSat,
            TokenKind::StarSat,
            TokenKind::Eof
        ]
    );
}

#[test]
fn test_multi_char_operators() {
    let src = "-> :: == != <= >= && || += -= *= /=";
    let tokens = tokenize(src);
    assert_eq!(
        tokens,
        vec![
            TokenKind::Arrow,
            TokenKind::ColonColon,
            TokenKind::EqEq,
            TokenKind::NotEq,
            TokenKind::LtEq,
            TokenKind::GtEq,
            TokenKind::AndAnd,
            TokenKind::OrOr,
            TokenKind::PlusEq,
            TokenKind::MinusEq,
            TokenKind::StarEq,
            TokenKind::SlashEq,
            TokenKind::Eof
        ]
    );
}

#[test]
fn test_number_literals_with_underscores() {
    let src = "1_000_000 42 0.016 3.14_159";
    let tokens = tokenize(src);
    assert_eq!(
        tokens,
        vec![
            TokenKind::Int(1_000_000),
            TokenKind::Int(42),
            TokenKind::Float(0.016),
            TokenKind::Float(3.14159),
            TokenKind::Eof
        ]
    );
}

#[test]
fn test_comments_ignored() {
    let src = "
    // This is a line comment
    let x = 10; /* multi-line
    comment */
    return x;
    ";
    let mut interner = Rodeo::default();
    let lexer = Lexer::new(src, FileId(0), &mut interner);
    let tokens = lexer.tokenize_all().unwrap();
    let kinds: Vec<_> = tokens.iter().map(|t| t.kind).collect();
    assert_eq!(kinds.len(), 9); // let, Ident(x), =, Int(10), ;, return, Ident(x), ;, Eof
}

#[test]
fn test_span_line_col_tracking() {
    let src = "fn main() {\n    let x = 123;\n}";
    let mut sm = SourceMap::new();
    let file_id = sm.add_file("test.ax".into(), src.into());
    let mut interner = Rodeo::default();
    let lexer = Lexer::new(src, file_id, &mut interner);
    let tokens = lexer.tokenize_all().unwrap();

    let let_tok = tokens.iter().find(|t| t.kind == TokenKind::Let).unwrap();
    let file = sm.get(file_id).unwrap();
    let (line, col) = file.line_col(let_tok.span.start);
    assert_eq!(line, 2);
    assert_eq!(col, 5);
}

#[test]
fn test_unexpected_token_error() {
    let src = "let @bad = 1;";
    let mut interner = Rodeo::default();
    let lexer = Lexer::new(src, FileId(0), &mut interner);
    assert!(lexer.tokenize_all().is_err());
}
