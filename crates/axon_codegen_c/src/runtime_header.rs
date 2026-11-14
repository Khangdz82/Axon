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

#if defined(_WIN32) || defined(_WIN64)
  #include <windows.h>
  #include <process.h>
  #include <malloc.h>
  #define AXON_THREAD_LOCAL __declspec(thread)
  #define AXON_FENCE() MemoryBarrier()
  #define AXON_PAUSE() YieldProcessor()
  #define AXON_YIELD() SwitchToThread()
  typedef volatile LONG axon_atomic_int_t;
  typedef volatile PVOID axon_atomic_ptr_t;
  #define AXON_CAS_INT(ptr, old_v, new_v) (InterlockedCompareExchange((volatile LONG*)(ptr), (LONG)(new_v), (LONG)(old_v)) == (LONG)(old_v))
  #define AXON_INC_INT(ptr) InterlockedIncrement((volatile LONG*)(ptr))
  #define AXON_DEC_INT(ptr) InterlockedDecrement((volatile LONG*)(ptr))
  #define AXON_EXCHANGE_INT(ptr, new_v) InterlockedExchange((volatile LONG*)(ptr), (LONG)(new_v))
#else
  #include <pthread.h>
  #include <sched.h>
  #include <stdatomic.h>
  #include <unistd.h>
  #define AXON_THREAD_LOCAL _Thread_local
  #define AXON_FENCE() atomic_thread_fence(memory_order_seq_cst)
  #define AXON_PAUSE() __builtin_ia32_pause()
  #define AXON_YIELD() sched_yield()
  typedef atomic_int axon_atomic_int_t;
  typedef atomic_uintptr_t axon_atomic_ptr_t;
  #define AXON_CAS_INT(ptr, old_v, new_v) atomic_compare_exchange_strong((ptr), &(old_v), (new_v))
  #define AXON_INC_INT(ptr) atomic_fetch_add((ptr), 1)
  #define AXON_DEC_INT(ptr) atomic_fetch_sub((ptr), 1)
  #define AXON_EXCHANGE_INT(ptr, new_v) atomic_exchange((ptr), (new_v))
#endif

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

// === COLORLESS CONCURRENCY: WORK-STEALING FIBER RUNTIME ===
#define AXON_DEQUE_CAP 2048
#define AXON_DEQUE_MASK (AXON_DEQUE_CAP - 1)
#define AXON_MAX_WORKERS 32

typedef struct Axon_Job {
    void* (*fn)(void*);
    void* arg;
    void* result;
    axon_atomic_int_t status; // 0=PENDING, 1=RUNNING, 2=COMPLETED
#if defined(_WIN32) || defined(_WIN64)
    HANDLE event;
#else
    pthread_mutex_t mutex;
    pthread_cond_t cond;
#endif
} Axon_Job;

typedef struct Axon_Worker {
    int id;
#if defined(_WIN32) || defined(_WIN64)
    HANDLE thread;
#else
    pthread_t thread;
#endif
    axon_atomic_int_t top;
    axon_atomic_int_t bottom;
    Axon_Job* deque[AXON_DEQUE_CAP];
    uint32_t rng;
} Axon_Worker;

typedef struct Axon_Scheduler {
    int worker_count;
    Axon_Worker workers[AXON_MAX_WORKERS];
    axon_atomic_int_t running;
    axon_atomic_int_t active_jobs;
#if defined(_WIN32) || defined(_WIN64)
    CRITICAL_SECTION injector_lock;
#else
    pthread_mutex_t injector_lock;
#endif
    Axon_Job* injector_queue[AXON_DEQUE_CAP];
    int injector_head;
    int injector_tail;
    int injector_count;
} Axon_Scheduler;

static Axon_Scheduler g_axon_sched;
static AXON_THREAD_LOCAL int g_axon_worker_id = -1;

static inline uint32_t axon_xorshift32(uint32_t* state) {
    uint32_t x = *state;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    *state = x;
    return x;
}

static inline void axon_worker_push(Axon_Worker* w, Axon_Job* job) {
    long b = (long)w->bottom;
    w->deque[b & AXON_DEQUE_MASK] = job;
    AXON_FENCE();
    w->bottom = b + 1;
}

static inline Axon_Job* axon_worker_pop(Axon_Worker* w) {
    long b = (long)w->bottom - 1;
    w->bottom = b;
    AXON_FENCE();
    long t = (long)w->top;
    if (t <= b) {
        Axon_Job* job = w->deque[b & AXON_DEQUE_MASK];
        if (t == b) {
            if (!AXON_CAS_INT(&w->top, t, t + 1)) {
                job = NULL;
            }
            w->bottom = t + 1;
        }
        return job;
    } else {
        w->bottom = t;
        return NULL;
    }
}

static inline Axon_Job* axon_worker_steal(Axon_Worker* w) {
    long t = (long)w->top;
    AXON_FENCE();
    long b = (long)w->bottom;
    if (t < b) {
        Axon_Job* job = w->deque[t & AXON_DEQUE_MASK];
        if (AXON_CAS_INT(&w->top, t, t + 1)) {
            return job;
        }
    }
    return NULL;
}

static inline void axon_injector_push(Axon_Job* job) {
#if defined(_WIN32) || defined(_WIN64)
    EnterCriticalSection(&g_axon_sched.injector_lock);
#else
    pthread_mutex_lock(&g_axon_sched.injector_lock);
#endif
    if (g_axon_sched.injector_count < AXON_DEQUE_CAP) {
        g_axon_sched.injector_queue[g_axon_sched.injector_tail] = job;
        g_axon_sched.injector_tail = (g_axon_sched.injector_tail + 1) & AXON_DEQUE_MASK;
        g_axon_sched.injector_count++;
    }
#if defined(_WIN32) || defined(_WIN64)
    LeaveCriticalSection(&g_axon_sched.injector_lock);
#else
    pthread_mutex_unlock(&g_axon_sched.injector_lock);
#endif
}

static inline Axon_Job* axon_injector_pop(void) {
    Axon_Job* job = NULL;
#if defined(_WIN32) || defined(_WIN64)
    EnterCriticalSection(&g_axon_sched.injector_lock);
#else
    pthread_mutex_lock(&g_axon_sched.injector_lock);
#endif
    if (g_axon_sched.injector_count > 0) {
        job = g_axon_sched.injector_queue[g_axon_sched.injector_head];
        g_axon_sched.injector_head = (g_axon_sched.injector_head + 1) & AXON_DEQUE_MASK;
        g_axon_sched.injector_count--;
    }
#if defined(_WIN32) || defined(_WIN64)
    LeaveCriticalSection(&g_axon_sched.injector_lock);
#else
    pthread_mutex_unlock(&g_axon_sched.injector_lock);
#endif
    return job;
}

static inline Axon_Job* axon_scheduler_get_work(void) {
    if (g_axon_worker_id >= 0 && g_axon_worker_id < g_axon_sched.worker_count) {
        Axon_Worker* self = &g_axon_sched.workers[g_axon_worker_id];
        Axon_Job* job = axon_worker_pop(self);
        if (job) return job;
    }

    if (g_axon_sched.worker_count > 1) {
        uint32_t seed = (g_axon_worker_id >= 0) ? g_axon_sched.workers[g_axon_worker_id].rng : 123456789;
        int victim_id = (int)(axon_xorshift32(&seed) % (uint32_t)g_axon_sched.worker_count);
        if (g_axon_worker_id >= 0) g_axon_sched.workers[g_axon_worker_id].rng = seed;
        if (victim_id != g_axon_worker_id) {
            Axon_Job* job = axon_worker_steal(&g_axon_sched.workers[victim_id]);
            if (job) return job;
        }
    }

    return axon_injector_pop();
}

static inline void axon_execute_job(Axon_Job* job) {
    if (AXON_CAS_INT(&job->status, 0, 1)) {
        job->result = job->fn(job->arg);
        AXON_EXCHANGE_INT(&job->status, 2);
#if defined(_WIN32) || defined(_WIN64)
        SetEvent(job->event);
#else
        pthread_mutex_lock(&job->mutex);
        pthread_cond_broadcast(&job->cond);
        pthread_mutex_unlock(&job->mutex);
#endif
        AXON_DEC_INT(&g_axon_sched.active_jobs);
    }
}

#if defined(_WIN32) || defined(_WIN64)
static unsigned __stdcall axon_worker_loop(void* arg) {
#else
static void* axon_worker_loop(void* arg) {
#endif
    Axon_Worker* self = (Axon_Worker*)arg;
    g_axon_worker_id = self->id;

    while (g_axon_sched.running) {
        Axon_Job* job = axon_scheduler_get_work();
        if (job) {
            axon_execute_job(job);
        } else {
            AXON_PAUSE();
            AXON_YIELD();
        }
    }
    return 0;
}

static inline void axon_scheduler_init(int worker_count) {
    if (g_axon_sched.running) return;

    if (worker_count <= 0) {
#if defined(_WIN32) || defined(_WIN64)
        SYSTEM_INFO sys_info;
        GetSystemInfo(&sys_info);
        worker_count = (int)sys_info.dwNumberOfProcessors;
#else
        worker_count = (int)sysconf(_SC_NPROCESSORS_ONLN);
#endif
    }
    if (worker_count < 2) worker_count = 2;
    if (worker_count > AXON_MAX_WORKERS) worker_count = AXON_MAX_WORKERS;

    g_axon_sched.worker_count = worker_count;
    g_axon_sched.running = 1;
    g_axon_sched.active_jobs = 0;
#if defined(_WIN32) || defined(_WIN64)
    InitializeCriticalSection(&g_axon_sched.injector_lock);
#else
    pthread_mutex_init(&g_axon_sched.injector_lock, NULL);
#endif
    g_axon_sched.injector_head = 0;
    g_axon_sched.injector_tail = 0;
    g_axon_sched.injector_count = 0;

    for (int i = 0; i < worker_count; ++i) {
        Axon_Worker* w = &g_axon_sched.workers[i];
        w->id = i;
        w->top = 0;
        w->bottom = 0;
        w->rng = (uint32_t)(1337 + i * 997);
#if defined(_WIN32) || defined(_WIN64)
        w->thread = (HANDLE)_beginthreadex(NULL, 0, axon_worker_loop, w, 0, NULL);
#else
        pthread_create(&w->thread, NULL, axon_worker_loop, w);
#endif
    }
}

static inline void axon_scheduler_shutdown(void) {
    if (!g_axon_sched.running) return;
    g_axon_sched.running = 0;
    for (int i = 0; i < g_axon_sched.worker_count; ++i) {
#if defined(_WIN32) || defined(_WIN64)
        WaitForSingleObject(g_axon_sched.workers[i].thread, 2000);
        CloseHandle(g_axon_sched.workers[i].thread);
#else
        pthread_join(g_axon_sched.workers[i].thread, NULL);
#endif
    }
#if defined(_WIN32) || defined(_WIN64)
    DeleteCriticalSection(&g_axon_sched.injector_lock);
#else
    pthread_mutex_destroy(&g_axon_sched.injector_lock);
#endif
}

static inline Axon_Job* axon_spawn(void* (*fn)(void*), void* arg) {
    if (!g_axon_sched.running) {
        axon_scheduler_init(0);
    }
    Axon_Job* job = (Axon_Job*)malloc(sizeof(Axon_Job));
    job->fn = fn;
    job->arg = arg;
    job->result = NULL;
    job->status = 0;
#if defined(_WIN32) || defined(_WIN64)
    job->event = CreateEvent(NULL, TRUE, FALSE, NULL);
#else
    pthread_mutex_init(&job->mutex, NULL);
    pthread_cond_init(&job->cond, NULL);
#endif
    AXON_INC_INT(&g_axon_sched.active_jobs);

    if (g_axon_worker_id >= 0 && g_axon_worker_id < g_axon_sched.worker_count) {
        axon_worker_push(&g_axon_sched.workers[g_axon_worker_id], job);
    } else {
        axon_injector_push(job);
    }
    return job;
}

static inline void* axon_join(Axon_Job* job) {
    if (!job) return NULL;
    while (job->status != 2) {
        Axon_Job* other = axon_scheduler_get_work();
        if (other) {
            axon_execute_job(other);
        } else {
            AXON_PAUSE();
            AXON_YIELD();
        }
    }
    void* res = job->result;
#if defined(_WIN32) || defined(_WIN64)
    CloseHandle(job->event);
#else
    pthread_mutex_destroy(&job->mutex);
    pthread_cond_destroy(&job->cond);
#endif
    free(job);
    return res;
}

// Backward compatibility stubs
static inline Axon_Job axon_spawn_job(void* val) {
    Axon_Job j;
    j.result = val;
    return j;
}

static inline void* axon_join_job(Axon_Job job) {
    return job.result;
}

// Linear Isolation Channel
#define AXON_CHANNEL_CAP 1024
#define AXON_CHANNEL_MASK (AXON_CHANNEL_CAP - 1)

typedef struct Axon_Channel {
    axon_atomic_int_t head;
    axon_atomic_int_t tail;
    void* buffer[AXON_CHANNEL_CAP];
} Axon_Channel;

static inline Axon_Channel* axon_channel_create(void) {
    Axon_Channel* ch = (Axon_Channel*)malloc(sizeof(Axon_Channel));
    ch->head = 0;
    ch->tail = 0;
    return ch;
}

static inline bool axon_channel_send(Axon_Channel* ch, void* val) {
    if (!ch) return false;
    long t = (long)ch->tail;
    long h = (long)ch->head;
    if (t - h >= AXON_CHANNEL_CAP) return false;
    ch->buffer[t & AXON_CHANNEL_MASK] = val;
    AXON_FENCE();
    ch->tail = t + 1;
    return true;
}

static inline void* axon_channel_recv(Axon_Channel* ch) {
    if (!ch) return NULL;
    while (1) {
        long h = (long)ch->head;
        long t = (long)ch->tail;
        if (h < t) {
            void* val = ch->buffer[h & AXON_CHANNEL_MASK];
            AXON_FENCE();
            if (AXON_CAS_INT(&ch->head, h, h + 1)) {
                return val;
            }
        } else {
            AXON_PAUSE();
            AXON_YIELD();
        }
    }
}

static inline void axon_channel_free(Axon_Channel* ch) {
    if (ch) free(ch);
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
