# Axon Programming Language (`.ax`) v0.9.1

Axon is a self-hosted, multi-OS native systems programming language engineered around a single goal: **Python-level readability with bare-metal hardware supremacy across CPU, GPU, WebAssembly, and Unikernels, kilobyte-scale compilers and executables, 24/7 zero-leak server stability, high-performance zero-GC AI/LLM & 2D/3D Game engines, and compile-time memory, taint, shape, and anti-crack security**.

The Axon v0.9.1 compiler (`bin/axc.exe`, **838.50 KB** / `858,624 bytes`, alongside cross-compiled `bin/axc-linux-x64` at **896.67 KB** / `918,192 bytes` and `bin/axc-macos-arm64` at **640.06 KB** / `655,424 bytes` — every standalone compiler binary is strictly **under 1 MB**) is written 100% in Axon (`compiler_ax/`, 17 modules, `20,705` lines) and emits native machine code and standalone executables across **11 hardware/OS targets** directly in memory in milliseconds, without `LLVM`, `Clang`, `GCC`, `MSVC`, `link.exe`, `lld`, or external runtimes.

- **Tiếng Việt:** Xem giáo trình 16 chương đầy đủ tại [`LEARN_AXON_VI.md`](LEARN_AXON_VI.md)
- **English:** Read the complete 16-chapter language guide at [`LEARN_AXON_EN.md`](LEARN_AXON_EN.md)
- **AI Coding Agents:** Ultra-compact grammar & `< 1 ms` feedback loop spec at [`AXON_LLM_SPEC.md`](AXON_LLM_SPEC.md) (`axc check --json`, `axc mcp`)

---

## Core Architecture (v0.9.1)

1. **11 Direct Native Cross-Compilation Targets (Zero External Linker / Zero Toolchain Setup):**
   - **Desktop & Server OS:** `win-x64` (`PE32+` `.exe` & `--shared` `.dll`/`.pyd`), `linux-x64` (`ELF64` direct `SYSCALL`), `linux-arm64` (`ELF64` AArch64 + `NEON`), `macos-arm64` (`Mach-O 64` 16KB page-aligned Apple Silicon + `NEON`), `macos-x64` (`Mach-O 64`).
   - **Web, Cloud & Bare-Metal:** `wasm32` (`WASI` + `0xFD` `SIMD128`), `unikernel-x64` (`Multiboot2` Ring-0 image + `COM1 UART`), `freestanding-x64`, `freestanding-riscv64` (`RV64GC` `EM_RISCV=243`).
   - **Direct GPU Offloading (`--emit-gpu`):** `spirv` (binary Khronos `SPIR-V 1.6` `.spv`) and `ptx` (NVIDIA `PTX 8.0` `.ptx`) directly from `@gpu_kernel` / `@shader` functions.
2. **Compile-Time Execution (`comptime`) & Direct C Header Reader (`@cImport` + Static COFF `.obj`/`.lib` Linker):**
   - Embedded `comptime` AST VM (`compiler_ax/comptime.ax`) runs real Axon functions and loops during semantic analysis to synthesize zero-allocation `.rdata` lookup tables, validate SQL/regexes via `comptime_assert`, and generate zero-reflection `to_json(struct_val)`.
   - `@cImport("header.h", link = "lib.dll")` parses C `#define` constants, `typedef struct` layouts, and function prototypes directly without `libclang`; `--link-lib foo.obj` statically links Win64 COFF `.obj` and `.lib` archives directly into `PE32+`.
3. **Runtime CPUID Multi-Versioning + Multi-Core `par_for` + OS-Native `async/await` (`IOCP`):**
   - Startup `CPUID` + `XGETBV` dispatches `Table[T]` SoA compute kernels across **Tier 0 (`SSE2` scalar safe on every CPU)**, **Tier 1 (`AVX2 + FMA3` 256-bit)**, and **Tier 2 (`AVX-512 EVEX` 512-bit)** (`--simd=scalar|avx2|avx512`).
   - `par_for p in table` partitions columnar blocks across all hardware CPU cores with compile-time data-race rejection (`E0204`), paired with stackless `async`/`await` coroutines on Windows `IOCP` (`std/async_io.ax`).
4. **Full-Stack Batteries-Included Stdlib (`27` Modules, Tree-Shaken):**
   - **Everyday & Backend Kit (`std/str.ax`, `std/http.ax`, `std/data.ax`, `std/test_kit.ax`, `std/json.ax`, `std/collections.ax`, `std/os.ax`):** Complete string ops (search/trim/case/split/replace/hex/Base64/URL), RFC-8259 JSON escaping, HTTP/1.1 request/response/route-param/SSE builder with CRLF injection guards, CSV, CLI flags, structured JSON logging, persisted `KvStore`, and zero-dependency assertion/microbenchmark suite. Reachability tree-shaking (`mark_reachable_fns_ax`) keeps binaries at `5.00 KB` when unused.
   - **AI/ML/DL/LoRA/Local/Serve (`std/ai*`):** Zero-GC `@no_alloc` / `@nan_guard` tiled FlashAttention, RMSNorm, SwiGLU, RoPE, AdamW/Lion autograd, Q4/Q8/NF4/BF16/FP16 quantization, LoRA/DoRA/QLoRA fine-tuning & checkpoint merging, Transformer GQA + MoE, K-Means/PCA/Linear/Logistic ML, GGUF/AXM loader + BPE tokenizer, KV-cached `std/ai_serve.ax`, `VramArena`, `PagedKvCache`, Symbolic Shapes (`E0602`), SPMD `ShardedTensor`, and Zero-Copy `DLPack`.
   - **2D & 3D Game Engine (`std/game.ax`, `std/game2d.ax`, `std/game3d.ax`, `std/math3d.ax`):** AABB/Circle/SAT physics, spatial hash broadphase, 2D camera/sprites/A*/particles; 3D quaternion SLERP skinning, frustum culling, Möller-Trumbore ray-triangle + BVH, PBR + CSM shadow maps, and generational ECS.
5. **v0.9.1 Compiler Speed, Size & Production Security Hardening:**
   - **$O(1)$ Compiler Speedups & Smaller Binaries:** Hash-indexed symbol lookups, zero-copy `nk_field`/`nk_tuple_idx` projections, scoped block slot reuse, direct immediate `nk_assign`, and recursive shadowed-`comptime` guard (`is_nonshadowed_const_int`) slimmed `bin/axc.exe` by another `-27 KB` down to **838.50 KB** (`858,624 B`), checking `examples/hello.ax` in **346-354 us** and a 6,000-line file in **25.44 ms**.
   - **Security Hardening:** CWE-158 embedded NUL-byte rejection in `fs_read_file`/`fs_write_file`/`os_exec`, dynamic auto-growing `h_jbuf_*` (`f"..."` & `to_json` >4KB without truncation/overflow) + RFC-8259 JSON escaping, parser recursion-depth DoS cap (`E0020`, cap `256`), hardened `std/cap.ax` (`< 32`, `DEL`, `:`, trailing `.`/` `), and `std/crypto.ax` NIST FIPS 180-4 SHA-256 + RFC 2104 HMAC-SHA256 + constant-time `ct_eq_str`.
   - Plus `std/decimal.ax`, `std/ringbuf.ax`, `@no_alloc` (`E0601`), `std/taint.ax` (`tainted[str]` `E0502`), Polymorphic `@shield` v2, and `@whitebox` $\text{GF}(2^8)$.
6. **Integrated `axpm` & `axc mcp`:** Single `< 1 MB` binary provides `axc init`, `build`, `run`, `test`, `check --json`, `targets`, `mcp`, and `axon.lock` FNV-64 integrity locking.

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
| **Axon v0.9.1 (`bin/axc.exe` Self-Hosted)** | **838.50 KB** (`858,624 B`) | **354 us check / 1 ms hello build** | **5.00 KB (`5,120 B`)** | **Yes (0 external tools)** |
| **C++23 (`MSVC 14.51 /O2 /arch:AVX2`)** | ~2,500 MB | ~1,660 ms | 108.50 KB (`111,104 B`) | No |
| **Rust (`rustc -O3 -C target-cpu=native -C strip=symbols`)** | ~1,200 MB | 157.8 ms | 127.00 KB (`130,048 B`) | Requires target toolchains/linkers |
| **Go (`go build -ldflags="-s -w"`)** | ~500 MB | 452.7 ms | 1,636.00 KB (`1,675,264 B`) | OS/CPU only (no GPU/Unikernel) |
| **Java 21 (`OpenJDK 21.0.13`)** | ~320 MB | ~620 ms | `.class` + ~320 MB JRE | Requires JVM |
| **Python 3.11** | ~100 MB | Interpreted | Script + ~100 MB Runtime | Requires Python VM |

### 2. Workload A: Entity Simulation (`examples/bench_soa_simd.ax`)
**1,048,576 64-byte entities (`16 x f32`) x 200 steps** (`209,715,200` entity updates = `838.86M` FLOPs, verified output `px = 26.00, py = 52.00`):

| Language & Compiler | Standalone Binary | Execution Time (Best of 3) | Axon Speedup |
| :--- | :---: | :---: | :---: |
| **Axon (`axc.exe`, SoA + Dead-Col Elim + Temporal SIMD)** | **5.50 KB (`5,632 B`)** | **8.60 ms** (`8,599 us`) | **1.00x (Baseline)** |
| **Rust (`rustc -O3 -C target-cpu=native -C strip=symbols`)** | 155.50 KB (`159,232 B`) | 1,195.50 ms (`1,195,499 us`) | **138.8x faster** |
| **C++23 (`MSVC cl /O2 /arch:AVX2 /fp:fast`)** | 139.00 KB (`142,336 B`) | 1,207.17 ms (`1,207,169 us`) | **140.2x faster** |
| **Go (`go build -ldflags="-s -w"`)** | 1,650.50 KB (`1,690,112 B`) | 1,211.72 ms (`1,211,719 us`) | **140.7x faster** |
| **Java 21 (`OpenJDK 21.0.13 HotSpot C2 JIT`)** | `.class` + ~320 MB JRE | 1,669.26 ms (`1,669,263 us`) | **193.8x faster** |
| **Python 3.11 (`__slots__`)** | ~100 MB Runtime | 32,867.40 ms (scaled from 5 steps) | **3,816.5x faster** |

### 3. Workload B: Enterprise Packet Buffer + Financial Settlement (`examples/bench_enterprise_suite.ax`)
**5,000,000 per-request `4 KB` packet buffers (`20 GB` logical heap traffic) + exact fixed-point tax/fee ledger settlement** (verified output `$448,750,000.00`):

| Language & Compiler | Memory / Decimal Model | Execution Time (Best of 3) | Axon Speedup |
| :--- | :---: | :---: | :---: |
| **Axon (`axc.exe`, Perceus Loop Self-FBIP + `dec`)** | **0 GC, 0 Syscall Reuse + 64-bit `dec`** | **47.37 ms** (`47,368 us`) | **1.00x (Baseline)** |
| **Rust (`rustc -O3 -C target-cpu=native`)** | Heap `Vec::with_capacity(4096)` + `i64` | 254.90 ms (`254,904 us`) | **5.02x faster** |
| **C++23 (`MSVC cl /O2 /arch:AVX2`)** | `std::vector<uint8_t>(4096)` + `int64_t` | 279.09 ms (`279,089 us`) | **5.49x faster** |
| **Java 21 (`OpenJDK 21 HotSpot C2 JIT`)** | GC `byte[4096]` + `BigDecimal` | 2,150.50 ms (`2,150,500 us`) | **42.33x faster** |
| **Go (`go build -ldflags="-s -w"`)** | GC `make([]byte, 0, 4096)` + `int64` | 3,482.53 ms (`3,482,531 us`) | **68.54x faster** |
| **Python 3.11** | `bytearray` + `decimal.Decimal` | 4,017.44 ms (scaled from 250k) | **79.07x faster** |

---

## Repository Structure

```text
├── bin/
│   ├── axc.exe                 # Self-hosted Axon v0.9.1 compiler for Windows x64 (838.50 KB / 858,624 B)
│   ├── axc-linux-x64           # Cross-compiled standalone Axon v0.9.1 compiler for Linux x86_64 (896.67 KB / 918,192 B)
│   └── axc-macos-arm64         # Cross-compiled standalone Axon v0.9.1 compiler for macOS Apple Silicon (640.06 KB / 655,424 B)
├── compiler_ax/                # 100% pure Axon self-hosted compiler source (17 modules, 20,705 lines)
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
├── std/                        # 27 pure Axon stdlib modules (7,841 lines: str, http, data, test_kit, ai*, game*, math3d, async_io, decimal, ringbuf, taint, cap, crdt, crypto, collections, json, os)
├── tests/                      # 23 native execution pass tests + 23 compile-time safety rejection gates
├── AXON_LLM_SPEC.md            # Compact grammar & error reference for AI Coding Agents
├── LEARN_AXON_VI.md            # Official Vietnamese learning guide (16 chapters)
└── LEARN_AXON_EN.md            # Official English learning guide (16 chapters)
```

---

## Verifying Deterministic 3-Stage Self-Hosting

```powershell
.\bin\axc.exe build compiler_ax/main.ax -o axc_stage1.exe
.\axc_stage1.exe build compiler_ax/main.ax -o axc_stage2.exe
.\axc_stage2.exe build compiler_ax/main.ax -o axc_stage3.exe
Get-FileHash axc_stage2.exe, axc_stage3.exe -Algorithm SHA256
# Verified v0.9.1 SHA256: 1E17D07BEA8721D75325757F36A9CBEAE75E39CB33A662F4EF4885AF7B61DE23
```
