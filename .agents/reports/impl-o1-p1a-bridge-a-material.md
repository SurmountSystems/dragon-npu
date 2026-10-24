# O1 P1-A — Bridge A / residual-matched material (offline)

**Date:** 2026-08-11
**Plan:** `.agents/plans/plan-option1-after-same-shot.md` (P1-A)
**Scope:** Offline only. Bridge A residual-matched / load-accepted classic-1688 hunt.
**Result:** **not found** · wall unchanged · invent_pass=**0** · Pass=**No** · unlock=**none**

---

## Honesty lock

| Field | Value |
|-------|--------|
| invent_pass | **0** |
| Pass | **No** |
| residual_matched_claim | **0** |
| load_accepted_claim | **0** |
| not_ran_on_npu | **1** |
| not_c_ready | **1** |
| unlock | **none** (first `C_UNLOCK=` = **`2c9c12e8` CONSUMED** — not reopened, not minted) |
| live_justified_from_material | **no** |
| Public mlir-aie AOT as residual identity | **refused** |
| Multi 32208 residual identity | **refused** |
| Capture-instr re-run / same-shot re-live | **not run** (empty reconfirm frozen) |

---

## Goal (this slice)

Hunt for **residual-matched** classic **1688** post-patch (sha16 **≠** lab oracle
`3c5ffca750922d3b`) **or** true **load-accepted** completing-stack peel (ERT
Completed + OFM allclose on this FW via product decide). Document exact wall if
still blocked. Prefer existing tools; no invent bins; no unlock mint.

---

## What was read (priors)

| Report / path | Takeaway used here |
|---------------|--------------------|
| `.agents/reports/wave9-a-bridge-a.md` | Sibling 1688 DD dump: 8 non-oracle sha classes = unpatched other M/K/N; **not** residual post-patch; INSTR_BO_LIMIT / private `run_`; FLM absent |
| `.agents/reports/wave8-material-track.md` | Capture re-land oracle only; multi 32208 refused; registry = Transaction/cache not live instr |
| `.agents/reports/wave7-capture-instr-attempt.md` | Classic 1688 primary = oracle `3c5ffca750922d3b`; CAPTURE_MODE_UNPATCHED |
| `.agents/reports/wave6-material-hunt.md` | Public peels research_dup; pure-direct `ccab3960…` synthetic; no host post-patch |
| `testdata/xdna2_cmd/external_known_good/PROVENANCE.md` | 300/660/2592/4512 research only; load-accepted table empty |
| Plan option1 + RESIDUAL Latest Turn | Same-shot proven; residual `wait_cmd_einval`; primary wall = material |

---

## What ran this slice (offline only)

| # | Action | Result |
|---|--------|--------|
| 1 | Re-fingerprint in-tree + bag ctrltext | Oracle / synthetic / public peels **unchanged** (table below) |
| 2 | `bash tools/host_capture_scan.sh --stdout-only` | roots=**8** hits=**147** multi=**1** research_dup=**11**; **no** residual-matched ingest |
| 3 | `collect_load_accepted_candidate.sh --dry-run` on oracle / multi / synthetic / bag 300 | All refuse: basename / multi_txn / research_duplicate; claims stay 0 |
| 4 | `load_accepted_path_status.sh` | Path B trees ready (IRON research); Path A FLM/Lemonade **absent**; bag research peels only |
| 5 | Presence check FastFlowLM / Lemonade / `/opt/xilinx` / `/opt/amd/ryzen-ai` | **All absent** |
| 6 | `/tmp` 1688-byte files | Only prior **`/tmp/w9a_dd_dump`** siblings (research; not new, not residual match) |
| 7 | Capture-instr / live apply / unlock mint | **Not run** |

No new bytes written under `testdata/xdna2_cmd/external_known_good/`. No PROVENANCE
load-accepted row filled. No product code change (honesty helpers already refuse
multi 32208 + research-duplicate shas).

---

## Material inventory (current, verified)

### Lab classic family (completing-stack residual target)

| Stream | Bytes | sha16 | Role | residual_matched | load_accepted |
|--------|------:|-------|------|------------------|---------------|
| `gt_capture/instr_raw_txn.bin` | 1688 | **`3c5ffca750922d3b`** | Unpatched lab oracle / DD map | **0** | **0** |
| `gt_capture/instr_vendor_post_patch.bin` | 1688 | **`3c5ffca750922d3b`** | Name says post_patch; body **unpatched** (txn_container) | **0** | **0** |
| `gt_capture/instr_vendor_txn_bin.bin` | 1688 | **`3c5ffca750922d3b`** | Same map | **0** | **0** |
| `gt_capture/instr_post_patch.bin` | 1688 | **`ccab3960f6dd789b`** | Offline pure-direct rela, **synthetic** paddrs | **0** (not live completing-stack) | **0** |
| `gt_capture/instr_vendor_get_transaction_bin.bin` | **32208** | `3de5f68d14ff1773` | Multi-txn diag only | **refused** | **refused** |

### Public bag (research only — not residual identity)

| Stream | Bytes | sha16 | Class |
|--------|------:|-------|--------|
| `working_stack_ctrltext.bin` / vector | 300 | `aaeb5b68cc298a45` | public-npu2-small research_dup |
| single_core matmul | 660 | `de8671245bb72436` | research_dup |
| whole_array | 2592 | `7ee4b267ce31cd70` | research_dup |
| cascade | 4512 | `18cb653c05a9d4c8` | research_dup |

None equal lab oracle or live post-patch. Helpers refuse load-accepted re-tag
(`research_duplicate_refused=1`).

### Found this slice?

| Question | Answer |
|----------|--------|
| New residual-matched classic 1688 post-patch? | **No** |
| New load-accepted peel (ERT Completed + OFM allclose + decide)? | **No** |
| New sha under bag? | **No** (no ingest) |
| Non-oracle 1688 still on host? | **Yes** — W9 DD siblings under `/tmp/w9a_dd_dump` only; unpatched **other** M/K/N; **not** residual match |

---

## Exact wall (primary)

**True residual-matched / load-accepted completing-stack material is still absent.**

| Wall | Exact fact |
|------|------------|
| Lab-key classic after public capture | Always oracle **`3c5ffca750922d3b`** (unpatched Transaction map) |
| Live post-patch instr BO via public DD | **Closed:** registry `get_instr_bo` = Transaction/cache host image; after execute throws **INSTR_BO_LIMIT** for staged family (W8 exact strings); private `xrt::run run_` not dumpable |
| Multi-txn 32208 | **Refused** residual identity (`multi_txn_refused=1`) |
| Sibling 1688 non-oracle (W9) | Unpatched other shapes — **not** lab residual post-patch |
| Synthetic pure-direct `ccab3960…` | In-tree GT only — **not** live completing-stack |
| Public mlir-aie AOT (300/660/2592/4512) | RESEARCH ONLY — **not** residual identity — **not** product load_accepted |
| FastFlowLM / Lemonade intercept | **Absent** (operator install only; agent will not network-install) |
| Hardened L3 / sudo | Stock param absent; sudo password required for OOT gate (not needed to prove this material wall) |
| Unlock | **none active**; do **not** mint from this report |

**One-line wall:** no public path peels live-patched classic-1688 of the lab residual key; bag holds only research peels and unpatched oracle.

---

## live_justified_from_material

| Field | Value |
|-------|--------|
| **live_justified_from_material** | **no** |

Reason: no residual-matched or load-accepted new material. Same-shot hygiene already
proven under CONSUMED `2c9c12e8`. A new live needs a **new** unlock + **new** named
reason after P1-B/P2 join — **not** from this material pin alone.

---

## Who acts next

1. **Operator (material path):** place a true completing-stack peel under
   `testdata/xdna2_cmd/external_known_good/` with PROVENANCE
   `completed_on_this_fw=yes` only when ERT Completed + OFM allclose is real —
   **or** install FastFlowLM/Lemonade if that is the chosen intercept stack.
2. **Agent offline (plan):** **P1-B** wait-path observability in parallel (or
   next); then **P2** join for one named live reason **or** stop.
3. **Frozen:** empty same-shot reconfirm; re-tag public AOT as residual/load-accepted;
   re-capture stock expecting a different oracle; invent residual_matched=1;
   mint unlock from this report.

---

## What this is not

- Not Pass / not `ran_on_npu` / not C-ready
- Not residual_matched / not load_accepted
- Not unlock mint / not sticky ALLOW
- Not public AOT re-run
- Not re-live CONSUMED `2c9c12e8`
- Not claim that DD sibling 1688 or synthetic `ccab3960…` closes product residual

---

## Artifacts

| Path | Role |
|------|------|
| `.agents/reports/impl-o1-p1a-bridge-a-material.md` | This report |
| Prior W9-A | `.agents/reports/wave9-a-bridge-a.md` |
| Prior W8 material | `.agents/reports/wave8-material-track.md` |
| `/tmp/w9a_dd_dump/` | Research sibling 1688 dumps (unchanged) |
| `testdata/xdna2_cmd/external_known_good/` | Research peels only (no new ingest) |

---

## Footer

```
found=no
new_sha=none
residual_matched_claim=0
load_accepted_claim=0
live_justified_from_material=no
invent_pass=0
Pass=No
unlock=none
first_C_UNLOCK=2c9c12e8_CONSUMED
wall=INSTR_BO_LIMIT_private_run_no_live_post_patch_peel_public_AOT_research_only
primary_next=P1-B_wait_offline_or_operator_load_accepted_bytes
```
