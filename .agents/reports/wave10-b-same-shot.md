# W10-B T1 — same-shot rela (Bridge B product offline wire)

**Date:** 2026-08-11
**Scope:** Offline product code only. No unlock mint. No live apply. No sticky ALLOW.
**Honesty:** invent_pass=0 · Pass=**No** · unlock=**none** (first `C_UNLOCK=` remains CONSUMED; do not reopen)

## One-line

**same-shot offline wire ready? yes** (pure plan + env-gated lab hook; live still needs **new** unlock + named reason; do not mint).

## Goal closed

W9 gap: disk last_apply / vendor UVAs **age out** across processes. Product lab must patch aiebu rela×12 with **in-process** BO `xdna_addr` after create/map, not aged `/tmp` last_apply paddrs.

## What landed

### Pure API (`silicon_contract.rs`)

| API | Role |
|-----|------|
| `ENV_LAB_SAME_SHOT_RELA` | `"XDNA2_LAB_SAME_SHOT_RELA"` — default **off** |
| `SAME_SHOT_RELA_PLAN_NOTE` / `SAME_SHOT_RELA_APPLY_NOTE` | Soft bag names |
| `SameShotPaddrSource` | `InProcessBoMap` / `DiskLastApply` / `DiskVendorJson` / `Absent` |
| `SameShotRelaPlan` | Plan bag: same_shot, paddrs_valid, apply_ready, refuse_stale_disk, source, IO options |
| `SameShotRelaApplyResult` | Pure apply research: patch_ok, classic_walk_ok, optional patched bytes |
| `lab_same_shot_rela_env_token_is_enabled` | Pure token table (`1`/`true`/`yes`/`on`) |
| `lab_same_shot_rela_env_requested` | Process env read (default false) |
| `plan_same_shot_rela_from_io_paddrs` | In-process bases → `same_shot=1`, refuse 0/`u64::MAX` |
| `plan_same_shot_rela_from_disk_extract` | Disk extract → `same_shot=0`, **always** `apply_ready=0` |
| `format_same_shot_rela_plan_note` | Soft: `same_shot=0\|1 invent_pass=0 not_ran_on_npu=1 refuse_stale_disk=1` |
| `format_same_shot_rela_apply_note` | Soft apply honesty |
| `apply_same_shot_rela_research` | Pure-direct patch + classic walk when plan ready |

Re-exports: `amd_xdna2` public surface.

### Lab hook (`timed_matmul` / direct path)

- **Default (env unset):** unchanged. Still patches aiebu with **current-process** `ios.ifm_addr` / wts / ofm / ctrl after create/pin when `XDNA2_LAB_OFM=1`. No disk UVAs loaded.
- **`XDNA2_LAB_SAME_SHOT_RELA=1`:** before patch, runs `plan_same_shot_rela_from_io_paddrs` on those same BO addresses. Invalid paddrs → status `same_shot_rela_plan_failed` (fail-closed; never invent). On success, patch uses plan IO tuple and appends plan soft note to `blob_note` / detail.
- **Never** loads `/tmp` last_apply or vendor JSON paddrs into the product rela path.

## Env names

| Env | Default | Effect |
|-----|---------|--------|
| `XDNA2_LAB_SAME_SHOT_RELA` | off | When `1`/`true`/`yes`/`on`: same-shot plan gate on aiebu rela |
| (existing) `XDNA2_LAB_OFM` | off | Still required for IO BO create + rela before EXEC |
| (existing) `XDNA2_LAB_RELA_MODE` | overwrite | Overwrite vs add rela trial |

Env alone ≠ unlock ≠ Pass ≠ `ran_on_npu`.

## How default path is unchanged

When `XDNA2_LAB_SAME_SHOT_RELA` is **unset**, the timed path still calls `patch_aiebu_elf_ctrltext_with_bos_ex` directly with `ios.*_addr` (same as pre-W10-B). No new fail status, no plan note required. The formal plan API is opt-in research / strict gate only.

## Tests / gates

```text
cargo fmt -p dragon-npu
cargo clippy -p dragon-npu --features amd-xdna2 --lib -- -D warnings   # clean
cargo test --features amd-xdna2 --lib w10_b_   # 4 ok
cargo test --features amd-xdna2 --lib w9_b_    # 4 ok (regression)
cargo test --features amd-xdna2 --lib w8_      # 5 ok (regression)
```

| Test | Contract |
|------|----------|
| `w10_b_same_shot_env_token_table` | default off; 1/true/yes/on enable |
| `w10_b_same_shot_plan_from_io_paddrs` | valid → apply_ready+same_shot; 0/MAX refuse; honesty tokens |
| `w10_b_same_shot_refuse_disk_extract` | last_apply/vendor → apply_ready=0 same_shot=0 |
| `w10_b_same_shot_apply_research_in_process` | staged ELF + synthetic paddrs → patch_ok; GT oracle when present |

## Offline ready for live join?

| Question | Answer |
|----------|--------|
| Pure same-shot plan + apply research | **yes** |
| Lab hook uses in-process BO paddrs (never default disk) | **yes** |
| invent_pass | **0** |
| Pass | **No** |
| Unlock minted this slice | **none** (do not mint) |
| Live L1b justified from this pin alone | **no** |
| Live still needs | **new** host-minted unlock + named reason + real map at apply time |

## Honesty pin

- invent_pass=**0**
- Pass=**No**
- unlock=**none**
- Offline same-shot wire ≠ residual match ≠ load-accepted ≠ `ran_on_npu`
- Disk research UVAs remain useful for offline M3 rela research only; product apply refuses them via `plan_same_shot_rela_from_disk_extract`
- True post-patch classic / residual-matched material still primary Pass wall

## Files touched

- `src/backends/amd_xdna2/silicon_contract.rs` — same-shot plan/apply APIs + w10_b tests
- `src/backends/amd_xdna2/direct.rs` — env-gated lab hook in timed aiebu rela path
- `src/backends/amd_xdna2.rs` — re-exports
- `.agents/reports/wave10-b-same-shot.md` — this report

## Next (not this slice)

- Live L1b only after **new** unlock + named reason under same-shot map at apply time
- Residual-matched / load-accepted classic material still highest Pass gap
- No empty reconfirm; no reopen CONSUMED unlocks
