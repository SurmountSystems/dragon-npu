# Wave4 review (general / effort 3)

**Date:** 2026-08-11
**Scope:** W4 product/tooling deltas only (scorecards, journal scrape, host
inventory, capture checklist, syncobj pack + wire, soft tool wires).
**Mode:** read-only review; no product edits.
**Honesty pin:** invent_pass=**0** · Pass=**No** · no `ran_on_npu` claim ·
active unlock **none** (first `C_UNLOCK=` CONSUMED per join).

Join intent SoT: `.agents/reports/wave4-join.md` (+ `wave4-*.md` siblings).

---

## Verdict

**0 open must-fix.**

W4 honesty and gate design look sound for offline work: material scorecards
force `residual_matched_claim=0` / `invent_pass=0`; product wait default stays
WAIT_CMD; research syncobj path is env-gated and fail-closed on handle 0;
tools unset `XDNA2_ALLOW_EXEC_CMD` and never invent Pass. A few **should-fix**
honesty-token / dual-API clarity items remain (not residual-root blockers).

---

## Check matrix

| Check | Result |
|-------|--------|
| invent_pass never forced to 1 | **Pass** — soft notes hard-code `invent_pass=0`; scorecard notes refuse `invent_pass=1` in unit tests |
| residual_matched_claim never true from scorecard alone | **Pass** — constructors lock `false`; format path always emits `residual_matched_claim=0` (ignores struct field for emission); bag/multi-txn tests assert no `=1` |
| Default wait still WAIT_CMD | **Pass** — `submit_and_wait_command` WAIT_CMD only; `wait_command` → `wait_cmd_ioctl`; timed path uses `_ex` but env default off → WaitCmd |
| No sticky ALLOW | **Pass** — inventory / journal / pass_unblock / post_c2 all `unset XDNA2_ALLOW_EXEC_CMD`; capture candidate never sets ALLOW |
| Fail-closed handle 0 + research env | **Pass** — `choose_lab_wait_path(on, 0)` → `InvalidArgument`; pack returns `None` for handle 0; unit tests on stub/real |
| Ioctl layout | **Pass (offline)** — `drm_syncobj_timeline_wait` size 48; ioctl const `0xc030_64ca` unit-locked; stack handle/point arrays live for ioctl call |
| Tests for new contracts | **Adequate** for offline honesty + gate; L8 live residual **not** covered (by design) |

---

## Severity-ordered findings

### Must-fix

*None.*

### Should-fix

1. **Stale soft tokens after wire landed**
   `format_syncobj_timeline_wait_pack_note` still emits both
   `product_live_still_wait_cmd_only=1` and **`not_live_wired=1`**
   (`silicon_contract.rs` ~12866–12870). After W4c, the research path **is**
   live-wired under `XDNA2_LAB_WAIT_VIA_SYNCOBJ=1` (see `direct.rs`
   `syncobj_timeline_wait_ioctl` + timed `_ex`). Default product path is still
   WAIT_CMD, so `product_live_still_wait_cmd_only=1` is defensible if read as
   “default path,” but **`not_live_wired=1` is now false** and can mislead L8
   prep / residual readers. Prefer split tokens, e.g.
   `product_default_wait_cmd=1 research_wire_present=1 not_live_proven=1
   l8_not_minted=1`.
   **Suggested failing test:**
   ```text
   note = format_syncobj_timeline_wait_pack_note(1, 0, 1000, 0);
   assert!(!residual_note_has_exact_token(&note, "not_live_wired=1"));
   // or assert research_wire_present=1 once renamed
   ```

2. **Dual wait-selection APIs (pure XRT vs product env)**
   - `select_wait_completion_path` / `should_wait_via_syncobj`: non-zero handle
     → Syncobj (**no env**). Soft note still says
     `product_live_still_wait_cmd_only=1`.
   - `choose_lab_wait_path`: requires env **and** handle.
   Callers that follow pure helpers alone may believe product auto-follows XRT
   when CREATE returns a non-zero handle. Docs on pure helpers are mostly clear;
   a single cross-link / rename (`xrt_parity_select_…` vs `lab_choose_…`) would
   reduce misuse risk.
   **Suggested failing test (contract clarity, optional):**
   ```text
   // Pure XRT parity prefers syncobj for handle≠0
   assert_eq!(select_wait_completion_path(7), WaitCompletionPath::SyncobjTimeline);
   // Product path without env still WaitCmd
   assert_eq!(choose_lab_wait_path(None, 7).unwrap(), LabWaitPath::WaitCmd);
   ```
   (Second half already covered; first half exists — keep a joint test that
   both APIs appear in one named contract.)

3. **Scorecard struct field vs soft emission**
   `MaterialExecReadinessScorecard.residual_matched_claim` is always constructed
   `false`, and `format_material_exec_readiness_note` **hard-codes**
   `residual_matched_claim=0` while only `_ = r.residual_matched_claim`. That is
   good fail-closed honesty today, but if a future edit sets the field `true`
   without updating the formatter, soft output stays honest while the struct
   lies. Prefer either delete the field, or assert `!r.residual_matched_claim`
   (debug_assert / unit) so struct and note cannot diverge.
   **Suggested failing test:**
   ```text
   // If constructors ever allow claim=true, format must still refuse =1
   // and/or panic in debug — currently all paths claim=false is the contract.
   let sc = score_material_exec_readiness(&lab_classic_bytes);
   assert!(!sc.residual_matched_claim);
   assert!(!sc.format_soft_note().contains("residual_matched_claim=1"));
   ```
   (Mostly already present; strengthen with “field always false” property over
   bag soft lines + multi sizes.)

### Nits

4. **`journal_npu_scrape.sh` soft-exits 0 on bad args**
   Documented intentional (evidence pack never hard-fails). Agents can miss
   typos. Acceptable for soft tool; optional `JOURNAL_SCRAPE_SKIP reason=bad_args`
   is already printed.

5. **`capture_from_host_candidate.sh` evidence dual-env never copies**
   Even with both operator envs and size 1688, script still exit 2 and refuses
   auto-ingest. Correct fail-closed; comment at top about
   `XDNA2_CAPTURE_ALLOW_RESIDUAL_MATCHED_CLAIM` is easy to misread as “claim
   allowed.” Wording already says PROVENANCE invent never; keep it loud.

6. **`format_syncobj_wait_path_decision_note` still `not_live_wired=1`?**
   Decision note predates pack; if it still claims not_live_wired, same should-fix
   as (1) for the decision helper surface used offline.

7. **Host inventory soft journal keywords**
   Read-only; path walks bounded. No security issue. Candidate listing ≠
   residual match is documented — good.

8. **Timed matmul always calls `submit_and_wait_command_ex`**
   Correct: env off → WaitCmd; env on requires non-zero CREATE handle or
   fails closed. Detail stamps `wait_path=` from `SubmitWaitResult` — good for
   future L8 last_apply. No sticky ALLOW interaction (submit still double-gated).

---

## Security (sticky ALLOW / research env)

| Surface | Sticky ALLOW? | Notes |
|---------|---------------|--------|
| `host_npu_stack_inventory.sh` | No | `unset XDNA2_ALLOW_EXEC_CMD` |
| `journal_npu_scrape.sh` | No | same |
| `pass_unblock_once.sh` | No | same; soft journal/host inv |
| `collect_post_c2_evidence.sh` | No | same; reads sysfs allow **status** only |
| `capture_from_host_candidate.sh` | No | refuse multi-txn; no copy as load-accepted |
| Syncobj research env | N/A | not EXEC gate; wait-only; handle 0 fail-closed |

EXEC_CMD remains feature + `XDNA2_ALLOW_EXEC_CMD=1`. Wait paths are
intentionally **not** EXEC-gated (cleanup/completion). Research syncobj does
not open submit.

---

## Correctness notes (ioctl / race)

- **Pack:** `timeout_ms==0` → `i64::MAX` (XRT forever); positive ms →
  saturating mono abs; negative now clamped to 0. Unit-tested offline.
- **Wire:** `check_wait_timeout_or_infinite_opt_in` still applies before
  syncobj ioctl (lab refuses infinite unless opt-in) — consistent with WAIT_CMD.
- **Env race:** tests use `lab_wait_via_syncobj_env_lock`; product path reads
  env once per choose. Parallel tests mutating env without the lock would flake
  (existing pattern; not new W4-only).
- **Pointer arrays:** `handle`/`point` stack locals for ioctl duration — OK.
- **Default product:** `submit_and_wait_command` does not consult research env —
  intentional isolation.

No layout bug found offline that warrants a must-fix. Live L8 still **not**
justified from wire alone (join is correct).

---

## Tests adequacy

Present and useful:

- Syncobj pack / timeout / errno classify unit tests
- `choose_lab_wait_path` env table
- Stub + real fake-fd wait_command_with_syncobj env gate
- Struct size + ioctl constant
- Material exec readiness empty / lab classic / cascade / path refuse / bag
  soft (claim=0, invent_pass=0)
- W4b bag + multi-txn soft print

Gaps (not must-fix; research/L8):

- No live residual class for SYNCOBJ_TIMELINE_WAIT EINVAL vs WAIT_CMD EINVAL
- No property that every `format_*` W4 soft note refuses invent tokens via
  shared honesty scanner (only sampled)
- Dual-API joint contract test (should-fix 2)

Do **not** invent green by weakening honesty asserts.

---

## Suggested failing tests (for real issues only — do not implement fixes here)

| ID | Intent |
|----|--------|
| S1 | Soft pack note must not claim `not_live_wired=1` after research wire exists (should-fix 1) |
| S2 | Joint: pure XRT select(handle≠0)=Syncobj **and** product choose(env off, handle≠0)=WaitCmd |
| S3 | Property: all bag soft lines from `format_material_exec_readiness_bag_soft_lines` match exact token `residual_matched_claim=0` and never `=1` (already partial) |
| S4 | Optional: `submit_and_wait_command` (non-`_ex`) detail/result never emits `wait_path=syncobj` even if env is set (env isolation) |

---

## Explicit non-claims (reviewer)

- This review does **not** grant product Pass.
- invent_pass=**0** · Pass=**No**.
- Syncobj pack/wire ≠ residual root for `wait_cmd_einval`.
- L8 not justified; no unlock mint.
- Scorecard / bag / public AOT ≠ residual match ≠ load-accepted.

---

## Summary for parent

W4 offline deltas review clean for honesty and security gates. **0 must-fix.**
Should-fix: retire or split stale `not_live_wired=1` soft token; clarify dual
wait-selection APIs; keep scorecard claim field/emission locked. Tests are
adequate for offline contracts; L8 remains operator/unlock gated.

**Report path:** `/home/hunter/Projects/ai/dragon-npu/.agents/reports/wave4-review.md`

---

## Fix note — should-fix #1 (soft tokens after W4c wire)

**Date:** 2026-08-11  
**Slice:** honesty tokens only; no unlock, no live.

**Problem:** `format_syncobj_timeline_wait_pack_note` (and the related
`format_syncobj_wait_path_decision_note`) still emitted `not_live_wired=1`
after W4c env-gated research wire landed. That token is false and misleads
L8 prep / residual readers.

**Change** (`silicon_contract.rs`):

| Removed | Added / kept |
|---------|----------------|
| `not_live_wired=1` | — |
| `product_live_still_wait_cmd_only=1` | `default_wait_cmd=1` |
| — | `research_wire_offline=1` |
| — | `not_live_proven=1` (pack already had it; decision note now has it) |
| — | `l8_not_minted=1` (same) |
| — | `invent_pass=0` always |

Unit tests assert **absence** of `not_live_wired=1` and presence of the new
tokens. Pack alone / research wire offline still ≠ Pass ≠ unlock ≠ live proven.

**Gates:**
- `cargo test --features amd-xdna2 --lib syncobj` → 6 ok
- `cargo clippy --features amd-xdna2 --lib -- -D warnings` → clean

Also closes nit 6 (decision note `not_live_wired`).
