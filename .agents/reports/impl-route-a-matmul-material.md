# Report: Route A — mlir-aie single_core matmul material (second set)

**Date:** 2026-08-10
**Status:** A0–A4 complete. Second public npu2 AOT set landed under
`testdata/xdna2_cmd/external_known_good/` with PROVENANCE + ingest.
**Honesty:** invent_pass=0 · product_pass=0 · ran_on_npu=0 · Pass=No · no
`C_UNLOCK` minted · no L3 / no `XDNA2_ALLOW_EXEC_CMD` · not product known-good.
**Did not** delete or overwrite vector_scalar_add material or
`working_stack_ctrltext.bin`. **Did not** edit `silicon_contract.rs`.

Related: `docs/CAPTURE_MATERIAL_MLIR_AIE.md`,
`.agents/reports/impl-mlir-aie-material-playbook.md`,
`.agents/reports/plan-explore-route-a-matmul-material.md`.

---

## A0 env probe

| Item | Result |
|------|--------|
| Capture venv | `/home/hunter/Projects/ai/mlir-aie-capture/venv` Python **3.14.6** |
| mlir-aie wheel | **1.4.0** under site-packages |
| Peano / llvm-aie | `…/site-packages/llvm-aie` (PEANO_INSTALL_DIR); wheel `21.0.0.2026062301+cb664e8c` |
| aiebu-asm | Symlink only: `mlir-aie-capture/tools/aiebu-asm` → ryzen-ai-install (not full foreign bin on PATH) |
| mlir-aie examples tree | `/home/hunter/Projects/ai/mlir-aie` HEAD `a8fea363dfd63015cffd380e0679950419a0a7cd` |
| Prior vector material | Intact (ctrltext 300 B sha16 `aaeb5b68cc298a45`) |

Env pattern (same as vector playbook):

```text
PATH=tools + mlir_aie/bin + llvm-aie/bin + venv/bin
PEANO_INSTALL_DIR=…/llvm-aie
PYTHONPATH=…/mlir_aie/python
```

---

## A1 build

**Example:** `programming_examples/basic/matrix_multiplication/single_core/`
**Target:** npu2
**Dims:** M=K=N=**64**, m=k=n=**32**, dtype_in=**i16**, dtype_out=**i32**
**Mode:** AOT compile-only (no NPU EXEC)

```bash
cd /home/hunter/Projects/ai/mlir-aie/programming_examples/basic/matrix_multiplication/single_core
python single_core.py --dev npu2 \
  -M 64 -K 64 -N 64 -m 32 -k 32 -n 32 \
  --dtype_in i16 --dtype_out i32 \
  --xclbin-path $OUT/single_core_matmul.xclbin \
  --insts-path $OUT/single_core_matmul.insts.bin
```

`OUT=/home/hunter/Projects/ai/mlir-aie-capture/aot_single_core_matmul_npu2`

**Result:** exit 0. Emitted xclbin + insts.bin + `.prj/` (includes `main.pdi`,
Peano core ELF). CLI has **no** `--elf-path` / `--pdi-path` (unlike
vector_scalar_add); aiebu ELF assembled after build:

```bash
aiebu-asm -t aie2txn -c single_core_matmul.insts.bin -o single_core_matmul.insts.elf
# aiebu: Header 0.1, gen4, cols/rows 1×6, TransactionSize 660, NumOps 17
llvm-objcopy -O binary --only-section=.ctrltext \
  single_core_matmul.insts.elf single_core_matmul.ctrltext.bin
cp .prj/main.pdi single_core_matmul.pdi
cp .prj/elfs_main_core_0_2/elfs_main_core_0_2.elf single_core_matmul_core.elf
```

**ctrltext vs insts:** same length **660 B**, **not** byte-identical (aiebu
normalizes one field; peel is preferred soft-compare body from aiebu ELF).

**Fallback:** not needed (primary single_core succeeded).

---

## A2 land (second set)

Directory: `testdata/xdna2_cmd/external_known_good/`

| Basename | size (B) | sha16 | sha256 |
|----------|----------|-------|--------|
| `mlir_aie_single_core_matmul_npu2.ctrltext.bin` | 660 | `de8671245bb72436` | `de8671245bb72436ecf54949952b25096d59096afef8ecf6317f04695f60454f` |
| `mlir_aie_single_core_matmul_npu2.insts.bin` | 660 | `34baaff69260033c` | `34baaff69260033c0092dce3a5d90fd936206e0dba309a810e02b21b7ee0a3de` |
| `mlir_aie_single_core_matmul_npu2.insts.elf` | 1408 | `6fbbf33de3543be7` | `6fbbf33de3543be76d8bbbdda46d0f05b3b73a6e546f7ad8588ce51d8be4e28e` |
| `mlir_aie_single_core_matmul_npu2.xclbin` | 11993 | `8a96c4cb5e01656c` | `8a96c4cb5e01656c74b21d727e660472d05904f2d98dac1677da99355ffbee52` |
| `mlir_aie_single_core_matmul_npu2.pdi` | 5536 | `cd3d9fa6577d1e37` | `cd3d9fa6577d1e37c1a57b359b731e457becb1219031d845c250919b52d500c2` |
| `mlir_aie_single_core_matmul_npu2_core.elf` | 4632 | `3c17f06f9b690348` | `3c17f06f9b690348d70df647a690d604d62b819eb4599b5115445e036c7e0861` |

**Preserved (not overwritten):**

| Basename | size | note |
|----------|------|------|
| `working_stack_ctrltext.bin` | 300 | Still vector primary |
| `mlir_aie_vector_scalar_add_npu2.*` | (prior set) | Untouched |

**ELF notes:** `.ctrltext` 0x294; `.rela.dyn` **5** entries; core ELF Peano
`.text` only.

**PROVENANCE:** append section
`### mlir-aie npu2 material — single_core matmul (second set; 2026-08-10)`
with example path, dims, commit, sizes, sha16/sha256, honesty invent_pass=0.

**Playbook:** brief note in `docs/CAPTURE_MATERIAL_MLIR_AIE.md` § prefer table
that single_core matmul is preferred next after vector and is now landed.

---

## A3 ingest

| Command | Outcome |
|---------|---------|
| `--ingest` ×6 distinct `mlir_aie_single_core_matmul_npu2.*` names | each **INSTALLED** exit 0; invent_pass=0 |
| `--ingest-pdi` on matmul pdi | exit 0; pdi_bytes=**5536** sha16=`cd3d9fa6577d1e37`; research_pdi_not_product_known_good=1 |
| `--status` | capture_bins=**5** (script counts `*.bin`/`*.ctrltext` only); blocked_on_external_bytes=0; Pass=No; unlock **none** |

---

## Size / class vs residual oracles

| Stream | size (B) | Class |
|--------|----------|--------|
| Matmul ctrltext (this set) | **660** | IRON single_core 64³ i16→i32 AOT |
| Vector ctrltext (primary bag) | **300** | Prior minimal load |
| Lab classic mladf | **1688** | Staged residual oracle (rela×12) |
| Matmul PDI | **5536** | Built offline; not CONFIG_CU proven |
| flexml spent unified-2x4x4 | **27200** | sha16 `0eeeb70c…` |
| flexml unspent llm-2x4x4 | **21664** | sha16 `ba0cd738…` |
| flexml unspent unified-4x4 | **13792** | sha16 `7d172bf2…` |

Soft compare vs lab classic **1688 B** still **mismatch expected** (matmul IRON
ctrltext ≠ mladf classic txn profile). Material **≠** Pass **≠** product
known-good **≠** C-ready **≠** `ran_on_npu`.

---

## What this is / is not

| Is | Is not |
|----|--------|
| Second public-stack npu2 matmul-class AOT set in the capture dir | Product Pass |
| Offline classifier / geometry research material | `ran_on_npu` |
| Proof single_core AOT works on this host env (small 64³) | Lab classic 1688 identity |
| PROVENANCE research-ready | New live unlock |
| Vector set + `working_stack_ctrltext.bin` still primary soft ctrltext | Replacement of spent flexml PDI path |

**Unlock:** none active. Do not mint from this report. Spent unlocks unchanged
(`2ae69036` + priors CONSUMED).

---

## Optional next (agent-legal, offline)

1. Soft C4/C5/C6 on matmul PDI/xclbin with existing pure helpers (still invent_pass=0).
2. Soft compare with explicit external path pointing at matmul ctrltext (research).
3. Larger or different dtype single_core / matrix_vector only if a named offline
   reason needs a different geometry.
4. Live still only under a **new** host-minted unlock + named offline reason
   (not from this material landing alone).

**Live / L3 / ALLOW:** not run. Not requested. No unlock.

---

## Footer

```
invent_pass=0
product_pass=0
ran_on_npu=0
Pass=No
C_UNLOCK_minted=0
route_a_matmul=landed_second_set
ctrltext_bytes=660
pdi_bytes=5536
vector_primary_preserved=1
working_stack_still_vector_300=1
silicon_contract_rs_untouched=1
```
