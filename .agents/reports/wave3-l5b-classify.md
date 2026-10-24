# Wave W3 O7 / O10 / O18 — soft-classify L5b last_apply + detail gaps

**Date:** 2026-08-11
**Scope:** offline soft interpret only. **No live.** **No unlock mint.**
**Honesty:** `invent_pass=0` · **Pass = No** · **not_ran_on_npu=1** · **not_c_ready=1**

---

## Inputs

| Item | Value |
|------|--------|
| last_apply | `/tmp/dragon_npu_last_apply_config_cu_821691.json` (**present**, schema=2) |
| Unlock (historical) | `C_UNLOCK=4008e6fd` **CONSUMED** |
| Live reports | `.agents/reports/live-shot-4008e6fd.md`, `.agents/reports/impl-l5b-debug-bo-dev-closeout.md` |
| Soft tools | p5 / classify / c4 / p6 path (results already recorded in L5b closeout; re-parse below from JSON detail) |

last_apply was **not missing**. Soft classify below is from the JSON body (status + detail) using the same axes the product pure helpers measure (`parse_wait_cmd_einval_axes` / soft classify honesty tokens). Prior L5b closeout already ran p5/c4/p6 on this pin.

---

## Plain English outcome

1. Product status is **`wait_cmd_einval`** after a successful research **DBG_BUF Dev** Assign.
2. **`debug_bo_ty=dev`** and **`attempt_ok=1`** are measured in detail (bo_handle=2, size=4096).
3. EXEC/WAIT still fails: WAIT_CMD **Invalid argument** (os error 22), hwctx=1, **seq=1**, timeout_ms=5000, request=`0xc0106449`.
4. Skip-CONFIG_CU shape (no `config_cu` / no `pdi_bytes`), lab classic **1688** + aiebu + rela×12.
5. **`ran_on_npu=false`**, **`real_ioctl=true`**, **`ert_error=false`**, no `ert_state` token.
6. Soft residual class (from prior p5 + same signals): **`not_pass_no_ert_error`**.
   C4_SKIP / P6_NOT_PASS. **Never** invent Pass from Dev Assign or wait axes.

---

## Top-level last_apply fields (measured)

| Field | Value |
|-------|--------|
| `status` | `wait_cmd_einval` |
| `ran_on_npu` | **false** |
| `real_ioctl` | **true** |
| `aiebu_elf` | true |
| `rela_count` | 12 |
| `ert_error` | false |
| `fw_status_hint` | `n/a` |
| `layout` | count4 |
| `dtype` | mladf_a16w4 |
| `tiles` | 32 |
| `qos_gops` / `qos_fps` | 100 / 30 |
| `total_s` | ≈ **0.203** |
| `schema` | 2 |
| `m,n,k` | 1, 256, 1152 (report dims; caller timed 1,1,1) |

Absent at **top-level** (must mine `detail` string): `config_cu*`, `pdi_bytes`, `cu_func`, `ert_state`, `ofm_*` stats, structured `debug_bo_*`, WAIT seq/request, bo_diag rows.

---

## Detail tokens — debug BO (L5b core)

From `detail` (abbreviated):

```
debug_bo_config op=assign param_type=1 hwctx=1 bo_handle=2 gate_open=1
  attempt_ok=1 path=lab invent_pass=0 not_c_ready=1 not_ran_on_npu=1
  debug_bo_alone_not_pass=1 research_only=1
  debug_bo_size=4096 debug_bo_ty=dev lab_debug_bo_attach=1
… submit_and_wait failed (wait_cmd_einval): WAIT_CMD failed
  (hwctx=1, seq=1, timeout_ms=5000, request=0xc0106449): Invalid argument (os error 22)
```

| Token | Measured |
|-------|----------|
| `debug_bo_ty` | **dev** |
| `attempt_ok` | **1** |
| `debug_bo_size` | 4096 |
| `bo_handle` (DBG) | 2 |
| `lab_debug_bo_attach` | 1 |
| honesty in detail | invent_pass=0 not_c_ready=1 not_ran_on_npu=1 debug_bo_alone_not_pass=1 research_only=1 |

**Research takeaway (not Pass):** Dev Assign succeeded live. Residual class remains post-attach **wait_cmd_einval** (same family as skip-CU `1f761bf4`).

---

## WAIT_CMD EINVAL axes (soft; O10)

Expected soft bag from `parse_wait_cmd_einval_axes` / `format_wait_cmd_einval_axes_note` on this pin:

| Axis | Value |
|------|--------|
| `status_is_wait_cmd_einval` | **1** |
| `invalid_argument` | **1** (os error 22) |
| `config_cu_present` | **0** |
| `skip_config_cu_shape` | **1** |
| `lab_classic_1688` | **1** |
| `userspace_elf_rela_ok` | **1** |
| `aiebu_rela_patched` | **1** |
| `ert_error` | **0** |
| `timeout_ms` | **5000** |
| `hwctx` | **1** |
| `seq` | **1** (see O10 note) |
| `request` | **0xc0106449** |
| `pdi_bytes` | absent |
| `aiebu_elf` | 1 |
| `rela_count` | 12 |
| `content_size` / `start_npu_size` | 1688 / 1688 |
| `ert_state` | **absent** |

**Soft honesty (always):**
`invent_pass=0 not_c_ready=1 not_ran_on_npu=1 axes_alone_not_pass=1 hypothesis_not_proven_root=1 wait_cmd_einval_neq_wait_timeout=1 product_pass_requires_live_decide=1`

### O10 — new last_apply mode vs skip-CU-only

| Axis | skip-CU `1f761bf4` | L5b `4008e6fd` (this pin) |
|------|--------------------|---------------------------|
| status | wait_cmd_einval | wait_cmd_einval |
| skip_config_cu_shape | 1 | 1 |
| lab classic 1688 + aiebu+rela | yes | yes |
| WAIT seq | **0** (fixture lock) | **1** |
| debug BO | none | **Dev Assign ok** |
| wall | ~0.17 s | ~0.20 s |
| residual class | wait_cmd_einval | wait_cmd_einval (unchanged for Pass) |

**O10 apply:** this is a **new mode** relative to pure skip-CU: **research debug-BO Dev attached**, then same WAIT EINVAL class. Axes helpers already cover WAIT fields; they do **not** yet fold `debug_bo_ty` / `attempt_ok` into `WaitCmdEinvalAxes` (gap → O7 instrument list). Do **not** re-spin skip-CU-only under CONSUMED unlocks.

---

## Soft residual / C4 / P6 (from L5b closeout + same signals)

| Check | Result |
|-------|--------|
| Residual class (soft) | **`not_pass_no_ert_error`** |
| CONFIG_CU checklist | `config_cu_present=0` (skip path) |
| C3 | evidence vs locked APP_LOAD baseline may flag new status shape; **≠** Pass |
| C4 | **C4_SKIP** (`json_ran_on_npu=false`) |
| P6 | **P6_NOT_PASS** / not product Pass |
| invent_pass | **0** |
| product Pass | **No** |
| ran_on_npu | **false** |

Prior closeout line:
`Soft interpret: residual_class=not_pass_no_ert_error; wait_cmd_einval lab classic axes (skip_cu + 1688 + aiebu+rela); C4_SKIP; P6_NOT_PASS. invent_pass=0.`

---

## O7 — richest last_apply / journal detail gaps

Gaps that would help offline residual dig **without inventing Pass**. Prefer instrument first if product code changes; this report is gap inventory only (no product edit this slice).

### A. Top-level schema gaps (must parse free-text `detail` today)

| Missing top-level / structured field | Why it would help O7 |
|--------------------------------------|----------------------|
| `config_cu_present` / `config_cu` / `pdi_bytes` / `cu_func` | Soft checklist and skip-vs-attach without regex on detail |
| `ert_state` | Distinguish pure WAIT fail vs ERT Error path |
| `wait_hwctx` / `wait_seq` / `wait_timeout_ms` / `wait_request` | Axes without re-parsing WAIT_CMD prose |
| `debug_bo_ty` / `debug_bo_attempt_ok` / `debug_bo_handle` / `debug_bo_size` | L5b mode visible in schema, not only detail |
| `ofm_raw_nonzero` / ofm first non-zero / max_abs_diff | True Pass path signals (expect absent on this fail) |
| `exec_cmd_allow` / L3 snapshot at shot | Stock vs L3 sandwich honesty |
| journal FW codes embedded | Dual residual APP_LOAD vs wait only when codes present |

### B. Detail / bo_diag gaps on **this** pin

| Gap | Observation |
|-----|-------------|
| DBG BO not in `bo_diag=[…]` | detail has bo_handle=2 for DBG Assign; bo_diag lists DevHeap=1, instr=3, ifm/wts/ofm/ctrl — **no row for debug BO** map/xdna_addr |
| `ert_state` | absent (WAIT fails before / without ERT state read) |
| `fw_status_hint` | always `n/a` on this pin (no journal-backed FW status in last_apply) |
| Journal not in JSON | L5b window (live report): no Get-bo-failed, no IO_PAGE_FAULT, no aie2_set_cmd_timeout — must stay out-of-band unless collect_post_c2 embeds |
| `config_cu=` token | absent by design (skip path); soft tools infer `config_cu_present=0` |

### C. Axes helper gaps (O10 fold)

| Gap | Note |
|-----|------|
| `WaitCmdEinvalAxes` has no `debug_bo_*` | L5b mode only visible via detail substring; optional extend soft note with `debug_bo_ty=` / `attempt_ok=` when present — **research only**, still invent_pass=0 |
| seq=1 vs fixture seq=0 | Documented here; fixture TDD remains skip-CU shape; do not rewrite green tests to force seq=1 |

### D. Explicit non-instrument for empty reconfirm

Do **not** mint stamps only to re-print the same gap list. Instrument only if operator asks for named product detail tokens / TDD.

---

## Honesty tokens (return bag)

```
status=wait_cmd_einval
debug_bo_ty=dev
attempt_ok=1
lab_debug_bo_attach=1
debug_bo_size=4096
debug_bo_handle=2
skip_config_cu_shape=1
config_cu_present=0
lab_classic_1688=1
userspace_elf_rela_ok=1
aiebu_elf=1
rela_count=12
wait_hwctx=1
wait_seq=1
wait_timeout_ms=5000
wait_request=0xc0106449
invalid_argument=1
ert_state=absent
ert_error=0
real_ioctl=true
ran_on_npu=false
residual_class=not_pass_no_ert_error
c4=C4_SKIP
p6=P6_NOT_PASS
invent_pass=0
not_c_ready=1
not_ran_on_npu=1
axes_alone_not_pass=1
debug_bo_alone_not_pass=1
product_pass_requires_live_decide=1
Pass=No
unlock=4008e6fd_CONSUMED
```

---

## Explicit non-claims

- **Pass = No**
- **invent_pass = 0**
- **ran_on_npu = false**
- Dev DBG_BUF Assign ok **≠** Pass
- wait_cmd_einval **≠** Pass
- Axes / soft classify **≠** proven FW root cause
- No unlock mint / no live / no sticky ALLOW
- Do **not** reopen `4008e6fd` or re-live Shmem `48da6e60` / spent flexml for this classify

---

## Ranked next (agent-legal offline)

1. Optional O7 product: structured last_apply / soft note fields for `debug_bo_*` + DBG row in bo_diag (TDD first) — only if operator wants code.
2. Material residual-matched / load-accepted path remains highest Pass-critical offline work.
3. Live only under **new** unlock + named reason (not this classify).

---

## Report path

`.agents/reports/wave3-l5b-classify.md`
