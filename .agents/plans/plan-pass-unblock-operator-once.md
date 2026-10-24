# Plan: What unblocks Pass next (honest) — autonomy + one privilege script

**Date:** 2026-08-10
**Mode:** plan only until intentional Approve
**Session plan:** this file
**Workspace mirror (after approve):** `.agents/plans/plan-pass-unblock-operator-once.md`
**SSoT residual:** `RESIDUAL.md` top

**Revise (operator):**
- Not “one script for one tiny thing only.”
- Prefer **full agent autonomy** for everything that does not need escalated privileges.
- When escalated privilege is required, put it in **one** script, review for **parsimony**, and **document why** privilege is proven necessary (reasoning + prior scripts/evidence).
- **Hard rule:** no irreversible mutable changes **outside** the current working tree (`dragon-npu`). Everything else = **read-only** (or ephemeral `/tmp` research dumps that do not rewrite the system). Use judgment; no data-loss risk.

---

## Locked honesty

| Fact | Value |
|------|--------|
| **Pass / `ran_on_npu`** | **No** (this plan does not invent Pass) |
| invent_pass | **0** unless true live Pass |
| Active unlock | **None** (recent one-shots CONSUMED, including flexml `2ae69036`) |
| Flexml live `2ae69036` | attach OK, still `wait_cmd_einval`, not Pass — **spent**; no re-live without new material |
| Capture under `external_known_good/` | still empty |
| Driver/MMU orphan path | parked (flexml shot did not re-hit orphan fault) |

**Product goal still:** live ERT Completed + non-zero OFM allclose + real ioctl → Pass.

---

## What unblocks Pass next (honest)

| Need | Who / how | Status |
|------|-----------|--------|
| Working-stack ctrltext under `testdata/xdna2_cmd/external_known_good/` | Whoever has a real file: agent `--ingest` if path known; human only if they alone hold the capture | Still empty |
| True matmul-matched program/PDI beyond flexml STX | Agent offline discovery (read-only outside tree) + new unlock only if new fingerprint | Flexml tried once; residual remains |
| Driver/MMU for prior orphan class | Only with new evidence; host/kernel may need privilege | Parked |

---

## Autonomy model (the real point)

### Layer A — Full agent autonomy (default)

Do without asking, without multi-tool homework, **mutating only inside the repo** (plus ephemeral `/tmp` research outputs):

- Status / residual honesty / reports under `.agents/`
- Offline fingerprint packs, dump-tool polish, soft compare when bytes exist
- Capture **ingest** when a path is known (copy into workspace bag)
- Stock live when unlock is active and host needs **no** sudo (`exec_cmd_allow` missing on stock)
- Code/docs TDD for honesty, classifiers, kit glue
- **Read-only** walks outside the tree (ryzen-ai-install, RyzenAI-SW, host xclbins, journal if readable without root)

### Layer B — One escalated-privilege script (only if proven)

**Only** when something **cannot** be done without elevated rights (e.g. temporary `exec_cmd_allow=1` on hardened module, sudo L3 open/restore, privileged journal if blocked, DKMS/module load).

| Requirement | Content |
|-------------|---------|
| **One script** | e.g. `tools/pass_unblock_privileged_once.sh` (name plain English) |
| **Parsimony** | Minimum syscalls/steps; no kitchen-sink installer; no permanent policy change |
| **Proven need** | Header + residual note: what failed without privilege, which prior run/script proved it, why Layer A is insufficient |
| **Scope** | Prefer temporary enable → run → restore; never leave L3 open; never store passwords |
| **Outside tree** | Privilege script may touch **module params / device policy** only when proven; still **no** irreversible edits to foreign project trees, home configs, or system packages unless operator later expands scope |
| **Output** | Single stdout report: what ran, restore verified, invent_pass=0, Pass still No unless live decide |

**Not “escalated privilege”** (do not hide these behind sudo theater):

| Action | Why Layer A |
|--------|-------------|
| Stock `config-cu-run` on this host | `exec_cmd_allow` sysfs **missing** → no sudo L3 required (proven prior stock lives) |
| Dump PDI from flexml/CVML under `~/Projects/ai/...` | Read host files; write `/tmp` + repo reports |
| Ingest ctrltext into `testdata/.../external_known_good/` | Workspace write only |
| Soft compare / fingerprint | Pure offline |

### Layer C — True human physics (rare)

Only when the agent **cannot** obtain the bytes or judgment:

- Capture from a working public stack that only you run / only you possess
- EULA / vendor download not present on disk
- Confirming intentional risk beyond stock live (if ever required)

Then: one command in Layer A (`--ingest PATH`) or Layer B if that capture path itself needs privilege — **not** five aliases.

---

## Why privilege may or may not be needed (document this in implement)

### Proven on this host (do not re-litigate)

| Evidence | Conclusion |
|----------|------------|
| Stock lives under `11bc9016`, `97f38307`, `2ae69036` | Agent ran `config-cu-run` **without** sudo L3 |
| Residual: `exec_cmd_allow=missing` | Stock path ≠ hardened L3 sandwich |
| Flexml + CVML PDI dumps | Offline, no privilege |

**Therefore:** default Pass-unblock work stays **Layer A**. Building a privilege script **only if** a concrete next step fails closed with “needs root/L3” (e.g. hardened module on a different host, or a tool that cannot read journal without root).

### When implement must write the privilege proof

If Layer B is opened, the script header **must** include:

1. **Symptom** that Layer A hit (exact error / missing sysfs / permission).
2. **Prior scripts** tried (`hitl_config_cu_once`, `lab_exec_once`, `config-cu-run`, …).
3. **Minimal privilege** requested (e.g. only `exec_cmd_allow` flip, not full DKMS).
4. **Restore** contract (always restore; exit non-zero if still open).
5. **Pass honesty** (privilege ≠ Pass).

If Layer A never hits a privilege wall for the approved work, **do not invent** a sudo script “for completeness.”

---

## Product approach (implement scope)

### A. Umbrella agent script (workspace-safe) — **primary**

**`tools/pass_unblock_once.sh`** (name may refine; plain English)

Not limited to “ingest only.” One entry that:

| Subcommand / mode | Autonomy | Mutates |
|-------------------|----------|---------|
| `--status` (default) | Agent | reports under repo + `/tmp` only |
| `--agent-offline` | Agent | fingerprint + bag status + residual pointer |
| `--ingest PATH` | Agent if path known; else human one-liner | bag under workspace |
| `--ingest-pdi PATH` | Agent when file exists | `/tmp` dump + repo report |
| `--live` | Agent if unlock active **and** stock (no sudo) | last_apply under `/tmp` / recipe pins; no tree-foreign writes |
| Prints `HUMAN_NEEDED` / `PRIVILEGE_NEEDED` | | only when true |

Footer always: `invent_pass=0`, **Pass=No** unless product decide already true.

### B. Privilege script — **only if proven**

**`tools/pass_unblock_privileged_once.sh`** (or fold behind `pass_unblock_once.sh --privileged` that refuses unless proof file / env set)

- Parsimonious sandwich for hardened L3 **or** other proven need
- Full reasoning block in header + `.agents/reports/privilege-proof-*.md`
- Agent runs it when needed (operator may type password on TTY); agent does not store secrets

### C. Residual + docs

- Residual: Pass=No; unlock none; next = Layer A offline/ingest/source discovery; privilege only if wall appears
- PROVENANCE / kit: one entrypoint, not multi-tool scavenger hunt
- Outside working tree: read-only discovery only

---

## Critical files

| Path | Role |
|------|------|
| `tools/pass_unblock_once.sh` | Primary autonomous surface |
| `tools/pass_unblock_privileged_once.sh` | Optional; only if privilege proven |
| `tools/pass_unblock_fingerprint_once.sh` | Reuse offline dumps |
| `tools/operator_external_capture.sh` | Reuse install/compare internals |
| `tools/hitl_config_cu_once.sh` / kit | Stock live (Layer A) |
| `RESIDUAL.md` | Honesty + next actor |
| `.agents/reports/*` | Status, ingest, privilege proof |

---

## Steps (after Approve)

### S0 — Residual honesty

- [ ] Pass=No; unlocks CONSUMED; flexml spent; empty bag; autonomy model pointer
- [ ] Freeze empty reconfirm mill

### S1 — `pass_unblock_once.sh` (Layer A, full offline + ingest + status)

- [ ] Status / agent-offline / ingest / optional ingest-pdi / live-refuse-if-no-unlock
- [ ] `HUMAN_NEEDED` only when bytes/path truly absent
- [ ] `PRIVILEGE_NEEDED=0` unless a real wall is detected (probe, do not invent)
- [ ] bash -n; agent runs `--status` and `--agent-offline` first

### S2 — Wire kit + PROVENANCE one entrypoint

- [ ] Kit alias → `pass_unblock_once.sh`
- [ ] PROVENANCE top: one command, not five

### S3 — Privilege script **only if** S1/S2 or next live hits a real permission wall

- [ ] Document proof report
- [ ] Parsimonious enable → action → restore
- [ ] If no wall: **skip**; residual notes “privilege not required on stock host”

### S4 — Continue agent autonomy on named residual science

- [ ] Further PDI/source search **read-only** outside tree
- [ ] Ingest when path appears
- [ ] New unlock + stock live only with named offline reason
- [ ] Still no invent Pass

### S5 — Driver/MMU

- [ ] Remains parked until new evidence

---

## Risks

| Risk | Mitigation |
|------|------------|
| Over-building sudo script without need | Privilege layer only after proven wall |
| Mutating outside workspace | Hard rule: outside tree read-only; no irreversible foreign writes |
| Leaving L3 open | Privilege script restore mandatory; exit 5 if open |
| Claiming Pass from tooling | Footer invent_pass=0; Pass=No locked offline |
| Flexml re-live empty | Forbidden without new material |
| Data loss | No rm of foreign trees; `/tmp` dumps only; bag writes confined |

---

## Verification

| Check | Proof |
|-------|--------|
| Agent status | `--status` runs, Pass=No, bag empty → HUMAN_NEEDED=1 or clear next agent path |
| Offline | `--agent-offline` fingerprints without privilege |
| Ingest refuse | oracle basename exit 2 |
| Stock live path | still no sudo required on this host (document in status) |
| Privilege script | absent **or** proof report + restore verified |
| Outside tree | no agent writes into ryzen-ai-install / system |

---

## Open questions

None blocking.

- Prefer privilege as separate file vs `pass_unblock_once.sh --privileged` — default: separate file only if needed, else zero privilege surface.

---

## Honesty footer

```
invent_pass=0
product_pass=0
ran_on_npu=0
Pass=No
C_UNLOCK active=none
2ae69036 CONSUMED (flexml; not Pass)
capture_bytes_present=0
driver_mmu=parked
autonomy=full Layer A inside workspace
outside_tree=read-only
privilege_script=only if proven necessary + parsimonious + documented
human_multi_tool_homework=forbidden
```

### Critical Files for Implementation

- `tools/pass_unblock_once.sh` — primary autonomous surface
- `tools/pass_unblock_privileged_once.sh` — only if proven
- `tools/pass_unblock_fingerprint_once.sh` — reuse
- `tools/operator_external_capture.sh` — reuse
- `tools/operator_app_load_kit.sh` — alias
- `RESIDUAL.md` — Pass=No; autonomy + privilege policy
- `testdata/xdna2_cmd/external_known_good/PROVENANCE.md` — one entrypoint
