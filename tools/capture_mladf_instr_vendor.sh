#!/usr/bin/env bash
# Capture vendor instruction BO after a working DD+XRT mladf matmul.
#
# Same deps/build style as capture_mladf_ofm_golden.sh.
# Does NOT set XDNA2_ALLOW_EXEC_CMD. Restore L3 yourself after.
#
# Usage:
#   bash tools/capture_mladf_instr_vendor.sh
#   bash tools/capture_mladf_instr_vendor.sh --install   # note: already under gt_capture
#   bash tools/silicon_lab_operator.sh capture-instr      # preferred one-liner
#
# Exit:
#   0  capture OK (OFM non-zero + instr ≥1688 non-zero) and L3 closed (or ALLOW override)
#   1  capture/run fail (OFM zero, link fail, missing dump) and L3 closed
#   2  usage / missing deps (L3 elevate still applies if open after pre-run work)
#   5  exec_cmd_allow still open after run — restore L3 (even if capture failed)
#      (override: XDNA2_CAPTURE_ALLOW_L3_OPEN=1 → keep base rc with loud WARN)
#
# HITL 3378dd7b root cause (fixed): get_transaction_bin needs inputs={IFM,WTS};
# IFM-only hit fit_MKN vector range_check. Always also dumps Transaction map
# unpatched (libtransaction) so registry miss still yields ≥1688 fallback.
# HITL false CLASS_HINT C: get_transaction_bin may return multi-txn ~32k (wrong
# body); classic 1688 map is preferred as instr_vendor_txn_bin + unpatched
# post_patch; oversized goes to instr_vendor_get_transaction_bin.bin only.
#
# Env:
#   XDNA2_CAPTURE_ALLOW_L3_OPEN=1  allow exit 0 when L3 still open (research only)
#
# Then offline:
#   bash tools/diff_vendor_instr_parity.sh
set -euo pipefail
export PATH="/usr/bin:${HOME}/.local/bin:${PATH}"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DD_ROOT="${DD_ROOT:-$HOME/Projects/ai/ryzen-ai-install/venv/lib/python3.12/site-packages/ryzenai_dynamic_dispatch}"
INC="${DD_ROOT}/include/ryzenai/dynamic_dispatch"
LIB="${DD_ROOT}/lib"
AIEBU_LIB="${AIEBU_LIB:-/usr/lib/libaiebu.a}"
OUT_BIN="${ROOT}/target/capture_mladf_instr_vendor"
OUT_DIR="${ROOT}/testdata/xdna2_cmd/gt_capture"
INSTALL=0
ALLOW_PATH="/sys/module/amdxdna/parameters/exec_cmd_allow"
# Base exit before L3 adjust; trap routes all exits through elevate (Issue 3).
_CAPTURE_BASE_RC=

while [[ $# -gt 0 ]]; do
  case "$1" in
    --install) INSTALL=1; shift ;;
    -h|--help)
      sed -n '1,30p' "$0" | sed 's/^# \?//'
      exit 0
      ;;
    -*)
      echo "ERROR: unknown flag $1" >&2
      exit 2
      ;;
    *)
      OUT_DIR="$1"
      shift
      ;;
  esac
done

warn() { echo "WARN: $*" >&2; }
note() { echo "$*"; }

# L3 post-check helper: prefer exit 5 when still open (even on capture fail).
# Override research-only: XDNA2_CAPTURE_ALLOW_L3_OPEN=1
# Prints status on stderr; ONLY the numeric rc on stdout (for $(...)).
l3_open_exit_adjust() {
  local base_rc="$1"
  if [[ ! -r "$ALLOW_PATH" ]]; then
    printf '%s\n' "$base_rc"
    return
  fi
  local allow_post
  allow_post="$(tr -d ' \n' <"$ALLOW_PATH" || true)"
  case "$allow_post" in
    0|N|n)
      echo "POST: exec_cmd_allow=$allow_post (closed — good)" >&2
      printf '%s\n' "$base_rc"
      ;;
    *)
      echo "WARN: POST: exec_cmd_allow=$allow_post still open — restore NOW:" >&2
      echo "WARN:   sudo sh -c 'echo 0 > $ALLOW_PATH'" >&2
      if [[ "${XDNA2_CAPTURE_ALLOW_L3_OPEN:-0}" == "1" ]]; then
        echo "WARN: XDNA2_CAPTURE_ALLOW_L3_OPEN=1 — not elevating to exit 5" >&2
        printf '%s\n' "$base_rc"
      else
        echo "WARN: CRITICAL: L3 still open → exit 5 (base was $base_rc)" >&2
        printf '%s\n' "5"
      fi
      ;;
  esac
}

# EXIT trap: every path (fail/die2/success) goes through L3 elevate (Issue 3).
# Disable trap before re-exit so nested EXIT cannot reset the elevated rc.
_on_exit_l3() {
  local raw=$?
  trap - EXIT
  local base="$raw"
  if [[ -n "${_CAPTURE_BASE_RC:-}" ]]; then
    base="$_CAPTURE_BASE_RC"
  fi
  local final
  final="$(l3_open_exit_adjust "$base")"
  exit "$final"
}

fail() {
  echo "ERROR: $*" >&2
  _CAPTURE_BASE_RC=1
  exit 1
}
die2() {
  echo "ERROR: $*" >&2
  _CAPTURE_BASE_RC=2
  exit 2
}

# Install trap after helpers; help already exited above without trap.
trap _on_exit_l3 EXIT

[[ -d "$DD_ROOT" ]] || die2 "DD_ROOT not found: $DD_ROOT"
[[ -f "$INC/ops/mladfmatmulbias/mladfmatmulbias.hpp" ]] || die2 "missing mladfmatmulbias.hpp"
[[ -f "$LIB/libdyn_dispatch_core.a" ]] || die2 "missing libdyn_dispatch_core.a"
[[ -f "$AIEBU_LIB" ]] || die2 "missing libaiebu at $AIEBU_LIB"

if [[ -r "$ALLOW_PATH" ]]; then
  allow_val="$(tr -d ' \n' <"$ALLOW_PATH" || true)"
  note "preflight: exec_cmd_allow=${allow_val:-?} ($ALLOW_PATH)"
  if [[ "${allow_val}" == "0" || "${allow_val}" == "N" || "${allow_val}" == "n" ]]; then
    warn "exec_cmd_allow closed — open L3 for vendor NPU execute:"
    warn "  sudo sh -c 'echo 1 > $ALLOW_PATH'"
    warn "  bash tools/capture_mladf_instr_vendor.sh"
    warn "  sudo sh -c 'echo 0 > $ALLOW_PATH'"
    warn "(continuing — run may fail with OFM zeros if L3 stays closed)"
  fi
else
  warn "exec_cmd_allow path not readable — cannot preflight L3"
fi

mkdir -p "$(dirname "$OUT_BIN")" "$OUT_DIR"

# Include path mirrors dump_dd_txn.sh (txn/txn_container.hpp + ops headers).
[[ -f "$INC/txn/txn_container.hpp" ]] || die2 "missing $INC/txn/txn_container.hpp (need Transaction map dump)"

note "compiling capture_mladf_instr_vendor …"
c++ -std=c++17 -O1 \
  -I"$INC" \
  -I"${DD_ROOT}/include" \
  "${ROOT}/tools/capture_mladf_instr_vendor.cpp" \
  -o "$OUT_BIN" \
  -Wl,--start-group \
    "${LIB}/libdyn_dispatch_core.a" \
    "${LIB}/libtransaction.a" \
    "${LIB}/libxaiengine.a" \
    "${LIB}/libxclbin.a" \
    "${LIB}/libhashlib.a" \
    "${LIB}/libpm_package.a" \
    "${AIEBU_LIB}" \
  -Wl,--end-group \
  -lxrt_coreutil -lxrt++ -lxrt_core -luuid -lz -lpthread -ldl \
  || fail "link failed (exit 1)"

note "running: $OUT_BIN '$OUT_DIR'"
set +e
"$OUT_BIN" "$OUT_DIR"
rc=$?
set -e
if [[ $rc -ne 0 ]]; then
  echo "CAPTURE_FAILED (exit $rc)." >&2
  echo "If hardened L3 was closed, open → re-run → restore:" >&2
  echo "  sudo sh -c 'echo 1 > $ALLOW_PATH'" >&2
  echo "  bash tools/silicon_lab_operator.sh capture-instr" >&2
  echo "  # or: bash tools/capture_mladf_instr_vendor.sh" >&2
  echo "  sudo sh -c 'echo 0 > $ALLOW_PATH'" >&2
  echo "HINT: get_transaction_bin needs IFM+WTS; Transaction map classic 1688" >&2
  echo "  preferred over multi-txn ~32k (diag-only get_transaction_bin dump)." >&2
  _CAPTURE_BASE_RC="$rc"
  exit "$rc"   # trap elevates to 5 if L3 open
fi

INSTR="$OUT_DIR/instr_vendor_post_patch.bin"
[[ -f "$INSTR" ]] || fail "missing $INSTR"
isz="$(wc -c <"$INSTR" | tr -d ' ')"
[[ "$isz" -ge 1688 ]] || fail "instr size $isz < 1688"
# Issue 10: refuse all-zero post_patch (python for portable nonzero).
if ! python3 -c "
import sys
p=sys.argv[1]
d=open(p,'rb').read()
sys.exit(0 if any(b!=0 for b in d) else 1)
" "$INSTR" 2>/dev/null; then
  fail "instr_vendor_post_patch.bin is all-zero (nonzero check)"
fi
note "OK: vendor instr $isz B → $INSTR"

if [[ -f "$OUT_DIR/instr_vendor_txn_bin.bin" ]]; then
  tsz="$(wc -c <"$OUT_DIR/instr_vendor_txn_bin.bin" | tr -d ' ')"
  note "OK: txn bin (unpatched label) $tsz B → $OUT_DIR/instr_vendor_txn_bin.bin"
fi
if [[ -f "$OUT_DIR/vendor_paddrs.json" ]]; then
  note "OK: vendor_paddrs.json present"
  # Issue 4: match capture_mode *value* only (not field names / notes).
  # e.g. "capture_mode": "txn_bin_fallback_unpatched"
  if grep -Eq '"capture_mode"[[:space:]]*:[[:space:]]*"txn_(bin_fallback_unpatched|container_unpatched)"' \
      "$OUT_DIR/vendor_paddrs.json" 2>/dev/null; then
    warn "!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!"
    warn "CAPTURE_MODE_UNPATCHED — post_patch file is UNPATCHED fallback"
    warn "  Transaction/cache registry path preferred when available (not proven live run_ BO)"
    warn "  unpatched ≠ C-ready; C only if agent says ready; diff may CLASS_HINT D"
    warn "  File: $OUT_DIR/vendor_paddrs.json"
    warn "!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!"
  fi
fi

if [[ "$INSTALL" -eq 1 ]]; then
  note "Install mode: files already under $OUT_DIR (gt_capture default)"
fi

echo
note "Offline next:"
note "  bash tools/diff_vendor_instr_parity.sh"
note "Honesty: capture ≠ ran_on_npu; unpatched fallback ≠ C-ready; no env HITL until re-diff"
_CAPTURE_BASE_RC=0
exit 0
