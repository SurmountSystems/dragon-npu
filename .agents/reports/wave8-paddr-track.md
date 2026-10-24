# W8 Track Paddr (O3 + M3 offline)

**Date:** 2026-08-11
**Scope:** Offline pure TDD only. No unlock mint. No live apply. No ALLOW.
**invent_pass=0** · Pass=**No** · L1b offline-ready=**No** (default; needs unlock + live after this wire)

## Goal

Bridge architecture: product lab may not need vendor post-patch if aiebu rela×12 uses **real device paddrs** from a live map (`io_paddrs` / `bo_diag`), not GT synthetic `0xa11f…` (patched oracle sha16 `ccab3960…`).

## APIs (silicon_contract + amd_xdna2 re-exports)

### O3 — Live paddr extract

| API | Role |
|-----|------|
| `LivePaddrsExtract` | ifm/wts/ofm/ctrl/instr options + `live_paddrs_present` + source |
| `extract_live_paddrs_from_detail` | Parse timed detail (`io_paddrs`, bo_diag instr, signals) |
| `extract_live_paddrs_from_vendor_json` | Parse vendor_paddrs / extract_paddrs JSON |
| `extract_live_paddrs_from_last_apply_json` | Schema-2 last_apply → detail extract |
| `format_live_paddrs_research_note` | Soft: `live_paddrs_research live_paddrs_present=0\|1 invent_pass=0 …` |
| `LIVE_PADDRS_RESEARCH_NOTE` | Bag name constant |

Honesty: never invent paddrs; zeros / `u64::MAX` absent; always `invent_pass=0 not_c_ready=1 not_ran_on_npu=1 residual_matched_claim=0`.

### M3 — Live-mapped pure-direct rela

| API | Role |
|-----|------|
| `LiveMappedRelaResearch` | Patch result + `differs_from_gt_synthetic` + shas |
| `research_live_mapped_rela` | ELF + paddrs → patched body vs GT synthetic oracle |
| `research_live_mapped_rela_from_extract` | From `LivePaddrsExtract` |
| `format_live_mapped_rela_research_note` | Soft bag `live_mapped_rela_research` |
| `LIVE_MAPPED_RELA_RESEARCH_NOTE` | Bag name |
| `GT_SYNTHETIC_PATCHED_CTRLTEXT_SHA256` / `_SHA16` | `ccab3960…` lock |
| `GT_SYNTHETIC_{IFM,WTS,OFM,CTRL}_PADDR` | `0xa11f…` dump paddrs |
| `LAB_CLASSIC_UNPATCHED_CTRLTEXT_SHA16` | `3c5ffca750922d3b` |

**Rela design (existing pure-direct):**
`patch_aiebu_elf_ctrltext_with_bos` peels `.ctrltext`, applies `.rela.dyn` with bases sym1=ifm, sym2=wts, sym3=ofm, sym4=ctrl (overwrite u64 LE). Staged mladf uses ×12 on classic 1688. Soft always `l1b_offline_ready=0` (wire alone ≠ live unlock).

### O1/O2 (cheap)

| API | Role |
|-----|------|
| `CapturePatchClass` | `ClassicUnpatched` / `TruePostPatchCandidate` / `MultiTxnWrongBody` / `Unknown` |
| `classify_capture_patch_class` | From capture_mode / sha / size |
| `format_capture_patch_class_note` | Loud refuse residual_matched for unpatched / multi |
| `CAPTURE_PATCH_CLASS_NOTE` | Bag name |

`txn_container_unpatched` → `classic_unpatched=1` + `loud_refuse_residual_matched_unpatched=1` + `residual_matched_claim=0`.

## Tests (green)

Filter:

```bash
cargo test --features amd-xdna2 --lib w8_
```

| Test | Covers |
|------|--------|
| `w8_o3_extract_live_paddrs_from_detail_present` | detail + instr xdna_addr + soft note |
| `w8_o3_extract_live_paddrs_absent_and_refuse_zeros` | empty / zeros / bo_diag instr prefer |
| `w8_o3_extract_from_vendor_json_fixture_and_last_apply` | fixtures under testdata |
| `w8_o1_o2_capture_patch_class_unpatched_refuse` | unpatched / multi / post-patch candidate |
| `w8_m3_live_mapped_rela_differs_from_gt_synthetic` | synthetic = `ccab3960…`; live paddrs differ; honesty |

**5 passed.**

## Gates

| Gate | Result |
|------|--------|
| `cargo fmt -p dragon-npu` | ok |
| `cargo clippy -p dragon-npu --features amd-xdna2 --lib -- -D warnings` | ok |
| `cargo test --features amd-xdna2 --lib w8_` | 5 ok |

## L1b offline-ready?

**No.** Offline pure wire + TDD only. Soft notes lock `l1b_offline_ready=0`.
Next for L1b: **new** host-minted unlock + named reason (live-mapped rela on classic 1688 aiebu under real map), not this pin alone. No unlock minted. No live apply.

## Honesty

- invent_pass=0
- Pass=No
- residual_matched_claim never set from paddrs/capture_mode alone
- GT synthetic `ccab3960…` ≠ load-accepted
- Unpatched CLASS A / `3c5ffca7…` ≠ Pass

## Files touched

- `src/backends/amd_xdna2/silicon_contract.rs` — O3/M3/O1/O2 APIs + tests
- `src/backends/amd_xdna2.rs` — re-exports
- `.agents/reports/wave8-paddr-track.md` — this report
