# Pass-unblock fingerprint once (offline)

**Date:** 2026-08-10T14:44:57-06:00

## Honesty

- invent_pass=0
- not product Pass; not_ran_on_npu=1
- do not reopen CONSUMED unlocks: `2ae69036`, `97f38307`, `11bc9016`, `3a70fabf`, `b17b0119`
- dump ≠ Pass ≠ C unlock ≠ matmul known-good PDI
- CVML baseline compare: bytes=`283840` sha16=`a1e65d04add3422c`
- spent default: `flexml_unified-2x4x4` `27200` / `0eeeb70c84df056e` under `2ae69036`
- force_redump=0; SKIPPED_SPENT_REDUMP=1; SPENT_DEFAULT_LEDGER_DRIFT=0
- ANY_DUMP_OK=1 (real dump only; pure skip does not set)
- DUMP_DIR=`/tmp/dragon_npu_pdi.Kw18Lw` (exclusive private; dumps chmod 0600)
- eligible unspent material ≠ unlock mint this turn

## Spent vs unspent ledger

| tag | pdi_bytes | sha16 | live |
|-----|-----------|-------|------|
| flexml STX `unified-2x4x4` | 27200 | `0eeeb70c84df056e` | **Spent** under `2ae69036` |
| flexml STX `llm-2x4x4` | 21664 | `ba0cd738fd870011` | **Not spent** |
| flexml STX `unified-4x4` | 13792 | `7d172bf223a9b06b` | **Not spent** |
| CVML AIE2P 4x4 Overlay | 283840 | `a1e65d04add3422c` | **Spent** prior |

## Capture dir

- path: `testdata/xdna2_cmd/external_known_good/`
- bins (non-oracle): **0**
- blocked_on_external_bytes=1 (expected; not invented)

## Fingerprint table

| tag | pdi_bytes | sha16 | vs CVML baseline | spent | out | note |
|-----|-----------|-------|------------------|-------|-----|------|
| `flexml_unified-2x4x4` | 27200 | `0eeeb70c84df056e` | **DIFF** | 1 | `(not dumped)` | already_measured_spent under 2ae69036; skip re-dump (not a new unlock reason) |
| `flexml_llm-2x4x4` | 21664 | `ba0cd738fd870011` | **DIFF** | 0 | `/tmp/dragon_npu_pdi.Kw18Lw/flexml_llm-2x4x4.bin` | differs_from_cvml_baseline; unspent_candidate |
| `flexml_unified-4x4` | 13792 | `7d172bf223a9b06b` | **DIFF** | 0 | `/tmp/dragon_npu_pdi.Kw18Lw/flexml_unified-4x4.bin` | differs_from_cvml_baseline; unspent_candidate |
| `cvml_AMD_AIE2P_4x4_Overlay` | 283840 | `a1e65d04add3422c` | **SAME** | 1 | `/tmp/dragon_npu_pdi.Kw18Lw/cvml_AMD_AIE2P_4x4_Overlay.bin` | matches_cvml_baseline; SPENT prior unlocks (not a new unlock reason) |
| `cvml_AMD_AIE2P_4x4_Overlay_CFG0` | 283840 | `a1e65d04add3422c` | **SAME** | 1 | `/tmp/dragon_npu_pdi.Kw18Lw/cvml_AMD_AIE2P_4x4_Overlay_CFG0.bin` | matches_cvml_baseline; SPENT prior unlocks (not a new unlock reason) |
| `cvml_AMD_AIE2P_8x4x1_Overlay` | 568224 | `19c430f04909c280` | **DIFF** | 0 | `/tmp/dragon_npu_pdi.Kw18Lw/cvml_AMD_AIE2P_8x4x1_Overlay.bin` | differs_from_cvml_baseline; unspent_candidate |

### Sources

- **flexml_unified-2x4x4**: `/home/hunter/Projects/ai/ryzen-ai-install/venv/lib/python3.12/site-packages/flexml/flexml_extras/data/ryzen-ai/stx/unified-2x4x4.xclbin`
  - sha256=`(ledger; re-dump skipped — use --force to measure)`
- **flexml_llm-2x4x4**: `/home/hunter/Projects/ai/ryzen-ai-install/venv/lib/python3.12/site-packages/flexml/flexml_extras/data/ryzen-ai/stx/llm-2x4x4.xclbin`
  - sha256=`ba0cd738fd870011815aa4e885429d2f32a908b63d8f10084799ff18f19c2478`
- **flexml_unified-4x4**: `/home/hunter/Projects/ai/ryzen-ai-install/venv/lib/python3.12/site-packages/flexml/flexml_extras/data/ryzen-ai/stx/unified-4x4.xclbin`
  - sha256=`7d172bf223a9b06b0f472fd47d9eae7547f4a440747c49fbf021b05f72978e70`
- **cvml_AMD_AIE2P_4x4_Overlay**: `/home/hunter/Projects/ai/RyzenAI-SW/Ryzen-AI-CVML-Library/linux/onnx/ryzen14/AMD_AIE2P_4x4_Overlay.xclbin`
  - sha256=`a1e65d04add3422c4701621d788414244a22948e03426f380f9287193c3b7ec6`
- **cvml_AMD_AIE2P_4x4_Overlay_CFG0**: `/home/hunter/Projects/ai/RyzenAI-SW/Ryzen-AI-CVML-Library/linux/onnx/ryzen14/AMD_AIE2P_4x4_Overlay_CFG0.xclbin`
  - sha256=`a1e65d04add3422c4701621d788414244a22948e03426f380f9287193c3b7ec6`
- **cvml_AMD_AIE2P_8x4x1_Overlay**: `/home/hunter/Projects/ai/RyzenAI-SW/Ryzen-AI-CVML-Library/linux/onnx/ryzen14/AMD_AIE2P_8x4x1_Overlay.xclbin`
  - sha256=`19c430f04909c2801f7b6877d3e0f8ab499bda7fb2d115b359df7166083facde`

## Inventory (S5)

See `.agents/reports/pdi-xclbin-inventory.md` (paths/sizes only; empty OK; **xclbin-only by design**).

## Verdict

| field | value |
|-------|-------|
| NAMED_OFFLINE_REASON | **1** |
| reason | preferred unspent flexml DIFF tag=flexml_llm-2x4x4 vs CVML baseline 283840/a1e65d04add3422c (eligible material; unlock NOT minted here) |
| ELIGIBLE_UNSPENT_DIFF | 1 |
| ANY_UNSPENT_DIFF | 1 |
| ANY_UNSPENT_FLEXML_DIFF | 1 |
| SKIPPED_SPENT_REDUMP | 1 |
| SPENT_DEFAULT_LEDGER_DRIFT | 0 |
| ANY_DUMP_OK | 1 |
| PREFERRED_BLOB | `/tmp/dragon_npu_pdi.Kw18Lw/flexml_llm-2x4x4.bin` |
| PREFERRED_TAG | `flexml_llm-2x4x4` |
| PREFERRED_REASON | first successful dump (DIFF) unspent candidate |
| PREFERRED_SPENT | 0 |
| HUMAN_NEEDED | **0** |
| HUMAN_CMD | none — agent finished offline |
| DUMP_DIR | `/tmp/dragon_npu_pdi.Kw18Lw` |
| invent_pass | 0 |
| not_ran_on_npu | 1 |

**Named offline reason (for parent unlock decision, not minted here):** preferred unspent flexml DIFF tag=flexml_llm-2x4x4 vs CVML baseline 283840/a1e65d04add3422c (eligible material; unlock NOT minted here)

## Live next (not run)

`--live` is refused unless RESIDUAL has an active non-CONSUMED `C_UNLOCK=`. This slice does not open L3 or run EXEC. After parent mints unlock + operator sandwich: set `XDNA2_LAB_PDI_BLOB` to an **unspent** preferred blob under DUMP_DIR and run stock CONFIG_CU once.

Plain-text dump (exclusive mktemp, not a fixed /tmp name): `/tmp/dragon_npu_pass_unblock_fingerprint.Nl5moM`
