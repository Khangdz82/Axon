// JavaScript Multi-Runtime Benchmark (Node.js & Bun)

function benchGEMM() {
    const n = 256;
    const a = new Float32Array(n * n);
    const b = new Float32Array(n * n);
    const c = new Float32Array(n * n);
    a.fill(0.5);
    b.fill(0.5);

    const t0 = performance.now();
    for (let i = 0; i < n; i++) {
        for (let k = 0; k < n; k++) {
            const aik = a[i * n + k];
            for (let j = 0; j < n; j++) {
                c[i * n + j] += aik * b[k * n + j];
            }
        }
    }
    const elapsed = performance.now() - t0;
    if (c[0] < 0) console.log("checksum:", c[0]);
    return elapsed;
}

function popcnt32(v) {
    v = v - ((v >>> 1) & 0x55555555);
    v = (v & 0x33333333) + ((v >>> 2) & 0x33333333);
    return ((v + (v >>> 4) & 0xF0F0F0F) * 0x1010101) >>> 24;
}

function benchBNN() {
    const rows = 4096;
    const words = 128; // 128 x 32-bit words = 4096 bits
    const weights = new Uint32Array(rows * words);
    const act = new Uint32Array(words);
    const out = new Int32Array(rows);
    act.fill(0xAAAAAAAA);
    weights.fill(0x55555555);

    const t0 = performance.now();
    for (let r = 0; r < rows; r++) {
        let sum = 0;
        const base = r * words;
        for (let w = 0; w < words; w++) {
            const x = weights[base + w] ^ act[w];
            sum += popcnt32(x);
        }
        out[r] = sum;
    }
    const elapsed = performance.now() - t0;
    if (out[0] < 0) console.log("checksum:", out[0]);
    return elapsed;
}

function quicksortI64(arr, low, high) {
    if (low >= high) return;
    const pivot = arr[high];
    let i = low - 1;
    for (let j = low; j < high; j++) {
        if (arr[j] <= pivot) {
            i++;
            const tmp = arr[i];
            arr[i] = arr[j];
            arr[j] = tmp;
        }
    }
    const tmp = arr[i + 1];
    arr[i + 1] = arr[high];
    arr[high] = tmp;
    const pi = i + 1;

    quicksortI64(arr, low, pi - 1);
    quicksortI64(arr, pi + 1, high);
}

function benchSort() {
    const n = 100000;
    const arr = new Float64Array(n);
    let seed = 42;
    for (let i = 0; i < n; i++) {
        seed = (Math.imul(seed, 1103515245) + 12345) & 0x7FFFFFFF;
        arr[i] = seed % 1000000;
    }

    const t0 = performance.now();
    quicksortI64(arr, 0, n - 1);
    const elapsed = performance.now() - t0;

    if (arr[0] < 0) console.log("checksum:", arr[0]);
    return elapsed;
}

function benchSieve() {
    const n = 2000000;
    const isPrime = new Uint8Array(n + 1);
    isPrime.fill(1);
    isPrime[0] = 0;
    isPrime[1] = 0;

    const t0 = performance.now();
    for (let p = 2; p * p <= n; p++) {
        if (isPrime[p]) {
            for (let i = p * p; i <= n; i += p) {
                isPrime[i] = 0;
            }
        }
    }

    let count = 0;
    for (let i = 2; i <= n; i++) {
        if (isPrime[i]) count++;
    }
    const elapsed = performance.now() - t0;

    if (count === 0) console.log("checksum:", count);
    return elapsed;
}

function benchPhysics() {
    const n = 50000;
    const x = new Float32Array(n);
    const y = new Float32Array(n);
    const z = new Float32Array(n);
    const vx = new Float32Array(n);
    const vy = new Float32Array(n);
    const vz = new Float32Array(n);

    y.fill(100.0);
    vx.fill(1.0);
    vz.fill(0.5);

    const dt = 0.016;
    const g = -9.81;

    const t0 = performance.now();
    for (let step = 0; step < 10; step++) {
        for (let i = 0; i < n; i++) {
            x[i] += vx[i] * dt;
            y[i] += vy[i] * dt;
            z[i] += vz[i] * dt;
            vy[i] += g * dt;
        }
    }
    const elapsed = performance.now() - t0;

    if (y[0] < -9999.0) console.log("checksum:", y[0]);
    return elapsed;
}

function benchJSON() {
    const doc = "{\"order_id\": 9876543, \"user_id\": 42109, \"symbol\": \"BTCUSD\", \"qty\": 1500, \"price\": 96500, \"status\": \"FILLED\"}";
    const docLen = doc.length;
    const key = "\"price\": ";
    const keyLen = key.length;

    const t0 = performance.now();
    let sumPrice = 0;
    for (let r = 0; r < 100000; r++) {
        for (let i = 0; i <= docLen - keyLen; i++) {
            if (doc.charCodeAt(i) === 34 && doc.charCodeAt(i + 1) === 112) { // '"p'
                if (doc.startsWith(key, i)) {
                    let val = 0;
                    let k = i + keyLen;
                    while (k < docLen) {
                        const code = doc.charCodeAt(k);
                        if (code >= 48 && code <= 57) {
                            val = val * 10 + (code - 48);
                            k++;
                        } else {
                            break;
                        }
                    }
                    sumPrice += val;
                    break;
                }
            }
        }
    }
    const elapsed = performance.now() - t0;

    if (sumPrice === 0) console.log("checksum:", sumPrice);
    return elapsed;
}

function fibJs(n) { return n <= 1 ? n : fibJs(n - 1) + fibJs(n - 2); }

function benchFib() {
    const t0 = performance.now();
    const r = fibJs(35);
    const el = performance.now() - t0;
    if (r !== 9227465) console.log("checksum:", r);
    return el;
}

function benchCRC32() {
    const n = 16777216;
    const buf = new Uint8Array(n);
    const tbl = new Uint32Array(256);
    for (let i = 0; i < 256; i++) {
        let c = i;
        for (let k = 0; k < 8; k++) c = (c & 1) ? ((c >>> 1) ^ 0xEDB88320) : (c >>> 1);
        tbl[i] = c;
    }
    for (let i = 0; i < n; i++) buf[i] = (i * 7 + 13) & 255;
    const t0 = performance.now();
    let crc = 0xFFFFFFFF;
    for (let i = 0; i < n; i++) crc = (crc >>> 8) ^ tbl[(crc ^ buf[i]) & 255];
    crc = (crc ^ 0xFFFFFFFF) >>> 0;
    const el = performance.now() - t0;
    if (crc === 0) console.log("checksum:", crc);
    return el;
}

function benchMandel() {
    const size = 512;
    const t0 = performance.now();
    let total = 0;
    for (let py = 0; py < size; py++) {
        const ci = (py / 512.0) * 2.4 - 1.2;
        for (let px = 0; px < size; px++) {
            const cr = (px / 512.0) * 3.0 - 2.0;
            let zr = 0.0, zi = 0.0, it = 0;
            while (it < 64) {
                const zr2 = zr * zr, zi2 = zi * zi;
                if (zr2 + zi2 > 4.0) break;
                zi = 2.0 * zr * zi + ci;
                zr = zr2 - zi2 + cr;
                it++;
            }
            total += it;
        }
    }
    const el = performance.now() - t0;
    if (total === 0) console.log("checksum:", total);
    return el;
}

function benchGCChurn() {
    const t0 = performance.now();
    let acc = 0;
    for (let i = 0; i < 1000000; i++) {
        const b = [0, 0, 0, 0, 0, 0, 0, 0];
        b[0] = i & 255;
        acc += b[0];
    }
    const el = performance.now() - t0;
    if (acc === 0) console.log("checksum:", acc);
    return el;
}

const runtime = typeof Bun !== 'undefined' ? "Bun (1.4)" : "Node.js (v24 V8)";
const tGemm = benchGEMM();
const tBnn = benchBNN();
const tSort = benchSort();
const tSieve = benchSieve();
const tPhys = benchPhysics();
const tJson = benchJSON();
const tFib = benchFib();
const tCrc = benchCRC32();
const tMandel = benchMandel();
const tGc = benchGCChurn();

console.log(JSON.stringify({
    lang: runtime,
    gemm: parseFloat(tGemm.toFixed(2)),
    bnn: parseFloat(tBnn.toFixed(2)),
    sort: parseFloat(tSort.toFixed(2)),
    sieve: parseFloat(tSieve.toFixed(2)),
    physics: parseFloat(tPhys.toFixed(2)),
    json: parseFloat(tJson.toFixed(2)),
    fib: parseFloat(tFib.toFixed(2)),
    crc32: parseFloat(tCrc.toFixed(2)),
    mandel: parseFloat(tMandel.toFixed(2)),
    gc_churn: parseFloat(tGc.toFixed(2))
}));
