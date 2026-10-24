# Wave W3 M-hunt — residual-matched / working-stack material avenues

**Date:** 2026-08-11
**Scope:** Offline host-path audit only. No live NPU apply. No unlock mint. No ALLOW=1.
**Honesty:** invent_pass=0 · residual_matched_claim=0 · product_pass=0 · ran_on_npu=0 · Pass=No

**Track map (plan + this hunt):**

| Id | Meaning this report |
|----|---------------------|
| **M1** | Working-stack capture playbook + what is already under the capture dir |
| **M3 / M3a** | Larger / different mlir-aie AOT (cascade, matrix_vector, etc.) |
| **M4** | FastFlowLM / Lemonade / Ryzen AI EP intercept |
| **M8** (hunt label) | XRT sample / xrt-smi / host path that could complete a job |
| **M9** (hunt label) | Side-channel dump feasibility (strace/gdb/DD registry) without invasive debug |

Plan SSoT Track M in `.agents/plans/plan-silicon-try-everything.md` lists **M1–M7** only. **M8/M9** here are host-avenue labels for checklist items 4–5, not new Pass claims.

Related: `docs/CAPTURE_MATERIAL_MLIR_AIE.md`,
`testdata/xdna2_cmd/external_known_good/PROVENANCE.md`,
`.agents/reports/wave0-material-avenues.md`,
`.agents/reports/wave1-material-o5.md`.

---

## 1. Capture dir inventory (M1) — sizes / sha only

Path: `/home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/external_known_good/`

**Status:** Not empty. Three public mlir-aie npu2 AOT sets + primary bag ctrltext. All **RESEARCH ONLY**. None is residual identity. None is load-accepted (no ERT Completed + OFM allclose on this firmware documented for these peels).

### Ctrltext streams (primary research surface)

| Set | Basename | size (B) | sha16 | Size-class / tag |
|-----|----------|----------|-------|------------------|
| Primary bag | `working_stack_ctrltext.bin` | 300 | `aaeb5b68cc298a45` | **`public-npu2-small`** RESEARCH ONLY (same body as vector) |
| vector_scalar_add | `mlir_aie_vector_scalar_add_npu2.ctrltext.bin` | 300 | `aaeb5b68cc298a45` | **`public-npu2-small`** |
| single_core matmul 64³ | `mlir_aie_single_core_matmul_npu2.ctrltext.bin` | 660 | `de8671245bb72436` | **`public-npu2-small`** |
| whole_array 4×4 128³ | `mlir_aie_whole_array_matmul_npu2.ctrltext.bin` | 2592 | `7ee4b267ce31cd70` | length → **other** (public IRON AOT research) |

### Sibling research files (not bag-bin count for soft “bins”)

Each of the three sets also has: `insts.bin`, `insts.elf`, `xclbin`, `pdi`, `*_core.elf`.
PROVENANCE tables (abridged):

| Stream | notable sibling sizes |
|--------|------------------------|
| vector | xclbin 8983; pdi 2528; insts.elf 992 |
| single_core | xclbin 11993; pdi 5536; insts.elf 1408 |
| whole_array | xclbin 73327; pdi 66864; insts.elf 3520 |

### Soft vs residual oracles (mismatch expected)

| Oracle | size | vs public peels |
|--------|------|-----------------|
| Lab classic `mladf-classic-1688` | 1688 | **mismatch** (300 / 660 / 2592) |
| Spent flexml STX PDIs | 27200 / 21664 / 13792 | **≠** public AOT PDI sizes/sha; all three **Spent** under CONSUMED unlocks |

**Working-stack live EXEC capture** (post-patch ctrltext from a job that completed **ERT Completed + useful OFM on this firmware**): **missing**. Public AOT is host-generated compile-only per PROVENANCE.

**Do not re-ingest same sha** for progress. Capture folder empty is **not** the residual.

---

## 2. mlir-aie / mlir-aie-capture (M3 / M3a)

### Host trees (present)

| Path | Role | Notes |
|------|------|--------|
| `/home/hunter/Projects/ai/mlir-aie` | Source clone | HEAD per PROVENANCE `a8fea363…`; examples under `programming_examples/` |
| `/home/hunter/Projects/ai/mlir-aie-capture/venv` | Build venv | mlir-aie wheel **1.4.0** + Peano/llvm-aie (prior successful AOT) |
| `/home/hunter/Projects/ai/mlir-aie-capture/tools/aiebu-asm` (+ `aiebu-dump`) | Symlinks | → ryzen-ai-install tools (not full foreign `venv/bin` on PATH) |
| `…/aot_vector_scalar_add/` | Built | Landed in capture dir |
| `…/aot_single_core_matmul_npu2/` | Built | Landed |
| `…/aot_whole_array_matmul_npu2/` | Built | Landed (+ 16 core ELFs in `.prj/`) |
| `…/aot_cascade_matmul_npu2/` | **Absent** | Not built this host |

### Cascade matmul source (M3a)

| Item | Value |
|------|--------|
| Source | `/home/hunter/Projects/ai/mlir-aie/programming_examples/basic/matrix_multiplication/cascade/` |
| Files | `cascade.py`, `Makefile`, `README.md`, `test.cpp` |
| Default dims | M=K=N=512, 4 cols (heavy); Makefile also has m/k/n tile knobs |
| Status | **Source only. No AOT artifacts. Not ingested.** |
| Residual value | Optional topology research only. **Not** residual identity. **Not** unlock alone. |

**Build env usability (honest):** Prior three AOTs prove the **same** toolchain path worked for AOT emit without NPU EXEC. Env is **usable for offline AOT** if PATH is kept clean:

- Use: `mlir-aie-capture/tools` + venv `mlir_aie` bin + venv python
- **Do not** put entire `ryzen-ai-install/venv/bin` first (breaks `import aie.iron` via MLIR shared-lib clash; documented in prior playbook)

### Concrete next build commands (if blocked on cascade only)

Modest dims (preferred over default 512³):

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
# if insts.bin present:
# aiebu-asm -t aie2txn -c "$OUT/cascade_matmul.insts.bin" -o "$OUT/cascade_matmul.insts.elf"
# llvm-objcopy -O binary --only-section=.ctrltext \
#   "$OUT/cascade_matmul.insts.elf" "$OUT/cascade_matmul.ctrltext.bin"
# ingest only if NEW size/sha (not duplicate of 300/660/2592):
# bash /home/hunter/Projects/ai/dragon-npu/tools/pass_unblock_once.sh \
#   --ingest "$OUT/cascade_matmul.ctrltext.bin" mlir_aie_cascade_matmul_npu2.ctrltext.bin
# fill PROVENANCE; invent_pass=0; still RESEARCH ONLY
```

Optional same-tree: `matrix_vector/` (also **not built**).

**Blocked for residual match:** more public AOT without live load-accept still leaves residual class post-attach / EXEC residual open.

---

## 3. FastFlowLM / Lemonade (M4)

| Project | Expected path under `~/Projects/ai` | Result |
|---------|-------------------------------------|--------|
| FastFlowLM | `…/FastFlowLM` (ROCm/FastFlowLM) | **Absent** |
| Lemonade | `…/lemonade` / lemonade-sdk | **Absent** |
| Capture bins from either | under `external_known_good/` | **None** |

**Blocked** until operator clones/installs and runs a **loadable** path on this firmware, then captures real post-patch ctrltext / instr BO / CONFIG_CU PDI and `--ingest`s.

Upstream pointers (not re-hosted):
https://github.com/ROCm/FastFlowLM · https://github.com/lemonade-sdk/lemonade

Source checkout alone is **not** capture material (same rule as mlir-aie).

---

## 4. XRT / xrt-smi / Ryzen AI path for a completed job (M8)

### Present on host (tooling, not residual-matched capture)

| Item | Path / note |
|------|-------------|
| Ryzen AI full venv | `/home/hunter/Projects/ai/ryzen-ai-install/venv` |
| eepy-min | `…/venv-eepy-min` + `activate-eepy.sh` |
| XRT shim (local) | `/home/hunter/Projects/ai/ryzen-ai-install/xrt-shim/` (`libxrt_core*.so`, `libxrt_driver_xdna.so`) |
| aiebu-asm / xclbinutil / flexml | under `ryzen-ai-install/venv/bin` |
| VitisAI EP libs | under venv `site-packages/onnxruntime/capi/` (wave0) |
| RyzenAI-SW samples | `/home/hunter/Projects/ai/RyzenAI-SW/` (CNN, LLM, CVML, onnx-benchmark) |
| CVML xclbins | `Ryzen-AI-CVML-Library/linux/onnx/ryzen14/` (prior **spent** research PDI class) |
| dragon-npu DD dump | `tools/dump_dd_txn.sh` / `capture_mladf_instr_vendor.*` (HITL-gated; L3 + allow) |
| System XRT inventory script | `scripts/inventory_txn_sources.sh` (expects `/usr/bin/xrt-smi`, `/usr/lib/libxrt_*` when host package installed) |

Docs historically note host packages: `xrt-smi`, `xclbinutil`, `xrt-plugin-amdxdna` under `/usr` layout (`docs/XDNA2_TXN_RESEARCH.md`). This hunt did **not** re-run live device probes or EXEC.

### Honest blocked for “complete a job → residual-matched peel”

1. **No** new post-EXEC ctrltext from a job that completed **ERT Completed + OFM allclose** on **this** firmware under the capture dir.
2. Flexml STX research PDIs already **spent**; pure re-live banned.
3. Hardened `exec_cmd_allow` / L3 default closed for dragon-npu live EXEC; EP path is vendor path, not pure-direct residual close.
4. `capture-instr` / vendor post-patch path is operator HITL (sudo/L3 sandwich when unlocked) — **not** agent-legal without unlock + named reason.
5. `xrt-bridge` feature is a **refuse** research stub (no real libxrt session for Pass).

**If operator chooses EP / XRT intercept later (not agent live this slice):**

```text
source /home/hunter/Projects/ai/ryzen-ai-install/activate-eepy.sh   # or full venv
# run a workload that already loads on this FW; capture instr/ctrltext/PDI with honesty
bash tools/pass_unblock_once.sh --ingest /absolute/path/to/file NAME
bash tools/operator_external_capture.sh --compare
# invent_pass=0; match ≠ Pass; new live still needs new C_UNLOCK + named reason
```

---

## 5. Side-channel dump feasibility (M9) — notes only

No invasive strace/gdb performed this hunt. Existing product docs already constrain what is safe/useful.

| Method | Feasibility | Residual honesty |
|--------|-------------|------------------|
| **DD `get_instr_bo` / instruction_cache** | Documented offline: Transaction/cache image only; **not** proven live `run_` BO after execute. Capture tool already tries registry + Transaction map fallbacks (`tools/capture_mladf_instr_vendor.cpp`). | Registry mode ≠ residual-matched live peel. Unpatched classic 1688 may already EXIST (CLASS A body) without closing FW residual. |
| **capture-instr HITL** | Scripted path exists; needs operator L3 sandwich when policy allows; OFM non-zero was observed historically while post_patch dump still hard. | Operator-only; not agent default. |
| **strace on XRT/EP** | Feasible in principle to log `ioctl` / file opens; **invasive**, noisy, EULA-sensitive, does not invent residual match. Prefer only if operator runs a known-good job and wants IOCTL fingerprints. | Dump logs ≠ Pass. Do not invent bins from strace text. |
| **gdb attach to live NPU process** | High risk (wedging driver historically from bad EXEC); not recommended as residual research default. Prefer journal + last_apply + public capture-instr path. | Forbidden as casual agent action. |
| **xclbin section / PDI dump offline** | Safe offline: `tools/dump_xclbin_pdi.sh`, soft section markers, Route B structure tools on **already landed** bytes. | Markers without load-accept ≠ residual identity. |
| **Journal / last_apply** | Safe read of existing `/tmp/dragon_npu_last_apply_*.json` + journal windows after operator shots. | Already used for WAIT EINVAL / IO_PAGE_FAULT class; invent_pass=0. |

**Summary:** Side-channel is **not** the highest-value offline agent work while public AOT is already classified and live residual-matched material is still missing. Prefer operator-completed working-stack peel or residual-matched material over gdb theater.

---

## 6. Agent-offline vs operator-install ranking

| Rank | Who | Action | Why |
|------|-----|--------|-----|
| **1** | **Operator (or unlocked live)** | Working-stack / residual-matched material: job that **completes** on this FW → capture post-patch ctrltext (and optional PDI) → `--ingest` + PROVENANCE honesty | Only avenue that can satisfy load-accepted / residual identity; public AOT ≠ this |
| **2** | **Agent offline** | Soft classify / Route B axes / residual match notes on **present** three AOT sets; keep taxonomy honest (`public-npu2-small` / other vs `mladf-classic-1688` / spent flexml) | Ready; mismatch expected; invent_pass=0 |
| **3** | **Agent offline (optional)** | Cascade (or matrix_vector) AOT under existing mlir-aie-capture env → new sha only if different → ingest | Toolchain ready; still RESEARCH ONLY; not required for residual close |
| **4** | **Operator install** | FastFlowLM / Lemonade if they want intercept of a stack that already loads | Trees absent |
| **5** | **Operator HITL** | EP/XRT sample job + capture-instr / peel when L3 policy allows | Tooling present; no residual-matched peel landed |
| **6** | **Frozen** | Re-live spent flexml; reopen CONSUMED unlocks; invent Pass from attach/capture/AOT alone | Banned |

### Forbidden (held)

- Live NPU apply / mint unlock / ALLOW=1 from this report
- Invent Pass / invent capture bytes / residual_matched_claim from public AOT lengths
- Empty reconfirm mill / re-live spent flexml

---

## 7. Top 3 ranked next material actions

1. **Prefer residual-matched or load-accepted material** (operator or unlocked live): a stack that reaches **ERT Completed + useful OFM** on this firmware, then real file under `testdata/xdna2_cmd/external_known_good/` with honest PROVENANCE. Public IRON AOT does **not** substitute.
2. **Agent offline only if new named delta:** optional **cascade** AOT with modest dims using the commands above → land only if size/sha is new; soft classify; still invent_pass=0 and residual_matched_claim=0. Do **not** re-ingest 300/660/2592.
3. **Operator M4/M8 only if they choose install/run:** FastFlowLM or Lemonade, **or** a known-good EP/XRT sample on this host, then capture + ingest; agent does not live-apply without a **new** unlock + named offline reason.

---

## Footer

```
invent_pass=0
residual_matched_claim=0
product_pass=0
ran_on_npu=0
Pass=No
C_UNLOCK_active=none
public_aot_present=1
public_aot_sets=3
public_npu2_small=vector_300+single_core_660
whole_array_ctrltext_bytes=2592
cascade_aot=not_built
fastflowlm=absent
lemonade=absent
working_stack_live_exec_capture=missing
report=.agents/reports/wave3-material-hunt.md
```
