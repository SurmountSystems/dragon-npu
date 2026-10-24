# Implement report — plan complete product silicon S0–S2b

**Date:** 2026-08-10
**Repo:** `/home/hunter/Projects/ai/dragon-npu` (branch `rust`)
**Scope:** Approved plan S0 → S1 → S2 → S2b only. **Stop for human S3 / S3b.**

---

## NEW C_UNLOCK

| Field | Value |
|-------|--------|
| **Active** | **`C_UNLOCK=11bc9016`** |
| Mint | `head -c 4 /dev/urandom \| xxd -p` (not historical, not hand-typed) |
| Not | `3a70fabf`, `b17b0119` |
| First `C_UNLOCK=` line | Unlock row in `RESIDUAL.md` — **no** word CONSUMED on that line |
| Prior | `C_UNLOCK=3a70fabf` CONSUMED; `C_UNLOCK=b17b0119` CONSUMED historical |
| Named offline reason | progressive last_apply + timeout honesty + ≥120s recipe after `3a70fabf` timeout shot |

**Unlock ≠ Pass ≠ `ran_on_npu`.** One human sandwich only. Do not mint a second unlock.

---

## Files touched

| Path | Change |
|------|--------|
| `RESIDUAL.md` | Top plain status + LIVE bookkeeping for plan active + unlock `11bc9016` |
| `AGENTS.md` | New Latest Turn for S0–S2b unlock mint |
| `tools/operator_app_load_kit.sh` | Overview unlock strings |
| `tools/hitl_c2_once.sh` | Header + sandwich `--timeout-s=120` |
| `tools/hitl_config_cu_once.sh` | Header + sandwich Unlock lines RESIDUAL-driven |
| `tools/hitl_c_pass_apply.sh` | Header comments + WARN/footer stamp wording |
| `docs/HITL_C_APP_LOAD_RECIPE.md` | Status, unlock policy, **active** Exact apply section + historical `3a70fabf` |
| `docs/HITL_BC.md` | Active unlock rows → `11bc9016` |
| `docs/CONSTRUCTION_MAP_APP_LOAD.md` | Live apply unlock lines |
| `.agents/plans/plan-complete-product-silicon.md` | Workspace plan mirror + implement status |
| `.agents/reports/plan-complete-product-silicon-s0.md` | S0 report |
| `.agents/reports/impl-complete-product-silicon-s0-s2b.md` | This file |

No Rust product edits. No git stage/commit.

---

## print-only excerpt (active vs not)

```text
$ bash tools/hitl_c2_once.sh --print-only
NOTE: C_UNLOCK=11bc9016 one-shot (active); restore L3 (exec_cmd_allow=0); ≠ ran_on_npu until Pass
…
# C_UNLOCK=11bc9016 one-shot (active); restore L3; ≠ ran_on_npu until Pass
# Outer ≥120s for CONFIG_CU silicon (default 30s killed prior shot)
sudo sh -c 'echo 1 > /sys/module/amdxdna/parameters/exec_cmd_allow'
bash tools/hitl_c_pass_apply.sh --assume-l3-already-open --timeout-s=120
sudo sh -c 'echo 0 > /sys/module/amdxdna/parameters/exec_cmd_allow'
```

**Not** all `# (record only)`. Executable sandwich lines present.

First residual match:

```text
$ grep -m1 -E 'C_UNLOCK=' RESIDUAL.md
| **Unlock** | **`C_UNLOCK=11bc9016`** active one-shot …  (no CONSUMED on line)
```

---

## S2 — Offline prep summary

### config-cu (preferred)

- Ran `bash tools/operator_app_load_kit.sh config-cu`
- Active unlock NOTE: `C_UNLOCK=11bc9016`
- PDI research dump OK (~283840 B) under `/tmp/dragon_npu_research_pdi_*.bin`
- Preflight: ELF_OK `lab_instr.elf` 32264 B; PDI_OK; LAST_APPLY pin under `/tmp`
- Host: `exec_cmd_allow=missing` (stock) → **stock sandwich, no sudo L3**
- Sandwich includes **`--timeout-s=120`** and filled `XDNA2_LAB_PDI_BLOB` / LAST_APPLY paths

### silicon_lab assets (present; no restage needed)

| File | Size |
|------|------|
| `lab_instr.elf` | 32264 |
| `lab_instr.bin` | 1688 |
| `wts.bin` | 174080 |
| `lab_ctrl.bin` | 29664 |
| `lab_ctrl_meta.json` | present |
| `ofm_golden.bin` | 512 |
| `ifm.bin` | 2304 |

### bash -n

`operator_app_load_kit.sh`, `hitl_c2_once.sh`, `hitl_config_cu_once.sh`, `hitl_c_pass_apply.sh` — all OK.

---

## S2b — Capture track summary

```text
$ bash tools/operator_app_load_kit.sh b-status
dir=…/testdata/xdna2_cmd/external_known_good
PROVENANCE=present
BINS=0  blocked_on_external_bytes=1
Honesty: invent_pass=0 not_c_ready=1 not_ran_on_npu=1 product_known_good=0
```

**b-steps:** install path → fill PROVENANCE capture table → `--compare`.
**Never invent bytes.** Park S3b if no public working stack is available; Track A (S3) still proceeds alone.

---

## Operator next

### S3 — Live sandwich (required for last_apply / Pass path)

**Preferred one command (paths filled, stock-aware, ≥120s):**

```bash
cd ~/Projects/ai/dragon-npu
export PATH="/usr/bin:${HOME}/.local/bin:${PATH}"
bash tools/operator_app_load_kit.sh config-cu
```

Then paste the printed **bash** or **fish** block from that output into the **same interactive shell** (re-export env; prep only sets env in the prep process).

**Shape (stock host example — no sudo; real paths come from config-cu print):**

```bash
cd ~/Projects/ai/dragon-npu
export PATH="/usr/bin:${HOME}/.local/bin:${PATH}"
unset XDNA2_ALLOW_EXEC_CMD
export XDNA2_LAB_PDI_BLOB=/tmp/dragon_npu_research_pdi_<from_config_cu>.bin
export XDNA2_LAB_CU_FUNC=0
export XDNA2_LAB_LAST_APPLY_JSON=/tmp/dragon_npu_last_apply_config_cu_<from_config_cu>.json
export XDNA2_LAB_BO_DIAG=1
bash tools/hitl_c_pass_apply.sh --assume-l3-already-open --timeout-s=120
bash tools/collect_post_c2_evidence.sh "$XDNA2_LAB_LAST_APPLY_JSON"
```

After shot: share hitl_c rc + last_apply path + collect output with agent for S4.

### S3b — Capture install (optional parallel)

```bash
bash tools/operator_app_load_kit.sh b-steps
bash tools/operator_app_load_kit.sh b-install /path/to/working_stack_ctrltext.bin
# edit testdata/xdna2_cmd/external_known_good/PROVENANCE.md capture table
bash tools/operator_app_load_kit.sh b-compare
```

---

## Honesty

| Claim | Truth |
|-------|--------|
| Product Pass | **No** |
| `ran_on_npu` | **No** |
| invent_pass | **0** |
| Agent live apply / L3 / ALLOW | **Did not** |
| Capture bytes invented | **No** (still BINS=0) |
| Unlock = Pass | **No** |
| Reopened 3a70fabf / b17b0119 | **No** |
| Second unlock minted | **No** |
| Empty offline-gates mill | **No** (targeted verify only) |

---

## Done criteria

- [x] S0 baseline residual + kit + plan mirror + S0 report
- [x] S1 unlock pin across residual, recipe, HITL_BC, construction map, hitl helpers, AGENTS
- [x] S2 config-cu prep + assets + print-only active executable sandwich ≥120s
- [x] S2b b-status / b-steps; operator note; no invent bytes
- [ ] S3 / S3b human
- [ ] S4+ agent after evidence

**Agent stops here.** Resume only after new last_apply / journal package and/or real capture bytes.
