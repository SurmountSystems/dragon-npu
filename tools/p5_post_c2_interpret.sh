#!/usr/bin/env bash
# P5 post-C2 offline interpret — classify + C3 + C4 + P6 one-shot (no L3 / no ALLOW).
#
# IMPL 68eeaaf6 + post-timeout honesty: after operator sandwich (or anytime on
# fixture), interpret last_apply offline. Prints dual residual honesty +
# CONFIG_CU tokens when present. Unlock SSoT (latest consumed one-shot):
# **C_UNLOCK=97f38307 CONSUMED** (2026-08-10 stock research-blob CONFIG_CU;
# status=wait_cmd_einval; last_apply pin). Prior: 11bc9016 CONSUMED (2026-08-10);
# 3a70fabf CONSUMED (2026-08-09 outer kill); b17b0119 CONSUMED historical.
# Not active. Agent never auto-marks unlocks.
#
# What this does (offline only; NO L3 / NO EXEC / never sets ALLOW):
#   1) Soft classify (tools/classify_last_apply_residual.sh)
#   2) C3 evidence detect (tools/c3_post_c2_classify.sh)
#   3) C4 Pass-evidence audit (tools/c4_pass_check.sh)
#   4) P6 true-Pass criteria (tools/p6_true_pass_check.sh) — offline only
#   5) Unlock bookkeeping note (human must mark CONSUMED only if sandwich ran)
#
# Always prints: not_c_ready=1 not_ran_on_npu=1 invent_pass=0
# Never opens L3. Never sets XDNA2_ALLOW_EXEC_CMD. Never invents Pass.
#
# Usage:
#   bash tools/p5_post_c2_interpret.sh
#   bash tools/p5_post_c2_interpret.sh /path/to/last_apply.json
#   # bash (portable env prefix):
#   env XDNA2_LAB_LAST_APPLY_JSON=/path/to.json bash tools/p5_post_c2_interpret.sh
#   # fish:
#   set -gx XDNA2_LAB_LAST_APPLY_JSON /path/to.json; bash tools/p5_post_c2_interpret.sh
#
# Exit:
#   0  soft interpret complete (C3_SKIP / C4_SKIP / P6_SKIP expected on locked
#      baseline; non-2 child soft failures also exit 0 with P5_INTERPRET_SOFT_WARN)
#   2  bad args / self path refuse, **or** any child returns 2 (path refuse)
#
# Honesty: locked baseline → C3_SKIP + C4_SKIP + P6_NOT_PASS/P6_SKIP + config_cu_present=0.
# New last_apply with config_cu= → C3_EVIDENCE possible; still ≠ product Pass.
set -euo pipefail
export PATH="/usr/bin${HOME:+:${HOME}/.local/bin}:${PATH:-}"
# Never inherit sticky userspace EXEC gate.
unset XDNA2_ALLOW_EXEC_CMD || true

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

IN=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    -h|--help)
      sed -n '1,40p' "$0" | grep -E '^#([^!]|$)' | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    -*)
      echo "ERROR: unknown $1 (allowlist: --help; optional last_apply path)" >&2
      exit 2
      ;;
    *)
      if [[ -n "$IN" ]]; then
        echo "ERROR: only one path argument allowed (got '$IN' and '$1')" >&2
        exit 2
      fi
      IN="$1"
      shift
      ;;
  esac
done

# Optional path pin for child scripts (same confinement they enforce).
# Pass argv path to children so explicit refuse exits 2 (env-only pin soft-falls
# through to fixture and would mask refuse as clean C3_SKIP).
CHILD_PATH_ARGS=()
if [[ -n "$IN" ]]; then
  if [[ ! -f "$IN" ]]; then
    echo "ERROR: path not a file: $IN" >&2
    echo "PLAIN: Explicit pin missing — matches outer timeout kill before last_apply" >&2
    echo "  stamp (incomplete evidence; not APP_LOAD classify; not product Pass)." >&2
    echo "P5_INTERPRET_PATH_REFUSE invent_pass=0 not_c_ready=1 not_ran_on_npu=1 incomplete_timeout_or_missing_pin=1" >&2
    exit 2
  fi
  export XDNA2_LAB_LAST_APPLY_JSON="$IN"
  CHILD_PATH_ARGS=("$IN")
fi

echo "=== p5_post_c2_interpret (offline; ≠ Pass ≠ ran_on_npu ≠ auto-CONSUME unlock) ==="
echo "ROOT=$ROOT"
echo "NOTE: XDNA2_ALLOW_EXEC_CMD unset; never open L3; invent_pass=0 not_c_ready=1 not_ran_on_npu=1"
echo "UNLOCK: C_UNLOCK=97f38307 is CONSUMED (not active; 2026-08-10 stock research-blob wait_cmd_einval). Further live C needs a new host-minted unlock after named offline delta."
if [[ -n "${XDNA2_LAB_LAST_APPLY_JSON:-}" ]]; then
  echo "XDNA2_LAB_LAST_APPLY_JSON=$XDNA2_LAB_LAST_APPLY_JSON"
  # Loud baseline fixture note if path is the locked repo last_apply.
  case "${XDNA2_LAB_LAST_APPLY_JSON}" in
    */testdata/xdna2_cmd/last_apply.json|"$ROOT/testdata/xdna2_cmd/last_apply.json")
      echo "LOUD: path is locked baseline fixture (2026-07-18 APP_LOAD) — not a timeout/CONFIG_CU shot pin."
      ;;
  esac
else
  echo "path: default (repo fixture or child discovery)"
  echo "LOUD: no explicit pin — child may fall back to locked baseline fixture only."
fi
echo

echo "--- (1/4) soft classify ---"
set +e
bash tools/classify_last_apply_residual.sh "${CHILD_PATH_ARGS[@]}"
CLASSIFY_RC=$?
set -e
echo "classify_rc=$CLASSIFY_RC"
echo

echo "--- (2/4) C3 evidence ---"
set +e
bash tools/c3_post_c2_classify.sh "${CHILD_PATH_ARGS[@]}"
C3_RC=$?
set -e
echo "c3_rc=$C3_RC"
echo

echo "--- (3/4) C4 Pass audit ---"
set +e
bash tools/c4_pass_check.sh "${CHILD_PATH_ARGS[@]}"
C4_RC=$?
set -e
echo "c4_rc=$C4_RC"
echo

echo "--- (4/4) P6 true-Pass criteria ---"
set +e
bash tools/p6_true_pass_check.sh "${CHILD_PATH_ARGS[@]}"
P6_RC=$?
set -e
echo "p6_rc=$P6_RC"
echo

echo "--- dual residual honesty ---"
echo "userspace: ELF+aiebu+rela may be OK while FW residual still APP_LOAD (0x3000003)."
echo "config_cu: attach tokens (config_cu= / pdi_bytes= / cu_func=) are research only;"
echo "  CONFIG_CU attach alone ≠ Pass ≠ residual closed. Wrong overlay may still 0x3000003."
echo "capture: testdata/xdna2_cmd/external_known_good/ may still lack working-stack bins."
echo "true Pass: ERT Completed + non-zero OFM allclose + real_ioctl via live decide only;"
echo "  offline C4_PASS_CANDIDATE / P6_OFFLINE_CANDIDATE ≠ product Pass ≠ invent ran_on_npu."
echo "always: invent_pass=0 not_c_ready=1 not_ran_on_npu=1 product_pass_requires_live_decide=1"
echo

echo "--- unlock bookkeeping (human; agent does not auto-edit) ---"
echo "C_UNLOCK=97f38307 is CONSUMED (2026-08-10 stock live: wait_cmd_einval after measured research-blob CONFIG_CU; see RESIDUAL)."
echo "Not active. Do not reopen under 97f38307. Do not mark CONSUMED again from soft reconfirm."
echo "Prior: 11bc9016 CONSUMED (2026-08-10); 3a70fabf CONSUMED (2026-08-09 outer timeout); b17b0119 CONSUMED historical."
echo "Further live C needs a **new** host-minted unlock + named offline reason (capture / matmul PDI / driver-MMU)."
echo "Agent: never open L3 / never ALLOW=1 / never invent Pass."
echo

# Promote path refuse (child exit 2) so operators do not treat refuse as clean skip.
if [[ "$CLASSIFY_RC" -eq 2 || "$C3_RC" -eq 2 || "$C4_RC" -eq 2 || "$P6_RC" -eq 2 ]]; then
  echo "P5_INTERPRET_PATH_REFUSE classify_rc=$CLASSIFY_RC c3_rc=$C3_RC c4_rc=$C4_RC p6_rc=$P6_RC invent_pass=0 not_c_ready=1 not_ran_on_npu=1" >&2
  exit 2
fi
if [[ "$CLASSIFY_RC" -ne 0 || "$C3_RC" -ne 0 || "$C4_RC" -ne 0 || "$P6_RC" -ne 0 ]]; then
  echo "P5_INTERPRET_SOFT_WARN classify_rc=$CLASSIFY_RC c3_rc=$C3_RC c4_rc=$C4_RC p6_rc=$P6_RC (still exit 0 soft)"
else
  echo "P5_INTERPRET_OK invent_pass=0 not_c_ready=1 not_ran_on_npu=1 product_pass_requires_live_decide=1"
fi
exit 0
