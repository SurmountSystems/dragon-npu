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

while [[ $# -gt 0 ]]; do
  case "$1" in
    --install) INSTALL=1; shift ;;
    -h|--help)
      sed -n '1,25p' "$0" | sed 's/^# \?//'
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

fail() { echo "ERROR: $*" >&2; exit 1; }
warn() { echo "WARN: $*" >&2; }

[[ -d "$DD_ROOT" ]] || fail "DD_ROOT not found: $DD_ROOT"
[[ -f "$INC/ops/mladfmatmulbias/mladfmatmulbias.hpp" ]] || fail "missing mladfmatmulbias.hpp"
[[ -f "$LIB/libdyn_dispatch_core.a" ]] || fail "missing libdyn_dispatch_core.a"
[[ -f "$AIEBU_LIB" ]] || fail "missing libaiebu at $AIEBU_LIB"

XRT_PROBE="$(mktemp "${TMPDIR:-/tmp}/dragon_npu_xrt_probe.XXXXXX")"
trap 'rm -f "$XRT_PROBE"' EXIT
if ! echo 'int main(){return 0;}' | c++ -x c++ - -lxrt_coreutil -o "$XRT_PROBE" 2>/dev/null; then
  fail "cannot link -lxrt_coreutil (install XRT)"
fi
rm -f "$XRT_PROBE"
trap - EXIT

# Preflight: hardened L3 often blocks vendor EXEC
ALLOW_PATH="/sys/module/amdxdna/parameters/exec_cmd_allow"
if [[ -r "$ALLOW_PATH" ]]; then
  allow_val="$(tr -d ' \n' <"$ALLOW_PATH" || true)"
  echo "preflight: exec_cmd_allow=${allow_val:-?} ($ALLOW_PATH)"
  if [[ "${allow_val}" == "0" || "${allow_val}" == "N" || "${allow_val}" == "n" ]]; then
    warn "exec_cmd_allow is closed — vendor NPU execute often fails until temporary open:"
    warn "  echo 1 | sudo tee $ALLOW_PATH"
    warn "  bash tools/capture_mladf_ofm_golden.sh --install"
    warn "  echo 0 | sudo tee $ALLOW_PATH   # always restore"
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
  echo "  1) Temporary L3: echo 1 | sudo tee $ALLOW_PATH" >&2
  echo "  2) Re-run: bash tools/capture_mladf_ofm_golden.sh --install" >&2
  echo "  3) Restore:  echo 0 | sudo tee $ALLOW_PATH" >&2
  echo "  4) diagnose: bash scripts/diagnose_amdxdna.sh" >&2
  exit "$rc"
fi

OFM="$OUT_DIR/ofm_golden.bin"
[[ -f "$OFM" ]] || fail "missing $OFM after capture"
bash "$ROOT/tools/validate_mladf_ofm_golden.sh" "$OFM"

echo
echo "Option A capture OK (vendor NPU OFM)."
echo "  $OFM"

if [[ "$INSTALL" -eq 1 ]]; then
  echo "Installing into silicon_lab …"
  bash "$ROOT/tools/silicon_lab_operator.sh" install-golden "$OFM"
  echo
  echo "Preflight (Pass assets):"
  bash "$ROOT/tools/silicon_lab_preflight.sh" --require-pass-assets --print-env || true
fi

echo
echo "If L3 was opened for this capture, restore now:"
echo "  echo 0 | sudo tee $ALLOW_PATH"
echo "Packing/capture alone never sets dragon-npu ran_on_npu."
