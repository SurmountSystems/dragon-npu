# Live shot report: `C_UNLOCK=1f761bf4` stock skip-CONFIG_CU isolation (`wait_cmd_einval`)

**Date:** 2026-08-11
**Honesty:** `invent_pass=0` · **no** product Pass · **no** `ran_on_npu` · unlock **CONSUMED**

---

## Plain English: what ran

One stock live shot under unlock `1f761bf4` with **CONFIG_CU skipped**:
`XDNA2_LAB_PDI_BLOB` unset, `XDNA2_LAB_SKIP_CONFIG_CU=1`, no sticky
`XDNA2_ALLOW_EXEC_CMD`. Goal: isolate lab classic START_NPU EXEC residual from
flexml/CVML overlay residual after three spent flexml CONFIG_CU attach shots.
Structured last_apply was written. Product status is honest **`wait_cmd_einval`**.
**No** CONFIG_CU attach tokens. Still no ERT Completed and no OFM allclose.

| Item | Value |
|------|--------|
| Unlock | `C_UNLOCK=1f761bf4` (one-shot; now **CONSUMED**) |
| Experiment | stock **skip-CONFIG_CU isolation** (no research PDI overlay) |
| Command | `unset XDNA2_LAB_PDI_BLOB; export XDNA2_LAB_SKIP_CONFIG_CU=1; unset XDNA2_ALLOW_EXEC_CMD; bash tools/operator_app_load_kit.sh config-cu-run-skip` |
| Outer budget | `XDNA2_HITL_OUTER_TIMEOUT_S=120` |
| Log | `/tmp/dragon_npu_live_shot_1f761bf4.log` |
| last_apply | `/tmp/dragon_npu_last_apply_config_cu_4080903.json` (2135 B) |
| status | `wait_cmd_einval` |
| `ran_on_npu` | `false` |
| `real_ioctl` | `true` |
| hitl_c | `hitl_c_rc=4` |
| Wall time | `total_s` ≈ **0.172** (JSON); wall clock ~5 s including offline contract |
| PDI / CONFIG_CU | **absent** — `config_cu_present=0`; no `pdi_bytes`; research_pdi_identity source=absent |
| research honesty | `research_pdi_not_product_known_good=1 invent_pass=0` |

### Measured detail tokens (from last_apply)

- **no** `config_cu=` / **no** `pdi_bytes=` / **no** `cu_func=` in detail (skip path confirmed)
- `aiebu_elf=1`, `rela_count=12`, rela_mode=overwrite, inst_size_mode=content
- content/start_npu=1688 B; bo_size=4096 B; layout=count4; tiles=32; qos 100/30
- WAIT_CMD failed: `timeout_ms=5000` then **Invalid argument (os error 22)** → product class **`wait_cmd_einval`**
- bo_diag: DevHeap `xdna_addr=0x4000000`; instr `0x4020000` (note: prior CONFIG_CU flexml shots often instr `0x4028000`); host UVAs for IFM/WTS/OFM/CTRL valid
- pack key still lab classic `mladfmatmulbias_prmptflat_mladf_2x4x4_flat_a16fw4acc16f_1_1152_256_128`

Userspace ELF+rela path **did** run without CONFIG_CU. The job did **not** complete as ERT Completed with non-zero OFM. Product Pass remains **false**.

---

## Journal (this boot / this shot window)

**Window:** 2026-08-11 ~11:15:41–11:15:46 (local)

- **`IO_PAGE_FAULT`:** **none** observed in filtered journal / dmesg window
- **`aie2_set_cmd_timeout`:** **none** observed in window
- **`aie2_error_backtrack`:** **none** observed in window
- Differs from CONFIG_CU flexml llm (`8352882b` ~`0x4fff8000…`) and unified-4x4 (`efd44f35` ~`0x8fff8000…`) which logged IO_PAGE_FAULT + cmd timeout over ~62 s total
- Differs from spent unified-2x4x4 `2ae69036` (no IO_PAGE_FAULT, but still ~62 s + CONFIG_CU attach)

---

## Residual class for this pin

**Class (SSoT):** honest WAIT EINVAL — product JSON **`status=wait_cmd_einval`** on lab classic START_NPU **without** CONFIG_CU overlay.

**Final soft residual (classify on last_apply only):** `not_pass_no_ert_error` (no `ert_error` field). C4_SKIP / P6_NOT_PASS / invent_pass=0.

**Log hygiene:** `/tmp/dragon_npu_live_shot_1f761bf4.log` may show **early** soft classify on the **locked fixture** `testdata/xdna2_cmd/last_apply.json` (`app_load_pdi_fail_after_aiebu` / historical `ert_cmd_error`) during offline contract preflight. That early soft is **not** residual-class SSoT for this shot. Use only final tokens on `/tmp/dragon_npu_last_apply_config_cu_4080903.json`. p5 footer unlock bookkeeping may name stale historical stamps; first-line parse SSoT is `RESIDUAL.md` top (`1f761bf4` CONSUMED). Do **not** re-live.

### Isolation vs prior CONFIG_CU attach shots

| Unlock | Overlay | status | config_cu | total_s | IO_PAGE_FAULT in window |
|--------|---------|--------|-----------|---------|-------------------------|
| `2ae69036` | unified-2x4x4 CONFIG_CU | wait_cmd_einval | **1** | ~62.6 | **none** |
| `8352882b` | llm-2x4x4 CONFIG_CU | wait_cmd_einval | **1** | ~61.6 | **yes ~0x4fff…** |
| `efd44f35` | unified-4x4 CONFIG_CU | wait_cmd_einval | **1** | ~62.1 | **yes ~0x8fff…** |
| **`1f761bf4`** | **skip CONFIG_CU** | **wait_cmd_einval** | **0** | **~0.17** | **none** |

**Finding:** product residual **class does not change** when CONFIG_CU is skipped
(still `wait_cmd_einval`). Overlay is **not** the sole cause of EXEC residual.
Overlay **can** add orphan IO_PAGE_FAULT bands and long attach wall time.
Skip path is a clean, short baseline without fault journal noise.

---

## Hard honesty

- **Pass = No**
- **invent_pass = 0**
- **ran_on_npu = false**
- Unlock **CONSUMED** — do not reopen; do not second live under `1f761bf4`
- Do **not** re-live spent flexml sha16s without a new unlock **and** a new named delta
- Skip-CU isolation measured ≠ Pass
- Dual public npu2 AOT under `testdata/xdna2_cmd/external_known_good/` remains **RESEARCH ONLY**

## Named offline reason that justified this mint (closed by shot)

Prior flexml lives attached then `wait_cmd_einval`; offline map audit orphan-only;
offline load-path ranked skip-CONFIG_CU as the next live experiment. Shot closed
the experiment: EXEC residual remains without overlay.
