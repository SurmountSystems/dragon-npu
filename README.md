# 🐉 DragonNPU (Rust)

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-Edition%202024-blue.svg)](https://www.rust-lang.org/)
[![Linux](https://img.shields.io/badge/platform-Linux-green.svg)](https://www.linux.org/)

**DragonNPU** is the production-quality Rust rewrite of the original Python DragonNPU framework. It brings vendor-agnostic NPU acceleration to Linux with a focus on safety, explicit error handling, and performance.

This crate is developed incrementally using Test-Driven Development (TDD). The Python implementation remains untouched as the reference for API parity and behavior. See [README.old.md](README.old.md) for the full original documentation, features, performance benchmarks, installation instructions, and examples.

For the complete rewrite strategy, phases, error model, and TDD workflow, read [PLAN.md](PLAN.md).

## Quick Start

Built for Rust Edition 2024.

```bash
cargo add dragon-npu
```

## Basic Usage

All operations return `Result<T, DragonNpuError>` with descriptive variants. No `unwrap()`, `expect()`, or panics are used in the library.

```rust
use dragon_npu::{DragonNpu, DragonNpuError};

fn main() -> Result<(), DragonNpuError> {
    // Initialize and check NPU status
    let npu = DragonNpu::new()?;
    let status = npu.status()?;
    println!("NPU: {:?}", status);

    // Compile a model (ONNX, PyTorch, etc.)
    let model = npu.compile_model("model.onnx", /* optimization_level */ 3, /* quantization */ "fp16")?;

    // Run inference
    let input = vec![0.0f32; 3 * 224 * 224]; // example tensor
    let output = model.run(&input)?;

    println!("Inference complete, output len: {}", output.len());
    Ok(())
}
```

## Development Status

Production-grade Rust implementation with real ONNX loading (tract-onnx + protobuf), full compiler passes with observable graph mutations, and strong AMD XDNA2 focus.

- Real hardware detection for AMD XDNA2 (Ryzen AI 300 / Strix Point etc.) is validated on physical devices (`simulated=false`, correct PCI + amdxdna driver, realistic 50 TOPS / bf16 parameters).
- `submit_and_run` is a first-class method on the `Backend` trait and `CompiledModel`, with a clean hook for future real NPU submission (currently exercises high-fidelity simulation or the explicit Real stub with actionable errors).
- Strict TDD + `cargo clippy -- -D warnings` discipline throughout.

See [docs/XDNA2_HARDWARE_TESTING.md](docs/XDNA2_HARDWARE_TESTING.md) for hardware owner guidance and real device output examples. Python reference files are kept only for API parity and behavior comparison (no bindings).

## AMD XDNA2 Hardware Testing

Real AMD XDNA2 hardware (Ryzen AI 300 / Strix Point and later) is the current primary focus.

If you have compatible hardware and the `amdxdna` driver, please see the dedicated testing guide:

→ **[docs/XDNA2_HARDWARE_TESTING.md](docs/XDNA2_HARDWARE_TESTING.md)**

It contains exact build commands, the most valuable tests to run, what output to expect on real hardware vs simulation, and what feedback is most useful.

Build with the feature:

```bash
cargo test --features amd-xdna2
```

The easiest way to see exactly what the library detects on your hardware is:

```bash
cargo run --features amd-xdna2 --example xdna2_probe
```

## High-Level Architecture

- **Core**: NPU discovery, status, device abstraction (`core.rs`)
- **Compiler**: Model parsing, optimization, quantization, code generation (`compiler.rs`)
- **Runtime**: Tensor management, execution, profiling (`runtime.rs`)
- **Backends**: Pluggable implementations (simulation always available; AMD XDNA, Intel VPU, etc.)
- **Error Handling**: Single `DragonNpuError` enum via `thiserror` — every public API uses `Result`

## Contributing

Please follow the guidelines in [AGENTS.md](AGENTS.md) and the detailed plan in [PLAN.md](PLAN.md). All changes must pass `cargo fmt`, `cargo clippy -- -D warnings`, and `cargo test`.

## License

DragonNPU is released under the MIT License. See [LICENSE](LICENSE) for details.