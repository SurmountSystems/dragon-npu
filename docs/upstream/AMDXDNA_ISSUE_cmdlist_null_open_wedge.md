# Upstream issue body (paste into amd/xdna-driver — do **not** auto-post)

Copy the section below into a new issue on
[amd/xdna-driver](https://github.com/amd/xdna-driver/issues).
Do not attach local machine secrets. Stack traces may be redacted slightly but
keep the RIP and key frames.

**When filing:** attach the real `dmesg`/journal oops (RIP `aie2_cmdlist_fill_slot`,
key frames, optional registers). Soften “reproducible” language if you only have
a single capture. Replace any in-tree patch `Signed-off-by` with your DCO identity
before opening a PR.

---

## Title

AIE2: NULL deref in `aie2_cmdlist_fill_slot` after EXEC_CMD leaves `amdxdna_drm_open` in D-state (refcnt wedge)

## Body

### Summary

On Ryzen AI 300 / Strix-class NPU (**PCI 1022:17f0**), submitting a command via
`DRM_IOCTL_AMDXDNA_EXEC_CMD` with an incomplete / scaffold cmdlist payload was
**observed** to trigger a **NULL pointer dereference** in `aie2_cmdlist_fill_slot`
(2026-07-13 lab incident). After the oops, subsequent `open("/dev/accel/accel0")`
callers hung in **D-state** inside `amdxdna_drm_open`, module **refcnt** stayed
elevated, and `rmmod` / PCI remove hung. Host recovery effectively required reboot
(or invasive PCI/bus reset attempts that may still fail while waiters exist).

This was recreated under a lightweight userspace path that creates BO + hwctx and
issues EXEC_CMD without a fully valid firmware transaction (research scaffold —
not XRT). Exact NULL object (client vs ops vs slot) remains a **hypothesis** from
the RIP/`RAX=0` pattern; defensive null-guards are appropriate regardless.

Even if userspace is wrong, the driver should return `-EINVAL` / `-ENODEV` /
`-ENOMEM` / `-EOPNOTSUPP` rather than oops and wedge open.

### Environment

| Item | Value |
|------|--------|
| Device | AMD XDNA2 NPU, **PCI ID 1022:17f0** |
| Kernel | **7.1.3-zen** (local; amdxdna loaded) |
| Firmware (GET_INFO) | **1.1.2.64** (observed via lightweight direct query) |
| Driver source base used for proposed patch | [amd/xdna-driver](https://github.com/amd/xdna-driver) `main` @ `905717f5cd470580054a1635b9eea24b84dfd67f` (`aie2_message.c` last touch `b9686b40a31149b4b2dd823d87be4567ba697618`) |
| Userspace | Out-of-tree lightweight DRM client (not XRT), research EXEC_CMD scaffold |

### Symptoms

1. **Oops** with RIP in **`aie2_cmdlist_fill_slot`** (amdxdna); lab notes include a
   zeroed register used as a base pointer (RAX=0 pattern consistent with
   `cmd_abo->client` or ops table missing — **hypothesis**, not proven which).
2. Afterward, **`amdxdna_drm_open`** waiters stick in **uninterruptible sleep (D)**.
3. **Module refcnt** high (example observation: refcnt≈22) → cannot `rmmod`.
4. **PCI remove / unbind** of the NPU function may hang; parent GPP bridge may
   expose reset while the NPU function itself often has no sysfs `reset`.
5. Device node may still exist but is effectively unusable until reboot.

### Suspected code path

In `src/driver/amdxdna/aie2_message.c`:

- `aie2_cmdlist_single_execbuf` obtains `cmdbuf_abo` via
  `aie2_cmdlist_get_cmd_buf`, vmaps it, and calls
  `aie2_cmdlist_fill_slot(cmd_buf, cmd_abo, …)` **without** validating
  `cmdbuf_abo`, `cmd_abo`, `cmd_buf`, or mailbox channel (multi path has the
  same class of issue).
- `aie2_cmdlist_fill_slot` does:

  ```c
  struct amdxdna_dev *xdna = cmd_abo->client->xdna;
  …
  ret = EXEC_MSG_OPS(xdna)->fill_*_slot(...);
  ```

  with no null checks on `cmd_abo`, `cmd_abo->client`, `slot`, or
  `EXEC_MSG_OPS(xdna)`.

A missing/invalid object or unset `exec_msg_ops` therefore becomes a kernel
NULL deref instead of a clean errno.

### Related upstream work (not a full substitute)

- Issue [#1347](https://github.com/amd/xdna-driver/issues/1347) — NULL-deref /
  UAF flushing timed-out mailbox messages on channel teardown (different path;
  same “NULL on teardown / bad state” theme).
- PR [#1441](https://github.com/amd/xdna-driver/pull/1441) —
  `accel/amdxdna: fix null pointer dereference for cmd_submit on aie4`
  (ops null-guard on **aie4**; AIE2 cmdlist fill/execbuf still open).
- PR [#1468](https://github.com/amd/xdna-driver/pull/1468) — test coverage for
  aie4 cmd_submit `EOPNOTSUPP`.
- PR [#598](https://github.com/amd/xdna-driver/pull/598) (historical) — ERT error
  handling when cmdlist execution fails (does not add fill_slot guards).

We did not find an open issue that matches **AIE2 `fill_slot` oops + post-oops
open D-state wedge** specifically (search ~2026-07).

### Proposed fix

Minimal defensive checks returning `-EINVAL` / `-ENODEV` / `-ENOMEM` /
`-EOPNOTSUPP` in:

- `aie2_cmdlist_fill_slot`
- `aie2_cmdlist_single_execbuf`
- `aie2_cmdlist_multi_execbuf` (same class)

Including guards for `job->ctx`/`priv` before `aie2_cmdlist_get_cmd_buf`, and a
cmd-BO vmap probe before `amdxdna_cmd_get_op` (which currently vmaps without a
NULL check).

A concrete unified diff rebased onto the tree SHA above is available in the
dragon-npu tree (for review / cherry-pick):

- Patch path (out of tree): `kernel/patches/0001-amdxdna-cmdlist-null-guards.patch`
- Notes: `kernel/patches/amdxdna-cmdlist-null-guards.md`
- Apply/build notes: `kernel/patches/README.md`

Happy to open a PR against amd/xdna-driver with the same change if desired
(with a real DCO `Signed-off-by`).

### Follow-ups (not required for this fix)

- Non-cmdlist sibling path: `aie2_execbuf` / `aie2_init_exec_req` still dereference
  `cmd_abo->client` / `EXEC_MSG_OPS` without the same guards (incident was on
  cmdlist / `force_cmdlist`, but same file).
- Null-safe `amdxdna_cmd_get_op` (and related helpers) in `amdxdna_ctx.h`.
- Full cmd BO layout validation (non-null garbage payloads).
- Fail-fast `open` when device is unhealthy; killable waits; lab `exec_cmd_allow`
  — **L3 materials** in dragon-npu (`0002-amdxdna-open-failfast-and-exec-gate.patch`);
  see addendum [`AMDXDNA_ISSUE_L3_open_failfast_addendum.md`](./AMDXDNA_ISSUE_L3_open_failfast_addendum.md).

### What this does **not** fix alone

Tasks **already** stuck in D-state on a running kernel still need unload/reboot
(or other recovery). Fail-fast open when the device is unhealthy, killable waits,
and TDR/FLR hardening are useful follow-ups (L3 materials ready; separate from
stopping the NULL deref).

### Request

1. Confirm whether AIE2 cmdlist should already validate these pointers elsewhere
   (if so, where) — if not, please accept null-guards on the fill/execbuf path.
2. Advice on any additional required validation of EXEC_CMD / cmd BO layout
   before fill_slot (we will keep userspace gated regardless).

### Attachments (when filing)

- `dmesg` / journal snippet with the oops (RIP `aie2_cmdlist_fill_slot`, key frames).
- Optional: registers if available; `ls -l /sys/bus/pci/devices/.../driver`, module
  refcnt, stack of D-state `amdxdna_drm_open` from `/proc/<pid>/stack`.

---

**Repo note:** This file is the ready-to-paste body only. Filing is a **human**
step after clean boot if desired; agents must not open the GitHub issue unless
explicitly asked.
