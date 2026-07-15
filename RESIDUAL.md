# Residual — clippy --all-targets + extract sha256/force refuse

**Date:** 2026-07-15  
**Honesty:** Silicon GEMM Pass still HITL (known-good AIE2/npu4 txn + sudo L3).

---

## Completed this round (clippy all-targets + extract)

| Issue | Fix |
|-------|-----|
| **clippy --all-targets** | match_same_arms, too_many_lines allow, cast_signed, redundant_clone, float_cmp, field_reassign_with_default |
| **extract sha256** | `sources.sha256` uses `sources/name` so `sha256sum -c` works from OUT |
| **extract force VE2** | Refuse **before** mkdir/copy (no partial stage on exit 4) |

## Prior (still true)

Full-alloc zero; ofm_decide_error; LAB_WEIGHT MVP; synthetic extract CI.

## LAB_WEIGHT MVP limits (intentional)

- f32 LE WTS (promoted host view)
- ones IFM golden only (`ones @ W`)
- plain MatMul/Gemm only — no `fused_*`, no Gemm alpha/beta ≠ 1, no bias BO
- Full alloc zeroing for page tails

## Still HITL

1. Known-good npu4/17f0 txn blob  
2. Sudo L3 single-shot lab apply  
3. True silicon Pass (OFM allclose on Real)

---

## Gates

```bash
export PATH=/usr/bin:$PATH
unset XDNA2_ALLOW_EXEC_CMD XDNA2_LAB_OFM XDNA2_LAB_WEIGHT
cargo fmt -- --check
cargo clippy -- -D warnings
cargo clippy --features amd-xdna2 -- -D warnings
cargo clippy --features amd-xdna2,xdna2-dangerous-exec-cmd -- -D warnings
cargo test --lib
cargo test --features amd-xdna2 --lib
cargo test --features amd-xdna2,xdna2-dangerous-exec-cmd --lib
cargo check --examples --features amd-xdna2
bash scripts/lab_exec_once.sh
bash scripts/extract_instruction_offline.sh
bash -n scripts/*.sh
```

Latest local: **112** default; **337+1 ignored** amd-xdna2; synth extract OK; lab dry-run 0.
