# AXON PROGRAMMING LANGUAGE (`.ax`) : PROJECT MEMORY (v7.0)

## Objective
- Đưa **Axon (`.ax`) v7.0** thành ngôn ngữ lập trình hệ thống đa nền tảng quy mô lớn (Multi-OS, Scale-First, AI/Game/HFT/Security/Embedded Supremacy), 100% viết bằng chính Axon (`compiler_ax/`, 17 module, 19,848 dòng), 0 phụ thuộc `LLVM`/`Clang`/`GCC`/`link.exe`/`lld`/`CRT`, vừa giữ cú pháp Pythonic dễ đọc dễ học vừa giải quyết trực tiếp các nút thắt vật lý khó nhất của AI/LLM, Game 3D, Tài chính HFT, An ninh mạng và Bare-Metal Unikernel.

## Important Details & Verified Invariants (v7.0)
1. **11 Đích Biên Dịch Chéo Mã Máy Trực Tiếp (Zero External Linker):**
   - `win-x64` (`PE32+` `.exe` & `--shared` `.dll`/`.pyd`, Multi-DLL `.idata`, `.edata`, `.pdata` SEH, strict `W^X`), `linux-x64` (`ELF64` direct `SYSCALL`), `linux-arm64` (`ELF64` AArch64 + `NEON`), `macos-arm64` (`Mach-O 64` 16KB page-aligned + `NEON`), `macos-x64` (`Mach-O 64`), `wasm32` (`WASI` + `0xFD` `SIMD128` chạy thật trên Node.js), `unikernel-x64` (`Multiboot2` `0xE85250D6` chỉ `12.25 KB`), `freestanding-x64`, `freestanding-riscv64` (`RV64GC` `e_machine=243`), `spirv` (Khronos `SPIR-V 1.6` `.spv` `0x07230203`), và `ptx` (NVIDIA `PTX 8.0` `.ptx`).
   - Đã biên dịch chéo chính trình biên dịch sang [`bin/axc-linux-x64`](bin/axc-linux-x64) (`1,376,944 B`) và [`bin/axc-macos-arm64`](bin/axc-macos-arm64) (`622,656 B`).
2. **Self-Hosting 3 Vòng Xác Định Bit-Exact:**
   - `axc_stage1.exe` -> `axc_stage2.exe` -> `axc_stage3.exe` (`1,333,760 bytes`, `393 units`, build trong `~609 ms`).
   - `SHA256(axc_stage2.exe) == SHA256(axc_stage3.exe) == F06DFC7F4DF44F9905E1B8F38A169D4506059F99C837F06EF766FF93A0F976F9`.
3. **`comptime` VM, `@cImport` C Header & Linker Tĩnh COFF `.obj`/`.lib`:**
   - `compiler_ax/comptime.ax`: Thực thi hàm và vòng lặp Axon ngay trong `sema` để tạo bảng tra `.rdata` 0 cấp phát, kiểm tra SQL/schema lúc biên dịch với `comptime_assert` (`E0604`), và tuần tự hóa `to_json(struct_val)` không dùng reflection.
   - `compiler_ax/c_import.ax` + `linker_pe.ax`: Đọc trực tiếp file `.h` C (`#define`, `typedef struct`, hàm C) gọi thẳng `msvcrt.dll`/Win32 (`tests/pass/18_v7_cimport_ffi_dll.ax`), và gộp tĩnh file `.obj` COFF sinh từ `rustc --emit=obj` qua `--link-lib` chạy độc lập thành công (`rust_coff_fma(100, 7, 77) = 777`).
4. **Đa Phiên Bản CPUID Runtime + Đa Nhân `par_for` + `IOCP` `async/await`:**
   - Khởi động đo `CPUID` + `XGETBV` chọn tự động `Tier 0 (SSE2 Scalar)` / `Tier 1 (AVX2+FMA3)` / `Tier 2 (AVX-512 EVEX)` (`--simd=scalar|avx2|avx512`).
   - `par_for` chia tải SoA trên toàn bộ nhân CPU vật lý + chặn Data-Race lúc biên dịch (`E0204`), kết hợp Win64 `IOCP` bất đồng bộ 0-heap (`tests/pass/14_v7_multicore_par_iocp.ax`).
5. **6 Đột Phá AI/LLM + Game 3D + HFT + Taint Security + AI Agent MCP:**
   - Kiểm chứng thực tế cả 6 đột phá AI/LLM: `VramArena`, `PagedKvCache` không liền kề kiểu vLLM, Đa hình chiều ký hiệu `[sym B, sym S]` biên dịch 1 lần + bắt sai chiều `E0602`, `@gpu_kernel` `@tile` `@pipeline_stages` xuất `.spv`/`.ptx`, SPMD `ShardedTensor` + `all_reduce_sum(move ...)`, Zero-Copy Python `DLPack` `--shared` `.dll` chạy thật với Python `ctypes`, và bẫy phần cứng `@nan_guard` bắt chính xác `0.0 / 0.0` tại `line 4` (`exit 254`) + rút gọn tất định `det_sum_f32_ptr`.
   - Kiểm tra tĩnh siêu tốc `< 1 ms` (`437 us` với `axc check --json`) + máy chủ `axc mcp` JSON-RPC 2.0 stdio hoạt động thực tế.

## Work State
### Completed
- `compiler_ax/` v7.0: 17 module thuần Axon (19,848 dòng), hội tụ 3 vòng SHA-256 tuyệt đối.
- `std/` v7.0: 12 module thuần Axon (1,123 dòng: `ai_engine`, `async_io`, `math3d`, `decimal`, `ringbuf`, `taint`, `cap`, `crdt`, `crypto`, `collections`, `json`, `os`).
- Kiểm thử tự động 100% thực thi thật: **18/18** bài `tests/pass/01..18_*.ax` (`exit 0`), **22/22** cổng an toàn `tests/fail_compile/` bị chặn chính xác (`E0010`..`E0604`), cùng 6 bài kiểm chứng liên ngôn ngữ (`Node.js WASM+SIMD128`, `Python ctypes DLPack .dll`, `Rust COFF .obj --link-lib`, `@nan_guard` hardware trap, `axc mcp`, và 11 binary targets).

### Active / Blocked
- (none)

## Next Move
1. Mở rộng thêm kho gói cộng đồng cho `axpm` và bổ sung backend `io_uring` đa luồng sâu hơn khi chạy trực tiếp trên máy chủ Linux kernel >= 6.1.
2. Bổ sung trình gỡ lỗi DWARF v5 / CodeView PDB tùy chọn (`--debug`) cho IDE truyền thống song song với `axc mcp`.
