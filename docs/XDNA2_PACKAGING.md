# XDNA2 packaging: DKMS long-term vs upstream

Decision tree for shipping hardened `amdxdna` (L2 null-guards + L3 open
fail-fast / `exec_cmd_allow`). Complements
[`XDNA2_HARDENED_INSTALL.md`](./XDNA2_HARDENED_INSTALL.md) and
[`kernel/dkms/README.md`](../kernel/dkms/README.md).

**Honesty:** patch materials in-tree ≠ host fixed. L2/L3 must be **built,
installed, and verified** on the machine. Agents do not claim reboot-proof
without the post-reboot checklist.

## Decision tree

```text
                    ┌─────────────────────────────┐
                    │ Need L2/L3 on this host?    │
                    └─────────────┬───────────────┘
                                  │
              ┌───────────────────┴───────────────────┐
              ▼                                       ▼
     Upstream already has                    Still out-of-tree
     equivalent guards?                      (today: yes for full L2+L3)
              │                                       │
         use distro /                          Lab path:
         mainline module                       updates/ or DKMS bridge
         (no dragon-npu                        (this repo's patches)
          OOT needed)
```

### Path A — Lab / out-of-tree (current default story)

| Step | Action |
|------|--------|
| 1 | Clean boot; `bash scripts/diagnose_amdxdna.sh` not `WEDGED_*` |
| 2 | Apply L2 then L3 @ documented SHA (`kernel/patches/README.md`) |
| 3 | Build via upstream xdna-driver flow **or** install under `updates/amdxdna-hardened/` |
| 4 | Lab load: `exec_cmd_allow=0`; verify with `scripts/verify_amdxdna_hardened_host.sh` |
| 5 | On **kernel bump**: rebuild OOT / reinstall (zen headers match `uname -r`) |

**When to use `kernel/dkms/` templates**

- You want DKMS package name/version scaffolding without vendoring full xdna-driver.
- Use `hooks/prepare_from_clone.sh` on an **already-patched** clone.
- `AUTOINSTALL=no` until a first build is proven.
- Do **not** use DKMS as an excuse to `rmmod -f` over a wedge.

**When not to full-vendor xdna-driver into git**

- Tree is large; stage under `kernel/dkms/staging/` (gitignored).
- Upstream CMake is the authority for that SHA’s build.

### Path B — Long-term: upstream merge → distro/mainline

| Goal | Action |
|------|--------|
| Land L2 | Smallest review unit — null-guards only ([CONTRIBUTION_PLAN](./upstream/CONTRIBUTION_PLAN.md)) |
| Land L3 | After L2; discuss `exec_cmd_allow` default with maintainers |
| Distro | Prefer packaged mainline/OOT once merged; drop private hardened rebuild |
| DKMS | **Bridge only** while out-of-tree; retire private DKMS when distro module is enough |

**Do not claim** amd/xdna-driver or mainline has merged our patches unless you
verify the tree. Optional WIP notes (e.g. community forks) are not “upstream
done.”

### Path C — Upstream-only (no L2/L3)

Accept stock amdxdna risks: scaffold EXEC_CMD can still oops; L1 userspace
double gate in dragon-npu remains **mandatory** regardless
([`XDNA2_NEVER_AGAIN.md`](./XDNA2_NEVER_AGAIN.md)).

## Materials checklist (repo)

| Item | Location |
|------|----------|
| L2/L3 patches | `kernel/patches/0001-…`, `0002-…` |
| Install playbook | `docs/XDNA2_HARDENED_INSTALL.md` |
| DKMS template | `kernel/dkms/` |
| Host verify (read-only) | `scripts/verify_amdxdna_hardened_host.sh` |
| Upstream filing | `docs/upstream/` |

## Cross-links

- Never-again policy: [`XDNA2_NEVER_AGAIN.md`](./XDNA2_NEVER_AGAIN.md)
- Contribution sequence: [`upstream/CONTRIBUTION_PLAN.md`](./upstream/CONTRIBUTION_PLAN.md)
- Crate packaging / MSRV: [`PACKAGING.md`](./PACKAGING.md)
