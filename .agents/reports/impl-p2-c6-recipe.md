# Implement report: P2 C6 + Step 2 recipe checklist (Pass-unblock offline)

**Date:** 2026-08-10
**Repo:** `/home/hunter/Projects/ai/dragon-npu`
**Slice:** plan step 3 P2 only (debug-BO uAPI prep + operator Step 2 checklist)
**Unlock:** **none** (no mint; `C_UNLOCK=2ae69036` and priors remain CONSUMED)

---

## 1. What landed

### C6 — Debug BO uAPI prep (research / offline only)

| Item | Location |
|------|----------|
| `DRM_AMDXDNA_HWCTX_ASSIGN_DBG_BUF` / `REMOVE_DBG_BUF` | already present; docs expanded (AIE2 value-mode) |
| `HwctxDbgBufOp` | `Assign` / `Remove` + `param_type` / `as_str` / `from_param_type` |
| `HwctxDbgBufAie2Parsed` | hwctx + op + bo_handle |
| `pack_config_hwctx_dbg_buf_aie2` | pure: `param_val = bo_handle as u64`, `param_val_size=0`, `pad=0`; refuse `bo_handle=0` |
| `validate_config_hwctx_dbg_buf_aie2` | pure layout: only assign/remove, pad/size 0, u32 BO, non-zero |
| `format_debug_bo_aie2_pack_detail` | soft detail token (`live_wired=0 invent_pass=0 research_only=1`) |
| `ENV_LAB_RESEARCH_DEBUG_BO` | `XDNA2_LAB_RESEARCH_DEBUG_BO` — **intent name only** |
| `lab_research_debug_bo_env_requested` | reads env; **never** issues CONFIG_HWCTX / EXEC |
| Soft C6 | `C6_DEBUG_BO_PREP_NOTE`, `DebugBoUapiPrep`, `debug_bo_uapi_prep_aie2`, `format_c6_debug_bo_prep_note` in `silicon_contract.rs` |

**AIE2/Strix fact (public driver + XRT shim):** for ASSIGN/REMOVE_DBG_BUF,
`param_val` is the GEM BO handle as a **value** (not a pointer). CONFIG_CU remains
pointer-to-buffer mode. No proprietary firmware protocol invented.

**Not landed (on purpose):** live `config_hwctx` for debug BO; EXEC without double
gates; auto-attach when env set.

### Step 2 recipe checklist

| Path | Change |
|------|--------|
| `docs/HITL_C_APP_LOAD_RECIPE.md` | New § **Step 2 prep checklist** — preconditions, CONSUMED list, minimal/unspent load first, optional debug BO table, operator sequence (prep → live under unlock → restore L3 → post-evidence → p5), honesty bans |
| `tools/operator_app_load_kit.sh` | Overview points at Step 2 + C6 research env honesty |
| `docs/CONSTRUCTION_MAP_APP_LOAD.md` | DBG_BUF glossary row → C6 pack + soft note + env intent-only |

### Residual / plan / AGENTS

| File | Pin |
|------|-----|
| `RESIDUAL.md` | Plain status: P2 landed; unlock none; next = human material / gated live |
| `AGENTS.md` | Latest Turn P2 honesty pin |
| `.agents/plans/plan-pass-unblock-sequence.md` | Progress board **P2 Done** |
| Session `plan.md` | Same board Done |

### Re-exports

Public `pub use` in `src/backends/amd_xdna2.rs` for debug-BO pack types/fns,
`ENV_LAB_RESEARCH_DEBUG_BO`, and C6 soft APIs.

### Tests

| Test name | Contract |
|-----------|----------|
| `pack_config_hwctx_dbg_buf_aie2_assign_remove_layout` | assign/remove pack + validate + detail honesty |
| `pack_config_hwctx_dbg_buf_aie2_refuses_zero_bo` | fail-closed zero handle |
| `validate_config_hwctx_dbg_buf_aie2_refuses_bad_layout` | wrong type / size / pad / wide val |
| `lab_research_debug_bo_env_is_intent_only` | env read only under lock |
| `c6_debug_bo_uapi_prep_aie2_contracts` | soft note tokens; zero-BO layout_ok=0; invent_pass=0 |

---

## 2. Red → green evidence

| Phase | Evidence |
|-------|----------|
| **Red (clippy)** | `doc_lazy_continuation` on pack doc list line; `cast_possible_truncation` on `param_val as u32`. |
| **Green** | Bullet list + blank line in docs; `u32::try_from(param_val)`; same tests pass. |

---

## 3. Commands + exit codes

```text
cargo fmt -p dragon-npu
  → exit 0

cargo clippy --features amd-xdna2 --lib -- -D warnings
  → exit 0

cargo test --features amd-xdna2 --lib pack_config_hwctx_dbg_buf
  → 2 passed, exit 0

cargo test --features amd-xdna2 --lib c6_debug
  → 1 passed, exit 0

cargo test --features amd-xdna2 --lib lab_research_debug
  → 1 passed, exit 0

cargo test --features amd-xdna2 --lib validate_config_hwctx_dbg
  → 1 passed, exit 0

cargo test --features amd-xdna2 --lib config_cu_uapi
  → 1 passed, exit 0

bash -n tools/operator_app_load_kit.sh
  → exit 0
```

---

## 4. Honesty

| Token / claim | Value |
|---------------|--------|
| invent_pass | **0** (hard-coded on C6 soft notes + pack detail) |
| not_c_ready | **1** on C6 soft notes |
| not_ran_on_npu | **1** on C6 soft notes |
| debug_bo_prep_alone_not_pass | **1** |
| research_only | **1** |
| live_wired | **0** (offline prep only) |
| Active unlock | **None** — not minted |
| Product Pass / `ran_on_npu` | **No** |
| Capture bins | Still may be **0** (not invented) |

---

## 5. Remains (not this slice)

| Who | What |
|-----|------|
| **Human / host** | Step 1 material into `testdata/xdna2_cmd/external_known_good/` + PROVENANCE |
| **Gated live** | New `C_UNLOCK` + named reason; follow Step 2 checklist (minimal / unspent first; optional debug BO when live-wired later) |
| **Re-interpret** | Only after new last_apply / journal / capture bytes (`p5` / c3 / c4 / p6) |
| **Not agent now** | Empty reconfirm mill; reopen CONSUMED unlocks; invent Pass; live wire of `XDNA2_LAB_RESEARCH_DEBUG_BO` without unlock + named product slice |

**Sequence offline C* P0–P2: done.** Next agent work only if named offline delta or evidence arrives.
