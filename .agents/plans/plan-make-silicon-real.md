# Plan: Firmware image-validation residual → real Pass

**Date:** 2026-07-20 (revised after diagnosis alignment)
**Mode:** Formal plan (explore → draft → **present / await approve** → act)
**Canonical plan file:**
`/home/hunter/.grok/sessions/%2Fhome%2Fhunter%2FProjects%2Fai%2Fdragon-npu/019f6476-0a84-70a0-a971-be89fa1011fe/plan.md`
**Workspace copy (after approval only):** `.agents/plans/plan-make-silicon-real.md`

**Workflow status**

| Phase | Status |
|-------|--------|
| understand | done (diagnosis matches driver `0x3000003`) |
| explore | done (CONFIG_CU ready; no in-tree firmware debug-BO API yet) |
| draft | done (this revision) |
| present | **awaiting approve / revise / abandon** |
| implement | after approve: **human sandwich first**; agent offline after evidence |

---

## Elevator truth (SSoT — do not dilute)

We built a serious pure-direct Ryzen NPU stack and lab harness, nearly wedged the driver once, locked safety gates, pulled real AMD matmul packaging, and reached the point where **userspace packaging looks right but firmware still will not load the job** (`AIE2_STATUS_APP_LOAD_PDI_FAIL` / `0x3000003`). Capture of a known-good working stack is empty. The next concrete measurement is **CONFIG_CU + opaque PDI on one unlocked live apply** under **`C_UNLOCK=3a70fabf`**. Until firmware accepts and OFM matches golden, **`ran_on_npu` stays false** by design.

We are **not** stuck on trivial userspace mistakes. We are at the **firmware image validation boundary** — the hardest part of a pure-direct path.

---

## Context

### Diagnosis (aligned with public amdxdna / AIE2 status)

| Code / signal | Meaning |
|---------------|---------|
| `0x3000003` | `AIE2_STATUS_APP_LOAD_PDI_FAIL` (APP ERT error range starts ~`0x3000001`) |
| WAIT completes + ERT Error | Envelope reached firmware; firmware **refused** app/partition load |
| Userspace ELF + 12 relas OK | **Necessary, not sufficient** |
| OFM zeros | Expected while load fails — secondary until residual moves |

Firmware has its own idea of a valid **PDI / CU configuration** for the partition it was given. Higher-level stacks (XRT, official Ryzen AI) register that via **`DRM_AMDXDNA_HWCTX_CONFIG_CU`** before real work. That is why the live CONFIG_CU shot is the right next experiment — not more rela/env matrix spam.

### Goal

1. **Measure** firmware reaction to CONFIG_CU + research PDI (one unlocked sandwich).
2. **Narrow** residual with honest tokens (`config_cu=1` vs `config_cu_failed` vs still no token).
3. Only then chase **PDI provenance** / working-stack capture / optional FW diagnostics.
4. Product Pass only on live ERT Completed + OFM allclose + `decide_ran_on_npu`.

### Current ground truth (host / tree)

| Fact | Value |
|------|--------|
| Residual | Userspace OK; FW **APP_LOAD_PDI_FAIL** |
| last_apply | Locked baseline (pre-CONFIG_CU live); `config_cu_present=0` |
| Unlock | **`C_UNLOCK=3a70fabf` active, unused**; `b17b0119` CONSUMED |
| Prep done | `a-prep` → research PDI ~283 840 B from host AIE2P overlay xclbin; env can be set |
| Live sandwich | **Not run** (`exec_cmd_allow` was `N` after print-only) |
| Offline p5/p6 after prep | Reconfirmed **locked fixture only** (`C3_SKIP`) — **not** new evidence |
| Machinery | CONFIG_CU + kit + p5/p6 committed (e.g. `32e51a9` impl / later plan commit) |
| Capture dir | `external_known_good/` BINS=0 |
| In-tree FW debug BO | **No** dedicated post-fail firmware debug-buffer reader yet; lab has `XDNA2_LAB_BO_DIAG`, journal, optional `tdr_dump_ctx` (stuck path — not APP_LOAD diagnostics) |

### Constraints

- Agent offline default; no L3/ALLOW from agent.
- One live shot under `3a70fabf`; unlock ≠ Pass.
- Prefer aiebu ELF EXEC; restore L3 always.
- No invent Pass / capture bytes / second unlock without named evidence.
- Empty reconfirm frozen.
- Do **not** widen experiment surface until load residual moves.

### Non-goals

- Offline gate mills without new last_apply.
- Reopen `b17b0119`.
- Treat research overlay PDI as known-good matmul.
- Product sudo-every-run.
- Implement firmware debug-BO plumbing **before** sandwich evidence (unless sandwich cannot run).

### Assumptions

- Research PDI is a **reasonable first measurement**, not guaranteed correct for custom aiebu + this FW revision.
- Still-`0x3000003` **with** `config_cu=1` is progress (narrows to blob/association).
- Successful pure-direct load contracts are only partially public; vendor stacks hide generation.

---

## Approach

### Ordered critical path (do not reorder without cause)

```text
1. Human: CONFIG_CU + PDI sandwich under C_UNLOCK=3a70fabf
   → restore L3 → new last_apply + journal
2. Agent offline: p5/p6 interpret; mark unlock used only if sandwich real
3a. If Pass → bookkeeping only (rare)
3b. If APP_LOAD + config_cu=1 → PDI / association primary suspect
    → prefer known-working XRT/DD PDI on same FW revision (capture awkward but highest value)
3c. If config_cu_failed / missing token → fix attach path offline (named delta)
4. Optional: FW diagnostic surface (debug BO / richer journal) only if load residual
   still stuck after CONFIG_CU exercised — research public uAPI first; no widen until then
5. Keep L1/L3 discipline forever
```

### Why this order

| Step | Why first |
|------|-----------|
| Live CONFIG_CU | Offline machinery done; only live measures firmware |
| Then PDI provenance | Research overlay may be wrong; only suspect after attach proven |
| Then extra diagnostics | Do not build new tools before the one-shot measurement |
| Gates unchanged | NEVER AGAIN; residual is not “open the gates wider” |

### Material alternatives rejected

| Idea | Why not now |
|------|-------------|
| More env HITL (size/layout/rela) | Closed; same APP_LOAD |
| Idle until capture bins exist | Blocks the only unlocked measurement |
| Agent opens L3 | Standing rules |
| Claim progress from p5 on locked fixture | Dishonest |

---

## Critical files

| Path | Role |
|------|------|
| `RESIDUAL.md` | Elevator + unlock SSoT |
| `docs/HITL_C_APP_LOAD_RECIPE.md` | Sandwich runbook under `3a70fabf` |
| `tools/operator_app_load_kit.sh` | `a-prep` |
| `tools/hitl_c2_once.sh` / `hitl_c_pass_apply.sh` | Live apply |
| `tools/p5_post_c2_interpret.sh` / `p6_true_pass_check.sh` | Post-shot offline |
| `src/backends/amd_xdna2/direct.rs` | CONFIG_CU attach |
| `testdata/xdna2_cmd/last_apply.json` | Evidence |
| `testdata/xdna2_cmd/external_known_good/` | Future known-good PDI/ctrltext |
| Host AIE2P xclbin (outside git) | Research PDI source only |

---

## Reuse

| Piece | Use |
|-------|-----|
| `try_lab_attach_config_cu_from_env` | Live attach when `XDNA2_LAB_PDI_BLOB` set |
| `pack_hwctx_param_config_cu` | Pack path |
| classify / C3 / C4 / P6 | Interpret; never invent Pass |
| Recipe fish/bash sandwich | Human only |

---

## Steps (after approval)

### Step 0 — Approve this revision (human)

Confirms: sandwich-first; no surface widen; elevator truth stays SSoT.

### Step 1 — Live CONFIG_CU sandwich (human) — **only critical path**

**Depends on:** active unlock, valid PDI path still on disk, sudo for L3 only.

```fish
cd /home/hunter/Projects/ai/dragon-npu
# refresh PDI if /tmp file gone:
bash tools/operator_app_load_kit.sh a-prep
set -gx XDNA2_LAB_PDI_BLOB /tmp/dragon_npu_research_pdi_….bin   # path from a-prep
set -gx XDNA2_LAB_CU_FUNC 0
# leave XDNA2_ALLOW_EXEC_CMD unset unless apply script sets it for the shot

sudo sh -c 'echo 1 > /sys/module/amdxdna/parameters/exec_cmd_allow'
bash tools/hitl_c_pass_apply.sh --assume-l3-already-open
sudo sh -c 'echo 0 > /sys/module/amdxdna/parameters/exec_cmd_allow'
cat /sys/module/amdxdna/parameters/exec_cmd_allow   # must be 0 or N

bash tools/p5_post_c2_interpret.sh
bash tools/p6_true_pass_check.sh
```

**Capture for agent (do not invent progress):**

- hitl_c / lab exit code
- new last_apply path
- detail: `config_cu=` / `config_cu_failed` / `pdi_bytes=` / `cu_func=` / `aiebu_elf=` / `rela_count=` / `ert_state=` / OFM
- journal: `Failed cmd|Status 0x|APP_LOAD|amdxdna`
- PDI provenance: path, size, sha256, xclbin source
- `exec_cmd_allow` after restore

**Mark `3a70fabf` used in RESIDUAL only after a real sandwich** (not print-only / not locked p5).

### Step 2 — Offline interpret (agent) — **only after Step 1 evidence**

| Outcome | Action |
|---------|--------|
| Pass (Completed + allclose + `ran_on_npu`) | Bookkeeping; residual closed for Pass claim |
| Still `0x3000003` + **`config_cu=1`** | Residual = **PDI / CU association / partition image** — Step 3 |
| `config_cu_failed` / no token | Named attach fix offline; no second unlock until fixed + ready |
| Same baseline fingerprint, no config_cu | Sandwich/env miss — re-run human checklist; do not mint unlock |

### Step 3 — If still APP_LOAD with CONFIG_CU present: PDI primary suspect (human + agent research)

**Prefer (highest value):** PDI (or full context setup) produced by a **known-working XRT / DynamicDispatch path on the same firmware revision**, even if capture is awkward. Install under `external_known_good/` + PROVENANCE via kit `b-install` when possible.

**Do not:** spam closed env matrix; claim research overlay is known-good.

Agent offline: soft compare when bytes exist; named pure deltas only if evidence supports.

### Step 4 — Optional firmware diagnostics (only if load residual still stuck after Step 1–3)

- Prefer **journal** + existing `XDNA2_LAB_BO_DIAG` first.
- Research whether this firmware exposes a **debug BO** / dump buffer after failed APP load (public uAPI / xdna-driver).
- **In-tree today:** no dedicated post-fail FW debug-BO reader — implement **only** if public path found and sandwich already exercised CONFIG_CU.
- Do **not** treat `tdr_dump_ctx=1` as APP_LOAD fix (stuck/TDR latch path).

### Step 5 — Safety discipline (always)

- L1 default off; L3 restore after every live shot.
- No widen to multi-flag experiments until residual class changes.
- Product users: admin-once install; no per-run sudo narrative.

### Step 6 — Freezes (ongoing)

Empty reconfirm; invent Pass; second unlock before `3a70fabf` used or without named delta after; invent capture bins.

---

## Risks

| Risk | Mitigation |
|------|------------|
| Research PDI wrong → still `0x3000003` | Expected; require `config_cu=` evidence; then Step 3 |
| Forget L3 restore | Script exit 5 + verify cat |
| p5 on locked fixture treated as progress | Explicit: not evidence |
| Build debug-BO tools too early | Step 4 only after sandwich |
| Vendor capture never arrives | Still run Step 1; residual can narrow without bins |

---

## Verification

### Pre-sandwich (optional, not a mill)

```bash
bash tools/hitl_c2_once.sh --print-only   # active 3a70fabf
test -f "$XDNA2_LAB_PDI_BLOB" && wc -c "$XDNA2_LAB_PDI_BLOB"
```

### Post-sandwich

```bash
cat /sys/module/amdxdna/parameters/exec_cmd_allow
bash tools/p5_post_c2_interpret.sh   # expect new evidence if last_apply updated
# config_cu_present / C3_EVIDENCE vs C3_SKIP
```

### Pass claim (unchanged P6)

ERT Completed + non-zero OFM allclose + real_ioctl + product `ran_on_npu` only.

---

## Open questions (defaults)

| # | Question | Default |
|---|----------|---------|
| 1 | First PDI = host AIE2P overlay dump? | **Yes** for measurement #1 |
| 2 | Implement debug BO before sandwich? | **No** |
| 3 | Second unlock if still APP_LOAD after good CONFIG_CU? | **Only** after named offline delta + explicit ready |

---

## Decision for you

| Option | Meaning |
|--------|---------|
| **Approve** | Human runs Step 1 sandwich; agent waits for last_apply then Step 2. |
| **Approve + I need sandwich commands only** | Same; reply with fish block only after approve. |
| **Revise** | Change order (e.g. capture-first). |
| **Abandon** | Stay at elevator truth; no live push. |

---

## Post-approval handoff

| Id | Owner | Work |
|----|-------|------|
| `live:config-cu-sandwich` | **Human** | Step 1 L3 sandwich + restore |
| `live:share-last-apply` | **Human** | Paste last_apply + journal + PDI provenance |
| `impl:p5-interpret-new` | Agent | Step 2 after evidence |
| `impl:pdi-provenance` | Human+Agent | Step 3 only if CONFIG_CU proven + still APP_LOAD |
| `impl:fw-debug-research` | Agent | Step 4 only if still stuck; public uAPI first |
| `impl:pass-bookkeeping` | Agent | Only on true Pass |

**Not seeded:** empty gates; auto second unlock; invent bins; pre-sandwich debug-BO feature work.

---

## One-screen summary

| Question | Answer |
|----------|--------|
| What's broken? | FW `APP_LOAD_PDI_FAIL` (`0x3000003`) |
| Userspace? | ELF+rela looks OK — not enough |
| Right next experiment? | **CONFIG_CU + PDI** under `3a70fabf` |
| Done prep? | a-prep / print-only yes; **live sandwich no** |
| After still fail + config_cu? | PDI provenance (prefer working XRT/DD same FW) |
| Debug BO? | Optional later; not in-tree yet |
| Pass today? | **No** |

---

### Critical Files for Implementation

- `RESIDUAL.md` — unlock + elevator SSoT after sandwich
- `docs/HITL_C_APP_LOAD_RECIPE.md` — human runbook
- `tools/operator_app_load_kit.sh` — a-prep
- `tools/hitl_c_pass_apply.sh` — live apply
- `tools/p5_post_c2_interpret.sh` — interpret
- `src/backends/amd_xdna2/direct.rs` — CONFIG_CU (only if attach fails)
- `testdata/xdna2_cmd/last_apply.json` — new evidence
