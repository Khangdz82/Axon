# Axon Programming Language (`.ax`) — v1.1.0

**Axon** is a self-hosted, multi-OS native systems programming language combining **Pythonic readability**, **native bare-metal performance** (`11.79x` faster than MSVC `/O2 /arch:AVX2` across the 30-benchmark suite, `19.00x` on Extreme AI/Game/UI workloads, `12.15x` faster compile), **zero-GC Perceus ownership**, **native zero-WebView HTML/CSS/Tailwind/JS Desktop GUI (Photon-D2D)**, a **production AI Agent Harness & Eval suite**, and **fail-closed compile-time & runtime security (`E0501`–`E0503`, `--shield`)**.

The entire compiler (`compiler_ax/` — `18` pure Axon modules, `37,137` lines) compiles `.ax` source and `.h` C headers directly in memory into standalone executables, shared libraries, GPU kernels, and WASI WebAssembly modules across **11 hardware/OS targets** — with **zero** dependency on `LLVM`, `Clang`, `GCC`, `MSVC`, `link.exe`, `lld`, or external SDKs.

---

## Key Highlights (v1.1.0)

- **Native Photon-D2D Web UI Engine (0 WebView, 0 Electron, 0 Blur):** Build crisp desktop UIs directly in HTML5 + CSS3 (Grid, Flexbox, `flex-wrap`, `position: absolute/fixed`, `:root`/`var(--*)`, compound/descendant selectors) + Tailwind CSS (`dark:`, `sm:`/`md:`/`lg:`/`xl:`) + `<script>` JS engine + `<img src>` (`gdiplus` + Direct2D `BitmapBrush`) + per-container `overflow-y-auto` scrollbars + live Unicode `<input>`/`<textarea>` + Per-Monitor HiDPI ClearType rendering (`std/ui.ax`, `std/gui.ax`, `std/html.ax`, `std/css.ax`, `std/ui_tailwind.ax`, `std/ui_layout.ax`, `std/ui_js.ax`).
- **Ultra-Lean Chromium Desktop Host (`std/webview.ax`):** For complex web apps, launch native Edge/Chromium desktop windows (`--app=`, no tabs/omnibox) with isolated ephemeral user profiles in `%TEMP%` (zero junk folders in cwd, automatic wipe on exit), capped 64MB V8 heap (`--js-flags="--max-old-space-size=64"`), single renderer process, 8MB disk cache, zero GPU shader disk cache, and zero background telemetry/sync (~35–55 MB idle RAM vs 250MB+ Electron).
- **Production AI Agent Harness & Eval Framework (`std/ai_harness.ax`, `std/ai_agent.ax`, `std/ai_eval.ax`):** Token-budgeted context manager, OpenAI/Anthropic/Gemini/Ollama tool-call normalizer, parallel tool dispatcher, sandboxed tools (path/command guards, secret redaction, prompt-injection detection), retry/circuit-breaker/rate-limiter, persistent `.axmem` key-value memory, DAG task planner, agent-to-agent swarm bus, ReAct loop, and benchmark eval graders (`p50`/`p95`/`p99`, `pass@k`, Markdown/JSONL export).
- **Full-Stack Production Stdlib (`93` Pure Axon Modules, `38,684` Lines):** Native TCP/UDP sockets, TLS 1.2/1.3 (WinHTTP Schannel + OpenSSL with fail-closed hostname checks), HTTP/1.1, HTTP/2 + HPACK + Protobuf v3 + gRPC, PostgreSQL Wire v3.0, MySQL Wire v10, SQLite3, universal 16-arg Win64/SysV C-ABI FFI (`std::ffi`), zero-cost generics (`Box[T]`, `Option[T]`, `Pair[A,B]`), zero-`nvcc` PTX JIT CUDA (`std::cuda`), GGUF/LLaMA/vLLM/FAISS, and 2D/3D Game Engine + Audio.
- **11.79x Faster Runtime & 12.15x Faster Compile vs MSVC `/O2 /arch:AVX2` (30/30 Benchmarks):** Across [`examples/bench_30_suite/axon_bench_30.ax`](examples/bench_30_suite/axon_bench_30.ax) (`0 B` leaks):
  - **Easy (`B01..B10`):** `45.36 ms` vs `312.27 ms` (**6.88x faster**) — ERMS `REP STOSD` canvas spans (`21.61x`), zero-IDIV calendar (`8.50x`), `FrameArena` (`12.71x`).
  - **Medium (`B11..B20`):** `31.18 ms` vs `276.24 ms` (**8.86x faster**) — Zero-VDOM reactive signals (`34.99x`), 100k-row virtual list (`27.79x`), AVX2+FMA3 RMSNorm+RoPE (`10.15x`).
  - **Extreme (`B21..B30`):** `43.56 ms` vs `827.56 ms` (**19.00x faster**) — PagedKV FlashAttention-2 (`52.71x`), Binary-Heap A* (`46.09x`), 64-dim L2 Vector Search (`39.81x`), ChaCha20 ARX (`15.82x`).
- **Self-Hosted Deterministic Toolchain:** Prebuilt binaries verified by 3-stage deterministic bootstrap (`SHA256(Stage 2) == SHA256(Stage 3)`): Windows x64 (`bin/axc.exe`, `1,843,712 B`), Linux x64 (`bin/axc-linux-x64`, `1,799,088 B`), and macOS Apple Silicon (`bin/axc-macos-arm64`, `1,278,016 B`).
- **Hardened Security & Verification (`71/71` pass, `34/34` fail_compile):** Compile-time capability attenuation (`E0501`), `tainted[str]` SQLi/CMDi blocking (`E0502`), strict `unsafe` boundary (`E0503`) covering raw pointer/FFI/atomic/SIMD intrinsics, Ring 0 <-> Ring 3 bridge (`std/ring_bridge.ax`), Shield v5 (`std/shield.ax`), PIE-safe runtime stack backtraces (`exit 101`), and static advisor `axc advise <file.ax> [--json]`.

---

## Documentation & Learning Path

- **Vietnamese Complete Textbook (20 Chapters + 4-Week Roadmap):** [`LEARN_AXON_VI.md`](LEARN_AXON_VI.md)
- **English Complete Textbook:** [`LEARN_AXON_EN.md`](LEARN_AXON_EN.md)
- **Standard Learning Guide (Humans & AI Agents):** [`docs/LEARNING_GUIDE_V1_1.md`](docs/LEARNING_GUIDE_V1_1.md)
- **Memory Architecture & Zero-Leak Guide:** [`docs/MEMORY_ARCHITECTURE.md`](docs/MEMORY_ARCHITECTURE.md)
- **Full-Stack Production Recipes & AI Agent Spec:** [`AGENTS.md`](AGENTS.md) | [`docs/AGENT_FULLSTACK_GUIDE.md`](docs/AGENT_FULLSTACK_GUIDE.md) | [`AXON_LLM_SPEC.md`](AXON_LLM_SPEC.md)
- **Benchmark Methodology & Report:** [`docs/BENCHMARK_REPORT.md`](docs/BENCHMARK_REPORT.md)

---

## Language Tour at a Glance

```axon
// 1. Pythonic ergonomics + zero-cost Error Unions (`T ! E`) + Generics (`Box[T]`)
struct Box[T] { val: T }

fn parse_port(raw: i64) -> i64 ! i64 {
    if raw <= 0 or raw > 65535 { return err(400) }
    return ok(raw)
}

fn main() -> i32 {
    p1 := parse_port(-1) or 8080          // fallback expression on error
    p2 := parse_port(443) or return 1     // early return on error
    b := Box[i64](val = p1 + p2)
    println(f"Sum port = {b.val}")
    return 0
}
```

```axon
// 2. Columnar `Table[T]` SoA + Auto-SIMD AVX2 + Dead-Column Elimination
struct Particle { x: f32, vx: f32, y: f32, vy: f32, mass: f32, charge: f32 }

fn step(world: Table[Particle], dt: f32) {
    // Compiles to 256-bit AVX2 `VFMADD231PS` / `VMULPS`; skips untouched columns
    for p in world {
        p.x += p.vx * dt
        p.y *= 3.0
    }
}
```

```axon
// 3. Native Zero-WebView HTML5 + Tailwind + JS Desktop App (Photon-D2D)
use std::ui
use std::gui

fn main() -> i32 {
    html := "<div class='flex flex-col h-screen bg-slate-950 text-slate-100 p-6 gap-4'>"
          + "<h1 class='text-2xl font-bold text-sky-400'>Axon Native Photon-D2D UI</h1>"
          + "<button id='btn' class='px-4 py-2 rounded-lg bg-sky-500 text-white' onclick=\"count = count + 1; setText('out', 'Clicks: ' + count)\">Click me</button>"
          + "<div id='out' class='text-slate-300'>Clicks: 0</div></div>"
          + "<script>let count = 0;</script>"
    mut doc := ui_window_html("Axon App", 960, 640, html)
    gui_run_ui_app(mut doc)
    return 0
}
```

---

## Quickstart: CLI, `axpm` & Browser Playground

```powershell
# Windows x64 — installs `axc.exe` + 93 stdlib modules + AXON_LLM_SPEC.md to ~/.axon
.\install.ps1
axc version

# Sub-millisecond static verification (human or AI-agent JSON)
.\bin\axc.exe check examples/hello.ax
.\bin\axc.exe check --json tests/pass/44_ai_harness.ax
.\bin\axc.exe advise examples/hello.ax

# Compile, run, test, and build hardened production binary with symbol map
.\bin\axc.exe run examples/hello.ax
.\bin\axc.exe test tests/pass/45_deep_security_wave2.ax
.\bin\axc.exe build examples/hello.ax --shield --emit-map -o bin/hello.exe

# Managed project (`axon.toml` + `axon.lock`) & built-in MCP JSON-RPC server
.\bin\axc.exe init my_service
.\bin\axc.exe mcp
```

*(Linux / macOS users run `./install.sh` to install `bin/axc-linux-x64` or `bin/axc-macos-arm64` alongside `std/` into `~/.axon`.)*

### Browser Playground ([`playground/index.html`](playground/index.html))
Open [`playground/index.html`](playground/index.html) directly in Chrome/Edge/Firefox (`file://` or any static host) to compile and execute Axon -> `WASM32+SIMD128` live in the browser (`1M` SIMD128 particles, `T ! Error` unions, AI inference, and mini 2D game).

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

## Standard Library (`93` Pure Axon Modules, `38,684` Lines)

| Domain | Key Modules | Highlights |
| :--- | :--- | :--- |
| **Native Web UI & WebView** | `ui`, `gui`, `html`, `css`, `ui_tailwind`, `ui_layout`, `ui_js`, `webview` | Photon-D2D Direct2D HiDPI ClearType 0-blur renderer (< 12MB RAM) + Ultra-Lean Chromium Host (`--app=`, isolated %TEMP% profile, 64MB V8 heap, 8MB disk cache, 0 GPU cache files, auto-cleanup on close) |
| **AI Agent Harness & Eval** | `ai_harness`, `ai_agent`, `ai_eval` | Context token budgeter, OpenAI/Anthropic/Gemini/Ollama tool normalization, parallel tool dispatch, sandboxed path/cmd/redaction/injection guards, `.axmem` memory, DAG planner, swarm bus, ReAct loop, eval graders & percentiles |
| **AI / LLM & GPU Compute** | `ai`, `ai_engine`, `ai_gguf`, `ai_lora`, `ai_serve`, `llama`, `vllm`, `cuda`, `faiss`, `onnx`, `safetensors`, `tiktoken`, `torch`, `quant`, `numpy` | Zero-`nvcc` PTX JIT CUDA driver, GGUF v2/v3 + Q4_K/Q8_0/F16 dequant, LLaMA RoPE/RMSNorm/SwiGLU/Top-P, vLLM PagedAttention, FAISS L2 search, LoRA/QLoRA, symbolic tensor shapes (`E0602`) |
| **Networking, TLS, RPC & DB** | `net`, `tls`, `http`, `grpc`, `sqlite`, `postgres`, `mysql`, `async_io` | TCP/UDP sockets with timeouts, TLS 1.2/1.3 (Schannel + OpenSSL fail-closed SNI/hostname), HTTP/1.1, HTTP/2 + HPACK + Protobuf v3 + gRPC, pure-Axon PostgreSQL Wire v3.0 & MySQL v10, SQLite3 |
| **System, FFI & Generics** | `ffi`, `sys`, `os`, `path`, `cli`, `sync`, `vec`, `collections`, `ringbuf`, `memory`, `mem_guard` | 16-arg Win64 + SysV C-ABI FFI, env/cwd/process/fs, mutex/channel/atomics, zero-cost generics (`Box[T]`, `Option[T]`, `Pair[A,B]`), lock-free SPSC ring buffer |
| **Data, Formats & Core** | `json`, `csv`, `toml`, `regex`, `compress`, `encoding`, `binary`, `data`, `str`, `math`, `decimal`, `rand`, `time`, `uuid`, `log` | SWAR JSON scanner + `to_json`, RFC-4180 CSV, TOML, Regex, LZ/RLE compression, Base64/Hex/HTML escape, checksummed `KvStore`, financial `dec`, monotonic `Stopwatch` |
| **2D/3D Games & Audio** | `game`, `game2d`, `game3d`, `game_ecs`, `game_physics`, `math3d`, `audio` | SoA `Table[T]` SIMD ECS, AABB/Circle/SAT physics, A*, BVH ray-triangle, PBR + CSM shadows, `vec3`/`mat4`/`quat`, PCM/ADSR/WAV audio |
| **Security & Kernel** | `cap`, `taint`, `crypto`, `shield`, `ring_bridge`, `crdt` | `RootCap`/`FsCap`/`NetCap` (`E0501`), `tainted[str]` (`E0502`), SHA-256/HMAC-SHA256/ChaCha20/constant-time `ct_eq_str`, Shield v5, Ring-0/Ring-3 SQ/CQ bridge |

---

## Security, Ownership & Verification Summary

- **Compile-Time Safety Gates (`34/34` verified in `tests/fail_compile/`):**
  Second-class reference isolation (`E0101`–`E0102`), Perceus linear ownership / move-in-loop / `par_for` race detection (`E0201`–`E0204`), immutability (`E0301`), capability attenuation (`E0501`), unsanitized `tainted[str]` SQL/shell sinks (`E0502`), **strict `unsafe` boundary (`E0503`)** blocking all unguarded raw pointer, FFI, SIMD pointer, and atomic intrinsics, `@no_alloc` verification (`E0601`), symbolic tensor shapes (`E0602`), `comptime_assert` (`E0604`), and parser recursion cap (`E0020`, depth limit `256`).
- **Fail-Closed Runtime Protection (`71/71` `tests/pass/*.ax` + `6/6` runtime traps -> `exit 101`):**
  Hardware/runtime traps with PIE-safe stack backtraces (`#0 fn -> #1 caller -> #2 main`) for integer `/ 0`, `% 0`, `INT64_MIN / -1`, checked `--shield` signed overflow, and `ByteBuf` / `str_byte` out-of-bounds accesses; CWE-158 `NUL`-byte rejection across FS/OS/SQLite/PostgreSQL/MySQL; TLS 1.2+ with fail-closed hostname verification; HTTP/2 4 MB frame caps & strict HPACK/gRPC framing; UNC NTLM leak prevention in native image loaders; bounds-hardened vLLM/GGUF/CUDA/KvStore/FFI/SafeTensors/ONNX modules.

---

## Verified Benchmark Highlights (`B1 – B10`)

Measured live on **Intel Core i5-10400F (6C/12T), 31.87 GB RAM, Windows 10 x64** against `MSVC 19.51` (`C++23`), `rustc 1.100.0`, `go1.26.4`, `OpenJDK 21.0.13`, `Node.js v24.15.0` WASI SIMD128, and `Python 3.11.0`. Full methodology, checksums, CV%, and RSS are in [`docs/BENCHMARK_REPORT.md`](docs/BENCHMARK_REPORT.md):

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
│   ├── axc.exe                  # Self-hosted Axon v1.1.0 Windows x64 compiler (1,843,712 B, --shield)
│   ├── axc-linux-x64            # Cross-compiled Axon v1.1.0 Linux x86_64 compiler (1,799,088 B)
│   └── axc-macos-arm64          # Cross-compiled Axon v1.1.0 Apple Silicon compiler (1,278,016 B)
├── playground/
│   └── index.html               # Zero-dependency browser Axon -> WASM32+SIMD128 IDE & runner
├── compiler_ax/                 # 18 pure Axon compiler modules (37,137 lines, zero LLVM/C/Rust)
├── std/                         # 93 pure Axon stdlib modules (38,222 lines: UI, AI Harness, Net/TLS/DB/gRPC, CUDA, Game, Crypto)
├── examples/                    # Quickstart, Photon-D2D HTML/Tailwind UI apps, kernel & benchmark suites
├── tests/
│   ├── pass/                    # 69 end-to-end native, UI, AI harness, full-stack, generics & security tests
│   ├── fail_compile/            # 34 static compile-time rejection gates (E0020..E0604 incl. E0503 unsafe)
│   ├── audit/                   # Continuous parser, MCP security & memory-leak soak fuzzers
│   └── wasm/                    # WASM32 dynamic growth test + Node WASI runner (`wasi_run.mjs`)
├── docs/
│   ├── AGENT_FULLSTACK_GUIDE.md # Copy-paste full-stack production recipes for AI agents
│   ├── BENCHMARK_REPORT.md      # Reproducible benchmark report
│   ├── LEARNING_GUIDE_V1_1.md   # Structured v1.1 learning roadmap
│   └── MEMORY_ARCHITECTURE.md   # Zero-GC Perceus + arena memory model guide
├── AGENTS.md                    # Mandatory AI coding agent rules & stdlib lookup matrix
├── AXON_LLM_SPEC.md             # Compact grammar/API spec for AI coding agents (`axc mcp`)
├── LEARN_AXON_EN.md             # Complete English textbook
└── LEARN_AXON_VI.md             # Complete 20-chapter Vietnamese textbook + 4-week roadmap
```

```powershell
# Reproduce bit-exact 3-stage `--shield` self-hosting on Windows x64:
.\bin\axc.exe build compiler_ax/main.ax --shield -o axc_stage1.exe
.\axc_stage1.exe build compiler_ax/main.ax --shield -o axc_stage2.exe
.\axc_stage2.exe build compiler_ax/main.ax --shield -o axc_stage3.exe
Get-FileHash axc_stage2.exe, axc_stage3.exe -Algorithm SHA256
# Stage 2 SHA256 == Stage 3 SHA256 (deterministic self-hosted bootstrap)
```
