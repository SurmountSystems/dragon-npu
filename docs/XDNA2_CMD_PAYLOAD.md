# XDNA2 command BO payload (Phase 1 research)

**Status:** Phase 1 **complete for offline envelope research** — typed builders + unit tests.  
**Does not** authorize default EXEC_CMD. Silicon compute is **not** claimed.  
**Source:** GPL-2.0 `amd/xdna-driver` (`amdxdna_ctx.h`, `aie2_message.c`, public uAPI).  
**Safety:** `docs/XDNA2_NEVER_AGAIN.md`. Host lab: L3 `exec_cmd_allow=0` by default.

**Txn / instruction BO research (opaque control code, capture paths, host inventory):**  
see **[`XDNA2_TXN_RESEARCH.md`](./XDNA2_TXN_RESEARCH.md)** (Phase 3.5 prep). Envelope builders
here are necessary but **not sufficient** for `ran_on_npu`.

### Phase 1.4 success criteria

| Criterion | Status |
|-----------|--------|
| Written layout doc (this file) with known vs opaque | **done** |
| Typed offline builders (header, START_NPU, CHAIN, PREEMPT, DPU) | **done** |
| Unit tests green (no `/dev/accel`, no EXEC_CMD) | **done** |
| Opaque list explicit (txn / prop_args legality) | **done** |
| Live EXEC_CMD + WAIT + known-good instruction BO | **not** Phase 1 (Phase 2+) |

## Layers

```text
Userspace EXEC_CMD ioctl  (amdxdna_drm_exec_cmd)
        │
        ▼
Cmd BO  ──►  struct amdxdna_cmd { u32 header; u32 data[]; }
        │         header packs STATE | EXTRA_CU_MASK | COUNT | OPCODE
        │
        ├─ ERT_CMD_CHAIN     → amdxdna_cmd_chain + cmd BO handles  (no CU masks)
        ├─ ERT_START_NPU     → [cu_mask…] + amdxdna_cmd_start_npu + args  ← AIE2 primary
        ├─ ERT_START_NPU_PREEMPT → [cu_mask…] + amdxdna_cmd_preempt_data + args
        ├─ ERT_START_NPU_PREEMPT_ELF → same preempt_data prefix (AIE2 feature-gated)
        └─ ERT_START_DPU     → [cu_mask…] + amdxdna_cmd_start_dpu  (AIE4/VE2; not AIE2 cmdlist)
        │
        ▼
Firmware / instruction buffer (txn control code)  ← STILL OPAQUE in public docs
```

Real **compute** needs a valid **instruction buffer** (txn control code) at a device address.  
Public headers document the **envelope**; the txn binary is the remaining research gap.

## GPL source map (local + upstream)

| What | Path |
|------|------|
| Packed header, ERT enums, payload structs | `src/driver/amdxdna/amdxdna_ctx.h` (amd/xdna-driver) |
| AIE2 fill_slot / START_NPU / PREEMPT | `src/driver/amdxdna/aie2_message.c` |
| AIE4 START_DPU path | `src/driver/amdxdna/aie4_hwctx.c` |
| VE2 START_DPU layout | `src/driver/amdxdna/ve2_hwctx.c` (`ve2_dpu_data`) |
| EXEC_CMD / WAIT_CMD uAPI | `include/uapi/drm/amdxdna_accel.h` or `src/include/uapi/drm_local/amdxdna_accel.h` |
| Lab clone (this workstation) | `~/Projects/kernel/xdna-driver/` (same files under `src/driver/amdxdna/`) |
| System uAPI (if installed) | `/usr/include/drm/amdxdna_accel.h` |

Structures described here are derived from **GPL-2.0** sources for interoperability. This document is research notes for an independent open userspace client, not a copy of AMD proprietary toolchain docs.

## Packed command header (`amdxdna_cmd.header`)

Bitfields (Linux `GENMASK` inclusive):

| Field | Bits | Mask (logical) | Notes |
|-------|------|----------------|--------|
| STATE | 3:0 | `0x0000_000f` | `ert_cmd_state` (4 bits) |
| *(rsvd)* | 9:4 | — | leave 0 |
| EXTRA_CU_MASK | 11:10 | `0x0000_0c00` | extra CU mask dwords **beyond the first** (2 bits → 0..3) |
| COUNT | 22:12 | `0x007f_f000` | **total** `data[]` dword count (11 bits → 0..2047) |
| OPCODE | 27:23 | `0x0f80_0000` | `ert_cmd_opcode` (5 bits → 0..31) |
| *(rsvd)* | 31:28 | — | leave 0 |

```text
 31     28 27    23 22        12 11 10 9      4 3    0
+---------+--------+------------+-----+--------+------+
|  (rsvd) | OPCODE |   COUNT    | XCU | (rsvd) |STATE |
+---------+--------+------------+-----+--------+------+
```

Rust: `pack_amdxdna_cmd_header` / `unpack_amdxdna_cmd_header` /
`parse_amdxdna_cmd_header_from_bytes` in `src/backends/amd_xdna2/direct.rs`.

**COUNT semantics** (from `amdxdna_cmd_get_payload` in `amdxdna_ctx.h`):

- `num_masks = 0` if opcode is `ERT_CMD_CHAIN`, else `1 + EXTRA_CU_MASK`
- Payload starts at `&data[num_masks]`
- Payload byte length = `(COUNT - num_masks) * 4`
- Driver rejects `COUNT <= num_masks` (NULL payload)

### ERT opcodes (`ert_cmd_opcode`)

| Name | Value | AIE2 cmdlist (`aie2_cmdlist_fill_slot`) |
|------|------:|----------------------------------------|
| `ERT_INVALID_CMD` | `!0` | not a real header opcode (5-bit field cannot hold `!0`) |
| `ERT_START_CU` | 0 | fill CF slot |
| `ERT_START_DPU` | 18 | **unsupported** (`-EOPNOTSUPP`) on AIE2 fill path |
| `ERT_CMD_CHAIN` | 19 | separate chain path (not fill_slot default branch) |
| `ERT_START_NPU` | 20 | **primary** — fill DPU/NPU slot from `amdxdna_cmd_start_npu` |
| `ERT_START_NPU_PREEMPT` | 21 | preempt feature-gated |
| `ERT_START_NPU_PREEMPT_ELF` | 22 | preempt feature-gated; same data prefix as PREEMPT |

### ERT states (`ert_cmd_state`)

| Name | Value |
|------|------:|
| NEW | 1 |
| QUEUED | 2 |
| RUNNING | 3 |
| COMPLETED | 4 |
| ERROR | 5 |
| ABORT | 6 |
| SUBMITTED | 7 |
| TIMEOUT | 8 |
| NORESPONSE | 9 |

Driver sets NEW on submit in the sched path; userspace should still seed NEW.

## Payload after header

### CU masks

For opcodes **other than** `ERT_CMD_CHAIN`, payload starts with  
`num_masks = 1 + EXTRA_CU_MASK` dwords of CU masks, then opcode-specific data  
(`amdxdna_cmd_get_payload` in the driver).

`amdxdna_cmd_get_cu_idx` uses `ffs` on the first non-zero mask word — a **zero**
mask yields `INVALID_CU_IDX` and fill returns `-EINVAL`. Typed builders default
to `cu_mask0 = 0x1`.

Rust helper: `amdxdna_cmd_num_cu_masks(opcode, extra_cu_mask)`.

### `ERT_START_NPU` — `amdxdna_cmd_start_npu` (AIE2 primary)

After CU masks:

| Offset | Type | Field |
|-------:|------|--------|
| 0 | u64 | `buffer` — instruction buffer **device address** |
| 8 | u32 | `buffer_size` — bytes |
| 12 | u32 | `prop_count` |
| 16… | u32[] | `prop_args` (properties + kernel args) |

Fixed prefix size = **16 bytes** (excluding CU masks and `prop_args`).

**Without a real instruction blob at `buffer`, this is not a compute job.**

### `ERT_CMD_CHAIN` — `amdxdna_cmd_chain`

| Offset | Type | Field |
|-------:|------|--------|
| 0 | u32 | `command_count` |
| 4 | u32 | `submit_index` |
| 8 | u32 | `error_index` |
| 12 | u32×3 | `reserved` |
| 24… | u64[] | `data[command_count]` — nested cmd BO handles |

No CU masks before chain body. Driver chain helper rejects
`command_count == 0` or `> 64` (`MAX_CHAINED_SUB_CMD`).

### `ERT_START_NPU_PREEMPT` / `PREEMPT_ELF` — `amdxdna_cmd_preempt_data`

After CU masks:

| Offset | Type | Field |
|-------:|------|--------|
| 0 | u64 | `inst_buf` |
| 8 | u64 | `save_buf` |
| 16 | u64 | `restore_buf` |
| 24 | u32 | `inst_size` |
| 28 | u32 | `save_size` |
| 32 | u32 | `restore_size` |
| 36 | u32 | `inst_prop_cnt` |
| 40… | u32[] | `prop_args` |

Fixed prefix size = **40 bytes** (excluding CU masks and `prop_args`).  
AIE2: requires preempt feature (`AIE2_PREEMPT`); else `-EOPNOTSUPP`.  
ELF path reuses this prefix; firmware may ignore/replace args.

### `ERT_START_DPU` — `amdxdna_cmd_start_dpu`

After CU masks:

| Offset | Type | Field |
|-------:|------|--------|
| 0 | u64 | `dtrace_buffer` |
| 8 | u64 | `instruction_buffer` |
| 16 | u32 | `instruction_buffer_size` |
| 20 | u16 | `uc_index` |
| 22 | u16 | `chained` — number of **following** `amdxdna_cmd_start_dpu` elements |

Fixed size = **24 bytes** after CU masks for a **single** descriptor. Used on
**AIE4 / VE2** paths. Rust builder `build_ert_start_dpu_cmd_bo` only supports
`chained == 0` (rejects multi-element series until implemented).

On **AIE2/XDNA2**, cmdlist fill does **not** accept this opcode; use
`ERT_START_NPU` instead. (Legacy AIE2 “dpu” fill for `ERT_START_NPU` still
reads **`amdxdna_cmd_start_npu`**, not this struct.)

## EXEC_CMD / WAIT_CMD ioctl (userspace)

### `amdxdna_drm_exec_cmd` (already ported)

- `type`: `AMDXDNA_CMD_SUBMIT_EXEC_BUF` / DEPENDENCY / SIGNAL  
- `cmd_handles` + `cmd_count`, `args` + `arg_count`  
- out: `seq`  

### `amdxdna_drm_wait_cmd` (Phase 2: driven by `wait_command`)

```c
struct amdxdna_drm_wait_cmd {
    __u32 hwctx;
    __u32 timeout; /* ms; 0 = infinite */
    __u64 seq;     /* from EXEC_CMD */
};
```

Ioctl: `DRM_IOCTL_AMDXDNA_WAIT_CMD` — **`DRM_IOWR`**, command id 9  
(`DRM_COMMAND_BASE + 9` = `0x49`), size 16 → request code **`0xc010_6449`**.

Ground truth is the **driver** header `drm_local/amdxdna_accel.h` (`DRM_IOWR`).  
Some installable `include/uapi/drm/amdxdna_accel.h` copies still say `DRM_IOW`
(`0x4010_6449`) — do **not** use that; it mismatches the module’s drm_local uAPI.

Rust:

| Symbol | Role |
|--------|------|
| `amdxdna_drm_wait_cmd` | 16-byte struct; const = `iowr::<…>(9)` → `0xc010_6449` |
| `DirectSubmission::wait_command` | Issues WAIT_CMD; **not** L1-gated |
| `submit_and_wait_command` | L1-gated EXEC_CMD + finite WAIT (refuses `timeout_ms=0`) |
| `DEFAULT_WAIT_CMD_TIMEOUT_MS` | 5000; env `XDNA2_WAIT_TIMEOUT_MS` for timed path |

**Infrastructure ready; silicon single-shot is operator-run** — see
[`XDNA2_EXEC_CMD_LAB.md`](./XDNA2_EXEC_CMD_LAB.md). Not claimed proven on silicon
from CI or default `cargo test`.

## Job opcodes (sched layer, not ERT header)

Driver job `opcode`: `OP_USER=0`, `OP_SYNC_BO=1`, `OP_NOOP=4`, …  
Distinct from ERT opcode in `amdxdna_cmd.header`.

## Known vs opaque

| Known (GPL headers / driver) | Opaque / research |
|------------------------------|-------------------|
| Packed `amdxdna_cmd` header bitfields | **Txn control-code** bytes in instruction BO |
| ERT opcodes/states | How compiler emits AIE2 control code |
| START_NPU / CHAIN / PREEMPT / DPU **envelopes** | Legal `prop_args` layouts per firmware / model |
| CU mask count rules (`get_payload` / `get_cu_idx`) | Instruction BO creation from ONNX weights |
| EXEC_CMD + WAIT_CMD uAPI structs | End-to-end GEMM correctness on silicon |
| AIE2 opcode routing in `aie2_cmdlist_fill_slot` | Full multi-kernel fusion encoding |
| L2 null-guards on fill_slot path | Capture of known-good XRT instruction BO (optional) |
| Pure `decide_ran_on_npu` / allclose + lab OFM wire (`XDNA2_LAB_OFM=1`) | Known-good txn blob that writes matching OFM |
| `XDNA2_INSTRUCTION_BLOB` load hook | Cleared known-good blob on host/CI |

**Opaque status (refreshed):** Cmd BO envelope packing is **done** and unit-tested.
Instruction/txn **bytes** remain opaque without a license-cleared ELF/`ml_txn` or
operator capture. See [`XDNA2_TXN_RESEARCH.md`](./XDNA2_TXN_RESEARCH.md) § known vs
opaque and capture pathways A/B/C.

**Honesty:** building a correctly laid-out Cmd BO is **not** the same as NPU
compute. Until a known-good instruction buffer is obtained, EXEC_CMD+WAIT succeed
under multi-layer gates, **and** numerical verify passes, do **not** claim
`ran_on_npu`.

## Lab enablement (Phase 2)

**Full procedure:** [`XDNA2_EXEC_CMD_LAB.md`](./XDNA2_EXEC_CMD_LAB.md).

Summary:

1. `bash scripts/diagnose_amdxdna.sh` → not WEDGED_* (prefer LIKELY_OK; idle refcnt)  
2. Build with `xdna2-dangerous-exec-cmd` **and** `XDNA2_ALLOW_EXEC_CMD=1`  
3. Temporarily `sudo modprobe amdxdna exec_cmd_allow=1` (then restore `=0`)  
4. Single-shot `submit_and_wait_command` with **finite timeout** + Phase 1 / known-good payload  
5. Never loop empty/scaffold payloads  

**Repo status:** wait + submit_and_wait **implemented and unit-tested when gated**.  
Silicon trial is **operator-only** — not claimed done by default tests.

See also: `docs/XDNA2_NEVER_AGAIN.md`, `docs/XDNA2_DIRECT_CAPABILITIES.md`,
`docs/XDNA2_REAL_EXECUTION.md`.

## Rust API (this repo)

| Symbol | Role |
|--------|------|
| `ErtCmdOpcode` / `ErtCmdState` | Typed ERT enums |
| `AmdxdnaCmdHeaderFields` | Unpacked header view |
| `pack_amdxdna_cmd_header` / `unpack_*` | Bitfield helpers |
| `parse_amdxdna_cmd_header_from_bytes` | LE header parse from Cmd BO slice |
| `amdxdna_cmd_num_cu_masks` | CU mask dword count (CHAIN → 0) |
| `build_amdxdna_cmd_bo` | Header + data dwords → `Vec<u8>` |
| `build_ert_start_npu_cmd_bo` | **Preferred** AIE2 START_NPU envelope (`Result`; COUNT overflow → err) |
| `build_ert_start_npu_preempt_cmd_bo` | PREEMPT / PREEMPT_ELF only (rejects other opcodes) |
| `build_ert_start_npu_preempt_elf_cmd_bo` | Convenience for PREEMPT_ELF |
| `build_ert_start_dpu_cmd_bo` | START_DPU single descriptor only (`chained==0`) |
| `build_ert_cmd_chain_bo` | CHAIN of cmd handles (`Result`) |
| `build_exec_cmd_payload_scaffold` | **Legacy** layout; clamps body to COUNT max |
| `AMDXDNA_CMD_COUNT_MAX` | `0x7ff` (11-bit COUNT field) |
| `amdxdna_drm_wait_cmd` | WAIT struct; ioctl const `0xc010_6449` |
| `DirectSubmission::wait_command` | Phase 2 WAIT_CMD ioctl (not L1-gated; infinite needs opt-in) |
| `fill_cmd_bo_with_payload` / `write_bo_bytes` | Host mmap-fill Cmd BO **before** EXEC_CMD (required) |
| `XDNA2_INSTRUCTION_BLOB` / `_HEX` + `get_bo_info.xdna_addr` | Optional START_NPU `buffer` device addr (Phase 3 hook; still not `ran_on_npu`) |
| `max_abs_diff_f32` / `verify_f32_allclose` / `decide_ran_on_npu` | Pure Phase 3.5 verify policy (`numerical.rs`) |
| `lab_io` + `build_ert_start_npu_cmd_bo_with_kernel_args` | LAB_OFM path: prop_count=0 kernel args + OFM decide wire |
| `scripts/lab_exec_once.sh` | Default dry-run; apply exports LAB_OFM=1 + blob |
| `host_matmul_f32_ones_vec` | Full host golden vector for elementwise compare |
| `timed_bo_roundtrip` / `read_bo_bytes` | Safe DMA timing (no EXEC_CMD); not GEMM |
| `submit_and_wait_command` / `SubmitWaitResult` | Gated EXEC_CMD + finite WAIT (caller must fill BO first) |
| `DEFAULT_WAIT_CMD_TIMEOUT_MS` | 5000 ms lab default |
| `check_finite_wait_timeout` | Pure unit-testable refuse of `timeout_ms=0` |

Public re-exports: `src/backends/amd_xdna2.rs` (`pub use direct::…` and `pub use numerical::…`
including `decide_ran_on_npu`, `NumericalVerifyCriteria`, `DEFAULT_VERIFY_*`).

## Provenance

Structures described here are derived from **GPL-2.0** amd/xdna-driver sources for interoperability. This document is research notes for an independent open userspace client, not a copy of AMD proprietary toolchain docs.
