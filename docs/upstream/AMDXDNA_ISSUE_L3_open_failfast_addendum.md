# Upstream issue addendum — L3 open fail-fast / exec_cmd_allow (do **not** auto-post)

Paste this as a **comment** on the L2 issue (or a short follow-up issue) after
the cmdlist null-guard discussion. Base tree same as L2:
`905717f5cd470580054a1635b9eea24b84dfd67f`.

Do **not** auto-file on GitHub. Rewrite DCO identity before any PR.

---

## Title (if separate issue)

AIE2 / amdxdna: fail-fast open when device unhealthy; lab `exec_cmd_allow` gate

## Body / comment

### Summary (follow-up to fill_slot NULL + open D-state)

After a job-path failure (or TDR stuck detect without full recovery), new
`open("/dev/accel/…")` callers can hang in **D-state** inside
`amdxdna_drm_open` (often around `iommu_sva_bind_device` or long waits), raising
module **refcnt** and blocking unload. L2 null-guards stop the specific
`aie2_cmdlist_fill_slot` oops; **L3** is a separate hardening layer so a device
that becomes **device-level** unhealthy **fails fast** instead of accumulating
unkillable openers.

### Proposed L3 (minimal)

1. **`atomic_t dev_unhealthy` on `struct amdxdna_dev`**
   - Auto-set on **TDR dump-only** stuck path only; **clear** after full TDR
     stop/restart. Stock **`tdr_dump_ctx` defaults false**, so the normal TDR
     path full-restarts and **never marks** — open fail-fast **does not auto-arm**
     under default module params. Lab/debug: `tdr_dump_ctx=1` to latch after
     stuck detect without restart. Always-on under defaults: `exec_cmd_allow=0`,
     killable open `dev_lock`, latch infrastructure.
   - **Do not** set from ordinary `aie2_sched_job_run` `-ENODEV`/`-EIO`:
     `aie2_*_execbuf` returns `-ENODEV` when per-ctx `mbox_chann` is NULL, and
     `aie2_unload_hwctx()` NULLs the channel while the DRM scheduler may still
     run — latching that would false-wedge all clients after a normal teardown.
   - While latched: `amdxdna_drm_open`, `CREATE_HWCTX`, `CONFIG_HWCTX`,
     `EXEC_CMD`, `SET_STATE` return **`-EIO`**. Leave DESTROY / WAIT / GET_INFO /
     BO for cleanup on already-open fds.
   - Open check runs **before** `iommu_sva_bind_device`, with re-checks after SVA
     and under `dev_lock` before `list_add`.

2. **Secondary: `mutex_lock_killable(&xdna->dev_lock)` in open**
   - Only the short client_list critical section **after** SVA bind.
   - **Honest residual:** `iommu_sva_bind_device` has no killable/timeout API;
     open is **not** killable overall. Fail-fast when latched is the primary win.

3. **Module param `exec_cmd_allow` (lab series default 0 / false)**
   - Refuses the **entire** `DRM_IOCTL_AMDXDNA_EXEC_CMD` ioctl (EXEC_BUF,
     DEPENDENCY, SIGNAL) with **`-EPERM`** when `exec_cmd_allow=0`.
   - Production/XRT: `modprobe amdxdna exec_cmd_allow=1`.
   - Upstream may prefer default **1**; lab series uses **0**.

4. **AIE2 scope:** automatic latch is on the AIE2 TDR path; AIE4 not covered yet.

5. **Stack L2 first:** L3 alone does **not** latch after a hard oops (no clean
   errno from a dead workqueue). Null-guards prevent the fill_slot oops.

### Concrete patch

Out-of-tree unified diff (same base SHA as L2 materials):

- `kernel/patches/0002-amdxdna-open-failfast-and-exec-gate.patch` (dragon-npu)
- Design notes: `kernel/patches/amdxdna-open-failfast.md`
- Apply after L2: `0001` then `0002` (`kernel/patches/README.md`)

### What L3 does **not** fix

- Tasks **already** in D-state.
- Hang **inside** `iommu_sva_bind_device` when unhealthy was never latched
  (e.g. hard oops with L2 not installed).
- Full FLR / DRM device-wedge protocol / Phase B cmd payload validation.

### Request

1. Interest in a fail-fast latch + open `-EIO` pattern (or preference for an
   existing DRM wedge / TDR path we should hook instead).
2. Guidance on default for `exec_cmd_allow` if accepted upstream (0 vs 1).
3. Whether IOMMU/SVA killable bind is on any roadmap (separate from amdxdna).

---

**Repo note:** Paste-ready only. Human files/comments on GitHub when ready.
