# Residual Pass-unblock build — closeout

**Date:** 2026-08-10
**Plan:** `.agents/plans/plan-residual-pass-build.md`
**Loop:** implement skill effort 3 (implement → review → fix → re-review → 0 open)

## Honesty (hard)

| Fact | Value |
|------|--------|
| Product Pass | **No** |
| `ran_on_npu` | **No** |
| invent_pass | **0** |
| Active `C_UNLOCK` | **None** (all one-shots CONSUMED) |
| Latest CONSUMED | `2ae69036` (flexml unified-2x4x4; 27200 / `0eeeb70c…`) |
| Also CONSUMED | `97f38307`, `11bc9016`, `3a70fabf`, `b17b0119` |
| Capture dir bins | **0** under `testdata/xdna2_cmd/external_known_good/` |

Offline tooling and docs **do not** claim Pass. Spent re-dump alone is not a new unlock reason.

## What was built (agent residual)

### Phase 0 — Docs S0–S3

| Slice | Deliverable |
|-------|-------------|
| S0 | HITL recipe + active-unlock scrub; first `C_UNLOCK=` parse SSoT |
| S1 | Spent vs unspent research PDI ledger in `RESIDUAL.md` + fingerprint footer |
| S2 | Public-stack ctrltext extract playbook (`docs/CONSTRUCTION_MAP_APP_LOAD.md`) |
| S3 | Skip-PDI vs CONFIG_CU experiment checklist |

Reports: `.agents/reports/impl-residual-pass-build-s0-s3.md`

### Phase 1 — Tooling S4–S5 + Layer A

| Slice | Deliverable |
|-------|-------------|
| Layer A | `tools/pass_unblock_once.sh` — status / agent-offline / ingest / ingest-pdi / live-refuse |
| S4 | `pass_unblock_fingerprint_once.sh` — skip spent default re-dump unless `--force` / `XDNA2_FP_FORCE_REDUMP=1`; exclusive private dump dir; `ANY_DUMP_OK` only on real dump |
| S5 | Read-only `*mladf*.xclbin` / `*gemm*.xclbin` inventory → `.agents/reports/pdi-xclbin-inventory.md` |

Reports: `.agents/reports/impl-residual-pass-build-s4-s5.md`

### Review / fix (effort 3)

- Fixed: `ANY_DUMP_OK` not set on pure skip; fixed `/tmp` dump basenames; dump modes 700/0600; preferred never from skip; force/env wording; inventory maxdepth + xclbin-only notes.
- Fixed (re-review nit): **`DUMP_DIR` created only when dumping PDI**, not on `--inventory-only`.
- Re-review: **0 open** (`/tmp/grok-1000/grok-review-residualpass3-general-r2.md`).

## What still unblocks Pass (unchanged)

| Need | How |
|------|-----|
| Working-stack ctrltext in the capture dir | When you have a real file: `bash tools/pass_unblock_once.sh --ingest /path/to/ctrltext.bin` |
| True matmul-matched PDI beyond spent flexml | Offline discovery + **new** unlock only with a **new** fingerprint (not re-live of `0eeeb70c…` alone) |
| Driver/MMU orphan class | Parked until new evidence |

Unspent research material still present offline (e.g. flexml `llm-2x4x4` / sha16 `ba0cd738…`) is **eligible for a future unlock mint**, not Pass, and not auto-minted this slice.

## Operator next (plain English)

1. **If you have working-stack ctrltext:** ingest into the capture directory with `pass_unblock_once.sh --ingest`.
2. **If you want another research PDI live shot:** only after a **new host-minted unlock** with a named offline reason (new fingerprint ≠ spent ledger).
3. **Agent:** idle on empty reconfirm; no second unlock without new material; no invent Pass.

## Commands (offline, any time)

```bash
bash tools/pass_unblock_once.sh --status
bash tools/pass_unblock_once.sh --agent-offline
bash tools/pass_unblock_fingerprint_once.sh --offline
bash tools/pass_unblock_fingerprint_once.sh --inventory-only
```

## Deferred (not in this closeout)

S6–S9 optional polish; live gated; driver/MMU research; privilege one-shot script (not invent until Layer A wall).

**Pass=No. invent_pass=0. not_ran_on_npu=1.**
