//! Minimal honest inference example (host / simulation path).
//!
//! ```bash
//! cargo run --example minimal_infer
//! cargo run --features amd-xdna2 --example minimal_infer
//! ```
//!
//! Loads the tiny JSON fixture under `tests/fixtures/`, compiles, runs, and
//! prints whether the backend reports `simulated`. **Never** claims
//! `ran_on_npu` or silicon GEMM Pass.

use earth_dragon_npu::DragonNpu;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let fixture =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/host_matmul_relu.json");
    let path = fixture.to_string_lossy();

    let npu = DragonNpu::new()?;
    let status = npu.status()?;
    println!(
        "backend vendor={} device={} simulated={}",
        status.vendor, status.device_name, status.simulated
    );

    let model = npu.compile_model(&path, 2, "fp32")?;
    println!(
        "compiled nodes={} (initial={}) score={} ops={:?}",
        model.node_count(),
        model.initial_node_count(),
        model.optimization_score(),
        model.ops()
    );

    // Identity 2x2 weight in fixture; input length 4 matches 2x2 flatten.
    let input = vec![1.0_f32, 2.0, 3.0, 4.0];
    let output = model.run(&input)?;
    println!("input={input:?}");
    println!("output={output:?}");
    if let Some(ms) = model.last_inference_time_ms() {
        println!("inference_time_ms={ms:.4}");
    }
    println!("ran_on_npu=false (honest: this example is host/sim run only)");
    if status.simulated {
        println!("path=simulation");
    } else {
        println!("path=hardware_backend_status_not_simulated (run() still may be host/sim)");
    }
    Ok(())
}
