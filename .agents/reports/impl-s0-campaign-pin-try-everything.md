# S0 — Campaign pin + plan mirror (try everything)

**Date:** 2026-08-11
**Kind:** Offline docs only (no code, no live, no unlock mint).
**Honesty:** invent_pass=0 · Pass=**No** · no active unlock · first `C_UNLOCK=` = `1f761bf4` **CONSUMED**.

## Done

1. **Plan mirror:** approved session plan body copied to workspace
   `.agents/plans/plan-silicon-try-everything.md`
   (session plan for `019f6476-0a84-70a0-a971-be89fa1011fe`).
2. **RESIDUAL pin:** plain status + ranked next updated for campaign start:
   - Campaign: try-everything Wave **W0 offline fan-out started**
   - invent_pass=0; Pass=No; no active unlock; first `C_UNLOCK=1f761bf4` CONSUMED
   - Freeze list still holds (no re-live spent flexml; no skip-CU reconfirm under CONSUMED)
   - Next: wave reports under `.agents/reports/wave0-*.md`
3. **AGENTS.md:** left unchanged. Parse rule and unlock state already correct
   (`C_UNLOCK=1f761bf4` CONSUMED; no mint this slice).

## Unlock state (unchanged)

| Item | Value |
|------|--------|
| Active unlock | **None** |
| First `C_UNLOCK=` | `1f761bf4` **CONSUMED** |
| Prior CONSUMED | `efd44f35`, `8352882b`, `2ae69036`, `97f38307`, `11bc9016`, `3a70fabf`, `b17b0119` |
| Pass / ran_on_npu | **No** / false |
| invent_pass | 0 |

## Explicit non-actions

- No unlock mint
- No live apply / no `XDNA2_ALLOW_EXEC_CMD=1`
- No Pass claim
- No empty gate reconfirm as progress

## Next (parent / Wave W0)

Spawn parallel L2 explore/impl for W0 tracks; write reports only:

- `.agents/reports/wave0-material-avenues.md`
- `.agents/reports/wave0-material-tools.md`
- `.agents/reports/wave0-wait-einval-uapi.md`
- `.agents/reports/wave0-xrt-dd-load-map.md`
- `.agents/reports/wave0-structural-diff.md`
- `.agents/reports/wave0-prep-inventory.md`

Then residual join pin (O12). No live until new host-minted unlock + named offline reason.
