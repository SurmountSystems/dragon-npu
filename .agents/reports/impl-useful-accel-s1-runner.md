# Implement report: useful-accel S1 (external-runner)

Plan slice: approved dual-track S1. Honest opt-in wrap for a working host stack (IRON first).

## Honesty (locked)

- `execution=external_runner`
- `ran_on_npu` always `false` (constant `RAN_ON_NPU`)
- `invent_pass=0` (constant `INVENT_PASS`)
- `product_known_good=false`
- Child stdout `PASS!` / `PASS` sets informational `child_said_pass` only
- Never sets `XDNA2_ALLOW_EXEC_CMD`; process runner strips it from the child
- Missing binary is unavailable / explicit error, not silent simulation Pass
- Not default `DragonNpu::new`
- Do not ingest IRON peels as product known-good
- No unlock mint. No product Pass claim.

## Product

`ExternalRunnerBackend` implements `Backend` (`status` / `compile_model` / `run` / `submit_and_run`).

- `status`: `available` only when the configured program exists as a file (symlink-to-file OK). `simulated=true`. Device name says wrap, not product silicon.
- `compile_model`: host `ModelCompiler` only.
- `run` / `submit_and_run`: refuse if binary missing; else spawn via injected `CommandRunner` or `ProcessCommandRunner`; non-zero child exit is an error; success returns an empty `Vec<f32>` (not a fake host golden).
- `run_external()` is the usable wrap API: report + JSON tokens.

Spawn: `std::process::Command::new(program).args(args)`. Program must be absolute, no `..`, no PATH search. Cwd if set must be an absolute existing directory, no `..`.

Construct: `ExternalRunnerBackend::new(config)` or `DragonNpu::with_backend`. Tests inject `ScriptedRunner` so mlir-aie is not required on PATH.

## CLI

- `--external-runner PATH` (absolute). Env `DRAGON_NPU_EXTERNAL_RUNNER` if flag omitted.
- `--external-arg ARG` repeatable; next token is literal (`--dev` is allowed).
- `--external-cwd DIR`. Env `DRAGON_NPU_EXTERNAL_CWD`.
- Default off. Host/sim `run` path unchanged when unset.
- External `run` compiles host-side once, then `run_external` once (does not double-spawn via `CompiledModel::run`).
- Text/JSON print `execution=external_runner`, `ran_on_npu=false`, `invent_pass=0`.

IRON argv (sibling tree; do not edit mlir-aie from this crate): see rustdoc on `external_runner.rs` and `docs/SECONDARY_BACKENDS.md`.

## Tests (named contracts)

1. `external_runner_child_pass_does_not_set_ran_on_npu` — fixture stdout `PASS!` → `execution=external_runner`, `ran_on_npu=false`, `invent_pass=0`, JSON same, `run` is not a silent host tensor.
2. `external_runner_missing_binary_is_unavailable` — missing path → `available=false` + explicit error.
3. Also: `external_runner_refuses_shell_injection`, `external_runner_empty_or_malformed_stdout`, `external_runner_real_echo_argv_is_not_shell`, `dragon_npu_new_does_not_select_external_runner`, `parse_run_external_runner_args`, `parse_external_runner_missing_value_is_usage`.

## RED then GREEN

**RED** (prior turn, before product honesty):

```text
cargo test --lib --features amd-xdna2 external_runner
```

- `external_runner_child_pass_does_not_set_ran_on_npu` failed: `ran_on_npu=true` from child `PASS!`.
- `external_runner_missing_binary_is_unavailable` failed: `Ok([0.5; 4])` silent host Pass.

**GREEN** (this continuation, same filter):

```text
export PATH="/usr/bin:$PATH"
cargo test --lib --features amd-xdna2 external_runner -- --nocapture
```

exit **0**. 8 passed (includes CLI parse tests matching `external_runner`).

## Verify commands

| Command | Exit |
|---------|------|
| `cargo fmt -p dragon-npu` | 0 |
| `cargo clippy -p dragon-npu --lib -- -D warnings` | 0 |
| `cargo clippy -p dragon-npu --bins -- -D warnings` | 0 |
| `cargo test --lib --features amd-xdna2 external_runner` | 0 |

`--all-targets` clippy stays red on **pre-existing** examples (`c4_pass_check`, `p6_true_pass_check`, `classify_last_apply`) outside this write scope.

## Files

Created: `src/backends/external_runner.rs`

Edited: `src/backends/mod.rs`, `src/lib.rs`, `src/core.rs`, `src/cli.rs`, `src/bin/dragon_npu.rs`, `docs/SECONDARY_BACKENDS.md`

Not edited: `src/backends/xrt_bridge.rs`, `src/backends/amd_xdna2/**`, `RESIDUAL.md`, `AGENTS.md`, `testdata/**`

## Next (not this slice)

S2 host graph / S3 silicon_contract stay on their owners. Live IRON child on the sibling tree is operator `run` with `--external-runner`; still not product Pass.
