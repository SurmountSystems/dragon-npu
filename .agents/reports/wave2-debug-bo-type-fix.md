# Wave2: research debug-BO type/size fix (offline)

**Date:** 2026-08-11
**Repo:** `/home/hunter/Projects/ai/dragon-npu`
**Slice:** post-L5 offline fix after live `48da6e60` (`debug_bo_failed`)
**Live this slice:** **no**
**invent_pass:** **0** always
**Pass:** **No**

---

## 1. Live residual (spent recipe)

| Item | Value |
|------|--------|
| Unlock | `C_UNLOCK=48da6e60` **CONSUMED** |
| status | `debug_bo_failed` |
| Lab BO | handle=2, size=**4096**, type=**Shmem** |
| Journal | `aie2_hwctx_cfg_debug_bo: Get bo 2 failed` |
| Userspace | Assign EINVAL; fail-closed before EXEC |
| invent_pass | 0 |

Do **not** re-live Shmem-4096 under `48da6e60`.

---

## 2. Research finding (public driver + XRT)

### Driver (root cause of `Get bo N failed`)

Public AIE2 path (`aie2_hwctx_cfg_debug_bo` in xdna-driver
`drivers/accel/amdxdna/aie2_ctx.c`):

```c
abo = amdxdna_gem_get_obj(client, bo_hdl, AMDXDNA_BO_DEV);
if (!abo) {
    XDNA_ERR(xdna, "Get bo %d failed", bo_hdl);
    return -EINVAL;
}
```

`amdxdna_gem_get_obj` returns NULL unless the GEM’s type equals the requested
type (`AMDXDNA_BO_DEV`). A **Shmem** BO therefore fails exactly as measured
live (`Get bo 2 failed` / EINVAL). This is a **type filter**, not “wrong size
alone.”

Same pattern in newer `aie2_ctx_attach_debug_bo` (still `AMDXDNA_BO_DEV`).

### XRT parity

| Source | Behavior |
|--------|----------|
| Shim test `TEST_create_free_debug_bo` | sizes `{ 0x1000 }` and `{ 0x100000 }` |
| Flags | `XRT_BO_FLAGS_CACHEABLE` + `XRT_BO_USE_DEBUG` |
| `bo_flags_to_type` | CACHEABLE + has_dev_mem → **`AMDXDNA_BO_DEV`** |
| `dbg_buffer::bind_hwctx` | `config_ctx_debug_bo` Assign after create |

Size **4096** remains a valid research page (matches shim positive test).
**Type must be Dev**, not Shmem.

### Lab precondition

CREATE_HWCTX path already calls `ensure_dev_heap`, so post-hwctx
`CREATE_BO(Dev)` is the same class of allocation used for instruction / PDI
Dev BOs.

---

## 3. Product fix landed?

**Yes (offline).** Lab research attach now uses XRT/driver-parity type.

| Item | Before (spent live) | After (this slice) |
|------|---------------------|--------------------|
| CREATE type | `BufferType::Shmem` | **`BufferType::Dev`** (`LAB_RESEARCH_DEBUG_BO_TYPE`) |
| CREATE size | 4096 | **4096** (unchanged; XRT `0x1000`) |
| Soft detail | `debug_bo_ty=shmem` | **`debug_bo_ty=dev`** + size + invent_pass=0 |
| Fail path error | size only | size + **ty=** token |

### Code / API

| Symbol | Role |
|--------|------|
| `LAB_RESEARCH_DEBUG_BO_TYPE` | const `BufferType::Dev` |
| `LAB_RESEARCH_DEBUG_BO_SIZE_BYTES` | 4096 (docs cite driver/XRT) |
| `lab_research_debug_bo_type_token` | soft `dev` / `shmem` / … |
| `lab_research_debug_bo_create_recipe` | pure `(Dev, 4096)` |
| `LabResearchDebugBoAttach.bo_type` | recorded on attach |
| `try_lab_attach_research_debug_bo_from_env` | CREATE Dev + Assign |

Re-exports on `amd_xdna2`. Docs/kit/lab_exec comments updated (recipe,
construction map, operator kit, lab_exec forward comment).

---

## 4. Tests

```text
cargo fmt -p dragon-npu
  → exit 0

cargo test --features amd-xdna2 --lib debug_bo
  → 7 passed (includes lab_research_debug_bo_create_recipe_is_dev_page)

cargo test --features amd-xdna2 --lib config_hwctx_dbg
  → 5 passed

cargo test --features amd-xdna2 --lib research_debug_bo
  → 6 passed

cargo clippy -p dragon-npu --features amd-xdna2 --lib -- -D warnings
  → exit 0

bash -n tools/operator_app_load_kit.sh scripts/lab_exec_once.sh
  → exit 0
```

Contracts: env unset → None; env set → Assign Dev + detail `debug_bo_ty=dev`;
guard Remove; invent_pass=0; recipe ≠ Shmem.

---

## 5. Unlock justified for L5b re-try?

**Yes, as a named offline reason for a future host-minted unlock** (parent
decides mint + live). This report does **not** mint and does **not** live.

Suggested unlock reason text (when parent wants one shot):

> Offline fix after CONSUMED `48da6e60`: lab research DBG_BUF CREATE type
> corrected from Shmem to **Dev** (`AMDXDNA_BO_DEV`) to match
> `aie2_hwctx_cfg_debug_bo` type filter and XRT CACHEABLE+USE_DEBUG; size
> remains 4096. Unit tests green. One stock research debug-BO shot
> (`XDNA2_LAB_RESEARCH_DEBUG_BO=1`, skip-CONFIG_CU or other named arm) to
> measure Assign under Dev.

| Gate | Verdict |
|------|---------|
| Named offline delta | **Yes** (type fix + TDD) |
| Mint from this report alone | **No** (parent coordinates) |
| Reopen `48da6e60` | **Forbidden** |
| Re-live Shmem-4096 | **Forbidden** |
| Assign success alone = Pass | **No** |
| invent_pass | **0** |
| Highest residual for product Pass | Still load-accepted / residual-matched material |

---

## 6. Honesty footer

- invent_pass=0
- not_c_ready
- not_ran_on_npu
- debug_bo_alone_not_pass=1
- product_fix_landed=1 (CREATE type Dev)
- unlock_minted_this_slice=0
- live_apply=0
- unlock_justified_for_parent_l5b=1 (named reason only)

### Artifacts

| Path | Role |
|------|------|
| `src/backends/amd_xdna2/direct.rs` | type/size recipe + attach |
| `src/backends/amd_xdna2.rs` | re-exports |
| `docs/HITL_C_APP_LOAD_RECIPE.md` | Step 2 DBG_BUF table |
| `docs/CONSTRUCTION_MAP_APP_LOAD.md` | glossary + skip env note |
| `tools/operator_app_load_kit.sh` / `scripts/lab_exec_once.sh` | comments |
| `.agents/reports/live-shot-48da6e60.md` | spent live evidence |
| This file | offline fix report |
