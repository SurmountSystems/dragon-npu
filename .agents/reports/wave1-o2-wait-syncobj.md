# Wave W1 O2 — WAIT EINVAL driver axes + syncobj parity (offline TDD)

**Date:** 2026-08-11
**Mode:** offline only. No live NPU. No unlock mint. No ALLOW=1.
**Honesty:** `invent_pass=0` · **not C-ready** · **not `ran_on_npu`** · Pass = No

**Upstream:** `.agents/reports/wave0-wait-einval-uapi.md` (W0 O1)

---

## Plain English

1. **Driver axis map (pure):** product `wait_cmd_einval` (os EINVAL) can come from
   either missing hwctx **or** missing out-fence/syncobj/seq. True wait budget
   expiry is driver `-ETIME` → product `wait_timeout`, **not** EINVAL. Soft map
   always invent_pass=0; axes alone ≠ Pass.
2. **Syncobj retain (product observability):** `CREATE_HWCTX.syncobj_handle` is
   now stored on `HardwareContext` and printed in last_apply detail. Product
   still uses `WAIT_CMD` only (no live syncobj wait path). XRT-parity wait is
   research sketch only.
3. **Live unlock:** **not justified** from this slice alone (no new material /
   no proven packing root; residual still driver/FW completion).

---

## Code landed?

**Yes** (offline pure + structural product observability).

| Item | Where | Kind |
|------|--------|------|
| `WaitCmdDriverAxis` | `silicon_contract.rs` | pure enum (hwctx / fence·syncobj·seq / ETIME / undifferentiated / other) |
| `WaitSubmitProductClass` | same | product class token |
| `WaitCmdEinvalDriverAxesMap` | same | map bag |
| `map_wait_cmd_einval_driver_axes` | same | pure map status+detail+optional journal |
| `format_wait_cmd_einval_driver_axes_note` | same | soft note invent_pass=0 |
| `wait_message_looks_like_os_einval` / `_true_timeout` | same | pure helpers |
| `journal_hints_invalid_syncobj_or_seq` / `_hwctx_missing` | same | pure journal token helpers |
| `WAIT_CMD_EINVAL_DRIVER_AXES_NOTE` | same | soft bag name |
| Re-exports | `amd_xdna2.rs` | public |
| Soft classify print | `examples/classify_last_apply.rs` | `wait_cmd_einval_driver_axes=` |
| `HardwareContext.syncobj_handle` | `direct.rs` | retain CREATE out-param |
| Real CREATE wires field | `direct.rs` try_create | `syncobj_handle: req.syncobj_handle` |
| Stub records non-zero fake | `StubDirectSubmission` | TDD surface |
| last_apply detail | progressive stamp + wait-fail path | `syncobj=` token |

### Soft honesty tokens (driver map always)

```
invent_pass=0 not_c_ready=1 not_ran_on_npu=1 axes_alone_not_pass=1
hypothesis_not_proven_root=1 product_pass_requires_live_decide=1
wait_cmd_einval_neq_wait_timeout=1 syncobj_wait_parity_alone_not_pass=1
driver_einval_fence_or_hwctx=0|1 etime_is_timeout=0|1
```

### Driver → product class (staged tree)

| Driver condition | Return | Product class |
|------------------|--------|---------------|
| `xa_load` hwctx miss | `-EINVAL` | `wait_cmd_einval` |
| `aie2_cmd_get_out_fence` NULL ("Invalid syncobj or sequence number") | `-EINVAL` | `wait_cmd_einval` |
| `dma_fence_wait_timeout` expiry | `-ETIME` | `wait_timeout` |
| Live skip-CU / flexml without journal split | os error 22 | `os_einval_undifferentiated` |

Live residual under `1f761bf4` maps to **undifferentiated** EINVAL (fence **or**
hwctx) until journal carries a split hint. That is honesty, not a root claim.

---

## Product fix yes/no

| Fix | Done? | Notes |
|-----|-------|-------|
| WAIT packing / ioctl constant | **No change** (already correct per W0 O1) | not the root |
| Classifier false `wait_timeout` | Prior T3 | retained |
| Driver axes soft map | **Yes** | invent_pass=0 |
| Store `syncobj_handle` on context | **Yes** | observability + parity prep |
| Live drm_syncobj wait path | **No** (unsafe without live) | see sketch below |
| invent Pass / ran_on_npu | **No** | forbidden |

### Syncobj wait sketch (research only — not implemented)

XRT shim prefers drm_syncobj timeline wait on CREATE_HWCTX `syncobj_handle`
with EXEC seq as timepoint, falling back to `WAIT_CMD` when syncobj is invalid.
Product could later:

1. Keep recording `syncobj_handle` (landed).
2. Optional env (e.g. research-only) to wait via `DRM_IOCTL_SYNCOBJ_WAIT` /
   timeline wait packing unit-tested offline.
3. Still require ERT Completed + OFM allclose for Pass.

**Do not** treat syncobj wait alone as Pass or as unlock justification.

---

## Unlock justified for live verification?

**No.**

- No new load-accepted material.
- No proven userspace packing root for live `wait_cmd_einval`.
- Syncobj retain ≠ completion path fix ≠ Pass.
- First residual unlock remains **CONSUMED** (`1f761bf4` and priors).
- Do **not** mint; do **not** re-live skip-CU / spent flexml under this report.

---

## TDD

| Test | Intent |
|------|--------|
| `wait_cmd_einval_driver_axes_map_hwctx_vs_fence_vs_etime` | RED→GREEN pure: hwctx vs fence vs ETIME vs undifferentiated; soft invent_pass=0 |
| `create_hwctx_records_syncobj_handle_in_context` | stub CREATE records non-zero `syncobj_handle` |

### Commands (implementer)

```bash
cargo fmt -p dragon-npu
cargo test --features amd-xdna2 --lib wait_cmd_einval_driver_axes
cargo test --features amd-xdna2 --lib create_hwctx_records_syncobj
cargo test --features amd-xdna2 --lib wait_cmd_einval_axes
cargo clippy --features amd-xdna2 --lib -- -D warnings
```

**TDD contract:** red sketch name from W0 O1
`wait_cmd_einval_driver_axes_map_hwctx_vs_fence_vs_etime` landed as unit test;
`create_hwctx_records_syncobj_handle_in_context` covers retain path. Parent or
process mop should re-run the filters above if this implementer shell did not
capture exit codes in-session.

---

## invent_pass

**0** (always on new soft notes and this report).

---

## Explicit non-claims

- Pass = No
- not C-ready
- not `ran_on_npu`
- Axes map alone ≠ proven root cause
- Syncobj field alone ≠ XRT parity wait ≠ Pass
- No live unlock justified
- No re-live under CONSUMED

---

## File map

| Path | Role |
|------|------|
| `src/backends/amd_xdna2/silicon_contract.rs` | driver axis map APIs + unit test |
| `src/backends/amd_xdna2/direct.rs` | `HardwareContext.syncobj_handle` + CREATE + detail |
| `src/backends/amd_xdna2.rs` | re-exports |
| `examples/classify_last_apply.rs` | soft print |
| `.agents/reports/wave0-wait-einval-uapi.md` | O1 parent |
| `.agents/reports/wave1-o2-wait-syncobj.md` | this report |
