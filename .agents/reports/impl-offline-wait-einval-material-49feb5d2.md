# Offline implement report: WAIT_CMD EINVAL dig + material inventory (`49feb5d2`)

**Date:** 2026-08-11
**Honesty:** `invent_pass=0` · **Pass = No** · **no** `ran_on_npu` · unlock **not minted**
**Scope:** offline only — no live NPU, no re-live under CONSUMED `1f761bf4`

---

## Plain English

1. **WAIT_CMD EINVAL dig (lab classic, no CONFIG_CU):** added pure parsers and a
   soft note that pull measured axes out of last_apply status/detail: product
   status `wait_cmd_einval`, Invalid argument / os error 22, WAIT_CMD
   `hwctx` / `seq` / `timeout_ms` / `request`, CONFIG_CU / PDI presence, aiebu +
   rela, lab classic 1688 sizes, ERT state when present. Fixture TDD matches the
   skip-CONFIG_CU shot under `1f761bf4`. This is **hypothesis + measured tokens
   only** — not a proven firmware root cause.

2. **Material inventory:** clear labels for what counts as load-accepted /
   residual-matched (live ERT Completed + non-zero OFM allclose via
   `decide_ran_on_npu`) versus **public-npu2-small** research-only files under
   `testdata/xdna2_cmd/external_known_good/`. Public AOT alone is never residual
   identity, never unlock alone, never Pass.

3. **Pins:** `RESIDUAL.md` top + Ranked next + `AGENTS.md` Latest Turn updated.
   First `C_UNLOCK=` remains `1f761bf4` **CONSUMED**. No unlock mint.

---

## APIs landed (`src/backends/amd_xdna2/silicon_contract.rs`)

| API | Role |
|-----|------|
| `WaitCmdEinvalAxes` | Measured axes bag |
| `parse_wait_cmd_einval_axes` | Pure parse from status + detail |
| `format_wait_cmd_einval_axes_note` | Soft note always invent_pass=0 |
| `WAIT_CMD_EINVAL_AXES_NOTE` | Soft bag name token |
| `MaterialResidualMatchKind` | load_accepted_criteria / public_npu2_small_research_only / lab_classic_mladf_oracle / flexml_spent_research / unknown |
| `MaterialResidualMatchInventory` | Inventory result (residual_matched_claim always false offline) |
| `classify_material_residual_match` | Length + tag hint → inventory |
| `format_material_residual_match_note` | Soft note always invent_pass=0 |
| `format_material_residual_match_checklist` | SSoT load-accepted vs research-only checklist |
| `MATERIAL_RESIDUAL_MATCH_NOTE` | Soft bag name token |

Re-exports: `src/backends/amd_xdna2.rs`.

### Soft honesty tokens (always)

- WAIT axes: `invent_pass=0 not_c_ready=1 not_ran_on_npu=1 axes_alone_not_pass=1 hypothesis_not_proven_root=1 wait_cmd_einval_neq_wait_timeout=1`
- Material inventory/checklist: `invent_pass=0 not_c_ready=1 not_ran_on_npu=1 residual_matched_claim=0 public_aot_neq_residual_identity=1 product_known_good=0`

### Skip-CU shape locked by TDD (from `1f761bf4` last_apply tokens)

| Axis | Value |
|------|--------|
| status | `wait_cmd_einval` |
| invalid_argument | yes (os error 22) |
| hwctx | 1 |
| seq | 0 |
| timeout_ms | 5000 |
| request | `0xc0106449` |
| config_cu_present | 0 |
| pdi_bytes | absent |
| lab classic 1688 | yes |
| aiebu + rela×12 | yes |
| ert_state | absent |
| detail token | `skip_cu_lab_classic_wait_einval` |

With `config_cu=1` + `pdi_bytes` → `wait_einval_with_config_cu` (not skip shape).

---

## Material path inventory (what counts)

| Kind | What it is | Residual match? | Unlock alone? | Pass? |
|------|------------|-----------------|---------------|-------|
| **Load-accepted / residual-matched** | Program material that can reach product Pass only after **live** ERT Completed + non-zero OFM allclose via `decide_ran_on_npu` + real_ioctl | only after live decide | no | only after live decide |
| **public-npu2-small** | Public mlir-aie / IRON npu2 AOT under `testdata/xdna2_cmd/external_known_good/` (vector ~300 B, single_core ~660 B, whole_array, …) | **no** | **no** | **no** |
| **mladf-classic-1688** | Lab staged classic ctrltext oracle | **no** (oracle path) | **no** | **no** |
| **flexml spent research** | Spent STX PDI fingerprints | **no** (re-live alone banned) | **no** | **no** |

---

## TDD

| Test | Result |
|------|--------|
| `wait_cmd_einval_axes_skip_cu_lab_classic_1f761bf4_shape` | **GREEN** (RED first: request parse failed on trailing `):` — fixed strip) |
| `material_residual_match_inventory_public_vs_load_accepted` | **GREEN** |

---

## Validate

| Command | Exit |
|---------|------|
| `cargo fmt -p dragon-npu` | 0 |
| `cargo clippy --features amd-xdna2 --lib -- -D warnings` | 0 |
| `cargo test --features amd-xdna2 --lib wait_cmd_einval_axes` | 0 (1 ok) |
| `cargo test --features amd-xdna2 --lib material_residual_match` | 0 (1 ok) |
| `bash tools/pass_unblock_once.sh --status` | 0 (Unlock none / CONSUMED; invent_pass=0; Pass=No) |

---

## Residual / AGENTS pin

- First `C_UNLOCK=` = `1f761bf4` **CONSUMED** (unchanged; no reopen; no mint)
- Highest-value next: load-accepted / residual-matched material
- Offline WAIT dig tools **landed** (not re-open as next live experiment)
- Freeze: spent flexml re-live, skip-CU reconfirm under CONSUMED, invent Pass, sticky ALLOW, empty reconfirm

---

## Explicit non-claims

- **Pass = No**
- **invent_pass = 0**
- **Unlock minted = No**
- Axes alone ≠ Pass ≠ proven root cause
- Public AOT ≠ residual identity ≠ unlock alone
- No live device work this slice
