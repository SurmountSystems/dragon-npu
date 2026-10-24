# S0 — W5 plan pin + plan mirror

**Date:** 2026-08-11
**Kind:** Offline docs only (no code, no live, no unlock mint).
**Honesty:** invent_pass=0 · Pass=**No** · not_ran_on_npu=1 · not_c_ready=1 ·
active unlock=**none** · first `C_UNLOCK=` = `4008e6fd` **CONSUMED**.

## Done

1. **Plan mirror:** approved session plan body written to workspace
   `.agents/plans/plan-silicon-try-everything.md`
   (session `019f6476-0a84-70a0-a971-be89fa1011fe` plan.md; load-accepted
   capture wall post W4; W5 agent-max scan/extract/PROVENANCE/Path A/B/C).
2. **RESIDUAL pin:** Latest Turn + plain status updated for W5 campaign start:
   - W4 offline complete; W5 capture automation next
   - invent_pass=0; Pass=No; no active unlock; first `C_UNLOCK=4008e6fd` CONSUMED
   - **No unlock mint** from this pin alone
   - Primary wall: operator load-accepted material
   - Agent: host scan / extract / PROVENANCE / path checklists offline
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
- No live apply / no `XDNA2_ALLOW_EXEC_CMD=1` / no L3 open
- No Pass claim / no `ran_on_npu`
- No empty gate reconfirm as progress
- No re-tag of public AOT as load-accepted

## Next (parent / Wave W5)

Spawn offline L2 for W5-2…W5-9 (parallel OK on disjoint scopes):

- W5-2 host capture scan tool
- W5-3 load-accepted PROVENANCE template
- W5-4 extract/collect + fail-closed ingest glue
- W5-5…W5-7 Path B/C/A checklists + host status
- W5-8 scorecard honesty if needed
- W5-9 join → `.agents/reports/wave5-join.md` (default: still no unlock)

No live until new host-minted unlock + named offline reason (prefer L1 after
load-accepted material; L8 only with reason beyond “wire landed”).
