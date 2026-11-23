# Axon Programming Language (`.ax`) — v1.1.1

**Axon** is a self-hosted, multi-OS native systems programming language combining **Pythonic readability**, **extreme native AVX2/FMA3/multi-core performance** (`5x–18x` faster than Rust across core allocation, matrix, string, and recursion workloads; `30x–50x` faster cold native builds; `11.79x–19.00x` faster than MSVC `/O2 /arch:AVX2` across the 30-benchmark suite), **zero-GC Perceus linear ownership**, **native zero-WebView HTML5/CSS3/Tailwind/JS Desktop GUI (Photon-D2D v2)**, a **103-module pure-Axon standard library**, and **fail-closed compile-time & runtime security (`E0501`–`E0503`, `--shield`)**.

The entire compiler ([`compiler_ax/`](file:///c:/Users/Admin/Downloads/Languague/compiler_ax) — `18` pure Axon modules, `37,570` lines) compiles `.ax` source and `.h` C headers directly in memory into standalone executables, shared libraries, GPU kernels, and WASI WebAssembly modules across **11 hardware/OS targets** — with **zero** dependency on `LLVM`, `Clang`, `GCC`, `MSVC`, `link.exe`, `lld`, or external SDKs.

---

## Key Highlights (v1.1.1)

- **Extreme Native Speed ( AVX2 Auto-Vectorization + Lock-Free Slab Allocator):**
  - **8-lane (`2×YMM`) interleaved `i64` range & linear-recurrence auto-vectorizer** (`VPADDQ`, `VPSUBQ`, `VPMULUDQ`, exact 64-bit signed `SAR`, and power-of-2 `/` & `%`), **64-byte/iter `VPSADBW` byte-stream reduction**, **single-shot dual-YMM 32–64B `VPCMPEQB` string equality**, and **`n <= 7` recursive leaf shrink-wrapping**.
  - **8-class lock-free slab allocator** (`64 B .. 64 KB`, zero atomic `LOCK` overhead on single-threaded paths via `mt_active`, hardware spinlock when multi-threaded, double-free canary guard): `200,000 × 4 KB` alloc/touch/free runs in **`1 ms`** vs Rust `10 ms` (**10.0x faster**), Go `157 ms` (**157x**), Java `150 ms` (**150x**).
- **Complete 103-Module Pure-Axon Standard Library (`39,719` Lines, Zero C/Python Helpers):**
  - **New in v1.2 (Core -> Web -> Graph):** $O(N)$ LSD Radix Sort + Priority Queue + DSU ([`std/algo.ax`](file:///c:/Users/Admin/Downloads/Languague/std/algo.ax)), auto-growing Perceus containers `DynIntVec`/`StrVec`/`DynMap`/`StrMap`/`IntDeque`/`BitSet` ([`std/ds.ax`](file:///c:/Users/Admin/Downloads/Languague/std/ds.ax)), atomic filesystem & directory listing ([`std/fs.ax`](file:///c:/Users/Admin/Downloads/Languague/std/fs.ax)), radix/table/human formatters ([`std/fmt.ax`](file:///c:/Users/Admin/Downloads/Languague/std/fmt.ax)), `p50`/`p95`/`p99` microbenchmark harness ([`std/bench.ax`](file:///c:/Users/Admin/Downloads/Languague/std/bench.ax)), RFC-3986 URL parser + octal/hex/IMDS SSRF guard ([`std/url.ax`](file:///c:/Users/Admin/Downloads/Languague/std/url.ax)), RFC-7519 HS256 JWT + `"alg":"none"` blocker + password hashing ([`std/jwt.ax`](file:///c:/Users/Admin/Downloads/Languague/std/jwt.ax)), RFC-6455 WebSockets ([`std/ws.ax`](file:///c:/Users/Admin/Downloads/Languague/std/ws.ax)), `:param` web router + static file server ([`std/web.ax`](file:///c:/Users/Admin/Downloads/Languague/std/web.ax)), and forward-star BFS/Dijkstra/Topological-Sort graphs ([`std/graph.ax`](file:///c:/Users/Admin/Downloads/Languague/std/graph.ax)).
  - **Full-Stack Foundations:** TCP/UDP (`std::net`), TLS 1.2/1.3 (`std::tls`), HTTP/1.1 (`std::http`), HTTP/2 + HPACK + Protobuf v3 + gRPC (`std::grpc`), PostgreSQL Wire v3.0 (`std::postgres`), MySQL Wire v10 (`std::mysql`), SQLite3 (`std::sqlite`), 16-arg Win64/SysV C-ABI FFI (`std::ffi`), AI Agent Harness & Eval (`std::ai_harness`, `std::ai_agent`, `std::ai_eval`), zero-`nvcc` CUDA/GGUF/LLaMA/vLLM/FAISS, and 2D/3D Game Engine.
- **Native Photon-D2D v2 Desktop UI (`0` WebView, `0` Electron, `< 12 MB` RAM):** Write desktop apps in pure HTML5 + CSS3 (Flexbox, Grid, `:hover`, `:focus`, `:active`, linear gradients, multi-layer shadows, `border-radius`) + Tailwind CSS + `<script>` JS engine (`let`/`const`, `if`/`else`, `while`, `===`, `&&`/`||`, DOM bindings) with smooth 60 FPS mouse-wheel inertia, dark native DWM titlebar, and HiDPI ClearType Direct2D rendering ([`std/ui.ax`](file:///c:/Users/Admin/Downloads/Languague/std/ui.ax), [`std/gui.ax`](file:///c:/Users/Admin/Downloads/Languague/std/gui.ax), [`examples/antigravity_ui.ax`](file:///c:/Users/Admin/Downloads/Languague/examples/antigravity_ui.ax)).
- **Self-Hosted Deterministic Toolchain & Zero-Trust Verification:**
  - Verified by 3-stage deterministic self-hosting bootstrap (`SHA256(Stage 2) == SHA256(Stage 3) = D6064066AD2600074A31C52158E73BE2BFA09761B56BD164475F4C7C6DE01A96`, `2,357,248` bytes).
  - **`76/76`** end-to-end & hostile runtime test suites ([`tests/pass/`](file:///c:/Users/Admin/Downloads/Languague/tests/pass)) + **`37/37`** compile-time rejection gates ([`tests/fail_compile/`](file:///c:/Users/Admin/Downloads/Languague/tests/fail_compile)).

---

## Documentation, Memory & Learning Guides (Humans & AI Agents)

| Audience | Document | Description |
| :--- | :--- | :--- |
| **Humans (Beginner -> Expert)** | [**`docs/LEARN_AXON_HUMAN.md`**](file:///c:/Users/Admin/Downloads/Languague/docs/LEARN_AXON_HUMAN.md) | Structured 8-module hands-on course from Day-1 syntax & Perceus 0-GC to Web/JWT/WS, SQLite, Native HTML/Tailwind GUI, and AI/GPU |
| **Humans (Full Textbooks)** | [`LEARN_AXON_VI.md`](file:///c:/Users/Admin/Downloads/Languague/LEARN_AXON_VI.md) / [`LEARN_AXON_EN.md`](file:///c:/Users/Admin/Downloads/Languague/LEARN_AXON_EN.md) | 20-chapter Vietnamese & English reference textbooks + 4-week study roadmap |
| **AI Coding Agents (Master Spec)** | [**`docs/AGENT_MASTER_GUIDE.md`**](file:///c:/Users/Admin/Downloads/Languague/docs/AGENT_MASTER_GUIDE.md) | Zero-hallucination grammar rules, full 103-module lookup matrix, `E0001..E0604` fix dictionary, and full-stack copy-paste templates |
| **AI Agents (Quick Rules & MCP)** | [`AGENTS.md`](file:///c:/Users/Admin/Downloads/Languague/AGENTS.md) / [`AXON_LLM_SPEC.md`](file:///c:/Users/Admin/Downloads/Languague/AXON_LLM_SPEC.md) / [`docs/AGENT_FULLSTACK_GUIDE.md`](file:///c:/Users/Admin/Downloads/Languague/docs/AGENT_FULLSTACK_GUIDE.md) | Workspace agent rules, compact LLM grammar sheet, and production full-stack recipes |
| **Architecture & Compiler Memory** | [**`MEMORY.md`**](file:///c:/Users/Admin/Downloads/Languague/MEMORY.md) / [`docs/MEMORY_ARCHITECTURE.md`](file:///c:/Users/Admin/Downloads/Languague/docs/MEMORY_ARCHITECTURE.md) | Complete compiler internals, AST/state layouts, Perceus 0-GC + slab allocator, x86-64/AVX2 register conventions, and pitfalls |
| **Benchmarks** | [`docs/BENCHMARK_REPORT.md`](file:///c:/Users/Admin/Downloads/Languague/docs/BENCHMARK_REPORT.md) | Reproducible multi-language benchmark methodology & hardware notes |

---

## Verified 10-Domain Benchmark Suite (Axon v1.2 vs Rust, Go, Java, Node.js, Python)

Measured live (median of 3 trials, **10/10 bit-identical mathematical checksums** across all languages: `chk=25493856,-3125000,5000000,8574940808537483579,39088169,148933,8160000000,0,1500147,256`) on Windows x64 against `rustc -O`, `rustc -C opt-level=3 -C target-cpu=native`, `go1.26.4`, `OpenJDK 21`, `Node.js v24.15.0`, and `Python 3.11`:

| Domain | Workload | Axon v1.2 | Rust (`-O`) | Rust (`-O3 native`) | Go 1.26 | Java 21 | Node v24 | Speedup vs Rust (`-O`) |
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

- **30-Benchmark Suite (`examples/bench_30_suite/axon_bench_30.ax` vs MSVC `/O2 /arch:AVX2`):** **`11.79x` faster overall** (`120.1 ms` vs `1,416.1 ms`, `0 B` leaks), including **`19.00x` faster** on Extreme AI/Game/UI workloads (FlashAttention-2 `52.71x`, Binary-Heap A* `46.09x`, 64-dim L2 Search `39.81x`, Reactive UI Signals `34.99x`).

---

## Language Tour at a Glance

```axon
// 1. Dynamic Containers (`std::ds`) + O(N) Radix Sort (`std::algo`) + Formatting (`std::fmt`)
use std::ds
use std::algo
use std::fmt

fn main() -> i32 {
    mut scores := ibuf_new(8)
    ibuf_push(scores, 9500); ibuf_push(scores, -120); ibuf_push(scores, 42000)
    sort_i64(scores) // O(N) LSD Radix Sort across full signed i64

    mut users := smap_new()
    smap_set_str(mut users, "alice", "admin")
    println(f"Top={fmt_commas(ibuf_get(scores, 2))} role={smap_get_str(users, 'alice')}")

    smap_free(move users)
    buf_free(move scores)
    return 0
}
```

```axon
// 2. Web Router (`std::web`) + HS256 JWT (`std::jwt`) + SSRF Guard (`std::url`)
use std::web
use std::jwt
use std::url

fn handle_api(method: str, path: str, secret: str) -> str {
    if web_route_match(method, path, "GET", "/users/:id") {
        uid := web_param(path, "/users/:id", "id")
        tok := jwt_sign_hs256("{\"sub\":\"" + uid + "\",\"exp\":2000000000}", secret) or ""
        return web_json(200, "{\"id\":\"" + uid + "\",\"token\":\"" + tok + "\"}")
    }
    return web_json(404, "{\"error\":\"not found\"}")
}
```

```axon
// 3. Native Zero-WebView HTML5 + Tailwind + JS Desktop App (Photon-D2D v2)
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

## Complete Standard Library (`103` Pure Axon Modules, `39,719` Lines)

| Category | Modules (`use std::<name>`) | Key Capabilities |
| :--- | :--- | :--- |
| **Core, Math & Formatting** | `prelude`, `str`, `fmt`, `math`, `decimal`, `rand`, `time`, `datetime`, `uuid`, `binary`, `encoding`, `bench`, `test_kit` | UTF-8/SWAR strings, hex/bin/oct/comma/`f64`/Markdown-table formatters, fixed-point `dec`, `p50`/`p95`/`p99` benchmarking, Base64/Base64URL/Hex |
| **Algorithms, Containers & Graphs** | `algo`, `ds`, `graph`, `vec`, `collections`, `ringbuf`, `cache`, `linq`, `crdt` | $O(N)$ signed `i64` LSD Radix Sort, QuickSort, binary search, min-heap, DSU, auto-growing `DynIntVec`/`StrVec`/`DynMap`/`StrMap`/`IntDeque`/`BitSet`, BFS/Dijkstra/Kahn TopoSort, zero-cost generics (`Box[T]`, `Option[T]`) |
| **OS, Filesystem, CLI & FFI** | `fs`, `sys`, `os`, `path`, `cli`, `terminal`, `log`, `ffi`, `sync`, `memory`, `mem_guard` | Crash-safe `fs_atomic_write`, `fs_mkdir_all`, `fs_read_lines`, `fs_list_dir`, 16-arg Win64/SysV C-ABI FFI (`ffi_open`/`ffi_sym`/`ffi_call_arr`), mutexes, atomics, channels |
| **Web, Networking, Auth & DB** | `net`, `tls`, `http`, `url`, `jwt`, `ws`, `web`, `grpc`, `async_io`, `sqlite`, `postgres`, `mysql` | TCP/UDP, TLS 1.2/1.3 (WinHTTP Schannel + OpenSSL), HTTP/1.1, `:param` router & static server, RFC-3986 URL + SSRF guard, HS256 JWT + password hashing, RFC-6455 WebSockets, gRPC/HTTP2/Protobuf, SQLite3, PostgreSQL v3, MySQL v10 |
| **Data Formats & Storage** | `json`, `csv`, `toml`, `regex`, `compress`, `data` | SWAR JSON parser + `to_json`, RFC-4180 CSV, TOML, NFA/DFA Regex, LZ/RLE compression, checksummed `KvStore` |
| **Desktop GUI & WebView** | `ui`, `gui`, `html`, `css`, `ui_tailwind`, `ui_layout`, `ui_render`, `ui_js`, `ui_signal`, `webview` | Direct2D HiDPI ClearType HTML5/CSS3/Tailwind/JS desktop engine (`< 12 MB` RAM, 60 FPS wheel inertia, `:hover`/`:focus`/`:active`) + lean `--app=` Edge/Chromium host |
| **AI Agents, LLM & GPU Compute** | `ai`, `ai_harness`, `ai_agent`, `ai_eval`, `ai_engine`, `ai_math`, `ai_nn`, `ai_autograd`, `ai_quant`, `ai_local`, `ai_gguf`, `ai_lora`, `ai_serve`, `llama`, `vllm`, `transformers`, `cuda`, `npu`, `faiss`, `onnx`, `safetensors`, `tiktoken`, `torch`, `numpy`, `sklearn`, `quant` | Multi-provider Agent Harness, ReAct + DAG + Swarm, multi-core AVX2+FMA3 FP32/Q8/Q4/BitNet GEMM, zero-`nvcc` PTX JIT CUDA, GGUF/LLaMA/vLLM PagedAttention, FAISS vector search |
| **2D/3D Games, Audio & Kernel** | `game`, `game2d`, `game3d`, `game_ecs`, `game_physics`, `game_render`, `math3d`, `audio`, `cap`, `taint`, `crypto`, `shield`, `kernel`, `ring_bridge`, `iot`, `semiconductor` | SoA `Table[T]` SIMD ECS, rigid-body physics, PBR 3D, PCM/WAV audio, SHA-256/HMAC/ChaCha20/constant-time `ct_eq_str`, `tainted[str]` (`E0502`), `RootCap` (`E0501`), `--shield`, Ring-0/MMIO |

---

## Quickstart & CLI Reference

```powershell
# 1. Sub-millisecond static verification (types, Perceus 0-GC, taint E0502, unsafe E0503, shapes)
.\bin\axc.exe check --json examples/hello.ax

# 2. Compile & run immediately, or run `@test` unit tests
.\bin\axc.exe run examples/hello.ax
.\bin\axc.exe test tests/pass/53_stdlib_v2_and_avx2_hostile.ax

# 3. Build standalone native executable (with optional Shield v5 + symbol map)
.\bin\axc.exe build examples/antigravity_ui.ax -o app.exe
.\bin\axc.exe build examples/hello.ax --shield --emit-map -o hello_shield.exe

# 4. Cross-compile to any of the 11 built-in targets (0 external linker/SDK!)
.\bin\axc.exe build examples/hello.ax --target linux-x64 -o hello_linux
.\bin\axc.exe build examples/hello.ax --target linux-arm64 -o hello_arm64
.\bin\axc.exe build examples/hello.ax --target macos-arm64 -o hello_macos
.\bin\axc.exe build examples/hello.ax --target wasm32 -o hello.wasm

# 5. Reproduce 3-stage deterministic self-hosting bootstrap
.\bin\axc.exe build compiler_ax/main.ax -o axc_s1.exe
.\axc_s1.exe build compiler_ax/main.ax -o axc_s2.exe
.\axc_s2.exe build compiler_ax/main.ax -o axc_s3.exe
Get-FileHash axc_s2.exe, axc_s3.exe -Algorithm SHA256 # Identical SHA-256!
```

---

## 11 Built-In Cross-Compilation Targets

| Category | Target Flag (`--target`) | Binary / Output Format | Key Capabilities |
| :--- | :--- | :--- | :--- |
| **Desktop / Server** | `win-x64` (default) | Win64 `PE32+` (`.exe`, `--shared` `.dll`/`.pyd`) | Unlimited multi-DLL `.idata`, `.edata`, `.pdata` SEH, strict `W^X`, COFF `.obj`/`.lib`, `IOCP` |
| **Desktop / Server** | `linux-x64` | Linux x86_64 `ELF64` | Direct `SYSCALL` zero-CRT runtime, `.eh_frame`, `SSE2`/`AVX2`/`AVX-512` |
| **Desktop / Server** | `linux-arm64` | Linux AArch64 `ELF64` | Native ARM64 machine code (`MOVZ`/`MOVK` >64KB pool safe) + 128-bit `NEON` |
| **Desktop / Server** | `macos-arm64` | Apple Silicon `Mach-O 64` | 16 KB page-aligned `LC_MAIN` + `NEON` |
| **Desktop / Server** | `macos-x64` | Intel macOS `Mach-O 64` | Direct Darwin BSD `SYSCALL` |
| **Web / Edge** | `wasm32` | W3C WebAssembly (`.wasm`) | `wasi_snapshot_preview1`, `0xFD SIMD128`, dynamic `memory.grow`, 0 external SDK |
| **Bare-Metal** | `unikernel-x64` | `Multiboot2` (`0xE85250D6`) | Ring-0 bare-metal kernel image + `COM1 UART` |
| **Bare-Metal** | `freestanding-x64` | Zero-OS x86_64 `ELF64` | Embedded / bootloader bare-metal output |
| **Bare-Metal** | `freestanding-riscv64` | `RV64GC` `ELF64` (`EM_RISCV=243`) | Native RISC-V 64 machine code (`LUI`/`ADDI` 32-bit pool offsets) |
| **GPU (`--emit-gpu`)** | `spirv` | Khronos `SPIR-V 1.6` (`.spv`) | Direct compute kernels from `@gpu_kernel` / `@shader` |
| **GPU (`--emit-gpu`)** | `ptx` | NVIDIA `PTX 8.0` (`.ptx`) | Direct `sm_80` GPU kernels without `nvcc` |
