# AI Agent Master Guide for Axon (`docs/AGENT_MASTER_GUIDE.md`)

> **Zero-Hallucination Specification, 103-Module Lookup Matrix, Error Dictionary (`E0001..E0604`), and Full-Stack Templates (`axc` v1.2.0).**
> Read this guide to generate **100% first-try compiling, zero-leak, memory-safe pure Axon (`.ax`)** without falling back to C, Python, Node.js, or PowerShell.

---

## 1. Sub-Second Agent Verification Loop

Always verify every `.ax` file immediately after writing or editing:

```powershell
# 1. Structured JSON static check (~10-35 ms: syntax, types, Perceus 0-GC, taint E0502, unsafe E0503)
.\bin\axc.exe check --json path/to/file.ax

# 2. Run unit tests (`@test fn ...`) or execute directly
.\bin\axc.exe test path/to/file.ax
.\bin\axc.exe run path/to/file.ax

# 3. Production build with overflow/integrity Shield + symbol map
.\bin\axc.exe build path/to/file.ax --shield --emit-map -o app.exe
```

---

## 2. Zero-Hallucination Grammar Rules (DO vs NEVER DO)

| Rule | CORRECT Axon (`.ax`) | FORBIDDEN (Will Fail Compile!) |
| :--- | :--- | :--- |
| **Boolean Logic** | `if a and b or not c { ... }` | `&&`, `||`, `!c` |
| **Immutable / Mutable** | `x := 10` / `mut y := 20` | `let x = 10`, `const x = 10`, `var y = 20` |
| **Reserved Keyword `pass`** | `mut bp := 0`, `mut step := 0` | `mut pass := 0` (`pass` is a keyword -> `E0020`!) |
| **Struct Init & Named Args** | `Point(x = 1, y = 2)` | `Point { x: 1, y: 2 }` (`:` is only for types!) |
| **If-Expression** | `v := if c { 1 } else { 2 }` | `v := c ? 1 : 2`, or `else if` without `{}` in expr |
| **Error Union Type** | `fn f() -> str ! i64` | `Result<String, i64>`, `(str, error)` |
| **Error Return** | `return ok(val)` / `return err(404)` | `Ok(val)`, `Err(404)`, `throw` |
| **Error Unwrap** | `x := f() or return` / `x := f() or "def"` | `f()?`, `f().unwrap()`, `f() or { ... }` |
| **Mut / Move Call Args** | `dvec_push(mut v, 1)` / `dvec_free(move v)` | `dvec_push(&mut v, 1)` / forgetting `move` |
| **Wrapping 64-bit Math** | `a +% b`, `a -% b`, `a *% b`, `i64_min -% 1` | Using `-` on `INT64_MIN - 1` under `--shield` |
| **Unsigned Right Shift** | `ushr64(x, k)` | `(x as u64) >> k` (`>>` on `i64` is signed `SAR`!) |
| **Type Cast** | `x as f64`, `y as i64`, `0 as i32` | `(double)x`, `i64(y)` |
| **String Interpolation** | `f"id={id} name={name}"` | `format!(...)`, `` `${id}` `` |
| **Generics** | `struct Box[T] { val: T }`, `Box[i64](val = 1)` | `Box<T>`, `Box::<i64>` |

---

## 3. Complete 103-Module "Need X -> Use Y" Matrix

### 3.1 Algorithms, Dynamic Data Structures & Graphs (v1.2 Core)
| Need | Module | Exact Functions & Types |
| :--- | :--- | :--- |
| **O(N) Sort, Binary Search, Heap, Union-Find** | `use std::algo` | `sort_i64(ibuf)` ($O(N)$ LSD Radix Sort), `sort_i64_range(ibuf, lo, hi)`, `unsafe { radix_sort_raw_i64(ptr, n) }`, `bsearch_i64`, `lower_bound_i64`, `upper_bound_i64`, `ibuf_reverse`, `ibuf_dedup_sorted`, `ibuf_sum`/`min`/`max`, `heap_new`/`heap_push_min`/`heap_pop_min`, `heap_kv_new`/`heap_push_kv_min`/`heap_pop_kv_min` (`-> (k, v)`), `dsu_new`/`dsu_find`/`dsu_union`/`dsu_same`/`dsu_components` |
| **Auto-Growing Vectors, Maps, Deque, BitSet** | `use std::ds` | `DynIntVec` (`dvec_new`, `dvec_push(mut v, x)`, `dvec_pop(mut v)`, `dvec_get`, `dvec_set`, `dvec_free(move v)`), `StrVec` (`svec_new`, `svec_push(mut sv, s)`, `svec_get`, `svec_contains`, `svec_join`, `svec_free(move sv)`), `DynMap` (`dmap_new`, `dmap_put(mut m, k, v)`, `dmap_get`, `dmap_contains`, `dmap_remove(mut m, k)`, `dmap_free(move m)`), `StrMap` (`smap_new`, `smap_set_str`, `smap_set_i64`, `smap_get_str`, `smap_get_i64`, `smap_has`/`smap_contains`, `smap_remove`, `smap_free(move sm)`), `IntDeque` (`deque_new`, `deque_push_front`/`back`, `deque_pop_front`/`back`, `deque_free`), `BitSet` (`bitset_new`, `bitset_set`/`clear`/`test`/`count`, `bitset_free`) |
| **Graphs (BFS, Dijkstra, Topological Sort)** | `use std::graph` | `Graph`, `graph_new(n_verts, est_edges)`, `graph_add_edge(mut g, u, v, w)`, `graph_add_undirected`, `graph_bfs(g, src)`, `graph_dijkstra(g, src)`, `graph_toposort(g)` (`-> IntBuf ! i64`, returns `err(-1)` on cycle!), `graph_free(move g)` |
| **Zero-Cost Generics & Fixed Collections** | `use std::vec`, `use std::collections`, `use std::ringbuf`, `use std::cache`, `use std::linq`, `use std::crdt` | `Box[T]`, `Pair[A,B]`, `Option[T]`, `some[T]`, `none_opt[T]`, `unwrap_or`, `min_val`, `max_val`, `clamp_val`, SPSC `ringbuf`, LRU `cache`, CRDTs |

### 3.2 Strings, Formatting, Filesystem, OS, CLI, FFI & Benchmarking
| Need | Module | Exact Functions & Types |
| :--- | :--- | :--- |
| **Strings & Splitting** | `use std::str` | `str_len`, `str_byte`, `str_eq`, `str_eq_ignore_case`, `str_slice`, `str_contains`, `str_starts_with`, `str_ends_with`, `str_find_byte`, `str_trim`, `str_to_lower`, `str_to_upper`, `str_replace`, `str_repeat`, `parse_i64`, `str_from_i64`, `url_encode`, `url_decode`, `base64_encode`, `base64_decode`, `buf_to_owned_str(move b)` |
| **Hex/Bin/Comma/`f64`/Table Formatting** | `use std::fmt` | `fmt_hex(val, upper, min_width)`, `fmt_bin(val, min_width)`, `fmt_oct(val)`, `fmt_commas(val)`, `fmt_f64(val, decimals)`, `fmt_pad_left`/`right`/`center`, `fmt_bytes_human(bytes)`, `fmt_duration_ns(ns)`, `fmt_markdown_table(headers, rows, ncols)` |
| **Ergonomic Filesystem & Dir Listing** | `use std::fs`, `use std::path` | `fs_read_text(p) -> str ! i64`, `fs_write_text(p, s) -> bool`, `fs_append_text`, `fs_atomic_write(p, s)` (crash-safe!), `fs_read_lines(p) -> StrVec ! i64`, `fs_is_dir`, `fs_is_file`, `fs_mkdir_all`, `fs_copy`, `fs_move_file`, `fs_temp_dir`, `fs_list_dir(p) -> StrVec`, `path_join`, `path_join_safe`, `path_normalize`, `path_ext` |
| **OS Env, Process, CLI, Logs** | `use std::sys`, `use std::os`, `use std::cli`, `use std::terminal`, `use std::log` | `sys_env_get`, `sys_env_set`, `sys_cwd`, `sys_pid`, `sys_fs_exists`, `sys_fs_remove`, `sys_fs_size`, `sys_exec_capture`, `args_count`, `args_get` |
| **Microbenchmarking & Testing** | `use std::bench`, `use std::test_kit` | `BenchStats`, `bench_blackbox_i64`, `bench_compute_stats(samples)` (`min_ns`, `p50_ns`, `p95_ns`, `p99_ns`, `max_ns`, `mean_ns`, `cv_pct`), `bench_format_summary` |
| **Universal 16-Arg C-ABI FFI** | `use std::ffi` | `ffi_open(win_dll, linux_so, mac_dylib)`, `ffi_sym(h, name)`, `ffi_call0`..`ffi_call8`, `ffi_call_arr`, `ffi_cstr`, `ffi_wstr`, `ffi_read_cstr`, `ffi_alloc`, `ffi_free`, `ffi_sext32` |

### 3.3 Web, Networking, TLS, URLs, JWT, WebSockets, gRPC & Databases
| Need | Module | Exact Functions & Types |
| :--- | :--- | :--- |
| **TCP / UDP / HTTP/1.1 / TLS 1.3** | `use std::net`, `use std::http`, `use std::tls`, `use std::async_io` | `tcp_listen`, `tcp_accept`, `tcp_connect`, `tcp_send_str`, `tcp_recv`, `https_get`, `https_post_json`, `https_request`, `http_get_header`, `http_is_safe_path` |
| **RFC-3986 URL & SSRF Guard** | `use std::url` | `Url`, `url_parse(s) -> Url ! i64`, `url_build(u)`, `url_query_parse(q) -> StrMap`, `url_query_get(q, k, def)`, `url_is_public_http(u)`, `url_str_is_public_http(raw_url)` (blocks `127.*`, `10.*`, `169.254.*`, `192.168.*`, `172.16..31.*`, `localhost`, `[::1]`, octal/hex/decimal IP bypasses!) |
| **Web Router, `:param` & Static Server** | `use std::web` | `web_route_match(req_method, req_path, expect_method, pattern)`, `web_param(req_path, pattern, name)`, `web_json(status, body)`, `web_html`, `web_text`, `web_redirect(status, loc)`, `web_bearer_token(raw_req)`, `web_serve_static(root_dir, req_path)` |
| **JWT (HS256) & Password Hashing** | `use std::jwt` | `jwt_sign_hs256(payload_json, secret) -> str ! i64`, `jwt_verify_hs256(token, secret, now_unix) -> str ! i64` (blocks `"alg":"none"`, constant-time MAC, enforces `exp`/`nbf`), `password_hash_sha256`, `password_verify_sha256` |
| **RFC-6455 WebSockets** | `use std::ws` | `WsFrame`, `ws_accept_key(key)`, `ws_build_upgrade_request`, `ws_build_upgrade_response`, `ws_encode_frame(op, payload, use_mask, mask_u32, fin) -> ByteBuf`, `ws_decode_frame(raw, offset, max_payload) -> WsFrame ! i64` |
| **SQLite3 / PostgreSQL / MySQL / gRPC** | `use std::sqlite`, `use std::postgres`, `use std::mysql`, `use std::grpc` | `sqlite_open`, `sqlite_exec`, `sqlite_prepare`, `sqlite_bind_i64`/`text`, `sqlite_step` (`100`=ROW, `101`=DONE), `sqlite_col_i64`/`text`, `sqlite_finalize`, `sqlite_close`; `pg_connect`/`pg_query`; `mysql_connect`/`mysql_query`; `grpc_connect`/`grpc_unary_call` |

### 3.4 Data Formats, Security, Desktop UI, Games & AI/GPU
| Need | Module | Exact Functions & Types |
| :--- | :--- | :--- |
| **JSON, CSV, TOML, Regex, Base64URL** | `use std::json`, `use std::csv`, `use std::toml`, `use std::regex`, `use std::encoding`, `use std::compress`, `use std::data` | `json_get_str(j, k)`, `json_get_i64(j, k)`, `to_json(val)`, `base64url_encode`, `base64url_decode`, `str_to_hex`, `str_from_hex` |
| **Crypto, Taint, Capabilities, Shield** | `use std::crypto`, `use std::taint`, `use std::cap`, `use std::shield`, `use std::uuid` | `sha256_str`, `hmac_sha256_str`, `ct_eq_str`, `uuid_v4`, `tainted[str]`, `RootCap`/`FsCap`/`NetCap` |
| **Native HTML5/CSS3/Tailwind/JS GUI** | `use std::ui`, `use std::gui`, `use std::webview` | `ui_window_html(title, w, h, html) -> UiDoc`, `gui_run_ui_app(mut doc)`, `ui_free(move doc)`, `webview_open_app` |
| **AI Agent Harness, LLM, CUDA, Games** | `use std::ai_harness`, `use std::ai_agent`, `use std::ai_eval`, `use std::ai_math`, `use std::cuda`, `use std::llama`, `use std::vllm`, `use std::faiss`, `use std::game2d`, `use std::game3d`, `use std::audio` | `gemm_fast_f32`, PTX JIT CUDA, GGUF/LLaMA/vLLM, FAISS L2 search, Agent ReAct/DAG/Swarm, 2D/3D ECS & Physics |

---

## 4. Compiler Diagnostic Fix Dictionary (`E0001 .. E0604`)

| Code | Meaning | Exact 1-Line Agent Fix |
| :--- | :--- | :--- |
| **`E0020`** | Syntax error / unexpected token / recursion >256 | Check: (1) did you name a var `pass`? Rename it; (2) replace `&&`/`||`/`!` with `and`/`or`/`not`; (3) replace `Struct { f: 1 }` with `Struct(f = 1)`; (4) wrap nested `else { if ... }` in if-expressions. |
| **`E0101` / `E0102`** | Second-class `&T` reference escaped | Store indices/handles (`i64`) or owned values in structs/returns instead of `&T`. |
| **`E0201`** | Use-after-move | Clone/extract needed values BEFORE passing `move x`. |
| **`E0202`** | Unconsumed owned variable leak | Call the matching destructor (`buf_free(move b)`, `svec_free(move sv)`, `smap_free(move sm)`, `dmap_free(move m)`, `graph_free(move g)`) or `return` it on every path. |
| **`E0203`** | Outer variable moved inside loop | Move `buf_free(move x)` AFTER the `while`/`for` loop, or allocate inside the loop. |
| **`E0204`** | Mutable capture race in `par_for` | Use `Table[T]` column writes inside `par_for`, or use regular `for` for scalar reductions. |
| **`E0301`** | Assignment to immutable variable | Change declaration `x := ...` to `mut x := ...` (and param `x: T` to `mut x: T`). |
| **`E0401`** | Static type mismatch | Add explicit `as i64` / `as f64` / `as i32`, or unwrap `T ! i64` with `or fallback`! |
| **`E0403`** | Unresolved identifier | Add `use std::<module>` (e.g., `use std::str` for `buf_to_owned_str`) or check function name in §3 above. |
| **`E0404`** | Call signature / `mut` / `move` mismatch | Match exact arg count, add `mut arg` for `mut` params, and `move arg` for `move` params. |
| **`E0405`** | Missing `return` in non-void function | Add explicit `return expr` on the final path (`return if c { a } else { b }`). |
| **`E0501`** | Missing capability (`FsCap`/`NetCap`) | Thread capability parameter from `main(root: RootCap)` or use high-level `std::fs`/`std::net`. |
| **`E0502`** | Unsanitized `tainted[str]` sink | Validate/parameterize input (e.g., `sqlite_bind_text` instead of SQL string concat) before sink. |
| **`E0503`** | Raw intrinsic outside `unsafe` | Prefer safe `std::*` wrappers (`std::ds`, `std::fs`, `std::ffi`), or wrap raw `ptr_read_*`/`ptr_write_*`/`mem_alloc_aligned` in `unsafe { ... }`. |
| **`E0601`** | Heap allocation inside `@no_alloc` fn | Remove `buf_new`/string `+` inside `@no_alloc` hot function or pass preallocated buffer. |
| **`E0602` / `E0604`** | Tensor shape / `comptime_assert` failure | Align matrix inner dimensions `[M, K] @ [K, N]` or fix failed compile-time assertion. |

---

## 5. Production Copy-Paste Full-Stack Template (HTTP + Router + JWT + SQLite)

```axon
use std::str
use std::net
use std::http
use std::web
use std::jwt
use std::sqlite

fn handle_request(db: i64, secret: str, raw_req: str) -> str {
    req := http_parse_request(raw_req)
    if web_route_match(req.method, req.path, "POST", "/auth/token") {
        tok := jwt_sign_hs256("{\"sub\":\"admin\",\"exp\":2000000000}", secret) or return web_json(500, "{\"error\":\"sign\"}")
        return web_json(200, "{\"token\":\"" + tok + "\"}")
    }
    if web_route_match(req.method, req.path, "GET", "/users/:id") {
        bearer := web_bearer_token(raw_req)
        _ := jwt_verify_hs256(bearer, secret, 1900000000) or return web_json(401, "{\"error\":\"unauthorized\"}")
        uid := parse_i64(web_param(req.path, "/users/:id", "id")) or return web_json(400, "{\"error\":\"bad id\"}")
        stmt := sqlite_prepare(db, "SELECT name FROM users WHERE id = ?1;") or return web_json(500, "{\"error\":\"db\"}")
        _ := sqlite_bind_i64(stmt, 1, uid)
        mut resp := web_json(404, "{\"error\":\"not found\"}")
        if sqlite_step(stmt) == 100 {
            name := sqlite_col_text(stmt, 0)
            resp = web_json(200, f"{{\"id\":{uid},\"name\":\"{name}\"}}")
        }
        sqlite_finalize(stmt)
        return resp
    }
    return web_json(404, "{\"error\":\"route not found\"}")
}
```
