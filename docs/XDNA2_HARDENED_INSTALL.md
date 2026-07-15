# XDNA2 hardened amdxdna install playbook (L4)

Single ops path to install **L2 + L3** hardened `amdxdna` after a **clean boot**,
without forcing load over a wedge and without inventing a broken build system.

**DKMS vs upstream decision tree (Phase 9):**
[`XDNA2_PACKAGING.md`](./XDNA2_PACKAGING.md) — lab OOT / `updates/` / DKMS bridge
vs long-term upstream merge → distro module. L2 materials ≠ host fixed until
installed and verified.

| Layer | What | Status in repo |
|-------|------|----------------|
| L1 | Userspace EXEC_CMD double gate | Done (default OFF) |
| L2 | Cmdlist null-guards patch | Materials + patch |
| L3 | Open fail-fast + `exec_cmd_allow` | Materials + patch |
| **L4** | Install script + DKMS template + this playbook | **This doc** |
| **Phase 0** | Host verify script + reboot/upgrade recipes | `scripts/verify_amdxdna_hardened_host.sh` |

**Lab host (horizon):** L2+L3 already under `updates/amdxdna-hardened/` with
`exec_cmd_allow=0` (see NEVER_AGAIN / DIRECT_CAPABILITIES). Other machines still
need a human install after clean boot. Agents must not `insmod` / `rmmod -f` /
reboot / claim reboot-proof without the post-reboot checklist.

## TL;DR

```bash
# 0) If wedged right now — stop. Do not open /dev/accel.
bash scripts/diagnose_amdxdna.sh
# WEDGED_*  → reboot (or recovery docs); re-run diagnose after

# 0b) Already installed? Read-only identity + gates:
bash scripts/verify_amdxdna_hardened_host.sh

# 1) After clean boot — dry-run / prep (default refuses if still wedged)
bash scripts/install_amdxdna_hardened.sh
bash scripts/install_amdxdna_hardened.sh --apply --workdir /tmp/xdna-driver-hardened

# 2) Build via upstream xdna-driver flow or kernel/dkms/ (see below)
#    Garuda: PATH=/usr/bin:/bin  (avoid wild linker)

# 3) Load lab defaults, verify without EXEC_CMD
sudo modprobe -r amdxdna    # only if unused / not wedged
sudo modprobe amdxdna       # L3: exec_cmd_allow=0
bash scripts/verify_amdxdna_hardened_host.sh
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

### Garuda / Arch build footguns (lab-proven)

| Footgun | Mitigation |
|---------|------------|
| `/usr/local/bin/ld` → **wild** breaks `module.lds` / module link | Force system linker: `export PATH=/usr/bin:/bin` for `configure_kernel` **and** `make` |
| OOT tree requests `npu.dev.sbin` only (no multi-name fallback) | Symlink under firmware dir (see § firmware alias below) |
| `softdep amdxdna … gpu_sched` in modprobe.d | **Do not** add softdep — while `amdgpu` holds `gpu_sched`, `modprobe -r amdxdna` becomes FATAL |
| Bare `insmod` of `amdxdna.ko` | Needs `amd_pmf` (and friends) already loaded; prefer `depmod -a` + `modprobe amdxdna` |
| Kernel package upgrade (`uname -r` changes) | Rebuild OOT module into **new** `updates/` tree (see § kernel upgrade) |

Lab host install path (not DKMS yet):

```text
/lib/modules/$(uname -r)/updates/amdxdna-hardened/amdxdna.ko
```

Typical source tree on this machine (if you keep one):  
`~/Projects/kernel/xdna-driver-hardened` @ `905717f5…` + L2+L3 applied.

### Bare Kbuild (last resort)

Often needs `config_kernel.h` / `tools/configure_kernel.sh` — see patches README
footnote. Prefer A or B. Always `PATH=/usr/bin:/bin` on Garuda if wild is installed.

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

Persist lab default — **options only** (no softdep):

```bash
# Correct (options only):
sudo tee /etc/modprobe.d/amdxdna-hardened.conf <<'EOF'
# Hardened L2+L3 lab defaults (dragon-npu)
# Hard deps (gpu-sched, amd-pmf) come from modules.dep — no softdep.
options amdxdna exec_cmd_allow=0
EOF
```

| Do | Do not |
|----|--------|
| `options amdxdna exec_cmd_allow=0` only | Active `softdep … gpu_sched` / `softdep amdxdna …` lines |
| Hard deps via `modules.dep` after `depmod` | `options amdxdna exec_cmd_allow=1` in lab conf (verify **fails** closed) |

`scripts/verify_amdxdna_hardened_host.sh` matches **active** conf lines only
(comments stripped). A commented reminder about softdep is ignored; an
**uncommented** softdep or `exec_cmd_allow=1` is a hard FAIL.

Horizon lab state (2026-07-14/15): this conf is installed; runtime
`/sys/module/amdxdna/parameters/exec_cmd_allow` = **N**.

**Userspace L1** remains required regardless:

```bash
# Never enable unless intentional
# cargo … --features amd-xdna2,xdna2-dangerous-exec-cmd
# XDNA2_ALLOW_EXEC_CMD=1
```

### Firmware alias (`npu.dev.sbin` → `npu_7`)

OOT `amdxdna` from amd/xdna-driver requests **`npu.dev.sbin`**. Distro
`linux-firmware` ships **`npu_7.sbin`** (→ 1.1.2.64 on this host). Until a
staging-style multi-name fallback lands upstream, keep a symlink:

```bash
# Arch/Garuda paths are often the same inode under /lib and /usr/lib
FW=/lib/firmware/amdnpu/17f0_10
ls -la "$FW"
# expect something like:
#   npu_7.sbin.zst -> npu.sbin.1.1.2.64.zst
#   npu.dev.sbin.zst -> npu_7.sbin.zst   # lab alias

# create if missing (root):
# sudo ln -sfn npu_7.sbin.zst "$FW/npu.dev.sbin.zst"
```

Re-check after every `linux-firmware` upgrade (package may drop local aliases).

## 5. Post-install verify

```bash
# Preferred one-shot (modinfo path, exec_cmd_allow, firmware, modprobe.d, diagnose)
bash scripts/verify_amdxdna_hardened_host.sh

# Or manual:
bash scripts/diagnose_amdxdna.sh
# expect: not WEDGED_*; ideally LIKELY_OK_METADATA

modinfo amdxdna | grep -E 'filename|exec_cmd|vermagic|srcversion' || true
# filename should be …/updates/amdxdna-hardened/amdxdna.ko
# exec_cmd_allow appears only on L3-patched modules
cat /sys/module/amdxdna/parameters/exec_cmd_allow 2>/dev/null   # expect N or 0

# Safe checklist — no dangerous EXEC_CMD feature (opens /dev/accel once)
timeout 15 cargo run --features amd-xdna2 --example npu_direct_bench -- --checklist
```

Horizon lab (2026-07-14/15): L2+L3 **installed** under `updates/amdxdna-hardened/`,
`exec_cmd_allow=0`, CREATE_HWCTX **Pass** on Real arm. See
[`XDNA2_NEVER_AGAIN.md`](./XDNA2_NEVER_AGAIN.md) and
[`XDNA2_DIRECT_CAPABILITIES.md`](./XDNA2_DIRECT_CAPABILITIES.md).

### Post-reboot verification checklist (honest — may be unproven)

A successful install **this boot** does not prove `updates/` + conf + firmware
alias survive reboot until you run this **after** a real reboot. Leave the
NEVER_AGAIN “reboot persistence” checkbox **unchecked** until then.

```bash
# 0) After reboot — read-only first
uname -r
bash scripts/verify_amdxdna_hardened_host.sh --post-reboot
# --post-reboot is a label only (same checks as default; does not prove reboot).
# Wrong-family firmware aliases hard-fail always, with or without this flag.
# expect: exit 0; filename under updates/amdxdna-hardened; exec_cmd_allow N/0;
#         npu.dev.sbin → npu_7; diagnose LIKELY_OK_METADATA (or MODULE_NOT_LOADED then modprobe)

# 1) If module not loaded but .ko is present:
# sudo modprobe amdxdna
# bash scripts/verify_amdxdna_hardened_host.sh --post-reboot

# 2) Safe live smoke (only if verify/diagnose not WEDGED_*)
timeout 15 cargo run --features amd-xdna2 --example npu_direct_bench -- --checklist
# expect: open / get_info / create_hwctx pass; submit permanently gated

# 3) Optional evidence snapshot for upstream pack
bash scripts/gather_amdxdna_upstream_evidence.sh
```

If after reboot `modinfo` points at **in-tree/stock** `amdxdna.ko` (no
`updates/amdxdna-hardened`): re-run depmod / reinstall (next section) — do not
assume the wedge is “fixed” by stock alone.

### Kernel upgrade rebuild recipe (`uname -r` change)

OOT modules are **per-kernel**. After `linux-zen` (or headers) upgrade:

```bash
# 1) Headers matching the running kernel
uname -r
sudo pacman -S linux-zen-headers   # or the package that matches uname -r

# 2) Rebuild from patched tree (example lab path)
export PATH=/usr/bin:/bin    # avoid wild linker on Garuda
SRC="${HOME}/Projects/kernel/xdna-driver-hardened"   # or re-apply via install script
# If tree missing:
#   bash scripts/install_amdxdna_hardened.sh --apply --workdir "$SRC"
# Then follow that tree's CMake/Kbuild flow for amdxdna.ko

# 3) Install into updates/ for THIS uname -r (example)
KVER=$(uname -r)
sudo mkdir -p "/lib/modules/${KVER}/updates/amdxdna-hardened"
sudo cp -f path/to/amdxdna.ko "/lib/modules/${KVER}/updates/amdxdna-hardened/"
sudo depmod -a "$KVER"

# 4) Firmware alias recheck (new firmware package may have dropped it)
ls -la /lib/firmware/amdnpu/17f0_10/npu.dev.sbin.zst

# 5) Load + verify (only if not WEDGED_*)
# sudo modprobe -r amdxdna   # only if unused
# sudo modprobe amdxdna
bash scripts/verify_amdxdna_hardened_host.sh
```

DKMS path (if you wire `kernel/dkms/` fully): rebuild with `dkms install` for
the new kernel; still re-run `verify_amdxdna_hardened_host.sh`.

## 6. Rollback

1. `sudo dkms remove -m amdxdna-hardened -v 0.1.0 --all` (if DKMS path used).
2. Remove `/etc/modprobe.d/amdxdna-hardened.conf` if added.
3. Remove `/lib/modules/$(uname -r)/updates/amdxdna-hardened/` and `sudo depmod -a`.
4. Restore distro/in-tree `amdxdna.ko` (package reinstall or reboot to stock).
5. `sudo modprobe amdxdna` or reboot.
6. Diagnose / verify again.

If the host wedges during experiments, do **not** fight force-rmmod first —
reboot, then reinstall or roll back on a clean kernel.

## 7. What L4 does *not* claim

- That every host has L2/L3 (only **this** lab host after install + verify).
- **Reboot persistence** until post-reboot checklist is run and NEVER_AGAIN marked.
- Recovery of current D-state waiters without reboot.
- Automatic GitHub issue/PR filing (paste-ready only; human posts).
- Phase 1+ real EXEC_CMD / txn payload validation (see [`XDNA2_CMD_PAYLOAD.md`](./XDNA2_CMD_PAYLOAD.md)).

## Cross-links

| Doc | Why |
|-----|-----|
| [`XDNA2_NEVER_AGAIN.md`](./XDNA2_NEVER_AGAIN.md) | Policy + checkboxes (L1–L4) |
| [`XDNA2_DIRECT_CAPABILITIES.md`](./XDNA2_DIRECT_CAPABILITIES.md) | Host Pass table (hwctx Pass, submit gated) |
| [`XDNA2_CMD_PAYLOAD.md`](./XDNA2_CMD_PAYLOAD.md) | Phase 1 payload research (no default EXEC_CMD) |
| [`kernel/patches/README.md`](../kernel/patches/README.md) | Apply order, SHA, dry-run |
| [`kernel/dkms/README.md`](../kernel/dkms/README.md) | DKMS template + Arch notes |
| [`scripts/install_amdxdna_hardened.sh`](../scripts/install_amdxdna_hardened.sh) | Automated preflight + apply |
| [`scripts/verify_amdxdna_hardened_host.sh`](../scripts/verify_amdxdna_hardened_host.sh) | **Phase 0** read-only hardened host check |
| [`scripts/diagnose_amdxdna.sh`](../scripts/diagnose_amdxdna.sh) | Read-only wedge diagnose |
| [`XDNA2_RECOVERY_ENGINEERING.md`](./XDNA2_RECOVERY_ENGINEERING.md) | Optional non-reboot recovery |
| [`upstream/README.md`](./upstream/README.md) | **Upstream contribution pack** (plan + evidence + filing) |
| [`upstream/AMDXDNA_ISSUE_combined.md`](./upstream/AMDXDNA_ISSUE_combined.md) | One-shot issue body (do not auto-post) |
