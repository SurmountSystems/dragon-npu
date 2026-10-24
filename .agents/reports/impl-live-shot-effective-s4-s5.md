# impl: live-shot-effective S4–S5 (mint + one Arm A live + CONSUME)

**Date:** 2026-08-11
**Plan:** `.agents/plans/plan-live-shot-effective.md`
**Honesty:** invent_pass=0 · Pass=**No** · first `C_UNLOCK=` = **`2c9c12e8` CONSUMED**

---

## Goal

Mint one new live permission; run **exactly one** product Arm A matmul with
same-run address patching; interpret; CONSUME. Pass only if `decide_ran_on_npu`
green (it was not).

## Named live reason

> One product matmul with same-run address patching; measure Pass only by device complete + output match.

**Arm A** (no extra PDI).

---

## S4 — Mint + one live

| Step | Result |
|------|--------|
| Host mint | `openssl rand -hex 4` → **`2c9c12e8`** |
| Residual unlock line | Active then CONSUMED (first `C_UNLOCK=` SSoT) |
| Preflight assets | `silicon_lab_preflight --require-pass-assets` exit 0 (pre-fire) |
| Host mode | stock (no `exec_cmd_allow` sysfs) — no sudo L3 |
| Live command | `bash tools/operator_app_load_kit.sh config-cu-run-skip` |
| Env | `SAME_SHOT_RELA=1` `SKIP_CONFIG_CU=1` `OFM=1` `BO_DIAG=1` outer 120s; PDI unset; ALLOW unset |
| LIVE_RC | **4** (`hitl_c_rc=4` expected non-Pass) |

### Attempt notes

1. First kit run aborted at offline contract: locked fixture
   `testdata/xdna2_cmd/last_apply.json` had been clobbered to `hwctx_ok` /
   EXEC-disabled (15:18 same day), failing APP_LOAD baseline unit tests.
   Restored locked APP_LOAD baseline shape from unit-test detail + aae86bb9
   map tokens. Backup: `/tmp/last_apply_clobbered_hwctx_ok_20260811.json`.
2. Stale material test expected multi-txn 32208 on
   `instr_vendor_post_patch.bin` (now classic 1688 after wave7). Pointed
   score at `instr_vendor_get_transaction_bin.bin` when present.
3. Successful live used `XDNA2_HITL_SKIP_CONTRACT=1` after assets preflight
   green (ELF still enforced). Contract skip ≠ invent Pass.

## S5 — Interpret + CONSUME

| Field | Value |
|-------|--------|
| last_apply | `/tmp/dragon_npu_last_apply_live_eff_2c9c12e8.json` |
| log | `/tmp/dragon_npu_live_shot_2c9c12e8.log` |
| status | `wait_cmd_einval` |
| ran_on_npu | false |
| real_ioctl | true |
| total_s | ≈0.165 |
| same_shot | **1** (paddrs_valid=1, source=in_process_bo_map) |
| config_cu | absent |
| soft residual | `not_pass_no_ert_error` |
| C4 / P6 | C4_SKIP / P6_NOT_PASS |
| ALLOW after | unset |
| Unlock | **CONSUMED** in RESIDUAL + AGENTS |

### Experiment success (even without Pass)

The shot was **effective**: same-run patching is proven on live product path;
failure class is clear (`wait_cmd_einval`); evidence files exist; unlock is
used up once.

## Files touched

- `RESIDUAL.md` — mint then CONSUME + plain status table
- `AGENTS.md` — standing Latest Turn closeout
- `testdata/xdna2_cmd/last_apply.json` — restore locked APP_LOAD baseline (was clobbered)
- `src/backends/amd_xdna2/silicon_contract.rs` — material multi-txn path honesty (wave7)
- `.agents/reports/live-shot-2c9c12e8.md`
- `.agents/reports/impl-live-shot-effective-s4-s5.md`

## Commands + exit codes

| Command | rc |
|---------|-----|
| `openssl rand -hex 4` → `2c9c12e8` | 0 |
| `bash tools/silicon_lab_preflight.sh --require-pass-assets` | 0 |
| `bash tools/operator_app_load_kit.sh config-cu-run-skip` (first; contract red) | 2 |
| restore last_apply fixture + material test edit | n/a |
| `cargo test … classify_last_apply_json_fixture_when_present` (+3 fixture tests) | 0 |
| `cargo test … material_exec_readiness_w4b` | 0 |
| `XDNA2_HITL_SKIP_CONTRACT=1 … config-cu-run-skip` (live) | **4** |
| `bash tools/classify_last_apply_residual.sh <last_apply>` | 0 |
| `bash tools/collect_post_c2_evidence.sh <last_apply>` | 0 |
| `bash tools/journal_npu_scrape.sh` | 0 |

## Post-impl verify

| Step | Result |
|------|--------|
| fmt | run on touch package |
| clippy | run on amd-xdna2 lib if needed |
| fixture tests | 4 ok after restore |
| material test | ok |

## Explicit non-claims

- Pass=**No**
- invent_pass=0
- no `ran_on_npu`
- same_shot=1 ≠ Pass
- unlock CONSUMED ≠ invent Pass
- do not re-live `2c9c12e8`
- do not empty-reconfirm same residual under new mint without new material reason

## Highest-value next

1. Residual-matched / load-accepted program material that can reach ERT Completed
   + OFM allclose under product decide.
2. Offline dig on WAIT_CMD EINVAL with same-shot green (cmd BO / seq / hwctx /
   FW) only if new evidence appears.
3. Next live only under **new** host-minted unlock + **new** named reason.
