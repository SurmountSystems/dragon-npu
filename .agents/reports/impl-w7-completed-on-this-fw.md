# W7-3 — PROVENANCE `completed_on_this_fw` soft research TDD

**Date:** 2026-08-11
**Scope:** offline pure helpers + unit test + soft classify print
**Live / unlock / AOT re-run:** none
**invent_pass:** **0** (locked)
**Pass / ran_on_npu / residual_matched / load_accepted from field alone:** **No**

## What landed

Soft research parse of PROVENANCE field `completed_on_this_fw` (yes / no /
absent / malformed) for load-accepted / external known-good notes. Research
scorecard only.

### Honesty (always)

| Claim | From this surface |
|-------|-------------------|
| `invent_pass` | **0** always |
| product Pass | **never** |
| `residual_matched_claim` | **0** always (even when value=yes) |
| `load_accepted_claim` | **0** always (field alone ≠ live decide) |
| `product_known_good` | **0** always |
| `not_c_ready` / `not_ran_on_npu` | **1** on soft note |

`completed_on_this_fw=yes` ≠ product Pass ≠ residual_matched ≠ load_accepted
(load_accepted still needs live ERT Completed + OFM allclose via
`decide_ran_on_npu`).

## APIs (`src/backends/amd_xdna2/silicon_contract.rs`)

| API | Role |
|-----|------|
| `COMPLETED_ON_THIS_FW_NOTE` | Soft bag name (`completed_on_this_fw_research`) |
| `CompletedOnThisFwValue` | `Yes` / `No` / `Absent` / `Malformed` + `as_str()` |
| `CompletedOnThisFwResearch` | Parse result; honesty flags locked false for residual/load_accepted |
| `CompletedOnThisFwResearch::format_soft_note` | Delegates to format helper |
| `classify_completed_on_this_fw_token` | Single token → enum (`yes`/`true`/`1`, `no`/`false`/`0`) |
| `parse_completed_on_this_fw_from_provenance` | Pure PROVENANCE text parse |
| `format_completed_on_this_fw_note` | Soft tokens; always `invent_pass=0` |

### Parse rules (pure)

- Key is whole-token `completed_on_this_fw` only (does **not** match
  `iron_completed_on_this_fw`).
- Forms: `key=value`, markdown table `| key | value |`, prose `key: value`.
- Values: yes/true/1 → Yes; no/false/0 → No; empty → Absent; other → Malformed.
- Skips instructional template language (`yes only if …`); prefers **last**
  non-instructional concrete value when multiple keys appear.

### Soft note tokens (always present)

```
completed_on_this_fw_research completed_on_this_fw={yes|no|absent|malformed}
residual_matched_claim=0 load_accepted_claim=0 product_known_good=0
invent_pass=0 not_c_ready=1 not_ran_on_npu=1
field_alone_not_pass=1 field_alone_not_load_accepted=1
field_alone_not_residual_matched=1 product_pass_requires_live_decide=1
raw=… detail=…
```

## Re-exports

`src/backends/amd_xdna2.rs` re-exports:

- `COMPLETED_ON_THIS_FW_NOTE`
- `CompletedOnThisFwResearch`, `CompletedOnThisFwValue`
- `classify_completed_on_this_fw_token`
- `format_completed_on_this_fw_note`
- `parse_completed_on_this_fw_from_provenance`

## Soft classify wire

`examples/classify_last_apply.rs` — in
`print_app_load_e2e_and_bag_drop_optional`, after bag PROVENANCE field check:

- present PROVENANCE → print `completed_on_this_fw_research=` via format helper
- absent PROVENANCE → print honesty-locked absent line (`invent_pass=0`,
  `residual_matched_claim=0`, `load_accepted_claim=0`)

## Tests

| Test | Filter |
|------|--------|
| `completed_on_this_fw_provenance_research_parse_honesty` | `completed_on_this_fw` |

Covers: absent/empty, no key, yes, no, synonyms (true/false/1/0), malformed,
`iron_` prefix isolation, template-only → absent, template-then-filled yes,
markdown table no, tracked bag PROVENANCE honesty (no invent residual/load_accepted).

### Green counts

```
cargo test --features amd-xdna2 --lib completed_on_this_fw
# 1 passed; 0 failed; 655 filtered out

cargo fmt -p dragon-npu
cargo clippy -p dragon-npu --features amd-xdna2 --lib -- -D warnings
# clean
```

(`--all-targets -D warnings` still red on pre-existing tree debt outside this
slice; not introduced by W7-3 product APIs.)

## Not done / out of scope

- Live NPU / ALLOW / unlock mint
- Re-run public AOT
- Invent capture bytes
- RESIDUAL unlock to active
- Auto-setting `load_accepted_claim=1` from PROVENANCE yes alone

## Residual honesty

still **no** product Pass; invent_pass=0; field parse ≠ residual match ≠
load-accepted re-tag. Primary wall remains residual-matched / product
load-accepted material for lab classic 1688.
