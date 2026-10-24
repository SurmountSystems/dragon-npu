# Wave5 Path B agent attempt — IRON/mlir-aie completed NPU run

**Date:** 2026-08-11
**Repo:** `/home/hunter/Projects/ai/dragon-npu`
**Operator ask:** "lol no u do it" — agent must attempt Path B, not hand checklists.
**Honesty footer:** invent_pass=0 · Pass=No · no C_UNLOCK · no L3 · no ALLOW

---

## Summary (one screen)

| Item | Result |
|------|--------|
| **NPU EXEC completed** | **yes** (IRON verify `PASS!`, exit 0) |
| **load_accepted_claim** | **0** (research-duplicate sha16 `aaeb5b68…`; helper refuse re-tag) |
| **invent_pass** | **0** |
| **product Pass / `ran_on_npu`** | **No** (IRON PASS ≠ product `decide_ran_on_npu`) |
| **Ingest of new load-accepted bins** | **refused** by policy (same 300 B peel already RESEARCH ONLY) |
| **Evidence dir** | `/home/hunter/Projects/ai/mlir-aie-capture/completed_run_vector_scalar_add_20260811_150247/` |
| **Blocker for load-accepted re-tag** | Known research peel sha; need **new** completing-stack bytes (different program / residual-matched), not re-label of 300/660/2592/4512 |

---

## Environment used

```text
VENV=/home/hunter/Projects/ai/mlir-aie-capture/venv
CAP=/home/hunter/Projects/ai/mlir-aie-capture
MLIR_AIE=/home/hunter/Projects/ai/mlir-aie
PATH=$CAP/tools:$VENV/lib/python3.14/site-packages/mlir_aie/bin:$VENV/bin:/usr/bin
PEANO_INSTALL_DIR=$VENV/lib/python3.14/site-packages/llvm-aie
PYTHONPATH=/usr/lib/python3.14/site-packages:$VENV/lib/python3.14/site-packages/mlir_aie/python
```

| Probe | Result |
|-------|--------|
| `import aie.iron` | ok |
| `/dev/accel/accel0` | present; open RDWR ok (user in `render`) |
| `xrt-smi examine` | XRT 2.21.75; amdxdna; **NPU Firmware 1.1.2.64**; device **RyzenAI-npu6** |
| pyxrt in capture venv alone | **missing** |
| system pyxrt | `/usr/lib/python3.14/site-packages/pyxrt.cpython-314-x86_64-linux-gnu.so` |
| Prior AOT dir | `$CAP/aot_vector_scalar_add/` present (research AOT from 2026-08-10) |

**Key fix for Path B on this host:** put **system** site-packages on `PYTHONPATH` so IRON can `import pyxrt`. Capture venv alone → "no NPU runtime device".

---

## Commands + exit codes

### RUN1 — AOT BYO without system pyxrt

```bash
cd $MLIR_AIE/programming_examples/basic/vector_scalar_add
# PYTHONPATH = capture iron only (no /usr/lib/python3.14/site-packages)
timeout 180 python3 vector_scalar_add.py --dev npu2 \
  --from-xclbin $CAP/aot_vector_scalar_add/vector_scalar_add.xclbin \
  --from-insts  $CAP/aot_vector_scalar_add/vector_scalar_add.insts.bin
```

| Field | Value |
|-------|--------|
| exit | **1** |
| stdout | `no NPU runtime device is available to run the pre-built artifacts` |
| NPU EXEC | **no** |

### RUN2 — AOT BYO **with** system pyxrt

```bash
export PYTHONPATH=/usr/lib/python3.14/site-packages:$VENV/lib/python3.14/site-packages/mlir_aie/python
timeout 180 python3 vector_scalar_add.py --dev npu2 \
  --from-xclbin $CAP/aot_vector_scalar_add/vector_scalar_add.xclbin \
  --from-insts  $CAP/aot_vector_scalar_add/vector_scalar_add.insts.bin
```

| Field | Value |
|-------|--------|
| exit | **0** |
| stdout | `PASS!` + Ran pre-built artifacts (xclbin + insts paths) |
| NPU EXEC | **yes** (IRON host runtime + verify) |
| pyxrt device name | `RyzenAI-npu6` |

### RUN3 — JIT end-to-end `--dev npu2`

```bash
# same PYTHONPATH with system pyxrt
timeout 600 python3 vector_scalar_add.py --dev npu2
```

| Field | Value |
|-------|--------|
| exit | **0** |
| stdout | `PASS!` |
| NPU EXEC | **yes** (compile + run + verify) |

### Collect / load-accepted helper

```bash
bash tools/collect_load_accepted_candidate.sh --dry-run \
  $CAP/completed_run_vector_scalar_add_20260811_150247/vector_scalar_add.ctrltext.bin \
  working_stack_ctrltext_completed_fw.bin
```

| Field | Value |
|-------|--------|
| exit | **2** |
| class | `research_duplicate_sha16` |
| sha16 | `aaeb5b68cc298a45` (matches landed `working_stack_ctrltext.bin` / vector 300 B) |
| load_accepted_claim | **0** (hard refuse re-tag of 300/660/2592/4512) |

No `--ingest` of a new load-accepted name performed (would only re-copy research-dup).

### Journal scrape (soft)

```bash
bash tools/journal_npu_scrape.sh --hours 1
```

| Field | Value |
|-------|--------|
| exit | **0** |
| match_total | **0** (no APP_LOAD / IO_PAGE_FAULT / timeout keywords in window) |
| honesty | journal scrape alone ≠ Pass |

---

## Files found / staged

| Path | Role |
|------|------|
| `$CAP/aot_vector_scalar_add/vector_scalar_add.{xclbin,insts.bin,insts.elf,pdi,ctrltext.bin}` | Prior AOT used for RUN2 |
| `$CAP/completed_run_vector_scalar_add_20260811_150247/` | Staged copy + `RUN_EVIDENCE.txt` + `SHA256SUMS` |
| `testdata/xdna2_cmd/external_known_good/mlir_aie_vector_scalar_add_npu2.*` | Already present RESEARCH ONLY (unchanged tags) |
| `testdata/xdna2_cmd/external_known_good/working_stack_ctrltext.bin` | Same sha16; still research primary soft-compare peel |
| PROVENANCE.md | Updated with Path B completed IRON run section |

**No new distinct binary identity** under capture dir (bytes already landed).

---

## Honesty matrix

| Claim | Value | Why |
|-------|-------|-----|
| invent_pass | **0** | Always |
| NPU EXEC completed (IRON Path B) | **yes** | RUN2 + RUN3 exit 0, `PASS!` |
| completed_on_this_fw (operator Path B sense) | **yes** for these peels | IRON verify on FW 1.1.2.64 |
| load_accepted_claim | **0** | research_duplicate_sha16; bag policy refuse re-tag |
| product Pass | **No** | No dragon-npu decide / unlock recipe |
| product `ran_on_npu` | **false** | Not claimed |
| C_UNLOCK / L3 / ALLOW | **none** | Not opened |
| Re-tag 300 B as load-accepted | **refused** | Helper + PROVENANCE policy |

---

## What this means for residual

1. **Path B works on this host** once system `pyxrt` is on `PYTHONPATH`.
2. Public vector_scalar_add peels are **not broken** on FW 1.1.2.64 under IRON/XRT.
3. That does **not** invent product Pass or residual match for dragon-npu lab classic 1688 / spent flexml.
4. Next material for true load-accepted / residual progress still needs **different** completing-stack bytes (not re-label of aaeb5b68 / de867124 / 2592 / 4512), or residual-matched working-stack peels (e.g. DD/mladf path), still with invent_pass=0.

---

## Footer

```
invent_pass=0
not_c_ready=1
not_ran_on_npu=1
Pass=No
load_accepted_claim=0
npu_exec_completed_iron=1
research_duplicate_sha16=aaeb5b68cc298a45
unlock=none
l3=not_opened
allow=unset
```
