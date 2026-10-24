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

## Mini glossary (so this doc is readable)

| Term | Plain meaning |
|------|----------------|
| **KMQ** | **Kernel Mode Queue.** This host’s `device_type=0`: the **kernel** owns the submit path. XRT/shim uses ioctls (`shim/kmq/`). **Not** “needs sudo.” |
| **UMQ** | **User Mode Queue.** Other NPU generations in the driver tree. **Not** this laptop’s bind path. Do not “switch to UMQ” on Strix NPU4. |
| **`umq_bo` field** | Optional buffer handle on CREATE_HWCTX; may be **0** on KMQ. ≠ “running as UMQ device.” |
| **xclbin** | Container for an NPU app image (includes PDI). |
| **PDI** | Opaque app/partition blob inside the xclbin; public stack registers it on the hardware context. |
| **CU** | Compute unit (one programmed slot the xclbin defines). |
| **CONFIG_CU** | Ioctl after create-context: “this CU uses that PDI buffer.” Lab: optional via `XDNA2_LAB_PDI_BLOB` (default off). |
| **DBG_BUF** | Optional hwctx debug buffer (`ASSIGN_DBG_BUF` / `REMOVE_DBG_BUF` in public uAPI; XRT `config_ctx_debug_bo`). **AIE2 pack (C6):** `pack_config_hwctx_dbg_buf_aie2` / `validate_config_hwctx_dbg_buf_aie2` (`param_val` = BO handle value). Soft prep: `format_c6_debug_bo_prep_note`. **Research-gated wire:** `DirectSubmission::config_hwctx_dbg_buf` refuse unless `XDNA2_LAB_RESEARCH_DEBUG_BO` set. **Lab timed path:** when that env is set, `try_lab_attach_research_debug_bo_from_env` CREATE 4096 B **Dev** (`AMDXDNA_BO_DEV`; not Shmem — driver type filter; live `48da6e60` Shmem failed) + Assign after hwctx (REMOVE on guard drop); soft: `format_research_debug_bo_config_detail` / `lab_debug_bo_attach=1` / `debug_bo_ty=dev`. Env unset → no DBG_BUF. Assign alone ≠ Pass. |
| **ctrltext** | Per-job tile program (what we peel from `lab_instr.elf`). |
| **START_NPU** | The run-this-job command (instruction buffer + I/O buffers). |
| **Privilege (product)** | Admin installs driver/udev **once**; product users open `/dev/accel` **without** sudo. Lab L3 flip is research-only — see [`XDNA2_NEVER_AGAIN.md`](./XDNA2_NEVER_AGAIN.md) § Product vs lab privileges. |

## Two different “loadable” things (easy to conflate)

| Piece | What it is | Who builds it in the public stack | Do we send it? |
|-------|------------|-------------------------------------|----------------|
| **CU / PDI** | Per-compute-unit app image from **xclbin** | XRT shim (KMQ = kernel-queue path): copy PDI into a buffer → **CONFIG_HWCTX / CONFIG_CU** → driver tells firmware the PDI address | **Optional** — default off; set `XDNA2_LAB_PDI_BLOB` for opaque CONFIG_CU (research; wrong overlay OK for machinery) |
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
`aiebu::aiebu_assembler` and `patch_ctrl_code`. Production path (KMQ =
kernel-mode queue / normal XRT→driver path) with an xclbin does steps 1–4.

---

## Our pure-direct lab path (what actually ran)

```text
1. CREATE_HWCTX (tiles=32, QoS gops/fps non-zero)  — no xclbin, no CU list
        │
        ▼
2. Optional lab CONFIG_CU (default **off**):
     if XDNA2_LAB_PDI_BLOB=/path/to/opaque.pdi is set:
       CREATE_BO(Dev) + write opaque bytes + CONFIG_HWCTX(CONFIG_CU)
     else: skip (historical path — never CONFIG_CU)
     Operator dump: bash tools/dump_xclbin_pdi.sh --print-env
     Kit: bash tools/operator_app_load_kit.sh a-prep
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
| **CU/PDI from xclbin + CONFIG_CU** | **Yes on production path (KMQ = kernel-queue XRT)** | **Optional** via `XDNA2_LAB_PDI_BLOB` (default off; host AIE2P dump OK for machinery, may be wrong CU for mladf) | **Machinery landed; live proof still open** |
| Full ELF in instr BO vs peeled ctrltext | Shim peels ctrltext | Peel ctrltext | Match shim test shape |
| Separate CTRL BO + meta size=6 patch | Yes in DD/meta | Yes (lab residual vs full XRT fidelity) | Partial; not proven root of 0x3000003 |
| Generation path aie2 msg vs aie4 | Host 1022:17f0 Strix | Same silicon; aie2-style status codes observed | Generation **catalog** closed; still worth noting aie4 enums differ on newer FW paths |

---

## Wave W0 depth — three construction paths + BO pins (2026-08-11)

**Research only. invent_pass=0. Not Pass. Not residual match. Not unlock alone.**
Named offline facts only (measured or open-source map). Soft compare ≠ Pass.

### Three distinct construction paths (easy to conflate)

| Path | Who builds | Order (summary) | Present in bag / lab? |
|------|------------|-----------------|------------------------|
| **A. XRT/DD production (KMQ)** | Shim + driver on live NPU | xclbin → CREATE_HWCTX → **CONFIG_CU** (PDI BO per CU) → per job aiebu peel/patch → **START_NPU** (instr Dev BO + IFM/WTS/OFM kernel args, `prop_count=0`) → EXEC/WAIT | Not captured as post-patch residual identity on this host |
| **B. Lab pure-direct** | dragon-npu silicon_lab | CREATE_HWCTX (no xclbin) → optional CONFIG_CU → DD classic 1688 + aiebu ELF rela×12 → START_NPU content=1688 layout xrt3 → EXEC | Live residual: historical `0x3000003`; post-CONFIG_CU flexml class `wait_cmd_einval` |
| **C. mlir-aie / IRON AOT (research generator)** | Public example AOT **offline** | Peano cores + CDO → **xclbin + PDI + insts.bin** emit **without** host EXEC → host `aiebu-asm -t aie2txn` → peel `.ctrltext` | Under `testdata/xdna2_cmd/external_known_good/` (vector 300 / single_core 660 / whole_array 2592 + siblings). **RESEARCH ONLY** |

Path C is **not** a completing XRT/DD load on this firmware. It is a public
generator of **roles** (ctrltext vs PDI vs core ELF) for offline classify.
**Do not** treat Path C bins as residual identity or Pass.

### Artifact roles (IRON bag; measured offline)

| Artifact | Role in load map | Lab analogue | Notes |
|----------|------------------|--------------|-------|
| `*.ctrltext.bin` / peeled `.ctrltext` | START_NPU **instruction_buffer** body | silicon_lab 1688 ctrltext | Primary soft-compare body |
| `*.insts.bin` | Raw txn before/after aiebu normalize | DD `lab_instr.bin` | May **≠** peel sha (aiebu normalizes a header field; vector 300 is identical; 660/2592 peels differ) |
| `*.insts.elf` | aiebu ELF (`.ctrltext` + `.rela.dyn` + `.dynsym`) | `lab_instr.elf` | Rela counts: lab **12**, vector **1**, single_core **5**, whole_array **20** |
| `*.pdi` | CONFIG_CU opaque app image candidate | `XDNA2_LAB_PDI_BLOB` | Sizes 2528 / 5536 / 66864 **≠** spent flexml 27200/21664/13792 |
| `*.xclbin` | Container (section map research only) | none in pure-direct | Markers/offsets offline; no private body decode |
| `*_core.elf` | Peano **compute** kernel | none on START_NPU path | **Not** ctrltext; do not EXEC as instr BO |

### START_NPU + BO pin order (XRT-ish vs lab)

Open-source map (`docs/XDNA2_XRT_CONTRACT_MLADF.md` + shim `exec_buf.h` /
`elf_io` + lab path):

```text
# Production / shim elf_io shape (after CONFIG_CU when xclbin path used)
CREATE_BO(Dev) instr  → fill peeled+patched ctrltext
CREATE_BO(Shmem) IFM / WTS / OFM  (+ optional CTRL)
START_NPU:
  instruction_buffer      = instr BO xdna_addr
  instruction_buffer_size = content length (not whole page unless size_mode=bo)
  instruction_prop_count  = 0
  kernel args (xrt3): n_bos=3, pad, IFM, WTS, OFM, pads  → 15 u32
EXEC_CMD pins arg_bo_handles; WAIT_CMD → ERT Completed + OFM check (Pass only then)

# Lab pure-direct (silicon_lab)
same START_NPU shape when LAB_OFM + pack mode
CTRL may be 4th BO (count4) or meta patch-only
rela patch: overwrite default (sym 1=IFM, 2=WTS, 3=OFM)
```

**Closed offline as sole root of historical APP_LOAD:** raw bin, content vs bo
size mode, xrt3 vs count4, rela overwrite vs add, QoS/tiles, DevHeap 64 MiB.
**Open:** matched material + topology (B1 hypothesis: flexml PDI vs lab classic
ctrltext mismatch) and residual class honesty after attach/skip.

### Isolation measured (live, already CONSUMED — do not re-run)

| Shot | What | Result (honest) |
|------|------|-----------------|
| flexml CONFIG_CU attach (`2ae69036` / `8352882b` / `efd44f35`) | Path B + research PDI | Attach OK → post-submit **`wait_cmd_einval`**; some IO_PAGE_FAULT bands. **≠ Pass** |
| skip-CONFIG_CU (`1f761bf4`) | Path B **without** PDI overlay | **`wait_cmd_einval` persists**; `config_cu_present=0`; total ~0.17s. Overlay is **not** sole residual root. **≠ Pass** |

Unlock stamps above are **CONSUMED**. invent_pass=0. No re-live under same unlock.

### Size-class / structural axes vs residual oracles (cite only)

| Stream | len | `CtrltextSizeClass` | Header (aiebu report) | rela | vs lab 1688 |
|--------|----:|---------------------|----------------------|-----:|-------------|
| lab classic | 1688 | `mladf_classic_1688` | ops **40**, gen4, **8×6** | 12 | self oracle |
| vector_scalar_add | 300 | `small_public_npu2` | classic aiebu wrap (rela×1) | 1 | size_class_match=0 |
| single_core matmul | 660 | `small_public_npu2` | ops **17**, gen4, **1×6** | 5 | size_class_match=0; header_cols_diff vs 8 |
| whole_array matmul | 2592 | `other` (>1024, ≠1688) | ops **68**, gen4, **8×6** | 20 | size_class_match=0; ops_diff=28 |

Route B tools: `research_external_vs_lab_structural_distance`,
`compare_rela_structure_lab_vs_external_elf`,
`fingerprint_capture_material_dir_research`,
`classify_material_residual_match` (always `residual_matched_claim=0` offline).
**Distance ≠ match ≠ Pass.** Report: `.agents/reports/wave0-structural-diff.md`.

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

## Operator tools (A + B)

| Track | Command | What it does |
|-------|---------|--------------|
| A prep | `bash tools/dump_xclbin_pdi.sh --print-env` | Dump opaque PDI; print `XDNA2_LAB_PDI_BLOB=` |
| **A max-prep** | `bash tools/operator_app_load_kit.sh config-cu` | a-prep + pin last_apply + preflight + filled sandwich |
| Post evidence | `bash tools/collect_post_c2_evidence.sh [json]` | Journal + p5 pack after restore (no L3) |
| A+B kit | `bash tools/operator_app_load_kit.sh` | Overview + status both tracks |
| B status | `bash tools/operator_external_capture.sh --status` | Capture dir ready? |
| B install | `… --install /path/ctrltext.bin` | Safe copy into `external_known_good/` |
| B compare | `… --compare` | Soft classify with external path |

Live apply only under an **active** one-shot in `RESIDUAL.md` (CONFIG_CU + PDI
path; ≥120s outer). **Current unlock: none.** All prior one-shots CONSUMED
(`2ae69036` flexml, `97f38307`, `11bc9016`, `3a70fabf`, `b17b0119`). Attach
CONFIG_CU ≠ Pass. invent_pass=0.

### Spent vs unspent research PDI ledger (not product known-good)

| tag | pdi_bytes | sha16 (16) | Live status |
|-----|-----------|------------|-------------|
| flexml STX `unified-2x4x4` | 27200 | `0eeeb70c84df056e` | **Spent** under `2ae69036` (`wait_cmd_einval`; not Pass) |
| flexml STX `llm-2x4x4` | 21664 | `ba0cd738fd870011` | **Not spent** (unspent candidate; needs **new** unlock) |
| flexml STX `unified-4x4` | 13792 | `7d172bf223a9b06b` | **Not spent** (weaker topology A/B; needs **new** unlock) |
| CVML AIE2P 4x4 Overlay | 283840 | `a1e65d04add3422c` | **Spent** prior unlocks (orphan IO_PAGE_FAULT class) |

Research dump path: `tools/dump_xclbin_pdi.sh` / `pass_unblock_fingerprint_once.sh`.
**Never** treat these as product known-good. Re-live spent FPs alone is not progress.

## Public-stack ctrltext extract playbook (path B)

**Research only. invent_pass=0. Match ≠ Pass. Capture ≠ C-ready ≠ `ran_on_npu`.**

**Hard rule:** residual tools need **concrete binaries** under
`testdata/xdna2_cmd/external_known_good/` (or
`bash tools/pass_unblock_once.sh --ingest PATH [NAME]` — **PATH required**).
A **source clone alone is not capture material.**

Exact operator steps when you have a public stack that **completes** on this NPU:

1. **Run a working public stack** that finishes EXEC (ERT Completed + useful OFM).
   Prefer the same mladf key family as lab when possible (`…_1_1152_256_128`).
   Stacks: XRT / DynamicDispatch / Ryzen AI EP (outside this repo).
2. **Capture post-patch ctrltext** (instruction BO image after peel/patch that
   goes into START_NPU), or aiebu ELF with the peel step documented.
3. **Ingest into the capture directory** (primary entrypoint):

   ```bash
   cd ~/Projects/ai/dragon-npu
   bash tools/pass_unblock_once.sh --ingest /path/to/ctrltext.bin working_stack_ctrltext.bin
   # same install surface:
   # bash tools/operator_external_capture.sh --install /path/to/ctrltext.bin working_stack_ctrltext.bin
   # kit: bash tools/operator_app_load_kit.sh b-install …
   ```

4. **Fill the capture table** in
   `testdata/xdna2_cmd/external_known_good/PROVENANCE.md` (source stack, key,
   peel/patch notes). Keep research-only honesty; no invent Pass tokens.
5. **Soft compare** (no L3 / no ALLOW / no invent Pass):

   ```bash
   bash tools/operator_external_capture.sh --compare
   # or kit: bash tools/operator_app_load_kit.sh b-compare
   ```

**Refuse list (not bag-eligible):** local oracles under lab / GT, including
`gt_capture/instr_post_patch.bin`, `instr_vendor_post_patch.bin`,
`ctrl_post_patch.bin`, `instr_raw_txn.bin`, `instr_vendor_txn_bin.bin`, and VE2
`validate.xclbin` / `NOT_AIE2` stubs. Agent **must not invent** capture bytes.

### MLIR-AIE / IRON material (concrete generator path)

When you do **not** already have a completing XRT/DD stack: build a **small**
public example (passthrough / vector_scalar_add / single_core matmul /
getting_started), collect `*.xclbin` / `insts_*.bin` / aiebu elf / `.ctrltext`,
ingest + PROVENANCE. Full step list:

**[`docs/CAPTURE_MATERIAL_MLIR_AIE.md`](./CAPTURE_MATERIAL_MLIR_AIE.md)**

That doc also covers Peano, aiebu, AMD IRON, and FastFlowLM/Lemonade **fallback
intercept**. Clone-exists ≠ capture bins.

## Residual-ready note (B1) — lab instruction key vs flexml overlay topology

**Status for residual pin (Phase 2 owns `RESIDUAL.md`):** this is a **named
hypothesis**, not a proven root cause, and **not** a Pass claim.

| Fact (measured) | Interpretation (hypothesis only) |
|-----------------|----------------------------------|
| All three flexml STX CONFIG_CU research PDIs were live-attached under stock and then failed post-attach (`wait_cmd_einval`; some with journal `IO_PAGE_FAULT` / cmd timeout). Spent sha16: unified-2x4x4 `0eeeb70c…`, llm-2x4x4 `ba0cd738…`, unified-4x4 `7d172bf2…`. | Overlay attach alone did not unlock compute. |
| Instruction path on those shots stayed the **lab classic pack key** `mladfmatmulbias_prmptflat_mladf_2x4x4_flat_a16fw4acc16f_1_1152_256_128` (silicon_lab aiebu ELF / 1688 B ctrltext). Live reports: `.agents/reports/live-shot-8352882b.md`, `live-shot-efd44f35.md`, `live-shot-2ae69036.md`. | **Hypothesis:** flexml overlay topology (partition / tile map / PDI) may be **mismatched** to the lab classic START_NPU ctrltext / map, so firmware accept-or-run fails even when CONFIG_CU attach returns success. |
| Public mlir-aie npu2 AOT under `testdata/xdna2_cmd/external_known_good/` is **research only** (size class `public-npu2-small`; residual-mismatched to spent flexml and lab classic 1688). | Capture presence ≠ product known-good ≠ Pass. |

**What this is not:** proven root cause; unlock justification alone; permission
to re-live spent flexml; invent_pass. Citable in a future unlock **reason text**
only with invent_pass=0 and a **new** named offline reason (e.g. skip-CONFIG_CU
isolation or new material). See report
`.agents/reports/impl-next-load-path-offline.md`.

## Skip-CONFIG_CU offline prep checklist (A/B isolation)

**Purpose:** isolate whether residual is **CONFIG_CU / PDI attach** vs
**START_NPU / map / lab ctrltext**. Offline prep anytime; **live arm only** under
a **new** active `C_UNLOCK` in `RESIDUAL.md`. invent_pass=0 always.
**skip-CU prep ≠ Pass ≠ `ran_on_npu`.**

### Exact env (skip arm)

```bash
# Required for skip arm (lab never issues CONFIG_CU):
unset XDNA2_LAB_PDI_BLOB          # or leave empty — both mean no CONFIG_CU
export XDNA2_LAB_SKIP_CONFIG_CU=1 # loud flag for logs / sandwich / last_apply pin name

# Keep usual lab honesty (never sticky product Pass gates):
unset XDNA2_ALLOW_EXEC_CMD
# Optional BO diag on live: export XDNA2_LAB_BO_DIAG=1
# Optional research: lab timed path CREATE+Assign 4096 B Dev debug BO
# (AMDXDNA_BO_DEV; not Shmem — spent under 48da6e60; invent_pass=0; not Pass):
# XDNA2_LAB_RESEARCH_DEBUG_BO=1
```

### Kit / script commands (prefer kit)

| Offline (no unlock) | Live (active unlock only) |
|---------------------|---------------------------|
| `bash tools/operator_app_load_kit.sh config-cu-skip` | `bash tools/operator_app_load_kit.sh config-cu-run-skip` |
| same: `bash tools/hitl_config_cu_once.sh prep-skip-pdi` | same: `bash tools/hitl_config_cu_once.sh run-stock-skip-pdi` |
| Compare prep with CONFIG_CU: `… config-cu` / `prep` | Compare live with CONFIG_CU: `… config-cu-run` / `run-stock` |

After any live shot: `bash tools/operator_app_load_kit.sh post-evidence [JSON]`
then `bash tools/p5_post_c2_interpret.sh` (or c3+c4+p6). Outer live budget
`--timeout-s=120` (or ≥120).

### Spent-sha process refuse (still applies if someone points PDI at spent)

Do **not** re-live these research fingerprints alone (attach already measured;
re-live without a **new** unlock + named delta is banned):

| Tag | Bytes | sha16 | Unlock (CONSUMED) |
|-----|------:|-------|-------------------|
| flexml unified-2x4x4 | 27200 | `0eeeb70c84df056e` | `2ae69036` |
| flexml llm-2x4x4 | 21664 | `ba0cd738fd870011` | `8352882b` |
| flexml unified-4x4 | 13792 | `7d172bf223a9b06b` | `efd44f35` |
| CVML/AIE2P research dump | 283840 | `a1e65d04add3422c` | `97f38307` / priors |

Skip arm must **not** export those paths as `XDNA2_LAB_PDI_BLOB`. If a spent
path is set, you left the skip experiment (CONFIG_CU arm) and must still obey
spent re-live rules. Ledger tooling:
`bash tools/pass_unblock_fingerprint_once.sh --offline` (skip spent re-dump
unless `--force`).

### Expected residual classes (honest; not Pass)

| Class / signal | May appear on skip arm? | Notes |
|----------------|-------------------------|--------|
| `config_cu=0` / absent in last_apply | **Yes (expected)** | No PDI → no CONFIG_CU |
| `wait_cmd_einval` | Possible | Same post-submit class as flexml attach shots |
| `wait_timeout` | Possible | True WAIT timeout (classifier-honest) |
| `app_load_pdi_fail_after_aiebu` / FW `0x3000003` | Possible | Historical without matched load image |
| Journal `IO_PAGE_FAULT` / `aie2_set_cmd_timeout` | Possible | Orphan vs BO map is offline audit only |
| `config_cu=1` + attach | **No** on pure skip | If present, PDI was set — not skip arm |
| Product Pass / `ran_on_npu=true` | **No** until OFM allclose + ERT Completed + decide | invent_pass=0 |

### A/B live sequence (docs only until new unlock)

| Step | Action | Notes |
|------|--------|-------|
| 1 | Offline prep **with** CONFIG_CU | `kit config-cu` / `hitl_config_cu_once prep` |
| 2 | Offline prep **without** CONFIG_CU | `kit config-cu-skip` → `SKIP=1`, PDI unset |
| 3 | Live arm A (CONFIG_CU) | Only under **new** active unlock; prefer **new** material, not spent flexml alone |
| 4 | Live arm B (skip-PDI) | Only under **new** unlock (or operator-documented two-arm plan); `config-cu-run-skip` |
| 5 | Compare last_apply + journal | `config_cu=` present vs absent; status / page-fault / timeout class |
| 6 | Never claim Pass from either arm alone | invent_pass=0; attach ≠ Pass; skip prep ≠ Pass |

**Hard rules:** active unlock required for live; never auto-mint unlock; do not
reopen CONSUMED stamps (`efd44f35`, `8352882b`, `2ae69036`, `97f38307`,
`11bc9016`, `3a70fabf`, `b17b0119`); tool headers:
`tools/hitl_config_cu_once.sh`, `tools/operator_app_load_kit.sh`.

## Highest-value next offline questions (named, not track letters)

1. **Load-accepted / residual-matched material (offline first)**
   Public IRON bins are present (`public-npu2-small` + whole_array `other`) but
   **residual-mismatched** to lab classic 1688 and spent flexml PDI class.
   Prefer material that shares lab classic topology **or** a public stack that
   **completes** EXEC on this firmware (true path-B capture). Soft classify /
   Route B distance OK; **≠** Pass **≠** unlock alone.

2. **Known-good instruction BO capture (path B, no unlock)**
   Follow **Public-stack ctrltext extract playbook** above → ingest → PROVENANCE
   → soft compare. Prefer completing XRT/DD over reopening L3 under consumed
   unlock. Generator playbook:
   **[`CAPTURE_MATERIAL_MLIR_AIE.md`](./CAPTURE_MATERIAL_MLIR_AIE.md)**.
   **Source tree ≠ capture material.** Public bins remain research only.

3. **Next live only under new unlock + named reason**
   Prior: skip-CONFIG_CU isolation **already run** under `1f761bf4` CONSUMED
   (residual persisted without overlay). Do **not** empty-reconfirm skip-CU.
   Future live needs **new** host-minted unlock + offline reason (new material
   and/or named delta). invent_pass=0.

4. **Do not**
   - Empty gate-only stamp rolls / empty reconfirm mill
   - Reopen execute-allow under any CONSUMED unlock
   - Re-live spent flexml / CVML sha16 alone
   - Re-run closed env matrix (content / xrt3 / rela add) without new bytes
   - Invent `ran_on_npu` / invent residual_matched / invent capture bytes
   - Claim Pass from attach, capture presence, or structural distance alone

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
| `WaitCmdEinvalAxes` + `parse_wait_cmd_einval_axes` / `format_wait_cmd_einval_axes_note` (`49feb5d2`) | Offline dig of WAIT_CMD EINVAL axes on lab classic START_NPU (skip-CONFIG_CU shape). Soft always `invent_pass=0 not_c_ready=1 not_ran_on_npu=1 axes_alone_not_pass=1 hypothesis_not_proven_root=1 product_pass_requires_live_decide=1`. Printed on soft classify when status/detail present (`wait_cmd_einval_axes=`). Axes alone ≠ Pass / ≠ proven root. |
| `MaterialResidualMatchInventory` + `classify_material_residual_match` / `format_material_residual_match_note` / `format_material_residual_match_checklist` (`49feb5d2`) | Residual-matched criteria vs public-npu2-small research-only inventory. Size class band `CTRLTEXT_SMALL_PUBLIC_NPU2_MAX_BYTES=1024` covers public peels 300 B + 660 B as research-only. **Hard lock:** `residual_matched_claim` never true offline. Soft always invent_pass=0 trio + `product_pass_requires_live_decide=1`. Soft classify prints inventory + checklist. |

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
`external_ctrltext_compare_dry_run`, `lab_export_fingerprint_feed`,
`wait_cmd_einval_axes`, `material_residual_match`.
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
- **No active unlock.** All prior one-shots **CONSUMED** (`2ae69036` flexml,
  `97f38307`, `11bc9016`, `3a70fabf`, `b17b0119`) — do not reopen L3 under them.

---

## See also

- Residual SSoT: [`../RESIDUAL.md`](../RESIDUAL.md)
- Field contract: [`XDNA2_XRT_CONTRACT_MLADF.md`](./XDNA2_XRT_CONTRACT_MLADF.md)
- HITL recipe (CONSUMED): [`HITL_C_APP_LOAD_RECIPE.md`](./HITL_C_APP_LOAD_RECIPE.md)
- Stage script: `tools/stage_aiebu_lab_instr.sh`
- Agent policy: `AGENTS.md` standing §6–9
