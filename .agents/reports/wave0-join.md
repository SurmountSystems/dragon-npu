# Wave W0 join (O12) — try-everything campaign

**Date:** 2026-08-11
**Kind:** Join pin only (read wave0 reports; no re-research). No code. No live. No unlock mint.
**Honesty:** invent_pass=**0** · Pass=**No** · ran_on_npu=**not claimed** · residual_matched_claim=**0**
**Unlock:** **none active.** First `C_UNLOCK=` in `RESIDUAL.md` = `1f761bf4` **CONSUMED**. Do not reopen. Do not mint from this join alone.

**Inputs (reports only):**

| Report | Track |
|--------|--------|
| `.agents/reports/impl-s0-campaign-pin-try-everything.md` | S0 campaign pin |
| `.agents/reports/wave0-material-avenues.md` | M1/M3/M4 material avenues |
| `.agents/reports/wave0-material-tools.md` | M5/M6 tools + taxonomy |
| `.agents/reports/wave0-wait-einval-uapi.md` | O1 WAIT EINVAL / driver uAPI |
| `.agents/reports/wave0-xrt-dd-load-map.md` | O3 XRT/DD construction paths |
| `.agents/reports/wave0-structural-diff.md` | O4 structural / byte axes |
| `.agents/reports/wave0-prep-inventory.md` | O6/O8/O9 debug-BO, pack, unspent PDI |

---

## Plain English summary

Wave W0 finished the offline fan-out for the silicon Pass try-everything campaign.
Every track stayed invent_pass=0 and Pass=No. No live shot. No unlock mint.

**What landed (offline facts, not Pass):**

1. **Capture material is present and classifiable.** Under
   `testdata/xdna2_cmd/external_known_good/` there are **three** public mlir-aie /
   IRON npu2 AOT sets (vector 300 B, single_core 660 B, whole_array 2592 B), plus
   siblings (insts, elf, pdi, xclbin). Bag bin count expected **7**. Empty capture
   folder is **not** the residual anymore.
2. **Public AOT is RESEARCH ONLY.** Size-class `public-npu2-small` for 300/660;
   whole_array 2592 is length class **`other`**. Soft mismatch vs lab classic
   **1688** and spent flexml PDIs is **expected**. Distance ≠ residual match ≠ Pass.
3. **WAIT packing is not a one-line product root.** Live residual stays
   `wait_cmd_einval` with and without CONFIG_CU. Driver path points at out-fence /
   syncobj or sequence resolution (`Invalid syncobj or sequence number`), not wrong
   ioctl size / seq=0 / short timeout alone. Optional gap: product drops CREATE
   `syncobj_handle` and always uses WAIT_CMD (XRT prefers syncobj wait when valid).
   That is **parity research**, not proven sole root, not Pass.
4. **Construction map deepened.** Three paths named (XRT/DD production, lab
   pure-direct, IRON AOT generator). Skip-CU isolation still stands: overlay is
   **not** sole residual root.
5. **Prep inventory:** C6 debug-BO pack/soft/recipe **ready offline**, not live-wired.
   Staged pack/OFM sizes and nonzero golden gates are green for preflight; pack green
   **≠** Pass. **Unspent flexml STX list is empty** (all three Spent + CVML 4x4 Spent).
6. **Tools path:** `pass_unblock_once --status` / soft compare / material inventory
   APIs are ready offline. Taxonomy needs no product enum for whole_array (honest
   `Other`). FastFlowLM and Lemonade **not installed** on this host.

**What is still blocked for Pass:**

| Blocker | Detail |
|---------|--------|
| **Load-accepted / residual-matched program** | No live ERT Completed + non-zero OFM allclose via `decide_ran_on_npu`. Public AOT peels are host compile-only, not that claim. |
| **Working-stack EXEC peel** | Post-patch ctrltext from a stack that **completed** on **this** firmware is **missing**. Highest material gap. |
| **FastFlowLM / Lemonade intercept** | Trees absent under `~/Projects/ai`; blocked until operator installs and captures real files. |
| **Unspent flexml for L4** | **Empty.** No L4 from flexml alone. |
| **Active unlock** | None. All listed `C_UNLOCK` CONSUMED. |
| **Live debug-BO** | Offline prep only; not wired; not a free unlock reason. |

**Frozen still holds:** re-live spent flexml; skip-CU reconfirm under CONSUMED
`1f761bf4`; invent Pass; sticky ALLOW; empty reconfirm mill; Pass-from-attach;
Pass-from-capture; agent live without **new** unlock + **new** named offline reason.

---

## Per-track join table

| Track | Outcome | invent_pass | Pass lever? |
|-------|---------|-------------|-------------|
| **S0** | Campaign pin + plan mirror; W0 fan-out started | 0 | No |
| **M1/M3** | Three public AOT sets present; working-stack live EXEC capture **missing** | 0 | No (research only) |
| **M4** | FastFlowLM / Lemonade **absent**; Ryzen AI EP libs present but no residual-matched peel | 0 | No |
| **M5/M6** | Status/compare path ready; bins≥1; soft mismatch expected; no taxonomy code change | 0 | No |
| **O1 WAIT** | No fixable WAIT pack root; fence/syncobj residual ownership; O2 optional | 0 | No |
| **O3 map** | Three construction paths + BO pins in construction map | 0 | No |
| **O4 structure** | All public peels size/rela/header distant from lab 1688; residual_matched=0 | 0 | No |
| **O6 debug-BO** | Offline pack/validate/soft **ready**; live_wired=0 | 0 | No |
| **O8 pack/OFM** | Staged sizes + nonzero golden preflight green | 0 | Pack ≠ Pass |
| **O9 unspent PDI** | Flexml STX unspent list **empty** | 0 | No L4 from flexml alone |

---

## Residual class (unchanged honesty)

- Userspace green: aiebu+rela×12, real_ioctl, CREATE/EXEC plumbing.
- Live class: post-attach / EXEC **`wait_cmd_einval`** (with or without CONFIG_CU).
- Historical APP_LOAD `0x3000003` is older residual; current measured class is WAIT EINVAL.
- invent_pass=0. No `ran_on_npu`.

---

## Wave W1 priorities (from W0 evidence only)

Ordered for parent / next wave. **No mint from this pin alone.**

### P0 — Material that can become load-accepted

1. **Working-stack live EXEC peel** (or equivalent residual-matched program bytes)
   into `testdata/xdna2_cmd/external_known_good/` with honest PROVENANCE
   (completed on this firmware? yes/no). Public IRON AOT alone does not close residual.
2. **FastFlowLM / Lemonade** only if operator installs and can run a loadable path
   on this firmware class, then `--ingest` real files. Agent does not invent install.

### P1 — Optional product offline (O2), not sole root

3. **Driver EINVAL / ETIME map** in pure tests or soft notes (hwctx miss vs fence miss
   vs ETIME) so docs cite code paths. Always invent_pass=0.
4. **Record `syncobj_handle`** on `HardwareContext` + last_apply detail (observability).
5. **XRT-parity syncobj wait** research / arg-pack unit tests only until a **new**
   unlock justifies live trial. Implementing syncobj wait alone ≠ Pass.

### P2 — Ready but gated

6. **debug-BO** offline prep is ready for a later live slice only under **new** unlock
   + **named** reason (not W0 join alone).
7. **No L4 from flexml alone** (unspent flexml empty). Optional never-live research
   blobs (e.g. CVML 8x4x1) still need new unlock + new named reason if ever considered.
8. **whole_array 2592** stays research-only **Other** class; optional cascade /
   matrix_vector AOT is optional research only, not required for honesty.

### Frozen (do not do as W1 “progress”)

- Re-live spent flexml sha16s / skip-CU under `1f761bf4`
- Empty reconfirm / status-only mill
- Unlock mint from dual/triple public AOT alone
- Invent residual_matched offline
- Bump WAIT timeout alone as the fix

---

## Honesty lock

| Token | Value |
|-------|--------|
| invent_pass | **0** |
| product Pass | **No** |
| ran_on_npu | **not claimed** |
| residual_matched_claim | **0** |
| Active unlock | **None** (`1f761bf4` CONSUMED + priors) |
| Capture bins (expected) | **7** (research; soft mismatch expected) |
| Unspent flexml STX | **Empty** |
| debug-BO live | **Not** wired |
| Campaign | try-everything; **W0 join done**; **W1 next** |

---

## Files

| Path | Role |
|------|------|
| `.agents/reports/wave0-join.md` | This join |
| `RESIDUAL.md` | Ranked next + plain status updated for W0 join / W1 |
| Plan | `.agents/plans/plan-silicon-try-everything.md` |

**AGENTS.md:** left unchanged (unlock parse rule already correct; prefer RESIDUAL SSoT).
