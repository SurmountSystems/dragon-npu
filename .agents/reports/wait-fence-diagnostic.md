# WAIT fence diagnostic (plan S3, offline)

**Date:** 2026-08-12
**Mode:** offline only. No live NPU. No unlock mint. No L3 / ALLOW.
**Honesty:** invent_pass=0 · not_c_ready=1 · not_ran_on_npu=1 ·
axes_alone_not_pass=1 · live_justified=0 · Pass=**No**
**This diagnostic is not an unlock reason.**

---

## Plain English

After EXEC returns a sequence number, product wait uses
`DRM_IOCTL_AMDXDNA_WAIT_CMD` (`0xc0106449`) and live last_apply is still
**`wait_cmd_einval`** (os error 22) almost immediately. That is not a hang,
not a timeout, and not Pass.

The staged driver has only two EINVAL wait paths: missing hardware context
(`xa_load` miss) or a NULL out-fence (`aie2_cmd_get_out_fence`). True wait
expiry is `-ETIME` and product already maps that to `wait_timeout`. First
job `seq=0` is legal. The ioctl request/size already matches the kernel
IOWR pack.

Best current story: the driver cannot resolve an out-fence for
`(hwctx, seq)`, or firmware never published a waitable fence. XRT prefers
a drm_syncobj timeline wait when CREATE_HWCTX returned a non-zero handle;
product default is still WAIT_CMD. That last point is a **parity gap**, not
a proven root.

Reuse: `WaitCmdEinvalAxes`, `map_wait_cmd_einval_driver_axes`,
`parse_post_fail_ert_research`. New tokens only name the ranked story.

---

## Ranked hypotheses

Live shape (skip-CU `1f761bf4`, same-shot consumed shot, L5b after Dev
DBG_BUF): status `wait_cmd_einval`, `hwctx=1`, `seq=0` (or `1` on L5b),
`timeout_ms=5000`, `request=0xc0106449`, wall ≈0.16–0.20 s, product
`wait_path=wait_cmd`, CREATE often greps `syncobj=1`.

| Rank | Token | What it means | Evidence | Status |
|------|-------|---------------|----------|--------|
| **1** | `out_fence_lookup_fail` | Driver cannot resolve out-fence for `(hwctx, seq)` | Staged `aie2_cmd_wait` returns `-EINVAL` when `aie2_cmd_get_out_fence` is NULL and prints “Invalid syncobj or sequence number”. Immediate fail, not hang. Product `WaitCmdDriverAxis::is_product_einval()`. | **Primary.** Best staged-driver match. Silent journal stays `os_einval_undifferentiated` (fence vs hwctx not split). |
| **2** | `unpublished_job` | Firmware / scheduler never published a waitable fence | Fence is published in `aie2_cmd_submit` via `drm_syncobj_add_point` after `job->seq = ctx->submitted++`. If that never happens (or is torn down), wait sees NULL. Compatible with rank 1. Closed when post-fail ERT is Error or Completed (job existed enough to reject or finish). | **Open** on historical last_apply with no ERT token. Next justified live can split via `ert_post_fail=`. |
| **3** | `syncobj_parity_unproven` | Product WAIT_CMD vs XRT syncobj timeline when handle ≠ 0 | Shim `hwq.cpp` uses `wait_cmd_syncobj` when `get_syncobj() != AMDXDNA_INVALID_FENCE_HANDLE`, else WAIT_CMD. Product retains the handle on `HardwareContext` and still defaults `submit_and_wait_command` to WAIT_CMD. Env `XDNA2_LAB_WAIT_VIA_SYNCOBJ` is research-only. | **Parity gap.** Not a proven residual root. Live A/B **not** justified. |
| 0 | `not_wait_fence_class` | Not this residual | Product `wait_timeout` or other class. | Timeout is a different class, already ruled out as this residual. |

Lower ranks from prior digs (not new API variants):

| Rank | Hypothesis | Status |
|------|------------|--------|
| 4 | Destroyed / wrong hwctx (`xa_load` miss) | Unlikely on stock timed path (wait before destroy; live `hwctx=1`). Same EINVAL class. |
| 5 | `timeout_ms=5000` too short | **Ruled out.** Driver timeout is `-ETIME` → `wait_timeout`. Shot wall 0.16 s. |
| 6 | `seq=0` illegal | **Ruled out.** First job is 0 (`submitted++`). Historical EXEC+WAIT seq=0 accepted. |
| 7 | Wrong ioctl / struct size | **Ruled out.** Product pack is IOWR size 16 → `0xc0106449`. Historical WAIT accepted on same pack. |

**Not named:** “command already done.” A completed job should still have a
fence. EINVAL is lookup fail, not “already signaled.”

---

## Code cites (verified this slice)

### Staged driver (ground truth for EINVAL vs ETIME)

`kernel/dkms/staging/amdxdna-hardened-0.1.0/src/driver/amdxdna/aie2_ctx.c`

- `aie2_cmd_submit` 882–969: `job->seq = ctx->submitted++` at **942**;
  `drm_syncobj_add_point(ctx->syncobj, chain, job->out_fence, *seq)` at
  **948**. That is the publish step rank 2 talks about.
- `aie2_cmd_get_out_fence` 971–989: `drm_syncobj_fence_get` then
  `dma_fence_chain_find_seqno`. NULL fence or failed lookup → NULL return.
- `aie2_cmd_wait` 991–1013: NULL fence → `XDNA_ERR(…, "Invalid syncobj or
  sequence number")` and **`-EINVAL`** at 997–999. Expiry is
  `dma_fence_wait_timeout` → **`-ETIME`** at 1005–1007.

`…/amdxdna_ctx.c`

- `amdxdna_cmd_wait` 792–817: `xa_load(&client->ctx_xa, ctx_hdl)` miss →
  **`-EINVAL`** at 804–808.
- `amdxdna_drm_wait_cmd_ioctl` 819–836: copies `hwctx` / `seq` / `timeout`
  and calls `amdxdna_cmd_wait`.

### Staged XRT shim (parity only)

`…/src/shim/hwq.cpp` `hwq::wait_command` **109–139**: if
`m_ctx->get_syncobj() != AMDXDNA_INVALID_FENCE_HANDLE` then
`drv_ioctl(wait_cmd_syncobj)`, else `wait_cmd_ioctl` (WAIT_CMD).

### Product wait (read-only this slice)

`src/backends/amd_xdna2/direct.rs`

- `amdxdna_drm_wait_cmd` **92–109** (`hwctx`, `timeout`, `seq`).
- `DRM_IOCTL_AMDXDNA_WAIT_CMD` **574–593** = `0xc010_6449` (IOWR, size 16).
- CREATE retains `syncobj_handle` **1819–1833**; field docs **2976–2986**.
- `wait_cmd_ioctl` **2852–2872** (product default WAIT_CMD; greps
  `wait_path=wait_cmd`, `request=0xc0106449`).
- `LabWaitPath` / `choose_lab_wait_path` **4356–4410**: env off → WAIT_CMD.
- `submit_and_wait_command` **4492–4514**: always WAIT_CMD after EXEC seq.
- `submit_and_wait_command_ex` **4517–4544**: research env only.

### Pure diagnostic (this slice)

`src/backends/amd_xdna2/silicon_contract.rs`

- Reused: `WaitCmdEinvalAxes` **12304**, `parse_wait_cmd_einval_axes`
  **12359**, `WaitCmdDriverAxis::is_product_einval` **12591–12598**,
  `map_wait_cmd_einval_driver_axes` **12713**,
  `parse_post_fail_ert_research` **13170**.
- New: `WaitFenceHypothesis` **13315–13351**, `WaitFenceDiagnostic`
  **13358–13387**, `parse_wait_fence_diagnostic` **13404–13467**,
  `format_wait_fence_diagnostic_note` **13476–13525** (honesty
  **hard-coded**).

On `wait_cmd_einval`, primary is always rank 1
`OutFenceLookupFail`. Rank 2/3 are **open flags**, not a second primary.
Format always emits `invent_pass=0 not_c_ready=1 not_ran_on_npu=1
axes_alone_not_pass=1 live_justified=0 diagnostic_alone_not_unlock=1`.

---

## What this does not do

- Does **not** prove a single residual root.
- Does **not** justify live syncobj A/B or a new unlock.
- Does **not** change product wait (`direct.rs` untouched).
- Does **not** invent Pass or `ran_on_npu`.

Primary wall for product Pass remains residual-matched / load-accepted
material plus a completing wait. This write-up only names the wait-fence
story as testable tokens.
