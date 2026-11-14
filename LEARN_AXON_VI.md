# Giáo Trình Ngôn Ngữ Lập Trình Axon v5.0 (Từ Cơ Bản Đến Làm Chủ Hệ Thống)

Tài liệu này là hướng dẫn chính thức của **Axon v5.0 (`.ax`)**: ngôn ngữ lập trình hệ thống tự thân (self-hosted) được thiết kế để kết hợp **cú pháp tự nhiên, dễ học của Python/Go** với **tốc độ mã máy trần vượt C++/Rust**, **kích thước tệp thực thi độc lập chỉ vài KB**, và **an toàn bộ nhớ + bảo mật chống dịch ngược ngay từ trình biên dịch**.

Mọi đoạn mã trong giáo trình này đều tương thích trực tiếp với trình biên dịch thực tế `bin/axc.exe` và `bin/axc0.exe` trong kho mã nguồn.

---

## Mục Lục

1. [Bắt đầu với Axon v5.0 trong 2 phút](#chương-1-bắt-đầu-với-axon-v50-trong-2-phút)
2. [Biến, Kiểu Dữ Liệu & Cú Pháp Phong Cách Python](#chương-2-biến-kiểu-dữ-liệu--cú-pháp-phong-cách-python)
3. [Hàm, Tham Số Đặt Tên, Mặc Định & Đường Ống `|>`](#chương-3-hàm-tham-số-đặt-tên-mặc-định--đường-ống-)
4. [Luồng Điều Khiển & Tinh Chỉnh Biên Mảng `where`](#chương-4-luồng-điều-khiển--tinh-chỉnh-biên-mảng-where)
5. [Xử Lý Lỗi Không Chi Phí `T ! Error`](#chương-5-xử-lý-lỗi-không-chi-phí-t--error)
6. [Struct, Phương Thức `impl` & Giao Diện Duck-Typing `interface`](#chương-6-struct-phương-thức-impl--giao-diện-duck-typing-interface)
7. [Số Thập Phân Tài Chính Chính Xác `dec`](#chương-7-số-thập-phân-tài-chính-chính-xác-dec)
8. [Quản Lý Bộ Nhớ Tĩnh Perceus & Tái Sử Dụng Tại Chỗ FBIP](#chương-8-quản-lý-bộ-nhớ-tĩnh-perceus--tái-sử-dụng-tại-chỗ-fbip)
9. [Bảng Cột `Table[T]` SoA & Tự Động Vector Hóa AVX2 FMA3](#chương-9-bảng-cột-tablet-soa--tự-động-vector-hóa-avx2-fma3)
10. [Đa Luồng An Toàn `spawn` & Cấu Trúc Phân Tán `CRDT`](#chương-10-đa-luồng-an-toàn-spawn--cấu-trúc-phân-tán-crdt)
11. [Bảo Mật Năng Lực, Chống Crack & Nhị Phân Sạch Antivirus](#chương-11-bảo-mật-năng-lực-chống-crack--nhị-phân-sạch-antivirus)
12. [Kiến Trúc Self-Hosting, Hàm Nội Tại & Bảng Mã Lỗi](#chương-12-kiến-trúc-self-hosting-hàm-nội-tại--bảng-mã-lỗi)

---

## Chương 1: Bắt đầu với Axon v5.0 trong 2 phút

### 1.1. Axon giải quyết bài toán gì?

Các ngôn ngữ lập trình truyền thống buộc người học phải đánh đổi:
- **Python / JavaScript:** Dễ viết nhưng chậm hàng trăm lần trên vòng lặp tính toán và cần máy ảo nặng hàng trăm MB.
- **Java / Go:** Phải gánh bộ thu gom rác (GC) gây giật độ trễ, tệp thực thi nặng từ vài MB đến hàng trăm MB JRE.
- **C / C++:** Nhanh và nhẹ nhưng đầy lỗ hổng tràn bộ đệm, con trỏ treo (Use-After-Free) và tràn số âm thầm.
- **Rust:** An toàn nhưng rào cản học tập cao do ký hiệu vòng đời `'a`, borrow checker phức tạp và thời gian biên dịch lâu.

**Axon v5.0** xóa bỏ sự đánh đổi đó bằng 4 trụ cột kỹ thuật:
1. **Cú pháp tối giản:** Không bắt buộc dấu chấm phẩy `;`, dùng `and` / `or` / `not`, nội suy chuỗi `f"..."`, tham số đặt tên.
2. **Perceus tĩnh (0 GC, 0 `'a` lifetime):** Tự động chèn lệnh giải phóng bộ nhớ xác định và tái sử dụng vùng nhớ ngay tại chỗ (FBIP).
3. **`Table[T]` theo cột (SoA) + AVX2 FMA3 tự động:** Tối ưu băng thông RAM và phát lệnh SIMD 256-bit trực tiếp từ vòng lặp `for` thông thường.
4. **Trình biên dịch độc lập ~547 KB:** Tự sinh mã máy `x86_64` và đóng gói thẳng tệp `.exe` chuẩn OS (~5.5 KB cho ứng dụng cơ bản) trong vài mili-giây.

### 1.2. Cài đặt & chạy chương trình đầu tiên

Trên Windows PowerShell:

```powershell
# Cài đặt axc.exe vào ~/.axon/bin và thêm vào PATH
.\install.ps1

# Kiểm tra phiên bản
.\bin\axc.exe version
```

Tạo tệp `examples/hello.ax`:

```axon
fn greet(name: str, role: str = "Systems Engineer") {
    println(f"Hello, {name}! Welcome to Axon v5.0 ({role}).")
}

@shield
fn main() -> i32 {
    greet("World")
    greet("Khang", role = "Language Architect")
    return 0
}
```

Kiểm tra kiểu và biên dịch ra tệp thực thi độc lập:

```powershell
# Chỉ kiểm tra an toàn kiểu, bộ nhớ và năng lực (~1 ms)
.\bin\axc.exe check examples/hello.ax

# Biên dịch thẳng ra tệp thực thi gốc Windows PE32+ (~5.5 KB)
.\bin\axc.exe build examples/hello.ax -o bin/hello.exe
.\bin\hello.exe
```

---

## Chương 2: Biến, Kiểu Dữ Liệu & Cú Pháp Phong Cách Python

### 2.1. Bất biến mặc định (`:=`) và Khả biến tường minh (`mut`)

Trong Axon, biến khai báo bằng `:=` (hoặc `let`) là **bất biến mặc định** để ngăn lỗi ghi đè ngoài ý muốn. Khi cần thay đổi giá trị, thêm từ khóa `mut`:

```axon
fn main() -> i32 {
    limit := 100              // Bất biến (suy luận kiểu i64)
    let ratio: f64 = 3.5      // Khai báo kiểu tường minh
    mut counter := 0          // Khả biến

    counter += 10
    println(f"limit={limit}, ratio={ratio}, counter={counter}")
    return 0
}
```

Nếu cố tình gán lại một biến không có `mut`, trình biên dịch chặn ngay lập tức với lỗi **`[SAFETY ERROR E0301]`**.

### 2.2. Các kiểu dữ liệu nguyên thủy

| Kiểu dữ liệu | Kích thước / Biểu diễn | Ví dụ hằng số | Ghi chú |
| :--- | :--- | :--- | :--- |
| `i64`, `int`, `usize` | Số nguyên 64-bit có dấu | `42`, `0xFF00` | Kiểu nguyên mặc định trên thanh ghi 64-bit |
| `i32` | Số nguyên 32-bit | `0` | Thường dùng cho mã thoát `fn main() -> i32` |
| `u8`, `byte` | Byte 8-bit | `65` | Dùng khi xử lý nhị phân |
| `f64`, `float` | Số thực dấu phẩy động 64-bit | `3.14159` | Chuẩn IEEE-754 double precision |
| `f32` | Số thực 32-bit | `1.5` | Tối ưu cho cột SIMD `Table[T]` (8 số / thanh ghi YMM) |
| `dec` | Số thập phân cố định 64-bit | `49.50d`, `0.08d` | Chính xác 4 chữ số thập phân (hệ số `10_000`), 0 sai số làm tròn |
| `bool` | Logic đúng / sai | `true`, `false` | Hỗ trợ `and`, `or`, `not` đoản mạch |
| `str` | Chuỗi UTF-8 slice (`ptr + len`) | `"Axon"`, `f"x={x}"` | Truy cập độ dài O(1), không kết thúc bằng NUL mù |

### 2.3. Logic đoản mạch (`and`, `or`, `not`) và Toán tử số học an toàn

- **Đoản mạch (Short-circuit):** Trong `a and b`, nếu `a` sai thì `b` không bao giờ được thực thi. Trong `a or b`, nếu `a` đúng thì `b` được bỏ qua. Điều này giúp viết biểu thức kiểm tra biên `if i < n and str_byte(s, i) == 65` an toàn tuyệt đối.
- **Chống tràn số tự động:** Trong hàm có bật `@shield`, các phép `+`, `-`, `*` kiểm tra cờ tràn phần cứng CPU (`JO`) và phép `/`, `%` luôn chặn chia cho `0` cũng như tràn `INT64_MIN / -1`.
- **Toán tử tràn có chủ đích (Wrapping):** Khi viết thuật toán băm (như FNV-1a hay mật mã) cần tràn bit 64-bit chuẩn xác, sử dụng `+%`, `-%`, `*%`:

```axon
mut hash := 0xcbf29ce484222325
hash = (hash ^ byte_val) *% 0x100000001b3
```

---

## Chương 3: Hàm, Tham Số Đặt Tên, Mặc Định & Đường Ống `|>`

### 3.1. Tham số mặc định và Gọi hàm theo tên (Named Arguments)

Bạn không cần tạo hàng loạt hàm nạp chồng (overloading) hay Builder pattern phức tạp như trong Java/Rust:

```axon
fn compute_score(base: i64, bonus: i64 = 10, multiplier: i64 = 2) -> i64 {
    return (base + bonus) * multiplier
}

fn main() -> i32 {
    s1 := compute_score(50)                               // (50 + 10) * 2 = 120
    s2 := compute_score(50, multiplier = 5)               // (50 + 10) * 5 = 300
    s3 := compute_score(bonus = 20, base = 30)            // (30 + 20) * 2 = 100
    println(f"s1={s1}, s2={s2}, s3={s3}")
    return 0
}
```

### 3.2. Tham số cập nhật tại chỗ (`mut`) và Trả về nhiều giá trị (Tuple)

- Khai báo tham số `mut x: i64` (gọi bằng `bump(mut val)`) cho phép hàm cập nhật trực tiếp biến của hàm gọi mà không cần con trỏ thô `*int`.
- Hàm có thể trả về một bộ Tuple nhiều giá trị và giải nén trực tiếp:

```axon
fn div_rem(a: i64, b: i64) -> (i64, i64) {
    return (a / b, a % b)
}

fn bump_twice(mut v: i64, step: i64) {
    v += step * 2
}

fn main() -> i32 {
    (q, r) := div_rem(29, 5)
    mut total := q
    bump_twice(mut total, r)
    println(f"q={q}, r={r}, total={total}")
    return 0
}
```

### 3.3. Toán tử đường ống `|>`

Toán tử `x |> f(a, b)` tự động truyền `x` làm đối số đầu tiên `f(x, a, b)`, giúp chuỗi biến đổi dữ liệu đọc xuôi từ trái sang phải:

```axon
fn add_offset(x: i64, off: i64) -> i64 { return x + off }
fn scale_up(x: i64, k: i64) -> i64 { return x * k }

fn main() -> i32 {
    result := 10 |> add_offset(5) |> scale_up(4)   // scale_up(add_offset(10, 5), 4) = 60
    println(f"piped={result}")
    return 0
}
```

---

## Chương 4: Luồng Điều Khiển & Tinh Chỉnh Biên Mảng `where`

### 4.1. `if / else if / else`, `while` và `for .. in`

Cú pháp điều kiện và vòng lặp gọn gàng, không cần dấu ngoặc tròn `(...)` bao quanh điều kiện:

```axon
fn main() -> i32 {
    mut sum := 0
    for i in 0..10 {
        if i % 2 == 0 {
            sum += i
        }
    }

    mut n := 5
    while n > 0 {
        n -= 1
    }
    println(f"even_sum={sum}")
    return 0
}
```

### 4.2. Loại bỏ kiểm tra biên tĩnh bằng mệnh đề tinh chỉnh `where`

Mọi truy cập chỉ số `table[idx]` trong Axon mặc định đều được bảo vệ chống tràn bộ đệm. Khi hiệu năng cực hạn cần loại bỏ cả lệnh `CMP + JAE` trong vòng lặp nóng, bạn có thể chứng minh biên ngay trên chữ ký hàm bằng `where idx < arr.len` hoặc duyệt qua `for idx in 0..arr.len`:

```axon
fn read_checked(arr: Table[i64], idx: usize where idx < arr.len) -> i64 {
    return arr[idx]   // Đã chứng minh tĩnh hợp lệ -> 0 chu kỳ kiểm tra biên thừa
}
```

---

## Chương 5: Xử Lý Lỗi Không Chi Phí `T ! Error`

Axon thay thế cơ chế `try / catch` nặng nề (cần bảng stack unwinding runtime) bằng kiểu hợp lỗi **`PayloadType ! ErrorType`** nằm gọn trên các thanh ghi/slot ngăn xếp:

- Trả về thành công bằng `ok(gia_tri)`.
- Trả về lỗi bằng `err(ma_loi)`.
- Xử lý tại nơi gọi bằng 3 cách cực ngắn gọn:
  1. **`expr or gia_tri_du_phong`**: Nhận giá trị dự phòng nếu hàm trả về lỗi.
  2. **`expr or return gia_tri_thoat`**: Thoát sớm khỏi hàm hiện tại nếu gặp lỗi.
  3. **`expr?`**: Tự động chuyển tiếp lỗi lên hàm gọi (khi hàm hiện tại cũng trả về `T ! E`).

```axon
fn parse_port(raw: i64) -> i64 ! i64 {
    if raw <= 0 or raw > 65535 {
        return err(400)
    }
    return ok(raw)
}

fn open_service(raw_port: i64) -> i64 ! i64 {
    port := parse_port(raw_port)?     // Tự động trả về err(400) nếu không hợp lệ
    return ok(port + 1000)
}

fn main() -> i32 {
    p_valid := parse_port(8080) or 80
    p_fallback := parse_port(-5) or 3000
    svc := open_service(443) or 0
    println(f"valid={p_valid}, fallback={p_fallback}, svc={svc}")
    return 0
}
```

---

## Chương 6: Struct, Phương Thức `impl` & Giao Diện Duck-Typing `interface`

### 6.1. Struct giá trị phẳng và `impl`

`struct` trong Axon được bố trí phẳng liên tiếp trên ngăn xếp (Stack), không tốn con trỏ đối tượng hay header 16-byte như Java/Python:

```axon
struct CloudPlan {
    monthly: dec
    nodes: i64
}

impl CloudPlan {
    fn subtotal(self) -> dec {
        return self.monthly * self.nodes
    }
}
```

Bạn có thể khởi tạo `struct` theo thứ tự vị trí `CloudPlan(49.50d, 4)` hoặc đặt tên trường rõ ràng `CloudPlan(monthly = 49.50d, nodes = 4)`.

### 6.2. Giao diện Duck-Typing đa hình trên Stack (`interface`)

Giống Go, một `struct` tự động thỏa mãn `interface` ngay khi có đủ phương thức tương ứng mà không cần khai báo `implements`. Khác với Go/Java phải cấp phát interface lên Heap (gây áp lực GC), Axon đóng gói thẻ định danh kiểu + dữ liệu trực tiếp trên Stack:

```axon
use std::decimal

interface Billable {
    fn total_cost(self) -> dec
}

struct DedicatedNode {
    unit_price: dec
    count: i64
}

impl DedicatedNode {
    fn total_cost(self) -> dec {
        return apply_tax(self.unit_price * self.count, rate_pct = 0.10d)
    }
}

fn print_invoice(item: Billable) {
    println(f"Invoice total: ${item.total_cost()}")
}

fn main() -> i32 {
    srv := DedicatedNode(unit_price = 100.00d, count = 2)
    print_invoice(srv)   // $220.00, 0 byte cấp phát Heap
    return 0
}
```

---

## Chương 7: Số Thập Phân Tài Chính Chính Xác `dec`

Trong IEEE-754 `f64`, phép tính `0.1 + 0.2` cho kết quả `0.30000000000000004`, gây sai lệch nghiêm trọng cho ngân hàng, kế toán, sàn giao dịch và thương mại điện tử.

Axon tích hợp sẵn kiểu nguyên thủy **`dec`** (hậu tố `d`, độ chính xác 4 chữ số thập phân = hệ số tỷ lệ `10_000`) chạy trực tiếp trên thanh ghi số nguyên 64-bit của CPU kèm bảo vệ chống tràn phần cứng:

```axon
use std::decimal

fn main() -> i32 {
    a := 0.10d
    b := 0.20d
    exact_sum := a + b                                // Chính xác tuyệt đối 0.30d

    total := apply_tax(199.99d, rate_pct = 0.08d)     // Thuế 8%
    (share, remainder) := split_exact_cents(100.00d, 3)

    println(f"0.10d + 0.20d = {exact_sum}")
    println(f"Taxed = {total}, 3-way split = {share} (rem = {remainder})")
    return 0
}
```

---

## Chương 8: Quản Lý Bộ Nhớ Tĩnh Perceus & Tái Sử Dụng Tại Chỗ FBIP

Axon đạt an toàn bộ nhớ tuyệt đối mà **không cần Garbage Collector** và **không bắt người học viết `'a` lifetime** nhờ 3 cơ chế kết hợp:

### 8.1. Tham chiếu hạng hai (Second-Class References)
Trình biên dịch cấm cất giữ tham chiếu bên trong `struct` (**`E0101`**) và cấm trả về tham chiếu trỏ tới biến cục bộ của hàm (**`E0102`**). Nhờ đó không bao giờ tồn tại lỗi con trỏ treo (Use-After-Free) và không cần chú thích vòng đời `'a`.

### 8.2. Chuyển quyền sở hữu tuyến tính (`move`) & Giải phóng xác định
Khi bạn truyền tài nguyên vùng nhớ (`ByteBuf`, `IntBuf`, `Table[T]`) với từ khóa `move`, quyền sở hữu được chuyển hẳn. Mọi truy cập lại biến cũ sau khi `move` đều bị chặn ngay lúc biên dịch với mã lỗi **`E0201`**. Với các vùng nhớ không `move` ra ngoài, trình biên dịch tự động sinh lệnh trả bộ nhớ về hệ điều hành (`VirtualFree`) tại mọi nhánh thoát của hàm.

### 8.3. Tái sử dụng bộ nhớ tại chỗ FBIP (Functional-But-In-Place)
Khi một vùng đệm cũ kết thúc vòng đời bằng `buf_free(move a)` ngay trước khi cấp phát vùng đệm mới `b := ibuf_new(...)` có dung lượng tương đương, bộ lập lịch Perceus tự động tái sử dụng ngay địa chỉ RAM của `a` cho `b` với **0 lời gọi hệ thống cấp phát mới**:

```axon
fn process_batch() -> i64 {
    buf_a := ibuf_new(256)
    ibuf_push(buf_a, 40)
    ibuf_push(buf_a, 60)
    partial := ibuf_get(buf_a, 0) + ibuf_get(buf_a, 1)
    buf_free(move buf_a)

    // Perceus FBIP tái sử dụng trực tiếp khối RAM của buf_a (0 syscall VirtualAlloc)
    buf_b := ibuf_new(256)
    ibuf_push(buf_b, partial + 45)
    ans := ibuf_get(buf_b, 0)
    buf_free(move buf_b)
    return ans
}

fn main() -> i32 {
    println(f"FBIP result = {process_batch()}")
    return 0
}
```

---

## Chương 9: Bảng Cột `Table[T]` SoA & Tự Động Vector Hóa AVX2 FMA3

Đây là vũ khí giúp code Axon viết ngắn gọn như Python nhưng chạy **nhanh gấp hơn 6 lần C++/Rust/Go** và **hơn 210 lần Python** trên các bài toán dữ liệu lớn, game engine, mô phỏng vật lý và AI.

### 9.1. Tại sao mảng truyền thống (AoS) trong C++/Rust/Go bị nghẽn RAM?
Khi bạn khai báo `Entity` gồm 16 trường `f32` (64 byte/phần tử) trong `std::vector<Entity>` (C++) hoặc `Vec<Entity>` (Rust), các trường nằm xen kẽ nhau (`px, vx, py, vy, pz, ...`). Khi vòng lặp chỉ cập nhật `px` và `vx`, CPU vẫn buộc phải kéo toàn bộ 64 byte của mỗi phần tử vào bộ nhớ đệm L1, lãng phí 75% băng thông RAM.

### 9.2. Cách `Table[T]` của Axon vận hành
Khi bạn khởi tạo `Table[Entity]::with_capacity(n)`:
1. Trình biên dịch tự động tách từng trường thành các mảng cột liên tiếp trong bộ nhớ (**Structure-of-Arrays - SoA**): toàn bộ `px[0..n]` nằm sát cạnh nhau, toàn bộ `vx[0..n]` nằm sát cạnh nhau.
2. Vòng lặp `for e in entities { e.px += e.vx * dt }` chỉ chạm đúng các cột đang dùng và tự động hạ xuống vòng lặp phần cứng **4x-Unrolled 256-bit AVX2 FMA3 (`VFMADD231PS`)**, tính đồng thời **32 phần tử `f32` (128 byte)** trên mỗi nhịp lặp!

```axon
struct Particle {
    px: f32
    vx: f32
    py: f32
    vy: f32
    mass: f32
    heat: f32
}

@shield
fn main() -> i32 {
    n := 4096
    particles := Table[Particle]::with_capacity(n)
    table_fill_f32(particles, 0, 1.0)    // cột 0 (px) = 1.0
    table_fill_f32(particles, 1, 3.5)    // cột 1 (vx) = 3.5

    dt := 2.0
    for p in particles {
        p.px += p.vx * dt                // Tự động sinh lệnh AVX2 VFMADD231PS
    }

    sample := table_get_f32(particles, 0, n - 1)
    println(f"Updated {n} particles via AVX2 SoA: px[{n - 1}] = {sample}")
    return 0
}
```

Bạn có thể tự kiểm chứng tốc độ thực tế trên 1,048,576 thực thể $\times$ 200 bước bằng tệp [`examples/bench_soa_simd.ax`](examples/bench_soa_simd.ax):

```powershell
.\bin\axc.exe build examples/bench_soa_simd.ax -o bin/bench_soa_simd.exe
.\bin\bench_soa_simd.exe
```

---

## Chương 10: Đa Luồng An Toàn `spawn` & Cấu Trúc Phân Tán `CRDT`

### 10.1. Tác vụ `spawn` và Kiểm tra cô lập vùng nhớ (`E0202`)
Axon ngăn chặn hoàn toàn lỗi tranh chấp dữ liệu đa luồng (Data Race) ngay lúc biên dịch: một khối `spawn { ... }` không được phép truy cập biến vùng nhớ độc lập (`isolated`) bên ngoài nếu chưa chuyển giao quyền sở hữu tường minh bằng `move` (**`E0202`**):

```axon
fn compute_chunk(isolated data: i64) -> i64 {
    task := spawn {
        moved_val := move data
        return moved_val * 2
    }
    return task.join()
}

fn main() -> i32 {
    println(f"Joined result = {compute_chunk(67)}")
    return 0
}
```

### 10.2. Cấu trúc dữ liệu phân tán tự hội tụ `std::crdt`
Khi đồng bộ trạng thái giữa nhiều nút mạng mà không muốn dùng khóa (mutex/lock), thư viện `std::crdt` cung cấp các cấu trúc bán dàn toán học (`GCounter4`, `LWWReg`) đảm bảo tính giao hoán, kết hợp và lũy đẳng (`merge(a, a) == a`):

```axon
use std::crdt

fn main() -> i32 {
    node_a := gcounter_inc(gcounter_new(), 0, 15)
    node_b := gcounter_inc(gcounter_new(), 1, 27)

    // Gộp theo bất kỳ thứ tự nào hoặc lặp lại nhiều lần đều hội tụ chính xác về 42
    merged := gcounter_merge(node_a, gcounter_merge(node_b, node_a))
    println(f"Converged cluster counter = {gcounter_value(merged)}")
    return 0
}
```

---

## Chương 11: Bảo Mật Năng Lực, Chống Crack & Nhị Phân Sạch Antivirus

Axon là ngôn ngữ hiếm hoi tích hợp sẵn cả bảo mật chống tấn công chuỗi cung ứng lẫn bảo vệ bản quyền phần mềm theo đúng chuẩn hệ điều hành:

### 11.1. Bảo mật cấp năng lực (Object-Capability Security - `std::cap`)
Các hàm I/O đặc quyền thô bị cấm gọi tự do nếu hàm không nắm giữ thẻ năng lực hợp lệ (**`E0501`**). Từ `RootCap` ban đầu ở `main`, bạn thu hẹp quyền xuống thư mục cụ thể và chế độ chỉ-đọc bằng `attenuate_fs`, đồng thời chặn đứng tấn công leo thang thư mục `../../`:

```axon
use std::cap

fn main() -> i32 {
    root := acquire_root_cap()
    ro_cap := attenuate_fs(root, base_dir = "examples", read_only = true)

    blocked := not is_safe_subpath("../../Windows/System32/cmd.exe")
    write_attempt := cap_write_file(ro_cap, "test.txt", buf_new(16)) or -403
    println(f"Path traversal blocked={blocked}, write on read-only={write_attempt}")
    return 0
}
```

### 11.2. Ba lớp chống dịch ngược & đánh cắp khóa (`@shield`, `@secret`, `@whitebox`)
1. **`@shield`:** Tự động bật kiểm tra tràn số phần cứng, chèn `ENDBR64` (Intel CET chống tấn công ROP/JOP), và tính mã băm FNV-1a 64-bit kiểm tra toàn vẹn vùng mã máy `.text` ngay khi tệp `.exe` khởi chạy. Nếu tệp thực thi bị sửa dù chỉ 1 byte mã máy (patching/cracking), chương trình khóa thực thi ngay lập tức.
2. **`@secret("...")`:** Mã hóa chuỗi nhạy cảm (PIN, khóa API) ngay lúc biên dịch; chỉ giải mã tạm thời vào khung ngăn xếp lúc hàm chạy và tự động ghi đè `0` xóa sạch ngăn xếp trước khi hàm `return` (kiểm tra quét chuỗi plaintext trong tệp `.exe` cho kết quả **0 rò rỉ**).
3. **`@whitebox` & `whitebox_sbox` (`std::crypto`):** Hòa tan khóa bí mật vào bảng tra cứu song ánh trên trường hữu hạn $\text{GF}(2^8)$ (đa thức tối giản `0x11B`), không để lộ khóa thô trong `.rdata`.

```axon
use std::crypto

@shield
fn main() -> i32 {
    api_pin := @secret("OMEGA-VAULT-9042")
    license_mac := whitebox_mac_str("AXON-PRO-LICENSE-2026")
    println(f"Secret length={api_pin.len}, White-Box MAC={license_mac}")
    return 0
}
```

### 11.3. Chuẩn nhị phân sạch Antivirus (AV-Clean Binary Law)
Nhiều công cụ đóng gói chống crack bị Windows Defender / EDR nhận nhầm là mã độc vì tạo phân vùng vừa ghi vừa thực thi (`RWX`), thiếu bảng tháo gỡ ngăn xếp hoặc nén làm độ hỗn loạn (Shannon entropy) vọt lên `> 7.2`. Trình liên kết PE32+ của Axon tuân thủ tuyệt đối chuẩn trình biên dịch hệ thống:
- Tách biệt nghiêm ngặt `W^X`: `.text` chỉ đọc-thực thi (`RX`), `.rdata` và `.pdata` chỉ đọc (`R`), **0 phân vùng `RWX`**.
- Phát sinh đầy đủ bảng `.pdata` và `.xdata` SEH `UNWIND_INFO` cho mọi hàm.
- Bật High-Entropy 64-bit ASLR + Hardware DEP/NX (`DllCharacteristics = 0x8160`).
- Giữ Shannon entropy vùng `.text` ở mức tự nhiên **`~5.61 < 6.50`**.

---

## Chương 12: Kiến Trúc Self-Hosting, Hàm Nội Tại & Bảng Mã Lỗi

### 12.1. Quy trình tự biên dịch 3 vòng xác định (3-Stage Bootstrap)
Trình biên dịch chính thức [`bin/axc.exe`](bin/axc.exe) được viết hoàn toàn bằng ngôn ngữ Axon trong thư mục [`compiler_ax/`](compiler_ax/):
- **Vòng 1:** `axc0.exe` (Stage-0) biên dịch `compiler_ax/main.ax` $\rightarrow$ `axc_stage1.exe`.
- **Vòng 2:** `axc_stage1.exe` tự biên dịch chính mã nguồn `compiler_ax/main.ax` $\rightarrow$ `axc_stage2.exe`.
- **Vòng 3:** `axc_stage2.exe` biên dịch lại `compiler_ax/main.ax` $\rightarrow$ `axc_stage3.exe`.
- **Chứng minh hội tụ:** `SHA256(axc_stage2.exe) == SHA256(axc_stage3.exe)` trùng khớp tuyệt đối từng bit trong ~114 ms.

### 12.2. Bảng tra cứu hàm nội tại (Built-in Intrinsics)

| Nhóm | Hàm nội tại | Công dụng |
| :--- | :--- | :--- |
| **Xuất & Thời gian** | `print(...)`, `println(...)`, `clock_ns()` | In chuỗi / `f"..."` ra stdout (0 CRT), đo thời gian độ phân giải nano-giây |
| **Bộ đệm Byte (`ByteBuf`)** | `buf_new(cap)`, `buf_len(b)`, `buf_get(b, i)`, `buf_set(b, i, v)`, `buf_push(b, v)`, `buf_push_u16/u32/u64`, `buf_put_u32/u64`, `buf_push_str`, `buf_slice_str`, `buf_free(move b)` | Quản lý vùng nhớ nhị phân tốc độ cao dùng cho trình biên dịch, giao thức mạng và tệp tin |
| **Bộ đệm Nguyên (`IntBuf`)** | `ibuf_new(cap)`, `ibuf_len(b)`, `ibuf_get(b, i)`, `ibuf_set(b, i, v)`, `ibuf_push(b, v)`, `buf_free(move b)` | Mảng động số nguyên 64-bit có kiểm tra biên phần cứng |
| **Chuỗi & Tham số CLI** | `str_len(s)`, `str_byte(s, i)`, `str_slice(s, a, b)`, `str_eq(a, b)`, `args_count()`, `args_get(i)` | Xử lý chuỗi UTF-8 O(1) và đọc đối số dòng lệnh |
| **Tệp tin (`T ! i64`)** | `fs_read_file(path)`, `fs_write_file(path, buf)` | Đọc/ghi tệp nhị phân trực tiếp qua Win64 API trả về kiểu hợp lỗi `T ! i64` |
| **Bảng SoA & Bảo mật** | `Table[T]::with_capacity(n)`, `table_fill_f32(t, col, v)`, `table_get_f32(t, col, i)`, `whitebox_sbox(seed, byte)` | Thao tác cột SoA SIMD và bảng S-Box mật mã hộp trắng $\text{GF}(2^8)$ |

### 12.3. Bảng tra cứu mã lỗi an toàn lúc biên dịch

| Mã lỗi | Ý nghĩa vi phạm | Cách khắc phục chuẩn trong Axon |
| :--- | :--- | :--- |
| **`E0101`** | Lưu trữ kiểu tham chiếu (`&T`) bên trong `struct` | Lưu giá trị trực tiếp hoặc chỉ số `i64` vào `Table[T]` |
| **`E0102`** | Hàm trả về tham chiếu (`-> &T`) gây nguy cơ con trỏ treo | Trả về giá trị trực tiếp trên thanh ghi/ngăn xếp hoặc `T ! Error` |
| **`E0201`** | Sử dụng lại biến sau khi đã chuyển quyền sở hữu (`move`) | Chỉ dùng biến trước khi `move` hoặc truyền mượn tạm thời không có `move` |
| **`E0202`** | Bắt giữ biến `isolated` vào `spawn` mà không chuyển giao bằng `move` | Chuyển quyền sở hữu rõ ràng bằng `move ten_bien` bên trong khối `spawn` |
| **`E0301`** | Cố tình gán hoặc sửa đổi biến/tham số bất biến (`:=` / `let`) | Khai báo biến với `mut x := ...` hoặc tham số `mut x: Type` |
| **`E0401`** | Kiểu giá trị khởi tạo không khớp với kiểu khai báo của biến | Đảm bảo kiểu hai vế tương thích hoặc dùng ép kiểu tường minh `as Type` |
| **`E0402`** | Kiểu biểu thức `return` không khớp với chữ ký trả về của hàm | Trả về đúng kiểu đã khai báo hoặc bọc trong `ok(...)` / `err(...)` cho `T ! E` |
| **`E0501`** | Gọi hàm hệ thống đặc quyền mà không nắm giữ thẻ năng lực (`RootCap`/`FsCap`/`NetCap`) | Truyền thẻ năng lực đã thu hẹp quyền từ `std::cap` vào hàm |
