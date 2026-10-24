# Live shot report: `C_UNLOCK=2c9c12e8` Arm A product matmul + same-shot (`wait_cmd_einval`)

**Date:** 2026-08-11
**Honesty:** `invent_pass=0` · **no** product Pass · **no** `ran_on_npu` · unlock **CONSUMED**

---

## Plain English: what ran

One stock product matmul live under unlock `2c9c12e8` with **Arm A** checklist:

- Same-run address patching on (`XDNA2_LAB_SAME_SHOT_RELA=1`)
- No research PDI / skip CONFIG_CU
- OFM path + BO diagnostics + outer budget ≥120s
- New last_apply under `/tmp` (not locked fixture)

Userspace built ELF+rela×12, patched from **this process’s** BO paddrs
(`same_shot=1 paddrs_valid=1 source=in_process_bo_map`), submitted EXEC, then
WAIT_CMD returned **EINVAL**. Product status is honest **`wait_cmd_einval`**.
Still no ERT Completed and no OFM allclose.

| Item | Value |
|------|--------|
| Unlock | `C_UNLOCK=2c9c12e8` (one-shot; now **CONSUMED**) |
| Experiment | Arm A product matmul + same-shot (skip CONFIG_CU) |
| Named reason | One product matmul with same-run address patching; measure Pass only by device complete + output match. |
| Command | `unset XDNA2_LAB_PDI_BLOB; export XDNA2_LAB_SKIP_CONFIG_CU=1 XDNA2_LAB_SAME_SHOT_RELA=1 XDNA2_LAB_OFM=1 XDNA2_LAB_BO_DIAG=1 XDNA2_HITL_OUTER_TIMEOUT_S=120 XDNA2_HITL_SKIP_CONTRACT=1; bash tools/operator_app_load_kit.sh config-cu-run-skip` |
| Outer budget | `timeout-s=120` / `XDNA2_HITL_OUTER_TIMEOUT_S=120` |
| Log | `/tmp/dragon_npu_live_shot_2c9c12e8.log` |
| last_apply | `/tmp/dragon_npu_last_apply_live_eff_2c9c12e8.json` (2937 B) |
| status | `wait_cmd_einval` |
| `ran_on_npu` | `false` |
| `real_ioctl` | `true` |
| hitl_c | `hitl_c_rc=4` |
| Wall time | `total_s` ≈ **0.165** |
| PDI / CONFIG_CU | **absent** — `config_cu_present=0` |
| Same-shot | `same_shot_gate=1 same_shot=1 paddrs_valid=1 source=in_process_bo_map refuse_stale_disk=1` |

### Measured detail tokens (from last_apply)

- **no** `config_cu=` / **no** `pdi_bytes=` (skip path confirmed)
- `aiebu_elf=1`, `rela_count=12`, rela_mode=overwrite, inst_size_mode=content
- content/start_npu=1688 B; bo_size=4096 B; layout=count4; tiles=32; qos 100/30
- WAIT_CMD failed: `timeout_ms=5000` then **Invalid argument (os error 22)** → product class **`wait_cmd_einval`**
- Same-shot plan tokens present and greppable at head of detail
- bo_diag: DevHeap `xdna_addr=0x4000000`; instr `0x4020000`; host UVAs for IFM/WTS/OFM/CTRL valid
- pack key lab classic `mladfmatmulbias_prmptflat_mladf_2x4x4_flat_a16fw4acc16f_1_1152_256_128`

### Soft classify / C4 / P6

- Soft residual: `not_pass_no_ert_error` (no `ert_error` field on this fail path)
- C4_SKIP / P6_NOT_PASS / invent_pass=0
- l1b_live_justified=0 (residual match still 0)

---

## Journal (scrape)

`journal_npu_scrape` soft tokens include historical IO_PAGE_FAULT / aie2_set_cmd_timeout /
debug_bo Get bo failed from **earlier** shots today (~10:43–13:16). This Arm A shot
ran ~17:01 with total_s≈0.16s — same short baseline class as skip-CU `1f761bf4`
(no multi-minute attach). **Do not** treat older flexml journal bands as this shot’s
root class. Product SSoT for this pin is last_apply `wait_cmd_einval` + same_shot=1.

---

## Residual class for this pin

**Class (SSoT):** honest WAIT EINVAL on lab classic START_NPU **with same-run
address patching proven**, without CONFIG_CU overlay.

**Finding:** Same-shot path **works live** (in-process paddrs, refuse stale disk).
Product residual **class does not change** vs skip-CU `1f761bf4` — still
`wait_cmd_einval`. Same-shot is necessary hygiene, not sufficient for Pass.

| Unlock | Overlay | Same-shot | status | total_s |
|--------|---------|-----------|--------|---------|
| `1f761bf4` | skip CONFIG_CU | (pre same-shot kit default) | wait_cmd_einval | ~0.17 |
| **`2c9c12e8`** | **skip CONFIG_CU** | **same_shot=1 measured** | **wait_cmd_einval** | **~0.165** |

---

## Hard honesty

- **Pass = No**
- **invent_pass = 0**
- **ran_on_npu = false**
- Unlock **CONSUMED** — do not reopen; do not second live under `2c9c12e8`
- Do **not** re-live as empty same-shot reconfirm
- Sticky ALLOW was **unset** after shot
- Offline contract was skipped on the successful apply (`XDNA2_HITL_SKIP_CONTRACT=1`)
  after prep assets green and locked fixture restore; ELF + preflight still enforced
