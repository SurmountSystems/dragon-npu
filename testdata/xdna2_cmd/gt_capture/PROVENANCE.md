# PROVENANCE — offline GT capture (dump_mladf_gt_offline)

**Not live XRT BO dump. Not ran_on_npu.**

## What this is

Pure-direct **patch oracle** for staged `lab_instr.elf` + `lab_ctrl.bin` + meta:
- `.rela.dyn` applied with fixed synthetic paddrs
- ctrl meta size=6 `read_add_write` patches applied

## Synthetic paddrs

| Role | Value |
|------|-------|
| IFM | `0xa11f00001000` |
| WTS | `0xa11f00002000` |
| OFM | `0xa11f00003000` |
| CTRL | `0xa11f00004000` |

## Files

| File | Role |
|------|------|
| `instr_pre.elf` | staged aiebu ELF |
| `instr_post_patch.bin` | ctrltext after rela patch |
| `ctrl_post_patch.bin` | lab_ctrl after meta patch |
| `kernel_args.json` | count4 + xrt3 u32 packs |
| `contract.json` | checksums + rela table |
| `MANIFEST.sha256` | file hashes |

## Key

`mladfmatmulbias_prmptflat_mladf_2x4x4_flat_a16fw4acc16f_1_1152_256_128`

## Next

1. Milestone B (optional vendor golden re-lock): `bash tools/hitl_b_gt_capture.sh`
2. Offline parity: `bash tools/silicon_contract_check.sh`
3. Milestone C Pass apply: `bash tools/hitl_c_pass_apply.sh`
