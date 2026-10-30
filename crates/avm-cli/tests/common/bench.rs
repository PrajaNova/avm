//! Timing helper for the benchmark suite (`bench.rs`): runs a closure many times, prints a
//! one-line summary and optionally appends a CSV row for tracking across commits.
use std::io::Write;
use std::time::Instant;

/// Latency statistics in milliseconds.
pub struct Stats {
    pub min: f64,
    pub median: f64,
    pub mean: f64,
    pub p95: f64,
    pub max: f64,
}

/// Scales a release-build budget for debug builds, which are roughly an order of magnitude slower.
pub fn budget_ms(release_ms: f64) -> f64 {
    if cfg!(debug_assertions) {
        release_ms * 10.0
    } else {
        release_ms
    }
}

/// Runs `op` `warmup` times unmeasured, then `iterations` times measured, and reports `name`.
/// When `AVM_BENCH_OUT` names a file, a `name,median,p95,mean` CSV row is appended to it.
pub fn measure(name: &str, warmup: usize, iterations: usize, mut op: impl FnMut()) -> Stats {
    for _ in 0..warmup {
        op();
    }
    let mut samples: Vec<f64> = (0..iterations)
        .map(|_| {
            let start = Instant::now();
            op();
            start.elapsed().as_secs_f64() * 1000.0
        })
        .collect();
    samples.sort_by(f64::total_cmp);
    let last = samples.len() - 1;
    let stats = Stats {
        min: samples[0],
        median: samples[last / 2],
        mean: samples.iter().sum::<f64>() / samples.len() as f64,
        p95: samples[(last as f64 * 0.95).round() as usize],
        max: samples[last],
    };
    println!(
        "{name:<34} n={iterations:<4} min {:>7.2}  median {:>7.2}  mean {:>7.2}  p95 {:>7.2}  max {:>7.2} ms",
        stats.min, stats.median, stats.mean, stats.p95, stats.max
    );
    if let Ok(path) = std::env::var("AVM_BENCH_OUT") {
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            let _ = writeln!(
                file,
                "{name},{:.3},{:.3},{:.3}",
                stats.median, stats.p95, stats.mean
            );
        }
    }
    stats
}

/// Fails when `stats.median` exceeds `limit_ms`, naming the benchmark.
pub fn assert_median_under(name: &str, stats: &Stats, limit_ms: f64) {
    assert!(
        stats.median < limit_ms,
        "{name}: median {:.2} ms exceeds the {limit_ms:.2} ms budget",
        stats.median
    );
}
