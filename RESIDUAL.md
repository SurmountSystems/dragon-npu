# Residual — silicon path (honest status)

> **Agent rules (read first):** Chat and status = **plain English first** (what
> happened / what is blocked / which path or file / next action). Use real paths
> (`testdata/xdna2_cmd/external_known_good/`), not nicknames (“bag,” “Path A,”
> “mill”). Do **not** mint thoughtless stamp/unlock hex. **Empty
> reconfirm and empty-folder `/implement` loops are frozen** — if capture bytes
> or evidence are still missing, say so **once** and **stop**; do not auto-queue
> another reconfirm. Name work by research question, not a new track letter.
> Full rules: `AGENTS.md` standing §6–9.

**Date:** 2026-07-20

### Plain status (read this first)

| | |
|--|--|
| **What is broken** | Firmware still rejects the instruction load (`0x3000003` APP_LOAD_PDI_FAIL) after userspace ELF+rela looks OK. |
| **What is missing** | A **real** capture from a working public stack under `testdata/xdna2_cmd/external_known_good/` (plus honest PROVENANCE). Right now that dir has PROVENANCE only — **no** `.bin` payload. |
| **What is not missing** | Offline research helpers already exist (extract dry-run, capture-dir checks, soft compare). Running them again without new bytes does **not** advance silicon. |
| **Unlock** | `C_UNLOCK=b17b0119` was used once and is **CONSUMED**. Do not reopen execute-allow under it. |
| **Pass?** | **No.** Not product Pass. Not `ran_on_npu`. |
| **Who acts next** | **Human:** place real capture bytes + PROVENANCE in the external known-good dir (see extract checklist). **Agent:** idle on this residual until that lands **or** the user asks for a named offline code change. Do **not** re-run empty `/implement`. |
| **Local refuse** | `gt_capture/instr_post_patch.bin` and VE2 `validate.xclbin` are **not** accepted as the known-good AIE2 capture. |

**LIVE bookkeeping stamp:** `588c1c72` — named offline code (extract + capture-dir
status helpers). **Not** a live unlock. **Not** Pass. Capture **bytes still absent**.
Last empty reconfirm stamp (historical, do not repeat): `fcec5772`.

**Prior research stamps (index only):** `2b01425e` (capture-dir + feed + dry-run
compose); `d2e287ab` (lab export fingerprint feed); `6033158a` (ctrltext compare
dry-run); `9cdfe8d0` (progress checklist); `8e898123` … `045739e1` — see PLAN.md.
Full demoted gate-only prior bodies live in `PLAN.md`.

**Honesty (do not invent):** Live EXEC already ran; execute-allow restored off.
Research offline only. Loading a research external file ≠ product known-good.
Section names / sha match / helper “complete” alone **never** close the firmware
residual. VE2 xclbin ≠ AIE2 known-good.

### LIVE — research helpers for capture compare (`588c1c72`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Policy** | Empty reconfirm stamps **frozen**. Empty-folder implement **frozen**. `588c1c72` = named offline helpers only (bytes still absent). | **No** |
| **Silicon** | Unchanged: userspace ELF+rela OK; firmware **APP_LOAD_PDI_FAIL** `0x3000003`. | **No** |
| **Unlock** | `C_UNLOCK=b17b0119` **CONSUMED**. Do not open execute-allow under it. This stamp did **not** mint unlock. | **No** |
| **Capture dir** | No working-stack known-good `.bin` offline under `testdata/xdna2_cmd/external_known_good/`. Soft/helpers report blocked-on-external-bytes (section names without bytes still blocked). Local post_patch refused. VE2 xclbin refused (`platform_not_aie2`). | **No** |
| **Code surface** | `AppLoadExtractBagScorecardPipeline` / `research_app_load_extract_bag_scorecard_pipeline` (+ variants) — composes extract dry-run + capture-dir check + soft status. Alone does **not** close residual. Soft always: invent Pass off, not C-ready, not `ran_on_npu`, not product known-good. | **No** (research) |
| **Docs** | CONSTRUCTION_MAP; PROVENANCE template in capture dir; this file + PLAN + AGENTS. | **No** |
| **Next work** | **Human** places real bytes + PROVENANCE → then soft compare. Offline unit tests ≠ Pass ≠ unlock. Agent: **no empty implement loop**. | **No** until real bytes |

**Conclusion:** Offline helpers landed. Capture **bytes** still **absent**. Residual
still firmware load reject. **No** Pass. **No** second unlock. Do not re-spin
empty implement.

### Prior — bag + feed + dry-run composition scorecard (`2b01425e`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Unit surface** | `AppLoadBagFeedDryRunScorecard` / `research_app_load_bag_feed_dry_run_scorecard` / `_ex` / `_c2_locked_absent_bag` + `APP_LOAD_BAG_FEED_DRY_RUN_SCORECARD_NOTE` (`scorecard_alone_closes_fw_residual=0` + honesty trio hard-coded). Soft: `app_load_bag_feed_dry_run_scorecard=`. | **No** (research) |

**Conclusion (historical `2b01425e`):** Bag+feed+dry-run composition scorecard
landed. Superseded for LIVE named delta by `588c1c72`. Residual still FW
`0x3000003`.

### Prior — lab export fingerprint auto-feed into compare dry-run (`d2e287ab`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Unit surface** | `LabExportFingerprintFeed` / `research_lab_export_fingerprint_feed` / `_from_elf` / `_from_last_apply_*` + `research_external_ctrltext_compare_dry_run_from_lab_feed` / `_ex` (`feed_alone_closes_fw_residual=0` + `lab_export_auto_feed=1` + honesty trio hard-coded). Soft: `lab_export_fingerprint_feed=` + dry-run auto-feed. | **No** (research) |

**Conclusion (historical `d2e287ab`):** Lab export fingerprint auto-feed into
compare dry-run landed. Superseded for LIVE named delta by `2b01425e` then
`588c1c72`. Residual still FW `0x3000003`.

### Prior — external ctrltext compare dry-run (`6033158a`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Unit surface** | `ExternalCtrltextCompareDryRun` / `research_external_ctrltext_compare_dry_run` / `_ex` / `_c2_locked_absent_bag` + `lab_fingerprint_export_is_ready` (`dry_run_alone_closes_fw_residual=0` + `blocked_on_external_bytes`). | **No** (research) |

**Conclusion (historical `6033158a`):** External ctrltext compare dry-run +
explicit blocked_on_external_bytes landed. Superseded for LIVE named delta by
`d2e287ab` then `2b01425e` then `588c1c72`. Residual still FW `0x3000003`.

### Prior — offline progress scorecard + operator extract checklist (`9cdfe8d0`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Unit surface** | `AppLoadOfflineProgressScorecard` / `ExternalCompareStatus` / operator extract checklist (`scorecard_alone_closes_fw_residual=0`). | **No** (research) |

**Conclusion (historical `9cdfe8d0`):** Progress scorecard + extract checklist
landed. Superseded for LIVE named delta by `6033158a` then `d2e287ab` then
`2b01425e` then `588c1c72`. Residual still FW `0x3000003`.

### Prior — soft e2e fixture path + bag-drop checklist (`8e898123`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Unit surface** | `ExternalKnownGoodBagDropCheck` / `AppLoadE2eFixtureResearch` (`bag_drop_alone_closes_fw_residual=0` + always `residual_not_closed=1`). | **No** (research) |
| **Docs** | CONSTRUCTION_MAP unit surface; bag PROVENANCE fail-closed checklist. | **No** |

**Conclusion (historical `8e898123`):** Soft e2e fixture + fail-closed bag-drop
landed. Superseded for LIVE named delta by `9cdfe8d0`. Residual still FW `0x3000003`.

### Prior — axlf section body bounds + journal tokens (`2c4b96bf`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Unit surface** | `CuPdiSectionBodyBoundsResearch` / `check_axlf_section_body_bounds` / journal token parsers (`body_bounds_alone_closes_fw_residual=0`). | **No** (research) |
| **Docs** | CONSTRUCTION_MAP unit surface; LIVE pins then. | **No** |

**Conclusion (historical `2c4b96bf`):** Body bounds + pure journal token parsers
landed. Superseded for LIVE named delta by `8e898123` then `9cdfe8d0`. Residual still FW `0x3000003`.

### Prior — BITSTREAM/AIE_PARTITION section-offset map (`a5787723`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Unit surface** | `CuPdiSectionOffsetMapResearch` / `parse_axlf_section_header` / `walk_axlf_section_headers` / `research_xclbin_cu_pdi_section_offset_map` (`section_offset_map_alone_closes_fw_residual=0`). | **No** (research) |
| **Docs** | CONSTRUCTION_MAP unit surface; LIVE pins then. | **No** |

**Conclusion (historical `a5787723`):** Public axlf BITSTREAM/AIE_PARTITION
section-offset map landed. Superseded for LIVE named delta by `2c4b96bf` then
`8e898123` then `9cdfe8d0`. Residual still FW `0x3000003`.

### Prior — CU/PDI section identity + partition-id field map (`7a5ab864`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Unit surface** | `CuPdiSectionIdentityResearch` + `AppLoadPartitionIdFieldMap`; identity/map alone never close residual. | **No** (research) |
| **Docs** | CONSTRUCTION_MAP unit surface; LIVE pins then. | **No** |

**Conclusion (historical `7a5ab864`):** Section identity + partition-id field map landed.
Superseded for LIVE named delta by `a5787723` then `2c4b96bf` then `8e898123` then `9cdfe8d0`. Residual still FW `0x3000003`.

### Prior — residual axis matrix + xclbin extract dry-run (`ac20bc3f`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Unit surface** | `AppLoadResidualAxisMatrix` + `XclbinExtractResearchDryRun` / VE2 refuse; hard locks match-alone=0 + name-alone≠proof. | **No** (research) |
| **Docs** | CONSTRUCTION_MAP unit surface; LIVE pins then. | **No** |

**Conclusion (historical `ac20bc3f`):** Axis matrix + xclbin extract dry-run landed.
Superseded for LIVE named delta by `7a5ab864` then `a5787723` then `2c4b96bf` then `8e898123` then `9cdfe8d0`. Residual still FW `0x3000003`.

**Construction map (2026-07-20, offline):** See
[`docs/CONSTRUCTION_MAP_APP_LOAD.md`](docs/CONSTRUCTION_MAP_APP_LOAD.md)
(Unit surface + Capture procedure + extract recipe §). Public stack order: xclbin
PDI → CONFIG_CU → aiebu ELF → peel/patch ctrltext → START_NPU. Our lab:
CREATE_HWCTX only → aiebu ELF → peel/patch → START_NPU (no PDI). Userspace
ELF+rela still OK; residual still FW `0x3000003`. Map + TDD **≠** Pass / **≠** unlock.

**Highest-value next:** Follow operator extract checklist → place working-stack
ctrltext under `testdata/xdna2_cmd/external_known_good/` + fill PROVENANCE
(research-ready) → soft `XDNA2_LAB_EXTERNAL_CTRLTEXT` +
`compare_external_ctrltext_to_lab_export` / dry-run / composition scorecard /
extract pipeline → named payload/path delta if actionable → only then new
host-minted `C_UNLOCK`.
**Not** gate-only stamp. **Not** status-name Track P. Match alone ≠ residual closed.
Bag-drop complete + e2e fixture research alone ≠ residual closed (stamp `8e898123`).
Progress scorecard alone ≠ residual closed (stamp `9cdfe8d0`).
Extract pipeline alone ≠ residual closed (stamp `588c1c72`).

### Prior — extract recipe + PROVENANCE field checks + CU/PDI necessity (`f06d5b38`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Unit surface** | PROVENANCE field validation + `CuPdiNecessityResearch` (`ctrltext_match_alone_closes_fw_residual=0`). | **No** (research) |
| **Docs** | Public-stack extract recipe + PROVENANCE field table. | **No** |

**Conclusion (historical `f06d5b38`):** Recipe + provenance + CU/PDI necessity landed.
Superseded for LIVE named delta by `ac20bc3f`. Residual still FW `0x3000003`.

### Prior — external known-good confinement + load wiring (`fec9c58b`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Unit surface** | Path confinement + load/compare wiring. Soft always honesty + `product_known_good=0`. | **No** (research) |
| **Docs** | Capture procedure + bag PROVENANCE template. | **No** |

**Conclusion (historical `fec9c58b`):** Load wiring ready when bytes arrive.
Superseded for LIVE named delta by `f06d5b38`. Residual still FW `0x3000003`.

### Prior — fingerprint export + external compare pure TDD (`695e5ad0`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Unit surface** | `CtrltextFingerprintExport` / `export_ctrltext_fingerprint_research` + `ExternalCtrltextCompareResearch` / `compare_external_ctrltext_to_lab_export`. Soft always `known_good_present=0` / `product_known_good=0`. Review-fix Issues 1–6 closed. | **No** (research) |
| **Gates then** | `silicon_contract` **84**; fingerprint_ **9**. | **No** |

**Conclusion (historical `695e5ad0`):** Export + compare helper landed. Superseded
for LIVE load wiring by `fec9c58b`. Residual still FW `0x3000003`.

### Prior — construction path pure TDD (`045739e1`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Unit surface** | `ConstructionPathResearch` + `PatchedCtrltextFingerprint` locks gap + corpus. | **No** (research) |
| **Gates** | `silicon_contract` **79** (then); construction_path 5. | **No** |

**Conclusion (historical `045739e1`):** Path gap + corpus fingerprint unit surface.
Superseded for LIVE by `695e5ad0` export/compare. Residual still FW `0x3000003`.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `fcec5772`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `fcec5772`):** A–O public APP_LOAD evidence remain **closed**. No open pure-research bag justified without new external public evidence. Residual still FW `APP_LOAD_PDI_FAIL` after userspace ELF+rela OK. `C_UNLOCK=b17b0119` **CONSUMED**. **No** second C unlock. **No** Pass. **No** `ran_on_npu`. Offline green ≠ C-ready.

**Highest-value next (historical gate-only `fcec5772`; superseded by LIVE freeze):** Was “prefer gate-only reconfirm”; **superseded** — mill frozen; next = construction research.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `f668bd8c`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `f668bd8c` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `f668bd8c`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `cba9660a`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `cba9660a` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `cba9660a`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `53eedb11`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `53eedb11` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `53eedb11`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `4d207032`)


| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `4d207032` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `4d207032`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `5dee4157`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `5dee4157` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `5dee4157`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `4996a947`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `4996a947` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `4996a947`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `ff610ad9`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `ff610ad9` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `ff610ad9`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `269aa092`)


| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `269aa092` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `269aa092`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `ebf44221`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `ebf44221` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `ebf44221`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `a2d12be8`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `a2d12be8` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `a2d12be8`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `37dfffc0`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `37dfffc0` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `37dfffc0`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `f0e08ad2`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `f0e08ad2` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `f0e08ad2`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `112c4a41`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `112c4a41` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `112c4a41`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `2d45242f`)


| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `2d45242f` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `2d45242f`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `981409fd`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `981409fd` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `981409fd`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `30ed268b`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `30ed268b` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `30ed268b`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `55ed307c`)


| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `55ed307c` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `55ed307c`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `7920e3dd`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `7920e3dd` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `7920e3dd`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `02968960`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `02968960` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `02968960`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `44be26c4`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `44be26c4` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `44be26c4`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `3a2d4d3e`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `3a2d4d3e` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `3a2d4d3e`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `2efc79ff`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `2efc79ff` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `2efc79ff`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `70ee651b`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `70ee651b` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `70ee651b`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `3033a53f`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `3033a53f` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `3033a53f`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `86e8c0cb`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `86e8c0cb` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `86e8c0cb`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `725acb24`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `725acb24` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `725acb24`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `503416ee`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `503416ee` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `503416ee`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `cc81e07a`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `cc81e07a` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `cc81e07a`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `7ab2af6b`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `7ab2af6b` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `7ab2af6b`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `6b7c0fe8`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `6b7c0fe8` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `6b7c0fe8`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `7186bea2`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `7186bea2` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `7186bea2`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `e192d5aa`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `e192d5aa` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `e192d5aa`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `f0fad9bd`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `f0fad9bd` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `f0fad9bd`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `7637973c`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `7637973c` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `7637973c`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `d0ba3a35`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `d0ba3a35` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `d0ba3a35`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `4c305d6a`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `4c305d6a` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `4c305d6a`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `c266dafb`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `c266dafb` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `c266dafb`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `fcaecdd9`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `fcaecdd9` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `fcaecdd9`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `e7ee2176`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `e7ee2176` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `e7ee2176`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `b952f91b`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `b952f91b` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `b952f91b`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `a6874f86`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `a6874f86` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `a6874f86`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `0084ed8c`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `0084ed8c` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `0084ed8c`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `d4c159b4`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `d4c159b4` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `d4c159b4`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `11dc5889`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `11dc5889` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `11dc5889`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `8bf02bf6`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `8bf02bf6` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `8bf02bf6`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `ec778e6f`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `ec778e6f` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `ec778e6f`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `c6d632f1`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `c6d632f1` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `c6d632f1`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `05686144`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `05686144` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `05686144`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `0a6a0282`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `0a6a0282` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `0a6a0282`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `21fa5386`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `21fa5386` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `21fa5386`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `b6a66dee`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `b6a66dee` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `b6a66dee`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `dd35ae15`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `dd35ae15` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `dd35ae15`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `2eeb5265`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `2eeb5265` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `2eeb5265`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `acaf47e3`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `acaf47e3` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `acaf47e3`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.



### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `e20f9d48`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `e20f9d48` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `e20f9d48`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `4d26a1ad`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `4d26a1ad` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `4d26a1ad`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `5692781f`)


| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `5692781f` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `5692781f`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `76474e2d`)


| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `76474e2d` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `76474e2d`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `9335d13e`)


| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `9335d13e` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `9335d13e`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `7a7ef744`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `7a7ef744` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `7a7ef744`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `194c716c`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `194c716c` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `194c716c`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `8f432e65`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `8f432e65` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `8f432e65`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `50db8bcf`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `50db8bcf` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `50db8bcf`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `a4dc17e1`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `a4dc17e1` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `a4dc17e1`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `55af70b3`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `55af70b3` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `55af70b3`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `b8477e60`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `b8477e60` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `b8477e60`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `00847df4`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `00847df4` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `00847df4`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `943498d1`)


| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `943498d1` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `943498d1`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `8291083a`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `8291083a` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `8291083a`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `675d42bc`)


| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `675d42bc` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `675d42bc`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `e21dd1e1`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `e21dd1e1` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `e21dd1e1`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `5474de95`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `5474de95` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `5474de95`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `9282a077`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `9282a077` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `9282a077`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `ed014081`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `ed014081` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `ed014081`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `0e4f10ae`)


| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `0e4f10ae` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `0e4f10ae`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `46783eea`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `46783eea` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `46783eea`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `90c17ab9`)


| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `90c17ab9` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `90c17ab9`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `e3a96bee`)


| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `e3a96bee` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `e3a96bee`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `c702b0de`)


| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `c702b0de` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `c702b0de`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `45b30bae`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `45b30bae` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `45b30bae`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `4d15b9b2`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `4d15b9b2` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `4d15b9b2`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `ed2e4e43`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | `bash tools/hitl_c2_once.sh --print-only` → `C_UNLOCK=b17b0119 is **CONSUMED**` (not active one-shot; HISTORICAL record-only sandwich; do not open L3 under consumed). hitl_c `--assume-l3-already-open` with L3=N → rc=1 offline-only refuse (`hitl_c_rc=1` / `exit=1`; no must-open-L3 recipe). | **No** |
| **Tracks A–O** | still closed (O token `a_n_closed_no_new`). **No new external public evidence.** Prefer stamp-only. **No** Track P+ bag. | **No** |

**Conclusion (gate-only `ed2e4e43` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `ed2e4e43`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `7c5c4c4f`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | record-only sandwich; hitl_c L3-closed refuse offline-only under CONSUMED (`hitl_c_rc=1` / `exit=1`). | **No** |
| **Tracks A–O** | still closed. **No new external public evidence.** | **No** |

**Conclusion (gate-only `7c5c4c4f` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `7c5c4c4f`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.


### Offline APP_LOAD gate-only maintain reconfirm (prior / historical — `7ab1c6ec`)

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Gates** | `run_offline_gates` → `hard_ok=15 hard_fail=0 soft_ok=6 soft_warn=0 soft_skip=0`; `OFFLINE_GATES_OK`; L3=`N`; `XDNA2_ALLOW_EXEC_CMD` unset. | **No** |
| **Soft residual** | `app_load_pdi_fail_after_aiebu` + honesty trio `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`; C4_SKIP; TMPDIR=/ clamp green. Dual residual: userspace ELF+rela OK vs FW APP_LOAD; checklist splits unchanged. | **No** |
| **hitl CONSUMED** | record-only sandwich; hitl_c L3-closed refuse offline-only under CONSUMED. | **No** |
| **Tracks A–O** | still closed. **No new external public evidence.** | **No** |

**Conclusion (gate-only `7ab1c6ec` — historical):** Same closed A–O state. Reconfirmed under `fcec5772` — still closed; **no** Track P bag invented.

**Highest-value next (historical gate-only `7ab1c6ec`; superseded by `fcec5772`):** Prefer gate-only reconfirm + stamp while A–O closed.

## Priority branch (single source of truth) — LIVE `588c1c72`; C2 **done**; offline green; **no** Pass

**Single source of truth for current state.** Operator C2 complete; `C_UNLOCK=b17b0119` **CONSUMED**. Tracks A–O closed. LIVE stamp `588c1c72` (extract→bag-drop→composition scorecard pipeline; bytes still absent); prior bag+feed scorecard `2b01425e`; lab export auto-feed `d2e287ab`; external compare dry-run `6033158a`; progress scorecard + extract checklist `9cdfe8d0`; soft e2e + bag-drop `8e898123`; body bounds + journal tokens `2c4b96bf`; section-offset map `a5787723`; section identity + partition-id `7a5ab864`; residual axis + xclbin extract `ac20bc3f`; extract/PROVENANCE/CU-PDI `f06d5b38`; load wiring `fec9c58b`; export/compare `695e5ad0`; construction TDD `045739e1`; last empty gate-only `fcec5772`.

| File / gate | State **now** |
|-------------|----------------|
| Classic vendor body | **EXISTS** — 1688 B CLASS A unpatched |
| Offline gates | **Green** (see Self-runnable; hard_fail=0) |
| Pure-direct patch | Unit-locked; **≠** true post-patch |
| Fingerprint export | **Ready** — unpatched sha + structural; `known_good_status=absent_awaiting_capture` |
| Lab export auto-feed | **Ready** (`d2e287ab`) — ELF/export path → dry-run; signals alone incomplete; feed alone ≠ residual closed |
| Bag+feed+dry-run scorecard | **Ready** (`2b01425e`) — composition soft row; bag absent after feed → `blocked_on_external_bytes=1`; scorecard alone ≠ residual closed |
| Extract→bag→scorecard pipeline | **Ready** (`588c1c72`) — extract markers + bag-drop + composition; markers without bytes → still `blocked_on_external_bytes=1`; pipeline alone ≠ residual closed |
| External load path | **Ready** (`fec9c58b`) — confined bag + env; post_patch oracles refused |
| PROVENANCE fields | **Ready** (`f06d5b38`) — research-ready fail-closed before lab-bag load |
| Bag-drop checklist | **Ready** (`8e898123`) — fail-closed complete; incomplete while bytes absent; drop alone ≠ residual closed |
| Soft e2e fixture path | **Ready** (`8e898123`) — class + journal + body bounds + bag drop; always residual_not_closed |
| Offline progress scorecard | **Ready** (`9cdfe8d0`) — composes class + bag + e2e + compare status; scorecard alone ≠ residual closed |
| Operator extract checklist | **Ready** (`588c1c72` stamp; prior `2b01425e`/`d2e287ab`/`6033158a`/`9cdfe8d0` steps) — pure extract-to-bag + dry-run blocked + lab auto-feed + composition scorecard step10 + extract pipeline step11; no invent bytes |
| External compare dry-run | **Ready** (`6033158a` + `d2e287ab` auto-feed) — bag absent → `blocked_on_external_bytes=1`; lab ready when ELF; refuse post_patch/VE2; match ≠ Pass; dry-run alone ≠ residual closed |
| CU/PDI necessity | **Ready** (`f06d5b38`) — match alone never closes residual; gap+FW open hypothesis |
| Residual axis matrix | **Ready** (`ac20bc3f`) — partition vs CU/PDI vs ctrltext; name/match alone never close |
| Xclbin extract dry-run | **Ready** (`ac20bc3f`) — public markers; VE2 NOT_AIE2 refuse; no PDI invent |
| CU/PDI section identity | **Ready** (`7a5ab864`) — `AIE_PARTITION`/`BITSTREAM` name surface; identity alone ≠ residual closed |
| Partition-id field map | **Ready** (`7a5ab864`) — journal fields mapped; `partition_id=absent_public`; map alone ≠ close |
| Section-offset map | **Ready** (`a5787723`) — public axlf BITSTREAM/AIE_PARTITION file offsets; map alone ≠ residual closed; no body decode |
| Section body bounds | **Ready** (`2c4b96bf`) — body windows vs file_len; OOB refuse; bounds alone ≠ residual closed; no body decode |
| Journal token parsers | **Ready** (`2c4b96bf`) — pure `failed_cmd` / `chain_status` line tokens; never invent residual closed |
| External known-good | **Absent** offline (awaiting capture); compare + load + provenance + bag-drop + scorecard + pipeline unit-locked |
| last_apply | **2026-07-18** live C2 — `ert_cmd_error` `ran_on_npu=false` aiebu=1 rela=12 layout=xrt3 |
| C3 pipeline | **`C3_SKIP`** same residual body FP as locked baseline |
| C4 pipeline | **`C4_SKIP`** no Pass |
| HITL C | One-shot **CONSUMED**; further C needs **new unlock** |
| C2 live | **Done** — lab rc=4; L3 restored; residual APP_LOAD |
| Pass? | **No** — need ERT Completed + non-zero OFM allclose + `ran_on_npu=true` |
| Tracks A–O | **Closed** (`a_n_closed_no_new`); no new external public known-good bytes |

### Highest-value next (single table — single source of truth)

| Priority | Action | Who |
|----------|--------|-----|
| **1** | Follow operator extract checklist; obtain known-good **external** ctrltext and/or CU PDI bytes offline; place under `testdata/xdna2_cmd/external_known_good/` + research-ready PROVENANCE; confirm bag-drop complete; feed via `XDNA2_LAB_EXTERNAL_CTRLTEXT` / `load_external_ctrltext_for_compare` → `compare_external_ctrltext_to_lab_export` / dry-run with lab export auto-feed / bag+feed+dry-run scorecard / extract→bag→scorecard pipeline (match ≠ residual closed; pipeline alone ≠ close; scorecard alone ≠ close; dry-run alone ≠ close; feed alone ≠ close; extract alone ≠ close; bag absent or markers-only → `blocked_on_external_bytes=1`) | agent / operator offline |
| **2** | Maintain `bash tools/run_offline_gates.sh` green; keep L3=`N`; ALLOW unset; **no** empty gate-only mill stamps | agent |
| **3** | Public APP_LOAD depth **only** with **new** external public evidence. Else do **not** invent Track P+ | agent |
| **4** | **Another live C only after new agent unlock** + offline delta (one-shot `b17b0119` **CONSUMED** 2026-07-18; `588c1c72` / `2b01425e` / `d2e287ab` / `6033158a` / `9cdfe8d0` / `8e898123` / `2c4b96bf` / `a5787723` / `7a5ab864` / `ac20bc3f` / `f06d5b38` / `fec9c58b` / `695e5ad0` / `045739e1` / A–O + gate-only `fcec5772` **did not** issue a second unlock) | agent then operator |
| **5** | Optional capture-instr logs only (true post-patch public path **closed**; not P0) | operator optional |
| **ban** | Env C spam; invent vendor / private PDI decode; L3 from agent; claim Pass; second C without new unlock; re-open INSTR_BO as P0; invent Track P+; prop_count=15 invent; treat pure-direct post_patch as known-good; treat VE2 xclbin as AIE2 known-good; invent partition_id from journal; treat section-offset map / body bounds / bag-drop complete / e2e fixture / progress scorecard / compare dry-run / lab export feed / bag+feed+dry-run scorecard / extract→bag→scorecard pipeline as residual closed | all |

**C policy (honesty):** `C_UNLOCK=b17b0119` is **CONSUMED** (operator C2 2026-07-18). Further C requires a **new agent unlock** + offline delta. Evidence never auto-unlocks. Unpatched CLASS A, registry BO, parity green, `CONTRACT_OK`, soft residual_class / CLASSIFY_SOFT_OK / checklist, and `OFFLINE_GATES_OK` **never** unlock C and **never** equal Pass. Under CONSUMED: record-only sandwich; **no** must-open-L3; hitl_c L3-closed refuse stays offline-only.

```bash
# HISTORICAL / CONSUMED record-only — do NOT open L3 under b17b0119
# bash tools/hitl_c2_once.sh --print-only
# Offline path under CONSUMED:
bash tools/c3_post_c2_classify.sh
bash tools/c4_pass_check.sh
bash tools/run_offline_gates.sh
# Future unlock only (not under CONSUMED):
# sudo sh -c 'echo 1 > /sys/module/amdxdna/parameters/exec_cmd_allow'
# bash tools/hitl_c_pass_apply.sh --assume-l3-already-open
# sudo sh -c 'echo 0 > /sys/module/amdxdna/parameters/exec_cmd_allow'
```

### Self-runnable gates (agent offline now)

**One-shot:** `bash tools/run_offline_gates.sh` → `OFFLINE_GATES_OK` (hard unit/style + soft classify/assets; **≠ C-ready**).

| Script / test | Expect (offline / LIVE `588c1c72`) |
|---------------|----------------------------------------|
| `bash tools/run_offline_gates.sh` | **0** `hard_fail=0` soft green `OFFLINE_GATES_OK` (≠ Pass ≠ unlock ≠ `ran_on_npu`) |
| `cargo test --features amd-xdna2 --lib fingerprint_` | **0** export + compare + corpus locks |
| `cargo test --features amd-xdna2 --lib external_known_good` | **0** confinement + load + refuse post_patch + PROVENANCE fields + bag-drop + extract checklist |
| `cargo test --features amd-xdna2 --lib bag_drop` | **0** fail-closed bag-drop completeness |
| `cargo test --features amd-xdna2 --lib app_load_e2e_fixture` | **0** soft e2e fixture path (residual_not_closed + honesty) |
| `cargo test --features amd-xdna2 --lib app_load_offline_progress` | **0** progress scorecard (incomplete + residual_not_closed + honesty) |
| `cargo test --features amd-xdna2 --lib external_ctrltext_compare_dry_run` | **0** dry-run blocked_on_external_bytes + refuse + match ≠ Pass |
| `cargo test --features amd-xdna2 --lib lab_export_fingerprint_feed` | **0** auto-feed ready/blocked + signals incomplete + refuse + honesty + match ≠ Pass |
| `cargo test --features amd-xdna2 --lib app_load_bag_feed_dry_run_scorecard` | **0** composition scorecard blocked_on_external_bytes + refuse + match ≠ Pass + honesty |
| `cargo test --features amd-xdna2 --lib app_load_extract_bag_scorecard` | **0** extract→bag→scorecard pipeline blocked_on_external_bytes + markers still blocked + refuse + match ≠ Pass + honesty |
| `cargo test --features amd-xdna2 --lib operator_extract_checklist` | **0** pure extract-to-bag refuse tokens (stamp `588c1c72` + step11) |
| `cargo test --features amd-xdna2 --lib cu_pdi_necessity` | **0** CU/PDI necessity residual matrix |
| `cargo test --features amd-xdna2 --lib cu_pdi_section` | **0** CU/PDI section identity (public names) |
| `cargo test --features amd-xdna2 --lib app_load_residual_axis` | **0** partition vs CU/PDI vs ctrltext axis matrix |
| `cargo test --features amd-xdna2 --lib app_load_partition_id` | **0** journal partition-id field map (`absent_public`) |
| `cargo test --features amd-xdna2 --lib xclbin_` | **0** extract dry-run + VE2 refuse + markers + section-offset map + body bounds |
| `cargo test --features amd-xdna2 --lib journal_failed_cmd` | **0** pure failed_cmd / chain_status token parsers |
| `cargo test --features amd-xdna2 --lib construction_path` | **0** path-gap locks |
| `bash tools/classify_last_apply_residual.sh` | **0** soft; `app_load_pdi_fail_after_aiebu` + `not_c_ready=1 not_ran_on_npu=1 invent_pass=0` |
| `bash tools/c4_pass_check.sh` | **0** `C4_SKIP` (`json_ran_on_npu_false`) |
| `bash tools/hitl_c2_once.sh --print-only` | **0** CONSUMED note; HISTORICAL record-only sandwich |
| `bash tools/hitl_c_pass_apply.sh --assume-l3-already-open` when L3=N | **exit 1** (`hitl_c_rc=1` / `exit=1`); offline-only under CONSUMED |
| `bash tools/smoke_tmpdir_root_clamp.sh` | **0** TMPDIR=/ fail-closed clamp |
| L3 / ALLOW | `exec_cmd_allow=N`; `XDNA2_ALLOW_EXEC_CMD` unset |

**Locked residual matrix (single source of truth):**

| Layer | State |
|-------|--------|
| Userspace | ELF+aiebu+rela×12 + xrt3 + content 1688 — **OK** |
| FW | `APP_LOAD_PDI_FAIL` (`0x3000003`) + `INVALID_PARAM` (`0x4000003`) — residual |
| Dual residual | userspace OK vs FW APP_LOAD; checklist splits unchanged |
| Honesty | `invent_pass=0` `not_c_ready=1` `not_ran_on_npu=1` always |

### Operator C2 result (2026-07-18 — historical evidence)

| Field | Live result |
|-------|-------------|
| Lab exit | **4** (`!ran_on_npu`) |
| POST L3 | **`N` / closed** |
| Residual class | **`app_load_pdi_fail_after_aiebu`** |
| Journal | **`0x3000003` APP_LOAD_PDI_FAIL** + **`0x4000003` INVALID_PARAM** |
| C_UNLOCK | **`b17b0119` CONSUMED** |

### Tracks A–O — prior / historical (closed)

Tracks A–E, F–J, K, L, M, N, O conclusions — **historical**. O token `a_n_closed_no_new`. M `new_public_pdi_fields=0`. N `closed_no_new`. No new external public known-good bytes on disk yet. Then `695e5ad0` export/compare; `fec9c58b` load/confinement; `f06d5b38` recipe + PROVENANCE + CU/PDI necessity; `ac20bc3f` residual axis + xclbin extract; `7a5ab864` CU/PDI section identity + partition-id; `a5787723` section-offset map; `2c4b96bf` body bounds + journal tokens; `8e898123` soft e2e fixture + bag-drop; `9cdfe8d0` progress scorecard + extract checklist; `6033158a` external ctrltext compare dry-run + `blocked_on_external_bytes`; `d2e287ab` lab export fingerprint auto-feed into dry-run; `2b01425e` bag+feed+dry-run composition scorecard; LIVE named delta `588c1c72` (extract→bag→scorecard pipeline; markers without bytes still `blocked_on_external_bytes=1`). Empty gate-only mill still frozen. Do **not** invent Track P+ / PDI layout / INSTR_BO as P0.

**Highest-value next (historical O/M/N; then `695e5ad0` / `fec9c58b` / `f06d5b38` / `ac20bc3f` / `7a5ab864` / `a5787723` / `2c4b96bf` / `8e898123` / `9cdfe8d0` / `6033158a` / `d2e287ab` / `2b01425e`; superseded for LIVE by `588c1c72`):** Prefer external known-good capture under `external_known_good/` + research-ready PROVENANCE + bag-drop complete + confined compare / composition scorecard / extract pipeline; not empty gate-only mill.

*(Highest-value single source of truth = single Priority table under “Highest-value next (single table — single source of truth)” above — LIVE stamp `588c1c72`.)*
