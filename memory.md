# AXON PROGRAMMING LANGUAGE (`.ax`) : PROJECT MEMORY (v1.0.0)

## Objective
- Duy trì và phát triển **Axon (`.ax`) v1.0.0** — ngôn ngữ lập trình hệ thống đa nền tảng viết 100% bằng chính Axon (`compiler_ax/`, `17` module, `23,763` dòng), `0` phụ thuộc `LLVM`/`Clang`/`GCC`/`link.exe`/`lld`/`CRT`, kích thước mỗi binary biên dịch **dưới 1 MB**, kèm `34` module `std/` (`9,710` dòng), Playground WebAssembly trình duyệt ([`playground/index.html`](playground/index.html)), và hệ thống bảo mật fail-closed.

## Verified Architecture & Release Invariants (`v1.0.0`)
1. **Toolchain `< 1 MB` Đa Hệ Điều Hành & Self-Hosting Bit-Exact:**
   - [`bin/axc.exe`](bin/axc.exe): **`996,864 B`** (**`973.50 KB`**, `--shield`, tự biên dịch `399 ms`).
   - [`bin/axc-linux-x64`](bin/axc-linux-x64): **`950,960 B`** (**`928.67 KB`**, build `359 ms`).
   - [`bin/axc-macos-arm64`](bin/axc-macos-arm64): **`737,344 B`** (**`720.06 KB`**, build `329 ms`).
   - Hội tụ 4 vòng `--shield`: `SHA256 = D9B85D3CB1FE4E66C4198DC1BF2FC37CC9168B4169FA3669B6AAC340C2984677`.
   - [`examples/hello.ax`](examples/hello.ax): `axc check` **`565 us`**, xuất `PE32+` **`5,120 B`** (**`5.00 KB`**) trong **`1 ms`**.
2. **Bảo Mật Compile-Time & Runtime Fail-Closed:**
   - Sửa wrapping fold `-9223372036854775808` (`INT64_MIN`: `0 -% v` trong `sema_perceus.ax`, `comptime.ax`, `c_import.ax`) tránh trap giả dưới `--shield`.
   - Chống traversal `std/http.ax` (`http_is_safe_path`: chặn `\`, `..`, `%5c`/`%2e`, NUL) và vá worst-case buffer sizing ở `std/encoding.ax` (`html_escape`: `n*6+8`) & `std/log.ax` (`log_sanitize`: `n*2+8`).
   - 25 cổng tĩnh (`E0020`, `E0101`–`E0204`, `E0501`–`E0502`, `E0601`–`E0604`, ...) và 6 bẫy runtime `exit 101` (`/0`, `%0`, `INT64_MIN/-1`, `buf_get` OOB, `str_byte` OOB, `--shield` add overflow).
3. **Browser Playground, Stdlib `34` Module & Benchmark Trung Thực:**
   - [`playground/index.html`](playground/index.html): Trình biên dịch Axon -> W3C WASM32 + `0xFD SIMD128` + WASI thuần HTML/JS (`1.19–2.05 KB` `.wasm`, `0.824–1.894 ms` compile trong Node VM, Canvas2D + hex inspector).
   - `7` module mới `v1.0.0`: `math`, `rand`, `path`, `encoding`, `log`, `time` (Native + WASM32) và `cli` (Native).
   - [`docs/BENCHMARK_REPORT.md`](docs/BENCHMARK_REPORT.md): Báo cáo `B1–B10` đo thực tế vs MSVC C++23 / Rust 1.100 / Go 1.26.4 / Java 21 / Node V8 / Python 3.11 (`B1b` `15.67 ms`, `B2 Table` `5.11 ms`, `B10` cold build `15.90 ms`, cùng phân tích trung thực `B1a`, `B4`, `B5`, `B6`, `B9` WASM linear allocator).

## Work State
### Completed
- Phát hành chính thức **`v1.0.0`** (commit `b141ebf`, tag `v1.0.0`, bộ cài `install.ps1` `1267.84 KB` -> `axc 1.0.0-selfhosted`).
- Verification sau chỉnh sửa cuối đạt 100%: Native+Shield **`29/29`**, `fail_compile` **`25/25`**, WASM32+WASI **`3/3`**, Trap Matrix **`6/6`**, Playground Node **`4/4`**.
- **Tối ưu Cú pháp Pythonic Chuyên sâu trên nhánh `feature/pythonic-syntax-ergonomics`**:
  - Bổ sung `def` / `fn`, dấu hai chấm `:` tùy chọn ở tiêu đề block, chuỗi nhánh `elif`, lệnh giữ chỗ `pass`.
  - Bổ sung comment `#` chuẩn Python, hằng số `True` / `False`, giá trị `None` / `nil` / `null` kiểu `NoneType`, alias kiểu `int` / `float`.
  - Bổ sung toán tử lũy thừa `**` và gán kết hợp `**=`, `%=` (sinh mã nhị phân x86_64 tính trên thanh ghi CPU, constant folding tại comptime).
  - Bổ sung toán tử 3 ngôi `x if cond else y`, kiểm tra khoảng `x in min..max` & `x not in min..max`, vòng lặp `for i in range(...)` 0 cấp phát heap, hàm tích hợp `len(x)`.
  - Bộ kiểm thử mới [`tests/pass/31_pythonic_deep_syntax.ax`](tests/pass/31_pythonic_deep_syntax.ax) chạy qua 100% tất cả 9 bước; toàn bộ test suite pass 100%.

### Active / Blocked
- (none)

## Next Move
1. Tối ưu register allocation cho chuỗi số nguyên phụ thuộc (`B1a`), inline atomics trong `std/ringbuf.ax` (`B4`), và free-list allocator cho backend `wasm32` (`B9`).
2. Bổ sung `io_uring` (Linux kernel >= 6.1) và debug symbols DWARF v5 / CodeView PDB (`--debug`).
