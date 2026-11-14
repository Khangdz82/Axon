# The Official Axon v5.0 Programming Guide (Zero to Systems Mastery)

This guide covers **Axon v5.0 (`.ax`)**: a self-hosted native systems programming language engineered to combine **Python/Go-level readability** with **bare-metal hardware speed**, **kilobyte-scale standalone executables**, and **compile-time memory and capability safety**.

Every code example in this guide uses genuine Axon v5.0 syntax supported directly by `bin/axc.exe` and `bin/axc0.exe`.

---

## Table of Contents

1. [Getting Started in 2 Minutes](#chapter-1-getting-started-in-2-minutes)
2. [Variables, Types & Pythonic Ergonomics](#chapter-2-variables-types--pythonic-ergonomics)
3. [Functions, Named/Default Parameters & Pipelines `|>`](#chapter-3-functions-nameddefault-parameters--pipelines-)
4. [Control Flow & Static Bounds Elision `where`](#chapter-4-control-flow--static-bounds-elision-where)
5. [Zero-Cost Register Error Unions `T ! Error`](#chapter-5-zero-cost-register-error-unions-t--error)
6. [Structs, Methods `impl` & Stack-Polymorphic Duck Interfaces](#chapter-6-structs-methods-impl--stack-polymorphic-duck-interfaces)
7. [Exact Fixed-Point Financial Decimals `dec`](#chapter-7-exact-fixed-point-financial-decimals-dec)
8. [Perceus Static Memory Management & FBIP In-Place Reuse](#chapter-8-perceus-static-memory-management--fbip-in-place-reuse)
9. [Columnar `Table[T]` SoA & Automatic 256-Bit AVX2 FMA3 SIMD](#chapter-9-columnar-tablet-soa--automatic-256-bit-avx2-fma3-simd)
10. [Colorless Concurrency `spawn` & Convergent `CRDT` Types](#chapter-10-colorless-concurrency-spawn--convergent-crdt-types)
11. [Object-Capability Security, Anti-Tamper & AV-Clean Binaries](#chapter-11-object-capability-security-anti-tamper--av-clean-binaries)
12. [Self-Hosting Internals, Built-In Intrinsics & Error Reference](#chapter-12-self-hosting-internals-built-in-intrinsics--error-reference)

---

## Chapter 1: Getting Started in 2 Minutes

### 1.1. Why Axon?

Mainstream languages force developers into painful trade-offs:
- **Python / JavaScript:** Easy to learn, but orders of magnitude slower on compute loops and dependent on 100+ MB runtimes.
- **Java / Go:** Garbage collection introduces latency spikes, and deployments require either a ~200 MB JRE or multi-megabyte runtime binaries.
- **C / C++:** Fast and compact, yet vulnerable to buffer overflows, use-after-free bugs, and silent signed integer overflows.
- **Rust:** Memory-safe, but steepens the learning curve with `'a` lifetime annotations, borrow-checker friction, and slow clean builds.

**Axon v5.0** eliminates those trade-offs through four architectural pillars:
1. **Zero-Ceremony Syntax:** Optional semicolons, `and` / `or` / `not` short-circuit logic, zero-heap `f"..."` strings, and named/default arguments.
2. **Static Perceus Ownership (0 GC, 0 `'a` Lifetimes):** Deterministic `VirtualFree` insertion on every exit path and Functional-But-In-Place (FBIP) zero-syscall memory reuse.
3. **Columnar `Table[T]` (SoA) + Automatic AVX2 FMA3:** Struct tables automatically split into contiguous columns and lower `for` loops into 4x-unrolled 256-bit `VFMADD231PS` vector instructions.
4. **~547 KB Self-Hosted Native Toolchain:** Emits `x86_64` machine code and OS-standard `PE32+` binaries (~5.5 KB for basic apps) directly in memory in milliseconds.

### 1.2. Installation & Your First Program

On Windows PowerShell:

```powershell
# Install axc.exe to ~/.axon/bin and add it to PATH
.\install.ps1

# Verify compiler version
.\bin\axc.exe version
```

Create `examples/hello.ax`:

```axon
fn greet(name: str, role: str = "Systems Engineer") {
    println(f"Hello, {name}! Welcome to Axon v5.0 ({role}).")
}

@shield
fn main() -> i32 {
    greet("World")
    greet("Khang", role = "Language Architect")
    return 0
}
```

Type-check and compile it into a standalone native executable:

```powershell
# Verify types, Perceus ownership, and capability rules (~1 ms)
.\bin\axc.exe check examples/hello.ax

# Build a standalone ~5.5 KB native PE32+ executable
.\bin\axc.exe build examples/hello.ax -o bin/hello.exe
.\bin\hello.exe
```

---

## Chapter 2: Variables, Types & Pythonic Ergonomics

### 2.1. Immutable by Default (`:=`) and Explicit Mutability (`mut`)

Bindings declared with `:=` (or `let`) are **immutable by default**. Add `mut` when a variable needs to change:

```axon
fn main() -> i32 {
    limit := 100              // Immutable (inferred i64)
    let ratio: f64 = 3.5      // Explicit type annotation
    mut counter := 0          // Mutable binding

    counter += 10
    println(f"limit={limit}, ratio={ratio}, counter={counter}")
    return 0
}
```

Reassigning an immutable variable or parameter is caught at compile time with **`[SAFETY ERROR E0301]`**.

### 2.2. Primitive Types

| Type | Representation | Literal Examples | Notes |
| :--- | :--- | :--- | :--- |
| `i64`, `int`, `usize` | 64-bit signed integer | `42`, `0xFF00` | Default register-sized integer |
| `i32` | 32-bit integer | `0` | Standard OS exit code for `fn main() -> i32` |
| `u8`, `byte` | 8-bit byte | `65` | Binary buffers and byte parsing |
| `f64`, `float` | 64-bit IEEE-754 float | `3.14159` | Default floating-point scalar |
| `f32` | 32-bit IEEE-754 float | `1.5` | Ideal for `Table[T]` SIMD columns (8 lanes per YMM register) |
| `dec` | 64-bit scaled decimal | `49.50d`, `0.08d` | Exact 4-decimal fixed-point (scale `10_000`), zero rounding drift |
| `bool` | Boolean value | `true`, `false` | Short-circuit `and`, `or`, `not` |
| `str` | UTF-8 string slice (`ptr + len`) | `"Axon"`, `f"x={x}"` | O(1) length, bounds-safe indexing |

### 2.3. Short-Circuit Booleans & Checked vs. Wrapping Arithmetic

- **Short-circuit `and` / `or`:** `i < n and str_byte(s, i) == 65` never evaluates the right-hand side when `i < n` is false.
- **Hardware Overflow Protection:** Under `@shield`, `+`, `-`, and `*` trap on signed CPU overflow (`JO`), while `/` and `%` always guard against division by zero and `INT64_MIN / -1`.
- **Explicit Wrapping Operators (`+%`, `-%`, `*%`):** Cryptographic hashes and PRNGs opt into two's-complement modular wraparound explicitly:

```axon
mut hash := 0xcbf29ce484222325
hash = (hash ^ byte_val) *% 0x100000001b3
```

---

## Chapter 3: Functions, Named/Default Parameters & Pipelines `|>`

### 3.1. Default Parameters & Named Arguments

Write clean, self-documenting APIs without method overloading or verbose builder structs:

```axon
fn compute_score(base: i64, bonus: i64 = 10, multiplier: i64 = 2) -> i64 {
    return (base + bonus) * multiplier
}

fn main() -> i32 {
    s1 := compute_score(50)                               // (50 + 10) * 2 = 120
    s2 := compute_score(50, multiplier = 5)               // (50 + 10) * 5 = 300
    s3 := compute_score(bonus = 20, base = 30)            // (30 + 20) * 2 = 100
    println(f"s1={s1}, s2={s2}, s3={s3}")
    return 0
}
```

### 3.2. In-Place `mut` Parameters & Tuple Destructuring

- Declare `mut x: i64` on a parameter (and pass `mut val` at the call site) for safe copy-in/write-back mutation without raw pointers.
- Return multiple values cleanly with tuples and destructuring bindings:

```axon
fn div_rem(a: i64, b: i64) -> (i64, i64) {
    return (a / b, a % b)
}

fn bump_twice(mut v: i64, step: i64) {
    v += step * 2
}

fn main() -> i32 {
    (q, r) := div_rem(29, 5)
    mut total := q
    bump_twice(mut total, r)
    println(f"q={q}, r={r}, total={total}")
    return 0
}
```

### 3.3. The Pipeline Operator `|>`

`x |> f(a, b)` desugars at compile time into `f(x, a, b)` with zero runtime overhead:

```axon
fn add_offset(x: i64, off: i64) -> i64 { return x + off }
fn scale_up(x: i64, k: i64) -> i64 { return x * k }

fn main() -> i32 {
    result := 10 |> add_offset(5) |> scale_up(4)   // 60
    println(f"piped={result}")
    return 0
}
```

---

## Chapter 4: Control Flow & Static Bounds Elision `where`

### 4.1. `if / else if / else`, `while`, and `for .. in`

```axon
fn main() -> i32 {
    mut sum := 0
    for i in 0..10 {
        if i % 2 == 0 {
            sum += i
        }
    }

    mut n := 5
    while n > 0 {
        n -= 1
    }
    println(f"even_sum={sum}")
    return 0
}
```

### 4.2. Zero-Cost Bounds Check Elision with `where`

Every `table[idx]` access in Axon is bounds-checked by default. When a hot function proves `idx < arr.len` in its parameter refinement clause (or iterates via `for idx in 0..arr.len`), the code generator elides the redundant runtime `CMP + JAE` instructions while preserving 100% memory safety:

```axon
fn read_checked(arr: Table[i64], idx: usize where idx < arr.len) -> i64 {
    return arr[idx]   // Statically proven in-bounds -> zero bounds-check overhead
}
```

---

## Chapter 5: Zero-Cost Register Error Unions `T ! Error`

Instead of heap-allocated exceptions or repetitive error-tuple boilerplate, Axon provides unboxed **`PayloadType ! ErrorType`** unions:

- Construct values with `ok(value)` and `err(code)`.
- Handle errors concisely with:
  1. **`expr or fallback_expr`**: Evaluates `fallback_expr` only when `expr` is an error.
  2. **`expr or return ret_val`**: Returns early from the enclosing function on error.
  3. **`expr?`**: Propagates the error directly to the caller.

```axon
fn parse_port(raw: i64) -> i64 ! i64 {
    if raw <= 0 or raw > 65535 {
        return err(400)
    }
    return ok(raw)
}

fn open_service(raw_port: i64) -> i64 ! i64 {
    port := parse_port(raw_port)?     // Propagates err(400) on failure
    return ok(port + 1000)
}

fn main() -> i32 {
    p_valid := parse_port(8080) or 80
    p_fallback := parse_port(-5) or 3000
    svc := open_service(443) or 0
    println(f"valid={p_valid}, fallback={p_fallback}, svc={svc}")
    return 0
}
```

---

## Chapter 6: Structs, Methods `impl` & Stack-Polymorphic Duck Interfaces

### 6.1. Flat Value Structs & `impl` Blocks

Axon structs live unboxed on the stack with zero object-header overhead, and support both positional and named-field construction:

```axon
struct CloudPlan {
    monthly: dec
    nodes: i64
}

impl CloudPlan {
    fn subtotal(self) -> dec {
        return self.monthly * self.nodes
    }
}
```

### 6.2. Structural Duck-Typed `interface` (Zero Heap Boxing)

Any `struct` implementing the required method signatures satisfies an `interface` automatically without `implements` declarations. Unlike Go or Java interfaces that box values onto the GC heap, Axon packs a concrete type tag + inline value slots directly on the stack:

```axon
use std::decimal

interface Billable {
    fn total_cost(self) -> dec
}

struct DedicatedNode {
    unit_price: dec
    count: i64
}

impl DedicatedNode {
    fn total_cost(self) -> dec {
        return apply_tax(self.unit_price * self.count, rate_pct = 0.10d)
    }
}

fn print_invoice(item: Billable) {
    println(f"Invoice total: ${item.total_cost()}")
}

fn main() -> i32 {
    srv := DedicatedNode(unit_price = 100.00d, count = 2)
    print_invoice(srv)   // $220.00 with 0 heap allocations
    return 0
}
```

---

## Chapter 7: Exact Fixed-Point Financial Decimals `dec`

Binary floating-point (`0.1 + 0.2 == 0.30000000000000004`) silently corrupts financial ledgers, tax calculations, and billing engines.

Axon provides a first-class **`dec`** primitive (literal suffix `d`, scale `10_000`) backed by 64-bit hardware integer registers with signed overflow protection:

```axon
use std::decimal

fn main() -> i32 {
    a := 0.10d
    b := 0.20d
    exact_sum := a + b                                // Exact 0.30d in 1 CPU cycle

    total := apply_tax(199.99d, rate_pct = 0.08d)     // 8% tax
    (share, remainder) := split_exact_cents(100.00d, 3)

    println(f"0.10d + 0.20d = {exact_sum}")
    println(f"Taxed = {total}, 3-way split = {share} (rem = {remainder})")
    return 0
}
```

---

## Chapter 8: Perceus Static Memory Management & FBIP In-Place Reuse

Axon guarantees memory safety **without a Garbage Collector** and **without `'a` lifetime annotations** by combining three compile-time rules:

### 8.1. Second-Class References
References cannot be stored inside structs (**`E0101`**) and cannot be returned from functions (**`E0102`**). Dangling pointers become structurally impossible without needing lifetime parameters.

### 8.2. Linear `move` Tracking & Deterministic Drops
Passing a buffer or table with `move` transfers ownership linearly; reading a moved variable afterward is rejected at compile time with **`E0201`** (including branch-aware flow analysis across early-return `if` blocks). Unmoved owned allocations automatically emit deterministic `VirtualFree` cleanup on every return path.

### 8.3. Functional-But-In-Place (FBIP) Memory Reuse
When an owned buffer is released right before allocating a compatible new buffer, the Perceus planner reuses the existing memory block in place with **zero OS syscalls**:

```axon
fn process_batch() -> i64 {
    buf_a := ibuf_new(256)
    ibuf_push(buf_a, 40)
    ibuf_push(buf_a, 60)
    partial := ibuf_get(buf_a, 0) + ibuf_get(buf_a, 1)
    buf_free(move buf_a)

    // Perceus FBIP reuses buf_a's backing allocation in place (0 VirtualAlloc syscalls)
    buf_b := ibuf_new(256)
    ibuf_push(buf_b, partial + 45)
    ans := ibuf_get(buf_b, 0)
    buf_free(move buf_b)
    return ans
}

fn main() -> i32 {
    println(f"FBIP result = {process_batch()}")
    return 0
}
```

---

## Chapter 9: Columnar `Table[T]` SoA & Automatic 256-Bit AVX2 FMA3 SIMD

This architecture allows idiomatic Axon loops to outperform idiomatic C++, Rust, and Go by **over 6x** and Python by **over 210x** on data-intensive simulations.

### 9.1. Why AoS Vectors Bottleneck on Memory Bandwidth
In C++ (`std::vector<Entity>`) or Rust (`Vec<Entity>`), a 16-field `Entity` (64 bytes) interleaves cold fields (`mass`, `armor`, `shield`, `team`) next to hot fields (`px`, `vx`). Updating `px += vx * dt` forces the CPU to stream all 64 bytes per entity across the memory bus, wasting 75% of cache-line bandwidth.

### 9.2. Automatic Structure-of-Arrays + 4x-Unrolled AVX2 FMA3
When you allocate `Table[Entity]::with_capacity(n)` in Axon:
1. Every struct field is stored in its own contiguous column (`px[0..n]`, `vx[0..n]`, ...).
2. `for p in particles { p.px += p.vx * dt }` streams only the active columns and compiles directly to a **4x-unrolled 128-byte (32 x `f32` per iteration) 256-bit AVX2 `VFMADD231PS` hardware loop**:

```axon
struct Particle {
    px: f32
    vx: f32
    py: f32
    vy: f32
    mass: f32
    heat: f32
}

@shield
fn main() -> i32 {
    n := 4096
    particles := Table[Particle]::with_capacity(n)
    table_fill_f32(particles, 0, 1.0)    // column 0 (px) = 1.0
    table_fill_f32(particles, 1, 3.5)    // column 1 (vx) = 3.5

    dt := 2.0
    for p in particles {
        p.px += p.vx * dt                // Emits 4x-unrolled 256-bit AVX2 VFMADD231PS
    }

    sample := table_get_f32(particles, 0, n - 1)
    println(f"Updated {n} particles via AVX2 SoA: px[{n - 1}] = {sample}")
    return 0
}
```

Run the full 1,048,576-entity x 200-step benchmark yourself in [`examples/bench_soa_simd.ax`](examples/bench_soa_simd.ax):

```powershell
.\bin\axc.exe build examples/bench_soa_simd.ax -o bin/bench_soa_simd.exe
.\bin\bench_soa_simd.exe
```

---

## Chapter 10: Colorless Concurrency `spawn` & Convergent `CRDT` Types

### 10.1. `spawn` Tasks & Compile-Time Isolation Enforcement (`E0202`)
Axon prevents data races at compile time: capturing an outer `isolated` variable inside `spawn { ... }` without transferring ownership via `move` is rejected with **`[SAFETY ERROR E0202]`**:

```axon
fn compute_chunk(isolated data: i64) -> i64 {
    task := spawn {
        moved_val := move data
        return moved_val * 2
    }
    return task.join()
}

fn main() -> i32 {
    println(f"Joined result = {compute_chunk(67)}")
    return 0
}
```

### 10.2. Lock-Free Join-Semilattice CRDTs (`std::crdt`)
For distributed state replication without locks or coordination bottlenecks, `std::crdt` provides mathematically convergent join-semilattices (`GCounter4`, `LWWReg`) satisfying idempotence, commutativity, and associativity:

```axon
use std::crdt

fn main() -> i32 {
    node_a := gcounter_inc(gcounter_new(), 0, 15)
    node_b := gcounter_inc(gcounter_new(), 1, 27)

    // Out-of-order and duplicate merges always converge deterministically to 42
    merged := gcounter_merge(node_a, gcounter_merge(node_b, node_a))
    println(f"Converged cluster counter = {gcounter_value(merged)}")
    return 0
}
```

---

## Chapter 11: Object-Capability Security, Anti-Tamper & AV-Clean Binaries

### 11.1. Object-Capability Security (`std::cap`)
Privileged raw system calls cannot be invoked without holding a capability token in scope (**`E0501`**). Starting from `RootCap` in `main`, applications attenuate permissions down to directory-scoped, read-only `FsCap` tokens that automatically block `..` path traversal, NUL injection, and drive escapes:

```axon
use std::cap

fn main() -> i32 {
    root := acquire_root_cap()
    ro_cap := attenuate_fs(root, base_dir = "examples", read_only = true)

    blocked := not is_safe_subpath("../../Windows/System32/cmd.exe")
    write_attempt := cap_write_file(ro_cap, "test.txt", buf_new(16)) or -403
    println(f"Path traversal blocked={blocked}, write on read-only={write_attempt}")
    return 0
}
```

### 11.2. Built-In Anti-Tamper & Key Protection (`@shield`, `@secret`, `@whitebox`)
1. **`@shield`:** Enables hardware signed-overflow traps, Intel CET `ENDBR64` prologues, and live startup `.text` 64-bit FNV-1a machine-code verification. Modifying a single instruction byte in the compiled `.exe` triggers an immediate integrity halt before `main` executes.
2. **`@secret("...")`:** Encrypts sensitive literals at compile time, decrypts them onto the local stack frame only while needed, and wipes those stack slots to `0` before `RET` (leaving **zero plaintext bytes** in the binary image).
3. **`@whitebox` & `whitebox_sbox` (`std::crypto`):** Dissolves secret key material into compile-time $\text{GF}(2^8)$ bijective lookup tables (irreducible polynomial `0x11B`) rather than exposing raw keys in `.rdata`.

```axon
use std::crypto

@shield
fn main() -> i32 {
    api_pin := @secret("OMEGA-VAULT-9042")
    license_mac := whitebox_mac_str("AXON-PRO-LICENSE-2026")
    println(f"Secret length={api_pin.len}, White-Box MAC={license_mac}")
    return 0
}
```

### 11.3. OS-Standard AV-Clean Binary Architecture
Packers and naive obfuscators trigger Antivirus false positives due to writable-executable (`RWX`) pages, missing unwind tables, or high-entropy encrypted sections (`> 7.2`). Axon's direct PE32+ linker produces clean OS-compliant binaries:
- Strict `W^X` page permissions: `.text` is `RX`, `.rdata` and `.pdata` are `R` (**0 `RWX` sections**).
- Complete `.pdata` `RUNTIME_FUNCTION` and `.xdata` `UNWIND_INFO` tables for every generated function.
- High-Entropy 64-bit ASLR + Hardware DEP/NX (`0x8160`).
- Natural `.text` Shannon entropy (`~5.61 < 6.50`).

---

## Chapter 12: Self-Hosting Internals, Built-In Intrinsics & Error Reference

### 12.1. Deterministic 3-Stage Self-Hosting Bootstrap
[`bin/axc.exe`](bin/axc.exe) is compiled from the 9 pure-Axon modules in [`compiler_ax/`](compiler_ax/):
- **Stage 1:** `axc0.exe` compiles `compiler_ax/main.ax` $\rightarrow$ `axc_stage1.exe`.
- **Stage 2:** `axc_stage1.exe` compiles `compiler_ax/main.ax` $\rightarrow$ `axc_stage2.exe`.
- **Stage 3:** `axc_stage2.exe` compiles `compiler_ax/main.ax` $\rightarrow$ `axc_stage3.exe`.
- **Fixed-Point Proof:** `SHA256(axc_stage2.exe) == SHA256(axc_stage3.exe)` matches bit-for-bit in ~114 ms, with whole-program dead-function tree-shaking keeping user binaries minimal.

### 12.2. Built-In Intrinsics Reference

| Category | Intrinsics | Purpose |
| :--- | :--- | :--- |
| **Output & Timing** | `print(...)`, `println(...)`, `clock_ns()` | Zero-CRT Win64 console output (`f"..."` supported) and nanosecond monotonic timer |
| **Byte Buffers (`ByteBuf`)** | `buf_new(cap)`, `buf_len(b)`, `buf_get(b, i)`, `buf_set(b, i, v)`, `buf_push(b, v)`, `buf_push_u16/u32/u64`, `buf_put_u32/u64`, `buf_push_str`, `buf_slice_str`, `buf_free(move b)` | Bounds-checked binary buffers for binary linkers, protocols, and file I/O |
| **Integer Buffers (`IntBuf`)** | `ibuf_new(cap)`, `ibuf_len(b)`, `ibuf_get(b, i)`, `ibuf_set(b, i, v)`, `ibuf_push(b, v)`, `buf_free(move b)` | Dynamic 64-bit integer arenas with hardware bounds checks |
| **Strings & CLI Args** | `str_len(s)`, `str_byte(s, i)`, `str_slice(s, a, b)`, `str_eq(a, b)`, `args_count()`, `args_get(i)` | O(1) UTF-8 slice operations and command-line argument access |
| **File I/O (`T ! i64`)** | `fs_read_file(path)`, `fs_write_file(path, buf)` | Direct Win64 binary file read/write returning `T ! i64` error unions |
| **SoA Tables & Crypto** | `Table[T]::with_capacity(n)`, `table_fill_f32(t, col, v)`, `table_get_f32(t, col, i)`, `whitebox_sbox(seed, byte)` | Columnar SIMD table helpers and $\text{GF}(2^8)$ white-box lookup tables |

### 12.3. Compile-Time Safety Error Codes

| Code | Violation Caught | Fix in Axon |
| :--- | :--- | :--- |
| **`E0101`** | Reference type (`&T`) stored inside a `struct` field | Store owned values directly or integer row indices into `Table[T]` |
| **`E0102`** | Function returns a reference (`-> &T`) risking a dangling pointer | Return values directly in registers/stack slots or via `T ! Error` |
| **`E0201`** | Variable accessed after linear ownership transfer (`move`) | Read the value before `move`, or pass it by value without `move` |
| **`E0202`** | Outer `isolated` variable captured in `spawn` without `move` | Explicitly transfer ownership with `move var_name` inside `spawn` |
| **`E0301`** | Mutation or reassignment of an immutable binding (`:=` / `let`) | Declare the variable with `mut x := ...` or parameter with `mut x: Type` |
| **`E0401`** | Initializer type does not match declared variable type | Match the declared type or convert explicitly with `as Type` |
| **`E0402`** | `return` expression type does not match function return signature | Return a compatible type or wrap in `ok(...)` / `err(...)` for `T ! E` |
| **`E0501`** | Privileged ambient I/O called without a capability token in scope | Pass an attenuated capability token (`RootCap` / `FsCap` / `NetCap`) from `std::cap` |
