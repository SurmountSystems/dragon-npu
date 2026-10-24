# AIE4 / NPU3 PCI ids from the V5 series (patch 5)

Read on 2026-10-23. This note records the device id table. It is not a
product Pass, not `ran_on_npu`, and not an unlock.

Sources:

- Cover letter, Lizhi Hou reply 2026-10-21 ("Applied patches to
  drm-misc-next"):
  [linux-kernel archive 2610.2/14306](https://lkml.iu.edu/2610.2/14306.html)
  (accessed: 2026-10-23).
- Patch 5, David Zhang, 2026-10-19, "accel/amdxdna: Add NPU3 classic
  device support":
  [linux-kernel archive 2610.2/07943](https://lkml.iu.edu/2610.2/07943.html)
  (accessed: 2026-10-23).

The patch writes `PCI_DEVICE(PCI_VENDOR_ID_AMD, …)`. That macro is the
AMD vendor id `0x1022`. Device ids below are what the patch prints.

## PCI match list after the patch (`pci_ids[]`)

| Vendor macro | Device id | Change in patch 5 |
|---|---|---|
| `PCI_VENDOR_ID_AMD` | `0x1502` | unchanged (still listed) |
| `PCI_VENDOR_ID_AMD` | `0x17f0` | unchanged (still listed) |
| `PCI_VENDOR_ID_AMD` | `0x17f1` | added |
| `PCI_VENDOR_ID_AMD` | `0x17f2` | unchanged (still listed) |
| `PCI_VENDOR_ID_AMD` | `0x17f3` | unchanged (still listed) |
| `PCI_VENDOR_ID_AMD` | `0x1B0B` | removed ("will not be released") |
| `PCI_VENDOR_ID_AMD` | `0x1B0C` | removed ("will not be released") |

`0x1502` is only in this match list. Patch 5 does not show which info
struct `0x1502` uses. That cell is not filled here.

## Device-info rows actually in the patch (`amdxdna_ids[]`)

| Device | Revision | Info struct | Change |
|---|---|---|---|
| `0x17f0` | `0x10` | `dev_npu4_info` | unchanged |
| `0x17f0` | `0x11` | `dev_npu5_info` | unchanged |
| `0x17f0` | `0x20` | `dev_npu6_info` | unchanged |
| `0x17f1` | `0x10` | `dev_npu3_classic_info` | added |
| `0x17f1` | `0x13` | `dev_npu3_classic_info` | added |
| `0x17f2` | `0x10` | `dev_npu3_pf_info` | unchanged |
| `0x17f2` | `0x13` | `dev_npu3_pf_info` | added |
| `0x17f3` | `0x10` | `dev_npu3_vf_info` | unchanged |
| `0x17f3` | `0x13` | `dev_npu3_vf_info` | added |
| `0x1B0B` | `0x10` | `dev_npu3_pf_info` | removed |
| `0x1B0C` | `0x10` | `dev_npu3_vf_info` | removed |

`dev_npu3_classic_info` is defined in
`drivers/accel/amdxdna/npu3_regs.c` in the same patch. The hunk is
`const struct amdxdna_dev_info dev_npu3_classic_info` with `.ops =
&aie4_classic_ops`, `.default_vbnv = "RyzenAI-npu3"`, and `.device_type
= AMDXDNA_DEV_TYPE_UMQ`. `amdxdna_pci_drv.h` adds `extern const struct
amdxdna_dev_info dev_npu3_classic_info`. Firmware lines added for that
classic device are `amdnpu/17f1_10/npu.sbin`,
`amdnpu/17f1_10/cert.sbin`, `amdnpu/17f1_13/npu.sbin`, and
`amdnpu/17f1_13/cert.sbin`.

## Q1

The series does not cover Strix. This host is PCI `1022:17f0` revision
`0x20`. In `drivers/accel/amdxdna/amdxdna_pci_drv.c`, patch 5 leaves
these device-info lines unchanged:

- `{ 0x17f0, 0x10, &dev_npu4_info }`
- `{ 0x17f0, 0x11, &dev_npu5_info }`
- `{ 0x17f0, 0x20, &dev_npu6_info }`

None of those pointers is `dev_npu3_classic_info`. Revision `0x10` is
npu4. Revision `0x11` is npu5. Revision `0x20` (this host) stays
`dev_npu6_info`. The PCI match line `{ PCI_DEVICE(PCI_VENDOR_ID_AMD,
0x17f0) }` is also unchanged.

The new classic binding is a different device id. Patch 5 adds
`{ PCI_DEVICE(PCI_VENDOR_ID_AMD, 0x17f1) }` and the info rows
`{ 0x17f1, 0x10, &dev_npu3_classic_info }` and
`{ 0x17f1, 0x13, &dev_npu3_classic_info }`. That info struct lives in
`drivers/accel/amdxdna/npu3_regs.c`. Patch 5 also adds PF/VF revision
`0x13` rows for `0x17f2` and `0x17f3`. It is not a single new row, and
it is not "every `0x17f0` line is NPU4."

Cover letter sentences read from
[14306](https://lkml.iu.edu/2610.2/14306.html) (accessed: 2026-10-23):
"This patch series extends the amdxdna accelerator driver to support
AMD AIE4 (NPU3) platforms, including classic and PF/VF for SR-IOV with
kernel-mode command submission (KMQ), system power management, and
runtime power management for the classic device." Testing: "Tested on
AMD AIE4/NPU3 hardware in both classic and SR-IOV (PF/VF) modes." and
"Confirmed no regression on existing AIE2 devices (NPU1/NPU4)."

The patch does not name a retail chip (Ryzen AI 300, Ryzen AI 400,
Gorgon, or Medusa). That half of the question is not in the table.

invent_pass=0. Pass=No. ran_on_npu=false. Unlock none.
`C_UNLOCK=2c9c12e8` CONSUMED.
