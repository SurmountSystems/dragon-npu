# Product Pass vs already done (2026-08-12)

invent_pass=0. Pass=**No**. `ran_on_npu` not claimed. This is not a mint.

## 1. Residual and unlock

- Residual class: still **`wait_cmd_einval`** (EXEC returns a seq; WAIT_CMD EINVAL).
- Unlock: **none**. First SSoT line: `C_UNLOCK=2c9c12e8` CONSUMED (same-shot Arm A; last_apply still wait_cmd_einval; `ran_on_npu=false`).
- `live_justified=0`. S3 ranked **OutFenceLookupFail** first. Diagnostic is not an unlock reason.

## 2. Useful-soon already shipped (not Pass)

- Opt-in `dragon-npu --external-runner` wrapping IRON/mlir-aie. Child `PASS!` is informational only; never product `ran_on_npu`. No product unlock needed.
- Host linear demo (MatMul+Relu+Add). Host only.
- S5: NPU already ran this week via IRON eltwise add / relu / softmax. eepy still CPU. Do **not** ask to re-run npu-iron to prove that again.

## 3. What still blocks product Pass

- Missing **residual-matched / load-accepted classic 1688** (completing stack that loads, ERT Completed, OFM allclose). Public peels 300/660/2592/4512 and unpatched oracle do **not** count.
- After EXEC, product wait still hits **WAIT_CMD EINVAL**. Same-shot paddrs already proven. Live wait A/B **not** justified from diagnostics alone.
- Human holds that material. Agent cannot invent it.

## 4. Frozen / banned

Do not mint. Do not reopen CONSUMED (`2c9c12e8` or priors). Do not invent Pass or `ran_on_npu`. Do not empty same-shot reconfirm. No sticky ALLOW. Wrapper / host demo / IRON `PASS!` / eepy CPU / wait diagnostic ≠ Pass ≠ unlock.

## 5. Agent-legal next (product Pass track only)

**None without human material.** No named offline delta on this track is waiting. Stop until residual-matched / load-accepted classic 1688 lands under `testdata/xdna2_cmd/external_known_good/` (with PROVENANCE), or the operator names a new offline code change. Operator may keep using `--external-runner` without an unlock.

Sources: `RESIDUAL.md` top; `AGENTS.md` Latest Turn; `.agents/reports/impl-useful-accel-s4-join.md`; `.agents/plans/plan-useful-npu-accel.md`.
