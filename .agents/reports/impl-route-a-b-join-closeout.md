# Join closeout: Route A matmul material + Route B structure tools

**Date:** 2026-08-10
**Role:** L2-R residual pin + status closeout
**Plan:** `.agents/plans/plan-route-a-b-material-structure.md`
**Scope:** residual honesty pin, offline `--status`, join report only
**Honesty footer:** invent_pass=0 · Pass=No · unlock=none · no live · no mint

---

## What Route A delivered

Second public npu2 AOT set under `testdata/xdna2_cmd/external_known_good/`:

| Item | Value |
|------|--------|
| Set | `mlir_aie_single_core_matmul_npu2.*` |
| ctrltext | **660 B**, sha16 **`de8671245bb72436`** |
| Dims / dtype | M=K=N=64, tiles 32, i16→i32, npu2 AOT |
| PDI | 5536 B (research ingest; not product known-good) |
| Primary soft ctrltext | **still** vector: `working_stack_ctrltext.bin` **300 B** sha16 `aaeb5b68cc298a45` |
| Vector set | **preserved** (`mlir_aie_vector_scalar_add_npu2.*`) |
| Soft vs lab classic 1688 | **mismatch expected** |
| Flexml match claim | **No** — 660 B IRON matmul ≠ 14–27 KiB flexml PDI class |

Report: `.agents/reports/impl-route-a-matmul-material.md`
Playbook: `docs/CAPTURE_MATERIAL_MLIR_AIE.md`

---

## What Route B delivered

Pure offline structural classifiers in `silicon_contract.rs` (+ re-exports,
soft PRINT in `classify_last_apply`):

| Slice | Capability |
|-------|------------|
| B2 | `CtrltextSizeClass` + multi-axis `research_external_vs_lab_structural_distance` |
| B1 | Opcode histogram + txn header soft on external-vs-lab compare |
| B3 | `fingerprint_capture_material_dir_research` multi-file bag fingerprint |
| B4 | optional `compare_rela_structure_lab_vs_external_elf` |

All soft notes hard-code invent_pass=0 / not_c_ready=1 / not_ran_on_npu=1 /
product_known_good=0. Distance or structural match **≠** Pass.

Report: `.agents/reports/impl-route-b-structure-tools.md`
Tests (implementer): structural_distance / external_ctrltext_compare /
capture_material / rela_structure green; fmt + clippy -D lib clean.

---

## Residual pin (this closeout)

| Field | Value |
|-------|--------|
| Capture | dual material present (vector 300 + matmul 660) |
| Structure tools | landed; residual can cite size class / op hist L1 / headers |
| Residual class | still post-attach **`wait_cmd_einval`** (flexml `2ae69036`) |
| Pass | **No** |
| invent_pass | **0** |
| unlock | **none active** (`2ae69036` + priors CONSUMED) |
| ran_on_npu | **false** / not claimed |
| Matmul = flexml? | **No** (explicit ban) |

Disk pins: `RESIDUAL.md` top table + three needs + ranked next; `AGENTS.md`
Latest Turn Route A+B join.

---

## Offline status (this turn)

```bash
bash tools/pass_unblock_once.sh --status
```

Observed:

- Product Pass: **No**
- invent_pass=0 product_pass=0 ran_on_npu=0
- Unlock: **none** (first `C_UNLOCK=` CONSUMED)
- capture_bins=5 (script counts `*.bin`/`*.ctrltext` only), blocked_on_external_bytes=0
- HUMAN_NEEDED=0 PRIVILEGE_NEEDED=0
- flexml live `2ae69036` CONSUMED not Pass

**Not run:** L3, `XDNA2_ALLOW_EXEC_CMD`, live apply, unlock mint, re-live spent
flexml, empty reconfirm mill.

---

## When a future unlock would be legitimate (do not mint here)

A **new** host-minted `C_UNLOCK` is only justified with a **named offline
reason**, for example:

1. **Unspent flexml topology** for a deliberate live CONFIG_CU shot: llm-2x4x4
   sha16 `ba0cd738fd870011` (21664 B) or unified-4x4 `7d172bf223a9b06b` (13792 B),
   with residual class still post-attach `wait_cmd_einval` learned under spent
   `2ae69036`, optional debug-BO prep (C6 offline only), full last_apply +
   journal plan.
2. **Justified new material** beyond the dual public AOT sets that changes the
   residual hypothesis (e.g. true load-accepted / residual-matched identity, not
   mere presence of 660 B matmul ctrltext).
3. **New failure-mode evidence** requiring a live sandwich the offline path
   cannot replace.

**Not enough alone:** dual material landing; structural distance tools;
soft-compare mismatch; empty reconfirm; reopening CONSUMED unlocks.

Unlock ≠ Pass ≠ `ran_on_npu`. Mint only as a **separate intentional step**, not
in this closeout.

---

## Operator note

Operator may **reboot when convenient**. This slice does **no** agent live
device work, no wedge risk, no sandwich prep.

---

## Status footer (mandatory)

```
invent_pass=0
product_pass=0
ran_on_npu=0
Pass=No
C_UNLOCK_active=none
C_UNLOCK_minted=0
route_a=matmul_660_landed
route_b=structure_tools_landed
vector_primary_300=aaeb5b68cc298a45
matmul_ctrltext_660=de8671245bb72436
matmul_is_flexml_matched=0
residual_class=wait_cmd_einval_post_attach
live_this_slice=0
l3=0
allow_exec=0
empty_reconfirm_mill=0
operator_reboot=optional_when_convenient
```

---

## Files touched this closeout

| Path | Change |
|------|--------|
| `RESIDUAL.md` | Top honesty: dual material + structure tools; three needs; ranked next |
| `AGENTS.md` | Latest Turn Route A+B join pin |
| `.agents/reports/impl-route-a-b-join-closeout.md` | this report |

No product code. No capture bin edits. No git commit.
