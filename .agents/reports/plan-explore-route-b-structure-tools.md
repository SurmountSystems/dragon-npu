# Plan explore: Route B = stronger offline structural classifiers

**Date:** 2026-08-10
**Scope:** Read-only inventory of offline compare/classify APIs for capture material.
**Honesty:** invent_pass=0; material ≠ Pass ≠ C-ready ≠ `ran_on_npu`. No unlock mint.

---

## 1. Current offline compare / classify APIs

Primary SSoT: `src/backends/amd_xdna2/silicon_contract.rs`
Public re-exports: `src/backends/amd_xdna2.rs` (`pub use silicon_contract::{…}`). Soft print surface: `examples/classify_last_apply.rs` under `XDNA2_LAB_PRINT_C2_CMDBUF_RESEARCH=1`.

### 1a. external_known_good / ctrltext compare

| API | Role |
|-----|------|
| `external_known_good_path_is_allowed` / `_load_path_is_allowed` | Confinement: `temp_dir` ∪ `testdata/xdna2_cmd/external_known_good/`; `.bin`/`.ctrltext`; refuse local post_patch basenames |
| `load_external_ctrltext_for_compare` / `load_external_ctrltext_from_env` | Size-capped load (`XDNA2_LAB_EXTERNAL_CTRLTEXT`) |
| `CtrltextFingerprintExport` / `export_ctrltext_fingerprint_research(elf)` | Lab unpatched peel identity: len, rela_count, classic_walk_ok, staged_mladf, txn header fields, unpatched sha256. **Always** `known_good_status=absent_awaiting_capture` |
| `ExternalCtrltextCompareResearch` / `compare_external_ctrltext_to_lab_export(lab, external)` | Soft sha / len / structural vs lab export |
| `ExternalCtrltextCompareDryRun` / `research_external_ctrltext_compare_dry_run*` | Bag may be absent → explicit `blocked_on_external_bytes=1` |
| `LabExportFingerprintFeed` / `research_lab_export_fingerprint_feed*` | Auto-feed lab export into dry-run |
| `AppLoadBagFeedDryRunScorecard` / `research_app_load_bag_feed_dry_run_scorecard*` | Composition bag + feed + dry-run |
| `AppLoadExtractBagScorecardPipeline` / `research_app_load_extract_bag_scorecard_pipeline*` | Extract markers + bag + scorecard |
| `check_external_known_good_bag_drop` / PROVENANCE validators | Fail-closed bag-drop completeness |
| `external_compare_status_from_surfaces` | `absent \| loaded \| match \| mismatch \| refused` for scorecard |
| `AppLoadOfflineProgressScorecard` / `research_app_load_offline_progress_scorecard*` | Composes class + bag + e2e + compare status |

Soft note names: `ctrltext_fingerprint_export=`, `external_ctrltext_compare=`, `external_ctrltext_compare_dry_run=`, etc. Always honesty trio + `product_known_good=0` even when research external present.

### 1b. Construction path / fingerprint

| API | Role |
|-----|------|
| `ConstructionPathResearch` / `research_construction_path_lab_vs_public(elf)` | Locks `lab_has_cu_pdi_config=0` offline → `construction_gap_cu_pdi=1` |
| `PatchedCtrltextFingerprint` / `fingerprint_patched_ctrltext_after_rela` (+ `_c2_locked`) | After-rela profile (len, classic, rela, staged, txn header, patched+unpatched sha) |
| `InstrPayloadProfile` / `profile_instr_payload_bytes` | Payload walk: blob_kind, classic, header, **opcode histogram** (`TxnOpOpcodeProfile`) |
| `CuPdiNecessityResearch` / `research_cu_pdi_necessity_*` | Ctrltext match alone ≠ residual close |
| CU/PDI section research | `research_cu_pdi_section_identity_*`, `_offset_map`, `_body_bounds`, xclbin extract dry-run |

### 1c. C1–C6 structural FP / residual class

| Phase | API | Soft note |
|-------|-----|-----------|
| **C1** | `validate_aiebu_elf_ctrltext_structure` / `format_c1_validator_note` | ELF32 LE + `.ctrltext` + rela OOB; classic walk reported not required for `ok` |
| **C2** | `StructuralLabFingerprint` / `structural_fingerprint_from_lab_{signals,detail}` / `structural_fingerprint_merge_elf` / `format_c2_structural_fingerprint_note` | content/bo/start_npu sizes, ty, map_offset, rela_count + **compact rela_targets**, config_cu/pdi_bytes/cu_func/layout/aiebu |
| **C3** | `ResidualClassifierSplit` / `classify_residual_split(_ex)` / `format_c3_residual_split_note` | Axes: pdi_geometry, cu_map, protocol_mismatch, **wait_cmd_einval**, app_load, unknown |
| **C4** | `classify_iron_artifact_ingest` / `format_c4_iron_ingest_note` / path allow | Empty blocked; present classifies; product_known_good=0 |
| **C5** | `fingerprint_pdi_geometry_bytes` / `compare_pdi_geometry_to_spent_flexml` / `format_c5_pdi_geometry_note` | len, sha16, first/second u32 LE, leading zeros, size class vs spent flexml 27200/`0eeeb70c…`; `spent_rematch_not_unlock_alone=1` |
| **C6** | `pack_config_hwctx_dbg_buf_aie2` / `validate_config_hwctx_dbg_buf_aie2` / `debug_bo_uapi_prep_aie2` / `format_c6_debug_bo_prep_note` | AIE2 DBG_BUF pack only; not live-wired |

Also: `SiliconResidualClass` / `classify_last_apply_residual(_ex)` (status residual including `wait_cmd_einval` honesty).

### 1d. PDI identity / research fingerprint

| API | Role |
|-----|------|
| `PdiBlobFingerprint` / `fingerprint_pdi_blob_bytes` | len + sha16 prefix |
| `ResearchPdiIdentity` / `research_pdi_identity_from_*` / `format_research_pdi_preflight_note` | Source kind + identity; research dump ≠ product known-good |
| `parse_pdi_fingerprint_from_detail` | last_apply detail tokens |
| C5 geometry vs spent flexml (above) | Ledger: unified-2x4x4 spent; llm-2x4x4 / unified-4x4 unspent |

---

## 2. What soft-compare reports today (size mismatch vs structural)

### Compare core (`compare_external_ctrltext_to_lab_export`)

When external present:

| Field | Meaning |
|-------|---------|
| `len_match` | `external_len == lab_len` |
| `sha_match` | external sha == lab unpatched sha (both non-empty) |
| `external_classic_walk_ok` / `lab_classic_walk_ok` | Classic v0.1 walk |
| `external_staged_mladf_classic_profile` / lab echo | 1688 / gen4 / 8×6 class |
| `structural_match` | **Requires** len_match **and** both classic_walk_ok **and** staged bits equal **and** txn_num_ops/dev_gen/cols/rows equal. Dual non-walkable garbage cannot claim structural_match. |

Detail tokens:

- absent → `external_absent_awaiting_capture`
- full match → `sha_and_structural_match_lab_unpatched` (**still invent_pass=0**)
- sha only → `sha_match_lab_unpatched`
- structural only → `structural_match_sha_diff`
- same len different content → `len_match_content_diff`
- else → `external_differs_from_lab`

**What soft note does *not* currently surface on the compare bag itself:**

- Opcode histogram (`TxnOpOpcodeProfile` exists on `InstrPayloadProfile` / `profile_instr_payload_bytes` soft note only)
- Per-side txn header fields as separate external vs lab tokens (they gate structural_match but are not printed as `external_txn_num_ops=` vs `lab_txn_num_ops=` on `ExternalCtrltextCompareResearch::format_soft_note`)
- Rela offset lists / patch targets on external raw ctrltext (export has rela_count from ELF only; raw peel has no rela)
- Distance metric for size-mismatched material (300 vs 1688 collapses to `external_differs_from_lab` + booleans)
- Explicit size-class labels (`external_size_class=npu2_minimal` vs `lab_size_class=mladf_classic_1688`)

### Present capture material (RESIDUAL pin)

- Dir: `testdata/xdna2_cmd/external_known_good/`
- `working_stack_ctrltext.bin` **300 B**, sha16 `aaeb5b68cc298a45` (mlir-aie `vector_scalar_add` npu2 AOT) + sibling `mlir_aie_vector_scalar_add_npu2.*`
- Lab classic mladf peel: **1688 B**
- Soft compare: **size mismatch expected**; not matmul-matched residual identity
- Scorecard status: `mismatch` (sha false) when compare runs with both sides; never Pass

### Operator `--compare` path

`tools/operator_external_capture.sh --compare` (`run_compare`):

1. First bag `.bin`/`.ctrltext` or `COMPARE_BLOCKED blocked_on_external_bytes=1` (exit 1)
2. Needs last_apply fixture (`XDNA2_LAB_LAST_APPLY_JSON` or testdata default)
3. Sets `XDNA2_LAB_PRINT_C2_CMDBUF_RESEARCH=1` + `XDNA2_LAB_EXTERNAL_CTRLTEXT=<first bag file>`
4. Runs `classify_last_apply_residual.sh` / `classify_last_apply` example
5. Reminds: match ≠ residual closed ≠ Pass

---

## 3. Gaps (Route B opportunities)

| Gap | Exists today? | Gap detail |
|-----|---------------|------------|
| **Opcode histograms** | Yes on `profile_instr_payload_bytes` / `TxnOpOpcodeProfile` | **Not** wired into `ExternalCtrltextCompareResearch` or bag soft-compare distance for 300 vs 1688 |
| **Tile/column hints** | Header `txn_num_cols` / `txn_num_rows` in export + structural_match | No named size-class / topology label for non-mladf (e.g. npu2 vector_scalar_add cols×rows vs 8×6) |
| **Rela targets** | `list_rela_patch_targets_overwrite`; C2 merge_elf first-8 offsets | Not compared external ELF vs lab ELF as a pure structure bag; raw ctrltext has no rela |
| **Section sizes** | xclbin section offset/body bounds research | Capture-dir multi-file (xclbin/pdi/elf) not folded into one “material family fingerprint” vs lab flexml |
| **Header layout** | Classic header fields on profile/export | No dedicated `header_layout_fingerprint` soft note for external peel alone when len mismatch kills structural_match |
| **public-npu2-vs-lab-flexml distance** | Binary match/mismatch + PDI spent compare | **No** multi-axis distance: size class, classic walk, op histogram L1, header (cols/rows/ops/gen), sha16, family label (`mlir_aie_npu2` vs `mladf_classic`) |

Net: tools answer “match or not.” For present **expected mismatch** material they do not yet answer “what kind of public program is this, and how far is it from lab flexml/mladf on structural axes.”

---

## 4. Existing unit test patterns (TDD)

Patterns in `silicon_contract.rs` tests (feature `amd-xdna2`):

1. **Synthetic ELF:** `build_strix_research_noop_txn` + `build_minimal_test_elf32` → export/compare/C1.
2. **Honesty locks:** every soft note asserts exact tokens via `residual_note_has_exact_token` (`invent_pass=0`, `not_c_ready=1`, `not_ran_on_npu=1`, `product_known_good=0`).
3. **Match ≠ Pass:** self-match still invent_pass=0; dual garbage structural_match=0.
4. **Empty / refuse:** empty ELF, non-ELF, post_patch basename, VE2, bag absent → blocked tokens.
5. **C1–C6 named contracts:** `c1_validate_aiebu_elf_ctrltext_structure_contracts`, `c2_structural_fingerprint_from_detail_and_elf`, `c3_residual_classifier_split_axes`, `c4_iron_artifact_ingest_empty_blocked_and_classify`, `c5_pdi_geometry_vs_spent_flexml_contracts`, `c6_debug_bo_uapi_prep_aie2_contracts`.
6. **Optional corpus:** silicon_lab ELF “when present” for classic 1688 profile.
7. **Filter-friendly names:** `fingerprint_export_*`, `external_ctrltext_compare_dry_run_*`, `construction_path_*`.

Suggested cargo filters for new work (same style):

```bash
cargo test --features amd-xdna2 --lib fingerprint_export
cargo test --features amd-xdna2 --lib external_ctrltext_compare
cargo test --features amd-xdna2 --lib c1_validate
cargo test --features amd-xdna2 --lib c5_pdi_geometry
cargo test --features amd-xdna2 --lib structural_distance   # proposed
```

---

## 5. Where residual pin cites material today

| Location | What it pins |
|----------|----------------|
| `RESIDUAL.md` top table | Capture material landed: 300 B / `aaeb5b68…`; mismatch vs 1688 expected; ≠ Pass; unlock none; residual class `wait_cmd_einval` (flexml `2ae69036`) |
| `AGENTS.md` Latest Turn | Same honesty + ranked next (matmul-matched material; unspent flexml; no unlock mint) |
| `testdata/xdna2_cmd/external_known_good/PROVENANCE.md` | Operator ingest/compare playbook; refuse list; research-only |
| `docs/CAPTURE_MATERIAL_MLIR_AIE.md` | Concrete mlir-aie → capture dir steps |
| Reports | `.agents/reports/residual-pin-mlir-aie-material-landed.md`, `impl-mlir-aie-material-playbook.md` |
| Plans | `.agents/plans/plan-pass-unblock-sequence.md` |

**Parse rule:** first `C_UNLOCK=` in RESIDUAL = active only if no CONSUMED → currently **none active** (`2ae69036` CONSUMED).

---

## 6. pass_unblock_once / operator_external_capture hooks

| Tool | Hook |
|------|------|
| `tools/pass_unblock_once.sh` | `--status` bag bin count + blocked_on_external; `--agent-offline` → fingerprint_once + bag; `--ingest` → `operator_external_capture --install` + soft C4/C5 honesty lines (not full structural distance); **no** automatic soft-compare of bag vs lab classic on ingest |
| `tools/pass_unblock_fingerprint_once.sh` | PDI spent ledger, skip spent re-dump, xclbin inventory |
| `tools/operator_external_capture.sh` | `--status` / `--install` / **`--compare`** (soft classify with EXTERNAL_CTRLTEXT) / `--validate` |
| `tools/operator_app_load_kit.sh` | `pass-unblock`, `b-compare` → external capture |
| `tools/classify_last_apply_residual.sh` + example | Soft rows including export + compare when env set |

**Gap for Route B:** after material lands, `--agent-offline` / `--ingest` do not print a rich multi-axis structural distance for bag vs lab; operator must run `--compare` and still gets mostly match/mismatch + existing soft tokens.

---

## 7. Recommended pure offline deltas (2–4; invent_pass=0)

All pure unit TDD; soft notes hard-code honesty; **no** Pass / unlock / live.

### B1 — Surface opcode + header on external compare

- **Fns:** extend `ExternalCtrltextCompareResearch` (or companion) with optional `external_op_profile` / `lab_op_profile` echoes from `profile_instr_payload_bytes`; print `op_*` and `external_txn_num_{ops,cols,rows}` / lab echoes on soft note when both classic.
- **Tests:** `external_ctrltext_compare_surfaces_opcode_histogram_when_classic` — synthetic noop vs mutated op mix; invent_pass=0; structural_match rules unchanged.
- **Filter:** `external_ctrltext_compare` / `fingerprint_export_compare`

### B2 — Size-class + distance bag for expected mismatch

- **Fns:** `pub fn classify_ctrltext_size_class(len, profile) -> CtrltextSizeClass` (`empty` / `mladf_classic_1688` / `small_public_npu2` / `other`); `pub fn research_external_vs_lab_structural_distance(lab: &CtrltextFingerprintExport, external: &[u8]) -> ExternalVsLabStructuralDistance` with fields: size_class_match, classic_both, op_hist_l1 (u32 sum abs), header_cols_diff, header_rows_diff, header_ops_diff, sha_match, len_match, detail token, soft note always `distance_alone_not_pass=1 invent_pass=0 not_c_ready=1 not_ran_on_npu=1 product_known_good=0`.
- **Tests:** `structural_distance_300_vs_lab_export_shape` (synthetic 300-class vs 1688 export: size_class mismatch, distance detail non-empty, invent_pass=0); self-match distance all-zero still invent_pass=0.
- **Filter:** `structural_distance`

### B3 — Capture-dir material family fingerprint (multi-file, optional)

- **Fns:** `pub fn fingerprint_capture_material_dir_research(root: &Path) -> CaptureMaterialFingerprint` — counts allowed bins; if present, profile first ctrltext; optional xclbin magic/section markers via existing extract dry-run; optional PDI geometry vs spent flexml; soft `capture_material_fingerprint=`; always product_known_good=0; empty dir → blocked_on_external_bytes style.
- **Tests:** `capture_material_fingerprint_empty_blocked`; `capture_material_fingerprint_ctrltext_only_no_pass` (temp dir synthetic).
- **Filter:** `capture_material_fingerprint`
- **Hook (optional polish, same slice or follow-up):** one line from `pass_unblock_once --status` / `--agent-offline` calling note formatter (still offline soft).

### B4 — Rela target structure compare (ELF vs ELF only)

- **Fns:** `pub fn compare_rela_structure_lab_vs_external_elf(lab_elf, ext_elf) -> RelaStructureCompareResearch` — counts, first-N offsets, sym index multiset equality; soft note; invent_pass=0; missing ELF → absent detail.
- **Tests:** `rela_structure_compare_synthetic_match_neq_pass`; count/offset mismatch tokens.
- **Filter:** `rela_structure_compare`
- **Note:** does not apply to raw 300 B ctrltext alone; useful when bag has `*.elf` / aiebu siblings (already true for mlir-aie material).

**Priority suggestion:** B2 (most value for current 300-vs-1688 pin) → B1 (cheap reuse of existing histogram) → B3 (operator visibility) → B4 (ELF siblings).

---

## 8. Explicit non-goals

- Do **not** invent Pass / `ran_on_npu` / C-ready from any new classifier.
- Do **not** mint unlock from structural distance alone.
- Do **not** treat soft-compare mismatch as failure of capture landing (expected for vector_scalar_add).
- Do **not** reopen CONSUMED unlocks or re-live spent flexml as progress.
- Do **not** invent matmul-matched bytes.

---

## 9. Key file paths

| Path | Role |
|------|------|
| `/home/hunter/Projects/ai/dragon-npu/src/backends/amd_xdna2/silicon_contract.rs` | All pure research APIs + tests |
| `/home/hunter/Projects/ai/dragon-npu/src/backends/amd_xdna2.rs` | Re-exports |
| `/home/hunter/Projects/ai/dragon-npu/examples/classify_last_apply.rs` | Soft print |
| `/home/hunter/Projects/ai/dragon-npu/tools/operator_external_capture.sh` | `--compare` |
| `/home/hunter/Projects/ai/dragon-npu/tools/pass_unblock_once.sh` | Layer A entry |
| `/home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/external_known_good/` | Present material |
| `/home/hunter/Projects/ai/dragon-npu/RESIDUAL.md` | Residual pin SSoT |

---

**End of explore report.** invent_pass=0; Pass=No.
