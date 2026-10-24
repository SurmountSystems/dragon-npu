# Join: useful NPU commands catalog

**Date:** 2026-08-12
**Slice:** useful-npu-commands (plan steps 1–6)
**Honesty:** invent_pass=0 · not_c_ready=1 · not_ran_on_npu=1 · Pass=**No**
**Unlock:** **none** (first `C_UNLOCK=` = **`2c9c12e8` CONSUMED**)

This join is **not** a mint. Named `just npu-relu` / `--iron` is **not**
product Pass. IRON child `PASS!` is **not** product `ran_on_npu`. Host
`--host-graph` is host math only.

---

## Decision

| Field | Value |
|-------|--------|
| Catalog | **landed** |
| invent_pass | **0** |
| not_c_ready | **1** |
| not_ran_on_npu | **1** |
| Pass | **No** |
| ran_on_npu | **false** (JSON and human tokens hard-coded) |
| residual class | still **`wait_cmd_einval`** |
| unlock | **none** (`2c9c12e8` CONSUMED; not reopened, not minted) |
| sticky ALLOW | **no** |

---

## What shipped

1. **Named IRON catalog** (`src/backends/iron_kernel.rs`):
   `vector_add`, `eltwise`, `relu`, `softmax`. Each name maps to a
   sibling mlir-aie cwd suffix plus python argv. Unknown / empty name
   fails closed. No `$MLIR_AIE` required (same resolve as
   `tools/npu_iron_env.sh`). Spawn stays argv-only (`Command::new`,
   never `sh -c`).
2. **Recipes:** `just npu-eltwise`, `npu-relu`, `npu-softmax` next to
   `npu-iron` (vector add). Shared `_npu-iron-named`. Env script gained
   `npu_iron_example_for_kernel` and a kernel argument on
   `npu_iron_export_env`. One env script only.
3. **CLI:** `dragon-npu run --iron NAME` fills cwd + argv from the
   table. `--host-graph` calls `evaluate_host_graph` on
   `tests/fixtures/host_matmul_relu_add.json` (default input `[-1, 2]`
   → `[0.5, 3.5]`, `execution=host`). Needs `--features amd-xdna2`.
   Default `run` without those flags stays simulation tanh. Flags are
   mutually exclusive with each other and with raw `--external-runner`.
   Help lists the four names and `--host-graph`.
4. **Numbers when the child emits them:** `parse_child_f32_vector`
   prefers a valid little-endian `<f4` npy, else the first finite JSON
   array in stdout. Missing numbers → empty `Vec<f32>`, not an error,
   not Pass. Live IRON `PASS!` alone stays `output_len=0`.
5. **Docs:** `docs/SECONDARY_BACKENDS.md` and README lead with
   `just npu-relu` / `--iron`. Plan mirrored at
   `.agents/plans/plan-useful-npu-commands.md`.

Did **not** edit sibling `/home/hunter/Projects/ai/mlir-aie`. Did **not**
retag public AOT peels. Did **not** open `XDNA2_ALLOW_EXEC_CMD`.

---

## Agent verify (operator idle)

| Recipe / command | Exit | Honesty |
|------------------|------|---------|
| `just npu-relu` | **0** | `child_said_pass=true` `child_exit_code=Some(0)` `execution=external_runner` `ran_on_npu=false` `invent_pass=0` `output_len=0` stdout `PASS!` |
| `just npu-eltwise` | **0** | same honesty tokens |
| `just npu-softmax` | **0** | same honesty tokens |
| `cargo run --features amd-xdna2 --bin dragon-npu -- run --host-graph --json` | **0** | `execution=host` `output=[0.5,3.5]` `ran_on_npu=false` `invent_pass=0` |

Kernel env was present (mlir-aie + capture venv + pyxrt). Nothing skipped.

---

## Unlock SSoT

First parse line in `RESIDUAL.md` (after `<!-- first C_UNLOCK=`):

```
C_UNLOCK=2c9c12e8 CONSUMED
```

Do **not** reopen. Do **not** mint. Catalog land is **not** a new live
reason.

---

## Who acts next

| Actor | Action |
|-------|--------|
| **Human** | Residual-matched / load-accepted classic **1688** when held. That is still the Pass wall. |
| **Agent** | Idle unless a **new named offline delta**. Do not ask the operator to re-run `just npu-iron`. |
| **Operator (optional useful)** | May run `just npu-relu` / `--iron` without a product unlock. That is still not Pass. |

---

## Soft / JSON footer

`invent_pass=0 not_c_ready=1 not_ran_on_npu=1`

Catalog ≠ Pass ≠ unlock.
