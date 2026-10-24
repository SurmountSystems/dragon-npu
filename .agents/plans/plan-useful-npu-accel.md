> **Workspace mirror** of the approved session plan
> (`019f6476-0a84-70a0-a971-be89fa1011fe` / `plan.md`).
>
> **Operator add-on (already approved, S0 pin):** S5 is **agent-try**, not
> parked human-only. Build the tools we need. Try every agent-doable useful
> path. Minimal operator involvement. No unrealistic asks.
>
> **Honesty:** invent_pass=0 · product Pass=**No** · unlock **none** · first
> `C_UNLOCK=` = **`2c9c12e8` CONSUMED**. This plan is **not** a mint.
>
> **Landed (S4 join, 2026-08-12):** S0–S5 **JOINED**. Useful-soon shipped
> (opt-in `--external-runner` + host linear demo). Residual still
> `wait_cmd_einval`; S3 `live_justified=0` (diagnostic ≠ unlock). S5: IRON
> eltwise/relu/softmax ran on the NPU this week; eepy still CPU; not
> product Pass. Next product live only under a **new** unlock and a **new**
> named reason. Join: `.agents/reports/impl-useful-accel-s4-join.md`.

# Plan: Address blockers and get useful Ryzen NPU acceleration

**Date:** 2026-08-12
**Kind:** Dual-track plan. Useful inference on stacks that already run, plus honest residual law for product Pass.
**Honesty:** invent_pass=0 · product Pass=**No** · unlock **none** (first `C_UNLOCK=` = **`2c9c12e8` CONSUMED**)

**Explore inputs:**
- `.agents/reports/plan-46-explore-residual-blockers.md`
- `.agents/reports/plan-46-explore-working-stacks.md`
- `.agents/reports/plan-46-explore-product-inference.md`

---

## What 4.6 xhigh changes (and what it does not)

A stronger model helps **pick the right work** and **see the WAIT story more clearly**. It does **not** invent residual-matched classic 1688 bytes, and it does **not** make `WAIT_CMD` succeed on the same lab image.

| Helps | Does not help |
|-------|----------------|
| Split “product Pass stamp” from “useful NPU work this week” | Magic Pass on lab classic 1688 |
| Rank WAIT EINVAL as out-fence / unpublished job, not timeout | Fix firmware by thinking harder |
| Name an honest wrap of IRON / eepy without laundering `ran_on_npu` | Reopen `2c9c12e8` or mint without a new reason |
| Stop empty same-shot reconfirm as a capability waste | Network-install FastFlowLM / Lemonade |

The previous loop kept treating “one more offline hygiene shot” as progress. That is why the residual is well instrumented and the laptop still has no product inference path. This plan uses the upgrade to **change the work**, not to re-spin the same wall.

---

## Context

### Two goals that must stay distinct

1. **Useful acceleration on this Ryzen NPU.** Real compute or inference on the device. Already proven for four public mlir-aie npu2 programs via IRON + XRT + system `pyxrt` on firmware **1.1.2.64**. Not an LLM chat stack yet.
2. **Product Pass** (`decide_ran_on_npu`: real ioctl + ERT Completed + OFM allclose). Still **No**. Residual class is still **`wait_cmd_einval`**.

IRON `PASS!` is **not** product Pass. eepy CPU smoke is **not** an NPU session. Same-shot `paddrs_valid=1` is **not** Pass.

### What still blocks product Pass

| Blocker | Who | Can we address it now? |
|---------|-----|------------------------|
| Residual-matched / load-accepted classic **1688** (or a completing-stack peel that actually loads and finishes) | **Human** (private stack / intercept) | Yes, by placing honest bytes under `testdata/xdna2_cmd/external_known_good/` with PROVENANCE. Public 300 / 660 / 2592 / 4512 and unpatched oracle `3c5ffca750922d3b` do **not** count. |
| Public DynamicDispatch live post-patch dump | Host / vendor | **No.** `get_instr_bo` is cache; staged family hits `INSTR_BO_LIMIT`; live `run_` is private. |
| WAIT_CMD EINVAL after EXEC returns a seq | Driver / firmware | **Not a pack bug.** Best match: driver cannot resolve the out-fence for `(hwctx, seq)`, or firmware never published a waitable fence. Timeout, `seq=0`, and ioctl size are ruled out. |
| Product still waits with WAIT_CMD; XRT often uses syncobj when the handle is non-zero | Agent (research wire exists) | Offline already. Live A/B **not** justified from that wire alone. |
| Same-shot address patch | Already landed | **Frozen.** Live under `2c9c12e8` proved it. Re-live is empty reconfirm. |
| Active unlock | None | **Do not mint** without a **new named reason**. |

### What already works on this machine (use this)

- IRON / mlir-aie npu2: vector add + three public matmuls, all IRON `PASS!` on this firmware, no dragon-npu L3 sandwich.
- `/dev/accel/accel0`, distro XRT, system `pyxrt`.
- eepy-min installed (VitisAI provider **listed**; smoke is **CPU only**; no proven VitisAI NPU session).
- dragon-npu CLI `compile` / `run` is **host or simulation**. Even with hardware detected, `Backend::run` stays the simulation arm.
- `xrt-bridge` is a refuse stub. Do not flip it on for inference.
- FastFlowLM and Lemonade are **absent**. No in-tree LLM NPU server.

Stock `amdxdna` has **no** `exec_cmd_allow` knob. IRON already submitted work through XRT without a dragon-npu unlock.

---

## Approach

**Do both, and keep the labels honest.**

**Useful-soon (default after Approve):** land an opt-in product backend that shells to a stack that already runs (IRON first, because NPU compute is proven; eepy second, because ONNX apps are the real inference path but NPU is unproven). CLI and JSON must say `execution=external_runner` and **`ran_on_npu=false`**. Never open `XDNA2_ALLOW_EXEC_CMD` from the wrapper. Never ingest IRON peels as product known-good.

**Pure-direct residual (unchanged law):** keep same-shot, post-fail ERT tokens, and syncobj research as landed science. Agent work here is only a **new named offline delta** (WAIT fence diagnostic write-up / unit surface, not another live). Next product live only under a **new** unlock **and** a **new** named reason (new residual-matched bytes, or a new last_apply class). Default is **not** mint.

**Use the NPU this week without waiting for Pass:** run IRON MobileNet / ResNet from the existing mlir-aie tree (S5 is **agent-try**). That is useful inference, not a lab stamp.

### Not this plan

- **Not** empty same-shot reconfirm under a new mint
- **Not** reopen `2c9c12e8` or any CONSUMED unlock
- **Not** claim product `ran_on_npu` from IRON, eepy, attach, or same-shot
- **Not** re-run the four public AOT peels as residual progress
- **Not** enable `xrt-bridge` as if it were a runtime
- **Not** agent network-install of FastFlowLM / Lemonade
- **Not** bump `timeout_ms` as the WAIT EINVAL fix
- **Not** treat public mlir-aie peels as residual identity

---

## Critical files

| Path | Why |
|------|-----|
| `src/backends/mod.rs` | `Backend` trait (`status` / `compile_model` / `run` / `submit_and_run`) |
| `src/backends/amd_xdna2.rs` | Real arm still fail-closed for model `run`; status can see hardware |
| `src/cli.rs` / `src/bin/dragon_npu.rs` | Today always `execution=host_or_simulation_run`, `ran_on_npu=false` |
| `src/backends/xrt_bridge.rs` | Refuse stub. Do **not** grow this into the wrapper. New module instead. |
| `docs/RYZEN_AI_EEPY_MIN.md` | eepy-min activate + honesty (CPU smoke ≠ NPU) |
| `docs/CAPTURE_MATERIAL_MLIR_AIE.md` | IRON peels RESEARCH ONLY |
| `docs/SECONDARY_BACKENDS.md` | Honest stubs; extend with external-runner honesty |
| `RESIDUAL.md` | Unlock SSoT; residual class; invent_pass |
| `src/backends/amd_xdna2/silicon_contract.rs` | Wait axes, material match, post-fail ERT. Soft always invent_pass=0 |
| `src/backends/amd_xdna2/direct.rs` | WAIT_CMD, syncobj retain, same-shot. No live from this plan alone |
| `tests/fixtures/host_matmul_relu.json` | Pattern for a tiny host-graph demo |
| `testdata/xdna2_cmd/external_known_good/` | Human landing for residual-matched bytes only |
| `/home/hunter/Projects/ai/mlir-aie` | Working IRON examples (sibling tree; read / run, do not edit unless asked) |
| `/home/hunter/Projects/ai/ryzen-ai-install/` | eepy-min venv (sibling; do not edit unless asked) |

## Reuse

| Piece | Path | How |
|-------|------|-----|
| `Backend` trait | `src/backends/mod.rs` | New opt-in impl. Do not replace `AmdXdna2Backend` default |
| CLI execution field | `src/cli.rs` | Add `external_runner`; keep `ran_on_npu=false` |
| Host graph | `evaluate_host_graph` | Tiny linear demo fallback when the wrapper is off or the child fails |
| IRON env recipe | working-stacks report | Document; subprocess must not invent peels as known-good |
| eepy activate | `docs/RYZEN_AI_EEPY_MIN.md` | Optional second runner after IRON wrapper exists |
| Post-fail ERT / wait axes | `silicon_contract.rs` | Cite in WAIT diagnostic; do not re-implement |
| Unlock parse | first `C_UNLOCK=` in `RESIDUAL.md` | Stay CONSUMED; no mint |

---

## Steps

Default order after Approve: **S0 → S1 ∥ S2 → S3 → S4**, then **S5 agent-try**.
S5 is **not** a product live unlock.

| Step | Who | What | Size |
|------|-----|------|------|
| **S0** | Agent | Residual + AGENTS honesty pin: dual track named; product Pass still No; unlock none (`2c9c12e8` CONSUMED); IRON/eepy ≠ `ran_on_npu`; freeze empty same-shot reconfirm; this plan is **not** a mint | 1 |
| **S1** | Agent | **Honest external-runner backend (TDD first).** New opt-in backend (feature or explicit construct, not default `DragonNpu::new`). Subprocess contract: parse IRON or eepy stdout; `execution=external_runner`; **refuse** `ran_on_npu=true` even if the child says PASS. Missing binary → honest unavailable, not a fake sim Pass. Prefer IRON first (NPU already proven). Do not open ALLOW / L3. Do not write into `xrt_bridge.rs`. | 2 |
| **S2** | Agent | **Host-graph tiny linear demo.** Pack a small MatMul+Relu+Add ONNX/JSON fixture + `dragon-npu compile` / `run` path that stays `ran_on_npu=false`. This is the CPU fallback and a real (host) inference surface while S1 is wired. | 2 |
| **S3** | Agent | **WAIT fence diagnostic (offline only).** Named write-up + any missing **pure** unit surface for “out-fence lookup fail vs unpublished job vs syncobj parity.” Cite staged driver `aie2_cmd_wait` / `xa_load`. Soft invent_pass=0. **Not** a live A/B. **Not** an unlock reason by itself. | 2 |
| **S4** | Agent | Join report + residual pin. List what shipped (wrapper / demo / diagnostic). Repeat: Pass=No; unlock none; no mint. | 1 |
| **S5** | **Agent-try** (operator expanded; not parked human-only) | **Use the NPU without product Pass:** try IRON `programming_examples/ml/mobilenet` or `resnet` from the existing mlir-aie tree; and/or one eepy VitisAI `InferenceSession` on the on-disk test ONNX. Record whether the EP actually used the NPU. Still **not** product `ran_on_npu`. Residual-matched 1688 drop stays the only human path that unblocks **product** live. | 2 |

**S1 and S2 may run in parallel** (disjoint files). S3 is independent of S1/S2 and may also run in parallel. S4 waits for those three. S5 is agent-try after (or alongside when it does not race product files).

**Live product apply is not in this plan.** If S5 or a later human drop produces residual-matched bytes, that is a **new plan** plus a **new** unlock, not a silent mint.

---

## Risks

| Risk | Mitigation |
|------|------------|
| Wrapper launders IRON PASS into `ran_on_npu` | Red test: child stdout contains PASS, product JSON still `ran_on_npu=false` and `execution=external_runner` |
| Growing `xrt-bridge` into a fake XRT link | New module; leave the refuse stub alone |
| Treating S3 as unlock justification | Join text: diagnostic ≠ live_justified |
| Editing sibling mlir-aie / ryzen-ai-install trees | Read and subprocess only unless the operator expands scope |
| S5 MobileNet fails and looks like residual regression | Separate evidence file; do not touch product last_apply / unlock |
| Scope creep into FastFlowLM install | Explicit non-goal; operator install only |
| Empty residual mill after S4 | Stop. Next agent work needs a new named delta or Approve of a new plan |

---

## Verification

### Honesty (every step)

- First `C_UNLOCK=` in `RESIDUAL.md` remains **`2c9c12e8` CONSUMED**
- Soft / join / CLI never set invent_pass or product `ran_on_npu` from a wrapper
- No `XDNA2_ALLOW_EXEC_CMD=1` in default tests

### S1 red → green

- New unit or example test: fixture child “PASS!” → product result `execution=external_runner`, `ran_on_npu=false`
- Missing runner binary → unavailable / explicit error, not silent host Pass
- `cargo test --lib` filter for the new module; `cargo fmt` + `clippy -D warnings` on touched packages

### S2 red → green

- Fixture graph evaluates on host (`host_ref` or CLI `run`)
- JSON still `ran_on_npu=false`
- Named test for the new fixture path

### S3

- Report under `.agents/reports/` with ranked hypotheses and code cites
- Any new parser/token: failing unit first, then same filter green
- Docs only is allowed if no new API is needed

### S4

- Residual Latest Turn names dual track, Pass=No, unlock none
- Workspace mirror of this plan under `.agents/plans/` after Approve (implementer, not this present)

### S5 (agent-try)

- IRON or eepy log on disk; say whether NPU ran
- Do not retag capture-dir peels as load-accepted
- Do not mint
- Still **not** product `ran_on_npu`

---

## Open questions

None blocking. Assumptions:

- IRON wrapper first, eepy second (NPU already proven on IRON).
- S5 is **agent-try** (operator expanded). It is not required to close S0–S4, and it is not parked as human-only.
- Product live apply stays out of this plan.

If you want **only** the residual stamp and no wrapper, say so on Revise. If you want the wrapper to default on in `DragonNpu::new`, say so; this plan keeps it **opt-in**.

---

## Who acts next (after Approve)

1. Agent: S0 pin (this slice), then S1 ∥ S2 ∥ S3, then S4 join.
2. Agent-try S5: IRON MobileNet / ResNet and/or one eepy VitisAI session. Human still owns residual-matched bytes under `testdata/xdna2_cmd/external_known_good/` when they hold them.
3. Agent does **not** mint. Agent does **not** re-live `2c9c12e8`.

### Critical Files for Implementation
- `src/backends/mod.rs`: Backend trait for the honest wrapper
- `src/cli.rs`: `execution=external_runner`, never `ran_on_npu` from wrap
- `RESIDUAL.md`: honesty + unlock SSoT
- `src/backends/amd_xdna2/silicon_contract.rs`: WAIT diagnostic only
- `docs/RYZEN_AI_EEPY_MIN.md`: eepy honesty
- `tests/fixtures/host_matmul_relu.json`: tiny host demo pattern
