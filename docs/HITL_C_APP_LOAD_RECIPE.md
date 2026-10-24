# HITL C recipe — APP_LOAD residual (aiebu ELF)

**Purpose:** One canonical operator sequence for a **single** Milestone C Pass
apply against the locked residual (historical APP_LOAD_PDI_FAIL; later stock
shots also `wait_cmd_einval` after CONFIG_CU).

**Status:** **No active unlock.** SSoT: first `C_UNLOCK=` in
[RESIDUAL.md](../RESIDUAL.md) is **`C_UNLOCK=2ae69036` CONSUMED** (2026-08-10
flexml unified-2x4x4 stock CONFIG_CU; `wait_cmd_einval`; not Pass). All prior
one-shots are also **CONSUMED** (`97f38307`, `11bc9016`, `3a70fabf`,
`b17b0119`). Do **not** reopen them. Do **not** live apply without a **new**
host-minted unlock and a named offline reason. Empty reconfirm stamps and
empty-folder implement loops are **frozen** (see `AGENTS.md` §6–9 /
`RESIDUAL.md`). Offline green ≠ Pass ≠ `ran_on_npu`. Unlock **≠** Pass **≠**
permanent open season. invent_pass=0.

Post-apply offline = `tools/c3_post_c2_classify.sh` + `tools/c4_pass_check.sh` +
`tools/p6_true_pass_check.sh` + `tools/run_offline_gates.sh` (or one-shot
`tools/p5_post_c2_interpret.sh`). **Offline C4/P6 ≠ invent product Pass** /
≠ `ran_on_npu` claim.


| Claim | Truth |
|-------|--------|
| Active live unlock right now | **None** — see `RESIDUAL.md` first `C_UNLOCK=` |
| One sandwich under **`C_UNLOCK=2ae69036`** | **CONSUMED** (2026-08-10 flexml STX unified-2x4x4; pdi 27200 / `0eeeb70c…`; `wait_cmd_einval`; not Pass) |
| One sandwich under **`C_UNLOCK=97f38307`** | **CONSUMED** (CVML research-blob; orphan IO_PAGE_FAULT class) |
| One sandwich under **`C_UNLOCK=11bc9016`** | **CONSUMED** (stock CONFIG_CU + PDI; ≥120s outer) — historical only |
| One sandwich under **`C_UNLOCK=3a70fabf`** | **CONSUMED** (2026-08-09 stock timeout shot) |
| One L3 sandwich under `b17b0119` | **CONSUMED** — historical; do **not** re-run under it |
| Permanent open season / more C without **new agent unlock** | **No** |
| Silicon Pass / `ran_on_npu` | **No** until OFM allclose + ERT Completed |
| Leave L3 open / set permanent userspace ALLOW | **No** — always restore L3=`exec_cmd_allow=0`; never leave `XDNA2_ALLOW_EXEC_CMD=1` sticky |
| Agent completes live apply without operator sudo when L3 closed | **No** — fail-closed exit 1 |
| Default outer 30s for CONFIG_CU silicon | **Too short** — use `--timeout-s=120` (or ≥120); WAIT stays finite |

Soft residual class / `CLASSIFY_SOFT_OK` / `app_load_checklist=…` /
`OFFLINE_GATES_OK` / `C3_SKIP` / **`C4_SKIP`** still **≠** Pass and never
substitute for a new unlock. Offline `c4_pass_check` is snapshot audit only.

single source of truth residual: [RESIDUAL.md](../RESIDUAL.md) · B/C overview: [HITL_BC.md](./HITL_BC.md) ·
contract: [XDNA2_XRT_CONTRACT_MLADF.md](./XDNA2_XRT_CONTRACT_MLADF.md) ·
construction / capture playbook: [CONSTRUCTION_MAP_APP_LOAD.md](./CONSTRUCTION_MAP_APP_LOAD.md).

---

## Unlock policy (hard)

C runs **only if the agent explicitly says ready** with a **host-minted** unlock
stamp that is **active** in `RESIDUAL.md` (first `C_UNLOCK=` line, no
CONSUMED). **Current:** **no active unlock.** All of the following are
**CONSUMED** (do not reopen): `2ae69036` (flexml), `97f38307`, `11bc9016`,
`3a70fabf`, `b17b0119`. Further live applies need a **new** unlock + named
offline reason — not open season, not empty gate-only hex. Evidence may
**justify** unlock; evidence **never auto-unlocks** C.

**Outer timeout hygiene (CONFIG_CU silicon):** recommend `--timeout-s=120` (or
≥120) on `hitl_c_pass_apply` / `lab_exec_once`. Default lab outer **30s** killed
`npu_direct_bench` (rc=124 → lab exit 4) before a finished last_apply on the
CONSUMED shot. **WAIT** stays finite (`XDNA2_WAIT_TIMEOUT_MS` ~5–15s; never
infinite by default). Progressive last_apply stamps mid-path after hwctx /
CONFIG_CU / pre-submit so kill after those stages still leaves JSON; kill
**during** a single ioctl can still lose that stage.

**Not** unlocked by any of (and none of these equal Pass either):

| Signal | Why it is not unlock |
|--------|----------------------|
| `CONTRACT_OK` / offline gates green / `OFFLINE_GATES_OK` | Userspace offline only (`tools/run_offline_gates.sh`) — **≠ agent unlock** |
| Unpatched `CLASS_HINT A` / `diff rc=0` | Body match only (classic 1688) |
| Registry / `registry_get_instr_bo` | Transaction/cache image — **not** proven live `run_` BO |
| Vendor miss soft-skip | Soft WARN in hitl_c; never unlock |
| `DRY_RUN_READY=1` / preflight green | Assets only; ≠ `ran_on_npu` |
| Env flag trials | **Ban** — content/xrt3/rela closed (same APP_LOAD) |
| Soft residual class / `CLASSIFY_SOFT_OK` / `app_load_pdi_fail_after_aiebu` / `not_c_ready=1` | Feedback only (`tools/classify_last_apply_residual.sh`); **≠ agent unlock** |
| Soft `app_load_checklist=…` / `class_implies_app_load_after_aiebu=1` / map_offset matrix | Offline signal lock only (`format_app_load_offline_checklist`); **≠ agent unlock** |
| `C3_SKIP` / `C3_EVIDENCE=1` | Offline evidence pipeline only (`c3_post_c2_classify.sh`); **≠ Pass** |
| `C4_SKIP` / `C4_PASS_CANDIDATE` | Offline Pass-evidence audit only (`c4_pass_check.sh`); **≠ invent product Pass** / **≠** `ran_on_npu` |
| `P6_SKIP` / `P6_NOT_PASS` / `P6_OFFLINE_CANDIDATE` | True product-Pass **criteria** offline (`p6_true_pass_check.sh`); **≠ invent product Pass** / live decide still owns flag |

Unpatched CLASS A **≠** C-ready **≠** `ran_on_npu`. Offline green **≠** agent unlock.
Soft `residual_class=…` report **≠** agent unlock. Offline **PassCandidate** audit **≠** product Pass.

---

## Prerequisites (offline — agent or operator, no sudo)

Under `testdata/xdna2_cmd/silicon_lab/` (or restage):

| Asset | Expect |
|-------|--------|
| `lab_instr.elf` | aiebu ELF (~32264 B), magic `7f454c46` — **not** raw `lab_instr.bin` EXEC |
| `wts.bin` | 174080 B, non-zero |
| `lab_ctrl.bin` | 29664 B, non-zero |
| `lab_ctrl_meta.json` | present (patch locations) |
| `ofm_golden.bin` | 512 B bf16 (or 1024 f32), non-zero |

Stage if needed:

```bash
cd ~/Projects/ai/dragon-npu
bash tools/export_mladf_wts.sh   # if WTS export missing
bash tools/stage_silicon_lab_assets.sh --wts-from testdata/xdna2_cmd/mladf_wts_export/wts.bin
# ELF if only .bin present:
bash tools/stage_aiebu_lab_instr.sh
```

### Offline gate sequence (run before any C; safe anytime)

```bash
# bash:
export PATH="/usr/bin:${HOME}/.local/bin:${PATH}"
# fish:
# set -gx PATH /usr/bin $HOME/.local/bin $PATH

cd ~/Projects/ai/dragon-npu

# One-shot offline suite (≠ C-ready):
bash tools/run_offline_gates.sh

cargo fmt -- --check
cargo clippy --features amd-xdna2 --lib --example classify_last_apply -- -D warnings
cargo test --features amd-xdna2 --lib pure_direct
cargo test --features amd-xdna2 --lib silicon_contract
cargo test --features amd-xdna2 --lib classic_1688

bash tools/diff_vendor_instr_parity.sh          # unpatched A common; ≠ C-ready
bash tools/test_diff_vendor_parity_offline.sh   # pass=4
bash tools/silicon_contract_check.sh            # CONTRACT_OK ≠ C-ready / ≠ unlock
bash tools/classify_last_apply_residual.sh      # soft; residual_class ≠ unlock
bash tools/silicon_lab_preflight.sh --require-pass-assets

# Script help only (no apply):
bash tools/hitl_c_pass_apply.sh --help
```

`hitl_c` has **no** dry-run apply mode: it always goes to lab EXEC when L3 is
open. Offline prep = gates + preflight above. With L3 closed,
`--assume-l3-already-open` fails early (prints open recipe) and does **not**
EXEC.

**Live hitl_c (when unlocked)** also runs
`silicon_contract_check.sh --require-pass-assets` by default (A9 hard gate;
stricter than soft contract alone), then `silicon_lab_preflight --require-pass-assets`
again. Manual offline prep above mirrors that path without L3.

---

## Exact apply — **historical** `C_UNLOCK=11bc9016` (CONSUMED; record + shape only)

> **`C_UNLOCK=11bc9016` CONSUMED.** Do **not** re-run live under this stamp.
> The steps below document the **shape** of a CONFIG_CU + PDI sandwich
> (**≥120s** outer) for when a **new** host-minted unlock is active in
> `RESIDUAL.md`. Unlock **≠** Pass **≠** `ran_on_npu` **≠** permanent open
> season. Always **restore L3** after apply (hardened host). Later flexml
> shot `2ae69036` and priors are also CONSUMED. Agent does **not** open L3 or
> run live apply without a new unlock.

### CONFIG_CU pre-step (before sandwich) — **preferred one command**

```bash
cd ~/Projects/ai/dragon-npu
export PATH="/usr/bin:${HOME}/.local/bin:${PATH}"

# Recommended: prep + pin last_apply + preflight + fish/bash sandwich with real paths:
bash tools/operator_app_load_kit.sh config-cu
# same: bash tools/hitl_config_cu_once.sh prep

# Then paste the printed sandwich (open L3 if hardened → hitl_c apply → restore → post-evidence).
# After restore, if you skipped the printed post line:
# bash tools/collect_post_c2_evidence.sh /tmp/dragon_npu_last_apply_config_cu_….json
```

Manual equivalent (if you prefer stepwise):

```bash
bash tools/operator_app_load_kit.sh a-prep
# export / set -gx XDNA2_LAB_PDI_BLOB=… from a-prep; optional XDNA2_LAB_CU_FUNC=0
# Prefer a shot-specific last_apply pin so locked baseline stays for C3:
# export XDNA2_LAB_LAST_APPLY_JSON=/tmp/dragon_npu_last_apply_config_cu_$$.json
```

Wrong overlay may still journal `0x3000003`. CONFIG_CU attach alone **≠** Pass.
`hitl_c_pass_apply` with PDI set auto-pins last_apply under `/tmp` when unset.

### Offline prep + printable sandwich (no L3 from agent)

```bash
cd ~/Projects/ai/dragon-npu
export PATH="/usr/bin:${HOME}/.local/bin:${PATH}"
# Preferred CONFIG_CU max-prep (paths filled; no L3):
bash tools/hitl_config_cu_once.sh prep
# Soft unlock NOTE + generic sandwich (also fine):
bash tools/hitl_c2_once.sh --print-only
# Offline residual / Pass audit (any time; ≠ unlock):
bash tools/c3_post_c2_classify.sh
bash tools/c4_pass_check.sh
bash tools/run_offline_gates.sh
# Fail-closed when L3 closed (expect exit 1) — refuse demo, not apply:
# bash tools/hitl_c_pass_apply.sh --assume-l3-already-open
```

### Live sandwich (operator only — **requires active unlock** in RESIDUAL)

Keep `XDNA2_LAB_PDI_BLOB` set in the same shell that runs apply. Prefer the
exact lines from `operator_app_load_kit.sh config-cu` or
`hitl_c2_once.sh --print-only` **only when** first `C_UNLOCK=` is active (not
CONSUMED). Shape (hardened L3; stock hosts omit sudo):

```bash
cd ~/Projects/ai/dragon-npu
export PATH="/usr/bin:${HOME}/.local/bin:${PATH}"
# optional: export XDNA2_LAB_BO_DIAG=1
# never leave sticky: unset XDNA2_ALLOW_EXEC_CMD
# export XDNA2_LAB_PDI_BLOB=…   # from a-prep
# export XDNA2_LAB_CU_FUNC=0    # optional

# Active unlock required (RESIDUAL first C_UNLOCK= without CONSUMED); ≠ Pass until live decide
sudo sh -c 'echo 1 > /sys/module/amdxdna/parameters/exec_cmd_allow'   # skip on stock
bash tools/hitl_c_pass_apply.sh --assume-l3-already-open --timeout-s=120
sudo sh -c 'echo 0 > /sys/module/amdxdna/parameters/exec_cmd_allow'   # skip on stock
cat /sys/module/amdxdna/parameters/exec_cmd_allow   # must be 0 or N (or missing on stock)
```

## Exact apply — **historical** `C_UNLOCK=3a70fabf` (CONSUMED; record only)

> **`C_UNLOCK=3a70fabf` CONSUMED** after 2026-08-09 stock timeout shot. Do **not**
> re-run under this stamp. Historical permission was for one CONFIG_CU sandwich;
> outer 30s killed the bench before finished last_apply. See
> `.agents/reports/live-shot-3a70fabf-timeout.md`.

What hitl_c enforces (default path when unlocked):

- Forces **`lab_instr.elf`** (refuses non-ELF magic; no `--blob` override)
- Runs **`silicon_contract_check.sh --require-pass-assets`** (A9) — hard-fail
  unless skip/force (see flags below)
- Then **`silicon_lab_preflight.sh --require-pass-assets`**
- Single-shot silicon-mladf apply; optional CONFIG_CU if `XDNA2_LAB_PDI_BLOB` set
- Post-check `exec_cmd_allow=0` (exit **5** if still open)

### Historical CONSUMED — `C_UNLOCK=b17b0119` (do not re-run)

> **Unlock CONSUMED.** Do **not** open execute-allow under `b17b0119`. Record-only
> archaeology. Live apply under that stamp is **closed**.

```bash
# HISTORICAL record-only — C_UNLOCK=b17b0119 CONSUMED; do not open L3 under this stamp
# sudo sh -c 'echo 1 > /sys/module/amdxdna/parameters/exec_cmd_allow'
# bash tools/hitl_c_pass_apply.sh --assume-l3-already-open
# sudo sh -c 'echo 0 > /sys/module/amdxdna/parameters/exec_cmd_allow'
```

### Flags — force vs skip (research only — **not** unlock)

| Flag / env | Contract behavior | Still enforced |
|------------|-------------------|----------------|
| *(default)* | **Runs** A9 with `--require-pass-assets`; **exit 2** if red | ELF magic, preflight, apply |
| `--force-apply-despite-offline-red` / `--force-apply` | **Still runs** A9; on red **continues** with WARN (`contract_status=red_forced`) | ELF magic, preflight, apply |
| `--skip-offline-contract` / `XDNA2_HITL_SKIP_CONTRACT=1` | **Does not run** A9 (`contract_status=skipped`); sticky env loud-WARN | ELF magic, preflight, apply |

Force ≠ skip. Force is “run contract, allow red”; skip is “omit contract entirely.”
Neither is agent unlock. Unset sticky `XDNA2_HITL_SKIP_CONTRACT` after re-apply.

| Flag | Meaning |
|------|---------|
| `--assume-l3-already-open` | Operator already set L3=1; exit **1** if still closed |
| `--no-bo-diag` / `--no-build` / `--timeout-s=N` | Lab UX only |

---

## Exit codes (hitl_c → lab_exec)

Same family as `scripts/lab_exec_once.sh`, plus hitl_c early exits:

| rc | Meaning |
|----|---------|
| **0** | Rare Pass: apply process 0 **and** L3 restored / closed (`ran_on_npu=true` expected) |
| **1** | Preflight refuse (WEDGED / high refcnt / …) **or** hitl_c `--assume-l3-already-open` but L3 still closed |
| **2** | Bad args / missing ELF / non-ELF magic / **offline contract red** (default A9) / preflight Pass-assets fail |
| **3** | Root/sudo needed but not available (`--print-sudo-only` / password required) |
| **4** | Live process non-zero or timeout **and** L3 restored OK — common `!ran_on_npu` / APP_LOAD residual |
| **5** | L3 left open / restore failed (**CRITICAL** — close L3 manually) |

**Note:** hitl_c POST always elevates to **rc=5** if `exec_cmd_allow` is still open,
**masking** a prior lab **4** (or other rc). Fix L3 first, then re-read whether
silicon failed vs restore failed.

```bash
# always verify after any apply (offline check OK; restore only if L3 left open):
cat /sys/module/amdxdna/parameters/exec_cmd_allow   # expect 0 or N
# if not 0/N (CRITICAL restore — only when L3 was opened under a live unlock):
# sudo sh -c 'echo 0 > /sys/module/amdxdna/parameters/exec_cmd_allow'
```

---

## Expected detail (userspace OK) vs current residual

Expect userspace path fields similar to locked C log (2026-07-16):

- `BLOB size≈32264`, `aiebu_elf=1`, `aiebu_rela_patched`, `rela_count=12`
- `inst_size_mode=content` `content_size=1688B` `bo_size=4096B` `start_npu_size=1688B`
- `qos_gops=100 qos_fps=30`, `tiles=32` (unless env override)

**Honesty until root cause fixed:** still **likely**

- exit **4** / `!ran_on_npu` (or **5** if L3 left open — restore first)
- `ert_state=5(Error)`
- OFM all zeros
- journal **APP_LOAD_PDI_FAIL** `0x3000003` (and often `0x4000003` INVALID_PARAM class nearby)

That is the **locked residual**, not a failed offline gate.

### Ban env C spam

Do **not** re-open live trials of:

- `XDNA2_LAB_INSTR_SIZE_MODE` (content trial closed — still APP_LOAD)
- `XDNA2_LAB_KERNEL_ARG_LAYOUT=xrt3` (closed — same APP_LOAD)
- `XDNA2_LAB_RELA_MODE=add` — **banned live** until agent explicitly unlocks a
  new offline-justified experiment (RESIDUAL: env HITL closed)
- tile / layout flag ping-pong

Defaults stay `content` + `count4` + `overwrite`.

---

## Success criteria (true Pass only) — P6

Claim `ran_on_npu` / product Pass **only** when **all** hold (live product path):

1. Real ioctl path (`real_ioctl=true`, not stub)
2. ERT **Completed** (`ert_state=4`) and/or status exact `ok_npu_verified`
3. OFM **non-zero** and **allclose** vs non-zero golden — only via live
   `decide_ran_on_npu` (offline snapshot cannot prove allclose)
4. `json_ran_on_npu=true` only when the product timed path set it (never invent
   from offline green / C4 PassCandidate)
5. Bench / hitl_c exit **0** with product `ran_on_npu=true`

**Offline helper (criteria readiness, not a Pass claim):**

```bash
bash tools/p6_true_pass_check.sh
# locked fixture → P6_NOT_PASS / P6_SKIP invent_pass=0 product_pass_requires_live_decide=1
# bash tools/p6_true_pass_check.sh --require-pass   # exit 2 if not offline candidate
```

SSoT pure tokens: `format_true_pass_criteria_checklist()` always includes
`invent_pass=0 not_c_ready=1 product_pass_requires_live_decide=1 offline_audit_only=1`
and `c4_pass_candidate_neq_product_pass=1`. Dual residual: userspace ELF+rela OK
≠ FW accept ≠ product Pass.

Never claim Pass from envelope accept alone, zero golden, offline
`CONTRACT_OK`, C4_PASS_CANDIDATE, P6_OFFLINE_CANDIDATE, or “gates green so try C.”

### If not Pass — paste for agent

- `status`, `aiebu_*`, `inst_size_mode=`, `start_npu_size=`, `layout=`, `ert_state`, `ofm_stats`, `fw_status_hint`
- `RESIDUAL_ENV` banner line from hitl_c
- hitl_c / lab_exec **exit code** (and whether L3 was left open → 5)
- Optional: `testdata/xdna2_cmd/last_apply.json` or `/tmp/dragon_npu_last_apply_*.json`
- Journal:
  `journalctl -k -b --no-pager | rg -i 'Failed cmd|Status 0x|APP_LOAD|amdxdna' | tail -20`

Always restore L3 after any **live** apply on a hardened host (not under CONSUMED unlocks):

```bash
# After live apply (new unlock only) — always verify restore on hardened hosts:
# sudo sh -c 'echo 0 > /sys/module/amdxdna/parameters/exec_cmd_allow'
cat /sys/module/amdxdna/parameters/exec_cmd_allow   # must be 0 or N
```

### Post-C2 offline (operator or agent — no L3)

**P5 one-shot (preferred after sandwich):**

```bash
# classify + C3 + C4; prints config_cu tokens when present; ≠ Pass ≠ auto-CONSUME
bash tools/p5_post_c2_interpret.sh
# optional path pin:
# bash tools/p5_post_c2_interpret.sh /tmp/dragon_npu_last_apply_*.json
```

**Step-by-step (same tools under the hood):**

```bash
# C3 pipeline (soft classify + body-first evidence; ≠ C-ready / ≠ Pass):
#   fingerprint == locked baseline → C3_SKIP (baseline_locked_app_load /
#     same_body_as_baseline; path alone / same-body copy ≠ evidence)
#   fingerprint differs (incl. config_cu= bit / status config_cu_failed) →
#     C3_EVIDENCE=1 even if residual still APP_LOAD
#     (shape_still_locked_app_load=1 annotation only — never force skip)
bash tools/c3_post_c2_classify.sh
# hard research mode (exit 2 if no new C2 body):
# bash tools/c3_post_c2_classify.sh --require-new
# raw classify only (expect config_cu_present=0|1 in checklist):
# bash tools/classify_last_apply_residual.sh

# C4 Pass-evidence audit (soft; baseline → C4_SKIP; ≠ invent product Pass):
bash tools/c4_pass_check.sh
# hard research mode (exit 2 if not PassCandidate):
# bash tools/c4_pass_check.sh --require-pass

# P6 true-Pass criteria (soft; baseline → P6_NOT_PASS/P6_SKIP; ≠ product Pass):
bash tools/p6_true_pass_check.sh
# bash tools/p6_true_pass_check.sh --require-pass

# Offline gates still ≠ C-ready:
bash tools/run_offline_gates.sh
```

**CONFIG_CU token expectations (after live path A sandwich):**

| Token / field | Meaning |
|---------------|---------|
| `config_cu=1` in detail | Lab attach issued CONFIG_HWCTX(CONFIG_CU) |
| `pdi_bytes=N` / `cu_func=N` | Opaque PDI length + CU function index |
| `status=config_cu_failed` | Attach failed fail-closed (still ≠ Pass) |
| `config_cu_present=1` (checklist) | Soft research flag from parse |
| `config_cu_attached_still_app_load=1` | Attach OK but residual still APP_LOAD |
| Always | `invent_pass=0 not_c_ready=1 not_ran_on_npu=1` |

Locked baseline fixture has **no** `config_cu=` → `config_cu_present=0` + C3_SKIP.

**Honesty:** Evidence = **fingerprint/body differs** from the locked baseline
fixture — not path or temp alone. Same-body copy under another path → `C3_SKIP`.
C3 without a **new** last_apply body (still only the locked fixture)
→ `C3_SKIP` ≠ silicon progress. Do **not** re-open the closed APP_LOAD signal
matrix on the same fixture. **`C4_SKIP`** on the same fixture is expected until
a real Pass snapshot (`json_ran_on_npu=true` + ERT Completed + non-zero OFM).
Offline C4 audit **≠** invent product Pass (live `decide_ran_on_npu` still owns
product flag). Mark an unlock **CONSUMED** in RESIDUAL only after a real
sandwich produced new last_apply — never from locked-fixture soft reconfirm.
All prior one-shots (`2ae69036`, `97f38307`, `11bc9016`, `3a70fabf`,
`b17b0119`) remain **CONSUMED** (do not reopen).

Then share classify output + last_apply + journal with agent for **C3** (fail
residual) or **C4** (only if OFM allclose + ERT Completed + `ran_on_npu=true`).

---


---

## Step 2 prep checklist — minimal load image + optional debug BO

**When to use:** after a **new** host-minted `C_UNLOCK` is active in `RESIDUAL.md`
(first `C_UNLOCK=` line, **no** CONSUMED), with a **named offline reason**
(unspent PDI fingerprint, capture-dir material, or a residual class that is not
“re-live spent flexml alone”).

**Do not use** under CONSUMED unlocks. **Do not** treat this checklist as Pass
or as permission to mint unlocks. invent_pass=0. Offline C6 pack helpers ≠ live.

### Preconditions (all required)

| Check | Requirement |
|-------|-------------|
| Active unlock | First `C_UNLOCK=` in `RESIDUAL.md` is **active** (not CONSUMED) |
| Named reason | Written offline reason (new material FP, unspent PDI, skip-PDI A/B, etc.) |
| Not spent re-live alone | Spent flexml `0eeeb70c…` / 27200 B under `2ae69036` alone is **not** a reason |
| L3 restore plan | You will restore `exec_cmd_allow=0` after the shot; no sticky `XDNA2_ALLOW_EXEC_CMD=1` |
| Outer timeout | Prefer `--timeout-s=120` (or ≥120) on CONFIG_CU silicon |

### CONSUMED unlocks (do **not** reopen)

| Unlock | Note |
|--------|------|
| `2ae69036` | flexml unified-2x4x4; `wait_cmd_einval`; spent sha16 `0eeeb70c…` |
| `97f38307` | CVML research-blob; orphan IO_PAGE_FAULT class |
| `11bc9016` | stock CONFIG_CU historical |
| `3a70fabf` | stock timeout historical |
| `b17b0119` | first one-shot historical |

### Preferred load target (minimal first)

1. **Material first (best):** a real capture under
   `testdata/xdna2_cmd/external_known_good/` (ctrltext / aiebu ELF / research PDI)
   with PROVENANCE filled — or ingest via
   `bash tools/pass_unblock_once.sh --ingest /path/to/file`.
2. **Unspent research PDI** (if no capture yet): prefer an **unspent** fingerprint
   from the residual ledger (e.g. flexml `llm-2x4x4` / `unified-4x4`), **not**
   re-live of spent `0eeeb70c…` alone.
3. **Minimal image:** smaller / identity-style load path before a full llm
   partition when the residual is still “firmware load accept,” not OFM matmul.

### Optional debug BO offline prep (C6; research only — no live assign)

Public uAPI (AIE2/Strix value-mode). Full offline landing:
`.agents/reports/impl-p2-c6-recipe.md`. Glossary:
[CONSTRUCTION_MAP_APP_LOAD.md](./CONSTRUCTION_MAP_APP_LOAD.md) DBG_BUF row.

| Item | Detail |
|------|--------|
| Ops | `DRM_AMDXDNA_HWCTX_ASSIGN_DBG_BUF` (=1) / `REMOVE_DBG_BUF` (=2) |
| Pack | Pure helpers: `pack_config_hwctx_dbg_buf_aie2` /
  `validate_config_hwctx_dbg_buf_aie2` in `direct.rs` (`param_val` = BO handle
  **value**, `param_val_size=0`) |
| Soft note | `format_c6_debug_bo_prep_note` / `debug_bo_uapi_prep_aie2` — always
  `invent_pass=0 not_c_ready=1 not_ran_on_npu=1 debug_bo_prep_alone_not_pass=1 research_only=1` |
| Research gate | `XDNA2_LAB_RESEARCH_DEBUG_BO` — when **unset** (default),
  `DirectSubmission::config_hwctx_dbg_buf` **refuses** and lab timed path
  does **not** create/assign a debug BO. When set, lab timed_matmul may
  CREATE 4096 B **Dev** (`AMDXDNA_BO_DEV`) + Assign after hwctx (and after
  CONFIG_CU if used); guard REMOVE+destroy on teardown. Env alone ≠ Pass /
  ≠ unlock / ≠ `ran_on_npu`. Not EXEC_CMD (no `XDNA2_ALLOW_EXEC_CMD`). |
| Lab type/size | `LAB_RESEARCH_DEBUG_BO_TYPE=Dev` + size **4096**. Driver
  `aie2_hwctx_cfg_debug_bo` looks up with `AMDXDNA_BO_DEV` only (Shmem-4096
  failed live `48da6e60` / `Get bo N failed`). XRT: CACHEABLE+USE_DEBUG → Dev. |
| Wire soft | `format_research_debug_bo_config_detail` / lab detail_token always
  `invent_pass=0 not_c_ready=1 not_ran_on_npu=1 debug_bo_alone_not_pass=1` |
| Offline verify | `cargo test --features amd-xdna2 --lib debug_bo` (or `c6_debug` /
  pack_config_hwctx_dbg / config_hwctx_dbg / research_debug_bo filter) |
| Honesty | Debug BO pack / env / soft note / assign alone **≠** Pass / **≠**
  `ran_on_npu` / **≠** C-ready |

**Research lab path:** when env is set, `timed_matmul_direct` auto-creates a
minimal **Dev** debug BO and Assigns (fail-closed on error; status
`debug_bo_failed`). When unset, zero behavior change. Live L5b needs a **new**
unlock + named reason (Dev vs spent Shmem); assign alone **≠ Pass**.

### Skip-CONFIG_CU offline prep (load-path isolation; no live here)

Full env / spent refuse / residual classes:
[CONSTRUCTION_MAP_APP_LOAD.md](./CONSTRUCTION_MAP_APP_LOAD.md)
§ Skip-CONFIG_CU offline prep checklist. Report:
`.agents/reports/impl-next-load-path-offline.md`.

| Offline now | Live later (new unlock only) |
|-------------|------------------------------|
| `bash tools/operator_app_load_kit.sh config-cu-skip` | `… config-cu-run-skip` |
| `unset XDNA2_LAB_PDI_BLOB`; `export XDNA2_LAB_SKIP_CONFIG_CU=1` | same env under active `C_UNLOCK` |
| Preflight: `bash tools/hitl_config_cu_once.sh preflight` after skip prep | outer `--timeout-s=120` |

**B1 residual-ready hypothesis (not proven RC):** lab classic pack key
`mladfmatmulbias_prmptflat_mladf_2x4x4_flat_a16fw4acc16f_1_1152_256_128` ran
under flexml CONFIG_CU overlays that attached then failed post-attach. Possible
overlay vs instruction topology mismatch — **hypothesis only**; invent_pass=0.
**skip-CU prep ≠ Pass.** Do not re-export spent flexml sha16 as PDI for “new”
progress.

### Operator sequence (under active unlock only)

| Step | Action |
|------|--------|
| 1 | Offline prep: `bash tools/operator_app_load_kit.sh config-cu` **or** `config-cu-skip` (A/B) (or `pass-unblock --agent-offline`) |
| 2 | Export load image env (`XDNA2_LAB_PDI_BLOB=…` for CONFIG_CU arm) **or** keep PDI unset + `XDNA2_LAB_SKIP_CONFIG_CU=1` for skip arm; never spent flexml alone |
| 3 | Optional research: set `XDNA2_LAB_RESEARCH_DEBUG_BO=1` so lab timed path
  CREATE+Assign 4096 B **Dev** debug BO (invent_pass=0; still needs **new**
  unlock for L5b; do not re-live spent Shmem-4096 under CONSUMED `48da6e60`) |
| 4 | Live sandwich: kit `config-cu-run` **or** `config-cu-run-skip` / recipe L3 lines **only if unlock active**; outer `--timeout-s=120` |
| 5 | **Restore L3** immediately (`exec_cmd_allow=0`) on hardened hosts |
| 6 | Collect: `bash tools/operator_app_load_kit.sh post-evidence [/path/to/last_apply.json]` |
| 7 | Interpret: `bash tools/p5_post_c2_interpret.sh` (or `c3_post_c2_classify` + `c4_pass_check` + `p6_true_pass_check`) |
| 8 | Share last_apply + journal + classify with agent; mark unlock **CONSUMED** after a real shot |

Also: `bash tools/collect_post_c2_evidence.sh` for journal + pack after the shot.

### What this checklist never claims

| Claim | Truth |
|-------|--------|
| Product Pass / `ran_on_npu` | **No** until OFM allclose + ERT Completed + real ioctl decide |
| C6 pack helpers alone | Research layout only — **≠** C-ready |
| Skip-CONFIG_CU prep / skip live alone | Isolation only — **≠** Pass |
| Spent flexml re-live alone | **Not** progress without new unlock + named delta |
| Empty capture dir soft-compare | **Not** progress |


## Highest-value next (after this recipe)

1. **No live sandwich until a new unlock** is host-minted in `RESIDUAL.md` with
   a named offline reason (prefer **skip-CONFIG_CU isolation**, or new
   capture-dir material / BO-map offline fix if Phase A lands one). Do **not**
   reopen CONSUMED stamps (includes `efd44f35` / `8352882b` / all three flexml
   STX research PDIs Spent). Follow **Step 2 prep checklist** above (minimal
   load first; optional debug BO research offline). Prep-only anytime:
   `bash tools/operator_app_load_kit.sh config-cu-skip` or `config-cu` (or
   `pass-unblock --agent-offline`). Live shape still uses **`--timeout-s=120`**.
   Unlock **≠** Pass.
2. **Post-apply offline residual / Pass audit (P5+P6)** after any future live:
   `p5_post_c2_interpret.sh` or `c3_post_c2_classify` + `c4_pass_check` +
   `p6_true_pass_check` + `run_offline_gates` → agent **C3** when fingerprint
   differs / **C4** only if `C4_PASS_CANDIDATE` (product Pass = live decide;
   P6 still `product_pass_requires_live_decide=1`). Same body as baseline →
   `C3_SKIP` + `C4_SKIP` + `P6_SKIP`.
3. **Capture path (parallel, no unlock):** public-stack ctrltext playbook in
   [CONSTRUCTION_MAP_APP_LOAD.md](./CONSTRUCTION_MAP_APP_LOAD.md) § Public-stack
   ctrltext extract → `bash tools/pass_unblock_once.sh --ingest …` → fill
   `testdata/xdna2_cmd/external_known_good/PROVENANCE.md` → soft compare.
   Empty gate-only maintain stamps remain **frozen**. Offline green alone still
   **≠** Pass / C-ready / `ran_on_npu`. invent_pass=0.

**Live path:** only under a **new** active unlock (never CONSUMED stamps above).
**Always after live:** restore L3 + offline classify/gates.
