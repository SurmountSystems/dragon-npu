# Host capture research inventory (load-accepted wall)

**Date:** 2026-08-11
**Scope:** Read-only host + in-tree audit of prior capture research, tooling, and gaps.
**Honesty:** invent_pass=0 · product_pass=0 · ran_on_npu=0 · Pass=**No** · residual_matched_claim=0
**Unlock:** none active (do not mint from this report).

---

## 1. Inventory of useful prior research / docs

### In-tree playbooks (primary)

| Path | Summary | Bins? | NPU EXEC complete? |
|------|---------|-------|--------------------|
| `/home/hunter/Projects/ai/dragon-npu/docs/CAPTURE_MATERIAL_MLIR_AIE.md` | Operator playbook: mlir-aie/IRON → collect xclbin/insts/elf/ctrltext → `--ingest` → PROVENANCE. Source tree ≠ capture. Peano/aiebu/FastFlowLM/Lemonade fallback. | Points at landed research AOT | **No** Pass claim; AOT is compile-side |
| `/home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/external_known_good/PROVENANCE.md` | Capture-dir law: ingest path required; refuse lab oracles / multi-txn as load-accepted; kit commands | Dir has research peels | Working-stack live EXEC capture **missing** |
| `/home/hunter/Projects/ai/dragon-npu/docs/CONSTRUCTION_MAP_APP_LOAD.md` | Public stack: CONFIG_CU + PDI then aiebu ctrltext vs lab START_NPU-only | N/A (map) | Hypothesis, not Pass |
| `/home/hunter/Projects/ai/dragon-npu/tools/pass_unblock_once.sh` | Layer A: status / offline / `--ingest PATH` / live refuse without unlock | Uses capture dir | Offline only by default |
| `/home/hunter/Projects/ai/dragon-npu/tools/host_npu_stack_inventory.sh` | Read-only host inventory (accel, tools, samples, candidates) | Reports only | No EXEC |
| `/home/hunter/Projects/ai/dragon-npu/tools/capture_from_host_candidate.sh` | Fail-closed candidate path; refuses multi-txn as load-accepted | Refuses invent | No invent load-accepted |

### Wave / join reports (already done; do not re-spin)

| Path | Summary |
|------|---------|
| `.agents/reports/wave0-material-avenues.md` | M1/M3 public AOT audit; M4 FastFlowLM/Lemonade **absent**; EP libs present; ranking load-accepted first |
| `.agents/reports/wave3-material-hunt.md` | Capture dir sizes/sha; cascade was then unbuilt (superseded by S7 land); M4/M8/M9 ranking |
| `.agents/reports/wave3-s7-cascade-closeout.md` | Cascade AOT **landed** (4512 B ctrltext); RESEARCH ONLY; L7 not justified |
| `.agents/reports/wave4-host-inventory.md` | Live host: `/dev/accel` present; `xrt-smi` + `xclbinutil` on PATH; capture files=25; multi-txn 32208 candidate **refused** as load-accepted |
| `.agents/reports/wave4-join.md` / `wave0-join.md` / `wave1-join.md` | Campaign joins; invent_pass=0; Pass=No |
| `.agents/reports/docs-mlir-aie-capture-playbook.md` | Docs pin that clone ≠ bins |
| `.agents/reports/impl-mlir-aie-material-playbook.md` | Material playbook implement note |
| `.agents/reports/residual-pin-mlir-aie-material-landed.md` | Dual public AOT residual pin (research only) |
| `.agents/reports/pdi-xclbin-inventory.md` | Offline PDI/xclbin inventory |
| `.agents/reports/plan-explore-capture-unblock.md` | Explore residual for capture wall |

### Host trees outside dragon-npu

| Path | Summary | Bins? | NPU run completed (this FW residual-matched)? |
|------|---------|-------|-----------------------------------------------|
| `/home/hunter/Projects/ai/mlir-aie` | Full Xilinx mlir-aie + IRON Python; programming_examples | Source + build system | Source ≠ capture |
| `/home/hunter/Projects/ai/mlir-aie-capture/` | AOT work tree: venv (mlir-aie 1.4 + Peano), `tools/aiebu-*`, four `aot_*` build outs | **Yes** (ctrltext/insts/elf/pdi/xclbin) | Host **AOT emit** done; **not** documented ERT Completed + OFM allclose as load-accepted product Pass |
| `/home/hunter/Projects/ai/ryzen-ai-install/` | Full Ryzen AI venv + eepy-min + xrt-shim; `aiebu-asm`, flexml, VOE xclbins, VitisAI EP | **Yes** (vendor EP stack) | EP smoke possible; **no** residual-matched peel under capture dir from EP |
| `/home/hunter/Projects/ai/RyzenAI-SW/` | Official samples (CNN, LLM/OGA, CVML, onnx-benchmark); Lemonade docs only in RAG Dataset rst | Samples + CVML xclbins | Vendor path; CVML PDIs already **spent** in prior live shots |
| FastFlowLM / Lemonade trees under `~/Projects/ai` | **Absent** (wave0/wave3 confirmed) | None | N/A |
| `/home/hunter/Downloads` | No NPU capture / FastFlowLM / mlir-aie load-accepted material found in focused scan | N/A | N/A |
| `/home/hunter/Projects/kernel/xdna-driver*` | Driver + shim_test ctrltext paths; `xrt-smi validate` in README | Kernel/test source | Not product capture bag |

### Capture dir current state (concrete)

Path: `/home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/external_known_good/`

| Set | Ctrltext (approx) | Tag | Role |
|-----|-------------------|-----|------|
| `working_stack_ctrltext.bin` + vector siblings | **300 B** sha16 `aaeb5b68cc298a45` | `public-npu2-small` RESEARCH ONLY | Primary bag peel |
| single_core matmul npu2 | **660 B** | `public-npu2-small` RESEARCH ONLY | Small matmul AOT |
| whole_array matmul npu2 | **2592 B** | `other` RESEARCH ONLY | Multi-core AOT |
| cascade matmul npu2 | **4512 B** | `other` RESEARCH ONLY | Cascade AOT (S7) |

Each set has `insts.bin` / `insts.elf` / `pdi` / `xclbin` / core elf siblings. **≠** lab classic 1688 · **≠** spent flexml · **≠** load-accepted · **≠** Pass.

**Refused as load-accepted:** multi-txn `gt_capture/instr_vendor_post_patch.bin` (32208); local GT oracles; VE2 validate.

---

## 2. Host tooling / installs (PATH-ish evidence)

From `.agents/reports/wave4-host-inventory.md` (2026-08-11) plus this tree walk:

| Tool / root | Status |
|-------------|--------|
| `/dev/accel` / `accel0` | Present, readable |
| `amdxdna` module + `/lib/firmware/amdnpu/*` | Present |
| `xrt-smi` | **Present** → `/usr/bin/xrt-smi` |
| `xclbinutil` | **Present** → `/usr/bin/xclbinutil` |
| `xbutil`, system `aiebu-asm`/`aiebu-dump` on default PATH | **Missing** on PATH |
| aiebu under venv / mlir-aie-capture | **Present** at `ryzen-ai-install/venv/bin/aiebu-asm` and `mlir-aie-capture/tools/aiebu-*` |
| `ryzen-ai-install` full + eepy-min | **Present** |
| `RyzenAI-SW` | **Present** |
| FastFlowLM / Lemonade install | **Absent** |
| `exec_cmd_allow` | Missing / stock closed (no sticky L3) |
| Completing-stack candidates (inventory) | 1 path: multi-txn 32208 (refused as load-accepted) |

**Do not** put entire `ryzen-ai-install/venv/bin` first when building mlir-aie IRON (MLIR shared-lib clash); use mlir-aie-capture tools + venv path as prior playbook.

---

## 3. Gaps (what agent still cannot invent)

1. **Load-accepted capture:** post-patch ctrltext (and optional CU/PDI) from a stack that reached **ERT Completed + non-zero OFM allclose** on **this** firmware via product `decide_ran_on_npu`. Public AOT peels do **not** count.
2. **Residual identity** vs lab classic 1688 or spent flexml STX program images.
3. **FastFlowLM / Lemonade** intercept bytes (trees not installed).
4. **New unlock / live apply / ALLOW=1 / Pass / ran_on_npu** from research alone.
5. **Re-live spent flexml** fingerprints or CONSUMED `C_UNLOCK` reopen.
6. Treating **source clone**, **EP install**, or **xrt-smi present** as residual progress.

---

## 4. Ranked agent-doable next steps (Paths A / B / C) — no invent Pass

Convention aligned with kit / residual:

- **Path A:** CONFIG_CU / PDI / load-path research (offline structure, unspent PDI list, no re-live spent).
- **Path B:** Capture bag / external known-good / soft compare / classify.
- **Path C:** Live Pass apply under active unlock only (currently **blocked**: no unlock).

| Rank | Path | Who | Action | Still invent_pass=0? |
|------|------|-----|--------|----------------------|
| **1** | **B** | **Operator** | Place **load-accepted** working-stack capture under `testdata/xdna2_cmd/external_known_good/` (or real path + `pass_unblock_once --ingest PATH`) with honest PROVENANCE (EXEC completed on this FW: yes/no). | Yes until live decide Pass |
| **2** | **B** | Agent offline | Soft classify / Route B size-class / residual_match tools on **already present** public AOT only if a **named** new note is needed; do **not** re-ingest same sha 300/660/2592/4512. | Yes; mismatch expected |
| **3** | **A** | Agent offline | Keep offline CONFIG_CU / PDI geometry / unspent flexml list honest; **no** live attach without **new** unlock + named reason. | Yes; attach ≠ Pass |
| **4** | **B** | Operator optional | Install FastFlowLM or Lemonade **or** run EP/XRT sample that already loads → peel post-patch ctrltext → ingest. Agent does not live-apply. | Yes |
| **5** | **B** | Operator optional | `xrt-smi validate` / vendor sample for host health only; capture only if job truly completes and peel is real. | Validate ≠ Pass |
| **6** | **C** | Frozen until unlock | HITL Pass apply / L3 sandwich only under **new** host-minted `C_UNLOCK` + offline delta. | Unlock ≠ Pass |
| **—** | Frozen | All | Re-live CONSUMED unlocks; spent flexml; invent Pass; sticky ALLOW; empty reconfirm mill; multi-txn as load-accepted. | — |

### Path mapping one-liners

- **Path A agent-max offline residual:** already largely closed (pack, scorecards, syncobj pack/wire offline, unspent PDI list). No Pass.
- **Path B highest remaining wall:** load-accepted **operator material**, not more public AOT.
- **Path C:** not open (no active unlock; L8 not justified from wire alone).

---

## 5. Footer

```
invent_pass=0
product_pass=0
ran_on_npu=0
Pass=No
C_UNLOCK_active=none
public_aot_present=1
public_aot_sets=4
public_npu2_small=vector_300+single_core_660
whole_array_ctrltext=2592
cascade_ctrltext=4512
fastflowlm=absent
lemonade=absent
xrt_smi=present
xclbinutil=present
ryzen_ai_install=present
mlir_aie_capture_aot=present
working_stack_live_exec_capture=missing
load_accepted_wall=open
report=.agents/reports/plan-host-capture-research.md
```
