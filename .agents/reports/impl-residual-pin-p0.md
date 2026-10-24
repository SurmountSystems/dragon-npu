# Residual honesty pin after P0 C1–C3

**Date:** 2026-08-10
**Repo:** `/home/hunter/Projects/ai/dragon-npu`
**Slice:** residual / AGENTS / plan status only (no Rust)

---

## 1. What changed

| File | Change |
|------|--------|
| `RESIDUAL.md` | Plain-status rows strengthened for P0 C1+C2+C3 with exact API names; honesty that offline validate/FP/split ≠ C-ready ≠ Pass ≠ unlock; unlock still none (do not mint from pin); capture still bins may be 0; next = P1 C4+C5 or human material Step 1; pointers to impl-p0 + sequence plan + this pin report. Honesty block mid-file aligned. History kept. |
| `AGENTS.md` | Prepended **Latest Turn** residual honesty pin after P0; prior S4–S5 turn demoted to Prior Turn (not deleted). |
| `.agents/plans/plan-pass-unblock-sequence.md` | Status line: P0 landed; next = P1 C4+C5. |
| `.agents/plans/plan-pass-unblock-creative-avenues.md` | Suggested next implement notes P0 landed; points next work at P1 C4+C5; keeps historical C1–C3 text as audit (done). |

No product Rust edits. No cargo. No unlock mint. No live.

---

## 2. SSoT statements now locked (plain English)

1. **Named offline delta:** P0 C1+C2+C3 landed:
   - C1: `validate_aiebu_elf_ctrltext_structure` + `format_c1_validator_note`
   - C2: `StructuralLabFingerprint` / `structural_fingerprint_from_lab_detail` + format note
   - C3: `ResidualClassifierSplit` / `classify_residual_split` + format note (`wait_cmd_einval` ≠ `wait_timeout`)
2. **Honesty:** still **no** Pass; invent_pass=0; **no** `ran_on_npu`; offline validate/FP/split **≠** C-ready.
3. **Unlock:** **none active** — all listed `C_UNLOCK` CONSUMED; **do not mint** from this pin alone; do not reopen.
4. **Capture:** still bins may be 0 under `testdata/xdna2_cmd/external_known_good/`.
5. **Next:** P1 C4+C5 agent offline; or human material Step 1; live only under **new** unlock + named reason. No empty reconfirm mill.
6. **Reports / plans:** `.agents/reports/impl-p0-c1-c3.md`, `.agents/plans/plan-pass-unblock-sequence.md`, this file.

---

## 3. Honesty confirmation

| Claim | Status |
|-------|--------|
| Product Pass | **No** |
| invent_pass | **0** |
| `ran_on_npu` | **No** |
| Active unlock | **None** (`C_UNLOCK=2ae69036` + priors CONSUMED) |
| Unlock minted by this pin | **No** |
| P0 alone = C-ready / Pass | **No** |
| Empty reconfirm mill | Still **frozen** |
| Capture known-good bins | Still may be **0** |

---

## 4. Out of scope this pin

- Re-running cargo / gates
- Live CONFIG_CU or L3
- Minting a new `C_UNLOCK`
- Inventing capture bytes
- Implementing P1 C4+C5 (named next slice only)
