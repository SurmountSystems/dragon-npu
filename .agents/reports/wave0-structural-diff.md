# Wave W0 O4 — Structural / byte axes on present dual (+ third) capture

**Date:** 2026-08-11
**Mode:** Offline only. Pure analysis + Route B policy/API surface. **invent_pass=0**.
**Pass=No.** Distance ≠ match ≠ residual_matched ≠ unlock alone.

---

## Goal

Document size-class / opcode / header / section / rela axes for present public
material under `testdata/xdna2_cmd/external_known_good/` vs lab
`mladf-classic-1688` and spent `flexml-2x4x4-class` PDIs.

---

## Present material inventory

Primary soft-compare ctrltext remains **vector 300 B**
(`working_stack_ctrltext.bin` = vector peel). Dual research set = 300 + 660.
Third set (whole_array 2592) also present; included for axes completeness.

| Stream | ctrltext (B) | sha16 (ctrltext) | class tag | rela (aiebu ELF) | PDI (B) | PDI sha16 |
|--------|-------------:|------------------|-----------|-----------------:|--------:|-----------|
| vector_scalar_add | **300** | `aaeb5b68cc298a45` | `public-npu2-small` | **1** | 2528 | `fe438059f5d0e4eb` |
| single_core matmul | **660** | `de8671245bb72436` | `public-npu2-small` | **5** | 5536 | `cd3d9fa6577d1e37` |
| whole_array matmul | **2592** | `7ee4b267ce31cd70` | (larger public AOT) | **20** | 66864 | `e721b80335e6d8ed` |
| lab classic (oracle) | **1688** | (silicon_lab unpatched) | `mladf-classic-1688` | **12** | n/a (optional research blob) | — |
| flexml unified-2x4x4 | n/a | n/a | `flexml-2x4x4-class` **Spent** | n/a | **27200** | `0eeeb70c84df056e` |
| flexml llm-2x4x4 | n/a | n/a | `flexml-2x4x4-class` Spent | n/a | **21664** | `ba0cd738fd870011` |
| flexml unified-4x4 | n/a | n/a | `flexml-2x4x4-class` Spent | n/a | **13792** | `7d172bf223a9b06b` |

Sources: `PROVENANCE.md` tables; aiebu reports in Route A / whole_array reports
(`.agents/reports/impl-route-a-matmul-material.md`,
`impl-feedback-larger-matmul-material.md`).

**insts.bin vs ctrltext peel:**

| Stream | same length? | same sha? |
|--------|--------------|-----------|
| vector 300 | yes | **yes** (byte-identical) |
| single_core 660 | yes | **no** (aiebu normalizes one header field; peel preferred) |
| whole_array 2592 | yes | **no** (same normalize class) |

---

## Size-class axis (`CtrltextSizeClass`)

Policy (`silicon_contract.rs`):

| Rule | Class |
|------|-------|
| len == 0 | `empty` |
| len == 1688 or staged_mladf profile | `mladf_classic_1688` |
| 1..=1024 (`CTRLTEXT_SMALL_PUBLIC_NPU2_MAX_BYTES`) | `small_public_npu2` |
| else | `other` |

| Stream | len | size class | size_class_match vs lab |
|--------|----:|------------|-------------------------|
| lab | 1688 | `mladf_classic_1688` | — |
| vector | 300 | `small_public_npu2` | **0** |
| single_core | 660 | `small_public_npu2` | **0** |
| whole_array | 2592 | `other` | **0** |

Material inventory: 300 and 660 →
`MaterialResidualMatchKind::PublicNpu2SmallResearchOnly` with
**`residual_matched_claim=0`** always offline. whole_array → length `other`
(not residual match; not load-accepted claim).

---

## Header / topology axis (classic XAie v0.1)

Lab staged classic profile (contract constants):

| Field | Lab classic |
|-------|-------------|
| txn_size | **1688** |
| num_ops | **40** |
| dev_gen | **4** (typical Strix class) |
| cols × rows | **8 × 6** |
| staged_mladf_classic_profile | **true** |

Public aiebu reports (PROVENANCE / build reports):

| Stream | txn_size | num_ops | gen | cols × rows | header_ops_diff vs lab | header_cols_diff vs lab |
|--------|----------|---------|-----|-------------|------------------------|-------------------------|
| single_core | 660 | **17** | 4 | **1 × 6** | \|17−40\|=**23** | \|1−8\|=**7** |
| whole_array | 2592 | **68** | 4 | **8 × 6** | \|68−40\|=**28** | **0** |
| vector | 300 | (not logged in PROVENANCE) | aiebu wrap | classic ELF | size-class mismatch dominates | size-class mismatch dominates |

**Note:** whole_array shares **8×6** grid with lab but **not** ops/size/body —
topology cols/rows alone ≠ residual identity.

---

## Opcode histogram axis

Route B: `TxnOpOpcodeProfile` + L1 distance when both sides classic-walk OK
(`research_external_vs_lab_structural_distance` → `op_hist_l1` / `op_hist_both`).

| Expectation (offline) | Reason |
|-----------------------|--------|
| Lab ops total 40 (staged) | contract lock |
| single_core ops total 17 | aiebu report |
| whole_array ops total 68 | aiebu report |
| `op_hist_l1` vs lab | **non-zero** whenever both walk (different programs) |
| `detail` for 300/660 vs lab export | typically **`size_class_mismatch_distance`** first |

Unit contract `structural_distance_300_vs_lab_export_shape`: synthetic 300 vs
lab-shaped export → `size_class_match=0`, `sha_match=0`, `len_match=0`,
`detail=size_class_mismatch_distance`, soft always
`distance_alone_not_pass=1 invent_pass=0`.

Real bag peels are same length classes as those unit locks (300 / 660).

---

## Rela structure axis

`compare_rela_structure_lab_vs_external_elf` (count + first-N offsets + sym multiset).

| Side | rela_count | vs lab 12 |
|------|------------|-----------|
| lab `lab_instr.elf` | **12** (IFM×8, WTS×2, OFM×2; sym 1/2/3) | self |
| vector `insts.elf` | **1** | count_match=**0** |
| single_core `insts.elf` | **5** | count_match=**0** |
| whole_array `insts.elf` | **20** | count_match=**0** |

Expected soft detail: `rela_count_mismatch` (unless comparing same ELF).
**Rela structure match ≠ Pass** (`rela_structure_alone_not_pass=1`).

---

## Section / xclbin axis (headers only)

Public bags include `.xclbin` siblings:

| Stream | xclbin (B) | Research use |
|--------|------------|--------------|
| vector | 8983 | section name/offset/body-bounds dry-run |
| single_core | 11993 | same |
| whole_array | 73327 | same (larger partition payload class) |

Hard locks (existing TDD):
`section_identity_alone_closes_fw_residual=0`,
`section_offset_map_alone_closes_fw_residual=0`,
`body_bounds_alone_closes_fw_residual=0`,
`public_pdi_image_decode=0`.
No private PDI field invent from this wave.

---

## PDI geometry vs spent flexml

`compare_pdi_geometry_to_spent_flexml` / size-class tags:

| Bag PDI | bytes | vs spent 27200 / 21664 / 13792 |
|---------|------:|--------------------------------|
| vector | 2528 | **Diff** (size class ≠ spent flexml) |
| single_core | 5536 | **Diff** |
| whole_array | 66864 | **Diff** (larger; still ≠ spent sha) |

Even synthetic size-match with wrong sha locks
`spent_rematch_not_unlock_alone=1 invent_pass=0`. Bag PDIs are **not** spent
rematch and **not** product known-good.

---

## Distance summary (vs lab classic 1688)

| External | size_class_match | len_match | sha_match | classic topology | rela count match | residual_matched_claim | invent_pass |
|----------|------------------|-----------|-----------|------------------|------------------|------------------------|-------------|
| vector 300 | 0 | 0 | 0 | not lab staged | 0 (1≠12) | **0** | **0** |
| single_core 660 | 0 | 0 | 0 | 1×6 ops17 ≠ 8×6 ops40 | 0 (5≠12) | **0** | **0** |
| whole_array 2592 | 0 (`other`) | 0 | 0 | 8×6 match cols/rows only; ops68≠40 | 0 (20≠12) | **0** | **0** |

**One line:** public dual (+ third) material is **present and classifiable**;
**structurally distant** from lab classic 1688 and **size-distant** from spent
flexml PDIs. **Not** residual identity. **Not** Pass.

Soft note always (Route B):
`distance_alone_not_pass=1 invent_pass=0 not_c_ready=1 not_ran_on_npu=1 product_known_good=0`.

---

## Tools used / available (no live)

| API / note | Role |
|------------|------|
| `classify_ctrltext_size_class` | size class |
| `research_external_vs_lab_structural_distance` | multi-axis distance soft note |
| `compare_external_ctrltext_to_lab_export` | soft sha/len/structural |
| `compare_rela_structure_lab_vs_external_elf` | rela counts / offsets / syms |
| `fingerprint_capture_material_dir_research` | multi-file bag fingerprint |
| `classify_material_residual_match` | inventory; never offline residual_matched |
| `fingerprint_pdi_geometry_bytes` / `compare_pdi_geometry_to_spent_flexml` | PDI vs spent |

Re-run soft path (operator optional):

```bash
export XDNA2_LAB_PRINT_C2_CMDBUF_RESEARCH=1
export XDNA2_LAB_EXTERNAL_CTRLTEXT="$PWD/testdata/xdna2_cmd/external_known_good/working_stack_ctrltext.bin"
cargo run --features amd-xdna2 --example classify_last_apply --quiet -- \
  testdata/xdna2_cmd/last_apply.json
# then same with mlir_aie_single_core_matmul_npu2.ctrltext.bin
unset XDNA2_LAB_EXTERNAL_CTRLTEXT
```

---

## What this is not

- Not Pass / not `ran_on_npu` / not residual_matched
- Not unlock mint / not live apply
- Not invent of op-histogram bin values for vector (PROVENANCE lacked NumOps)
- Not claim whole_array closes residual because 8×6 matches lab cols/rows

---

## Files touched

| Path | Change |
|------|--------|
| `.agents/reports/wave0-structural-diff.md` | This report |
| (product code) | **none** |

Construction map axes table cross-links this report (O3 map edit).

---

## Honesty lock

| Token | Value |
|-------|-------|
| invent_pass | **0** |
| residual_matched_claim | **0** (offline hard lock) |
| product Pass | **No** |
| ran_on_npu | **not claimed** |
| distance_alone_not_pass | **1** |
| unlock | **none active** |
