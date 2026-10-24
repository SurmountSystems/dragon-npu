# useful wave 2 + second identity — wrap restore + review-fix

**Named offline delta:** Review-fix for catalog wave 2 + second identity
`iron_npu2_vector_add`. Default wrap restored. Export is opt-in. Not
product Pass. No mint.

## Wrap restore (Issue 1)

Sibling `vector_scalar_add.py --aot-dir` writes artifacts and returns
without `PASS!`. Unconditional `--aot-dir` on `just npu-iron` broke the
useful wrap.

- Default product argv for `vector_add` is
  `["vector_scalar_add.py", "--dev", "npu2"]` (no `--aot-dir`).
- `iron_kernel_cwd_and_argv_with_aot(..., None)` does not append
  `--aot-dir`.
- `just npu-iron` uses `dragon-npu run --iron vector_add` (run+verify).
- Opt-in export: `just npu-iron-export` /
  `dragon-npu run --iron vector_add --iron-export`. Creates the exclusive
  this-run directory and passes `--aot-dir` only then. No child `PASS!`
  expected on that path. The export directory is kept so the second-identity
  dump is not deleted.

## Issue fixes (2–11)

2. Kind-match `iron_npu2_vector_add` only at exact **300 B**. 660 B is
   `kind_match_for_job=false`. Capture helper uses the same band.
3. One allow-list: `is_iron_npu2_vector_add_job_name` (case-insensitive,
   hyphen). Classify, refuse, and capture share it. Empty job refuse is
   false.
4. `format_iron_job_collect_note(..., true)` exact-locks
   `residual_matched_claim=0 invent_pass=0 not_ran_on_npu=1` and the
   listed honesty tokens. Empty/whitespace job still honest.
5. `iron_job_neq_classic_1688=1` only for `IronNpu2VectorAdd`.
6. IRON job + 1688 remaps to `Unknown`. Format note stays claim 0 even
   if the struct field is flipped.
7. Only `vector_add` accepts `--aot-dir` (catalog loop; gelu + Some(aot)
   still has no `--aot-dir`).
8. Missing aot dir / empty npy → None / empty listing. Valid `out.npy`
   after empty `output.npy` still loads.
9. `tools/test_capture_iron_npu2_vector_add_fail_closed.sh`: temp 300 /
   660 / 1688, hyphen alias. Exit 2. Honesty tokens. No copy into
   `testdata/xdna2_cmd/external_known_good/`.
10. Help lead lists `npu-swiglu` and `npu-scale-shift`.
11. This-run aot dir is created only on explicit export.

## Prior wave-2 land (still true)

- Wave 2 names: `gelu`, `silu`, `eltwise_mul`, `swiglu`, `scale_shift`.
- Recipes: `just npu-gelu` / `npu-silu` / `npu-mul` / `npu-swiglu` /
  `npu-scale-shift`. Unknown name fails closed.
- Second identity is not classic 1688. Public 300 B peel may kind-match
  the IRON job. Never 1688 load-accepted.
- Sibling `/home/hunter/Projects/ai/mlir-aie` was **not** edited.

## Agent-run (this host; not product Pass)

Prior wave-2 agent-run of wave-2 recipes: `child_said_pass=true`,
`ran_on_npu=false`. Prior `just npu-iron` used unconditional `--aot-dir`
and printed `child_said_pass=false` (export-only). This review-fix
restores the wrap in product argv and recipes. Unit tests lock the wrap.
The live wrap was not re-run this slice.

## Honesty

- invent_pass=0
- product Pass = **No**
- `ran_on_npu` = false (wrap / child `PASS!` is `execution=external_runner`)
- unlock **none** (`C_UNLOCK=2c9c12e8` CONSUMED)
- Do not mint. Do not reopen CONSUMED. No sticky ALLOW.

## Who acts next

Human residual-matched classic 1688 when held. Agent only on a **new
named offline delta**. Do **not** ask the operator to re-run
`just npu-iron`. Agent does **not** mint.

## Reports

`/tmp/grok-1000/grok-impl-summary-5dca83c3.md`

invent_pass=0 product_pass=0 ran_on_npu=0 unlock=none C_UNLOCK=2c9c12e8 CONSUMED

## Closeout (implement loop 5dca83c3 CLOSED)

Implement loop CLOSED. Effort 3. Review rounds 2. Re-review r2: **0 open**
(general + tests + plan). Merged review:
`/tmp/grok-1000/grok-review-5dca83c3.md`. Named delta already landed
(catalog wave 2, `iron_npu2_vector_add`, wrap restore). Product Pass is
still **No**. `ran_on_npu` is still false. Unlock remains **none**
(`C_UNLOCK=2c9c12e8` CONSUMED). Catalog / wrap / export collect are not
Pass and are not an unlock. Do not mint. Do not reopen CONSUMED. Do not
ask the operator to re-run `just npu-iron`. Next is a new named offline
delta or a human residual-matched classic 1688 when they actually hold
it. Do not idle only on a human 1688 file.
