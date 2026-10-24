# S0: useful-npu-accel plan pin + workspace mirror

**Date:** 2026-08-12
**Slice:** S0 only (docs/residual pin)
**Honesty:** invent_pass=0 · not_c_ready=1 · not_ran_on_npu=1 · Pass=**No** ·
unlock **none**
**Live:** not run · L3 not opened · ALLOW not set · unlock **not** minted

## Goal

Pin residual + AGENTS after plan **useful-npu-accel** approval. Name the dual
track. Keep product Pass law unchanged. Record that the operator expanded S5
to **agent-try**.

## Dual track named

1. **Useful-soon:** opt-in external-runner wrap of stacks that already run
   (IRON first). Never launder product `ran_on_npu`.
2. **Pure-direct residual unchanged:** still `wait_cmd_einval`; same-shot
   proven under **`C_UNLOCK=2c9c12e8` CONSUMED**; next product live only
   under a **new** unlock and a **new** named reason.

This plan is **not** a mint.

## Done

1. **Plan mirror:** approved session plan written to
   `.agents/plans/plan-useful-npu-accel.md` with a short note that the
   operator expanded S5 to agent-try.
2. **RESIDUAL.md:** Latest Turn (useful-npu-accel S0) prepended; O1 post-fail
   ERT kept as Prior Turn. Date set to 2026-08-12.
3. **AGENTS.md:** matching short Latest Turn prepended; O1 P5 kept as Prior.
4. This report.

## First unlock line (SSoT)

```
C_UNLOCK=2c9c12e8 CONSUMED
```

(Read first `C_UNLOCK=` in `RESIDUAL.md`. Active only if that line has no
CONSUMED. Here: CONSUMED → **none active**.)

## Honesty locks

| Claim | Status |
|-------|--------|
| Pass | **No** |
| invent_pass | **0** |
| not_c_ready | **1** |
| not_ran_on_npu | **1** |
| ran_on_npu | **false** / not claimed |
| IRON PASS / eepy CPU smoke | **not** product `ran_on_npu` |
| residual class | still `wait_cmd_einval` |
| same-shot live | proven under `2c9c12e8`; not Pass |
| empty same-shot reconfirm | **frozen** |
| reopen `2c9c12e8` | **forbidden** |
| mint this pin | **no** |
| sticky ALLOW | **no** |

## Who acts next

- Agent: **S1 ∥ S2 ∥ S3**, then **S4** join.
- Agent-try **S5** (operator expanded): IRON MobileNet / ResNet and/or one
  eepy VitisAI session. Still **not** product `ran_on_npu`.
- Human when they hold it: residual-matched / load-accepted classic 1688
  under `testdata/xdna2_cmd/external_known_good/`.
- Agent does **not** mint. Next product live only under a **new** unlock and
  a **new** named reason.

## Not done (out of S0)

- S1 external-runner backend (no product Rust this slice)
- S2 host-graph tiny linear demo
- S3 WAIT fence diagnostic
- S4 join
- S5 agent-try run

## Explicit non-actions

- No unlock mint
- No live apply / no `XDNA2_ALLOW_EXEC_CMD=1` / no L3 open
- No Pass claim / no `ran_on_npu` from IRON or eepy
- No empty same-shot reconfirm
- No product Rust / no `src/` edit

## Files changed

| Path | Role |
|------|------|
| `RESIDUAL.md` | Latest Turn prepend; first `C_UNLOCK=` still `2c9c12e8` CONSUMED |
| `AGENTS.md` | Standing Latest Turn prepend |
| `.agents/plans/plan-useful-npu-accel.md` | Workspace plan mirror + S5 agent-try note |
| `.agents/reports/impl-useful-accel-s0-pin.md` | This S0 closeout |

## Plan

`.agents/plans/plan-useful-npu-accel.md`
