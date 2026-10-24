# S0 — W4 campaign pin + plan mirror

**Date:** 2026-08-11
**Kind:** Offline docs only (no code, no live, no unlock mint).
**Honesty:** invent_pass=0 · Pass=**No** · active unlock=**none** · first
`C_UNLOCK=` = `4008e6fd` **CONSUMED**.

## Done

1. **Plan mirror:** approved session plan body written to workspace
   `.agents/plans/plan-silicon-try-everything.md`
   (session `019f6476-0a84-70a0-a971-be89fa1011fe` plan.md; residual-resolve /
   agent-max W4 plan with assumption catalog A1–A12 and W4a–W4c steps).
2. **RESIDUAL pin:** Latest Turn + plain status updated for W4 campaign start:
   - Campaign W4 agent-max residual resolve **APPROVED**
   - invent_pass=0; Pass=No; no active unlock; first `C_UNLOCK=4008e6fd` CONSUMED
   - Agent offline W4a: host inventory, scorecards, journal tools, syncobj pack
   - Operator one ask: load-accepted capture into
     `testdata/xdna2_cmd/external_known_good/` (public mlir-aie peels research-only)
   - Reports path after wave: `.agents/reports/wave4-*.md`
   - Freeze: no re-live CONSUMED; no empty reconfirm; no invent Pass
3. **AGENTS.md:** Latest Turn prepended (same honesty / unlock / who-acts).

## Unlock state (unchanged)

| Item | Value |
|------|--------|
| Active unlock | **None** |
| First `C_UNLOCK=` | `4008e6fd` **CONSUMED** |
| Prior CONSUMED | `48da6e60`, `1f761bf4`, `efd44f35`, `8352882b`, `2ae69036`, `97f38307`, `11bc9016`, `3a70fabf`, `b17b0119` |
| Pass / ran_on_npu | **No** / false |
| invent_pass | 0 |

## Explicit non-actions

- No unlock mint
- No live apply / no `XDNA2_ALLOW_EXEC_CMD=1`
- No Pass claim / no `ran_on_npu`
- No empty gate reconfirm as progress

## Next (parent / Wave W4a)

Spawn parallel L2 for W4a tracks; write reports only:

- `.agents/reports/wave4-host-inventory.md` (T1 + T8)
- `.agents/reports/wave4-scorecard-tools.md` (T3 + T6)
- `.agents/reports/wave4-syncobj-pack.md` (T5)
- `.agents/reports/wave4-journal-tool.md` (T4)

Then W4b discovery / join pin. No live until new host-minted unlock + named
offline reason (prefer L8 after P-sync offline green, or L1 after load-accepted
material).
