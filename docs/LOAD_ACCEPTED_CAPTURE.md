# Load-accepted capture (operator paths A / B / C)

**Date:** 2026-08-11
**Status:** Operator playbook. **Not** product Pass. **Not** an unlock.
**Not** `ran_on_npu` unless live product decide says so. invent_pass=0 always.

Policy: empty reconfirm and empty-folder implement are frozen (`AGENTS.md` §6–9).
Agents **must not invent** capture bytes. Agents **must not** re-tag existing
public AOT peels as load-accepted without completed-run evidence.

---

## Plain English (read first)

| Question | Answer |
|----------|--------|
| What is still missing for residual progress? | A **load-accepted** program image: bytes from a stack that **actually finished** on **this** host firmware (useful output / ERT Completed class evidence), then placed under `testdata/xdna2_cmd/external_known_good/` with honest PROVENANCE. |
| Are the public mlir-aie peels (300 / 660 / 2592 / 4512 B) enough? | **No.** They are **RESEARCH ONLY** (compile/AOT emit). They do **not** count as load-accepted. |
| Does `xrt-smi validate` health green mean Pass? | **No.** Health / device probe ≠ product Pass ≠ `ran_on_npu`. |
| Is FastFlowLM / Lemonade installed here? | **Absent** on this host (as of W5). Operator may install; agent will **not** download or network-install. |
| Does offline validate / soft compare mean Pass? | **No.** invent_pass=0. Validate ≠ Pass. AOT ≠ load-accepted. |

### What “load-accepted” means (SSoT criteria)

Product-facing criteria (never claimed from offline alone):

1. Live job path with **real ioctl**
2. **ERT Completed** (or equivalent completed state the product classifies as pass-ready)
3. Non-zero OFM / useful output that **allclose** vs golden where the product requires it
4. Product `decide_ran_on_npu` true (see pure checklist in code:
   `format_material_residual_match_checklist`)

**For operator capture of material** (what you drop in the capture directory):

- Prefer peels from a run that **completed on this firmware** (not compile-only).
- PROVENANCE must say so honestly: `completed_on_this_fw=yes` only when true.
- Still **≠** product Pass until dragon-npu live decide is green under an active
  unlock recipe. Material land ≠ Pass ≠ unlock.

Refuse as load-accepted:

- Public AOT-only peels already under the capture dir (300 / 660 / 2592 / 4512)
  **without** a new completed-run on this FW
- Multi-txn wrong-body historical `instr_vendor_post_patch.bin` (32208 B)
- Lab oracles (`instr_post_patch.bin`, `lab_instr.bin`, …) and VE2 stubs

---

## Path map (three avenues)

| Path | Plain name | Host status (typical) | Agent may |
|------|------------|------------------------|-----------|
| **B** | IRON / mlir-aie completed NPU run | Trees often present; prior work was **AOT only** | Document + dry-run checklist; **not** invent completed-run evidence |
| **C** | XRT / `xrt-smi` | `xrt-smi` often present | Read-only inventory; journal scrape after **you** run validate/samples |
| **A** | FastFlowLM / Lemonade intercept | **Absent** | Install recipe only; no download |

Related deep playbooks:

- mlir-aie AOT + research peels: [`CAPTURE_MATERIAL_MLIR_AIE.md`](./CAPTURE_MATERIAL_MLIR_AIE.md)
- Capture table template: `testdata/xdna2_cmd/external_known_good/PROVENANCE.md`
- One-shot Layer A: `bash tools/pass_unblock_once.sh`
- Host inventory: `bash tools/host_npu_stack_inventory.sh`
- Path availability (this doc’s helper): `bash tools/load_accepted_path_status.sh`

---

## Path B — IRON / mlir-aie (completed run, not compile-only)

### Goal

Run the **smallest** public npu2 example **to completion on the NPU** on this
host, then capture xclbin / insts / aiebu ELF / ctrltext into the capture dir
with PROVENANCE `completed_on_this_fw=yes`.

**AOT compile-only is already done** for several examples. That path alone must
stay tagged **RESEARCH ONLY**. Do **not** re-label those bins as load-accepted.

### Dry-run checklist (no NPU required)

Use existing trees when present (example host paths; adjust if yours differ):

| Item | Example path |
|------|----------------|
| mlir-aie source / examples | `/home/hunter/Projects/ai/mlir-aie` |
| Capture venv + wheels | `/home/hunter/Projects/ai/mlir-aie-capture/venv` |
| aiebu-asm symlink only | `/home/hunter/Projects/ai/mlir-aie-capture/tools/aiebu-asm` |
| Prior AOT outputs (research) | `…/mlir-aie-capture/aot_vector_scalar_add/` etc. |

```bash
# 1) Trees present?
test -d /home/hunter/Projects/ai/mlir-aie && echo "mlir-aie=present" || echo "mlir-aie=absent"
test -d /home/hunter/Projects/ai/mlir-aie-capture/venv && echo "capture_venv=present" || echo "capture_venv=absent"
test -x /home/hunter/Projects/ai/mlir-aie-capture/tools/aiebu-asm && echo "aiebu-asm=present" || echo "aiebu-asm=absent"

# 2) Clean PATH (do NOT put entire ryzen-ai-install/venv/bin first — breaks aie.iron)
export VENV=/home/hunter/Projects/ai/mlir-aie-capture/venv
export CAP=/home/hunter/Projects/ai/mlir-aie-capture
export PATH="$CAP/tools:$VENV/lib/python3.14/site-packages/mlir_aie/bin:$VENV/bin:/usr/bin"
export PEANO_INSTALL_DIR="$VENV/lib/python3.14/site-packages/llvm-aie"
export PYTHONPATH="$VENV/lib/python3.14/site-packages/mlir_aie/python${PYTHONPATH:+:$PYTHONPATH}"
source "$VENV/bin/activate"
python -c "import aie.iron; print('iron_import=ok')"

# 3) Prior research AOT still on disk (not load-accepted)
ls -la "$CAP/aot_vector_scalar_add/" 2>/dev/null || echo "no prior AOT dir"

# 4) dragon-npu path status helper (read-only)
cd /home/hunter/Projects/ai/dragon-npu
bash tools/load_accepted_path_status.sh
```

Dry-run success = trees + import OK. **Still invent_pass=0.** Still no
load-accepted claim from AOT files already under `external_known_good/`.

### Exact commands: smallest example **to completion** on NPU

From this repo, `just npu-iron` finds the mlir-aie tree and wraps the same
example through `dragon-npu --external-runner`. You do not need `$MLIR_AIE`.
That wrap is **not** product Pass. Raw IRON (below) is the capture path.

Prefer `vector_scalar_add` (minimal). Upstream README: standalone (no
`--xclbin-path`) JITs and verifies on NPU; or build AOT then `--from-xclbin` /
`--from-insts`.

```bash
export VENV=/home/hunter/Projects/ai/mlir-aie-capture/venv
export CAP=/home/hunter/Projects/ai/mlir-aie-capture
export MLIR_AIE=/home/hunter/Projects/ai/mlir-aie
export PATH="$CAP/tools:$VENV/lib/python3.14/site-packages/mlir_aie/bin:$VENV/bin:/usr/bin"
export PEANO_INSTALL_DIR="$VENV/lib/python3.14/site-packages/llvm-aie"
export PYTHONPATH="$VENV/lib/python3.14/site-packages/mlir_aie/python${PYTHONPATH:+:$PYTHONPATH}"
source "$VENV/bin/activate"

EX="$MLIR_AIE/programming_examples/basic/vector_scalar_add"
cd "$EX"

# Option 1 — end-to-end on NPU (JIT + run + verify). Prefer when XRT/device open works.
# Record stdout: must show successful verify, not hang/timeout.
python3 vector_scalar_add.py --dev npu2

# Option 2 — reuse prior AOT artifacts, then run on NPU (still a real device run)
OUT="$CAP/aot_vector_scalar_add"
python3 vector_scalar_add.py --dev npu2 \
  --from-xclbin "$OUT/vector_scalar_add.xclbin" \
  --from-insts  "$OUT/vector_scalar_add.insts.bin"
```

**Success criteria for this path (operator honesty):**

- Script exits 0
- Upstream verify / golden check passes (not “compile finished”)
- No hang / indefinite wait that you kill without completion
- Optional: `bash tools/journal_npu_scrape.sh --hours 2` shows no
  APP_LOAD / IO_PAGE_FAULT storm for that window (soft evidence only)

If the run **fails** or only AOT was used: keep PROVENANCE as research-only.
**Do not** set `completed_on_this_fw=yes`.

### After success: where artifacts are + how to drop them

| Kind | Where to look |
|------|----------------|
| xclbin / insts / pdi / elf (AOT emit) | Example cwd, `--aot-dir`, or prior `$CAP/aot_*/` |
| After JIT run | Often under example tree / temp IRON dirs; re-emit with `--xclbin-path` / `--insts-path` / `--elf-path` if you need stable paths |
| ctrltext peel | `llvm-objcopy -O binary --only-section=.ctrltext insts.elf ctrltext.bin` (or byte-identical `insts.bin` when aiebu ctrltext == stream) |

Ingest into dragon-npu (PATH required; no invent):

```bash
REPO=/home/hunter/Projects/ai/dragon-npu
# Example names — use NEW basenames when this is a completed-on-FW peel, not a re-tag of research AOT
bash "$REPO/tools/pass_unblock_once.sh" --ingest /absolute/path/to/ctrltext.bin \
  working_stack_ctrltext_completed_fw.bin
# Optional siblings:
bash "$REPO/tools/pass_unblock_once.sh" --ingest /absolute/path/to/insts.elf \
  mlir_aie_vector_scalar_add_npu2_completed.insts.elf
```

Edit `testdata/xdna2_cmd/external_known_good/PROVENANCE.md`:

| Field | Completed-run example |
|-------|------------------------|
| Source stack | mlir-aie IRON `vector_scalar_add` **live NPU run** |
| Build / run command | exact `python3 … --dev npu2` line |
| Target | npu2; host model |
| **completed_on_this_fw** | **yes** (only if true) |
| Files | basename, size, sha256/sha16 |
| Honesty | invent_pass=0; material land **≠** product Pass; still need product live decide for Pass |

Then:

```bash
cd "$REPO"
bash tools/operator_external_capture.sh --compare
bash tools/pass_unblock_once.sh --status
```

### Refuse re-tag of existing public AOT

Already landed research peels (do **not** upgrade without new completed-run):

| Size (ctrltext) | Example | Tag |
|-----------------|---------|-----|
| 300 B | vector_scalar_add | `public-npu2-small` RESEARCH ONLY |
| 660 B | single_core matmul | `public-npu2-small` RESEARCH ONLY |
| 2592 B | whole_array matmul | research / `other` |
| 4512 B | cascade matmul | research / `other` |

Same sha + “I wish this counted” is **not** load-accepted. Need a **new** run
with completion evidence, or honest keep-as-research.

Full AOT playbook remains: [`CAPTURE_MATERIAL_MLIR_AIE.md`](./CAPTURE_MATERIAL_MLIR_AIE.md).

---

## Path C — XRT / `xrt-smi`

### Host status

| Item | Typical |
|------|---------|
| `xrt-smi` | Often `/usr/bin/xrt-smi` **present** |
| `xclbinutil` | Often present |
| Health validate | Operator may run; **health ≠ Pass** |

```bash
command -v xrt-smi && xrt-smi --version
# Device examine (read-only-ish; still not product Pass):
# xrt-smi examine
# Operator health probe (may touch device — you run it, not the agent default):
# xrt-smi validate
```

**Honesty:** `xrt-smi validate` success means the vendor tool thinks the device
is healthy enough for its suite. It is **not** dragon-npu product Pass, not
`ran_on_npu`, not residual-matched lab ctrltext, not an unlock.

### Where artifacts might land

| Source | Notes |
|--------|-------|
| XRT / validate working dirs | Tool-specific under `/tmp`, cwd, or XRT install samples |
| `/opt/xilinx/xrt` / `/usr/share/xrt` | Samples when installed |
| Host inventory | `bash tools/host_npu_stack_inventory.sh` lists roots + sample bins (no copy) |

Do **not** peel and claim load-accepted unless the job **truly completed** and
you can document what ran. Validate-only green is not enough for PROVENANCE
`completed_on_this_fw=yes` on a ctrltext peel.

### After you run validate / a completing sample

```bash
cd /home/hunter/Projects/ai/dragon-npu
# Soft journal scrape (user session; no sudo; never invents Pass)
bash tools/journal_npu_scrape.sh --hours 2
# Optional boot window:
# bash tools/journal_npu_scrape.sh --boot

# Only if you have a real completed-run peel path:
# bash tools/pass_unblock_once.sh --ingest /absolute/path/to/ctrltext.bin NAME
# fill PROVENANCE completed_on_this_fw=yes only when true
```

Peel only if the job **truly completes**. Fail / timeout / APP_LOAD in journal
→ research notes only, not load-accepted.

---

## Path A — FastFlowLM / Lemonade

### Honest host status: **absent**

As of W5 material hunt / host inventory:

| Stack | Status |
|-------|--------|
| FastFlowLM clone under `~/Projects/ai` (e.g. `FastFlowLM`) | **Absent** |
| Lemonade / lemonade-sdk tree | **Absent** |
| Capture bins from FLM/Lemonade intercept | **None** |

Agent **will not** `git clone`, `pip install`, or network-install these for you.

### Operator install recipe only (you run; agent does not download)

Follow **upstream** README (links accessed for reference; versions change):

| Stack | URL |
|-------|-----|
| FastFlowLM | https://github.com/ROCm/FastFlowLM |
| Lemonade | https://github.com/lemonade-sdk/lemonade |

Suggested operator shape (outside dragon-npu; not agent-executed):

```text
# Example only — use upstream docs for real flags and deps
cd ~/Projects/ai
git clone https://github.com/ROCm/FastFlowLM.git
# build/run a workload that ALREADY loads on this firmware class
# capture post-patch ctrltext / instr BO / CONFIG_CU PDI with honest notes
```

Same pattern for Lemonade if that is the stack that completes on your machine.

### Once present: scan → collect / ingest

Read-only places to look (examples; do not invent files):

| Area | Why |
|------|-----|
| `/tmp` | Transient BO dumps / tool outputs |
| `~/.cache` | Framework caches |
| Clone dir (`~/Projects/ai/FastFlowLM`, lemonade tree) | Build/run outputs |
| Prior inventory roots | `host_npu_stack_inventory.sh` SAMPLE/CANDIDATE paths |

Then same end-state as other paths:

```bash
REPO=/home/hunter/Projects/ai/dragon-npu
bash "$REPO/tools/host_npu_stack_inventory.sh"
bash "$REPO/tools/pass_unblock_once.sh" --ingest /absolute/path/to/ctrltext.bin working_stack_ctrltext.bin
# PROVENANCE: source=FastFlowLM|Lemonade, completed_on_this_fw=yes only if true
bash "$REPO/tools/operator_external_capture.sh --compare"
```

Source checkout alone is **not** capture material (same rule as mlir-aie).

---

## After any path succeeds (shared)

1. Real file under `testdata/xdna2_cmd/external_known_good/` (or `--ingest PATH`)
2. PROVENANCE filled: source, command, sizes/sha, **completed_on_this_fw**, invent_pass=0
3. Soft compare: `bash tools/operator_external_capture.sh --compare`
4. Still **no** product Pass until live product path + `decide_ran_on_npu`
5. Still **no** unlock mint from capture alone; never reopen CONSUMED `C_UNLOCK`

### Fail-closed refuse list (reminder)

| Refuse | Why |
|--------|-----|
| Re-tag 300/660/2592/4512 AOT as load-accepted | No completed-run evidence |
| Multi-txn 32208 vendor post_patch | Wrong body class |
| Lab `instr_post_patch.bin` / `lab_instr.bin` | Local oracle |
| VE2 `validate.xclbin` | NOT_AIE2 |
| Empty PROVENANCE invent tokens | Fail bag-drop honesty |

---

## Quick operator checklist

- [ ] Read load-accepted criteria (top of this doc)
- [ ] `bash tools/load_accepted_path_status.sh` (which of A/B/C available)
- [ ] Prefer Path B completed NPU run if mlir trees present
- [ ] Or Path C only with true completing job + journal scrape
- [ ] Or install Path A yourself if that is the stack that loads
- [ ] Ingest real path + PROVENANCE `completed_on_this_fw=yes` only when true
- [ ] Soft compare; invent_pass=0; **no** Pass claim; **no** unlock mint
- [ ] **Do not** re-tag existing public AOT peels as load-accepted

---

## Honesty footer

```
invent_pass=0
product_pass=0
not_c_ready=1
not_ran_on_npu=1
aot_not_load_accepted=1
validate_not_pass=1
flm_absent_until_operator_install=1
```
