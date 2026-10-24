# Wave W0 O3 — XRT/DD load map depth (offline)

**Date:** 2026-08-11
**Mode:** Offline only. No unlock mint. No live. **invent_pass=0**.
**Pass=No.** **ran_on_npu=false** (not claimed). Distance / attach / capture ≠ Pass.

---

## Goal

Deepen offline understanding of how a working public stack packs START_NPU +
CU/PDI order + BO pins vs our lab classic path. Update construction map only
with real new offline facts.

---

## Sources read

| Path | Use |
|------|-----|
| `docs/CONSTRUCTION_MAP_APP_LOAD.md` | Prior map + residual B1 + skip-CU checklist |
| `docs/XDNA2_XRT_CONTRACT_MLADF.md` | START_NPU / kernel-arg / rela contract |
| `docs/CAPTURE_MATERIAL_MLIR_AIE.md` | IRON generator playbook |
| `testdata/xdna2_cmd/external_known_good/PROVENANCE.md` | Landed dual + whole_array material |
| Live reports | flexml attach + skip-CU isolation (`1f761bf4`) |
| Prior offline | `.agents/reports/impl-next-load-path-offline.md`, Route A/B |

---

## New offline facts (additive)

### 1. Three construction paths (do not conflate)

| Path | Summary | Residual role |
|------|---------|---------------|
| **A. XRT/DD KMQ production** | xclbin → CREATE_HWCTX → **CONFIG_CU(PDI)** → aiebu peel/patch → START_NPU (`prop_count=0`, instr Dev + IFM/WTS/OFM) → EXEC/WAIT | Completing stack **not** captured as residual identity on this host |
| **B. Lab pure-direct** | CREATE_HWCTX (no xclbin) → optional CONFIG_CU → classic 1688 aiebu rela×12 → START_NPU xrt3 | Live residual class evolved: historical APP_LOAD `0x3000003`; post-flexml **`wait_cmd_einval`** |
| **C. mlir-aie / IRON AOT** | Offline emit xclbin+PDI+insts **without** host EXEC → aiebu-asm → peel ctrltext | Present under capture dir; **RESEARCH ONLY**; residual-mismatched |

Path C is a **generator of artifact roles**, not proof the firmware will accept
those bytes on this host.

### 2. Artifact roles (measured offline on bag)

| Artifact | Load-map role | Must not |
|----------|---------------|----------|
| peeled `.ctrltext` | START_NPU instruction body | treat as product known-good |
| `insts.bin` | pre-aiebu / raw txn | assume sha == peel (660/2592 differ; 300 same) |
| `insts.elf` | aiebu ELF + rela | confuse with Peano core ELF |
| `.pdi` | CONFIG_CU candidate | equate to spent flexml PDI sizes |
| `.xclbin` | container / section research | private body decode |
| `*_core.elf` | Peano compute kernel | use as START_NPU instr BO |

### 3. START_NPU + BO pin order (XRT-ish vs lab)

**Match (contract green offline):**

- `instruction_prop_count = 0` (kernel args ≠ props)
- Instr BO type **Dev**; peel ctrltext into content; page-pad BO common
- Kernel args xrt3: n_bos=3, IFM, WTS, OFM (lab also has count4+CTRL path)
- Rela overwrite default; lab staged map sym 1/2/3 → IFM/WTS/OFM

**Closed as sole APP_LOAD root (prior env HITL):** content vs bo size mode,
xrt3 vs count4, rela add, QoS/tiles, DevHeap 64 MiB align.

**Still open / hypothesis:**

- Production always has CU/PDI when xclbin path is used; lab CONFIG_CU is
  optional and was tried with **wrong** research overlays (spent flexml).
- **B1 hypothesis:** flexml PDI topology mismatched to lab classic START_NPU
  ctrltext (still hypothesis, not proven root).
- **Isolation fact:** skip-CONFIG_CU under `1f761bf4` still
  `wait_cmd_einval` with `config_cu_present=0` → overlay is **not** sole
  residual root. invent_pass=0.

### 4. Construction map update

Additive section landed in `docs/CONSTRUCTION_MAP_APP_LOAD.md`:

**§ Wave W0 depth — three construction paths + BO pins (2026-08-11)**

Also refreshed highest-value next questions (skip-CU already measured;
prefer residual-matched material; no empty reconfirm).

---

## What this is not

- Not Pass / not `ran_on_npu` / not residual_matched
- Not unlock mint / not live apply
- Not reconfirm-only: new map section + path taxonomy + isolation pin
- Not claim that public IRON bins are load-accepted on this firmware

---

## Files touched

| Path | Change |
|------|--------|
| `docs/CONSTRUCTION_MAP_APP_LOAD.md` | Additive W0 depth section; next-questions refresh |
| `.agents/reports/wave0-xrt-dd-load-map.md` | This report |

---

## Honesty lock

| Token | Value |
|-------|-------|
| invent_pass | **0** |
| product Pass | **No** |
| ran_on_npu | **not claimed** |
| residual_matched | **not claimed** |
| unlock | **none active** (all listed CONSUMED) |
