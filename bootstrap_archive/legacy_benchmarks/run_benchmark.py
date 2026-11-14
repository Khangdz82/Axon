import os
import sys
import subprocess
from pathlib import Path

# Add root directory to sys.path
root_dir = Path(__file__).parent.parent
sys.path.insert(0, str(root_dir))

from src.cli import compile_aether, find_vcvars

AETHER_MAIN_HARNESS = """
int main(void) {
    const size_t N = 1000000;
    const int FRAMES = 100;
    const float dt = 0.016f;

    Aether_Table_Particle particles = Aether_Table_Particle_create(N);
    for (size_t i = 0; i < N; ++i) {
        particles.pos_x[i] = (float)i;
        particles.pos_y[i] = (float)i * 0.5f;
        particles.vel_x[i] = 1.0f;
        particles.vel_y[i] = -0.5f;
        particles.mass[i]  = 1.0f;
    }

    double t0 = aether_get_time_sec();

    for (int f = 0; f < FRAMES; ++f) {
        // Run via colorless fiber with linear isolated ownership transfer
        particles = simulate_world(particles);
    }

    double t1 = aether_get_time_sec();
    double elapsed_ms = (t1 - t0) * 1000.0;

    printf("[Aether Native SoA + Colorless Fibers]\\n");
    printf("  Entities: %zu\\n", N);
    printf("  Frames:   %d\\n", FRAMES);
    printf("  Total Time: %.2f ms (%.3f ms/frame)\\n", elapsed_ms, elapsed_ms / FRAMES);
    printf("  Throughput: %.2f M updates/sec\\n", ((double)N * FRAMES / 1e6) / (elapsed_ms / 1000.0));
    printf("  Checksum: pos_x[0]=%.2f, pos_y[0]=%.2f\\n\\n", particles.pos_x[0], particles.pos_y[0]);

    Aether_Table_Particle_free(&particles);
    return 0;
}
"""

def run_bench():
    vcvars = find_vcvars()
    build_dir = Path("build")
    build_dir.mkdir(exist_ok=True)

    # 1. Compile C++ baseline
    print("==================================================")
    print("1. Compiling & Running C++ / Rust AoS Baseline...")
    print("==================================================")
    cpp_src = Path("benchmarks/cpp_baseline.cpp").resolve()
    cpp_exe = (build_dir / "cpp_baseline.exe").resolve()
    cl_cmd = f'"{vcvars}" && cl /O2 /arch:AVX2 /fp:fast "{cpp_src}" /Fe:"{cpp_exe}" /Fo:"{build_dir.resolve()}/"'
    res = subprocess.run(f'cmd /c "{cl_cmd}"', capture_output=True, text=True, shell=True)
    if res.returncode != 0:
        print("[!] C++ compilation failed:\n", res.stdout, res.stderr)
        return
    subprocess.run(f'"{cpp_exe}"', shell=True)

    # 2. Compile Aether Native SoA
    print("==================================================")
    print("2. Compiling & Running Aether Native SoA...")
    print("==================================================")
    ae_src = "examples/particle_sim.ae"
    ae_exe = str(build_dir / "aether_particle_sim.exe")
    compile_aether(ae_src, ae_exe, mode="release", run_after=True, main_harness=AETHER_MAIN_HARNESS)

if __name__ == "__main__":
    run_bench()
