# Direct NPU capability checklist (frozen interface)

Contract for colibrì tools and dragon-npu bring-up. IDs are stable strings from
`DirectCapability::all_ids()` / `AmdXdna2Backend::direct_capability_ids()`.

## Checklist

| ID | Meaning | Host status (2026-07-13, 1022:17f0 + amdxdna, FW 1.1.2.64) |
|----|---------|--------------------------------------------------------------|
| `open` | Open `/dev/accel/accelN` | **Pass** |
| `get_info` | Typed GET_INFO (FW, power, TOPS, AIE, sensors) | **Pass** |
| `create_bo` | CREATE_BO (Shmem / DevHeap / Cmd) | **Pass** (DevHeap: one per client fd) |
| `sync_bo` | SYNC_BO | **Pass** |
| `create_hwctx` | CREATE_HWCTX (+ 64 MiB DevHeap mmap + valid `qos_p`) | **Iterate** — qos_p + `#[repr(C)]` QosInfo + tile×4 fixed; sizes wired via `create_hwctx_with_sizes`. Re-probe after driver reload if prior EXEC_CMD oops wedged amdxdna |
| `destroy_hwctx` | DESTROY_HWCTX + GEM_CLOSE | **Wired** (tracked UMQ/log BOs closed on destroy) |
| `submit` | EXEC_CMD ioctl path | **Permanently gated** — feature `xdna2-dangerous-exec-cmd` + `XDNA2_ALLOW_EXEC_CMD=1` (see `docs/XDNA2_NEVER_AGAIN.md`). Scaffold oopsed driver 2026-07-13. **L2+L3 materials** (null-guards + open fail-fast + lab `exec_cmd_allow=0`): `kernel/patches/0001-…` + `0002-…` — **not** claimed installed on host until after clean boot/DKMS. |
| `timed_gemm` | Timed matmul / buffer op with JSON timings | **API present** — default stops at `hwctx_ok` / `hwctx_blocked`; never fakes Pass; EXEC_CMD opt-in |

## Probe baseline

```bash
cargo run --features amd-xdna2 --example xdna2_probe
cargo run --features amd-xdna2 --example npu_direct_bench -- --checklist
cargo run --features amd-xdna2 --example npu_direct_bench -- --shape q_a
```

## CREATE_HWCTX requirements (from public xdna-driver)

Source: `amdxdna_drm_create_hwctx_ioctl` / `aie2_ctx_init` (GPL-2.0, amd/xdna-driver):

1. `ext == 0` and `ext_flags == 0` (else **-EINVAL**).
2. **`qos_p` must be a valid userspace pointer** — driver always `copy_from_user`s `amdxdna_qos_info` (zeroed defaults OK).
3. `num_tiles != 0`; prefer multiples of AIE2 `core.row_count` (often **4**).
4. Client must own exactly one **64 MiB** `AMDXDNA_BO_DEV_HEAP`, **mmap'd** so heap UVA is valid (else ENOENT / EINVAL “Invalid dev heap userptr”).
5. UMQ/log BOs optional on KMQ devices.

## EXEC_CMD payload (research)

- Public headers define ioctl + BO types only.
- Driver uses ERT-style command state / `OP_USER` / cmdlist path (`force_cmdlist=true`).
- Full firmware **txn op** encoding is **not** documented in redistributable headers.
- Scaffold: `build_exec_cmd_payload_scaffold()` + `ExecCmdPayloadHeader` (LE header words only).

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
| `XDNA2_HWCTX_UMQ` / `LOG` | Pin BO sizes (also API: `create_hwctx_with_sizes`) |

## Honesty rule

Do **not** report microbench Pass from probe-only or scaffold submit. Gate needs
`ran_on_npu=true` with real device time for product shapes.

## Driver wedge recovery

If CREATE_HWCTX hangs after a scaffold EXEC_CMD experiment:

**First check whether the driver is already wedged** (D-state open waiters, high
module refcnt). See the honesty table in
[`kernel/patches/README.md`](../kernel/patches/README.md) — **Install honesty
(wedged vs clean host)**. While wedged, **do not thrash** `rmmod`/`modprobe`;
prefer reboot (or the recovery docs). Reloading only works when the module is
unused and not stuck.

```bash
# Only if diagnose shows no D-state / unload is possible; may require root.
# If rmmod hangs or refcnt is high → reboot instead.
sudo modprobe -r amdxdna && sudo modprobe amdxdna
# or reboot
```

Then re-run `npu_direct_bench --checklist` **without** `XDNA2_ALLOW_EXEC_CMD` and
**without** the `xdna2-dangerous-exec-cmd` feature (legacy `XDNA2_TRY_SUBMIT` is ignored).

Optional after **clean boot**: install L2+L3-patched `amdxdna` from
[`kernel/patches/README.md`](../kernel/patches/README.md) (apply **0001 then 0002**)
so scaffold EXEC_CMD returns errno instead of oopsing and open fails fast when
unhealthy (userspace double gate stays on regardless; lab module default
`exec_cmd_allow=0`).
