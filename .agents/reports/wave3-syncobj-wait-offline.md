# Wave W3 O13 — syncobj wait path offline

**Date:** 2026-08-11
**Mode:** offline only. No live NPU. No unlock mint. No L3/ALLOW.
**Honesty:** `invent_pass=0` · **not C-ready** · **not `ran_on_npu`** · Pass = No

**Upstream:**
- `.agents/reports/wave0-wait-einval-uapi.md` (W0 O1)
- `.agents/reports/wave1-o2-wait-syncobj.md` (W1 O2 retain + axes)

---

## Plain English

1. XRT (staged shim `hwq.cpp`) waits on the CREATE_HWCTX **syncobj** when the
   handle is non-zero, using `DRM_IOCTL_SYNCOBJ_TIMELINE_WAIT` with the EXEC
   sequence as the timeline timepoint. If the handle is zero
   (`AMDXDNA_INVALID_FENCE_HANDLE`), it falls back to `WAIT_CMD`.
2. Product already **retains** `syncobj_handle` on `HardwareContext` (W1 O2) but
   still always issues **WAIT_CMD** in `wait_command` / `submit_and_wait_command`.
3. Live residual remains `wait_cmd_einval` (skip-CU and CONFIG_CU). That is a
   driver fence/ctx completion class, **not** proven as “forgot syncobj wait.”
4. This slice: pure decision + pack sketch + unit tests. **No** live ioctl wire.

---

## Research finding (code + staged driver/shim)

### XRT parity rule (ground truth in-tree)

| Source | Behavior |
|--------|----------|
| `kernel/dkms/staging/.../src/shim/hwq.cpp` `wait_command` | `if (syncobj != AMDXDNA_INVALID_FENCE_HANDLE)` → `wait_cmd_syncobj`; else `wait_cmd_ioctl` (WAIT_CMD) |
| `platform_host.cpp` `wait_cmd_syncobj` | `handle = ctx_syncobj_handle`, `timepoint = seq`, `timeout_ms` |
| `platform.cpp` `wait_syncobj` | `DRM_IOCTL_SYNCOBJ_TIMELINE_WAIT`, flags `WAIT_FOR_SUBMIT`, abs nsec timeout |
| uAPI | `AMDXDNA_INVALID_FENCE_HANDLE == 0` |

So: **yes, product *should* prefer syncobj timeline wait when handle ≠ 0** for
XRT parity. That is a design gap relative to the working public stack, not a
proven fix for current live EINVAL.

### Product today

| Item | State |
|------|--------|
| CREATE records `syncobj_handle` | Yes (W1 O2) |
| last_apply `syncobj=` detail | Yes |
| `wait_command` | WAIT_CMD only (`0xc0106449`) |
| `submit_and_wait_command` | EXEC + WAIT_CMD only |
| Live SYNCOBJ_TIMELINE_WAIT | **Not implemented** |

### Is live EINVAL “because WAIT_CMD instead of syncobj”?

| Claim | Verdict |
|-------|---------|
| WAIT packing / request code wrong | **No** (W0 O1; historical WAIT success) |
| seq=0 illegal | **No** (first job uses 0) |
| Product drops syncobj wait | **Yes, parity gap** |
| Syncobj wait alone would Pass | **Unproven** / forbidden claim |
| Fixable packing bug as sole root | **No** (W0 O1 closed) |

Driver `aie2_cmd_wait` still does fence lookup for WAIT_CMD; EINVAL is
“Invalid syncobj or sequence number” / bad hwctx when the out-fence is missing.
Switching userspace to timeline wait may use a **different** ioctl path that
waits the same fence chain via drm_syncobj. That is a reasonable **future** A/B
under a **new** unlock, not offline proof and not unlock justification alone.

---

## Deliverable decision

### Implement full product live path now?

**No.**

Reasons:

1. Live wire needs `DRM_IOCTL_SYNCOBJ_TIMELINE_WAIT` packing, abs-timeout
   conversion, error classify (ETIME vs EINVAL), and timed_matmul /
   `submit_and_wait` API surface changes (syncobj handle plumbing into wait).
2. Residual is still post-attach / completion class; material load remains the
   highest Pass lever.
3. No active unlock; re-live under CONSUMED is frozen.
4. Task scope: pure offline TDD if justified; stop if large/uncertain.

### Implement minimal pure offline TDD?

**Yes** (landed this report).

| API | Role |
|-----|------|
| `AMDXDNA_INVALID_FENCE_HANDLE` | uAPI `0` sentinel |
| `should_wait_via_syncobj(handle)` | pure bool (≠ 0) |
| `WaitCompletionPath` | `SyncobjTimeline` \| `WaitCmd` |
| `select_wait_completion_path` | XRT selection |
| `SyncobjTimelineWaitArgs` | handle + timepoint=seq + timeout_ms |
| `pack_syncobj_timeline_wait_args` | `Some` only when handle valid |
| `format_syncobj_wait_path_decision_note` | soft invent_pass=0 |
| `SYNCOBJ_WAIT_PATH_DECISION_NOTE` | bag name |

**Not done:** live ioctl, L3, ALLOW, unlock mint, invent Pass.

---

## Code changed?

**Yes** (offline pure + docs only).

| Path | Change |
|------|--------|
| `src/backends/amd_xdna2/silicon_contract.rs` | decision helpers + unit test `should_wait_via_syncobj_xrt_parity_decision` |
| `src/backends/amd_xdna2.rs` | re-exports |
| `src/backends/amd_xdna2/direct.rs` | `HardwareContext.syncobj_handle` doc points at pure selection / not live-wired |
| `.agents/reports/wave3-syncobj-wait-offline.md` | this report |

---

## TDD

| Test | Contract |
|------|----------|
| `should_wait_via_syncobj_xrt_parity_decision` | handle 0 → WaitCmd / no pack; non-zero → SyncobjTimeline + timepoint=seq; soft note invent_pass=0 not_live_wired |

```bash
cargo fmt -p dragon-npu
cargo test --features amd-xdna2 --lib should_wait_via_syncobj
cargo clippy -p dragon-npu --features amd-xdna2 --lib -- -D warnings
```

Implementer note: pure APIs + unit test landed on disk; process mop / parent
should re-run the filters above and record exit codes if not captured here.

---

## Unlock justified?

**No.** Decision research + pure helpers ≠ packing root proof ≠ Pass ≠ unlock.

Do **not** mint. Do **not** re-live skip-CU / spent flexml under this report.

---

## invent_pass

**0**

---

## Explicit non-claims

- Pass = No
- not C-ready
- not `ran_on_npu`
- Syncobj research alone ≠ Pass
- Pure decision alone ≠ live wait fix
- Soft note always `invent_pass=0 not_c_ready=1 not_ran_on_npu=1 syncobj_wait_parity_alone_not_pass=1 not_live_wired=1`
- No live unlock justified

---

## Recommended next (outside this slice)

| Priority | Work | Kind |
|----------|------|------|
| P0 residual | residual-matched / load-accepted material | material |
| P1 optional | under **new** unlock: research A/B live wait via syncobj when `syncobj≠0` (env-gated), still require ERT Completed + OFM allclose | live one-shot |
| P2 offline | if live A/B planned: pack `drm_syncobj_timeline_wait` repr + timeout_ms→abs_ns pure tests before wire | TDD |
| Frozen | empty reconfirm, reopen CONSUMED, invent Pass | — |

---

## File map

| Path | Role |
|------|------|
| `src/backends/amd_xdna2/silicon_contract.rs` | pure decision + soft note + test |
| `src/backends/amd_xdna2/direct.rs` | WAIT_CMD only; retain syncobj field |
| `kernel/dkms/staging/.../src/shim/hwq.cpp` | XRT prefer syncobj |
| `kernel/dkms/staging/.../src/shim/platform.cpp` | TIMELINE_WAIT packing |
| `.agents/reports/wave0-wait-einval-uapi.md` | WAIT EINVAL dig |
| `.agents/reports/wave1-o2-wait-syncobj.md` | retain + axes |
| `.agents/reports/wave3-syncobj-wait-offline.md` | this report |
