# Wave 9 join — W9-B L1b-prep + W9-A intercept/sibling (W9-0 … W9 join)

**Date:** 2026-08-11
**Kind:** Campaign join (W9-B offline L1b readiness + W9-A residual post-patch hunt;
**no** live L1b; **no** unlock mint).
**Honesty locked:** invent_pass=**0** · Pass=**No** · unlock=**none** · first
`C_UNLOCK=` = **`4008e6fd` CONSUMED** · sticky_ALLOW=**0** · no invent Pass ·
no residual_matched · no load_accepted · public mlir-aie AOT re-run **frozen**.

---

## 1. Campaign status table

| Slice | Status | One-line outcome |
|-------|--------|------------------|
| **W9-0** S0 pin + plan mirror | **Landed** | Docs/residual only; plan → `.agents/plans/plan-silicon-try-everything.md`; invent_pass=0; unlock none |
| **W9-B** L1b-prep + T3 readiness | **Landed (offline)** | Live paddrs **found on disk** (vendor fixtures + `/tmp` last_apply `bo_diag`); `paddr_evidence_ready=1`; `l1b_offline_ready=1` when extract succeeds; **`l1b_live_justified=0`** (unlock still required; host UVAs age out) |
| **W9-A** intercept / sibling | **Landed (wall)** | No residual-matched post-patch classic 1688; sibling non-oracle 1688s = unpatched other M/K/N only; INSTR_BO_LIMIT / private `run_` closed; FLM/Lemonade absent |
| **Live L1b** | **Skipped** | **Not justified** — no residual-matched material; no active unlock; stale host UVAs must **not** drive a new product live patch; **do not mint** |
| **W9 join** | **This report** | Both bridges closed offline; unlock none; next named delta below |

---

## 2. W9-B — L1b prep + T3 readiness (detail)

Report: `.agents/reports/wave9-b-l1b-prep.md`

| Fact | Value |
|------|--------|
| Live paddrs on disk | **Yes** (vendor JSON + multiple last_apply `bo_diag` rows) |
| Vendor fixture triad | e.g. `gt_capture/vendor_paddrs_from_last_apply.json` ifm/wts/ofm/ctrl present |
| O3 extract change | Also mines `bo_diag` labels when `io_paddrs` token absent (pure; refuse 0 / `u64::MAX`) |
| APIs | `L1bReadiness` / `evaluate_l1b_readiness` / `format_l1b_readiness_checklist*` / soft score tokens |
| Tests | `cargo test --features amd-xdna2 --lib w9_b_` → **4 ok** (+ O3 regression) |
| `paddr_evidence_ready` | **1** offline when paddrs present |
| `l1b_offline_ready` | **1** when paddrs present (pure-direct rela **can** use research paddrs offline) |
| `unlock_active` | **0** (no mint this wave) |
| **`l1b_live_justified`** | **0** without unlock_active; unlock alone still needs **same-shot** paddrs at apply time |
| Host UVA caveat | Research UVAs from Shmem **age out across processes** — must **not** drive product live patch |
| residual_matched / invent_pass | **0** / **0** locked |
| Unlock mint this track | **none** |

**One-liner:** Offline evidence ready; live L1b **not** justified on stale UVAs without new unlock + same-shot map.

---

## 3. W9-A — Bridge A residual post-patch hunt (detail)

Report: `.agents/reports/wave9-a-bridge-a.md`

| Fact | Value |
|------|--------|
| Goal | Residual-matched classic **1688** post-patch (sha16 ≠ lab oracle `3c5ffca750922d3b`) or exact walls |
| Lab oracle unpatched sha16 | **`3c5ffca750922d3b`** (NumOps 40, gen4 8×6) |
| T2 sibling keys (prmptflat) | **56** classic 1688 bins; **9** sha classes; **8** non-oracle = unpatched **other M/K/N** only |
| Residual-matched post-patch | **No** (`residual_matched_claim=0`) |
| Synthetic offline patch | In-tree `ccab3960f6dd789b` (synthetic paddrs; **not** completing-stack live post-patch) |
| T1 / T2-ep | Public `get_instr_bo` = cache not live `run_`; INSTR_BO_LIMIT after execute; multi 32208 refused |
| FastFlowLM / Lemonade | **Absent** (operator install only; no silent network install) |
| Stock capture-instr / public AOT | **Not re-run** (W8 already oracle; AOT frozen) |
| Host hunt | No new residual-matched peel into `testdata/xdna2_cmd/external_known_good/` |
| load_accepted_claim | **0** |

**One-liner:** Wall — siblings ≠ post-patch; public live instr dump closed; no residual-matched classic 1688.

---

## 4. Live L1b — why skipped (locked decision)

| Gate | Met? |
|------|------|
| Residual-matched post-patch classic 1688 | **No** (W9-A wall; oracle + unpatched siblings only) |
| Load-accepted ERT+OFM via product `decide_ran_on_npu` | **No** |
| Offline `paddr_evidence_ready` | **Yes** (research UVAs on disk) |
| Same-shot live device paddrs at apply time | **No** (disk UVAs age out; must not drive product patch) |
| New host-minted unlock + named reason | **No** (forbidden this join; `4008e6fd` CONSUMED) |
| Offline L1b checklist alone | **Not enough** (`l1b_live_justified=0` without unlock_active) |
| Public AOT / sibling shape table as residual identity | **No** (frozen / research only) |

**Decision:** **Live not run.** Do **not** mint. Do **not** reopen `4008e6fd` or prior CONSUMED list. Do **not** empty reconfirm unpatched oracle. Do **not** re-live spent flexml / skip-CU without new reason. Do **not** invent residual match / load_accepted / Pass.

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
paddr_evidence_ready=1
l1b_offline_ready=1
l1b_live_justified=0
true_post_patch_classic_1688=missing
oracle_sha16=3c5ffca750922d3b
w9a_non_oracle_sibling_1688=unpatched_other_dims_only
multi_txn_32208=refused
INSTR_BO_LIMIT=proven
unlock=none
C_UNLOCK=4008e6fd CONSUMED
public_mlir_aie_aot_rerun=frozen
sticky_ALLOW=0
live_l1b=skipped
```

---

## 6. Next named delta (not this join)

1. **Same-shot paddr L1b wiring (product):** live L1b only when paddrs are taken from the **same** apply map shot (not aged host UVAs from old last_apply), under a **new** host-minted unlock + named reason. Offline `paddr_evidence_ready=1` alone does **not** open live.
2. **Private `run_` intercept / true post-patch:** residual-matched classic 1688 post-patch still highest Pass gap. Paths: completing-stack live-patched peel (or residual-shaped load-accepted ERT+OFM under `testdata/xdna2_cmd/external_known_good/`), DD/XRT private BO dump tool / source fork, or operator FLM/Lemonade install. Not another unpatched re-capture of `3c5ffca750922d3b`. Not multi 32208. Not public AOT re-run. Not sibling other-dim 1688 as residual identity.
3. **Freeze list:** empty reconfirm of unpatched oracle; reopen CONSUMED unlocks; invent residual match / load_accepted / Pass; sticky ALLOW; agent live without unlock; product live patch from **stale** disk UVAs; public AOT re-run as residual match.

---

## 7. Artifacts

| Path | Role |
|------|------|
| `.agents/reports/impl-w9-s0-pin-mirror.md` | W9 S0 plan pin |
| `.agents/reports/wave9-b-l1b-prep.md` | W9-B L1b readiness + O3 bo_diag mine |
| `.agents/reports/wave9-a-bridge-a.md` | W9-A sibling / intercept walls |
| `.agents/reports/wave9-join.md` | This join |
| `.agents/plans/plan-silicon-try-everything.md` | Campaign plan |
| `/tmp/w9a_dd_dump/` | Research sibling 1688 dumps (not load-accepted) |
