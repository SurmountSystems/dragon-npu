# Plan 46 explore — what still blocks product silicon Pass

**Date:** 2026-08-12
**Mode:** offline read + code verify. No product edit. No live NPU. No L3 / ALLOW.
**Honesty:** invent_pass=**0** · Pass=**No** · `ran_on_npu` not claimed · unlock **none**

---

## Residual class (one sentence)

Product live still dies as honest **`wait_cmd_einval`**: EXEC_CMD returns a seq, then `DRM_IOCTL_AMDXDNA_WAIT_CMD` (`0xc0106449`) fails immediately with EINVAL (`hwctx=1`, `seq=0`, `timeout_ms=5000`), so there is no ERT Completed and no OFM allclose, even after same-shot in-process paddrs under consumed `2c9c12e8`.

Pass still requires product `decide_ran_on_npu`: real ioctl + submit/wait ok + instruction blob + non-zero golden + OFM allclose. Envelope EXEC alone is never Pass.

---

## Blocker table

| Blocker | Who owns | Addressable now? | How |
|---------|----------|------------------|-----|
| Residual-matched / load-accepted classic **1688** (ERT Completed + OFM allclose via `decide_ran_on_npu`) | **Human** (private stack / FLM / intercept peel) | **Yes** (human only) | Place honest bytes under `testdata/xdna2_cmd/external_known_good/` + PROVENANCE. Public 300/660/2592/4512 and unpatched oracle `3c5ffca750922d3b` do **not** count. |
| Public DD live post-patch peel | **Host / vendor stack** | **No** (frozen public path) | `get_instr_bo` is Transaction/cache; staged family hits **INSTR_BO_LIMIT**; private `run_` not dumpable (P1-A / W8). |
| WAIT_CMD EINVAL (fence/seq vs hwctx) | **Driver / FW** (userspace pack already matches uAPI) | **Not as a pack fix** | Staged driver EINVAL is `xa_load` miss **or** `aie2_cmd_get_out_fence` NULL (“Invalid syncobj or sequence number”). True expiry is `-ETIME`, not EINVAL. |
| Product wait path still WAIT_CMD (XRT prefers syncobj when handle ≠ 0) | **Agent** (research wire exists) | **Offline yes; live no** | Env `XDNA2_LAB_WAIT_VIA_SYNCOBJ` is wired; default off. P1-B / P2: `live_justified_from_wait_theory=no`. Do **not** mint to A/B this alone. |
| Same-shot rela before EXEC | **Already landed** | **Frozen reconfirm** | `XDNA2_LAB_SAME_SHOT_RELA=1` is kit-defaulted, forwarded by `lab_exec_once`, applied in `timed_matmul` **before** EXEC. Live `2c9c12e8`: `same_shot=1` / in-process paddrs. Re-live is empty reconfirm. |
| Post-fail ERT token on WAIT Err | **Already landed (P5)** | Observability only | Next justified live can grep `ert_post_fail=present\|unread`. Historical `2c9c12e8` last_apply has no token (`historical_no_ert_token`). ≠ Pass. |
| CONFIG_CU / spent flexml PDI | **Closed science** | **Frozen** | Attach can succeed; residual still WAIT EINVAL (or older APP_LOAD). Re-live spent fingerprints banned. |
| Active live unlock | **None** | **Do not mint** | SSoT: first `C_UNLOCK=` in `RESIDUAL.md` = **`2c9c12e8` CONSUMED** (`grep -m1`). Active only if that line has no CONSUMED. |
| L3 / `XDNA2_ALLOW_EXEC_CMD` | **Operator / host** | Not this slice | Scripts must not open them. Unlock ≠ Pass. |

**Do not mint a new unlock** without a **new named offline reason** (new residual-matched bytes, or a new last_apply class / measurable wait theory). Same-shot already proven. Syncobj wire already landed offline. Neither is a mint reason.

---

## WAIT EINVAL hypotheses (ranked by evidence, not invention)

Live shape (skip-CU `1f761bf4` and same-shot `2c9c12e8`): status `wait_cmd_einval`, os error 22, `hwctx=1 seq=0 timeout_ms=5000 request=0xc0106449`, wall ≈0.16–0.17 s, aiebu+rela×12, classic 1688, config_cu absent. L5b (`4008e6fd`) same class after Dev DBG_BUF ok (`seq=1`).

| Rank | Hypothesis | Evidence | Status |
|------|------------|----------|--------|
| **1** | Driver cannot resolve **out-fence** for `(hwctx, seq)` after EXEC returned seq | Wave0 staged `aie2_cmd_wait`: NULL fence → `-EINVAL` + “Invalid syncobj or sequence number”. Immediate fail, not hang. Skip-CU journal often silent. | Best driver match. **Not** proven which of fence vs hwctx on silent journal (`os_einval_undifferentiated`). |
| **2** | **FW never published** a waitable fence (job never really scheduled / aborted before fence) | Same EINVAL with and without CONFIG_CU; overlay isolation (`1f761bf4`) did not change class. Historical older path: WAIT **accepted** then ERT Error / APP_LOAD. `2c9c12e8` last_apply had **no** ERT token (pre-P5). | Compatible with (1). Next live can split R1 vs R4 via post-fail ERT. |
| **3** | Product uses **WAIT_CMD** while XRT uses **syncobj timeline** when CREATE returns `syncobj=1` | Live `2c9c12e8` greps `syncobj=1` + `wait_path=wait_cmd`. Handle is retained on `HardwareContext`. WAIT_CMD is still valid uAPI and historically completed. | **Parity gap / hypothesis.** P1-B: not a proven residual root. Live A/B **not** justified. |
| **4** | Invalid / destroyed **hwctx** | Driver `xa_load` miss → EINVAL. Timed path waits **before** destroy. Live always `hwctx=1` after CREATE. | Unlikely on stock path. |
| **5** | `timeout_ms=5000` too short or mis-encoded | Driver timeout is `-ETIME` → product `wait_timeout`. Shot wall 0.16 s. Flexml ~62 s is attach, not the 5 s wait. T3: `timeout_ms=` must not steal class. | **Ruled out** as sole root. |
| **6** | `seq=0` illegal | First job is 0 (`submitted++`). Historical `EXEC_CMD+WAIT seq=0 accepted`. | **Ruled out**. |
| **7** | Wrong ioctl / struct size | Unit + live use `iowr` size 16 → `0xc0106449`. Docs warn against `0x4010_6449` IOW copies. Historical WAIT accepted on same pack. | **Ruled out**. |

**Not named:** “cmd already done” as a distinct proven class. A completed job should still have a fence; EINVAL is lookup fail, not “already signaled.”

---

## Code verify (docs can lie; these are current)

1. **WAIT args:** `amdxdna_drm_wait_cmd { hwctx, timeout: timeout_ms, seq }` via `wait_cmd_ioctl`. Timed path uses `submit_and_wait_command_ex(..., ctx.syncobj_handle)` then still WAIT_CMD unless research env is on.
2. **Default wait:** `choose_lab_wait_path` → `LabWaitPath::WaitCmd` when `XDNA2_LAB_WAIT_VIA_SYNCOBJ` unset/false. Confirmed in `direct.rs`.
3. **Same-shot before EXEC:** `lab_same_shot_rela_env_requested()` → plan from **in-process** IFM/WTS/OFM/CTRL paddrs → `patch_aiebu_elf_ctrltext_with_bos_ex` → write instr BO. Kit/hitl default env to `1`. Live measured `same_shot=1`.
4. **Unlock SSoT:** first `C_UNLOCK=` in `RESIDUAL.md` (`grep -m1`). Today: **`2c9c12e8` CONSUMED**. No active one-shot.
5. **Kernel in-tree:** no `aie2_cmd_get_out_fence` source in this repo. Ground truth is Wave0 notes on staged `amdxdna_ctx.c` / `aie2_ctx.c` plus `docs/XDNA2_CMD_PAYLOAD.md` (struct + `0xc010_6449`).

---

## Short list (stop here)

- invent_pass=**0**. Same-shot measured ≠ Pass. Syncobj research ≠ Pass. Post-fail ERT dump ≠ Pass.
- Primary wall is still **material identity** for lab classic 1688, not another hygiene reconfirm.
- Userspace WAIT pack / ioctl constant / classifier honesty / same-shot / syncobj **retain** are already done. Residual is completion + bytes the FW will actually run.
- Empty same-shot re-live under `2c9c12e8` is **frozen**.
- Do **not** mint unlock from this report.
- Agent next only on a **new named offline delta**. Human next: load-accepted peel or private stack.
- Next justified live (new unlock + new reason) should keep skip-CU vs overlay explicit and must record `wait_path=`, `syncobj=`, and `ert_post_fail=`.
- IRON public AOT **PASS!** on this FW is a working stack proof, **not** residual match and **not** product `ran_on_npu`.
