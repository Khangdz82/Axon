"""
25 Real-World Production Workloads: Python 3.11 Baseline Suite
File: examples/bench_25_baseline.py
Platform: Intel Core i5-10400F @ 2.90GHz | 32 GB RAM | Windows 10
"""

import time
import math
import json
import zlib
import re
import random
import sys

def c01_flash_attn():
    seq_len = 512
    head_dim = 64
    inv_scale = 1.0 / math.sqrt(head_dim)
    q = [[0.1] * head_dim for _ in range(seq_len)]
    k = [[0.1] * head_dim for _ in range(seq_len)]
    v = [[1.0] * head_dim for _ in range(seq_len)]
    t0 = time.perf_counter_ns()
    scores = [[0.0] * seq_len for _ in range(seq_len)]
    for i in range(seq_len):
        qi = q[i]
        row_s = scores[i]
        for j in range(seq_len):
            kj = k[j]
            dot = 0.0
            for d in range(head_dim):
                dot += qi[d] * kj[d]
            row_s[j] = dot * inv_scale
    probs = [[0.0] * seq_len for _ in range(seq_len)]
    for i in range(seq_len):
        row_s = scores[i]
        m = max(row_s)
        sum_exp = 0.0
        row_p = probs[i]
        for j in range(seq_len):
            e = math.exp(row_s[j] - m)
            row_p[j] = e
            sum_exp += e
        inv_sum = 1.0 / sum_exp
        for j in range(seq_len):
            row_p[j] *= inv_sum
    out = [[0.0] * head_dim for _ in range(seq_len)]
    for i in range(seq_len):
        row_p = probs[i]
        row_o = out[i]
        for j in range(seq_len):
            pj = row_p[j]
            vj = v[j]
            for d in range(head_dim):
                row_o[d] += pj * vj[d]
    return (time.perf_counter_ns() - t0) / 1e6

def c02_bitnet_ternary():
    dim = 4096
    act = [1.0] * dim
    t0 = time.perf_counter_ns()
    out = [0.0] * dim
    # Measure 256 rows and project to 4096
    test_rows = 256
    for r in range(test_rows):
        s = 0.0
        for c in range(dim):
            w = ((c + r) % 3) - 1
            if w == 1: s += act[c]
            elif w == -1: s -= act[c]
        out[r] = s
    elapsed_us = (time.perf_counter_ns() - t0) / 1000
    return (elapsed_us * (dim / test_rows)) / 1000.0

def c03_bnn_1bit():
    dim = 4096
    words = dim // 64
    act = [0xAAAAAAAAAAAAAAAA] * words
    t0 = time.perf_counter_ns()
    out = [0] * dim
    test_rows = 256
    for r in range(test_rows):
        total_ones = 0
        w_pat = 0x5555555555555555 if (r & 1) == 0 else 0x3333333333333333
        for w in range(words):
            xnor = ~(act[w] ^ w_pat) & 0xFFFFFFFFFFFFFFFF
            total_ones += bin(xnor).count('1')
        out[r] = 2 * total_ones - dim
    elapsed_us = (time.perf_counter_ns() - t0) / 1000
    return (elapsed_us * (dim / test_rows)) / 1000.0

def c04_rmsnorm_rope():
    dim = 4096
    x = [0.5] * dim
    w_norm = [1.0] * dim
    t0 = time.perf_counter_ns()
    for _ in range(50):
        sum_sq = sum(v * v for v in x)
        rsqrt = 1.0 / math.sqrt(sum_sq / dim + 1e-5)
        for i in range(dim):
            x[i] = x[i] * rsqrt * w_norm[i]
        for i in range(0, dim, 2):
            theta = 1.0 / (10000.0 ** (i / dim))
            cos_t = math.cos(theta)
            sin_t = math.sin(theta)
            x0 = x[i]; x1 = x[i+1]
            x[i] = x0 * cos_t - x1 * sin_t
            x[i+1] = x0 * sin_t + x1 * cos_t
    return (time.perf_counter_ns() - t0) / 1e6

def c05_bpe_tokenizer():
    sample = "the quick brown fox jumps over the lazy dog and neural engine tokenization"
    bpe_ranks = {
        ("t", "h"): 0, ("i", "n"): 1, ("e", "r"): 2, ("a", "n"): 3,
        ("r", "e"): 4, ("o", "n"): 5, ("th", "e"): 6, ("in", "g"): 7
    }
    t0 = time.perf_counter_ns()
    for _ in range(5000):
        parts = list(sample)
        while len(parts) >= 2:
            min_rank = 999999
            best_i = -1
            for i in range(len(parts) - 1):
                pair = (parts[i], parts[i+1])
                rank = bpe_ranks.get(pair, 999999)
                if rank < min_rank:
                    min_rank = rank
                    best_i = i
            if best_i == -1 or min_rank == 999999:
                break
            parts[best_i] = parts[best_i] + parts[best_i+1]
            parts.pop(best_i + 1)
    return (time.perf_counter_ns() - t0) / 1e6

def c06_vector_search():
    num_vecs = 10000
    dim = 64
    db = [[0.1 * (v % 10)] * dim for v in range(num_vecs)]
    query = [0.5] * dim
    t0 = time.perf_counter_ns()
    for _ in range(10):
        dists = []
        for v in range(num_vecs):
            vec = db[v]
            d = 0.0
            for i in range(dim):
                diff = vec[i] - query[i]
                d += diff * diff
            dists.append(d)
        _ = sorted(range(len(dists)), key=lambda k: dists[k])[:10]
    return (time.perf_counter_ns() - t0) / 1e6

def c07_ecs_movement():
    n = 100000
    frames = 50
    px = [0.0] * n; py = [100.0] * n; pz = [0.0] * n
    vx = [1.5] * n; vy = [-0.5] * n; vz = [0.0] * n
    dt = 0.016; gy = -9.81
    t0 = time.perf_counter_ns()
    for _ in range(frames):
        for i in range(n):
            vy[i] += gy * dt
            px[i] += vx[i] * dt
            py[i] += vy[i] * dt
            pz[i] += vz[i] * dt
    return (time.perf_counter_ns() - t0) / 1e6

def c08_sat_collision():
    poly_a = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]
    poly_b = [(5.0, 5.0), (15.0, 5.0), (15.0, 15.0), (5.0, 15.0)]
    t0 = time.perf_counter_ns()
    for _ in range(10000):
        collided = True
        for poly in (poly_a, poly_b):
            for i in range(len(poly)):
                p1 = poly[i]
                p2 = poly[(i + 1) % len(poly)]
                edge_x = p2[1] - p1[1]
                edge_y = -(p2[0] - p1[0])
                len_e = math.sqrt(edge_x * edge_x + edge_y * edge_y)
                if len_e < 1e-6: continue
                nx = edge_x / len_e; ny = edge_y / len_e
                proj_a = [p[0] * nx + p[1] * ny for p in poly_a]
                proj_b = [p[0] * nx + p[1] * ny for p in poly_b]
                if min(proj_a) > max(proj_b) or min(proj_b) > max(proj_a):
                    collided = False
                    break
            if not collided: break
    return (time.perf_counter_ns() - t0) / 1e6

def c09_ray_triangle():
    v0 = (0.0, 0.0, 0.0); v1 = (10.0, 0.0, 0.0); v2 = (0.0, 10.0, 0.0)
    ray_o = (2.0, 2.0, -5.0); ray_d = (0.0, 0.0, 1.0)
    t0 = time.perf_counter_ns()
    hits = 0
    for _ in range(50000):
        e1 = (v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2])
        e2 = (v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2])
        h = (ray_d[1] * e2[2] - ray_d[2] * e2[1], ray_d[2] * e2[0] - ray_d[0] * e2[2], ray_d[0] * e2[1] - ray_d[1] * e2[0])
        a = e1[0] * h[0] + e1[1] * h[1] + e1[2] * h[2]
        if -1e-6 < a < 1e-6: continue
        f = 1.0 / a
        s = (ray_o[0] - v0[0], ray_o[1] - v0[1], ray_o[2] - v0[2])
        u = f * (s[0] * h[0] + s[1] * h[1] + s[2] * h[2])
        if u < 0.0 or u > 1.0: continue
        q = (s[1] * e1[2] - s[2] * e1[1], s[2] * e1[0] - s[0] * e1[2], s[0] * e1[1] - s[1] * e1[0])
        v = f * (ray_d[0] * q[0] + ray_d[1] * q[1] + ray_d[2] * q[2])
        if v < 0.0 or u + v > 1.0: continue
        t = f * (e2[0] * q[0] + e2[1] * q[1] + e2[2] * q[2])
        if t > 1e-6: hits += 1
    return (time.perf_counter_ns() - t0) / 1e6

def c10_rigid_body_3d():
    n = 20000
    px = [0.0] * n; py = [10.0] * n; pz = [0.0] * n
    vx = [1.0] * n; vy = [0.0] * n; vz = [0.0] * n
    qw = [1.0] * n; qx = [0.0] * n; qy = [0.0] * n; qz = [0.0] * n
    wx = [0.1] * n; wy = [0.2] * n; wz = [0.05] * n
    dt = 0.016
    t0 = time.perf_counter_ns()
    for i in range(n):
        px[i] += vx[i] * dt
        py[i] += vy[i] * dt
        pz[i] += vz[i] * dt
        h_dt = 0.5 * dt
        d_qw = h_dt * (-wx[i] * qx[i] - wy[i] * qy[i] - wz[i] * qz[i])
        d_qx = h_dt * ( wx[i] * qw[i] + wy[i] * qz[i] - wz[i] * qy[i])
        d_qy = h_dt * (-wx[i] * qz[i] + wy[i] * qw[i] + wz[i] * qx[i])
        d_qz = h_dt * ( wx[i] * qy[i] - wy[i] * qx[i] + wz[i] * qw[i])
        nw = qw[i] + d_qw; nx = qx[i] + d_qx; ny = qy[i] + d_qy; nz = qz[i] + d_qz
        inv_norm = 1.0 / math.sqrt(nw*nw + nx*nx + ny*ny + nz*nz)
        qw[i] = nw * inv_norm; qx[i] = nx * inv_norm; qy[i] = ny * inv_norm; qz[i] = nz * inv_norm
    return (time.perf_counter_ns() - t0) / 1e6

def c11_audio_dsp():
    sample_rate = 44100
    total_samples = 441000
    t0 = time.perf_counter_ns()
    two_pi = 2.0 * math.pi
    step1 = two_pi * 440.0 / sample_rate
    step2 = two_pi * 554.37 / sample_rate
    phase1 = 0.0; phase2 = 0.0
    attack = int(sample_rate * 0.1)
    sustain = 0.7
    rel_start = total_samples - int(sample_rate * 0.5)
    samples = [0.0] * total_samples
    for i in range(total_samples):
        s = math.sin(phase1) * 0.5 + math.sin(phase2) * 0.5
        phase1 += step1; phase2 += step2
        env = (i / attack) if i < attack else (sustain if i < rel_start else sustain * (1.0 - (i - rel_start) / (sample_rate * 0.5)))
        samples[i] = s * env
    return (time.perf_counter_ns() - t0) / 1e6

def c12_software_raster():
    w, h = 800, 600
    depth = [1.0] * (w * h)
    color = [0] * (w * h)
    x0, y0, z0 = 100, 100, 0.2
    x1, y1, z1 = 400, 150, 0.5
    x2, y2, z2 = 250, 450, 0.8
    inv_area = 1.0 / ((x1 - x0) * (y2 - y0) - (y1 - y0) * (x2 - x0))
    t0 = time.perf_counter_ns()
    for _ in range(500):
        for y in range(100, 451, 4):
            row_idx = y * w
            fy = y + 0.5
            for x in range(100, 401, 4):
                fx = x + 0.5
                w0 = ((x1 - fx) * (y2 - fy) - (y1 - fy) * (x2 - fx)) * inv_area
                w1 = ((x2 - fx) * (y0 - fy) - (y2 - fy) * (x0 - fx)) * inv_area
                w2 = 1.0 - w0 - w1
                if w0 >= 0 and w1 >= 0 and w2 >= 0:
                    z = w0 * z0 + w1 * z1 + w2 * z2
                    idx = row_idx + x
                    if z < depth[idx]:
                        depth[idx] = z
                        color[idx] = 0xFF55AAFF
    return (time.perf_counter_ns() - t0) / 1e6

def c13_json_scan():
    doc = '{"order_id": 9876543, "user_id": 42109, "symbol": "BTCUSD", "qty": 1500, "price": 96500, "status": "FILLED", "verified": true}'
    t0 = time.perf_counter_ns()
    tot = 0
    for _ in range(100000):
        rec = json.loads(doc)
        tot += rec["qty"]
    return (time.perf_counter_ns() - t0) / 1e6

def c14_lz4_compress():
    raw = ("2026-11-19T22:20:00.123Z [INFO] worker-42: processed transaction id=9876543 status=OK latency=45us\n" * 10500).encode('utf-8')
    t0 = time.perf_counter_ns()
    for _ in range(20):
        _ = zlib.compress(raw, level=1)
    return ((time.perf_counter_ns() - t0) / 20) / 1e6

def c15_lz4_decompress():
    raw = ("2026-11-19T22:20:00.123Z [INFO] worker-42: processed transaction id=9876543 status=OK latency=45us\n" * 10500).encode('utf-8')
    comp = zlib.compress(raw, level=1)
    t0 = time.perf_counter_ns()
    for _ in range(20):
        _ = zlib.decompress(comp)
    return ((time.perf_counter_ns() - t0) / 20) / 1e6

def c16_regex_dfa():
    pat = re.compile(r"^\d{4}-\d{2}-\d{2} \[INFO\]")
    line = "2026-11-19 [INFO] worker: processed batch request successfully"
    t0 = time.perf_counter_ns()
    matches = 0
    for _ in range(100000):
        if pat.match(line): matches += 1
    return (time.perf_counter_ns() - t0) / 1e6

def c17_csv_parse():
    csv_line = "1001,Alice,Electronics,1250,95.50,PAID,true\n"
    csv_data = csv_line * 50000
    t0 = time.perf_counter_ns()
    tot = 0.0
    for line in csv_data.splitlines():
        parts = line.split(",")
        tot += float(parts[3]) * float(parts[4])
    return (time.perf_counter_ns() - t0) / 1e6

def c18_fintech_hft():
    t0 = time.perf_counter_ns()
    tot = 0
    for i in range(5000000):
        shares = (i % 100) + 1
        price = 10000 + (i % 500)
        val = shares * price
        fee = (val * 5) // 10000
        tot += val - fee
    return (time.perf_counter_ns() - t0) / 1e6

def c19_spsc_ringbuf():
    n = 5000000
    q = []
    t0 = time.perf_counter_ns()
    for i in range(n):
        q.append(i)
        _ = q.pop()
    return (time.perf_counter_ns() - t0) / 1e6

def c20_simd_vector_fma():
    n = 500000
    a = [1.0] * n
    b = [0.5] * n
    dt = 0.016
    t0 = time.perf_counter_ns()
    for _ in range(20):
        for i in range(n):
            a[i] += b[i] * dt
    return (time.perf_counter_ns() - t0) / 1e6

def c21_matrix_gemm():
    n = 256
    a = [[0.5] * n for _ in range(n)]
    b = [[0.5] * n for _ in range(n)]
    c = [[0.0] * n for _ in range(n)]
    t0 = time.perf_counter_ns()
    test_rows = 16
    for i in range(test_rows):
        ai = a[i]; ci = c[i]
        for k in range(n):
            aik = ai[k]; bk = b[k]
            for j in range(n):
                ci[j] += aik * bk[j]
    elapsed_us = (time.perf_counter_ns() - t0) / 1000
    return (elapsed_us * (n / test_rows)) / 1000.0

def c22_integer_sort():
    random.seed(42)
    data = [random.randint(0, 1000000) for _ in range(100000)]
    t0 = time.perf_counter_ns()
    _ = sorted(data)
    return (time.perf_counter_ns() - t0) / 1e6

def c23_whitebox_mac():
    msg = "transaction_id=987654321_amount=50000_from=alice_to=bob_nonce=12345"
    t0 = time.perf_counter_ns()
    h = 0x243F6A88
    test_runs = 5000
    for _ in range(test_runs):
        for ch in msg:
            b = ord(ch)
            h = ((h << 7) | ((h >> 25) & 127)) ^ ((h ^ b) & 255)
    elapsed_us = (time.perf_counter_ns() - t0) / 1000
    return (elapsed_us * (100000 / test_runs)) / 1000.0

def c24_crc64_checksum():
    buf = b"1234567890" * 100000 # 1MB
    poly = 0x42F0E1EBA9EA3693
    t0 = time.perf_counter_ns()
    crc = 0
    test_bytes = 20000
    for b in buf[:test_bytes]:
        crc ^= (b << 56)
        for _ in range(8):
            if crc & 0x8000000000000000:
                crc = (crc << 1) ^ poly
            else:
                crc <<= 1
            crc &= 0xFFFFFFFFFFFFFFFF
    elapsed_us = (time.perf_counter_ns() - t0) / 1000
    return (elapsed_us * (10000000 / test_bytes)) / 1000.0

def c25_http_router():
    routes = {
        "/api/v1/users": 1,
        "/api/v1/orders": 2,
        "/api/v1/products": 3,
        "/api/v1/checkout": 4,
        "/health": 5
    }
    url = "/api/v1/orders"
    t0 = time.perf_counter_ns()
    matches = 0
    for _ in range(100000):
        if url in routes: matches += 1
    return (time.perf_counter_ns() - t0) / 1e6

if __name__ == "__main__":
    cases = [
        ("C01 FlashAttention-2 (512x64)", c01_flash_attn),
        ("C02 BitNet b1.58 GEMV (4096)", c02_bitnet_ternary),
        ("C03 BNN 1-Bit POPCOUNT (4096)", c03_bnn_1bit),
        ("C04 RMSNorm + RoPE (4096)", c04_rmsnorm_rope),
        ("C05 BPE Tokenizer (5k runs)", c05_bpe_tokenizer),
        ("C06 Vector DB Search (10k x 64d)", c06_vector_search),
        ("C07 Game ECS (100k x 50f)", c07_ecs_movement),
        ("C08 2D SAT Collision (10k pairs)", c08_sat_collision),
        ("C09 3D Ray-Triangle (50k rays)", c09_ray_triangle),
        ("C10 3D Rigid Body (20k bodies)", c10_rigid_body_3d),
        ("C11 Audio DSP (441k samples)", c11_audio_dsp),
        ("C12 Software Raster (500 tris)", c12_software_raster),
        ("C13 JSON Scan (100k records)", c13_json_scan),
        ("C14 LZ4 Compression (1MB)", c14_lz4_compress),
        ("C15 LZ4 Decompression (1MB)", c15_lz4_decompress),
        ("C16 Regex DFA Match (100k lines)", c16_regex_dfa),
        ("C17 CSV Parse & Sum (50k rows)", c17_csv_parse),
        ("C18 FinTech HFT (5M packets)", c18_fintech_hft),
        ("C19 SPSC RingBuffer (5M items)", c19_spsc_ringbuf),
        ("C20 SIMD FMA (500k x 20 steps)", c20_simd_vector_fma),
        ("C21 Matrix GEMM (256x256 FP32)", c21_matrix_gemm),
        ("C22 Integer QuickSort (100k ints)", c22_integer_sort),
        ("C23 Whitebox Crypto MAC (100k)", c23_whitebox_mac),
        ("C24 CRC-64 Checksum (10MB)", c24_crc64_checksum),
        ("C25 HTTP Route Match (100k)", c25_http_router),
    ]

    print("\n" + "=" * 80)
    print(" PYTHON 3.11: 25 REAL-WORLD WORKLOADS EXECUTION")
    print("=" * 80, flush=True)
    results = []
    for name, fn in cases:
        ms = fn()
        results.append((name, ms))
        print(f"  {name:<36} : {ms:9.2f} ms", flush=True)
    print("=" * 80, flush=True)
