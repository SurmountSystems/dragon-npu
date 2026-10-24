# Wave6-2 — non-public residual-identity material hunt

**Date:** 2026-08-11
**Scope:** Offline / read-only host hunt for non-public program bytes that could
identity-match or bridge lab classic **1688** (mladf-matmul-bias / DD / XRT
post-patch path).
**Not done:** public mlir-aie AOT re-run; L3 open; ALLOW; C_UNLOCK mint; network
install; invent bins; re-tag research_dup as residual match.

| Honesty field | Value |
|---------------|--------|
| invent_pass | **0** |
| residual_matched_claim | **0** |
| load_accepted_claim | **0** |
| product_known_good | **0** |
| Pass | **No** |
| not_ran_on_npu | **1** |
| not_c_ready | **1** |
| L1 unlock justified | **No** |
| New sha ingested | **none** |

Related: `RESIDUAL.md` top (W5 Path B all four public peels PASS research-only);
`.agents/reports/wave5-host-scan-agent.md`; `wave3-material-hunt.md`;
`tools/host_capture_scan.sh`; `tools/collect_load_accepted_candidate.sh`.

---

## 1. What ran

| Step | Result |
|------|--------|
| `bash tools/host_capture_scan.sh --stdout-only` | 8 roots present; **143** hits; multi_txn=**1**; research_dup=**11**; large_soft=2 |
| In-tree 1688 identity compare | raw vs offline post_patch **DIFF** (48 bytes); lab raw == DD dump == vendor_txn_bin |
| Host find size=1688 + name patterns | No new non-public classic ctrltext outside tree |
| Dry-run `collect_load_accepted_candidate.sh` | All residual candidates refused (see §4) |
| `capture_mladf_instr_vendor` live | **Not run** (needs L3; banned this wave) |
| Public mlir-aie AOT re-run | **Not run** (already PASS on FW 1.1.2.64) |
| Ingest | **none** |

---

## 2. Paths examined

### Present roots

| Path | Role / find |
|------|-------------|
| `/tmp` | Junk `instr_post_patch.bin` size=2; prior live-shot logs/last_apply; CVML PDI dumps; **no** new 1688 post-patch |
| `~/.cache` | Mesa/nix/Brave noise; uv onnxruntime wheel cache; **no** DD instruction cache of classic txn |
| `/home/hunter/Projects/ai/ryzen-ai-install` | `libtransaction.a` (DD + voe); phoenix xclbins; aiebu preempt/copy helpers; **no** loose 1688 post-patch |
| `/home/hunter/Projects/ai/RyzenAI-SW` | CVML linux/windows **overlay** xclbins (0.46–6+ MiB); **no** classic 1688 ctrltext |
| `/home/hunter/Projects/ai/mlir-aie-capture` | Public AOT + `completed_run_*` Path B peels (research_dup shas only) |
| dragon-npu `testdata/xdna2_cmd/gt_capture` | Offline GT + vendor capture (primary non-public surface) |
| dragon-npu `testdata/xdna2_cmd/external_known_good` | Public mlir-aie research peels only |
| dragon-npu `testdata/xdna2_cmd/silicon_lab` + `dd_matmul_txn` | Offline DD classic dumps (unpatched) |

### Absent

| Path | Note |
|------|------|
| FastFlowLM / Lemonade trees | Still missing (Path A install not present) |
| `/opt/xilinx`, `/opt/amd/ryzen-ai` | Missing |
| Live XRT/DD **post-patch** BO dump at residual identity | Still missing (true gap) |

---

## 3. Lab classic 1688 family (in-tree, non-public)

All classic START_NPU txn headers `0001040608010000` (or close). **Size class =
mladf-classic-1688** for body length; identity is by sha.

| Stream | size | sha16 | Provenance | Residual match? | Load-accepted? |
|--------|------|-------|------------|-----------------|----------------|
| `lab_instr.bin` / `silicon_lab/lab_instr.bin` / `gt_capture/instr_raw_txn.bin` / `instr_vendor_txn_bin.bin` / DD dump key `…_1_1152_256_128` | 1688 | **`3c5ffca750922d3b`** | Unpatched libtransaction / DD map extract | Size-class only; **not** post-patch live | **No** |
| `gt_capture/instr_post_patch.bin` | 1688 | **`ccab3960f6dd789b`** | Offline pure-direct rela patch with **synthetic** paddrs (`0xa11f…`) — `dump_mladf_gt_offline` | **Oracle body**, not completing-stack live capture | **No** (basename refuse + no ERT/OFM evidence) |
| Other DD keys at 1688 B (`…_1024_1152_128`, `…_11008_2048_128`, `…_1152_1280_128`, …) | 1688 | `34e515248f70b4d1`, `42d539f4695d7831`, … | Unpatched DD dumps for other M/K/N | Other classic keys; **not** lab residual identity | **No** |
| `gt_capture/instr_vendor_post_patch.bin` | **32208** | `3de5f68d14ff1773` | HITL get_transaction_bin multi-txn wrong body (name misleading; content unpatched multi) | **Refused** multi_txn | **No** |
| `gt_capture/ctrl_post_patch.bin` | 29664 | `18d4b7c0c46417c5` | Offline ctrl meta patch | Ctrl stream, not classic instr | **No** |
| `gt_capture/instr_pre.elf` / `silicon_lab/lab_instr.elf` | 32264 | `6ed93809133413fa` | aiebu ELF + rela×12 around classic 1688 ctrltext | Userspace ELF path already known; ≠ load-accepted | **No** |
| Root `lab_instr.elf` | 1072 | `6b46469491781d50` | Stale/small research ELF (not the 32264 staged aiebu) | Not residual bridge | **No** |

**raw vs offline post_patch:** same length 1688; **48 byte diffs** at rela patch
sites. That offline post_patch is the pure-direct GT oracle for lab classic —
it is **already in-tree** under `gt_capture/` and is **not** a new host find.
It is **not** a live XRT/DD registry post-patch from a job that completed with
ERT Completed + OFM allclose on this firmware.

**Compare to public research peels (do not re-tag):**

| Public peel | size | sha16 | Class |
|-------------|------|-------|--------|
| vector / working_stack_ctrltext | 300 | `aaeb5b68cc298a45` | research_duplicate |
| single_core matmul | 660 | `de8671245bb72436` | research_duplicate |
| whole_array | 2592 | `7ee4b267ce31cd70` | research_duplicate |
| cascade | 4512 | `18cb653c05a9d4c8` | research_duplicate |

None equal `3c5ffca750922d3b` or `ccab3960f6dd789b`. Public AOT **≠** lab classic 1688.

---

## 4. Candidates + refuse reasons (dry-run collect)

| Candidate | size | sha16 | Outcome |
|-----------|------|-------|---------|
| `gt_capture/instr_post_patch.bin` | 1688 | `ccab3960f6dd789b` | **REFUSE** basename local oracle; completing_stack_evidence_ok=0; residual_matched_claim=0 |
| `gt_capture/instr_raw_txn.bin` | 1688 | `3c5ffca750922d3b` | **REFUSE** basename; evidence_ok=0 |
| `gt_capture/instr_vendor_post_patch.bin` | 32208 | `3de5f68d14ff1773` | **REFUSE** multi_txn + basename |
| `gt_capture/ctrl_post_patch.bin` | 29664 | `18d4b7c0c46417c5` | **REFUSE** basename; not 1688 |
| `lab_instr.elf` (root 1072) | 1072 | `6b46469491781d50` | **REFUSE** basename |
| `/tmp/instr_post_patch.bin` | 2 | `73cb3858a687a849` | **REFUSE** basename + junk size |
| CVML `AMD_AIE2P_4x4_Overlay.xclbin` (linux) | 1716843 | (large) | classify dry-run only; evidence_ok=0; **not** classic 1688; **not** residual match; **not** ingested |
| aiebu `copy_DDR_to_Mem_Tile.bin` | (helper) | — | dry-run only; not residual; not ingested |
| Public ctrltext 300/660/2592/4512 | as above | known research | **REFUSE** research_duplicate_sha16 (not re-run AOT) |

**Ingest decision:** no path met fail-closed ingest for residual identity.
**New sha ingested: none.**

---

## 5. Host non-public avenues (honest residual value)

| Avenue | Status | Residual value for classic 1688 |
|--------|--------|----------------------------------|
| Offline pure-direct post_patch GT | **Present** (`ccab3960…`) | Already known oracle; synthetic paddrs; ≠ load-accepted; ≠ invent Pass |
| Unpatched DD classic dumps | **Present** (`3c5ffca7…` + sibling keys) | Size-class material for offline tools; already used as lab body |
| Multi-txn 32208 vendor dump | Present, **refused** | Wrong body class; do not re-live as residual match |
| Live `registry_get_instr_bo` / true post-patch | **Missing** | Highest residual gap (prior capture-instr only got multi or unpatched map) |
| `capture_mladf_instr_vendor` re-run | Not run (L3) | Would need operator L3 + unlock policy; prior path produced multi 32208 |
| FastFlowLM / Lemonade intercept | Trees **absent** | Install-only; no agent network install |
| CVML AIE2P overlays | Present large xclbin/PDI | CONFIG_CU research; spent flexml/CVML PDIs already exercised live under CONSUMED unlocks; **≠** 1688 ctrltext identity |
| libtransaction.a | Present (~321 MiB DD) | Source of offline dumps already extracted; no new post-patch |
| Path B completed_run_* | Present | Public research peels only (IRON PASS ≠ residual match) |

---

## 6. Residual-matched claim (honest)

**residual_matched_claim=0**

Rationale in plain English:

1. Lab residual identity for the product path is **classic 1688** with
   rela-patched device paddrs from a **working** stack (ERT Completed + useful
   OFM), not public IRON AOT and not multi-txn 32208.
2. The only in-tree 1688 post-patch body is the **offline pure-direct oracle**
   with synthetic paddrs. That is research/GT material already under
   `gt_capture/`. Helpers correctly refuse it as load-accepted / residual match
   without completing-stack evidence.
3. Host scan found **no new** non-public post-patch 1688 outside that oracle.
4. Public mlir-aie peels (300/660/2592/4512) remain research_duplicate relative
   to residual identity even after Path B IRON PASS on FW 1.1.2.64.

---

## 7. L1 unlock justified?

**No.**

Unlock requires a **new** host-minted `C_UNLOCK` **and** a named offline reason
that is not empty reconfirm. This hunt adds **no** residual-matched material
and **no** load-accepted capture. Do **not** mint. First residual
`C_UNLOCK=` remains **`4008e6fd` CONSUMED** (do not reopen).

---

## 8. Who acts next

1. **Highest residual-useful:** operator (or supervised L3) completing-stack
   capture of **true post-patch classic 1688** (or aiebu ELF ctrltext after
   live rela) into `testdata/xdna2_cmd/external_known_good/` with load-accepted
   PROVENANCE only when ERT Completed + OFM allclose / `decide_ran_on_npu`.
2. Optional: re-run `capture-instr` only under explicit operator L3 + policy —
   prefer classic 1688 map, refuse multi 32208 as primary.
3. **Do not:** re-run public AOT; re-ingest research_dup; re-live spent flexml
   under CONSUMED unlocks; invent Pass; empty reconfirm mill.

---

## Footer

```
invent_pass=0
not_ran_on_npu=1
not_c_ready=1
Pass=No
load_accepted_claim=0
residual_matched_claim=0
product_known_good=0
new_sha_ingested=none
l1_unlock_justified=no
multi_txn_refused=1
research_dup_not_retagged=1
public_aot_rerun=0
entrypoint=tools/host_capture_scan.sh + tools/collect_load_accepted_candidate.sh --dry-run
report=.agents/reports/wave6-material-hunt.md
```
