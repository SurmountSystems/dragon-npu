# grok-build: `plan_panel_click_clarify_revise_quit_buttons` CI fail map

**Workspace explored:** `/home/hunter/Projects/surmount/grok-build` (read-only)
**Session report path:** this file (dragon-npu session workspace)
**Safety:** pure TUI unit test in `xai-grok-pager`. No sudo, no kernel, no host mutation, no brick risk.
**Date:** 2026-08-10
**Runtime note:** explore agent had no shell; single-test command below is the verify recipe (not re-run here). On-tree product + test already match the 2026-08-09 decisive-Revise contract.

---

## 1. Test definition

| Item | Value |
|------|--------|
| **Crate** | `xai-grok-pager` |
| **Module path** | `app::agent_view::plan::approve_plan_flush_tests` |
| **Function** | `plan_panel_click_clarify_revise_quit_buttons` |
| **File** | `/home/hunter/Projects/surmount/grok-build/crates/codegen/xai-grok-pager/src/app/agent_view/plan.rs` |
| **Lines** | **4358–4464** (module starts ~1528) |
| **Helpers** | `install_plan_approval` (~1542), `parse_outcome` (~1706), `make_agent` |

### What the test does (three sequential blocks)

1. **Clarify** (`questions_button_area` click)
   - Expect: park stays, `focus = Prompt`, `prompt_intent = Questions` (focus-only).

2. **Revise** (`send_button_area` click)
   - Expect: `plan_approval_view.is_none()`, `line_viewer.is_none()`, ACP JSON `outcome == "cancelled"`.
   - Named contract (2026-08-09): **decisive** `request_plan_revise`, not focus-only.

3. **Quit** (`abandon_button_area` click)
   - Expect: park cleared, ACP `outcome == "abandoned"`.

Each block: `install_plan_approval` → `show_plan_preview()` → inject hit `Rect` + `feedback_active` + `last_modal_area` → `handle_line_viewer_mouse` Left Down.

### Why ~0.088s failure is an assert, not a hang

- No network, no sleep, no PTY.
- Failure is panic/assert on first mismatched branch.
- **Most documented failure mode** (report `impl-plan-panel-revise-test-2026-08-09.md`): Revise path already decisive, but an **older test** still did:

```rust
let pav = agent.plan_approval_view.as_ref().unwrap(); // None after revise → panic
assert_eq!(pav.focus, PlanApprovalFocus::Prompt);
assert_eq!(pav.prompt_intent, PlanPromptIntent::Revise);
```

That yields exactly: fast fail, `unwrap()` on `None` (historically ~line 4307; line numbers moved; Revise block is now ~4418–4430).

**Alternate mismatch** (test already decisive, product still focus-only):

- `plan_approval_view.is_none()` fails (park still Some), or
- `parse_outcome` / `try_recv` fails (no ACP message).

---

## 2. Product click path (Clarify / Revise / Quit)

| Role | Absolute path | What matters |
|------|----------------|--------------|
| **Mouse dispatch (panel)** | `.../app/agent_view/viewer.rs` **~465–559** | `handle_line_viewer_mouse`: hit-test footer areas |
| **Revise submit** | `.../app/agent_view/plan.rs` **`request_plan_revise` ~694** | freeform from composer → `send_plan_feedback` |
| **Revise body** | `plan.rs` **`send_plan_feedback` ~721** | take park → `send_cancelled` → clear `line_viewer` → toast |
| **Clarify focus** | `plan.rs` **`focus_plan_prompt` ~865** | set Prompt + intent only |
| **Quit** | `plan.rs` **`abandon_plan` ~615** | `send_abandoned` → clear park/panel |
| **Soft-park sibling** | `plan.rs` **`handle_soft_park_cta_click` ~657** | same decisive Revise / focus Clarify / abandon Quit |
| **Keyboard accelerators** | `viewer.rs` **~199–222** | empty-prompt `a`/`A`/`?`/`s`/`q`; `s` → `request_plan_revise` |
| **ACP wire** | `.../views/plan_approval_view.rs` **~556–572** | `approved` / `abandoned` / `cancelled` / `questions` |
| **Hit rect paint** | `.../views/file_search/line_viewer.rs` **~546–560, ~1511+, ~1730–1734** | `questions_button_area`, `send_button_area` (Revise), `abandon_button_area` |
| **Chrome sync** | `plan.rs` **`sync_plan_viewer_approval_chrome` ~337** | keeps `feedback_active` while park live |

### Dispatch table (panel Left Down)

| Hit area field | Product action (when `plan_approval_view` is Some) |
|----------------|-----------------------------------------------------|
| `questions_button_area` | `focus_plan_prompt(Questions)` — focus-only |
| `send_button_area` | **`request_plan_revise()`** — decisive ACP `cancelled` |
| `abandon_button_area` | `abandon_plan()` — ACP `abandoned` |
| `approve_button_area` | `approve_plan()` |
| `approve_notes_button_area` | `focus_plan_prompt(ApproveNotes)` |

**On current tree, panel wire-up matches the test’s decisive-Revise asserts.** Soft-park and panel `s` already covered by sibling tests.

---

## 3. Most likely failure mode (for the reported red)

| Rank | Mode | Signature | Likely when |
|------|------|-----------|-------------|
| **1** | **Contract lag: product decisive, test still focus-only** | `unwrap()` on `None` in Revise branch; ~0.08s | Intermediate commit after Revise product fix, before test update |
| **2** | **Contract lag: test decisive, product still focus-only** | assert park/panel not cleared; or no ACP `cancelled` | Intermediate commit after test update, before product wire |
| **3** | **Clarify / Quit only** | intent wrong / no `abandoned` | less likely; wire has been stable longer |
| **4** | **Fixture / panel open** | `line_viewer` unwrap on `show_plan_preview` | only if preview open breaks; unrelated to 0.088s CTA assert class |

**Prior in-tree diagnosis already green-fixed the Rank-1 case:**

- Product: `.agents/reports/impl-plan-revise-stuck-2026-08-09.md`
- Test: `.agents/reports/impl-plan-panel-revise-test-2026-08-09.md` (assert rewrite; no product revert)

**If CI still red on a tip that includes both reports’ code:** re-run **only this filter** (below) and paste panic text. Do not assume residual Rank-1 without reading the assertion line.

**False-approve process note (related, not this panic):** `bug:exit-plan-mode-false-approve` / RESIDUAL soft item: tool success of `exit_plan_mode` ≠ operator Approve. Product CTAs only. Keep that guard when touching plan park; do not invent freeform chat approve menus.

---

## 4. Critical files table

| File | Why |
|------|-----|
| `crates/codegen/xai-grok-pager/src/app/agent_view/plan.rs` | Test body; `request_plan_revise` / `send_plan_feedback` / `abandon_plan` / soft-park CTAs / flush suite |
| `crates/codegen/xai-grok-pager/src/app/agent_view/viewer.rs` | Panel mouse + empty-prompt keys |
| `crates/codegen/xai-grok-pager/src/views/plan_approval_view.rs` | ACP outcomes, `PlanPromptIntent`, park state |
| `crates/codegen/xai-grok-pager/src/views/file_search/line_viewer.rs` | Footer hit areas paint |
| `crates/codegen/xai-grok-pager/docs/user-guide/19-plan-mode.md` | User-facing CTA contract (if product behavior changes) |
| `FORK.md` (§ plan approval CTAs + filter list) | Ship note + regression filter snippet |
| `.agents/reports/impl-plan-revise-stuck-2026-08-09.md` | Decisive Revise product fix |
| `.agents/reports/impl-plan-panel-revise-test-2026-08-09.md` | This test’s contract update |

---

## 5. Recommended steps (required, not optional polish)

Do **not** treat sibling coverage as optional polish. Elevate to **required** for any fix touch:

1. **Confirm tip tree** has both decisive product (`viewer.rs` send → `request_plan_revise`) **and** decisive test asserts (no `unwrap` park after Revise).
2. **Single-test red/green** with the command in §7 (capture exact panic).
3. **Sibling regression filter (required):**
   - `soft_park_revise_cta_click_submits_cancelled_immediately`
   - `soft_park_revise_cta_click_includes_existing_freeform`
   - `panel_empty_prompt_s_submits_cancelled_immediately`
   - `plan_panel_click_approve_button_approves`
   - `plan_panel_click_approve_notes_button_focuses_prompt`
   - `plan_panel_click_clarify_revise_quit_buttons`
   - `soft_park_clarify_cta_then_enter` (or clarify sibling name in suite)
   - soft-park Approve without panel (`soft_park_card_or_chrome_cta_click_approve_without_panel` if present)
4. **False-approve guard:** do not change soft-park so that `exit_plan_mode` tool return implies Approve; keep CTAs as the only approval path.
5. **Docs:** if product contract changes, update `19-plan-mode.md` + FORK ship note same turn (not “later polish”).
6. **Post-edit:** `cargo fmt -p xai-grok-pager`; targeted clippy on package if product `*.rs` changed; **not** full workspace as first gate.

### Fix strategy by mode

| If you see… | Do |
|-------------|-----|
| `unwrap` on `None` after Revise click | Align test with decisive contract (already on current tip); **never** re-soft Revise to make old asserts green |
| Park still Some after Revise | Fix product: `send_area` / soft-park revise / empty `s` must call `request_plan_revise` |
| Wrong ACP outcome string | Fix `send_cancelled` / `send_abandoned` wire only; keep string names stable |
| Clarify intent wrong | Fix `questions_area` → `focus_plan_prompt(Questions)` only |

---

## 6. What NOT to do

| Waste / risk | Why |
|--------------|-----|
| Full `just check` / full workspace nextest as the **first** loop | ~28k tests; this failure is a ~0.1s unit assert |
| Revert Revise to focus-only to green the test | Re-opens dogfood “stuck panel / Enter:approve” bug |
| Weaken asserts (`is_some()` without outcome check) | Hides missing ACP notify |
| Parallel parent greps + full suite while one filter is enough | Token/time waste; map is complete |
| Host keyring / sudo / “live” plan mode dogfood as **diagnosis** | Unit test is hermetic; dogfood only after green filter |
| invent freeform chat approve/revise menus | Violates plan CTA process law |

---

## 7. Verification (single test only)

From grok-build root:

```bash
# Narrowest (this exact test)
cargo test -p xai-grok-pager --lib -- plan_panel_click_clarify_revise_quit_buttons -- --nocapture

# Or with nextest (matches just check unit runner)
cargo nextest run -p xai-grok-pager --lib plan_panel_click_clarify_revise_quit_buttons

# Required sibling slice after any edit
cargo test -p xai-grok-pager --lib -- \
  soft_park_revise_cta_click_submits_cancelled_immediately \
  soft_park_revise_cta_click_includes_existing_freeform \
  panel_empty_prompt_s_submits_cancelled_immediately \
  plan_panel_click_approve_button_approves \
  plan_panel_click_approve_notes_button_focuses_prompt \
  plan_panel_click_clarify_revise_quit_buttons
```

Prefer `just cargo-ci cargo test -p xai-grok-pager --lib --locked -- <filters>` when matching CI hermetic cargo wrapper.

**Done when:** this filter is green; sibling slice green; product still decisive Revise (not focus-only).

---

## 8. Bottom line

- **Test:** `plan.rs` **L4358** in `approve_plan_flush_tests`.
- **Product:** panel footer mouse in `viewer.rs`; Revise → `request_plan_revise` → ACP `cancelled`.
- **Fast fail class:** assert/panic on Revise contract mismatch (historic: unwrap after park cleared).
- **Current tip code already encodes the fixed contract**; if CI still fails, capture this filter’s panic line first, then fix **mismatch**, never full-suite thrash.
- **No product edits performed** in this explore pass.
