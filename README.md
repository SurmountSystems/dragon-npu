# 🐉 DragonNPU (Rust)

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-Edition%202024-blue.svg)](https://www.rust-lang.org/)
[![Linux](https://img.shields.io/badge/platform-Linux-green.svg)](https://www.linux.org/)

**DragonNPU** is the production-quality Rust rewrite of the original Python DragonNPU framework. It brings vendor-agnostic NPU acceleration to Linux with a focus on safety, explicit error handling, and performance.

This crate is developed incrementally using Test-Driven Development (TDD). The Python implementation remains untouched as the reference for API parity and behavior. See [README.old.md](README.old.md) for the full original documentation.

For the complete rewrite strategy, phases, error model, and TDD workflow, read [PLAN.md](PLAN.md). Session notes: [AGENTS.md](AGENTS.md).

## Quick start (library)

Built for Rust Edition 2024.

```bash
cargo add dragon-npu
# or, from this repo:
cargo test --lib
cargo test --features amd-xdna2 --lib
# secondary / research features (honest stubs — see docs/SECONDARY_BACKENDS.md):
cargo test --features intel-vpu,qualcomm-hexagon --lib
cargo test --features xrt-bridge --lib
```

**MSRV:** Rust **1.87+** (edition 2024 + crate APIs). Packaging policy: [docs/PACKAGING.md](docs/PACKAGING.md).

```rust
use dragon_npu::{DragonNpu, DragonNpuError};

fn main() -> Result<(), DragonNpuError> {
    let npu = DragonNpu::new()?;
    let status = npu.status()?;
    println!("NPU: {:?}", status);

    let model = npu.compile_model("model.onnx", /* optimization_level */ 3, /* quantization */ "fp16")?;
    let input = vec![0.0f32; 3 * 224 * 224];
    let output = model.run(&input)?;

    println!("Inference complete, output len: {}", output.len());
    Ok(())
}
```

All public library APIs return `Result<T, DragonNpuError>`. No `unwrap()` / panics in library code.

## CLI (`dragon-npu`)

Thin wrappers over the library. **Simulation works without hardware.** Does **not** enable EXEC_CMD.

```bash
cargo run --bin dragon-npu -- status
cargo run --bin dragon-npu -- status --json
cargo run --bin dragon-npu -- compile tests/fixtures/host_matmul_relu.json -O 2
cargo run --bin dragon-npu -- run tests/fixtures/host_matmul_relu.json --input-len 4
# optional XDNA2 diagnostics (no EXEC_CMD):
cargo run --features amd-xdna2 --bin dragon-npu -- probe
```

| Subcommand | Role |
|------------|------|
| `status` | Backend status (human or `--json`) |
| `compile <model>` | Compile only; prints nodes / passes / score |
| `run <model>` | Compile + `run()` with zeros or `--input` (JSON list or raw LE f32) |
| `probe` | Needs `amd-xdna2`; safe GET_INFO-style diagnostics only |

Exit codes: `0` success, `1` error, `2` usage. Never exit 0 claiming silicon GEMM Pass.

## Examples

```bash
# Minimal host/sim infer (no hardware required)
cargo run --example minimal_infer

# XDNA2 bring-up matrix (open, GET_INFO, hwctx grid; EXEC_CMD double-gated)
cargo run --features amd-xdna2 --example xdna2_probe

# Colibrì-friendly timed JSON (see docs/COLIBRI_ADAPTER.md)
cargo run --features amd-xdna2 --example npu_direct_bench -- --checklist
cargo run --features amd-xdna2 --example npu_direct_bench -- --bo-roundtrip
```

## Benchmarks (Criterion)

```bash
# Compile benches only (CI-friendly)
cargo bench --no-run
cargo bench --features amd-xdna2 --no-run

# Run simulation bench
cargo bench --bench sim_compile_run

# Safe XDNA2 direct timings only (status / GET_INFO / bo_roundtrip if device open)
cargo bench --features amd-xdna2 --bench xdna2_direct
```

Benches **never** enable EXEC_CMD and never claim `ran_on_npu` GEMM Pass.

## Garuda / zen + hardened amdxdna (lab hosts)

On Garuda (or Arch zen) with Ryzen AI / XDNA2:

1. **Linker PATH:** some Garuda “wild” toolchains put non-GNU `ld` first. Prefer:
   ```bash
   export PATH=/usr/bin:$PATH
   ```
2. **Hardened module (L2+L3 materials in-tree):** follow
   [docs/XDNA2_HARDENED_INSTALL.md](docs/XDNA2_HARDENED_INSTALL.md).
   Read-only verify:
   ```bash
   bash scripts/verify_amdxdna_hardened_host.sh
   ```
3. **Safety permanent gates:** [docs/XDNA2_NEVER_AGAIN.md](docs/XDNA2_NEVER_AGAIN.md)
   - Cargo feature `xdna2-dangerous-exec-cmd` is **OFF by default** (not implied by `amd-xdna2`)
   - Runtime still requires `XDNA2_ALLOW_EXEC_CMD=1` for any EXEC_CMD
   - Host module param `exec_cmd_allow=0` by default (L3)
   - **Do not** set `ALLOW=1` casually; empty scaffold EXEC oopsed stock amdxdna (2026-07-13)

## What works today vs blocked

| Capability | Status |
|------------|--------|
| Simulation compile/run | Works (default) |
| ONNX/JSON load + host graph (`host_ref`) | Works for allowlisted ops + weights |
| Real open + GET_INFO (FW, power, TOPS) | Works on lab host with amdxdna |
| CREATE_HWCTX / destroy | Pass on hardened host (probe/checklist) |
| BO roundtrip DMA (`ran_on_device_path`) | Honest silicon DMA when Real open |
| NPU GEMM / `ran_on_npu` product Pass | **Blocked** until known-good txn + OFM numerical verify ([TXN research](docs/XDNA2_TXN_RESEARCH.md)) |
| EXEC_CMD by default | **Never** — double-gated |
| Intel VPU / Hexagon | **Honest stubs** (`available=false`, run refuses) |
| `xrt-bridge` | Research stub only — never default / never hot path |

### Optional Cargo features

| Feature | Default | Role |
|---------|---------|------|
| `simulation` | yes | Host simulation |
| `amd-xdna2` | no | Primary XDNA2 direct path |
| `xdna2-dangerous-exec-cmd` | no | L1 EXEC_CMD compile gate (+ env) |
| `intel-vpu` | no | Stub — not auto-selected |
| `qualcomm-hexagon` | no | Stub — not auto-selected |
| `xrt-bridge` | no | Research only — not enabled by `amd-xdna2` |

Details: [docs/SECONDARY_BACKENDS.md](docs/SECONDARY_BACKENDS.md),
[docs/XDNA2_PACKAGING.md](docs/XDNA2_PACKAGING.md) (DKMS vs upstream),
[docs/PACKAGING.md](docs/PACKAGING.md) (crates.io / MSRV / CI).

Roadmap Phases 0–9: see [PLAN.md](PLAN.md). Colibrì JSON contract: [docs/COLIBRI_ADAPTER.md](docs/COLIBRI_ADAPTER.md). Direct capabilities: [docs/XDNA2_DIRECT_CAPABILITIES.md](docs/XDNA2_DIRECT_CAPABILITIES.md). Instruction/txn research + capture prep: [docs/XDNA2_TXN_RESEARCH.md](docs/XDNA2_TXN_RESEARCH.md) (`scripts/inventory_txn_sources.sh` report-only; `scripts/capture_instruction_blob_checklist.md`).

## Development status

- Real ONNX loading (tract-onnx), compiler passes, Phase 7 typed IR (`DragonNpuIR`).
- AMD XDNA2 lightweight direct path (Phase 0–7 bring-up + Phase 8 product surface).
- Strict TDD + `cargo clippy -- -D warnings` on both default and `amd-xdna2` features.

Hardware owner guide: [docs/XDNA2_HARDWARE_TESTING.md](docs/XDNA2_HARDWARE_TESTING.md).

## High-level architecture

- **Core**: NPU discovery, status, device abstraction (`core.rs`)
- **Compiler**: Model parsing, optimization, quantization (`compiler.rs`)
- **IR**: Typed dual IR (`ir.rs`)
- **Backends**: Simulation + AMD XDNA2 (feature); Intel/Hexagon honest stubs; optional research `xrt-bridge`
- **CLI**: optional binary `dragon-npu` (`src/bin/dragon_npu.rs` + `cli` module)
- **Error handling**: single `DragonNpuError` via `thiserror` — every public API uses `Result`

## Contributing

Please follow [AGENTS.md](AGENTS.md) and [PLAN.md](PLAN.md). All changes must pass:

```bash
export PATH=/usr/bin:$PATH
cargo fmt -- --check
cargo clippy -- -D warnings
cargo clippy --features amd-xdna2 -- -D warnings
cargo clippy --features intel-vpu,qualcomm-hexagon -- -D warnings
cargo clippy --features xrt-bridge -- -D warnings
cargo test --lib
cargo test --features amd-xdna2 --lib
cargo test --features intel-vpu,qualcomm-hexagon --lib
cargo test --features xrt-bridge --lib
cargo build --bin dragon-npu
```

## License

DragonNPU is released under the MIT License. See [LICENSE](LICENSE) for details.
