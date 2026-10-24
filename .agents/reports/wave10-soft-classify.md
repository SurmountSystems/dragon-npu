# W10-soft — L1b readiness tokens on soft classify

**Date:** 2026-08-11
**Scope:** Soft classify path only. No unlock mint. No live apply. No ALLOW.
**Honesty:** invent_pass=0 · Pass=No · unlock=none · residual_matched_claim=0

## One-line

Soft `classify_last_apply` now always prints L1b readiness / wall tokens so operators see plain residual state without a separate tool.

## What landed

**File:** `examples/classify_last_apply.rs` (soft printer only; pure W9 helpers reused).

Always-on soft lines (after material residual inventory, before C2 research optional):

| Line prefix | Source | Always honesty |
|-------------|--------|----------------|
| `live_paddrs_extract=` | `extract_live_paddrs_from_last_apply_json` + `format_live_paddrs_research_note` | invent_pass=0 · paddrs_alone_not_pass=1 |
| `l1b_readiness=` | `format_l1b_readiness_checklist_from_extract` (offline unlock=false) | invent_pass=0 · l1b_live_justified=0 · residual_matched=0 · checklist_alone_not_pass=1 · checklist_alone_not_unlock=1 |
| `l1b_soft_score=` | `format_l1b_soft_score_tokens` | true_post_patch_missing=1 · oracle_sha16 · INSTR_BO_LIMIT=1 |

Exact wall tokens greppable on soft report:

- `true_post_patch_missing=1`
- `oracle_sha16=3c5ffca750922d3b` (lab classic unpatched)
- `INSTR_BO_LIMIT=1`
- `paddr_evidence_ready=0|1` (from extract; fixture last_apply has none)
- `l1b_live_justified=0` (offline unlock_active=false always)
- `invent_pass=0`

Help text documents the three soft line prefixes.

**No pure code change.** Reused W9:

- `extract_live_paddrs_from_last_apply_json`
- `format_l1b_readiness_checklist_from_extract` / `format_l1b_soft_score_tokens`
- `format_live_paddrs_research_note`

Also: clippy `too_many_lines` allow on pre-existing research printer
`print_ctrltext_fingerprint_export_optional` so example `-D warnings` is clean.

## Smoke (fixture)

```bash
cargo run --features amd-xdna2 --example classify_last_apply --quiet -- \
  testdata/xdna2_cmd/last_apply.json
```

Observed (locked historical fixture; no IO paddrs):

```text
live_paddrs_extract=... invent_pass=0 ... live_paddrs_present=0 source=absent ...
l1b_readiness=l1b_readiness_checklist invent_pass=0 not_c_ready=1 not_ran_on_npu=1
  residual_matched=0 ... paddr_evidence_ready=0 unlock_required=1
  l1b_offline_ready=0 l1b_live_justified=0 ... true_post_patch_missing=1
  oracle_sha16=3c5ffca750922d3b INSTR_BO_LIMIT=1
l1b_soft_score=true_post_patch_missing=1 oracle_sha16=3c5ffca750922d3b
  INSTR_BO_LIMIT=1 live_paddrs_present=0 instr_paddr_known=0
CLASSIFY_SOFT_OK residual=not_pass_no_ert_error (report only; ≠ C-ready ≠ ran_on_npu)
```

When a last_apply with bo_diag / io_paddrs is classified, `paddr_evidence_ready=1`
and `l1b_offline_ready=1` may flip; **`l1b_live_justified` stays 0** offline
(no unlock inferred; mint still human).

## Gates

```text
cargo fmt -p dragon-npu
cargo clippy -p dragon-npu --features amd-xdna2 --example classify_last_apply -- -D warnings  # clean
cargo test --features amd-xdna2 --lib w9_b_   # 4 ok (L1b pure regression)
```

## Honesty pin

- invent_pass=**0** always on soft L1b lines
- never invent Pass / residual_matched / load_accepted
- no unlock mint · no live · no ALLOW
- checklist alone ≠ C-ready ≠ ran_on_npu
- offline paddr presence ≠ live L1b justified

## Not in this slice

- Live L1b shot (needs **new** unlock + named reason + same-shot device paddrs)
- Residual pin / plan mirror (parent if needed)
- Material true post-patch still highest Pass gap
