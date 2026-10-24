# Wave4 discovery (W4b) — bag score + candidate honesty

**Date:** 2026-08-11
**Scope:** offline scorecard + host inventory soft + journal scrape; no live EXEC
**Honesty:** invent_pass=0 · Pass=**No** · no unlock · residual_matched_claim=0 on all bag rows

## Summary

1. Ran `format_material_exec_readiness_bag_soft_lines` over
   `testdata/xdna2_cmd/external_known_good/` (via unit test
   `material_exec_readiness_w4b_bag_and_candidate_print` + prior print capture).
2. Evaluated host inventory candidate
   `testdata/xdna2_cmd/gt_capture/instr_vendor_post_patch.bin` (32208 multi-txn).
3. **No safe completing capture path without operator.** Did not ingest multi-txn
   as residual_matched / load-accepted. Did not invent PROVENANCE.
4. Soft tools: `host_npu_stack_inventory.sh` exit **0**;
   `journal_npu_scrape.sh --hours 24 --lines 20` exit **0**.

## Bag score table

Capture dir: `testdata/xdna2_cmd/external_known_good/`.
API: `score_material_exec_readiness_from_path` / bag soft lines.
**Every row:** residual_matched_claim=**0**, invent_pass=**0**, product_known_good=**0**,
scorecard_alone_not_pass=**1**.

| bag_file | blob_len | size_class | classic_walk_ok | op_count | distance_to_1688 | residual_matched_claim | detail |
|----------|---------:|------------|-----------------|----------|-----------------:|------------------------|--------|
| mlir_aie_cascade_matmul_npu2.ctrltext.bin | 4512 | other | 0 | 116 | 2824 | **0** | other_size_walk_fail_or_nonclassic |
| mlir_aie_cascade_matmul_npu2.insts.bin | 4512 | other | 0 | 116 | 2824 | **0** | other_size_walk_fail_or_nonclassic |
| mlir_aie_single_core_matmul_npu2.ctrltext.bin | 660 | small_public_npu2 | 0 | 17 | 1028 | **0** | small_public_npu2_research |
| mlir_aie_single_core_matmul_npu2.insts.bin | 660 | small_public_npu2 | 0 | 17 | 1028 | **0** | small_public_npu2_research |
| mlir_aie_vector_scalar_add_npu2.ctrltext.bin | 300 | small_public_npu2 | 0 | 8 | 1388 | **0** | small_public_npu2_research |
| mlir_aie_vector_scalar_add_npu2.insts.bin | 300 | small_public_npu2 | 0 | 8 | 1388 | **0** | small_public_npu2_research |
| mlir_aie_whole_array_matmul_npu2.ctrltext.bin | 2592 | other | 0 | 68 | 904 | **0** | other_size_walk_fail_or_nonclassic |
| mlir_aie_whole_array_matmul_npu2.insts.bin | 2592 | other | 0 | 68 | 904 | **0** | other_size_walk_fail_or_nonclassic |
| working_stack_ctrltext.bin | 300 | small_public_npu2 | 0 | 8 | 1388 | **0** | small_public_npu2_research (alias of vector AOT) |

**Lab oracle (not bag; for distance baseline):** `lab_instr.bin` 1688 ·
size_class=mladf_classic_1688 · classic_walk_ok=1 · op_count=40 · distance=0 ·
residual_matched_claim still **0** (scorecard never invents match).

Public mlir-aie npu2 AOT = **RESEARCH ONLY**. Not residual identity to spent
flexml or lab classic 1688. Not load-accepted.

## Candidates evaluated

| path | bytes | score (raw) | residual_matched_claim | action |
|------|------:|-------------|------------------------|--------|
| `testdata/xdna2_cmd/gt_capture/instr_vendor_post_patch.bin` | 32208 | classic_walk_ok=1, op_count=500, distance_to_1688=30520, size_class=**other**, detail=other_size_classic_walk_ok | **0** | **Refuse ingest.** Historical multi-txn wrong body (HITL false CLASS_HINT C era). |
| same path via `score_material_exec_readiness_from_path` | — | path_refused=1 (gt_capture not bag root) | **0** | Correct confinement: not product known-good path. |

### Candidate handling honesty

- Multi-txn 32208 is **historical wrong-body / multi-class**, not residual-matched classic 1688.
- **Do not** copy into `external_known_good/` as load-accepted.
- **Do not** invent PROVENANCE claiming completed-on-FW / residual match.
- Inventory `who_acts=agent: attempt capture from …instr_vendor_post_patch.bin` is
  **checklist only** — agent evaluation completed; safe capture path **not** found.

## Optional capture attempt

**Result:** no safe completing capture path without operator.

- Ryzen AI / install roots present (inventory: 2 roots, xrt-smi, xclbinutil).
- Live EP / EXEC would need L3 `exec_cmd_allow` + ALLOW double gate — **not opened**.
- No new bytes produced. Prefer soft refuse over fake ingest.

Tool: `tools/capture_from_host_candidate.sh` (W4b T7 fail-closed).

```text
bash tools/capture_from_host_candidate.sh
# → exit 2 multi_txn_refused=1 residual_matched_claim=0 invent_pass=0
bash -n tools/capture_from_host_candidate.sh  # clean
```

## Host soft probes (this turn)

| command | exit |
|---------|-----:|
| `bash tools/host_npu_stack_inventory.sh` | **0** |
| `bash tools/journal_npu_scrape.sh --hours 24 --lines 20` | **0** |

Journal soft (24h, last 20 of 48 matches): IO_PAGE_FAULT ~`0x8fff…`,
`aie2_set_cmd_timeout` (TXN OP `0xd`), `Get bo 2 failed` (debug-BO research).
Soft tokens ≠ residual root; journal_scrape_alone_not_pass=1.

Inventory: accel=1, amdxdna present, candidate_n=1 (multi-txn only),
bag_files_n=25, exec_cmd_allow=missing, who_acts still names multi-txn path
(operator material ask remains valid).

## blocked_on_host_stack / operator material

| flag | value | meaning |
|------|------:|---------|
| blocked_on_host_stack | **0** | Host has accel + tools + firmware; not a missing-stack host block |
| operator material ask still needed | **yes** | Need true load-accepted / residual-matched peel under `external_known_good/` (or path + `--ingest`). Public AOT does **not** count. Multi-txn does **not** count. |

## Unit surface (W4b)

- Test: `material_exec_readiness_w4b_bag_and_candidate_print`
  bag soft honesty + multi-txn 32208 → Other, claim=0.
- Filter: `cargo test --features amd-xdna2 --lib material_exec_readiness` → **2 ok**.

## Honesty footer

- invent_pass=**0**
- Pass=**No**
- not_ran_on_npu=**1**
- not_c_ready=**1**
- residual_matched_claim=**0** (all bag rows + candidate)
- No unlock mint · no CONSUMED re-live · no ALLOW · no EXEC
- Scorecard / discovery alone ≠ Pass ≠ load-accepted

## Join prep (for next wave)

- Bag scored; research AOT present; residual gap = **matched material**, not host stack.
- Candidate multi-txn closed as refuse.
- Operator one ask unchanged: load-accepted capture →
  `testdata/xdna2_cmd/external_known_good/` (or path + `--ingest`).
- Live only under **new** unlock + named offline reason.

## Files

- `.agents/reports/wave4-discovery.md` (this report)
- `tools/capture_from_host_candidate.sh` (fail-closed T7)
- `src/backends/amd_xdna2/silicon_contract.rs` (W4b unit test)
- Prior: `.agents/reports/wave4-host-inventory.md`, `wave4-scorecard-tools.md`
