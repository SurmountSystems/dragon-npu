# Explore: next useful NPU inference (not Pass theater)

**Date:** 2026-08-12. invent_pass=0. IRON `PASS!` ≠ product `ran_on_npu`. Unlock none. Do not mint.

## What exists

| Surface | Does | Does not |
|---------|------|----------|
| `just npu-iron` + `tools/npu_iron_env.sh` | Resolve mlir-aie/capture venv; spawn `python vector_scalar_add.py --dev npu2` | Claim Pass; pass model tensors; dump AOT in the example dir |
| `ExternalRunnerBackend` | Opt-in argv wrap; `execution=external_runner`; `ran_on_npu=false` even on `PASS!`; missing bin errors | Default `DragonNpu::new`; return OFM (success is empty `Vec<f32>`); ingest peels as known-good |
| `dragon-npu run` **without** `--external-runner` | `CompiledModel::run` → Simulation **tanh**, `execution=host_or_simulation_run`, `ran_on_npu=false` | Host graph; NPU; `submit_and_run` |
| Host graph | Identity/Relu/Add/Mul/MatMul/Gemm. Fixture `host_matmul_relu.json`: `[-1,2]`→`[0,2]`. `host_matmul_relu_add.json`: →`[0.5, 3.5]` | Used by default CLI. Only `XDNA2_REAL_SUBMIT_MODE=host_ref` |

S5 already `PASS!` on this host (do not re-run): eltwise add, relu, softmax, vector_scalar_add. MobileNet/ResNet need torch + a long compile. eepy is CPU.

## After `just npu-iron`: example-dir artifacts?

**None collectable.** `/home/hunter/Projects/ai/mlir-aie/programming_examples/basic/vector_scalar_add/` is sources only (`.py`/Makefile/lits). Same for `ml/eltwise`, `eltwise_unary`, `softmax` (no `build/`, no `.xclbin`). JIT cache is not in those dirs. AOT already sits in sibling `mlir-aie-capture/aot_*` + `completed_run_*` and in-tree RESEARCH peels. Do not retag as load-accepted.

## ONE next slice

**IRON kernel catalog as a product command (still wrap, still `ran_on_npu=false`).**

Ship `just npu-eltwise` / `npu-relu` / `npu-softmax` plus `dragon-npu run --iron eltwise|relu|softmax` that reuse `npu_iron_env.sh` (cwd + argv table). Optional later: parse child stdout/npy so `run()` is not an empty tensor.

Touch: `justfile`, `tools/npu_iron_env.sh`, `src/cli.rs`, `src/bin/dragon_npu.rs`, `src/backends/external_runner.rs`, `docs/SECONDARY_BACKENDS.md`. Tests: named kernel argv + honesty tokens. Do not edit sibling mlir-aie.

**Operator does not:** re-run `just npu-iron`; mint unlock; drop 1688; install torch; compile MobileNet; set `MLIR_AIE`/`PYTHONPATH`.

Not this slice: host-graph-on-CLI (CPU only), eepy VAIP chase, Pass/live, retag peels.

```
invent_pass=0 product_pass=0 ran_on_npu=0 unlock=none
```
