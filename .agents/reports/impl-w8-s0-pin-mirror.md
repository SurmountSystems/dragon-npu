# W8 S0 — plan pin + mirror (Track Material + Track Paddr)

**Date:** 2026-08-11
**Stamp:** docs/residual only (no product code)
**invent_pass=0 · Pass=No · unlock=none · first C_UNLOCK=`4008e6fd` CONSUMED**

## Done (this slice)

1. **Plan mirror:** full session plan copied to
   `.agents/plans/plan-silicon-try-everything.md`
   (source: session `019f6476-0a84-70a0-a971-be89fa1011fe` plan.md —
   *After W7 join / W8 menu*). Workspace mirror header prepended (W8 S0).
2. **RESIDUAL.md:** Latest Turn prepended (W8 S0). Prior W7 join kept as Prior.
3. **AGENTS.md:** Latest Turn prepended (matching short style). Prior W7 join kept.

## Honesty (locked)

| Claim | Value |
|-------|--------|
| invent_pass | **0** |
| Product Pass | **No** |
| Active unlock | **none** |
| First `C_UNLOCK=` | **`4008e6fd` CONSUMED** (do not reopen / do not mint) |
| W7 | Closed: unpatched classic re-land; true post-patch missing; no live |
| Public AOT re-run | **Frozen** (not residual progress) |
| Empty reconfirm | **Frozen** |
| Unlock mint from this pin / plan alone | **Forbidden** |

## Named campaign state

- **W7 closed:** capture-instr re-landed classic 1688 as unpatched oracle
  sha16 `3c5ffca750922d3b`; collect refuse; residual_matched=0; load_accepted=0;
  W7-5 live not justified; no unlock mint.
- **W8 approved:** parallel **Track Material** (M1 true post-patch classic 1688,
  M4 INSTR_BO_LIMIT evidence, A5 DD instruction_cache dump after fit_MKN) +
  **Track Paddr** (O3 live paddr extract from last_apply/journal; M3 pure-direct
  rela with real device paddrs, not synthetic).
- **Agent next:** run Material + Paddr offline (and agent-legal material probes)
  in parallel. Live only under **new** unlock + named reason after residual-
  shaped material or named wait-path delta. No L1/L8 from plan alone.

## Explicit non-claims

- Plan approve ≠ Pass ≠ unlock mint
- Mirror ≠ live execute
- Unpatched classic re-land ≠ true post-patch ≠ C-ready
- IRON PASS ≠ product `decide_ran_on_npu`
- No sticky ALLOW · no invent capture · no empty reconfirm
- No product code this slice · no live · no unlock mint

## Artifacts

| Path | Role |
|------|------|
| `.agents/plans/plan-silicon-try-everything.md` | W8 workspace plan mirror |
| `RESIDUAL.md` (top Latest Turn) | SSoT residual + unlock |
| `AGENTS.md` (top Latest Turn) | Standing pin short form |
| This file | S0 closeout |

## Next (not this slice)

- **Track Material:** M1 / M4 / A5 agent attempts until new sha or proven error
- **Track Paddr:** O3 + M3 pure TDD / extract; unlock + L1b only when justified
- Join after track(s); invent_pass=0; Pass only via live `decide_ran_on_npu`
