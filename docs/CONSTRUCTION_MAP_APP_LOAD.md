# Construction map — how loadable bytes are built (APP_LOAD residual)

**Date:** 2026-07-20  
**Status:** Offline research notes only. **Not** a Pass. **Not** a live unlock.
**Not** a new lettered track. Named question: *how does a working public stack
build what firmware loads, and what does our lab build instead?*

Policy: empty maintain stamps **frozen**; empty-folder implement **frozen**
(`AGENTS.md` §6–9). Public status
name `APP_LOAD_PDI_FAIL == 0x3000003` is confirmatory (already locked) — this
doc is about **construction steps and bytes**, not re-listing error enums.

---

## Plain English goal

Firmware rejected our job with **load failed** (`0x3000003`) even after we sent
a proper-looking ELF instruction image (rela patched, layout green offline).
To unblock, we need to see **how a working stack constructs the bytes** the
firmware accepts — then diff that against what we submit.

---

## Two different “loadable” things (easy to conflate)

| Piece | What it is | Who builds it in the public stack | Do we send it? |
|-------|------------|-------------------------------------|----------------|
| **CU / PDI** | Per-compute-unit app image from **xclbin** | XRT shim: copy PDI into BO → **CONFIG_HWCTX / CONFIG_CU** → driver mailbox `MSG_OP_CONFIG_CU` with PDI device address | **No** — pure-direct never loads xclbin PDIs |
| **Instruction / ctrltext** | Tile control program for one job (classic txn after aiebu) | DD txn dump → **aiebu-asm** → ELF → peel `.ctrltext` → patch `.rela.dyn` with buffer paddrs → **START_NPU** `instruction_buffer` | **Yes** — this is what `lab_instr.elf` + Rust rela patch do |

The error name says “load PDI,” but on a live EXEC it can fire when the
**application firmware** rejects the job’s control image. The public driver only
forwards an address and checks status — it does **not** document FW acceptance
rules. Still: the **working path always has a CU/PDI step we skip**. That is a
**structural construction gap**, not a status-name catalog row.

Sources (local trees on this machine):

- Driver: `~/Projects/kernel/xdna-driver/src/driver/amdxdna/aie2_message.c`
  (`aie2_config_cu`, `aie2_register_pdis`)
- Shim: `…/src/shim/kmq/hwctx.cpp` (xclbin → PDI BOs → CONFIG_CU)
- Shim test ELF path: `…/test/shim_test/io.cpp` + `exec_buf.h`
  (aiebu assembler → `add_ctrl_bo` + `patch_ctrl_code` on `.ctrltext`)
- Our stage: `tools/stage_aiebu_lab_instr.sh`
- Failing corpus: `testdata/xdna2_cmd/silicon_lab/lab_instr.elf` +
  `testdata/xdna2_cmd/last_apply.json`

---

## Working public stack (construction order)

```text
1. xclbin (from higher stack: Ryzen AI / XRT / ONNX EP)
        │
        ▼
2. CREATE_HWCTX (partition / tiles / QoS)
        │
        ▼
3. For each CU in xclbin:
     - extract PDI bytes
     - CREATE_BO (cacheable) + copy + sync host→device
     - CONFIG_HWCTX(CONFIG_CU): { cu_bo, cu_func }
        │
        ▼
4. Driver aie2_config_cu → mailbox MSG_OP_CONFIG_CU
     packs PDI device addr (shifted) + function id
        │
        ▼
5. Per job: instruction image
     DynamicDispatch classic txn (e.g. 1688 B) + ctrl packet
        │
        ▼
6. aiebu-asm (type aie2txn): txn (+ optional ctrl) → ELF
     sections: .ctrltext (program) + often .ctrldata (ctrl packet)
               + .rela.dyn + .dynsym
        │
        ▼
7. Host peels .ctrltext, applies .rela.dyn (buffer paddrs), fills instr BO (Dev)
        │
        ▼
8. START_NPU: instruction_buffer = instr BO paddr,
              instruction_buffer_size = content length,
              instruction_prop_count = 0
     kernel args: IFM / WTS / OFM (and sometimes CTRL)
        │
        ▼
9. EXEC_CMD + WAIT → ERT Completed + non-zero OFM  (Pass criteria)
```

Shim test `elf_io` path implements steps 6–8 explicitly via
`aiebu::aiebu_assembler` and `patch_ctrl_code`. Production KMQ hwctx with
xclbin implements steps 1–4.

---

## Our pure-direct lab path (what actually ran)

```text
1. CREATE_HWCTX (tiles=32, QoS gops/fps non-zero)  — no xclbin, no CU list
        │
        ▼
2. (skip) CONFIG_CU / PDI register  — ctx has no cus → driver config_cu no-op
        │
        ▼
3. Stage: DD dump → lab_instr.bin (1688 B classic v0.1, gen4, 8×6 class)
           + lab_ctrl.bin (29664 B)
        │
        ▼
4. aiebu-asm -t aie2txn -c lab_instr.bin -p lab_ctrl.bin → lab_instr.elf
   Measured on disk (silicon_lab):
     - ELF size 32264 B
     - .ctrltext 1688 B
     - .ctrldata 29664 B
     - .rela.dyn 12 entries (sym indices 1→IFM×8, 2→WTS×2, 3→OFM×2)
     - dynsym names opaque all "6"
        │
        ▼
5. Live EXEC: peel .ctrltext, rela overwrite with live paddrs,
   Dev BO content=1688 page-padded bo_size=4096, map_offset=no-mmap (expected Dev)
        │
        ▼
6. START_NPU size=content 1688, layout=xrt3 (15 u32 kernel args), CTRL separate BO
        │
        ▼
7. Result (last_apply, operator C2): real_ioctl=true, aiebu_elf=1, rela=12,
   ert_state=Error, OFM zeros, journal 0x3000003 (+ chain 0x4000003)
   → ran_on_npu=false
```

**Userspace contract for steps 4–6 is green offline.** Residual is firmware
reject after that — not “forgot aiebu.”

Note: `testdata/xdna2_cmd/lab_instr.elf` at **repo root of that dir** is a
**different** tiny ELF (1072 B, 416 B ctrltext) — research sample only. Live
silicon corpus is **`silicon_lab/lab_instr.elf`**.

---

## Side-by-side: what matches vs what might still differ

| Item | Working / XRT-ish | Our failing run | Already tried / closed? |
|------|-------------------|-----------------|-------------------------|
| Classic txn body 1688 | DD extract | Same class (CLASS A unpatched body may exist) | Env body parity offline green; not Pass |
| aiebu ELF + rela×12 | Yes | Yes | Closed as “forgot ELF” |
| START_NPU prop_count=0 | Yes | Yes | Contract |
| Instr BO type Dev | Yes | Yes | Closed |
| Page pad BO 4096 / size mode content | Common | content=1688 start_npu=1688 | Size-mode env trials **closed** (still load fail) |
| Kernel arg layout xrt3 vs count4 | Shim xrt3-like | Live used **xrt3** | Layout env trials **closed** |
| Rela overwrite vs add | XRT patch_ctrl_code | overwrite default; add trial closed | Closed |
| QoS non-zero / tiles | Required | 100/30, tiles=32 | Closed soft residual |
| DevHeap 64 MiB align | Required | OK on live path | Closed (map_host) |
| **CU/PDI from xclbin + CONFIG_CU** | **Yes on production KMQ** | **Never** | **Open construction hypothesis** |
| Full ELF in instr BO vs peeled ctrltext | Shim peels ctrltext | Peel ctrltext | Match shim test shape |
| Separate CTRL BO + meta size=6 patch | Yes in DD/meta | Yes (lab residual vs full XRT fidelity) | Partial; not proven root of 0x3000003 |
| Generation path aie2 msg vs aie4 | Host 1022:17f0 Strix | Same silicon; aie2-style status codes observed | Generation **catalog** closed; still worth noting aie4 enums differ on newer FW paths |

---

## What this is **not** (honesty)

- Not proof that missing CONFIG_CU alone causes `0x3000003` on START_NPU.
  Shim also has **partition-only** hwctx constructors without PDI; some tests
  may pass instruction-only. Need a **byte-level known-good** capture, not a
  guess.
- Not FW acceptance criteria (closed firmware).
- Not a new C unlock. Unlock still needs a **named offline delta** (code or
  evidence that changes the payload/path) + **new host-minted** stamp.
- Not inventing vendor post-patch dumps or private PDI decode.

---

## Highest-value next offline questions (named, not track letters)

1. **PDI / CONFIG_CU necessity**  
   Can we extract, offline, a CU PDI blob from a public xclbin / XRT path that
   successfully runs mladf (or any matmul) on this silicon class — and document
   size/header — without claiming Pass?

2. **Known-good instruction BO capture**  
   From a higher-level run that succeeds (Ryzen AI / ORT / DD execute), capture
   the **post-patch ctrltext** (or instr BO) and byte-diff vs our patched 1688.
   Prefer public tooling (shim dump, DD tests) over reopening L3 under consumed
   unlock.

3. **Triton-XDNA / mlir-aie artifacts**  
   Secondary: what does a successful open matmul hand to XRT (`aie.xclbin` /
   equivalent)? Map sections to PDI vs ctrltext roles. Only after (1)/(2) if
   still blocked.

4. **Do not**  
   - Empty gate-only stamp rolls  
   - Reopen execute-allow under `C_UNLOCK=b17b0119`  
   - Re-run closed env matrix (content / xrt3 / rela add) without new bytes  
   - Invent `ran_on_npu`

---

## Failing corpus fingerprint (for future byte-diff)

| Field | Value |
|-------|--------|
| Key | `mladfmatmulbias_prmptflat_mladf_2x4x4_flat_a16fw4acc16f_1_1152_256_128` |
| ELF | `testdata/xdna2_cmd/silicon_lab/lab_instr.elf` (32264 B) |
| ctrltext | 1688 B |
| ctrldata | 29664 B |
| rela | 12 |
| last_apply | `testdata/xdna2_cmd/last_apply.json` — `aiebu_elf=true`, `rela_count=12`, `layout=xrt3`, `ert_error=true`, `ran_on_npu=false` |
| Live unlock used | `b17b0119` **CONSUMED** |

When a known-good image exists, diff **patched ctrltext** first (not whole ELF),
then compare whether that run also had CONFIG_CU / PDI BOs.

---

## Unit surface (pure TDD — stamps `045739e1` + `695e5ad0` + `fec9c58b` + `f06d5b38` + `ac20bc3f` + `7a5ab864` + `a5787723` + `2c4b96bf` + `8e898123` + `9cdfe8d0` + `6033158a` + `d2e287ab` + `2b01425e` + `588c1c72`)

Offline helpers in `src/backends/amd_xdna2/silicon_contract.rs` (re-exported from
`amd_xdna2`). **Research only** — always `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`.
**Not** a Pass. **Not** a new live unlock.

| API | Role |
|-----|------|
| `ConstructionPathResearch` + `research_construction_path_lab_vs_public(elf)` | Locks lab = instruction/ctrltext only (`lab_has_cu_pdi_config=0`) vs public stack requires CU/PDI step → `construction_gap_cu_pdi=1`. Soft note: `construction_path_research …`. |
| `CuPdiNecessityResearch` + `research_cu_pdi_necessity_for_app_load` / `_c2_locked` (`f06d5b38`) | Residual matrix: lab ctrltext-only + gap + FW APP_LOAD name → open **hypothesis** that CU/PDI identity may still be required. **Hard lock:** `ctrltext_match_alone_closes_fw_residual=0` even on research sha match. Soft always `product_known_good=0 invent_pass=0`. |
| `AppLoadResidualAxisMatrix` + `research_app_load_residual_axis_matrix` / `_c2_locked` (`ac20bc3f`) | Unifies partition status-name surface vs CU/PDI construction gap vs lab ctrltext-only + journal `0x3000003` / `APP_LOAD_PDI_FAIL`. **Hard locks:** `ctrltext_match_alone_closes_fw_residual=0` + `partition_name_alone_proves_root_cause=0`. Soft always honesty trio + `product_known_good=0`. |
| `XclbinExtractResearchDryRun` + `research_xclbin_extract_dry_run` / `_ve2_fixture_refused` (`ac20bc3f`) | Public `xclbin2` magic + section **name** markers from docs/`xclbinutil` text only. VE2 / `NOT_AIE2_KNOWN_GOOD` → `eligible_as_aie2_external_known_good=0`. Never invents PDI layout (`do_not_invent_pdi_layout=1` / `public_pdi_image_decode=0`). Research eligible ≠ product known-good. |
| `CuPdiSectionIdentityResearch` + `classify_xclbin_section_kind` / `research_cu_pdi_section_identity_from_{markers,text}` (`7a5ab864`) | Classifies public section **names** into CU/PDI identity (`AIE_PARTITION`/`BITSTREAM`) vs topology vs metadata. **Hard lock:** `section_identity_alone_closes_fw_residual=0`. No body layout invent. Soft always honesty trio + `product_known_good=0`. |
| `CuPdiSectionOffsetMapResearch` + `parse_axlf_section_header` / `walk_axlf_section_headers` / `research_xclbin_cu_pdi_section_offset_map` (`a5787723`) | Public axlf **section header** field map: BITSTREAM/AIE_PARTITION **file offset+size** only (layout from `xrt/detail/xclbin.h`). **Hard lock:** `section_offset_map_alone_closes_fw_residual=0`. No body/PDI decode. VE2 fixture may parse offsets and still `product_known_good=0` / NOT_AIE2. Soft always honesty trio. |
| `CuPdiSectionBodyBoundsResearch` + `check_axlf_section_body_bounds` / `axlf_section_body_window_in_bounds` / `research_xclbin_cu_pdi_section_body_bounds` (`2c4b96bf`) | Body **window** vs `file_len` from offset map — overflow-safe OOB refuse. **Hard lock:** `body_bounds_alone_closes_fw_residual=0`. Never slices/decodes section bodies. Header-prefix buffers honestly report OOB. Soft always honesty trio + `product_known_good=0`. |
| `parse_failed_cmd_status_token` / `parse_chain_status_token` / `first_hex_status_token` (`2c4b96bf`) | Pure single-line journal token extractors; multi-line pair uses them. Unit-locked C2 `0x3000003`/`0x4000003`; last_apply userspace detail → absent (no invent). Never invent residual closed / Pass. |
| `ExternalKnownGoodBagDropCheck` + `check_external_known_good_bag_drop` (`8e898123`) | Fail-closed bag-drop completeness: ctrltext bytes + research-ready PROVENANCE; refuse local post-patch basenames + VE2/NOT_AIE2. **Hard lock:** `bag_drop_alone_closes_fw_residual=0`. Incomplete when bytes still absent. Soft always honesty trio + `product_known_good=0`. |
| `AppLoadE2eFixtureResearch` + `research_app_load_e2e_fixture_path` / `_c2_locked_absent_bag` (`8e898123`) | Soft end-to-end fixture orchestration: residual class + journal tokens + body-bounds surface + bag drop against last_apply/C2. **Hard locks:** `residual_not_closed=1` + `body_bounds_alone_closes_fw_residual=0` + `ctrltext_match_alone_closes_fw_residual=0` + honesty trio. Never invents journal codes from userspace detail alone. |
| `AppLoadOfflineProgressScorecard` + `research_app_load_offline_progress_scorecard` / `_c2_locked_absent_bag` + `ExternalCompareStatus` (`9cdfe8d0`) | Composes residual class + bag-drop + e2e residual_not_closed + journal token presence + external_compare_status (`absent\|loaded\|match\|mismatch\|refused`). **Hard locks:** `residual_not_closed=1` + `scorecard_alone_closes_fw_residual=0` + `ctrltext_match_alone_closes_fw_residual=0` + honesty trio. Never invents journal from detail alone. Match ≠ Pass. |
| `format_external_known_good_operator_extract_checklist` (`588c1c72` stamp; prior `2b01425e`/`d2e287ab`/`6033158a`/`9cdfe8d0` steps) | Pure operator extract-to-bag steps (public stack → bag → PROVENANCE → bag-drop → soft compare → dry-run → lab auto-feed → composition scorecard → extract pipeline). Unit-locked refuse tokens: local post_patch, VE2/NOT_AIE2, invent Pass, second unlock; `blocked_on_external_bytes_when_bag_absent=1` + `lab_export_auto_feed=1` + `dry_run_alone_closes_fw_residual=0` + `feed_alone_closes_fw_residual=0` + `scorecard_alone_closes_fw_residual=0` + `extract_alone_closes_fw_residual=0` + `pipeline_alone_closes_fw_residual=0` + step9/step10/step11. Research-only; no invent bytes. |
| `ExternalCtrltextCompareDryRun` + `research_external_ctrltext_compare_dry_run` / `_ex` / `_c2_locked_absent_bag` (`6033158a`) | Compare dry-run when bag may be absent: explicit **`blocked_on_external_bytes=1`** (not silent absent); `lab_export_ready` when fingerprint has unpatched sha; refuse local post_patch basename + VE2/NOT_AIE2 (`platform_not_aie2=1`); optional xclbin extract refuse companion. When both present, runs compare surfaces — **match ≠ Pass**. **Hard locks:** `residual_not_closed=1` + `dry_run_alone_closes_fw_residual=0` + `ctrltext_match_alone_closes_fw_residual=0` + honesty trio hard-coded. |
| `LabExportFingerprintFeed` + `research_lab_export_fingerprint_feed` / `_from_elf` / `_from_last_apply_*` + `research_external_ctrltext_compare_dry_run_from_lab_feed` / `_ex` (`d2e287ab`) | Auto-feed lab export fingerprint into compare dry-run from staged ELF / export path (never invent external bytes or unpatched sha from last_apply signals alone). Lab-ready + bag absent → `lab_export_ready=1` + `blocked_on_external_bytes=1`. Soft: `lab_export_fingerprint_feed=`. **Hard locks:** `residual_not_closed=1` + `feed_alone_closes_fw_residual=0` + `dry_run_alone_closes_fw_residual=0` + `scorecard_alone_closes_fw_residual=0` + honesty trio hard-coded (`lab_export_auto_feed=1` always). Match ≠ Pass. |
| `AppLoadBagFeedDryRunScorecard` + `research_app_load_bag_feed_dry_run_scorecard` / `_ex` / `_c2_locked_absent_bag` (`2b01425e`) | Composition soft row: bag-drop + lab feed + dry-run-from-feed. After auto-feed, bag absent → explicit **`blocked_on_external_bytes=1`**. Distinct detail tokens (lab-ready blocked / lab-not-ready / refuse post_patch / VE2 / match still open / external load refused). Refuse merge non-clobber from bag+feed. Soft: `app_load_bag_feed_dry_run_scorecard=`. **Hard locks:** `residual_not_closed=1` + `bag_drop_alone_closes_fw_residual=0` + `feed_alone_closes_fw_residual=0` + `dry_run_alone_closes_fw_residual=0` + `scorecard_alone_closes_fw_residual=0` + `ctrltext_match_alone_closes_fw_residual=0` + honesty trio hard-coded (`lab_export_auto_feed=1` always). Match ≠ Pass. |
| `AppLoadExtractBagScorecardPipeline` + `research_app_load_extract_bag_scorecard_pipeline` / `_ex` / `_from_inputs` / `_c2_locked_absent_bag` (`588c1c72`) | End-to-end research row: extract dry-run + bag-drop + bag-feed composition scorecard. Public section markers **without** ctrltext → still **`blocked_on_external_bytes=1`**. Distinct detail tokens (extract eligible / markers / magic blocked vs lab-ready / refuse / match still open). Refuse merge non-clobber from extract+bag+feed. Soft: `app_load_extract_bag_scorecard_pipeline=`. Optional soft env `XDNA2_LAB_XCLBIN_SECTION_TEXT` (section names only; never product known-good). **Hard locks:** `residual_not_closed=1` + `extract_alone_closes_fw_residual=0` + `pipeline_alone_closes_fw_residual=0` + all alone_closes + honesty trio hard-coded. Match ≠ Pass. |
| `AppLoadPartitionIdFieldMap` + `research_app_load_partition_id_field_map` / `_c2_locked` (`7a5ab864`) | Maps journal fields for `0x3000003` residual: `failed_cmd_status` / `chain_status` present; **`partition_id` always `absent_public`**. **Hard locks:** `field_map_alone_closes_fw_residual=0` + `root_cause_closed=0` + `do_not_invent_partition_id=1`. |
| `PatchedCtrltextFingerprint` + `fingerprint_patched_ctrltext_after_rela` / `_c2_locked` | Structural corpus lock for silicon_lab (1688 / rela×12 / classic walk / unpatched sha). Patched sha **depends on paddrs** — prefer structural + unpatched sha for identity. |
| `CtrltextFingerprintExport` + `export_ctrltext_fingerprint_research(elf)` (`695e5ad0`) | Exportable unpatched identity bag (`format_export_text` portable lines). Always `known_good_status=absent_awaiting_capture` from constructors — never invents known-good. |
| `ExternalCtrltextCompareResearch` + `compare_external_ctrltext_to_lab_export` (`695e5ad0`) | Soft sha/len/structural diff vs lab export. Missing external → `known_good_absent=1`. Match **≠** Pass (failing corpus self-match still invent_pass=0). Soft note always `known_good_present=0` / `product_known_good=0` even when research external present. Positive `structural_match` requires classic walk OK on **both** sides (not dual garbage). |
| External bag confinement + load (`fec9c58b`) | `external_known_good_path_is_allowed` / `_load_path_is_allowed` / `load_external_ctrltext_for_compare` / `load_external_ctrltext_from_env`. Roots: clamped `temp_dir` ∪ `testdata/xdna2_cmd/external_known_good/`. Ext: `.bin` / `.ctrltext`. Refuse local post-patch oracle basenames. Size-capped via `read_lab_file_capped`. |
| PROVENANCE field validation (`f06d5b38`) | `validate_external_known_good_provenance_text` / `external_known_good_lab_bag_provenance_allows_load` / `external_known_good_provenance_research_ready`. Lab bag load requires research-ready PROVENANCE: research-only honesty (prose or exact `not_c_ready=1`; **not** substring that matches `not_c_ready=0`) + source/stack (whole-token `xrt`) + refuse invent/honesty-inverse tokens (ascii-lower exact, incl. `product_known_good=1` / `not_c_ready=0`). Bag PROVENANCE only under bag root (no grandparent). Empty / missing markers fail-closed. Research-ready **≠** product known-good. |
| Soft classify (optional) | `XDNA2_LAB_PRINT_C2_CMDBUF_RESEARCH=1` → `construction_path_research=` + `ctrltext_fingerprint_export=` + `lab_export_fingerprint_feed=` + `external_ctrltext_compare=` + `external_ctrltext_compare_dry_run=` (auto-feed via feed) + `app_load_bag_feed_dry_run_scorecard=` (composition; bag absent after feed → `blocked_on_external_bytes=1`) + `app_load_extract_bag_scorecard_pipeline=` (extract→bag→scorecard; markers without bytes still blocked) + `cu_pdi_necessity_research=` + `app_load_residual_axis_matrix=` + `xclbin_extract_research_dry_run=` + `cu_pdi_section_identity_research=` + `cu_pdi_section_offset_map_research=` + `cu_pdi_section_body_bounds_research=` + `app_load_partition_id_field_map=` + `external_known_good_provenance_check=` + `external_known_good_bag_drop_check=` + `app_load_e2e_fixture_research=` + `app_load_offline_progress_scorecard=` + `external_known_good_operator_extract_checklist=` in `examples/classify_last_apply.rs`. Optional `XDNA2_LAB_EXTERNAL_CTRLTEXT=` (confined) feeds compare/dry-run when set; unset → `blocked_on_external_bytes=1`. Optional `XDNA2_LAB_XCLBIN_SECTION_TEXT=` research section-name paste for pipeline extract input. Lab export auto-feeds from staged ELF when present. |

**Offline honesty:** Local pure-direct `gt_capture/instr_post_patch.bin` is **not**
a working public-stack known-good capture. VE2 `validate.xclbin` is **NOT_AIE2**
(unit-locked refuse; section **headers** may still parse for offset research;
body windows vs header prefix are OOB — honest).
No external known-good ctrltext/PDI was present on disk for this stamp;
export/compare + confined load + provenance + axis matrix + extract dry-run +
section identity + section-offset map + body bounds + journal tokens +
partition-id field map + bag-drop + e2e + progress scorecard + operator extract
checklist + compare dry-run (`blocked_on_external_bytes`) + lab export
fingerprint auto-feed are ready when real AIE2 bytes arrive.

Tests (filter): `cargo test --features amd-xdna2 --lib construction_path`,
`cu_pdi_necessity`, `cu_pdi_section`, `app_load_residual_axis`,
`app_load_partition_id`, `xclbin_`, `journal_failed_cmd`, `fingerprint_`,
`external_known_good`, `app_load_e2e_fixture`, `bag_drop`,
`app_load_offline_progress`, `operator_extract_checklist`,
`external_ctrltext_compare_dry_run`, `lab_export_fingerprint_feed`.
Map + TDD **≠** Pass / **≠** second C unlock.

---

## Capture procedure — external known-good (when a public stack works)

**Goal:** feed real post-patch ctrltext (and note CU/PDI necessity) into
`compare_external_ctrltext_to_lab_export` against the lab fingerprint export.

### Where to put bytes

```text
testdata/xdna2_cmd/external_known_good/   # dedicated bag (gitignored bins)
  PROVENANCE.md                          # tracked template — fill capture table
  working_stack_ctrltext.bin             # example name (.bin or .ctrltext)
```

- Bins stay **gitignored** (`testdata/**/*.bin`). Keep PROVENANCE.md filled.
- Temp files under `TMPDIR` (clamped) are allowed for unit tests only.
- **Refuse basenames** (never known-good — matches `REFUSED_KNOWN_GOOD_BASENAMES`):
  `instr_post_patch.bin`, `instr_vendor_post_patch.bin`, `ctrl_post_patch.bin`,
  `instr_raw_txn.bin`, `instr_vendor_txn_bin.bin`. Also refuse VE2 NOT_AIE2
  xclbin / `ve2_latency/` as known-good (outside bag roots).

### How to capture (operator outline)

1. Use a **working** public path (XRT KMQ / DynamicDispatch / Ryzen AI EP) that
   reaches ERT Completed for a real matmul if possible — same mladf key preferred.
2. Dump the **ctrltext** that lands in the instruction BO after host peel/patch
   (or dump aiebu ELF and peel `.ctrltext`; document which).
3. Note whether that run also performed **CONFIG_CU / PDI** (construction gap
   token `construction_gap_cu_pdi=1` already locks that lab lacks this step).
   Optional future PDI bag uses the **same confinement roots** — no private PDI
   layout decoder.
4. Place bytes under `external_known_good/` + update PROVENANCE capture table
   (must stay research-ready for load — see PROVENANCE field checks below).

### Public-stack extract recipe (exact offline steps — fail-closed provenance)

When a working stack is available **on this machine**, extract research bytes
**without** inventing Pass / reopening L3 under consumed unlock:

#### A) Ctrltext (preferred first compare target)

```text
# Prefer: post-peel .ctrltext that START_NPU uses (unpatched peel for fair sha
# vs lab export_ctrltext_fingerprint_research, OR document post-rela paddrs).
#
# From aiebu ELF produced by working stack / shim test elf_io path:
  readelf -S working.elf | grep -E 'ctrltext|rela'
  # peel .ctrltext with existing lab tooling, or:
  # llvm-objcopy -O binary --only-section=.ctrltext working.elf bag.ctrltext
  cp bag.ctrltext testdata/xdna2_cmd/external_known_good/working_stack_ctrltext.bin
```

Also acceptable: raw instr BO dump **after** host peel (same length class as
lab 1688 when same mladf key). **Refuse** renaming lab
`gt_capture/instr_post_patch.bin` into this bag (loader refuse list).

#### B) CU / PDI note (optional; no private decoder)

```text
# Production KMQ path (open source map only — no layout invent):
#   xclbin → shim extracts PDI → CREATE_BO + CONFIG_CU → driver aie2_config_cu
# Capture what you can without private reverse-engineering:
#   - whether CONFIG_CU ran (yes/no)
#   - PDI BO size if logged / dumped as opaque blob
#   - xclbin path / key used
# Place optional notes as notes_cu_pdi.txt; optional opaque .bin only under the
# same bag root with PROVENANCE filled. Do NOT invent a PDI field map.
```

#### C) PROVENANCE requirements (unit-locked, fail-closed)

Before load treats lab-bag bytes as research-present, `PROVENANCE.md` must pass
`validate_external_known_good_provenance_text`:

| Requirement | Fail if missing |
|-------------|-----------------|
| Non-empty body | empty / whitespace-only |
| Research-only honesty | no prose `research only` / `not product pass` / `not c-ready` **and** no exact token `not_c_ready=1` (substring `not_c_ready=0` is **not** a research marker) |
| Source/stack marker | no `source stack` / `public stack` / `working public` / `dynamicdispatch` / `ryzen ai` / whole-token `xrt` |
| No product invent / honesty-inverse tokens | ascii-lower exact: `invent_pass=1` / `invent_pass=true` / `ran_on_npu=1` / `ran_on_npu=true` / `c_ready=1` / `product_known_good=1` / `not_c_ready=0` |

Tracked template already passes. Fill the capture record table when bytes land.
Research-ready **≠** product known-good **≠** Pass **≠** second C unlock.

### How to compare (offline soft)

```bash
export XDNA2_LAB_PRINT_C2_CMDBUF_RESEARCH=1
export XDNA2_LAB_EXTERNAL_CTRLTEXT="$PWD/testdata/xdna2_cmd/external_known_good/working_stack_ctrltext.bin"
cargo run --features amd-xdna2 --example classify_last_apply --quiet -- \
  testdata/xdna2_cmd/last_apply.json
# external_ctrltext_compare= … product_known_good=0 invent_pass=0 not_c_ready=1
unset XDNA2_LAB_EXTERNAL_CTRLTEXT
```

Or pure unit: `load_external_ctrltext_for_compare` →
`compare_external_ctrltext_to_lab_export`.

### Honesty

- Capture **≠** Pass. Loading known-good for research **≠** C-ready **≠** second unlock.
- Soft note always `product_known_good=0` even when research external present.
- `C_UNLOCK=b17b0119` **CONSUMED** — do not reopen L3 under it.

---

## See also

- Residual SSoT: [`../RESIDUAL.md`](../RESIDUAL.md)  
- Field contract: [`XDNA2_XRT_CONTRACT_MLADF.md`](./XDNA2_XRT_CONTRACT_MLADF.md)  
- HITL recipe (CONSUMED): [`HITL_C_APP_LOAD_RECIPE.md`](./HITL_C_APP_LOAD_RECIPE.md)  
- Stage script: `tools/stage_aiebu_lab_instr.sh`  
- Agent policy: `AGENTS.md` standing §6–9  
