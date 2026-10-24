# Wave W1 join — O2 + O5/material closeout

**Date:** 2026-08-11
**Kind:** Join pin only (read wave1 reports; no re-research). No live. No unlock mint.
**Honesty:** invent_pass=**0** · Pass=**No** · ran_on_npu=**not claimed** · residual_matched_claim=**0**
**Unlock:** **none active.** First `C_UNLOCK=` in `RESIDUAL.md` = `1f761bf4` **CONSUMED**. Do not reopen. Do not mint from this join alone.

**Inputs (reports only):**

| Report | Track |
|--------|--------|
| `.agents/reports/wave0-join.md` | W0 join (parent) |
| `.agents/reports/wave1-o2-wait-syncobj.md` | O2 WAIT driver axes + syncobj retain |
| `.agents/reports/wave1-o5-aiebu.md` | O5 aiebu offline wrap/peel |
| `.agents/reports/wave1-material-o5.md` | Material + fingerprint drift + O5 join |
| `.agents/reports/wave0-prep-inventory.md` | O6 debug-BO offline prep (not live-wired) |

---

## Plain English summary

Wave W1 finished the offline O2 product slice and the material/O5 closeout.
Both tracks stayed invent_pass=0 and Pass=No. No live shot. No unlock mint.

**What landed (offline facts, not Pass):**

1. **O2 — WAIT EINVAL driver axes + syncobj retain.** Pure map
   (`map_wait_cmd_einval_driver_axes` / soft note) separates hwctx miss vs
   fence·syncobj·seq vs ETIME vs undifferentiated. Soft always invent_pass=0;
   axes alone ≠ Pass. Product now stores `CREATE_HWCTX.syncobj_handle` on
   `HardwareContext` and prints `syncobj=` in last_apply detail. Live still
   uses WAIT_CMD only (no syncobj wait path wired). **Live unlock is not
   justified from O2 alone** (no new material; no proven packing root).
2. **O5 / material — no new bins.** Capture still three public IRON AOT sets
   (300 / 660 / 2592); no new size class this wave. Working-stack live EXEC
   peel still missing. aiebu wrap/peel for lab classic (rela×12) and public
   (rela 1/5/20) is already understood; aiebu **cannot invent residual match**
   or vendor post-patch. Fingerprint ledger drift fixed: all three flexml STX
   + CVML baseline marked **Spent** in `pass_unblock_fingerprint_once.sh`
   (aligned with RESIDUAL SSoT).
3. **debug-BO (from W0 prep):** offline pack/soft/recipe ready; **not**
   live-wired. Prep alone ≠ Pass ≠ unlock reason.

**What is still blocked for Pass:**

| Blocker | Detail |
|---------|--------|
| **Load-accepted / residual-matched program** | No live ERT Completed + non-zero OFM allclose via `decide_ran_on_npu` |
| **Working-stack EXEC peel** | Still missing under `testdata/xdna2_cmd/external_known_good/` |
| **Active unlock** | None. `1f761bf4` + priors CONSUMED |
| **Live debug-BO** | Needs **live wire** first (offline prep only today) |

---

## Per-track join table

| Track | Outcome | invent_pass | Unlock from this alone? |
|-------|---------|-------------|-------------------------|
| **O2** | Driver axes soft map + `syncobj_handle` retain on context / last_apply | 0 | **No** |
| **O5 aiebu** | Wrap/peel closed offline; cannot invent residual match | 0 | **No** |
| **Material** | No new bins; cascade AOT not landed; FastFlowLM/Lemonade absent | 0 | **No** |
| **Fingerprint** | Spent drift fixed (llm / u4 / unified-2x4x4 all Spent) | 0 | **No** |
| **debug-BO** | Offline ready; live_wired=0 (W0 prep still holds) | 0 | **No** |

---

## Live queue status (Track L honesty)

| Slot | Status | Why |
|------|--------|-----|
| **L1** residual-matched load | **Blocked** | No residual-matched / load-accepted program bytes |
| **L2** | **No** | No justified second serial live from W1 offline alone |
| **L3** new blob | **No** | No new capture bins / new size class this wave |
| **L4** unspent flexml | **Empty** | All three flexml STX Spent; no L4 from flexml alone |
| **L5** debug-BO live | **Needs live wire first** | Offline prep only today; wire research path (TDD fail-closed) before any L5 unlock consideration |
| **L6** | **No** | No further queue item justified |

**Do not mint** from W1 join alone. Next live only under **new** host-minted
unlock + **new** named offline reason.

---

## Residual class (unchanged honesty)

- Userspace green: aiebu+rela×12, real_ioctl, CREATE/EXEC plumbing.
- Live class: post-attach / EXEC **`wait_cmd_einval`** (with or without CONFIG_CU).
- invent_pass=0. No `ran_on_npu`. Pass=No.

---

## Next (ranked)

1. **Optional:** wire debug-BO research path offline (TDD fail-closed; still
   invent_pass=0; not live apply). Only after a real wire + named reason
   consider L5 unlock — **not** from this join alone.
2. **Parallel:** keep hunting working-stack material (EXEC peel, cascade AOT
   only if new sha/size lands, FastFlowLM/Lemonade if operator installs).
3. **Frozen still holds:** re-live spent flexml; skip-CU reconfirm under
   CONSUMED `1f761bf4`; invent Pass; sticky ALLOW; empty reconfirm; Pass from
   O2 axes alone; Pass from aiebu re-wrap; L4 from empty unspent-flexml list.

---

## Honesty lock

| Token | Value |
|-------|--------|
| invent_pass | **0** |
| product Pass | **No** |
| ran_on_npu | **not claimed** |
| residual_matched_claim | **0** |
| Active unlock | **None** (`1f761bf4` CONSUMED + priors) |
| O2 unlock justified | **No** |
| New bins this wave | **None** |
| Unspent flexml STX | **Empty** |
| debug-BO live | **Not** wired |
| Campaign | try-everything; **W1 join done** |

---

## Files

| Path | Role |
|------|------|
| `.agents/reports/wave1-join.md` | This join |
| `.agents/reports/wave1-o2-wait-syncobj.md` | O2 detail |
| `.agents/reports/wave1-o5-aiebu.md` | O5 detail |
| `.agents/reports/wave1-material-o5.md` | Material + fingerprint |
| `.agents/reports/wave0-prep-inventory.md` | debug-BO offline prep |
| `RESIDUAL.md` | Ranked next + plain status pinned for W1 join |
