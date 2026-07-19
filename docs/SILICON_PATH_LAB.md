# Silicon path (gate-only `b6a66dee`; prior `dd35ae15`) lab (WTS → ctrl → gated EXEC)

## Plain English

1. **Weights** — The NPU program needs real packed weights (174080 bytes), not zeros.  
2. **Control packet** — Same program expects a ~29 KB control blob (`lab_ctrl.bin`).  
3. **One gated run** — Temporary driver unlock + EXEC + compare outputs to a **non-zero** golden.

Packing/layout alone never sets `ran_on_npu`.

## Strategy (offline first; HITL B + optional capture; C2 **CONSUMED** historical)

Default progress is **offline** (contract tests, GT oracle, pure-direct /
**APP_LOAD** residual). Public true post-patch dump research is **closed offline**.
Live EXEC is **not** a debugger — see [HITL_BC.md](./HITL_BC.md),
[HITL_C_APP_LOAD_RECIPE.md](./HITL_C_APP_LOAD_RECIPE.md) (canonical C sandwich — **CONSUMED**
under `b17b0119`), [XDNA2_XRT_CONTRACT_MLADF.md](./XDNA2_XRT_CONTRACT_MLADF.md),
[RESIDUAL.md](../RESIDUAL.md).

| Mode | Who | Command |
|------|-----|---------|
| Offline check | anyone | `bash tools/silicon_lab_operator.sh check` (**CONTRACT_OK ≠ Pass** / ≠ unlock alone) |
| B GT | you once | `bash tools/hitl_b_gt_capture.sh` |
| Capture | optional | L3 → `capture-instr` → close → `diff` — **optional** logs only (unpatched CLASS A already body-green) |
| **C2** | **CONSUMED / historical** (`C_UNLOCK=b17b0119`) | already ran once; recipe = record-only: [HITL_C_APP_LOAD_RECIPE.md](./HITL_C_APP_LOAD_RECIPE.md); unlock **≠** Pass; **not** unpatched A / mere `diff rc=0` / registry / offline green alone |

**Never EXEC raw `lab_instr.bin`.** Prefer `lab_instr.elf` (aiebu + rela).  
**No Pass claim** without OFM allclose on real ioctl + ERT Completed.  
**Unpatched CLASS A / mere `diff rc=0` = body match only → ≠ Pass.**  
Further C after the consumed one-shot needs a **new agent unlock** (evidence may justify; never auto-unlocks).

**Highest-value now (single source of truth):** maintain `run_offline_gates` green + L3=`N` under gate-only
`b6a66dee` (prior `dd35ae15`; `C_UNLOCK=b17b0119` **CONSUMED**); offline c3/c4 classify; **not** operator C2
re-run. Further live C only after **new** unlock. Maintain pure-direct offline (not capture-golden as P1).

## Operator one-liner helper

```bash
bash tools/silicon_lab_operator.sh status           # assets + exec_cmd_allow + readiness
bash tools/silicon_lab_operator.sh stage            # instr+ctrl+WTS (+ aiebu ELF when possible)
bash tools/silicon_lab_operator.sh check            # offline contract (ELF + GT + preflight)
bash tools/silicon_lab_operator.sh preflight        # require-pass assets
bash tools/silicon_lab_operator.sh dry-run          # no L3 / no dragon-npu EXEC
# Optional OFM re-lock only (not current highest-value; assets often already staged):
# sudo sh -c 'echo 1 > /sys/module/amdxdna/parameters/exec_cmd_allow'
# bash tools/silicon_lab_operator.sh capture-golden   # DD load_xrt + install-golden
# sudo sh -c 'echo 0 > /sys/module/amdxdna/parameters/exec_cmd_allow'
# Optional capture-instr (logs only; unpatched body class already green offline):
# sudo sh -c 'echo 1 > /sys/module/amdxdna/parameters/exec_cmd_allow'
# bash tools/silicon_lab_operator.sh capture-instr
# sudo sh -c 'echo 0 > /sys/module/amdxdna/parameters/exec_cmd_allow'
# bash tools/diff_vendor_instr_parity.sh   # unpatched A = body match only ≠ C-ready
# Milestone C2 one-shot C_UNLOCK=b17b0119 — full recipe docs/HITL_C_APP_LOAD_RECIPE.md:
# sudo sh -c 'echo 1 > /sys/module/amdxdna/parameters/exec_cmd_allow'
# bash tools/hitl_c_pass_apply.sh --assume-l3-already-open
# sudo sh -c 'echo 0 > /sys/module/amdxdna/parameters/exec_cmd_allow'   # restore L3
```

`capture-golden` runs `tools/capture_mladf_ofm_golden.sh` (same synthetic tensors
as WTS export, bf16 ones IFM, vendor `execute`, 512 B bf16 dump). Optional
re-lock only — **not** current P1 once golden is staged.

## Phase D diagnostics (map_host / 0x106)

Live 2026-07-16 pure-direct apply reached EXEC but failed before GEMM Pass:

| Userspace | Kernel journal fingerprint |
|-----------|----------------------------|
| `status=exec_eio` (or historical `payload_unsupported`) | `MSG_OP_MAP_HOST_BUFFER (0x106)` |
| `submit_and_wait` EIO (`os error 5`) | `aie2_map_host_buf size 0x4000000` (64 MiB DevHeap) |
| FW status `0x4000003` (`AIE2_STATUS_INVALID_PARAM`) | Often **unaligned DevHeap UVA** (not BO handles) |
| `inst_blob_addr_invalid` if `xdna_addr=0` / `u64::MAX` | (often after Shmem instr BO — XRT uses **BO_DEV**) |
| `detail` includes `bo_diag=[DevHeap: …; instr: …]` | Compare map_offset / xdna_addr / **vaddr % 0x4000000** |

### Root cause (D3c — 64 MiB UVA align)

Firmware maps the client DevHeap via `aie2_map_host_buf` in `dev_mem_size` (64 MiB) chunks.
The **user VA must be 64 MiB-aligned**. Plain `mmap` of the DRM BO often returns only
2 MiB alignment (e.g. live `vaddr=0x7f3515400000` → `INVALID_PARAM`).

XRT (`shim/buffer.cpp` `mmap_ptr` + `kmq/pcidev.cpp` `heap_page_size`) reserves an aligned
anonymous VA window, then `MAP_FIXED`s the DevHeap BO. Pure-direct now does the same in
`mmap_dev_heap_drm_aligned` / `ensure_dev_heap`.

After re-apply, `bo_diag` DevHeap `vaddr` should satisfy `vaddr % 0x4000000 == 0`.

**Offline helpers (no L3):**

- Instruction BO = `BufferType::Dev` + `write_dev_bo_via_heap` (D2)
- SHMEM PASID pin for IFM/WTS/OFM/ctrl (D2b)
- DevHeap mmap **64 MiB-aligned UVA** (D3c)
- Refuse invalid `xdna_addr` (D1)
- On submit failure: auto `bo_diag` dump of DevHeap + instruction (+ IFM/WTS/OFM/ctrl if bound)
- Optional always-on: `export XDNA2_LAB_BO_DIAG=1`
- OFM decide refuse: `ofm_stats=[raw_len=… nonzero_f32=… first_bf16_le=…]` (no fake Pass)

**Pass apply — HISTORICAL / CONSUMED (operator C2 — one-shot `C_UNLOCK=b17b0119`; unlock ≠ Pass):**

Offline `CONTRACT_OK` alone is **not** Pass **and never substitutes for agent unlock**.
Unpatched `diff` rc=0 / CLASS A is **body match only** → **≠ Pass**. Registry dump =
Transaction/cache image only (not proven live-patched `run_` BO). INSTR_BO true
post-patch public path is **closed** — do **not** treat registry mode as a preferred
C pre-req. Silicon residual is **APP_LOAD**; **C2 CONSUMED** (ran once under `b17b0119`;
do **not** re-run under this stamp; further C needs **new agent unlock**; evidence may justify, never auto-unlocks).
Offline work = maintain pure-direct tests / contract. Canonical C sequence:
[HITL_C_APP_LOAD_RECIPE.md](./HITL_C_APP_LOAD_RECIPE.md). See [HITL_BC.md](./HITL_BC.md) /
`RESIDUAL.md`. Do not iterate flags via raw `lab_exec_once` as progress.

```bash
bash tools/silicon_lab_operator.sh check   # CONTRACT_OK ≠ Pass / ≠ unlock alone
bash tools/diff_vendor_instr_parity.sh     # unpatched A ≠ Pass; rc=2 → agent fix
# optional capture-instr = logs only (public true post-patch path closed)
# C2 one-shot C_UNLOCK=b17b0119 — see docs/HITL_C_APP_LOAD_RECIPE.md:
# sudo sh -c 'echo 1 > /sys/module/amdxdna/parameters/exec_cmd_allow'
# bash tools/hitl_c_pass_apply.sh --assume-l3-already-open
# sudo sh -c 'echo 0 > /sys/module/amdxdna/parameters/exec_cmd_allow'
```

Expect detail: `aiebu_elf=1`, `rela_count=12`, `qos_gops=100`, `tiles=…`.  
Raw `.bin` EXEC is refused. **No Pass / `ran_on_npu` claim** without OFM allclose + ERT Completed.

## Stage offline assets

```bash
# needs Ryzen AI DD install for dump_dd_txn
bash tools/silicon_lab_operator.sh stage
# or low-level:
# bash tools/stage_silicon_lab_assets.sh --export-wts
# bash tools/stage_silicon_lab_assets.sh --wts-from /path/to/amd_packed_wts.bin
```

Produces under `testdata/xdna2_cmd/silicon_lab/` (gitignored bins):

| File | Size | Role |
|------|------|------|
| `lab_instr.bin` | 1688 | Raw DD txn (offline dump only) |
| `lab_instr.elf` | ~32 KiB | **EXEC payload** — aiebu ELF (`.ctrltext` + `.rela.dyn`); stage via `tools/stage_aiebu_lab_instr.sh` |
| `lab_ctrl.bin` | 29664 | Control packet (= ELF `.ctrldata`) |
| `lab_ctrl_meta.json` | — | Sizes + patch locations |
| `ifm.bin` | 2304 | bf16 ones (also rebuilt in Rust) |
| `wts.bin` | 174080 | zeros unless `--wts-from` / `--export-wts` |
| `ofm_expected.bin` | 512 | zeros — **not** Pass golden |

## Real AMD-packed WTS (Phase A / Milestone A)

`pack_mladf_ref` alone **does not** invent an AMD weight shuffle. Zero WTS is
length-only plumbing.

### Working path on this project (preferred)

Vendor `mladfmatmulbias::export_const_params` is linked offline when Ryzen AI DD
+ system XRT + `libaiebu` are present:

```bash
# 1) Export real AMD-shuffled lab WTS (174080 B, non-zero)
bash tools/export_mladf_wts.sh
# → testdata/xdna2_cmd/mladf_wts_export/wts.bin + PROVENANCE.md

# 2) Offline validate (same gate as Rust validate_mladf_wts_packed_for_pass)
bash tools/validate_mladf_wts.sh testdata/xdna2_cmd/mladf_wts_export/wts.bin

# 3) Stage silicon lab assets with that WTS
bash tools/stage_silicon_lab_assets.sh --wts-from testdata/xdna2_cmd/mladf_wts_export/wts.bin
# equivalent one-shot (export then stage):
# bash tools/stage_silicon_lab_assets.sh --export-wts

# 4) Wire lab env
export XDNA2_LAB_WTS_PACKED=$PWD/testdata/xdna2_cmd/mladf_wts_export/wts.bin
export XDNA2_LAB_REQUIRE_NONZERO_WTS=1
```

**Deps (fail-closed):**

| Need | Default |
|------|---------|
| `DD_ROOT` | `~/Projects/ai/ryzen-ai-install/venv/lib/python3.12/site-packages/ryzenai_dynamic_dispatch` |
| XRT | `-lxrt_coreutil -lxrt++ -lxrt_core` on system |
| `AIEBU_LIB` | `/usr/lib/libaiebu.a` |

**Honesty notes for the export tool:**

- Host tensors are **lab synthetic** (int4 pattern + float scales=1 + int8 zeros + float bias=0).  
- Attr `op_version=v2` (this DD build refuses `flat` with “flat version is not supported for matmulbias”; open headers map both flat and v2 to `MatmulWtsShuffle::BFP_UNIFIED`).  
- Output **must** be exactly **174080** bytes and non-zero — tool refuses otherwise.  
- **Do not commit** `wts.bin` (EULA / gitignored).  
- Export ≠ Pass: still need non-zero OFM golden + L1/L3 EXEC + allclose for `ran_on_npu`.

### Other honest options

1. **Capture** — run vendor ORT / Vitis EP once, dump the WTS BO (174080 B, non-zero), then `--wts-from`.  
2. **`--wts-from`** — any operator-supplied 174080 non-zero file after validate:  
   `bash tools/validate_mladf_wts.sh /path/to/amd_packed_wts.bin`  
   `bash tools/stage_silicon_lab_assets.sh --wts-from /path/to/amd_packed_wts.bin`

Rust pure gate (no device):

```text
validate_mladf_wts_packed_for_pass(wts, 174080)  // len + not all zero
// also: validate_wts_packed_len_nonzero (mladf_pack)
```

Then:

```bash
export XDNA2_LAB_WTS_PACKED=…/wts.bin   # must be non-zero for Pass
export XDNA2_LAB_REQUIRE_NONZERO_WTS=1  # refuse zero / missing pack at BO create
```

**Not claimed:** pure-Rust port of the AMD shuffle (not needed if export/capture works).

## Control packet (Phase B / Milestone B)

The staged mladf program expects a **fourth** kernel-arg BO: the runtime control
packet (**29664** bytes for the staged key), not just IFM/WTS/OFM.

### Working path (preferred)

```bash
# 1) Stage vendor dump (instr + ctrl + meta + size fixtures)
bash tools/stage_silicon_lab_assets.sh
# → testdata/xdna2_cmd/silicon_lab/lab_ctrl.bin (29664) + lab_ctrl_meta.json
#    stage runs tools/validate_mladf_ctrl.sh automatically

# 2) Offline validate (same gate as Rust validate_mladf_ctrl_packed_for_pass)
bash tools/validate_mladf_ctrl.sh testdata/xdna2_cmd/silicon_lab/lab_ctrl.bin

# 3) Wire lab env (count-4 kernel args + pin CTRL before instruction)
export XDNA2_LAB_CTRL_PACKED=$PWD/testdata/xdna2_cmd/silicon_lab/lab_ctrl.bin
export XDNA2_LAB_MLADF_META=$PWD/testdata/xdna2_cmd/silicon_lab/lab_ctrl_meta.json
# default XDNA2_LAB_REQUIRE_CTRL=0 for unit tests; set 1 for Pass-readiness dry-run
export XDNA2_LAB_REQUIRE_CTRL=1
```

### Rust (pure + BO)

| Helper | Role |
|--------|------|
| `validate_mladf_ctrl_packed_for_pass` | len + not empty/all-zero (offline Pass gate) |
| `load_ctrl_packed_from_env` | `XDNA2_LAB_CTRL_PACKED` capped read; size match to plan |
| `attach_ctrl_bo` / `IoBoSet.ctrl` | fourth Shmem BO fill |
| `pack_start_npu_kernel_args_with_ctrl` | kernel-arg count **4** + ctrl paddr |
| `build_lab_arg_bo_handles_ex` | pin list IFM/WTS/OFM/**CTRL**/inst |
| `parse_mladf_ctrl_patch_locations` | meta `control_packet_patch_locations` offsets (non-array → Err) |
| `ctrl_u64_patches_for_io_paddrs` / `patch_ctrl_from_meta_io_paddrs` | pure paddr writes |
| `patch_ctrl_packet_u64_le` | LE u64 overwrite at offsets (OOB refuse) |
| `XDNA2_LAB_CTRL_AUTO_PATCH=1` | opt-in: after IFM/WTS/OFM BO create, write device paddrs into ctrl before attach (needs `XDNA2_LAB_MLADF_META`) |

**REQUIRE_CTRL:** `XDNA2_LAB_REQUIRE_CTRL=1` refuses missing/**empty/all-zero**/wrong-size ctrl when
the plan has `ctrl_bytes` (pack mode **and** `create_and_fill_mladf_io_bos_ex`). Default
**unset** so unit tests and 3-BO plumbing stay green.

**Auto-patch (lab, optional):** default **off**. Set
`export XDNA2_LAB_CTRL_AUTO_PATCH=1` for Phase C dry-runs so vendor dump paddrs are
replaced with this process's IFM/WTS/OFM `xdna_addr` values (u64 LE overwrite residual).

**Honesty / residual:**

- `lab_ctrl.bin` is a **vendor DD dump**, not hand-written microcode.  
- Meta lists `size: 6` and `operation: read_add_write`: lab applies
  **existing field + BO base** and writes **6** LE bytes (preserves tile offsets
  e.g. OFM `0x100`, WTS `0x15400`). Full u64 overwrite was wrong (zero OFM after EXEC).  
- Ctrl attach / count-4 ≠ `ran_on_npu` / silicon Pass.

## Live EXEC (Phase C / Milestone C — operator HITL)

**Goal:** `ran_on_npu=true` **only** after gated EXEC+WAIT and OFM allclose vs a
**non-zero** golden. This agent session does **not** claim silicon Pass without a
verified live run.

### Pass criteria (airtight)

Product `ran_on_npu` requires **all** of:

| Criterion | Notes |
|-----------|--------|
| Real ioctl path | `RealDirectSubmission` (never stub) |
| EXEC+WAIT OK | L1 feature + `XDNA2_ALLOW_EXEC_CMD=1` + L3 `exec_cmd_allow=1` |
| Instruction blob used | non-empty START_NPU txn |
| OFM readback | `XDNA2_LAB_OFM=1` + successful BO read |
| Decide true | allclose vs **non-zero** golden for mladf |

Pure helpers (no device):

```text
decide_ran_on_npu / silicon_ran_on_npu_flags_ok
classify_lab_matmul_status → ok_npu_verified only when decide_ran
validate_mladf_{wts,ctrl,ofm_golden}_packed_for_pass
silicon_pass_asset_readiness  // offline checklist object
```

**Never** true from envelope-only submit, zero WTS, zero golden, or stub path.

`XDNA2_LAB_ALLOW_ZERO_GOLDEN=1` only bypasses **offline resolve** of a zero file
(shell validator exits **3**, not 0). Product decide still returns
`AllZeroGolden` → `ran_on_npu=false`. `--require-pass-assets` ignores ALLOW.

### OFM golden (operator-supplied)

| Source | Pass? |
|--------|--------|
| `pack_mladf_ref` `ofm_expected.bin` (512 zeros) | **No** — refused |
| Host f32 ones×ones golden | **Invalid** for a16w4 (pack mode refuses) |
| Non-zero bf16 **512** B (`m=1,n=256`) or f32 **1024** B | Yes candidate |
| Capture from known-good vendor/XRT run of **same** key+WTS | Preferred |

```bash
# Offline validate (same gate as Rust validate_mladf_ofm_golden_packed_for_pass)
bash tools/validate_mladf_ofm_golden.sh /path/to/ofm_golden.bin
export XDNA2_LAB_OFM_GOLDEN=/path/to/ofm_golden.bin
```

**Capture checklist (optional):** after a known-good vendor ORT/Vitis EP run of the
staged key, dump the OFM BO (512 B bf16), write `ofm_golden.bin` under
`testdata/xdna2_cmd/silicon_lab/` (gitignored), record provenance, then validate.
Do **not** invent zeros. Host reference for a16w4 needs real AMD-packed WTS math
(not shipped as pure-Rust golden generator).

### Offline Pass-readiness preflight (no L3 / no EXEC)

```bash
# After stage + real WTS + non-zero golden:
bash tools/stage_silicon_lab_assets.sh --export-wts   # or --wts-from …
# place non-zero golden:
# cp /path/to/captured_ofm.bin testdata/xdna2_cmd/silicon_lab/ofm_golden.bin
#   or: export XDNA2_LAB_OFM_GOLDEN=…

# Soft report (warn on missing golden/zero WTS; still exit 0):
bash tools/silicon_lab_preflight.sh --print-env

# Fail closed if any Pass asset missing (instr + WTS nz + CTRL nz + golden nz):
bash tools/silicon_lab_preflight.sh --require-pass-assets --print-env
```

### Exact dry-run then apply (copy-paste)

```bash
export PATH=/usr/bin:$PATH
# Prefer real AMD WTS already staged; golden path required for Pass attempt:
export XDNA2_LAB_OFM_GOLDEN=$PWD/testdata/xdna2_cmd/silicon_lab/ofm_golden.bin  # non-zero bf16 512

# 1) Offline asset gate
bash tools/silicon_lab_preflight.sh --require-pass-assets --print-env

# 2) Lab dry-run: diagnose + silicon recipe + env print — NEVER opens L3 / EXEC
bash scripts/lab_exec_once.sh --silicon-mladf --require-pass-assets
# (sets PACK/KEY/META/WTS/CTRL/REQUIRE_*/OFM/AUTO_PATCH + default blob=lab_instr.elf)

# 3) Apply only if dry-run OK + operator accepts wedge risk (HITL sudo for L3):
# Prefer lab_instr.elf (aiebu); script upgrades .bin→.elf when sibling exists.
# bash scripts/lab_exec_once.sh --silicon-mladf --require-pass-assets \
#   --apply --i-accept-wedge-risk --blob "$XDNA2_INSTRUCTION_BLOB"
#
# Exit codes (unchanged honesty):
#   0 = dry-run plan OK, OR apply cargo exit 0 + restore OK
#       (npu_direct_bench exits 0 only when ran_on_npu=true — rare)
#   4 = process non-zero / !ran_on_npu but gates restored (common without golden match)
#   5 = restore failed — fix exec_cmd_allow=0 manually
#
# Always verify after apply:
# cat /sys/module/amdxdna/parameters/exec_cmd_allow   # expect N or 0
# bash scripts/diagnose_amdxdna.sh
```

Manual env (equivalent to `--silicon-mladf` wiring):

```bash
export XDNA2_INSTRUCTION_BLOB=$PWD/testdata/xdna2_cmd/silicon_lab/lab_instr.elf
export XDNA2_LAB_OFM=1
export XDNA2_LAB_PACK=mladf_a16w4
export XDNA2_LAB_MLADF_KEY='mladfmatmulbias_prmptflat_mladf_2x4x4_flat_a16fw4acc16f_1_1152_256_128'
export XDNA2_LAB_MLADF_META=$PWD/testdata/xdna2_cmd/silicon_lab/lab_ctrl_meta.json
export XDNA2_LAB_WTS_PACKED=$PWD/testdata/xdna2_cmd/mladf_wts_export/wts.bin
export XDNA2_LAB_CTRL_PACKED=$PWD/testdata/xdna2_cmd/silicon_lab/lab_ctrl.bin
export XDNA2_LAB_OFM_GOLDEN=$PWD/testdata/xdna2_cmd/silicon_lab/ofm_golden.bin
export XDNA2_LAB_REQUIRE_NONZERO_WTS=1
export XDNA2_LAB_REQUIRE_CTRL=1
export XDNA2_LAB_CTRL_AUTO_PATCH=1
```

### Honesty (Milestone C)

- Dry-run green ≠ `ran_on_npu`.  
- Apply exit **4** when `!ran_on_npu` is expected without golden match; restore still
  clears L3.  
- Never leave `exec_cmd_allow=1` permanent; never set `XDNA2_ALLOW_EXEC_CMD=1` in
  default tests.  
- This document alone does not claim silicon Pass.

## Related

- [HITL_BC.md](./HITL_BC.md) — Milestone B/C only  
- [XDNA2_XRT_CONTRACT_MLADF.md](./XDNA2_XRT_CONTRACT_MLADF.md) — shim/XRT field table  
- [AMD_MLADF_MATMUL_PACK.md](./AMD_MLADF_MATMUL_PACK.md)  
- [XDNA2_EXEC_CMD_LAB.md](./XDNA2_EXEC_CMD_LAB.md)  
- [XDNA2_NEVER_AGAIN.md](./XDNA2_NEVER_AGAIN.md)  
- [XDNA2_DIRECT_CAPABILITIES.md](./XDNA2_DIRECT_CAPABILITIES.md) env table  
- Tools: `tools/silicon_contract_check.sh`, `tools/hitl_b_gt_capture.sh`, `tools/hitl_c_pass_apply.sh`  

