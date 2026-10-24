# Implement report — residual Pass-unblock Phase 0 (S0–S3)

**Date:** 2026-08-10
**Plan:** `.agents/plans/plan-residual-pass-build.md`
**Mode:** docs-first only (no cargo, no live, no unlock mint)

## Honesty (unchanged)

| Fact | Value |
|------|--------|
| Product Pass | **No** |
| `ran_on_npu` | **No** (not claimed) |
| invent_pass | **0** |
| Active unlock | **None** |
| Latest CONSUMED | `C_UNLOCK=2ae69036` (flexml unified-2x4x4 stock) |
| Also CONSUMED | `97f38307`, `11bc9016`, `3a70fabf`, `b17b0119` |
| Capture bins | Still 0 under `testdata/xdna2_cmd/external_known_good/` (PROVENANCE only) |

## What changed

### S0 — Unlock SSoT scrub

- `docs/HITL_C_APP_LOAD_RECIPE.md`: header Status + claim table + unlock policy no longer say Active `11bc9016`. Active = **None**; flexml `2ae69036` and all priors **CONSUMED**. Exact-apply section labeled **historical** (shape for future unlock only). Highest-value next: offline / capture / prep, not live under consumed stamps.
- `docs/HITL_BC.md`: intro, Milestone C, HV, honesty rows match RESIDUAL (no active unlock).
- `docs/CONSTRUCTION_MAP_APP_LOAD.md`: live stamp and “highest-value” bullets no longer claim active `11bc9016`.

### S1 — Spent vs unspent PDI ledger

- `RESIDUAL.md` top: new **Spent vs unspent research PDI ledger** table + three-need pointer.
- CONSTRUCTION_MAP: same ledger next to operator tools.
- Rows: flexml unified-2x4x4 27200 / `0eeeb70c…` **spent**; llm-2x4x4 21664 / `ba0cd7…` **not spent**; unified-4x4 13792 / `7d172b…` **not spent**; CVML 283840 / `a1e65d…` **spent**.

### S2 — Public-stack ctrltext extract playbook

- CONSTRUCTION_MAP: full **Public-stack ctrltext extract playbook** (run stack → capture → `--ingest` → PROVENANCE → soft compare).
- `testdata/xdna2_cmd/external_known_good/PROVENANCE.md`: short exact steps + refuse list + entrypoint with preferred basename.
- invent_pass=0; match ≠ Pass; local oracles refused.

### S3 — Skip-PDI vs CONFIG_CU checklist

- CONSTRUCTION_MAP: **Skip-PDI vs CONFIG_CU experiment checklist** pointing at `config-cu-skip` / `prep-skip-pdi` / `run-stock-skip-pdi` / `XDNA2_LAB_SKIP_CONFIG_CU=1`.
- Requires **new** unlock each live; never auto-mint; invent_pass=0.

### Plan mirror

- `.agents/plans/plan-residual-pass-build.md`: Phase 0 S0–S3 status table + report path.

## Files touched

| Path | Role |
|------|------|
| `docs/HITL_C_APP_LOAD_RECIPE.md` | S0 unlock scrub |
| `docs/HITL_BC.md` | S0 unlock scrub |
| `docs/CONSTRUCTION_MAP_APP_LOAD.md` | S0 + S1 ledger + S2 playbook + S3 checklist |
| `RESIDUAL.md` | S1 ledger + three-need pointer |
| `testdata/xdna2_cmd/external_known_good/PROVENANCE.md` | S2 short playbook |
| `.agents/plans/plan-residual-pass-build.md` | Plan status |
| `.agents/reports/impl-residual-pass-build-s0-s3.md` | This report |

**Not touched:** product Rust, live runners under CONSUMED unlocks, S4–S9 shell polish, AGENTS Latest Turn (already unlock-none / flexml spent).

## Verify

- Grep HITL_C recipe: no “Active” unlock for `11bc9016` in header/status table after scrub.
- No shell edits → no `bash -n`.
- No cargo.

## Next (not this slice)

- Optional S4–S9 soft tooling if operator wants inventory/taxonomy polish.
- Live only after **new** host-minted unlock + named offline reason (unspent PDI, skip-PDI, or capture-driven instr).
- Human capture bytes still required for path B bags.
