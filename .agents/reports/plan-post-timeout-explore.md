# Explore report: post-timeout offline plan scope (`C_UNLOCK=3a70fabf`)

**Date:** 2026-08-10
**Mode:** Read-only inventory for a follow-on offline plan.
**Honesty:** `invent_pass=0` · **no** product Pass · **no** `ran_on_npu` · timeout ≠ invent APP_LOAD class

---

## 1. RESIDUAL.md top (facts)

Source: `/home/hunter/Projects/ai/dragon-npu/RESIDUAL.md` (plain status table + LIVE bookkeeping).

| Topic | Fact |
|-------|------|
| **What broke** | Live shot under `3a70fabf` (stock, PDI+CONFIG_CU): outer **30s** killed `npu_direct_bench` (**rc=124** → lab exit 4). |
| **last_apply** | **Missing** (killed mid-flight before stamp). |
| **Journal (same window)** | **IO_PAGE_FAULT** on amdxdna + **`aie2_set_cmd_timeout`** (DPU PC `0xffffffff`, TXN OP ID `0xd`, ctx 15). Incomplete / FW timeout class, **not** a clean finished `0x3000003` APP_LOAD snapshot from this shot. |
| **Prior residual** | 2026-07-18 locked APP_LOAD after aiebu still historical baseline. |
| **Unlock** | **`C_UNLOCK=3a70fabf` CONSUMED** after real stock live EXEC attempt. Further live C needs **new** host-minted unlock + named offline reason. Prior `b17b0119` historical only. |
| **Pass / ran_on_npu** | **No.** invent_pass=0. Timeout ≠ invent APP_LOAD. |
| **Host** | Stock (`exec_cmd_allow=missing`); restore OK; refcnt 0 / accel ok (not wedged per residual). |
| **Capture** | `testdata/xdna2_cmd/external_known_good/` still needs real bytes. |
| **Highest-value next (residual)** | **Agent offline:** progressive last_apply stamps so kill/timeout still leaves evidence; optional journal-token interpret. **Human optional:** retry only after **new** unlock + longer outer timeout; hardened module; path B capture. **No** reopen under `3a70fabf`. |
| **Bans** | Empty reconfirm / empty-folder implement frozen (`AGENTS.md` §6–9). No invent Pass / capture bytes. Collect only if last_apply **exists**. |

---

## 2. Live-shot timeout report

Source: `/home/hunter/Projects/ai/dragon-npu/.agents/reports/live-shot-3a70fabf-timeout.md` (**exists**).

- Operator ran CONFIG_CU shot under unlock `3a70fabf` on **stock** amdxdna (no L3). Offline contract green; prep/build OK.
- Path: `lab_exec_once` → `timeout 30s cargo run … npu_direct_bench` with silicon-mladf + `XDNA2_LAB_PDI_BLOB`.
- Child **did not finish**. Coreutils `timeout` → **rc=124** → lab **exit 4**. RESTORE clean (`l3_we_enabled=0`, ALLOW unset).
- Pinned path example: `/tmp/dragon_npu_last_apply_config_cu_477280.json` **never created**.
- `collect_post_c2_evidence` correctly refuses a **missing explicit** path.
- Class: process died **mid-flight** → **no** structured residual snapshot. Not the same as finished `ert_state=Error` + journal `0x3000003`.
- Unlock recommendation in report: mark **`3a70fabf` CONSUMED** (aligned with RESIDUAL top).

---

## 3. How last_apply is written today

### Call chain

| Piece | Path |
|-------|------|
| Bench entry | `examples/npu_direct_bench.rs` → `timed_matmul` |
| Closure `stamp` | `src/backends/amd_xdna2/direct.rs` ~4562–4596 → `finalize_lab_timed_result` |
| Finalize + write gate | `src/backends/amd_xdna2/silicon_contract.rs` `finalize_lab_timed_result` ~9586–9600 |
| JSON write | `write_last_apply_best_effort` / `write_last_apply_to_path` ~10264–10349 (best-effort, 0600, path confinement) |
| Path pin | env `XDNA2_LAB_LAST_APPLY_JSON` if allowed |

### `write_la` policy (`direct.rs`)

- `write_la_default = LAB_OFM early || XDNA2_LAB_LAST_APPLY_JSON set`
- After EXEC gate check: `write_la = write_la_default || try_submit` (EXEC open → write even without pin/OFM)

### Stamp call sites in `timed_matmul_direct` (`direct.rs`)

All stamps are on **normal Rust returns** after a stage fails or at the **end** after destroy hwctx. Grep shows **19** `stamp(` sites, including:

| When (return-path only) | Approx lines | Status examples |
|-------------------------|--------------|-----------------|
| CREATE_HWCTX failed | ~4616 | `hwctx_blocked` |
| CONFIG_CU attach failed | ~4653 | `config_cu_failed` |
| lab env invalid (instr size / rela / layout) | ~4706, 4732, 4758 | `lab_env_invalid` |
| instr load / BO / fill failures | ~4797–5341 (multiple) | `inst_blob_*`, fill fails, etc. |
| submit / wait / decide paths | ~5372–5448 | submit_wait / OFM paths |
| **Success / completed timed path** | **~5791** | after destroy; final status/detail |

### Progressive / mid-path stamps?

**None today.**

- No stamp after “hwctx OK, about to CONFIG_CU.”
- No stamp after “CONFIG_CU OK, about to fill.”
- No stamp before `submit_and_wait` without returning.
- **SIGTERM/SIGKILL from outer `timeout` never runs `stamp`.** Hang mid-ioctl → **zero JSON**.

That is the core product gap named by residual after this shot.

---

## 4. Tools: last_apply / journal / missing / timeout

| Tool | Path | Missing last_apply | Timeout-specific |
|------|------|--------------------|------------------|
| **collect_post_c2** | `tools/collect_post_c2_evidence.sh` | Explicit path not a file → **exit 2** `EVIDENCE_PATH_REFUSE`. No path → falls back to **locked** `testdata/xdna2_cmd/last_apply.json` or newest temp; can still print journal. | No special timeout class; journal slice is best-effort `journalctl` + rg. |
| **p5** | `tools/p5_post_c2_interpret.sh` | Explicit path missing → **exit 2**. No path → children use fixture discovery (baseline soft skip). | No timeout taxonomy. Header still mentions unlock bookkeeping for `3a70fabf` (stale vs CONSUMED residual — doc drift). |
| **c3** | `tools/c3_post_c2_classify.sh` | No path → **`C3_SKIP no_last_apply`** exit 0 (soft). `--require-new` → exit 2. Explicit bad path refuse → 2. | No. |
| **c4** | `tools/c4_pass_check.sh` | No fixture → **`C4_SKIP no_fixture`** exit 0; `--require-pass` → 2. Explicit missing → **C4_REFUSE** exit 2. | No. |
| **p6** | `tools/p6_true_pass_check.sh` | Same pattern as c4: **`P6_SKIP no_fixture`** soft; require → 2; path refuse → 2. | No. |
| **classify** | `tools/classify_last_apply_residual.sh` | Soft on fixture/env; used by p5/c3. | Classifies finished detail tokens; cannot invent residual from kill alone. |

**Implication:** After a timeout kill with **no** JSON, post tools either **refuse the pin path (exit 2)** or **soft-skip / re-read locked baseline** (easy to misread as “same residual as 2026-07-18” if operator forgets the pin was empty). Journal alone must **not** invent APP_LOAD class or Pass (residual honesty).

---

## 5. Default timeouts + stock sandwich docs

| Knob | Default | Where |
|------|---------|--------|
| Outer lab timeout | **30s** (`LAB_EXEC_TIMEOUT_S` / `--timeout-s`) | `scripts/lab_exec_once.sh:64` |
| WAIT_CMD | **5000 ms** (`XDNA2_WAIT_TIMEOUT_MS`) | `lab_exec_once.sh:65`; `DEFAULT_WAIT_CMD_TIMEOUT_MS` in `direct.rs:3641` |
| Apply build | Separate cargo build **outside** outer timeout window | `lab_exec_once.sh` apply path |
| Live run | `timeout ${TIMEOUT_S}s … cargo run … npu_direct_bench` | `lab_exec_once.sh` ~900+ |
| Non-zero process rc (incl. 124) | lab **exit 4** after restore | lab script |

Docs:

- `docs/XDNA2_EXEC_CMD_LAB.md` — outer `timeout 30s` + finite WAIT 5000; never infinite WAIT default; L1+L3 on **hardened**.
- `docs/XDNA2_NEVER_AGAIN.md` — triple gate; stock vs hardened privilege.
- `tools/hitl_config_cu_once.sh` — **stock**: `exec_cmd_allow=missing` → no sudo L3; userspace ALLOW only via lab_exec; wedge risk real without L2 null-guards; pins `XDNA2_LAB_LAST_APPLY_JSON` under `/tmp/dragon_npu_last_apply_config_cu_<pid>.json`.
- Live-shot report: longer outer budget (`--timeout-s 120`) is **retry hygiene**, not Pass; keep WAIT ~5–15s unless WAIT itself is suspect.

Normal WAIT timeout should still **return** and stamp. Outer 30s kill implies hang **outside** a healthy finite WAIT return (or multi-step setup longer than remaining budget).

---

## 6. Plans status (workspace)

| Plan | Path | Status vs post-timeout reality |
|------|------|--------------------------------|
| Session `plan.md` | Host session path cited in plans; **not** in-repo under `.agents/` | Canonical for formal plan mode when present; not re-read here as a workspace file. |
| App-load residual product | `.agents/plans/plan-app-load-residual-product.md` | **Approved**; P0–P2 done; P1 machinery landed; **P3** still blocked on capture bytes; **P4** table still says “Blocked on unlock” / implement line mixed — **stale vs RESIDUAL**: live shot **ran**, unlock **CONSUMED**, last_apply **missing**. P5 offline interpret ready but needs **real** last_apply; P6 criteria offline ready; product Pass live-only. |
| Make silicon real | `.agents/plans/plan-make-silicon-real.md` | Workflow **awaiting approve** in header; still says unlock **active unused** and last_apply locked baseline — **stale** after 2026-08-09 timeout shot. Critical path was sandwich-first; sandwich partially happened (timeout mid-flight). |

**Plan gap for next offline plan:** elevate former “optional product quality” progressive stamps (named in live-shot report § Offline agent deltas) to **required** agent work before any **new** unlock.

---

## Agent can do fully offline (no sudo, no EXEC_CMD, no L3)

1. **Progressive / partial last_apply stamps** in `timed_matmul_direct` after durable stages (at least: hwctx OK; config_cu ok/fail; pre-EXEC / pre-submit) so outer kill still leaves JSON. TDD pure + feature tests; honesty tokens `invent_pass=0 not_ran_on_npu=1`.
2. **Docs / residual / tool honesty** that outer timeout kill ⇒ missing last_apply is **expected**; collect exit 2 on missing pin is correct; do not invent residual class from journal alone.
3. Optional pure parsers for journal tokens seen this shot (`IO_PAGE_FAULT`, `aie2_set_cmd_timeout`, DPU PC / TXN OP) for soft reports — **never** map kill → Pass or locked APP_LOAD without snapshot.
4. Soft-tool notes: p5 unlock bookkeeping still saying “active 3a70fabf” should align with **CONSUMED** residual when editing docs (no empty reconfirm mill).
5. Offline gates already green for CONFIG_CU / p5 / p6 machinery — re-run only after **named** code delta, not empty reconfirm.
6. Prep tooling already agent-owned: `a-prep`, PDI dump, preflight, sandwich **print** (`tools/hitl_config_cu_once.sh`, `operator_app_load_kit.sh`).

---

## Human-only

1. Any **new** live EXEC / sandwich (needs **new** unlock after `3a70fabf` consumed).
2. Opening L3 on hardened hosts (`exec_cmd_allow=1` then restore 0); stock path still needs human paste of userspace ALLOW sandwich when re-shot.
3. Journal diagnosis beyond what agent can read offline; wedge recovery / module reload / hardened install.
4. Placing real capture under `testdata/xdna2_cmd/external_known_good/` + PROVENANCE.
5. Choosing longer outer timeout on **retry** after new unlock (`--timeout-s 120` etc.).
6. Marking unlock CONSUMED in residual was residual SSoT work after real shot (already done for `3a70fabf`).

---

## Safety: what would brick or waste human time

| Risk | Why |
|------|-----|
| **Re-live under `3a70fabf`** | Unlock **CONSUMED**; process law: one-shot used. Second use without new host-minted unlock violates residual / AGENTS. |
| **Empty reconfirm implement** | Capture still empty; same residual facts already proven; frozen §8–9. |
| **Sudo / L3 spam on stock** | Stock has no `exec_cmd_allow`; fake L3 ceremony wastes time. Hardened L3 left open is CRITICAL. |
| **Outer 30s default on complex CONFIG_CU+silicon-mladf** | Already failed once; another short outer budget may re-kill before stamp even after progressive stamps land. |
| **Stock wedge risk** | No L2 null-guards; historical EXEC oops / D-state open hang. Progressive stamps reduce **evidence** loss; do not remove wedge risk. |
| **Treating locked fixture p5 as new evidence** | Collect without pin falls back to `testdata/xdna2_cmd/last_apply.json` (2026-07-18 APP_LOAD baseline) — not the timeout shot. |
| **Inventing APP_LOAD / Pass from journal + rc=124** | Forbidden. Residual class for this shot = incomplete / unknown until structured last_apply exists. |
| **`XDNA2_ALLOW_EXEC_CMD=1` in default tests** | NEVER AGAIN L1. |

---

## Recommended required agent phases (elevate former “optional”)

Former live-shot report listed progressive stamps as **optional product quality**. Residual now ranks them **agent highest-value**. Recommend plan phases:

| Phase | Required? | Deliverable |
|-------|-----------|-------------|
| **A. Progressive last_apply** | **Required** before new unlock | Mid-stage `stamp`/write after hwctx success, after CONFIG_CU success/fail, and immediately pre-submit (best-effort overwrite same pin path). Document: kill still may lose in-flight ioctl stage. |
| **B. Timeout / missing-path honesty** | **Required** docs+tool notes | Recipe + collect/p5 notes: rc=124 + missing pin = incomplete; do not soft-claim baseline as this shot. |
| **C. Journal soft taxonomy (optional-but-useful)** | Recommended if A done | Pure parse of timeout / IO_PAGE_FAULT signals for soft classify; still `not_c_ready=1 invent_pass=0`. |
| **D. Default outer budget guidance** | Docs only | Document that first stock CONFIG_CU silicon path should use **≥120s** outer unless progressive stamps + known stage times say otherwise; WAIT stays finite. |
| **E. New unlock + live retry** | Human after A (+ B) | New host-minted unlock + named offline reason (stamps exist). Not agent live apply. |
| **F. PDI provenance / capture** | Human / later | Unchanged value if completed shot still APP_LOAD with `config_cu=1`. |
| **Not in plan** | — | Empty gates mill; reopen `3a70fabf`; invent Pass. |

---

## Critical file paths for implementers

| Path | Role |
|------|------|
| `RESIDUAL.md` | Unlock CONSUMED + residual SSoT |
| `.agents/reports/live-shot-3a70fabf-timeout.md` | Shot forensics |
| `src/backends/amd_xdna2/direct.rs` | `stamp` closure, timed path, CONFIG_CU attach, WAIT default |
| `src/backends/amd_xdna2/silicon_contract.rs` | `finalize_lab_timed_result`, `write_last_apply_*` |
| `examples/npu_direct_bench.rs` | Live bench entry |
| `scripts/lab_exec_once.sh` | Outer timeout 30s, WAIT 5s, exit 4 on kill |
| `tools/hitl_config_cu_once.sh` | Stock sandwich print, last_apply pin, post |
| `tools/collect_post_c2_evidence.sh` | Post pack; refuse missing pin |
| `tools/p5_post_c2_interpret.sh` | classify+c3+c4+p6 |
| `tools/c3_post_c2_classify.sh` | C3_SKIP if no last_apply |
| `tools/c4_pass_check.sh` / `tools/p6_true_pass_check.sh` | Soft skip / refuse |
| `docs/XDNA2_EXEC_CMD_LAB.md` | timeout + WAIT policy |
| `docs/XDNA2_NEVER_AGAIN.md` | L1/L2/L3; stock vs lab |
| `docs/HITL_C_APP_LOAD_RECIPE.md` | Live recipe |
| `testdata/xdna2_cmd/last_apply.json` | **Locked baseline only** (not this shot) |
| `testdata/xdna2_cmd/external_known_good/` | Capture still empty |
| `.agents/plans/plan-app-load-residual-product.md` | Approved product plan (P4 status needs refresh) |
| `.agents/plans/plan-make-silicon-real.md` | Silicon-real draft (stale unlock row) |

---

## One-line plan thesis

**Offline first:** make last_apply survive outer kill (progressive stamps + honesty), then human may take a **new** unlock with a longer outer budget; do **not** re-spin empty gates or re-fire `3a70fabf`.
