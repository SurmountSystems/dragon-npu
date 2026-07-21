# Never again: amdxdna wedge policy (2026-07-13)

## What happened

1. dragon-npu issued **scaffold EXEC_CMD** (not a real firmware txn).
2. Kernel workqueue hit **NULL deref** in `aie2_cmdlist_fill_slot` (`amdxdna`).
3. Later **`open("/dev/accel/accel0")` hung** in D-state (`amdxdna_drm_open`).
4. Module **refcnt** stayed high → no `rmmod`; PCI remove/unbind hung.

This is **not** acceptable bring-up. Permanent controls below exist so we cannot
recreate it by accident.

## Permanent controls (userspace — this repo) — **L1 complete**

| Layer | Mechanism | Default |
|-------|-----------|---------|
| **Compile** | Cargo feature `xdna2-dangerous-exec-cmd` | **OFF** |
| **Runtime** | Env `XDNA2_ALLOW_EXEC_CMD=1` | **unset** |
| **API** | `exec_cmd_is_enabled()` / `submit_command` / `submit_and_wait_command` | hard error if gated |
| **WAIT_CMD** | `wait_command` (not L1-gated; cleanup path) | finite timeout in lab helpers |
| **Probe / timed_matmul** | Never call EXEC_CMD unless both layers allow | skip submit |
| **Honesty** | `ran_on_npu` only for verified NPU compute; never envelope / inst-blob-without-verify Pass; `bo_roundtrip` uses `ran_on_device_path` only; offline `tools/c4_pass_check.sh` is snapshot audit only (**≠ invent Pass**) | enforced |
| **Lab procedure** | [`XDNA2_EXEC_CMD_LAB.md`](./XDNA2_EXEC_CMD_LAB.md) | operator single-shot only |

Enabling EXEC_CMD requires **both** userspace gates (and, **if L3 is installed**, the kernel param):

```bash
# L1 (always required in this repo)
cargo build --features amd-xdna2,xdna2-dangerous-exec-cmd
XDNA2_ALLOW_EXEC_CMD=1 cargo run --features amd-xdna2,xdna2-dangerous-exec-cmd --example …

# L3 (only after installing the lab-patched module — default refuses EXEC_CMD):
#   sudo modprobe amdxdna exec_cmd_allow=1
# Without this, the kernel returns -EPERM even when L1 is fully open.
# Stock/distro amdxdna without L3 has no exec_cmd_allow param.
```

Legacy `XDNA2_TRY_SUBMIT` is **fully ignored** (retired). Userspace only the double gate above can enable EXEC_CMD from dragon-npu; a lab L3 module adds a third gate.

## Product vs lab privileges (who needs root?)

**KMQ (kernel-mode queue) is not “sudo.”** On this host the NPU reports
`device_type=0` (KMQ): the **kernel driver** owns the submit path. That is a
device contract, not a privilege model. End users still use a normal device node.

| Who | When | Needs root? | What |
|-----|------|-------------|------|
| **Admin** | Install once | **Yes** (or package manager) | Driver/module, firmware, udev/group so `/dev/accel/accel0` is openable (often group `render`) |
| **Product user** | Every day | **No** | Open device, create hwctx, run when product works — **no** per-run `sudo` |
| **Lab operator** | Supervised research only | **Temporary** | Flip L3 `exec_cmd_allow=1` on **hardened** modules only; restore `=0` after; L1 still needs feature + `XDNA2_ALLOW_EXEC_CMD=1` |
| **Agent default** | Offline Track A | **No** | No casual sudo; no default ALLOW; no invent `ran_on_npu` |

| Confusion | Truth |
|-----------|--------|
| “KMQ means users need sudo” | **False.** KMQ = queue ownership in the driver. Privilege = udev + who may load modules. |
| “Lab L3 is the product path” | **False.** Stock/distro modules have **no** `exec_cmd_allow`. L3 is hardened-lab only. |
| “Open L1 for product builds” | **False.** Default product builds keep `xdna2-dangerous-exec-cmd` **OFF**. |
| “`umq_bo` on CREATE_HWCTX means UMQ device” | **False.** Field may be 0 on KMQ; UMQ is other generations (`device_type` ≠ 0). |

Packaging tree (DKMS vs upstream): [`XDNA2_PACKAGING.md`](./XDNA2_PACKAGING.md).
Lab enable order: [`XDNA2_EXEC_CMD_LAB.md`](./XDNA2_EXEC_CMD_LAB.md).

### Secondary backends and `xrt-bridge` (Phase 9)

- Features `intel-vpu`, `qualcomm-hexagon`, and `xrt-bridge` are **honest stubs /
  research only**. They never claim silicon Pass or set `ran_on_npu`.
- `xrt-bridge` is **not** default and is **not** enabled by `amd-xdna2`. It must
  not open EXEC_CMD or recommend enabling L1 gates.
- See [`SECONDARY_BACKENDS.md`](./SECONDARY_BACKENDS.md).

## Permanent controls (driver — **L2+L3 materials complete**, **L4 install packaging complete**, **lab host installed**)

Shippable in-repo artifacts (applyable patches + docs + issue bodies + install path):

| Artifact | Path |
|----------|------|
| L2 unified patch | [`kernel/patches/0001-amdxdna-cmdlist-null-guards.patch`](../kernel/patches/0001-amdxdna-cmdlist-null-guards.patch) |
| L3 unified patch | [`kernel/patches/0002-amdxdna-open-failfast-and-exec-gate.patch`](../kernel/patches/0002-amdxdna-open-failfast-and-exec-gate.patch) |
| Apply/build README | [`kernel/patches/README.md`](../kernel/patches/README.md) (apply **0001 then 0002**) |
| L2 design notes | [`kernel/patches/amdxdna-cmdlist-null-guards.md`](../kernel/patches/amdxdna-cmdlist-null-guards.md) |
| L3 design notes | [`kernel/patches/amdxdna-open-failfast.md`](../kernel/patches/amdxdna-open-failfast.md) |
| **L4 install script** | [`scripts/install_amdxdna_hardened.sh`](../scripts/install_amdxdna_hardened.sh) (wedge-aware; default dry-run) |
| **L4 install playbook** | [`docs/XDNA2_HARDENED_INSTALL.md`](./XDNA2_HARDENED_INSTALL.md) |
| **L4 DKMS scaffolding** | [`kernel/dkms/`](../kernel/dkms/) (template + prepare hook; no full vendor) |
| Upstream issue L2 (paste-ready) | [`docs/upstream/AMDXDNA_ISSUE_cmdlist_null_open_wedge.md`](./upstream/AMDXDNA_ISSUE_cmdlist_null_open_wedge.md) |
| Upstream L3 addendum (paste-ready) | [`docs/upstream/AMDXDNA_ISSUE_L3_open_failfast_addendum.md`](./upstream/AMDXDNA_ISSUE_L3_open_failfast_addendum.md) |
| Upstream combined L2+L3 (paste-ready) | [`docs/upstream/AMDXDNA_ISSUE_combined.md`](./upstream/AMDXDNA_ISSUE_combined.md) |

**Base tree:** amd/xdna-driver @ `905717f5cd470580054a1635b9eea24b84dfd67f`.

Goals for amdxdna (GPL, forkable):

1. **No NULL deref** on bad/empty cmdlist — return `-EINVAL` / `-ENODEV` / `-ENOMEM` / `-EOPNOTSUPP`. (**L2 patch — materials ready**)
2. **Fail-fast open/ioctls when latched unhealthy** — return `-EIO` instead of infinite D. (**L3 materials ready**)  
   **Honest coverage:** automatic latch is **TDR dump-only only**. Stock **`tdr_dump_ctx=0`** means the periodic TDR **full-restarts** and **never marks** — open fail-fast **does not auto-arm** under default module params. Always-on L3 under defaults: **`exec_cmd_allow=0`**, secondary killable open `dev_lock`, latch infrastructure. To auto-arm fail-fast after stuck detect (lab/debug): `modprobe amdxdna tdr_dump_ctx=1`. Do **not** mark from ordinary job `-ENODEV` (mbox teardown race).
3. Secondary killable `dev_lock` on open only — SVA bind residual documented. (**L3**)
4. Module param `exec_cmd_allow=0` lab default so **entire** EXEC_CMD ioctl is refused in-kernel. (**L3** — always on after install)

**Lab host (horizon, 2026-07-14/15):** OOT L2+L3 is **installed** under
`updates/amdxdna-hardened/`, `exec_cmd_allow=0`, firmware `npu.dev.sbin`→`npu_7`.
Userspace L1 remains the always-on shield on every machine; kernel L3 adds a third
gate only when this module is loaded. Other hosts still need the install playbook.

Verify anytime (read-only, no `/dev/accel`):

```bash
bash scripts/verify_amdxdna_hardened_host.sh
bash scripts/diagnose_amdxdna.sh
```

## Phase 1 payload research (offline only)

Cmd BO **envelope** layout (header, START_NPU, CHAIN, PREEMPT, DPU) is documented and
unit-tested in-tree — see **`docs/XDNA2_CMD_PAYLOAD.md`**. That does **not** authorize
EXEC_CMD. Instruction **txn** bytes remain opaque; never submit scaffolds or empty
instruction buffers as “success.”

Instruction/txn capture pathways, license notes, host inventory, and pure
`decide_ran_on_npu` honesty criteria:
[`XDNA2_TXN_RESEARCH.md`](./XDNA2_TXN_RESEARCH.md) ·
[`scripts/capture_instruction_blob_checklist.md`](../scripts/capture_instruction_blob_checklist.md)
(envelope packing remains CMD_PAYLOAD).

## Phase 2 EXEC_CMD + WAIT (infrastructure ready; silicon operator-only)

- APIs: `DirectSubmission::wait_command`, `submit_and_wait_command` (L1-gated submit;
  finite timeout; no retry loop). Lab procedure: **[`XDNA2_EXEC_CMD_LAB.md`](./XDNA2_EXEC_CMD_LAB.md)**.
- Default `cargo test` / feature-on-without-ALLOW still **refuse** EXEC_CMD.
- **Not** claimed proven on silicon unless an operator recorded a gated single-shot.
- Still require: diagnose clean, L2 preferred, L1 + temporary L3 `exec_cmd_allow=1`,
  Phase 1 / known-good payload, `timeout` wrappers, restore `exec_cmd_allow=0`.

## Ops rules (humans + agents)

1. **Never** open `/dev/accel` in a loop while D-state waiters exist.
2. **Diagnose / verify first:** `bash scripts/verify_amdxdna_hardened_host.sh` then `bash scripts/diagnose_amdxdna.sh`.
3. **No EXEC_CMD** until a **known-good** payload exists (not a research scaffold). Envelope: [`XDNA2_CMD_PAYLOAD.md`](./XDNA2_CMD_PAYLOAD.md). Txn capture + Pass blockers: [`XDNA2_TXN_RESEARCH.md`](./XDNA2_TXN_RESEARCH.md).
4. Hardware probes: always `timeout …`.
5. File/track upstream: [amd/xdna-driver issues](https://github.com/amd/xdna-driver/issues) — paste body from `docs/upstream/…` when ready (human files). **Agents do not open PRs/issues.**

## Recovery (if it ever happens again)

1. Stop opening the device.
2. `bash scripts/diagnose_amdxdna.sh`.
3. Prefer reboot to clear D-state (PCI reset/remove may hang — measured).
4. After clean boot: confirm with diagnose; only then continue bring-up **without** dangerous-exec feature.
5. Optionally install L2+L3-patched `amdxdna` **only after** clean unload/boot:
   - Playbook: [`docs/XDNA2_HARDENED_INSTALL.md`](./XDNA2_HARDENED_INSTALL.md)
   - Script: `bash scripts/install_amdxdna_hardened.sh` (refuses if still wedged)
   - Patches: `kernel/patches/README.md` · DKMS template: `kernel/dkms/README.md`
   Lab default after L3: `exec_cmd_allow=0`; production/XRT needs `exec_cmd_allow=1`.
   Open fail-fast auto-latch needs non-default `tdr_dump_ctx=1` (see `kernel/patches/amdxdna-open-failfast.md`).

## Definition of “fixed forever”

- [x] Default dragon-npu builds **cannot** EXEC_CMD. (**L1**)
- [x] Double gate (feature + env) documented and tested. (**L1**)
- [x] **L2 materials:** applyable null-guard patch + build/apply docs + upstream issue body (in-tree).
- [x] **L3 materials:** open fail-fast / killable `dev_lock` / `exec_cmd_allow` patch + docs + issue addendum (in-tree).
- [x] **L4 materials:** install script + HARDENED_INSTALL playbook + DKMS scaffolding + combined issue body (in-tree). **Does not mean host is fixed.**
- [x] **L2/L3 installed on host (lab):** OOT module in `updates/amdxdna-hardened/` on **7.1.3-zen1-2-zen**, srcversion `E36F27FC09FB5CA0C0C3E87`, `exec_cmd_allow=0` (runtime **N**), firmware alias `npu.dev.sbin.zst`→`npu_7.sbin.zst` (1.1.2.64); smoke open/get_info/**hwctx_ok** (2026-07-14/15). Host table: [`XDNA2_DIRECT_CAPABILITIES.md`](./XDNA2_DIRECT_CAPABILITIES.md). Evidence: [`docs/upstream/LAB_EVIDENCE.md`](./upstream/LAB_EVIDENCE.md).
- [ ] **Reboot persistence proven:** after a real reboot, `bash scripts/verify_amdxdna_hardened_host.sh --post-reboot` exit 0 (still on `updates/…`, `exec_cmd_allow=0`, alias intact). **Not claimed until user reboots.** Checklist: [`XDNA2_HARDENED_INSTALL.md`](./XDNA2_HARDENED_INSTALL.md) § post-reboot.
- [ ] Issue **filed** on amd/xdna-driver with oops + open-wedge notes (paste-ready: [`docs/upstream/AMDXDNA_ISSUE_combined.md`](./upstream/AMDXDNA_ISSUE_combined.md); human posts). Pack: [`docs/upstream/README.md`](./upstream/README.md). Fork WIP (no PR): `cryptoquick/xdna-driver` branch `amdxdna/cmdlist-null-guards` @ `a9ef3c4…` (L2 only).
- [x] Optional: long-term host track via **`updates/` + modprobe.d** (not full DKMS yet; scaffolding remains in `kernel/dkms/`).

## L1 unit-test matrix (lib only; no `/dev/accel` required)

Lib tests assume EXEC_CMD is **not** fully enabled in the process. Run both:

```bash
# Feature OFF (compile-time gate) — primary CI
cargo test --features amd-xdna2 --lib

# Feature ON, env OFF (runtime gate) — must still refuse EXEC_CMD
# Do NOT set XDNA2_ALLOW_EXEC_CMD=1 for this job.
cargo test --features amd-xdna2,xdna2-dangerous-exec-cmd --lib
```

A third job with `XDNA2_ALLOW_EXEC_CMD=1` is **manual/dangerous** (scaffold can oops an **unpatched** driver).
