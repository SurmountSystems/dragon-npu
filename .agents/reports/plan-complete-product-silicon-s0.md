# S0 report — plan complete product silicon baseline

**Date:** 2026-08-10
**Scope:** S0 docs baseline only (unlock mint lands in S1 of same implement wave).

## Done

1. **RESIDUAL.md top plain status** rewritten for plan active:
   - Who acts next: agent offline prep → **human** S3 sandwich and/or S3b capture
   - Pass? No. invent_pass=0
   - Capture still empty under `testdata/xdna2_cmd/external_known_good/`
   - Named offline reason for unlock: progressive last_apply + timeout honesty + ≥120s recipe

2. **Unlock (S1 same wave):** first `C_UNLOCK=` in RESIDUAL is **`C_UNLOCK=11bc9016`** active (no CONSUMED on that line). Prior `3a70fabf` / `b17b0119` on **Prior unlocks** row as CONSUMED.

3. **Kit overview** (`tools/operator_app_load_kit.sh`): no longer claims `3a70fabf` active; shows `11bc9016` active + historical CONSUMED.

4. **Workspace plan mirror:** `.agents/plans/plan-complete-product-silicon.md`

## Verify

```text
grep -m1 C_UNLOCK RESIDUAL.md
→ C_UNLOCK=11bc9016 … active … (no CONSUMED on that line)
```

## Not done in S0 alone

Human live sandwich / capture install (S3 / S3b). See final impl report for S1–S2b.
