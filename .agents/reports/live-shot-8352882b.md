# Live shot report: `C_UNLOCK=8352882b` stock flexml llm-2x4x4 CONFIG_CU (`wait_cmd_einval`)

**Date:** 2026-08-11
**Honesty:** `invent_pass=0` · **no** product Pass · **no** `ran_on_npu` · unlock **CONSUMED**

---

## Plain English: what ran

One stock live CONFIG_CU shot under unlock `8352882b` used the **unspent** flexml STX **llm-2x4x4** research PDI (21664 B, sha16 `ba0cd738fd870011`). This is **not** a re-live of spent unified-2x4x4 (`0eeeb70c84df056e` under `2ae69036`). Structured last_apply was written. Product status is honest **`wait_cmd_einval`**. CONFIG_CU attach measured with the new PDI size. Still no ERT Completed and no OFM allclose.

| Item | Value |
|------|--------|
| Unlock | `C_UNLOCK=8352882b` (one-shot; now **CONSUMED**) |
| Experiment | stock CONFIG_CU with flexml **llm-2x4x4** PDI (**not** skip CONFIG_CU; **not** spent unified) |
| Command | `export XDNA2_LAB_PDI_BLOB=/tmp/dragon_npu_pdi_flexml_llm-2x4x4.bin` then `bash tools/operator_app_load_kit.sh config-cu-run` |
| Outer budget | `XDNA2_HITL_OUTER_TIMEOUT_S=120` |
| Log | `/tmp/dragon_npu_live_shot_8352882b.log` |
| last_apply | `/tmp/dragon_npu_last_apply_config_cu_2328515.json` (2213 B) |
| status | `wait_cmd_einval` |
| `ran_on_npu` | `false` |
| `real_ioctl` | `true` |
| hitl_c | `hitl_c_rc=4` |
| Wall time | `total_s` ≈ 61.57 |
| PDI | flexml llm-2x4x4 **21664** B; sha16=`ba0cd738fd870011` (DIFF vs spent unified 27200 / `0eeeb70c84df056e`; DIFF vs CVML 283840 / `a1e65d04add3422c`) |
| research honesty | `research_pdi_not_product_known_good=1` |

### Measured detail tokens (from last_apply)

- `config_cu=1` (CONFIG_CU attach measured)
- `pdi_bytes=21664` (llm flexml size)
- `cu_func=0`
- `aiebu_elf=1`
- `rela_count=12`
- WAIT_CMD failed: `timeout_ms=5000` then **Invalid argument (os error 22)** → product class **`wait_cmd_einval`**
- bo_diag: DevHeap `xdna_addr=0x4000000`, instr `0x4028000`, host UVAs for IFM/WTS/OFM/CTRL
- tiles=32; qos 100/30; layout=count4
- pack key `mladfmatmulbias_prmptflat_mladf_2x4x4_flat_a16fw4acc16f_1_1152_256_128`

CONFIG_CU attach and aiebu ELF+rela userspace path **did** run with the llm-2x4x4 blob. The job did **not** complete as ERT Completed with non-zero OFM. Product Pass remains **false**.

---

## Journal (this boot / this shot window)

**Window:** 2026-08-11 ~10:43–10:44 (local)

- **`IO_PAGE_FAULT`** (AMD-Vi, domain `0x0001`, flags `0x0007`) at addresses including:
  - `0x4fff80007f00`, `0x4fff80008000`, `0x4fff80009000`, … through `0x4fff8000f000`
  - Cluster around **`0x4fff8000…`** (differs from CVML prior pins near `0x2fff8000…`; **differs from spent flexml unified `2ae69036`**, which had **no** IO_PAGE_FAULT in its shot window)
- **`aie2_error_backtrack`:** Row 0 Col 4 module 2 event 73 cat 8; Row 1 Col 1 module 0 event 137 cat 2
- **`aie2_set_cmd_timeout`:** Context ID 15; DPU PC=`0xffffffff`; TXN OP ID=`0xffffffff`; Context PC=`0x28b1573a`

---

## Residual class for this pin

**Class:** honest WAIT EINVAL (`wait_cmd_einval`) / FW timeout (`aie2_set_cmd_timeout`) / early `aie2_error_backtrack` / **IO_PAGE_FAULT** orphan cluster ~`0x4fff8000…` / flexml research PDI not product matmul known-good.

Do **not** invent locked APP_LOAD (`0x3000003`) from this pin alone (no `ert_error` / no ofm_all_zero ERT Error path on this last_apply). Soft classify: `not_pass_no_ert_error` with `config_cu_present=1`. C4_SKIP / P6_NOT_PASS / invent_pass=0.

Compare prior:

| Unlock | PDI | sha16 | status | IO_PAGE_FAULT in window |
|--------|-----|-------|--------|-------------------------|
| `2ae69036` | unified-2x4x4 27200 | `0eeeb70c84df056e` | wait_cmd_einval | **none** |
| **`8352882b`** | **llm-2x4x4 21664** | **`ba0cd738fd870011`** | **wait_cmd_einval** | **yes ~0x4fff8000…** |
| `97f38307` | CVML 283840 | `a1e65d04add3422c` | wait_cmd_einval | yes ~0x2fff8000… |

---

## Hard honesty

- **Pass = No**
- **invent_pass = 0**
- **ran_on_npu = false**
- Unlock **CONSUMED** — do not reopen; do not second live under `8352882b`
- Do **not** re-live spent `ba0cd738fd870011` or `0eeeb70c84df056e` without a new unlock **and** a new named delta
- Dual public npu2 AOT under `testdata/xdna2_cmd/external_known_good/` remains **RESEARCH ONLY**

## Named offline reason (mint)

Prior CONSUMED shot `2ae69036` proved CONFIG_CU attach on spent unified-2x4x4 then failed with wait_cmd_einval / backtrack. This unlock used unspent flexml topology **llm-2x4x4** (21664 B, sha16 `ba0cd738fd870011`) and was **not** a re-live of `0eeeb70c84df056e`. Capture directory contains public npu2 AOT sets (**research only**, residual-mismatched to flexml and lab classic 1688). Success still required ERT Completed + non-zero OFM allclose via product decide. Unlock ≠ Pass.

## Artifacts

- last_apply: `/tmp/dragon_npu_last_apply_config_cu_2328515.json`
- PDI: `/tmp/dragon_npu_pdi_flexml_llm-2x4x4.bin` (21664 / `ba0cd738fd870011…`)
- live log: `/tmp/dragon_npu_live_shot_8352882b.log`
- impl report: `.agents/reports/impl-feedback-live-llm-2x4x4.md`
- residual SSoT: `RESIDUAL.md` top
