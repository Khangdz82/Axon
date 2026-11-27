#!/usr/bin/env python3
"""100-Benchmark Strict Light-to-Impossible Suite: Axon (-O3) vs Rust (rustc -C opt-level=3 -C target-cpu=native).

Strict verification gates:
  1. 100/100 checksum parity (0 mismatches; exact i64 match on integer/crypto/sort/graph/DP/parse/bit benchmarks).
  2. 100/100 full win (Axon median ms <= Rust median ms on every single benchmark).
  3. Total wall time < 100.0 ms hard ceiling, < 50.0 ms strict target.
"""

import json
import math
import os
import statistics
import subprocess
import sys
import tempfile
import time

DIR = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(DIR, "..", ".."))
AXC = os.path.join(ROOT, "bin", "axc.exe" if os.name == "nt" else "axc-linux-x64")
SRC_AX = os.path.join(DIR, "axon_100_extreme.ax")
SRC_RS = os.path.join(DIR, "rust_100_extreme.rs")
OUT_JSON = os.path.join(DIR, "results_vs_rust_100.json")

BENCH_META = [
    # Domain 1: Scalar, Bitwise & Control (B001..B010)
    ("B001", "D1_Scalar",  "Light",      "branchless_clamp_4m",    "exact"),
    ("B002", "D1_Scalar",  "Light",      "fib_rec_37",             "exact"),
    ("B003", "D1_Scalar",  "Light",      "sieve_500k",             "exact"),
    ("B004", "D1_Scalar",  "Light",      "popcnt_4m",              "exact"),
    ("B005", "D1_Scalar",  "Light",      "splitmix64_3m",          "exact"),
    ("B006", "D1_Scalar",  "Light",      "collatz_steps_32k",      "exact"),
    ("B007", "D1_Scalar",  "Light",      "gcd_euclid_64k",         "exact"),
    ("B008", "D1_Scalar",  "Light",      "morton3d_encode_4m",     "exact"),
    ("B009", "D1_Scalar",  "Light",      "modexp_64_48k",          "exact"),
    ("B010", "D1_Scalar",  "Light",      "mat4x4_int_batch_32k",   "exact"),
    # Domain 2: SIMD Array & Reduction Math (B011..B020)
    ("B011", "D2_SIMD",    "Light",      "dot_f64_1m",             "fp"),
    ("B012", "D2_SIMD",    "Light",      "saxpy_f32_1m",           "fp"),
    ("B013", "D2_SIMD",    "Light",      "prefix_sum_i64_512k",    "exact"),
    ("B014", "D2_SIMD",    "Light",      "histogram_256_2m",       "exact"),
    ("B015", "D2_SIMD",    "Light",      "minmax_i64_2m",          "exact"),
    ("B016", "D2_SIMD",    "Light",      "relu6_f32_1m",           "fp"),
    ("B017", "D2_SIMD",    "Light",      "l1_l2_norm_f64_1m",      "fp"),
    ("B018", "D2_SIMD",    "Medium",     "conv1d_k9_f32_512k",     "fp"),
    ("B019", "D2_SIMD",    "Medium",     "variance_f64_1m",        "fp"),
    ("B020", "D2_SIMD",    "Medium",     "mat_transpose_512_f64",  "fp"),
    # Domain 3: Parsing, Text & Serialization (B021..B030)
    ("B021", "D3_Parse",   "Medium",     "parse_i64_250k",         "exact"),
    ("B022", "D3_Parse",   "Medium",     "mem_find_2m",            "exact"),
    ("B023", "D3_Parse",   "Medium",     "json_30k_parse",         "exact"),
    ("B024", "D3_Parse",   "Medium",     "base64_512k_rt",         "exact"),
    ("B025", "D3_Parse",   "Medium",     "csv_50k_agg",            "exact"),
    ("B026", "D3_Parse",   "Medium",     "hex_rt_512k",            "exact"),
    ("B027", "D3_Parse",   "Medium",     "utf8_count_2m",          "exact"),
    ("B028", "D3_Parse",   "Medium",     "rle_rt_512k",            "exact"),
    ("B029", "D3_Parse",   "Medium",     "varint_pb_250k",         "exact"),
    ("B030", "D3_Parse",   "Medium",     "url_query_50k",          "exact"),
    # Domain 4: Sorting & Data Structures (B031..B040)
    ("B031", "D4_DS",      "Medium",     "sort_48k_i64",           "exact"),
    ("B032", "D4_DS",      "Medium",     "hashmap_12k",            "exact"),
    ("B033", "D4_DS",      "Medium",     "spsc_ring_1m",           "exact"),
    ("B034", "D4_DS",      "Medium",     "bsearch_80k",            "exact"),
    ("B035", "D4_DS",      "Medium",     "dsu_8x16k_ops",          "exact"),
    ("B036", "D4_DS",      "Medium",     "fenwick_8x16k_ops",      "exact"),
    ("B037", "D4_DS",      "Medium",     "topk_heap_500k",         "exact"),
    ("B038", "D4_DS",      "Medium",     "bitset_popcnt_4m",       "exact"),
    ("B039", "D4_DS",      "Medium",     "bloom_filter_200k",      "exact"),
    ("B040", "D4_DS",      "Medium",     "radix_hist_scatter_500k","exact"),
    # Domain 5: Graphs & Dynamic Programming (B041..B050)
    ("B041", "D5_GraphDP", "Hard",       "dijkstra_6k",            "exact"),
    ("B042", "D5_GraphDP", "Hard",       "bfs_16k",                "exact"),
    ("B043", "D5_GraphDP", "Hard",       "toposort_32k",           "exact"),
    ("B044", "D5_GraphDP", "Hard",       "floyd_warshall_160",     "exact"),
    ("B045", "D5_GraphDP", "Hard",       "pagerank_16k",           "fp"),
    ("B046", "D5_GraphDP", "Hard",       "knapsack_dp_2m",         "exact"),
    ("B047", "D5_GraphDP", "Hard",       "edit_dist_96x128",       "exact"),
    ("B048", "D5_GraphDP", "Hard",       "lcs_dp_96x128",          "exact"),
    ("B049", "D5_GraphDP", "Hard",       "coin_change_8x12k",      "exact"),
    ("B050", "D5_GraphDP", "Hard",       "lis_patience_32x2k",     "exact"),
    # Domain 6: Cryptography & Hashing (B051..B060)
    ("B051", "D6_Crypto",  "Hard",       "fnv1a_batch_2m",         "exact"),
    ("B052", "D6_Crypto",  "Hard",       "crc32_2m",               "exact"),
    ("B053", "D6_Crypto",  "Hard",       "sha256_batch_512k",      "exact"),
    ("B054", "D6_Crypto",  "Hard",       "chacha20_1m",            "exact"),
    ("B055", "D6_Crypto",  "Hard",       "siphash24_batch_2m",     "exact"),
    ("B056", "D6_Crypto",  "Hard",       "murmur3_batch_2m",       "exact"),
    ("B057", "D6_Crypto",  "Hard",       "xxhash64_batch_2m",      "exact"),
    ("B058", "D6_Crypto",  "Hard",       "blake2s_batch_512k",     "exact"),
    ("B059", "D6_Crypto",  "Hard",       "hmac_sha256_512",        "exact"),
    ("B060", "D6_Crypto",  "Hard",       "speck_ctr_2m",           "exact"),
    # Domain 7: Signal Processing & Numerical (B061..B070)
    ("B061", "D7_Signal",  "Hard",       "fft_16384",              "fp"),
    ("B062", "D7_Signal",  "Hard",       "dct_ii_192x64",          "fp"),
    ("B063", "D7_Signal",  "Hard",       "fir_64tap_128k",         "fp"),
    ("B064", "D7_Signal",  "Hard",       "iir_biquad_64x4k",       "fp"),
    ("B065", "D7_Signal",  "Hard",       "rk4_lorenz_128x1k",      "fp"),
    ("B066", "D7_Signal",  "Hard",       "gl16_quad_32k",          "fp"),
    ("B067", "D7_Signal",  "Hard",       "mc_pi_4m",               "exact"),
    ("B068", "D7_Signal",  "Hard",       "black_scholes_48k",      "fp"),
    ("B069", "D7_Signal",  "Hard",       "newton_cubic_48k",       "fp"),
    ("B070", "D7_Signal",  "Hard",       "bilinear_interp_512",    "fp"),
    # Domain 8: Physics, Geometry & Graphics (B071..B080)
    ("B071", "D8_Physics", "Extreme",    "nbody_3d_96x50",         "fp"),
    ("B072", "D8_Physics", "Extreme",    "mandelbrot_320",         "fp"),
    ("B073", "D8_Physics", "Extreme",    "julia_224",              "fp"),
    ("B074", "D8_Physics", "Extreme",    "ray_sphere_48k",         "exact"),
    ("B075", "D8_Physics", "Extreme",    "aabb_overlap_1k",        "exact"),
    ("B076", "D8_Physics", "Extreme",    "sobel_edge_512",         "exact"),
    ("B077", "D8_Physics", "Extreme",    "gauss_blur_512",         "fp"),
    ("B078", "D8_Physics", "Extreme",    "heat_2d_256x16",         "fp"),
    ("B079", "D8_Physics", "Extreme",    "verlet_3d_64kx12",       "fp"),
    ("B080", "D8_Physics", "Extreme",    "polygon_pip_64k",        "exact"),
    # Domain 9: Dense & Sparse Linear Algebra (B081..B090)
    ("B081", "D9_LinAlg",  "Extreme",    "fp32_gemm_384",          "fp"),
    ("B082", "D9_LinAlg",  "Extreme",    "fp64_gemm_256",          "fp"),
    ("B083", "D9_LinAlg",  "Extreme",    "csr_spmv_32kx3",         "fp"),
    ("B084", "D9_LinAlg",  "Extreme",    "cholesky_160",           "fp"),
    ("B085", "D9_LinAlg",  "Extreme",    "lu_factor_8x96",         "fp"),
    ("B086", "D9_LinAlg",  "Extreme",    "qr_mgs_128",             "fp"),
    ("B087", "D9_LinAlg",  "Extreme",    "jacobi_sym_8x32",        "fp"),
    ("B088", "D9_LinAlg",  "Extreme",    "tridiag_thomas_2k",      "fp"),
    ("B089", "D9_LinAlg",  "Extreme",    "cg_sparse_8kx16",        "fp"),
    ("B090", "D9_LinAlg",  "Extreme",    "outer_rank1_512x4",      "fp"),
    # Domain 10: Impossible AI, Quantized LLM & Vector DB (B091..B100)
    ("B091", "D10_AI_LLM", "Impossible", "bnn_gemv_4096",          "exact"),
    ("B092", "D10_AI_LLM", "Impossible", "q4_0_gemv_2048x2",       "fp"),
    ("B093", "D10_AI_LLM", "Impossible", "q8_0_gemv_2048",         "fp"),
    ("B094", "D10_AI_LLM", "Impossible", "bitnet_158_2048",        "fp"),
    ("B095", "D10_AI_LLM", "Impossible", "faiss_l2_5k_128d",       "fp"),
    ("B096", "D10_AI_LLM", "Impossible", "cosine_topk_4k_64d",     "fp"),
    ("B097", "D10_AI_LLM", "Impossible", "llm_ffn_1024",           "fp"),
    ("B098", "D10_AI_LLM", "Impossible", "softmax_2d_384",         "fp"),
    ("B099", "D10_AI_LLM", "Impossible", "flash_attn_128x64",      "fp"),
    ("B100", "D10_AI_LLM", "Impossible", "kmeans_lloyd_1536x32",   "fp"),
]

assert len(BENCH_META) == 100, f"Expected 100 benchmarks, got {len(BENCH_META)}"


def run_json_exe(exe_path: str, runs: int = 5):
    results = []
    for _ in range(runs):
        out = subprocess.check_output([exe_path], cwd=ROOT, text=True)
        parsed = None
        for ln in reversed(out.splitlines()):
            s = ln.strip()
            if s.startswith("{"):
                parsed = json.loads(s)
                break
        if parsed is None:
            raise RuntimeError(f"No JSON output from {exe_path}:\n{out}")
        if len(parsed["times_us"]) != 100 or len(parsed["chks"]) != 100:
            raise RuntimeError(f"Expected 100 entries, got {len(parsed['times_us'])}/{len(parsed['chks'])}")
        results.append(parsed)
    return results


def main() -> int:
    runs = 5
    with tempfile.TemporaryDirectory(prefix="axon_vs_rust_100_") as tmp:
        ax_exe = os.path.join(tmp, "axon_100_extreme.exe")
        rs_exe = os.path.join(tmp, "rust_100_extreme.exe")

        t0 = time.perf_counter()
        p_ax = subprocess.run([AXC, "build", SRC_AX, "-O3", "-o", ax_exe], cwd=ROOT, capture_output=True, text=True)
        ax_compile_ms = (time.perf_counter() - t0) * 1000.0
        if p_ax.returncode != 0:
            print("AXON BUILD FAILED:\n", p_ax.stdout, p_ax.stderr)
            return 1

        t0 = time.perf_counter()
        p_rs = subprocess.run(
            ["rustc", "-C", "opt-level=3", "-C", "target-cpu=native", SRC_RS, "-o", rs_exe],
            cwd=ROOT, capture_output=True, text=True,
        )
        rs_compile_ms = (time.perf_counter() - t0) * 1000.0
        if p_rs.returncode != 0:
            print("RUST BUILD FAILED:\n", p_rs.stdout, p_rs.stderr)
            return 1

        ax_size = os.path.getsize(ax_exe)
        rs_size = os.path.getsize(rs_exe)
        print(f"Compiled Axon (-O3): {ax_compile_ms:.1f} ms ({ax_size:,} B) | Rust (-O3 -C target-cpu=native): {rs_compile_ms:.1f} ms ({rs_size:,} B)")

        ax_runs = run_json_exe(ax_exe, runs)
        rs_runs = run_json_exe(rs_exe, runs)

    ax_chks = ax_runs[0]["chks"]
    rs_chks = rs_runs[0]["chks"]
    mismatches = []
    losses = []
    rows = []
    ratios = []
    domain_totals = {}
    ax_total_ms = 0.0
    rs_total_ms = 0.0
    axon_wins = 0

    print("\n" + "=" * 116)
    print(f"{'ID':<5} {'Domain':<11} {'Tier':<10} {'Benchmark':<24} {'Axon (ms)':>10} {'Rust (ms)':>10} {'Speedup':>9} {'Checksum':<14}")
    print("-" * 116)

    for idx, (bid, dom, tier, name, mode) in enumerate(BENCH_META):
        ax_ms = statistics.median(r["times_us"][idx] for r in ax_runs) / 1000.0
        rs_ms = statistics.median(r["times_us"][idx] for r in rs_runs) / 1000.0
        ax_total_ms += ax_ms
        rs_total_ms += rs_ms
        ratio = rs_ms / max(ax_ms, 0.001)
        ratios.append(ratio)

        d_ax, d_rs, d_w, d_n = domain_totals.get(dom, (0.0, 0.0, 0, 0))
        won = ax_ms <= rs_ms
        if won:
            axon_wins += 1
        else:
            losses.append((bid, name, ax_ms, rs_ms, ratio))
        domain_totals[dom] = (d_ax + ax_ms, d_rs + rs_ms, d_w + (1 if won else 0), d_n + 1)

        ac, rc = ax_chks[idx], rs_chks[idx]
        if mode == "exact":
            ok = (ac == rc)
        else:
            tol = max(10, int(abs(rc) * 0.003))
            ok = abs(ac - rc) <= tol
        if not ok:
            mismatches.append((bid, name, ac, rc))

        status = "EXACT" if ac == rc else ("FP_OK" if ok else f"DIFF({ac}!={rc})")
        mark = "" if won else " [LOSS]"
        print(f"{bid:<5} {dom:<11} {tier:<10} {name:<24} {ax_ms:10.3f} {rs_ms:10.3f} {ratio:8.2f}x {status:<14}{mark}")
        rows.append({
            "id": bid, "domain": dom, "tier": tier, "name": name,
            "axon_ms": round(ax_ms, 4), "rust_ms": round(rs_ms, 4),
            "speedup": round(ratio, 3), "axon_chk": ac, "rust_chk": rc, "ok": ok, "win": won,
        })

    print("=" * 116)
    print("DOMAIN SUMMARY:")
    for dom, (d_ax, d_rs, d_w, d_n) in domain_totals.items():
        print(f"  {dom:<11}: Axon = {d_ax:7.2f} ms | Rust = {d_rs:7.2f} ms | Speedup = {d_rs/max(d_ax,0.001):6.2f}x | Wins = {d_w}/{d_n}")

    geomean = math.exp(sum(math.log(max(x, 1e-6)) for x in ratios) / len(ratios))
    wall_speedup = rs_total_ms / max(ax_total_ms, 0.001)
    ax_run_totals = [sum(r["times_us"]) / 1000.0 for r in ax_runs]
    rs_run_totals = [sum(r["times_us"]) / 1000.0 for r in rs_runs]

    print("=" * 116)
    print(f"TOTAL WALL TIME : Axon = {ax_total_ms:.2f} ms (runs: min={min(ax_run_totals):.2f}, med={statistics.median(ax_run_totals):.2f}, max={max(ax_run_totals):.2f}) | Rust = {rs_total_ms:.2f} ms | Wall Speedup = {wall_speedup:.2f}x")
    print(f"GEOMEAN SPEEDUP : {geomean:.2f}x | AXON WINS: {axon_wins}/100 | COMPILE SPEEDUP: {rs_compile_ms/max(ax_compile_ms,0.1):.1f}x")
    print(f"CHECKSUM PARITY : {100 - len(mismatches)}/100 verified")
    print(f"STRICT GATES    : <100ms={'PASS' if ax_total_ms < 100.0 else 'FAIL'} | <50ms_TARGET={'PASS' if ax_total_ms < 50.0 else 'FAIL'} | 100/100_WIN={'PASS' if axon_wins == 100 else 'FAIL'} | PARITY={'PASS' if not mismatches else 'FAIL'}")

    if mismatches:
        print("MISMATCHES:")
        for m in mismatches:
            print("  ", m)
    if losses:
        print("LOSSES:")
        for l in losses:
            print(f"   {l[0]} {l[1]}: Axon={l[2]:.3f}ms vs Rust={l[3]:.3f}ms ({l[4]:.2f}x)")

    summary = {
        "runs": runs,
        "axon_compile_ms": round(ax_compile_ms, 2),
        "rust_compile_ms": round(rs_compile_ms, 2),
        "axon_bin_bytes": ax_size,
        "rust_bin_bytes": rs_size,
        "axon_total_ms": round(ax_total_ms, 3),
        "rust_total_ms": round(rs_total_ms, 3),
        "axon_run_totals_ms": [round(x, 3) for x in ax_run_totals],
        "rust_run_totals_ms": [round(x, 3) for x in rs_run_totals],
        "wall_speedup": round(wall_speedup, 3),
        "geomean_speedup": round(geomean, 3),
        "axon_wins": axon_wins,
        "checksum_mismatches": len(mismatches),
        "gate_under_100ms": ax_total_ms < 100.0,
        "gate_under_50ms": ax_total_ms < 50.0,
        "gate_full_win": axon_wins == 100,
        "benchmarks": rows,
    }
    with open(OUT_JSON, "w", encoding="utf-8", newline="\n") as f:
        json.dump(summary, f, indent=2)
    return 0 if (not mismatches and axon_wins == 100 and ax_total_ms < 50.0) else 2


if __name__ == "__main__":
    sys.exit(main())
