# S3: WAIT fence diagnostic (offline)

**Date:** 2026-08-12
**Slice:** useful-npu-accel **S3** only
**Honesty:** invent_pass=0 · not_c_ready=1 · not_ran_on_npu=1 ·
axes_alone_not_pass=1 · live_justified=0 · Pass=**No**
**Unlock:** none minted · none reopened · diagnostic ≠ unlock
**Live:** not run · L3 not opened · ALLOW not set

## Why this is not empty reconfirm

Prior wait axes and post-fail ERT already classify EINVAL vs ETIME and
optional ERT after WAIT fail. They did **not** emit the plan-S3 ranked
story as tokens (`OutFenceLookupFail` vs `UnpublishedJob` vs
`SyncobjParityUnproven`) and wait notes did **not** lock
`live_justified=0`. Docs-only would have left those tokens untestable.

## What landed

### Pure API (`silicon_contract.rs` only)

| Symbol | Role |
|--------|------|
| `WAIT_FENCE_DIAGNOSTIC_NOTE` | soft bag name `wait_fence_diagnostic` |
| `WaitFenceHypothesis` | rank 1/2/3 + `NotWaitFenceClass` (rank 0) |
| `WaitFenceDiagnostic` | open flags + reused driver axis + post-fail ERT |
| `parse_wait_fence_diagnostic` | status + detail + optional journal |
| `format_wait_fence_diagnostic_note` / `_soft` | greppable line |

Parser **reuses** `parse_wait_cmd_einval_axes`,
`map_wait_cmd_einval_driver_axes`, `parse_post_fail_ert_research`,
`parse_last_apply_rich_tokens`. It does not re-implement them.

On `wait_cmd_einval`:

- `primary` = `OutFenceLookupFail` (rank 1)
- `out_fence_lookup_open` when driver axis `is_product_einval()`
- `unpublished_job_open` unless post-fail ERT is Error or Completed
- `syncobj_parity_unproven` when `syncobj≠0` and `wait_path` is
  `wait_cmd` or absent
- `timeout_ruled_out` / `seq_zero_ruled_out` / `ioctl_size_ruled_out`
  when this is the EINVAL class (ioctl also requires request absent or
  `0xc0106449`)

Honesty in the formatter is **hard-coded** (not from mutable fields):

```
invent_pass=0 not_c_ready=1 not_ran_on_npu=1
axes_alone_not_pass=1 live_justified=0
hypothesis_not_proven_root=1
product_pass_requires_live_decide=1
diagnostic_alone_not_unlock=1
```

### Ranked write-up

`.agents/reports/wait-fence-diagnostic.md` cites staged
`aie2_cmd_wait` / `xa_load` / `drm_syncobj_add_point`, shim `hwq.cpp`,
and product `direct.rs` (read-only).

### Not changed (write-scope)

- `direct.rs` not edited (no live-wire wait path)
- `amd_xdna2.rs` no re-export (classify / public crate surface not in
  this slice)
- `cli.rs` / `external_runner` / `host_graph` / residual / AGENTS not
  edited

## TDD

| Step | Evidence |
|------|----------|
| **RED** | `cargo test --features amd-xdna2 --lib wait_fence_diagnostic_ranks_out_fence_unpublished_syncobj` → compile fail: `cannot find function parse_wait_fence_diagnostic` / undeclared `WaitFenceHypothesis` (test landed before product) |
| **GREEN** | same filter → **ok** (1 passed). Recheck after parallel S2 settled: **TEST_EXIT=0** |

Unit locks: `as_str` / `rank`; live-shaped skip-CU / same-shot detail
(`seq=0`, `syncobj=1`, `wait_path=wait_cmd`, `request=0xc0106449`);
journal fence miss → `FenceOrSyncobjOrSeqMiss`; ERT Error →
`unpublished_job_open=0`; `wait_timeout` → `NotWaitFenceClass`; empty
honesty; exact tokens `invent_pass=0` / `live_justified=0` and **not**
`invent_pass=1` / `ran_on_npu=1` / `live_justified=1`.

## Post-impl verify

| Command | Exit |
|---------|------|
| `cargo fmt -p dragon-npu` | **0** |
| `cargo test --features amd-xdna2 --lib wait_fence_diagnostic_ranks_out_fence_unpublished_syncobj` | **0** (1 ok) |
| `cargo clippy -p dragon-npu --lib -- -D warnings` | **101** |

Clippy 101 is **not** this slice. Failures are
`src/backends/external_runner.rs` (`struct_excessive_bools`) and
`src/cli.rs` (`too_many_lines` on `parse_args`). Both are S1/S2 write
scope. This slice did not touch them. `WaitFenceDiagnostic` already
allows `struct_excessive_bools` like the other research bags.

## Honesty

| Claim | Status |
|-------|--------|
| Pass | **No** |
| invent_pass | **0** |
| not_c_ready | **1** |
| not_ran_on_npu | **1** |
| live_justified | **0** |
| diagnostic_alone_not_unlock | **1** |
| product wait path | still WAIT_CMD |
| mint | **none** |

## Who acts next

S4 join (other dual-track slices) can cite this report. Product live
still needs a **new** unlock **and** a **new** named reason (not this
diagnostic). Human material wall unchanged.
