# Why Max could not reproduce: zen stock ≠ mainline amdxdna

Lab host runs **linux-zen 7.1.3**. Original oops was on **in-tree** zen
`amdxdna`. Max tested **latest amd/xdna-driver** (and suggested latest mainline).
Those are different codebases for the critical `fill_slot` path.

## Three trees

| Tree | How `fill_slot` gets `xdna` | Short START_NPU payload |
|------|----------------------------|-------------------------|
| **Stock zen 7.1.3** (this laptop’s packaged `.ko`) | **`cmd_abo->client->xdna`** (see disasm) | has `fill_npu_dpu` helpers; crash can happen **before** them if `client` is NULL |
| **amd/xdna-driver** staging (Max’s test) | still **`cmd_abo->client->xdna`** (source) | rejects short payload at **`-EINVAL`** (`cmd_len < sizeof(*sn)`) — Max’s L842 |
| **torvalds/linux master** | **`to_xdna_dev(to_gobj(cmd_abo)->dev)`** | same class of length check |

## Smoking gun: zen stock binary matches the oops

Decompressed:

`/lib/modules/7.1.3-zen1-2-zen/kernel/drivers/accel/amdxdna/amdxdna.ko.zst`  
→ srcversion **`B9ED3E2DD923D9C8E3F4EDB`**, intree **Y**, vermagic **7.1.3-zen1-2-zen**.

`llvm-objdump -d` of `aie2_cmdlist_fill_slot`:

```text
aie2_cmdlist_fill_slot:
  ...
  movq  0x1b0(%rsi), %rax    # load field from cmd_abo (client @ +0x1b0 on this build)
  movq  %rdx, (%rsp)
  movq  0x50(%rax), %rbp     # load through that pointer (xdna @ client+0x50)
  ...
```

Lab oops (2026-07-13 22:00:51):

```text
Code: ... 48 8b 86 b0 01 00 00 48 89 14 24 <48> 8b 68 50 ...
RAX: 0000000000000000
CR2: 0000000000000050
RIP: aie2_cmdlist_fill_slot+0x26/0x130 [amdxdna]
```

That is the **same instruction sequence**: load from `cmd_abo+0x1b0` into RAX, then
`mov 0x50(%rax)`. Crash when RAX is NULL → fault address `0x50`.

Mainline does **not** use this prologue for `xdna` (uses `to_xdna_dev(...)` instead).

## What Max saw

With **current** xdna-driver + our incomplete START_NPU-style CMD BO, the job path
reaches `aie2_cmdlist_fill_npu_dpu` and returns **`-EINVAL`** on short payload.
No need for a NULL `client`; the command is rejected on length.

So:

- Max is right that **his tree** does not need this PR for that repro.
- We are right that **zen 7.1 stock** has the **client-indirect** `fill_slot` and
  the oops bytes match that binary.
- “Different code” is the bridge: **distro zen package ≠ latest xdna-driver ≠ tip mainline**.

## Lab module vs stock (do not confuse)

| Module | path | notes |
|--------|------|--------|
| Stock zen | `.../kernel/drivers/accel/amdxdna/amdxdna.ko.zst` | intree, ~0.8.0 era strings |
| Lab hardened | `.../updates/amdxdna-hardened/amdxdna.ko` | OOT, `exec_cmd_allow`, wins over stock via `modprobe` |

`modprobe amdxdna` **always** picks updates/ if present. To test stock, hide updates
(see `scripts/check_amdxdna_zen_and_mainline.sh` / prior notes).

## Empirical Track A still open

Hiding updates + load stock + run repro would confirm zen stock behavior live.
CREATE_HWCTX may still need heap/QoS care; fill_slot only runs after a successful
submit path.

## For your notes / optional future message (your words only)

Facts you can use if you write later:

1. Stock zen 7.1.3 `fill_slot` disassembly matches the oops machine code (client load).
2. Mainline rewrote that to `to_xdna_dev`.
3. Latest xdna-driver rejects the incomplete CMD with -EINVAL before deeper damage.
4. That is why Max could not reproduce on latest driver; the lab kernel package is behind.

No requirement to reopen PR #1493.
