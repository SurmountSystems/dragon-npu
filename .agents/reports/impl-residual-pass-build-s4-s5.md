# Implement report — residual Pass-unblock Phase 1 (S4–S5)

**Date:** 2026-08-10
**Plan:** `.agents/plans/plan-residual-pass-build.md` (A5 + A4)
**Mode:** offline shell polish only (no cargo, no live, no unlock mint)
**Review-fix:** all open issues in `/tmp/grok-1000/grok-review-residualpass3.md`
addressed (Issue 9 wontfix). Re-review nit: `DUMP_DIR` deferred until after
`--inventory-only` early exit (no empty private dir litter). Re-review R2:
**0 open** (`/tmp/grok-1000/grok-review-residualpass3-general-r2.md`).

## Honesty (unchanged)

| Fact | Value |
|------|--------|
| Product Pass | **No** |
| `ran_on_npu` | **No** (not claimed) |
| invent_pass | **0** |
| Active unlock | **None** |
| Latest CONSUMED | `C_UNLOCK=2ae69036` (flexml unified-2x4x4 stock) |
| Also CONSUMED | `97f38307`, `11bc9016`, `3a70fabf`, `b17b0119` |
| Capture bins | Still 0 under `testdata/xdna2_cmd/external_known_good/` |

## What changed

### S4 — Fingerprint skip re-dump of spent default (plan A5)

File: `tools/pass_unblock_fingerprint_once.sh`

- Prints **spent vs unspent research PDI ledger** footer matching `RESIDUAL.md`
- **Skip re-dump** of spent default tag `flexml_unified-2x4x4` unless
  `--force` / `--force-redump` or `XDNA2_FP_FORCE_REDUMP=1`
- Skip: `out=(not dumped)`, `source_status=ledger_only`; **no** `ANY_DUMP_OK`;
  **no** PREFERRED from skip
- Real dumps: exclusive private `DUMP_DIR` (`mktemp -d`, chmod 700); outs chmod 0600
- **`--inventory-only`:** does **not** create `DUMP_DIR` (lazy create after that exit)
- Verdict: `NAMED_OFFLINE_REASON=1` for preferred unspent **flexml** DIFF or
  `SPENT_DEFAULT_LEDGER_DRIFT=1`; `ELIGIBLE_UNSPENT_DIFF` for any unspent DIFF
  (eligible ≠ mint this turn)
- invent_pass=0 footer always; Pass=No footer tokens
- `usage()` for `--help`

### S5 — Read-only `*mladf*.xclbin` / `*gemm*.xclbin` inventory (plan A4)

- **xclbin-only by design**; `find -maxdepth 12`
- Report: `.agents/reports/pdi-xclbin-inventory.md` (paths/sizes only; empty OK)
- `--inventory-only` path

### Wire + docs

- `tools/pass_unblock_once.sh --agent-offline` documents S4/S5 + `--force` or env
- `AGENTS.md` Latest Turn: S4–S5
- `RESIDUAL.md` elevator + ledger tooling pointer
- Plan: Phase 1 S4–S5 status table

## Verify (this host, post review-fix)

| Command | Exit |
|---------|------|
| `bash -n tools/pass_unblock_fingerprint_once.sh` | 0 |
| `bash -n tools/pass_unblock_once.sh` | 0 |
| `bash tools/pass_unblock_fingerprint_once.sh --inventory-only` | 0 (count=0 honest) |
| `bash tools/pass_unblock_fingerprint_once.sh --offline` | 0 |
| `bash tools/pass_unblock_once.sh --status` | 0 |
| `bash tools/pass_unblock_once.sh --agent-offline` | 0 (`fingerprint_rc=0`) |
| `bash tools/pass_unblock_fingerprint_once.sh --help` | 0 |

Offline smoke highlights:

- `RESULT=SKIP_SPENT_REDUMP` + `out=(not dumped)` for spent default
- `ANY_DUMP_OK=1` only after real dumps (llm/4x4/CVML)
- `PREFERRED_BLOB` under exclusive `DUMP_DIR/.../flexml_llm-2x4x4.bin` mode 600
- `NAMED_OFFLINE_REASON=1` preferred unspent flexml; invent_pass=0 Pass=No
- Inventory count 0 (honest empty); xclbin-only noted

## Files touched

| Path | Role |
|------|------|
| `tools/pass_unblock_fingerprint_once.sh` | S4 + S5 + review-fix |
| `tools/pass_unblock_once.sh` | agent-offline notes + force wording |
| `AGENTS.md` | Latest Turn S4–S5 |
| `RESIDUAL.md` | elevator + ledger tooling pointer |
| `.agents/plans/plan-residual-pass-build.md` | Phase 1 status |
| `.agents/reports/pdi-xclbin-inventory.md` | S5 output (generated) |
| `.agents/reports/pass-unblock-fingerprint-once.md` | fingerprint report (regenerated) |
| `.agents/reports/impl-residual-pass-build-s4-s5.md` | This report |

**Not touched:** product Rust, live runners, unlock mint, S6–S9.

## Next (not this slice)

- Optional S6–S9 (Rust taxonomy, PROVENANCE table soft check, …)
- Live only after **new** host-minted unlock + named offline reason
- Human capture bytes still required for path B bags
- invent_pass=0; Pass still **No**
