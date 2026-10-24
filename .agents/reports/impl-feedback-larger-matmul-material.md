# Report: third public mlir-aie npu2 matmul set (whole_array, larger)

**Date:** 2026-08-11
**Status:** Third public npu2 AOT set landed under
`testdata/xdna2_cmd/external_known_good/` with PROVENANCE + offline ingest.
**Honesty:** invent_pass=0 · product_pass=0 · ran_on_npu=0 · Pass=No · no
`C_UNLOCK` minted · no L3 / no `XDNA2_ALLOW_EXEC_CMD` · not product known-good ·
not flexml-matched · not lab classic 1688 identity · **not unlock reason alone**.

Related: `docs/CAPTURE_MATERIAL_MLIR_AIE.md`,
`.agents/reports/impl-route-a-matmul-material.md`,
`testdata/xdna2_cmd/external_known_good/PROVENANCE.md`.

---

## Goal

Optional offline: land a **third** public mlir-aie npu2 matmul capture set
**larger** than single_core **660 B** (prefer whole-array / cascade style,
modest M/K/N). RESEARCH ONLY.

---

## Env (reused from prior playbook)

| Item | Result |
|------|--------|
| Capture venv | `/home/hunter/Projects/ai/mlir-aie-capture/venv` |
| mlir-aie wheel | **1.4.0** under site-packages |
| Peano / llvm-aie | `…/site-packages/llvm-aie`; `PEANO_INSTALL_DIR` |
| aiebu-asm | Symlink only: `mlir-aie-capture/tools/aiebu-asm` → ryzen-ai-install (no permanent PATH change) |
| mlir-aie tree | `/home/hunter/Projects/ai/mlir-aie` HEAD `a8fea363dfd63015cffd380e0679950419a0a7cd` |
| Prior sets | vector 300 B + single_core 660 B **preserved** |

PATH pattern:

```text
PATH=tools + mlir_aie/bin + llvm-aie/bin + venv/bin
PEANO_INSTALL_DIR=…/llvm-aie
PYTHONPATH=…/mlir_aie/python
```

---

## Example chosen

**`programming_examples/basic/matrix_multiplication/whole_array/`**

- Public multi-core design (4 rows × N_cols cols; default/chosen **4** cols → **16** cores).
- Larger instruction stream than single_core by construction.
- Cascade (`cascade/`) left unbuilt; whole_array met the “larger than 660 B” goal on first AOT try.

### Build (AOT only, no NPU EXEC)

```bash
cd /home/hunter/Projects/ai/mlir-aie/programming_examples/basic/matrix_multiplication/whole_array
python whole_array.py --dev npu2 \
  -M 128 -K 128 -N 128 -m 16 -k 16 -n 16 \
  --n-aie-cols 4 \
  --dtype_in i16 --dtype_out i32 \
  --xclbin-path $OUT/whole_array_matmul.xclbin \
  --insts-path $OUT/whole_array_matmul.insts.bin
```

`OUT=/home/hunter/Projects/ai/mlir-aie-capture/aot_whole_array_matmul_npu2`

**Result:** exit 0. Emitted xclbin + insts.bin + `.prj/` (`main.pdi`, 16 Peano
core ELFs under `elfs_main_core_*`).

Post-build peel (same pattern as single_core):

```bash
aiebu-asm -t aie2txn -c whole_array_matmul.insts.bin -o whole_array_matmul.insts.elf
# aiebu: Header 0.1, gen4, cols/rows 8×6, TransactionSize 2592, NumOps 68
llvm-objcopy -O binary --only-section=.ctrltext \
  whole_array_matmul.insts.elf whole_array_matmul.ctrltext.bin
cp .prj/main.pdi whole_array_matmul.pdi
cp .prj/elfs_main_core_0_2/elfs_main_core_0_2.elf whole_array_matmul_core.elf
```

**ctrltext vs insts:** both **2592 B**, **not** byte-identical (aiebu normalizes).

---

## Landed files (third set)

Directory: `/home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/external_known_good/`

| Basename | size (B) | sha16 | sha256 |
|----------|----------|-------|--------|
| `mlir_aie_whole_array_matmul_npu2.ctrltext.bin` | **2592** | `7ee4b267ce31cd70` | `7ee4b267ce31cd70bfd880ab882a339e456a2a9ecabc4c1b38622fb159f2d1af` |
| `mlir_aie_whole_array_matmul_npu2.insts.bin` | 2592 | `ae692499a79c74b4` | `ae692499a79c74b46175cb4841f235edb0059175144941522f792105b758dec3` |
| `mlir_aie_whole_array_matmul_npu2.insts.elf` | 3520 | `0984b40244c33f68` | `0984b40244c33f681bfb9fecc664fde51e8b69f7a7d3d70042a81d23d90bb1a9` |
| `mlir_aie_whole_array_matmul_npu2.xclbin` | 73327 | `44ab78cf9eeb315e` | `44ab78cf9eeb315ee8ef67dba011b7095916df4218783668dfcc2e9ea66639b0` |
| `mlir_aie_whole_array_matmul_npu2.pdi` | 66864 | `e721b80335e6d8ed` | `e721b80335e6d8ed162380284ee2cf1287b0fde3059f2610feb01802e3b8a6c3` |
| `mlir_aie_whole_array_matmul_npu2_core.elf` | 4456 | `2d47c4d6017cc6a6` | `2d47c4d6017cc6a643914cfef59495c03f31b25084c75ceede698e8463cbfc10` |

**Preserved (not overwritten):**

| Basename | size | note |
|----------|------|------|
| `working_stack_ctrltext.bin` | 300 | Still vector primary |
| `mlir_aie_vector_scalar_add_npu2.*` | (first set) | Untouched |
| `mlir_aie_single_core_matmul_npu2.*` | 660 ctrltext (second set) | Untouched |

**ELF notes:** `.ctrltext` **0xa20** (2592 B); `.rela.dyn` **20** entries;
representative core is Peano tile (0,2) only (16 cores exist in `.prj/`).

**PROVENANCE:** append section
`### mlir-aie npu2 material — whole_array matmul (third set; 2026-08-11)`.

**Playbook:** `docs/CAPTURE_MATERIAL_MLIR_AIE.md` prefer table notes whole_array
landed third set.

---

## Offline ingest

| Command | Outcome |
|---------|---------|
| `--ingest` ×6 distinct `mlir_aie_whole_array_matmul_npu2.*` names | exit 0; invent_pass=0 |
| `--ingest-pdi` on whole_array pdi | pdi_bytes=**66864** sha16=`e721b80335e6d8ed`; research_pdi_not_product_known_good=1 |
| `--status` | capture_bins=**7** (script counts `*.bin`/`*.ctrltext`); blocked_on_external_bytes=0; Pass=No; unlock **none** |

---

## Size / class vs residual oracles

| Stream | size (B) | Class |
|--------|----------|--------|
| Whole_array ctrltext (this set) | **2592** | IRON whole_array 4×4 128³ i16→i32 AOT |
| Single_core matmul ctrltext | **660** | Prior second set |
| Vector ctrltext (primary bag) | **300** | Prior minimal load |
| Lab classic mladf | **1688** | Staged residual oracle (rela×12) |
| Whole_array PDI | **66864** | Built offline; not CONFIG_CU proven |
| flexml spent unified-2x4x4 | **27200** | sha16 `0eeeb70c…` |
| flexml unspent llm-2x4x4 | **21664** | sha16 `ba0cd738…` |
| flexml unspent unified-4x4 | **13792** | sha16 `7d172bf2…` |

Soft compare vs lab classic **1688 B** still **mismatch expected** (2592 ≠ 1688;
IRON whole_array ≠ mladf classic txn). Material **≠** Pass **≠** product
known-good **≠** C-ready **≠** `ran_on_npu` **≠** unlock alone.

---

## What this is / is not

| Is | Is not |
|----|--------|
| Third public-stack npu2 matmul AOT set (larger multi-core) | Product Pass |
| Offline classifier / geometry research material | `ran_on_npu` |
| Proof whole_array AOT works on this host env (modest 128³, 4 cols) | Lab classic 1688 identity |
| PROVENANCE research-ready | New live unlock |
| Vector + single_core sets still intact | flexml-matched residual fix |
| Larger than single_core 660 B (2592 B ctrltext) | Reason to mint `C_UNLOCK` |

**Unlock:** none active. Do not mint from this report. Spent unlocks unchanged
(`2ae69036` + priors CONSUMED).

---

## Optional next (agent-legal, offline)

1. Soft C4/C5/C6 / structure tools on whole_array PDI/xclbin (still invent_pass=0).
2. Soft compare with external path pointing at whole_array ctrltext (research).
3. Cascade AOT only if a named offline reason needs cascade geometry (not required).
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
third_set=whole_array_matmul_npu2
ctrltext_bytes=2592
pdi_bytes=66864
n_aie_cols=4
cores_in_prj=16
dims=128x128x128_i16_i32
vector_primary_preserved=1
single_core_preserved=1
working_stack_still_vector_300=1
not_unlock_alone=1
```
