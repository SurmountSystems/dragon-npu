# HITL C recipe — APP_LOAD residual (aiebu ELF)

**Purpose:** One canonical operator sequence for a **single** Milestone C Pass
apply against the locked **APP_LOAD_PDI_FAIL** residual.

**Status:** **Unlocked once** under **`C_UNLOCK=3a70fabf`** for **one**
CONFIG_CU-enabled apply (optional `XDNA2_LAB_PDI_BLOB` path). Prior
`C_UNLOCK=b17b0119` **CONSUMED** (operator C2 2026-07-18) — do **not** reopen
under it. Empty reconfirm stamps and empty-folder implement loops are **frozen**
(see `AGENTS.md` §6–9 / `RESIDUAL.md`). Offline green ≠ Pass ≠ `ran_on_npu`.
Unlock **≠** Pass **≠** permanent open season.

Post-apply offline = `tools/c3_post_c2_classify.sh` + `tools/c4_pass_check.sh` +
`tools/p6_true_pass_check.sh` + `tools/run_offline_gates.sh` (or one-shot
`tools/p5_post_c2_interpret.sh`). **Offline C4/P6 ≠ invent product Pass** /
≠ `ran_on_npu` claim.


| Claim | Truth |
|-------|--------|
| One L3 sandwich + `hitl_c` under **`C_UNLOCK=3a70fabf`** | **Active one-shot** — CONFIG_CU + PDI path; agent unlock only |
| One L3 sandwich under `b17b0119` | **CONSUMED** — historical; do **not** re-run under it |
| Permanent open season / more C without **new agent unlock** after this shot | **No** |
| Silicon Pass / `ran_on_npu` | **No** until OFM allclose + ERT Completed |
| Leave L3 open / set permanent userspace ALLOW | **No** — always restore L3=`exec_cmd_allow=0`; never leave `XDNA2_ALLOW_EXEC_CMD=1` sticky |
| Agent completes live apply without operator sudo when L3 closed | **No** — fail-closed exit 1 |

Soft residual class / `CLASSIFY_SOFT_OK` / `app_load_checklist=…` /
`OFFLINE_GATES_OK` / `C3_SKIP` / **`C4_SKIP`** still **≠** Pass and never
substitute for this stamp. Offline `c4_pass_check` is snapshot audit only.

single source of truth residual: [RESIDUAL.md](../RESIDUAL.md) · B/C overview: [HITL_BC.md](./HITL_BC.md) ·
contract: [XDNA2_XRT_CONTRACT_MLADF.md](./XDNA2_XRT_CONTRACT_MLADF.md).

---

## Unlock policy (hard)

C runs **only if the agent explicitly says ready** with a **host-minted** unlock
stamp. **Active:** `C_UNLOCK=3a70fabf` one-shot for **one** CONFIG_CU + PDI
sandwich. **Historical:** `C_UNLOCK=b17b0119` is **CONSUMED** — do not reopen
under it. After `3a70fabf` is used, further applies need a **new** unlock — not
open season, not empty gate-only hex. Evidence may **justify** unlock; evidence
**never auto-unlocks** C.

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

## Exact apply — **active** `C_UNLOCK=3a70fabf` (CONFIG_CU one-shot)

> **Agent unlock active once.** Permission for **one** L3 sandwich that uses the
> optional CONFIG_CU path (`XDNA2_LAB_PDI_BLOB`). Unlock **≠** Pass **≠**
> `ran_on_npu` **≠** permanent open season. Always **restore L3** after apply.
> Prior `b17b0119` remains **CONSUMED** (historical section below) — do not
> reopen under it. Agent does **not** open L3 or run live apply.

### CONFIG_CU pre-step (before sandwich)

```bash
cd ~/Projects/ai/dragon-npu
export PATH="/usr/bin:${HOME}/.local/bin:${PATH}"

# Dump opaque host AIE2P PDI + print env (default off until you export):
bash tools/operator_app_load_kit.sh a-prep
# or: bash tools/dump_xclbin_pdi.sh --print-env

# bash — export path printed by a-prep (example shape only):
# export XDNA2_LAB_PDI_BLOB=/path/from/a-prep/pdi.bin
# export XDNA2_LAB_CU_FUNC=0   # optional; default 0
# fish:
# set -gx XDNA2_LAB_PDI_BLOB /path/from/a-prep/pdi.bin
# set -gx XDNA2_LAB_CU_FUNC 0
```

Wrong overlay may still journal `0x3000003`. CONFIG_CU attach alone **≠** Pass.

### Offline prep + printable sandwich (no L3 from agent)

```bash
cd ~/Projects/ai/dragon-npu
export PATH="/usr/bin:${HOME}/.local/bin:${PATH}"
# Soft active unlock NOTE + optional gates + preflight + **executable** L3 lines:
bash tools/hitl_c2_once.sh
# sandwich text only (does not open L3):
bash tools/hitl_c2_once.sh --print-only
# Offline residual / Pass audit (any time; ≠ unlock):
bash tools/c3_post_c2_classify.sh
bash tools/c4_pass_check.sh
bash tools/run_offline_gates.sh
# Fail-closed when L3 closed (expect exit 1) — refuse demo, not apply:
# bash tools/hitl_c2_once.sh --apply-if-l3-open
# bash tools/hitl_c_pass_apply.sh --assume-l3-already-open
```

### Live L3 sandwich (operator only — under `C_UNLOCK=3a70fabf`)

Keep `XDNA2_LAB_PDI_BLOB` set in the same shell that runs apply. Prefer the
exact lines from `hitl_c2_once.sh --print-only`. Shape:

```bash
cd ~/Projects/ai/dragon-npu
export PATH="/usr/bin:${HOME}/.local/bin:${PATH}"
# optional: export XDNA2_LAB_BO_DIAG=1
# never leave sticky: unset XDNA2_ALLOW_EXEC_CMD
# export XDNA2_LAB_PDI_BLOB=…   # from a-prep
# export XDNA2_LAB_CU_FUNC=0    # optional

# C_UNLOCK=3a70fabf one-shot (active); restore L3; ≠ ran_on_npu until Pass
sudo sh -c 'echo 1 > /sys/module/amdxdna/parameters/exec_cmd_allow'
bash tools/hitl_c_pass_apply.sh --assume-l3-already-open
sudo sh -c 'echo 0 > /sys/module/amdxdna/parameters/exec_cmd_allow'
cat /sys/module/amdxdna/parameters/exec_cmd_allow   # must be 0 or N
```

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

Always restore L3 after any **live** apply (active `C_UNLOCK=3a70fabf`; not under CONSUMED `b17b0119`):

```bash
# After live apply under C_UNLOCK=3a70fabf — always verify restore:
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
product flag). Mark `C_UNLOCK=3a70fabf` used in RESIDUAL **only** after a real
sandwich produced new last_apply — never from locked-fixture soft reconfirm.

Then share classify output + last_apply + journal with agent for **C3** (fail
residual) or **C4** (only if OFM allclose + ERT Completed + `ran_on_npu=true`).

---

## Highest-value next (after this recipe)

1. **Operator: one CONFIG_CU live sandwich under `C_UNLOCK=3a70fabf`** —
   `a-prep` → export `XDNA2_LAB_PDI_BLOB` → L3 sandwich → restore L3 →
   `bash tools/p5_post_c2_interpret.sh` → `bash tools/p6_true_pass_check.sh` →
   share last_apply + journal. Unlock **≠** Pass. Prior `b17b0119` already used
   (historical only).
2. **Post-apply offline residual / Pass audit (P5+P6):** `p5_post_c2_interpret.sh`
   or `c3_post_c2_classify` + `c4_pass_check` + `p6_true_pass_check` +
   `run_offline_gates` → agent **C3** when fingerprint differs / **C4** only if
   `C4_PASS_CANDIDATE` (product Pass = live decide still; P6 still
   `product_pass_requires_live_decide=1`). Same body as baseline → `C3_SKIP` +
   `C4_SKIP` + `P6_SKIP`. If still `0x3000003` with `config_cu=1` → wrong PDI /
   multi-CU / ctrltext residual (named next delta only if justified).
3. **Capture path (optional, parallel):** place working-stack ctrltext under
   `testdata/xdna2_cmd/external_known_good/` + PROVENANCE → soft compare.
   Empty gate-only maintain stamps remain **frozen**. Offline green alone still
   **≠** Pass / C-ready / `ran_on_npu`. After this one-shot is used, further live
   C needs a **new** agent unlock.

**Active live path:** CONFIG_CU section above under `C_UNLOCK=3a70fabf`.  
**Always after live:** restore L3 + offline classify/gates.
