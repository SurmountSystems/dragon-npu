# PROVENANCE — external known-good bag (operator capture)

**Research only. Not product Pass. Not C-ready. Not `ran_on_npu`.**

Place working **public-stack** post-patch ctrltext (and optionally notes about
CU/PDI) here when you have a stack that completes EXEC (ERT Completed + useful
OFM). Bins under this directory are gitignored; this PROVENANCE note is tracked.

**Operator shortcuts (prefer these over hand-copying env names):**

```bash
bash tools/operator_app_load_kit.sh              # overview A+B
bash tools/operator_external_capture.sh          # checklist + status
bash tools/operator_external_capture.sh --install /path/to/ctrltext.bin
# edit this file's capture table, then:
bash tools/operator_external_capture.sh --compare
```

## Why this directory exists

Lab residual is FW `APP_LOAD_PDI_FAIL` (`0x3000003`) after userspace aiebu+rela
OK. Offline export/compare (`export_ctrltext_fingerprint_research` /
`compare_external_ctrltext_to_lab_export`) need a **real external** identity
from a working path — not our pure-direct oracle.

**Never** treat these as known-good by default (loader refuses basenames):

| Path / basename | Why refused |
|-----------------|-------------|
| `instr_post_patch.bin` (e.g. `gt_capture/…`) | Pure-direct patch oracle (our paddrs) |
| `instr_vendor_post_patch.bin` | Vendor capture when present still local lab |
| `ctrl_post_patch.bin` | Lab ctrl patch oracle |
| `instr_raw_txn.bin` | Unpatched raw txn oracle (not public-stack known-good) |
| `instr_vendor_txn_bin.bin` | Vendor txn diagnostic dump (not known-good) |
| VE2 `validate.xclbin` / `ve2_latency/` | **NOT_AIE2** research stub |

## What to put here

| File (example) | Content |
|----------------|---------|
| `working_stack_ctrltext.bin` or `*.ctrltext` | Post-patch **ctrltext** bytes from a working public stack (XRT/DD after rela / CONFIG_CU path). Prefer unpatched peel for fair sha vs lab export, **or** document which. |
| `notes_cu_pdi.txt` (optional) | Whether the run also did CONFIG_CU / PDI BOs; sizes; **no** private PDI decode required. PDI bag confinement (future) mirrors this dir policy — do not invent a decoder. |

## Fail-closed bag-drop checklist (stamp `8e898123`)

Pure helper: `check_external_known_good_bag_drop` / soft line
`external_known_good_bag_drop_check=`. Incomplete drops refuse research-complete.

| Gate | Fail-closed if |
|------|----------------|
| Ctrltext bytes | No non-empty `.bin` / `.ctrltext` bag-resident **or** env path |
| PROVENANCE research-ready | Missing research-only + source/stack markers, or invent tokens |
| Basename refuse | `instr_post_patch.bin` / vendor post_patch / raw txn oracles |
| Platform refuse | VE2 / `NOT_AIE2_KNOWN_GOOD` claim |

`bag_drop_complete=1` **≠** Pass **≠** residual closed **≠** product known-good.
Soft always `bag_drop_alone_closes_fw_residual=0 invent_pass=0 product_known_good=0`.

E2e soft orchestration (body-bounds + journal tokens + bag drop + residual class):
`app_load_e2e_fixture_research=` under `XDNA2_LAB_PRINT_C2_CMDBUF_RESEARCH=1` —
always `residual_not_closed=1`.

Offline progress scorecard (stamp `9cdfe8d0`): composes class + bag-drop + e2e
+ external_compare_status (`absent|loaded|match|mismatch|refused`) —
`app_load_offline_progress_scorecard=` always
`scorecard_alone_closes_fw_residual=0 residual_not_closed=1 invent_pass=0`.

External ctrltext compare dry-run (stamp `6033158a`): when bag absent soft line
`external_ctrltext_compare_dry_run=` reports explicit
`blocked_on_external_bytes=1` (not silent absent). Refuse local post_patch /
VE2 (`platform_not_aie2=1`). Match ≠ Pass. Always
`dry_run_alone_closes_fw_residual=0 residual_not_closed=1 invent_pass=0`.

Lab export fingerprint auto-feed (stamp `d2e287ab`): soft
`lab_export_fingerprint_feed=` derives lab fingerprint from staged ELF / export
path when available (never invents external bytes or unpatched sha from
last_apply signals alone). Soft dry-run auto-feeds that export so lab-ready +
bag absent → `lab_export_ready=1` + `blocked_on_external_bytes=1`. Always
`feed_alone_closes_fw_residual=0 residual_not_closed=1 invent_pass=0`.

Bag + feed + dry-run composition scorecard (stamp `2b01425e`): soft
`app_load_bag_feed_dry_run_scorecard=` composes bag-drop + lab feed + dry-run
after auto-feed. When bag absent after feed → explicit
`blocked_on_external_bytes=1` (not silent). Match ≠ Pass. Always
`scorecard_alone_closes_fw_residual=0 residual_not_closed=1 invent_pass=0`.

Extract → bag-drop → composition scorecard pipeline (stamp `588c1c72`): soft
`app_load_extract_bag_scorecard_pipeline=` folds public xclbin section markers
(extract dry-run) into bag-drop + bag-feed composition scorecard. Markers present
**without** ctrltext bytes → still explicit `blocked_on_external_bytes=1`.
Optional research paste env `XDNA2_LAB_XCLBIN_SECTION_TEXT` (section **names**
only; never product known-good / PDI decode). Match ≠ Pass. Always
`extract_alone_closes_fw_residual=0 pipeline_alone_closes_fw_residual=0
residual_not_closed=1 invent_pass=0`.

## Operator extract-to-bag checklist (stamp `588c1c72`)

Pure helper: `format_external_known_good_operator_extract_checklist` / soft line
`external_known_good_operator_extract_checklist=`. **No invent bytes.** Match ≠ Pass.

| Step | Action | Refuse if |
|------|--------|-----------|
| 1 | Run working public stack (XRT / DynamicDispatch) same mladf key when possible | — |
| 2 | Capture post-patch **ctrltext** (or aiebu ELF + document peel) | local pure-direct oracle only |
| 3 | Copy to this directory with clear basename (`.bin` / `.ctrltext`) | `instr_post_patch.bin`, `instr_vendor_post_patch.bin`, `ctrl_post_patch.bin`, `instr_raw_txn.bin` |
| 4 | Fill capture table below; keep research-only + source/stack markers | invent tokens / `product_known_good=1` |
| 5 | Confirm bag-drop complete (bytes + this PROVENANCE) via soft classify or `cargo test … bag_drop` | VE2 / `NOT_AIE2` as AIE2 known-good |
| 6 | Set confined `XDNA2_LAB_EXTERNAL_CTRLTEXT` + PRINT flag soft compare | path outside bag/temp; symlink final component |
| 7 | Read `external_ctrltext_compare=` + scorecard — match **≠** residual closed | claim Pass / second `C_UNLOCK` |
| 8 | Soft `external_ctrltext_compare_dry_run=` — bag absent → `blocked_on_external_bytes=1`; dry-run alone ≠ residual closed | invent Pass from dry-run |
| 9 | Soft `lab_export_fingerprint_feed=` auto-feeds lab export into dry-run when staged ELF present; feed alone ≠ residual closed | invent Pass from feed |
| 10 | Soft `app_load_bag_feed_dry_run_scorecard=` composition row — bag absent after auto-feed → `blocked_on_external_bytes=1`; scorecard alone ≠ residual closed | invent Pass from scorecard |
| 11 | Soft `app_load_extract_bag_scorecard_pipeline=` — public section markers without bytes still `blocked_on_external_bytes=1`; extract/pipeline alone ≠ residual closed | invent Pass from markers / pipeline |

## Capture outline (operator, offline when stack works)

1. Run a working public path (Ryzen AI / XRT / DynamicDispatch) for the same
   mladf key if possible (`…_1_1152_256_128`).
2. Capture instruction image **after** host peel/patch (ctrltext content that
   goes into the instr BO / START_NPU buffer), **or** aiebu ELF + document
   peel step.
3. Copy bytes to this directory with a clear name; fill the table below.
4. Confirm bag-drop complete (bytes + this PROVENANCE filled) via soft classify
   or `cargo test --features amd-xdna2 --lib bag_drop`.
5. Compare offline (no L3 / no ALLOW / no invent Pass):

```bash
# Soft classify research lines (PRINT flag) + confined env path:
export XDNA2_LAB_PRINT_C2_CMDBUF_RESEARCH=1
export XDNA2_LAB_EXTERNAL_CTRLTEXT="$PWD/testdata/xdna2_cmd/external_known_good/working_stack_ctrltext.bin"
cargo run --features amd-xdna2 --example classify_last_apply --quiet -- \
  testdata/xdna2_cmd/last_apply.json
# Look for external_ctrltext_compare= … product_known_good=0 invent_pass=0
# and app_load_offline_progress_scorecard= … residual_not_closed=1
unset XDNA2_LAB_EXTERNAL_CTRLTEXT
```

Unit / pure path:

```bash
cargo test --features amd-xdna2 --lib external_known_good
cargo test --features amd-xdna2 --lib app_load_offline_progress
cargo test --features amd-xdna2 --lib operator_extract_checklist
```

## Honesty

- Capture **≠** Pass.
- Loading known-good for research **≠** C-ready **≠** second `C_UNLOCK`.
- Soft compare always `invent_pass=0 not_c_ready=1 not_ran_on_npu=1 product_known_good=0`.
- Match to lab unpatched peel may only show we already match ourselves (failing
  corpus) — still residual until FW accepts a live job.
- Ctrltext sha match alone **never** closes FW residual (CU/PDI construction gap
  may still apply — see `cu_pdi_necessity_research`).
- Progress scorecard alone **never** closes residual
  (`scorecard_alone_closes_fw_residual=0`).
- `C_UNLOCK=b17b0119` remains **CONSUMED** until a new host-minted unlock after
  a real offline payload/path delta.

## PROVENANCE field checks (fail-closed before research-present)

Unit-locked by `validate_external_known_good_provenance_text` (stamp `f06d5b38`).
Lab-bag load via `external_known_good_load_path_is_allowed` requires **all**:

| Check | This template |
|-------|----------------|
| Non-empty | yes |
| Research-only marker (prose `Research only` / `Not product Pass` / `Not C-ready`, or exact `not_c_ready=1`; **not** `not_c_ready=0`) | yes (header + honesty) |
| Source/stack marker (`Source stack` / `public stack` / whole-token XRT / …) | yes (sections) |
| No invent / honesty-inverse tokens (`invent_pass=1|true`, `ran_on_npu=1|true`, `c_ready=1`, `product_known_good=1`, `not_c_ready=0`; case-insensitive) | yes |

Research-ready **≠** product known-good. Empty capture table is OK until bytes land.

## Capture record (fill when bytes land)

| Field | Value |
|-------|--------|
| Date | |
| Source stack | (e.g. XRT shim / DD / ONNX EP) |
| Key / shape | |
| File basename | |
| Bytes / sha256 | |
| Unpatched or post-rela? | |
| CONFIG_CU / PDI present on that run? | yes / no / unknown |
| Notes | |
