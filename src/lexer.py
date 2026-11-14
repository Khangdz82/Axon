# Aether Lexer
import re
from dataclasses import dataclass
from typing import List

@dataclass
class Token:
    type: str
    value: str
    line: int
    col: int

KEYWORDS = {
    "struct", "fn", "let", "mut", "move", "isolated",
    "Table", "for", "in", "return", "spawn"
}

TYPE_KEYWORDS = {
    "i32", "i64", "u32", "u64", "f32", "f64", "bool", "void"
}

class Lexer:
    def __init__(self, code: str):
        self.code = code
        self.pos = 0
        self.line = 1
        self.col = 1
        self.length = len(code)

    def error(self, msg: str):
        raise SyntaxError(f"Lexer error at line {self.line}, col {self.col}: {msg}")

    def peek(self, offset: int = 0) -> str:
        idx = self.pos + offset
        return self.code[idx] if idx < self.length else "\0"

    def advance(self) -> str:
        ch = self.peek()
        self.pos += 1
        if ch == "\n":
            self.line += 1
            self.col = 1
        else:
            self.col += 1
        return ch

    def tokenize(self) -> List[Token]:
        tokens: List[Token] = []
        while self.pos < self.length:
            ch = self.peek()

            # Skip whitespace
            if ch.isspace():
                self.advance()
                continue

            # Skip single-line comments
            if ch == "/" and self.peek(1) == "/":
                while self.peek() != "\0" and self.peek() != "\n":
                    self.advance()
                continue

            start_line, start_col = self.line, self.col

            # Multi-character symbols
            two_ch = ch + self.peek(1)
            if two_ch in {"->", "+=", "-=", "*=", "/=", "+%", "-%", "*%", "+!", "-!", "*!"}:
                self.advance()
                self.advance()
                tokens.append(Token(two_ch, two_ch, start_line, start_col))
                continue

            # Single-character symbols
            if ch in "+-*/=(){}[]:;,.&":
                self.advance()
                tokens.append(Token(ch, ch, start_line, start_col))
                continue

            # Number literal (int or float)
            if ch.isdigit():
                num_str = ""
                has_dot = False
                while self.peek().isdigit() or (self.peek() == "." and not has_dot and self.peek(1).isdigit()):
                    if self.peek() == ".":
                        has_dot = True
                    num_str += self.advance()
                t_type = "FLOAT" if has_dot else "INT"
                tokens.append(Token(t_type, num_str, start_line, start_col))
                continue

            # Identifier or keyword
            if ch.isalpha() or ch == "_":
                ident = ""
                while self.peek().isalnum() or self.peek() == "_":
                    ident += self.advance()

                if ident in KEYWORDS:
                    t_type = ident.upper()
                elif ident in TYPE_KEYWORDS:
                    t_type = "TYPE"
                else:
                    t_type = "IDENT"
                tokens.append(Token(t_type, ident, start_line, start_col))
                continue

            self.error(f"Unexpected character '{ch}'")

        tokens.append(Token("EOF", "", self.line, self.col))
        return tokens
