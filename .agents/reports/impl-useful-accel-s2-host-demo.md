# Implement report: S2 host-graph tiny linear demo

**Date:** 2026-08-12
**Plan step:** S2 (useful-soon CPU fallback while S1 external runner is wired)
**Honesty:** invent_pass=0 · `ran_on_npu` stays false · this is **not** NPU acceleration · not product Pass

## Named delta

Checked-in tiny **MatMul + Relu + Add** host fixture plus a named unit test that evaluates it on `evaluate_host_graph` with a known golden.

`evaluate_host_graph` already implemented those ops. Product evaluate logic was **not** rewritten. Red was the missing fixture path; green was landing the JSON (and a test-only loader).

## Files (write scope only)

| Path | Role |
|------|------|
| `tests/fixtures/host_matmul_relu_add.json` | Tiny 2x2 identity weight + bias `[0.5, 1.5]`. Named nodes so W and bias do not collide. |
| `tests/fixtures/host_matmul_relu_add.PROVENANCE.md` | Host-only, hand-written, not residual material. |
| `src/backends/amd_xdna2/host_graph.rs` | **Tests only:** `evaluate_host_linear_demo` + `fixture_host_matmul_relu_add_known_output`. |

Did **not** edit `src/cli.rs`, `dragon_npu.rs`, `external_runner.rs`, `silicon_contract.rs`, `direct.rs`, `RESIDUAL.md`, `AGENTS.md`, or `testdata/xdna2_cmd/**`.

Existing CLI `compile` / `run` already stay host/sim (`ran_on_npu=false`). Point them at the new fixture the same way as `host_matmul_relu.json`.

## Golden

Input `[-1.0, 2.0]`:

1. MatMul with identity `W` → `[-1.0, 2.0]`
2. Relu → `[0.0, 2.0]`
3. Add bias `[0.5, 1.5]` → **`[0.5, 3.5]`**

`evaluate_host_linear_demo` returns `(out, false)` so the test can lock `ran_on_npu` false. The host path never sets that flag true.

## TDD (same test)

**Test:** `backends::amd_xdna2::host_graph::tests::fixture_host_matmul_relu_add_known_output`

**RED (observed, fixture absent):**

```text
cargo test --features amd-xdna2 --lib fixture_host_matmul_relu_add_known_output -- --nocapture
exit 101
host linear demo fixture: ModelParseError { model: ".../tests/fixtures/host_matmul_relu_add.json", reason: "could not read file: No such file or directory (os error 2)" }
```

**GREEN (same filter after fixture land):**

```text
cargo test --features amd-xdna2 --lib fixture_host_matmul_relu_add_known_output -- --nocapture
exit 0
test ...fixture_host_matmul_relu_add_known_output ... ok
```

Expectations were **not** rewritten to finish green.

## Verify

| Step | Command | Exit |
|------|---------|------|
| fmt | `cargo fmt -p dragon-npu` | **0** |
| clippy | `cargo clippy -p dragon-npu --lib --features amd-xdna2 -- -D warnings` | **0** |
| named test | `cargo test --features amd-xdna2 --lib fixture_host_matmul_relu_add_known_output` | **0** |
| sibling fixtures | `cargo test --features amd-xdna2 --lib fixture_host_` (add_mul, matmul_relu, matmul_relu_add) | **0** (3 ok) |

A parallel S3 wait-fence red (tests in `silicon_contract.rs` before APIs existed) briefly made a later `cargo test --lib` fail to compile. Out of S2 write scope. After those APIs existed, the S2 filter was green again.

## Not claimed

- Product Pass / `decide_ran_on_npu`
- NPU acceleration
- Unlock mint or live apply
- IRON / eepy wrap (S1)
