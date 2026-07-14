# XDNA2 hardened amdxdna install playbook (L4)

Single ops path to install **L2 + L3** hardened `amdxdna` after a **clean boot**,
without forcing load over a wedge and without inventing a broken build system.

| Layer | What | Status in repo |
|-------|------|----------------|
| L1 | Userspace EXEC_CMD double gate | Done (default OFF) |
| L2 | Cmdlist null-guards patch | Materials + patch |
| L3 | Open fail-fast + `exec_cmd_allow` | Materials + patch |
| **L4** | Install script + DKMS template + this playbook | **This doc** |

Host still needs a human to complete install after clean boot. Agents must not
`insmod` / `rmmod -f` / reboot / claim the host is fixed.

## TL;DR

```bash
# 0) If wedged right now — stop. Do not open /dev/accel.
bash scripts/diagnose_amdxdna.sh
# WEDGED_*  → reboot (or recovery docs); re-run diagnose after

# 1) After clean boot — dry-run / prep (default refuses if still wedged)
bash scripts/install_amdxdna_hardened.sh
bash scripts/install_amdxdna_hardened.sh --apply --workdir /tmp/xdna-driver-hardened

# 2) Build via upstream xdna-driver flow or kernel/dkms/ (see below)

# 3) Load lab defaults, verify without EXEC_CMD
sudo modprobe -r amdxdna    # only if unused / not wedged
sudo modprobe amdxdna       # L3: exec_cmd_allow=0
bash scripts/diagnose_amdxdna.sh
timeout 15 cargo run --features amd-xdna2 --example npu_direct_bench -- --checklist
```

## 1. Preflight (always)

### Read-only diagnose

```bash
bash scripts/diagnose_amdxdna.sh
# optional: --inxi / sudo --with-root for stacks
```

**Safety:** diagnose never opens `/dev/accel` (open can hang forever while wedged).

#### Diagnose verdicts (`scripts/diagnose_amdxdna.sh`)

| Verdict | Install? |
|---------|----------|
| `WEDGED_OPEN` / `WEDGED_OTHER` | **No** — reboot or recovery first |
| `SUSPICIOUS_REFCNT` | Prefer clean boot; careful |
| `LIKELY_OK_METADATA` / `MODULE_NOT_LOADED` | Yes, proceed |
| `PARTIAL` / `UNKNOWN` / `DRIVER_MISMATCH` | Read full diagnose; do not force |

#### Install-script preflight (`install_amdxdna_hardened.sh`)

A **related but thinner** subset — not identical enums to diagnose:

| Verdict | Default / check | `--apply` (source-only) |
|---------|-----------------|-------------------------|
| `OK` / `MODULE_NOT_LOADED` | continue | continue |
| `WEDGED_OPEN` / `WEDGED_OTHER` | refuse (exit 1), wedge guidance | refuse |
| `SUSPICIOUS_REFCNT` | warn, continue | refuse unless `--allow-suspicious` |
| `PREFLIGHT_UNKNOWN` (ps/refcnt unusable) | refuse | refuse |
| `DRIVER_MISMATCH` / `PARTIAL_PCI` | warn, continue | **warn, continue** (source prep OK; load steps suppressed) |

Hard wedge refuse messages are distinct from SUSPICIOUS / UNKNOWN. Prefer full
diagnose as the authority before any `modprobe`. Residual: waiters whose wchan
does not contain `amdxdna` may be missed — bias is refuse on unknown, not claim OK.

`--allow-suspicious` only relaxes **`SUSPICIOUS_REFCNT`**. PCI verdicts do not
need it. Offline source prep when host health cannot be checked:

```bash
bash scripts/install_amdxdna_hardened.sh --apply --skip-wedge-check \
  --offline-source-only --workdir /tmp/xdna-driver-hardened
# load steps suppressed until diagnose is clean
```

### Clean boot requirement

A live wedge (D-state open waiters, high refcnt) **cannot** safely replace the
module: `rmmod` fails, force unload is dangerous, and loading a second copy does
not unstick already-D tasks.

1. Prefer **reboot**.
2. Optional engineering recovery (root, may still fail):
   [`XDNA2_RECOVERY_ENGINEERING.md`](./XDNA2_RECOVERY_ENGINEERING.md)
3. After boot: diagnose again until **not** `WEDGED_*`.

## 2. Apply L2 + L3 (source)

Documented base: [amd/xdna-driver](https://github.com/amd/xdna-driver) @
`905717f5cd470580054a1635b9eea24b84dfd67f`.

```bash
# Check artifacts + preflight + next steps
bash scripts/install_amdxdna_hardened.sh

# Clone @ SHA, dry-run + apply 0001 then 0002, print build commands
# (refuses if WEDGED_* / PREFLIGHT_UNKNOWN; SUSPICIOUS_REFCNT needs --allow-suspicious)
bash scripts/install_amdxdna_hardened.sh --apply --workdir /tmp/xdna-driver-hardened

# Offline / CI: skip host wedge check, dry-run patches only
bash scripts/install_amdxdna_hardened.sh --dry-run-patches --skip-wedge-check \
  --workdir /tmp/xdna-driver-hardened

# Offline / CI: apply patches without host preflight (required companion flag)
# --skip-wedge-check --apply alone exits 2; load instructions are suppressed.
bash scripts/install_amdxdna_hardened.sh --apply --skip-wedge-check \
  --offline-source-only --workdir /tmp/xdna-driver-hardened
```

Manual equivalent: [`kernel/patches/README.md`](../kernel/patches/README.md)
(apply order **0001 → 0002**).

| Patch | Role |
|-------|------|
| `0001-amdxdna-cmdlist-null-guards.patch` | L2 — no NULL deref on bad cmdlist |
| `0002-amdxdna-open-failfast-and-exec-gate.patch` | L3 — latch + killable open lock + `exec_cmd_allow` |

**Stack L2 first:** L3 alone does not latch after a hard oops.

## 3. Build and install

Prefer the **upstream xdna-driver** CMake / package / DKMS flow for the checked-out
release. Do not invent a second broken Makefile.

### Option A — upstream project build

```bash
cd /tmp/xdna-driver-hardened
# Follow that tree's README / docs for the SHA you checked out.
```

### Option B — DKMS template (this repo)

```bash
bash kernel/dkms/hooks/prepare_from_clone.sh /tmp/xdna-driver-hardened
# Then root steps in kernel/dkms/README.md
```

Details: [`kernel/dkms/README.md`](../kernel/dkms/README.md).

### Arch / Garuda zen headers

```bash
uname -r
sudo pacman -S linux-zen-headers dkms base-devel   # match running kernel
```

### Bare Kbuild (last resort)

Often needs `config_kernel.h` / `tools/configure_kernel.sh` — see patches README
footnote. Prefer A or B.

## 4. Modprobe options (lab default)

After L3 is **installed** in the module you load:

| Param | Lab default | Meaning |
|-------|-------------|---------|
| `exec_cmd_allow` | **0** | Entire EXEC_CMD ioctl → `-EPERM` |
| `tdr_dump_ctx` | stock **0** | Full TDR restart; open fail-fast **does not auto-arm** |

```bash
# Lab bring-up (refuse EXEC_CMD in kernel)
sudo modprobe amdxdna
# explicit: sudo modprobe amdxdna exec_cmd_allow=0

# Known-good payload experiments only (still needs L1 userspace gates):
# sudo modprobe amdxdna exec_cmd_allow=1

# Optional lab: auto-arm open fail-fast on TDR dump-only stuck detect
# sudo modprobe amdxdna exec_cmd_allow=0 tdr_dump_ctx=1
```

Persist lab default (optional):

```bash
echo 'options amdxdna exec_cmd_allow=0' | sudo tee /etc/modprobe.d/amdxdna-hardened.conf
```

**Userspace L1** remains required regardless:

```bash
# Never enable unless intentional
# cargo … --features amd-xdna2,xdna2-dangerous-exec-cmd
# XDNA2_ALLOW_EXEC_CMD=1
```

## 5. Post-install verify

```bash
bash scripts/diagnose_amdxdna.sh
# expect: not WEDGED_*; ideally LIKELY_OK_METADATA

modinfo amdxdna | grep -E 'filename|exec_cmd|vermagic|srcversion' || true
# filename should point at your DKMS/updates path if you replaced the module
# exec_cmd_allow appears only on L3-patched modules

# Safe checklist — no dangerous EXEC_CMD feature
timeout 15 cargo run --features amd-xdna2 --example npu_direct_bench -- --checklist
```

Only after this is clean, check **“L2/L3 installed on host”** in
[`XDNA2_NEVER_AGAIN.md`](./XDNA2_NEVER_AGAIN.md).

## 6. Rollback

1. `sudo dkms remove -m amdxdna-hardened -v 0.1.0 --all` (if DKMS path used).
2. Remove `/etc/modprobe.d/amdxdna-hardened.conf` if added.
3. Restore distro/in-tree `amdxdna.ko` (package reinstall or reboot to stock).
4. `sudo modprobe amdxdna` or reboot.
5. Diagnose again.

If the host wedges during experiments, do **not** fight force-rmmod first —
reboot, then reinstall or roll back on a clean kernel.

## 7. What L4 does *not* claim

- Host already has L2/L3 (until you install and verify).
- Recovery of current D-state waiters without reboot.
- Automatic GitHub issue filing (paste-ready only).
- Phase B real EXEC_CMD payload validation.

## Cross-links

| Doc | Why |
|-----|-----|
| [`XDNA2_NEVER_AGAIN.md`](./XDNA2_NEVER_AGAIN.md) | Policy + checkboxes (L1–L4) |
| [`kernel/patches/README.md`](../kernel/patches/README.md) | Apply order, SHA, dry-run |
| [`kernel/dkms/README.md`](../kernel/dkms/README.md) | DKMS template + Arch notes |
| [`scripts/install_amdxdna_hardened.sh`](../scripts/install_amdxdna_hardened.sh) | Automated preflight + apply |
| [`scripts/diagnose_amdxdna.sh`](../scripts/diagnose_amdxdna.sh) | Read-only wedge diagnose |
| [`XDNA2_RECOVERY_ENGINEERING.md`](./XDNA2_RECOVERY_ENGINEERING.md) | Optional non-reboot recovery |
| [`upstream/AMDXDNA_ISSUE_combined.md`](./upstream/AMDXDNA_ISSUE_combined.md) | One-shot issue body (do not auto-post) |
