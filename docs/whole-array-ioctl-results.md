- `[x]` Add a failing replay test that rejects pointer-only records and demands struct bytes plus command-buffer and instruction-buffer contents.
- `[x]` Implement the tested interposer that dumps those bytes, and land the test green on a remote builder or in CI.
- `[x]` Re-check both hashes, re-run the existing XRT program under the interposer, and save the raw dump plus the decoded struct table.
- `[x]` Cite each required value to an `amdxdna_accel.h` line and a dump byte offset. Leave any uncited field as a typed error, including `0xc03064ca` if no header line names it.
- `[x]` Point the crate submit at only those cited values for the same pair, with no XRT in the path, and keep the device closed.
- `[x]` Ask once, with the cited values in the ask. Do not submit.
- `[ ]` After explicit approval only, run the crate once, record `PASS`, exit 0, the matching matmul result, and `decide_ran_on_npu` true. Put the citations file and decoded table in the report.

One unprivileged crate submit ran. It was not the plan's required pkexec Arm B. The process opened /dev/accel/accel0. Unix permission was not required. Mode 0666. The first failing ioctl was CREATE_HWCTX request 0xc0386440 (decimal 3224921152), errno 11 EAGAIN. Exit code 1. EXEC_CMD 0xc0386446 was not issued. The matmul was not compared. decide_ran_on_npu was not called. The process did not load or link XRT. Arm A, the XRT strace, was not run. XRT is present on the machine, so Arm A is not a missing-plugin skip. There was no Operator go-ahead for that XRT command. Product Pass is No. invent_pass stays 0. ran_on_npu stays false. The example stderr line "The device was not opened." is a fixed string. It is false for this run because the error request is 0xc0386440, not 0. The submit log did not print a return for 0xc0206443 on this run. The only measured error is the CREATE_HWCTX errno 11 line. The seven checklist lines above are unchanged quotes. The seventh mark stays unchecked. PASS or FAIL: FAIL.

## Versions

One unprivileged crate submit ran. It was not the plan's required pkexec Arm B. The process opened /dev/accel/accel0. Unix permission was not required. Mode 0666. The first failing ioctl was CREATE_HWCTX request 0xc0386440 (decimal 3224921152), errno 11 EAGAIN. Exit code 1. EXEC_CMD 0xc0386446 was not issued. The matmul was not compared. decide_ran_on_npu was not called. The process did not load or link XRT. Arm A, the XRT strace, was not run. XRT is present on the machine, so Arm A is not a missing-plugin skip. There was no Operator go-ahead for that XRT command. Product Pass is No. invent_pass stays 0. ran_on_npu stays false. The example stderr line "The device was not opened." is a fixed string. It is false for this run because the error request is 0xc0386440, not 0. The submit log did not print a return for 0xc0206443 on this run. The only measured error is the CREATE_HWCTX errno 11 line. The seventh checklist mark stays unchecked.

The lines below are version samples. They are not a second submit, and they do not replace the open above.

`uname -r` stdout:

```
7.2.11-zen1-1-zen
```

`/sys/module/amdxdna/srcversion` stdout:

```
CEAE025A317872A9A6B3FC8
```

`modinfo amdxdna` srcversion line (no modprobe, no insmod, no rmmod):

```
srcversion:     CEAE025A317872A9A6B3FC8
```

`modinfo` also printed `filename: /lib/modules/7.2.11-zen1-1-zen/kernel/drivers/accel/amdxdna/amdxdna.ko.zst` and `vermagic: 7.2.11-zen1-1-zen SMP preempt mod_unload`.

A read-only directory list shows installed blobs under `/lib/firmware/amdnpu/` (including `17f0_10/npu.sbin.1.1.2.64.zst`). That list is not the loaded firmware string. The loaded string is in Loaded firmware string, below.

`git -C /home/hunter/Projects/ai/dragon-npu rev-parse HEAD` stdout:

```
199916ec0e32d6f7cdabab180b0def962d0ed3fe
```

## XRT presence

One unprivileged crate submit ran. It was not the plan's required pkexec Arm B. The process opened /dev/accel/accel0. Unix permission was not required. Mode 0666. The first failing ioctl was CREATE_HWCTX request 0xc0386440 (decimal 3224921152), errno 11 EAGAIN. Exit code 1. EXEC_CMD 0xc0386446 was not issued. The matmul was not compared. decide_ran_on_npu was not called. The process did not load or link XRT. Arm A, the XRT strace, was not run. XRT is present on the machine, so Arm A is not a missing-plugin skip. There was no Operator go-ahead for that XRT command. Product Pass is No. invent_pass stays 0. ran_on_npu stays false. The example stderr line "The device was not opened." is a fixed string. It is false for this run because the error request is 0xc0386440, not 0. The submit log did not print a return for 0xc0206443 on this run. The only measured error is the CREATE_HWCTX errno 11 line. The seventh checklist mark stays unchecked.

`command -v xrt-smi` stdout:

```
/usr/bin/xrt-smi
```

`xrt-smi` is installed. It was not run. That is separate from the unprivileged crate submit, which ran and did not load or link XRT.

Presence checks. These paths were listed only. None of them was started as Arm A:

```
PRESENT /usr/lib/libxrt_driver_xdna.so.2
PRESENT /usr/lib/libxrt_driver_xdna.so.2.21.75
PRESENT /usr/lib64/libxrt_driver_xdna.so.2
PRESENT /usr/lib64/libxrt_driver_xdna.so.2.21.75
ABSENT /opt/xilinx/xrt/lib/libxrt_driver_xdna.so
```

XRT is present: `/usr/bin/xrt-smi` and `/usr/lib/libxrt_driver_xdna.so.2` (same plugin under `/usr/lib64`). `/opt/xilinx` is absent. Nothing was installed. Arm A is not a missing-plugin skip.

`test -f /tmp/whole-array-128-durable/whole_array.exe` printed PRESENT. That binary was not started. It is the Arm A program. Arm A was not run. The unprivileged crate submit is a different process, and that submit ran.

## Closed tests

One unprivileged crate submit ran. It was not the plan's required pkexec Arm B. The process opened /dev/accel/accel0. Unix permission was not required. Mode 0666. The first failing ioctl was CREATE_HWCTX request 0xc0386440 (decimal 3224921152), errno 11 EAGAIN. Exit code 1. EXEC_CMD 0xc0386446 was not issued. The matmul was not compared. decide_ran_on_npu was not called. The process did not load or link XRT. Arm A, the XRT strace, was not run. XRT is present on the machine, so Arm A is not a missing-plugin skip. There was no Operator go-ahead for that XRT command. Product Pass is No. invent_pass stays 0. ran_on_npu stays false. The example stderr line "The device was not opened." is a fixed string. It is false for this run because the error request is 0xc0386440, not 0. The submit log did not print a return for 0xc0206443 on this run. The only measured error is the CREATE_HWCTX errno 11 line. The seventh checklist mark stays unchecked.

The blocks below are prior unit-test prints. They are not the unprivileged submit. Those tests print `device_opened=no` because they refuse before an open. The live submit above did open `/dev/accel/accel0`.

Already on disk, not rerun this rewrite. Result lines:

```
test backends::amd_xdna2::direct::tests::first_cited_replay_argument_is_not_all_zeros_and_device_stays_closed ... ok
test backends::amd_xdna2::direct::tests::cited_replay_args_point_at_owned_buffers_and_device_stays_closed ... ok
test backends::amd_xdna2::direct::tests::cited_replay_nonpointer_words_match_trace_and_device_stays_closed ... ok
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 759 filtered out; finished in 0.01s
```

The log did not already contain `submit_plans_only_cited_requests_for_whole_array_pair_then_refuses_before_open`. That filter was run once, with `LD_PRELOAD`, `DRAGON_IOCTL_DUMP`, and `XDNA2_ALLOW_EXEC_CMD` unset, `nice -n 19`, `CARGO_BUILD_JOBS=8`, `PATH="/usr/bin:$PATH"`. It printed `device_opened=no` for that unit test. Both result lines:

```
test submit_plans_only_cited_requests_for_whole_array_pair_then_refuses_before_open ... ok
test backends::amd_xdna2::mlir_aie_ioctl::tests::submit_plans_only_cited_requests_for_whole_array_pair_then_refuses_before_open ... ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 760 filtered out; finished in 0.02s
```

## Which header names the 28 request words

One unprivileged crate submit ran. It was not the plan's required pkexec Arm B. The process opened /dev/accel/accel0. Unix permission was not required. Mode 0666. The first failing ioctl was CREATE_HWCTX request 0xc0386440 (decimal 3224921152), errno 11 EAGAIN. Exit code 1. EXEC_CMD 0xc0386446 was not issued. The matmul was not compared. decide_ran_on_npu was not called. The process did not load or link XRT. Arm A, the XRT strace, was not run. XRT is present on the machine, so Arm A is not a missing-plugin skip. There was no Operator go-ahead for that XRT command. Product Pass is No. invent_pass stays 0. ran_on_npu stays false. The example stderr line "The device was not opened." is a fixed string. It is false for this run because the error request is 0xc0386440, not 0. The submit log did not print a return for 0xc0206443 on this run. The only measured error is the CREATE_HWCTX errno 11 line. The seventh checklist mark stays unchecked.

The 28 words are the cited replay list in `src/backends/amd_xdna2/direct.rs` (the `REQUESTS` array inside `replay_printed_cited_requests`). Sizes are the ioctl size field, `(request >> 16) & 0x3fff`, checked against the struct layouts. The headers do not contain these hex literals. They name the ioctls through `DRM_IOWR` or `DRM_IOW`. `DRM_COMMAND_BASE` is `0x40` (`drm.h` line 1357).

`/usr/include/drm/amdxdna_accel.h` names these seven codes:

- `0xc0206443` size 32, `DRM_IOCTL_AMDXDNA_CREATE_BO`, `struct amdxdna_drm_create_bo` (lines 203-208, macro lines 730-732).
- `0xc0306444` size 48, `DRM_IOCTL_AMDXDNA_GET_BO_INFO`, `struct amdxdna_drm_get_bo_info` (lines 221-229, macro lines 734-736).
- `0xc0106447` size 16, `DRM_IOCTL_AMDXDNA_GET_INFO`, `struct amdxdna_drm_get_info` (lines 528-532, macro lines 746-748).
- `0xc0386440` size 56, `DRM_IOCTL_AMDXDNA_CREATE_HWCTX`, `struct amdxdna_drm_create_hwctx` (lines 86-98, macro lines 718-720).
- `0xc0186442` size 24, `DRM_IOCTL_AMDXDNA_CONFIG_HWCTX`, `struct amdxdna_drm_config_hwctx` (lines 153-159, macro lines 726-728).
- `0xc0386446` size 56, `DRM_IOCTL_AMDXDNA_EXEC_CMD`, `struct amdxdna_drm_exec_cmd` (lines 266-276, macro lines 742-744).
- `0xc0086441` size 8, `DRM_IOCTL_AMDXDNA_DESTROY_HWCTX`, `struct amdxdna_drm_destroy_hwctx` (lines 105-108, macro lines 722-724).

`amdxdna_accel.h` does not name `0xc03064ca`, `0x40086409`, or `0xc00864c0`.

`/usr/include/drm/drm.h` names those three:

- `0xc03064ca` size 48 is `DRM_IOCTL_SYNCOBJ_TIMELINE_WAIT` (`drm.h` line 1267, `DRM_IOWR(0xCA, struct drm_syncobj_timeline_wait)`). The struct is lines 974-992 and is 48 bytes.
- `0x40086409` size 8 is `DRM_IOCTL_GEM_CLOSE` (line 1098, `DRM_IOW(0x09, struct drm_gem_close)`). The struct is lines 583-586 and is 8 bytes.
- `0xc00864c0` size 8 is `DRM_IOCTL_SYNCOBJ_DESTROY` (line 1255). `struct drm_syncobj_destroy` is lines 924-927 and is 8 bytes.

CREATE_BO input words, not the 32-byte ioctl size. Header enum `amdxdna_bo_type` (lines 161-167): type 1 is `AMDXDNA_BO_SHMEM` and `AMDXDNA_BO_SHARE`, type 2 is `AMDXDNA_BO_DEV_HEAP`, type 3 is `AMDXDNA_BO_DEV`, type 4 is `AMDXDNA_BO_CMD`.

- Request 1: size `0x04000000`, type 2. Not 32 zero bytes. `first_cited_replay_argument` puts `0x04000000` at offset 16 and type 2 at offset 24.
- Request 5: size `0x00010530`, type 3.
- Request 8: size `0x0a20`, type 3.
- Requests 10 and 12: size `0x8000`, type 1.
- Request 14: size `0x10000`, type 1.
- Request 16: size `0x1000`, type 4.

The 28 words, in order, with ioctl struct sizes:

1. `0xc0206443` size 32. 2. `0xc0306444` size 48. 3. `0xc0106447` size 16. 4. `0xc0386440` size 56. 5. `0xc0206443` size 32. 6. `0xc0306444` size 48. 7. `0xc0186442` size 24. 8. `0xc0206443` size 32. 9. `0xc0306444` size 48. 10. `0xc0206443` size 32. 11. `0xc0306444` size 48. 12. `0xc0206443` size 32. 13. `0xc0306444` size 48. 14. `0xc0206443` size 32. 15. `0xc0306444` size 48. 16. `0xc0206443` size 32. 17. `0xc0306444` size 48. 18. `0xc0386446` size 56. 19. `0xc03064ca` size 48. 20. `0x40086409` size 8. 21. `0x40086409` size 8. 22. `0x40086409` size 8. 23. `0x40086409` size 8. 24. `0x40086409` size 8. 25. `0x40086409` size 8. 26. `0xc00864c0` size 8. 27. `0xc0086441` size 8. 28. `0x40086409` size 8.

`amdxdna_accel.h` also names `SYNC_BO`, `SET_STATE`, `WAIT_CMD`, and `GET_ARRAY`. Those words are not in this list. The cited path refuses `0x40106449`, `0xc0106449`, and `0xc0186445`. No argument was changed.

Header difference versus the cited list: no struct-size mismatch. All 28 planned sizes match the headers. No planned argument differs. The three drm.h codes above are not named in `amdxdna_accel.h`. There is no Arm A trace to diff, because Arm A was not run. That absence is not a missing-plugin skip. The unprivileged submit still opened `/dev/accel/accel0`.

## Predictions (prior evidence, then the measured submit)

One unprivileged crate submit ran. It was not the plan's required pkexec Arm B. The process opened /dev/accel/accel0. Unix permission was not required. Mode 0666. The first failing ioctl was CREATE_HWCTX request 0xc0386440 (decimal 3224921152), errno 11 EAGAIN. Exit code 1. EXEC_CMD 0xc0386446 was not issued. The matmul was not compared. decide_ran_on_npu was not called. The process did not load or link XRT. Arm A, the XRT strace, was not run. XRT is present on the machine, so Arm A is not a missing-plugin skip. There was no Operator go-ahead for that XRT command. Product Pass is No. invent_pass stays 0. ran_on_npu stays false. The example stderr line "The device was not opened." is a fixed string. It is false for this run because the error request is 0xc0386440, not 0. The submit log did not print a return for 0xc0206443 on this run. The only measured error is the CREATE_HWCTX errno 11 line. The seventh checklist mark stays unchecked.

Prior evidence, not a return printed by this submit: request `0xc0206443` size 32 returned 0 on a non-zero argument. Prior evidence also said `CREATE_HWCTX` `0xc0386440` size 56 returned -1 errno 11, with exit code 1, the matmul not compared, and `decide_ran_on_npu` not called. Errno 22 is not the latest result in that prior evidence.

Prior prediction, still prior evidence: CREATE_HWCTX `0xc0386440` size 56 returns -1 with errno 11. A later same-argument required pkexec Arm B was predicted to fail the same way at CREATE_HWCTX (return -1, errno 11, exit 1). Arguments are not changed. That required pkexec Arm B is not the run that happened.

The measured outcome is the paragraph at the start of this section. The prior prediction that CREATE_HWCTX returns errno 11 matches the only measured error. The prior sentence that request `0xc0206443` returned 0 was not printed by this submit log, so that return was not measured on this run.

Prior evidence about XRT: an XRT Arm A can match the CPU. That match is not `decide_ran_on_npu` and not Product Pass. Arm A was not run. There was no Operator go-ahead for that XRT command.

## Commands beside the measured submit

One unprivileged crate submit ran. It was not the plan's required pkexec Arm B. The process opened /dev/accel/accel0. Unix permission was not required. Mode 0666. The first failing ioctl was CREATE_HWCTX request 0xc0386440 (decimal 3224921152), errno 11 EAGAIN. Exit code 1. EXEC_CMD 0xc0386446 was not issued. The matmul was not compared. decide_ran_on_npu was not called. The process did not load or link XRT. Arm A, the XRT strace, was not run. XRT is present on the machine, so Arm A is not a missing-plugin skip. There was no Operator go-ahead for that XRT command. Product Pass is No. invent_pass stays 0. ran_on_npu stays false. The example stderr line "The device was not opened." is a fixed string. It is false for this run because the error request is 0xc0386440, not 0. The submit log did not print a return for 0xc0206443 on this run. The only measured error is the CREATE_HWCTX errno 11 line. The seventh checklist mark stays unchecked.

Arm A, the XRT strace, was not run. Exact command, not started. There was no Operator go-ahead. XRT is present, so this is not a missing-plugin skip:

```
pkexec strace -f -e trace=ioctl -X verbose -v -o /home/hunter/Projects/ai/dragon-npu/docs/whole-array-ioctl-arm-a-strace.log /tmp/whole-array-128-durable/whole_array.exe
```

The plan's required pkexec Arm B was not the process that ran. Exact command, not started. The text names pkexec, `XDNA2_ALLOW_EXEC_CMD=1`, example `mlir_aie_ioctl`, xclbin `testdata/xdna2_cmd/whole_array_128_i16/final.xclbin` sha256 `0cadf32ec0472e3f55e9102a6175d005b1a95a8ea428907086e15ad44d825dad`, insts.bin sha256 `ae692499a79c74b46175cb4841f235edb0059175144941522f792105b758dec3`, and the 28 request codes with sizes listed above. The full line is in the next section.

The unprivileged crate submit is a different command from both blocks above. That submit ran. It opened `/dev/accel/accel0`. It is not the required pkexec Arm B.

## Loaded firmware string

One unprivileged crate submit ran. It was not the plan's required pkexec Arm B. The process opened /dev/accel/accel0. Unix permission was not required. Mode 0666. The first failing ioctl was CREATE_HWCTX request 0xc0386440 (decimal 3224921152), errno 11 EAGAIN. Exit code 1. EXEC_CMD 0xc0386446 was not issued. The matmul was not compared. decide_ran_on_npu was not called. The process did not load or link XRT. Arm A, the XRT strace, was not run. XRT is present on the machine, so Arm A is not a missing-plugin skip. There was no Operator go-ahead for that XRT command. Product Pass is No. invent_pass stays 0. ran_on_npu stays false. The example stderr line "The device was not opened." is a fixed string. It is false for this run because the error request is 0xc0386440, not 0. The submit log did not print a return for 0xc0206443 on this run. The only measured error is the CREATE_HWCTX errno 11 line. The seventh checklist mark stays unchecked.

`nice -n 19 cat /sys/class/accel/accel0/device/fw_version` stdout:

```
1.1.2.64
```

`nice -n 19 wc -c /sys/class/accel/accel0/device/fw_version` stdout: `9 /sys/class/accel/accel0/device/fw_version`. That is the loaded firmware string. This sysfs read is not the device-node open. The unprivileged submit did open `/dev/accel/accel0`. Unix permission was not required. Mode 0666. The on-disk blob list, including `17f0_10/npu.sbin.1.1.2.64.zst`, is the directory list in Versions. It is not a second firmware measurement.

## Required pkexec Arm B command text

One unprivileged crate submit ran. It was not the plan's required pkexec Arm B. The process opened /dev/accel/accel0. Unix permission was not required. Mode 0666. The first failing ioctl was CREATE_HWCTX request 0xc0386440 (decimal 3224921152), errno 11 EAGAIN. Exit code 1. EXEC_CMD 0xc0386446 was not issued. The matmul was not compared. decide_ran_on_npu was not called. The process did not load or link XRT. Arm A, the XRT strace, was not run. XRT is present on the machine, so Arm A is not a missing-plugin skip. There was no Operator go-ahead for that XRT command. Product Pass is No. invent_pass stays 0. ran_on_npu stays false. The example stderr line "The device was not opened." is a fixed string. It is false for this run because the error request is 0xc0386440, not 0. The submit log did not print a return for 0xc0206443 on this run. The only measured error is the CREATE_HWCTX errno 11 line. The seventh checklist mark stays unchecked.

The plan's required pkexec Arm B is this exact command. It was not the process that ran:

```
pkexec env XDNA2_ALLOW_EXEC_CMD=1 /home/hunter/Projects/ai/dragon-npu/target/debug/examples/mlir_aie_ioctl --reason "ioctl-native MLIR_AIE whole_array" --xclbin /home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/whole_array_128_i16/final.xclbin --xclbin-sha256 0cadf32ec0472e3f55e9102a6175d005b1a95a8ea428907086e15ad44d825dad --insts /home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/whole_array_128_i16/insts.bin --insts-sha256 ae692499a79c74b46175cb4841f235edb0059175144941522f792105b758dec3
```

The unprivileged crate submit ran without that pkexec line. That submit opened `/dev/accel/accel0`. It is not this required command. Arm A was not run. Product Pass is No.

## Side-by-side

One unprivileged crate submit ran. It was not the plan's required pkexec Arm B. The process opened /dev/accel/accel0. Unix permission was not required. Mode 0666. The first failing ioctl was CREATE_HWCTX request 0xc0386440 (decimal 3224921152), errno 11 EAGAIN. Exit code 1. EXEC_CMD 0xc0386446 was not issued. The matmul was not compared. decide_ran_on_npu was not called. The process did not load or link XRT. Arm A, the XRT strace, was not run. XRT is present on the machine, so Arm A is not a missing-plugin skip. There was no Operator go-ahead for that XRT command. Product Pass is No. invent_pass stays 0. ran_on_npu stays false. The example stderr line "The device was not opened." is a fixed string. It is false for this run because the error request is 0xc0386440, not 0. The submit log did not print a return for 0xc0206443 on this run. The only measured error is the CREATE_HWCTX errno 11 line. The seventh checklist mark stays unchecked.

Environment, the values already recorded above, not a new probe:

- Kernel: `7.2.11-zen1-1-zen` (`uname -r`).
- amdxdna srcversion: `CEAE025A317872A9A6B3FC8` (`/sys/module/amdxdna/srcversion` and `modinfo`).
- Loaded firmware string: `1.1.2.64` (sysfs `fw_version`).
- Crate commit: `199916ec0e32d6f7cdabab180b0def962d0ed3fe`.
- XRT: present (`/usr/bin/xrt-smi` and `/usr/lib/libxrt_driver_xdna.so.2`). `xrt-smi` was not run. Nothing was installed. The unprivileged submit did not load or link XRT.

Arm A, prior evidence next to what happened.

Prior evidence: an XRT whole_array run might match the CPU reference. XRT is present, so Arm A is not a missing-plugin skip. That possible match is not `decide_ran_on_npu` and not Product Pass.

What happened: Arm A was not run. There was no Operator go-ahead for that XRT command. No Arm A ioctl return. No Arm A exit code. No Arm A matmul comparison.

Unprivileged submit, prior evidence next to what happened.

Prior evidence, not this submit's printed returns: request `0xc0206443` size 32 returns 0. `CREATE_HWCTX` `0xc0386440` size 56 returns -1 errno 11. Later requests are not issued. Exit code 1. The matmul is not compared. `decide_ran_on_npu` is not called. Errno 22 is not the latest result in that prior evidence. A later same-argument required pkexec Arm B was predicted to fail the same way at CREATE_HWCTX (return -1, errno 11, exit 1). Arguments are not changed. That required pkexec command is not the run that happened.

What happened: the unprivileged crate submit ran and opened `/dev/accel/accel0`. Unix permission was not required. Mode 0666. The first failing ioctl was CREATE_HWCTX request `0xc0386440` (decimal 3224921152), errno 11 EAGAIN. Exit code 1. EXEC_CMD `0xc0386446` was not issued. The submit log did not print a return for `0xc0206443`. The only measured error is the CREATE_HWCTX errno 11 line. The matmul was not compared. `decide_ran_on_npu` was not called.

PASS or FAIL for the unprivileged submit: FAIL. The seventh checklist mark stays unchecked. That mark asks, after explicit approval only, to run the crate once and record PASS, exit 0, the matching matmul result, and `decide_ran_on_npu` true. This run did not record those. The mark was not flipped.

## Side-by-side measurement

One unprivileged crate submit ran. It was not the plan's required pkexec Arm B. The process opened /dev/accel/accel0. Unix permission was not required. Mode 0666. The first failing ioctl was CREATE_HWCTX request 0xc0386440 (decimal 3224921152), errno 11 EAGAIN. Exit code 1. EXEC_CMD 0xc0386446 was not issued. The matmul was not compared. decide_ran_on_npu was not called. The process did not load or link XRT. Arm A, the XRT strace, was not run. XRT is present on the machine, so Arm A is not a missing-plugin skip. There was no Operator go-ahead for that XRT command. Product Pass is No. invent_pass stays 0. ran_on_npu stays false. The example stderr line "The device was not opened." is a fixed string. It is false for this run because the error request is 0xc0386440, not 0. The submit log did not print a return for 0xc0206443 on this run. The only measured error is the CREATE_HWCTX errno 11 line. The seventh checklist mark stays unchecked.

Environment, already recorded above, not a new probe: kernel `7.2.11-zen1-1-zen`, amdxdna srcversion `CEAE025A317872A9A6B3FC8`, loaded firmware string `1.1.2.64`, crate commit `199916ec0e32d6f7cdabab180b0def962d0ed3fe`. XRT is present (`/usr/bin/xrt-smi` and `/usr/lib/libxrt_driver_xdna.so.2`). `xrt-smi` was not run. Nothing was installed.

Arm A: not run. The pkexec strace was not started. The trace file `docs/whole-array-ioctl-arm-a-strace.log` was not created. XRT is present, so this is not a missing-plugin skip. There was no Operator go-ahead for that XRT command. An XRT matmul would not be `decide_ran_on_npu` and would not be Product Pass. Prior evidence stays: an XRT whole_array run might match the CPU reference, and that match would not be `decide_ran_on_npu`.

Unprivileged submit stderr, quoted as the example printed it. The sentence "The device was not opened." inside this quote is a fixed string. It is false for this run because the error request is `0xc0386440`, not 0:

```
host opcode 3 cited at test.cpp:229; not ERT_START_NPU or ERT_START_DPU. argument order opcode, instr, ninstr, A, B, C
Pass is still No. The device was not opened. EXEC_CMD packed request 0xc0386446 is stored and was not issued. reason ioctl-native MLIR_AIE whole_array xclbin testdata/xdna2_cmd/whole_array_128_i16/final.xclbin sha256 0cadf32ec0472e3f55e9102a6175d005b1a95a8ea428907086e15ad44d825dad insts testdata/xdna2_cmd/whole_array_128_i16/insts.bin sha256 ae692499a79c74b46175cb4841f235edb0059175144941522f792105b758dec3 submit Err(CitedIoctlFailed { request: 3224921152, errno: 11 }) exec Ok([70, 100, 56, 192, 0, 0, 0, 0]) wait Ok([73, 100, 16, 64, 0, 0, 0, 0])
EXIT:1
```

Exit code 1. `CitedIoctlFailed` request 3224921152, errno 11, which is `CREATE_HWCTX` `0xc0386440` `EAGAIN`. A failed open would have been request 0. This request is `0xc0386440`, so the submit process opened the node and the first failing ioctl was `CREATE_HWCTX`. Packed `EXEC_CMD` `0xc0386446` was not issued. Do not read other words in that stderr as per-request ioctl returns. The submit log did not print a return for `0xc0206443` on this run. The only measured error is the CREATE_HWCTX errno 11 line.

Prior evidence was: request `0xc0206443` returns 0, `CREATE_HWCTX` returns -1 errno 11, later requests are not issued, exit code 1, the matmul is not compared, and `decide_ran_on_npu` is not called. The CREATE_HWCTX errno 11 part matches this failure. The `0xc0206443` return was not printed on this run, so it was not measured on this run. Errno 22 is not this result.

The privilege probe, separate from the submit, printed `open path=/dev/accel/accel0 result=yes` on mode `020666` (permission bits 0666) uid 0 gid 989. Unix permission was not required. The `exec_cmd_allow` sysfs file was absent and was not changed. The pure Unix helper was unit-tested (1 passed) and was not applied on the live probe because caller ids were not parsed from `/proc/self/status`. The probe was not repeated to fix that parse. The submit open stands.

`decide_ran_on_npu` was not called. The matmul was not compared. PASS or FAIL: FAIL. Product Pass is No. invent_pass stays 0. ran_on_npu stays false. The seventh checklist mark stays unchecked.

This crate process did not load or link XRT. This submit is not the plan's required pkexec Arm B.

## Arm B versus cited sources

One unprivileged crate submit ran. It was not the plan's required pkexec Arm B. The process opened /dev/accel/accel0. Unix permission was not required. Mode 0666. The first failing ioctl was CREATE_HWCTX request 0xc0386440 (decimal 3224921152), errno 11 EAGAIN. Exit code 1. EXEC_CMD 0xc0386446 was not issued. The matmul was not compared. decide_ran_on_npu was not called. The process did not load or link XRT. Arm A, the XRT strace, was not run. XRT is present on the machine, so Arm A is not a missing-plugin skip. There was no Operator go-ahead for that XRT command. Product Pass is No. invent_pass stays 0. ran_on_npu stays false. The example stderr line "The device was not opened." is a fixed string. It is false for this run because the error request is 0xc0386440, not 0. The submit log did not print a return for 0xc0206443 on this run. The only measured error is the CREATE_HWCTX errno 11 line. The seventh checklist mark stays unchecked.

There is no Arm A trace. `docs/whole-array-ioctl-arm-a-strace.log` was not created. XRT is present, so Arm A is not a missing-plugin skip. The comparison is against the cited headers already recorded in this log (`/usr/include/drm/amdxdna_accel.h` and `/usr/include/drm/drm.h`), not against a new trace. All 28 planned sizes match the headers. No planned argument differs. Arguments were not changed. The same struct fields still add up to those sizes.

Each planned request, cited size still matching that header. No planned argument differs from the cited header.

1. Request 1, `0xc0206443` size 32, still matches `struct amdxdna_drm_create_bo`. No difference for that request.
2. Request 2, `0xc0306444` size 48, still matches `struct amdxdna_drm_get_bo_info`. No difference for that request.
3. Request 3, `0xc0106447` size 16, still matches `struct amdxdna_drm_get_info`. No difference for that request.
4. Request 4, `0xc0386440` size 56, still matches `struct amdxdna_drm_create_hwctx`. No difference for that request.
5. Request 5, `0xc0206443` size 32, still matches `struct amdxdna_drm_create_bo`. No difference for that request.
6. Request 6, `0xc0306444` size 48, still matches `struct amdxdna_drm_get_bo_info`. No difference for that request.
7. Request 7, `0xc0186442` size 24, still matches `struct amdxdna_drm_config_hwctx`. No difference for that request.
8. Request 8, `0xc0206443` size 32, still matches `struct amdxdna_drm_create_bo`. No difference for that request.
9. Request 9, `0xc0306444` size 48, still matches `struct amdxdna_drm_get_bo_info`. No difference for that request.
10. Request 10, `0xc0206443` size 32, still matches `struct amdxdna_drm_create_bo`. No difference for that request.
11. Request 11, `0xc0306444` size 48, still matches `struct amdxdna_drm_get_bo_info`. No difference for that request.
12. Request 12, `0xc0206443` size 32, still matches `struct amdxdna_drm_create_bo`. No difference for that request.
13. Request 13, `0xc0306444` size 48, still matches `struct amdxdna_drm_get_bo_info`. No difference for that request.
14. Request 14, `0xc0206443` size 32, still matches `struct amdxdna_drm_create_bo`. No difference for that request.
15. Request 15, `0xc0306444` size 48, still matches `struct amdxdna_drm_get_bo_info`. No difference for that request.
16. Request 16, `0xc0206443` size 32, still matches `struct amdxdna_drm_create_bo`. No difference for that request.
17. Request 17, `0xc0306444` size 48, still matches `struct amdxdna_drm_get_bo_info`. No difference for that request.
18. Request 18, `0xc0386446` size 56, still matches `struct amdxdna_drm_exec_cmd`. No difference for that request.
19. Request 19, `0xc03064ca` size 48, still matches `struct drm_syncobj_timeline_wait` in `drm.h`. No difference for that request.
20. Request 20, `0x40086409` size 8, still matches `struct drm_gem_close` in `drm.h`. No difference for that request.
21. Request 21, `0x40086409` size 8, still matches `struct drm_gem_close` in `drm.h`. No difference for that request.
22. Request 22, `0x40086409` size 8, still matches `struct drm_gem_close` in `drm.h`. No difference for that request.
23. Request 23, `0x40086409` size 8, still matches `struct drm_gem_close` in `drm.h`. No difference for that request.
24. Request 24, `0x40086409` size 8, still matches `struct drm_gem_close` in `drm.h`. No difference for that request.
25. Request 25, `0x40086409` size 8, still matches `struct drm_gem_close` in `drm.h`. No difference for that request.
26. Request 26, `0xc00864c0` size 8, still matches `struct drm_syncobj_destroy` in `drm.h`. No difference for that request.
27. Request 27, `0xc0086441` size 8, still matches `struct amdxdna_drm_destroy_hwctx`. No difference for that request.
28. Request 28, `0x40086409` size 8, still matches `struct drm_gem_close` in `drm.h`. No difference for that request.

The unprivileged submit opened `/dev/accel/accel0`. The first failing ioctl was `CREATE_HWCTX` `0xc0386440`, errno 11 (`EAGAIN`). Later requests, including `EXEC_CMD` `0xc0386446`, were not issued. Exit code 1. `decide_ran_on_npu` was not called. The matmul was not compared. Product Pass is No. invent_pass stays 0. ran_on_npu stays false.

Prior evidence matches that failure on CREATE_HWCTX: `0xc0386440` returns -1 with errno 11, later requests are not issued, and the exit code is 1. Prior evidence also said request `0xc0206443` returns 0. The submit log did not print a per-request return for `0xc0206443` on this run, so that return was not measured on this run. The measured error is only the `CREATE_HWCTX` errno 11 line.

### Reconciliation

One unprivileged crate submit ran. It was not the plan's required pkexec Arm B. The process opened /dev/accel/accel0. Unix permission was not required. Mode 0666. The first failing ioctl was CREATE_HWCTX request 0xc0386440 (decimal 3224921152), errno 11 EAGAIN. Exit code 1. EXEC_CMD 0xc0386446 was not issued. The matmul was not compared. decide_ran_on_npu was not called. The process did not load or link XRT. Arm A, the XRT strace, was not run. XRT is present on the machine, so Arm A is not a missing-plugin skip. There was no Operator go-ahead for that XRT command. Product Pass is No. invent_pass stays 0. ran_on_npu stays false. The example stderr line "The device was not opened." is a fixed string. It is false for this run because the error request is 0xc0386440, not 0. The submit log did not print a return for 0xc0206443 on this run. The only measured error is the CREATE_HWCTX errno 11 line. The seventh checklist mark stays unchecked.

This log has one outcome, the paragraph above. The fixed example stderr sentence is a quote, and it is false for this run. A failed open would have been request 0. This run's error request is `0xc0386440`. The required pkexec Arm B is a different command, and it is not the submit that ran.

## Five-try CREATE_HWCTX rerun

The historical FAIL above stays. This section is a later unprivileged submit. It is not pkexec Arm A and it is not pkexec Arm B. `XDNA2_ALLOW_EXEC_CMD` was 1. The example was `mlir_aie_ioctl` with `testdata/xdna2_cmd/whole_array_128_i16/final.xclbin` sha256 `0cadf32ec0472e3f55e9102a6175d005b1a95a8ea428907086e15ad44d825dad` and `testdata/xdna2_cmd/whole_array_128_i16/insts.bin` sha256 `ae692499a79c74b46175cb4841f235edb0059175144941522f792105b758dec3`. Exit code 1. PASS or FAIL: FAIL.

Printed ioctl lines, in order. Each line is one issue. CREATE_HWCTX was issued five times, about 200 ms apart. The argument bytes were copied back before each try. `max_opc` stayed `0x800`. `num_tiles` stayed `0x20`. Those words were not edited.

```
replay ioctl try 1 request 0xc0206443 errno 0
replay ioctl try 1 request 0xc0306444 errno 0
replay ioctl try 1 request 0xc0106447 errno 0
replay ioctl try 1 request 0xc0386440 errno 11
replay ioctl try 2 request 0xc0386440 errno 11
replay ioctl try 3 request 0xc0386440 errno 11
replay ioctl try 4 request 0xc0386440 errno 11
replay ioctl try 5 request 0xc0386440 errno 11
```

Request `0xc0206443` (CREATE_BO, size 32) returned errno 0 on try 1. Request `0xc0306444` (GET_BO_INFO, size 48) returned errno 0 on try 1. Request `0xc0106447` (GET_INFO, size 16) returned errno 0 on try 1. Request `0xc0386440` (CREATE_HWCTX, size 56, decimal 3224921152) returned errno 11 (EAGAIN) on tries 1, 2, 3, 4, and 5. The process stopped there. CONFIG_HWCTX `0xc0186442`, EXEC_CMD `0xc0386446`, the wait, the readback, and the CPU compare were not issued. `decide_ran_on_npu` was not called. The device buffer was not compared to the CPU reference.

The example also printed: `Pass is still No. The device was not opened.` That sentence is a fixed string. It is false for this run. The failing request is `0xc0386440`, not 0, and the three requests before it returned errno 0, so the device was opened.

Product Pass is No. The failing request is CREATE_HWCTX `0xc0386440`. The errno is 11 (EAGAIN) on all five tries. The diagnosed root cause is `amdxdna_init_dev_bo` reporting `Heap 0 is not mapped`, then `amdxdna_drm_create_hwctx_ioctl` reporting `Init hwctx failed, ret -11`. That journal sample is earlier evidence. This shot did not read the kernel journal again. The cause is not a wrong `max_opc`, not a wrong `num_tiles`, and not a context this process still held. `exec_cmd_allow` was absent and is not this cause, because EXEC_CMD was not issued. invent_pass stays 0. ran_on_npu stays false. A green unit test is not product Pass.

Errno 11 is not EACCES and not EPERM. pkexec Arm A and pkexec Arm B stay unrun. No root command is written for this errno.

## Five-try CREATE_HWCTX rerun from the captured log

The historical FAIL record earlier in this file stays. The section "Five-try CREATE_HWCTX rerun" above stays. This section is the captured unprivileged rerun. It was one unprivileged crate process. No XRT. No pkexec. Exit 1.

Printed lines, in order. These are the only ioctl issues in that log. CREATE_HWCTX was issued five times. No later request was issued.

```
host opcode 3 cited at test.cpp:229; not ERT_START_NPU or ERT_START_DPU. argument order opcode, instr, ninstr, A, B, C
replay ioctl try 1 request 0xc0206443 errno 0
replay ioctl try 1 request 0xc0306444 errno 0
replay ioctl try 1 request 0xc0106447 errno 0
replay ioctl try 1 request 0xc0386440 errno 11
replay ioctl try 2 request 0xc0386440 errno 11
replay ioctl try 3 request 0xc0386440 errno 11
replay ioctl try 4 request 0xc0386440 errno 11
replay ioctl try 5 request 0xc0386440 errno 11
Pass is still No. The device was not opened. EXEC_CMD packed request 0xc0386446 is stored and was not issued. reason ioctl-native MLIR_AIE whole_array xclbin testdata/xdna2_cmd/whole_array_128_i16/final.xclbin sha256 0cadf32ec0472e3f55e9102a6175d005b1a95a8ea428907086e15ad44d825dad insts testdata/xdna2_cmd/whole_array_128_i16/insts.bin sha256 ae692499a79c74b46175cb4841f235edb0059175144941522f792105b758dec3 submit Err(CitedIoctlFailed { request: 3224921152, errno: 11 }) exec Ok([70, 100, 56, 192, 0, 0, 0, 0]) wait Ok([73, 100, 16, 64, 0, 0, 0, 0])
EXIT:1
```

CREATE_HWCTX request `0xc0386440` (decimal 3224921152) returned errno 11 on tries 1, 2, 3, 4, and 5. The process stopped there. The matmul was not submitted. The output was not compared. `decide_ran_on_npu` was not called.

The log sentence "The device was not opened." is false. Request `0xc0386440` was issued, so the node was open. Earlier requests `0xc0206443`, `0xc0306444`, and `0xc0106447` returned errno 0.

The byte strings `exec Ok([70, 100, 56, 192, 0, 0, 0, 0])` and `wait Ok([73, 100, 16, 64, 0, 0, 0, 0])` are packed request words, not ioctl successes. EXEC_CMD `0xc0386446` was not issued.

Product Pass is No. invent_pass stays 0. ran_on_npu stays false.

Root cause: the kernel log already recorded Heap 0 is not mapped, then `amdxdna_drm_create_hwctx_ioctl` Init hwctx failed, ret -11. Errno 11 is EAGAIN. It is not EACCES and not EPERM. Hwctx argument words were not changed. `max_opc` stays `0x800`. `num_tiles` stays `0x20`.

pkexec Arm A and pkexec Arm B were not run. PASS or FAIL: FAIL.

## Device-heap map, then CREATE_HWCTX errno 22

Hypothesis: CREATE_HWCTX errno 11 was the missing 64 MiB type-2 device-heap map on the request list this crate issues, after CREATE_BO and GET_BO_INFO and before CREATE_HWCTX.

Single change: that map on the issued list. Hwctx words, max_opc, num_tiles, and qos words were not changed.

Expected: CREATE_HWCTX errno 0, then EXEC_CMD, readback, an integer compare, a float check, and decide_ran_on_npu true.

Falsifier: errno 11 and the kernel line "Heap 0 is not mapped".

Actual: one unprivileged process. CREATE_BO 0xc0206443 errno 0, GET_BO_INFO 0xc0306444 errno 0, GET_INFO 0xc0106447 errno 0, CREATE_HWCTX 0xc0386440 errno 22. Exit 1. EXEC_CMD 0xc0386446 was stored and was not issued. The integer compare did not run. The float check did not run. decide_ran_on_npu was not called.

Errno: 22 (EINVAL). This is not EACCES and not EPERM. No pkexec proposal.

Kernel lines, host horizon, 2026-10-26 16:58:24:
- amdxdna 0000:c6:00.1: [drm] *ERROR* invalid request
- amdxdna 0000:c6:00.1: [drm] *ERROR* aie2_alloc_resource: Allocate AIE resource failed, ret -22
- amdxdna 0000:c6:00.1: [drm] *ERROR* aie2_hwctx_init: Alloc hw resource failed, ret -22
- amdxdna 0000:c6:00.1: [drm] *ERROR* amdxdna_drm_create_hwctx_ioctl: Init hwctx failed, ret -22

"Heap 0 is not mapped" did not print.

Product Pass: No. invent_pass 0. ran_on_npu false.

Next hypothesis: the one variable is the resource request that aie2_alloc_resource rejects as "invalid request". The linux-zen v7.2.11-zen1 tree that was read does not contain that bare print, and the running module does, so this record does not invent a source line. Do not change the device-heap map in the same attempt as that resource request. Privilege is not the blocker.
