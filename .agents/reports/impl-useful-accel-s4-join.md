# S4 join: useful-npu-accel dual track (S0–S5)

**Date:** 2026-08-12
**Slice:** useful-npu-accel **S4** join (docs/residual only)
**Mode:** join + honesty pin. No product Rust. No live NPU. No unlock mint.
**Honesty:** invent_pass=0 · not_c_ready=1 · not_ran_on_npu=1 · Pass=**No**
**Unlock:** **none** (first `C_UNLOCK=` = **`2c9c12e8` CONSUMED**)

This join is **not** a mint. Diagnostic ≠ unlock. IRON `PASS!` ≠ product
`ran_on_npu`. Host linear demo ≠ NPU acceleration.

---

## Decision

| Field | Value |
|-------|--------|
| Dual track | **JOINED** (useful-soon shipped; pure-direct residual unchanged) |
| invent_pass | **0** |
| not_c_ready | **1** |
| not_ran_on_npu | **1** |
| Pass | **No** |
| ran_on_npu | **false** / not claimed |
| residual class | still **`wait_cmd_einval`** |
| live_justified (S3) | **0** |
| unlock | **none** (`2c9c12e8` CONSUMED; not reopened, not minted) |
| empty same-shot reconfirm | **frozen** |
| sticky ALLOW | **no** |

---

## What shipped (cite, do not re-do)

| Step | Report | Outcome |
|------|--------|---------|
| **S0** pin | `.agents/reports/impl-useful-accel-s0-pin.md` | Dual track named. Plan mirrored. Unlock none. S5 expanded to agent-try. |
| **S1** useful-soon | `.agents/reports/impl-useful-accel-s1-runner.md` | Opt-in `ExternalRunnerBackend` + CLI `--external-runner`. Child `PASS!` sets informational `child_said_pass` only. Never `ran_on_npu`. Missing binary is unavailable, not a silent host Pass. Not default `DragonNpu::new`. |
| **S2** useful-soon | `.agents/reports/impl-useful-accel-s2-host-demo.md` | Tiny host MatMul+Relu+Add fixture. Input `[-1, 2]` → golden `[0.5, 3.5]`. Host only. `ran_on_npu` stays false. |
| **S3** residual | `.agents/reports/impl-useful-accel-s3-wait-diag.md` + `wait-fence-diagnostic.md` | Ranked WAIT fence story. Primary: `OutFenceLookupFail`. `live_justified=0`. Diagnostic is **not** an unlock reason. Product wait path still WAIT_CMD. |
| **S5** agent-try | `.agents/reports/useful-accel-s5-try-npu.md` | NPU ran this week via IRON eltwise add / relu / softmax (`PASS!`, exit 0). MobileNet not prebuilt (no xclbin; run needs torch). eepy VitisAI listed; offload **not** proven (CPU fallback). Still **not** product Pass. |
| **S4** join | this file | Honesty pin. Who acts next. Stop empty reconfirm. |

Plan: `.agents/plans/plan-useful-npu-accel.md`

---

## Two tracks stay distinct

1. **Useful-soon (shipped).** Operator can run `dragon-npu` with
   `--external-runner` against mlir-aie examples **without** a product
   unlock. Host CLI `compile` / `run` on the tiny linear fixture stays
   host/sim. IRON child `PASS!` is the external stack result, not
   `decide_ran_on_npu`.
2. **Pure-direct residual (unchanged).** Same-shot already proven under
   **`C_UNLOCK=2c9c12e8` CONSUMED**. Residual class is still
   **`wait_cmd_einval`**. S3 names the wait-fence hypotheses and locks
   `live_justified=0`. Next product live only under a **new** unlock
   **and** a **new** named reason.

---

## Unlock SSoT

First parse line in `RESIDUAL.md` (after the `<!-- first C_UNLOCK=`
comment) is still:

```
C_UNLOCK=2c9c12e8 CONSUMED
```

- **Do not reopen** `2c9c12e8`.
- **Do not mint** from this join.
- Diagnostic, wrapper, host demo, and IRON agent-try are **not** a new
  named live reason.

---

## Who acts next

| Actor | Action |
|-------|--------|
| **Human** | Residual-matched / load-accepted classic **1688** (or a completing-stack peel that actually loads and finishes) into `testdata/xdna2_cmd/external_known_good/` with honest PROVENANCE. Public 300 / 660 / 2592 / 4512 and unpatched oracle do **not** count. |
| **Operator (optional, no unlock)** | `dragon-npu run … --external-runner` against mlir-aie examples. Still not product `ran_on_npu`. |
| **Agent** | Only on a **new named offline delta**. Freeze empty same-shot reconfirm. Do **not** mint. |

Primary wall for product Pass is unchanged: residual-matched /
load-accepted material plus a completing wait (`decide_ran_on_npu`:
real ioctl + ERT Completed + OFM allclose).

---

## Explicit non-actions this join

- No `src/` edit
- No unlock mint / no reopen CONSUMED
- No live apply / no `XDNA2_ALLOW_EXEC_CMD=1` / no L3 open
- No Pass claim / no product `ran_on_npu`
- No empty same-shot reconfirm
- No retag of IRON peels as load-accepted

## Combined reviewer summary

`/tmp/grok-1000/grok-impl-summary-useful-accel.md`
