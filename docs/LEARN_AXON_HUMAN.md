# Learn Axon from Zero to Expert (`docs/LEARN_AXON_HUMAN.md`)

> Complete hands-on textbook & curriculum for developers learning **Axon (`.ax` v1.2.0)** — from your first 5-line script to zero-GC systems engineering, full-stack web/JWT/WebSockets/SQL, native HTML5/Tailwind desktop apps, and AVX2/GPU AI compute.

---

## Why Axon?

Axon gives you **Python's clean syntax** with **native bare-metal speed faster than Rust/Go/Java**, **zero garbage-collector pauses (Perceus 0-GC)**, **zero Rust lifetime headaches**, **17 ms compile times**, and **103 built-in standard library modules** (HTTP, TLS, SQLite, PostgreSQL, MySQL, gRPC, WebSockets, JWT, Desktop HTML/CSS/Tailwind GUI, C FFI, and AI/CUDA) in a single `2.3 MB` compiler binary (`bin/axc.exe`).

```powershell
# Verify, run, test, or build a standalone native .exe in milliseconds:
.\bin\axc.exe check app.ax
.\bin\axc.exe run app.ax
.\bin\axc.exe test app.ax
.\bin\axc.exe build app.ax -o app.exe
```

---

## Module 1 — Basics: Variables, Types, Strings & Control Flow

### 1.1 Immutable by Default (`:=`) & Mutable (`mut`)
Variables declared with `:=` are **immutable by default**. Add `mut` only when a variable needs to change:

```axon
fn main() -> i32 {
    name := "Axon"          // str (immutable)
    version := 120          // i64 (64-bit signed integer)
    pi := 3.14159265        // f64 (64-bit IEEE-754 float)
    price := 19.99d         // dec (exact 64-bit fixed-point financial decimal!)
    active := true          // bool

    mut score := 10
    score += 5              // OK because `score` is `mut`
    println(f"Hello {name} v{version}! score={score}, pi={pi}, price={price}, active={active}")
    return 0
}
```

### 1.2 Operators & Control Flow
- **Boolean operators:** Always write English `and`, `or`, `not` (never `&&`, `||`, `!`).
- **Arithmetic:** `+`, `-`, `*`, `/`, `%`, `**` (checked under `--shield`). For intentional modular 64-bit wrap-around (hashes, PRNGs), use `+%`, `-%`, `*%`.
- **Bitwise:** `&`, `|`, `^`, `<<`, `>>` (signed shift), `ushr64(x, k)` (unsigned logical shift), `popcnt64(x)`.
- **Loops:** `for i in 0..n { ... }` (`0` up to `n - 1`) and `while cond { ... }` (`break`, `continue`).

```axon
fn main() -> i32 {
    mut total := 0
    for i in 0..100 {
        if i % 2 == 0 and not (i == 50) {
            total += i
        }
    }
    tag := if total > 1000 { "large" } else { "small" }
    println(f"total={total} ({tag})")
    return 0
}
```

> [!IMPORTANT]
> Never name a variable `pass` — `pass` is a reserved no-op keyword in Axon (like Python). Use `step`, `bp`, or `round`.

---

## Module 2 — Functions, Tuples & Zero-Cost Error Unions (`T ! i64`)

Axon has **no exceptions** (which hide control flow and allocate stack traces). Instead, functions that may fail return a zero-cost register-packed **Error Union** `T ! i64`:

```axon
use std::str

// Default & named parameters use `=`
fn divide_checked(a: i64, b: i64, scale: i64 = 1) -> i64 ! i64 {
    if b == 0 {
        return err(400) // return error code
    }
    return ok((a / b) * scale)
}

// Multiple return values via Tuples
fn min_max(a: i64, b: i64) -> (i64, i64) {
    if a <= b { return (a, b) }
    return (b, a)
}

fn compute_pipeline(input: str) -> i64 ! i64 {
    n := parse_i64(input) or return err(422) // propagate error early
    q := divide_checked(100, n, scale = 10) or 0 // fallback default value `0` on error!
    return ok(q)
}

fn main() -> i32 {
    res := compute_pipeline("25") or -1
    (lo, hi) := min_max(res, 10)
    println(f"res={res}, lo={lo}, hi={hi}")
    return 0
}
```

### Three Ways to Unwrap `T ! i64`
1. **Fallback value:** `x := may_fail() or 42`
2. **Early return from `main`/value fn:** `x := may_fail() or return 1`
3. **Propagate inside `T ! i64` fn:** `x := may_fail() or return` (or `or return err(500)`)

---

## Module 3 — Perceus 0-GC Memory Ownership (Fast as C, Safe as Rust, Easy as Python)

How does Axon allocate and free `200,000 × 4 KB` buffers in **`1 ms`** (`10x` faster than Rust, `150x` faster than Go/Java) with **zero garbage collector** and **zero lifetime annotations**?

1. **Primitive values (`i64`, `f64`, `bool`, `str` slices)** copy freely in registers.
2. **Heap-owned containers (`ByteBuf`, `IntBuf`, `DynIntVec`, `StrVec`, `DynMap`, `StrMap`, `Graph`, `Table[T]`)** have **exactly one owner**.
3. Before an owned variable goes out of scope, you either **return it** or **consume it with `move`** (e.g., `buf_free(move b)` or `buf_to_owned_str(move b)`). If you forget or double-free, `axc check` tells you at compile time!
4. **Mutating a container** requires `mut` at BOTH the declaration and call site (`dvec_push(mut vec, 42)`), making state changes 100% visible when reading code.

```axon
use std::str

fn build_greeting(user: str) -> str {
    b := buf_new(64)               // allocate owned ByteBuf from 0-LOCK slab freelist
    buf_push_str(b, "Welcome, ")
    buf_push_str(b, user)
    buf_push(b, 33)                // '!'
    return buf_to_owned_str(move b) // consumes `b` with zero-copy ownership transfer!
}

fn main() -> i32 {
    msg := build_greeting("Alice")
    println(msg)
    return 0
}
```

---

## Module 4 — Essential Standard Library: Dynamic Containers, Sorting, Files & Formatting

### 4.1 Auto-Growing Data Structures (`std::ds`)
Unlike fixed-capacity `buf_new(cap)` / `ibuf_new(cap)`, `std::ds` containers grow geometrically (`2x`) automatically:

```axon
use std::ds
use std::algo
use std::fmt

fn main() -> i32 {
    // 1. Auto-growing integer vector
    mut v := dvec_new()
    for i in 0..100 { dvec_push(mut v, (100 - i) * 7) }
    println(f"dvec len={dvec_len(v)}, first={dvec_get(v, 0)}")
    dvec_free(move v)

    // 2. Auto-rehashing String Dictionary (`StrMap`)
    mut cfg := smap_new()
    smap_set_str(mut cfg, "host", "127.0.0.1")
    smap_set_i64(mut cfg, "port", 8080)
    println(f"endpoint = {smap_get_str(cfg, 'host')}:{smap_get_i64(cfg, 'port')}")
    smap_free(move cfg)

    // 3. O(N) LSD Radix Sort + Binary Search (`std::algo`) + Comma Formatting (`std::fmt`)
    nums := ibuf_new(16)
    ibuf_push(nums, 5000000); ibuf_push(nums, -42); ibuf_push(nums, 1024)
    sort_i64(nums)
    idx := bsearch_i64(nums, 1024)
    println(f"max={fmt_commas(ibuf_get(nums, 2))}, idx_of_1024={idx}, hex=0x{fmt_hex(255, true, 4)}")
    buf_free(move nums)
    return 0
}
```

### 4.2 Safe Filesystem & Atomic Writes (`std::fs`)
```axon
use std::fs
use std::ds

fn main() -> i32 {
    dir := fs_temp_dir() + "/axon_demo"
    fs_mkdir_all(dir)
    path := dir + "/notes.txt"

    // Crash-safe temp-file + atomic OS rename!
    fs_atomic_write(path, "line 1\nline 2\nline 3")

    lines := fs_read_lines(path) or return 1
    for i in 0..svec_len(lines) {
        println(f"[{i}] {svec_get(lines, i)}")
    }
    svec_free(move lines)
    return 0
}
```

---

## Module 5 — Structs, Zero-Cost Generics & SIMD `Table[T]`

```axon
use std::vec

struct Pair[A, B] {
    first: A
    second: B
}

// Data-Oriented Structure-of-Arrays (`Table[T]`): auto-vectorized with 256-bit AVX2!
struct Body {
    x: f32
    y: f32
    vx: f32
    vy: f32
}

fn integrate(world: Table[Body], dt: f32) {
    for b in world {
        b.x += b.vx * dt
        b.y += b.vy * dt
    }
}

fn main() -> i32 {
    p := Pair[i64, str](first = 200, second = "OK")
    println(f"{p.first} {p.second}")

    world := Table[Body](1024)
    integrate(world, 0.016)
    buf_free(move world)
    return 0
}
```

---

## Module 6 — Full-Stack Web, JWT, URL/SSRF Guard, WebSockets & SQLite

Axon ships complete pure-Axon networking, TLS/HTTPS (`std::tls`), web routing (`std::web`), JWT (`std::jwt`), WebSockets (`std::ws`), SQLite (`std::sqlite`), PostgreSQL (`std::postgres`), and MySQL (`std::mysql`):

```axon
use std::sqlite
use std::web
use std::jwt
use std::url

fn main() -> i32 {
    // 1. Embedded SQLite3 (uses built-in winsqlite3.dll on Windows / libsqlite3 on Unix!)
    db := sqlite_open(":memory:") or return 1
    _ := sqlite_exec(db, "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT);")
    _ := sqlite_exec(db, "INSERT INTO users (name) VALUES ('Ada Lovelace');")

    stmt := sqlite_prepare(db, "SELECT id, name FROM users WHERE id = ?1;") or return 1
    _ := sqlite_bind_i64(stmt, 1, 1)
    if sqlite_step(stmt) == 100 { // 100 = SQLITE_ROW
        println(f"DB Row: id={sqlite_col_i64(stmt, 0)}, name={sqlite_col_text(stmt, 1)}")
    }
    sqlite_finalize(stmt)
    sqlite_close(db)

    // 2. SSRF validation (`std::url`) + HS256 JWT (`std::jwt`) + Router (`std::web`)
    assert(not url_str_is_public_http("http://169.254.169.254/latest/meta-data/")) // blocked!
    assert(url_str_is_public_http("https://api.github.com/repos"))                 // allowed!

    tok := jwt_sign_hs256("{\"sub\":\"1\",\"exp\":2000000000}", "my-32-byte-hmac-secret-key-2026") or return 1
    claims := jwt_verify_hs256(tok, "my-32-byte-hmac-secret-key-2026", 1900000000) or return 1
    resp := web_json(200, claims)
    println(resp)
    return 0
}
```

---

## Module 7 — Native HTML5 + CSS3 + Tailwind + JS Desktop Apps (`std::ui`, `std::gui`)

Build crisp `< 12 MB` RAM native desktop apps using Direct2D Hardware Acceleration (`Photon-D2D v2`) with **zero Electron/WebView bloat**:
- Supports Flexbox, Grid, `border-radius`, `linear-gradient(...)`, multi-layer `box-shadow`, `:hover`, `:focus`, `:active`, smooth 60 FPS mouse-wheel inertia, live `<input>` editing, and embedded `<script>` JS (`let`/`const`, `if`/`else`, `while`, `setText`, `getValue`, `addClass`, `removeClass`, `toggleClass`).
- Run [`examples/antigravity_ui.ax`](../examples/antigravity_ui.ax) (`.\bin\axc.exe run examples/antigravity_ui.ax`) for a complete showcase dashboard!

---

## Module 8 — High-Performance AVX2 Compute, Benchmarks & Security (`--shield`)

### 8.1 Writing Loops That Auto-Vectorize to `8×i64` / `64B` AVX2
Axon's `x86-64` backend automatically vectorizes pure `for i in 0..n` reductions and byte-stream sums:
- **Range & Recurrence `8×i64` SIMD:** `for i in 0..n` bodies using `+`, `+%`, `-`, `-%`, `* const`, `*% const`, pow2 `/` & `%`, `<<`, `>>`, `ushr64`, `&`, `|`, `^` and accumulating into an outer `i64` (`acc += ...` or `acc = acc ^ ...`) automatically compile to **2x-interleaved 256-bit `YMM0..YMM15` AVX2**!
- **64-Byte/Iter `VPSADBW` Sum:** Summing `ptr_read_u8(ptr, i)` in `for i in 0..n` or `while i < n` compiles to `64 B/iter` `VPSADBW` (~16 GB/s–45 GB/s)!
- Measure percentiles cleanly with `std::bench` (`bench_compute_stats`, `bench_format_summary`).

### 8.2 Compile-Time Taint (`E0502`), `unsafe` (`E0503`) & Binary `--shield`
- Mark untrusted inputs `tainted[str]` — Axon blocks passing `tainted[str]` to SQL/command sinks at compile time (`E0502`) until explicitly validated.
- Raw pointer/FFI intrinsics require `unsafe { ... }` (`E0503`), keeping normal application code 100% memory-safe.
- Build production binaries with `axc build app.ax --shield --emit-map -o app.exe` for runtime signed-overflow traps, polymorphic `.text` MAC verification, anti-debug checks, and PIE stack backtraces.

---

## 7-Day Practical Mastery Plan

1. **Day 1:** Run `examples/hello.ax`; practice `mut`, `for i in 0..n`, `T ! i64` error unions, and `axc check --json`.
2. **Day 2:** Master Perceus ownership (`ByteBuf`, `IntBuf`, `move`, `mut`) and `std::ds` (`DynIntVec`, `StrMap`, `DynMap`).
3. **Day 3:** Sort, search, and traverse graphs with `std::algo` (`sort_i64`, `heap_kv_new`, `dsu_new`) and `std::graph` (`graph_dijkstra`, `graph_toposort`).
4. **Day 4:** Build a CLI tool with `std::cli`, `std::fs` (`fs_atomic_write`, `fs_list_dir`), `std::json`, and `std::fmt`.
5. **Day 5:** Build a REST + WebSocket microservice with `std::net`, `std::web`, `std::jwt`, `std::ws`, and `std::sqlite`.
6. **Day 6:** Build a native dark-mode desktop app in HTML/Tailwind/JS with `std::ui` & `std::gui` (study `examples/antigravity_ui.ax`).
7. **Day 7:** Profile a compute/AI kernel with `std::bench`, `Table[T]`, and `std::ai_math`, then ship a hardened `--shield` binary!
