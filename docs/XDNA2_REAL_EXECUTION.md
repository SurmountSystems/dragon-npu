# AMD XDNA2 Real Execution Path — Production Readiness

This document describes the current architecture and the plan for a **production-ready real hardware execution path** for AMD XDNA2 (Ryzen AI 300 series / Strix Point and similar, using the `amdxdna` Linux kernel driver).

## Current Architecture (as of 2026)

DragonNPU uses a clean pluggable design centered on the `Xdna2Executor` trait:

```rust
pub trait Xdna2Executor {
    fn compile_model(...) -> Result<CompiledModel, DragonNpuError>;
    fn run(&self, input: &[f32]) -> Result<Vec<f32>, DragonNpuError>;
    fn submit_and_run(&self, model: &CompiledModel, input: &[f32]) -> Result<Vec<f32>, DragonNpuError>;
    fn is_real(&self) -> bool;
}
```

- `AmdXdna2Backend` holds an `Execution` enum (`Simulation` or `Real`).
- `SimulationBackend` provides high-fidelity simulation (realistic timing via `peak_tops` + `latency_factor`, bf16-native modeling, tanh activation, etc.).
- The `RealXdna2Executor` path is the production target for actual hardware submission. It is under active development.
- `CompiledModel::submit_and_run(...)` and `AmdXdna2Backend::submit_and_run(...)` are the **stable public production boundary**.

`submit_and_run` is deliberately the primary method for real hardware work. It receives a fully compiled `CompiledModel` (which already contains real ONNX weights via `initializers()`, shapes, dtypes, applied optimizations, etc.).

## Path to Production-Ready Real Execution

### Philosophy: Lightweight Direct Driver Only

This project deliberately pursues a **pure lightweight, open, direct-to-amdxdna-driver** approach (no XRT, no heavy official runtimes). 

The goal is a focused, maintainable, lower-dependency implementation that talks straight to the mainline Linux `amdxdna` kernel driver via `/dev/accel/accelN` and the vendor ioctls defined in `amdxdna_accel.h`.

This matches the original fork motivation: a better, lighter, more open version instead of wrapping the official heavy stacks.

**Current state of the direct path**:
- Excellent device detection + multi-index open (pure std, no extra crates).
- Rich read-only observability (`real_device_*`, sysfs access).
- Typed high-level driver queries via `DRM_IOCTL_AMDXDNA_GET_INFO` (firmware version, power mode, resource info / TOPS, sensors, AIE version).
- Full `DirectSubmission` trait + ported uAPI structs; real CREATE_BO / SYNC_BO / GET_BO_INFO / GEM_CLOSE.
- CREATE_HWCTX: DevHeap 64 MiB + mmap, **valid `qos_p`** (driver always copy_from_user), tile multiples of 4; see `docs/XDNA2_DIRECT_CAPABILITIES.md`.
- EXEC_CMD scaffolding + research payload header (`build_exec_cmd_payload_scaffold`); firmware txn encoding still open.
- `timed_matmul` + `examples/npu_direct_bench` JSON surface for colibrì tools (honest skip statuses).

Actual GEMM / model run on silicon still depends on working hwctx **and** a known-good command BO payload.

### Safety / NEVER AGAIN (EXEC_CMD)

Scaffold EXEC_CMD previously NULL-deref'd `amdxdna` (`aie2_cmdlist_fill_slot`) and wedged
`open()` (2026-07-13). **Default and `amd-xdna2`-only builds permanently cannot issue
`DRM_IOCTL_AMDXDNA_EXEC_CMD`.**

| Layer | Requirement |
|-------|-------------|
| Compile | Cargo feature `xdna2-dangerous-exec-cmd` (default **OFF**, not implied by `amd-xdna2`) |
| Runtime | Env `XDNA2_ALLOW_EXEC_CMD=1` (or `true`) |
| API | `exec_cmd_is_enabled()` / `exec_cmd_block_reason()`; `RealDirectSubmission::submit_command` hard-fails otherwise |
| Kernel (if L3 lab module installed) | `modprobe amdxdna exec_cmd_allow=1` — default 0 refuses all EXEC_CMD subtypes with `-EPERM` |

Legacy `XDNA2_TRY_SUBMIT` is **ignored**. Never claim product Pass from a research scaffold.
Full policy: **`docs/XDNA2_NEVER_AGAIN.md`**. Capability status: **`docs/XDNA2_DIRECT_CAPABILITIES.md`**.  
Driver L2+L3 materials (null-guards + open fail-fast; **not** installed until clean boot/DKMS): **`kernel/patches/README.md`**.

### 2. Production Architecture Goals

- **Zero panics / unwraps** on the real path (already a project invariant).
- Clear separation: Simulation for development/CI, Real only when hardware is present and the direct lightweight path is active.
- `real_execution_active()` and `hardware_detected()` remain the primary signals.
- `CompiledModel` must eventually carry XDNA2-native artifacts (or enough information to produce them at submission time).
- Graceful degradation and rich error reporting when real submission fails (driver issues, thermal throttling, unsupported operators after compilation, etc.).

### 3. Key Production Concerns

- **Device lifecycle**: Opening/closing accel devices, context creation, queue management.
- **Model loading / compilation**: Our internal `ModelCompiler` + real ONNX weight data (multi-dtype + bf16 raw) is already strong. For the direct path we will produce (or feed) artifacts compatible with the amdxdna driver's command format.
- **Input/Output handling**: Efficient zero-copy or DMA paths from Rust `&[f32]` / tensors into NPU memory.
- **Quantization & dtype**: XDNA2 has excellent native bf16 and int8 support. Production path must preserve and exploit this (our `Initializer` + `as_f32_data()` already gives us the data).
- **Error modes on real hardware**: Thermal/power limits, command timeouts, driver resets, unsupported graph patterns.

## Optimization Strategies for Real XDNA2

### Compiler / Graph Level (already partially in place)
- Heavy use of our existing `FuseOperations`, `LayoutOptimization` (NHWC is usually preferred), `QuantizeWeights`, and `MemoryPlanning`.
- Weight-aware constant folding (onx-16 work) to reduce memory traffic.
- Operator fusion beyond what we have today (conv + bias + activation is critical on XDNA2).

### Hardware-Specific
- **Native bf16 / Block FP16**: XDNA2 (especially NPU5+) has very strong bf16 performance. Prefer bf16 quantization when accuracy allows.
- **Tiling / Multi-tile execution**: XDNA2 contains multiple AIE2 tiles. Production implementation should exploit data parallelism across tiles where the graph allows.
- **Memory layout & DMA**: Minimize host ↔ device copies. Use the weight data already extracted in `CompiledModel::initializers()` to pre-load constants.
- **Power & Thermal awareness**: Real execution should expose or respect power caps and thermal throttling. Add optional configuration for "performance" vs "sustained" modes.
- **Operator mapping**: Map our high-level ops to the most efficient XDNA2 kernels (many common ops have highly optimized implementations in the AMD stack).

### Future Compiler Improvements
- Better shape inference and static memory planning targeted at XDNA2's on-chip memory hierarchy.
- Graph partitioning for multi-tile or multi-NPU scenarios.
- Ahead-of-time compilation to XDNA2-specific artifacts (outside our current `ModelGraph`).

## Benchmarking Methodology

### Core Metrics (already partially captured)
- `last_inference_time_ms()` on `CompiledModel` (wall time including any sleeps we do in simulation for realism).
- `optimization_score()`, `fused_operations_count()`, `node_count()`, `memory_saved_mb()`.
- `has_real_weights()` + `total_weight_elements()` for model size analysis.

### Production Benchmarking Suite (recommended)

1. **Latency**
   - Warmup + steady-state single-batch latency (p50 / p99 / p999).
   - Use high-resolution timers (`std::time::Instant` or better, `perfcounter` if available).
   - Measure both `submit_and_run` and internal command submission time when possible.

2. **Throughput**
   - Sustained multi-inference throughput (images/sec, tokens/sec, etc.).
   - Vary batch size (XDNA2 may have different sweet spots than CPU/GPU).

3. **Power & Thermal**
   - Log package power, NPU power (if exposed), temperatures during runs.
   - Compare "performance" vs "sustained" modes.
   - Measure throttling behavior under long workloads.

4. **Accuracy vs Simulation**
   - Bit-exact or near-exact comparison between real hardware output and our high-fidelity simulation (important for validation).
   - Track numerical differences introduced by bf16 vs fp32, different quantization paths, etc.

5. **End-to-End vs Baselines**
   - Compare against:
     - Our own SimulationBackend (with matching config).
     - CPU (reference implementation).
     - GPU (if available on the same machine).
     - Previous Python `dragon_npu` reference (if numbers exist).
   - Use the existing `xdna2_probe` example as a starting harness, then move to proper criterion benchmarks.

6. **Methodology Requirements**
   - Lock CPU frequency / governor where possible.
   - Control for background load.
   - Report full system configuration (CPU model, RAM, kernel, driver version, NPU firmware if queryable).
   - Run multiple trials and report statistical summary.
   - Separate "compilation time" from "steady-state inference time".

### Recommended Tooling
- `criterion` for micro/macro benchmarks (already a planned item in the original roadmap).
- Custom harness using the public `DragonNpu` / `AmdXdna2Backend` API (so the same code works in simulation and real mode).
- Optional integration with `perf`, `powerstat`, or AMD-specific tooling when running on real hardware.

## Current Status & Next Steps (Production Path)

**What is actually solid today (as of this session):**
- Real device detection + multi-index open on `/dev/accel/accelN` (pure std, no deps).
- Rich observability (`real_device_*`, sysfs reads, power control).
- Typed direct driver queries via `DRM_IOCTL_AMDXDNA_GET_INFO` (firmware version, power mode, resource info with real TOPS numbers, sensors). These work on physical hardware.
- Real ONNX model loading with actual multi-dtype weights (including bf16 raw data).
- Solid compiler with real optimization passes.
- High-fidelity simulation backend.

**What is not implemented yet (the real blocker):**
- Actual command submission: `create_hwctx`, BO management, `EXEC_CMD`, and feeding real `CompiledModel` weights + graphs to the NPU.
- The `submit_and_run` path on real hardware still returns a clear "not yet implemented" error.

---

## Realistic Recommendation for a Usable + Performant Lightweight Direct Implementation

Given the hard constraint of staying **pure direct** (no XRT, no heavy official runtimes), here is the pragmatic path:

### Phase 0: Done — Diagnostics & Observability (Current Strength)
You already have one of the best open, lightweight observability layers for amdxdna on Linux. This is genuinely useful for bring-up and debugging.

### Phase 1: Foundation for Submission (High Value, Relatively Low Risk)
**Goal**: Make the direct path able to create real hardware contexts and manage buffers.

**Recommended immediate work (safe to do autonomously):**
1. **Implement full BO (Buffer Object) management** using the already-ported structs:
   - `create_buffer` → real `DRM_IOCTL_AMDXDNA_CREATE_BO`
   - `get_bo_info`, `sync_buffer`
   - Proper handling of different BO types (`Shmem`, `DevHeap`, `Cmd`, etc.)

2. **Implement real `create_hwctx` / `destroy_hwctx`**:
   - The struct `amdxdna_drm_create_hwctx` is fully known.
   - This requires creating special BOs first:
     - UMQ BO (User Mode Queue) — critical for low-latency submission
     - Log buffer BO
   - Then calling `DRM_IOCTL_AMDXDNA_CREATE_HWCTX`
   - You get back `handle`, `umq_doorbell`, `syncobj_handle`

3. **Improve error handling and observability** around these operations.

This phase alone would be a major milestone. Being able to create/destroy real hardware contexts from Rust via pure ioctls would already be more open and direct than most existing solutions.

### Phase 2: The Hard Problem — Command Submission Format (The Real Unknown)
This is where things get difficult for a pure lightweight implementation.

From the uAPI:
- `DRM_IOCTL_AMDXDNA_EXEC_CMD` takes an array of `cmd_handles` (BOs) + `args`.
- The actual command content lives inside command BOs (`AMDXDNA_BO_CMD` type).
- The format of those command buffers (the "control code" / "txn ops") is **not publicly documented** in the headers.

**Realistic options** (ranked by how well they match your "lightweight direct" goal):

**Best alignment with your philosophy:**
- Treat command format discovery as a research project.
- Start by getting the infrastructure right (hwctx + BOs + doorbell setup + basic EXEC_CMD scaffolding).
- Use the working `get_*` queries + new hwctx/BO code to do bring-up experiments (e.g. create context, allocate buffers, try simple commands, observe what the firmware does via logs/sensors).
- Look for any open-source examples, older XDNA1 traces, or kernel driver code that shows command construction.
- Goal for "usable": Get a trivial workload (vector add, simple matmul, or even just "run this pre-existing .xclbin-like blob") executing end-to-end.

**More pragmatic but slightly against pure-direct preference:**
- Accept using AMD's compiler tools *offline only* to produce command buffers / artifacts, then load and submit those artifacts purely via the direct path at runtime. This keeps the hot path (execution) lightweight and open.

**Avoid (per your direction):**
- Depending on XRT or IRON at runtime for submission.

### Phase 3: Performance & Polish
- Proper tiling / multi-column hwctx usage
- Efficient weight loading from `CompiledModel::initializers()` into device BOs
- bf16 native paths
- Power/thermal management via the direct queries we already have
- Proper async / doorbell driven submission for low latency

---

## My Concrete Recommendation for Next Steps (Prioritized)

Given where the code is today, here is what I would do in order:

1. **Implement real BO creation + sync + info** (biggest immediate win for the direct layer).
2. **Implement real `create_hwctx`** (including the required UMQ + log BO setup). This is very achievable with the current scaffolding.
3. **Heavily improve the `direct.rs` module** with better documentation of the actual uAPI requirements (UMQ, doorbells, syncobjs, command BO format unknowns).
4. **Add a "direct bring-up" example** that exercises hwctx creation/destruction + simple buffer allocation on real hardware (similar to how the current probe + integration test work for diagnostics).
5. **Start a dedicated reverse-engineering / discovery effort** for the command payload format (this is the part that will take the most time and may need some creativity or luck).
6. **Only after the above** start wiring `submit_and_run` to actually use real hwctx + command submission.

This path keeps you true to the lightweight direct vision while making steady, measurable progress toward something that is actually *usable*.

Would you like me to start executing any of the above (for example, begin implementing real BO management + create_hwctx with proper ioctl calls right now)? Or would you prefer a different emphasis?

The `xdna2_probe` example in `examples/` is the recommended minimal harness for early real-hardware bring-up.

The `direct` submodule (`src/backends/amd_xdna2/direct.rs`) is the home for our lightweight, open, direct-to-amdxdna implementation. It contains the `DirectSubmission` trait, uAPI constants, and structs. This is where the real command submission logic will live when we implement the ioctl layer — staying true to the goal of a lighter, focused, non-heavy-runtime path.

**Cargo integration test + probe example for real-device layer:**
- `cargo test --features amd-xdna2 --test xdna2_real_hardware -- --nocapture`
- `cargo run --features amd-xdna2 --example xdna2_probe`

These are the two recommended ways for hardware owners to validate the full real (non-simulation) device open + diagnostics layer on physical XDNA2 systems.

They exercise `RealXdna2Device`, `RealXdna2DeviceInfo`, multi-index open, and the complete set of public diagnostics. On non-target platforms they skip or show simulated state cleanly.

On real hardware you will see concrete values such as `real_device_open=true`, `real_device_path=/dev/accel/accel0`, and rich `real_device_info` containing the PCI 1022:17f0 details and detected variant.

---

This document will be updated as the real execution implementation progresses. All public APIs (`submit_and_run`, `real_execution_active`, etc.) are intended to remain stable.