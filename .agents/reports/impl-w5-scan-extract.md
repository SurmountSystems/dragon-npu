# impl W5-2 / W5-3 / W5-4 — offline capture automation

**Date:** 2026-08-11
**Scope:** host capture scan, load-accepted PROVENANCE template, fail-closed collect helper
**Mode:** offline only

## Honesty footer (required)

| Token | Value |
|-------|--------|
| invent_pass | **0** |
| Pass | **No** |
| ran_on_npu | **no** / not_ran_on_npu=1 |
| not_c_ready | 1 |
| load_accepted_claim | **0** (tools never invent this) |
| residual_matched_claim | **0** |
| Live apply / ALLOW / L3 | **not used** |
| Network install | **not used** |
| Invent capture bytes | **no** |

Multi-txn **32208** and known research-duplicate sha16 (public mlir-aie 300 /
660 / 2592 / 4512 peels) **must not** be claimed load-accepted. Existing
research-only rows in `external_known_good/` were **not** re-tagged.

## What landed

### W5-2 Host capture scan

| Item | Path |
|------|------|
| New tool | [`tools/host_capture_scan.sh`](../../tools/host_capture_scan.sh) |
| Report (when run) | `.agents/reports/wave5-host-capture-scan.md` |
| Cross-link | [`tools/host_npu_stack_inventory.sh`](../../tools/host_npu_stack_inventory.sh) header + who-acts lines |

**Behavior (read-only, exit 0 soft):**

- Walks candidate roots: `/tmp`, `~/.cache`, `ryzen-ai-install`,
  `mlir-aie-capture`, `RyzenAI-SW` / `RyzenAI`, XRT sample paths, FastFlowLM /
  Lemonade paths if present, in-tree `gt_capture` / bag / `silicon_lab`
- For each capture-like file (bounded maxdepth=5, list_cap=48): size, sha16,
  first 8 bytes hex, `refuse_class`
- Refuse classes: `multi_txn_32208`, `research_duplicate_sha16`,
  `large_soft` / `large_container_soft`, else `none` with
  `load_accepted_claim=0` always
- Optional durable report via default or `--report`; `--stdout-only` skips write

```bash
bash tools/host_capture_scan.sh
bash tools/host_capture_scan.sh --stdout-only
bash tools/host_capture_scan.sh -h
```

### W5-3 PROVENANCE load-accepted template

| Item | Path |
|------|------|
| Updated | [`testdata/xdna2_cmd/external_known_good/PROVENANCE.md`](../../testdata/xdna2_cmd/external_known_good/PROVENANCE.md) |

**Added section:** *Load-accepted vs research-only (W5-3)* with:

- Table: research_only vs load_accepted when honest / forbidden
- Hard rules: invent_pass=0, multi-txn 32208 refuse, research-duplicate sha refuse
- Load-accepted capture table fields:
  - `completed_on_this_fw=yes|no`
  - `stack` (FastFlowLM / IRON example / xrt-smi / DynamicDispatch / …)
  - `firmware version`
  - `how extracted`
  - invent_pass=0 / claim_class / residual_matched_claim rules
- Explicit: **do not re-tag** existing 300 / 660 / 2592 / 4512 research rows
- Operator shortcuts list now includes W5 scan + collect helpers

### W5-4 Extract / collect helper

| Item | Path |
|------|------|
| New tool | [`tools/collect_load_accepted_candidate.sh`](../../tools/collect_load_accepted_candidate.sh) |
| Wraps | `capture_from_host_candidate.sh` + `pass_unblock_once.sh --ingest` |
| Cross-link | `capture_from_host_candidate.sh` usage Related lines |

**Behavior:**

| Mode | Action | load_accepted_claim |
|------|--------|---------------------|
| default / `--dry-run PATH [NAME]` | classify only | always 0 |
| `--ingest PATH [NAME]` | pass_unblock ingest only if not refused | still 0 after copy |

**Refuse exit 2:**

- missing / empty path
- size **32208** (multi-txn wrong body)
- sha16 in known research peel set
- refused oracle / VE2 basenames
- unsafe install NAME

Ingest still stores **research material only**; operator must fill PROVENANCE
load-accepted fields only when live ERT Completed + OFM allclose evidence exists
(`decide_ran_on_npu`). Helper never sets invent_pass=1 or residual_matched_claim=1.

```bash
bash tools/collect_load_accepted_candidate.sh --dry-run /path/to/candidate.bin
bash tools/collect_load_accepted_candidate.sh --ingest /path/to/new.bin working_stack_ctrltext.bin
bash tools/collect_load_accepted_candidate.sh -h
```

## Commands to verify (operator / agent offline)

```bash
# Syntax
bash -n tools/host_capture_scan.sh
bash -n tools/collect_load_accepted_candidate.sh
bash -n tools/host_npu_stack_inventory.sh
bash -n tools/capture_from_host_candidate.sh

# Soft scan (writes wave5-host-capture-scan.md)
bash tools/host_capture_scan.sh

# Fail-closed smokes (expect exit 2, invent_pass=0)
# multi-txn if present:
#   bash tools/collect_load_accepted_candidate.sh --dry-run \
#     testdata/xdna2_cmd/gt_capture/instr_vendor_post_patch.bin
# research-duplicate (bag vector peel, if present):
#   bash tools/collect_load_accepted_candidate.sh --dry-run \
#     testdata/xdna2_cmd/external_known_good/mlir_aie_vector_scalar_add_npu2.ctrltext.bin
```

**Note:** This implement slice authored the tools and docs offline. Re-run the
commands above on the host to refresh `wave5-host-capture-scan.md` and confirm
exit codes. Static review: fail-closed axes and honesty footers are present in
both new scripts; no ALLOW / EXEC_CMD / unlock mint paths.

## Files changed

| Path | Change |
|------|--------|
| `tools/host_capture_scan.sh` | **new** W5-2 read-only scan |
| `tools/collect_load_accepted_candidate.sh` | **new** W5-4 fail-closed collect |
| `testdata/xdna2_cmd/external_known_good/PROVENANCE.md` | W5-3 load-accepted template + shortcuts |
| `tools/host_npu_stack_inventory.sh` | Related + who-acts pointers to W5 tools |
| `tools/capture_from_host_candidate.sh` | Related pointers |
| `.agents/reports/impl-w5-scan-extract.md` | this report |

## Not done / frozen

- No live apply, no C_UNLOCK mint, no Pass, no invent bytes
- No re-tag of public mlir-aie research AOT as load-accepted
- Operator still owns true load-accepted material (completing stack evidence)
- L8 / syncobj live still not opened from this slice

## Who acts next

1. **Operator:** place true completing-stack capture under
   `testdata/xdna2_cmd/external_known_good/` and fill the load-accepted table
   only when `completed_on_this_fw=yes` with firmware + stack + extract method.
2. **Agent:** run `host_capture_scan` / dry-run collect offline; refuse
   multi-txn and research-duplicate as load-accepted; no empty reconfirm mill.
