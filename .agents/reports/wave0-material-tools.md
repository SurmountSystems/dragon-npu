# Wave W0 Track M — material tools (M5 / M6)

**Date:** 2026-08-11
**Scope:** Offline Layer A tools + material taxonomy check. No live apply. No unlock mint.
**Honesty:** invent_pass=0 · product_pass=0 · ran_on_npu=0 · Pass=No.

Companion: `.agents/reports/wave0-material-avenues.md` (M1/M3/M4).

---

## M5 — `pass_unblock_once` status + soft compare path

### Commands (operator / agent offline)

```bash
cd /home/hunter/Projects/ai/dragon-npu

# Primary status (Pass=No; unlock parse; capture bin count)
bash tools/pass_unblock_once.sh --status

# Soft external compare (first bag .bin/.ctrltext vs last_apply + soft classify)
bash tools/operator_external_capture.sh --compare
# alias surface: bash tools/operator_app_load_kit.sh b-compare

# Optional offline fingerprint pack (spent ledger; invent_pass=0)
bash tools/pass_unblock_once.sh --agent-offline
```

This implementer session had **no shell execution tool**; status/compare below are
derived from **script source + on-disk capture inventory + prior land reports**.
Operator can re-run the three commands above to refresh live stdout; outcomes
should match the inventory (bins ≥ 1, soft mismatch expected).

### Expected `--status` (from `tools/pass_unblock_once.sh` + tree)

| Field | Expected value |
|-------|----------------|
| Product Pass | **No** |
| invent_pass | **0** |
| Unlock | **none** (first `C_UNLOCK=` in `RESIDUAL.md` is `1f761bf4` **CONSUMED**) |
| Capture dir | `testdata/xdna2_cmd/external_known_good/` |
| PROVENANCE | present |
| bins | **7** (see avenues report; `*.bin`/`*.ctrltext` only) |
| blocked_on_external_bytes | **0** |
| HUMAN_NEEDED | **0** (bins > 0) |
| stock / privilege | stock path; privilege script not invented |
| `--live` | **refused** until new host-minted unlock + named offline reason |

Prior land report (whole_array): `.agents/reports/impl-feedback-larger-matmul-material.md`
recorded `--status` capture_bins=**7**, Pass=No, unlock none.

### Soft compare path (dual + triple material)

`operator_external_capture.sh --compare`:

1. Picks **first** non-empty allowed bag `*.bin` / `*.ctrltext` (find order; typically `working_stack_ctrltext.bin` = vector **300 B** primary).
2. Requires last_apply JSON (fixture or `XDNA2_LAB_LAST_APPLY_JSON`).
3. Sets `XDNA2_LAB_EXTERNAL_CTRLTEXT` + `XDNA2_LAB_PRINT_C2_CMDBUF_RESEARCH=1`.
4. Runs `classify_last_apply_residual.sh` (soft).
5. Always: invent_pass=0; product_known_good=0; match ≠ residual closed ≠ Pass ≠ second unlock.

**Soft mismatch expected** for all present public peels vs lab classic **1688**:

| External peel | len | size_class (length-only) | vs lab 1688 |
|---------------|-----|--------------------------|-------------|
| working_stack / vector | 300 | `small_public_npu2` | **mismatch** |
| single_core matmul | 660 | `small_public_npu2` | **mismatch** |
| whole_array matmul | 2592 | `other` | **mismatch** |

To soft-compare a non-primary peel without overwriting primary:

```bash
export XDNA2_LAB_PRINT_C2_CMDBUF_RESEARCH=1
export XDNA2_LAB_EXTERNAL_CTRLTEXT=/home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/external_known_good/mlir_aie_single_core_matmul_npu2.ctrltext.bin
bash tools/classify_last_apply_residual.sh testdata/xdna2_cmd/last_apply.json
# or whole_array ctrltext path similarly
```

### Material residual inventory (pure API; offline)

From `silicon_contract.rs` (`classify_material_residual_match`):

| Input | kind | size_class | residual_matched_claim |
|-------|------|------------|------------------------|
| len=300 | `public_npu2_small_research_only` | `small_public_npu2` | **false** |
| len=660 | `public_npu2_small_research_only` | `small_public_npu2` | **false** |
| len=2592 | `unknown` (detail `other_size_not_residual_match_claim`) | `other` | **false** |
| tag `mlir_aie` + any len | `public_npu2_small_research_only` | class from len | **false** |
| load-accepted / residual-matched tag | `load_accepted_criteria` only | — | **false** offline |

Checklist SSoT: `format_material_residual_match_checklist` always
`public_aot_neq_residual_identity=1 invent_pass=0 product_pass_requires_live_decide=1`.

### Prior fingerprint report staleness note

`.agents/reports/pass-unblock-fingerprint-once.md` still shows capture bins=**0**
from an earlier fingerprint run (2026-08-10). **Superseded by on-disk reality**
and whole_array land report (bins=7). Do not re-queue empty reconfirm from that
stale fingerprint row.

---

## M6 — Material taxonomy gaps

### What exists today

`CtrltextSizeClass`:

| Variant | Rule |
|---------|------|
| Empty | len 0 |
| MladfClassic1688 | len 1688 or staged mladf profile |
| SmallPublicNpu2 | `1..=1024` (`CTRLTEXT_SMALL_PUBLIC_NPU2_MAX_BYTES`) — covers 300 + 660 |
| Other | everything else non-empty non-1688 (covers **2592** whole_array) |

`MaterialResidualMatchKind`: load-accepted criteria · public-npu2-small research · lab classic · flexml spent · unknown.

Unit lock already expects len=1025 → Other / Unknown / residual_matched_claim=false
(`material_residual_match_inventory_public_vs_load_accepted`).

### Decision this wave: **no product code change**

| Candidate gap | Verdict |
|---------------|---------|
| whole_array **2592** needs new enum | **No.** Honest path is `Other` + not residual match. Tag `mlir_aie` still research-only. |
| New `PublicNpu2Medium` size class | **Optional future polish only** — not required for honesty; would still set residual_matched_claim=0. Not landed this wave (no invent residual_matched). |
| residual_matched offline for public AOT | **Forbidden.** Load-accepted requires live ERT Completed + OFM allclose + real_ioctl via `decide_ran_on_npu`. |

No NEW file sizes outside existing classes that demanded a residual_matched claim.
**invent_pass=0.** Taxonomy sufficient for offline inventory.

### Optional residual note (docs only; not implemented)

If residual prose wants a named tag for whole_array without claiming match:

- e.g. research tag **`public-npu2-whole-array-2592`** or size-class prose “public AOT larger-than-small (other)”
- Still RESEARCH ONLY; still ≠ Pass / unlock alone
- Prefer residual pin update when operator asks; do not mint unlock from it

---

## Frozen / not done

- No live NPU / no `--live` under CONSUMED
- No re-live spent flexml
- No invent bins
- No residual_matched_claim=1 offline
- No empty reconfirm mill

---

## Footer

```
invent_pass=0
product_pass=0
ran_on_npu=0
Pass=No
C_UNLOCK_active=none
capture_bins_expected=7
blocked_on_external_bytes=0
soft_mismatch_expected=1
taxonomy_code_change=0
residual_matched_claim=0
shell_status_this_session=documented_from_inventory_not_reexecuted
```
