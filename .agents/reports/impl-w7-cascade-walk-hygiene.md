# W7-4 — cascade MASK* walk classifier hygiene

**Scope:** offline only. invent_pass=0. No live NPU / unlock / ALLOW. No residual match to lab classic 1688. No load_accepted re-tag.

## Problem (red observed)

Unit `optional_cascade_public_aot_classifies_if_present` **FAILED** on this host:

```
txn_header_only … walk_err=invalid argument: MASK* size field 28 != 32
cascade ctrltext expected ClassicTxnV01
```

Cascade public peel is present under
`testdata/xdna2_cmd/external_known_good/mlir_aie_cascade_matmul_npu2.ctrltext.bin`
(4512 B, classic v0.1 gen4 8×6 ops=116). Header is valid; walk hard-failed on
MASKWRITE/MASKPOLL size field.

## Root cause

Classic v0.1 MASK* ops have **two legitimate total sizes** (size field LE u32 at +24):

| Encoding | Size field | Who |
|----------|------------|-----|
| **Compact** | **28** | Public mlir-aie npu2 AOT peels (vector 300, single_core 660, whole_array 2592, cascade 4512) |
| **Padded** | **32** | Lab classic 1688 / DynamicDispatch matmul dumps / prior synthetic tests |

Layout through the size field is the same:

`OpHdr(4) + pad(4) + RegOff(8) + Value(4) + Mask(4) + Size(4) [=28]`
Padded adds 4 trailing zero bytes → total 32.

Walker previously required size == 32 only and advanced by hard-coded 32, so
all public mlir-aie peels with MASK* (including cascade) classified as
`TxnHeaderOnly` instead of `ClassicTxnV01`.

Measured cascade MASKWRITE @ off=112: size@24=28; full walk with {28,32}
accept → 116 ops, end==4512.

## Fix

File: `src/backends/amd_xdna2/txn.rs`

1. Split constants: `OP_MASK_V01_SIZE_COMPACT=28`, `OP_MASK_V01_SIZE_PADDED=32`.
2. `is_known_mask_v01_size` accepts only 28 or 32.
3. `op_len_v01` for opcodes 3/4/7: read size@+24, require known size, advance by **size** (not hard-coded 32).
4. Unknown sizes still fail closed → `TxnHeaderOnly` (not invent walk OK).

Docs: `docs/XDNA2_TXN_RESEARCH.md` MASK* row updated for dual size.

Process mop (pre-existing, unblocked clippy -D): two tiny nits in
`silicon_contract.rs` (`items_after_statements`, `if_not_else`) unrelated to
MASK*; no behavior change.

## Tests (TDD)

| Test | Role |
|------|------|
| `optional_cascade_public_aot_classifies_if_present` | **was red** → **green** ClassicTxnV01 ops=116 size=4512; not lab 1688 profile |
| `synthetic_mask_compact_28_walk` | **new** pure compact 28 MASKWRITE+MASKPOLL |
| `mask_unknown_size_is_header_only` | **new** size=20 stays TxnHeaderOnly |
| `synthetic_write_mask_block_custom_walk` | still green padded 32 |

Commands (all exit 0):

```bash
cargo fmt
cargo clippy --features amd-xdna2 --lib -- -D warnings
cargo test --features amd-xdna2 --lib txn::
# 26 passed (includes cascade + new MASK* units)
```

Also checked: `optional_lab_matmul_blob_classifies_if_present`,
`classify_staged_lab_raw_bin_if_present` still green (lab size=32 path).

## Honesty

- invent_pass=0
- Pass = No; ran_on_npu = No
- Cascade / public AOT = RESEARCH ONLY; residual_matched_claim=0 vs lab classic 1688
- Walk hygiene ≠ load_accepted ≠ residual identity close
- Unlock: none; no live apply

## Files touched

- `src/backends/amd_xdna2/txn.rs` (walk + tests)
- `docs/XDNA2_TXN_RESEARCH.md` (MASK* size table)
- `src/backends/amd_xdna2/silicon_contract.rs` (clippy mop only)
- `.agents/reports/impl-w7-cascade-walk-hygiene.md` (this report)
