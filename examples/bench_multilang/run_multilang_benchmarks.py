import subprocess
import json
import os
import sys

def run_command(cmd, shell=False):
    res = subprocess.run(cmd, shell=shell, capture_output=True, text=True)
    if res.returncode != 0:
        print(f"Error running {cmd}: {res.stderr}")
        return None
    return res.stdout.strip()

def extract_json(stdout):
    for line in stdout.splitlines():
        line = line.strip()
        if line.startswith("{") and line.endswith("}"):
            try:
                return json.loads(line)
            except:
                pass
    return None

def main():
    print("=" * 80)
    print(" MULTI-LANGUAGE REAL-WORLD PRODUCTION BENCHMARK SUITE")
    print(" Benchmarking: C, Rust, Go, Java, C# (.NET 10), Node.js, Bun, Python, Axon")
    print("=" * 80)

    results = []

    # 1. C (MSVC)
    print("[1/9] Compiling and running C (MSVC /O2 /arch:AVX2)...")
    vcvars = r"C:\Program Files\Microsoft Visual Studio\18\Community\VC\Auxiliary\Build\vcvars64.bat"
    cmd_c = f'call "{vcvars}" && cl /O2 /arch:AVX2 /nologo examples\\bench_multilang\\c_bench.c /Fe:examples\\bench_multilang\\c_bench.exe && examples\\bench_multilang\\c_bench.exe'
    out_c = run_command(cmd_c, shell=True)
    data_c = extract_json(out_c) if out_c else None
    if data_c: results.append(data_c)

    # 2. Rust
    print("[2/9] Compiling and running Rust (rustc -C opt-level=3)...")
    cmd_rust_compile = ["rustc", "-C", "opt-level=3", "-C", "target-cpu=native", "examples/bench_multilang/rust_bench.rs", "-o", "examples/bench_multilang/rust_bench.exe"]
    run_command(cmd_rust_compile)
    out_rust = run_command(["examples/bench_multilang/rust_bench.exe"])
    data_rust = extract_json(out_rust) if out_rust else None
    if data_rust: results.append(data_rust)

    # 3. Go
    print("[3/9] Running Go (go 1.26)...")
    out_go = run_command(["go", "run", "examples/bench_multilang/go_bench.go"])
    data_go = extract_json(out_go) if out_go else None
    if data_go: results.append(data_go)

    # 4. Java
    print("[4/9] Compiling and running Java (JDK 21 LTS)...")
    javac = r"G:\RE\tools\jdk-21.0.13+9\bin\javac.EXE"
    java = r"G:\RE\tools\jdk-21.0.13+9\bin\java.EXE"
    run_command([javac, "-d", "examples/bench_multilang", "examples/bench_multilang/java_bench/Main.java"])
    out_java = run_command([java, "-cp", "examples/bench_multilang", "java_bench.Main"])
    data_java = extract_json(out_java) if out_java else None
    if data_java: results.append(data_java)

    # 5. C# (.NET 10)
    print("[5/9] Compiling and running C# (.NET 10 Release)...")
    out_cs = run_command(["dotnet", "run", "--project", "examples/bench_multilang/cs_bench", "-c", "Release"])
    data_cs = extract_json(out_cs) if out_cs else None
    if data_cs: results.append(data_cs)

    # 6. Node.js
    print("[6/9] Running JavaScript on Node.js (v24 V8)...")
    out_node = run_command(["node", "examples/bench_multilang/js_bench.js"])
    data_node = extract_json(out_node) if out_node else None
    if data_node: results.append(data_node)

    # 7. Bun
    print("[7/9] Running JavaScript on Bun (1.4 JSC)...")
    out_bun = run_command(["bun", "examples/bench_multilang/js_bench.js"])
    data_bun = extract_json(out_bun) if out_bun else None
    if data_bun: results.append(data_bun)

    # 8. Python
    print("[8/9] Running Python (CPython 3.11)...")
    out_py = run_command(["python", "examples/bench_multilang/python_bench.py"])
    data_py = extract_json(out_py) if out_py else None
    if data_py: results.append(data_py)

    # 9. Axon
    print("[9/9] Running Axon (v8.0 Native Zero-GC)...")
    out_axon = run_command([".\\bin\\axc.exe", "run", "examples/bench_multilang/axon_bench.ax"])
    data_axon = extract_json(out_axon) if out_axon else None
    if data_axon: results.append(data_axon)

    print("\n" + "=" * 80)
    print(" RAW EXECUTION TIMES (Milliseconds - Lower is Better)")
    print("=" * 80)

    workloads = [
        ("gemm", "Matrix GEMM (256x256 FP32)"),
        ("bnn", "BNN POPCOUNT GEMV (4096)"),
        ("sort", "Quicksort (100k Int64)"),
        ("sieve", "Sieve Primes (2,000,000)"),
        ("physics", "3D Kinematics (50k x 10)"),
        ("json", "JSON Scan (100k runs)")
    ]

    header = f"{'Language / Runtime':<30} | " + " | ".join([f"{w[0].upper():<9}" for w in workloads])
    print(header)
    print("-" * len(header))

    for r in results:
        row = f"{r['lang']:<30} | "
        for k, _ in workloads:
            row += f"{r[k]:<9.2f} | "
        print(row)

    # Speedup relative to Python
    if data_py:
        print("\n" + "=" * 80)
        print(" SPEEDUP vs PYTHON 3.11 (Higher is Better, e.g. 10x, 100x)")
        print("=" * 80)
        header_sp = f"{'Language / Runtime':<30} | " + " | ".join([f"{w[0].upper():<9}" for w in workloads])
        print(header_sp)
        print("-" * len(header_sp))

        for r in results:
            row = f"{r['lang']:<30} | "
            for k, _ in workloads:
                py_val = data_py[k]
                cur_val = r[k]
                sp = py_val / cur_val if cur_val > 0 else 0
                row += f"{sp:<8.1f}x | "
            print(row)

    # Write summary JSON for reporting
    with open("examples/bench_multilang/results.json", "w") as fp:
        json.dump(results, fp, indent=2)

if __name__ == "__main__":
    main()
