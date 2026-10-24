# Live shot report: `C_UNLOCK=97f38307` stock research-blob CONFIG_CU (`wait_cmd_einval`)

**Date:** 2026-08-10
**Honesty:** `invent_pass=0` · **no** product Pass · **no** `ran_on_npu` · unlock **CONSUMED**

---

## Plain English: what ran

One stock live CONFIG_CU shot under unlock `97f38307` used the **research PDI blob** leg (CVML/AIE2P dump, not product matmul known-good). Structured last_apply was written. Product status is honest **`wait_cmd_einval`** (T3 classifier works; not false `wait_timeout`). Same residual class as the prior `11bc9016` pin.

| Item | Value |
|------|--------|
| Unlock | `C_UNLOCK=97f38307` (one-shot; now **CONSUMED**) |
| Experiment | **research blob** (`config-cu-run`); **not** skip CONFIG_CU |
| Command | `bash tools/operator_app_load_kit.sh config-cu-run` |
| Log | `/tmp/dragon_npu_live_shot_97f38307.log` |
| last_apply | `/tmp/dragon_npu_last_apply_config_cu_3377745.json` (2215 B) |
| status | `wait_cmd_einval` |
| `ran_on_npu` | `false` |
| `real_ioctl` | `true` |
| hitl_c | `hitl_c_rc=4` |
| Wall time | `total_s` ≈ 60.88 |
| PDI | CVML dump 283840 B; `pdi_fp` sha16=`a1e65d04add3422c`; research_pdi_not_product_known_good=1 |

### Measured detail tokens (from last_apply)

- `config_cu=1` (CONFIG_CU attach measured)
- `pdi_bytes=283840`
- `cu_func=0`
- `aiebu_elf=1`
- `rela_count=12`
- WAIT_CMD failed immediately: `timeout_ms=5000` then **Invalid argument (os error 22)** → product class **`wait_cmd_einval`**
- bo_diag: DevHeap `xdna_addr=0x4000000`, instr `0x4068000`, host UVAs for IFM/WTS/OFM/CTRL

CONFIG_CU attach and aiebu ELF+rela userspace path **did** run. The job did **not** complete as ERT Completed with non-zero OFM. Product Pass remains **false**.

---

## Journal (this boot)

**Window for this shot:** 2026-08-10 ~12:15–12:16

- **IO_PAGE_FAULT** at addresses in the `0x2fff8000…` range (e.g. `0x2fff80007f00`, `0x2fff80008000`, …)
- **`aie2_set_cmd_timeout`** (DPU PC=`0xffffffff`, TXN OP ID=`0xd`, Context ID=15)

**Prior similar journal:** 2026-08-10 ~11:34–11:35 under unlock `11bc9016` (fault `0x2fff80007f40` orphan vs bo_diag map). Same class.

---

## Residual class for this pin

**Class:** honest WAIT EINVAL (`wait_cmd_einval`) / FW timeout / orphan IO_PAGE_FAULT vs BO map / research PDI not product known-good.

Do **not** invent locked APP_LOAD (`0x3000003`) from this pin alone. Soft classify on this pin: `not_pass_no_ert_error` with `config_cu_present=1`. Historical fixture `testdata/xdna2_cmd/last_apply.json` may still classify as APP_LOAD; that is a **different** pin (2026-07-18).

Offline p5 / c3 / c4 / p6 on this evidence: **not** Pass; invent_pass=0; C3 evidence (body differs from baseline); C4_SKIP / P6_NOT_PASS / P6_SKIP.

---

## Unlock bookkeeping

**Mark `C_UNLOCK=97f38307` CONSUMED.**

- One-shot was for a real live CONFIG_CU + research PDI stock path.
- Real attempt completed far enough to write last_apply and hit WAIT_CMD EINVAL.
- Unlock ≠ Pass. Consuming means **do not** re-open under `97f38307`.
- Prior CONSUMED stay historical only: `11bc9016` (2026-08-10), `3a70fabf` (2026-08-09), `b17b0119` (2026-07-18).
- Skip-PDI A/B was **not** used for this one-shot. Re-running skip-PDI needs a **new** host-minted unlock **and** a named offline reason (not empty reconfirm).

---

## Not Pass

| Claim | Value |
|-------|--------|
| Product Pass | **No** |
| `ran_on_npu` | **No** (`false`) |
| invent_pass | **0** |
| CONFIG_CU attach alone | research progress only, not Pass |
| Research CVML PDI | not product matmul known-good |
| Offline C4 / P6 candidate | still requires live decide for product Pass |

---

## Who acts next

| Who | Next |
|-----|------|
| **Agent** | Only if a **named offline delta** is real: capture bytes under `testdata/xdna2_cmd/external_known_good/`, matmul-matched PDI source, or driver/MMU map work. Else **pin blocker once (T6)** and **stop**. |
| **Human** | Optional real capture under `testdata/xdna2_cmd/external_known_good/` when public-stack bytes exist. No stock sandwich paste theater. Do not reopen consumed unlocks. |
| **Live again** | Only after new named offline reason + **new** host-minted unlock. **No** second unlock this slice. |

---

## Paths to keep

- Live log: `/tmp/dragon_npu_live_shot_97f38307.log`
- last_apply pin: `/tmp/dragon_npu_last_apply_config_cu_3377745.json`
- Residual SSoT: `RESIDUAL.md` top Unlock / plain status
- Prior same-class shot: `.agents/reports/live-shot-11bc9016.md`
- T4 closeout: `.agents/reports/impl-t4-live-97f38307-closeout.md`
