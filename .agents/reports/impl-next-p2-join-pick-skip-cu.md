# Implement report: Phase 2 join pick — skip-CONFIG_CU isolation

**Date:** 2026-08-11
**Repo:** `/home/hunter/Projects/ai/dragon-npu`
**Mode:** Offline only. **No unlock mint. No live apply.** invent_pass=0.
**First `C_UNLOCK=` remains CONSUMED `efd44f35` until Phase 3 mints.**

---

## Chosen experiment

**Name:** `skip-CONFIG_CU isolation`

**Why primary:** Offline A closed map audit with **no fixable userspace map/bind
bug** (all multi-shot IO_PAGE_FAULT addresses **orphan** vs lab BO map). Offline
B ranked skip-CONFIG_CU isolation as the top live experiment once all three
flexml STX research PDIs are Spent on CONFIG_CU attach. The next useful live
measurement is lab classic START_NPU **without** flexml/CVML overlay, so EXEC
residual can be separated from overlay residual.

**Why not map-fix live:** There is no named product bug of the form “forgot to
pin BO X” or “instr paddr wrong by a page.” DevHeap stays `0x4000000`, instr
`0x4028000`, I/O handles valid. Faults sit in high bands (`0x4fff8000…` /
`0x8fff8000…` / prior `0x2fff8000…`) far outside lab spans. Live map surgery
would invent a fix without evidence. Map audit ≠ Pass ≠ unlock.

---

## Inputs joined

| Report | Result used |
|--------|-------------|
| `.agents/reports/impl-next-fault-vs-bo-map.md` | fixable map bug? **no**; primary **orphan**; bands 4fff/8fff; `2ae69036` none |
| `.agents/reports/impl-next-load-path-offline.md` | primary live: skip-CONFIG_CU via kit `config-cu-run-skip` / `run-stock-skip-pdi`; B1 hypothesis only; C6 offline only |

---

## Residual / unlock state (this pin)

| Item | Value |
|------|--------|
| Active unlock | **none** |
| First `C_UNLOCK=` | `efd44f35` **CONSUMED** (do not reopen) |
| Prior CONSUMED | `8352882b`, `2ae69036`, `97f38307`, `11bc9016`, `3a70fabf`, `b17b0119` |
| invent_pass | **0** |
| Pass / `ran_on_npu` | **No** |
| Spent flexml refuse | **on** (`0eeeb70c` / `ba0cd738` / `7d172bf2` re-live alone banned) |
| Mint this slice | **No** (Phase 3 may mint) |

Disk pins: `RESIDUAL.md` top (named pending experiment), `AGENTS.md` Latest Turn.

---

## Phase 3 exact commands (after new host-minted unlock lands in RESIDUAL)

Do **not** run live under CONSUMED `efd44f35`. Only after Phase 3 mints a **new**
`C_UNLOCK=` line without CONSUMED:

```bash
# 0) Confirm unlock is active (first C_UNLOCK= line must not say CONSUMED)
grep -m1 -oE 'C_UNLOCK=[0-9a-fA-F]+' RESIDUAL.md
# optional status
bash tools/pass_unblock_once.sh --status

# 1) Offline prep for skip path (no EXEC; safe anytime)
bash tools/operator_app_load_kit.sh config-cu-skip
# equivalent: bash tools/hitl_config_cu_once.sh prep-skip-pdi

# 2) Env for skip isolation (no PDI overlay; no sticky ALLOW)
unset XDNA2_LAB_PDI_BLOB
export XDNA2_LAB_SKIP_CONFIG_CU=1
unset XDNA2_ALLOW_EXEC_CMD
# do not export spent flexml/CVML sha as PDI

# 3) One stock skip live under the NEW unlock only
bash tools/operator_app_load_kit.sh config-cu-run-skip
# equivalent: bash tools/hitl_config_cu_once.sh run-stock-skip-pdi

# 4) Post: collect last_apply + journal; mark unlock CONSUMED; invent_pass=0
# Expect possible: config_cu absent; wait_cmd_einval / wait_timeout / APP_LOAD /
# journal timeout or IO_PAGE_FAULT. Success still requires ERT Completed +
# non-zero OFM allclose. Unlock ≠ Pass.
```

Kit aliases and spent refuse live in `tools/operator_app_load_kit.sh` /
`docs/HITL_C_APP_LOAD_RECIPE.md` Step 2 skip subsection /
`docs/CONSTRUCTION_MAP_APP_LOAD.md`.

---

## Unlock reason (paste at Phase 3 mint)

Prior flexml lives `2ae69036` / `8352882b` / `efd44f35` attached (CONFIG_CU) then `wait_cmd_einval`; llm/4x4 logged IO_PAGE_FAULT orphan bands (~0x4fff… / ~0x8fff…) vs lab BO map (DevHeap 0x4000000 / instr 0x4028000); unified-2x4x4 had no IO_PAGE_FAULT. Offline map audit: orphan only, no fixable userspace map/bind bug. Offline load-path note: lab classic instr pack key vs flexml overlay is hypothesis only. This unlock runs **one** stock shot with **CONFIG_CU skipped** to isolate EXEC residual from overlay residual. Not a re-live of spent flexml fingerprints. Success still requires ERT Completed + non-zero OFM allclose. Unlock ≠ Pass.

---

## Honesty lock

| Claim | Truth |
|-------|--------|
| Product Pass / `ran_on_npu` | **No** |
| invent_pass | **0** |
| Map audit alone unlocks live | **No** |
| B1 proven root cause | **No** (hypothesis) |
| skip-CU prep = Pass | **No** |
| Unlock minted this pin | **No** |
| Live apply this pin | **No** |
| Re-live spent flexml | **Banned** |
| Reopen CONSUMED | **Banned** |

---

## Artifacts

- This report: `.agents/reports/impl-next-p2-join-pick-skip-cu.md`
- A: `.agents/reports/impl-next-fault-vs-bo-map.md`
- B: `.agents/reports/impl-next-load-path-offline.md`
- Residual / AGENTS: `RESIDUAL.md` top, `AGENTS.md` Latest Turn
