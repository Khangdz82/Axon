# Axon Programming Language (`.ax`) v7.0

Axon is a self-hosted, multi-OS native systems programming language engineered around a single goal: **Python-level readability with bare-metal hardware supremacy across CPU, GPU, WebAssembly, and Unikernels, kilobyte-scale standalone executables, 24/7 zero-leak server stability, and compile-time memory, taint, shape, and anti-crack security**.

The Axon v7.0 compiler (`bin/axc.exe`, **1.27 MB** / `1,333,760 bytes`, alongside cross-compiled `bin/axc-linux-x64` at `1.31 MB` and `bin/axc-macos-arm64` at `608 KB`) is written 100% in Axon (`compiler_ax/`, 17 modules, 19,858 lines) and emits native machine code and standalone executables across **11 hardware/OS targets** directly in memory in milliseconds, without `LLVM`, `Clang`, `GCC`, `MSVC`, `link.exe`, `lld`, or external runtimes.

- **Tiếng Việt:** Xem giáo trình 16 chương đầy đủ tại [`LEARN_AXON_VI.md`](LEARN_AXON_VI.md)
- **English:** Read the complete 16-chapter language guide at [`LEARN_AXON_EN.md`](LEARN_AXON_EN.md)
- **AI Coding Agents:** Ultra-compact grammar & `< 1 ms` feedback loop spec at [`AXON_LLM_SPEC.md`](AXON_LLM_SPEC.md) (`axc check --json`, `axc mcp`)

---

## Core Architecture (v7.0)

1. **11 Direct Native Cross-Compilation Targets (Zero External Linker / Zero Toolchain Setup):**
   - **Desktop & Server OS:** `win-x64` (`PE32+` `.exe` & `--shared` `.dll`/`.pyd`), `linux-x64` (`ELF64` direct `SYSCALL`), `linux-arm64` (`ELF64` AArch64 + `NEON`), `macos-arm64` (`Mach-O 64` 16KB page-aligned Apple Silicon + `NEON`), `macos-x64` (`Mach-O 64`).
   - **Web, Cloud & Bare-Metal:** `wasm32` (`WASI` + `0xFD` `SIMD128`), `unikernel-x64` (`Multiboot2` Ring-0 `12.25 KB` image + `COM1 UART`), `freestanding-x64`, `freestanding-riscv64` (`RV64GC` `EM_RISCV=243`).
   - **Direct GPU Offloading (`--emit-gpu`):** `spirv` (binary Khronos `SPIR-V 1.6` `.spv`) and `ptx` (NVIDIA `PTX 8.0` `.ptx`) directly from `@gpu_kernel` / `@shader` functions.
2. **Compile-Time Execution (`comptime`) & Direct C Header Reader (`@cImport` + Static COFF `.obj`/`.lib` Linker):**
   - Embedded `comptime` AST VM (`compiler_ax/comptime.ax`) runs real Axon functions and loops during semantic analysis to synthesize zero-allocation `.rdata` lookup tables, validate SQL/regexes via `comptime_assert`, and generate zero-reflection `to_json(struct_val)`.
   - `@cImport("header.h", link = "lib.dll")` parses C `#define` constants, `typedef struct` layouts, and function prototypes directly without `libclang`; `--link-lib foo.obj` statically links Win64 COFF `.obj` and `.lib` archives directly into `PE32+`.
3. **Runtime CPUID Multi-Versioning + Multi-Core `par_for` + OS-Native `async/await` (`IOCP`):**
   - Startup `CPUID` + `XGETBV` dispatches `Table[T]` SoA compute kernels across **Tier 0 (`SSE2` scalar safe on every CPU)**, **Tier 1 (`AVX2 + FMA3` 256-bit)**, and **Tier 2 (`AVX-512 EVEX` 512-bit)** (`--simd=scalar|avx2|avx512`).
   - `par_for p in table` partitions columnar blocks across all hardware CPU cores with compile-time data-race rejection (`E0204`), paired with stackless `async`/`await` coroutines on Windows `IOCP` (`std/async_io.ax`).
4. **All 6 AI/LLM Systems Breakthroughs + 3D Game + HFT + Taint Security (`std/`):**
   - **AI/LLM (`std/ai_engine.ax`):** `VramArena`, vLLM-style non-contiguous `PagedKvCache`, Symbolic Shape Polymorphism (`fn rmsnorm_2d[sym B, sym S](...)` compiled once with `E0602` shape verification), schedule-separated `@gpu_kernel` (`@tile`, `@pipeline_stages`), SPMD `ShardedTensor` + linear `all_reduce_sum(move shard)`, Zero-Copy Python `DLPack` `--shared` `@py_export`, and hardware `@nan_guard` (`0x7FF` exponent trap with exact line number + bit-exact pairwise `det_sum_f32_ptr`).
   - **3D Game (`std/math3d.ax`), HFT (`std/decimal.ax`, `std/ringbuf.ax`, `@no_alloc` `E0601`), & Security (`std/taint.ax`, `tainted[str]` `E0502`, Polymorphic `@shield` v2, `@whitebox` $\text{GF}(2^8)$).**
5. **Integrated Package Manager (`axpm`) & AI-Native Tooling (`axc mcp` + `axc check --json`):**
   - Single binary provides `axc init`, `build`, `run`, `test`, `check --json` (`~440 us` structured diagnostics with `file:line:col` carets and hints), `targets`, and `mcp` (JSON-RPC 2.0 stdio server for AI coding agents) plus `axon.lock` FNV-64 integrity locking.

---

## Quickstart

```powershell
# Install toolchain to ~/.axon
.\install.ps1

# Sub-millisecond static verification (human-readable or AI JSON)
.\bin\axc.exe check examples/hello.ax
.\bin\axc.exe check --json tests/pass/16_v7_ai_llm_supremacy.ax

# Compile & run standalone native executables, unit tests, and cross-OS binaries
.\bin\axc.exe build examples/hello.ax -o bin/hello.exe
.\bin\axc.exe run tests/pass/16_v7_ai_llm_supremacy.ax
.\bin\axc.exe test tests/pass/17_v7_unit_tests.ax

# One-command cross-compilation (zero Docker, zero external SDKs)
.\bin\axc.exe build examples/hello.ax --target linux-x64 -o bin/hello-linux
.\bin\axc.exe build examples/hello.ax --target macos-arm64 -o bin/hello-macos
.\bin\axc.exe build examples/hello.ax --target wasm32 -o bin/hello.wasm
.\bin\axc.exe build examples/hello.ax --target unikernel-x64 -o bin/unikernel.elf
```

---

## Verified Physical Benchmarks (Measured Live on Host)

Hardware: **Intel Core i5-10400F (6C/12T), 32 GB DDR4, Windows 10 x64**.

### 1. Toolchain & Standalone Binary Footprint

| Language & Toolchain | Installed Toolchain Size | Clean Check / Build | Standalone `hello` Binary | Cross-Compile 11 Targets Built-In |
| :--- | :---: | :---: | :---: | :---: |
| **Axon v7.0 (`bin/axc.exe` Self-Hosted)** | **~1.30 MB** | **< 1 ms check / 9 ms build** | **5.50 KB (`5,632 B`)** | **Yes (0 external tools)** |
| **C++23 (`MSVC 14.51 /O2 /arch:AVX2`)** | ~2,500 MB | ~1,660 ms | 108.50 KB (`111,104 B`) | No |
| **Rust (`rustc -O3 -C target-cpu=native -C strip=symbols`)** | ~1,200 MB | 157.8 ms | 127.00 KB (`130,048 B`) | Requires target toolchains/linkers |
| **Go (`go build -ldflags="-s -w"`)** | ~500 MB | 452.7 ms | 1,636.00 KB (`1,675,264 B`) | OS/CPU only (no GPU/Unikernel) |
| **Java 21 (`OpenJDK 21.0.13`)** | ~320 MB | ~620 ms | `.class` + ~320 MB JRE | Requires JVM |
| **Python 3.11** | ~100 MB | Interpreted | Script + ~100 MB Runtime | Requires Python VM |

### 2. Workload A: Entity Simulation (`examples/bench_soa_simd.ax`)
**1,048,576 64-byte entities (`16 x f32`) x 200 steps** (`209,715,200` entity updates = `838.86M` FLOPs, verified output `px = 26.00, py = 52.00`):

| Language & Compiler | Standalone Binary | Execution Time (Best of 3) | Axon Speedup |
| :--- | :---: | :---: | :---: |
| **Axon (`axc.exe`, SoA + Dead-Col Elim + Temporal SIMD)** | **5.50 KB (`5,632 B`)** | **8.61 ms** (`8,612 us`) | **1.00x (Baseline)** |
| **Rust (`rustc -O3 -C target-cpu=native -C strip=symbols`)** | 155.50 KB (`159,232 B`) | 1,195.50 ms (`1,195,499 us`) | **138.8x faster** |
| **C++23 (`MSVC cl /O2 /arch:AVX2 /fp:fast`)** | 139.00 KB (`142,336 B`) | 1,207.17 ms (`1,207,169 us`) | **140.2x faster** |
| **Go (`go build -ldflags="-s -w"`)** | 1,650.50 KB (`1,690,112 B`) | 1,211.72 ms (`1,211,719 us`) | **140.7x faster** |
| **Java 21 (`OpenJDK 21.0.13 HotSpot C2 JIT`)** | `.class` + ~320 MB JRE | 1,669.26 ms (`1,669,263 us`) | **193.8x faster** |
| **Python 3.11 (`__slots__`)** | ~100 MB Runtime | 32,867.40 ms (scaled from 5 steps) | **3,816.5x faster** |

### 3. Workload B: Enterprise Packet Buffer + Financial Settlement (`examples/bench_enterprise_suite.ax`)
**5,000,000 per-request `4 KB` packet buffers (`20 GB` logical heap traffic) + exact fixed-point tax/fee ledger settlement** (verified output `$448,750,000.00`):

| Language & Compiler | Memory / Decimal Model | Execution Time (Best of 3) | Axon Speedup |
| :--- | :---: | :---: | :---: |
| **Axon (`axc.exe`, Perceus Loop Self-FBIP + `dec`)** | **0 GC, 0 Syscall Reuse + 64-bit `dec`** | **50.81 ms** (`50,808 us`) | **1.00x (Baseline)** |
| **Rust (`rustc -O3 -C target-cpu=native`)** | Heap `Vec::with_capacity(4096)` + `i64` | 254.90 ms (`254,904 us`) | **5.02x faster** |
| **C++23 (`MSVC cl /O2 /arch:AVX2`)** | `std::vector<uint8_t>(4096)` + `int64_t` | 279.09 ms (`279,089 us`) | **5.49x faster** |
| **Java 21 (`OpenJDK 21 HotSpot C2 JIT`)** | GC `byte[4096]` + `BigDecimal` | 2,150.50 ms (`2,150,500 us`) | **42.33x faster** |
| **Go (`go build -ldflags="-s -w"`)** | GC `make([]byte, 0, 4096)` + `int64` | 3,482.53 ms (`3,482,531 us`) | **68.54x faster** |
| **Python 3.11** | `bytearray` + `decimal.Decimal` | 4,017.44 ms (scaled from 250k) | **79.07x faster** |

---

## Repository Structure

```text
├── bin/
│   ├── axc.exe                 # Self-hosted Axon v7.0 compiler for Windows x64 (1.27 MB)
│   ├── axc-linux-x64           # Cross-compiled standalone Axon v7.0 compiler for Linux x86_64 (1.31 MB)
│   └── axc-macos-arm64         # Cross-compiled standalone Axon v7.0 compiler for macOS Apple Silicon (608 KB)
├── compiler_ax/                # 100% pure Axon self-hosted compiler source (17 modules, 19,848 lines)
│   ├── tokens_ast.ax           # Token kinds, packed Ty, string pool & AST arena
│   ├── diagnostics.ax          # Source line:col spans, caret renderer & <1ms `--json` diagnostics
│   ├── lexer.ax                # Byte-level lexer (f-strings, dec, IEEE-754, attributes, v7 keywords)
│   ├── parser.ax               # Recursive-descent & Pratt parser (comptime, @cImport, symbolic shapes, par_for)
│   ├── c_import.ax             # Direct C `.h` header parser (#define, typedef struct, prototypes)
│   ├── comptime.ax             # Embedded compile-time AST VM (.rdata LUTs & comptime_assert)
│   ├── sema_effects.ax         # Symbolic Tensor shape unifier (E0602), Taint (E0502), @no_alloc (E0601), Race (E0204)
│   ├── sema_perceus.ax         # Exhaustive Perceus ownership verifier, Loop Self-FBIP & SIMD planner
│   ├── linker_pe.ax            # Win64 PE32+ (.exe/.dll/.pyd, Multi-DLL .idata, .edata) & Static COFF .obj/.lib linker
│   ├── linker_unix_bare.ax     # Linux ELF64 (x64/ARM64/RV64), macOS Mach-O 64 (16KB), & Multiboot2 Unikernel linker
│   ├── codegen_runtime.ax      # Zero-CRT OS runtime (Win64/Linux/macOS/UART), CPUID+XGETBV, @shield v2, @nan_guard
│   ├── codegen_calls.ax        # Intrinsics, 128-bit XMM Math3D, DLPack, IOCP, Atomics, `to_json` & Win64 C-ABI FFI
│   ├── codegen_x86_64.ax       # Multi-versioned Scalar/AVX2/AVX-512 x86_64 + OS Thread Pool `par_for`
│   ├── codegen_arm64_rv64.ax   # Native AArch64 + 128-bit NEON & Bare-metal RISC-V 64 (`RV64GC`) backends
│   ├── backend_gpu_wasm.ax     # Binary Khronos SPIR-V 1.6 (.spv), NVIDIA PTX 8.0 (.ptx) & WASM32 + SIMD128 + WASI
│   ├── pkg_mcp.ax              # Integrated `axpm` package manager (`axon.toml`/`axon.lock`) & JSON-RPC 2.0 MCP server
│   └── main.ax                 # Multi-OS CLI driver (`build`, `check`, `run`, `test`, `init`, `targets`, `mcp`)
├── std/                        # 12 pure Axon stdlib modules (ai_engine, async_io, math3d, decimal, ringbuf, taint, cap, crdt, crypto, collections, json, os)
├── tests/                      # 18 native execution pass tests + 22 compile-time safety rejection gates
├── AXON_LLM_SPEC.md            # Compact grammar & error reference for AI Coding Agents
├── LEARN_AXON_VI.md            # Official Vietnamese learning guide (v7.0, 16 chapters)
└── LEARN_AXON_EN.md            # Official English learning guide (v7.0, 16 chapters)
```

---

## Verifying Deterministic 3-Stage Self-Hosting

```powershell
.\bin\axc.exe build compiler_ax/main.ax -o axc_stage1.exe
.\axc_stage1.exe build compiler_ax/main.ax -o axc_stage2.exe
.\axc_stage2.exe build compiler_ax/main.ax -o axc_stage3.exe
Get-FileHash axc_stage2.exe, axc_stage3.exe -Algorithm SHA256
# Verified SHA256: F06DFC7F4DF44F9905E1B8F38A169D4506059F99C837F06EF766FF93A0F976F9
```
