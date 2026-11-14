# MASTER ENGINEERING PLAN: AXON (`.ax`) — MILESTONES M6, M7 & M8
**Mục tiêu tối thượng:** Biến Axon thành ngôn ngữ lập trình hệ thống tự chủ 100% (Self-Hosted), không phụ thuộc LLVM/Rust/C/Python, dung lượng trình biên dịch dưới `3 MB`, tốc độ tự biên dịch dưới `150 ms`, và hiệu năng thực thi thực tế vượt trội so với `C++23 (-O3)` và `Rust (--release)`.

---

## PHẦN 0: LUẬT CHỐNG GIAN LẬN BẮT BUỘC (ANTI-CHEAT GUARDRAILS)
AI Agent bị **CẤM TUYỆT ĐỐI** thực hiện các hành vi đi đường tắt sau đây. Nếu vi phạm bất kỳ điều nào, toàn bộ Milestone đó bị hủy bỏ và phải làm lại:

1. **CẤM gọi trình biên dịch hoặc linker bên ngoài (`Zero External Toolchain`):**
   - Trong toàn bộ mã nguồn `compiler_ax/` và `std/`, nghiêm cấm gọi tiến trình con (`exec`, `spawn_process`, `system`, `Command`) tới `gcc`, `clang`, `zig`, `rustc`, `ld`, `lld`, `link.exe`, `nasm`, `as`, hoặc `python`.
   - `axc` bắt buộc phải tự sinh trực tiếp mã máy (`x86_64` / `ARM64`) và tự đóng gói trực tiếp file thực thi nhị phân (`ELF64` / `PE32+` / `Mach-O`) bằng các thao tác ghi byte thuần túy.
2. **CẤM giữ lại Python, C/C++ hoặc phụ thuộc Rust sau M6:**
   - Sau khi hoàn tất M6, toàn bộ repo chính (ngoại trừ thư mục lưu trữ `bootstrap_archive/`) phải đạt: **`0` file `.py`**, **`0` file `.c`/`.cpp`/`.h`**, **`0` file `.rs`**.
   - File thực thi `axc` sau khi self-host không được phép liên kết động (dynamic link) với bất kỳ thư viện ngoài nào ngoài thư viện lõi tối thiểu của hệ điều hành (`libc` / `libSystem` / `kernel32.dll`).
3. **CẤM viết hàm rỗng, mã giả hoặc tối ưu hóa giả (`No Stubs / No Fake Passes`):**
   - Nghiêm cấm để lại `todo`, `unimplemented`, `panic("not yet")` hoặc bỏ qua các pha tối ưu hóa (Perceus Reuse, SoA Auto-SIMD, Refinement Bounds Elision).
   - Mọi tối ưu hóa phải được kiểm chứng bằng **mã Assembly đầu ra thực tế** và **đo đếm syscall cấp phát bộ nhớ (`mmap`/`brk`/`VirtualAlloc`)**.
4. **CẤM làm vỡ 6 bất biến kiến trúc từ M0–M5:**
   - Phiên bản trình biên dịch viết bằng Axon (`compiler_ax/`) bắt buộc phải giữ nguyên 100% luật an toàn của bản gốc: **Không GC**, **Không ký hiệu lifetime `'a`**, **Second-Class References**, **Perceus In-Place Reuse**, **Native SoA `Table[T]`**, **Local Reasoning song song**, **Colorless Concurrency (`isolated`)**, và **Toán học xác định Zero-UB**.
