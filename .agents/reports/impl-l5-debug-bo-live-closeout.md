# W2 L5 closeout — research debug-BO one live stock shot

**Date:** 2026-08-11
**Unlock:** `C_UNLOCK=48da6e60` **CONSUMED**
**invent_pass:** **0**
**Pass:** **No**
**ran_on_npu:** **false**

---

## What was asked

Wave W2 L5: one live research debug-BO stock shot under a **new** host-minted
unlock. Named offline reason: W0/W1 delivered DBG_BUF gate + lab timed attach
path + O2 axes. Goal: EXEC residual evidence **with** research DBG_BUF attach.
Not skip-CU reconfirm under CONSUMED `1f761bf4`; not re-live spent flexml.

---

## What ran

1. **Mint:** `openssl rand -hex 4` → `48da6e60`
2. **Pin:** first `C_UNLOCK=` in `RESIDUAL.md` set active with named reason
3. **Prep delta:** `scripts/lab_exec_once.sh` now **explicitly forwards**
   `XDNA2_LAB_RESEARCH_DEBUG_BO` and `XDNA2_LAB_SKIP_CONFIG_CU` into the cargo
   child (plus loud RESEARCH_DEBUG_BO log line)
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
| last_apply | `/tmp/dragon_npu_last_apply_config_cu_343412.json` |
| log | `/tmp/dragon_npu_live_shot_48da6e60.log` |
| status | **`debug_bo_failed`** |
| real_ioctl | true |
| ran_on_npu | false |
| total_s | ≈ 0.170 |
| hitl_c_rc | 4 |
| config_cu | absent (skip path) |
| DBG_BUF | Assign **EINVAL** hwctx=1 bo=**2** size=**4096** Shmem |
| journal | `aie2_hwctx_cfg_debug_bo: Get bo 2 failed` (13:16:27) |
| EXEC / WAIT | **not reached** (fail-closed before EXEC) |

Detail (abbreviated):

```
debug_bo_config op=assign … gate_open=1 attempt_ok=0 path=lab invent_pass=0 …
CONFIG_HWCTX(assign DBG_BUF) failed hwctx=1 bo=2: Invalid argument (os error 22)
```

Soft interpret: residual_class=`not_pass_no_ert_error`; C3_EVIDENCE (body ≠
locked APP_LOAD baseline); C4_SKIP; P6_NOT_PASS. invent_pass=0 always.

---

## Honesty gates

| Claim | Verdict |
|-------|---------|
| Product Pass | **No** |
| invent_pass | **0** |
| ran_on_npu | **false** |
| Unlock active after shot | **No** (CONSUMED) |
| Sticky ALLOW permanent | **No** |
| Second live same unlock | **Forbidden** (not done) |
| Re-live spent flexml | **Not done** |
| Reopen `1f761bf4` | **Not done** |

---

## Research takeaway (not Pass)

Default research DBG_BUF recipe (**4096 B Shmem**) fails Assign on this stock
amdxdna with driver `Get bo 2 failed` / userspace EINVAL. Fail-closed path
worked as designed (status `debug_bo_failed`, no silent continue to EXEC).

Optional **offline** next (no mint from this report alone):

- Inspect public uAPI / XRT `config_ctx_debug_bo` for expected BO type (Dev vs
  Shmem) and size
- Unit/stub already cover gate + attach; any live re-try needs **new** unlock +
  named BO-type/size hypothesis

Highest residual for product Pass remains **load-accepted / residual-matched
program material**, not another DBG_BUF reconfirm of Shmem-4096.

---

## Artifacts

| Path | Role |
|------|------|
| `/home/hunter/Projects/ai/dragon-npu/RESIDUAL.md` | first `C_UNLOCK=48da6e60` CONSUMED |
| `.agents/reports/live-shot-48da6e60.md` | live evidence detail |
| `.agents/reports/impl-l5-debug-bo-live-closeout.md` | this closeout |
| `/tmp/dragon_npu_last_apply_config_cu_343412.json` | structured last_apply |
| `/tmp/dragon_npu_live_shot_48da6e60.log` | full kit log |
| `scripts/lab_exec_once.sh` | explicit RESEARCH_DEBUG_BO forward |

---

## Return summary for parent

- **Unlock hex:** `48da6e60`
- **Consumed?** yes
- **status:** `debug_bo_failed`
- **ran_on_npu:** false
- **Reports:**
  - `/home/hunter/Projects/ai/dragon-npu/.agents/reports/live-shot-48da6e60.md`
  - `/home/hunter/Projects/ai/dragon-npu/.agents/reports/impl-l5-debug-bo-live-closeout.md`
