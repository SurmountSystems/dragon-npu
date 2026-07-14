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

## Relation to “fork Linux”

We do **not** need a full distro kernel fork to start. We need:

1. **Rescue path** (this doc + kmod + bridge reset) for live boxes  
2. **Out-of-tree amdxdna** (or distro patch) with open timeout + exec gate  
3. Only if rejected upstream: carry a **zen/Arch package fork** of the module

Full tree fork is optional escalation, not step zero.
