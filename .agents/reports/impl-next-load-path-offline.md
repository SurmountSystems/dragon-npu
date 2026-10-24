# Implement report: Phase 1B next load-path offline prep

**Date:** 2026-08-11
**Repo:** `/home/hunter/Projects/ai/dragon-npu`
**Mode:** Offline only. No unlock mint. No live apply. **invent_pass=0**.
**Residual pin:** Phase 2 join owns `RESIDUAL.md` (not edited this slice).

---

## Goal

Different **load path** offline prep after all three flexml STX research PDIs
are Spent on CONFIG_CU attach:

1. Overlay vs lab instruction mismatch honesty (B1)
2. Skip-CONFIG_CU recipe checklist (no live)
3. Research debug-BO offline prep (C6 / existing hooks)
4. Ranked B2 live-experiment menu for Phase 2

---

## What landed

### B1 — residual-ready hypothesis note (docs)

| Location | Content |
|----------|---------|
| `docs/CONSTRUCTION_MAP_APP_LOAD.md` § **Residual-ready note (B1)** | Lab classic pack key vs flexml overlay topology = **hypothesis**, not proven root cause |
| `docs/HITL_C_APP_LOAD_RECIPE.md` Step 2 skip subsection | Same one-line B1 callout + invent_pass=0 |
| `tools/operator_app_load_kit.sh` overview | B1 one-liner + spent sha list |
| `tools/hitl_config_cu_once.sh` header | B1 pointer + spent refuse + expected classes |

**Measured facts (not claims of RC):**

- Stock CONFIG_CU attached flexml STX PDIs then failed post-attach:
  - unified-2x4x4 27200 / sha16 `0eeeb70c84df056e` under `2ae69036` → `wait_cmd_einval` (no IO_PAGE_FAULT in shot window)
  - llm-2x4x4 21664 / sha16 `ba0cd738fd870011` under `8352882b` → `wait_cmd_einval` + IO_PAGE_FAULT ~`0x4fff8000…`
  - unified-4x4 13792 / sha16 `7d172bf223a9b06b` under `efd44f35` → `wait_cmd_einval` + IO_PAGE_FAULT ~`0x8fff8000…`
- Instruction path on those shots stayed lab classic pack key
  `mladfmatmulbias_prmptflat_mladf_2x4x4_flat_a16fw4acc16f_1_1152_256_128`
  (silicon_lab aiebu ELF / 1688 B ctrltext). Live evidence:
  `.agents/reports/live-shot-2ae69036.md`, `live-shot-8352882b.md`,
  `live-shot-efd44f35.md`.

**Hypothesis (Phase 2 may pin to residual):** flexml overlay topology may be
mismatched to the lab classic START_NPU ctrltext / map, so firmware accept-or-run
fails even when CONFIG_CU attach succeeds.

**What B1 is not:** proven root cause; unlock mint; Pass; re-live spent flexml;
public bins product known-good.

### Skip-CONFIG_CU checklist (no live)

Expanded in `docs/CONSTRUCTION_MAP_APP_LOAD.md` and mirrored in recipe + tool
headers. Prefer existing kit / hitl helpers — **no** new agent-only scripts.

| Item | Detail |
|------|--------|
| Offline prep | `bash tools/operator_app_load_kit.sh config-cu-skip` (= `hitl_config_cu_once.sh prep-skip-pdi`) |
| Live later (new unlock only) | `bash tools/operator_app_load_kit.sh config-cu-run-skip` (= `run-stock-skip-pdi`) |
| Env | `unset XDNA2_LAB_PDI_BLOB`; `export XDNA2_LAB_SKIP_CONFIG_CU=1`; `unset XDNA2_ALLOW_EXEC_CMD` |
| Spent refuse | Do not export spent flexml / CVML sha as PDI alone (table in map) |
| Expected classes | `config_cu` absent; `wait_cmd_einval` / `wait_timeout` / APP_LOAD / journal timeout or IO_PAGE_FAULT **possible** |
| Honesty | skip-CU prep ≠ Pass ≠ `ran_on_npu`; invent_pass=0 |

### Debug-BO offline prep (C6)

Existing C6 landing (unchanged product code this slice):

| Item | Location |
|------|----------|
| Pack / validate | `pack_config_hwctx_dbg_buf_aie2` / `validate_config_hwctx_dbg_buf_aie2` (`direct.rs`) |
| Soft | `format_c6_debug_bo_prep_note` / `debug_bo_uapi_prep_aie2` — always invent_pass=0 |
| Env intent only | `XDNA2_LAB_RESEARCH_DEBUG_BO` — **not** live-wired |
| Prior report | `.agents/reports/impl-p2-c6-recipe.md` |
| Docs this slice | Recipe Step 2 “Optional debug BO offline prep”; kit overview; hitl header |

**Offline prep path:** soft classify / unit contracts only. No live
`DRM_AMDXDNA_HWCTX_ASSIGN_DBG_BUF` / `REMOVE_DBG_BUF`.
**debug-BO prep ≠ Pass.**

### Public capture honesty

`testdata/xdna2_cmd/external_known_good/` dual public mlir-aie npu2 AOT remains
**research only** (size class `public-npu2-small`; residual-mismatched to spent
flexml and lab classic 1688). ≠ product known-good ≠ Pass ≠ unlock alone.

---

## B2 menu (Phase 2 live experiments — ranked)

Phase 2 does **not** know Phase A BO-map results. Rank **primary** first.

| Rank | Experiment name | When | Unlock | Notes |
|-----:|-----------------|------|--------|-------|
| **1 (primary)** | **skip-CONFIG_CU isolation** | Always top unless A lands a concrete map fix | **New** host-mint only | Measure lab classic START_NPU **without** flexml/CVML overlay. Env: skip checklist above. Compare `config_cu` absent vs prior attach shots. invent_pass=0. |
| 2 | BO-map / fault-addr fix live | **Only if** Phase A offline finds a map-bound fault + product-legal fix | New unlock + named A reason | Do not invent MMU surgery without journal+map evidence. |
| 3 | Matched-material CONFIG_CU | New capture / matmul-matched PDI+ctrltext under `external_known_good/` + PROVENANCE | New unlock + material FP in reason | Prefer minimal load. Public npu2-small alone is residual-mismatched research. |
| 4 | Debug-BO live wire-up | Only after offline C6 remains green **and** residual needs attach-side visibility | New unlock; may combine with (1) | Today pack is offline-only; live ASSIGN is **not** product-ready from this pin. |
| 5 | **Banned** | — | — | Re-live spent flexml sha alone; reopen CONSUMED unlocks; empty reconfirm mill; Pass-from-attach; Pass-from-capture; invent_pass. |

**Recommended primary live experiment name for Phase 2:**
**`skip-CONFIG_CU isolation`**
(kit: `config-cu-run-skip` / `run-stock-skip-pdi` under a **new** unlock + B1/isolation reason text).

---

## Files touched

| Path | Change |
|------|--------|
| `docs/CONSTRUCTION_MAP_APP_LOAD.md` | B1 note; full skip-CONFIG_CU checklist; spent table; next-questions refresh |
| `docs/HITL_C_APP_LOAD_RECIPE.md` | Step 2: C6 offline prep, skip prep, operator sequence dual-arm, highest-value |
| `tools/operator_app_load_kit.sh` | Overview: Path A-skip, B1, spent list, C6 honesty |
| `tools/hitl_config_cu_once.sh` | Header: skip env, spent refuse, residual classes, B1, C6 |
| `.agents/reports/impl-next-load-path-offline.md` | This report |
| `RESIDUAL.md` | **Not edited** (Phase 2 join) |

---

## Honesty lock

| Claim | Truth |
|-------|--------|
| Product Pass / `ran_on_npu` | **No** |
| invent_pass | **0** |
| skip-CU prep | **≠** Pass |
| debug-BO prep | **≠** Pass |
| B1 overlay vs instr | **Hypothesis only** |
| Public capture bins | **Research only** |
| Live apply / unlock mint this slice | **None** |
| Active unlock | **None** (all listed flexml + prior CONSUMED) |

---

## Verify (offline)

```bash
bash -n tools/operator_app_load_kit.sh
bash -n tools/hitl_config_cu_once.sh
bash tools/operator_app_load_kit.sh overview   # prints Path A-skip + honesty; no live
```

No product Rust edits this slice; no cargo gate required beyond existing C6 green
from prior P2.

---

## Phase 2 handoff (one line)

Pin B1 hypothesis + skip checklist into residual if desired; **do not mint**
unlock from this report alone. Next live, if any: **`skip-CONFIG_CU isolation`**
under a **new** unlock + named offline reason.
