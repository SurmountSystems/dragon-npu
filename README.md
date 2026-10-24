## Development status (2026-10-12)

This product is Earth Dragon NPU. The package and the command are `earth-dragon-npu`. The library crate is `earth_dragon_npu`.

The Rust minimum for this crate is 1.99. `cargo update` and `cargo audit` were run on the stable 1.99 toolchain. The audit loaded 1307 security advisories from the RustSec database, scanned Cargo.lock (159 crate dependencies), and exited 0. No advisory warning remained. The earlier anymap2 advisory (RUSTSEC-2026-0319) closed because tract-onnx is now 0.23.8 and that line no longer pulls in liquid or anymap2. That was an upgrade. It was not an audit ignore.

Product Pass is still no. invent_pass stays 0. ran_on_npu stays false. The first unlock line remains `2c9c12e8` consumed. Do not mint a new unlock. Do not reopen a consumed one.

An IRON catalog child PASS is an external runner. It is not `decide_ran_on_npu`. That function, in `src/backends/amd_xdna2/numerical.rs`, is a pure check. It returns true only when the host golden and the device buffer are both non-empty, the host golden is not all zeros, the caller recorded a real gated submit-and-wait, an instruction blob was present, the path was a real ioctl, and the float values match within the relative and absolute tolerances. A length mismatch, a NaN, or an infinity fails the match. The function does not open a device and does not submit work. `XDNA2_LAB_ALLOW_ZERO_GOLDEN` does not override the all-zero golden refusal.

`testdata/xdna2_cmd/external_known_good/` holds research peels (300, 660, 2592, and 4512 bytes). Those peels are not a load-accepted lab matmul. The 1688-byte file `testdata/xdna2_cmd/silicon_lab/lab_instr.bin` is the unpatched lab body.

The hardware outcome still open is one matmul this crate submits, the device finishes, and the output matches `decide_ran_on_npu`.

The single next offline step is not started. The person who holds a residual-matched classic 1688-byte capture places that file under `testdata/xdna2_cmd/external_known_good/`.

# Earth Dragon NPU (Rust)

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-Edition%202024-blue.svg)](https://www.rust-lang.org/)
[![Linux](https://img.shields.io/badge/platform-Linux-green.svg)](https://www.linux.org/)

**Earth Dragon NPU** is the production-quality Rust rewrite of the original Python framework. It brings vendor-agnostic NPU acceleration to Linux with a focus on safety, explicit error handling, and performance.

This crate is developed incrementally using Test-Driven Development (TDD). The Python implementation remains untouched as the reference for API parity and behavior. See [README.old.md](README.old.md) for the full original documentation.

For the complete rewrite strategy, phases, error model, and TDD workflow, read [PLAN.md](PLAN.md). Session notes: [AGENTS.md](AGENTS.md).

## Quick start (library)

Built for Rust Edition 2024.

```bash
cargo add earth-dragon-npu
# or, from this repo:
cargo test --lib
cargo test --features amd-xdna2 --lib
# secondary / research features (honest stubs; see docs/SECONDARY_BACKENDS.md):
cargo test --features intel-vpu,qualcomm-hexagon --lib
cargo test --features xrt-bridge --lib
```

**MSRV:** Rust **1.99+** (edition 2024 + crate APIs). Packaging policy: [docs/PACKAGING.md](docs/PACKAGING.md).

```rust
use earth_dragon_npu::{DragonNpu, DragonNpuError};

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

## CLI (`earth-dragon-npu`)

Thin wrappers over the library. **Simulation works without hardware.** Does **not** enable EXEC_CMD. The command people type is `earth-dragon-npu`.

```bash
earth-dragon-npu status
earth-dragon-npu status --json
earth-dragon-npu compile tests/fixtures/host_matmul_relu.json -O 2
earth-dragon-npu run tests/fixtures/host_matmul_relu.json --input-len 4
# optional XDNA2 diagnostics (no EXEC_CMD; needs the amd-xdna2 Cargo feature):
earth-dragon-npu probe
```

| Subcommand | Role |
|------------|------|
| `status` | Backend status (human or `--json`) |
| `compile <model>` | Compile only; prints nodes / passes / score |
| `run <model>` | Compile + `run()` with zeros or `--input` (JSON list or raw LE f32) |
| `probe` | Needs `amd-xdna2`; safe GET_INFO-style diagnostics only |

Exit codes: `0` success, `1` error, `2` usage. Never exit 0 claiming silicon GEMM Pass.

### Named IRON (`just npu-relu`, `--iron`)

Finds the sibling `mlir-aie` tree (you do not need `$MLIR_AIE`) and runs
a named example. Catalog: `vector_add`, `eltwise`, `relu`, `softmax`,
`gelu`, `silu`, `eltwise_mul`, `swiglu`, `scale_shift`. Child `PASS!` is
still **not** product `ran_on_npu`. `job=iron_npu2_vector_add` is not
classic 1688. Host linear is `earth-dragon-npu run --host-graph` (needs
the `amd-xdna2` Cargo feature).

```bash
just npu-relu          # IRON ReLU
just npu-eltwise       # IRON eltwise add
just npu-softmax       # IRON softmax
just npu-gelu          # IRON GELU
just npu-silu          # IRON SiLU
just npu-mul           # IRON eltwise mul
just npu-swiglu        # IRON SwiGLU
just npu-scale-shift   # IRON scale-and-shift
just npu-iron          # IRON vector add run+verify wrap (no --aot-dir)
just npu-iron-export   # export-only collect (writes artifacts; no child PASS! expected)
earth-dragon-npu run --iron relu
just npu-iron-paths    # print resolved trees
just install           # cargo install --path . --force
```

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

## NPU access privileges (product)

NPU access is a **normal device node** (`/dev/accel/accel0`). **Install may need
admin once** (driver, firmware, udev/group). **Day-to-day inference does not
need sudo.** Temporary root is only for lab research gates on hardened modules
(see [docs/XDNA2_NEVER_AGAIN.md](docs/XDNA2_NEVER_AGAIN.md) § Product vs lab
privileges). Device type **KMQ** (`device_type=0` on this host) means the kernel
owns the queue, not "users must be root."

## Garuda / zen + hardened amdxdna (lab hosts)

On Garuda (or Arch zen) with Ryzen AI / XDNA2:

1. **Linker PATH:** some Garuda toolchains put non-GNU `ld` first. Prefer:
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
| EXEC_CMD by default | **Never.** Double-gated |
| Intel VPU / Hexagon | **Honest stubs** (`available=false`, run refuses) |
| `xrt-bridge` | Research stub only. Never default. Never the hot path |

### Optional Cargo features

| Feature | Default | Role |
|---------|---------|------|
| `simulation` | yes | Host simulation |
| `amd-xdna2` | no | Primary XDNA2 direct path |
| `xdna2-dangerous-exec-cmd` | no | L1 EXEC_CMD compile gate (+ env) |
| `intel-vpu` | no | Stub. Not auto-selected |
| `qualcomm-hexagon` | no | Stub. Not auto-selected |
| `xrt-bridge` | no | Research only. Not enabled by `amd-xdna2` |

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
- **CLI**: command `earth-dragon-npu`. Package `earth-dragon-npu`. Library crate `earth_dragon_npu`.
- **Error handling**: single `DragonNpuError` via `thiserror`. Every public API uses `Result`

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
cargo build --bins
```

## License

Earth Dragon NPU is released under the MIT License. See [LICENSE](LICENSE) for details.
