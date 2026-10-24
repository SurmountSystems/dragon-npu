# U4 closeout — flexml unified-2x4x4 stock CONFIG_CU live (`C_UNLOCK=2ae69036` CONSUMED)

**Date:** 2026-08-10
**Honesty:** invent_pass=0 · no product Pass · no `ran_on_npu` · unlock CONSUMED

---

## Mission done

One host-minted unlock + one stock live CONFIG_CU with the flexml STX unified-2x4x4 PDI that offline fingerprint proved **DIFF** from the failed CVML research dump. Evidence interpreted offline (p5/c3/c4/p6). Unlock consumed. No second live.

## Named offline reason (pre-live)

| | CVML baseline (failed lives) | Preferred flexml |
|--|------------------------------|------------------|
| path / dump | CVML AIE2P research dump | `/tmp/dragon_npu_pdi_flexml_unified-2x4x4.bin` |
| bytes | 283840 | **27200** |
| sha16 | `a1e65d04add3422c` | **`0eeeb70c84df056e`** |
| fingerprint | — | **DIFF** (from `pass-unblock-fingerprint-once`) |

## Steps executed

1. **Mint** `C_UNLOCK=2ae69036` (host `openssl rand -hex 4`); first `C_UNLOCK=` line in `RESIDUAL.md` active without CONSUMED.
2. **Live (stock, no sudo L3):**
   ```bash
   export XDNA2_LAB_PDI_BLOB=/tmp/dragon_npu_pdi_flexml_unified-2x4x4.bin
   export XDNA2_LAB_CU_FUNC=0
   export XDNA2_HITL_OUTER_TIMEOUT_S=120
   bash tools/operator_app_load_kit.sh config-cu-run
   ```
   Log: `/tmp/dragon_npu_live_shot_2ae69036.log`
3. **Interpret:** last_apply pin + p5/c3/c4/p6 via kit post path.
4. **Consume** unlock in `RESIDUAL.md`; write live-shot + this closeout.

## Live result (facts)

| Field | Value |
|-------|--------|
| last_apply | `/tmp/dragon_npu_last_apply_config_cu_675133.json` |
| status | `wait_cmd_einval` |
| ran_on_npu | false |
| real_ioctl | true |
| hitl_c_rc | 4 |
| total_s | ≈ 62.59 |
| config_cu | 1 |
| pdi_bytes | 27200 |
| pdi sha16 | `0eeeb70c84df056e` |
| cu_func | 0 |
| aiebu_elf / rela | 1 / 12 |
| WAIT | EINVAL os error 22 immediately (`timeout_ms=5000` in message; not a true wait_timeout class) |

### Journal class (shot window)

- `aie2_error_backtrack` Row0 Col4 event 73 cat 8
- `aie2_set_cmd_timeout` DPU PC / TXN OP = `0xffffffff`
- **No** `IO_PAGE_FAULT` in window (unlike CVML `97f38307` / `11bc9016`)

### Offline helpers

| Helper | Result |
|--------|--------|
| soft classify | `not_pass_no_ert_error`; `config_cu_present=1`; invent_pass=0 |
| C3 | evidence body differs from baseline; not Pass |
| C4 | C4_SKIP (`json_ran_on_npu_false`) |
| P6 | P6_NOT_PASS / P6_SKIP |

## Honesty locks

- invent_pass=0
- **no** product Pass
- **no** `ran_on_npu`
- unlock CONSUMED ≠ reopen
- flexml attach progress ≠ Pass
- do not invent APP_LOAD `0x3000003` from this pin alone
- do not reopen `2ae69036` / `97f38307` / `11bc9016` / `3a70fabf` / `b17b0119`

## Residual after U4

Still blocked on firmware accept + ERT Completed + OFM allclose. Flexml PDI changed attach size and removed orphan IO_PAGE_FAULT in this window, but WAIT EINVAL + FW timeout remain. Next agent work only with a **new named offline delta** (real capture bytes, matmul-matched program/PDI path, or driver/MMU). No empty reconfirm mill.

## Artifacts

- `/tmp/dragon_npu_live_shot_2ae69036.log`
- `/tmp/dragon_npu_last_apply_config_cu_675133.json`
- `/tmp/dragon_npu_pdi_flexml_unified-2x4x4.bin`
- `.agents/reports/live-shot-2ae69036.md` (this mission’s live report)
- `RESIDUAL.md` top Unlock CONSUMED

## One-line return

**Pass? No · status=`wait_cmd_einval` · unlock `2ae69036` CONSUMED · invent_pass=0 · config_cu=1 pdi_bytes=27200 sha16=0eeeb70c84df056e · no ran_on_npu**
