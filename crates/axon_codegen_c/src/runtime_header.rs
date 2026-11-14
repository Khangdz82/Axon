//! C23 Runtime Header for Axon Transpiled Code
//!
//! Provides 64-byte cache-line aligned allocations for Native SoA,
//! SIMD auto-vectorization pragmas, checked/wrapping/saturating arithmetic,
//! and thread/job primitives.

pub const RUNTIME_HEADER: &str = r#"
#ifndef AXON_RUNTIME_H
#define AXON_RUNTIME_H

#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <assert.h>
#include <string.h>

#if defined(__STDC_VERSION__) && __STDC_VERSION__ >= 202311L
  #include <stdalign.h>
#elif defined(_MSC_VER)
  #define alignas(x) __declspec(align(x))
#else
  #include <stdalign.h>
#endif

// Restrict pointer for SIMD aliasing guarantees
#if defined(_MSC_VER)
  #define AXON_RESTRICT __restrict
#elif defined(__GNUC__) || defined(__clang__)
  #define AXON_RESTRICT __restrict__
#else
  #define AXON_RESTRICT
#endif

// SIMD Vectorization Pragmas
#if defined(__clang__)
  #define AXON_SIMD_LOOP _Pragma("clang loop vectorize(enable)") _Pragma("clang loop interleave(enable)")
#elif defined(__GNUC__)
  #define AXON_SIMD_LOOP _Pragma("GCC ivdep")
#elif defined(_MSC_VER)
  #define AXON_SIMD_LOOP __pragma(loop(ivdep))
#else
  #define AXON_SIMD_LOOP
#endif

// 64-Byte Cache-Line Aligned Memory Allocation
static inline void* axon_aligned_alloc(size_t alignment, size_t size) {
    if (size == 0) return NULL;
#if defined(_MSC_VER)
    void* ptr = _aligned_malloc(size, alignment);
    if (!ptr) {
        fprintf(stderr, "Axon Runtime Error: out of memory\n");
        exit(1);
    }
    return ptr;
#else
    // aligned_alloc requires size to be multiple of alignment
    size_t rounded_size = (size + alignment - 1) & ~(alignment - 1);
    void* ptr = aligned_alloc(alignment, rounded_size);
    if (!ptr) {
        fprintf(stderr, "Axon Runtime Error: out of memory\n");
        exit(1);
    }
    return ptr;
#endif
}

static inline void axon_aligned_free(void* ptr) {
    if (!ptr) return;
#if defined(_MSC_VER)
    _aligned_free(ptr);
#else
    free(ptr);
#endif
}

// Checked Arithmetic (+, -, *)
static inline int32_t axon_checked_add_i32(int32_t a, int32_t b) {
    int64_t res = (int64_t)a + (int64_t)b;
    assert(res >= INT32_MIN && res <= INT32_MAX && "Integer overflow in '+'");
    return (int32_t)res;
}

static inline int32_t axon_checked_sub_i32(int32_t a, int32_t b) {
    int64_t res = (int64_t)a - (int64_t)b;
    assert(res >= INT32_MIN && res <= INT32_MAX && "Integer overflow in '-'");
    return (int32_t)res;
}

static inline int32_t axon_checked_mul_i32(int32_t a, int32_t b) {
    int64_t res = (int64_t)a * (int64_t)b;
    assert(res >= INT32_MIN && res <= INT32_MAX && "Integer overflow in '*'");
    return (int32_t)res;
}

static inline int64_t axon_checked_add_i64(int64_t a, int64_t b) {
#if defined(__GNUC__) || defined(__clang__)
    int64_t res;
    assert(!__builtin_add_overflow(a, b, &res) && "Integer overflow in '+'");
    return res;
#else
    if ((b > 0 && a > INT64_MAX - b) || (b < 0 && a < INT64_MIN - b)) {
        assert(false && "Integer overflow in '+'");
    }
    return a + b;
#endif
}

// Wrapping Arithmetic (+%, -%, *%)
static inline int32_t axon_wrap_add_i32(int32_t a, int32_t b) {
    return (int32_t)((uint32_t)a + (uint32_t)b);
}

static inline int32_t axon_wrap_sub_i32(int32_t a, int32_t b) {
    return (int32_t)((uint32_t)a - (uint32_t)b);
}

static inline int32_t axon_wrap_mul_i32(int32_t a, int32_t b) {
    return (int32_t)((uint32_t)a * (uint32_t)b);
}

// Saturating Arithmetic (+!, -!, *!)
static inline int32_t axon_sat_add_i32(int32_t a, int32_t b) {
    int64_t res = (int64_t)a + (int64_t)b;
    if (res > INT32_MAX) return INT32_MAX;
    if (res < INT32_MIN) return INT32_MIN;
    return (int32_t)res;
}

static inline int32_t axon_sat_sub_i32(int32_t a, int32_t b) {
    int64_t res = (int64_t)a - (int64_t)b;
    if (res > INT32_MAX) return INT32_MAX;
    if (res < INT32_MIN) return INT32_MIN;
    return (int32_t)res;
}

static inline int32_t axon_sat_mul_i32(int32_t a, int32_t b) {
    int64_t res = (int64_t)a * (int64_t)b;
    if (res > INT32_MAX) return INT32_MAX;
    if (res < INT32_MIN) return INT32_MIN;
    return (int32_t)res;
}

// Minimal Fiber/Job Handle for MVP Concurrency
typedef struct Axon_Job {
    void* value;
} Axon_Job;

static inline Axon_Job axon_spawn_job(void* val) {
    Axon_Job job;
    job.value = val;
    return job;
}

static inline void* axon_join_job(Axon_Job job) {
    return job.value;
}

// Primitive Table Definitions
typedef struct Axon_Table_i64 {
    size_t len;
    size_t capacity;
    int64_t* AXON_RESTRICT data;
} Axon_Table_i64;

typedef struct Axon_Table_i32 {
    size_t len;
    size_t capacity;
    int32_t* AXON_RESTRICT data;
} Axon_Table_i32;

typedef struct Axon_Table_f32 {
    size_t len;
    size_t capacity;
    float* AXON_RESTRICT data;
} Axon_Table_f32;

typedef struct Axon_Table_f64 {
    size_t len;
    size_t capacity;
    double* AXON_RESTRICT data;
} Axon_Table_f64;

typedef struct Axon_Table_usize {
    size_t len;
    size_t capacity;
    size_t* AXON_RESTRICT data;
} Axon_Table_usize;

#endif // AXON_RUNTIME_H
"#;
