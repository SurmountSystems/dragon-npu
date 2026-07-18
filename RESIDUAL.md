# Residual — silicon path (honest status)

**Date:** 2026-07-18  
**IMPL stamp:** operator **C2 live complete** (post-`3a14f38d` readiness) — **no** Pass / **no** `ran_on_npu`  
**C_UNLOCK=b17b0119** one-shot **CONSUMED** by this operator C2 (further C needs **new agent unlock**)  
**Honesty:** Live EXEC ran; L3 **restored** (`exec_cmd_allow=N`). **Not** silicon Pass.

### Operator C2 result (2026-07-18 ~03:47, this host)

| Field | Live result |
|-------|-------------|
| Process | `hitl_c2_once` + L3 open → `hitl_c --assume-l3-already-open` → L3 restore |
| Lab exit | **4** (`!ran_on_npu` / `blocked_npu_compute` — expected without Pass) |
| POST L3 | **`N` / closed** (`POST_OK`) |
| `last_apply.json` | Updated **2026-07-18 03:47** — still `status=ert_cmd_error` `ran_on_npu=false` |
| Userspace | **OK** — `real_ioctl=true` `real_device_open=true` aiebu ELF + rela×12 + xrt3 + content 1688 / bo 4096 + tiles=32 + 15 kernel args + valid IO paddrs |
| ERT | `ert_state=5(Error)` / `SubmitWaitNotOk` (WAIT returned; OFM verify refused) |
| OFM | **all zero** (`raw_nonzero_bytes=0`) |
| Journal (this shot) | **`Failed cmd idx 0, status 0x3000003`** (`APP_LOAD_PDI_FAIL`); also **`Status 0x4000003`** (`AIE2_STATUS_INVALID_PARAM`) |
| Residual class | **`app_load_pdi_fail_after_aiebu`** (locked) |
| C3 body FP | **Same** as locked baseline fingerprint → `C3_SKIP` (class unchanged; journal codes are the new hard evidence) |
| C4 | **`C4_SKIP`** `json_ran_on_npu_false` |

**Conclusion:** One-shot pure-direct C2 **executed cleanly** (userspace green path, gates restored). Residual remains **FW APP_LOAD after full userspace** — **not** offline prep, not L3 hygiene, not map_offset. Env trials already **CLOSED**. Do **not** re-apply with flag spam. Further C needs **new agent unlock** + new offline hypothesis.

**L3** = `exec_cmd_allow` (driver); **userspace ALLOW** = `XDNA2_ALLOW_EXEC_CMD`. Leave **both** closed.  
**Do not invent vendor bytes. Do not open L3 from agent. Do not set `XDNA2_ALLOW_EXEC_CMD=1` in default work.**

## Priority branch (SSoT) — C2 **done** (APP_LOAD); offline green; **no** Pass

**Single source of truth for current state.** Older “C2 awaiting operator / last_apply 2026-07-16 only” stamps below are **historical** where superseded.

**Offline APP_LOAD signal matrix (`aae86bb9` locks):** still valid; live C2 **reconfirmed** residual class + journal **`0x3000003` / `0x4000003`**.

| File / gate | State **now** |
|-------------|----------------|
| Classic vendor body | **EXISTS** — 1688 B CLASS A unpatched |
| Offline gates | **Green** (`OFFLINE_GATES_OK` during C2 prep) |
| Pure-direct patch | Unit-locked; **≠** true post-patch |
| last_apply | **2026-07-18 03:47** live C2 — `ert_cmd_error` `ran_on_npu=false` aiebu=1 rela=12 layout=xrt3 |
| C3 pipeline | **`C3_SKIP`** same residual body FP as locked baseline (journal APP_LOAD codes new hard confirm) |
| C4 pipeline | **`C4_SKIP`** no Pass |
| HITL C | One-shot **used**; recipe still valid only after **new unlock** |
| C2 live | **Done** — lab rc=4; L3 restored; residual APP_LOAD |
| Pass? | **No** — need ERT Completed + non-zero OFM allclose + `ran_on_npu=true` |

### Offline APP_LOAD signal locks (`aae86bb9` — same fixture, deeper parse)

Same `testdata/xdna2_cmd/last_apply.json` residual class as before
(`app_load_pdi_fail_after_aiebu`). These are **unit-locked parse tokens**, not a new
silicon root-cause discovery beyond the already-locked FW APP_LOAD residual.
Soft `format_app_load_offline_checklist` prints the matrix; **never** unlocks C.

| Signal | Fixture value | Implication (signal lock) |
|--------|---------------|---------------------------|
| `instr_map_offset` | `0xffffffffffffffff` (`u64::MAX`) | Expected Dev no-mmap when `ty=Dev` → `map_offset_not_app_load_root=1` (not APP_LOAD cause by itself) |
| `instr_bo_type` / `ty` | `Dev` | D2 path OK (`instr_ty_dev=1`) |
| `instr_xdna_addr` | `0x4020000` | Valid non-zero / non-MAX device addr |
| `content_size` / `start_npu_size` | 1688 / 1688 | Content-mode START_NPU honest (`start_npu_matches_content=1`) |
| `bo_size` | 4096 | `bo_gt_content=1` (size only; not page-align claim); live content trial **CLOSED** still APP_LOAD |
| `SubmitWaitNotOk` | present (substring) | WAIT accepted; OFM verify refused after ERT Error — **not** ioctl hang |
| OFM | `raw_nonzero_bytes=0` | Expected when FW rejects (`ofm_all_zero=1`) |
| `fw_status_code` in detail | **absent** | `fw_code_absent_in_detail=1`; checklist uses `class_implies_app_load_after_aiebu=1` — does **not** invent `fw_name=APP_LOAD_PDI_FAIL` without hex |
| Residual class | `app_load_pdi_fail_after_aiebu` | Userspace ELF+rela×12 **OK**; FW residual class locked (journal code from prior HITL, not re-invented) |

**Conclusion (offline):** residual class unchanged — **FW APP_LOAD after green userspace
contract**. Signal locks rule out map_offset / size triad / raw-bin as *alternate*
root-cause narratives for this fixture. Env HITL still **CLOSED**.

### INSTR_BO_LIMIT / true post-patch research (IMPL 0eea644f — **closed offline**, architecture residual)

Read-only against DD 1.7.1 `libdyn_dispatch_core.so` + headers. **Do not reopen** as P0 research:

| Finding | Detail |
|---------|--------|
| **Where** | `instruction_cache::put` — throw `… is greater than the INSTR_BO_LIMIT` |
| **Check** | `align_to_next(getInstrBufSize(txn)=len+8, **0x8000**) > capacity_` |
| **Defaults** | align quantum **32 KiB**; capacity ctor **0x3c00000 (60 MiB)** |
| **get_instr_bo** | cache get → on miss Transaction load — **not** live execute instr BO |
| **mladfmatmulbias** | public IFM/WTS/OFM/const/ctrl only; **`run_` private**; **no** live instr BO API |
| **get_transaction_bin ~32208** | classic-format **wrong body** (500 ops); **not ELF**; **`find(classic1688)==-1`** |
| **True post-patch public path** | **Not available** offline — **XRT/DD architecture** residual (closed) |
| **Pure-direct** | Unpatched classic body parity **green** (CLASS A); do **not** invent vendor post-patch / force CLASS B |

### What landed (tools + pure-direct 4ef82a68)

1. **Capture:** classic 1688 Transaction-map → `instr_vendor_txn_bin.bin` + unpatched `post_patch`; multi-txn → `instr_vendor_get_transaction_bin.bin` only; registry refuses oversized multi-class BO as post_patch; **0eea644f** logs INSTR_BO research + multi hdr/embed negatives.
2. **Diff:** unpatched mode → pure `instr_raw_txn` (no vendor-paddr re-patch); sibling recover **only** if unpatched mode (or multi-txn wrong body **and** mode ≠ `registry_get_instr_bo`); **never** replace registry post_patch with unpatched sibling; **C** only true same-class non-rela.
3. Offline case **D** (oversized + sibling → A/D not C).
4. **Pure:** `txn.rs` staged classic (1688/40 ops) vs multi wrong-body (32208/500 ops) profile helpers + embed-slice negative test.
5. **IMPL 4ef82a68 (landed):** pure-direct patch TDD (multi-sym synthetic overwrite/add, negative addend wrap, staged ELF golden walk + CLASS A body fixtures); `format_app_load_residual_diag` **unit/API research helper only** (not live-wired into timed detail); START_NPU content/bo honesty (content live trial closed; content clamps to `bo_alloc` when both set).

### Highest-value next (single table — SSoT)

| Priority | Action | Who |
|----------|--------|-----|
| **1** | **APP_LOAD root-cause offline** with **new** hypothesis only (journal **`0x3000003` APP_LOAD_PDI_FAIL** + **`0x4000003` INVALID_PARAM** reconfirmed live). Do **not** env-flag spam. Prefer research that can change pure-direct patch / START_NPU / ctrl rela / BO type without inventing vendor post-patch | agent |
| **2** | Maintain `bash tools/run_offline_gates.sh` green; keep L3=`N` | agent / ops |
| **3** | **Another live C only after new agent unlock** + offline delta (one-shot `b17b0119` **consumed**) | agent then operator |
| **4** | Optional capture-instr logs only (true post-patch public path **closed**) | operator optional |
| **ban** | Env C spam; invent vendor; L3 from agent; claim Pass; second C without new unlock; re-open INSTR_BO as P0 | all |

**Operator C2 exact commands (one-shot; agent does not run these):**

```bash
cd ~/Projects/ai/dragon-npu
export PATH="/usr/bin:${HOME}/.local/bin:${PATH}"
# Prep (no L3 / no EXEC) — prints bash+fish sandwich + post-C2 C3/C4 steps:
bash tools/hitl_c2_once.sh
# optional: export XDNA2_LAB_BO_DIAG=1
sudo sh -c 'echo 1 > /sys/module/amdxdna/parameters/exec_cmd_allow'
bash tools/hitl_c_pass_apply.sh --assume-l3-already-open
# or after L3 open: bash tools/hitl_c2_once.sh --apply-if-l3-open
sudo sh -c 'echo 0 > /sys/module/amdxdna/parameters/exec_cmd_allow'
cat /sys/module/amdxdna/parameters/exec_cmd_allow   # must be 0 or N
# Post-C2 offline (C3 evidence + C4 PassCandidate audit; ≠ invent product Pass):
bash tools/c3_post_c2_classify.sh
bash tools/c4_pass_check.sh
bash tools/run_offline_gates.sh
# Share last_apply / detail / journal with agent (C3 residual or offline PassCandidate).
```

**C policy (honesty):** Explicit agent unlock (`C_UNLOCK=b17b0119`) is **sufficient** for **one** C attempt (recipe). **Not** permanent open season — **further C requires a new agent unlock**. New last_apply/journal evidence may **justify** that unlock decision; evidence **never auto-unlocks** C alone. Standing rule 2 bans **env-flag spam re-apply** without new offline evidence — it does **not** block this explicit one-shot unlock. Unpatched CLASS A, registry BO dump, parity green, `CONTRACT_OK`, soft residual_class / CLASSIFY_SOFT_OK / checklist, and `OFFLINE_GATES_OK` **never** unlock C alone and **never** equal Pass. True post-patch via public DD **closed** — do **not** prefer `registry_get_instr_bo` as a C pre-req (cache path only, not proven live-patched `run_` BO). Optional recapture for logs only:

```bash
# optional only — logs / diagnostics; not a C unlock; unpatched body class already green offline
sudo sh -c 'echo 1 > /sys/module/amdxdna/parameters/exec_cmd_allow'
bash tools/silicon_lab_operator.sh capture-instr
sudo sh -c 'echo 0 > /sys/module/amdxdna/parameters/exec_cmd_allow'
bash tools/diff_vendor_instr_parity.sh   # unpatched A = body match only; C2 unlocked once (b17b0119) ≠ Pass
```

### Self-runnable gates (agent offline now)

**One-shot:** `bash tools/run_offline_gates.sh` → `OFFLINE_GATES_OK` (hard unit/style + soft classify/assets; **≠ C-ready**). CI job `offline-gates` runs the same without `--require-assets`.

| Script / test | Expect **now** (`f76067f8`) |
|---------------|----------------------------------------|
| `bash tools/run_offline_gates.sh` | **0** `OFFLINE_GATES_OK` (≠ Pass ≠ permanent unlock ≠ `ran_on_npu`) |
| `cargo fmt -- --check` | **0** |
| `cargo clippy --features amd-xdna2 --lib --example classify_last_apply -- -D warnings` | **0** |
| `cargo test --features amd-xdna2 --lib pure_direct` | **3 ok** |
| `cargo test --features amd-xdna2 --lib silicon_contract` | **45+ ok** (APP_LOAD + fingerprint/baseline C3 + checklist + fixture) |
| `cargo test --features amd-xdna2 --lib classic_1688` | **ok** |
| `bash tools/diff_vendor_instr_parity.sh` | **rc=0** CLASS_HINT **A** (≠ C-ready) |
| `bash tools/test_diff_vendor_parity_offline.sh` | **pass=4** fail=0 |
| `bash tools/silicon_contract_check.sh` | **CONTRACT_OK** (≠ C-ready; soft classify if fixture) |
| `bash tools/classify_last_apply_residual.sh` | **0** soft; fixture → `app_load_pdi_fail_after_aiebu` + checklist + `not_c_ready=1` |
| `bash tools/c3_post_c2_classify.sh` | **0** `C3_SKIP no_new_c2_evidence` on baseline fixture (soft; ≠ C-ready) |
| `bash tools/c3_post_c2_classify.sh --require-new` | **exit 2** on baseline (hard research mode) |
| `cargo run --features amd-xdna2 --example classify_last_apply --quiet -- testdata/xdna2_cmd/last_apply.json` | **0** report only |
| `bash tools/silicon_lab_preflight.sh --require-pass-assets` | **0** (assets only) |
| `bash tools/hitl_c2_once.sh --print-only` | **0** sandwich print (no L3; post steps → `c3_post_c2_classify`) |
| `bash tools/hitl_c2_once.sh --apply-if-l3-open` when L3=N | **exit 1** fail-closed |
| `bash tools/hitl_c_pass_apply.sh --assume-l3-already-open` when L3=N | **exit 1** fail-closed |
| `bash -n tools/c3_post_c2_classify.sh` + `hitl_c2_once.sh` + `hitl_c_pass_apply.sh` | **0** (no apply) |

**Locked residual matrix (SSoT):**

| Layer | State |
|-------|--------|
| Userspace ELF+rela×12 | **OK** (aiebu_elf=1, rela_count=12, paddrs valid, QoS/tiles OK; Dev instr + content START_NPU sizes OK offline) |
| FW APP_LOAD | **Locked** — `ert_state=5(Error)` / journal `0x3000003` APP_LOAD_PDI_FAIL; OFM zeros; `SubmitWaitNotOk`; `ran_on_npu=false` |
| map_offset MAX / Dev ty | **Not root cause** (expected Dev BO; unit-locked `aae86bb9`) |
| Env trials (`content`, `xrt3`) | **CLOSED** (same APP_LOAD) |
| True post-patch public DD | **Closed offline** (INSTR_BO / no live `run_` BO) |
| Unpatched classic body | **CLASS A** body match — **≠ C-ready** |
| HITL C recipe | **UNLOCKED once** (`C_UNLOCK=b17b0119`) — still **≠** Pass / **≠** `ran_on_npu`; C2 live **awaiting operator** (`de4d9fa4`) |
| Classifier TDD + soft CLI | Pure `AppLoadPdiFailAfterAiebu` + `format_app_load_offline_checklist`; soft classify (load confined; `CLASSIFY_SOFT_OK`) — **≠** Pass |

Env HITL closed. Recipe: [docs/HITL_C_APP_LOAD_RECIPE.md](docs/HITL_C_APP_LOAD_RECIPE.md).

---

## Historical (superseded SSoT — do not use for “what next”)

### HITL capture-instr fail then multi-txn false C (summary)

- Early fail: IFM-only `get_transaction_bin` → no post_patch (fixed: IFM+WTS + Transaction map).
- Later HITL: vendor **exists**; multi-txn 32k written as post_patch; classic 1688 sibling correct; false CLASS_HINT C fixed offline (sibling recover gated on unpatched / non-registry).
- Reconfirm stamps `dd160a9f` / `81e050de` / `085f25fb` recorded **MISSING** vendor at that time — **superseded** by current SSoT above (vendor classic body EXISTS, CLASS A unpatched).

### Prior offline sprint language (effort-5)

Historical: “vendor still MISSING / mandatory capture-instr / pass=3”. **Superseded:** vendor classic body exists; offline matrix is pass=4; INSTR_BO post-patch research **closed offline**; pure-direct TDD **landed**; highest-value = silicon APP_LOAD when agent ready + maintain offline gates.

---

## Strategy (stop apply ping-pong)

| Mode | Who | Command |
|------|-----|---------|
| **Offline** | agent | contract, GT, pure-direct / APP_LOAD residual (post-patch path closed offline) |
| **B GT** | you once | `bash tools/hitl_b_gt_capture.sh` (done if assets present) |
| **Capture** | optional | logs only — public true post-patch path **closed** (registry = cache, not live `run_` BO) |
| **C2 apply** | **you now** (`C_UNLOCK=b17b0119`) | `hitl_c2_once.sh` prep + one `hitl_c` sandwich + **restore L3** — unlock **≠** Pass; false unlocks alone still never equal Pass; agent run **sudo-blocked** |

See `docs/HITL_BC.md` · `docs/HITL_C_APP_LOAD_RECIPE.md`. **Do not** serial “try flag X on live silicon” as default progress.

---

## Locked residual (C apply 2026-07-16)

Userspace **OK** on live C:

| Field | Live value |
|-------|------------|
| `aiebu_elf` | 1 |
| `rela_count` | 12 |
| `aiebu_rela_patched` | yes |
| QoS | 100/30 |
| tiles | 32 |
| DevHeap | 64 MiB-aligned |
| BO addrs | all valid |
| `bo_size` / `inst_bo_size` | **4096** (zero-pad page) |
| `ert_state` | **5 (Error)** |
| OFM | all zeros |
| Journal | `Status 0x4000003` + `Failed cmd … 0x3000003` = **APP_LOAD_PDI_FAIL** |

**Not raw-bin class** (that residual was closed offline: ELF+rela required).  
**bo_size=4096 alone did not clear APP_LOAD_PDI_FAIL.**

### Historical offline knobs (env HITL **CLOSED** — not next trial)

Landed for research/defaults only. **Do not** re-open live env C applies; true post-patch research **closed offline** (SSoT INSTR_BO table). Offline pure-direct TDD **landed** (4ef82a68); next silicon = APP_LOAD only if agent ready; maintain gates; optional recapture logs only.

| Env | Default | Historical note |
|-----|---------|-----------------|
| `XDNA2_LAB_INSTR_SIZE_MODE` | **`content`** | START_NPU size = ctrltext/txn_size (**1688**); BO still zero-pad 4096. Alt: `bo`. Live trial with content **CLOSED** (still APP_LOAD_PDI_FAIL). |
| `XDNA2_LAB_KERNEL_ARG_LAYOUT` | `count4` | **`xrt3` trial CLOSED** (live same `0x3000003`). Offline both layouts still contract-green. |
| `XDNA2_LAB_RELA_MODE` | `overwrite` | Residual offline: **`add`** = RMW placeholder+base+addend (holes `0xb1`/`0x1e`/`0x159`). **Ban** live env apply; C only if agent ready (unpatched A ≠ unlock; no “prefer true post-patch” pre-req — path closed). |

Detail prints: `inst_size_mode=` `content_size=` `bo_size=` `start_npu_size=` `rela_mode=` `layout=`.  
Content mode pure resolve: `min(content_len, bo_alloc)` when both non-zero (never advertise DMA longer than BO).

### HITL trial log — **CLOSED** (no more env apply spam)

| Trial | Env | Live | Result |
|-------|-----|------|--------|
| **1** (~21:03) | `content` size 1688 + `count4` | EXEC | **`0x3000003` APP_LOAD_PDI_FAIL** — size mode ≠ fix |
| **2** (~21:14) | `content` + **`layout=xrt3`** (RESIDUAL_ENV confirmed) | EXEC | **Same `0x3000003` / `ert_Error` / OFM zeros** — kernel-arg layout ≠ fix |

**Both planned residual env trials failed identically.**  
Do **not** re-apply with `RELA_MODE=add`, tile flips, or more layout flags as next work.

*(Highest-value table is only the single SSoT table at top — no second priority list here.)*

Historical (when vendor was MISSING): capture HITL was P0. INSTR_BO true post-patch research **closed offline** (top SSoT). Residual = **XRT/DD architecture** (no public live instr BO) + **APP_LOAD**.

Optional offline (no sudo) — **extract-paddrs does NOT auto re-patch**:

```bash
bash tools/silicon_lab_operator.sh extract-paddrs
# bash: export XDNA2_GT_PADDRS_JSON=testdata/xdna2_cmd/gt_capture/vendor_paddrs_from_last_apply.json
# fish: set -gx XDNA2_GT_PADDRS_JSON testdata/xdna2_cmd/gt_capture/vendor_paddrs_from_last_apply.json
bash tools/dump_mladf_gt_offline.sh
bash tools/diff_vendor_instr_parity.sh   # unpatched A = body match only; ≠ Pass
```

**C one-shot unlock (`C_UNLOCK=b17b0119`)** — operator **C2 now** (top SSoT). Unlock **≠** Pass.
Still **not** unlocked by mere `CONTRACT_OK`, unpatched CLASS A / mere `diff rc=0`, registry BO, parity green, residual_class / `CLASSIFY_SOFT_OK` / checklist, or `OFFLINE_GATES_OK` / `run_offline_gates` green. Further C after this one-shot needs a **new agent unlock** (evidence may justify; never auto-unlocks).

Product G2/F2 multi-key stays parked until first Pass.

---

## Done (offline / pure-direct)

### Assets & lab plumbing (A/B/C)

| Item | Status |
|------|--------|
| AMD-packed WTS export 174080 non-zero | done (`export_mladf_wts`) |
| CTRL 29664 + count-4 pin + auto-patch residual | done |
| OFM golden capture / validate / stage | done |
| Preflight + `lab_exec_once --silicon-mladf` recipe | done |
| Zero golden / zero WTS fail-closed | done |
| Pass flags pure (`decide_ran_on_npu`, `silicon_ran_on_npu_flags_ok`) | done |
| aiebu ELF + rela×12 patch path | done (live still APP_LOAD_PDI_FAIL) |
| Instr BO zero-pad page + size dual-mode (`content`/`bo`) | done offline (content live trial CLOSED; content clamps to `bo_alloc`) |
| Rela `overwrite`/`add` modes | done offline + multi-sym pure-direct TDD (**4ef82a68 landed**) |
| Pure-direct staged ELF golden + CLASS A body fixtures | **landed** offline (≠ vendor true post-patch) |
| `format_app_load_residual_diag` | **landed** unit/API research helper only — **not** live-wired into timed detail |
| last_apply residual classifier (`5216cef3`) | **landed** pure: `parse_last_apply_detail_signals` / `classify_last_apply_residual` / `SiliconResidualClass` + note — unit only; **not** Pass / C unlock |
| APP_LOAD offline signal locks (`aae86bb9`) | **landed** pure: map_offset / Dev ty / sizes / `SubmitWaitNotOk` parse + `format_app_load_offline_checklist`; path canonicalize mid-symlink refuse; soft classify print; **≠** C unlock |
| Dynsym report + post-patch txn walk contract | done offline |
| `hitl_c --skip-offline-contract` / `XDNA2_HITL_SKIP_CONTRACT=1` | done |

### Phase D — EXEC unblock (userspace)

| Item | Status |
|------|--------|
| **D1** refuse `xdna_addr` 0 / `u64::MAX` | done |
| **D2** instruction BO = `BufferType::Dev` + heap fill | **live OK** — `ty=Dev xdna_addr=0x4020000` |
| **D2b** SHMEM PASID pin | **live OK** |
| **D3c DevHeap UVA 64 MiB align** | **live map_host clear** |
| **D3d ctrl size=6 read_add_write** | landed |
| **D3e ERT state after WAIT** | landed — live `ert_state=Error` |
| **D4** honest `exec_eio` | done |

### Phase E readiness (offline)

| Item | Status |
|------|--------|
| OFM decide wire when LAB_OFM=1 | done |
| Dump-on-mismatch `ofm_stats` | done |
| Product Pass after allclose | **blocked on FW accept + HITL** |

---

## Critical path honesty (updated — not “capture MISSING / rc=3”)

**Unpatched vendor classic body already EXISTS** (SSoT at top). Agent cannot invent
vendor bytes. Public true post-patch dump research **closed offline** (no live `run_`
instr BO API; registry = Transaction/cache only).

| Item | Notes **now** |
|------|----------------|
| Vendor classic body / unpatched CLASS A | **Satisfied** offline (sibling recover or classic post_patch). Body match only. |
| Registry / “true post-patch” dump | **Not available** via public DD after execute (research closed). Mode name `registry_get_instr_bo` = cache image only — **not** proven live-patched `run_` BO. |
| Silicon residual | **APP_LOAD_PDI_FAIL** — operator **C2** under one-shot `C_UNLOCK=b17b0119` (≠ Pass); pure-direct TDD **landed**; not INSTR_BO P0 |
| C unlock | **One-shot unlocked** (`C_UNLOCK=b17b0119`) for **one** operator C2. Unlock **≠** Pass / **≠** open season. Further C needs **new agent unlock** (evidence may justify; never auto-unlocks). Mere unpatched `diff rc=0` / CLASS A / registry / parity / `CONTRACT_OK` / residual_class / `CLASSIFY_SOFT_OK` / checklist / `OFFLINE_GATES_OK` / `run_offline_gates` green **≠ Pass** (same as top SSoT). Optional recapture = logs only. |

---

## Residual product work (post first Pass / non-critical path)

| Item | Notes |
|------|--------|
| **G2 BO reuse** | Multi-shot keep hwctx + WTS/ctrl/instr; document until Pass |
| F2 `npu_exec` product wire | After fixture Pass green |
| Multi-key / larger M | After first Pass |

---

## Autonomous validation (no sudo / no ALLOW)

Run with **bash** (fish: use `bash -c '…'` or enter a bash subshell). PATH:

```bash
# bash:
export PATH=/usr/bin:$PATH
# fish:
# set -gx PATH /usr/bin $PATH
cargo fmt -- --check
cargo clippy --features amd-xdna2 --lib -- -D warnings
cargo test --features amd-xdna2 --lib
bash -n tools/silicon_lab_operator.sh scripts/lab_exec_once.sh tools/*.sh
bash tools/silicon_contract_check.sh || true
bash tools/silicon_lab_preflight.sh --print-env || true
bash tools/silicon_lab_operator.sh status || true
bash scripts/lab_exec_once.sh --silicon-mladf  # dry-run only
```

**Never** set `XDNA2_ALLOW_EXEC_CMD=1` in default tests. Dry-run green ≠ `ran_on_npu`.

---

## Env recipe (dry-run first)

```bash
# Fish-safe: run block under bash, or open a bash subshell.
bash tools/export_mladf_wts.sh
bash tools/stage_silicon_lab_assets.sh --wts-from testdata/xdna2_cmd/mladf_wts_export/wts.bin
bash tools/silicon_lab_preflight.sh --require-pass-assets --print-env
bash scripts/lab_exec_once.sh --silicon-mladf --require-pass-assets
# Live C2: one-shot C_UNLOCK=b17b0119 (top SSoT) — unlock ≠ Pass; restore L3 after.
# Unpatched CLASS A / mere diff rc=0 / registry alone / parity green / CONTRACT_OK /
# residual_class / CLASSIFY_SOFT_OK / checklist / OFFLINE_GATES_OK ≠ Pass (top SSoT).
# Further C needs new agent unlock (evidence may justify; never auto-unlocks).
# INSTR_BO true post-patch public path closed (registry = cache, not live run_ BO).
# apply: bash tools/hitl_c_pass_apply.sh --assume-l3-already-open
```

See `docs/SILICON_PATH_LAB.md` · `docs/XDNA2_XRT_CONTRACT_MLADF.md` · `docs/HITL_BC.md` · `docs/HITL_C_APP_LOAD_RECIPE.md`.
