# Wave W4c — Env-gated syncobj wait wire (offline product delta)

**Date:** 2026-08-11  
**Mode:** offline only. No live NPU. No ALLOW. No L8 unlock mint.  
**Honesty:** `invent_pass=0` · `not_live_proven=1` · **L8 not minted** · Pass = **No**

**Upstream:** `.agents/reports/wave4-syncobj-pack.md` (W4a T5 pure pack)

---

## Plain English

1. Product default wait path is still **WAIT_CMD** (`DRM_IOCTL_AMDXDNA_WAIT_CMD`) when
   `XDNA2_LAB_WAIT_VIA_SYNCOBJ` is unset or false.
2. Research path (this slice): if env is `1`/`true`/`yes` **and** CREATE_HWCTX
   `syncobj_handle != 0`, `wait_command_with_syncobj` /
   `submit_and_wait_command_ex` may issue `DRM_IOCTL_SYNCOBJ_TIMELINE_WAIT`
   using the W4a pack helpers (absolute mono timeout, WAIT_FOR_SUBMIT flags).
3. Fail-closed: research env **on** + handle **0** → clear
   `InvalidArgument` (no silent fall-back to WAIT_CMD while the research flag is on).
4. Soft tokens: `wait_path=wait_cmd` or `wait_path=syncobj` on errors / timed detail.
5. **L8 not minted.** Env alone ≠ unlock ≠ Pass ≠ `ran_on_npu`.

---

## Code landed

| Piece | Role |
|-------|------|
| `drm_syncobj_timeline_wait` (48 B) | uAPI struct for timeline wait |
| `ioctl::DRM_IOCTL_SYNCOBJ_TIMELINE_WAIT` = `0xc030_64ca` | core DRM `IOWR(0xCA)` |
| `LabWaitPath` / `choose_lab_wait_path` / `_from_env` | pure env+handle selection |
| `DirectSubmission::wait_command_with_syncobj` | trait; Real issues syncobj or WAIT_CMD |
| `RealDirectSubmission::{wait_cmd_ioctl,syncobj_timeline_wait_ioctl}` | ioctl helpers |
| `submit_and_wait_command` | **unchanged default** → WAIT_CMD only |
| `submit_and_wait_command_ex(..., syncobj_handle)` | research-capable wait |
| `SubmitWaitResult.wait_path` | soft token `wait_cmd` / `syncobj` |
| timed matmul | uses `_ex` with `ctx.syncobj_handle` (env still default off) |
| Stub / SharedReal / host_graph | forward / gate without hardware |

**Env name:** `XDNA2_LAB_WAIT_VIA_SYNCOBJ` (`ENV_LAB_WAIT_VIA_SYNCOBJ` in silicon_contract).

---

## Gates (offline green)

```bash
cargo fmt -p dragon-npu
cargo test --features amd-xdna2 --lib syncobj
# 6 passed (pack + create handle + wire layout + real/stub env)
cargo test --features amd-xdna2 --lib choose_lab_wait
# 1 passed
cargo test --features amd-xdna2 --lib wait_command
# 4 passed
cargo clippy --features amd-xdna2 --lib -- -D warnings
# clean
```

---

## L8 unlock criteria (design only — **not** minted)

Later live research (L8) needs **all** of:

1. Offline green pack (W4a) + this wire (W4c) tests.
2. Named offline reason that is **not** "syncobj wire landed alone" (compare to
   prior live residual classes: skip-CU isolation under **CONSUMED**
   `C_UNLOCK=1f761bf4` → `wait_cmd_einval` without overlay; L5b Dev debug-BO
   under **CONSUMED** `C_UNLOCK=4008e6fd` → Assign ok then WAIT EINVAL).
3. One **new** host-minted `C_UNLOCK=` (do not reopen CONSUMED list).
4. Live shot under that unlock with research env set **only if** CREATE returns
   non-zero syncobj; record `wait_path=` + last_apply + journal.
5. Pass still requires live ERT Completed + non-zero OFM allclose via
   `decide_ran_on_npu`. Syncobj wait alone ≠ Pass. `invent_pass=0`.

**This slice does not mint L8.**

---

## Unlock justified?

**No.** Offline research wire ≠ live residual root ≠ Pass ≠ unlock.

Do **not** mint. Do **not** re-live CONSUMED unlocks / spent flexml. Do **not**
set sticky ALLOW.

---

## invent_pass

**0**

## not_live_proven

**1**

## Explicit non-claims

- No `ran_on_npu`
- No product Pass
- No live SYNCOBJ_TIMELINE_WAIT proven on silicon this slice
- No default product wait path change when env unset
- No L8 unlock mint
- No claim that syncobj wait clears live `wait_cmd_einval`

---

## Highest remaining (unchanged)

Working-stack residual-matched / load-accepted program material. Live queue:
L1 blocked; L4 not open from unspent alone; L5/L5b closed; L7 no; **L8** needs
this wire (done offline) **plus** a **new** host unlock after a named offline
reason.
