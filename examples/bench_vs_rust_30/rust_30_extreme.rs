// Rust (`rustc -C opt-level=3 -C target-cpu=native`) — 30-Benchmark "Light to Impossible" Baseline
// Exact 1-to-1 algorithmic, input, and checksum parity with `axon_30_extreme.ax`.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use std::hint::black_box;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

// --- Tier 1: Light (B01..B08) ---

fn b01_branchless_clamp_10m() -> i64 {
    let mut acc: i64 = 0;
    for i in 0i64..10_000_000 {
        let x = (i.wrapping_mul(37).wrapping_add(13)) & 1023;
        let v = x.clamp(100, 900);
        acc = acc.wrapping_add(v);
    }
    acc
}

fn fib_rec(n: i64) -> i64 {
    if n <= 1 { return n; }
    fib_rec(n - 1) + fib_rec(n - 2)
}

fn b02_fib_rec_38() -> i64 {
    fib_rec(black_box(38))
}

fn b03_sieve_1m() -> i64 {
    let n = 1_000_000usize;
    let mut p = vec![1u8; n + 1];
    p[0] = 0;
    p[1] = 0;
    let mut i = 2usize;
    while i * i <= n {
        if p[i] != 0 {
            let mut j = i * i;
            while j <= n {
                p[j] = 0;
                j += i;
            }
        }
        i += 1;
    }
    p[2..=n].iter().map(|&b| b as i64).sum()
}

fn b04_popcnt_10m() -> i64 {
    let mut acc: i64 = 0;
    for i in 0u64..10_000_000 {
        let v = i.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xA5A5_A5A5_A5A5_A5A5;
        acc = acc.wrapping_add(v.count_ones() as i64);
    }
    acc
}

fn b05_dot_f64_8m(a: &[f64], b: &[f64]) -> i64 {
    let mut sum = 0.0f64;
    for (&x, &y) in a.iter().zip(b.iter()) {
        sum += x * y;
    }
    (sum * 100.0) as i64
}

fn b06_splitmix64_25m() -> i64 {
    let mut s: u64 = 0x1234_5678_9ABC_DEF0;
    let mut acc: u64 = 0;
    for _ in 0..25_000_000 {
        s = s.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = s;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        acc ^= z;
    }
    acc as i64
}

fn b07_parse_i64_500k(buf: &[u8], stride: usize, cnt: usize) -> i64 {
    let mut acc: i64 = 0;
    for r in 0..cnt {
        let base = r * stride;
        let mut v: i64 = 0;
        for j in 0..10 {
            v = v * 10 + (buf[base + j] - b'0') as i64;
        }
        acc = acc.wrapping_add(v);
    }
    acc
}

fn b08_fnv1a_8m(data: &[u8]) -> i64 {
    let mut h: u64 = 0xCBF2_9CE4_8422_2325;
    for &b in data {
        h = (h ^ b as u64).wrapping_mul(0x0000_0100_0000_01B3);
    }
    h as i64
}

// --- Tier 2: Medium (B09..B16) ---

fn b09_sort_500k(orig: &[i64], work: &mut [i64]) -> i64 {
    work.copy_from_slice(orig);
    work.sort_unstable();
    work[0] ^ work[123_456] ^ work[250_000] ^ work[375_000] ^ work[work.len() - 1]
}

fn b10_hashmap_500k() -> i64 {
    let mut m: HashMap<i64, i64> = HashMap::with_capacity(64);
    for i in 0i64..250_000 {
        m.insert(i * 13 + 7, i * 3 + 1);
    }
    let mut acc: i64 = 0;
    for i in 0i64..250_000 {
        acc = acc.wrapping_add(*m.get(&(i * 13 + 7)).unwrap_or(&0));
    }
    acc
}

fn b11_mem_find_4m(hay: &str) -> i64 {
    let mut pos = 0usize;
    let mut acc: i64 = 0;
    let mut hits: i64 = 0;
    while let Some(rel) = hay[pos..].find("AXON_NEEDLE!") {
        let idx = pos + rel;
        acc = acc.wrapping_add(idx as i64);
        hits += 1;
        pos = idx + 12;
    }
    acc ^ (hits << 32)
}

fn parse_json_i64_field(line: &str, pat: &str) -> i64 {
    if let Some(p) = line.find(pat) {
        let bytes = &line.as_bytes()[p + pat.len()..];
        let mut v: i64 = 0;
        for &b in bytes {
            if b.is_ascii_digit() {
                v = v * 10 + (b - b'0') as i64;
            } else {
                break;
            }
        }
        return v;
    }
    0
}

fn b12_json_100k(doc: &str) -> i64 {
    let mut acc: i64 = 0;
    for line in doc.lines() {
        if line.len() > 2 {
            let id_v = parse_json_i64_field(line, "\"id\":");
            let sc_v = parse_json_i64_field(line, "\"score\":");
            acc = acc.wrapping_add(id_v).wrapping_add(sc_v);
        }
    }
    acc
}

fn b13_base64_2m(raw: &[u8]) -> i64 {
    const ENC: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut dec_tbl = [0u8; 256];
    for (i, &c) in ENC.iter().enumerate() { dec_tbl[c as usize] = i as u8; }
    let mut enc = Vec::with_capacity((raw.len() + 2) / 3 * 4);
    for chunk in raw.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let t = (b0 << 16) | (b1 << 8) | b2;
        enc.push(ENC[((t >> 18) & 63) as usize]);
        enc.push(ENC[((t >> 12) & 63) as usize]);
        enc.push(if chunk.len() > 1 { ENC[((t >> 6) & 63) as usize] } else { b'=' });
        enc.push(if chunk.len() > 2 { ENC[(t & 63) as usize] } else { b'=' });
    }
    let mut dec = Vec::with_capacity(raw.len());
    for chunk in enc.chunks_exact(4) {
        let v0 = dec_tbl[chunk[0] as usize] as u32;
        let v1 = dec_tbl[chunk[1] as usize] as u32;
        let v2 = dec_tbl[chunk[2] as usize] as u32;
        let v3 = dec_tbl[chunk[3] as usize] as u32;
        let t = (v0 << 18) | (v1 << 12) | (v2 << 6) | v3;
        dec.push(((t >> 16) & 255) as u8);
        if chunk[2] != b'=' { dec.push(((t >> 8) & 255) as u8); }
        if chunk[3] != b'=' { dec.push((t & 255) as u8); }
    }
    let mut acc = dec.len() as i64;
    let mut i = 0;
    while i < dec.len() { acc = acc.wrapping_add(dec[i] as i64); i += 64; }
    acc
}

fn b14_csv_100k_agg(data: &[u8]) -> i64 {
    let n = data.len();
    let mut i = 0usize;
    let mut sum_v: i64 = 0;
    let mut max_v: i64 = 0;
    while i < n {
        let mut c0 = 0i64; while i < n { let ch = data[i]; i += 1; if ch == b',' { break; } c0 = c0 * 10 + (ch - b'0') as i64; }
        let mut c1 = 0i64; while i < n { let ch = data[i]; i += 1; if ch == b',' { break; } c1 = c1 * 10 + (ch - b'0') as i64; }
        let mut c2 = 0i64; while i < n { let ch = data[i]; i += 1; if ch == b',' { break; } c2 = c2 * 10 + (ch - b'0') as i64; }
        let mut c3 = 0i64; while i < n { let ch = data[i]; i += 1; if ch == b'\n' { break; } c3 = c3 * 10 + (ch - b'0') as i64; }
        sum_v = sum_v.wrapping_add(c0).wrapping_add(c1).wrapping_add(c2).wrapping_add(c3);
        if c2 > max_v { max_v = c2; }
    }
    sum_v ^ (max_v << 20)
}

#[repr(align(64))]
struct PadAtom(AtomicUsize);

fn b15_spsc_ring_10m() -> i64 {
    let cap = 1024usize;
    let mask = cap - 1;
    let head = PadAtom(AtomicUsize::new(0));
    let tail = PadAtom(AtomicUsize::new(0));
    let mut buf = vec![0i64; cap];
    let mut src = [0i64; 256];
    let mut dst = [0i64; 256];
    let mut acc: i64 = 0;
    let mut base: i64 = 0;
    while base < 10_000_000 {
        for j in 0..256 { src[j] = base + j as i64; }
        let h = head.0.load(Ordering::Relaxed);
        let t = tail.0.load(Ordering::Acquire);
        let free = cap - h.wrapping_sub(t);
        let n = 256.min(free);
        for j in 0..n { buf[(h.wrapping_add(j)) & mask] = src[j]; }
        head.0.store(h.wrapping_add(n), Ordering::Release);

        let t2 = tail.0.load(Ordering::Relaxed);
        let h2 = head.0.load(Ordering::Acquire);
        let avail = h2.wrapping_sub(t2);
        let got = 256.min(avail);
        for j in 0..got { dst[j] = buf[(t2.wrapping_add(j)) & mask]; }
        tail.0.store(t2.wrapping_add(got), Ordering::Release);
        for j in 0..got { acc = acc.wrapping_add(dst[j]); }
        base += 256;
    }
    acc
}

fn b16_dijkstra_20k(adj: &[Vec<(usize, i64)>]) -> i64 {
    let n = adj.len();
    let inf: i64 = 1_000_000_000_000_000;
    let mut dist = vec![inf; n];
    dist[0] = 0;
    let mut pq: BinaryHeap<Reverse<(i64, usize)>> = BinaryHeap::new();
    pq.push(Reverse((0, 0)));
    while let Some(Reverse((d, u))) = pq.pop() {
        if d > dist[u] { continue; }
        for &(v, w) in &adj[u] {
            let nd = d + w;
            if nd < dist[v] {
                dist[v] = nd;
                pq.push(Reverse((nd, v)));
            }
        }
    }
    dist.iter().filter(|&&d| d < 1_000_000_000_000).copied().fold(0i64, i64::wrapping_add)
}

// --- Tier 3: Hard (B17..B23) ---

fn b17_crc32_8m(data: &[u8], tbl: &[u32; 256]) -> i64 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &b in data {
        crc = (crc >> 8) ^ tbl[((crc ^ b as u32) & 255) as usize];
    }
    (crc ^ 0xFFFF_FFFF) as i64
}

fn b18_sha256_4m(data: &[u8]) -> i64 {
    const K: [u32; 64] = [
        0x428a2f98,0x71374491,0xb5c0fbcf,0xe9b5dba5,0x3956c25b,0x59f111f1,0x923f82a4,0xab1c5ed5,
        0xd807aa98,0x12835b01,0x243185be,0x550c7dc3,0x72be5d74,0x80deb1fe,0x9bdc06a7,0xc19bf174,
        0xe49b69c1,0xefbe4786,0x0fc19dc6,0x240ca1cc,0x2de92c6f,0x4a7484aa,0x5cb0a9dc,0x76f988da,
        0x983e5152,0xa831c66d,0xb00327c8,0xbf597fc7,0xc6e00bf3,0xd5a79147,0x06ca6351,0x14292967,
        0x27b70a85,0x2e1b2138,0x4d2c6dfc,0x53380d13,0x650a7354,0x766a0abb,0x81c2c92e,0x92722c85,
        0xa2bfe8a1,0xa81a664b,0xc24b8b70,0xc76c51a3,0xd192e819,0xd6990624,0xf40e3585,0x106aa070,
        0x19a4c116,0x1e376c08,0x2748774c,0x34b0bcb5,0x391c0cb3,0x4ed8aa4a,0x5b9cca4f,0x682e6ff3,
        0x748f82ee,0x78a5636f,0x84c87814,0x8cc70208,0x90befffa,0xa4506ceb,0xbef9a3f7,0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
        0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];
    let compress = |h: &mut [u32; 8], blk: &[u8]| {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([blk[i*4], blk[i*4+1], blk[i*4+2], blk[i*4+3]]);
        }
        for i in 16..64 {
            let s0 = w[i-15].rotate_right(7) ^ w[i-15].rotate_right(18) ^ (w[i-15] >> 3);
            let s1 = w[i-2].rotate_right(17) ^ w[i-2].rotate_right(19) ^ (w[i-2] >> 10);
            w[i] = w[i-16].wrapping_add(s0).wrapping_add(w[i-7]).wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hv] = *h;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = hv.wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hv = g; g = f; f = e; e = d.wrapping_add(t1); d = c; c = b; b = a; a = t1.wrapping_add(t2);
        }
        h[0] = h[0].wrapping_add(a); h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c); h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e); h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g); h[7] = h[7].wrapping_add(hv);
    };
    let full = data.len() / 64;
    for b in 0..full { compress(&mut h, &data[b*64..(b+1)*64]); }
    let mut tail = Vec::with_capacity(128);
    tail.extend_from_slice(&data[full*64..]);
    tail.push(0x80);
    while tail.len() % 64 != 56 { tail.push(0); }
    tail.extend_from_slice(&((data.len() as u64) * 8).to_be_bytes());
    for chunk in tail.chunks_exact(64) { compress(&mut h, chunk); }
    (((h[0] as u64) << 32) | (h[1] as u64)) as i64
}

#[inline(always)]
fn qr(s: &mut [u32; 16], a: usize, b: usize, c: usize, d: usize) {
    s[a] = s[a].wrapping_add(s[b]); s[d] ^= s[a]; s[d] = s[d].rotate_left(16);
    s[c] = s[c].wrapping_add(s[d]); s[b] ^= s[c]; s[b] = s[b].rotate_left(12);
    s[a] = s[a].wrapping_add(s[b]); s[d] ^= s[a]; s[d] = s[d].rotate_left(8);
    s[c] = s[c].wrapping_add(s[d]); s[b] ^= s[c]; s[b] = s[b].rotate_left(7);
}

fn b19_chacha20_4m() -> i64 {
    let mut acc: u64 = 0;
    for blk in 0u32..65536 {
        let mut s: [u32; 16] = [
            0x61707865, 0x3320646e, 0x79622d32, 0x6b206574,
            0x03020100, 0x07060504, 0x0b0a0908, 0x0f0e0d0c,
            0x13121110, 0x17161514, 0x1b1a1918, 0x1f1e1d1c,
            blk, 0x09000000, 0x4a000000, 0x00000000,
        ];
        for _ in 0..10 {
            qr(&mut s, 0, 4, 8, 12); qr(&mut s, 1, 5, 9, 13); qr(&mut s, 2, 6, 10, 14); qr(&mut s, 3, 7, 11, 15);
            qr(&mut s, 0, 5, 10, 15); qr(&mut s, 1, 6, 11, 12); qr(&mut s, 2, 7, 8, 13); qr(&mut s, 3, 4, 9, 14);
        }
        acc ^= (s[0].wrapping_add(0x61707865) as u64) ^ (s[12].wrapping_add(blk) as u64);
    }
    acc as i64
}

fn b20_nbody_3d_256() -> i64 {
    let n = 256usize;
    let dt = 0.01f64;
    let mut px = vec![0.0f64; n]; let mut py = vec![0.0f64; n]; let mut pz = vec![0.0f64; n];
    let mut vx = vec![0.0f64; n]; let mut vy = vec![0.0f64; n]; let mut vz = vec![0.0f64; n];
    let mut pm = vec![0.0f64; n];
    for i in 0..n {
        let fi = (i + 1) as f64;
        px[i] = fi * 0.13; py[i] = fi * 0.27 - 10.0; pz[i] = fi * 0.09 - 5.0;
        pm[i] = 0.01 + ((i % 7) as f64) * 0.005;
    }
    for _ in 0..200 {
        for i in 0..n {
            let (xi, yi, zi) = (px[i], py[i], pz[i]);
            let (mut ax, mut ay, mut az) = (0.0, 0.0, 0.0);
            for j in 0..n {
                let dx = px[j] - xi; let dy = py[j] - yi; let dz = pz[j] - zi;
                let d2 = dx * dx + dy * dy + dz * dz + 0.01;
                let inv = 1.0 / (d2 * d2.sqrt());
                let s = pm[j] * inv;
                ax += dx * s; ay += dy * s; az += dz * s;
            }
            vx[i] += ax * dt; vy[i] += ay * dt; vz[i] += az * dt;
        }
        for i in 0..n { px[i] += vx[i] * dt; py[i] += vy[i] * dt; pz[i] += vz[i] * dt; }
    }
    let sum: f64 = (0..n).map(|i| px[i] + py[i] + pz[i]).sum();
    (sum * 100.0) as i64
}

fn b21_knapsack_dp_20m() -> i64 {
    let cap = 20_000usize;
    let mut dp = vec![0i64; cap + 1];
    for it in 0..1000usize {
        let w = 5 + ((it * 37 + 11) % 250);
        let v = (10 + ((it * 97 + 23) % 500)) as i64;
        for c in (w..=cap).rev() {
            let cand = dp[c - w] + v;
            if cand > dp[c] { dp[c] = cand; }
        }
    }
    dp[cap]
}

fn b22_mandelbrot_1k() -> i64 {
    let size = 1000usize;
    let inv = 1.0 / (size as f64);
    let mut total: i64 = 0;
    for y in 0..size {
        let cy = (y as f64) * 2.4 * inv - 1.2;
        for x in 0..size {
            let cx = (x as f64) * 3.0 * inv - 2.0;
            let (mut zr, mut zi) = (0.0f64, 0.0f64);
            let mut it = 0i64;
            while it < 256 {
                let zr2 = zr * zr; let zi2 = zi * zi;
                if zr2 + zi2 > 4.0 { break; }
                zi = 2.0 * zr * zi + cy;
                zr = zr2 - zi2 + cx;
                it += 1;
            }
            total += it;
        }
    }
    total
}

fn b23_fft_65536() -> i64 {
    let n = 65536usize;
    let mut re = vec![0.0f64; n];
    let mut im = vec![0.0f64; n];
    for i in 0..n {
        re[i] = ((i % 64) as f64) * 0.05 - 1.5;
        im[i] = ((i % 32) as f64) * 0.03 - 0.5;
    }
    // Bit-reversal permutation
    let mut j = 0usize;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 { j ^= bit; bit >>= 1; }
        j ^= bit;
        if i < j { re.swap(i, j); im.swap(i, j); }
    }
    let mut len = 2usize;
    while len <= n {
        let half = len >> 1;
        let ang = -2.0 * std::f64::consts::PI / (len as f64);
        let (wlen_r, wlen_i) = (ang.cos(), ang.sin());
        let mut i = 0;
        while i < n {
            let (mut wr, mut wi) = (1.0f64, 0.0f64);
            for k in 0..half {
                let u_r = re[i + k]; let u_i = im[i + k];
                let v_r = re[i + k + half] * wr - im[i + k + half] * wi;
                let v_i = re[i + k + half] * wi + im[i + k + half] * wr;
                re[i + k] = u_r + v_r; im[i + k] = u_i + v_i;
                re[i + k + half] = u_r - v_r; im[i + k + half] = u_i - v_i;
                let nwr = wr * wlen_r - wi * wlen_i;
                wi = wr * wlen_i + wi * wlen_r;
                wr = nwr;
            }
            i += len;
        }
        len <<= 1;
    }
    let mut mag_sum = 0.0f64;
    let mut i = 0;
    while i < n { mag_sum += (re[i] * re[i] + im[i] * im[i]).sqrt(); i += 64; }
    (mag_sum * 10.0) as i64
}

// --- Tier 4: Extreme / Impossible (B24..B30) ---

fn b24_fp32_gemm_512(a: &[f32], b: &[f32], c: &mut [f32]) -> i64 {
    let n = 512usize;
    c.fill(0.0);
    for i in 0..n {
        let ci = &mut c[i * n..(i + 1) * n];
        for k in 0..n {
            let aik = a[i * n + k];
            let bk = &b[k * n..(k + 1) * n];
            for j in 0..n { ci[j] += aik * bk[j]; }
        }
    }
    let mut sum = 0.0f64;
    let mut i = 0;
    while i < n * n { sum += c[i] as f64; i += 17; }
    (sum * 10.0) as i64
}

fn b25_fp64_gemm_512(a: &[f64], b: &[f64], c: &mut [f64]) -> i64 {
    let n = 512usize;
    c.fill(0.0);
    for i in 0..n {
        let ci = &mut c[i * n..(i + 1) * n];
        for k in 0..n {
            let aik = a[i * n + k];
            let bk = &b[k * n..(k + 1) * n];
            for j in 0..n { ci[j] += aik * bk[j]; }
        }
    }
    let mut sum = 0.0f64;
    let mut i = 0;
    while i < n * n { sum += c[i]; i += 17; }
    (sum * 10.0) as i64
}

fn b26_bnn_gemv_4096(w: &[u64], a: &[u64], out: &mut [i64]) -> i64 {
    let rows = 4096usize;
    let words = 64usize;
    let mut acc: i64 = 0;
    for _ in 0..8 {
        for r in 0..rows {
            let row = &w[r * words..(r + 1) * words];
            let mut pop = 0i64;
            for k in 0..words { pop += (row[k] ^ a[k]).count_ones() as i64; }
            out[r] = pop;
        }
        let mut i = 0;
        while i < rows { acc = acc.wrapping_add(out[i]); i += 16; }
    }
    acc
}

fn b27_q4_0_gemv_4096(act: &[f32], q4: &[u8], out: &mut [f32]) -> i64 {
    let rows = 4096usize;
    let nblk = 128usize;
    let mut sum = 0.0f64;
    for _ in 0..8 {
        for r in 0..rows {
            let mut acc = 0.0f32;
            let row_off = r * nblk * 20;
            for b in 0..nblk {
                let bo = row_off + b * 20;
                let scale = f32::from_le_bytes([q4[bo], q4[bo+1], q4[bo+2], q4[bo+3]]);
                let ab = &act[b * 32..(b + 1) * 32];
                let mut bs = 0.0f32;
                for j in 0..16 {
                    let byte = q4[bo + 4 + j];
                    let lo = (byte & 0x0F) as i32 - 8;
                    let hi = ((byte >> 4) & 0x0F) as i32 - 8;
                    bs += (lo as f32) * ab[j * 2] + (hi as f32) * ab[j * 2 + 1];
                }
                acc += bs * scale;
            }
            out[r] = acc;
        }
        let mut i = 0;
        while i < rows { sum += out[i] as f64; i += 16; }
    }
    (sum * 10.0) as i64
}

fn b28_q8_0_gemv_4096(act: &[f32], q8: &[u8], out: &mut [f32]) -> i64 {
    let rows = 4096usize;
    let nblk = 128usize;
    let mut sum = 0.0f64;
    for _ in 0..8 {
        for r in 0..rows {
            let mut acc = 0.0f32;
            let row_off = r * nblk * 36;
            for b in 0..nblk {
                let bo = row_off + b * 36;
                let scale = f32::from_le_bytes([q8[bo], q8[bo+1], q8[bo+2], q8[bo+3]]);
                let ab = &act[b * 32..(b + 1) * 32];
                let mut bs = 0.0f32;
                for j in 0..32 {
                    bs += (q8[bo + 4 + j] as i8 as f32) * ab[j];
                }
                acc += bs * scale;
            }
            out[r] = acc;
        }
        let mut i = 0;
        while i < rows { sum += out[i] as f64; i += 16; }
    }
    (sum * 10.0) as i64
}

fn b29_faiss_l2_10k_128d(vecs: &[f32], qrys: &[f32], dst: &mut [f32]) -> i64 {
    let mut sum = 0.0f64;
    for q in 0..32usize {
        let qry = &qrys[q * 128..(q + 1) * 128];
        for v in 0..10_000usize {
            let vp = &vecs[v * 128..(v + 1) * 128];
            let mut d = 0.0f32;
            for k in 0..128 { let df = vp[k] - qry[k]; d += df * df; }
            dst[v] = d;
        }
        let mut i = 0;
        while i < 10_000 { sum += dst[i] as f64; i += 64; }
    }
    (sum * 10.0) as i64
}

fn b30_llm_ffn_4096(x: &[f32], nw: &[f32], w1: &[f32], w2: &[f32]) -> i64 {
    let dim = 4096usize;
    let mut norm = vec![0.0f32; dim];
    let mut gate = vec![0.0f32; dim];
    let mut up = vec![0.0f32; dim];
    let mut act = vec![0.0f32; dim];
    let mut sum = 0.0f64;
    for _ in 0..4 {
        let ss: f32 = x.iter().map(|&v| v * v).sum();
        let inv = 1.0f32 / (ss / dim as f32 + 1e-5f32).sqrt();
        for i in 0..dim { norm[i] = x[i] * inv * nw[i]; }
        for r in 0..dim {
            let row1 = &w1[r * dim..(r + 1) * dim];
            let mut s1 = 0.0f32;
            for c in 0..dim { s1 += row1[c] * norm[c]; }
            gate[r] = s1;
        }
        for r in 0..dim {
            let row2 = &w2[r * dim..(r + 1) * dim];
            let mut s2 = 0.0f32;
            for c in 0..dim { s2 += row2[c] * norm[c]; }
            up[r] = s2;
        }
        for i in 0..dim {
            let g = gate[i];
            let silu = g / (1.0f32 + (-g).exp());
            act[i] = silu * up[i];
        }
        let mut i = 0;
        while i < dim { sum += act[i] as f64; i += 16; }
    }
    (sum * 10.0) as i64
}

fn main() {
    // --- Pre-allocate identical inputs ---
    let n_dot = 8_000_000usize;
    let a_dot: Vec<f64> = (0..n_dot).map(|i| ((i & 63) as f64) * 0.01 + 0.5).collect();
    let b_dot: Vec<f64> = (0..n_dot).map(|i| ((i & 31) as f64) * 0.02 + 0.25).collect();

    let n_p7 = 500_000usize;
    let mut p7_buf = vec![0u8; n_p7 * 11];
    for r in 0..n_p7 {
        let mut v = 1_000_000_000u64 + ((r as u64 * 7919 + 12345) % 899_999_999);
        let base = r * 11;
        for d in (0..10).rev() { p7_buf[base + d] = b'0' + (v % 10) as u8; v /= 10; }
        p7_buf[base + 10] = b'\n';
    }

    let n8m = 8_388_608usize;
    let data8m: Vec<u8> = (0..n8m).map(|i| ((i * 31 + 17) & 255) as u8).collect();
    let mut crc_tbl = [0u32; 256];
    for ci in 0..256u32 {
        let mut c = ci;
        for _ in 0..8 { c = if (c & 1) != 0 { (c >> 1) ^ 0xEDB8_8320 } else { c >> 1 }; }
        crc_tbl[ci as usize] = c;
    }

    let ns = 500_000usize;
    let mut orig_sort = Vec::with_capacity(ns);
    let mut rng: u64 = 0x1234_5678_9ABC_DEF0;
    for _ in 0..ns {
        rng = rng.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = rng;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        orig_sort.push(z as i64);
    }
    let mut work_sort = vec![0i64; ns];

    let n4m = 4_194_304usize;
    let mut hay_bytes = vec![b'a'; n4m];
    for k in 0..2000usize {
        let off = k * 2048 + 100;
        hay_bytes[off..off + 12].copy_from_slice(b"AXON_NEEDLE!");
    }
    let hay_str = String::from_utf8(hay_bytes).unwrap();

    let mut json_doc = String::with_capacity(4_500_000);
    for i in 0..100_000usize {
        use std::fmt::Write;
        let _ = writeln!(json_doc, "{{\"id\":{},\"score\":{},\"ok\":true}}", i, (i * 17 + 5) % 10000);
    }

    let b2m: Vec<u8> = (0..2_097_152usize).map(|i| ((i * 19 + 3) & 255) as u8).collect();

    let mut csv_buf = Vec::with_capacity(3_500_000);
    for i in 0..100_000usize {
        use std::io::Write;
        let _ = writeln!(csv_buf, "{},{},{},{}", i, (i * 13 + 7) % 10000, (i * 29 + 3) % 50000, (i * 7 + 1) % 1000);
    }

    let mut adj: Vec<Vec<(usize, i64)>> = vec![Vec::with_capacity(5); 20_000];
    for i in 0..20_000usize {
        for e in 1..=5usize {
            let dst = (i + e * 137 + (i % 19)) % 20_000;
            let wt = (1 + ((i * 31 + e * 17) % 100)) as i64;
            adj[i].push((dst, wt));
        }
    }

    let b4m: Vec<u8> = (0..n4m).map(|i| ((i * 31 + 7) & 255) as u8).collect();

    let n512 = 512 * 512;
    let a32: Vec<f32> = (0..n512).map(|i| (((i % 97) as f64) * 0.01) as f32).collect();
    let b32: Vec<f32> = (0..n512).map(|i| (((i % 89) as f64) * 0.01) as f32).collect();
    let mut c32 = vec![0.0f32; n512];

    let a64: Vec<f64> = (0..n512).map(|i| ((i % 97) as f64) * 0.01).collect();
    let b64: Vec<f64> = (0..n512).map(|i| ((i % 89) as f64) * 0.01).collect();
    let mut c64 = vec![0.0f64; n512];

    let a_bnn: Vec<u64> = (0..64u64).map(|i| (i.wrapping_add(1)).wrapping_mul(0x9E37_79B9_7F4A_7C15)).collect();
    let w_bnn: Vec<u64> = (0..(4096 * 64) as u64).map(|i| (i.wrapping_add(7)).wrapping_mul(0xBF58_476D_1CE4_E5B9)).collect();
    let mut o_bnn = vec![0i64; 4096];

    let act_q: Vec<f32> = (0..4096usize).map(|i| (((i % 31) as f64) * 0.02 - 0.3) as f32).collect();
    let mut q4 = vec![0u8; 4096 * 128 * 20];
    let mut q8 = vec![0u8; 4096 * 128 * 36];
    let sc4 = (0.05f32).to_le_bytes();
    let sc8 = (0.01f32).to_le_bytes();
    for r in 0..4096 * 128 {
        let o4 = r * 20;
        q4[o4..o4 + 4].copy_from_slice(&sc4);
        for b in 0..16 { q4[o4 + 4 + b] = ((r + b * 7) & 0xFF) as u8; }
        let o8 = r * 36;
        q8[o8..o8 + 4].copy_from_slice(&sc8);
        for b in 0..32 { q8[o8 + 4 + b] = ((((r + b * 5) % 251) as i32 - 125) & 0xFF) as u8; }
    }
    let mut out_q = vec![0.0f32; 4096];

    let vec_f: Vec<f32> = (0..10_000 * 128).map(|i| (((i % 53) as f64) * 0.02 - 0.5) as f32).collect();
    let qry_f: Vec<f32> = (0..32 * 128).map(|i| (((i % 47) as f64) * 0.02 - 0.4) as f32).collect();
    let mut dst_f = vec![0.0f32; 10_000];

    let x_llm: Vec<f32> = (0..4096).map(|i| (((i % 29) as f64) * 0.03 - 0.4) as f32).collect();
    let nw_llm: Vec<f32> = (0..4096).map(|i| (1.0 + ((i % 11) as f64) * 0.01) as f32).collect();
    let w1_llm: Vec<f32> = (0..4096 * 4096).map(|i| (((i % 19) as f64) * 0.002 - 0.018) as f32).collect();
    let w2_llm: Vec<f32> = (0..4096 * 4096).map(|i| (((i % 23) as f64) * 0.002 - 0.022) as f32).collect();

    let mut times_us = Vec::with_capacity(30);
    let mut chks = Vec::with_capacity(30);
    macro_rules! bench {
        ($e:expr) => {{
            let t0 = Instant::now();
            let c = black_box($e);
            let us = t0.elapsed().as_micros() as i64;
            times_us.push(us);
            chks.push(c);
        }};
    }

    bench!(b01_branchless_clamp_10m());
    bench!(b02_fib_rec_38());
    bench!(b03_sieve_1m());
    bench!(b04_popcnt_10m());
    bench!(b05_dot_f64_8m(&a_dot, &b_dot));
    bench!(b06_splitmix64_25m());
    bench!(b07_parse_i64_500k(&p7_buf, 11, n_p7));
    bench!(b08_fnv1a_8m(&data8m));

    bench!(b09_sort_500k(&orig_sort, &mut work_sort));
    bench!(b10_hashmap_500k());
    bench!(b11_mem_find_4m(&hay_str));
    bench!(b12_json_100k(&json_doc));
    bench!(b13_base64_2m(&b2m));
    bench!(b14_csv_100k_agg(&csv_buf));
    bench!(b15_spsc_ring_10m());
    bench!(b16_dijkstra_20k(&adj));

    bench!(b17_crc32_8m(&data8m, &crc_tbl));
    bench!(b18_sha256_4m(&b4m));
    bench!(b19_chacha20_4m());
    bench!(b20_nbody_3d_256());
    bench!(b21_knapsack_dp_20m());
    bench!(b22_mandelbrot_1k());
    bench!(b23_fft_65536());

    bench!(b24_fp32_gemm_512(&a32, &b32, &mut c32));
    bench!(b25_fp64_gemm_512(&a64, &b64, &mut c64));
    bench!(b26_bnn_gemv_4096(&w_bnn, &a_bnn, &mut o_bnn));
    bench!(b27_q4_0_gemv_4096(&act_q, &q4, &mut out_q));
    bench!(b28_q8_0_gemv_4096(&act_q, &q8, &mut out_q));
    bench!(b29_faiss_l2_10k_128d(&vec_f, &qry_f, &mut dst_f));
    bench!(b30_llm_ffn_4096(&x_llm, &nw_llm, &w1_llm, &w2_llm));

    print!("{{\"lang\":\"Rust\",\"times_us\":[");
    for (i, t) in times_us.iter().enumerate() {
        if i > 0 { print!(","); }
        print!("{}", t);
    }
    print!("],\"chks\":[");
    for (i, c) in chks.iter().enumerate() {
        if i > 0 { print!(","); }
        print!("{}", c);
    }
    println!("]}}");
}
