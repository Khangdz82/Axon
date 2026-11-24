# Porting Rust, C, and C++ to Pure Axon

## 1. Ownership & Memory Model Mapping

| Rust / C++ Construct | Pure Axon (`v1.2.0`) |
|---|---|
| `let x = 5; let mut y = 10;` | `x := 5; mut y := 10` |
| `fn update(st: &mut State)` / `update(&mut st)` | `fn update(mut st: State)` / `update(mut st)` |
| `Result<T, i64>` + `?` operator | `T ! i64` + `val := call() or return` |
| `Option<T>` / `std::optional<T>` | `Option[T]` (`some(v)`, `none_opt[T]()`, `unwrap_or` in `use std::vec`) |
| `Vec<i64>` / `std::vector<int64_t>` | `DynIntVec` (`dvec_new`, `dvec_push` in `use std::ds`) or fixed `ibuf_new(cap)` |
| `Vec<String>` / `HashMap<String, String>` | `StrVec` (`svec_*`) / `StrMap` (`smap_*`) in `use std::ds` |
| `x.wrapping_add(y)` / C `uint64_t` `+` | `x +% y`, `x -% y`, `x *% y`, `ushr64(x, k)` |
| `Box<Node>` / pointer-chasing AST or Tree | Flat Arena (`IntBuf` / `MatF64` row table + integer indices) — zero borrow-checker fighting & L1 cache optimal! |
| `extern "C"` / `dlopen` / `LoadLibraryA` | `use std::ffi` (`ffi_open`, `ffi_sym`, `ffi_call0..ffi_call8`, `ffi_cstr`, `ffi_wstr`) |

## 2. SIMD & Compiler Optimization Mapping (`AxIR`)
- **Do NOT write inline assembly or target-specific intrinsics** unless doing bare-metal OS work: `AxIR` (`-O2`/`-O3`) automatically applies:
  - **Pass 3 (`SRoA`):** Registers 1..4-field value structs (`Complex`, `Vec3`, `Rational`, `Pair`).
  - **Pass 6 (`MagicDiv`):** Replaces constant signed `/`, `%` (`x / 10`, `x % 1000000007`) with `IMUL` magic multipliers.
  - **Pass 7 (`RCE`) & Pass 8 (`Auto-SIMD`):** Eliminates bounds checks inside `for i in 0..n` / `while i < n` and emits `AVX2+FMA3`, `ARM64 NEON`, or `WASM SIMD128` automatically!
