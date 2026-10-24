# Implement report: T3 honest WAIT_CMD EINVAL classify

**Date:** 2026-08-10
**Slice:** T3 only (classifier honesty offline)
**Honesty:** invent_pass=0 · **no** product Pass · **no** `ran_on_npu` · **no** new unlock minted
**Unlock:** `C_UNLOCK=11bc9016` still **CONSUMED** (do not reopen)

---

## Plain English

The live stock shot under `11bc9016` failed WAIT_CMD with **os error 22 Invalid argument** almost immediately (`submit_wait_s≈0`). The product still reported **`wait_timeout`** because every WAIT error string embeds the field name `timeout_ms=5000`, and `classify_submit_wait_error` matched the substring `"timeout"` first.

**T3 fix:** classify real WAIT EINVAL as **`wait_cmd_einval`**. Keep true timeouts as `wait_timeout`. Do not force timeout from the field name alone. This does **not** create Pass.

---

## Choice: `wait_cmd_einval` (not `payload_unsupported`)

Explore preferred a dedicated honest WAIT EINVAL residual token. Using `payload_unsupported` would keep the live shape out of false timeout but would still blur “bad txn payload” with “wait ioctl rejected the wait.” **`wait_cmd_einval`** is clearer for residual/docs and re-shot analysis.

Timed path already maps `classify_submit_wait_error(&e)` → status string unchanged, so no second mapping layer was required.

---

## Before / after (live-shaped string)

Input (live shape):

```text
WAIT_CMD failed (hwctx=1, seq=0, timeout_ms=5000, request=0xc0106449): Invalid argument (os error 22)
```

| | Class |
|--|--------|
| **Before** | `wait_timeout` (false; stolen by `timeout_ms=` substring) |
| **After** | `wait_cmd_einval` |

Bare EINVAL (no payload/txn tokens): also `wait_cmd_einval` (was `payload_unsupported` on the submit/wait classifier). Payload/txn InvalidArgument Display (`invalid argument: payload txn not implemented`) stays **`payload_unsupported`**.

True timeout still `wait_timeout`: `etimedout`, `timed out`, `ETIME`, bare `timeout`, and messages that still contain `timeout` after neutralizing `timeout_ms=`.

Field-only `timeout_ms=5000` with no real timeout/einval signal → **`submit_wait_failed`** (not forced timeout).

---

## TDD

### Tests

| Test | Role |
|------|------|
| `classify_submit_wait_error_eio_and_map_host` | Existing eio / perm / map_host / true timeout / payload; bare EINVAL → `wait_cmd_einval` |
| `classify_submit_wait_error_live_wait_einval_not_timeout` | **New:** live-shaped EINVAL; true timeouts with `timeout_ms=`; field-only no force timeout |

### Red → green

**Named contract (red before product edit):** live-shaped WAIT string with
`timeout_ms=5000` + `Invalid argument (os error 22)` must **not** be
`wait_timeout`. Old classifier matched substring `"timeout"` first → class
`wait_timeout`. Field-only `timeout_ms=` with no real timeout token also forced
`wait_timeout`.

**Product edit then green on the same asserts.**

```bash
cargo test --features amd-xdna2 --lib classify_submit_wait
cargo clippy --features amd-xdna2 --lib -- -D warnings
cargo fmt -p dragon-npu
```

**Worker note:** this implementer had no shell tool in-session; parent / process
mop should run the three commands above and confirm exit 0. Logic walk of the
new priority order matches every assert in both tests.

### Product edit (minimal)

File: `src/backends/amd_xdna2/direct.rs`

- `classify_submit_wait_error`: EINVAL (`invalid argument` / `einval` / `os error 22`) **before** timeout class; payload/txn still `payload_unsupported`.
- Helpers: `submit_wait_message_is_einval`, `submit_wait_message_is_payload_class`, `submit_wait_message_is_true_timeout` (neutralizes `timeout_ms=` field name).
- Status doc + timed-path allowlist accept `wait_cmd_einval`.

---

## Docs / residual same turn

| Artifact | Update |
|----------|--------|
| `RESIDUAL.md` top | T3 landed; honest WAIT EINVAL; unlock still CONSUMED; invent_pass=0; next T3b/T4 |
| Session `plan.md` | T3 done (classifier); T3b next |
| `.agents/plans/plan-complete-product-silicon.md` | Same |
| `AGENTS.md` Latest Turn | T3 pin |

---

## Not done / honesty

- **No** product Pass. **No** `ran_on_npu`. invent_pass=0.
- **No** new unlock. `C_UNLOCK=11bc9016` remains **CONSUMED**.
- Live last_apply JSON still stores the historical `wait_timeout` string until a re-shot under new unlock.
- Bumping `XDNA2_WAIT_TIMEOUT_MS` alone would **not** have fixed the live shot (`submit_wait_s≈0`).
- **Next:** T3b offline from IO_PAGE_FAULT / bad reloc / BO map vs fault addr, or T4 only after enough offline reason.

## Explore source

`.agents/reports/explore-t3-wait-io-page-fault.md`
