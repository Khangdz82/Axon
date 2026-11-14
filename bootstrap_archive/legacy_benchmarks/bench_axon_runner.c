#define _CRT_SECURE_NO_WARNINGS
#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <windows.h>

#define main Axon_main_unused
#include "bench_particles_axon.c"
#undef main

static inline double get_time_ms(LARGE_INTEGER start, LARGE_INTEGER end, LARGE_INTEGER freq) {
    return (double)(end.QuadPart - start.QuadPart) * 1000.0 / (double)freq.QuadPart;
}

void bench_axon_scale(size_t N, int FRAMES, float dt, LARGE_INTEGER freq) {
    printf("--- Scale: N = %zu particles, %d frames ---\n", N, FRAMES);

    Axon_Table_Particle tbl = Axon_Table_Particle_with_capacity(N);
    for (size_t i = 0; i < N; ++i) {
        tbl.x[i] = (float)i;
        tbl.y[i] = (float)i * 0.5f;
        tbl.vx[i] = 1.0f;
        tbl.vy[i] = -0.5f;
    }

    LARGE_INTEGER t0, t1;
    QueryPerformanceCounter(&t0);
    for (int f = 0; f < FRAMES; ++f) {
        Axon_step_physics(tbl, dt);
    }
    QueryPerformanceCounter(&t1);

    double ms = get_time_ms(t0, t1, freq);
    double tp = ((double)N * FRAMES / 1e6) / (ms / 1000.0);
    printf("  [Axon Native SoA]    Total: %8.2f ms | %6.3f ms/f | Throughput: %8.2f M/s | Chk: %.2f\n\n",
           ms, ms / FRAMES, tp, tbl.x[0]);

    Axon_Table_Particle_free(&tbl);
}

int main(int argc, char** argv) {
    LARGE_INTEGER freq;
    QueryPerformanceFrequency(&freq);

    printf("=== AXON NATIVE SoA PARTICLE SIMULATION BENCHMARK ===\n\n");
    bench_axon_scale(100000, 100, 0.016f, freq);
    bench_axon_scale(1000000, 100, 0.016f, freq);
    bench_axon_scale(10000000, 100, 0.016f, freq);
    return 0;
}
