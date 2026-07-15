# Colibrì adapter contract — `npu_direct_bench` JSON

Stable machine interface for the sibling **colibrì** project (and other tools) to
consume DragonNPU timed direct-path results.

**Primary entry (not the `dragon-npu` CLI):**

```bash
cargo run --features amd-xdna2 --example npu_direct_bench -- --checklist
cargo run --features amd-xdna2 --example npu_direct_bench -- --bo-roundtrip
cargo run --features amd-xdna2 --example npu_direct_bench -- --shape q_a
cargo run --features amd-xdna2 --example npu_direct_bench -- --m 1 --n 256 --k 1024 --dtype f32
```

Always prints **one JSON object on stdout**. Human diagnostics go to **stderr**.

Library type: `TimedOpResult` → `to_json_value()` in
`src/backends/amd_xdna2/direct.rs`. Constant (feature-free crate root):
`dragon_npu::TIMED_OP_JSON_SCHEMA_VERSION` (currently **1**); also re-exported
as `dragon_npu::backends::amd_xdna2::TIMED_OP_JSON_SCHEMA_VERSION` when the
feature is enabled.

Also see: [XDNA2_DIRECT_CAPABILITIES.md](./XDNA2_DIRECT_CAPABILITIES.md),
[XDNA2_NEVER_AGAIN.md](./XDNA2_NEVER_AGAIN.md).

---

## Schema version

| Field | Type | Notes |
|-------|------|--------|
| `schema_version` | integer | **1** — present on timed-op JSON and checklist / feature-disabled objects |

Bump only when removing/renaming fields or changing honesty semantics of
`ran_on_npu` / product Pass rules. Additive optional fields may keep version 1
if documented here.

---

## Timed op object (matmul / bo_roundtrip)

Produced by `TimedOpResult::to_json_value()` and extended by the example with
host metadata (`hardware_detected`, `real_device_open`, `capabilities`, …).

| Field | Type | Meaning |
|-------|------|---------|
| `schema_version` | u32 | See above |
| `op` | string | `"matmul"`, `"bo_roundtrip"`, … |
| `m` / `n` / `k` | u32 | Matrix dims; **0** for non-matmul |
| `size_bytes` | u64 or null | BO size for `bo_roundtrip` |
| `dtype` | string | e.g. `"f32"`, `"bytes"` |
| `total_s` | f64 | Wall seconds |
| `host_prep_s` | f64 | Host prep stage |
| `hwctx_s` | f64 or null | CREATE_HWCTX stage |
| `fill_s` | f64 or null | Host fill stage |
| `submit_wait_s` | f64 or null | submit+wait when attempted |
| `device_s` | f64 or null | Device SYNC / submit stage |
| `host_ref_s` | f64 or null | CPU golden matmul time |
| `host_ref_ok` | bool | CPU golden succeeded |
| `host_ref_detail` | string | Checksum / skip reason |
| `ran_on_npu` | bool | **Verified NPU compute** only — still false until numerical GEMM verify |
| `ran_on_device_path` | bool | Real ioctl DMA path verify (BO roundtrip); **not** GEMM Pass |
| `real_ioctl` | bool | RealDirectSubmission vs stub |
| `status` | string | Stable machine status (`hwctx_ok`, `bo_roundtrip_ok`, …) |
| `detail` | string | Human detail |
| `efficiency_notes` | string | Stage ratios; DMA-only wording for bo_roundtrip |

Example-added fields (may appear alongside):

| Field | Meaning |
|-------|---------|
| `hardware_detected` | Sysfs/modalias style detection |
| `real_device_open` | `/dev/accel` open succeeded |
| `capabilities` | `DirectCapability` id list |
| `timed_gemm` | `"pass"` only if `ran_on_npu`; else `"blocked_npu_compute"` / `"not_requested"` |
| `bo_roundtrip_product_pass` | `real_device_open && real_ioctl && ran_on_device_path` |
| `bo_roundtrip_note` | When product pass false |

---

## Exit code contract

| Mode | Exit 0 | Exit 1 | Exit 2 |
|------|--------|--------|--------|
| Feature missing | — | always (JSON has `feature_disabled`) | — |
| `--bo-roundtrip` | only if `real_device_open && real_ioctl && ran_on_device_path` | — | otherwise (JSON still printed) |
| timed GEMM / default shape | only if `ran_on_npu` | — | otherwise (JSON still printed; today always 2 for GEMM) |
| `--checklist` | always 0 after print (checklist is diagnostic; read `notes` / host fields) | | |
| `--help` | 0 | | |

**Never** map exit 0 to silicon GEMM Pass without `ran_on_npu=true`.

**Never** map stub in-memory BO success to device path Pass.

---

## Honesty rules (must not be weakened)

1. `ran_on_npu=true` only after known-good instruction txn + **numerical verify**
   (Phase 3.5 criteria in DIRECT_CAPABILITIES). Envelope / inst-blob submit alone
   is **not** enough.
2. `bo_roundtrip` product Pass requires open + real ioctl + `ran_on_device_path`.
3. EXEC_CMD remains double-gated (`xdna2-dangerous-exec-cmd` + `XDNA2_ALLOW_EXEC_CMD=1`).
   Default benches/examples/CLI must not enable it.
4. Do not invent CLI flags that imply unsafe enablement when they do nothing.

---

## Checklist JSON (`--checklist`)

Top-level object includes `schema_version`, `capabilities`, `host` (open, fw,
hwctx status, gemm/bo notes), `notes` (capability Pass/fail strings), `detail`.

Interpret:

- `notes.timed_gemm`: `"pass"` iff `host.timed_gemm_ran_on_npu`, else `"blocked_npu_compute"`
- `notes.bo_roundtrip`: `"pass_device_path"` only for real device DMA path
- `notes.submit`: `"permanently_gated"` unless L1 exec feature+env open

---

## Env vars (optional; colibrì may set)

| Var | Role |
|-----|------|
| `XDNA2_HOST_REF=1` | Force host golden matmul fields |
| `XDNA2_WAIT_TIMEOUT_MS` | WAIT timeout when L1 open (lab) |
| `XDNA2_REAL_SUBMIT_MODE` | Library `submit_and_run` modes — not required for npu_direct_bench |
| `XDNA2_ALLOW_EXEC_CMD=1` | **Dangerous** — only with feature `xdna2-dangerous-exec-cmd` |

Full list: [XDNA2_DIRECT_CAPABILITIES.md](./XDNA2_DIRECT_CAPABILITIES.md).

---

## What colibrì should **not** call for timed metrics

- `dragon-npu status` / `compile` / `run` — product library UX; not the timed
  direct JSON contract.
- `xdna2_probe` — human bring-up matrix; richer, not schema-stable for automation.

Use **`npu_direct_bench`** for adapter automation.
