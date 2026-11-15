# AXON PROGRAMMING LANGUAGE (`.ax`) : PROJECT MEMORY (v0.9.0)

## Objective
- Đưa **Axon (`.ax`) v0.9.0** thành ngôn ngữ lập trình hệ thống đa nền tảng quy mô lớn (Multi-OS, Scale-First, AI/ML/LoRA, Game 2D/3D, HFT, Security, Embedded), 100% viết bằng chính Axon (`compiler_ax/`, 17 module, `20,321` dòng), 0 phụ thuộc `LLVM`/`Clang`/`GCC`/`link.exe`/`lld`/`CRT`, vừa giữ cú pháp Pythonic dễ đọc dễ học vừa đạt kích thước bộ biên dịch ở mức **kilobyte (< 1 MB)** trên tất cả các nền tảng.

## Important Details & Verified Invariants (v0.9.0)
1. **11 Đích Biên Dịch Chéo Mã Máy Trực Tiếp & Bộ Cài Nhẹ Mức KB (< 1 MB):**
   - `win-x64` (`PE32+` `.exe` & `--shared` `.dll`/`.pyd`, Multi-DLL `.idata`, `.edata`, `.pdata` SEH, strict `W^X`), `linux-x64` (`ELF64` direct `SYSCALL`), `linux-arm64` (`ELF64` AArch64 + `NEON`), `macos-arm64` (`Mach-O 64` 16KB page-aligned + `NEON`), `macos-x64` (`Mach-O 64`), `wasm32` (`WASI` + `0xFD` `SIMD128`), `unikernel-x64` (`Multiboot2` `0xE85250D6`), `freestanding-x64`, `freestanding-riscv64` (`RV64GC` `e_machine=243`), `spirv` (Khronos `SPIR-V 1.6` `.spv`), và `ptx` (NVIDIA `PTX 8.0` `.ptx`).
   - Tối ưu ABI thanh ghi 6-slot, mã hóa trực tiếp phép toán hằng `x86_64` immediate-RHS, gộp helper runtime dùng chung (`10000020..10000032`) và hạ floor cấp phát trang đầu xuống `4096 B` / `512 ints` (`2048`/`512` compiler slots):
     - [`bin/axc.exe`](bin/axc.exe): **`886,272 B`** (**`865.50 KB`**, giảm `-33.5%` từ `1,333,760 B`, tự biên dịch `575-579 ms`).
     - [`bin/axc-linux-x64`](bin/axc-linux-x64): **`918,192 B`** (**`896.67 KB`**).
     - [`bin/axc-macos-arm64`](bin/axc-macos-arm64): **`639,040 B`** (**`624.06 KB`**).
     - `examples/hello.ax`: kiểm tra **`395 us`**, xuất executable **`5,120 B`** (**`5.00 KB`**) trong **`1 ms`**.
2. **Self-Hosting 3 Vòng Xác Định Bit-Exact (v0.9.0):**
   - `s1 -> s2 -> s3` (`886,272 bytes`, `408 units`, `575-579 ms`).
   - `SHA256(s2) == SHA256(s3) == 6B4A8BC2DBC553B23EFEA00F595E2829C58AD92E9B8C33C6F99BB09CFF4E8317`.
3. **Mở Rộng Hệ Thống An Toàn Quy Mô Lớn (> 6,000 Hàm) & Bảo Mật Sâu:**
   - Tách ID helper nội bộ lên `10000000+` (`codegen_runtime.ax`, `codegen_x86_64.ax`, `codegen_arm64_rv64.ax`), loại bỏ triệt để lỗi tràn/đụng ID hàm từ hàm thứ `3,000` trở lên và OOB mảng trạng thái trên ARM64.
   - Mở rộng động sức chứa `toks` & `mtoks` trong `compiler_ax/main.ax` theo kích thước mã nguồn (`max(65536, buf_len(src) * 4 + 32768)`), đã kiểm chứng biên dịch và chạy thành công chương trình tới **6,000 hàm** (`N=6000`).
   - Bẫy tràn kích thước/biên/NULL ở các trình nạp `GGUF`/`AXM`, `LoRA`, `VramArena`, đường dẫn FS, `h_free_mem`, `dec`, `tainted[str]` (`E0502`), `@no_alloc` (`E0601`), `@nan_guard`, `@shield` v2, `@whitebox` $\text{GF}(2^8)$.
4. **Hệ Sinh Thái AI/ML/DL/LoRA/Serve & Game Engine 2D/3D Thuần Axon Zero-GC:**
   - **AI/LLM (`std/ai*.ax` — 9 module):** `ai`, `ai_engine`, `ai_math` (FlashAttention tile + Loss + RoPE + RMSNorm + SwiGLU), `ai_quant` (Q4/Q8/NF4/BF16/FP16), `ai_autograd` (AdamW/Lion), `ai_nn` (Transformer GQA/MoE + K-Means/PCA/Linear/Logistic ML), `ai_lora` (LoRA/DoRA/QLoRA merge & checkpoint), `ai_local` (GGUF/AXM loader + BPE tokenizer), `ai_serve` (KV-cached inference server).
   - **Game Engine 2D/3D (`std/game.ax`, `std/game2d.ax`, `std/game3d.ax`, `std/math3d.ax`):** SAT/AABB/Circle 2D physics, spatial hash, camera/sprites/A*/particles; 3D quaternion SLERP skinning, frustum culling, Möller-Trumbore ray-triangle + BVH, PBR + CSM shadow maps, và generational ECS.
5. **Nền Tảng Hạ Tầng v7 Kế Thừa (`comptime`, `@cImport`, `COFF .obj`, `CPUID`, `par_for`, `IOCP`, `axpm`, `axc mcp`):**
   - Toàn bộ tính năng `comptime` `.rdata` LUT, `@cImport`, `par_for`, `IOCP`, `axpm` và `axc mcp` (`v0.9.0`) giữ nguyên ổn định và hội tụ bit-exact.

## Work State
### Completed
- `compiler_ax/` v0.9.0: 17 module thuần Axon (`20,321` dòng), hội tụ 3 vòng SHA-256 tuyệt đối (`6B4A8BC2DBC553B23EFEA00F595E2829C58AD92E9B8C33C6F99BB09CFF4E8317`).
- `std/` v0.9.0: **23 module** thuần Axon (`6,398` dòng: `ai*`, `game*`, `math3d`, `async_io`, `decimal`, `ringbuf`, `taint`, `cap`, `crdt`, `crypto`, `collections`, `json`, `os`).
- Kiểm thử tự động 100% thực thi thật: **21/21** bài `tests/pass/01..21_*.ax` (`exit 0`, gồm cả `19_v7_ai_full_suite.ax` `6/6`, `20_v7_game2d_3d.ax` `4/4`, `21_security_large_scale.ax` `3/3`), và **22/22** cổng an toàn `tests/fail_compile/` bị chặn chính xác.

### Active / Blocked
- (none)

## Next Move
1. Bổ sung gói cộng đồng `axpm` và backend `io_uring` đa luồng cho Linux kernel >= 6.1.
2. Tùy chọn thêm symbol gỡ lỗi DWARF v5 / CodeView PDB (`--debug`) song song với `axc mcp`.
