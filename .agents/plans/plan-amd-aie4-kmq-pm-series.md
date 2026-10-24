# Plan: AMD upstream "Kernel submission and PM for AIE4" series — what it means for dragon-npu

**Date:** 2026-10-23
**Kind:** Watch + offline-prep plan for an upstream kernel change. Not a spec. Specs live in `~/Projects/surmount/specs`; nothing here goes there.
**Honesty:** invent_pass=0 · Pass=**No** · ran_on_npu=false · unlock **none** (`2c9c12e8` CONSUMED; do not mint, do not reopen). `decide_ran_on_npu` (`src/backends/amd_xdna2/numerical.rs`) stays the **only** Pass gate. An upstream merge is not Pass. A kernel bump is not Pass.

---

## Kernel submission context (2026-10-24)

invent_pass=0. Pass=**No**. ran_on_npu=false. Unlock **none**. First unlock line `2c9c12e8` CONSUMED. Do not mint. Do not reopen CONSUMED. No new test. `decide_ran_on_npu` was not edited. This host stays PCI `1022:17f0` revision `0x20` on `dev_npu6_info`. The 17f0 recovery procedure stays diagnose, bridge reset of `0000:00:08.2`, and the rescue module.

**Series.** Lizhi Hou replied "Applied patches to drm-misc-next" on 2026-10-21 to V5 "accel/amdxdna: Kernel submission and PM for AIE4" ([14306](https://lkml.iu.edu/2610.2/14306.html), accessed: 2026-10-24). The cover says the series supports AMD AIE4 (NPU3), including classic and PF/VF for SR-IOV, with kernel-mode command submission (KMQ).

**Q4.** Stays open. The pull [607373](https://lists.freedesktop.org/archives/dri-devel/2026-October/607373.html) (accessed: 2026-10-24) requests tag `drm-misc-next-2026-10-23` and says "drm-misc-next for v7.5". That name is a requested tag. It is not a released mainline tag. No mainline version is claimed. Watch for a linux tag, or a drm-next tag after `drm-next-2026-10-21`, that contains `drm-misc-next-2026-10-23`.

**KMQ ops, not `aie2_ops`.** Patch 16 wires the AIE4 submit callback in `drivers/accel/amdxdna/aie4_pci.c` ([07861](https://lkml.iu.edu/2610.2/07861.html), accessed: 2026-10-24):

```
const struct amdxdna_dev_ops aie4_classic_ops = {
	.hwctx_fini		= aie4_hwctx_fini,
	.cmd_submit		= aie4_cmd_submit,
```

The same patch also sets `.cmd_submit = aie4_cmd_submit` on `aie4_vf_ops` in that file. Fences in that patch are `dma_fence_signal` and `dma_fence_init(..., dma_fence_context_alloc(1), 0)`. Do not treat `&aie2_ops` as this kernel-mode queue.

Patch 5 binds that ops struct to NPU3 classic only ([07943](https://lkml.iu.edu/2610.2/07943.html), accessed: 2026-10-24). `dev_npu3_classic_info` in `drivers/accel/amdxdna/npu3_regs.c` has `.ops = &aie4_classic_ops`. The info rows are `{ 0x17f1, 0x10, &dev_npu3_classic_info }` and `{ 0x17f1, 0x13, &dev_npu3_classic_info }`. `PCI_VENDOR_ID_AMD` is `0x1022`. The unchanged `0x17f0` rows stay `{ 0x17f0, 0x10, &dev_npu4_info }`, `{ 0x17f0, 0x11, &dev_npu5_info }`, and `{ 0x17f0, 0x20, &dev_npu6_info }`. This host keeps the AIE2 submit path.

**Disagreement on the device-type enum.** The cover calls the feature KMQ. The same patch 5 struct prints `.device_type = AMDXDNA_DEV_TYPE_UMQ` on `dev_npu3_classic_info`. The submit callback above is still `aie4_cmd_submit`. Those two names are both in the pages that were opened. The enum word UMQ does not move this host onto that ops struct.

**17f0 runtime hooks only.** Patch 19 adds `.runtime_resume = aie2_hw_resume` and `.runtime_suspend = aie2_hw_suspend` on `aie2_ops` ([08350](https://lkml.iu.edu/2610.2/08350.html), accessed: 2026-10-24). Those point at the existing `aie2_hw_resume` and `aie2_hw_suspend`. Linux master `npu6_regs.c`, fetched 2026-10-24, still ends `dev_npu6_info` with `.ops = &aie2_ops` ([npu6_regs.c](https://raw.githubusercontent.com/torvalds/linux/master/drivers/accel/amdxdna/npu6_regs.c), accessed: 2026-10-24). The V5 cover diffstat does not list `npu6_regs.c`. That `.ops` line is the AIE2 table. It is not `aie4_classic_ops`. The same master file also prints `.device_type = AMDXDNA_DEV_TYPE_KMQ` on `dev_npu6_info`. That enum value is the pre-existing AIE2 device type. It is not the AIE4 `aie4_cmd_submit` callback.

**Suspend.** Patch 18 ([08021](https://lkml.iu.edu/2610.2/08021.html), accessed: 2026-10-24) adds `aie4_classic_suspend` and sets `.suspend = pm_ptr(aie4_classic_suspend)` and `.resume = pm_ptr(aie4_classic_resume)` on `aie4_classic_ops`. The commit text says `aie4_hwctx_suspend_all()` destroys or drains contexts and waits for in-flight jobs, and `aie4_hwctx_resume_all()` recreates firmware contexts and kicks doorbells through `aie4_hwctx_resume_jobs()`. **Disagreement:** that page does not use the words job-save or firmware-restore. The AIE4 behavior that is on the page is the suspend and resume pair above. It is not wired onto `aie2_ops`.

**Status, clock, and SR-IOV.** The cover lists "Add clock, DPM frequency, and resource info queries for AIE4" and "Add get and set power_mode for AIE4" as AIE4 patches. Patch 18 restores SR-IOV virtual functions on PF resume through `aie4_restore_sriov()`. No AIE2 clock-query hook and no AIE2 SR-IOV hook were on the pages opened. For `1022:17f0` those queries are N/A. The only AIE2 hook in patch 19 is the runtime suspend and resume pair above.

**Shared code stays on the D-AIE4-3 list.** Do not delete that list. Patch 15 moves `populate_range` into common GEM code and says "No functional change for AIE2". Patch 02 removes doorbell mmap and points common `.mmap` at `drm_gem_mmap`, which applies to every `amdxdna_fops` device, including `0x17f0`. Both stay a re-check. They were not run.

**Horizon re-check, list only.** Not run: `power/runtime_status`, `autosuspend_delay_ms`, first-submit latency after idle, a submit across suspend/resume, and whether a bridge reset of `0000:00:08.2` is still needed after resume.

**This machine's 7.2.11 tree.** `uname -r` printed `7.2.11-zen1-1-zen`. The build directory `drivers/accel/amdxdna` has `Kconfig` only, not the C sources. Strings from the installed `amdxdna.ko.zst` (decompressed, no modprobe) include `aie2_ops` and `aie2_cmd_submit`. The same string search did not show `aie4_cmd_submit` or `aie4_classic`. Older `aie4_` names are present (`aie4_hwctx_create`, `aie4_suspend_fw`, `aie4_doorbell_mmap`). Absence of the V5 submit symbol is from that string search, not from the uname string alone.

---

## Classic 1688 origin (2026-10-23)

invent_pass=0. Pass=**No**. ran_on_npu=false. Unlock **none**. First unlock line `2c9c12e8` CONSUMED. Do not mint. Do not reopen CONSUMED. Older Wave 2 sentences below stay. No new test. `decide_ran_on_npu` was not edited.

The classic 1688-byte body in this tree is the unpatched DynamicDispatch dump of `mladfmatmulbias_prmptflat_mladf_2x4x4_flat_a16fw4acc16f_1_1152_256_128` (`docs/BUILD_NPU_MATMUL_PROGRAM.md` lines 18-41). `STAGED_MLADF_CLASSIC_INSTR_BYTES` is 1688 at `src/backends/amd_xdna2/txn.rs` line 30. `MATERIAL_EXEC_LAB_CLASSIC_BYTES` aliases it at `src/backends/amd_xdna2/silicon_contract.rs` line 14525. `LAB_CLASSIC_UNPATCHED_CTRLTEXT_SHA16` is `3c5ffca750922d3b` at line 16485. Measured this turn: `testdata/xdna2_cmd/silicon_lab/lab_instr.bin` is 1688 bytes, sha256 `3c5ffca750922d3b6b1dec66b792273e46504cc1d3de6b5b96e12ac5c8197368`. That file is the unpatched lab body. It is not residual-matched. `classify_material_residual_match` sets `residual_matched_claim` false on every return (line 13907). IRON and mlir-aie tags stay research-only (lines 13935-13948). Public true post-patch dump research is closed offline (`docs/XDNA2_XRT_CONTRACT_MLADF.md` lines 56 and 99-101). Research ctrltext peels measured this turn stay rejected: 300, 660, 2592, and 4512 bytes under `testdata/xdna2_cmd/external_known_good/`. Do not retag them as load-accepted.

No residual-matched buffer was emitted. No submission command was written. The single file only the Operator holds is a residual-matched classic 1688-byte ctrltext to place under `testdata/xdna2_cmd/external_known_good/`. Do not ask the Operator to re-run `just npu-iron`.

**Q4.** Stays open. Watch for a linux tag, or a drm-next tag after `drm-next-2026-10-21`, that contains `drm-misc-next-2026-10-23`.

**Gate.** `/tmp/grok-goal-5f7882d2f7dc/implementer/gate.log`. nproc printed 16. `CARGO_BUILD_JOBS=8`. `nice -n 19`. `PATH="/usr/bin:$PATH"`. Four exits 0, 0, 0, 0. Clippy default and clippy `--features amd-xdna2` (lib, bins, tests, examples, benches, `-D warnings`) each exited 0. `cargo test --lib` 168 passed, 0 failed, 0 ignored. `cargo test --features amd-xdna2 --lib` 737 passed, 0 failed, 1 ignored.

---

## Ops pointer and Q3 (2026-10-23)

invent_pass=0. Pass=**No**. ran_on_npu=false. Unlock **none**. `C_UNLOCK=2c9c12e8` CONSUMED. Do not mint. Do not reopen CONSUMED. Older Wave 2 text below stays, including the sentence that the ops pointer page was missing. This block is the newer read. No new test. `decide_ran_on_npu` was not edited.

**Q2. Yes.** `dev_npu6_info.ops` is `&aie2_ops`. Source is linux master `drivers/accel/amdxdna/npu6_regs.c`, fetched 2026-10-23 from https://raw.githubusercontent.com/torvalds/linux/master/drivers/accel/amdxdna/npu6_regs.c (accessed 2026-10-23). That file is linux master on that date. It is not the drm-misc-next tag. The V5 series does not edit `npu6_regs.c`. The cover index https://lkml.iu.edu/2610.2/14306.html (accessed 2026-10-23) lists `npu4_regs.c` and does not list `npu6_regs.c`. V5 patch 19 (https://lkml.iu.edu/2610.2/08350.html, accessed 2026-10-23) adds `.runtime_resume = aie2_hw_resume` and `.runtime_suspend = aie2_hw_suspend` on `aie2_ops` in `aie2_pci.c`. Because `dev_npu6_info.ops` is `&aie2_ops`, those hooks are on the ops table this host uses (`1022:17f0` rev `0x20`). An ops pointer is not Pass.

**Q3.** Three page reads, 2026-10-23.

- V5 patch 15, https://lkml.iu.edu/2610.2/07837.html and https://lists.freedesktop.org/archives/dri-devel/2026-October/606189.html (both accessed 2026-10-23). Quoted sentence: "No functional change for AIE2." The same paragraph says the move is `aie2_populate_range()` from `aie2_ctx.c` into `amdxdna_gem.c` as `amdxdna_populate_range()`.
- The body after the signature was read on those pages. It was not truncated. Deleted opening: `static int aie2_populate_range(struct amdxdna_gem_obj *abo)` then `struct amdxdna_dev *xdna = to_xdna_dev(to_gobj(abo)->dev);`. Added opening: `int amdxdna_populate_range(struct amdxdna_gem_obj *abo)` then the same first statement. Both sides call `hmm_range_fault_unlocked_timeout(&mapp->range, timeout);` and both end `return ret == -EBUSY ? -ETIME : ret;`. The call site changes from `aie2_populate_range(abo)` to `amdxdna_populate_range(abo)`. The statements inside the braces match. The body moved and the name changed. The body was not rewritten.
- V5 patch 02, https://lists.freedesktop.org/archives/dri-devel/2026-October/606181.html (accessed 2026-10-23). Quoted: "Remove .mmap handler and use standard drm_gem_mmap." and "Set hwctx->doorbell_offset to AMDXDNA_INVALID_DOORBELL_OFFSET on context creation so userspace does not receive a valid-looking BAR offset." Hunks: `hwctx->doorbell_offset = AMDXDNA_INVALID_DOORBELL_OFFSET` replaces `resp.doorbell_offset`, and `amdxdna_fops.mmap` becomes `drm_gem_mmap` instead of `amdxdna_drm_gem_mmap`.

**Q4.** Stays open. https://lists.freedesktop.org/archives/dri-devel/2026-October/607373.html (accessed 2026-10-23) requests tag `drm-misc-next-2026-10-23`. That is not a released mainline tag.

**Next offline delta (named only, not started).** One name: a residual-matched classic 1688-byte capture under `testdata/xdna2_cmd/external_known_good/`. Who acts: the person who holds that file. Do not ask the operator to re-run `just npu-iron`. Do not mint.

## Wave 2 (2026-10-23)

invent_pass=0. Pass=**No**. ran_on_npu=false. Unlock **none**. `C_UNLOCK=2c9c12e8` CONSUMED. Do not mint. Do not reopen CONSUMED. Older Q2, Q3, and Q4 text below stays. This block is the newer read.

**Q2.** Patch 19 ([08350](https://lkml.iu.edu/2610.2/08350.html), accessed 2026-10-23) adds `.runtime_resume = aie2_hw_resume` and `.runtime_suspend = aie2_hw_suspend` on `aie2_ops` in `drivers/accel/amdxdna/aie2_pci.c`. Function names: `aie2_hw_resume`, `aie2_hw_suspend`. Same patch adds `amdxdna_pm_runtime_suspend` and `amdxdna_pm_runtime_resume` in `drivers/accel/amdxdna/amdxdna_pm.c`, and the `runtime_suspend` / `runtime_resume` fields on `struct amdxdna_dev_ops`. Patch 5 ([07943](https://lkml.iu.edu/2610.2/07943.html), accessed 2026-10-23) keeps `{ 0x17f0, 0x20, &dev_npu6_info }`. The ops pointer page is missing. V5 08 `npu4_regs.c` ([606195](https://lists.freedesktop.org/archives/dri-devel/2026-October/606195.html), accessed 2026-10-23) does not set `dev_npu6_info.ops`. Do not guess `&aie2_ops`. The file still needed is the one that defines `dev_npu6_info`. No new test.

**Q3.** Plan sentence already in this file: the D-AIE4-3 re-check list is a read of the patches, not a measurement on a running kernel. This wave cites the cover ([14306](https://lkml.iu.edu/2610.2/14306.html), accessed 2026-10-23): the uninterruptible HMM invalidate wait move of `aie2_hmm_invalidate()` into common GEM code, and the `aie2_populate_range()` move into common GEM code. That cover text has no `drm_mm_init` sentence. V5 08 cites `aie2_pm.c | 10 +-` and moves `npuclk_freq`, `hclk_freq`, `max_tops`, and `curr_tops` onto `struct aie_device`. Still open, pages not opened this wave: the V5 15 "No functional change for AIE2" line and the `populate_range` body after the signature, and the V5 02 doorbell / `drm_gem_mmap` hunk.

**Q4.** Open. Pull [607373](https://lists.freedesktop.org/archives/dri-devel/2026-October/607373.html) (accessed 2026-10-23) requests tag `drm-misc-next-2026-10-23` and says "drm-misc-next for v7.5". That is a requested tag, not a released mainline tag. v7.5 is not recorded as a found release. The drm pull [607011](https://lists.freedesktop.org/archives/dri-devel/2026-October/607011.html) (accessed 2026-10-23) is tag `drm-next-2026-10-21` and does not name `drm-misc-next-2026-10-23`. Watch signal: a linux tag, or a drm-next tag after `drm-next-2026-10-21`, that contains `drm-misc-next-2026-10-23`.

**Horizon list (not run).** After the first kernel that carries the series: HMM wait (uninterruptible, `MAX_SCHEDULE_TIMEOUT`); `populate_range` move; doorbell and common `.mmap`; runtime PM hooks on `aie2_ops`; `aie2_pm.c` field move onto `struct aie_device`; probe; recovery already in `docs/XDNA2_RECOVERY_ENGINEERING.md` (diagnose, bridge reset of `0000:00:08.2`, rescue module). No modprobe, insmod, rmmod, reboot, suspend, sudo, EXEC_CMD, or `/dev/accel`.

**Next offline delta.** The operator places a residual-matched classic 1688 under `testdata/xdna2_cmd/external_known_good/`. Smaller real step: read the missing `dev_npu6_info` ops page. No empty reconfirm. No `just npu-iron`. No unlock mint. Do not edit `decide_ran_on_npu`.

**Gate.** Log `call-4f6b0998-5910-4941-afc8-b57ababb657d-21.log`. nproc printed 16. `JOBS=8`. `nice` and `PATH` were not printed in that log. Exits 0, 0, 0, 0. Default lib 168 passed. Feature lib 737 passed, 1 ignored.

---

## Plain English summary

AMD (David / Yidong Zhang) posted a 19-patch series, "accel/amdxdna: Kernel submission and PM for AIE4". It adds kernel-mode command submission (KMQ) for AIE4, fences, BO reservation locking, clock/DPM/telemetry queries, system + runtime suspend/resume, and SR-IOV PF/VF.

**Big finding: this series targets AIE4 = NPU3, not this host's Strix NPU.** Horizon's NPU is `1022:17f0` rev 0x20, which the driver (and this repo) treat as **npu4 / AIE2-family (AIE2P)**. The cover letter says it was tested on "AMD AIE4/NPU3 hardware" and "Confirmed no regression on existing AIE2 devices (NPU1/NPU4)". So the new KMQ/PM/telemetry code paths do **not** run on horizon. Phoronix's "NPU3 in Ryzen AI 300" framing is wrong for 17f0 (see Evidence).

**Second finding (newer than the parent's note):** Lizhi Hou replied to V5 on 2026-10-21: "Applied patches to drm-misc-next". So it is **merged into drm-misc-next**, not yet in a Linus release. Expected path: drm-next → next merge window → a mainline release → linux-zen.

What still touches 17f0: a few shared/AIE2 files change (`aie2_ctx.c`, `aie2_pci.c/h`, `aie2_pm.c`, `npu4_regs.c`, common GEM: HMM invalidate wait made uninterruptible and `populate_range` made common, "Remove mmap for doorbell"). Those are regression surface for us, not new features.

---

## Evidence (Strix coverage)

| Source | What it says |
|--------|--------------|
| V5 cover letter, https://lkml.iu.edu/2610.2/14306.html | "support AMD AIE4 (NPU3) platforms"; testing "on AMD AIE4/NPU3 hardware"; "no regression on existing AIE2 devices (NPU1/NPU4)"; patch 5 "Add NPU3 classic device support"; file `npu3_regs.c` +105 vs `npu4_regs.c` +30/- (shared refactor) |
| Same page, reply by Lizhi Hou, 2026-10-21 | "Applied patches to drm-misc-next" |
| Host, 2026-10-23 | `lspci -nn`: `c6:00.1 ... Strix/Krackan/Strix Halo Neural Processing Unit [1022:17f0] (rev 20)`; kernel `7.2.11-zen1-1-zen` |
| Repo `docs/BUILD_NPU_MATMUL_PROGRAM.md` | classic header "device gen 4 (Strix)" |
| Repo `docs/XDNA2_CMD_PAYLOAD.md` | `ERT_START_DPU` is "AIE4/VE2; not AIE2 cmdlist" |
| Repo `docs/upstream/LAB_EVIDENCE.md` | firmware `amdnpu/17f0_10/npu_7.sbin` (npu4 family) |
| Phoronix 2026-09-26, https://www.phoronix.com/news/AMDXDNA-NPU3-Enhancements | calls AIE4 "NPU3 in Ryzen AI 300" — conflicts with the driver's own npu4 = 17f0 naming |

**Q1 (amended 2026-10-23 from the patch 5 diff, not from a guessed id):** The series does not cover Strix. File `drivers/accel/amdxdna/amdxdna_pci_drv.c`. Unchanged device-info lines: `{ 0x17f0, 0x10, &dev_npu4_info }`, `{ 0x17f0, 0x11, &dev_npu5_info }`, `{ 0x17f0, 0x20, &dev_npu6_info }`. Unchanged match line: `{ PCI_DEVICE(PCI_VENDOR_ID_AMD, 0x17f0) }`. Added match line: `{ PCI_DEVICE(PCI_VENDOR_ID_AMD, 0x17f1) }`. Added info rows: `{ 0x17f1, 0x10, &dev_npu3_classic_info }` and `{ 0x17f1, 0x13, &dev_npu3_classic_info }`. That struct is `const struct amdxdna_dev_info dev_npu3_classic_info` in `drivers/accel/amdxdna/npu3_regs.c` (`.ops = &aie4_classic_ops`). This host is `1022:17f0` rev `0x20`, so it stays `dev_npu6_info`. Rev `0x10` is npu4. Rev `0x11` is npu5. Patch 5 is not one new row: it also adds PF/VF rev `0x13` rows and removes unreleased `0x1B0B` / `0x1B0C`. `0x1502` stays in the match list with no info struct in the hunk. Cover letter sentences read on [14306](https://lkml.iu.edu/2610.2/14306.html) (accessed: 2026-10-23): "This patch series extends the amdxdna accelerator driver to support AMD AIE4 (NPU3) platforms, including classic and PF/VF for SR-IOV with kernel-mode command submission (KMQ), system power management, and runtime power management for the classic device." "Tested on AMD AIE4/NPU3 hardware in both classic and SR-IOV (PF/VF) modes." "Confirmed no regression on existing AIE2 devices (NPU1/NPU4)." Patch body: [07943](https://lkml.iu.edu/2610.2/07943.html) (accessed: 2026-10-23). Quote home: `docs/upstream/AIE4_NPU3_PCI_IDS.md`. The patch does not name a retail SKU.

---

## Interaction with standing rules / current deltas

- Latest residual: implement loop `5dca83c3` CLOSED; default `just npu-iron` = run+verify wrap; invent_pass=0 not_c_ready=1 not_ran_on_npu=1.
- **Who acts next stays the same:** human residual-matched classic 1688 when held; agent only on a **new named offline delta**. Do **not** ask the operator to re-run `just npu-iron`. Agent does not mint.
- The upstream series is a **named external input**, not itself a delta. Each item below becomes a delta only when a file lands offline.
- Frozen stays frozen: empty same-shot reconfirm, reopening CONSUMED, minting, sticky ALLOW.
- No EXEC_CMD / device submit, no driver load/unload, no reboot is part of this plan.

---

## What changes for `src/backends/amd_xdna2/` (if it ever applies)

| Area | Today on 17f0 (AIE2 KMQ, `device_type=0`) | AIE4 series | Effect for us |
|------|------|------|------|
| Submission | Kernel already owns submit (`XDNA2_NEVER_AGAIN.md`: KMQ is queue ownership) via ERT cmdlist / EXEC_CMD | AIE4 moves from userspace doorbell mmap to KMQ by default; `ERT_START_DPU` packets | None on 17f0. `build_ert_start_dpu_cmd_bo` only matters on AIE4 hosts |
| Fences | Wait ioctl on cmd BO state | dma_resv job fences + job timeout stopping the hwctx | Waits end in finite time on AIE4; our finite-wait rule unchanged |
| Suspend/resume | Recovery = `XDNA2_RECOVERY_ENGINEERING.md` (diagnose, bridge reset `0000:00:08.2`, rescue kmod) | Recreate fw contexts, re-ring doorbells, wait resumes device, `-ENODEV` on failed resume | AIE4-only. Our recovery doc stays the 17f0 tool. Do not treat it as a fix for the cmdlist NULL wedge |
| Telemetry / probe | GET_INFO works (FW, power, TOPS) | AIE version, clock, DPM freq, resource queries for AIE4 | Probe may report new fields only on AIE4 |
| Shared GEM | — | HMM invalidate wait uninterruptible; `populate_range` common | Possible behavior change on 17f0 BO paths: regression watch |

---

## Named offline deltas (doable now, no device)

1. **D-AIE4-1 Coverage table:** CLOSED 2026-10-23, amended the same day. The series does not cover Strix. Unchanged `{ 0x17f0, 0x20, &dev_npu6_info }` in `drivers/accel/amdxdna/amdxdna_pci_drv.c`. Added `{ PCI_DEVICE(PCI_VENDOR_ID_AMD, 0x17f1) }` maps to `&dev_npu3_classic_info` in `npu3_regs.c`. Rev `0x10` of `0x17f0` stays npu4. Rev `0x11` stays npu5. More than one row was added (classic `0x17f1` revs `0x10` and `0x13`, plus PF/VF rev `0x13`). Recorded in `docs/upstream/AIE4_NPU3_PCI_IDS.md`. invent_pass=0. Pass=No. ran_on_npu=false. Unlock none. `C_UNLOCK=2c9c12e8` CONSUMED.
2. **D-AIE4-2 Probe classification:** CLOSED 2026-10-23. Pure unit test `backends::tests::classify_amd_npu_pci_id_aie2_rows_and_aie4_fail_closed`. `1022:17f0` rev `0x10` is `DevNpu4`, rev `0x11` is `DevNpu5`, rev `0x20` is `DevNpu6` (not every `17f0` is npu4). AIE4 ids `0x17f1` / `0x17f2` / `0x17f3` at revs `0x10` and `0x13` fail closed with the sentence "AIE4: not supported by this crate's AIE2 cmdlist path". No new Pass field. `cmdlist_class_from_sysfs` parses sysfs text and does not open `/dev/accel`. Green command: `PATH="/usr/bin:$PATH" nice -n 19 cargo test --features amd-xdna2 --lib backends::tests::classify_amd_npu_pci_id_aie2_rows_and_aie4_fail_closed -- --exact` → `ok`, 1 passed, 737 filtered out, EXIT:0. The earlier red was a compile failure because the classifier was absent. That red log was not captured, so no red exit code is recorded. invent_pass=0. Pass=No. ran_on_npu=false. Unlock none. `C_UNLOCK=2c9c12e8` CONSUMED.
3. **D-AIE4-3 Shared-code regression list:** CLOSED 2026-10-23. Offline read of the V5 hunks. No device open, no submit. Re-check these after the kernel that contains drm-misc-next from 2026-10-21 is actually running. invent_pass=0. Pass=No. ran_on_npu=false. Unlock none. `C_UNLOCK=2c9c12e8` CONSUMED.

   1. **HMM wait (V5 14).** Pages read: [08040](https://lkml.iu.edu/2610.2/08040.html) and [606200](https://lists.freedesktop.org/archives/dri-devel/2026-October/606200.html) (accessed 2026-10-23). `aie2_hmm_invalidate()` moves into `amdxdna_hmm_invalidate()` in `amdxdna_gem.c`. The `.hmm_invalidate` callback leaves `aie2_ops` in `aie2_pci.c`. The new wait is `dma_resv_wait_timeout(to_gobj(abo)->resv, DMA_RESV_USAGE_BOOKKEEP, false, MAX_SCHEDULE_TIMEOUT)`. Comment read on that page: "The pages must not be released while the device may still access them, so this wait must be neither interruptible nor time limited." After the kernel bump, a stuck BO fence can block page release with no signal and no timeout. A userspace WAIT_CMD timeout does not cover this kernel wait. The removed callback is the AIE2 one, so the wait is on the AIE2 path. An older V2 copy used an interruptible wait. The merged text that was read uses `false` and `MAX_SCHEDULE_TIMEOUT`.

   2. **populate_range (V5 15).** Pages read: [07837](https://lkml.iu.edu/2610.2/07837.html) and [606189](https://lists.freedesktop.org/archives/dri-devel/2026-October/606189.html) (accessed 2026-10-23). Commit message read: "Move aie2_populate_range() from aie2_ctx.c into amdxdna_gem.c as amdxdna_populate_range() ... No functional change for AIE2." The call site becomes `amdxdna_populate_range(abo)`. The function body after the signature was truncated on the pages read, so the body is not quoted here. Re-check, once that kernel is present, that the moved function still faults the user mapping back in before the job fence when `map_invalid` is set. The 64 MiB DevHeap UVA alignment is `mmap_dev_heap_drm_aligned` in `src/backends/amd_xdna2/direct.rs` (align `0x4000000`), documented in `docs/SILICON_PATH_LAB.md`. That alignment is a userspace mmap choice. The V5 15 move is an HMM range fault. `lab_io.rs` does not hold `mmap_dev_heap_drm_aligned`.

   3. **Doorbell (V5 02).** Page read: [606181](https://lists.freedesktop.org/archives/dri-devel/2026-October/606181.html) (accessed 2026-10-23). In `aie4_hwctx_create` the assignment becomes `hwctx->doorbell_offset = AMDXDNA_INVALID_DOORBELL_OFFSET`. The patch deletes `aie4_hwctx_valid_doorbell` and `aie4_doorbell_mmap` (the BAR `io_remap_pfn_range` path) and removes `.mmap = aie4_doorbell_mmap` from `aie4_vf_ops`. It removes `int (*mmap)(...)` from `struct amdxdna_dev_ops`. In `amdxdna_pci_drv.c` it deletes `amdxdna_drm_gem_mmap` and sets `.mmap = drm_gem_mmap` on the common file operations. That file-operations change applies to every device that uses `amdxdna_fops`, including AIE2 `0x17f0`. The patch does not edit `aie2_ctx.c`. This crate sends `umq_doorbell` only as a CREATE_HWCTX request field (`XDNA2_HWCTX_UMQ_DOORBELL`, default 0) in `src/backends/amd_xdna2/direct.rs` (field at the request struct, filled near the CREATE_HWCTX pack). It does not mmap a doorbell BAR. Re-check for `17f0`: this crate does not use the low page-offset DRM fd mmap that the common file-operations change sends to `drm_gem_mmap`. The invalid doorbell offset assignment sits inside `aie4_hwctx_create`.

   4. **Runtime PM (V5 19).** Page read: [08350](https://lkml.iu.edu/2610.2/08350.html) (accessed 2026-10-23). The file list includes `aie2_pci.c`. It does not list `aie2_pm.c`. The hunk adds `.runtime_resume = aie2_hw_resume` and `.runtime_suspend = aie2_hw_suspend` on `const struct amdxdna_dev_ops aie2_ops`. AIE4 classic gets `.runtime_resume = pm_ptr(aie4_classic_resume)` and `.runtime_suspend = pm_ptr(aie4_classic_suspend)`. Common `amdxdna_pm_ops` becomes `RUNTIME_PM_OPS(amdxdna_pm_runtime_suspend, amdxdna_pm_runtime_resume, NULL)`, and `.driver.pm = pm_ptr(&amdxdna_pm_ops)`. `amdxdna_pm_runtime_suspend` defaults `ret = -EBUSY` and calls `ops->runtime_suspend` when that hook is present. `amdxdna_pm_runtime_resume` defaults `ret = -EOPNOTSUPP` and calls `ops->runtime_resume` when that hook is present. Commit text read: the feature is for the AIE4 classic device, and "Runtime suspend returns -EBUSY for a device without runtime_suspend, so the PM core keeps it active." `aie2_ops` now has the hooks, so the AIE2 path calls the existing `aie2_hw_suspend` and `aie2_hw_resume`. The `amdxdna_ctx.c` body was truncated after the diff header on the page read. The `-EAGAIN` wait-ioctl retry sentence is in the commit message. The wait-path source was not read.

   5. **`aie2_pm.c` (V5 08).** Page read: [606195](https://lists.freedesktop.org/archives/dri-devel/2026-October/606195.html) (accessed 2026-10-23). Subject: "accel/amdxdna: Add clock, DPM frequency, and resource info queries for AIE4". Diffstat line: `drivers/accel/amdxdna/aie2_pm.c | 10 +-`. That matches the cover cumulative `aie2_pm.c | 10 +-` on [14306](https://lkml.iu.edu/2610.2/14306.html). Index `4fe6030d2c41..f4ced7b67c25`. In `aie2_pm_set_clk_gating`, `ndev->clk_gating = val` becomes `ndev->aie.clk_gating = val`. In `aie2_pm_set_dpm`, `ndev->priv->hw_ops->set_dpm(ndev, dpm_level)` becomes `set_dpm(&ndev->aie, dpm_level)`. Resume, init, and the clk-gating call site take `&ndev->aie` and `ndev->aie.clk_gating`. The same patch moves `clk_gating`, `npuclk_freq`, `hclk_freq`, `max_tops`, and `curr_tops` from `amdxdna_dev_hdl` onto `struct aie_device`, and changes `set_dpm` to `(struct aie_device *aie, u32)`. This is a kernel-internal field move. These ten lines do not change heap page size or BO mmap. This crate does not call `set_dpm`. Userspace GET_INFO keeps the ioctl structs this hunk leaves alone.

   6. **Heap `drm_mm_init` sentence.** The V5 cover "Changes in v5" that was read on [14306](https://lkml.iu.edu/2610.2/14306.html) (accessed 2026-10-23) is three bullets: rebase HMM into common GEM, rebase `populate_range` into common GEM, and "Move runtime PM reference calls to the last patch, so each patch is bisectable on its own." No `dev_heap_max_size` sentence and no `drm_mm_init` sentence is in that cover letter. Record: not found in this series. A separate patch, "accel/amdxdna: Reject device heap BO when dev_mem_size is not set" (Lizhi Hou, applied to drm-misc-next on 2026-10-12, [09457](https://lkml.iu.edu/hypermail/linux/kernel/2610.1/09457.html)), returns `-EOPNOTSUPP` when `dev_mem_size` is 0. Sentence read there: "NPU3 devices do not set dev_mem_size, since they do not use a device heap." That patch is adjacent history. It is not a V5 hunk.
4. **D-AIE4-4 Recovery doc note:** CLOSED 2026-10-23. Paragraph "AIE4 power management and 17f0 recovery" is in `docs/XDNA2_RECOVERY_ENGINEERING.md` under the driver-fork roadmap. The 17f0 procedure stays diagnose, bridge reset of `0000:00:08.2`, and the rescue module. V5 19 runtime suspend and resume, including the new `aie2_ops` hooks to `aie2_hw_suspend` and `aie2_hw_resume`, is power management. The AIE4 job-timeout is recorded as a pattern for the `open` timeout / killable-wait row. The V5 14 unlimited HMM wait is named as the D-AIE4-3 re-check, separate from this recovery engine. invent_pass=0. Pass=No. ran_on_npu=false. Unlock none. `C_UNLOCK=2c9c12e8` CONSUMED.
5. **D-AIE4-5 Colibri contract note:** CLOSED 2026-10-23. `docs/COLIBRI_ADAPTER.md` documents optional `npu_generation`. Schema stays 1. `TimedOpResult::to_json_value()` does not emit the field in this delta. When a later producer sets it: `npu4` is `1022:17f0` rev `0x10`, `npu5` is rev `0x11`, `npu6` is rev `0x20`, `npu3` is AIE4 classic `0x17f1`. Colibrì may refuse `npu3`. The field does not set `ran_on_npu`. The colibrì tree was not edited. invent_pass=0. Pass=No. ran_on_npu=false. Unlock none. `C_UNLOCK=2c9c12e8` CONSUMED.

Each is its own turn; each records invent_pass=0 / Pass=No in the residual.

**Offline gate (once, after all five deltas):** GREEN 2026-10-23 on the tree that includes the three clippy fixes in `src/backends/mod.rs`. Commands used `PATH="/usr/bin:$PATH" CARGO_BUILD_JOBS=8 nice -n 19`. `cargo test --lib` EXIT 0 (168 passed, 0 failed, 0 ignored). `cargo test --features amd-xdna2 --lib` EXIT 0 (737 passed, 0 failed, 1 ignored). Four clippy commands EXIT 0: `--lib --bins --tests --examples` with and without `--features amd-xdna2`, and `--benches` with and without `--features amd-xdna2`, each with `-D warnings`. The same lib/bins/tests/examples clippy command without the feature was EXIT 101 before those three fixes and EXIT 0 after. invent_pass=0. Pass=No. ran_on_npu=false. Unlock none. `C_UNLOCK=2c9c12e8` CONSUMED. Q4 stays the open sentence below. Q2 stays partly answered.

---

## What waits

- **Merge into a mainline release, then linux-zen:** watched weekly (routine, Mondays ~9 AM MT). On arrival: run D-AIE4-3 checklist read-only (probe, GET_INFO), no submit.
- **Hardware:** AIE4 features need AIE4 silicon. Horizon cannot test them.
- **Product Pass:** unchanged wall — live `decide_ran_on_npu` on a residual-matched classic 1688 job. This series does not move it.

---

## Colibri-native adapter path

- `~/Projects/surmount/colibri/docs/xdna.md` (read-only peek): colibri already has an experimental opt-in `--xdna` / `COLI_XDNA=1` backend (`libcoli_xdna.so`) that submits the public IRON npu2 64x64x64 int16 matmul at session start and tiles expert GEMM via `TILED_64`; no token generate, no throughput claim. Not edited.
- dragon-npu's role stays the `npu_direct_bench` JSON contract (`docs/COLIBRI_ADAPTER.md`). Order Hunter wants: **prefill first**, then resident (always-used) layers; experts stay CPU/disk.
- Prefill readiness here needs a real `decide_ran_on_npu` matmul first. Until then any colibri NPU number is IRON external-runner, not dragon-npu Pass.
- AIE4 suspend/resume would matter for all-day colibri use only on AIE4 hosts. On 17f0 the all-day concern is the AIE2 wedge/recovery story.

---

## Open questions

- Q1: **Answered. The series does not cover Strix.** Unchanged `{ 0x17f0, 0x10, &dev_npu4_info }`, `{ 0x17f0, 0x11, &dev_npu5_info }`, `{ 0x17f0, 0x20, &dev_npu6_info }` in `drivers/accel/amdxdna/amdxdna_pci_drv.c`. Added `{ PCI_DEVICE(PCI_VENDOR_ID_AMD, 0x17f1) }` points at `&dev_npu3_classic_info` in `npu3_regs.c`. This host rev `0x20` stays npu6. Cover letter: tested on AIE4/NPU3, no-regression check on AIE2 (NPU1/NPU4). Retail chip names are not in patch 5.
- Q2: **Partly answered from V5 19, not closed as absent.** AIE2 runtime PM is in this series. [08350](https://lkml.iu.edu/2610.2/08350.html) (accessed 2026-10-23) adds `.runtime_resume = aie2_hw_resume` and `.runtime_suspend = aie2_hw_suspend` on `aie2_ops`. With those hooks present, AIE2 calls the existing suspend and resume functions. The new `-EBUSY` keep-active default is the path for a device that has no `runtime_suspend` hook. KMQ job fences and a job-timeout that stops the hardware context were not shown on the AIE2 files in the patches opened for D-AIE4-3. The question's "npu4" label is the AIE2 family. This host rev `0x20` stays `dev_npu6_info`. Rev `0x10` stays npu4.
- Q3: **Answered as the D-AIE4-3 re-check list.** The list is a read of the patches. It is not a measurement on a running kernel. The HMM invalidate wait is now common, uninterruptible, and unlimited. V5 15 states "No functional change for AIE2" for the `populate_range` move, and the function body after the signature was truncated on the page read. The doorbell BAR mmap removal is inside `aie4_hwctx_create`. The common file operations `.mmap` becomes `drm_gem_mmap` for every `amdxdna_fops` device, including `0x17f0`. V5 08 `aie2_pm.c` is a 10-line field move onto `struct aie_device`. The V5 cover letter that was read has no `dev_heap_max_size` or `drm_mm_init` sentence. Lab 64 MiB DevHeap alignment stays `mmap_dev_heap_drm_aligned` in `src/backends/amd_xdna2/direct.rs`.
- Q4: Which mainline release carries drm-misc-next from 2026-10-21 (likely the next merge window after the current one)?

---

## Do not

- Claim Pass, ran_on_npu, or unlock from the merge or a kernel bump.
- Submit EXEC_CMD, load/unload amdxdna, reboot, or ask the operator to re-run `just npu-iron`.
- Put this into the specs repo.
