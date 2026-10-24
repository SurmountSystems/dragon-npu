# Live shot report: `C_UNLOCK=48da6e60` stock research debug-BO (`debug_bo_failed`)

**Date:** 2026-08-11
**Honesty:** `invent_pass=0` · **no** product Pass · **no** `ran_on_npu` · unlock **CONSUMED**

---

## Plain English: what ran

One stock live shot under unlock `48da6e60` with:

- **CONFIG_CU skipped** (`XDNA2_LAB_PDI_BLOB` unset, `XDNA2_LAB_SKIP_CONFIG_CU=1`)
- **Research debug-BO enabled** (`XDNA2_LAB_RESEARCH_DEBUG_BO=1`)
- No sticky `XDNA2_ALLOW_EXEC_CMD`

Goal: exercise the W1 lab DBG_BUF attach path on a real device and collect
post-fail evidence. This is **not** a reconfirm of skip-CU under CONSUMED
`1f761bf4` alone and **not** a re-live of spent flexml PDIs. The new named
delta is **research DBG_BUF Assign** on the stock skip-CU baseline.

Product status is honest **`debug_bo_failed`**. Assign failed closed **before**
EXEC, so this shot did **not** re-measure `wait_cmd_einval`. Still no ERT
Completed and no OFM allclose.

| Item | Value |
|------|--------|
| Unlock | `C_UNLOCK=48da6e60` (one-shot; now **CONSUMED**) |
| Experiment | stock **research debug-BO** + skip-CONFIG_CU |
| Command | `export XDNA2_LAB_RESEARCH_DEBUG_BO=1; unset XDNA2_LAB_PDI_BLOB; export XDNA2_LAB_SKIP_CONFIG_CU=1; unset XDNA2_ALLOW_EXEC_CMD; bash tools/operator_app_load_kit.sh config-cu-run-skip` |
| Outer budget | `XDNA2_HITL_OUTER_TIMEOUT_S=120` |
| Log | `/tmp/dragon_npu_live_shot_48da6e60.log` |
| last_apply | `/tmp/dragon_npu_last_apply_config_cu_343412.json` (1131 B) |
| status | `debug_bo_failed` |
| `ran_on_npu` | `false` |
| `real_ioctl` | `true` |
| hitl_c | `hitl_c_rc=4` |
| Wall time | `total_s` ≈ **0.170** (JSON) |
| PDI / CONFIG_CU | **absent** — skip path |
| DBG_BUF | Assign **failed** — bo_handle=**2**, size=**4096**, Shmem, EINVAL (os error 22) |
| research honesty | invent_pass=0 not_c_ready=1 not_ran_on_npu=1 debug_bo_alone_not_pass=1 |

### Measured detail tokens (from last_apply)

- `debug_bo_config op=assign param_type=1 hwctx=1 bo_handle=0 gate_open=1 attempt_ok=0 path=lab`
- Fail message: `CONFIG_HWCTX(assign DBG_BUF) failed hwctx=1 bo=2: Invalid argument (os error 22)`
- tiles=32; qos 100/30
- **no** `aiebu_elf` / rela / layout tokens (fail-closed before instr EXEC path)
- `submit_wait_s` null (no WAIT)

Userspace research gate **did** open and lab **did** CREATE + Assign attempt.
Driver rejected the BO for DBG_BUF. Product Pass remains **false**.

---

## Journal (this boot / this shot window)

**Window:** 2026-08-11 ~13:15:49–13:16:32 (local)

- **`aie2_hwctx_cfg_debug_bo: Get bo 2 failed`** — **yes** (13:16:27)
- **`IO_PAGE_FAULT`:** none in this shot window
- **`aie2_set_cmd_timeout`:** none in this shot window (older flexml lines remain in journal history)

### Interpretation

Driver `aie2_hwctx_cfg_debug_bo` could not resolve BO handle **2** for the
research 4096 B Shmem buffer. Product maps that to CONFIG_HWCTX Assign EINVAL
and fail-closed status `debug_bo_failed`. Optional offline follow-up (not this
unlock): try Dev BO type / different size under a **new** unlock + named reason.
Do **not** re-run the same Shmem-4096 recipe under `48da6e60`.

---

## Residual class for this pin

**Class (SSoT):** research **debug-BO attach fail** — product JSON
`status=debug_bo_failed` after real ioctl Assign EINVAL; **no** EXEC residual
class update on this arm (EXEC not reached).

**Soft residual (classify on last_apply only):** `not_pass_no_ert_error` (no
`ert_error` field; early fail). C3_EVIDENCE body differs from locked APP_LOAD
baseline. C4_SKIP / P6_NOT_PASS / invent_pass=0.

**Compare to skip-CU without DBG_BUF (`1f761bf4`):** that shot reached WAIT and
got `wait_cmd_einval` (~0.17 s). This shot fails **earlier** at DBG_BUF Assign
with similar wall time. DBG_BUF path is live-exercised and currently unusable
with default research params on this stock host.

---

## Hard honesty

- **Pass = No**
- **invent_pass = 0**
- **ran_on_npu = false**
- Unlock **CONSUMED** — do not reopen; do not second live under `48da6e60`
- Do **not** re-live spent flexml sha16s without a new unlock **and** a new named delta
- Debug-BO Assign fail ≠ Pass; Assign success alone would still ≠ Pass
- Dual public npu2 AOT under `testdata/xdna2_cmd/external_known_good/` remains **RESEARCH ONLY**

## Named offline reason that justified this mint (closed by shot)

W0/W1 offline: research `config_hwctx_dbg_buf` fail-closed gate + lab timed
optional attach + O2 WAIT axes. Lab_exec now forwards
`XDNA2_LAB_RESEARCH_DEBUG_BO` explicitly. Shot closed the experiment: stock
Shmem 4096 DBG_BUF Assign fails with driver `Get bo 2 failed` / EINVAL.
