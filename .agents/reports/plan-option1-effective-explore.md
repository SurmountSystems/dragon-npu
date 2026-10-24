# Explore: Make option 1 (product NPU matmul + same-run patch) most effective next

**Date:** 2026-08-11
**Kind:** Read-only plan input after Arm A live under `C_UNLOCK=2c9c12e8` CONSUMED.
**Honesty:** invent_pass=0 · Pass=**No** · unlock **none** (first `C_UNLOCK=` = **`2c9c12e8` CONSUMED**) · same_shot measured ≠ Pass.

**Sources:** `RESIDUAL.md` top; `.agents/plans/plan-live-shot-effective.md`;
`.agents/reports/live-shot-2c9c12e8.md`; `.agents/reports/impl-live-shot-effective-s4-s5.md`;
`.agents/reports/plan-live-shot-waste-modes.md`; `.agents/plans/plan-silicon-try-everything.md`;
`.agents/reports/wave9-a-bridge-a.md`; `.agents/reports/wave10-join.md`;
`docs/CONSTRUCTION_MAP_APP_LOAD.md`; offline WAIT dig `49feb5d2` / wave0 uAPI.

---

## Plain English

Option 1 was: **one product matmul** with **same-run address patching**, Pass only via
`decide_ran_on_npu` (ERT Completed + OFM allclose + real_ioctl). That experiment
**succeeded as science** and **failed as Pass**. Do **not** re-run the same Arm A
recipe under a new mint just to reconfirm same-shot + `wait_cmd_einval`.

Most effective next work changes **what the firmware sees as the job**
(instruction identity / load-accepted material) and/or **how completion is waited**
(syncobj parity), not another empty same-shot burn.

---

## 1. What option 1 already proved (do not re-test)

| Proven fact | Evidence |
|-------------|----------|
| Same-shot product path works **live** | `same_shot_gate=1 same_shot=1 paddrs_valid=1 source=in_process_bo_map refuse_stale_disk=1` under `2c9c12e8` |
| Kit + env + outer budget can fire stock Arm A | `config-cu-run-skip`, outer ≥120s, last_apply under `/tmp`, BO_DIAG + OFM on |
| Userspace ELF+rela×12 on lab classic 1688 still green on live path | aiebu_elf=1, rela×12, content/start_npu=1688, pack key classic 1152×256 |
| EXEC residual **persists without CONFIG_CU overlay** (reconfirm of skip-CU) | Same class as `1f761bf4`: `wait_cmd_einval`, total_s≈0.16–0.17, `config_cu` absent |
| Same-shot is **necessary hygiene, not sufficient for Pass** | Residual class unchanged with same-shot measured |
| Stale disk UVAs are the wrong L1b story | W9–W10 + live refuse_stale_disk=1 |
| Prep S0–S5 of live-shot-effective is **done** | Plan closed by S4–S5 CONSUME |

**Do not re-test:** offline same-shot wire alone; empty same-shot reconfirm live;
skip-CU isolation alone; spent flexml re-live; public IRON AOT as residual identity;
oracle unpatched re-capture as progress.

---

## 2. What still blocks Pass for option 1

Pass still requires **all** of:

1. Live `real_ioctl` on product lab path
2. ERT **Completed**
3. Non-zero OFM **allclose** vs golden
4. Via product **`decide_ran_on_npu`**

After Arm A same-shot, the measured wall is still:

| Layer | State |
|-------|--------|
| Address patch same-run | **Solved live** |
| Submit envelope (aiebu+rela, BO map) | Userspace OK; WAIT fails immediately |
| Residual class | **`wait_cmd_einval`** (WAIT_CMD EINVAL, hwctx=1 seq=0 timeout_ms=5000) |
| Instruction **identity** | Still **unpatched oracle** classic 1688 (`3c5ffca750922d3b`) + pure-direct rela — **not** completing-stack post-patch / load-accepted peel |
| CONFIG_CU / PDI | Absent this shot; with spent flexml still post-attach `wait_cmd_einval` historically — overlay alone did not unlock compute |
| `residual_matched` / `load_accepted` | **0 / 0** |
| Primary wall (SSoT) | Residual-matched / load-accepted completing-stack material for ERT Completed + OFM allclose |

**Bridge B hypothesis (same-shot alone can Pass without vendor post-patch):** still
**open as theory**, but **weakened as near-term bet** by live same-shot + same class.
Next live on Bridge B alone needs a **new** completion/wait theory or material change,
not “same-shot again.”

**Bridge A hypothesis (FW needs residual-matched / load-accepted body):** still the
**highest leverage gap** for option-1-class Pass.

---

## 3. Ranked effectiveness levers

| Rank | Lever | Why it moves option 1 | Who / mode |
|------|-------|------------------------|------------|
| **1** | **Bridge A: residual-matched / load-accepted classic 1688 (or completing-stack peel)** past INSTR_BO_LIMIT / private `run_` / EP intercept / instrumented DD | Changes instruction identity the FW actually runs; unpatched+same-shot already failed live | **Agent offline** hunt first; **human** only for sudo L3 / network install / hand peel if proven wall |
| **2** | **Wait-path parity: retain CREATE_HWCTX `syncobj_handle` + optional drm_syncobj wait (XRT fallback is WAIT_CMD)** | Product always WAIT_CMD; XRT prefers syncobj when valid. Offline hypothesis H8 from wave0 — **not proven root**, but **new** named reason if offline pack+wire green and last_apply still shows wait_cmd_einval with same-shot | **Agent offline** (pack already partially landed W3–W4); **needs new unlock live** only after offline green + named wait theory |
| **3** | **True post-patch body from same completing stack** (not GT synthetic paddrs `ccab3960…`, not multi 32208) | Vendor post-patch path public API wall (INSTR_BO_LIMIT); siblings = other dims only (W9-A) | **Agent offline** intercept design + host tools; **human** for large DD/XRT instrumented build if required |
| **4** | **Arm B CONFIG_CU only with matmul-matched / unspent / new PDI reason** | Spent flexml attach already → wait_cmd_einval; re-burn is waste | **Agent offline** fingerprint + refuse spent; **new unlock live** only if new material |
| **5** | **Offline WAIT axes dig with new last_apply only** (cmd BO lifecycle, seq, fence, journal fence-miss string) | Axes tools already exist (`49feb5d2`); dig is useful only if **new** evidence, not re-parse `2c9c12e8` forever | **Agent offline** |
| **6** | **Driver / hardened L3 / MMU** | No fixable userspace map bug (orphan faults); stock path already real_ioctl | **Human** sudo / reboot walls only when agent proves need |
| **7** | **Public mlir-aie AOT as EXEC body** | Research-only; residual-mismatched to lab 1688; live would re-prove mismatch | **Freeze** for residual identity (classify offline OK) |

### Mode tags (for plan steps)

| Tag | Meaning |
|-----|---------|
| **agent offline** | Pure TDD, host scan, intercept design, docs — no unlock |
| **needs new unlock live** | One stock/product shot under **new** host-minted `C_UNLOCK` + **new** named reason |
| **human capture** | Place load-accepted peel, sudo L3, network install, or large source build OK |

---

## 4. Waste modes to freeze (empty same-shot reconfirm and kin)

Do **not** burn a new unlock on any of these alone:

1. **Empty same-shot reconfirm** — Arm A again with only “same_shot=1 already proven”
2. **Skip-CONFIG_CU isolation reconfirm** — already `1f761bf4` + `2c9c12e8`
3. **Re-live spent flexml / CVML PDI fingerprints** alone
4. **Stale disk UVA patch** as product L1b
5. **Public AOT re-run** as residual match / Pass material
6. **Re-capture unpatched oracle** as progress
7. **Second live under CONSUMED** unlock (`2c9c12e8` and priors)
8. **Syncobj wire alone → L8 live** without new wait theory + unlock
9. **Invent Pass** from soft tokens, IRON PASS, attach, same_shot, or offline checklist
10. **Outer 30s kill / no last_apply** (already fixed for good shots; keep ≥120s)
11. **Sticky ALLOW / L3 left open**
12. **Mint unlock from prep green alone** without named residual delta

---

## 5. Recommended NEW plan phases (not re-run S0–S5 prep)

**Plan name (suggested):** option1-after-same-shot — material + wait theory before next burn.

| Phase | Work | Mode | Exit / success |
|-------|------|------|----------------|
| **P0** | Residual honesty pin: same-shot live proven; primary wall = Bridge A material + optional wait-path theory; freeze empty reconfirm; unlock none | agent offline | First `C_UNLOCK=` stays CONSUMED; invent_pass=0 |
| **P1-A** | Bridge A material track: private `run_` / EP / instrumented DD intercept design; host hunt; refuse multi 32208 / oracle rematch as match | agent offline | Either residual-matched classic 1688 sha ≠ oracle **or** exact wall re-documented with new avenue tried |
| **P1-B** | Wait-path offline: ensure `syncobj_handle` retained on `HardwareContext` + last_apply token; unit pack for syncobj timeline wait args; soft note invent_pass=0 | agent offline | Wire green offline; **≠ Pass ≠ unlock** |
| **P2** | Join: rank **one** next live reason in plain English (material arm **or** wait-path arm **or** deliberate dual arm with two unlocks — never dual under one stamp) | agent offline | Written named reason; refuse if neither arm gained evidence |
| **P3** | **Only if P2 has new reason:** mint **one** new unlock + **one** live (prefer material arm if material landed; else wait-path arm with same-shot still on) | needs new unlock live | last_apply + journal + classify; CONSUME; Pass only via decide |
| **P4** | Interpret + residual pin; if fail, next offline delta from **new** class only | agent offline | Stop empty mill |

**Default order:** **P1-A parallel with P1-B** → P2 join → live only if justified.
**Do not** default to “mint + Arm A same-shot again.”

**If material remains human-only wall:** stop agent live; document proven wall once; operator place peel under `testdata/xdna2_cmd/external_known_good/` with PROVENANCE (or approve instrumented build).

---

## 6. Critical file paths

| Path | Role |
|------|------|
| `/home/hunter/Projects/ai/dragon-npu/RESIDUAL.md` | Unlock SSoT; residual class; primary wall |
| `/home/hunter/Projects/ai/dragon-npu/.agents/plans/plan-live-shot-effective.md` | Option-1 prep plan (S0–S5 **done**) |
| `/home/hunter/Projects/ai/dragon-npu/.agents/plans/plan-silicon-try-everything.md` | Bridge A/B; T1 material + wait; freeze list |
| `/home/hunter/Projects/ai/dragon-npu/.agents/reports/live-shot-2c9c12e8.md` | Arm A same-shot live evidence |
| `/home/hunter/Projects/ai/dragon-npu/.agents/reports/impl-live-shot-effective-s4-s5.md` | Mint + CONSUME closeout |
| `/home/hunter/Projects/ai/dragon-npu/.agents/reports/plan-live-shot-waste-modes.md` | Waste modes + pre-live gate |
| `/home/hunter/Projects/ai/dragon-npu/.agents/reports/wave9-a-bridge-a.md` | Sibling/oracle wall; INSTR_BO_LIMIT |
| `/home/hunter/Projects/ai/dragon-npu/.agents/reports/wave10-join.md` | Same-shot offline wire closed |
| `/home/hunter/Projects/ai/dragon-npu/docs/CONSTRUCTION_MAP_APP_LOAD.md` | CU/PDI vs ctrltext; highest-value next |
| `/home/hunter/Projects/ai/dragon-npu/src/backends/amd_xdna2/silicon_contract.rs` | same_shot plan, wait axes, material match |
| `/home/hunter/Projects/ai/dragon-npu/src/backends/amd_xdna2/direct.rs` | Same-shot lab hook; WAIT_CMD; hwctx |
| `/home/hunter/Projects/ai/dragon-npu/tools/operator_app_load_kit.sh` | Live sandwich entry |
| `/home/hunter/Projects/ai/dragon-npu/scripts/lab_exec_once.sh` | Env forward (SAME_SHOT_RELA, …) |
| `/home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/silicon_lab/` | Staged ELF / WTS / ctrl / golden |
| `/home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/external_known_good/` | Capture / load-accepted landing (research vs claim) |
| `/tmp/dragon_npu_last_apply_live_eff_2c9c12e8.json` | Shot-specific last_apply (evidence) |

---

## 7. One-line handoff for next plan author

**Do not re-prove same-shot.** Change instruction identity (Bridge A / load-accepted)
and/or wait completion path offline first; mint a live unlock only with a **new**
named reason that cites that delta; Pass only via `decide_ran_on_npu`.

---

**Report path:** `/home/hunter/Projects/ai/dragon-npu/.agents/reports/plan-option1-effective-explore.md`
