# Wave W3 S7 / M3a — cascade matmul AOT attempt

**Date:** 2026-08-11
**Scope:** Offline only. No live NPU. No unlock mint. No ALLOW=1.
**Honesty:** invent_pass=0 · residual_matched_claim=0 · product_pass=0 · ran_on_npu=0 · Pass=No
**Result:** **landed = yes** (new ctrltext size + sha16)

Related: `docs/CAPTURE_MATERIAL_MLIR_AIE.md`,
`.agents/reports/wave3-material-hunt.md` § M3a,
`testdata/xdna2_cmd/external_known_good/PROVENANCE.md` (fourth set).

---

## 1. Goal

Build public mlir-aie **cascade** matmul AOT offline (modest dims), peel
ctrltext / aiebu ELF / xclbin / `.prj/main.pdi`, and ingest only if **new**
size or sha16 vs already-landed public sets (300 / 660 / 2592).

---

## 2. Host path (pre-existing)

| Item | Path / note |
|------|-------------|
| Source | `/home/hunter/Projects/ai/mlir-aie/programming_examples/basic/matrix_multiplication/cascade/` (`cascade.py`) |
| mlir-aie HEAD | `a8fea363dfd63015cffd380e0679950419a0a7cd` |
| Capture venv | `/home/hunter/Projects/ai/mlir-aie-capture/venv` (mlir-aie **1.4.0**) |
| aiebu tools | `mlir-aie-capture/tools/{aiebu-asm,aiebu-dump}` → ryzen-ai-install (PATH clean; **not** full ryzen venv first) |
| Prior AOT dirs | vector / single_core / whole_array present; **cascade dir was absent** before this attempt |
| Workdir created | `/home/hunter/Projects/ai/mlir-aie-capture/aot_cascade_matmul_npu2/` |

---

## 3. Build (success)

Env (same as prior successful AOTs):

```bash
export VENV=/home/hunter/Projects/ai/mlir-aie-capture/venv
export OUT=/home/hunter/Projects/ai/mlir-aie-capture/aot_cascade_matmul_npu2
export PEANO_INSTALL_DIR="$VENV/lib/python3.14/site-packages/llvm-aie"
export PYTHONPATH="$VENV/lib/python3.14/site-packages/mlir_aie/python${PYTHONPATH:+:$PYTHONPATH}"
export PATH="/home/hunter/Projects/ai/mlir-aie-capture/tools:$VENV/lib/python3.14/site-packages/mlir_aie/bin:$VENV/bin:$PATH"
```

Command (modest dims; default cascade is 512³):

```bash
python cascade.py --dev npu2 \
  -M 128 -K 128 -N 128 -m 32 -k 32 -n 32 \
  --n-aie-cols 4 \
  --dtype_in i16 --dtype_out i32 \
  --xclbin-path "$OUT/cascade_matmul.xclbin" \
  --insts-path "$OUT/cascade_matmul.insts.bin"
```

- **Exit:** 0
- **Log:** `/tmp/cascade_aot_build.log` (host ephemeral)
- Immediate outputs: `cascade_matmul.insts.bin` **4512 B**, `cascade_matmul.xclbin` **46153 B**, `.prj/` with 16 core ELFs + `main.pdi`

Post-process:

```bash
aiebu-asm -t aie2txn -c cascade_matmul.insts.bin -o cascade_matmul.insts.elf
# Header 0.1; gen4; 8×6; TransactionSize 4512; NumOps 116
llvm-objcopy -O binary --only-section=.ctrltext \
  cascade_matmul.insts.elf cascade_matmul.ctrltext.bin
cp cascade_matmul.prj/main.pdi cascade_matmul.pdi
cp cascade_matmul.prj/elfs_main_core_0_2/elfs_main_core_0_2.elf cascade_matmul_core.elf
```

Notes:

- `dump_xclbin_pdi.sh` on this xclbin: **no** standalone `PDI` section (expected for mlir-aie AOT; use `.prj/main.pdi`).
- AIE_PARTITION dump as RAW fails (JSON-only) — not used as capture bytes.

---

## 4. Novelty check (vs landed peels)

| Landed ctrltext | size | sha16 |
|-----------------|------|-------|
| vector / working_stack | 300 | `aaeb5b68cc298a45` |
| single_core | 660 | `de8671245bb72436` |
| whole_array | 2592 | `7ee4b267ce31cd70` |
| **cascade (this turn)** | **4512** | **`18cb653c05a9d4c8`** |

**NEW** size and sha16 → ingest allowed.

---

## 5. Ingest (capture dir)

Via `bash tools/pass_unblock_once.sh --ingest …` (research-only; invent_pass=0):

| Basename under `testdata/xdna2_cmd/external_known_good/` | size (B) | sha16 |
|----------------------------------------------------------|----------|-------|
| `mlir_aie_cascade_matmul_npu2.ctrltext.bin` | 4512 | `18cb653c05a9d4c8` |
| `mlir_aie_cascade_matmul_npu2.insts.bin` | 4512 | `e1e4db951d44092d` |
| `mlir_aie_cascade_matmul_npu2.insts.elf` | 5632 | `486bc26f5f5de1be` |
| `mlir_aie_cascade_matmul_npu2.xclbin` | 46153 | `478214ec08d481f7` |
| `mlir_aie_cascade_matmul_npu2.pdi` | 39696 | `e2a2b177dc3f4a5e` |
| `mlir_aie_cascade_matmul_npu2_core.elf` | 3308 | `0f56a114b434bbf7` |

Full sha256 in PROVENANCE fourth-set table.

Primary bag `working_stack_ctrltext.bin` **unchanged** (still 300 B vector peel).

---

## 6. Soft compare / honesty

- `bash tools/operator_external_capture.sh --compare` after ingest:
  `sha_match=0` · `structural_match=0` · `residual_not_closed=1` ·
  `invent_pass=0` · `not_c_ready=1` · `not_ran_on_npu=1` · `product_known_good=0`
  (mismatch vs lab classic expected)
- Cascade PDI **39696** ≠ spent flexml 27200 / 21664 / 13792 (geometry rematch alone still ≠ unlock).
- Public AOT host-compile **≠** load-accepted residual-matched material (no ERT Completed + OFM allclose on this firmware).
- **No** Pass · **no** unlock mint · **no** live apply.

---

## 7. Docs touched

| File | Change |
|------|--------|
| `testdata/xdna2_cmd/external_known_good/PROVENANCE.md` | Fourth set cascade table + honesty |
| `docs/CAPTURE_MATERIAL_MLIR_AIE.md` | cascade row marked landed with sizes/sha16 |

---

## 8. Verdict

| Question | Answer |
|----------|--------|
| Build succeeded? | **Yes** |
| New size/sha vs prior public peels? | **Yes** (4512 / `18cb653c05a9d4c8`) |
| Landed under capture dir? | **Yes** (six files) |
| PROVENANCE updated? | **Yes** |
| Pass / residual match / unlock? | **No / no / no** (invent_pass=0) |

**Blocked residual close:** more public AOT does not by itself close post-attach
EXEC residual or invent residual identity to lab classic / flexml. Next material
value remains load-accepted / residual-matched peels or operator working-stack
capture, not empty reconfirm of this set.

**landed: yes**
**report:** `/home/hunter/Projects/ai/dragon-npu/.agents/reports/wave3-cascade-aot-attempt.md`
