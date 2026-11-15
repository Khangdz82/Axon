# AXON PROGRAMMING LANGUAGE (`.ax`) : PROJECT MEMORY (v1.0.0)

## Objective
- Hoàn thiện phiên bản chính thức **Axon (`.ax`) v1.0.0** — ngôn ngữ lập trình hệ thống đa nền tảng viết 100% bằng chính Axon (`compiler_ax/`, `17` module, `23,763` dòng), 0 phụ thuộc `LLVM`/`Clang`/`GCC`/`link.exe`/`lld`/`CRT`, kích thước mỗi bộ biên dịch **dưới 1 MB** trên Windows/Linux/macOS, kèm bộ thư viện chuẩn toàn diện (`34` module, `9,710` dòng), backend `--target wasm32` (`SIMD128` + `WASI` không cần SDK), Playground WebAssembly tĩnh trong trình duyệt (`playground/index.html`), và kiểm chứng bảo mật production + benchmark `B1–B10` trung thực.

## Important Details & Verified Invariants (v1.0.0)
1. **11 Đích Biên Dịch Chéo Mã Máy Trực Tiếp & Binary `< 1 MB` Với `--shield`:**
   - [`bin/axc.exe`](bin/axc.exe): **`996,864 B`** (**`973.50 KB`**), tự biên dịch `--shield` trong `399 ms` (`471 units`).
   - [`bin/axc-linux-x64`](bin/axc-linux-x64): **`950,960 B`** (**`928.67 KB`**), biên dịch chéo `--shield` trong `359 ms`.
   - [`bin/axc-macos-arm64`](bin/axc-macos-arm64): **`737,344 B`** (**`720.06 KB`**), biên dịch chéo `--shield` trong `329 ms`.
   - `examples/hello.ax`: `axc check` **`565 us`**, xuất `PE32+` **`5,120 B`** (**`5.00 KB`**) trong **`1 ms`**.
2. **Self-Hosting 3 Vòng Xác Định Bit-Exact (`v1.0.0` `--shield`):**
   - `Stage 2 SHA256 == Stage 3 SHA256 == D9B85D3CB1FE4E66C4198DC1BF2FC37CC9168B4169FA3669B6AAC340C2984677`.
3. **Sửa Lỗi Bảo Mật Sâu & Hardening (`v1.0.0`):**
   - Sửa trap `--shield` khi fold hằng `-9223372036854775808` (`INT64_MIN`): đổi `0 - v` sang `0 -% v` trong `compiler_ax/sema_perceus.ax`, `compiler_ax/comptime.ax`, và `compiler_ax/c_import.ax`.
   - `std/http.ax` (`http_is_safe_path`): chặn backslash `\`, traversal segment `..`, `%5c`/`%5C`, `%2e`/`%2E`, NUL và ký tự điều khiển `< 32`.
   - Sửa capacity `ByteBuf` trường hợp xấu nhất trong `std/encoding.ax` (`html_escape`: `n * 6 + 8` cho `&quot;`) và `std/log.ax` (`log_sanitize`: `n * 2 + 8` cho escapes).
   - Hệ thống runtime fail-closed (`rc = 101`): chia `0`, modulo `0`, `INT64_MIN / -1`, `buf_get` OOB, `str_byte` OOB, và tràn số nguyên dưới `--shield`.
4. **Playground Trình Duyệt & Thư Viện Chuẩn Production (`34` Module, `9,710` Dòng):**
   - [`playground/index.html`](playground/index.html): Trình biên dịch + thực thi Axon -> `W3C WASM32 + 0xFD SIMD128 + WASI preview1` viết thuần HTML/JS (`0` dependency, không cần tải compiler), 4 mẫu tương tác (**1M SIMD Particles**, **`T ! Error`**, **Small AI Inference**, **Mini 2D Game**), Canvas2D trực tiếp & hex inspector (`~1.2–2.1 KB`, `< 1 ms` warm compile).
   - 7 module stdlib mới v1.0.0: `std/math.ax`, `std/rand.ax`, `std/path.ax`, `std/encoding.ax`, `std/log.ax`, `std/time.ax` (chạy song song Native + `--target wasm32`) và `std/cli.ax` (Native CLI).
   - [`docs/BENCHMARK_REPORT.md`](docs/BENCHMARK_REPORT.md): Báo cáo `B1–B10` trung thực đo thực tế (AVX2 vector dot `B1b` `15.67 ms`, Particle `Table[Body]` `B2` `5.11 ms`, GEMM `B3` `12.97 ms`, Build speed `B10` `15.90 ms` nhanh hơn `19x–100x`, kèm phân tích rõ vùng cần tối ưu tiếp ở `B1a`, `B4`, `B5`, `B6`, `B9` WASM bump allocator).

## Work State
### Completed
- Phát hành chính thức Axon `v1.0.0` (`axon.toml`, `axon.lock`, `install.ps1`, `install.sh`, `compiler_ax/main.ax`, `compiler_ax/pkg_mcp.ax`, `std/`, `docs/BENCHMARK_REPORT.md`, `playground/index.html`, `README.md`, `memory.md`).
- Toàn bộ bộ kiểm thử `tests/pass/` (`27` bài + `--shield`), `tests/fail_compile/` (`25` cổng), `tests/wasm/` + `wasm32` (`3` bài WASI), runtime trap matrix (`6` bẫy `rc=101`), và `playground/index.html` Node WASM verification (`4/4`) đạt chuẩn trước/sau build release.

### Active / Blocked
- (none)

## Next Move
1. Tối ưu register allocation vòng lặp số nguyên phụ thuộc (`B1a`), inline atomic trong `std/ringbuf.ax` (`B4`), và bổ sung free-list allocator cho backend `wasm32` (`B9`).
2. Thêm `io_uring` cho Linux kernel >= 6.1 và debug info DWARF v5 / CodeView PDB (`--debug`).
