# Live shot waste modes and offline countermeasures

**Date:** 2026-08-11
**Kind:** Plan / ops checklist (docs only).
**Honesty:** invent_pass=**0** · Pass=**No** · unlock=**none** (first `C_UNLOCK=` = `4008e6fd` **CONSUMED**; do not reopen). Offline checklist ≠ Pass ≠ `ran_on_npu` ≠ unlock mint.

**Sources:** `RESIDUAL.md` top (W10 join); `.agents/reports/wave7-join.md` … `wave10-join.md`; `wave10-b-same-shot.md`; `wave9-b-l1b-prep.md`; live-shot reports (`3a70fabf` timeout, `2ae69036` flexml, `48da6e60` Shmem debug BO, …); `docs/HITL_C_APP_LOAD_RECIPE.md`; `docs/CONSTRUCTION_MAP_APP_LOAD.md` header; `tools/silicon_lab_preflight.sh`; `tools/collect_post_c2_evidence.sh`; `tools/journal_npu_scrape.sh`; `tools/operator_app_load_kit.sh`.

---

## Plain English goal

A live unlock is a **one-shot**. Wasting it means you burn the stamp and learn almost nothing (or re-prove a known failure). This note lists concrete waste modes, the offline check that catches each one **before** L3/apply, and what to force on disk after any shot so a fail still teaches.

---

## Waste modes (concrete) and offline countermeasures

| # | Waste mode | What you burn / learn poorly | Offline countermeasure (do before live) |
|---|------------|------------------------------|----------------------------------------|
| 1 | **Forgot same-shot rela gate for L1b** | Rela uses aged host UVAs from disk last_apply / vendor JSON; map is not this process’s BOs. Result is noise, not Bridge B. | For L1b: set `XDNA2_LAB_SAME_SHOT_RELA=1` and keep `XDNA2_LAB_OFM=1`. Confirm soft plan tokens: `same_shot=1`, `apply_ready=1`, `refuse_stale_disk=1`. Product path must **never** load disk UVAs (`plan_same_shot_rela_from_disk_extract` always `apply_ready=0`). Soft: `l1b_live_justified` still needs **active unlock** (offline stays 0). |
| 2 | **Patched from disk UVAs** (stale last_apply / vendor paddrs as live patch source) | Same as #1; W9 found paddrs on disk but host UVAs **age out across processes**. | Refuse product live patch from `/tmp/dragon_npu_last_apply_*.json` or `gt_capture/vendor_paddrs*.json`. Disk extract = research only. Live map only from **in-process** BO `xdna_addr` after create. |
| 3 | **OFM golden missing or all-zero** | Even a good device path cannot claim Pass; `decide_ran_on_npu` refuse; shot teaches layout only. | `bash tools/silicon_lab_preflight.sh --require-pass-assets` and `bash tools/validate_mladf_ofm_golden.sh`. Expect non-zero 512 B bf16 or 1024 B f32 under stage / `XDNA2_LAB_OFM_GOLDEN`. `XDNA2_LAB_ALLOW_ZERO_GOLDEN` must **not** be used for Pass-ready. Stage: `tools/stage_silicon_lab_assets.sh`. |
| 4 | **Outer wait / lab timeout too short** | Process killed mid-flight (`timeout` rc=124 → lab exit 4); **no last_apply** (e.g. `3a70fabf`). Unlock consumed, residual class unknown. | CONFIG_CU silicon: `--timeout-s=120` (or ≥120) on `hitl_c_pass_apply` / kit `config-cu-run`. Default outer **30s is too short**. WAIT stays finite: `XDNA2_WAIT_TIMEOUT_MS` ~5–15s (never infinite by default). Prefer progressive last_apply pin so kill after CONFIG_CU still leaves JSON. |
| 5 | **Re-live a research PDI that already failed (spent fingerprint)** | Same residual class as prior shot (e.g. flexml `0eeeb70c…` / 27200 under `2ae69036`; CVML `a1e65d04…`). Zero new information. | Fingerprint pack + residual ledger: refuse pure re-export of spent sha16 as “new” progress. Prefer unspent flexml topology or new capture under `testdata/xdna2_cmd/external_known_good/`. Recipe Step 2: spent re-live alone is **not** a named unlock reason. Soft: `research_pdi_not_product_known_good=1`. |
| 6 | **Left userspace ALLOW or L3 sticky** | Host left dangerous; second accidental EXEC; honesty breach; wedge risk. | Never leave sticky: after sandwich restore `exec_cmd_allow=0` (hardened). Unset `XDNA2_ALLOW_EXEC_CMD`. Post: `collect_post_c2_evidence` / kit `post-evidence` must show L3 closed. hitl_c exit **5** if L3 still open. |
| 7 | **No last_apply pin** | Shot finishes or dies with no structured JSON; agent cannot classify; journal alone is weak. | Before apply: `export XDNA2_LAB_LAST_APPLY_JSON=/tmp/dragon_npu_last_apply_config_cu_$$.json` (shot-specific; keep locked fixture for C3 baseline). Kit `config-cu` / `hitl_c` with PDI auto-pins when unset. After: path must exist and be shared. |
| 8 | **No journal scrape** | Miss IO_PAGE_FAULT band, APP_LOAD `0x3000003`, `aie2_set_cmd_timeout`, `Get bo failed`, orphan vs mapped addr. Same product status, different root. | Immediately after restore: `bash tools/journal_npu_scrape.sh` and/or `bash tools/collect_post_c2_evidence.sh /path/to/last_apply.json` (runs journal + p5 pack). |
| 9 | **Multi-txn 32208 as “primary” / load-accepted** | Wrong body (historical false CLASS C); not classic 1688 residual identity; invent residual_matched risk. | Offline refuse: host scan / `collect_load_accepted_candidate` / capture tools flag `multi_txn_32208`. Do not ingest 32208 as residual match. Prefer classic **1688** oracle path; multi = diagnostic only. |
| 10 | **Raw `.bin` EXEC instead of aiebu ELF** | Known APP_LOAD / load-reject path without rela×12; re-proves closed residual. | Preflight + hitl_c force ELF magic `7f454c46` (~32264 B `lab_instr.elf`). Stage: `bash tools/stage_aiebu_lab_instr.sh`. Refuse silent `lab_instr.bin` EXEC (no `--blob` override on C). |
| 11 | **Debug BO wrong type (Shmem) under research gate** | Assign fails before EXEC (`debug_bo_failed` / journal `Get bo N failed`); spent unlock on type mistake (`48da6e60`). | Offline C6: pack is value-mode BO handle; lab type must be **Dev** 4096 (`AMDXDNA_BO_DEV`), not Shmem. Env `XDNA2_LAB_RESEARCH_DEBUG_BO=1` only when intentional. Do not re-live spent Shmem path. |
| 12 | **Skip-CONFIG_CU reconfirm with no new named reason** | Re-prove `wait_cmd_einval` without overlay under CONSUMED unlock; empty reconfirm. | Live only under **new** unlock + named offline delta. Offline prep: `operator_app_load_kit.sh config-cu-skip` / preflight. Isolation already proven under `1f761bf4` (CONSUMED). |
| 13 | **Public mlir-aie AOT / residual-mismatched instruction as “Pass material”** | Research peels (300/660/2592/4512) ≠ lab classic 1688 identity; live only re-proves residual mismatch. | Public AOT re-run **frozen** for residual identity. Soft: `residual_matched=0`, research size-class tags. Use only as research classify offline. |
| 14 | **Missing WTS / ctrl non-zero assets** | Pre-EXEC fail or meaningless OFM zeros; not a firmware residual class. | Preflight `--require-pass-assets`: WTS 174080 non-zero, ctrl 29664 non-zero, meta present. `validate_mladf_wts` / `validate_mladf_ctrl` / stage scripts. |
| 15 | **Live without active unlock / reopen CONSUMED** | Policy violation; untracked shot; stamp ledger lies. | Grep first `C_UNLOCK=` in `RESIDUAL.md`: active only if **no** CONSUMED on that line. Kit / hitl_c2 `--print-only` must show active stamp before sandwich. Do not reopen CONSUMED list. |
| 16 | **Outer timeout kill with no progressive stamp** | Same family as #4: mid-ioctl death, empty evidence. | Prefer progressive last_apply after hwctx / CONFIG_CU / pre-submit (product path). Outer ≥120s for CONFIG_CU attach shots (~60s seen on flexml). |
| 17 | **Sticky skip-contract / force-apply as “unlock”** | Offline red papered over; fail mode shifts; stamp still burns. | Default A9 + preflight hard. Unset `XDNA2_HITL_SKIP_CONTRACT` after any research re-apply. Force ≠ unlock. |
| 18 | **Second live under same unlock** | Policy: one-shot only; second is unauthorized open season. | Mark CONSUMED after **one** real sandwich with last_apply. New live needs **new** host-minted unlock + named reason. |

---

## Maximum learning if fail (required evidence package)

If the shot fails (or even if it “almost” works), require this package so the unlock is not wasted.

### Files (must exist and be shared)

| File | Why |
|------|-----|
| **Shot-specific last_apply JSON** | e.g. `/tmp/dragon_npu_last_apply_config_cu_<pid>.json` — status, detail tokens, BO diag, config_cu/pdi, wait axes |
| **Journal scrape output** | Window covering the shot: APP_LOAD / IO_PAGE_FAULT / timeout / Get bo / backtrack |
| **PDI / instruction identity** | Path + size + sha16 of `XDNA2_LAB_PDI_BLOB` (if any) and instr ELF path/size |
| **Env / kit printout** | Prep lines: timeout, SAME_SHOT, SKIP_CONFIG_CU, RESEARCH_DEBUG_BO, LAST_APPLY pin, ALLOW unset |
| **L3 post state** | `exec_cmd_allow` readback closed (or missing on stock) |
| **Optional** | Soft classify / p5 / c3 / c4 / p6 stdout; live-shot report under `.agents/reports/live-shot-<unlock>.md` |

Do **not** rely only on chat memory or a locked fixture `testdata/xdna2_cmd/last_apply.json` for C3 “new evidence.”

### Commands (run after restore, before declaring “done”)

```bash
# 1) L3 + last_apply + journal + p5 pack (preferred one-shot)
bash tools/collect_post_c2_evidence.sh /tmp/dragon_npu_last_apply_config_cu_….json
# same family:
# bash tools/operator_app_load_kit.sh post-evidence /tmp/dragon_npu_last_apply_config_cu_….json

# 2) Soft residual class (≠ unlock, ≠ Pass)
bash tools/classify_last_apply_residual.sh
# or:
# bash tools/p5_post_c2_interpret.sh /tmp/dragon_npu_last_apply_….json

# 3) Journal alone if collect was skipped
bash tools/journal_npu_scrape.sh

# 4) True-Pass criteria audit only if last_apply claims near-Pass (still ≠ invent Pass)
bash tools/p6_true_pass_check.sh
bash tools/c4_pass_check.sh
```

Offline **before** the next unlock (not a substitute for the package above):

```bash
bash tools/silicon_lab_preflight.sh --require-pass-assets
bash tools/run_offline_gates.sh
# CONFIG_CU prep / skip prep without L3:
bash tools/operator_app_load_kit.sh config-cu          # or config-cu-skip
bash tools/hitl_config_cu_once.sh prep                 # or preflight
```

### Tokens to require on last_apply (greppable)

Minimum set for a **finished** apply (missing ⇒ incomplete / waste risk):

| Token / field | Honest meaning if present |
|---------------|---------------------------|
| `status=` | Product class (`wait_cmd_einval`, `wait_timeout`, `debug_bo_failed`, `ok_npu_verified`, …) |
| `ran_on_npu=` | Must stay **false** unless OFM allclose + ERT Completed + real ioctl decide |
| `real_ioctl=` | **true** for a real device shot |
| `aiebu_elf=` / rela | Userspace ELF+rela path used (not raw bin) |
| `config_cu=` / `config_cu_present=` | 0/1 whether CONFIG_CU ran |
| `pdi_bytes=` + pdi sha16 in detail or report | Overlay identity (or absent if skip-CU) |
| `timeout_ms=` on wait | WAIT budget used |
| `ert_state=` when readable | Completed vs Error vs unknown |
| `io_paddrs` **or** `bo_diag` ifm/wts/ofm/ctrl | Map audit / same-shot evidence for next offline dig |
| `instr` xdna_addr / map_offset when BO diag on | Orphan vs mapped fault work |
| `debug_bo_ty=` / `lab_debug_bo_attach=` | Only if research debug BO path |
| Soft honesty on classify | Always: `invent_pass=0` `not_c_ready=1` `not_ran_on_npu=1` |

**Incomplete shot** (timeout kill): path missing → collect refuses with `incomplete_timeout_or_missing_pin=1`. That is a **timeout class**, not APP_LOAD invent. Next live needs outer ≥120s + pin + progressive stamp.

**L1b-oriented shot** (when unlocked + justified): also require same-shot soft notes (`same_shot=1`, in-process source) and never claim residual_matched from paddrs alone.

---

## One-page pre-live gate (print and check)

- [ ] First `C_UNLOCK=` in `RESIDUAL.md` is **active** (not CONSUMED)
- [ ] Named offline reason written (not “reconfirm spent flexml alone”)
- [ ] Preflight `--require-pass-assets` green (ELF, WTS nz, ctrl nz, OFM golden nz)
- [ ] Instr is **ELF**, not raw `.bin`
- [ ] PDI (if used) is **unspent** or new capture; fingerprint known
- [ ] Not multi-txn 32208 as primary residual
- [ ] Outer `--timeout-s=120` (or ≥120) for CONFIG_CU
- [ ] `XDNA2_LAB_LAST_APPLY_JSON` shot-specific pin set
- [ ] L1b only: `XDNA2_LAB_SAME_SHOT_RELA=1` + OFM path; **no** disk UVA patch
- [ ] Debug BO only if intentional: **Dev** 4096, not Shmem
- [ ] No sticky `XDNA2_ALLOW_EXEC_CMD`; restore L3 after
- [ ] Post pack: collect_post + journal + classify; mark unlock **CONSUMED** after one real sandwich

---

## Honesty lock

- invent_pass=**0** always on soft / offline tools.
- Offline green ≠ Pass ≠ `ran_on_npu` ≠ unlock mint.
- Same-shot offline wire **ready** (W10-B) ≠ live L1b justified.
- Stale disk UVAs **forbidden** for product live patch.
- Public AOT re-run **frozen** for residual identity.
- Do **not** reopen CONSUMED unlocks. Do **not** mint from this checklist alone.

---

## Related paths

| Path | Role |
|------|------|
| `/home/hunter/Projects/ai/dragon-npu/RESIDUAL.md` | Unlock SSoT (first `C_UNLOCK=`) |
| `/home/hunter/Projects/ai/dragon-npu/docs/HITL_C_APP_LOAD_RECIPE.md` | Sandwich + Step 2 checklist |
| `/home/hunter/Projects/ai/dragon-npu/docs/CONSTRUCTION_MAP_APP_LOAD.md` | CU/PDI vs ctrltext construction |
| `/home/hunter/Projects/ai/dragon-npu/tools/silicon_lab_preflight.sh` | Pass-asset offline gate |
| `/home/hunter/Projects/ai/dragon-npu/tools/collect_post_c2_evidence.sh` | Max post-shot paste pack |
| `/home/hunter/Projects/ai/dragon-npu/tools/operator_app_load_kit.sh` | config-cu / post-evidence umbrella |
| `/home/hunter/Projects/ai/dragon-npu/.agents/reports/wave10-join.md` | Same-shot + L1b wall |
| `/home/hunter/Projects/ai/dragon-npu/.agents/reports/live-shot-3a70fabf-timeout.md` | Timeout waste case study |
| `/home/hunter/Projects/ai/dragon-npu/.agents/reports/live-shot-2ae69036.md` | Spent flexml attach + wait_cmd_einval |

---

**Report path:** `/home/hunter/Projects/ai/dragon-npu/.agents/reports/plan-live-shot-waste-modes.md`
