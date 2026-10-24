# W3 O9 — non-flexml unspent PDI scan (read-only)

**Date:** 2026-08-11
**Slice:** campaign try-everything wave3 offline inventory
**Scope:** host research xclbin/PDI candidates that are **not** the three spent flexml STX fingerprints + CVML 4x4 baseline `a1e65d04…`.
**Method:** RESIDUAL spent ledger + prior fingerprint dumps + directory listing + PROVENANCE. **No** force re-dump of spent flexml. **No** live. **No** unlock mint.

## Honesty

| Token | Value |
|-------|--------|
| invent_pass | **0** |
| product Pass | **No** |
| ran_on_npu | **false** (not claimed) |
| product_known_good | **0** |
| Active `C_UNLOCK` | **none** (first residual line CONSUMED; do not mint) |
| Re-dump spent flexml | **not done** (banned) |
| Scan alone | **≠** Pass **≠** residual match **≠** unlock |

Empty flexml-unspent list is honest success. Non-flexml unspent research blobs may still exist without justifying L4 alone.

---

## SSoT spent ledger (RESIDUAL.md + `pass_unblock_fingerprint_once.sh`)

Tool: `tools/pass_unblock_fingerprint_once.sh`
`is_spent_sha16` / `is_spent_flexml_tag` match residual (all three flexml STX + CVML 4x4).
S4: skip re-dump of spent flexml unless `--force` / `XDNA2_FP_FORCE_REDUMP=1` (not used here).

| tag | pdi_bytes | sha16 | Live | status |
|-----|-----------|-------|------|--------|
| flexml STX `unified-2x4x4` | 27200 | `0eeeb70c84df056e` | under `2ae69036` | **Spent** |
| flexml STX `llm-2x4x4` | 21664 | `ba0cd738fd870011` | under `8352882b` | **Spent** |
| flexml STX `unified-4x4` | 13792 | `7d172bf223a9b06b` | under `efd44f35` | **Spent** |
| CVML AIE2P 4x4 Overlay | 283840 | `a1e65d04add3422c` | prior unlocks | **Spent** |
| CVML AIE2P 4x4 Overlay_CFG0 | 283840 | `a1e65d04add3422c` | same fingerprint as 4x4 | **Spent** (same sha16) |

**Unspent flexml STX count:** **0** (all three Spent). Pure re-live of these sha16s is banned.

Prior fingerprint report `.agents/reports/pass-unblock-fingerprint-once.md` (2026-08-10) still labeled llm / unified-4x4 as unspent; **RESIDUAL live closeouts supersede** that table. Current script spent set is aligned with residual.

---

## Search roots (present)

| Root | Present | Notes |
|------|---------|--------|
| `/home/hunter/Projects/ai/ryzen-ai-install` (flexml package) | yes | STX + aie4 + apx + ve2 xclbins |
| `/home/hunter/Projects/ai/RyzenAI-SW/.../ryzen14/` | yes | CVML AIE2P overlays |
| `testdata/xdna2_cmd/external_known_good/` | yes | public mlir-aie npu2 AOT `.pdi` + `.xclbin` |
| `/tmp/dragon_npu_pdi.*` | yes | prior exclusive dumps (e.g. `Kw18Lw`, `4zSVPo`) |
| S5 `*mladf*.xclbin` / `*gemm*.xclbin` under install roots | count 0 | `.agents/reports/pdi-xclbin-inventory.md` honest empty |

---

## Candidate table (path · size · sha16 · spent vs unspent vs unknown)

### A. Spent research PDIs (ledger; do not re-live alone)

| path / source | pdi_bytes | sha16 | class | note |
|---------------|-----------|-------|-------|------|
| `…/flexml/…/stx/unified-2x4x4.xclbin` → PDI | 27200 | `0eeeb70c84df056e` | **spent** | `2ae69036` wait_cmd_einval |
| `…/flexml/…/stx/llm-2x4x4.xclbin` → PDI | 21664 | `ba0cd738fd870011` | **spent** | `8352882b` + IO_PAGE_FAULT |
| `…/flexml/…/stx/unified-4x4.xclbin` → PDI | 13792 | `7d172bf223a9b06b` | **spent** | `efd44f35` + IO_PAGE_FAULT |
| `…/ryzen14/AMD_AIE2P_4x4_Overlay.xclbin` → PDI | 283840 | `a1e65d04add3422c` | **spent** | orphan IO_PAGE_FAULT class |
| `…/ryzen14/AMD_AIE2P_4x4_Overlay_CFG0.xclbin` → PDI | 283840 | `a1e65d04add3422c` | **spent** | same sha16 as 4x4 baseline |

xclbin shell sizes (approx, from prior plan inventory): STX ~100–129 KiB; CVML 4x4 family larger. Dump dirs still hold measured bins under `/tmp/dragon_npu_pdi.Kw18Lw/` (llm, unified-4x4, CVML 4x4×2, 8x4x1) without re-dumping unified-2x4x4.

### B. Non-flexml unspent PDI fingerprints (known sha16; never live under residual)

| path | pdi_bytes | sha16 | class | note |
|------|-----------|-------|-------|------|
| `/home/hunter/Projects/ai/RyzenAI-SW/Ryzen-AI-CVML-Library/linux/onnx/ryzen14/AMD_AIE2P_8x4x1_Overlay.xclbin` → dump | **568224** | `19c430f04909c280` | **unspent** | Prior offline dump; DIFF vs CVML 4x4; **never live**; not product known-good; not flexml residual match |
| `testdata/xdna2_cmd/external_known_good/mlir_aie_vector_scalar_add_npu2.pdi` | **2528** | `fe438059f5d0e4eb` | **unspent** | public-npu2-small RESEARCH ONLY (PROVENANCE) |
| `…/mlir_aie_single_core_matmul_npu2.pdi` | **5536** | `cd3d9fa6577d1e37` | **unspent** | public-npu2-small RESEARCH ONLY |
| `…/mlir_aie_whole_array_matmul_npu2.pdi` | **66864** | `e721b80335e6d8ed` | **unspent** | larger public AOT RESEARCH ONLY; ≠ spent flexml sizes/sha |

Sibling public xclbins (not separate PDI class; listed for completeness):

| path | xclbin bytes | sha16 (xclbin) | note |
|------|--------------|----------------|------|
| `mlir_aie_vector_scalar_add_npu2.xclbin` | 8983 | `849d0ba7276ada71` | AOT container; PDI above |
| `mlir_aie_single_core_matmul_npu2.xclbin` | 11993 | `8a96c4cb5e01656c` | AOT container |
| `mlir_aie_whole_array_matmul_npu2.xclbin` | 73327 | `44ab78cf9eeb315e` | AOT container |

### C. Host xclbins present; PDI fingerprint **unknown** this scan (not dumped)

No force re-dump; these were never in the default fingerprint candidate list (except existence notes).

| path | PDI status | provisional class | note |
|------|------------|-------------------|------|
| `…/ryzen14/AMD_AIE2P_4x4_Overlay_CFG1.xclbin` | **unknown** | unknown | Present; may share family with spent 4x4; dump only if named reason later |
| `…/ryzen14/AMD_AIE2P_Nx4_Overlay.xclbin` | **unknown** | unknown | Present; never live-measured |
| `…/flexml/…/aie4/base.xclbin` | **unknown** | unknown | Small base; not Strix NPU4 product path (plan-explore note) |
| `…/flexml/…/apx/base.xclbin` | **unknown** | unknown | Present; not residual-matched claim |

### D. Present but **refuse** as AIE2 product research known-good

| path | class | note |
|------|-------|------|
| `…/flexml/…/ve2/dummy.xclbin` | refuse | VE2 / NOT_AIE2 |
| `…/flexml/…/ve2/unified-1x4x4.xclbin` | refuse | VE2 |
| `testdata/xdna2_cmd/ve2_latency/sources/ve2_runner_latency_validate.xclbin` | refuse | NOT_AIE2 research stub |

### E. Missing offline (not invented)

| reference | status |
|-----------|--------|
| xdna-driver `local_shim_test_data/npu4/gemm/gemm.xclbin` | **absent** |
| `*mladf*.xclbin` / `*gemm*.xclbin` under S5 roots | **count 0** |

---

## Counts

| Bucket | Count |
|--------|-------|
| Spent research PDI fingerprints (distinct sha16) | **4** (3 flexml STX + CVML 4x4; CFG0 same sha) |
| **Unspent** PDI with known sha16 (non-flexml) | **4** |
| Unspent flexml STX | **0** |
| Unknown PDI (xclbin present, not dumped) | **4** |
| Refuse (VE2 / NOT_AIE2) | **3+** |
| S5 mladf/gemm xclbin inventory | **0** |

**Unspent count (report headline):** **4** non-flexml unspent PDI fingerprints (8x4x1 + 3 public mlir-aie).
**Unspent flexml STX:** **0**.

---

## L4 live queue — can it open?

**No, not from this scan alone.**

L4 live needs **all** of:

1. **Unspent** material (or a named delta beyond spent sha16), **and**
2. A **new** host-minted `C_UNLOCK=` (none active; do **not** mint from this pin), **and**
3. A **named offline reason** (residual-matched / load-accepted / isolation experiment), not “we listed unspent paths.”

| Material | Eligible as research blob? | Alone enough for unlock mint? | Alone enough for Pass? |
|----------|----------------------------|-------------------------------|-------------------------|
| CVML 8x4x1 `19c430f0…` | yes (never live) | **no** | **no** |
| public-npu2 PDIs (`fe438059` / `cd3d9fa6` / `e721b803`) | yes RESEARCH ONLY | **no** (residual-mismatched) | **no** |
| spent flexml three | already measured | **no** (re-live banned) | **no** |
| CFG1 / Nx4 / aie4 / apx unknown | dump first if needed | **no** | **no** |

This slice: inventory only. **Unlock not minted. invent_pass=0. Pass=No.**

---

## Sources

- `RESIDUAL.md` § Spent vs unspent research PDI ledger
- `tools/pass_unblock_fingerprint_once.sh` (`is_spent_sha16`, spent tags, CVML candidates, S4 skip)
- `.agents/reports/pdi-xclbin-inventory.md` (S5 empty)
- `.agents/reports/pass-unblock-fingerprint-once.md` + `/tmp/dragon_npu_pass_unblock_fingerprint.Nl5moM` (8x4x1 dump)
- `testdata/xdna2_cmd/external_known_good/PROVENANCE.md` (public PDI tables)
- `.agents/reports/wave0-prep-inventory.md` (prior empty flexml-unspent)
- Host dirs: flexml `stx/` / `aie4/` / `apx/` / `ve2/`; CVML `ryzen14/`; capture bag; `/tmp/dragon_npu_pdi.*`

---

## Footer

```
invent_pass=0
not_c_ready=1
not_ran_on_npu=1
product_known_good=0
unlock_not_minted=1
unspent_flexml_stx=0
unspent_non_flexml_known_sha16=4
l4_live_queue_open_from_this_scan=0
```
