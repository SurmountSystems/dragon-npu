# Explore plan: matmul-matched PDI source discovery (not CVML research dump)

**Date:** 2026-08-10
**Mode:** read-only explore → plan report only (no product edits)
**Honesty:** invent_pass=0 · **no** product Pass · **no** `ran_on_npu` · **no** active `C_UNLOCK` (latest `97f38307` **CONSUMED**)

---

## Plain English goal

Live CONFIG_CU under research CVML/AIE2P PDI already measured attach (`config_cu=1`, 283840 B) and still failed the job (`wait_cmd_einval` + orphan IO_PAGE_FAULT). Product residual next step is **find a PDI that matches the matmul / 2x4x4 construction path**, dump it with existing tools, fingerprint it offline, then (only with a **new** host-minted unlock + named reason) run **one** live stock shot.

This plan is **source discovery**, not inventing bytes, not reconfirm gates, not Pass.

---

## Current live facts (do not re-open)

| Item | Value |
|------|--------|
| Last unlock | `C_UNLOCK=97f38307` **CONSUMED** |
| Experiment | research blob (`config-cu-run`); skip-PDI **not** run under that unlock |
| PDI | CVML/AIE2P dump `pdi_bytes=283840` sha16=`a1e65d04add3422c` |
| status | `wait_cmd_einval` (not false `wait_timeout`) |
| CONFIG_CU | measured `config_cu=1` |
| Pass | **no** · `ran_on_npu=false` · invent_pass=0 |
| Reports | `.agents/reports/live-shot-97f38307.md`, `impl-t4-live-97f38307-closeout.md`, `impl-t3c-pdi-identity.md` |

---

## 1) `tools/dump_xclbin_pdi.sh` (what it dumps today)

**Role:** opaque `xclbinutil --dump-section PDI:raw:OUT` for lab env `XDNA2_LAB_PDI_BLOB`.

**Default candidates** (first existing file wins):

1. `~/Projects/ai/RyzenAI-SW/Ryzen-AI-CVML-Library/linux/onnx/ryzen14/AMD_AIE2P_4x4_Overlay.xclbin`
2. `…/AMD_AIE2P_4x4_Overlay_CFG0.xclbin`
3. `…/AMD_AIE2P_8x4x1_Overlay.xclbin`

**Host status:** those CVML ryzen14 xclbins **exist** on this machine (also CFG1, Nx4). Script honesty already says:

- CVML / AIE2P overlay PDI **≠** mladf matmul known-good
- dump ≠ Pass ≠ unlock
- `research_pdi_not_product_known_good=1 invent_pass=0 not_ran_on_npu=1`

**Outputs:** `PDI_OK bytes=… sha256=…`, soft `pdi_fp len=… sha16=…`, optional `--print-env` for export lines.
**Also lists skip A/B:** unset `XDNA2_LAB_PDI_BLOB` / `XDNA2_LAB_SKIP_CONFIG_CU=1` / `hitl_config_cu_once.sh prep-skip-pdi`.

**Gap:** default candidate list is **only** CVML vision overlays. It does **not** include Strix flexml `2x4x4` xclbins that better match the staged mladf key topology.

---

## 2) Research PDI identity / fingerprint helpers

**Module:** `src/backends/amd_xdna2/silicon_contract.rs` (re-exported from `amd_xdna2`).

| API / const | Role |
|-------------|------|
| `RESEARCH_PDI_IDENTITY_CLASS` | `research_pdi_not_product_known_good` |
| `RESEARCH_PDI_DEFAULT_DUMP_BYTES` | **283840** (11bc9016 / 97f38307 live size) |
| `ResearchPdiSourceKind` | `Absent` / `CvmlAie2pResearchOverlay` / `UnknownOpaque` |
| `PdiBlobFingerprint` | `len_bytes` + sha256 prefix16 + full hex |
| `fingerprint_pdi_blob_bytes` | pure digest of opaque bytes |
| `research_pdi_identity_from_bytes` / `_from_last_apply_signals` | scorecard; **always** `product_known_good=false` |
| `research_pdi_identity_default_dump_locked` | size-only default-dump shape |
| `format_research_pdi_preflight_note` | soft preflight string |
| `parse_pdi_fingerprint_from_detail` | parse `pdi_fp len=… sha16=…` from last_apply detail |

**Soft wire:** `examples/classify_last_apply.rs` prints `research_pdi_identity=…`.

**Important:** taxonomy has **no** `MatmulMatched` / `FlexmlStx2x4x4` kind yet. A new blob from flexml would land as `UnknownOpaque` (or CVML if path still says overlay). Offline identity alone **never** sets product known-good.

**CONFIG_CU parse surface (related):** `LastApplyDetailSignals.config_cu` / `pdi_bytes` / `config_cu_failed`; checklist tokens `config_cu_present=` / `config_cu_attached_still_app_load=`. Attach signal ≠ Pass.

---

## 3) CONFIG_CU attach path (lab)

**Code:** `try_lab_attach_config_cu_from_env` in `src/backends/amd_xdna2/direct.rs`.

| Step | Behavior |
|------|----------|
| Trigger | non-empty `XDNA2_LAB_PDI_BLOB` after CREATE_HWCTX |
| Skip | env unset / empty → `Ok(None)` (historical no-CONFIG_CU path) |
| Attach | read capped file → `CREATE_BO(Dev)` → write opaque bytes → `config_hwctx_cu` with `(bo, cu_func)` |
| `cu_func` | `XDNA2_LAB_CU_FUNC` (default **0**) |
| Fail | status `config_cu_failed`, fail-closed, destroy hwctx; invent_pass=0 |
| Lifetime | `LabPdiBoGuard` keeps PDI BO until end of timed path |
| Detail token | `config_cu=1 pdi_bytes=N cu_bo=… cu_func=… invent_pass=0 not_ran_on_npu=1` |

**Skip tooling (T3c):**
`hitl_config_cu_once.sh prep-skip-pdi` / `run-stock-skip-pdi`, kit aliases `config-cu-skip` / `config-cu-run-skip`. Lab skips when blob unset; `XDNA2_LAB_SKIP_CONFIG_CU=1` is a **loud sandwich/log flag**, not a separate driver path.

**Public stack reference** (`docs/CONSTRUCTION_MAP_APP_LOAD.md` + shim `kmq/hwctx.cpp`):

```text
xclbin → parse CUs → for each CU: copy PDI → CONFIG_HWCTX(CONFIG_CU) → then aiebu/ctrltext job
```

Lab without blob creates tiles-only hwctx (no xclbin CU list). That is the construction gap, not proof that wrong PDI alone caused historical `0x3000003` or current WAIT EINVAL.

---

## 4) In-repo docs (matmul / CU / PDI hints)

| Doc | Useful claim for this plan |
|-----|----------------------------|
| `docs/CONSTRUCTION_MAP_APP_LOAD.md` | Working KMQ path always has CU/PDI step; lab optional via blob; host AIE2P dump “may be wrong CU for mladf”; next: mladf-related PDI or external known-good; Triton-XDNA / `aie.xclbin` secondary |
| `docs/AMD_MLADF_MATMUL_PACK.md` | Staged key `…_mladf_2x4x4_flat_…` — **topology token is 2x4x4**, not CVML 4x4 overlay names |
| `docs/XDNA2_XRT_CONTRACT_MLADF.md` | ELF+rela userspace green; residual was FW load/job reject; next = construction/byte research |
| `docs/HITL_C_APP_LOAD_RECIPE.md` | CONFIG_CU sandwich recipe; unlock stamps; ≥120s outer; attach ≠ Pass |
| `docs/BUILD_NPU_MATMUL_PROGRAM.md` | Matmul **txn** comes from DynamicDispatch `libtransaction.a` (ctrltext), **not** from CVML xclbin PDI |

**Split to keep straight:**

| Byte class | Source | Lab use |
|------------|--------|---------|
| **ctrltext / aiebu ELF** | DD txn dump → aiebu | START_NPU instruction (already staged) |
| **PDI / CU image** | xclbin PDI section(s) | CONFIG_CU after CREATE_HWCTX (optional today) |

Matching the matmul **txn** does not auto-select the matmul **PDI**. Matching the **2x4x4 / Strix** xclbin is the offline lever this plan names.

---

## 5) Host / sibling paths (real files, no invent)

### A. Already used (research, not matmul-matched)

| Path | Notes |
|------|--------|
| `…/RyzenAI-SW/…/ryzen14/AMD_AIE2P_*_Overlay*.xclbin` | Default dump; live 283840 B; sha16 `a1e65d04…` |
| Windows CVML ryzen14/15/17 overlays | Same family; not better for Linux pure-direct by default |

### B. **Best present candidates for topology match (priority)**

Under full Ryzen AI install (eepy venv), **flexml** ships Strix xclbins whose names match staged **2x4x4**:

| Absolute path (this host) | Approx size (RECORD) | Why interesting |
|---------------------------|----------------------|-----------------|
| `/home/hunter/Projects/ai/ryzen-ai-install/venv/lib/python3.12/site-packages/flexml/flexml_extras/data/ryzen-ai/stx/unified-2x4x4.xclbin` | ~128891 B | Strix **unified 2x4x4** — closest name match to `mladf_2x4x4` |
| `…/stx/llm-2x4x4.xclbin` | ~115721 B | LLM 2x4x4 overlay; may share PDI family |
| `…/stx/unified-4x4.xclbin` | ~100618 B | Strix 4x4 unified (control / A/B vs 2x4x4) |

**Files confirmed present** via directory listing of that `stx/` folder.

Also (weaker / different class):

| Path | Size | Caution |
|------|------|---------|
| `…/device_essentials_strx…/data/parts/xilinx/xclbin/strx/base.xclbin` | **3395 B** | Tiny; likely stub/base, not full app PDI |
| flexml VE2 `dummy.xclbin` / `unified-1x4x4.xclbin` | multi-MB | **VE2** — refuse as product AIE2 known-good (existing research locks) |
| flexml `aie4/base.xclbin` | small | Not Strix NPU4 product path |

### C. Referenced but **missing** offline (human download / build)

`xdna-driver` shim_test expects (not present under `test/` without `local_shim_test_data`):

- `local_shim_test_data/npu4/gemm/gemm.xclbin` (+ `gemm_int8.elf` partial-ELF flow)
- `local_shim_test_data/npu4/1x4/1x4.xclbin`, `nop/nop.xclbin`, `aie_debug/verify_4x4.xclbin`, …

Download path is vendor `build/build.sh` / VTD bins (see `CMake/native.cmake` note: binaries under `build/amdxdna_bins/`). **Agent must not invent these files.**

### D. Name-only references in Ryzen AI Python (not verified as loose files)

`ryzenai_onnx_utils` strategies mention strings like:

- `/xclbin/stx/llama2_mladf_2x4x4_v2_vision.xclbin`
- `stx_LLM_TPS_mladf_2x4x4_bfp16_gemm_flat_rms_silu_mul`

Those look like **runtime package ids**, not necessarily free-standing paths under the venv. Discovery should prefer **on-disk `.xclbin`** under flexml first, then optional `find` under venv for `*mladf*2x4x4*.xclbin` if any appear after install updates.

### E. Inventory helper already in-tree

`scripts/inventory_txn_sources.sh` (read-only, exit 0) searches RyzenAI-SW, xdna-driver, testdata for `*.xclbin` — useful smoke, **not** a Pass gate. Does **not** currently include `ryzen-ai-install/venv`.

---

## 6) What offline discovery can do vs needs human

### Agent / offline (allowed without live unlock)

1. **Inventory** flexml stx + CVML + any other host xclbins; record path, size, mtime.
2. **Dump PDI** with existing tool (no product edit required):

   ```bash
   bash tools/dump_xclbin_pdi.sh \
     --xclbin /home/hunter/Projects/ai/ryzen-ai-install/venv/lib/python3.12/site-packages/flexml/flexml_extras/data/ryzen-ai/stx/unified-2x4x4.xclbin \
     --out /tmp/dragon_npu_pdi_unified_2x4x4.bin \
     --print-env
   ```

   Repeat for `llm-2x4x4` and optionally `unified-4x4`. Compare `pdi_fp` to CVML `len=283840 sha16=a1e65d04add3422c`.

3. **xclbinutil --info** (if on PATH) for section names / CU count text → soft research only (section map APIs already offline).
4. **Fingerprint table** (len + sha16) into residual/report; soft classify with blob bytes if useful.
5. Optional **named offline product delta** later (not this explore): extend `DEFAULT_XCLBIN_CANDIDATES` / source kind taxonomy for flexml 2x4x4; **only** if operator asks implement.
6. **Do not** claim product known-good, Pass, or C-ready from dump alone.

### Human / gated

| Action | Why |
|--------|-----|
| Mint **new** `C_UNLOCK=` | All prior unlocks CONSUMED; agent must not reopen |
| Live sandwich (`run-stock` / L3) | Needs active unlock; stock may be agent-runnable, hardened L3 needs sudo |
| Provide missing shim_test gemm xclbin | Not on disk; vendor download / EULA |
| Place external known-good **ctrltext** under `testdata/xdna2_cmd/external_known_good/` | Separate residual axis (path B); still ≠ Pass alone |
| Confirm redistribution of flexml xclbins if any copy into repo | Prefer env path outside git; do not commit proprietary blobs |

### Hard stops (do not do)

- Empty reconfirm `/implement` with no new bytes
- Invent PDI / capture bytes
- Second unlock without named offline reason
- Treat CONFIG_CU attach or fingerprint match as Pass
- VE2 xclbin as AIE2 product known-good

---

## 7) Ranked next experiments

### Offline (now — no unlock)

| Rank | Experiment | Success signal | Failure / next |
|------|------------|----------------|----------------|
| **O1** | Dump PDI from `unified-2x4x4.xclbin` | Non-empty PDI; **sha16 ≠** `a1e65d04…` and/or len ≠ 283840 | If dump fails (no PDI section), try multi-section dump research / `xclbinutil --info` |
| **O2** | Dump `llm-2x4x4.xclbin` PDI | Second fingerprint; compare to O1 | Same as above |
| **O3** | Dump `unified-4x4.xclbin` | Topology A/B fingerprint | Document only |
| **O4** | Soft identity notes: path hints → `UnknownOpaque` (or new taxonomy if implement) | Report table of len/sha16/source | Never product_known_good=1 |
| **O5** | Optional: inventory find for `*mladf*2x4x4*.xclbin` under venv + RyzenAI-SW | New on-disk hits | Name-only strings without files → park |

### Live (only after O1+ meaningful new fingerprint + **new unlock**)

| Rank | Experiment | Unlock gate | Success signal (still not Pass) |
|------|------------|-------------|----------------------------------|
| **L1** | One stock CONFIG_CU shot with **O1 blob** as `XDNA2_LAB_PDI_BLOB` | New host-minted unlock; named reason = matmul-topology PDI dump differs from CVML | last_apply: `config_cu=1`, new `pdi_bytes`/`pdi_fp`, journal class change vs orphan fault / WAIT EINVAL |
| **L2** | Skip-PDI A/B (`run-stock-skip-pdi`) | **Separate** new unlock if L1 already consumed | Baseline without CONFIG_CU vs L1 |
| **L3** | Try `llm-2x4x4` PDI if L1 same residual | New unlock each one-shot | Same measurement bar |

**Live recipe shape (after unlock minted in residual):**

```bash
# Offline already produced OUT + fingerprint
export XDNA2_LAB_PDI_BLOB=/tmp/dragon_npu_pdi_unified_2x4x4.bin
export XDNA2_LAB_CU_FUNC=0   # or try values if multi-CU info suggests
# Active C_UNLOCK=… must be on RESIDUAL first line without CONSUMED
bash tools/operator_app_load_kit.sh config-cu-run   # or hitl_config_cu_once run-stock
# Then: collect evidence, mark unlock CONSUMED, never claim Pass without OFM allclose
```

### Parallel residual (not PDI, but related)

| Rank | Experiment | Note |
|------|------------|------|
| **B1** | External known-good **ctrltext** capture into `testdata/xdna2_cmd/external_known_good/` | Construction map path B; soft compare; **match ≠ Pass** |
| **M1** | Driver/MMU map work for orphan `0x2fff8000…` faults | T3b; independent of PDI source |

---

## 8) After discovery: how a better blob is used (and what is still not Pass)

```text
Offline dump (flexml 2x4x4) → fingerprint vs CVML
        │
        ▼
Named offline reason in residual (sha/len differ; topology match)
        │
        ▼
Host mints new C_UNLOCK=… (active one-shot)
        │
        ▼
export XDNA2_LAB_PDI_BLOB=<new dump>
lab: CREATE_HWCTX → CONFIG_CU(opaque PDI) → aiebu START_NPU …
        │
        ▼
last_apply + journal → soft classify / p5 / c3 / c4 / p6
        │
        ├── config_cu failed → fail-closed attach (not Pass)
        ├── config_cu=1 + still wait_cmd_einval / APP_LOAD / fault → wrong PDI still open; or MMU residual
        └── ERT Completed + OFM allclose + real_ioctl → only then product Pass / ran_on_npu
```

**What will NOT get Pass alone**

| Claimed progress | Why not Pass |
|------------------|--------------|
| Found flexml 2x4x4 xclbin on disk | Discovery only |
| Successful `dump_xclbin_pdi` / new sha16 | Opaque research bytes |
| `ResearchPdiIdentity` soft note | Always invent_pass=0 / not product known-good |
| CONFIG_CU attach measured (`config_cu=1`) | Attach ≠ job complete |
| Different residual class vs CVML pin | Progress ≠ OFM allclose |
| Skip-PDI A/B result | A/B only |
| External ctrltext match | ctrltext match alone does not close FW residual (locked) |
| New unlock mint | Unlock ≠ Pass |
| Offline gates green | Never unlock C / never ran_on_npu |

**True Pass still requires** live decide: real ioctl, ERT Completed, non-zero OFM golden allclose (`evaluate_silicon_pass_evidence` / P6 criteria) — unchanged.

---

## 9) Critical files (absolute)

| Path | Why |
|------|-----|
| `/home/hunter/Projects/ai/dragon-npu/tools/dump_xclbin_pdi.sh` | PDI dump + default CVML candidates |
| `/home/hunter/Projects/ai/dragon-npu/tools/hitl_config_cu_once.sh` | prep / skip / run-stock |
| `/home/hunter/Projects/ai/dragon-npu/tools/operator_app_load_kit.sh` | umbrella config-cu-* |
| `/home/hunter/Projects/ai/dragon-npu/src/backends/amd_xdna2/direct.rs` | `try_lab_attach_config_cu_from_env`, timed CONFIG_CU |
| `/home/hunter/Projects/ai/dragon-npu/src/backends/amd_xdna2/silicon_contract.rs` | ResearchPdiIdentity + fingerprints + CONFIG_CU parse |
| `/home/hunter/Projects/ai/dragon-npu/docs/CONSTRUCTION_MAP_APP_LOAD.md` | Public vs lab construction |
| `/home/hunter/Projects/ai/dragon-npu/docs/AMD_MLADF_MATMUL_PACK.md` | 2x4x4 staged key |
| `/home/hunter/Projects/ai/dragon-npu/docs/XDNA2_XRT_CONTRACT_MLADF.md` | Userspace vs FW residual |
| `/home/hunter/Projects/ai/dragon-npu/RESIDUAL.md` | Unlock SSoT |
| `/home/hunter/Projects/ai/ryzen-ai-install/venv/lib/python3.12/site-packages/flexml/flexml_extras/data/ryzen-ai/stx/*.xclbin` | **Primary matmul-topology candidates** |
| `/home/hunter/Projects/ai/RyzenAI-SW/Ryzen-AI-CVML-Library/linux/onnx/ryzen14/AMD_AIE2P_*.xclbin` | Prior research dump sources |
| `/home/hunter/Projects/kernel/xdna-driver/src/shim/kmq/hwctx.cpp` | Public multi-CU PDI → CONFIG_CU |
| `/home/hunter/Projects/kernel/xdna-driver/test/shim_test/dev_info.cpp` | npu4 gemm/nop xclbin paths (data may be missing) |

---

## 10) Recommended immediate offline sequence (agent-doable)

1. Dump O1 `unified-2x4x4` + O2 `llm-2x4x4` (+ optional O3).
2. Write a short fingerprint table (path → pdi len → sha16) vs CVML baseline.
3. If dump succeeds and fingerprints **differ**, that is a **named offline reason** for a future unlock (document in residual when implement/HITL asks).
4. If dump fails (no `PDI` section name), run `xclbinutil --info` and research multi-CU section names; do not invent layout.
5. **Stop** without live shot until unlock is minted; no empty reconfirm mill.

---

## 11) One-line residual for orchestrator

**Matmul-matched PDI discovery:** prefer flexml Strix `unified-2x4x4.xclbin` / `llm-2x4x4.xclbin` (on disk under ryzen-ai-install venv), dump with `dump_xclbin_pdi.sh --xclbin …`, fingerprint offline vs CVML `283840/a1e65d04…`; live only after **new** unlock; still **no** Pass without OFM allclose. Skip inventing bytes; skip reopening `97f38307`.
