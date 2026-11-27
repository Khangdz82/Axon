// Rust 100-Benchmark Strict Light-to-Impossible Suite (`rustc -C opt-level=3 -C target-cpu=native`)
// Exact algorithmic & checksum parity with `axon_100_extreme.ax`, with `std::hint::black_box` anti-DCE.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, VecDeque};
use std::hint::black_box;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

// ==================== Domain 1: Scalar, Bitwise & Control (B001..B010) ====================

fn b001_branchless_clamp(n: i64) -> i64 {
    let n = black_box(n);
    let mut acc: i64 = 0;
    for i in 0..n {
        let x = (i.wrapping_mul(37).wrapping_add(13)) & 1023;
        acc = acc.wrapping_add(x.clamp(100, 900));
    }
    acc
}

fn fib_rec(n: i64) -> i64 {
    if n <= 1 { return n; }
    fib_rec(n - 1) + fib_rec(n - 2)
}

fn b002_fib_rec() -> i64 { fib_rec(black_box(37)) }

fn b003_sieve(n: usize) -> i64 {
    let n = black_box(n);
    let mut is_p = vec![1u8; n + 1];
    is_p[0] = 0; is_p[1] = 0;
    let mut i = 2usize;
    while i * i <= n {
        if is_p[i] != 0 {
            let mut j = i * i;
            while j <= n { is_p[j] = 0; j += i; }
        }
        i += 1;
    }
    is_p.iter().map(|&b| b as i64).sum()
}

fn b004_popcnt(n: i64) -> i64 {
    let n = black_box(n);
    let mut acc: i64 = 0;
    for i in 0..n {
        let v = (i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(0xBF58_476D_1CE4_E5B9);
        acc = acc.wrapping_add(v.count_ones() as i64);
    }
    acc
}

fn b005_splitmix64(n: i64) -> i64 {
    let n = black_box(n);
    let mut acc: u64 = 0;
    for i in 0..n {
        let mut z = (i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(0x1234_5678_9ABC_DEF0);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z = z ^ (z >> 31);
        acc ^= z;
    }
    acc as i64
}

fn b006_collatz(n: i64) -> i64 {
    let n = black_box(n);
    let mut total: i64 = 0;
    for s in 1..=n {
        let mut x = s;
        let mut steps: i64 = 0;
        while x != 1 {
            if (x & 1) == 0 { x >>= 1; } else { x = x * 3 + 1; }
            steps += 1;
        }
        total = total.wrapping_add(steps);
    }
    total
}

fn b007_gcd_euclid(n: i64) -> i64 {
    let n = black_box(n);
    let mut acc: i64 = 0;
    for i in 1..=n {
        let mut a = (i.wrapping_mul(2654435761) & 0xFFFFF) + 1;
        let mut b = (i.wrapping_mul(2246822519) & 0xFFFFF) + 1;
        while b != 0 { let t = a % b; a = b; b = t; }
        acc = acc.wrapping_add(a);
    }
    acc
}

#[inline(always)]
fn part1by2(mut x: u64) -> u64 {
    x &= 0x3FF;
    x = (x ^ (x << 16)) & 0xFF0000FF;
    x = (x ^ (x << 8)) & 0x0300F00F;
    x = (x ^ (x << 4)) & 0x030C30C3;
    x = (x ^ (x << 2)) & 0x09249249;
    x
}

fn b008_morton3d(n: i64) -> i64 {
    let n = black_box(n);
    let mut acc: u64 = 0;
    for i in 0..n {
        let u = i as u64;
        let x = (u.wrapping_mul(13) + 5) & 1023;
        let y = (u.wrapping_mul(29) + 11) & 1023;
        let z = (u.wrapping_mul(47) + 19) & 1023;
        let code = part1by2(x) | (part1by2(y) << 1) | (part1by2(z) << 2);
        acc = acc.wrapping_add(code);
    }
    acc as i64
}

fn b009_modexp(n: i64) -> i64 {
    let n = black_box(n);
    let m: i64 = 1_000_000_007;
    let mut acc: i64 = 0;
    for i in 1..=n {
        let mut base = (i * 31 + 17) % m;
        let mut exp = (i * 19 + 1234567) & 0x3FFFFFFF;
        let mut res: i64 = 1;
        while exp > 0 {
            if (exp & 1) != 0 { res = (res * base) % m; }
            base = (base * base) % m;
            exp >>= 1;
        }
        acc = acc.wrapping_add(res);
    }
    acc
}

fn b010_mat4x4_int(a_arr: &[i64], b_arr: &[i64], n: usize) -> i64 {
    let mut acc: i64 = 0;
    for k in 0..n {
        let a = &a_arr[k * 16..(k + 1) * 16];
        let b = &b_arr[k * 16..(k + 1) * 16];
        let mut c00 = 0i64; let mut c11 = 0i64; let mut c22 = 0i64; let mut c33 = 0i64;
        for m in 0..4 {
            c00 = c00.wrapping_add(a[m] * b[m * 4]);
            c11 = c11.wrapping_add(a[4 + m] * b[m * 4 + 1]);
            c22 = c22.wrapping_add(a[8 + m] * b[m * 4 + 2]);
            c33 = c33.wrapping_add(a[12 + m] * b[m * 4 + 3]);
        }
        let mut s = c00.wrapping_add(c11).wrapping_add(c22).wrapping_add(c33);
        for r in 0..4 {
            for c in 0..4 {
                let v = a[r*4]*b[c] + a[r*4+1]*b[4+c] + a[r*4+2]*b[8+c] + a[r*4+3]*b[12+c];
                s = s.wrapping_add(v ^ (r as i64 * 7 + c as i64));
            }
        }
        acc = acc.wrapping_add(s);
    }
    acc
}

// ==================== Domain 2: SIMD Array & Reduction Math (B011..B020) ====================

fn b011_dot_f64(a: &[f64], b: &[f64]) -> i64 {
    let mut s = 0.0f64;
    for i in 0..a.len() { s += a[i] * b[i]; }
    (black_box(s) * 1000.0) as i64
}

fn b012_saxpy_f32(x: &[f32], y_orig: &[f32]) -> i64 {
    let mut y = y_orig.to_vec();
    let alpha = black_box(2.5f32);
    for i in 0..x.len() { y[i] = alpha * x[i] + y[i]; }
    let mut s = 0.0f64;
    for &v in &y { s += v as f64; }
    (s * 100.0) as i64
}

fn b013_prefix_sum_i64(src: &[i64]) -> i64 {
    let n = src.len();
    let mut out = vec![0i64; n];
    let mut acc: i64 = 0;
    for i in 0..n { acc = acc.wrapping_add(src[i]); out[i] = acc; }
    black_box(out[0]) ^ out[n / 2] ^ out[n - 1]
}

fn b014_histogram_256(data: &[u8]) -> i64 {
    let mut hist = [0i64; 256];
    for &b in data { hist[b as usize] += 1; }
    let mut acc: i64 = 0;
    for i in 0..256 { acc = acc.wrapping_add(hist[i].wrapping_mul((i + 1) as i64)); }
    acc
}

fn b015_minmax_i64(a: &[i64]) -> i64 {
    let mut mn = i64::MAX;
    let mut mx = i64::MIN;
    let mut s: i64 = 0;
    for &v in a {
        if v < mn { mn = v; }
        if v > mx { mx = v; }
        s = s.wrapping_add(v);
    }
    s ^ mn ^ mx
}

fn b016_relu6_f32(x: &[f32]) -> i64 {
    let mut s = 0.0f64;
    for &v in x {
        let r = if v < 0.0 { 0.0 } else if v > 6.0 { 6.0 } else { v };
        s += r as f64;
    }
    (black_box(s) * 100.0) as i64
}

fn b017_l1_l2_f64(a: &[f64]) -> i64 {
    let mut l1 = 0.0f64;
    let mut l2 = 0.0f64;
    for &v in a { l1 += v.abs(); l2 += v * v; }
    ((black_box(l1) + l2.sqrt()) * 100.0) as i64
}

fn b018_conv1d_k9_f32(x: &[f32]) -> i64 {
    let n = x.len();
    let k: [f32; 9] = [0.05, 0.1, 0.12, 0.15, 0.16, 0.15, 0.12, 0.1, 0.05];
    let mut out = vec![0.0f32; n - 8];
    for i in 0..(n - 8) {
        let mut s = 0.0f32;
        for t in 0..9 { s += x[i + t] * k[t]; }
        out[i] = s;
    }
    let sum: f64 = out.iter().map(|&v| v as f64).sum();
    (black_box(sum) * 100.0) as i64
}

fn b019_variance_f64(a: &[f64]) -> i64 {
    let n = a.len() as f64;
    let mut sum = 0.0f64;
    for &v in a { sum += v; }
    let mean = sum / n;
    let mut var = 0.0f64;
    for &v in a { let d = v - mean; var += d * d; }
    ((black_box(var) / n) * 1_000_000.0) as i64
}

fn b020_mat_transpose_512(src: &[f64], n: usize) -> i64 {
    let mut dst = vec![0.0f64; n * n];
    for r in 0..n {
        for c in 0..n { dst[c * n + r] = src[r * n + c]; }
    }
    let mut s = 0.0f64;
    for i in (0..(n * n)).step_by(257) { s += dst[i]; }
    (black_box(s) * 1000.0) as i64
}

// ==================== Domain 3: Parsing, Text & Serialization (B021..B030) ====================

fn b021_parse_i64(buf: &[u8], stride: usize, cnt: usize) -> i64 {
    let mut acc: i64 = 0;
    for r in 0..cnt {
        let p = r * stride;
        let mut v: i64 = 0;
        for k in 0..10 { v = v * 10 + (buf[p + k] - b'0') as i64; }
        acc = acc.wrapping_add(v);
    }
    acc
}

fn b022_mem_find(hay: &[u8]) -> i64 {
    let needle = b"AXON_NEEDLE!";
    let mut pos = 0usize;
    let mut acc: i64 = 0;
    let mut hits: i64 = 0;
    while pos + needle.len() <= hay.len() {
        if let Some(rel) = hay[pos..].windows(needle.len()).position(|w| w == needle) {
            let idx = pos + rel;
            acc = acc.wrapping_add(idx as i64);
            hits += 1;
            pos = idx + needle.len();
        } else { break; }
    }
    acc ^ (hits << 32)
}

fn b023_json_parse(data: &[u8]) -> i64 {
    let n = data.len();
    let mut pos = 0usize;
    let mut acc: i64 = 0;
    while pos < n {
        while pos < n && data[pos] != b':' { pos += 1; }
        if pos >= n { break; }
        pos += 1;
        let mut id: i64 = 0;
        while pos < n && data[pos] >= b'0' && data[pos] <= b'9' { id = id * 10 + (data[pos] - b'0') as i64; pos += 1; }
        while pos < n && data[pos] != b':' { pos += 1; }
        if pos >= n { break; }
        pos += 1;
        let mut val: i64 = 0;
        while pos < n && data[pos] >= b'0' && data[pos] <= b'9' { val = val * 10 + (data[pos] - b'0') as i64; pos += 1; }
        while pos < n && data[pos] != b'\n' { pos += 1; }
        pos += 1;
        acc = acc.wrapping_add(id).wrapping_add(val);
    }
    acc
}

fn b024_base64_rt(raw: &[u8]) -> i64 {
    const ENC: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut dec_tbl = [0u8; 256];
    for i in 0..64 { dec_tbl[ENC[i] as usize] = i as u8; }
    let n = raw.len();
    let mut enc = vec![0u8; ((n + 2) / 3) * 4];
    let mut ei = 0usize;
    let mut i = 0usize;
    while i + 3 <= n {
        let v = ((raw[i] as u32) << 16) | ((raw[i+1] as u32) << 8) | (raw[i+2] as u32);
        enc[ei] = ENC[((v >> 18) & 63) as usize];
        enc[ei+1] = ENC[((v >> 12) & 63) as usize];
        enc[ei+2] = ENC[((v >> 6) & 63) as usize];
        enc[ei+3] = ENC[(v & 63) as usize];
        i += 3; ei += 4;
    }
    let mut dec = vec![0u8; n];
    let mut di = 0usize;
    let mut pi = 0usize;
    while pi < ei {
        let v = ((dec_tbl[enc[pi] as usize] as u32) << 18)
              | ((dec_tbl[enc[pi+1] as usize] as u32) << 12)
              | ((dec_tbl[enc[pi+2] as usize] as u32) << 6)
              | (dec_tbl[enc[pi+3] as usize] as u32);
        dec[di] = ((v >> 16) & 255) as u8;
        dec[di+1] = ((v >> 8) & 255) as u8;
        dec[di+2] = (v & 255) as u8;
        di += 3; pi += 4;
    }
    while di < n { dec[di] = raw[di]; di += 1; }
    let mut acc: i64 = n as i64;
    for k in (0..n).step_by(64) { acc = acc.wrapping_add(dec[k] as i64); }
    acc
}

fn b025_csv_agg(data: &[u8]) -> i64 {
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

fn b026_hex_rt(raw: &[u8]) -> i64 {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut dtbl = [0u8; 256];
    for i in 0..10 { dtbl[(b'0' + i) as usize] = i; }
    for i in 0..6 { dtbl[(b'a' + i) as usize] = 10 + i; }
    let n = raw.len();
    let mut enc = vec![0u8; n * 2];
    for i in 0..n {
        let b = raw[i];
        enc[i * 2] = HEX[(b >> 4) as usize];
        enc[i * 2 + 1] = HEX[(b & 15) as usize];
    }
    let mut dec = vec![0u8; n];
    for i in 0..n {
        dec[i] = (dtbl[enc[i * 2] as usize] << 4) | dtbl[enc[i * 2 + 1] as usize];
    }
    let mut acc: i64 = 0;
    for i in (0..n).step_by(32) { acc = acc.wrapping_add(dec[i] as i64); }
    acc
}

fn b027_utf8_count(data: &[u8]) -> i64 {
    let mut cps: i64 = 0;
    let mut ascii: i64 = 0;
    for &b in data {
        if (b & 0xC0) != 0x80 { cps += 1; }
        if b < 0x80 { ascii += 1; }
    }
    cps ^ (ascii << 1)
}

fn b028_rle_rt(data: &[u8]) -> i64 {
    let n = data.len();
    let chunk = 65536usize;
    let mut acc: i64 = 0;
    let mut enc = vec![0u8; chunk * 2];
    let mut dec = vec![0u8; chunk];
    for c0 in (0..n).step_by(chunk) {
        let c1 = (c0 + chunk).min(n);
        let mut ei = 0usize;
        let mut i = c0;
        while i < c1 {
            let v = data[i];
            let mut run = 1usize;
            while i + run < c1 && run < 255 && data[i + run] == v { run += 1; }
            enc[ei] = run as u8;
            enc[ei + 1] = v;
            ei += 2;
            i += run;
        }
        let mut di = 0usize;
        let mut p = 0usize;
        while p < ei {
            let cnt = enc[p] as usize;
            let v = enc[p + 1];
            for _ in 0..cnt { dec[di] = v; di += 1; }
            p += 2;
        }
        acc = acc.wrapping_add(ei as i64).wrapping_add(dec[0] as i64).wrapping_add(dec[di - 1] as i64);
    }
    acc
}

fn b029_varint_pb(vals: &[u64]) -> i64 {
    let n = vals.len();
    let blk = 16384usize;
    let mut acc: i64 = 0;
    let mut buf = vec![0u8; blk * 10];
    for b0 in (0..n).step_by(blk) {
        let b1 = (b0 + blk).min(n);
        let mut pos = 0usize;
        for &val in &vals[b0..b1] {
            let mut v = val;
            while v >= 0x80 {
                buf[pos] = ((v & 0x7F) as u8) | 0x80;
                pos += 1;
                v >>= 7;
            }
            buf[pos] = v as u8;
            pos += 1;
        }
        let mut p = 0usize;
        while p < pos {
            let mut res: u64 = 0;
            let mut sh = 0u32;
            loop {
                let b = buf[p]; p += 1;
                res |= ((b & 0x7F) as u64) << sh;
                if (b & 0x80) == 0 { break; }
                sh += 7;
            }
            acc = acc.wrapping_add(res as i64);
        }
    }
    acc
}

fn b030_url_query(data: &[u8]) -> i64 {
    let n = data.len();
    let mut i = 0usize;
    let mut acc: i64 = 0;
    while i < n {
        while i < n && data[i] != b'=' && data[i] != b'\n' { i += 1; }
        if i >= n { break; }
        if data[i] == b'\n' { i += 1; continue; }
        i += 1;
        let mut v: i64 = 0;
        while i < n && data[i] >= b'0' && data[i] <= b'9' { v = v * 10 + (data[i] - b'0') as i64; i += 1; }
        acc = acc.wrapping_add(v);
        if i < n && (data[i] == b'&' || data[i] == b'\n') { i += 1; }
    }
    acc
}

// ==================== Domain 4: Sorting & Data Structures (B031..B040) ====================

fn b031_sort_i64(orig: &[i64]) -> i64 {
    let mut work = orig.to_vec();
    work.sort_unstable();
    let n = work.len();
    work[0] ^ work[n / 4] ^ work[n / 2] ^ work[n * 3 / 4] ^ work[n - 1]
}

fn b032_hashmap(n: i64) -> i64 {
    let mut m: HashMap<i64, i64> = HashMap::with_capacity(64);
    for i in 0..n { m.insert(i * 13 + 7, i * 3 + 1); }
    let mut acc: i64 = 0;
    for i in 0..n { acc = acc.wrapping_add(*m.get(&(i * 13 + 7)).unwrap_or(&0)); }
    acc
}

#[repr(align(64))]
struct PadAtom(AtomicUsize);

fn b033_spsc_ring(total: i64) -> i64 {
    let cap = 1024usize;
    let mask = cap - 1;
    let head = PadAtom(AtomicUsize::new(0));
    let tail = PadAtom(AtomicUsize::new(0));
    let mut buf = vec![0i64; cap];
    let mut src = [0i64; 256];
    let mut dst = [0i64; 256];
    let mut acc: i64 = 0;
    let mut base: i64 = 0;
    while base < total {
        for j in 0..256 { src[j] = base + j as i64; }
        let h = head.0.load(Ordering::Relaxed);
        let t = tail.0.load(Ordering::Acquire);
        let n = 256.min(cap - h.wrapping_sub(t));
        for j in 0..n { buf[(h.wrapping_add(j)) & mask] = src[j]; }
        head.0.store(h.wrapping_add(n), Ordering::Release);
        let t2 = tail.0.load(Ordering::Relaxed);
        let h2 = head.0.load(Ordering::Acquire);
        let got = 256.min(h2.wrapping_sub(t2));
        for j in 0..got { dst[j] = buf[(t2.wrapping_add(j)) & mask]; }
        tail.0.store(t2.wrapping_add(got), Ordering::Release);
        for j in 0..got { acc = acc.wrapping_add(dst[j]); }
        base += 256;
    }
    acc
}

fn b034_bsearch(sorted: &[i64], queries: &[i64]) -> i64 {
    let mut acc: i64 = 0;
    for &q in queries {
        let mut lo = 0usize;
        let mut hi = sorted.len();
        while lo < hi {
            let mid = lo + ((hi - lo) >> 1);
            if sorted[mid] < q { lo = mid + 1; } else { hi = mid; }
        }
        acc = acc.wrapping_add(lo as i64);
    }
    acc
}

fn b035_dsu(shards: usize, n: usize, ops: usize) -> i64 {
    let mut par = vec![0usize; n];
    let mut rnk = vec![0u8; n];
    let mut acc: i64 = 0;
    for sh in 0..shards {
        for i in 0..n { par[i] = i; rnk[i] = 0; }
        for i in 0..ops {
            let mut a = (i.wrapping_mul(37) + sh * 13 + 11) % n;
            let mut b = (i.wrapping_mul(97) + sh * 29 + 23) % n;
            while par[a] != a { par[a] = par[par[a]]; a = par[a]; }
            while par[b] != b { par[b] = par[par[b]]; b = par[b]; }
            if a != b {
                if rnk[a] < rnk[b] { par[a] = b; }
                else { par[b] = a; if rnk[a] == rnk[b] { rnk[a] += 1; } }
                acc += 1;
            }
        }
    }
    acc
}

fn b036_fenwick(shards: usize, n: usize, ops: usize) -> i64 {
    let mut bit = vec![0i64; n + 1];
    let mut acc: i64 = 0;
    for sh in 0..shards {
        bit.fill(0);
        for i in 1..=ops {
            let mut idx = ((i * 37 + sh * 11) % n) + 1;
            let delta = ((i + sh) & 15) as i64 + 1;
            while idx <= n { bit[idx] = bit[idx].wrapping_add(delta); idx += idx & idx.wrapping_neg(); }
            let mut q = ((i * 53 + sh * 19) % n) + 1;
            let mut s: i64 = 0;
            while q > 0 { s = s.wrapping_add(bit[q]); q -= q & q.wrapping_neg(); }
            acc = acc.wrapping_add(s);
        }
    }
    acc
}

fn b037_topk_heap(arr: &[i64], k: usize) -> i64 {
    let mut pq: BinaryHeap<Reverse<i64>> = BinaryHeap::with_capacity(k + 1);
    for &v in arr {
        if pq.len() < k { pq.push(Reverse(v)); }
        else if let Some(&Reverse(top)) = pq.peek() {
            if v > top { pq.pop(); pq.push(Reverse(v)); }
        }
    }
    pq.into_iter().map(|Reverse(v)| v).fold(0i64, i64::wrapping_add)
}

fn b038_bitset_popcnt(a: &[u64], b: &[u64], c: &[u64]) -> i64 {
    let mut cnt: i64 = 0;
    for i in 0..a.len() {
        cnt += ((a[i] & b[i]) ^ c[i]).count_ones() as i64;
    }
    cnt
}

fn b039_bloom_filter(bits: &[u64], queries: &[u64]) -> i64 {
    let mask = (bits.len() * 64 - 1) as u64;
    let mut hits: i64 = 0;
    for &q in queries {
        let h1 = q.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        let h2 = (h1 ^ (h1 >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        let i0 = (h1 & mask) as usize;
        let i1 = (h2 & mask) as usize;
        let i2 = ((h1.wrapping_add(h2)) & mask) as usize;
        let b0 = (bits[i0 >> 6] >> (i0 & 63)) & 1;
        let b1 = (bits[i1 >> 6] >> (i1 & 63)) & 1;
        let b2 = (bits[i2 >> 6] >> (i2 & 63)) & 1;
        hits += (b0 & b1 & b2) as i64;
    }
    hits
}

fn b040_radix_hist_scatter(arr: &[i64]) -> i64 {
    let n = arr.len();
    let mut cnt = [0usize; 16];
    for &v in arr { cnt[(v & 15) as usize] += 1; }
    let mut off = [0usize; 16];
    let mut sum = 0usize;
    for b in 0..16 { off[b] = sum; sum += cnt[b]; }
    let mut out = vec![0i64; n];
    for &v in arr {
        let b = (v & 15) as usize;
        out[off[b]] = v;
        off[b] += 1;
    }
    out[0] ^ out[n / 2] ^ out[n - 1]
}

// ==================== Domain 5: Graphs & Dynamic Programming (B041..B050) ====================

fn b041_dijkstra(adj: &[Vec<(usize, i64)>]) -> i64 {
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
            if nd < dist[v] { dist[v] = nd; pq.push(Reverse((nd, v))); }
        }
    }
    dist.iter().filter(|&&d| d < 1_000_000_000_000).copied().fold(0i64, i64::wrapping_add)
}

fn b042_bfs(adj: &[Vec<usize>]) -> i64 {
    let n = adj.len();
    let mut dist = vec![-1i64; n];
    let mut q = VecDeque::with_capacity(n);
    dist[0] = 0;
    q.push_back(0usize);
    while let Some(u) = q.pop_front() {
        let nd = dist[u] + 1;
        for &v in &adj[u] {
            if dist[v] < 0 { dist[v] = nd; q.push_back(v); }
        }
    }
    dist.iter().filter(|&&d| d >= 0).copied().fold(0i64, i64::wrapping_add)
}

fn b043_toposort(adj: &[Vec<usize>]) -> i64 {
    let n = adj.len();
    let mut indeg = vec![0i32; n];
    for u in 0..n { for &v in &adj[u] { indeg[v] += 1; } }
    let mut q = VecDeque::with_capacity(n);
    for i in 0..n { if indeg[i] == 0 { q.push_back(i); } }
    let mut acc: i64 = 0;
    let mut rank: i64 = 1;
    while let Some(u) = q.pop_front() {
        acc = acc.wrapping_add((u as i64).wrapping_mul(rank));
        rank += 1;
        for &v in &adj[u] {
            indeg[v] -= 1;
            if indeg[v] == 0 { q.push_back(v); }
        }
    }
    acc
}

fn b044_floyd_warshall(n: usize) -> i64 {
    let inf: i64 = 1_000_000_000;
    let mut d = vec![inf; n * n];
    for i in 0..n {
        d[i * n + i] = 0;
        for j in 0..n {
            if i != j && ((i * 17 + j * 31) % 7) == 0 {
                d[i * n + j] = 1 + ((i * 13 + j * 7) % 50) as i64;
            }
        }
    }
    for k in 0..n {
        let kn = k * n;
        for i in 0..n {
            let dik = d[i * n + k];
            if dik >= inf { continue; }
            let in_off = i * n;
            for j in 0..n {
                let cand = dik + d[kn + j];
                if cand < d[in_off + j] { d[in_off + j] = cand; }
            }
        }
    }
    d.iter().filter(|&&v| v < inf).copied().fold(0i64, i64::wrapping_add)
}

fn b045_pagerank(row_ptr: &[usize], col_idx: &[usize], out_deg: &[f64], n: usize) -> i64 {
    let mut pr = vec![1.0 / (n as f64); n];
    let mut nxt = vec![0.0f64; n];
    let damp = 0.85f64;
    let base = (1.0 - damp) / (n as f64);
    for _ in 0..15 {
        for r in 0..n {
            let mut s = 0.0f64;
            for idx in row_ptr[r]..row_ptr[r + 1] {
                let c = col_idx[idx];
                s += pr[c] / out_deg[c];
            }
            nxt[r] = base + damp * s;
        }
        std::mem::swap(&mut pr, &mut nxt);
    }
    let mut acc = 0.0f64;
    for i in 0..n { acc += pr[i] * ((i % 97) as f64 + 1.0); }
    (acc * 1_000_000.0) as i64
}

fn b046_knapsack_dp() -> i64 {
    let cap = 10_000usize;
    let mut dp = vec![0i64; cap + 1];
    for it in 0..200usize {
        let w = 5 + ((it * 37 + 11) % 250);
        let v = (10 + ((it * 97 + 23) % 500)) as i64;
        for c in (w..=cap).rev() {
            let cand = dp[c - w] + v;
            if cand > dp[c] { dp[c] = cand; }
        }
    }
    dp[cap]
}

fn b047_edit_dist(pairs: usize, len: usize) -> i64 {
    let mut total: i64 = 0;
    let mut a = vec![0u8; len];
    let mut b = vec![0u8; len];
    let mut prev = vec![0i64; len + 1];
    let mut curr = vec![0i64; len + 1];
    for p in 0..pairs {
        for i in 0..len {
            a[i] = ((p * 7 + i * 3) % 26) as u8;
            b[i] = ((p * 11 + i * 5 + 1) % 26) as u8;
        }
        for j in 0..=len { prev[j] = j as i64; }
        for i in 1..=len {
            curr[0] = i as i64;
            let ca = a[i - 1];
            for j in 1..=len {
                let cost = if ca == b[j - 1] { 0 } else { 1 };
                let d1 = prev[j] + 1;
                let d2 = curr[j - 1] + 1;
                let d3 = prev[j - 1] + cost;
                curr[j] = d1.min(d2).min(d3);
            }
            std::mem::swap(&mut prev, &mut curr);
        }
        total += prev[len];
    }
    total
}

fn b048_lcs_dp(pairs: usize, len: usize) -> i64 {
    let mut total: i64 = 0;
    let mut a = vec![0u8; len];
    let mut b = vec![0u8; len];
    let mut prev = vec![0i64; len + 1];
    let mut curr = vec![0i64; len + 1];
    for p in 0..pairs {
        for i in 0..len {
            a[i] = ((p * 5 + i * 3) % 12) as u8;
            b[i] = ((p * 7 + i * 4 + 2) % 12) as u8;
        }
        prev.fill(0);
        for i in 1..=len {
            curr[0] = 0;
            let ca = a[i - 1];
            for j in 1..=len {
                if ca == b[j - 1] { curr[j] = prev[j - 1] + 1; }
                else { curr[j] = prev[j].max(curr[j - 1]); }
            }
            std::mem::swap(&mut prev, &mut curr);
        }
        total += prev[len];
    }
    total
}

fn b049_coin_change(queries: usize, cap: usize) -> i64 {
    let inf: i64 = 1_000_000_000;
    let mut total: i64 = 0;
    let mut dp = vec![inf; cap + 1];
    for q in 0..queries {
        dp.fill(inf);
        dp[0] = 0;
        for c_idx in 0..32usize {
            let coin = 1 + ((c_idx * 17 + q * 3) % 97);
            for a in coin..=cap {
                let cand = dp[a - coin] + 1;
                if cand < dp[a] { dp[a] = cand; }
            }
        }
        total += dp[cap];
    }
    total
}

fn b050_lis_patience(seqs: usize, len: usize) -> i64 {
    let mut total: i64 = 0;
    let mut tails = Vec::with_capacity(len);
    for s in 0..seqs {
        tails.clear();
        let mut x: u64 = (s as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(1337);
        for _ in 0..len {
            x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let v = (x >> 33) as i64;
            let mut lo = 0usize;
            let mut hi = tails.len();
            while lo < hi {
                let mid = (lo + hi) >> 1;
                if tails[mid] < v { lo = mid + 1; } else { hi = mid; }
            }
            if lo == tails.len() { tails.push(v); } else { tails[lo] = v; }
        }
        total += tails.len() as i64;
    }
    total
}

// ==================== Domain 6: Cryptography & Hashing (B051..B060) ====================

fn b051_fnv1a_batch(data: &[u8]) -> i64 {
    let mut acc: u64 = 0;
    for chunk in data.chunks_exact(512) {
        let mut h: u64 = 0xCBF2_9CE4_8422_2325;
        for &b in chunk { h = (h ^ b as u64).wrapping_mul(0x0000_0100_0000_01B3); }
        acc ^= h;
    }
    acc as i64
}

fn b052_crc32(data: &[u8], tbl: &[u32; 256]) -> i64 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &b in data { crc = (crc >> 8) ^ tbl[((crc ^ b as u32) & 255) as usize]; }
    (crc ^ 0xFFFF_FFFF) as i64
}

const SHA256_K: [u32; 64] = [
    0x428a2f98,0x71374491,0xb5c0fbcf,0xe9b5dba5,0x3956c25b,0x59f111f1,0x923f82a4,0xab1c5ed5,
    0xd807aa98,0x12835b01,0x243185be,0x550c7dc3,0x72be5d74,0x80deb1fe,0x9bdc06a7,0xc19bf174,
    0xe49b69c1,0xefbe4786,0x0fc19dc6,0x240ca1cc,0x2de92c6f,0x4a7484aa,0x5cb0a9dc,0x76f988da,
    0x983e5152,0xa831c66d,0xb00327c8,0xbf597fc7,0xc6e00bf3,0xd5a79147,0x06ca6351,0x14292967,
    0x27b70a85,0x2e1b2138,0x4d2c6dfc,0x53380d13,0x650a7354,0x766a0abb,0x81c2c92e,0x92722c85,
    0xa2bfe8a1,0xa81a664b,0xc24b8b70,0xc76c51a3,0xd192e819,0xd6990624,0xf40e3585,0x106aa070,
    0x19a4c116,0x1e376c08,0x2748774c,0x34b0bcb5,0x391c0cb3,0x4ed8aa4a,0x5b9cca4f,0x682e6ff3,
    0x748f82ee,0x78a5636f,0x84c87814,0x8cc70208,0x90befffa,0xa4506ceb,0xbef9a3f7,0xc67178f2,
];

fn sha256_digest_words(data: &[u8]) -> [u32; 8] {
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
        0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];
    let compress = |h: &mut [u32; 8], blk: &[u8]| {
        let mut w = [0u32; 64];
        for i in 0..16 { w[i] = u32::from_be_bytes([blk[i*4], blk[i*4+1], blk[i*4+2], blk[i*4+3]]); }
        for i in 16..64 {
            let s0 = w[i-15].rotate_right(7) ^ w[i-15].rotate_right(18) ^ (w[i-15] >> 3);
            let s1 = w[i-2].rotate_right(17) ^ w[i-2].rotate_right(19) ^ (w[i-2] >> 10);
            w[i] = w[i-16].wrapping_add(s0).wrapping_add(w[i-7]).wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hv] = *h;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = hv.wrapping_add(s1).wrapping_add(ch).wrapping_add(SHA256_K[i]).wrapping_add(w[i]);
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
    let mut tail = [0u8; 128];
    let rem = data.len() - full * 64;
    tail[..rem].copy_from_slice(&data[full*64..]);
    tail[rem] = 0x80;
    let tlen = if rem + 1 <= 56 { 64 } else { 128 };
    let bitlen = ((data.len() as u64) * 8).to_be_bytes();
    tail[tlen - 8..tlen].copy_from_slice(&bitlen);
    compress(&mut h, &tail[..64]);
    if tlen == 128 { compress(&mut h, &tail[64..128]); }
    h
}

fn b053_sha256_batch(data: &[u8]) -> i64 {
    let mut acc: u64 = 0;
    for blk in data.chunks_exact(16384) {
        let h = sha256_digest_words(blk);
        acc ^= ((h[0] as u64) << 32) | (h[1] as u64);
    }
    acc as i64
}

#[inline(always)]
fn chacha_qr(s: &mut [u32; 16], a: usize, b: usize, c: usize, d: usize) {
    s[a] = s[a].wrapping_add(s[b]); s[d] ^= s[a]; s[d] = s[d].rotate_left(16);
    s[c] = s[c].wrapping_add(s[d]); s[b] ^= s[c]; s[b] = s[b].rotate_left(12);
    s[a] = s[a].wrapping_add(s[b]); s[d] ^= s[a]; s[d] = s[d].rotate_left(8);
    s[c] = s[c].wrapping_add(s[d]); s[b] ^= s[c]; s[b] = s[b].rotate_left(7);
}

fn b054_chacha20(nblocks: u32) -> i64 {
    let mut acc: u64 = 0;
    for blk in 0u32..nblocks {
        let mut s: [u32; 16] = [
            0x61707865, 0x3320646e, 0x79622d32, 0x6b206574,
            0x03020100, 0x07060504, 0x0b0a0908, 0x0f0e0d0c,
            0x13121110, 0x17161514, 0x1b1a1918, 0x1f1e1d1c,
            blk, 0x09000000, 0x4a000000, 0x00000000,
        ];
        for _ in 0..10 {
            chacha_qr(&mut s, 0, 4, 8, 12); chacha_qr(&mut s, 1, 5, 9, 13); chacha_qr(&mut s, 2, 6, 10, 14); chacha_qr(&mut s, 3, 7, 11, 15);
            chacha_qr(&mut s, 0, 5, 10, 15); chacha_qr(&mut s, 1, 6, 11, 12); chacha_qr(&mut s, 2, 7, 8, 13); chacha_qr(&mut s, 3, 4, 9, 14);
        }
        acc ^= (s[0].wrapping_add(0x61707865) as u64) ^ (s[12].wrapping_add(blk) as u64);
    }
    acc as i64
}

#[inline(always)]
fn sipround(v0: &mut u64, v1: &mut u64, v2: &mut u64, v3: &mut u64) {
    *v0 = v0.wrapping_add(*v1); *v1 = v1.rotate_left(13); *v1 ^= *v0; *v0 = v0.rotate_left(32);
    *v2 = v2.wrapping_add(*v3); *v3 = v3.rotate_left(16); *v3 ^= *v2;
    *v0 = v0.wrapping_add(*v3); *v3 = v3.rotate_left(21); *v3 ^= *v0;
    *v2 = v2.wrapping_add(*v1); *v1 = v1.rotate_left(17); *v1 ^= *v2; *v2 = v2.rotate_left(32);
}

fn b055_siphash24_batch(words: &[u64]) -> i64 {
    let mut acc: u64 = 0;
    for msg in words.chunks_exact(32) {
        let mut v0: u64 = 0x736f6d6570736575 ^ 0x0706050403020100;
        let mut v1: u64 = 0x646f72616e646f6d ^ 0x0f0e0d0c0b0a0908;
        let mut v2: u64 = 0x6c7967656e657261 ^ 0x0706050403020100;
        let mut v3: u64 = 0x7465646279746573 ^ 0x0f0e0d0c0b0a0908;
        for &m in msg {
            v3 ^= m;
            sipround(&mut v0, &mut v1, &mut v2, &mut v3);
            sipround(&mut v0, &mut v1, &mut v2, &mut v3);
            v0 ^= m;
        }
        v2 ^= 0xff;
        for _ in 0..4 { sipround(&mut v0, &mut v1, &mut v2, &mut v3); }
        acc ^= v0 ^ v1 ^ v2 ^ v3;
    }
    acc as i64
}

fn b056_murmur3_batch(words: &[u64]) -> i64 {
    let c1: u64 = 0x87c37b91114253d5;
    let c2: u64 = 0x4cf5ad432745937f;
    let mut acc: u64 = 0;
    for msg in words.chunks_exact(32) {
        let mut h1: u64 = 0x9747b28c;
        let mut h2: u64 = 0x9747b28c;
        for i in (0..32).step_by(2) {
            let mut k1 = msg[i].wrapping_mul(c1).rotate_left(31).wrapping_mul(c2);
            h1 ^= k1; h1 = h1.rotate_left(27).wrapping_add(h2).wrapping_mul(5).wrapping_add(0x52dce729);
            let mut k2 = msg[i + 1].wrapping_mul(c2).rotate_left(33).wrapping_mul(c1);
            h2 ^= k2; h2 = h2.rotate_left(31).wrapping_add(h1).wrapping_mul(5).wrapping_add(0x38495ab5);
            let _ = (&mut k1, &mut k2);
        }
        h1 ^= 256; h2 ^= 256;
        h1 = h1.wrapping_add(h2); h2 = h2.wrapping_add(h1);
        h1 ^= h1 >> 33; h1 = h1.wrapping_mul(0xff51afd7ed558ccd); h1 ^= h1 >> 33; h1 = h1.wrapping_mul(0xc4ceb9fe1a85ec53); h1 ^= h1 >> 33;
        h2 ^= h2 >> 33; h2 = h2.wrapping_mul(0xff51afd7ed558ccd); h2 ^= h2 >> 33; h2 = h2.wrapping_mul(0xc4ceb9fe1a85ec53); h2 ^= h2 >> 33;
        acc ^= h1 ^ h2;
    }
    acc as i64
}

fn b057_xxhash64_batch(words: &[u64]) -> i64 {
    const P1: u64 = 0x9E3779B185EBCA87;
    const P2: u64 = 0xC2B2AE3D27D4EB4F;
    const P3: u64 = 0x165667B19E3779F9;
    const P4: u64 = 0x85EBCA77C2B2AE63;
    let mut acc: u64 = 0;
    for msg in words.chunks_exact(64) {
        let mut v1 = P1.wrapping_add(P2);
        let mut v2 = P2;
        let mut v3 = 0u64;
        let mut v4 = 0u64.wrapping_sub(P1);
        for i in (0..64).step_by(4) {
            v1 = v1.wrapping_add(msg[i].wrapping_mul(P2)).rotate_left(31).wrapping_mul(P1);
            v2 = v2.wrapping_add(msg[i+1].wrapping_mul(P2)).rotate_left(31).wrapping_mul(P1);
            v3 = v3.wrapping_add(msg[i+2].wrapping_mul(P2)).rotate_left(31).wrapping_mul(P1);
            v4 = v4.wrapping_add(msg[i+3].wrapping_mul(P2)).rotate_left(31).wrapping_mul(P1);
        }
        let mut h = v1.rotate_left(1).wrapping_add(v2.rotate_left(7)).wrapping_add(v3.rotate_left(12)).wrapping_add(v4.rotate_left(18));
        h = h.wrapping_add(512);
        h ^= h >> 33; h = h.wrapping_mul(P2); h ^= h >> 29; h = h.wrapping_mul(P3); h ^= h >> 32;
        let _ = P4;
        acc ^= h;
    }
    acc as i64
}

#[inline(always)]
fn b2s_g(v: &mut [u32; 16], a: usize, b: usize, c: usize, d: usize, x: u32, y: u32) {
    v[a] = v[a].wrapping_add(v[b]).wrapping_add(x); v[d] = (v[d] ^ v[a]).rotate_right(16);
    v[c] = v[c].wrapping_add(v[d]); v[b] = (v[b] ^ v[c]).rotate_right(12);
    v[a] = v[a].wrapping_add(v[b]).wrapping_add(y); v[d] = (v[d] ^ v[a]).rotate_right(8);
    v[c] = v[c].wrapping_add(v[d]); v[b] = (v[b] ^ v[c]).rotate_right(7);
}

fn b058_blake2s_batch(nblocks: usize) -> i64 {
    let mut acc: u64 = 0;
    for blk in 0..nblocks {
        let mut m = [0u32; 16];
        for i in 0..16 { m[i] = ((blk * 17 + i * 31) as u32).wrapping_mul(0x9E3779B9); }
        let mut v: [u32; 16] = [
            0x6a09e667 ^ 0x01010020, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
            0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
            0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
            0x510e527f ^ 64, 0x9b05688c, 0x1f83d9ab ^ 0xFFFF_FFFF, 0x5be0cd19,
        ];
        for r in 0..10usize {
            let s = (r * 3) & 15;
            b2s_g(&mut v, 0, 4, 8, 12, m[s], m[(s+1)&15]);
            b2s_g(&mut v, 1, 5, 9, 13, m[(s+2)&15], m[(s+3)&15]);
            b2s_g(&mut v, 2, 6, 10, 14, m[(s+4)&15], m[(s+5)&15]);
            b2s_g(&mut v, 3, 7, 11, 15, m[(s+6)&15], m[(s+7)&15]);
            b2s_g(&mut v, 0, 5, 10, 15, m[(s+8)&15], m[(s+9)&15]);
            b2s_g(&mut v, 1, 6, 11, 12, m[(s+10)&15], m[(s+11)&15]);
            b2s_g(&mut v, 2, 7, 8, 13, m[(s+12)&15], m[(s+13)&15]);
            b2s_g(&mut v, 3, 4, 9, 14, m[(s+14)&15], m[(s+15)&15]);
        }
        acc ^= (((v[0] ^ v[8]) as u64) << 32) | ((v[1] ^ v[9]) as u64);
    }
    acc as i64
}

fn b059_hmac_sha256(data: &[u8]) -> i64 {
    let mut ipad = [0x36u8; 64];
    let mut opad = [0x5Cu8; 64];
    for i in 0..32 { let kb = (i * 7 + 3) as u8; ipad[i] ^= kb; opad[i] ^= kb; }
    let mut inner_buf = vec![0u8; 64 + 512];
    let mut outer_buf = [0u8; 96];
    inner_buf[..64].copy_from_slice(&ipad);
    outer_buf[..64].copy_from_slice(&opad);
    let mut acc: u64 = 0;
    for msg in data.chunks_exact(512) {
        inner_buf[64..].copy_from_slice(msg);
        let ih = sha256_digest_words(&inner_buf);
        for w in 0..8 { outer_buf[64 + w*4..64 + (w+1)*4].copy_from_slice(&ih[w].to_be_bytes()); }
        let oh = sha256_digest_words(&outer_buf);
        acc ^= ((oh[0] as u64) << 32) | (oh[1] as u64);
    }
    acc as i64
}

fn b060_speck_ctr(nblocks: usize) -> i64 {
    let rk: [u64; 8] = [
        0x0706050403020100, 0x0f0e0d0c0b0a0908, 0x1716151413121110, 0x1f1e1d1c1b1a1918,
        0x2726252423222120, 0x2f2e2d2c2b2a2928, 0x3736353433323130, 0x3f3e3d3c3b3a3938,
    ];
    let mut acc: u64 = 0;
    for blk in 0..nblocks {
        let mut x = (blk as u64).wrapping_mul(0x9E3779B97F4A7C15);
        let mut y = (blk as u64) ^ 0xA5A5A5A5A5A5A5A5;
        for r in 0..8 {
            x = x.rotate_right(8).wrapping_add(y) ^ rk[r];
            y = y.rotate_left(3) ^ x;
        }
        acc ^= x ^ y;
    }
    acc as i64
}

// ==================== Domain 7: Signal Processing & Numerical (B061..B070) ====================

fn b061_fft_16384() -> i64 {
    let n = 16384usize;
    let mut re = vec![0.0f64; n];
    let mut im = vec![0.0f64; n];
    for i in 0..n { re[i] = ((i % 64) as f64) * 0.05 - 1.5; im[i] = ((i % 32) as f64) * 0.03 - 0.5; }
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
                let ur = re[i+k]; let ui = im[i+k];
                let vr = re[i+k+half]*wr - im[i+k+half]*wi;
                let vi = re[i+k+half]*wi + im[i+k+half]*wr;
                re[i+k] = ur + vr; im[i+k] = ui + vi;
                re[i+k+half] = ur - vr; im[i+k+half] = ui - vi;
                let nwr = wr * wlen_r - wi * wlen_i;
                wi = wr * wlen_i + wi * wlen_r;
                wr = nwr;
            }
            i += len;
        }
        len <<= 1;
    }
    let mut s = 0.0f64;
    for i in 0..64 { s += re[i].abs() + im[i].abs(); }
    (s * 100.0) as i64
}

fn b062_dct_ii(data: &[f64], cos_tbl: &[f64], blocks: usize) -> i64 {
    let mut acc = 0.0f64;
    for b in 0..blocks {
        let x = &data[b * 64..(b + 1) * 64];
        for k in 0..64 {
            let row = &cos_tbl[k * 64..(k + 1) * 64];
            let mut s = 0.0f64;
            for n in 0..64 { s += x[n] * row[n]; }
            acc += s.abs();
        }
    }
    (black_box(acc) * 100.0) as i64
}

fn b063_fir_64tap(x: &[f64], h: &[f64; 64]) -> i64 {
    let m = x.len() - 64;
    let mut acc = 0.0f64;
    for i in 0..m {
        let mut s = 0.0f64;
        for k in 0..64 { s += x[i + k] * h[k]; }
        acc += s;
    }
    (black_box(acc) * 100.0) as i64
}

fn b064_iir_biquad(chans: usize, len: usize) -> i64 {
    let mut acc = 0.0f64;
    for c in 0..chans {
        let (mut z1a, mut z2a) = (0.0f64, 0.0f64);
        let (mut z1b, mut z2b) = (0.0f64, 0.0f64);
        let (mut z1c, mut z2c) = (0.0f64, 0.0f64);
        let (mut z1d, mut z2d) = (0.0f64, 0.0f64);
        let b0 = 0.25f64; let b1 = 0.50f64; let b2 = 0.25f64;
        let a1 = -0.35f64; let a2 = 0.12f64;
        let mut ch_sum = 0.0f64;
        for i in 0..len {
            let x0 = (((c * 17 + i) & 63) as f64) * 0.03 - 0.9;
            let y1 = b0 * x0 + z1a; z1a = b1 * x0 - a1 * y1 + z2a; z2a = b2 * x0 - a2 * y1;
            let y2 = b0 * y1 + z1b; z1b = b1 * y1 - a1 * y2 + z2b; z2b = b2 * y1 - a2 * y2;
            let y3 = b0 * y2 + z1c; z1c = b1 * y2 - a1 * y3 + z2c; z2c = b2 * y2 - a2 * y3;
            let y4 = b0 * y3 + z1d; z1d = b1 * y3 - a1 * y4 + z2d; z2d = b2 * y3 - a2 * y4;
            ch_sum += y4;
        }
        acc += ch_sum;
    }
    (black_box(acc) * 100.0) as i64
}

fn b065_rk4_lorenz(ntraj: usize, steps: usize) -> i64 {
    let dt = 0.005f64;
    let sigma = 10.0f64; let rho = 28.0f64; let beta = 8.0 / 3.0;
    let mut acc = 0.0f64;
    for t in 0..ntraj {
        let mut x = 1.0 + (t as f64) * 0.02;
        let mut y = 1.0 - (t as f64) * 0.01;
        let mut z = 1.0 + (t as f64) * 0.015;
        for _ in 0..steps {
            let k1x = sigma*(y - x); let k1y = x*(rho - z) - y; let k1z = x*y - beta*z;
            let x2 = x + 0.5*dt*k1x; let y2 = y + 0.5*dt*k1y; let z2 = z + 0.5*dt*k1z;
            let k2x = sigma*(y2 - x2); let k2y = x2*(rho - z2) - y2; let k2z = x2*y2 - beta*z2;
            let x3 = x + 0.5*dt*k2x; let y3 = y + 0.5*dt*k2y; let z3 = z + 0.5*dt*k2z;
            let k3x = sigma*(y3 - x3); let k3y = x3*(rho - z3) - y3; let k3z = x3*y3 - beta*z3;
            let x4 = x + dt*k3x; let y4 = y + dt*k3y; let z4 = z + dt*k3z;
            let k4x = sigma*(y4 - x4); let k4y = x4*(rho - z4) - y4; let k4z = x4*y4 - beta*z4;
            x += (dt / 6.0) * (k1x + 2.0*k2x + 2.0*k3x + k4x);
            y += (dt / 6.0) * (k1y + 2.0*k2y + 2.0*k3y + k4y);
            z += (dt / 6.0) * (k1z + 2.0*k2z + 2.0*k3z + k4z);
        }
        acc += x + y + z;
    }
    (black_box(acc) * 100.0) as i64
}

fn b066_gl16_quad(n: usize) -> i64 {
    const X: [f64; 8] = [0.09501250983763744, 0.28160355077925891, 0.45801677765722738, 0.61787624440264375,
                         0.75540440835500303, 0.86563120238783174, 0.94457502307323258, 0.98940093499164993];
    const W: [f64; 8] = [0.18945061045506850, 0.18260341504492359, 0.16915651939500254, 0.14959598881657673,
                         0.12462897125553387, 0.09515851168249278, 0.06225352393864789, 0.02715245941175409];
    let mut acc = 0.0f64;
    for i in 0..n {
        let a = 0.5 + ((i % 97) as f64) * 0.05;
        let b = 0.1 + ((i % 53) as f64) * 0.02;
        let mut s = 0.0f64;
        for k in 0..8 {
            let x = X[k];
            let xp = 0.5 * (1.0 + x);
            let xm = 0.5 * (1.0 - x);
            s += W[k] * (1.0 / (1.0 + a * xp * xp) + b * xp * xp * xp + 1.0 / (1.0 + a * xm * xm) + b * xm * xm * xm);
        }
        acc += 0.5 * s;
    }
    (black_box(acc) * 1000.0) as i64
}

fn b067_mc_pi(n: i64) -> i64 {
    let n = black_box(n);
    let r2: u64 = 1u64 << 60;
    let mut hits: i64 = 0;
    for i in 0..n {
        let mut z = (i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(0x123456789ABCDEF0);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        let x = z & 0x3FFF_FFFF;
        let y = z >> 34;
        if x * x + y * y < r2 { hits += 1; }
    }
    hits
}

#[inline(always)]
fn norm_cdf_poly(x: f64) -> f64 {
    let ax = x.abs();
    let t = 1.0 / (1.0 + 0.2316419 * ax);
    let poly = t * (0.319381530 + t * (-0.356563782 + t * (1.781477937 + t * (-1.821255978 + t * 1.330274429))));
    // Rational approx to exp(-0.5*x^2) via 1/(1 + u + u^2/2 + u^3/6 + u^4/24 + u^5/120) for identical cross-lang accuracy
    let u = 0.5 * ax * ax;
    let e = if u > 20.0 { 0.0 } else { 1.0 / (1.0 + u * (1.0 + u * (0.5 + u * (0.16666666666666666 + u * (0.041666666666666664 + u * 0.008333333333333333))))) };
    let pdf = 0.3989422804014327 * e;
    if x >= 0.0 { 1.0 - pdf * poly } else { pdf * poly }
}

fn b068_black_scholes(n: usize) -> i64 {
    let mut acc = 0.0f64;
    for i in 0..n {
        let s = 80.0 + ((i % 41) as f64);
        let k = 100.0;
        let t = 0.25 + ((i % 16) as f64) * 0.1;
        let r = 0.05;
        let v = 0.15 + ((i % 20) as f64) * 0.01;
        let sqt = t.sqrt();
        // Fast log(s/k) via atanh series (y = (s-k)/(s+k))
        let y = (s - k) / (s + k);
        let y2 = y * y;
        let ln_sk = 2.0 * y * (1.0 + y2 * (0.3333333333333333 + y2 * (0.2 + y2 * 0.14285714285714285)));
        let d1 = (ln_sk + (r + 0.5 * v * v) * t) / (v * sqt);
        let d2 = d1 - v * sqt;
        let u = r * t;
        let disc = 1.0 / (1.0 + u * (1.0 + u * (0.5 + u * 0.16666666666666666)));
        let call = s * norm_cdf_poly(d1) - k * disc * norm_cdf_poly(d2);
        acc += call;
    }
    (black_box(acc) * 100.0) as i64
}

fn b069_newton_cubic(n: usize) -> i64 {
    let mut acc = 0.0f64;
    for i in 0..n {
        let a = 1.0 + ((i % 64) as f64) * 0.05;
        let b = 2.0 + ((i % 128) as f64) * 0.1;
        let mut x = 1.5f64;
        for _ in 0..12 {
            let f = x * x * x + a * x - b;
            let df = 3.0 * x * x + a;
            x -= f / df;
        }
        acc += x;
    }
    (black_box(acc) * 1000.0) as i64
}

fn b070_bilinear_interp(src: &[f64], sw: usize, dw: usize) -> i64 {
    let scale = (sw - 1) as f64 / (dw as f64);
    let mut acc = 0.0f64;
    for y in 0..dw {
        let fy = (y as f64) * scale;
        let iy = fy as usize;
        let dy = fy - (iy as f64);
        let r0 = iy * sw;
        let r1 = (iy + 1) * sw;
        for x in 0..dw {
            let fx = (x as f64) * scale;
            let ix = fx as usize;
            let dx = fx - (ix as f64);
            let v00 = src[r0 + ix]; let v10 = src[r0 + ix + 1];
            let v01 = src[r1 + ix]; let v11 = src[r1 + ix + 1];
            let top = v00 + dx * (v10 - v00);
            let bot = v01 + dx * (v11 - v01);
            acc += top + dy * (bot - top);
        }
    }
    (black_box(acc) * 100.0) as i64
}

// ==================== Domain 8: Physics, Geometry & Graphics (B071..B080) ====================

fn b071_nbody_3d(n: usize, steps: usize) -> i64 {
    let dt = 0.01f64;
    let mut px = vec![0.0f64; n]; let mut py = vec![0.0f64; n]; let mut pz = vec![0.0f64; n];
    let mut vx = vec![0.0f64; n]; let mut vy = vec![0.0f64; n]; let mut vz = vec![0.0f64; n];
    let mut pm = vec![0.0f64; n];
    for i in 0..n {
        let fi = (i + 1) as f64;
        px[i] = fi * 0.13; py[i] = fi * 0.27 - 10.0; pz[i] = fi * 0.09 - 5.0;
        pm[i] = 0.01 + ((i % 7) as f64) * 0.005;
    }
    for _ in 0..steps {
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

fn b072_mandelbrot(size: usize) -> i64 {
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

fn b073_julia(size: usize) -> i64 {
    let inv = 1.0 / (size as f64);
    let (cr, ci) = (-0.8f64, 0.156f64);
    let mut total: i64 = 0;
    for y in 0..size {
        let y0 = (y as f64) * 2.6 * inv - 1.3;
        for x in 0..size {
            let mut zr = (x as f64) * 3.0 * inv - 1.5;
            let mut zi = y0;
            let mut it = 0i64;
            while it < 256 {
                let zr2 = zr * zr; let zi2 = zi * zi;
                if zr2 + zi2 > 4.0 { break; }
                zi = 2.0 * zr * zi + ci;
                zr = zr2 - zi2 + cr;
                it += 1;
            }
            total += it;
        }
    }
    total
}

fn b074_ray_sphere(nrays: usize) -> i64 {
    let mut sx = [0.0f64; 32]; let mut sy = [0.0f64; 32]; let mut sz = [0.0f64; 32]; let mut sr2 = [0.0f64; 32];
    for k in 0..32 {
        sx[k] = ((k % 8) as f64) * 1.5 - 5.25;
        sy[k] = ((k / 8) as f64) * 1.5 - 2.25;
        sz[k] = 6.0 + ((k % 5) as f64) * 1.2;
        let r = 0.6 + ((k % 4) as f64) * 0.15;
        sr2[k] = r * r;
    }
    let mut hits: i64 = 0;
    for i in 0..nrays {
        let dx = ((i % 320) as f64) * 0.005 - 0.8;
        let dy = ((i / 320) as f64) * 0.005 - 0.75;
        let dz = 1.0f64;
        let a = dx * dx + dy * dy + dz * dz;
        for k in 0..32 {
            let b = -(dx * sx[k] + dy * sy[k] + dz * sz[k]);
            let c = sx[k] * sx[k] + sy[k] * sy[k] + sz[k] * sz[k] - sr2[k];
            if b * b - a * c >= 0.0 { hits += 1; }
        }
    }
    hits
}

fn b075_aabb_overlap(n: usize) -> i64 {
    let mut x0 = vec![0.0f64; n]; let mut x1 = vec![0.0f64; n];
    let mut y0 = vec![0.0f64; n]; let mut y1 = vec![0.0f64; n];
    let mut z0 = vec![0.0f64; n]; let mut z1 = vec![0.0f64; n];
    for i in 0..n {
        let fx = ((i * 37) % 1000) as f64 * 0.1;
        let fy = ((i * 73) % 1000) as f64 * 0.1;
        let fz = ((i * 19) % 1000) as f64 * 0.1;
        x0[i] = fx; x1[i] = fx + 8.5;
        y0[i] = fy; y1[i] = fy + 8.5;
        z0[i] = fz; z1[i] = fz + 8.5;
    }
    let mut cnt: i64 = 0;
    for i in 0..n {
        let (ax0, ax1, ay0, ay1, az0, az1) = (x0[i], x1[i], y0[i], y1[i], z0[i], z1[i]);
        for j in (i + 1)..n {
            if ax0 <= x1[j] && ax1 >= x0[j] && ay0 <= y1[j] && ay1 >= y0[j] && az0 <= z1[j] && az1 >= z0[j] {
                cnt += 1;
            }
        }
    }
    cnt
}

fn b076_sobel_edge(img: &[u8], w: usize) -> i64 {
    let mut acc: i64 = 0;
    for y in 1..(w - 1) {
        let r0 = (y - 1) * w; let r1 = y * w; let r2 = (y + 1) * w;
        for x in 1..(w - 1) {
            let gx = -(img[r0+x-1] as i64) + (img[r0+x+1] as i64)
                     - 2 * (img[r1+x-1] as i64) + 2 * (img[r1+x+1] as i64)
                     - (img[r2+x-1] as i64) + (img[r2+x+1] as i64);
            let gy = -(img[r0+x-1] as i64) - 2 * (img[r0+x] as i64) - (img[r0+x+1] as i64)
                     + (img[r2+x-1] as i64) + 2 * (img[r2+x] as i64) + (img[r2+x+1] as i64);
            acc = acc.wrapping_add(gx.abs() + gy.abs());
        }
    }
    acc
}

fn b077_gauss_blur(img: &[f32], w: usize) -> i64 {
    let mut tmp = vec![0.0f32; w * w];
    let mut dst = vec![0.0f32; w * w];
    for y in 0..w {
        let r = y * w;
        for x in 2..(w - 2) {
            tmp[r + x] = (img[r+x-2] + 4.0*img[r+x-1] + 6.0*img[r+x] + 4.0*img[r+x+1] + img[r+x+2]) * 0.0625;
        }
    }
    let mut acc = 0.0f64;
    for y in 2..(w - 2) {
        let r = y * w;
        for x in 2..(w - 2) {
            let v = (tmp[r-2*w+x] + 4.0*tmp[r-w+x] + 6.0*tmp[r+x] + 4.0*tmp[r+w+x] + tmp[r+2*w+x]) * 0.0625;
            dst[r + x] = v;
            acc += v as f64;
        }
    }
    (black_box(acc) * 100.0) as i64
}

fn b078_heat_2d(w: usize, steps: usize) -> i64 {
    let mut a = vec![0.0f64; w * w];
    let mut b = vec![0.0f64; w * w];
    for y in 0..w {
        for x in 0..w { a[y * w + x] = (((x * 7 + y * 13) & 63) as f64) * 0.1; }
    }
    for _ in 0..steps {
        for y in 1..(w - 1) {
            let r = y * w;
            for x in 1..(w - 1) {
                b[r + x] = 0.2 * (a[r + x] + a[r + x - 1] + a[r + x + 1] + a[r - w + x] + a[r + w + x]);
            }
        }
        std::mem::swap(&mut a, &mut b);
    }
    let mut s = 0.0f64;
    for i in (0..(w * w)).step_by(17) { s += a[i]; }
    (black_box(s) * 1000.0) as i64
}

fn b079_verlet_3d(n: usize, steps: usize) -> i64 {
    let dt = 0.01f64;
    let mut px = vec![0.0f64; n]; let mut py = vec![0.0f64; n]; let mut pz = vec![0.0f64; n];
    let mut vx = vec![0.0f64; n]; let mut vy = vec![0.0f64; n]; let mut vz = vec![0.0f64; n];
    for i in 0..n {
        px[i] = ((i % 97) as f64) * 0.1 - 4.0;
        py[i] = ((i % 89) as f64) * 0.1 - 4.0;
        pz[i] = ((i % 83) as f64) * 0.1 - 4.0;
        vx[i] = 0.5; vy[i] = -0.3; vz[i] = 0.2;
    }
    for _ in 0..steps {
        for i in 0..n {
            let ax = -0.05 * px[i]; let ay = -9.81 - 0.05 * py[i]; let az = -0.05 * pz[i];
            px[i] += vx[i] * dt + 0.5 * ax * dt * dt;
            py[i] += vy[i] * dt + 0.5 * ay * dt * dt;
            pz[i] += vz[i] * dt + 0.5 * az * dt * dt;
            vx[i] += ax * dt; vy[i] += ay * dt; vz[i] += az * dt;
        }
    }
    let s: f64 = (0..n).map(|i| px[i] + py[i] + pz[i]).sum();
    (black_box(s) * 10.0) as i64
}

fn b080_polygon_pip(npts: usize) -> i64 {
    let nv = 32usize;
    let mut vx = [0.0f64; 32]; let mut vy = [0.0f64; 32];
    for i in 0..nv {
        let ang = (i as f64) * 6.283185307179586 / (nv as f64);
        let r = if (i & 1) == 0 { 1.0 } else { 0.6 };
        vx[i] = r * ang.cos(); vy[i] = r * ang.sin();
    }
    let mut inside_cnt: i64 = 0;
    for p in 0..npts {
        let px = ((p % 512) as f64) * 0.004 - 1.0;
        let py = ((p / 512) as f64) * 0.005 - 1.0;
        let mut inside = false;
        let mut j = nv - 1;
        for i in 0..nv {
            if ((vy[i] > py) != (vy[j] > py)) && (px < (vx[j] - vx[i]) * (py - vy[i]) / (vy[j] - vy[i]) + vx[i]) {
                inside = !inside;
            }
            j = i;
        }
        if inside { inside_cnt += 1; }
    }
    inside_cnt
}

// ==================== Domain 9: Dense & Sparse Linear Algebra (B081..B090) ====================

fn b081_fp32_gemm(a: &[f32], b: &[f32], n: usize) -> i64 {
    let mut c = vec![0.0f32; n * n];
    for i in 0..n {
        for k in 0..n {
            let aik = a[i * n + k];
            let brow = &b[k * n..(k + 1) * n];
            let crow = &mut c[i * n..(i + 1) * n];
            for j in 0..n { crow[j] += aik * brow[j]; }
        }
    }
    let sum: f64 = c.iter().map(|&v| v as f64).sum();
    (sum * 100.0) as i64
}

fn b082_fp64_gemm(a: &[f64], b: &[f64], n: usize) -> i64 {
    let mut c = vec![0.0f64; n * n];
    for i in 0..n {
        for k in 0..n {
            let aik = a[i * n + k];
            let brow = &b[k * n..(k + 1) * n];
            let crow = &mut c[i * n..(i + 1) * n];
            for j in 0..n { crow[j] += aik * brow[j]; }
        }
    }
    let sum: f64 = c.iter().sum();
    (sum * 100.0) as i64
}

fn b083_csr_spmv(rp: &[usize], ci: &[usize], val: &[f64], x: &[f64], n: usize) -> i64 {
    let mut y = vec![0.0f64; n];
    let mut acc = 0.0f64;
    for _ in 0..3 {
        for r in 0..n {
            let mut s = 0.0f64;
            for idx in rp[r]..rp[r + 1] { s += val[idx] * x[ci[idx]]; }
            y[r] = s;
        }
        acc += y[0] + y[n / 2] + y[n - 1];
    }
    for &v in &y { acc += v; }
    (black_box(acc) * 100.0) as i64
}

fn b084_cholesky(n: usize) -> i64 {
    let mut a = vec![0.0f64; n * n];
    for i in 0..n {
        for j in 0..=i {
            let v = if i == j { (n as f64) + 5.0 } else { 0.05 / (1.0 + ((i - j) as f64)) };
            a[i * n + j] = v; a[j * n + i] = v;
        }
    }
    let mut l = vec![0.0f64; n * n];
    for i in 0..n {
        for j in 0..=i {
            let mut s = a[i * n + j];
            for k in 0..j { s -= l[i * n + k] * l[j * n + k]; }
            l[i * n + j] = if i == j { s.sqrt() } else { s / l[j * n + j] };
        }
    }
    let mut tr = 0.0f64;
    for i in 0..n { tr += l[i * n + i]; }
    (black_box(tr) * 10000.0) as i64
}

fn b085_lu_factor(nmats: usize, n: usize) -> i64 {
    let mut a = vec![0.0f64; n * n];
    let mut s = 0.0f64;
    for m in 0..nmats {
        for i in 0..n {
            for j in 0..n {
                a[i * n + j] = if i == j { 10.0 + (m as f64) * 0.25 } else { (((i * 7 + j * 13 + m * 3) % 19) as f64 - 9.0) * 0.01 };
            }
        }
        for k in 0..n {
            let piv = a[k * n + k];
            for i in (k + 1)..n {
                let f = a[i * n + k] / piv;
                a[i * n + k] = f;
                for j in (k + 1)..n { a[i * n + j] -= f * a[k * n + j]; }
            }
        }
        for i in 0..n { s += a[i * n + i]; }
    }
    (black_box(s) * 10000.0) as i64
}

fn b086_qr_mgs(n: usize) -> i64 {
    let mut q = vec![0.0f64; n * n]; // stored column-major: col j at q[j*n .. (j+1)*n]
    for j in 0..n {
        for i in 0..n {
            q[j * n + i] = if i == j { 4.0 } else { (((i * 11 + j * 7) % 17) as f64 - 8.0) * 0.02 };
        }
    }
    let mut r_diag = 0.0f64;
    for k in 0..n {
        let mut nrm = 0.0f64;
        for i in 0..n { let v = q[k * n + i]; nrm += v * v; }
        let rkk = nrm.sqrt();
        r_diag += rkk;
        let inv = 1.0 / rkk;
        for i in 0..n { q[k * n + i] *= inv; }
        for j in (k + 1)..n {
            let mut dot = 0.0f64;
            for i in 0..n { dot += q[k * n + i] * q[j * n + i]; }
            for i in 0..n { q[j * n + i] -= dot * q[k * n + i]; }
        }
    }
    (black_box(r_diag) * 10000.0) as i64
}

fn b087_jacobi_sym(nmats: usize, n: usize) -> i64 {
    let mut acc = 0.0f64;
    let mut a = vec![0.0f64; n * n];
    for m in 0..nmats {
        for i in 0..n {
            for j in 0..=i {
                let v = if i == j { (i as f64) + 1.0 + (m as f64) * 0.1 } else { 0.05 / (1.0 + ((i - j) as f64)) };
                a[i * n + j] = v; a[j * n + i] = v;
            }
        }
        for _ in 0..4 {
            for p in 0..n {
                for q in (p + 1)..n {
                    let apq = a[p * n + q];
                    if apq.abs() < 1e-12 { continue; }
                    let tau = (a[q * n + q] - a[p * n + p]) / (2.0 * apq);
                    let t = if tau >= 0.0 { 1.0 / (tau + (1.0 + tau * tau).sqrt()) } else { -1.0 / (-tau + (1.0 + tau * tau).sqrt()) };
                    let c = 1.0 / (1.0 + t * t).sqrt();
                    let s = t * c;
                    let app = a[p * n + p]; let aqq = a[q * n + q];
                    a[p * n + p] = app - t * apq; a[q * n + q] = aqq + t * apq;
                    a[p * n + q] = 0.0; a[q * n + p] = 0.0;
                    for k in 0..n {
                        if k != p && k != q {
                            let akp = a[k * n + p]; let akq = a[k * n + q];
                            let nkp = c * akp - s * akq; let nkq = s * akp + c * akq;
                            a[k * n + p] = nkp; a[p * n + k] = nkp;
                            a[k * n + q] = nkq; a[q * n + k] = nkq;
                        }
                    }
                }
            }
        }
        for i in 0..n { acc += a[i * n + i]; }
    }
    (black_box(acc) * 100.0) as i64
}

fn b088_tridiag_thomas(nsolves: usize, n: usize) -> i64 {
    let mut c_p = vec![0.0f64; n];
    let mut d_p = vec![0.0f64; n];
    let mut acc = 0.0f64;
    for s in 0..nsolves {
        let a_sub = -1.0f64; let b_diag = 4.0 + ((s & 7) as f64) * 0.1; let c_sup = -1.0f64;
        c_p[0] = c_sup / b_diag;
        d_p[0] = (1.0 + (s % 13) as f64 * 0.1) / b_diag;
        for i in 1..n {
            let denom = b_diag - a_sub * c_p[i - 1];
            c_p[i] = c_sup / denom;
            let rhs = 1.0 + (((s + i) % 13) as f64) * 0.1;
            d_p[i] = (rhs - a_sub * d_p[i - 1]) / denom;
        }
        let mut x_next = d_p[n - 1];
        acc += x_next;
        for i in (0..(n - 1)).rev() {
            x_next = d_p[i] - c_p[i] * x_next;
            acc += x_next;
        }
    }
    (black_box(acc) * 100.0) as i64
}

fn b089_cg_sparse(n: usize, iters: usize) -> i64 {
    let mut x = vec![0.0f64; n];
    let mut r = vec![0.0f64; n];
    let mut p = vec![0.0f64; n];
    let mut ap = vec![0.0f64; n];
    let mut rsold = 0.0f64;
    for i in 0..n { let b = 1.0 + ((i % 19) as f64) * 0.05; r[i] = b; p[i] = b; rsold += b * b; }
    for _ in 0..iters {
        let mut pap = 0.0f64;
        for i in 0..n {
            let mut v = 6.0 * p[i];
            if i > 0 { v -= p[i - 1]; }
            if i + 1 < n { v -= p[i + 1]; }
            if i >= 64 { v -= 0.5 * p[i - 64]; }
            if i + 64 < n { v -= 0.5 * p[i + 64]; }
            ap[i] = v;
            pap += p[i] * v;
        }
        let alpha = rsold / pap;
        let mut rsnew = 0.0f64;
        for i in 0..n {
            x[i] += alpha * p[i];
            r[i] -= alpha * ap[i];
            rsnew += r[i] * r[i];
        }
        let beta = rsnew / rsold;
        for i in 0..n { p[i] = r[i] + beta * p[i]; }
        rsold = rsnew;
    }
    let s: f64 = x.iter().sum();
    (black_box(s) * 100.0) as i64
}

fn b090_outer_rank1(x: &[f64], y: &[f64], n: usize) -> i64 {
    let mut a = vec![0.0f64; n * n];
    for step in 0..4 {
        let alpha = 0.25 + (step as f64) * 0.1;
        for i in 0..n {
            let ax = alpha * x[i];
            let row = &mut a[i * n..(i + 1) * n];
            for j in 0..n { row[j] += ax * y[j]; }
        }
    }
    let mut s = 0.0f64;
    for i in (0..(n * n)).step_by(31) { s += a[i]; }
    (black_box(s) * 1000.0) as i64
}

// ==================== Domain 10: Impossible AI, Quantized LLM & Vector DB (B091..B100) ====================

fn b091_bnn_gemv(w: &[u64], a: &[u64], rows: usize, k_words: usize) -> i64 {
    let mut acc: i64 = 0;
    for r in 0..rows {
        let row = &w[r * k_words..(r + 1) * k_words];
        let mut pc: i64 = 0;
        for kw in 0..k_words { pc += (row[kw] ^ a[kw]).count_ones() as i64; }
        acc = acc.wrapping_add(pc);
    }
    acc
}

fn b092_q4_0_gemv(act: &[f32], q4: &[u8], rows: usize, cols: usize, reps: usize) -> i64 {
    let nblocks = cols / 32;
    let mut total = 0.0f64;
    for _ in 0..reps {
        for r in 0..rows {
            let mut sum = 0.0f32;
            for b in 0..nblocks {
                let off = (r * nblocks + b) * 20;
                let scale = f32::from_le_bytes([q4[off], q4[off+1], q4[off+2], q4[off+3]]);
                let x = &act[b * 32..(b + 1) * 32];
                let mut bs = 0.0f32;
                for j in 0..16 {
                    let byte = q4[off + 4 + j];
                    let lo = (byte & 0x0F) as i32 - 8;
                    let hi = ((byte >> 4) & 0x0F) as i32 - 8;
                    bs += (lo as f32) * x[j * 2] + (hi as f32) * x[j * 2 + 1];
                }
                sum += scale * bs;
            }
            total += sum as f64;
        }
    }
    (total * 10.0) as i64
}

fn b093_q8_0_gemv(act: &[f32], q8: &[u8], rows: usize, cols: usize) -> i64 {
    let nblocks = cols / 32;
    let mut total = 0.0f64;
    for r in 0..rows {
        let mut sum = 0.0f32;
        for b in 0..nblocks {
            let off = (r * nblocks + b) * 36;
            let scale = f32::from_le_bytes([q8[off], q8[off+1], q8[off+2], q8[off+3]]);
            let x = &act[b * 32..(b + 1) * 32];
            let mut bs = 0.0f32;
            for j in 0..32 { bs += (q8[off + 4 + j] as i8 as f32) * x[j]; }
            sum += scale * bs;
        }
        total += sum as f64;
    }
    (total * 10.0) as i64
}

fn b094_bitnet_158(act: &[f32], w_tern: &[i8], rows: usize, cols: usize) -> i64 {
    let mut total = 0.0f64;
    for r in 0..rows {
        let row = &w_tern[r * cols..(r + 1) * cols];
        let mut s = 0.0f32;
        for c in 0..cols { s += (row[c] as f32) * act[c]; }
        total += s as f64;
    }
    (black_box(total) * 10.0) as i64
}

fn b095_faiss_l2(vecs: &[f32], qrys: &[f32], nvecs: usize, nq: usize, dim: usize) -> i64 {
    let mut acc = 0.0f64;
    for q in 0..nq {
        let qv = &qrys[q * dim..(q + 1) * dim];
        let mut best = f32::MAX;
        let mut best_id = 0usize;
        for i in 0..nvecs {
            let vv = &vecs[i * dim..(i + 1) * dim];
            let mut d = 0.0f32;
            for k in 0..dim { let df = vv[k] - qv[k]; d += df * df; }
            if d < best { best = d; best_id = i; }
        }
        acc += (best as f64) + (best_id as f64) * 0.001;
    }
    (acc * 100.0) as i64
}

fn b096_cosine_topk(vecs: &[f32], qrys: &[f32], nvecs: usize, nq: usize, dim: usize) -> i64 {
    let mut acc = 0.0f64;
    for q in 0..nq {
        let qv = &qrys[q * dim..(q + 1) * dim];
        let mut qn = 0.0f32;
        for k in 0..dim { qn += qv[k] * qv[k]; }
        let qinv = 1.0 / qn.sqrt();
        let mut best = -1e9f32;
        for i in 0..nvecs {
            let vv = &vecs[i * dim..(i + 1) * dim];
            let mut dot = 0.0f32; let mut vn = 0.0f32;
            for k in 0..dim { dot += vv[k] * qv[k]; vn += vv[k] * vv[k]; }
            let sim = dot * qinv / vn.sqrt();
            if sim > best { best = sim; }
        }
        acc += best as f64;
    }
    (black_box(acc) * 10000.0) as i64
}

fn b097_llm_ffn(x: &[f32], nw: &[f32], w1: &[f32], w2: &[f32], n: usize) -> i64 {
    let mut ss = 0.0f32;
    for i in 0..n { ss += x[i] * x[i]; }
    let inv = 1.0 / (ss / (n as f32) + 1e-5).sqrt();
    let mut norm = vec![0.0f32; n];
    for i in 0..n { norm[i] = x[i] * inv * nw[i]; }
    let mut gate = vec![0.0f32; n];
    let mut up = vec![0.0f32; n];
    for r in 0..n {
        let row1 = &w1[r * n..(r + 1) * n];
        let row2 = &w2[r * n..(r + 1) * n];
        let (mut s1, mut s2) = (0.0f32, 0.0f32);
        for c in 0..n { s1 += row1[c] * norm[c]; s2 += row2[c] * norm[c]; }
        gate[r] = s1; up[r] = s2;
    }
    let mut total = 0.0f64;
    for i in 0..n {
        let g = gate[i];
        let silu = g / (1.0 + (-g).exp());
        total += (silu * up[i]) as f64;
    }
    (total * 1000.0) as i64
}

#[inline(always)]
fn fast_exp_neg(u: f64) -> f64 {
    // u >= 0 -> exp(-u) via 1 / (1 + u + ... + u^5/120) clamped for u > 20 (exact cross-lang parity!)
    if u > 20.0 { return 0.0; }
    1.0 / (1.0 + u * (1.0 + u * (0.5 + u * (0.16666666666666666 + u * (0.041666666666666664 + u * 0.008333333333333333)))))
}

fn b098_softmax_2d(mat: &[f64], n: usize) -> i64 {
    let mut acc = 0.0f64;
    let mut row_tmp = vec![0.0f64; n];
    for r in 0..n {
        let row = &mat[r * n..(r + 1) * n];
        let mut mx = -1e30f64;
        for &v in row { if v > mx { mx = v; } }
        let mut s = 0.0f64;
        for c in 0..n { let e = fast_exp_neg(mx - row[c]); row_tmp[c] = e; s += e; }
        let inv = 1.0 / s;
        for c in 0..n { acc += row_tmp[c] * inv * ((c % 17) as f64 + 1.0); }
    }
    (black_box(acc) * 1000.0) as i64
}

fn b099_flash_attn(q: &[f64], k: &[f64], v: &[f64], nheads: usize, seq: usize, d: usize) -> i64 {
    let scale = 1.0 / (d as f64).sqrt();
    let mut scores = vec![0.0f64; seq];
    let mut acc = 0.0f64;
    for h in 0..nheads {
        let base = h * seq * d;
        for i in 0..seq {
            let qi = &q[base + i * d..base + (i + 1) * d];
            let mut mx = -1e30f64;
            for j in 0..seq {
                let kj = &k[base + j * d..base + (j + 1) * d];
                let mut dot = 0.0f64;
                for m in 0..d { dot += qi[m] * kj[m]; }
                let sc = dot * scale;
                scores[j] = sc;
                if sc > mx { mx = sc; }
            }
            let mut sum_e = 0.0f64;
            for j in 0..seq { let e = fast_exp_neg(mx - scores[j]); scores[j] = e; sum_e += e; }
            let inv = 1.0 / sum_e;
            for m in 0..d {
                let mut out_m = 0.0f64;
                for j in 0..seq { out_m += scores[j] * inv * v[base + j * d + m]; }
                acc += out_m;
            }
        }
    }
    (black_box(acc) * 1000.0) as i64
}

fn b100_kmeans_lloyd(pts: &[f64], npts: usize, dim: usize, k: usize, iters: usize) -> i64 {
    let mut cent = vec![0.0f64; k * dim];
    cent.copy_from_slice(&pts[..k * dim]);
    let mut sums = vec![0.0f64; k * dim];
    let mut cnts = vec![0usize; k];
    for _ in 0..iters {
        sums.fill(0.0); cnts.fill(0);
        for i in 0..npts {
            let p = &pts[i * dim..(i + 1) * dim];
            let mut best = f64::MAX;
            let mut bid = 0usize;
            for c in 0..k {
                let cv = &cent[c * dim..(c + 1) * dim];
                let mut d = 0.0f64;
                for m in 0..dim { let df = p[m] - cv[m]; d += df * df; }
                if d < best { best = d; bid = c; }
            }
            cnts[bid] += 1;
            let srow = &mut sums[bid * dim..(bid + 1) * dim];
            for m in 0..dim { srow[m] += p[m]; }
        }
        for c in 0..k {
            if cnts[c] > 0 {
                let inv = 1.0 / (cnts[c] as f64);
                for m in 0..dim { cent[c * dim + m] = sums[c * dim + m] * inv; }
            }
        }
    }
    let s: f64 = cent.iter().sum();
    (black_box(s) * 1000.0) as i64
}

// ==================== Main Harness (100 Benchmarks) ====================

fn main() {
    // Prepare deterministic shared inputs matching `axon_100_extreme.ax` bit-for-bit
    let mat4_a: Vec<i64> = (0..32_000 * 16).map(|i| (((i * 7 + 3) & 31) as i64) - 15).collect();
    let mat4_b: Vec<i64> = (0..32_000 * 16).map(|i| (((i * 11 + 5) & 31) as i64) - 15).collect();
    let a_f64_1m: Vec<f64> = (0..1_000_000usize).map(|i| ((i % 97) as f64) * 0.01 - 0.5).collect();
    let b_f64_1m: Vec<f64> = (0..1_000_000usize).map(|i| ((i % 89) as f64) * 0.01 - 0.4).collect();
    let x_f32_1m: Vec<f32> = (0..1_000_000usize).map(|i| ((i % 83) as f32) * 0.1 - 1.5).collect();
    let y_f32_1m: Vec<f32> = (0..1_000_000usize).map(|i| ((i % 71) as f32) * 0.05 - 0.5).collect();
    let src_i64_512k: Vec<i64> = (0..524_288i64).map(|i| ((i.wrapping_mul(17) + 3) & 255) - 128).collect();
    let data_2m: Vec<u8> = (0..2_000_000usize).map(|i| ((i * 31 + 17) & 255) as u8).collect();
    let arr_i64_2m: Vec<i64> = (0..2_000_000i64).map(|i| (i.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407)) >> 16).collect();
    let mat512_f64: Vec<f64> = (0..512 * 512).map(|i| ((i % 101) as f64) * 0.01 - 0.5).collect();

    // D3 Parse inputs
    let cnt_p21 = 250_000usize;
    let mut p21_buf = vec![0u8; cnt_p21 * 11];
    for r in 0..cnt_p21 {
        let mut x = (r as u64) * 19937 + 1234567890;
        let off = r * 11;
        for d in (0..10).rev() { p21_buf[off + d] = b'0' + (x % 10) as u8; x /= 10; }
        p21_buf[off + 10] = b'\n';
    }
    let mut hay_2m = vec![b'a'; 2_000_000];
    for i in 0..2_000_000 { hay_2m[i] = b'a' + (i % 26) as u8; }
    let ndl = b"AXON_NEEDLE!";
    let mut p = 1000usize;
    while p + 12 < 2_000_000 { hay_2m[p..p + 12].copy_from_slice(ndl); p += 4096; }

    let mut json_30k = Vec::with_capacity(30_000 * 32);
    for i in 0..30_000usize {
        let s = format!("{{\"id\":{},\"val\":{}}}\n", i, (i * 7 + 3) % 10000);
        json_30k.extend_from_slice(s.as_bytes());
    }
    let b512k: Vec<u8> = (0..524_288usize).map(|i| ((i * 37 + 11) & 255) as u8).collect();
    let mut csv_50k = Vec::with_capacity(50_000 * 24);
    for i in 0..50_000usize {
        let s = format!("{},{},{},{}\n", i, (i * 3) % 1000, (i * 17 + 5) % 9999, (i * 7) % 500);
        csv_50k.extend_from_slice(s.as_bytes());
    }
    let b512k_hex: Vec<u8> = (0..524_288usize).map(|i| ((i * 29 + 7) & 255) as u8).collect();
    let mut utf8_2m = vec![0u8; 2_000_000];
    for i in 0..2_000_000 { utf8_2m[i] = if (i % 7) == 0 { 0xC3 } else if (i % 7) == 1 { 0xA9 } else { b'a' + (i % 26) as u8 }; }
    let mut rle_512k = vec![0u8; 524_288];
    for i in 0..524_288 { rle_512k[i] = ((i / 11) & 31) as u8; }
    let varint_250k: Vec<u64> = (0..250_000u64).map(|i| (i.wrapping_mul(2654435761)) & 0x0FFF_FFFF).collect();
    let mut url_50k = Vec::with_capacity(50_000 * 32);
    for i in 0..50_000usize {
        let s = format!("a={}&b={}&c={}\n", i % 1000, (i * 13) % 5000, (i * 7) % 999);
        url_50k.extend_from_slice(s.as_bytes());
    }

    // D4 DS inputs
    let sort_48k: Vec<i64> = {
        let mut v = Vec::with_capacity(48_000);
        let mut s: u64 = 0x1234_5678_9ABC_DEF0;
        for _ in 0..48_000 { s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407); v.push((s >> 1) as i64); }
        v
    };
    let sorted_64k: Vec<i64> = (0..65536i64).map(|i| i * 3 + 1).collect();
    let bsearch_q: Vec<i64> = (0..80_000i64).map(|i| ((i.wrapping_mul(37) + 11) & 65535) * 3 + (i & 3)).collect();
    let topk_500k: Vec<i64> = (0..500_000i64).map(|i| ((i.wrapping_mul(2654435761) ^ (i >> 3)) & 0x7FFF_FFFF)).collect();
    let bs_a: Vec<u64> = (0..65536u64).map(|i| i.wrapping_mul(0x9E37_79B9_7F4A_7C15)).collect();
    let bs_b: Vec<u64> = (0..65536u64).map(|i| i.wrapping_mul(0xBF58_476D_1CE4_E5B9)).collect();
    let bs_c: Vec<u64> = (0..65536u64).map(|i| i.wrapping_mul(0x94D0_49BB_1331_11EB)).collect();
    let mut bloom_bits = vec![0u64; 1024]; // 65536 bits
    for i in 0..8000u64 {
        let h1 = i.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        let h2 = (h1 ^ (h1 >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        for &idx in &[(h1 & 65535) as usize, (h2 & 65535) as usize, ((h1.wrapping_add(h2)) & 65535) as usize] {
            bloom_bits[idx >> 6] |= 1u64 << (idx & 63);
        }
    }
    let bloom_q: Vec<u64> = (0..200_000u64).map(|i| i & 16383).collect();

    // D5 Graph inputs
    let mut adj_6k = vec![Vec::with_capacity(6); 6000];
    for u in 0..6000usize {
        for e in 0..6usize {
            let v = (u * 31 + e * 97 + 13) % 6000;
            let w = 1 + ((u + e * 7) % 100) as i64;
            adj_6k[u].push((v, w));
        }
    }
    let mut adj_bfs = vec![Vec::with_capacity(4); 16_000];
    for u in 0..16_000usize {
        for e in 0..4usize { adj_bfs[u].push((u * 13 + e * 47 + 7) % 16_000); }
    }
    let mut adj_dag = vec![Vec::with_capacity(4); 32_000];
    for u in 0..31_999usize {
        for e in 1..=4usize {
            let v = u + e * 3;
            if v < 32_000 { adj_dag[u].push(v); }
        }
    }
    let pr_n = 16_000usize;
    let mut pr_rp = vec![0usize; pr_n + 1];
    let mut pr_ci = Vec::with_capacity(pr_n * 6);
    let mut pr_od = vec![0.0f64; pr_n];
    for r in 0..pr_n {
        pr_rp[r] = pr_ci.len();
        for e in 0..6usize {
            let c = (r * 29 + e * 71 + 5) % pr_n;
            pr_ci.push(c);
            pr_od[c] += 1.0;
        }
    }
    pr_rp[pr_n] = pr_ci.len();
    for v in &mut pr_od { if *v == 0.0 { *v = 1.0; } }

    // D6 Crypto inputs
    let b2m_c: Vec<u8> = (0..2_097_152usize).map(|i| ((i * 31 + 17) & 255) as u8).collect();
    let mut crc_tbl = [0u32; 256];
    for i in 0..256u32 {
        let mut c = i;
        for _ in 0..8 { c = if (c & 1) != 0 { (c >> 1) ^ 0xEDB88320 } else { c >> 1 }; }
        crc_tbl[i as usize] = c;
    }
    let b512k_sha: Vec<u8> = (0..524_288usize).map(|i| ((i * 19 + 3) & 255) as u8).collect();
    let words_256k: Vec<u64> = (0..262_144u64).map(|i| i.wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(0x1234567)).collect();
    let hmac_256k: Vec<u8> = (0..262_144usize).map(|i| ((i * 13 + 5) & 255) as u8).collect();

    // D7 Signal inputs
    let dct_data: Vec<f64> = (0..192 * 64).map(|i| ((i % 64) as f64) * 0.04 - 1.2).collect();
    let mut dct_cos = vec![0.0f64; 64 * 64];
    for k in 0..64 {
        for n in 0..64 {
            dct_cos[k * 64 + n] = (std::f64::consts::PI / 64.0 * ((n as f64) + 0.5) * (k as f64)).cos();
        }
    }
    let fir_x: Vec<f64> = (0..131_136usize).map(|i| ((i % 64) as f64) * 0.03 - 1.0).collect();
    let mut fir_h = [0.0f64; 64];
    for k in 0..64 { fir_h[k] = 1.0 / (1.0 + ((k as i64 - 32).abs() as f64)); }
    let img_256: Vec<f64> = (0..256 * 256).map(|i| ((i % 128) as f64) * 0.02).collect();

    // D8 Graphics inputs
    let sobel_img: Vec<u8> = (0..512 * 512).map(|i| ((i * 17 + (i >> 9) * 31) & 255) as u8).collect();
    let blur_img: Vec<f32> = (0..512 * 512).map(|i| ((i % 251) as f32) * 0.01).collect();

    // D9 LinAlg inputs
    let a384: Vec<f32> = (0..384 * 384).map(|i| ((i % 37) as f32) * 0.01 - 0.18).collect();
    let b384: Vec<f32> = (0..384 * 384).map(|i| ((i % 41) as f32) * 0.01 - 0.20).collect();
    let a256: Vec<f64> = (0..256 * 256).map(|i| ((i % 37) as f64) * 0.01 - 0.18).collect();
    let b256: Vec<f64> = (0..256 * 256).map(|i| ((i % 41) as f64) * 0.01 - 0.20).collect();
    let sp_n = 32_768usize;
    let mut sp_rp = vec![0usize; sp_n + 1];
    let mut sp_ci = Vec::with_capacity(sp_n * 8);
    let mut sp_val = Vec::with_capacity(sp_n * 8);
    let sp_x: Vec<f64> = (0..sp_n).map(|i| ((i % 53) as f64) * 0.02 - 0.5).collect();
    for r in 0..sp_n {
        sp_rp[r] = sp_ci.len();
        for e in 0..8usize {
            sp_ci.push((r * 13 + e * 97 + 7) % sp_n);
            sp_val.push(0.1 + (e as f64) * 0.05);
        }
    }
    sp_rp[sp_n] = sp_ci.len();
    let rk_x: Vec<f64> = (0..512).map(|i| ((i % 29) as f64) * 0.02 - 0.3).collect();
    let rk_y: Vec<f64> = (0..512).map(|i| ((i % 31) as f64) * 0.02 - 0.3).collect();

    // D10 AI/LLM inputs
    let bnn_w: Vec<u64> = (0..4096 * 64).map(|i| (i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)).collect();
    let bnn_a: Vec<u64> = (0..64).map(|i| (i as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9)).collect();
    let act_2k: Vec<f32> = (0..2048).map(|i| ((i % 31) as f32) * 0.02 - 0.3).collect();
    let mut q4_2k = vec![0u8; 2048 * 64 * 20];
    let mut q8_2k = vec![0u8; 2048 * 64 * 36];
    for r in 0..2048 * 64 {
        let off4 = r * 20;
        q4_2k[off4..off4 + 4].copy_from_slice(&0.05f32.to_le_bytes());
        for b in 0..16 { q4_2k[off4 + 4 + b] = ((r + b * 7) & 0xFF) as u8; }
        let off8 = r * 36;
        q8_2k[off8..off8 + 4].copy_from_slice(&0.01f32.to_le_bytes());
        for b in 0..32 { q8_2k[off8 + 4 + b] = (((r + b * 5) % 251) as i32 - 125) as i8 as u8; }
    }
    let bitnet_w: Vec<i8> = (0..2048 * 2048).map(|i| ((i % 3) as i8) - 1).collect();
    let faiss_v: Vec<f32> = (0..5000 * 128).map(|i| ((i % 53) as f32) * 0.02 - 0.5).collect();
    let faiss_q: Vec<f32> = (0..16 * 128).map(|i| ((i % 47) as f32) * 0.02 - 0.4).collect();
    let cos_v: Vec<f32> = (0..4096 * 64).map(|i| ((i % 43) as f32) * 0.02 - 0.4 + 0.05).collect();
    let cos_q: Vec<f32> = (0..8 * 64).map(|i| ((i % 37) as f32) * 0.02 - 0.3 + 0.05).collect();
    let llm_x: Vec<f32> = (0..1024).map(|i| ((i % 29) as f32) * 0.03 - 0.4).collect();
    let llm_nw: Vec<f32> = (0..1024).map(|i| 1.0 + ((i % 11) as f32) * 0.01).collect();
    let llm_w1: Vec<f32> = (0..1024 * 1024).map(|i| ((i % 19) as f32) * 0.002 - 0.018).collect();
    let llm_w2: Vec<f32> = (0..1024 * 1024).map(|i| ((i % 23) as f32) * 0.002 - 0.022).collect();
    let sm_mat: Vec<f64> = (0..384 * 384).map(|i| ((i % 61) as f64) * 0.05 - 1.5).collect();
    let att_q: Vec<f64> = (0..4 * 128 * 64).map(|i| ((i % 37) as f64) * 0.03 - 0.5).collect();
    let att_k: Vec<f64> = (0..4 * 128 * 64).map(|i| ((i % 41) as f64) * 0.03 - 0.6).collect();
    let att_v: Vec<f64> = (0..4 * 128 * 64).map(|i| ((i % 43) as f64) * 0.03 - 0.6).collect();
    let km_pts: Vec<f64> = (0..1536 * 32).map(|i| (((i * 17 + 13) % 1009) as f64) * 0.002 - 1.0).collect();

    let mut times = Vec::with_capacity(100);
    let mut chks = Vec::with_capacity(100);
    macro_rules! run {
        ($e:expr) => {{
            let t0 = Instant::now();
            let c = black_box($e);
            times.push(t0.elapsed().as_micros() as u64);
            chks.push(c);
        }};
    }

    // D1
    run!(b001_branchless_clamp(4_000_000));
    run!(b002_fib_rec());
    run!(b003_sieve(500_000));
    run!(b004_popcnt(4_000_000));
    run!(b005_splitmix64(3_000_000));
    run!(b006_collatz(32_000));
    run!(b007_gcd_euclid(64_000));
    run!(b008_morton3d(4_000_000));
    run!(b009_modexp(48_000));
    run!(b010_mat4x4_int(&mat4_a, &mat4_b, 32_000));
    // D2
    run!(b011_dot_f64(&a_f64_1m, &b_f64_1m));
    run!(b012_saxpy_f32(&x_f32_1m, &y_f32_1m));
    run!(b013_prefix_sum_i64(&src_i64_512k));
    run!(b014_histogram_256(&data_2m));
    run!(b015_minmax_i64(&arr_i64_2m));
    run!(b016_relu6_f32(&x_f32_1m));
    run!(b017_l1_l2_f64(&a_f64_1m));
    run!(b018_conv1d_k9_f32(&x_f32_1m[..524_288]));
    run!(b019_variance_f64(&a_f64_1m));
    run!(b020_mat_transpose_512(&mat512_f64, 512));
    // D3
    run!(b021_parse_i64(&p21_buf, 11, cnt_p21));
    run!(b022_mem_find(&hay_2m));
    run!(b023_json_parse(&json_30k));
    run!(b024_base64_rt(&b512k));
    run!(b025_csv_agg(&csv_50k));
    run!(b026_hex_rt(&b512k_hex));
    run!(b027_utf8_count(&utf8_2m));
    run!(b028_rle_rt(&rle_512k));
    run!(b029_varint_pb(&varint_250k));
    run!(b030_url_query(&url_50k));
    // D4
    run!(b031_sort_i64(&sort_48k));
    run!(b032_hashmap(12_000));
    run!(b033_spsc_ring(1_000_000));
    run!(b034_bsearch(&sorted_64k, &bsearch_q));
    run!(b035_dsu(8, 4096, 16_000));
    run!(b036_fenwick(8, 4096, 16_000));
    run!(b037_topk_heap(&topk_500k, 64));
    run!(b038_bitset_popcnt(&bs_a, &bs_b, &bs_c));
    run!(b039_bloom_filter(&bloom_bits, &bloom_q));
    run!(b040_radix_hist_scatter(&topk_500k));
    // D5
    run!(b041_dijkstra(&adj_6k));
    run!(b042_bfs(&adj_bfs));
    run!(b043_toposort(&adj_dag));
    run!(b044_floyd_warshall(160));
    run!(b045_pagerank(&pr_rp, &pr_ci, &pr_od, pr_n));
    run!(b046_knapsack_dp());
    run!(b047_edit_dist(96, 128));
    run!(b048_lcs_dp(96, 128));
    run!(b049_coin_change(8, 12_000));
    run!(b050_lis_patience(32, 2048));
    // D6
    run!(b051_fnv1a_batch(&b2m_c));
    run!(b052_crc32(&b2m_c, &crc_tbl));
    run!(b053_sha256_batch(&b512k_sha));
    run!(b054_chacha20(16_384));
    run!(b055_siphash24_batch(&words_256k));
    run!(b056_murmur3_batch(&words_256k));
    run!(b057_xxhash64_batch(&words_256k));
    run!(b058_blake2s_batch(8192));
    run!(b059_hmac_sha256(&hmac_256k));
    run!(b060_speck_ctr(262_144));
    // D7
    run!(b061_fft_16384());
    run!(b062_dct_ii(&dct_data, &dct_cos, 192));
    run!(b063_fir_64tap(&fir_x, &fir_h));
    run!(b064_iir_biquad(64, 4096));
    run!(b065_rk4_lorenz(128, 1000));
    run!(b066_gl16_quad(32_768));
    run!(b067_mc_pi(4_000_000));
    run!(b068_black_scholes(48_000));
    run!(b069_newton_cubic(48_000));
    run!(b070_bilinear_interp(&img_256, 256, 512));
    // D8
    run!(b071_nbody_3d(96, 50));
    run!(b072_mandelbrot(320));
    run!(b073_julia(224));
    run!(b074_ray_sphere(48_000));
    run!(b075_aabb_overlap(1024));
    run!(b076_sobel_edge(&sobel_img, 512));
    run!(b077_gauss_blur(&blur_img, 512));
    run!(b078_heat_2d(256, 16));
    run!(b079_verlet_3d(65_536, 12));
    run!(b080_polygon_pip(64_000));
    // D9
    run!(b081_fp32_gemm(&a384, &b384, 384));
    run!(b082_fp64_gemm(&a256, &b256, 256));
    run!(b083_csr_spmv(&sp_rp, &sp_ci, &sp_val, &sp_x, sp_n));
    run!(b084_cholesky(160));
    run!(b085_lu_factor(8, 96));
    run!(b086_qr_mgs(128));
    run!(b087_jacobi_sym(8, 32));
    run!(b088_tridiag_thomas(2048, 128));
    run!(b089_cg_sparse(8192, 16));
    run!(b090_outer_rank1(&rk_x, &rk_y, 512));
    // D10
    run!(b091_bnn_gemv(&bnn_w, &bnn_a, 4096, 64));
    run!(b092_q4_0_gemv(&act_2k, &q4_2k, 2048, 2048, 2));
    run!(b093_q8_0_gemv(&act_2k, &q8_2k, 2048, 2048));
    run!(b094_bitnet_158(&act_2k, &bitnet_w, 2048, 2048));
    run!(b095_faiss_l2(&faiss_v, &faiss_q, 5000, 16, 128));
    run!(b096_cosine_topk(&cos_v, &cos_q, 4096, 8, 64));
    run!(b097_llm_ffn(&llm_x, &llm_nw, &llm_w1, &llm_w2, 1024));
    run!(b098_softmax_2d(&sm_mat, 384));
    run!(b099_flash_attn(&att_q, &att_k, &att_v, 4, 128, 64));
    run!(b100_kmeans_lloyd(&km_pts, 1536, 32, 16, 4));

    print!("{{\"lang\":\"Rust\",\"times_us\":[");
    for (i, t) in times.iter().enumerate() { if i > 0 { print!(","); } print!("{}", t); }
    print!("],\"chks\":[");
    for (i, c) in chks.iter().enumerate() { if i > 0 { print!(","); } print!("{}", c); }
    println!("]}}");
}
