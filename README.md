# Axon Programming Language (`.ax`) v1.0.0

Axon is a self-hosted, multi-OS native systems programming language engineered around **Python-level readability, bare-metal speed across CPU, GPU, WebAssembly, and Unikernels, sub-1 MB compiler binaries, zero-GC predictability via Perceus ownership, full-stack stdlib modules (`AI/ML`, `2D/3D Game`, `Web/HTTP`, `Crypto`, `Math`), and fail-closed compile-time & runtime security**.

The Axon `v1.0.0` compiler (`bin/axc.exe`, **973.50 KB** / `996,864 bytes` built with `--shield`, alongside `bin/axc-linux-x64` at **928.67 KB** / `950,960 bytes` and `bin/axc-macos-arm64` at **720.06 KB** / `737,344 bytes` — every standalone compiler binary is strictly **under 1 MB**) is written 100% in Axon (`compiler_ax/`, `17` modules, `23,763` lines) and emits native machine code and standalone executables across **11 hardware/OS targets** directly in memory without `LLVM`, `Clang`, `GCC`, `MSVC`, `link.exe`, `lld`, or external runtimes.

- **Interactive Browser Playground (Zero Install):** Open [`playground/index.html`](playground/index.html) directly in any browser — compiles live Axon code to W3C WebAssembly + `0xFD SIMD128` + WASI in `< 1 ms` (~`1.2–2.1 KB` `.wasm`), with live Canvas2D + binary hex/opcode inspector.
- **Rigorous 10-Suite Benchmark Report:** See [`docs/BENCHMARK_REPORT.md`](docs/BENCHMARK_REPORT.md) for methodology, checksums, RSS, CV%, and candid engineering analysis vs MSVC C++23, Rust 1.100, Go 1.26.4, OpenJDK 21, Node V8 WASM, and Python 3.11.
- **Tiếng Việt:** Xem giáo trình 16 chương đầy đủ tại [`LEARN_AXON_VI.md`](LEARN_AXON_VI.md)
- **English:** Read the complete 16-chapter language guide at [`LEARN_AXON_EN.md`](LEARN_AXON_EN.md)
- **AI Coding Agents:** Ultra-compact grammar & sub-millisecond feedback spec at [`AXON_LLM_SPEC.md`](AXON_LLM_SPEC.md) (`axc check --json`, `axc mcp`)

---

## Core Architecture (`v1.0.0`)

1. **11 Direct Native Cross-Compilation Targets (Zero External Linker / Zero SDK):**
   - **Desktop & Server OS:** `win-x64` (`PE32+` `.exe` & `--shared` `.dll`/`.pyd`), `linux-x64` (`ELF64` direct `SYSCALL`), `linux-arm64` (`ELF64` AArch64 + `NEON`), `macos-arm64` (`Mach-O 64` 16KB page-aligned Apple Silicon + `NEON`), `macos-x64` (`Mach-O 64`).
   - **Web, Cloud & Bare-Metal:** `wasm32` (`W3C WASI preview1` + `0xFD` `SIMD128` + dynamic `memory.grow` — compiles & executes standalone without emscripten/wasmtime SDKs), `unikernel-x64` (`Multiboot2` Ring-0 + `COM1 UART`), `freestanding-x64`, `freestanding-riscv64` (`RV64GC` `EM_RISCV=243`).
   - **Direct GPU Offloading (`--emit-gpu`):** `spirv` (binary Khronos `SPIR-V 1.6` `.spv`) and `ptx` (NVIDIA `PTX 8.0` `.ptx`).
2. **Browser-Native Zero-Dependency Playground (`playground/index.html`):**
   - Standalone static HTML/JS IDE running a real recursive-descent Axon -> `WASM32 + 0xFD SIMD128 + WASI` compiler directly in the browser.
   - Includes **1M SIMD Particles**, **`T ! Error` Union Handling**, **Small AI Inference (Linear + ReLU + Softmax)**, and **Mini 2D Game** interactive simulations, compilation/runtime/hex metrics, and `.wasm` download.
3. **Compile-Time Execution (`comptime`) & Direct C Header Reader (`@cImport` + Static COFF `.obj`/`.lib` Linker):**
   - Embedded `comptime` AST VM (`compiler_ax/comptime.ax`) runs real Axon code at compile time to synthesize zero-alloc `.rdata` tables, enforce `comptime_assert`, and emit zero-reflection `to_json(struct_val)`.
   - `@cImport("header.h", link = "lib.dll")` parses C `#define` constants, `typedef struct` layouts, and function prototypes directly without `libclang`; `--link-lib foo.obj` statically links Win64 COFF `.obj` and `.lib` archives.
4. **Runtime CPUID Multi-Versioning + Multi-Core `par_for` + OS-Native `async/await` (`IOCP`):**
   - Startup `CPUID` + `XGETBV` dispatches `Table[T]` SoA kernels across **Tier 0 (`SSE2`)**, **Tier 1 (`AVX2 + FMA3`)**, and **Tier 2 (`AVX-512 EVEX`)**.
   - `par_for p in table` partitions columnar blocks across hardware cores with compile-time data-race rejection (`E0204`), alongside stackless `async`/`await` coroutines (`std/async_io.ax`).
5. **Production Batteries-Included Stdlib (`34` Pure Axon Modules, `9,710` Lines, Tree-Shaken):**
   - **New in `v1.0.0` (Portable Native & WASM32 + Native CLI):**
     - `std/math.ax` — Overflow-safe integer math (`min`/`max`/`clamp`/`abs`/`gcd`/`lcm`/`mod_pow`/`isqrt`), fixed-point `1e6` helpers, geometry/lerp/easing, fast vector/scalar math.
     - `std/rand.ax` — Deterministic SplitMix64 + Xoshiro128++ PRNG (`rng_new`, `rng_next_u64`, `rng_range_i64`, `rng_next_f64`, `rng_free`).
     - `std/path.ax` — POSIX/Windows normalization (`path_join`, `path_dirname`, `path_basename`, `path_ext`) and CWE-22 traversal guards (`path_is_safe_rel`).
     - `std/encoding.ax` — Portable Hex, Base64/Base64URL, CRC32, FNV-1a 64-bit, HTML entity escaping (worst-case `&quot;` safe), URL percent encoding/decoding.
     - `std/log.ax` — Structured JSON logger with CWE-117 CRLF/control-byte sanitization (`log_sanitize` capacity-safe on worst-case escapes).
     - `std/time.ax` — Portable nanosecond monotonic `Stopwatch` (`sw_start`, `sw_elapsed_ns`/`us`/`ms`, `sw_lap`) across Native and WASI `clock_time_get`.
     - `std/cli.ax` — Typed flag/option extraction (`cli_get_i64`, `cli_require_opt` returning `str ! i64`, positional arguments).
   - **Core, Web & Data:** `std/str.ax`, `std/http.ax` (with CWE-22 backslash/`..`/`%2e`/`%5c` traversal & CRLF/NUL guards), `std/data.ax`, `std/test_kit.ax`, `std/json.ax`, `std/collections.ax`, `std/os.ax`, `std/decimal.ax`, `std/ringbuf.ax`.
   - **AI/ML/DL/LoRA/Serve (`9` modules):** `std/ai.ax`, `std/ai_math.ax`, `std/ai_nn.ax`, `std/ai_autograd.ax`, `std/ai_quant.ax` (Q4/Q8/NF4/FP16/BF16), `std/ai_lora.ax` (LoRA/DoRA/QLoRA), `std/ai_local.ax` (GGUF/AXM/BPE), `std/ai_serve.ax`, `std/ai_engine.ax` (`VramArena`, `PagedKvCache`, Symbolic Shapes `E0602`, `DLPack`).
   - **2D & 3D Game Engine:** `std/game.ax`, `std/game2d.ax`, `std/game3d.ax`, `std/math3d.ax` (AABB/Circle/SAT physics, sprites/tilemaps/A*/particles, quaternion skinning, BVH ray-triangle, PBR + CSM shadows, ECS).
6. **Hardened `v1.0.0` Compile-Time & Runtime Security:**
   - Wrapping `INT64_MIN` constant negation fix (`0 -% v` in `sema_perceus.ax`, `comptime.ax`, `c_import.ax`) under `--shield` self-protection.
   - Fail-closed hardware/bounds traps (`exit 101`) for integer divide-by-zero, `INT64_MIN / -1`, checked `--shield` signed overflow, and `ByteBuf`/`str_byte` out-of-bounds indexing.
   - Compile-time checks for `tainted[str]` SQL & OS shell sinks (`E0502`), capability attenuation (`E0501`), Perceus linear ownership (`E0201`–`E0204`), parser depth DoS (`E0020`, cap `256`), CWE-158 embedded `NUL` FS/exec rejection, NIST FIPS 180-4 SHA-256 + RFC 2104 HMAC + `ct_eq_str`, Polymorphic `@shield` v2, and `@whitebox` $\text{GF}(2^8)$.

---

## Quickstart

```powershell
# Install Axon v1.0.0 compiler + stdlib to ~/.axon
.\install.ps1
axc version

# Sub-millisecond static verification (human-readable or AI JSON)
.\bin\axc.exe check examples/hello.ax
.\bin\axc.exe check --json tests/pass/16_v7_ai_llm_supremacy.ax

# Compile & run native binaries, unit tests, and cross-OS/WebAssembly binaries
.\bin\axc.exe build examples/hello.ax -o bin/hello.exe
.\bin\axc.exe run tests/pass/26_v1_stdlib_production.ax
.\bin\axc.exe test tests/pass/17_v7_unit_tests.ax

# One-command cross-compilation (zero Docker, zero external SDKs)
.\bin\axc.exe build examples/hello.ax --target linux-x64 -o bin/hello-linux
.\bin\axc.exe build examples/hello.ax --target macos-arm64 -o bin/hello-macos
.\bin\axc.exe build tests/pass/27_wasm32_portable_suite.ax --target wasm32 -o bin/portable.wasm
.\bin\axc.exe build examples/hello.ax --target unikernel-x64 -o bin/unikernel.elf
```

---

## Verified Benchmark Highlights (`B1 – B10`, Measured on Host)

Hardware: **Intel Core i5-10400F (6C/12T), 32 GB DDR4, Windows 10 x64**. Full tables, min/p95/CV%, RSS, checksums, and caveats are documented in [`docs/BENCHMARK_REPORT.md`](docs/BENCHMARK_REPORT.md).

| Benchmark | Workload | Axon `win-x64` | Axon `wasm32` | C++23 MSVC | Rust 1.100 | Go 1.26.4 | Java 21 | Python 3.11 |
| :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| **B1b** | Vector `f32` Dot (`409.6M` FMAs) | **`15.67 ms`** | — | `25.57 ms` | `49.69 ms` | `411.77 ms` | `409.95 ms` | `24,186.91 ms` |
| **B2** | Coupled Particle SoA (`32.8M` updates) | **`5.11 ms`** (`Table`) | `10.58 ms` (`2.9 KB`) | `4.10 ms` | `5.60 ms` | `50.00 ms` | `7.28 ms` | `22,329.02 ms` |
| **B3** | Dense FP32 GEMM `512x512` | `12.97 ms` | — | `10.52 ms` | `9.79 ms` | `62.57 ms` | `61.68 ms` | `12,689.96 ms` |
| **B7** | Dynamic `4 KB` Frame Churn (`500K`) | `94.67 ms` (`3.0 MB` RSS) | — | `24.75 ms` | `25.74 ms` | `334.17 ms` | `163.36 ms` | `390.25 ms` |
| **B8 (1T)** | `Table[Agent]` Compute (`134.2M` updates) | `173.80 ms` | — | `172.89 ms` | `174.43 ms` | `285.17 ms` | `178.02 ms` | — |
| **B10 Build** | Cold Clean Build Time (`B9` App) | **`15.90 ms`** (`34.0 KB`) | **`15.94 ms`** (`14.1 KB`) | `1,602.68 ms` (`169 KB`) | `431.70 ms` (`163 KB`) | `303.62 ms` (`1.63 MB`) | `519.29 ms` (bytecode) | Interpreted |

- `examples/hello.ax` checks in **`565 us`** (internal verifier) and builds a standalone `PE32+` executable of **`5,120 bytes` (`5.00 KB`)** in **`1 ms`**.

---

## Repository Structure

```text
├── bin/
│   ├── axc.exe                 # Self-hosted Axon v1.0.0 compiler for Windows x64 (973.50 KB / 996,864 B, --shield)
│   ├── axc-linux-x64           # Cross-compiled standalone Axon v1.0.0 compiler for Linux x86_64 (928.67 KB / 950,960 B)
│   └── axc-macos-arm64         # Cross-compiled standalone Axon v1.0.0 compiler for macOS Apple Silicon (720.06 KB / 737,344 B)
├── playground/
│   └── index.html              # Zero-dependency browser Axon -> WASM32+SIMD128 IDE & runner (< 1 ms, ~1.2-2.1 KB)
├── compiler_ax/                # 100% pure Axon self-hosted compiler source (17 modules, 23,763 lines)
├── std/                        # 34 pure Axon stdlib modules (9,710 lines: math, rand, path, encoding, log, time, cli, str, http, data, test_kit, ai*, game*, math3d, async_io, cap, crypto, taint, ...)
├── tests/
│   ├── pass/                   # 27 end-to-end native execution & stdlib/security verification programs
│   ├── fail_compile/           # 25 static compile-time safety rejection gates (Perceus, taint, cap, shape, @no_alloc, depth)
│   └── wasm/                   # WASM32 dynamic growth & Node.js WASI preview1 runner (`wasi_run.mjs`)
├── docs/
│   └── BENCHMARK_REPORT.md     # Honest, reproducible B1-B10 benchmark report & engineering notes
├── AXON_LLM_SPEC.md            # Compact grammar & error reference for AI Coding Agents
├── LEARN_AXON_VI.md            # Official Vietnamese learning guide (16 chapters)
└── LEARN_AXON_EN.md            # Official English learning guide (16 chapters)
```

---

## Verifying Deterministic 3-Stage Self-Hosting

```powershell
.\bin\axc.exe build compiler_ax/main.ax --shield -o axc_stage1.exe
.\axc_stage1.exe build compiler_ax/main.ax --shield -o axc_stage2.exe
.\axc_stage2.exe build compiler_ax/main.ax --shield -o axc_stage3.exe
Get-FileHash axc_stage2.exe, axc_stage3.exe -Algorithm SHA256
# Verified v1.0.0 SHA256: D9B85D3CB1FE4E66C4198DC1BF2FC37CC9168B4169FA3669B6AAC340C2984677
```
