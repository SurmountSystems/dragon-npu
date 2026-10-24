# Plan explore: Route A — better public matmul material (mlir-aie)

**Date:** 2026-08-10
**Mode:** read-only explore (no product code edits)
**Honesty:** invent_pass=0 · Pass=No · not_c_ready=1 · not_ran_on_npu=1 ·
product_known_good=0 · **no unlock** · capture material ≠ Pass

**Scope:** Route A = build/ingest **matmul-class** public npu2 artifacts from
mlir-aie, beyond the landed `vector_scalar_add` research bins.

---

## 1. Capture dir today (vector_scalar_add material)

**Path:** `/home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/external_known_good/`

| Basename | size (B) | sha16 | Role |
|----------|----------|-------|------|
| `working_stack_ctrltext.bin` | 300 | `aaeb5b68cc298a45` | Primary soft-compare ctrltext (alias of peel) |
| `mlir_aie_vector_scalar_add_npu2.ctrltext.bin` | 300 | `aaeb5b68cc298a45` | Same bytes as insts/ctrltext peel |
| `mlir_aie_vector_scalar_add_npu2.insts.bin` | 300 | `aaeb5b68cc298a45` | Instruction stream |
| `mlir_aie_vector_scalar_add_npu2.insts.elf` | 992 | `790d8b3e6d25bda8` | aiebu ELF (`.ctrltext` 0x12c, 1× `.rela.dyn`) |
| `mlir_aie_vector_scalar_add_npu2.xclbin` | 8983 | `849d0ba7276ada71` | Host AOT xclbin |
| `mlir_aie_vector_scalar_add_npu2.pdi` | 2528 | `fe438059f5d0e4eb` | Built PDI (not live-proven CONFIG_CU) |
| `mlir_aie_vector_scalar_add_npu2_core.elf` | 2468 | `ffd58932f0f38bca` | Peano core ELF (`.text` only) |
| `PROVENANCE.md` | text | — | Research-ready provenance |

**PROVENANCE highlights** (`testdata/xdna2_cmd/external_known_good/PROVENANCE.md`):

- Source: public Xilinx/mlir-aie IRON `@iron.jit` AOT, **npu2**, not XRT live EXEC on this host
- Example: `programming_examples/basic/vector_scalar_add/`
- Commit (shallow): `a8fea363dfd63015cffd380e0679950419a0a7cd`
- Workdir: `/home/hunter/Projects/ai/mlir-aie-capture/aot_vector_scalar_add/`
- Shape: problem_size=1024 int32, aie_tile_width=32
- Soft compare: **300 B vs lab classic 1688 B** = **mismatch expected** (not matmul / not mladf identity)
- invent_pass=0; material ≠ product known-good ≠ C-ready ≠ Pass

**Prior playbook report:**
`/home/hunter/Projects/ai/dragon-npu/.agents/reports/impl-mlir-aie-material-playbook.md`
(residual pin: `residual-pin-mlir-aie-material-landed.md`)

---

## 2. mlir-aie tree and matmul examples

**Clone present:** `/home/hunter/Projects/ai/mlir-aie` (source only until bins are copied into dragon-npu capture dir)

### Matmul-related paths (concrete)

| Path | Kind | Entry / build |
|------|------|----------------|
| `/home/hunter/Projects/ai/mlir-aie/programming_examples/basic/matrix_multiplication/` | Tree root | README + common makefile |
| `…/basic/matrix_multiplication/single_core/` | **Recommended first** | `single_core.py`, `Makefile`, `test.cpp` |
| `…/basic/matrix_multiplication/matrix_vector/` | Smaller matmul-adjacent | `matrix_vector.py` (M×K @ K-vector) |
| `…/basic/matrix_multiplication/whole_array/` | Larger multi-core | `whole_array.py` |
| `…/basic/matrix_multiplication/cascade/` | Cascade variant | `cascade.py` |
| `/home/hunter/Projects/ai/mlir-aie/programming_examples/getting_started/03_matrix_multiplication_single_core/` | Getting-started AOT demo | `matrix_multiplication_single_core.py` |
| `…/ml/block_datatypes/matrix_multiplication/single_core/` | ML/bfp matmul family | Makefile + `single_core.py` |
| `…/ml/block_datatypes/matrix_multiplication/single_core_no_tiling/` | Smaller tiling path | Makefile |
| `…/aie_kernels/aie2/mm.cc`, `…/aie2p/mm.cc` | Kernel sources | Used via `kernels.mm` |

### npu2 / device flags

**vector_scalar_add (proven AOT path on this host):**

```bash
cd /home/hunter/Projects/ai/mlir-aie/programming_examples/basic/vector_scalar_add
python vector_scalar_add.py --dev npu2 \
  --xclbin-path $OUT/….xclbin \
  --insts-path $OUT/….insts.bin \
  --pdi-path $OUT/….pdi \
  --elf-path $OUT/….insts.elf
```

**single_core matmul (Makefile → `--dev ${devicename}`):**

```bash
# From programming_examples/basic/matrix_multiplication/single_core/
# makefile-common sets: aieargs+=--dev ${devicename} …
# lit default for Strix: devicename=npu2
make -f Makefile devicename=npu2
# or smaller problem if supported by kwargs:
# env M=64 K=64 N=64 m=32 k=32 n=32 make devicename=npu2
```

Direct Python (from README; uses `add_compile_args` → `--dev` / compile-only paths):

```bash
python3 single_core.py --help
# Compile-only pattern (same family as vector_scalar_add AOT):
# python3 single_core.py --dev npu2 --xclbin-path … --insts-path …  [and pdi/elf if exposed]
python3 single_core.py -M 64 -K 64 -N 64 -m 32 -k 32 -n 32 --dtype_in i16 --dtype_out i32
```

Lit evidence (npu2):
`…/single_core/tests/run_strix_makefile_default.lit` → `make … devicename=npu2`
`run_strix_makefile_small_m.lit` and `run_makefile_small_m.lit` exist for smaller-M configs.

**Default single_core dims:** M=K=N=512, tile m=k=n=32, i16→i32 (heavier than vector add).
Prefer **small M/K/N** for first AOT emit (faster aiecc; still matmul geometry).

### Getting-started matmul

`/home/hunter/Projects/ai/mlir-aie/programming_examples/getting_started/03_matrix_multiplication_single_core/`
- AOT via `CompilableDesign` for 256³ and 512³
- README: `python3 matrix_multiplication_single_core.py`
- Good on-ramp docs; **basic/single_core** is closer to the Makefile/AOT artifact layout already used for vector_scalar_add

---

## 3. Playbook: `docs/CAPTURE_MATERIAL_MLIR_AIE.md` (key steps)

Full path: `/home/hunter/Projects/ai/dragon-npu/docs/CAPTURE_MATERIAL_MLIR_AIE.md`

| Step | Action |
|------|--------|
| 1 | Clone mlir-aie recursive (source **≠** capture material) |
| 2 | Env for **npu2**; Peano/llvm-aie + aiebu; follow upstream install |
| 3 | Build **smallest** example first: prefer `passthrough_kernel` → `vector_scalar_add` → **`single_core` matmul** → getting_started |
| 4 | Collect xclbin, insts, aiebu ELF, ctrltext peel, optional PDI; size + sha |
| 5 | Ingest into capture dir (see §4) |
| 6 | Fill PROVENANCE (source, commit, build line, target, honesty) |
| 7 | Soft compare / `pass_unblock_once --agent-offline` (still invent_pass=0) |
| 8 | Residual pin only after real files (no unlock from material alone) |

Policy: empty reconfirm frozen; invent_pass=0 always; clone alone is not progress.

---

## 4. `pass_unblock_once.sh --ingest` (multi-file / naming)

**Script:** `/home/hunter/Projects/ai/dragon-npu/tools/pass_unblock_once.sh`

```bash
bash tools/pass_unblock_once.sh --ingest PATH [NAME]
bash tools/pass_unblock_once.sh --ingest-pdi PATH   # research PDI → mktemp only, not bag basename
```

| Rule | Detail |
|------|--------|
| PATH required | Bare `--ingest` → exit 2 refuse |
| NAME optional | Single path component only (`safe_install_name`: no `/`, `\`, `..`) |
| Multi-file | **One file per --ingest call**; use a **distinct NAME** each time |
| Default name | Without NAME, install uses capture script default (often source basename) |
| Refused basenames | `instr_post_patch.bin`, `instr_vendor_post_patch.bin`, `ctrl_post_patch.bin`, `instr_raw_txn.bin`, `lab_instr.bin`/`elf`, `validate.xclbin`, … |
| Bag count | Only `*.bin` / `*.ctrltext` under capture dir (maxdepth 1), non-refused |
| PDI | `--ingest-pdi` copies to exclusive mktemp under `$TMPDIR`; does **not** install as capture known-good product |
| Honesty lines | C4 iron ingest + C5 spent-flexml geometry notes always invent_pass=0 |

**Example multi-file matmul land:**

```bash
REPO=/home/hunter/Projects/ai/dragon-npu
OUT=…/aot_single_core_matmul_npu2   # outside dragon-npu

# Peel ctrltext from aiebu ELF when present
llvm-objcopy -O binary --only-section=.ctrltext $OUT/insts.elf $OUT/ctrltext.bin

bash $REPO/tools/pass_unblock_once.sh --ingest $OUT/ctrltext.bin \
  mlir_aie_single_core_matmul_npu2.ctrltext.bin
bash $REPO/tools/pass_unblock_once.sh --ingest $OUT/insts.bin \
  mlir_aie_single_core_matmul_npu2.insts.bin
bash $REPO/tools/pass_unblock_once.sh --ingest $OUT/insts.elf \
  mlir_aie_single_core_matmul_npu2.insts.elf
bash $REPO/tools/pass_unblock_once.sh --ingest $OUT/final.xclbin \
  mlir_aie_single_core_matmul_npu2.xclbin
# Optional research PDI fingerprint only:
bash $REPO/tools/pass_unblock_once.sh --ingest-pdi $OUT/something.pdi

# Do NOT overwrite working_stack_ctrltext.bin unless intentional primary compare swap
```

Then edit PROVENANCE capture table + optional soft
`bash tools/operator_external_capture.sh --compare`.

---

## 5. Unspent flexml fingerprints (ba0cd738, 7d172bf2)

Documented in:

| Location | Content |
|----------|---------|
| `/home/hunter/Projects/ai/dragon-npu/RESIDUAL.md` | Spent/unspent PDI ledger (top status + § ledger) |
| `AGENTS.md` Latest Turn residual pins | Ranked next: unspent llm / unified-4x4 |
| `.agents/reports/pass-unblock-fingerprint-once.md` | Full FP table + source xclbin paths + sha256 |
| `.agents/reports/live-shot-2ae69036.md` / `impl-u4-flexml-live-closeout.md` | Spent unified-2x4x4 live |

| tag | pdi_bytes | sha16 | Live status |
|-----|-----------|-------|-------------|
| flexml STX `unified-2x4x4` | **27200** | `0eeeb70c84df056e` | **Spent** under `2ae69036` (`wait_cmd_einval`) |
| flexml STX `llm-2x4x4` | **21664** | **`ba0cd738fd870011`** | **Not spent** (needs **new** unlock for live) |
| flexml STX `unified-4x4` | **13792** | **`7d172bf223a9b06b`** | **Not spent** (weaker topology A/B) |
| CVML AIE2P 4x4 Overlay | **283840** | `a1e65d04add3422c` | Spent prior (orphan IO_PAGE_FAULT class) |

**Source xclbins** (from fingerprint report):

- llm-2x4x4:
  `/home/hunter/Projects/ai/ryzen-ai-install/venv/lib/python3.12/site-packages/flexml/flexml_extras/data/ryzen-ai/stx/llm-2x4x4.xclbin`
  sha256 starts `ba0cd738fd870011…`
- unified-4x4:
  `…/flexml/.../stx/unified-4x4.xclbin`
  sha256 starts `7d172bf223a9b06b…`

These are **research CONFIG_CU overlays**, not mlir-aie matmul ctrltext and **not** product known-good. Unspent ≠ unlock minted.

---

## 6. Size / kind comparison (flexml dumps vs vector_scalar_add vs lab)

| Kind | Approx size | What it is | Relation to residual |
|------|-------------|------------|----------------------|
| mlir-aie vector_scalar_add **ctrltext/insts** | **300 B** | Tiny IRON control stream | Soft-compare only; ≠ lab classic |
| mlir-aie vector_scalar_add **insts.elf** | 992 B | aiebu wrap + 1 rela | Structure research (C1-class) |
| mlir-aie vector_scalar_add **pdi** | **2528 B** | Minimal public AOT PDI | Far smaller than flexml PDIs |
| mlir-aie vector_scalar_add **xclbin** | 8983 B | Small example package | Research only |
| Lab classic **mladf ctrltext** | **1688 B** | DD `mladfmatmulbias` classic v0.1 txn | Soft-compare oracle; not external match |
| Lab aiebu **lab_instr.elf** (typical) | ~32 KiB | ELF + rela×12 | Userspace green; FW residual separate |
| flexml **unified-2x4x4** PDI | **27200 B** | Spent live attach | CONFIG_CU green → `wait_cmd_einval` |
| flexml **llm-2x4x4** PDI | **21664 B** | Unspent candidate | Next live only with **new** unlock |
| flexml **unified-4x4** PDI | **13792 B** | Unspent, weaker A/B | Same unlock rule |
| CVML **AIE2P 4x4** PDI | **283840 B** | Spent prior | Orphan IO_PAGE_FAULT class |

**Gap Route A must address (honest):** vector_scalar_add is **not** matmul-matched and will not close soft identity vs 1688. Matmul AOT from mlir-aie may produce **larger** insts/ctrltext/PDI and **closer residual class for research**, but still:

- Offline AOT ≠ Pass
- Soft match ≠ residual closed
- Material alone ≠ `C_UNLOCK`
- invent_pass=0

---

## 7. Recommended smallest matmul to build first

### Primary recommendation

**`programming_examples/basic/matrix_multiplication/single_core/`**

| Why | Detail |
|-----|--------|
| Closest to residual intent | Real `C = A @ B` microkernel (`kernels.mm` / `aie_kernels/aie2{,p}/mm.cc`) |
| Same IRON/Makefile family | Proven AOT emit pattern already used for vector_scalar_add |
| npu2 supported | lit `devicename=npu2`; makefile injects `--dev ${devicename}` |
| Tunable small | Start with reduced `-M/-K/-N` (and/or `make` M/K/N) before 512³ |
| Artifacts | Expect `build/final_*.xclbin`, `build/insts_*.bin`; peel `.ctrltext` if ELF present |

**Suggested first command shape (host with existing mlir-aie-capture venv):**

```bash
# Reuse env pattern from impl-mlir-aie-material-playbook.md
cd /home/hunter/Projects/ai/mlir-aie/programming_examples/basic/matrix_multiplication/single_core
# Prefer compile-only AOT (no live EXEC required for material):
# make / python with --dev npu2 --xclbin-path --insts-path [--pdi-path] [--elf-path]
# Start small, e.g. M=K=N=64 or 128 with m=k=n=32 if design allows
```

### Secondary (if single_core aiecc is heavy)

| Option | Path | Notes |
|--------|------|-------|
| matrix_vector | `…/matrix_multiplication/matrix_vector/` | Matmul-adjacent, default M=K=288; smaller story than full GEMM array |
| single_core_no_tiling (ml/bfp) | `…/ml/block_datatypes/matrix_multiplication/single_core_no_tiling/` | Possibly simpler tiling; different dtype story |
| getting_started 03 | `…/getting_started/03_matrix_multiplication_single_core/` | Doc-friendly AOT for 256/512; larger default |

### Do not treat as Pass material

- whole_array / multi-col sweeps (heavy)
- mobilenet / magika / full ML pipelines
- Overwriting `working_stack_ctrltext.bin` with matmul without PROVENANCE row (keep vector_scalar_add basenames; add `mlir_aie_single_core_matmul_npu2.*`)

---

## 8. Route A ranked next (agent-legal; no unlock)

1. **Offline:** AOT-compile `single_core` matmul for **npu2** with small M/K/N; collect xclbin/insts/(elf)/ctrltext peel/PDI if emitted.
2. **Ingest** under distinct basenames (`mlir_aie_single_core_matmul_npu2.*`); update PROVENANCE honesty block.
3. **Soft** compare / C4–C5 classify / `--agent-offline` fingerprint notes — always invent_pass=0.
4. **Do not** mint unlock from new bins alone. Live still needs **new** host unlock + named reason (unspent flexml and/or justified material + residual class from `2ae69036`).
5. **Separate track:** unspent flexml llm-2x4x4 (`ba0cd738…`) / unified-4x4 (`7d172bf2…`) for CONFIG_CU A/B under a **new** unlock only.

---

## 9. Key absolute paths (index)

| Role | Path |
|------|------|
| Capture dir | `/home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/external_known_good/` |
| PROVENANCE | `…/external_known_good/PROVENANCE.md` |
| Playbook | `/home/hunter/Projects/ai/dragon-npu/docs/CAPTURE_MATERIAL_MLIR_AIE.md` |
| Ingest tool | `/home/hunter/Projects/ai/dragon-npu/tools/pass_unblock_once.sh` |
| Residual SSoT | `/home/hunter/Projects/ai/dragon-npu/RESIDUAL.md` |
| mlir-aie clone | `/home/hunter/Projects/ai/mlir-aie` |
| Prior AOT workdir | `/home/hunter/Projects/ai/mlir-aie-capture/aot_vector_scalar_add/` |
| Recommended example | `/home/hunter/Projects/ai/mlir-aie/programming_examples/basic/matrix_multiplication/single_core/` |
| Prior material report | `/home/hunter/Projects/ai/dragon-npu/.agents/reports/impl-mlir-aie-material-playbook.md` |
| Flexml FP report | `/home/hunter/Projects/ai/dragon-npu/.agents/reports/pass-unblock-fingerprint-once.md` |

---

## Footer

```
invent_pass=0
product_pass=0
ran_on_npu=0
Pass=No
C_UNLOCK_minted=0
unlock_active=none
route=A_public_matmul_material
recommended_first=basic/matrix_multiplication/single_core --dev npu2
present_material=vector_scalar_add_300B (research only)
```
