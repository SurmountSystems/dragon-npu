# Upstream issue body — combined L2 + L3 (paste once — do **not** auto-post)

Single paste-ready issue covering **both** the AIE2 cmdlist NULL-guard fix (L2)
and open fail-fast / lab `exec_cmd_allow` (L3). Prefer one issue with two
patches over two disconnected threads.

**When filing:** attach
`docs/upstream/evidence/ATTACH_fill_slot_oops_20260713_220051.txt`
(and optionally the 03:28 capture). Soften “always repro” language (lab
scaffold EXEC_CMD, not XRT). Rewrite any in-tree `Signed-off-by` / `From` to a
**real DCO identity** before a PR.

Source materials:

- L2 detail: [`AMDXDNA_ISSUE_cmdlist_null_open_wedge.md`](./AMDXDNA_ISSUE_cmdlist_null_open_wedge.md)
- L3 detail: [`AMDXDNA_ISSUE_L3_open_failfast_addendum.md`](./AMDXDNA_ISSUE_L3_open_failfast_addendum.md)
- Patches: `kernel/patches/0001-…`, `0002-…` @ base
  `905717f5cd470580054a1635b9eea24b84dfd67f`

Agents must **not** open the GitHub issue unless a human explicitly asks.

---

## Title

AIE2: cmdlist NULL deref wedges open; fail-fast unhealthy + lab exec_cmd_allow

## Body

### Summary

On Ryzen AI 300 / Strix-class NPU (**PCI 1022:17f0**), an incomplete / scaffold
`DRM_IOCTL_AMDXDNA_EXEC_CMD` cmdlist payload was **observed** (2026-07-13 lab)
to trigger a **NULL pointer dereference** in `aie2_cmdlist_fill_slot`. After the
oops, `open("/dev/accel/accel0")` callers hung in **D-state** inside
`amdxdna_drm_open`, module **refcnt** stayed elevated, and `rmmod` / PCI remove
hung. Recovery effectively required reboot.

Even if userspace is wrong, the driver should:

1. **(L2)** Return `-EINVAL` / `-ENODEV` / `-ENOMEM` / `-EOPNOTSUPP` instead of
   NULL-deref on bad/empty cmdlist fill/execbuf paths.
2. **(L3)** When the device is marked **device-level unhealthy**, fail open and
   selected job ioctls with **`-EIO`** instead of accumulating unkillable openers;
   optionally refuse all EXEC_CMD in lab with `exec_cmd_allow=0`.

We stack **L2 first**: L3 alone does not latch after a hard oops (no clean errno
from a dead workqueue).

### Environment

| Item | Value |
|------|--------|
| Device | AMD XDNA2 NPU, **PCI ID 1022:17f0** (ThinkPad P14s Gen 6 / Ryzen AI 7 PRO 350 lab) |
| Kernel | **7.1.3-zen1-2-zen** (Garuda / Arch; stock in-tree amdxdna + lab OOT rebuild) |
| Firmware | **1.1.2.64** via `amdnpu/17f0_10/npu_7.sbin` (linux-firmware); OOT tree requests `npu.dev.sbin` (aliased in lab) |
| Driver base for patches | [amd/xdna-driver](https://github.com/amd/xdna-driver) @ `905717f5cd470580054a1635b9eea24b84dfd67f` (**OOT** `src/driver/amdxdna/`) |
| Userspace | Out-of-tree lightweight DRM client (not XRT), research EXEC_CMD scaffold that triggered the 2026-07-13 oops |
| Post-patch lab | Same host runs OOT + L2+L3 with `exec_cmd_allow=0`; open/GET_INFO/CREATE_HWCTX smoke OK without EXEC_CMD |

### Symptoms

1. **Oops** (captured **twice** on 2026-07-13, journal still retained):  
   `BUG: kernel NULL pointer dereference, address: 0000000000000050`  
   RIP: `aie2_cmdlist_fill_slot+0x26/0x130 [amdxdna]`  
   Workqueue: `amdxdna_js` / `drm_sched_run_job_work`  
   Call chain: `aie2_cmdlist_single_execbuf` → `aie2_sched_job_run` → …  
   **RAX=0**, CR2=`0x50` — faulting insn loads `0x50(%rax)` after loading a
   field from `cmd_abo` (matches NULL `cmd_abo->client` then `client->…` at +0x50).
2. Afterward, **`amdxdna_drm_open`** waiters in **D-state**.
3. High **module refcnt** → cannot `rmmod`.
4. PCI remove/unbind of the NPU function may hang; parent GPP bridge may expose
   sysfs `reset` while the NPU function often has none.

### Proposed fix — L2 (null-guards)

In `src/driver/amdxdna/aie2_message.c`, defensive checks in:

- `aie2_cmdlist_fill_slot`
- `aie2_cmdlist_single_execbuf`
- `aie2_cmdlist_multi_execbuf`

including guards before `aie2_cmdlist_get_cmd_buf` and a cmd-BO vmap probe before
`amdxdna_cmd_get_op`.

Unified diff (out of tree, same base SHA):  
`0001-amdxdna-cmdlist-null-guards.patch`

Related upstream (not a full substitute): #1347, PR #1441 (aie4), PR #1468,
historical PR #598. We did not find an open issue matching **AIE2 fill_slot oops
+ post-oops open D-state** specifically (~2026-07).

### Proposed fix — L3 (fail-fast + lab gate)

Minimal hardening on the same tree:

1. **`atomic_t dev_unhealthy`** on `struct amdxdna_dev`  
   - Auto-**set** only on **TDR dump-only** stuck path; **clear** after full TDR
     restart. Stock **`tdr_dump_ctx=0`** means TDR full-restarts and **never
     marks** — open fail-fast **does not auto-arm** under default params.
     Lab/debug: `tdr_dump_ctx=1`.  
   - **Do not** mark from ordinary `aie2_sched_job_run` `-ENODEV`/`-EIO` (mbox
     teardown race / false wedge).  
   - While latched: open, `CREATE_HWCTX`, `CONFIG_HWCTX`, `EXEC_CMD`, `SET_STATE`
     → **`-EIO`**. Leave DESTROY / WAIT / GET_INFO / BO for cleanup.  
   - Open check **before** `iommu_sva_bind_device`, with re-checks after SVA and
     under `dev_lock`.

2. **Secondary:** `mutex_lock_killable` on the short open `dev_lock` section
   after SVA. Residual: `iommu_sva_bind_device` has no killable API.

3. **`exec_cmd_allow` module param (lab series default 0)**  
   Refuses entire EXEC_CMD ioctl (all subtypes) with **`-EPERM`**. Production/XRT:
   `modprobe amdxdna exec_cmd_allow=1`. Upstream may prefer default 1.

4. **AIE2 scope** for automatic latch (AIE4 not covered yet).

Unified diff: `0002-amdxdna-open-failfast-and-exec-gate.patch`  
Apply after L2.

### What this does **not** fix

- Tasks **already** in D-state on a running kernel.
- Hang inside `iommu_sva_bind_device` when unhealthy was never latched.
- Full FLR / DRM device-wedge protocol / complete firmware cmd validation.

### Request

1. Accept AIE2 cmdlist null-guards (or point to existing validation we missed).
2. Interest in fail-fast unhealthy latch + open `-EIO` (or preferred DRM/TDR hook).
3. Guidance on default for `exec_cmd_allow` if accepted (0 lab vs 1 production).
4. Whether IOMMU/SVA killable bind is on any roadmap.

### Attachments (when filing)

- `dmesg` / journal oops (RIP `aie2_cmdlist_fill_slot`, key frames).
- Optional: module refcnt, `/proc/<pid>/stack` for D-state `amdxdna_drm_open`.

### Patch / DCO note

Out-of-tree unified diffs rebased on the SHA above are available for review.
`From` / `Signed-off-by` in those files use a placeholder for local `git am`;
rewrite to a real identity before any amd/xdna-driver PR.

---

**Repo note:** Paste-ready only. Human files on
[amd/xdna-driver issues](https://github.com/amd/xdna-driver/issues) when ready.
