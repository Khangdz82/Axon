# AXON COMPILER PROGRESS TRACKER

## Milestones Overview
- [x] **Milestone M0: Nền móng Workspace, Lexer & Index-Based Parser** (COMPLETED & VERIFIED)
  - [x] Workspace configuration (`Cargo.toml`)
  - [x] `axon_span`: ByteSpan, FileId, SourceMap, line/column tracking, pretty error formatting
  - [x] `axon_lexer`: `logos` token engine + `lasso` interner + overflow operators (`+%`, `-%`, `*%`, `+!`, `-!`, `*!`)
  - [x] `axon_ast`: `bumpalo` arena-allocated index-based AST (`NodeId(u32)`, `ExprId(u32)`, `StmtId(u32)`, `TypeId(u32)`)
  - [x] `axon_parser`: Pratt Parser + recursive descent for struct, fn, Table, where, spawn, join
  - [x] CLI driver: `axc check <file.ax>` with source code error diagnostics
  - [x] Gate M0 verification: 53/53 tests passed (7 lexer tests + 31 valid syntax tests + 15 syntax error tests with exact line/col reporting)
- [ ] **Milestone M1: Type Checker & Second-Class Reference Verifier (Local Reasoning)** (NEXT)
- [ ] **Milestone M2: Perceus IR & In-Place Memory Reuse (FBIP)** (LOCKED)
- [ ] **Milestone M3: Native SoA (Table[T]) & C23 Transpiler Backend (MVP End-to-End)** (LOCKED)
- [ ] **Milestone M4: Runtime Đa luồng Colorless Concurrency** (LOCKED)
- [ ] **Milestone M5: Dual-Backend Production (Cranelift Dev Mode & MLIR Release Mode)** (LOCKED)
