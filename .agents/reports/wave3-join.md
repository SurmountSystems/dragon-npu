# Wave W3 join (O12) — offline only

**Date:** 2026-08-11
**Scope:** Join of W3 offline fan-out reports. **No live.** **No unlock mint.**
**Honesty:** invent_pass=**0** · Pass=**No** · not_c_ready=1 · not_ran_on_npu=1
**Active unlock:** **none** (first `C_UNLOCK=4008e6fd` **CONSUMED** — do not reopen)

Joined reports (read-only; no re-research):

| Report | Role |
|--------|------|
| `.agents/reports/impl-s5-plan-mirror-l5b-pin.md` | Plan mirror after L5b; unlock still CONSUMED |
| `.agents/reports/wave3-material-hunt.md` | Material avenues (M-hunt) |
| `.agents/reports/wave3-syncobj-wait-offline.md` | O13 pure syncobj wait decision helpers |
| `.agents/reports/wave3-l5b-classify.md` | Soft classify L5b last_apply |
| `.agents/reports/wave3-unspent-pdi.md` | Non-flexml unspent PDI inventory |
| `.agents/reports/live-shot-4008e6fd.md` | L5b live closeout (prior; closed) |

---

## Plain English

Wave 3 was **offline only**. Four workstreams ran in parallel:

1. **Material hunt** — still the **highest** Pass lever and still **open**. Capture dir has public mlir-aie npu2 AOT research peels only (`public-npu2-small` + whole_array research). Cascade AOT **not built**. FastFlowLM / Lemonade **absent** on host. No residual-matched / load-accepted working-stack ctrltext from a job that hit ERT Completed + OFM allclose on this firmware.

2. **Syncobj wait (O13)** — pure decision helpers **landed offline** and are **not live-wired**. Product still always uses WAIT_CMD. XRT prefers `DRM_IOCTL_SYNCOBJ_TIMELINE_WAIT` when `syncobj_handle ≠ 0`. That is a **parity gap**, not proven root of live `wait_cmd_einval`, and **not** unlock or Pass alone.

3. **L5b soft classify** — last_apply present; status remains **`wait_cmd_einval`** after research **DBG_BUF Dev** Assign **ok**. Soft residual **`not_pass_no_ert_error`**; C4_SKIP; P6_NOT_PASS. Dev Assign ok ≠ Pass.

4. **Unspent PDI scan** — **4** non-flexml unspent PDI fingerprints listed (CVML 8x4x1 + 3 public mlir-aie). Unspent flexml STX = **0** (all three Spent). **L4 is not open** from this scan alone (needs named offline reason + new host-minted unlock).

**No** product Pass. **No** `ran_on_npu`. **No** unlock minted. Re-live under CONSUMED unlocks is **frozen**.

---

## Honesty tokens

```
invent_pass=0
product_pass=0
ran_on_npu=0
not_c_ready=1
active_unlock=none
C_UNLOCK=4008e6fd CONSUMED
residual_class=wait_cmd_einval
syncobj_wait_helpers_not_live_wired=1
l4_not_open_from_unspent_scan_alone=1
material_still_highest_gap=1
```

---

## O13 — syncobj wait path (offline)

| Item | State |
|------|--------|
| APIs | `should_wait_via_syncobj` · `select_wait_completion_path` · `pack_syncobj_timeline_wait_args` · soft note |
| Live ioctl | **Not wired** (`wait_command` still WAIT_CMD only) |
| XRT parity | Prefer timeline wait when handle ≠ 0; else WAIT_CMD |
| Alone unlock? | **No** |
| Alone Pass? | **No** |

Soft note always: invent_pass=0 not_c_ready=1 not_ran_on_npu=1 syncobj_wait_parity_alone_not_pass=1 not_live_wired=1.

**L8 note:** any future live syncobj wait A/B needs (1) more offline packing/timeout tests if not already enough, (2) product live wire under **new** unlock, (3) still ERT Completed + OFM allclose for Pass. L8 is **not** open from pure helpers alone.

Report: `.agents/reports/wave3-syncobj-wait-offline.md`

---

## Material hunt (M)

| Avenue | Result |
|--------|--------|
| Capture dir | Not empty; public AOT RESEARCH ONLY (vector 300 B, matmul 660 B, whole_array 2592 B ctrltext) |
| Residual match / load-accept | **Missing** (no ERT Completed + OFM allclose capture) |
| Cascade matmul AOT | **Not built** (source only under mlir-aie cascade/) |
| FastFlowLM / Lemonade | **Absent** under `~/Projects/ai` |
| Soft vs lab 1688 / spent flexml | **Mismatch expected** |

**Highest remaining gap for Pass:** residual-matched / load-accepted material.

Report: `.agents/reports/wave3-material-hunt.md`

---

## L5b soft classify (closed live; offline interpret)

| Field | Value |
|-------|--------|
| last_apply | `/tmp/dragon_npu_last_apply_config_cu_821691.json` |
| status | `wait_cmd_einval` |
| debug_bo | **Dev** Assign ok (`attempt_ok=1`, bo=2, size=4096) |
| config_cu | absent (skip path) |
| soft class | `not_pass_no_ert_error` |
| C4 / P6 | C4_SKIP / P6_NOT_PASS |
| seq | **1** (vs skip-CU fixture seq=0) |

**L5 / L5b closed.** Do not re-live under `4008e6fd` or `48da6e60`.

Report: `.agents/reports/wave3-l5b-classify.md` · live: `.agents/reports/live-shot-4008e6fd.md`

---

## Unspent non-flexml PDI (inventory only)

| Material | pdi_bytes | sha16 | class |
|----------|-----------|-------|-------|
| CVML AIE2P 8x4x1 Overlay | 568224 | `19c430f04909c280` | **unspent** (never live) |
| mlir-aie vector pdi | 2528 | `fe438059f5d0e4eb` | unspent RESEARCH ONLY |
| mlir-aie single_core matmul pdi | 5536 | `cd3d9fa6577d1e37` | unspent RESEARCH ONLY |
| mlir-aie whole_array matmul pdi | 66864 | `e721b80335e6d8ed` | unspent RESEARCH ONLY |

Spent flexml STX (re-live banned alone): `0eeeb70c…` / `ba0cd738…` / `7d172bf2…`.
Spent CVML 4x4: `a1e65d04…`.

**L4 from scan alone:** **No.** Unspent list ≠ named offline reason ≠ unlock mint ≠ Pass. Public PDI remains residual-mismatched research.

Report: `.agents/reports/wave3-unspent-pdi.md`

---

## Live queue honesty (post-W3)

| Slot | State |
|------|--------|
| **L1** | Still **blocked** (no residual-matched / load-accepted material that alone unlocks a justified live stock path) |
| **L4** | **Not open** from unspent PDI list alone |
| **L5 / L5b** | **Closed** (`48da6e60` Shmem fail; `4008e6fd` Dev ok then wait_cmd_einval) |
| **L8** | Needs **live wire** of syncobj wait **first**, only under a **new** unlock after more offline justification if any; pure helpers ≠ L8 open |
| Empty reconfirm / reopen CONSUMED / invent Pass / re-live spent flexml | **Frozen** |

---

## Explicit non-claims

- invent_pass=0; Pass=**No**; no `ran_on_npu`
- Syncobj pure helpers ≠ Pass ≠ unlock alone ≠ live fix
- Unspent PDI inventory ≠ residual match ≠ L4 open ≠ unlock mint
- Public AOT / cascade-not-built / FastFlowLM absent ≠ invent residual match
- Dev DBG_BUF Assign ok ≠ Pass; wait_cmd_einval ≠ Pass
- No unlock minted this join; no live apply

---

## Who acts next

1. **Highest:** residual-matched / load-accepted program material (human capture or build path that can complete on this firmware, then ingest under `testdata/xdna2_cmd/external_known_good/` with honest PROVENANCE).
2. **Optional offline:** further WAIT EINVAL dig with DBG_BUF context (instrument gaps only); optional cascade AOT if new size/sha (still RESEARCH ONLY).
3. **Optional later live (not now):** under **new** host-minted unlock + named reason only — e.g. unspent research blob isolation and/or env-gated syncobj wait A/B after live wire. Not justified from W3 join alone.
4. **Frozen:** re-live CONSUMED `4008e6fd` / `48da6e60` / `1f761bf4` / spent flexml; invent Pass; sticky ALLOW; empty reconfirm mill.

---

## Report paths

| Path |
|------|
| `.agents/reports/wave3-join.md` (this file) |
| `.agents/reports/wave3-material-hunt.md` |
| `.agents/reports/wave3-syncobj-wait-offline.md` |
| `.agents/reports/wave3-l5b-classify.md` |
| `.agents/reports/wave3-unspent-pdi.md` |
| `.agents/reports/impl-s5-plan-mirror-l5b-pin.md` |
| `RESIDUAL.md` top · `AGENTS.md` Latest Turn |
