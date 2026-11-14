#define _CRT_SECURE_NO_WARNINGS
#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <windows.h>
#include <immintrin.h>

// 1. AoS (Array of Structs)
typedef struct {
    float x;
    float y;
    float vx;
    float vy;
} Particle_AoS;

void integrate_aos(Particle_AoS* __restrict p, size_t count, float dt) {
    for (size_t i = 0; i < count; ++i) {
        p[i].x += p[i].vx * dt;
        p[i].y += p[i].vy * dt;
    }
}

// 2. SoA (Structure of Arrays - standard heap)
typedef struct {
    float* __restrict x;
    float* __restrict y;
    float* __restrict vx;
    float* __restrict vy;
} Particle_SoA;

void integrate_soa(Particle_SoA* __restrict p, size_t count, float dt) {
    #pragma loop(ivdep)
    for (size_t i = 0; i < count; ++i) {
        p->x[i] += p->vx[i] * dt;
        p->y[i] += p->vy[i] * dt;
    }
}

// 3. SoA AVX2 Intrinsics (8 floats per register, 64-byte aligned)
void integrate_soa_avx2(Particle_SoA* __restrict p, size_t count, float dt) {
    __m256 vdt = _mm256_set1_ps(dt);
    size_t i = 0;
    size_t vec_count = count - (count % 8);
    for (; i < vec_count; i += 8) {
        __m256 x = _mm256_loadu_ps(&p->x[i]);
        __m256 vx = _mm256_loadu_ps(&p->vx[i]);
        __m256 y = _mm256_loadu_ps(&p->y[i]);
        __m256 vy = _mm256_loadu_ps(&p->vy[i]);

        x = _mm256_fmadd_ps(vx, vdt, x);
        y = _mm256_fmadd_ps(vy, vdt, y);

        _mm256_storeu_ps(&p->x[i], x);
        _mm256_storeu_ps(&p->y[i], y);
    }
    for (; i < count; ++i) {
        p->x[i] += p->vx[i] * dt;
        p->y[i] += p->vy[i] * dt;
    }
}

static inline double get_time_ms(LARGE_INTEGER start, LARGE_INTEGER end, LARGE_INTEGER freq) {
    return (double)(end.QuadPart - start.QuadPart) * 1000.0 / (double)freq.QuadPart;
}

void run_bench(size_t N, int FRAMES, float dt, LARGE_INTEGER freq) {
    printf("--- Scale: N = %zu particles, %d frames ---\n", N, FRAMES);

    // 1. Benchmark AoS
    Particle_AoS* aos = (Particle_AoS*)malloc(N * sizeof(Particle_AoS));
    for (size_t i = 0; i < N; ++i) {
        aos[i].x = (float)i;
        aos[i].y = (float)i * 0.5f;
        aos[i].vx = 1.0f;
        aos[i].vy = -0.5f;
    }
    LARGE_INTEGER t0, t1;
    QueryPerformanceCounter(&t0);
    for (int f = 0; f < FRAMES; ++f) {
        integrate_aos(aos, N, dt);
    }
    QueryPerformanceCounter(&t1);
    double aos_ms = get_time_ms(t0, t1, freq);
    double aos_tp = ((double)N * FRAMES / 1e6) / (aos_ms / 1000.0);
    printf("  [C++23 AoS]          Total: %8.2f ms | %6.3f ms/f | Throughput: %8.2f M/s | Chk: %.2f\n",
           aos_ms, aos_ms / FRAMES, aos_tp, aos[0].x);
    free(aos);

    // 2. Benchmark SoA
    Particle_SoA soa;
    soa.x = (float*)_aligned_malloc(N * sizeof(float), 64);
    soa.y = (float*)_aligned_malloc(N * sizeof(float), 64);
    soa.vx = (float*)_aligned_malloc(N * sizeof(float), 64);
    soa.vy = (float*)_aligned_malloc(N * sizeof(float), 64);
    for (size_t i = 0; i < N; ++i) {
        soa.x[i] = (float)i;
        soa.y[i] = (float)i * 0.5f;
        soa.vx[i] = 1.0f;
        soa.vy[i] = -0.5f;
    }
    QueryPerformanceCounter(&t0);
    for (int f = 0; f < FRAMES; ++f) {
        integrate_soa(&soa, N, dt);
    }
    QueryPerformanceCounter(&t1);
    double soa_ms = get_time_ms(t0, t1, freq);
    double soa_tp = ((double)N * FRAMES / 1e6) / (soa_ms / 1000.0);
    printf("  [C++23 SoA (Auto)]   Total: %8.2f ms | %6.3f ms/f | Throughput: %8.2f M/s | Chk: %.2f\n",
           soa_ms, soa_ms / FRAMES, soa_tp, soa.x[0]);

    // 3. Benchmark SoA AVX2 Intrinsics
    QueryPerformanceCounter(&t0);
    for (int f = 0; f < FRAMES; ++f) {
        integrate_soa_avx2(&soa, N, dt);
    }
    QueryPerformanceCounter(&t1);
    double avx_ms = get_time_ms(t0, t1, freq);
    double avx_tp = ((double)N * FRAMES / 1e6) / (avx_ms / 1000.0);
    printf("  [C++23 SoA (AVX2)]   Total: %8.2f ms | %6.3f ms/f | Throughput: %8.2f M/s | Chk: %.2f\n",
           avx_ms, avx_ms / FRAMES, avx_tp, soa.x[0]);

    _aligned_free(soa.x);
    _aligned_free(soa.y);
    _aligned_free(soa.vx);
    _aligned_free(soa.vy);
    printf("\n");
}

int main(int argc, char** argv) {
    LARGE_INTEGER freq;
    QueryPerformanceFrequency(&freq);

    printf("=== C++23 PARTICLE SIMULATION BENCHMARK ===\n\n");
    run_bench(100000, 100, 0.016f, freq);
    run_bench(1000000, 100, 0.016f, freq);
    run_bench(10000000, 100, 0.016f, freq);
    return 0;
}
