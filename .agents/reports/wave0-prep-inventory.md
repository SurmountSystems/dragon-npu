# Wave W0 prep inventory — O6 + O8 + O9 (offline only)

**Date:** 2026-08-11
**Repo:** `/home/hunter/Projects/ai/dragon-npu`
**Unlock:** **none active.** First `C_UNLOCK=` in `RESIDUAL.md` = `1f761bf4` **CONSUMED**.
**Honesty (always):** `invent_pass=0` · `not_c_ready=1` · `not_ran_on_npu=1` · **Pass=No**
**Forbidden this slice:** live ASSIGN debug BO · unlock mint · `ALLOW=1` · invent Pass · re-live spent flexml · force re-dump spent default

---

## O6 — C6 debug-BO offline prep

### Verdict: **ready offline** (prep complete; not live-wired)

| Item | Status | Location |
|------|--------|----------|
| uAPI constants | Present | `DRM_AMDXDNA_HWCTX_ASSIGN_DBG_BUF` (=1) / `REMOVE_DBG_BUF` (=2) in `src/backends/amd_xdna2/direct.rs` |
| AIE2 pack | Present | `pack_config_hwctx_dbg_buf_aie2` / `validate_config_hwctx_dbg_buf_aie2` / `HwctxDbgBufOp` — `param_val` = BO handle **value**, `param_val_size=0` |
| Soft note | Present | `debug_bo_uapi_prep_aie2` / `format_c6_debug_bo_prep_note` / `C6_DEBUG_BO_PREP_NOTE` in `silicon_contract.rs` — hard-codes `invent_pass=0 not_c_ready=1 not_ran_on_npu=1 debug_bo_prep_alone_not_pass=1 research_only=1 live_wired=0` |
| Env intent-only | Present | `ENV_LAB_RESEARCH_DEBUG_BO` = `XDNA2_LAB_RESEARCH_DEBUG_BO`; `lab_research_debug_bo_env_requested` reads env only — **never** issues CONFIG_HWCTX / EXEC |
| Recipe | Present | `docs/HITL_C_APP_LOAD_RECIPE.md` § Optional debug BO offline prep (C6); Step 2 checklist row 3 |
| Construction map | Present | `docs/CONSTRUCTION_MAP_APP_LOAD.md` DBG_BUF row |
| Kit / hitl headers | Present | `tools/operator_app_load_kit.sh`, `tools/hitl_config_cu_once.sh` note intent-only |
| Implement report | Present | `.agents/reports/impl-p2-c6-recipe.md` |
| Unit contracts | Present | `pack_config_hwctx_dbg_buf_*`, `c6_debug_bo_uapi_prep_aie2_contracts`, `lab_research_debug_bo_env_is_intent_only` |

### Gaps

**None for offline prep.** Live ASSIGN/REMOVE is intentionally **not** wired.
**debug-BO prep alone ≠ Pass ≠ C-ready ≠ `ran_on_npu`.**

### Offline verify (when operator/agent has cargo)

```bash
cargo test --features amd-xdna2 --lib debug_bo
# or: pack_config_hwctx_dbg / c6_debug / lab_research_debug
```

---

## O8 — OFM golden / pack contract re-check

### Contract (staged key)

```
mladfmatmulbias_prmptflat_mladf_2x4x4_flat_a16fw4acc16f_1_1152_256_128
M=1 K=1152 N=256 Gs=128
```

| Asset | Expected bytes | Staged paths (present on disk) | Pass-readiness |
|-------|----------------|--------------------------------|----------------|
| IFM | **2304** bf16 | `testdata/xdna2_cmd/silicon_lab/ifm.bin`, `mladf_ref/ifm.bin` | Size contract OK (ones pack) |
| WTS | **174080** nonzero | `silicon_lab/wts.bin` (from `mladf_wts_export/wts.bin`); export PROVENANCE: 149120 nonzero | Length+nonzero gate **documented green** |
| CTRL | **29664** nonzero | `silicon_lab/lab_ctrl.bin` | Length gate documented |
| OFM expected (placeholder) | **512** zeros | `silicon_lab/ofm_expected.bin`, `mladf_ref/ofm_expected.bin` | **Blocked** as Pass golden (all-zero refused) |
| OFM golden (capture) | **512** bf16 nonzero | `silicon_lab/ofm_golden.bin`, `mladf_ofm_capture/ofm_golden.bin`, `gt_capture/ofm_golden.bin` sha256 `8e6b548203bfc086…` | Size+nonzero **Pass-candidate gate** documented; still **≠** product Pass |
| Instr classic | **1688** | `silicon_lab/lab_instr.bin` (+ `.elf` aiebu path) | Userspace contract material |

### Validators (offline scripts present)

| Script | Role |
|--------|------|
| `tools/validate_mladf_ofm_golden.sh` | size bf16 512 / f32 1024 + non-zero; ALLOW_ZERO → exit 3 not 0 |
| `tools/validate_mladf_wts.sh` | exact 174080 + non-zero |
| `tools/validate_mladf_ctrl.sh` | exact 29664 + non-zero |
| `tools/silicon_lab_preflight.sh --require-pass-assets` | fail-closed Pass-asset gate |
| Rust | `validate_mladf_ofm_golden_packed_for_pass`, `validate_mladf_wts_packed_for_pass`, `validate_mladf_ctrl_packed_for_pass` |

### O8 status summary

| Field | Value |
|-------|--------|
| Pack size contract | **Green** (staged IFM/WTS/CTRL/OFM sizes match docs + Rust staged constants) |
| Nonzero WTS | **Green** (export provenance) |
| Nonzero OFM golden | **Green for preflight** (capture golden 512 B, all nonzero bytes per capture PROVENANCE) |
| Product Pass | **No** — pack/OFM green **≠** Pass; needs live ERT Completed + OFM allclose via `decide_ran_on_npu` |
| Residual class | Still post-attach **`wait_cmd_einval`** / EXEC residual — not “missing pack sizes” |

---

## O9 — Unspent PDI inventory

### SSoT spent ledger (`RESIDUAL.md` § Spent vs unspent research PDI)

Research overlays only. invent_pass=0. Re-live of a spent fingerprint alone is not progress.

| tag | pdi_bytes | sha16 (16) | Live status (RESIDUAL SSoT) |
|-----|-----------|------------|------------------------------|
| flexml STX `unified-2x4x4` | 27200 | `0eeeb70c84df056e` | **Spent** under `2ae69036` (`wait_cmd_einval`) |
| flexml STX `llm-2x4x4` | 21664 | `ba0cd738fd870011` | **Spent** under `8352882b` (`wait_cmd_einval` + IO_PAGE_FAULT) |
| flexml STX `unified-4x4` | 13792 | `7d172bf223a9b06b` | **Spent** under `efd44f35` (`wait_cmd_einval` + IO_PAGE_FAULT ~`0x8fff…`) |
| CVML AIE2P 4x4 Overlay | 283840 | `a1e65d04add3422c` | **Spent** prior unlocks (orphan IO_PAGE_FAULT class) |

**All three flexml STX research PDIs are Spent.** There is **no** remaining unspent flexml STX candidate on the residual ledger.

### Tooling notes

| Tool | Role |
|------|------|
| `tools/pass_unblock_fingerprint_once.sh --offline` | Dump/fingerprint + ledger footer; **skips** spent-default re-dump unless `--force` / `XDNA2_FP_FORCE_REDUMP=1` (do **not** force for empty reconfirm) |
| Prior report | `.agents/reports/pass-unblock-fingerprint-once.md` (2026-08-10; ledger **stale** for llm / unified-4x4 — still printed “NOT spent” before later live closeouts) |
| S5 inventory | `.agents/reports/pdi-xclbin-inventory.md` — honest empty count 0 for `*mladf*.xclbin` / `*gemm*.xclbin` under install roots (xclbin-only by design) |

**Drift (honest):** `pass_unblock_fingerprint_once.sh` still hard-codes llm-2x4x4 / unified-4x4 as `UNSPENT_*` and `is_spent_sha16` only marks `0eeeb70c…` + CVML `a1e65d04…`. **RESIDUAL.md wins** for live status. Do not treat script “unspent candidate” labels for the three flexml tags as current truth. Optional mop: align script spent set to residual (not done this inventory).

### Remaining research-only material (not “unspent flexml for re-live”)

| Candidate | Identity | Live? | Notes |
|-----------|----------|-------|-------|
| CVML `AMD_AIE2P_8x4x1_Overlay` | 568224 B sha16 `19c430f04909c280` (prior fingerprint dump) | **Never live** under residual ledger | Research-only DIFF vs CVML 4x4 baseline; **not** product known-good; not flexml residual match |
| Public mlir-aie / IRON npu2 | size-class **`public-npu2-small`**: vector ctrltext 300 B sha16 `aaeb5b68cc298a45`; matmul 660 B `de8671245bb72436`; PDI ~2.5–5.5 KiB under `testdata/xdna2_cmd/external_known_good/` | **Not** residual identity | RESEARCH ONLY; soft mismatch vs spent flexml + lab classic 1688 expected |
| Skip-CONFIG_CU baseline | no PDI | **Spent** under `1f761bf4` | Do not reconfirm under CONSUMED |

### Unspent flexml list (honest)

**Empty** for the three flexml STX research PDIs.
Optional non-flexml research blob never live: CVML 8x4x1 only (not a free unlock reason; needs **new** unlock + **new** named offline reason if ever considered).

---

## Honesty footer

| Token | Value |
|-------|--------|
| invent_pass | **0** |
| not_c_ready | **1** |
| not_ran_on_npu | **1** |
| product Pass | **No** |
| Active unlock | **None** (`1f761bf4` CONSUMED + priors) |
| Live debug BO | **Not** wired; not run |
| Re-live spent flexml | **Banned** |
| Pack green | **≠** Pass |
| Debug-BO prep | **≠** Pass |
| Unspent flexml inventory | **Empty** (all three Spent) |

---

## Return bullets (for parent)

- **debug-BO ready?** Yes offline (constants + pack + soft note + recipe + intent-only env). **Not** live-wired. invent_pass=0.
- **pack/OFM status:** Staged pack sizes green; WTS+ctrl nonzero documented; OFM golden 512 B nonzero Pass-candidate gate; `ofm_expected` zeros blocked. **Pack/OFM green ≠ Pass.**
- **unspent PDI list:** **Empty** for flexml STX (all three Spent + CVML 4x4 Spent). Optional never-live research: CVML 8x4x1; public-npu2-small capture = research only.
- **report path:** `/home/hunter/Projects/ai/dragon-npu/.agents/reports/wave0-prep-inventory.md`
