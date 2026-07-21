#!/usr/bin/env bash
# Operator C2 one-shot helper — prep + exact sandwich + optional apply.
#
# IMPL de4d9fa4: C2 live blocked on operator sudo when L3 closed.
# Unlock SSoT: first C_UNLOCK=… line in RESIDUAL.md (soft NOTE only — never
# hard-blocks prep/print/apply). Active stamp is whatever RESIDUAL first-match
# presents without CONSUMED on that line (P4: C_UNLOCK=3a70fabf one-shot for
# CONFIG_CU sandwich). When the first match is marked CONSUMED (or stamp appears
# CONSUMED elsewhere), printed sandwich L3/apply lines are comment-only
# HISTORICAL (non-executable); further C needs a **new** agent unlock + offline
# delta. Historical b17b0119 (operator C2 2026-07-18) remains CONSUMED mid-file.
# Never invent a stamp. CONSUMED never hard-blocks --apply-if-l3-open (standing
# soft-NOTE policy; operator still owns L3).
#
# What this does (default = safe offline prep, NO L3 open, NO EXEC):
#   1) Soft NOTE if RESIDUAL has C_UNLOCK= (active vs CONSUMED)
#   2) Unlock-ban soft NOTE (false unlocks ≠ Pass)
#   3) Optional offline gates (soft; ≠ C-ready)
#   4) Preflight --require-pass-assets (assets only)
#   5) Print exact bash + fish L3 sandwich commands (even if preflight red)
#   6) Print post-C2 classify + offline gates steps
#
# Live apply is NEVER started unless L3 is already open AND you pass
# --apply-if-l3-open (delegates to hitl_c_pass_apply.sh --assume-l3-already-open).
# This script does NOT write exec_cmd_allow (never opens L3 by itself).
# Unsets ambient XDNA2_ALLOW_EXEC_CMD at entry (never sets it).
#
# Usage:
#   bash tools/hitl_c2_once.sh                    # prep + print sandwich (default)
#   bash tools/hitl_c2_once.sh --print-only       # sandwich only (no gates/preflight)
#   bash tools/hitl_c2_once.sh --apply-if-l3-open # prep + apply only if L3 already 1
#   bash tools/hitl_c2_once.sh --skip-offline-gates
#   bash tools/hitl_c2_once.sh --skip-preflight
#   bash tools/hitl_c2_once.sh --run-offline-gates # hard-fail offline gates
#
# Flag conflicts (exit 2):
#   --print-only with --apply-if-l3-open
#   --skip-offline-gates with --run-offline-gates
#
# Fail-closed demo (agent / CI — L3 closed → refuse):
#   bash tools/hitl_c_pass_apply.sh --assume-l3-already-open
#   # expect exit 1 when exec_cmd_allow is 0/N
#   bash tools/hitl_c2_once.sh --apply-if-l3-open
#   # same refuse (exit 1) when L3 closed
#
# After operator sandwich: restore L3, then:
#   bash tools/p5_post_c2_interpret.sh         # P5: classify+C3+C4 one-shot (≠ Pass)
#   # or: bash tools/c3_post_c2_classify.sh    # soft; C3_SKIP if no new evidence
#   # bash tools/c3_post_c2_classify.sh --require-new  # hard research (exit 2 if no new)
#   # bash tools/c4_pass_check.sh              # soft; C4_SKIP without PassCandidate
#   bash tools/run_offline_gates.sh
#   # paste last_apply / journal for agent C3 / offline PassCandidate audit (≠ product Pass)
#   # Mark C_UNLOCK=3a70fabf CONSUMED only if sandwich actually ran (new last_apply)
#
# Exit:
#   0  prep/print ok (or rare Pass path via hitl_c when L3 open)
#   1  --apply-if-l3-open but L3 closed / hitl_c refuse (not silicon Pass)
#   2  bad args / flag conflict / preflight or gates hard fail
#   4  live apply !ran_on_npu (common APP_LOAD) and L3 restored
#   5  L3 left open after apply (CRITICAL — close manually)
#
# Fish-safe: always invoke with bash.
set -euo pipefail
export PATH="/usr/bin:${HOME}/.local/bin:${PATH}"
# Never inherit sticky userspace EXEC gate into lab/cargo children.
unset XDNA2_ALLOW_EXEC_CMD || true
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

PRINT_ONLY=0
APPLY_IF_L3=0
SKIP_GATES=0
SKIP_PREFLIGHT=0
FORCE_GATES=0

for a in "$@"; do
  case "$a" in
    --print-only) PRINT_ONLY=1 ;;
    --apply-if-l3-open|--apply-if-l3-already-open) APPLY_IF_L3=1 ;;
    --skip-offline-gates) SKIP_GATES=1 ;;
    --skip-preflight) SKIP_PREFLIGHT=1 ;;
    --run-offline-gates) FORCE_GATES=1 ;;
    -h|--help)
      grep -E '^#([^!]|$)' "$0" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      echo "ERROR: unknown arg: $a" >&2
      echo "allowlist: --print-only --apply-if-l3-open --skip-offline-gates --skip-preflight --run-offline-gates" >&2
      exit 2
      ;;
  esac
done

if [[ "$PRINT_ONLY" -eq 1 && "$APPLY_IF_L3" -eq 1 ]]; then
  echo "ERROR: --print-only conflicts with --apply-if-l3-open (refuse silent override)" >&2
  exit 2
fi
if [[ "$SKIP_GATES" -eq 1 && "$FORCE_GATES" -eq 1 ]]; then
  echo "ERROR: --skip-offline-gates conflicts with --run-offline-gates" >&2
  exit 2
fi

ALLOW_PATH="/sys/module/amdxdna/parameters/exec_cmd_allow"

read_allow() {
  if [[ -r "$ALLOW_PATH" ]]; then
    tr -d '[:space:]' <"$ALLOW_PATH" || true
  else
    echo "missing"
  fi
}

echo "=== Operator C2 one-shot helper (IMPL de4d9fa4 prep) ==="
echo "ROOT=$ROOT"
echo "Honesty: unlock ≠ Pass ≠ ran_on_npu. This script never opens L3."
echo "NOTE: XDNA2_ALLOW_EXEC_CMD unset at entry (userspace gate; never set here)." >&2

# Soft unlock stamp from RESIDUAL SSoT (NOTE only — never hard-blocks prep).
# Tolerate markdown: **C_UNLOCK=…**, `C_UNLOCK=…`, indent, mid-line.
# Detect CONSUMED so historical b17b0119 does not print as active one-shot.
_c_unlock_tok=""
_c_unlock_consumed=0
if [[ -f "$ROOT/RESIDUAL.md" ]]; then
  _c_unlock_line="$(
    { grep -m1 -E 'C_UNLOCK=[A-Za-z0-9_]+' "$ROOT/RESIDUAL.md" || true; } | head -n1
  )"
  _c_unlock_tok="$(
    { grep -oE 'C_UNLOCK=[A-Za-z0-9_]+' <<<"${_c_unlock_line}" || true; } \
      | head -n1 | tr -d '`"*' | tr -d '[:space:]'
  )"
  if [[ -n "${_c_unlock_tok}" ]]; then
    _c_unlock_stamp="${_c_unlock_tok#C_UNLOCK=}"
    if grep -qiE 'CONSUMED' <<<"${_c_unlock_line}"; then
      _c_unlock_consumed=1
    elif grep -E "C_UNLOCK=${_c_unlock_stamp}" "$ROOT/RESIDUAL.md" \
      | grep -qiE 'CONSUMED'; then
      _c_unlock_consumed=1
    elif grep -qiE "${_c_unlock_stamp}[^A-Za-z0-9_].*CONSUMED|CONSUMED[^A-Za-z0-9_].*${_c_unlock_stamp}" \
      "$ROOT/RESIDUAL.md"; then
      _c_unlock_consumed=1
    fi
  fi
fi
if [[ -n "${_c_unlock_tok}" && "${_c_unlock_consumed}" -eq 1 ]]; then
  echo "NOTE: ${_c_unlock_tok} is **CONSUMED** (historical one-shot; not active)." >&2
  echo "NOTE: further C needs a **new** agent unlock + offline delta; restore L3; ≠ Pass / ≠ ran_on_npu." >&2
  echo "NOTE: unlock ≠ Pass ≠ permanent open season. L3=exec_cmd_allow; userspace ALLOW=XDNA2_ALLOW_EXEC_CMD (leave both closed after)." >&2
elif [[ -n "${_c_unlock_tok}" ]]; then
  echo "NOTE: ${_c_unlock_tok} one-shot (active); restore L3 (exec_cmd_allow=0); ≠ ran_on_npu until Pass" >&2
  echo "NOTE: unlock ≠ Pass ≠ permanent open season. L3=exec_cmd_allow; userspace ALLOW=XDNA2_ALLOW_EXEC_CMD (leave both closed after)." >&2
else
  echo "NOTE: no C_UNLOCK= stamp in RESIDUAL.md (soft; prep still allowed)." >&2
  echo "NOTE: prefer agent unlock stamp before live apply; research force still allowed." >&2
fi

# Soft unlock-ban (parity with hitl_c honesty; never hard-blocks).
echo "NOTE: unlock-ban — false unlocks alone never equal Pass:" >&2
echo "NOTE:   CONTRACT_OK / OFFLINE_GATES_OK / run_offline_gates green" >&2
echo "NOTE:   unpatched CLASS A / mere diff rc=0 (body match only)" >&2
echo "NOTE:   registry BO alone (Transaction/cache, not live run_ BO)" >&2
echo "NOTE:   residual_class / CLASSIFY_SOFT_OK / app_load_checklist" >&2
echo "NOTE: See RESIDUAL.md / docs/HITL_C_APP_LOAD_RECIPE.md." >&2

allow_now="$(read_allow)"
echo "driver: exec_cmd_allow=$allow_now (sample at start; re-read before apply)"

# Soft C_UNLOCK comment line for sandwich (parsed token or placeholder).
if [[ -n "${_c_unlock_tok}" && "${_c_unlock_consumed}" -eq 1 ]]; then
  _c_unlock_comment="${_c_unlock_tok} CONSUMED — further C needs NEW agent unlock (not active)"
elif [[ -n "${_c_unlock_tok}" ]]; then
  _c_unlock_comment="${_c_unlock_tok} one-shot (active)"
else
  _c_unlock_comment="C_UNLOCK=<see RESIDUAL.md>"
fi

print_sandwich() {
  echo ""
  if [[ "${_c_unlock_consumed}" -eq 1 ]]; then
    echo "======== OPERATOR C2 SANDWICH — HISTORICAL / CONSUMED (record only) ========"
    echo "NOTE: stamp consumed — L3/apply lines below are commented (non-executable)."
    echo "NOTE: further live C needs a NEW agent unlock + offline delta first."
    echo "NOTE: CONSUMED is soft-NOTE only (never hard-blocks --apply-if-l3-open)."
    echo ""
    echo "--- bash (HISTORICAL / CONSUMED; do not paste-run L3 under old stamp) ---"
    cat <<EOF
cd ${ROOT}
export PATH="/usr/bin:\${HOME}/.local/bin:\${PATH}"
# optional: export XDNA2_LAB_BO_DIAG=1
# never leave sticky: unset XDNA2_ALLOW_EXEC_CMD

# ${_c_unlock_comment}; restore L3; ≠ ran_on_npu until Pass
# (record only) sudo sh -c 'echo 1 > /sys/module/amdxdna/parameters/exec_cmd_allow'
# (record only) bash tools/hitl_c_pass_apply.sh --assume-l3-already-open
# (record only) sudo sh -c 'echo 0 > /sys/module/amdxdna/parameters/exec_cmd_allow'
# (record only) cat /sys/module/amdxdna/parameters/exec_cmd_allow   # must be 0 or N
EOF
    echo ""
    echo "--- fish (HISTORICAL / CONSUMED; do not paste-run L3 under old stamp) ---"
    cat <<EOF
cd ${ROOT}
set -gx PATH /usr/bin \$HOME/.local/bin \$PATH
# optional: set -gx XDNA2_LAB_BO_DIAG 1
# never leave sticky: set -e XDNA2_ALLOW_EXEC_CMD

# ${_c_unlock_comment}; restore L3; ≠ ran_on_npu until Pass
# (record only) sudo sh -c 'echo 1 > /sys/module/amdxdna/parameters/exec_cmd_allow'
# (record only) bash tools/hitl_c_pass_apply.sh --assume-l3-already-open
# (record only) sudo sh -c 'echo 0 > /sys/module/amdxdna/parameters/exec_cmd_allow'
# (record only) cat /sys/module/amdxdna/parameters/exec_cmd_allow   # must be 0 or N
EOF
  else
    echo "======== OPERATOR C2 SANDWICH (you run; agent does not) ========"
    echo "NOTE: P4 CONFIG_CU path — run a-prep + export PDI env before L3 (comments below)."
    echo "NOTE: CONFIG_CU attach alone ≠ Pass ≠ ran_on_npu (wrong overlay may still 0x3000003)."
    echo ""
    echo "--- bash ---"
    cat <<EOF
cd ${ROOT}
export PATH="/usr/bin:\${HOME}/.local/bin:\${PATH}"
# optional: export XDNA2_LAB_BO_DIAG=1
# never leave sticky: unset XDNA2_ALLOW_EXEC_CMD

# CONFIG_CU pre-step (P4 unlock intent; default off until exported):
# bash tools/operator_app_load_kit.sh a-prep
# export XDNA2_LAB_PDI_BLOB=/path/from/a-prep   # host path printed by a-prep; do not invent
# export XDNA2_LAB_CU_FUNC=0   # optional; default 0
# honesty: attach alone ≠ Pass ≠ ran_on_npu

# ${_c_unlock_comment}; restore L3; ≠ ran_on_npu until Pass
sudo sh -c 'echo 1 > /sys/module/amdxdna/parameters/exec_cmd_allow'
bash tools/hitl_c_pass_apply.sh --assume-l3-already-open
sudo sh -c 'echo 0 > /sys/module/amdxdna/parameters/exec_cmd_allow'
cat /sys/module/amdxdna/parameters/exec_cmd_allow   # must be 0 or N
EOF
    echo ""
    echo "--- fish ---"
    cat <<EOF
cd ${ROOT}
set -gx PATH /usr/bin \$HOME/.local/bin \$PATH
# optional: set -gx XDNA2_LAB_BO_DIAG 1
# never leave sticky: set -e XDNA2_ALLOW_EXEC_CMD

# CONFIG_CU pre-step (P4 unlock intent; default off until exported):
# bash tools/operator_app_load_kit.sh a-prep
# set -gx XDNA2_LAB_PDI_BLOB /path/from/a-prep   # host path printed by a-prep; do not invent
# set -gx XDNA2_LAB_CU_FUNC 0   # optional; default 0
# honesty: attach alone ≠ Pass ≠ ran_on_npu

# ${_c_unlock_comment}; restore L3; ≠ ran_on_npu until Pass
sudo sh -c 'echo 1 > /sys/module/amdxdna/parameters/exec_cmd_allow'
bash tools/hitl_c_pass_apply.sh --assume-l3-already-open
sudo sh -c 'echo 0 > /sys/module/amdxdna/parameters/exec_cmd_allow'
cat /sys/module/amdxdna/parameters/exec_cmd_allow   # must be 0 or N
EOF
  fi
  echo ""
  echo "If L3 left open after any failure:"
  echo "  sudo sh -c 'echo 0 > /sys/module/amdxdna/parameters/exec_cmd_allow'"
  echo ""
  echo "======== POST-C2 (after sandwich; no L3 needed) ========"
  cat <<'EOF'
# P5 one-shot interpret (classify + C3 + C4 + P6; ≠ Pass ≠ auto-CONSUME unlock):
bash tools/p5_post_c2_interpret.sh
# or step-by-step:
# C3 pipeline (soft classify + new-evidence detect; ≠ C-ready / ≠ Pass):
bash tools/c3_post_c2_classify.sh
# hard research if you expect a fresh C2 shot:
# bash tools/c3_post_c2_classify.sh --require-new
# raw classify only:
# bash tools/classify_last_apply_residual.sh

# C4 offline PassCandidate audit (≠ invent product Pass; always not_c_ready=1):
bash tools/c4_pass_check.sh
# bash tools/c4_pass_check.sh --require-pass   # exit 2 if not PassCandidate

# P6 true product-Pass criteria (offline readiness; still ≠ product Pass):
bash tools/p6_true_pass_check.sh
# bash tools/p6_true_pass_check.sh --require-pass  # exit 2 if not offline candidate

# Offline gates (≠ C-ready):
bash tools/run_offline_gates.sh

# Paste for agent C3 (fail residual) or C4 offline PassCandidate (product Pass =
# live decide only if OFM allclose + ERT Completed + ran_on_npu=true):
#   - hitl_c / lab_exec exit code
#   - last_apply.json (testdata/xdna2_cmd/ or /tmp/dragon_npu_last_apply_*.json)
#   - detail: aiebu_elf= rela_count= ert_state= ofm_stats= fw_status_hint=
#   - journal: journalctl -k -b --no-pager | rg -i 'Failed cmd|Status 0x|APP_LOAD|amdxdna' | tail -20
#   - cat /sys/module/amdxdna/parameters/exec_cmd_allow   # prove restored 0/N
# Honesty: C3 without new last_apply → C3_SKIP ≠ silicon progress
#          C4_SKIP / C4_PASS_CANDIDATE offline ≠ invent product Pass
#          P6_SKIP / P6_NOT_PASS / P6_OFFLINE_CANDIDATE ≠ product Pass
EOF
  echo "==============================================================="
}

if [[ "$PRINT_ONLY" -eq 1 ]]; then
  print_sandwich
  echo "PRINT_ONLY: no gates, no preflight, no apply."
  exit 0
fi

PREP_RC=0

# Optional offline gates (soft unless --run-offline-gates forces hard fail).
if [[ "$FORCE_GATES" -eq 1 ]] || [[ "$SKIP_GATES" -eq 0 ]]; then
  if [[ -f "$ROOT/tools/run_offline_gates.sh" ]]; then
    echo "--- offline gates (soft prep; ≠ C-ready / ≠ unlock) ---"
    set +e
    bash "$ROOT/tools/run_offline_gates.sh"
    og=$?
    set -e
    if [[ "$og" -ne 0 ]]; then
      if [[ "$FORCE_GATES" -eq 1 ]]; then
        echo "ERROR: run_offline_gates rc=$og (--run-offline-gates hard)" >&2
        PREP_RC=2
      else
        echo "WARN: run_offline_gates rc=$og (soft; continuing prep). Prefer green before C2." >&2
      fi
    else
      echo "NOTE: OFFLINE_GATES_OK still ≠ Pass / ≠ permanent unlock." >&2
    fi
  else
    echo "WARN: tools/run_offline_gates.sh missing (soft skip)." >&2
  fi
else
  echo "NOTE: offline gates skipped (--skip-offline-gates)."
fi

if [[ "$SKIP_PREFLIGHT" -eq 0 ]]; then
  echo "--- preflight Pass assets ---"
  set +e
  bash tools/silicon_lab_preflight.sh --require-pass-assets
  pf=$?
  set -e
  if [[ "$pf" -ne 0 ]]; then
    echo "ERROR: preflight --require-pass-assets rc=$pf (assets red)" >&2
    PREP_RC=2
  fi
else
  echo "NOTE: preflight skipped (--skip-preflight)."
fi

# Always print sandwich before any hard exit so operator still has exact commands.
print_sandwich

if [[ "$PREP_RC" -ne 0 ]]; then
  echo "" >&2
  echo "ERROR: C2 prep hard-fail (rc=$PREP_RC). Sandwich printed above for operator recovery." >&2
  echo "Fix assets/gates offline, then re-run. Do not invent Pass / do not leave L3 open." >&2
  exit "$PREP_RC"
fi

if [[ "$APPLY_IF_L3" -eq 0 ]]; then
  # Fresh sample for operator messaging (not used for apply).
  allow_now="$(read_allow)"
  echo ""
  if [[ "${_c_unlock_consumed}" -eq 1 ]]; then
    echo "C2_PREP_OK: HISTORICAL/CONSUMED sandwich printed (L3/apply lines non-executable)."
    echo "NOTE: do **not** open L3 under consumed unlock (${_c_unlock_tok:-C_UNLOCK})."
    echo "NOTE: further live C needs a **NEW** agent unlock + offline delta first."
    echo "NOTE: CONSUMED is soft-NOTE only (never hard-blocks --apply-if-l3-open)."
    case "$allow_now" in
      1|Y|y)
        echo "NOTE: L3 is already open (exec_cmd_allow=$allow_now) — restore promptly:"
        echo "  sudo sh -c 'echo 0 > /sys/module/amdxdna/parameters/exec_cmd_allow'"
        echo "  (consumed stamp — do not treat open L3 as permission to re-apply under old unlock)."
        ;;
      *)
        echo "NOTE: L3 closed (exec_cmd_allow=$allow_now) — correct while stamp is CONSUMED."
        echo "Offline next: bash tools/c3_post_c2_classify.sh ; bash tools/run_offline_gates.sh"
        ;;
    esac
  else
    echo "C2_PREP_OK: sandwich printed. Operator must open L3 (sudo), run hitl_c, restore L3."
    case "$allow_now" in
      1|Y|y)
        echo "NOTE: L3 is already open (exec_cmd_allow=$allow_now)."
        echo "  Prefer: bash tools/hitl_c2_once.sh --apply-if-l3-open"
        echo "  Then restore: sudo sh -c 'echo 0 > /sys/module/amdxdna/parameters/exec_cmd_allow'"
        echo "  (default prep does not apply — avoids surprise EXEC)."
        ;;
      *)
        echo "NOTE: L3 closed (exec_cmd_allow=$allow_now). Agent cannot complete C2 without operator sudo."
        echo "Fail-closed check (expect exit 1 when L3 closed):"
        echo "  bash tools/hitl_c_pass_apply.sh --assume-l3-already-open"
        echo "  bash tools/hitl_c2_once.sh --apply-if-l3-open"
        ;;
    esac
  fi
  exit 0
fi

# Live apply only if L3 already open — never open it here.
# Re-read immediately before decision (gates/preflight may have taken a long time).
allow_now="$(read_allow)"
echo "driver: exec_cmd_allow=$allow_now (re-read immediately before apply decision)"
case "$allow_now" in
  1|Y|y)
    echo "--- L3 open; delegating to hitl_c_pass_apply.sh --assume-l3-already-open ---"
    set +e
    bash tools/hitl_c_pass_apply.sh --assume-l3-already-open
    rc=$?
    set -e
    # Re-read for post-check (hitl_c also checks; this is belt-and-suspenders).
    allow_post="$(read_allow)"
    echo "POST: hitl_c rc=$rc exec_cmd_allow=$allow_post"
    case "$allow_post" in
      0|N|n) echo "POST_OK: L3 closed" ;;
      *)
        echo "CRITICAL: L3 still open ($allow_post) — restore now:" >&2
        echo "  sudo sh -c 'echo 0 > /sys/module/amdxdna/parameters/exec_cmd_allow'" >&2
        rc=5
        ;;
    esac
    echo ""
    echo "Post-C2 (offline):"
    echo "  bash tools/c3_post_c2_classify.sh"
    echo "  bash tools/c4_pass_check.sh"
    echo "  bash tools/run_offline_gates.sh"
    exit "$rc"
    ;;
  *)
    echo "" >&2
    echo "ERROR: --apply-if-l3-open but exec_cmd_allow=$allow_now (L3 closed)." >&2
    echo "This is NOT an NPU/FW failure — L3 is closed (fail-closed)." >&2
    echo "This script never sets exec_cmd_allow (no passwordless L3 from agent)." >&2
    if [[ "${_c_unlock_consumed}" -eq 1 ]]; then
      echo "NOTE: unlock is **CONSUMED** — do **not** open L3 under ${_c_unlock_tok:-old stamp}." >&2
      echo "NOTE: further live C needs a **NEW** agent unlock + offline delta first." >&2
      echo "NOTE: CONSUMED is soft-NOTE only (this refuse is L3-closed, not a hard CONSUMED gate)." >&2
      echo "Offline next: bash tools/c3_post_c2_classify.sh ; bash tools/run_offline_gates.sh" >&2
    else
      echo "Open L3, then either:" >&2
      echo "  sudo sh -c 'echo 1 > /sys/module/amdxdna/parameters/exec_cmd_allow'" >&2
      echo "  bash tools/hitl_c2_once.sh --apply-if-l3-open" >&2
      echo "  sudo sh -c 'echo 0 > /sys/module/amdxdna/parameters/exec_cmd_allow'" >&2
      echo "or run the printed sandwich commands above." >&2
    fi
    exit 1
    ;;
esac
