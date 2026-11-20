import time
import json

# --- W1: Matrix GEMM 256x256 FP32 ---
def bench_gemm():
    n = 256
    a = [0.5] * (n * n)
    b = [0.5] * (n * n)
    c = [0.0] * (n * n)

    t0 = time.perf_counter()
    for i in range(n):
        for k in range(n):
            aik = a[i * n + k]
            for j in range(n):
                c[i * n + j] += aik * b[k * n + j]
    elapsed = (time.perf_counter() - t0) * 1000.0
    if c[0] < 0:
        print("checksum:", c[0])
    return elapsed

# --- W2: BNN 1-Bit POPCOUNT GEMV (4096 rows x 4096 cols = 64 words) ---
def bench_bnn():
    rows = 4096
    words = 64
    weights = [0x5555555555555555] * (rows * words)
    act = [0xAAAAAAAAAAAAAAAA] * words
    out = [0] * rows

    t0 = time.perf_counter()
    for r in range(rows):
        s = 0
        base = r * words
        for w in range(words):
            x = weights[base + w] ^ act[w]
            s += x.bit_count()
        out[r] = s
    elapsed = (time.perf_counter() - t0) * 1000.0
    if out[0] < 0:
        print("checksum:", out[0])
    return elapsed

# --- W3: Quicksort (100,000 int64) ---
def quicksort_i64(arr, low, high):
    stack = [(low, high)]
    while stack:
        l, h = stack.pop()
        if l >= h:
            continue
        pivot = arr[h]
        i = l - 1
        for j in range(l, h):
            if arr[j] <= pivot:
                i += 1
                arr[i], arr[j] = arr[j], arr[i]
        arr[i + 1], arr[h] = arr[h], arr[i + 1]
        pi = i + 1
        stack.append((l, pi - 1))
        stack.append((pi + 1, h))

def bench_sort():
    n = 100000
    arr = [0] * n
    seed = 42
    for i in range(n):
        seed = (seed * 1103515245 + 12345) & 0x7FFFFFFF
        arr[i] = seed % 1000000

    t0 = time.perf_counter()
    quicksort_i64(arr, 0, n - 1)
    elapsed = (time.perf_counter() - t0) * 1000.0
    if arr[0] < 0:
        print("checksum:", arr[0])
    return elapsed

# --- W4: Sieve of Eratosthenes (2,000,000) ---
def bench_sieve():
    n = 2000000
    is_prime = bytearray(b'\x01') * (n + 1)
    is_prime[0] = 0
    is_prime[1] = 0

    t0 = time.perf_counter()
    p = 2
    while p * p <= n:
        if is_prime[p]:
            for i in range(p * p, n + 1, p):
                is_prime[i] = 0
        p += 1

    count = 0
    for i in range(2, n + 1):
        if is_prime[i]:
            count += 1
    elapsed = (time.perf_counter() - t0) * 1000.0
    if count == 0:
        print("checksum:", count)
    return elapsed

# --- W5: 3D Particle Kinematics (50,000 particles x 10 steps) ---
def bench_physics():
    n = 50000
    x = [0.0] * n
    y = [100.0] * n
    z = [0.0] * n
    vx = [1.0] * n
    vy = [0.0] * n
    vz = [0.5] * n

    dt = 0.016
    g = -9.81

    t0 = time.perf_counter()
    for _ in range(10):
        for i in range(n):
            x[i] += vx[i] * dt
            y[i] += vy[i] * dt
            z[i] += vz[i] * dt
            vy[i] += g * dt
    elapsed = (time.perf_counter() - t0) * 1000.0
    if y[0] < -9999.0:
        print("checksum:", y[0])
    return elapsed

# --- W6: JSON Scanner (100,000 runs) ---
def bench_json():
    doc = b"{\"order_id\": 9876543, \"user_id\": 42109, \"symbol\": \"BTCUSD\", \"qty\": 1500, \"price\": 96500, \"status\": \"FILLED\"}"
    doc_len = len(doc)
    key = b"\"price\": "
    key_len = len(key)

    t0 = time.perf_counter()
    sum_price = 0
    for _ in range(100000):
        for i in range(doc_len - key_len + 1):
            if doc[i] == 34 and doc[i + 1] == 112: # '"p'
                if doc[i:i + key_len] == key:
                    val = 0
                    k = i + key_len
                    while k < doc_len and 48 <= doc[k] <= 57:
                        val = val * 10 + (doc[k] - 48)
                        k += 1
                    sum_price += val
                    break
    elapsed = (time.perf_counter() - t0) * 1000.0
    if sum_price == 0:
        print("checksum:", sum_price)
    return elapsed

def fib_py(n):
    return n if n <= 1 else fib_py(n - 1) + fib_py(n - 2)

def bench_fib():
    t0 = time.perf_counter()
    r = fib_py(35)
    el = (time.perf_counter() - t0) * 1000.0
    if r != 9227465: print("checksum:", r)
    return el

def bench_crc32():
    n = 16777216
    buf = bytes([(i * 7 + 13) & 255 for i in range(n)])
    tbl = []
    for i in range(256):
        c = i
        for _ in range(8): c = ((c >> 1) ^ 0xEDB88320) if (c & 1) else (c >> 1)
        tbl.append(c)
    t0 = time.perf_counter()
    crc = 0xFFFFFFFF
    for b in buf:
        crc = (crc >> 8) ^ tbl[(crc ^ b) & 255]
    crc ^= 0xFFFFFFFF
    el = (time.perf_counter() - t0) * 1000.0
    if crc == 0: print("checksum:", crc)
    return el

def bench_mandel():
    size = 512
    t0 = time.perf_counter()
    total = 0
    for py in range(size):
        ci = (py / 512.0) * 2.4 - 1.2
        for px in range(size):
            cr = (px / 512.0) * 3.0 - 2.0
            zr = 0.0; zi = 0.0; it = 0
            while it < 64:
                zr2 = zr * zr; zi2 = zi * zi
                if zr2 + zi2 > 4.0: break
                zi = 2.0 * zr * zi + ci
                zr = zr2 - zi2 + cr
                it += 1
            total += it
    el = (time.perf_counter() - t0) * 1000.0
    if total == 0: print("checksum:", total)
    return el

def bench_gc_churn():
    t0 = time.perf_counter()
    acc = 0
    for i in range(1000000):
        b = [0, 0, 0, 0, 0, 0, 0, 0]
        b[0] = i & 255
        acc += b[0]
    el = (time.perf_counter() - t0) * 1000.0
    if acc == 0: print("checksum:", acc)
    return el

if __name__ == "__main__":
    t_gemm = bench_gemm()
    t_bnn = bench_bnn()
    t_sort = bench_sort()
    t_sieve = bench_sieve()
    t_phys = bench_physics()
    t_json = bench_json()
    t_fib = bench_fib()
    t_crc = bench_crc32()
    t_mandel = bench_mandel()
    t_gc = bench_gc_churn()

    res = {
        "lang": "Python 3.11",
        "gemm": round(t_gemm, 2),
        "bnn": round(t_bnn, 2),
        "sort": round(t_sort, 2),
        "sieve": round(t_sieve, 2),
        "physics": round(t_phys, 2),
        "json": round(t_json, 2),
        "fib": round(t_fib, 2),
        "crc32": round(t_crc, 2),
        "mandel": round(t_mandel, 2),
        "gc_churn": round(t_gc, 2)
    }
    print(json.dumps(res))
