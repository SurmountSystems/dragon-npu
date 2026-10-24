//! Criterion bench: safe XDNA2 direct path only (feature `amd-xdna2`).
//!
//! ```bash
//! cargo bench --features amd-xdna2 --bench xdna2_direct -- --no-run
//! cargo bench --features amd-xdna2 --bench xdna2_direct
//! ```
//!
//! **Never** enables EXEC_CMD. Times status/open-path diagnostics and optional
//! `timed_bo_roundtrip` when a real device is open. Does not claim `ran_on_npu`.

#![cfg(feature = "amd-xdna2")]
#![allow(missing_docs)]

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use earth_dragon_npu::backends::amd_xdna2::AmdXdna2Backend;

fn xdna2_status_path(c: &mut Criterion) {
    c.bench_function("xdna2_new_with_real_stub_status", |b| {
        b.iter(|| {
            let backend = AmdXdna2Backend::new_with_real_stub();
            let open = backend.real_device_open();
            let detected = backend.hardware_detected();
            black_box((open, detected));
        });
    });
}

fn xdna2_get_info_if_open(c: &mut Criterion) {
    let backend = AmdXdna2Backend::new_with_real_stub();
    if !backend.real_device_open() {
        // Still register a no-op so the bench group compiles and runs on CI.
        c.bench_function("xdna2_get_info_skipped_no_device", |b| {
            b.iter(|| black_box(0u32));
        });
        return;
    }
    c.bench_function("xdna2_get_direct_firmware", |b| {
        b.iter(|| {
            let fw = backend.get_direct_firmware_version();
            black_box(fw.is_ok());
        });
    });
}

fn xdna2_bo_roundtrip_if_open(c: &mut Criterion) {
    let backend = AmdXdna2Backend::new_with_real_stub();
    if !backend.real_device_open() {
        c.bench_function("xdna2_bo_roundtrip_skipped_no_device", |b| {
            b.iter(|| black_box(0u32));
        });
        return;
    }
    // Safe DMA path only — no EXEC_CMD.
    c.bench_function("xdna2_timed_bo_roundtrip_4k", |b| {
        b.iter(|| {
            let r = backend.timed_bo_roundtrip(4096);
            // Honesty: never treat as NPU GEMM.
            assert!(!r.ran_on_npu);
            black_box(r.total_s);
        });
    });
}

criterion_group!(
    benches,
    xdna2_status_path,
    xdna2_get_info_if_open,
    xdna2_bo_roundtrip_if_open
);
criterion_main!(benches);
