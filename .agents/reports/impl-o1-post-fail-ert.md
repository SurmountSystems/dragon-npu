# O1 P5 — post-fail ERT / Cmd BO dump (offline)

**Date:** 2026-08-11
**Kind:** named offline delta (plan option1 explore P5)
**Unlock:** first `C_UNLOCK=` = **`2c9c12e8` CONSUMED** — not reopened, not minted
**Pass:** **No**. invent_pass=0. **no** `ran_on_npu`.

## Why this is not empty reconfirm

P0–P2 joined with `live_justified=no`. Primary wall remains residual-matched
material (human). Explore P5 was still open agent-doable work: after
`wait_cmd_einval`, product last_apply should greppably record whether Cmd BO
ERT was readable so the **next** justified live can split R1 vs R4 vs
expected-absent.

## What landed

### Pure API (`silicon_contract.rs`)

| Symbol | Role |
|--------|------|
| `POST_FAIL_ERT_RESEARCH_NOTE` | soft bag name |
| `PostFailErtPresence` | present / unread / absent / expected_absent / missing_from_detail |
| `PostFailErtResearch` | parsed bag + `split_hint` (hypothesis only) |
| `parse_post_fail_ert_research` | status+detail → bag |
| `format_post_fail_ert_research_soft` / `_note` | soft line; **hard-coded** invent_pass=0 |
| `format_post_fail_ert_detail_tokens` | product greppable detail tokens |
| `post_fail_ert_state_name` | pure ERT name map (no `direct` dep) |

Soft always includes: `invent_pass=0 not_c_ready=1 not_ran_on_npu=1
research_alone_not_pass=1 research_alone_not_unlock=1
product_pass_requires_live_decide=1 hypothesis_not_proven_root=1`.

`split_hint` labels (not proven root):

- `r4_fw_reject_hint` — `ert_state=5(Error)` present
- `r1_fence_or_state_present` — numeric state present, not Error
- `r1_fence_maybe_unread` — `ert_state=unread`
- `pre_exec_ert_expected_absent` — fail before EXEC / no Cmd BO
- `historical_no_ert_token` — pre-P5 last_apply shape (live `2c9c12e8`)

### Product wire (`direct.rs`)

Timed matmul already read Cmd BO ERT header after `submit_and_wait` (before
GEM_CLOSE). **Ok** path already grepped `ert_state=N(Name)`. **Err** path
historically dropped it → same-shot last_apply had no ERT token.

**WAIT Err** appends tokens from **local** facts (`ert_read_attempted`,
`cmd_bo_valid_for_ert` next to the header read):

- readable → `ert_state=N(Name) ert_post_fail=present`
- read fail → `ert_state=unread ert_post_fail=unread`

**Pre-EXEC** (`cmd_bo_failed`, `cmd_bo_fill_failed`) append
`format_post_fail_ert_detail_tokens(None, false, false)` →
`ert_post_fail=expected_absent`.

### Soft classify (`examples/classify_last_apply.rs`)

Prints `post_fail_ert_research=…` when status/detail present (same gate as
wait-path soft).

### Re-exports

`amd_xdna2` re-exports pure API + types + note constant.

### Docs

- `RESIDUAL.md` Latest Turn pin (top); mid-file unlock parse line corrected to
  first = `2c9c12e8` CONSUMED (was lagging `1f761bf4`).
- `AGENTS.md` Latest Turn pin.

## TDD

| Step | Evidence |
|------|----------|
| **RED** | `cargo test --features amd-xdna2 --lib post_fail_ert_research_from_wait_cmd_einval_detail` → compile fail `cannot find function parse_post_fail_ert_research` / `PostFailErtPresence` (test landed before product) |
| **GREEN** | same filter → **ok** (1 passed) |

## Post-impl verify

| Step | Command | Exit |
|------|---------|------|
| fmt | `cargo fmt -p dragon-npu` | 0 |
| clippy lib | `cargo clippy -p dragon-npu --features amd-xdna2 --lib -- -D warnings` | 0 |
| test | `cargo test --features amd-xdna2 --lib post_fail_ert` | 0 (1 ok) |
| example check | `cargo check --features amd-xdna2 --example classify_last_apply` | 0 |

Note: `clippy --all-targets` still hits pre-existing `txn.rs` test lints
unrelated to this slice; lib path (product + unit tests) is clean.

## Explicit honesty

- **invent_pass=0** hard-coded on soft note
- **no** Pass claim
- **no** unlock mint; **`2c9c12e8` remains CONSUMED**
- **no** live NPU / L3 / ALLOW
- **no** invent residual-matched material
- post-fail ERT observability ≠ residual root fix ≠ C-ready

## Review fix-round (all open issues)

1. Pin text: WAIT Err = `present|unread` only; pre-EXEC = `expected_absent`.
2. Pre-EXEC `cmd_bo_failed` / `cmd_bo_fill_failed` wire `expected_absent`.
3. Unit tests expanded: absent parse/format, completed split_hint, soft honesty
   tokens, format→parse round-trip, soft on Error with invent_pass=0.
4. WAIT Err uses local `ert_read_attempted` / `cmd_bo_valid_for_ert` (not bare
   `true, true` at the format call site).

## Next

Human: residual-matched / load-accepted material into
`testdata/xdna2_cmd/external_known_good/`.
Agent: only new named offline delta; freeze empty same-shot reconfirm.
Next justified live under **new** unlock can carry greppable post-fail ERT.
