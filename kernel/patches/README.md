# amdxdna driver patches (L2 + L3 — never-again hardening)

Shippable kernel-side materials so a bad/scaffold **EXEC_CMD** cannot NULL-deref
in AIE2 cmdlist fill/execbuf and leave `/dev/accel` opens stuck in D-state, and
so a wedged device fails open/job ioctls fast with a lab EXEC_CMD gate.

**This directory does not install a fixed module on the host.** Userspace L1 gates
remain the primary shield until a patched `amdxdna` is built, installed after a
**clean boot / unload**, and verified.

| Artifact | Purpose |
|----------|---------|
| [`0001-amdxdna-cmdlist-null-guards.patch`](./0001-amdxdna-cmdlist-null-guards.patch) | **L2** — null-guards on AIE2 cmdlist fill/execbuf |
| [`0002-amdxdna-open-failfast-and-exec-gate.patch`](./0002-amdxdna-open-failfast-and-exec-gate.patch) | **L3** — unhealthy latch, killable open lock, `exec_cmd_allow` |
| [`amdxdna-cmdlist-null-guards.md`](./amdxdna-cmdlist-null-guards.md) | L2 design notes |
| [`amdxdna-open-failfast.md`](./amdxdna-open-failfast.md) | L3 design notes |
| Upstream contribution pack | [`docs/upstream/README.md`](../../docs/upstream/README.md) (plan, evidence, filing checklist — do not auto-post) |
| Combined issue body | [`docs/upstream/AMDXDNA_ISSUE_combined.md`](../../docs/upstream/AMDXDNA_ISSUE_combined.md) (do not auto-post) |
| L2 issue body | [`docs/upstream/AMDXDNA_ISSUE_cmdlist_null_open_wedge.md`](../../docs/upstream/AMDXDNA_ISSUE_cmdlist_null_open_wedge.md) |
| L3 issue addendum | [`docs/upstream/AMDXDNA_ISSUE_L3_open_failfast_addendum.md`](../../docs/upstream/AMDXDNA_ISSUE_L3_open_failfast_addendum.md) |

Related repo docs:

- [`docs/XDNA2_NEVER_AGAIN.md`](../../docs/XDNA2_NEVER_AGAIN.md) — L1/L2/L3/L4 policy
- **[`docs/XDNA2_HARDENED_INSTALL.md`](../../docs/XDNA2_HARDENED_INSTALL.md)** — **L4 install playbook** (preflight → apply → build → verify → rollback)
- **[`scripts/install_amdxdna_hardened.sh`](../../scripts/install_amdxdna_hardened.sh)** — wedge-aware L4 install helper (default dry-run; refuses WEDGED_*)
- [`kernel/dkms/`](../dkms/) — L4 DKMS template + `prepare_from_clone.sh` (no full vendor of xdna-driver)
- [`docs/XDNA2_RECOVERY_ENGINEERING.md`](../../docs/XDNA2_RECOVERY_ENGINEERING.md) — wedged-host recovery
- [`kernel/amdxdna_rescue/`](../amdxdna_rescue/) — optional rescue kmod (diagnose / PCI levers)

## Base revision (record this when re-applying)

| Item | Value |
|------|--------|
| Repo | <https://github.com/amd/xdna-driver> |
| **Tree SHA (main at fetch)** | `905717f5cd470580054a1635b9eea24b84dfd67f` |
| Last commit touching `aie2_message.c` (L2) | `b9686b40a31149b4b2dd823d87be4567ba697618` |
| L2 file | `src/driver/amdxdna/aie2_message.c` |
| L3 files | `amdxdna_drm.{c,h}`, `amdxdna_ctx.c`, `aie2_ctx.c`, `aie2_tdr.c` |
| Patch strip | `-p1` (`a/` `b/` prefixes) |

If main has moved, rebase both patches or re-fetch and re-apply by symbol name.

## Apply order (required stack)

Patches are **file-independent** (L2 and L3 touch different files) but the
documented stack is always **L2 then L3**:

```bash
# 1) Clone at the recorded base
git clone https://github.com/amd/xdna-driver.git
cd xdna-driver
git checkout 905717f5cd470580054a1635b9eea24b84dfd67f

# 2) Dry-run full stack, then apply
PATCHDIR=/path/to/dragon-npu/kernel/patches

patch --dry-run -p1 < "$PATCHDIR/0001-amdxdna-cmdlist-null-guards.patch"
patch --dry-run -p1 < "$PATCHDIR/0002-amdxdna-open-failfast-and-exec-gate.patch"

patch -p1 < "$PATCHDIR/0001-amdxdna-cmdlist-null-guards.patch"
patch -p1 < "$PATCHDIR/0002-amdxdna-open-failfast-and-exec-gate.patch"

# Or git am (rewrite From/Signed-off-by to a real DCO identity first):
# git am "$PATCHDIR/0001-amdxdna-cmdlist-null-guards.patch"
# git am "$PATCHDIR/0002-amdxdna-open-failfast-and-exec-gate.patch"
```

Dry-run verification when packaging (agent workspace, **no** host module load,
**no** `/dev/accel` open):

```text
# pristine tree @ 905717f5… :
#   0002 alone     → patch --dry-run -p1  APPLY_OK
#   0001 then 0002 → both apply cleanly   APPLY_OK
```

## What the patches do

### L2 (`0001-…`)

Null-checks before use of cmdlist fill/execbuf objects; return
`-EINVAL` / `-ENODEV` / `-ENOMEM` / `-EOPNOTSUPP` instead of NULL-deref.
See [`amdxdna-cmdlist-null-guards.md`](./amdxdna-cmdlist-null-guards.md).

### L3 (`0002-…`)

| Mechanism | Behavior |
|-----------|----------|
| `atomic_t dev_unhealthy` | Auto-**set** only on TDR **dump-only** stuck path (**not** ordinary job `-ENODEV` / mbox races); **clear** after full TDR restart |
| Auto latch vs defaults | Stock **`tdr_dump_ctx=0`** → TDR full-restarts and **never marks**; open fail-fast **does not auto-arm** until `tdr_dump_ctx=1` (or an explicit mark). Always-on: `exec_cmd_allow=0`, killable open `dev_lock`, latch checks. |
| `amdxdna_drm_open` | If unhealthy → **`-EIO` before SVA bind** (primary); re-check after SVA / under lock; **secondary** `mutex_lock_killable(dev_lock)` |
| CREATE / CONFIG_HWCTX, EXEC_CMD, SET_STATE | If unhealthy → **`-EIO`** (DESTROY/WAIT/GET_INFO/BO remain for cleanup) |
| `exec_cmd_allow = false` (lab default) | Entire EXEC_CMD ioctl (all subtypes) → **`-EPERM`** unless `modprobe … exec_cmd_allow=1` |

Does **not** make `iommu_sva_bind_device` killable (no kernel API). Does **not**
unstick already-D tasks. **L3 alone does not latch after oops** — stack **L2 first**.
Style is minimal and upstreamable; lab default for `exec_cmd_allow` may flip to 1
for an upstream PR.

**DCO note:** patch `From` / `Signed-off-by` use a dragon-npu placeholder
(`devnull@localhost`) so `git am` works in this tree. Rewrite to a **real
identity** before any amd/xdna-driver PR.

## Build the out-of-tree module (xdna-driver tree)

Requires kernel headers matching the **running** kernel (`uname -r`), build tools,
and usually root for install.

### Preferred: upstream CMake / DKMS path

Follow the **xdna-driver project README / build docs** for the supported CMake
or DKMS flow (package name and scripts evolve; do not invent a second build
system). In practice:

1. Apply **both** patches to the **same source tree** your DKMS or vendor package builds from, **or**
2. Build a local DKMS package from the patched clone, then `dkms install` / distro install.

Many systems already ship `amdxdna` via DKMS or a vendor package (not in-tree
mainline on all distros). If your distro only ships a prebuilt `.ko`, you must
rebuild from source; there is no “drop-in replace without matching source”
guarantee.

### Lab modprobe after install

```bash
# 1) Lab bring-up: EXEC_CMD refused in kernel (default after L3 = 0).
#    Open fail-fast latch does NOT auto-arm: stock tdr_dump_ctx=0 full-restarts only.
modprobe amdxdna
# or explicit: modprobe amdxdna exec_cmd_allow=0

# 2) Intentional payload experiments / known-good EXEC_CMD on this lab module
#    (still requires userspace L1: feature + XDNA2_ALLOW_EXEC_CMD=1).
#    Without this step, even a correct payload gets kernel -EPERM — not a broken driver.
modprobe -r amdxdna   # if safe / unused
modprobe amdxdna exec_cmd_allow=1

# 3) Optional: exercise automatic open fail-fast after TDR stuck detect
#    (dump-only; does NOT restart contexts — lab/debug only, not production recovery).
# modprobe amdxdna tdr_dump_ctx=1
# # combine: modprobe amdxdna exec_cmd_allow=0 tdr_dump_ctx=1

# Production / XRT (same as step 2):
# modprobe amdxdna exec_cmd_allow=1
# or: options amdxdna exec_cmd_allow=1  in /etc/modprobe.d/
```

### Footnote: bare Kbuild sketch (often incomplete alone)

Upstream `src/driver/amdxdna/Makefile` expects a build root separate from
`SRC_DIR` and usually needs `config_kernel.h` (via `tools/configure_kernel.sh`
or CMake). A raw module build may fail without those steps:

```bash
# Last resort only — prefer CMake/DKMS above
cd xdna-driver/src/driver/amdxdna
make -C /lib/modules/$(uname -r)/build M=$PWD modules
```

## Install honesty (wedged vs clean host)

| Situation | What works |
|-----------|------------|
| Module **in use** / open waiters in **D-state** / high **refcnt** | **Cannot** safely `rmmod` or load a replacement over the wedged instance. Reboot (or measured recovery) first. See recovery docs. |
| Clean boot, no D-state waiters | `modprobe -r amdxdna` (if unused) → install new `.ko` → `modprobe amdxdna` → diagnose. |
| Live wedge **right now** | Do **not** open `/dev/accel` in a loop; do **not** assume these patches are already loaded on the laptop. |

```bash
# After clean boot + install — read-only checks only
bash scripts/diagnose_amdxdna.sh
# Optional: modinfo amdxdna | grep -E 'filename|exec_cmd'
# Confirm you did *not* enable userspace EXEC_CMD unless intentional + double gate
```

## What this does *not* claim

- The **running host** already has L2/L3. It does not, until you install.
- Full recovery of an already-wedged kernel without reboot.
- Killable/timeout conversion of `iommu_sva_bind_device` (needs IOMMU API work).
- A complete firmware command validator or Phase B payload work.
- Upstream merge — issue bodies are ready; **user files** the GitHub issue.

## L4 install path (after clean boot)

Do **not** install over a wedged module. Prefer:

```bash
bash scripts/diagnose_amdxdna.sh
bash scripts/install_amdxdna_hardened.sh              # preflight + next steps
bash scripts/install_amdxdna_hardened.sh --apply \
  --workdir /tmp/xdna-driver-hardened                 # clone @ SHA, apply 0001+0002
# then build/DKMS per docs/XDNA2_HARDENED_INSTALL.md and kernel/dkms/README.md
```

Full playbook: [`docs/XDNA2_HARDENED_INSTALL.md`](../../docs/XDNA2_HARDENED_INSTALL.md).

## Follow-ups (beyond L2+L3 materials / L4 packaging)

1. **Host install** via L4 playbook after clean boot (NEVER_AGAIN checkbox).
2. File + land patches (or refined versions) on amd/xdna-driver (real DCO).
3. Sibling non-cmdlist `aie2_execbuf` / `aie2_init_exec_req` null-guards (L2.1).
4. Null-safe `amdxdna_cmd_get_op` (and related helpers) in `amdxdna_ctx.h`.
5. Optional upstream default flip of `exec_cmd_allow` to 1.
6. Enroll `kernel/dkms/` package long-term on this host (scaffolding ready).
