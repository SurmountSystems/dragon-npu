# Wave5 Path C agent attempt — `xrt-smi` examine + validate

**Date:** 2026-08-11
**Operator cue:** "lol no u do it" (agent runs Path C, not operator-only)
**Repo:** `/home/hunter/Projects/ai/dragon-npu`
**Workdir evidence:** `/tmp/dragon_npu_path_c_validate_1149575/`, `/tmp/dragon_npu_path_c_1149468/`

---

## Honesty (always)

| Token | Value |
|-------|--------|
| invent_pass | **0** |
| Pass | **No** |
| ran_on_npu | **false** (not claimed) |
| load_accepted_claim | **0** |
| not_c_ready | **1** |
| dragon-npu L3 / ALLOW | **not used** |
| C_UNLOCK mint | **none** |
| bins invented / re-tagged | **no** |
| `--ingest` | **not run** (no new completed-run peel) |

`xrt-smi validate` health outcome (here: **FAILED** for missing archives) is **≠** product Pass and **≠** load-accepted alone. Even a future green validate would still be health-only unless a real completing job yields peelable material with honest PROVENANCE.

---

## Goal vs result

| Goal | Result |
|------|--------|
| Locate `xrt-smi` | **Present** `/usr/bin/xrt-smi` |
| Read-only examine | **OK** — device visible |
| Attempt validate | **FAILED** — no test archives |
| NPU sample complete | **No** — tests skipped, no device job |
| Peel / ingest load-accepted | **None** — nothing new to peel |
| Journal scrape after | **OK** — no new Path-C window noise |
| Host capture scan | **OK** — known research only |

**Primary wall for Path C on this host:** XRT runtime package is installed, but **platform validate test path / archives are missing** (`/opt/xilinx/xrt/test/` absent). Validate cannot run gemm/latency/throughput without those archives.

---

## Commands and exit codes

### 1) Locate + version

```text
which xrt-smi
# → /usr/bin/xrt-smi
# owned by: xrt 1:2.21.75-10
# also: xrt-plugin-amdxdna 1:2.21.75-2

timeout 30 xrt-smi --version
# exit 0
# XRT 2.21.75  Hash 4eb1f439…  NPU Firmware 1.1.2.64
# amdxdna / virtio-pci: 7.1.5-zen1-2-zen
```

### 2) Examine (read-only)

```text
timeout 60 xrt-smi examine
# exit 0
# Device: [0000:c6:00.1]  RyzenAI-npu6
# Processor: AMD Ryzen AI 7 PRO 350
# NPU Firmware Version: 1.1.2.64

timeout 90 xrt-smi examine -d 0000:c6:00.1 -r platform
# exit 0
# Name RyzenAI-npu6  Power Mode Default  Total Columns 8

timeout 90 xrt-smi examine -d 0000:c6:00.1 -r aie-partitions
# exit 0
# AIE Partitions: No hardware contexts running on device
```

Artifacts: `/tmp/dragon_npu_path_c_1149468/examine_*.txt`

### 3) Validate (attempted; failed)

```text
# latency only
timeout 180 xrt-smi validate -d 0000:c6:00.1 -r latency --batch
# process exit 0 (tool reports Validation failed)
# WARNING: User doesn't have admin permissions to set performance mode.
#          Running validate in Default mode
# Test latency: Error(s): No archive found, skipping test  [FAILED]

timeout 120 xrt-smi validate -d 0000:c6:00.1 -r latency --verbose --batch
# same: No archive found, skipping test [FAILED]

timeout 120 xrt-smi validate -d 0000:c6:00.1 -r all --verbose --batch
# gemm:     No archive provided, skipping test [FAILED]
# latency:  No archive found, skipping test     [FAILED]
# throughput: No archive found, skipping test   [FAILED]
# Validation failed
```

Artifacts:
`/tmp/dragon_npu_path_c_validate_1149575/validate_latency.txt`
`/tmp/dragon_npu_path_c_validate_1149575/validate_latency_verbose.txt`
`/tmp/dragon_npu_path_c_validate_1149575/validate_all_verbose.txt`

### 4) Root cause of validate failure

| Check | Result |
|-------|--------|
| `/opt/xilinx/xrt/test/` | **Missing** (`/opt/xilinx` absent entirely) |
| Binary path hint (`strings xrt-smi`) | expects `/opt/xilinx/xrt/test/` |
| Arch `xrt` package file list | runtime + headers + cmake; **no** validate xclbin/test tree |
| Host search for `gemm.xclbin` / `recipe_gemm.json` / `validate.xclbin` | **none** under `/usr`, `/opt`, ryzen-ai-install |
| Flexml/vendor xclbins in ryzen-ai venv | present as **research** STX/flexml material — **not** xrt-smi validate archives; **not** used as load-accepted peels here |

**Conclusion:** Path C validate is **blocked by missing platform test archives**, not by device absence. Examine proves the NPU node is visible; validate never scheduled a real job.

### 5) Artifact hunt after attempt

| Location | Finding |
|----------|---------|
| `/tmp` new peels (last 10 min) | **no** new `.xclbin` / ctrltext / insts from validate |
| `/tmp/.tmpxrt*` | stale XRT dirs from earlier sessions (not this shot) |
| `~/.cache` via host scan | no new completed-run peel |
| `/opt/xilinx` | absent |
| Capture bag | existing **RESEARCH ONLY** public mlir-aie AOT only |

**NPU complete yes/no:** **No.**

**New peels found for load-accepted ingest:** **None.**

### 6) Journal scrape (after attempt)

```text
bash tools/journal_npu_scrape.sh --hours 2
# exit 0
# match_total=1
# journal_hint=2026-08-11T13:16:27-06:00 … aie2_hwctx_cfg_debug_bo: Get bo 2 failed
# soft_tokens: get_bo_failed=1  (historical L5 research; not from this Path C shot)
# JOURNAL_SCRAPE_OK invent_pass=0 not_c_ready=1 not_ran_on_npu=1
```

No APP_LOAD / IO_PAGE_FAULT / cmd_timeout lines attributed to the validate skip path (expected: skipped before real EXEC).

### 7) Host capture scan

```text
bash tools/host_capture_scan.sh
# exit 0
# hits_n=120  multi_txn_n=1  research_dup_n=11
# load_accepted_claim=0  invent_pass=0  Pass=No
# report: .agents/reports/wave5-host-capture-scan.md
```

Only known research peels + multi-txn refuse. **No** `collect_load_accepted_candidate.sh --ingest` (would be dishonest without a new completed-run peel). Dry-run collect intentionally **skipped** for re-tag risk on public AOT sha16s.

### 8) Path status helper (context)

```text
bash tools/load_accepted_path_status.sh
# path_c_xrt_smi=1  path_c_xclbinutil=1
# path_b trees ready (operator NPU run preferred)
# path_a FastFlowLM/Lemonade absent
```

---

## NPU complete / peels summary

| Question | Answer |
|----------|--------|
| Did any xrt-smi validate test complete on NPU? | **No** (all skipped: no archive) |
| Health green? | **No** (Validation failed) |
| Device present for examine? | **Yes** (`0000:c6:00.1` RyzenAI-npu6, FW 1.1.2.64) |
| Peels from this attempt? | **None** |
| Ingest performed? | **No** |
| Unlock / L3 / ALLOW? | **No** |

---

## What would unblock Path C (operator / human TTY — not agent-done)

Agent did **not** sudo-install packages or invent archives. If operator wants validate to actually run:

1. Install a full XRT layout that provides **platform test archives** under
   `/opt/xilinx/xrt/test/` (or whatever path the distro’s amdxdna XRT build documents for NPU validate).
   Exact package name on Garuda/Arch is **not** present on this host as a ready test-data package; stock `xrt` + `xrt-plugin-amdxdna` alone are insufficient.
2. Optional privileged performance mode (tool warned; ran Default): only if operator wants it — **not required** for archive presence.
3. Re-run:

```bash
xrt-smi examine
xrt-smi validate -d 0000:c6:00.1 -r all --verbose --batch
# if a test actually runs and dumps artifacts, then:
cd /home/hunter/Projects/ai/dragon-npu
bash tools/journal_npu_scrape.sh --hours 1
bash tools/host_capture_scan.sh
# only if NEW completed-run peel (never multi-txn 32208; never re-tag public AOT):
# bash tools/collect_load_accepted_candidate.sh --dry-run /path/to/peel name
```

Even then: **validate health green ≠ Pass ≠ load-accepted** without documented completed-job peel + honest PROVENANCE.

## Preferred alternate (still primary wall material)

**Path B** is ready on this host (mlir-aie + capture venv + aiebu-asm). Smallest completing NPU run:

```bash
# see docs/LOAD_ACCEPTED_CAPTURE.md Path B
# python3 vector_scalar_add.py --dev npu2
# then ingest + completed_on_this_fw=yes only if true
```

Path A (FastFlowLM / Lemonade) remains **absent**; agent will not network-install.

---

## Footer (honest)

```
invent_pass=0
Pass=No
ran_on_npu=false
load_accepted_claim=0
not_c_ready=1
npu_complete=no
peels_new=0
ingest=none
unlock_mint=none
l3_allow=none
path_c_blocker=missing_/opt/xilinx/xrt/test_archives
xrt_smi_examine=ok
xrt_smi_validate=failed_skip_no_archive
report=.agents/reports/wave5-path-c-agent-attempt.md
```
