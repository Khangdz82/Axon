# AXON COMPILER & ECOSYSTEM PROGRESS TRACKER

**Current Production Release:** **`v1.0.0`** (Commit `b141ebf`, Tag `v1.0.0`)  
**Self-Hosted Binaries (`--shield` Verified `< 1 MB`):** [`bin/axc.exe`](bin/axc.exe) `973.50 KB` (`996,864 B`) | [`bin/axc-linux-x64`](bin/axc-linux-x64) `928.67 KB` (`950,960 B`) | [`bin/axc-macos-arm64`](bin/axc-macos-arm64) `720.06 KB` (`737,344 B`)  
**Deterministic Hash (Stage 2 == Stage 3 == Stage 4):** `SHA256 = D9B85D3CB1FE4E66C4198DC1BF2FC37CC9168B4169FA3669B6AAC340C2984677`

---

## Official Self-Hosted Releases (`v5.0` -> `v1.0.0`)

- [x] **Release `v1.0.0`: Official Production Release, Browser WASM32 Playground, 34-Module Stdlib & Rigorous `B1–B10` Verification** (COMPLETED & VERIFIED)
  - [x] **Interactive Browser Playground ([`playground/index.html`](playground/index.html)):** Standalone zero-dependency HTML/JS recursive-descent compiler emitting valid W3C WebAssembly + `0xFD SIMD128` + `wasi_snapshot_preview1` with Live Canvas2D & binary opcode inspector:
    - `particles` (`1M SIMD128`): `1,740 B` (`1.70 KB`, `26` `0xFD` opcodes, `5.15 ms` WASI run), `errors` (`T ! Error`): `1,215 B` (`1.19 KB`), `ai` (Linear+ReLU+Softmax): `2,095 B` (`2.05 KB`), `game` (2D arcade): `1,726 B` (`1.69 KB`) — `4/4` `WebAssembly.validate = true`, `rc = 0`.
  - [x] **7 New `v1.0.0` Stdlib Modules (`34` total modules, `9,710` lines):**
    - Portable (Native + `--target wasm32`): `std/math.ax`, `std/rand.ax`, `std/path.ax`, `std/encoding.ax`, `std/log.ax`, `std/time.ax`.
    - Native CLI: `std/cli.ax`.
  - [x] **Deep Security Hardening & Bug Fixes:**
    - Fixed `INT64_MIN` unary negation constant-fold trap under `--shield` (`0 - v` -> `0 -% v` in `compiler_ax/sema_perceus.ax`, `compiler_ax/comptime.ax`, `compiler_ax/c_import.ax`).
    - Hardened `http_is_safe_path` in `std/http.ax` against backslash `\`, `..` segment traversal, and `%5c`/`%2e` smuggling.
    - Fixed worst-case buffer sizing in `html_escape` (`std/encoding.ax`, `n * 6 + 8`) and `log_sanitize` (`std/log.ax`, `n * 2 + 8`).
  - [x] **Honest 10-Suite Empirical Benchmark Report ([`docs/BENCHMARK_REPORT.md`](docs/BENCHMARK_REPORT.md)):**
    - Directly measured `B1–B10` vs `MSVC 19.51` (`C++23`), `rustc 1.100`, `Go 1.26.4`, `OpenJDK 21.0.13`, `Node v24.15` WASI SIMD128, and `Python 3.11.0` (`B1b` `15.67 ms`, `B2 Table` `5.11 ms`, `B3` `12.97 ms`, `B7` `3,008 KB` RSS, `B10` cold build `15.90 ms` win-x64 / `15.94 ms` wasm32), transparently documenting remaining optimization targets (`B1a`, `B4`, `B5`, `B6`, `B9`).
  - [x] **Clean Repo & Full Gate `v1.0.0` Verification:**
    - Removed temporary junk/stale artifacts safely; added [`tests/wasm/wasi_run.mjs`](tests/wasm/wasi_run.mjs); installed via `.\install.ps1` (`1267.84 KB` total).
    - **Native + `--shield` Pass Suite:** `29/29` passed (`tests/pass/01..27` incl. `axc test` + `--shield` on `25` and `26`).
    - **`fail_compile` Safety Gates:** `25/25` rejected cleanly (`E0020`, `E0101`–`E0604`).
    - **WASM32 + WASI Suite:** `3/3` passed (`01_wasm32_dynamic_growth.ax`, `24_wasm32_simd128_wasi.ax`, `27_wasm32_portable_suite.ax`).
    - **Runtime Trap Matrix:** `6/6` blocked fail-closed (`rc = 101`: `div0`, `mod0`, `intmin_div_neg1`, `buf_get_oob`, `str_byte_oob`, `shield_add_ovf`).
    - **Playground WASM Node Gate:** `4/4` passed.

- [x] **Release `v0.9.1`: Compiler Perf/Size Optimization, Production Security Hardening & Zero-GC Stdlib Expansion** (`ff634de`)
  - [x] Added `std/str.ax`, `std/http.ax`, `std/data.ax`, `std/test_kit.ax` (`27` stdlib modules, `7,841` lines); slimmed `bin/axc.exe` to `858,624 B` (`838.50 KB`) with $O(1)$ symbol hashing, zero-copy projections, and scoped slot reuse.
  - [x] Hardened CWE-158 embedded `NUL` checks in `fs_read_file`/`fs_write_file`/`os_exec`, dynamic `h_jbuf_*` auto-growth with RFC-8259 JSON escaping, parser depth limit (`E0020`, cap `256`), `std/cap.ax`, and FIPS 180-4 SHA-256 + HMAC-SHA256 (`std/crypto.ax`).
  - [x] Verified `SHA256 = 1E17D07BEA8721D75325757F36A9CBEAE75E39CB33A662F4EF4885AF7B61DE23`, `23/23` pass, `23/23` fail_compile, `N=6000` check in `25.44 ms`.

- [x] **Release `v0.9.0`: Zero-GC AI/ML/LoRA & 2D/3D Game Engine Stdlib, Large-Scale (`> 6,000` Fns) & Sub-1 MB Compiler** (`23fb188`)
  - [x] Added full Zero-GC AI suite (`std/ai.ax`, `ai_math`, `ai_nn`, `ai_autograd`, `ai_quant`, `ai_lora`, `ai_local`, `ai_serve`) and 2D/3D Game Engine (`std/game.ax`, `game2d`, `game3d`); expanded compiler module to `23` stdlib modules (`6,398` lines).
  - [x] Raised internal helper IDs to `10,000,000+` & dynamic token arenas (`N=6000` scale fix); reduced `bin/axc.exe` by `-33.5%` to `886,272 B` (`865.50 KB`, `SHA256 = 6B4A8BC2DBC553B23EFEA00F595E2829C58AD92E9B8C33C6F99BB09CFF4E8317`).
  - [x] Verified `21/21` pass and `22/22` fail_compile.

- [x] **Milestone `v7.0`: 11-Target Cross-Compiler, `comptime` VM, `@cImport` + Static COFF Linker, `par_for` + `IOCP`, 6 AI/LLM Breakthroughs, 3D/HFT/Taint & `axc mcp`** (`ee3d19c`)
  - [x] Shipped 11 direct targets (`win-x64`, `linux-x64`, `linux-arm64`, `macos-arm64`, `macos-x64`, `wasm32`, `unikernel-x64`, `freestanding-x64`, `freestanding-riscv64`, `spirv`, `ptx`).
  - [x] Added `compiler_ax/comptime.ax`, `c_import.ax`, `backend_gpu_wasm.ax`, `codegen_arm64_rv64.ax`, `linker_unix_bare.ax`, `pkg_mcp.ax`, plus bilingual 16-chapter textbooks (`LEARN_AXON_EN.md`, `LEARN_AXON_VI.md`) and `AXON_LLM_SPEC.md`.

- [x] **Milestone `v6.0` & `v5.0`: Pure-Axon Self-Hosted Compiler Architecture, Temporal AVX2 SoA & Loop Self-FBIP** (`5c01931`, `c007a65`)
  - [x] Re-architected compiler from bootstrap stage into 100% pure-Axon self-hosting toolchain (`SHA256 = 15AB01D09E0835EB750ADAA202981D8FB893E761A6882561A60696169B587AF3` in `v6.0`, `546,304 B`) with dead-column SoA elimination, AVX2 FMA3 auto-dispatch, loop self-FBIP reuse, and polymorphic `@shield` v2.

---

## Historical R&D Foundation Milestones (`M0 – M36`)

Prior foundational engineering phases preserved from git history (`M0–M36`) before the `v5.0–v1.0.0` pure-Axon release line:

- **M0 – M5 (Bootstrap & IR Foundations):** Lexer, arena AST, Pratt parser, second-class reference verifier, linear ownership (`move`), Perceus IR + FBIP reuse, Table SoA C23 codegen, Chase-Lev fiber runtime, Cranelift/MLIR dual-backend prototypes (`d3c8b19`).
- **M6 – M11 (Self-Hosting, Auto-SIMD & Ergonomics):** Pure-Axon self-hosted compiler rewrite, AVX2/AVX-512/NEON vectorization, incremental content-addressed cache, whole-program tree shaking, `T ! Error` unions, loop fusion, initial WASM target (`ed8b739`, `08aeed4`).
- **M12 – M17 (Pythonic Syntax, Hardware FMV & AI Exploration):** `and`/`or`/`not`, `Table[T]`, `f"..."`, named arguments, runtime CPUID dispatch, `comptime`/`@cImport`/`@py_export`, E-Graph passes, initial AI/tensor & bilingual textbook series (`afd7936`, `e9a9348`).
- **M18 – M20 (Hardening & Binary Packaging):** Multi-layer `@shield`, `@secret`, `install.sh` / `install.ps1` installers (`39ba064`).
- **M21 – M31 (AV-Clean Binaries, Numerics & Systems Proving):** Clean PE `.pdata`/`.xdata` & ELF `.eh_frame`, `@whitebox` $\text{GF}(2^8)$, microscaling numerics, rematerialization/MoE/SSM experiments, embedded SMT & Unikernel experiments (`061dc0e`).
- **M32 – M36 (Capabilities, CRDTs & V4 Textbooks):** Object-capability (`std/cap.ax`), deterministic `Crdt[T]` (`std/crdt.ax`), GPU UI/SDF exploration, and updated language documentation (`b1e05d2`).
