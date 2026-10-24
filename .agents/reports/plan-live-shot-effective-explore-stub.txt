# Plan: Make one live product matmul shot actually worth running

**Date:** 2026-08-11
**Kind:** Prep plan so option 1 (one real product NPU matmul attempt) teaches as much as possible.
**Honesty:** invent_pass=0 · Pass=**No** today · unlock **none** · do not invent Pass from prep alone.

**Explore notes:**
`.agents/reports/plan-live-shot-waste-modes.md`
(parent synthesis also from explore inventory for same-shot path)

---

## Plain English goal

We want **one** real run of our product matmul path on the NPU that:

1. Builds the job the way the lab does (ELF + rela, real buffers)
2. Patches addresses from **this run’s** buffers only (not old temp-file addresses)
3. Submits and waits with enough time and evidence
4. Scores success **only** if the device completed and the output matches golden

“Most effective” means: if it **passes**, we know; if it **fails**, we still get a clear failure class and files we can read. Not another vague timeout with empty logs.

---

## What is already ready

| Piece | Status |
|-------|--------|
| Same-run address patching code | Ready offline (W10) |
| Soft status printer for this path | Ready |
| Lab assets on disk (ELF, weights, ctrl, golden) | Present under `testdata/xdna2_cmd/silicon_lab/` (must re-check before fire) |
| Permission for a live shot | **Not active** (last one-shot already used) |
| Product Pass | **No** |

Code readiness ≠ permission to run live ≠ Pass.

---

## What makes a shot effective (checklist)

Do **not** fire until these are true (or an arm is deliberately chosen and written down):

1. **Fresh live permission** written in residual (new one-shot; old ones stay used-up).
2. **Assets green:** instruction ELF, non-zero weights, non-zero control blob, non-zero output golden. Fail closed if any missing.
3. **Output path on** so buffers exist and can be checked after.
4. **Same-run address gate on** so we prove we used this process’s addresses (and fail loud if addresses are invalid).
5. **New result file** for this shot under `/tmp` (not the old locked fixture).
6. **Outer time budget ≥ 120 seconds** so the process is not killed before writing results (30s already burned a past shot).
7. **Device wait stays finite** (a few seconds to ~15s; never “wait forever”).
8. **Buffer diagnostics on** so we can map page faults after a fail.
9. **One clear experiment arm** written down:
   - **Arm A:** no extra PDI overlay (cleanest test of EXEC + same-run patch)
   - **Arm B:** attach a PDI only if it is a *new* reason (not re-running a PDI fingerprint that already failed)
10. **After the shot is planned in advance:** journal scrape + classify + residual pin; restore any host safety flags.

---

## Approach (recommended)

### Phase 1 — Offline prep (no live yet)

Make the run hard to mess up:

1. **Wire the same-run flag into the operator/kit path** so nobody has to remember a hand export (today it is easy to forget; default path may still patch in-process, but the shot is not clearly labeled and not fail-closed the same way).
2. **Loud preflight:** assets, ELF not raw bin, golden non-zero, outer timeout, last_apply path, same-run flag present.
3. **Write the named reason** for a new live permission (plain sentence):
   “One product matmul with same-run address patching; measure Pass only by device complete + output match.”
4. **Pick Arm A or Arm B** before mint (default recommend **Arm A** for maximum clarity on EXEC residual).
5. **Run offline gates + preflight** until green.

### Phase 2 — One live shot (only after Phase 1 green)

1. Mint **one** new live permission in residual (host-minted; not invented casually).
2. Run **exactly one** product apply with the checklist env.
3. Restore host safety (no sticky allow / no L3 left open on hardened hosts).
4. Immediately collect evidence (result JSON + journal + classify).

### Phase 3 — Interpret (always, pass or fail)

1. Classify residual honestly.
2. Mark permission **used up**.
3. Pass **only** if product decide says the NPU ran and OFM matched.
4. If fail: pin the class (wait error, load reject, page fault, output mismatch, setup fail) with the evidence paths. That still counts as a successful *experiment*.

---

## What we will not do

- Re-run a known-bad research PDI just to burn another permission
- Use old temp-file addresses as the live patch
- Kill the process at 30s on a long attach path
- Call IRON success or soft checklists a product Pass
- Leave execute permission permanently on
- Second live under the same one-shot
- Public AOT re-run as if it were residual identity

---

## Critical files

| Path | Why |
|------|-----|
| `RESIDUAL.md` | Live permission source of truth |
| `tools/operator_app_load_kit.sh` | Operator/agent sandwich entry |
| `tools/hitl_c_pass_apply.sh` / `tools/hitl_config_cu_once.sh` | Live apply helpers |
| `scripts/lab_exec_once.sh` | Must forward same-run env into cargo child |
| `tools/silicon_lab_preflight.sh` | Asset gate before fire |
| `tools/collect_post_c2_evidence.sh` | Post-shot pack |
| `src/backends/amd_xdna2/direct.rs` | Same-run lab hook |
| `src/backends/amd_xdna2/silicon_contract.rs` | Same-run plan + honesty tokens |
| `testdata/xdna2_cmd/silicon_lab/` | Staged ELF / WTS / ctrl / golden |

---

## Implementation steps (after Approve)

| Step | Who | What | Size |
|------|-----|------|------|
| **S0** | Agent | Residual pin: this plan; unlock still none; named reason text drafted | 1 |
| **S1** | Agent | Kit + lab_exec: default/export/forward **same-run** flag + loud preflight echo | 2 |
| **S2** | Agent | Optional louder result fields (`same_shot=…`) on last_apply when gate on | 1 |
| **S3** | Agent | Full offline preflight green: assets, gates, sandwich print includes same-run | 1 |
| **S4** | Agent | Mint one new live permission + **one** product live under checklist (Arm A default) | 2 |
| **S5** | Agent | Collect evidence, classify, CONSUME permission, residual pin; Pass only if decide green | 1 |

**Default after Approve:** implement S0–S3 first (prep). **S4–S5 only if** prep green and this plan’s live step is still in scope (Approve of this plan = go for prep + one live when prep is green, unless operator revises to “prep only”).

**Assumption (state in plan):** Approve means **prep + one live when prep is green**. If you only want prep with no live yet, say so on Revise.

---

## Verification

### Offline (before live)

- `bash tools/silicon_lab_preflight.sh --require-pass-assets` green
- Sandwich / dry print shows same-run flag and ≥120s outer timeout
- `cargo test --features amd-xdna2 --lib w10_b_` still green
- First residual unlock line still not casually reopened

### After live

- Result JSON exists and is not the locked fixture
- Journal scrape present
- Soft classify run
- Residual says used-up permission
- `ran_on_npu` true **only** if decide green; else honest fail class

---

## Risks

| Risk | Mitigation |
|------|------------|
| Shot fails for same old EXEC residual | Still wins if same-run + bo_diag + journal prove the class |
| Kit forgets same-run env | S1 wires it; preflight fails closed if missing when required |
| Outer kill again | Force ≥120s |
| Spent PDI re-live | Default Arm A; Arm B only with new named PDI reason |
| Operator expects Pass from prep | Explicit non-claim: prep ≠ Pass |

---

## Open questions (optional freeform)

- Prefer **Arm A** (no PDI overlay) or **Arm B** (CONFIG_CU with a *new* PDI reason)? Default: **Arm A**.
- On Approve: **prep + one live** (assumed), or **prep only**?

---

### Critical Files for Implementation
- `tools/operator_app_load_kit.sh` / `tools/hitl_config_cu_once.sh` — sandwich must include same-run flag
- `scripts/lab_exec_once.sh` — forward env into cargo child
- `tools/silicon_lab_preflight.sh` — fail closed before burn
- `RESIDUAL.md` — one-shot permission
- `src/backends/amd_xdna2/direct.rs` — same-run hook already landed
- `tools/collect_post_c2_evidence.sh` — post-shot learning pack
