---
name: axon-anything
description: Autonomous, behavior-preserving code migration & porting engine from ANY language (Python/NumPy/Pandas/Scikit-Learn, Rust, C/C++, Go, Java, TypeScript/Node.js) into 100% Pure Axon (`.ax` v1.2.0). Activate whenever the user asks to port, rewrite, translate, migrate, or accelerate code/repos into Axon.
---

# `axon-anything` — Universal Behavior-Preserving Porting Engine for Axon `v1.2.0`

Inspired by battle-tested multi-stage compiler migration pipelines (such as `rust-anything`), `axon-anything` eliminates hallucinated rewrites by enforcing a deterministic **6-Stage Measure-First Porting Pipeline** backed by Axon's **115 native `std/*.ax` modules**, `AxIR` (`-O2`/`-O3`/`-Os`) optimizer, and Perceus 0-GC ownership verifier.

## Reference Guides (Read On Demand)
- [115-Module Ecosystem Mapping](references/stdlib_map_115.md) — Exact 1-to-1 lookup from Python, Rust, C/C++, Go, Java, and TypeScript libraries to `std::*`.
- [Porting from Python / Data Science](references/from_python_ds.md) — `NumPy`/`SciPy`/`Pandas`/`Polars`/`Statsmodels`/`Scikit-Learn` -> `std::linalg`, `std::numeric`, `std::prob`, `std::dataframe`, `std::stats`, `std::ml`.
- [Porting from Rust & C/C++](references/from_rust_cpp.md) — Borrow-checker/RAII/Templates/SIMD/FFI -> Perceus `move`, flat arenas, generics `[T]`, `AxIR` auto-SIMD, `std::ffi`.
- [Porting from Go, Java & TypeScript](references/from_go_java_ts.md) — Goroutines/Channels/Streams/Express/JWT/SQL/Redis -> `std::web`, `std::auth`, `std::db_pool`, `std::redis`, `std::kv_acid`, `std::grpc`.
- **Mandatory Rules:** Always obey [`.agents/rules/axon-porting-rules.md`](../../rules/axon-porting-rules.md).
- **MCP Tool:** Query `axon_port_guide` (and `axon_docs`, `axon_check`, `axon_fix`) via `axc mcp` for instant online lookup.

---

## The 6-Stage `axon-anything` Workflow

### Stage 1 — `ANALYZE` (Dependency DAG & Idiom Mapping)
Run the pure-Axon analyzer on the source directory or file:
```powershell
.\bin\axc.exe run .agents/skills/axon-anything/scripts/axon_anything.ax -- analyze <source_dir_or_file>
```
1. Inventory source files, lines of code, entry points, external packages, FFI boundaries, and stateful I/O.
2. Sort modules bottom-up (**leaf pure math/types first -> data structures/storage -> services/CLI/GUI last**).
3. Map every external dependency to Axon's 115 built-in `std::*` modules using [stdlib_map_115.md](references/stdlib_map_115.md) (`0` C/Python fallbacks allowed!).

### Stage 2 — `RECORD` (Capture Golden I/O Oracle)
Before writing Axon code, capture deterministic golden inputs & outputs (`oracle.jsonl`, each line `{"case":"...","expected":"..."}`) from the legacy implementation or specification vectors (RFC vectors, SQL queries, CSV aggregates, numerical outputs).

### Stage 3 — `BRIDGE` (Incremental Seam When Porting Large Systems)
For multi-thousand-line systems:
- **C/C++/Rust shared libraries:** Bind legacy `.dll`/`.so`/`.dylib` temporarily via `std::ffi` (`ffi_open`, `ffi_sym`, `ffi_call*`) OR export Axon's finished leaf modules as native C-ABI DLLs (`axc build mod.ax --shared -o mod.dll`) while porting upper layers.
- **Microservices / Scripts:** Keep wire compatibility (HTTP JSON via `std::web`, gRPC via `std::grpc`, Redis via `std::redis`, SQLite/Postgres via `std::sqlite`/`std::postgres`).

### Stage 4 — `PORT` (Leaf-Up Pure-Axon Synthesis)
Translate module-by-module from leaves to root:
1. Replace pointer-heavy object graphs (`Box<Node>`, Python dicts-of-objects, Java class hierarchies) with **Data-Oriented Flat Arenas** (`MatF64`, `DataFrame`, `StrVec`, `DynMap`, `IntBuf`).
2. Use `+%`, `-%`, `*%`, `ushr64` for intentional wrapping bit-math so `--shield` overflow protection stays armed on business/scientific logic.
3. Run `< 10 ms` static verification after **every** file edit:
   ```powershell
   .\bin\axc.exe check --json path/to/ported.ax
   ```

### Stage 5 — `REPLAY & DIFFERENTIAL GATE` (Zero-Regression Proof)
Run the automated 4-gate harness (`check` + `@test` + `--shield` build + golden output diff):
```powershell
.\bin\axc.exe run .agents/skills/axon-anything/scripts/axon_anything.ax -- gate path/to/ported.ax
.\bin\axc.exe run .agents/skills/axon-anything/scripts/axon_anything.ax -- diff oracle.jsonl path/to/ported.ax
```

### Stage 6 — `LIFT` (Idiomatic `AxIR` & Zero-Garbage Optimization)
Once behavior is 100% bit/tolerance-verified:
1. **Promote Hot Inner Loops for `AxIR` Auto-SIMD:** Ensure dense inner loops over `MatF64` / `IntBuf` use simple `mut j := 0; while j < n` indexing so `AxIR` Pass 7 (`RCE`) strips bounds branches and Pass 8 emits 8-wide `AVX2+FMA3` (`VFMADD231PD` / `VPADDD`), `ARM64 NEON`, or `WASM SIMD128`.
2. **Promote Value Aggregates for `SRoA`:** Small `1..4`-field scalar structs (`Complex`, `Rational`, `Vec3`, `Pair`) stay in CPU registers automatically via `AxIR` Pass 3 (`SRoA`).
3. **Production Build & Size/Speed Benchmark:**
   ```powershell
   .\bin\axc.exe build path/to/ported.ax -O3 --shield -o app.exe
   .\bin\axc.exe run .agents/skills/axon-anything/scripts/axon_anything.ax -- bench path/to/ported.ax
   ```
   Use `-Os` when targeting minimum binary footprint, or `--target linux-x64` / `--target macos-arm64` / `--target wasm32` for instant cross-OS delivery with **zero** `.o`/`.obj`/`target/` build garbage!
