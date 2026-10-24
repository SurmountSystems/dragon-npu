# PROVENANCE — external known-good bag (operator capture)

**Research only. Not product Pass. Not C-ready. Not `ran_on_npu`.**

Place working **public-stack** post-patch ctrltext (and optionally notes about
CU/PDI) here when you have a stack that completes EXEC (ERT Completed + useful
OFM). Bins under this directory are gitignored; this PROVENANCE note is tracked.

**Operator shortcuts — one primary entrypoint first:**

```bash
# PRIMARY: status / offline fingerprint / ingest (Pass still No; invent_pass=0)
bash tools/pass_unblock_once.sh
bash tools/pass_unblock_once.sh --agent-offline
bash tools/pass_unblock_once.sh --ingest /path/to/ctrltext.bin working_stack_ctrltext.bin

# W5 host capture automation (offline only; invent_pass=0)
bash tools/host_capture_scan.sh                  # W5-2 read-only host scan
bash tools/host_npu_stack_inventory.sh           # broader stack probe
bash tools/collect_load_accepted_candidate.sh --dry-run /path/to/candidate.bin
# --ingest only when not multi-txn / not research-duplicate sha (still research material)

# Secondary aliases (same kit surface)
bash tools/operator_app_load_kit.sh pass-unblock
bash tools/operator_app_load_kit.sh              # overview A+B
bash tools/operator_external_capture.sh          # checklist + status
bash tools/operator_external_capture.sh --install /path/to/ctrltext.bin
# edit this file's capture table, then:
bash tools/operator_external_capture.sh --compare
# kit: bash tools/operator_app_load_kit.sh b-compare
```

## Load-accepted vs research-only (W5-3)

**Do not re-tag** existing public mlir-aie npu2 AOT rows (300 / 660 / 2592 /
4512 B peels and siblings) as classic **1688** load-accepted. Those rows stay
**RESEARCH ONLY** (size-class `public-npu2-small` / `other`). A completed
IRON this-run collect may be tagged `job=iron_npu2_vector_add` (a different
identity). That is **not** mladf classic 1688. Research-only ingest is
allowed; **load-accepted** for the 1688 job still needs a completing stack
on **this** firmware.

| Claim | When it is honest | When it is forbidden |
|-------|-------------------|----------------------|
| **research_only** | Offline AOT / host peel / classifier / geometry; no live ERT Completed + OFM allclose on this FW | Setting `load_accepted=1`, `ran_on_npu=1`, `invent_pass=1`, product Pass |
| **load_accepted** | Operator documents live **ERT Completed** + **OFM allclose** via product `decide_ran_on_npu` on this host firmware for these exact bytes | Multi-txn **32208** B wrong body; sha16 matching known research peels (`aaeb5b68…`, `de867124…`, `7ee4b267…`, `18cb653c…`); invent from attach-only / wait_cmd_einval / scan alone |

Hard rules (always):

- `invent_pass=0`
- Capture / ingest / soft compare **≠** product Pass **≠** `ran_on_npu`
- Multi-txn size **32208** → refuse load-accepted (and refuse residual_matched)
- Research-duplicate sha16 → refuse load-accepted claim (may still sit as research)
- Helpers: `tools/collect_load_accepted_candidate.sh`, `tools/capture_from_host_candidate.sh`

### Load-accepted capture table (fill only when true)

Copy this block when you have a **completing** working-stack capture (not a
re-label of the mlir-aie research rows above). Leave blank until then.

| Field | Value (example / allowed) |
|-------|---------------------------|
| Date | YYYY-MM-DD |
| Basename | e.g. `working_stack_ctrltext.bin` (not oracle basenames) |
| Bytes / sha16 / sha256 | measured |
| **completed_on_this_fw** | **yes** only if live ERT Completed + OFM allclose on this FW; else **no** |
| **stack** | `FastFlowLM` / `IRON example` / `xrt-smi` / `DynamicDispatch` / `Ryzen AI EP` / `other` (name it) |
| **firmware version** | e.g. amdxdna / amdnpu version from sysfs or package (record exact string) |
| **how extracted** | e.g. post-rela ctrltext peel / aiebu `.ctrltext` / XRT dump path (concrete command or tool) |
| invent_pass | **0** (never 1) |
| claim_class | `research_only` **or** `load_accepted` — load_accepted only when completed_on_this_fw=yes |
| residual_matched_claim | **0** unless evidence + operator PROVENANCE review (helpers keep 0) |
| Notes | No Pass token. Unlock still separate. |

**Empty table is correct** until a true completing capture lands. Do not fill
`completed_on_this_fw=yes` for research-only mlir-aie AOT.

### Path B agent attempt (2026-08-11) — IRON NPU verify PASS (≠ product load_accepted)

Agent ran Path B under operator “you do the capture” ask. **Not** a dragon-npu
live apply. **Not** product Pass. **Not** residual match to lab classic 1688.
Helpers still refuse `load_accepted_claim` for research-duplicate sha16
`aaeb5b68cc298a45` (300 B public peel).

| Field | Value |
|-------|-------|
| Date | 2026-08-11 |
| Command | `python3 vector_scalar_add.py --dev npu2 --from-xclbin …/aot_vector_scalar_add/vector_scalar_add.xclbin --from-insts …/vector_scalar_add.insts.bin` with system `pyxrt` on `PYTHONPATH` |
| Result | **PASS!** exit 0 (IRON upstream verify) |
| Log | `.agents/reports/path-b-vector-scalar-add-npu2-pass-20260811.log` |
| Device | RyzenAI-npu6 BDF `0000:c6:00.1` |
| NPU Firmware | **1.1.2.64** (`xrt-smi examine`) |
| XRT | 2.21.75 |
| amdxdna | 7.1.5-zen1-2-zen |
| Ctrltext | 300 B sha16 `aaeb5b68cc298a45` (byte-identical to bag research peel + `working_stack_ctrltext.bin`) |
| **iron_completed_on_this_fw** | **yes** (IRON verify PASS on this host/FW) |
| **product load_accepted_claim** | **0** (research-duplicate sha; no product `decide_ran_on_npu`) |
| **residual_matched_claim** | **0** (still not lab classic 1688 / not spent flexml identity) |
| invent_pass | **0** |
| Pass / ran_on_npu | **No** / not claimed |
| Unlock | **none** (no mint from this pin) |
| Prerequisite note | Capture venv alone lacked `pyxrt`; host `/usr/lib/python3.14/site-packages/pyxrt…so` required on `PYTHONPATH` |

Filled product load-accepted table above remains empty: IRON PASS ≠ product ERT
Completed + OFM allclose via dragon-npu decide. Same 300 B peel stays research
for residual-match helpers until a different residual-matched identity lands.

### Exact public-stack → bag steps (playbook)

Full construction context: `docs/CONSTRUCTION_MAP_APP_LOAD.md` § Public-stack
ctrltext extract playbook.

**Generate from open stack (mlir-aie / IRON / npu2):** full concrete steps in
[`docs/CAPTURE_MATERIAL_MLIR_AIE.md`](../../../docs/CAPTURE_MATERIAL_MLIR_AIE.md).
**Load-accepted (completed-on-FW) operator page (Paths A/B/C):**
[`docs/LOAD_ACCEPTED_CAPTURE.md`](../../../docs/LOAD_ACCEPTED_CAPTURE.md).
Public AOT peels already here (300 / 660 / 2592 / 4512) are **RESEARCH ONLY** —
do **not** re-tag as load-accepted without a completed NPU run on this firmware.
**Rule:** a source clone is **not** capture material. Residual tools need
**files here** (or `--ingest PATH` with a real path).

Short form (already-working XRT/DD stack):

1. Run a **working** public stack that completes on NPU (XRT / DD / Ryzen AI EP).
2. Capture **post-patch ctrltext** (not a refused local oracle basename).
3. `bash tools/pass_unblock_once.sh --ingest /path/to/ctrltext.bin working_stack_ctrltext.bin`
   (**PATH required**; bare `--ingest` refuses.)
4. Fill the **capture table** below (source stack, key, peel notes).
5. Soft compare: `bash tools/operator_external_capture.sh --compare` (or kit
   `b-compare`).

**Refuse as bag:** `gt_capture/` / lab oracles (`instr_post_patch.bin`,
`instr_vendor_post_patch.bin`, `ctrl_post_patch.bin`, `instr_raw_txn.bin`, …),
VE2 `validate.xclbin`. Match ≠ Pass. invent_pass=0. No live unlock needed for
ingest/compare.

## Why this directory exists

Lab residual is FW `APP_LOAD_PDI_FAIL` (`0x3000003`) after userspace aiebu+rela
OK. Offline export/compare (`export_ctrltext_fingerprint_research` /
`compare_external_ctrltext_to_lab_export`) need a **real external** identity
from a working path — not our pure-direct oracle.

**Never** treat these as known-good by default (loader refuses basenames):

| Path / basename | Why refused |
|-----------------|-------------|
| `instr_post_patch.bin` (e.g. `gt_capture/…`) | Pure-direct patch oracle (our paddrs) |
| `instr_vendor_post_patch.bin` | Vendor capture when present still local lab |
| `ctrl_post_patch.bin` | Lab ctrl patch oracle |
| `instr_raw_txn.bin` | Unpatched raw txn oracle (not public-stack known-good) |
| `instr_vendor_txn_bin.bin` | Vendor txn diagnostic dump (not known-good) |
| VE2 `validate.xclbin` / `ve2_latency/` | **NOT_AIE2** research stub |

## What to put here

| File (example) | Content |
|----------------|---------|
| `working_stack_ctrltext.bin` or `*.ctrltext` | Post-patch **ctrltext** bytes from a working public stack (XRT/DD after rela / CONFIG_CU path). Prefer unpatched peel for fair sha vs lab export, **or** document which. |
| `notes_cu_pdi.txt` (optional) | Whether the run also did CONFIG_CU / PDI BOs; sizes; **no** private PDI decode required. PDI bag confinement (future) mirrors this dir policy — do not invent a decoder. |

## Fail-closed bag-drop checklist (stamp `8e898123`)

Pure helper: `check_external_known_good_bag_drop` / soft line
`external_known_good_bag_drop_check=`. Incomplete drops refuse research-complete.

| Gate | Fail-closed if |
|------|----------------|
| Ctrltext bytes | No non-empty `.bin` / `.ctrltext` bag-resident **or** env path |
| PROVENANCE research-ready | Missing research-only + source/stack markers, or invent tokens |
| Basename refuse | `instr_post_patch.bin` / vendor post_patch / raw txn oracles |
| Platform refuse | VE2 / `NOT_AIE2_KNOWN_GOOD` claim |

`bag_drop_complete=1` **≠** Pass **≠** residual closed **≠** product known-good.
Soft always `bag_drop_alone_closes_fw_residual=0 invent_pass=0 product_known_good=0`.

E2e soft orchestration (body-bounds + journal tokens + bag drop + residual class):
`app_load_e2e_fixture_research=` under `XDNA2_LAB_PRINT_C2_CMDBUF_RESEARCH=1` —
always `residual_not_closed=1`.

Offline progress scorecard (stamp `9cdfe8d0`): composes class + bag-drop + e2e
+ external_compare_status (`absent|loaded|match|mismatch|refused`) —
`app_load_offline_progress_scorecard=` always
`scorecard_alone_closes_fw_residual=0 residual_not_closed=1 invent_pass=0`.

External ctrltext compare dry-run (stamp `6033158a`): when bag absent soft line
`external_ctrltext_compare_dry_run=` reports explicit
`blocked_on_external_bytes=1` (not silent absent). Refuse local post_patch /
VE2 (`platform_not_aie2=1`). Match ≠ Pass. Always
`dry_run_alone_closes_fw_residual=0 residual_not_closed=1 invent_pass=0`.

Lab export fingerprint auto-feed (stamp `d2e287ab`): soft
`lab_export_fingerprint_feed=` derives lab fingerprint from staged ELF / export
path when available (never invents external bytes or unpatched sha from
last_apply signals alone). Soft dry-run auto-feeds that export so lab-ready +
bag absent → `lab_export_ready=1` + `blocked_on_external_bytes=1`. Always
`feed_alone_closes_fw_residual=0 residual_not_closed=1 invent_pass=0`.

Bag + feed + dry-run composition scorecard (stamp `2b01425e`): soft
`app_load_bag_feed_dry_run_scorecard=` composes bag-drop + lab feed + dry-run
after auto-feed. When bag absent after feed → explicit
`blocked_on_external_bytes=1` (not silent). Match ≠ Pass. Always
`scorecard_alone_closes_fw_residual=0 residual_not_closed=1 invent_pass=0`.

Extract → bag-drop → composition scorecard pipeline (stamp `588c1c72`): soft
`app_load_extract_bag_scorecard_pipeline=` folds public xclbin section markers
(extract dry-run) into bag-drop + bag-feed composition scorecard. Markers present
**without** ctrltext bytes → still explicit `blocked_on_external_bytes=1`.
Optional research paste env `XDNA2_LAB_XCLBIN_SECTION_TEXT` (section **names**
only; never product known-good / PDI decode). Match ≠ Pass. Always
`extract_alone_closes_fw_residual=0 pipeline_alone_closes_fw_residual=0
residual_not_closed=1 invent_pass=0`.

## Operator extract-to-bag checklist (stamp `588c1c72`)

Pure helper: `format_external_known_good_operator_extract_checklist` / soft line
`external_known_good_operator_extract_checklist=`. **No invent bytes.** Match ≠ Pass.

| Step | Action | Refuse if |
|------|--------|-----------|
| 1 | Run working public stack (XRT / DynamicDispatch) same mladf key when possible | — |
| 2 | Capture post-patch **ctrltext** (or aiebu ELF + document peel) | local pure-direct oracle only |
| 3 | Copy to this directory with clear basename (`.bin` / `.ctrltext`) | `instr_post_patch.bin`, `instr_vendor_post_patch.bin`, `ctrl_post_patch.bin`, `instr_raw_txn.bin` |
| 4 | Fill capture table below; keep research-only + source/stack markers | invent tokens / `product_known_good=1` |
| 5 | Confirm bag-drop complete (bytes + this PROVENANCE) via soft classify or `cargo test … bag_drop` | VE2 / `NOT_AIE2` as AIE2 known-good |
| 6 | Set confined `XDNA2_LAB_EXTERNAL_CTRLTEXT` + PRINT flag soft compare | path outside bag/temp; symlink final component |
| 7 | Read `external_ctrltext_compare=` + scorecard — match **≠** residual closed | claim Pass / second `C_UNLOCK` |
| 8 | Soft `external_ctrltext_compare_dry_run=` — bag absent → `blocked_on_external_bytes=1`; dry-run alone ≠ residual closed | invent Pass from dry-run |
| 9 | Soft `lab_export_fingerprint_feed=` auto-feeds lab export into dry-run when staged ELF present; feed alone ≠ residual closed | invent Pass from feed |
| 10 | Soft `app_load_bag_feed_dry_run_scorecard=` composition row — bag absent after auto-feed → `blocked_on_external_bytes=1`; scorecard alone ≠ residual closed | invent Pass from scorecard |
| 11 | Soft `app_load_extract_bag_scorecard_pipeline=` — public section markers without bytes still `blocked_on_external_bytes=1`; extract/pipeline alone ≠ residual closed | invent Pass from markers / pipeline |

## Capture outline (operator, offline when stack works)

1. Run a working public path (Ryzen AI / XRT / DynamicDispatch) for the same
   mladf key if possible (`…_1_1152_256_128`).
2. Capture instruction image **after** host peel/patch (ctrltext content that
   goes into the instr BO / START_NPU buffer), **or** aiebu ELF + document
   peel step.
3. Copy bytes to this directory with a clear name; fill the table below.
4. Confirm bag-drop complete (bytes + this PROVENANCE filled) via soft classify
   or `cargo test --features amd-xdna2 --lib bag_drop`.
5. Compare offline (no L3 / no ALLOW / no invent Pass):

```bash
# Soft classify research lines (PRINT flag) + confined env path:
export XDNA2_LAB_PRINT_C2_CMDBUF_RESEARCH=1
export XDNA2_LAB_EXTERNAL_CTRLTEXT="$PWD/testdata/xdna2_cmd/external_known_good/working_stack_ctrltext.bin"
cargo run --features amd-xdna2 --example classify_last_apply --quiet -- \
  testdata/xdna2_cmd/last_apply.json
# Look for external_ctrltext_compare= … product_known_good=0 invent_pass=0
# and app_load_offline_progress_scorecard= … residual_not_closed=1
unset XDNA2_LAB_EXTERNAL_CTRLTEXT
```

Unit / pure path:

```bash
cargo test --features amd-xdna2 --lib external_known_good
cargo test --features amd-xdna2 --lib app_load_offline_progress
cargo test --features amd-xdna2 --lib operator_extract_checklist
```

## Honesty

- Capture **≠** Pass.
- Loading known-good for research **≠** C-ready **≠** second `C_UNLOCK`.
- Soft compare always `invent_pass=0 not_c_ready=1 not_ran_on_npu=1 product_known_good=0`.
- Match to lab unpatched peel may only show we already match ourselves (failing
  corpus) — still residual until FW accepts a live job.
- Ctrltext sha match alone **never** closes FW residual (CU/PDI construction gap
  may still apply — see `cu_pdi_necessity_research`).
- Progress scorecard alone **never** closes residual
  (`scorecard_alone_closes_fw_residual=0`).
- `C_UNLOCK=b17b0119` remains **CONSUMED** until a new host-minted unlock after
  a real offline payload/path delta.

## PROVENANCE field checks (fail-closed before research-present)

Unit-locked by `validate_external_known_good_provenance_text` (stamp `f06d5b38`).
Lab-bag load via `external_known_good_load_path_is_allowed` requires **all**:

| Check | This template |
|-------|----------------|
| Non-empty | yes |
| Research-only marker (prose `Research only` / `Not product Pass` / `Not C-ready`, or exact `not_c_ready=1`; **not** `not_c_ready=0`) | yes (header + honesty) |
| Source/stack marker (`Source stack` / `public stack` / whole-token XRT / …) | yes (sections) |
| No invent / honesty-inverse tokens (`invent_pass=1|true`, `ran_on_npu=1|true`, `c_ready=1`, `product_known_good=1`, `not_c_ready=0`; case-insensitive) | yes |

Research-ready **≠** product known-good. Empty capture table is OK until bytes land.

## Capture record (fill when bytes land)

| Field | Value |
|-------|--------|
| Date | 2026-08-10 (AOT land); **2026-08-11 Path B IRON live verify** |
| Source stack | public stack: Xilinx/mlir-aie IRON `@iron.jit` on **npu2**. AOT peels land 2026-08-10. **2026-08-11:** agent Path B run completed on this host firmware (IRON verify `PASS!`, exit 0) for the same sha16 `aaeb5b68…` peels. Research-only for product bag policy (research-dup sha refuses load-accepted re-tag). Not product Pass. Not C-ready. invent_pass=0. not_c_ready=1. product `ran_on_npu` still **false** (no dragon-npu decide). product_known_good=0. |
| Key / shape | `vector_scalar_add` problem_size=1024 (int32), aie_tile_width=32, single-core transform |
| File basename | see **mlir-aie npu2 material** table below |
| Bytes / sha256 | see table |
| Unpatched or post-rela? | Host-generated instruction stream / aiebu ELF `.ctrltext` peel (0x12c = 300 B). Not lab pure-direct paddrs. Not post-host-rela on this firmware. |
| CONFIG_CU / PDI present on that run? | **Built offline:** yes — AOT emitted `.pdi` (2528 B) + `.xclbin` (8983 B). IRON XRT path loads xclbin+insts (not dragon-npu CONFIG_CU sandwich). |
| Notes | Path B IRON completed-run evidence 2026-08-11 (see section below). **load_accepted_claim=0** (research-duplicate sha16; helper refuse re-tag). invent_pass=0. No C_UNLOCK. Match ≠ Pass. |

### Path B completed IRON run (2026-08-11) — vector_scalar_add

**Agent attempt** (operator said agent must try Path B). Report:
`.agents/reports/wave5-path-b-agent-attempt.md`.

| Field | Value |
|-------|--------|
| Date | 2026-08-11 |
| Host | horizon; kernel 7.1.5-zen1-2-zen; device **RyzenAI-npu6**; NPU FW **1.1.2.64**; XRT 2.21.75 |
| Stack | mlir-aie IRON + system **pyxrt** (`/usr/lib/python3.14/site-packages`) + capture venv |
| Prerequisite | Capture venv alone has **no** pyxrt; must put system site-packages on `PYTHONPATH` |
| RUN2 (AOT BYO) | `python3 vector_scalar_add.py --dev npu2 --from-xclbin …/aot_vector_scalar_add/vector_scalar_add.xclbin --from-insts …/insts.bin` → **exit 0**, stdout **`PASS!`** |
| RUN3 (JIT e2e) | `python3 vector_scalar_add.py --dev npu2` → **exit 0**, stdout **`PASS!`** |
| Evidence dir | `/home/hunter/Projects/ai/mlir-aie-capture/completed_run_vector_scalar_add_20260811_150247/` |
| ctrltext sha16 | `aaeb5b68cc298a45` (byte-identical to already-landed research peels) |
| collect helper | `collect_load_accepted_candidate.sh --dry-run` → **REFUSE** `research_duplicate_sha16` (do not re-tag 300 B as load-accepted) |
| load_accepted_claim | **0** |
| invent_pass | **0** |
| product Pass / `ran_on_npu` | **No** (IRON PASS ≠ product `decide_ran_on_npu`) |
| Unlock / L3 / ALLOW | **not** opened; no `C_UNLOCK` mint |

### mlir-aie npu2 material (2026-08-10 AOT; 2026-08-11 IRON live verify)

**Honesty (required):** AOT peels from public mlir-aie (2026-08-10). **Same
bytes** completed IRON verify on this host FW 1.1.2.64 (2026-08-11 Path B).
Still **research-only** for load-accepted bag policy (research-dup sha16).
**invent_pass=0.** Not product Pass. Not C-ready. Not product `ran_on_npu`.
product_known_good=0. Source clone alone was **not** capture material; these
are concrete binary files.

| Field | Value |
|-------|--------|
| Source repo | https://github.com/Xilinx/mlir-aie |
| Example path | `programming_examples/basic/vector_scalar_add/` |
| Example commit (shallow clone) | `a8fea363dfd63015cffd380e0679950419a0a7cd` |
| Toolchain | mlir-aie wheel **1.4.0** (cp314 manylinux), llvm-aie / Peano **21.0.0.2026062301+cb664e8c**, aiebu-asm from host ryzen-ai-install venv |
| Target | **npu2** (`--dev npu2`) |
| Build command | ```python vector_scalar_add.py --dev npu2 --xclbin-path … --insts-path … --pdi-path … --elf-path …``` (AOT compile-only; no NPU EXEC required for emit) |
| Workdir (outside dragon-npu) | `/home/hunter/Projects/ai/mlir-aie-capture/aot_vector_scalar_add/` |
| Peel | `llvm-objcopy -O binary --only-section=.ctrltext` on `insts.elf` → `ctrltext.bin` (byte-identical to `insts.bin`) |

| File basename | size (B) | sha16 | sha256 |
|---------------|----------|-------|--------|
| `mlir_aie_vector_scalar_add_npu2.ctrltext.bin` | 300 | `aaeb5b68cc298a45` | `aaeb5b68cc298a450f01fb88d83426dcd702a01dfc2e79684b5f631db5885fd6` |
| `mlir_aie_vector_scalar_add_npu2.insts.bin` | 300 | `aaeb5b68cc298a45` | `aaeb5b68cc298a450f01fb88d83426dcd702a01dfc2e79684b5f631db5885fd6` |
| `mlir_aie_vector_scalar_add_npu2.insts.elf` | 992 | `790d8b3e6d25bda8` | `790d8b3e6d25bda8065605ac345d555d7bdc34642e36230fc81a1ea74d8db057` |
| `mlir_aie_vector_scalar_add_npu2.xclbin` | 8983 | `849d0ba7276ada71` | `849d0ba7276ada71ff2bc475bfdb0f8cd618335a3b6d500ddccf628ff315ca22` |
| `mlir_aie_vector_scalar_add_npu2.pdi` | 2528 | `fe438059f5d0e4eb` | `fe438059f5d0e4eb649f498d4d9cf71a8576fece357bbdf407f927f43fd8534e` |
| `mlir_aie_vector_scalar_add_npu2_core.elf` | 2468 | `ffd58932f0f38bca` | `ffd58932f0f38bcaf964956d13ea1b0b381ed30faa36f5d27482d696474fec79` |

**ELF notes:** `insts.elf` is aiebu-wrapped control/instruction ELF with sections
`.ctrltext` (0x12c), `.rela.dyn` (one rela), `.dynsym`. Core ELF is Peano
`elf32-aie` with `.text` only (compute kernel), not aiebu ctrltext.

**Done on Path B (2026-08-11):** IRON live EXEC + verify `PASS!` for these exact
peels (sha16 `aaeb5b68…`) on FW 1.1.2.64 — see **Path B completed IRON run**.

**Still not claimed:** product Pass; product `ran_on_npu` / `decide_ran_on_npu`;
load_accepted re-tag (research-dup); new `C_UNLOCK`; matmul-matched residual
identity (this is vector_scalar_add, not mladf matmul).

### Path B completed IRON run (2026-08-11) — single_core / whole_array / cascade matmul

**Agent attempt** after vector_scalar_add Path B PASS. Report:
`.agents/reports/wave5-path-b-matmul-attempt.md`.

| Field | Value |
|-------|--------|
| Date | 2026-08-11 |
| Host | horizon; device **RyzenAI-npu6**; NPU FW **1.1.2.64**; XRT 2.21.75 |
| Stack | mlir-aie IRON + system **pyxrt** on `PYTHONPATH` + capture venv |
| single_core (660 B) | `python3 single_core.py --dev npu2 -M 64 -K 64 -N 64 -m 32 -k 32 -n 32 --dtype_in i16 --dtype_out i32 -w 1 -i 2` → **exit 0**, **`PASS!`**, NPU ~165 us, ~3.19 GFLOPS |
| whole_array (2592 B) | `python3 whole_array.py --dev npu2 -M 128 -K 128 -N 128 -m 16 -k 16 -n 16 --n-aie-cols 4 --dtype_in i16 --dtype_out i32 -w 1 -i 2` → **exit 0**, **`PASS!`**, NPU ~297 us, ~14.13 GFLOPS |
| cascade (4512 B) | `python3 cascade.py --dev npu2 -M 128 -K 128 -N 128 -m 32 -k 32 -n 32 --n-aie-cols 4 --dtype_in i16 --dtype_out i32 -w 1 -i 2` → **exit 0**, **`PASS!`**, NPU ~1126 us, ~3.73 GFLOPS |
| Evidence dirs | `…/mlir-aie-capture/completed_run_single_core_matmul_20260811_150700/`; `…/completed_run_whole_array_matmul_20260811_150751/`; `…/completed_run_cascade_matmul_20260811_150822/` |
| ctrltext sha16 | `de8671245bb72436` (660), `7ee4b267ce31cd70` (2592), `18cb653c05a9d4c8` (4512) — **byte-identical** to already-landed research peels |
| `completed_on_this_fw` | **yes** (IRON verify on FW 1.1.2.64 for all three) |
| collect helper | all three dry-run → **REFUSE** `research_duplicate_sha16` (do not re-tag 660/2592/4512 as load-accepted) |
| load_accepted_claim | **0** |
| invent_pass | **0** |
| product Pass / `ran_on_npu` | **No** (IRON PASS ≠ product `decide_ran_on_npu`) |
| Unlock / L3 / ALLOW | **not** opened; no `C_UNLOCK` mint |
| journal scrape (1h) | match_total=0 (soft only; ≠ Pass) |

### mlir-aie npu2 material — single_core matmul (second set; 2026-08-10 AOT; 2026-08-11 IRON live verify)

**Honesty (required):** AOT peels from public mlir-aie (2026-08-10). **Same
bytes** completed IRON verify on this host FW 1.1.2.64 (2026-08-11 Path B).
Still **research-only** for load-accepted bag policy (research-dup sha16
`de867124…`). **invent_pass=0.** Not product Pass. Not C-ready. Not product
`ran_on_npu`. product_known_good=0. **Does not replace** vector_scalar_add
material or `working_stack_ctrltext.bin` (primary soft-compare ctrltext remains
the 300 B vector peel). Matmul-class geometry **≠** lab classic mladf 1688 B
identity; soft mismatch vs lab export remains expected until a true
residual-matched stream is captured.

| Field | Value |
|-------|--------|
| Source repo | https://github.com/Xilinx/mlir-aie |
| Example path | `programming_examples/basic/matrix_multiplication/single_core/` |
| Example commit (shallow clone) | `a8fea363dfd63015cffd380e0679950419a0a7cd` |
| Toolchain | mlir-aie wheel **1.4.0** (cp314 manylinux), llvm-aie / Peano **21.0.0.2026062301+cb664e8c**, aiebu-asm from host ryzen-ai-install (symlink only under `mlir-aie-capture/tools/`) |
| Target | **npu2** (`--dev npu2`) |
| Dims | M=K=N=**64**, tiles m=k=n=**32**, dtype_in=i16, dtype_out=i32 |
| Build command | ```python single_core.py --dev npu2 -M 64 -K 64 -N 64 -m 32 -k 32 -n 32 --dtype_in i16 --dtype_out i32 --xclbin-path … --insts-path …``` (AOT compile-only; CLI has no `--elf-path` / `--pdi-path`) |
| aiebu ELF | Post-build: `aiebu-asm -t aie2txn -c insts.bin -o insts.elf` |
| Peel | `llvm-objcopy -O binary --only-section=.ctrltext` on `insts.elf` → `ctrltext.bin` (660 B; **not** byte-identical to raw `insts.bin` — aiebu normalizes one header field) |
| PDI / core ELF | From `.prj/main.pdi` and `.prj/elfs_main_core_0_2/*.elf` |
| Workdir (outside dragon-npu) | `/home/hunter/Projects/ai/mlir-aie-capture/aot_single_core_matmul_npu2/` |

| File basename | size (B) | sha16 | sha256 |
|---------------|----------|-------|--------|
| `mlir_aie_single_core_matmul_npu2.ctrltext.bin` | 660 | `de8671245bb72436` | `de8671245bb72436ecf54949952b25096d59096afef8ecf6317f04695f60454f` |
| `mlir_aie_single_core_matmul_npu2.insts.bin` | 660 | `34baaff69260033c` | `34baaff69260033c0092dce3a5d90fd936206e0dba309a810e02b21b7ee0a3de` |
| `mlir_aie_single_core_matmul_npu2.insts.elf` | 1408 | `6fbbf33de3543be7` | `6fbbf33de3543be76d8bbbdda46d0f05b3b73a6e546f7ad8588ce51d8be4e28e` |
| `mlir_aie_single_core_matmul_npu2.xclbin` | 11993 | `8a96c4cb5e01656c` | `8a96c4cb5e01656c74b21d727e660472d05904f2d98dac1677da99355ffbee52` |
| `mlir_aie_single_core_matmul_npu2.pdi` | 5536 | `cd3d9fa6577d1e37` | `cd3d9fa6577d1e37c1a57b359b731e457becb1219031d845c250919b52d500c2` |
| `mlir_aie_single_core_matmul_npu2_core.elf` | 4632 | `3c17f06f9b690348` | `3c17f06f9b690348d70df647a690d604d62b819eb4599b5115445e036c7e0861` |

**ELF notes:** aiebu ELF `.ctrltext` size **0x294** (660 B); `.rela.dyn` has **5**
entries; gen4 / 1×6 cols×rows per aiebu report. Core ELF is Peano compute
kernel, not aiebu ctrltext.

**Size vs residual oracles (research only; residual pin tags 2026-08-11):**

| Stream | size (B) | Size-class / topology tag |
|--------|----------|---------------------------|
| This matmul ctrltext | **660** | **`public-npu2-small`** RESEARCH ONLY (IRON single_core 64³ i16→i32 AOT) |
| vector_scalar_add (primary) | **300** | **`public-npu2-small`** RESEARCH ONLY (minimal load) |
| Lab classic mladf ctrltext | **1688** | **`mladf-classic-1688`** staged residual oracle (rela×12) |
| flexml spent unified-2x4x4 PDI | **27200** | **`flexml-2x4x4-class`** spent (sha16 `0eeeb70c…`; not this PDI) |
| flexml spent llm-2x4x4 PDI | **21664** | **`flexml-2x4x4-class`** Spent under `8352882b` (sha16 `ba0cd738…`) |
| flexml spent unified-4x4 PDI | **13792** | **`flexml-2x4x4-class`** Spent under `efd44f35` (sha16 `7d172bf2…`) |
| This matmul PDI | **5536** | Built offline; not live CONFIG_CU proven |

**One-line residual note:** dual public material present (`public-npu2-small`);
still residual-mismatched to spent `flexml-2x4x4-class` and to lab classic
`mladf-classic-1688`. Not residual identity; not unlock alone; not Pass.

**Done on Path B (2026-08-11):** IRON live EXEC + verify `PASS!` for these exact
peels (sha16 `de867124…`) on FW 1.1.2.64 — see **Path B completed IRON run**
(matmul section above).

**Still not claimed:** product Pass; product `ran_on_npu` / `decide_ran_on_npu`;
load_accepted re-tag (research-dup); new `C_UNLOCK`; identity match to lab
classic 1688 or flexml PDI; swap of `working_stack_ctrltext.bin` primary
(left as vector 300 B on purpose).

### mlir-aie npu2 material — whole_array matmul (third set; 2026-08-11 AOT + IRON live verify)

**Honesty (required):** AOT peels from public mlir-aie whole_array. **Same
bytes** completed IRON verify on this host FW 1.1.2.64 (2026-08-11 Path B).
Still **research-only** for load-accepted bag policy (research-dup sha16
`7ee4b267…`). **invent_pass=0.** Not product Pass. Not C-ready. Not product
`ran_on_npu`. product_known_good=0. **Does not replace** vector_scalar_add,
single_core matmul, or `working_stack_ctrltext.bin` (primary soft-compare
ctrltext remains the 300 B vector peel). Whole-array matmul geometry **≠** lab
classic mladf 1688 B identity; **≠** flexml spent research PDI class; soft
mismatch vs lab export remains expected. **Not an unlock reason alone.**

| Field | Value |
|-------|--------|
| Source repo | https://github.com/Xilinx/mlir-aie |
| Example path | `programming_examples/basic/matrix_multiplication/whole_array/` |
| Example commit (shallow clone) | `a8fea363dfd63015cffd380e0679950419a0a7cd` |
| Toolchain | mlir-aie wheel **1.4.0** (cp314 manylinux), llvm-aie / Peano **21.0.0.2026062301+cb664e8c**, aiebu-asm from host ryzen-ai-install (symlink only under `mlir-aie-capture/tools/`) |
| Target | **npu2** (`--dev npu2`) |
| Topology | **4×4** compute array (`--n-aie-cols 4`, fixed 4 rows) = **16** Peano core ELFs in `.prj/` |
| Dims | M=K=N=**128**, tiles m=k=n=**16**, dtype_in=i16, dtype_out=i32 (modest; larger ctrltext than single_core 660 B) |
| Build command | ```python whole_array.py --dev npu2 -M 128 -K 128 -N 128 -m 16 -k 16 -n 16 --n-aie-cols 4 --dtype_in i16 --dtype_out i32 --xclbin-path … --insts-path …``` (AOT compile-only; no NPU EXEC) |
| aiebu ELF | Post-build: `aiebu-asm -t aie2txn -c insts.bin -o insts.elf` (Header 0.1, gen4, cols/rows **8×6**, TransactionSize **2592**, NumOps **68**) |
| Peel | `llvm-objcopy -O binary --only-section=.ctrltext` → `ctrltext.bin` (**2592 B**; **not** byte-identical to raw `insts.bin`) |
| PDI / core ELF | From `.prj/main.pdi` and first core `.prj/elfs_main_core_0_2/*.elf` (16 cores present; only one representative core ELF landed) |
| Workdir (outside dragon-npu) | `/home/hunter/Projects/ai/mlir-aie-capture/aot_whole_array_matmul_npu2/` |

| File basename | size (B) | sha16 | sha256 |
|---------------|----------|-------|--------|
| `mlir_aie_whole_array_matmul_npu2.ctrltext.bin` | 2592 | `7ee4b267ce31cd70` | `7ee4b267ce31cd70bfd880ab882a339e456a2a9ecabc4c1b38622fb159f2d1af` |
| `mlir_aie_whole_array_matmul_npu2.insts.bin` | 2592 | `ae692499a79c74b4` | `ae692499a79c74b46175cb4841f235edb0059175144941522f792105b758dec3` |
| `mlir_aie_whole_array_matmul_npu2.insts.elf` | 3520 | `0984b40244c33f68` | `0984b40244c33f681bfb9fecc664fde51e8b69f7a7d3d70042a81d23d90bb1a9` |
| `mlir_aie_whole_array_matmul_npu2.xclbin` | 73327 | `44ab78cf9eeb315e` | `44ab78cf9eeb315ee8ef67dba011b7095916df4218783668dfcc2e9ea66639b0` |
| `mlir_aie_whole_array_matmul_npu2.pdi` | 66864 | `e721b80335e6d8ed` | `e721b80335e6d8ed162380284ee2cf1287b0fde3059f2610feb01802e3b8a6c3` |
| `mlir_aie_whole_array_matmul_npu2_core.elf` | 4456 | `2d47c4d6017cc6a6` | `2d47c4d6017cc6a643914cfef59495c03f31b25084c75ceede698e8463cbfc10` |

**ELF notes:** aiebu ELF `.ctrltext` size **0xa20** (2592 B); `.rela.dyn` has
**20** entries; gen4 / 8×6 cols×rows per aiebu report. Representative core ELF
is Peano compute kernel for tile (0,2), not aiebu ctrltext.

**Size vs residual oracles (research only):**

| Stream | size (B) | Class note |
|--------|----------|------------|
| This whole_array ctrltext | **2592** | IRON whole_array 4×4 128³ i16→i32 AOT (**larger** than single_core 660) |
| single_core matmul ctrltext | **660** | Prior second set |
| vector_scalar_add (primary bag) | **300** | Prior minimal load |
| Lab classic mladf ctrltext | **1688** | Staged residual oracle (rela×12); **≠** this 2592 body |
| flexml spent unified-2x4x4 PDI | **27200** | sha16 `0eeeb70c…` (not this PDI) |
| flexml spent llm-2x4x4 PDI | **21664** | Spent under `8352882b` (sha16 `ba0cd738…`) |
| flexml spent unified-4x4 PDI | **13792** | Spent under `efd44f35` (sha16 `7d172bf2…`) |
| This whole_array PDI | **66864** | Built offline; not live CONFIG_CU proven |

**Done on Path B (2026-08-11):** IRON live EXEC + verify `PASS!` for these exact
peels (sha16 `7ee4b267…`) on FW 1.1.2.64 — see **Path B completed IRON run**
(matmul section above).

**Still not claimed:** product Pass; product `ran_on_npu`; load_accepted re-tag
(research-dup); new `C_UNLOCK`; identity match to lab classic 1688 or flexml
PDI; swap of `working_stack_ctrltext.bin` primary. Cascade matmul fourth set
landed below (also Path B complete same day).

### mlir-aie npu2 material — cascade matmul (fourth set; 2026-08-11 AOT + IRON live verify)

**Honesty (required):** AOT peels from public mlir-aie cascade. **Same bytes**
completed IRON verify on this host FW 1.1.2.64 (2026-08-11 Path B). Still
**research-only** for load-accepted bag policy (research-dup sha16
`18cb653c…`). **invent_pass=0.** Not product Pass. Not C-ready. Not product
`ran_on_npu`. product_known_good=0. **Does not replace** vector, single_core,
whole_array, or `working_stack_ctrltext.bin` (primary soft-compare ctrltext
remains the 300 B vector peel). Cascade-stream topology research only.
**≠** lab classic mladf 1688 B identity; **≠** flexml spent research PDI class;
soft mismatch vs lab export remains expected. **Not residual identity. Not an
unlock reason alone. Public AOT ≠ Pass. IRON PASS ≠ product Pass.**

| Field | Value |
|-------|--------|
| Source repo | https://github.com/Xilinx/mlir-aie |
| Example path | `programming_examples/basic/matrix_multiplication/cascade/` |
| Example commit (shallow clone) | `a8fea363dfd63015cffd380e0679950419a0a7cd` |
| Toolchain | mlir-aie wheel **1.4.0** (cp314), llvm-aie / Peano under capture venv, aiebu-asm symlink under `mlir-aie-capture/tools/` |
| Target | **npu2** (`--dev npu2`) |
| Topology | **4×4** compute array (`--n-aie-cols 4`, fixed 4 rows) using AIE **cascade streams** for vertical partial-product accumulate = **16** Peano core ELFs in `.prj/` |
| Dims | M=K=N=**128**, tiles m=k=n=**32**, dtype_in=i16, dtype_out=i32 (modest; default cascade Makefile is 512³) |
| Build command | ```python cascade.py --dev npu2 -M 128 -K 128 -N 128 -m 32 -k 32 -n 32 --n-aie-cols 4 --dtype_in i16 --dtype_out i32 --xclbin-path … --insts-path …``` (AOT compile-only; no NPU EXEC) |
| aiebu ELF | Post-build: `aiebu-asm -t aie2txn -c insts.bin -o insts.elf` (Header 0.1, gen4, cols/rows **8×6**, TransactionSize **4512**, NumOps **116**, UID `9d1e9296…`) |
| Peel | `llvm-objcopy -O binary --only-section=.ctrltext` → `ctrltext.bin` (**4512 B**; distinct sha from prior 300/660/2592 peels) |
| PDI / core ELF | From `.prj/main.pdi` and representative `.prj/elfs_main_core_0_2/*.elf` (16 cores present; only one core ELF landed). Standalone xclbin section `PDI` absent (`dump_xclbin_pdi.sh` refuse); AIE_PARTITION JSON-only — same pattern as other mlir-aie AOT sets. |
| Workdir (outside dragon-npu) | `/home/hunter/Projects/ai/mlir-aie-capture/aot_cascade_matmul_npu2/` |
| Build log | `/tmp/cascade_aot_build.log` (host ephemeral) |

| File basename | size (B) | sha16 | sha256 |
|---------------|----------|-------|--------|
| `mlir_aie_cascade_matmul_npu2.ctrltext.bin` | 4512 | `18cb653c05a9d4c8` | `18cb653c05a9d4c89683f545d8ad6fd2cb8eae36279ed33358ea5ba33a195541` |
| `mlir_aie_cascade_matmul_npu2.insts.bin` | 4512 | `e1e4db951d44092d` | `e1e4db951d44092d041e206a0e1e22980e853c7f4e256804c5be7339c39b861c` |
| `mlir_aie_cascade_matmul_npu2.insts.elf` | 5632 | `486bc26f5f5de1be` | `486bc26f5f5de1be207cc1fab5534c150c4076370485e1b20eefc7aa06998680` |
| `mlir_aie_cascade_matmul_npu2.xclbin` | 46153 | `478214ec08d481f7` | `478214ec08d481f73a54b707500f6456c11773756b15c8a07cf1ecad13b38929` |
| `mlir_aie_cascade_matmul_npu2.pdi` | 39696 | `e2a2b177dc3f4a5e` | `e2a2b177dc3f4a5ea2021b19433cfd18d8312af20c523a3346d2ba0f73c6e6f9` |
| `mlir_aie_cascade_matmul_npu2_core.elf` | 3308 | `0f56a114b434bbf7` | `0f56a114b434bbf757eec6f1135d3e13025af2c9835a152c4c175a25d90433f6` |

**ELF notes:** aiebu ELF size 5632; `.ctrltext` peel **4512 B** (matches
TransactionSize). Representative core ELF is Peano compute kernel for tile
(0,2), not aiebu ctrltext. Cascade kernels use put_only / get_only / put_get
paths (vertical cascade accumulate), unlike whole_array parallel-only matmul.

**Size vs residual oracles (research only):**

| Stream | size (B) | Class note |
|--------|----------|------------|
| This cascade ctrltext | **4512** | IRON cascade 4×4 128³ i16→i32 AOT (**new** sha16 `18cb653c…`; larger than whole_array 2592) |
| whole_array matmul ctrltext | **2592** | Prior third set |
| single_core matmul ctrltext | **660** | Prior second set |
| vector_scalar_add (primary bag) | **300** | Prior minimal load / primary soft bag |
| Lab classic mladf ctrltext | **1688** | Staged residual oracle; **≠** this 4512 body |
| flexml spent unified-2x4x4 PDI | **27200** | sha16 `0eeeb70c…` (not this PDI) |
| flexml spent llm-2x4x4 PDI | **21664** | Spent under `8352882b` (sha16 `ba0cd738…`) |
| flexml spent unified-4x4 PDI | **13792** | Spent under `efd44f35` (sha16 `7d172bf2…`) |
| This cascade PDI | **39696** | Built offline from `.prj/main.pdi`; not live CONFIG_CU proven; **≠** spent flexml sizes/sha |

**Ingest:** `bash tools/pass_unblock_once.sh --ingest` for each basename above
(research-only; invent_pass=0). Soft compare: sha_match=0 structural_match=0
expected vs lab classic; residual_not_closed=1.

**Done on Path B (2026-08-11):** IRON live EXEC + verify `PASS!` for these exact
peels (sha16 `18cb653c…`) on FW 1.1.2.64 — see **Path B completed IRON run**
(matmul section above).

**Still not claimed:** product Pass; product `ran_on_npu`; load_accepted re-tag
(research-dup); new `C_UNLOCK`; residual identity; swap of
`working_stack_ctrltext.bin` primary.
