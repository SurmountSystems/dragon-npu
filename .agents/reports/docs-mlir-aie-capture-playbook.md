# Report: MLIR-AIE → capture directory material playbook (docs pin)

**Date:** 2026-08-10
**Scope:** Docs / residual pin only. No product Rust. No unlock mint. No invent bins.
**Honesty:** invent_pass=0 · product_pass=0 · not_ran_on_npu=1 · Pass=No

---

## Why this pin

Agents were treating “mlir-aie (or IRON) clone exists” as if Step 1 material
were done. Residual tools (`pass_unblock_once`, soft compare, bag-drop checks)
need **concrete binaries** under
`testdata/xdna2_cmd/external_known_good/` or an explicit
`bash tools/pass_unblock_once.sh --ingest PATH [NAME]`.

Verified from `tools/pass_unblock_once.sh`:

- Usage: `--ingest PATH [NAME]`
- Missing / refused path → exit **2**
- PATH is required; NAME is optional single path component

---

## What landed

| Path | Change |
|------|--------|
| [`docs/CAPTURE_MATERIAL_MLIR_AIE.md`](../../docs/CAPTURE_MATERIAL_MLIR_AIE.md) | **New** full playbook: clone recursive → npu2 env → smallest examples → collect xclbin/insts/elf/ctrltext → ingest → PROVENANCE → residual pin; Peano/aiebu/IRON/FastFlowLM fallback; honesty footer |
| [`docs/CONSTRUCTION_MAP_APP_LOAD.md`](../../docs/CONSTRUCTION_MAP_APP_LOAD.md) | Path-B playbook: hard rule source≠bins; MLIR-AIE subsection + highest-value next links |
| [`RESIDUAL.md`](../../RESIDUAL.md) | Who acts next + creative avenues: material rule + playbook link; refuse “clone exists” as progress |
| [`.agents/plans/plan-pass-unblock-sequence.md`](../plans/plan-pass-unblock-sequence.md) | Step 1 operator sequence + board + human/host table → playbook |
| [`.agents/plans/plan-pass-unblock-creative-avenues.md`](../plans/plan-pass-unblock-creative-avenues.md) | Avenue A → playbook; source tree ≠ capture material |
| [`testdata/xdna2_cmd/external_known_good/PROVENANCE.md`](../../testdata/xdna2_cmd/external_known_good/PROVENANCE.md) | Short form points at playbook; PATH required on ingest |

---

## Playbook essence (for scanners)

1. Clone [mlir-aie](https://github.com/Xilinx/mlir-aie) **recursive**; record commit.
2. Env for **npu2**; Peano / aiebu / upstream install.
3. Build **smallest** example: passthrough_kernel / vector_scalar_add /
   single_core matmul / getting_started.
4. Collect `*.xclbin`, `insts_*.bin`, aiebu elf, `.ctrltext`.
5. Copy into `testdata/xdna2_cmd/external_known_good/` via
   `bash tools/pass_unblock_once.sh --ingest PATH [NAME]`.
6. PROVENANCE: repo+commit, example path, build cmd, npu2, sizes+sha,
   invent_pass=0, not proven Pass on this firmware.
7. Soft compare / status; residual pin **only after real files exist**.
8. Supporting: Peano, aiebu, AMD IRON, FastFlowLM/Lemonade fallback.
9. Does **not**: claim Pass, mint unlock, invent `ran_on_npu`.

---

## Explicitly not done

- Product Rust changes
- Unlock mint or reopen CONSUMED `C_UNLOCK`
- Invent capture bins
- Live apply / L3 / ALLOW
- Product Pass / `ran_on_npu`

---

## Residual honesty after pin

- Capture dir may still have **bins=0** (docs only; no new files dropped).
- Active unlock: **none**
- Pass: **No**
- Next human action: follow playbook until allowed files exist under the
  capture directory, then agent may ingest/classify.

---

## Footer

```
invent_pass=0
product_pass=0
not_ran_on_npu=1
not_c_ready=1
source_tree_is_not_capture_material=1
docs_only=1
```
