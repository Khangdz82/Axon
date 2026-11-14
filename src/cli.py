# Aether CLI Compiler Driver
import os
import sys
import subprocess
import argparse
from pathlib import Path
from typing import Optional

_root_dir = Path(__file__).resolve().parent.parent
if str(_root_dir) not in sys.path:
    sys.path.insert(0, str(_root_dir))

from src.lexer import Lexer
from src.parser import Parser
from src.analyzer import LocalReasoningAnalyzer
from src.codegen_c import CCodeGen

VCVARS_PATH = r"C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"

def find_vcvars() -> str:
    if os.path.exists(VCVARS_PATH):
        return VCVARS_PATH
    # Fallback to search vswhere
    try:
        out = subprocess.check_output([
            r"C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe",
            "-latest", "-property", "installationPath"
        ], text=True).strip()
        candidate = os.path.join(out, "VC", "Auxiliary", "Build", "vcvars64.bat")
        if os.path.exists(candidate):
            return candidate
    except Exception:
        pass
    return ""

def compile_aether(source_path: str, output_exe: Optional[str] = None, mode: str = "release", run_after: bool = False, main_harness: Optional[str] = None):
    with open(source_path, "r", encoding="utf-8") as f:
        code = f.read()

    # 1. Frontend
    tokens = Lexer(code).tokenize()
    prog = Parser(tokens).parse_program()

    # 2. Local Reasoning Type & Safety Analysis
    analyzer = LocalReasoningAnalyzer(prog)
    analyzer.analyze()

    # 3. Code Generation (C23 / Vectorized C)
    cg = CCodeGen(prog)
    c_code = cg.generate()

    if main_harness:
        c_code += "\n\n" + main_harness

    # Output paths
    build_dir = Path("build")
    build_dir.mkdir(exist_ok=True)
    base_name = Path(source_path).stem
    c_file = build_dir / f"{base_name}.c"

    with open(c_file, "w", encoding="utf-8") as f:
        f.write(c_code)

    if not output_exe:
        output_exe = str(build_dir / f"{base_name}.exe")

    # Native Compilation via MSVC
    runtime_dir = Path("runtime").resolve()
    vcvars = find_vcvars()
    if not vcvars:
        print("Warning: Visual Studio vcvars64.bat not found. Emitted C file only at:", c_file)
        return

    has_main = any(f.name == "main" for f in prog.functions) or (main_harness is not None)

    c_flags = "/O2 /arch:AVX2 /fp:fast" if mode == "release" else "/Od"
    include_flag = f'/I"{runtime_dir}"'

    if has_main:
        cl_cmd = f'"{vcvars}" && cl {c_flags} {include_flag} "{c_file.resolve()}" /Fe:"{Path(output_exe).resolve()}" /Fo:"{build_dir.resolve()}/"'
        target_name = output_exe
    else:
        obj_file = build_dir / f"{base_name}.obj"
        cl_cmd = f'"{vcvars}" && cl /c {c_flags} {include_flag} "{c_file.resolve()}" /Fo:"{obj_file.resolve()}"'
        target_name = str(obj_file)

    print(f"[*] Compiling Aether -> {mode.upper()} ({target_name})...")
    res = subprocess.run(f'cmd /c "{cl_cmd}"', capture_output=True, text=True, shell=True)
    if res.returncode != 0:
        print("[!] Compilation Failed:")
        print(res.stdout)
        print(res.stderr)
        sys.exit(1)

    print(f"[+] Build Successful: {target_name}")

    if run_after and has_main:
        print(f"[*] Running {output_exe}...\n" + "=" * 50)
        run_res = subprocess.run(f'"{Path(output_exe).resolve()}"', shell=True)
        print("=" * 50)

def main():
    parser = argparse.ArgumentParser(description="Aether Compiler & Toolchain")
    subparsers = parser.add_subparsers(dest="cmd")

    build_p = subparsers.add_parser("build", help="Build Aether file")
    build_p.add_argument("file", help="Path to .ae file")
    build_p.add_argument("-o", "--output", help="Output executable path")
    build_p.add_argument("--mode", choices=["dev", "release"], default="release")

    run_p = subparsers.add_parser("run", help="Build and run Aether file")
    run_p.add_argument("file", help="Path to .ae file")
    run_p.add_argument("--mode", choices=["dev", "release"], default="release")

    args = parser.parse_args()
    if args.cmd == "build":
        compile_aether(args.file, args.output, args.mode, run_after=False)
    elif args.cmd == "run":
        compile_aether(args.file, None, args.mode, run_after=True)
    else:
        parser.print_help()

if __name__ == "__main__":
    main()
