# MASTER ENGINEERING PLAN: NGÔN NGỮ LẬP TRÌNH HỆ THỐNG THẾ HỆ MỚI (CODENAME: AXON / `.ax`)

## PHẦN I: 6 BẤT BIẾN KIẾN TRÚC (ARCHITECTURAL INVARIANTS — CẤM VI PHẠM)
1. **Zero Garbage Collector & Zero Lifetime Annotations:**
   - Không Tracing GC hay Reference Counting toàn cục lúc chạy.
   - Không ký hiệu lifetime (`'a`) hay con trỏ thô (`*`, `&`).
   - Value Semantics + Second-Class References (`let` đọc, `mut` sửa tại chỗ). Cấm lưu tham chiếu trong `struct`, `Table`, hoặc biến toàn cục.
2. **Perceus In-Place Memory Reuse (FBIP):**
   - Phân tích vòng đời tĩnh lúc biên dịch.
   - Hợp nhất Drop + Alloc thành AllocInPlace ghi đè trực tiếp lên vùng nhớ cũ.
3. **Native Structure-of-Arrays (SoA) mặc định cho tập hợp:**
   - `Table[T]` tách từng trường thành các mảng độc lập căn chỉnh 64-byte cache line.
   - Tự động vector hóa SIMD (AVX-512 / AVX2 / NEON).
4. **Local Reasoning (Suy luận cục bộ 100%):**
   - Tính an toàn của hàm chỉ phụ thuộc chữ ký của chính nó.
   - Song song hóa toàn bộ kiểm tra kiểu qua `rayon`.
5. **Colorless Concurrency & Linear Isolation:**
   - Không `async`/`await`. Mọi hàm viết tuần tự chạy trên M:N Work-Stealing Fibers.
   - Dữ liệu gửi qua luồng mang thuộc tính `isolated` chuyển giao quyền sở hữu duy nhất, triệt tiêu Data Race không cần Mutex.
6. **Zero Undefined Behavior (UB) & CHERI-Ready:**
   - `+`, `-`, `*` checked overflow. `+%` wrapping, `+!` saturating.
   - Refinement Types chứng minh biên mảng lúc biên dịch để loại bỏ bounds check thừa lúc chạy.

---

## PHẦN II: TECH STACK & WORKSPACE
- Host: Rust (Edition 2021+)
- Memory: Index-Based Flat Arena (`bumpalo`), no `Box<Expr>` or recursive pointer graphs.
- Lexer: `logos`, String Interner: `lasso`.
- Concurrency & Parsing: Pratt Parser + Recursive Descent, `rayon`.
