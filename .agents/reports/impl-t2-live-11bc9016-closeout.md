# Implement report: T2 closeout after live stock `11bc9016`

**Date:** 2026-08-10
**Slice:** T2 only (interpret + residual honesty + plan update)
**Honesty:** invent_pass=0 · **no** product Pass · **no** `ran_on_npu` · **no** new unlock minted

---

## What this slice did

Closed out the measured stock CONFIG_CU live shot under `C_UNLOCK=11bc9016` without re-running live EXEC, without inventing Pass, and without minting a new unlock (T4 stays after T3).

### Evidence used (already measured; not re-run)

| Item | Value |
|------|--------|
| Unlock used | `C_UNLOCK=11bc9016` → **CONSUMED** |
| Command | `bash tools/operator_app_load_kit.sh config-cu-run` / hitl run-stock |
| Log | `/tmp/dragon_npu_live_shot_11bc9016.log` |
| last_apply | `/tmp/dragon_npu_last_apply_config_cu_1374078.json` (2208 B) |
| status | `wait_timeout` |
| ran_on_npu | `false` |
| real_ioctl | `true` |
| detail | `config_cu=1` pdi_bytes=283840 cu_func=0; aiebu_elf=1 rela_count=12; WAIT_CMD 5s then EINVAL (os error 22); total_s≈62.87 |
| hitl_c | rc=4; RUN_STOCK_DONE |
| Journal | IO_PAGE_FAULT + aie2_set_cmd_timeout (2026-08-10 ~11:34–11:35) |

Residual class for **this** pin: wait_timeout / FW timeout / IO_PAGE_FAULT. Do not invent locked APP_LOAD from this pin alone.

---

## Files changed

| Path | Change |
|------|--------|
| `.agents/reports/live-shot-11bc9016.md` | **New** plain-English live shot report |
| `RESIDUAL.md` | Top plain status: unlock CONSUMED; residual class; who next = T3 |
| Session `plan.md` | T0–T2 done; T3 next; unlock CONSUMED; no human stock paste theater |
| `.agents/plans/plan-complete-product-silicon.md` | Workspace mirror aligned |
| `tools/p5_post_c2_interpret.sh` | Unlock banner → latest CONSUMED is `11bc9016` |
| `tools/operator_app_load_kit.sh` | Overview unlock/residual text for CONSUMED `11bc9016` |
| `tools/hitl_config_cu_once.sh` | Do-not-reopen list includes `11bc9016` |
| `AGENTS.md` | New Latest Turn pin for this live shot |
| `.agents/reports/impl-t2-live-11bc9016-closeout.md` | This report |

---

## Residual unlock line quote (SSoT)

From `RESIDUAL.md` Unlock row:

> **`C_UNLOCK=11bc9016` CONSUMED** (2026-08-10 stock live; wait_timeout; last_apply pin `/tmp/dragon_npu_last_apply_config_cu_1374078.json`; journal IO_PAGE_FAULT + aie2_set_cmd_timeout). Unlock ≠ Pass ≠ `ran_on_npu`. Do **not** reopen.

First `C_UNLOCK=` in residual is **CONSUMED** (no active one-shot). Prior: `3a70fabf` CONSUMED; `b17b0119` CONSUMED.

---

## Plan status

| Step | Status |
|------|--------|
| T0 stock live-shot runner | **done** |
| T1 one live stock shot under `11bc9016` | **done** |
| T2 interpret + residual honesty | **done** (this slice) |
| T3 named offline delta | **next** (wait / WAIT_CMD EINVAL / BO map / IO_PAGE_FAULT) |
| T4 new unlock + re-shot | after T3 only |
| Goal | still product Pass / `ran_on_npu` |

---

## Honesty checklist

| Claim | Value |
|-------|--------|
| invent_pass | **0** |
| product Pass | **No** |
| ran_on_npu | **No** |
| New unlock minted this slice | **No** |
| Live EXEC re-run this slice | **No** |
| L3 / ALLOW opened this slice | **No** |
| Capture bytes invented | **No** |
| APP_LOAD invented from this pin | **No** |

---

## Shell check

```text
bash -n tools/p5_post_c2_interpret.sh
bash -n tools/operator_app_load_kit.sh
bash -n tools/hitl_config_cu_once.sh
```

(Run at closeout; expect exit 0.)

---

## Next (not this slice)

**Agent T3:** named offline product delta from wait_timeout / WAIT_CMD EINVAL (os error 22) / IO_PAGE_FAULT / BO map. No second unlock until T3 lands. No empty reconfirm mill.
