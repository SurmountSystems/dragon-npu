# Live shot report: `C_UNLOCK=4008e6fd` L5b research debug-BO **Dev** (`wait_cmd_einval`)

**Date:** 2026-08-11
**Honesty:** `invent_pass=0` · **no** product Pass · **no** `ran_on_npu` · unlock **CONSUMED**

---

## Plain English: what ran

One stock live shot under unlock `4008e6fd` with:

- **CONFIG_CU skipped** (`XDNA2_LAB_PDI_BLOB` unset, `XDNA2_LAB_SKIP_CONFIG_CU=1`)
- **Research debug-BO enabled** (`XDNA2_LAB_RESEARCH_DEBUG_BO=1`)
- Product CREATE type **`BufferType::Dev`** 4096 (offline fix after Shmem L5 fail)
- No sticky `XDNA2_ALLOW_EXEC_CMD`

Goal: verify driver accepts **Dev** DBG_BUF Assign (failed on Shmem under
`48da6e60`), then continue into EXEC residual. This is **not** a re-live of
Shmem-4096, **not** spent flexml, and **not** reopen of `48da6e60`.

| Item | Value |
|------|--------|
| Unlock | `C_UNLOCK=4008e6fd` (one-shot; now **CONSUMED**) |
| Experiment | L5b stock **research debug-BO Dev** + skip-CONFIG_CU |
| Command | `export XDNA2_LAB_RESEARCH_DEBUG_BO=1; unset XDNA2_LAB_PDI_BLOB; export XDNA2_LAB_SKIP_CONFIG_CU=1; unset XDNA2_ALLOW_EXEC_CMD; bash tools/operator_app_load_kit.sh config-cu-run-skip` |
| Outer budget | `XDNA2_HITL_OUTER_TIMEOUT_S=120` |
| Log | `/tmp/dragon_npu_live_shot_4008e6fd.log` |
| last_apply | `/tmp/dragon_npu_last_apply_config_cu_821691.json` (2383 B) |
| status | **`wait_cmd_einval`** |
| `ran_on_npu` | `false` |
| `real_ioctl` | `true` |
| hitl_c | `hitl_c_rc=4` |
| Wall time | `total_s` ≈ **0.203** (JSON) |
| PDI / CONFIG_CU | **absent** — skip path (`config_cu_present=0`) |
| DBG_BUF | Assign **ok** — bo_handle=**2**, size=**4096**, **Dev**, `attempt_ok=1` |
| research honesty | invent_pass=0 not_c_ready=1 not_ran_on_npu=1 debug_bo_alone_not_pass=1 |

### Measured detail tokens (from last_apply)

```
debug_bo_config op=assign param_type=1 hwctx=1 bo_handle=2 gate_open=1
  attempt_ok=1 path=lab invent_pass=0 … debug_bo_size=4096 debug_bo_ty=dev
  lab_debug_bo_attach=1
… submit_and_wait failed (wait_cmd_einval): WAIT_CMD failed
  (hwctx=1, seq=1, timeout_ms=5000, request=0xc0106449): Invalid argument
aiebu_elf=1 rela_count=12 content=1688B layout=count4 tiles=32 qos 100/30
instr: ty=Dev xdna_addr=0x4028000
```

Userspace research gate opened; lab CREATE **Dev** + Assign **succeeded**.
EXEC/WAIT then failed with **EINVAL** (same residual class as skip-CU without
DBG_BUF). Product Pass remains **false**.

---

## Contrast vs L5 Shmem (`48da6e60`)

| Axis | L5 `48da6e60` Shmem | L5b `4008e6fd` Dev |
|------|---------------------|---------------------|
| BO type | Shmem 4096 | **Dev** 4096 |
| Assign | **failed** EINVAL | **ok** `attempt_ok=1` |
| status | `debug_bo_failed` | **`wait_cmd_einval`** |
| EXEC reached? | **no** (fail-closed) | **yes** (WAIT EINVAL) |
| Journal Get bo failed | yes (prior shot) | **no new** in L5b window |
| Pass / ran_on_npu | No / false | No / false |

### Interpretation

Driver `aie2_hwctx_cfg_debug_bo` type filter (`AMDXDNA_BO_DEV`) explains the
Shmem fail. **Dev** Assign is live-verified. Residual that remains is post-
attach **WAIT_CMD EINVAL** on the lab classic 1688 aiebu path (not a debug-BO
type bug). Debug-BO attach alone ≠ Pass.

---

## Journal (this shot window)

**Window:** 2026-08-11 ~13:26:32–13:27:37 (local)

- **`aie2_hwctx_cfg_debug_bo: Get bo … failed`:** **none** in L5b window
  (only historical 13:16:27 from spent Shmem `48da6e60`)
- **`IO_PAGE_FAULT`:** none in L5b window
- **`aie2_set_cmd_timeout`:** none in L5b window

---

## Residual class for this pin

**Class (SSoT):** post-attach **`wait_cmd_einval`** after successful research
DBG_BUF **Dev** Assign + skip-CONFIG_CU lab classic EXEC path.

**Soft residual (classify on last_apply):** `not_pass_no_ert_error`;
`wait_cmd_einval_lab_classic_axes` (skip_cu_shape=1 lab_classic_1688=1);
C4_SKIP / P6_NOT_PASS / invent_pass=0.

**Compare to skip-CU without DBG_BUF (`1f761bf4`):** same product status
`wait_cmd_einval`, similar ~0.2 s wall, no CONFIG_CU. L5b adds measured
`debug_bo_ty=dev` attach success without changing EXEC residual class.

---

## Hard honesty

- **Pass = No**
- **invent_pass = 0**
- **ran_on_npu = false**
- Unlock **CONSUMED** — do not reopen; do not second live under `4008e6fd`
- Do **not** re-live Shmem under `48da6e60`
- Do **not** re-live spent flexml sha16s without a new unlock **and** new named delta
- Dev Assign ok ≠ Pass; wait_cmd_einval ≠ Pass

## Named offline reason that justified this mint (closed by shot)

Live `48da6e60` Shmem Assign fail + offline product fix to `BufferType::Dev`
(`.agents/reports/wave2-debug-bo-type-fix.md`). Shot closed the experiment:
**Dev Assign works**; residual remains **wait_cmd_einval** / no `ran_on_npu`.
