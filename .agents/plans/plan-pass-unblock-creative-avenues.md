# Plan: Pass-unblock creative avenues (rule-respecting)

**Date:** 2026-08-10
**Status:** operator guidance pinned (not a Pass claim; not an unlock mint)
**SSoT residual:** `RESIDUAL.md` top + § Creative avenues
**Prior build plan:** `.agents/plans/plan-residual-pass-build.md` (S0–S5 landed)

## Locked honesty

| Fact | Value |
|------|--------|
| Product Pass / `ran_on_npu` | **No** |
| invent_pass | **0** |
| Active unlock | **None** (`2ae69036` + priors CONSUMED) |
| Capture `testdata/xdna2_cmd/external_known_good/` | bins = **0** (PROVENANCE only) |
| Offline tooling | **Healthy** (`pass_unblock_once`, fingerprint, gates) |
| Empty implement mill | **Frozen** (AGENTS.md §6–9) |

Core diagnosis is **unchanged**. This plan ranks **new material paths** and
**named offline prep** so the next implement loop has real work without inventing
Pass or reopening CONSUMED unlocks.

---

## What still unblocks Pass (unchanged)

| Need | How |
|------|-----|
| Working-stack ctrltext in the capture dir | `--ingest` or place file under `external_known_good/` with PROVENANCE |
| True matmul-matched / load-accepted image beyond spent flexml | New fingerprint + **new** unlock if live; prefer minimal load target first |
| Driver/MMU orphan class | Parked until new evidence (flexml live had no IO_PAGE_FAULT) |

---

## Public sources (verified; accessed 2026-08-10)

| Role | URL |
|------|-----|
| MLIR-AIE + IRON examples (`programming_examples/`, npu2) | https://github.com/Xilinx/mlir-aie |
| AMD IRON operator / LLM examples | https://github.com/amd/IRON |
| Peano / llvm-aie (`aie2p-none-unknown-elf`) | https://github.com/Xilinx/llvm-aie |
| aiebu (asm/dump ELF / ctrltext) | https://github.com/Xilinx/aiebu |
| FastFlowLM (ROCm home) | https://github.com/ROCm/FastFlowLM |
| Lemonade | https://github.com/lemonade-sdk/lemonade |
| amdxdna driver | https://github.com/amd/xdna-driver |
| XRT | https://github.com/Xilinx/XRT |
| Riallto (simpler examples) | https://github.com/AMDResearch/Riallto |

Do not vendor proprietary blobs into git. Capture under
`testdata/xdna2_cmd/external_known_good/` with honest PROVENANCE only.

---

## Ranked avenues

### A — Generate material (highest leverage; often human/host)

**Concrete playbook:** [`docs/CAPTURE_MATERIAL_MLIR_AIE.md`](../../docs/CAPTURE_MATERIAL_MLIR_AIE.md)

**IRON / MLIR-AIE / Peano on npu2:** recursive clone is **setup only**. Residual
tools need **built binaries** under `testdata/xdna2_cmd/external_known_good/`
(or `pass_unblock_once --ingest PATH`). Build smallest public example
(passthrough / vector_scalar_add / single_core matmul / getting_started; npu2).
Collect `*.xclbin`, `insts_*.bin`, aiebu ELF, `.ctrltext`. Ingest + PROVENANCE
(repo+commit, build cmd, sizes+sha, invent_pass=0). **Source tree ≠ capture
material.**

**FastFlowLM / Lemonade intercept:** capture instr BO, CONFIG_CU buffer,
post-patch ctrltext, or partial dumps from a job that already loads on this
firmware class. Size/header-only dumps still progress. Still ends at the same
capture directory.

**Acceptance:** non-empty allowed file under `external_known_good/` + PROVENANCE;
soft classify / ingest does not invent Pass.

### B — Next authorized live only (new unlock + named reason)

1. Prefer **minimal load target** (identity/no-op/smaller partition) over full
   llm-2x4x4 research PDI alone.
2. Wire or research **debug BO** assign/remove (`DRM_AMDXDNA_HWCTX_ASSIGN_DBG_BUF`
   / `REMOVE_DBG_BUF`) on that shot; sync after fail for richer residual
   (still invent_pass=0).
3. Optional: systematic rejection surface (vary size/header/align/CU map) under
   **one** unlock — catalog FW codes; evidence only.
4. Restore L3 / safety gates exactly as written.

Unlock mint rules unchanged: host-minted `C_UNLOCK`, named offline fingerprint
delta, one-shot CONSUMED after use. Unlock ≠ Pass.

### C — Named offline code deltas (agent-legal now)

These prepare for the next material without inventing bytes:

| Id | Work | Notes |
|----|------|--------|
| C1 | Stronger aiebu / ELF / rela / ctrltext header validators | Refuse packing that cannot match public IRON txn/opcode or tile map shapes (soft honesty tokens) |
| C2 | Fingerprint/dump records BO sizes, alignments, rela targets, CU config layout | Structural compare for future capture |
| C3 | Residual classifier splits | Bad PDI geometry vs bad CU map vs protocol mismatch vs wait_cmd_einval (never invent Pass) |
| C4 | Optional IRON artifact ingest path | When good binary appears, classify immediately |
| C5 | Offline PDI geometry walkers vs spent flexml 2x4x4 | Compare partition geometry to known spent FPs |
| C6 | Debug-BO uAPI prep (pack/constants only if not already) | Research-only until unlock; no live without unlock |

**Acceptance for any C* slice:** pure TDD or script tests green; invent_pass=0;
not_c_ready=1; not_ran_on_npu=1; no unlock mint; no empty reconfirm-only stamp.

### D — Supporting (low priority)

- Record host firmware (`npu.sbin`) version + protocol major/minor vs research PDI era.
- Riallto lower-barrier examples as alternate material source.
- xdna-driver docs as SSoT for ioctl/status codes.

---

## Explicit non-goals (still)

- Re-run offline gates with no new files.
- Mint unlocks without named fingerprint + residual reason.
- Soft-compare against empty `external_known_good/`.
- Claim progress from packaging firmware already rejected.
- Empty implement → review loops on the same residual table.
- Invent product Pass / `ran_on_npu`.

---

## Recommended sequence

1. **Material:** IRON/mlir-aie npu2 minimal example **or** FastFlowLM/Lemonade
   intercept → capture dir + PROVENANCE.
2. **Live (after new unlock):** debug BO + minimal load image (or new material).
3. **Parallel agent:** C1–C6 offline tooling so next capture is immediately actionable.
4. **Re-interpret** residual only after new bytes or richer last_apply.

---

## Suggested next implement (when operator starts agent work)

**Status (2026-08-10):** **P0 C1–C3 landed** offline (validators + structural
fingerprint + residual splits). Report: `.agents/reports/impl-p0-c1-c3.md`.
Residual honesty pin: `RESIDUAL.md` top + `.agents/reports/impl-residual-pin-p0.md`.
Still **no** Pass / invent_pass=0 / **no** `ran_on_npu` / **no** unlock mint.

Prefer the **next named offline** slice (P1), not re-status of C1–C3:

```text
implement --effort 3 Pass-unblock offline P1 (creative avenues C4+C5):
1) Optional IRON artifact ingest path (classify when good binary appears)
2) Offline PDI geometry walkers vs spent flexml 2x4x4
No unlock mint. No Pass claim. No empty capture invent.
```

(Historical P0 implement text kept for audit — **done**, do not re-run as next:)

```text
implement --effort 3 Pass-unblock offline prep (creative avenues C1–C3):
1) Stronger aiebu/ELF/rela/ctrltext validators (pure TDD; soft honesty)
2) Structural fingerprint fields: BO sizes/align/rela targets/CU layout
3) Residual classifier split tokens (geometry vs CU map vs protocol; invent_pass=0)
No unlock mint. No Pass claim. No empty capture invent.
```

(Leading slash omitted here so auto-run does not spin until you intend it.)

---

## Done when (this plan)

- Creative avenues are residual-pinned (this file + `RESIDUAL.md`).
- Operator chooses either material path (A) or offline C* implement.
- Still **Pass=No** until live decide criteria met.
