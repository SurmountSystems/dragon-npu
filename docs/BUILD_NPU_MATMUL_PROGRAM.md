# Building the NPU matrix-multiply program (plain English)

## What “the program” is

Your NPU does not run ONNX or Rust directly. For each job it needs a **binary
control program** (a few KB–MB) that tells the hardware tiles what to do —
including matrix multiply.

We **do not hand-write** that microcode. AMD already ships thousands of
pre-built matrix-multiply programs inside Ryzen AI **DynamicDispatch**
(`libtransaction.a`). Our job is to **extract** the right one and feed it to
dragon-npu.

## What we built

| Piece | What it does |
|-------|----------------|
| `tools/dump_dd_txn.cpp` + `tools/dump_dd_txn.sh` | Opens AMD’s program library and writes real matmul binaries to disk |
| Staged `testdata/xdna2_cmd/lab_instr.bin` | One small Strix matmul program (gitignored binary) |
| Rust `txn` module | Checks the file is a real program container (classic v0.1 header + ops) |

### Extract on this machine

Needs the full Ryzen AI / DynamicDispatch install (you already have it under
`~/Projects/ai/ryzen-ai-install/venv`):

```bash
bash tools/dump_dd_txn.sh 'mladfmatmulbias_prmptflat' \
  testdata/xdna2_cmd/dd_matmul_txn 20

# Exact staged key: txn + sibling *_ctrl_meta.json (IFM/WTS/OFM sizes)
bash tools/dump_dd_txn.sh \
  'mladfmatmulbias_prmptflat_mladf_2x4x4_flat_a16fw4acc16f_1_1152_256_128' \
  /tmp/wp0_dump 5
```

Example result:

- **6565+** matmul keys under `mladfmatmul…`
- Classic header: version 0.1, **device gen 4** (Strix), **8×6** tile grid
- Small sample ~**1688 bytes** (not a blank NOOP)
- Sibling `*_ctrl_meta.json` when present (JSON sizes; gitignored with dumps)

## Filling memory (pack layouts)

See **[AMD_MLADF_MATMUL_PACK.md](./AMD_MLADF_MATMUL_PACK.md)**.

```bash
# Offline size-correct host buffers (IFM bf16 ones; WTS zeros unless --wts-from):
bash tools/pack_mladf_ref.sh
export XDNA2_LAB_OFM=1   # pack mode applies on timed path only with LAB_OFM
export XDNA2_LAB_PACK=mladf_a16w4
export XDNA2_LAB_WTS_PACKED=$PWD/testdata/xdna2_cmd/mladf_ref/wts.bin
# a16w4 decide refuses f32 ones + all-zero golden — need a real non-zero golden:
# export XDNA2_LAB_OFM_GOLDEN=/path/to/ofm_golden.bin   # bf16 m*n*2 or f32 m*n*4
```

IFM/OFM bf16 pack + WTS **size** plumbing is in-tree. Weight **shuffle** still
usually comes from an AMD-packed file for now (`pack_mladf_ref` does **not** fake
it). f32 ones×ones and all-zero `ofm_expected.bin` are **invalid** Pass goldens.

## What this is *not* yet

Extracting the program ≠ green checkmark that “NPU matmul works end-to-end.”

Still needed for a true Pass:

1. **Real packed weights** (not zero-fill) for the a16w4 shuffle  
2. **Temporary driver unlock** (sudo) so `EXEC_CMD` is allowed once  
3. **Read the output** and compare to a golden for *that* kernel (bf16 domain)  
4. Possibly the **control packet** BO (meta lists four buffers)

Until those succeed, we still report **no** `ran_on_npu`.

## Lab use (after extract)

```bash
export XDNA2_INSTRUCTION_BLOB=$PWD/testdata/xdna2_cmd/lab_instr.bin
export XDNA2_INSTRUCTION_REQUIRE_TXN=1
# optional live shot (needs root + risk accept):
# bash scripts/lab_exec_once.sh --apply --i-accept-wedge-risk --blob "$XDNA2_INSTRUCTION_BLOB"
```

## License

Dumped binaries are **AMD’s**. Lab/private use under your Ryzen AI license.
Do **not** commit them to public git or rehost on a public CDN.

## Why we didn’t “invent” matmul from scratch

Writing working NPU control code by hand is a multi-year firmware project.
Using AMD’s already-compiled matmul programs is the realistic path to a
verifiable Pass on silicon.
