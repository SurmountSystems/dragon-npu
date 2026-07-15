# Upstream contribution plan — amdxdna L2/L3 hardening

## Goal

Get defensive driver fixes into **amd/xdna-driver** (and eventually mainline-adjacent
trees) so bad/scaffold `EXEC_CMD` cannot oops `aie2_cmdlist_fill_slot` and leave
`amdxdna_drm_open` in D-state. Lab `exec_cmd_allow` + open fail-fast are secondary
hardening; discuss defaults with maintainers.

## Recommended sequence (you do the GitHub posts)

```text
1. Reboot proof (optional but nice)     → confirm updates/ module still loads
2. Capture oops attachment             → journal/dmesg from 2026-07-13 if still available
3. File ONE issue (combined body)      → docs/upstream/AMDXDNA_ISSUE_combined.md
4. Wait for maintainer feedback        → L2 interest? L3 shape? exec_cmd_allow default?
5. Fork amd/xdna-driver, rebase patches → real DCO Signed-off-by
6. PR1: L2 only (easiest review)       → 0001
7. PR2: L3 (or revised per feedback) → 0002; default exec_cmd_allow may flip to 1
8. Follow-ups                          → staging tree port, npu_7 firmware fallback, etc.
```

Do **not** bundle dragon-npu userspace, Garuda install scripts, or DKMS packaging
into the amd/xdna-driver PR. Upstream wants small, reviewable kernel diffs.

## What to file where

| Destination | What |
|-------------|------|
| **[amd/xdna-driver issues](https://github.com/amd/xdna-driver/issues)** | Bug + proposed fix (combined issue). **Primary.** |
| **amd/xdna-driver PRs** | After feedback; checkpatch + real DCO; OOT `src/driver/amdxdna/` first |
| **linux-next / drm** (later) | Only if maintainers route staging `drivers/accel/amdxdna/` that way |
| **dragon-npu** | Consumer notes, L1 gates, install playbook — **not** the long-term home of the driver |

## What you already have (checklist)

- [x] L2 patch (`0001-…`) dry-run + applied on base SHA  
- [x] L3 patch (`0002-…`) stacked on L2  
- [x] Design notes (`kernel/patches/amdxdna-*.md`)  
- [x] Paste-ready issue bodies under `docs/upstream/`  
- [x] Host: OOT L2+L3 built, installed under `updates/`, `exec_cmd_allow=0`  
- [x] Host: firmware alias `npu.dev.sbin` → stock `npu_7` (1.1.2.64)  
- [x] Host: open / GET_INFO / CREATE_HWCTX smoke OK on hardened module  
- [x] L1 userspace double gate in dragon-npu (separate from kernel PR)  
- [x] Phase 0 verify script: `scripts/verify_amdxdna_hardened_host.sh`  
- [ ] Reboot persistence proof (user deferred — see HARDENED_INSTALL post-reboot checklist)  
- [x] Raw oops attachment on disk: `evidence/ATTACH_fill_slot_oops_20260713_220051.txt`  
- [ ] Real DCO identity on commits (replace `devnull@localhost` before PR)  
- [ ] Issue filed (human)  
- [ ] PR opened (human) against **amd/xdna-driver**  
- [x] Optional WIP: fork branch `cryptoquick/xdna-driver` `amdxdna/cmdlist-null-guards` @ `a9ef3c4…` (L2 only; not a filed PR)

## How to turn patches into a PR (when ready)

```bash
# 1) Fork amd/xdna-driver on GitHub, then:
git clone git@github.com:<YOU>/xdna-driver.git
cd xdna-driver
git fetch origin
git checkout -b amdxdna-cmdlist-null-guards 905717f5cd470580054a1635b9eea24b84dfd67f
# or rebase onto current main if SHA is stale — re-apply by hand if needed

# 2) Apply with YOUR identity (edit From/Signed-off-by first, or use git commit -s)
git am --signoff /path/to/dragon-npu/kernel/patches/0001-amdxdna-cmdlist-null-guards.patch
# For L2-only PR stop here. For series:
git am --signoff /path/to/dragon-npu/kernel/patches/0002-amdxdna-open-failfast-and-exec-gate.patch

# 3) checkpatch (from their README)
cp tools/pre-commit .git/hooks/   # optional
./tools/codingsty_check.sh src/driver/amdxdna

# 4) Push + PR against amd/xdna-driver
# Cover letter: problem, lab env, stack L2 first, honest residuals
```

**DCO:** in-tree patches use placeholder `dragon-npu maintainers <devnull@localhost>`.
Replace with your real name/email and `Signed-off-by:` before any PR
(`git commit --amend -s` / re-am with fixed headers).

**checkpatch:** required by amd/xdna-driver contributor guidelines.

## Upstream vs lab: honesty points (put in issue/PR)

1. **L2** is pure correctness — strongest candidate.  
2. **L3** `exec_cmd_allow` default **0** is lab-friendly; upstream may want default **1**.  
3. Automatic unhealthy latch only arms with **`tdr_dump_ctx=1`** (stock default 0).  
4. Patches target **OOT** `src/driver/amdxdna/`; staging tree differs (firmware fallback
   `npu.dev.sbin` then `npu_7.sbin`). Port later if maintainers prefer staging.  
5. We do **not** claim full firmware cmd validation or killable SVA bind.  
6. Single lab oops capture — phrase as **observed**, not “always repro on all boards.”

## Where should this work live long-term?

| Home | Good for | Weak for |
|------|----------|----------|
| **amd/xdna-driver (upstream)** | Canonical fix; review; XRT users | Your Garuda packaging |
| **Personal fork of xdna-driver** | PRs, rebased series, CI on driver only | Userspace NPU API |
| **Separate `amdxdna-hardened` repo** | Lab install notes, updates/ recipes, evidence, patch series as sole focus | Duplicate if upstream merges quickly |
| **dragon-npu (here)** | L1 gates, NPU library, “how we hit the bug”, install playbook for this laptop | Kernel history noise; wrong audience for driver review |

### Recommendation

1. **Upstream truth:** patches + issue/PR on **amd/xdna-driver**. That is the only
   place the fix “counts” for everyone.  
2. **While out-of-tree:** keep a **thin tracking surface**:
   - Either leave `kernel/patches/` + `docs/upstream/` in dragon-npu (OK short-term), or  
   - Move kernel-only materials to a small sibling repo  
     e.g. `amdxdna-lab` / `xdna-driver-hardened-notes` with: patches, evidence,
     Garuda install, CONTRIBUTING → link from dragon-npu README.  
3. **Do not** grow dragon-npu into a second xdna-driver monorepo (no full vendor of
   amd sources, no competing DKMS packaging as the product).  
4. After merge upstream: dragon-npu keeps L1 + “use distro/upstream driver” docs;
   delete or archive local 0001/0002 as “merged as of SHA …”.

**Practical choice now:** stay in dragon-npu for the contribution pack (this
directory) until the issue is filed and a PR is open; if kernel work continues
for weeks after that, split to a sibling repo so dragon-npu stays a Rust NPU lib.

## What not to contribute upstream

- `scripts/install_amdxdna_hardened.sh`, rescue kmod, diagnose scripts (reference only)  
- dragon-npu L1 Cargo feature / env gates (mention as userspace defense, not part of PR)  
- `softdep` modprobe.d recipes, wild-linker PATH hacks (distro notes)  
- Forcing `exec_cmd_allow=0` as production default without maintainer buy-in  

## After reboot (your TODO)

```bash
bash scripts/verify_amdxdna_hardened_host.sh --post-reboot
# or manual:
modinfo amdxdna | rg 'filename|srcversion|exec_cmd'
bash scripts/diagnose_amdxdna.sh
timeout 15 cargo run --features amd-xdna2 --example npu_direct_bench -- --checklist
bash scripts/gather_amdxdna_upstream_evidence.sh
```

If stock comes back: re-check `updates/` path and `depmod` (see HARDENED_INSTALL § kernel upgrade).

## Optional L3 PR series

L3 (`0002-…` open fail-fast + `exec_cmd_allow`) is **optional** as a second PR after
maintainers accept or comment on L2. Do **not** open L3 PR or push series unless a
human decides. Upstream may want `exec_cmd_allow` default **1**; lab keeps **0**.
