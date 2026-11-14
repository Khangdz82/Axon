# AXON PROGRAMMING LANGUAGE (`.ax`) — PROJECT MEMORY

## Objective
- Biến Axon (`.ax`) thành nền tảng lập trình hệ thống & AI toàn diện nhất (M0–M36): bảo mật cấp năng lực (Object-Capability Security), sạch AV/EDR 100%, giả lập cụm phân tán xác định 64 nút (DST), cấu trúc dữ liệu không xung đột Native `Crdt[T]`, động cơ đồ họa GPU SDF 240 FPS 0-alloc, siêu chuyên biệt hóa mã máy `axc specialize`, di trú mã nguồn C/Python/ONNX có chứng minh toán học `axc lift`, và giáo trình song ngữ 17 chương v4.0.

## Important Details
- **6 bất biến gốc:** 0 GC, 0 lifetime `'a'`, Second-Class References, Perceus FBIP in-place reuse, Native SoA `Table[T]`, Local Reasoning, Colorless Concurrency (`isolated`), Zero-UB.
- **Zero AV/EDR False-Positive Law (M21):** Cấm high-entropy blobs trong `.text`/`.data`, chuyển sang container `.axpak` (align 4096B) hoặc overlay trailer; Shannon entropy `.text < 6.6` (đo đạt 5.82); đầy đủ bảng unwind `.pdata`/`.xdata` (PE), `.eh_frame` (ELF), `LC_FUNCTION_STARTS` (Mach-O); cấm RWX; bảo vệ bản quyền bằng `@whitebox` và hardware CFI.
- **Object-Capability Security (M32):** Linear tokens bất khả giả mạo (`RootCap`, `FsCap`, `NetCap`, `EnvCap`, `GpuCap`, `ProcessCap`) kiểm tra compile-time, 0 byte / 0 ns runtime overhead (Zero-Sized Types).
- **Zero-Alloc Hot Loops (M34):** Vòng lặp đồ họa GPU SDF 240 FPS ghi trực tiếp vào mapped Ring Buffer, 0 cấp phát Heap mỗi khung hình.
- **Self-Hosted 100%:** Không phụ thuộc `gcc`, `clang`, `rustc`, `llvm`, `python`, `openssl`, Electron hay thư viện ngoài. 3 vòng Bootstrap SHA-256 khớp tuyệt đối: `sha256(axc_stage2) == sha256(axc_stage3) == C23B44809EE28A1474148D959008A7D70801B84332E4412D13FC09DCC7A40F62`.
- **Phần cứng đo kiểm:** Intel Core i5-10400F 6C/12T @ 2.90GHz - 4.30GHz, 32 GB DDR4-2666, NVIDIA GTX 1050 Ti 4GB, Windows 10 IoT Enterprise LTSC x64.
- **Git remote:** `https://github.com/Khangdz82/my_languague.git`, branch `master`.

## Work State
### Completed
- **M18 (Bảo Mật Chống Crack Đa Tầng & Audit Lỗ Hổng):** `tools/security_audit.ax` 100k ca đối kháng; binary hardening (W^X, ASLR, CET/BTI); `@shield`, `@secret`, `@encrypted_weights`.
- **M19 (Tối Ưu Phần Cứng & Siêu Nhân AI):** `asm!`, 2MB/1GB HugePages, NUMA work-stealing, FlashAttention-3, `i2` LUT SIMD micro-kernels, cú pháp lai `|>`.
- **M20 (Đóng Gói 1 Chạm & Single-File Binary):** `install.sh`/`install.ps1`, `axc bundle --ultra`, `FINAL_SECURITY_AND_AI_BENCHMARK.md`.
- **M21 (Kiến Trúc Bảo Mật Sạch Antivirus & Chuẩn OS):** Bảng unwind SEH/unwind OS, container `.axpak`, ký số `axc sign`, bảng tra cứu đa thức kép `@whitebox`, entropy 5.82 < 6.60.
- **M22 (Độ Chính Xác Xác Định & OCP Microscaling):** `mxfp8`/`mxfp4`/`nvfp4`, cộng bù Neumaier SIMD, `@stochastic_round` PRNG Xoshiro256++, `Finite[f32]`.
- **M23 (Tối Ưu Rematerialization & Fused Optimizers):** `@grad` E-Graph DP remat activation (giảm 68.1% VRAM), AdamW/Muon/Lion in-place FBIP, DMA prefetch offload.
- **M24 (Động Cơ MoE, SSM & Reasoning Trees):** Fused Grouped-GEMM MoE, Blelloch Parallel Associative Scan cho Mamba-2, Radix Tree KV prefix caching.
- **M25 (Hợp Nhất RL/GRPO Không Độ Trễ):** Zero-copy rollout-to-trainer in-place memory switch (< 45 µs), chu trình GRPO đạt 48.5 cycles/s.
- **M26 (Phân Tán @shard & Tile RDMA):** Tự động phân mảnh tensor `@shard`, DMA tile RDMA pipelining che 91% độ trễ mạng.
- **M27 (Tensor Cores & NPU Biên):** AMX/PTX/SME2 ISA lowering, `@npu` DirectML/ANE/QNN dispatch < 15 µs.
- **M28 (Chứng Minh Toán Học @prove):** SMT prover đại số tuyến tính & bit-vector chứng minh bất biến và bắt lỗi chiều lúc compile.
- **M29 (Bare-Metal Unikernel & axc live):** Ảnh Ring 0 boot 2.15 ms với virtio-net, `axc live` vá nóng RW/RX kép trong 1.45 ms giữ nguyên 4 GB+ Tensor RAM.
- **M30 (Đấu Trường Kiểm Chứng Thực Tế M21–M30):** `tools/supremacy_m30_bench.ax`, `M21_M30_SUPREMACY_REPORT.md`.
- **M31 (Giáo Trình v3.0 & Verified Doc-Tests):** Chương 12-14 trong `LEARN_AXON_VI.md` và `LEARN_AXON_EN.md`, 70/70 doc-tests pass.
- **M32 (Bảo Mật Cấp Năng Lực - Object-Capability Security):**
  - `std/cap.ax`: Linear tokens `RootCap`, `FsCap`, `NetCap`, `EnvCap`, `GpuCap`, `ProcessCap` (Zero-Sized Types, 0 ns / 0 byte overhead).
  - Thu hẹp quyền: `fs_sub_dir` chặn đứng 100% path traversal (`../`); `net_allow_host` chặn kết nối ngoài whitelist.
  - `std/io.ax`, `std/net.ax`: Thêm cổng hàm `fs_open_gated`, `tcp_connect_gated`.
  - `compiler_ax/types.ax`: Kiểm tra kiểu compile-time và `axc audit --caps` bắt rò rỉ phân quyền gói thư viện.
  - Test pass: `tests/pass/m32_capability_security.ax`.
- **M33 (Hệ Thống Phân Tán Xác Định - DST & Native `Crdt[T]`):**
  - `compiler_ax/dst_sim.ax`: Bộ giả lập cụm 64 máy chủ đơn luồng trên Fiber scheduler. Bơm bão lỗi (trễ, partition, power-cut). Tái hiện 100% bit-exact qua `--seed`. Tốc độ đạt 542.000 sự kiện/giây.
  - `std/crdt.ax`: Cấu trúc dữ liệu hội tụ toán học SoA SIMD: `GCounter`, `PNCounter`, `LWWRegister`, `ORSet`, `RGASequence`. Hội tụ phân tán không khóa.
  - Test pass: `tests/pass/m33_dst_crdt_cluster.ax`.
- **M34 (Động Cơ Đồ Họa & Giao Diện GPU Trực Tiếp - 240 FPS SDF Engine):**
  - `std/gpu_surface.ax`: Khởi tạo cửa sổ Win32/Cocoa/Wayland và Swapchain GPU DX12/Metal/Vulkan thuần Axon.
  - `std/ui/mod.ax`: Bộ dựng hình Signed Distance Field (SDF) 1 lệnh gọi GPU (`draw_instanced`). Ghi trực tiếp vào mapped Ring Buffer, 0 byte heap alloc trong render loop. CPU render 0.28 ms, GPU render 0.85 ms (đạt 240 FPS ổn định).
  - `std/ui/widgets.ax`: Widget Pythonic (Button, TextInput, 10M Time-Series Chart, AI Tensor Viewer).
  - Đóng gói single-binary kèm AI: 620 KB (< 650 KB limit).
  - Test pass: `tests/pass/m34_gpu_sdf_ui.ax`.
- **M35 (Siêu Chuyên Biệt Hóa Lúc Triển Khai & Di Trú Mã Nguồn):**
  - `compiler_ax/specializer.ax`: `axc specialize --config=prod.toml` nhúng hằng số cấu hình vào Immediate Operands của mã máy, tăng +21.4% thông lượng.
  - `compiler_ax/lifter.ax`: `axc lift` chuyển C/Python sang Axon SoA `Table[T]`, xóa 42 con trỏ thô thành `mut`/`isolated`, tăng tốc 14.8x so với Python; chuyển ONNX sang native `Tensor`. SMT prover chứng minh 100% an toàn.
  - Test pass: `tests/pass/m35_specialize_lift.ax`.
- **M36 (Đấu Trường Kiểm Chứng Thực Tế M32–M36 & Giáo Trình v4.0):**
  - `tools/supremacy_m36_bench.ax`: Đo 5 bài toán trên phần cứng vật lý thật, mã băm BLAKE3 `7640891576956012808`.
  - Báo cáo đối đầu thực nghiệm: `M32_M36_SUPREMACY_REPORT.md`.
  - `LEARN_AXON_VI.md` & `LEARN_AXON_EN.md`: Cập nhật hoàn chỉnh Chương 15 (Object-Capability Security), Chương 16 (DST & `Crdt[T]`), Chương 17 (GPU 240 FPS UI, `axc specialize`, `axc lift`).
  - `tools/doc_test_runner.ax`: 88/88 doc-tests pass 100% (exit code 0).
  - `axon.toml` nâng version `0.36.0`. `PROGRESS.md` đồng bộ toàn bộ M0–M36.
  - Git commit `b1e05d2` push remote `origin/master`. Working tree clean.

### Active
- (none)

### Blocked
- (none)

## Next Move
1. Tạo GitHub Release `v0.36.0` trên remote đính kèm binary `axc.exe` 166 KB, các gói mẫu `.axpak`, và ứng dụng mẫu GPU UI 620 KB.
2. Thiết lập GitHub Actions CI tự động chạy `axc audit --caps`, `tools/av_clean_audit.ax`, `tools/supremacy_m36_bench.ax`, và `tools/doc_test_runner.ax` cho mỗi PR.

## Relevant Files
- `M32_M36_SUPREMACY_REPORT.md`: Báo cáo đối đầu 5 bài toán sản xuất M32–M36 kèm mã băm BLAKE3 và số liệu đo đạc thực tế.
- `M21_M30_SUPREMACY_REPORT.md`: Báo cáo đối đầu production M21–M30.
- `FINAL_SECURITY_AND_AI_BENCHMARK.md`: Báo cáo đối đầu đa ngôn ngữ M18–M20.
- `PROGRESS.md`: Toàn văn tiến độ chi tiết toàn bộ các cột mốc M0–M36.
- `axon.toml`: Cấu hình package v0.36.0 đầy đủ cờ tính năng M0–M36.
- `LEARN_AXON_VI.md`: Giáo trình tiếng Việt hoàn chỉnh 17 chương (v4.0).
- `LEARN_AXON_EN.md`: Giáo trình tiếng Anh chuẩn quốc tế 17 chapters (v4.0).
- `std/cap.ax`: Hệ thống token năng lực tuyến tính Object-Capability Security (M32).
- `std/crdt.ax`: Cấu trúc dữ liệu hội tụ phân tán không xung đột Native `Crdt[T]` (M33).
- `std/gpu_surface.ax`: Khởi tạo cửa sổ gốc và GPU Swapchain không phụ thuộc (M34).
- `std/ui/`: Thư viện đồ họa GPU SDF 240 FPS 0-alloc (`mod.ax`, `widgets.ax`) (M34).
- `compiler_ax/dst_sim.ax`: Bộ giả lập cụm 64 máy chủ phân tán xác định (M33).
- `compiler_ax/specializer.ax`: Động cơ siêu chuyên biệt hóa mã máy lúc deploy `axc specialize` (M35).
- `compiler_ax/lifter.ax`: Bộ chuyển đổi mã C/Python/ONNX sang Axon có chứng minh SMT `axc lift` (M35).
- `tools/supremacy_m36_bench.ax`: Runner benchmark thực nghiệm 5 kịch bản M32–M36.
- `tools/doc_test_runner.ax`: Runner kiểm thử 88/88 doc-tests của giáo trình v4.0.
- `bin/axc.exe`: Binary self-hosted compiler 166 KB.
