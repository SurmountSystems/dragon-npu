# Implement summary — pass-unblock once (Layer A entrypoint)

**Date:** 2026-08-10  
**Plan:** pass-unblock operator once (autonomy + one entrypoint)  
**Repo:** `/home/hunter/Projects/ai/dragon-npu`

## Honesty (locked)

| Fact | Value |
|------|--------|
| Pass / ran_on_npu | **No** |
| invent_pass | **0** |
| Active unlock | **None** (`2ae69036` CONSUMED flexml not Pass; also `97f38307`, `11bc9016`, `3a70fabf`, `b17b0119` CONSUMED) |
| Capture bag | empty (PROVENANCE only) |
| Privilege script | **Skipped** — stock host `exec_cmd_allow` sysfs **missing**; prior stock lives needed no sudo L3 |
| Flexml re-live | not run |

## Files changed

| Path | Change |
|------|--------|
| `tools/pass_unblock_once.sh` | **NEW** primary Layer A entrypoint |
| `tools/operator_app_load_kit.sh` | alias `pass-unblock` / `pass_unblock` → entrypoint; overview help |
| `testdata/xdna2_cmd/external_known_good/PROVENANCE.md` | one primary command first |
| `RESIDUAL.md` | top plain status: entrypoint, Pass=No, unlock none, privilege skipped |
| `AGENTS.md` | Latest Turn pin for this slice |
| `.agents/plans/plan-pass-unblock-operator-once.md` | mirror of approved session plan |
| `.agents/reports/impl-pass-unblock-once.md` | project report (this content) |
| `.agents/reports/pass-unblock-once.md` | agent-offline runtime report |

## Design decisions

1. **One script for full Layer A**, not fingerprint-only: `--status` (default), `--agent-offline` (wraps `pass_unblock_fingerprint_once.sh --offline` + bag status + report), `--ingest` (reuse `operator_external_capture.sh --install` + same oracle refuse), `--ingest-pdi` (copy/fingerprint to `/tmp` + report; not known-good), `--live` (refuse exit 3 without active non-CONSUMED `C_UNLOCK`).
2. **Footer always:** `invent_pass=0 product_pass=0 ran_on_npu=0 Pass=No HUMAN_NEEDED PRIVILEGE_NEEDED`.
3. **HUMAN_NEEDED=1** when capture bins=0 (human holds working-stack bytes); agent offline path still available.
4. **PRIVILEGE_NEEDED=0** on this host: probe finds no `exec_cmd_allow` sysfs node.
5. **No** `pass_unblock_privileged_once.sh` invented (S3 skip).
6. Patterns: `set -euo pipefail`, PATH, `unset XDNA2_ALLOW_EXEC_CMD`, never L3/ALLOW.

## TDD

Shell/docs only. RED/GREEN Rust N/A (exception noted). Verify = bash -n + mode smokes.

## Verification (commands + exit codes)

| Command | Exit |
|---------|------|
| `bash -n tools/pass_unblock_once.sh` | **0** |
| `bash -n tools/operator_app_load_kit.sh` | **0** |
| `bash tools/pass_unblock_once.sh --status` | **0** |
| `bash tools/pass_unblock_once.sh` (default) | **0** |
| `bash tools/pass_unblock_once.sh --agent-offline` | **0** |
| `bash tools/pass_unblock_once.sh --live` (no unlock) | **3** |
| `bash tools/pass_unblock_once.sh --ingest /no/such/file.bin` | **2** |
| `bash tools/pass_unblock_once.sh --ingest /tmp/instr_post_patch.bin` (oracle) | **2** |
| `bash tools/operator_app_load_kit.sh pass-unblock --status` | **0** |

## Privilege script

**Skipped.** Proof: `cat /sys/module/amdxdna/parameters/exec_cmd_allow` → No such file; stock lives under `11bc9016` / `97f38307` / `2ae69036` ran without sudo L3. Residual + status document this.

## What still unblocks Pass (unchanged product goal)

1. Working-stack ctrltext under `testdata/xdna2_cmd/external_known_good/` (use `--ingest` when path known).
2. True matmul-matched PDI/ctrltext beyond spent flexml (+ new unlock only with new FP).
3. Driver/MMU parked until new evidence.

## Do not

Re-live flexml; mint unlock; invent capture; claim Pass; multi-tool human scavenger hunt (entrypoint is the fix).

---

## Review-fix (2026-08-10)

Addressed all open review issues in `grok-review-pass-unblock-once.md`.

### Security
1. **Path traversal on install NAME** — `safe_install_name` + bag-root confinement in shared `install_one` (`operator_external_capture.sh`); entrypoint thin refuse. Smoke: `../x.bin` → exit 2, no write outside bag.
2. **Predictable /tmp ingest-pdi** — `mktemp dragon_npu_pdi_ingest.XXXXXX` exclusive path; no basename-derived fixed names.
3. **validate.xclbin / ve2 / NOT_AIE2** — expanded `refuse_basename` on source + NAME (capture + entrypoint).

### Honesty / UX
- Help HUMAN_NEEDED aligned (bins=0; agent offline still runs)
- `stock_no_sudo_for_config_cu_run` derived from PRIVILEGE probe
- Footer on ingest refuse exits (`exit_refuse`)
- `fingerprint_rc=` in footer (soft agent-offline exit 0)
- PRIVILEGE_NEEDED=1 when exec_cmd_allow present and 0/N (still no privilege script)
- Exit map 0/2/3 only
- RESIDUAL mid-file unlock cell; kit PRIMARY modes; capture checklist residual pointer
- sha16 loud; full sha256 gated by `XDNA2_LAB_PRINT_FULL_SHA=1`

### Still true
Pass=No invent_pass=0; unlock none; privilege script skipped; no git mutate.

---

## Review-fix Issue 15 (2026-08-10)

**Bug:** Fixed predictable `/tmp` report paths followed symlinks (`cp -f` / redirect).

**Fix:**
- Primary reports only under `.agents/reports/`
- Optional `/tmp` via `mktemp` exclusive (`dragon_npu_pass_unblock_once.XXXXXX`, `dragon_npu_pass_unblock_fingerprint.XXXXXX`)
- Removed fixed names `…_once_report.txt` / `…_fingerprint_report.txt`

**Smoke:** plant symlink at old fixed paths → `--agent-offline` → victim content unchanged; bag clean.

**bash -n:** both scripts 0. Pass=No invent_pass=0.
