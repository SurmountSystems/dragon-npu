# Wave4 scorecard tools (W4a T3 + T6)

**Date:** 2026-08-11  
**Scope:** offline pure TDD only  
**Honesty:** invent_pass=0 · Pass=No · no unlock · no live NPU · no ALLOW

## Summary

Landed two pure research surfaces under `silicon_contract.rs` (re-exported on
`amd-xdna2`):

| Slice | API | Purpose |
|-------|-----|---------|
| **T3** | `LastApplyRichTokens` / `parse_last_apply_rich_tokens` / `format_last_apply_rich_tokens_note` | Parse structured last_apply tokens when present: `wait_path=`, `syncobj=` / `syncobj_handle=`, `ert_state=`, debug-BO fields (`debug_bo_config` region, `attempt_ok`, `debug_bo_size`, `debug_bo_ty`, `op`, `bo_handle`, `gate_open`, path) |
| **T6** | `MaterialExecReadinessScorecard` / `score_material_exec_readiness` / `_from_path` / bag soft lines | Score instruction material: classic walk, op count, distance-to-1688, size class, optional rela_count; **residual_matched_claim always 0** |

Neither surface invents Pass, residual match, unlock, or live status when tokens
are absent.

## T3 — last_apply richness

- Soft bag: `last_apply_rich_tokens`
- Soft always: `invent_pass=0 not_c_ready=1 not_ran_on_npu=1 rich_tokens_alone_not_pass=1`
- Fixture `testdata/xdna2_cmd/last_apply.json`: `ert_state=5`, no debug_bo
- L5b-shaped string: Dev debug-BO Assign ok + `syncobj=1` + wait_cmd_einval status
- Explicit `wait_path=syncobj_timeline` + `syncobj_handle=`
- Empty → `detail=no_detail`

## T6 — Material EXEC readiness scorecard

- Soft bag: `material_exec_readiness_scorecard`
- Soft always: `invent_pass=0 not_c_ready=1 not_ran_on_npu=1 product_known_good=0 scorecard_alone_not_pass=1 residual_matched_claim=0`
- Inputs: raw bytes, or confined path (`external_known_good` / IRON roots / `testdata/xdna2_cmd` lab fixtures)
- aiebu ELF: peel `.ctrltext`, optional `rela_count` from `.rela.dyn`
- **Locked:** `residual_matched_claim=false` on every constructor path; soft note
  hard-codes `residual_matched_claim=0`
- Bag helper: `format_material_exec_readiness_bag_soft_lines` one soft line per
  `*.ctrltext.bin` / `working_stack_ctrltext.bin` / `*.insts.bin` under capture dir

### Score cases (TDD)

| Case | Result |
|------|--------|
| empty bytes | `empty_refused=1`, distance=1688, residual_matched_claim=0 |
| lab classic `lab_instr.bin` 1688 | classic walk ok, op_count=40, distance=0, size_class=mladf_classic_1688, claim=0 |
| cascade ctrltext 4512 | size_class=other, distance=2824, claim=0 |
| path `/etc/passwd` | `path_refused=1`, claim=0 |

## Gates

```
cargo fmt -p dragon-npu
cargo test --features amd-xdna2 --lib last_apply_rich
cargo test --features amd-xdna2 --lib material_exec_readiness
cargo clippy --features amd-xdna2 --lib -- -D warnings
```

All green (2 unit tests + clippy -D).

## Integration

- Public re-exports in `src/backends/amd_xdna2.rs`
- Pure + unit tests only (no pass_unblock soft print required)
- Optional bag soft lines available for agent-offline tooling later

## Not done / frozen

- No Pass · invent_pass=0 · no `ran_on_npu`
- No unlock mint · no CONSUMED re-live
- residual_matched_claim never true from scorecard alone
- Scorecard / rich tokens alone ≠ load-accepted residual match

## Files

- `src/backends/amd_xdna2/silicon_contract.rs` (T3+T6 pure APIs + unit tests)
- `src/backends/amd_xdna2.rs` (re-exports)
- `.agents/reports/wave4-scorecard-tools.md` (this report)
