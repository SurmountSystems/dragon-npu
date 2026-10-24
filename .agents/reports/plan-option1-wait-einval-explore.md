# Explore: product EXEC residual `wait_cmd_einval` after same-shot live

**Date:** 2026-08-11
**Mode:** read-only explore. No product edits. No live. No unlock mint.
**Honesty:** invent_pass=0 · Pass=**No** · unlock first `C_UNLOCK=` = **`2c9c12e8` CONSUMED**
**Focus:** WAIT_CMD EINVAL vs timeout vs APP_LOAD; offline work that makes next product live more informative (without inventing Pass).

**Primary live pin:** Arm A same-shot product matmul under `2c9c12e8`
→ last_apply `/tmp/dragon_npu_last_apply_live_eff_2c9c12e8.json`
→ status=`wait_cmd_einval`; same_shot=1; paddrs_valid=1; config_cu absent; total_s≈0.165.

---

## 1. What `wait_cmd_einval` means (honest)

| Claim | Truth |
|-------|--------|
| Product status | Machine status from `classify_submit_wait_error` when WAIT/submit path returns **EINVAL** (`Invalid argument` / `os error 22` / `einval`), and the message is **not** the payload/txn `InvalidArgument` class. |
| Ioctl behavior | **`DRM_IOCTL_AMDXDNA_WAIT_CMD` refused** (or returned EINVAL) almost immediately. Live shape: `submit_wait_s≈0`, wall ~0.16–0.17s on skip-CU. |
| Not a timed-out wait | True budget expiry in AIE2 is **`-ETIME`** → product **`wait_timeout`**. Field name `timeout_ms=5000` is always embedded in WAIT error strings; T3 fixed false `wait_timeout` from that substring alone. |
| Not “FW hang waited full budget” | Skip-CU / same-shot shots do **not** sit for 5s then fail; EINVAL is immediate refuse of the wait completion path. |
| Not OFM mismatch after clean wait | Wait never completed successfully; no ERT Completed path, no OFM allclose, `ran_on_npu=false`. |
| Driver axes (staged) | EINVAL from: (a) bad/missing hwctx (`xa_load` miss), or (b) **`aie2_cmd_get_out_fence` NULL** (“Invalid syncobj or sequence number”). Userspace often cannot split (a) vs (b) without journal. |
| vs APP_LOAD | Historical APP_LOAD = WAIT **accepted**, then ERT Error + journal **`0x3000003`**. Soft residual on recent skip-CU/same-shot last_apply is often `not_pass_no_ert_error` (no `ert_error` token). Different residual family. |

**One line:** `wait_cmd_einval` = **wait completion ioctl refused (EINVAL)**, not “job ran and timed out,” not “job completed with wrong OFM,” not APP_LOAD after successful wait.

**Code SSoT:**

- `src/backends/amd_xdna2/direct.rs` — `classify_submit_wait_error`, `submit_wait_message_is_einval` / `_true_timeout` / `_payload_class`; WAIT pack `0xc010_6449`.
- `src/backends/amd_xdna2/silicon_contract.rs` — `ResidualClassifierSplit::WaitCmdEinval` (never folds to timeout); `WaitCmdEinvalAxes` / `WaitCmdDriverAxis` / driver map notes.

**Docs:** `docs/XDNA2_EXEC_CMD_LAB.md` — finite WAIT default 5000 ms → **true hang** → `wait_timeout`; never default infinite. That is orthogonal to EINVAL refuse.

---

## 2. What same-shot already ruled out

Live `2c9c12e8` (Arm A) + prior skip-CU `1f761bf4` isolation:

| Ruled out / narrowed | Evidence |
|----------------------|----------|
| **Stale disk UVAs as sole residual** | same_shot=1, paddrs_valid=1, source=`in_process_bo_map`, refuse_stale_disk=1. Product patched rela from **this process’s** BO map. Class still `wait_cmd_einval` (same as pre–same-shot skip-CU). |
| **CONFIG_CU overlay as sole root** | Skip-CU still `wait_cmd_einval` (~0.17s, silent journal). Flexml CONFIG_CU can **add** ~62s attach + optional IO_PAGE_FAULT / cmd timeout journal; class shared. |
| **Wrong WAIT ioctl id / struct size as sole root** | Product uses `0xc010_6449` size-16; historical path accepted WAIT; unit guards against wrong `0x4010_6449`. |
| **timeout_ms=5000 “too short” as root of EINVAL** | EINVAL ≠ ETIME; immediate fail; bumping wait budget alone would not reclass or Pass. |
| **seq=0 illegal alone** | Driver first job seq starts at 0; historical EXEC+WAIT seq=0 accepted (later APP_LOAD era). |
| **Classifier honesty bug** | T3 landed: live-shaped EINVAL → `wait_cmd_einval`, not false `wait_timeout`. Honesty only; invent_pass=0. |

**Same-shot finding (plain English):** same-run address patching **works live** and is necessary hygiene. It is **not sufficient** for Pass. Residual class did **not** move.

---

## 3. Open root-cause hypotheses (ranked)

Higher = more residual-useful / still open for next informative live.

| Rank | Hypothesis | Status after same-shot |
|------|------------|------------------------|
| **R1** | **Job / fence never published for WAIT** (`aie2_cmd_get_out_fence` NULL → EINVAL). EXEC may return seq, but wait path has no out-fence. Driver/FW/job lifecycle residual. | **Open.** Best matches immediate EINVAL + silent skip-CU journal. |
| **R2** | **Instruction / load material not residual-matched** to a completing stack (lab classic 1688 unpatched/post-patch identity vs working CU/PDI path). FW never schedules a completable job → fence miss. | **Open; highest product Pass gap.** Public AOT research peels ≠ residual identity. |
| **R3** | **Product always WAIT_CMD; drops CREATE_HWCTX `syncobj_handle`** (XRT may prefer drm_syncobj wait when valid). Possible **parity gap**, not proven sole root of EINVAL; WAIT_CMD is still valid uAPI. | **Open as observability / optional research wire.** Offline pack already exists; live wire under new unlock only if justified. |
| **R4** | **Cmd BO / START_NPU / aiebu content accepted by EXEC but rejected later by FW** without ERT surface on this path. | **Open; hard to split from R1 offline.** Historical APP_LOAD had ERT Error after wait ok. |
| **R5** | **hwctx missing at wait** (destroyed / wrong handle). | **Unlikely** on stock timed path (wait before destroy; live hwctx=1). Keep as low-priority check token only. |
| **R6** | Overlay-only MMU (`IO_PAGE_FAULT` bands on flexml). | **Ruled out as sole root** by skip-CU + same-shot; still additive noise on CONFIG_CU lives. |
| **R7** | Stale paddrs / disk UVAs. | **Ruled out for product same-shot path** by `2c9c12e8`. |

W0 O1 dig verdict (still holds): **no fixable userspace WAIT packing bug** as sole root of live EINVAL. Residual ownership for Pass remains **job completion + load material + decide**, not a one-line pack fix.

---

## 4. Offline-only work that makes next live more informative

Do **not** mint unlock from this list alone. Prefer evidence that **changes what next last_apply / journal can prove**.

| Priority | Offline work | Why it helps next live |
|----------|--------------|------------------------|
| **P0** | **Residual-matched / load-accepted material hunt** (true post-patch classic 1688 or completing-stack peel into capture dir + classify). Refuse invent residual match from public AOT alone. | Next live can test a **different body**, not re-prove same skip-CU shape. |
| **P1** | **Journal correlation checklist** for next shot only: require scrape window **tight to shot wall** (same-shot ~0.16s); tokens for “Invalid syncobj or sequence number” vs hwctx miss vs silence. Soft `map_wait_cmd_einval_driver_axes` already exists. | Splits R1 fence miss vs silent FW vs wrong window pollution (flexml history). |
| **P2** | **last_apply completeness for wait path:** ensure detail always greps `wait_cmd_einval` axes (hwctx/seq/timeout_ms/request), same_shot tokens, bo_diag, `config_cu_present=0|1`, ERT if any. Soft classify already prints axes. | Reduces “unknown last_apply” on next unlock. |
| **P3** | **Syncobj observability only:** confirm `syncobj_handle` appears in create detail / context (if not already greppable on stock path); pure pack tests for research wait path already offline-green. **Do not** treat wire-on as Pass. | Makes optional XRT-parity live (R3) measurable under **new** unlock + named reason. |
| **P4** | **A/B offline of instruction identity** vs IRON peels (size class, structural distance, classic walk) — already partly landed; use to **refuse** residual_matched claim when wrong. | Stops worthless lives on research-only peels. |
| **P5** | **ERT / Cmd BO post-fail read path** offline design: if WAIT EINVAL, can product still dump Cmd BO ERT state safely? (If not on stock driver, document “absent” as expected.) | Historical APP_LOAD had ert_state; current skip path often lacks it. |

**Already landed offline (do not re-spin empty):** T3 classify honesty; WaitCmdEinvalAxes dig; driver axes map; skip-CU isolation; same-shot wire + live proof of in-process paddrs; syncobj pure wait helpers (not default product wait).

---

## 5. What must NOT be done

| Forbidden | Why |
|-----------|-----|
| **Second live under `2c9c12e8`** | Unlock **CONSUMED**. One-shot only. |
| **Empty same-shot reconfirm** (same Arm A, no new material/reason) | Same residual already measured; wastes unlock economics. |
| **Invent Pass / invent `ran_on_npu` / invent residual match** | invent_pass=0 always until live decide_ran_on_npu with ERT Completed + non-zero OFM allclose. |
| **Re-live spent flexml fingerprints** alone | Spent under `2ae69036` / `8352882b` / `efd44f35`; attach ≠ Pass. |
| **Reopen any CONSUMED unlock** | List includes `2c9c12e8`, `4008e6fd`, `1f761bf4`, priors. |
| **Sticky ALLOW / leave L3 open** | Lab restore after shot. |
| **Bump `XDNA2_WAIT_TIMEOUT_MS` alone as “the fix”** | Wrong residual class lever for EINVAL. |
| **Treat soft classify / axes / same_shot=1 / offline gates as C-ready** | Soft always not_c_ready / not_ran_on_npu. |
| **Treat public mlir-aie AOT peels as residual identity** | Research only; residual_matched_claim=0. |

**Next live only if:** **new** host-minted unlock **and** **new** named offline reason (typically residual-matched material or justified research path with measurable journal/ERT delta). Same-shot remains hygiene when OFM path is on; it is not itself the unlock reason again.

---

## 6. Class comparison cheat sheet

| Class | OS / product signal | Typical wall | Means |
|-------|---------------------|--------------|--------|
| **`wait_cmd_einval`** | WAIT EINVAL / os error 22 | ~0.16s skip-CU | Wait **refused**; no successful completion wait |
| **`wait_timeout`** | etimedout / timed out / ETIME (not mere `timeout_ms=`) | ≈ wait budget | Wait **ran** until budget; hang-class |
| **APP_LOAD** | Wait often OK; ERT Error + `0x3000003` | varies | FW load reject after userspace ELF+rela green (historical C2) |
| **Pass** | `ok_npu_verified` + decide | — | real_ioctl + ERT Completed + OFM allclose only |

---

## 7. Key paths

| Path | Role |
|------|------|
| `src/backends/amd_xdna2/direct.rs` | WAIT ioctl, classify_submit_wait_error, timed path |
| `src/backends/amd_xdna2/silicon_contract.rs` | residual split, WaitCmdEinvalAxes, driver axes, same-shot plan |
| `docs/XDNA2_EXEC_CMD_LAB.md` | finite WAIT lab policy |
| `.agents/reports/live-shot-2c9c12e8.md` | same-shot live evidence |
| `.agents/reports/live-shot-1f761bf4.md` | skip-CU isolation |
| `.agents/reports/impl-t3-wait-cmd-einval-classify.md` | honesty fix |
| `.agents/reports/wave0-wait-einval-uapi.md` | driver/uAPI dig + ranked H1–H8 |
| `.agents/reports/impl-offline-wait-einval-material-49feb5d2.md` | axes TDD |
| `.agents/reports/wave10-b-same-shot.md` | same-shot offline wire |

---

## 8. Bottom line (bullets)

- **`wait_cmd_einval`** = WAIT_CMD **EINVAL refuse**, not hang-timeout, not APP_LOAD, not Pass.
- **Same-shot ruled out stale paddrs** as the residual; class **unchanged** vs skip-CU.
- **Still open:** fence/job publish (R1), residual-matched load material (R2), optional syncobj parity as gap not proven root (R3).
- **Next informative offline:** material that can complete on device + tight journal/ERT observability; not empty reconfirm.
- **Must not:** second live under `2c9c12e8`, invent Pass, re-live spent flexml, timeout-bump-as-fix.

**Report path:** `/home/hunter/Projects/ai/dragon-npu/.agents/reports/plan-option1-wait-einval-explore.md`
