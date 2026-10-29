# Whole-array privilege probe

This user opened `/dev/accel/accel0` without pkexec, sudo, or su. The unprivileged probe opened the node once, issued no ioctl, and closed it. A later crate submit also opened the node, then stopped on `CREATE_HWCTX` with errno 11. Errno 11 is `EAGAIN` (`/usr/include/asm-generic/errno-base.h` line 15: `#define EAGAIN 11`). That is not `EACCES` (13) and not `EPERM` (1). Privilege is not required to open the node. Product Pass is No. `invent_pass` stays 0. `decide_ran_on_npu` was not printed and was not called.

The kernel parameter file `/sys/module/amdxdna/parameters/exec_cmd_allow` was absent. A later `test -e` printed `SYSFS_EXISTS:no`. The value was not 0 and was not any other integer, because the file was not there. The kernel exec gate was not changed.

## Unit test

Command:

```
env -u LD_PRELOAD -u DRAGON_IOCTL_DUMP -u XDNA2_ALLOW_EXEC_CMD nice -n 19 env CARGO_BUILD_JOBS=8 PATH="/usr/bin:$PATH" cargo test --features amd-xdna2 --lib unix_open_read_write_allowed -- --nocapture
```

Shell status: exit 0.

Quoted result:

```
running 1 test
test backends::unix_open::tests::unix_open_read_write_allowed_by_mode_owner_and_group ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 762 filtered out; finished in 0.00s
```

The log also contains 11 compile warnings in `direct.rs` and `mlir_aie_ioctl.rs`. The named test was not changed. The asserts were not weakened.

## Probe build and one open

Build command:

```
env -u LD_PRELOAD -u DRAGON_IOCTL_DUMP -u XDNA2_ALLOW_EXEC_CMD nice -n 19 env CARGO_BUILD_JOBS=8 PATH="/usr/bin:$PATH" cargo build --example accel_privilege_probe
```

Build shell status: exit 0. Last line: `Finished dev profile [unoptimized + debuginfo] target(s) in 1.72s`.

Run command, once:

```
env -u LD_PRELOAD -u DRAGON_IOCTL_DUMP -u XDNA2_ALLOW_EXEC_CMD target/debug/examples/accel_privilege_probe
```

Probe shell status: exit 0.

Quoted stdout:

```
stat path=/dev/accel/accel0 mode=020666 uid=0 gid=989
command program=id args=[] status=exit status: 0 stdout=uid=1000(hunter) gid=1000(hunter) groups=1000(hunter),3(sys),96(scanner),108(vboxusers),209(cups),958(sambashare),969(realtime),985(video),989(render),991(lp),994(input),998(wheel) stderr=
command program=id args=["-G"] status=exit status: 0 stdout=1000 3 96 108 209 958 969 985 989 991 994 998 stderr=
command program=groups args=[] status=exit status: 0 stdout=hunter sys scanner vboxusers cups sambashare realtime video render lp input wheel stderr=
CapEff=absent
exec_cmd_allow path=/sys/module/amdxdna/parameters/exec_cmd_allow present=no
allowed=not-computed caller ids missing from /proc/self/status
open path=/dev/accel/accel0 result=yes closed=yes errno=none
Unix permission is not required to open the node.
```

Stderr was empty. `CapEff` was absent and the pure Unix check was not computed, because the probe did not find caller ids in `/proc/self/status`. The open still returned yes. Mode `020666` is a character device with mode `0666`. Group 989 is `render`, and `id -G` includes 989. The probe was not run a second time.

## One crate submit

The probe printed `open path=/dev/accel/accel0 result=yes`, so one submit was run. It was not retried.

Build command:

```
env -u LD_PRELOAD -u DRAGON_IOCTL_DUMP nice -n 19 env CARGO_BUILD_JOBS=8 PATH="/usr/bin:$PATH" cargo build --example mlir_aie_ioctl --features amd-xdna2,xdna2-dangerous-exec-cmd
```

Build shell status: exit 0. Last line: `Finished dev profile [unoptimized + debuginfo] target(s) in 1.52s`.

Run command, once, with `XDNA2_ALLOW_EXEC_CMD=1` only on that process:

```
env -u LD_PRELOAD -u DRAGON_IOCTL_DUMP XDNA2_ALLOW_EXEC_CMD=1 target/debug/examples/mlir_aie_ioctl --reason "ioctl-native MLIR_AIE whole_array" --xclbin testdata/xdna2_cmd/whole_array_128_i16/final.xclbin --xclbin-sha256 0cadf32ec0472e3f55e9102a6175d005b1a95a8ea428907086e15ad44d825dad --insts testdata/xdna2_cmd/whole_array_128_i16/insts.bin --insts-sha256 ae692499a79c74b46175cb4841f235edb0059175144941522f792105b758dec3
```

Example exit code: 1.

Quoted stdout and stderr (the example writes these lines to stderr):

```
host opcode 3 cited at test.cpp:229; not ERT_START_NPU or ERT_START_DPU. argument order opcode, instr, ninstr, A, B, C
Pass is still No. The device was not opened. EXEC_CMD packed request 0xc0386446 is stored and was not issued. reason ioctl-native MLIR_AIE whole_array xclbin testdata/xdna2_cmd/whole_array_128_i16/final.xclbin sha256 0cadf32ec0472e3f55e9102a6175d005b1a95a8ea428907086e15ad44d825dad insts testdata/xdna2_cmd/whole_array_128_i16/insts.bin sha256 ae692499a79c74b46175cb4841f235edb0059175144941522f792105b758dec3 submit Err(CitedIoctlFailed { request: 3224921152, errno: 11 }) exec Ok([70, 100, 56, 192, 0, 0, 0, 0]) wait Ok([73, 100, 16, 64, 0, 0, 0, 0])
EXIT:1
```

`printf '%x' 3224921152` printed `request_hex=c0386440`. That is `CREATE_HWCTX`. Errno 11 is `EAGAIN`, not a privilege failure.

The sentence "The device was not opened" is a fixed string in `examples/mlir_aie_ioctl.rs`. The error request is not 0. In `send_cited_whole_array_device_once`, a failed `libc::open` returns request 0. This run returned request `0xc0386440`, so the submit process opened `/dev/accel/accel0` and the first failing ioctl was `CREATE_HWCTX`. The replay returns on that first failure and then closes the fd. Packed `EXEC_CMD` `0xc0386446` is later in the request list, so it was not issued. Ioctl arguments were not changed.

`decide_ran_on_npu` does not appear in the example output. The call in `send_cited_whole_array_once` sits after a successful device return. This run returned `CitedIoctlFailed` before that call, so `decide_ran_on_npu` was not called. The matmul was not compared to the CPU reference.

Product Pass is No. Exit code 1, no CPU match, and `decide_ran_on_npu` was not true. `invent_pass` stays 0.

No XRT. `whole_array.exe` was not run. `decide_ran_on_npu` was not edited. `docs/whole-array-ioctl-results.md` was not edited.

The logs were captured for this run and are quoted above.
