# Residual honesty pin — mlir-aie capture material landed

**Date:** 2026-08-10
**Scope:** docs / residual honesty only. **No** live apply. **No** unlock mint.
**Honesty:** invent_pass=0 · Pass=No · ran_on_npu=No · product_known_good=0 ·
not_c_ready=1

---

## What landed

Verified under `testdata/xdna2_cmd/external_known_good/` (was bins=0 / PROVENANCE-only):

| File | Notes |
|------|--------|
| `working_stack_ctrltext.bin` | **300 B**, sha16 `aaeb5b68cc298a45` |
| `mlir_aie_vector_scalar_add_npu2.ctrltext.bin` | same 300 B / same sha16 |
| `mlir_aie_vector_scalar_add_npu2.insts.bin` | same 300 B |
| `mlir_aie_vector_scalar_add_npu2.insts.elf` | 992 B (aiebu ELF) |
| `mlir_aie_vector_scalar_add_npu2.xclbin` | 8983 B |
| `mlir_aie_vector_scalar_add_npu2.pdi` | 2528 B |
| `mlir_aie_vector_scalar_add_npu2_core.elf` | 2468 B |
| `PROVENANCE.md` | source/commit/build/hashes updated |

Playbook + implement report:

- `docs/CAPTURE_MATERIAL_MLIR_AIE.md`
- `.agents/reports/impl-mlir-aie-material-playbook.md`

**What this is:** public mlir-aie `vector_scalar_add` **npu2** AOT research
material. Offline ingest/classify can run.

**What this is not:** matmul-matched residual identity vs lab classic mladf
**1688 B** ctrltext. Soft compare size mismatch (**300 vs 1688**) is
**expected**. Material alone is **not** product known-good, **not** C-ready,
**not** Pass, **not** `ran_on_npu`, **not** a live unlock reason by itself.

---

## Residual class (unchanged for Pass)

- Still **no** product Pass; invent_pass=0; **no** `ran_on_npu`.
- Last spent live: flexml unified-2x4x4 under **`C_UNLOCK=2ae69036` CONSUMED**
  → status `wait_cmd_einval` after CONFIG_CU attach (27200 B, sha16
  `0eeeb70c…`); journal cmd timeout / backtrack. Better information than pure
  APP_LOAD / CVML orphan IO_PAGE_FAULT; **not** Pass.
- **Unlock:** none active. Do **not** mint from this pin. Do not reopen
  CONSUMED (`2ae69036`, `97f38307`, `11bc9016`, `3a70fabf`, `b17b0119`).

---

## Disk updates this pin

| Path | Change |
|------|--------|
| `RESIDUAL.md` top plain-status + creative avenues | bins=0 / empty capture claims corrected; who-acts-next after material; honesty invent_pass=0 |
| `AGENTS.md` Latest Turn | short residual honesty pin for material landed |
| this report | brief SSoT for the pin |

---

## Who acts next

1. **Optional human material:** matmul-matched or working-stack ctrltext that
   can close identity vs lab classic 1688, or intercept of a stack that already
   loads. Present vector_scalar_add bins stay research-only offline identity.
2. Prefer **minimal load** for next live; unspent research flexml:
   llm-2x4x4 `ba0cd738fd870011` / unified-4x4 `7d172bf223a9b06b`.
3. Next live **only** under a **new** host-minted unlock with a named reason
   (unspent topology and/or justified new material + residual class from
   `2ae69036`). Capture landing alone is **not** unlock.
4. **Frozen:** empty reconfirm as if capture were still empty; reopen CONSUMED;
   Pass-from-attach; Pass-from-capture; invent_pass.

**Agent-legal only:** classify/ingest present or new human bytes; unlock
justification text; residual splitters only on new last_apply failure modes.
No live apply from this pin.

---

## Stop conditions

- Do not re-spin empty implement because “bins used to be 0.”
- Do not claim Pass, C-ready, or product known-good from these bins.
- Do not mint or reopen unlock from this report.
