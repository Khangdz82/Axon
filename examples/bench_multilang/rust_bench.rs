use std::time::Instant;

// --- W1: Matrix GEMM 256x256 FP32 ---
fn bench_gemm() -> f64 {
    let n = 256;
    let mut a = vec![0.5f32; n * n];
    let b = vec![0.5f32; n * n];
    let mut c = vec![0.0f32; n * n];

    let t0 = Instant::now();
    for i in 0..n {
        for k in 0..n {
            let aik = a[i * n + k];
            for j in 0..n {
                c[i * n + j] += aik * b[k * n + j];
            }
        }
    }
    let elapsed = t0.elapsed().as_secs_f64() * 1000.0;
    if c[0] < 0.0 { println!("checksum: {}", c[0]); }
    elapsed
}

// --- W2: BNN 1-Bit POPCOUNT GEMV (4096 rows x 4096 cols = 64 words) ---
fn bench_bnn() -> f64 {
    let rows = 4096;
    let words = 64;
    let weights = vec![0x5555555555555555u64; rows * words];
    let act = vec![0xAAAAAAAAAAAAAAAAu64; words];
    let mut out = vec![0i32; rows];

    let t0 = Instant::now();
    for r in 0..rows {
        let mut sum = 0i32;
        let base = r * words;
        for w in 0..words {
            let x = weights[base + w] ^ act[w];
            sum += x.count_ones() as i32;
        }
        out[r] = sum;
    }
    let elapsed = t0.elapsed().as_secs_f64() * 1000.0;
    if out[0] < 0 { println!("checksum: {}", out[0]); }
    elapsed
}

// --- W3: Quicksort (100,000 int64) ---
fn quicksort_i64(arr: &mut [i64], low: isize, high: isize) {
    if low >= high { return; }
    let pivot = arr[high as usize];
    let mut i = low - 1;
    for j in low..high {
        if arr[j as usize] <= pivot {
            i += 1;
            arr.swap(i as usize, j as usize);
        }
    }
    arr.swap((i + 1) as usize, high as usize);
    let pi = i + 1;

    quicksort_i64(arr, low, pi - 1);
    quicksort_i64(arr, pi + 1, high);
}

fn bench_sort() -> f64 {
    let n = 100000;
    let mut arr = vec![0i64; n];
    let mut seed = 42u64;
    for i in 0..n {
        seed = (seed.wrapping_mul(1103515245) + 12345) & 0x7FFFFFFF;
        arr[i] = (seed % 1000000) as i64;
    }

    let t0 = Instant::now();
    let high = (n - 1) as isize;
    quicksort_i64(&mut arr, 0, high);
    let elapsed = t0.elapsed().as_secs_f64() * 1000.0;

    if arr[0] < 0 { println!("checksum: {}", arr[0]); }
    elapsed
}

// --- W4: Sieve of Eratosthenes (2,000,000) ---
fn bench_sieve() -> f64 {
    let n = 2000000;
    let mut is_prime = vec![true; n + 1];
    is_prime[0] = false;
    is_prime[1] = false;

    let t0 = Instant::now();
    let mut p = 2;
    while p * p <= n {
        if is_prime[p] {
            let mut i = p * p;
            while i <= n {
                is_prime[i] = false;
                i += p;
            }
        }
        p += 1;
    }

    let mut count = 0;
    for i in 2..=n {
        if is_prime[i] { count += 1; }
    }
    let elapsed = t0.elapsed().as_secs_f64() * 1000.0;

    if count == 0 { println!("checksum: {}", count); }
    elapsed
}

// --- W5: 3D Particle Kinematics (50,000 particles x 10 steps) ---
#[derive(Clone, Copy)]
struct Particle {
    x: f32, y: f32, z: f32,
    vx: f32, vy: f32, vz: f32,
}

fn bench_physics() -> f64 {
    let n = 50000;
    let mut p = vec![Particle {
        x: 0.0, y: 100.0, z: 0.0,
        vx: 1.0, vy: 0.0, vz: 0.5,
    }; n];
    let dt = 0.016f32;
    let g = -9.81f32;

    let t0 = Instant::now();
    for _step in 0..10 {
        for i in 0..n {
            p[i].x += p[i].vx * dt;
            p[i].y += p[i].vy * dt;
            p[i].z += p[i].vz * dt;
            p[i].vy += g * dt;
        }
    }
    let elapsed = t0.elapsed().as_secs_f64() * 1000.0;

    if p[0].y < -9999.0 { println!("checksum: {}", p[0].y); }
    elapsed
}

// --- W6: JSON Scanner (100,000 runs) ---
fn bench_json() -> f64 {
    let doc = b"{\"order_id\": 9876543, \"user_id\": 42109, \"symbol\": \"BTCUSD\", \"qty\": 1500, \"price\": 96500, \"status\": \"FILLED\"}";
    let doc_len = doc.len();
    let key = b"\"price\": ";
    let key_len = key.len();

    let t0 = Instant::now();
    let mut sum_price: i64 = 0;
    for _ in 0..100000 {
        for i in 0..=(doc_len - key_len) {
            if doc[i] == b'\"' && doc[i + 1] == b'p' {
                if &doc[i..i + key_len] == key {
                    let mut val = 0i64;
                    let mut k = i + key_len;
                    while k < doc_len && doc[k] >= b'0' && doc[k] <= b'9' {
                        val = val * 10 + (doc[k] - b'0') as i64;
                        k += 1;
                    }
                    sum_price += val;
                    break;
                }
            }
        }
    }
    let elapsed = t0.elapsed().as_secs_f64() * 1000.0;

    if sum_price == 0 { println!("checksum: {}", sum_price); }
    elapsed
}

fn main() {
    let t_gemm = bench_gemm();
    let t_bnn = bench_bnn();
    let t_sort = bench_sort();
    let t_sieve = bench_sieve();
    let t_phys = bench_physics();
    let t_json = bench_json();

    println!("{{\"lang\": \"Rust (rustc -O3)\", \"gemm\": {:.2}, \"bnn\": {:.2}, \"sort\": {:.2}, \"sieve\": {:.2}, \"physics\": {:.2}, \"json\": {:.2}}}",
             t_gemm, t_bnn, t_sort, t_sieve, t_phys, t_json);
}
