# Status report: accomplishments and blockers

**Date:** 2026-08-10
**SSoT residual:** `RESIDUAL.md` top table
**Live status tool:** `bash tools/pass_unblock_once.sh --status`
**Honesty:** invent_pass=0 · Pass=No · ran_on_npu=0 · unlock=none active

---

## Snapshot (one screen)

| Item | Value |
|------|--------|
| Product Pass | **No** |
| ran_on_npu | **No** |
| invent_pass | **0** |
| Active unlock | **None** (all CONSUMED) |
| Capture dir | **Has real bins** (dual public npu2 AOT sets) |
| capture_bins (script) | **5** (`*.bin` / `.ctrltext` count) |
| blocked_on_external_bytes | **0** |
| HUMAN_NEEDED (empty bag) | **0** |
| Residual class (last live) | post-attach **`wait_cmd_einval`** (flexml `2ae69036`) |
| Soft vs lab classic 1688 B | **mismatch expected** (not residual close) |

---

## What was accomplished

### 1. Empty-capture wall cleared

Earlier residual tools could only say the capture directory was empty. That is
**fixed**.

- Playbook: `docs/CAPTURE_MATERIAL_MLIR_AIE.md` (source clone ≠ capture material)
- Tooling: `tools/pass_unblock_once.sh` (`--status` / `--ingest PATH` / offline)
- Offline gates, P0–P2 classifiers (C1–C6 research), honesty locks

### 2. Dual public npu2 material landed (Route A family)

Under `testdata/xdna2_cmd/external_known_good/`:

| Set | Primary size | sha16 | Role |
|-----|--------------|-------|------|
| **vector** `vector_scalar_add` | **300 B** ctrltext | `aaeb5b68cc298a45` | Soft-compare **primary** (`working_stack_ctrltext.bin`) |
| **matmul** `single_core` 64³ i16→i32 | **660 B** ctrltext | `de8671245bb72436` | Second set (`mlir_aie_single_core_matmul_npu2.*`) |

Also present per set: insts, aiebu ELF, xclbin, PDI, core.elf. Full PROVENANCE.

Scale honesty (not flexml-matched):

| Class | Approx size |
|-------|-------------|
| Public vector ctrltext | 300 B |
| Public matmul ctrltext | 660 B |
| Lab classic mladf peel | 1688 B |
| Unspent flexml PDI candidates | 14–22 KiB (`7d172bf2…` / `ba0cd738…`) |
| Spent flexml unified-2x4x4 | 27200 B (`0eeeb70c…`) |

Reports: `impl-mlir-aie-material-playbook.md`, `impl-route-a-matmul-material.md`

### 3. Stronger offline structure tools (Route B)

Pure TDD in `silicon_contract.rs` (always invent_pass=0):

- Multi-axis `research_external_vs_lab_structural_distance` + size classes
- Opcode histogram + per-side txn headers on external-vs-lab compare soft notes
- Capture-dir multi-file fingerprint
- Optional ELF rela structure compare

Residual can cite shape/topology distance, not only “300 vs 1688.” Distance **≠** Pass.

Report: `impl-route-b-structure-tools.md`

### 4. Live evidence progress (historical; not Pass)

Under spent unlocks, stock path reached **CONFIG_CU attach** for flexml unified-2x4x4
(`2ae69036`, pdi 27200, sha16 `0eeeb70c…`). Residual class moved to post-attach
**`wait_cmd_einval`** (better than pure APP_LOAD / CVML orphan IO_PAGE_FAULT alone).
Attach **≠** Pass. That fingerprint is **spent** (do not re-live alone).

### 5. Offline product readiness around silicon honesty

- WAIT EINVAL vs false wait_timeout classification
- PDI identity / spent-unspent ledger
- Operator one-shot: `pass_unblock_once.sh`
- No invent Pass from soft tools

Join pin: `impl-route-a-b-join-closeout.md`

---

## What is still blocking Pass

### Primary wall: firmware never completes a product job

Pass requires **all** of:

1. Real ioctl path with honest gates
2. Job accepted and **ERT Completed** (not Error / wait_cmd_einval forever)
3. Non-zero OFM + product allclose via `decide_ran_on_npu`

Present state: userspace + CONFIG_CU attach can be green; **FW still does not
deliver Completed + OFM allclose**. invent_pass=0.

### Material is real but not residual-matched

| Have | Missing |
|------|---------|
| Public IRON AOT vector 300 + matmul 660 | True load-accepted / residual-matched image beyond spent flexml |
| Soft compare + structural distance | Lab classic 1688 identity match (not claimed) |
| Flexml attach once | Deliberately **different** next image (unspent topology or justified new material) |

**Do not claim** 660 B single_core matmul = flexml PDI class or lab 1688 identity.

### No active live unlock

| Unlock | State |
|--------|--------|
| `2ae69036` flexml unified-2x4x4 | **CONSUMED** (attach + wait_cmd_einval) |
| `97f38307` CVML research | **CONSUMED** |
| `11bc9016`, `3a70fabf`, `b17b0119` | **CONSUMED** |

Live stock shot is **refused** until a **new** host-minted unlock **and** a
**named offline reason**. Dual capture + structure tools alone **≠** unlock.

### Unspent candidates (not yet live)

| tag | pdi_bytes | sha16 | Note |
|-----|-----------|-------|------|
| flexml llm-2x4x4 | 21664 | `ba0cd738fd870011` | Preferred unspent candidate |
| flexml unified-4x4 | 13792 | `7d172bf223a9b06b` | Weaker topology A/B |

### Parked / frozen

| Item | Status |
|------|--------|
| Driver/MMU orphan class | **Parked** (flexml live had no IO_PAGE_FAULT; need new journal) |
| Empty reconfirm mill | **Frozen** |
| Re-live spent fingerprint alone | **Banned** |
| Pass from attach / capture bins / distance tools | **Banned** |

### Status script wording lag (not product Pass)

`--status` may still list “bag empty” as need (1) even when `capture_bins=5`.
Disk residual + this report supersede: empty bag is **done** for dual public
AOT; residual-matched identity is still open.

---

## What does *not* block anymore

- Empty capture directory
- Missing offline classify/ingest entrypoint
- Missing structural distance beyond size-only mismatch
- Missing operator playbook for mlir-aie → capture dir
- (On this stock host) sudo L3 for stock config-cu-run path

---

## Ranked next (honest)

1. **Optional offline:** deeper Route B classify on dual material; point soft
   compare at matmul peel; larger geometry only with named reason. invent_pass=0.
2. **Next live (operator-gated):** new unlock only with named reason, e.g.
   unspent flexml `ba0cd738…` / `7d172bf2…` + post-attach residual from
   `2ae69036` + full last_apply/journal plan. Optional C6 debug-BO prep
   (research offline only until unlocked).
3. **Do not:** mint unlock from this report alone; re-live spent flexml;
   claim Pass; empty reconfirm; agent live sandwich without unlock.

Unlock ≠ Pass ≠ ran_on_npu.

---

## Key paths

| Path | Role |
|------|------|
| `RESIDUAL.md` | Living residual SSoT |
| `docs/CAPTURE_MATERIAL_MLIR_AIE.md` | Material playbook |
| `testdata/xdna2_cmd/external_known_good/` | Capture binaries + PROVENANCE |
| `tools/pass_unblock_once.sh` | Status / ingest / offline |
| `.agents/plans/plan-route-a-b-material-structure.md` | Last dual-route plan |
| `.agents/reports/impl-route-a-b-join-closeout.md` | A+B join pin |

---

## Footer

```
invent_pass=0
product_pass=0
ran_on_npu=0
Pass=No
C_UNLOCK_active=none
capture_bytes_present=1
```
