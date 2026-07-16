# Residual — silicon path (WTS → ctrl → EXEC + OFM)

**Date:** 2026-07-16

## Done (offline / pure-direct)

### Buffer fill (prior plan)
- IFM size + bf16 pack; OFM size + bf16 unpack
- WTS **correct length** (zeros or `XDNA2_LAB_WTS_PACKED`)
- Lab BO create/fill for `XDNA2_LAB_PACK=mladf_a16w4`
- pack_mladf_ref + timed pack wire + zero-golden refuse

### Milestone A — real AMD-packed WTS

| Item | Status |
|------|--------|
| **Offline AMD export** | `tools/export_mladf_wts.{cpp,sh}` → **174080 non-zero** |
| **Validate / stage / REQUIRE_NONZERO_WTS** | done |
| **Honesty** | packing ≠ `ran_on_npu` |

### Milestone B — control packet BO ~29 KB

| Item | Status |
|------|--------|
| **Validate / stage / REQUIRE_CTRL / count-4 pin** | done |
| **Meta u64-LE patch residual** | not full XRT size=6 read_add_write |

### Milestone C — Live L3 EXEC + OFM → `ran_on_npu` (this turn)

| Item | Status |
|------|--------|
| **Pass criteria pure** | `decide_ran_on_npu` + `silicon_ran_on_npu_flags_ok`; classify `ok_npu_verified` only on decide; zero golden refuse for mladf |
| **OFM golden offline gate** | `validate_mladf_ofm_golden_packed_for_pass` + `tools/validate_mladf_ofm_golden.sh` (512 bf16 / 1024 f32, non-zero) |
| **Asset readiness** | `silicon_pass_asset_readiness` + `tools/silicon_lab_preflight.sh` (`--require-pass-assets`) |
| **Lab recipe** | `lab_exec_once.sh --silicon-mladf` wires PACK/KEY/META/WTS/CTRL/REQUIRE_*/OFM/GOLDEN/AUTO_PATCH; dry-run default; apply still risk+blob |
| **Docs** | `docs/SILICON_PATH_LAB.md` Phase C exact dry-run → apply |
| **Live EXEC this session** | **Not claimed** — operator HITL only (sudo L3 + non-zero golden) |

Pure helpers: `validate_mladf_{wts,ctrl,ofm_golden}_packed_for_pass`,
`silicon_pass_asset_readiness`, `silicon_ran_on_npu_flags_ok`,
`check_mladf_ofm_golden_not_all_zero`, `patch_ctrl_from_meta_io_paddrs`,
`bytes_are_all_zero`.

## Still open (HITL)

1. **Non-zero bf16 OFM golden** for staged key (capture from vendor run or host-ref after real WTS) — agent cannot invent Pass golden
2. **Sudo L3** single-shot: `lab_exec_once.sh --silicon-mladf --apply --i-accept-wedge-risk …`
3. **`ran_on_npu=true`** only after L1+L3 + EXEC+WAIT + non-zero golden allclose on silicon

## Env recipe (dry-run first)

```bash
bash tools/export_mladf_wts.sh
bash tools/stage_silicon_lab_assets.sh --wts-from testdata/xdna2_cmd/mladf_wts_export/wts.bin
# export XDNA2_LAB_OFM_GOLDEN=…/nonzero_bf16_512.bin
bash tools/silicon_lab_preflight.sh --require-pass-assets --print-env
bash scripts/lab_exec_once.sh --silicon-mladf --require-pass-assets
# apply only with operator risk accept — exit 4 if !ran_on_npu still possible
```

See `docs/SILICON_PATH_LAB.md`.
