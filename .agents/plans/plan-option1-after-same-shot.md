# Plan: Make option 1 effective after same-shot live evidence

**Date:** 2026-08-11
**Kind:** Next plan for product Pass path after option 1 already ran once.
**Honesty:** invent_pass=0 · Pass=**No** · unlock **none** (first `C_UNLOCK=` = **`2c9c12e8` CONSUMED**)

**Explore inputs:**
- `.agents/reports/plan-option1-effective-explore.md`
- `.agents/reports/plan-option1-wait-einval-explore.md`
- Residual Latest Turn S4–S5 / live-shot `2c9c12e8`

---

## Context

### What “option 1” is

One **product** NPU matmul attempt that:

1. Uses the lab product path (ELF + rela, real buffers, OFM check)
2. Patches addresses from **this process only** (same-run / same-shot)
3. Submits and waits with enough evidence
4. Scores **Pass only** via `decide_ran_on_npu` (real_ioctl + ERT Completed + OFM allclose)

### What already happened (do not re-prove)

| Fact | Evidence |
|------|----------|
| Same-shot **works live** | `same_shot=1 paddrs_valid=1 source=in_process_bo_map` under `2c9c12e8` |
| Prep kit (flag forward, tokens, ≥120s outer) | S0–S3 of live-shot-effective **done** |
| Arm A (no CONFIG_CU) still fails | status=`wait_cmd_einval`, ~0.16s, `ran_on_npu=false` |
| Same-shot is **necessary hygiene, not Pass** | Residual class **unchanged** vs prior skip-CU |

**`wait_cmd_einval` means:** WAIT_CMD returned EINVAL almost immediately (wait **refused**). It is **not** a full-budget hang (`wait_timeout`), **not** historical APP_LOAD after successful wait, **not** OFM mismatch after Completed.

### What still blocks Pass

Pass still needs ERT **Completed** + non-zero OFM **allclose**. Primary wall remains:

- **Instruction / load identity** still lab classic unpatched oracle path (not residual-matched / load-accepted completing-stack material)
- Open: job/out-fence never published for WAIT; optional wait-path parity gap (product always WAIT_CMD)

**Assumption:** Approve means offline material + wait prep first; **one live only if join writes a new named reason**. Default is **not** mint + same Arm A again.

---

## Approach

**Do not re-run the previous “make the live shot clean” prep.** That work succeeded as science and failed as Pass.

**Most effective next:** change **what the firmware runs** (Bridge A / load-accepted peel) and/or **how completion is observed** (wait-path parity offline), then burn **one** unlock only with a **new** named reason.

### Recommended path

1. **Pin honesty** (same-shot proven; freeze empty reconfirm).
2. **Parallel offline:** (A) residual-matched material hunt / Bridge A; (B) wait-path observability + optional syncobj pack (not Pass).
3. **Join:** rank **one** live reason in plain English, or stop if neither arm gained evidence.
4. **Live only if justified:** new unlock + one shot (same-shot still on as hygiene) + CONSUME + interpret.
5. Pass **only** if product decide says yes.

### Not this plan

- **Not** empty same-shot reconfirm live under a new mint
- **Not** second live under `2c9c12e8` or any CONSUMED unlock
- **Not** re-live spent flexml PDI fingerprints alone
- **Not** invent Pass from soft tokens, IRON, attach, or same_shot=1
- **Not** bump wait timeout alone as the fix for EINVAL
- **Not** treat public mlir-aie AOT peels as residual identity

---

## Critical files

| Path | Why |
|------|-----|
| `RESIDUAL.md` | Unlock SSoT; residual class; primary wall |
| `src/backends/amd_xdna2/direct.rs` | Same-shot lab hook; WAIT_CMD; hwctx / syncobj |
| `src/backends/amd_xdna2/silicon_contract.rs` | same_shot plan; wait axes; material match honesty |
| `tools/operator_app_load_kit.sh` | Live sandwich entry when live is justified |
| `scripts/lab_exec_once.sh` | Env forward (SAME_SHOT_RELA, …) |
| `testdata/xdna2_cmd/silicon_lab/` | Staged product ELF / WTS / ctrl / golden |
| `testdata/xdna2_cmd/external_known_good/` | Landing for load-accepted / residual-matched peels |
| `docs/CONSTRUCTION_MAP_APP_LOAD.md` | CU/PDI vs ctrltext construction story |
| `.agents/reports/live-shot-2c9c12e8.md` | Same-shot live evidence |
| `.agents/plans/plan-live-shot-effective.md` | Prior option-1 prep (S0–S5 **closed**) |

## Reuse

| Piece | Path | How |
|-------|------|-----|
| Same-shot plan + gate | `silicon_contract.rs` / `direct.rs` | Keep on for any product live; do not re-prove |
| Wait EINVAL axes / soft | `silicon_contract.rs` | Extend only if new last_apply tokens needed |
| Capture / host scan tools | `tools/host_capture_scan.sh`, collect helpers | Prefer before inventing new scripts |
| Bridge A prior | `.agents/reports/wave9-a-bridge-a.md` | Continue from INSTR_BO_LIMIT wall, not from zero |
| Kit + preflight | `operator_app_load_kit.sh`, `silicon_lab_preflight.sh` | Use as-is for justified live |

---

## Steps

| Step | Who | What | Size |
|------|-----|------|------|
| **P0** | Agent | Residual + AGENTS honesty pin: same-shot live proven under `2c9c12e8` CONSUMED; primary wall = residual-matched material (+ optional wait theory); freeze empty same-shot reconfirm; unlock **none** | 1 |
| **P1-A** | Agent | **Bridge A / material track (offline):** hunt residual-matched or load-accepted classic-1688 / completing-stack peel; refuse multi 32208 and public AOT-as-identity; document exact wall if still blocked | 2 |
| **P1-B** | Agent | **Wait-path offline:** ensure create/detail can show syncobj handle if present; unit/pack for optional drm_syncobj wait research path; soft invent_pass=0; **≠ unlock ≠ Pass** | 2 |
| **P2** | Agent | **Join:** one plain-English named live reason **or** explicit stop (neither arm justified). Prefer material arm if material landed; else wait-path arm only with a **new** wait theory. Never dual arms under one stamp | 1 |
| **P3** | Agent | **Only if P2 justified:** mint **one** new host unlock + **one** product live (same-shot still on as hygiene; outer ≥120s; new last_apply under `/tmp`) | 2 |
| **P4** | Agent | Evidence pack + classify + CONSUME + residual pin; Pass only if decide green; else pin new class and **stop** empty mill | 1 |

**Default order after Approve:** P0 → **P1-A ∥ P1-B** → P2 → (P3–P4 only if P2 says live).

**If material is a human-only wall** (sudo L3, private stack, large instrumented build): agent **stops** at P2 with the proven wall once; operator places peel under `testdata/xdna2_cmd/external_known_good/` with PROVENANCE, or expands scope.

---

## Risks

| Risk | Mitigation |
|------|------------|
| Re-burning unlocks on same residual | Freeze empty same-shot reconfirm; P2 gate before mint |
| False “residual match” from public AOT | Scorecard honesty; residual_matched only with real claim criteria |
| Treating syncobj wire as Pass | Soft always invent_pass=0; live only with named wait theory |
| Journal noise from older flexml shots | Tight scrape window to this shot’s wall (~0.16s skip-CU) |
| Scope creep into driver/MMU without evidence | Offline first; human sudo only after agent names need |

---

## Verification

### Offline (before any live)

- Residual first unlock line still **`2c9c12e8` CONSUMED** (no casual reopen)
- P1-A report: either new sha/path for residual-matched material **or** exact wall documented
- P1-B: unit/pack green for wait observability; soft notes invent_pass=0
- P2 join report states **live justified: yes/no** with one named reason

### If live (P3–P4)

- New unlock only; CONSUMED after one shot
- last_apply not the locked fixture; same_shot tokens still present when gate on
- `ran_on_npu` true **only** if decide green; else honest fail class
- No sticky ALLOW / L3 left open

### Red/green where product behavior changes

- Any new pure classifier / material-match / wait-token helpers: write failing unit first, then green the same filter under `cargo test --features amd-xdna2 --lib …`
- Docs/residual-only steps: no TDD required

---

## Open questions

None blocking. Assumptions stated above.

If you want **prep-only (P0–P2, never live)** even when P2 would justify a shot, say so on Revise.

---

## Who acts next (after Approve)

1. Agent: P0 pin + parallel P1-A / P1-B
2. Agent: P2 join (live yes/no)
3. Live only if P2 yes; else stop and report the wall

### Critical Files for Implementation
- `RESIDUAL.md` — honesty + unlock SSoT
- `src/backends/amd_xdna2/direct.rs` — same-shot + WAIT path
- `src/backends/amd_xdna2/silicon_contract.rs` — match / wait axes honesty
- `testdata/xdna2_cmd/external_known_good/` — load-accepted landing
- `tools/operator_app_load_kit.sh` — justified live entry only
