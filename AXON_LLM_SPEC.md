# Axon v1.0.0 Compact Language Specification for AI Coding Agents (`AXON_LLM_SPEC.md`)

Axon (`.ax`) is a zero-GC, zero-lifetime-annotation (`0 'a`), self-hosted native systems language with sub-millisecond static verification (`axc check --json <file.ax>`) and an embedded JSON-RPC 2.0 MCP server (`axc mcp`).

## 1. Fast AI Feedback Loop
- Verify syntax, types, Perceus ownership, taint, `@no_alloc`, `par_for` data races, and tensor shapes in `< 1 ms`:
  `axc check --json path/to/file.ax`
- Build and run:
  `axc run path/to/file.ax`
  `axc test path/to/file.ax`
  `axc build path/to/file.ax -o app.exe --target win-x64`
- 11 native targets (`--target`): `win-x64`, `linux-x64`, `linux-arm64`, `macos-arm64`, `macos-x64`, `wasm32`, `unikernel-x64`, `freestanding-x64`, `freestanding-riscv64`, `spirv`, `ptx`.

## 2. Core Syntax Rules (Write Idiomatic Axon on First Try)
1. **Bindings:**
   - Immutable by default: `x := 42` or `x: i64 = 42`
   - Mutable: `mut total := 0` (mutating without `mut` triggers `E0301`)
2. **Types:**
   - Primitives: `i32`, `i64`, `u8`, `u32`, `u64`, `usize`, `f32`, `f64`, `bool`, `str`, `dec` (`10.25d` exact 4-decimal scaled `i64`)
   - SIMD Math: `vec3`, `vec4`, `quat`, `mat4` (`std::math3d`)
   - Owned Buffers & Tables: `ByteBuf`, `IntBuf`, `Table[T]` (`Table[Particle]::with_capacity(n)`)
   - Security: `tainted[str]` (`std::taint`)
   - AI Tensors: `Tensor[f32, [B, S, D]]`, `ShardedTensor[f32, [M, N]]` (`std::ai_engine`)
3. **Boolean & Arithmetic Operators:**
   - Short-circuit words: `and`, `or`, `not` (never `&&`, `||`, `!` for booleans; `!` is reserved for `T ! Error`)
   - Wrapping integer ops: `+%`, `-%`, `*%`
4. **Functions, Named Arguments & Struct Constructors:**
   - Named arguments and struct fields ALWAYS use `=` (not `:`):
     `ev := TradeEvent(order_id = 9001, symbol = "BTCUSD", filled = true)`
   - Mutable inout parameter: `fn update(mut acc: i64)` called as `update(mut acc)`
   - Linear move parameter: `fn consume(move buf: ByteBuf)` called as `consume(move buf)`
5. **Zero-Cost Error Unions (`T ! Error`):**
   ```axon
   fn parse_port(raw: i64) -> i64 ! i64 {
       if raw <= 0 or raw > 65535 { return err(400) }
       return ok(raw)
   }
   fn boot() -> i64 ! i64 {
       p1 := parse_port(-1) or 8080      // fallback default
       p2 := parse_port(443) or return   // propagate error in RAX/RDX registers
       return ok(p1 + p2)
   }
   ```

## 3. Perceus Linear Ownership & Concurrency Rules
- Never store `&T` inside a `struct` (`E0101`) or return `&T` from a function (`E0102`).
- After `move x`, reading `x` is rejected (`E0201`). Moving an outer variable inside `while`/`for` is rejected (`E0203`).
- `par_for p in table { p.px += p.vx * dt }` runs across OS worker threads + CPUID-dispatched `SSE2 / AVX2 / AVX-512` (`NEON` on ARM64). Mutating outer variables inside `par_for` is rejected (`E0204`).
- `async fn` / `await expr` lowers to zero-heap stackless coroutines backed by Win64 `IOCP` (`std::async_io`).

## 4. Compile-Time Execution (`comptime`) & Direct C FFI (`@cImport`)
- `comptime fn`, `comptime CONST := expr`, `comptime { comptime_assert(cond, "msg") }`, `comptime expr`, and `to_json(struct_val)` (zero reflection).
- Direct C header import without bindings:
  `@cImport("sqlite3.h", link = "sqlite3.dll")`
- Shared libraries / Python Zero-Copy `DLPack` extensions:
  `@py_export fn kernel(dlt_ptr: i64) -> f64 { ... }` compiled with `axc build file.ax --shared -o mod.dll`.

## 5. Domain Decorators & Error Codes
- `@no_alloc`: Forbids heap allocations inside HFT / audio / kernel hot paths (`E0601`).
- `@nan_guard`: Hardware IEEE-754 `0x7FF` exponent trap on `NaN`/`Inf` in AI training/inference (`exit 254` with exact source line).
- `@gpu_kernel`, `@tile(64, 4, 1)`, `@pipeline_stages(3)`: Compiles directly to Khronos `SPIR-V 1.6` (`.spv`) and NVIDIA `PTX 8.0` (`.ptx`).
- `@shield`, `@whitebox`, `@secret`: Polymorphic `.text` MAC verification, $\text{GF}(2^8)$ S-box, rolling stack cipher.
- `@test`: Discovered and executed by `axc test <file.ax>` with built-in `assert(cond)`.
- **Diagnostic Codes:** `E0010`/`E0020` (Lex/Parse), `E0101`-`E0102` (Second-class refs), `E0201`-`E0204` (Perceus move / loop move / `par_for` race), `E0301` (Immutability), `E0401`-`E0407` (Type / return / symbol / arity / missing return / `/ 0` / loop `break`), `E0501` (Capability `std::cap`), `E0502` (Unsanitized `tainted[str]` sink), `E0601` (`@no_alloc` violation), `E0602` (Symbolic `Tensor` shape mismatch), `E0603`-`E0604` (`comptime` VM error / `comptime_assert`), `E0605` (Missing `@cImport` header).
