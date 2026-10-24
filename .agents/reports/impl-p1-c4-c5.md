# Implement report: P1 C4 + C5 (Pass-unblock offline)

**Date:** 2026-08-10
**Repo:** `/home/hunter/Projects/ai/dragon-npu`
**Slice:** plan step 3 P1 only (IRON / public artifact ingest + PDI geometry vs spent flexml)
**Unlock:** **none** (no mint; `C_UNLOCK=2ae69036` and priors remain CONSUMED)

---

## 1. What landed

### C4 — IRON / public artifact ingest

| Item | Location |
|------|----------|
| `C4_IRON_INGEST_NOTE` | `silicon_contract.rs` |
| `IronArtifactKind` | absent / ctrltext_bin / aiebu_elf / pdi_opaque / unknown_opaque / refused_local_oracle / path_not_allowed |
| `IronArtifactIngestClassify` | path_allowed, blocked_on_empty, bytes_present, len, sha16, kind, product_known_good=false |
| `iron_artifact_extension_ok` | `.bin` / `.ctrltext` / `.elf` / `.pdi` (broader than compare-bag) |
| `iron_artifact_path_is_allowed` | same roots as external known-good bag (`temp` ∪ `testdata/xdna2_cmd/external_known_good/`); refuse post-patch oracles; no `..` |
| `guess_iron_artifact_kind` | ELF magic → aiebu_elf; size class / `.pdi` → pdi_opaque; `.bin`/`.ctrltext` → ctrltext_bin |
| `classify_iron_artifact_ingest` | pure path+bytes classify |
| `format_c4_iron_ingest_note` | soft bag `c4_iron_artifact_ingest` |

**Behavior:** Empty / absent bytes → `blocked_on_empty=1`. Present file classifies with size/sha16/kind. Always `product_known_good=0`. Ingest alone ≠ Pass / C-ready.

### C5 — PDI geometry vs spent flexml

| Item | Location |
|------|----------|
| `C5_PDI_GEOMETRY_NOTE` | soft bag name |
| `SPENT_FLEXML_UNIFIED_2X4X4_BYTES` | `27200` |
| `SPENT_FLEXML_UNIFIED_2X4X4_SHA16` | `0eeeb70c84df056e` |
| `PdiGeometrySizeClass` | empty / spent_flexml_2x4x4 / research_cvml_dump / other |
| `PdiGeometryFingerprint` | len, sha16, first/second LE u32 peeks, leading zeros, size class |
| `PdiGeometryVsSpent` | empty / match_spent / size_match_sha_diff / diff / unknown |
| `PdiGeometryVsSpentResult` | compare + geometry + `spent_rematch_not_unlock_alone=true` |
| `fingerprint_pdi_geometry_bytes` | pure header walk + fingerprint |
| `compare_pdi_geometry_to_spent_flexml` | full-byte compare |
| `compare_pdi_fingerprint_to_spent_flexml` | fingerprint-only compare (MatchSpent lock without inventing spent body) |
| `format_c5_pdi_geometry_note` | soft bag `c5_pdi_geometry_vs_spent_flexml` |

**Honesty lock:** spent re-match always carries `spent_rematch_not_unlock_alone=1` — not a new unlock reason alone.

### Script glue

`tools/pass_unblock_once.sh --ingest` prints C4/C5 soft honesty pointers after install (does not invent Pass; empty capture still `blocked_on_empty=1`).

### Re-exports

Public `pub use` in `src/backends/amd_xdna2.rs` for types, consts, and functions above.

### Tests

| Test name | Contract |
|-----------|----------|
| `c4_iron_artifact_ingest_empty_blocked_and_classify` | empty blocked; extension allowlist; oracle refuse; path not allowed; temp-root ctrltext classify; ELF magic; PDI size class; honesty tokens |
| `c5_pdi_geometry_vs_spent_flexml_contracts` | empty / diff / size_match_sha_diff / research dump size class / MatchSpent via fingerprint / unknown incomplete / header peeks + honesty |

---

## 2. Red → green evidence

| Phase | Evidence |
|-------|----------|
| **Red (clippy)** | First clippy `-D warnings` failed: missing docs on C5 fields/variants/methods; unnested or-patterns; `usize`→`u32` cast; `uninlined_format_args` on spent constants. |
| **Green** | Docs + `Some("ctrltext" \| "bin")` + `u32::try_from(count)` + inline format; same two tests pass. |

Contracts written with product helpers in one implement pass; refuse/empty paths exercised green (empty blocked, oracle refuse, path not allowed, spent rematch ≠ unlock).

---

## 3. Commands + exit codes

```text
cargo fmt -p dragon-npu
  → exit 0

cargo fmt -p dragon-npu -- --check
  → exit 0 (FMT_RC=0)

cargo clippy --features amd-xdna2 --lib -- -D warnings
  → exit 0

cargo test --features amd-xdna2 --lib c4_iron
  → 1 passed, exit 0

cargo test --features amd-xdna2 --lib c5_pdi
  → 1 passed, exit 0

cargo test --features amd-xdna2 --lib c1_validate
  → 1 passed, exit 0

cargo test --features amd-xdna2 --lib c2_structural
  → 1 passed, exit 0

cargo test --features amd-xdna2 --lib c3_residual
  → 1 passed, exit 0

cargo test --features amd-xdna2 --lib research_pdi
  → 1 passed, exit 0

cargo test --features amd-xdna2 --lib external_known_good_path
  → 1 passed, exit 0

bash -n tools/pass_unblock_once.sh
  → exit 0
```

---

## 4. Honesty

| Token / claim | Value |
|---------------|--------|
| invent_pass | **0** (hard-coded on C4/C5 soft notes) |
| not_c_ready | **1** on C4/C5 soft notes |
| not_ran_on_npu | **1** on C4/C5 soft notes |
| product_known_good | **0** on C4 |
| spent_rematch_not_unlock_alone | **1** on C5 |
| iron_ingest_alone_not_pass | **1** |
| geometry_alone_not_pass | **1** |
| Product Pass | **No** |
| `ran_on_npu` | **No** / not invented |
| Unlock mint | **None** — no `C_UNLOCK` edit to active |
| Live / L3 / ALLOW | **Not** opened |
| Capture bytes | **Not** invented; dir may stay empty |
| Ingest / geometry alone | **≠** C-ready / **≠** Pass / **≠** unlock |

---

## 5. What remains (plain English)

| Next | Who | Notes |
|------|-----|--------|
| **P2 C6** | Agent | Debug-BO uAPI prep (constants/pack; research-only until unlock) + Step 2 live recipe checklist |
| **Material** | Human/host | Real capture under `testdata/xdna2_cmd/external_known_good/` with PROVENANCE (`bash tools/pass_unblock_once.sh --ingest …`) |
| **Live** | New unlock only | Host-minted `C_UNLOCK` + named fingerprint reason; spent flexml rematch alone is **not** enough |

Still **no** product Pass until live firmware accept + ERT Completed + non-zero OFM allclose via product decide.

---

## Files touched

- `src/backends/amd_xdna2/silicon_contract.rs` (C4/C5 APIs + unit tests)
- `src/backends/amd_xdna2.rs` (re-exports)
- `tools/pass_unblock_once.sh` (post-ingest C4/C5 honesty echo)
- `RESIDUAL.md` (minimal offline delta note)
- `AGENTS.md` (Latest Turn honesty pin)
- `.agents/plans/plan-pass-unblock-sequence.md` (P1 → Done)
- `.agents/reports/impl-p1-c4-c5.md` (this report)
