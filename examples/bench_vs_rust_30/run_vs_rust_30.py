#!/usr/bin/env python3
"""30-Benchmark Light-to-Impossible Runner: Axon (-O3) vs Rust (rustc -C opt-level=3 -C target-cpu=native)."""

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
SRC_AX = os.path.join(DIR, "axon_30_extreme.ax")
SRC_RS = os.path.join(DIR, "rust_30_extreme.rs")
OUT_JSON = os.path.join(DIR, "results_vs_rust_30.json")

BENCH_META = [
    ("B01", "Light",      "branchless_clamp_10m", "exact"),
    ("B02", "Light",      "fib_rec_38",           "exact"),
    ("B03", "Light",      "sieve_1m",             "exact"),
    ("B04", "Light",      "popcnt_10m",           "exact"),
    ("B05", "Light",      "dot_f64_8m",           "fp"),
    ("B06", "Light",      "splitmix64_25m",       "exact"),
    ("B07", "Light",      "parse_i64_500k",       "exact"),
    ("B08", "Light",      "fnv1a_8m",             "exact"),
    ("B09", "Medium",     "sort_500k_i64",        "exact"),
    ("B10", "Medium",     "hashmap_500k",         "exact"),
    ("B11", "Medium",     "mem_find_4m",          "exact"),
    ("B12", "Medium",     "json_100k_parse",      "exact"),
    ("B13", "Medium",     "base64_2m_rt",         "exact"),
    ("B14", "Medium",     "csv_100k_agg",         "exact"),
    ("B15", "Medium",     "spsc_ring_10m",        "exact"),
    ("B16", "Medium",     "dijkstra_20k",         "exact"),
    ("B17", "Hard",       "crc32_8m",             "exact"),
    ("B18", "Hard",       "sha256_4m",            "exact"),
    ("B19", "Hard",       "chacha20_4m",          "exact"),
    ("B20", "Hard",       "nbody_3d_256",         "fp"),
    ("B21", "Hard",       "knapsack_dp_20m",      "exact"),
    ("B22", "Hard",       "mandelbrot_1k",        "fp"),
    ("B23", "Hard",       "fft_65536",            "fp"),
    ("B24", "Impossible", "fp32_gemm_512",        "fp"),
    ("B25", "Impossible", "fp64_gemm_512",        "fp"),
    ("B26", "Impossible", "bnn_gemv_4096",        "exact"),
    ("B27", "Impossible", "q4_0_gemv_4096",       "fp"),
    ("B28", "Impossible", "q8_0_gemv_4096",       "fp"),
    ("B29", "Impossible", "faiss_l2_10k_128d",    "fp"),
    ("B30", "Impossible", "llm_ffn_4096",         "fp"),
]


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
        results.append(parsed)
    return results


def main() -> int:
    runs = 5
    with tempfile.TemporaryDirectory(prefix="axon_vs_rust_30_") as tmp:
        ax_exe = os.path.join(tmp, "axon_30_extreme.exe")
        rs_exe = os.path.join(tmp, "rust_30_extreme.exe")

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
    rows = []
    ratios = []
    ax_total_ms = 0.0
    rs_total_ms = 0.0
    axon_wins = 0

    print("\n" + "=" * 104)
    print(f"{'ID':<4} {'Tier':<10} {'Benchmark':<22} {'Axon (ms)':>10} {'Rust (ms)':>10} {'Speedup':>10} {'Checksum':<14}")
    print("-" * 104)

    for idx, (bid, tier, name, mode) in enumerate(BENCH_META):
        ax_ms = statistics.median(r["times_us"][idx] for r in ax_runs) / 1000.0
        rs_ms = statistics.median(r["times_us"][idx] for r in rs_runs) / 1000.0
        ax_total_ms += ax_ms
        rs_total_ms += rs_ms
        ratio = rs_ms / max(ax_ms, 0.001)
        ratios.append(ratio)
        if ax_ms <= rs_ms:
            axon_wins += 1

        ac, rc = ax_chks[idx], rs_chks[idx]
        if mode == "exact":
            ok = (ac == rc)
        else:
            tol = max(5, int(abs(rc) * 0.002))
            ok = abs(ac - rc) <= tol
        if not ok:
            mismatches.append((bid, name, ac, rc))

        status = "EXACT" if ac == rc else ("FP_OK" if ok else f"DIFF({ac}!={rc})")
        print(f"{bid:<4} {tier:<10} {name:<22} {ax_ms:10.3f} {rs_ms:10.3f} {ratio:9.2f}x {status:<14}")
        rows.append({
            "id": bid, "tier": tier, "name": name,
            "axon_ms": round(ax_ms, 4), "rust_ms": round(rs_ms, 4),
            "speedup": round(ratio, 3), "axon_chk": ac, "rust_chk": rc, "ok": ok,
        })

    geomean = math.exp(sum(math.log(max(x, 1e-6)) for x in ratios) / len(ratios))
    wall_speedup = rs_total_ms / max(ax_total_ms, 0.001)
    print("=" * 104)
    print(f"TOTAL WALL TIME : Axon = {ax_total_ms:.2f} ms | Rust = {rs_total_ms:.2f} ms | Wall Speedup = {wall_speedup:.2f}x")
    print(f"GEOMEAN SPEEDUP : {geomean:.2f}x | AXON WINS: {axon_wins}/30 | COMPILE SPEEDUP: {rs_compile_ms/max(ax_compile_ms,0.1):.1f}x")
    print(f"CHECKSUM PARITY : {30 - len(mismatches)}/30 verified")
    if mismatches:
        print("MISMATCHES:")
        for m in mismatches:
            print("  ", m)

    summary = {
        "runs": runs,
        "axon_compile_ms": round(ax_compile_ms, 2),
        "rust_compile_ms": round(rs_compile_ms, 2),
        "axon_bin_bytes": ax_size,
        "rust_bin_bytes": rs_size,
        "axon_total_ms": round(ax_total_ms, 3),
        "rust_total_ms": round(rs_total_ms, 3),
        "wall_speedup": round(wall_speedup, 3),
        "geomean_speedup": round(geomean, 3),
        "axon_wins": axon_wins,
        "checksum_mismatches": len(mismatches),
        "benchmarks": rows,
    }
    with open(OUT_JSON, "w", encoding="utf-8", newline="\n") as f:
        json.dump(summary, f, indent=2)
    return 0 if not mismatches else 2


if __name__ == "__main__":
    sys.exit(main())
