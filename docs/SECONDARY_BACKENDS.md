# Secondary backends (Intel VPU, Qualcomm Hexagon, XRT bridge, external runner)

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
| *(always compiled)* | `ExternalRunnerBackend` | **No** — opt-in wrap | Child process only. `execution=external_runner`. **`ran_on_npu=false` always.** |

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

### External runner (IRON first; honest wrap)

`ExternalRunnerBackend` shells to a stack that already runs (mlir-aie / IRON
`python` / `make` under `programming_examples/`, later eepy). It is **opt-in**:
`DragonNpu::with_backend`, CLI `--external-runner PATH`, or env
`DRAGON_NPU_EXTERNAL_RUNNER`. Default `DragonNpu::new` is unchanged.

Honesty the code actually prints:

- `execution=external_runner`
- `ran_on_npu=false` even when the child stdout contains `PASS!` or `PASS`
- `invent_pass=0` (hard-coded)
- `product_known_good=false` (IRON peels are not product known-good)
- Missing binary is `available=false` and an explicit run error, **not** a
  silent host/simulation Pass
- Spawn is `Command::new` plus an argv array. Never `sh -c`. The child does
  not receive `XDNA2_ALLOW_EXEC_CMD`.

Usual operator commands (find mlir-aie even when `$MLIR_AIE` is unset):

```text
just npu-relu          # named IRON ReLU
just npu-eltwise       # named IRON eltwise add
just npu-softmax       # named IRON softmax
just npu-gelu          # named IRON GELU (same unary script as relu)
just npu-silu          # named IRON SiLU
just npu-mul           # named IRON eltwise mul
just npu-swiglu        # named IRON SwiGLU
just npu-scale-shift   # named IRON scale-and-shift
just npu-iron          # named IRON vector add run+verify wrap (no --aot-dir)
just npu-iron-export   # export-only collect (writes artifacts; no child PASS! expected)
dragon-npu run --iron relu
dragon-npu run --host-graph    # host linear; needs --features amd-xdna2
```

Catalog names: `vector_add`, `eltwise`, `relu`, `softmax`, `gelu`, `silu`,
`eltwise_mul`, `swiglu`, `scale_shift`. Unknown names fail closed. The
recipes set `PYTHONPATH` so `aie` and `pyxrt` import, then run
`dragon-npu run --iron NAME`. See `justfile` and `tools/npu_iron_env.sh`.
`just install` puts this tree on `PATH`. `just npu-iron-paths` (optional
`kernel=relu`) prints what it would use.

`just npu-iron` is the run+verify wrap. Sibling `vector_scalar_add.py --aot-dir`
writes artifacts and returns without `PASS!`. That export path is opt-in:
`just npu-iron-export` or `dragon-npu run --iron vector_add --iron-export`.

Two identities stay distinct. Wrap `job=iron_npu2_vector_add` (optional
export collect for vector add) is **not** classic 1688 load-accepted.
A public 300-byte peel may be the right *kind* of bytes for that IRON job.
It is never relabeled as mladf classic 1688. Child `PASS!` stays
`execution=external_runner` and `ran_on_npu=false`. Product Pass still
needs live `decide_ran_on_npu`.

`--host-graph` is host `evaluate_host_graph` on
`tests/fixtures/host_matmul_relu_add.json` (default input `[-1, 2]`).
JSON says `execution=host`. Not NPU. Rebuild the binary with
`--features amd-xdna2` for that flag.

Raw `--external-runner` remains for one-off argv. Prefer `--iron` / `just`.

Do **not** expand `$MLIR_AIE` when it is empty: the cwd becomes
`/programming_examples/...` and the wrap fails.

Long form (only after `MLIR_AIE` is a real directory):

```text
export MLIR_AIE=/path/to/mlir-aie
dragon-npu run tests/fixtures/host_matmul_relu.json --iron vector_add
```

Optional cwd env: `DRAGON_NPU_EXTERNAL_CWD`. Program path must be **absolute**
(no PATH search). This wrap is useful compute on a working stack. It is **not**
product Pass and **not** a reason to mint an unlock.

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
