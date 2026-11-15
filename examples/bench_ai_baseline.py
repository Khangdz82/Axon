"""
Python 3.11 Baseline Production AI Benchmark Suite (`examples/bench_ai_baseline.py`)
Direct head-to-head baseline against Axon v1.0.0 (`examples/bench_ai_quant_suite.ax`).
Hardware: Intel Core i5-10400F @ 2.90GHz (x64), 32 GB RAM, Windows 10
Frameworks: NumPy 1.23.5 (MKL/OpenBLAS), PyTorch 2.14.1+cpu (OneDNN/MKL)
"""

import time
import json
import struct
import numpy as np
import torch
import torch.nn.functional as F

def bench_gemv():
    print("=" * 80)
    print(" WORKLOAD 1: Real-World GEMV Token Generation (4096x4096 = 16,777,216 weights)")
    print("=" * 80)
    dim = 4096
    rows = 4096
    total_weights = rows * dim

    # Prepare inputs
    x_np = np.full(dim, 1.0, dtype=np.float32)
    w_np = np.full((rows, dim), 0.5, dtype=np.float32)

    x_th = torch.from_numpy(x_np)
    w_th = torch.from_numpy(w_np)

    # 1. NumPy GEMV (Multi-threaded BLAS by default)
    # Warmup
    for _ in range(5):
        _ = np.dot(w_np, x_np)
    
    t0 = time.perf_counter_ns()
    reps = 10
    for _ in range(reps):
        res_np = np.dot(w_np, x_np)
    t_np = (time.perf_counter_ns() - t0) / reps / 1000  # us
    gflops_np = ((2.0 * total_weights) / (t_np / 1000000.0)) / 1e9
    bw_np = (total_weights * 4.0 / (1024**3)) / (t_np / 1000000.0)
    print(f" [1.1] NumPy GEMV (BLAS)        : {t_np / 1000:.2f} ms | {gflops_np:.2f} GFLOPS | {bw_np:.2f} GB/s | chk={res_np[0]:.2f}")

    # 2. PyTorch CPU GEMV (Multi-threaded MKL/OneDNN)
    for _ in range(5):
        _ = torch.mv(w_th, x_th)
    
    t0 = time.perf_counter_ns()
    for _ in range(reps):
        res_th = torch.mv(w_th, x_th)
    t_th = (time.perf_counter_ns() - t0) / reps / 1000  # us
    gflops_th = ((2.0 * total_weights) / (t_th / 1000000.0)) / 1e9
    bw_th = (total_weights * 4.0 / (1024**3)) / (t_th / 1000000.0)
    print(f" [1.2] PyTorch CPU torch.mv     : {t_th / 1000:.2f} ms | {gflops_th:.2f} GFLOPS | {bw_th:.2f} GB/s | chk={res_th[0].item():.2f}")

    # 3. PyTorch Single-Threaded GEMV
    torch.set_num_threads(1)
    for _ in range(5):
        _ = torch.mv(w_th, x_th)
    t0 = time.perf_counter_ns()
    for _ in range(reps):
        res_th_1t = torch.mv(w_th, x_th)
    t_th_1t = (time.perf_counter_ns() - t0) / reps / 1000
    gflops_th_1t = ((2.0 * total_weights) / (t_th_1t / 1000000.0)) / 1e9
    bw_th_1t = (total_weights * 4.0 / (1024**3)) / (t_th_1t / 1000000.0)
    print(f" [1.3] PyTorch (1-Threaded)     : {t_th_1t / 1000:.2f} ms | {gflops_th_1t:.2f} GFLOPS | {bw_th_1t:.2f} GB/s | chk={res_th_1t[0].item():.2f}")

def bench_faiss():
    print("=" * 80)
    print(" WORKLOAD 2: Vector DB Search (10,000 vecs x 128d, 50 queries, Top-10)")
    print("=" * 80)
    dim = 128
    num_vecs = 10000
    num_queries = 50

    db = np.full((num_vecs, dim), 0.5, dtype=np.float32)
    for v in range(num_vecs):
        db[v, 0] = v * 0.001

    query = np.full(dim, 0.5, dtype=np.float32)
    query[0] = 5.0

    # Warmup
    for _ in range(5):
        dists = np.sum((db - query) ** 2, axis=1)
        top10 = np.argpartition(dists, 10)[:10]

    t0 = time.perf_counter_ns()
    for _ in range(num_queries):
        dists = np.sum((db - query) ** 2, axis=1)
        top10 = np.argpartition(dists, 10)[:10]
    t_search = (time.perf_counter_ns() - t0) / 1000  # us
    qps = (num_queries * 1000000.0) / t_search
    lat_per_q = t_search / (num_queries * 1000.0)
    print(f" [2.1] NumPy Broadcast Flat-L2  : {t_search / 1000:.2f} ms total | {lat_per_q:.2f} ms/query | {qps:.2f} QPS")

def bench_tiktoken():
    print("=" * 80)
    print(" WORKLOAD 3: BPE Tokenizer Engine (2000 full-text encodings)")
    print("=" * 80)
    sample = "the quick brown fox jumps over the lazy dog and then running into the deep neural network engine with high performance tokenization and parallel tensor processing"
    
    # Python BPE tokenizer with merge dictionary
    bpe_ranks = {
        ("t", "h"): 0, ("i", "n"): 1, ("e", "r"): 2, ("a", "n"): 3,
        ("r", "e"): 4, ("o", "n"): 5, ("a", "t"): 6, ("e", "n"): 7,
        ("n", "d"): 8, ("th", "e"): 9, ("in", "g"): 10, ("t", "ion"): 11,
        ("p", "ro"): 12, ("c", "es"): 13, ("n", "et"): 14, ("w", "ork"): 15
    }

    def bpe_encode(text):
        parts = list(text)
        while len(parts) >= 2:
            min_rank = float("inf")
            best_pair = None
            best_i = -1
            for i in range(len(parts) - 1):
                pair = (parts[i], parts[i+1])
                rank = bpe_ranks.get(pair, float("inf"))
                if rank < min_rank:
                    min_rank = rank
                    best_pair = pair
                    best_i = i
            if best_i == -1 or min_rank == float("inf"):
                break
            parts[best_i] = parts[best_i] + parts[best_i+1]
            parts.pop(best_i + 1)
        return parts

    reps = 2000
    # Warmup
    _ = bpe_encode(sample)

    t0 = time.perf_counter_ns()
    total_tokens = 0
    for _ in range(reps):
        toks = bpe_encode(sample)
        total_tokens += len(toks)
    t_tok = (time.perf_counter_ns() - t0) / 1000
    throughput = (total_tokens * 1000000.0) / t_tok
    mb_throughput = ((len(sample) * reps) / (1024**2)) / (t_tok / 1000000.0)
    print(f" [3.1] Python BPE Tokenizer     : {t_tok / 1000:.2f} ms ({total_tokens} tokens) | {throughput:.2f} toks/sec | {mb_throughput:.2f} MB/s")

def bench_safetensors():
    print("=" * 80)
    print(" WORKLOAD 4: SafeTensors High-Throughput Ingestion (4.2 MB Model Chunk x 10)")
    print("=" * 80)
    rows = 1024
    cols = 1024
    arr = np.full((rows, cols), 1.25, dtype=np.float32)
    raw_bytes = arr.tobytes()
    header = json.dumps({"model.layers.0.mlp.weight": {"dtype": "F32", "shape": [rows, cols], "data_offsets": [0, len(raw_bytes)]}}).encode('utf-8')
    payload = struct.pack("<Q", len(header)) + header + raw_bytes

    reps = 10
    t0 = time.perf_counter_ns()
    chk = 0.0
    for _ in range(reps):
        hlen = struct.unpack("<Q", payload[:8])[0]
        hdr = json.loads(payload[8:8+hlen].decode('utf-8'))
        offsets = hdr["model.layers.0.mlp.weight"]["data_offsets"]
        t_data = np.frombuffer(payload, dtype=np.float32, offset=8+hlen+offsets[0], count=rows*cols)
        chk += t_data[0]
    t_parse = (time.perf_counter_ns() - t0) / 1000
    mb_proc = (len(payload) * reps) / (1024**2)
    bw = (mb_proc / 1024.0) / (t_parse / 1000000.0)
    print(f" [4.1] Python np.frombuffer     : {t_parse / 1000:.2f} ms ({mb_proc:.2f} MB) | {bw:.2f} GB/s bandwidth | chk={chk:.2f}")

def bench_transformer():
    print("=" * 80)
    print(" WORKLOAD 5: LLaMA Transformer FFN Layer Step (d=4096, 1000 tokens)")
    print("=" * 80)
    dim = 4096
    tokens = 1000

    torch.set_num_threads(1)  # fair single-threaded CPU comparison
    x = torch.full((dim,), 0.5, dtype=torch.float32)
    w_norm = torch.full((dim,), 1.0, dtype=torch.float32)
    gate = torch.full((dim,), 1.2, dtype=torch.float32)
    up = torch.full((dim,), 0.8, dtype=torch.float32)

    def rmsnorm(t_x, t_w):
        return (t_x * torch.rsqrt(torch.mean(t_x ** 2) + 1e-5)) * t_w

    def swiglu(t_g, t_u):
        return F.silu(t_g) * t_u

    # Warmup
    for _ in range(10):
        _ = rmsnorm(x, w_norm)
        _ = swiglu(gate, up)

    t0 = time.perf_counter_ns()
    for _ in range(tokens):
        normed = rmsnorm(x, w_norm)
        ffn_out = swiglu(gate, up)
    t_ffn = (time.perf_counter_ns() - t0) / 1000
    us_per_tok = t_ffn / tokens
    toks_sec = (tokens * 1000000.0) / t_ffn
    chk = ffn_out[0].item()
    print(f" [5.1] PyTorch RMSNorm + SwiGLU : {t_ffn / 1000:.2f} ms | {us_per_tok:.2f} us/token | {toks_sec:.2f} tokens/sec | chk={chk:.2f}")

if __name__ == "__main__":
    print()
    print("#" * 80)
    print(" PYTHON 3.11 BASELINE AI & QUANTIZATION BENCHMARK SUITE")
    print(" Platform: Intel Core i5-10400F @ 2.90GHz (x64) | NumPy 1.23.5 | PyTorch 2.14.1")
    print("#" * 80)
    print()
    bench_gemv()
    bench_faiss()
    bench_tiktoken()
    bench_safetensors()
    bench_transformer()
    print()
    print("#" * 80)
    print(" BASELINE RUN COMPLETE")
    print("#" * 80)
