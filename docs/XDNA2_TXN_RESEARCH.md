# XDNA2 instruction / txn BO research (Phase 3.5 prep)

**Status:** Offline research consolidated — envelope known; **txn control-code bytes still opaque**.  
**Date:** 2026-07-15  
**Safety:** No EXEC_CMD by default. No silicon GEMM Pass. No proprietary bytes committed.  
**Cross-links:** [`XDNA2_CMD_PAYLOAD.md`](./XDNA2_CMD_PAYLOAD.md) · [`XDNA2_DIRECT_CAPABILITIES.md`](./XDNA2_DIRECT_CAPABILITIES.md) · [`XDNA2_EXEC_CMD_LAB.md`](./XDNA2_EXEC_CMD_LAB.md) · [`XDNA2_NEVER_AGAIN.md`](./XDNA2_NEVER_AGAIN.md)

---

## 1. Architecture: Cmd BO envelope vs instruction/txn BO

```text
Userspace (dragon-npu or XRT)
  ├─ Instruction BO   bytes = txn control code / patched ctrltext   ← CONTENT OPAQUE
  ├─ Data BOs         IFM / WTS / OFM / intermediate / ctrlpkt…
  └─ Cmd BO           ERT envelope only:
        header (STATE | EXTRA_CU_MASK | COUNT | OPCODE)
        + CU mask(s)
        + amdxdna_cmd_start_npu {
              u64 buffer;        // device addr of Instruction BO
              u32 buffer_size;   // bytes
              u32 prop_count;    // leading PROPERTY dwords only (XRT shim: usually 0)
              u32 prop_args[];   // properties THEN kernel args (IFM/WTS/OFM paddrs)
          }
        │
        ▼  DRM_IOCTL_AMDXDNA_EXEC_CMD  (L1 + L3 gated on lab)
Kernel AIE2 (force_cmdlist=true by default)
  ├─ aie2_cmdlist_fill_slot  →  cmd_chain_slot_{dpu|npu} in ctx private cmd_buf
  │     inst_buf_addr ← sn->buffer
  │     inst_size     ← sn->buffer_size
  └─ mailbox MSG_OP_CHAIN_EXEC_{DPU|NPU}
        │
        ▼ Firmware CERT
  Executes txn ops at inst_buf_addr  (ISA not in public GPL headers)
```

**Critical honesty:** a correctly packed START_NPU Cmd BO is **not** NPU compute. Real compute needs a **device-valid instruction blob** at `buffer` / `inst_buf_addr`. Public headers document the **envelope** only.

CREATE_HWCTX UMQ/log BOs are **queue infrastructure**, not the txn instruction BO.

There is **no** separate uAPI `AMDXDNA_BO_INSTRUCTION` CREATE_BO type; instruction is a XRT use-flag / allocation policy. Driver types: SHARE / DEV_HEAP / DEV / CMD.

---

## 2. Known vs opaque (GPL mine + project)

| Known (GPL / public / dragon-npu) | Opaque / still research |
|-----------------------------------|-------------------------|
| Packed `amdxdna_cmd` header bitfields | **Txn binary format** (CERT opcode stream) |
| ERT opcodes/states (`START_NPU=20`, CHAIN, PREEMPT, …) | How ONNX/MLIR becomes `ml_txn` / ctrltext |
| START_NPU / CHAIN / PREEMPT / DPU **envelopes** | Legal `prop_args` layouts per model / firmware gen |
| CU mask COUNT rules (`get_payload` / `get_cu_idx`; zero mask → `-EINVAL`) | Instruction BO alignment / residency contract beyond “addr+size” |
| AIE2 fill_slot routing (`PARTIAL_ELF` vs legacy DPU slot) | When `EXEC_NPU_TYPE_PARTIAL_ELF` vs full ELF is required |
| EXEC_CMD + WAIT_CMD uAPI | End-to-end GEMM correctness on silicon |
| L2 null-guards on fill_slot path | Minimal known-good GEMM txn for Ryzen AI 300 (17f0) |
| `XDNA2_INSTRUCTION_BLOB` / `_HEX` load → CREATE_BO fill → `xdna_addr` | Capture of known-good XRT instruction BO (operator) |
| Pure numerical verify helpers (`decide_ran_on_npu`) | Known-good txn that writes OFM matching host golden |
| Lab OFM path (`XDNA2_LAB_OFM=1`): IFM/WTS/OFM + `prop_count=0` kernel args + decide wire | True silicon Pass without partner blob |
| Pure weight EXEC bind plan (`select_lab_matmul_weight` + `WeightExecBind` stub lifecycle) | Silicon OFM match with known-good txn |
| Timed path model WTS (`XDNA2_LAB_OFM=1` + `XDNA2_LAB_WEIGHT=1` + `XDNA2_LAB_WEIGHT_MODEL`) fail-closed MVP | Product Pass still needs known-good npu4 blob |
| IR-native host_ref (`evaluate_host_ir` = lower + host_graph) | Still host-only (never `ran_on_npu`) |
| Offline extract script + VE2 stage with `NOT_AIE2_KNOWN_GOOD` | VE2 nop.elf is **not** AIE2/17f0 known-good |
| Health field `txn_op_idx` proves FW walks “txn ops” | Documented txn opcode ISA |

### VE2 Runner `nop.elf` (research only — not Pass)

Staged xdna-driver `shim_ve2/Runner/latency/nop.elf` + `validate.xclbin` are **VE2 edge**
fixtures (`ve2_ops`, different FW). Offline copy via
`scripts/extract_instruction_offline.sh --stage-ve2-nop` writes gitignored sources +
`PROVENANCE.md` with **`NOT_AIE2_KNOWN_GOOD=1`**. Never auto-wire as `lab_instr.bin`
without `--force-lab-instr`, and never claim Strix `1022:17f0` silicon Pass from VE2.

### OFM / prop_count=0 (Phase 3.5 lab)

XRT shim_test **always** sets `instruction_prop_count = 0` for START_NPU and packs
IFM/WTS/OFM as **kernel args** in the flexible `prop_args[]` array (PARTIAL_ELF
layout: `u64 3`, `u64 0`, `u32 0`, IFM, WTS, OFM, `0`, `0` paddrs). Do **not** set
`prop_count = prop_args.len()` for I/O bind — that marks every dword as properties.

Dragon-npu:

| API | Role |
|-----|------|
| `pack_start_npu_kernel_args` | Pure PARTIAL_ELF kernel-arg pack |
| `build_ert_start_npu_cmd_bo_with_kernel_args` | Envelope with **prop_count=0** |
| `build_ert_start_npu_cmd_bo` | Legacy: all dwords as properties (empty/tests) |
| `lab_decide_from_ofm_bytes` | OFM bytes → ones golden → `decide_ran_on_npu` |
| `lab_decide_from_ofm_bytes_with_golden` | OFM vs **model** host golden (requires golden len = m·n) |
| `select_lab_matmul_weight` / `WeightExecBind` | Pure plan + live IFM/WTS/OFM (stub-tested; WeightExecBind not RAII) |
| `timed_matmul_direct` + `XDNA2_LAB_OFM=1` | Optional create/fill/read OFM; ones IFM/WTS default; `ran_on_npu` only if decide true |
| `+ XDNA2_LAB_WEIGHT=1` + `XDNA2_LAB_WEIGHT_MODEL` | Model WTS + ones IFM golden; **requires LAB_OFM**; MVP refuses fused/alpha≠1/bias; full alloc zero |

Default (LAB_OFM unset): empty kernel args, hard-false `ran_on_npu`.

Sources: `amdxdna_ctx.h`, `aie2_message.c`, `aie2_msg_priv.h`, shim `exec_buf.h` / `io.cpp` (GPL tree under `~/Projects/kernel/xdna-driver`); dragon-npu `src/backends/amd_xdna2/direct.rs`.

---

## 3. Host inventory (horizon lab, 2026-07-15)

| Item | Finding |
|------|---------|
| **XRT** | **2.21.75** via pacman (`xrt 1:2.21.75-10`); Apache-2.0 AND GPL-2.0-only |
| **Tools** | `/usr/bin/xrt-smi`, `xclbinutil`, `xrt-runner`, `xrt-replay`, `xrt-tracer` present; classic `xbutil` name absent |
| **Headers / libs** | `/usr/include/xrt/`, `/usr/lib/libxrt_*.so.2.21.75` — **not** `/opt/xilinx` layout |
| **xrt-plugin-amdxdna** | Present (`libxrt_driver_xdna.so.2.21.75`, `xrt-plugin-amdxdna 1:2.21.75-2`) — **LicenseRef-AMD-Proprietary AND Apache-2.0** |
| **BO dump flag** | Stock shim `m_dump_arg_bos = false` hardcoded — no `/tmp/BO_DUMPS.*` without rebuild/patch |
| **`local_shim_test_data`** | **Missing** on host (nop/vadd/gemm ELFs for npu3/npu4 not present) |
| **xdna-driver clone** | Present; **`xrt/` submodule empty** without `submodule update` |
| **amdxdna** | OOT hardened L2+L3 under `updates/`; **`exec_cmd_allow=N`** |
| **NPU** | `/dev/accel/accel0`, PCI `1022:17f0` (Strix / Ryzen AI 300) |
| **dragon-npu** | Load hook only; no capture API; `xrt-bridge` research stub (no libxrt link) |
| **Repo fixtures** | Host-graph JSON only; **no** instruction binaries in git |

---

## 4. Capture pathways A / B / C (license notes)

### Pathway A — offline extract (preferred first; **no EXEC**)

| Step | Notes |
|------|--------|
| `xclbinutil --info` / section dump on a known `.xclbin` | Apache XRT tool; may yield AIE partition / PDI-like sections — **not always** runtime-patched ctrltext |
| Offline `xrt::elf` / `xrt::module` over a known-good **ELF** | Best path if partner/open ELF exists; ELF *is* the instruction container in shim tests |
| `ml_txn.bin` → aiebu `blob_instr_transaction` → ELF (shim `txn_file2elf`) | Needs test data + aiebu; still offline until device paddr patch |
| Synthetic envelope only (`build_ert_start_npu_cmd_bo`) | MIT project code — **not** known-good txn |

**License:** XRT userspace Apache-2.0 OK for tooling. Do **not** reverse-engineer proprietary plugin `.so`. Do not commit Windows RyzenAI overlays without redistribution grant. Firmware `amdnpu/*.sbin` is **not** an instruction fixture.

### Pathway B — offline ELF materialize + feed dragon-npu load hook

```text
known-good ELF / patched ctrltext file
        → XDNA2_INSTRUCTION_BLOB=/path/to.bin
        → load_instruction_blob_from_env
        → CREATE_BO(Shmem) + write_bo_bytes + get_bo_info.xdna_addr
        → build_ert_start_npu_cmd_bo(addr, size, prop_args, cu_mask)
```

Still **no** dragon-npu EXEC unless operator opens L1+L3. See `scripts/capture_instruction_blob_checklist.md`.

### Pathway C — live XRT capture (operator lab; **not** dragon-npu L1)

Requires temporarily `exec_cmd_allow=1`, test ELF/xclbin, preferably shim rebuild with `m_dump_arg_bos=true`. Submits via **XRT → driver EXEC_CMD**. Restore `exec_cmd_allow=0` after.

Treat captured dumps as **lab-only** until license/provenance review; default CI must not depend on them.

---

## 5. Mapping to dragon-npu hooks

| Hook | Location | Role |
|------|----------|------|
| `decode_instruction_hex` / `load_instruction_blob_from_{path,env}` | `direct.rs` | Pure/FS load; env `XDNA2_INSTRUCTION_BLOB` / `_HEX` |
| `write_bo_bytes` / `fill_cmd_bo_with_payload` | `direct.rs` | Host fill before EXEC (required) |
| `build_ert_start_npu_cmd_bo` | `direct.rs` | Offline envelope; points at inst `xdna_addr` |
| `submit_and_wait_command` | `direct.rs` | L1-gated EXEC + finite WAIT |
| `timed_matmul_direct` | `direct.rs` | Full path; statuses `ok_submit_wait_*`; **`ran_on_npu`** only via `XDNA2_LAB_OFM=1` + OFM decide |
| `lab_io` / `lab_decide_from_ofm_bytes` | `lab_io.rs` | Pure pack + OFM bytes → decide; IoBoSet create/zero/readback |
| `max_abs_diff_f32` / `verify_f32_allclose` / `decide_ran_on_npu` | `numerical.rs` | Pure Phase 3.5 verify policy (wired through lab_io when LAB_OFM=1) |
| `host_matmul_f32_ones` / `host_matmul_f32_ones_vec` | `direct` / `numerical` | Host golden for all-ones GEMM contract |
| `xrt_bridge` | `xrt_bridge.rs` | Research stub — never capture, never EXEC |
| Staging dir | `testdata/xdna2_cmd/` | Gitignored binaries; README only in git |

Operator inventory: `bash scripts/inventory_txn_sources.sh`  
Operator capture procedure: `scripts/capture_instruction_blob_checklist.md`

---

## 6. What blocks `ran_on_npu`

Product flag `TimedOpResult.ran_on_npu` stays **false** until **all** of:

| # | Criterion | Status (this sprint) |
|---|-----------|----------------------|
| 1 | L1 open (`xdna2-dangerous-exec-cmd` + `XDNA2_ALLOW_EXEC_CMD=1`) + lab L3 `exec_cmd_allow=1` | Operator only; default closed |
| 2 | CREATE_HWCTX + host-filled Cmd BO | Done (bring-up) |
| 3 | Known-good instruction/txn blob → device BO → non-zero START_NPU `buffer` | **Blocked** — no cleared blob on host/CI |
| 4 | EXEC_CMD + WAIT OK | Infrastructure done; silicon single-shot not recorded |
| 5 | **Numerical verify** of device output vs host golden | **Lab wire done** (`XDNA2_LAB_OFM=1`); default path hard-false; needs known-good txn for true Pass |

`decide_ran_on_npu` encodes the pure policy:

- `false` if `host_golden` or `device_or_candidate` is **empty** (empty OFM must never Pass)
- `false` if `!submit_wait_ok`
- `false` if `!had_instruction_blob`
- `false` if `!real_ioctl_path`
- `false` if allclose fails (length mismatch / non-finite / tol)
- `true` only if all of the above pass

Structured reasons: `decide_ran_on_npu_reason` → `RanOnNpuBlockReason`.  
L1/L3 gates are **outside** this pure helper — set `submit_wait_ok` only after real gated EXEC+WAIT.

**Envelope-only or inst-blob submit without verify must never set Pass.**  
See `docs/XDNA2_DIRECT_CAPABILITIES.md` Phase 3.5 section.

---

## 7. Next lab steps (single-shot only when blob exists)

1. Run `bash scripts/inventory_txn_sources.sh` (read-only).  
2. Follow Pathway A: obtain Apache/MIT-cleared ELF or partner `local_shim_test_data` for **npu4 / 17f0**.  
3. Materialize ctrltext → file under **gitignored** `testdata/xdna2_cmd/` (or `/tmp`).  
4. Set `XDNA2_INSTRUCTION_BLOB` without opening L1; confirm load path + START_NPU packing unit tests.  
5. Only then: [`XDNA2_EXEC_CMD_LAB.md`](./XDNA2_EXEC_CMD_LAB.md) single-shot under triple gate + finite timeout.  
6. After wait: bind/read OFM BO → `verify_f32_allclose` → `decide_ran_on_npu` → optional product flip.  
7. Restore `exec_cmd_allow=0`; unset `XDNA2_ALLOW_EXEC_CMD`.

**Do not** loop empty/scaffold payloads. **Do not** claim GEMM Pass from `ok_submit_wait_*` alone.

---

## 8. Bottom line

1. Cmd BO = **pointer envelope**; Instruction BO = **opaque txn bytes**.  
2. Host has full XRT 2.21 + plugin, but **no ready known-good instruction binary** and **no stock dump path**.  
3. Best safe next work: offline ELF/xclbin materialize + feed `XDNA2_INSTRUCTION_BLOB`.  
4. Pure numerical policy + lab OFM wire unit-tested; product `ran_on_npu` still hard-false by default until `XDNA2_LAB_OFM=1` + known-good blob + OFM match.  
5. Never commit proprietary reverse dumps into default CI fixtures.

*Research + pure helpers only; no EXEC issued by this document.*
