# PROVENANCE — OFM golden capture (capture_mladf_ofm_golden)

**Lab Option A capture. NOT dragon-npu ran_on_npu by itself.**

## Source

AMD Ryzen AI DynamicDispatch `mladfmatmulbias::execute` with
`load_xrt=true`, same lab-synthetic host tensors as `export_mladf_wts`.

| Field | Value |
|-------|-------|
| Key target | `mladfmatmulbias_prmptflat_mladf_2x4x4_flat_a16fw4acc16f_1_1152_256_128` |
| M × K × N × Gs | 1 × 1152 × 256 × 128 |
| OFM bytes | 512 (bf16 LE) |
| Non-zero bytes | 512 |
| IFM | bf16 ones (matches dragon-npu lab fill) |
| WTS path | same synthetic int4/scales/zeros/bias → vendor pack |
| op_version | v2 |

## Honesty

- Golden is **vendor NPU OFM** for this lab tensor set.
- Pair with WTS from the same export (or re-export before capture).
- Pair with WTS from export_mladf_wts using the same synthetic weights.
- Use: `bash tools/silicon_lab_operator.sh install-golden <ofm.bin>`
  or the capture script's auto-install into silicon_lab/.
