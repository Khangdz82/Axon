use std::time::Instant;

struct Vec4 {
    x: f32,
    y: f32,
    z: f32,
    w: f32,
}

#[inline(never)]
fn create_vec4_box(x: f32, y: f32, z: f32, w: f32) -> Box<Vec4> {
    Box::new(Vec4 { x, y, z, w })
}

fn main() {
    let iters = 10_000_000;
    println!("=== RUST BOX ALLOCATION BENCHMARK ===");
    println!("Iterations: {} (10 Million allocations)\n", iters);

    let t0 = Instant::now();
    let mut acc = 0.0f32;
    for i in 0..iters {
        let b = create_vec4_box(i as f32, (i + 1) as f32, (i + 2) as f32, (i + 3) as f32);
        acc += b.x + b.y + b.z + b.w;
        // dropped here
    }
    let elapsed = t0.elapsed();
    let ms = elapsed.as_secs_f64() * 1000.0;
    let ns_per_op = (ms * 1e6) / (iters as f64);
    let ops_per_sec = (iters as f64 / 1e6) / (ms / 1000.0);

    println!(
        "  [Rust Box::new + drop]   Total: {:8.2} ms | {:6.2} ns/op | Throughput: {:8.2} M allocs/s (acc: {:.1})",
        ms, ns_per_op, ops_per_sec, acc
    );
}
