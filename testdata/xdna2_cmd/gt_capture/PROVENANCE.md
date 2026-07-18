# PROVENANCE — offline GT capture (dump_mladf_gt_offline)

**Not live XRT BO dump. Not ran_on_npu.**

## What this is

Pure-direct **patch oracle** for staged `lab_instr.elf` + `lab_ctrl.bin` + meta:
- `.rela.dyn` applied with fixed synthetic paddrs
- ctrl meta size=6 `read_add_write` patches applied
- START_NPU envelope + synthetic hwctx/bo_table (layout only)

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
| `start_npu_cmd.bin` | ERT_START_NPU envelope (count4, prop_count=0) |
| `hwctx.json` | synthetic tiles=32 qos 100/30 |
| `bo_table.json` | instr Dev + IFM/WTS/OFM/ctrl Shmem sizes |
| `kernel_args.json` | count4 + xrt3 u32 packs |
| `contract.json` | checksums + rela table |
| `ifm.bin` / `wts.bin` / `ofm_golden.bin` | optional copies from silicon_lab |
| `MANIFEST.sha256` | file hashes |

## Key

`mladfmatmulbias_prmptflat_mladf_2x4x4_flat_a16fw4acc16f_1_1152_256_128`

## Next

1. Offline contract (self-check only): `bash tools/silicon_contract_check.sh` — **CONTRACT_OK ≠ C-ready**
2. Offline parity: `bash tools/diff_vendor_instr_parity.sh` (0=unpatched body match CLASS A ≠ C-ready; 2=mismatch+CLASS_HINT; 3=miss)
3. Agent offline pure-direct / APP_LOAD residual if needed (ban env C applies)
4. Optional capture-instr (logs only; unpatched A already body-green): L3 → `capture-instr` → close → `diff`
5. Milestone C **only if agent says ready** — **not** unlocked by `diff rc=0` / CLASS A / registry / parity alone: `bash tools/hitl_c_pass_apply.sh`
6. Optional Milestone B OFM golden re-lock: `bash tools/hitl_b_gt_capture.sh --with-vendor`

**Honesty:** unpatched CLASS A / mere `diff rc=0` = body match only → **≠ C-ready**, **≠ `ran_on_npu`**.
