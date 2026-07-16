# PROVENANCE — AMD-packed mladf WTS (export_mladf_wts)

**Lab only. NOT a silicon Pass. NOT product weights for deployment.**

## License

Packed via AMD Ryzen AI DynamicDispatch `export_const_params` under the
operator's AMD EULA install. Do **not** commit this binary to public git.

## Key (target)

```
mladfmatmulbias_prmptflat_mladf_2x4x4_flat_a16fw4acc16f_1_1152_256_128
```

| Field | Value |
|-------|-------|
| M × K × N × Gs | 1 × 1152 × 256 × 128 |
| WTS bytes | 174080 |
| Non-zero bytes | 149120 |
| op_version attr | v2 (flat refused by this DD; BFP_UNIFIED shuffle) |
| Source | vendor `mladfmatmulbias::export_const_params` |
| Synthetic host tensors | int4 ones-ish + float scales=1 + int8 zeros + float bias=0 |

## Honesty

- This is a **real AMD shuffle**, not pure-Rust fake packing.
- Host weights are **lab synthetic** (not a trained model).
- extra_blob_*.bin may be kernel side-data; lab uses wts.bin only.
- Never claim `ran_on_npu` from packing alone.
- Wire with: `export XDNA2_LAB_WTS_PACKED=<this>/wts.bin` and
  `XDNA2_LAB_REQUIRE_NONZERO_WTS=1`.
