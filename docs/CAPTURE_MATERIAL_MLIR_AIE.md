# Capture material from MLIR-AIE (not the source tree)

**Date:** 2026-08-10 (size-class / RESEARCH ONLY pin: 2026-08-11)
**Status:** Operator playbook. Research only. **Not** a Pass. **Not** an unlock.
**Not** residual identity. **Not** unlock justification alone. **Not**
`ran_on_npu`. invent_pass=0 always.

Policy: empty reconfirm and empty-folder implement are **frozen** (`AGENTS.md`
§6–9). Residual tools need **concrete binaries** under
`testdata/xdna2_cmd/external_known_good/` (or an explicit `--ingest` path). A
cloned source tree is not capture material.

**Landed dual public material (RESEARCH ONLY):** size-class tag
**`public-npu2-small`** — vector_scalar_add ctrltext **300 B** + single_core
matmul ctrltext **660 B** under the capture dir. Still residual-mismatched to
spent **`flexml-2x4x4-class`** and lab **`mladf-classic-1688`**. Soft tools may
cite size-class / opcode / section / rela axes without claiming match.

Related:

- **Load-accepted operator page (Paths A/B/C):** [`LOAD_ACCEPTED_CAPTURE.md`](./LOAD_ACCEPTED_CAPTURE.md)
  — completed NPU run vs AOT research-only; refuse re-tag of 300/660/2592/4512
- Generic path-B extract: [`CONSTRUCTION_MAP_APP_LOAD.md`](./CONSTRUCTION_MAP_APP_LOAD.md)
  § Public-stack ctrltext extract playbook
- Capture dir template: `testdata/xdna2_cmd/external_known_good/PROVENANCE.md`
- One-shot Layer A entry: `bash tools/pass_unblock_once.sh`
- Path availability helper: `bash tools/load_accepted_path_status.sh`
- Sequence Step 1: `.agents/plans/plan-pass-unblock-sequence.md`

---

## Plain English rule (read first)

| Is this enough for residual tools? | Answer |
|------------------------------------|--------|
| `git clone https://github.com/Xilinx/mlir-aie` somewhere on disk | **No.** Source only. |
| Built example that produced `*.xclbin`, `insts_*.bin`, aiebu ELF, `.ctrltext` | **Maybe** — those **files** are candidates. |
| Those files copied under `testdata/xdna2_cmd/external_known_good/` + honest PROVENANCE | **Yes** for offline ingest/compare research. |
| Soft compare / fingerprint / offline C* after that | Still **≠** Pass · **≠** C-ready · **≠** unlock · **≠** residual identity · **≠** `ran_on_npu` |
| Dual public npu2 AOT bins present (`public-npu2-small`) | **Research only.** Completes empty-folder Step 1 for offline classify. **Does not** match spent flexml or lab classic 1688. |

Agents **must not** treat “mlir-aie clone exists” or “IRON checkout present” as
progress on the capture directory. Agents **must not invent** capture bytes.
Agents **must not** treat dual research bins as Pass, residual identity, or
unlock justification alone.

---

## Goal

Produce **real working-stack bytes** (ctrltext, aiebu ELF, instruction stream,
and/or CU/PDI-bearing xclbin pieces) from a **public** npu2 example path, then
**ingest** them so offline compare and residual tools can run against something
other than our pure-direct lab oracle.

This playbook does **not**:

- Claim product Pass
- Mint or reopen `C_UNLOCK`
- Set `XDNA2_ALLOW_EXEC_CMD=1` or open L3
- Invent `ran_on_npu`
- Vendor proprietary blobs into git as known-good Pass

---

## Public sources (accessed 2026-08-10)

| Role | URL |
|------|-----|
| **MLIR-AIE + IRON examples** (primary generator) | https://github.com/Xilinx/mlir-aie |
| AMD IRON (operator / LLM examples; sibling path) | https://github.com/amd/IRON |
| Peano / llvm-aie (`aie2p-none-unknown-elf` toolchain) | https://github.com/Xilinx/llvm-aie |
| aiebu (asm / dump ELF / `.ctrltext`) | https://github.com/Xilinx/aiebu |
| FastFlowLM (fallback intercept of a stack that already loads) | https://github.com/ROCm/FastFlowLM |
| Lemonade (fallback intercept) | https://github.com/lemonade-sdk/lemonade |
| Riallto (simpler AMD research examples) | https://github.com/AMDResearch/Riallto |

Follow **upstream** install docs for mlir-aie / IRON / Peano on your host. This
repo does **not** re-host the 11 GiB toolchain tree.

---

## Step-by-step playbook

### 1. Clone mlir-aie (recursive)

Outside this repo (example path; use any stable host path):

```bash
git clone --recursive https://github.com/Xilinx/mlir-aie.git
cd mlir-aie
git submodule update --init --recursive
# Record: git rev-parse HEAD  → you will paste this into PROVENANCE later
```

**Still not capture material.** Residual tools ignore this tree until you copy
built binaries into the capture directory (step 5).

### 2. Env for npu2; follow upstream install

- Target **npu2** / Strix-class host (Ryzen AI 300 class) when the example
  accepts a device flag (often `--dev npu2` or docs equivalent).
- Install **Peano / llvm-aie**, **XRT** (if the example path needs it), and
  **aiebu** per upstream README / programming guide.
- Do **not** skip submodules; missing Peano or aiebu usually means empty
  build outputs, not a “successful” residual step.

Exact package names and venv steps change upstream. Prefer their current
`README` / `programming_guide` over any frozen copy here.

### 3. Build the **smallest** public example first

Prefer minimal load / data-movement over full LLM mats. Search under
`programming_examples/` (and IRON equivalents) for names like:

| Prefer first | Why |
|--------------|-----|
| `passthrough_kernel` | Smallest “bytes moved” story |
| `vector_scalar_add` | Tiny compute, easier dump (**landed** under capture dir as first set) |
| `single_core` matmul (or smallest matmul) | Closer to residual *shape*, still small (**landed** second set: `mlir_aie_single_core_matmul_npu2.*`, M=K=N=64 i16→i32, ctrltext 660 B — size-class **`public-npu2-small` RESEARCH ONLY**; still ≠ **`mladf-classic-1688`** / ≠ **`flexml-2x4x4-class`** / ≠ Pass / ≠ residual identity) |
| `whole_array` matmul | Multi-core 4×N_cols parallel matmul (**landed** third set: `mlir_aie_whole_array_matmul_npu2.*`, M=K=N=128 i16→i32, `--n-aie-cols 4`, ctrltext **2592 B** — larger than single_core 660; still ≠ lab 1688 / ≠ flexml / ≠ Pass / ≠ unlock alone) |
| `cascade` matmul | Cascade-stream variant (**landed** fourth set: `mlir_aie_cascade_matmul_npu2.*`, M=K=N=128 i16→i32 tiles 32, `--n-aie-cols 4`, ctrltext **4512 B** sha16 `18cb653c05a9d4c8` — still ≠ lab 1688 / ≠ flexml / ≠ Pass / ≠ residual identity / ≠ unlock alone) |
| `getting_started` | Upstream on-ramp |

Build with the **upstream** recipe for that example (often a `make` target or
Python/IRON driver with npu2). Goal: **any successful on-device or offline
artifact emit**, not Pass inside dragon-npu.

If the host cannot run on NPU yet, still collect **host-generated** xclbin /
instruction bins when the build system writes them (document “built but not
executed on this firmware”).

### 3b. Prefer **completed NPU run** when aiming for load-accepted (W5 Path B)

AOT emit (paths with `--xclbin-path` / `--insts-path` only) is **compile-only**.
Landed peels from that path stay **RESEARCH ONLY** (`public-npu2-small` / other).

For **load-accepted** material (still ≠ product Pass until product decide), the
operator must run the example **to completion on the NPU** on this host, then
document `completed_on_this_fw=yes` in PROVENANCE.

Canonical operator page (exact commands, refuse re-tag list, Paths A/C too):

[`LOAD_ACCEPTED_CAPTURE.md`](./LOAD_ACCEPTED_CAPTURE.md)

**Smallest example — completed run (not compile-only):**

```bash
# Env: mlir-aie-capture venv + tools (see LOAD_ACCEPTED_CAPTURE Path B dry-run)
cd /home/hunter/Projects/ai/mlir-aie/programming_examples/basic/vector_scalar_add

# End-to-end JIT + NPU verify (upstream: standalone without --xclbin-path)
python3 vector_scalar_add.py --dev npu2

# Or run pre-built AOT artifacts on NPU (still a real device run):
# python3 vector_scalar_add.py --dev npu2 \
#   --from-xclbin /path/to/vector_scalar_add.xclbin \
#   --from-insts  /path/to/vector_scalar_add.insts.bin
```

**Refuse:** re-tagging existing public AOT ctrltext **300 / 660 / 2592 / 4512**
under `external_known_good/` as load-accepted without that completed-run
evidence. Same sha + wishful PROVENANCE is not progress.

After a true completed run: collect xclbin/insts/elf/ctrltext (step 4), ingest
(step 5), PROVENANCE with `completed_on_this_fw=yes` only when true (step 6).

### 4. Collect concrete artifacts

From the example **build/output** dirs (names vary; search under the example
tree), gather what exists:

| Artifact | Typical names / notes |
|----------|------------------------|
| xclbin | `*.xclbin` (may hold PDI / AIE partition) |
| Instruction stream | `insts_*.bin`, `*inst*.bin`, transaction bins |
| aiebu ELF | `*.elf` with `.ctrltext` / `.rela.dyn` when aiebu was used |
| ctrltext peel | `.ctrltext` section dump, or post-patch instruction BO image |
| Optional notes | kernel args, tile map, CONFIG_CU / PDI sizes |

Record for each file: **byte size** and **sha256** (or at least sha16 of first
16 hex of sha256).

Refuse as **our** capture known-good (do not install under these basenames):

- Lab/GT oracles: `instr_post_patch.bin`, `instr_vendor_post_patch.bin`,
  `ctrl_post_patch.bin`, `instr_raw_txn.bin`, `lab_instr.bin`, …
- VE2 / `NOT_AIE2` stubs (`validate.xclbin`, paths claiming VE2-only)

### 5. Copy into the capture directory

```bash
REPO=~/Projects/ai/dragon-npu   # or your dragon-npu root
CAP="$REPO/testdata/xdna2_cmd/external_known_good"

# Prefer the Layer A ingest path (PATH is required):
bash "$REPO/tools/pass_unblock_once.sh" --ingest /absolute/path/to/ctrltext.bin working_stack_ctrltext.bin

# Optional second file (e.g. aiebu ELF or xclbin research dump) — distinct NAME:
bash "$REPO/tools/pass_unblock_once.sh" --ingest /absolute/path/to/example.elf mlir_aie_passthrough.elf

# Same install surface as:
# bash tools/operator_external_capture.sh --install /path/to/file [NAME]
```

**`--ingest` requires `PATH`.** Bare `--ingest` with no path exits refuse (2).
`NAME` is a **single path component** only (no `/`, no `..`).

Placing files by hand under `$CAP/` is also fine if you then fill PROVENANCE
and run soft compare. Bins are gitignored; **do not** force-add secrets or
huge proprietary dumps to git.

### 6. PROVENANCE (required for honest research)

Edit `testdata/xdna2_cmd/external_known_good/PROVENANCE.md` capture table:

| Field | Example content |
|-------|-----------------|
| Source | `https://github.com/Xilinx/mlir-aie` + **commit** `git rev-parse HEAD` |
| Example path | e.g. `programming_examples/.../passthrough_kernel` |
| Build command | exact make/python line that produced the files |
| Target | npu2 / host model; whether EXEC completed on **this** firmware |
| Files | basename, size, sha256 (or sha16) |
| Honesty | research only; **invent_pass=0**; **not** proven product known-good on this firmware; match ≠ Pass |

If the example never completed EXEC on **this** host’s firmware, say so. Do not
upgrade “built offline” into “known-good Pass.”

### 7. Ingest + offline tools (after real files exist)

```bash
cd ~/Projects/ai/dragon-npu

# Status should show capture bins ≥ 1 when install worked
bash tools/pass_unblock_once.sh --status

# Soft compare / kit (no L3, no ALLOW, no invent Pass)
bash tools/operator_external_capture.sh --compare
# or: bash tools/operator_app_load_kit.sh b-compare

# Optional offline fingerprint pack (still invent_pass=0)
bash tools/pass_unblock_once.sh --agent-offline
```

C1–C6 / soft classify on real bytes remain **≠** C-ready **≠** Pass **≠** unlock
**≠** residual identity. Route B size-class / opcode / section / rela axes are
citable in unlock **reason text** only, without claiming match.

### 8. Residual pin (only after real files exist)

When capture dir has real non-empty allowed files + PROVENANCE:

1. Note sizes/sha, source, and size-class tags in residual / report (plain
   English). Tag dual public AOT as **`public-npu2-small` RESEARCH ONLY**.
2. Soft classify residual class if last_apply changes later.
3. Still **no** auto unlock mint; still **no** Pass claim; dual bins alone are
   **not** unlock justification.

If bins are still 0: say so once and **stop**. Do not re-spin empty implement.

---

## Supporting toolchains (when mlir-aie alone is stuck)

| Piece | Use |
|-------|-----|
| **Peano / llvm-aie** | Compile AIE cores; required by many mlir-aie examples |
| **aiebu** | Assemble classic txn → ELF; dump `.ctrltext` for compare |
| **AMD IRON** | Higher-level Python API on top of mlir-aie; same capture rule (need **outputs**, not clone) |
| **FastFlowLM / Lemonade** | **Fallback:** intercept instr BO / CONFIG_CU / post-patch ctrltext from a stack that **already loads** on this firmware class |
| **Riallto** | Simpler research examples if mlir-aie install wall is high |

Fallback intercept still ends at the **same** capture directory + PROVENANCE.
Source checkout of FastFlowLM alone is also **not** capture material.

---

## Agent rules (do not skip)

1. **Source tree ≠ capture material.** Clone + “submodules present” is not
   `external_known_good/` progress.
2. **Ingest needs a real file path:**
   `bash tools/pass_unblock_once.sh --ingest PATH [NAME]`
   Missing path → refuse (exit 2). Do not invent PATH.
3. **No invent bins.** Empty capture dir → report blocked once; idle.
4. **No Pass / unlock / residual identity / `ran_on_npu`** from this playbook
   alone. Dual **`public-npu2-small`** research bins ≠ Pass ≠ unlock alone ≠
   match to **`flexml-2x4x4-class`** or **`mladf-classic-1688`**.
5. **No** reopen of CONSUMED `C_UNLOCK` stamps from residual.
6. Prefer **smallest** example first; large LLM xclbins are harder to classify
   and are still not product known-good by default.
7. **AOT ≠ load-accepted.** Do **not** re-tag public AOT peels (300 / 660 /
   2592 / 4512) as load-accepted without completed-on-this-FW evidence.
   See [`LOAD_ACCEPTED_CAPTURE.md`](./LOAD_ACCEPTED_CAPTURE.md).

---

## Quick checklist

- [ ] mlir-aie recursive clone + **recorded commit**
- [ ] Peano / aiebu / upstream env for npu2
- [ ] Smallest example built
- [ ] **(Load-accepted track)** smallest example **completed on NPU** (not AOT-only)
- [ ] Collected `*.xclbin` / `insts_*.bin` / aiebu elf / `.ctrltext` (what exists)
- [ ] Sizes + sha recorded
- [ ] `pass_unblock_once.sh --ingest PATH [NAME]` (or hand-copy + PROVENANCE)
- [ ] PROVENANCE table filled (honesty: invent_pass=0; `completed_on_this_fw=` honest)
- [ ] Soft compare / status shows bins ≥ 1
- [ ] Residual note only after real files exist
- [ ] **No** unlock mint; **no** Pass claim
- [ ] **No** re-tag of existing public AOT 300/660/2592/4512 as load-accepted

---

## Honesty footer

```
invent_pass=0
product_pass=0
not_c_ready=1
not_ran_on_npu=1
source_tree_is_not_capture_material=1
aot_not_load_accepted=1
```
