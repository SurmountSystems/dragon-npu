# Live shot report: `C_UNLOCK=2ae69036` stock flexml unified-2x4x4 CONFIG_CU (`wait_cmd_einval`)

**Date:** 2026-08-10
**Honesty:** `invent_pass=0` · **no** product Pass · **no** `ran_on_npu` · unlock **CONSUMED**

---

## Plain English: what ran

One stock live CONFIG_CU shot under unlock `2ae69036` used the **flexml STX unified-2x4x4** research PDI (offline fingerprint **DIFF** from the failed CVML dump). Structured last_apply was written. Product status is honest **`wait_cmd_einval`**. CONFIG_CU attach measured with the new PDI size. Still no ERT Completed and no OFM allclose.

| Item | Value |
|------|--------|
| Unlock | `C_UNLOCK=2ae69036` (one-shot; now **CONSUMED**) |
| Experiment | stock CONFIG_CU with flexml preferred PDI (**not** skip CONFIG_CU) |
| Command | `export XDNA2_LAB_PDI_BLOB=/tmp/dragon_npu_pdi_flexml_unified-2x4x4.bin` then `bash tools/operator_app_load_kit.sh config-cu-run` |
| Outer budget | `XDNA2_HITL_OUTER_TIMEOUT_S=120` |
| Log | `/tmp/dragon_npu_live_shot_2ae69036.log` |
| last_apply | `/tmp/dragon_npu_last_apply_config_cu_675133.json` (2214 B) |
| status | `wait_cmd_einval` |
| `ran_on_npu` | `false` |
| `real_ioctl` | `true` |
| hitl_c | `hitl_c_rc=4` |
| Wall time | `total_s` ≈ 62.59 |
| PDI | flexml unified-2x4x4 **27200** B; sha16=`0eeeb70c84df056e` (DIFF vs CVML 283840 / `a1e65d04add3422c`) |
| research honesty | `research_pdi_not_product_known_good=1` |

### Measured detail tokens (from last_apply)

- `config_cu=1` (CONFIG_CU attach measured)
- `pdi_bytes=27200` (flexml size; **not** the prior CVML 283840)
- `cu_func=0`
- `aiebu_elf=1`
- `rela_count=12`
- WAIT_CMD failed immediately: `timeout_ms=5000` then **Invalid argument (os error 22)** → product class **`wait_cmd_einval`**
- bo_diag: DevHeap `xdna_addr=0x4000000`, instr `0x4028000`, host UVAs for IFM/WTS/OFM/CTRL
- tiles=32; qos 100/30; layout=count4

CONFIG_CU attach and aiebu ELF+rela userspace path **did** run with the flexml blob. The job did **not** complete as ERT Completed with non-zero OFM. Product Pass remains **false**.

---

## Journal (this boot / this shot window)

**Window:** 2026-08-10 ~12:52–12:53

- **`aie2_error_backtrack`:** Row 0, Col 4, module 2, event ID 73, category 8
- **`aie2_set_cmd_timeout`:** Context ID 15; DPU PC=`0xffffffff`; TXN OP ID=`0xffffffff`; Context PC=`0x28b15445`
- **No `IO_PAGE_FAULT`** lines in this shot window (differs from CVML pins under `97f38307` / `11bc9016`, which logged orphan faults near `0x2fff8000…`)

---

## Residual class for this pin

**Class:** honest WAIT EINVAL (`wait_cmd_einval`) / FW timeout (`aie2_set_cmd_timeout`) / early `aie2_error_backtrack` / flexml research PDI not product matmul known-good.

Do **not** invent locked APP_LOAD (`0x3000003`) from this pin alone (no `ert_error` / no ofm_all_zero ERT Error path on this last_apply). Soft classify: `not_pass_no_ert_error` with `config_cu_present=1` and `pdi_bytes=27200`.

Offline p5 / c3 / c4 / p6 on this evidence: **not** Pass; invent_pass=0; C3 evidence (body differs from baseline); C4_SKIP / P6_NOT_PASS / P6_SKIP.

### What changed vs CVML research-blob shot

| | CVML (`97f38307`) | flexml (`2ae69036`) |
|--|-------------------|---------------------|
| pdi_bytes | 283840 | **27200** |
| sha16 | `a1e65d04add3422c` | **`0eeeb70c84df056e`** |
| product status | `wait_cmd_einval` | `wait_cmd_einval` (same) |
| IO_PAGE_FAULT | yes (~`0x2fff80007f00`) | **no** in this window |
| FW timeout log | yes | yes (`aie2_set_cmd_timeout`) |
| Pass / ran_on_npu | no | no |

Progress note: different PDI size attached; fault signature shifted (no orphan IO_PAGE_FAULT this time). Still **not** product Pass.

---

## Unlock bookkeeping

**Mark `C_UNLOCK=2ae69036` CONSUMED.**

- One-shot was for a real live CONFIG_CU + flexml preferred PDI stock path.
- Real attempt completed far enough to write last_apply and hit WAIT_CMD EINVAL.
- Unlock ≠ Pass. Consuming means **do not** re-open under `2ae69036`.
- Prior CONSUMED stay historical only: `97f38307`, `11bc9016`, `3a70fabf`, `b17b0119`.
- Skip-PDI was **not** used. Second live under this unlock is **forbidden**.

---

## Not Pass

| Claim | Value |
|-------|--------|
| Product Pass | **No** |
| `ran_on_npu` | **No** (`false`) |
| invent_pass | **0** |
| CONFIG_CU attach alone | research progress only, not Pass |
| Flexml PDI | research / not product matmul known-good |
| Offline C4 / P6 candidate | still requires live decide for product Pass |

---

## Who acts next

| Who | Next |
|-----|------|
| **Agent** | Only if a **named offline delta** is real: capture bytes under `testdata/xdna2_cmd/external_known_good/`, true matmul-matched PDI/ctrltext source, or driver/MMU work. Else **stop**. No empty reconfirm mill. |
| **Human** | Optional real capture under `testdata/xdna2_cmd/external_known_good/` when public-stack bytes exist. Do not reopen consumed unlocks. |
| **Live again** | Only after new named offline reason + **new** host-minted unlock. |

---

## Paths to keep

- Live log: `/tmp/dragon_npu_live_shot_2ae69036.log`
- last_apply pin: `/tmp/dragon_npu_last_apply_config_cu_675133.json`
- PDI blob: `/tmp/dragon_npu_pdi_flexml_unified-2x4x4.bin` (27200 / `0eeeb70c84df056e`)
- Residual SSoT: `RESIDUAL.md` top Unlock / plain status
- Closeout: `.agents/reports/impl-u4-flexml-live-closeout.md`
