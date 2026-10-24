# Wave W0 Track M — material avenues (M1 / M3 / M4)

**Date:** 2026-08-11
**Scope:** Offline audit of capture material avenues. No live NPU apply. No unlock mint. No ALLOW=1.
**Honesty:** invent_pass=0 · product_pass=0 · ran_on_npu=0 · Pass=No · residual_matched_claim never from public AOT alone.

Related: `docs/CAPTURE_MATERIAL_MLIR_AIE.md`,
`testdata/xdna2_cmd/external_known_good/PROVENANCE.md`,
`.agents/reports/impl-feedback-larger-matmul-material.md`,
`.agents/reports/wave0-material-tools.md` (M5/M6).

---

## M1 / M3 — Capture material audit

### Playbook (`docs/CAPTURE_MATERIAL_MLIR_AIE.md`)

| Item | Status |
|------|--------|
| Source tree ≠ capture material | Documented and enforced |
| Prefer smallest example first | vector → single_core → whole_array (all noted landed) |
| Ingest via `pass_unblock_once.sh --ingest PATH [NAME]` | Documented |
| PROVENANCE + soft compare after real files | Documented |
| FastFlowLM / Lemonade as fallback intercept | Documented (see M4) |
| invent_pass=0 footer | Present |

Playbook already lists **three** public sets as landed (vector, single_core, whole_array). Dual **`public-npu2-small`** tag still correct for 300 B + 660 B; whole_array 2592 B is a **larger** public AOT set (length class `other`, research only).

### Capture dir inventory

Path: `/home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/external_known_good/`

**Present (concrete binaries + PROVENANCE):**

| Set | Ctrltext size | sha16 (ctrltext) | Size-class / tag | Role |
|-----|---------------|------------------|------------------|------|
| vector_scalar_add (primary `working_stack_ctrltext.bin` + siblings) | **300 B** | `aaeb5b68cc298a45` | **`public-npu2-small`** RESEARCH ONLY | Minimal load peel |
| single_core matmul 64³ i16→i32 | **660 B** | `de8671245bb72436` | **`public-npu2-small`** RESEARCH ONLY | Small matmul AOT |
| whole_array matmul 4×4 128³ i16→i32 | **2592 B** | `7ee4b267ce31cd70` | length → **`other`** (not small band ≤1024); still public IRON AOT research | Larger multi-core AOT |

Each set also has: `insts.bin`, `insts.elf`, `xclbin`, `pdi`, representative `*_core.elf` (PROVENANCE tables).

**`pass_unblock_once` bin count (script rule):** only non-oracle `*.bin` / `*.ctrltext` at depth 1. Expected **7**:

1. `working_stack_ctrltext.bin`
2. `mlir_aie_vector_scalar_add_npu2.ctrltext.bin`
3. `mlir_aie_vector_scalar_add_npu2.insts.bin`
4. `mlir_aie_single_core_matmul_npu2.ctrltext.bin`
5. `mlir_aie_single_core_matmul_npu2.insts.bin`
6. `mlir_aie_whole_array_matmul_npu2.ctrltext.bin`
7. `mlir_aie_whole_array_matmul_npu2.insts.bin`

`.elf` / `.pdi` / `.xclbin` are research siblings, not bag-bin count.
**blocked_on_external_bytes=0** (bins ≥ 1). Capture folder empty is **not** the residual.

### Host AOT build tree (already used; no new land needed this wave)

Outside dragon-npu (read-only for this audit):

| Path | Content |
|------|---------|
| `/home/hunter/Projects/ai/mlir-aie` | Clone HEAD `a8fea363…` (per PROVENANCE) |
| `/home/hunter/Projects/ai/mlir-aie-capture/venv` | mlir-aie wheel 1.4.0 + Peano |
| `/home/hunter/Projects/ai/mlir-aie-capture/tools/aiebu-asm` | Symlink → ryzen-ai-install |
| `…/aot_vector_scalar_add/` | Sources already copied into capture dir |
| `…/aot_single_core_matmul_npu2/` | Same |
| `…/aot_whole_array_matmul_npu2/` | Same (+ 16 core ELFs in `.prj/`; one representative landed) |

**No NEW unlanded AOT sets found** under `mlir-aie-capture/` beyond the three already ingested. Do not re-ingest same sha for “progress.”

### Gaps (honest)

| Gap | Exact path / command | Blocks Pass? |
|-----|----------------------|--------------|
| **Working-stack live EXEC capture** (post-patch ctrltext from a stack that **completed ERT Completed + useful OFM on this firmware**) | Not present. Public AOT is host-generated compile-only. | **Yes** for residual identity / load-accepted |
| **Lab classic 1688 identity match** | Soft compare vs staged mladf 1688 still mismatch (300/660/2592 ≠ 1688 body) | Expected; not fixed by more small AOT |
| **flexml residual-matched program** (not spent PDI re-live) | All three flexml STX PDIs **Spent** under CONSUMED unlocks | Yes for that avenue |
| **cascade matmul AOT** (optional larger/different topology) | Source: `/home/hunter/Projects/ai/mlir-aie/programming_examples/basic/matrix_multiplication/cascade/` — **not built** this host | No (optional research only) |
| **matrix_vector** example AOT | Source under same matmul tree — **not built** | No (optional) |
| **passthrough_kernel** | Playbook prefer-first; not required once vector landed | No |

**Not missing for offline classify:** public IRON AOT bytes (three sets present).

### Soft vs residual oracles (mismatch expected)

| Stream | size | vs lab `mladf-classic-1688` | vs spent `flexml-2x4x4-class` PDI |
|--------|------|----------------------------|-----------------------------------|
| vector 300 | small public | mismatch | N/A (ctrltext ≠ PDI) |
| single_core 660 | small public | mismatch | N/A |
| whole_array 2592 | other | mismatch | N/A |
| whole_array PDI 66864 | opaque AOT | N/A | ≠ spent flexml sizes/sha |

**Material ≠ Pass ≠ product known-good ≠ C-ready ≠ unlock alone ≠ residual identity.**

---

## M4 — FastFlowLM / Lemonade / Ryzen AI EP probe (read-only)

### FastFlowLM

| Probe | Result |
|-------|--------|
| `/home/hunter/Projects/ai/FastFlowLM` | **Absent** |
| Other obvious clone under `~/Projects/ai` | **Not listed** |
| Capture bins from FastFlowLM intercept | **None** |

**Blocked for capture production:** no install / clone.
**If operator wants this avenue:**

```text
# Outside dragon-npu; follow upstream README (not re-hosted here)
git clone https://github.com/ROCm/FastFlowLM.git
# build/run a workload that already loads on this firmware class
# then capture post-patch ctrltext / instr BO / CONFIG_CU PDI
bash /home/hunter/Projects/ai/dragon-npu/tools/pass_unblock_once.sh \
  --ingest /absolute/path/to/captured_ctrltext.bin working_stack_ctrltext.bin
# fill PROVENANCE honesty: invent_pass=0; live EXEC yes/no on this FW
```

Source checkout alone is **not** capture material (same rule as mlir-aie).

### Lemonade

| Probe | Result |
|-------|--------|
| `/home/hunter/Projects/ai/lemonade` or lemonade-sdk tree | **Absent** under `~/Projects/ai` |
| Capture bins from Lemonade intercept | **None** |

**Blocked.** Same ingest end-state as FastFlowLM if operator installs later:
https://github.com/lemonade-sdk/lemonade → run loadable path → real file → `--ingest`.

### Ryzen AI EP / ORT VitisAI (present, not residual-matched capture)

| Item | Path / note |
|------|-------------|
| Full Ryzen AI venv | `/home/hunter/Projects/ai/ryzen-ai-install/venv` |
| VitisAI EP shared libs | `…/site-packages/onnxruntime/capi/libonnxruntime_providers_vitisai.so` (+ `libonnxruntime_vitisai_ep.so`, `libonnxruntime_providers_ryzenai.so`) |
| flexml package | `…/site-packages/flexml/` (STX xclbin research; **spent** PDIs already live-shot) |
| aiebu-asm | `…/venv/bin/aiebu-asm` (also symlinked for mlir-aie-capture) |
| eepy-min path | `/home/hunter/Projects/ai/ryzen-ai-install/venv-eepy-min/` + `docs/RYZEN_AI_EEPY_MIN.md` |
| RyzenAI-SW examples | `/home/hunter/Projects/ai/RyzenAI-SW/` (CNN / LLM / CVML; vendor samples) |
| CVML overlays | `…/Ryzen-AI-CVML-Library/linux/onnx/ryzen14/*.xclbin` (prior spent research PDI class) |

**What EP presence means:** vendor EP tooling is **installed**. Smoke / provider list can work offline of pure-direct.
**What it does not mean:** product Pass, residual-matched ctrltext for dragon-npu lab, or free invent of capture bytes.

**Blocked for “working capture” from EP this wave:**

1. No new post-EXEC ctrltext peel landed under `external_known_good/` from an EP session that completed ERT Completed + OFM on **this** firmware.
2. Flexml STX research PDIs already **spent** under CONSUMED unlocks; pure re-live banned.
3. EP path ≠ pure-direct residual close (`docs/RYZEN_AI_EEPY_MIN.md`: EP is vendor path, not dragon-npu EXEC_CMD research).
4. Hardened `exec_cmd_allow` may block EP EXEC until operator lab allow (product default offline).

**Steps if operator chooses EP intercept later (not agent live):**

```text
source /home/hunter/Projects/ai/ryzen-ai-install/activate-eepy.sh   # or full venv
# run a *known-good-on-this-FW* EP workload; capture instr/ctrltext/PDI with PROVENANCE
bash tools/pass_unblock_once.sh --ingest PATH NAME
bash tools/operator_external_capture.sh --compare
# still invent_pass=0; match ≠ Pass; new live still needs new C_UNLOCK + named reason
```

---

## Avenue ranking (offline honesty)

| Rank | Avenue | State |
|------|--------|--------|
| 1 | **Load-accepted / residual-matched program** (live ERT Completed + OFM allclose via `decide_ran_on_npu`) | **Open** — highest value; public AOT ≠ this |
| 2 | Soft classify / structure tools on present dual+triple public AOT | **Ready offline** (mismatch expected) |
| 3 | Optional cascade / matrix_vector AOT | Source only; not required |
| 4 | FastFlowLM / Lemonade intercept | **Blocked** — not installed |
| 5 | EP-generated working-stack peel | Tooling present; **no** residual-matched capture landed; not agent live |
| 6 | Re-live spent flexml / skip-CU under CONSUMED | **Frozen** |

---

## Forbidden (this wave — held)

- Live NPU apply / mint unlock / ALLOW=1
- Invent Pass / invent capture bytes
- Empty reconfirm mill / re-live spent flexml
- Claim residual_matched from public AOT lengths alone

---

## Footer

```
invent_pass=0
product_pass=0
ran_on_npu=0
Pass=No
C_UNLOCK_active=none
public_aot_present=1
public_aot_sets=3
public_npu2_small=vector_300+single_core_660
whole_array_ctrltext_bytes=2592
working_stack_live_exec_capture=missing
fastflowlm=absent
lemonade=absent
ryzen_ai_ep_libs=present
residual_matched_claim=0
```
