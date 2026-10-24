# Plan: Build backlog for residual Pass-unblock (three needs)

**Date:** 2026-08-10
**Mode:** approved; Phase 0 **S0–S3** + Phase 1 **S4–S5** landed
**Session plan / workspace mirror:** `.agents/plans/plan-residual-pass-build.md`
**Explore evidence:** `/tmp/grok-1000/explore-residual-pass-build.md`
**SSoT residual:** `RESIDUAL.md` top
**Implement reports:** `.agents/reports/impl-residual-pass-build-s0-s3.md`,
`.agents/reports/impl-residual-pass-build-s4-s5.md`

**Operator ask:** deeply and thoroughly plan **what to build** to address residual tasks that still unblock Pass (three-need table unchanged).

### Phase 0 implement status (S0–S3)

| Step | Work | Status | Acceptance |
|------|------|--------|------------|
| **S0** | H1 Unlock SSoT scrub | **Done** | HITL_C recipe + HITL_BC + CONSTRUCTION_MAP: **no** active unlock; `2ae69036` CONSUMED flexml; not “Active 11bc9016” |
| **S1** | A1+A2 spent/unspent PDI ledger | **Done** | `RESIDUAL.md` elevator + CONSTRUCTION_MAP table; spent flexml 27200 / unspent llm + 4x4 / spent CVML |
| **S2** | B1 public-stack ctrltext extract playbook | **Done** | CONSTRUCTION_MAP § + PROVENANCE short form; ingest via `pass_unblock_once.sh --ingest`; refuse oracles; invent_pass=0 |
| **S3** | A7 skip-PDI vs CONFIG_CU checklist | **Done** | CONSTRUCTION_MAP checklist; points at prep-skip-pdi / SKIP_CONFIG_CU; new unlock each live; never auto-mint |

### Phase 1 implement status (S4–S5)

| Step | Work | Status | Acceptance |
|------|------|--------|------------|
| **S4** | A5 fingerprint skip re-dump spent default | **Done** | Ledger footer; skip `0eeeb70c…` re-dump unless `--force` / `XDNA2_FP_FORCE_REDUMP=1`; spent DIFF ≠ new unlock reason; invent_pass=0 |
| **S5** | A4 read-only `*mladf*` / `*gemm*.xclbin` inventory | **Done** | `.agents/reports/pdi-xclbin-inventory.md`; missing roots OK; empty OK; no invent |

**Not done:** S6–S9; any live; unlock mint; Pass claim.

---

## Locked honesty (do not re-litigate)

| Fact | Value |
|------|--------|
| Product Pass / `ran_on_npu` | **No** — this plan does not invent Pass |
| invent_pass | **0** unless true live decide |
| Active unlock | **None** — first `C_UNLOCK=` is CONSUMED (`2ae69036` flexml, plus prior stack) |
| Flexml live `2ae69036` | CONFIG_CU attach OK (27200 B, sha16 `0eeeb70c…`); status `wait_cmd_einval`; **not Pass**; **spent** |
| Capture `external_known_good/` | PROVENANCE only; bins = 0 |
| Driver/MMU orphan | **Parked** — flexml shot had **no** IO_PAGE_FAULT; CVML orphan class not the active residual |
| Privilege script | **Not required** on stock host (`exec_cmd_allow` sysfs missing) |
| Layer A entrypoint | **Landed** — `tools/pass_unblock_once.sh` |

**Product Pass still requires (unchanged):** live firmware accept + **ERT Completed** + non-zero OFM allclose + real ioctl via product `decide_ran_on_npu` / C4–P6.
CONFIG_CU attach, fingerprint DIFF, bag-drop, soft match, offline gates, unlock mint — **none** equal Pass.

---

## What still unblocks Pass (unchanged table)

| Need | How | Physics vs build |
|------|-----|------------------|
| Working-stack ctrltext in the capture dir | When you have a real file: `bash tools/pass_unblock_once.sh --ingest /path/to/ctrltext.bin` | **Human/agent holds bytes**; tooling largely done |
| True matmul-matched PDI beyond spent flexml | Offline discovery + new unlock only with a new fingerprint | **Discovery/discipline buildable**; true gemm xclbin may be vendor-missing |
| Driver/MMU orphan class | Parked until new evidence | **No product build** until un-parked |

---

## Goal of this plan

Define a **build backlog** (code, docs, soft tools) that:

1. Maximizes Layer A autonomy for everything offline.
2. Does **not** invent capture bytes, Pass, or empty reconfirm mills.
3. Makes spent vs unspent experiments explicit so agents cannot re-live flexml as “progress.”
4. Leaves driver/MMU parked with clear un-park conditions.
5. Keeps a single human-facing entrypoint (`pass_unblock_once.sh`) — no multi-tool scavenger hunts.

**Non-goals**

- Invent product Pass offline.
- Reopen CONSUMED unlocks (`2ae69036`, `97f38307`, `11bc9016`, `3a70fabf`, `b17b0119`).
- Privilege theater script without a proven wall.
- Product MMU/driver “fix” without new journal evidence.
- Commit proprietary PDI/xclbin into git.
- Second empty gate reconfirm with bins=0 and same residual.

---

## Context: what is already built (do not rebuild)

### Layer A / capture / PDI machinery (landed)

| Area | Key surfaces |
|------|----------------|
| Entrypoint | `tools/pass_unblock_once.sh` — `--status`, `--agent-offline`, `--ingest`, `--ingest-pdi`, live-refuse when no unlock |
| Capture install/compare | `tools/operator_external_capture.sh`; kit aliases `b-status` / `b-install` / `b-compare` / `pass-unblock` |
| Capture confinement | `external_known_good_path_is_allowed`, load helpers, refuse local oracles |
| Soft compare / scorecards | fingerprint export, external compare, bag-drop dry-run, honesty tokens (`ctrltext_match_alone_closes_fw_residual=0`) |
| PDI dump + fingerprint | `tools/dump_xclbin_pdi.sh`, `pass_unblock_fingerprint_once.sh`, CONFIG_CU attach path |
| Research PDI identity | `fingerprint_pdi_blob_bytes`, `research_pdi_identity_from_*`, preflight notes |
| IO_PAGE_FAULT audit | T3b pure APIs + unit lock orphan `0x2fff80007f40` |
| WAIT honesty | `wait_cmd_einval` (not false `wait_timeout`) |
| True Pass criteria offline | P6 checklist / `evaluate_silicon_pass_evidence` — **maintain only** |
| Stock live runner | `operator_app_load_kit.sh config-cu` / `config-cu-run` (only with active unlock) |

### Measured fingerprint ledger (offline + live)

| tag | pdi_bytes | sha16 | Live |
|-----|-----------|-------|------|
| flexml_unified-2x4x4 | 27200 | `0eeeb70c84df056e` | **Spent** `2ae69036` — still not Pass |
| flexml_llm-2x4x4 | 21664 | `ba0cd738fd870011` | **Not live** (unspent candidate) |
| flexml_unified-4x4 | 13792 | `7d172bf223a9b06b` | **Not live** (weaker topology A/B) |
| cvml_AMD_AIE2P_4x4_Overlay | 283840 | `a1e65d04add3422c` | **Spent** prior unlocks (orphan class) |
| cvml_AMD_AIE2P_8x4x1 | 568224 | `19c430f04909c280` | Not preferred |

**Missing offline:** npu4 shim-test `gemm.xclbin`, free-standing `*mladf*2x4x4*.xclbin` as loose files (txn bins exist; those are ctrltext, not PDI).

---

## Approach (recommended path)

**Build what improves discovery, honesty, and experiment discipline.**
Do **not** build a second capture mill or invent matmul-known-good PDI from code.

Three workstreams map 1:1 to residual needs:

| Stream | Build weight | Leverage for Pass |
|--------|--------------|-------------------|
| **B — Capture ctrltext** | Small polish + docs playbook | High **if** human supplies bytes; tools ready |
| **A — PDI / program identity** | Medium: ledger, taxonomy, inventory, live discipline | Medium — unspent flexml FPs or vendor gemm; still not guaranteed Pass |
| **M — Driver/MMU** | Near-zero until un-park | Low for current flexml residual; keep audit |

**Dependency order**

```text
Honesty locks (always)
    │
    ├─► Docs SSoT scrub (active unlock false claims)
    │
    ├─► Path B: playbook + optional PROVENANCE-table check
    │     Human/agent: --ingest when bytes exist → soft compare
    │     Live of corrected instr: NEW unlock only after named delta
    │
    ├─► Path A: spent/unspent ledger + inventory + optional taxonomy
    │     Live unspent FP or skip-PDI: NEW unlock + named offline reason
    │     Never re-live spent 27200 as sole progress
    │
    └─► Path M: PARKED
              Un-park only if orphan class returns with new evidence
                        ▼
              Future live under NEW C_UNLOCK → last_apply → p5/c3/c4/p6
              Pass only if ERT Completed + OFM allclose
```

---

## Stream B — Working-stack ctrltext (`external_known_good/`)

### Blocking physics (not code)

- Capture dir has no non-oracle `.bin` / `.ctrltext`.
- Soft helpers correctly report `blocked_on_external_bytes=1`.
- Bytes must come from a **public stack that completes** on NPU (XRT / DynamicDispatch / Ryzen AI EP), not lab oracles (`gt_capture/*` refused).
- Agent **cannot invent** those bytes.

### What to build

| ID | Deliverable | Type | Why | Acceptance (invent_pass=0) |
|----|-------------|------|-----|----------------------------|
| **B1** | **Public-stack ctrltext extract playbook** | Docs | Operator knows exact steps → bag → PROVENANCE → compare; single entrypoint | Doc points at real paths; refuse-oracle list; `pass_unblock_once.sh --ingest`; no invent-bytes language |
| **B2** | **PROVENANCE capture-table completeness soft check** | Optional Rust and/or shell | bins>0 but blank capture table can look “ready” while provenance is incomplete | Soft token `PROVENANCE_TABLE_INCOMPLETE=1` when bins>0 and table empty; still `bag_drop_alone_closes_fw_residual=0` |
| **B3** | **Post-ingest soft compare nudge** | Optional shell in entrypoint | After successful `--ingest`, print (or optionally run) `b-compare` next step | No L3/ALLOW; soft only; fail-closed if bag empty |

### What not to build (B)

- More bag/scorecard composition APIs without bytes.
- Empty reconfirm of bag_drop while bins=0.
- Auto-mint unlock from soft match.

### Human / agent after B builds

| Who | Action |
|-----|--------|
| Whoever holds bytes | `bash tools/pass_unblock_once.sh --ingest /path/to/ctrltext.bin [NAME]` |
| Operator | Fill capture table in `PROVENANCE.md` |
| Agent | Soft compare interpretation; residual notes; **still invent_pass=0** |
| Live of corrected instr | **New** host-minted unlock + named offline reason only |

---

## Stream A — True matmul-matched PDI beyond spent flexml

### Blocking physics

1. Flexml `unified-2x4x4` **live-spent** under `2ae69036` — attach measured, residual still `wait_cmd_einval` + FW timeout. Re-live same unlock **forbidden**. Re-live same blob needs **new** unlock **and** a named reason that is not “reconfirm same FP.”
2. CVML research overlays **spent** with orphan IO_PAGE_FAULT class.
3. **No** proven “mladf matmul product known-good” PDI on disk. Construction map: DD supplies **ctrltext/txn**; xclbin supplies **PDI/CU**. Matching txn ≠ selecting matmul PDI.
4. Shim-test / vendor **gemm xclbin** missing offline.
5. Taxonomy: `ResearchPdiSourceKind` is only Absent / CvmlAie2pResearchOverlay / UnknownOpaque — no flexml/matmul-matched kinds (and even a new kind must stay `product_known_good=false` until live Pass).

### What to build

| ID | Deliverable | Type | Why | Acceptance (invent_pass=0) |
|----|-------------|------|-----|----------------------------|
| **A1** | **Spent vs unspent PDI ledger** | Docs + soft report section and/or fingerprint script footer | Prevent re-live spent FPs as progress | Table: `unified-2x4x4` spent; `llm-2x4x4` / `unified-4x4` not spent; CVML spent; always not product known-good |
| **A2** | **Docs residual honesty** (HITL recipe + “next live” notes) | Docs | Stale “Active 11bc9016” / “next = flexml” misleads | Matches `RESIDUAL.md`: no active unlock; flexml unified spent; unspent candidates named |
| **A3** | **`ResearchPdiSourceKind` flexml/path taxonomy** | Optional Rust TDD | Clearer soft classify after dumps | Unit: flexml path → kind ≠ CVML; always `research_pdi_not_product_known_good=1` / never product_known_good |
| **A4** | **Read-only inventory extend** for `*mladf*`, `*gemm*.xclbin` under install trees | Shell/report | Find better-than-flexml if install grows | Read-only outside tree; report paths/sizes; empty result OK; no invent |
| **A5** | **Fingerprint: skip re-dump of spent default** unless `--force` / new path | Shell polish | Agent-offline must not pretend new reason from same sha16 | Same sha16 → “already measured / spent” note; no fake unlock justification |
| **A6** | **Multi-CU / cu_func research offline** | Docs + optional soft parse of `xclbinutil --info` | Flexml used `cu_func=0` only | Unit locks on soft parse if added; live multi-CU only under **new** unlock |
| **A7** | **Skip-PDI vs CONFIG_CU experiment checklist** | Docs | Discriminate attach vs map residual | Requires **new** unlock each live; never auto-mint; invent_pass=0 |

### What not to build (A)

- Private PDI layout decoder / invent known-good.
- Re-live spent 27200 as sole implement goal.
- Unlock mint from fingerprint table alone.
- Commit flexml/CVML blobs into git.

### Live experiments (after offline delta + new unlock only)

| Experiment | Needs new unlock? | Named offline reason |
|------------|-------------------|----------------------|
| Live `llm-2x4x4` PDI (21664 / `ba0cd7…`) | **Yes** | Unspent FP ≠ spent flexml/CVML |
| Live `unified-4x4` | **Yes** (weaker) | Topology A/B only |
| Re-live spent flexml 27200 | Prefer **forbid** | Only if combined with other named delta (cu_func, skip-PDI, capture-driven instr fix) |
| Skip-PDI stock run | **Yes** | Discriminates attach vs map |
| Pass claim after attach | **Never** | |

**Honesty:** true matmul-matched PDI may still be **unavailable offline**. Product can improve discovery and discipline; **bytes** may remain human/vendor.

---

## Stream M — Driver / MMU orphan (parked)

### Current residual class

| Shot family | Journal class | Implication |
|-------------|---------------|-------------|
| CVML (prior unlocks) | Orphan IO_PAGE_FAULT ~`0x2fff8000…` vs lab BO map | T3b locked; not fixed by userspace map audit alone |
| Flexml `2ae69036` | **No** IO_PAGE_FAULT; `aie2_error_backtrack` + `aie2_set_cmd_timeout` + `wait_cmd_einval` | Active residual is **not** orphan-MMU for this path |

### What to build now

| ID | Deliverable | When |
|----|-------------|------|
| **M0** | **Nothing product** (park default) | Until un-park evidence |
| **M1** (optional hygiene) | Soft residual note: `wait_cmd_einval` + timeout **without** page fault (flexml-style class) | Optional small classify polish; does not invent `0x3000003` APP_LOAD |
| **M2** (optional hygiene) | Err-path emit `io_paddrs` on submit_wait Err (parity with Ok) | Low priority; not root cause |

### Un-park conditions (only then schedule real work)

- New journal with multi-fault / PASID / domain detail, **or**
- Matmul-matched PDI live that **returns** orphan class, **or**
- Public-stack success map proving device maps correctly on same host, **or**
- Operator expands scope to hardened driver / dyndbg install.

### Forbidden until un-park

- Product “MMU fix” driver patch without evidence.
- Empty reconfirm of T3b orphan unit.
- Privilege script for MMU theater.

---

## Cross-cutting honesty builds

| ID | Deliverable | Files (primary) | Acceptance |
|----|-------------|-----------------|------------|
| **H1** | Unlock SSoT scrub | `docs/HITL_C_APP_LOAD_RECIPE.md` (header unlock table); any CONSTRUCTION_MAP live stamp claiming active `11bc9016` | Story matches `RESIDUAL.md`: **no** active unlock; `2ae69036` CONSUMED flexml |
| **H2** | Residual elevator: three-need table + spent ledger pointer | `RESIDUAL.md` top (only if out of date after approve implement) | Plain English first; invent_pass=0; no empty reconfirm mill |
| **H3** | Soft residual class for flexml-style path (optional) | `silicon_contract.rs` soft classify | Distinct from APP_LOAD `0x3000003`; invent_pass=0; not_c_ready |

**Already landed — do not rebuild:** P6 true-Pass checklist, C3/C4 offline, progressive last_apply stamps, stock config-cu-run, pass_unblock_once Layer A.

---

## Numbered implement steps (after Approve)

Ordered for dependency and leverage. Each leaf is agent-doable offline unless marked live/human.

### Phase 0 — Honesty / docs (size 1 each)

| Step | Work | Size | Depends |
|------|------|------|---------|
| **S0** | H1: HITL recipe + stale active-unlock scrub | 1 | — |
| **S1** | A1+A2: spent/unspent PDI ledger in residual and/or fingerprint report footer | 1 | S0 preferred |
| **S2** | B1: public-stack ctrltext extract playbook (docs only) | 1 | S0 preferred |
| **S3** | A7: skip-PDI vs CONFIG_CU experiment checklist (docs) | 1 | S1 |

### Phase 1 — Soft tooling polish (optional; size 1–2)

| Step | Work | Size | Depends |
|------|------|------|---------|
| **S4** | A5: fingerprint skip re-dump of spent default unless `--force`/new path | 1 | S1 |
| **S5** | A4: read-only inventory for `*mladf*` / `*gemm*.xclbin` → report under `.agents/reports/` | 1 | — |
| **S6** | A3: `ResearchPdiSourceKind` flexml/path taxonomy + unit tests | 2 | — |
| **S7** | B2: PROVENANCE table completeness soft check | 1 | — |
| **S8** | B3: post-ingest compare nudge in `pass_unblock_once.sh` | 1 | — |
| **S9** | H3/M1: soft residual class for wait_cmd_einval + timeout without page fault | 1 | — |

### Phase 2 — Live (not auto-scheduled; gates)

| Step | Work | Gate |
|------|------|------|
| **L0** | New host-minted unlock **only** with named offline reason | Unlock ≠ Pass |
| **L1** | Unspent PDI live (prefer `llm-2x4x4`) **or** skip-PDI A/B **or** capture-driven instr apply | Active unlock + stock path |
| **L2** | Interpret last_apply; CONSUME unlock; p5/c3/c4/p6; Pass only if live decide | invent_pass=0 unless Pass |

### Phase 3 — Parked (do not implement as Pass-unblock)

| Step | Work |
|------|------|
| **P-MMU** | Driver/MMU product fix |
| **P-PRIV** | Privilege script without proven wall |
| **P-EMPTY** | Empty reconfirm gates with no new files/evidence |

**Recommended first implement slice after Approve:** **S0 → S1 → S2 → S3** (docs honesty + ledger + playbooks). Then S4–S5 if inventory/discipline polish is wanted. Defer S6–S9 unless operator wants soft taxonomy. Live only when new material exists.

---

## Critical files

| Path | Role |
|------|------|
| `RESIDUAL.md` | SSoT unlock + residual elevator |
| `AGENTS.md` | Standing silicon/HITL rules |
| `docs/HITL_C_APP_LOAD_RECIPE.md` | Stale active unlock scrub (H1) |
| `docs/CONSTRUCTION_MAP_APP_LOAD.md` | Construction hypothesis; live stamp honesty |
| `docs/HITL_BC.md` | Cross-links only if drift |
| `tools/pass_unblock_once.sh` | Layer A entrypoint (B3, live-refuse) |
| `tools/pass_unblock_fingerprint_once.sh` | Ledger footer / skip spent re-dump (A1/A5) |
| `tools/dump_xclbin_pdi.sh` | PDI candidates |
| `tools/operator_external_capture.sh` | Capture install/compare |
| `tools/operator_app_load_kit.sh` | Kit aliases / config-cu |
| `testdata/xdna2_cmd/external_known_good/PROVENANCE.md` | Capture provenance template |
| `src/backends/amd_xdna2/silicon_contract.rs` | Soft classify, PDI identity, fault audit, Pass evidence |
| `.agents/reports/live-shot-2ae69036.md` | Flexml live pin |
| `.agents/reports/pass-unblock-fingerprint-once.md` | Fingerprint table |

**Outside tree (read-only only):**

- Flexml STX: `…/ryzen-ai-install/venv/.../flexml/.../stx/`
- CVML: `…/RyzenAI-SW/Ryzen-AI-CVML-Library/linux/onnx/ryzen14/`

---

## Reuse (do not reinvent)

- Capture confinement + compare pure APIs (already tested).
- `ResearchPdiIdentity` / fingerprint helpers.
- T3b `audit_io_page_fault_vs_lab_map`.
- P6 / C4 evaluate helpers.
- `pass_unblock_once.sh` as the **only** human-facing multi-step umbrella.

---

## Risks and mitigations

| Risk | Mitigation |
|------|------------|
| Empty reconfirm mill | Hard stop: bins=0 + same residual → no new implement stamp |
| Re-live spent flexml as progress | Spent ledger (A1); refuse as sole named reason |
| Stale docs claim active unlock | S0 scrub first |
| Soft match mistaken for Pass | Honesty tokens always invent_pass=0 not_ran_on_npu |
| Outside-tree writes | Read-only; ephemeral `/tmp` dumps only |
| Privilege theater | Stock host needs no L3 script |
| Vendor gemm never appears | Inventory honest empty; path B + unspent flexml remain options |
| MMU “fix” distraction | Parked until un-park evidence |

---

## Verification (after implement slices)

| Slice | Verify |
|-------|--------|
| Docs (S0–S3) | Manual: `RESIDUAL` unlock story matches recipe; no “Active 11bc9016”; ledger lists spent/unspent |
| Shell polish (S4–S5, S8) | `bash -n` on touched scripts; `--agent-offline` / `--status` smoke; invent_pass=0 footer |
| Rust (S6, S7, S9) | Red→green unit tests for named contracts; `cargo test --features amd-xdna2 --lib` filters; clippy -D on touched; fmt |
| Live (L*) | Only with active unlock; last_apply pin; CONSUME; **no** Pass invent |
| Full offline | Optional `run_offline_gates` only if product code changed — **not** empty reconfirm |

---

## What NOT to build (summary ban list)

- Empty `/implement` reconfirm with no new files/evidence.
- Reopen CONSUMED unlocks.
- Mint unlock from offline-green alone.
- Invent capture bytes / product known-good PDI.
- Privilege script without `exec_cmd_allow` wall.
- Product MMU driver patch without un-park evidence.
- Multi-tool scavenger hunt replacing `pass_unblock_once.sh`.
- VE2 / validate.xclbin as AIE2 known-good.
- Commit proprietary xclbin/PDI.

---

## Who does what next (plain English)

| Who | Do |
|-----|----|
| **Agent (after Approve)** | S0–S3 first: docs honesty, spent ledger, extract playbook, skip-PDI checklist. Then optional S4–S9 soft polish. |
| **Whoever has ctrltext path** | `bash tools/pass_unblock_once.sh --ingest …` + fill PROVENANCE. |
| **Human / install (optional)** | Obtain npu4 gemm / real mladf xclbin if flexml family exhausted. |
| **Live** | Only after named offline delta + **new** host-minted unlock. |
| **Nobody** | Empty mill; invent Pass; privilege theater; reopen CONSUMED. |

---

## Open questions (non-blocking; freeform)

- Prefer implement **docs-only first slice (S0–S3)** only, or include optional soft taxonomy (S6) in the same approve?
  **Recommendation:** docs-first; soft optional in a second slice.
- Is unspent `llm-2x4x4` an acceptable next live candidate if capture still empty, or wait for capture bytes only?
  **Recommendation:** either is valid offline prep; live still needs new unlock + operator intent for device risk.
- Expand outside-tree scope later for hardened driver work? Default **no** until un-park.

---

## One-line summary

Layer A tools are in place; **build** residual Pass-unblock as **honesty docs, spent/unspent PDI discipline, capture playbook, optional soft taxonomy/inventory** — not empty reconfirm, not invent Pass, not MMU theater. Real Pass still needs working-stack **ctrltext** and/or better-than-spent-flexml **PDI/program** plus live ERT+OFM.

---

### Critical Files for Implementation

- `docs/HITL_C_APP_LOAD_RECIPE.md` — stale active unlock scrub
- `RESIDUAL.md` — SSoT ledger + residual elevator
- `tools/pass_unblock_once.sh` — entrypoint polish
- `tools/pass_unblock_fingerprint_once.sh` — spent ledger / skip re-dump
- `tools/dump_xclbin_pdi.sh` — PDI candidates
- `tools/operator_external_capture.sh` — capture path
- `src/backends/amd_xdna2/silicon_contract.rs` — optional taxonomy / soft class
- `testdata/xdna2_cmd/external_known_good/PROVENANCE.md` — capture template
- `docs/CONSTRUCTION_MAP_APP_LOAD.md` — construction honesty
