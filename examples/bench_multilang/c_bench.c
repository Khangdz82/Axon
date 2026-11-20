#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <windows.h>
#include <intrin.h>

static double get_time_ms(LARGE_INTEGER start, LARGE_INTEGER end, LARGE_INTEGER freq) {
    return (double)(end.QuadPart - start.QuadPart) * 1000.0 / (double)freq.QuadPart;
}

// --- W1: Matrix GEMM 256x256 FP32 ---
static double bench_gemm(void) {
    int n = 256;
    float* a = (float*)malloc(n * n * sizeof(float));
    float* b = (float*)malloc(n * n * sizeof(float));
    float* c = (float*)malloc(n * n * sizeof(float));
    for (int i = 0; i < n * n; i++) {
        a[i] = 0.5f;
        b[i] = 0.5f;
        c[i] = 0.0f;
    }

    LARGE_INTEGER freq, t0, t1;
    QueryPerformanceFrequency(&freq);
    QueryPerformanceCounter(&t0);

    for (int i = 0; i < n; i++) {
        for (int k = 0; k < n; k++) {
            float aik = a[i * n + k];
            for (int j = 0; j < n; j++) {
                c[i * n + j] += aik * b[k * n + j];
            }
        }
    }

    QueryPerformanceCounter(&t1);
    double elapsed = get_time_ms(t0, t1, freq);

    // Prevent dead code elimination
    if (c[0] < 0.0f) printf("checksum: %f\n", c[0]);

    free(a); free(b); free(c);
    return elapsed;
}

// --- W2: BNN 1-Bit POPCOUNT GEMV (4096 rows x 4096 cols = 64 words) ---
static double bench_bnn(void) {
    int rows = 4096;
    int words = 64;
    unsigned __int64* weights = (unsigned __int64*)malloc(rows * words * sizeof(unsigned __int64));
    unsigned __int64* act = (unsigned __int64*)malloc(words * sizeof(unsigned __int64));
    int* out = (int*)malloc(rows * sizeof(int));

    for (int i = 0; i < words; i++) act[i] = 0xAAAAAAAAAAAAAAAAULL;
    for (int i = 0; i < rows * words; i++) weights[i] = 0x5555555555555555ULL;

    LARGE_INTEGER freq, t0, t1;
    QueryPerformanceFrequency(&freq);
    QueryPerformanceCounter(&t0);

    for (int r = 0; r < rows; r++) {
        int sum = 0;
        int base = r * words;
        for (int w = 0; w < words; w++) {
            unsigned __int64 x = weights[base + w] ^ act[w];
            sum += (int)__popcnt64(x);
        }
        out[r] = sum;
    }

    QueryPerformanceCounter(&t1);
    double elapsed = get_time_ms(t0, t1, freq);

    if (out[0] < 0) printf("checksum: %d\n", out[0]);

    free(weights); free(act); free(out);
    return elapsed;
}

// --- W3: Quicksort (100,000 int64) ---
static void quicksort_i64(__int64* arr, int low, int high) {
    if (low >= high) return;
    __int64 pivot = arr[high];
    int i = low - 1;
    for (int j = low; j < high; j++) {
        if (arr[j] <= pivot) {
            i++;
            __int64 tmp = arr[i];
            arr[i] = arr[j];
            arr[j] = tmp;
        }
    }
    __int64 tmp = arr[i + 1];
    arr[i + 1] = arr[high];
    arr[high] = tmp;
    int pi = i + 1;

    quicksort_i64(arr, low, pi - 1);
    quicksort_i64(arr, pi + 1, high);
}

static double bench_sort(void) {
    int n = 100000;
    __int64* arr = (__int64*)malloc(n * sizeof(__int64));
    unsigned int seed = 42;
    for (int i = 0; i < n; i++) {
        seed = (seed * 1103515245U + 12345U) & 0x7FFFFFFFU;
        arr[i] = (seed % 1000000);
    }

    LARGE_INTEGER freq, t0, t1;
    QueryPerformanceFrequency(&freq);
    QueryPerformanceCounter(&t0);

    quicksort_i64(arr, 0, n - 1);

    QueryPerformanceCounter(&t1);
    double elapsed = get_time_ms(t0, t1, freq);

    if (arr[0] < 0) printf("checksum: %lld\n", arr[0]);

    free(arr);
    return elapsed;
}

// --- W4: Sieve of Eratosthenes (2,000,000) ---
static double bench_sieve(void) {
    int n = 2000000;
    char* is_prime = (char*)malloc(n + 1);
    memset(is_prime, 1, n + 1);
    is_prime[0] = 0;
    is_prime[1] = 0;

    LARGE_INTEGER freq, t0, t1;
    QueryPerformanceFrequency(&freq);
    QueryPerformanceCounter(&t0);

    for (int p = 2; p * p <= n; p++) {
        if (is_prime[p]) {
            for (int i = p * p; i <= n; i += p) {
                is_prime[i] = 0;
            }
        }
    }

    int count = 0;
    for (int i = 2; i <= n; i++) {
        if (is_prime[i]) count++;
    }

    QueryPerformanceCounter(&t1);
    double elapsed = get_time_ms(t0, t1, freq);

    if (count == 0) printf("checksum: %d\n", count);

    free(is_prime);
    return elapsed;
}

// --- W5: 3D Particle Kinematics (50,000 particles x 10 steps) ---
typedef struct {
    float x, y, z;
    float vx, vy, vz;
} Particle;

static double bench_physics(void) {
    int n = 50000;
    Particle* p = (Particle*)malloc(n * sizeof(Particle));
    for (int i = 0; i < n; i++) {
        p[i].x = 0.0f; p[i].y = 100.0f; p[i].z = 0.0f;
        p[i].vx = 1.0f; p[i].vy = 0.0f; p[i].vz = 0.5f;
    }
    float dt = 0.016f;
    float g = -9.81f;

    LARGE_INTEGER freq, t0, t1;
    QueryPerformanceFrequency(&freq);
    QueryPerformanceCounter(&t0);

    for (int step = 0; step < 10; step++) {
        for (int i = 0; i < n; i++) {
            p[i].x += p[i].vx * dt;
            p[i].y += p[i].vy * dt;
            p[i].z += p[i].vz * dt;
            p[i].vy += g * dt;
        }
    }

    QueryPerformanceCounter(&t1);
    double elapsed = get_time_ms(t0, t1, freq);

    if (p[0].y < -9999.0f) printf("checksum: %f\n", p[0].y);

    free(p);
    return elapsed;
}

// --- W6: JSON Scanner (100,000 runs) ---
static double bench_json(void) {
    const char* doc = "{\"order_id\": 9876543, \"user_id\": 42109, \"symbol\": \"BTCUSD\", \"qty\": 1500, \"price\": 96500, \"status\": \"FILLED\"}";
    int doc_len = (int)strlen(doc);
    const char* key = "\"price\": ";
    int key_len = (int)strlen(key);

    LARGE_INTEGER freq, t0, t1;
    QueryPerformanceFrequency(&freq);
    QueryPerformanceCounter(&t0);

    long long sum_price = 0;
    for (int r = 0; r < 100000; r++) {
        for (int i = 0; i <= doc_len - key_len; i++) {
            if (doc[i] == '\"' && doc[i + 1] == 'p') {
                if (strncmp(&doc[i], key, key_len) == 0) {
                    int val = 0;
                    int k = i + key_len;
                    while (doc[k] >= '0' && doc[k] <= '9') {
                        val = val * 10 + (doc[k] - '0');
                        k++;
                    }
                    sum_price += val;
                    break;
                }
            }
        }
    }

    QueryPerformanceCounter(&t1);
    double elapsed = get_time_ms(t0, t1, freq);

    if (sum_price == 0) printf("checksum: %lld\n", sum_price);

    return elapsed;
}

static long long fib_c(int n) {
    if (n <= 1) return n;
    return fib_c(n - 1) + fib_c(n - 2);
}

static double bench_fib(void) {
    LARGE_INTEGER freq, t0, t1;
    QueryPerformanceFrequency(&freq);
    QueryPerformanceCounter(&t0);
    volatile long long r = fib_c(35);
    QueryPerformanceCounter(&t1);
    if (r != 9227465) printf("checksum: %lld\n", r);
    return get_time_ms(t0, t1, freq);
}

static double bench_crc32(void) {
    int n = 16777216;
    unsigned char* buf = (unsigned char*)malloc(n);
    unsigned int tbl[256];
    for (int i = 0; i < 256; i++) {
        unsigned int c = (unsigned int)i;
        for (int k = 0; k < 8; k++) c = (c & 1) ? ((c >> 1) ^ 0xEDB88320u) : (c >> 1);
        tbl[i] = c;
    }
    for (int i = 0; i < n; i++) buf[i] = (unsigned char)((i * 7 + 13) & 255);
    LARGE_INTEGER freq, t0, t1;
    QueryPerformanceFrequency(&freq);
    QueryPerformanceCounter(&t0);
    unsigned int crc = 0xFFFFFFFFu;
    for (int i = 0; i < n; i++) crc = (crc >> 8) ^ tbl[(crc ^ buf[i]) & 255];
    crc ^= 0xFFFFFFFFu;
    QueryPerformanceCounter(&t1);
    if (crc == 0) printf("checksum: %u\n", crc);
    free(buf);
    return get_time_ms(t0, t1, freq);
}

static double bench_mandel(void) {
    int size = 512;
    LARGE_INTEGER freq, t0, t1;
    QueryPerformanceFrequency(&freq);
    QueryPerformanceCounter(&t0);
    long long total = 0;
    for (int py = 0; py < size; py++) {
        double ci = ((double)py / 512.0) * 2.4 - 1.2;
        for (int px = 0; px < size; px++) {
            double cr = ((double)px / 512.0) * 3.0 - 2.0;
            double zr = 0.0, zi = 0.0;
            int it = 0;
            while (it < 64) {
                double zr2 = zr * zr, zi2 = zi * zi;
                if (zr2 + zi2 > 4.0) break;
                zi = 2.0 * zr * zi + ci;
                zr = zr2 - zi2 + cr;
                it++;
            }
            total += it;
        }
    }
    QueryPerformanceCounter(&t1);
    if (total == 0) printf("checksum: %lld\n", total);
    return get_time_ms(t0, t1, freq);
}

static double bench_gc_churn(void) {
    LARGE_INTEGER freq, t0, t1;
    QueryPerformanceFrequency(&freq);
    QueryPerformanceCounter(&t0);
    volatile long long acc = 0;
    for (int i = 0; i < 1000000; i++) {
        long long* volatile b = (long long*)malloc(64);
        b[0] = i & 255;
        acc += b[0];
        free(b);
    }
    QueryPerformanceCounter(&t1);
    if (acc == 0) printf("checksum: %lld\n", acc);
    return get_time_ms(t0, t1, freq);
}

int main(void) {
    double t_gemm = bench_gemm();
    double t_bnn = bench_bnn();
    double t_sort = bench_sort();
    double t_sieve = bench_sieve();
    double t_phys = bench_physics();
    double t_json = bench_json();
    double t_fib = bench_fib();
    double t_crc = bench_crc32();
    double t_mandel = bench_mandel();
    double t_gc = bench_gc_churn();

    printf("{\"lang\": \"C (MSVC /O2)\", \"gemm\": %.2f, \"bnn\": %.2f, \"sort\": %.2f, \"sieve\": %.2f, \"physics\": %.2f, \"json\": %.2f, \"fib\": %.2f, \"crc32\": %.2f, \"mandel\": %.2f, \"gc_churn\": %.2f}\n",
           t_gemm, t_bnn, t_sort, t_sieve, t_phys, t_json, t_fib, t_crc, t_mandel, t_gc);
    return 0;
}
