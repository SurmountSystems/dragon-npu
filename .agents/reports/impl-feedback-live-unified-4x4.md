# Implement feedback: live stock flexml unified-4x4 (`efd44f35`)

**Date:** 2026-08-11
**Unlock:** `C_UNLOCK=efd44f35` **CONSUMED**
**Honesty:** invent_pass=0 · Pass=No · ran_on_npu=false · Unlock ≠ Pass

## What was done

1. Reused existing dump `/tmp/dragon_npu_pdi_flexml_unified-4x4.bin` (dump tool needs `--xclbin`/`--out`; bytes already present).
2. Verified size **13792**, sha16 **`7d172bf223a9b06b`**. Refused spent `0eeeb70c84df056e` / `ba0cd738fd870011`.
3. Minted unlock `efd44f35` via `openssl rand -hex 4`. First `C_UNLOCK=` in `RESIDUAL.md` set **active** (avoided the word “CONSUMED” on that line so `require_active_unlock` would not false-positive).
4. One stock live: `XDNA2_LAB_PDI_BLOB=/tmp/dragon_npu_pdi_flexml_unified-4x4.bin` + `bash tools/operator_app_load_kit.sh config-cu-run`. Finite outer 120s; ALLOW unset; single shot.
5. Interpreted last_apply + journal; p5/c3/c4/p6 ran via collect path. invent_pass=0.
6. **CONSUMED** unlock; ledger marks unified-4x4 **Spent**.

## Live outcome

| Field | Value |
|-------|--------|
| last_apply | `/tmp/dragon_npu_last_apply_config_cu_2618482.json` |
| status | `wait_cmd_einval` |
| config_cu | `1` |
| pdi_bytes | **13792** |
| sha16 | `7d172bf223a9b06b` |
| real_ioctl | true |
| ran_on_npu | false |
| Pass | **No** |
| total_s | ≈62.12 |
| hitl_c_rc | 4 |
| log | `/tmp/dragon_npu_live_shot_efd44f35.log` |

## Residual class vs Shot L (`8352882b`)

| | Shot L llm-2x4x4 | This shot unified-4x4 |
|--|------------------|------------------------|
| Unlock | `8352882b` CONSUMED | `efd44f35` CONSUMED |
| pdi_bytes / sha16 | 21664 / `ba0cd738…` | 13792 / `7d172bf2…` |
| status | wait_cmd_einval | wait_cmd_einval |
| IO_PAGE_FAULT | **yes** ~`0x4fff8000…` | **yes** ~`0x8fff8000…` |
| aie2_error_backtrack | yes | not in filtered window |
| aie2_set_cmd_timeout | yes; TXN OP `0xffffffff` | yes; TXN OP **`0xd`** |
| vs spent unified-2x4x4 | (both differ: that pin had **no** IO_PAGE_FAULT) | same |

## Notes for residual honesty

- Topology A/B across flexml research PDIs still lands post-attach **`wait_cmd_einval`** with measured `config_cu=1`.
- IO_PAGE_FAULT **address band moves** with PDI topology (`0x2fff…` CVML / `0x4fff…` llm / `0x8fff…` unified-4x4 / **none** on spent unified-2x4x4). That is journal evidence for orphan map class variation, **not** Pass.
- TXN OP ID=`0xd` on this pin (vs `0xffffffff` on Shot L) is a new timeout fingerprint detail only.
- Lab instruction pack remains classic 2x4x4 a16w4; only the CONFIG_CU PDI blob was unified-4x4.
- All three flexml STX research PDIs in the residual ledger are now **Spent**. Further live needs **new** unlock + **named** offline reason (not re-live of spent sha16).

## Reports / pins

- `.agents/reports/live-shot-efd44f35.md`
- `.agents/reports/impl-feedback-live-unified-4x4.md` (this file)
- `RESIDUAL.md` top Unlock + spent ledger
- `AGENTS.md` Latest Turn live closeout

## Bans honored

- No re-live of `0eeeb70c` or `ba0cd738`
- No reopen of `8352882b`
- No invent Pass
- One unlock, one shot only
