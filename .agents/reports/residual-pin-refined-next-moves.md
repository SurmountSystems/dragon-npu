# Residual pin: refined next-moves (operator 2026-08-10)

**Kind:** residual honesty pin only (docs). No product code. No unlock mint.
No live apply. No invent capture bytes. No Pass / `ran_on_npu` claim.

## What was pinned

Operator confirmed refined residual framing after flexml live `2ae69036`.
Surgical updates only; history retained below new rows.

### Useful change from flexml `2ae69036`

| Fact | Detail |
|------|--------|
| CONFIG_CU attach | Succeeded for unified-2x4x4, 27200 B, sha16 `0eeeb70c84df056e` |
| Product status | `wait_cmd_einval` (WAIT_CMD EINVAL) |
| Journal | `aie2_error_backtrack` + `aie2_set_cmd_timeout` (DPU/TXN `0xffffffff`) |
| Not seen in window | `IO_PAGE_FAULT` (unlike CVML `97f38307` / `11bc9016`) |
| Class meaning | **Different residual class** from CVML orphan-fault path: post-attach wait EINVAL + cmd timeout/backtrack, not pure APP_LOAD. **Better information**, not Pass |
| Re-live | Pure re-live of spent fingerprint **banned**; new live needs named offline reason tied to new material or deliberately different unspent image |

### Residual confirmed (unchanged honesty)

- Userspace + CONFIG_CU attach largely green; firmware still no ERT Completed + OFM
- Capture under `testdata/xdna2_cmd/external_known_good/` may still be bins=0
- No active unlock; all listed `C_UNLOCK` CONSUMED (`2ae69036` + priors)
- Three needs unchanged: working-stack ctrltext, true matmul-matched PDI beyond spent flexml, driver/MMU parked without new journal
- invent_pass=0; no `ran_on_npu`; attach ≠ Pass

### Ranked next (operator refined)

1. **Highest leverage (human):** real material →
   `testdata/xdna2_cmd/external_known_good/` (IRON/mlir-aie, FastFlowLM/Lemonade
   intercept) with PROVENANCE.
2. Prefer **minimal load** for next live. Unspent candidates (research only):
   flexml llm-2x4x4 sha16 `ba0cd738fd870011`; unified-4x4 `7d172bf223a9b06b`.
3. Next live only under a **new** unlock. Reason shape: unspent flexml topology
   and/or new material + post-attach `wait_cmd_einval` residual from `2ae69036`
   + optional debug BO; full last_apply + journal; re-interpret with p5/C*.
4. **Keep frozen:** empty reconfirm; empty soft-compare; reopen CONSUMED;
   MMU without new journal; Pass from CONFIG_CU attach alone.

### Agent-legal only

| Allowed | Forbidden from this pin |
|---------|-------------------------|
| Ingest/classify new human bytes | Mint `C_UNLOCK` |
| Prepare unlock justification + Step 2 checklist vs residual class | Live apply / L3 / ALLOW |
| Strengthen residual splitters only if next last_apply is a new failure mode | Invent capture bins |
| Refuse same status scripts / re-fingerprint spent default | Claim Pass or `ran_on_npu` |
| | Empty reconfirm mill; re-live spent `0eeeb70c…` alone |

## Files touched

| Path | Change |
|------|--------|
| `RESIDUAL.md` | Plain status table + creative avenues: refined residual class, ranked next, agent-legal bounds |
| `.agents/plans/plan-pass-unblock-sequence.md` | Status, progress board, locked honesty, remaining work A–E, phases P0–P2 Done / material+live remaining |
| `AGENTS.md` | New Latest Turn residual refine pin (prior P2 turn kept) |
| This report | `.agents/reports/residual-pin-refined-next-moves.md` |

## Honesty footer

- Product Pass: **No**
- invent_pass: **0**
- ran_on_npu: **No**
- Active unlock: **None**
- Do not reopen: `2ae69036`, `97f38307`, `11bc9016`, `3a70fabf`, `b17b0119`
- Offline sequence P0–P2: **done** (no empty re-spin)
- Next human action: material Step 1
- Next live: only after host-minted new unlock with named reason as above
