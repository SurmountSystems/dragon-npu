# impl: live-shot-effective S0–S3 prep

**Date:** 2026-08-11
**Plan:** `.agents/plans/plan-live-shot-effective.md`
**Honesty:** invent_pass=0 · Pass=**No** · unlock **none** · first `C_UNLOCK=` = **`4008e6fd` CONSUMED**
**Live / mint:** **not** this slice (S4–S5 out of scope)

---

## Goal

Make option 1 (one product matmul with same-run address patching) hard to misconfigure before a justified live shot: residual pin, kit/lab_exec same-run flag, loud last_apply tokens, offline preflight green.

## Named live reason (for S4 only — not minted)

> One product matmul with same-run address patching; measure Pass only by device complete + output match.

**Arm A default** (no extra PDI).

---

## S0 — Residual / AGENTS pin

- Prepended Latest Turn on `RESIDUAL.md` and short standing Latest Turn on `AGENTS.md`.
- Plan **live-shot-effective** APPROVED; S0–S3 prep; unlock **none**; do not mint; do not reopen `4008e6fd`.
- Reports point here + plan path.

## S1 — Kit + lab_exec same-run flag

| File | Change |
|------|--------|
| `scripts/lab_exec_once.sh` | Forward `XDNA2_LAB_SAME_SHOT_RELA` in `RUN_ENV` into cargo child; loud preflight echo when set; EXEC line shows same_shot_rela |
| `tools/hitl_config_cu_once.sh` | `ensure_same_shot_rela` defaults to `1` when unset/empty; preflight + all sandwich pastes export it; run-stock calls it |
| `tools/hitl_c_pass_apply.sh` | Defaults `XDNA2_LAB_SAME_SHOT_RELA=1` when unset; loud banner + RESIDUAL_ENV line |
| `tools/operator_app_load_kit.sh` | Overview Path A-skip documents Arm A same-shot default + ≥120s |

## S2 — Louder same_shot= tokens when gate on

| Item | Detail |
|------|--------|
| APIs | `SAME_SHOT_LAST_APPLY_GATE_NOTE`, `format_same_shot_last_apply_gate_tokens`, `format_same_shot_last_apply_detail` |
| Live path | `direct.rs` uses `format_same_shot_last_apply_detail` when `XDNA2_LAB_SAME_SHOT_RELA` gate is on (fail + success detail paths share the plan note builder) |
| Tokens | Leading: `same_shot_gate=1 same_shot=0\|1 … invent_pass=0 not_ran_on_npu=1` then full plan note |
| TDD | `w10_b_same_shot_last_apply_gate_tokens_loud` — GREEN with other `w10_b_*` |

Soft classify still offline `l1b_live_justified=0` — not claimed.

## S3 — Offline preflight green

| Check | Result |
|-------|--------|
| `bash tools/silicon_lab_preflight.sh --require-pass-assets` | exit 0; instr+wts+ctrl+golden OK; DRY_RUN_READY=1 |
| Sandwich print (`prep-skip-pdi`) | `SAME_SHOT_RELA=1`, `export XDNA2_LAB_SAME_SHOT_RELA=1`, `--timeout-s=120` |
| `cargo test --features amd-xdna2 --lib w10_b_` | 5 passed |
| `bash -n` touched scripts | BASH_N_OK |
| `cargo fmt -p dragon-npu` | clean |
| `cargo clippy -p dragon-npu --features amd-xdna2 --lib -- -D warnings` | exit 0 |

---

## Explicit non-claims

- unlock still **none** (no mint)
- Pass=**No**
- invent_pass=0
- no `ran_on_npu`
- prep ≠ Pass ≠ live justified
- public AOT re-run still frozen
- stale disk UVAs still forbidden for product live patch

## Next (not this slice)

- **S4:** host-mint new `C_UNLOCK` + one product live under checklist (Arm A) only if plan live step still in scope
- **S5:** collect evidence, classify, CONSUME, residual pin; Pass only if decide green
