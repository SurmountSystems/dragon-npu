# wave4 host NPU stack inventory (W4a T1 + T8)

- **Date:** 2026-08-11T14:10:09-06:00
- **Pass:** **No**
- **invent_pass:** 0
- **not_ran_on_npu:** 1
- **not_c_ready:** 1

## Counts

| metric | value |
|--------|-------|
| accel devices (/dev/accel*) | 1 |
| sysfs accel | 1 |
| amdxdna module | present |
| tools present | 2 |
| install roots present | 2 |
| sample files found | 69 |
| completing-stack candidates | 1 |
| in-tree capture dir files | 25 |
| firmware | present |
| exec_cmd_allow | missing |
| journal soft ok | 1 |

## Who acts next (T8)

`agent: attempt capture from /home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/gt_capture/instr_vendor_post_patch.bin`

Agent may attempt capture from host candidate path(s). Candidate ≠ residual match ≠ Pass. Live still needs active unlock + named reason.

### Candidate paths

| bytes | path |
|-------|------|
| 32208 | `/home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/gt_capture/instr_vendor_post_patch.bin` |

## Honesty

- invent_pass=0
- Pass=No
- not_ran_on_npu=1
- not_c_ready=1
- Read-only host probes only (no sudo, modprobe, EXEC_CMD, ALLOW).
- In-tree research AOT under the capture dir is not a new completing-stack candidate.

## Full stdout log

```
================================================================
host_npu_stack_inventory - read-only (W4a T1 + T8)
================================================================
ROOT=/home/hunter/Projects/ai/dragon-npu
date=2026-08-11T14:10:08-06:00
Honesty: invent_pass=0 not_ran_on_npu=1 not_c_ready=1 Pass=No
No sudo / no modprobe / no EXEC_CMD / no ALLOW / no invent bytes

=== /dev/accel* ===
  present readable: /dev/accel

=== /sys/class/accel ===
  accel0 -> /sys/devices/pci0000:00/0000:00:08.2/0000:c6:00.1/accel/accel0

=== amdxdna module / firmware (soft) ===
  module=present
  exec_cmd_allow=missing (stock path; no L3 knob)
  firmware_dir=/lib/firmware/amdnpu
    bytes=73534  npu.sbin.1.5.2.380.zst
    bytes=75293  npu.sbin.1.5.5.391.zst
    bytes=72779  npu.sbin.1.0.0.63.zst
    bytes=125276  npu.sbin.1.1.2.64.zst
    bytes=72747  npu.sbin.1.0.0.166.zst
    bytes=125264  npu.sbin.1.1.2.65.zst

=== tools on PATH (which) ===
  present: xrt-smi -> /usr/bin/xrt-smi
  present: xclbinutil -> /usr/bin/xclbinutil
  missing: xbutil
  missing: aiebu-asm
  missing: aiebu-dump
  missing: aiecompiler
  missing: xchesscc
  missing: vitis
  missing: xrt

=== known install / sample roots ===
  present: /home/hunter/Projects/ai/ryzen-ai-install
  present: /home/hunter/Projects/ai/RyzenAI-SW
  missing: /home/hunter/Projects/ai/RyzenAI
  missing: /home/hunter/ryzen-ai
  missing: /opt/xilinx/xrt
  missing: /opt/xilinx
  missing: /opt/amd/ryzen-ai
  missing: /usr/lib/xrt
  missing: /usr/share/xrt

=== sample binaries / docs on disk (bounded; no copy) ===
  SAMPLE bytes=2373  path=/home/hunter/Projects/ai/ryzen-ai-install/README_GRAB_VITIS_EP.md
  SAMPLE bytes=348  path=/home/hunter/Projects/ai/ryzen-ai-install/venv/lib/python3.12/site-packages/_plotly_utils/README.md
  SAMPLE bytes=1119443  path=/home/hunter/Projects/ai/ryzen-ai-install/venv/voe-4.0-linux_x86_64/xclbins/phoenix/1x4.xclbin
  SAMPLE bytes=4149726  path=/home/hunter/Projects/ai/ryzen-ai-install/venv/voe-4.0-linux_x86_64/xclbins/phoenix/4x4.xclbin
  SAMPLE bytes=10615  path=/home/hunter/Projects/ai/ryzen-ai-install/venv/hybrid-llm/hybrid-llm-artifacts/examples/vlm/README.md
  SAMPLE bytes=231  path=/home/hunter/Projects/ai/ryzen-ai-install/venv/quicktest/README.md
  SAMPLE bytes=231  path=/home/hunter/Projects/ai/ryzen-ai-install/venv-eepy-min/quicktest/README.md
  SAMPLE bytes=8234  path=/home/hunter/Projects/ai/RyzenAI-SW/CNN-examples/Nemotron-OCR-V2/README.md
  SAMPLE bytes=742  path=/home/hunter/Projects/ai/RyzenAI-SW/CNN-examples/getting_started_resnet/Readme.md
  SAMPLE bytes=4664  path=/home/hunter/Projects/ai/RyzenAI-SW/CNN-examples/getting_started_resnet/bf16/README.md
  SAMPLE bytes=9285  path=/home/hunter/Projects/ai/RyzenAI-SW/CNN-examples/getting_started_resnet/bf16/docs/README_C++.md
  SAMPLE bytes=680  path=/home/hunter/Projects/ai/RyzenAI-SW/CNN-examples/getting_started_resnet/int8/Readme.md
  SAMPLE bytes=4236  path=/home/hunter/Projects/ai/RyzenAI-SW/CNN-examples/hello_world/README.md
  SAMPLE bytes=80  path=/home/hunter/Projects/ai/RyzenAI-SW/CNN-examples/hello_world/models/readme.md
  SAMPLE bytes=3089  path=/home/hunter/Projects/ai/RyzenAI-SW/CNN-examples/iGPU/getting_started/README.md
  SAMPLE bytes=2405  path=/home/hunter/Projects/ai/RyzenAI-SW/CNN-examples/image_classification/README.md
  SAMPLE bytes=7620  path=/home/hunter/Projects/ai/RyzenAI-SW/CNN-examples/object_detection/yolov8m/README.md
  SAMPLE bytes=3251  path=/home/hunter/Projects/ai/RyzenAI-SW/CNN-examples/object_detection/yolov8s-worldv2/README.md
  SAMPLE bytes=9262  path=/home/hunter/Projects/ai/RyzenAI-SW/CNN-examples/quark_quantization/README.md
  SAMPLE bytes=9436  path=/home/hunter/Projects/ai/RyzenAI-SW/CNN-examples/super-resolution/README.md
  SAMPLE bytes=4217  path=/home/hunter/Projects/ai/RyzenAI-SW/CNN-examples/torchvision_inference/README.md
  SAMPLE bytes=7400  path=/home/hunter/Projects/ai/RyzenAI-SW/Demos/ASR/Parakeet-TDT/README.md
  SAMPLE bytes=5000  path=/home/hunter/Projects/ai/RyzenAI-SW/Demos/ASR/Whisper/README.md
  SAMPLE bytes=3109  path=/home/hunter/Projects/ai/RyzenAI-SW/Demos/NPU-GPU-Pipeline/README.md
  ... listed first 24 of 69

=== completing-stack capture candidates (host / out-of-bag) ===
  candidates=1 (paths below; not residual-matched; invent_pass=0)
  CANDIDATE bytes=32208  path=/home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/gt_capture/instr_vendor_post_patch.bin

=== in-tree capture dir (testdata/xdna2_cmd/external_known_good/) ===
  dir=present files=25
  PROVENANCE=present
  note: research-only public AOT / cascade already here ≠ load-accepted residual match ≠ Pass

=== soft journal keywords (last 6h; user-readable only) ===
  journal_ok=1 hits≈20 (tail 20 lines)
  J: Aug 11 10:48:21 horizon kernel: amdxdna 0000:c6:00.1: AMD-Vi: Event logged [IO_PAGE_FAULT domain=0x0001 address=0x8fff80008a40 flags=0x0007]
  J: Aug 11 10:48:21 horizon kernel: amdxdna 0000:c6:00.1: AMD-Vi: Event logged [IO_PAGE_FAULT domain=0x0001 address=0x8fff80009000 flags=0x0007]
  J: Aug 11 10:48:21 horizon kernel: amdxdna 0000:c6:00.1: AMD-Vi: Event logged [IO_PAGE_FAULT domain=0x0001 address=0x8fff80009400 flags=0x0007]
  J: Aug 11 10:48:21 horizon kernel: amdxdna 0000:c6:00.1: AMD-Vi: Event logged [IO_PAGE_FAULT domain=0x0001 address=0x8fff80009800 flags=0x0007]
  J: Aug 11 10:48:21 horizon kernel: amdxdna 0000:c6:00.1: AMD-Vi: Event logged [IO_PAGE_FAULT domain=0x0001 address=0x8fff80009c00 flags=0x0007]
  J: Aug 11 10:48:21 horizon kernel: amdxdna 0000:c6:00.1: AMD-Vi: Event logged [IO_PAGE_FAULT domain=0x0001 address=0x8fff8000a000 flags=0x0007]
  J: Aug 11 10:49:23 horizon kernel: amdxdna 0000:c6:00.1: [drm] *ERROR* aie2_set_cmd_timeout: Firmware timeout state capture:
  J: Aug 11 10:49:23 horizon kernel: amdxdna 0000:c6:00.1: [drm] *ERROR* aie2_set_cmd_timeout:         Version: 2.0
  J: Aug 11 10:49:23 horizon kernel: amdxdna 0000:c6:00.1: [drm] *ERROR* aie2_set_cmd_timeout:         Report size: 0x230
  J: Aug 11 10:49:23 horizon kernel: amdxdna 0000:c6:00.1: [drm] *ERROR* aie2_set_cmd_timeout:         Context ID: 15
  J: Aug 11 10:49:23 horizon kernel: amdxdna 0000:c6:00.1: [drm] *ERROR* aie2_set_cmd_timeout:         DPU PC: 0xffffffff
  J: Aug 11 10:49:23 horizon kernel: amdxdna 0000:c6:00.1: [drm] *ERROR* aie2_set_cmd_timeout:         TXN OP ID: 0xd
  J: Aug 11 10:49:23 horizon kernel: amdxdna 0000:c6:00.1: [drm] *ERROR* aie2_set_cmd_timeout:         Context PC: 0x28b1550b
  J: Aug 11 10:49:23 horizon kernel: amdxdna 0000:c6:00.1: [drm] *ERROR* aie2_set_cmd_timeout:         Fatal error type: 0x0
  J: Aug 11 10:49:23 horizon kernel: amdxdna 0000:c6:00.1: [drm] *ERROR* aie2_set_cmd_timeout:         Fatal error exception type: 0x0
  J: Aug 11 10:49:23 horizon kernel: amdxdna 0000:c6:00.1: [drm] *ERROR* aie2_set_cmd_timeout:         Fatal error exception PC: 0x0
  J: Aug 11 10:49:23 horizon kernel: amdxdna 0000:c6:00.1: [drm] *ERROR* aie2_set_cmd_timeout:         Fatal error app module: 0x0
  J: Aug 11 10:49:23 horizon kernel: amdxdna 0000:c6:00.1: [drm] *ERROR* aie2_set_cmd_timeout:         Fatal error task ID: 0
  J: Aug 11 10:49:23 horizon kernel: amdxdna 0000:c6:00.1: [drm] *ERROR* aie2_set_cmd_timeout:         Timed out sub command ID: 0
  J: Aug 11 13:16:27 horizon kernel: amdxdna 0000:c6:00.1: [drm] *ERROR* aie2_hwctx_cfg_debug_bo: Get bo 2 failed

=== English summary ===
  accel devices: 1  sysfs accel: 1  amdxdna: present
  tools present: 2  install roots present: 2
  sample files listed/count: min(69,24)/69
  firmware: present  exec_cmd_allow: missing
  in-tree capture dir files: 25  completing-stack candidates: 1
  Capture candidates: 1 host path(s) may hold capturable material (not proven load-accepted).

=== Who acts next (T8) ===
  agent: attempt capture from /home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/gt_capture/instr_vendor_post_patch.bin
  Also: operator may still place a true load-accepted working-stack capture under the capture dir.
  Live apply still needs active C_UNLOCK + named offline reason. Unlock ≠ Pass.

=== Footer (honest) ===
invent_pass=0
not_ran_on_npu=1
not_c_ready=1
Pass=No
accel_dev_n=1
accel_sys_n=1
tools_present_n=2
install_roots_present_n=2
sample_bins_n=69
candidate_n=1
bag_files_n=25
journal_ok=1
who_acts=agent: attempt capture from /home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/gt_capture/instr_vendor_post_patch.bin
entrypoint=tools/host_npu_stack_inventory.sh
```
