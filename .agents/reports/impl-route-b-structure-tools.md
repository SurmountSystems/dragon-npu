# Impl report: Route B offline structural classifiers

**Date:** 2026-08-10
**Role:** L2-B implementer
**Plan:** Route B offline structural tools (explore inventory
`.agents/reports/plan-explore-route-b-structure-tools.md`)
**Scope:** pure offline TDD in `silicon_contract.rs` + re-exports + soft PRINT in
`examples/classify_last_apply.rs`
**Honesty footer:** invent_pass=0 · not_c_ready=1 · not_ran_on_npu=1 ·
product_known_good=0 · **Pass=No** · **unlock=none** · distance/match ≠ Pass

---

## Goal (done)

Stronger pure offline structural tools so residual can cite multi-axis distance
(size class, opcode hist, header topology), not only "300 B vs 1688 B".

Did **not**: edit capture dir bins/PROVENANCE; mint unlock; invent Pass; live
apply; rewrite tests to fake green.

---

## Delivered slices

| Slice | Status | What landed |
|-------|--------|-------------|
| **B2** | done first | `CtrltextSizeClass` + `research_external_vs_lab_structural_distance` multi-axis distance |
| **B1** | done | `TxnOpOpcodeProfile` / per-side txn header on `ExternalCtrltextCompareResearch` soft note |
| **B3** | done | `fingerprint_capture_material_dir_research` multi-file bag fingerprint |
| **B4** | done (cheap) | `compare_rela_structure_lab_vs_external_elf` first-N offsets + sym multiset |

Also: `TxnOpOpcodeProfile::format_soft_tokens_with_prefix` / `l1_distance`;
`CtrltextFingerprintExport.op_profile` filled from peel; classify example soft
prints under existing PRINT path.

---

## APIs

Primary file: `src/backends/amd_xdna2/silicon_contract.rs`
Re-exports: `src/backends/amd_xdna2.rs`

### B2 — structural distance + size class

| Symbol | Role |
|--------|------|
| `CtrltextSizeClass` | `empty` / `mladf_classic_1688` / `small_public_npu2` / `other` |
| `CTRLTEXT_SMALL_PUBLIC_NPU2_MAX_BYTES` | 512 (band for public npu2 minimal) |
| `classify_ctrltext_size_class(len, profile?)` | length + staged profile → class |
| `classify_ctrltext_size_class_from_export(lab)` | lab export → class |
| `ExternalVsLabStructuralDistance` | size class, classic_both, `op_hist_l1`, header abs diffs, sha/len |
| `research_external_vs_lab_structural_distance(lab, external)` | multi-axis distance |
| `EXTERNAL_VS_LAB_STRUCTURAL_DISTANCE_NOTE` | soft bag name |
| soft always | `distance_alone_not_pass=1 invent_pass=0 not_c_ready=1 not_ran_on_npu=1 product_known_good=0` |

Details tokens include `size_class_mismatch_distance`,
`structural_distance_zero_self_match`, `op_hist_l1_nonzero_distance`, etc.

### B1 — compare soft note surfaces opcode + header

| Extension | Role |
|-----------|------|
| `CtrltextFingerprintExport.op_profile` | filled via `profile_instr_payload_bytes` |
| `ExternalCtrltextCompareResearch.lab_op_profile` / `external_op_profile` | per-side histograms |
| `lab_txn_*` / `external_txn_*` soft tokens | num_ops / dev_gen / cols / rows when present |
| `TxnOpOpcodeProfile::format_soft_tokens_with_prefix("lab_"\|"external_")` | prefixed `op_*` |
| `TxnOpOpcodeProfile::l1_distance` | L1 over opcode bins |

### B3 — capture material dir fingerprint

| Symbol | Role |
|--------|------|
| `CaptureMaterialFingerprint` | bin/elf/xclbin counts, first ctrltext size class + classic, magics |
| `fingerprint_capture_material_dir_research(root)` | confined to temp ∪ `external_known_good/` |
| `CAPTURE_MATERIAL_FINGERPRINT_NOTE` | soft bag name |
| soft always | `capture_material_alone_not_pass=1 invent_pass=0 … product_known_good=0` |
| empty / no ctrltext | `blocked_on_external_bytes=1` |

Helpers: `capture_material_root_is_allowed`, `scan_capture_material_dir_entries`,
`capture_material_blocked` (private).

### B4 — rela structure ELF vs ELF

| Symbol | Role |
|--------|------|
| `RelaStructureCompareResearch` | count / first-N offsets / sym multiset |
| `RELA_STRUCTURE_COMPARE_FIRST_N` | 8 |
| `compare_rela_structure_lab_vs_external_elf(lab, external?)` | pure structure |
| `RELA_STRUCTURE_COMPARE_NOTE` | soft bag name |
| soft always | `rela_structure_alone_not_pass=1 invent_pass=0 …` |
| match detail | `rela_structure_match_neq_pass` |

### Soft wire (classify example)

`examples/classify_last_apply.rs` (under existing PRINT / external path):

- `external_vs_lab_structural_distance=…` when external bytes present
- `capture_material_fingerprint=…` for bag root

Never Pass / unlock / live device.

---

## Tests (red → green)

Named contracts under `silicon_contract::tests` (feature `amd-xdna2`).

### B2 filters (`structural_distance`)

| Test | Contract |
|------|----------|
| `structural_distance_size_class_labels` | empty / 1688 / 300 / 512 / 513 / staged force |
| `structural_distance_300_vs_lab_export_shape` | small_public_npu2 vs mladf_classic_1688; size_class_match=0; honesty tokens |
| `structural_distance_self_match_zero_still_invent_pass_0` | self peel: op_hist_l1=0 still invent_pass=0; mutated op L1>0 |

### B1 filters (`external_ctrltext_compare`)

| Test | Contract |
|------|----------|
| `external_ctrltext_compare_surfaces_opcode_histogram_when_classic` | lab_/external_ op tokens + txn headers on soft note; match ≠ invent_pass |
| (existing 8 dry-run / honesty / refuse tests still green) | |

### B3 filters (`capture_material`)

| Test | Contract |
|------|----------|
| `capture_material_fingerprint_empty_blocked` | empty confined dir → blocked_on_external_bytes |
| `capture_material_fingerprint_ctrltext_only_no_pass` | present bin classifies; product_known_good=0 |

### B4 filters (`rela_structure`)

| Test | Contract |
|------|----------|
| `rela_structure_compare_synthetic_match_neq_pass` | synthetic match still invent_pass=0 |

### Observed commands (green after product edit)

```text
cargo test --features amd-xdna2 --lib structural_distance
  → 3 passed

cargo test --features amd-xdna2 --lib external_ctrltext_compare
  → 9 passed (incl. surfaces_opcode_histogram)

cargo test --features amd-xdna2 --lib capture_material
  → 2 passed

cargo test --features amd-xdna2 --lib rela_structure
  → 1 passed

cargo test --features amd-xdna2 --lib fingerprint_export
  → 5 passed
```

TDD order: named red contracts for size-class / distance / opcode surface /
capture empty / rela match-neq-pass, then product APIs, then same filters green.
No assert rewrite to invent Pass.

---

## Gates

```text
cargo fmt -- --check          → clean (EXIT 0)
cargo clippy --features amd-xdna2 --lib -- -D warnings  → clean (EXIT 0)
```

Clippy notes fixed during impl: `too_many_lines` split into
`scan_capture_material_dir_entries`; `manual_let_else`; `or_fun_call` →
`unwrap_or_else`; redundant `Elf32Rela::sym` closure → method map.

---

## Residual impact (how residual can cite distance)

Before Route B: soft compare for 300 vs 1688 mostly collapsed to
`external_differs_from_lab` + len mismatch.

After Route B residual / status can cite e.g.:

- `external_size_class=small_public_npu2 lab_size_class=mladf_classic_1688 size_class_match=0`
- `op_hist_l1=… op_hist_both=…` when both classic
- `header_cols_diff` / `header_rows_diff` / `header_ops_diff` / `header_dev_gen_diff`
- bag: `bin_count` / `first_ctrltext_size_class` / `first_classic_walk_ok`
- still always: invent_pass=0, distance_alone_not_pass=1, product_known_good=0

Material under `testdata/xdna2_cmd/external_known_good/` (Route A) **not**
modified by this slice. Soft compare still **expected mismatch** for public
mlir-aie 300 B vs lab classic 1688 B.

---

## Honesty footer (mandatory)

| Claim | Value |
|-------|--------|
| invent_pass | **0** (hard-coded on all new soft notes) |
| not_c_ready | **1** |
| not_ran_on_npu | **1** |
| product_known_good | **0** |
| product Pass | **No** |
| C_UNLOCK | **none active** (no mint; do not reopen CONSUMED) |
| distance / structural_match / rela match | **≠** Pass |
| capture material fingerprint | **≠** Pass / ≠ C-ready |
| live apply / ALLOW / L3 | **not** done |

---

## Files touched

| Path | Change |
|------|--------|
| `src/backends/amd_xdna2/silicon_contract.rs` | B1–B4 product + unit tests |
| `src/backends/amd_xdna2.rs` | public re-exports |
| `examples/classify_last_apply.rs` | soft PRINT distance + bag fingerprint |
| `.agents/reports/impl-route-b-structure-tools.md` | this report |

---

## Not done / out of scope

- Capture bin / PROVENANCE edits (Route A)
- Live CONFIG_CU / unlock / Pass decide
- Auto wire into `pass_unblock_once --agent-offline` beyond classify PRINT (optional follow-up; not required for B0–B6 offline tools)
- Residual.md re-pin (parent / residual honesty owner can cite new soft tokens when next pin lands)

---

## Next (agent-legal only)

1. Residual prose may cite multi-axis distance tokens when material discuss returns.
2. Optional: one line in operator `--compare` path already hits classify PRINT.
3. Still no unlock from this report alone. Human material / unspent topology / new unlock reason only for any future live shot.
