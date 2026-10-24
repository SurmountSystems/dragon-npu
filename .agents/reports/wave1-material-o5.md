# Wave W1 — material + O5 aiebu offline closeout

**Date:** 2026-08-11
**Scope:** Offline only. Material avenue + aiebu O5 + fingerprint ledger drift.
**Honesty:** invent_pass=0 · product_pass=0 · ran_on_npu=0 · Pass=No · no
`C_UNLOCK` minted · no L3 · no `XDNA2_ALLOW_EXEC_CMD` · no residual_matched claim
from public AOT alone.

Reports:

- O5 detail: `.agents/reports/wave1-o5-aiebu.md`
- This join: `.agents/reports/wave1-material-o5.md`
- Fingerprint tool: `tools/pass_unblock_fingerprint_once.sh` (ledger fix)
- Bag PROVENANCE: `testdata/xdna2_cmd/external_known_good/PROVENANCE.md` (Spent labels)

---

## 1. Capture dir status (no new bins this slice)

Path: `testdata/xdna2_cmd/external_known_good/`

| Set | Ctrltext (B) | sha16 | Tag |
|-----|-------------:|-------|-----|
| vector_scalar_add (+ primary `working_stack_ctrltext.bin`) | 300 | `aaeb5b68cc298a45` | `public-npu2-small` RESEARCH ONLY |
| single_core matmul 64³ | 660 | `de8671245bb72436` | `public-npu2-small` RESEARCH ONLY |
| whole_array matmul 4×4 128³ | 2592 | `7ee4b267ce31cd70` | public IRON AOT larger; still research only |

Siblings present for each: `insts.bin`, `insts.elf`, `xclbin`, `pdi`, core ELF.
**New bins this wave:** **none** (no new size class landed).

Working-stack **live EXEC peel** (ERT Completed + useful OFM on this firmware)
still **missing**. Public AOT ≠ residual identity.

---

## 2. Material — one larger path attempted / documented

### Already built (not re-run as progress)

| Workdir under `mlir-aie-capture/` | Landed? |
|----------------------------------|---------|
| `aot_vector_scalar_add/` | yes |
| `aot_single_core_matmul_npu2/` | yes |
| `aot_whole_array_matmul_npu2/` | yes |

### Candidate larger / different: cascade matmul

| Item | Value |
|------|--------|
| Source | `/home/hunter/Projects/ai/mlir-aie/programming_examples/basic/matrix_multiplication/cascade/` |
| Toolchain | same venv: mlir-aie 1.4.0 + Peano under `mlir-aie-capture/venv` |
| Status | **Not built** on this host (wave0 also noted unbuilt) |
| Why not landed this slice | No new AOT artifact produced in this implementer run; default cascade dims are large (512³ / 4 cols) and prior whole_array already met “larger than 660” |

**Exact offline AOT command (ready when operator/agent shell runs it):**

```bash
export VENV=/home/hunter/Projects/ai/mlir-aie-capture/venv
export OUT=/home/hunter/Projects/ai/mlir-aie-capture/aot_cascade_matmul_npu2
export PEANO_INSTALL_DIR="$VENV/lib/python3.14/site-packages/llvm-aie"
export PYTHONPATH="$VENV/lib/python3.14/site-packages/mlir_aie/python${PYTHONPATH:+:$PYTHONPATH}"
export PATH="/home/hunter/Projects/ai/mlir-aie-capture/tools:$VENV/lib/python3.14/site-packages/mlir_aie/bin:$VENV/bin:$PATH"
mkdir -p "$OUT"
cd /home/hunter/Projects/ai/mlir-aie/programming_examples/basic/matrix_multiplication/cascade
python cascade.py --dev npu2 \
  -M 128 -K 128 -N 128 -m 32 -k 32 -n 32 \
  --n-aie-cols 4 \
  --dtype_in i16 --dtype_out i32 \
  --xclbin-path "$OUT/cascade_matmul.xclbin" \
  --insts-path "$OUT/cascade_matmul.insts.bin"
# then (if insts.bin present):
# aiebu-asm -t aie2txn -c "$OUT/cascade_matmul.insts.bin" -o "$OUT/cascade_matmul.insts.elf"
# llvm-objcopy -O binary --only-section=.ctrltext "$OUT/cascade_matmul.insts.elf" "$OUT/cascade_matmul.ctrltext.bin"
# land under external_known_good only if real new bytes (new sha/size)
```

**Blocked / caveats:**

- Full `ryzen-ai-install/venv/bin` on PATH breaks `import aie.iron` (use aiebu symlink only).
- Cascade default 512³ may be heavy; modest 128³ flags above are intentional.
- Even on success: RESEARCH ONLY; invent_pass=0; not residual identity; not unlock alone.
- FastFlowLM / Lemonade: **not installed** (wave0); no EP peel from that avenue.

### Other unbuilt optional examples (not claimed this slice)

- `matrix_vector/` under same matmul tree
- Further whole_array dim sweeps (would likely stay same program family)

---

## 3. O5 aiebu (summary)

Full writeup: `.agents/reports/wave1-o5-aiebu.md`.

| Finding | Detail |
|---------|--------|
| Lab wrap | `silicon_lab/lab_instr.elf` 32264 B; ctrltext 1688; **rela×12**; ctrl 29664 |
| Public wraps | vector rela **1**; single_core **5**; whole_array **20** |
| Normalization | peel may ≠ raw insts sha (single_core / whole_array) |
| Cannot | invent 1688 identity; invent vendor post-patch; close residual by re-wrap |
| Userspace | aiebu+rela green offline; residual is FW/EXEC/WAIT |

---

## 4. Fingerprint drift fix (O9 follow-up)

**Bug:** `tools/pass_unblock_fingerprint_once.sh` still labeled flexml
`llm-2x4x4` / `unified-4x4` as **Not spent / unspent_candidate** while
`RESIDUAL.md` SSoT marks both **Spent** (unlocks `8352882b` / `efd44f35`).

**Fix (landed):**

- Ledger constants: all three flexml STX + CVML baseline are Spent sha16s
- `is_spent_sha16` includes `ba0cd738…` and `7d172bf2…`
- `is_spent_flexml_tag` + skip re-dump for all three unless `--force`
- Markdown ledger rows: **Spent** under correct unlocks
- CONSUMED list in report honesty includes `1f761bf4`, `efd44f35`, `8352882b`
- PROVENANCE bag tables: “unspent” → “spent” for llm / u4 rows

**Expected after re-run offline:**

- llm / u4: `spent=1`, `RESULT=SKIP_SPENT_REDUMP` (unless `--force`)
- no preferred **unspent flexml** DIFF from those three
- invent_pass=0; spent DIFF alone ≠ unlock reason

Re-run when shell available:

```bash
bash tools/pass_unblock_once.sh --status
bash tools/pass_unblock_fingerprint_once.sh --offline
```

---

## 5. Soft status (intended)

| Field | Value |
|-------|--------|
| Pass | No |
| invent_pass | 0 |
| ran_on_npu | false / 0 |
| Unlock | none active (first `C_UNLOCK=` in RESIDUAL is CONSUMED) |
| Flexml research PDIs | **all three Spent** |
| Capture public AOT | present; residual-mismatched |
| Working-stack EXEC peel | still missing |

---

## 6. Frozen / next

**Do not:** re-live spent flexml; invent Pass; invent residual_matched; force
re-dump spent as progress; empty reconfirm mill.

**Next real material:** cascade (or other) AOT **only if** new size/sha lands; or
true working-stack EXEC peel; next live only under **new** host-minted unlock +
named offline reason.
)
