# Campaign try-everything — W0+W1 offline status pin

**Date:** 2026-08-11
**Kind:** Status pin only (read wave0/wave1 joins + track reports). No code. No live. No unlock mint.
**Honesty:** invent_pass=**0** · Pass=**No** · ran_on_npu=**not claimed** · residual_matched_claim=**0**
**Unlock:** **none active.** First `C_UNLOCK=` in `RESIDUAL.md` = `1f761bf4` **CONSUMED**. Do not reopen. Do not mint from this pin alone.

**Inputs (reports only):**

| Report | Role |
|--------|------|
| `.agents/reports/wave0-join.md` | W0 offline fan-out join |
| `.agents/reports/wave1-join.md` | W1 O2 + material/O5 join |
| `.agents/reports/wave1-o2-wait-syncobj.md` | O2 WAIT axes + syncobj retain |
| `.agents/reports/wave1-debug-bo-wire.md` | Debug-BO research gate + trait wire |
| `.agents/reports/wave1-material-o5.md` | Material + O5 aiebu / fingerprint |

Plan: `.agents/plans/plan-silicon-try-everything.md`. Residual SSoT: `RESIDUAL.md` top.

---

## Plain English

Waves **W0 and W1** finished the **offline** half of the silicon Pass try-everything campaign. Product tools, maps, and research wires advanced. **Still no product Pass.** **Still no active unlock.** Live queue for L1–L4 stays blocked on missing residual-matched material and empty unspent-flexml. L5 (debug-BO live) waits on **lab sandwich path wire** after the offline research trait/gate already landed.

---

## Accomplished offline (this campaign wave)

### Wave W0 (fan-out)

| Track | What landed (not Pass) |
|-------|------------------------|
| **S0** | Campaign pin + plan mirror |
| **M1/M3** | Three public mlir-aie / IRON npu2 AOT sets under `testdata/xdna2_cmd/external_known_good/` (vector 300 B, single_core 660 B, whole_array 2592 B) + siblings; **RESEARCH ONLY** |
| **M4** | FastFlowLM / Lemonade **absent** on host (blocked until operator installs) |
| **M5/M6** | `pass_unblock_once` status/compare + material inventory path ready offline |
| **O1 WAIT** | No fixable one-line WAIT pack root; residual ownership fence/syncobj/seq or undifferentiated EINVAL |
| **O3 map** | Construction paths named (XRT/DD, lab pure-direct, IRON AOT) |
| **O4 structure** | Public peels distant from lab classic 1688; residual_matched=0 |
| **O6 prep** | C6 debug-BO pack/soft/recipe **offline ready** |
| **O8 pack/OFM** | Staged sizes + nonzero golden preflight green (pack ≠ Pass) |
| **O9 PDI** | Unspent flexml STX list **empty** (all three Spent + CVML Spent) |

### Wave W1 (product offline)

| Track | What landed (not Pass) |
|-------|------------------------|
| **O2** | Pure driver axes map (`map_wait_cmd_einval_driver_axes` + soft note invent_pass=0); `CREATE_HWCTX.syncobj_handle` retained on `HardwareContext` + `syncobj=` in last_apply. Live still WAIT_CMD only. **Unlock not justified from O2 alone.** |
| **O5 aiebu** | Wrap/peel closed offline (lab rela×12; public 1/5/20); **cannot invent residual match** |
| **Material** | **No new bins** this wave; cascade AOT not landed; working-stack live EXEC peel still missing |
| **Fingerprint** | Spent ledger drift fixed (all three flexml STX + CVML Spent in fingerprint tool) |
| **Debug-BO wire** | Research **gate + trait** `config_hwctx_dbg_buf` fail-closed offline (`XDNA2_LAB_RESEARCH_DEBUG_BO`); unit contracts refuse when unset. Soft always invent_pass=0. **Lab sandwich auto-wire still open** (CREATE BO + assign in `lab_exec_once` / timed path **not** done). Wire alone ≠ unlock mint. |

### Live history still standing (not this offline wave)

- Skip-CONFIG_CU isolation under **`1f761bf4` CONSUMED**: still `wait_cmd_einval`; overlay not sole residual root.
- Three flexml CONFIG_CU attaches Spent (unified-2x4x4 / llm / unified-4x4); residual class post-attach **`wait_cmd_einval`**.

---

## Live queue (Track L honesty)

| Slot | Status | Why |
|------|--------|-----|
| **L1** residual-matched load | **Blocked** | No residual-matched / load-accepted program bytes (live ERT Completed + OFM allclose via `decide_ran_on_npu`) |
| **L2** second serial live | **Blocked / No** | No justified second live from W0/W1 offline alone |
| **L3** new blob | **Blocked / No** | No new capture size class this wave; public AOT still RESEARCH ONLY |
| **L4** unspent flexml | **Blocked / Empty** | All three flexml STX Spent; no L4 from flexml alone |
| **L5** debug-BO live | **After lab-path wire (in flight)** | Research trait/gate offline **landed**; auto CREATE+assign in lab sandwich **still open**. Any L5 live needs **new** host-minted unlock + named reason after lab path is real — **not** from this pin alone |
| **L6** further queue | **No** | Nothing justified |

**Do not mint** from W0/W1 offline or this status pin alone. Next live only under **new** host-minted unlock + **new** named offline reason.

---

## Honesty lock

| Token | Value |
|-------|--------|
| invent_pass | **0** |
| product Pass | **No** |
| ran_on_npu | **not claimed** |
| residual_matched_claim | **0** |
| Active unlock | **None** (`1f761bf4` CONSUMED + priors) |
| Capture material | Public AOT present (**RESEARCH ONLY**); working-stack EXEC peel **missing** |
| Unspent flexml STX | **Empty** |
| debug-BO research wire | **Offline trait/gate yes**; lab sandwich auto-wire **not** done |
| Campaign | try-everything; **W0+W1 offline status pinned** |

**Frozen still holds:** re-live spent flexml; skip-CU reconfirm under CONSUMED `1f761bf4`; invent Pass; sticky ALLOW; empty reconfirm mill; Pass-from-attach; Pass-from-capture; Pass from O2 axes / aiebu re-wrap / public AOT alone; agent live without new unlock + new named reason; L4 from empty unspent list; L5 live without lab path + new unlock.

---

## Highest remaining (Pass lever)

1. **Working-stack residual-matched / load-accepted program material** — live ERT Completed + non-zero OFM allclose via product `decide_ran_on_npu`. Prefer EXEC peel (or equivalent) into `testdata/xdna2_cmd/external_known_good/` with honest PROVENANCE (completed on this firmware? yes/no). Public IRON AOT alone does **not** close residual.
2. **Optional agent offline:** finish **lab-path wire** for debug-BO (CREATE BO + gated assign in lab sandwich, still invent_pass=0, no live apply) so L5 recipe is non-fiction when a future unlock is justified.
3. **Operator optional:** install FastFlowLM / Lemonade and capture real residual-matched peels if those stacks can complete on this firmware; or place any true working-stack capture under the external known-good dir.

---

## Files

| Path | Role |
|------|------|
| `.agents/reports/campaign-try-everything-w1-status.md` | This status pin |
| `.agents/reports/wave0-join.md` | W0 join |
| `.agents/reports/wave1-join.md` | W1 join |
| `.agents/reports/wave1-o2-wait-syncobj.md` | O2 detail |
| `.agents/reports/wave1-debug-bo-wire.md` | Debug-BO research wire |
| `.agents/reports/wave1-material-o5.md` | Material + O5 |
| `RESIDUAL.md` | Plain status / ranked next (updated to match) |
| `.agents/plans/plan-silicon-try-everything.md` | Campaign plan |
