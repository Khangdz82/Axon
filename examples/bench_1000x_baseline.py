"""
Head-to-Head 1,000x Baseline Benchmark Suite (Python 3.11)
File: examples/bench_1000x_baseline.py

Workloads:
  B1: Game ECS SIMD Simulation (100,000 entities x 50 frames)
  B2: Naive Attention vs FlashAttention-2 (seq_len=512, d=64)
  B3: 1M element SIMD array vectorization (200 steps)
  B4: 100,000 JSON microservice records scanning
  B5: 5,000,000 packet financial transaction settlement
"""

import time
import math
import json

def bench_b1_ecs():
    print("=" * 80)
    print(" WORKLOAD B1: Game Engine ECS Simulation (100,000 entities x 50 frames)")
    print("=" * 80)
    num_entities = 100000
    frames = 50

    px = [0.0] * num_entities
    py = [100.0] * num_entities
    pz = [0.0] * num_entities
    vx = [1.5] * num_entities
    vy = [-0.5] * num_entities
    vz = [0.0] * num_entities

    dt = 0.016
    gx = 0.0
    gy = -9.81
    gz = 0.0

    t0 = time.perf_counter_ns()
    for _ in range(frames):
        for i in range(num_entities):
            vx[i] += gx * dt
            vy[i] += gy * dt
            vz[i] += gz * dt
            px[i] += vx[i] * dt
            py[i] += vy[i] * dt
            pz[i] += vz[i] * dt
    t_ecs = (time.perf_counter_ns() - t0) / 1000 # us
    ms_tot = t_ecs / 1000.0
    ms_frame = ms_tot / frames
    fps = (frames * 1000000.0) / t_ecs
    updates_sec = (num_entities * frames * 1000000.0) / t_ecs
    print(f" [B1] Python List-based ECS    : {ms_tot:.2f} ms total | {ms_frame:.2f} ms/frame ({fps:.2f} FPS) | {updates_sec:.2f} updates/s")
    return ms_tot

def bench_b2_attention():
    print("=" * 80)
    print(" WORKLOAD B2: Naive Attention (seq_len=512, head_dim=64)")
    print("=" * 80)
    seq_len = 512
    head_dim = 64
    inv_scale = 1.0 / math.sqrt(head_dim)

    # Initialize matrices
    q = [[0.1] * head_dim for _ in range(seq_len)]
    k = [[0.1] * head_dim for _ in range(seq_len)]
    v = [[1.0] * head_dim for _ in range(seq_len)]

    t0 = time.perf_counter_ns()
    
    # 1. Compute Q * K^T / sqrt(d) -> [seq_len x seq_len] score matrix
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

    # 2. Softmax per row
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

    # 3. Output = P * V -> [seq_len x head_dim]
    out = [[0.0] * head_dim for _ in range(seq_len)]
    for i in range(seq_len):
        row_p = probs[i]
        row_o = out[i]
        for j in range(seq_len):
            pj = row_p[j]
            vj = v[j]
            for d in range(head_dim):
                row_o[d] += pj * vj[d]

    t_attn = (time.perf_counter_ns() - t0) / 1000 # us
    ms_tot = t_attn / 1000.0
    mem_mb = (seq_len * seq_len * 4.0) / (1024 * 1024)
    print(f" [B2] Python Naive Attention   : {ms_tot:.2f} ms total | Matrix Memory: {mem_mb:.2f} MB | out[0][0]={out[0][0]:.2f}")
    return ms_tot

def bench_b3_simd_array():
    print("=" * 80)
    print(" WORKLOAD B3: 1M Element Array Vectorization (200 steps)")
    print("=" * 80)
    n = 1000000
    total_steps = 200
    bench_steps = 10
    dt = 0.016

    a = [1.0] * n
    b = [0.5] * n

    t0 = time.perf_counter_ns()
    for _ in range(bench_steps):
        for i in range(n):
            a[i] += b[i] * dt
    t_measured = (time.perf_counter_ns() - t0) / 1000 # us
    ms_step = (t_measured / bench_steps) / 1000.0
    ms_projected = ms_step * total_steps
    gflops = ((n * 2.0) / (ms_step / 1000.0)) / 1e9
    print(f" [B3] Python Array Iteration   : {ms_projected:.2f} ms (200 steps equiv, {ms_step:.2f} ms/step) | {gflops:.3f} GFLOPS | a[0]={a[0]:.2f}")
    return ms_projected

def bench_b4_json():
    print("=" * 80)
    print(" WORKLOAD B4: JSON Microservice Records Scanning (100,000 records)")
    print("=" * 80)
    doc = '{"order_id": 9876543, "user_id": 42109, "symbol": "BTCUSD", "qty": 1500, "price": 96500, "status": "FILLED", "verified": true}'
    reps = 100000

    # Warmup
    _ = json.loads(doc)

    t0 = time.perf_counter_ns()
    total_qty = 0
    for _ in range(reps):
        rec = json.loads(doc)
        total_qty += rec["qty"]
    t_json = (time.perf_counter_ns() - t0) / 1000 # us
    ms_tot = t_json / 1000.0
    us_rec = t_json / reps
    qps = (reps * 1000000.0) / t_json
    print(f" [B4] Python C-json loads      : {ms_tot:.2f} ms total | {us_rec:.2f} us/record | {qps:.2f} recs/s | sum={total_qty}")
    return ms_tot

def bench_b5_financial():
    print("=" * 80)
    print(" WORKLOAD B5: Financial Transaction Settlement (5,000,000 packets)")
    print("=" * 80)
    n = 5000000
    t0 = time.perf_counter_ns()
    tot_vol = 0
    tot_fees = 0
    chk = 0
    for i in range(n):
        shares = (i % 100) + 1
        price = 10000 + (i % 500)
        val = shares * price
        fee = (val * 5) // 10000
        tot_vol += val
        tot_fees += fee
        chk ^= (val + fee)
    t_fin = (time.perf_counter_ns() - t0) / 1000 # us
    ms_tot = t_fin / 1000.0
    ops_sec = (n * 1000000.0) / t_fin
    print(f" [B5] Python HFT Settlement    : {ms_tot:.2f} ms total | {ops_sec:.2f} pkts/s | vol={tot_vol} fees={tot_fees} chk={chk}")
    return ms_tot

if __name__ == "__main__":
    print()
    print("#" * 80)
    print(" PYTHON 3.11 1,000X BASELINE BENCHMARK SUITE")
    print(" Platform: Intel Core i5-10400F @ 2.90GHz (x64) | 32 GB RAM | Single-Threaded")
    print("#" * 80)
    print()
    t_b1 = bench_b1_ecs()
    t_b2 = bench_b2_attention()
    t_b3 = bench_b3_simd_array()
    t_b4 = bench_b4_json()
    t_b5 = bench_b5_financial()
    print()
    print("#" * 80)
    print(" BASELINE MEASUREMENTS SUMMARY")
    print(f"   B1 (Game ECS 100k x 50f)       : {t_b1:.2f} ms")
    print(f"   B2 (Naive Attention 512x64)    : {t_b2:.2f} ms")
    print(f"   B3 (1M Array Vector 200 steps) : {t_b3:.2f} ms")
    print(f"   B4 (JSON Scan 100,000 recs)    : {t_b4:.2f} ms")
    print(f"   B5 (Financial 5,000,000 pkts)  : {t_b5:.2f} ms")
    print("#" * 80)
    print()
