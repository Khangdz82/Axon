# MEMORY.md — Persistent Architecture, Compiler, Perceus & Stdlib Reference (`axc` v1.1.1)

> Canonical engineering memory bank for **Humans** and **AI Coding Agents** working on the Axon compiler (`compiler_ax/`), standard library (`std/`), desktop UI engine, and native applications.

---

## 1. Golden Project Invariants

1. **100% Pure Axon Self-Hosting:** Every compiler pass (`compiler_ax/*.ax`, `18` files, `37,570` lines) and standard library module (`std/*.ax`, `103` files, `39,719` lines) is written in pure `.ax` with zero C/C++/Rust/LLVM/Python runtime or build dependencies.
2. **Deterministic 3-Stage Bootstrap Gate:** Any edit to `compiler_ax/*.ax` MUST pass:
   - `bin/axc.exe build compiler_ax/main.ax -o axc_s1.exe`
   - `axc_s1.exe build compiler_ax/main.ax -o axc_s2.exe`
   - `axc_s2.exe build compiler_ax/main.ax -o axc_s3.exe`
   - `SHA256(axc_s2.exe) == SHA256(axc_s3.exe)` (`2,357,248` bytes; current verified hash: `D6064066AD2600074A31C52158E73BE2BFA09761B56BD164475F4C7C6DE01A96`).
3. **Full Regression Gate:** `76/76` `tests/pass/*.ax` + `37/37` `tests/fail_compile/*.ax` must pass before promoting changes.

---

## 2. Compiler Architecture (`compiler_ax/` — 18 Modules)

| Module | Responsibility |
| :--- | :--- |
| `tokens_ast.ax` | Token IDs (`tk_*`), AST node kinds (`nk_*`, `100..165`), operator IDs (`op_*`, `aop_*`), type tags (`ty_*`), target IDs (`tgt_*`, `0..10`), `ushr64`/`popcnt64`/span packing |
| `lexer.ax` | Single-pass UTF-8 / indent-tolerant / brace-delimited tokenizer, f-string interpolation, hex/bin/oct/float/dec literals |
| `parser.ax` | Recursive-descent + Pratt precedence parser (max recursion depth `256` -> `E0020` guard against stack overflow) |
| `c_import.ax` | Built-in `.h` C header parser for `@cimport` zero-wrapper C struct/function binding |
| `generics.ax` | Zero-cost generic struct/function (`Box[T]`, `Option[T]`, `Pair[A,B]`) AST cloner & monomorphizer |
| `comptime.ax` | Compile-time constant evaluator, `comptime_assert` (`E0604`), and LUT synthesizer |
| `sema_perceus.ax` | Type checker, Perceus linear ownership (`E0201`–`E0204`), `&T` escape blocker (`E0101`–`E0102`), immutability (`E0301`) |
| `sema_effects.ax` | Capability attenuation (`E0501`), taint flow (`E0502`), strict `unsafe` boundary (`E0503`), `@no_alloc` (`E0601`), symbolic tensor shapes (`E0602`) |
| `diagnostics.ax` | Human & structured `--json` error formatter (`E0001..E0604`), static advisor (`axc advise`), source snippet renderer |
| `codegen_runtime.ax` | Multi-OS x86-64 emitters, runtime helpers (`h_alloc_mem`, `h_free_mem`, `h_str_eq`, `h_sec_trap` PIE stack backtrace, `h_shield_verify`), `.rdata` interning, `.data` layout |
| `codegen_calls.ax` | Intrinsics (`ptr_read_*`, `ptr_write_*`, `buf_*`, `ibuf_*`, `mem_stride_zero_u8`, `str_eq`), 16-arg Win64/SysV `c_call_v` FFI, SIMD `math3d`, AI tensor/pool intrinsics |
| `codegen_x86_64.ax` | Function prolog/epilog, GPR/XMM local promotion, scalar/CMOV peepholes, SoA `Table[T]` SSE2/AVX2/AVX-512 codegen, `par_for` work-stealing, `8×i64` & `64B VPSADBW` AVX2 loop auto-vectorizers |
| `codegen_arm64_rv64.ax` | Native AArch64 (`linux-arm64`, `macos-arm64` + NEON) and `RV64GC` (`freestanding-riscv64`) machine-code backends |
| `backend_gpu_wasm.ax` | Direct WebAssembly (`wasm32` WASI + `0xFD` SIMD128), Khronos `SPIR-V 1.6`, and NVIDIA `PTX 8.0` GPU emitters |
| `linker_pe.ax` | In-memory Win64 `PE32+` `.exe`/`.dll` + COFF `.obj`/`.lib` linker (unlimited dynamic DLL `.idata` IAT/ILT, `.edata`, `.pdata` SEH unwind, strict `W^X`) |
| `linker_unix_bare.ax` | Linux/FreeBSD `ELF64`, Apple `Mach-O 64` (16 KB page-aligned), and `Multiboot2` Unikernel linkers |
| `pkg_mcp.ax` | Package manager (`axon.toml`/`axon.lock`) & built-in MCP JSON-RPC server (`axc mcp`: `axon_check`, `axon_docs`, `axon_spec`, `axon_fix`) |
| `main.ax` | Top-level CLI driver (`check`, `run`, `build`, `test`, `advise`, `fmt`, `init`, `mcp`), transitive module loader, and `.map` symbol emitter |

---

## 3. Memory Model: Perceus 0-GC + 8-Class Slab Allocator

### 3.1 Perceus Ownership Rules (Compile-Time Enforced)
- **Owned linear types:** `ByteBuf`, `IntBuf`, `Table[T]`, and any `struct` containing an owned field (e.g., `DynIntVec`, `StrVec`, `DynMap`, `StrMap`, `IntDeque`, `BitSet`, `Graph`, `UiDoc`).
- **Scope exit:** Every owned variable MUST either be consumed via `move` (e.g., `buf_free(move b)`, `buf_to_owned_str(move b)`, `smap_free(move m)`) or returned (`return b`).
- **Loop move ban (`E0203`):** Moving an outer owned variable inside a `while`/`for` loop is rejected at compile time (prevents use-after-free on iteration 2).
- **Second-class borrows (`E0101`/`E0102`):** `&T` references cannot be stored inside structs or returned from functions, guaranteeing zero dangling pointers without Rust lifetime annotations.

### 3.2 Native Runtime Slab Allocator (`codegen_runtime.ax`)
- **8 size classes (`tag = 1..8`):** `64 B`, `128 B`, `256 B`, `512 B`, `1 KB`, `2 KB`, `4 KB`, `64 KB` (up to `64` cached blocks per class in `.data` freelists; larger allocations route directly to `HeapAlloc` / `mmap`).
- **Metadata header (64-byte aligned user pointer `RDX`):**
  - `[RDX - 8]`: raw OS base pointer (for `HeapFree`)
  - `[RDX - 16]`: slab `class_tag` (`1..8`, or `0` for direct OS block)
  - `[RDX - 24]`: live canary (`1` = allocated, `0` = freed -> traps immediately on double-free or unaligned free!).
- **Zero-LOCK single-thread fastpath (`data_off_mt_active = 0x104`):**
  - `mt_active` starts at `0` (ordinary `MOV`/`CMP` slab pop/push -> `1 ms` for `200,000 × 4 KB` alloc+free!).
  - Set to `1` BEFORE `CreateThread` / `clone` in `par_for`, `ai_pool`, and `spawn`, automatically activating `XCHG` acquire / `MOV` release spinlocks (`data_off_slab_lock = 0x100`) for thread-safe multi-core allocation.
- **Important Pitfall for `unsafe` Raw Buffers:** Blocks popped from the slab freelist reuse prior memory; `buf_new`/`ibuf_new` track initialized `len` (`[ptr + 0]`), whereas raw `mem_alloc_aligned` callers that rely on zeroed memory (such as radix-sort histograms!) MUST explicitly call `mem_set(ptr, 0, bytes)`.

---

## 4. x86-64 Codegen & AVX2 Register Conventions

- **Promoted Local GPRs (`pr = 1..10`):**
  - `1=RBX`, `2=R12`, `3=R13`, `4=R14`, `5=R15`, `6=RSI`, `7=RDI`, `8=R10`, `9=R11`, `10=R8`.
  - **`RAX`, `RCX`, `RDX`, `R9` are NEVER assigned to promoted locals!** Any inline intrinsic or loop kernel using ONLY `{RAX, RCX, RDX, R9}` is guaranteed zero promoted-GPR collisions.
- **Promoted Local `f64` XMMs (`fr = 1..8`):**
  - Assigned to `XMM6..XMM13` (which are callee-saved on Win64!).
  - **`XMM0..XMM5` (`YMM0..YMM5`) are caller-saved volatile on BOTH Win64 and SysV!**
  - Any AVX2 kernel using `YMM6..YMM15` (such as `try_emit_avx2_nbody_while_ax` or 2x-interleaved `8×i64` `try_emit_avx2_for_range_ax`) MUST save/restore `XMM6..XMM15` in RBP stack slots around the loop, and MUST emit **`VZEROUPPER` (`C5 F8 77`)** on every exit from 256-bit `YMM` state.
- **`.rdata` Intern Record Tags (`cg.interns`, stride 4 `[tag, f1, f2, roff]`):**
  - `1`: Pool string slice `(soff, slen)`
  - `3`: External DLL import symbol
  - `4`: Comptime LUT
  - `5`: Scalar `u64`/`f64` constant (`align16` optional)
  - `7`: 32-byte aligned `4×u64` broadcast vector `[bits, bits, bits, bits]` (`cg_intern_u64x4_rdata`)
  - `8`: 32-byte aligned `4×u64` linear sequence `[base, base+step, base+2*step, base+3*step]` (`cg_intern_vec4_seq_rdata`)
- **Active compiler loop/string fastpaths:**
  1. `try_emit_fast_rec_fn_ax`: Leaf shrink-wraps `fib`-like binary recursion up to `n <= 7` (`LEA` base formulas, `0` stack frames for `n <= 7`, `--shield` `JO` preserved).
  2. `try_emit_avx2_u8_sum_while_ax` & `nk_for_range` `u8` sum: `64 B/iter` `2 × VMOVDQU + 2 × VPSADBW + 2 × VPADDQ` + scalar tail.
  3. `try_emit_avx2_for_range_ax`: `8×i64` (`2×YMM` interleaved) auto-vectorizer for `for i in start..end` supporting both direct `iv` expressions and `z = z +% C0` linear recurrences (`+`, `-`, `* C`, pow2 `/` & `%`, `<<`, exact 64-bit signed `>>`, `ushr64`, `&`, `|`, `^`).
  4. `h_str_eq`: Inline `len`/`ptr` check at call site + single-shot dual-YMM `32..64 B` `VPCMPEQB + VPAND + VPMOVMSKB` + `>64 B` 32B/iter loop + overlapped tail.

---

## 5. Critical Axon Syntax & Pitfalls (Never Forget!)

1. **Reserved Keywords:** `fn`, `mut`, `move`, `if`, `else`, `while`, `for`, `in`, `par_for`, `return`, `break`, `continue`, **`pass`**, `struct`, `interface`, `use`, `as`, `and`, `or`, `not`, `true`, `false`, `ok`, `err`, `spawn`, `await`, `comptime`, `unsafe`, `isolated`.
   - **Pitfall:** NEVER name a variable or parameter `pass` (it is `tk_pass`, Python-style no-op statement -> `E0020`!). Use `bp`, `step`, or `round`.
2. **Boolean Operators:** Always write `and`, `or`, `not` (NEVER `&&`, `||`, `!`).
3. **If-Expressions:** `x := if cond { a } else { b }` requires braces `{ ... }` on BOTH branches; for chained conditions in expression position write `else { if c2 { b } else { d } }` (statement `if ... else if ...` works without outer braces).
4. **Error Unions (`T ! i64`):**
   - Return `ok(v)` or `err(code)`.
   - Unwrap with `v := call() or return`, `v := call() or return -1`, or `v := call() or fallback_expr` (NEVER `or { block }`).
5. **Wrapping vs Checked Arithmetic:**
   - Standard `+`, `-`, `*` trap on signed 64-bit overflow when compiled with `--shield` (including `INT64_MIN - 1`!).
   - For intentional modular/hash/PRNG wrap-around (`SplitMix64`, `FNV-1a`, `INT64_MIN -% 1`), ALWAYS use `+%`, `-%`, `*%` so code runs identically under both normal and `--shield` builds!
6. **Struct Construction & Named Args:** Use `=` (NEVER `:`): `Url(scheme = "https", userinfo = "", host = "example.com", port = 443, path = "/", query = "", fragment = "")`.
7. **Mutable & Move Call Sites:** Caller MUST annotate `mut` and `move` explicitly (`dvec_push(mut v, 10)`, `dvec_free(move v)`).

---

## 6. Standard Verification Commands

```powershell
# 1. Fast static JSON check (< 35 ms)
.\bin\axc.exe check --json path/to/file.ax

# 2. Hostile v1.2 stdlib + AVX2 differential + security suite (AVX2, --shield, and --simd=scalar)
.\bin\axc.exe run tests/pass/53_stdlib_v2_and_avx2_hostile.ax
.\bin\axc.exe run tests/pass/52_hostile_zero_trust_suite.ax
.\bin\axc.exe run tests/pass/51_desktop_ui_v2_and_sec.ax

# 3. Full regression sweep (76 pass + 37 fail_compile)
Get-ChildItem tests\pass\*.ax | ForEach-Object { .\bin\axc.exe run $_.FullName }
Get-ChildItem tests\fail_compile\*.ax | ForEach-Object { .\bin\axc.exe check $_.FullName }
```
