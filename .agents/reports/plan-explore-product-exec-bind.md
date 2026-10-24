# Product EXEC bind vs IRON vector_add (offline map)

**Honesty:** invent_pass=0 · Pass=**No** · unlock **none** (`C_UNLOCK=2c9c12e8` CONSUMED) · wrap ≠ EXEC ≠ Pass. Do not mint. Do not reopen CONSUMED. Do not ask the operator to re-run `just npu-iron`. Do not retag 300/660 as classic 1688.

---

## 1. How product EXEC binds instruction bytes today

There is **one** live product submit path: `timed_matmul_direct` in
`src/backends/amd_xdna2/direct.rs`. CLI `--iron` / `just npu-iron` never
reaches it.

| Step | What happens |
|------|----------------|
| Load | `load_instruction_blob_from_env`: `XDNA2_INSTRUCTION_BLOB` first, else `XDNA2_INSTRUCTION_HEX`. Full file kept (ELF must keep `.rela.dyn`). |
| Peel | `txn::instruction_payload_bytes`: aiebu ELF32 + `.ctrltext` → peel stream for START_NPU size; else raw bytes. Optional `XDNA2_INSTRUCTION_REQUIRE_TXN=1`. |
| Device | Instruction BO is **Dev** (heap). Raw txn is written immediately. ELF BO stays **zero** until in-process rela patch. |
| Patch | `patch_aiebu_elf_ctrltext_with_bos_ex` with **four** bases: IFM / WTS / OFM / CTRL. Same-shot paddrs optional (`XDNA2_LAB_SAME_SHOT_RELA`). |
| Walk | `validate_patched_ctrltext_classic`. Staged-mladf check (`1688` / 40 ops) only when `patched.len() == 1688`. A 300 B peel walks as generic classic. |
| I/O | Only when `XDNA2_LAB_OFM=1`. Pack is `LabPackMode::F32Ones` (default) or `MladfA16W4`. Kernel args are **3 or 4** (IFM/WTS/OFM[+CTRL]). Op name is always `"matmul"`. |
| Pass | `ran_on_npu` only via `decide_ran_on_npu` after ERT Completed + OFM allclose. Envelope / blob accept ≠ Pass. |

**Refuse raw (lab apply, not the Rust loader):**

- `scripts/lab_exec_once.sh` silicon-mladf `--apply`: auto-upgrade `.bin` → sibling `.elf`; else **die** (raw 1688 historically `APP_LOAD_PDI_FAIL`).
- `tools/hitl_c_pass_apply.sh` / preflight `--require-pass-assets`: require `lab_instr.elf` ELF magic.
- Rust `load_instruction_blob_from_path` **does** accept raw 300 B; detail then says `aiebu_elf=0 (raw txn — may APP_LOAD_PDI_FAIL)`.

Default lab blob is still `testdata/xdna2_cmd/silicon_lab/lab_instr.elf` (classic **1688** + rela×12). Nothing points that env at the IRON 300 B peel.

---

## 2. Job identity: landed vs not wired to EXEC

**Landed (classify / wrap / collect only):**

- `JobResidualIdentity::IronNpu2VectorAdd` (`iron_npu2_vector_add`).
- Kind-match **exact 300 B** only (`aaeb5b68cc298a45`). 660 is public-small but **not** this job. 1688 under this job remaps to Unknown / `iron_job_neq_classic_1688`.
- Always `residual_matched_claim=0`. Capture helper fail-closed (`tools/test_capture_iron_npu2_vector_add_fail_closed.sh`).
- Wrap: `dragon-npu run --iron vector_add` → `execution=external_runner`, `ran_on_npu=false`. Export is opt-in (`--iron-export` / `just npu-iron-export`).

**Not wired to EXEC:**

- No job-scoped timed path. No `XDNA2_INSTRUCTION_BLOB` set from `--iron`.
- No `LabPackMode` for 256-element vector add.
- This-run npy is wrap OFM parse only, not lab BO fill.
- Sibling xclbin / PDI are **not** attached on the product START_NPU path.

Bag files (research only):
`testdata/xdna2_cmd/external_known_good/mlir_aie_vector_scalar_add_npu2.*`

| Artifact | Size | Role |
|----------|------|------|
| `.ctrltext.bin` / `.insts.bin` | **300** | ClassicTxnV01 peel (1×6, 8 ops). Same sha16 as `working_stack_ctrltext.bin`. |
| `.insts.elf` | **992** | **aiebu** ELF: `.ctrltext` 300 + `.rela.dyn` (inventory: 2 relas). |
| `_core.elf` | 2468 | Peano compute kernel only. **Not** aiebu ctrltext. |
| `.xclbin` / `.pdi` | 8983 / 2528 | IRON/XRT load path. Not product `lab_instr.elf`. |

So: **not only raw 300 B**. A real aiebu instruction ELF exists. IRON live PASS used **xclbin + insts.bin** via pyxrt, not dragon-npu EXEC.

---

## 3. I/O / golden: 256-vector vs mladf 1688

| | Lab classic 1688 | IRON vector_add 300 |
|--|------------------|---------------------|
| Grid / ops | 8×6, 40 ops | 1×6, 8 ops |
| Tiles default | 32 when `LAB_PACK=mladf_a16w4` | Would need 1-col class, not 32 |
| Relas | 12 | 2 (four-base patch is a mismatch risk) |
| Host BOs | IFM 2304 bf16 + WTS 174080 + OFM 512 + optional CTRL 29664 | In + out (256 elems). No WTS. No mladf CTRL. |
| Golden | mladf bf16 / f32 ones matmul | Host `in + scalar` (IRON example). Wrap `shape [256]` is **not** OFM. |
| npy | lab files on disk | Wrap parses **`<f4` only**. Child `PASS!` often `output_len=0`. IRON buffers are typically **int32**, so this-run npy is **not** a drop-in lab IFM/OFM. |

**Can product attach this-run npy as EXEC I/O?** Not honestly today. Need a new i32 (or documented dtype) pack, two-buffer kernel-arg layout, and a host golden. Reusing wrap npy as mladf IFM/WTS/OFM would be a lie.

---

## 4. Smallest honest EXEC delta vs wrap-only

**Honest product EXEC (named offline delta):**

1. Job-owned blob bind: default or env to `mlir_aie_vector_scalar_add_npu2.insts.elf` (prefer ELF), peel 300, stamp `job=iron_npu2_vector_add`. Refuse treating it as classic 1688.
2. New pack: 256-element IFM+OFM, no WTS, no mladf golden. Host golden = input + scalar. `expect_staged=false`. Tiles for 1×6, not 32.
3. Rela: map **two** holes to in/out paddrs (do not pretend 12-slot mladf).
4. Soft always `invent_pass=0 not_ran_on_npu=1 residual_matched_claim=0`. Child `PASS!` must not set `ran_on_npu`.
5. Unit tests: 300 kind-match bind; 660/1688 refuse identity; wrap report stays `execution=external_runner`.

**Wrap-only (not this delta):** more catalog names, npy parse tweaks, re-running `just npu-iron`, collect/export, retag 300 as load-accepted.

---

## 5. Live / unlock

Default **no mint**. Offline bind+pack **≠** unlock.

Live product submit still needs a **new** host-minted unlock **and** a **new** named reason (example: “one product START_NPU of job `iron_npu2_vector_add` with 300 B aiebu + matching 256 I/O; Pass only by ERT Completed + OFM allclose”). Do **not** reopen `2c9c12e8`. Same-shot empty reconfirm stays frozen.

**Likely live result without more load-path work:** `wait_cmd_einval` (same residual as skip-CU / 2c9c12e8). IRON PASS used XRT xclbin+insts. Product START_NPU + lab I/O + optional wrong PDI can also `APP_LOAD_PDI_FAIL`. Prep is justified; Pass is not.

---

## 6. Frozen

- Empty same-shot reconfirm.
- Retag 300 / 660 as classic 1688 or 1688 load-accepted.
- Ask the operator to re-run `just npu-iron`.
- Claim Pass from wrap / catalog / kind-match / ELF land.

---

## Verdict

Current bind is **matmul-only** (`XDNA2_INSTRUCTION_BLOB` → Dev instr BO → 3/4 mladf-shaped kernel args). Second identity is **classify + wrap only**. Smallest honest next step is offline: bind the **992 B aiebu `insts.elf`** (300 B ClassicTxnV01) as `iron_npu2_vector_add` with a **256-element two-buffer pack + host golden**. That is not wrap, not 1688, and not Pass. Live stays gated on a new unlock + new reason; expect `wait_cmd_einval` until the load path matches what IRON already does with xclbin+insts.
