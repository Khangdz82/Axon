# Axon v1.0.0 Full-Stack Production Guide for AI Coding Agents

This guide provides **complete, verified, copy-pasteable recipes** for building real-world applications **100% in pure Axon** — covering Networking, TLS/OpenSSL/HTTPS, PostgreSQL, MySQL, SQLite3, gRPC/Protobuf/HTTP2, Cross-Platform Desktop GUI, Arbitrary C-DLL FFI, OS/Filesystem/Env, Generics, AI/CUDA, and Security — with **zero C or Python fallbacks**.

---

## 1. TCP/UDP Networking & HTTP Server/Client (`std::net`, `std::http`)

```axon
use std::net
use std::http

fn run_echo_server(port: i64) -> i64 ! i64 {
    ln := tcp_listen("127.0.0.1", port, 128) or return
    println(f"Listening on 127.0.0.1:{port}")
    client := tcp_accept(ln) or return
    _ := tcp_set_nodelay(client, true)
    req_bytes := tcp_recv(client, 8192) or buf_new(0)
    req_str := buf_slice_str(req_bytes, 0, buf_len(req_bytes))
    buf_free(move req_bytes)

    resp := http_build_response(200, "application/json", "{\"status\":\"ok\",\"engine\":\"axon\"}")
    _ := tcp_send_str(client, resp) or -1
    tcp_close(client)
    tcp_listener_close(ln)
    return ok(0)
}
```

---

## 2. TLS 1.2/1.3 & HTTPS (`std::tls`)

### A. Zero-Dependency Native HTTPS Client (WinHTTP Schannel on Windows, OpenSSL on Linux/macOS)
```axon
use std::tls

fn fetch_api() -> i64 ! i64 {
    res := https_get("api.github.com", "/zen") or return
    println(f"HTTP {res.status}: {res.body}")

    post_res := https_post_json("httpbin.org", "/post", "{\"lang\":\"axon\"}") or return
    println(f"POST {post_res.status}: {post_res.body}")
    return ok(0)
}
```

### B. OpenSSL 3.x / 1.1.x Client & Server Sockets (SNI + ALPN `h2` + Peer Verification)
```axon
use std::net
use std::tls

fn connect_tls_h2(host: str, port: i64) -> i64 ! i64 {
    ctx := tls_ctx_client() or return
    _ := tls_ctx_set_verify_peer(ctx, "ca-bundle.pem")
    _ := tls_ctx_set_alpn_h2(ctx)
    sock := tcp_connect(host, port) or return
    ts := tls_wrap_client(ctx, sock, host) or return
    println(f"Negotiated TLS: {tls_version(ts)}")
    _ := tls_write_str(ts, "GET / HTTP/1.1\r\nHost: example.com\r\nConnection: close\r\n\r\n") or -1
    data := tls_read(ts, 4096) or buf_new(0)
    println(buf_slice_str(data, 0, buf_len(data)))
    buf_free(move data)
    tls_close(ts)
    tls_ctx_free(ctx)
    return ok(0)
}
```

---

## 3. Databases: SQLite3, PostgreSQL, MySQL (`std::sqlite`, `std::postgres`, `std::mysql`)

### A. Embedded SQLite3 (Built-in `winsqlite3.dll` on Windows / `libsqlite3.so` on Linux/macOS)
```axon
use std::sqlite

fn demo_sqlite() -> i64 ! i64 {
    db := sqlite_open("app.db") or return // or ":memory:"
    _ := sqlite_exec(db, "CREATE TABLE IF NOT EXISTS items (id INTEGER PRIMARY KEY, name TEXT, qty INTEGER);")

    // SQL-injection-proof prepared INSERT
    ins := sqlite_prepare(db, "INSERT INTO items (name, qty) VALUES (?, ?);") or return
    _ := sqlite_bind_text(ins, 1, "Axon Compiler")
    _ := sqlite_bind_i64(ins, 2, 100)
    _ := sqlite_step(ins) // 101 = SQLITE_DONE
    sqlite_finalize(ins)

    // Prepared SELECT
    sel := sqlite_prepare(db, "SELECT id, name, qty FROM items WHERE qty > ?;") or return
    _ := sqlite_bind_i64(sel, 1, 10)
    while sqlite_step(sel) == 100 { // 100 = SQLITE_ROW
        println(f"row: id={sqlite_col_i64(sel, 0)} name={sqlite_col_text(sel, 1)} qty={sqlite_col_i64(sel, 2)}")
    }
    sqlite_finalize(sel)
    sqlite_close(db)
    return ok(0)
}
```

### B. PostgreSQL Wire Protocol v3.0 (Zero external DLL required) + `libpq` FFI
```axon
use std::postgres

fn demo_postgres() -> i64 ! i64 {
    conn := pg_connect("127.0.0.1", 5432, "postgres", "secret", "mydb") or return
    res := pg_query(conn, "SELECT id, email FROM users LIMIT 10;") or return
    mut r := 0
    while r < res.row_count {
        println(f"{pg_result_get(res, r, 0)} -> {pg_result_get(res, r, 1)}")
        r += 1
    }
    pg_close(conn)
    return ok(0)
}
```

### C. MySQL / MariaDB Wire Protocol v10 (`mysql_native_password` SHA-1) + `libmysql` FFI
```axon
use std::mysql

fn demo_mysql() -> i64 ! i64 {
    conn := mysql_connect("127.0.0.1", 3306, "root", "secret", "mydb") or return
    if mysql_ping(conn) {
        res := mysql_query(conn, "SELECT version(), 42;") or return
        if res.row_count > 0 {
            println(f"MySQL {mysql_result_get(res, 0, 0)} val={mysql_result_get(res, 0, 1)}")
        }
    }
    mysql_close(conn)
    return ok(0)
}
```

---

## 4. gRPC + HTTP/2 + Protobuf v3 (`std::grpc`)

```axon
use std::grpc

fn call_greeter_rpc() -> i64 ! i64 {
    // Encode Protobuf v3 request: message HelloRequest { string name = 1; sint64 id = 2; }
    req := buf_new(64)
    pb_write_str(req, 1, "Axon")
    pb_write_sint64(req, 2, 42)

    mut ch := grpc_connect("127.0.0.1", 50051) or return
    resp := grpc_unary_call(mut ch, "/helloworld.Greeter/SayHello", req) or return
    buf_free(move req)

    parsed := pb_decode(resp.payload)
    println(f"gRPC status={resp.status} reply={pb_get_str(parsed, 1, '')}")
    buf_free(move resp.payload)
    grpc_close(ch)
    return ok(0)
}
```

---

## 5. Cross-Platform Native Desktop GUI (`std::gui` + `std::ui`)

Build full native desktop applications with HTML5 + CSS + Tailwind + Reactive Signals rendered straight into a real OS window (`Win32 HWND/StretchDIBits` on Windows, `SDL2` on Linux/macOS) — **zero Electron, zero WebView2, ~30 KB executable**:

```axon
use std::ui
use std::gui

fn main() {
    html := "<div class='p-6 bg-slate-900 text-white'>" +
            "<h1 class='text-2xl font-bold'>Axon Native Desktop App</h1>" +
            "<p id='status' class='mt-2 text-slate-300'>100% Pure Axon Win32/SDL2 GUI</p>" +
            "<button id='btn_run' class='mt-4 px-4 py-2 bg-blue-600 text-white'>Click Me</button>" +
            "</div>"
    mut app := ui_window_html("Axon Studio", 960, 640, html)
    _ := gui_run_ui_app(mut app) // Live 60 FPS native window + mouse click hit-testing!
}
```

Native OS dialogs & clipboard:
- `gui_msgbox("Title", "Saved!", 0)`
- `gui_clipboard_set("hello")` / `txt := gui_clipboard_get()`
- `path := gui_file_open_dialog("Select File")`

---

## 6. Binding ANY External C Library Directly in Pure Axon (`std::ffi`)

Never write C glue code. Bind any `.dll`, `.so`, or `.dylib` in 5 lines using `std::ffi`:

```axon
use std::ffi

fn call_custom_c_lib() {
    lib := ffi_open("mylib.dll", "libmylib.so", "libmylib.dylib")
    if lib == 0 { return }
    fn_add := ffi_sym(lib, "my_add_i64")
    res := ffi_call2(fn_add, 40, 2)
    println(f"C returned: {res}")
    ffi_close(lib)
}
```
- Supports `ffi_call0`..`ffi_call8` and `ffi_call_arr(fn_ptr, argc, args_ibuf)` up to **16 arguments** on **both Win64 and Linux/macOS SysV x86_64 ABI**.
- Always sign-extend 32-bit C `int` return values with `ffi_sext32(r)`.

---

## 7. Zero-Cost Monomorphized Generics (`std::vec`)

```axon
use std::vec

struct CacheEntry[K, V] {
    key: K,
    val: V
}

fn main() {
    e := CacheEntry[str, i64]("hits", 42)
    opt := some[i64](e.val)
    v := unwrap_or(opt, 0)         // Type argument inferred!
    clamped := clamp_val(v, 0, 10) // @inline monomorphized, 0 overhead!
    println(f"{e.key}={clamped}")
}
```

---

## 8. OS, Environment, Filesystem & Process Capture (`std::sys`)

```axon
use std::sys

fn main() {
    println(f"PID={sys_pid()} CWD={sys_cwd()}")
    _ := sys_env_set("APP_MODE", "prod")
    println(f"APP_MODE={sys_env_get('APP_MODE')}")
    if sys_fs_exists("axon.toml") {
        println(f"axon.toml size={sys_fs_size('axon.toml')} bytes")
    }
    out := sys_exec_capture("git rev-parse --short HEAD")
    println(f"commit={out}")
}
```
