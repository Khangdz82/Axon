# AXON PROGRAMMING LANGUAGE (`.ax`) : PROJECT MEMORY (v0.9.1)

## Objective
- Đưa **Axon (`.ax`) v0.9.1** thành ngôn ngữ lập trình hệ thống đa nền tảng quy mô lớn (Multi-OS, Scale-First, AI/ML/LoRA, Game 2D/3D, HFT, Security, Web/Backend/CLI), 100% viết bằng chính Axon (`compiler_ax/`, 17 module, `20,705` dòng), 0 phụ thuộc `LLVM`/`Clang`/`GCC`/`link.exe`/`lld`/`CRT`, vừa giữ cú pháp Pythonic dễ đọc dễ học vừa đạt kích thước bộ biên dịch ở mức **kilobyte (< 1 MB)** trên tất cả các nền tảng.

## Important Details & Verified Invariants (v0.9.1)
1. **11 Đích Biên Dịch Chéo Mã Máy Trực Tiếp & Bộ Cài Siêu Nhẹ Mức KB (< 1 MB):**
   - `win-x64` (`PE32+` `.exe` & `--shared` `.dll`/`.pyd`, Multi-DLL `.idata`, `.edata`, `.pdata` SEH, strict `W^X`), `linux-x64` (`ELF64` direct `SYSCALL`), `linux-arm64` (`ELF64` AArch64 + `NEON`), `macos-arm64` (`Mach-O 64` 16KB page-aligned + `NEON`), `macos-x64` (`Mach-O 64`), `wasm32` (`WASI` + `0xFD` `SIMD128`), `unikernel-x64` (`Multiboot2` `0xE85250D6`), `freestanding-x64`, `freestanding-riscv64` (`RV64GC` `e_machine=243`), `spirv` (Khronos `SPIR-V 1.6` `.spv`), và `ptx` (NVIDIA `PTX 8.0` `.ptx`).
   - Tối ưu v0.9.1 (`O(1)` symbol hash lookups, zero-copy `nk_field`/`nk_tuple_idx` projection, scoped block slot reuse, direct immediate `nk_assign`, recursive `is_nonshadowed_const_int`, dynamic `h_jbuf_*`/`h_str_concat`):
     - [`bin/axc.exe`](bin/axc.exe): **`858,624 B`** (**`838.50 KB`**, giảm thêm `-27,648 B` so với `v0.9.0`, tự biên dịch `613-659 ms`, bộ cài `install.ps1` `1098.96 KB`).
     - [`bin/axc-linux-x64`](bin/axc-linux-x64): **`918,192 B`** (**`896.67 KB`**, build `609 ms`).
     - [`bin/axc-macos-arm64`](bin/axc-macos-arm64): **`655,424 B`** (**`640.06 KB`**, build `570 ms`).
     - `examples/hello.ax`: kiểm tra **`354 us`** (`346 us` JSON), xuất executable **`5,120 B`** (**`5.00 KB`**) trong **`1 ms`**.
     - `examples/bench_soa_simd.ax`: **`8,599 us`** (`8.60 ms`); `examples/bench_enterprise_suite.ax`: **`47,368 us`** (`47.37 ms`).
2. **Self-Hosting 3 Vòng Xác Định Bit-Exact (v0.9.1):**
   - `s1 -> s2 -> s3` (`858,624 bytes`, `417 units`, `613-659 ms`).
   - `SHA256(s2) == SHA256(s3) == 1E17D07BEA8721D75325757F36A9CBEAE75E39CB33A662F4EF4885AF7B61DE23`.
3. **Bảo Mật Production Chuyên Sâu & Mở Rộng Quy Mô Lớn (`> 6,000` Dòng/Hàm):**
   - Chống CWE-158 nhúng byte `NUL` trong `fs_read_file` (Win64/Linux), `fs_write_file` (Win64/Linux) và `os_exec`.
   - `h_jbuf_ensure` tự mở rộng động (`f"..."` và `to_json` >4KB không tràn/cắt cụt) + escape JSON chuẩn RFC-8259 (`h_jbuf_qstr`), cùng fix lỗi shadowing hằng `comptime` trong `is_nonshadowed_const_int`.
   - Giới hạn độ sâu đệ quy parser (`p_depth_enter`/`p_depth_leave`, cap `256` -> lỗi sạch `E0020` chống DoS tràn stack).
   - `std/cap.ax` chặn ký tự điều khiển `< 32`, `DEL`, `:`, đuôi `.`/` `; `std/crypto.ax` bổ sung NIST FIPS 180-4 SHA-256, RFC 2104 HMAC-SHA256 và so sánh hằng thời gian `ct_eq_str`.
4. **Thư Viện Chuẩn Toàn Diện `27` Module (`7,841` dòng, Tree-Shaken Zero-GC):**
   - **Thư viện đa dụng mới (`v0.9.1`):** `std/str.ax` (tìm kiếm/trim/case/split/replace/hex/Base64/URL/`buf_to_owned_str`), `std/http.ax` (HTTP/1.1 request/response/route-param/SSE + chống CRLF injection), `std/data.ax` (CSV/CLI flags/JSON logger/`KvStore` bền vững), `std/test_kit.ax` (assertion & microbenchmark suite).
   - **AI/LLM (`std/ai*.ax` — 9 module)** & **Game Engine 2D/3D (`std/game*.ax`, `std/math3d.ax`)** giữ nguyên toàn bộ năng lực Zero-GC và kiểm chứng đầy đủ.

## Work State
### Completed
- `compiler_ax/` v0.9.1: 17 module thuần Axon (`20,705` dòng), hội tụ 3 vòng SHA-256 tuyệt đối (`1E17D07BEA8721D75325757F36A9CBEAE75E39CB33A662F4EF4885AF7B61DE23`).
- `std/` v0.9.1: **27 module** thuần Axon (`7,841` dòng: `str`, `http`, `data`, `test_kit`, `ai*`, `game*`, `math3d`, `async_io`, `decimal`, `ringbuf`, `taint`, `cap`, `crdt`, `crypto`, `collections`, `json`, `os`).
- Kiểm thử production thực thi thật trên `bin/axc.exe`: **23/23** `tests/pass/01..23_*.ax` (`exit 0`, gồm bài 22 `54/54` & bài 23 `17/17`), `axc test tests/pass/17_v7_unit_tests.ax` (`3/3`), **23/23** cổng an toàn `tests/fail_compile/` bị chặn sạch (`exit 1`), và bài đo `N=6000` (`check` `25.44 ms`, `build` `23 ms`).

### Active / Blocked
- (none)

## Next Move
1. Bổ sung gói cộng đồng `axpm` và backend `io_uring` đa luồng cho Linux kernel >= 6.1.
2. Tùy chọn thêm symbol gỡ lỗi DWARF v5 / CodeView PDB (`--debug`) song song với `axc mcp`.
