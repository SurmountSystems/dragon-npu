# Plan 46 explore — useful inference without lab-classic START_NPU Pass

**Honesty:** invent_pass=0. Unlock **none**. Do **not mint**. This report is not Pass, not `ran_on_npu`, not live.

Operator goal is useful Ryzen NPU acceleration, not a lab stamp.

## Product inference today (host only vs NPU)

| Surface | What actually runs |
|---------|--------------------|
| `dragon-npu compile` / `run` | Host `ModelCompiler` + `CompiledModel::run` → `Backend::run`. JSON always `ran_on_npu=false`, `execution=host_or_simulation_run`. Never `submit_and_run`. |
| `DragonNpu::new` + `amd-xdna2` | Detects hardware for **status**. `Backend::run` stays the Simulation arm even when `/dev/accel` is open. |
| Host graph | `evaluate_host_graph` / `evaluate_host_ir`: Identity, Relu, Add, Mul, MatMul, Gemm, Constant, a few fused linear names. Needs weights. Conv refused. Used only if `XDNA2_REAL_SUBMIT_MODE=host_ref` (or `host_validate`). Default Real submit is fail-closed. |
| Models | ONNX (tract-onnx), `.onnx.json`, structured `.txt`. In-tree demos: `tests/fixtures/host_matmul_relu.json`, `host_add_mul.json`. Not an LLM. |
| Simulation `run` | Timed fake activations (tanh-style), not a real MatMul graph unless host_ref is selected. |
| Silicon today | Open, GET_INFO, CREATE_HWCTX, Shmem **BO roundtrip** (`ran_on_device_path`). GEMM / `ran_on_npu` **blocked**. Residual still `wait_cmd_einval`. |

## xrt-bridge, Colibrì, adapters

- **`xrt-bridge` is a research refuse stub.** No `libxrt`, no EP, not selected by `DragonNpu::new`. Compile/run/submit all error. Not a hook you can flip on.
- **`docs/COLIBRI_ADAPTER.md`** is the **timed JSON** contract for `npu_direct_bench`, not a model runner. Colibrì must not use `dragon-npu run` for silicon metrics.
- README **works vs blocked**: host graph works; NPU GEMM blocked; Intel/Hexagon honest stubs.
- **Backend trait** (`status` / `compile_model` / `run` / `submit_and_run`) is the only in-tree place a new runner can land. No product `Command::new` wrapper for IRON/eepy/ORT exists today.

## External runner (honest wrap)

Yes, product **can** wrap a working stack **without** claiming `ran_on_npu`:

- **eepy-min** (out of tree): ORT `VitisAIExecutionProvider`. Known vendor ONNX path. Needs L3 `exec_cmd_allow` for live EP; CPU session already works. See `docs/RYZEN_AI_EEPY_MIN.md`.
- **IRON / mlir-aie**: already completed four public npu2 peels on FW 1.1.2.64. That is **their** PASS, not product Pass.
- New backend should: subprocess or FFI to that stack; label `execution=external_runner`; `available` honest; **never** set `ran_on_npu`; never open L1 ALLOW from the wrapper.

## First useful kernel (pure-direct)

`npu_direct_bench --shape q_a` is **1×256×1024 f32** timed matmul (Colibrì). Lab classic identity is **mladf 1×1152×256 a16w4** START_NPU. Default `timed_matmul` is still envelope / 1×1×1 probe. **Useful silicon without Pass today:** `--bo-roundtrip` (DMA). **Useful compute soon:** wrap eepy/IRON, do not wait for classic 1688.

## Thin wrapper vs continue pure-direct

Pure-direct owns the lightweight stack and the stamp. It is still blocked on residual-matched material + WAIT. A thin honest wrapper ships acceleration **now** on a stack that already runs, and must not launder that into `ran_on_npu`. They are not substitutes.

## Recommended dual track

1. **Useful soon:** new opt-in `Backend` that shells to eepy Vitis EP (or IRON for tiny AOT). CLI `run` prints `execution=external_runner`, `ran_on_npu=false`. Host-graph tiny linear demo stays the CPU fallback.
2. **Pure-direct (unchanged law):** lab classic only under **new** unlock + named reason. No empty reconfirm. No Pass from wrap.

## HIGH-VALUE named offline deltas (in-tree)

1. Honest **external-runner backend** (Backend trait + CLI labels; TDD refuse `ran_on_npu`).
2. Pack a **host-graph LLM-tiny** (few MatMul+Relu+Add layers as ONNX/JSON + `dragon-npu run` demo).
3. **WAIT_CMD EINVAL driver walk / wait diagnostic patch** (userspace already logged syncobj-then-discard; W0 report exists; a named kernel diagnostic is still a delta, not a reconfirm).
4. Optional: subprocess contract tests that parse IRON/eepy stdout without ingesting peels as product known-good.

**Frozen (do not spend tokens):** empty capture reconfirm, reopen CONSUMED unlocks, re-run public AOT IRON peels, soft-classify mill, mint from this report.

**Who acts next:** implementer on (1) or (2) if the operator wants useful acceleration; human material / new unlock only for track 2 live.
