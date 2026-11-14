use std::hint::black_box;
use std::time::Instant;

fn main() {
    let n = 10_000_000;
    let iters = 10;
    let total_ops = n * iters;
    let data: Vec<i64> = (0..n as i64).collect();

    println!("=== RUST BOUNDS CHECK BENCHMARK ===");
    println!("Total lookups: {} (100 Million array accesses)\n", total_ops);

    // 1. Rust Checked Indexing
    let t0 = Instant::now();
    let mut sum_checked: i64 = 0;
    for _ in 0..iters {
        let mut s: i64 = 0;
        for i in 0..n {
            s = s.wrapping_add(black_box(data[i]));
        }
        sum_checked = sum_checked.wrapping_add(s);
    }
    let ms_checked = t0.elapsed().as_secs_f64() * 1000.0;
    let tp_checked = (total_ops as f64 / 1e6) / (ms_checked / 1000.0);
    println!(
        "  [Rust Checked Index]     Total: {:8.2} ms | {:5.2} ns/op | Throughput: {:8.2} M ops/s (chk: {})",
        ms_checked,
        (ms_checked * 1e6) / total_ops as f64,
        tp_checked,
        sum_checked
    );

    // 2. Rust Unchecked (simulating Axon Refinement Elided)
    let t0 = Instant::now();
    let mut sum_elided: i64 = 0;
    for _ in 0..iters {
        let mut s: i64 = 0;
        for i in 0..n {
            unsafe {
                s = s.wrapping_add(black_box(*data.get_unchecked(i)));
            }
        }
        sum_elided = sum_elided.wrapping_add(s);
    }
    let ms_elided = t0.elapsed().as_secs_f64() * 1000.0;
    let tp_elided = (total_ops as f64 / 1e6) / (ms_elided / 1000.0);
    println!(
        "  [Rust Unchecked/Elided]  Total: {:8.2} ms | {:5.2} ns/op | Throughput: {:8.2} M ops/s (chk: {})",
        ms_elided,
        (ms_elided * 1e6) / total_ops as f64,
        tp_elided,
        sum_elided
    );

    println!(
        "\n  Speedup: {:.2}x faster",
        ms_checked / ms_elided
    );
}
