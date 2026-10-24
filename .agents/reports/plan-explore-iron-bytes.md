# IRON / mlir-aie instruction bytes already on this host

**Date:** 2026-08-13  
**Honesty:** invent_pass=0. IRON `PASS!` ≠ product `ran_on_npu`. Do **not** invent a 1688 file. Do **not** retag 300/660/2592/4512 as classic 1688 or load-accepted.

No L3 spawn (no spawn tool in this window). Walked capture dir, PROVENANCE, wave-6 distance, catalog, mlir-aie examples, and `/home/hunter/Projects/ai/mlir-aie-capture/`.

---

## 1. What already exists

### Durable peels (use these)

`testdata/xdna2_cmd/external_known_good/` plus identical workdirs under `/home/hunter/Projects/ai/mlir-aie-capture/`. All four public npu2 AOT sets completed IRON verify on FW **1.1.2.64**. All **ClassicTxn v0.1 / gen4**. None is lab classic 1688.

| Job | ctrltext B | sha16 | insts.bin | grid / NumOps | rela×N (aiebu) | PDI / xclbin / insts.elf |
|-----|-----------:|-------|-----------|---------------|----------------|--------------------------|
| **vector_scalar_add** (primary bag + `working_stack_ctrltext.bin`) | **300** | `aaeb5b68cc298a45` | **300**, same sha (byte-identical to ctrltext) | **1×6 / 8** | 2 (wave6 walk; PROVENANCE said 1) | 2528 / 8983 / 992 |
| single_core matmul 64³ i16→i32 | **660** | `de8671245bb72436` | 660, **different** sha `34baaff69260033c` | 1×6 / 17 | 5 | 5536 / 11993 / 1408 |
| whole_array 128³ 4-col | **2592** | `7ee4b267ce31cd70` | 2592, sha `ae692499…` | 8×6 / 68 | 20 | 66864 / 73327 / 3520 |
| cascade 128³ 4-col | **4512** | `18cb653c05a9d4c8` | 4512, sha `e1e4db95…` | 8×6 / 116 | 36 | 39696 / 46153 / 5632 |

Lab oracle (do **not** confuse): `testdata/xdna2_cmd/lab_instr.bin` **1688** sha16 `3c5ffca750922d3b`, NumOps **40**, rela×**12**. Distance map: `.agents/reports/wave6-distance-lab-vs-iron.md`.

### Sibling trees (source only)

`/home/hunter/Projects/ai/mlir-aie/programming_examples/` has **no committed** `insts.bin` / xclbin / pdi. `vector_scalar_add` lit mentions `test_stx/` but that dir is not in the tree.

Catalog wrap (`src/backends/iron_kernel.rs`) already ran on this host (`child_said_pass=true`, `ran_on_npu=false`):

| Catalog | Example cwd | Default I/O | Insts on disk? |
|---------|-------------|-------------|----------------|
| `vector_add` | `basic/vector_scalar_add` | 1024 × i32 in + out (4 KiB each). Golden = in+1 | **yes** (300 B peel) |
| `eltwise` / `eltwise_mul` | `ml/eltwise` | 65536 × bf16 × 2 in + out | **no** (JIT only) |
| `relu` / `gelu` / `silu` | `ml/eltwise_unary` | 65536 × bf16 in + out | **no** |
| `softmax` | `ml/softmax` | 262144 × bf16, tile 1024 | **no** |
| `swiglu` | `ml/swiglu` | 16384 × bf16 + 2× weights | **no** |
| `scale_shift` | `ml/scale_shift` | 65536 × bf16 × 3 in + out | **no** |

Wave-2 examples can emit insts via sibling `--xclbin-path` + `--insts-path` (compile-only). Only `vector_add` has `--aot-dir` in the product catalog.

### This-run AOT

Product creates `$TMPDIR/dragon_npu_iron_aot_vector_add_{pid}_{stamp}` only on `--iron-export`. Default wrap does **not**. No leftover `dragon_npu_iron_aot_*` seen under `/tmp` listing. Durable copy is `mlir-aie-capture/aot_vector_scalar_add/` + completed_run `…/20260811_150247/`. No this-run `*.npy` golden in those dirs.

---

## 2. vector_add npu2 (first honest bind)

- **Length:** 300 B. **Format:** ClassicTxnV01 (START_NPU txn, v0.1, gen4, 1 col × 6 rows, NumOps 8, txn_size=300).
- **I/O:** `problem_size=1024` int32. IFM 4096 B, OFM 4096 B. Host golden: `arange(1..1024)+1`. C++ `test.cpp` same 1024/1024.
- **Siblings (bag names):**  
  `mlir_aie_vector_scalar_add_npu2.{ctrltext,insts}.bin` 300  
  `.insts.elf` 992 · `.pdi` 2528 · `.xclbin` 8983 · `_core.elf` 2468  
  Paths: `testdata/xdna2_cmd/external_known_good/` and `mlir-aie-capture/aot_vector_scalar_add/`.
- **Kind:** product job `iron_npu2_vector_add` matches **exactly 300 B** only. 660 is not that job. Never 1688.

---

## 3. How IRON submits vs product EXEC

IRON (working on this host) is **XRT**, not raw START_NPU:

1. Load **xclbin** → `register_xclbin` + `hw_context` (PDI/CDO inside the xclbin).
2. Instruction stream as either:
   - **raw `insts.bin`** into an instruction BO; kernel args `opcode=3`, instr BO, ninstr, then data BOs (`setup_and_run_aie`), or
   - **`xrt::elf` + `xrt::ext::kernel`** (current `vector_scalar_add/test.cpp`: `kernel(opcode, 0, 0, bo_in, bo_out)`).
3. Python JIT: `NPUKernel(xclbin, insts)` → `CachedXRTRuntime.load_and_run`. Optional full-ELF path skips the pair.

Product lab EXEC expects **our** START_NPU Cmd BO + instruction blob (prefer aiebu ELF + rela patch) + I/O BOs via `EXEC_CMD`/`WAIT`. It does **not** load an xclbin. CONFIG_CU PDI is a separate overlay. So IRON’s 300 B stream is the right *instruction* identity for `iron_npu2_vector_add`; the **context load** (xclbin vs CONFIG_CU vs skip-CU) is still a product bind gap.

---

## 4. Short enough without 1688

Use the **300 B** vector stream. Eight ops, two I/O buffers, 4 KiB each, golden is `x+1`. 660 is next-smallest (matmul, 3 tensors, i16/i32). 2592/4512 are longer and closer to lab *grid* only (still wrong NumOps/sha). Wave-2 JIT jobs are useful as wrap, not as bytes until someone AOT-exports them.

---

## 5. Honesty + what is still missing for product EXEC

- IRON PASS on 300/660/2592/4512 is **not** `decide_ran_on_npu`.
- `working_stack_ctrltext.bin` **is** the 300 B peel, **not** 1688.
- Kind-match `iron_npu2_vector_add` ≠ residual match to `mladf_classic_1688`.

Still missing to bind 300 B through product EXEC (not wrap):

1. START_NPU / kernel-arg layout for **2 BOs** (IFM+OFM), not mladf IFM/WTS/OFM+ctrl.
2. Same-shot paddr / rela on the 300 B aiebu ELF (rela×2, ELF 992 B), not lab rela×12.
3. Context: load the **2528 B PDI** (or skip overlay and accept that IRON used xclbin). Product has no xclbin load path.
4. Golden 1024×i32 (generate; no npy in the capture dir).
5. Live `decide_ran_on_npu` under a **new** unlock + named reason. Do not mint from this inventory.

**Recommended first example:** `vector_scalar_add` npu2, 300 B, sha16 `aaeb5b68cc298a45`. Use the files already in `testdata/xdna2_cmd/external_known_good/`.
