# HITL milestones B & C (what you run)

Agent does **offline** work (contract, GT, pure-direct / APP_LOAD residual).
Public true post-patch dump research is **closed offline** (no live `run_` instr
BO API; registry = Transaction/cache only). You run **a few deliberate HITL
sessions** (typically **B** offline/optional OFM; **capture-instr** optional for
logs; then **one C** only when `RESIDUAL.md` has an **active** one-shot unlock
for a CONFIG_CU + PDI sandwich with **≥120s** outer).

**Unlock right now:** **None.** First `C_UNLOCK=` in `RESIDUAL.md` is
**`C_UNLOCK=2ae69036` CONSUMED** (flexml stock). Also CONSUMED: `97f38307`,
`11bc9016`, `3a70fabf`, `b17b0119`. Do **not** reopen. Live C needs a **new**
host-minted unlock + named offline reason. Not flag ping-pong.

Do **not** EXEC raw `lab_instr.bin`. Prefer wrappers below (ELF-only on C).

**Unpatched CLASS A / mere `diff rc=0` = body match only → ≠ Pass, ≠ `ran_on_npu`.**
Unlock **≠** Pass **≠** `ran_on_npu`. invent_pass=0.

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
# Optional L3 window (logs only; not current HV — no active unlock; all C one-shots CONSUMED):
# sudo sh -c 'echo 1 > /sys/module/amdxdna/parameters/exec_cmd_allow'
# bash tools/silicon_lab_operator.sh capture-instr   # or: capture_mladf_instr_vendor.sh
# sudo sh -c 'echo 0 > /sys/module/amdxdna/parameters/exec_cmd_allow'
bash tools/diff_vendor_instr_parity.sh   # offline; unpatched A = body match ≠ Pass
# CLASS_HINT A–E guides agent offline fix; capture ≠ C unlock
# Unlock: None active — see RESIDUAL.md / HITL_C_APP_LOAD_RECIPE.md
# CONSUMED: 2ae69036 (flexml), 97f38307, 11bc9016, 3a70fabf, b17b0119
```

Highest-value next (no live without new unlock): **Layer A offline** via
`bash tools/pass_unblock_once.sh` (`--status` / `--agent-offline` / `--ingest`),
**or** place capture under `testdata/xdna2_cmd/external_known_good/`, **or** prep
CONFIG_CU without applying (`config-cu`). Empty gate-only maintain stamps are
**frozen**. All prior C unlocks **CONSUMED**. Unlock **≠** Pass. INSTR_BO true
post-patch research **closed offline**. **Ban** more env C spam
(`XDNA2_LAB_KERNEL_ARG_LAYOUT` / `XDNA2_LAB_INSTR_SIZE_MODE` /
`XDNA2_LAB_RELA_MODE` trials closed).

**Optional OFM golden re-lock only (commented — not current HV; no active unlock):**

```bash
# sudo sh -c 'echo 1 > /sys/module/amdxdna/parameters/exec_cmd_allow'
# bash tools/hitl_b_gt_capture.sh --with-vendor --assume-l3-open
# sudo sh -c 'echo 0 > /sys/module/amdxdna/parameters/exec_cmd_allow'
```

---

## Milestone C — Pass apply (**no active unlock**; all prior one-shots CONSUMED)

**Canonical recipe (prereqs, CONFIG_CU pre-step, offline gates, sandwich, success, bans):**
[HITL_C_APP_LOAD_RECIPE.md](./HITL_C_APP_LOAD_RECIPE.md).

**Agent unlock:** **None active.** Latest CONSUMED one-shot was
`C_UNLOCK=2ae69036` (flexml). Also CONSUMED: `97f38307`, `11bc9016`,
`3a70fabf`, `b17b0119` — do not reopen. A **new** host-minted unlock is
permission for **one** sandwich that may use optional CONFIG_CU
(`XDNA2_LAB_PDI_BLOB` from `bash tools/operator_app_load_kit.sh config-cu` /
`a-prep`) with outer **≥120s**. **Not** permanent open season. Unlock **≠**
Pass / **≠** `ran_on_npu`. **Execute-allow** = driver `exec_cmd_allow`
(restore to 0); **userspace ALLOW** = `XDNA2_ALLOW_EXEC_CMD` (leave unset/0).
`tools/hitl_c2_once.sh` prints **executable** sandwich lines only when RESIDUAL
first `C_UNLOCK=` is active (agent does **not** complete apply).

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
bash tools/hitl_c2_once.sh --print-only  # expect no active unlock (all CONSUMED) unless RESIDUAL mint
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
match). Live value **only under a new active unlock** = CONFIG_CU or skip-PDI
A/B (not env spam). Soft offline: `c3_post_c2_classify.sh` + `c4_pass_check.sh`
(audit only — **≠ invent product Pass**). Not unpatched A / registry / parity
alone.

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
- **Highest-value now (no live):** `pass_unblock_once` offline / capture under
  `external_known_good/` / prep-only `config-cu`; empty maintain stamps
  **frozen**; all prior C unlocks **CONSUMED** including flexml `2ae69036`;
  post-patch dump research **closed offline**; capture-instr **optional** logs
  only
- C exit 0 only if product `ran_on_npu=true` (live decide); unlock **≠** Pass;
  unpatched A / parity / registry / residual_class / CLASSIFY_SOFT_OK /
  checklist / OFFLINE_GATES_OK / `C3_SKIP` / `C4_SKIP` / offline
  `C4_PASS_CANDIDATE` alone ≠ product Pass
- Never leave `exec_cmd_allow=1` (L3); never leave sticky `XDNA2_ALLOW_EXEC_CMD=1`
- Offline `c4_pass_check` **PassCandidate** audit ≠ invent product Pass
- Raw `.bin` EXEC → APP_LOAD_PDI_FAIL; not a useful apply
- Ban env flag ping-pong on live silicon
