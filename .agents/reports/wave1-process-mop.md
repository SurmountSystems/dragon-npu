# Wave W1 process mop — O2 fmt/clippy/tests

**Date:** 2026-08-11
**Role:** process mop L2 (O2 fallout only)
**Honesty:** invent_pass=**0** · Pass=**No** · no unlock mint · no live · not C-ready · not `ran_on_npu`

**O2 land report:** `.agents/reports/wave1-o2-wait-syncobj.md`

---

## Verdict

| Check | Result | Exit code |
|-------|--------|-----------|
| `cargo fmt -p dragon-npu` | **NOT RUN** (shell tool absent in this L2) | n/a |
| `cargo clippy --features amd-xdna2 --lib -- -D warnings` | **NOT RUN** | n/a |
| `cargo test --features amd-xdna2 --lib wait_cmd_einval_driver_axes` | **NOT RUN** | n/a |
| `cargo test --features amd-xdna2 --lib create_hwctx_records_syncobj` | **NOT RUN** | n/a |
| O2 static compile-shape review | **OK** (no missing-field / missing-export fallout found) | n/a |
| Files fixed this mop | **none** | — |

**Overall:** **YELLOW / incomplete** — static review green; cargo gates **not** executed in this session. Parent (or a mop L2 with shell) must re-run the command block below and record exit codes.

---

## Why cargo was not run

This process-mop subagent’s tool schema included read/grep/edit/todos only.
Host shell tools (`run_terminal_cmd` / `run_terminal_command`) were **not** available, so fmt/clippy/test exit codes could not be captured here.

Same class of limit noted on O2 implement report: “Parent or process mop should re-run the filters… if this implementer shell did not capture exit codes.”

---

## Parent re-run commands (required for green)

From repo root `/home/hunter/Projects/ai/dragon-npu`:

```bash
cargo fmt -p dragon-npu
cargo clippy --features amd-xdna2 --lib -- -D warnings
cargo test --features amd-xdna2 --lib wait_cmd_einval_driver_axes
cargo test --features amd-xdna2 --lib create_hwctx_records_syncobj
```

Optional extra (O2 implement also listed):

```bash
cargo test --features amd-xdna2 --lib wait_cmd_einval_axes
```

Expect: all exit **0**. If clippy/fmt dirties tree, re-run fmt then clippy; only fix O2 fallout in:

- `src/backends/amd_xdna2/silicon_contract.rs`
- `src/backends/amd_xdna2/direct.rs`
- `src/backends/amd_xdna2.rs`
- `examples/classify_last_apply.rs`

---

## Static review (O2 only) — what was checked

### Driver WAIT axes (pure)

| Item | Path | Status |
|------|------|--------|
| `WaitCmdDriverAxis` / `WaitSubmitProductClass` / `WaitCmdEinvalDriverAxesMap` | `silicon_contract.rs` | present |
| `map_wait_cmd_einval_driver_axes` | same | present; EINVAL vs ETIME split; fence journal wins over hwctx when both |
| `format_wait_cmd_einval_driver_axes_note` | same | soft tokens include `invent_pass=0 not_c_ready=1 not_ran_on_npu=1 axes_alone_not_pass=1` |
| Unit test `wait_cmd_einval_driver_axes_map_hwctx_vs_fence_vs_etime` | same (~24906) | present (skip-CU undifferentiated, fence, hwctx, ETIME, other, helpers) |
| Re-exports | `amd_xdna2.rs` | `WaitCmdDriverAxis`, `WaitCmdEinvalDriverAxesMap`, `WaitSubmitProductClass`, `WAIT_CMD_EINVAL_DRIVER_AXES_NOTE`, map/format + journal helpers |

### Syncobj retain

| Item | Path | Status |
|------|------|--------|
| `HardwareContext.syncobj_handle` | `direct.rs` | field + docs (WAIT_CMD still product path) |
| Real CREATE wires `req.syncobj_handle` | `direct.rs` try_create | present |
| Stub fake non-zero syncobj | `StubDirectSubmission::create_hwctx` | present |
| All `Ok(HardwareContext {…})` sites | only real + stub | both set `syncobj_handle` (no missing-field fallout) |
| last_apply / wait-fail detail `syncobj=` | progressive stamp + wait fail path | present |
| Unit test `create_hwctx_records_syncobj_handle_in_context` | `direct.rs` tests | present |
| `HwCtxRecord` / destroy path | `amd_xdna2.rs` | still UMQ/log only (syncobj close not product path; intentional O2 scope) |

### Soft classify example

| Item | Path | Status |
|------|------|--------|
| import map + format | `examples/classify_last_apply.rs` | present |
| `print_wait_cmd_einval_driver_axes_soft` | same | soft print; invent_pass via format note |

### Honesty locks (static)

- Soft map note always: `invent_pass=0`, `not_c_ready=1`, `not_ran_on_npu=1`, `axes_alone_not_pass=1`, `hypothesis_not_proven_root=1`, `product_pass_requires_live_decide=1`, `wait_cmd_einval_neq_wait_timeout=1`, `syncobj_wait_parity_alone_not_pass=1`
- No Pass claim, no unlock mint, no live apply in O2 surface
- Live residual under CONSUMED unlocks unchanged by this mop

---

## Files fixed this mop

**None.** Static review found no O2 missing-field / missing-export fallout. Cargo gate fallout unknown until parent re-runs commands above.

---

## invent_pass

**0**

---

## Explicit non-claims

- Pass = No
- not C-ready
- not `ran_on_npu`
- This report does **not** claim fmt/clippy/test green
- Axes map alone ≠ proven root
- Syncobj field alone ≠ XRT wait parity ≠ Pass
- No unlock mint / no live from this mop
