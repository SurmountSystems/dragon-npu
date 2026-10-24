# Plan: Pass-unblock recommended sequence (material + offline C* + live)

**Date:** 2026-08-10
**Status:** **In progress** — **P0–P2 offline done** (C1–C6 + Step 2 recipe).
**Operator refined next-moves (2026-08-10):** residual class after flexml
`2ae69036` is post-attach `wait_cmd_einval` + cmd timeout/backtrack (≠ CVML
orphan IO_PAGE_FAULT). Remaining = **material (human)** + **gated live** +
re-interpret (evidence-only). Still no Pass / no unlock mint.
**Workspace path:** `.agents/plans/plan-pass-unblock-sequence.md`
**Session plan source:** `~/.grok/sessions/…/plan.md` (mirrored here after revise)
**Workspace mirrors:**
`.agents/plans/plan-pass-unblock-creative-avenues.md` (avenues) +
`.agents/plans/plan-pass-unblock-sequence.md` (this sequence file)
**SSoT residual:** `RESIDUAL.md` top + § Creative avenues
**Reports:**
`.agents/reports/residual-pin-refined-next-moves.md` ·
`.agents/reports/impl-p0-c1-c3.md` · `impl-p1-c4-c5.md` · `impl-p2-c6-recipe.md` ·
`impl-seq-mirror.md` · `impl-residual-pin-p0.md`

**Operator sequence (locked; refined ranking):**

1. Generate/capture real **binaries** → `testdata/xdna2_cmd/external_known_good/`
   (**highest leverage**). Concrete playbook:
   [`docs/CAPTURE_MATERIAL_MLIR_AIE.md`](../../docs/CAPTURE_MATERIAL_MLIR_AIE.md)
   (mlir-aie recursive clone → npu2 env → smallest example → collect xclbin /
   insts / aiebu elf / ctrltext →
   `bash tools/pass_unblock_once.sh --ingest PATH [NAME]` — **PATH required** →
   PROVENANCE → residual pin after real files). **Source tree ≠ capture
   material.** FastFlowLM/Lemonade intercept is fallback.
2. Next authorized unlock only with named reason: unspent flexml topology
   (llm-2x4x4 `ba0cd738…` / unified-4x4 `7d172bf2…`) and/or new material +
   post-attach `wait_cmd_einval` residual from `2ae69036` + optional debug BO;
   prefer **minimal load**; full last_apply+journal.
3. Offline C* tooling **done** (P0–P2); agent does not re-spin empty offline.
4. Re-interpret only after new bytes or richer last_apply (p5/C*).

---

## Progress board (2026-08-10)

| Track | Status | Notes |
|-------|--------|--------|
| Plan mirror + residual pin after P0 | **Done** | Workspace sequence plan + `RESIDUAL.md` / `AGENTS.md` honesty pin |
| **P0** C1 + C2 + C3 | **Done** | Pure TDD in `silicon_contract.rs`; fmt/clippy/tests green |
| **P1** C4 + C5 | **Done** | IRON ingest + PDI geometry vs spent flexml; report `impl-p1-c4-c5.md` |
| **P2** C6 + live recipe checklist | **Done** | Debug-BO pack/soft + Step 2 recipe; report `impl-p2-c6-recipe.md`; no live without unlock |
| **Operator residual refine** | **Pinned** | Useful class: attach OK → `wait_cmd_einval` + timeout/backtrack; re-live spent FP banned; report `residual-pin-refined-next-moves.md` |
| **Step 1** material | **Open** (human/host) | **Highest leverage**; capture dir still bins may be **0**. Playbook: `docs/CAPTURE_MATERIAL_MLIR_AIE.md` (clone ≠ bins) |
| **Step 2** unlock + live | **Blocked** | No active unlock; reason must cite unspent image / new material + `2ae69036` residual class; do not mint without named reason |
| **Step 4** re-interpret | **Idle** | Only after new capture / last_apply / journal |
| Product Pass / `ran_on_npu` | **No** | invent_pass=0 throughout; attach ≠ Pass |

---

## Locked honesty (do not re-litigate)

| Fact | Value |
|------|--------|
| Product Pass / `ran_on_npu` | **No** — this plan does not invent Pass |
| invent_pass | **0** unless true live `decide_ran_on_npu` |
| Active unlock | **None** — all listed `C_UNLOCK` CONSUMED (`2ae69036`, `97f38307`, `11bc9016`, `3a70fabf`, `b17b0119`, …) |
| Capture `testdata/xdna2_cmd/external_known_good/` | PROVENANCE only; **bins may still be 0** |
| Offline tooling | Healthy Layer A + **P0 C1–C3** + fingerprint S4–S5 |
| Empty implement mill | **Frozen** (AGENTS.md §6–9) |
| Spent flexml default | `0eeeb70c…` / 27200 B under `2ae69036` — **re-live alone banned** (≠ progress) |
| Residual class (latest flexml) | Post-attach **`wait_cmd_einval`** + journal cmd timeout / backtrack — **≠** pure APP_LOAD; **≠** CVML orphan IO_PAGE_FAULT |
| Userspace + CONFIG_CU attach | Largely **green** on flexml stock; still no ERT Completed / OFM |
| Unspent live candidates | llm-2x4x4 `ba0cd738fd870011`; unified-4x4 `7d172bf223a9b06b` (research only) |
| Driver/MMU orphan | **Parked** until new journal evidence (flexml shot had no IO_PAGE_FAULT) |
| Offline validate / FP / split alone | **≠** C-ready · **≠** Pass · **≠** unlock |
| Empty soft-compare / status re-spin | **Frozen** |

**Product Pass still requires:** live firmware accept + ERT Completed + non-zero OFM allclose + real ioctl via product decide / C4–P6.
Material ingest, CONFIG_CU attach, fingerprint DIFF, unlock mint, offline gates — **none** equal Pass.

---

## Goal (unchanged)

Turn the creative-avenue backlog into a **single ordered plan** with:

- Clear **who** (agent vs human/host) per step.
- **Agent-first parallel offline** (sequence step 3) legal without new capture bytes.
- **Human-gated material + live** (steps 1–2) that must not be faked by agents.
- **Re-interpret only** after new evidence (step 4).
- No empty reconfirm loops; no unlock without named fingerprint + residual reason.

---

## Sequence overview (updated)

```text
  Step 3 offline C* (agent)
  ┌──────── P0 DONE ────────┐   ┌──── P1 OPEN ────┐   ┌── P2 DONE ──┐
  │ C1 validators           │   │ C4 IRON ingest  │   │ C6 debug BO │
  │ C2 structural FP        │   │ C5 PDI geometry │   │ + live recipe│
  │ C3 residual splits      │   │ vs spent flexml │   │ checklist    │
  └────────────┬────────────┘   └────────┬────────┘   └──────┬───────┘
               │                         │                   │
  Step 1 ──────┼─────────────────────────┼───────────────────┼──► capture bins ≥ 1
  material     │                         │                   │     + PROVENANCE
  (human)      ▼                         ▼                   ▼
  Step 2 ── new unlock + named reason ──► live: minimal image and/or new material
            (optional debug BO)           only when unlock active
                      │
                      ▼
  Step 4 ── re-interpret residual ──► only after new bytes / last_apply / journal
            (do not re-spin locked baseline)
```

---

## Accomplished (this campaign so far)

### Prior foundation (already green before sequence P0)

- Residual Pass-build **S0–S5**: recipe SSoT, PDI ledger, capture playbook, skip-PDI checklist, spent-fingerprint skip, xclbin inventory.
- Layer A: `tools/pass_unblock_once.sh`, fingerprint once, kit/PROVENANCE honesty.
- Creative avenues pinned: `RESIDUAL.md` + `.agents/plans/plan-pass-unblock-creative-avenues.md`.
- Live history: multiple CONSUMED unlocks; residual class includes `wait_cmd_einval`, APP_LOAD, research PDI identity; **still no Pass**.

### Sequence implement wave (P0) — landed 2026-08-10

| Item | Detail |
|------|--------|
| Plan mirror | `.agents/plans/plan-pass-unblock-sequence.md` |
| Residual / AGENTS pin | Latest Turn honesty; next = P1 or material |
| **C1** | `validate_aiebu_elf_ctrltext_structure`, `AiebuElfCtrltextValidateReason` / `Result`, `format_c1_validator_note` |
| **C2** | `StructuralLabFingerprint`, `structural_fingerprint_from_lab_detail` / `_signals` / `merge_elf`, `format_c2_structural_fingerprint_note` |
| **C3** | `ResidualClassifierSplit`, `classify_residual_split` / `_ex`, `format_c3_residual_split_note` (`wait_cmd_einval` **≠** `wait_timeout`) |
| Location | `src/backends/amd_xdna2/silicon_contract.rs` + public re-exports in `amd_xdna2.rs` |
| Tests | `c1_validate_aiebu_elf_ctrltext_structure_contracts`, `c2_structural_fingerprint_from_detail_and_elf`, `c3_residual_classifier_split_axes` |
| Gates | `cargo fmt -p dragon-npu`; clippy amd-xdna2 lib `-D warnings`; targeted lib tests + classify/construction/config_cu related green |
| Soft honesty | All new notes: `invent_pass=0 not_c_ready=1 not_ran_on_npu=1` (+ alone-not-pass tokens) |
| Report | `.agents/reports/impl-p0-c1-c3.md` |

**Explicitly not done in P0:** C4, C5, C6, unlock mint, live shot, invent capture bytes, product Pass.

---

## Remaining work (what we can still do)

### A — Agent-legal only (operator refine; no empty offline mill)

Offline **P0–P2 are done.** Agent may:

| Action | When |
|--------|------|
| Ingest/classify new human bytes | Real file arrives under capture dir or `--ingest` |
| Prepare unlock **justification** text + Step 2 checklist vs residual class | On request; host still mints `C_UNLOCK` |
| Strengthen residual splitters | **Only** if next last_apply is a **new** failure mode |
| **Refuse** | Same status scripts as progress; re-fingerprint spent default; empty soft-compare |

Optional polish (only if named; not empty reconfirm): soft-print wire for C1–C6 notes; fingerprint dump glue; bash -n if scripts change.

### B — Human / host (Step 1) — **highest leverage** for Pass path

**Playbook (concrete):** [`docs/CAPTURE_MATERIAL_MLIR_AIE.md`](../../docs/CAPTURE_MATERIAL_MLIR_AIE.md)

| Action | Detail |
|--------|--------|
| Generate or intercept material | Follow playbook: mlir-aie recursive clone + Peano/aiebu + **smallest** npu2 example (`passthrough_kernel` / `vector_scalar_add` / single_core matmul / getting_started); **or** FastFlowLM/Lemonade intercept of a stack that already loads |
| Collect | `*.xclbin`, `insts_*.bin`, aiebu elf, `.ctrltext` (what the build actually wrote) |
| Place | `testdata/xdna2_cmd/external_known_good/` **or** `bash tools/pass_unblock_once.sh --ingest PATH [NAME]` (**PATH required**; exit 2 if missing) |
| PROVENANCE | Repo+commit, example path, build cmd, npu2, sizes+sha, honesty invent_pass=0 not proven Pass on this firmware |
| Then | Soft classify / C1–C6 on real bytes; residual pin; **still invent_pass=0** until live Pass |

**Not Step 1 complete:** mlir-aie/IRON/Peano **source tree present** with no
allowed files under the capture directory.

**Agent stop:** do not invent bins; do not empty-reconfirm when still bins=0;
do not treat “clone exists” as capture progress.

### C — Live (Step 2) — gated

Only when **all** of:

1. Host-minted **new** `C_UNLOCK=<hex>` with **named offline reason**. Shape:
   unspent flexml topology (llm-2x4x4 `ba0cd738…` / unified-4x4 `7d172bf2…`)
   and/or new capture material + post-attach **`wait_cmd_einval`** residual from
   `2ae69036` + optional debug BO. **Not** re-live of spent `0eeeb70c…` alone.
2. Prefer: capture-dir material or unspent blob; **minimal load** first; optional
   **debug BO** after fail; full last_apply + journal.
3. Safety: L1/L2/L3 restore; no sticky ALLOW.

**Do not reopen:** `2ae69036`, `97f38307`, `11bc9016`, `3a70fabf`, `b17b0119`.

### D — Re-interpret (Step 4) — evidence only

| Trigger | Action |
|---------|--------|
| New capture bin | ingest → soft classify / C* → residual pin |
| New last_apply after unlock | p5 / c3 / c4 / p6 interpret path; pin CONSUMED; honesty |
| Same locked baseline only | **Do not** re-spin implement or reconfirm mill |

### E — Explicitly out of scope / parked

- Empty gate reconfirm with bins=0 and same residual.
- Soft-compare against empty capture dir as “progress.”
- Unlock mint without named fingerprint + residual reason.
- Un-parking driver/MMU without new journal evidence.
- Claiming product Pass from offline C* or CONFIG_CU attach alone.
- Re-live of spent unified-2x4x4 / `0eeeb70c…` alone.
- Vendoring proprietary full xclbin/PDI as known-good Pass.

---

## Implement phases (revised)

| Phase | Scope | Status |
|-------|-------|--------|
| **P0** | C1 + C2 + C3 | **Done** |
| **P1** | C4 + C5 | **Done** |
| **P2** | C6 + Step 2 recipe checklist | **Done** |
| **P3** | Step 1 ingest polish when material arrives | Evidence-gated (human material first) |
| **P4** | Unlock mint + live | Gated on named reason + prep; reason must fit refined residual class |
| **P5** | Step 4 re-interpret | Evidence-gated |

**Default agent posture after this pin:** wait for human material or explicit
named offline ask. Do not invent unlock. Do not re-spin empty offline C*.

---

## Critical files

| Path | Role |
|------|------|
| `src/backends/amd_xdna2/silicon_contract.rs` | C1–C5 pure surface (C1–C3 done) |
| `src/backends/amd_xdna2/txn.rs` | aiebu peel if C4/C5 need |
| `src/backends/amd_xdna2/direct.rs` | C6 debug BO constants / research wire |
| `tools/pass_unblock_once.sh` | Ingest / status / agent-offline; C4 hook |
| `tools/pass_unblock_fingerprint_once.sh` | Structural FP dump (C2/C5 glue) |
| `docs/HITL_C_APP_LOAD_RECIPE.md` / `docs/CONSTRUCTION_MAP_APP_LOAD.md` | Material + live checklists (P2) |
| `RESIDUAL.md` | SSoT after each real delta |
| `testdata/xdna2_cmd/external_known_good/PROVENANCE.md` | Capture honesty |

---

## Done when (plan success criteria)

| Track | Done when |
|-------|-----------|
| Offline C* (P0–P2) | C1–C6 (or agreed subset) landed with tests + honesty tokens; residual pin — **P0 met** |
| Material (Step 1) | Capture dir has real file + PROVENANCE **or** operator defers (honest empty still OK) |
| Live (Step 2) | Only if unlock minted: shot run, CONSUMED, last_apply shared, no invent Pass |
| Re-interpret (Step 4) | Residual updated only on new evidence |
| Product Pass | **Not** required for this plan to close offline slices |

---

## Open questions (freeform; optional)

- **Q1 (resolved for P0):** First implement was **P0 only** (C1–C3). Done.
- **Q2:** Prefer next implement **P1 only** (C4+C5), or **P1+P2** together (ingest + geometry + debug-BO prep)?
- **Q3:** Is host ready to install mlir-aie/IRON (or drop a capture path) for Step 1, or stay agent offline-only until a file appears?
- **Q4:** Debug BO: env-gated lab wire in product path vs constants/docs only until first unlock? (Default: constants/docs first.)

**Default if no answers:** implement **P1** (C4+C5) offline only; no unlock mint; Step 1 human when ready; P2 after P1 or as docs-only parallel.

---

## Handoff after Approve of this revise

1. Refresh workspace mirror `.agents/plans/plan-pass-unblock-sequence.md` from this session plan.
2. Keep residual SSoT aligned (already P0-pinned).
3. Next implement: **P1 C4 + C5** pure TDD + residual honesty pin.
4. Do **not** empty reconfirm; do **not** mint unlock without named reason; do **not** re-interpret locked baseline alone.

### Suggested next implement prompt (agent-doable)

```text
implement Pass-unblock sequence P1 (offline C4+C5 only):
1) C4 IRON/public artifact ingest path (allowed paths + classify hook; empty still blocked; invent_pass=0)
2) C5 offline PDI geometry walker vs spent flexml fingerprints (re-match ≠ unlock alone)
Reuse C1–C3. No unlock mint. No Pass claim. No invent capture bytes.
Residual honesty pin after land.
```

(Leading slash omitted here so auto-run does not spin until you intend it.)
