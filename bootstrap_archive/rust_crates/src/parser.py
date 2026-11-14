# Aether Recursive Descent Parser
from typing import List, Optional
from src.lexer import Token, Lexer
from src.ast_nodes import (
    Program, StructDef, FieldDef, FuncDef, Param, TypeNode,
    Statement, LetStmt, AssignStmt, ForStmt, ReturnStmt, ExprStmt,
    Expr, BinaryExpr, NumberLiteral, Identifier, FieldAccess,
    CallExpr, MethodCallExpr, SpawnExpr
)

class Parser:
    def __init__(self, tokens: List[Token]):
        self.tokens = tokens
        self.pos = 0

    def current(self) -> Token:
        return self.tokens[self.pos]

    def peek(self, offset: int = 1) -> Token:
        idx = min(self.pos + offset, len(self.tokens) - 1)
        return self.tokens[idx]

    def error(self, msg: str):
        tok = self.current()
        raise SyntaxError(f"Parser error at line {tok.line}, col {tok.col} near '{tok.value}': {msg}")

    def match(self, *expected_types: str) -> bool:
        if self.current().type in expected_types:
            self.pos += 1
            return True
        return False

    def expect(self, expected_type: str) -> Token:
        tok = self.current()
        if tok.type != expected_type:
            self.error(f"Expected '{expected_type}', found '{tok.type}' ('{tok.value}')")
        self.pos += 1
        return tok

    def parse_program(self) -> Program:
        program = Program()
        while self.current().type != "EOF":
            if self.current().type == "STRUCT":
                program.structs.append(self.parse_struct())
            elif self.current().type == "FN":
                program.functions.append(self.parse_function())
            else:
                self.error(f"Top level declaration must be struct or fn, got '{self.current().value}'")
        return program

    def parse_struct(self) -> StructDef:
        tok = self.expect("STRUCT")
        name = self.expect("IDENT").value
        self.expect("{")
        fields = []
        while self.current().type != "}" and self.current().type != "EOF":
            fname = self.expect("IDENT").value
            self.expect(":")
            ftype = self.parse_type().name
            fields.append(FieldDef(tok.line, tok.col, fname, ftype))
            self.match(",")
        self.expect("}")
        return StructDef(tok.line, tok.col, name, fields)

    def parse_type(self) -> TypeNode:
        tok = self.current()
        if self.match("&"):
            base = self.parse_type()
            return TypeNode(tok.line, tok.col, name=f"&{base.name}", is_table=False)
        if self.match("TABLE"):
            self.expect("[")
            elem_tok = self.expect("IDENT")
            self.expect("]")
            return TypeNode(tok.line, tok.col, name=f"Table[{elem_tok.value}]", is_table=True, elem_type=elem_tok.value)
        elif self.match("TYPE", "IDENT"):
            return TypeNode(tok.line, tok.col, name=tok.value, is_table=False)
        self.error("Expected type specification")

    def parse_function(self) -> FuncDef:
        tok = self.expect("FN")
        name = self.expect("IDENT").value
        self.expect("(")
        params = []
        if self.current().type != ")":
            params.append(self.parse_param())
            while self.match(","):
                params.append(self.parse_param())
        self.expect(")")

        ret_type = None
        if self.match("->"):
            ret_type = self.parse_type()

        self.expect("{")
        body = self.parse_block_statements()
        self.expect("}")
        return FuncDef(tok.line, tok.col, name, params, ret_type, body)

    def parse_param(self) -> Param:
        tok = self.current()
        intent = "let"
        if self.current().type in {"LET", "MUT", "MOVE", "ISOLATED"}:
            intent = self.current().type.lower()
            self.pos += 1
        name = self.expect("IDENT").value
        self.expect(":")
        type_node = self.parse_type()
        return Param(tok.line, tok.col, name, type_node, intent)

    def parse_block_statements(self) -> List[Statement]:
        stmts = []
        while self.current().type not in {"}", "EOF"}:
            stmts.append(self.parse_statement())
        return stmts

    def parse_statement(self) -> Statement:
        tok = self.current()
        if self.match("LET"):
            name = self.expect("IDENT").value
            type_node = None
            if self.match(":"):
                type_node = self.parse_type()
            self.expect("=")
            expr = self.parse_expression()
            self.match(";")
            return LetStmt(tok.line, tok.col, name, type_node, expr)

        if self.match("RETURN"):
            is_move = self.match("MOVE")
            expr = None
            if self.current().type not in {";", "}"}:
                expr = self.parse_expression()
            self.match(";")
            return ReturnStmt(tok.line, tok.col, is_move, expr)

        if self.match("FOR"):
            loop_var = self.expect("IDENT").value
            self.expect("IN")
            iter_target = self.expect("IDENT").value
            self.expect("{")
            body = self.parse_block_statements()
            self.expect("}")
            return ForStmt(tok.line, tok.col, loop_var, iter_target, body)

        # Assignment or expression statement
        if tok.type == "IDENT":
            next_tok = self.peek(1)
            # Check for field assignment: p.pos_x += ...
            if next_tok.type == "." and self.peek(3).type in {"=", "+=", "-=", "*=", "/="}:
                obj = self.expect("IDENT").value
                self.expect(".")
                fld = self.expect("IDENT").value
                op = self.current().type
                self.pos += 1
                expr = self.parse_expression()
                self.match(";")
                return AssignStmt(tok.line, tok.col, FieldAccess(tok.line, tok.col, obj, fld), op, expr)
            # Check for direct variable assignment: x = ...
            if next_tok.type in {"=", "+=", "-=", "*=", "/="}:
                name = self.expect("IDENT").value
                op = self.current().type
                self.pos += 1
                expr = self.parse_expression()
                self.match(";")
                return AssignStmt(tok.line, tok.col, Identifier(tok.line, tok.col, name), op, expr)

        expr = self.parse_expression()
        self.match(";")
        return ExprStmt(tok.line, tok.col, expr)

    def parse_expression(self) -> Expr:
        return self.parse_binary()

    def parse_binary(self) -> Expr:
        left = self.parse_primary()
        while self.current().type in {"+", "-", "*", "/", "+%", "-%", "*%", "+!", "-!", "*!"}:
            op = self.current().type
            self.pos += 1
            right = self.parse_primary()
            left = BinaryExpr(left.line, left.col, op, left, right)
        return left

    def parse_primary(self) -> Expr:
        tok = self.current()

        if self.match("SPAWN"):
            self.expect("{")
            body = self.parse_block_statements()
            self.expect("}")
            return SpawnExpr(tok.line, tok.col, body)

        if tok.type in {"INT", "FLOAT"}:
            self.pos += 1
            return NumberLiteral(tok.line, tok.col, tok.value, tok.type == "FLOAT")

        if tok.type == "IDENT":
            ident_name = tok.value
            self.pos += 1
            # Function call: foo(mut a, b)
            if self.match("("):
                args = []
                if self.current().type != ")":
                    args.append(self.parse_call_arg())
                    while self.match(","):
                        args.append(self.parse_call_arg())
                self.expect(")")
                return CallExpr(tok.line, tok.col, ident_name, args)
            # Method call or Field access: obj.method() or obj.field
            if self.match("."):
                member = self.expect("IDENT").value
                if self.match("("):
                    margs = []
                    if self.current().type != ")":
                        margs.append(self.parse_expression())
                        while self.match(","):
                            margs.append(self.parse_expression())
                    self.expect(")")
                    return MethodCallExpr(tok.line, tok.col, ident_name, member, margs)
                return FieldAccess(tok.line, tok.col, ident_name, member)
            return Identifier(tok.line, tok.col, ident_name)

        if self.match("("):
            expr = self.parse_expression()
            self.expect(")")
            return expr

        self.error(f"Unexpected token in expression: '{tok.value}'")

    def parse_call_arg(self) -> tuple:
        intent = "let"
        if self.current().type in {"MUT", "MOVE"}:
            intent = self.current().type.lower()
            self.pos += 1
        expr = self.parse_expression()
        return (intent, expr)
