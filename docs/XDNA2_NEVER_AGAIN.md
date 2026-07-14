# Never again: amdxdna wedge policy (2026-07-13)

## What happened

1. dragon-npu issued **scaffold EXEC_CMD** (not a real firmware txn).
2. Kernel workqueue hit **NULL deref** in `aie2_cmdlist_fill_slot` (`amdxdna`).
3. Later **`open("/dev/accel/accel0")` hung** in D-state (`amdxdna_drm_open`).
4. Module **refcnt** stayed high → no `rmmod`; PCI remove/unbind hung.

This is **not** acceptable bring-up. Permanent controls below exist so we cannot
recreate it by accident.

## Permanent controls (userspace — this repo) — **L1 complete**

| Layer | Mechanism | Default |
|-------|-----------|---------|
| **Compile** | Cargo feature `xdna2-dangerous-exec-cmd` | **OFF** |
| **Runtime** | Env `XDNA2_ALLOW_EXEC_CMD=1` | **unset** |
| **API** | `exec_cmd_is_enabled()` / `RealDirectSubmission::submit_command` | hard error if gated |
| **Probe / timed_matmul** | Never call EXEC_CMD unless both layers allow | skip submit |
| **Honesty** | `ran_on_npu` only for real compute; never scaffold Pass | enforced |

Enabling EXEC_CMD requires **both** userspace gates (and, **if L3 is installed**, the kernel param):

```bash
# L1 (always required in this repo)
cargo build --features amd-xdna2,xdna2-dangerous-exec-cmd
XDNA2_ALLOW_EXEC_CMD=1 cargo run --features amd-xdna2,xdna2-dangerous-exec-cmd --example …

# L3 (only after installing the lab-patched module — default refuses EXEC_CMD):
#   sudo modprobe amdxdna exec_cmd_allow=1
# Without this, the kernel returns -EPERM even when L1 is fully open.
# Stock/distro amdxdna without L3 has no exec_cmd_allow param.
```

Legacy `XDNA2_TRY_SUBMIT` is **fully ignored** (retired). Userspace only the double gate above can enable EXEC_CMD from dragon-npu; a lab L3 module adds a third gate.

## Permanent controls (driver — **L2+L3 materials complete**, **L4 install packaging complete**, host install pending)

Shippable in-repo artifacts (applyable patches + docs + issue bodies + install path):

| Artifact | Path |
|----------|------|
| L2 unified patch | [`kernel/patches/0001-amdxdna-cmdlist-null-guards.patch`](../kernel/patches/0001-amdxdna-cmdlist-null-guards.patch) |
| L3 unified patch | [`kernel/patches/0002-amdxdna-open-failfast-and-exec-gate.patch`](../kernel/patches/0002-amdxdna-open-failfast-and-exec-gate.patch) |
| Apply/build README | [`kernel/patches/README.md`](../kernel/patches/README.md) (apply **0001 then 0002**) |
| L2 design notes | [`kernel/patches/amdxdna-cmdlist-null-guards.md`](../kernel/patches/amdxdna-cmdlist-null-guards.md) |
| L3 design notes | [`kernel/patches/amdxdna-open-failfast.md`](../kernel/patches/amdxdna-open-failfast.md) |
| **L4 install script** | [`scripts/install_amdxdna_hardened.sh`](../scripts/install_amdxdna_hardened.sh) (wedge-aware; default dry-run) |
| **L4 install playbook** | [`docs/XDNA2_HARDENED_INSTALL.md`](./XDNA2_HARDENED_INSTALL.md) |
| **L4 DKMS scaffolding** | [`kernel/dkms/`](../kernel/dkms/) (template + prepare hook; no full vendor) |
| Upstream issue L2 (paste-ready) | [`docs/upstream/AMDXDNA_ISSUE_cmdlist_null_open_wedge.md`](./upstream/AMDXDNA_ISSUE_cmdlist_null_open_wedge.md) |
| Upstream L3 addendum (paste-ready) | [`docs/upstream/AMDXDNA_ISSUE_L3_open_failfast_addendum.md`](./upstream/AMDXDNA_ISSUE_L3_open_failfast_addendum.md) |
| Upstream combined L2+L3 (paste-ready) | [`docs/upstream/AMDXDNA_ISSUE_combined.md`](./upstream/AMDXDNA_ISSUE_combined.md) |

**Base tree:** amd/xdna-driver @ `905717f5cd470580054a1635b9eea24b84dfd67f`.

Goals for amdxdna (GPL, forkable):

1. **No NULL deref** on bad/empty cmdlist — return `-EINVAL` / `-ENODEV` / `-ENOMEM` / `-EOPNOTSUPP`. (**L2 patch — materials ready**)
2. **Fail-fast open/ioctls when latched unhealthy** — return `-EIO` instead of infinite D. (**L3 materials ready**)  
   **Honest coverage:** automatic latch is **TDR dump-only only**. Stock **`tdr_dump_ctx=0`** means the periodic TDR **full-restarts** and **never marks** — open fail-fast **does not auto-arm** under default module params. Always-on L3 under defaults: **`exec_cmd_allow=0`**, secondary killable open `dev_lock`, latch infrastructure. To auto-arm fail-fast after stuck detect (lab/debug): `modprobe amdxdna tdr_dump_ctx=1`. Do **not** mark from ordinary job `-ENODEV` (mbox teardown race).
3. Secondary killable `dev_lock` on open only — SVA bind residual documented. (**L3**)
4. Module param `exec_cmd_allow=0` lab default so **entire** EXEC_CMD ioctl is refused in-kernel. (**L3** — always on after install)

Until a fixed module is **installed on this host**, **userspace gates are the primary shield**.
L2+L3 materials are complete in-tree; they do **not** mean the running kernel already has the fix.

## Ops rules (humans + agents)

1. **Never** open `/dev/accel` in a loop while D-state waiters exist.
2. **Diagnose first:** `bash scripts/diagnose_amdxdna.sh`.
3. **No EXEC_CMD** until a **known-good** payload exists (not a research scaffold).
4. Hardware probes: always `timeout …`.
5. File/track upstream: [amd/xdna-driver issues](https://github.com/amd/xdna-driver/issues) — paste body from `docs/upstream/…` when ready (human files).

## Recovery (if it ever happens again)

1. Stop opening the device.
2. `bash scripts/diagnose_amdxdna.sh`.
3. Prefer reboot to clear D-state (PCI reset/remove may hang — measured).
4. After clean boot: confirm with diagnose; only then continue bring-up **without** dangerous-exec feature.
5. Optionally install L2+L3-patched `amdxdna` **only after** clean unload/boot:
   - Playbook: [`docs/XDNA2_HARDENED_INSTALL.md`](./XDNA2_HARDENED_INSTALL.md)
   - Script: `bash scripts/install_amdxdna_hardened.sh` (refuses if still wedged)
   - Patches: `kernel/patches/README.md` · DKMS template: `kernel/dkms/README.md`
   Lab default after L3: `exec_cmd_allow=0`; production/XRT needs `exec_cmd_allow=1`.
   Open fail-fast auto-latch needs non-default `tdr_dump_ctx=1` (see `kernel/patches/amdxdna-open-failfast.md`).

## Definition of “fixed forever”

- [x] Default dragon-npu builds **cannot** EXEC_CMD. (**L1**)
- [x] Double gate (feature + env) documented and tested. (**L1**)
- [x] **L2 materials:** applyable null-guard patch + build/apply docs + upstream issue body (in-tree).
- [x] **L3 materials:** open fail-fast / killable `dev_lock` / `exec_cmd_allow` patch + docs + issue addendum (in-tree).
- [x] **L4 materials:** install script + HARDENED_INSTALL playbook + DKMS scaffolding + combined issue body (in-tree). **Does not mean host is fixed.**
- [ ] **L2/L3 installed on host:** patched `amdxdna` built/installed after clean boot or DKMS (unchecked until done — use L4 playbook).
- [ ] Issue **filed** on amd/xdna-driver with oops + open-wedge notes (paste-ready combined body; human posts).
- [ ] Optional: DKMS package with hardened driver **tracked long-term on this host** (scaffolding in `kernel/dkms/`; host not enrolled yet).

## L1 unit-test matrix (lib only; no `/dev/accel` required)

Lib tests assume EXEC_CMD is **not** fully enabled in the process. Run both:

```bash
# Feature OFF (compile-time gate) — primary CI
cargo test --features amd-xdna2 --lib

# Feature ON, env OFF (runtime gate) — must still refuse EXEC_CMD
# Do NOT set XDNA2_ALLOW_EXEC_CMD=1 for this job.
cargo test --features amd-xdna2,xdna2-dangerous-exec-cmd --lib
```

A third job with `XDNA2_ALLOW_EXEC_CMD=1` is **manual/dangerous** (scaffold can oops an **unpatched** driver).
