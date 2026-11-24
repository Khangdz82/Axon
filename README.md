# Axon Programming Language (`.ax`) — v1.2.0

**Axon** is a self-hosted, multi-OS native systems & data-science programming language combining **Pythonic readability**, **extreme native AVX2/FMA3/NEON/WASM-SIMD performance**, **zero-GC Perceus linear ownership**, **`AxIR` Mid-Level Optimization (`-O0 | -O2 | -O3 | -Os`)**, **native zero-WebView HTML5/CSS3/Tailwind/JS Desktop GUI (Photon-D2D v2)**, a **115-module pure-Axon standard library**, and **fail-closed compile-time & runtime security (`E0501`–`E0503`, `--shield`)**.

The entire compiler ([`compiler_ax/`](file:///c:/Users/Admin/Downloads/Languague/compiler_ax) — **`19` pure Axon modules, `40,878` lines**) compiles `.ax` source and `.h` C headers directly in memory into standalone executables, shared libraries, GPU kernels, and WASI WebAssembly modules across **11 hardware/OS targets** — with **zero** dependency on `LLVM`, `Clang`, `GCC`, `MSVC`, `link.exe`, `lld`, or external SDKs, and **zero build/run temp garbage**.

---

## Key Highlights (v1.2.0)

- **`AxIR` Mid-Level Optimizer & Multi-Arch Auto-Vectorization (`-O0 | -O2 | -O3 | -Os`):**
  - **5-Pass Low-RAM (`< 4 MB` bump-arena) `AxIR` Pipeline ([`compiler_ax/ir_opt.ax`](file:///c:/Users/Admin/Downloads/Languague/compiler_ax/ir_opt.ax)):** Whole-program Call-Graph Reachability (`DCE-0`), frequency-weighted bottom-up auto-inliner, **`SRoA`** (Scalar Replacement of 1–4-field value structs into CPU registers), Shield-safe `ConstProp`/`GVN`/`CSE` + dead-branch pruning, and trap-free `LICM`.
  - **Backend Codegen Optimizations:** **64-bit `MagicDiv`** (replaces `40–90c` signed `IDIV` with `4c` `IMUL` magic multipliers), Java HotSpot C2-style **`RCE`** (Range-Check Elimination inside `0..n` loops), **`8×f64` AVX2+FMA3** (`VFMADD231PD` / `VADDPD`), **`8×i64` AVX2**, **ARM64 `128-bit NEON`** + on-demand DCE (`1.40 MB` `axc-macos-arm64`), **WASM `0xFD SIMD128`**, and `-Os` size-focused unroll gating.
  - **8-Class Lock-Free Slab Allocator:** `64 B .. 64 KB`, zero `LOCK` overhead on single-threaded paths via `mt_active`, double-free canary guard (`200,000 × 4 KB` alloc/touch/free in **`1 ms`** vs Rust `10 ms`, Go `157 ms`, Java `150 ms`).
- **Complete `115`-Module Pure-Axon Standard Library (`48,817` Lines, Zero C/Python Helpers):**
  - **Mathematics Suite (`v1.2.0`):** Dense `MatF64` (LU, Cholesky, Householder QR, Least-Squares, Jacobi Eigen, Hestenes-Jacobi SVD) + sparse `CsrMat` (`CG`, `BiCGSTAB`) ([`std/linalg.ax`](file:///c:/Users/Admin/Downloads/Languague/std/linalg.ax)); zero-`libm` transcendentals, `Complex`, Radix-2 `FFT`/`IFFT`, 16-pt Gauss-Legendre, `CubicSpline`, `BrentSolver`, `Rk4State`, `LbfgsState`, `Simplex LP` ([`std/numeric.ax`](file:///c:/Users/Admin/Downloads/Languague/std/numeric.ax)); `lgamma`, `gammainc`, `betainc`, 8 distributions + `ProbRng` ([`std/prob.ax`](file:///c:/Users/Admin/Downloads/Languague/std/prob.ax)); overflow-safe base-$10^9$ `BigInt`, Miller–Rabin primality & `Rational` ([`std/bigint.ax`](file:///c:/Users/Admin/Downloads/Languague/std/bigint.ax)).
  - **Data Science & ML Suite (`v1.2.0`):** Arrow-style columnar `DataFrame` (CSV, JSONL, binary `AXDF` IPC, null bitmask, `groupby_agg`, hash `join`, `describe`) ([`std/dataframe.ax`](file:///c:/Users/Admin/Downloads/Languague/std/dataframe.ax)); Welford moments, $O(N)$ Quickselect quantiles, `welch_ttest`, `chi2`, `anova_1way`, `ks_2samp`, `OLS` econometrics, `Holt-Winters` ([`std/stats.ax`](file:///c:/Users/Admin/Downloads/Languague/std/stats.ax)); `PCA`, `Ridge`, `Lasso`, `Logistic`, `DecisionTree`, `RandomForest`, `GBDT`, `KMeans++`, `DBSCAN`, `KNN`, `GaussianNB`, `ROC-AUC`/`F1` ([`std/ml.ax`](file:///c:/Users/Admin/Downloads/Languague/std/ml.ax)).
  - **OS Auth & Identity Security (`v1.2.0`):** Kernel CSPRNG (`BCryptGenRandom`/`getentropy`/`/dev/urandom`), `SID`/`UID`, Admin elevation, `LogonUserW`, **Win32 DPAPI**, **Credential Vault** ([`std/os_auth.ax`](file:///c:/Users/Admin/Downloads/Languague/std/os_auth.ax)); RFC 9106 **`Argon2id` `v=19`**, RFC 7693 `BLAKE2b`, `PBKDF2`, `HKDF`, RFC 6238 `TOTP`/`HOTP`, `OAuth2 PKCE`, `RbacPolicy` ([`std/auth.ax`](file:///c:/Users/Admin/Downloads/Languague/std/auth.ax)).
  - **Database Suite & Upgrades (`v1.2.0`):** RESP2/RESP3 Redis + `RedisPipeline` ([`std/redis.ax`](file:///c:/Users/Admin/Downloads/Languague/std/redis.ax)); Zero-DLL Embedded ACID KV Store with IEEE `CRC-32` WAL, `begin`/`commit`/`rollback`, TTL & `compact` ([`std/kv_acid.ax`](file:///c:/Users/Admin/Downloads/Languague/std/kv_acid.ax)); SQLi-proof `SqlBuilder`, `SqlitePool` WAL & `db_migrate_sqlite` ([`std/db_pool.ax`](file:///c:/Users/Admin/Downloads/Languague/std/db_pool.ax)); **PostgreSQL 14–17 `SCRAM-SHA-256`** ([`std/postgres.ax`](file:///c:/Users/Admin/Downloads/Languague/std/postgres.ax)); SQLite `f64`/`blob`/`null` ([`std/sqlite.ax`](file:///c:/Users/Admin/Downloads/Languague/std/sqlite.ax)).
- **`axon-anything` Universal Code Migration Engine:**
  - Behavior-preserving 6-stage porting pipeline (`Analyze -> Record -> Bridge -> Port -> Diff Gate -> Lift`) from Python/NumPy/Pandas/Scikit-Learn, Rust, C/C++, Go, Java, and TypeScript into 100% Pure Axon ([`.agents/skills/axon-anything/SKILL.md`](file:///c:/Users/Admin/Downloads/Languague/.agents/skills/axon-anything/SKILL.md), [`.agents/rules/axon-porting-rules.md`](file:///c:/Users/Admin/Downloads/Languague/.agents/rules/axon-porting-rules.md), pure-Axon CLI [`axon_anything.ax`](file:///c:/Users/Admin/Downloads/Languague/.agents/skills/axon-anything/scripts/axon_anything.ax), and MCP tool `axon_port_guide`).
- **Native Photon-D2D v2 Desktop UI (`0` WebView, `0` Electron, `< 12 MB` RAM):** Write desktop apps in pure HTML5 + CSS3 + Tailwind CSS + `<script>` JS with HiDPI ClearType Direct2D rendering ([`std/ui.ax`](file:///c:/Users/Admin/Downloads/Languague/std/ui.ax), [`std/gui.ax`](file:///c:/Users/Admin/Downloads/Languague/std/gui.ax)).
- **Self-Hosted Deterministic Toolchain:**
  - Verified by 3-stage deterministic self-hosting bootstrap (`SHA256(Stage 2) == SHA256(Stage 3) = 129366ABCC3124DFBC09D3794916178F5000B6A9C3B2792902BE5380A2E52300`, `2,639,872` bytes).
  - **`77/77`** end-to-end runtime test suites ([`tests/pass/`](file:///c:/Users/Admin/Downloads/Languague/tests/pass)) + **`37/37`** compile-time rejection gates ([`tests/fail_compile/`](file:///c:/Users/Admin/Downloads/Languague/tests/fail_compile)).

---

## Documentation, Memory & Learning Guides (Humans & AI Agents)

| Audience | Document | Description |
| :--- | :--- | :--- |
| **Humans (Beginner -> Expert)** | [**`docs/LEARN_AXON_HUMAN.md`**](file:///c:/Users/Admin/Downloads/Languague/docs/LEARN_AXON_HUMAN.md) | Structured 8-module hands-on course from Day-1 syntax & Perceus 0-GC to Web/JWT/WS, SQLite, Native HTML/Tailwind GUI, and AI/GPU |
| **Humans (Full Textbooks)** | [`LEARN_AXON_VI.md`](file:///c:/Users/Admin/Downloads/Languague/LEARN_AXON_VI.md) / [`LEARN_AXON_EN.md`](file:///c:/Users/Admin/Downloads/Languague/LEARN_AXON_EN.md) | 20-chapter Vietnamese & English reference textbooks + 4-week study roadmap |
| **AI Coding Agents (Porting)** | [**`.agents/skills/axon-anything/SKILL.md`**](file:///c:/Users/Admin/Downloads/Languague/.agents/skills/axon-anything/SKILL.md) | Universal 6-stage migration engine + 115-module mapping from Python/DS, Rust, C/C++, Go, Java, TS |
| **AI Agents (Master Spec & Rules)** | [`AGENTS.md`](file:///c:/Users/Admin/Downloads/Languague/AGENTS.md) / [`docs/AGENT_MASTER_GUIDE.md`](file:///c:/Users/Admin/Downloads/Languague/docs/AGENT_MASTER_GUIDE.md) / [`AXON_LLM_SPEC.md`](file:///c:/Users/Admin/Downloads/Languague/AXON_LLM_SPEC.md) | Workspace agent rules, full 115-module lookup matrix, `E0001..E0604` fix dictionary, and compact LLM spec |
| **Architecture & Compiler Memory** | [**`MEMORY.md`**](file:///c:/Users/Admin/Downloads/Languague/MEMORY.md) / [`docs/MEMORY_ARCHITECTURE.md`](file:///c:/Users/Admin/Downloads/Languague/docs/MEMORY_ARCHITECTURE.md) | Compiler `AxIR` passes, AST/state layouts, Perceus 0-GC + slab allocator, x86-64/AVX2/ARM64 register conventions |
| **Benchmarks** | [`docs/BENCHMARK_REPORT.md`](file:///c:/Users/Admin/Downloads/Languague/docs/BENCHMARK_REPORT.md) | Reproducible multi-language benchmark methodology & hardware notes |

---

## Verified 10-Domain Benchmark Suite (Axon vs Rust, Go, Java, Node.js, Python)

Measured live (median of 3 trials, **10/10 bit-identical mathematical checksums** across all languages) on Windows x64 against `rustc -O`, `rustc -C opt-level=3 -C target-cpu=native`, `go1.26.4`, `OpenJDK 21`, `Node.js v24.15.0`, and `Python 3.11`:

| Domain | Workload | Axon | Rust (`-O`) | Rust (`-O3 native`) | Go 1.26 | Java 21 | Node v24 | Speedup vs Rust (`-O`) |
| :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: | :--- |
| **Cold Build** | Standalone native `.exe` compile | **`17 ms`** | `854 ms` | `524 ms` | `715 ms` | `641 ms` | — | **`50.2x` faster** (`30.8x` vs native) |
| **1. `alloc4k`** | `200,000 × 4 KB` alloc + R/W + free | **`1 ms`** | `10 ms` | `10 ms` | `157 ms` | `150 ms` | `410 ms` | **`10.0x` faster** |
| **2. `divmod50M`** | `50M` signed `i64` `/` & `%` pow2 | **`19 ms`** | `39 ms` | `17 ms` | `62 ms` | `76 ms` | `345 ms` | **`2.05x` faster** |
| **3. `streq5M`** | `5M × 64 B` heap-distinct `str_eq` | **`14 ms`** | `24 ms` | `26 ms` | `21 ms` | `20 ms` | `8 ms` | **`1.71x–1.86x` faster** |
| **4. `splitmix20M`** | `20M` SplitMix64 `i64` PRNG | **`18 ms`** | `15 ms` | `14 ms` | `28 ms` | `23 ms` | `956 ms` | `0.83x` (`1.56x` vs Go) |
| **5. `fib38`** | Recursive `fib(38) = 39,088,169` | **`11 ms`** | `104 ms` | `96 ms` | `184 ms` | `141 ms` | `434 ms` | **`9.45x` faster** (`8.73x` vs native) |
| **6. `sieve2M`** | Prime Sieve `2,000,000` (`148,933`) | **`3 ms`** | `3 ms` | `3 ms` | `4 ms` | `19 ms` | `31 ms` | **`1.00x` (tied)** |
| **7. `bytesum64M`** | `64 MB` unsigned byte stream sum | **`4 ms`** | `19 ms` | `7 ms` | `23 ms` | `30 ms` | `93 ms` | **`4.75x` faster** (`1.75x` vs native) |
| **8. `sort200K`** | Stdlib sort `200K` signed `i64` | **`4 ms`** | `3 ms` | `2 ms` | `13 ms` | `86 ms` | `22 ms` | `0.75x` (`3.25x` vs Go, `21.5x` vs Java) |
| **9. `qsort200K`** | Identical Lomuto QuickSort `200K` | **`13 ms`** | `12 ms` | `11 ms` | `13 ms` | `76 ms` | `20 ms` | `0.92x` (`5.85x` vs Java) |
| **10. `gemm512`** | `512 × 512` FP32 MatMul (`268.4M` FLOPs) | **`2 ms`** | `18 ms` | `10 ms` | `78 ms` | `167 ms` | `301 ms` | **`9.00x` faster** (`5.00x` vs native) |
| **Total Runtime** | **Sum of all 10 compute/mem domains** | **`88–91 ms`** | **`247 ms`** | **`196 ms`** | **`574 ms`** | **`774 ms`** | **`2,620 ms`** | **`2.71x` overall** (`6.3x` Go, `8.5x` Java, `28.8x` Node, `>270x` Python) |

---

## Complete Standard Library (`115` Pure Axon Modules, `48,817` Lines)

| Category | Modules (`use std::<name>`) | Key Capabilities |
| :--- | :--- | :--- |
| **Mathematics & Numerical** | `linalg`, `numeric`, `prob`, `bigint`, `math`, `decimal`, `rand` | `MatF64` (LU, Cholesky, QR, Eigen, SVD), `CsrMat` (`CG`/`BiCGSTAB`), `Complex`, Radix-2 `FFT`, `Gauss-Legendre`, `BrentSolver`, `Rk4State`, `LbfgsState`, `Simplex LP`, 8 distributions, arbitrary-precision `BigInt` & `Rational` |
| **Data Science & ML** | `dataframe`, `stats`, `ml`, `csv`, `json`, `toml` | Columnar `DataFrame` (CSV/JSONL/Binary IPC `AXDF`, `groupby_agg`, hash `join`), `welch_ttest`, `anova_1way`, `ks_2samp`, `OLS`, `Holt-Winters`, `PCA`, `Ridge`, `Lasso`, `RandomForest`, `GBDT`, `KMeans++`, `DBSCAN`, `ROC-AUC` |
| **OS Auth & Security** | `os_auth`, `auth`, `crypto`, `jwt`, `cap`, `taint`, `shield` | Kernel CSPRNG, Win32 DPAPI & Credential Vault, `Argon2id` `v=19`, `BLAKE2b`, `PBKDF2`, `HKDF`, `TOTP`/`HOTP`, `OAuth2 PKCE`, `RbacPolicy`, `tainted[str]` (`E0502`), `RootCap` (`E0501`), `--shield` |
| **Databases & Storage** | `postgres`, `sqlite`, `mysql`, `redis`, `kv_acid`, `db_pool` | PostgreSQL Wire v3 + `SCRAM-SHA-256`, SQLite3 (`f64`/`blob`/`null`), MySQL v10, Redis RESP2/3 + Pipeline, Embedded CRC32-WAL `KvStore`, `SqlBuilder`, `SqlitePool`, `db_migrate_sqlite` |
| **Core, Containers & Graphs** | `prelude`, `str`, `fmt`, `algo`, `ds`, `graph`, `vec`, `collections`, `ringbuf`, `cache`, `linq`, `crdt`, `bench`, `test_kit` | $O(N)$ signed `i64` Radix Sort, min-heap, DSU, auto-growing `DynIntVec`/`StrVec`/`DynMap`/`StrMap`/`IntDeque`/`BitSet`, BFS/Dijkstra/TopoSort, zero-cost generics (`Box[T]`, `Option[T]`) |
| **Web, Networking & gRPC** | `net`, `tls`, `http`, `url`, `ws`, `web`, `grpc`, `async_io` | TCP/UDP, TLS 1.2/1.3 (WinHTTP Schannel + OpenSSL), HTTP/1.1, `:param` router, SSRF guard, RFC-6455 WebSockets, gRPC + HTTP/2 + Protobuf v3 |
| **OS, Filesystem & FFI** | `fs`, `sys`, `os`, `path`, `cli`, `terminal`, `log`, `ffi`, `sync`, `memory`, `mem_guard`, `encoding`, `binary`, `regex`, `compress`, `time`, `datetime`, `uuid` | Crash-safe `fs_atomic_write`, `fs_mkdir_all`, `fs_list_dir`, 16-arg Win64/SysV C-ABI FFI (`ffi_open`/`ffi_sym`/`ffi_call_arr`), mutexes, atomics, channels |
| **Desktop GUI & WebView** | `ui`, `gui`, `html`, `css`, `ui_tailwind`, `ui_layout`, `ui_render`, `ui_js`, `ui_signal`, `webview` | Direct2D HiDPI ClearType HTML5/CSS3/Tailwind/JS desktop engine (`< 12 MB` RAM, 60 FPS inertia) + Edge/Chromium `--app=` host |
| **AI, LLM, GPU & Games** | `ai`, `ai_agent`, `ai_autograd`, `ai_engine`, `ai_eval`, `ai_gguf`, `ai_harness`, `ai_local`, `ai_lora`, `ai_math`, `ai_nn`, `ai_quant`, `ai_serve`, `cuda`, `npu`, `faiss`, `llama`, `vllm`, `transformers`, `onnx`, `safetensors`, `tiktoken`, `torch`, `numpy`, `sklearn`, `quant`, `game`, `game2d`, `game3d`, `game_ecs`, `game_physics`, `game_render`, `math3d`, `audio`, `kernel`, `ring_bridge`, `iot`, `semiconductor` | Multi-provider AI Agent Harness, AVX2+FMA3 Q4/Q8/BitNet GEMM, zero-`nvcc` PTX JIT CUDA, GGUF/LLaMA/vLLM, FAISS, SoA `Table[T]` 2D/3D Game Engine & Physics, Ring-0/MMIO |

---

## Quickstart & CLI Reference

```powershell
# 1. Sub-millisecond static verification (types, Perceus 0-GC, taint E0502, unsafe E0503, shapes)
.\bin\axc.exe check --json examples/hello.ax

# 2. Compile & run immediately (zero leftover temp binaries!) or run `@test` unit tests
.\bin\axc.exe run tests/pass/54_v1_2_ds_math_auth_db_ir.ax
.\bin\axc.exe test tests/pass/53_stdlib_v2_and_avx2_hostile.ax

# 3. Build standalone native executable (`-O2` default, `-O3` max speed/SIMD, `-Os` smallest binary)
.\bin\axc.exe build examples/hello.ax -O3 --shield --emit-map -o hello.exe
.\bin\axc.exe build examples/hello.ax -Os -o hello_tiny.exe

# 4. Cross-compile to any of the 11 built-in targets (0 external linker/SDK!)
.\bin\axc.exe build examples/hello.ax --target linux-x64 -o hello_linux
.\bin\axc.exe build examples/hello.ax --target macos-arm64 -o hello_macos
.\bin\axc.exe build examples/hello.ax --target wasm32 -o hello.wasm

# 5. Port code from Python/Rust/C++/Go/Java/TS with `axon-anything`
.\bin\axc.exe run .agents/skills/axon-anything/scripts/axon_anything.ax -- analyze path/to/legacy_src
.\bin\axc.exe run .agents/skills/axon-anything/scripts/axon_anything.ax -- gate path/to/ported.ax
```

---

## 11 Built-In Cross-Compilation Targets

| Category | Target Flag (`--target`) | Binary / Output Format | Key Capabilities |
| :--- | :--- | :--- | :--- |
| **Desktop / Server** | `win-x64` (default) | Win64 `PE32+` (`.exe`, `--shared` `.dll`/`.pyd`) | Unlimited multi-DLL `.idata`, `.edata`, `.pdata` SEH, strict `W^X`, COFF `.obj`/`.lib`, `IOCP` |
| **Desktop / Server** | `linux-x64` | Linux x86_64 `ELF64` | Direct `SYSCALL` zero-CRT runtime, `.eh_frame`, `SSE2`/`AVX2`/`AVX-512` |
| **Desktop / Server** | `linux-arm64` | Linux AArch64 `ELF64` | Native ARM64 machine code + 128-bit `NEON` + DCE |
| **Desktop / Server** | `macos-arm64` | Apple Silicon `Mach-O 64` | 16 KB page-aligned `LC_MAIN` + `NEON` + DCE |
| **Desktop / Server** | `macos-x64` | Intel macOS `Mach-O 64` | Direct Darwin BSD `SYSCALL` |
| **Web / Edge** | `wasm32` | W3C WebAssembly (`.wasm`) | `wasi_snapshot_preview1`, `0xFD SIMD128`, dynamic `memory.grow`, 0 external SDK |
| **Bare-Metal** | `unikernel-x64` | `Multiboot2` (`0xE85250D6`) | Ring-0 bare-metal kernel image + `COM1 UART` |
| **Bare-Metal** | `freestanding-x64` | Zero-OS x86_64 `ELF64` | Embedded / bootloader bare-metal output |
| **Bare-Metal** | `freestanding-riscv64` | `RV64GC` `ELF64` (`EM_RISCV=243`) | Native RISC-V 64 machine code + Reachability DCE |
| **GPU (`--emit-gpu`)** | `spirv` | Khronos `SPIR-V 1.6` (`.spv`) | Direct compute kernels from `@gpu_kernel` / `@shader` |
| **GPU (`--emit-gpu`)** | `ptx` | NVIDIA `PTX 8.0` (`.ptx`) | Direct `sm_80` GPU kernels without `nvcc` |
