# Wave 8 join — Track Material + Track Paddr (W8-0 … W8 join)

**Date:** 2026-08-11
**Kind:** Campaign join (Material agent hunt + Paddr pure TDD; **no** live L1b).
**Honesty locked:** invent_pass=**0** · Pass=**No** · unlock=**none** · first
`C_UNLOCK=` = **`4008e6fd` CONSUMED** · sticky_ALLOW=**0** · no invent Pass ·
no reopen CONSUMED · public mlir-aie AOT re-run **frozen** · L1b offline-ready=**0**.

---

## 1. Campaign status table

| Slice | Status | One-line outcome |
|-------|--------|------------------|
| **W8-0** S0 pin + plan mirror | **Landed** | Docs/residual only; plan → `.agents/plans/plan-silicon-try-everything.md`; invent_pass=0; unlock none |
| **Track Material M1** true post-patch classic 1688 | **Landed (blocked)** | Re-capture still oracle unpatched sha16 **`3c5ffca750922d3b`**; no new post-patch classic 1688 |
| **Track Material M4** INSTR_BO_LIMIT / private `run_` | **Landed (proven)** | Registry `get_instr_bo` throws INSTR_BO_LIMIT after execute; public live instr BO closed; `run_` private |
| **Track Material A5** DD instruction_cache after fit_MKN | **Landed (wall)** | Stock OFM non-zero OK; registry still INSTR_BO_LIMIT; hardened L3 = sudo password wall (`sudo: a password is required`) |
| **Track Paddr O3** live paddr extract | **Landed** | Pure TDD: `extract_live_paddrs_from_*` + soft note; zeros/MAX refuse; invent_pass=0 |
| **Track Paddr M3** live-mapped pure-direct rela | **Landed** | Pure TDD: live paddrs differ from GT synthetic `ccab3960…`; soft `l1b_offline_ready=0` |
| **Track Paddr O1/O2** capture patch class | **Landed** | Unpatched / multi loud refuse residual_matched; CLASS A not match |
| **Live L1b** | **Skipped** | **Not justified** — no residual-matched new material; offline wire alone ≠ unlock; **do not mint** |
| **W8 join** | **This report** | Both tracks closed offline; unlock none; next named delta below |

---

## 2. Track Material (detail)

Report: `.agents/reports/wave8-material-track.md`

| Fact | Value |
|------|--------|
| W8 classic primary sha16 | **`3c5ffca750922d3b`** (same as lab oracle / W7 re-land) |
| True post-patch classic 1688 | **none** (`same_as_oracle=True` on every classic path) |
| Multi 32208 | sha16 `3de5f68d14ff1773` · **refused** residual identity (`multi_txn_refused=1`) |
| Collect dry-run | exit **2** refuse; residual_matched_claim=**0**; load_accepted_claim=**0** |
| INSTR_BO_LIMIT | Live repeated after execute + fit_MKN for staged + GM keys |
| Live instr BO public API | **false** (`mladfmatmulbias` IO/const only; private `run_`) |
| A5 L3 hardened | `sudo -n` / null passphrase → password required; param absent on stock |
| residual_matched_claim | **0** |
| load_accepted_claim | **0** |

**One-liner:** Material re-ran capture + registry; still unpatched oracle; public true post-patch path **closed** (INSTR_BO_LIMIT + private `run_`); multi refused; no unlock.

---

## 3. Track Paddr (detail)

Report: `.agents/reports/wave8-paddr-track.md`

| Fact | Value |
|------|--------|
| O3 APIs | `LivePaddrsExtract` / `extract_live_paddrs_from_{detail,vendor_json,last_apply_json}` + format note |
| M3 APIs | `LiveMappedRelaResearch` / `research_live_mapped_rela` (+ from extract) + format note |
| O1/O2 | `CapturePatchClass` / `classify_capture_patch_class` (unpatched / multi refuse residual) |
| Tests | `cargo test --features amd-xdna2 --lib w8_` → **5 ok** |
| Gates | fmt + clippy `-D` amd-xdna2 lib + w8_ filter green |
| GT synthetic patched sha16 | `ccab3960…` (local rela; **≠** vendor live post-patch) |
| Lab classic unpatched sha16 | `3c5ffca750922d3b` |
| **l1b_offline_ready** | **0** (wire alone ≠ live unlock) |
| invent_pass | **0** |
| Unlock mint this track | **none** |

**One-liner:** Paddr pure TDD landed; L1b needs **live paddrs + new unlock + named reason**, not this join alone.

---

## 4. Live L1b — why skipped

| Gate | Met? |
|------|------|
| Residual-matched new peel (true post-patch classic 1688) | **No** (still oracle `3c5ffca750922d3b`) |
| Load-accepted ERT+OFM via product `decide_ran_on_npu` | **No** |
| Live device paddrs from a real map shot | **No** (O3 parsers only; no live extract this wave) |
| New host-minted unlock + named reason | **No** (forbidden without residual-shaped delta) |
| Offline M3 wire alone | **Not enough** (`l1b_offline_ready=0`) |
| Public AOT as residual identity | **No** (frozen; research only) |

**Decision:** **Live not run.** Do **not** mint. Do **not** reopen `4008e6fd` or prior CONSUMED list. Do **not** empty reconfirm unpatched oracle. Do **not** re-live spent flexml / skip-CU without new reason.

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
l1b_offline_ready=0
true_post_patch_classic_1688=missing
oracle_sha16=3c5ffca750922d3b
w8_classic_primary_sha16=3c5ffca750922d3b
multi_txn_32208=refused
INSTR_BO_LIMIT=proven
unlock=none
C_UNLOCK=4008e6fd CONSUMED
public_mlir_aie_aot_rerun=frozen
sticky_ALLOW=0
```

---

## 6. Next named delta (not this join)

1. **Material still highest gap:** true completing-stack **post-patch** classic 1688, or load-accepted peel under `testdata/xdna2_cmd/external_known_good/` with ERT Completed + OFM allclose via product `decide_ran_on_npu`. Not another unpatched re-capture of `3c5ffca750922d3b`. Not multi 32208. Not public AOT re-run.
2. **L1b only after:** live-mapped paddrs from a real last_apply/journal map **and** a **new** host-minted unlock with named reason (live-mapped rela on classic 1688 aiebu under real map). Offline O3/M3 wire alone does **not** open L1b.
3. **Human-only walls (exact):** hardened L2+L3 install / `exec_cmd_allow` needs sudo password on this host; DD private `run_` / source fork for live instr dump is human/source, not empty reconfirm.
4. **Freeze:** empty reconfirm of unpatched oracle; reopen CONSUMED unlocks; invent residual match / load_accepted / Pass; sticky ALLOW; agent live without unlock.

---

## 7. Artifacts

| Path | Role |
|------|------|
| `.agents/reports/wave8-material-track.md` | Material M1/M4/A5 evidence |
| `.agents/reports/wave8-paddr-track.md` | Paddr O3/M3 pure TDD |
| `.agents/reports/impl-w8-s0-pin-mirror.md` | W8 S0 plan pin |
| `.agents/plans/plan-silicon-try-everything.md` | Campaign plan mirror |
| `.agents/reports/wave7-join.md` | Prior campaign join |
| This file | W8 join closeout |

---

## One-liner

**W8 joined:** Material still oracle unpatched `3c5ffca750922d3b` + INSTR_BO_LIMIT + multi refuse + L3 sudo wall; Paddr O3+M3 pure TDD green with `l1b_offline_ready=0`; invent_pass=0; Pass=No; unlock=none; live not justified; next = true post-patch / load-accepted material, L1b only after live paddrs + new unlock.
