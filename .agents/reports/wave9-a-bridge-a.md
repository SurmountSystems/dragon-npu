# W9-A Bridge A — residual-matched classic 1688 post-patch hunt

**Date:** 2026-08-11
**Scope:** Agent Bridge A only. Offline + stock host tools. No unlock mint. No sticky ALLOW.
**Result:** **no residual-matched post-patch classic 1688** · walls proven · invent_pass=0 · Pass=**No** · unlock=**none**

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
| unlock | **none** (first `C_UNLOCK=` still **`4008e6fd` CONSUMED** — not reopened, not minted) |
| Public mlir-aie AOT re-run | **not run** (frozen) |
| Stock capture-instr re-run | **not run** (W8 proved oracle unpatched) |
| Multi 32208 residual identity | **refused** |

---

## Goal

Find **residual-matched** classic **1688** **post-patch** (sha16 **≠** lab oracle `3c5ffca750922d3b`)
**or** prove exact walls.

Priority order executed: **T2-sibling-keys** (primary this wave) → **T1-run-intercept / T2-ep** (API + stack walls) → **T4-host-hunt**.

Oracle / lab staged key family:

- key: `mladfmatmulbias_prmptflat_mladf_2x4x4_flat_a16fw4acc16f_1_1152_256_128`
- unpatched body sha16: **`3c5ffca750922d3b`**
- NumOps: **40** · classic header `0001040608010000…`

---

## What was tried

### T2-sibling-keys — dump all DD keys at exactly 1688 B

| Step | Command / action | Result |
|------|------------------|--------|
| 1 | Existing binary `target/dump_dd_txn` against Ryzen AI 1.7.1 DD | OK |
| 2 | Prefix `mladfmatmulbias_prmptflat` → `/tmp/w9a_dd_dump` max=500 | **matches=474** (incl. siblings); **dumped=158 primary** keys |
| 3 | Filter size **exactly 1688** | **56** bins |
| 4 | sha16 + NumOps + `cmp -l` vs lab raw | Table below |
| 5 | Prefix `mladfmatmulbias_gateup` sample (max 30) | No 1688 (bodies ~100k–1M B) |

Research dumps only under **`/tmp/w9a_dd_dump`** (not claimed load-accepted; not copied into `testdata/xdna2_cmd/external_known_good/`).

#### Unique classic 1688 sha16 classes (prmptflat dump)

| sha16 | same_oracle | count | diff_bytes vs lab raw | Role |
|-------|-------------|------:|----------------------:|------|
| **`3c5ffca750922d3b`** | **yes** | 16 | 0 | Unpatched lab family + other dim keys that share **identical** body |
| `34e515248f70b4d1` | no | 10 | 8 | Unpatched **other M/K/N** embeds |
| `24fabc031d0b0db2` | no | 10 | 8 | Unpatched other dims |
| `f50b3084791e3ef5` | no | 7 | 8 | Unpatched other dims |
| `e1ea54f6c22f8660` | no | 6 | 8 | Unpatched other dims |
| `42d539f4695d7831` | no | 3 | 8 | Unpatched other dims |
| `3d264feb4da65913` | no | 2 | 46 | Unpatched other dims |
| `e90176f5be12a624` | no | 1 | 56 | Unpatched other dims |
| `df49d3b08ec260e2` | no | 1 | 56 | Unpatched other dims |

All rows: classic START_NPU, **NumOps=40**, gen4 8×6. Diffs sit in mid/late txn constants (shape-related embeds), **not** a live paddr rela pattern.

**Staged residual key dump:**

```
/tmp/w9a_dd_dump/…_1_1152_256_128.bin
sha16=3c5ffca750922d3b  (= oracle)
```

**In-tree reference (already known, not new host find):**

| Path | sha16 | Note |
|------|-------|------|
| `gt_capture/instr_raw_txn.bin` / vendor txn / lab_instr | `3c5ffca750922d3b` | Unpatched oracle |
| `gt_capture/instr_post_patch.bin` | **`ccab3960f6dd789b`** | Offline pure-direct rela with **synthetic** paddrs; **48** byte diffs vs raw; **not** completing-stack live post-patch |
| `gt_capture/instr_vendor_post_patch.bin` | `3c5ffca750922d3b` | W7/W8 capture: name says post_patch, body **unpatched** (CAPTURE_MODE_UNPATCHED) |

#### Residual-match decision (T2)

| Claim | Verdict |
|-------|---------|
| Non-oracle classic 1688 **exists** in DD map | **Yes** (8 sha classes) |
| Those are **unpatched alternate shapes** | **Yes** |
| Those are **residual-matched post-patch** of lab 1152×256 path | **No** |
| residual_matched_claim | **0** |

Other-shape 1688 siblings are research inventory only. They do **not** close the product residual (need live-patched body of the **same** completing-stack identity, or proven load-accepted peel with ERT+OFM).

---

### T1-run-intercept / T2-ep — private instruction image after execute

| Avenue | Evidence | Wall |
|--------|----------|------|
| Public `instruction_registry::get_instr_bo` | Header: returns cache BO from Transaction/map `put(key)` / `put(key,txn_string)` — **not** live `run_` image | **INSTR_BO_LIMIT** after execute for staged+GM keys (W8 exact strings; not re-proven live this wave to avoid empty reconfirm) |
| `mladfmatmulbias` private `xrt::run run_` | Header private; public: IFM/WTS/OFM/const + `get_transaction_bin` | **No public dump of live-patched instr BO** (`live_instr_bo_public_api=false` W8) |
| `get_transaction_bin` after execute | W8: multi **32208** wrong body (NumOps 500); classic prefer = unpatched 1688 | Multi **refused** residual identity |
| `fusion_rt::get_txns()` | Public fused compile helper — not post-execute paddr-patched classic peel for lab residual | Not residual post-patch path |
| FastFlowLM / Lemonade intercept | Trees **absent** on host | Install wall (operator; **no** silent network install) |
| ORT / VitisAI / Ryzen AI venv | Present under `ryzen-ai-install` venv | No agent-legal private post-patch instr dump API found without DD/XRT source fork or new debug tooling |
| Stock `capture_mladf_instr_vendor` | W8 already: OFM non-zero, classic 1688 = oracle, registry INSTR_BO_LIMIT | **Not re-run** (rules: do not re-capture stock expecting different oracle) |

Exact prior errors (cited, not re-minted):

```
Instruction buffer size for transaction
  mladfmatmulbias_prmptflat_mladf_2x4x4_flat_a16fw4acc16f_1_1152_256_128
  is greater than the INSTR_BO_LIMIT
```

Hardened L3 (`exec_cmd_allow` sysfs): stock param **absent**; sudo password required (W8 A5). Not retried this wave.

---

### T4-host-hunt — read-only demo / patched ctrltext

| Root | Find |
|------|------|
| `/tmp` | W9 dump only (research); no unknown post-patch |
| `~/.cache` | No 1688 instr/ctrltext |
| `ryzen-ai-install` | libtransaction only (map source for dump); no loose post-patch 1688 |
| `RyzenAI-SW` | No classic 1688 |
| `mlir-aie-capture` | Public research peels only (not classic residual) |
| `dragon-npu/testdata` | Known oracle + synthetic `ccab3960…` only |
| FastFlowLM / Lemonade / `/opt/xilinx` | **Absent** |
| `bash tools/host_capture_scan.sh --stdout-only` | roots=8 hits=147 multi=1 research_dup=11; **no residual-matched ingest** |

No new file claimed into `external_known_good/`. No invent bins.

---

## New classic 1688 sha ≠ oracle?

| Question | Answer |
|----------|--------|
| Any classic 1688 sha ≠ oracle found this wave? | **Yes** — **unpatched DD sibling keys** (table above) under `/tmp/w9a_dd_dump` |
| Residual-matched **post-patch** of lab classic? | **No** |
| New sha that closes residual / load-accepted? | **No** |

One-line: **wall** — siblings ≠ post-patch; public live instr dump closed; FLM absent.

---

## Exact walls (summary)

| Wall | Exact fact / error |
|------|--------------------|
| True post-patch classic 1688 | Every **lab-key** dump remains oracle `3c5ffca750922d3b` |
| Registry post-execute dump | INSTR_BO_LIMIT (W8); API = Transaction/cache not live `run_` |
| Multi-txn residual | 32208 refused (`multi_txn_refused`) |
| Sibling 1688 non-oracle | Unpatched other M/K/N — **not** residual identity |
| Synthetic offline patch | `ccab3960f6dd789b` already in-tree; not live completing-stack |
| FLM / Lemonade intercept | Packages **not installed** |
| Hardened L3 | sudo password / param absent on stock |
| Unlock | none active; do **not** mint from this report |

---

## What this is not

- Not Pass / not `ran_on_npu` / not load-accepted
- Not residual_matched
- Not unlock mint / not sticky ALLOW
- Not public AOT re-run
- Not re-live CONSUMED unlocks
- Not claim that non-oracle sibling 1688 can replace lab residual bytes without shape+load evidence

---

## Artifacts

| Path | Role |
|------|------|
| `/tmp/w9a_dd_dump/` | Research: 158 primary prmptflat dumps; 56×1688 |
| `/tmp/w9a_1688_full_table.txt` | Catalog lines (sha/numops/diff/key) |
| `/tmp/w9a_gateup/` | Gateup sample (no 1688) |
| `.agents/reports/wave9-a-bridge-a.md` | This report |
| Prior: `.agents/reports/wave8-material-track.md` | INSTR_BO_LIMIT + capture oracle |

---

## Who acts next

1. **Human / operator** for completing-stack **live-patched** classic 1688 (or residual-shaped peel) with ERT Completed + OFM allclose into `testdata/xdna2_cmd/external_known_good/` — or install FLM/Lemonade if that is the chosen intercept stack.
2. **Agent** idle on empty reconfirm of oracle `3c5ffca750922d3b` and public AOT. Optional later: DD source fork / private BO dump tool (new named code delta), not unlock from this pin alone.
3. **No live** dragon-npu apply; **no unlock mint** from sibling table alone.

---

## Footer

```
invent_pass=0
Pass=No
residual_matched_claim=0
load_accepted_claim=0
unlock=none
new_residual_post_patch_classic_1688=no
non_oracle_unpatched_sibling_1688=yes
```
