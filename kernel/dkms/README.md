# DKMS scaffolding — hardened amdxdna (L4)

**Purpose:** make long-term install of L2+L3-patched `amdxdna` reproducible via
DKMS **without vendoring** the entire [amd/xdna-driver](https://github.com/amd/xdna-driver)
tree into dragon-npu.

| Artifact | Role |
|----------|------|
| [`dkms.conf`](./dkms.conf) | Template package name/version + MAKE stubs (`AUTOINSTALL=no`) |
| [`hooks/prepare_from_clone.sh`](./hooks/prepare_from_clone.sh) | Stage a **already-patched** clone for `/usr/src/…` |
| `staging/` | Local stage dir from the hook — **gitignored** (`kernel/dkms/staging/`) |

This is **ops packaging**, not a claim that the host already runs the hardened
module. Host install remains unchecked in [`docs/XDNA2_NEVER_AGAIN.md`](../../docs/XDNA2_NEVER_AGAIN.md)
until a human completes clean boot → build → modprobe → diagnose.

## Prerequisites

1. **Clean host** — no WEDGED_* from `bash scripts/diagnose_amdxdna.sh`.
2. **Kernel headers** matching `uname -r`.
3. **Patched source** — L2 then L3 on documented SHA (see
   [`../patches/README.md`](../patches/README.md)).

### Arch / Garuda (zen) notes

This lab class often runs a **zen** kernel (e.g. `7.1.3-zen1-2-zen`):

```bash
# Headers for the *running* kernel
uname -r
sudo pacman -S linux-zen-headers   # or linux-headers / linux-lts-headers

# DKMS tooling
sudo pacman -S dkms base-devel
```

If headers and `uname -r` disagree, DKMS build will fail — fix headers first.
Secure Boot: out-of-tree modules may need signing; that is distro/policy specific
and not automated here.

## Recommended flow

```bash
# From dragon-npu root — refuses if host is wedged
bash scripts/install_amdxdna_hardened.sh --apply --workdir /tmp/xdna-driver-hardened

# Stage for DKMS (no root). Default stage is under kernel/dkms/staging/
# (gitignored — do not commit). Fails if L2/L3 markers missing unless
# --allow-unpatched. No rsync --delete unless --replace-stage.
bash kernel/dkms/hooks/prepare_from_clone.sh /tmp/xdna-driver-hardened

# Root, clean host only — review dkms.conf MAKE paths for your tree first
STAGE=kernel/dkms/staging/amdxdna-hardened-0.1.0
sudo mkdir -p /usr/src/amdxdna-hardened-0.1.0
sudo rsync -a "$STAGE/" /usr/src/amdxdna-hardened-0.1.0/
sudo dkms add -m amdxdna-hardened -v 0.1.0
sudo dkms build -m amdxdna-hardened -v 0.1.0 -k "$(uname -r)"
sudo dkms install -m amdxdna-hardened -v 0.1.0 -k "$(uname -r)"
```

`AUTOINSTALL` defaults to **no** so a stub MAKE does not rebuild on every kernel
upgrade until you verify a first build.

If bare Kbuild fails (missing `config_kernel.h`, wrong `SRC_DIR`), **stop** and
use the **upstream xdna-driver CMake / package README** for that SHA, then either:

- Point DKMS at the upstream-built artifact path, or  
- Install the `.ko` via the project’s own install target (still OK for L4).

Do **not** invent a second broken Makefile.

### Stage path safety

- Default STAGE must stay under `kernel/dkms/staging/` and contain `amdxdna-hardened`.
- Outside that prefix: `--force-stage /abs/path`.
- Refuses `/` and (without force) system prefixes. Prefer no `--delete` unless
  `--replace-stage`.

## Lab modprobe after install

```bash
# Only if diagnose is not WEDGED_* and stock module can unload cleanly
sudo modprobe -r amdxdna     # never rmmod -f by default
sudo modprobe amdxdna        # L3 lab default: exec_cmd_allow=0

# Persist lab default (optional):
# echo 'options amdxdna exec_cmd_allow=0' | sudo tee /etc/modprobe.d/amdxdna-hardened.conf

bash scripts/diagnose_amdxdna.sh
modinfo amdxdna | grep -E 'filename|exec_cmd|vermagic' || true
```

Production / XRT needs `exec_cmd_allow=1`. Automatic open fail-fast latch still
needs non-default `tdr_dump_ctx=1` — see
[`../patches/amdxdna-open-failfast.md`](../patches/amdxdna-open-failfast.md).

## Uninstall / rollback

```bash
sudo dkms remove -m amdxdna-hardened -v 0.1.0 --all
# Restore distro/in-tree module path if needed, then:
sudo modprobe amdxdna
# or reboot
```

## Versioning

| Field | Suggested |
|-------|-----------|
| PACKAGE_NAME | `amdxdna-hardened` |
| PACKAGE_VERSION | `0.1.0` (bump when patches or base SHA change) |
| Base driver SHA | `905717f5cd470580054a1635b9eea24b84dfd67f` |

Record the **exact SHA + patch filenames** in your host runbook when you mark
NEVER_AGAIN “L2/L3 installed on host”.

## What this does not do

- Does not vendor full xdna-driver sources in git.
- Does not auto-sign modules for Secure Boot.
- Does not force-load over a wedged module.
- Does not post GitHub issues.
- Does not claim DKMS `MAKE[]` works on every xdna-driver release without edits.

## Cross-links

- Playbook: [`docs/XDNA2_HARDENED_INSTALL.md`](../../docs/XDNA2_HARDENED_INSTALL.md)
- Install script: [`scripts/install_amdxdna_hardened.sh`](../../scripts/install_amdxdna_hardened.sh)
- Patches: [`kernel/patches/README.md`](../patches/README.md)
- Policy: [`docs/XDNA2_NEVER_AGAIN.md`](../../docs/XDNA2_NEVER_AGAIN.md)
