# W9-B — L1b prep + T3 readiness (offline Bridge B)

**Date:** 2026-08-11
**Scope:** Offline only. No unlock mint. No live apply. No ALLOW.
**Honesty:** invent_pass=0 · Pass=No · unlock=none (first `C_UNLOCK=` remains CONSUMED; not reopened)

## One-line

**Live paddrs found on disk: yes** (vendor fixtures + multiple `/tmp` last_apply `bo_diag` rows). **l1b_live_justified=0** (unlock still required).

## Did we find live paddrs on disk?

**Yes.**

| Path | Source | live triad | Values (hex) |
|------|--------|------------|--------------|
| `testdata/xdna2_cmd/gt_capture/vendor_paddrs_from_last_apply.json` | vendor_json | **yes** | ifm=`0x7f57b364c000` wts=`0x7f57b3482000` ofm=`0x7f57b364b000` ctrl=`0x7f57b3643000` |
| `testdata/xdna2_cmd/gt_capture/vendor_paddrs.json` | vendor_json | **yes** (ctrl null OK) | ifm=`0x7fd181309000` wts=`0x7fd18083e000` ofm=`0x7fd181308000` |
| `/tmp/dragon_npu_last_apply_config_cu_821691.json` | last_apply bo_diag | **yes** | ifm=`0x7fcbe9a30000` wts=`0x7fcbe977f000` ofm=`0x7fcbe9a2f000` ctrl=`0x7fcbe9a27000` instr=`0x4028000` |
| `/tmp/dragon_npu_last_apply_config_cu_4080903.json` | last_apply bo_diag | **yes** | ifm=`0x7f8e02ba0000` wts=`0x7f8e02b75000` ofm=`0x7f8e02b74000` ctrl=`0x7f8e02b6c000` instr=`0x4020000` |
| `/tmp/dragon_npu_last_apply_config_cu_675133.json` | last_apply bo_diag | **yes** | ifm=`0x7f32b68fc000` wts=`0x7f32b68d1000` ofm=`0x7f32b68d0000` ctrl=`0x7f32b68c8000` instr=`0x4028000` |
| `/tmp/dragon_npu_last_apply_config_cu_{1374078,2328515,2618482,3377745}.json` | last_apply bo_diag | **yes** | host UVAs per shot (see mine log) |
| `/tmp/dragon_npu_last_apply_config_cu_343412.json` | last_apply | **no** | debug_bo_failed; no bo_diag IO |
| `testdata/xdna2_cmd/last_apply.json` | last_apply | **no** | historical `hwctx_ok` fixture; no `io_paddrs` / IO bo_diag |

Notes:

- Recent live last_apply files often lack the `io_paddrs ifm=…` token and only carry labeled `bo_diag=[… ifm: xdna_addr=…; wts: …; ofm: …; ctrl: …]`.
- O3 extract was extended to mine those bo_diag labels when `io_paddrs` is absent (still pure; never invents zeros / `u64::MAX`).
- Host UVAs from Shmem BOs pass `is_valid_device_paddr` (non-zero / non-MAX). They are **research paddrs** for offline rela wire, not product residual match.

## l1b_live_justified for mint?

**0** (must stay 0 for mint offline).

| Gate | Offline value | Meaning |
|------|---------------|---------|
| `live_paddrs_present` | **1** (when extract succeeds) | Evidence on disk |
| `paddr_evidence_ready` | **1** when paddrs present | Mint *prep* evidence only |
| `unlock_required` | **1** always | Live L1b needs a **new** host-minted unlock |
| `unlock_active` | **0** (offline prep) | No unlock mint this wave |
| `l1b_offline_ready` | **1** when paddrs present | Pure-direct rela wire can use paddrs offline |
| `l1b_live_justified` | **0** unless paddrs **and** unlock_active | Live shot **not** justified without unlock |
| `residual_matched` | **0** locked | Offline checklist never claims residual identity |
| `invent_pass` | **0** locked | Never Pass from checklist |

Soft score wall tokens (cheap T3): `true_post_patch_missing=1` · `oracle_sha16=3c5ffca750922d3b` · `INSTR_BO_LIMIT=1`.

## APIs added / changed

**New (pure TDD in `silicon_contract.rs`):**

- `L1B_READINESS_CHECKLIST_NOTE`
- `L1bReadiness` struct
- `evaluate_l1b_readiness(extract, unlock_active)`
- `format_l1b_soft_score_tokens(extract)`
- `format_l1b_readiness_checklist(&L1bReadiness)`
- `format_l1b_readiness_checklist_from_extract(extract)` (offline unlock=false)

**Changed:**

- `extract_live_paddrs_from_detail` — also mines `bo_diag` labels `ifm` / `wts` / `ofm` / `ctrl` / `instr` when `io_paddrs` token is absent.

**Re-exports:** `amd_xdna2` module public surface updated.

## Tests

```text
cargo fmt -p dragon-npu
cargo clippy -p dragon-npu --features amd-xdna2 --lib -- -D warnings   # clean
cargo test --features amd-xdna2 --lib w9_b_   # 4 ok
cargo test --features amd-xdna2 --lib w8_o3_  # 3 ok (O3 regression)
```

| Test | Contract |
|------|----------|
| `w9_b_extract_live_paddrs_from_bo_diag_labels_alone` | bo_diag-only detail → live triad + instr |
| `w9_b_l1b_readiness_absent_paddrs_not_justified` | empty extract → all readiness 0; invent_pass=0 |
| `w9_b_l1b_readiness_paddrs_present_unlock_still_required` | paddrs + unlock=false → offline_ready=1, live_justified=0; unlock=true → justified=1 still invent_pass=0 |
| `w9_b_l1b_readiness_from_vendor_fixture` | real vendor JSON → offline ready, live unjustified |

## Honesty pin

- invent_pass=**0**
- Pass=**No**
- unlock=**none** (do not mint from this pin)
- `l1b_live_justified=0` offline even though paddrs exist
- Offline paddr wire ≠ residual match ≠ load-accepted ≠ `ran_on_npu`
- True post-patch classic still missing; public AOT frozen; no re-capture this slice

## Next (not this slice)

- L1b live only after **new** unlock + named reason + live device paddrs at apply time (disk research UVAs age out across processes).
- Optional: soft-wire checklist into classify soft line (not required for W9-B).
- Material residual / true post-patch still highest Pass gap.
