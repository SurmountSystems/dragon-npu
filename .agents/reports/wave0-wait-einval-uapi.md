# Wave W0 O1 — WAIT EINVAL / driver uAPI deep dive

**Date:** 2026-08-11
**Mode:** read-only research (report only). No product edits. No live. No unlock mint.
**Honesty:** `invent_pass=0` · **not C-ready** · **not `ran_on_npu`** · Pass = No

**Goal:** name whether product status `wait_cmd_einval` on the skip-CU shape
(`hwctx=1 seq=0 timeout_ms=5000 request=0xc0106449 Invalid argument`) has a
**fixable userspace product bug** vs FW/driver-only residual.

**Evidence sources:** `direct.rs` WAIT/EXEC packing; staged driver
`amdxdna_ctx.c` / `aie2_ctx.c`; shim wait path; live shots `1f761bf4` /
flexml CONFIG_CU; offline dig `49feb5d2`; T3 classify honesty.

---

## 1. What userspace does on WAIT_CMD

### Pack path (product)

| Step | Behavior | Where |
|------|----------|--------|
| Create | `CREATE_HWCTX` → `handle` used as wait `hwctx`; **`syncobj_handle` is logged then discarded** (not stored on `HardwareContext`) | `direct.rs` create_hwctx ~1750–1776, `HardwareContext` ~2772–2783 |
| Build | START_NPU envelope + host-fill Cmd BO (`BufferType::Cmd`) | timed_matmul path ~5713–5744 |
| Submit | `DRM_IOCTL_AMDXDNA_EXEC_CMD`; kernel writes `req.seq`; userspace returns that seq | `submit_command` ~2132–2150 |
| Wait | `amdxdna_drm_wait_cmd { hwctx, timeout: timeout_ms, seq }` via `DRM_IOCTL_AMDXDNA_WAIT_CMD` = **`0xc010_6449`** | `wait_command` ~2199–2225 |
| Pair helper | `submit_and_wait_command`: L1-gated EXEC + finite wait; refuses `timeout_ms=0` | ~3944–3961 |
| Default budget | `DEFAULT_WAIT_CMD_TIMEOUT_MS = 5000` / env `XDNA2_WAIT_TIMEOUT_MS` | lab + timed path |
| Error text | Always embeds field names: `WAIT_CMD failed (hwctx=…, seq=…, timeout_ms=…, request=0x…): {os error}` | wait_command Err arm |

Struct (matches staged uAPI `amdxdna_accel.h`):

```text
struct amdxdna_drm_wait_cmd {
  __u32 hwctx;    // context handle from CREATE_HWCTX
  __u32 timeout;  // ms; 0 = infinite (product refuses 0 without opt-in)
  __u64 seq;      // from EXEC_CMD out
};
// DRM_IOWR cmd id 9, size 16 → 0xc010_6449
```

### Skip-CU live shape (`1f761bf4`, axes locked by TDD)

| Axis | Value |
|------|--------|
| status | `wait_cmd_einval` |
| errno | Invalid argument / os error **22** |
| hwctx | **1** |
| seq | **0** |
| timeout_ms | **5000** |
| request | **0xc0106449** |
| config_cu / pdi | **absent** |
| userspace | aiebu+rela×12, lab classic 1688 |
| total_s | ≈**0.17** (no long attach) |
| journal window | **no** IO_PAGE_FAULT, **no** `aie2_set_cmd_timeout` |

Same WAIT EINVAL class on three spent flexml CONFIG_CU attaches (with overlay
noise / ~62 s wall for attach), so residual is **not** “forgot CONFIG_CU” alone.

### Classification honesty (already landed, T3)

`classify_submit_wait_error` now maps real EINVAL → `wait_cmd_einval` **before**
timeout class, so the field name `timeout_ms=` cannot force false
`wait_timeout`. That was a **userspace honesty** bug; it is fixed. It does
**not** remove EINVAL from the driver.

---

## 2. Likely EINVAL causes (with evidence)

### Driver ground truth (staged tree)

**Ioctl entry** `amdxdna_drm_wait_cmd_ioctl` → `amdxdna_cmd_wait(client, hwctx, seq, timeout)`:

| Condition | Return | Evidence |
|-----------|--------|----------|
| `ops->cmd_wait` missing | `-EOPNOTSUPP` | `amdxdna_ctx.c` ~799–800 |
| `xa_load(ctx_xa, hwctx)` miss (bad / destroyed handle) | **`-EINVAL`** | ~805–808 |
| AIE2 wait path | see below | `aie2_cmd_wait` |

**AIE2 wait** (`aie2_ctx.c` ~991–1012):

```text
out_fence = aie2_cmd_get_out_fence(ctx, seq);
if (!out_fence) {
  XDNA_ERR(..., "Invalid syncobj or sequence number");
  return -EINVAL;   // ← product os error 22 class
}
// else dma_fence_wait_timeout → 0 ok, or -ETIME on timeout
```

`aie2_cmd_get_out_fence`: `drm_syncobj_fence_get(ctx->syncobj)` then
`dma_fence_chain_find_seqno(&fence, seq)`. NULL fence / failed lookup →
EINVAL path.

**True wait budget expiry is `-ETIME`**, not EINVAL. Product maps etimedout /
“timed out” → `wait_timeout`. Live residual is **not** that class.

**First job seq is 0** (`job->seq = ctx->submitted++` starting at 0). Historical
lab evidence: `EXEC_CMD+WAIT seq=0 accepted` (D3c era, then ERT Error / APP_LOAD).
So **seq=0 alone is not an invalid argument**.

### Ranked hypotheses for live `wait_cmd_einval`

| # | Hypothesis | Fixable in product packing? | Evidence |
|---|------------|-----------------------------|----------|
| H1 | **Driver cannot resolve out-fence for (hwctx, seq)** (“Invalid syncobj or sequence number”) after EXEC returned seq | **No** as a struct/timeout bug; maybe deeper lifecycle | Driver only EINVAL on fence miss / bad ctx; live always seq=0 after successful EXEC; wait fails immediately |
| H2 | **Wrong ioctl / struct size / request code** | Would be yes if true | Unit + live use `0xc0106449` matching `DRM_IOWR` size 16; docs warn against wrong `0x4010_6449` IOW copies. Historical WAIT **accepted** on same packing → **ruled out** as sole root |
| H3 | **timeout_ms=5000 too short** | No (and wrong lever) | EINVAL is not ETIME; skip-CU `total_s≈0.17`; flexml `submit_wait` immediate EINVAL while wall ~62 s is CONFIG_CU attach, not 5 s wait |
| H4 | **seq=0 illegal** | No | Driver first job uses 0; historical accept; `dma_fence_chain_find_seqno` treats seqno 0 specially (no search fail) |
| H5 | **Destroyed / wrong hwctx** | Unlikely on stock path | Timed path waits **before** `destroy_hwctx_and_owned_bos`; live always `hwctx=1` after create |
| H6 | **Cmd BO type wrong** | Unlikely | Kernel maps CREATE `AMDXDNA_BO_CMD` → internal **SHARE**; submit looks up SHARE; EXEC succeeds and returns seq |
| H7 | **FW job abort / TDR / async free races fence visibility** | Driver/FW residual | CONFIG_CU lives: journal `aie2_set_cmd_timeout` / optional IO_PAGE_FAULT **while** same wait_cmd_einval; skip-CU: **silent journal** but same status → residual **persists without overlay** |
| H8 | **Product never uses CREATE_HWCTX `syncobj_handle` wait** (XRT prefers drm_syncobj wait when syncobj valid; WAIT_CMD only as fallback) | **Possible product gap / parity**, **not proven root of EINVAL** | Shim `hwq.cpp` wait_command: if syncobj ≠ invalid → `wait_cmd_syncobj`, else WAIT_CMD. Product drops `syncobj_handle` from `HardwareContext` and always WAIT_CMD. WAIT_CMD is still valid uAPI and previously completed |

### Isolation table (class vs overlay)

| Unlock | Overlay | status | total_s | Journal noise |
|--------|---------|--------|---------|---------------|
| `2ae69036` | flexml 2x4x4 | wait_cmd_einval | ~62 | timeout/backtrack, no IO_PAGE_FAULT |
| `8352882b` / `efd44f35` | other flexml | wait_cmd_einval | ~62 | + IO_PAGE_FAULT bands |
| **`1f761bf4`** | **skip CU** | **wait_cmd_einval** | **~0.17** | **none in window** |

**Finding:** EXEC residual class is **WAIT_CMD EINVAL**, independent of CONFIG_CU.
Overlay can add MMU/timeout journal and wall time; it is not the sole cause.

---

## 3. Named product bug?

### Verdict: **no** (for fixable WAIT packing / seq / timeout as root of live EINVAL)

| Claim | Result |
|-------|--------|
| Wrong WAIT struct / ioctl request | **No** — matches uAPI; historical WAIT success |
| Infinite vs finite timeout policy | **No** — 5000 ms finite; infinite would be 0 (refused) |
| seq=0 always wrong | **No** — first job and historical accept |
| CONFIG_CU skip as sole root | **No** — skip-CU still wait_cmd_einval |
| Classifier false `wait_timeout` | **Was yes (T3); fixed** — honesty only, invent_pass=0 |

**Residual ownership for Pass:** still **driver/FW / load material / job completion**, not a one-line userspace WAIT pack fix. Product userspace path is green (real_ioctl, aiebu+rela, create/submit plumbing). WAIT EINVAL means the **wait completion path rejected the wait** (fence/ctx), not that OFM allclose failed after a clean wait.

### Optional product **gap** (not named “the bug,” not Pass lever alone)

- Capture and retain `syncobj_handle` from CREATE_HWCTX.
- Research XRT-parity completion: `DRM_IOCTL_SYNCOBJ_WAIT` / timeline wait on that handle with seq as timepoint (shim `wait_cmd_syncobj`), **or** prove WAIT_CMD path remains correct on stock host with journal `Invalid syncobj or sequence number`.
- Do **not** treat implementing syncobj wait alone as Pass; still need ERT Completed + OFM allclose via `decide_ran_on_npu`.

---

## 4. Honesty footer

- **invent_pass = 0**
- **not_c_ready = 1**
- **not_ran_on_npu = 1**
- Axes / this dig alone ≠ Pass ≠ proven single root
- No unlock mint; first residual unlock remains CONSUMED (`1f761bf4` and priors)
- No re-live under CONSUMED; no sticky ALLOW

---

## 5. Recommended O2 follow-up (if any)

| Priority | O2 slice | Kind | Suggested red test name / contract |
|----------|----------|------|-------------------------------------|
| **P0 offline** | Document driver EINVAL map in pure tests / note (hwctx miss vs fence miss vs ETIME) so residual docs cite code paths | TDD/docs | `wait_cmd_einval_driver_axes_map_hwctx_vs_fence_vs_etime` — pure fixture: message tokens distinguish wait_cmd_einval vs wait_timeout; optional soft note `driver_einval_fence_or_hwctx=1 etime_is_timeout=1` always invent_pass=0 |
| **P1 offline** | Store `syncobj_handle` on `HardwareContext` + format in last_apply detail (no live wait change required) | product observability | `create_hwctx_records_syncobj_handle_in_context` |
| **P2 research** | Offline design for optional syncobj wait path (XRT parity); unit-test arg packing only until new unlock | research | `syncobj_wait_arg_pack_matches_shim_timepoint_seq` |
| **Not O2** | Re-live skip-CU / re-live spent flexml / bump WAIT ms alone / invent Pass | frozen | — |

**Highest-value residual next (outside pure WAIT packing):** load-accepted /
residual-matched program material that can reach live ERT Completed + non-zero
OFM allclose. WAIT axes dig is **closed as “no packing root”** for W0 O1.

---

## 6. File map

| Path | Role |
|------|------|
| `src/backends/amd_xdna2/direct.rs` | WAIT/EXEC pack, classify_submit_wait_error, timed path |
| `src/backends/amd_xdna2/silicon_contract.rs` | `parse_wait_cmd_einval_axes`, residual split |
| `kernel/dkms/staging/.../amdxdna_ctx.c` | wait ioctl → cmd_wait |
| `kernel/dkms/staging/.../aie2_ctx.c` | aie2_cmd_wait / get_out_fence / submit seq |
| `kernel/dkms/staging/.../src/shim/hwq.cpp` | XRT prefers syncobj wait |
| `.agents/reports/live-shot-1f761bf4.md` | skip-CU evidence |
| `.agents/reports/impl-offline-wait-einval-material-49feb5d2.md` | axes TDD |
| `.agents/reports/impl-t3-wait-cmd-einval-classify.md` | honesty fix history |
