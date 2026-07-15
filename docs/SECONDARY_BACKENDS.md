# Secondary backends (Intel VPU, Qualcomm Hexagon, XRT bridge)

**Policy:** AMD XDNA2 (`amd-xdna2`) is the primary hardware target. Secondary
backends ship as **honest stubs** until the XDNA2 MVP (known-good txn +
numerical verify → `ran_on_npu`) is green. They must never fake hardware Pass.

| Feature | Module | Auto `DragonNpu::new`? | Real inference? |
|---------|--------|------------------------|-----------------|
| *(default)* `simulation` | `SimulationBackend` | Yes (no `amd-xdna2`) | Host sim only |
| `amd-xdna2` | `AmdXdna2Backend` | Yes when feature on | Direct path; `ran_on_npu` still gated |
| `intel-vpu` | `IntelVpuBackend` | **No** — opt-in type | **No** (stub refuses run) |
| `qualcomm-hexagon` | `QualcommHexagonBackend` | **No** — opt-in type | **No** (stub refuses run) |
| `xrt-bridge` | `XrtBridge` | **No** | **No** (research refuse) |

## Honesty contract

1. `NpuStatus.available == false` for stubs (not claiming a working device).
2. `NpuStatus.simulated == true`.
3. `device_name` / `driver_version` explicitly say stub / not-implemented.
4. `run` / `submit_and_run` return `DragonNpuError::BackendError` — **no** silent
   simulation success that looks like Intel/Qualcomm/XRT silicon.
5. `compile_model` on Intel/Hexagon may use host `ModelCompiler` for graph
   analysis only; the `CompiledModel` still refuses `run`.
6. Never claim `ran_on_npu`. Never open EXEC_CMD from these modules.
7. Do not enable `xdna2-dangerous-exec-cmd` or `XDNA2_ALLOW_EXEC_CMD` “for”
   secondary backends.

## Linux detection notes (future — not wired as success)

### Intel VPU

- Kernel: `intel_vpu` / `ivpu` (Meteor Lake+ NPU).
- Userspace historically: Level Zero, OpenVINO NPU plugin.
- Sysfs: may appear under `/sys/class/accel` or accel DRM depending on stack.
- Dragon-npu today: feature compiles the stub only; no device open, no Level Zero.

### Qualcomm Hexagon

- Weakest **Linux laptop** story (Snapdragon X / mobile often vendor QNN).
- Possible signals: `remoteproc`, vendor firmware nodes — not standardized like
  amdxdna accel.
- Dragon-npu today: honest refuse stub only.

### XRT bridge (`xrt-bridge`)

- **Research only:** offline comparison with XRT capture / ERT packaging notes.
- **Not** the production path (production = pure direct amdxdna under `amd-xdna2`).
- Feature is never in `default` and is **not** enabled by `amd-xdna2`.
- No `libxrt` link. All APIs refuse real XRT sessions.
- Cross-links: [`XDNA2_REAL_EXECUTION.md`](./XDNA2_REAL_EXECUTION.md),
  [`XDNA2_NEVER_AGAIN.md`](./XDNA2_NEVER_AGAIN.md).

## Build matrix

```bash
export PATH=/usr/bin:$PATH
cargo test --lib
cargo test --features amd-xdna2 --lib
cargo test --features intel-vpu --lib
cargo test --features qualcomm-hexagon --lib
cargo test --features intel-vpu,qualcomm-hexagon --lib
cargo test --features xrt-bridge --lib
# Clippy -D warnings for the same feature sets (see docs/PACKAGING.md)
```

## When to implement for real

Only after XDNA2 silicon MVP criteria (known-good instruction txn + numerical
verify). Until then, prefer contributing to XDNA2 direct path rather than
half-implemented vendor stacks.

See also: [`PACKAGING.md`](./PACKAGING.md), [`PLAN.md`](../PLAN.md) Phase 9.
