# Implement report: post-timeout offline evidence + honesty

**Date:** 2026-08-10
**Plan:** `.agents/plans/plan-post-timeout-offline.md` (approved)
**Scope:** Offline only. No sudo, no L3, no `XDNA2_ALLOW_EXEC_CMD=1` in tests, no live EXEC, no reopen `C_UNLOCK=3a70fabf`.

## Elevator result

Progressive last_apply mid-path stamps, missing-pin honesty, journal soft taxonomy, outer-budget guidance, and residual SSoT for **CONSUMED** unlock all landed. **No** product Pass. **No** `ran_on_npu`. **No** new unlock minted.

---

## Deliverables

### 1. Progressive last_apply (`timed_matmul_direct`)

**Files:** `src/backends/amd_xdna2/direct.rs`, `src/backends/amd_xdna2/silicon_contract.rs`

| Stage | Status token | When |
|-------|--------------|------|
| After CREATE_HWCTX OK | `lab_partial_hwctx_ok` | `write_la_default` (LAST_APPLY or LAB_OFM) |
| After CONFIG_CU attach OK | `lab_partial_config_cu` | same; fail path already stamped `config_cu_failed` |
| Immediately before `submit_and_wait` | `lab_partial_pre_submit` | `write_la` |

- Reuses existing `stamp` → `finalize_lab_timed_result` / `write_last_apply_*` (overwrite same pin).
- Code comment documents: kill **during** a single ioctl can still lose that stage; progressive stamps only help when Rust regains control between stages.
- Detail always includes `invent_pass=0 not_ran_on_npu=1 not_c_ready=1` via `format_lab_partial_detail`.

**Pure helpers**

- `LAB_PARTIAL_STATUS_HWCTX_OK` / `_CONFIG_CU` / `_PRE_SUBMIT`
- `lab_partial_honesty_tokens()`, `is_lab_partial_progressive_status()`, `format_lab_partial_detail()`

### 2. Missing-pin honesty

**Files:** `tools/collect_post_c2_evidence.sh`, `tools/p5_post_c2_interpret.sh`

| Case | Behavior | Exit |
|------|----------|------|
| Explicit path not a file | Plain English: matches outer timeout kill (rc=124 → lab exit 4); incomplete; not APP_LOAD invent | **2** |
| No path → repo fixture | **LOUD: BASELINE FIXTURE ONLY** (2026-07-18 APP_LOAD; not this shot) | 0 soft |
| p5 unlock note | **`C_UNLOCK=3a70fabf` CONSUMED (not active)** | — |

Smoke:

```
bash tools/collect_post_c2_evidence.sh /tmp/missing.json  → collect_missing_rc=2
bash tools/p5_post_c2_interpret.sh /tmp/missing.json      → p5_missing_rc=2
bash tools/collect_post_c2_evidence.sh                    → LOUD baseline + exit 0
```

### 3. Journal soft taxonomy

**File:** `src/backends/amd_xdna2/silicon_contract.rs`

- `JournalSoftTaxonomy` + `classify_journal_soft_signals(text)`
- Signals: incomplete outer kill (`rc=124`, timeout kill language), `IO_PAGE_FAULT`, `aie2_set_cmd_timeout`
- Soft class token: `incomplete_timeout_or_journal` (`JOURNAL_SOFT_INCOMPLETE_CLASS`)
- `format_soft_note()` always: `invent_pass=0 not_c_ready=1 not_ran_on_npu=1 product_pass_requires_live_decide=1`
- **Never** maps journal-only → PassCandidate or locked APP_LOAD class

### 4. Outer budget + recipe hygiene

| File | Change |
|------|--------|
| `scripts/lab_exec_once.sh` | Help: CONFIG_CU prefer ≥120s; **loud WARN** when silicon-mladf or PDI blob with outer ≤30s; WAIT stays finite |
| `tools/hitl_config_cu_once.sh` | Sandwich print: `--timeout-s=120`; unlock **CONSUMED**; only after new unlock |
| `docs/HITL_C_APP_LOAD_RECIPE.md` | Unlock CONSUMED; timeout hygiene; no “active 3a70fabf” |
| `docs/XDNA2_EXEC_CMD_LAB.md` | Outer ≥120 for CONFIG_CU; progressive stamp note |

### 5. RESIDUAL.md SSoT

- Unlock **`C_UNLOCK=3a70fabf` CONSUMED** (top + mid tables)
- Who acts next: human optional after **new** unlock + ≥120s; agent offline interpret only after real pin
- Progressive last_apply marked landed offline
- **No** Pass / **no** `ran_on_npu`

---

## TDD evidence (red → green)

| Test | Command | Result |
|------|---------|--------|
| `progressive_last_apply_partial_status_honesty` | `cargo test --features amd-xdna2 --lib progressive_last_apply` | **ok** |
| `journal_soft_taxonomy_timeout_and_page_fault` | `cargo test --features amd-xdna2 --lib journal_soft_taxonomy` | **ok** |
| config_cu suite (7) | `cargo test --features amd-xdna2 --lib config_cu` | **7 ok** |
| `write_last_apply_best_effort_tmp` | `cargo test --features amd-xdna2 --lib write_last_apply` | **ok** |

Contract covered by progressive test: partial status names, honesty trio in detail + JSON (`ran_on_npu: false`), progressive ≠ locked APP_LOAD class.

Red phase: tests written to assert pure helpers + write path before/with implementation (observed green after wire-up; pure contracts exercise helpers independently of live path).

---

## Gates

| Check | Result |
|-------|--------|
| `cargo fmt -p dragon-npu` | clean |
| `cargo clippy -p dragon-npu --features amd-xdna2 --lib -- -D warnings` | **exit 0** |
| `cargo clippy … --all-targets -D warnings` | pre-existing noise in unrelated tests/examples (not introduced here); lib target is the package gate used historically for this work |
| `bash -n` collect / p5 / hitl_config_cu_once / lab_exec_once | **BASH_N_OK** |
| Live EXEC / L3 / ALLOW=1 | **not run** |

---

## Honesty

- invent_pass=0, not_c_ready=1, not_ran_on_npu=1 on partial and journal soft paths
- Unlock **not** reopened; no new unlock minted
- Product Pass still false; criteria offline ≠ silicon Pass
- Empty full-gate reconfirm was **not** the deliverable

## Operator next (human)

1. Optional: new host-minted unlock + named reason “progressive last_apply + timeout honesty landed”
2. Sandwich with **`--timeout-s=120`** (WAIT finite)
3. Share real last_apply pin → agent offline p5/c3/c4/p6 only
4. **No** reopen under `3a70fabf`
