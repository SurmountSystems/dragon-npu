# Plan: Complete remaining product silicon residual (workspace mirror)

**Date:** 2026-08-10
**Session plan SSoT:** host session `plan.md` (complete product silicon)
**Implement status:** **T0–T4 research-blob done.** Unlock **`97f38307` CONSUMED**. **Not** Pass. Skip-PDI A/B optional only with **new** unlock + named offline reason. Goal still product Pass.

## Elevator truth

Product silicon is still **not** Pass. Latest stock live under research CVML PDI reproduced the same WAIT EINVAL / orphan IO_PAGE_FAULT class as `11bc9016`.

| Item | Value |
|------|--------|
| **Unlock** | **`C_UNLOCK=97f38307` CONSUMED** (2026-08-10 research-blob stock; do not reopen) |
| **Prior CONSUMED** | `11bc9016` (2026-08-10 WAIT EINVAL + orphan fault); `3a70fabf`; `b17b0119` |
| **This pin (97f38307)** | last_apply `/tmp/dragon_npu_last_apply_config_cu_3377745.json`; status=`wait_cmd_einval`; `config_cu=1`; `pdi_bytes=283840`; pdi sha16=`a1e65d04add3422c`; journal IO_PAGE_FAULT ~`0x2fff80007f00` |
| **T3 offline** | `wait_cmd_einval` |
| **T3b offline** | fault **`0x2fff80007f40` → orphan** vs bo_diag map |
| **T3c offline** | research PDI ≠ product known-good; fingerprint; A/B **skip CONFIG_CU** tooling |
| **T4 live** | research-blob leg **done**; **not** Pass; invent_pass=0; **no** `ran_on_npu` |
| **Capture** | `testdata/xdna2_cmd/external_known_good/` PROVENANCE only; **no** real `.bin` |
| **Pass / ran_on_npu** | **No.** invent_pass=0. Unlock ≠ Pass. Identity ≠ Pass. |

## Steps status (T-track)

| Step | Status | Owner |
|------|--------|--------|
| T0 Stock live-shot runner | **done** (`config-cu-run` / `run-stock`) | agent |
| T1 One live stock shot under `11bc9016` | **done** (last_apply exists; unlock CONSUMED) | agent |
| T2 Interpret + residual honesty | **done** (p5/c3/c4/p6 not Pass) | agent |
| T3 Named offline delta (WAIT classifier) | **done** (`wait_cmd_einval`; report impl-t3) | agent |
| T3b Further offline (IO_PAGE_FAULT / reloc / BO map) | **done** (orphan lock; report impl-t3b) | agent |
| T3c PDI identity + skip CONFIG_CU A/B | **done** (report impl-t3c) | agent |
| T4 One stock live under `97f38307` (research blob) | **done** (report live-shot-97f38307 + impl-t4 closeout; **not** Pass) | agent |
| Skip-PDI A/B live | **optional only** with **new** unlock + named offline reason | agent |
| T5 Pass closeout | only if live decide true | agent |
| T6 Hard stop | pin blocker once when no capture / no matmul PDI / no driver-MMU delta | agent |
| Track B capture | parallel when bytes exist; never invent | human path / agent install |

## Residual class (this pin)

**Honest WAIT EINVAL (`wait_cmd_einval`) / FW timeout / orphan IO_PAGE_FAULT vs BO map / research PDI not product known-good.** Fault addresses in `0x2fff8000…` are not equal any reported BO base. Default dump is CVML/AIE2P research overlay, not product matmul known-good. Do not invent locked APP_LOAD from this pin alone.

## Who acts next

**Agent only if** a named offline delta is real:

1. Real capture bytes under `testdata/xdna2_cmd/external_known_good/`, **or**
2. Matmul-matched PDI source (not CVML research overlay alone), **or**
3. Driver / MMU map work that addresses orphan IO_PAGE_FAULT

Else **T6 hard stop**: pin the blocker once. Do not mint unlock or re-spin empty implement.

**Skip-PDI A/B** (`config-cu-run-skip`) is **optional** only after a **new** host-minted unlock with a named offline reason. Do **not** reopen `97f38307` to run skip-PDI.

## Honesty

- **No** product Pass. invent_pass=0. **No** `ran_on_npu`.
- Do **not** reopen `97f38307`, `11bc9016`, `3a70fabf`, or `b17b0119`.
- Do **not** invent Pass, capture bytes, or a second fake PDI blob.
- Research PDI identity / skip tooling ≠ Pass.
- Research-blob live shot ≠ Pass.
- **Hard-stop risk** if capture and matmul-matched PDI remain absent.

## Reports

- `.agents/reports/live-shot-97f38307.md`
- `.agents/reports/impl-t4-live-97f38307-closeout.md`
- `.agents/reports/impl-t3c-pdi-identity.md`
- `.agents/reports/impl-t3b-io-page-fault-bo-map.md`
- `.agents/reports/impl-t3-wait-cmd-einval-classify.md`
- `.agents/reports/live-shot-11bc9016.md`
- `.agents/reports/impl-t2-live-11bc9016-closeout.md`
- Explore: `.agents/reports/explore-t3b-io-page-fault-pass-path.md`
- Explore: `.agents/reports/explore-t3-wait-io-page-fault.md`
- Prior: `.agents/reports/live-shot-3a70fabf-timeout.md`
- Prior prep: `.agents/reports/impl-complete-product-silicon-s0-s2b.md`
