# Wave4 join — offline complete (W4a/b/c)

**Date:** 2026-08-11
**Kind:** Join pin only (docs/residual). No live. No unlock mint.
**Honesty:** invent_pass=**0** · Pass=**No** · active unlock=**none** · first
`C_UNLOCK=` = **`4008e6fd` CONSUMED**

---

## One-screen status

### What landed

| Wave | Slice | Outcome |
|------|--------|---------|
| **W4a** | Host inventory (T1+T8) | accel=1, amdxdna present, tools/firmware present; `blocked_on_host_stack=0`. Report: `wave4-host-inventory.md` |
| **W4a** | Scorecards (T3+T6) | Pure last_apply rich tokens + material EXEC readiness scorecard; **residual_matched_claim always 0**. Report: `wave4-scorecard-tools.md` |
| **W4a** | Journal scrape (T4) | `tools/journal_npu_scrape.sh` soft always-0; wired into post-C2 + pass_unblock. Report: `wave4-journal-tool.md` |
| **W4a** | Syncobj pack (T5) | Pure timeout/flags/errno pack + fail-closed handle 0; unit green. Report: `wave4-syncobj-pack.md` |
| **W4b** | Discovery | Bag scored (all claim=0); multi-txn 32208 **refused** ingest; no safe capture without operator. Report: `wave4-discovery.md` |
| **W4c** | Syncobj wire | Env-gated `XDNA2_LAB_WAIT_VIA_SYNCOBJ` research path offline; default still WAIT_CMD; unit/clippy green. Report: `wave4-syncobj-wire.md` |

S0 pin/mirror: `.agents/reports/impl-s0-w4-pin-mirror.md` · plan
`.agents/plans/plan-silicon-try-everything.md`.

### Honesty / unlock

| Item | Value |
|------|--------|
| invent_pass | **0** |
| Pass | **No** |
| ran_on_npu | **false** (not claimed) |
| Active unlock | **None** |
| First `C_UNLOCK=` | **`4008e6fd` CONSUMED** (do not reopen) |
| Prior CONSUMED | `48da6e60`, `1f761bf4`, `efd44f35`, `8352882b`, `2ae69036`, `97f38307`, `11bc9016`, `3a70fabf`, `b17b0119` |

Offline tools ≠ Pass ≠ residual match ≠ unlock.

### Material wall

- **Operator load-accepted** capture still **open**. Need true residual-matched /
  load-accepted peel under `testdata/xdna2_cmd/external_known_good/` (or path +
  `bash tools/pass_unblock_once.sh --ingest PATH`).
- Public mlir-aie npu2 AOT (300 / 660 / 2592 / cascade 4512) scored
  **residual_matched_claim=0** — research only; **do not count** as load-accepted.
- Host candidate `gt_capture/instr_vendor_post_patch.bin` (32208 multi-txn):
  **refused** (`multi_txn_refused=1`). Historical wrong-body; not residual match.

### L8 (syncobj wait live research)

| Criterion | Status |
|-----------|--------|
| Offline pack (W4a T5) green | **Yes** |
| Offline wire (W4c) green | **Yes** |
| Named offline reason **beyond** “wire landed alone” | **No** (still missing) |
| New host-minted `C_UNLOCK=` | **No** (join does **not** mint) |
| **Live L8 justified now?** | **No** |

**Why not:** Syncobj pack+wire prove userspace can select
`DRM_IOCTL_SYNCOBJ_TIMELINE_WAIT` under research env. They do **not** prove that
path is the residual root for live `wait_cmd_einval` (seen under CONSUMED
skip-CU `1f761bf4` and L5b Dev DBG `4008e6fd`). Wire alone ≠ unlock ≠ Pass.

**If later justified** (needs new offline delta + new unlock mint, not this join):

- Recipe shape (design only): stock **skip-CONFIG_CU** isolation +
  `XDNA2_LAB_WAIT_VIA_SYNCOBJ=1`
  (and CREATE must return non-zero `syncobj_handle`; fail-closed if handle 0).
- Record: `wait_path=` on last_apply/detail + journal scrape + no sticky ALLOW.
- Pass still only via live ERT Completed + OFM allclose (`decide_ran_on_npu`).

**L8 not run.** Offline ready for L8 **only after** a named reason + new unlock.

### Freeze list (reminder)

- No re-live under **CONSUMED** unlocks (`4008e6fd` + priors)
- No re-live **spent** flexml fingerprints alone
- No invent Pass / invent residual match / invent load-accepted
- No sticky `XDNA2_ALLOW_EXEC_CMD=1`
- No empty reconfirm mill as progress
- No multi-txn / public AOT as product known-good

### Who acts next (primary)

**Operator material.** Highest Pass gap remains load-accepted /
residual-matched program bytes. Agent W4 offline is complete; L8 is **not**
the primary next action until a named offline reason justifies a **new** unlock.

Secondary (agent, offline only, no unlock): idle on material wall, or only a
**named** offline code delta the operator asks for. Do not mint L8 from this join.

---

## Report index

| Report | Role |
|--------|------|
| `impl-s0-w4-pin-mirror.md` | S0 campaign pin + plan mirror |
| `wave4-host-inventory.md` | T1+T8 host stack |
| `wave4-scorecard-tools.md` | T3+T6 pure scorecards |
| `wave4-journal-tool.md` | T4 journal scrape |
| `wave4-syncobj-pack.md` | T5 pack pure |
| `wave4-discovery.md` | W4b bag + refuse multi-txn |
| `wave4-syncobj-wire.md` | W4c env-gated wire offline |
| `wave4-join.md` | this join |

## Explicit non-claims

- No product Pass · invent_pass=0 · no `ran_on_npu`
- No L8 unlock mint · no live SYNCOBJ_TIMELINE_WAIT proven
- No claim syncobj wait clears `wait_cmd_einval`
- No load-accepted material invented
