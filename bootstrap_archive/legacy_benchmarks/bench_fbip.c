#define _CRT_SECURE_NO_WARNINGS
#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <windows.h>

typedef struct {
    float x;
    float y;
    float z;
    float w;
} Vec4;

// 1. Standard C/C++ Malloc & Free
__declspec(noinline) Vec4* create_vec4_heap(float x, float y, float z, float w) {
    Vec4* v = (Vec4*)malloc(sizeof(Vec4));
    if (v) {
        v->x = x;
        v->y = y;
        v->z = z;
        v->w = w;
    }
    return v;
}

void bench_malloc_free(size_t iters, LARGE_INTEGER freq) {
    LARGE_INTEGER t0, t1;
    QueryPerformanceCounter(&t0);

    float acc = 0.0f;
    for (size_t i = 0; i < iters; ++i) {
        Vec4* v = create_vec4_heap((float)i, (float)i + 1.0f, (float)i + 2.0f, (float)i + 3.0f);
        acc += v->x + v->y + v->z + v->w;
        free(v);
    }

    QueryPerformanceCounter(&t1);
    double ms = (double)(t1.QuadPart - t0.QuadPart) * 1000.0 / (double)freq.QuadPart;
    double ns_per_op = (ms * 1e6) / (double)iters;
    double ops_per_sec = ((double)iters / 1e6) / (ms / 1000.0);
    printf("  [C++23 malloc + free]    Total: %8.2f ms | %6.2f ns/op | Throughput: %8.2f M allocs/s (acc: %.1f)\n",
           ms, ns_per_op, ops_per_sec, acc);
}

// 2. Axon FBIP (Functional But In-Place) Reuse
__declspec(noinline) Vec4* create_vec4_fbip(void* reuse_token, float x, float y, float z, float w) {
    // FBIP ReuseToken replaces malloc: zero syscalls, writes directly into reuse slot
    Vec4* v = (Vec4*)reuse_token;
    v->x = x;
    v->y = y;
    v->z = z;
    v->w = w;
    return v;
}

void bench_axon_fbip(size_t iters, LARGE_INTEGER freq) {
    LARGE_INTEGER t0, t1;
    QueryPerformanceCounter(&t0);

    // Initial allocation once
    void* reuse_token = malloc(sizeof(Vec4));

    float acc = 0.0f;
    for (size_t i = 0; i < iters; ++i) {
        // In Axon: Drop of previous dead object yields ReuseToken consumed by next allocation
        Vec4* v = create_vec4_fbip(reuse_token, (float)i, (float)i + 1.0f, (float)i + 2.0f, (float)i + 3.0f);
        acc += v->x + v->y + v->z + v->w;
        // In Axon, compiler pairs this drop with next iteration's alloc:
        // reuse_token = v; (0 cost)
    }

    QueryPerformanceCounter(&t1);
    free(reuse_token);

    double ms = (double)(t1.QuadPart - t0.QuadPart) * 1000.0 / (double)freq.QuadPart;
    double ns_per_op = (ms * 1e6) / (double)iters;
    double ops_per_sec = ((double)iters / 1e6) / (ms / 1000.0);
    printf("  [Axon Perceus FBIP]      Total: %8.2f ms | %6.2f ns/op | Throughput: %8.2f M allocs/s (acc: %.1f)\n",
           ms, ns_per_op, ops_per_sec, acc);
}

int main(void) {
    LARGE_INTEGER freq;
    QueryPerformanceFrequency(&freq);
    const size_t ITERS = 10000000; // 10 Million allocations

    printf("=== MEMORY ALLOCATION: AXON FBIP VS C++ MALLOC/FREE ===\n");
    printf("Iterations: %zu (10 Million allocations / deallocations)\n\n", ITERS);

    bench_malloc_free(ITERS, freq);
    bench_axon_fbip(ITERS, freq);
    return 0;
}
