# HITL milestones B & C (what you run)

Agent does **offline** work (Track A: contract, GT, pure-direct / **APP_LOAD**
residual). Public true post-patch dump research is **closed offline** (no live
`run_` instr BO API; registry = Transaction/cache only). You run **a few
deliberate HITL sessions** (typically **B** offline/optional OFM; **capture-instr**
optional for logs; then **C2** under one-shot **`C_UNLOCK=b17b0119`**) — not flag
ping-pong.

Do **not** EXEC raw `lab_instr.bin`. Prefer wrappers below (ELF-only on C).

**Unpatched CLASS A / mere `diff rc=0` = body match only → ≠ Pass, ≠ `ran_on_npu`.**
**One-shot unlock** (`C_UNLOCK=b17b0119`) = permission for **one** C2 only; unlock **≠** Pass.

Contract: [XDNA2_XRT_CONTRACT_MLADF.md](./XDNA2_XRT_CONTRACT_MLADF.md) · SSoT: [RESIDUAL.md](../RESIDUAL.md).

## Milestone B — Ground truth (unblocks agent)

**Offline only (no sudo, ~30s):**

```bash
cd ~/Projects/ai/dragon-npu
bash tools/hitl_b_gt_capture.sh
# or: bash tools/dump_mladf_gt_offline.sh
```

Writes pure-direct `gt_capture/instr_post_patch.bin` (synthetic paddrs).

**Optional vendor instruction dump** (one L3 window — only if classic 1688 body
missing, or to re-land classic/registry post_patch on disk; unpatched body class
may already be green via sibling):

```bash
sudo sh -c 'echo 1 > /sys/module/amdxdna/parameters/exec_cmd_allow'
bash tools/silicon_lab_operator.sh capture-instr   # or: capture_mladf_instr_vendor.sh
sudo sh -c 'echo 0 > /sys/module/amdxdna/parameters/exec_cmd_allow'
bash tools/diff_vendor_instr_parity.sh   # offline; unpatched A = body match ≠ Pass
# CLASS_HINT A–E guides agent offline fix; capture ≠ C unlock
# C2 one-shot C_UNLOCK=b17b0119 (operator; unlock ≠ Pass) — see HITL_C_APP_LOAD_RECIPE.md
```

Highest-value next after unpatched body green: operator **C2** under
`C_UNLOCK=b17b0119` for **APP_LOAD_PDI_FAIL** (INSTR_BO true post-patch research
**closed offline**; unlock **≠** Pass) — **ban** more env C applies
(`XDNA2_LAB_KERNEL_ARG_LAYOUT` / `XDNA2_LAB_INSTR_SIZE_MODE` / `XDNA2_LAB_RELA_MODE`
spam closed).

**Optional OFM golden re-lock only:**

```bash
sudo sh -c 'echo 1 > /sys/module/amdxdna/parameters/exec_cmd_allow'
bash tools/hitl_b_gt_capture.sh --with-vendor --assume-l3-open
sudo sh -c 'echo 0 > /sys/module/amdxdna/parameters/exec_cmd_allow'
```

---

## Milestone C — Pass apply (**UNLOCKED once** — `C_UNLOCK=b17b0119`)

**Canonical recipe (prereqs, offline gates, sandwich, success, bans):**
[HITL_C_APP_LOAD_RECIPE.md](./HITL_C_APP_LOAD_RECIPE.md).

**Agent unlock (one-shot):** `C_UNLOCK=b17b0119` in [RESIDUAL.md](../RESIDUAL.md).
Permission for **one** L3 sandwich + `hitl_c` only. **Not** permanent open
season. Unlock **≠** Pass / **≠** `ran_on_npu`. **L3** = `exec_cmd_allow`
(restore to 0); **userspace ALLOW** = `XDNA2_ALLOW_EXEC_CMD` (leave unset/0).
Further C needs a **new agent unlock** (evidence may justify; never auto-unlocks).
Prep stamp `de4d9fa4` added `tools/hitl_c2_once.sh` (does **not** complete C2
without operator sudo).

**Still not** a substitute unlock (and none equal Pass): offline `CONTRACT_OK` /
`OFFLINE_GATES_OK` / `run_offline_gates` green; vendor miss soft-skip; unpatched
CLASS A / mere `diff rc=0` (body match only); registry BO dump alone
(Transaction/cache image, not proven live-patched `run_` BO); soft
residual_class / `CLASSIFY_SOFT_OK` / `app_load_checklist`; `C3_SKIP` /
`C4_SKIP` / offline `C4_PASS_CANDIDATE` (snapshot audit only — **≠ invent product Pass**).
Post-C2 offline: `c3_post_c2_classify.sh` + `c4_pass_check.sh` + `run_offline_gates`.

**Offline prep only (no L3 / no apply):**

```bash
bash tools/hitl_c2_once.sh               # soft unlock NOTE + gates + preflight + sandwich print
# or piecemeal:
bash tools/run_offline_gates.sh          # OFFLINE_GATES_OK ≠ Pass
bash tools/silicon_contract_check.sh
bash tools/silicon_lab_preflight.sh --require-pass-assets
bash tools/hitl_c_pass_apply.sh --help   # docs only; no EXEC
# Fail-closed when L3 closed (expect exit 1):
# bash tools/hitl_c_pass_apply.sh --assume-l3-already-open
```

**Live sandwich (C2 — operator now; agent does not run):**

```bash
cd ~/Projects/ai/dragon-npu
# C_UNLOCK=b17b0119 one-shot; restore L3; ≠ ran_on_npu until Pass
sudo sh -c 'echo 1 > /sys/module/amdxdna/parameters/exec_cmd_allow'
# bash: export XDNA2_LAB_BO_DIAG=1
# fish: set -gx XDNA2_LAB_BO_DIAG 1
bash tools/hitl_c_pass_apply.sh --assume-l3-already-open
# or after L3 open: bash tools/hitl_c2_once.sh --apply-if-l3-open
sudo sh -c 'echo 0 > /sys/module/amdxdna/parameters/exec_cmd_allow'
cat /sys/module/amdxdna/parameters/exec_cmd_allow   # must be 0 or N
# Post-C2 offline (no L3) — C3 pipeline (C3_SKIP without new evidence):
bash tools/c3_post_c2_classify.sh
bash tools/run_offline_gates.sh
```

Forces **aiebu ELF** (refuses silent raw `.bin`). Expect detail:

- `BLOB size≈32264`, `aiebu_elf=1`, `aiebu_rela_patched`, `rela_count=12`
- `inst_size_mode=content` `content_size=1688B` `bo_size=4096B` `start_npu_size=1688B` (default after APP_LOAD residual)
- `qos_gops=100 qos_fps=30`, `tiles=32` (unless env override)

Exit 4 with `!ran_on_npu` / **APP_LOAD_PDI_FAIL** is the **expected current**
outcome until root cause fixed — not an offline-gate failure.

### Do **not** env-spam C applies

Env residual trials (`content` + `xrt3`) already **closed** with identical
`APP_LOAD_PDI_FAIL`. Unpatched classic body may already exist (CLASS A body
match). Next value: **operator C2** (unlocked `b17b0119`) then
`c3_post_c2_classify.sh` (C3 when `C3_EVIDENCE=1`) + `c4_pass_check.sh`
(offline **PassCandidate** audit only — **≠ invent product Pass**) + maintain
pure-direct offline gates. Not unpatched A / registry / parity alone.

**Ban live** `XDNA2_LAB_RELA_MODE=add` (and other env C spam) until the agent
explicitly unlocks a new offline-justified experiment — align RESIDUAL env HITL
**CLOSED**. Defaults: `content` + `count4` + `overwrite`.

**Paste only if not Pass:** `status`, `aiebu_*`, `inst_size_mode=`, `start_npu_size=`, `layout=`, `ert_state`, `ofm_stats`, `fw_status_hint`, and optional journal line.  
Or share `/tmp/dragon_npu_last_apply.json` / `testdata/xdna2_cmd/last_apply.json` if written.

**extract-paddrs does NOT auto re-patch** — set paddrs then dump:
```bash
bash tools/silicon_lab_operator.sh extract-paddrs
# bash: export XDNA2_GT_PADDRS_JSON=testdata/xdna2_cmd/gt_capture/vendor_paddrs_from_last_apply.json
# fish: set -gx XDNA2_GT_PADDRS_JSON testdata/xdna2_cmd/gt_capture/vendor_paddrs_from_last_apply.json
bash tools/dump_mladf_gt_offline.sh
```

---

## Also useful (no sudo)

```bash
bash tools/silicon_lab_operator.sh check    # offline contract (+ soft vendor parity)
bash tools/silicon_lab_operator.sh status
bash tools/dump_mladf_gt_offline.sh
bash tools/diff_vendor_instr_parity.sh --self
bash tools/silicon_lab_operator.sh extract-paddrs   # last_apply → paddrs JSON (no auto re-patch)
```

## Honesty

- B offline ≠ live XRT BO dump; B vendor OFM golden optional  
- **Highest-value now = operator C2** under `C_UNLOCK=b17b0119` (APP_LOAD residual; post-patch dump research **closed offline**); capture-instr **optional** for logs only; maintain pure-direct offline gates meanwhile  
- C exit 0 only if product `ran_on_npu=true` (live decide); **C2 unlocked once** under `C_UNLOCK=b17b0119` (unpatched A / parity / registry / residual_class / CLASSIFY_SOFT_OK / checklist / OFFLINE_GATES_OK / `C3_SKIP` / `C4_SKIP` / offline `C4_PASS_CANDIDATE` alone ≠ product Pass / ≠ further unlock)  
- Never leave `exec_cmd_allow=1` (L3); never leave sticky `XDNA2_ALLOW_EXEC_CMD=1`  
- Offline `c4_pass_check` **PassCandidate** audit ≠ invent product Pass  
- Raw `.bin` EXEC → APP_LOAD_PDI_FAIL; not a useful apply  
- Ban env flag ping-pong on live silicon
