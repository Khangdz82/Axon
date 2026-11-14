# Axon Programming Language (`.ax`) v6.0

Axon is a self-hosted native systems programming language engineered around a single goal: **Python-level readability with bare-metal hardware supremacy, kilobyte-scale standalone executables, 24/7 zero-leak server stability, and compile-time memory + anti-crack security**.

The Axon v6.0 compiler (`bin/axc.exe`, **533.50 KB** / `546,304 bytes`) is written 100% in Axon (`compiler_ax/`) and emits native `x86_64` + 256-bit `AVX2/FMA3` machine code and standalone `PE32+` executables directly in memory in milliseconds, without `LLVM`, `Clang`, `GCC`, `MSVC`, `rustc`, `CRT`, or external linkers. Total installed toolchain footprint (`axc.exe` + `std/*.ax`) is **549.64 KB**.

- **Tiếng Việt:** Xem giáo trình 13 chương đầy đủ tại [`LEARN_AXON_VI.md`](LEARN_AXON_VI.md)
- **English:** Read the complete 13-chapter language guide at [`LEARN_AXON_EN.md`](LEARN_AXON_EN.md)

---

## Core Architecture (v6.0)

1. **Pythonic Zero-Ceremony Syntax:** `:=` type inference, `and` / `or` / `not` short-circuit logic, zero-heap `f"..."` formatted strings, named and default arguments, `|>` pipelines, exact fixed-point `dec` (`49.50d`), `T ! Error` register-level error unions (`or`, `or return`, `?`), and stack-polymorphic Duck `interface` types.
2. **Exhaustive Static Verification + Perceus Loop Self-FBIP (0 GC, 0 `'a` Lifetimes, 0 Leaks):** Rejects 17 classes of safety errors at compile time (`E0101` through `E0501`, including branch-aware `E0201`, loop-outer move `E0203`, symbol/arity/mode checks `E0403`-`E0404`, control-flow return completeness `E0405`, constant `/ 0` `E0406`, and loop-scoped `break`/`continue` `E0407`). Perceus **Loop Self-FBIP** hoists and resets per-iteration buffers in place (`5,000,000` continuous server cycles / `340 GB` logical allocations in `69 ms`, flat `3.15 MB` RAM, `0` leaks).
3. **Columnar `Table[T]` + Dead-Column Elimination + Polyhedral Temporal AVX2 FMA3:** Struct tables automatically strip unread columns across the whole program (`16 -> 4` active columns) and tile time-stepped loops into **256-byte spatial blocks (`YMM1..YMM8`)** fused in CPU registers.
4. **Polymorphic `@shield` v2, `@whitebox` $\text{GF}(2^8)$ & AV-Clean PE32+ Binaries:** Per-binary seeded nonlinear `.text` MAC split into 16-byte `(share0, share1)` shares verified at `_start` and `@shield` prologues (live 1-bit `.text` tamper test immediately halts with `exit 255`, `0.00%` loop slowdown), rolling-state `@secret` stack encryption + wipe, constant-time `verify_tag`, strict `W^X` (0 RWX pages), `.pdata`/`.xdata` SEH, High-Entropy ASLR + DEP, and natural `.text` Shannon entropy (`5.08 - 5.61 < 6.50`).

---

## Quickstart

```powershell
# Install ultra-lightweight toolchain (~549.64 KB total) to ~/.axon
.\install.ps1

# Check types, Perceus ownership, return paths, and capability safety
.\bin\axc.exe check examples/hello.ax

# Compile directly to a standalone 3.50 KB (3,584 B) native executable in < 1 ms
.\bin\axc.exe build examples/hello.ax -o bin/hello.exe
.\bin\hello.exe

# Run benchmarks & full v6.0 showcase
.\bin\axc.exe build examples/v5_showcase.ax -o bin/v5_showcase.exe
.\bin\v5_showcase.exe
```

---

## Verified Physical Benchmarks (Measured Live on Host)

Hardware: **Intel Core i5-10400F (6C/12T), 32 GB DDR4, Windows 10 x64**.

### 1. Toolchain & `hello.exe` Footprint

| Language & Toolchain | Installed Toolchain Size | Clean Build Time | Standalone `hello` Binary | Binary Size Ratio vs Axon |
| :--- | :---: | :---: | :---: | :---: |
| **Axon v6.0 (`bin/axc.exe` Self-Hosted)** | **0.54 MB (`549.64 KB`)** | **< 1 ms** (`8.5 ms` OS) | **3.50 KB (`3,584 B`)** | **1.0x** |
| **C++23 (`MSVC 14.51 /O2 /arch:AVX2`)** | ~2,500 MB | ~1,660 ms | 108.50 KB (`111,104 B`) | **31.0x larger** |
| **Rust (`rustc -O3 -C target-cpu=native -C strip=symbols`)** | ~1,200 MB | 157.8 ms | 127.00 KB (`130,048 B`) | **36.3x larger** |
| **Go (`go build -ldflags="-s -w"`)** | ~500 MB | 452.7 ms | 1,636.00 KB (`1,675,264 B`) | **467.4x larger** |
| **Java 21 (`OpenJDK 21.0.13`)** | ~320 MB | ~620 ms | `.class` + ~320 MB JRE | **Requires JVM** |
| **Python 3.11** | ~100 MB | Interpreted | Script + ~100 MB Runtime | **Requires Python VM** |

### 2. Workload A: Entity Simulation (`examples/bench_soa_simd.ax`)
**1,048,576 64-byte entities (`16 x f32`) x 200 steps** (`209,715,200` entity updates = `838.86M` FLOPs, verified output `px = 26.00, py = 52.00`):

| Language & Compiler | Standalone Binary | Execution Time (Best of 3) | Axon v6.0 Speedup |
| :--- | :---: | :---: | :---: |
| **Axon v6.0 (`axc.exe`, SoA + Dead-Col Elim + Temporal AVX2)** | **5.50 KB (`5,632 B`)** | **8.61 ms** (`8,612 µs`) | **1.00x (Baseline)** |
| **Rust (`rustc -O3 -C target-cpu=native -C strip=symbols`)** | 155.50 KB (`159,232 B`) | 1,195.50 ms (`1,195,499 µs`) | **138.8x faster** |
| **C++23 (`MSVC cl /O2 /arch:AVX2 /fp:fast`)** | 139.00 KB (`142,336 B`) | 1,207.17 ms (`1,207,169 µs`) | **140.2x faster** |
| **Go (`go build -ldflags="-s -w"`)** | 1,650.50 KB (`1,690,112 B`) | 1,211.72 ms (`1,211,719 µs`) | **140.7x faster** |
| **Java 21 (`OpenJDK 21.0.13 HotSpot C2 JIT`)** | `.class` + ~320 MB JRE | 1,669.26 ms (`1,669,263 µs`) | **193.8x faster** |
| **Python 3.11 (`__slots__`)** | ~100 MB Runtime | 32,867.40 ms (scaled from 5 steps) | **3,816.5x faster** |

### 3. Workload B: Enterprise Packet Buffer + Financial Settlement (`examples/bench_enterprise_suite.ax`)
**5,000,000 per-request `4 KB` packet buffers (`20 GB` logical heap traffic) + exact fixed-point tax/fee ledger settlement** (verified output `$448,750,000.00`):

| Language & Compiler | Memory / Decimal Model | Execution Time (Best of 3) | Axon v6.0 Speedup |
| :--- | :---: | :---: | :---: |
| **Axon v6.0 (`axc.exe`, Perceus Loop Self-FBIP + `dec`)** | **0 GC, 0 Syscall Reuse + 64-bit `dec`** | **50.81 ms** (`50,808 µs`) | **1.00x (Baseline)** |
| **Rust (`rustc -O3 -C target-cpu=native`)** | Heap `Vec::with_capacity(4096)` + `i64` | 254.90 ms (`254,904 µs`) | **5.02x faster** |
| **C++23 (`MSVC cl /O2 /arch:AVX2`)** | `std::vector<uint8_t>(4096)` + `int64_t` | 279.09 ms (`279,089 µs`) | **5.49x faster** |
| **Java 21 (`OpenJDK 21 HotSpot C2 JIT`)** | GC `byte[4096]` + `BigDecimal` | 2,150.50 ms (`2,150,500 µs`) | **42.33x faster** |
| **Go (`go build -ldflags="-s -w"`)** | GC `make([]byte, 0, 4096)` + `int64` | 3,482.53 ms (`3,482,531 µs`) | **68.54x faster** |
| **Python 3.11** | `bytearray` + `decimal.Decimal` | 4,017.44 ms (scaled from 250k) | **79.07x faster** |

---

## Repository Structure

```text
├── bin/axc.exe                 # Self-hosted standalone Axon v6.0 compiler (533.50 KB)
├── compiler_ax/                # 100% pure Axon self-hosted compiler source (9,194 lines)
│   ├── tokens_ast.ax           # Token kinds, packed Ty, string pool & AST arena
│   ├── lexer.ax                # Byte-level lexer (f-strings, dec, IEEE-754, attributes)
│   ├── parser.ax               # Recursive-descent & Pratt parser
│   ├── sema_perceus.ax         # Exhaustive verifier (E0101-E0501), Self-FBIP & Temporal SIMD planner
│   ├── linker_pe.ax            # Direct Win64 PE32+ builder (W^X, .pdata, ASLR, Polymorphic @shield v2)
│   ├── codegen_runtime.ax      # On-demand zero-CRT Win64 helpers, @shield v2 & @whitebox GF(2^8)
│   ├── codegen_calls.ax        # Intrinsics (ushr64, exit_process), Fast/General ABIs & Duck dispatch
│   ├── codegen_x86_64.ax       # Compact disp8 x86_64, Register-Tiled AVX2 FMA3, const-inliner & tree-shaker
│   └── main.ax                 # CLI driver & recursive local/installed module loader
├── std/                        # Pure Axon stdlib (cap, decimal, crdt, crypto, collections, json, os)
├── examples/                   # Runnable showcases & benchmarks (hello, v5_showcase, bench_soa_simd, bench_enterprise_suite)
├── tests/                      # 11 native execution pass tests (incl. 5M soak & live anti-crack) + 17 fail_compile gates
├── bootstrap_archive/          # Zero-dependency Stage-0 native bootstrap compiler (axc_native)
├── install.ps1 / install.sh    # Ultra-lightweight (~550 KB) one-step installers
├── LEARN_AXON_VI.md            # Official Vietnamese learning guide (v6.0, 32/32 doc-tests verified)
└── LEARN_AXON_EN.md            # Official English learning guide (v6.0, 32/32 doc-tests verified)
```

---

## Rebuilding & Verifying Deterministic 3-Stage Self-Hosting

```powershell
# 1. Build Stage-0 from zero-dependency Rust bootstrap source
cargo build --release --manifest-path bootstrap_archive/axc_native/Cargo.toml
Copy-Item -Force bootstrap_archive/axc_native/target/release/axc.exe bin/axc0.exe

# 2. Run 3-stage self-hosting bootstrap & verify bit-exact convergence
.\bin\axc0.exe build compiler_ax/main.ax -o bin/axc_stage1.exe
.\bin\axc_stage1.exe build compiler_ax/main.ax -o bin/axc_stage2.exe
.\bin\axc_stage2.exe build compiler_ax/main.ax -o bin/axc_stage3.exe
Get-FileHash bin/axc_stage2.exe, bin/axc_stage3.exe -Algorithm SHA256
# Verified SHA256: 15AB01D09E0835EB750ADAA202981D8FB893E761A6882561A60696169B587AF3
```
