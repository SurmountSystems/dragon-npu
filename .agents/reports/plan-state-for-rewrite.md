# Plan state for rewrite (post W4)

**Date:** 2026-08-11
**Kind:** Read-only inventory for next plan rewrite (not an unlock, not Pass).
**Sources:** session `plan.md` (019f6476…), workspace mirror
`.agents/plans/plan-silicon-try-everything.md`, `RESIDUAL.md` top,
`.agents/reports/wave4-join.md` + W0–W3 joins, `docs/CAPTURE_MATERIAL_MLIR_AIE.md`,
capture dir + PROVENANCE, tools inventory.

**Honesty footer (always):** invent_pass=0 · Pass=**No** · active unlock=**none** ·
first `C_UNLOCK=` = **`4008e6fd` CONSUMED** · unlock ≠ Pass ≠ `ran_on_npu`

---

## 1. Accomplished (W0–W4) — do not re-do as progress

### Campaign waves

| Wave | Landed (not Pass) | Primary reports |
|------|-------------------|-----------------|
| **W0** | Material inventory; WAIT uAPI axes; XRT/DD load map; structural diff; debug-BO prep; unspent flexml scan empty | `wave0-join.md`, `wave0-*.md` |
| **W1** | WAIT driver soft map; `syncobj_handle` retain on CREATE; aiebu wrap/peel closed offline; debug-BO trait + lab path (Shmem first) | `wave1-join.md`, `campaign-try-everything-w1-status.md` |
| **W2** | L5 Shmem DBG Assign **fail** → offline Dev type fix → L5b Dev Assign **ok** → still `wait_cmd_einval` | `live-shot-48da6e60.md`, `live-shot-4008e6fd.md`, `wave2-debug-bo-type-fix.md` |
| **W3** | Material hunt; pure syncobj decision helpers (`should_wait_via_syncobj` / select / pack); L5b soft classify; unspent non-flexml PDI list; join pin | `wave3-join.md`, `wave3-syncobj-wait-offline.md` |
| **S7** | Cascade public mlir-aie npu2 AOT **landed** (ctrltext **4512 B**, sha16 `18cb653c05a9d4c8`); ClassicTxnV01; L7 live **not** justified | `wave3-s7-cascade-closeout.md` |
| **W4a** | Host inventory (T1+T8); material/last_apply scorecards (T3+T6, residual_matched_claim always 0); journal scrape (T4); syncobj pack pure (T5) | `wave4-host-inventory.md`, `wave4-scorecard-tools.md`, `wave4-journal-tool.md`, `wave4-syncobj-pack.md` |
| **W4b** | Bag scored (all claim=0); multi-txn 32208 **refused**; no safe host capture without operator | `wave4-discovery.md` |
| **W4c** | Env-gated syncobj wait wire offline (`XDNA2_LAB_WAIT_VIA_SYNCOBJ`); default still WAIT_CMD | `wave4-syncobj-wire.md` |
| **W4 join** | Offline complete; L8 **not** run / **not** justified from wire alone; operator material wall restated | `wave4-join.md`, `impl-s0-w4-pin-mirror.md` |

### Live ledger (CONSUMED — do not re-live alone)

| Unlock | Experiment | Outcome |
|--------|------------|---------|
| `4008e6fd` | L5b Dev DBG + skip-CU | Assign ok then **wait_cmd_einval** |
| `48da6e60` | L5 Shmem DBG + skip-CU | **debug_bo_failed** (Assign EINVAL) |
| `1f761bf4` | Skip-CONFIG_CU isolation | **wait_cmd_einval** ~0.17s; no IO_PAGE_FAULT |
| `efd44f35` | flexml unified-4x4 CONFIG_CU | wait_cmd_einval + IO_PAGE_FAULT |
| `8352882b` | flexml llm-2x4x4 CONFIG_CU | wait_cmd_einval + IO_PAGE_FAULT |
| `2ae69036` | flexml unified-2x4x4 CONFIG_CU | wait_cmd_einval |
| `97f38307` | CVML / research CONFIG_CU | wait_cmd_einval / prior class |
| `11bc9016` / `3a70fabf` / `b17b0119` | Historical stock / sandwich | Historical CONSUMED |

### Material landed (research only — not residual match)

Under `testdata/xdna2_cmd/external_known_good/`:

| Set | Size class | Ctrltext | sha16 (ctrltext) | Role |
|-----|------------|----------|------------------|------|
| vector scalar add npu2 | `public-npu2-small` | 300 B | `aaeb5b68cc298a45` | RESEARCH ONLY; primary `working_stack_ctrltext.bin` |
| single_core matmul npu2 | `public-npu2-small` | 660 B | `de8671245bb72436` | RESEARCH ONLY |
| whole_array matmul npu2 | `other` | 2592 B | (see PROVENANCE) | RESEARCH ONLY |
| cascade matmul npu2 | `other` | **4512 B** | `18cb653c05a9d4c8` | RESEARCH ONLY; ClassicTxnV01 116 ops |
| siblings | — | insts / elf / pdi / xclbin / core.elf per set | — | RESEARCH ONLY |

W4b bag score: **every** bag row `residual_matched_claim=0`. Multi-txn
`gt_capture/instr_vendor_post_patch.bin` (32208) **refused**.

### Product / tooling already green offline (subset)

Double-gate EXEC; CREATE_HWCTX; DevHeap 64 MiB UVA; aiebu+rela; optional CONFIG_CU;
OFM decide; C1–C6 / C4 / P5 / P6 readiness; `pass_unblock_once` + fingerprint Spent
ledger; WAIT axes / `wait_cmd_einval` honesty; material residual-match kinds;
DBG Dev recipe; BO map / IO_PAGE_FAULT audit pure helpers; research PDI identity;
syncobj pure + pack + env-gated wire; host inventory + journal scrape + capture
candidate fail-closed.

### Falsified (stop treating as residual root)

| Claim | Evidence | Freeze |
|-------|----------|--------|
| Empty capture folder is the wall | Four public AOT sets present | Empty-folder reconfirm forever |
| Overlay / CONFIG_CU required for residual class | Skip-CU same wait_cmd_einval | Bare attach re-live theater |
| DBG_BUF Assign is residual root | Dev Assign ok, still EINVAL | Re-live DBG alone |
| Flexml STX re-attach alone is progress | All three Spent | Spent fingerprint alone |
| Cascade / public AOT is residual match | Size classes ≠ 1688 / ≠ flexml; claim=0 | L7 without new hypothesis |
| Syncobj pack+wire alone justifies L8 | No named reason beyond wire land | Mint from join alone |

---

## 2. Honesty state (SSoT)

| Fact | Value |
|------|--------|
| Product Pass | **No** |
| invent_pass | **0** |
| ran_on_npu | **false** (never claimed) |
| Active unlock | **None** |
| First `C_UNLOCK=` (parse SSoT) | **`4008e6fd` CONSUMED** |
| Residual class | Post-attach / EXEC **`wait_cmd_einval`** (lab classic 1688 aiebu+rela×12; same family skip-CU + L5b) |
| Overlay | Not sole cause (`1f761bf4`) |
| Flexml STX | All three **Spent** |
| Capture dir | Full of **RESEARCH ONLY** peels; **not** load-accepted |
| Syncobj | Offline pack+wire green; default wait still WAIT_CMD; **L8 not run** |
| Who acts next (primary) | **Operator:** load-accepted material |
| Who acts next (agent) | W4 offline **done**; idle or named offline delta only; **no** L8 mint from join |

Parse rule: first `C_UNLOCK=` line in `RESIDUAL.md` only; line must lack CONSUMED
to be active. Current first line is CONSUMED → no active one-shot.

---

## 3. Remaining work categories

### A. Agent offline (legal only if **named** delta; do not empty-reconfirm)

| Status | Item |
|--------|------|
| **Done (W4)** | T1 inventory, T3/T6 scorecards, T4 journal, T5 pack, P-sync-2 wire offline, W4b discovery |
| **Optional later** | Sharper FW-seen helpers if new last_apply tokens; dyndbg **doc-only** recipe; assumption A1–A12 scorecard soft print polish if still missing cells |
| **Not progress alone** | Re-run inventory / scorecards / gates with no new bytes; re-fingerprint spent PDI; more public AOT peels of same classes |

### B. Material wall (operator primary)

| Need | Detail |
|------|--------|
| **What** | **Load-accepted** NPU program capture: bytes from a stack that **already completed** on this machine’s firmware (ERT Completed / useful OFM), honest PROVENANCE |
| **Where** | `testdata/xdna2_cmd/external_known_good/` or `bash tools/pass_unblock_once.sh --ingest PATH [NAME]` |
| **Does not count** | Public mlir-aie 300 / 660 / 2592 / cascade 4512; multi-txn 32208; lab/gt_capture oracles; invent bins |
| **Playbook** | `docs/CAPTURE_MATERIAL_MLIR_AIE.md` (AOT research only); PROVENANCE wants **completed-on-this-FW** for load-accepted claim |
| **Optional** | FastFlowLM / Lemonade / completing EP sample **only if** operator wants that intercept (inventory found roots; no safe capture without EXEC/gates) |
| **After drop** | Agent: soft compare, scorecard claim check, unlock mint criteria, justified live, Pass bookkeeping via `decide_ran_on_npu` only |

### C. Live unlock (serial; strict mint)

| Id | Experiment | Justified when |
|----|------------|----------------|
| **L1** | Residual-matched / load-accepted material verification | Honest load-accepted material lands |
| **L8** | Skip-CU + `XDNA2_LAB_WAIT_VIA_SYNCOBJ=1` | Named offline reason **beyond** “wire landed”; new host-minted unlock. **Not** justified from W4 join alone. Offline pack+wire already green. |
| **L4u** | Unspent research PDI + new wait-path or instr delta | Bare attach deprioritized |
| **L5/L5b** | DBG | **Closed** |
| **L7** | Public AOT as instr | **Closed no** without new non-wrong-body hypothesis |
| **L-skip** | Re-skip-CU | **Frozen** under CONSUMED `1f761bf4` |

Mint rules: named offline delta; new hex; one shot; no sticky ALLOW; invent_pass=0;
unlock ≠ Pass ≠ `ran_on_npu`. Never reopen CONSUMED list.

### D. Freeze list (rewrite must keep)

1. Re-live CONSUMED / spent flexml alone
2. Empty reconfirm / empty-folder implement / stamp pollution
3. Pass from attach, public AOT, cascade land, DBG ok, syncobj pure/wire alone, unspent list alone
4. Sticky ALLOW
5. Invent capture / residual_matched_claim
6. L8 mint from wire join alone
7. New third-party network install without operator OK

---

## 4. Plan sources snapshot

| Artifact | Role | Rewrite note |
|----------|------|--------------|
| Session `plan.md` (019f6476…) | Approved residual-resolve / W4 agent-max plan | Still describes pre-W4 “agent does next” steps that **W4 completed**; Who acts / material wall still correct |
| `.agents/plans/plan-silicon-try-everything.md` | Workspace mirror of same | Same; W4a/b/c steps should move to **Accomplished** |
| `RESIDUAL.md` top | Live SSoT unlock + honesty | First C_UNLOCK CONSUMED; W4 join Latest Turn |
| `wave4-join.md` | Offline complete pin | L8 not justified; operator material primary |
| `docs/CAPTURE_MATERIAL_MLIR_AIE.md` | Public AOT → capture dir playbook | RESEARCH ONLY; not load-accepted |

**Plan rewrite intent (for implementer of next plan):**
Lead with **operator material wall** + honesty. Demote W4 tool build from “next”
to accomplished. Keep assumption catalog A1–A12 with current validator results
where known (A9/A10 falsified; A3 pack/wire offline green, live unknown; A6 host
inventory ran, no safe autonomous capture). Live queue: L1 after material; L8 only
with new named reason + mint. Stop empty agent loops.

---

## 5. Capture path file inventory

### Directory: `testdata/xdna2_cmd/external_known_good/`

| Basename pattern | Present | Notes |
|------------------|---------|-------|
| `PROVENANCE.md` | yes | Tracked; research-only records for mlir-aie AOT |
| `working_stack_ctrltext.bin` | yes | 300 B vector peel alias; **not** load-accepted |
| `mlir_aie_vector_scalar_add_npu2.*` | yes | ctrltext, insts, insts.elf, pdi, xclbin, core.elf |
| `mlir_aie_single_core_matmul_npu2.*` | yes | same siblings; ctrltext 660 B |
| `mlir_aie_whole_array_matmul_npu2.*` | yes | ctrltext 2592 B |
| `mlir_aie_cascade_matmul_npu2.*` | yes | ctrltext 4512 B |
| Load-accepted / residual-matched peel | **no** | Wall open |

Bins gitignored; PROVENANCE tracked. Playbook: `docs/CAPTURE_MATERIAL_MLIR_AIE.md`.

### Tools (capture / host / pass_unblock / journal)

| Tool | Role |
|------|------|
| `tools/pass_unblock_once.sh` | Primary status / agent-offline / ingest / live refuse without unlock |
| `tools/pass_unblock_fingerprint_once.sh` | PDI fingerprint + spent ledger |
| `tools/operator_external_capture.sh` | Install/compare capture kit |
| `tools/operator_app_load_kit.sh` | Umbrella (pass-unblock, a-prep, config-cu, …) |
| `tools/capture_from_host_candidate.sh` | W4b T7 fail-closed host candidate capture |
| `tools/capture_mladf_instr_vendor.{sh,cpp}` | Vendor instr capture (HITL; multi-txn refuse as load-accepted) |
| `tools/capture_mladf_ofm_golden.{sh,cpp}` | OFM golden capture |
| `tools/host_npu_stack_inventory.sh` | W4a T1 read-only inventory |
| `tools/journal_npu_scrape.sh` | W4a T4 journal scrape |
| `tools/hitl_b_gt_capture.sh` / `hitl_c_pass_apply.sh` / `hitl_c2_once.sh` / `hitl_config_cu_once.sh` | HITL B/C / config-cu |
| `tools/collect_post_c2_evidence.sh` | Post-C2 journal + evidence |
| `tools/p5_post_c2_interpret.sh` / `tools/p6_true_pass_check.sh` | Post-shot interpret / Pass criteria |

Related soft/docs: `docs/HITL_C_APP_LOAD_RECIPE.md`, `docs/CONSTRUCTION_MAP_APP_LOAD.md`.

---

## 6. Critical code/docs for next plan

| Path | Role |
|------|------|
| `RESIDUAL.md` / `AGENTS.md` | Unlock SSoT + standing honesty |
| `src/backends/amd_xdna2/direct.rs` | wait path, DBG, syncobj wire |
| `src/backends/amd_xdna2/silicon_contract.rs` | axes, material scorecards, residual match |
| `src/backends/amd_xdna2/txn.rs` | blob classify |
| `testdata/xdna2_cmd/external_known_good/` | material dir |
| `tools/pass_unblock_once.sh` | status / ingest |
| `.agents/reports/wave4-join.md` | W4 offline complete |

---

## 7. Success criteria for a rewritten plan (suggested)

1. Operator material ask is either **satisfied** or **blocked with proof** (exact path/file), not a vague chat quiz.
2. W4 tools stay in **Accomplished**, not “next agent work.”
3. L8 only after named offline reason + new unlock; never from wire-alone.
4. Pass only via live `decide_ran_on_npu` (ERT Completed + OFM allclose + real_ioctl).
5. invent_pass=0; no reopen CONSUMED; no empty reconfirm.

---

## Explicit non-claims

- No product Pass · invent_pass=0 · no `ran_on_npu`
- No active unlock · no L8 run · no load-accepted material invented
- This report does **not** mint unlock or re-open live
