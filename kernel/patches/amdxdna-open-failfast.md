# L3 materials: open fail-fast, `exec_cmd_allow`, killable `dev_lock`

**Status:** L3 materials complete — real patch in-tree; **not** installed on host by default  
**Incident context:** 2026-07-13, 1022:17f0 — after `aie2_cmdlist_fill_slot` oops,  
`amdxdna_drm_open` waiters stuck in **D-state**, module refcnt elevated  
**Source base:** [amd/xdna-driver](https://github.com/amd/xdna-driver) @ `905717f5cd470580054a1635b9eea24b84dfd67f`  
**Shippable patch:** [`0002-amdxdna-open-failfast-and-exec-gate.patch`](./0002-amdxdna-open-failfast-and-exec-gate.patch)  
**Stack order:** apply **0001 (L2)** then **0002 (L3)** (files independent; order is policy)  
**Apply/build:** [`README.md`](./README.md)  
**License:** GPL-2.0 (same as driver)

L3 does **not** claim the running laptop already has this module. L1 userspace gates
remain the primary shield until install after clean boot.

## Goals

1. **Fail-fast when wedged (when latched)** — `open` / selected mutating ioctls  
   return **`-EIO`** instead of hanging forever. Auto-arm of the latch is **narrow**  
   (see coverage honesty below).  
2. **Secondary killable wait** on open’s short `dev_lock` critical section (not SVA).  
3. **Module param `exec_cmd_allow`** (lab default **false**) so EXEC_CMD can be  
   refused in-kernel for bring-up kernels — **always on** under L3 default load.

### What is always on vs conditional after a stock L3 install

| L3 piece | Under default module params? | Notes |
|----------|------------------------------|--------|
| `exec_cmd_allow=0` → EXEC_CMD **`-EPERM`** | **Always on** | Lab default; flip with `exec_cmd_allow=1` |
| `mutex_lock_killable(dev_lock)` on open | **Always on** | Narrow; SVA still unkillable |
| Latch infrastructure (`dev_unhealthy`, checks) | **Always present** | Checks no-op while latch is 0 |
| **Automatic** open fail-fast arm (latch → 1) | **Not default** | Only TDR **dump-only** path marks |

## How “wedged / unhealthy” is detected and set

### Latch

| Item | Detail |
|------|--------|
| Field | `atomic_t dev_unhealthy` on `struct amdxdna_dev` (`amdxdna_drm.h`) |
| Init | Zero via `devm_drm_dev_alloc` (healthy) |
| Read | `amdxdna_dev_is_unhealthy(xdna)` |
| Set | `amdxdna_dev_mark_unhealthy(xdna, reason)` — once; logs on first set |
| Clear | `amdxdna_dev_clear_unhealthy(xdna)` — logs if it was set |

There is **no** pre-existing “wedged” flag at this base SHA; L3 introduces this sticky latch.

### Coverage honesty: auto latch is **TDR dump-only only** (non-default)

Stock upstream AIE2 TDR:

| Param | Default | Effect with L3 |
|-------|---------|----------------|
| `timeout_in_sec` | `2` | TDR watchdog runs (unless set to 0) |
| **`tdr_dump_ctx`** | **`false` (0)** | Stuck detect → **full** `stop_all` + `restart_all` → L3 **clears** latch; **never marks** |

So under **default** `modprobe amdxdna` after L3 install:

- Open fail-fast **does not auto-arm** from stuck detect.  
- Periodic TDR still tries full recovery and **clears** any prior latch.  
- Always-on wins remain: **`exec_cmd_allow=0`**, killable open `dev_lock`, fail-fast **checks** ready if something marks.

**To exercise automatic open fail-fast after stuck detect** (lab / debug only):

```bash
# Dump-only TDR: on stuck detect, dump ctx and MARK unhealthy (no restart).
# Do not use as production recovery policy.
modprobe amdxdna tdr_dump_ctx=1
# or at runtime if param is writable: echo 1 > /sys/module/amdxdna/parameters/tdr_dump_ctx
```

There is **no** in-tree L3 sysfs/debugfs “force mark” helper yet; the public
`amdxdna_dev_mark_unhealthy()` is for driver call sites / future debugfs.
Do **not** reintroduce job_run `-ENODEV` device-wide mark (mbox teardown race).

**Optional future design** (not in this patch): mark on TDR *detect* before full
restart, clear only after restart completes — fail-fast during the recovery
window without job_run false wedges.

### Who sets the latch (narrow — post-review)

| Caller | When | Why |
|--------|------|-----|
| `aie2_tdr_force_recover` (`aie2_tdr.c`) | **`dump_only=true`** only — needs **`tdr_dump_ctx=1`** (or a caller that passes dump_only) after stuck detect | Device stuck and we are **not** restarting |
| Explicit future call sites | True device-gone paths / ops force-mark | Helper remains public |

### Who does **not** set the latch

| Path | Why not |
|------|---------|
| `aie2_sched_job_run` on **any** job errno (`-ENODEV`, `-EIO`, `-EINVAL`, …) | **Too broad.** Upstream `aie2_*_execbuf` returns **`-ENODEV` when `mbox_chann` is NULL**. `aie2_unload_hwctx()` **NULLs the channel while the DRM scheduler may still run**. Late jobs after teardown return `-ENODEV` without the device being globally dead. Latching that would brick **all** clients’ `open` / CREATE_HWCTX / EXEC_CMD until TDR full restart (or forever if TDR dump-only / disabled). |
| L2 null-guard returns | L2 returns `-EINVAL` / `-ENOMEM` / `-ENODEV` / `-EOPNOTSUPP` for missing objects — **per-job**, not “device dead.” |
| `amdxdna_pm_resume_get` early fail in job_run | Same narrow policy; do not conflate resume errno with global death without a dedicated device-gone path. |

Comment in `aie2_sched_job_run` documents this deliberately (no mark call).

### Who clears the latch

| Caller | When |
|--------|------|
| `aie2_tdr_force_recover` | After **`aie2_rq_stop_all` + `aie2_rq_restart_all`** (full recover, not dump-only) |

Clear is **best-effort**. If firmware remains dead after restart, TDR may re-fire (dump-only re-latches).

### Scope note (AIE2)

Automatic latch is wired only on the **AIE2 TDR** path (`aie2_tdr.c`). AIE4/other backends do not participate yet (Strix/XDNA2 lab focus). Fail-fast checks on open/ioctls are generic once the latch is set.

### What open / ioctls do when latched

| Entry | Behavior |
|-------|----------|
| `amdxdna_drm_open` | **`-EIO` immediately** before `iommu_sva_bind_device`; re-check after SVA and under `dev_lock` before `list_add` |
| `CREATE_HWCTX` | **`-EIO`** |
| `CONFIG_HWCTX` | **`-EIO`** |
| `EXEC_CMD` | **`-EIO`** (also gated by `exec_cmd_allow`) |
| `SET_STATE` | **`-EIO`** |
| **Allowed** on already-open fd | `DESTROY_HWCTX`, `WAIT_CMD`, `GET_INFO` / `GET_ARRAY`, `CREATE_BO` / BO ops (cleanup + diagnostics) |

## Fail-fast vs killable (priority)

| Mechanism | Role |
|-----------|------|
| **Fail-fast open when latched** | **Primary** mitigation once unhealthy is set |
| **`mutex_lock_killable(dev_lock)`** | **Secondary** — only the short client_list section **after** SVA bind |
| **`iommu_sva_bind_device`** | **Unchanged / unkillable** — dominant hang locus when device is corrupted; no public timeout API |

Open is **not** killable overall. Commit subject leads with fail-fast + `exec_cmd_allow`; killable is a narrow secondary.

## `exec_cmd_allow` module param

| Item | Value |
|------|--------|
| Location | `src/driver/amdxdna/amdxdna_ctx.c` |
| Type | `bool exec_cmd_allow = false`, `module_param(..., 0644)` |
| **Default** | **false (0)** — lab default for this series |
| Enforce | Entire `DRM_IOCTL_AMDXDNA_EXEC_CMD` → **`-EPERM`** if false (**all subtypes**: `EXEC_BUF`, `DEPENDENCY`, `SIGNAL`) |
| Production | `modprobe amdxdna exec_cmd_allow=1` or distro conf; XRT needs allow=1 |
| Upstream note | Mainline PR may prefer default **true**; lab series keeps **false** |

```bash
# Lab default after L3 install (EXEC_CMD off in kernel — all subtypes)
modprobe amdxdna
# or: modprobe amdxdna exec_cmd_allow=0

# Intentional payload / XRT experiment on a lab-patched module (still behind L1):
modprobe amdxdna exec_cmd_allow=1
# plus userspace: feature xdna2-dangerous-exec-cmd + XDNA2_ALLOW_EXEC_CMD=1
```

With L3 default 0, **even a known-good payload cannot be validated** until `exec_cmd_allow=1` — intentional; easy to misread as “driver broken.”

Userspace L1 gates remain mandatory regardless of this param.

## Residual risks (honest)

1. **Already-D tasks** are not woken — reboot / recovery still required.  
2. **`iommu_sva_bind_device` hang** before unhealthy is latched (e.g. first oops with L2 not installed) still possible.  
3. **Mid-open race** shrinks with re-checks after SVA / under `dev_lock`; bind itself still unkillable.  
4. **False clear** after TDR restart that did not truly heal firmware.  
5. **Default `exec_cmd_allow=0`** breaks XRT and blocks known-good payload tests until flipped.  
6. **L3 alone does not latch after oops** (workqueue dies without clean errno). **Stack L2 first** so fill_slot cannot oops.  
7. **Default `tdr_dump_ctx=0` → automatic latch never sets** from stuck detect; open fail-fast is dormant until dump-only mode or an explicit mark. Do not over-expect “fail-fast when wedged” from a stock lab module load.  
8. Automatic latch is **AIE2 TDR dump-only only**; AIE4 not covered.  
9. Does **not** implement full FLR, DRM device wedge events, Phase B payload validation, or debugfs force-mark.

## Relation to L1 / L2

| Layer | Role |
|-------|------|
| **L1** userspace | Feature + env; never accidental EXEC_CMD from dragon-npu |
| **L2** driver | No NULL deref on cmdlist fill/execbuf (**required** so L3 can matter after bad EXEC_CMD) |
| **L3** driver | Fail-fast open/ioctls + lab EXEC_CMD refuse + secondary killable open lock |

**Deliberate submit (lab, after L2+L3 install):** all three:

1. `cargo … --features amd-xdna2,xdna2-dangerous-exec-cmd`  
2. `XDNA2_ALLOW_EXEC_CMD=1`  
3. `modprobe amdxdna exec_cmd_allow=1` (or runtime sysfs/module param write)

## Ship later

1. Install only after **clean boot** (or verified unload) — see README.  
2. Prefer stack **0001 then 0002**.  
3. File upstream after L2 issue; L3 follow-up PR (real DCO).  
4. Optional: flip default `exec_cmd_allow` to true for upstream submission.  
5. Optional: AIE4 TDR/job latch; debugfs force-mark/clear for ops.
