# Upstream contribution pack (amdxdna / amd/xdna-driver)

Materials for contributing the **L2 + L3** amdxdna hardening work to
[amd/xdna-driver](https://github.com/amd/xdna-driver).

**Agents must not open GitHub issues or PRs unless a human explicitly asks.**

## Contribution status (Phase 0 truth)

| Item | Status |
|------|--------|
| Paste-ready issue bodies | In-tree under this directory |
| Oops evidence on disk | [`evidence/`](./evidence/) — primary attach: `ATTACH_fill_slot_oops_20260713_220051.txt` |
| Lab host L2+L3 | Installed under `updates/amdxdna-hardened/` (see [`LAB_EVIDENCE.md`](./LAB_EVIDENCE.md)) |
| **Issue filed** on amd/xdna-driver | **No** — human posts when ready ([`ISSUE_FILING_CHECKLIST.md`](./ISSUE_FILING_CHECKLIST.md)) |
| **PR to amd/xdna-driver** | **No** — human only |
| Personal fork WIP (L2 only) | [cryptoquick/xdna-driver](https://github.com/cryptoquick/xdna-driver) branch **`amdxdna/cmdlist-null-guards`** @ **`a9ef3c4d210d55e8d015d83fb6ebc0fc0b9242a4`** — evidence/WIP only; **not** a substitute for filing + PR with real DCO |
| L3 PR series | **Optional follow-up** after L2 interest; do not bundle with first PR unless maintainers ask |

Recommended order: **file issue → wait → PR1 = L2 only → optional PR2 = L3**.  
Details: [`CONTRIBUTION_PLAN.md`](./CONTRIBUTION_PLAN.md).

## Index

| Doc | Purpose |
|-----|---------|
| **[CONTRIBUTION_PLAN.md](./CONTRIBUTION_PLAN.md)** | How to contribute, order of operations, **where this work should live** |
| **[LAB_EVIDENCE.md](./LAB_EVIDENCE.md)** | Host evidence checklist + filled lab facts (horizon / Garuda) |
| **[ISSUE_FILING_CHECKLIST.md](./ISSUE_FILING_CHECKLIST.md)** | Step-by-step issue filing (human posts) |
| [AMDXDNA_ISSUE_combined.md](./AMDXDNA_ISSUE_combined.md) | **Paste-ready** single issue (L2+L3) |
| [AMDXDNA_ISSUE_cmdlist_null_open_wedge.md](./AMDXDNA_ISSUE_cmdlist_null_open_wedge.md) | L2-only detail |
| [AMDXDNA_ISSUE_L3_open_failfast_addendum.md](./AMDXDNA_ISSUE_L3_open_failfast_addendum.md) | L3 addendum / comment body |

## Patches (shippable)

| Patch | Layer |
|-------|--------|
| [`kernel/patches/0001-amdxdna-cmdlist-null-guards.patch`](../../kernel/patches/0001-amdxdna-cmdlist-null-guards.patch) | L2 null-guards |
| [`kernel/patches/0002-amdxdna-open-failfast-and-exec-gate.patch`](../../kernel/patches/0002-amdxdna-open-failfast-and-exec-gate.patch) | L3 fail-fast + `exec_cmd_allow` |
| [`kernel/patches/README.md`](../../kernel/patches/README.md) | Apply order, base SHA, design notes |

**Base:** `amd/xdna-driver` @ `905717f5cd470580054a1635b9eea24b84dfd67f`  
**Tree targeted by patches:** OOT `src/driver/amdxdna/` (not staging `drivers/accel/amdxdna/`).

## Related host docs (not for filing)

| Doc | Why |
|------|-----|
| [`docs/XDNA2_NEVER_AGAIN.md`](../XDNA2_NEVER_AGAIN.md) | Policy + L1–L4 checklist |
| [`docs/XDNA2_HARDENED_INSTALL.md`](../XDNA2_HARDENED_INSTALL.md) | Garuda install playbook + post-reboot / kernel upgrade |
| [`docs/XDNA2_DIRECT_CAPABILITIES.md`](../XDNA2_DIRECT_CAPABILITIES.md) | Host capability Pass table |
| [`docs/XDNA2_CMD_PAYLOAD.md`](../XDNA2_CMD_PAYLOAD.md) | Payload research (separate from upstream bug) |
| [`docs/XDNA2_RECOVERY_ENGINEERING.md`](../XDNA2_RECOVERY_ENGINEERING.md) | Wedge recovery notes |
| [`scripts/verify_amdxdna_hardened_host.sh`](../../scripts/verify_amdxdna_hardened_host.sh) | Read-only hardened host check |

## Evidence gather helper

```bash
bash scripts/verify_amdxdna_hardened_host.sh   # identity + gates + diagnose summary
bash scripts/gather_amdxdna_upstream_evidence.sh
# writes docs/upstream/evidence/HOST_SNAPSHOT_*.txt (gitignored by default under evidence/)
```
