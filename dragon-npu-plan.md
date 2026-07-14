# Plan: Reuse dragon-npu direct XDNA2 as colibrì NPU submission layer

**Location:** this file lives in the dragon-npu repo (`dragon-npu-plan.md`).  
**Related colibrì copy:** `colibri/.agents/plans/plan-dragon-npu-direct-xdna2.md` (same content when written).  
**Date:** 2026-07-12

## Context

**Goal:** Make colibrì’s AMD NPU path use **dragon-npu’s lightweight direct amdxdna stack** (`/dev/accel` + ioctls) as the real submission layer — not stock ORT / VitisAI EP (unavailable on Arch) and not as a permanent dependency on vendor-only XRT ELF graphs if we can own the path.

**Why now**
- M1 ADR on the colibrì host is **Fail** under the ORT-first spike (no VitisAI EP).
- Device + driver are real: `1022:17f0`, `amdxdna`, FW `1.1.2.64`.
- **dragon-npu** (this repo, branch `rust`) already has hardware-validated open + GET_INFO + BO ioctl scaffolding and an explicit philosophy: **no heavy XRT/IRON/ORT for the core path**.
- colibrì already has tools scaffolding (`npu_probe`, `benchmark_npu_shapes`, `npu_xrt`) that need a real `matmul`/run backend.

**Constraints**
- colibrì **default build stays pure C, zero NPU deps**.
- Product engine hook (`backend_npu` / `glm.c`) only after design gates (Pass-product-candidate); tools can advance earlier.
- dragon-npu is **outside** the colibrì repo; integration must be explicit (path dep / submodule / extract crate).
- dragon-npu **cannot execute models/GEMM on silicon yet** — `CREATE_HWCTX` still **EINVAL** on real hardware; command payload format is the research gap.
- Gate metrics stay honest: microbench ≥1.3× @ S=1 **incl transfer** for product shapes; no false Pass from probe-only.

**Non-goals (this plan)**
- Shipping colibrì `glm.c` NPU dispatch before microbench + PROFILE gates.
- Depending on Ubuntu-only Ryzen AI Software / VitisAI EP as the primary path.
- Reverse-engineering FastFlowLM proprietary kernels as the product API.
- Multi-backend vtable / CUDA+NPU dual-link (colibrì design K5/K9).

**Assumptions**
- Primary host remains Arch/Garuda with in-tree `amdxdna` + user in `render`.
- We will continue hardware iteration on the same `1022:17f0` class device.
- Prefer **one submission story** long-term: direct amdxdna (this crate). XRT/ELF may remain a **fallback/smoke** path in colibrì only if useful.

## Approach

**Recommended path: dual-repo, tools-first, unblock execution in dragon-npu, then wire colibrì.**

1. **Treat this repo as the submission authority** for “talk to XDNA2 without ORT.”
2. **Finish the missing direct-path primitives here** (hwctx lifecycle → command BO payload → minimal buffer GEMM or vendor-known txn) until something runs on silicon with measured time.
3. **Expose a thin, stable surface** colibrì tools can call (Rust CLI and/or `cdylib` C ABI later) — do not drag `tract-onnx` into colibrì’s default build.
4. **colibrì** replaces/augments `XrtGemmBackend` with a **DirectXdna2Backend** that shells to / links that surface for timed matmul when available; keep skip taxonomy and gate JSON.
5. **Only after Pass-micro (and PROFILE floor)** revisit colibrì `backend_npu` mirroring CUDA — still opt-in `NPU=1`.

```text
┌─────────────────────────────┐     ┌──────────────────────────────────┐
│ colibrì c/tools             │     │ dragon-npu (Rust, amd-xdna2)     │
│ npu_probe / microbench      │────▶│ AmdXdna2Backend + direct::       │
│ Direct backend adapter      │     │ open, BO, hwctx, EXEC_CMD, GEMM  │
└─────────────────────────────┘     └──────────────────────────────────┘
         │                                         │
         │  later (gated)                          │
         ▼                                         ▼
┌─────────────────────────────┐            /dev/accel/accelN
│ backend_npu + matmul_qt     │            amdxdna ioctl uAPI
└─────────────────────────────┘
```

**Not X (rejected for primary path)**
- **ORT VitisAI EP on Arch** — no package; already failed colibrì M1.
- **XRT-only forever** — fine for optional smoke; contradicts this project’s direct philosophy and still needs precompiled ELFs (IRON).
- **Inlining full dragon-npu into colibrì tree immediately** — couples ONNX compiler + NPU bring-up; prefer path dependency / extracted `dragon-npu-xdna` crate later.
- **Engine hook before timed GEMM** — violates measurement-first design.

## Critical files (this repo)

| Path | Why |
|------|-----|
| `src/backends/amd_xdna2.rs` | Backend + Real arm + public direct helpers |
| `src/backends/amd_xdna2/direct.rs` | Direct uAPI, BO/hwctx/submit scaffolding |
| `examples/xdna2_probe.rs` | Hardware bring-up harness |
| `tests/xdna2_real_hardware.rs` | HW integration tests |
| `PLAN.md` | Live status (CREATE_HWCTX blocker) |
| `docs/XDNA2_REAL_EXECUTION.md` | Readiness narrative |
| `docs/XDNA2_HARDWARE_TESTING.md` | HW owner guide |

## Critical files (colibrì — consumer)

| Path | Why |
|------|-----|
| `c/tools/npu_xrt/backend.py` | Current run stub / skip schema to dual-backend |
| `c/tools/benchmark_npu_shapes.py` | Gate metrics + `--npu-backend` |
| `c/tools/npu_probe.py` | Discovery; add direct-stack fields |
| `c/tools/NPU_ADR.md` | Re-measure after real NPU totals |
| `grok-design-doc-a7f3c2e1.md` | Gates M0–M2; amend K4 ORT-first → direct |
| `c/backend_cuda.h` / `c/glm.c` | Pattern for later `backend_npu` only |

## Reuse

| Symbol / module | Path | How |
|-----------------|------|-----|
| `AmdXdna2Backend::new_with_real_stub` | `src/backends/amd_xdna2.rs` | Entry for real device work |
| `DirectSubmission` / `RealDirectSubmission` | `src/backends/amd_xdna2/direct.rs` | Extend until GEMM/run works |
| `probe_accel_devices` / open helpers | `src/backends/mod.rs` | Align probe language with colibrì |
| colibrì `XrtGemmBackend` skip dict | colibrì `npu_xrt/backend.py` | Mirror return schema for Direct backend |
| colibrì product shapes | `benchmark_npu_shapes.py` | Same gate columns |
| CUDA `matmul_qt` fail-fast | colibrì `glm.c` | Mirror only at engine milestone |

## Steps

Ordered; each step independently reviewable.

### In this repo (dragon-npu) — do first

1. **Inventory & pin interface**  
   - Freeze a minimal “Direct NPU capability” checklist: open, GET_INFO, create_bo, sync_bo, create_hwctx, destroy_*, submit, timed_gemm.  
   - Document current pass/fail on this host via `cargo run --features amd-xdna2 --example xdna2_probe`.  
   - **Depends on:** nothing.

2. **Unblock hwctx lifecycle**  
   - Resolve `CREATE_HWCTX` EINVAL (BO types/sizes/flags, destroy leaks, ioctl nr candidates already partially explored).  
   - Success: create + destroy hwctx on real silicon without leak/EBUSY death spiral.  
   - **Depends on:** 1.

3. **Command payload research spike**  
   - Characterize EXEC_CMD / txn format from kernel headers, public xdna-driver tests, any redistributable validate archives, or minimal known-good blob.  
   - Prefer **documented or reverse-engineered-from-public** sources; record provenance.  
   - Success: one non-dummy command completes without abort (even no-op or latency probe).  
   - **Depends on:** 2.

4. **Minimal timed buffer op**  
   - Implement smallest useful compute: e.g. copy, fill, or fixed-shape GEMM if payload allows.  
   - Public API sketch: `timed_matmul(m,n,k, dtype) -> timings` or `submit_buffer_op(...)`.  
   - **Depends on:** 3.

5. **Stable caller surface for colibrì tools**  
   - Option A (fast): example/CLI JSON stdout (`npu-direct-bench --shape q_a ...`).  
   - Option B (better later): `cdylib` C ABI with no tract dependency in the link line (split crate `dragon-npu-xdna-sys` / `dragon-npu-direct`).  
   - Prefer A for first green microbench; B before engine.  
   - **Depends on:** 4.

### In colibrì — after timed op exists

6. **Tools adapter**  
   - Add `--npu-backend direct` (and `auto`: direct → xrt → ort → skip).  
   - Implement `DirectXdna2Backend` parallel to `XrtGemmBackend` with same timing fields.  
   - Probe: report `direct_stack` (binary present / device open / hwctx ok).  
   - **Depends on:** 5.

7. **Re-run gate + ADR amendment**  
   - Fill microbench tables with real NPU totals; re-evaluate Fail / Pass-micro / Pass-product-candidate.  
   - Amend design K4: primary spike = **direct amdxdna (dragon-npu)**; ORT EP optional.  
   - **Depends on:** 6.

8. **Engine path (only if Pass-product-candidate)**  
   - `backend_npu` C API wrapping the **C ABI** from step 5B (not Python).  
   - `matmul_qt` allow-list hook; CUDA mutex; pure-C default untouched.  
   - **Depends on:** 7 candidate outcome.

## Risks

| Risk | Mitigation |
|------|------------|
| **CREATE_HWCTX never clears on this kernel/FW** | Cap research time; fall back to XRT+IRON ELF for tools-only Pass-micro; keep direct path as long-term |
| **Command payload remains closed** | Use public xdna-driver test ELFs / validate archives if redistributable; document legal/source |
| **tract-onnx pulled into colibrì** | Never link full crate into default `glm`; CLI subprocess or split direct-only crate |
| **Two half-working stacks (XRT + direct)** | `auto` preference order; ADR states primary |
| **Product PROFILE still <10%** | Even with GEMM wins, outcome may stay Pass-micro·Fail-product — acceptable early stop |
| **Unsafe ioctl surface** | Keep unsafe only in `direct`; colibrì tools stay safe Python/C |
| **Repo split friction** | Path dependency + documented versions; optional git submodule later |

## Verification

**This repo (hardware host)**
```bash
cd ~/Projects/ai/dragon-npu
cargo test --features amd-xdna2
cargo test --features amd-xdna2 --test xdna2_real_hardware -- --nocapture
cargo run --features amd-xdna2 --example xdna2_probe
# After step 4:
# cargo run --features amd-xdna2 --example timed_gemm -- --m 1 --n 256 --k 1024
```

**colibrì tools**
```bash
cd ~/Projects/surmount/colibri/c
python3 tools/npu_probe.py
python3 tools/benchmark_npu_shapes.py --npu-backend direct --preset tiny --batch-sizes 1
python3 -m unittest discover -s tests -p 'test_*.py' -q
make check   # pure C still green; no NPU link
```

**Gate**
- At least one **product** shape S=1 with real `median_total_s` and `speedup_cpu_over_npu_total >= 1.3` → Pass-micro eligible.
- Warm PROFILE fractions still required for Pass-product-candidate (colibrì ADR rules).

## Open questions

1. **Crate packaging:** path-dep from colibrì on this repo vs extract `dragon-npu-direct`?
2. **First compute primitive:** true GEMM for gate, or latency/copy probe first then GEMM as 4b?
3. **XRT role after direct works:** delete from colibrì, or keep optional `--npu-backend xrt`?
4. **Kernel headers source of truth:** system `/usr/include` amdxdna uAPI vs vendored xdna-driver copies?

## Implementation todos

| Id | Step |
|----|------|
| `impl:direct-checklist` | Step 1 inventory + probe baseline on this host |
| `impl:hwctx-unblock` | Step 2 CREATE_HWCTX/destroy on silicon |
| `impl:payload-spike` | Step 3 EXEC_CMD / txn format |
| `impl:timed-op` | Step 4 minimal timed buffer/GEMM API |
| `impl:caller-surface` | Step 5 CLI JSON and/or cdylib split |
| `impl:colibri-adapter` | Step 6 `--npu-backend direct` + probe fields |
| `impl:adr-remeasure` | Step 7 microbench + ADR + design K4 note |
| `impl:engine-gated` | Step 8 only if Pass-product-candidate |

## Honest current status (2026-07-13)

| Capability | Status |
|------------|--------|
| Open `/dev/accel/accelN` | Works |
| GET_INFO (FW, power, TOPS, …) | Works on 1022:17f0 |
| CREATE_BO / SYNC_BO / GET_BO_INFO | Real ioctls; GEM_CLOSE for destroy |
| CREATE_HWCTX | **Fixed qos_p + repr(C) QosInfo + tile×4; re-probe** (was NULL qos_p / wrong defaults) |
| EXEC_CMD useful payload | Scaffold only (header words); txn research gap |
| GEMM / model run on silicon | API `timed_matmul` + JSON CLI; not Pass until hwctx+payload |
| colibrì integration | JSON surface ready (`npu_direct_bench`); adapter not started |

**Docs:** `docs/XDNA2_DIRECT_CAPABILITIES.md`. **CLI:** `cargo run --features amd-xdna2 --example npu_direct_bench -- --checklist`.

**Next concrete work:** confirm CREATE_HWCTX on silicon after qos fix; if green, payload spike; then colibrì step 6.
