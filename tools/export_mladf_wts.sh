#!/usr/bin/env bash
# Build + run export_mladf_wts against local Ryzen AI DynamicDispatch (lab only).
#
# Produces a **real** AMD-shuffled WTS blob (174080 B, non-zero) via vendor
# `mladfmatmulbias::export_const_params`. Not a pure-Rust fake shuffle.
# Does **not** claim silicon Pass / ran_on_npu.
#
# Requirements (fail-closed with clear errors if missing):
#   - Ryzen AI DD: $DD_ROOT (default under ryzen-ai-install venv)
#   - System XRT: -lxrt_coreutil -lxrt++ -lxrt_core
#   - libaiebu: /usr/lib/libaiebu.a (or AIEBU_LIB)
#
# Usage:
#   bash tools/export_mladf_wts.sh [OUT_DIR]
#
# After success:
#   bash tools/validate_mladf_wts.sh $OUT_DIR/wts.bin
#   bash tools/stage_silicon_lab_assets.sh --wts-from $OUT_DIR/wts.bin
#   export XDNA2_LAB_WTS_PACKED=$OUT_DIR/wts.bin
#   export XDNA2_LAB_REQUIRE_NONZERO_WTS=1
#
# Do not commit proprietary wts.bin to public git (gitignored).
set -euo pipefail
export PATH="/usr/bin:${HOME}/.local/bin:${PATH}"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DD_ROOT="${DD_ROOT:-$HOME/Projects/ai/ryzen-ai-install/venv/lib/python3.12/site-packages/ryzenai_dynamic_dispatch}"
INC="${DD_ROOT}/include/ryzenai/dynamic_dispatch"
LIB="${DD_ROOT}/lib"
AIEBU_LIB="${AIEBU_LIB:-/usr/lib/libaiebu.a}"
OUT_BIN="${ROOT}/target/export_mladf_wts"
OUT_DIR="${1:-$ROOT/testdata/xdna2_cmd/mladf_wts_export}"

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  sed -n '1,35p' "$0" | sed 's/^# \?//'
  exit 0
fi

fail() { echo "ERROR: $*" >&2; exit 1; }

[[ -d "$DD_ROOT" ]] || fail "DD_ROOT not found: $DD_ROOT (install Ryzen AI / set DD_ROOT)"
[[ -f "$INC/ops/mladfmatmulbias/mladfmatmulbias.hpp" ]] || fail "missing mladfmatmulbias.hpp under $INC"
[[ -f "$LIB/libdyn_dispatch_core.a" ]] || fail "missing $LIB/libdyn_dispatch_core.a"
[[ -f "$AIEBU_LIB" ]] || fail "missing libaiebu at $AIEBU_LIB (install aiebu / set AIEBU_LIB)"

# XRT must be on the default linker path (package xrt / xrt-dev).
XRT_PROBE="$(mktemp "${TMPDIR:-/tmp}/dragon_npu_xrt_probe.XXXXXX")"
trap 'rm -f "$XRT_PROBE"' EXIT
if ! echo 'int main(){return 0;}' | c++ -x c++ - -lxrt_coreutil -o "$XRT_PROBE" 2>/dev/null; then
  fail "cannot link -lxrt_coreutil (install XRT development libraries)"
fi
rm -f "$XRT_PROBE"
trap - EXIT

mkdir -p "$(dirname "$OUT_BIN")" "$OUT_DIR"

echo "compiling export_mladf_wts (DD + XRT + aiebu; ~heavy link) …"
# -O1: vendor headers emit many -Wunused under -Wall; keep our tool clean.
# Do not use -Werror (DD headers are not clean).
c++ -std=c++17 -O1 \
  -I"$INC" \
  -I"${DD_ROOT}/include" \
  "${ROOT}/tools/export_mladf_wts.cpp" \
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
  || fail "link failed — see docs/SILICON_PATH_LAB.md Phase A (DD+XRT+aiebu)"

echo "running: $OUT_BIN '$OUT_DIR'"
"$OUT_BIN" "$OUT_DIR"

# Fail-closed size + non-zero (shell double-check; C++ already gates).
WTS="$OUT_DIR/wts.bin"
[[ -f "$WTS" ]] || fail "missing $WTS"
got="$(wc -c <"$WTS" | tr -d ' ')"
[[ "$got" == "174080" ]] || fail "wts.bin size $got != 174080"
cmp_rc=0
cmp -n 174080 -s -- "$WTS" /dev/zero || cmp_rc=$?
if [[ $cmp_rc -eq 0 ]]; then
  fail "wts.bin is all zeros after export"
fi
if [[ $cmp_rc -ne 1 ]]; then
  fail "zero-check failed for $WTS (cmp exit $cmp_rc)"
fi
[[ -f "$OUT_DIR/PROVENANCE.md" ]] || fail "missing PROVENANCE.md"

echo
echo "AMD-packed WTS OK (lab synthetic host tensors → vendor shuffle)."
echo "  $WTS"
echo "Validate + stage:"
echo "  bash tools/validate_mladf_wts.sh $WTS"
echo "  bash tools/stage_silicon_lab_assets.sh --wts-from $WTS"
echo "  export XDNA2_LAB_WTS_PACKED=$WTS"
echo "  export XDNA2_LAB_REQUIRE_NONZERO_WTS=1"
echo "Packing alone never sets ran_on_npu."
