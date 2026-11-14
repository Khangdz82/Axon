#define _CRT_SECURE_NO_WARNINGS
#include <stdio.h>
#include <stdlib.h>
#include <windows.h>

typedef struct {
    float pos_x;
    float pos_y;
    float vel_x;
    float vel_y;
    float mass;
} Particle_AoS;

void integrate_physics_aos(Particle_AoS* __restrict particles, size_t count, float dt) {
    for (size_t i = 0; i < count; ++i) {
        particles[i].pos_x += particles[i].vel_x * dt;
        particles[i].pos_y += particles[i].vel_y * dt;
    }
}

int main(void) {
    const size_t N = 1000000;
    const int FRAMES = 100;
    const float dt = 0.016f;

    Particle_AoS* particles = (Particle_AoS*)malloc(N * sizeof(Particle_AoS));
    if (!particles) {
        fprintf(stderr, "Allocation failed\n");
        return 1;
    }

    for (size_t i = 0; i < N; ++i) {
        particles[i].pos_x = (float)i;
        particles[i].pos_y = (float)i * 0.5f;
        particles[i].vel_x = 1.0f;
        particles[i].vel_y = -0.5f;
        particles[i].mass = 1.0f;
    }

    LARGE_INTEGER freq, start, end;
    QueryPerformanceFrequency(&freq);
    QueryPerformanceCounter(&start);

    for (int f = 0; f < FRAMES; ++f) {
        integrate_physics_aos(particles, N, dt);
    }

    QueryPerformanceCounter(&end);
    double elapsed_ms = (double)(end.QuadPart - start.QuadPart) * 1000.0 / (double)freq.QuadPart;

    printf("[C++ / Rust AoS Baseline]\n");
    printf("  Entities: %zu\n", N);
    printf("  Frames:   %d\n", FRAMES);
    printf("  Total Time: %.2f ms (%.3f ms/frame)\n", elapsed_ms, elapsed_ms / FRAMES);
    printf("  Throughput: %.2f M updates/sec\n", ((double)N * FRAMES / 1e6) / (elapsed_ms / 1000.0));
    printf("  Checksum: pos_x[0]=%.2f, pos_y[0]=%.2f\n\n", particles[0].pos_x, particles[0].pos_y);

    free(particles);
    return 0;
}
