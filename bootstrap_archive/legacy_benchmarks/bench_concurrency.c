#define _CRT_SECURE_NO_WARNINGS
#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <windows.h>
#include <process.h>

#define N 500000 // 500K particles (fits in 12MB L3 Cache)
#define FRAMES 500
#define DT 0.016f

typedef struct {
    float* __restrict x;
    float* __restrict y;
    float* __restrict vx;
    float* __restrict vy;
} Table_Particle;

static inline Table_Particle alloc_table(size_t cap) {
    Table_Particle tbl;
    tbl.x = (float*)_aligned_malloc(cap * sizeof(float), 64);
    tbl.y = (float*)_aligned_malloc(cap * sizeof(float), 64);
    tbl.vx = (float*)_aligned_malloc(cap * sizeof(float), 64);
    tbl.vy = (float*)_aligned_malloc(cap * sizeof(float), 64);
    for (size_t i = 0; i < cap; ++i) {
        tbl.x[i] = (float)i;
        tbl.y[i] = (float)i * 0.5f;
        tbl.vx[i] = 1.0f;
        tbl.vy[i] = -0.5f;
    }
    return tbl;
}

static inline void free_table(Table_Particle* tbl) {
    _aligned_free(tbl->x);
    _aligned_free(tbl->y);
    _aligned_free(tbl->vx);
    _aligned_free(tbl->vy);
}

void step_range(Table_Particle* __restrict p, size_t start, size_t end, float dt) {
    #pragma loop(ivdep)
    for (size_t i = start; i < end; ++i) {
        p->x[i] += p->vx[i] * dt;
        p->y[i] += p->vy[i] * dt;
    }
}

// 1. Single Thread
double bench_single_thread(Table_Particle* p, LARGE_INTEGER freq) {
    LARGE_INTEGER t0, t1;
    QueryPerformanceCounter(&t0);
    for (int f = 0; f < FRAMES; ++f) {
        step_range(p, 0, N, DT);
    }
    QueryPerformanceCounter(&t1);
    return (double)(t1.QuadPart - t0.QuadPart) * 1000.0 / (double)freq.QuadPart;
}

// 2. OS Thread Pool / Multi-thread
typedef struct {
    Table_Particle* p;
    size_t start;
    size_t end;
    float dt;
    HANDLE done_event;
} ThreadChunk;

unsigned __stdcall thread_worker(void* arg) {
    ThreadChunk* c = (ThreadChunk*)arg;
    step_range(c->p, c->start, c->end, c->dt);
    SetEvent(c->done_event);
    return 0;
}

double bench_os_threads(Table_Particle* p, int num_threads, LARGE_INTEGER freq) {
    ThreadChunk* chunks = (ThreadChunk*)malloc(num_threads * sizeof(ThreadChunk));
    HANDLE* events = (HANDLE*)malloc(num_threads * sizeof(HANDLE));
    size_t chunk_size = N / num_threads;

    for (int t = 0; t < num_threads; ++t) {
        events[t] = CreateEvent(NULL, FALSE, FALSE, NULL);
        chunks[t].p = p;
        chunks[t].start = t * chunk_size;
        chunks[t].end = (t == num_threads - 1) ? N : (t + 1) * chunk_size;
        chunks[t].dt = DT;
        chunks[t].done_event = events[t];
    }

    LARGE_INTEGER t0, t1;
    QueryPerformanceCounter(&t0);

    for (int f = 0; f < FRAMES; ++f) {
        for (int t = 0; t < num_threads; ++t) {
            HANDLE th = (HANDLE)_beginthreadex(NULL, 0, thread_worker, &chunks[t], 0, NULL);
            CloseHandle(th);
        }
        WaitForMultipleObjects(num_threads, events, TRUE, INFINITE);
    }

    QueryPerformanceCounter(&t1);

    for (int t = 0; t < num_threads; ++t) {
        CloseHandle(events[t]);
    }
    free(events);
    free(chunks);
    return (double)(t1.QuadPart - t0.QuadPart) * 1000.0 / (double)freq.QuadPart;
}

// 3. Axon Chase-Lev Work-Stealing Fiber Scheduler (Persistent Pool)
#define main Axon_main_concur
#include "multi_fiber_concurrency.c"
#undef main

typedef struct {
    Table_Particle* p;
    size_t start;
    size_t end;
    float dt;
} FiberChunk;

void* axon_fiber_task(void* raw_ctx) {
    FiberChunk* c = (FiberChunk*)raw_ctx;
    step_range(c->p, c->start, c->end, c->dt);
    return NULL;
}

double bench_axon_fibers(Table_Particle* p, int num_fibers, LARGE_INTEGER freq) {
    FiberChunk* chunks = (FiberChunk*)malloc(num_fibers * sizeof(FiberChunk));
    size_t chunk_size = N / num_fibers;

    for (int t = 0; t < num_fibers; ++t) {
        chunks[t].p = p;
        chunks[t].start = t * chunk_size;
        chunks[t].end = (t == num_fibers - 1) ? N : (t + 1) * chunk_size;
        chunks[t].dt = DT;
    }

    Axon_Job** jobs = (Axon_Job**)malloc(num_fibers * sizeof(Axon_Job*));

    LARGE_INTEGER t0, t1;
    QueryPerformanceCounter(&t0);

    for (int f = 0; f < FRAMES; ++f) {
        for (int t = 0; t < num_fibers; ++t) {
            jobs[t] = axon_spawn(axon_fiber_task, &chunks[t]);
        }
        for (int t = 0; t < num_fibers; ++t) {
            axon_join(jobs[t]);
        }
    }

    QueryPerformanceCounter(&t1);

    free(jobs);
    free(chunks);
    return (double)(t1.QuadPart - t0.QuadPart) * 1000.0 / (double)freq.QuadPart;
}

int main(void) {
    LARGE_INTEGER freq;
    QueryPerformanceFrequency(&freq);

    axon_scheduler_init(0); // auto-detect hardware concurrency (12 threads)

    Table_Particle tbl = alloc_table(N);

    printf("=== CONCURRENCY BENCHMARK: AXON WORK-STEALING FIBERS VS OS THREADS ===\n");
    printf("Particles: %zu, Frames: %d, Total updates: %zu M\n\n",
           (size_t)N, FRAMES, (size_t)N * FRAMES / 1000000);

    // Single Thread
    double st_ms = bench_single_thread(&tbl, freq);
    printf("  [Single Thread Baseline]    Total: %8.2f ms | Speedup: 1.00x | Throughput: %8.2f M/s\n",
           st_ms, ((double)N * FRAMES / 1e6) / (st_ms / 1000.0));

    // OS Threads
    for (int threads = 2; threads <= 6; threads += 2) {
        double os_ms = bench_os_threads(&tbl, threads, freq);
        printf("  [OS Threads (spawn %d)]      Total: %8.2f ms | Speedup: %4.2fx | Throughput: %8.2f M/s\n",
               threads, os_ms, st_ms / os_ms, ((double)N * FRAMES / 1e6) / (os_ms / 1000.0));
    }

    // Axon Chase-Lev Work-Stealing Fibers
    for (int fibers = 2; fibers <= 12; fibers += (fibers == 6 ? 6 : 2)) {
        double fiber_ms = bench_axon_fibers(&tbl, fibers, freq);
        printf("  [Axon Fibers (%2d workers)]   Total: %8.2f ms | Speedup: %4.2fx | Throughput: %8.2f M/s\n",
               fibers, fiber_ms, st_ms / fiber_ms, ((double)N * FRAMES / 1e6) / (fiber_ms / 1000.0));
    }

    free_table(&tbl);
    axon_scheduler_shutdown();
    return 0;
}
