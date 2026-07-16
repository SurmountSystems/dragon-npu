# XDNA2 command / instruction blob staging (lab only)

## Quick stage (research NOOP only)

```bash
bash scripts/materialize_txn_offline.sh
# → lab_instr.bin classic v0.1 NOOP (valid XAie header; NOT GEMM)
# optional: --from-aiebu for lab copy_DDR txn if Ryzen AI aiebu present
```

Pure Rust: `dragon_npu::backends::amd_xdna2::{classify_instruction_blob, build_strix_research_noop_txn}`.

## Purpose

Local staging directory for **operator-captured** or **license-cleared** instruction
(txn control-code) bytes used with:

```bash
export XDNA2_INSTRUCTION_BLOB=/absolute/or/repo/path/testdata/xdna2_cmd/lab_instr.bin
```

Dragon-npu loads these via `load_instruction_blob_from_env` /
`load_instruction_blob_from_path` and may fill a device BO for START_NPU
`buffer` / `buffer_size` when EXEC gates are intentionally open.

This directory is **not** a source of default CI fixtures. No proprietary
binaries are committed.

## What is gitignored

`.gitignore` ignores binary / dump content under this tree while keeping this
README and optional non-binary audit notes:

```gitignore
testdata/xdna2_cmd/**
!testdata/xdna2_cmd/README.md
!testdata/xdna2_cmd/**/PROVENANCE.md
!testdata/xdna2_cmd/**/*.sha256
!testdata/xdna2_cmd/**/*.txt
```

Also ignored elsewhere (mis-stage protection): `**/lab_instr.bin`, `**/ml_txn.bin`,
`**/*_instr.bin`, `testdata/**/*.{bin,elf}`, `*.elf`.

Typical local files (not in git):

- `lab_instr.bin` — raw instruction BO contents  
- `*.elf`, `*.xclbin`, `ml_txn.bin` — sources / intermediates  
- optional `PROVENANCE.md` / `*.sha256` (may be committed if non-secret)  
- `mladf_ref/{ifm,wts,ofm_expected}.bin` — from `bash tools/pack_mladf_ref.sh`
  (staged a16w4 sizes; WTS may be zeros — not Pass weights)  

```bash
bash tools/pack_mladf_ref.sh
export XDNA2_LAB_PACK=mladf_a16w4
export XDNA2_LAB_WTS_PACKED=$PWD/testdata/xdna2_cmd/mladf_ref/wts.bin
```

## How to set the env hook

```bash
# Prefer absolute path
export XDNA2_INSTRUCTION_BLOB="$PWD/testdata/xdna2_cmd/lab_instr.bin"

# Alternative: hex (BLOB wins if both set)
# export XDNA2_INSTRUCTION_HEX=deadbeef...

unset XDNA2_ALLOW_EXEC_CMD   # keep L1 closed unless operator lab
```

See:

- `docs/XDNA2_TXN_RESEARCH.md` — architecture + capture pathways  
- `scripts/capture_instruction_blob_checklist.md` — operator procedure  
- `scripts/inventory_txn_sources.sh` — read-only host inventory  
- `scripts/extract_instruction_offline.sh` — offline inventory + optional VE2 stage  
- `docs/XDNA2_EXEC_CMD_LAB.md` — single-shot EXEC only after blob exists  

## Offline extract / VE2 staging (research only)

```bash
# Report only (exit 0)
bash scripts/extract_instruction_offline.sh

# Optional: copy VE2 Runner latency nop.elf into gitignored out dir
# Writes PROVENANCE.md with NOT_AIE2_KNOWN_GOOD=1 — never AIE2/17f0 Pass.
bash scripts/extract_instruction_offline.sh --stage-ve2-nop
# → testdata/xdna2_cmd/ve2_latency/sources/ve2_runner_latency_nop.elf
# lab_instr.bin only with --force-lab-instr --allow-ve2-as-lab-instr (still not known-good).
# lab_exec_once --apply refuses NOT_AIE2 PROVENANCE blobs unless --allow-research-blob.
```

**VE2 `shim_ve2/Runner/latency/nop.elf` is PLATFORM=VE2_EDGE** — different device ops /
firmware from Strix PCI `1022:17f0` / npu4 AIE2. Do **not** set
`XDNA2_INSTRUCTION_BLOB` to VE2 artifacts and claim silicon Pass.

Real npu4 path remains partner `local_shim_test_data/npu4/...` or
ml_txn→patched ctrltext (see TXN_RESEARCH).

## Honesty

- Loading a blob ≠ NPU compute Pass.  
- `ok_submit_wait_with_inst_blob` ≠ `ran_on_npu`.  
- Product `ran_on_npu` requires real ioctl path + submit/wait OK + blob +
  **numerical verify** of device output (`decide_ran_on_npu` policy).  
- VE2 staged copies are **research provenance only** (`NOT_AIE2_KNOWN_GOOD=1`).  
- Never reverse-engineer proprietary XRT plugin guts into this tree.
