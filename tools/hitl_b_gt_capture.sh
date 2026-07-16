#!/usr/bin/env bash
# Milestone B — Ground-truth capture (YOU run this once).
#
# What it does:
#   1) Offline patch oracle dump (no device) → testdata/xdna2_cmd/gt_capture/
#   2) Optional vendor OFM golden re-capture under temporary L3 (NPU)
#   3) Re-run offline dump + preflight
#
# Fish-safe: run with bash. You only need sudo for the vendor step.
#
# Usage:
#   bash tools/hitl_b_gt_capture.sh              # offline only (safe, no sudo)
#   bash tools/hitl_b_gt_capture.sh --with-vendor  # + vendor golden (needs sudo L3)
#   bash tools/hitl_b_gt_capture.sh --with-vendor --assume-l3-open
#
# After success: agent can do offline parity without more HITL until Pass apply.
set -euo pipefail
export PATH="/usr/bin:${HOME}/.local/bin:${PATH}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

WITH_VENDOR=0
ASSUME_L3=0
for a in "$@"; do
  case "$a" in
    --with-vendor) WITH_VENDOR=1 ;;
    --assume-l3-open|--assume-l3-already-open) ASSUME_L3=1 ;;
    -h|--help)
      sed -n '1,30p' "$0" | sed 's/^# \?//'
      exit 0
      ;;
    *)
      echo "ERROR: unknown arg: $a" >&2
      exit 2
      ;;
  esac
done

ALLOW_PATH="/sys/module/amdxdna/parameters/exec_cmd_allow"
WE_OPENED_L3=0

restore_l3() {
  if [[ "$WE_OPENED_L3" -eq 1 ]]; then
    echo "RESTORE: writing exec_cmd_allow=0 …"
    if [[ -w "$ALLOW_PATH" ]]; then
      echo 0 >"$ALLOW_PATH" || true
    else
      sudo sh -c "echo 0 > $ALLOW_PATH" || true
    fi
  fi
  if [[ -r "$ALLOW_PATH" ]]; then
    echo "POST: exec_cmd_allow=$(tr -d '[:space:]' <"$ALLOW_PATH")"
  fi
}
trap restore_l3 EXIT

echo "=== Milestone B: GT capture ==="
echo "repo: $ROOT"
echo ""

# Ensure ELF present
if [[ ! -f testdata/xdna2_cmd/silicon_lab/lab_instr.elf ]]; then
  if [[ -f testdata/xdna2_cmd/silicon_lab/lab_instr.bin ]]; then
    bash tools/stage_aiebu_lab_instr.sh
  else
    echo "ERROR: no silicon_lab assets — run: bash tools/silicon_lab_operator.sh stage" >&2
    exit 2
  fi
fi

echo "--- (1/3) Offline patch oracle (no L3) ---"
bash tools/dump_mladf_gt_offline.sh
echo ""

if [[ "$WITH_VENDOR" -eq 1 ]]; then
  echo "--- (2/3) Vendor OFM golden (needs NPU + temporary L3) ---"
  allow_now="?"
  [[ -r "$ALLOW_PATH" ]] && allow_now="$(tr -d '[:space:]' <"$ALLOW_PATH" || true)"
  echo "preflight: exec_cmd_allow=$allow_now"

  if [[ "$allow_now" == "1" || "$allow_now" == "Y" || "$allow_now" == "y" ]]; then
    echo "L3 already open (ok for vendor capture)."
  else
    if [[ "$ASSUME_L3" -eq 1 ]]; then
      echo "ERROR: --assume-l3-open but exec_cmd_allow=$allow_now" >&2
      exit 2
    fi
    echo "Opening L3 temporarily (sudo) …"
    sudo sh -c "echo 1 > $ALLOW_PATH"
    WE_OPENED_L3=1
    echo "exec_cmd_allow=$(tr -d '[:space:]' <"$ALLOW_PATH")"
  fi

  bash tools/silicon_lab_operator.sh capture-golden
  echo ""
  echo "--- (3/3) Refresh offline GT + preflight ---"
  bash tools/dump_mladf_gt_offline.sh
  bash tools/silicon_lab_preflight.sh --require-pass-assets --print-env | head -40
else
  echo "--- (2/3) Vendor capture SKIPPED (offline-only mode) ---"
  echo "  To re-lock OFM golden from NPU (one sudo window):"
  echo "    bash tools/hitl_b_gt_capture.sh --with-vendor"
  echo ""
  echo "--- (3/3) Preflight (may warn if golden missing) ---"
  bash tools/silicon_lab_preflight.sh --print-env || true
fi

echo ""
echo "=== Milestone B complete ==="
echo "Artifacts: $ROOT/testdata/xdna2_cmd/gt_capture/"
echo "  MANIFEST.sha256  instr_post_patch.bin  ctrl_post_patch.bin  contract.json"
echo ""
echo "Tell the agent: \"B done\" (or paste nothing if offline-only succeeded)."
echo "Next HITL when agent says Pass-ready:"
echo "  bash tools/hitl_c_pass_apply.sh"
echo ""
echo "Honesty: B ≠ ran_on_npu. Offline dump ≠ live XRT BO capture."
