# Axon Programming Language (`.ax`) — v1.2.0

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![CI Protect](https://github.com/Khangdz82/Axon/actions/workflows/ci-protect.yml/badge.svg)](https://github.com/Khangdz82/Axon/actions/workflows/ci-protect.yml)
[![Playground](https://img.shields.io/badge/Playground-WASM32_Live-orange.svg)](https://khangdz82.github.io/Axon/)

**Axon** is a self-hosted, multi-OS native systems & data-science programming language combining **Pythonic readability**, **AVX2/FMA3/NEON/WASM-SIMD performance**, **zero-GC Perceus linear ownership**, **`AxIR` Mid-Level Optimization (`-O0 | -O2 | -O3 | -Os`)**, **a lightweight native Direct2D UI engine (`Photon-Core`)**, a **143-module pure-Axon standard library**, and **fail-closed compile-time & runtime security (`E0501`–`E0503`, `--shield`, `std::sec_test`)**.

The entire compiler ([`compiler_ax/`](compiler_ax/) — **`19` pure Axon modules, `~41.1K` lines**) compiles `.ax` source and `.h` C headers directly in memory into standalone executables (`~1.4–2.5 MB` standalone compiler binaries, zero external dependencies), shared libraries, GPU kernels, and WASI WebAssembly modules across **11 hardware/OS targets** — with **zero** dependency on `LLVM`, `Clang`, `GCC`, `MSVC`, `link.exe`, `lld`, or external SDKs, and **zero build/run temp garbage**.

---

## Key Highlights (v1.2.0)

- **`AxIR` Mid-Level Optimizer & Multi-Arch Auto-Vectorization (`-O0 | -O2 | -O3 | -Os`):**
  - **5-Pass Low-RAM (`< 4 MB` bump-arena) `AxIR` Pipeline ([`compiler_ax/ir_opt.ax`](compiler_ax/ir_opt.ax)):** Whole-program Call-Graph Reachability (`DCE-0`), frequency-weighted bottom-up auto-inliner, **`SRoA`** (Scalar Replacement of 1–4-field value structs into CPU registers), Shield-safe `ConstProp`/`GVN`/`CSE` + dead-branch pruning, and trap-free `LICM`.
  - **Backend Codegen Optimizations:** **64-bit `MagicDiv`** (replaces `40–90c` signed `IDIV` with `4c` `IMUL` magic multipliers), Java HotSpot C2-style **`RCE`** (Range-Check Elimination inside `0..n` loops), **`8×f64` AVX2+FMA3** (`VFMADD231PD` / `VADDPD`), **`8×i64` AVX2**, **ARM64 `128-bit NEON`** + on-demand DCE (`1.42 MB` `axc-macos-arm64`, `2.49 MB` `axc-linux-x64`, `2.53 MB` `axc.exe`), **WASM `0xFD SIMD128`**, and `-Os` size-focused unroll gating.
  - **8-Class Lock-Free Slab Allocator:** `64 B .. 64 KB`, zero `LOCK` overhead on single-threaded paths via `mt_active`, TTAS spinlock + atomic `XCHG` double-free canary guard (`200,000 × 4 KB` hot-slot LIFO alloc/touch/free cycle in L1 cache in **`1 ms`** vs Rust `10 ms`, Go `157 ms`, Java `150 ms`; for cold/DRAM-bound `500K × 4 KB` dynamic allocation + transform, see `B7` in [`docs/BENCHMARK_REPORT.md`](docs/BENCHMARK_REPORT.md): `94.67 ms` at `3.0 MB` peak RSS).
- **Tiered `143`-Module Pure-Axon Standard Library (`~75.4K` Lines, Zero C/Python Helpers):**
  - **Core & Production-Verified:** Containers, strings, sorting/graphs, filesystem/OS/FFI, TCP/UDP/TLS/HTTP/WebSockets/JWT, SQLite/PostgreSQL (`SCRAM-SHA-256`)/Redis/ACID-WAL KV, OS Auth/DPAPI/Argon2id/TOTP, capability/taint guards (`E0501`–`E0503`), and `std::sec_test` fuzzing/SAST/SARIF.
  - **Lightweight Built-Ins & Preview Suites:** Compact single-file pure-Axon modules designed for zero-dependency / embedded workloads: numerical linear algebra (`std::linalg`, `std::numeric`, `std::prob`, `std::bigint`), `DataFrame`/ML (`std::dataframe`, `std::stats`, `std::ml`), Direct2D `Photon-Core` UI (`std::photon`, `std::ui`, `std::gui`), MCP/Python-compat layers, and AI/GPU/Game/IoT bindings.
- **`axon-anything` Universal Code Migration Engine:**
  - Behavior-preserving 6-stage porting pipeline (`Analyze -> Record -> Bridge -> Port -> Diff Gate -> Lift`) from Python/NumPy/Pandas/Scikit-Learn, Rust, C/C++, Go, Java, and TypeScript into 100% Pure Axon ([`.agents/skills/axon-anything/SKILL.md`](.agents/skills/axon-anything/SKILL.md), [`.agents/rules/axon-porting-rules.md`](.agents/rules/axon-porting-rules.md), pure-Axon CLI [`axon_anything.ax`](.agents/skills/axon-anything/scripts/axon_anything.ax), and MCP tool `axon_port_guide`).
- **Native Photon-Core / Photon-D2D v2 Desktop UI (Preview, `0` WebView, `0` Electron):** Lightweight Direct2D HiDPI ClearType engine supporting a practical subset of HTML5 + CSS3 + Tailwind utilities + `<script>` JS ([`std/photon.ax`](std/photon.ax), [`std/ui.ax`](std/ui.ax), [`std/gui.ax`](std/gui.ax)).
- **Self-Hosted Deterministic Toolchain:**
  - Verified by 3-stage deterministic self-hosting bootstrap (`SHA256(Stage 2) == SHA256(Stage 3) = D80317DD8B4C1B4F20C7A422263D9FABB3B2FC4F885145E7AACC870CDE208E3A`, `2,657,280` bytes).
  - **`85/85`** end-to-end runtime test suites ([`tests/pass/`](tests/pass/)) + **`42/42`** compile-time rejection gates ([`tests/fail_compile/`](tests/fail_compile/)).

---

## Documentation, Memory & Learning Guides (Humans & AI Agents)

| Audience | Document | Description |
| :--- | :--- | :--- |
| **Humans (Beginner -> Expert)** | [**`docs/LEARN_AXON_HUMAN.md`**](docs/LEARN_AXON_HUMAN.md) | Structured 8-module hands-on course from Day-1 syntax & Perceus 0-GC to Web/JWT/WS, SQLite, Native HTML/Tailwind GUI, and AI/GPU |
| **Humans (Full Textbooks)** | [`LEARN_AXON_VI.md`](LEARN_AXON_VI.md) / [`LEARN_AXON_EN.md`](LEARN_AXON_EN.md) | 20-chapter Vietnamese & English reference textbooks + 4-week study roadmap |
| **AI Coding Agents (Porting)** | [**`.agents/skills/axon-anything/SKILL.md`**](.agents/skills/axon-anything/SKILL.md) | Universal 6-stage migration engine + stdlib mapping from Python/DS, Rust, C/C++, Go, Java, TS |
| **AI Agents (Master Spec & Rules)** | [`AGENTS.md`](AGENTS.md) / [`docs/AGENT_MASTER_GUIDE.md`](docs/AGENT_MASTER_GUIDE.md) / [`AXON_LLM_SPEC.md`](AXON_LLM_SPEC.md) | Workspace agent rules, full stdlib lookup matrix, `E0001..E0604` fix dictionary, and compact LLM spec |
| **Architecture & Compiler Memory** | [**`docs/internal/MEMORY.md`**](docs/internal/MEMORY.md) / [`docs/MEMORY_ARCHITECTURE.md`](docs/MEMORY_ARCHITECTURE.md) | Compiler `AxIR` passes, AST/state layouts, Perceus 0-GC + slab allocator, x86-64/AVX2/ARM64 register conventions |
| **Benchmarks & Security** | [`docs/BENCHMARK_REPORT.md`](docs/BENCHMARK_REPORT.md) / [`SECURITY.md`](SECURITY.md) / [`CONTRIBUTING.md`](CONTRIBUTING.md) | Reproducible benchmark methodology, private vulnerability disclosure, and PR contribution guide |

---

## Verified 10-Domain Benchmark Suite (Axon vs Rust, Go, Java, Node.js, Python)

Measured live (median of 3 trials, **10/10 bit-identical mathematical checksums** across all languages) on Windows x64 against `rustc -O`, `rustc -C opt-level=3 -C target-cpu=native`, `go1.26.4`, `OpenJDK 21`, `Node.js v24.15.0`, and `Python 3.11`:

| Domain | Workload | Axon | Rust (`-O`) | Rust (`-O3 native`) | Go 1.26 | Java 21 | Node v24 | Speedup vs Rust (`-O`) |
| :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: | :--- |
| **Cold Build** | Standalone native `.exe` compile | **`17 ms`** | `854 ms` | `524 ms` | `715 ms` | `641 ms` | — | **`50.2x` faster** (`30.8x` vs native) |
| **1. `alloc4k`** | `200,000 × 4 KB` hot-slot LIFO reuse (L1-cached) | **`1 ms`** | `10 ms` | `10 ms` | `157 ms` | `150 ms` | `410 ms` | **`10.0x` faster** *(see `B7` for cold DRAM)* |
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

## Standard Library (`143` Pure Axon Modules, `~75.4K` Lines)

### Tier 1 — Core & Production-Verified Modules

| Category | Modules (`use std::<name>`) | Key Capabilities |
| :--- | :--- | :--- |
| **Core, Containers & Concurrency** | `prelude`, `str`, `fmt`, `algo`, `ds`, `graph`, `vec`, `collections`, `itertools`, `ringbuf`, `cache`, `linq`, `crdt`, `threading`, `futures`, `sync`, `bench`, `test_kit` | $O(N)$ signed `i64` Radix Sort, min-heap, DSU, `DynIntVec`/`StrVec`/`DynMap`/`StrMap`/`IntDeque`/`BitSet`, BFS/Dijkstra/TopoSort, zero-GIL OS threads, `ThreadPool`/`Future`, mutexes, atomics, channels, zero-cost generics |
| **OS, Filesystem & FFI** | `fs`, `sys`, `os`, `path`, `shutil`, `glob`, `tempfile`, `subprocess`, `cli`, `terminal`, `log`, `logging`, `ffi`, `memory`, `mem_guard`, `encoding`, `binary`, `regex`, `compress`, `zlib`, `time`, `datetime`, `uuid` | Crash-safe `fs_atomic_write`, symlink-safe `shutil`/`glob`, `O_NOFOLLOW` `tempfile`, zero-shell `subprocess`, gzip/zlib inflate/deflate, ReDoS-bounded `regex`, 16-arg Win64/SysV C-ABI FFI (`ffi_open`/`ffi_sym`/`ffi_call_arr`) |
| **Web, Networking & gRPC** | `net`, `socket`, `tls`, `http`, `requests`, `fetch`, `url`, `ws`, `web`, `grpc`, `async_io` | TCP/UDP, TLS 1.2/1.3 (WinHTTP Schannel + OpenSSL), HTTP/1.1, `requests` sessions/cookies/multipart, `fetch`/SSE, `:param` router, RFC-4291 IPv6 SSRF guard, WebSockets, gRPC + HTTP/2 + Protobuf v3 |
| **Databases & Storage** | `postgres`, `sqlite`, `mysql`, `redis`, `kv_acid`, `db_pool` | PostgreSQL Wire v3 + `SCRAM-SHA-256`, SQLite3 (`f64`/`blob`/`null`), MySQL v10, Redis RESP2/3 + Pipeline, Embedded CRC32-WAL `KvStore`, `SqlBuilder`, `SqlitePool`, `db_migrate_sqlite` |
| **OS Auth, Crypto & SecTest** | `os_auth`, `auth`, `crypto`, `secrets`, `jwt`, `cap`, `taint`, `shield`, `sec_test`, `sec_fuzz`, `sec_oracle`, `sec_sast`, `sec_corpus`, `sec_meta`, `sec_instr`, `sec_mutate` | Kernel CSPRNG, Win32 DPAPI & Credential Vault, `Argon2id` `v=19`, `BLAKE2b`, `PBKDF2`, `HKDF`, `TOTP`/`HOTP`, `OAuth2 PKCE`, `RbacPolicy`, `tainted[str]` (`E0502`), `RootCap` (`E0501`), `--shield`, Greybox Fuzzer + CmpLog + SAST v2 + SARIF 2.1.0 + Dudect + Mutation Testing |

### Tier 2 — Lightweight Built-Ins & Preview Suites

> Compact, single-file pure-Axon implementations tailored for zero-dependency embedded/scripting tasks rather than full drop-in replacements for external C/Fortran libraries.

| Category | Modules (`use std::<name>`) | Scope |
| :--- | :--- | :--- |
| **Mathematics & Numerical** | `linalg`, `numeric`, `prob`, `bigint`, `math`, `decimal`, `rand` | `MatF64` (LU, Cholesky, QR, Eigen, SVD), `CsrMat` (`CG`/`BiCGSTAB`), `Complex`, Radix-2 `FFT`, `Gauss-Legendre`, `BrentSolver`, `Rk4State`, `LbfgsState`, `Simplex LP`, 8 distributions, `BigInt` & `Rational` |
| **Data Science & ML** | `dataframe`, `stats`, `ml`, `csv`, `json`, `json_dom`, `toml`, `configparser`, `difflib` | Columnar `DataFrame` (CSV/JSONL/Binary IPC `AXDF`, `groupby_agg`, hash `join`), `welch_ttest`, `anova_1way`, `ks_2samp`, `OLS`, `Holt-Winters`, `PCA`, `Ridge`, `Lasso`, `RandomForest`, `GBDT`, `KMeans++`, `DBSCAN`, `ROC-AUC`, `JsonDoc` DOM |
| **Desktop GUI & WebView** | `photon`, `ui`, `gui`, `html`, `css`, `ui_tailwind`, `ui_layout`, `ui_render`, `ui_js`, `ui_signal`, `webview` | Photon-Core Direct2D HiDPI engine (HTML5/CSS3/Tailwind/JS subset, `< 12 MB` RAM) + Edge/Chromium `--app=` WebView host |
| **MCP & Python Compat** | `mcp`, `mcp_python` | JSON-RPC 2.0 MCP SDK & Python AST/Schema reader |
| **AI, LLM, GPU, Games & IoT** | `ai`, `ai_agent`, `ai_autograd`, `ai_engine`, `ai_eval`, `ai_gguf`, `ai_harness`, `agent_toolkit`, `ai_local`, `ai_lora`, `ai_math`, `ai_nn`, `ai_quant`, `ai_serve`, `cuda`, `npu`, `faiss`, `llama`, `vllm`, `transformers`, `onnx`, `safetensors`, `tiktoken`, `torch`, `numpy`, `sklearn`, `quant`, `game`, `game2d`, `game3d`, `game_ecs`, `game_physics`, `game_render`, `math3d`, `audio`, `kernel`, `ring_bridge`, `iot`, `semiconductor` | Pure-Axon tensor/quantization kernels (AVX2+FMA3 Q4/Q8/BitNet, PTX JIT CUDA), hardened `ai_harness` / `agent_toolkit`, SoA `Table[T]` 2D/3D Game Engine & Physics, Ring-0/MMIO/IoT primitives |

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

---

## License

This project is licensed under the **MIT License** — see [`LICENSE`](LICENSE) for details.
