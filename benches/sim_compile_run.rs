//! Criterion bench: simulation compile + run (always available).
//!
//! ```bash
//! cargo bench --bench sim_compile_run
//! # CI / compile-only:
//! cargo bench --bench sim_compile_run -- --no-run
//! # or:
//! cargo bench --no-run
//! ```
//!
//! Never opens `/dev/accel` intentionally — uses `DragonNpu::new()` default path
//! (simulation without `amd-xdna2`; with feature may prefer XDNA2 backend but
//! still times host/sim compile+run of a tiny JSON fixture).

#![allow(missing_docs)]

use std::hint::black_box;
use std::path::PathBuf;

use criterion::{Criterion, criterion_group, criterion_main};
use earth_dragon_npu::DragonNpu;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/host_matmul_relu.json")
}

fn sim_compile(c: &mut Criterion) {
    let path = fixture_path();
    let path_str = path.to_string_lossy().into_owned();
    c.bench_function("sim_compile_opt2", |b| {
        b.iter(|| {
            let npu = DragonNpu::new().expect("DragonNpu::new");
            let model = npu
                .compile_model(black_box(&path_str), black_box(2), black_box("fp32"))
                .expect("compile");
            black_box(model.node_count());
        });
    });
}

fn sim_compile_run(c: &mut Criterion) {
    let path = fixture_path();
    let path_str = path.to_string_lossy().into_owned();
    let npu = DragonNpu::new().expect("DragonNpu::new");
    let model = npu
        .compile_model(&path_str, 2, "fp32")
        .expect("compile once");
    let input = vec![1.0_f32, 0.0, 0.0, 1.0];
    c.bench_function("sim_run_matmul_relu_fixture", |b| {
        b.iter(|| {
            let out = model.run(black_box(&input)).expect("run");
            black_box(out.len());
        });
    });
}

criterion_group!(benches, sim_compile, sim_compile_run);
criterion_main!(benches);
