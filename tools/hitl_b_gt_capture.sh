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
# After success: optional capture-instr for logs only (not auto C).
# C only if agent says ready — not mere CONTRACT_OK / unpatched CLASS A /
# diff rc=0 / registry alone (registry_get_instr_bo = Transaction/cache image,
# not proven live-patched run_ BO).
# Exit: 0 ok; 2 usage/assets; 5 L3 restore failed (exec_cmd_allow still open).
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
RESTORE_FAILED=0
MAIN_RC=0

read_allow() {
  if [[ -r "$ALLOW_PATH" ]]; then
    tr -d '[:space:]' <"$ALLOW_PATH" || true
  else
    echo "missing"
  fi
}

restore_l3() {
  if [[ "$WE_OPENED_L3" -eq 1 ]]; then
    echo "RESTORE: writing exec_cmd_allow=0 …"
    if [[ -w "$ALLOW_PATH" ]]; then
      echo 0 >"$ALLOW_PATH" || true
    else
      sudo sh -c "echo 0 > $ALLOW_PATH" || true
    fi
    local post
    post="$(read_allow)"
    echo "POST: exec_cmd_allow=$post"
    case "$post" in
      0|N|n)
        echo "POST_OK: L3 closed"
        ;;
      *)
        echo "CRITICAL: exec_cmd_allow still open ($post) after restore" >&2
        echo "  sudo sh -c 'echo 0 > /sys/module/amdxdna/parameters/exec_cmd_allow'" >&2
        RESTORE_FAILED=1
        ;;
    esac
  elif [[ -r "$ALLOW_PATH" ]]; then
    echo "POST: exec_cmd_allow=$(read_allow) (we did not open L3)"
  fi
  if [[ "$RESTORE_FAILED" -eq 1 ]]; then
    exit 5
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
  allow_now="$(read_allow)"
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
    echo "exec_cmd_allow=$(read_allow)"
  fi

  bash tools/silicon_lab_operator.sh capture-golden
  echo ""
  echo "--- (3/3) Refresh offline GT + preflight ---"
  bash tools/dump_mladf_gt_offline.sh
  # Avoid pipefail+head SIGPIPE (exit 141) on long preflight output.
  set +e
  bash tools/silicon_lab_preflight.sh --require-pass-assets --print-env >"$ROOT/testdata/xdna2_cmd/gt_capture/preflight_out.txt" 2>&1
  pf_rc=$?
  set -e
  head -40 "$ROOT/testdata/xdna2_cmd/gt_capture/preflight_out.txt" || true
  if [[ "$pf_rc" -ne 0 ]]; then
    echo "WARN: preflight rc=$pf_rc (see gt_capture/preflight_out.txt)" >&2
    MAIN_RC=2
  fi
else
  echo "--- (2/3) Vendor capture SKIPPED (offline-only mode) ---"
  echo "  To re-lock OFM golden from NPU (one sudo window):"
  echo "    bash tools/hitl_b_gt_capture.sh --with-vendor"
  echo ""
  echo "--- (3/3) Preflight (may warn if golden missing) ---"
  set +e
  bash tools/silicon_lab_preflight.sh --print-env >"$ROOT/testdata/xdna2_cmd/gt_capture/preflight_out.txt" 2>&1
  set -e
  head -40 "$ROOT/testdata/xdna2_cmd/gt_capture/preflight_out.txt" || true
fi

echo ""
if [[ "$MAIN_RC" -ne 0 ]]; then
  echo "=== Milestone B incomplete (rc=$MAIN_RC) ===" >&2
  exit "$MAIN_RC"
fi
echo "=== Milestone B complete ==="
echo "Artifacts: $ROOT/testdata/xdna2_cmd/gt_capture/"
echo "  MANIFEST.sha256  instr_post_patch.bin  ctrl_post_patch.bin  contract.json"
echo ""
echo "Tell the agent: \"B done\" (or paste nothing if offline-only succeeded)."
echo "Next (not auto C): residual = pure-direct / APP_LOAD (post-patch dump research closed offline)."
echo "Optional capture if classic body missing or re-land registry/classic on disk:"
echo "  sudo sh -c 'echo 1 > /sys/module/amdxdna/parameters/exec_cmd_allow'"
echo "  bash tools/silicon_lab_operator.sh capture-instr"
echo "  sudo sh -c 'echo 0 > /sys/module/amdxdna/parameters/exec_cmd_allow'"
echo "  bash tools/diff_vendor_instr_parity.sh   # unpatched A = body match ≠ C-ready"
echo "C only if agent says ready (not mere CONTRACT_OK / unpatched A / registry alone):"
echo "  bash tools/hitl_c_pass_apply.sh --assume-l3-already-open"
echo ""
echo "Honesty: B ≠ ran_on_npu. Offline dump ≠ live XRT BO. CONTRACT_OK ≠ C-ready. unpatched A ≠ C-ready."
# Single-owner lab window: do not run concurrent L3 scripts.
