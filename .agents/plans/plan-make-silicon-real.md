# Plan: Make silicon Pass real (`ran_on_npu`)

**Date:** 2026-07-20  
**Mode:** Formal plan (explore → draft → **present / await approve** → act)  
**Canonical plan file:**  
`/home/hunter/.grok/sessions/%2Fhome%2Fhunter%2FProjects%2Fai%2Fdragon-npu/019f6476-0a84-70a0-a971-be89fa1011fe/plan.md`  
**Workspace copy (after approval only):** `.agents/plans/plan-make-silicon-real.md`

**Workflow status**

| Phase | Status |
|-------|--------|
| understand | done |
| explore | done (RESIDUAL SSoT, unlock, tools, host, last_apply) |
| draft | done (this file) |
| present | **approved** |
| implement | handoff — **human live sandwich next**; agent waits for evidence |

---

## Context

### Goal (plain English)

**Make it real** means: firmware **accepts** the job, ERT finishes **Completed**, OFM is **non-zero and matches golden**, and the product path sets **`ran_on_npu=true`** via live `decide_ran_on_npu` — not offline scorecards, not unlock stamps, not “gates green.”

We already have userspace packaging that looks correct (aiebu ELF + 12 relas, Dev instr BO, real ioctl). Firmware still rejects load with **`0x3000003` APP_LOAD_PDI_FAIL**. Offline agent half of residual research is **largely done**. The bottleneck is **one supervised live try** (and whatever evidence-driven delta follows).

### What “real” is (exact Pass bar — P6)

Claim product Pass / `ran_on_npu` **only** when **all** hold on a **live** apply:

1. **ERT** state **Completed** (`ert_state=4` / status exact `ok_npu_verified` — not “contains Completed”).
2. **OFM** non-zero and **allclose** vs golden via product `decide_ran_on_npu` (offline snapshot alone cannot invent allclose).
3. **`real_ioctl=true`** (not stub path).
4. **`json_ran_on_npu=true`** only when the timed product path set it.
5. Lab process + L3 restore honest (prefer exit 0 with L3 closed after Pass).

Anything short of that = residual still open. Offline `C4_PASS_CANDIDATE` / `P6_OFFLINE_CANDIDATE` **≠** product Pass.

### Current ground truth (this host / tree)

| Fact | Value |
|------|--------|
| Residual class | Userspace ELF+rela OK; FW **APP_LOAD** `0x3000003` (+ often `0x4000003`) |
| last_apply | `testdata/xdna2_cmd/last_apply.json` — `ran_on_npu=false`, `ert_cmd_error`, OFM zeros (2026-07-18 baseline body) |
| Active unlock | **`C_UNLOCK=3a70fabf`** one-shot for **one** CONFIG_CU + `XDNA2_LAB_PDI_BLOB` sandwich — **still unused** |
| Prior unlock | `b17b0119` **CONSUMED** — never reopen under it |
| L3 on host | `exec_cmd_allow=N` (closed) |
| Device | `/dev/accel/accel0` present; KMQ `device_type=0` |
| Capture dir | `testdata/xdna2_cmd/external_known_good/` — PROVENANCE only, **BINS=0** |
| Working tree | Dirty: P1–P6 agent work uncommitted (CONFIG_CU, kit, p5/p6, docs); branch `rust` ahead of origin by 1 |

### What is already done (do not rebuild)

| Area | Status |
|------|--------|
| Optional lab **CONFIG_CU** after CREATE_HWCTX | Landed: `XDNA2_LAB_PDI_BLOB` → PDI BO → `CONFIG_HWCTX(CONFIG_CU)` in `direct.rs` (default off) |
| Operator kit | `tools/operator_app_load_kit.sh` (`a-prep`, `b-*`, offline-gates) |
| PDI dump | `tools/dump_xclbin_pdi.sh` (host AIE2P research bytes; not known-good matmul) |
| Live apply helpers | `tools/hitl_c2_once.sh`, `tools/hitl_c_pass_apply.sh`, recipe `docs/HITL_C_APP_LOAD_RECIPE.md` |
| Post-shot interpret | `tools/p5_post_c2_interpret.sh` (classify + C3 + C4 + P6) |
| True-Pass criteria offline | `tools/p6_true_pass_check.sh` + example — locked fixture → NOT_PASS / SKIP |
| Privilege / product-vs-lab docs | NEVER_AGAIN + PACKAGING + RESIDUAL English (P2) |
| Empty reconfirm / empty implement | **Frozen** (`AGENTS.md` §8–9) |

### Constraints

- Agent default = **offline**; no casual sudo; never default `XDNA2_ALLOW_EXEC_CMD=1`.
- Live apply only under **active** unlock + human L3 sandwich; unlock ≠ Pass.
- Prefer aiebu **ELF** EXEC; refuse silent raw txn product path.
- No invent capture bytes / Pass / `ran_on_npu`.
- No empty gate mills without new evidence.
- GPG-signed commits only (human TTY for passphrase); agent never disables signing.
- Product users: **no per-run sudo** (admin installs driver/udev once). Lab L3 flip is research-only.

### Non-goals (this plan)

- Another offline reconfirm of gates with no new last_apply.
- Reopening under `b17b0119`.
- Minting a second unlock **before** `3a70fabf` is used (or without new offline delta after it is used and fails).
- Inventing `external_known_good` bins.
- Full Colibri product integration before Pass honesty is real.
- Switching this host to UMQ (chip is KMQ).
- Committing unsigned.

### Assumptions

- Host stays PCI `1022:17f0` / NPU4 → KMQ.
- Wrong overlay PDI may still fail load — still useful **evidence** (config_cu token + journal), not Pass.
- After `3a70fabf` is used, further live applies need a **new** agent unlock justified by evidence — never auto.

---

## Approach

### Recommended path (critical path first)

```text
                    ┌─────────────────────────────────────┐
                    │  Human: one live CONFIG_CU sandwich │
                    │  under C_UNLOCK=3a70fabf             │
                    └─────────────────┬───────────────────┘
                                      │
                    share last_apply + journal + L3 restored
                                      │
                    ┌─────────────────▼───────────────────┐
                    │  Agent offline: p5 + p6 interpret   │
                    └─────────────────┬───────────────────┘
                                      │
           ┌──────────────────────────┼──────────────────────────┐
           │                          │                          │
           ▼                          ▼                          ▼
    Pass (rare)              Still APP_LOAD              Other residual
    mark unlock used         with config_cu=1            (config_cu_failed,
    celebrate; product       → named next: wrong PDI /   attach fail, etc.)
    honesty only             multi-CU / ctrltext /       → named fix only
                             capture bytes               if evidence justifies
```

**Parallel optional track (does not replace the sandwich):**

- When a **public** stack (Ryzen AI / DD / EP) actually completes on NPU: install ctrltext into `external_known_good/` via kit `b-install` + PROVENANCE → soft compare. Unblocks offline byte research; **≠** Pass alone.

**Hygiene track (human TTY, anytime):**

- GPG-signed commit of dirty P1–P6 tree so history matches SSoT. Does not unlock Pass.

### Why not more offline code first?

Offline machinery already answers: “can we attach CONFIG_CU?”, “how do we interpret?”, “what is Pass?”. The open question is firmware reaction to **that path live**. More scorecards without a new last_apply violate the empty-loop freeze.

### Material alternatives rejected

| Alternative | Why not |
|-------------|---------|
| Idle until capture bins exist | Freezes critical path; sandwich can run with research PDI today |
| Spam env matrix again (size/layout/rela-add) | Already closed → same APP_LOAD |
| Agent opens L3 / ALLOW | Standing rules |
| Claim Pass from offline C4/P6 green | Dishonest |
| Product requires sudo every run | Privilege model wrong |

---

## Critical files

| Path | Role |
|------|------|
| `RESIDUAL.md` | SSoT: residual, unlock, next actor |
| `docs/HITL_C_APP_LOAD_RECIPE.md` | Exact sandwich under `3a70fabf` + Pass bar |
| `docs/HITL_BC.md` | B/C budget; C2 helpers |
| `docs/SILICON_PATH_LAB.md` | Lab apply / Pass criteria narrative |
| `docs/XDNA2_NEVER_AGAIN.md` | L1/L3; product vs lab privileges |
| `AGENTS.md` | Standing rules §1–9 |
| `tools/operator_app_load_kit.sh` | `a-prep` / capture B |
| `tools/hitl_c2_once.sh` | Print sandwich; fail-closed if L3 closed on apply-if |
| `tools/hitl_c_pass_apply.sh` | Live apply entry |
| `tools/p5_post_c2_interpret.sh` | Post-shot offline interpret |
| `tools/p6_true_pass_check.sh` | True-Pass criteria offline |
| `src/backends/amd_xdna2/direct.rs` | CONFIG_CU attach + timed path |
| `src/backends/amd_xdna2/silicon_contract.rs` | Honesty / Pass evidence pure |
| `testdata/xdna2_cmd/last_apply.json` | Live evidence (gitignored snapshot / fixture) |
| `testdata/xdna2_cmd/external_known_good/` | Optional capture (empty bins) |
| `testdata/xdna2_cmd/silicon_lab/lab_instr.elf` | Instruction corpus for apply |

---

## Reuse (do not reimplement)

| Symbol / tool | Use |
|---------------|-----|
| `try_lab_attach_config_cu_from_env` | Live attach when env set |
| `pack_hwctx_param_config_cu` | Unit-tested pack |
| `decide_ran_on_npu` / C4 / P6 pure | Pass ownership |
| `classify_last_apply_residual` | Residual class |
| kit / hitl_c2 / p5 / p6 shells | Operator + agent post path |
| Recipe § CONFIG_CU pre-step + L3 sandwich | Human runbook |

---

## Steps (after approval)

### Step 0 — Approve this plan (human)

- **Approve** → proceed per owner below (mostly human live).
- **Revise** → say what to change (PDI source, commit first, capture-first, etc.).
- **Abandon** → no action; tree stays dirty offline-ready.

### Step 1 — Optional: commit dirty offline work (human TTY)

**Owner:** Human (GPG). **Depends on:** wanting clean history before live.

- Stage P1–P6 machinery + docs (not last_apply secrets if any).
- Signed commit message must say: machinery only; **not** Pass; unlock `3a70fabf` still for live CONFIG_CU.
- Agent may draft message; agent does **not** bypass GPG.

**Not required** to run the sandwich.

### Step 2 — Live CONFIG_CU sandwich (human) — **critical path**

**Owner:** Human. **Depends on:** active `C_UNLOCK=3a70fabf`, sudo for L3 flip only, assets from preflight.

Exact shape (prefer recipe + `hitl_c2_once --print-only` for host-specific lines):

```bash
# 1) Prep opaque PDI + print env (research overlay OK for first smoke)
bash tools/operator_app_load_kit.sh a-prep
# export XDNA2_LAB_PDI_BLOB=/path/from/a-prep   # bash
# set -gx XDNA2_LAB_PDI_BLOB /path/from/a-prep  # fish
# optional: XDNA2_LAB_CU_FUNC=0

# 2) Print sandwich (confirm unlock active, not CONSUMED)
bash tools/hitl_c2_once.sh --print-only

# 3) Open L3 briefly (sudo) → apply with blob env in same shell → restore L3
#    Follow printed lines from hitl_c2_once / docs/HITL_C_APP_LOAD_RECIPE.md
#    Prefer lab_instr.elf path (not raw .bin)

# 4) Always verify L3 closed
cat /sys/module/amdxdna/parameters/exec_cmd_allow   # expect N/0

# 5) Offline interpret (no sudo)
bash tools/p5_post_c2_interpret.sh   # or path to new last_apply
bash tools/p6_true_pass_check.sh
```

**Shot success (not Pass):** real_ioctl path ran; detail shows `config_cu=1` **or** honest `config_cu_failed` / attach error; L3 restored; new last_apply shared with agent.

**Mark unlock used** in `RESIDUAL.md` **only after** a real sandwich (not after print-only).

### Step 3 — Interpret evidence (agent offline) — **only after Step 2**

**Owner:** Agent. **Depends on:** new last_apply / journal from Step 2.

Branch table:

| Live outcome | Agent action | Next unlock? |
|--------------|--------------|--------------|
| **Pass** (Completed + OFM allclose + `ran_on_npu=true`) | Update RESIDUAL honesty; stop residual research; optional product follow-on plan | No more C for residual |
| Still `0x3000003` **with** `config_cu=1` | Document: userspace CU path exercised; residual = wrong PDI / CU map / ctrltext / FW partition — **named** next research only | New unlock only if next live try justified |
| `config_cu_failed` / no token | Fix attach path offline (path, size, ioctl) with TDD; then re-unlock only if fix is real | After fix lands + agent ready |
| Same body fingerprint as baseline, no config_cu | Treat as failed sandwich / env not applied — re-check human steps before new unlock | Do not mint unlock for no-op |
| New residual class (not APP_LOAD) | Classify; open named offline work only if pure-testable | Case-by-case |

**Forbidden after Step 2 without new named delta:** empty `run_offline_gates` reconfirm loops as “progress.”

### Step 4 — Optional capture track (human when public stack works)

**Owner:** Human when EP/DD matmul actually runs. **Independent of** Step 2 timing.

```bash
bash tools/operator_app_load_kit.sh b-steps
bash tools/operator_app_load_kit.sh b-install /path/to/ctrltext.bin working_stack_ctrltext.bin
# edit PROVENANCE.md
bash tools/operator_app_load_kit.sh b-compare
```

Agent: soft compare + offline byte research only. Match ≠ Pass.

### Step 5 — If still APP_LOAD after good CONFIG_CU: evidence-driven deltas only

**Owner:** Agent offline, only if Step 3 branch says so. Candidates (pick by evidence, not all at once):

1. **PDI source** — hunt mladf-related / matching overlay xclbin vs generic AIE2P dump; multi-CU configs.
2. **Working-stack ctrltext** — Step 4 bytes → diff vs pure/lab ELF ctrltext.
3. **Public stack order** — re-read XRT/DD CONFIG_CU + load order vs our timed path (shim `hwctx.cpp` reference already in tree notes).
4. **Do not** re-open closed env matrix without a path change.

Each delta ends with: pure tests + honesty tokens still `not_ran_on_npu` until live Pass — then **new** unlock mint only when agent explicitly says ready.

### Step 6 — True product Pass bookkeeping (only if live Pass)

**Owner:** Agent + human.

- RESIDUAL: Pass proven; residual closed or narrowed.
- Docs: SILICON_PATH / recipe “still open” rows → done where honest.
- Product path: keep L1 default OFF; document admin-once install; no lab ALLOW in product defaults.
- Follow-on (separate plan): Colibri / CLI `npu_exec` honesty flip only after Pass is real.

### Step 7 — Explicit freezes (ongoing)

- Empty reconfirm / empty-folder implement without bytes or last_apply delta.
- Reopen `b17b0119`.
- Invent Pass from offline tools.
- Second unlock before `3a70fabf` is used, or without evidence after use.

---

## Risks

| Risk | Mitigation |
|------|------------|
| Wrong research PDI → still `0x3000003` | Expected for first smoke; still require `config_cu=` evidence; then Step 5 |
| Human forgets restore L3 | Recipe + hitl exit 5; always verify `exec_cmd_allow` |
| Soft tools mis-mark CONSUMED | Mark only after real sandwich; no stamp+CONSUMED co-line |
| Treat offline P6 green as Pass | P6 always `product_pass_requires_live_decide=1` |
| Dirty tree confuses “landed” | Optional Step 1 commit; RESIDUAL already describes machinery |
| Sudo password / wedge | Lab only; NEVER_AGAIN recovery docs; no force rmmod in agent |
| Capture never arrives | Critical path does not depend on BINS>0 |

---

## Verification

### Before live (offline, optional sanity — not a mill)

```bash
bash tools/hitl_c2_once.sh --print-only    # expect active C_UNLOCK=3a70fabf
bash tools/operator_app_load_kit.sh a-prep # or --list-default dump
# locked fixture honesty:
bash tools/p5_post_c2_interpret.sh
bash tools/p6_true_pass_check.sh           # expect P6_NOT_PASS / P6_SKIP
```

### After live

```bash
cat /sys/module/amdxdna/parameters/exec_cmd_allow   # N
bash tools/p5_post_c2_interpret.sh /path/to/new_last_apply.json
bash tools/p6_true_pass_check.sh /path/to/new_last_apply.json
# Inspect: config_cu=, ert_state=, ofm, journal 0x3000003 presence
```

### Pass claim checklist

- [ ] `ert_state` Completed (exact)
- [ ] OFM non-zero + allclose (live decide)
- [ ] `real_ioctl`
- [ ] product `ran_on_npu=true`
- [ ] L3 restored
- [ ] RESIDUAL updated honestly

---

## Open questions (non-blocking defaults)

| # | Question | Default if silent |
|---|----------|-------------------|
| 1 | Commit dirty tree before sandwich? | Optional; sandwich can run dirty |
| 2 | First PDI = host AIE2P overlay dump? | **Yes** for sequence smoke |
| 3 | Capture-first vs sandwich-first? | **Sandwich-first** (unlock already active) |
| 4 | Agent mint second unlock if first fails? | **No** until named offline delta + explicit ready |

---

## Decision for you

| Option | Meaning |
|--------|---------|
| **Approve** | Critical path = human Step 2 sandwich under `3a70fabf`; agent waits for evidence then Step 3. No empty offline mills. |
| **Approve + commit first** | Human Step 1 signed commit, then Step 2. |
| **Approve + capture-first** | Prefer Step 4 before sandwich (slower if no working public stack). |
| **Revise** | Tell what to change. |
| **Abandon** | Keep offline machinery; no live push. |

---

## Post-approval handoff (`impl:*` / `live:*` seeds)

Merge only after **Approve**. Agent does **not** run live L3.

| Id | Step | Owner |
|----|------|-------|
| `live:config-cu-sandwich` | Step 2 — L3 sandwich under `3a70fabf` + restore | **Human** |
| `live:share-last-apply` | Step 2 — share last_apply + journal | **Human** |
| `impl:p5-interpret-new` | Step 3 — p5/p6 + RESIDUAL unlock bookkeeping | Agent after evidence |
| `impl:named-delta-only` | Step 5 — only if interpret names a delta | Agent |
| `impl:pass-bookkeeping` | Step 6 — only if true Pass | Agent |
| `human:optional-commit` | Step 1 — GPG commit dirty tree | Human TTY |
| `human:optional-capture` | Step 4 — external_known_good bins | Human when stack works |

**Not seeded:** empty `run_offline_gates` loops; auto second unlock; invent bins.

---

## One-screen summary

| Question | Answer |
|----------|--------|
| What does “real” mean? | Live Pass: ERT Completed + OFM allclose + `ran_on_npu` |
| What’s blocking? | Firmware APP_LOAD; **human has not run** the unlocked CONFIG_CU sandwich |
| Unlock? | **`C_UNLOCK=3a70fabf` active** (one shot); `b17b0119` dead |
| Agent offline done? | CONFIG_CU, kit, p5, p6, privilege docs — **yes** (tree dirty) |
| Who acts next? | **Human:** `a-prep` → export PDI → L3 sandwich → restore → p5/p6 → share evidence |
| Then agent? | Interpret only; named code delta only if evidence demands |
| Capture bins? | Optional parallel; still empty |
| Product sudo? | No for normal users |
| Pass today? | **No** |

---

### Critical Files for Implementation

- `RESIDUAL.md` — unlock + residual SSoT after sandwich  
- `docs/HITL_C_APP_LOAD_RECIPE.md` — human sandwich runbook  
- `tools/operator_app_load_kit.sh` — `a-prep`  
- `tools/hitl_c2_once.sh` — sandwich print / helper  
- `tools/p5_post_c2_interpret.sh` — post-shot interpret  
- `tools/p6_true_pass_check.sh` — Pass criteria offline  
- `src/backends/amd_xdna2/direct.rs` — CONFIG_CU path (touch only if attach fails)  
- `testdata/xdna2_cmd/last_apply.json` — new live evidence  
