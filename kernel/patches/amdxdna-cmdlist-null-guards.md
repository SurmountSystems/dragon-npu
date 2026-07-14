# L2 materials: null-guard AIE2 cmdlist fill/execbuf

**Status:** L2 materials complete — real patch in-tree; **not** installed on host by default  
**Incident:** 2026-07-13, 1022:17f0, oops in `aie2_cmdlist_fill_slot` after scaffold EXEC_CMD  
**Source base:** [amd/xdna-driver](https://github.com/amd/xdna-driver) @ `905717f5cd470580054a1635b9eea24b84dfd67f`  
**File:** `src/driver/amdxdna/aie2_message.c` (last touch `b9686b40a31149b4b2dd823d87be4567ba697618`)  
**Shippable patch:** [`0001-amdxdna-cmdlist-null-guards.patch`](./0001-amdxdna-cmdlist-null-guards.patch)  
**Apply/build:** [`README.md`](./README.md)  
**License:** GPL-2.0 (same as driver)

These materials harden **missing-object** NULL derefs on the cmdlist path. They are
**not** a claim that this laptop is already fixed forever, and they are **not** full
cmd-BO payload validation (non-null but garbage content can still fail deeper in
`fill_*_slot` / firmware).

## Root-cause hypothesis

`aie2_cmdlist_single_execbuf` maps a cmdlist buffer and calls:

```c
void *cmd_buf = amdxdna_gem_vmap(cmdbuf_abo);
…
ret = aie2_cmdlist_fill_slot(cmd_buf, cmd_abo, &size, &op);
```

`aie2_cmdlist_fill_slot` immediately does:

```c
struct amdxdna_dev *xdna = cmd_abo->client->xdna;
```

and later calls `EXEC_MSG_OPS(xdna)->fill_*_slot(...)`.

A **NULL** `cmd_abo`, `cmd_abo->client`, `cmd_buf`, `cmdbuf_abo`, or missing
`exec_msg_ops` yields a kernel NULL deref (observed RIP in fill_slot with RAX=0).

Userspace garbage must never reach that path without validation; the driver
must still **not oops** if it does.

## Patch goals

1. Null-check `cmd_abo`, `cmd_abo->client`, `slot` / `cmd_buf`, `cmdbuf_abo` before use.  
2. Null-check `EXEC_MSG_OPS(xdna)` before call.  
3. Prefer `-EINVAL` / `-ENODEV` / `-ENOMEM` / `-EOPNOTSUPP` over crash.  
4. Guard `job->ctx` / `job->ctx->priv` before `aie2_cmdlist_get_cmd_buf`.  
5. Fail closed if command BO vmap is NULL before `amdxdna_cmd_get_op` (helper itself still lacks a NULL check).  
6. (Follow-up / L3) Mark device unhealthy after job-path oops so later `open` fails fast.

## What this patch does **not** fully cover

| Residual | Notes |
|----------|--------|
| Non-null but **invalid layout** cmd BO | Scaffold with valid pointers can still fault or misbehave inside `fill_*_slot` / memcpy. Needs payload validation (Phase B / upstream). |
| `amdxdna_cmd_get_op` / related helpers | Prefer fixing vmap NULL in `amdxdna_ctx.h` helpers long-term; L2 probes vmap in `fill_slot` only. |
| Non-cmdlist **`aie2_execbuf` / `aie2_init_exec_req`** | Same class of `cmd_abo->client` / `EXEC_MSG_OPS` deref; out of incident `force_cmdlist` path; L2.1 follow-up if any device runs non-cmdlist. |
| Open D-state / TDR / FLR | L3; null-guards do not unstick already-wedged waiters. |

## Concrete patch (applyable)

See **`0001-amdxdna-cmdlist-null-guards.patch`**. Summary of returns:

| Check | errno |
|-------|--------|
| null `slot` / `cmd_abo` / `size` / `cmd_op` / client / job / ctx / priv | `-EINVAL` |
| null `xdna` / `dev_handle` / mailbox channel | `-ENODEV` |
| null `amdxdna_gem_vmap` result (cmdbuf or cmd BO probe) | `-ENOMEM` |
| null `EXEC_MSG_OPS` | `-EOPNOTSUPP` |

Also hardens **multi** execbuf with the same class of guards (sibling of single).

Verified with `patch --dry-run -p1` against a pristine copy of
`aie2_message.c` from the base SHA above (see README).

## Related upstream work (not a full substitute)

- PR [#1441](https://github.com/amd/xdna-driver/pull/1441): null-guard `cmd_submit` when ops missing (**aie4**).  
- Issue [#1347](https://github.com/amd/xdna-driver/issues/1347): NULL-deref on mailbox timeout teardown.  
- PR [#1468](https://github.com/amd/xdna-driver/pull/1468): aie4 cmd_submit EOPNOTSUPP test.

Search (2026-07): **no open issue** for AIE2 `fill_slot` + post-oops open wedge.
Ready-to-paste issue: [`docs/upstream/AMDXDNA_ISSUE_cmdlist_null_open_wedge.md`](../../docs/upstream/AMDXDNA_ISSUE_cmdlist_null_open_wedge.md).

## How we ship it later

1. File issue on amd/xdna-driver with oops + open-D symptoms (issue body ready).  
2. Offer this patch (or refined version after stack traces); rewrite **From/Signed-off-by** to a real DCO identity before PR.  
3. Optional local DKMS from xdna-driver git until distro catches up.  
4. Userspace remains double-gated regardless of driver version.

## What this does *not* fix alone

Tasks **already** stuck in D-state in a running kernel — only unload/reboot
clears them. This patch is the defensive L2 fix for **subsequent boots after
install**. The live host is **not** claimed to be running this module until
install is done.
