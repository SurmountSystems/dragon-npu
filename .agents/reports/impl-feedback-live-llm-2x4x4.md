# Impl feedback: live flexml llm-2x4x4 CONFIG_CU (`8352882b`)

**Date:** 2026-08-11
**Scope:** one stock live CONFIG_CU only · invent_pass=0 · Unlock ≠ Pass

---

## Outcome (one line)

Unlock **`8352882b` CONSUMED**. Stock live with flexml **llm-2x4x4** PDI (21664 / `ba0cd738fd870011`) measured **`config_cu=1`**, then **`wait_cmd_einval`**. **Pass=No.** `ran_on_npu=false`.

---

## Steps executed

1. **Dump PDI offline** from
   `/home/hunter/Projects/ai/ryzen-ai-install/venv/lib/python3.12/site-packages/flexml/flexml_extras/data/ryzen-ai/stx/llm-2x4x4.xclbin`
   → `/tmp/dragon_npu_pdi_flexml_llm-2x4x4.bin` (already present; refuse-clobber; size/sha verified).
2. **Fingerprint verify:** size **21664**, sha16 **`ba0cd738fd870011`**. Refused spent `0eeeb70c84df056e`.
3. **Mint unlock:** `openssl rand -hex 4` → **`8352882b`**. First `C_UNLOCK=` in `RESIDUAL.md` set active with named reason; AGENTS Latest Turn prepended.
4. **One stock live:**
   `export XDNA2_LAB_PDI_BLOB=/tmp/dragon_npu_pdi_flexml_llm-2x4x4.bin`
   `export XDNA2_LAB_CU_FUNC=0`
   `export XDNA2_HITL_OUTER_TIMEOUT_S=120`
   `bash tools/operator_app_load_kit.sh config-cu-run`
   No sticky ALLOW. No sudo (stock `exec_cmd_allow=missing`). Single shot only.
5. **Post-live:** last_apply + kit post-evidence / p5 interpret ran inside run-stock. C4_SKIP / P6_NOT_PASS.
6. **CONSUME** first `C_UNLOCK=8352882b` with outcome summary. invent_pass=0.
7. **Reports:** this file + `.agents/reports/live-shot-8352882b.md`.

---

## Measured results

| Field | Value |
|-------|--------|
| Unlock | `8352882b` → **CONSUMED** |
| last_apply | `/tmp/dragon_npu_last_apply_config_cu_2328515.json` |
| status | `wait_cmd_einval` |
| config_cu | `1` |
| pdi_bytes | `21664` |
| sha16 | `ba0cd738fd870011` |
| real_ioctl | `true` |
| ran_on_npu | `false` |
| hitl_c_rc | `4` |
| total_s | ≈ 61.57 |
| aiebu_elf / rela | true / 12 |
| Pass | **No** |
| invent_pass | **0** |

### Journal (distinct from spent unified)

- **IO_PAGE_FAULT** cluster ~`0x4fff8000…` (yes on this shot; **no** on `2ae69036` unified)
- `aie2_error_backtrack` (row/col events)
- `aie2_set_cmd_timeout` Context 15; DPU/TXN PC `0xffffffff`

### Residual class

Post-attach **`wait_cmd_einval`** + cmd timeout/backtrack + **IO_PAGE_FAULT** at high orphan-ish `0x4fff8000…` vs lab BO map (DevHeap `0x4000000`, instr `0x4028000`, host UVAs). Not pure APP_LOAD. Attach ≠ Pass.

---

## What this is not

- Not product Pass
- Not `ran_on_npu`
- Not re-live of spent unified `0eeeb70c`
- Not second unlock for unified-4x4 (parent next task)
- Not residual identity with public npu2 AOT research material

---

## Ledger after shot

| tag | pdi_bytes | sha16 | Live |
|-----|-----------|-------|------|
| flexml unified-2x4x4 | 27200 | `0eeeb70c84df056e` | Spent `2ae69036` |
| flexml llm-2x4x4 | 21664 | `ba0cd738fd870011` | **Spent `8352882b`** |
| flexml unified-4x4 | 13792 | `7d172bf223a9b06b` | Not spent (needs **new** unlock) |

---

## Next (agent-legal)

1. Optional offline map audit of `0x4fff8000…` faults vs BO map (no live).
2. Next live only under **new** host-minted unlock + named reason (e.g. unspent unified-4x4).
3. **Frozen:** reopen `8352882b` / re-live spent flexml / invent Pass / sticky ALLOW.

**Unlock active:** none.
**Pass:** No.
**Reports:** `.agents/reports/live-shot-8352882b.md`, this file, `RESIDUAL.md` top.
