# Wave5 Path B matmul attempt — IRON npu2 larger examples completed

**Date:** 2026-08-11
**Repo:** `/home/hunter/Projects/ai/dragon-npu`
**Capture tree:** `/home/hunter/Projects/ai/mlir-aie-capture`
**Operator ask:** keep doing capture work after vector_scalar_add Path B PASS
**Honesty footer:** invent_pass=0 · Pass=No · no C_UNLOCK · no L3 · no ALLOW

---

## Summary (one screen)

| Item | Result |
|------|--------|
| **single_core matmul NPU EXEC** | **yes** (IRON `PASS!`, exit 0) |
| **whole_array matmul NPU EXEC** | **yes** (IRON `PASS!`, exit 0) |
| **cascade matmul NPU EXEC** | **yes** (IRON `PASS!`, exit 0) |
| **New ctrltext identity** | **no** — all three match already-landed RESEARCH ONLY peels |
| **load_accepted_claim** | **0** (helper refuse re-tag 660/2592/4512) |
| **invent_pass** | **0** |
| **product Pass / `ran_on_npu`** | **No** (IRON PASS ≠ product `decide_ran_on_npu`) |
| **Ingest of new load-accepted bins** | **refused** (research_duplicate_sha16) |
| **completed_on_this_fw** | **yes** for 300/660/2592/4512 public peels (vector earlier + these three) |

---

## Environment (same Path B fix as vector)

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
| `import aie.iron` + `import pyxrt` | ok |
| system pyxrt | `/usr/lib/python3.14/site-packages/pyxrt…so` |
| device | `RyzenAI-npu6` |
| NPU FW | **1.1.2.64** (prior examine) |
| `/dev/accel/accel0` | present, open OK |

**Key:** system site-packages on `PYTHONPATH` (capture venv alone lacks pyxrt).

---

## Runs

### RUN1 — single_core matmul 64³ i16→i32 (660 B class)

```bash
cd $MLIR_AIE/programming_examples/basic/matrix_multiplication/single_core
timeout 900 python3 single_core.py --dev npu2 \
  -M 64 -K 64 -N 64 -m 32 -k 32 -n 32 \
  --dtype_in i16 --dtype_out i32 -w 1 -i 2
```

| Field | Value |
|-------|--------|
| exit | **0** |
| stdout | **`PASS!`** |
| NPU time (avg/min/max us) | 164.5 / 149.6 / 179.4 |
| NPU GFLOPS | 3.19 |
| wall | ~7 s (JIT + run) |
| Evidence | `$CAP/completed_run_single_core_matmul_20260811_150700/` |
| ctrltext | **660 B** sha16 **`de8671245bb72436`** (matches bag) |

### RUN2 — whole_array matmul 128³ 4-col i16→i32 (2592 B class)

```bash
cd $MLIR_AIE/programming_examples/basic/matrix_multiplication/whole_array
timeout 1200 python3 whole_array.py --dev npu2 \
  -M 128 -K 128 -N 128 -m 16 -k 16 -n 16 \
  --n-aie-cols 4 --dtype_in i16 --dtype_out i32 -w 1 -i 2
```

| Field | Value |
|-------|--------|
| exit | **0** |
| stdout | **`PASS!`** |
| NPU time (avg/min/max us) | 296.8 / 277.7 / 315.8 |
| NPU GFLOPS | 14.13 |
| wall | ~10 s |
| Evidence | `$CAP/completed_run_whole_array_matmul_20260811_150751/` |
| ctrltext | **2592 B** sha16 **`7ee4b267ce31cd70`** (matches bag) |

### RUN3 — cascade matmul 128³ 4-col i16→i32 (4512 B class)

```bash
cd $MLIR_AIE/programming_examples/basic/matrix_multiplication/cascade
timeout 1200 python3 cascade.py --dev npu2 \
  -M 128 -K 128 -N 128 -m 32 -k 32 -n 32 \
  --n-aie-cols 4 --dtype_in i16 --dtype_out i32 -w 1 -i 2
```

| Field | Value |
|-------|--------|
| exit | **0** |
| stdout | **`PASS!`** |
| NPU time (avg/min/max us) | 1125.5 / 1048.7 / 1202.2 |
| NPU GFLOPS | 3.73 |
| wall | ~10 s |
| Evidence | `$CAP/completed_run_cascade_matmul_20260811_150822/` |
| ctrltext | **4512 B** sha16 **`18cb653c05a9d4c8`** (matches bag) |

---

## Collect / load-accepted helper

All three dry-runs:

```bash
bash tools/collect_load_accepted_candidate.sh --dry-run <ctrltext> <install_name>
```

| Peel | sha16 | class | load_accepted_claim |
|------|-------|-------|---------------------|
| single_core 660 | `de8671245bb72436` | `research_duplicate_sha16` | **0** |
| whole_array 2592 | `7ee4b267ce31cd70` | `research_duplicate_sha16` | **0** |
| cascade 4512 | `18cb653c05a9d4c8` | `research_duplicate_sha16` | **0** |

No `--ingest` of new load-accepted names (would only re-copy research-dup).
PROVENANCE updated: `completed_on_this_fw=yes` for these three classes.

### Journal scrape (soft)

```bash
bash tools/journal_npu_scrape.sh --hours 1
```

| Field | Value |
|-------|--------|
| match_total | **0** |
| honesty | journal scrape alone ≠ Pass |

---

## Files staged

| Path | Role |
|------|------|
| `$CAP/completed_run_single_core_matmul_20260811_150700/` | run.log, RUN_EVIDENCE.txt, SHA256SUMS, AOT sibling peels |
| `$CAP/completed_run_whole_array_matmul_20260811_150751/` | same pattern |
| `$CAP/completed_run_cascade_matmul_20260811_150822/` | same pattern |
| `testdata/xdna2_cmd/external_known_good/PROVENANCE.md` | Path B matmul section + honesty refresh |
| bag binaries | **unchanged** (same sha; no re-tag) |

**No new distinct binary identity** under capture dir.

---

## Honesty matrix

| Claim | Value | Why |
|-------|-------|-----|
| invent_pass | **0** | Always |
| NPU EXEC completed (IRON Path B) | **yes** (3/3 matmul + prior vector) | exit 0, `PASS!` |
| completed_on_this_fw | **yes** for 300/660/2592/4512 public peels | IRON verify on FW 1.1.2.64 |
| load_accepted_claim | **0** | research_duplicate_sha16; bag policy refuse re-tag |
| residual_matched_claim | **0** | sizes ≠ lab classic 1688; not flexml match |
| product Pass | **No** | No dragon-npu decide / unlock recipe |
| product `ran_on_npu` | **false** | Not claimed |
| C_UNLOCK / L3 / ALLOW | **none** | Not opened |
| Re-tag aaeb5b68 / de867124 / 7ee4b267 / 18cb653c as load-accepted | **refused** | Helper + PROVENANCE policy |

---

## What this means for residual

1. Path B works for **larger** IRON matmul examples on this host (not only vector_scalar_add).
2. Public 660 / 2592 / 4512 peels are **not broken** on FW 1.1.2.64 under IRON/XRT.
3. That does **not** invent product Pass, residual match to lab classic 1688, or load-accepted re-tag.
4. True load-accepted / residual progress still needs **different** completing-stack bytes (working-stack residual-matched peels, e.g. DD/mladf path), not re-label of public mlir-aie AOT classes.

---

## Footer

```
invent_pass=0
not_c_ready=1
not_ran_on_npu=1
Pass=No
load_accepted_claim=0
npu_exec_completed_iron=1
research_duplicate_sha16=de8671245bb72436,7ee4b267ce31cd70,18cb653c05a9d4c8
completed_on_this_fw=yes
unlock=none
l3=not_opened
allow=unset
```
