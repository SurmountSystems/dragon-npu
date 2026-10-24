# Wave 7 join — agent-max residual close (W7-0 … W7-6)

**Date:** 2026-08-11
**Kind:** Campaign join (offline + capture-instr attempt; **no** live L1).
**Honesty locked:** invent_pass=**0** · Pass=**No** · unlock=**none** · first
`C_UNLOCK=` = **`4008e6fd` CONSUMED** · sticky_ALLOW=**0** · no invent Pass ·
no reopen CONSUMED · public mlir-aie AOT re-run **frozen**.

---

## 1. Campaign status table

| Slice | Status | One-line outcome |
|-------|--------|------------------|
| **W7-0** S0 pin + plan mirror | **Landed** | Docs/residual only; plan → `.agents/plans/plan-silicon-try-everything.md`; invent_pass=0; unlock none |
| **W7-1** capture-instr classic-1688 prefer | **Landed (attempt)** | Classic **1688** primary re-landed as `instr_vendor_post_patch.bin`; body **unpatched** sha16 **`3c5ffca750922d3b`** (same as lab raw / DD dump); multi 32208 diag only; OFM non-zero on vendor stack; **not** Pass / **not** residual_matched / **not** load_accepted |
| **W7-2** collect dry-run + score honesty | **Landed (this join)** | All W7-1 candidates **refuse** collect; residual_matched_claim=**0**; load_accepted_claim=**0**; host scan reconfirm only (no new secret peel) |
| **W7-3** `completed_on_this_fw` soft token | **Landed** | Pure parse + soft note; field alone never residual_matched / load_accepted / Pass |
| **W7-4** cascade MASK* walk hygiene | **Landed** | Compact size **28** + padded **32** accepted; cascade 4512 → ClassicTxnV01 ops=116; research only |
| **W7-5** live L1 | **Skipped** | **Not justified** — no residual-matched new material; same oracle sha as lab raw; **do not mint** unlock; **do not live** |
| **W7-6** join | **This report** | Wave closed offline + capture attempt honesty |

---

## 2. W7-1 capture outcome (detail)

Report: `.agents/reports/wave7-capture-instr-attempt.md`

| Artifact | Bytes | sha16 | Role |
|----------|------:|------|------|
| `gt_capture/instr_vendor_post_patch.bin` | **1688** | **`3c5ffca750922d3b`** | Primary classic **UNPATCHED** (`txn_container_unpatched`) |
| `gt_capture/instr_vendor_txn_bin.bin` | 1688 | `3c5ffca750922d3b` | Same classic map |
| `gt_capture/instr_vendor_get_transaction_bin.bin` | 32208 | `3de5f68d14ff1773` | Multi-txn **diag only** — refuse residual identity |
| `gt_capture/instr_raw_txn.bin` / `lab_instr.bin` | 1688 | `3c5ffca750922d3b` | Lab oracle (identity match to “vendor post_patch”) |
| `gt_capture/instr_post_patch.bin` | 1688 | `ccab3960f6dd789b` | Offline pure-direct post-rela (≠ unpatched body) |
| `gt_capture/ofm_golden.bin` | 512 | `8e6b548203bfc086` | Vendor DD OFM non-zero (NPU ran **outside** dragon-npu Pass path) |

**Identity honesty:**

- Re-landed primary is **classic size-class** but **unpatched** body class (CLASS_HINT **A** vs pure raw; **not** true post-patch).
- sha16 matches lab oracle → **not** new residual-shaped completing-stack material.
- Multi 32208 **refused** as residual identity (wrong body class).
- Capture tool OFM ≠ product `decide_ran_on_npu` (no dragon-npu apply Pass).
- `vendor_paddrs.json`: `capture_mode=txn_container_unpatched`, `from_registry=false`, `classic_unpatched=true`.

**Tool fix (stock L3):** `silicon_lab_operator.sh` treated `exec_cmd_allow=missing` as still-open (exit 5). Fixed: `missing` → closed on stock in-tree driver. `bash -n` OK. Not a Pass unlock.

**Sudo / human:**

- Stock amdxdna: capture-instr ran **without** sudo (no L3 param).
- `sudo -n true` → password required — wall for **hardened L2+L3 install / root sysfs only**, not for classic 1688 re-land on stock.
- Agent did **not** park on operator bag-drop; capture was attempted and succeeded for unpatched classic primary.

---

## 3. W7-2 collect dry-run + residual_matched / load_accepted score

Commands re-run this join (all **exit 2** refuse; invent_pass=0):

| Candidate | class / reason | residual_matched_claim | load_accepted_claim |
|-----------|----------------|------------------------:|--------------------:|
| `instr_vendor_post_patch.bin` | `refused_basename` local_oracle_or_wrong_platform | **0** | **0** |
| `instr_vendor_txn_bin.bin` | `refused_basename` local_oracle_or_wrong_platform | **0** | **0** |
| `instr_vendor_get_transaction_bin.bin` | `historical_multi_txn_wrong_body` size 32208 | **0** | **0** (`multi_txn_refused=1`) |
| `instr_post_patch.bin` | `refused_basename` local_oracle (offline pure rela body) | **0** | **0** |

Nested `capture_from_host_candidate`: all rows
`completing_stack_evidence_ok=0` · `load_accepted=0` · `Pass=No` ·
`not_ran_on_npu=1` · `not_c_ready=1`.

**Vendor parity (offline):** `diff_vendor_instr_parity.sh` → **rc=0**
`CLASS_HINT A` unpatched body class; note **NOT true post-patch; NOT C-ready**.

**Optional host scan:** `host_capture_scan.sh --stdout-only` → 8 roots, **144** hits;
multi_txn=**1**; research_dup=**11**; large_soft=**2**. No new non-public
completing-stack peel. All hits `load_accepted_claim=0 invent_pass=0`.
**Not** empty reconfirm mill — one scan to reconfirm W6 hunt still empty of
residual-matched material after W7-1 re-land.

**Score (locked):**

```
residual_matched_claim=0
load_accepted_claim=0
product_known_good=0
invent_pass=0
Pass=No
not_ran_on_npu=1
not_c_ready=1
classic_unpatched_class_a=1
true_post_patch_missing=1
```

---

## 4. W7-3 / W7-4 (offline landed; not Pass)

| Slice | Report | Honesty |
|-------|--------|---------|
| W7-3 | `.agents/reports/impl-w7-completed-on-this-fw.md` | Soft PROVENANCE `completed_on_this_fw`; always residual_matched=0 load_accepted=0 invent_pass=0 |
| W7-4 | `.agents/reports/impl-w7-cascade-walk-hygiene.md` | MASK* size 28/32 walk; cascade research ClassicTxnV01; ≠ lab classic 1688 residual |

---

## 5. W7-5 live — why skipped

Live L1 requires **new residual-shaped bytes** + **new host-minted unlock** + named reason.

| Gate | Met? |
|------|------|
| Residual-matched new peel | **No** (same sha as lab raw) |
| Load-accepted ERT+OFM evidence | **No** |
| New unlock minted | **No** (forbidden without named offline reason) |
| Public AOT as residual identity | **No** (frozen; research_dup only) |

**Decision:** **W7-5 not run.** Do **not** mint. Do **not** reopen
`4008e6fd` or prior CONSUMED list. Do **not** re-live spent flexml / skip-CU
empty reconfirm.

---

## 6. Human still needed? (proven walls only)

Agent tried capture without parking on operator bag-drop. **Proven** human walls:

1. **Hardened L2+L3 OOT module** install / `exec_cmd_allow` gate — needs sudo password (stock has no param). Evidence: `sudo -n true` → `a password is required`; sysfs param missing on intree module.
2. **True load-accepted / residual-matched completing-stack** material with live **ERT Completed + OFM allclose** via product `decide_ran_on_npu` (or operator-placed capture under `testdata/xdna2_cmd/external_known_good/` with honest PROVENANCE). Evidence: W7-1 unpatched CLASS A only; collect refuse; W6 hunt empty of secret post-patch.
3. **New unlock mint + supervised live** — host-minted only after named offline reason; not from this join.

**Not** a human-only wall for: re-running capture-instr on stock (agent did it); offline classify/parity/collect; soft tokens; MASK* walk.

---

## 7. Next named delta (if any)

| Rank | Next | Notes |
|------|------|-------|
| **P0** | **True post-patch classic 1688** (or residual-shaped peel) that is **not** lab oracle unpatched sha `3c5ffca750922d3b` | INSTR_BO_LIMIT / no public live `run_` BO still architecture residual; unpatched CLASS A **≠** C-ready |
| P1 | Completing-stack load-accepted capture into `testdata/xdna2_cmd/external_known_good/` with ERT+OFM evidence | Collect still fail-closed without evidence flags |
| P2 | Optional: hardened L3 install (operator) if product gate required | Not required for stock capture re-land |
| **Frozen** | Public mlir-aie AOT re-run (300/660/2592/4512) | Already IRON PASS research; not residual progress |
| **Frozen** | Unlock mint / live from this join | No residual_matched material |
| **Frozen** | Empty reconfirm mill / reopen CONSUMED / invent Pass / sticky ALLOW | Standing rules |

No agent empty implement loop: W7 agent-legal steps for this wave are **exhausted**
(capture tried; collect refuse definitive; offline soft/walk done; live not justified).

---

## 8. Honesty footer (join)

```
invent_pass=0
Pass=No
not_ran_on_npu=1
not_c_ready=1
load_accepted_claim=0
residual_matched_claim=0
product_known_good=0
unlock=none
C_UNLOCK=4008e6fd CONSUMED
sticky_ALLOW=0
public_mlir_aie_aot_rerun=frozen
W7-5_live=skipped_no_L1
classic_1688_primary=unpatched_re_landed
true_post_patch=missing
```

---

## 9. Artifacts

| Path | Role |
|------|------|
| `.agents/reports/impl-w7-s0-pin-mirror.md` | W7-0 S0 |
| `.agents/reports/wave7-capture-instr-attempt.md` | W7-1 |
| `.agents/reports/impl-w7-completed-on-this-fw.md` | W7-3 |
| `.agents/reports/impl-w7-cascade-walk-hygiene.md` | W7-4 |
| `.agents/reports/wave6-distance-lab-vs-iron.md` | W6 distance (prior) |
| `.agents/reports/wave6-material-hunt.md` | W6 hunt (prior) |
| `.agents/plans/plan-silicon-try-everything.md` | Plan mirror |
| This file | **W7-6 join** |
| `RESIDUAL.md` / `AGENTS.md` top | Join closeout pin |

---

## 10. One-liner

**W7 closed:** agent capture re-landed classic **1688 unpatched** (oracle sha);
collect/score stay **0/0**; W7-3/4 offline green; **W7-5 no live**; unlock
**none** (`4008e6fd` CONSUMED); invent_pass=**0**; Pass=**No**; true post-patch
still missing; public AOT re-run frozen.
