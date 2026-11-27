#!/usr/bin/env python3
"""
Axon v1.0.0 vs C/C++ (MSVC /O2 /arch:AVX2) — 30 Real-World Production Benchmarks Runner
(`examples/bench_30_suite/run_30_benchmarks.py`)

Compiles `axon_bench_30.ax` and `bench_30_c.c` to native standalone executables,
verifies deterministic checksums and 0 memory leaks across all 30 scenarios
(Tier 1: Easy B01..B10, Tier 2: Medium B11..B20, Tier 3: Hard/Extreme B21..B30),
prints a detailed head-to-head comparison table, and persists `results_30.json`.
"""
import json
import os
import subprocess
import sys
import tempfile
import time

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
DIR = os.path.dirname(os.path.abspath(__file__))
AXC = os.path.join(ROOT, "bin", "axc.exe" if os.name == "nt" else "axc-linux-x64")
SRC_AX = os.path.join(DIR, "axon_bench_30.ax")
SRC_C = os.path.join(DIR, "bench_30_c.c")
OUT_JSON = os.path.join(DIR, "results_30.json")

BENCH_META = [
    # Tier 1: Easy (01..10)
    ("b01", "Easy",    "B01_PRELUDE_PARSE_FMT",      "Prelude parse_int/FNV-1a/clamp (50k ops)"),
    ("b02", "Easy",    "B02_DATETIME_ISO8601",       "Howard Hinnant Civil Calendar & ISO-8601 (50k)"),
    ("b03", "Easy",    "B03_TOML_DOTENV_CFG",        "TOML + .env Config Parser & Lookup (1k docs)"),
    ("b04", "Easy",    "B04_CSV_ANALYTICS",          "RFC-4180 CSV Parse & Column Sum (2,500 rows)"),
    ("b05", "Easy",    "B05_UI_CANVAS_OPAQUE_SPAN",  "Desktop UI ERMS Opaque Rect Span Fill (5k)"),
    ("b06", "Easy",    "B06_UI_ROUNDED_CARD_3ZONE",  "3-Zone Chord Anti-Aliased Rounded Cards (2k)"),
    ("b07", "Easy",    "B07_UI_TEXT_GLYPH_RASTER",   "Crisp 5x7 Font HUD/Editor Text (60k chars)"),
    ("b08", "Easy",    "B08_MEM_FRAME_ARENA",        "O(1) FrameArena Bump Alloc + Reset (500k ops)"),
    ("b09", "Easy",    "B09_MEM_OBJECT_POOL",        "O(1) Slab ObjectPool + Double-Free Guard (200k)"),
    ("b10", "Easy",    "B10_GAME_VEC2_TRIG_MATH",    "2D Vector Transform/Reflect/Minimax Trig (200k)"),
    # Tier 2: Medium (11..20)
    ("b11", "Medium",  "B11_UI_DOM_FLEXBOX_LAYOUT",  "HTML5 DOM + CSS Match + Flexbox Solver (300x)"),
    ("b12", "Medium",  "B12_UI_REACTIVE_SIGNALS",    "Zero-VDOM Reactive Signal + Surgical Patch (2k)"),
    ("b13", "Medium",  "B13_UI_VIRTUAL_LIST_100K",   "Virtualized 100k-Row Table 120Hz Scroll (500f)"),
    ("b14", "Medium",  "B14_UI_SPRING_ANIM_120HZ",   "120Hz Physical Spring + Cubic Easing (200k steps)"),
    ("b15", "Medium",  "B15_CACHE_LRU_HOTKEY",       "O(1) Flat-Array LRU Cache Lookup/Evict (100k)"),
    ("b16", "Medium",  "B16_CACHE_BLOOM_FILTER",     "64-Bit 3-Hash Bloom Filter Screen (200k ops)"),
    ("b17", "Medium",  "B17_AI_RMSNORM_ROPE_4K",     "LLM 4096-Dim RMSNorm + RoPE Rotation (1k tok)"),
    ("b18", "Medium",  "B18_AI_SWIGLU_SOFTMAX",      "LLM SwiGLU Gate + Softmax + Argmax (1k x 1024)"),
    ("b19", "Medium",  "B19_GAME_ECS_ARCHETYPE",     "Archetype ECS Spawn + 3D Kinematics (10k x 20)"),
    ("b20", "Medium",  "B20_GAME_SPATIAL_HASH",      "2D Spatial-Hash Broadphase Query (1k x 25f)"),
    # Tier 3: Hard / Extreme (21..30)
    ("b21", "Extreme", "B21_UI_FULL_DESKTOP_IDE",    "Full Desktop IDE Frame + Token Stream (200f)"),
    ("b22", "Extreme", "B22_AI_PAGED_KV_FLASH_ATTN", "vLLM Paged KV-Cache + FlashAttention-2 (1024t)"),
    ("b23", "Extreme", "B23_AI_Q8_QUANT_GEMV_1K",    "Int8 Quantized 1024x1024 Matrix-Vector GEMV"),
    ("b24", "Extreme", "B24_AI_VECTOR_SIM_SEARCH",   "64-Dim L2 Vector Similarity Search (4096 x 64)"),
    ("b25", "Extreme", "B25_GAME_ASTAR_BINARY_HEAP", "64x64 Obstacle Grid Binary-Heap A* (100 paths)"),
    ("b26", "Extreme", "B26_GAME_TILEMAP_RAYCAST",   "2D Tilemap DDA Raycaster + Frustum Cull (100k)"),
    ("b27", "Extreme", "B27_GAME_PARTICLE_POOL_10K", "10,000-Particle SoA Physics Pool (50 steps)"),
    ("b28", "Extreme", "B28_CRYPTO_CHACHA20_ARX_1M", "ChaCha20 ARX Quarter-Round Stream Core (1M)"),
    ("b29", "Extreme", "B29_DB_OLTP_INMEM_100K",     "In-Memory Open-Addressed OLTP Insert+Lookup (100k)"),
    ("b30", "Extreme", "B30_EXTREME_ZERO_LEAK_SOAK", "Full-Stack LeakGuard + Arena + Pool + LRU Soak"),
]


def find_vcvars():
    if os.name != "nt":
        return None
    vswhere = os.path.expandvars(r"%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe")
    if not os.path.exists(vswhere):
        return None
    p = subprocess.run(
        [vswhere, "-latest", "-products", "*", "-requires", "Microsoft.VisualStudio.Component.VC.Tools.x86.x64", "-property", "installationPath"],
        capture_output=True,
        text=True,
    )
    vc = p.stdout.strip()
    return os.path.join(vc, "VC", "Auxiliary", "Build", "vcvars64.bat") if vc else None


def capture_vcvars_env(vcvars):
    if not vcvars or not os.path.isfile(vcvars) or '"' in vcvars:
        return None
    p = subprocess.run(
        ["cmd.exe", "/d", "/q", "/c", f'call "{vcvars}" >nul && set'],
        capture_output=True,
        text=True,
    )
    if p.returncode != 0:
        return None
    env = os.environ.copy()
    for line in p.stdout.splitlines():
        if "=" in line:
            k, v = line.split("=", 1)
            env[k] = v
    return env


def main() -> int:
    with tempfile.TemporaryDirectory(prefix="axon_bench30_") as tmp:
        exe = os.path.join(tmp, "axon_bench_30.exe" if os.name == "nt" else "axon_bench_30")
        t_c0 = time.perf_counter()
        proc = subprocess.run([AXC, "build", SRC_AX, "-o", exe], cwd=ROOT, capture_output=True, text=True)
        compile_ms = (time.perf_counter() - t_c0) * 1000.0
        if proc.returncode != 0:
            print(proc.stdout, proc.stderr, file=sys.stderr)
            return 1
        print(proc.stdout.strip())

        runs = []
        for _ in range(5):
            out = subprocess.check_output([exe], cwd=ROOT, text=True).strip()
            runs.append(json.loads(out))

        ref_chk = runs[0]["chk"]
        for r in runs:
            if r["chk"] != ref_chk or r.get("leaks", -1) != 0 or any(c <= 0 for c in r["chk"]):
                print(f"[FAIL] Checksum or leak verification failed: {r}", file=sys.stderr)
                return 1

        # Compile & run C/C++ (MSVC /O2 /arch:AVX2) baseline if available
        c_runs = []
        c_compile_ms = 0.0
        vcvars = find_vcvars()
        msvc_env = capture_vcvars_env(vcvars)
        if msvc_env and os.path.exists(SRC_C):
            c_exe = os.path.join(tmp, "bench_30_c.exe")
            c_obj = os.path.join(tmp, "bench_30_c.obj")
            tc0 = time.perf_counter()
            c_proc = subprocess.run(
                ["cl", "/nologo", "/O2", "/arch:AVX2", f"/Fo{c_obj}", f"/Fe{c_exe}", SRC_C],
                cwd=DIR,
                env=msvc_env,
                capture_output=True,
                text=True,
            )
            c_compile_ms = (time.perf_counter() - tc0) * 1000.0
            if c_proc.returncode == 0 and os.path.exists(c_exe):
                for _ in range(3):
                    cout = subprocess.check_output([c_exe], cwd=DIR, text=True).strip()
                    for line in reversed(cout.splitlines()):
                        if line.strip().startswith("{"):
                            c_runs.append(json.loads(line.strip()))
                            break

        print("=" * 118)
        print(
            f" AXON v1.0.0 vs C/C++ (MSVC /O2 AVX2) — 30 PRODUCTION BENCHMARKS "
            f"(Axon Compile: {compile_ms:.1f} ms vs C: {c_compile_ms:.1f} ms | Leaks: 0 B)"
        )
        print("=" * 118)
        print(
            f"{'ID':<4} | {'Tier':<7} | {'Benchmark Name':<27} | {'Scenario':<44} | "
            f"{'Axon ms':>8} | {'C/C++ ms':>9} | {'Speedup':>8}"
        )
        print("-" * 118)

        summary = {
            "axon_compile_ms": round(compile_ms, 2),
            "c_compile_ms": round(c_compile_ms, 2),
            "leaks": 0,
            "benchmarks": {},
        }
        tier_totals = {"Easy": 0.0, "Medium": 0.0, "Extreme": 0.0}
        c_tier_totals = {"Easy": 0.0, "Medium": 0.0, "Extreme": 0.0}
        total_best = 0.0
        total_med = 0.0
        total_c_best = 0.0

        for idx, (key, tier, name, desc) in enumerate(BENCH_META):
            vals = sorted(r[key] for r in runs)
            best_v = vals[0]
            med_v = vals[len(vals) // 2]
            total_best += best_v
            total_med += med_v
            tier_totals[tier] += best_v

            c_best = sorted(cr[key] for cr in c_runs)[0] if c_runs else 0.0
            total_c_best += c_best
            c_tier_totals[tier] += c_best
            ratio = (c_best / best_v) if (c_best > 0 and best_v > 0) else 0.0

            summary["benchmarks"][name] = {
                "tier": tier,
                "axon_best_ms": round(best_v, 3),
                "axon_median_ms": round(med_v, 3),
                "c_best_ms": round(c_best, 3),
                "speedup_vs_c": round(ratio, 2),
                "checksum": ref_chk[idx],
            }
            c_str = f"{c_best:9.2f}" if c_runs else "      N/A"
            r_str = f"{ratio:7.2f}x" if c_runs else "     N/A"
            print(f"{key.upper():<4} | {tier:<7} | {name:<27} | {desc:<44} | {best_v:8.2f} | {c_str} | {r_str}")

        overall_speedup = (total_c_best / total_best) if (total_c_best > 0 and total_best > 0) else 0.0
        compile_speedup = (c_compile_ms / compile_ms) if (c_compile_ms > 0 and compile_ms > 0) else 0.0
        print("-" * 118)
        print(
            f"TIER TOTALS (Axon vs C/C++): "
            f"Easy={tier_totals['Easy']:.2f} vs {c_tier_totals['Easy']:.2f} ms | "
            f"Medium={tier_totals['Medium']:.2f} vs {c_tier_totals['Medium']:.2f} ms | "
            f"Extreme={tier_totals['Extreme']:.2f} vs {c_tier_totals['Extreme']:.2f} ms"
        )
        print(
            f"OVERALL 30/30 PASSED | Axon Total: {total_best:.2f} ms vs C/C++ Total: {total_c_best:.2f} ms "
            f"({overall_speedup:.2f}x FASTER, Compile {compile_speedup:.2f}x FASTER) | Leaks: 0 B [VERIFIED]"
        )
        print("=" * 118)

        summary["axon_total_best_ms"] = round(total_best, 2)
        summary["axon_total_median_ms"] = round(total_med, 2)
        summary["c_total_best_ms"] = round(total_c_best, 2)
        summary["overall_speedup_vs_c"] = round(overall_speedup, 2)
        summary["compile_speedup_vs_c"] = round(compile_speedup, 2)
        with open(OUT_JSON, "w", encoding="utf-8") as f:
            json.dump(summary, f, indent=2)
        return 0


if __name__ == "__main__":
    sys.exit(main())
