# HITL milestones B & C (what you run)

Agent does **offline** work (Track A: contract, GT, pure-direct / **APP_LOAD**
residual). Public true post-patch dump research is **closed offline** (no live
`run_` instr BO API; registry = Transaction/cache only). You run **a few
deliberate HITL sessions** (typically **B** offline/optional OFM; **capture-instr**
optional for logs; then **one C** under active **`C_UNLOCK=3a70fabf`** for a
CONFIG_CU + PDI sandwich). Prior **`C_UNLOCK=b17b0119`** is **CONSUMED**
(historical C2 2026-07-18) — do not reopen under it. Not flag ping-pong.

Do **not** EXEC raw `lab_instr.bin`. Prefer wrappers below (ELF-only on C).

**Unpatched CLASS A / mere `diff rc=0` = body match only → ≠ Pass, ≠ `ran_on_npu`.**
**Active one-shot** `C_UNLOCK=3a70fabf` = agent unlock for **one** CONFIG_CU apply.
Prior `b17b0119` = **CONSUMED**. Unlock **≠** Pass.

Contract: [XDNA2_XRT_CONTRACT_MLADF.md](./XDNA2_XRT_CONTRACT_MLADF.md) · single source of truth: [RESIDUAL.md](../RESIDUAL.md).

## Milestone B — Ground truth (unblocks agent)

**Offline only (no sudo, ~30s) — Milestone B does not open L3:**

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
# Optional L3 window (logs only; not current HV — active HV = CONFIG_CU under
# C_UNLOCK=3a70fabf; prior b17b0119 CONSUMED):
# sudo sh -c 'echo 1 > /sys/module/amdxdna/parameters/exec_cmd_allow'
# bash tools/silicon_lab_operator.sh capture-instr   # or: capture_mladf_instr_vendor.sh
# sudo sh -c 'echo 0 > /sys/module/amdxdna/parameters/exec_cmd_allow'
bash tools/diff_vendor_instr_parity.sh   # offline; unpatched A = body match ≠ Pass
# CLASS_HINT A–E guides agent offline fix; capture ≠ C unlock
# Active C: C_UNLOCK=3a70fabf — see HITL_C_APP_LOAD_RECIPE.md
# Historical C2: C_UNLOCK=b17b0119 CONSUMED
```

Highest-value next: **one CONFIG_CU live sandwich** under active
`C_UNLOCK=3a70fabf` (`a-prep` → `XDNA2_LAB_PDI_BLOB` → L3 → restore), or place
capture under `testdata/xdna2_cmd/external_known_good/`. Empty gate-only maintain
stamps are **frozen**. Prior `C_UNLOCK=b17b0119` **CONSUMED**. Unlock **≠** Pass.
INSTR_BO true post-patch research **closed offline**. **Ban** more env C spam
(`XDNA2_LAB_KERNEL_ARG_LAYOUT` / `XDNA2_LAB_INSTR_SIZE_MODE` /
`XDNA2_LAB_RELA_MODE` trials closed).

**Optional OFM golden re-lock only (commented — not current HV; active HV =
CONFIG_CU under `3a70fabf`; prior `b17b0119` CONSUMED):**

```bash
# sudo sh -c 'echo 1 > /sys/module/amdxdna/parameters/exec_cmd_allow'
# bash tools/hitl_b_gt_capture.sh --with-vendor --assume-l3-open
# sudo sh -c 'echo 0 > /sys/module/amdxdna/parameters/exec_cmd_allow'
```

---

## Milestone C — Pass apply (**active** `C_UNLOCK=3a70fabf`; prior `b17b0119` CONSUMED)

**Canonical recipe (prereqs, CONFIG_CU pre-step, offline gates, sandwich, success, bans):**
[HITL_C_APP_LOAD_RECIPE.md](./HITL_C_APP_LOAD_RECIPE.md).

**Agent unlock (active one-shot):** `C_UNLOCK=3a70fabf` — permission for **one**
L3 sandwich that uses optional CONFIG_CU (`XDNA2_LAB_PDI_BLOB` from
`bash tools/operator_app_load_kit.sh a-prep`). **Not** permanent open season.
Unlock **≠** Pass / **≠** `ran_on_npu`. **Execute-allow** = driver
`exec_cmd_allow` (restore to 0); **userspace ALLOW** = `XDNA2_ALLOW_EXEC_CMD`
(leave unset/0). Prior `C_UNLOCK=b17b0119` **CONSUMED** — do not reopen under it.
`tools/hitl_c2_once.sh` prints **executable** sandwich lines when RESIDUAL first
`C_UNLOCK=` is active (agent does **not** complete apply without operator sudo).

**Still not** a substitute unlock (and none equal Pass): offline `CONTRACT_OK` /
`OFFLINE_GATES_OK` / `run_offline_gates` green; vendor miss soft-skip; unpatched
CLASS A / mere `diff rc=0` (body match only); registry BO dump alone
(Transaction/cache image, not proven live-patched `run_` BO); soft
residual_class / `CLASSIFY_SOFT_OK` / `app_load_checklist`; `C3_SKIP` /
`C4_SKIP` / offline `C4_PASS_CANDIDATE` (snapshot audit only — **≠ invent product Pass**).
Post-apply offline: `c3_post_c2_classify.sh` + `c4_pass_check.sh` + `run_offline_gates`.

**Offline prep + sandwich print (no L3 from agent):**

```bash
bash tools/operator_app_load_kit.sh a-prep   # dump PDI; print XDNA2_LAB_PDI_BLOB=
# export XDNA2_LAB_PDI_BLOB=…   # from a-prep; optional XDNA2_LAB_CU_FUNC=0
bash tools/hitl_c2_once.sh               # soft unlock NOTE + gates + preflight + sandwich print
# or:
bash tools/hitl_c2_once.sh --print-only  # expect active C_UNLOCK=3a70fabf + real L3 lines
bash tools/run_offline_gates.sh          # OFFLINE_GATES_OK ≠ Pass
bash tools/silicon_contract_check.sh
bash tools/silicon_lab_preflight.sh --require-pass-assets
bash tools/hitl_c_pass_apply.sh --help   # docs only; no EXEC
# Fail-closed when L3 closed (expect exit 1):
# bash tools/hitl_c_pass_apply.sh --assume-l3-already-open
```

**HISTORICAL / CONSUMED under `b17b0119` only (record — do not re-run under that stamp):**

```bash
# HISTORICAL / CONSUMED — do not re-run L3 under b17b0119
# C_UNLOCK=b17b0119 one-shot CONSUMED
# (record only) sudo / hitl_c / restore
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
match). Live value under active unlock = CONFIG_CU sandwich (not env spam). Soft
offline: `c3_post_c2_classify.sh` + `c4_pass_check.sh` (audit only — **≠ invent
product Pass**). Not unpatched A / registry / parity alone.

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
- **Highest-value now = one CONFIG_CU sandwich** under active `C_UNLOCK=3a70fabf`
  (or capture under `external_known_good/`); empty maintain stamps **frozen**;
  prior `b17b0119` **CONSUMED**; post-patch dump research **closed offline**;
  capture-instr **optional** logs only
- C exit 0 only if product `ran_on_npu=true` (live decide); unlock **≠** Pass;
  unpatched A / parity / registry / residual_class / CLASSIFY_SOFT_OK /
  checklist / OFFLINE_GATES_OK / `C3_SKIP` / `C4_SKIP` / offline
  `C4_PASS_CANDIDATE` alone ≠ product Pass
- Never leave `exec_cmd_allow=1` (L3); never leave sticky `XDNA2_ALLOW_EXEC_CMD=1`  
- Offline `c4_pass_check` **PassCandidate** audit ≠ invent product Pass  
- Raw `.bin` EXEC → APP_LOAD_PDI_FAIL; not a useful apply  
- Ban env flag ping-pong on live silicon
