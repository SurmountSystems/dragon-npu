# Report: mlir-aie capture-material playbook (concrete bins)

**Date:** 2026-08-10
**Status:** **Acceptance A** — real binaries under
`testdata/xdna2_cmd/external_known_good/` + PROVENANCE + ingest attempted.
**Honesty:** invent_pass=0 · product_pass=0 · ran_on_npu=0 · Pass=No · no
`C_UNLOCK` minted · no L3 / no `XDNA2_ALLOW_EXEC_CMD` · not product known-good.

Playbook doc: `docs/CAPTURE_MATERIAL_MLIR_AIE.md`.

---

## 1. Probe

| Item | Result |
|------|--------|
| Capture dir | Existed; **bins=0** before this run (only `PROVENANCE.md`) |
| mlir-aie on disk | **None** under `~/Projects/ai` / `/tmp` before clone |
| PATH tools | No `aiecc` / `aiebu-dump` on system PATH initially |
| `aiebu-asm` later found | `/home/hunter/Projects/ai/ryzen-ai-install/venv/bin/aiebu-asm` (host Ryzen AI venv) |
| Disk | Root ~1.3 T free; `/tmp` 12 G free; enough |
| Python | 3.14.6 system + venv |
| Sibling trees | `ryzen-ai-install`, `RyzenAI-SW` (xclbins present but **not** used as mlir-aie material; residual already has spent flexml fingerprints) |
| Docs | `docs/CAPTURE_MATERIAL_MLIR_AIE.md` present and followed |

---

## 2. Environment setup (outside dragon-npu)

| Path | Role |
|------|------|
| `/home/hunter/Projects/ai/mlir-aie-capture/venv` | Python 3.14 venv |
| mlir-aie wheel **1.4.0** | `mlir_aie-1.4.0-cp314-cp314-manylinux_2_35_x86_64.whl` (GitHub release) |
| llvm-aie / Peano | `llvm_aie-21.0.0.2026062301+cb664e8c` py3-none manylinux wheel |
| aiebu-asm | Symlinked into `mlir-aie-capture/tools/` from ryzen-ai-install |
| `/home/hunter/Projects/ai/mlir-aie` | Shallow clone for examples only (source **≠** capture) |

```text
clone HEAD: a8fea363dfd63015cffd380e0679950419a0a7cd
PEANO_INSTALL_DIR=…/site-packages/llvm-aie
PYTHONPATH=…/site-packages/mlir_aie/python
PATH=…/mlir-aie-capture/tools + …/mlir_aie/bin + venv/bin
```

**First failure (resolved):** full AOT with `--elf-path` needed `aiebu-asm`.
Without it, aiecc failed at stage “insts.elf”. Compile without ELF still
emitted xclbin + insts + pdi. With aiebu-asm on PATH, ELF succeeded.

**Contamination note:** Putting entire `ryzen-ai-install/venv/bin` first on
PATH broke `import aie.iron` (MLIR shared lib clash). Fix: only symlink
`aiebu-asm`, not the full foreign bin.

---

## 3. Build (succeeded)

**Example:** `programming_examples/basic/vector_scalar_add/`
**Target:** `npu2`
**Mode:** AOT compile-only (no NPU EXEC required for artifact emit)

```bash
cd /home/hunter/Projects/ai/mlir-aie/programming_examples/basic/vector_scalar_add
python vector_scalar_add.py --dev npu2 \
  --xclbin-path $OUT/vector_scalar_add.xclbin \
  --insts-path $OUT/vector_scalar_add.insts.bin \
  --pdi-path $OUT/vector_scalar_add.pdi \
  --elf-path $OUT/vector_scalar_add.insts.elf
```

`OUT=/home/hunter/Projects/ai/mlir-aie-capture/aot_vector_scalar_add`

**Peel:**

```bash
llvm-objcopy -O binary --only-section=.ctrltext \
  vector_scalar_add.insts.elf vector_scalar_add.ctrltext.bin
```

`ctrltext.bin` is **byte-identical** to `insts.bin` (300 B).
`insts.elf` sections: `.ctrltext` 0x12c, `.rela.dyn` (1 rela), `.dynsym`,
`.note.xrt.UID`.

Also present under `.prj/`: Peano core ELF (`.text` only), CDO bins, MLIR
intermediates (not all copied into the bag).

**Not built:** full matmul / passthrough live EXEC on this firmware (not
required for offline material; residual still open regardless).

---

## 4. Files landed

Directory: `testdata/xdna2_cmd/external_known_good/`

| Basename | size (B) | sha16 | sha256 |
|----------|----------|-------|--------|
| `working_stack_ctrltext.bin` | 300 | `aaeb5b68cc298a45` | `aaeb5b68cc298a450f01fb88d83426dcd702a01dfc2e79684b5f631db5885fd6` |
| `mlir_aie_vector_scalar_add_npu2.ctrltext.bin` | 300 | `aaeb5b68cc298a45` | (same) |
| `mlir_aie_vector_scalar_add_npu2.insts.bin` | 300 | `aaeb5b68cc298a45` | (same) |
| `mlir_aie_vector_scalar_add_npu2.insts.elf` | 992 | `790d8b3e6d25bda8` | `790d8b3e6d25bda8065605ac345d555d7bdc34642e36230fc81a1ea74d8db057` |
| `mlir_aie_vector_scalar_add_npu2.xclbin` | 8983 | `849d0ba7276ada71` | `849d0ba7276ada71ff2bc475bfdb0f8cd618335a3b6d500ddccf628ff315ca22` |
| `mlir_aie_vector_scalar_add_npu2.pdi` | 2528 | `fe438059f5d0e4eb` | `fe438059f5d0e4eb649f498d4d9cf71a8576fece357bbdf407f927f43fd8534e` |
| `mlir_aie_vector_scalar_add_npu2_core.elf` | 2468 | `ffd58932f0f38bca` | `ffd58932f0f38bcaf964956d13ea1b0b381ed30faa36f5d27482d696474fec79` |

`PROVENANCE.md` updated with source repo, commit, build command, target npu2,
file table, and honesty block:

> Generated offline from public mlir-aie example; not yet proven on this host's
> firmware. For classifier / geometry ingest only. invent_pass=0.

---

## 5. Ingest / residual tooling

| Command | Outcome |
|---------|---------|
| `--ingest` same-path (already copied) | exit **2** refuse clobber (expected) |
| `--ingest` temp path → `working_stack_ctrltext.bin` | exit **0** INSTALLED; bytes=300; invent_pass=0 |
| `--ingest-pdi` on mlir pdi | exit **0**; pdi_bytes=2528 sha16=`fe438059f5d0e4eb`; research_pdi_not_product_known_good=1 |
| `--status` | **capture_bins=3**, `blocked_on_external_bytes=0`, `HUMAN_NEEDED=0`, Pass=No, unlock **none** |
| `operator_external_capture.sh --compare` | Soft compare **ran**; external_len=**300** vs lab_len=**1688**; sha_match=0 structural_match=0; bag_drop_complete=1; residual still `app_load_pdi_fail_after_aiebu`; invent_pass=0 not_c_ready=1 not_ran_on_npu=1 |

**Honest compare read:** external material is **vector_scalar_add** control text
(300 B, not classic mladf txn profile). Lab export is staged **mladf classic**
1688 B. Mismatch is **expected** and does **not** invent Pass. Bag is no longer
empty for residual tools.

---

## 6. What this is / is not

| Is | Is not |
|----|--------|
| Real public-stack-generated npu2 binaries in the capture dir | Product Pass |
| Offline AOT material for classifier / geometry / bag-drop | `ran_on_npu` |
| Proof wheels + Peano + aiebu-asm can emit on this host | Matmul-matched residual closer alone |
| PROVENANCE research-ready | New live unlock |
| Soft compare unblocked (`blocked_on_external_bytes=0`) | Live EXEC on this firmware |

**Unlock:** none active. Do not mint from this report. Spent flexml fingerprints
unchanged.

---

## 7. Optional next (agent-legal, offline)

1. AOT-compile a **small matmul** mlir-aie example for npu2 (closer geometry to
   residual) and land under distinct basenames + PROVENANCE row.
2. Soft C4/C5/C6 on the new PDI/xclbin with existing pure helpers (still
   invent_pass=0).
3. Human: only if a **working** matmul EXEC stack can capture post-patch
   ctrltext that matches the lab key; this playbook material is intentionally
   minimal load, not a Pass claim.

**Live / L3 / ALLOW:** not run. Not requested. No unlock.

---

## 8. Footer

```
invent_pass=0
product_pass=0
ran_on_npu=0
Pass=No
C_UNLOCK_minted=0
capture_bins>=3
blocked_on_external_bytes=0
acceptance=A
```
