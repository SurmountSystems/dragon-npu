# HITL C recipe — APP_LOAD residual (aiebu ELF)

**Purpose:** One canonical operator sequence for a **single** Milestone C Pass
apply against the locked **APP_LOAD_PDI_FAIL** residual.

**Status:** `C_UNLOCK=b17b0119` **CONSUMED** (operator C2 2026-07-18). Empty
empty reconfirm stamps and empty-folder implement loops are **frozen**
(see `AGENTS.md` §6–9 / `RESIDUAL.md`
LIVE). Further C needs a **new host-minted unlock** after a **named offline
delta** (construction/byte evidence), not another reconfirm stamp. Offline green
≠ Pass ≠ `ran_on_npu`.

Post-C2 offline under CONSUMED = `tools/c3_post_c2_classify.sh` (C3_SKIP without new evidence) +
`tools/c4_pass_check.sh` (C4_SKIP without PassCandidate snapshot).
**Offline C4 check ≠ invent product Pass** / ≠ `ran_on_npu` claim.
Live L3 sandwich is **future-unlock-only** (not under CONSUMED `b17b0119`).


| Claim | Truth |
|-------|--------|
| One L3 sandwich + `hitl_c` under `b17b0119` | **CONSUMED** — do not re-run; needs **new** unlock |
| Permanent open season / more C without **new agent unlock** | **No** (evidence may justify unlock; never auto-unlocks) |
| Silicon Pass / `ran_on_npu` | **No** until OFM allclose + ERT Completed |
| Leave L3 open / set permanent userspace ALLOW | **No** — always restore L3=`exec_cmd_allow=0`; never leave `XDNA2_ALLOW_EXEC_CMD=1` sticky |
| Agent completes C2 without sudo when L3 closed | **No** — fail-closed exit 1 |

Soft residual class / `CLASSIFY_SOFT_OK` / `app_load_checklist=…` /
`OFFLINE_GATES_OK` / `C3_SKIP` / **`C4_SKIP`** still **≠** Pass and never
substitute for this stamp. Offline `c4_pass_check` is snapshot audit only.

single source of truth residual: [RESIDUAL.md](../RESIDUAL.md) · B/C overview: [HITL_BC.md](./HITL_BC.md) ·
contract: [XDNA2_XRT_CONTRACT_MLADF.md](./XDNA2_XRT_CONTRACT_MLADF.md).

---

## Unlock policy (hard)

C runs **only if the agent explicitly says ready** with a **new host-minted**
unlock stamp. `C_UNLOCK=b17b0119` is **CONSUMED**. **Further applies require a
new unlock** after real offline delta — not open season, not empty gate-only
hex. New last_apply/journal or construction/byte evidence may **justify** that
unlock; evidence **never auto-unlocks** C.

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

## Exact apply (CONSUMED — `C_UNLOCK=b17b0119`)

> **Unlock CONSUMED.** Do **not** open execute-allow under `b17b0119`. Record-only /
> offline path only. Live apply under this unlock is **closed**. Offline path =
> `c3` / `c4` / `run_offline_gates` only. **Do not** mint empty maintain stamps.

**Historical operator C2 (already ran).** Agent does **not** re-run this under `b17b0119`. Unlock **≠** Pass.

### Offline path under CONSUMED (no L3 / no EXEC)

```bash
cd ~/Projects/ai/dragon-npu
export PATH="/usr/bin:${HOME}/.local/bin:${PATH}"
# Soft CONSUMED NOTE + optional gates + preflight + prints HISTORICAL sandwich text:
bash tools/hitl_c2_once.sh
# sandwich text only (record-only; does not open L3):
bash tools/hitl_c2_once.sh --print-only
# Offline residual / Pass audit only:
bash tools/c3_post_c2_classify.sh
bash tools/c4_pass_check.sh
bash tools/run_offline_gates.sh
# Fail-closed when L3 closed (expect exit 1) — refuse demo, not apply:
# bash tools/hitl_c2_once.sh --apply-if-l3-open
# bash tools/hitl_c_pass_apply.sh --assume-l3-already-open
```

### HISTORICAL / CONSUMED sandwich (record-only — **all L3/apply lines commented**)

Do **not** paste-run under `b17b0119`. Kept for residual archaeology only.

```bash
cd ~/Projects/ai/dragon-npu
# bash: export XDNA2_LAB_BO_DIAG=1
# fish: set -gx XDNA2_LAB_BO_DIAG 1

# HISTORICAL record-only — C_UNLOCK=b17b0119 CONSUMED; do not open L3 under this stamp
# sudo sh -c 'echo 1 > /sys/module/amdxdna/parameters/exec_cmd_allow'
# bash tools/hitl_c_pass_apply.sh --assume-l3-already-open
# or: # bash tools/hitl_c2_once.sh --apply-if-l3-open
# sudo sh -c 'echo 0 > /sys/module/amdxdna/parameters/exec_cmd_allow'
# cat /sys/module/amdxdna/parameters/exec_cmd_allow   # must be 0 or N
```

What hitl_c enforces (default path; **Future unlock only** — not active under CONSUMED):

- Forces **`lab_instr.elf`** (refuses non-ELF magic; no `--blob` override)
- Runs **`silicon_contract_check.sh --require-pass-assets`** (A9) — hard-fail
  unless skip/force (see flags below)
- Then **`silicon_lab_preflight.sh --require-pass-assets`**
- Single-shot silicon-mladf apply; post-check `exec_cmd_allow=0` (exit **5** if still open)

### Future unlock only (live L3 apply — **not** under `b17b0119`)

Live apply requires a **new** agent unlock stamp. Until then, leave all lines commented.

```bash
# FUTURE UNLOCK ONLY — requires new C_UNLOCK=… (not b17b0119 CONSUMED)
# bash tools/hitl_c_pass_apply.sh
# bash tools/hitl_c_pass_apply.sh --assume-l3-already-open
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

## Success criteria (true Pass only)

Claim `ran_on_npu` / Pass **only** when **all** hold:

1. Real ioctl path (not stub)
2. ERT **Completed** (not Error / timeout)
3. OFM **allclose** vs non-zero golden (`decide_ran_on_npu` / product flags)
4. Bench / hitl_c exit **0** with `ran_on_npu=true`

Never claim Pass from envelope accept alone, zero golden, offline
`CONTRACT_OK`, or “gates green so try C.”

### If not Pass — paste for agent

- `status`, `aiebu_*`, `inst_size_mode=`, `start_npu_size=`, `layout=`, `ert_state`, `ofm_stats`, `fw_status_hint`
- `RESIDUAL_ENV` banner line from hitl_c
- hitl_c / lab_exec **exit code** (and whether L3 was left open → 5)
- Optional: `testdata/xdna2_cmd/last_apply.json` or `/tmp/dragon_npu_last_apply_*.json`
- Journal:  
  `journalctl -k -b --no-pager | rg -i 'Failed cmd|Status 0x|APP_LOAD|amdxdna' | tail -20`

Always restore L3 after any **live** apply (Future unlock only — not under CONSUMED `b17b0119`):

```bash
# FUTURE UNLOCK / live apply only — under CONSUMED, L3 should already be N:
# sudo sh -c 'echo 0 > /sys/module/amdxdna/parameters/exec_cmd_allow'
cat /sys/module/amdxdna/parameters/exec_cmd_allow   # must be 0 or N
```

### Post-C2 offline (operator or agent — no L3)

```bash
# C3 pipeline (soft classify + body-first evidence; ≠ C-ready / ≠ Pass):
#   fingerprint == locked baseline → C3_SKIP (baseline_locked_app_load /
#     same_body_as_baseline; path alone / same-body copy ≠ evidence)
#   fingerprint differs → C3_EVIDENCE=1 even if residual still APP_LOAD
#     (shape_still_locked_app_load=1 annotation only — never force skip)
bash tools/c3_post_c2_classify.sh
# hard research mode (exit 2 if no new C2 body):
# bash tools/c3_post_c2_classify.sh --require-new
# raw classify only:
# bash tools/classify_last_apply_residual.sh

# C4 Pass-evidence audit (soft; baseline → C4_SKIP; ≠ invent product Pass):
bash tools/c4_pass_check.sh
# hard research mode (exit 2 if not PassCandidate):
# bash tools/c4_pass_check.sh --require-pass

# Offline gates still ≠ C-ready:
bash tools/run_offline_gates.sh
```

**Honesty:** Evidence = **fingerprint/body differs** from the locked baseline
fixture — not path or temp alone. Same-body copy under another path → `C3_SKIP`.
C3 without a **new** last_apply body (still only the 2026-07-16 locked fixture)
→ `C3_SKIP` ≠ silicon progress. Do **not** re-open the closed APP_LOAD signal
matrix on the same fixture. **`C4_SKIP`** on the same fixture is expected until
a real Pass snapshot (`json_ran_on_npu=true` + ERT Completed + non-zero OFM).
Offline C4 audit **≠** invent product Pass (live `decide_ran_on_npu` still owns
product flag).

Then share classify output + last_apply + journal with agent for **C3** (fail
residual) or **C4** (only if OFM allclose + ERT Completed + `ran_on_npu=true`).

---

## Highest-value next (after this recipe)

1. **Construction / byte research offline** (named question — how a working
   public stack builds loadable bytes; DynamicDispatch / XRT first → byte-compare
   to our failing image). Empty gate-only maintain stamps are **frozen**.
   `C_UNLOCK=b17b0119` **CONSUMED** — **no** live sandwich under it. Offline green
   alone still does **not** equal Pass / C-ready / `ran_on_npu`. Further live C needs a
   **new agent unlock** (evidence may justify; never auto-unlocks).
2. **Offline residual / Pass audit (CONSUMED path):** `bash tools/c3_post_c2_classify.sh`
   + `bash tools/c4_pass_check.sh` + `run_offline_gates.sh` → agent **C3** when
   fingerprint differs (`C3_EVIDENCE=1`, even if still APP_LOAD) / **C4** only if
   `C4_PASS_CANDIDATE` (still product Pass = live decide). Same body as baseline →
   `C3_SKIP` + `C4_SKIP` (path alone ≠ evidence) — readiness only, not silicon
   progress. **C2 is not pending** under `b17b0119` (already completed / CONSUMED).
3. Optional capture-instr = **logs only** (public true post-patch path closed).

**Under CONSUMED offline path only:** `c3_post_c2_classify.sh` + `c4_pass_check.sh` +
`run_offline_gates.sh`. Live L3 sandwich only after **new** agent unlock (Future unlock
block above — not `b17b0119`).
