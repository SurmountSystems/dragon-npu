# Human checklist — file amd/xdna-driver issue

Do **not** auto-post. Use this when you are ready.

## Before you open the browser

1. [ ] Read [`CONTRIBUTION_PLAN.md`](./CONTRIBUTION_PLAN.md) once.  
2. [ ] Optional: reboot + confirm hardened module still loads:  
   `bash scripts/verify_amdxdna_hardened_host.sh --post-reboot` (`modinfo` → `updates/`).  
3. [ ] Run `bash scripts/gather_amdxdna_upstream_evidence.sh`.  
4. [x] **Oops captured** (2026-07-14 hunt):  
   `docs/upstream/evidence/ATTACH_fill_slot_oops_20260713_220051.txt`  
   (second capture: `…_032844.txt`; see `LAB_EVIDENCE.md`).  
5. [ ] Open [`AMDXDNA_ISSUE_combined.md`](./AMDXDNA_ISSUE_combined.md) — copy **Title** + **Body** only (from `## Title` through patch/DCO note).  
6. Optional context: WIP L2 branch on fork  
   `cryptoquick/xdna-driver` `amdxdna/cmdlist-null-guards` @ `a9ef3c4…` — link only if useful; prefer attaching in-repo `0001` patch.  

## On GitHub

1. Go to <https://github.com/amd/xdna-driver/issues/new>.  
2. Paste title + body.  
3. Attach:  
   - **`ATTACH_fill_slot_oops_20260713_220051.txt`** (required)  
   - `kernel/patches/0001-….patch` and optionally `0002-….patch`  
   - optional: second oops + host snapshot  
4. Labels: bug / driver if available.  
5. Submit as **you** (not a bot).  

## After filing

1. [ ] Paste issue URL into this file or AGENTS.md / NEVER_AGAIN.  
2. [ ] Mark NEVER_AGAIN “Issue filed” checkbox.  
3. [ ] Wait for maintainer feedback before large PR work.  
4. [ ] Prefer **PR1 = L2 only** unless they ask for both.  

## Do not

- Claim “always reproduces on all kernels” without multi-machine proof.  
- Demand `exec_cmd_allow=0` as upstream default.  
- Include dragon-npu Cargo features as part of the kernel fix.  
- Force-push or open PR with `devnull@localhost` Signed-off-by.  
