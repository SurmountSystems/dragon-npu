# Dual-live join closeout (feedback plan)

**Date:** 2026-08-11
**Scope:** docs/report join only. **No** new unlock. **No** live apply.
**Honesty:** invent_pass=**0** · Pass=**No** · ran_on_npu=**false** · Unlock ≠ Pass

---

## One-line status

Both unspent flexml STX research PDIs (llm-2x4x4 and unified-4x4) got a real
CONFIG_CU attach under separate one-shot unlocks; both failed post-attach with
`wait_cmd_einval` and **different** IO_PAGE_FAULT address bands. All three
flexml STX research fingerprints are now **Spent**. Still **no** product Pass.

---

## What worked

| Step | Result |
|------|--------|
| Offline residual pin | Dual/triple public mlir-aie npu2 material tagged **RESEARCH ONLY** with size-class tags (`public-npu2-small` vs `flexml-2x4x4-class` vs `mladf-classic-1688`). Not residual identity. Not unlock alone. |
| Third public set (optional) | whole_array matmul: ctrltext **2592 B** sha16 `7ee4b267ce31cd70`; pdi **66864** sha16 `e721b80335e6d8ed`. Still research only. |
| Shot L stock live | CONFIG_CU attach measured on flexml **llm-2x4x4** (21664 / `ba0cd738fd870011`): `config_cu=1`, `real_ioctl=true`. |
| Shot U stock live | CONFIG_CU attach measured on flexml **unified-4x4** (13792 / `7d172bf223a9b06b`): `config_cu=1`, `real_ioctl=true`. |
| Userspace contract path | aiebu ELF + rela path still green on both shots; last_apply pins written. |

Attach succeeding on three different flexml topologies is real progress vs pure
APP_LOAD-only history. It is **not** Pass.

---

## What failed

| Claim | Reality |
|-------|---------|
| Product Pass | **No** on both lives |
| `ran_on_npu` | **false** on both |
| ERT Completed + OFM allclose | **Not** observed |
| Unlock ⇒ success | Both unlocks **CONSUMED** after one failing stock shot each |

### Live L — `C_UNLOCK=8352882b` CONSUMED

| Field | Value |
|-------|--------|
| PDI | flexml **llm-2x4x4** · 21664 B · sha16 `ba0cd738fd870011` |
| last_apply | `/tmp/dragon_npu_last_apply_config_cu_2328515.json` |
| status | `wait_cmd_einval` |
| config_cu | 1 |
| real_ioctl | true |
| ran_on_npu | false |
| Pass | **No** |
| Journal | **IO_PAGE_FAULT** ~`0x4fff8000…` + `aie2_error_backtrack` + `aie2_set_cmd_timeout` (DPU/TXN `0xffffffff`) |
| Reports | `.agents/reports/live-shot-8352882b.md`, `.agents/reports/impl-feedback-live-llm-2x4x4.md` |

### Live U — `C_UNLOCK=efd44f35` CONSUMED

| Field | Value |
|-------|--------|
| PDI | flexml **unified-4x4** · 13792 B · sha16 `7d172bf223a9b06b` |
| last_apply | `/tmp/dragon_npu_last_apply_config_cu_2618482.json` |
| status | `wait_cmd_einval` |
| config_cu | 1 |
| real_ioctl | true |
| ran_on_npu | false |
| Pass | **No** |
| Journal | **IO_PAGE_FAULT** ~`0x8fff8000…` + `aie2_set_cmd_timeout` (DPU `0xffffffff`, TXN OP **`0xd`**) |
| Reports | `.agents/reports/live-shot-efd44f35.md`, `.agents/reports/impl-feedback-live-unified-4x4.md` |

---

## Residual class evolution

Residual after CONFIG_CU attach remains **`wait_cmd_einval`** (WAIT EINVAL, not a
mis-labeled pure timeout). That class held across three flexml stock shots.

**IO_PAGE_FAULT presence changed by topology:**

| Unlock | Topology | pdi_bytes / sha16 | IO_PAGE_FAULT | Notes |
|--------|----------|-------------------|---------------|-------|
| `2ae69036` | unified-2x4x4 | 27200 / `0eeeb70c84df056e` | **None** in shot window | cmd timeout / backtrack only |
| `8352882b` | llm-2x4x4 | 21664 / `ba0cd738fd870011` | **Yes** ~`0x4fff8000…` | + backtrack + timeout |
| `efd44f35` | unified-4x4 | 13792 / `7d172bf223a9b06b` | **Yes** ~`0x8fff8000…` | TXN OP `0xd` (vs L’s `0xffffffff`) |

Also historical CVML research-blob shots used a different orphan band (~`0x2fff8000…`).
Band motion with PDI topology is useful journal evidence for map/orphan research.
It is **not** Pass and does **not** unlock a re-live of any spent fingerprint.

---

## Flexml STX research PDI ledger (all Spent)

Research overlays only. **Not** product known-good. invent_pass=0.

| tag | pdi_bytes | sha16 (16) | Live |
|-----|-----------|------------|------|
| flexml STX `unified-2x4x4` | 27200 | `0eeeb70c84df056e` | **Spent** `2ae69036` — wait_cmd_einval, **no** IO_PAGE_FAULT |
| flexml STX `llm-2x4x4` | 21664 | `ba0cd738fd870011` | **Spent** `8352882b` — wait_cmd_einval + IO_PAGE_FAULT ~`0x4fff…` |
| flexml STX `unified-4x4` | 13792 | `7d172bf223a9b06b` | **Spent** `efd44f35` — wait_cmd_einval + IO_PAGE_FAULT ~`0x8fff…` |

Pure re-live of any spent sha16 alone is **banned**.

---

## Public capture material (still RESEARCH ONLY)

Under `testdata/xdna2_cmd/external_known_good/`:

| Set | Size class tag | Ctrltext (example) | Role |
|-----|----------------|--------------------|------|
| vector_scalar_add npu2 | `public-npu2-small` | 300 B / `aaeb5b68cc298a45` | Research peel |
| single_core matmul npu2 | `public-npu2-small` | 660 B / `de8671245bb72436` | Research peel |
| whole_array matmul npu2 | `public-npu2-small` (larger) | 2592 B / `7ee4b267ce31cd70`; pdi 66864 / `e721b80335e6d8ed` | Research peel |

Contrasts (not matches):

- Lab oracle **`mladf-classic-1688`** (1688 B ctrltext, rela×12 path)
- Spent research PDIs **`flexml-2x4x4-class`** (table above)

Public material ≠ residual identity ≠ unlock justification alone ≠ Pass.
Route B structure axes may be cited in a **future** unlock reason text without
claiming match. Offline reports:
`.agents/reports/impl-feedback-offline-residual-pin.md`,
`.agents/reports/impl-feedback-larger-matmul-material.md`.

---

## Unlock SSoT

| Item | Value |
|------|--------|
| First `C_UNLOCK=` in `RESIDUAL.md` | `C_UNLOCK=efd44f35` **CONSUMED** |
| Active one-shot | **None** |
| Prior CONSUMED (do not reopen) | `8352882b`, `2ae69036`, `97f38307`, `11bc9016`, `3a70fabf`, `b17b0119` |
| New unlock from this join | **None** (join closeout only) |

Unlock ≠ Pass ≠ `ran_on_npu`. invent_pass=0.

---

## Frozen (do not do)

1. Re-live spent flexml sha16s: `0eeeb70c…`, `ba0cd738…`, `7d172bf2…`
2. Reopen any CONSUMED `C_UNLOCK`
3. Invent Pass / invent `ran_on_npu` / Pass-from-attach / Pass-from-capture
4. Empty reconfirm mill (same residual, no new material or named offline delta)
5. Agent live apply without a **new** host-minted unlock + named offline reason
6. Sticky `XDNA2_ALLOW_EXEC_CMD` / permanent L3 open
7. Claim dual/triple public npu2 bins are load-accepted residual identity

---

## Ranked next (honest)

1. **Optional offline:** IO_PAGE_FAULT band compare (`0x4fff` llm vs `0x8fff`
   unified-4x4 vs **none** on unified-2x4x4) against lab BO map / orphan audit
   tools already in tree. invent_pass=0; audit alone ≠ Pass.
2. **Material:** residual-matched or load-accepted image beyond spent flexml
   research PDIs and beyond `public-npu2-small` research peels. Soft classify
   present capture is fine; expect mismatch.
3. **Next live only** under a **new** unlock + **named** offline reason (new
   material and/or driver/MMU evidence). Topology re-spin of spent flexml alone
   is not enough.
4. Keep frozen list above.

---

## Consistency check (this join)

| Check | Result |
|-------|--------|
| First `C_UNLOCK=` CONSUMED | Yes — `efd44f35` |
| Active unlock | **None** |
| Pass | **No** |
| invent_pass | **0** |
| All three flexml STX Spent | Yes |
| Residual / AGENTS top aligned | Yes (no dishonest fix needed) |

---

## Source reports

- Offline pin: `.agents/reports/impl-feedback-offline-residual-pin.md`
- Larger matmul material: `.agents/reports/impl-feedback-larger-matmul-material.md`
- Live L: `.agents/reports/live-shot-8352882b.md`, `.agents/reports/impl-feedback-live-llm-2x4x4.md`
- Live U: `.agents/reports/live-shot-efd44f35.md`, `.agents/reports/impl-feedback-live-unified-4x4.md`
- Prior spent no-fault pin: `.agents/reports/live-shot-2ae69036.md`, `.agents/reports/impl-u4-flexml-live-closeout.md`
- SSoT: `RESIDUAL.md` top; `AGENTS.md` Latest Turn live closeouts
