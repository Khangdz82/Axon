#ifndef AETHER_RUNTIME_H
#define AETHER_RUNTIME_H

#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <stdbool.h>
#include <string.h>

#if defined(_WIN32) || defined(_WIN64)
    #include <windows.h>
    #include <malloc.h>
    #define AETHER_ALIGN_ALLOC(size, align) _aligned_malloc((size), (align))
    #define AETHER_ALIGN_FREE(ptr) _aligned_free(ptr)
    #define AETHER_RESTRICT __restrict
    #define AETHER_SIMD_LOOP __pragma(loop(ivdep))
#else
    #include <stdlib.h>
    #include <pthread.h>
    #include <time.h>
    static inline void* aether_posix_aligned_alloc(size_t size, size_t align) {
        void* ptr = NULL;
        if (posix_memalign(&ptr, align, size) != 0) return NULL;
        return ptr;
    }
    #define AETHER_ALIGN_ALLOC(size, align) aether_posix_aligned_alloc((size), (align))
    #define AETHER_ALIGN_FREE(ptr) free(ptr)
    #define AETHER_RESTRICT __restrict__
    #define AETHER_SIMD_LOOP _Pragma("clang loop vectorize(enable)")
#endif

// 64-byte Cache-Line Alignment for AVX-512 / AVX2 / L1 Cache
#define AETHER_CACHE_LINE 64

// High-resolution timer for nanosecond-level benchmarking
static inline double aether_get_time_sec(void) {
#if defined(_WIN32) || defined(_WIN64)
    static LARGE_INTEGER freq;
    static int initialized = 0;
    if (!initialized) {
        QueryPerformanceFrequency(&freq);
        initialized = 1;
    }
    LARGE_INTEGER now;
    QueryPerformanceCounter(&now);
    return (double)now.QuadPart / (double)freq.QuadPart;
#else
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (double)ts.tv_sec + (double)ts.tv_nsec * 1e-9;
#endif
}

// Colorless Concurrency: Lightweight Fiber / Worker Task Handle
typedef struct {
#if defined(_WIN32) || defined(_WIN64)
    HANDLE thread_handle;
#else
    pthread_t thread;
#endif
    void* (*task_fn)(void*);
    void* arg;
    void* result;
} Aether_Worker;

#if defined(_WIN32) || defined(_WIN64)
static DWORD WINAPI aether_worker_thunk(LPVOID param) {
    Aether_Worker* w = (Aether_Worker*)param;
    w->result = w->task_fn(w->arg);
    return 0;
}
#else
static void* aether_worker_thunk_posix(void* param) {
    Aether_Worker* w = (Aether_Worker*)param;
    w->result = w->task_fn(w->arg);
    return NULL;
}
#endif

static inline Aether_Worker* aether_spawn(void* (*fn)(void*), void* arg) {
    Aether_Worker* w = (Aether_Worker*)malloc(sizeof(Aether_Worker));
    if (!w) return NULL;
    w->task_fn = fn;
    w->arg = arg;
    w->result = NULL;
#if defined(_WIN32) || defined(_WIN64)
    w->thread_handle = CreateThread(NULL, 0, aether_worker_thunk, w, 0, NULL);
#else
    pthread_create(&w->thread, NULL, aether_worker_thunk_posix, w);
#endif
    return w;
}

static inline void* aether_join(Aether_Worker* w) {
    if (!w) return NULL;
#if defined(_WIN32) || defined(_WIN64)
    WaitForSingleObject(w->thread_handle, INFINITE);
    CloseHandle(w->thread_handle);
#else
    pthread_join(w->thread, NULL);
#endif
    void* res = w->result;
    free(w);
    return res;
}

// Zero-UB Arithmetic Handlers
static inline int32_t aether_add_checked_i32(int32_t a, int32_t b) {
#if defined(__GNUC__) || defined(__clang__)
    int32_t res;
    if (__builtin_add_overflow(a, b, &res)) {
        fprintf(stderr, "Aether Panic: integer overflow in addition!\n");
        exit(1);
    }
    return res;
#else
    if ((b > 0 && a > 2147483647 - b) || (b < 0 && a < (-2147483647 - 1) - b)) {
        fprintf(stderr, "Aether Panic: integer overflow in addition!\n");
        exit(1);
    }
    return a + b;
#endif
}

static inline int32_t aether_add_wrap_i32(int32_t a, int32_t b) {
    return (int32_t)((uint32_t)a + (uint32_t)b);
}

static inline int32_t aether_add_sat_i32(int32_t a, int32_t b) {
    int64_t res = (int64_t)a + (int64_t)b;
    if (res > INT32_MAX) return INT32_MAX;
    if (res < INT32_MIN) return INT32_MIN;
    return (int32_t)res;
}

#endif // AETHER_RUNTIME_H
