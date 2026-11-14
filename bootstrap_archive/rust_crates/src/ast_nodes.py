# Aether AST Definitions
from dataclasses import dataclass, field
from typing import List, Optional, Union, Any

@dataclass
class ASTNode:
    line: int = 0
    col: int = 0

@dataclass
class TypeNode(ASTNode):
    name: str = ""
    is_table: bool = False
    elem_type: Optional[str] = None

@dataclass
class FieldDef(ASTNode):
    name: str = ""
    type_name: str = ""

@dataclass
class StructDef(ASTNode):
    name: str = ""
    fields: List[FieldDef] = field(default_factory=list)

@dataclass
class Param(ASTNode):
    name: str = ""
    type_node: Optional[TypeNode] = None
    intent: str = "let"  # "let", "mut", "move", "isolated"

@dataclass
class Expr(ASTNode):
    pass

@dataclass
class NumberLiteral(Expr):
    value: str = ""
    is_float: bool = False

@dataclass
class Identifier(Expr):
    name: str = ""

@dataclass
class FieldAccess(Expr):
    obj: str = ""
    field: str = ""

@dataclass
class BinaryExpr(Expr):
    op: str = ""
    left: Optional[Expr] = None
    right: Optional[Expr] = None

@dataclass
class CallExpr(Expr):
    callee: str = ""
    args: List[tuple] = field(default_factory=list)  # list of (intent, Expr)

@dataclass
class MethodCallExpr(Expr):
    obj: str = ""
    method: str = ""
    args: List[Expr] = field(default_factory=list)

@dataclass
class SpawnExpr(Expr):
    body: List['Statement'] = field(default_factory=list)

@dataclass
class Statement(ASTNode):
    pass

@dataclass
class LetStmt(Statement):
    name: str = ""
    type_node: Optional[TypeNode] = None
    expr: Optional[Expr] = None

@dataclass
class AssignStmt(Statement):
    target: Union[Identifier, FieldAccess] = None
    op: str = "="
    expr: Optional[Expr] = None

@dataclass
class ForStmt(Statement):
    loop_var: str = ""
    iter_target: str = ""
    body: List[Statement] = field(default_factory=list)

@dataclass
class ReturnStmt(Statement):
    is_move: bool = False
    expr: Optional[Expr] = None

@dataclass
class ExprStmt(Statement):
    expr: Optional[Expr] = None

@dataclass
class FuncDef(ASTNode):
    name: str = ""
    params: List[Param] = field(default_factory=list)
    return_type: Optional[TypeNode] = None
    body: List[Statement] = field(default_factory=list)

@dataclass
class Program(ASTNode):
    structs: List[StructDef] = field(default_factory=list)
    functions: List[FuncDef] = field(default_factory=list)
