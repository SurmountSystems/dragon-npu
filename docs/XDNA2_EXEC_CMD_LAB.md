# Phase 2 lab: first safe EXEC_CMD + WAIT

**Goal:** one EXEC_CMD that **completes or fails cleanly** (errno / wait return),
never oops, never D-state wedge. This document is **infrastructure + procedure**.
It does **not** claim silicon compute is proven.

**Status (repo):** Phase 2 **APIs ready** (`wait_command`, `submit_and_wait_command`,
`fill_cmd_bo_with_payload`, unit tests for refuse-when-gated). **Silicon single-shot
is operator-run only** — do not mark “proven on silicon” until a gated single-shot
is actually recorded.

**Preferred single-shot wrapper:**
[`scripts/lab_exec_once.sh`](../scripts/lab_exec_once.sh) — **default dry-run**
(diagnose + plan only). `--apply` requires `--i-accept-wedge-risk` **and**
`--blob PATH`, refuses WEDGED/high refcnt, trap-restores `exec_cmd_allow=0` +
unsets `XDNA2_ALLOW_EXEC_CMD`. Apply success **≠** product `ran_on_npu` (need
known-good blob + OFM numerical verify; optional `XDNA2_LAB_OFM=1`).

Related: [`XDNA2_NEVER_AGAIN.md`](./XDNA2_NEVER_AGAIN.md) ·
[`XDNA2_CMD_PAYLOAD.md`](./XDNA2_CMD_PAYLOAD.md) ·
[`XDNA2_DIRECT_CAPABILITIES.md`](./XDNA2_DIRECT_CAPABILITIES.md) ·
[`XDNA2_REAL_EXECUTION.md`](./XDNA2_REAL_EXECUTION.md) ·
[`XDNA2_TXN_RESEARCH.md`](./XDNA2_TXN_RESEARCH.md)

---

## Preconditions (hard)

Do **not** enable any EXEC_CMD path until all of the following hold:

| Check | How |
|-------|-----|
| Not wedged | `bash scripts/diagnose_amdxdna.sh` → not `WEDGED_*` (prefer **LIKELY_OK**) |
| Hardened L2+L3 on lab host (recommended) | `bash scripts/verify_amdxdna_hardened_host.sh` exit 0 |
| Module refcnt idle | diagnose / `lsmod` — no stuck open waiters; **refcnt ideally 0** when idle |
| No D-state `amdxdna_drm_open` | diagnose; if present → **stop**, reboot/recover, do not thrash open |
| Phase 1 envelope **host-filled** into Cmd BO | `build_ert_start_npu_cmd_bo` + `fill_cmd_bo_with_payload` (mmap write). **Never** EXEC_CMD on empty/unfilled BO |
| Userspace L1 gates remain intentional | Feature + env only for this single session |

Stock (unpatched) amdxdna still NULL-deref risk on garbage/empty cmdlist (2026-07-13).
Lab host with L2 null-guards is **safer**, not a license to spam EXEC_CMD.

---

## Enable order (temporary)

**Order matters. Restore defaults after one shot.**

```text
1. Preflight: diagnose + (lab) verify_amdxdna_hardened_host
2. Build with L1 feature:
     cargo build --features amd-xdna2,xdna2-dangerous-exec-cmd --example …
3. Runtime L1:
     export XDNA2_ALLOW_EXEC_CMD=1
4. Kernel L3 (lab module only; temporary):
     sudo modprobe -r amdxdna   # only if unload is clean — else reboot first
     sudo modprobe amdxdna exec_cmd_allow=1
     # Prefer: options line for one session only; do NOT commit exec_cmd_allow=1
     # into permanent repo modprobe.d defaults.
5. Single-shot under timeout (see below)
6. Restore kernel gate:
     sudo modprobe -r amdxdna && sudo modprobe amdxdna exec_cmd_allow=0
     # or reboot; confirm exec_cmd_allow is N / 0 via verify script
7. Unset XDNA2_ALLOW_EXEC_CMD
```

**Never** leave `exec_cmd_allow=1` as the permanent host default in this repo’s
documented configs. Operator temporary override only.

### Timeout wrappers

```bash
# Always wrap live device work. Outer timeout(1) + finite WAIT_CMD.
export XDNA2_WAIT_TIMEOUT_MS=5000
# Do NOT set XDNA2_WAIT_ALLOW_INFINITE unless you truly need infinite WAIT.

# Live path (when L1 open): probe builds START_NPU, host-fills Cmd BO, then
# submit_and_wait once — or refuses EXEC_CMD if fill fails (no empty fallback).
timeout 30s env XDNA2_ALLOW_EXEC_CMD=1 \
  cargo run --features amd-xdna2,xdna2-dangerous-exec-cmd --example xdna2_probe

# Library path (same fill + submit_and_wait policy when L1 open):
#   timed_matmul_direct / npu_direct_bench — still ran_on_npu=false for
#   envelope / inst-blob-without-verify (statuses ok_submit_wait_envelope /
#   ok_submit_wait_with_inst_blob) unless XDNA2_LAB_OFM=1 + OFM decide true.
# Prefer --bo-roundtrip for safe timings without EXEC.

# Preferred one-shot wrapper (default dry-run; no EXEC):
bash scripts/lab_exec_once.sh
bash scripts/lab_exec_once.sh --apply --i-accept-wedge-risk --blob /path/to.bin
# Apply exports XDNA2_LAB_OFM=1. npu_direct_bench exits 0 only if ran_on_npu;
# otherwise exit 2 → lab script exit 4 after restore (not product Pass).
```

- Prefer `timeout(1)` **and** finite `submit_and_wait_command(..., timeout_ms)`.
- Prefer **`lab_exec_once.sh`** over ad-hoc enable/restore so gates cannot stay open.
- `--timeout-s` / `--wait-timeout-ms` must be **≥ 1** (0 refused).

### L3 `exec_cmd_allow` + finite WAIT (Phase 6 note)

| Layer | Gate | Failure mode if wrong |
|-------|------|------------------------|
| **L1 userspace** | Feature `xdna2-dangerous-exec-cmd` + `XDNA2_ALLOW_EXEC_CMD=1` | `exec_cmd_is_enabled` false → no EXEC_CMD ioctl |
| **L3 kernel (hardened)** | `exec_cmd_allow=1` temporary | `-EPERM` / status `exec_cmd_perm` even if L1 open |
| **WAIT** | `DEFAULT_WAIT_CMD_TIMEOUT_MS` (5000) or `XDNA2_WAIT_TIMEOUT_MS` | `wait_timeout` if firmware hangs; **never** leave infinite wait as default |

Both L1 and L3 must be open for a single-shot EXEC on a hardened lab host. Finite WAIT is independent of EXEC_CMD enablement (`wait_command` is not L1-gated) but lab procedures still wrap with `timeout(1)`.

Public constant: `DEFAULT_WAIT_CMD_TIMEOUT_MS` in `direct.rs` (re-exported from `amd_xdna2`).
- `timeout_ms == 0` is **infinite** in the driver; **`submit_and_wait_command` refuses 0**.
- Raw `wait_command(..., 0)` also refuses unless `XDNA2_WAIT_ALLOW_INFINITE=1`.
- **Never** loop EXEC_CMD on failure (no tight retry, no empty-scaffold matrix).
- **Never** submit without `fill_cmd_bo_with_payload` success (`Command.payload` alone
  is **not** written to the kernel BO by `submit_command`).

---

## Preferred payload path (Phase 1 builders + host fill)

| Prefer | Avoid for “default happy path” |
|--------|--------------------------------|
| `build_ert_start_npu_cmd_bo` + `fill_cmd_bo_with_payload` | Empty Cmd BO / unfilled handle + EXEC_CMD |
| Lab I/O: `build_ert_start_npu_cmd_bo_with_kernel_args` (`prop_count=0`) | Treating IFM/WTS/OFM paddrs as `prop_count=len` |
| `build_ert_cmd_chain_bo` / preempt / DPU when needed | `cmd_bo_handle: None` “fallback” submit |
| Known-good instruction BO (capture procedure below) | Claiming `ran_on_npu` from envelope-only |
| Host-filled payload only after fill success | Empty / scaffold instruction `buffer=0` loops |
| `lab_exec_once.sh` dry-run then gated apply | Leaving `exec_cmd_allow=1` permanent |

**Instruction blob capture (offline preferred):**  
[`scripts/capture_instruction_blob_checklist.md`](../scripts/capture_instruction_blob_checklist.md) ·  
[`docs/XDNA2_TXN_RESEARCH.md`](./XDNA2_TXN_RESEARCH.md) ·  
read-only inventory: `bash scripts/inventory_txn_sources.sh`  
Staging: `testdata/xdna2_cmd/` (binaries gitignored).  
**Still refuse empty scaffold** Cmd BO / zero-size instruction submits.

Rust entry points:

```text
build_ert_start_npu_cmd_bo(...)     — offline envelope bytes (always safe)
fill_cmd_bo_with_payload(...)       — GET_BO_INFO → mmap → write → SYNC (required pre-submit)
DirectSubmission::submit_command    — EXEC_CMD only if exec_cmd_is_enabled()
DirectSubmission::wait_command      — WAIT_CMD; not L1-gated; finite timeout unless opt-in
submit_and_wait_command(...)        — L1 gate + finite wait; one shot (caller must fill BO first)
```

**Honesty:** accept + wait return ≠ `ran_on_npu`. Envelope-only START_NPU with
empty instruction buffer is research plumbing, not GEMM Pass.

---

## Userspace double gate (always on)

| Layer | Mechanism | Default |
|-------|-----------|---------|
| Compile | `xdna2-dangerous-exec-cmd` | **OFF** (not pulled by `amd-xdna2`) |
| Runtime | `XDNA2_ALLOW_EXEC_CMD=1` | **unset** |
| Kernel (L3 lab) | `exec_cmd_allow` | **0** |

```bash
# CI / default lib tests — must refuse EXEC_CMD
cargo test --features amd-xdna2 --lib
cargo test --features amd-xdna2,xdna2-dangerous-exec-cmd --lib
# Do NOT set XDNA2_ALLOW_EXEC_CMD=1 for the above.

# Operator-only ignored test (still must not be CI default):
# cargo test --features amd-xdna2,xdna2-dangerous-exec-cmd --lib \
#   hw_exec_cmd_submit_and_wait -- --ignored
# with XDNA2_ALLOW_EXEC_CMD=1 XDNA2_HW_EXEC=1 after preflight.
```

---

## Suggested single-shot checklist (operator)

1. `bash scripts/diagnose_amdxdna.sh` → clean.
2. `bash scripts/verify_amdxdna_hardened_host.sh` (lab).
3. Build with `amd-xdna2,xdna2-dangerous-exec-cmd`.
4. Temporary `exec_cmd_allow=1` (if L3 module).
5. `XDNA2_ALLOW_EXEC_CMD=1` + **one** `timeout …` process that:
   - CREATE_HWCTX
   - `build_ert_start_npu_cmd_bo` (or known-good bytes)
   - CREATE_BO(Cmd) sized to payload
   - **`fill_cmd_bo_with_payload`** (must succeed; else **stop** — no EXEC_CMD)
   - `submit_and_wait_command` **once**
   - DESTROY / GEM_CLOSE
6. Restore `exec_cmd_allow=0`; unset ALLOW; re-run diagnose.
7. Record result (errno / wait / dmesg) — **only then** claim silicon trial outcome.

Default `xdna2_probe` / `npu_direct_bench` / `timed_matmul` **never** EXEC_CMD
unless L1 is fully open. When open, they **refuse** rather than empty-submit if
Cmd BO host fill fails. Do not add permanent probe loops.

---

## If something goes wrong

1. Stop opening `/dev/accel`.
2. `bash scripts/diagnose_amdxdna.sh`.
3. Prefer reboot over thrashing `rmmod` if D-state waiters / high refcnt.
4. Policy + recovery: [`XDNA2_NEVER_AGAIN.md`](./XDNA2_NEVER_AGAIN.md),
   [`XDNA2_RECOVERY_ENGINEERING.md`](./XDNA2_RECOVERY_ENGINEERING.md).

---

## What Phase 2 does **not** claim

- [x] WAIT + submit_and_wait APIs exist and are unit-tested when gated closed.
- [x] Host fill path (`fill_cmd_bo_with_payload` / `write_bo_bytes`) before EXEC_CMD.
- [x] Probe / timed_matmul refuse empty Cmd BO submit when fill fails.
- [x] Lab procedure written (this file).
- [x] Phase 3: stage timings, host_ref, instruction-blob env hook, `bo_roundtrip` DMA path.
- [x] Phase 3.5: pure `decide_ran_on_npu` / allclose + lab OFM wire (`XDNA2_LAB_OFM=1`, fail-closed setup).
- [x] Optional model WTS: `XDNA2_LAB_WEIGHT=1` + `XDNA2_LAB_WEIGHT_MODEL` (requires LAB_OFM; fail-closed).
- [x] `scripts/lab_exec_once.sh` dry-run default; apply exports LAB_OFM=1 + trap restore;
      research NOT_AIE2 PROVENANCE blobs need `--allow-research-blob`.
- [ ] Silicon single-shot recorded on this host under full gates (operator HITL).
- [ ] Known-good txn instruction BO for real GEMM (operator capture HITL).
- [ ] Product `ran_on_npu=true` on this host (needs known-good txn writing matching OFM).
