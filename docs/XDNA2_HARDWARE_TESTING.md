# AMD XDNA2 Hardware Testing Guide

This guide is for running the `dragon-npu` Rust library on real AMD XDNA2 hardware (Ryzen AI 300 series / Strix Point, Strix Halo, Krackan Point, etc.).

The goal is to validate that detection, status reporting, and the overall `DragonNpu` API behave correctly when real hardware is present.

## Prerequisites

- A machine with AMD XDNA2 NPU (e.g. Ryzen AI 9 HX 370, etc.).
- The `amdxdna` Linux kernel driver loaded and working.
- Rust toolchain (stable) with `cargo`.
- The `amd-xdna2` feature must be enabled at build time.

You can verify the driver is present with:

```bash
ls /sys/class/accel/
# You should see entries like accel0, accel1, etc. if the driver is active.
```

## Building with XDNA2 Support

All XDNA2-specific code lives behind the `amd-xdna2` Cargo feature.

```bash
# From the root of the dragon-npu repository
cargo build --features amd-xdna2
```

For development and testing:

```bash
cargo test --features amd-xdna2
```

## Recommended Test Commands

Run the full test suite with the XDNA2 feature enabled (good smoke test):

```bash
cargo test --features amd-xdna2 -- --nocapture
```

### Best Way to Inspect Real Hardware (Recommended)

Use the dedicated probe example:

```bash
cargo run --features amd-xdna2 --example xdna2_probe
```

It now prints both the normal `AmdXdna2Backend::new()` path (enhanced simulation with realistic XDNA2 parameters) and the forced `new_with_real_stub()` path (exercises the Real execution arm plumbing). Minimal key=value output only. This is the fastest way for hardware owners to validate everything.

Capability checklist + JSON timed op (colibrì tools surface):

```bash
cargo run --features amd-xdna2 --example npu_direct_bench -- --checklist
cargo run --features amd-xdna2 --example npu_direct_bench -- --shape q_a
```

See `docs/XDNA2_DIRECT_CAPABILITIES.md` for the frozen open/GET_INFO/BO/hwctx/submit/timed_gemm table and CREATE_HWCTX driver requirements (`qos_p`, DevHeap mmap, `ext_flags==0`).

### Real Device Layer Integration Test (Primary Validation for Production Open Path)

For end-to-end validation of the **real (non-simulation) XDNA2 device open + diagnostics layer** on your physical hardware, use the dedicated integration test:

```bash
cargo test --features amd-xdna2 --test xdna2_real_hardware -- --nocapture
```

- On your Ryzen AI 300 / XDNA2 + amdxdna system this will:
  - Construct via `new_with_real_stub()`
  - Assert `real_execution_active()` and `hardware_detected()`
  - Exercise the multi-index `/dev/accel` open helper and the public `real_device_open()` / `real_device_path()` diagnostics
  - Reach the `submit_and_run` and `compile_model` error paths inside `RealXdna2Executor` (proving the production boundary is wired)
- On any other machine it detects the lack of 1022:17f0 amdxdna hardware via the probe and skips cleanly with a message (no test failures, the binary remains valid).
- This is the cargo-native way to prove the "device open + observability layer is solid" on the exact target platform.

Run the probe example first for quick human-readable output, then this integration test for automated gate-style validation of the real arm.

### Specific High-Value Tests

```bash
# The main preference + honesty test
cargo test --features amd-xdna2 dragon_npu_with_amd_xdna2_feature_prefers_xdna2_backend -- --nocapture
```

Run the example above first — it will give the clearest picture of what your hardware reports.

## What to Look For on Real Hardware

When running on a machine with working XDNA2 hardware, you should see:

- `hardware_detected()` → `true`
- `detected_device()` → `Some(...)` with real values for:
  - `pci_vendor`: `"0x1022"`
  - `pci_device`: `"0x17f0"` (or family equivalent)
  - `npu_name`: something like the name reported by the driver (e.g. from `/sys/class/accel/.../name`)
  - `xdna2_variant`: `Some("NPU5")` or `Some("NPU4")` / `Some("NPU6")` depending on your chip
- `NpuStatus` from `status()` or `DragonNpu::status()`:
  - `vendor`: `"AMD"`
  - `device_name`: contains `"AMD XDNA2"` and `"hardware detected"`
  - `simulated`: `false`
  - `peak_tops`: around 50.0
  - `supported_dtypes` should prominently include `bf16`
  - `driver_version`: something containing `amdxdna` and the detected revision

Real model loading (via `compile_model` with a `.onnx` file) now extracts actual initializer/weight tensor data (float32 with shape + values) when present in the protobuf. This data is available via `CompiledModel::initializers()` and will be used for future constant folding and real execution paths on your hardware.

On machines without real XDNA2 hardware (CI, other laptops), the same code paths must still work and report `simulated: true` with clear messaging.

## Using the Library Programmatically (Quick Smoke Test)

You can write a tiny test in your own project or run this in a `#[test]`:

```rust
use dragon_npu::DragonNpu;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let npu = DragonNpu::new()?;
    let status = npu.status()?;

    println!("Vendor: {}", status.vendor);
    println!("Device: {}", status.device_name);
    println!("Simulated: {}", status.simulated);
    println!("Peak TOPS: {}", status.peak_tops);
    println!("Detected device: {:?}", npu.backend.as_ref()); // if you expose it

    // On real hardware you should see simulated == false and good numbers
    Ok(())
}
```

Build it with `--features amd-xdna2`.

## What Feedback Is Most Useful

Please share:

1. Output of the key tests above (especially anything unexpected).
2. The exact values from `detected_device()` and `status()` on your hardware.
3. Your chip / laptop model (e.g. "Ryzen AI 9 HX 370 in Framework laptop" or "Strix Halo desktop").
4. Kernel version and whether you needed any special steps to get the amdxdna driver working.
5. Any crashes, panics, or surprising behavior.

## Real Hardware Validation Data (2026-05-30)

The following is the exact output from the first successful run of `xdna2_probe` on physical AMD XDNA2 hardware (Ryzen AI 300 series / Strix Point class, PCI 1022:17f0 rev 0x20, amdxdna driver active):

```
hardware_detected=true
detected_npu_type=Some("XDNA2 (17f0 rev 0x20)")
device.name=accel0
device.driver=amdxdna
device.pci=0x1022:0x17f0 rev=0x20
device.npu_name=(empty — common on some systems)
device.modalias=pci:v00001022d000017F0sv000017AAsd0000512Ebc11sc80i00
device.variant=Some("XDNA2")
status.simulated=false
status.device_name=AMD XDNA2 (17f0 rev 0x20) - hardware detected
status.driver_version=amdxdna (XDNA2 0x20 detected)
status.peak_tops=50
status.compute_units=8
status.memory_mb=8192
status.dtypes=["fp32", "fp16", "bf16", "int8"]
real_execution_active=true   # because the probe used new_with_real_stub()

--- submit_and_run on real stub (expected) ---
compile_for_submit_and_run_error=CompilationFailed { model: "model.onnx", reason: "real XDNA2 execution not fully implemented yet (device detected; production submission logic under active development)" }
```

Notes from this run:
- Detection, PCI ID, driver, and `simulated=false` are all correct.
- `npu_name` being empty is common; the library gracefully falls back to PCI + revision.
- The `detected_npu_type` redundancy ("XDNA2 ... / XDNA2") was fixed in the same session.
- `real_execution_active=true` only when explicitly using the stub constructor (normal `.new()` path still uses the high-fidelity simulation arm with XDNA2-realistic parameters until the real submission path is implemented).

## Next Steps After Initial Validation

Once basic detection + status look good, we can move on to:
- Compiling and running small real models through the XDNA2 path
- Comparing simulated vs hardware inference time and accuracy
- Continuing development of real submission paths for production use on XDNA2 hardware

---

Thank you for testing on real hardware. This is the highest-leverage next step for the project.

If anything in these instructions is unclear or the tests don't behave as expected, open an issue or paste the output here and we'll fix it quickly.
