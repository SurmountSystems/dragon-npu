> **Workspace mirror** of the approved session plan
> (`019f6476-0a84-70a0-a971-be89fa1011fe` / `plan.md`).
>
> **Honesty:** invent_pass=0 · product Pass=**No** · unlock **none** · first
> `C_UNLOCK=` = **`2c9c12e8` CONSUMED**. This plan is **not** a mint.
> Do **not** ask the operator to re-run `just npu-iron`.

# Plan: Useful NPU commands, not another operator rerun

**Date:** 2026-08-12
**Honesty:** invent_pass=0 · product Pass=**No** · unlock **none** (first `C_UNLOCK=` = **`2c9c12e8` CONSUMED**)

**Explore inputs:**
- `.agents/reports/plan-explore-next-residual.md`
- `.agents/reports/plan-explore-next-useful.md`

You do **not** need to run `just npu-iron` again. The agent already ran it this session (`child_said_pass=true`, `ran_on_npu=false`). Asking you to reconfirm that is wasted time. This plan is the next real product increment after that wrap.

---

## Context

Two goals stay distinct.

1. **Useful work on this Ryzen NPU this week.** IRON already completed vector add, eltwise add, relu, and softmax on firmware 1.1.2.64. That is real device compute through XRT. It is **not** product Pass.
2. **Product Pass** (`decide_ran_on_npu`: real ioctl + ERT Completed + OFM allclose). Still **No**. Residual class is still **`wait_cmd_einval`**. Unlock **none**. Same-shot live under `2c9c12e8` is CONSUMED. `live_justified=0`.

What already shipped and is done:

- `just npu-iron` plus `tools/npu_iron_env.sh` resolve mlir-aie and the capture venv. They spawn `vector_scalar_add.py --dev npu2`. Proven this session.
- `dragon-npu --external-runner` wraps an argv. JSON says `execution=external_runner` and **`ran_on_npu=false`** even when the child prints `PASS!`.
- Host linear graph exists (MatMul + Relu + Add). Fixture `host_matmul_relu_add.json` expects `[-1, 2]` → `[0.5, 3.5]`. Default `dragon-npu run` still does simulation tanh and never calls that graph.
- Success from the external runner still returns an **empty** `Vec<f32>`. The wrap proves the child ran. It does not give you numbers.

What still blocks product Pass (not this plan):

- Residual-matched / load-accepted classic **1688** under `testdata/xdna2_cmd/external_known_good/` with PROVENANCE. Public peels 300 / 660 / 2592 / 4512 and unpatched oracle `3c5ffca750922d3b` do **not** count.
- Human holds that material. Agent cannot invent it.
- Next product live needs a **new** unlock and a **new** named reason. Diagnostics alone are not a reason.

What is **not** collectable after `just npu-iron`: the mlir-aie example directories are sources only. No new xclbin or insts to ingest. AOT already lives in `mlir-aie-capture` and in-tree RESEARCH peels. Do **not** retag those as load-accepted.

---

## Approach

**Change the product command surface. Do not change Pass law. Do not ask you to rerun proven recipes.**

After Approve, the agent ships a named IRON catalog so `dragon-npu` and `just` can run the four kernels this machine already completed, without you setting `MLIR_AIE` or pasting argv. When the child prints or writes numbers, `run` returns those numbers instead of an empty tensor. When it does not, keep empty plus honest `child_said_pass`. A second step wires the existing host linear fixture to `dragon-npu run --host-graph` so default-path `run` is not only tanh.

Product Pass stays idle. No mint. No reopen of `2c9c12e8`. No operator command to "just try it."

Material alternatives rejected:

- **Not another `just npu-iron` handoff** — already green this session.
- **Not a Pass / live / unlock plan** — no residual-matched 1688, `live_justified=0`.
- **Not MobileNet / ResNet this slice** — needs torch and a long compile. Unrealistic ask.
- **Not eepy VitisAI chase** — listed provider, CPU fallback only.
- **Not host-graph as if it were NPU** — host only; label it host.
- **Not retagging public AOT peels** as load-accepted.

---

## Critical files

| Path | Why |
|------|-----|
| `justfile` | Named recipes `npu-eltwise`, `npu-relu`, `npu-softmax` next to `npu-iron` |
| `tools/npu_iron_env.sh` | One resolve path: mlir-aie root, capture venv, `PYTHONPATH`, named kernel cwd + argv |
| `src/cli.rs` | `--iron NAME` and `--host-graph` parse + help; lock tests |
| `src/bin/dragon_npu.rs` | Dispatch named kernel / host graph; print honest JSON |
| `src/backends/external_runner.rs` | Kernel table; optional stdout/npy parse; empty tensor when no numbers |
| `src/backends/amd_xdna2/host_graph.rs` | Existing eval; CLI should call it |
| `docs/SECONDARY_BACKENDS.md` | Lead with named `just` / `--iron`; never claim Pass |
| `RESIDUAL.md` / `AGENTS.md` | Honesty pin after land: catalog ≠ Pass ≠ unlock |

---

## Reuse

| Symbol / module | Path | How |
|-----------------|------|-----|
| `npu_iron_env.sh` | `tools/npu_iron_env.sh` | Extend with a kernel name → cwd + python argv. Do not invent a second env script. |
| `ExternalRunnerBackend` | `src/backends/external_runner.rs` | Keep argv-only (no `sh -c`). Add a named-kernel constructor. Keep `ran_on_npu=false`. |
| `evaluate_host_graph` | `src/backends/amd_xdna2/host_graph.rs` | Call from CLI `--host-graph`. Do not invent a second evaluator. |
| `host_matmul_relu_add.json` | `tests/fixtures/host_matmul_relu_add.json` | Golden for `--host-graph` demo. |
| IRON examples already proven | `/home/hunter/Projects/ai/mlir-aie/programming_examples/` | `basic/vector_scalar_add`, `ml/eltwise`, `ml/eltwise_unary` (relu), `ml/softmax`. Do not edit the sibling tree. |

---

## Steps

1. **Honesty pin** — Residual and standing docs: this plan is a useful-command slice. Pass stays No. Unlock stays none. Do not mint. Do not ask the operator to re-run `just npu-iron`.

2. **Named IRON catalog (product)** — One table of four kernels (`vector_add`, `eltwise`, `relu`, `softmax`). `just npu-eltwise` / `npu-relu` / `npu-softmax` reuse `npu_iron_env.sh`. CLI: `dragon-npu run --iron relu` (and the other names) fills cwd + argv. No `MLIR_AIE` required. Child `PASS!` stays `execution=external_runner` and `ran_on_npu=false`.

3. **Return numbers when the child actually emits them** — If stdout or a known npy next to the example has a parseable vector, `run()` returns that `Vec<f32>`. If not, keep today's empty vector plus `child_said_pass`. Never invent OFM. Never set `ran_on_npu`.

4. **Host linear on the CLI** — `dragon-npu run --host-graph` (or compile+run of the existing fixture) calls `evaluate_host_graph`. JSON says host, not NPU. Default `run` without flags can stay simulation tanh so we do not surprise existing tests; the new flag is the honest host path.

5. **Agent verify, operator idle** — Implementer runs the new `just` recipes and CLI flags. You are not asked to paste a command. Report child exit, `child_said_pass`, `ran_on_npu=false`. Do not retag peels. Do not mint.

6. **Join** — Residual + `AGENTS.md` Latest Turn: catalog landed; still no Pass; unlock none; next Pass move is still human 1688.

---

## Risks

- **Child scripts print `PASS!` but no tensor** — Keep empty output + honesty tokens. Do not fabricate numbers.
- **Re-running four IRON kernels takes minutes** — Agent owns that after Approve. Not an operator chore. Skip a kernel only if the host env is actually missing (report which).
- **Temptation to call IRON `PASS!` product Pass** — Tests and JSON stay `ran_on_npu=false`. Soft always `invent_pass=0`.
- **Sibling mlir-aie edits** — Forbidden. Catalog points at existing examples only.

---

## Verification

Red → green (named contracts):

- CLI parse: `--iron relu` (and the other three names) builds the documented cwd + argv. Unknown name fails closed. Command: `cargo test --lib iron_kernel` (new filter) after a failing test that expects the table.
- Honesty: success path still `execution=external_runner`, `ran_on_npu=false`, `invent_pass=0` even when stdout is `PASS!`.
- Output parse: fixture stdout / tiny npy → non-empty `Vec<f32>`; missing numbers → empty vec, not an error, not Pass.
- Host graph: `--host-graph` on `tests/fixtures/host_matmul_relu_add.json` returns `[0.5, 3.5]` (or the documented fixture pair) with execution labeled host.

After green tests, agent runs `just npu-relu` (and siblings if cheap). Operator does **not** rerun.

Gates on touched packages: `cargo fmt`, `cargo clippy -p dragon-npu --all-targets -- -D warnings`, targeted tests above.

---

## Who does what

| Who | Does | Does not |
|-----|------|----------|
| **Agent** (after Approve) | Catalog, parse, host-graph CLI, tests, agent-run verify, residual pin | Mint unlock; claim `ran_on_npu`; retag peels; edit mlir-aie |
| **You** | Approve / Revise / Quit in the plan panel | Re-run `just npu-iron`; set `MLIR_AIE`; install torch; drop 1688 unless you already hold it |

Product Pass remains human-gated on residual-matched classic 1688. That is not a step in this plan.

---

## Open questions

None blocking. Assumed: you want useful product commands next, not another Pass theater loop.

If that is wrong, Revise in the plan panel and say you want Pass-only idle instead.

---

```
invent_pass=0 product_pass=0 ran_on_npu=0 unlock=none C_UNLOCK=2c9c12e8 CONSUMED
```

### Critical Files for Implementation
- `justfile` — named IRON recipes
- `tools/npu_iron_env.sh` — kernel table + env resolve
- `src/cli.rs` — `--iron` / `--host-graph`
- `src/bin/dragon_npu.rs` — dispatch + honest JSON
- `src/backends/external_runner.rs` — catalog + optional output parse
- `src/backends/amd_xdna2/host_graph.rs` — existing host eval
- `docs/SECONDARY_BACKENDS.md` — operator-facing commands
- `RESIDUAL.md` — honesty pin after land
