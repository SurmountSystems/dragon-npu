# Implement a405f5eb — idle blocked (empty reconfirm hard stop)

**Date:** 2026-08-10
**Host IMPL_ID:** `a405f5eb`
**Effort requested:** 3
**Outcome:** **No implementer spawned.** Project AGENTS.md standing rules §6–9
(freeze empty reconfirm; no mill without new material).

## User description

Re-stated residual “What still unblocks Pass” + “Agent next: Idle on empty
reconfirm…” + status/agent-offline commands. No new capture path, no named
offline code delta, no explicit unlock mint request.

## Precondition probe (this host)

| Check | Result |
|-------|--------|
| Capture dir `testdata/xdna2_cmd/external_known_good/` | PROVENANCE present; **bins=0** |
| First `C_UNLOCK=` in RESIDUAL.md | `2ae69036` **CONSUMED** |
| Active unlock | **none** |
| Product Pass / invent_pass / ran_on_npu | **No** / **0** / **No** |
| Driver/MMU | still parked |

`bash tools/pass_unblock_once.sh --status` and `--agent-offline` agree:
`blocked_on_external_bytes=1`, `HUMAN_NEEDED=1`, `Pass=No`.

## Why no implement loop

Implement skill: **do not spawn for pure status with no work.**

Project law (AGENTS.md §9): if the last runs already proved the **same** blocker
with **no new files, evidence, or named code delta**, **stop**. Do not re-queue
empty `/implement`, re-run gates only to reconfirm absent bytes, mint unlocks, or
invent Pass.

Prior closeout already landed residual Pass-unblock S0–S5 tooling:
`.agents/reports/impl-residual-pass-build-closeout.md`.

## What still unblocks Pass (unchanged)

| Need | How |
|------|-----|
| Working-stack ctrltext in the capture dir | When you have a real file: `bash tools/pass_unblock_once.sh --ingest /path/to/ctrltext.bin` |
| True matmul-matched PDI beyond spent flexml | Offline discovery + **new** unlock only with a **new** fingerprint |
| Driver/MMU orphan class | Parked until new evidence |

## Agent next

**Idle.** Resume only with:

1. Real capture file under the capture dir (or a path to `--ingest`), or
2. A named offline code/research delta the operator asked for, or
3. Explicit new host-minted unlock + named offline reason (not re-live of spent `0eeeb70c…` alone).

Unspent flexml (e.g. llm-2x4x4) remains research-eligible, not Pass, not auto-unlocked.

## Next implement prompt

**Omitted** (leading `/implement` not emitted). Remaining work is operator-gated
or evidence-gated; auto-run must not re-spin this blocked status.

**Pass=No. invent_pass=0. not_ran_on_npu=1.**
