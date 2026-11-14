# Axon Programming Language (`.ax`) v5.0

Axon is a self-hosted native systems programming language designed around a single goal: **Python-level simplicity with bare-metal speed, tiny standalone binaries, and compile-time memory and capability safety**.

The Axon compiler (`bin/axc.exe`, ~547 KB) is written 100% in Axon (`compiler_ax/`) and emits native `x86_64` + 256-bit `AVX2/FMA3` machine code and standalone `PE32+` executables directly in memory, without `LLVM`, `Clang`, `GCC`, `rustc`, or external linkers.

- **Tiếng Việt:** Xem giáo trình đầy đủ tại [`LEARN_AXON_VI.md`](LEARN_AXON_VI.md)
- **English:** Read the complete language guide at [`LEARN_AXON_EN.md`](LEARN_AXON_EN.md)

---

## Core Architecture

1. **Pythonic Zero-Ceremony Syntax:** `:=` type inference, `and` / `or` / `not` short-circuit logic, zero-heap `f"..."` formatted strings, named and default arguments, `|>` pipelines, exact fixed-point `dec` (`49.50d`), `T ! Error` register-level error unions (`or`, `or return`, `?`), and stack-polymorphic Duck `interface` types.
2. **Static Perceus Memory Management (0 GC, 0 `'a` Lifetimes):** Second-class references (`E0101`/`E0102`), linear `move` tracking (`E0201`/`E0202`), deterministic `VirtualFree` insertion on every exit path, and Functional-But-In-Place (FBIP) zero-syscall memory reuse.
3. **Native Columnar `Table[T]` + Automatic AVX2 FMA3 SIMD:** Struct tables are laid out automatically as contiguous Structure-of-Arrays (SoA) columns and compiled into 4x-unrolled 128-byte `VFMADD231PS` hardware loops.
4. **OS-Standard Binary Hardening & Anti-Tamper:** Strict `W^X` sections (0 RWX pages), High-Entropy 64-bit ASLR + Hardware DEP/NX, Intel CET (`ENDBR64`), `.pdata`/`.xdata` SEH unwind metadata, low `.text` Shannon entropy (`5.61 < 6.50`), `@shield` live `.text` FNV-1a verification, `@whitebox` $\text{GF}(2^8)$ bijective lookup tables, `@secret` stack decryption + auto-wipe, and compile-time Object-Capability security (`std::cap`).

---

## Quickstart

```powershell
# Check types, Perceus ownership, and capability safety
.\bin\axc.exe check examples/hello.ax

# Compile directly to a standalone ~5.5 KB native executable in ~2 ms
.\bin\axc.exe build examples/hello.ax -o bin/hello.exe
.\bin\hello.exe

# Run the full v5.0 showcase
.\bin\axc.exe build examples/v5_showcase.ax -o bin/v5_showcase.exe
.\bin\v5_showcase.exe
```

---

## Verified Physical Benchmark

Measured on **Intel Core i5-10400F (6C/12T), 32 GB DDR4, Windows 10 x64** running [`examples/bench_soa_simd.ax`](examples/bench_soa_simd.ax): **1,048,576 64-byte entities (`16 x f32`) x 200 simulation steps** (`209,715,200` updates = `838.86M` FLOPs, verified output `px = 26.00, py = 52.00`):

| Language & Compiler | Compile Time | Standalone Binary | Execution Time (Best of 3) | Axon Speedup | Axon Size Advantage |
| :--- | :---: | :---: | :---: | :---: | :---: |
| **Axon v5.0 (`bin/axc.exe` Self-Hosted)** | **1 ms** (11 ms OS) | **6.50 KB (`6,656 B`)** | **216.34 ms** (`216,337 µs`) | **1.00x** | **1.00x** |
| **Go (`go build -ldflags="-s -w"`)** | 13,680 ms | 1,649.50 KB (`1,689,088 B`) | 1,340.98 ms (`1,340,982 µs`) | **6.20x faster** | **253.8x smaller** |
| **Rust (`rustc -O3 -C target-cpu=native -C strip=symbols`)** | 519 ms | 155.50 KB (`159,232 B`) | 1,384.79 ms (`1,384,789 µs`) | **6.40x faster** | **23.9x smaller** |
| **C++23 (`MSVC cl /O2 /arch:AVX2`)** | ~1,100 ms | 140.50 KB (`143,872 B`) | 1,436.37 ms (`1,436,374 µs`) | **6.64x faster** | **21.6x smaller** |
| **Java 21 (`OpenJDK 21 HotSpot C2 JIT`)** | 959 ms | 1.93 KB + ~200 MB JRE | 2,389.97 ms (`2,389,968 µs`) | **11.05x faster** | **No 200 MB JVM** |
| **Python 3.11 (`__slots__`)** | Interpreted | ~100 MB Runtime | ~46,323.56 ms (scaled from 5 steps) | **214.12x faster** | **No Python VM** |

---

## Repository Structure

```text
├── bin/axc.exe                 # Self-hosted standalone Axon v5.0 compiler (~547 KB)
├── compiler_ax/                # 100% pure Axon self-hosted compiler source
│   ├── tokens_ast.ax           # Token kinds, packed Ty, string pool & AST arena
│   ├── lexer.ax                # Byte-level lexer (f-strings, dec, IEEE-754, attributes)
│   ├── parser.ax               # Recursive-descent & Pratt parser
│   ├── sema_perceus.ax         # Type/capability safety verifier, Perceus & SoA SIMD planner
│   ├── linker_pe.ax            # Direct Win64 PE32+ executable builder (W^X, .pdata, ASLR)
│   ├── codegen_runtime.ax      # Zero-CRT Win64 runtime helpers, @shield & @whitebox GF(2^8)
│   ├── codegen_calls.ax        # Intrinsics, Fast/General ABIs & Duck interface dispatch
│   ├── codegen_x86_64.ax       # Direct x86_64 + 4x-unrolled AVX2 FMA3 codegen & tree-shaker
│   └── main.ax                 # CLI driver & recursive module loader
├── std/                        # Pure Axon standard library (cap, decimal, crdt, crypto)
├── examples/                   # Runnable examples (hello.ax, v5_showcase.ax, bench_soa_simd.ax)
├── tests/                      # 8 native execution pass tests + 11 compile-time safety gates
├── bootstrap_archive/          # Zero-dependency Stage-0 native bootstrap compiler (axc_native)
├── LEARN_AXON_VI.md            # Official Vietnamese learning guide (v5.0)
└── LEARN_AXON_EN.md            # Official English learning guide (v5.0)
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
```
