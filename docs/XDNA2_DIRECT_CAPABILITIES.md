# Direct NPU capability checklist (frozen interface)

Contract for colibrì tools and dragon-npu bring-up. IDs are stable strings from
`DirectCapability::all_ids()` / `AmdXdna2Backend::direct_capability_ids()`.

**JSON adapter schema** (`schema_version`, exit codes, field table):
[COLIBRI_ADAPTER.md](./COLIBRI_ADAPTER.md).

## Checklist

| ID | Meaning | Host status (2026-07-15, 1022:17f0 + **hardened OOT** amdxdna 1.0.0, FW 1.1.2.64, `exec_cmd_allow=0`) |
|----|---------|--------------------------------------------------------------|
| `open` | Open `/dev/accel/accelN` | **Pass** |
| `get_info` | Typed GET_INFO (FW, power, TOPS, AIE, sensors) | **Pass** (FW 1.1.2.64, tops_max=58) |
| `create_bo` | CREATE_BO (Shmem / DevHeap / Cmd) | **Pass** (DevHeap: one per client fd) |
| `sync_bo` | SYNC_BO | **Pass** |
| `create_hwctx` | CREATE_HWCTX (+ 64 MiB DevHeap mmap + valid `qos_p`) | **Pass** on Real arm — multi-size grid (4/8 tiles, UMQ/log variants) succeeds; destroy cleans up |
| `destroy_hwctx` | DESTROY_HWCTX + GEM_CLOSE | **Pass** (create+destroy in probe/checklist) |
| `submit` | EXEC_CMD ioctl path | **Permanently gated** — L1: feature `xdna2-dangerous-exec-cmd` + `XDNA2_ALLOW_EXEC_CMD=1`; L3 lab: `exec_cmd_allow=0` on host. Scaffold oopsed stock driver 2026-07-13. Phase 2: `submit_and_wait_command` + lab doc; silicon single-shot **operator-only** (not proven by default tests). |
| `timed_gemm` | Timed matmul / buffer op with JSON timings | **API present — NPU GEMM still blocked** — default `hwctx_ok`; L1 open: START_NPU + host-fill + submit+wait → `ok_submit_wait_envelope` or `ok_submit_wait_with_inst_blob` (≠ Pass). Stage timings: `hwctx_s` / `fill_s` / `submit_wait_s`. Optional CPU `host_ref_*`. **`ran_on_npu` false** until instruction blob + numerical verify (Phase 3.5). |
| `bo_roundtrip` | Timed Shmem CREATE_BO + fill + SYNC TO/FROM + verify | **Honest silicon DMA path** — no EXEC_CMD; default gates OK. `ran_on_device_path=true` **only** on `RealDirectSubmission` verify (`bo_roundtrip_ok`). Stub in-memory → `bo_roundtrip_stub_ok` + `ran_on_device_path=false`. Product Pass also needs `real_device_open`. **Never** maps to `timed_gemm` / `ran_on_npu`. |

## Probe baseline

```bash
cargo run --features amd-xdna2 --example xdna2_probe
cargo run --features amd-xdna2 --example npu_direct_bench -- --checklist
cargo run --features amd-xdna2 --example npu_direct_bench -- --shape q_a
cargo run --features amd-xdna2 --example npu_direct_bench -- --bo-roundtrip
```

## CREATE_HWCTX requirements (from public xdna-driver)

Source: `amdxdna_drm_create_hwctx_ioctl` / `aie2_ctx_init` (GPL-2.0, amd/xdna-driver):

1. `ext == 0` and `ext_flags == 0` (else **-EINVAL**).
2. **`qos_p` must be a valid userspace pointer** — driver always `copy_from_user`s `amdxdna_qos_info` (zeroed defaults OK).
3. `num_tiles != 0`; prefer multiples of AIE2 `core.row_count` (often **4**).
4. Client must own exactly one **64 MiB** `AMDXDNA_BO_DEV_HEAP`, **mmap'd** so heap UVA is valid (else ENOENT / EINVAL “Invalid dev heap userptr”).
5. UMQ/log BOs optional on KMQ devices.

## EXEC_CMD payload (research) — next hard problem

With **hwctx create/destroy working**, the remaining blocker for real compute is the
**command BO contents**, not the ioctl plumbing.

- Public headers define ioctl + BO types only.
- Driver uses ERT-style command state / `OP_USER` / cmdlist path (`force_cmdlist=true`).
- Full firmware **txn op** encoding is **not** documented in redistributable headers.
- **Phase 1 offline complete:** packed `amdxdna_cmd` header + typed builders
  (`build_ert_start_npu_cmd_bo`, chain, preempt, DPU envelope) + unit tests —
  see **`docs/XDNA2_CMD_PAYLOAD.md`** (known vs opaque table; txn still opaque).
- Instruction **txn** bytes at `start_npu.buffer` remain the hard unknown.
- **Phase 3.5 prep (offline done):** consolidated research
  [`XDNA2_TXN_RESEARCH.md`](./XDNA2_TXN_RESEARCH.md); read-only inventory
  `bash scripts/inventory_txn_sources.sh` (**report only, always exit 0 — not a
  Pass gate**); operator capture checklist
  `scripts/capture_instruction_blob_checklist.md`; pure
  `decide_ran_on_npu` / allclose helpers exist (see Phase 3.5 section below) but
  are **not** auto-wired — product Pass still needs known-good blob + OFM readback.
- **Do not** enable L1 (`xdna2-dangerous-exec-cmd` + `XDNA2_ALLOW_EXEC_CMD`) or L3
  (`exec_cmd_allow=1`) to “try” empty scaffolds — that oopsed stock amdxdna (2026-07-13).
- Safer research paths: GPL shim reverse-eng; optional XRT capture of known-good
  instruction BOs; never claim `ran_on_npu` until verified payload + WAIT complete.
- Phase 2 **infrastructure ready** (`wait_command`, `submit_and_wait_command`, unit
  refuse-when-gated tests). Full lab procedure:
  [`XDNA2_EXEC_CMD_LAB.md`](./XDNA2_EXEC_CMD_LAB.md). Silicon single-shot **not**
  claimed unless operator-recorded.

## Env knobs (bring-up)

| Var | Role |
|-----|------|
| `XDNA2_BO_FLAGS` | CREATE_BO flags matrix |
| `XDNA2_HWCTX_EXT_FLAGS` | Must stay 0 on current driver |
| `XDNA2_HWCTX_MAX_OPC` / `UMQ_DOORBELL` | Request field overrides |
| `XDNA2_HWCTX_QOS_NULL=1` | Force qos_p=0 (expect fail; experiment only) |
| `XDNA2_HWCTX_SKIP_HEAP=1` | Skip DevHeap (expect ENOENT) |
| `XDNA2_HWCTX_TILES` / `MEM` / `UMQ` / `LOG` | Probe grid |
| `XDNA2_ALLOW_EXEC_CMD=1` | Only with Cargo feature `xdna2-dangerous-exec-cmd`; still may oops if payload is garbage |
| `XDNA2_WAIT_TIMEOUT_MS` | Finite WAIT_CMD timeout for timed_matmul / lab; default **`DEFAULT_WAIT_CMD_TIMEOUT_MS` = 5000**; `submit_and_wait` refuses 0 |
| `XDNA2_WAIT_ALLOW_INFINITE=1` | Opt-in only for `timeout_ms=0` WAIT (lab rare); default refuse infinite |
| `XDNA2_HW_EXEC=1` | Operator-only ignored HW test marker; never set in default CI |
| `XDNA2_HWCTX_UMQ` / `LOG` | Pin BO sizes (also API: `create_hwctx_with_sizes`) |
| `XDNA2_HWCTX_TILES` / `TILES_SINGLE` | Probe / timed single-shot tile count; library heuristic `suggest_hwctx_tiles` → 4 or 8 (×4) from `tops_max` when not overridden |
| `XDNA2_INSTRUCTION_BLOB` | Path to raw instruction/txn bytes for START_NPU `buffer` (optional; no proprietary default) |
| `XDNA2_INSTRUCTION_HEX` | Hex instruction bytes if no file (optional `0x`, whitespace ok) |
| `XDNA2_HOST_REF=1` | Force host f32 golden matmul baseline in `TimedOpResult.host_ref_*` (also auto for small f32 shapes) |
| `XDNA2_REAL_SUBMIT_MODE` | Phase 4–5 Real `submit_and_run`: unset = fail-closed `RealSubmitNotReady`; `host_ref` / `host_validate` = Phase 5 host f32 graph (Identity/Relu/Add/Mul/MatMul/Gemm when weights allow; not NPU); `npu_exec` = gated EXEC attempt via `timed_matmul_direct` (still not numerical Pass) |
| `XDNA2_HOST_MATMUL_K` | Optional: pin MatMul `K` when weight shape missing (Phase 5 host graph only) |
| `XDNA2_LAB_PACK` | Lab I/O pack when **`XDNA2_LAB_OFM=1`**: unset/`f32_ones` or `mladf_a16w4`. Unknown values fail-closed. See `AMD_MLADF_MATMUL_PACK.md` |
| `XDNA2_LAB_WTS_PACKED` | Path to pre-packed WTS for mladf (exact meta length, e.g. 174080; read capped 64 MiB). Prefer `tools/export_mladf_wts.sh` → `wts.bin` (vendor `export_const_params`); also pack `--wts-from` / capture |
| `XDNA2_LAB_CTRL_PACKED` | Path to control packet bin (e.g. 29664 B staged `lab_ctrl.bin`); fourth START_NPU kernel-arg BO. Prefer stage dump + `tools/validate_mladf_ctrl.sh` |
| `XDNA2_LAB_REQUIRE_CTRL` | `1`/`true`: refuse mladf fill when plan has `ctrl_bytes` but ctrl missing/empty/all-zero (fill API + pack mode); pack mode also refuses unset path. Default off for unit tests |
| `XDNA2_LAB_CTRL_AUTO_PATCH` | `1`/`true`: opt-in lab — apply meta IFM/WTS/OFM paddrs into ctrl (`size=6` `read_add_write`: existing tile offset + base) before attach; needs `XDNA2_LAB_MLADF_META`. Default off |
| `XDNA2_LAB_KERNEL_ARG_LAYOUT` | `count4` (default) or `xrt3`. Aliases: `4`→count4; `3`/`classic`→xrt3. **Unknown tokens refuse** (`lab_env_invalid`) |
| `XDNA2_LAB_INSTR_SIZE_MODE` | `content` (default): START_NPU size = min(ctrltext, bo_alloc); `bo`: BO alloc. Aliases: `alloc`/`page`→bo. **Unknown refuse** |
| `XDNA2_LAB_RELA_MODE` | `overwrite` (default) or `add` (RMW). Alias: `rmw`→add. **Unknown refuse** |
| `XDNA2_HITL_SKIP_CONTRACT` | `1`/`true`/`yes`/`on`: skip long contract (still ELF+preflight). Sticky env warned; ≠ proof of prior green. Or `--skip-offline-contract` |
| `XDNA2_LAB_REQUIRE_NONZERO_WTS` | `1`/`true`: refuse all-zero WTS at BO fill; pack mode also refuses unset `XDNA2_LAB_WTS_PACKED` (Pass readiness). **Wins over** `ALLOW_ZERO_WTS` when both set |
| `XDNA2_LAB_ALLOW_ZERO_WTS` | `1`/`true`: allow zero WTS for plumbing only when **REQUIRE is unset** (not Pass). Does **not** override REQUIRE |
| `XDNA2_LAB_OFM_GOLDEN` | OFM golden for pack-mode decide: f32 LE `m*n*4` or bf16 LE `m*n*2`. **Required** for mladf; **all-zero refused** by default. Offline: `tools/validate_mladf_ofm_golden.sh` |
| `XDNA2_LAB_ALLOW_ZERO_GOLDEN` | `1`/`true` bypasses **offline resolve/load** only (shell validator exits 3, not 0). Product `decide_ran_on_npu` **always** refuses all-zero golden (`AllZeroGolden`). Never greened by `--require-pass-assets` |
| `XDNA2_LAB_MLADF_KEY` / `XDNA2_LAB_MLADF_META` | mladf instr key / ctrl_meta JSON for BO sizes (meta sizes capped 64 MiB) |
| `XDNA2_LAB_BO_DIAG` | `1`/`true`: always attach DevHeap + instr (+ I/O) GET_BO_INFO into timed `detail`. Submit failures dump `bo_diag` even when unset. No secrets. |
| `XDNA2_PACK_WTS_FROM` | **pack_mladf_ref / stage tools only** (same as `--wts-from`); not read by Rust lab path |
| `DD_ROOT` / `AIEBU_LIB` | **export_mladf_wts.sh only** — Ryzen AI DynamicDispatch root + libaiebu path |
| `SILICON_LAB_DIR` | **scripts only** — default `testdata/xdna2_cmd/silicon_lab` for `silicon_lab_preflight` / `lab_exec_once --silicon-mladf` |

### Phase 6 APIs (efficiency / observability — not GEMM tops)

| API | Role |
|-----|------|
| `suggest_hwctx_tiles` / `AmdXdna2Backend::suggested_hwctx_tiles` | Heuristic tiles 4 or 8 from resource TOPS |
| `set_direct_power_mode` / `power_mode::*` | SET_STATE power (uAPI); operator-tested |
| `fill_weight_bos_batch` | Multi-BO fill, one submission session |
| `TimedOpResult::efficiency_notes` | Stage ratios; bo_roundtrip = DMA-only |

## Honesty rule

Do **not** report microbench **GEMM / NPU** Pass from probe-only, envelope submit, or
instruction-blob submit without numerical verify. Product `timed_gemm` Pass requires
`ran_on_npu=true` (Phase 3.5 criterion below).

**Safe timed path today:** `bo_roundtrip` / `timed_bo_roundtrip` → `ran_on_device_path`
(driver DMA only). Do not rename that to NPU compute.

### What is required for true `ran_on_npu` GEMM

All of:

1. L1 open (`xdna2-dangerous-exec-cmd` + `XDNA2_ALLOW_EXEC_CMD=1`) and lab L3 `exec_cmd_allow=1` if applicable.
2. Working CREATE_HWCTX + host-filled Cmd BO (never empty).
3. **Known-good instruction/txn blob** loaded (`XDNA2_INSTRUCTION_BLOB` / `_HEX`), device BO filled, non-zero `xdna_addr` in START_NPU.
4. EXEC_CMD + WAIT complete without error.
5. **Numerical verify** against host golden (or fixed test vector).

#### Phase 3.5 — pure verify + lab OFM wire (silicon Pass still HITL)

| Piece | Status |
|-------|--------|
| `max_abs_diff_f32` / `verify_f32_allclose` | **Implemented** (pure; unit-tested) |
| `host_matmul_f32_ones_vec` | **Implemented** (full golden vector) |
| `NumericalVerifyCriteria` / `NumericalVerifyInput` | **Implemented** |
| `decide_ran_on_npu` / `silicon_ran_on_npu_flags_ok` | **Implemented** — true **only** if non-empty buffers ∧ `submit_wait_ok` ∧ `had_instruction_blob` ∧ `real_ioctl_path` ∧ allclose (+ OFM readback for outer flags) |
| Default tolerances | `DEFAULT_VERIFY_ATOL=1e-4`, `DEFAULT_VERIFY_RTOL=1e-3` (`NumericalVerifyCriteria::default`) |
| Lab packing (`lab_io`, kernel args `prop_count=0`) | **Implemented** + unit tests |
| Wire into `timed_matmul_direct` → set `ran_on_npu` | **Lab-only:** `XDNA2_LAB_OFM=1` + OFM readback + decide; **default still hard-false** |
| Status when decide true | `ok_npu_verified` (never from envelope alone) |
| Zero OFM golden (mladf) | **Refused** unless `XDNA2_LAB_ALLOW_ZERO_GOLDEN=1` (not product Pass) |
| `tools/silicon_lab_preflight.sh` | **Implemented** — offline asset gate; `--require-pass-assets` fail-closed; no L3/EXEC |
| `scripts/lab_exec_once.sh` | **Implemented** — default dry-run; `--silicon-mladf` recipe; apply needs risk+blob |
| Known-good txn blob + non-zero golden on host | **Blocked** — capture HITL; see [`SILICON_PATH_LAB.md`](./SILICON_PATH_LAB.md) Phase C |

**Still no product Pass without a known-good blob + real non-empty device OFM compare.**
Empty golden/candidate → `decide_ran_on_npu` **false**. Envelope or
`ok_submit_wait_with_inst_blob` alone keeps **`ran_on_npu=false`**.
Dry-run green ≠ `ran_on_npu`. Host has no passwordless sudo / no golden → live L3 apply may exit 3/4.

Cross-links: [`SILICON_PATH_LAB.md`](./SILICON_PATH_LAB.md) · [`XDNA2_TXN_RESEARCH.md`](./XDNA2_TXN_RESEARCH.md) · [`XDNA2_EXEC_CMD_LAB.md`](./XDNA2_EXEC_CMD_LAB.md) · [`XDNA2_CMD_PAYLOAD.md`](./XDNA2_CMD_PAYLOAD.md) · [`XDNA2_NEVER_AGAIN.md`](./XDNA2_NEVER_AGAIN.md).

## Host install status (lab)

| Item | Horizon (2026-07-15) |
|------|----------------------|
| Module | OOT L2+L3 under `updates/amdxdna-hardened/amdxdna.ko` (DRM 1.0.0) |
| `exec_cmd_allow` | **0 / N** (modprobe.d options-only; **no** softdep on `gpu_sched`) |
| Firmware | `npu.dev.sbin.zst` → `npu_7.sbin.zst` → 1.1.2.64 |
| CREATE_HWCTX | **Pass** (see checklist above) |
| Reboot persistence | **Not proven** until post-reboot verify |

Read-only check: `bash scripts/verify_amdxdna_hardened_host.sh`  
Playbook: [`XDNA2_HARDENED_INSTALL.md`](./XDNA2_HARDENED_INSTALL.md) · Policy: [`XDNA2_NEVER_AGAIN.md`](./XDNA2_NEVER_AGAIN.md) · Payload research: [`XDNA2_CMD_PAYLOAD.md`](./XDNA2_CMD_PAYLOAD.md) · Upstream pack: [`upstream/README.md`](./upstream/README.md)

## Driver wedge recovery

If CREATE_HWCTX hangs after a scaffold EXEC_CMD experiment:

**First check whether the driver is already wedged** (D-state open waiters, high
module refcnt). See the honesty table in
[`kernel/patches/README.md`](../kernel/patches/README.md) — **Install honesty
(wedged vs clean host)**. While wedged, **do not thrash** `rmmod`/`modprobe`;
prefer reboot (or the recovery docs). Reloading only works when the module is
unused and not stuck.

```bash
bash scripts/verify_amdxdna_hardened_host.sh
bash scripts/diagnose_amdxdna.sh
# Only if diagnose shows no D-state / unload is possible; may require root.
# If rmmod hangs or refcnt is high → reboot instead.
sudo modprobe -r amdxdna && sudo modprobe amdxdna
# or reboot
```

Then re-run `npu_direct_bench --checklist` **without** `XDNA2_ALLOW_EXEC_CMD` and
**without** the `xdna2-dangerous-exec-cmd` feature (legacy `XDNA2_TRY_SUBMIT` is ignored).

Lab host already has L2+L3 under `updates/`. Other machines / after kernel upgrade:
[`XDNA2_HARDENED_INSTALL.md`](./XDNA2_HARDENED_INSTALL.md) (apply **0001 then 0002**,
rebuild for new `uname -r`). Userspace double gate stays on regardless; lab module
default `exec_cmd_allow=0`.
