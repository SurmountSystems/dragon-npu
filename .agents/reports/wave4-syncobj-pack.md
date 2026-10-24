# Wave W4a T5 — Syncobj pack / timeout pure unit tests (offline)

**Date:** 2026-08-11
**Mode:** offline only. No live NPU. No ALLOW. No unlock mint.
**Honesty:** `invent_pass=0` · `not_live_proven=1` · **L8 not minted** · Pass = No

**Upstream:**
- `.agents/reports/wave3-syncobj-wait-offline.md` (W3 O13 decision helpers)
- `.agents/reports/wave1-o2-wait-syncobj.md` (CREATE retain + axes)
- `.agents/reports/wave0-wait-einval-uapi.md` (W0 O1)

---

## Plain English

1. W3 already locked **which** wait path XRT would pick (`should_wait_via_syncobj`
   when CREATE returns a non-zero syncobj handle). Product still only issues
   **WAIT_CMD** on the live path.
2. This slice adds the pure **pack** pieces a later live wire would need:
   absolute timeout nanoseconds from ms, DRM wait flag constants, errno/message
   classify (ETIME vs EINVAL vs other), and fail-closed full ioctl-arg pack
   when handle is 0.
3. **Not done:** wiring `DRM_IOCTL_SYNCOBJ_TIMELINE_WAIT` into `wait_command`
   (that is W4c later). Default product wait path unchanged.
4. **L8** (live research shot under a new unlock) is **not** minted here.
   Design criteria only (see below).

---

## Code landed

| API | Role |
|-----|------|
| `syncobj_timeout_ms_to_abs_nsec(ms, now)` | XRT parity: `0` → `i64::MAX`; else saturating `now + ms*1e6` |
| `DRM_SYNCOBJ_WAIT_FLAGS_*` | Public uAPI bit constants (`WAIT_ALL` / `FOR_SUBMIT` / `AVAILABLE` / `DEADLINE`) |
| `SYNCOBJ_TIMELINE_WAIT_FLAGS_XRT_DEFAULT` | `WAIT_FOR_SUBMIT` only (staged `platform.cpp`) |
| `SYNCOBJ_ERRNO_EINVAL` / `_ETIME` / `_ETIMEDOUT` | 22 / 62 / 110 |
| `SyncobjWaitErrorClass` | `Etime` \| `Einval` \| `Other` |
| `classify_syncobj_wait_errno` | pure errno (pos/neg) |
| `classify_syncobj_wait_error_message` | pure string (reuses WAIT honesty helpers) |
| `SyncobjTimelineWaitIoctlPack` | handle + timepoint + abs nsec + flags + count=1 |
| `pack_syncobj_timeline_wait_ioctl` | `Some` only when handle ≠ 0; fail-closed |
| `ENV_LAB_WAIT_VIA_SYNCOBJ` | `"XDNA2_LAB_WAIT_VIA_SYNCOBJ"` name only (not product-read) |
| `lab_wait_via_syncobj_research_env_enabled` | pure `1`/`true`/`yes` intent parse |
| `format_syncobj_timeline_wait_pack_note` | soft invent_pass=0 not_live_wired l8_not_minted |
| `SYNCOBJ_TIMELINE_WAIT_PACK_NOTE` | bag name |

**Re-exports:** `src/backends/amd_xdna2.rs`
**Unit test:** `syncobj_timeline_wait_pack_timeout_and_errno_classify`
**Still WAIT_CMD only on live path:** `wait_command` / `submit_and_wait_command` unchanged.

### XRT ground (staged tree, offline)

| Source | Behavior |
|--------|----------|
| `platform.cpp` `timeout_ms2abs_ns` | `timeout_ms==0` → forever (`INT64_MAX`); else mono now + ms×1e6 |
| `platform.cpp` `wait_syncobj` | `DRM_IOCTL_SYNCOBJ_TIMELINE_WAIT`, flags `WAIT_FOR_SUBMIT` |
| `hwq.cpp` `wait_command` | non-zero CREATE syncobj → syncobj wait; else WAIT_CMD |
| uAPI | handle `0` = `AMDXDNA_INVALID_FENCE_HANDLE` |

---

## TDD / gates

| Test | Contract |
|------|----------|
| `syncobj_timeline_wait_pack_timeout_and_errno_classify` | flags bits; abs nsec (0 forever, overflow saturate, neg now clamp); errno ±ETIME/ETIMEDOUT/EINVAL/other; message classify; handle 0 → no pack; soft note invent_pass=0 l8_not_minted |
| `should_wait_via_syncobj_xrt_parity_decision` | prior W3 (still green) |
| `create_hwctx_records_syncobj_handle_in_context` | prior W1 (still green) |

```bash
cargo fmt -p dragon-npu
cargo test --features amd-xdna2 --lib syncobj
# → 3 passed
cargo clippy --features amd-xdna2 --lib -- -D warnings
# → clean
```

---

## L8 unlock criteria (design note only — **not** minted)

Later live research (W4c / L8) would need **all** of:

1. Offline green pack + timeout + classify tests (this slice).
2. Explicit research env intent: `XDNA2_LAB_WAIT_VIA_SYNCOBJ=1` (name
   documented; product does not read it yet; enable alone ≠ unlock).
3. A **new** host-minted `C_UNLOCK=` with a **named** offline reason that is
   not "pack helpers landed alone."
4. Live wire of `DRM_IOCTL_SYNCOBJ_TIMELINE_WAIT` only under that unlock
   (still WAIT_CMD default when env off / handle 0).
5. Pass still requires live ERT Completed + non-zero OFM allclose via
   `decide_ran_on_npu`. Syncobj wait alone ≠ Pass.

**This slice does not mint L8.** Tokens: `l8_not_minted=1` ·
`syncobj_pack_alone_not_unlock=1` · `not_live_proven=1`.

---

## Unlock justified?

**No.** Pack/timeout pure TDD ≠ packing root proof for live `wait_cmd_einval`
≠ Pass ≠ unlock.

Do **not** mint. Do **not** re-live CONSUMED unlocks / spent flexml under this
report. Do **not** set sticky ALLOW.

---

## invent_pass

**0**

## not_live_proven

**1**

## Explicit non-claims

- No `ran_on_npu`
- No product Pass
- No live SYNCOBJ_TIMELINE_WAIT
- No default product wait path change
- No L8 unlock mint
- No claim that syncobj wait would clear live `wait_cmd_einval`

---

## Highest remaining (unchanged)

Working-stack residual-matched / load-accepted program material. Cascade /
public AOT research and syncobj pack helpers are not the Pass lever alone.
Live queue: L1 blocked; L4 not open from unspent alone; L5/L5b closed; L7 no;
**L8** needs live wire first under a **new** unlock after more offline if
justified.
