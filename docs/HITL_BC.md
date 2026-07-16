# HITL milestones B & C (what you run)

Agent does offline work. You run **at most two** deliberate sessions.

## Milestone B — Ground truth (unblocks agent)

**Offline only (no sudo, ~30s):**

```fish
cd ~/Projects/ai/dragon-npu
bash tools/hitl_b_gt_capture.sh
```

Writes `testdata/xdna2_cmd/gt_capture/` (patch oracle: post-patch instr/ctrl + MANIFEST).

**Optional vendor golden re-lock (sudo once):**

```fish
sudo sh -c 'echo 1 > /sys/module/amdxdna/parameters/exec_cmd_allow'
bash tools/hitl_b_gt_capture.sh --with-vendor --assume-l3-open
sudo sh -c 'echo 0 > /sys/module/amdxdna/parameters/exec_cmd_allow'
```

Then tell the agent **“B done”** (or just leave it — agent can read `gt_capture/`).

---

## Milestone C — Pass apply (only when agent says ready)

```fish
cd ~/Projects/ai/dragon-npu
sudo sh -c 'echo 1 > /sys/module/amdxdna/parameters/exec_cmd_allow'
set -x XDNA2_LAB_BO_DIAG 1
bash tools/hitl_c_pass_apply.sh --assume-l3-already-open
sudo sh -c 'echo 0 > /sys/module/amdxdna/parameters/exec_cmd_allow'
```

Forces **aiebu ELF** (refuses silent raw `.bin`). Exit 4 with `!ran_on_npu` is common until Pass.

**Paste only if not Pass:** `status`, `aiebu_*`, `ert_state`, `ofm_stats`, and optional journal line.

---

## Also useful (no sudo)

```fish
bash tools/silicon_lab_operator.sh check    # offline contract
bash tools/silicon_lab_operator.sh status
bash tools/dump_mladf_gt_offline.sh
```

## Honesty

- B offline ≠ live XRT BO dump; B vendor = OFM golden only  
- C exit 0 only if `ran_on_npu=true`  
- Never leave `exec_cmd_allow=1`
