# Wave1: research debug-BO CONFIG_HWCTX wire (fail-closed)

**Date:** 2026-08-11
**Repo:** `/home/hunter/Projects/ai/dragon-npu`
**Slice:** O6 → live-capable research path offline (L5 recipe non-fiction)
**Unlock:** **none** (no mint; do not reopen CONSUMED stamps)
**invent_pass:** **0** always
**Live NPU apply this slice:** **no**

---

## 1. What was wired

### Trait + gate (fail-closed)

| Item | Location | Behavior |
|------|----------|----------|
| `ENV_LAB_RESEARCH_DEBUG_BO` | `direct.rs` | Research **gate** for DBG_BUF CONFIG_HWCTX (was intent-only docs) |
| `research_debug_bo_config_is_enabled` | `direct.rs` | True iff env set non-empty after trim |
| `research_debug_bo_config_block_reason` | `direct.rs` | Stable refuse message (mentions invent_pass=0 / not Pass) |
| `DirectSubmission::config_hwctx_dbg_buf` | trait default + Real + Stub + SharedReal | Assign/remove debug BO via AIE2 value-mode pack |
| Real path | `RealDirectSubmission` | Gate → pack → validate → `DRM_IOCTL_AMDXDNA_CONFIG_HWCTX` |
| Stub path | `StubDirectSubmission` | Gate → pack → validate → record `StubDbgBufOpRecord` (no ioctl) |
| `StubDbgBufOpRecord` + `dbg_buf_op_records()` | stub observability | Unit tests prove assign/remove recorded when gate open |
| `format_research_debug_bo_config_detail` | soft wire token | Always `invent_pass=0 not_c_ready=1 not_ran_on_npu=1 debug_bo_alone_not_pass=1 research_only=1` |

### Re-exports

`src/backends/amd_xdna2.rs` exports: gate helpers, format detail, `StubDbgBufOpRecord`.

### Docs (honesty)

- `docs/HITL_C_APP_LOAD_RECIPE.md` — research gate + wire soft; L5 still needs unlock + real BO
- `docs/CONSTRUCTION_MAP_APP_LOAD.md` — DBG_BUF row + kit comment
- `tools/operator_app_load_kit.sh` / `tools/hitl_config_cu_once.sh` — gate wording

### Still **not** live / not auto

| Still open | Why |
|------------|-----|
| Auto attach in `timed_matmul` / lab EXEC after CONFIG_CU | Intentionally omitted (no silent DBG_BUF) |
| CREATE debug BO + fill + assign in lab sandwich | Needs L5 unlock + BO size/type recipe |
| Live ioctl under operator sandwich | No active `C_UNLOCK`; this slice offline only |
| Pass / `ran_on_npu` | Debug-BO alone never Pass |

---

## 2. Gate model (double-gate spirit)

| Layer | Rule |
|-------|------|
| Default tests | Env **unset** → trait **refuses** (no ioctl, no stub record) |
| Research open | `XDNA2_LAB_RESEARCH_DEBUG_BO=1` → Stub records; Real may ioctl |
| Not EXEC_CMD | Does **not** require `XDNA2_ALLOW_EXEC_CMD` or dangerous-exec feature |
| Product honesty | Env set alone ≠ Pass ≠ C-ready ≠ unlock ≠ `ran_on_npu` |

CONFIG_CU remains separate (`XDNA2_LAB_PDI_BLOB`). DBG_BUF is opt-in research only.

---

## 3. Tests (TDD contracts)

| Test | Contract |
|------|----------|
| `pack_config_hwctx_dbg_buf_aie2_*` (existing) | Pure pack/validate |
| `lab_research_debug_bo_env_is_intent_only` | Env read + now asserts gate mirrors env |
| **`config_hwctx_dbg_buf_refuses_when_research_env_unset`** | Stub + Real refuse; no records; soft detail honesty |
| **`config_hwctx_dbg_buf_stub_records_when_research_env_set`** | Assign+remove recorded; zero BO still refused; soft path=stub |

Filter:

```bash
cargo test --features amd-xdna2 --lib debug_bo
# also matches:
cargo test --features amd-xdna2 --lib config_hwctx_dbg
```

Post-edit mop (operator or process mop):

```bash
cargo fmt -p dragon-npu
cargo clippy -p dragon-npu --features amd-xdna2 --all-targets -- -D warnings
cargo test --features amd-xdna2 --lib debug_bo
```

**Note:** this implementer turn could not execute cargo in-process (no shell tool in subagent tool list). Parent/process mop should run the three commands and fix any fmt/clippy fallout.

---

## 4. Unlock justified?

**No.** This slice does **not** justify a new `C_UNLOCK` mint.

Reasons for a **future** L5 unlock (when operator wants one live shot) could cite:

1. Research-gated `config_hwctx_dbg_buf` path exists and unit-tested fail-closed.
2. Named residual still post-attach `wait_cmd_einval` / prior spent flexml history.
3. Optional debug BO may enrich residual (journal/FW visibility) — **hypothesis only**, not Pass lever.

Do **not** mint from this report alone.

---

## 5. Remaining live enable checklist (L5 recipe)

When a **new** host-minted unlock exists and operator wants one research shot:

1. Active unlock only; outer timeout ≥120s; restore L3 after.
2. Prep CONFIG_CU (or skip-CU isolation) as usual.
3. Create a real debug BO (size/type TBD from public XRT/`config_ctx_debug_bo` research).
4. `export XDNA2_LAB_RESEARCH_DEBUG_BO=1`
5. Call `config_hwctx_dbg_buf(..., Assign, bo)` after CREATE_HWCTX (and after CONFIG_CU if used).
6. Run EXEC as usual; pin last_apply + journal.
7. Optional REMOVE before destroy; never claim Pass from assign alone.
8. Mark unlock CONSUMED; soft note path with invent_pass=0.

**Not done this slice:** lab auto-wire of steps 3–5 into `lab_exec_once` / timed path.

---

## 6. Honesty footer

- invent_pass=0
- not_c_ready
- not_ran_on_npu
- debug_bo_alone_not_pass=1
- unlock_justified=0
- live_apply=0
