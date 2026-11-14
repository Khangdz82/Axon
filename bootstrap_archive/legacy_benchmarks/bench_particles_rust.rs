use std::time::Instant;

#[derive(Clone, Copy)]
struct ParticleAoS {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
}

fn integrate_aos(particles: &mut [ParticleAoS], dt: f32) {
    for p in particles.iter_mut() {
        p.x += p.vx * dt;
        p.y += p.vy * dt;
    }
}

struct ParticleSoA {
    x: Vec<f32>,
    y: Vec<f32>,
    vx: Vec<f32>,
    vy: Vec<f32>,
}

impl ParticleSoA {
    fn new(n: usize) -> Self {
        let mut x = Vec::with_capacity(n);
        let mut y = Vec::with_capacity(n);
        let mut vx = Vec::with_capacity(n);
        let mut vy = Vec::with_capacity(n);
        for i in 0..n {
            x.push(i as f32);
            y.push(i as f32 * 0.5);
            vx.push(1.0);
            vy.push(-0.5);
        }
        Self { x, y, vx, vy }
    }

    fn integrate(&mut self, dt: f32) {
        let len = self.x.len();
        let x = self.x.as_mut_slice();
        let y = self.y.as_mut_slice();
        let vx = self.vx.as_slice();
        let vy = self.vy.as_slice();

        for i in 0..len {
            x[i] += vx[i] * dt;
            y[i] += vy[i] * dt;
        }
    }
}

fn run_bench(n: usize, frames: usize, dt: f32) {
    println!("--- Scale: N = {} particles, {} frames ---", n, frames);

    // 1. Rust AoS
    let mut aos: Vec<ParticleAoS> = (0..n)
        .map(|i| ParticleAoS {
            x: i as f32,
            y: i as f32 * 0.5,
            vx: 1.0,
            vy: -0.5,
        })
        .collect();

    let t0 = Instant::now();
    for _ in 0..frames {
        integrate_aos(&mut aos, dt);
    }
    let aos_ms = t0.elapsed().as_secs_f64() * 1000.0;
    let aos_tp = ((n * frames) as f64 / 1e6) / (aos_ms / 1000.0);
    println!(
        "  [Rust AoS]           Total: {:8.2} ms | {:6.3} ms/f | Throughput: {:8.2} M/s | Chk: {:.2}",
        aos_ms,
        aos_ms / frames as f64,
        aos_tp,
        aos[0].x
    );

    // 2. Rust SoA
    let mut soa = ParticleSoA::new(n);
    let t0 = Instant::now();
    for _ in 0..frames {
        soa.integrate(dt);
    }
    let soa_ms = t0.elapsed().as_secs_f64() * 1000.0;
    let soa_tp = ((n * frames) as f64 / 1e6) / (soa_ms / 1000.0);
    println!(
        "  [Rust SoA]           Total: {:8.2} ms | {:6.3} ms/f | Throughput: {:8.2} M/s | Chk: {:.2}\n",
        soa_ms,
        soa_ms / frames as f64,
        soa_tp,
        soa.x[0]
    );
}

fn main() {
    println!("=== RUST PARTICLE SIMULATION BENCHMARK ===\n");
    let dt = 0.016;
    run_bench(100_000, 100, dt);
    run_bench(1_000_000, 100, dt);
    run_bench(10_000_000, 100, dt);
}
