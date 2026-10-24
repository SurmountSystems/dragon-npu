# P1-B — Wait-path offline (option1-after-same-shot)

**Date:** 2026-08-11
**Mode:** offline only. No live NPU. No unlock mint. No L3 / ALLOW.
**Plan:** `.agents/plans/plan-option1-after-same-shot.md` step P1-B
**Honesty:** invent_pass=**0** · Pass=**No** · unlock **none** · wire alone ≠ residual root

---

## Plain English

1. Product still fails with **`wait_cmd_einval`** (WAIT_CMD refused with EINVAL almost immediately). That is not a hang and not Pass.
2. CREATE_HWCTX already returns a DRM **syncobj** handle. Live same-shot under consumed `2c9c12e8` already greps `syncobj=1` and `wait_path=wait_cmd` in last_apply detail.
3. XRT prefers drm_syncobj timeline wait when the handle is non-zero; product default remains **WAIT_CMD**. That parity gap is a **hypothesis**, not a proven residual root.
4. This slice only tightens **offline observability** + soft classify. It does **not** default product wait to syncobj live, does **not** mint unlock, does **not** invent Pass.

---

## What already existed (mostly green)

| Piece | State before this slice |
|-------|-------------------------|
| `HardwareContext.syncobj_handle` retained on CREATE | Yes (W1 O2); unit `create_hwctx_records_syncobj_handle_in_context` |
| Detail greppable `syncobj=` when handle non-zero | Yes on timed path (`hwctx ok … syncobj=N`); live `2c9c12e8` has `syncobj=1` |
| Detail greppable `wait_path=` on wait fail | Yes after W4c (`wait_path=wait_cmd` / `syncobj`) |
| Pure XRT decision: `should_wait_via_syncobj` / `select_wait_completion_path` / `pack_syncobj_timeline_wait_args` | Green (W3 O13) |
| Pure pack + errno classify: `pack_syncobj_timeline_wait_ioctl` / flags / ETIME vs EINVAL | Green (W4a T5) |
| Env-gated research wire: `XDNA2_LAB_WAIT_VIA_SYNCOBJ` + `wait_command_with_syncobj` | Green offline (W4c); product default WAIT_CMD |
| `LastApplyRichTokens` / parse `syncobj=` / `wait_path=` | Green (W4a T3); unit covered L5b + wait_path shapes |
| Soft classify wait EINVAL axes + driver axes | Present |
| Soft classify rich tokens / wait-path research bag | **Missing** → filled this slice |

Prior reports: `.agents/reports/wave3-syncobj-wait-offline.md`, `wave4-syncobj-pack.md`, `wave4-syncobj-wire.md`, `wave1-o2-wait-syncobj.md`.

---

## What changed (this slice only)

1. **`format_syncobj_wait_path_decision_note` / pack note** always emit
   `wait_path_syncobj_research=1 invent_pass=0 not_c_ready=1 not_ran_on_npu=1`.
2. **New pure helper** `format_wait_path_syncobj_research_soft(status, detail)`:
   greppable detail_syncobj / detail_wait_path + rich note + decision note;
   always `research_alone_not_pass=1 research_alone_not_unlock=1`.
3. **Soft classify** (`examples/classify_last_apply.rs`) prints
   `last_apply_rich_tokens=` and `wait_path_syncobj_research=` when status/detail present.
4. **Re-export** on `amd_xdna2` for the new formatter.
5. **Unit test** `wait_path_syncobj_research_soft_from_last_apply_detail` (live-shaped detail).

**Not changed:** product default wait path (still WAIT_CMD); no live ioctl; no unlock mint; no Pass claim.

---

## Gates

```text
cargo test --features amd-xdna2 --lib wait_path_syncobj   # 1 ok
cargo test --features amd-xdna2 --lib syncobj              # 7 ok
cargo test --features amd-xdna2 --lib last_apply_rich      # 1 ok
cargo fmt -p dragon-npu
cargo clippy --features amd-xdna2 --lib -- -D warnings     # clean
cargo check --features amd-xdna2 --example classify_last_apply
```

---

## live_justified_from_wait_theory

**no**

Reason: this slice is observability + soft honesty only. No **new** measurable wait theory that would justify a live A/B under a new unlock (e.g. no offline proof that WAIT_CMD EINVAL is solely “forgot syncobj,” and no new last_apply residual class). Existing research wire remains env-gated offline; product default WAIT_CMD. Wait theory alone still ≠ residual-matched material.

---

## invent_pass

**0**

## Pass

**No**

## Unlock

**none** (do not mint from P1-B; do not reopen CONSUMED `2c9c12e8`)

---

## Explicit non-claims

- No `ran_on_npu`
- No product Pass
- Syncobj research wire alone ≠ residual root of `wait_cmd_einval`
- Soft `wait_path_syncobj_research=1` ≠ unlock
- Do not default product wait to syncobj live
- Do not run L8-style live from wire alone

---

## Who acts next

- **P1-A** (material / Bridge A) in parallel if not already reported
- **P2 join** after both arms: one named live reason or stop
- Live only if P2 yes under **new** unlock + **new** named reason
