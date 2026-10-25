### Latest Turn (mlir-aie cite gates green; invent_pass=0)

- **Named delta:** No product edit this turn. `WHOLE_ARRAY_HOST_OPCODE` stays the integer 3. `WHOLE_ARRAY_HOST_ARG_ORDER` stays opcode, instr, ninstr, A, B, C. Packed EXEC_CMD, packed WAIT_CMD, xclbin AIE_PARTITION, numeric `amdxdna_cmd_start_npu` offsets, `num_tiles`, and the instruction CREATE_BO type stay typed errors. The device was not opened. `testdata/xdna2_cmd/whole_array_128_i16/mlir-aie-ioctl-command.txt` was not run. `XDNA2_ALLOW_EXEC_CMD` was not set. `decide_ran_on_npu` was not edited. invent_pass=0 not_c_ready=1 not_ran_on_npu=1; Pass=**No**; ran_on_npu=false; unlock **none**; first unlock line `2c9c12e8` CONSUMED. Do not mint. Do not reopen CONSUMED.
- **Wall:** product Pass still needs live `decide_ran_on_npu` on buffers this crate submitted. A green offline gate is not Pass.
- **Who acts next:** a later slice still has to submit the verified pair itself. This turn does not run that command. Do not ask the Operator to re-run `just npu-iron`. Agent does not mint.
- **Q4:** stays open. Watch for a linux tag, or a drm-next tag after `drm-next-2026-10-21`, that contains `drm-misc-next-2026-10-23`.
- **Gate:** nproc printed 16. `CARGO_BUILD_JOBS=8`. `nice -n 19`. `PATH="/usr/bin:$PATH"`. `cargo fmt -- --check` exit 0. `cargo clippy --lib --tests --examples --benches -- -D warnings` exit 0. `cargo clippy --features amd-xdna2 --all-targets -- -D warnings` exit 0. `cargo test --lib` exit 0 (168 passed, 0 failed, 0 ignored). `cargo test --features amd-xdna2 --lib` exit 0 (753 passed, 0 failed, 1 ignored).
- **Report:** `/home/hunter/.agents/reports/dragon-npu-mlir-aie-cite-impl.md`.

### Latest Turn (ioctl-native MLIR_AIE prep; invent_pass=0)

- **Named delta:** This is ioctl-native MLIR_AIE prep. It is not product Pass. START_NPU does not count as MLIR_AIE. The command `testdata/xdna2_cmd/whole_array_128_i16/mlir-aie-ioctl-command.txt` was not run. `/dev/accel` was not opened. `XDNA2_ALLOW_EXEC_CMD` was not set. `decide_ran_on_npu` was not edited. The first gate shell printed FILTER_EXIT:0, CLIPPY_EXIT:0, CLIPPY_XDNA_EXIT:101, TEST_LIB_EXIT:0, TEST_XDNA_EXIT:0. The red clippy was single-character matmul names plus an exact f32 narrowing check in `src/backends/amd_xdna2/mlir_aie_ioctl.rs`, then shared failure returns in `examples/mlir_aie_ioctl.rs`. The tests were not weakened. No packed ioctl constant was invented. Typed errors for unproven packed ioctls stayed. The rerun of `cargo clippy --features amd-xdna2 --all-targets -- -D warnings` printed CLIPPY_XDNA_EXIT:0. The mlir_aie filter rerun printed FILTER_EXIT:0. Default lib tests were 168 passed. Feature lib tests were 750 passed, 0 failed, 1 ignored. invent_pass=0 not_c_ready=1 not_ran_on_npu=1; Pass=**No**; ran_on_npu=false; unlock **none**; first unlock line `2c9c12e8` CONSUMED. Do not mint. Do not reopen CONSUMED.
- **Wall:** product Pass still needs live `decide_ran_on_npu` on buffers this crate submitted. Offline ioctl prep is not Pass. An external PASS string is not product Pass.
- **Who acts next:** a later slice still has to submit the verified pair itself. This turn does not run that command. Do not ask the Operator to re-run `just npu-iron`. Agent does not mint.
- **Q4:** stays open. Watch for a linux tag, or a drm-next tag after `drm-next-2026-10-21`, that contains `drm-misc-next-2026-10-23`.
- **Report:** `/home/hunter/.agents/reports/dragon-npu-mlir-aie-ioctl-impl.md`.

### Latest Turn (recorded external whole-array proof, not product Pass; invent_pass=0)

- **Named delta:** This is a recorded hardware proof of an already-finished external program, not a product Pass. The program is `/tmp/whole-array-128-durable/whole_array.exe`. The xclbin is `testdata/xdna2_cmd/whole_array_128_i16/final.xclbin`, sha256 `0cadf32ec0472e3f55e9102a6175d005b1a95a8ea428907086e15ad44d825dad`, size 73327. The instructions are `testdata/xdna2_cmd/whole_array_128_i16/insts.bin`, sha256 `ae692499a79c74b46175cb4841f235edb0059175144941522f792105b758dec3`, size 2592. The host is PCI `1022:17f0` rev `0x20`. This crate calls rev `0x20` npu6. The public IRON name used to build the pair was npu2. Do not rename the device. The mlir-aie checkout is `a8fea363dfd63015cffd380e0679950419a0a7cd`. Shape is M=K=N=128, tile m=k=n=16, 4 columns, dtype i16 in, i32 out, kernel name `MLIR_AIE`. Result: exit 0, stdout `PASS!` against the CPU reference, one sample 1977 microseconds, about 2.12 GFLOPS. Stderr was empty. The journal window 2026-10-24 01:00:41 through 01:01:30 had no amdxdna lines. It did not hang. `decide_ran_on_npu` returned false, reason `EmptyBuffers`, because this crate did not submit. The call used empty buffers, `submit_wait_ok` false, and `real_ioctl_path` false. invent_pass=0 not_c_ready=1 not_ran_on_npu=1; Pass=**No**; ran_on_npu=false; unlock **none**; first unlock line `2c9c12e8` CONSUMED. Do not mint. Do not reopen CONSUMED. The older xclbin sha256 `286fe2324ef02cad008c99718cf648a6f0286aa5b6170dba62856ca1b90665c1` must not be used. This turn does not run the program.
- **Wall:** product Pass still needs live `decide_ran_on_npu` on buffers this crate submitted. An external PASS string is not product Pass.
- **Who acts next:** the crate still has to submit this same verified pair itself so `decide_ran_on_npu` can see real buffers. That submit is a later slice. This turn does not run it. Do not ask the Operator to re-run `just npu-iron`. Do not ask anyone to place a classic 1688 file. Agent does not mint.
- **Q4:** stays open. Watch for a linux tag, or a drm-next tag after `drm-next-2026-10-21`, that contains `drm-misc-next-2026-10-23`.
- **Report:** `/home/hunter/.agents/reports/dragon-npu-xrt-sequence.md`.

### Latest Turn (whole-array 128 live run of fresh xclbin 0cadf32e; invent_pass=0)

- **Named delta:** One live run of /tmp/whole-array-128-durable/whole_array.exe used testdata/xdna2_cmd/whole_array_128_i16/final.xclbin, the fresh file whose live sha256 is 0cadf32ec0472e3f55e9102a6175d005b1a95a8ea428907086e15ad44d825dad (73327 bytes). It did not use the older xclbin 286fe2324ef02cad008c99718cf648a6f0286aa5b6170dba62856ca1b90665c1. insts.bin live sha256 is ae692499a79c74b46175cb4841f235edb0059175144941522f792105b758dec3 (2592 bytes). mlir-aie HEAD was a8fea363dfd63015cffd380e0679950419a0a7cd with an empty porcelain status. Exit code 0. The stdout line is PASS!. That PASS is the external binary's CPU-reference result. It is not this crate's gated ioctl. A scratch caller of decide_ran_on_npu_reason, with empty host golden, empty device buffer, had_instruction_blob true, submit_wait_ok false, and real_ioctl_path false, returned Err(EmptyBuffers). decide_ran_on_npu returned false. Pass=**No**. invent_pass=0. ran_on_npu=false. not_c_ready=1. not_ran_on_npu=1. Unlock **none**. First unlock line `C_UNLOCK=2c9c12e8` CONSUMED. Do not mint. Do not reopen CONSUMED. residual_matched_claim stays false. decide_ran_on_npu was not edited. The kernel journal for 2026-10-24 01:00:41 through 01:00:44 had no entries, and an amdxdna filter through 01:01:30 also had no entries. dmesg was not permitted. No amdxdna task was stuck in D state. The run did not hang, so no root recovery step was taken.
- **Wall:** product Pass still needs live decide_ran_on_npu from this crate's own gated submit and matching buffers. An external PASS is not Pass.
- **Who acts next:** nobody places a file and nobody re-runs `just npu-iron` because of this note. Agent does not mint.
- **Q4:** stays open. Watch for a linux tag, or a drm-next tag after `drm-next-2026-10-21`, that contains `drm-misc-next-2026-10-23`.
- **Report:** /home/hunter/.agents/reports/dragon-npu-whole-array-live-run.md.

## Development status (2026-10-12)

The Rust minimum for this crate is 1.99. `cargo update` and `cargo audit` were run on the stable 1.99 toolchain. The audit loaded 1307 security advisories from the RustSec database, scanned Cargo.lock (159 crate dependencies), and exited 0. No advisory warning remained. The earlier anymap2 advisory (RUSTSEC-2026-0319) closed because tract-onnx is now 0.23.8 and that line no longer pulls in liquid or anymap2. That was an upgrade. It was not an audit ignore.

Product Pass is still no. invent_pass stays 0. ran_on_npu stays false. The first unlock line remains `2c9c12e8` consumed. Do not mint a new unlock. Do not reopen a consumed one.

An IRON catalog child PASS is an external runner. It is not `decide_ran_on_npu`. That function, in `src/backends/amd_xdna2/numerical.rs`, is a pure check. It returns true only when the host golden and the device buffer are both non-empty, the host golden is not all zeros, the caller recorded a real gated submit-and-wait, an instruction blob was present, the path was a real ioctl, and the float values match within the relative and absolute tolerances. A length mismatch, a NaN, or an infinity fails the match. The function does not open a device and does not submit work. `XDNA2_LAB_ALLOW_ZERO_GOLDEN` does not override the all-zero golden refusal.

`testdata/xdna2_cmd/external_known_good/` holds research peels (300, 660, 2592, and 4512 bytes). Those peels are not a load-accepted lab matmul. The 1688-byte file `testdata/xdna2_cmd/silicon_lab/lab_instr.bin` is the unpatched lab body.

The hardware outcome still open is one matmul this crate submits, the device finishes, and the output matches `decide_ran_on_npu`.

# Residual — silicon path (honest status)

> **Agent rules (read first):** Chat and status = **plain English first** (what
> happened / what is blocked / which path or file / next action). Use real paths
> (`testdata/xdna2_cmd/external_known_good/`), not nicknames (“bag,” “Path A,”
> “mill”). Do **not** mint thoughtless stamp/unlock hex. **Empty
> reconfirm and empty-folder `/implement` loops are frozen** — if capture bytes
> or evidence are still missing, say so **once** and **stop**; do not auto-queue
> another reconfirm. Name work by research question, not a new track letter.
> Full rules: `AGENTS.md` standing §6–9.

**Date:** 2026-08-13

### Latest Turn (AIE4 kernel submission is not this host; invent_pass=0)

- **Named delta:** V5 "accel/amdxdna: Kernel submission and PM for AIE4" was applied to drm-misc-next on 2026-10-21 ([14306](https://lkml.iu.edu/2610.2/14306.html), accessed: 2026-10-24). Kernel-mode submit is `.cmd_submit = aie4_cmd_submit` on `aie4_classic_ops` in `drivers/accel/amdxdna/aie4_pci.c` ([07861](https://lkml.iu.edu/2610.2/07861.html), accessed: 2026-10-24). Patch 5 binds that struct with `.ops = &aie4_classic_ops` on `dev_npu3_classic_info` for `1022:17f1` revisions `0x10` and `0x13` ([07943](https://lkml.iu.edu/2610.2/07943.html), accessed: 2026-10-24). This host is `1022:17f0` revision `0x20` and stays on `dev_npu6_info`, so it keeps the AIE2 submit path. Do not read `&aie2_ops` as that AIE4 callback. Linux master `npu6_regs.c` still has `.ops = &aie2_ops` ([npu6_regs.c](https://raw.githubusercontent.com/torvalds/linux/master/drivers/accel/amdxdna/npu6_regs.c), accessed: 2026-10-24). Patch 19 only adds `.runtime_suspend = aie2_hw_suspend` and `.runtime_resume = aie2_hw_resume` on `aie2_ops` ([08350](https://lkml.iu.edu/2610.2/08350.html), accessed: 2026-10-24). Patch 18's `aie4_classic_suspend` drains jobs and the resume path recreates firmware contexts ([08021](https://lkml.iu.edu/2610.2/08021.html), accessed: 2026-10-24). That page does not say job-save or firmware-restore. Clock, DPM, and SR-IOV restore (`aie4_restore_sriov`) are AIE4. No AIE2 hook for those was on the pages opened, so they are N/A for `17f0`. Patch 15 `populate_range` and patch 02 doorbell mmap stay on the D-AIE4-3 shared-code list. They were not run. Horizon list not run: `power/runtime_status`, `autosuspend_delay_ms`, first-submit latency after idle, a submit across suspend/resume, and whether bridge reset of `0000:00:08.2` is still needed after resume. The 17f0 procedure stays diagnose, that bridge reset, and the rescue module. `uname -r` printed `7.2.11-zen1-1-zen`. Strings from the installed module include `aie2_ops` and `aie2_cmd_submit` and do not include `aie4_cmd_submit` or `aie4_classic`. That is the string search, not uname alone. Q4 stays open: [607373](https://lists.freedesktop.org/archives/dri-devel/2026-October/607373.html) (accessed: 2026-10-24) requests tag `drm-misc-next-2026-10-23` for v7.5. That name is a requested tag, not a released mainline tag. No version is claimed. Pass=**No**. invent_pass=0. not_c_ready=1. not_ran_on_npu=1. ran_on_npu=false. Unlock **none**. First unlock line `C_UNLOCK=2c9c12e8` CONSUMED. Do not mint. Do not reopen CONSUMED. `decide_ran_on_npu` was not edited.
- **Wall:** product Pass still needs live `decide_ran_on_npu`. An applied drm-misc-next series is not Pass. A runtime-PM hook on `aie2_ops` is not Pass.
- **Who acts next:** the person who holds a residual-matched classic 1688-byte ctrltext places it under `testdata/xdna2_cmd/external_known_good/`. Do not ask the Operator to re-run `just npu-iron`. Agent does not mint. The horizon list above stays unrun.
- **Q4:** stays open. Watch for a linux tag, or a drm-next tag after `drm-next-2026-10-21`, that contains `drm-misc-next-2026-10-23`.
- **Gate:** four commands, shell exported `CARGO_BUILD_JOBS=4` before cargo, `nice -n 19`, `PATH="/usr/bin:$PATH"`. The log file does not record `CARGO_BUILD_JOBS`. Exits 0, 0, 0, 0. `cargo test --lib` 168 passed, 0 failed, 0 ignored. `cargo test --features amd-xdna2 --lib` 737 passed, 0 failed, 1 ignored. A green gate is not product Pass. `uname -r` printed `7.2.11-zen1-1-zen`. nproc printed 16.

### Latest Turn (1688 is a project-wave rule; whole_array.py not compiled; invent_pass=0)

- **Named delta:** The only-acceptable residual-matched classic 1688-byte ctrltext sentence is a project-wave rule, not a driver or firmware size gate. `decide_ran_on_npu` in `src/backends/amd_xdna2/numerical.rs` does not mention length 1688. It returns true only when the host golden and the device buffer are both non-empty, the golden is not all zeros, the caller recorded a real gated submit-and-wait, an instruction blob was present, the path was a real ioctl, and the floats match within tolerance. The chosen offline route is public mlir-aie `whole_array.py`. Download yes. The file is 18212 bytes. The URL is https://raw.githubusercontent.com/Xilinx/mlir-aie/main/programming_examples/basic/matrix_multiplication/whole_array/whole_array.py. The instruction filename is the caller-supplied `--insts-path`. This agent cannot run python3. The refusal quote is exactly: Refused: grok-oss does not spawn python3. This agent did not compile that recipe. The fetched GitHub releases list was 372879 bytes. That download is not given a durable path in this note. The first html_url is https://github.com/Xilinx/mlir-aie/releases/tag/v1.4.5-alpha.1. The API url is https://api.github.com/repos/Xilinx/mlir-aie/releases/414321393. Whole-file counts verified: 156 `browser_download_url` lines, 0 `insts.bin`, 0 `.xclbin`, 312 `.whl`, and 0 non-wheel download URLs. One wheel: https://github.com/Xilinx/mlir-aie/releases/download/v1.4.5-alpha.1/mlir_aie-1.4.5a1-cp311-cp311-manylinux_2_35_x86_64.whl. Download of a prebuilt instruction binary from those assets is no. Raw insts and xclbin fetch bodies are the text "404: Not Found". The GitHub code search body is HTTP 401. The raw request URLs were not stored, so they are not written here. Rejected and not used: 300-byte sha16 `aaeb5b68cc298a45`, 660-byte sha16 `de8671245bb72436`, 2592-byte sha16 `7ee4b267ce31cd70`, 4512-byte sha16 `18cb653c05a9d4c8`, and the lab 1688-byte sha16 `3c5ffca750922d3b`. Pass=**No**. invent_pass=0. ran_on_npu=false. residual_matched_claim stays false. Unlock **none** because the first unlock line is `C_UNLOCK=2c9c12e8` CONSUMED. Do not mint. Do not reopen a consumed unlock. No submit was executed. No new classifier input. This is not product Pass.
- **Wall:** product Pass still needs a live `decide_ran_on_npu` result. This note is not product Pass. Downloading the Python recipe is not a compiled instruction file.
- **Who acts next:** a later wave may build the whole-array instruction file outside this agent (python3 is refused here) and then residual-check it. Do not ask the Operator to re-run `just npu-iron`. Do not ask the Operator to place a 1688-byte file as the next step of this wave.
- **Gate:** no new test. `decide_ran_on_npu` was not edited. No cargo was run this turn. No submit was executed.

### Latest Turn (classic 1688 origin cited; no residual-matched buffer offline; invent_pass=0)

- **Named delta:** The classic 1688-byte body in this tree is the unpatched DynamicDispatch dump of `mladfmatmulbias_prmptflat_mladf_2x4x4_flat_a16fw4acc16f_1_1152_256_128` (`docs/BUILD_NPU_MATMUL_PROGRAM.md` lines 18-41; `docs/XDNA2_XRT_CONTRACT_MLADF.md` lines 24 and 56). `STAGED_MLADF_CLASSIC_INSTR_BYTES` is 1688 at `src/backends/amd_xdna2/txn.rs` line 30. `MATERIAL_EXEC_LAB_CLASSIC_BYTES` aliases that constant at `src/backends/amd_xdna2/silicon_contract.rs` line 14525. `LAB_CLASSIC_UNPATCHED_CTRLTEXT_SHA16` is `3c5ffca750922d3b` at line 16485. This turn measured `testdata/xdna2_cmd/silicon_lab/lab_instr.bin` at 1688 bytes, sha256 `3c5ffca750922d3b6b1dec66b792273e46504cc1d3de6b5b96e12ac5c8197368` (same bytes as `testdata/xdna2_cmd/lab_instr.bin` and `testdata/xdna2_cmd/gt_capture/instr_raw_txn.bin`). That sha16 identifies the unpatched lab body. It is not a residual-matched or load-accepted claim. `classify_material_residual_match` sets `residual_matched_claim` false on every return (line 13907). Length 1688 is the lab classic oracle (lines 13967-13970; test at lines 30516-30519). A `residual-matched` tag is criteria only and still false (lines 13915-13923; test at lines 30512-30514). IRON and mlir-aie tags stay public research (lines 13935-13948). Public true post-patch dump research is closed offline (`docs/XDNA2_XRT_CONTRACT_MLADF.md` lines 56 and 99-101). The synthetic GT patch `testdata/xdna2_cmd/gt_capture/instr_post_patch.bin` measured sha256 `ccab3960f6dd789b7cad9ea602dc2ee6a0ac4c3805324625ea8228156713dbf0`, which is `GT_SYNTHETIC_PATCHED_CTRLTEXT_SHA256` (lines 16474-16479) and is not load-accepted. No new buffer was built. No unit test was added. `decide_ran_on_npu` was not edited. invent_pass=0. Pass=**No**. ran_on_npu=false. Unlock **none**. First unlock line `2c9c12e8` CONSUMED. Do not mint. Do not reopen CONSUMED.
- **Rejected as the known-good:** research ctrltext peels measured this turn under `testdata/xdna2_cmd/external_known_good/`: 300 bytes sha16 `aaeb5b68cc298a45`, 660 bytes sha16 `de8671245bb72436`, 2592 bytes sha16 `7ee4b267ce31cd70`, 4512 bytes sha16 `18cb653c05a9d4c8`. Also rejected: `testdata/xdna2_cmd/silicon_lab/lab_instr.bin` (unpatched 1688 above). Do not retag those public peels as load-accepted.
- **Wall:** product Pass still needs live `decide_ran_on_npu`. The single file only the Operator holds is a residual-matched classic 1688-byte ctrltext to place under `testdata/xdna2_cmd/external_known_good/`. This tree cannot emit that file. No submission command was written.
- **Who acts next:** the Operator, for that one file. Do not ask the Operator to re-run `just npu-iron`. Agent does not mint.
- **Q4:** stays open. Watch for a linux tag, or a drm-next tag after `drm-next-2026-10-21`, that contains `drm-misc-next-2026-10-23`.
- **Gate:** `/tmp/grok-goal-5f7882d2f7dc/implementer/gate.log`. The 20:29:12 four-exit log is withdrawn. nproc is 16. `CARGO_BUILD_JOBS=8`. `nice -n 19`. `PATH="/usr/bin:$PATH"`. Six commands ran one after another. Shell exits 0, 0, 0, 0, 0, 0. (1) `cargo clippy --lib --bins --tests --examples -- -D warnings` exit 0, wall 1 s (`date +%s` 1792809294 to 1792809295), summary `Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.08s`. (2) `cargo clippy --features amd-xdna2 --lib --bins --tests --examples -- -D warnings` exit 0, wall 0 s (`date +%s` start and end both 1792809298), summary `Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.04s`. (3) `cargo clippy --benches -- -D warnings` exit 0, wall 0 s (both 1792809300), summary `Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.04s`. (4) `cargo clippy --features amd-xdna2 --benches -- -D warnings` exit 0, wall 0 s (both 1792809303), summary `Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.05s`. (5) `cargo test --lib` exit 0, wall 0 s (both 1792809305), summary `test result: ok. 168 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s`. (6) `cargo test --features amd-xdna2 --lib` exit 0, wall 1 s (1792809308 to 1792809309), summary `test result: ok. 737 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.18s`. These exits are not product Pass.

### Latest Turn (ops pointer yes; Q3 pages read; invent_pass=0)

- **Named offline delta:** The ops pointer is yes. Source is linux
  master `npu6_regs.c`, fetched 2026-10-23, not the drm-misc-next tag.
  `dev_npu6_info.ops` is `&aie2_ops`. The V5 series does not edit
  `npu6_regs.c`. V5 patch 19
  (https://lkml.iu.edu/2610.2/08350.html, accessed 2026-10-23) adds
  `.runtime_resume = aie2_hw_resume` and
  `.runtime_suspend = aie2_hw_suspend` on `aie2_ops`, so those hooks
  are on the ops table this host uses (`1022:17f0` rev `0x20`). Q3,
  pages opened 2026-10-23: V5 15
  (https://lkml.iu.edu/2610.2/07837.html) says "No functional change
  for AIE2." The `populate_range` body after the signature was read
  and matches across the move (name changes from
  `aie2_populate_range` to `amdxdna_populate_range`; both sides return
  `ret == -EBUSY ? -ETIME : ret`). V5 02
  (https://lists.freedesktop.org/archives/dri-devel/2026-October/606181.html)
  sets `hwctx->doorbell_offset = AMDXDNA_INVALID_DOORBELL_OFFSET` and
  sets `amdxdna_fops.mmap` to `drm_gem_mmap`. Q4 stays open. Pull
  https://lists.freedesktop.org/archives/dri-devel/2026-October/607373.html
  (accessed 2026-10-23) requests tag `drm-misc-next-2026-10-23`. That
  is not a released mainline tag. No new test. `decide_ran_on_npu`
  was not edited. invent_pass=0 not_c_ready=1 not_ran_on_npu=1;
  Pass=**No**; unlock **none** (`2c9c12e8` CONSUMED). Do not mint. Do
  not reopen CONSUMED. No sticky ALLOW.
- **Wall:** product Pass still needs live `decide_ran_on_npu`. An ops
  pointer is not Pass. The person who holds a residual-matched classic
  1688 still places it under
  `testdata/xdna2_cmd/external_known_good/`.
- **Who acts next:** that person, for that one file. Do not ask the
  operator to re-run `just npu-iron`. Agent does not mint. The one
  next offline delta is named only. It is not started.
- **Gate:** no new test this turn.
- **Reports:** `/home/hunter/.agents/reports/dragon-npu-aie4-ops-page.md`;
  plan `.agents/plans/plan-amd-aie4-kmq-pm-series.md`;
  `/home/hunter/.agents/reports/dragon-npu-aie4-kmq-pm.md`.

### Prior Turn (Wave 2 Q2 Q3 Q4; invent_pass=0)

- **Named offline delta:** Wave 2. Patch 19 adds
  `.runtime_resume = aie2_hw_resume` and
  `.runtime_suspend = aie2_hw_suspend` on `aie2_ops` in `aie2_pci.c`,
  and `amdxdna_pm_runtime_suspend` / `amdxdna_pm_runtime_resume` in
  `amdxdna_pm.c`. Patch 5 keeps `{ 0x17f0, 0x20, &dev_npu6_info }`.
  The ops pointer page is missing. V5 08 `npu4_regs.c` does not set
  `dev_npu6_info.ops`. Do not guess `&aie2_ops`. Q4 stays open. Pull
  https://lists.freedesktop.org/archives/dri-devel/2026-October/607373.html
  (accessed 2026-10-23) requests tag `drm-misc-next-2026-10-23`. That
  is not a released mainline tag. Watch signal: a linux tag, or a
  drm-next tag after `drm-next-2026-10-21`, that contains
  `drm-misc-next-2026-10-23`. Q3 cover and V5 08 lines are cited. The
  V5 15 `populate_range` body and the V5 02 doorbell hunk stay open.
  No new test. `decide_ran_on_npu` was not edited. invent_pass=0
  not_c_ready=1 not_ran_on_npu=1; Pass=**No**; unlock **none**
  (`2c9c12e8` CONSUMED). Do not mint. Do not reopen CONSUMED. No
  sticky ALLOW.
- **Wall:** product Pass still needs live `decide_ran_on_npu`. The
  operator places a residual-matched classic 1688 under
  `testdata/xdna2_cmd/external_known_good/`. A requested tag is not
  Pass.
- **Who acts next:** smaller offline read is the missing
  `dev_npu6_info` ops page. Do **not** ask the operator to re-run
  `just npu-iron`. Agent does **not** mint. Horizon list is not run.
- **Gate:** log
  `call-4f6b0998-5910-4941-afc8-b57ababb657d-21.log`. nproc printed
  16. `JOBS=8`. `nice` and `PATH` were not printed in that log.
  Exits 0, 0, 0, 0. Default lib 168 passed, 0 failed, 0 ignored.
  Feature lib 737 passed, 0 failed, 1 ignored.
- **Reports:** `/home/hunter/.agents/reports/dragon-npu-aie4-kmq-pm.md`
  section Wave 2.
- **Honesty footer:**
  `invent_pass=0 product_pass=0 ran_on_npu=0 unlock=none C_UNLOCK=2c9c12e8 CONSUMED`

### Prior Turn (D-AIE4 offline gate green; invent_pass=0)

- **Named offline delta:** Offline gate for D-AIE4-1 through D-AIE4-5
  is green on the tree that includes the three clippy fixes in
  `src/backends/mod.rs`. `classify_amd_npu_pci_id` no longer carries
  `#[must_use]` (the Result is already must-use). The AIE4 device
  arm is `0x17f1..=0x17f3` at revisions `0x10 | 0x13`.
  `parse_sysfs_hex_u32` uses `strip_prefix("0x").unwrap_or(trimmed)`.
  Classification is unchanged: `1022:17f0` rev `0x10` is npu4, rev
  `0x11` is npu5, rev `0x20` is npu6. AIE4 ids fail closed. The
  series does not cover Strix. invent_pass=0 not_c_ready=1
  not_ran_on_npu=1; Pass=**No**; unlock **none** (`2c9c12e8`
  CONSUMED). Do not mint. Do not reopen CONSUMED. No sticky ALLOW.
- **Wall:** product Pass still needs live `decide_ran_on_npu`. A
  green offline gate is not Pass.
- **Who acts next:** Q4 stays open: which mainline release carries
  the 2026-10-21 drm-misc-next. Do not ask the operator to re-run
  `just npu-iron`. Agent does not mint.
- **Gate:** `cargo test --lib` EXIT 0 (168 passed, 0 failed, 0
  ignored). `cargo test --features amd-xdna2 --lib` EXIT 0 (737
  passed, 0 failed, 1 ignored). Four clippy commands EXIT 0.
- **Honesty footer:**
  `invent_pass=0 product_pass=0 ran_on_npu=0 unlock=none C_UNLOCK=2c9c12e8 CONSUMED`
- **Reports:** `/home/hunter/.agents/reports/dragon-npu-aie4-kmq-pm.md`

### Prior Turn (D-AIE4-5 colibri adapter note; invent_pass=0)

- **Named offline delta:** **D-AIE4-5 CLOSED.**
  `docs/COLIBRI_ADAPTER.md` documents optional `npu_generation`. Schema
  stays 1. `TimedOpResult::to_json_value()` does not emit the field in
  this delta. When a later producer sets it, `npu4` is `1022:17f0` rev
  `0x10`, `npu5` is rev `0x11`, `npu6` is rev `0x20`, and `npu3` is
  AIE4 classic `0x17f1`. Colibrì may refuse `npu3`. The field does not
  set `ran_on_npu`. `/home/hunter/Projects/surmount/colibri` was not
  edited. invent_pass=0 not_c_ready=1 not_ran_on_npu=1; Pass=**No**;
  unlock **none** (`2c9c12e8` CONSUMED). Do not mint. Do not reopen
  CONSUMED. No sticky ALLOW.
- **Wall:** product Pass still needs live `decide_ran_on_npu`. An
  optional JSON field idea is not Pass.
- **Who acts next:** one offline gate for the five deltas (`cargo test
  --lib`, `cargo test --features amd-xdna2 --lib`, clippy). Q4 stays
  open: which mainline release carries the 2026-10-21 drm-misc-next.
  Do **not** ask the operator to re-run `just npu-iron`. Agent does
  **not** mint.
- **Honesty footer:**
  `invent_pass=0 product_pass=0 ran_on_npu=0 unlock=none C_UNLOCK=2c9c12e8 CONSUMED`

### Prior Turn (D-AIE4-4 recovery note; invent_pass=0)

- **Named offline delta:** **D-AIE4-4 CLOSED.** Paragraph "AIE4 power
  management and 17f0 recovery" is in
  `docs/XDNA2_RECOVERY_ENGINEERING.md`, under the driver-fork roadmap.
  The 17f0 procedure stays diagnose, bridge reset of `0000:00:08.2`,
  and the rescue module. V5 19 runtime suspend and resume is power
  management, including the `aie2_ops` hooks to `aie2_hw_suspend` and
  `aie2_hw_resume`. The AIE4 job-timeout is a pattern for the `open`
  timeout / killable-wait row. The V5 14 unlimited HMM wait stays the
  D-AIE4-3 re-check. invent_pass=0 not_c_ready=1 not_ran_on_npu=1;
  Pass=**No**; unlock **none** (`2c9c12e8` CONSUMED). Do not mint. Do
  not reopen CONSUMED. No sticky ALLOW.
- **Wall:** product Pass still needs live `decide_ran_on_npu`. A
  recovery paragraph is not Pass.
- **Who acts next:** D-AIE4-5 colibri adapter note in this repo. Do
  **not** ask the operator to re-run `just npu-iron`. Agent does
  **not** mint. Do not edit `/home/hunter/Projects/surmount/colibri`.
- **Honesty footer:**
  `invent_pass=0 product_pass=0 ran_on_npu=0 unlock=none C_UNLOCK=2c9c12e8 CONSUMED`

### Prior Turn (D-AIE4-3 shared-code re-check list; invent_pass=0)

- **Named offline delta:** **D-AIE4-3 CLOSED.** Offline read of the V5
  shared hunks. No device open and no submit. The re-check list is in
  `.agents/plans/plan-amd-aie4-kmq-pm-series.md` item 3. Short form:
  V5 14 HMM wait is `dma_resv_wait_timeout(..., false,
  MAX_SCHEDULE_TIMEOUT)` in common GEM, and the comment says the wait
  must be neither interruptible nor time limited. V5 15 says "No
  functional change for AIE2" for the `populate_range` move. The
  function body after the signature was truncated on the page read.
  V5 02 sets `hwctx->doorbell_offset = AMDXDNA_INVALID_DOORBELL_OFFSET`
  inside `aie4_hwctx_create` and sets common `.mmap = drm_gem_mmap`.
  This crate's doorbell use is the CREATE_HWCTX field `umq_doorbell`
  (default 0) in `src/backends/amd_xdna2/direct.rs`. V5 19 adds
  `.runtime_suspend = aie2_hw_suspend` and `.runtime_resume =
  aie2_hw_resume` on `aie2_ops`. V5 08 `aie2_pm.c` is the 10-line move
  of `clk_gating` and `set_dpm` onto `struct aie_device`
  (`aie2_pm.c | 10 +-`). The V5 cover "Changes in v5" that was read has
  no `dev_heap_max_size` or `drm_mm_init` sentence. Lab 64 MiB
  alignment stays `mmap_dev_heap_drm_aligned` (align `0x4000000`).
  invent_pass=0 not_c_ready=1 not_ran_on_npu=1; Pass=**No**; unlock
  **none** (`2c9c12e8` CONSUMED). Do not mint. Do not reopen CONSUMED.
  No sticky ALLOW.
- **Q2:** AIE2 runtime PM is in this series via those two `aie2_ops`
  assignments ([08350](https://lkml.iu.edu/2610.2/08350.html), accessed
  2026-10-23). KMQ job fences and a job-timeout stop of the hardware
  context were not shown on the AIE2 files opened for this delta.
- **Q3:** Answered by the list above. It is a patch read. It is not a
  measurement on a running kernel.
- **Wall:** product Pass still needs live `decide_ran_on_npu`. A
  re-check list is not Pass.
- **Who acts next:** D-AIE4-4 recovery-doc paragraph. Do **not** ask
  the operator to re-run `just npu-iron`. Agent does **not** mint.
- **Honesty footer:**
  `invent_pass=0 product_pass=0 ran_on_npu=0 unlock=none C_UNLOCK=2c9c12e8 CONSUMED`

### Prior Turn (D-AIE4-1 Strix answer + D-AIE4-2 classification; invent_pass=0)

- **Q1:** The series does not cover Strix. This host is `1022:17f0`
  rev `0x20`. In `drivers/accel/amdxdna/amdxdna_pci_drv.c`, patch 5
  leaves `{ 0x17f0, 0x10, &dev_npu4_info }`,
  `{ 0x17f0, 0x11, &dev_npu5_info }`, and
  `{ 0x17f0, 0x20, &dev_npu6_info }` unchanged, and leaves
  `{ PCI_DEVICE(PCI_VENDOR_ID_AMD, 0x17f0) }` unchanged. The new classic
  device is `{ PCI_DEVICE(PCI_VENDOR_ID_AMD, 0x17f1) }` with
  `{ 0x17f1, 0x10, &dev_npu3_classic_info }` and
  `{ 0x17f1, 0x13, &dev_npu3_classic_info }`. That struct is
  `dev_npu3_classic_info` in `drivers/accel/amdxdna/npu3_regs.c`
  (`.ops = &aie4_classic_ops`). Rev `0x10` of `0x17f0` is npu4. Rev
  `0x11` is npu5. Rev `0x20` stays npu6. Patch 5 adds more than one
  row (classic plus PF/VF rev `0x13`). Cover letter sentences read
  from [14306](https://lkml.iu.edu/2610.2/14306.html) (accessed
  2026-10-23): "This patch series extends the amdxdna accelerator
  driver to support AMD AIE4 (NPU3) platforms, including classic and
  PF/VF for SR-IOV with kernel-mode command submission (KMQ), system
  power management, and runtime power management for the classic
  device." "Tested on AMD AIE4/NPU3 hardware in both classic and
  SR-IOV (PF/VF) modes." "Confirmed no regression on existing AIE2
  devices (NPU1/NPU4)."
- **Named offline delta:** **D-AIE4-2 CLOSED.** Pure test
  `backends::tests::classify_amd_npu_pci_id_aie2_rows_and_aie4_fail_closed`.
  Command: `PATH="/usr/bin:$PATH" nice -n 19 cargo test --features
  amd-xdna2 --lib
  backends::tests::classify_amd_npu_pci_id_aie2_rows_and_aie4_fail_closed
  -- --exact`. Result: `ok`. 1 passed. 0 failed. 737 filtered out.
  EXIT:0. The AIE4 sentence is "AIE4: not supported by this crate's
  AIE2 cmdlist path". `cmdlist_class_from_sysfs` does not open
  `/dev/accel`. The red compile (classifier absent) was not captured,
  so no red exit code is recorded. No new Pass field.
- **Honesty:** still **no** Pass; invent_pass=0; **no** `ran_on_npu`.
  Classification is not `decide_ran_on_npu`. Unlock **none** (first
  `C_UNLOCK=` = **`2c9c12e8` CONSUMED**). Do **not** mint. Do **not**
  reopen CONSUMED. No sticky ALLOW.
- **Who acts next:** D-AIE4-3 shared-code regression list. Do **not**
  ask the operator to re-run `just npu-iron`. Agent does **not** mint.
- **Honesty footer:**
  `invent_pass=0 product_pass=0 ran_on_npu=0 unlock=none C_UNLOCK=2c9c12e8 CONSUMED`

### Prior Turn (D-AIE4-1 coverage table; invent_pass=0)

- **Named offline delta:** **D-AIE4-1 CLOSED.** Read V5 patch 5,
  "accel/amdxdna: Add NPU3 classic device support"
  ([07943](https://lkml.iu.edu/2610.2/07943.html), accessed 2026-10-23).
  The cover letter index
  ([14306](https://lkml.iu.edu/2610.2/14306.html), accessed 2026-10-23)
  does not contain the id table. Lizhi Hou's reply on that page
  (2026-10-21) says the series was applied to drm-misc-next.
- **IDs in the patch 5 table:** PCI match list is AMD `0x1502`,
  `0x17f0`, added `0x17f1`, `0x17f2`, `0x17f3`. Removed `0x1B0B` and
  `0x1B0C` ("will not be released"). Device-info rows: `0x17f0` rev
  `0x10` → `dev_npu4_info`; rev `0x11` → `dev_npu5_info`; rev `0x20` →
  `dev_npu6_info`. Added NPU3 classic: `0x17f1` rev `0x10` and rev
  `0x13` → `dev_npu3_classic_info` (`aie4_classic_ops`, vbnv
  `RyzenAI-npu3`). NPU3 PF `0x17f2` and VF `0x17f3` (revs `0x10` and
  `0x13`) stay on the AIE4 side. `0x1502` is in the match list only.
  Patch 5 does not show its info struct. That cell is blank.
- **Q1:** The table covers `1022:17f0` and does not treat it as NPU3.
  This host (`1022:17f0` rev `0x20`) is `dev_npu6_info`. NPU3 classic
  is `1022:17f1`. The new AIE4 classic path does not bind to `17f0`.
  Retail SKU names are not in the patch.
- **Honesty:** still **no** Pass; invent_pass=0; **no** `ran_on_npu`.
  An upstream id table is not `decide_ran_on_npu`. Unlock **none**
  (first `C_UNLOCK=` = **`2c9c12e8` CONSUMED**). Do **not** mint. Do
  **not** reopen CONSUMED. No sticky ALLOW.
- **Note:** `docs/upstream/AIE4_NPU3_PCI_IDS.md`.
- **Who acts next:** D-AIE4-2 (pure probe classification). Do **not**
  ask the operator to re-run `just npu-iron`. Agent does **not** mint.
- **Honesty footer:**
  `invent_pass=0 product_pass=0 ran_on_npu=0 unlock=none C_UNLOCK=2c9c12e8 CONSUMED`

### Prior Turn (useful wave 2 implement loop CLOSED; invent_pass=0)

- **Named offline delta:** Implement loop **`5dca83c3` CLOSED** (effort 3;
  review rounds 2). Re-review r2: **0 open** (general + tests + plan).
  Catalog wave 2, second identity `iron_npu2_vector_add`, and wrap restore
  already landed. Default `just npu-iron` is the run+verify wrap. Export
  is opt-in (`just npu-iron-export` / `--iron-export`). Kind-match is
  exact **300 B**. One allow-list. Catalog / wrap / export collect ≠
  Pass ≠ unlock. invent_pass=0 not_c_ready=1 not_ran_on_npu=1. Pass=**No**.
  Unlock **none** (first `C_UNLOCK=` = **`2c9c12e8` CONSUMED**). Do **not**
  mint. Do **not** reopen CONSUMED. No sticky ALLOW.
- **Honesty:** still **no** Pass; invent_pass=0; **no** `ran_on_npu`.
  Child `PASS!` stays `execution=external_runner`. Named wrap ≠
  `decide_ran_on_npu`. Do not idle only on a human 1688 file.
- **Primary wall (product Pass):** live `decide_ran_on_npu` (ioctl + ERT
  Completed + OFM allclose). Classic 1688 is still missing for the lab
  matmul job.
- **Who acts next:** Human residual-matched classic 1688 when held.
  Agent only on a **new named offline delta**. Do **not** ask the
  operator to re-run `just npu-iron`. Agent does **not** mint.
- **Highest value next:**
  1. A new named offline delta that either makes more IRON wrap useful
     (new catalog names that already run on this host) or honestly
     advances product EXEC, without claiming Pass from wrap.
  2. Human residual-matched classic 1688 when they actually hold it.
  3. Frozen: empty same-shot reconfirm, reopen CONSUMED, mint from this
     closeout, retag public 300/660 peels as 1688 load-accepted, ask
     the operator to re-run `just npu-iron`.
- **Reports:** `.agents/reports/impl-useful-wave2-identity.md`;
  `/tmp/grok-1000/grok-impl-summary-5dca83c3.md`;
  `/tmp/grok-1000/grok-review-5dca83c3.md`.
- **Honesty footer:**
  `invent_pass=0 product_pass=0 ran_on_npu=0 unlock=none C_UNLOCK=2c9c12e8 CONSUMED`

### Prior Turn (useful wave 2 review-fix: default wrap restored; invent_pass=0)

- **Named offline delta:** Review-fix for catalog wave 2 + second
  identity. Default `just npu-iron` / `dragon-npu run --iron vector_add`
  is the **run+verify wrap** again (no `--aot-dir`). Sibling
  `vector_scalar_add.py --aot-dir` is export-only (no child `PASS!`).
  Collect is opt-in: `just npu-iron-export` or `--iron-export`. Kind-match
  for `iron_npu2_vector_add` is exact **300 B** (660 excluded). One
  allow-list (case-insensitive, hyphen). invent_pass=0 not_c_ready=1
  not_ran_on_npu=1. Pass=**No**. Unlock **none** (first `C_UNLOCK=` =
  **`2c9c12e8` CONSUMED**). Catalog ≠ Pass ≠ unlock. Do **not** mint.
  Do **not** reopen CONSUMED. No sticky ALLOW.
- **Honesty:** still **no** Pass; invent_pass=0; **no** `ran_on_npu`.
  Child `PASS!` stays `execution=external_runner`. Named wrap ≠
  `decide_ran_on_npu`. Second identity ≠ 1688 load-accepted. Export-only
  collect ≠ Pass.
- **Primary wall (product Pass):** live `decide_ran_on_npu` (ioctl + ERT
  Completed + OFM allclose). Classic 1688 is still missing for the lab
  matmul job.
- **Who acts next:** Human residual-matched classic 1688 when held.
  Agent only on a **new named offline delta**. Do **not** ask the
  operator to re-run `just npu-iron`. Agent does **not** mint.
- **Reports:** `.agents/reports/impl-useful-wave2-identity.md`;
  `/tmp/grok-1000/grok-impl-summary-5dca83c3.md`.
- **Honesty footer:**
  `invent_pass=0 product_pass=0 ran_on_npu=0 unlock=none C_UNLOCK=2c9c12e8 CONSUMED`

### Prior Turn (useful wave 2 + second identity JOINED; invent_pass=0)

- **Named offline delta:** Catalog **wave 2** landed: `gelu`, `silu`,
  `eltwise_mul`, `swiglu`, `scale_shift` (plus wave 1 names). Recipes
  `just npu-gelu` / `npu-silu` / `npu-mul` / `npu-swiglu` /
  `npu-scale-shift` and `dragon-npu run --iron NAME`. Unknown name still
  fails closed. Vector add may pass `--aot-dir` at a this-run directory
  and parse npy / printed vectors when present. Collect PROVENANCE job is
  **`iron_npu2_vector_add`**, a different identity from frozen classic
  **1688**. Public 300-byte peel may be the right *kind* for that IRON
  job. It is **not** 1688 load-accepted. Capture helper refuses
  `residual_matched_claim` against classic 1688 for this job.
  Agent ran `just npu-gelu` / `npu-silu` / `npu-mul` / `npu-swiglu` /
  `npu-scale-shift`: `child_said_pass=true`, `ran_on_npu=false`,
  `output_len=0`. Agent ran `just npu-iron`: this-run `--aot-dir`
  wrote `insts.bin` 300 B plus xclbin/pdi/elf; child printed artifact
  paths (not `PASS!`); `child_said_pass=false`; collect note
  `job=iron_npu2_vector_add residual_matched_claim=0`. That 300 B peel
  is the IRON-job kind, never 1688 load-accepted. invent_pass=0
  not_c_ready=1 not_ran_on_npu=1. Pass=**No**. Unlock **none** (first
  `C_UNLOCK=` = **`2c9c12e8` CONSUMED**). Catalog ≠ Pass ≠ unlock. Do
  **not** mint. Do **not** reopen CONSUMED. No sticky ALLOW.
- **Honesty:** still **no** Pass; invent_pass=0; **no** `ran_on_npu`.
  Child `PASS!` stays `execution=external_runner`. Wave 2 names ≠ Pass.
  Second identity ≠ 1688 load-accepted. Wrap ≠ `decide_ran_on_npu`.
- **Primary wall (product Pass):** live `decide_ran_on_npu` (ioctl + ERT
  Completed + OFM allclose). Classic 1688 is still missing for the lab
  matmul job. The IRON vector-add job is a named second identity, not a
  reason to idle on a 1688 file.
- **Who acts next:** This named delta is done. Human residual-matched
  classic 1688 when held. Agent only on a **new named offline delta**.
  Do **not** ask the operator to re-run `just npu-iron`. Agent does
  **not** mint.
- **Reports:** `.agents/reports/impl-useful-wave2-identity.md`;
  `/tmp/grok-1000/grok-impl-summary-5dca83c3.md`.
- **Honesty footer:**
  `invent_pass=0 product_pass=0 ran_on_npu=0 unlock=none C_UNLOCK=2c9c12e8 CONSUMED`

### Prior Turn (useful-npu-commands review-fix; invent_pass=0)

- **Named offline delta:** Catalog review-fix. `--iron` no longer compiles
  a dummy host fixture. Leftover `output.npy` is this-run only when the
  child mentions the basename. Stdout `shape [256]` is not OFM. Host
  default fixture resolves from the crate root. Shell catalog suffixes
  removed (Rust `--iron` is the only suffix map). invent_pass=0
  not_c_ready=1 not_ran_on_npu=1. Pass=**No**. Unlock **none** (first
  `C_UNLOCK=` = **`2c9c12e8` CONSUMED**). Catalog ≠ Pass ≠ unlock. Do
  **not** mint. Do **not** reopen CONSUMED. No sticky ALLOW.
- **Honesty:** still **no** Pass; invent_pass=0; **no** `ran_on_npu`.
  Review-fix ≠ Pass.
- **Primary wall (product Pass):** residual-matched / load-accepted
  classic **1688**.
- **Who acts next:** Human residual-matched classic 1688 when held.
  Agent only on a **new named offline delta**. Do **not** ask the
  operator to re-run `just npu-iron`. Agent does **not** mint.
- **Reports:** `/tmp/grok-1000/grok-review-32f40c3e.md`;
  `/tmp/grok-1000/grok-impl-summary-32f40c3e.md`.

### Prior Turn (useful-npu-commands JOINED; invent_pass=0)

- **Named delta:** Plan **useful-npu-commands** catalog **landed**.
  Named IRON: `just npu-relu` / `npu-eltwise` / `npu-softmax` / `npu-iron`
  and `dragon-npu run --iron NAME` (`vector_add`, `eltwise`, `relu`,
  `softmax`). Unknown name fails closed. Child numbers parsed when
  stdout or a known npy actually has them; `PASS!` alone stays empty
  `Vec<f32>` plus `child_said_pass`. `--host-graph` runs host
  `evaluate_host_graph` on `tests/fixtures/host_matmul_relu_add.json`
  (`[-1, 2]` → `[0.5, 3.5]`, `execution=host`). Default `run` is still
  simulation tanh. Agent verified `just npu-relu`, `npu-eltwise`,
  `npu-softmax`: child exit 0, `child_said_pass=true`,
  `execution=external_runner`, `ran_on_npu=false`, `invent_pass=0`,
  `output_len=0` (no invent OFM). invent_pass=0 not_c_ready=1
  not_ran_on_npu=1. Pass=**No**. Unlock **none** (first `C_UNLOCK=` =
  **`2c9c12e8` CONSUMED**). Catalog ≠ Pass ≠ unlock. Do **not** mint.
  Do **not** reopen CONSUMED. No sticky ALLOW.
- **Honesty:** still **no** Pass; invent_pass=0; **no** `ran_on_npu`.
  IRON child `PASS!` is **not** product `ran_on_npu`. Host linear is
  **not** NPU. Named commands ≠ product Pass.
- **Primary wall (product Pass):** residual-matched / load-accepted
  classic **1688**. Public peels and unpatched oracle do **not** count.
  Next Pass move is still human residual-matched classic 1688.
- **Who acts next:** Human residual-matched / load-accepted classic 1688
  when held. Agent only on a **new named offline delta**. Operator may
  run `just npu-relu` / `--iron` without a product unlock. Agent does
  **not** mint. Do **not** ask the operator to re-run `just npu-iron`.
- **Reports:** `.agents/reports/impl-useful-commands-catalog.md`; plan
  `.agents/plans/plan-useful-npu-commands.md`.

### Prior Turn (useful-npu-commands pin; invent_pass=0)

- **Named delta (docs/residual only):** Plan **useful-npu-commands**
  APPROVED. Named IRON catalog + host-graph CLI. Not a Pass plan. Do
  **not** ask the operator to re-run `just npu-iron`. invent_pass=0
  not_c_ready=1 not_ran_on_npu=1. Pass=**No**. Unlock **none** (first
  `C_UNLOCK=` = **`2c9c12e8` CONSUMED**). Catalog ≠ Pass ≠ unlock. IRON
  child `PASS!` is **not** product `ran_on_npu`. Do **not** mint. Do
  **not** reopen CONSUMED. No sticky ALLOW.
- **Honesty:** still **no** Pass; invent_pass=0; **no** `ran_on_npu`.
  Plan approve ≠ Pass. Named `just npu-relu` / `--iron` ≠ product Pass.
- **Primary wall (product Pass):** residual-matched / load-accepted
  classic **1688**. Public peels and unpatched oracle do **not** count.
- **Who acts next:** Agent ships catalog, parse, `--host-graph`, tests,
  agent-run verify. Human residual-matched classic 1688 when held. Agent
  does **not** mint.
- **Reports:** plan `.agents/plans/plan-useful-npu-commands.md`.

### Prior Turn (useful-npu-accel S0–S5 JOINED; invent_pass=0)

- **Named delta (docs/residual only):** Dual-track plan **useful-npu-accel**
  **S0–S5 JOINED.** (1) **Useful-soon shipped:** opt-in external-runner wrap
  (`--external-runner`) plus host linear demo. IRON child `PASS!` does
  **not** set product `ran_on_npu`. Operator can run `dragon-npu` against
  mlir-aie examples **without** a product unlock. (2) **Pure-direct residual
  unchanged:** still **`wait_cmd_einval`**. S3 ranked out-fence lookup first
  (`OutFenceLookupFail`); **`live_justified=0`**. Diagnostic is **not** an
  unlock reason. Same-shot proven under **`C_UNLOCK=2c9c12e8` CONSUMED**;
  next product live only under a **new** unlock and a **new** named reason.
  invent_pass=0 not_c_ready=1 not_ran_on_npu=1. Pass=**No**. Unlock **none**
  (first `C_UNLOCK=` = **`2c9c12e8` CONSUMED**).
- **S5 agent-try (landed):** NPU ran this week via IRON eltwise add / relu /
  softmax (`PASS!`). Not product Pass. MobileNet not prebuilt (torch
  missing). eepy VitisAI listed; offload not proven (CPU fallback). IRON
  `PASS!` and eepy CPU smoke are **not** product `ran_on_npu`.
- **Honesty:** still **no** Pass; invent_pass=0; **no** `ran_on_npu`. Join
  ≠ Pass. Wrapper ≠ Pass. Host demo ≠ NPU. Diagnostic ≠ unlock. Same-shot
  measured ≠ Pass. Freeze empty same-shot reconfirm. Do **not** mint. Do
  **not** reopen CONSUMED. No sticky ALLOW.
- **Primary wall (product Pass):** residual-matched / load-accepted
  completing-stack classic **1688** for ERT Completed + OFM allclose. Public
  mlir-aie peels and unpatched oracle do **not** count.
- **Who acts next:** **Human** residual-matched / load-accepted classic 1688
  when held. **Agent** only on a **new named offline delta**. Operator may
  use `--external-runner` without unlock. Agent does **not** mint.
- **Reports:** `.agents/reports/impl-useful-accel-s4-join.md`; S1
  `impl-useful-accel-s1-runner.md`; S2 `impl-useful-accel-s2-host-demo.md`;
  S3 `impl-useful-accel-s3-wait-diag.md` + `wait-fence-diagnostic.md`; S5
  `useful-accel-s5-try-npu.md`; plan `.agents/plans/plan-useful-npu-accel.md`.

### Prior Turn (useful-npu-accel S0 pin; invent_pass=0)

- **Named delta (docs/residual only):** Plan **useful-npu-accel** APPROVED.
  Dual track named: (1) **Useful-soon:** opt-in external-runner wrap of stacks
  that already run (IRON first). Never launder product `ran_on_npu`.
  (2) **Pure-direct residual unchanged:** still **`wait_cmd_einval`**;
  same-shot proven under **`C_UNLOCK=2c9c12e8` CONSUMED**; next product live
  only under a **new** unlock and a **new** named reason. This plan is
  **not** a mint. invent_pass=0 not_c_ready=1 not_ran_on_npu=1. Pass=**No**.
  Unlock **none** (first `C_UNLOCK=` = **`2c9c12e8` CONSUMED**). IRON `PASS!`
  and eepy CPU smoke are **not** product `ran_on_npu`. Freeze empty same-shot
  reconfirm. Do **not** mint. Do **not** reopen CONSUMED. No sticky ALLOW.
- **Operator add-on (already approved):** S5 is **agent-try**, not parked
  human-only. Build the tools we need. Try every agent-doable useful path.
  Minimal operator involvement. No unrealistic asks.
- **Honesty:** still **no** Pass; invent_pass=0; **no** `ran_on_npu`. Plan
  approve ≠ Pass. Mirror ≠ unlock. Same-shot measured ≠ Pass. IRON PASS ≠
  product `decide_ran_on_npu`. eepy CPU smoke ≠ NPU session.
- **Primary wall (product Pass):** residual-matched / load-accepted
  completing-stack classic **1688** for ERT Completed + OFM allclose. Public
  mlir-aie peels and unpatched oracle do **not** count.
- **Who acts next:** Agent **S1 ∥ S2 ∥ S3** then **S4** join; **S5**
  agent-try IRON MobileNet/ResNet and/or one eepy VitisAI session (still
  **not** product `ran_on_npu`). Human residual-matched / load-accepted
  material when held. Agent does **not** mint. Next product live only under
  **new** unlock + **new** named reason.
- **Reports:** `.agents/reports/impl-useful-accel-s0-pin.md`; plan
  `.agents/plans/plan-useful-npu-accel.md`.

### Prior Turn (O1 post-fail ERT / Cmd BO dump; invent_pass=0)

- **Named offline delta:** Plan option1 explore **P5** landed offline. Pure
  post-fail ERT observability: `PostFailErtPresence` /
  `PostFailErtResearch` / `parse_post_fail_ert_research` /
  `format_post_fail_ert_research_soft` / `format_post_fail_ert_detail_tokens`
  in `silicon_contract.rs`. Soft always **invent_pass=0** not_c_ready=1
  not_ran_on_npu=1 research_alone_not_pass=1 research_alone_not_unlock=1.
  Product: timed **WAIT Err** greps `ert_post_fail=present|unread` (from local
  read facts); pre-EXEC **`cmd_bo_failed` / `cmd_bo_fill_failed`** greps
  `ert_post_fail=expected_absent`. Soft classify prints
  `post_fail_ert_research=…`. Historical same-shot last_apply (no token) →
  `ert_post_fail=missing_from_detail` / `split_hint=historical_no_ert_token`.
- **Honesty:** still **no** Pass; invent_pass=0; **no** `ran_on_npu`. Unlock
  **none** (first `C_UNLOCK=` = **`2c9c12e8` CONSUMED**). Do **not** reopen.
  Do **not** mint. Post-fail ERT dump ≠ residual match ≠ Pass. Freeze empty
  same-shot reconfirm.
- **Primary wall:** residual-matched / load-accepted completing-stack material
  for ERT Completed + OFM allclose. Next live only under **new** unlock + named
  reason (new last_apply can then split R1 vs R4 via post-fail ERT tokens).
- **Who acts next:** **human** material into
  `testdata/xdna2_cmd/external_known_good/`, or agent only on a **new named
  offline delta**.
- **Reports:** `.agents/reports/impl-o1-post-fail-ert.md`; prior join
  `.agents/reports/impl-o1-p2-join.md`.

### Prior Turn (option1-after-same-shot P0–P2 join closeout; invent_pass=0)

- **Named delta:** Plan **option1-after-same-shot** offline **P0–P2 JOINED**.
  P1-A: residual-matched / load-accepted material **not found**
  (`live_justified_from_material=no`). P1-B: wait-path = observability only
  (`live_justified_from_wait_theory=no`). **P2 join:** **live_justified=no**.
  **Do not mint.** **Do not run P3/P4.** Residual still **`wait_cmd_einval`**;
  invent_pass=0; Pass=**No**; **no** `ran_on_npu`. Unlock **none** (first
  `C_UNLOCK=` = **`2c9c12e8` CONSUMED**). Same-shot live under that unlock
  remains closed science ≠ Pass.
- **Stop reason:** neither arm gained evidence that changes the product residual.
  Empty same-shot reconfirm **frozen**. Do **not** reopen `2c9c12e8`.
- **Primary wall:** residual-matched / load-accepted completing-stack material
  for ERT Completed + OFM allclose. Wait-path soft/syncobj research ≠ root fix.
- **Who acts next:** **human** for load-accepted peel / private stack into
  `testdata/xdna2_cmd/external_known_good/` (or scope expand). **Agent** only
  on a **new named offline delta** — not empty reconfirm.
- **Reports:** `.agents/reports/impl-o1-p2-join.md`;
  `.agents/reports/impl-o1-p1a-bridge-a-material.md`;
  `.agents/reports/impl-o1-p1b-wait-path.md`;
  `.agents/reports/impl-o1-p0-honesty-pin.md`; plan
  `.agents/plans/plan-option1-after-same-shot.md`.

### Prior Turn (option1-after-same-shot P0 honesty pin; invent_pass=0)

- **Plan approved:** `.agents/plans/plan-option1-after-same-shot.md` (session
  plan aligned). Same-shot product live already **proven** under
  **`C_UNLOCK=2c9c12e8` CONSUMED** (S4–S5 Arm A); residual class still
  **`wait_cmd_einval`**; still **no** Pass; invent_pass=0; **no** `ran_on_npu`.
- **Honesty:** same-shot measured ≠ Pass. Unlock **none active** (first
  `C_UNLOCK=` remains **`2c9c12e8` CONSUMED**). Do **not** reopen `2c9c12e8`.
  Do **not** mint from this pin. Freeze empty same-shot reconfirm / empty
  re-live of the same Arm A residual. No sticky ALLOW. No invent Pass.
- **Primary wall:** residual-matched / load-accepted completing-stack material
  for ERT Completed + OFM allclose; optional wait-path theory offline only.
- **Next work (then):** offline **P1-A** ∥ **P1-B** → **P2** join. Superseded
  by Latest Turn join (**live_justified=no**).
- **Reports:** `.agents/reports/impl-o1-p0-honesty-pin.md`; plan
  `.agents/plans/plan-option1-after-same-shot.md`; prior live
  `.agents/reports/live-shot-2c9c12e8.md` /
  `.agents/reports/impl-live-shot-effective-s4-s5.md`.

### Prior Turn (S4–S5 closeout — live-shot-effective Arm A; invent_pass=0)

- **Named live evidence:** One product Arm A stock live under **`C_UNLOCK=2c9c12e8`
  CONSUMED**. Command: `config-cu-run-skip` (skip CONFIG_CU / no research PDI)
  with `XDNA2_LAB_SAME_SHOT_RELA=1`, OFM=1, BO_DIAG=1, outer≥120s. last_apply
  `/tmp/dragon_npu_last_apply_live_eff_2c9c12e8.json`; log
  `/tmp/dragon_npu_live_shot_2c9c12e8.log`.
- **Measured:** status=`wait_cmd_einval`; `ran_on_npu=false`; `real_ioctl=true`;
  total_s≈**0.165**; aiebu_elf=1 rela×12; **config_cu absent**; WAIT_CMD
  EINVAL (hwctx=1 seq=0 timeout_ms=5000 request=0xc0106449). **Same-shot
  gate:** `same_shot_gate=1 same_shot=1 paddrs_valid=1
  source=in_process_bo_map refuse_stale_disk=1` (in-process paddrs used;
  not stale disk UVAs). Soft residual `not_pass_no_ert_error`; C4_SKIP /
  P6_NOT_PASS. invent_pass=0.
- **Named reason (closed by shot):** “One product matmul with same-run address
  patching; measure Pass only by device complete + output match.”
- **Honesty:** still **no** Pass; invent_pass=0; **no** `ran_on_npu`. Same-shot
  measured ≠ Pass. Prep ≠ Pass. Unlock **CONSUMED** — do not reopen; do not
  second live under `2c9c12e8`. No sticky ALLOW. Journal scrape shows prior-boot
  noise (flexml IO_PAGE_FAULT / cmd timeout); this short shot (~0.16s) has no
  new timeout/fault class beyond WAIT EINVAL.
- **Unlock:** first `C_UNLOCK=` = **`2c9c12e8` CONSUMED**. Prior CONSUMED:
  `4008e6fd`, `48da6e60`, `1f761bf4`, … **Do not reopen.**
- **Primary wall:** still residual-matched / load-accepted completing-stack
  material for ERT Completed + OFM allclose. Same-shot path is proven live;
  EXEC residual class unchanged (`wait_cmd_einval` without overlay).
- **Who acts next:** **stop** empty reconfirm. Next live only under **new**
  unlock + **new** named reason (not same-shot reconfirm alone). Prefer Bridge A
  / load-accepted material over re-burn.
- **Reports:** `.agents/reports/impl-live-shot-effective-s4-s5.md`;
  `.agents/reports/live-shot-2c9c12e8.md`; plan
  `.agents/plans/plan-live-shot-effective.md`; prep
  `.agents/reports/impl-live-shot-effective-s0-s3.md`.

### Prior Turn (S4 mint — live-shot-effective Arm A product matmul; invent_pass=0)

- **Named delta:** Plan **live-shot-effective** prep **S0–S3 GREEN**. Host-minted
  **`C_UNLOCK=2c9c12e8`** (then active; now CONSUMED by Latest Turn closeout)
  for one product Arm A live: same-run address patching, skip CONFIG_CU / no
  research PDI, outer ≥120s, BO_DIAG, OFM path, new last_apply under `/tmp`.
- **Named live reason:** “One product matmul with same-run address patching;
  measure Pass only by device complete + output match.”
- **Honesty:** invent_pass=0; unlock ≠ Pass ≠ `ran_on_npu`. Prep green ≠ Pass.
  Superseded by Latest Turn CONSUME + measured outcomes.
- **Reports:** `.agents/reports/impl-live-shot-effective-s0-s3.md`; plan
  `.agents/plans/plan-live-shot-effective.md`.

### Prior Turn (live-shot-effective S0–S3 prep — plan APPROVED; invent_pass=0)

- **Named delta (docs + kit prep only):** Plan **live-shot-effective**
  **APPROVED** (`.agents/plans/plan-live-shot-effective.md`). **S0–S3 prep**
  landed: residual pin; kit + `lab_exec_once` forward/export
  `XDNA2_LAB_SAME_SHOT_RELA` for product Arm A; louder `same_shot=…` /
  `same_shot_gate=1` last_apply tokens when gate on; offline preflight path.
  **Named live reason (for when S4 mints):** “One product matmul with same-run
  address patching; measure Pass only by device complete + output match.”
  **Arm A default** (no extra PDI). invent_pass=0 · Pass=**No** · unlock
  **none** at prep close. Public AOT re-run remains **frozen**. Stale disk
  UVAs remain **forbidden** for product live patch. **Superseded for unlock
  by Latest Turn mint `2c9c12e8`.**
- **Honesty:** still **no** Pass; invent_pass=0; **no** `ran_on_npu`. Prep ≠
  Pass ≠ unlock ≠ `ran_on_npu`. Same-shot kit default ≠ live justified. Soft
  tokens ≠ C-ready. Do **not** reopen CONSUMED. No sticky ALLOW. No invent
  Pass. No invent capture bytes.
- **Unlock (historical at prep):** first was **`4008e6fd` CONSUMED**. Live only
  under **new** host-minted unlock (S4) + named reason after prep green.
- **Primary wall:** one **justified** product live matmul under same-run
  patching + **new** unlock + named reason (Arm A), and/or true
  completing-stack post-patch / load-accepted peel (Bridge A).
- **Who acts next:** S0–S3 prep complete; S4 mint active above.
- **Reports:** `.agents/reports/impl-live-shot-effective-s0-s3.md`; plan
  `.agents/plans/plan-live-shot-effective.md`; prior W10
  `.agents/reports/wave10-join.md`.

### Prior Turn (W10 join closeout — same-shot wire + soft; invent_pass=0)

- **Named delta:** Campaign **W10 JOINED** (S0 + W10-B + W10-soft). **W10-B:**
  same-shot offline product wire **ready=yes** (`plan_same_shot_*` +
  `XDNA2_LAB_SAME_SHOT_RELA` env-gated lab hook; product rela never loads disk
  UVAs). **W10-soft:** soft `classify_last_apply` always prints L1b readiness /
  wall tokens; offline **`l1b_live_justified=0`**. **Live L1b not run** (wire
  alone ≠ unlock; **do not mint**). invent_pass=0 · Pass=**No** ·
  not_ran_on_npu=1 · not_c_ready=1 · residual_matched=0 · load_accepted=0 ·
  unlock **none**. Public AOT re-run remains **frozen**. Stale disk UVAs remain
  **forbidden** for product live L1b.
- **Honesty:** still **no** Pass; invent_pass=0; **no** `ran_on_npu`. Same-shot
  offline wire ≠ residual match ≠ load-accepted ≠ `ran_on_npu`. Soft checklist
  ≠ C-ready ≠ unlock. Stale UVAs ≠ product live patch. Do **not** mint. Do
  **not** reopen CONSUMED. No sticky ALLOW. No invent Pass.
- **Unlock:** **none active.** First `C_UNLOCK=` = **`4008e6fd` CONSUMED**.
  **Do not mint** from this pin alone. Do not reopen `4008e6fd` or prior
  CONSUMED list.
- **Primary wall:** one **justified** live L1b under same-shot map + **new**
  unlock + named reason, and/or true completing-stack **post-patch** classic
  1688 / load-accepted peel (Bridge A).
- **Who acts next:** Idle empty reconfirm. Next named deltas: **justified L1b
  live** under **new** host-minted unlock + named reason (same-shot offline
  already green), **or** Bridge A residual-matched / intercept material. Point
  at plan `.agents/plans/plan-silicon-try-everything.md`. Freeze public AOT
  re-run / reopen CONSUMED / invent Pass / product live from stale disk UVAs /
  mint from wire or soft tokens alone.
- **Reports:** `.agents/reports/wave10-join.md`;
  `.agents/reports/wave10-b-same-shot.md`;
  `.agents/reports/wave10-soft-classify.md`;
  `.agents/reports/impl-w10-s0-pin-mirror.md`; plan
  `.agents/plans/plan-silicon-try-everything.md`.

### Prior Turn (W10 S0 — plan pin + same-shot L1b path; invent_pass=0)

- **Named delta (docs/residual only):** Campaign **W9 closed** (L1b-prep offline
  ready; intercept/sibling hunt done; live L1b not justified; no mint). Campaign
  **W10 APPROVED:** same-shot L1b product path (**W10-B** T1-same-shot
  in-process paddrs → rela before EXEC) + optional soft-wire (**W10-soft**
  T3 L1b checklist into classify); then **W10-join**. Plan mirrored to
  `.agents/plans/plan-silicon-try-everything.md`. invent_pass=0 · Pass=**No** ·
  not_ran_on_npu=1 · not_c_ready=1 · unlock **none**. Public AOT re-run remains
  **frozen**. Stale disk UVAs are **forbidden** for product live L1b.
- **Honesty:** still **no** Pass; invent_pass=0; **no** `ran_on_npu`. Plan
  approve ≠ Pass. Mirror ≠ unlock. Offline paddr evidence ≠ same-shot map ≠
  L1b live. Stale UVAs ≠ product live patch. No live from this pin alone. No
  invent Pass. No sticky ALLOW.
- **Unlock:** **none active.** First `C_UNLOCK=` = **`4008e6fd` CONSUMED**.
  **Do not mint** from this pin alone. Do not reopen `4008e6fd` or prior
  CONSUMED list. Live only under **new** host-minted unlock + named reason
  after same-shot path offline-green (or residual material).
- **Primary wall:** same-shot live-mapped rela (Bridge B product path), and/or
  true completing-stack **post-patch** classic 1688 / load-accepted peel.
- **Who acts next (agent):** **W10-B** (same-shot product path) parallel with
  optional **W10-soft**; then **W10-join**. **Live L1b only if** same-shot
  path offline-green **and** **new** unlock + named reason. Freeze empty
  reconfirm / public AOT re-run / reopen CONSUMED / invent Pass / product live
  from stale disk UVAs / L1b from offline checklist alone.
- **Reports:** `.agents/reports/impl-w10-s0-pin-mirror.md`; plan
  `.agents/plans/plan-silicon-try-everything.md`; W9 join
  `.agents/reports/wave9-join.md`.

### Prior Turn (W9 join closeout — L1b-prep + intercept; invent_pass=0)

- **Named delta:** Campaign **W9 JOINED** (S0 + W9-B + W9-A). **W9-B:** live
  paddrs **found on disk** (vendor fixtures + `/tmp` last_apply `bo_diag`);
  offline `paddr_evidence_ready=1` / `l1b_offline_ready=1` when extract succeeds;
  pure TDD `L1bReadiness` landed; **`l1b_live_justified=0`** (unlock still
  required; host UVAs **age out across processes** and must **not** drive a
  product live patch). **W9-A:** no residual-matched post-patch classic 1688;
  sibling non-oracle 1688s = unpatched other M/K/N only; INSTR_BO_LIMIT /
  private `run_` closed; FLM/Lemonade absent. **Live L1b not run** (not
  justified; **do not mint**). invent_pass=0 · Pass=**No** · not_ran_on_npu=1 ·
  not_c_ready=1 · residual_matched=0 · load_accepted=0 · unlock **none**. Public
  AOT re-run remains **frozen**.
- **Honesty:** still **no** Pass; invent_pass=0; **no** `ran_on_npu`. Offline
  paddr evidence ≠ live L1b. Stale UVAs ≠ same-shot map. Unpatched CLASS A /
  sibling shapes ≠ residual match ≠ load-accepted. Do **not** mint. Do **not**
  reopen CONSUMED. No sticky ALLOW. No invent Pass.
- **Unlock:** **none active.** First `C_UNLOCK=` = **`4008e6fd` CONSUMED**.
  **Do not mint** from this pin alone. Do not reopen `4008e6fd` or prior
  CONSUMED list.
- **Primary wall:** true completing-stack **post-patch** classic 1688 (or
  load-accepted residual-shaped peel), plus **same-shot** live paddrs if L1b is
  ever justified under a **new** unlock.
- **Who acts next:** Idle empty reconfirm. Next named deltas: same-shot paddr
  L1b product wiring; private `run_` intercept / true post-patch material.
  **L1b only after** same-shot map paddrs **and** **new** host-minted unlock +
  named reason. Freeze public AOT re-run / reopen CONSUMED / invent Pass /
  product live from stale disk UVAs / multi as residual.
- **Reports:** `.agents/reports/wave9-join.md`;
  `.agents/reports/wave9-b-l1b-prep.md`;
  `.agents/reports/wave9-a-bridge-a.md`;
  `.agents/reports/impl-w9-s0-pin-mirror.md`.

### Prior Turn (W9 S0 — plan pin + W9 tracks; invent_pass=0)

- **Named delta (docs/residual only):** Campaign **W8 closed** (Material: oracle
  unpatched `3c5ffca750922d3b`, INSTR_BO_LIMIT proven, multi refuse, A5 L3 sudo
  wall; residual_matched=0; load_accepted=0. Paddr: O3+M3 pure TDD landed;
  `l1b_offline_ready=0`; live not justified). Campaign **W9 APPROVED:** **W9-B**
  L1b-prep + T3 readiness; **W9-A** intercept/sibling; **W9-join**. Plan
  mirrored to `.agents/plans/plan-silicon-try-everything.md`. invent_pass=0 ·
  Pass=**No** · not_ran_on_npu=1 · not_c_ready=1 · unlock **none**. Public AOT
  re-run remains **frozen**.
- **Honesty:** still **no** Pass; invent_pass=0; **no** `ran_on_npu`. Plan
  approve ≠ Pass. Mirror ≠ unlock. Unpatched CLASS A ≠ residual match.
  Offline paddr wire ≠ L1b. No live from this pin alone. No invent Pass.
- **Unlock:** **none active.** First `C_UNLOCK=` = **`4008e6fd` CONSUMED**.
  **Do not mint** from this pin alone. Do not reopen `4008e6fd` or prior
  CONSUMED list. Live only under **new** host-minted unlock + named reason.
- **Primary wall:** true completing-stack **post-patch** classic 1688 (or
  load-accepted residual-shaped peel), and/or live device paddrs for Bridge B.
- **Who acts next (agent):** **W9-B** (T1-L1b-prep + T3 readiness) parallel with
  **W9-A** (run_ intercept / EP / sibling 1688 keys by host trees); then
  **W9-join**. **Live L1b only if** W9-B finds live paddrs on disk **and**
  **new** unlock + named reason. Freeze empty reconfirm / public AOT re-run /
  reopen CONSUMED / invent Pass / L1b from offline wire alone.
- **Reports:** `.agents/reports/impl-w9-s0-pin-mirror.md`; plan
  `.agents/plans/plan-silicon-try-everything.md`; W8 join
  `.agents/reports/wave8-join.md`.

### Prior Turn (W8 join closeout — Material + Paddr; invent_pass=0)

- **Named delta:** Campaign **W8 JOINED** (S0 + Track Material M1/M4/A5 + Track
  Paddr O3/M3). **Material:** re-capture still oracle unpatched classic 1688
  sha16 **`3c5ffca750922d3b`**; INSTR_BO_LIMIT proven after execute; multi 32208
  refused; A5 hardened L3 = sudo password wall; residual_matched=0;
  load_accepted=0; true post-patch still **missing**. **Paddr:** O3+M3 pure TDD
  landed (`w8_` 5 ok); soft `l1b_offline_ready=0`. **Live L1b not justified** —
  **do not mint** unlock. invent_pass=0 · Pass=**No** · not_ran_on_npu=1 ·
  not_c_ready=1 · unlock **none**. Public AOT re-run remains **frozen**.
- **Honesty:** still **no** Pass; invent_pass=0; **no** `ran_on_npu`. Unpatched
  CLASS A ≠ residual match ≠ load-accepted. Offline paddr wire ≠ L1b unlock.
  Capture OFM ≠ product `decide_ran_on_npu`. Do **not** mint. Do **not** reopen
  CONSUMED. No sticky ALLOW. No empty reconfirm of oracle `3c5ffca750922d3b`.
- **Unlock:** **none active.** First `C_UNLOCK=` = **`4008e6fd` CONSUMED**.
  **Do not mint** from this pin alone. Do not reopen `4008e6fd` or prior
  CONSUMED list.
- **Primary wall:** true completing-stack **post-patch** classic 1688 (or
  load-accepted residual-shaped peel). Material gap still highest.
- **Who acts next:** Idle empty reconfirm. **L1b only after** live device paddrs
  from a real map **and** **new** host-minted unlock + named reason. Human only
  for proven walls (sudo L3 / true load-accepted ERT+OFM under
  `testdata/xdna2_cmd/external_known_good/`). Freeze public AOT re-run / reopen
  CONSUMED / invent Pass / multi as residual.
- **Reports:** `.agents/reports/wave8-join.md`;
  `.agents/reports/wave8-material-track.md`;
  `.agents/reports/wave8-paddr-track.md`;
  `.agents/reports/impl-w8-s0-pin-mirror.md`.

### Prior Turn (W8 S0 — plan pin + mirror; Track Material + Track Paddr; invent_pass=0)

- **Named delta (docs/residual only):** Campaign **W7 closed** (unpatched classic
  re-land sha16 `3c5ffca750922d3b`; true post-patch still missing; collect refuse;
  residual_matched=0; load_accepted=0; **no live**). Campaign **W8 APPROVED:**
  parallel **Track Material** (M1 true post-patch / M4 INSTR_BO_LIMIT / A5 DD
  instruction_cache dump) + **Track Paddr** (O3 live paddr extract + M3 pure-direct
  rela with real device paddrs). Plan mirrored to
  `.agents/plans/plan-silicon-try-everything.md`. invent_pass=0 · Pass=**No** ·
  not_ran_on_npu=1 · not_c_ready=1 · unlock **none**.
- **Honesty:** still **no** Pass; invent_pass=0; **no** `ran_on_npu`. Plan approve
  ≠ Pass. Mirror ≠ unlock. Unpatched CLASS A ≠ residual match. No empty reconfirm.
  Public AOT re-run remains **frozen**. **No unlock from plan alone.** No sticky
  ALLOW. No invent capture. No dragon-npu live apply from this pin alone.
- **Unlock:** **none active.** First `C_UNLOCK=` = **`4008e6fd` CONSUMED**.
  **Do not mint** from this pin alone. Do not reopen `4008e6fd` or prior
  CONSUMED list. Live only under **new** host-minted unlock + named reason.
- **Primary wall:** true completing-stack **post-patch** classic 1688 (or
  residual-shaped peel), and/or live-mapped rela paddrs (M3) for product lab path.
- **Who acts next (agent):** W8 parallel tracks Material (M1/M4/A5) + Paddr
  (O3/M3). No L1/L8 without new material or named wait reason. Freeze empty
  reconfirm / public AOT re-run / reopen CONSUMED / invent Pass.
- **Reports:** `.agents/reports/impl-w8-s0-pin-mirror.md`; plan
  `.agents/plans/plan-silicon-try-everything.md`; W7 join
  `.agents/reports/wave7-join.md`.

### Prior Turn (W7 join closeout — capture unpatched + collect refuse; invent_pass=0)

- **Named delta:** Campaign **W7 agent-max residual close JOINED** (W7-0…W7-6).
  W7-1 capture-instr re-landed classic **1688** primary as
  `gt_capture/instr_vendor_post_patch.bin` with **unpatched** body sha16
  **`3c5ffca750922d3b`** (same as lab raw / DD dump / `instr_raw_txn.bin`).
  Multi **32208** diag only. W7-2 collect dry-run **refuse** all candidates
  (`basename` oracle / multi-txn); residual_matched_claim=**0**;
  load_accepted_claim=**0**. W7-3 soft `completed_on_this_fw` + W7-4 MASK*
  walk hygiene landed offline. **W7-5 live skipped** (no residual-matched new
  material; no unlock mint). Public AOT re-run remains **frozen**. invent_pass=0
  · Pass=**No** · not_ran_on_npu=1 · not_c_ready=1 · unlock **none**.
- **Honesty:** still **no** Pass; invent_pass=0; **no** `ran_on_npu`. Unpatched
  CLASS A ≠ C-ready ≠ residual match ≠ load-accepted. Capture OFM ≠ product
  `decide_ran_on_npu`. Do **not** mint unlock. Do **not** reopen CONSUMED.
  No sticky ALLOW. No dragon-npu live apply from this pin.
- **Unlock:** **none active.** First `C_UNLOCK=` = **`4008e6fd` CONSUMED**.
  **Do not mint** from this pin alone. Do not reopen `4008e6fd` or prior
  CONSUMED list.
- **Primary wall:** true completing-stack **post-patch** classic 1688 (or
  residual-shaped peel) with load-accepted ERT Completed + OFM allclose evidence.
  Unpatched classic re-land does **not** close residual.
- **Who acts next:** Idle agent empty reconfirm. Human only for proven walls:
  hardened L3 install (sudo) and/or true load-accepted material under
  `testdata/xdna2_cmd/external_known_good/`. Next live only under **new** unlock
  + named residual-shaped bytes. Freeze public AOT re-run / reopen CONSUMED /
  invent Pass.
- **Reports:** `.agents/reports/wave7-join.md`;
  `.agents/reports/wave7-capture-instr-attempt.md`;
  `.agents/reports/impl-w7-completed-on-this-fw.md`;
  `.agents/reports/impl-w7-cascade-walk-hygiene.md`.

### Prior Turn (W7 S0 — agent-max residual close pin + plan mirror; invent_pass=0)

- **Named delta (docs/residual only):** Campaign **W6 closed** (distance map
  lab classic 1688 vs IRON peels; non-public material hunt empty;
  residual_matched_claim=**0**; **L1 not** justified from W6 alone). Campaign
  **W7 agent-max residual close APPROVED.** Agent owns capture-instr / L3
  sandwich / ingest / unlock mint / live / interpret **until** proven
  human-only wall (exact error on disk). **Do not** park on operator bag-drop
  by default. Public AOT re-run remains **frozen**. Plan mirrored to
  `.agents/plans/plan-silicon-try-everything.md`. invent_pass=0 · Pass=**No** ·
  not_ran_on_npu=1 · not_c_ready=1 · unlock **none**.
- **Honesty:** still **no** Pass; invent_pass=0; **no** `ran_on_npu`. Plan
  approve ≠ Pass. Mirror ≠ unlock. W6 distance/hunt ≠ residual match ≠
  load-accepted. IRON PASS ≠ product `decide_ran_on_npu`. No invent Pass. No
  sticky ALLOW. No dragon-npu live apply from this pin alone.
- **Unlock:** **none active.** First `C_UNLOCK=` = **`4008e6fd` CONSUMED**.
  **Do not mint** from this pin alone. Do not reopen `4008e6fd` or prior
  CONSUMED list. Live only under **new** host-minted unlock + named reason.
- **Primary wall:** true completing-stack post-patch classic **1688** (or
  residual-shaped peel) for product lab path. Agent must **try** capture-instr
  / HITL recipes before asking operator.
- **Who acts next (agent):** **W7-1** capture-instr / classic-1688-prefer (L3
  via recipe; restore L3) → **W7-2** collect/score honesty → optional **W7-3**
  soft `completed_on_this_fw` token + **W7-4** cascade walk offline. **W7-5**
  live only if new residual-shaped bytes + new unlock. Idle only after agent
  exhausted agent-legal steps or hit proven human-only wall. Freeze empty
  reconfirm / public AOT re-run / reopen CONSUMED.
- **Reports:** `.agents/reports/impl-w7-s0-pin-mirror.md`; plan
  `.agents/plans/plan-silicon-try-everything.md`; W6 closeouts
  `.agents/reports/wave6-distance-lab-vs-iron.md`,
  `.agents/reports/wave6-material-hunt.md`.

### Prior Turn (W6 S0 — residual identity pin + plan mirror; invent_pass=0)

- **Named delta (docs/residual only):** Campaign **W5 closed** (Path B IRON
  all four public mlir-aie npu2 AOT peels **PASS!** on FW **1.1.2.64**; all
  `research_duplicate_sha16`; load_accepted_claim=0; residual_matched_claim=0
  for lab classic **1688**). Campaign **W6 residual identity APPROVED.**
  Residual identity is **primary**; **freeze** public AOT re-run (300 / 660 /
  2592 / 4512). Plan mirrored to `.agents/plans/plan-silicon-try-everything.md`.
  invent_pass=0 · Pass=**No** · not_ran_on_npu=1 · not_c_ready=1 · unlock
  **none**.
- **Honesty:** still **no** Pass; invent_pass=0; **no** `ran_on_npu`. IRON
  PASS ≠ product `decide_ran_on_npu` ≠ residual match ≠ load-accepted re-tag.
  Plan approve ≠ Pass. Mirror ≠ unlock. No invent Pass. No sticky ALLOW. No
  dragon-npu live apply from this pin.
- **Unlock:** **none active.** First `C_UNLOCK=` = **`4008e6fd` CONSUMED**.
  **Do not mint** from this pin alone. Do not reopen `4008e6fd` or prior
  CONSUMED list.
- **Primary wall:** residual-matched / product load-accepted material for lab
  classic **1688** / matmul-bias working stack. Public AOT peels remain
  **RESEARCH ONLY** (already proven on this FW under IRON).
- **Who acts next (agent offline):** **W6-1** distance map (lab classic 1688
  vs IRON completed peels) + **W6-2** non-public material hunt (DD/XRT
  post-patch / vendor trees) **in parallel**. Then W6-3…W6-5 only if new
  bytes. L1 only if residual match + **new** unlock. **No L8** from wire
  alone. Idle empty reconfirm / public AOT re-run.
- **Reports:** `.agents/reports/impl-w6-s0-pin-mirror.md`; plan
  `.agents/plans/plan-silicon-try-everything.md`; W5 closeouts under
  `.agents/reports/wave5-*.md`.

### Prior Turn (W5 join amend — vector + matmul Path B all PASS; invent_pass=0)

- **Named delta:** Campaign **W5 join amended** after matmul Path B. Path B
  IRON on FW **1.1.2.64** (system `pyxrt` on `PYTHONPATH`) completed **all
  four** public mlir-aie npu2 AOT peels:
  - vector_scalar_add **300 B** sha16 `aaeb5b68cc298a45` **PASS!**
  - single_core matmul **660 B** sha16 `de8671245bb72436` **PASS!**
  - whole_array matmul **2592 B** sha16 `7ee4b267ce31cd70` **PASS!**
  - cascade matmul **4512 B** sha16 `18cb653c05a9d4c8` **PASS!**
  All four: `completed_on_this_fw=yes`; collect class
  **`research_duplicate_sha16`**; load_accepted_claim=**0**;
  residual_matched_claim=**0** for lab classic **1688**. invent_pass=0 ·
  Pass=**No** · not_ran_on_npu=1 · not_c_ready=1 · unlock **none**.
- **Honesty:** IRON PASS ≠ product `decide_ran_on_npu` ≠ residual match ≠
  product load-accepted re-tag. No invent Pass. No unlock mint. No ALLOW.
  No dragon-npu live apply. Bag binaries unchanged (same research peels).
- **Unlock:** **none active.** First `C_UNLOCK=` = **`4008e6fd` CONSUMED**.
  **Do not mint** from this amend.
- **Primary wall:** still residual-matched / product load-accepted material
  for lab classic **1688** / matmul-bias working stack. Public AOT peels
  (300/660/2592/4512) are **RESEARCH ONLY** and already proven on this FW.
- **Who acts next:** (1) **Residual-useful:** non-public DD/XRT post-patch
  ctrltext for matmul-bias lab path, or intercept peels from a completing
  stack that matches residual identity — **not** re-run public AOT.
  (2) L1 only if residual match + **new** unlock. (3) **No L8** from wire
  alone. (4) Idle empty reconfirm / public AOT re-run.
- **Reports:** `.agents/reports/wave5-join.md` (amended);
  `wave5-join-matmul-amend.md`; `wave5-path-b-matmul-attempt.md`;
  `wave5-path-b-agent-attempt.md`; `wave5-path-c-agent-attempt.md`;
  `wave5-host-scan-agent.md`;
  `path-b-vector-scalar-add-npu2-pass-20260811.log`.

### Prior Turn (W5 join — capture automation + agent Path B/C; invent_pass=0)

- **Named delta:** Campaign **W5 join closed** (vector-only Path B at first
  pin). Tools landed: host scan, collect fail-closed, PROVENANCE load-accepted
  template, `docs/LOAD_ACCEPTED_CAPTURE.md`, `load_accepted_path_status.sh`.
  Operator “lol no u do it” → agent ran Path B/C. **Path B then:** IRON
  `vector_scalar_add` **PASS!** on NPU (FW **1.1.2.64**; system `pyxrt`);
  300 B sha16 `aaeb5b68cc298a45`. **Path C:** validate archives missing.
  **Path A:** FLM/Lemonade absent. invent_pass=0 · Pass=**No**. **Superseded
  for Path B completeness** by Latest Turn matmul amend above (all four peels).
- **Honesty:** IRON PASS ≠ product Pass ≠ residual match. No invent
  load-accepted. No unlock mint.
- **Unlock:** **none active.** First `C_UNLOCK=` = **`4008e6fd` CONSUMED**.
- **Reports:** `.agents/reports/wave5-join.md` (pre-matmul pin);
  `wave5-path-b-agent-attempt.md`; `wave5-path-c-agent-attempt.md`;
  `wave5-host-scan-agent.md`;
  `path-b-vector-scalar-add-npu2-pass-20260811.log`.

### Prior Turn (W5 agent host-scan + Path B IRON PASS; invent_pass=0)

- **Named delta:** Operator deferred bag-drop to **agent** capture attempt.
  W5 tools used: `load_accepted_path_status.sh`, `host_capture_scan.sh
  --report` (120 hits; multi_txn=1; research_dup=11), dry-run
  `collect_load_accepted_candidate.sh` (all load_accepted_claim=0).
  **Path B:** IRON `vector_scalar_add` **PASS!** on NPU after fixing
  missing `pyxrt` via system site-packages on `PYTHONPATH`. Log:
  `.agents/reports/path-b-vector-scalar-add-npu2-pass-20260811.log`.
  Ctrltext still **300 B** sha16 `aaeb5b68cc298a45` (research peel identity;
  helpers **refuse** product load_accepted re-tag). Path C: `xrt-smi
  examine` sees RyzenAI-npu6 FW **1.1.2.64** (health ≠ Pass). Path A
  FastFlowLM/Lemonade still **absent**. invent_pass=0 · Pass=**No** ·
  not_ran_on_npu=1 · not_c_ready=1 · load_accepted_claim=0.
- **Honesty:** IRON verify PASS ≠ product `decide_ran_on_npu` ≠ residual
  match to lab classic 1688 / spent flexml. No invent load-accepted. No
  unlock mint. No ALLOW. No dragon-npu live apply.
- **Unlock:** **none active.** First `C_UNLOCK=` = **`4008e6fd` CONSUMED**.
  **Do not mint** from this pin.
- **Primary wall (updated):** still **residual-matched / product
  load-accepted** material. IRON scalar-add completed-on-FW evidence is
  real but **same small peel**; product residual identity gap remains.
- **Who acts next:** (1) larger completing-stack ctrltext (matmul IRON /
  DD/XRT post-patch **not** research-dup sha) if residual progress needed;
  (2) agent idle on empty reconfirm; (3) live product shot only under **new**
  unlock + named reason.
- **Reports:** `.agents/reports/wave5-host-scan-agent.md`;
  `.agents/reports/wave5-host-capture-scan.md`;
  `.agents/reports/path-b-vector-scalar-add-npu2-pass-20260811.log`;
  PROVENANCE Path B section.

### Prior Turn (W5-5/6/7 path docs + status helper; invent_pass=0)

- **Named offline delta (docs/tools only):** Operator load-accepted Paths
  **B** (mlir-aie/IRON completed NPU run), **C** (`xrt-smi` / XRT health ≠
  Pass), **A** (FastFlowLM/Lemonade **absent**, install-only). One page:
  `docs/LOAD_ACCEPTED_CAPTURE.md`. Extended
  `docs/CAPTURE_MATERIAL_MLIR_AIE.md` §3b + refuse re-tag of public AOT
  300/660/2592/4512. PROVENANCE links load-accepted page. Helper:
  `bash tools/load_accepted_path_status.sh` (read-only; invent_pass=0).
  invent_pass=0 · Pass=**No** · not_ran_on_npu=1 · not_c_ready=1.
- **Honesty:** AOT ≠ load-accepted; validate ≠ Pass; FLM absent; no network
  install; no live apply; no invent bins; no unlock mint.
- **Unlock:** **none active.** First `C_UNLOCK=` = **`4008e6fd` CONSUMED**.
- **Primary wall:** still **operator** completed-on-FW capture into
  `testdata/xdna2_cmd/external_known_good/`. Research peels do **not** count.
- **Who acts next:** (1) **Operator** Path B completed NPU run (or C/A).
  (2) **Agent** remaining W5 offline only if named. (3) L1 only after
  load-accepted lands.
- **Reports:** `.agents/reports/impl-w5-path-docs.md`; prior
  `.agents/reports/impl-w5-s0-pin-mirror.md`.

### Prior Turn (W5 plan approved — S0 pin + plan mirror; invent_pass=0)

- **Named delta (docs/residual only):** Campaign **W5 load-accepted capture
  automation APPROVED.** W4 offline complete. Plan mirrored to
  `.agents/plans/plan-silicon-try-everything.md` (session plan + workspace
  mirror). invent_pass=0 · Pass=**No** · not_ran_on_npu=1 · not_c_ready=1.
- **Honesty:** invent_pass=0 · Pass=**No** · active unlock=**none** · first
  `C_UNLOCK=` = **`4008e6fd` CONSUMED**. Do **not mint** from this pin alone.
  Do **not** claim Pass or `ran_on_npu`. Plan approve ≠ Pass. Mirror ≠ unlock.
- **Primary wall:** **Operator** load-accepted material into
  `testdata/xdna2_cmd/external_known_good/` (or path + `--ingest`). Public
  mlir-aie peels (300 / 660 / 2592 / cascade 4512) research-only claim=0.
  Multi-txn 32208 refused.
- **Agent next (W5 offline, no unlock):** host capture scan, extract/collect
  glue, load-accepted PROVENANCE template, Path B/C/A checklists (read-only /
  dry-run). **No** re-live CONSUMED; **no** empty reconfirm; **no** sticky
  ALLOW; **no** invent residual match / Pass.
- **L8:** still **not** justified from W4 wire alone. Live only under **new**
  host-minted unlock + named offline reason after material or new wait-path
  hypothesis.
- **Who acts next:** (1) **Operator material** (primary wall). (2) **Agent**
  W5-2…W5-9 offline tools. (3) L1 only after load-accepted lands.
- **Freeze:** re-live CONSUMED; spent flexml; invent Pass; sticky ALLOW; empty
  reconfirm; L8 mint from wire alone; re-tag research AOT as load-accepted.
- **Reports:** `.agents/reports/impl-w5-s0-pin-mirror.md`; plan
  `.agents/plans/plan-silicon-try-everything.md`; prior W4
  `.agents/reports/wave4-join.md`.

### Prior Turn (W4 join — offline complete; invent_pass=0)

- **Named offline delta:** Campaign **W4a/b/c offline complete.** Landed: host
  inventory, material/last_apply scorecards, journal scrape helper, syncobj pack
  + env-gated wait wire (`XDNA2_LAB_WAIT_VIA_SYNCOBJ`), bag discovery (all
  residual_matched_claim=0), multi-txn candidate **refused**. Join:
  `.agents/reports/wave4-join.md`.
- **Honesty:** invent_pass=0 · Pass=**No** · active unlock=**none** · first
  `C_UNLOCK=` = **`4008e6fd` CONSUMED**. Do **not mint** this pin. Do **not**
  claim Pass or `ran_on_npu`. Offline tools ≠ Pass.
- **L8:** **Not run.** Offline pack+wire **green** (ready for L8 only after a
  **named** offline reason + **new** host unlock). **Live L8 not justified**
  from wire alone (compare CONSUMED skip-CU / L5b `wait_cmd_einval`). If later
  justified: skip-CU + `XDNA2_LAB_WAIT_VIA_SYNCOBJ=1` (design only; not unlocked).
- **Material wall:** Operator **load-accepted** capture still open under
  `testdata/xdna2_cmd/external_known_good/` (or path + `--ingest`). Public
  mlir-aie peels (300 / 660 / 2592 / cascade 4512) research-only claim=0. Multi-
  txn 32208 **refused**.
- **Who acts next (primary):** **Operator material.** Agent W4 offline done.
- **Freeze:** re-live CONSUMED; spent flexml; invent Pass; sticky ALLOW; empty
  reconfirm.
- **Reports:** `.agents/reports/wave4-join.md`, `wave4-host-inventory.md`,
  `wave4-scorecard-tools.md`, `wave4-journal-tool.md`, `wave4-syncobj-pack.md`,
  `wave4-discovery.md`, `wave4-syncobj-wire.md`, `impl-s0-w4-pin-mirror.md`.

### Prior Turn (W4 campaign pin + plan mirror S0; invent_pass=0)

- **Campaign:** **W4 agent-max residual resolve APPROVED.** Plan mirrored to
  `.agents/plans/plan-silicon-try-everything.md`.
- **Honesty:** invent_pass=0 · Pass=**No** · active unlock=**none** · first
  `C_UNLOCK=` = **`4008e6fd` CONSUMED**. Do **not mint**. Do **not** claim Pass
  or `ran_on_npu`.
- **Agent offline (W4a next):** host inventory, assumption/material scorecards,
  journal tools, syncobj pack/unit tests. **No** re-live CONSUMED; **no** empty
  reconfirm; **no** sticky ALLOW.
- **Operator one ask:** put a **load-accepted** capture into
  `testdata/xdna2_cmd/external_known_good/` (or path +
  `bash tools/pass_unblock_once.sh --ingest PATH`). Public mlir-aie peels
  (300 / 660 / 2592 / cascade 4512) are **research-only** and **do not count**.
- **Reports after wave:** `.agents/reports/wave4-*.md`; S0 report
  `.agents/reports/impl-s0-w4-pin-mirror.md`.

<!-- first C_UNLOCK= line is parse SSoT for live run-stock -->
`C_UNLOCK=2c9c12e8` CONSUMED (2026-08-11 **S4–S5 live-shot-effective Arm A** product matmul + same-shot; last_apply `/tmp/dragon_npu_last_apply_live_eff_2c9c12e8.json`; status=`wait_cmd_einval`; `real_ioctl=true`; `ran_on_npu=false`; total_s≈**0.165**; `same_shot_gate=1 same_shot=1 paddrs_valid=1 source=in_process_bo_map`; config_cu absent; aiebu+rela×12; WAIT EINVAL hwctx=1 seq=0 timeout_ms=5000; invent_pass=0; **no** Pass).

**Named reason + measured outcome (`2c9c12e8`):** Plan live-shot-effective APPROVED; S0–S3 prep green. One stock Arm A product matmul with same-run address patching. **Result:** same-shot paddrs **applied** (`same_shot=1`); EXEC residual **still** `wait_cmd_einval` (same class as skip-CU `1f761bf4`). Not research PDI re-burn; not re-live CONSUMED. Unlock ≠ Pass. Reports: `.agents/reports/live-shot-2c9c12e8.md`, `.agents/reports/impl-live-shot-effective-s4-s5.md`.

Prior CONSUMED (history; do not reopen):
`C_UNLOCK=4008e6fd` CONSUMED (2026-08-11 **L5b** stock **research debug-BO Dev** + skip-CONFIG_CU; last_apply `/tmp/dragon_npu_last_apply_config_cu_821691.json`; status=`wait_cmd_einval`; `real_ioctl=true`; `ran_on_npu=false`; total_s≈**0.20**; DBG_BUF Assign **ok** bo=2 size=4096 **Dev** `attempt_ok=1` `debug_bo_ty=dev`; then WAIT EINVAL hwctx=1 seq=1 timeout_ms=5000; invent_pass=0; **no** Pass).
`C_UNLOCK=48da6e60` CONSUMED (2026-08-11 stock **research debug-BO** + skip-CONFIG_CU; last_apply `/tmp/dragon_npu_last_apply_config_cu_343412.json`; status=`debug_bo_failed`; `real_ioctl=true`; `ran_on_npu=false`; total_s≈**0.17**; CONFIG_HWCTX Assign DBG_BUF **EINVAL** bo=2 size=4096 **Shmem**; journal `aie2_hwctx_cfg_debug_bo: Get bo 2 failed`; fail-closed **before** EXEC; invent_pass=0; **no** Pass).
`C_UNLOCK=1f761bf4` CONSUMED (2026-08-11 stock **skip-CONFIG_CU isolation**; last_apply `/tmp/dragon_npu_last_apply_config_cu_4080903.json`; status=`wait_cmd_einval`; `config_cu` **absent** (`config_cu_present=0`); no `pdi_bytes`; `real_ioctl=true`; `ran_on_npu=false`; total_s≈**0.17** (vs ~62s CONFIG_CU attach shots); journal **no** IO_PAGE_FAULT / **no** `aie2_set_cmd_timeout` in shot window; invent_pass=0; **no** Pass).
`C_UNLOCK=efd44f35` CONSUMED (2026-08-11 stock flexml **unified-4x4** CONFIG_CU; last_apply `/tmp/dragon_npu_last_apply_config_cu_2618482.json`; status=`wait_cmd_einval`; pdi_bytes=**13792** sha16=`7d172bf223a9b06b`; `config_cu=1`; `real_ioctl=true`; `ran_on_npu=false`; journal **IO_PAGE_FAULT** ~`0x8fff8000…` + `aie2_set_cmd_timeout` DPU=`0xffffffff` TXN_OP=`0xd`; invent_pass=0; **no** Pass).
`C_UNLOCK=8352882b` CONSUMED (2026-08-11 stock flexml **llm-2x4x4** CONFIG_CU; last_apply `/tmp/dragon_npu_last_apply_config_cu_2328515.json`; status=`wait_cmd_einval`; pdi_bytes=**21664** sha16=`ba0cd738fd870011`; `config_cu=1`; `real_ioctl=true`; `ran_on_npu=false`; journal **IO_PAGE_FAULT** ~`0x4fff8000…` + `aie2_error_backtrack` + `aie2_set_cmd_timeout` DPU/TXN `0xffffffff`; invent_pass=0; **no** Pass).
`C_UNLOCK=2ae69036` CONSUMED (2026-08-10 stock flexml unified-2x4x4 CONFIG_CU; last_apply `/tmp/dragon_npu_last_apply_config_cu_675133.json`; status=`wait_cmd_einval`; pdi_bytes=27200 sha16=`0eeeb70c84df056e`; invent_pass=0; no ran_on_npu).

### Plain status (read this first)

| | |
|--|--|
| **What is broken** | Still no product Pass. Residual class post-attach / EXEC **`wait_cmd_einval`** (L5b Dev DBG_BUF Assign ok then WAIT EINVAL; same family as skip-CU). **FW still no** ERT Completed + OFM allclose. Pure re-live of **spent** fingerprints **banned**. Historical (2026-07-18): APP_LOAD `0x3000003`. |
| **What changed offline / live** | **W5 join closed.** Tools: scan/collect/PROVENANCE/`LOAD_ACCEPTED_CAPTURE`/path status. Agent Path B IRON `vector_scalar_add` **PASS!** on FW **1.1.2.64** (same 300 B sha `aaeb5b68…`; load_accepted re-tag refused; **stack proven working**). Path C validate archives missing (no peel). Path A FLM absent. invent_pass=0 · not_ran_on_npu=1 · not_c_ready=1. **No** unlock mint. Join: `.agents/reports/wave5-join.md`. |
| **What is still missing for Pass** | Live firmware accept + ERT Completed + non-zero OFM allclose via product `decide_ran_on_npu`. **Highest remaining / primary wall:** residual-matched / **load-accepted** material for lab classic **1688** (IRON scalar-add ≠ that identity). Public peels 300/660/2592/4512 research only. invent_pass=0. |
| **Operator tools** | **Load-accepted paths:** `docs/LOAD_ACCEPTED_CAPTURE.md` + `bash tools/load_accepted_path_status.sh`. Scan: `bash tools/host_capture_scan.sh`. Collect: `bash tools/collect_load_accepted_candidate.sh --dry-run PATH`. Ingest umbrella: `bash tools/pass_unblock_once.sh`. Journal: `bash tools/journal_npu_scrape.sh`. Path B needs system `pyxrt` on `PYTHONPATH`. **Ban** re-live CONSUMED `4008e6fd` / re-tag public AOT / spent flexml. |
| **Unlock** | **No active unlock.** First `C_UNLOCK=` = `2c9c12e8` **CONSUMED** (Arm A same-shot product live; `wait_cmd_einval`; same_shot=1 measured). Prior: `4008e6fd`, `48da6e60`, `1f761bf4`, `efd44f35`, `8352882b`, `2ae69036`, `97f38307`, `11bc9016`, `3a70fabf`, `b17b0119`. **Do not reopen.** Unlock ≠ Pass ≠ `ran_on_npu`. invent_pass=0. |
| **Prior unlocks** | `C_UNLOCK=2c9c12e8` CONSUMED (Arm A same-shot; last_apply `…live_eff_2c9c12e8.json`; wait_cmd_einval). `C_UNLOCK=4008e6fd` CONSUMED (Dev debug-BO). `C_UNLOCK=1f761bf4` CONSUMED (skip-CONFIG_CU). Others CONSUMED. |
| **Pass?** | **No.** Not product Pass. Not `ran_on_npu`. invent_pass=0. Same-shot live ≠ Pass. residual_matched_claim=0. wait_cmd_einval ≠ Pass. |
| **Who acts next** | (1) **Stop** empty same-shot reconfirm. (2) Residual-matched / load-accepted material for ERT Completed + OFM allclose. (3) Next live only under **new** unlock + **new** named reason. (4) **Frozen:** re-live CONSUMED; spent flexml; invent Pass; sticky ALLOW; empty reconfirm. invent_pass=0; Pass=**No**. |
| **Sudo?** | **Product users: no.** This stock host: live shot needs **no** sudo for `exec_cmd_allow` (sysfs missing). **Do not invent** `pass_unblock_privileged_once.sh` until a real Layer A wall. Hardened lab only: temporary `exec_cmd_allow`. See `docs/XDNA2_NEVER_AGAIN.md`. |
| **Local refuse** | `gt_capture/instr_post_patch.bin` and VE2 `validate.xclbin` are **not** accepted as the known-good AIE2 capture. Multi-txn `instr_vendor_post_patch.bin` (32208) **refused** as load-accepted. |

### Spent vs unspent research PDI ledger

Research overlays only. **Not** product known-good. invent_pass=0. Re-live of a
**spent** fingerprint alone is not progress (needs new unlock **and** a named
delta beyond “same sha16”).

| tag | pdi_bytes | sha16 (16) | Live |
|-----|-----------|------------|------|
| flexml STX `unified-2x4x4` | 27200 | `0eeeb70c84df056e` | **Spent** under `2ae69036` (`wait_cmd_einval`; not Pass) |
| flexml STX `llm-2x4x4` | 21664 | `ba0cd738fd870011` | **Spent** under `8352882b` (`wait_cmd_einval` + IO_PAGE_FAULT; not Pass) |
| flexml STX `unified-4x4` | 13792 | `7d172bf223a9b06b` | **Spent** under `efd44f35` (`wait_cmd_einval` + IO_PAGE_FAULT ~`0x8fff8000…`; not Pass) |
| CVML AIE2P 4x4 Overlay | 283840 | `a1e65d04add3422c` | **Spent** prior unlocks (orphan IO_PAGE_FAULT class) |

Fingerprint tooling: `tools/pass_unblock_fingerprint_once.sh`
(`--offline` skips spent-default re-dump unless `--force` /
`XDNA2_FP_FORCE_REDUMP=1`; ledger footer; S5 inventory) /
`tools/dump_xclbin_pdi.sh`. Inventory report:
`.agents/reports/pdi-xclbin-inventory.md`. Capture ctrltext playbook +
skip-PDI checklist: `docs/CONSTRUCTION_MAP_APP_LOAD.md`. **MLIR-AIE → capture
dir (concrete binaries, not clone):**
[`docs/CAPTURE_MATERIAL_MLIR_AIE.md`](docs/CAPTURE_MATERIAL_MLIR_AIE.md).

**Three needs (updated 2026-08-11 residual pin feedback):** (1) working-stack
ctrltext under `testdata/xdna2_cmd/external_known_good/` — **met for dual public
npu2 AOT research sets** size-class **`public-npu2-small`**: vector **300 B**
sha16 `aaeb5b68cc298a45` (primary `working_stack_ctrltext.bin`) + single_core
matmul **660 B** sha16 `de8671245bb72436`; **RESEARCH ONLY** — not residual
identity, not unlock justification alone; soft mismatch expected vs spent
**`flexml-2x4x4-class`** and lab **`mladf-classic-1688`**; material ≠ Pass ≠
product known-good; (2) true residual-matched / load-accepted image beyond spent
flexml **still open** (660 B matmul AOT is **not** that claim); Route B can cite
size-class / opcode / header / section / rela **axes** for unlock reason text,
not close residual; (3) driver/MMU orphan class **parked** (flexml `2ae69036` had
no IO_PAGE_FAULT; do not un-park without new journal).

**Size-class / topology tags (residual pin language):**

| Tag | What it names | Example sizes |
|-----|---------------|---------------|
| **`public-npu2-small`** | Public mlir-aie / IRON npu2 **minimal** AOT peels (research only; length ≤1024) | vector ctrltext **300 B**; single_core matmul ctrltext **660 B**; PDI ~2.5–5.5 KiB |
| **`public-npu2-other`** (size-class `other`) | Larger public mlir-aie npu2 AOT peels (research only; not residual match) | whole_array **2592 B**; cascade **4512 B** sha16 `18cb653c05a9d4c8` |
| **`mladf-classic-1688`** | Lab staged classic mladf ctrltext oracle (rela×12 path) | **1688 B** ctrltext |
| **`flexml-2x4x4-class`** | Flexml STX research PDI topology class (**all three Spent** 2026-08-11) | unified-2x4x4 **27200 B** spent; llm-2x4x4 **21664 B** spent; unified-4x4 **13792 B** spent |

**One-line residual note:** public research sets present (small 300/660 + other
2592/4512 cascade); still residual-mismatched to spent **`flexml-2x4x4-class`**
and to lab classic **`mladf-classic-1688`**. Cascade land ≠ residual identity.

### Creative avenues (rule-respecting; residual honesty after pin feedback 2026-08-11)

Core diagnosis **refined, not overturned**: offline tooling healthy (P0–P2 +
Route B structure tools); capture dir has public mlir-aie npu2 AOT research sets
(vector 300 + single_core 660 size-class **`public-npu2-small`**; whole_array
2592 + cascade **4512** size-class **`other`**; all RESEARCH ONLY; none
flexml-matched nor lab-1688 identity); all unlocks CONSUMED; Pass=No;
invent_pass=0; empty implement mill frozen. Wall is now **load-accepted /
residual-matched image + gated live**, not “capture folder empty” and not missing
status scripts.

**Useful change from latest flexml (`2ae69036`):** CONFIG_CU attach succeeded for
unified-2x4x4 (27200 B, sha16 `0eeeb70c…`). Residual moved from pure APP_LOAD /
orphan IO_PAGE_FAULT class to **`wait_cmd_einval` + journal cmd timeout /
backtrack**. That is a **different residual class** from the CVML orphan-fault
path and is better information. Pure re-live of the spent fingerprint is
**banned**; new live needs a named offline reason tied to **new material** or a
**deliberately different** unspent image.

**Material rule (pinned):** residual tools and soft compare need **files** under
`testdata/xdna2_cmd/external_known_good/` (or
`bash tools/pass_unblock_once.sh --ingest PATH [NAME]` with a real **PATH**).
Having mlir-aie / IRON / Peano **checked out** is setup only. **As of 2026-08-11,
four public npu2 AOT sets are in the capture dir** (RESEARCH ONLY): vector
(300 B primary, small), single_core (660 B, small), whole_array (2592 B,
other), cascade (4512 B sha16 `18cb653c…`, other). That **completes
empty-folder Step 1 for offline classify**, and **does not** complete residual
identity, flexml-matched / load-accepted image, unlock justification alone, or
Pass.
Soft tools can cite multi-axis structural distance and size-class / opcode /
section / rela feature axes (Route B) — **citable in unlock reason text without
claiming match**. Playbook:
[`docs/CAPTURE_MATERIAL_MLIR_AIE.md`](docs/CAPTURE_MATERIAL_MLIR_AIE.md).
Supporting: Peano, aiebu, AMD IRON, FastFlowLM/Lemonade fallback. Does **not**
claim Pass, mint unlock, or invent `ran_on_npu`.

**Ranked next (campaign try-everything; W0+W1 offline status pinned):**

0. **W0+W1 offline status closed (2026-08-11):** invent_pass=0; Pass=**No**;
   **no active unlock** (first `C_UNLOCK=` = `1f761bf4` **CONSUMED**). Live
   queue: **L1–L4 blocked**; **L5 after lab-path wire (in flight)**; L6 no.
   Status: `.agents/reports/campaign-try-everything-w1-status.md`. Joins:
   `wave0-join.md`, `wave1-join.md`. Plan:
   `.agents/plans/plan-silicon-try-everything.md`. **Do not mint** from this
   pin alone.
1. **Highest remaining (Pass lever):** **working-stack residual-matched /
   load-accepted program material** (live ERT Completed + non-zero OFM
   allclose via `decide_ran_on_npu`). Prefer **EXEC peel** into
   `testdata/xdna2_cmd/external_known_good/` (honest PROVENANCE). Present
   public AOT (300 / 660 / 2592 / **4512 cascade**) remains RESEARCH ONLY,
   residual-mismatched. **Cascade landed** (S7; sha16 `18cb653c…`); further
   public AOT only if **new** sha/size. FastFlowLM / Lemonade **absent**
   until operator installs. Checklist:
   `format_material_residual_match_checklist` /
   `classify_material_residual_match` (always invent_pass=0).
2. **Optional offline product (L5 prep):** finish debug-BO **lab sandwich path
   wire** (CREATE debug BO + gated `config_hwctx_dbg_buf` in lab_exec / timed
   path; invent_pass=0; **not** live apply). Research trait/gate already
   landed: `.agents/reports/wave1-debug-bo-wire.md`. L5 live only under
   **new** host-minted unlock + named reason **after** lab path is real —
   not from research wire alone.
3. **O2 closed offline (do not re-open as unlock reason alone):** driver
   EINVAL/ETIME soft map + `syncobj_handle` on `HardwareContext` / last_apply.
   Syncobj wait path **not** live-wired. Report:
   `.agents/reports/wave1-o2-wait-syncobj.md`. O2 ≠ Pass ≠ unlock mint.
4. **O5 / material closed offline:** aiebu wrap understood; fingerprint ledger
   Spent-aligned; no new capture bins. Reports: `wave1-o5-aiebu.md`,
   `wave1-material-o5.md`.
5. **Done / do not re-open as next live:** **`skip-CONFIG_CU isolation`** under
   `C_UNLOCK=1f761bf4` **CONSUMED**. Isolation stands: EXEC residual persists
   without overlay. Prior: `.agents/reports/live-shot-1f761bf4.md`.
6. **Keep frozen:** empty reconfirm; skip-CU reconfirm under CONSUMED; re-live
   spent flexml (`0eeeb70c` / `ba0cd738` / `7d172bf2`); reopen CONSUMED; MMU
   work without new journal; Pass from CONFIG_CU attach alone; Pass from capture
   bins alone; treating public AOT as residual identity or unlock alone; agent
   live without **new** unlock + **new** named offline reason; invent Pass from
   axes/inventory/aiebu wrap alone; sticky ALLOW; mint unlock for same residual
   still green offline; L4 from empty unspent-flexml ledger; L5 live without
   lab-path wire + new unlock.

**Agent-legal only (no unlock mint from this pin alone):**

- Optional debug-BO **lab sandwich path wire** offline (still invent_pass=0;
  no live apply; no unlock mint from wire alone).
- Classify/ingest on **present** capture bytes and any new human bytes
  (`pass_unblock_once --ingest PATH`, C4 IRON path).
- Use WAIT EINVAL axes / material inventory on **new** last_apply only if the
  failure mode changes (not re-spin of skip-CU or locked baseline).
- Unlock **justification text** only until host mints a **new** unlock for a
  **new** named reason.
- **Refuse:** same status scripts as progress; re-fingerprint spent default
  without `--force` / named reason; invent Pass from capture or soft mismatch;
  re-live skip-CU under `1f761bf4`; claim residual close from size-class or
  aiebu re-wrap alone.

**Does not help:** empty gate reconfirm; unlock without named FP+reason;
packaging already rejected by FW; empty implement loops; re-live spent
flexml/skip-CU alone; claiming Pass because bins are no longer zero; L4 from
spent flexml list; O2 axes alone as unlock mint; L5 from research trait alone.

**Recommended sequence:** (1) **highest** hunt working-stack residual-matched
material; (2) optional finish debug-BO lab-path wire offline; (3) new unlock
+ new named reason only when justified (L1 when residual-matched material
exists, or L5 after lab path + real reason); (4) re-interpret only after new
justified live or richer last_apply.
Campaign plan: `.agents/plans/plan-silicon-try-everything.md`.
Prior plans: `.agents/plans/plan-pass-unblock-sequence.md`,
`.agents/plans/plan-pass-unblock-creative-avenues.md`.
Pin reports: `.agents/reports/campaign-try-everything-w1-status.md`,
`.agents/reports/wave1-join.md`,
`.agents/reports/wave1-o2-wait-syncobj.md`,
`.agents/reports/wave1-debug-bo-wire.md`,
`.agents/reports/wave1-material-o5.md`,
`.agents/reports/wave1-o5-aiebu.md`,
`.agents/reports/wave0-join.md`,
`.agents/reports/wave0-prep-inventory.md`,
`.agents/reports/live-shot-1f761bf4.md`.

**Unlock (parse first `C_UNLOCK=` — SSoT is top of this file):** first is
`C_UNLOCK=2c9c12e8` **CONSUMED** (2026-08-11 Arm A same-shot live-shot-effective;
last_apply `/tmp/dragon_npu_last_apply_live_eff_2c9c12e8.json`;
status=`wait_cmd_einval`; same_shot=1; invent_pass=0; **no** Pass). Prior also
CONSUMED include `4008e6fd`, `1f761bf4`, … Unlock **≠** Pass **≠** `ran_on_npu`.
Empty reconfirm mill **frozen**. **Do not** treat older mid-file stamps as
parse-first.

**LIVE bookkeeping (recent CONSUMED; do not reopen):**
- **`C_UNLOCK=1f761bf4` CONSUMED** (2026-08-11 skip-CONFIG_CU; last_apply
  `/tmp/dragon_npu_last_apply_config_cu_4080903.json`; `wait_cmd_einval`;
  config_cu absent; reports `.agents/reports/live-shot-1f761bf4.md`,
  `.agents/reports/impl-t-skip-cu-1f761bf4-closeout.md`).
- **`C_UNLOCK=efd44f35` CONSUMED** (flexml unified-4x4 CONFIG_CU; last_apply
  `/tmp/dragon_npu_last_apply_config_cu_2618482.json`; IO_PAGE_FAULT ~`0x8fff…`).
- **`C_UNLOCK=8352882b` CONSUMED** (flexml llm-2x4x4; last_apply
  `/tmp/dragon_npu_last_apply_config_cu_2328515.json`; IO_PAGE_FAULT ~`0x4fff…`).
- **`C_UNLOCK=2ae69036` CONSUMED** (2026-08-10 stock flexml unified-2x4x4
  CONFIG_CU; last_apply `/tmp/dragon_npu_last_apply_config_cu_675133.json`;
  status=`wait_cmd_einval`; `config_cu=1` `pdi_bytes=27200`; journal
  `aie2_error_backtrack` + `aie2_set_cmd_timeout` without IO_PAGE_FAULT).
- Prior **`97f38307`** (CVML), **`11bc9016`**, **`3a70fabf`**, **`b17b0119`**
  CONSUMED. Host still stock (`exec_cmd_allow=missing`).

Capture dir **has** dual mlir-aie npu2 research bins size-class
**`public-npu2-small`** (vector 300 B sha16 `aaeb5b68cc298a45` + matmul 660 B
sha16 `de8671245bb72436`); **RESEARCH ONLY**; residual-mismatched to spent
**`flexml-2x4x4-class`** and lab **`mladf-classic-1688`**; **not** Pass; invent_pass=0.

**Named offline reason for unlock `1f761bf4` (CONSUMED):** skip-CONFIG_CU
isolation after orphan map audit + three spent flexml CONFIG_CU attach shots.
Measured: residual class still `wait_cmd_einval` without overlay. (Historical
`2ae69036` reason was flexml STX unified-2x4x4 PDI DIFF vs CVML — attach only,
not Pass.)

**Prior research stamps (index only):** `2b01425e` (capture-dir + feed + dry-run
compose); `d2e287ab` (lab export fingerprint feed); `6033158a` (ctrltext compare
dry-run); `9cdfe8d0` (progress checklist); `8e898123` … `045739e1` — see PLAN.md.
Full demoted gate-only prior bodies live in `PLAN.md`.

**Honesty (do not invent):** Latest live under **`1f761bf4`** is `wait_cmd_einval`
on skip-CONFIG_CU (final soft residual **`not_pass_no_ert_error`** on last_apply
classify; product status SSoT = JSON `status=wait_cmd_einval`); **no** product
Pass; invent_pass=0; **no** `ran_on_npu`. Prior flexml CONFIG_CU lives same class
with optional IO_PAGE_FAULT. Do **not** invent APP_LOAD class from skip-CU pin
alone (no `0x3000003` / ert_error on that last_apply). Do **not** reopen CONSUMED
unlocks (`1f761bf4`, `efd44f35`, `8352882b`, `2ae69036`, `97f38307`, `11bc9016`,
`3a70fabf`, `b17b0119`). Capture research bins **≠** Pass **≠** unlock alone.
Research PDI ≠ product known-good. VE2 xclbin ≠ AIE2 known-good.
**Hard stop risk:** without load-accepted / residual-matched material or a named
offline delta beyond dual research bins / skip-CU reconfirm, do not mint a new
unlock or re-spin empty implement.

### Prior LIVE — research helpers for capture compare (`588c1c72`)

> **Historical framing:** Unlock state **superseded** by top plain status
> (`C_UNLOCK=11bc9016` CONSUMED after 2026-08-10 live). This stamp did **not**
> mint unlock. “No second unlock” below was true **at `588c1c72` only** — do not
> read as current product unlock policy.

| Check | Result | 2nd C / Pass? |
|-------|--------|---------------|
| **Policy** | Empty reconfirm stamps **frozen**. Empty-folder implement **frozen**. `588c1c72` = named offline helpers only (bytes still absent). | **No** |
| **Silicon** | Unchanged: userspace ELF+rela OK; firmware **APP_LOAD_PDI_FAIL** `0x3000003`. | **No** |
| **Unlock (at stamp)** | `C_UNLOCK=b17b0119` **CONSUMED**. This stamp did **not** mint unlock. **Then:** no second unlock. **Current:** see top Unlock row (now **none** — all one-shots CONSUMED including flexml `2ae69036`). | **No** (then) |
| **Capture dir** | No working-stack known-good `.bin` offline under `testdata/xdna2_cmd/external_known_good/`. Soft/helpers report blocked-on-external-bytes (section names without bytes still blocked). Local post_patch refused. VE2 xclbin refused (`platform_not_aie2`). | **No** |
| **Code surface** | `AppLoadExtractBagScorecardPipeline` / `research_app_load_extract_bag_scorecard_pipeline` (+ variants) — composes extract dry-run + capture-dir check + soft status. Alone does **not** close residual. Soft always: invent Pass off, not C-ready, not `ran_on_npu`, not product known-good. | **No** (research) |
| **Docs** | CONSTRUCTION_MAP; PROVENANCE template in capture dir; this file + PLAN + AGENTS. | **No** |
| **Next work (then)** | **Human** places real bytes + PROVENANCE → then soft compare. Offline unit tests ≠ Pass ≠ unlock. Agent: **no empty implement loop**. | **No** until real bytes |

**Conclusion (historical `588c1c72`):** Offline helpers landed. Capture **bytes**
still **absent**. Residual still firmware load reject. **No** Pass. At this stamp
there was **no** second unlock; **later** P4 issued active `C_UNLOCK=3a70fabf`
(see top plain status). Do not re-spin empty implement.

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

**Highest-value next (LIVE — mirrors single-table SSoT below; P5 `68eeaaf6`):**
(1) **Operator:** one CONFIG_CU sandwich under **active** unlock in top Unlock
row → restore L3 → `bash tools/p5_post_c2_interpret.sh` → share last_apply.
Unlock ≠ Pass ≠ `ran_on_npu`. Mark unlock used only after real sandwich (not
from locked-fixture C3_SKIP). (2) **Optional:** place working-stack ctrltext
under `testdata/xdna2_cmd/external_known_good/` + PROVENANCE → soft compare.
(3) **Agent:** offline interpret only until new evidence. Canonical Priority
table: “Highest-value next (single table — single source of truth)” mid-file.
**Not** gate-only stamp. **Not** “only then new C_UNLOCK” while top Unlock is
still active. Match / bag-drop / extract / P5 alone ≠ residual closed.


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
| P6 pipeline | **Criteria ready offline** (`4520851d`); locked fixture → **`P6_NOT_PASS` / `P6_SKIP`** (≠ product Pass) |
| HITL C | **`C_UNLOCK=3a70fabf` CONSUMED** (2026-08-09 stock timeout; CONFIG_CU path); not active |
| HITL C prior | Prior `C_UNLOCK=b17b0119` **CONSUMED** (historical only; do not reopen) |
| C2 live (historical) | **Done** under `b17b0119` — lab rc=4; L3 restored; residual APP_LOAD |
| Pass? | **No** — need live ERT Completed + non-zero OFM allclose + `ran_on_npu=true` (product decide) |
| Tracks A–O | **Closed** (`a_n_closed_no_new`); no new external public known-good bytes |
| Progressive last_apply | **Landed offline** — mid-path stamps + missing-pin honesty + journal soft taxonomy |

### Highest-value next (single table — single source of truth)

| Priority | Action | Who |
|----------|--------|-----|
| **1** | **Optional live only after new host-minted unlock:** sandwich with **`--timeout-s=120`** (or ≥120), pin last_apply away from baseline, share last_apply + journal. Progressive stamps leave mid-path JSON if kill after durable stages. Unlock ≠ Pass ≠ `ran_on_npu`. **No** reopen under `3a70fabf`. | **operator** (agent does not open L3 / ALLOW) |
| **2** | Offline: after any **real** new last_apply pin, interpret with P5 + P6. True product Pass only on live decide (ERT Completed + OFM allclose). Missing pin → collect exit 2 (incomplete, not APP_LOAD invent). | agent offline |
| **3** | Optional / parallel: obtain known-good **external** ctrltext under `testdata/xdna2_cmd/external_known_good/` + PROVENANCE; soft compare (match ≠ residual closed) | agent / operator offline |
| **4** | Maintain offline gates only after named code delta; **no** empty gate-only mill stamps | agent |
| **5** | Public APP_LOAD depth **only** with **new** external public evidence. Else do **not** invent Track P+ | agent |
| **ban** | Env C spam; invent vendor / private PDI decode; L3 from agent; claim Pass; reopen under `3a70fabf` or `b17b0119`; invent Pass from journal alone; treat baseline fixture as timeout-shot evidence; empty reconfirm mill; invent Track P+ | all |

**C policy (honesty):** **`C_UNLOCK=3a70fabf` CONSUMED** (2026-08-09 stock CONFIG_CU + outer timeout). Further C needs a **new** host-minted unlock + offline delta (progressive last_apply + timeout honesty now landed). Unlock ≠ Pass ≠ permanent open ≠ `ran_on_npu`. Evidence never auto-unlocks. Unpatched CLASS A, registry BO, parity green, `CONTRACT_OK`, soft residual_class / CLASSIFY_SOFT_OK / checklist, and `OFFLINE_GATES_OK` **never** unlock C and **never** equal Pass. Agent still never opens L3.

**C policy (historical prior):** `C_UNLOCK=b17b0119` **CONSUMED** (operator C2 2026-07-18) — do **not** reopen under that prior stamp.

```bash
# Unlock CONSUMED — print-only / prep only until a NEW unlock exists:
# bash tools/operator_app_load_kit.sh a-prep
# export XDNA2_LAB_PDI_BLOB=…   # path from a-prep; optional XDNA2_LAB_CU_FUNC=0
bash tools/hitl_config_cu_once.sh prep   # sandwich print uses --timeout-s=120; unlock CONSUMED
# Operator live only after NEW unlock (not agent): stock = no L3; hardened = open/restore L3
# Prefer: bash tools/hitl_c_pass_apply.sh --assume-l3-already-open --timeout-s=120
# Offline anytime (≠ unlock / ≠ Pass) — P5 one-shot preferred after real last_apply:
bash tools/p5_post_c2_interpret.sh /path/to/shot_last_apply.json
# P6 true-Pass criteria (offline readiness; locked fixture → P6_SKIP / P6_NOT_PASS):
bash tools/p6_true_pass_check.sh
# or step-by-step:
bash tools/c3_post_c2_classify.sh
bash tools/c4_pass_check.sh
bash tools/run_offline_gates.sh
# HISTORICAL only — do NOT open L3 under b17b0119:
# (record) C_UNLOCK=b17b0119 already used
```

### Self-runnable gates (agent offline now)

**One-shot:** `bash tools/run_offline_gates.sh` → `OFFLINE_GATES_OK` (hard unit/style + soft classify/assets; **≠ C-ready**).

| Script / test | Expect (offline / LIVE `4520851d` P6) |
|---------------|----------------------------------------|
| `bash tools/run_offline_gates.sh` | **0** `hard_fail=0` soft green `OFFLINE_GATES_OK` (≠ Pass ≠ unlock ≠ `ran_on_npu`) |
| `cargo test --features amd-xdna2 --lib true_pass` | **0** P6 criteria checklist + p6 note honesty (≠ product Pass) |
| `cargo test --features amd-xdna2 --lib c4_pass` | **0** C4 evaluate_silicon_pass_evidence gates |
| `cargo test --features amd-xdna2 --lib config_cu` | **0** parse CONFIG_CU signals + checklist + fingerprint bit (P5) |
| `cargo test --features amd-xdna2 --lib fingerprint_` | **0** export + compare + corpus locks + C3 fixture FP |
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
| `bash tools/classify_last_apply_residual.sh` | **0** soft; `app_load_pdi_fail_after_aiebu` + `config_cu_present=0` + `not_c_ready=1 not_ran_on_npu=1 invent_pass=0` |
| `bash tools/c3_post_c2_classify.sh` | **0** `C3_SKIP` baseline (locked fixture; config_cu bit 0) |
| `bash tools/c4_pass_check.sh` | **0** `C4_SKIP` (`json_ran_on_npu_false`) |
| `bash tools/p6_true_pass_check.sh` | **0** `P6_NOT_PASS` / `P6_SKIP` on locked fixture; criteria tokens; invent_pass=0 |
| `bash tools/p5_post_c2_interpret.sh` | **0** soft; `P5_INTERPRET_OK` / soft warn; locked fixture → C3_SKIP + C4_SKIP + P6_SKIP |
| `bash tools/hitl_c2_once.sh --print-only` | **0**; NOTE `C_UNLOCK=3a70fabf one-shot (active)`; sandwich header **not** HISTORICAL; L3/apply lines **executable** (not only `# (record only)`). After this one-shot is used, Expect flips back to HISTORICAL (mark stamp used in Unlock row). |
| `bash tools/hitl_c_pass_apply.sh --assume-l3-already-open` when L3=N | **exit 1** (`hitl_c_rc=1` / `exit=1`); fail-closed when L3 closed (agent does not open L3) |
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

**Highest-value next (historical O/M/N; then research stamps through `588c1c72`; superseded for LIVE by P4 unlock + P5 `68eeaaf6`):** Was capture-first “only then new C_UNLOCK”; **superseded** — top Unlock row is active one-shot; next = CONFIG_CU sandwich → `p5_post_c2_interpret` (see single-table SSoT). Optional external capture remains parallel, not a gate before the active unlock.

*(Highest-value single source of truth = single Priority table under “Highest-value next (single table — single source of truth)” above — LIVE = top plain status + P5 `68eeaaf6`; not historical `588c1c72` alone.)*

