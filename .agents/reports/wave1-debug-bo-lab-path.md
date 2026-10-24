# Wave1: research debug-BO lab timed path (fail-closed)

**Date:** 2026-08-11
**Repo:** `/home/hunter/Projects/ai/dragon-npu`
**Slice:** optional research debug-BO attach in `timed_matmul_direct` (offline TDD)
**Unlock:** **none** (no mint; do not reopen CONSUMED stamps)
**invent_pass:** **0** always
**Live NPU apply this slice:** **no**

---

## 1. What landed

### Lab attach API

| Item | Location | Behavior |
|------|----------|----------|
| `LAB_RESEARCH_DEBUG_BO_SIZE_BYTES` | `direct.rs` | **4096** (one host page). Research default; public uAPI is handle-only. |
| `LabResearchDebugBoAttach` | `direct.rs` | `bo_handle` + `size_bytes` + `hwctx`; `detail_token()` always invent_pass=0 |
| `LabResearchDebugBoGuard` | `direct.rs` | On drop: best-effort `REMOVE_DBG_BUF` then GEM_CLOSE |
| `try_lab_attach_research_debug_bo_from_env` | `direct.rs` | Env **unset** → `Ok(None)`; set → CREATE Shmem 4096 + Assign (fail-closed) |

### Timed path wire

After optional CONFIG_CU attach in `timed_matmul_direct`:

1. Call `try_lab_attach_research_debug_bo_from_env`.
2. On success: stamp detail (`debug_bo_config` / `lab_debug_bo_attach=1`); hold guard for lifetime.
3. On fail (env set): status `debug_bo_failed`, destroy hwctx, return (fail-closed).
4. Env unset: empty guard; **zero behavior change** vs prior default tests.

Detail merge via `diag_debug_bo` in the existing stamp closure (alongside CONFIG_CU).

### Size / type choice (documented)

| Choice | Rationale |
|--------|-----------|
| **4096 B** | Minimal page; enough to exercise CREATE + ASSIGN/REMOVE |
| **`BufferType::Shmem`** | Host-mappable research default |
| **Uncertainty** | Live FW may prefer Dev or another size; uAPI does not encode size in CONFIG_HWCTX. Tune under a **new** unlock if needed. |

---

## 2. Tests (TDD contracts)

| Test | Contract |
|------|----------|
| `try_lab_attach_research_debug_bo_unset_is_none` | Env unset → None; timed_matmul no DBG_BUF records |
| `try_lab_attach_research_debug_bo_stub_records_assign` | Env set → Assign recorded; guard drop → Remove |
| `timed_matmul_lab_research_debug_bo_records_assign_on_stub` | Full timed path Assign+Remove; invent_pass=0; no ran_on_npu |
| `lab_research_debug_bo_attach_detail_token_honesty` | Soft tokens locked |
| Prior `config_hwctx_dbg_buf_*` | Still green (gate + stub) |

Commands (exit 0 this turn):

```bash
cargo test --features amd-xdna2 --lib debug_bo
# 6 passed

cargo test --features amd-xdna2 --lib config_hwctx_dbg
# 5 passed

cargo fmt -p dragon-npu
cargo clippy -p dragon-npu --features amd-xdna2 --lib -- -D warnings
# exit 0
```

---

## 3. Docs

| Path | Update |
|------|--------|
| `docs/HITL_C_APP_LOAD_RECIPE.md` | Lab path + size; Step 2 optional env creates BO |
| `docs/CONSTRUCTION_MAP_APP_LOAD.md` | DBG_BUF glossary + kit comment |
| `tools/operator_app_load_kit.sh` | Overview |
| `tools/hitl_config_cu_once.sh` | Header note |

Re-exports: `LAB_RESEARCH_DEBUG_BO_SIZE_BYTES`, `LabResearchDebugBoAttach`,
`LabResearchDebugBoGuard`, `try_lab_attach_research_debug_bo_from_env` on
`amd_xdna2`.

---

## 4. Unlock justified for L5?

**No mint from this slice.**

A **future** L5 one-shot unlock (operator-hosted) could cite:

1. Lab path now CREATE+Assign when `XDNA2_LAB_RESEARCH_DEBUG_BO=1` (unit-tested).
2. Residual still post-attach `wait_cmd_einval` / spent flexml history.
3. Optional debug BO may enrich journal/FW visibility (**hypothesis only**).

Do **not** mint from this report alone. Unlock ≠ Pass ≠ `ran_on_npu`.

---

## 5. Remaining live L5 checklist

When a **new** host-minted unlock exists:

1. Active unlock only; outer timeout ≥120s; restore L3 after.
2. Prep CONFIG_CU or skip-CU as usual (not spent flexml re-live alone).
3. `export XDNA2_LAB_RESEARCH_DEBUG_BO=1` (lab auto CREATE 4096 Shmem + Assign).
4. Live apply under sandwich; pin last_apply + journal.
5. If FW rejects size/type: research alternate size or Dev BO (named offline delta first).
6. Optional: confirm REMOVE in detail/records; never claim Pass from assign alone.
7. Mark unlock CONSUMED.

Still need: active unlock, ALLOW/L3 only under recipe, real device evidence.

---

## 6. Honesty footer

- invent_pass=0
- not_c_ready
- not_ran_on_npu
- debug_bo_alone_not_pass=1
- unlock_justified=0
- live_apply=0
- lab_path_wired=1
- env_unset_zero_change=1
