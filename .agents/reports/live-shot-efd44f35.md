# Live shot report: `C_UNLOCK=efd44f35` stock flexml unified-4x4 CONFIG_CU (`wait_cmd_einval`)

**Date:** 2026-08-11
**Honesty:** `invent_pass=0` · **no** product Pass · **no** `ran_on_npu` · unlock **CONSUMED**

---

## Plain English: what ran

One stock live CONFIG_CU shot under unlock `efd44f35` used the **unspent** flexml STX **unified-4x4** research PDI (13792 B, sha16 `7d172bf223a9b06b`). This is **not** a re-live of spent unified-2x4x4 (`0eeeb70c84df056e` under `2ae69036`) and **not** a re-live of spent llm-2x4x4 (`ba0cd738fd870011` under `8352882b` / Shot L). Structured last_apply was written. Product status is honest **`wait_cmd_einval`**. CONFIG_CU attach measured with the new PDI size. Still no ERT Completed and no OFM allclose.

| Item | Value |
|------|--------|
| Unlock | `C_UNLOCK=efd44f35` (one-shot; now **CONSUMED**) |
| Experiment | stock CONFIG_CU with flexml **unified-4x4** PDI (**not** skip CONFIG_CU; **not** spent fingerprints) |
| Command | `export XDNA2_LAB_PDI_BLOB=/tmp/dragon_npu_pdi_flexml_unified-4x4.bin` then `bash tools/operator_app_load_kit.sh config-cu-run` |
| Outer budget | `XDNA2_HITL_OUTER_TIMEOUT_S=120` |
| Log | `/tmp/dragon_npu_live_shot_efd44f35.log` |
| last_apply | `/tmp/dragon_npu_last_apply_config_cu_2618482.json` (2214 B) |
| status | `wait_cmd_einval` |
| `ran_on_npu` | `false` |
| `real_ioctl` | `true` |
| hitl_c | `hitl_c_rc=4` |
| Wall time | `total_s` ≈ 62.12 (JSON); outer ~67 s |
| PDI | flexml unified-4x4 **13792** B; sha16=`7d172bf223a9b06b` (DIFF vs spent unified-2x4x4 27200 / `0eeeb70c84df056e`; DIFF vs spent llm-2x4x4 21664 / `ba0cd738fd870011`; DIFF vs CVML 283840 / `a1e65d04add3422c`) |
| research honesty | `research_pdi_not_product_known_good=1` |

### Measured detail tokens (from last_apply)

- `config_cu=1` (CONFIG_CU attach measured)
- `pdi_bytes=13792` (unified-4x4 flexml size)
- `cu_func=0`
- `aiebu_elf=1`
- `rela_count=12`
- WAIT_CMD failed: `timeout_ms=5000` then **Invalid argument (os error 22)** → product class **`wait_cmd_einval`**
- bo_diag: DevHeap `xdna_addr=0x4000000`, instr `0x4028000`, host UVAs for IFM/WTS/OFM/CTRL
- tiles=32; qos 100/30; layout=count4
- pack key still lab classic `mladfmatmulbias_prmptflat_mladf_2x4x4_flat_a16fw4acc16f_1_1152_256_128` (instruction path); only CONFIG_CU PDI overlay was unified-4x4

CONFIG_CU attach and aiebu ELF+rela userspace path **did** run with the unified-4x4 blob. The job did **not** complete as ERT Completed with non-zero OFM. Product Pass remains **false**.

---

## Journal (this boot / this shot window)

**Window:** 2026-08-11 ~10:48–10:49 (local)

- **`IO_PAGE_FAULT` yes** (AMD-Vi, domain `0x0001`, flags `0x0007`) at addresses including:
  - `0x8fff80007f40`, `0x8fff80008000`, `0x8fff80009000`, `0x8fff80008a40`, … through `0x8fff8000a000`
  - Cluster around **`0x8fff8000…`**
  - Differs from Shot L llm-2x4x4 (`8352882b`) ~`0x4fff8000…`
  - Differs from CVML prior pins ~`0x2fff8000…`
  - Differs from spent flexml unified-2x4x4 `2ae69036` (**no** IO_PAGE_FAULT in that shot window)
  - `amd_iommu_report_page_fault: 180 callbacks suppressed` (more fault traffic than the 9 logged lines)
- **`aie2_set_cmd_timeout`:** Context ID 15; DPU PC=`0xffffffff`; **TXN OP ID=`0xd`** (not `0xffffffff` as on Shot L); Context PC=`0x28b1550b`
- **`aie2_error_backtrack`:** not observed in the filtered shot window (Shot L had backtrack)

---

## Residual class for this pin

**Class:** honest WAIT EINVAL (`wait_cmd_einval`) / FW timeout (`aie2_set_cmd_timeout`) / **IO_PAGE_FAULT** orphan cluster ~`0x8fff8000…` / flexml research PDI not product matmul known-good.

Do **not** invent locked APP_LOAD (`0x3000003`) from this pin alone (no `ert_error` / no ofm_all_zero ERT Error path on this last_apply). Soft classify on live path: `wait_cmd_einval` with `config_cu_present=1`. C4_SKIP / P6_NOT_PASS / invent_pass=0.

Compare prior flexml topology A/B:

| Unlock | PDI | sha16 | status | IO_PAGE_FAULT in window | TXN OP ID |
|--------|-----|-------|--------|-------------------------|-----------|
| `2ae69036` | unified-2x4x4 27200 | `0eeeb70c84df056e` | wait_cmd_einval | **none** | (timeout path) |
| `8352882b` (Shot L) | llm-2x4x4 21664 | `ba0cd738fd870011` | wait_cmd_einval | **yes ~0x4fff8000…** | `0xffffffff` |
| **`efd44f35`** | **unified-4x4 13792** | **`7d172bf223a9b06b`** | **wait_cmd_einval** | **yes ~0x8fff8000…** | **`0xd`** |
| `97f38307` | CVML 283840 | `a1e65d04add3422c` | wait_cmd_einval | yes ~0x2fff8000… | — |

---

## Hard honesty

- **Pass = No**
- **invent_pass = 0**
- **ran_on_npu = false**
- Unlock **CONSUMED** — do not reopen; do not second live under `efd44f35`
- Do **not** re-live spent `7d172bf223a9b06b`, `ba0cd738fd870011`, or `0eeeb70c84df056e` without a new unlock **and** a new named delta
- Dual public npu2 AOT under `testdata/xdna2_cmd/external_known_good/` remains **RESEARCH ONLY**
- CONFIG_CU attach alone ≠ Pass. Unlock ≠ Pass.

## Named offline reason that justified this mint (closed by shot)

Prior CONSUMED `8352882b` used unspent llm-2x4x4 then `wait_cmd_einval` + IO_PAGE_FAULT ~`0x4fff8000…`. Prior CONSUMED `2ae69036` was spent unified-2x4x4 without IO_PAGE_FAULT. This unlock used unspent **unified-4x4** as topology A/B — measured attach + same post-attach class + **new** IO_PAGE_FAULT cluster ~`0x8fff8000…` and TXN OP ID=`0xd`.
