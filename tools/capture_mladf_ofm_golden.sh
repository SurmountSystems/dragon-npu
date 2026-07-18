#!/usr/bin/env bash
# Option A — capture OFM golden via vendor DD+XRT (instrumented).
#
# Builds/runs tools/capture_mladf_ofm_golden.cpp:
#   same lab-synthetic tensors as export_mladf_wts
#   load_xrt=true → NPU execute → ofm_golden.bin (512 B bf16)
#
# Does **not** set XDNA2_ALLOW_EXEC_CMD or leave exec_cmd_allow=1.
# Prints clear L3 hints if capture fails.
#
# Usage:
#   bash tools/capture_mladf_ofm_golden.sh [OUT_DIR]
#   bash tools/capture_mladf_ofm_golden.sh --install   # also install into silicon_lab
#
# Exit:
#   0  capture OK (and L3 closed, or XDNA2_CAPTURE_ALLOW_L3_OPEN=1)
#   1  capture/run/validate fail (and L3 closed)
#   2  usage / missing deps (and L3 closed)
#   5  exec_cmd_allow still open after run — restore L3 (even if capture failed)
#      (override: XDNA2_CAPTURE_ALLOW_L3_OPEN=1 → keep base rc with loud WARN)
#
# L3 policy matches capture_mladf_instr_vendor.sh (EXIT trap / any rc → 5 if open).
#
# After success:
#   bash tools/silicon_lab_operator.sh install-golden $OUT/ofm_golden.bin
#   bash tools/silicon_lab_operator.sh preflight
set -euo pipefail
export PATH="/usr/bin:${HOME}/.local/bin:${PATH}"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DD_ROOT="${DD_ROOT:-$HOME/Projects/ai/ryzen-ai-install/venv/lib/python3.12/site-packages/ryzenai_dynamic_dispatch}"
INC="${DD_ROOT}/include/ryzenai/dynamic_dispatch"
LIB="${DD_ROOT}/lib"
AIEBU_LIB="${AIEBU_LIB:-/usr/lib/libaiebu.a}"
OUT_BIN="${ROOT}/target/capture_mladf_ofm_golden"
OUT_DIR="${ROOT}/testdata/xdna2_cmd/mladf_ofm_capture"
INSTALL=0
ALLOW_PATH="/sys/module/amdxdna/parameters/exec_cmd_allow"
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

# Prefer exit 5 when L3 still open (any base rc). Research: XDNA2_CAPTURE_ALLOW_L3_OPEN=1.
# Status on stderr; ONLY numeric rc on stdout.
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

# EXIT trap: fail/die2/success all elevate if L3 open (match capture-instr).
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

# Help already exited without trap. Install L3 trap for all remaining paths.
trap _on_exit_l3 EXIT

[[ -d "$DD_ROOT" ]] || die2 "DD_ROOT not found: $DD_ROOT"
[[ -f "$INC/ops/mladfmatmulbias/mladfmatmulbias.hpp" ]] || die2 "missing mladfmatmulbias.hpp"
[[ -f "$LIB/libdyn_dispatch_core.a" ]] || die2 "missing libdyn_dispatch_core.a"
[[ -f "$AIEBU_LIB" ]] || die2 "missing libaiebu at $AIEBU_LIB"

# XRT probe (cleanup must not clobber L3 EXIT trap).
XRT_PROBE="$(mktemp "${TMPDIR:-/tmp}/dragon_npu_xrt_probe.XXXXXX")"
if ! echo 'int main(){return 0;}' | c++ -x c++ - -lxrt_coreutil -o "$XRT_PROBE" 2>/dev/null; then
  rm -f "$XRT_PROBE"
  die2 "cannot link -lxrt_coreutil (install XRT)"
fi
rm -f "$XRT_PROBE"

if [[ -r "$ALLOW_PATH" ]]; then
  allow_val="$(tr -d ' \n' <"$ALLOW_PATH" || true)"
  echo "preflight: exec_cmd_allow=${allow_val:-?} ($ALLOW_PATH)"
  if [[ "${allow_val}" == "0" || "${allow_val}" == "N" || "${allow_val}" == "n" ]]; then
    warn "exec_cmd_allow is closed — vendor NPU execute often fails until temporary open:"
    warn "  sudo sh -c 'echo 1 > $ALLOW_PATH'"
    warn "  bash tools/capture_mladf_ofm_golden.sh --install"
    warn "  sudo sh -c 'echo 0 > $ALLOW_PATH'   # always restore"
  fi
else
  warn "no $ALLOW_PATH (module not loaded?) — capture may still fail"
fi

mkdir -p "$(dirname "$OUT_BIN")" "$OUT_DIR"

echo "compiling capture_mladf_ofm_golden (DD + XRT + aiebu; load_xrt path) …"
c++ -std=c++17 -O1 \
  -I"$INC" \
  -I"${DD_ROOT}/include" \
  "${ROOT}/tools/capture_mladf_ofm_golden.cpp" \
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
  || fail "link failed"

echo "running: $OUT_BIN '$OUT_DIR'"
set +e
"$OUT_BIN" "$OUT_DIR"
rc=$?
set -e
if [[ $rc -ne 0 ]]; then
  echo "" >&2
  echo "CAPTURE_FAILED (exit $rc). Common fixes:" >&2
  echo "  1) Temporary L3: sudo sh -c 'echo 1 > $ALLOW_PATH'" >&2
  echo "  2) Re-run: bash tools/capture_mladf_ofm_golden.sh --install" >&2
  echo "  3) Restore:  sudo sh -c 'echo 0 > $ALLOW_PATH'" >&2
  echo "  4) diagnose: bash scripts/diagnose_amdxdna.sh" >&2
  _CAPTURE_BASE_RC="$rc"
  exit "$rc"   # trap elevates to 5 if L3 open
fi

OFM="$OUT_DIR/ofm_golden.bin"
[[ -f "$OFM" ]] || fail "missing $OFM after capture"
bash "$ROOT/tools/validate_mladf_ofm_golden.sh" "$OFM" || fail "validate_mladf_ofm_golden failed"

echo
echo "Option A capture OK (vendor NPU OFM)."
echo "  $OFM"

if [[ "$INSTALL" -eq 1 ]]; then
  echo "Installing into silicon_lab …"
  bash "$ROOT/tools/silicon_lab_operator.sh" install-golden "$OFM" || fail "install-golden failed"
  echo
  echo "Preflight (Pass assets):"
  # Soft: do not fail capture if preflight warns; L3 still checked on exit.
  bash "$ROOT/tools/silicon_lab_preflight.sh" --require-pass-assets --print-env || true
fi

echo "Packing/capture alone never sets dragon-npu ran_on_npu."
_CAPTURE_BASE_RC=0
exit 0
