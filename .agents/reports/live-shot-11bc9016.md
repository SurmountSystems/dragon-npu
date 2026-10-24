# Live shot report: `C_UNLOCK=11bc9016` stock CONFIG_CU (wait_timeout)

**Date:** 2026-08-10
**Honesty:** `invent_pass=0` · **no** product Pass · **no** `ran_on_npu` · unlock **CONSUMED**

---

## Plain English: what ran

One stock live CONFIG_CU shot under unlock `11bc9016` finished with a structured last_apply (unlike the prior `3a70fabf` outer-kill with no pin).

| Item | Value |
|------|--------|
| Unlock | `C_UNLOCK=11bc9016` (one-shot; now **CONSUMED**) |
| Command | `bash tools/operator_app_load_kit.sh config-cu-run` (or `hitl_config_cu_once run-stock`) |
| Log | `/tmp/dragon_npu_live_shot_11bc9016.log` |
| last_apply | `/tmp/dragon_npu_last_apply_config_cu_1374078.json` (2208 B) |
| status | `wait_timeout` |
| `ran_on_npu` | `false` |
| `real_ioctl` | `true` |
| hitl_c | `hitl_c_rc=4`; `RUN_STOCK_DONE` |
| Wall time | `total_s` ≈ 62.87 |

### Measured detail tokens (from last_apply)

- `config_cu=1` (CONFIG_CU attach measured)
- `pdi_bytes=283840`
- `cu_func=0`
- `aiebu_elf=1`
- `rela_count=12`
- WAIT_CMD failed: `timeout_ms=5000` then **Invalid argument (os error 22)**

CONFIG_CU attach and aiebu ELF+rela userspace path **did** run. The job did **not** complete as ERT Completed with non-zero OFM. Product Pass remains **false**.

---

## Journal (this boot)

**Window:** 2026-08-10 ~11:34–11:35

- **IO_PAGE_FAULT** at addresses in the `0x2fff8000…` range
- **`aie2_set_cmd_timeout`** (DPU PC=`0xffffffff`, TXN OP ID=`0xd`, Context ID=15)

**Prior similar journal:** 2026-08-09 under unlock `3a70fabf` (that shot was outer-killed before last_apply).

---

## Residual class for this pin

**Class:** wait_timeout / FW timeout / IO_PAGE_FAULT.

Do **not** invent locked APP_LOAD (`0x3000003`) from this pin alone. Historical fixture `testdata/xdna2_cmd/last_apply.json` may still classify as APP_LOAD; that is a **different** pin (2026-07-18). This shot’s pin is wait_timeout + journal IO fault / FW timeout.

Offline p5 / c3 / c4 / p6 on this evidence: **not** Pass; `invent_pass=0`; CONFIG_CU tokens measured.

---

## Unlock bookkeeping

**Mark `C_UNLOCK=11bc9016` CONSUMED.**

- One-shot was for a real live CONFIG_CU + PDI stock path.
- Real attempt completed far enough to write last_apply and hit WAIT_CMD failure.
- Unlock ≠ Pass. Consuming means **do not** re-open under `11bc9016`.
- Prior CONSUMED stay historical only: `3a70fabf` (2026-08-09), `b17b0119` (2026-07-18).
- A further live shot needs a **new** host-minted unlock **after** a named offline delta (T3), not another empty reconfirm.

---

## Not Pass

| Claim | Value |
|-------|--------|
| Product Pass | **No** |
| `ran_on_npu` | **No** (`false`) |
| invent_pass | **0** |
| CONFIG_CU attach alone | research progress only, not Pass |
| Offline C4 / P6 candidate | still requires live decide for product Pass |

---

## Who acts next

| Who | Next |
|-----|------|
| **Agent (T3)** | Named offline product delta from wait_timeout / WAIT_CMD EINVAL (os error 22) / BO map / IO_PAGE_FAULT / FW timeout class. Examples: wait budget honesty, WAIT_CMD error path, map hygiene research, residual classify for this pin class. **No** second unlock until T3 lands. |
| **Human** | No stock sandwich paste theater required for this path. Capture under `testdata/xdna2_cmd/external_known_good/` still optional when real public-stack bytes exist. Do not reopen consumed unlocks. |
| **Live again** | Only after T3 named offline delta + **new** host-minted unlock (T4). |

---

## Paths to keep

- Live log: `/tmp/dragon_npu_live_shot_11bc9016.log`
- last_apply pin: `/tmp/dragon_npu_last_apply_config_cu_1374078.json`
- Residual SSoT: `RESIDUAL.md` top Unlock / plain status
- Prior timeout report (no pin): `.agents/reports/live-shot-3a70fabf-timeout.md`
- T2 closeout: `.agents/reports/impl-t2-live-11bc9016-closeout.md`
