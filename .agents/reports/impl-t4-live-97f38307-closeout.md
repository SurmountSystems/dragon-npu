# Implement report: T4 closeout after live stock `97f38307`

**Date:** 2026-08-10
**Slice:** T4 closeout only (residual / plan / report; **no** new live EXEC)
**Honesty:** invent_pass=0 · **no** product Pass · **no** `ran_on_npu` · **no** second unlock minted

---

## What this slice did

Closed out the measured stock research-blob CONFIG_CU live shot under `C_UNLOCK=97f38307` without re-running live EXEC, without inventing Pass, and without minting a new unlock.

### Evidence used (already measured; not re-run)

| Item | Value |
|------|--------|
| Unlock used | `C_UNLOCK=97f38307` → **CONSUMED** |
| Experiment | research PDI blob (`config-cu-run`); skip CONFIG_CU **not** run |
| Command | `bash tools/operator_app_load_kit.sh config-cu-run` |
| Log | `/tmp/dragon_npu_live_shot_97f38307.log` |
| last_apply | `/tmp/dragon_npu_last_apply_config_cu_3377745.json` (2215 B) |
| status | `wait_cmd_einval` (T3 classifier honest) |
| ran_on_npu | `false` |
| real_ioctl | `true` |
| detail | `config_cu=1` pdi_bytes=283840 cu_func=0; aiebu_elf=1 rela_count=12; WAIT_CMD EINVAL (os error 22); total_s≈60.88 |
| PDI | CVML 283840; sha16=`a1e65d04add3422c`; research_pdi_not_product_known_good=1 |
| hitl_c | rc=4 |
| Journal | IO_PAGE_FAULT ~`0x2fff80007f00` + aie2_set_cmd_timeout (2026-08-10 ~12:15–12:16) |
| C4 / P6 | not Pass; invent_pass=0 |

Residual class for **this** pin: same as `11bc9016` — honest WAIT EINVAL / orphan IO_PAGE_FAULT / research PDI not product known-good. Do not invent locked APP_LOAD from this pin alone.

---

## Files changed

| Path | Change |
|------|--------|
| `RESIDUAL.md` | Top plain status: unlock **`97f38307` CONSUMED**; residual class; who next = offline delta or T6 hard stop |
| `.agents/reports/live-shot-97f38307.md` | **New** plain-English live shot report |
| Session `plan.md` | T0–T4 research-blob done; not Pass; skip-PDI optional only with new unlock; hard-stop risk |
| `.agents/plans/plan-complete-product-silicon.md` | Workspace mirror aligned |
| `tools/p5_post_c2_interpret.sh` | Unlock banner → latest CONSUMED is `97f38307` |
| `tools/operator_app_load_kit.sh` | Overview unlock/residual text for CONSUMED `97f38307` |
| `tools/hitl_config_cu_once.sh` | Do-not-reopen list includes `97f38307` |
| `AGENTS.md` | New Latest Turn pin for this live closeout |
| `.agents/reports/impl-t4-live-97f38307-closeout.md` | This report |

---

## Residual unlock line quote (SSoT)

From `RESIDUAL.md` Unlock row:

> **`C_UNLOCK=97f38307` CONSUMED** (2026-08-10 stock research-blob CONFIG_CU; last_apply `/tmp/dragon_npu_last_apply_config_cu_3377745.json`; status=`wait_cmd_einval`; invent_pass=0). Unlock ≠ Pass ≠ `ran_on_npu`. Do **not** reopen.

First `C_UNLOCK=` in residual is **CONSUMED** (no active one-shot). Prior: `11bc9016` CONSUMED; `3a70fabf` CONSUMED; `b17b0119` CONSUMED.

---

## Plan status

| Step | Status |
|------|--------|
| T0 Stock live-shot runner | **done** |
| T1 One live stock shot under `11bc9016` | **done** |
| T2 Interpret + residual honesty | **done** |
| T3 Named offline delta (WAIT classifier) | **done** |
| T3b Orphan IO_PAGE_FAULT vs BO map | **done** |
| T3c Research PDI identity + skip A/B tooling | **done** |
| T4 One stock live under `97f38307` (research blob) | **done** (this closeout; **not** Pass) |
| Skip-PDI A/B live | **optional only** with **new** unlock + named offline reason |
| T5 Pass closeout | only if live decide true |
| T6 Hard stop | pin once when no capture / no matmul PDI / no driver-MMU delta |

Goal still product Pass / `ran_on_npu`.

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
| Skip-PDI claimed done | **No** (not the experiment for this one-shot) |

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

**Agent only if** new offline material: real capture under `testdata/xdna2_cmd/external_known_good/`, matmul-matched PDI source, or driver/MMU map fix. Else **T6 hard stop**: pin the blocker once and do not empty-reconfirm or mint unlock without a named reason. Skip-PDI A/B is optional only under a **new** unlock after such a reason.
