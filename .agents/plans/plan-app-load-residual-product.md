# Plan: Close APP_LOAD residual and keep product usable without sudo

**Date:** 2026-07-20  
**Mode:** Formal plan workflow (explore → draft → **present / await approve** → implement)  
**Canonical plan file (this file):**  
`/home/hunter/.grok/sessions/%2Fhome%2Fhunter%2FProjects%2Fai%2Fdragon-npu/019f6476-0a84-70a0-a971-be89fa1011fe/plan.md`  
**Workspace copy (after approval only):** `.agents/plans/plan-app-load-residual-product.md`  
*(Plan mode forbids writing the workspace copy until product edits are allowed.)*

**Workflow status**

| Phase | Status |
|-------|--------|
| understand | done |
| explore | done (CONFIG_CU / residual / privilege / KMQ) |
| draft | done (this file) |
| present | **approved** |
| implement | **P2 docs landed**; **P4 unlock active** `3a70fabf`; **P5 interpret** + **P6 criteria offline** (`4520851d`); live Pass still blocked on human sandwich + FW accept |

---

## Context

### Goal (plain English)

Firmware on this laptop’s NPU still **refuses to load our job** (`0x3000003`). Userspace packaging (ELF + relocations) already looks correct. We need a **single solid plan** that:

1. Explains what is broken and what is already built (no dialect-only status).
2. Separates **product UX** (normal user, no sudo per run) from **lab HITL** (temporary root only to open/close research gates).
3. Names the next work by **research question**, not empty reconfirm loops.
4. Captures **device type** (this chip is KMQ) without implying users need root because of KMQ.
5. Leaves a clean handoff for Colibri / product stack **after** residual truth is stable (not invent Pass to ship).

### What is broken (current residual)

| Fact | Status |
|------|--------|
| Userspace ELF + 12 relas, real ioctl, Dev instr BO | OK on last live run |
| Firmware job result | `ert_state=Error`, OFM zeros |
| Journal | `0x3000003` (APP_LOAD_PDI_FAIL) (+ chain `0x4000003`) |
| Product Pass / `ran_on_npu` | **No** |
| Live unlock `C_UNLOCK=b17b0119` | **CONSUMED** — do not reopen under it |
| Capture dir `testdata/xdna2_cmd/external_known_good/` | PROVENANCE only; **no** working-stack `.bin` |

### What already landed offline (do not re-build)

- Optional lab **configure compute unit** after create-context: env `XDNA2_LAB_PDI_BLOB` → opaque PDI BO → `CONFIG_HWCTX(CONFIG_CU)` (default **off**).
- Symbols: `ENV_LAB_PDI_BLOB`, `pack_hwctx_param_config_cu`, `try_lab_attach_config_cu_from_env`, `LabPdiBoGuard` in `direct.rs`; timed path wires attach after CREATE_HWCTX.
- Operator tools: `tools/dump_xclbin_pdi.sh`, `tools/operator_external_capture.sh`, `tools/operator_app_load_kit.sh`.
- Host PDI dump smoke: **283 840 B** from AIE2P overlay xclbin (research bytes; not known-good matmul).
- Honesty locks in `silicon_contract`: attach / match / gates **≠** Pass; offline matrix still reports construction gap open until live evidence says otherwise.
- Unit tests: `cargo test --features amd-xdna2 --lib config_cu` (pack, attach unset/set, honesty tokens).

### Working tree note (P1 not fully committed)

Branch `rust` is **ahead of origin by 1** and currently has **uncommitted** P1-related edits (as of plan present):

| State | Paths (representative) |
|-------|------------------------|
| Modified | `direct.rs`, `silicon_contract.rs`, `amd_xdna2.rs`, `RESIDUAL.md`, `AGENTS.md`, `CONSTRUCTION_MAP_APP_LOAD.md`, capture `PROVENANCE.md` |
| Untracked | `tools/dump_xclbin_pdi.sh`, `tools/operator_app_load_kit.sh`, `tools/operator_external_capture.sh` |

Step 2 (optional signed commit) is how this lands in git history. **Do not** treat dirty tree as Pass or unlock.

### Constraints (standing rules)

- Default agent work is **offline**; no casual sudo; no default `XDNA2_ALLOW_EXEC_CMD=1`.
- **No empty reconfirm / empty-folder implement** loops (`AGENTS.md` §8–9).
- Live apply only with **new host-minted unlock** + agent ready + L3 sandwich; unlock ≠ Pass; **never** reopen under consumed `b17b0119`.
- Prefer aiebu **ELF** EXEC; refuse silent raw txn as product path.
- Plain English in chat/status; real paths over nicknames (“bag,” track letters alone).
- GPG-signed commits only; agent never disables signing.

### Non-goals

- Invent capture bytes, vendor post-patch, or Pass / `ran_on_npu`.
- Re-run closed env matrix (size mode / layout / rela-add) without a path change.
- Require **sudo every inference** for product users.
- Switch this Strix host to UMQ (device type is KMQ; not a software preference).
- Second C unlock without new evidence + host-minted stamp.
- Full Colibri product integration **in this plan** (deferred follow-on once residual path is honest).
- Empty offline gate mills / composition scorecards without new bytes (frozen §9).

### Assumptions

- Host remains PCI `1022:17f0` / NPU4 family → driver **`device_type=0` (KMQ)**.
- Hardened lab module may keep `exec_cmd_allow=0` by default; that is **lab policy**, not product “must sudo.”
- Wrong overlay PDI may still fail load; that is still useful evidence, not Pass.
- Product node `/dev/accel/accel0` is expected usable by group `render` (or equivalent) after admin install.

---

## Glossary (plan-level, once)

| Term | Meaning |
|------|---------|
| **KMQ** | Kernel Mode Queue. This chip’s device type: kernel driver owns the submit path. **Not** “needs sudo.” |
| **UMQ** | User Mode Queue. Other NPU generations in the driver tree. **Not** this laptop’s bind path. |
| **`umq_bo` field** | Optional buffer handle on CREATE_HWCTX; may be 0 on KMQ. ≠ “running as UMQ device.” |
| **PDI** | Opaque app/partition blob from an xclbin; public stacks register it on the hardware context. |
| **CONFIG_CU** | Ioctl after create-context: “this compute unit uses that PDI buffer.” |
| **ctrltext** | Per-job tile program (`.ctrltext` from lab ELF). |
| **L1** | Userspace double gate: Cargo feature `xdna2-dangerous-exec-cmd` + env `XDNA2_ALLOW_EXEC_CMD=1`. |
| **L3** | Kernel param `exec_cmd_allow` on **hardened** module only (lab). Stock module has no L3. |
| **Pass / `ran_on_npu`** | ERT Completed + non-zero OFM allclose + real ioctl. Never invent. |

---

## Approach

### Recommended path

Treat work as **three parallel tracks** with clear owners. Do not serialize empty offline loops.

```text
Track Product (always)
  Document + keep: normal user opens /dev/accel; admin installs driver once.
  No per-run sudo. L1 stays OFF in default product builds.

Track Lab residual — configure-CU (agent offline done; human for live)
  Offline machinery: DONE.
  Live: only after NEW unlock → dump PDI → set XDNA2_LAB_PDI_BLOB → one sandwich → restore L3.
  Outcome: journal/status delta vs locked 0x3000003 (even fail is evidence).

Track Lab residual — working-stack capture (human when stack works)
  Place real ctrltext under testdata/xdna2_cmd/external_known_good/ + PROVENANCE.
  Soft compare. Match ≠ Pass, but unblocks offline byte research.
```

### Material alternatives rejected

- **Not “idle forever until capture dir has a bin”** — freezes empty reconfirm only; named offline code was allowed and is largely done.
- **Not “always use UMQ”** — this device reports KMQ; wrong contract.
- **Not “product needs sudo because KMQ”** — KMQ is queue ownership; privilege is udev/driver install vs lab gates.
- **Not more composition scorecards without bytes** — frozen by §9.

### Phase map

| Phase | Owner | Status | Deliverable |
|-------|-------|--------|-------------|
| **P0** Formal plan + privilege/KMQ truth | Agent (this doc) | **Done** | Approved plan |
| **P1** CONFIG_CU + operator tools offline | Agent | **Landed** (may be uncommitted) | Env attach + dump/kit + tests |
| **P2** Docs honesty pass (product vs lab privileges; plain English residual) | Agent | **Done** | NEVER_AGAIN + PACKAGING privilege tables; RESIDUAL sudo row; CONSTRUCTION_MAP UMQ/`umq_bo`; README admin-once |
| **P3** Human capture into `external_known_good/` | Human | **Blocked on bytes** | Real `.bin` + PROVENANCE → soft compare |
| **P4** One new live unlock + CONFIG_CU sandwich | Human + agent ready | **Blocked on unlock** | Honest last_apply; L3 restored |
| **P5** Interpret live + capture evidence | Agent offline | After P3 and/or P4 | Named next code delta only if justified |
| **P6** True Pass criteria | Offline criteria ready (`4520851d`); live Pass open | Criteria offline OK; product Pass live-only | Completed + OFM allclose → `ran_on_npu` |

---

## Critical files

| Path | Why |
|------|-----|
| `RESIDUAL.md` | Single source of truth for residual status (English first) |
| `AGENTS.md` | Standing rules §1–9 (offline, unlock, freeze empty loops) |
| `docs/CONSTRUCTION_MAP_APP_LOAD.md` | Public build order vs lab; glossary |
| `docs/XDNA2_NEVER_AGAIN.md` | L1/L2/L3; wedge history; production vs lab `exec_cmd_allow` |
| `docs/XDNA2_EXEC_CMD_LAB.md` | Lab enable order + restore |
| `docs/SILICON_PATH_LAB.md` | Operator status/apply/L3 sandwich |
| `docs/HITL_C_APP_LOAD_RECIPE.md` | One live C when unlocked |
| `docs/XDNA2_PACKAGING.md` | Driver/packaging; product install shape |
| `src/backends/amd_xdna2/direct.rs` | CONFIG_CU pack/attach/ioctl; timed path; tests |
| `src/backends/amd_xdna2/silicon_contract.rs` | Construction / CU-PDI honesty research |
| `tools/operator_app_load_kit.sh` | One entry for prep + capture |
| `tools/dump_xclbin_pdi.sh` | Opaque PDI dump + env print |
| `tools/operator_external_capture.sh` | Capture install/compare |
| `tools/hitl_c2_once.sh` / `hitl_c_pass_apply.sh` | Live apply when unlocked |
| `testdata/xdna2_cmd/external_known_good/` | Capture dir (bytes still missing) |
| `testdata/xdna2_cmd/silicon_lab/lab_instr.elf` | Failing instruction corpus |
| `testdata/xdna2_cmd/last_apply.json` | Locked failing live snapshot |
| Host `AMD_AIE2P_*.xclbin` | Research PDI source (outside git) |
| `~/Projects/kernel/xdna-driver/src/shim/kmq/hwctx.cpp` | Production CONFIG_CU reference |

---

## Reuse

| Symbol / module | Path | How |
|-----------------|------|-----|
| `try_lab_attach_config_cu_from_env` | `direct.rs` | Live path already wired; do not reimplement |
| `pack_hwctx_param_config_cu` | `direct.rs` | Keep unit-tested pack |
| `LabPdiBoGuard` | `direct.rs` | BO lifetime across timed path |
| `research_construction_path_lab_vs_public` | `silicon_contract.rs` | Soft honesty only; do not invent gap closed |
| `operator_app_load_kit.sh` | `tools/` | Operator entry; extend docs not flags |
| `run_offline_gates.sh` | `tools/` | Soft offline suite; never C-ready |
| `decide_ran_on_npu` / OFM golden | `numerical` + lab_io | Only true Pass gate |

---

## Steps

Ordered for **implementation after approval**. Dependencies noted.

### Step 0 — Approve this plan (human)

- Approve / revise / abandon.
- No code until approve (except this plan artifact).

### Step 1 — Docs: privilege model + residual English (agent) — **P2**

**Depends on:** approval.

1. Add a short **Product vs lab privileges** section (or table) to `docs/XDNA2_NEVER_AGAIN.md` and/or `docs/XDNA2_PACKAGING.md`:
   - Admin once: driver, firmware, udev/group so `/dev/accel` is openable.
   - User always: no sudo for open/hwctx/run when product works.
   - Lab only: temporary L3 flip + L1 ALLOW for supervised EXEC on hardened hosts.
2. Ensure `RESIDUAL.md` top table stays English-first and points at kit commands (already partial).
3. Ensure `CONSTRUCTION_MAP` glossary keeps KMQ/UMQ/privilege separation (already partial).
4. Optional README one-liner: “NPU access is a normal device node; install may need admin once.”

**Not in this step:** live apply, invent unlock, empty gate mill.

### Step 2 — Commit hygiene for already-landed P1 (human on TTY) — optional

**Depends on:** human wants git history.

- Staged/signed commit of CONFIG_CU + tools (agent does not force-push; GPG on TTY).
- Message should state: machinery only; not Pass; unlock still consumed.

### Step 3 — Capture when a public stack works (human) — **P3**

**Depends on:** a run that actually completes on NPU (Ryzen AI / DD / EP).

```bash
bash tools/operator_app_load_kit.sh b-steps
bash tools/operator_app_load_kit.sh b-install /path/to/ctrltext.bin working_stack_ctrltext.bin
# fill testdata/xdna2_cmd/external_known_good/PROVENANCE.md
bash tools/operator_app_load_kit.sh b-compare
```

Agent after bytes: soft classify + offline research only; **no** invent Pass.

### Step 4 — One live CONFIG_CU try (human + agent ready) — **P4**

**Depends on:** **new** host-minted `C_UNLOCK`, not `b17b0119`.

```bash
bash tools/operator_app_load_kit.sh a-prep   # prints XDNA2_LAB_PDI_BLOB=
# export env; then L3 sandwich via hitl_c2_once / recipe only when agent says ready
# restore exec_cmd_allow=0; share last_apply
```

Success criteria for the **shot** (not Pass): real_ioctl path ran; detail has `config_cu=1` or honest `config_cu_failed`; L3 restored; classify residual class.

### Step 5 — Interpret evidence offline (agent) — **P5**

**Depends on:** new last_apply and/or capture bytes.

- If still `0x3000003` with CONFIG_CU: wrong PDI / multi-CU / ctrltext residual → named next delta only.
- If capture sha matches lab ctrltext and still fail: strengthens “not ctrltext body alone.”
- If Completed + non-zero OFM: run Pass criteria path → only then `ran_on_npu`.

### Step 6 — True Pass (live) — **P6**

**Depends on:** firmware accept + golden OFM.

- Exact: ERT Completed, non-zero OFM allclose, `real_ioctl`, no invent.
- **Offline criteria readiness landed (`4520851d`):**
  `format_true_pass_criteria_checklist` / `format_true_pass_p6_note` +
  `tools/p6_true_pass_check.sh`. Locked fixture → `P6_NOT_PASS` / `P6_SKIP`.
  Offline C4 PassCandidate **≠** product Pass (`product_pass_requires_live_decide=1`).
- **Live product Pass still open** — only via `decide_ran_on_npu` after sandwich
  yields Completed + allclose. Do **not** invent Pass offline.

---

## Risks

| Risk | Mitigation |
|------|------------|
| Wrong overlay PDI still fails load | Document as expected for machinery; capture mladf-related PDI or path P3 |
| Confusing lab L3 with product sudo | Step 1 privilege table; NEVER ship “sudo every run” |
| Empty implement loops | §9 freeze; stop if no new bytes/evidence/named delta |
| Consumed unlock reused | Hard ban on `b17b0119`; new host-minted only |
| Offline matrix invents gap closed | Keep `lab_has_cu_pdi_config=false` offline until live evidence policy says otherwise |
| Agent opens L3 / ALLOW | Standing rules; agent offline default |
| UMQ confusion in future ports | Device_type check; separate plan if porting to UMQ silicon |

---

## Verification

### Offline (no sudo, no EXEC)

```bash
bash -n tools/dump_xclbin_pdi.sh tools/operator_external_capture.sh tools/operator_app_load_kit.sh
bash tools/operator_app_load_kit.sh              # overview; capture dir blocked ok
bash tools/dump_xclbin_pdi.sh --list-default
cargo test --features amd-xdna2 --lib config_cu
cargo test --features amd-xdna2 --lib construction_path
cargo test --features amd-xdna2 --lib cu_pdi
cargo clippy --features amd-xdna2 --lib -- -D warnings
cargo fmt -- --check
# optional soft suite
bash tools/operator_app_load_kit.sh offline-gates
```

### Privilege sanity (manual)

```bash
ls -la /dev/accel/accel0          # expect group render (or world-usable)
cat /sys/class/accel/accel0/device/device_type   # expect 0 = KMQ on this host
cat /sys/module/amdxdna/parameters/exec_cmd_allow  # expect 0/N when idle (hardened lab)
# Normal user: open/probe/hwctx without sudo (existing probe path)
```

### Live (only when unlocked)

- Kit `a-prep` → env set → one sandwich → restore L3 → classify last_apply.
- Never claim Pass without OFM allclose.

---

## Open questions

1. **When to mint a new live unlock?** Recommendation: after P2 docs land **and** either (a) operator is ready for one CONFIG_CU sandwich, or (b) capture bytes land and offline compare changes the hypothesis. Not before.
2. **Best PDI for live A?** Default host AIE2P overlay for sequence smoke vs hunting an mladf-related xclbin. Prefer: first smoke with dump tool; if still fail, escalate PDI source (human).
3. **Product packaging default for `exec_cmd_allow`:** if shipping hardened L3 long-term, product hosts need admin-set `=1` **or** upstream without L3. Capture in packaging doc; do not block residual work.

---

## Decision for you

| Option | Meaning |
|--------|---------|
| **Approve** | Agent proceeds with **Step 1 (P2 docs)** only unless you expand scope; no live apply; no empty reconfirm. Seeds `impl:*` todos below. |
| **Approve + commit prep** | Step 1 + draft commit message for already-landed P1 (you sign on TTY). |
| **Revise** | Tell what to change (phases, privilege wording, unlock timing, Colibri scope). |
| **Abandon** | Keep machinery as-is; plan discarded for action. |

---

## Post-approval handoff (`impl:*` seeds)

Only merge these after **Approve**. Do not implement until then.

| Id | Step | Owner |
|----|------|-------|
| `impl:p2-privilege-docs` | Step 1 — NEVER_AGAIN + PACKAGING product-vs-lab table | Agent |
| `impl:p2-residual-english` | Step 1 — RESIDUAL / CONSTRUCTION_MAP English + KMQ consistency | Agent |
| `impl:p2-readme-oneliner` | Step 1 — optional README admin-once bullet | Agent |
| `impl:p2-gates` | Offline verify after docs (bash -n kit; config_cu tests if touched; fmt) | Agent |
| `impl:p1-commit-msg` | Step 2 — draft signed-commit message only if user asked commit prep | Agent draft / Human TTY |
| `impl:p3-capture-wait` | Step 3 — blocked on human bytes; agent soft-compare only after install | Human then Agent |
| `impl:p4-live-config-cu` | Step 4 — blocked on **new** unlock; one sandwich | Human + Agent ready |
| `impl:p5-interpret` | Step 5 — after new evidence only | Agent |
| `impl:p6-pass` | Step 6 — true Pass criteria only on live allclose | Live |

**Explicitly not seeded as agent default work:** empty `run_offline_gates` reconfirm loops, second unlock mint, ALLOW/L3 flips.

---

## Follow-on (out of this plan’s Steps)

After residual honesty is stable (docs + either capture or CONFIG_CU live evidence interpreted):

- Colibri adapter / product surface still uses host_ref / fail-closed real submit until `ran_on_npu`.
- Packaging: stock vs hardened `exec_cmd_allow` story for end users (open question 3).
- Separate plan if/when porting to UMQ silicon.

---

## Summary (one screen)

| Question | Answer |
|----------|--------|
| What’s wrong? | Firmware rejects load (`0x3000003`) after good userspace ELF. |
| What’s already built? | Optional CONFIG_CU + PDI dump + capture kit offline (working tree dirty; not all committed). |
| Do users need sudo? | **No** for day-to-day. Admin once for driver/udev. Sudo only for lab L3 flip on hardened research hosts. |
| Why KMQ? | This chip **is** KMQ (`device_type=0`). Not a sudo choice. |
| What’s frozen? | Empty reconfirm; spent unlock `b17b0119`; invent Pass. |
| Who unblocks next? | Human: capture bytes and/or new unlock live shot. Agent after approve: privilege/residual docs (P2). |
| Colibri? | Product stack deferred; this plan is residual + privilege truth first. |

---

### Critical Files for Implementation

- `docs/XDNA2_NEVER_AGAIN.md` — product vs lab privilege table  
- `docs/XDNA2_PACKAGING.md` — admin-once / user-always  
- `RESIDUAL.md` — English status + next actor  
- `docs/CONSTRUCTION_MAP_APP_LOAD.md` — KMQ glossary consistency  
- `README.md` — optional one-liner (admin once)  
- `tools/operator_app_load_kit.sh` — operator entry (docs only unless bug)  
- `src/backends/amd_xdna2/direct.rs` — already-landed CONFIG_CU (touch only if P5 requires)  
- `src/backends/amd_xdna2/silicon_contract.rs` — honesty tokens (touch only if P5 requires)  
