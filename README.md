# Axon Programming Language (`.ax`) — v1.1.0

**Axon** is a self-hosted, multi-OS native systems programming language combining **Pythonic readability**, **11.79x faster execution than MSVC `/O2 /arch:AVX2`** (`19.00x` on Extreme AI/Game/UI workloads, `12.15x` faster compile), **zero-GC predictability via Perceus ownership + `std::mem_guard`**, and **fail-closed compile-time & runtime security (Shield v5)**.

The entire compiler (`compiler_ax/`) is written in 100% pure Axon and compiles `.ax` source and `.h` C headers directly in memory into standalone executables, shared libraries, GPU kernels, and WASI WebAssembly modules across **11 hardware/OS targets** — with **zero** dependency on `LLVM`, `Clang`, `GCC`, `MSVC`, `link.exe`, `lld`, or external SDKs.

---

## Why Axon v1.1?

- **11.79x Faster Runtime & 12.15x Faster Compile vs MSVC `/O2 /arch:AVX2` (30/30 Benchmarks):** Across [`examples/bench_30_suite/axon_bench_30.ax`](examples/bench_30_suite/axon_bench_30.ax) (`0 B` leaks, `61/61` pass, `34/34` fail_compile):
  - **Easy (`B01..B10`):** `45.36 ms` vs `312.27 ms` (**6.88x faster**) — ERMS `REP STOSD` canvas spans (`21.61x`), zero-IDIV Howard Hinnant calendar (`8.50x`), `FrameArena` (`12.71x`).
  - **Medium (`B11..B20`):** `31.18 ms` vs `276.24 ms` (**8.86x faster**) — Zero-VDOM reactive signals (`34.99x`), 100k-row virtual list (`27.79x`), AVX2+FMA3 RMSNorm+RoPE (`10.15x`).
  - **Extreme (`B21..B30`):** `43.56 ms` vs `827.56 ms` (**19.00x faster**) — PagedKV FlashAttention-2 (`52.71x`), Binary-Heap A* (`46.09x`), 64-dim L2 Vector Search (`39.81x`), ChaCha20 ARX (`15.82x`).
- **Self-Hosted Deterministic Toolchain:** Prebuilt binaries verified by 3-stage deterministic bootstrap (`SHA256(Stage 2) == SHA256(Stage 3)`): Windows x64 (`bin/axc.exe`, `1,836,544 B`), Linux x64 (`bin/axc-linux-x64`, `1,799,088 B`), and macOS Apple Silicon (`bin/axc-macos-arm64`, `1,278,016 B`).
- **Ring 0 <-> Ring 3, Shield v5 & Intelligent Compiler:** [`std/ring_bridge.ax`](std/ring_bridge.ax) (cache-line isolated SQ/CQ + `ring0_probe_user_range`), [`std/shield.ax`](std/shield.ax) (2-adic Hensel Newton-Raphson + 3-layer MBA + `ShieldMesh`), and `axc advise <file.ax> [--json]` static performance/security telemetry.
- **Documentation, Memory & Learning (v1.1):**
  - **Memory Architecture & Zero-Leak Guide:** [`docs/MEMORY_ARCHITECTURE.md`](docs/MEMORY_ARCHITECTURE.md)
  - **Standard Learning Path (Humans & AI Agents):** [`docs/LEARNING_GUIDE_V1_1.md`](docs/LEARNING_GUIDE_V1_1.md)
  - **English / Vietnamese Complete Guides:** [`LEARN_AXON_EN.md`](LEARN_AXON_EN.md) | [`LEARN_AXON_VI.md`](LEARN_AXON_VI.md)
  - **AI Coding Agents & Specs:** [`AGENTS.md`](AGENTS.md) | [`docs/AGENT_FULLSTACK_GUIDE.md`](docs/AGENT_FULLSTACK_GUIDE.md) | [`AXON_LLM_SPEC.md`](AXON_LLM_SPEC.md)

---

## Language Tour at a Glance

Axon pairs zero-cost bare-metal performance with Pythonic ergonomics: `def` or `fn` functions, optional `:` block headers, `elif` chains, `pass` stubs, `#` comments, `True`/`False` literals, `**` power operators, `x if cond else y` ternaries, `in 0..100` range tests, `range(n)` loops, `len()` built-ins, `and`/`or`/`not` logic, default & named `=` arguments, zero-cost `T ! Error` unions in CPU registers, and Perceus linear ownership (`move`) with zero garbage-collector pauses and zero lifetime annotations:

```axon
# 1. Pythonic syntax, def/fn, optional colons, default & named arguments, ** power (`tests/pass/31_pythonic_deep_syntax.ax`)
def greet(name: str, power_level: int = 2 ** 10): {
    status := "Legendary" if power_level > 1000 else "Rookie"
    println(f"Hello, {name}! Level: {power_level} ({status})")
}

@shield
def main() -> i32: {
    greet("Architect")
    greet("Khang", power_level = 9001)

    for i in range(3): {
        if i in 0..2: {
            # In-place modulo and power arithmetic
            mut n := 10
            n **= 2
            n %= 7
        } else: {
            pass
        }
    }
    return 0
}
```

```axon
// 2. Zero-cost Error Unions (`T ! E`) with register-passed propagation (`AXON_LLM_SPEC.md`)
fn parse_port(raw: i64) -> i64 ! i64 {
    if raw <= 0 or raw > 65535 { return err(400) }
    return ok(raw)
}

fn boot() -> i64 ! i64 {
    p1 := parse_port(-1) or 8080      // fallback default on error
    p2 := parse_port(443) or return   // propagate error directly without exceptions
    return ok(p1 + p2)
}
```

```axon
// 3. Columnar `Table[T]` SoA + Auto-SIMD + Dead-Column Elimination (`tests/pass/04_table_soa_avx2_simd.ax`)
struct Particle { x: f32, vx: f32, y: f32, vy: f32, mass: f32, charge: f32 }

fn step(world: Table[Particle], dt: f32) {
    // Compiles to 256-bit AVX2 `VFMADD231PS` / `VMULPS`; skips `vy`, `mass`, `charge` from RAM traffic
    for p in world {
        p.x += p.vx * dt
        p.y *= 3.0
    }
}
```

---

## Quickstart: CLI, `axpm` & Browser Playground

### Install & Run Native Toolchain
```powershell
# Windows x64 — installs `axc.exe` + 34 stdlib modules + AXON_LLM_SPEC.md to ~/.axon
.\install.ps1
axc version

# Static verification (human diagnostics or AI-agent JSON)
.\bin\axc.exe check examples/hello.ax
.\bin\axc.exe check --json tests/pass/16_v7_ai_llm_supremacy.ax

# Compile, execute, and test natively
.\bin\axc.exe build examples/hello.ax -o bin/hello.exe
.\bin\axc.exe run tests/pass/26_v1_stdlib_production.ax
.\bin\axc.exe test tests/pass/17_v7_unit_tests.ax

# Initialize a managed project (`axon.toml` + `axon.lock`) or launch MCP JSON-RPC server
.\bin\axc.exe init my_service
.\bin\axc.exe mcp
```

*(Linux / macOS users can run `./install.sh` to install `bin/axc-linux-x64` or `bin/axc-macos-arm64` alongside `std/` into `~/.axon`.)*

### Run Directly in the Browser ([`playground/index.html`](playground/index.html))
Open [`playground/index.html`](playground/index.html) directly in Chrome/Edge/Firefox (`file://` or any static host). No backend or compiler download required:
- **1M SIMD128 Particles:** Compiles SoA particle simulation to `1,740 B` (`1.70 KB`) `.wasm` (`26` `0xFD SIMD128` opcodes) with live Canvas2D streaming.
- **`T ! Error` Error Unions:** Compiles multi-value `(result i32 i64)` error handling to `1,215 B` (`1.19 KB`) `.wasm`.
- **Small AI Inference:** Compiles Linear + ReLU + Softmax inference to `2,095 B` (`2.05 KB`) `.wasm` (`195 us` WASI inference).
- **Mini 2D Game:** Compiles interactive arcade physics to `1,726 B` (`1.69 KB`) `.wasm` with live Canvas2D input loop and hex opcode inspector.

---

## 11 Built-In Cross-Compilation Targets

| Category | Target Flag (`--target`) | Binary / Output Format | Key Capabilities |
| :--- | :--- | :--- | :--- |
| **Desktop / Server** | `win-x64` (default) | Win64 `PE32+` (`.exe`, `--shared` `.dll`/`.pyd`) | Multi-DLL `.idata`, `.edata`, `.pdata` SEH, strict `W^X`, static COFF `.obj`/`.lib`, `IOCP` |
| **Desktop / Server** | `linux-x64` | Linux x86_64 `ELF64` | Direct `SYSCALL` zero-CRT runtime, `.eh_frame`, `SSE2`/`AVX2`/`AVX-512` |
| **Desktop / Server** | `linux-arm64` | Linux AArch64 `ELF64` | Native ARM64 machine code + 128-bit `NEON` |
| **Desktop / Server** | `macos-arm64` | Apple Silicon `Mach-O 64` | 16 KB page-aligned `LC_MAIN` + `NEON` |
| **Desktop / Server** | `macos-x64` | Intel macOS `Mach-O 64` | Direct Darwin BSD `SYSCALL` |
| **Web / Edge** | `wasm32` | W3C WebAssembly (`.wasm`) | `wasi_snapshot_preview1`, `0xFD SIMD128`, dynamic `memory.grow`, 0 external SDK |
| **Bare-Metal** | `unikernel-x64` | `Multiboot2` (`0xE85250D6`) | Ring-0 bare-metal kernel image + `COM1 UART` |
| **Bare-Metal** | `freestanding-x64` | Zero-OS x86_64 `ELF64` | Embedded / bootloader bare-metal output |
| **Bare-Metal** | `freestanding-riscv64` | `RV64GC` `ELF64` (`EM_RISCV=243`) | Native RISC-V 64 machine code |
| **GPU (`--emit-gpu`)** | `spirv` | Khronos `SPIR-V 1.6` (`.spv`) | Direct compute kernels from `@gpu_kernel` / `@shader` |
| **GPU (`--emit-gpu`)** | `ptx` | NVIDIA `PTX 8.0` (`.ptx`) | Direct `sm_80` GPU kernels without `nvcc` |

---

## Standard Library (`34` Pure Axon Modules, `9,710` Lines)

- **Portable Core (`v1.0.0` Native + `--target wasm32`):**
  - `std/math.ax` (`283` lines) — Overflow-safe integer math (`min`/`max`/`clamp`/`abs`/`gcd`/`lcm`/`mod_pow`/`isqrt`), fixed-point `1e6`, interpolation/easing.
  - `std/rand.ax` (`102` lines) — Deterministic SplitMix64 + Xoshiro128++ PRNG (`rng_new`, `rng_next_u64`, `rng_range_i64`, `rng_next_f64`, `rng_free`).
  - `std/path.ax` (`182` lines) — POSIX/Windows normalization (`path_join`, `path_dirname`, `path_basename`, `path_ext`) + CWE-22 traversal guards (`path_is_safe_rel`).
  - `std/encoding.ax` (`247` lines) — Hex, Base64/Base64URL, CRC32, FNV-1a 64-bit, worst-case safe HTML escaping (`n * 6 + 8` capacity), URL percent coding.
  - `std/log.ax` (`76` lines) — CWE-117 CRLF/control-byte sanitized JSON logger (`log_sanitize` capacity-safe).
  - `std/time.ax` (`49` lines) — Monotonic nanosecond `Stopwatch` (`sw_start`, `sw_elapsed_ns`/`us`/`ms`, `sw_lap`) across native OS clocks and WASI `clock_time_get`.
- **CLI, Web, Data & Core Runtime:**
  - `std/cli.ax`, `std/str.ax`, `std/http.ax` (HTTP/1.1 parser/router/SSE + CRLF & path traversal hardening), `std/data.ax` (CSV/CLI flags/`KvStore`), `std/json.ax`, `std/collections.ax`, `std/os.ax`, `std/test_kit.ax`, `std/decimal.ax`, `std/ringbuf.ax`, `std/async_io.ax`.
- **AI, ML, LoRA & Local/Hybrid Inference (`9` Modules):**
  - `std/ai.ax`, `std/ai_engine.ax`, `std/ai_math.ax`, `std/ai_nn.ax`, `std/ai_autograd.ax`, `std/ai_quant.ax` (Q4/Q8/NF4/FP16/BF16), `std/ai_lora.ax` (LoRA/DoRA/QLoRA), `std/ai_local.ax` (GGUF/AXM/BPE), `std/ai_serve.ax` (tiled FlashAttention, RoPE, RMSNorm, SwiGLU, GQA/MoE, K-Means/PCA/Linear/Logistic, `VramArena`, `PagedKvCache`, Symbolic Shapes `E0602`, Zero-Copy `DLPack`).
- **2D & 3D Game Engine (`4` Modules):**
  - `std/game.ax`, `std/game2d.ax`, `std/game3d.ax`, `std/math3d.ax` (AABB/Circle/SAT physics, spatial hash broadphase, camera/sprites/A*/particles, quaternion skinning, BVH ray-triangle, PBR + CSM shadows, ECS).
- **Security & Distributed Systems:**
  - `std/cap.ax`, `std/taint.ax`, `std/crypto.ax` (NIST FIPS 180-4 SHA-256, RFC 2104 HMAC-SHA256, constant-time `ct_eq_str`), `std/crdt.ax`.

---

## Security, Ownership & Verification Summary

- **Compile-Time Safety Enforcements (`25/25` gates verified in `tests/fail_compile/`):**
  Second-class reference isolation (`E0101`–`E0102`), Perceus linear ownership / move-in-loop / `par_for` race detection (`E0201`–`E0204`), immutability (`E0301`), capability attenuation (`E0501`), unsanitized `tainted[str]` SQL & shell sinks (`E0502`), `@no_alloc` verification (`E0601`), symbolic tensor shapes (`E0602`), `comptime_assert` (`E0604`), and parser recursion cap (`E0020`, depth limit `256`).
- **Fail-Closed Runtime Protection (`6/6` runtime traps -> `exit 101`):**
  Hardware/runtime traps for integer `/ 0`, `% 0`, `INT64_MIN / -1`, checked `--shield` signed overflow, and `ByteBuf` / `str_byte` out-of-bounds accesses, alongside CWE-158 embedded `NUL` rejection in filesystem/`os_exec` calls, wrapping `INT64_MIN` constant folding (`0 -% v`), Polymorphic `@shield` v2, and `@whitebox` $\text{GF}(2^8)$.
- **Post-Edit `v1.0.0` Test Suite Verification:**
  - `29/29` Native & `--shield` pass checks (`27` files in `tests/pass/` incl. `axc test` + `--shield` on `25` and `26`)
  - `25/25` `tests/fail_compile/` negative safety gates rejected cleanly
  - `3/3` `--target wasm32` WASI + SIMD128 suites validated via `node tests/wasm/wasi_run.mjs`
  - `6/6` adversarial runtime traps (`exit 101`)
  - `4/4` browser playground WASM modules validated & executed in Node WASI (`WebAssembly.validate = true`, `rc = 0`)

---

## Verified Benchmark Highlights (`B1 – B10`)

Measured live on **Intel Core i5-10400F (6C/12T), 31.87 GB RAM, Windows 10 x64** against `MSVC 19.51` (`C++23`), `rustc 1.100.0`, `go1.26.4`, `OpenJDK 21.0.13`, `Node.js v24.15.0` WASI SIMD128, and `Python 3.11.0`. Full methodology, checksums, CV%, RSS, and transparent headroom analysis (`B1a`, `B4`, `B5`, `B6`, `B9` WASM bump allocator) are in [`docs/BENCHMARK_REPORT.md`](docs/BENCHMARK_REPORT.md):

| Benchmark | Workload | Axon `win-x64` | Axon `wasm32` | C++23 MSVC | Rust 1.100 | Go 1.26.4 | Java 21 |
| :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: |
| **B1b** | Vector `f32` Dot (`409.6M` FMAs) | **`15.67 ms`** | — | `25.57 ms` | `49.69 ms` | `411.77 ms` | `409.95 ms` |
| **B2** | Particle SoA (`16,384 x 2,000` steps) | **`5.11 ms`** (`Table`) | `10.58 ms` (`2.9 KB`) | `4.10 ms` | `5.60 ms` | `50.00 ms` | `7.28 ms` |
| **B3** | Dense FP32 GEMM `512x512` | `12.97 ms` | — | `10.52 ms` | `9.79 ms` | `62.57 ms` | `61.68 ms` |
| **B7** | Dynamic `4 KB` Frame Churn (`500K`) | `94.67 ms` (`3.0 MB` RSS) | — | `24.75 ms` | `25.74 ms` | `334.17 ms` | `163.36 ms` |
| **B10 Build** | Cold Clean Build Time (`B9` App) | **`15.90 ms`** (`34.0 KB`) | **`15.94 ms`** (`14.1 KB`) | `1,602.68 ms` (`169 KB`) | `431.70 ms` (`163 KB`) | `303.62 ms` (`1.63 MB`) | `519.29 ms` (`.class`) |

---

## Repository Structure & Self-Hosting Reproduction

```text
├── bin/
│   ├── axc.exe                 # Self-hosted Axon v1.0.0 Windows x64 compiler (973.50 KB / 996,864 B, --shield)
│   ├── axc-linux-x64           # Cross-compiled Axon v1.0.0 Linux x86_64 compiler (928.67 KB / 950,960 B)
│   └── axc-macos-arm64         # Cross-compiled Axon v1.0.0 Apple Silicon compiler (720.06 KB / 737,344 B)
├── playground/
│   └── index.html              # Zero-dependency browser Axon -> WASM32+SIMD128 IDE & runner
├── compiler_ax/                # 18 pure Axon compiler modules (self-hosted, zero LLVM/C/Rust)
├── std/                        # 78 pure Axon stdlib modules (net, tls, postgres, mysql, sqlite, grpc, gui, ffi, sys, vec, ai, game, crypto)
├── examples/                   # Quickstart, native HTML/Tailwind UI, kernel & 9-language benchmark suite
├── tests/
│   ├── pass/                   # 56 end-to-end native, full-stack, AI, Game2D/3D, generics & security tests
│   ├── fail_compile/           # 34 static compile-time rejection gates (incl. E0503 unsafe)
│   ├── audit/                  # Continuous parser, MCP security & memory-leak soak fuzzers
│   └── wasm/                   # WASM32 dynamic growth test + Node WASI runner (`wasi_run.mjs`)
├── docs/
│   ├── AGENT_FULLSTACK_GUIDE.md # Complete copy-paste full-stack production recipes for AI agents
│   └── BENCHMARK_REPORT.md     # Reproducible benchmark report
├── AGENTS.md                   # Mandatory AI coding agent rules & 78-module lookup matrix
├── AXON_LLM_SPEC.md            # Compact spec for AI coding agents (`axc mcp`)
├── LEARN_AXON_EN.md            # Complete 16-chapter English textbook
└── LEARN_AXON_VI.md            # Complete 16-chapter Vietnamese textbook
```

```powershell
# Reproduce bit-exact 3-stage `--shield` self-hosting on Windows x64:
.\bin\axc.exe build compiler_ax/main.ax --shield -o axc_stage1.exe
.\axc_stage1.exe build compiler_ax/main.ax --shield -o axc_stage2.exe
.\axc_stage2.exe build compiler_ax/main.ax --shield -o axc_stage3.exe
Get-FileHash axc_stage2.exe, axc_stage3.exe -Algorithm SHA256
# Verified v1.0.0 SHA256: D9B85D3CB1FE4E66C4198DC1BF2FC37CC9168B4169FA3669B6AAC340C2984677
```
