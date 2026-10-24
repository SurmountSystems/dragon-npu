# Implement report: useful-accel fix round 1

Same feature as S1. Roles stay (this implementer). Review source:
`/tmp/grok-1000/grok-review-useful-accel.md` (plus tests + security slots).

## Honesty (locked)

- invent_pass=0
- Product Pass: **No**
- `ran_on_npu`: never true from this wrap
- Unlock: **none**. Did **not** mint. Did **not** reopen `2c9c12e8`.
- No `XDNA2_ALLOW_EXEC_CMD`. No live apply. No residual/AGENTS/xrt_bridge edits.

## Per issue

### Issue 5 (product, TDD required)

`ExternalRunnerReport::to_json` serialized public `ran_on_npu` / `invent_pass` /
`execution` fields. After mutate, JSON could lie.

1. Landed named test `external_runner_json_honesty_uses_constants_not_mutated_fields`.
2. Observed **RED** (test first, product still field-based):

```bash
export PATH="/usr/bin:$PATH"
cargo test --lib --features amd-xdna2 external_runner_json_honesty_uses_constants_not_mutated_fields
```

Fail reason: `assert_eq!(j["ran_on_npu"], false)` left `Bool(true)` right `false`.
JSON followed mutated fields (`ran_on_npu=true`, `invent_pass=1`,
`execution="host_or_simulation_run"`).

3. Smallest product fix: `to_json` always emits `EXECUTION_EXTERNAL_RUNNER`,
   `RAN_ON_NPU`, `INVENT_PASS`, `PRODUCT_KNOWN_GOOD` constants. Child fields
   (`child_exit_code`, `child_said_pass`, stdout/stderr, path, note) stay
   report-sourced.
4. Same filter **GREEN**: 1 passed, exit 0.
5. Also `assert_eq!(report.ran_on_npu, RAN_ON_NPU)` and
   `assert_eq!(report.invent_pass, INVENT_PASS)` on the unmutated `from_child`
   path in `external_runner_child_pass_does_not_set_ran_on_npu`.

### Issue 6 (security nit)

`src/bin/dragon_npu.rs` human wrap printed `run ok:` before `child_exit_ok()`.

Fix: `run ok:` only when `report.child_exit_ok()`. On child failure print
`run failed: external runner child exited {:?} (not a silent host Pass);`
plus honesty tokens, then the same human token helper. Process exit was already
1 when the child is not ok. JSON path unchanged (uses `to_json` constants).

No new bin/unit test: the CLI wrap has no existing bin harness. Adding one
would be a large spawn/fixture harness for a three-line print branch. Honesty
tokens stay constants; `ran_on_npu` is not invented.

### Issue 1 (tests)

Dropped tautological `evaluate_host_linear_demo` helper (always
`Ok((out, false))`). `fixture_host_matmul_relu_add_known_output` now compiles
the checked-in fixture and locks golden via `evaluate_host_graph` like
`fixture_host_matmul_relu_known_output`. Golden stays `[-1.0, 2.0] → [0.5, 3.5]`.

### Issue 2 (tests)

Added `external_runner_missing_or_unusable_cwd_and_empty_program`:
missing cwd, file-as-cwd (`/usr/bin/true` or `/bin/true`), empty / whitespace
program, relative `./runner`, PATH-relative `python3`.

### Issue 3 (tests)

`external_runner_missing_binary_is_unavailable` now requires `unavailable` **and**
`ran_on_npu=false` **and** `invent_pass=0`. Dropped `|| contains("external")`.

### Issue 4 (tests)

`external_runner_real_echo_argv_is_not_shell` now `panic!` if neither
`/usr/bin/echo` nor `/bin/echo` exists (no silent return). When echo exists:
literal `hello; rm -rf /` argv, `child_said_pass`, `!ran_on_npu`,
`invent_pass == 0`.

## Verify

| Command | Exit |
|---------|------|
| `cargo test --lib --features amd-xdna2 external_runner_json_honesty_uses_constants_not_mutated_fields` (GREEN) | 0 |
| `cargo fmt -p dragon-npu` | 0 |
| `cargo clippy -p dragon-npu --lib -- -D warnings` | 0 |
| `cargo clippy -p dragon-npu --bins -- -D warnings` | 0 |
| `cargo test --lib --features amd-xdna2 external_runner` | 0 (10 passed) |
| `cargo test --lib --features amd-xdna2 fixture_host_matmul_relu_add_known_output` | 0 |
| `cargo test --lib --features amd-xdna2 wait_fence_diagnostic` | 0 |

Did **not** use `cargo fmt --all` or clippy `--all-targets` as a required gate
(examples have pre-existing lint).

## Files

- `src/backends/external_runner.rs` (to_json constants + tests 2–5)
- `src/bin/dragon_npu.rs` (Issue 6 human wrap)
- `src/backends/amd_xdna2/host_graph.rs` (Issue 1 fixture lock)

Did **not** grow `xrt_bridge.rs`. Did **not** edit `RESIDUAL.md` / `AGENTS.md`.

## Remaining open (this implementer)

0. All six review items landed. Next live product apply still needs a **new**
unlock + new named reason; this slice is not that.
