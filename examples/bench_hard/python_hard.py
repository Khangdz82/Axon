import json, math, time

def bench_nbody():
    n, steps, dt = 256, 40, 0.01
    px = [i * 0.1 + 1.0 for i in range(n)]
    py = [i * 0.2 - 0.5 for i in range(n)]
    pz = [i * 0.05 for i in range(n)]
    vx, vy, vz = [0.0]*n, [0.0]*n, [0.0]*n
    sqrt = math.sqrt
    t0 = time.perf_counter()
    for _ in range(steps):
        for a in range(n):
            ax, ay, az = px[a], py[a], pz[a]
            fx = fy = fz = 0.0
            for b in range(n):
                if a != b:
                    dx, dy, dz = px[b] - ax, py[b] - ay, pz[b] - az
                    d2 = dx*dx + dy*dy + dz*dz + 0.01
                    inv = 1.0 / (d2 * sqrt(d2))
                    fx += dx * inv; fy += dy * inv; fz += dz * inv
            vx[a] += fx * dt; vy[a] += fy * dt; vz[a] += fz * dt
        for k in range(n):
            px[k] += vx[k]*dt; py[k] += vy[k]*dt; pz[k] += vz[k]*dt
    ms = (time.perf_counter() - t0) * 1000.0
    return ms, int(sum(px[i] + py[i] for i in range(n)) + 0.5)

def bench_matmul64():
    n = 192; nn = n * n
    a = [(i % 17) * 0.25 for i in range(nn)]
    b = [(i % 13) * 0.5 for i in range(nn)]
    c = [0.0] * nn
    t0 = time.perf_counter()
    for i in range(n):
        in_off = i * n
        for k in range(n):
            aik = a[in_off + k]; kn_off = k * n
            for j in range(n):
                c[in_off + j] += aik * b[kn_off + j]
    ms = (time.perf_counter() - t0) * 1000.0
    return ms, int(c[0] + c[nn - 1])

def bench_arx():
    t0 = time.perf_counter()
    a, b, c, d = 0x61707865, 0x3320646E, 0x79622D32, 0x6B206574
    m = 0xFFFFFFFF
    for _ in range(2000000):
        a = (a + b) & m; d ^= a; d = ((d << 16) | (d >> 16)) & m
        c = (c + d) & m; b ^= c; b = ((b << 12) | (b >> 20)) & m
        a = (a + b) & m; d ^= a; d = ((d << 8)  | (d >> 24)) & m
        c = (c + d) & m; b ^= c; b = ((b << 7)  | (b >> 25)) & m
    ms = (time.perf_counter() - t0) * 1000.0
    return ms, (a ^ b ^ c ^ d) & m

def bench_dijkstra():
    nv, deg = 10000, 8; ne = nv * deg; inf = 1000000000000
    to, wt = [0]*ne, [0]*ne; dist = [inf]*nv
    hp_d, hp_u = [0]*(ne+16), [0]*(ne+16)
    seed = 1234567
    for u in range(nv):
        for e in range(deg):
            seed = (seed * 1103515245 + 12345) & 0x7FFFFFFF
            to[u*deg + e] = (u + 1 + (seed % 97)) % nv
            wt[u*deg + e] = 1 + ((seed >> 8) % 100)
    t0 = time.perf_counter()
    dist[0] = 0; hsz = 1; hp_d[0] = 0; hp_u[0] = 0
    while hsz > 0:
        d, cur = hp_d[0], hp_u[0]; hsz -= 1
        if hsz > 0:
            ld, lu = hp_d[hsz], hp_u[hsz]; p = 0
            while True:
                l = p*2 + 1
                if l >= hsz: break
                sm = l+1 if (l+1 < hsz and hp_d[l+1] < hp_d[l]) else l
                if ld <= hp_d[sm]: break
                hp_d[p], hp_u[p] = hp_d[sm], hp_u[sm]; p = sm
            hp_d[p], hp_u[p] = ld, lu
        if d > dist[cur]: continue
        base = cur * deg
        for e in range(deg):
            nxt, nd = to[base + e], d + wt[base + e]
            if nd < dist[nxt]:
                dist[nxt] = nd; idx = hsz; hsz += 1
                while idx > 0:
                    par = (idx - 1) >> 1
                    if hp_d[par] <= nd: break
                    hp_d[idx], hp_u[idx] = hp_d[par], hp_u[par]; idx = par
                hp_d[idx], hp_u[idx] = nd, nxt
    ms = (time.perf_counter() - t0) * 1000.0
    return ms, sum(dist)

def bench_knapsack():
    n, cap = 200, 15000
    dp = [0]*(cap+1); wt, vl = [0]*n, [0]*n
    seed = 9876543
    for i in range(n):
        seed = (seed * 1103515245 + 12345) & 0x7FFFFFFF
        wt[i] = 10 + (seed % 300); vl[i] = 5 + ((seed >> 8) % 500)
    t0 = time.perf_counter()
    for i in range(n):
        w, v = wt[i], vl[i]
        for c in range(cap, w - 1, -1):
            cand = dp[c - w] + v
            if cand > dp[c]: dp[c] = cand
    ms = (time.perf_counter() - t0) * 1000.0
    return ms, dp[cap]

def bench_hashmap():
    n, cap = 200000, 524288; mask = cap - 1
    keys, vals = [0]*cap, [0]*cap
    t0 = time.perf_counter()
    for i in range(1, n + 1):
        k1 = (i * 2654435761) & 0x7FFFFFFF
        if k1 == 0: k1 = 1
        slot = (((k1 * 11400714819323198485) & 0xFFFFFFFFFFFFFFFF) >> 45) & mask
        while True:
            if keys[slot] == 0 or keys[slot] == k1:
                keys[slot] = k1; vals[slot] = i*3 + 7; break
            slot = (slot + 1) & mask
    acc = 0
    for i in range(1, n + 1):
        k1 = (i * 2654435761) & 0x7FFFFFFF
        if k1 == 0: k1 = 1
        slot = (((k1 * 11400714819323198485) & 0xFFFFFFFFFFFFFFFF) >> 45) & mask
        while True:
            if keys[slot] == k1: acc += vals[slot]; break
            if keys[slot] == 0: break
            slot = (slot + 1) & mask
    ms = (time.perf_counter() - t0) * 1000.0
    return ms, acc

t1, c1 = bench_nbody()
t2, c2 = bench_matmul64()
t3, c3 = bench_arx()
t4, c4 = bench_dijkstra()
t5, c5 = bench_knapsack()
t6, c6 = bench_hashmap()
print(json.dumps({"lang": "Python 3.11", "nbody": round(t1,2), "matmul64": round(t2,2), "arx": round(t3,2), "dijkstra": round(t4,2), "knapsack": round(t5,2), "hashmap": round(t6,2), "chk": [c1, c2, c3, c4, c5, c6]}))
