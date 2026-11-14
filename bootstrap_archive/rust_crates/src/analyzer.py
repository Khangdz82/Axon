# Aether Local Reasoning Semantic Analyzer
from typing import Dict, Set, Optional, List
from src.ast_nodes import (
    Program, StructDef, FuncDef, Statement, LetStmt, AssignStmt,
    ForStmt, ReturnStmt, ExprStmt, Expr, Identifier, FieldAccess,
    BinaryExpr, CallExpr, MethodCallExpr, SpawnExpr, NumberLiteral
)

class SemanticError(Exception):
    def __init__(self, msg: str, line: int = 0, col: int = 0):
        super().__init__(f"Semantic error at line {line}, col {col}: {msg}")
        self.msg = msg
        self.line = line
        self.col = col

class LocalReasoningAnalyzer:
    def __init__(self, program: Program):
        self.program = program
        self.structs: Dict[str, StructDef] = {s.name: s for s in program.structs}
        self.functions: Dict[str, FuncDef] = {f.name: f for f in program.functions}

    def analyze(self):
        # 1. Verify Struct definitions (no reference types allowed in structs)
        for s in self.program.structs:
            self._verify_struct(s)

        # 2. Local Reasoning: Verify each function completely independently (Hermetic)
        for f in self.program.functions:
            self._verify_function(f)

    def _verify_struct(self, s: StructDef):
        for f in s.fields:
            if f.type_name.startswith("&") or "ref" in f.type_name.lower():
                raise SemanticError(
                    f"Struct '{s.name}' cannot contain reference field '{f.name}'. "
                    f"Aether enforces Second-Class References.",
                    s.line, s.col
                )

    def _verify_function(self, f: FuncDef):
        # Local symbol table: var_name -> intent ("let", "mut", "move", "isolated", "owned")
        env: Dict[str, str] = {}
        moved_vars: Set[str] = set()

        for p in f.params:
            env[p.name] = p.intent

        self._verify_block(f.body, env, moved_vars, f)

    def _verify_block(self, stmts: List[Statement], env: Dict[str, str], moved_vars: Set[str], f: FuncDef):
        for stmt in stmts:
            if isinstance(stmt, LetStmt):
                if stmt.expr:
                    self._check_expr(stmt.expr, env, moved_vars)
                env[stmt.name] = "owned"

            elif isinstance(stmt, AssignStmt):
                if isinstance(stmt.target, Identifier):
                    target_name = stmt.target.name
                    if target_name in moved_vars:
                        raise SemanticError(f"Cannot assign to moved variable '{target_name}'", stmt.line, stmt.col)
                    intent = env.get(target_name, "owned")
                    if intent == "let":
                        raise SemanticError(f"Cannot mutate immutable 'let' variable '{target_name}'", stmt.line, stmt.col)
                elif isinstance(stmt.target, FieldAccess):
                    obj_name = stmt.target.obj
                    if obj_name in moved_vars:
                        raise SemanticError(f"Cannot access moved variable '{obj_name}'", stmt.line, stmt.col)
                if stmt.expr:
                    self._check_expr(stmt.expr, env, moved_vars)

            elif isinstance(stmt, ForStmt):
                if stmt.iter_target in moved_vars:
                    raise SemanticError(f"Cannot iterate over moved variable '{stmt.iter_target}'", stmt.line, stmt.col)
                # Loop variable is a synthetic proxy with 'mut' intent if table is mutable
                local_env = dict(env)
                local_env[stmt.loop_var] = "proxy_mut"
                self._verify_block(stmt.body, local_env, moved_vars, f)

            elif isinstance(stmt, ReturnStmt):
                if stmt.is_move:
                    if isinstance(stmt.expr, Identifier):
                        if stmt.expr.name in moved_vars:
                            raise SemanticError(f"Cannot move already-moved variable '{stmt.expr.name}'", stmt.line, stmt.col)
                        moved_vars.add(stmt.expr.name)
                elif stmt.expr:
                    self._check_expr(stmt.expr, env, moved_vars)

            elif isinstance(stmt, ExprStmt):
                if stmt.expr:
                    self._check_expr(stmt.expr, env, moved_vars)

    def _check_expr(self, expr: Expr, env: Dict[str, str], moved_vars: Set[str]):
        if isinstance(expr, Identifier):
            if expr.name in moved_vars:
                raise SemanticError(f"Use of moved variable '{expr.name}' (Linear isolation violation)", expr.line, expr.col)

        elif isinstance(expr, BinaryExpr):
            self._check_expr(expr.left, env, moved_vars)
            self._check_expr(expr.right, env, moved_vars)

        elif isinstance(expr, CallExpr):
            for intent, arg in expr.args:
                if intent == "move" and isinstance(arg, Identifier):
                    if arg.name in moved_vars:
                        raise SemanticError(f"Cannot move already moved variable '{arg.name}'", arg.line, arg.col)
                    moved_vars.add(arg.name)
                else:
                    self._check_expr(arg, env, moved_vars)

        elif isinstance(expr, MethodCallExpr):
            if expr.obj in moved_vars:
                raise SemanticError(f"Cannot call method on moved variable '{expr.obj}'", expr.line, expr.col)
            for arg in expr.args:
                self._check_expr(arg, env, moved_vars)

        elif isinstance(expr, SpawnExpr):
            # Spawn block creates a concurrent fiber.
            # Any variables accessed from outer scope must be 'isolated' and are moved into the fiber.
            sub_env = dict(env)
            sub_moved = set(moved_vars)
            self._verify_block(expr.body, sub_env, sub_moved, None)
            # Propagate newly moved variables to parent scope
            moved_vars.update(sub_moved)
