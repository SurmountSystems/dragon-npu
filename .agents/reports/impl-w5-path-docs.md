# Report: W5-5 / W5-6 / W5-7 load-accepted path docs

**Date:** 2026-08-11
**Scope:** Documentation + read-only status helper only.
**Honesty:** invent_pass=0 · Pass=**No** · not_ran_on_npu=1 · not_c_ready=1
· AOT ≠ load-accepted · validate ≠ Pass · FLM absent · no network install ·
no live apply · no invent bins · no unlock mint.

---

## Delivered

### W5-5 Path B (IRON / mlir-aie)

| Item | Location |
|------|----------|
| One operator page (preferred) | `docs/LOAD_ACCEPTED_CAPTURE.md` Path B |
| Dry-run checklist using existing mlir-aie + mlir-aie-capture trees | same doc |
| Exact commands: smallest npu2 example **to completion** on NPU | `vector_scalar_add.py --dev npu2` (+ `--from-xclbin` / `--from-insts` option) |
| After success: where to look + ingest + PROVENANCE `completed_on_this_fw=yes` | same doc |
| Refuse re-tag public AOT 300/660/2592/4512 as load-accepted | same doc + playbook rule 7 |
| Playbook extension | `docs/CAPTURE_MATERIAL_MLIR_AIE.md` §3b + checklist + Related links |

### W5-6 Path C (XRT)

| Item | Location |
|------|----------|
| `xrt-smi` present; health ≠ Pass | `docs/LOAD_ACCEPTED_CAPTURE.md` Path C |
| Artifact land notes + journal scrape | `bash tools/journal_npu_scrape.sh` after operator run |
| Peel only if job truly completes | explicit refuse language in Path C |

### W5-7 Path A (FastFlowLM / Lemonade)

| Item | Location |
|------|----------|
| Honest host status **absent** | Path A section |
| Operator install recipe only (links; agent does not download) | FastFlowLM + Lemonade GitHub URLs |
| Once present: scan `/tmp`, `~/.cache`, flm dirs → same `--ingest` helpers | Path A + shared after-success |

### Optional helper

| Item | Location |
|------|----------|
| Read-only Path A/B/C availability | `tools/load_accepted_path_status.sh` |
| Always exit 0; invent_pass=0 footer | yes |

### Pins / links

| File | Change |
|------|--------|
| `testdata/xdna2_cmd/external_known_good/PROVENANCE.md` | Link load-accepted page; refuse re-tag note |
| `RESIDUAL.md` | Latest Turn W5-5/6/7 + plain status tools/who-acts |
| `AGENTS.md` | Latest Turn mirror |

---

## Verify

```text
chmod +x tools/load_accepted_path_status.sh   # if needed
bash -n tools/load_accepted_path_status.sh
bash tools/load_accepted_path_status.sh
# Expect: path tokens + invent_pass=0; no Pass claim
```

Docs-only + bash helper; no cargo product change. No live NPU. No download.
Operator (or process mop) can run the two bash lines above for host-local
confirmation.

---

## Honesty (locked)

| Claim | Value |
|-------|-------|
| invent_pass | 0 |
| Pass | No |
| ran_on_npu | false / not claimed |
| Public AOT load-accepted | **No** (research only) |
| xrt-smi validate = Pass | **No** |
| FastFlowLM on host | **Absent** (until operator installs) |
| Unlock | none active (`4008e6fd` CONSUMED) |

---

## Who acts next

1. **Operator:** Path B completed NPU run (preferred when trees present), or Path C completing job, or install Path A; then ingest + honest PROVENANCE.
2. **Agent:** remaining W5 offline slices if named; **idle** on empty reconfirm; **no** re-tag research peels.

---

## Files changed

| Path | Role |
|------|------|
| `docs/LOAD_ACCEPTED_CAPTURE.md` | **New** primary operator page (A/B/C) |
| `docs/CAPTURE_MATERIAL_MLIR_AIE.md` | §3b completed-run; rule 7; checklist; Related |
| `tools/load_accepted_path_status.sh` | **New** read-only path availability |
| `testdata/xdna2_cmd/external_known_good/PROVENANCE.md` | Link + refuse re-tag |
| `RESIDUAL.md` | Latest Turn + plain status |
| `AGENTS.md` | Latest Turn pin |
| `.agents/reports/impl-w5-path-docs.md` | This report |
