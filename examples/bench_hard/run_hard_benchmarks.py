import json, os, shutil, subprocess, sys, tempfile

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
DIR = os.path.dirname(os.path.abspath(__file__))
AXC = os.path.join(ROOT, "bin", "axc.exe")

def run_json(cmd, cwd=DIR):
    p = subprocess.run(cmd, cwd=cwd, capture_output=True, text=True, shell=isinstance(cmd, str))
    if p.returncode != 0:
        raise RuntimeError(f"Failed ({cmd}): {p.stderr or p.stdout}")
    for line in reversed(p.stdout.strip().splitlines()):
        line = line.strip()
        if line.startswith("{"):
            return json.loads(line)
    raise RuntimeError(f"No JSON output from {cmd}: {p.stdout}")

def find_vcvars():
    vswhere = os.path.expandvars(r"%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe")
    if not os.path.exists(vswhere): return None
    p = subprocess.run([vswhere, "-latest", "-products", "*", "-requires", "Microsoft.VisualStudio.Component.VC.Tools.x86.x64", "-property", "installationPath"], capture_output=True, text=True)
    vc = p.stdout.strip()
    return os.path.join(vc, "VC", "Auxiliary", "Build", "vcvars64.bat") if vc else None

def main():
    tmp = tempfile.mkdtemp(prefix="axon_hard_")
    results = []
    try:
        vcvars = find_vcvars()
        if vcvars:
            print("[1/7] C (MSVC /O2 /arch:AVX2)...")
            c_exe = os.path.join(tmp, "c_hard.exe")
            c_obj = os.path.join(tmp, "c_hard.obj")
            run_json(f'"{vcvars}" >nul && cl /O2 /arch:AVX2 /Fo"{c_obj}" /Fe:"{c_exe}" "{os.path.join(DIR, "c_hard.c")}" >nul && "{c_exe}"')
            results.append(run_json([c_exe]))
        if shutil.which("rustc"):
            print("[2/7] Rust (rustc -O3)...")
            r_exe = os.path.join(tmp, "rust_hard.exe")
            subprocess.run(["rustc", "-C", "opt-level=3", os.path.join(DIR, "rust_hard.rs"), "-o", r_exe], check=True)
            results.append(run_json([r_exe]))
        if shutil.which("go"):
            print("[3/7] Go (go 1.26)...")
            g_exe = os.path.join(tmp, "go_hard.exe")
            subprocess.run(["go", "build", "-o", g_exe, os.path.join(DIR, "go_hard.go")], check=True)
            results.append(run_json([g_exe]))
        if shutil.which("node"):
            print("[4/7] Node.js (v24 V8)...")
            results.append(run_json(["node", os.path.join(DIR, "js_hard.js")]))
        if shutil.which("bun"):
            print("[5/7] Bun (JSC)...")
            results.append(run_json(["bun", os.path.join(DIR, "js_hard.js")]))
        print("[6/7] Python 3.11...")
        results.append(run_json([sys.executable, os.path.join(DIR, "python_hard.py")]))
        print("[7/7] Axon v1.0.0 (Native Zero-GC)...")
        a_exe = os.path.join(tmp, "axon_hard.exe")
        subprocess.run([AXC, "build", os.path.join(DIR, "axon_hard.ax"), "-o", a_exe], check=True)
        results.append(run_json([a_exe]))
    finally:
        shutil.rmtree(tmp, ignore_errors=True)

    # Verify exact mathematical checksums across ALL languages!
    ref = results[0]["chk"]
    print("\n=== CHECKSUM VERIFICATION ===")
    for r in results:
        ok = (r["chk"] == ref)
        print(f"  {r['lang']:<28} chk={r['chk']} {'[MATCH]' if ok else '[MISMATCH!]'}")
        if not ok:
            print("  WARNING: Checksum mismatch!", file=sys.stderr)

    cols = ["nbody", "matmul64", "arx", "dijkstra", "knapsack", "hashmap"]
    print("\n" + "=" * 96)
    print(f"{'Language / Runtime':<28} | " + " | ".join(f"{c.upper():<9}" for c in cols))
    print("-" * 96)
    for r in results:
        print(f"{r['lang']:<28} | " + " | ".join(f"{r[c]:<9.2f}" for c in cols))
    print("=" * 96)
    with open(os.path.join(DIR, "results_hard.json"), "w") as f:
        json.dump(results, f, indent=2)

if __name__ == "__main__":
    main()
