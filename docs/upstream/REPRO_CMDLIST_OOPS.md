# Repro: `aie2_cmdlist_fill_slot` lab oops (EXEC_CMD scaffold)

## Danger

On **unpatched** `amdxdna`, this sequence **observed** a kernel NULL deref in
`aie2_cmdlist_fill_slot` and could leave `/dev/accel` opens in **D-state**.
Only run on a machine you can reboot. Prefer a patched module first if you only
want “does not oops” validation.

## Program

[`examples/repro_amdxdna_cmdlist_oops.c`](../../examples/repro_amdxdna_cmdlist_oops.c)

```bash
gcc -O2 -Wall -Wextra -o /tmp/repro_amdxdna_cmdlist_oops \
  examples/repro_amdxdna_cmdlist_oops.c

# REFUSES without the flag
/tmp/repro_amdxdna_cmdlist_oops --i-know-this-can-wedge
```

## What it does

Same class of bring-up as the 2026-07-13 dragon-npu scaffold:

1. `open("/dev/accel/accel0")`
2. `CREATE_BO` DEV_HEAP (64 MiB; ignored if already present)
3. `CREATE_BO` SHMEM UMQ + log
4. `CREATE_HWCTX` (with `qos_p` filled)
5. `CREATE_BO` type **CMD**, zeros or incomplete `ERT_START_NPU`-style header
6. `DRM_IOCTL_AMDXDNA_EXEC_CMD` (`SUBMIT_EXEC_BUF`, `cmd_count=1`)

## Outcomes

| Driver | Typical result |
|--------|----------------|
| Stock unpatched | May **oops** (`fill_slot`); check `journalctl -k -b` |
| Staging + L2 null-guards (PR) | `EXEC_CMD` fails with **errno**, no oops |
| L3 `exec_cmd_allow=0` | **`-EPERM`** before job path |

## Maintainer note (client NULL)

The PR discusses null-guards including `cmd_abo->client`. Reviewers may note that
a handle lookup should always return a BO with a live client. This program does
**not** implement a close-vs-submit race; it reproduces the **lab incomplete
cmdlist EXEC_CMD** that hit `aie2_cmdlist_fill_slot` with **RAX=0** in the
captured oops. Exact object that was NULL at that instruction is best confirmed
on your tree (offsets differ by build).

## Attach / link

- Oops: `docs/upstream/evidence/ATTACH_fill_slot_oops_20260713_220051.txt`
- This repro source: `examples/repro_amdxdna_cmdlist_oops.c`
