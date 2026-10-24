# Wave 10 join — same-shot L1b path + soft classify (W10-S0 … W10-join)

**Date:** 2026-08-11
**Kind:** Campaign join (W10-S0 pin + W10-B same-shot offline wire + W10-soft
classify tokens; **no** live L1b; **no** unlock mint).
**Honesty locked:** invent_pass=**0** · Pass=**No** · unlock=**none** · first
`C_UNLOCK=` = **`4008e6fd` CONSUMED** · sticky_ALLOW=**0** · no invent Pass ·
no residual_matched · no load_accepted · public mlir-aie AOT re-run **frozen** ·
stale disk UVAs **forbidden** for product live.

---

## 1. Campaign status table

| Slice | Status | One-line outcome |
|-------|--------|------------------|
| **W10-S0** plan pin + mirror | **Landed** | Docs/residual only; plan → `.agents/plans/plan-silicon-try-everything.md`; invent_pass=0; unlock none; W9 closed, W10 approved |
| **W10-B** same-shot (Bridge B) | **Landed (offline)** | Pure same-shot plan/apply + env-gated lab hook; **same-shot offline wire ready=yes**; product rela never loads disk UVAs; invent_pass=0 |
| **W10-soft** L1b classify lines | **Landed (offline)** | Soft `classify_last_apply` always prints `live_paddrs_extract=` / `l1b_readiness=` / `l1b_soft_score=`; offline `l1b_live_justified=0` |
| **Live L1b** | **Skipped** | **Not justified** — offline wire alone ≠ unlock; no active unlock; stale disk UVAs forbidden; **do not mint** |
| **W10 join** | **This report** | Offline Bridge B closed; soft tokens on; unlock none; next named delta below |

---

## 2. W10-B — same-shot rela product offline wire (detail)

Report: `.agents/reports/wave10-b-same-shot.md`

| Fact | Value |
|------|--------|
| Goal | Patch aiebu rela×12 with **in-process** BO `xdna_addr`, not aged `/tmp` last_apply / vendor UVAs |
| Pure plan API | `plan_same_shot_rela_from_io_paddrs` / `plan_same_shot_rela_from_disk_extract` / `SameShotRelaPlan` / `SameShotPaddrSource` |
| Pure apply research | `apply_same_shot_rela_research` + soft plan/apply notes |
| Env | `XDNA2_LAB_SAME_SHOT_RELA` default **off**; `1`/`true`/`yes`/`on` enable |
| Lab hook | When env on: plan from current-process IO paddrs before patch; invalid → `same_shot_rela_plan_failed` (fail-closed) |
| Disk UVAs into product rela | **Never** (`plan_same_shot_rela_from_disk_extract` always `apply_ready=0`) |
| Default path (env unset) | Unchanged: still patches with in-process `ios.*_addr` when OFM path on |
| Tests | `cargo test --features amd-xdna2 --lib w10_b_` → **4 ok** (+ w9_b / w8 regression) |
| **same-shot offline wire ready** | **yes** |
| **live L1b justified from wire alone** | **no** |
| Unlock mint this track | **none** |

**API / env one-liners (from W10-B):**

| Name | Role |
|------|------|
| `ENV_LAB_SAME_SHOT_RELA` / `XDNA2_LAB_SAME_SHOT_RELA` | Opt-in same-shot plan gate (default off) |
| `SameShotPaddrSource` | `InProcessBoMap` / `DiskLastApply` / `DiskVendorJson` / `Absent` |
| `SameShotRelaPlan` | `same_shot` / `paddrs_valid` / `apply_ready` / `refuse_stale_disk` |
| `plan_same_shot_rela_from_io_paddrs` | In-process bases → apply_ready when valid |
| `plan_same_shot_rela_from_disk_extract` | Disk extract → always apply_ready=0 |
| `apply_same_shot_rela_research` | Pure-direct patch + classic walk research |
| `lab_same_shot_rela_env_token_is_enabled` / `_requested` | Token table + process env |

**One-liner:** Offline same-shot wire ready; live still needs **new** unlock + named reason + real map at apply time.

---

## 3. W10-soft — L1b readiness on soft classify (detail)

Report: `.agents/reports/wave10-soft-classify.md`

| Soft line prefix | Source | Honesty always |
|------------------|--------|----------------|
| `live_paddrs_extract=` | `extract_live_paddrs_from_last_apply_json` + research note | invent_pass=0 · paddrs_alone_not_pass=1 |
| `l1b_readiness=` | `format_l1b_readiness_checklist_from_extract` (offline unlock=false) | invent_pass=0 · l1b_live_justified=0 · residual_matched=0 |
| `l1b_soft_score=` | `format_l1b_soft_score_tokens` | true_post_patch_missing=1 · oracle_sha16 · INSTR_BO_LIMIT=1 |

**Greppable wall tokens on soft report:** `true_post_patch_missing=1` ·
`oracle_sha16=3c5ffca750922d3b` · `INSTR_BO_LIMIT=1` · `paddr_evidence_ready=0|1` ·
`l1b_live_justified=0` · `invent_pass=0`.

Fixture smoke: locked historical last_apply has no IO paddrs →
`paddr_evidence_ready=0` / `l1b_offline_ready=0`; with bo_diag / io_paddrs those
may flip **offline** while **`l1b_live_justified` stays 0** without unlock.

**One-liner:** Soft always shows L1b wall; checklist alone ≠ C-ready ≠ unlock.

---

## 4. Live L1b — why skipped (locked decision)

| Gate | Met? |
|------|------|
| Same-shot offline wire ready (W10-B) | **Yes** |
| Soft L1b readiness tokens (W10-soft) | **Yes** |
| Residual-matched post-patch classic 1688 | **No** (W9-A wall still stands) |
| Load-accepted ERT+OFM via product `decide_ran_on_npu` | **No** |
| Same-shot **live** device paddrs under **new** unlock | **No** (no active unlock; no live run) |
| New host-minted unlock + named reason | **No** (forbidden this join; `4008e6fd` CONSUMED) |
| Offline wire / checklist alone | **Not enough** for live |
| Stale disk UVAs for product live patch | **Forbidden** |
| Public AOT as residual identity | **No** (frozen / research only) |

**Decision:** **Live not run.** Do **not** mint. Do **not** reopen `4008e6fd`
or prior CONSUMED list. Do **not** empty reconfirm unpatched oracle. Do **not**
re-live spent flexml / skip-CU without new reason. Do **not** invent residual
match / load_accepted / Pass. Do **not** product-live-patch from stale disk UVAs.

---

## 5. Honesty footer (locked)

```
invent_pass=0
Pass=No
not_ran_on_npu=1
not_c_ready=1
load_accepted_claim=0
residual_matched_claim=0
product_known_good=0
same_shot_offline_wire_ready=1
l1b_live_justified=0
true_post_patch_classic_1688=missing
oracle_sha16=3c5ffca750922d3b
INSTR_BO_LIMIT=proven
stale_disk_uva_product_live=forbidden
unlock=none
C_UNLOCK=4008e6fd CONSUMED
public_mlir_aie_aot_rerun=frozen
sticky_ALLOW=0
live_l1b=skipped
mint=no
```

---

## 6. Next named delta (not this join)

1. **W11 / T1-L1b-live (only if justified):** **new** host-minted unlock + named
   reason + one product live under `XDNA2_LAB_SAME_SHOT_RELA=1` (and full
   silicon-mladf recipe) → interpret via product `decide_ran_on_npu`. Offline
   wire ready alone does **not** open live. Do **not** mint from this join.
2. **Bridge A material:** private `run_` / EP intercept / load-accepted peel past
   INSTR_BO_LIMIT; residual-matched classic 1688 post-patch still highest Pass
   gap. Not another unpatched re-capture of `3c5ffca750922d3b`. Not multi 32208.
   Not public AOT re-run. Not sibling other-dim 1688 as residual identity.
3. **Freeze list:** empty reconfirm of same-shot offline green; reopen CONSUMED
   unlocks; invent residual match / load_accepted / Pass; sticky ALLOW; agent
   live without unlock; product live patch from **stale** disk UVAs; public AOT
   re-run as residual match; mint from plan / join / soft tokens alone.

Plan SoT: `.agents/plans/plan-silicon-try-everything.md` (post-W10 refresh).

---

## 7. Artifacts

| Path | Role |
|------|------|
| `.agents/reports/wave10-join.md` | This join |
| `.agents/reports/wave10-b-same-shot.md` | W10-B same-shot offline wire |
| `.agents/reports/wave10-soft-classify.md` | W10-soft L1b soft lines |
| `.agents/reports/impl-w10-s0-pin-mirror.md` | W10 S0 pin |
| `.agents/plans/plan-silicon-try-everything.md` | Campaign plan (post-W10) |
| `RESIDUAL.md` top | SSoT residual + unlock |
| `AGENTS.md` top | Standing pin short form |
