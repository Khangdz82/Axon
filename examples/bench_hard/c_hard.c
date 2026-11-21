#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <math.h>
#include <windows.h>

static double ms_since(LARGE_INTEGER t0, LARGE_INTEGER t1, LARGE_INTEGER freq) {
    return (double)(t1.QuadPart - t0.QuadPart) * 1000.0 / (double)freq.QuadPart;
}

static double bench_nbody(long long* out_chk) {
    int n = 256, steps = 40;
    double dt = 0.01;
    double *px = malloc(n*8), *py = malloc(n*8), *pz = malloc(n*8);
    double *vx = calloc(n, 8), *vy = calloc(n, 8), *vz = calloc(n, 8);
    for (int i = 0; i < n; i++) { px[i] = i*0.1 + 1.0; py[i] = i*0.2 - 0.5; pz[i] = i*0.05; }
    LARGE_INTEGER f, t0, t1; QueryPerformanceFrequency(&f); QueryPerformanceCounter(&t0);
    for (int s = 0; s < steps; s++) {
        for (int a = 0; a < n; a++) {
            double ax = px[a], ay = py[a], az = pz[a], fx = 0, fy = 0, fz = 0;
            for (int b = 0; b < n; b++) {
                if (a != b) {
                    double dx = px[b] - ax, dy = py[b] - ay, dz = pz[b] - az;
                    double d2 = dx*dx + dy*dy + dz*dz + 0.01;
                    double inv = 1.0 / (d2 * sqrt(d2));
                    fx += dx * inv; fy += dy * inv; fz += dz * inv;
                }
            }
            vx[a] += fx * dt; vy[a] += fy * dt; vz[a] += fz * dt;
        }
        for (int k = 0; k < n; k++) { px[k] += vx[k]*dt; py[k] += vy[k]*dt; pz[k] += vz[k]*dt; }
    }
    QueryPerformanceCounter(&t1);
    double sum = 0; for (int i = 0; i < n; i++) sum += px[i] + py[i];
    *out_chk = (long long)(sum + 0.5);
    free(px); free(py); free(pz); free(vx); free(vy); free(vz);
    return ms_since(t0, t1, f);
}

static double bench_matmul64(long long* out_chk) {
    int n = 192, nn = n*n;
    double *a = malloc(nn*8), *b = malloc(nn*8), *c = calloc(nn, 8);
    for (int i = 0; i < nn; i++) { a[i] = (i % 17) * 0.25; b[i] = (i % 13) * 0.5; }
    LARGE_INTEGER f, t0, t1; QueryPerformanceFrequency(&f); QueryPerformanceCounter(&t0);
    for (int i = 0; i < n; i++) {
        int in_off = i * n;
        for (int k = 0; k < n; k++) {
            double aik = a[in_off + k];
            int kn_off = k * n;
            for (int j = 0; j < n; j++) c[in_off + j] += aik * b[kn_off + j];
        }
    }
    QueryPerformanceCounter(&t1);
    *out_chk = (long long)(c[0] + c[nn - 1]);
    free(a); free(b); free(c);
    return ms_since(t0, t1, f);
}

static double bench_arx_crypto(long long* out_chk) {
    LARGE_INTEGER f, t0, t1; QueryPerformanceFrequency(&f); QueryPerformanceCounter(&t0);
    uint32_t a = 0x61707865u, b = 0x3320646Eu, c = 0x79622D32u, d = 0x6B206574u;
    for (int i = 0; i < 2000000; i++) {
        a += b; d ^= a; d = (d << 16) | (d >> 16);
        c += d; b ^= c; b = (b << 12) | (b >> 20);
        a += b; d ^= a; d = (d << 8)  | (d >> 24);
        c += d; b ^= c; b = (b << 7)  | (b >> 25);
    }
    QueryPerformanceCounter(&t1);
    *out_chk = (long long)(a ^ b ^ c ^ d);
    return ms_since(t0, t1, f);
}

static double bench_dijkstra(long long* out_chk) {
    int nv = 10000, deg = 8, ne = nv * deg;
    long long inf = 1000000000000LL;
    long long *to = malloc(ne*8), *wt = malloc(ne*8), *dist = malloc(nv*8);
    long long *hp_d = malloc((ne+16)*8), *hp_u = malloc((ne+16)*8);
    long long seed = 1234567;
    for (int u = 0; u < nv; u++) {
        for (int e = 0; e < deg; e++) {
            seed = (seed * 1103515245LL + 12345LL) & 0x7FFFFFFFLL;
            to[u*deg + e] = (u + 1 + (seed % 97)) % nv;
            wt[u*deg + e] = 1 + ((seed >> 8) % 100);
        }
        dist[u] = inf;
    }
    LARGE_INTEGER f, t0, t1; QueryPerformanceFrequency(&f); QueryPerformanceCounter(&t0);
    dist[0] = 0; int hsz = 1; hp_d[0] = 0; hp_u[0] = 0;
    while (hsz > 0) {
        long long d = hp_d[0], cur = hp_u[0]; hsz--;
        if (hsz > 0) {
            long long ld = hp_d[hsz], lu = hp_u[hsz]; int p = 0;
            while (1) {
                int l = p*2 + 1; if (l >= hsz) break;
                int sm = (l+1 < hsz && hp_d[l+1] < hp_d[l]) ? l+1 : l;
                if (ld <= hp_d[sm]) break;
                hp_d[p] = hp_d[sm]; hp_u[p] = hp_u[sm]; p = sm;
            }
            hp_d[p] = ld; hp_u[p] = lu;
        }
        if (d > dist[cur]) continue;
        long long base = cur * deg;
        for (int e = 0; e < deg; e++) {
            long long nxt = to[base + e], nd = d + wt[base + e];
            if (nd < dist[nxt]) {
                dist[nxt] = nd; int idx = hsz++;
                while (idx > 0) {
                    int par = (idx - 1) >> 1;
                    if (hp_d[par] <= nd) break;
                    hp_d[idx] = hp_d[par]; hp_u[idx] = hp_u[par]; idx = par;
                }
                hp_d[idx] = nd; hp_u[idx] = nxt;
            }
        }
    }
    QueryPerformanceCounter(&t1);
    long long chk = 0; for (int u = 0; u < nv; u++) chk += dist[u];
    *out_chk = chk;
    free(to); free(wt); free(dist); free(hp_d); free(hp_u);
    return ms_since(t0, t1, f);
}

static double bench_knapsack(long long* out_chk) {
    int n = 200, cap = 15000;
    long long *dp = calloc(cap+1, 8), *wt = malloc(n*8), *vl = malloc(n*8);
    long long seed = 9876543;
    for (int i = 0; i < n; i++) {
        seed = (seed * 1103515245LL + 12345LL) & 0x7FFFFFFFLL;
        wt[i] = 10 + (seed % 300); vl[i] = 5 + ((seed >> 8) % 500);
    }
    LARGE_INTEGER f, t0, t1; QueryPerformanceFrequency(&f); QueryPerformanceCounter(&t0);
    for (int i = 0; i < n; i++) {
        long long w = wt[i], v = vl[i];
        for (long long c = cap; c >= w; c--) {
            long long cand = dp[c - w] + v;
            if (cand > dp[c]) dp[c] = cand;
        }
    }
    QueryPerformanceCounter(&t1);
    *out_chk = dp[cap];
    free(dp); free(wt); free(vl);
    return ms_since(t0, t1, f);
}

static double bench_hashmap(long long* out_chk) {
    int n = 200000, cap = 524288, mask = cap - 1;
    long long *keys = calloc(cap, 8), *vals = malloc(cap * 8);
    LARGE_INTEGER f, t0, t1; QueryPerformanceFrequency(&f); QueryPerformanceCounter(&t0);
    for (long long i = 1; i <= n; i++) {
        long long k1 = (i * 2654435761LL) & 0x7FFFFFFFLL; if (!k1) k1 = 1;
        long long slot = (long long)(((uint64_t)k1 * 11400714819323198485ULL) >> 45) & mask;
        while (1) {
            if (keys[slot] == 0 || keys[slot] == k1) { keys[slot] = k1; vals[slot] = i*3 + 7; break; }
            slot = (slot + 1) & mask;
        }
    }
    long long acc = 0;
    for (long long i = 1; i <= n; i++) {
        long long k1 = (i * 2654435761LL) & 0x7FFFFFFFLL; if (!k1) k1 = 1;
        long long slot = (long long)(((uint64_t)k1 * 11400714819323198485ULL) >> 45) & mask;
        while (1) {
            if (keys[slot] == k1) { acc += vals[slot]; break; }
            if (keys[slot] == 0) break;
            slot = (slot + 1) & mask;
        }
    }
    QueryPerformanceCounter(&t1);
    *out_chk = acc;
    free(keys); free(vals);
    return ms_since(t0, t1, f);
}

int main(void) {
    long long c1, c2, c3, c4, c5, c6;
    double t1 = bench_nbody(&c1), t2 = bench_matmul64(&c2), t3 = bench_arx_crypto(&c3);
    double t4 = bench_dijkstra(&c4), t5 = bench_knapsack(&c5), t6 = bench_hashmap(&c6);
    printf("{\"lang\": \"C (MSVC /O2 AVX2)\", \"nbody\": %.2f, \"matmul64\": %.2f, \"arx\": %.2f, \"dijkstra\": %.2f, \"knapsack\": %.2f, \"hashmap\": %.2f, \"chk\": [%lld, %lld, %lld, %lld, %lld, %lld]}\n",
           t1, t2, t3, t4, t5, t6, c1, c2, c3, c4, c5, c6);
    return 0;
}
