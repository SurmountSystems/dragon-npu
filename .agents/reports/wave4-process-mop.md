# W4 process mop — fmt / clippy / tests

**Role:** `[process-mop]`  
**Repo:** `/home/hunter/Projects/ai/dragon-npu`  
**Package:** `dragon-npu`  
**Scope:** silicon_contract, direct, W4 tools scripts  
**Honesty:** invent_pass=0; no Pass; no live NPU; no unlock mint.

## Commands and exit codes

| Step | Command | Exit |
|------|---------|------|
| fmt | `cargo fmt -p dragon-npu` | **0** |
| clippy | `cargo clippy --features amd-xdna2 --lib -- -D warnings` | **0** |
| test syncobj | `cargo test --features amd-xdna2 --lib syncobj` | **0** (6 passed) |
| test material_exec_readiness | `cargo test --features amd-xdna2 --lib material_exec_readiness` | **0** (2 passed) |
| test last_apply_rich | `cargo test --features amd-xdna2 --lib last_apply_rich` | **0** (1 passed) |
| test choose_lab_wait | `cargo test --features amd-xdna2 --lib choose_lab_wait` | **0** (1 passed) |
| bash -n tools | `bash -n tools/host_npu_stack_inventory.sh tools/journal_npu_scrape.sh tools/capture_from_host_candidate.sh tools/pass_unblock_once.sh tools/collect_post_c2_evidence.sh` | **0** |

## Test detail

### syncobj (6 ok)
- `backends::amd_xdna2::direct::tests::drm_syncobj_timeline_wait_struct_and_ioctl_layout`
- `backends::amd_xdna2::direct::tests::create_hwctx_records_syncobj_handle_in_context`
- `backends::amd_xdna2::direct::tests::real_wait_command_with_syncobj_fake_fd_not_exec_gated`
- `backends::amd_xdna2::direct::tests::stub_wait_command_with_syncobj_env_gate_no_hw`
- `backends::amd_xdna2::silicon_contract::tests::should_wait_via_syncobj_xrt_parity_decision`
- `backends::amd_xdna2::silicon_contract::tests::syncobj_timeline_wait_pack_timeout_and_errno_classify`

### material_exec_readiness (2 ok)
- `backends::amd_xdna2::silicon_contract::tests::material_exec_readiness_scorecard_lab_cascade_empty`
- `backends::amd_xdna2::silicon_contract::tests::material_exec_readiness_w4b_bag_and_candidate_print`

### last_apply_rich (1 ok)
- `backends::amd_xdna2::silicon_contract::tests::last_apply_rich_tokens_fixture_and_l5b_shape`

### choose_lab_wait (1 ok)
- `backends::amd_xdna2::direct::tests::choose_lab_wait_path_env_gate_and_handle`

## Fixes applied

None. All gates green on first pass; no product code edits required.

## Verdict

**GREEN.** W4 process mop complete. Offline only; no Pass / no `ran_on_npu` / no unlock.
