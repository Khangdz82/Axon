# Aether Language Specification (v0.1)
*Eliminating C++ and Rust Bottlenecks via Value Semantics, Native SoA, and Colorless Concurrency.*

---

## 1. Core Principles

1. **Zero Lifetime Annotations ('a)**: References are second-class. They cannot be stored in structs or globals. Dangling pointers are mathematically impossible without a borrow checker.
2. **Native SoA Memory Layout**: Tables (`Table[T]`) store struct fields in separate 64-byte cache-line aligned arrays. SIMD vectorization is automatic.
3. **Perceus In-Place Reuse (FBIP)**: When an owned value is consumed and a new value is constructed, memory is reused in-place with zero reallocation.
4. **Colorless Concurrency**: No `async/await`. Sequential fibers with linear `isolated` ownership transfer eliminate data races at compile time with zero mutex overhead.
5. **Zero Undefined Behavior**: Explicit arithmetic (`+` checked, `+%` wrap, `+!` saturate). Bounds checks eliminated via compile-time local interval reasoning.

---

## 2. Grammar (EBNF)

```ebnf
Program         ::= ( StructDef | FuncDef )*

StructDef       ::= "struct" Identifier "{" ( FieldDef ","? )* "}"
FieldDef        ::= Identifier ":" Type

FuncDef         ::= "fn" Identifier "(" ( Param ("," Param)* )? ")" ( "->" Type )? Block
Param           ::= ( "let" | "mut" | "move" | "isolated" )? Identifier ":" Type

Type            ::= PrimitiveType | Identifier | TableType
TableType       ::= "Table" "[" Identifier "]"
PrimitiveType   ::= "i32" | "i64" | "u32" | "u64" | "f32" | "f64" | "bool" | "void"

Block           ::= "{" Statement* "}"

Statement       ::= LetStmt
                  | ReturnStmt
                  | ForStmt
                  | ExprStmt
                  | AssignStmt

LetStmt         ::= "let" Identifier ( ":" Type )? "=" Expr ";"
ReturnStmt      ::= "return" ( "move" )? Expr? ";"
ForStmt         ::= "for" Identifier "in" Expr Block
AssignStmt      ::= ( Identifier | FieldAccess ) ( "=" | "+=" | "-=" | "*=" | "/=" ) Expr ";"
ExprStmt        ::= Expr ";"

Expr            ::= BinaryExpr | PrimaryExpr
BinaryExpr      ::= PrimaryExpr ( Op PrimaryExpr )*
Op              ::= "+" | "-" | "*" | "/" | "+%" | "-%" | "*%" | "+!" | "-!" | "*!"

PrimaryExpr     ::= SpawnExpr
                  | CallExpr
                  | MethodCallExpr
                  | FieldAccess
                  | Identifier
                  | NumberLiteral
                  | ParenExpr

SpawnExpr       ::= "spawn" Block
MethodCallExpr  ::= Identifier "." Identifier "(" ( Expr ("," Expr)* )? ")"
CallExpr        ::= Identifier "(" ( ( "mut" | "move" )? Expr ("," ( "mut" | "move" )? Expr)* )? ")"
FieldAccess     ::= Identifier "." Identifier
ParenExpr       ::= "(" Expr ")"
```

---

## 3. Memory Layout & Lowering Rules

### 3.1 Table[T] Layout (Native SoA)
For a struct:
```aether
struct Particle {
    pos_x: f32,
    pos_y: f32,
    vel_x: f32,
    vel_y: f32,
    mass:  f32,
}
```
`Table[Particle]` is lowered to C runtime representation:
```c
typedef struct {
    size_t count;
    size_t capacity;
    float* __restrict pos_x;
    float* __restrict pos_y;
    float* __restrict vel_x;
    float* __restrict vel_y;
    float* __restrict mass;
} Aether_Table_Particle;
```
Each field array is aligned to a 64-byte boundary (`_aligned_malloc(size, 64)`) to fit CPU L1/L2 cache lines and AVX-512 / AVX2 / NEON vector registers.

### 3.2 Proxy Access Loop Lowering
```aether
for p in particles {
    p.pos_x += p.vel_x * dt;
    p.pos_y += p.vel_y * dt;
}
```
Lowers to:
```c
#pragma omp simd
for (size_t __i = 0; __i < particles.count; ++__i) {
    particles.pos_x[__i] += particles.vel_x[__i] * dt;
    particles.pos_y[__i] += particles.vel_y[__i] * dt;
}
```
Zero aliasing between `pos_x` and `vel_x` guarantees automatic CPU SIMD auto-vectorization.
