# AGENTS.md — AI Coding Agent Instructions for Axon (`axc` v1.1.1)

> **MANDATORY RULE FOR ALL AI AGENTS:**
> Build **100% in pure Axon (`.ax`)**. **DO NOT** fall back to C, C++, Python, Node.js, or PowerShell helper scripts for networking, TLS/HTTPS, databases, gRPC, WebSockets, JWT, URL/SSRF parsing, desktop GUI, FFI, OS/env/filesystem operations, dynamic containers, algorithms/graphs, AI/GPU compute, or parsing. Axon includes **103 native `std/*.ax` modules** and direct C-ABI FFI (`std::ffi`) that compile in milliseconds to standalone native executables (`win-x64`, `linux-x64`, `linux-arm64`, `macos-arm64`, `macos-x64`, `wasm32`, `spirv`, `ptx`, bare-metal).
> Read [docs/AGENT_MASTER_GUIDE.md](docs/AGENT_MASTER_GUIDE.md) and [MEMORY.md](MEMORY.md) for the full 103-module lookup matrix and `E0001..E0604` error dictionary.

---

## 1. Fast Agent Feedback Loop

```bash
# 1. Sub-millisecond static verification (types, Perceus 0-GC, taint E0502, unsafe E0503, @no_alloc, shapes)
bin/axc.exe check --json path/to/app.ax

# 2. Run built-in @test unit tests
bin/axc.exe test path/to/app.ax

# 3. Compile & execute immediately
bin/axc.exe run path/to/app.ax

# 4. Production build with security shield + symbol map
bin/axc.exe build path/to/app.ax -o app.exe --shield --emit-map

# 5. Query stdlib via MCP (when axc mcp is connected): axon_docs, axon_spec, axon_fix, axon_check
```

For full copy-paste production recipes, read [docs/AGENT_FULLSTACK_GUIDE.md](docs/AGENT_FULLSTACK_GUIDE.md) and [AXON_LLM_SPEC.md](AXON_LLM_SPEC.md).

---

## 2. "Need X? Use THIS Axon Module" (Zero C / Zero Python Matrix)

| Task | Exact Axon Module(s) | Key APIs |
|---|---|---|
| **TCP / UDP Sockets, DNS, HTTP/1.1** | `use std::net`, `use std::http` | `tcp_listen`, `tcp_accept`, `tcp_connect`, `tcp_send_bytes`, `tcp_send_str`, `tcp_recv`, `tcp_recv_exact`, `udp_bind`, `http_fetch_plain`, `http_parse_request` |
| **TLS 1.2/1.3, OpenSSL, HTTPS** | `use std::tls` | `https_get`, `https_post_json`, `https_request` (WinHTTP Schannel out-of-the-box on Windows + OpenSSL on Linux/macOS), `tls_ctx_client`, `tls_ctx_server`, `tls_ctx_set_alpn_h2`, `tls_wrap_client`, `tls_read`, `tls_write_bytes` |
| **PostgreSQL** | `use std::postgres` | Pure-Axon Wire v3.0 (`pg_connect`, `pg_query`, `pg_encode_extended_query`, `pg_result_get`, `pg_close`) + `libpq` FFI (`libpq_load`, `libpq_available`) |
| **MySQL / MariaDB** | `use std::mysql` | Pure-Axon Wire v10 (`mysql_connect`, `mysql_query`, `mysql_ping`, `mysql_result_get`, `mysql_close`, `mysql_sha1`) + `libmysql_available` |
| **SQLite3 (Embedded SQL)** | `use std::sqlite` | `sqlite_open(":memory:")`, `sqlite_exec`, `sqlite_prepare`, `sqlite_bind_i64`, `sqlite_bind_text`, `sqlite_step` (`100`=ROW, `101`=DONE), `sqlite_col_i64`, `sqlite_col_text`, `sqlite_finalize`, `sqlite_close` (uses built-in `winsqlite3.dll` on Windows!) |
| **gRPC + HTTP/2 + Protobuf v3** | `use std::grpc` | `pb_write_i64`, `pb_write_sint64`, `pb_write_str`, `pb_decode`, `pb_get_i64`, `pb_get_str`, `h2_encode_frame`, `h2_recv_frame`, `hpack_encode_grpc_request`, `grpc_connect`, `grpc_unary_call`, `grpc_listen`, `grpc_server_accept_unary`, `grpc_server_reply` |
| **Cross-Platform Desktop GUI** | `use std::gui`, `use std::ui` | `ui_window_html`, `gui_run_ui_app`, `gui_open`, `gui_poll_events`, `gui_present_canvas`, `gui_msgbox`, `gui_clipboard_set`/`get`, `gui_file_open_dialog` (Win32 `user32`/`gdi32` + Linux/macOS `SDL2`) |
| **Bind ANY C DLL/SO/Dylib** | `use std::ffi` | `ffi_open`, `ffi_sym`, `ffi_call0`..`ffi_call8`, `ffi_call_arr` (up to 16 args Win64 + SysV ABI!), `ffi_cstr`, `ffi_wstr`, `ffi_read_cstr`, `ffi_alloc`, `ffi_free`, `ffi_sext32` |
| **OS Env, CWD, FS, Process** | `use std::sys`, `use std::path`, `use std::cli` | `sys_env_get`, `sys_env_set`, `sys_cwd`, `sys_pid`, `sys_fs_exists`, `sys_fs_mkdir`, `sys_fs_remove`, `sys_fs_size`, `sys_exec_capture`, `fs_read_file`, `fs_write_file` |
| **Generics (Zero-Cost)** | `use std::vec` | `struct Box[T]`, `Pair[A, B]`, `Option[T]`, `some[T]`, `none_opt[T]`, `unwrap_or`, `min_val`, `max_val`, `clamp_val`, `swap_pair` |
| **JSON, Regex, Compression, Encoding** | `use std::json`, `use std::regex`, `use std::compress`, `use std::encoding` | SWAR JSON parser + `to_json(val)`, regex engine, LZ/RLE compression, Base64/Hex/UTF-8 |
| **Crypto, Hashing, UUID, Security** | `use std::crypto`, `use std::uuid`, `use std::taint`, `use std::cap` | SHA-256, constant-time `ct_eq_str`, `@whitebox`, `tainted[str]` (`E0502`), `RootCap`/`FsCap`/`NetCap` (`E0501`) |
| **Deep Security Testing, Bug Hunting & Oracles (v3.0)** | `use std::sec_test` (`sec_corpus`, `sec_fuzz`, `sec_oracle`, `sec_sast`, `sec_meta`, `sec_instr`, `sec_mutate`) | Greybox 64KB-bitmap fuzzer + REDQUEEN/Range/XOR CmpLog (`fuzz_cmplog_i64`/`str`/`range`/`xor`) + LibAFLGo Directed DGF (`fuzz_report_distance`) + MOpt bandit, Zero-edit Auto-Instrumenter (`sec_instr_source`/`file`), Token+Data-Flow SAST v2 (`sast_scan_dir`/`sec_hunt_sast_dir`, 16 CWE rules), Metamorphic Bypass Oracles (`meta_url_variants`/`path`/`http`/`jwt`), Security Mutation Tester (`sec_mutate_source`), Stateful sequence fuzzer + `ddmin` (`seq_fuzz_*`/`shrink_seq_*`), `Dudect` Welch t-test, Complexity/ReDoS, ASan-lite redzone canary (`rz_*`), Crash Vault + Backtrace Dedup (`sec_vault_*`), `SecReport` (Console/Markdown/JSON/**SARIF 2.1.0**) |
| **O(N) Sort, Binary Search, Heap, Union-Find** | `use std::algo` | `sort_i64` ($O(N)$ LSD Radix Sort), `sort_i64_range`, `bsearch_i64`, `lower_bound_i64`, `upper_bound_i64`, `ibuf_dedup_sorted`, `heap_new`/`heap_push_min`/`heap_pop_min`, `heap_kv_new`/`heap_push_kv_min`/`heap_pop_kv_min`, `dsu_new`/`dsu_find`/`dsu_union` |
| **Auto-Growing Vec, Map, StrMap, Deque, BitSet** | `use std::ds` | `DynIntVec` (`dvec_*`), `StrVec` (`svec_*`), `DynMap` (`dmap_*`), `StrMap` (`smap_*`), `IntDeque` (`deque_*`), `BitSet` (`bitset_*`) |
| **Graphs (BFS, Dijkstra, Topological Sort)** | `use std::graph` | `graph_new`, `graph_add_edge`, `graph_bfs`, `graph_dijkstra`, `graph_toposort` (cycle-detecting!), `graph_free` |
| **Web Router, JWT, WebSockets, URL/SSRF Guard** | `use std::web`, `use std::jwt`, `use std::ws`, `use std::url` | `web_route_match`, `web_param`, `web_json`, `web_bearer_token`, `web_serve_static`, `jwt_sign_hs256`, `jwt_verify_hs256`, `ws_accept_key`, `ws_encode_frame`, `ws_decode_frame`, `url_parse`, `url_query_get`, `url_str_is_public_http` |
| **Filesystem, Formatting & Benchmarking** | `use std::fs`, `use std::fmt`, `use std::bench` | `fs_read_text`, `fs_atomic_write`, `fs_mkdir_all`, `fs_read_lines`, `fs_list_dir`, `fmt_hex`, `fmt_bin`, `fmt_commas`, `fmt_f64`, `fmt_bytes_human`, `bench_compute_stats` |
| **AI / LLM / CUDA GPU / Vector DB** | `use std::cuda`, `use std::ai_engine`, `use std::ai_gguf`, `use std::ai_lora`, `use std::faiss`, `use std::llama`, `use std::torch` | Zero-nvcc PTX JIT CUDA (`nvcuda.dll`/`libcuda.so`), GGUF loader, Q4/Q8/BitNet/BNN AVX2 kernels, LoRA, FAISS L2 search |
| **2D / 3D Game Engine & Physics** | `use std::game2d`, `use std::game3d`, `use std::game_ecs`, `use std::game_physics`, `use std::math3d`, `use std::audio` | SoA `Table[T]` SIMD (`AVX2`/`NEON`), ECS, rigid-body physics, `vec3`/`mat4`/`quat`, audio |

---

## 3. Critical Axon Grammar & Safety Rules (Avoid Common Agent Mistakes)

1. **Boolean operators & reserved keywords:** Use `and`, `or`, `not` (NEVER `&&`, `||`, `!`), and NEVER name a variable `pass` (`pass` is a reserved no-op keyword -> `E0020`). Use `+%`, `-%`, `*%` for intentional 64-bit modular wrap-around (`--shield` traps signed overflow on `+`, `-`, `*`).
2. **Error unions (`T ! i64`):**
   - Return: `return ok(val)` / `return err(-1)`.
   - Unwrap: `x := foo() or return` (propagate), `x := foo() or return -1` (return value), or `x := foo() or default_val` (fallback expression).
   - **NEVER** write `or { block }` — `or` takes a single expression or `return [expr]`.
3. **Mutability & Ownership (Perceus 0-GC):**
   - Immutable by default: `x := 10`. Mutable: `mut x := 10`.
   - Named struct fields & args use `=` (not `:`): `Point(x = 1, y = 2)`.
   - Pass mutable params with `mut arg`; pass owned `ByteBuf`/`IntBuf` to `buf_free(move b)` or `buf_to_owned_str(move b)`.
   - Never move an outer variable inside a `while`/`for` loop (`E0203`), and never store/return `&T` (`E0101`/`E0102`).
4. **Enforced `unsafe` Boundary (`E0503`):**
   - High-level `std::*` APIs (`std::net`, `std::tls`, `std::sqlite`, `std::postgres`, `std::mysql`, `std::grpc`, `std::gui`, `std::ffi`, `std::sys`) are **100% safe to call** — no `unsafe` block needed in application code!
   - Direct calls to raw intrinsics (`ptr_read_*`, `ptr_write_*`, `mem_alloc_aligned`, `mem_free_aligned`, `dll_load`, `dll_sym`, `c_call_v`, Ring-0/MMIO) or `unsafe fn` MUST be inside `unsafe { ... }` or an `unsafe fn` (otherwise rejected with `E0503`).
5. **Fixed-Capacity vs Owned Buffers:**
   - `buf_new(cap)` / `ibuf_new(cap)` allocate fixed capacity `cap` (with bounds-trapped `buf_get`/`ibuf_get` and runtime stack backtrace on OOB!). Always size `cap` generously for expected elements, or wrap with `std::collections`.
6. **Zero-Cost Generics:**
   - Declare `struct MyBox[T] { val: T }` or `fn pick[T](a: T, b: T) -> T { ... }`.
   - Instantiate explicitly (`MyBox[i64](42)`, `pick[f64](1.0, 2.0)`) or let Axon infer generic fn type args (`pick(10, 20)`).
7. **Debugging & Symbols:**
   - Any OOB / overflow / NULL trap automatically prints an exact PIE-safe **Stack Backtrace** (`#0 fn_name -> #1 caller -> #2 main`) with exit code `101`.
   - Use `axc build app.ax --emit-map -o app.exe` to generate `app.map` with function offsets, sizes, and source lines.
