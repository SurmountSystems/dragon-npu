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

**Optional feature `xrt-bridge` (Phase 9):** research/capture comparison stub only.
Never default, never on the hot path, never links `libxrt`, never enables EXEC_CMD.
Production remains direct amdxdna. See [`SECONDARY_BACKENDS.md`](./SECONDARY_BACKENDS.md).

**Current state of the direct path**:
- Excellent device detection + multi-index open (pure std, no extra crates).
- Rich read-only observability (`real_device_*`, sysfs access).
- Typed high-level driver queries via `DRM_IOCTL_AMDXDNA_GET_INFO` (firmware version, power mode, resource info / TOPS, sensors, AIE version).
- Full `DirectSubmission` trait + ported uAPI structs; real CREATE_BO / SYNC_BO / GET_BO_INFO / GEM_CLOSE.
- CREATE_HWCTX: DevHeap 64 MiB + mmap, **valid `qos_p`** (driver always copy_from_user), tile multiples of 4; see `docs/XDNA2_DIRECT_CAPABILITIES.md`.
- CREATE_HWCTX + destroy: **working** on Real arm (hardened OOT host, 2026-07-15).
- EXEC_CMD plumbing + Phase 1 **typed offline** Cmd BO builders
  (`pack`/`unpack`/`parse_amdxdna_cmd_header_from_bytes`,
  `build_ert_start_npu_cmd_bo`, chain, preempt, DPU envelope); see
  **`docs/XDNA2_CMD_PAYLOAD.md`** (Phase 1.4 criteria met for envelope research).
- Firmware **txn / instruction** encoding still **opaque**.
- Phase 2 **infrastructure:** `DirectSubmission::wait_command` issues
  `DRM_IOCTL_AMDXDNA_WAIT_CMD`; `submit_and_wait_command` is L1-gated with
  finite timeout. Lab procedure: **`docs/XDNA2_EXEC_CMD_LAB.md`**. Silicon
  single-shot is **operator-run** — not claimed proven by default tests.
- `timed_matmul` + `examples/npu_direct_bench` JSON surface for colibrì tools
  (honest skip statuses; `ok_submit_wait_envelope` / `ok_submit_wait_with_inst_blob`
  ≠ `ran_on_npu`). Stage timings + optional `host_ref_*` CPU baseline.
- Phase 3 **safe silicon timing:** `timed_bo_roundtrip` / `--bo-roundtrip`
  (`ran_on_device_path`; no EXEC_CMD). Instruction blob env hook
  (`XDNA2_INSTRUCTION_BLOB` / `_HEX`) for START_NPU device addr — still not
  product GEMM Pass without numerical verify.

Actual GEMM / model run on silicon still depends on a **known-good instruction
buffer + EXEC_CMD under multi-layer gates + numerical verify**. Phase 1–3
plumbing is not product compute Pass.

### Safety / NEVER AGAIN (EXEC_CMD)

Scaffold EXEC_CMD previously NULL-deref'd `amdxdna` (`aie2_cmdlist_fill_slot`) and wedged
`open()` (2026-07-13). **Default and `amd-xdna2`-only builds permanently cannot issue
`DRM_IOCTL_AMDXDNA_EXEC_CMD`.**

| Layer | Requirement |
|-------|-------------|
| Compile | Cargo feature `xdna2-dangerous-exec-cmd` (default **OFF**, not implied by `amd-xdna2`) |
| Runtime | Env `XDNA2_ALLOW_EXEC_CMD=1` (or `true`) |
| API | `exec_cmd_is_enabled()` / `exec_cmd_block_reason()`; `submit_command` + `submit_and_wait_command` hard-fail when gated; `wait_command` not L1-gated |
| Kernel (if L3 lab module installed) | `modprobe amdxdna exec_cmd_allow=1` — default 0 refuses all EXEC_CMD subtypes with `-EPERM` |

Legacy `XDNA2_TRY_SUBMIT` is **ignored**. Never claim product Pass from a research scaffold.
Full policy: **`docs/XDNA2_NEVER_AGAIN.md`**. Lab single-shot: **`docs/XDNA2_EXEC_CMD_LAB.md`**.  
Capability status: **`docs/XDNA2_DIRECT_CAPABILITIES.md`**.  
Driver L2+L3: **`kernel/patches/README.md`** / hardened install playbook.

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
- **Phase 4:** Real arm `compile_model` / `submit_and_run` wired (see below).
- **Phase 5:** Host graph interpreter + weight BO plan (see below); still host-only for model Ok.

**What is not claimed (the remaining blocker for silicon inference Pass):**
- Known-good instruction **txn** + numerical verify → `ran_on_npu=true` product Pass.
- Full graph execution on NPU (weights → BOs → verified output matching host).

### Phase 3.5 lab harness (wire done — not product Pass without known-good txn)

- Consolidated txn research: [`docs/XDNA2_TXN_RESEARCH.md`](./XDNA2_TXN_RESEARCH.md)
- Read-only inventory: `bash scripts/inventory_txn_sources.sh` (**always exit 0 — smoke report, not a Pass gate**)
- Operator capture checklist: `scripts/capture_instruction_blob_checklist.md`
- Pure policy: `src/backends/amd_xdna2/numerical.rs` (`decide_ran_on_npu` — requires non-empty buffers + submit + blob + real ioctl + allclose)
- Lab OFM: `lab_io.rs` + `timed_matmul_direct` when `XDNA2_LAB_OFM=1` (fail-closed setup; prop_count=0 kernel args; OFM host-zero; decide → `ok_npu_verified`)
- Operator wrapper: `scripts/lab_exec_once.sh` (default dry-run; apply sets LAB_OFM=1)
- Residual: [`RESIDUAL.md`](../RESIDUAL.md)

Product Pass still blocked on operator **known-good** blob + Real OFM match. Default path remains hard-false. See Phase 3.5 table in [`XDNA2_DIRECT_CAPABILITIES.md`](./XDNA2_DIRECT_CAPABILITIES.md).

---

## Phase 4 — Wire Real `compile_model` / `submit_and_run`

### 4.1 `compile_model` (Real arm)
- Always uses host-side `ModelCompiler` (same passes as simulation).
- Does **not** require `/dev/accel` open.
- Stores graph `ops` on `CompiledModel` for allowlist checks.
- `AmdXdna2Backend::xdna2_compile_plan(&model)` reports ops, unsupported list, hwctx size hints.

### 4.2 `submit_and_run` (Real arm) — `XDNA2_REAL_SUBMIT_MODE`

| Mode | Env value | Behavior |
|------|-----------|----------|
| **Fail-closed (default)** | unset / other | `DragonNpuError::RealSubmitNotReady` with device open, EXEC gates, op plan, and mode hints. **No** NPU Pass. |
| **Host graph** | `host_ref` or `host_validate` | Phase 5 host f32: **Identity/Relu/Add/Mul/MatMul/Gemm** with real math when weights/shapes present. Not NPU. Missing weights → error (no fake golden). |
| **Gated EXEC probe** | `npu_exec` | **EXEC-only by design** (requires L1 double gate). Coarse op allowlist, then Phase 3 `timed_matmul_direct` **1×1×1 probe** — **model graph is NOT executed**. Always `Err(RealSubmitNotReady)` with probe status. For CREATE_HWCTX-only without opening EXEC, use `timed_matmul` / `xdna2_probe` (`hwctx_ok`). |

Lab/tests: `AmdXdna2Backend::submit_and_run_with_mode(..., RealSubmitMode)` avoids env races.

### 4.3 Op allowlist (Phase 4 → Phase 5 host graph)
- **host_ref Ok path:** `Identity`, `Relu`, `Add`, `Mul`, `MatMul`, `Gemm` (host interpreter; needs weights for non-elementwise unary ops).
- **npu_exec graph pre-check:** same list (probe still ignores the graph).
- Unsupported → `DragonNpuError::UnsupportedOps`. Heuristic 100×`conv` models are not allowlisted.

### 4.4 Session / hwctx hygiene
- **One** `RealDirectSubmission` per open Real device fd (DevHeap fd-scoped; `Drop` munmap + GEM_CLOSE).
- Ephemeral hwctx for probes: `destroy_hwctx_and_owned_bos` closes UMQ/log after `destroy_hwctx`.
- `RealXdna2Device::Drop` best-effort destroys tracked hwctxs + BOs.
- Simulation arm unchanged.

### 4.5 Honesty (CRITICAL)
Without a known-good instruction txn + numerical verify, **do not** claim full NPU inference Pass.  
`host_ref` Ok ≠ silicon. `npu_exec` envelope status ≠ `ran_on_npu`. See Phase 3.5 /
[`XDNA2_DIRECT_CAPABILITIES.md`](./XDNA2_DIRECT_CAPABILITIES.md) and
[`XDNA2_TXN_RESEARCH.md`](./XDNA2_TXN_RESEARCH.md).

---

## Phase 5 — Model pipeline (weights → host graph execution)

**Goal:** Real host-side graph execution using `CompiledModel` ops + initializers, plus weight → BO **planning** for future NPU upload. NPU txn/instruction remains opaque — **no silicon graph Pass**.

### 5.1 Host graph interpreter (`src/backends/amd_xdna2/host_graph.rs`)
- `evaluate_host_graph(model, input)` runs allowlisted ops on host f32:
  - **Identity**, **Relu**, **Add**, **Mul**, **MatMul**, **Gemm**
- Wired into Real `submit_and_run_with_mode(HostRef)` / `XDNA2_REAL_SUBMIT_MODE=host_ref`
- Status: still **not** `ran_on_npu`

### 5.2 Shape / IR assumptions (pragmatic)
- Parallel-Vec IR: node `inputs` names matching initializers preferred; else sequential unused initializers for **required** tensors only (linear-chain JSON with flat `ops`).
- JSON `nodes` may carry `inputs: ["x","W","b"]` for named binding (DefaultModelLoader + OnnxModelLoader).
- MatMul: `A[M,K] @ B[K,N]` row-major; weight rank-2 `[K,N]`; library uses explicit `HostGraphOptions.matmul_k` (CLI may set via `XDNA2_HOST_MATMUL_K` / `from_env`; default eval is env-isolated).
- **Gemm bias: named third input only** — never greedily steals the next sequential initializer (would steal next layer weights).
- Missing weights / bad shapes → clear `RuntimeError` (no fake MatMul golden).
- Unsupported ops (e.g. Conv) → `UnsupportedOps`.
- JSON load refuses `initializers` when shape product ≠ data length.

### 5.3 Weight BO plan
- `Xdna2WeightPlan { name, dtype, shape, nbytes, suggested_bo_type }` via `build_weight_bo_plan` (`nbytes` = host payload bytes).
- Optional `upload_weights_to_shmem_bos` (create Shmem → fill → **destroy all** on success or error) — returns upload **count** (handles not live); **not** inference.
- Checked-in fixtures under `tests/fixtures/` are loaded by unit tests.

### 5.3b Weight EXEC bind (lab plumbing + optional timed wire)
- `select_lab_matmul_weight` → first MatMul/Gemm weight + `(K,N)` + f32 LE WTS (Gemm `transB` honored).
- `WeightExecBind` / `create_and_fill_weight_io_bos` keeps **live** IFM/WTS/OFM (unlike throwaway upload). **Not RAII** — caller must destroy. Full alloc (page) zeroed on OFM + IFM/WTS tails.
- `lab_decide_from_ofm_bytes_with_golden` for model-WTS golden (golden length must be `m*n`).
- **Timed wire (fail-closed):** `XDNA2_LAB_OFM=1` + `XDNA2_LAB_WEIGHT=1` + `XDNA2_LAB_WEIGHT_MODEL=/path/model.json`  
  ones IFM + model WTS; model `(K,N)` must match timed `k,n`. Weight without OFM → refuse.
- **MVP limits (intentional):** f32 LE WTS; ones IFM golden only; plain MatMul/Gemm only — refuse `fused_*`, Gemm `alpha`/`beta` ≠ 1, named bias (`check_lab_weight_mvp_model` / `LAB_WEIGHT_MVP_DOC`).
- Default LAB_OFM still ones IFM/WTS. **Not** silicon Pass without known-good txn + OFM match.

### 5.4 Honesty
- Host pipeline ready for golden/reference and local validate.
- NPU graph execution still needs known-good txn + bind weights into command path + numerical verify.

---

## Phase 6 — Efficiency and hardware quality (host + observability)

**Goal:** Better host reference performance, hardware **config/observability**, BO path hygiene, compiler fusions that prepare XDNA2 — **without** claiming silicon GEMM tops (txn still opaque).

### 6.1 Host dtype-aware path
- MatMul/Gemm use a **once-per-op** weight view: borrow `data_f32`, multiply `data_i8` without a full temp clone, else convert once via `as_f32_data` (bf16/fp16).
- Consecutive **MatMul/Gemm + Relu** fuse on the host (Relu on write for MatMul; Gemm applies bias then in-place Relu).
- Still host-only math — not NPU throughput.

### 6.2 Tile / resource heuristics
- `suggest_hwctx_tiles(resource)` / `suggest_hwctx_tiles_from_tops` → multiple of 4, clamp **4..=8** (higher TOPS ≥40 → 8).
- Backend: `AmdXdna2Backend::suggested_hwctx_tiles()` when Real + GET_INFO works.
- Defaults: `DEFAULT_HWCTX_TILES=4`, env override still `XDNA2_HWCTX_TILES` / `XDNA2_HWCTX_TILES_SINGLE` on timed paths.

### 6.3 Power mode
- GET remains `get_direct_power_mode` (GET_INFO param 9).
- SET implemented from public uAPI: `amdxdna_drm_set_power_mode` + `SET_STATE` (`0xc010_6448`) param `SET_POWER_MODE=0` → `set_direct_power_mode` / `DirectSubmission::set_power_mode`.
- Unit tests: struct sizes + range refuse. **Live SET is operator-only** — do not claim measured TOPS change without lab measure.

### 6.4 BO efficiency
- Roundtrip path still uses exclusive SYNC FROM + `read_bo_bytes_no_sync` (no double FROM).
- `fill_weight_bos_batch` fills many pre-created BOs on one submission/session (stub-tested).
- `upload_weights_to_shmem_bos` remains create→fill→destroy session.

### 6.5 Profiling
- `TimedOpResult` stages: `host_prep_s`, `fill_s`, `device_s` (sync/submit), `submit_wait_s`, `hwctx_s`, `total_s`, host_ref_*.
- `efficiency_notes()` compares stages; for `bo_roundtrip` labels **DMA-only** + approx MiB/s from SYNC FROM — **not** GEMM.
- JSON includes `efficiency_notes`.

### 6.6 Compiler XDNA2-oriented fusion
- New fuse names: `fused_matmul_relu`, `fused_gemm_relu`, `fused_matmul_add`, `fused_gemm_add`,
  `fused_matmul_add_relu`, `fused_gemm_add_relu`, `fused_conv_bias_relu` (plus existing conv pairs).
- Layout: level ≥2 still sets **NHWC** on all nodes.

### 6.7 Timeouts / L3 gate interaction
- Public `DEFAULT_WAIT_CMD_TIMEOUT_MS = 5000`; override `XDNA2_WAIT_TIMEOUT_MS`.
- L1 (`feature` + `XDNA2_ALLOW_EXEC_CMD=1`) + L3 (`exec_cmd_allow=1`) both required for EXEC_CMD on hardened host; WAIT uses finite timeout by default (`submit_and_wait` refuses 0 unless `XDNA2_WAIT_ALLOW_INFINITE=1`).
- See [`XDNA2_EXEC_CMD_LAB.md`](./XDNA2_EXEC_CMD_LAB.md).

### 6.8 Honesty (CRITICAL)
**Efficiency work ≠ silicon GEMM tops.** No `ran_on_npu` from host fusion, tile suggestions, power SET, or BO DMA MiB/s.

---

## Phase 7 — Compiler IR modernization (host-side)

**Goal:** Typed multi-op IR good enough for shape infer / fold / DCE / cost heuristics without a big-bang rewrite of Phase 4–6 host_ref paths.

### 7.1 Dual representation
- New module `src/ir.rs`: `DragonNpuIR`, `IrNode`, `NodeOp`, `NodeAttrs`, `TensorDesc`, `DType`, `TensorId` / `NodeId`.
- `ModelGraph` parallel Vecs **remain** (production fuse / `CompiledModel` packing). Fields are `pub(crate)` for conversion.
- `DragonNpuIR::from_model_graph` / `to_model_graph` roundtrip ops, edges, initializers, and attrs.

### 7.2 ONNX / JSON attributes
- ONNX protobuf path extracts Gemm/Conv-class attributes (`alpha`, `beta`, `transA`/`transB`, pads, strides, …) into `ModelGraph.node_attrs`.
- JSON loaders accept optional per-node `"attrs": { ... }`.
- Case-insensitive op parse for known ops; fused Phase 6 names preserved as `NodeOp::Fused`.

### 7.3 Shape inference + fold + DCE
- `infer_shapes` for MatMul/Gemm/Add/Mul/Relu/Identity/fused dense (+ honest Conv unknowns as `-1`).
- `constant_fold` for small Identity/Relu/Add/Mul of constant tensors (≤4096 elems).
- `dead_code_elim` from true graph sinks (unconsumed outputs); unconnected heuristic graphs keep all sinks.
- `ModelCompiler::compile` runs IR `optimize_level2` at **optimization_level ≥ 2**, then existing fuse/layout/quantize/memory passes.

### 7.4 Cost model (honest heuristic)
- `estimate_xdna2_cost(ir, tiles, tops_max) → CostEstimate` with `host_ops`, `npu_candidate_ops`, `rough_macs`, notes.
- Marks MatMul/Gemm/Conv (+ fused dense) as NPU candidates; static weight bytes vs typical DevHeap budget in notes only.
- **Not** measured silicon time; **not** `ran_on_npu`; no EXEC_CMD / `/dev/accel` in Phase 7 tests.

### 7.5 IR-native host_ref
- `evaluate_host_ir` / `evaluate_host_ir_with_options`: `DragonNpuIR::to_model_graph` → shell `CompiledModel` → existing `evaluate_host_graph_with_options`.
- Reuses allowlist/goldens; refuses Conv etc. **Still host-only** — never sets `ran_on_npu`.

### 7.6 Honesty (CRITICAL)
IR modernization is **compiler/host only**. No silicon Pass, no EXEC_CMD by default, no fake GEMM tops.

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