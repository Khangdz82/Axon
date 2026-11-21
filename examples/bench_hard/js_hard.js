const { performance } = require('perf_hooks');

function benchNBody() {
    const n = 256, steps = 40, dt = 0.01;
    const px = new Float64Array(n), py = new Float64Array(n), pz = new Float64Array(n);
    const vx = new Float64Array(n), vy = new Float64Array(n), vz = new Float64Array(n);
    for (let i = 0; i < n; i++) { px[i] = i*0.1 + 1.0; py[i] = i*0.2 - 0.5; pz[i] = i*0.05; }
    const t0 = performance.now();
    for (let s = 0; s < steps; s++) {
        for (let a = 0; a < n; a++) {
            const ax = px[a], ay = py[a], az = pz[a];
            let fx = 0, fy = 0, fz = 0;
            for (let b = 0; b < n; b++) {
                if (a !== b) {
                    const dx = px[b] - ax, dy = py[b] - ay, dz = pz[b] - az;
                    const d2 = dx*dx + dy*dy + dz*dz + 0.01;
                    const inv = 1.0 / (d2 * Math.sqrt(d2));
                    fx += dx * inv; fy += dy * inv; fz += dz * inv;
                }
            }
            vx[a] += fx * dt; vy[a] += fy * dt; vz[a] += fz * dt;
        }
        for (let k = 0; k < n; k++) { px[k] += vx[k]*dt; py[k] += vy[k]*dt; pz[k] += vz[k]*dt; }
    }
    const ms = performance.now() - t0;
    let sum = 0; for (let i = 0; i < n; i++) sum += px[i] + py[i];
    return [ms, Math.trunc(sum + 0.5)];
}

function benchMatMul64() {
    const n = 192, nn = n * n;
    const a = new Float64Array(nn), b = new Float64Array(nn), c = new Float64Array(nn);
    for (let i = 0; i < nn; i++) { a[i] = (i % 17) * 0.25; b[i] = (i % 13) * 0.5; }
    const t0 = performance.now();
    for (let i = 0; i < n; i++) {
        const inOff = i * n;
        for (let k = 0; k < n; k++) {
            const aik = a[inOff + k], knOff = k * n;
            for (let j = 0; j < n; j++) c[inOff + j] += aik * b[knOff + j];
        }
    }
    const ms = performance.now() - t0;
    return [ms, Math.trunc(c[0] + c[nn - 1])];
}

function benchARX() {
    const t0 = performance.now();
    let a = 0x61707865 >>> 0, b = 0x3320646E >>> 0, c = 0x79622D32 >>> 0, d = 0x6B206574 >>> 0;
    for (let i = 0; i < 2000000; i++) {
        a = (a + b) >>> 0; d ^= a; d = ((d << 16) | (d >>> 16)) >>> 0;
        c = (c + d) >>> 0; b ^= c; b = ((b << 12) | (b >>> 20)) >>> 0;
        a = (a + b) >>> 0; d ^= a; d = ((d << 8)  | (d >>> 24)) >>> 0;
        c = (c + d) >>> 0; b ^= c; b = ((b << 7)  | (b >>> 25)) >>> 0;
    }
    const ms = performance.now() - t0;
    return [ms, (a ^ b ^ c ^ d) >>> 0];
}

function benchDijkstra() {
    const nv = 10000, deg = 8, ne = nv * deg, inf = 1e12;
    const to = new Int32Array(ne), wt = new Int32Array(ne), dist = new Float64Array(nv);
    const hpD = new Float64Array(ne + 16), hpU = new Int32Array(ne + 16);
    let seed = 1234567;
    for (let u = 0; u < nv; u++) {
        for (let e = 0; e < deg; e++) {
            seed = (Math.imul(seed, 1103515245) + 12345) & 0x7FFFFFFF;
            to[u*deg + e] = (u + 1 + (seed % 97)) % nv;
            wt[u*deg + e] = 1 + ((seed >>> 8) % 100);
        }
        dist[u] = inf;
    }
    const t0 = performance.now();
    dist[0] = 0; let hsz = 1; hpD[0] = 0; hpU[0] = 0;
    while (hsz > 0) {
        const d = hpD[0], cur = hpU[0]; hsz--;
        if (hsz > 0) {
            const ld = hpD[hsz], lu = hpU[hsz]; let p = 0;
            while (true) {
                const l = p*2 + 1; if (l >= hsz) break;
                const sm = (l+1 < hsz && hpD[l+1] < hpD[l]) ? l+1 : l;
                if (ld <= hpD[sm]) break;
                hpD[p] = hpD[sm]; hpU[p] = hpU[sm]; p = sm;
            }
            hpD[p] = ld; hpU[p] = lu;
        }
        if (d > dist[cur]) continue;
        const base = cur * deg;
        for (let e = 0; e < deg; e++) {
            const nxt = to[base + e], nd = d + wt[base + e];
            if (nd < dist[nxt]) {
                dist[nxt] = nd; let idx = hsz++;
                while (idx > 0) {
                    const par = (idx - 1) >> 1;
                    if (hpD[par] <= nd) break;
                    hpD[idx] = hpD[par]; hpU[idx] = hpU[par]; idx = par;
                }
                hpD[idx] = nd; hpU[idx] = nxt;
            }
        }
    }
    const ms = performance.now() - t0;
    let chk = 0; for (let u = 0; u < nv; u++) chk += dist[u];
    return [ms, chk];
}

function benchKnapsack() {
    const n = 200, cap = 15000;
    const dp = new Int32Array(cap + 1), wt = new Int32Array(n), vl = new Int32Array(n);
    let seed = 9876543;
    for (let i = 0; i < n; i++) {
        seed = (Math.imul(seed, 1103515245) + 12345) & 0x7FFFFFFF;
        wt[i] = 10 + (seed % 300); vl[i] = 5 + ((seed >>> 8) % 500);
    }
    const t0 = performance.now();
    for (let i = 0; i < n; i++) {
        const w = wt[i], v = vl[i];
        for (let c = cap; c >= w; c--) {
            const cand = dp[c - w] + v;
            if (cand > dp[c]) dp[c] = cand;
        }
    }
    const ms = performance.now() - t0;
    return [ms, dp[cap]];
}

function benchHashMap() {
    const n = 200000, cap = 524288, mask = cap - 1;
    const keys = new Int32Array(cap), vals = new Int32Array(cap);
    const t0 = performance.now();
    for (let i = 1; i <= n; i++) {
        let k1 = Math.imul(i, 2654435761) & 0x7FFFFFFF; if (k1 === 0) k1 = 1;
        let slot = Number((BigInt(k1) * 11400714819323198485n >> 45n) & BigInt(mask));
        while (true) {
            if (keys[slot] === 0 || keys[slot] === k1) { keys[slot] = k1; vals[slot] = i*3 + 7; break; }
            slot = (slot + 1) & mask;
        }
    }
    let acc = 0;
    for (let i = 1; i <= n; i++) {
        let k1 = Math.imul(i, 2654435761) & 0x7FFFFFFF; if (k1 === 0) k1 = 1;
        let slot = Number((BigInt(k1) * 11400714819323198485n >> 45n) & BigInt(mask));
        while (true) {
            if (keys[slot] === k1) { acc += vals[slot]; break; }
            if (keys[slot] === 0) break;
            slot = (slot + 1) & mask;
        }
    }
    const ms = performance.now() - t0;
    return [ms, acc];
}

const [t1, c1] = benchNBody();
const [t2, c2] = benchMatMul64();
const [t3, c3] = benchARX();
const [t4, c4] = benchDijkstra();
const [t5, c5] = benchKnapsack();
const [t6, c6] = benchHashMap();
const rt = typeof Bun !== 'undefined' ? 'Bun (JSC)' : 'Node.js (v24 V8)';
console.log(JSON.stringify({ lang: rt, nbody: +t1.toFixed(2), matmul64: +t2.toFixed(2), arx: +t3.toFixed(2), dijkstra: +t4.toFixed(2), knapsack: +t5.toFixed(2), hashmap: +t6.toFixed(2), chk: [c1, c2, c3, c4, c5, c6] }));
