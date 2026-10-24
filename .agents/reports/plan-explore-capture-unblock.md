# Plan explore: Track B capture as Pass unblock

**Date:** 2026-08-10
**Scope:** read-only map of external known-good capture (path B) and what it can unblock.
**Honesty:** invent_pass=0 · not C-ready · not ran_on_npu · capture ≠ Pass · match ≠ residual closed.

---

## Plain status

| Item | Today |
|------|--------|
| Capture dir | `/home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/external_known_good/` has **only** tracked `PROVENANCE.md` — **no** `.bin` / `.ctrltext` |
| Soft compare | Ready offline; reports `blocked_on_external_bytes=1` until human drops real bytes |
| Live residual (stock) | Latest T4 under `C_UNLOCK=97f38307` **CONSUMED**: `wait_cmd_einval` + orphan IO_PAGE_FAULT after research-blob CONFIG_CU (not Pass). Historical fixture still APP_LOAD `0x3000003` |
| Unlock | **No** active one-shot. Do not reopen `97f38307` / `11bc9016` / `3a70fabf` / `b17b0119` |
| Path B vs Pass | Path B is **research identity** for instruction/ctrltext. Alone it does **not** grant product Pass. Pass still needs live ERT Completed + non-zero OFM allclose |

---

## 1. Capture directory (`external_known_good/`)

**Path:** `/home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/external_known_good/`

| Contents | Role |
|----------|------|
| `PROVENANCE.md` only | Tracked template: research-only honesty, refuse table, bag-drop checklist, operator extract steps, empty **Capture record** table |
| No `.bin` / `.ctrltext` | Soft helpers → incomplete bag-drop / `blocked_on_external_bytes=1` |

**PROVENANCE expects when bytes land:**

- Working public-stack **post-patch ctrltext** (example basename `working_stack_ctrltext.bin` or `*.ctrltext`)
- Optional `notes_cu_pdi.txt` (CONFIG_CU/PDI sizes; no private PDI decode)
- Filled capture table: date, source stack, key/shape, basename, sha256, unpatched vs post-rela, CONFIG_CU/PDI on that run?

**Refused as “external known-good” (local oracles):**
`instr_post_patch.bin`, `instr_vendor_post_patch.bin`, `ctrl_post_patch.bin`, `instr_raw_txn.bin`, `instr_vendor_txn_bin.bin`, plus script also refuses `lab_instr.bin` / `lab_instr.elf`. VE2 / `NOT_AIE2` names refused as AIE2 known-good.

**Gitignore (`.gitignore` ~102–118):**

- `testdata/xdna2_cmd/**` ignored broadly
- Un-ignore dirs + `**/PROVENANCE.md` + `**/*.sha256` + `**/*.txt` + README
- `testdata/**/*.bin` and `testdata/**/*.elf` stay ignored
→ **Bins under external_known_good stay out of git; PROVENANCE stays tracked.** Correct.

---

## 2. Operator tools (path B)

### `tools/operator_external_capture.sh`

| Mode | Behavior |
|------|----------|
| default | Checklist + bag status |
| `--status` | Dir/PROVENANCE/bins; exit **1** if BINS=0 |
| `--print-steps` | Compact install + compare commands |
| `--validate PATH` | Non-empty + refuse basename/VE2 |
| `--install PATH [NAME]` | Safe copy into bag (default name `working_stack_ctrltext.bin`); no clobber |
| `--compare` | Soft classify `last_apply` with first bag bin via `XDNA2_LAB_EXTERNAL_CTRLTEXT` + PRINT flag |

Never: L3, `XDNA2_ALLOW_EXEC_CMD`, invent Pass, second unlock.

### `tools/operator_app_load_kit.sh` (B subcommands)

| Cmd | Delegates to |
|-----|----------------|
| `b-status` | `operator_external_capture.sh --status` |
| `b-steps` | `--print-steps` |
| `b-install PATH [NAME]` | `--install` |
| `b-compare` | `--compare` |

Overview also prints path A (CONFIG_CU / PDI) and current unlock CONSUMED notes. Path A is **orthogonal** research (matmul-matched PDI / CONFIG_CU), not a substitute for path B ctrltext identity.

---

## 3. Soft compare APIs (code surface)

**Home:** `src/backends/amd_xdna2/silicon_contract.rs` (re-exported via amd-xdna2).
**Wire-up:** `examples/classify_last_apply.rs` under `XDNA2_LAB_PRINT_C2_CMDBUF_RESEARCH=1`.

| API | Role |
|-----|------|
| `export_ctrltext_fingerprint_research(elf)` | Lab unpatched identity (len/sha/classic profile) — never invents known-good |
| `compare_external_ctrltext_to_lab_export(lab, external)` | Soft len/sha/structural; absent → `known_good_absent`; **always** soft `invent_pass=0 product_known_good=0 known_good_present=0` |
| `external_known_good_path_is_allowed` / `_load_path_is_allowed` | Confinement: temp ∪ bag root; `.bin`/`.ctrltext`; no `..`; refuse local oracle basenames; lab bag needs research-ready PROVENANCE |
| `load_external_ctrltext_for_compare` / `load_external_ctrltext_from_env` | Size-capped load; Unix `O_NOFOLLOW`; env `XDNA2_LAB_EXTERNAL_CTRLTEXT` |
| `check_external_known_good_bag_drop` | Pure fail-closed completeness; `bag_drop_alone_closes_fw_residual=0` |
| `format_external_known_good_operator_extract_checklist` | Pure 11-step extract-to-bag checklist |
| Dry-run / feed / scorecard / extract pipeline | `research_external_ctrltext_compare_dry_run*`, `research_lab_export_fingerprint_feed*`, `research_app_load_bag_feed_dry_run_scorecard*`, `research_app_load_extract_bag_scorecard_pipeline*` — bag absent → explicit `blocked_on_external_bytes=1`; alone_closes always 0 |
| CU/PDI necessity / axis matrix | Hard lock: **`ctrltext_match_alone_closes_fw_residual=0`** even on sha match |

**Unit filters (docs):**
`cargo test --features amd-xdna2 --lib external_known_good` · bag_drop · operator_extract · app_load_offline_progress (and related scorecard names in CONSTRUCTION_MAP).

---

## 4. Human vs agent once a real path exists

### Human only (cannot invent)

1. Run a **working** public stack (XRT / DynamicDispatch / Ryzen AI EP) that actually completes on NPU for a comparable mladf key when possible (`…_1_1152_256_128`).
2. Capture **instruction image after host peel/patch** (ctrltext that goes into instr BO / START_NPU), or aiebu ELF with documented peel.
3. Optionally note whether that run used **CONFIG_CU / PDI** and sizes (not a full private PDI dump required for path B).
4. Install:
   `bash tools/operator_app_load_kit.sh b-install /path/to/ctrltext.bin working_stack_ctrltext.bin`
5. Fill capture table in `external_known_good/PROVENANCE.md` (keep research-only + source/stack markers; no invent tokens).
6. Soft compare:
   `bash tools/operator_app_load_kit.sh b-compare`
   Paste soft lines to agent.
7. Any **new live** apply still needs a **new host-minted** `C_UNLOCK` + named offline reason (unlock CONSUMED today).

### Agent can do immediately after bytes + PROVENANCE land (offline)

- Confirm bag-drop complete + load confinement (status, unit tests, soft classify).
- Run soft compare vs lab export (`silicon_lab` / staged ELF fingerprint).
- Interpret: **match** (same as our failing peel) vs **structural match sha_diff** vs **content_diff** — each has different residual hypotheses.
- Update residual / construction notes from evidence (still invent_pass=0).
- If match shows public stack also uses same unpatched body, push research toward **CU/PDI construction / BO map / MMU** (path A matmul-matched PDI, driver fault addr), not another empty ctrltext reconfirm.
- If mismatch, plan offline byte-diff / patch strategy against known-good ctrltext (still not Pass until live accept + OFM).
- **Must not:** invent capture bytes; claim Pass/ran_on_npu from compare; reopen CONSUMED unlock; empty implement loop while dir was empty (now unblocked only for **named** compare/delta work).

### Still human (or new unlock) after offline compare

- Live lab apply of a corrected blob (new unlock + L3 sandwich if hardened host).
- Product Pass: `decide_ran_on_npu` with real ioctl + ERT Completed + OFM allclose.
- Matmul-matched PDI source if compare + live still show CU/PDI gap (path A; research CVML dump already measured and is **not** product known-good).

---

## 5. What capture needs for Pass *progress* (ctrltext? PDI? both?)

### Path B deliverable (required for this track)

| Need | Why |
|------|-----|
| **Working-stack post-patch ctrltext** (`.bin` or `.ctrltext`) | Soft identity vs lab unpatched peel; answers “are we patching the same instruction body a public stack uses?” |
| **PROVENANCE** research-ready | Load path fail-closed without it under bag root |
| Prefer same **mladf key** as lab | Fair compare (lab corpus: classic 1688 B, rela×12, key `…_1_1152_256_128`) |
| Document unpatched vs post-rela | Avoid false sha miss |

### Not required for bag-drop / soft compare, but high value for Pass residual

| Optional / parallel | Why |
|---------------------|-----|
| **CONFIG_CU / PDI presence note** (and sizes) | Construction map: public stack often CONFIG_CU **then** ctrltext; lab may be ctrltext-only. `ctrltext_match_alone_closes_fw_residual=0` |
| **Matmul-matched PDI** (path A) | Research CVML/AIE2P PDI was live-shot under `97f38307` and still WAIT EINVAL / orphan fault; product Pass likely needs **right** PDI identity, not random overlay |
| Full ELF vs peel | Bag expects **raw ctrltext**, not whole `.elf` (extension allowlist) |

### What Pass itself still requires (after any capture)

Product Pass is **not** soft compare green. It is:

1. Live apply path that FW accepts (ERT Completed, not APP_LOAD / WAIT EINVAL class alone).
2. Non-zero OFM + golden allclose → `decide_ran_on_npu` / product Pass criteria (C4/P6 offline readiness already exist; live still no).
3. No invent from offline-only fingerprints.

**Bottom line:** Path B unblocks **offline root-cause ranking** (ctrltext identity). Pass still needs **live** accept + OFM; residual evidence after T4 also points at **BO map / orphan IO_PAGE_FAULT** and **matmul-matched PDI**, so capture alone may prove “ctrltext already OK → focus CU/PDI/MMU” or “ctrltext wrong → fix patch.”

---

## Critical files (absolute)

| Path | Role |
|------|------|
| `/home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/external_known_good/PROVENANCE.md` | Template + honesty + capture table |
| `/home/hunter/Projects/ai/dragon-npu/tools/operator_external_capture.sh` | Path B kit |
| `/home/hunter/Projects/ai/dragon-npu/tools/operator_app_load_kit.sh` | A+B umbrella (`b-*`) |
| `/home/hunter/Projects/ai/dragon-npu/src/backends/amd_xdna2/silicon_contract.rs` | Load/compare/bag-drop/scorecard pure APIs |
| `/home/hunter/Projects/ai/dragon-npu/examples/classify_last_apply.rs` | Soft print wire |
| `/home/hunter/Projects/ai/dragon-npu/docs/CONSTRUCTION_MAP_APP_LOAD.md` | Lab vs public CU/PDI + unit surface |
| `/home/hunter/Projects/ai/dragon-npu/RESIDUAL.md` | SSoT: capture absent; unlocks CONSUMED; no Pass |
| `/home/hunter/Projects/ai/dragon-npu/.gitignore` | Bins ignored; PROVENANCE tracked |

---

## Gaps

1. **No real capture bytes** in `external_known_good/` (blocker for all soft compare non-absent paths).
2. **No agent-runnable “extract from working public stack”** — human must obtain ctrltext from a stack that already works.
3. **Path B does not ship PDI** — PDI progress is path A / separate matmul-matched source; bag optional notes only.
4. **Unlock CONSUMED** — offline compare can run without unlock; any live re-apply needs new host mint + offline reason.
5. **Latest live class** is WAIT EINVAL + IO_PAGE_FAULT after research CONFIG_CU, not only historical APP_LOAD — capture may not fix map orphan by itself.
6. Empty implement / reconfirm while capture missing remains **frozen** (AGENTS §6–9).

---

## Recommended steps if capture path is chosen

1. Human: get working public-stack ctrltext (same key if possible).
2. `bash tools/operator_app_load_kit.sh b-install /path/to/file.bin working_stack_ctrltext.bin`
3. Fill PROVENANCE capture table (research-only language intact).
4. `bash tools/operator_app_load_kit.sh b-compare` → paste soft lines.
5. Agent offline only:
   - Classify match vs structural-only vs full diff.
   - If **sha match lab unpatched**: treat as evidence lab body identity OK → prioritize CU/PDI/MMU residual (matmul-matched PDI, fault map), not more ctrltext mills.
   - If **diff**: plan named offline delta (patch/export fix) against known-good bytes; still invent_pass=0.
6. Do **not** claim Pass, reopen CONSUMED unlocks, or auto-queue empty reconfirm.
7. Live re-shot only after **new** unlock justified by that named offline delta.

---

## Honesty footer

```
invent_pass=0
not_c_ready=1
not_ran_on_npu=1
product_known_good=0
bag_drop_alone_closes_fw_residual=0
ctrltext_match_alone_closes_fw_residual=0
capture_bytes_present=0   # as of this explore
C_UNLOCK active=none (97f38307 CONSUMED)
```

No product code edits in this explore.
