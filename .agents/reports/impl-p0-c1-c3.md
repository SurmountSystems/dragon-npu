# Implement report: P0 C1 + C2 + C3 (Pass-unblock offline)

**Date:** 2026-08-10
**Repo:** `/home/hunter/Projects/ai/dragon-npu`
**Slice:** plan step 3 P0 only (validators + structural fingerprint + residual splits)
**Unlock:** **none** (no mint; `C_UNLOCK=2ae69036` and priors remain CONSUMED)

---

## 1. What landed

### C1 — aiebu / ELF / rela / ctrltext structure validators

| Item | Location |
|------|----------|
| `AiebuElfCtrltextValidateReason` | `silicon_contract.rs` |
| `AiebuElfCtrltextValidateResult` | same |
| `validate_aiebu_elf_ctrltext_structure` | pure; empty / bad magic / not ELF32 LE / missing `.ctrltext` / rela parse fail / rela offset OOB / empty or oversize ctrltext |
| `format_c1_validator_note` | soft bag `c1_aiebu_elf_ctrltext_validator` |
| Const | `C1_AIEBU_VALIDATOR_NOTE`, size class max 64 MiB |

**Behavior:** Stronger than `is_aiebu_elf_ctrltext` alone. Classic walk is reported (`classic_walk_ok`) but not required for structural `ok` (IRON / non-classic research images may still validate as ELF structure).

### C2 — structural lab fingerprint

| Item | Location |
|------|----------|
| `StructuralLabFingerprint` | BO sizes, map_offset / no-mmap, rela count + target summary, `config_cu` / `pdi_bytes` / `cu_func`, layout, aiebu |
| `structural_fingerprint_from_lab_signals` | from `LastApplyDetailSignals` |
| `structural_fingerprint_from_lab_detail` | parse detail → signals → FP |
| `structural_fingerprint_merge_elf` | merge aiebu rela offsets into FP |
| `format_c2_structural_fingerprint_note` | soft bag `c2_structural_lab_fingerprint` |
| Const | `C2_STRUCTURAL_FINGERPRINT_NOTE` |

### C3 — residual classifier splits

| Item | Location |
|------|----------|
| `ResidualClassifierSplit` | `pdi_geometry` / `cu_map` / `protocol_mismatch` / `wait_cmd_einval` / `app_load` / `unknown` |
| `ResidualClassifierSplitResult` | split + existing `SiliconResidualClass` + status echo |
| `classify_residual_split` / `_ex` | pure priority: wait_cmd_einval → APP_LOAD class → CU map fail → PDI geometry → protocol → unknown |
| `format_c3_residual_split_note` | soft bag `c3_residual_classifier_split` |
| Const | `C3_RESIDUAL_SPLIT_NOTE` |

**Honesty lock:** `wait_cmd_einval` is **not** folded back into `wait_timeout` (token `wait_cmd_einval_neq_wait_timeout=1` on soft note).

### Re-exports

Public `pub use` in `src/backends/amd_xdna2.rs` for the types, consts, and functions above.

### Tests

| Test name | Contract |
|-----------|----------|
| `c1_validate_aiebu_elf_ctrltext_structure_contracts` | empty / bad magic / not ELF32 LE / missing ctrltext / synthetic ok + honesty / rela OOB |
| `c2_structural_fingerprint_from_detail_and_elf` | detail parse fields + empty + ELF merge + honesty |
| `c3_residual_classifier_split_axes` | wait_cmd_einval / app_load / cu_map / pdi_geometry / protocol / unknown + honesty |

---

## 2. Red → green evidence

| Phase | Evidence |
|-------|----------|
| **Red (compile)** | First compile failed: `C1_CTRLTEXT_SIZE_CLASS_MAX: usize = MAX_INSTRUCTION_BLOB_BYTES` type mismatch (`u64` vs `usize`). Fixed to literal `64 * 1024 * 1024`. |
| **Red (clippy)** | `cast_possible_truncation` on `as usize`; `too_many_lines` on validator. Fixed size const + `#[allow(clippy::too_many_lines)]` on single pure entry. |
| **Green** | Same three tests pass; related filters pass (see below). |

Named TDD contracts were written with the product helpers in one implement pass; red evidence is the compile/clippy fails above plus intentional refuse paths exercised green (empty / bad magic / OOB / wait_cmd_einval ≠ timeout).

---

## 3. Commands + exit codes

```text
cargo fmt -p dragon-npu
  → exit 0

cargo fmt -p dragon-npu -- --check
  → exit 0 (FMT_RC=0)

cargo clippy --features amd-xdna2 --lib -- -D warnings
  → exit 0 (CLIPPY_RC=0)

cargo test --features amd-xdna2 --lib c1_validate_aiebu
  → 1 passed, exit 0

cargo test --features amd-xdna2 --lib c2_structural
  → 1 passed, exit 0

cargo test --features amd-xdna2 --lib c3_residual_classifier
  → 1 passed, exit 0

cargo test --features amd-xdna2 --lib classify_last_apply
  → 4 passed, exit 0

cargo test --features amd-xdna2 --lib construction_path
  → 5 passed, exit 0

cargo test --features amd-xdna2 --lib config_cu
  → 7 passed, exit 0
```

---

## 4. Honesty

| Token / claim | Value |
|---------------|--------|
| invent_pass | **0** (hard-coded on all new soft notes) |
| not_c_ready | **1** on all new soft notes |
| not_ran_on_npu | **1** on all new soft notes |
| Product Pass | **No** |
| `ran_on_npu` | **No** / not invented |
| Unlock mint | **None** — no `C_UNLOCK` edit to active |
| Live / L3 / ALLOW | **Not** opened |
| Capture bytes | **Not** invented; dir may stay empty |
| Validate / FP alone | **≠** C-ready / **≠** Pass (`validate_alone_not_c_ready=1`, `fingerprint_alone_not_pass=1`, `split_alone_not_pass=1`) |

---

## 5. What remains (plain English)

| Next | Who | Notes |
|------|-----|--------|
| **P1 C4** | Agent | IRON artifact ingest path (allowed basenames/dirs + classify into pass_unblock / soft compare); empty still blocked |
| **P1 C5** | Agent | Offline PDI geometry walker vs spent flexml 2x4x4 fingerprints; re-match ≠ new unlock alone |
| **P2 C6** | Agent | Debug-BO uAPI prep (constants/pack; research-only until unlock) |
| **Material** | Human/host | Real capture under `testdata/xdna2_cmd/external_known_good/` with PROVENANCE |
| **Live** | New unlock only | Host-minted `C_UNLOCK` + named fingerprint reason; prefer minimal load + optional debug BO |

Still **no** product Pass until live firmware accept + ERT Completed + non-zero OFM allclose via product decide.

---

## Files touched

- `src/backends/amd_xdna2/silicon_contract.rs` (C1/C2/C3 APIs + unit tests)
- `src/backends/amd_xdna2.rs` (re-exports)
- `RESIDUAL.md` (minimal offline delta note)
- `.agents/reports/impl-p0-c1-c3.md` (this report)
