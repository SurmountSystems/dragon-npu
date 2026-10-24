# Report: offline residual honesty pin (feedback)

**Date:** 2026-08-11
**Scope:** docs / residual pin only. Offline. No live NPU. No unlock mint.
**invent_pass=0** always.

## What changed

Incorporated operator feedback on dual public mlir-aie npu2 material:

1. **RESEARCH ONLY** labeling hardened for dual capture-dir sets (vector 300 B +
   single_core matmul 660 B). Explicitly **not** residual identity, **not**
   unlock justification alone, **not** Pass / product known-good / C-ready.
2. **Size-class / topology tags** added to residual pin language:
   - `public-npu2-small` — public mlir-aie / IRON npu2 minimal AOT peels
   - `mladf-classic-1688` — lab staged classic mladf ctrltext oracle (1688 B)
   - `flexml-2x4x4-class` — flexml STX research PDI topology (spent + unspent)
3. **One-line residual note:** dual public material present
   (`public-npu2-small`); still residual-mismatched to spent
   `flexml-2x4x4-class` and to lab classic `mladf-classic-1688`.
4. **Route B axes** (size class, opcode/header soft, section map, rela
   structure, multi-axis structural distance in `silicon_contract.rs`) noted as
   **citable in unlock reason text without claiming match**. No new product
   APIs; docs residual only.
5. **Frozen** restated: re-live CONSUMED, empty reconfirm, Pass-from-attach /
   Pass-from-capture bins, agent live without unlock, MMU without new journal.

## Files touched

| Path | Change |
|------|--------|
| `/home/hunter/Projects/ai/dragon-npu/RESIDUAL.md` | Plain status, three needs, material rule, size-class tag table, one-line residual, ranked next, honesty closeout |
| `/home/hunter/Projects/ai/dragon-npu/AGENTS.md` | Prepended Latest Turn (feedback); prior Route A+B turn marked superseded for RESEARCH ONLY + tags |
| `/home/hunter/Projects/ai/dragon-npu/docs/CAPTURE_MATERIAL_MLIR_AIE.md` | Header RESEARCH ONLY; landed dual material blurb; plain-English table; agent rules; residual pin step |
| `/home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/external_known_good/PROVENANCE.md` | Size-vs-oracle table uses residual pin tags + one-line residual note |
| `/home/hunter/Projects/ai/dragon-npu/.agents/reports/impl-feedback-offline-residual-pin.md` | This report |

## Honesty tokens

| Token | Value |
|-------|--------|
| invent_pass | **0** |
| product Pass | **No** |
| ran_on_npu | **No** / not claimed |
| C_UNLOCK first line | **`C_UNLOCK=2ae69036` CONSUMED** (unchanged; not reopened; not re-minted) |
| Dual public material | **RESEARCH ONLY** · `public-npu2-small` |
| Residual match to flexml / lab 1688 | **No** (mismatch expected) |
| Unlock from this pin | **None** — dual bins ≠ unlock alone |
| Live apply this slice | **None** |

## What next

Live unlock remains **separate parent work**. From residual ranking:

1. Optional offline classify on dual research material (Route B axes; invent_pass=0).
2. Prefer minimal load; unspent flexml llm-2x4x4 / unified-4x4 if live is ever justified.
3. Next live only under a **new** host-minted unlock + **named** reason (dual
   public research bins alone are **not** that reason).
4. Keep frozen: empty reconfirm, reopen CONSUMED `2ae69036`, Pass from attach or
   bins, agent live without unlock, MMU without new journal.

**Agent-legal only until then:** classify present bytes; unlock justification
text only; no unlock mint; no live apply.
