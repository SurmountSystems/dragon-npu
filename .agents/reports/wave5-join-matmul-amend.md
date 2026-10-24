# Wave5 join amend — matmul Path B closed

**Date:** 2026-08-11
**Kind:** Docs/residual pin only. No unlock mint. No invent Pass.
**Honesty:** invent_pass=**0** · Pass=**No** · unlock=**none** · first
`C_UNLOCK=` = **`4008e6fd` CONSUMED** · load_accepted_claim=**0** ·
residual_matched_claim=**0** (lab classic 1688)

---

## Why this amend

First `wave5-join.md` pin had matmul Path B as **pending**. Matmul runs landed
same day (report: `wave5-path-b-matmul-attempt.md`). Join + RESIDUAL + AGENTS
Latest Turn updated so status is not “vector only.”

---

## Results (FW 1.1.2.64, IRON, system pyxrt)

| Peel | Size | sha16 | IRON | completed_on_this_fw | collect class |
|------|------|-------|------|----------------------|---------------|
| vector_scalar_add (prior) | 300 B | `aaeb5b68cc298a45` | PASS! | yes | research_duplicate_sha16 |
| single_core matmul | 660 B | `de8671245bb72436` | PASS! | yes | research_duplicate_sha16 |
| whole_array matmul | 2592 B | `7ee4b267ce31cd70` | PASS! | yes | research_duplicate_sha16 |
| cascade matmul | 4512 B | `18cb653c05a9d4c8` | PASS! | yes | research_duplicate_sha16 |

- **New binary identity:** no (bag peels unchanged; re-tag refused).
- **Product Pass / `ran_on_npu`:** No / false.
- **Unlock:** none; do not mint.

---

## Files touched this amend

| Path | Change |
|------|--------|
| `.agents/reports/wave5-join.md` | Matmul closed; all four peels table; next = non-public residual identity |
| `RESIDUAL.md` | Latest Turn = vector + three matmuls completed_on_this_fw |
| `AGENTS.md` | Latest Turn line aligned |
| `.agents/reports/wave5-join-matmul-amend.md` | this note |

Detail evidence remains in `wave5-path-b-matmul-attempt.md` (not re-run here).

---

## Next residual-useful path (not public AOT re-run)

1. Non-public DD/XRT **post-patch** ctrltext for matmul-bias lab path, **or**
2. Intercept peels from a completing stack that **matches residual identity**
   (lab classic 1688 / working stack).
3. Do **not** re-run public mlir-aie AOT 300/660/2592/4512 for residual progress.
4. L1 only after residual match + **new** host-minted unlock + named reason.

---

## Footer

```
invent_pass=0
not_c_ready=1
not_ran_on_npu=1
Pass=No
load_accepted_claim=0
residual_matched_claim=0
completed_on_this_fw=yes   # public AOT 300/660/2592/4512 IRON on 1.1.2.64
unlock=none
C_UNLOCK=4008e6fd CONSUMED
```
