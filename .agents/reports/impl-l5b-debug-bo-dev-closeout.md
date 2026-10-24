# W2 L5b closeout — research debug-BO **Dev** one live stock shot

**Date:** 2026-08-11
**Unlock:** `C_UNLOCK=4008e6fd` **CONSUMED**
**invent_pass:** **0**
**Pass:** **No**
**ran_on_npu:** **false**

---

## What was asked

Wave W2 L5b: one live research debug-BO stock shot **after** offline Dev type
fix. Named offline reason: live `48da6e60` failed Assign with Shmem 4096;
driver requires `AMDXDNA_BO_DEV`; product now CREATE `BufferType::Dev` 4096.
Goal: verify Dev Assign, then measure EXEC residual. Not re-live Shmem; not
spent flexml; not reopen `48da6e60`.

---

## What ran

1. **Mint:** `openssl rand -hex 4` → `4008e6fd`
2. **Pin:** first `C_UNLOCK=` in `RESIDUAL.md` set active with named reason
3. **Product (already landed offline):** `LAB_RESEARCH_DEBUG_BO_TYPE = BufferType::Dev`
   (unit `lab_research_debug_bo_create_recipe_is_dev_page` green)
4. **Live (one shot only):**
   ```bash
   export XDNA2_LAB_RESEARCH_DEBUG_BO=1
   unset XDNA2_LAB_PDI_BLOB
   export XDNA2_LAB_SKIP_CONFIG_CU=1
   unset XDNA2_ALLOW_EXEC_CMD
   export XDNA2_HITL_OUTER_TIMEOUT_S=120
   bash tools/operator_app_load_kit.sh config-cu-run-skip
   ```
5. **Consume:** unlock marked CONSUMED in `RESIDUAL.md` after attempt
6. **Interpret:** p5 / classify / c3 / c4 / p6 on last_apply (all invent_pass=0)

### Device / host

| Check | Result |
|-------|--------|
| `/dev/accel/accel0` | present |
| `exec_cmd_allow` sysfs | **missing** (stock; no sudo L3) |
| Sticky ALLOW after | **unset** |

---

## Measured outcome

| Field | Value |
|-------|--------|
| last_apply | `/tmp/dragon_npu_last_apply_config_cu_821691.json` |
| log | `/tmp/dragon_npu_live_shot_4008e6fd.log` |
| status | **`wait_cmd_einval`** |
| real_ioctl | true |
| ran_on_npu | false |
| total_s | ≈ 0.203 |
| hitl_c_rc | 4 |
| config_cu | absent (skip path) |
| DBG_BUF | Assign **ok** hwctx=1 bo=**2** size=**4096** **Dev** `attempt_ok=1` |
| journal L5b window | **no** `Get bo failed`; **no** IO_PAGE_FAULT / cmd_timeout |
| EXEC / WAIT | **reached** — WAIT_CMD EINVAL (hwctx=1 seq=1 timeout_ms=5000) |

Detail (abbreviated):

```
debug_bo_config … attempt_ok=1 … debug_bo_size=4096 debug_bo_ty=dev lab_debug_bo_attach=1
… submit_and_wait failed (wait_cmd_einval): WAIT_CMD … Invalid argument (os error 22)
```

Soft interpret: residual_class=`not_pass_no_ert_error`; wait_cmd_einval lab
classic axes (skip_cu + 1688 + aiebu+rela); C4_SKIP; P6_NOT_PASS. invent_pass=0.

---

## Honesty gates

| Claim | Verdict |
|-------|---------|
| Product Pass | **No** |
| invent_pass | **0** |
| ran_on_npu | **false** |
| Debug-BO Assign ok | **Yes** (Dev) |
| Unlock active after shot | **No** (CONSUMED) |
| Sticky ALLOW permanent | **No** |
| Second live same unlock | **Forbidden** (not done) |
| Re-live Shmem / spent flexml | **Not done** |
| Reopen `48da6e60` | **Not done** |

---

## Research takeaway (not Pass)

| Finding | Meaning |
|---------|---------|
| Shmem-4096 Assign fails live | Matches driver type filter (`AMDXDNA_BO_DEV`) |
| Dev-4096 Assign succeeds live | Offline type fix **verified** |
| EXEC residual unchanged | Still `wait_cmd_einval` on skip-CU lab classic path |
| invent_pass | **0** always |

**Next (not from this unlock alone):** residual-matched / load-accepted material
for FW accept; optional offline WAIT EINVAL dig with DBG_BUF attached is
research-only. **Do not** second live under `4008e6fd`. **Do not** invent Pass
from Dev Assign success.

---

## Report paths

- `.agents/reports/live-shot-4008e6fd.md`
- `.agents/reports/impl-l5b-debug-bo-dev-closeout.md` (this file)
- Prior contrast: `.agents/reports/live-shot-48da6e60.md`,
  `.agents/reports/wave2-debug-bo-type-fix.md`
- `RESIDUAL.md` top (first `C_UNLOCK=` CONSUMED)
