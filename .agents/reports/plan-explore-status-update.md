# Status update: useful NPU commands vs product Pass

**Date:** 2026-08-13. invent_pass=0. Product Pass=**No**. Unlock **none**. This note is not a mint and not a new campaign.

Sources: `RESIDUAL.md` top (first `C_UNLOCK=` = `2c9c12e8` CONSUMED); `.agents/reports/impl-useful-commands-catalog.md`; plans `.agents/plans/plan-useful-npu-commands.md` and `.agents/plans/plan-useful-npu-accel.md`; prior explores `.agents/reports/plan-explore-next-residual.md` and `plan-explore-next-useful.md`; product `src/cli.rs`, `justfile`, `src/backends/iron_kernel.rs`, `docs/SECONDARY_BACKENDS.md`.

The useful-npu-commands plan (steps 1–6) and the later review-fix (0-open) **landed**. There is **no unused catalog wrap** still waiting in that plan. Prefer record closeout and idle on product Pass.

---

## Landed (facts)

Useful-soon wrap and the named IRON catalog are in the tree. They are not product Pass.

- **Named IRON catalog** in `src/backends/iron_kernel.rs`: `vector_add`, `eltwise`, `relu`, `softmax`. Each name maps to an existing mlir-aie example cwd plus python argv. Unknown or empty name fails closed. Resolve matches `tools/npu_iron_env.sh` (no `$MLIR_AIE` required). Spawn is argv-only (`Command::new`), never `sh -c`.
- **Just recipes** in `justfile`: `npu-iron` (vector add), `npu-eltwise`, `npu-relu`, `npu-softmax`, shared `_npu-iron-named`, optional `npu-iron-paths`. Recipes print honesty then run `dragon-npu run --iron NAME`.
- **CLI:** `dragon-npu run --iron NAME` fills cwd and argv from the table. `--host-graph` calls `evaluate_host_graph` on `tests/fixtures/host_matmul_relu_add.json` (default input `[-1, 2]` → `[0.5, 3.5]`, `execution=host`). Needs `--features amd-xdna2`. Default `run` without those flags is still simulation tanh. `--iron`, `--host-graph`, and raw `--external-runner` are mutually exclusive. Help lists the four names.
- **Review-fix (0-open, residual Latest Turn):** `--iron` does not compile a dummy host fixture. Leftover `output.npy` counts only when this run mentions the basename. Stdout `shape [256]` is not OFM. Host fixture resolves from the crate root. Shell suffix maps were removed; Rust `--iron` is the only name table.
- **Numbers when the child emits them:** `parse_child_f32_vector` prefers a valid little-endian `<f4` npy, else the first finite JSON array in stdout. Missing numbers stay an empty `Vec<f32>` plus `child_said_pass`. Live IRON `PASS!` alone is `output_len=0`. Never invent OFM.
- **Honesty tokens (hard-coded):** `execution=external_runner` or `execution=host`; **`ran_on_npu=false` even on child `PASS!`**; `invent_pass=0`; IRON peels are not product known-good. Docs in `docs/SECONDARY_BACKENDS.md` and CLI help lead with `just npu-relu` / `--iron`.
- **Earlier useful-npu-accel (S0–S5 JOINED):** opt-in `--external-runner`; host linear demo; wait-fence diagnostic with `live_justified=0`; S5 recorded IRON eltwise / relu / softmax on firmware 1.1.2.64 this week; eepy still CPU. That wrap is also not Pass.
- **Agent verify claimed green (join report, operator idle):** `just npu-relu`, `npu-eltwise`, `npu-softmax` exit 0 with `child_said_pass=true`, `ran_on_npu=false`, `invent_pass=0`, `output_len=0`. `dragon-npu run --host-graph --json` exit 0 with `execution=host`, output `[0.5, 3.5]`, `ran_on_npu=false`. Sibling mlir-aie was not edited. Public AOT peels were not retagged. `XDNA2_ALLOW_EXEC_CMD` was not opened.

---

## Still blocked (Pass track)

Product Pass (`decide_ran_on_npu`: real ioctl + ERT Completed + OFM allclose) is still **No**. Residual class is still **`wait_cmd_einval`**. Same-shot live under `2c9c12e8` already ran and is **CONSUMED**. Unlock is **none**. `live_justified=0`. Diagnostics and IRON `PASS!` are not an unlock reason.

The Pass wall is still **residual-matched / load-accepted classic 1688** under `testdata/xdna2_cmd/external_known_good/` with PROVENANCE. Public peels 300 / 660 / 2592 / 4512 and the unpatched oracle do not count. The human holds that material. The agent cannot invent it.

**Who acts next on Pass:** the human, when they hold residual-matched classic 1688. The agent only on a **new named offline delta**. The agent does not mint.

**What must not happen:** do not mint. Do not reopen CONSUMED `2c9c12e8` (or any prior CONSUMED unlock). Do not ask the operator to re-run `just npu-iron` (already proven this week). Do not invent Pass or `ran_on_npu`. Do not empty same-shot reconfirm. No sticky ALLOW. Catalog, wrapper, host graph, and IRON `PASS!` are not Pass and not unlock.

---

## Honest next (ranked)

1. **Idle on product Pass until the human places residual-matched classic 1688.** That is the only Pass move. Record this closeout. Stop the useful-commands implement loop.

2. **Optional useful-soon wrap only if the operator names a new product slice.** There is **no leftover named gap** in the four-kernel catalog. Parse honesty (this-run npy, skip `shape [256]`, no dummy compile) already landed in the review-fix. Adding MobileNet / ResNet is not an unused leftover; it needs torch and a long compile and was rejected as this slice. eepy is still CPU-only and is not a new catalog name. Better tensor parse is not waiting: live IRON still prints `PASS!` without a this-run vector, and empty output plus `child_said_pass` is the honest contract. A later wrap would be a **new named plan** (for example more kernels the operator actually wants), still `ran_on_npu=false`, still not Pass.

3. **Not a next step:** empty reconfirm of green gates; same-shot re-live; reopen `2c9c12e8`; invent Pass; retag public AOT peels; ask the operator to re-run `just npu-iron`; mint a new unlock from the catalog land; start a new stamp mill or campaign.

---

## Suggested plan shape

Two phases. Prefer record closeout and idle. Do not implement from this note.

**Phase 1. Record closeout (docs only).** Update the useful-npu-commands plan header to say steps 1–6 and the review-fix landed, Pass still No, unlock none. Point at this report and the catalog join. Do not mint. Do not reopen CONSUMED.

**Phase 2. Idle on Pass.** Wait for residual-matched classic 1688 (human) or a new named offline ask. Operator may keep using `just npu-relu` / `--iron` / `--host-graph` without an unlock. Those commands stay useful wrap, not Pass.

Optional later (not this closeout, only if the operator names it): a new useful wrap plan for extra kernels or another stack. That would still be wrap-only and still not Pass.

```
invent_pass=0 product_pass=0 ran_on_npu=0 unlock=none C_UNLOCK=2c9c12e8 CONSUMED
```
