# XDNA2 XRT / shim contract (staged mladf matmul lab)

Short field table for **one** kernel:

`mladfmatmulbias_prmptflat_mladf_2x4x4_flat_a16fw4acc16f_1_1152_256_128`

**Not a novel.** Cited to local shim sources + live residuals (2026-07-16).  
Does **not** claim `ran_on_npu`.

## Sources

| Source | Path (this machine) |
|--------|---------------------|
| Shim exec_buf | `~/Projects/kernel/xdna-driver/test/shim_test/exec_buf.h` |
| Shim elf_io init | `…/test/shim_test/io.cpp` `elf_io_test_bo_set::init_cmd` |
| Pure-direct lab | `src/backends/amd_xdna2/{lab_io,txn,silicon_contract,direct}.rs` |
| Staged assets | `testdata/xdna2_cmd/silicon_lab/` (gitignored bins) |
| Offline oracle | `testdata/xdna2_cmd/gt_capture/` via `tools/hitl_b_gt_capture.sh` |

## Instruction blob

| Item | Contract |
|------|----------|
| Raw DD txn | 1688 B classic XAie v0.1 — **offline dump only** |
| EXEC payload | **aiebu ELF** (`lab_instr.elf`, ~32 KiB) with `.ctrltext` + `.rela.dyn` |
| Peel | Device BO holds **ctrltext** (1688 B), not whole ELF |
| Rela | 12 entries on staged ELF: IFM×8, WTS×2, OFM×2 (sym 1/2/3) |
| Patch rule (default) | **overwrite**: `ctrltext[offset] = base_paddr + addend` u64 LE |
| Patch rule (trial) | **add**: `placeholder + base + addend` (`XDNA2_LAB_RELA_MODE=add`; holes `0xb1`/`0x1e`/`0x159`) |
| Rela sym map (lab) | **Rela symbol index** bases `1→ifm`, `2→wts`, `3→ofm`, `4→ctrl` when present; staged uses **1–3 only** (no sym4 ctrl reloc) |
| Dynsym names | Staged ELF: all non-null names are **`"6"`** (opaque aiebu) — sizes 708/120/1380; **do not invent name→role map** |
| START_NPU size | Dual mode: `XDNA2_LAB_INSTR_SIZE_MODE=content\|bo` (default **content**; content clamps to BO len; BO still zero-pad page). Unknown token **refuse** |
| RELA / layout env | Fail-closed on typos (`overwrite\|add`, `count4\|xrt3`) |
| Post-patch walk | Classic v0.1: `txn_size=1688`, `num_ops=40` — offline GT+contract refuse; live apply walk-gates after rela write |
| Raw EXEC | Live → FW **`0x3000003` APP_LOAD_PDI_FAIL**, `ert_state=5(Error)`, OFM zeros |
| ELF+rela+bo=4096 | Still live **APP_LOAD_PDI_FAIL** (2026-07-16) — not raw-bin class |

Pure-direct: `patch_aiebu_elf_ctrltext_with_bos_ex` + `validate_patched_ctrltext_classic` + `classify_instruction_blob`.  
Apply scripts **refuse** silent raw `.bin` when ELF sibling exists (`lab_exec_once`, `hitl_c_pass_apply`).

## START_NPU / kernel args

From shim_test `exec_buf.h` + `elf_io_test_bo_set::init_cmd`:

| Field | Value |
|-------|--------|
| Opcode | `ERT_START_NPU` |
| `instruction_prop_count` | **0** (kernel args ≠ properties) |
| Instruction BO | bound as ctrl BO (`add_ctrl_bo`); type **Dev** (not Shmem) |
| Classic XRT layout (`xrt3`) | `u64 n_bos=3`, `u64 0`, `u32 0`, IFM, WTS, OFM, `u64 0`, `u64 0` → **15 u32** |
| Lab count4 | `n_bos=4` when CTRL paddr present; IFM/WTS/OFM/CTRL + trailing zero |
| Env | `XDNA2_LAB_KERNEL_ARG_LAYOUT=xrt3\|count4` (default count4) |
| Offline | Both layouts green in `silicon_contract` / GT `kernel_args.json` |

Live residual: both layouts can still yield `ert_state=Error` after aiebu rela — **not** solved offline alone.  
Env residual trials (`content` size + **`xrt3`**) **CLOSED** (same `APP_LOAD_PDI_FAIL`). Unpatched classic 1688 body may already **EXIST** (CLASS A body match via sibling). Public true post-patch dump research **closed offline** (no live `run_` instr BO API; registry = Transaction/cache only). **Highest-value next = gate-only maintain** `b6a66dee` (prior `dd35ae15`): `run_offline_gates` + L3=`N`; `C_UNLOCK=b17b0119` **CONSUMED** — further C needs **new** agent unlock ([HITL_C_APP_LOAD_RECIPE.md](./HITL_C_APP_LOAD_RECIPE.md)); unlock **≠** Pass / **≠** `ran_on_npu`. Unpatched CLASS A / mere `diff rc=0` / registry alone **≠** Pass.

## Control packet

| Item | Contract |
|------|----------|
| Size | 29664 B (`lab_ctrl.bin` == ELF `.ctrldata`) |
| Meta | `lab_ctrl_meta.json` `control_packet_patch_locations` |
| Operation | staged `read_add_write` **size=6** (tile offset + BO base; not full u64 wipe) |
| Pin | CTRL in `arg_bo_handles` when present; may be patch-only vs kernel-arg (count4 vs xrt3) |

## I/O BOs

| BO | Type | Size (staged) |
|----|------|----------------|
| IFM | Shmem | 2304 (bf16) |
| WTS | Shmem | 174080 (AMD pack; non-zero for Pass) |
| OFM | Shmem | 512 (bf16) |
| CTRL | Shmem | 29664 |
| Instr | **Dev** | ctrltext 1688 after peel/patch |

DevHeap UVA must be **64 MiB-aligned** (else map_host `0x4000003`).

## QoS / tiles

| Item | Contract |
|------|----------|
| QoS | Non-zero **gops + fps** (lab default 100 / 30). Zero → driver WARN `Invalid QoS gops 0 fps 0` (soft; max DPM). Env: `XDNA2_HWCTX_QOS_GOPS` / `_FPS` |
| Tiles | Staged mladf is 8×6 class; lab default **32** tiles when `LAB_OFM` + `LAB_PACK=mladf_a16w4` |

## Live residual map (honest)

| Symptom | Likely class |
|---------|----------------|
| Raw `.bin` EXEC | `0x3000003` `APP_LOAD_PDI_FAIL` |
| Unaligned DevHeap UVA | `0x4000003` `INVALID_PARAM` (map_host) |
| **2026-07-16 C apply** `aiebu_elf=1` `rela_count=12` `qos=100/30` `tiles=32` DevHeap aligned **`bo_size=4096`** + journal `Status 0x4000003` / `Failed cmd … 0x3000003` | **Still `APP_LOAD_PDI_FAIL`** after rela + page pad. Env residual trials **CLOSED**. Unpatched classic body may exist (CLASS A ≠ Pass). Post-patch dump research **closed offline**. **Next:** gate-only maintain `b6a66dee` (prior `dd35ae15`; `C_UNLOCK=b17b0119` **CONSUMED**; unlock ≠ Pass); optional recapture logs only. Ban env C. Unpatched A / registry alone ≠ Pass |
| `ert_state=Completed` + zero OFM | bind/patch residual after wait |

Decode: `0x3000003` = `AIE2_STATUS_APP_LOAD_PDI_FAIL`; `0x4000003` = `AIE2_STATUS_INVALID_PARAM` (overall chain status in job completion log).

## Vendor capture / parity (after env trials closed)

Env layout/size trials **failed** (`APP_LOAD_PDI_FAIL`). Classic unpatched 1688 body
may already exist (sibling recover → CLASS A body match). Public true post-patch dump
research **closed offline**. **Optional** recapture for logs only (not mandatory once
body green):

```bash
# Optional one L3 window (vendor NPU; not pure-direct Pass / not C unlock; not HV under CONSUMED):
# sudo sh -c 'echo 1 > /sys/module/amdxdna/parameters/exec_cmd_allow'
# bash tools/silicon_lab_operator.sh capture-instr
# sudo sh -c 'echo 0 > /sys/module/amdxdna/parameters/exec_cmd_allow'
# Offline (agent or you):
bash tools/diff_vendor_instr_parity.sh   # unpatched A = body match only ≠ C-ready
```

Writes `gt_capture/instr_vendor_post_patch.bin` + `vendor_paddrs.json` (schema v1,
`capture_mode`) + classic `instr_vendor_txn_bin.bin` when available.  
`capture_mode=txn_*_unpatched` → unpatched body class; `registry_get_instr_bo` =
Transaction/cache image only (not proven live-patched `run_` BO). In
`silicon_contract_check`: **miss** soft WARN (CONTRACT_OK possible — **≠ Pass**);
**match** OK as body class only when unpatched.
**No more env apply spam.** Operator **C2 CONSUMED** under **`C_UNLOCK=b17b0119`**
(unlock **≠** Pass; further C needs **new** unlock) — **not** mere unpatched `diff rc=0` / registry / parity alone. Gate-only maintain `b6a66dee` (prior `dd35ae15`).
Further C needs a **new agent unlock** (evidence may justify; never auto-unlocks).

See [HITL_BC.md](./HITL_BC.md), [SILICON_PATH_LAB.md](./SILICON_PATH_LAB.md), [RESIDUAL.md](../RESIDUAL.md).

## Offline gates

```bash
cargo test --features amd-xdna2 --lib silicon_contract
cargo test --features amd-xdna2 --lib patch_aiebu
bash tools/silicon_contract_check.sh
```

Never set `XDNA2_ALLOW_EXEC_CMD=1` in default tests. Never claim Pass from this doc alone.

**Highest-value next (single source of truth / gate-only `b6a66dee`):** maintain `run_offline_gates` green + L3=`N`; prefer stamp-only while A–O closed (prior `dd35ae15`); **ban** invent Track P+; `C_UNLOCK=b17b0119` **CONSUMED** — further C needs new agent unlock. Offline green ≠ C-ready ≠ Pass ≠ `ran_on_npu`.
