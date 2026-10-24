# Explore: imaginative next (not idle on human 1688)

**Date:** 2026-08-13. invent_pass=0. Product Pass=**No**. Unlock **none** (`2c9c12e8` CONSUMED). This note is not a mint.

Operator pin: they cannot magically produce residual-matched / load-accepted classic 1688. Do **not** make idle-until-human-1688 the primary next. Do **not** invent Pass or `ran_on_npu`. Do **not** mint a live unlock without a new named reason. Do **not** retag public AOT peels (300 / 660 / 2592 / 4512) as load-accepted for the lab 1688 job.

## Constraints (from residual + code)

- Residual wall for the **current** product Pass story is still classic 1688. That wall is real. It is not the only useful move.
- Named IRON catalog already landed: `vector_add`, `eltwise`, `relu`, `softmax` in `src/backends/iron_kernel.rs`. `just npu-relu` and friends wrap mlir-aie. Child `PASS!` is still `ran_on_npu=false`.
- `decide_ran_on_npu` in `src/backends/amd_xdna2/numerical.rs` is **not** hardcoded to 1688. It is ioctl + instruction blob + submit/wait + non-empty non-zero golden + OFM allclose. The 1688 lock lives in residual / capture identity, not in that function.
- `docs/SECONDARY_BACKENDS.md`: XRT bridge is research refuse, never the product backend. IRON is opt-in wrap only. There is no "switch product job to IRON peel" plan in that file.
- Capture helpers: `tools/host_capture_scan.sh` walks the host for ctrltext / insts / xclbin / pdi / elf and prints a scorecard (no copy). `tools/capture_from_host_candidate.sh` fail-closes unless completing-stack evidence is explicit; size≠1688 refuses residual match for the **current** 1688 identity.
- Sibling mlir-aie examples are mostly sources. After `just npu-iron`, example dirs do not grow collectable `build/` or `.xclbin` (prior explore `.agents/reports/plan-explore-next-useful.md`). JIT cache is elsewhere. `vector_scalar_add.py` **does** support `--aot-dir` / `--xclbin-path` / `--from-xclbin` if we pass those flags. Do not edit the sibling tree.

## Sibling examples that look runnable without torch

Under `/home/hunter/Projects/ai/mlir-aie/programming_examples/` (numpy + IRON, not MobileNet/ResNet):

- Already cataloged: `basic/vector_scalar_add`, `ml/eltwise`, `ml/eltwise_unary` (relu/silu/gelu), `ml/softmax`.
- Next primitives: `ml/swiglu`, `ml/norm` (rms/layer), `ml/rope`, `ml/scale_shift`, `ml/eltwise` `--op mul`.
- Small basic: `memcpy`, `passthrough_kernel`, `vector_scalar_mul`, `vector_vector_add`, `vector_vector_mul`, `matrix_scalar_add`, `row_wise_bias_add`.
- Matmul without torch: `getting_started/03_matrix_multiplication_single_core` and `basic/matrix_multiplication/single_core` (i16). Public 660 B peel already exists as RESEARCH ONLY.
- Skip for this week: `ml/mobilenet`, `ml/resnet`, `ml/bottleneck`, `ml/conv2d*` (heavier / i8 / torch-adjacent).

Dump: only some designs document `--aot-dir` / `--xclbin-path` (vector_scalar_add is the clear one). Live JIT `PASS!` still leaves the example dir empty unless those flags are passed.

---

## Ranked imaginative avenues

### 1. IRON catalog wave 2 plus this-run tensors (useful wrap)

**Agent would:** Add no-torch names to `iron_kernel.rs` and `justfile`: `gelu` / `silu` (same `eltwise_unary.py --op`), `eltwise_mul`, `swiglu`, `scale_shift`, maybe `norm`. For designs that already accept `--aot-dir` or compile-args, point this-run artifacts at a temp dir so `parse_child_f32_vector` can return a real vector when the child writes npy or a JSON array. Do not edit sibling mlir-aie. Tests: unknown name still fails closed; honesty tokens stay `ran_on_npu=false`.

**Why no human 1688:** Uses examples that already run on this firmware (S5 already `PASS!` on relu/eltwise/softmax/vector add). New names are more of the same wrap.

**Honesty:** wrap. Still-not-Pass. Useful NPU inference primitives in the product CLI.

**Biggest risk:** Many JIT examples still print `PASS!` and no npy (catalog review-fix already treats that as empty vec + `child_said_pass`). Without `--aot-dir` support on that example, output stays empty. Prior join: example dirs are sources only.

### 2. Host graph plus IRON activations (first useful "model")

**Agent would:** Keep MatMul/Gemm on `evaluate_host_graph`. Dispatch Relu / Add / Softmax nodes to the named IRON catalog when `--iron-graph` (new flag) is set. Fixture: small linear then relu, input `[-1, 2]`, host would give `[0, 2]` if CPU-only; NPU relu is the offload. JSON still says which slices were `execution=external_runner` vs `execution=host`. Never set `ran_on_npu`.

**Why no human 1688:** Host math plus IRON children that already pass. No classic txn required.

**Honesty:** wrap for NPU slices, host for matmul. Still-not-Pass. This is the first product-shaped inference path that actually uses the Ryzen NPU.

**Biggest risk:** IRON kernels are bf16 and large default lengths (eltwise 65536). Host fixtures are tiny f32. Size and dtype mismatch can make the graph a toy or a compile-time sink. Child still may return no OFM numbers.

### 3. New Pass identity: IRON npu2 job, not mladf 1688

**Agent would:** Name a second product job in residual + contract notes, for example `iron_npu2_vector_add` or `iron_npu2_relu`. Document that classic 1688 remains a **separate** frozen identity. Parameterize residual-match helpers so "match" means that job's size class and peel, not 1688. `decide_ran_on_npu` already accepts any golden/OFM. Add a lab pack / golden for the IRON shape (vector add 1024, or relu length), not AMD a16w4 174080 WTS. Public 300 / 660 peels stay RESEARCH for 1688; they may be the **right** bytes for this new identity after a completed IRON run, still not "1688 load-accepted."

**Why no human 1688:** The Pass question becomes "did product submit the IRON job and allclose," not "did we get DD classic 1688."

**Honesty:** new Pass identity. Still-not-Pass until product-owned real ioctl + ERT Completed + OFM allclose. IRON child `PASS!` must not be laundered.

**Biggest risk:** Product direct EXEC of public peels already died as `wait_cmd_einval` (same-shot `2c9c12e8`, skip-CU `1f761bf4`). INSTR_BO_LIMIT blocks the DynamicDispatch post-patch 1688 path. IRON Python/XRT already runs; the product ioctl path may still EINVAL. This identity is honest only if we keep wrap and product-decide as two columns.

### 4. Named AOT collect for the IRON job (not 1688 ingest)

**Agent would:** Wire catalog argv to `--aot-dir` (vector_scalar_add first) into a this-run directory under `/tmp` or gitignored scratch. Ingest helper copies xclbin / insts / pdi / elf with PROVENANCE `job=iron_npu2_vector_add` and `completed_on_this_fw=` only when the child actually ran. Refuse `residual_matched_claim` against classic 1688. Offline classify size class as `public-npu2-small` or a new `iron-npu2-job` tag.

**Why no human 1688:** Collects the job we can already run. Does not wait for DD classic bytes.

**Honesty:** research / new-identity material. Not 1688 load-accepted. Not Pass. Not an unlock by itself.

**Biggest risk:** Re-running IRON to dump AOT is live NPU (this explore must not run it). AOT-only without a completed run is the W5 trap (300 / 660 RESEARCH). JIT cache is not in the example dir unless `--aot-dir` is used.

### 5. IRON single-core matmul as a useful matmul (not mladf)

**Agent would:** Add catalog name `matmul_single` pointing at `getting_started/03_matrix_multiplication_single_core` or `basic/matrix_multiplication/single_core` with `--dev npu2`. Optional later: treat that job as the Pass identity in avenue 3.

**Why no human 1688:** mlir-aie already has a public i16 matmul. W5 Path B already `PASS!` on the public single_core peel via IRON (research, residual_matched_claim=0 for 1688).

**Honesty:** wrap unless we also adopt avenue 3. Still-not-Pass. Useful NPU matmul without the AMD packed a16w4 layout.

**Biggest risk:** First compile can be long. 660 B peel is the wrong size for lab 1688. Do not retag it. Same `wait_cmd_einval` if we later try product EXEC of that peel.

### 6. Default CLI host graph (CPU useful, not NPU)

**Agent would:** Make `dragon-npu run` without flags use `evaluate_host_graph` on a small fixture instead of simulation tanh. Already works as `--host-graph` (`[-1, 2]` → `[0.5, 3.5]`).

**Why no human 1688:** Pure host.

**Honesty:** host only. Still-not-Pass. Useful as a product demo, not Ryzen inference.

**Biggest risk:** Does not advance NPU. Easy to confuse with Pass in chat. Low leverage next to 1–3.

### 7. XRT as product backend (do not pick)

`xrt-bridge` is an honest refuse stub. SECONDARY_BACKENDS says production is pure direct `amd-xdna2`. Promoting XRT would fight the fork reason and still would not mint 1688.

**Honesty:** dead end for this week.

---

## Default path for the revised plan

**Do avenues 1 and 3 together, in that order, as one plan.**

1. **This week, no mint:** Catalog wave 2 (gelu, silu, eltwise_mul, swiglu, scale_shift) plus this-run artifact/npy flags where the example already accepts them, so `dragon-npu run --iron NAME` can return numbers. Useful Ryzen NPU primitives without 1688.
2. **Same plan, docs + TDD, no mint:** Pin a second Pass identity `iron_npu2_relu` (or vector_add) as a **different job** from mladf classic 1688. Residual match and capture helpers learn the new name. `decide_ran_on_npu` stays generic. Public peels are not retagged as 1688 load-accepted.
3. **Mint only if** the identity work produces a **new named reason** (product-owned submit of the IRON job, not "child said PASS"). Do not reopen `2c9c12e8`.

**Alternates:** (A) Host graph + IRON relu offload if the operator wants a graph-shaped demo first. (B) Catalog `matmul_single` if they want NPU matmul more than more activations.

Do not idle on human 1688. Do not ask the operator to re-run `just npu-iron`. Do not implement from this note.

```
invent_pass=0 product_pass=0 ran_on_npu=0 unlock=none C_UNLOCK=2c9c12e8 CONSUMED
```
