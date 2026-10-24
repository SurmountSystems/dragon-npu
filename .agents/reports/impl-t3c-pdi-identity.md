# Implement report: T3c research PDI identity + skip CONFIG_CU A/B

**Date:** 2026-08-10
**Slice:** T3c only (offline product delta; no live EXEC)
**Honesty:** invent_pass=0 · **no** product Pass · **no** `ran_on_npu` · **no** invented capture bytes
**Unlock:** **`C_UNLOCK=97f38307` active** (host-minted from kernel RNG after T3c
named offline reason). Prior **`C_UNLOCK=11bc9016` CONSUMED** (do not reopen).

---

## Plain English

The stock live shot under `11bc9016` used a **research** PDI from
`dump_xclbin_pdi` (CVML / AIE2P overlay, 283840 B). That attach was measured
(`config_cu=1`), then WAIT EINVAL + orphan IO_PAGE_FAULT.

**T3c** makes that honesty product-visible offline and gives the next live shot
a real A/B lever:

1. Score default dump / last_apply PDI size as **research, not product matmul
   known-good** (`research_pdi_not_product_known_good=1`).
2. Pure fingerprint of opaque PDI bytes (`len` + sha256 first 16 hex) for soft
   last_apply compare when bytes exist.
3. Soft preflight note: attach measured ≠ Pass; wrong overlay may still
   IO_PAGE_FAULT / APP_LOAD.
4. Tooling: **skip CONFIG_CU** by unsetting `XDNA2_LAB_PDI_BLOB` (or
   `XDNA2_LAB_SKIP_CONFIG_CU=1`) so the next experiment can compare research blob
   vs no CONFIG_CU.

This does **not** create Pass. It does **not** invent a second PDI blob.

---

## Named offline reason for T4 unlock

| Fact | Source |
|------|--------|
| WAIT EINVAL (not timeout mislabel) | T3 `wait_cmd_einval` |
| Fault `0x2fff80007f40` **orphan** vs BO map | T3b audit |
| Research PDI ≠ product known-good | T3c pure identity + dump notes |
| Next live needs recorded experiment | blob **or** skip CONFIG_CU (T3c kit) |

**Active unlock for T4:** `C_UNLOCK=97f38307` (first Unlock row in `RESIDUAL.md`;
no CONSUMED on that line). Do not reopen `11bc9016` / `3a70fabf` / `b17b0119`.

---

## API names (pure)

| API | Role |
|-----|------|
| `RESEARCH_PDI_IDENTITY_CLASS` | Soft class token `research_pdi_not_product_known_good` |
| `RESEARCH_PDI_DEFAULT_DUMP_BYTES` | 283840 (11bc9016 live size shape) |
| `PDI_FINGERPRINT_SHA_PREFIX_HEX` | 16 |
| `ResearchPdiSourceKind` | Absent / CvmlAie2pResearchOverlay / UnknownOpaque |
| `PdiBlobFingerprint` | `len_bytes` + `sha256_prefix16` + full hex |
| `ResearchPdiIdentity` | Scorecard + `format_soft_note()` |
| `fingerprint_pdi_blob_bytes` | Bytes → fingerprint |
| `research_pdi_identity_from_bytes` | Optional bytes + path hint |
| `research_pdi_identity_from_last_apply_signals` | last_apply size / CONFIG_CU + optional blob |
| `research_pdi_identity_default_dump_locked` | Default dump locked not product KG |
| `research_pdi_source_kind_from_hint` | Path/source taxonomy |
| `format_research_pdi_preflight_note` | Soft preflight string |
| `parse_pdi_fingerprint_from_detail` | Parse `pdi_fp len=… sha16=…` from detail |

**Module:** `src/backends/amd_xdna2/silicon_contract.rs` (re-exported from `amd_xdna2`).

**Soft wire:** `examples/classify_last_apply.rs` prints
`research_pdi_identity=…` from last_apply signals (soft only).

---

## Tooling delta (next live options)

| Command | Effect |
|---------|--------|
| `bash tools/hitl_config_cu_once.sh prep-skip-pdi` | Unset PDI; sandwich without CONFIG_CU |
| `bash tools/hitl_config_cu_once.sh run-stock-skip-pdi` | Stock live A/B without CONFIG_CU |
| `bash tools/operator_app_load_kit.sh config-cu-skip` | Same prep skip |
| `bash tools/operator_app_load_kit.sh config-cu-run-skip` | Same live skip |
| `export XDNA2_LAB_SKIP_CONFIG_CU=1` + `unset XDNA2_LAB_PDI_BLOB` | Manual skip |
| `bash tools/dump_xclbin_pdi.sh --list-default` | Candidate list + honesty + skip env |
| dump with `--print-env` | Emits `pdi_fp len=… sha16=…` + research notes |

Lab already skips CONFIG_CU when `XDNA2_LAB_PDI_BLOB` is unset (no code invent of a second blob).

---

## TDD

### Tests (filter)

| Test | Role |
|------|------|
| `fingerprint_pdi_blob_bytes_len_and_sha_prefix` | len + sha16 + stable digest |
| `research_pdi_identity_not_product_known_good` | default dump / last_apply size / path hint honesty |
| `parse_pdi_fingerprint_from_detail_compact_and_split` | soft detail parse |

### Gate commands (parent if shell unavailable here)

```bash
cd /home/hunter/Projects/ai/dragon-npu
cargo fmt -p dragon-npu
cargo clippy -p dragon-npu --features amd-xdna2 --all-targets -- -D warnings
cargo test --features amd-xdna2 --lib fingerprint_pdi_blob
cargo test --features amd-xdna2 --lib research_pdi_identity
cargo test --features amd-xdna2 --lib parse_pdi_fingerprint
bash -n tools/hitl_config_cu_once.sh tools/operator_app_load_kit.sh tools/dump_xclbin_pdi.sh
```

---

## Files touched

- `src/backends/amd_xdna2/silicon_contract.rs` — pure helpers + tests
- `src/backends/amd_xdna2.rs` — re-exports
- `examples/classify_last_apply.rs` — soft `research_pdi_identity=`
- `tools/hitl_config_cu_once.sh` — prep-skip-pdi / run-stock-skip-pdi
- `tools/operator_app_load_kit.sh` — config-cu-skip / config-cu-run-skip + overview
- `tools/dump_xclbin_pdi.sh` — list honesty + pdi_fp + skip env notes
- `RESIDUAL.md` — T3c landed; unlock mint pending host hex
- `.agents/plans/plan-complete-product-silicon.md` — T3c done; T4 next
- `.agents/reports/impl-t3c-pdi-identity.md` — this report

---

## Not done / still true

- **No** product Pass. **No** `ran_on_npu`. invent_pass=0.
- **No** live EXEC in this slice (T4 next).
- Capture dir still empty of real known-good bytes.
- Do **not** reopen `11bc9016` / `3a70fabf` / `b17b0119`.
- T4 live under `97f38307` only; mark CONSUMED after the shot.
