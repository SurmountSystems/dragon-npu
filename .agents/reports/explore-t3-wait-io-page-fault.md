# Explore report: T3 offline delta after live WAIT EINVAL + IO_PAGE_FAULT

**Date:** 2026-08-10
**Mode:** Read-only codebase exploration (no product edits).
**Unlock context:** Live residual after `C_UNLOCK=11bc9016` sandwich (operator evidence in chat).
**Honesty:** invent_pass=0 · **no** product Pass · **no** `ran_on_npu` · unlock ≠ Pass

---

## Plain English summary

The live shot finished with `status=wait_timeout`, detail showing **WAIT_CMD failed** with **os error 22 Invalid argument**, `timeout_ms=5000`, `total_s≈63`, `config_cu=1`, PDI 283840, aiebu+rela OK, and journal **IO_PAGE_FAULT** at `0x2fff8000…` plus **aie2_set_cmd_timeout** (DPU PC `0xffffffff`, TXN OP ID `0xd`).

**Highest-value offline finding:** the product **mis-classifies** this residual. Almost every WAIT_CMD error string includes the field name `timeout_ms=…`, so `classify_submit_wait_error` matches substring `"timeout"` and labels **any** WAIT failure (including **EINVAL**) as `wait_timeout`. That is a pure userspace honesty bug. Fixing it turns false “wait timeout” into an honest **WAIT EINVAL** (or FW/MMU residual with journal), which is what this shot actually is.

Increasing `XDNA2_WAIT_TIMEOUT_MS` or outer budget alone will **not** fix Pass: `submit_wait_s≈0` means the wait ioctl returned **immediately**, not after 5s.

---

## Investigation answers (1–5)

### 1. How WAIT_CMD is packed; why EINVAL; 5000 vs outer 120s

| Piece | Fact | Path |
|-------|------|------|
| uAPI struct | `hwctx: u32`, `timeout: u32` (ms; 0=infinite), `seq: u64` — **16 bytes** | `direct.rs` `amdxdna_drm_wait_cmd` |
| Ioctl | `DRM_IOCTL_AMDXDNA_WAIT_CMD` = `0xc010_6449` (`DRM_IOWR`, size 16) | `direct.rs` ioctl consts + unit tests |
| Call | After EXEC_CMD returns `seq`, lab calls `wait_command(fd, hwctx, seq, timeout_ms)` | `submit_and_wait_command` ~3767–3783 |
| Default WAIT | `DEFAULT_WAIT_CMD_TIMEOUT_MS = 5000` | `direct.rs` ~3641 |
| Env | `XDNA2_WAIT_TIMEOUT_MS` (lab_exec default 5000; must stay finite) | `lab_exec_once.sh` `WAIT_MS`, timed path ~4716–4717 |
| Outer vs WAIT | Outer `timeout(1)` / `--timeout-s` (recipe ≥120s after prior 30s kill) wraps the **whole** cargo child. WAIT is a **separate** finite ioctl budget (5s default). They are not the same. | recipe / `lab_exec_once.sh` |
| Infinite refuse | `timeout_ms=0` refused unless `XDNA2_WAIT_ALLOW_INFINITE=1` | `check_finite_wait_timeout` / `check_wait_timeout_or_infinite_opt_in` |

**Error format (always embeds the field name):**

```text
WAIT_CMD failed (hwctx=…, seq=…, timeout_ms=5000, request=0xc0106449): … Invalid argument (os error 22)
```

**Classifier (bug):**

```4005:4028:src/backends/amd_xdna2/direct.rs
fn classify_submit_wait_error(err: &DirectError) -> &'static str {
    let s = err.to_string().to_ascii_lowercase();
    if s.contains("timeout") || s.contains("etimedout") || s.contains("etime") {
        "wait_timeout"
    } else if … {
        …
    } else if … || s.contains("invalid argument") || s.contains("einval") {
        "payload_unsupported"
    } else {
        "submit_wait_failed"
    }
}
```

Because `"timeout_ms=5000"` contains `"timeout"`, **WAIT EINVAL never reaches the `invalid argument` / `einval` arm**. Unit test `classify_submit_wait_error_eio_and_map_host` only checks bare strings without `timeout_ms=` — so the live wire shape is **untested**.

**Likely real kernel reasons for WAIT EINVAL (research, not proven on this host):**

- Job already aborted / invalid `seq` or `hwctx` after FW fault (page fault / cmd timeout).
- Driver reject of wait parameters (less likely if pack/size unit tests match uAPI).
- Struct/ioctl const wrong would usually fail every wait; live EXEC+WAIT path has worked before (2026-07-18 finished with ERT Error, not wait EINVAL).

**Silicon-mladf path:** `lab_exec_once --silicon-mladf` + `XDNA2_LAB_PDI_BLOB` → CONFIG_CU after CREATE_HWCTX → full IFM/WTS/OFM/CTRL fill → aiebu rela → START_NPU → `submit_and_wait_command`. Progressive last_apply stamps `lab_partial_pre_submit` immediately before submit when pin/write_la is on.

### 2. `submit_wait_s ≈ 0` while `total_s ≈ 63`

**Yes — that pattern means wait failed immediately (or near-immediately), while earlier stages burned wall time.**

Code:

```5561:5564:src/backends/amd_xdna2/direct.rs
let d0 = Instant::now();
let sw = submit_and_wait_command(submission, 0, &cmd, timeout_ms);
let ds = d0.elapsed().as_secs_f64();
submit_wait_s = Some(ds);
```

- A **true** 5s WAIT timeout would put `submit_wait_s ≈ 5` (plus small overhead).
- `submit_wait_s ≈ 0` + os error 22 ⇒ ioctl returned **EINVAL without waiting**.
- `total_s ≈ 63` is wall clock from timed path start: CREATE_HWCTX + **CONFIG_CU** (write **283840** PDI Dev BO) + DevHeap + I/O BO create/fill (WTS 174080 etc.) + rela write + Cmd BO + destroy. That easily dominates; it is **not** “waited 63s.”

**Do not** treat status `wait_timeout` as evidence that `XDNA2_WAIT_TIMEOUT_MS=5000` was too short for this shot.

### 3. IO_PAGE_FAULT vs BO `xdna_addr` / Shmem paddrs in rela

| Source | Pattern | Notes |
|--------|---------|--------|
| Historical C2 lock (2026-07-18) | instr `0x4020000`; IFM/WTS/OFM/CTRL **host UVAs** `0x7fda…` | `c2_live_io_paddrs_locked()` in `silicon_contract.rs` |
| This shot (operator) | DevHeap `0x4000000`, instr `0x4068000`; IFM/WTS/OFM host-looking | Same **class** as prior: **Dev instr** + **Shmem I/O PASID UVAs** |
| Journal fault | `0x2fff8000…` | **Not** in DevHeap low range (`0x40xxxxx`) and **not** like high PASID UVAs (`0x7f…`) |

**Rela path (live):** after I/O BO create+pin, `patch_aiebu_elf_ctrltext_with_bos_ex` writes IFM/WTS/OFM/CTRL paddrs into `.ctrltext` (sym 1–4), then Dev instr BO is filled with patched ctrltext. Shmem I/O is intentional (`create_io_bos` uses `BufferType::Shmem` + `pin_bo_for_device_access` for PASID).

**Conclusion:** “Host-looking” IFM/WTS/OFM paddrs are **not** automatically wrong (XRT/lab historical path uses the same shape). The fault address `0x2fff8000` is the interesting anomaly: likely a **bad reloc / tile-relative / unmapped** access during EXEC, correlated with `aie2_set_cmd_timeout` and a **failed or aborted wait**, not with “Shmem vs Dev” alone.

Pure offline follow-on (optional, not this T3): parse `io_paddrs` / `bo_diag` from last_apply detail and journal fault addr; unit-lock “fault not equal to any reported BO base” vs “fault within BO span.”

### 4. Kernel arg layout count4 vs xrt3; caller m,n,k vs plan

| Item | Fact |
|------|------|
| Default layout | `LabKernelArgLayout::Count4Ctrl` (`XDNA2_LAB_KERNEL_ARG_LAYOUT` unset → count4) |
| xrt3 | `n_bos=3`, IFM/WTS/OFM, two trailing zeros; ctrl pin-only (not in classic args) |
| Prior HITL | count4 and xrt3 both still hit **APP_LOAD** `0x3000003` (env matrix **closed** as Pass lever) |
| Bench shape | checklist / default often `timed_matmul(1,1,1,"f32")` |
| Mladf pack | `create_and_fill_lab_io_bos_for_pack_mode` uses **plan** M/N/K (e.g. 1,256,1152 staged key); `report_dims` overwrites TimedOpResult m/n/k; BO sizes from plan, **not** 1×1×1 f32 |
| Host ref | skipped for mladf (ones golden invalid) |

**Caller 1×1×1 is OK for silicon-mladf I/O sizing** when pack mode is mladf. Layout count4 vs xrt3 is **not** the highest-value T3 for this residual (already closed as sole Pass lever offline).

### 5. What XRT / public stacks do for WAIT and BO map

From in-repo construction map and lab docs (not a live XRT run this explore):

- Rela: peel `.ctrltext`, apply `.rela.dyn` with live buffer paddrs (`patch_ctrl_code` parity) — lab already does this.
- I/O: host-visible BOs with PASID (Shmem class) is normal for XRT elf_io style tests; instr often device/cacheable (lab uses **Dev** instr).
- Wait: finite wait on submission sequence / fence; lab maps that to **WAIT_CMD(seq)** with finite ms. Public stacks do **not** treat “field name timeout_ms in error text” as a timeout class.

No XRT source tree is required for the recommended T3; the bug is local string classification.

---

## Recommended single T3 delta

### Title

**T3 — Honest WAIT residual: stop false `wait_timeout` on WAIT_CMD EINVAL (timeout_ms field false match)**

### Goal

When WAIT_CMD returns **os error 22 / Invalid argument**, status must **not** be `wait_timeout`. Prefer a dedicated token (e.g. `wait_cmd_einval`) or at least the existing `payload_unsupported` / `submit_wait_failed` path that does **not** steal the timeout arm via `timeout_ms=`. Optionally attach journal soft tokens (`io_page_fault`, `aie2_set_cmd_timeout`) as research detail only — **never** invent Pass.

This unsticks residual analysis for the 11bc9016 shot and any future shot where wait fails for non-timeout reasons while the error string still contains `timeout_ms=`.

### Why this beats other candidates for **one** T3

| Candidate | Moves Pass? | Honest residual? | Agent offline? |
|-----------|-------------|------------------|----------------|
| **Classifier fix (this T3)** | Indirect (correct next lever) | **Yes — primary** | Pure TDD |
| Bump WAIT / outer only | **No** (`submit_wait_s≈0`) | No | N/A |
| Re-open count4 vs xrt3 | Low (closed before) | No new honesty | Env only |
| Invent Dev I/O instead of Shmem | Speculative; historical C2 used Shmem UVAs | Unproven | Larger HITL risk |
| Full rela re-architecture | Speculative | Research | Multi-session |

### Files to touch

| File | Change |
|------|--------|
| `src/backends/amd_xdna2/direct.rs` | Rewrite `classify_submit_wait_error` so **real** wait timeouts (ETIMEDOUT / `etime` / message phrases like `wait timed out`) win **without** matching the field name `timeout_ms=`. Prefer: match **errno / phrase** for EINVAL **before** loose `"timeout"`, or strip/ignore `timeout_ms=<digits>` before substring search. Add status token **`wait_cmd_einval`** (recommended) for WAIT-path Invalid argument / os error 22. Keep `wait_timeout` only for genuine timed waits. |
| `src/backends/amd_xdna2/direct.rs` (tests) | Extend / replace `classify_submit_wait_error_eio_and_map_host` with **live-shaped** strings. |
| Optional: `src/backends/amd_xdna2/silicon_contract.rs` | Soft note or residual class hook when status is `wait_cmd_einval` + detail/journal has `IO_PAGE_FAULT` → research class (e.g. incomplete / mmu fault), still `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`. **Do not** map to locked APP_LOAD. |
| Optional docs one-liner | `docs/XDNA2_EXEC_CMD_LAB.md` or HITL recipe: “status `wait_timeout` only means real wait ETIMEDOUT; EINVAL is not a timeout.” |

### TDD red test names (suggested)

1. **`classify_submit_wait_error_live_wait_cmd_einval_not_timeout`**
   Input (exact live shape):
   `WAIT_CMD failed (hwctx=1, seq=0, timeout_ms=5000, request=0xc0106449): Invalid argument (os error 22)`
   Assert: status **≠** `wait_timeout`; assert **`wait_cmd_einval`** (or agreed token).

2. **`classify_submit_wait_error_true_etimedout_still_wait_timeout`**
   e.g. `WAIT_CMD failed (... timeout_ms=5000 ...): Connection timed out (os error 110)` or driver `etimedout` — still `wait_timeout`.

3. **`classify_submit_wait_error_timeout_ms_field_alone_not_timeout`**
   Message contains `timeout_ms=5000` but body is `permission denied` / `eperm` → **`exec_cmd_perm`**, not `wait_timeout`.

4. **(optional pure)** **`submit_wait_s_near_zero_with_wait_cmd_einval_detail_note`** — pure helper or detail format when `submit_wait_s < 0.05` and error is EINVAL: append `wait_returned_immediately=1` so last_apply readers do not invent a 5s wait.

**Red order:** write tests first; observe fail on current `classify_submit_wait_error` (live-shaped EINVAL → `wait_timeout`); then minimal product edit; green same filters.

**Commands (implementer):**

```bash
cargo test --features amd-xdna2 --lib classify_submit_wait_error
cargo clippy -p dragon-npu --features amd-xdna2 --lib -- -D warnings
cargo fmt -p dragon-npu
```

### Acceptance (offline)

- Live-shaped WAIT EINVAL → **not** `wait_timeout`.
- True ETIMEDOUT still → `wait_timeout`.
- Soft honesty trio preserved on any residual note path: invent_pass=0, not_c_ready=1, not_ran_on_npu=1.
- **No** live EXEC, **no** L3, **no** ALLOW=1 in tests, **no** Pass claim, **no** second unlock mint.

---

## Explicit non-goals

- Do **not** invent product Pass or `ran_on_npu`.
- Do **not** mint or reopen unlocks (`11bc9016` is operator one-shot; agent offline only for this T3).
- Do **not** invent capture bytes under `testdata/xdna2_cmd/external_known_good/`.
- Do **not** claim IO_PAGE_FAULT is fixed by classifier work alone.
- Do **not** re-open count4/xrt3 env matrix as the main deliverable.
- Do **not** raise default WAIT to 120s as a “fix” for this residual (`submit_wait_s≈0`).
- Do **not** empty-reconfirm offline gates without a named code delta.

---

## Confidence and risks

| | |
|--|--|
| **Confidence (false wait_timeout bug)** | **High** — code path + unit-test gap + live error shape line up. |
| **Confidence (WAIT EINVAL root cause on silicon)** | **Medium-low** — consistent with aborted job after IO_PAGE_FAULT / aie2_set_cmd_timeout, but kernel source not traced this explore. |
| **Confidence (Shmem paddrs are the fault)** | **Low** — historical C2 used host UVAs for I/O; fault `0x2fff8000` is elsewhere. |
| **Pass impact of T3 alone** | **Honesty / diagnosis only** — required so next offline/live lever is real. Pass still needs FW accept + non-zero OFM allclose. |

**Risks**

1. Renaming status may break scripts that only look for `wait_timeout` — grep consumers and update soft classify if needed.
2. Over-tight timeout matching might miss real driver messages that say only “timeout” without etimedout — keep true timeout phrases tested green.
3. Operator may still paste old last_apply with false status; note in residual that **pre-T3** `wait_timeout` + os 22 is misclassified.

---

## After T3 (not in this slice)

Once residual is honestly `wait_cmd_einval` + journal IO_PAGE_FAULT:

1. Offline pure: fault-addr vs `io_paddrs` / `bo_diag` span audit from last_apply (named research).
2. Human: share full last_apply JSON + journal snippet for that shot.
3. Further live only under residual policy (new unlock if 11bc9016 consumed; agent does not open L3).

---

## Key code anchors (absolute)

- `/home/hunter/Projects/ai/dragon-npu/src/backends/amd_xdna2/direct.rs` — `amdxdna_drm_wait_cmd`, `wait_command`, `submit_and_wait_command`, `classify_submit_wait_error`, timed submit block ~5530–5800, `DEFAULT_WAIT_CMD_TIMEOUT_MS`
- `/home/hunter/Projects/ai/dragon-npu/src/backends/amd_xdna2/lab_io.rs` — Shmem I/O + pin; kernel arg layouts
- `/home/hunter/Projects/ai/dragon-npu/src/backends/amd_xdna2/txn.rs` — `patch_aiebu_elf_ctrltext_with_bos_ex`
- `/home/hunter/Projects/ai/dragon-npu/src/backends/amd_xdna2/silicon_contract.rs` — journal soft taxonomy; `c2_live_io_paddrs_locked`
- `/home/hunter/Projects/ai/dragon-npu/scripts/lab_exec_once.sh` — outer timeout vs `XDNA2_WAIT_TIMEOUT_MS`

---

**Report complete. Ready for implementer TDD on classifier honesty only.**
