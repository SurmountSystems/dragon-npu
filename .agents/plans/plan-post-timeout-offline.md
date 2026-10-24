# Plan: Post-timeout offline evidence + honesty (after `3a70fabf`)

**Date:** 2026-08-10
**Mode:** Formal plan (explore → draft → present / await approve → implement)
**Explore report:** `.agents/reports/plan-post-timeout-explore.md`
**Live-shot report:** `.agents/reports/live-shot-3a70fabf-timeout.md`
**Session plan (mirror):** host session `plan.md` for this chat

**Workflow status**

| Phase | Status |
|-------|--------|
| understand | done |
| explore | done (explore report on disk) |
| draft | done (this file) |
| present | done (operator Approve) |
| implement | **done** — report `.agents/reports/impl-post-timeout-offline.md` |
| agent residual | **CLOSED** — see `.agents/plans/plan-silicon-whats-next.md` (no operator-only required steps; agent idle) |

---

## Elevator truth

The one-shot unlock **`C_UNLOCK=3a70fabf` is CONSUMED**. Live stock CONFIG_CU + PDI ran under userspace ALLOW; the outer **30s** lab timeout killed `npu_direct_bench` (**rc=124** → lab exit 4). **No** last_apply JSON was written because stamps only run on a normal Rust return. Journal in the same window shows **IO_PAGE_FAULT** and **`aie2_set_cmd_timeout`** (incomplete class, not a finished APP_LOAD snapshot). Machine restored cleanly; not wedged.

**Next value is offline:** make structured evidence survive outer kill, teach tools and docs not to lie about missing pins, classify timeout/journal soft signals without inventing Pass. **No** second live shot under `3a70fabf`. **No** empty gate reconfirm mill. Product Pass and `ran_on_npu` remain false.

---

## Context

### Why we plan (not re-live)

| Fact | Implication |
|------|-------------|
| Unlock one-shot used | Further live C needs **new** host-minted unlock + named offline reason |
| last_apply missing | Post tools cannot interpret this shot; collect correctly refused the pin |
| Outer 30s < setup + hang budget | Retry with same defaults wastes human time even after stamps land |
| Stock host | No sudo L3; fake L3 ceremony is waste; wedge risk still real without L2 guards |
| Capture dir empty | External known-good still human-only; not this plan’s live path |

### Goal

1. **Progressive last_apply** so a kill after durable stages still leaves a JSON pin with honest status/detail tokens.
2. **Timeout / missing-path honesty** in tools and docs (no baseline false comfort).
3. **Journal soft taxonomy** for the signals we already saw (timeout / IO_PAGE_FAULT / aie2_set_cmd_timeout).
4. **Default outer budget guidance** so the next human sandwich (only after new unlock) is not 30s by default for CONFIG_CU silicon.
5. **Residual / recipe / plan doc SSoT** aligned to CONSUMED unlock and incomplete residual class.

### Non-goals

- Live EXEC, L3 open, sudo, reopening `3a70fabf`, inventing Pass or APP_LOAD class from kill alone.
- Empty `/implement` reconfirm of green offline gates with no named code delta.
- Hardened module install / driver patch apply on the host (human, deferred).
- Placing capture bytes under `testdata/xdna2_cmd/external_known_good/` (human when public stack works).
- **grok-build** `just check` / plan-panel click test failure (different repo; out of this workspace).
- Minting a new unlock in this plan (agent may only document readiness criteria; host mints unlock later).

### Assumptions

- Product stamp path stays best-effort write (0600, path confinement); progressive writes overwrite the same pin path.
- Unit tests stay offline: no `/dev/accel`, no `XDNA2_ALLOW_EXEC_CMD=1` in default tests.
- A hang **inside** a single ioctl can still lose that stage’s stamp; progressive stamps reduce loss, they do not remove wedge risk.

---

## Approach

**Offline-first product quality elevated from “optional” to required.**

1. Add mid-path **best-effort stamps** in `timed_matmul_direct` after stages that already returned control to Rust (hwctx created; CONFIG_CU attached or failed; immediately before submit_and_wait). Keep final stamp on all return paths.
2. Token honesty on partial files: always `invent_pass=0 not_ran_on_npu=1`; status like `lab_partial_hwctx_ok` / `lab_partial_config_cu` / `lab_partial_pre_submit` (exact names chosen to match existing status style; document in residual).
3. Soft classify / p5 notes: missing explicit pin = incomplete evidence, **not** locked baseline success; journal-only path never claims C-ready or Pass.
4. Docs: recipe + lab_exec + config-cu helper say CONFIG_CU silicon first shot should use **≥120s** outer timeout; WAIT stays finite (~5–15s).
5. Pure journal helpers for soft reports only.
6. Refresh RESIDUAL / HITL recipe unlock rows so nothing still says `3a70fabf` is active.

**Not chosen**

- **Not re-live under `3a70fabf`** — unlock consumed; wastes time and violates one-shot law.
- **Not lengthening outer timeout alone without progressive stamps** — still zero JSON if kill mid-flight; human time wasted again.
- **Not inventing residual class from journal alone** — incomplete until structured last_apply exists.
- **Not parent-thread implement marathons** — after approve, implementer subagent owns red→green.

---

## Critical files

| Path | Why |
|------|-----|
| `src/backends/amd_xdna2/direct.rs` | `stamp` closure, timed path, CONFIG_CU attach, WAIT default |
| `src/backends/amd_xdna2/silicon_contract.rs` | `finalize_lab_timed_result`, `write_last_apply_*`, residual class helpers |
| `examples/npu_direct_bench.rs` | Live bench entry (no behavior change required if timed path owns stamps) |
| `scripts/lab_exec_once.sh` | Outer timeout default/docs comments; exit 4 on kill |
| `tools/hitl_config_cu_once.sh` | Stock sandwich print, pin path, post-shot guidance |
| `tools/collect_post_c2_evidence.sh` | Refuse missing pin; loud incomplete note |
| `tools/p5_post_c2_interpret.sh` | Unlock bookkeeping CONSUMED; missing pin honesty |
| `tools/c3_post_c2_classify.sh` / `c4` / `p6` | Soft skip vs invent; optional incomplete token |
| `docs/HITL_C_APP_LOAD_RECIPE.md` | Unlock CONSUMED; timeout hygiene |
| `docs/XDNA2_EXEC_CMD_LAB.md` | Outer vs WAIT policy |
| `RESIDUAL.md` | SSoT unlock + residual class + next actor |
| `.agents/reports/live-shot-3a70fabf-timeout.md` | Forensics reference |
| `testdata/xdna2_cmd/last_apply.json` | Locked baseline only — never treat as this shot |

---

## Reuse

| Symbol / module | Path | How |
|-----------------|------|-----|
| `stamp` closure | `direct.rs` | Call best-effort at mid-stages (same finalize path) |
| `finalize_lab_timed_result` | `silicon_contract.rs` | Already best-effort write; progressive overwrites same path |
| `write_last_apply_to_path` / path allowlist | `silicon_contract.rs` | No second write API unless confinement needs a partial-only helper |
| `classify_last_apply_residual` / detail parsers | `silicon_contract.rs` | Extend pure tokens for partial / journal soft taxonomy |
| p5 / c3 / c4 / p6 shell | `tools/` | Honesty strings + exit maps only; no live apply |
| Offline gates | `tools/run_offline_gates.sh` | Run **after** named code delta only |

---

## Steps (all agent-required offline unless marked human)

### Step 1 — Progressive last_apply (required, TDD)

1. **Red first:** pure unit contract that mid-stage finalize writes a last_apply JSON with partial status and honesty trio (`invent_pass=0`, `not_ran_on_npu=1`, not product Pass). Prefer stub/timed path unit surface already used for last_apply; do not open `/dev/accel`.
2. **Green:** in `timed_matmul_direct`, after CREATE_HWCTX success, after CONFIG_CU success **or** fail return path (fail already stamps; ensure success path stamps before continuing), and immediately before `submit_and_wait_command`, call the same stamp/finalize path with distinct partial statuses/details (`config_cu=…` when known).
3. Document in code comment: kill **during** ioctl can still lose that stage; progressive stamps only help when Rust regains control between stages.
4. fmt + clippy `-D` on touched packages + targeted `cargo test --features amd-xdna2 --lib` filters for last_apply / config_cu / silicon_contract.

### Step 2 — Timeout and missing-pin honesty (required)

1. `collect_post_c2_evidence.sh`: keep exit 2 on missing explicit path; print plain English that this matches outer timeout kill, not APP_LOAD classify.
2. p5 / classify docs headers: **`C_UNLOCK=3a70fabf` CONSUMED**; do not soft-imply active unlock.
3. When no path is given, tools that fall back to locked fixture must print a loud **baseline fixture only** note so operators do not confuse 2026-07-18 APP_LOAD JSON with the timeout shot.
4. bash -n on touched scripts.

### Step 3 — Journal soft taxonomy (required; elevated from optional)

1. Pure helpers (unit-tested) recognizing soft signals: outer kill incomplete, `IO_PAGE_FAULT`, `aie2_set_cmd_timeout` (and related journal tokens already seen).
2. Soft classify / optional checklist line: incomplete residual class; **always** `not_c_ready=1 invent_pass=0 not_ran_on_npu=1`.
3. **Never** map journal-only → PassCandidate or locked APP_LOAD without last_apply snapshot.

### Step 4 — Default outer budget + recipe hygiene (required docs)

1. HITL recipe + `hitl_config_cu_once` / kit print: recommend **`--timeout-s 120`** (or ≥120) for CONFIG_CU silicon stock path; WAIT stays finite.
2. `lab_exec_once` comment or help: 30s is a short lab default; CONFIG_CU silicon should raise outer budget. Do **not** silently change global default to 120 without doc note (either raise default for silicon-mladf apply only **or** print loud WARN when silicon-mladf + default 30s — prefer loud WARN + recipe 120 so non-silicon dry paths stay snappy).
3. Align RESIDUAL “who acts next” after steps 1–3 land.

### Step 5 — Residual and plan SSoT pin (required docs)

1. Confirm RESIDUAL top stays CONSUMED + incomplete class (already largely done; fix any mid-file “active unlock” drift in tools/docs touched above).
2. Note workspace plans `plan-app-load-residual-product.md` / `plan-make-silicon-real.md` P4 unlock rows are stale; add a short supersession note pointing at this plan (no novel Pass claims).
3. Soft gates only after named delta: `config_cu` tests + bash -n + relevant silicon_contract filters. **No** empty full-gate reconfirm as the only deliverable.

### Step 6 — Human-only after offline green (not agent live)

1. **Optional later:** new host-minted unlock + named reason “progressive last_apply + timeout honesty landed.”
2. Human paste sandwich with **≥120s** outer; stock = no L3 sudo.
3. Optional hardened install (L2 null-guards) if human wants wedge reduction.
4. Optional external known-good capture when public stack works.
5. Agent offline interpret only after a **real** last_apply file exists.

---

## Risks (brick / waste prioritization)

| Risk | Mitigation (this plan) |
|------|------------------------|
| **Re-live under `3a70fabf`** | Forbidden in steps; residual CONSUMED; plan non-goal |
| **Wedge stock amdxdna** | No agent live EXEC; no L3; progressive stamps only touch offline write path in tests |
| **Leave L3 open** | Stock path has no L3; docs still say restore if hardened |
| **Waste human on prep** | Agent owns prep/print; human only pastes sandwich when **new** unlock exists |
| **Waste human on 30s re-kill** | Step 4 outer budget guidance before any new unlock |
| **False comfort from fixture** | Step 2 loud baseline-only note |
| **Invent Pass from journal** | Step 3 honesty trio hard-coded; C4/P6 still fail-closed on `ran_on_npu` |
| **Empty reconfirm mill** | Step 5 bans gates-only “done”; frozen AGENTS §8–9 |
| **ALLOW=1 in tests** | NEVER AGAIN; verify tests do not set it |
| **Scope creep to grok-build CI** | Explicit non-goal; separate workspace |

---

## Verification

| Check | Command / proof |
|-------|-----------------|
| Red→green progressive stamp | Named unit test(s) for partial last_apply write + honesty tokens; observed red then green |
| CONFIG_CU pure still green | `cargo test --features amd-xdna2 --lib config_cu` (and new partial filters) |
| Clippy / fmt | package-scoped fmt + `clippy … -D warnings` on touched crates |
| Shell | `bash -n` on touched tools/scripts |
| Soft honesty | p5/c3/c4/p6 on **missing** pin or incomplete fixture → skip/refuse, never Pass |
| No live | Agent does not open L3 / ALLOW / EXEC on host for this plan |
| Residual | Top unlock row still CONSUMED; next actor = human only after new unlock |

---

## Open questions

None blocking. Defaults assumed:

- Prefer **loud WARN** when silicon-mladf apply uses default 30s rather than silently changing all lab defaults to 120s.
- Journal taxonomy is soft research only until a completed last_apply exists.
- grok-build plan-panel test failure is **out of scope** for this workspace unless the operator opens a grok-build session for it.

---

### Critical Files for Implementation

- `src/backends/amd_xdna2/direct.rs` — progressive stamp sites on timed path
- `src/backends/amd_xdna2/silicon_contract.rs` — finalize / write / pure residual tokens
- `scripts/lab_exec_once.sh` — outer timeout WARN / help
- `tools/hitl_config_cu_once.sh` — sandwich print ≥120s + CONSUMED unlock
- `tools/collect_post_c2_evidence.sh` / `tools/p5_post_c2_interpret.sh` — missing pin honesty
- `docs/HITL_C_APP_LOAD_RECIPE.md` / `docs/XDNA2_EXEC_CMD_LAB.md` — timeout hygiene
- `RESIDUAL.md` — SSoT after implement
