# AMD mladf matmul buffer packing (plain English)

## Why this exists

We extracted a **real** NPU matrix-multiply **program** for Strix. That program
does **not** read plain `float32` “all ones” buffers.

It expects:

| Buffer | Meaning | Staged size |
|--------|---------|-------------|
| **IFM** (input) | Activations as **16-bit** values | **2304** bytes |
| **WTS** (weights) | Packed **4-bit** weights + scales/bias, shuffled | **174080** bytes |
| **OFM** (output) | Results as **16-bit** values | **512** bytes |
| Ctrl packet | Runtime control (needed later for EXEC) | **29664** bytes |

Program extract path (plain English): [BUILD_NPU_MATMUL_PROGRAM.md](./BUILD_NPU_MATMUL_PROGRAM.md).

## Target kernel (staged lab)

```text
mladfmatmulbias_prmptflat_mladf_2x4x4_flat_a16fw4acc16f_1_1152_256_128
```

### Key anatomy (parse → M / K / N / Gs)

```text
mladfmatmulbias _ prmptflat _ mladf _ 2x4x4 _ flat _ a16fw4acc16f _ 1 _ 1152 _ 256 _ 128
|---------------| |--------| |-----| |-----| |----| |-------------| | | |----| |---| |---|
 op family         mode       stack   tiles   layout  dtype tags      M K      N     Gs
```

| Field | Value | Source |
|-------|--------|--------|
| M × K × N | **1 × 1152 × 256** | Trailing digits in key |
| Group size (Gs) | **128** | Last trailing digit group |
| Activation | a16 (bf16-style, 2 bytes/elem) | tag `a16f…` |
| Weight | w4 (4-bit packed) | tag `…w4…` |
| Accum / out | acc16f (2 bytes/elem) | tag `…acc16f` |

Sanity (a16 formula):

- `IFM = M×K×2 = 1×1152×2 = **2304**`
- `OFM = M×N×2 = 1×256×2 = **512**`
- `WTS = **174080**` (from AMD `*_ctrl_meta` only — not a simple M×K×N formula)
- `ctrl = **29664**` (from meta; not filled by default lab path yet)

Rust: `parse_mladf_instr_key` → `MladfMatmulShape { m, k, n, group_size }`  
and `staged_mladf_buffer_plan()` / `parse_mladf_ctrl_meta_json` print the sizes above.

```bash
# pure unit path (no /dev/accel):
cargo test --features amd-xdna2 --lib mladf -- --nocapture
# staged plan asserts IFM=2304 OFM=512 WTS=174080 CTRL=29664
```

## Dump txn + ctrl_meta (WP0 fixtures)

Needs local Ryzen AI DynamicDispatch (`~/Projects/ai/ryzen-ai-install/venv`):

```bash
bash tools/dump_dd_txn.sh \
  'mladfmatmulbias_prmptflat_mladf_2x4x4_flat_a16fw4acc16f_1_1152_256_128' \
  /tmp/wp0_dump 5
```

What the tool writes (gitignored if under `testdata/xdna2_cmd/dd_matmul_txn/`):

| File | Content |
|------|---------|
| `<key>.bin` | Classic v0.1 XAie txn (instruction program) |
| `<key>_ctrl_meta.json` | JSON sizes for IFM/WTS/OFM/ctrl (**preferred** when payload is JSON) |
| `<key>_ctrl.bin` | Control packet binary (optional sibling) |

Legacy dumps may still have `*_ctrl_meta.bin` (same JSON bytes, wrong extension).
Re-run the dump script to get `.json`.

Verify sizes from meta (pin **exact** staged basename — avoid `*1152_256_128*`
globs that also match GM siblings with OFM=8388608):

```bash
python3 - <<'PY'
import json, sys
# Exact staged key only (no _1_*_gm):
p = ("/tmp/wp0_dump/mladfmatmulbias_prmptflat_mladf_2x4x4_flat_"
     "a16fw4acc16f_1_1152_256_128_ctrl_meta.json")
try:
    d = json.load(open(p))
except FileNotFoundError:
    sys.exit(f"missing {p} — re-run dump_dd_txn.sh for the exact staged key")
for b in d["external_buffers"].values():
    print(f"{b['name']:28} {b['size_in_bytes']}")
# expect: coalesed_inputs 2304, coalesed_weights 174080, coalesed_outputs 512
#          runtime_control_packet 29664
PY
```

Env keys must be **map keys** (or dump basenames without path). Rust normalizes
`.json`/`.bin` and `_ctrl_meta`/`_ctrl`/`_param`/`_*_*_gm` suffixes via
`normalize_mladf_instr_key`; prefer the bare map key in `XDNA2_LAB_MLADF_KEY`.

### Non-exact / GM keys (BO sizes)

| API | What you get |
|-----|----------------|
| `is_exact_staged_mladf_key` | Sibling/ext strip only — **GM never exact-staged** |
| `mladf_buffer_plan_from_key(gm, Some(wts))` | Shape OK; OFM = `M×N×2`; **ctrl = 0** (not GM meta) |
| `parse_mladf_ctrl_meta_json` / `XDNA2_LAB_MLADF_META` | **Authoritative** IFM/WTS/OFM/ctrl (e.g. GM OFM 8 MiB) |

Do not use WTS override alone for GM kernels — load sibling `*_ctrl_meta.json`.

**Do not commit** AMD proprietary `.bin` / meta dumps to public git.

## What dragon-npu implements now

| Piece | Status |
|-------|--------|
| Parse key → M/K/N/Gs (GM/`_ctrl` strip) | Done (`normalize_mladf_instr_key` + parse) |
| Staged constants 2304 / 174080 / 512 / 29664 | Done (exact staged key only) |
| Parse `*_ctrl_meta` JSON sizes | Done (meta authoritative; a16 IFM check) |
| Dump txn + sibling ctrl_meta as `.json` | Done (`tools/dump_dd_txn.*`) |
| Pack IFM as bf16 ones / from f32 | Done |
| Unpack OFM bf16 → f32 | Done |
| WTS correct **length** (zeros or file load) | Done |
| Offline host pack tool (`pack_mladf_ref`) | Done — IFM correct; WTS zeros or `--wts-from` |
| WTS **AMD shuffle** offline export | Done — `tools/export_mladf_wts.{cpp,sh}` links DD `export_const_params` + XRT + aiebu → 174080 non-zero; validate via `tools/validate_mladf_wts.sh` |
| Pure-Rust byte-identical shuffle | **Not claimed** — use export / capture |
| Timed path `XDNA2_LAB_PACK=mladf_a16w4` | Done (`create_and_fill_lab_io_bos_for_pack_mode`) |
| OFM golden env (`XDNA2_LAB_OFM_GOLDEN`) | Done (bf16 or f32; fail-closed size) |
| Ctrl packet bind in START_NPU | Done — `attach_ctrl_bo` / count-4 args / `XDNA2_LAB_CTRL_PACKED` + `REQUIRE_CTRL`; validate `tools/validate_mladf_ctrl.sh` |
| Meta patch locations → pure u64 LE | Done — `parse_mladf_ctrl_patch_locations` + `patch_ctrl_from_meta_io_paddrs` (size=6 `read_add_write` residual) |
| Live EXEC Pass | **Later** (operator HITL; not claimed from ctrl attach) |

## Env (lab)

```bash
# 1) Produce staged host buffers (gitignored):
bash tools/pack_mladf_ref.sh
# → testdata/xdna2_cmd/mladf_ref/{ifm,wts,ofm_expected}.bin + PROVENANCE.md
#    ifm=2304 bf16 ones; wts=174080 zeros (or --wts-from AMD-packed); ofm=512 zeros
#
# Produced vs consumed by lab:
#   ifm.bin          — produced for offline inspection; lab rebuilds bf16 ones in-process
#   wts.bin          — optional via XDNA2_LAB_WTS_PACKED (only packed file lab loads)
#   ofm_expected.bin — zeros placeholder; not a Pass golden (refused if used as-is)

# Pack mode applies on the timed path only when XDNA2_LAB_OFM=1 (I/O create/fill).
export XDNA2_LAB_OFM=1
export XDNA2_LAB_PACK=mladf_a16w4          # default is f32_ones; unknown values fail-closed
export XDNA2_LAB_MLADF_KEY='mladfmatmulbias_prmptflat_mladf_2x4x4_flat_a16fw4acc16f_1_1152_256_128'
# optional:
export XDNA2_LAB_MLADF_META=/path/to/*_ctrl_meta.json
export XDNA2_LAB_WTS_PACKED=$PWD/testdata/xdna2_cmd/mladf_ref/wts.bin   # must be exactly 174080 B for staged key
# OFM golden required for mladf ran_on_npu (no f32 ones default). Must be non-zero
# unless XDNA2_LAB_ALLOW_ZERO_GOLDEN=1 (plumbing only — not product Pass):
# export XDNA2_LAB_OFM_GOLDEN=/path/to/nonzero_ofm.bin  # bf16 m*n*2 or f32 m*n*4
# do NOT use pack_mladf_ref ofm_expected.bin (all zeros) as Pass golden
```

Default `XDNA2_LAB_PACK` unset → old f32 ones path (unchanged).

`timed_matmul_direct` with **`XDNA2_LAB_OFM=1`** respects `XDNA2_LAB_PACK`
(without LAB_OFM, pack env is ignored for I/O):
- `f32_ones` (default): ones IFM/WTS + f32 ones×ones golden  
- `mladf_a16w4`: bf16 IFM + packed/zero WTS; **refuses** f32 ones golden and
  **all-zero** golden (unless `XDNA2_LAB_ALLOW_ZERO_GOLDEN=1`); needs a
  **non-zero** `XDNA2_LAB_OFM_GOLDEN` file (bf16 or f32) for Pass-path decide  
- Unknown `XDNA2_LAB_PACK` values **fail closed** (no silent ones fallback)  
- `XDNA2_LAB_WEIGHT=1` **overrides pack mode** and uses the **f32 model-weight**
  path (ones IFM × model WTS golden) — it is **not** an mladf bf16 golden source  
- Timed `dtype` for mladf must be f32/bf16/a16 (documentation only; fill is bf16;
  result dtype reports `mladf_a16w4` when that pack path ran)

## Honesty

- Zero-filled WTS is **valid length** for BO plumbing tests; it is **not** a
  correct matmul weight tensor for OFM golden.
- **f32 ones×ones golden is invalid for a16w4** — pack-mode decide fail-closes
  without `XDNA2_LAB_OFM_GOLDEN` (bf16 `m*n*2` or f32 `m*n*4`).
- `ofm_expected.bin` from `pack_mladf_ref` is zeros placeholder, not a Pass target.
- Ctrl meta shows a **fourth** buffer (control packet). Live EXEC may need that
  wired separately — packing IFM/WTS/OFM alone is necessary but may not be
  sufficient for Pass.
- No `ran_on_npu` claim from packing alone.

## Code

- `src/backends/amd_xdna2/mladf_pack.rs` — pure parse/pack  
- Public re-exports from `amd_xdna2` (`STAGED_MLADF_*`, `parse_mladf_instr_key`, …)  
- `lab_io::create_and_fill_mladf_io_bos` — fill BOs without EXEC  
- `lab_io::lab_decide_from_ofm_bytes_pack_mode` / `load_ofm_golden_from_env`  
- `tools/dump_dd_txn.sh` / `tools/dump_dd_txn.cpp` — dump txn + `*_ctrl_meta` siblings  
- `tools/pack_mladf_ref.sh` / `tools/pack_mladf_ref.cpp` — offline IFM/WTS/OFM sizes  

## Next

1. ~~Real AMD-packed `wts.bin`~~ — `bash tools/export_mladf_wts.sh` then `--wts-from` / `XDNA2_LAB_WTS_PACKED`  
2. Non-zero bf16 OFM golden for the same key — validate with `tools/validate_mladf_ofm_golden.sh`; see Phase C in `docs/SILICON_PATH_LAB.md`  
3. ~~Ctrl BO + offline preflight~~ — `tools/silicon_lab_preflight.sh` + `lab_exec_once --silicon-mladf`  
4. Live L3 EXEC + allclose → `ran_on_npu` (operator HITL only; dry-run green ≠ Pass)  

