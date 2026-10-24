# Closeout: Phase 3 skip-CONFIG_CU isolation (`C_UNLOCK=1f761bf4`)

**Date:** 2026-08-11
**Mode:** one deliberate live + CONSUME + interpret. invent_pass=0.
**Pass?** **No.** `ran_on_npu`? **false.**

---

## What we did

1. Confirmed first `C_UNLOCK=` was `efd44f35` CONSUMED (not reopened).
2. Host-minted **`NEW=1f761bf4`** via `openssl rand -hex 4`.
3. Activated unlock in `RESIDUAL.md` with named offline reason from join report
   (skip-CONFIG_CU after orphan map audit; not re-live spent flexml).
4. Offline prep: `bash tools/operator_app_load_kit.sh config-cu-skip` (rc=0).
5. Env: unset PDI; `XDNA2_LAB_SKIP_CONFIG_CU=1`; unset ALLOW.
6. One stock live: `bash tools/operator_app_load_kit.sh config-cu-run-skip`
   → `LIVE_RC=4` (`hitl_c_rc=4` expected non-Pass).
7. Post: p5 interpret, collect evidence, classify soft — all invent_pass=0.
8. Marked unlock **CONSUMED** with measured outcomes; residual + AGENTS pins.

## Measured outcomes

| Field | Value |
|-------|--------|
| last_apply | `/tmp/dragon_npu_last_apply_config_cu_4080903.json` |
| status | `wait_cmd_einval` |
| config_cu | **absent** (`config_cu_present=0`) |
| pdi_bytes | **absent** |
| real_ioctl | true |
| ran_on_npu | false |
| total_s | ≈0.172 |
| journal | no IO_PAGE_FAULT; no aie2_set_cmd_timeout in window |
| soft residual (final) | **`not_pass_no_ert_error`** on last_apply `/tmp/dragon_npu_last_apply_config_cu_4080903.json` (SSoT for soft class) |
| C4 / P6 | C4_SKIP / P6_NOT_PASS |
| early log soft | **not SSoT** — log mid-stream may classify locked fixture `app_load_pdi_fail_after_aiebu`; ignore for this shot |

## Isolation finding (plain English)

**Skipping CONFIG_CU did not change the product residual class.** Lab classic
START_NPU alone still fails WAIT_CMD with EINVAL (`wait_cmd_einval`). Prior
flexml CONFIG_CU shots shared that class but added long attach time (~62 s)
and, on llm/4x4, orphan IO_PAGE_FAULT journal bands. Overlay residual is
**additive**, not the sole root of EXEC failure.

## Highest-value next residual

1. **Load-accepted / residual-matched program material** (working-stack path
   that can reach ERT Completed + non-zero OFM allclose) — not another skip-CU
   reconfirm.
2. Offline dig: **why WAIT_CMD returns EINVAL** immediately on lab classic
   alone (cmd BO / seq / hwctx / FW state without CU). Hypothesis only until
   evidence.
3. Next live only under a **new** host-minted unlock + **new** named offline
   reason. Freeze re-live of spent flexml and skip-CU under CONSUMED unlocks.

## Files touched

- `RESIDUAL.md` (first unlock CONSUMED + plain status table)
- `AGENTS.md` (Latest Turn mint + closeout)
- `.agents/reports/live-shot-1f761bf4.md`
- `.agents/reports/impl-t-skip-cu-1f761bf4-closeout.md`
- Log: `/tmp/dragon_npu_live_shot_1f761bf4.log`

## Commands + exit codes (summary)

| Command | rc |
|---------|-----|
| `openssl rand -hex 4` → `1f761bf4` | 0 |
| `bash tools/pass_unblock_once.sh --status` (pre, after mint fix) | 0; active unlock shown |
| `bash tools/operator_app_load_kit.sh config-cu-skip` | 0 |
| `bash tools/operator_app_load_kit.sh config-cu-run-skip` | **4** (hitl_c non-Pass) |
| `bash tools/p5_post_c2_interpret.sh <last_apply>` | 0 |
| `bash tools/collect_post_c2_evidence.sh <last_apply>` | 0 |
| `bash tools/classify_last_apply_residual.sh <last_apply>` | 0 |
| `bash tools/pass_unblock_once.sh --status` (post CONSUME) | (run after pin) |


## Operator offline re-check (no re-live)

One-liner after this pin (read-only interpret; **ban** stock live under CONSUMED):

```bash
bash tools/pass_unblock_once.sh --status \
  && bash tools/p5_post_c2_interpret.sh /tmp/dragon_npu_last_apply_config_cu_4080903.json \
  && bash tools/classify_last_apply_residual.sh /tmp/dragon_npu_last_apply_config_cu_4080903.json
```

Expect: Unlock none / first `C_UNLOCK=1f761bf4` CONSUMED; soft residual
`not_pass_no_ert_error`; invent_pass=0; not_ran_on_npu=1. Do **not** run
`config-cu-run-skip` or reopen CONSUMED.

## Honesty lock

| Claim | Truth |
|-------|--------|
| Product Pass | **No** |
| invent_pass | **0** |
| ran_on_npu | **false** |
| Skip-CU = Pass | **No** |
| Reopen CONSUMED | **Banned** |
| Re-live spent flexml | **Banned** |
| Sticky ALLOW | **No** (unset) |
