# Lab evidence — horizon (Garuda / XDNA2)

Filled facts for upstream issue attachments. **Do not paste secrets** (full
hostname is fine; redact serials if any). Update after reboot.

## Machine

| Item | Value |
|------|--------|
| Host | horizon (Garuda, Arch base) |
| Kernel | `7.1.3-zen1-2-zen` |
| Laptop | Lenovo ThinkPad P14s Gen 6 AMD (21QLCTO1WW) |
| CPU | AMD Ryzen AI 7 PRO 350 |
| NPU PCI | **0000:c6:00.1** · **1022:17f0** · subsys 17AA:512E |
| Stock driver (distro) | in-tree `linux-zen` `amdxdna` · srcversion `B9ED3E2DD923D9C8E3F4EDB` · DRM **0.8.0** |
| Hardened driver (lab) | OOT from amd/xdna-driver @ `905717f5…` + L2+L3 · srcversion `E36F27FC09FB5CA0C0C3E87` · DRM **1.0.0** |
| Install path | `/lib/modules/$(uname -r)/updates/amdxdna-hardened/amdxdna.ko` |
| Lab param | `exec_cmd_allow=0` via `/etc/modprobe.d/amdxdna-hardened.conf` (no softdep) |
| Firmware | linux-firmware `amdnpu/17f0_10/npu_7.sbin` → 1.1.2.64; OOT alias `npu.dev.sbin.zst` → `npu_7.sbin.zst` |

## Incident (2026-07-13) — reason for L2

| Item | Observation |
|------|-------------|
| Trigger | Lightweight userspace scaffold `DRM_IOCTL_AMDXDNA_EXEC_CMD` (research, not XRT); process name in related WARN: `real_xdna2_on_t` |
| Kernel fault | **`BUG: kernel NULL pointer dereference`** · RIP **`aie2_cmdlist_fill_slot+0x26/0x130 [amdxdna]`** · address **`0x50`** (NULL+0x50) · workqueue **`amdxdna_js` / `drm_sched_run_job_work`** · caller **`aie2_cmdlist_single_execbuf` → `aie2_sched_job_run`** |
| Registers (primary) | **RAX=0**, CR2=`0x50`; instruction stream loads via `cmd_abo` then `mov … 0x50(%rax)` (matches `cmd_abo->client` NULL → deref `client->…` at +0x50) |
| Captures in journal | **Twice same day:** 2026-07-13 **03:28:44** and **22:00:51** MDT (boots -2 and -1 as of 2026-07-14) |
| Aftermath | Open wedge / high refcnt / reboot recovery (documented in NEVER_AGAIN); same evening: rescue kmod taint |
| Soften for issue | Lab / research scaffold — not “always repro on XRT”; still a driver bug to return errno not oops |

### Attachment files (on disk — gitignored under `evidence/`)

| File | Use |
|------|-----|
| **`docs/upstream/evidence/ATTACH_fill_slot_oops_20260713_220051.txt`** | **Primary** — attach to GitHub issue |
| `docs/upstream/evidence/ATTACH_fill_slot_oops_20260713_032844.txt` | Second capture same day (optional) |
| `docs/upstream/evidence/OOPS_INDEX.md` | Index of raw dumps |
| Full windows | `OOPS_20260713_*_kernel.txt` |

Re-extract if needed:

```bash
journalctl -b -1 --since '2026-07-13 22:00:40' --until '2026-07-13 22:01:30'
# or: bash scripts/gather_amdxdna_upstream_evidence.sh
```

## Post-hardening validation (2026-07-14 / rechecked 2026-07-15)

| Check | Result |
|-------|--------|
| Module identity | `updates/amdxdna-hardened/amdxdna.ko`, `exec_cmd_allow` present, value **N**/0 |
| Diagnose | `LIKELY_OK_METADATA`, refcnt 0, no D-state waiters |
| dragon-npu checklist | open **pass**, get_info **pass**, CREATE_HWCTX **hwctx_ok** / **Pass**, EXEC_CMD **compile-gated** |
| Read-only verify script | `bash scripts/verify_amdxdna_hardened_host.sh` (Phase 0) |
| Side note | Stock in-tree path previously saw CREATE_HWCTX **EINVAL** (map host buffer / opcode 0x106); OOT 1.0.0 + same firmware blob achieved hwctx_ok — **separate** from L2/L3 story; optional context only |
| Reboot persistence | **Not proven** — run `verify_amdxdna_hardened_host.sh --post-reboot` after a real reboot before claiming |

## Upstream fork / PR status (do not auto-file)

| Item | Value |
|------|--------|
| Issue on amd/xdna-driver | **Not filed** (human; [`ISSUE_FILING_CHECKLIST.md`](./ISSUE_FILING_CHECKLIST.md)) |
| PR on amd/xdna-driver | **Not opened** |
| WIP fork branch (L2) | `https://github.com/cryptoquick/xdna-driver/tree/amdxdna/cmdlist-null-guards` @ `a9ef3c4d210d55e8d015d83fb6ebc0fc0b9242a4` |
| L3 as separate PR | Optional follow-up only |
| Oops attach path | `docs/upstream/evidence/ATTACH_fill_slot_oops_20260713_220051.txt` |

## Build notes (for packagers / self, not required in issue)

| Footgun | Mitigation |
|---------|------------|
| `/usr/local/bin/ld` → **wild** breaks `module.lds` | `PATH=/usr/bin:/bin` for configure_kernel + make |
| OOT requests `npu.dev.sbin` only | Symlink to stock `npu_7.sbin.zst` or port staging multi-name fallback |
| Bare `insmod` | Needs `amd_pmf` loaded; prefer `depmod` + `modprobe` |
| `softdep … gpu_sched` | Causes `modprobe -r` FATAL while amdgpu holds gpu_sched — **do not** use |

## Patch files to attach or link

- `kernel/patches/0001-amdxdna-cmdlist-null-guards.patch`  
- `kernel/patches/0002-amdxdna-open-failfast-and-exec-gate.patch`  
- Base SHA: `905717f5cd470580054a1635b9eea24b84dfd67f`  
- Apply order: **0001 then 0002** on OOT tree  

GitHub: attach as files on the issue, or push a branch on your fork and link.

## Snapshot helper

```bash
bash scripts/gather_amdxdna_upstream_evidence.sh
```
