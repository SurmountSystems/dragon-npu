# XDNA2 / amdxdna wedge — engineering recovery (no full reboot)

## Incident model

1. Userspace issued a **non-verified EXEC_CMD / cmdlist** payload (scaffold).
2. Kernel workqueue `amdxdna_js` hit **NULL deref** in `aie2_cmdlist_fill_slot`.
3. Collateral: `drm_mm_takedown` warnings during BO free / CREATE_BO cleanup.
4. Afterward, **`open("/dev/accel/accel0")` never returns** — tasks sleep in
   `amdxdna_drm_open` (D-state). Module **refcnt stays high** → `rmmod` fails.

This is a **device/driver soft-deadlock**, not a userspace leak of fds that
`kill` can fix. D-state tasks ignore `SIGKILL` until the wait ends.

## Why open hangs (code-level)

Upstream `amdxdna_drm_open` (amd/xdna-driver) roughly:

1. `kzalloc` client  
2. **`iommu_sva_bind_device`** (can sleep)  
3. short `mutex_lock(dev_lock)` / list_add / unlock  
4. return  

Stuck wchan `amdxdna_drm_open` ⇒ typically (2) or a lock taken under DRM open
wrappers, not “random modprobe bookkeeping.” After firmware/job state is
corrupted by the oops, bind or internal locks may never complete.

## Hardware topology (this host class: Strix/Krackan)

| Address | ID | Role |
|---------|-----|------|
| `0000:c6:00.1` | `1022:17f0` | **NPU** (`amdxdna`) — often **no** sysfs `reset` |
| `0000:c6:00.0` | `1022:1116` | PCIe dummy function |
| `0000:00:08.2` | `1022:1111` | **Internal GPP bridge** — **has** sysfs `reset` |

**Lever:** secondary bus reset on `0000:00:08.2` resets the NPU (and dummy),
**not** the discrete/main `amdgpu` graphics device on other bridges.

## Recovery tools in this repo

### 1. Read-only diagnosis (always first)

```bash
bash scripts/diagnose_amdxdna.sh --inxi | pbcopy   # fish: use |
```

Never opens `/dev/accel`.

### 2. Engineering recovery (root)

```bash
sudo bash scripts/amdxdna_recover_eng.sh diagnose
sudo bash scripts/amdxdna_recover_eng.sh recover
```

Pipeline:

1. Userspace snapshot  
2. Optional **rescue kmod** stack dump (`kernel/amdxdna_rescue`)  
3. **Sysfs bridge reset** on GPP `0000:00:08.2`  
4. kmod `reset_bus` + `wake` if still D  
5. PCI **remove** NPU → **rescan** → `modprobe -r` / `modprobe amdxdna`  
6. Timed open verify  

### 3. Rescue kmod (GPL, out-of-tree)

```bash
cd kernel/amdxdna_rescue
make
sudo insmod amdxdna_rescue.ko action=diagnose
sudo dmesg -T | tail -80
sudo rmmod amdxdna_rescue
# stronger:
sudo insmod amdxdna_rescue.ko action=full
```

Actions: `diagnose` | `reset_npu` | `reset_bus` | `wake` | `full`.

## If recovery still fails

Then the wait is not broken by PCI reset (e.g. pure software lock in a dead
thread with no bus side-effect). Options:

1. **Reboot** — guaranteed clear of D-state and module text.  
2. **Fork the driver** (below) so a *future* wedge is survivable: open timeouts,
   debugfs force-flush, refuse scaffold EXEC_CMD unless explicit.  
3. **Livepatch** (heavy) — only if we must unstick a production box once.

## Driver fork roadmap (so we never need penance again)

Out-of-tree or in-tree patch series on `amdxdna`:

| Change | Why |
|--------|-----|
| `open` timeout / killable waits where legal | Prevent infinite D in SVA bind |
| Debugfs `force_client_teardown` | Drop clients without full rmmod |
| Hard gate EXEC_CMD on known payload magic / CAP | No accidental cmdlist NULL |
| Better error paths on BO free after failed job | Avoid `drm_mm_takedown` WARN storms |
| Module param `exec_cmd_allow=0` default | Match userspace double gate |

Userspace already: feature `xdna2-dangerous-exec-cmd` default OFF + env
`XDNA2_ALLOW_EXEC_CMD` unset; diagnose script; no auto scaffold submit
(see `docs/XDNA2_NEVER_AGAIN.md`).

### AIE4 power management and 17f0 recovery

The 17f0 recovery procedure stays the diagnose, bridge reset of
`0000:00:08.2`, and rescue module path in this file. AIE4 runtime power
management is the V5 19 suspend and resume path that Lizhi Hou applied to
drm-misc-next on 2026-10-21. That patch also assigns the existing
`aie2_hw_suspend` and `aie2_hw_resume` as `.runtime_suspend` and
`.runtime_resume` on `aie2_ops`. Those hooks are power management. They
do not clear a wedged `amdxdna_drm_open`.

This host is PCI `1022:17f0` revision `0x20`, which patch 5 leaves on
`dev_npu6_info`. The AIE4 classic info struct `dev_npu3_classic_info`
is the new `0x17f1` rows.

The AIE4 job-timeout that stops a hardware context is a pattern for the
`open` timeout / killable-wait row in the table above: a wait that can
end, in place of an unbounded D-state inside SVA bind. V5 14 also makes
the common HMM invalidate wait uninterruptible and unlimited
(`dma_resv_wait_timeout` with `false` and `MAX_SCHEDULE_TIMEOUT`). That
wait is a kernel-bump re-check. The list is D-AIE4-3 in
`.agents/plans/plan-amd-aie4-kmq-pm-series.md`. It is a separate question
from this recovery engine.

invent_pass stays 0. Pass stays No. `ran_on_npu` stays false. Unlock
none. First `C_UNLOCK=` is `2c9c12e8` CONSUMED.

### AIE4 kernel submission and what 17f0 does not get (2026-10-24)

The 17f0 procedure in this file does not change. It stays diagnose,
bridge reset of `0000:00:08.2`, and the rescue module. Do not replace
that with a runtime-PM resume.

Lizhi Hou applied V5 "accel/amdxdna: Kernel submission and PM for AIE4"
to drm-misc-next on 2026-10-21
([14306](https://lkml.iu.edu/2610.2/14306.html), accessed: 2026-10-24).
The pull
[607373](https://lists.freedesktop.org/archives/dri-devel/2026-October/607373.html)
(accessed: 2026-10-24) requests tag `drm-misc-next-2026-10-23` for
v7.5. That name is a requested tag, not a released mainline tag. No
mainline version is claimed here.

Kernel-mode submit for NPU3 classic is not the AIE2 ops table. Patch 16
sets `.cmd_submit = aie4_cmd_submit` on `const struct amdxdna_dev_ops
aie4_classic_ops` in `drivers/accel/amdxdna/aie4_pci.c`
([07861](https://lkml.iu.edu/2610.2/07861.html), accessed: 2026-10-24).
The same file's `aie4_vf_ops` gets the same callback. Patch 5 points
`dev_npu3_classic_info` at `&aie4_classic_ops` for `1022:17f1` revisions
`0x10` and `0x13` (`npu3_regs.c`,
[07943](https://lkml.iu.edu/2610.2/07943.html), accessed: 2026-10-24).
`0x17f0` revision `0x20` stays `dev_npu6_info` and keeps the AIE2 submit
path. Patch 5 also prints `.device_type = AMDXDNA_DEV_TYPE_UMQ` on
`dev_npu3_classic_info`. The cover still calls the feature KMQ. Both
lines were read. The enum word does not attach `aie4_cmd_submit` to this
host.

Linux master `npu6_regs.c` still has `.ops = &aie2_ops` on
`dev_npu6_info`
([npu6_regs.c](https://raw.githubusercontent.com/torvalds/linux/master/drivers/accel/amdxdna/npu6_regs.c),
accessed: 2026-10-24). Patch 19 is the only AIE2 power hook in this
series: `.runtime_suspend = aie2_hw_suspend` and
`.runtime_resume = aie2_hw_resume` on `aie2_ops`
([08350](https://lkml.iu.edu/2610.2/08350.html), accessed: 2026-10-24).
That is not AIE4 job suspend. Patch 18 puts
`.suspend = pm_ptr(aie4_classic_suspend)` on `aie4_classic_ops` and
describes draining in-flight jobs, then recreating firmware contexts on
resume
([08021](https://lkml.iu.edu/2610.2/08021.html), accessed: 2026-10-24).
That page does not use the words job-save or firmware-restore. The
behavior on the page is AIE4-only. It does not change the 17f0 rescue
steps.

Clock, DPM, and resource queries are listed on the cover as AIE4
patches. Patch 18's `aie4_restore_sriov()` is PF resume for AIE4 SR-IOV.
No AIE2 hook for those queries was on the pages opened. They are N/A
for `1022:17f0`.

D-AIE4-3 stays. Patch 15 (`populate_range` moved into common GEM code,
"No functional change for AIE2") and patch 02 (doorbell mmap removed,
common `.mmap` set to `drm_gem_mmap`) are still shared-code re-checks.
Do not delete that list.

Horizon re-check, list only. Do not run it from this note:
`power/runtime_status`, `autosuspend_delay_ms`, first-submit latency
after idle, a submit across suspend/resume, and whether bridge reset of
`0000:00:08.2` is still needed after resume.

`uname -r` on this machine printed `7.2.11-zen1-1-zen`. The installed
module's strings include `aie2_ops` and `aie2_cmd_submit`. They did not
include `aie4_cmd_submit` or `aie4_classic`. That is a string search of
`amdxdna.ko.zst`, not a claim from the uname line alone. The module
build tree under that kernel only has `Kconfig` for this driver, not
the C sources.

invent_pass stays 0. Pass stays No. `ran_on_npu` stays false. Unlock
none. First `C_UNLOCK=` is `2c9c12e8` CONSUMED. Do not mint.

## Relation to “fork Linux”

We do **not** need a full distro kernel fork to start. We need:

1. **Rescue path** (this doc + kmod + bridge reset) for live boxes  
2. **Out-of-tree amdxdna** (or distro patch) with open timeout + exec gate  
3. Only if rejected upstream: carry a **zen/Arch package fork** of the module

Full tree fork is optional escalation, not step zero.
