#!/usr/bin/env python3
import subprocess
import time
import os
import sys
from pathlib import Path

ROOT = Path(__file__).parent.parent.resolve()
BENCH_DIR = ROOT / "benchmarks"
AXC = ROOT / "target" / "release" / "axc.exe"
RUSTC = r"C:\Users\Admin\.cargo\bin\rustc.exe"
VCVARS = r"C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"

def run_cmd(cmd, cwd=ROOT):
    p = subprocess.run(cmd, capture_output=True, text=True, cwd=cwd, shell=True)
    return p.stdout, p.stderr, p.returncode

def main():
    print("=" * 70)
    print("      AXON COMPREHENSIVE PERFORMANCE & ARCHITECTURE BENCHMARK SUITE")
    print("=" * 70)
    print("Host Hardware: Intel(R) Core(TM) i5-10400F @ 2.90GHz (6C / 12T, AVX2)")
    print("Host Memory:   32 GB DDR4 Dual-Channel")
    print("OS & Tools:    Windows x64 | MSVC 19.37 | Rustc 1.100.0 | Axon axc 0.1.0\n")

    # 1. Particles Benchmark
    print("[1/5] Running Particle Simulation Benchmark (SIMD & Cache Locality)...")
    out_cpp, _, _ = run_cmd(f'"{BENCH_DIR / "bench_particles_cpp.exe"}"')
    out_rust, _, _ = run_cmd(f'"{BENCH_DIR / "bench_particles_rust.exe"}"')
    out_axon, _, _ = run_cmd(f'"{BENCH_DIR / "bench_axon_runner.exe"}"')

    print(out_cpp.strip())
    print(out_rust.strip())
    print(out_axon.strip())
    print("-" * 70)

    # 2. FBIP Allocation Benchmark
    print("\n[2/5] Running In-Place Memory Reuse Benchmark (Perceus FBIP vs Malloc/Free)...")
    out_fbip_c, _, _ = run_cmd(f'"{BENCH_DIR / "bench_fbip.exe"}"')
    out_fbip_rs, _, _ = run_cmd(f'"{BENCH_DIR / "bench_fbip_rust.exe"}"')
    print(out_fbip_c.strip())
    print(out_fbip_rs.strip())
    print("-" * 70)

    # 3. Refinement Types Bounds Elision
    print("\n[3/5] Running Refinement Types Bounds Check Elision Benchmark...")
    out_bounds_c, _, _ = run_cmd(f'"{BENCH_DIR / "bench_bounds.exe"}"')
    out_bounds_rs, _, _ = run_cmd(f'"{BENCH_DIR / "bench_bounds_rust.exe"}"')
    print(out_bounds_c.strip())
    print(out_bounds_rs.strip())
    print("-" * 70)

    # 4. Multi-Fiber Concurrency Benchmark
    print("\n[4/5] Running Colorless Work-Stealing Fibers vs OS Threads Benchmark...")
    out_concur, _, _ = run_cmd(f'"{BENCH_DIR / "bench_concurrency.exe"}"')
    print(out_concur.strip())
    print("-" * 70)

    # 5. Compilation Latency
    print("\n[5/5] Measuring Compilation & Dev Cycle Latency...")
    # Axon Dev JIT
    t0 = time.perf_counter()
    run_cmd(f'"{AXC}" run --dev tests/pass/particle_sim.ax')
    t_dev = (time.perf_counter() - t0) * 1000.0

    # Axon Release Transpile + MSVC
    t0 = time.perf_counter()
    run_cmd(f'"{AXC}" build --release tests/pass/particle_sim.ax')
    t_axon_rel = (time.perf_counter() - t0) * 1000.0

    # C++ MSVC compile
    t0 = time.perf_counter()
    run_cmd(f'cmd /c "call \\"{VCVARS}\\" >nul 2>&1 && cl /nologo /O2 /arch:AVX2 /fp:fast benchmarks/bench_particles_cpp.cpp /Febenchmarks/bench_particles_cpp.exe"')
    t_cpp = (time.perf_counter() - t0) * 1000.0

    # Rust rustc compile
    t0 = time.perf_counter()
    run_cmd(f'"{RUSTC}" -C opt-level=3 -C target-cpu=native benchmarks/bench_particles_rust.rs -o benchmarks/bench_particles_rust.exe')
    t_rust = (time.perf_counter() - t0) * 1000.0

    print(f"  [Axon Cranelift Dev JIT]   Compile + Run: {t_dev:8.2f} ms")
    print(f"  [Axon C23 + MSVC /O2]      Transpile + Link: {t_axon_rel:8.2f} ms")
    print(f"  [C++23 MSVC /O2 /AVX2]     Compile + Link: {t_cpp:8.2f} ms")
    print(f"  [Rust 1.100 rustc -O3]     Compile + Link: {t_rust:8.2f} ms")
    print("=" * 70)

if __name__ == "__main__":
    main()
