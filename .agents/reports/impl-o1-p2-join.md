# O1 P2 join — option1-after-same-shot (offline closeout)

**Date:** 2026-08-11
**Plan:** `.agents/plans/plan-option1-after-same-shot.md` step **P2**
**Mode:** docs/residual join only. No live NPU. No unlock mint. No L3 / ALLOW.

---

## Decision

| Field | Value |
|-------|--------|
| **live_justified** | **no** |
| invent_pass | **0** |
| Pass | **No** |
| ran_on_npu | **false** / not claimed |
| unlock | **none** (first `C_UNLOCK=` = **`2c9c12e8` CONSUMED** — not reopened, not minted) |
| P3 / P4 live | **not run** (not justified) |

### Join rule (from plan)

Live only if **material landed** **or** a **new wait theory** is measurable and named.
Both arms report **no**:

| Arm | Report | Verdict |
|-----|--------|---------|
| **P1-A** Bridge A / material | `.agents/reports/impl-o1-p1a-bridge-a-material.md` | `live_justified_from_material=**no**` |
| **P1-B** wait-path offline | `.agents/reports/impl-o1-p1b-wait-path.md` | `live_justified_from_wait_theory=**no**` |

Therefore: **explicit stop** with the proven wall once. Do **not** mint. Do **not** run P3/P4.

---

## Named reason for stop (plain English)

Same-shot product live under consumed unlock `2c9c12e8` already proved the hygiene path (`same_shot=1`, in-process paddrs) and still ended **`wait_cmd_einval`** with **no** Pass. Offline follow-up did **not** land residual-matched or load-accepted instruction material, and wait-path work only improved **observability** (soft tokens / research notes), not a measurable root fix for WAIT_CMD EINVAL. Burning another unlock on the same Arm A residual would be empty same-shot reconfirm. Stop until new material or a named, measurable wait theory exists.

---

## Wall (still primary)

1. **Residual-matched / load-accepted completing-stack material** still missing for lab classic **1688** (not public mlir-aie AOT peels, not multi 32208, not unpatched oracle `3c5ffca750922d3b` as “post-patch”).
2. Wait-path work = **observability only** (detail greps, soft classify, env-gated research wire). Product default remains WAIT_CMD. Syncobj research alone ≠ residual root ≠ unlock.

Pass still requires product `decide_ran_on_npu`: real_ioctl + ERT **Completed** + OFM allclose.

---

## What closed offline (P0–P2)

| Step | Outcome |
|------|---------|
| **P0** honesty pin | Same-shot proven under `2c9c12e8` CONSUMED; freeze empty reconfirm; unlock none |
| **P1-A** material | Host scan + bag/oracle inventory; residual_matched=0; load_accepted=0; no new bag ingest |
| **P1-B** wait-path | Soft rich tokens + wait_path_syncobj research note; invent_pass=0; no product wait default change |
| **P2** join | **live_justified=no**; stop; no mint |

Same-shot live science (prior S4–S5) remains closed evidence — **not** re-run this join.

---

## Unlock SSoT

```
C_UNLOCK=2c9c12e8 CONSUMED
```

- **Do not reopen** `2c9c12e8`.
- **Do not mint** from this join.
- Prior CONSUMED list unchanged (`4008e6fd`, `48da6e60`, `1f761bf4`, …).

---

## Who acts next

| Actor | Action |
|-------|--------|
| **Human** | Load-accepted peel / private completing-stack material into `testdata/xdna2_cmd/external_known_good/` with honest PROVENANCE, **or** expand scope (sudo L3 instrumented capture / private stack). |
| **Agent** | Only on a **new named offline delta** (real residual-matched bytes, or a measurable wait theory with new last_apply class). **Freeze** empty same-shot reconfirm and empty implement re-spins. |

---

## Explicit non-claims

- No product Pass
- invent_pass=0
- No `ran_on_npu`
- Same-shot ≠ Pass
- Soft wait_path / syncobj research ≠ unlock
- Public AOT / IRON PASS ≠ residual identity
- No sticky ALLOW

---

## Inputs used (not re-researched)

- `.agents/reports/impl-o1-p0-honesty-pin.md`
- `.agents/reports/impl-o1-p1a-bridge-a-material.md`
- `.agents/reports/impl-o1-p1b-wait-path.md`
- `.agents/plans/plan-option1-after-same-shot.md`
- `RESIDUAL.md` top (pre-join Latest Turn was P0)

## Plan complete?

**Offline vertical P0–P2 complete.** P3–P4 remain plan steps but are **gated off** until a future join can set `live_justified=yes` with new evidence.
