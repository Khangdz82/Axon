# Axon Porting Rules for AI Coding Agents (`.agents/rules/axon-porting-rules.md`)

> **MANDATORY ZERO-FALLBACK PORTING CONTRACT (`axc` v1.2.0 — 115 Native `std/*.ax` Modules):**
> When translating, migrating, or porting ANY codebase or algorithm from **Python, Rust, C/C++, Go, Java, or TypeScript/Node.js** into Axon (`.ax`), you MUST write **100% pure Axon** using Axon's built-in 115 `std/*.ax` modules and `std::ffi`. NEVER shell out to Python/Node/C helper scripts, NEVER generate `.o`/`.c` glue wrappers, and NEVER skip differential verification.

---

## 1. Six Mandatory Syntax & Ownership Translation Laws

1. **Logical Operators & Keywords:**
   - Translate `&&`, `||`, `!` (C/C++/Rust/Go/Java/TS) and Python `not`/`and`/`or` to **`and`**, **`or`**, **`not`**.
   - NEVER name a variable, field, or parameter `pass` (`pass` is a reserved no-op statement -> `E0020`; rename to `passwd`, `pass_idx`, or `secret`).
   - Float literals MUST be decimal (`0.000000000001` or `num_pow2_int(-50)`), NOT scientific `1e-12` (`E0020`).
   - Chained `if` expressions require parentheses inside `else` (`if a { 1 } else { (if b { 2 } else { 3 }) }`) or prefer clean statement-form `if a { x = 1 } else if b { x = 2 } else { x = 3 }`.

2. **Error Unions (`T ! i64`) Instead of Exceptions / `Result<T, E>` / `(T, error)`:**
   - Signature: `fn parse_port(s: str) -> i64 ! i64`
   - Return: `return ok(val)` or `return err(400)`.
   - Unwrap:
     - Propagate error: `p := parse_port(raw) or return` (or `or return err(400)` / `or return -1`).
     - Fallback value: `p := parse_port(raw) or 8080`.
   - **FORBIDDEN:** `or { ... }` block syntax (`E0020`). `or` takes a single expression or `return [expr]`.

3. **Perceus 0-GC Deterministic Ownership (Replaces GC in Go/Java/Python/TS & Borrow Checker Lifetimes in Rust):**
   - Immutable by default (`x := 10`); mutable bindings require `mut` (`mut x := 10`), and mutable reference args require `mut` at BOTH declaration and call site (`fn step(mut st: State)` -> `step(mut st)`).
   - Owned heap containers (`ByteBuf`, `IntBuf`, `MatF64`, `DataFrame`, `StrVec`, `StrMap`, `DynMap`, `KvStore`, `BigInt`) MUST be freed on every exit path via their `*_free(move x)` / `buf_free(move b)` / `buf_to_owned_str(move b)` destructor.
   - NEVER move an outer owned variable inside a `while`/`for` loop (`E0203`), and NEVER store or return `&T` references (`E0101`/`E0102`).
   - Prefer **Flat Arena Structs** (e.g., `MatF64` node tables, `StrVec` string pools, `IntBuf` index arrays) over pointer-linked object graphs (`Box<Node>` / Java object trees) — this is 5–20x faster in CPU L1/L2 cache AND trivially Perceus-clean!

4. **Overflow & Safety Semantics (`--shield` Compatibility):**
   - Standard `+`, `-`, `*` on `i64` trap signed overflow under `--shield`.
   - When porting cryptographic hashes, PRNGs, CRCs, or bitwise mixers (SHA-256, BLAKE2b, Argon2id, SplitMix64, FNV-1a, Murmur), ALWAYS use modular wrap-around operators **`+%`**, **`-%`**, **`*%`** and unsigned logical shift **`ushr64(x, k)`**!

5. **Closures / Callbacks -> Reverse-Communication Steppers:**
   - When porting Python/Rust/JS higher-order numerical or iterator APIs (`scipy.optimize.brentq(f, a, b)`, `odeint`, `L-BFGS`), use Axon's zero-allocation **Reverse-Communication Stepper Structs** (`BrentSolver`, `Rk4State`, `LbfgsState` in `std::numeric`) so callers evaluate `f(st.x_eval)` inline with full access to local state and zero heap closures.

6. **Sub-Millisecond Verification Gate Before Completion:**
   - Every ported module MUST pass all 4 gates with zero temp files left behind:
     1. `bin/axc.exe check --json <file.ax>` -> `{"ok":true,"diagnostics":[]}`
     2. `bin/axc.exe test <file.ax>` -> `0` failures
     3. `bin/axc.exe build <file.ax> -O2 --shield -o app.exe` -> traps `0` overflows/leaks
     4. Behavioral equivalence check against recorded golden I/O (`axon_anything.ax diff`).
