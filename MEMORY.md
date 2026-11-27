# MEMORY.md — Persistent Architecture, Compiler, `AxIR`, Perceus & Stdlib Reference (`axc` v1.2.0)

> Canonical engineering memory bank for **Humans** and **AI Coding Agents** working on the Axon compiler (`compiler_ax/`), standard library (`std/`), `axon-anything` porting suite, and native applications.

---

## 1. Golden Project Invariants

1. **100% Pure Axon Self-Hosting:** Every compiler pass ([`compiler_ax/`](compiler_ax/) — **`19` files, `43,219` LOC**) and standard library module ([`std/`](std/) — **`143` files, `81,151` LOC**) is written in pure `.ax` with zero C/C++/Rust/LLVM/Python runtime or build dependencies and **zero build/run temp garbage** (`0` `.o`/`.obj`/`target/` or leaked `.axon_run_*` files).
2. **Deterministic 3-Stage Bootstrap Gate:** Any edit to `compiler_ax/*.ax` MUST pass:
   ```powershell
   .\bin\axc.exe build compiler_ax/main.ax -o axc_s1.exe
   .\axc_s1.exe build compiler_ax/main.ax -o axc_s2.exe
   .\axc_s2.exe build compiler_ax/main.ax -o axc_s3.exe
   ```
   - Verified `SHA256(axc_s2.exe) == SHA256(axc_s3.exe) == 8DA48F4B6BAA79BBF23D0BC67B77C23D56B4A1E76B6A2198B9D641EFD748C201` (`2,915,328` bytes `win-x64`, `2,866,264` bytes `linux-x64`, `1,572,928` bytes `macos-arm64`).
   - Cross-compiled outputs: `bin/axc-linux-x64` (`2,866,264` bytes), `bin/axc-macos-arm64` (`1,572,928` bytes).
3. **Full Regression Gate:** **`85/85`** [`tests/pass/*.ax`](tests/pass/) (`83` native + `2` WASM32) + **`42/42`** [`tests/fail_compile/*.ax`](tests/fail_compile/) + `--shield` / `--simd=scalar` / `-O3` / `-Os` / `wasm32` WASI + `safe-anything . --fail-on low --include-tests` (`0` findings) must pass before promoting changes.

---

## 2. Compiler Architecture (`compiler_ax/` — 19 Modules)

| Module | Responsibility |
| :--- | :--- |
| `tokens_ast.ax` | Token IDs (`tk_*`), AST node kinds (`nk_*`, `100..165`), operator IDs (`op_*`, `aop_*`), type tags (`ty_*`), target IDs (`tgt_*`, `0..10`), `ushr64`/`popcnt64`/span packing |
| `lexer.ax` | Single-pass UTF-8 / indent-tolerant / brace-delimited tokenizer, f-string interpolation, hex/bin/oct/float/dec literals |
| `parser.ax` | Recursive-descent + Pratt precedence parser (max recursion depth `256` -> `E0020` guard) |
| `c_import.ax` | Built-in `.h` C header parser for `@cimport` zero-wrapper C struct/function binding |
| `generics.ax` | Zero-cost generic struct/function (`Box[T]`, `Option[T]`, `Pair[A,B]`) AST cloner & monomorphizer |
| `comptime.ax` | Compile-time constant evaluator, `comptime_assert` (`E0604`), and LUT synthesizer |
| `sema_perceus.ax` | Type checker, Perceus linear ownership (`E0201`–`E0204`), `&T` escape blocker (`E0101`–`E0102`), immutability (`E0301`) |
| `sema_effects.ax` | Capability attenuation (`E0501`), taint flow (`E0502`), strict `unsafe` boundary (`E0503`), `@no_alloc` (`E0601`), symbolic tensor shapes (`E0602`) |
| `ir_opt.ax` | **`AxIR` Mid-Level Optimizer (`-O0\|-O1\|-O2\|-O3\|-Os` in `sym[35]`):** BFS Call-Graph `DCE-0`, frequency-weighted bottom-up auto-inliner, `SRoA`, Shield-safe `ConstProp`/`GVN`/`CSE`, dead-branch pruning, and trap-free `LICM` |
| `diagnostics.ax` | Human & structured `--json` error formatter (`E0001..E0604`), static advisor (`axc advise`), source snippet renderer |
| `codegen_runtime.ax` | Multi-OS x86-64 emitters, runtime helpers (`h_alloc_mem`, `h_free_mem`, `h_str_eq`, `h_sec_trap` PIE stack backtrace, `h_shield_verify`), `.rdata` interning, `.data` layout |
| `codegen_calls.ax` | Intrinsics (`ptr_read_*`, `ptr_write_*`, `buf_*`, `ibuf_*`, `str_eq`), 16-arg Win64/SysV `c_call_v` FFI, SIMD `math3d`, AI tensor/pool intrinsics |
| `codegen_x86_64.ax` | Function prolog/epilog, GPR/XMM promotion, `MagicDiv` signed 64-bit `IMUL` lowering, `RCE` loop bounds elimination, SoA `Table[T]` & `8×i64` / `8×f64` AVX2+FMA3 (`VFMADD231PD`) / `64B VPSADBW` vectorizers, `-Os` unroll gate |
| `codegen_arm64_rv64.ax` | Native AArch64 (`linux-arm64`, `macos-arm64` + 128-bit `NEON` `ADD V0.2D` + helper/fn DCE) and `RV64GC` (`freestanding-riscv64` + DCE) backends |
| `backend_gpu_wasm.ax` | Direct WebAssembly (`wasm32` WASI + `0xFD SIMD128` `i64x2.add` vectorizer), Khronos `SPIR-V 1.6`, and NVIDIA `PTX 8.0` GPU emitters |
| `linker_pe.ax` | In-memory Win64 `PE32+` `.exe`/`.dll` + COFF `.obj`/`.lib` linker (unlimited `.idata` IAT, `.edata`, `.pdata` SEH, strict `W^X`) |
| `linker_unix_bare.ax` | Linux/FreeBSD `ELF64`, Apple `Mach-O 64` (16 KB page-aligned), and `Multiboot2` Unikernel linkers |
| `pkg_mcp.ax` | Package manager (`axon.toml`/`axon.lock`), `axc fmt`/`advise`/`lsp`, and MCP JSON-RPC server (`axon_check`, `axon_docs`, `axon_port_guide`, `axon_fix`, `axon_spec`, etc.) |
| `main.ax` | Top-level CLI driver (`check`, `run`, `build`, `test`, `mcp`, `repl`), `--` runtime arg forwarding, auto-cleaned `.axon_run_<tag>.exe`, `.map` symbol emitter |

---

## 3. `AxIR` Optimization Pipeline & Compiler State Layout

- **Optimization Flags (`sym[35]` via `opt_parse_flag`):**
  - `0` = `-O0` (unoptimized fast debug)
  - `1` = `-O1`
  - `2` = `-O2` (default balanced production)
  - `3` = `-O3` (aggressive inlining + unrolling + AVX2/NEON/SIMD128)
  - `4` = `-Os` / `-Oz` (minimum binary size; disables loop unrolling in `codegen_x86_64.ax`)
- **5 `AxIR` Passes (`run_axir_passes` in `ir_opt.ax`, single reusable `IntBuf` arena):**
  1. `ir_build_callgraph`: Whole-program BFS reachability (`DCE-0`) + exact call-site counts + method-target flags.
  2. `ir_pass_auto_inline`: Call-frequency-weighted bottom-up cost-model auto-inliner (`ccnt > 24 -> cost <= 3`, `ccnt > 8 -> cost <= 6`, `ccnt > 3 -> cost <= 10`, single-caller `ccnt == 1 -> cost <= 26`).
  3. `ir_pass_sroa_block`: Scalar Replacement of Aggregates (`SRoA`) for non-escaping 1..4-field scalar structs (`Complex`, `Rational`, `Vec3`, `Pair`).
  4. `ir_fold_gvn_block`: Shield-safe constant folding, SSA constant propagation, strength reduction (`x ** 2 -> x * x`, exact power-of-2 float reciprocal `x / 2.0 -> x * 0.5`), dead-branch pruning, and dominance-sound `GVN`/`CSE`.
  5. `ir_pass_licm_block`: Trap-free Loop-Invariant Code Motion hoisting pure invariant locals out of `while`/`for`.
  - **Capacity Invariant:** `ir_collect_muts_stmt` uses `32768`-slot declaration/mut buffers (`16,384` locals) to safely handle giant compiler functions like `emit_expr_ax`.
- **`MagicDiv` & `RCE` (`codegen_x86_64.ax`):**
  - `magic64_signed_div` lowers signed 64-bit `/` and `%` by any positive constant `1..2^62` (`x / 10`, `x % 1000000007`) to 4-cycle `IMUL` + `SAR`.
  - `cg.state[cg_fpromo_base(cg) + 140..143]` holds `RCE` state (`rce_iv_slot`, `rce_buf_slot1`, `rce_buf_slot2`, `last_nonneg_iv_slot`, `state_need = f_cnt + 432`), stripping per-iteration bounds checks on read-only `ibuf_get`/`buf_get` inside `0..len` loops.

---

## 4. Memory Model: Perceus 0-GC + 8-Class Slab Allocator

- **Owned Linear Types:** `ByteBuf`, `IntBuf`, `Table[T]`, `MatF64`, `CsrMat`, `BigInt`, `DataFrame`, `KvStore`, and any `struct` containing an owned field (`DynIntVec`, `StrVec`, `DynMap`, `StrMap`, `IntDeque`, `BitSet`, `Graph`, `UiDoc`, `RandomForest`, `GbdtModel`).
- **Scope Exit & Loop Ban (`E0203`):** Every owned variable MUST be consumed via `move` (`buf_free(move b)`, `mat_free(move m)`, `df_free(move df)`) or returned. Moving an outer owned variable inside a `while`/`for` loop is rejected (`E0203`), and `&T` references cannot be stored in structs or returned (`E0101`/`E0102`).
- **8-Class Slab Allocator (`codegen_runtime.ax`):** `64 B`, `128 B`, `256 B`, `512 B`, `1 KB`, `2 KB`, `4 KB`, `64 KB` (`data_off_mt_active = 0x104` skips `LOCK` on single-threaded paths; double-free canary at `[RDX - 24]`). Raw `mem_alloc_aligned` callers requiring zeroed memory MUST call `mem_set(ptr, 0, bytes)`.

---

## 5. x86-64 Codegen & AVX2 Register Conventions

- **Promoted Local GPRs (`pr = 1..10`):** `RBX`, `R12`, `R13`, `R14`, `R15`, `RSI`, `RDI`, `R10`, `R11`, `R8`.
  - **`RAX`, `RCX`, `RDX`, `R9` are NEVER assigned to promoted locals.**
- **Promoted Local `f64` XMMs (`fr = 1..8`):** Assigned to callee-saved `XMM6..XMM13` (Win64). `XMM0..XMM5` (`YMM0..YMM5`) are caller-saved volatile on both Win64 and SysV. Any kernel touching `YMM6..YMM15` MUST save/restore them around the loop and emit `VZEROUPPER` (`C5 F8 77`).
- **Active Vectorizers & Fastpaths:** `try_emit_fast_rec_fn_ax` (`n <= 7`), `try_emit_avx2_u8_sum_while_ax` (`64 B/iter` `VPSADBW`), `try_emit_avx2_for_range_ax` (`8×i64`), `try_emit_avx2_f64_dot_sum_while_ax` (`8×f64` `VFMADD231PD` / `VADDPD`), `try_emit_avx2_f64_axpy_while_ax` (unwraps `nk_unsafe_block`), `try_emit_avx2_mandel_while_ax` (`4×f64`), `h_ai_gemm_f32` (`4×16` AVX2+FMA3), `h_sha256_blocks` (`10000056`, register-resident Horner+ILP multi-block SHA-256 via `sha256_compress_blocks_ptr`), `ai_pool_dispatch` (persistent 12-thread OS pool for 3-arg `fn_addr` workers), `h_str_eq` (`VPCMPEQB`), plus ARM64 NEON (`2×i64`) and WASM SIMD128 (`i64x2.add`, `128 KB`/fn + `512 KB` code buffers). Note: `emit_expr_ax` (`nk_ident`) applies deferred `cg.state[24]` (`defer_iv`) offset inside 4× unrolled loops.

---

## 6. `std/` 115-Module Ecosystem & Symbol Namespace Rules

Because `load_and_merge_program_v7` merges imported modules into a single flat namespace, **new modules MUST use module-prefixed helper names** (`la_*`, `num_*`, `prob_*`, `bi_*`, `rat_*`, `df_*`, `stats_*`, `ml_*`, `os_auth_*`, `auth_*`, `redis_*`, `kv_*`, `dbp_*`) to guarantee `0` symbol collisions when all 115 modules are imported together.

Key `v1.2.0` additions:
- **Math:** `linalg.ax`, `numeric.ax` (Reverse-Communication `BrentSolver`/`Rk4State`/`LbfgsState` + `Simplex`), `prob.ax`, `bigint.ax` (base-$10^9$ limbs).
- **Data Science:** `dataframe.ax` (CSV/JSONL/`AXDF` IPC), `stats.ax` (Welford/Quickselect/`OLS`/`Holt-Winters`), `ml.ax` (Flat-Arena `CartTree`/`RandomForest`/`GBDT`/`PCA`/`Lasso`/`KMeans++`/`DBSCAN`).
- **OS Auth & Security:** `os_auth.ax` (`std::ffi` Win32 `bcrypt`/`advapi32`/`crypt32` + Unix `libc` with graceful static fallback), `auth.ax` (RFC 9106 `Argon2id` `v=19`, `BLAKE2b`, `PBKDF2`, `HKDF`, `TOTP`, `PKCE`, `RBAC`), and **SecTest v4.0** (`sec_sast.ax` + `sec_test.ax`: 26 rule families `AX-SAST-001..026` covering Axon + polyglot XSS/CORS/TLS/secrets/AI-agent/MCP/Trojan-Source/prompt-injection/Docker/GHA, inline `ax-sec:ignore` / `safe-anything:ignore` suppressions, automatic secret redaction, and recursive `sast_scan_repo` / `sec_hunt_sast_repo`).
- **Databases:** `redis.ax`, `kv_acid.ax`, `db_pool.ax`, plus `postgres.ax` (`SCRAM-SHA-256`) and `sqlite.ax` (`f64`/`blob`/`null`/`col_type`).
- **Extreme Performance Stdlib Wave:** `algo.ax` (3-phase 12T parallel MSB-8 + L1D-resident LSD radix sort), `linalg.ax` (12T AVX2 FMA3 `MatF64` GEMM), `crypto.ax` (`sha256_compress_blocks_ptr` multi-block native kernel), `ds.ax` (1-branch `DynMap` sentinel + Fibonacci hash), `ringbuf.ax` (2-chunk contiguous `mem_copy` batching).
- **30-Benchmark Light-to-Impossible vs Rust (`examples/bench_vs_rust_30/`):** Axon `-O3` vs `rustc -C opt-level=3 -C target-cpu=native` (`results_vs_rust_30.json`, 5-run median): **`266.45 ms`** vs **`973.46 ms`** (**`3.65x`** total wall speedup, **`2.33x`** geomean, **`24/30`** wins, **`30/30`** checksum parity, **`20.0x`** faster compile `147.95 ms` vs `2961.59 ms`).

---

## 7. Critical Axon Syntax & Pitfalls (Never Forget!)

1. **Reserved Keywords:** NEVER name a variable `pass` (`tk_pass` -> `E0020`). Always use `and`, `or`, `not` (NEVER `&&`, `||`, `!`).
2. **Float Literals:** Write decimal floats (`0.000000000001` or `num_pow2_int(-40)`), NEVER scientific notation `1e-12` (`E0020`).
3. **Hex 64-bit Constants with Bit 63 Set:** Write `(0xbb67ae85 << 32) | 0x84caa73b` or negative signed `i64` rather than unsigned literals `>= 2^63` in expressions if MSB precision matters.
4. **If-Expressions:** `x := if cond { a } else { b }` requires `{ ... }` on BOTH branches; for chained expressions, write `else { if c2 { b } else { d } }` or use statement `if ... else if ...`.
5. **Error Unions (`T ! i64`):** Unwrap with `v := call() or return`, `v := call() or return -1`, or `v := call() or fallback_expr` (NEVER `or { block }`).
6. **Wrapping vs `--shield` Arithmetic:** Use `+%`, `-%`, `*%`, and `ushr64` for hashes, PRNGs, Argon2id/BLAKE2b, and modular wrap-around.
7. **Heap Strings from `StrVec`/`StrMap`:** When returning a slice backed by a mutable `StrVec` or `StrMap` (`svec_get`, `smap_get_str`) across mutations or moves, materialize an independent owned string via `"" + svec_get(...)`.
8. **Struct Construction & Call Sites:** Use `=` for named fields (`Point(x = 1, y = 2)`), and annotate `mut` and `move` explicitly at call sites.

---

## 8. Standard Verification Commands

```powershell
# 1. Fast static JSON check (< 35 ms)
.\bin\axc.exe check --json path/to/file.ax

# 2. Comprehensive v1.2.0 suite across -O2, -O3, -Os, --shield, and --simd=scalar
.\bin\axc.exe run tests/pass/54_v1_2_ds_math_auth_db_ir.ax
.\bin\axc.exe run --shield tests/pass/54_v1_2_ds_math_auth_db_ir.ax
.\bin\axc.exe run --simd=scalar tests/pass/54_v1_2_ds_math_auth_db_ir.ax

# 3. `axon-anything` porting harness (Analyze + Gate + Size Matrix)
.\bin\axc.exe run .agents/skills/axon-anything/scripts/axon_anything.ax -- analyze std/ml.ax
.\bin\axc.exe run .agents/skills/axon-anything/scripts/axon_anything.ax -- gate tests/pass/54_v1_2_ds_math_auth_db_ir.ax

# 4. Full regression sweep (85 pass + 42 fail_compile) + safe-anything
Get-ChildItem tests\pass\*.ax | ForEach-Object { .\bin\axc.exe run $_.FullName }
Get-ChildItem tests\fail_compile\*.ax | ForEach-Object { .\bin\axc.exe check $_.FullName }
safe-anything . --fail-on low --include-tests
```
