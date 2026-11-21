use std::time::Instant;

fn bench_nbody() -> (f64, i64) {
    let n = 256usize;
    let steps = 40;
    let dt = 0.01f64;
    let mut px = vec![0.0f64; n];
    let mut py = vec![0.0f64; n];
    let mut pz = vec![0.0f64; n];
    let mut vx = vec![0.0f64; n];
    let mut vy = vec![0.0f64; n];
    let mut vz = vec![0.0f64; n];
    for i in 0..n { px[i] = i as f64 * 0.1 + 1.0; py[i] = i as f64 * 0.2 - 0.5; pz[i] = i as f64 * 0.05; }
    let t0 = Instant::now();
    for _ in 0..steps {
        for a in 0..n {
            let (ax, ay, az) = (px[a], py[a], pz[a]);
            let (mut fx, mut fy, mut fz) = (0.0, 0.0, 0.0);
            for b in 0..n {
                if a != b {
                    let (dx, dy, dz) = (px[b] - ax, py[b] - ay, pz[b] - az);
                    let d2 = dx*dx + dy*dy + dz*dz + 0.01;
                    let inv = 1.0 / (d2 * d2.sqrt());
                    fx += dx * inv; fy += dy * inv; fz += dz * inv;
                }
            }
            vx[a] += fx * dt; vy[a] += fy * dt; vz[a] += fz * dt;
        }
        for k in 0..n { px[k] += vx[k]*dt; py[k] += vy[k]*dt; pz[k] += vz[k]*dt; }
    }
    let ms = t0.elapsed().as_secs_f64() * 1000.0;
    let sum: f64 = (0..n).map(|i| px[i] + py[i]).sum();
    (ms, (sum + 0.5) as i64)
}

fn bench_matmul64() -> (f64, i64) {
    let n = 192usize;
    let nn = n * n;
    let a: Vec<f64> = (0..nn).map(|i| (i % 17) as f64 * 0.25).collect();
    let b: Vec<f64> = (0..nn).map(|i| (i % 13) as f64 * 0.5).collect();
    let mut c = vec![0.0f64; nn];
    let t0 = Instant::now();
    for i in 0..n {
        let in_off = i * n;
        for k in 0..n {
            let aik = a[in_off + k];
            let kn_off = k * n;
            for j in 0..n { c[in_off + j] += aik * b[kn_off + j]; }
        }
    }
    let ms = t0.elapsed().as_secs_f64() * 1000.0;
    (ms, (c[0] + c[nn - 1]) as i64)
}

fn bench_arx_crypto() -> (f64, i64) {
    let t0 = Instant::now();
    let (mut a, mut b, mut c, mut d) = (0x61707865u32, 0x3320646Eu32, 0x79622D32u32, 0x6B206574u32);
    for _ in 0..2_000_000 {
        a = a.wrapping_add(b); d ^= a; d = d.rotate_left(16);
        c = c.wrapping_add(d); b ^= c; b = b.rotate_left(12);
        a = a.wrapping_add(b); d ^= a; d = d.rotate_left(8);
        c = c.wrapping_add(d); b ^= c; b = b.rotate_left(7);
    }
    let ms = t0.elapsed().as_secs_f64() * 1000.0;
    (ms, (a ^ b ^ c ^ d) as i64)
}

fn bench_dijkstra() -> (f64, i64) {
    let (nv, deg) = (10000usize, 8usize);
    let ne = nv * deg;
    let inf = 1_000_000_000_000i64;
    let mut to = vec![0usize; ne];
    let mut wt = vec![0i64; ne];
    let mut dist = vec![inf; nv];
    let mut hp_d = vec![0i64; ne + 16];
    let mut hp_u = vec![0usize; ne + 16];
    let mut seed: i64 = 1234567;
    for u in 0..nv {
        for e in 0..deg {
            seed = (seed.wrapping_mul(1103515245).wrapping_add(12345)) & 0x7FFFFFFF;
            to[u*deg + e] = (u + 1 + (seed as usize % 97)) % nv;
            wt[u*deg + e] = 1 + ((seed >> 8) % 100);
        }
    }
    let t0 = Instant::now();
    dist[0] = 0; let mut hsz = 1usize; hp_d[0] = 0; hp_u[0] = 0;
    while hsz > 0 {
        let (d, cur) = (hp_d[0], hp_u[0]); hsz -= 1;
        if hsz > 0 {
            let (ld, lu) = (hp_d[hsz], hp_u[hsz]); let mut p = 0;
            loop {
                let l = p*2 + 1; if l >= hsz { break; }
                let sm = if l+1 < hsz && hp_d[l+1] < hp_d[l] { l+1 } else { l };
                if ld <= hp_d[sm] { break; }
                hp_d[p] = hp_d[sm]; hp_u[p] = hp_u[sm]; p = sm;
            }
            hp_d[p] = ld; hp_u[p] = lu;
        }
        if d > dist[cur] { continue; }
        let base = cur * deg;
        for e in 0..deg {
            let (nxt, nd) = (to[base + e], d + wt[base + e]);
            if nd < dist[nxt] {
                dist[nxt] = nd; let mut idx = hsz; hsz += 1;
                while idx > 0 {
                    let par = (idx - 1) >> 1;
                    if hp_d[par] <= nd { break; }
                    hp_d[idx] = hp_d[par]; hp_u[idx] = hp_u[par]; idx = par;
                }
                hp_d[idx] = nd; hp_u[idx] = nxt;
            }
        }
    }
    let ms = t0.elapsed().as_secs_f64() * 1000.0;
    (ms, dist.iter().sum())
}

fn bench_knapsack() -> (f64, i64) {
    let (n, cap) = (200usize, 15000usize);
    let mut dp = vec![0i64; cap + 1];
    let mut wt = vec![0usize; n];
    let mut vl = vec![0i64; n];
    let mut seed: i64 = 9876543;
    for i in 0..n {
        seed = (seed.wrapping_mul(1103515245).wrapping_add(12345)) & 0x7FFFFFFF;
        wt[i] = 10 + (seed as usize % 300);
        vl[i] = 5 + ((seed >> 8) % 500);
    }
    let t0 = Instant::now();
    for i in 0..n {
        let (w, v) = (wt[i], vl[i]);
        for c in (w..=cap).rev() {
            let cand = dp[c - w] + v;
            if cand > dp[c] { dp[c] = cand; }
        }
    }
    let ms = t0.elapsed().as_secs_f64() * 1000.0;
    (ms, dp[cap])
}

fn bench_hashmap() -> (f64, i64) {
    let (n, cap) = (200_000i64, 524_288usize);
    let mask = cap - 1;
    let mut keys = vec![0i64; cap];
    let mut vals = vec![0i64; cap];
    let t0 = Instant::now();
    for i in 1..=n {
        let mut k1 = (i.wrapping_mul(2654435761)) & 0x7FFFFFFF; if k1 == 0 { k1 = 1; }
        let mut slot = (((k1 as u64).wrapping_mul(11400714819323198485)) >> 45) as usize & mask;
        loop {
            if keys[slot] == 0 || keys[slot] == k1 { keys[slot] = k1; vals[slot] = i*3 + 7; break; }
            slot = (slot + 1) & mask;
        }
    }
    let mut acc = 0i64;
    for i in 1..=n {
        let mut k1 = (i.wrapping_mul(2654435761)) & 0x7FFFFFFF; if k1 == 0 { k1 = 1; }
        let mut slot = (((k1 as u64).wrapping_mul(11400714819323198485)) >> 45) as usize & mask;
        loop {
            if keys[slot] == k1 { acc += vals[slot]; break; }
            if keys[slot] == 0 { break; }
            slot = (slot + 1) & mask;
        }
    }
    let ms = t0.elapsed().as_secs_f64() * 1000.0;
    (ms, acc)
}

fn main() {
    let (t1, c1) = bench_nbody();
    let (t2, c2) = bench_matmul64();
    let (t3, c3) = bench_arx_crypto();
    let (t4, c4) = bench_dijkstra();
    let (t5, c5) = bench_knapsack();
    let (t6, c6) = bench_hashmap();
    println!("{{\"lang\": \"Rust (rustc -O3)\", \"nbody\": {:.2}, \"matmul64\": {:.2}, \"arx\": {:.2}, \"dijkstra\": {:.2}, \"knapsack\": {:.2}, \"hashmap\": {:.2}, \"chk\": [{}, {}, {}, {}, {}, {}]}}",
             t1, t2, t3, t4, t5, t6, c1, c2, c3, c4, c5, c6);
}
