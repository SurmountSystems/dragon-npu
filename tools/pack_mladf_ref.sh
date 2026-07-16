#!/usr/bin/env bash
# Build + run pack_mladf_ref (staged mladf a16w4 host buffers).
#
# Produces gitignored lab fixtures under testdata/xdna2_cmd/mladf_ref/:
#   ifm.bin (2304)  wts.bin (174080)  ofm_expected.bin (512)  PROVENANCE.md
#
# IFM is a correct bf16 ones pack. WTS is zero-filled unless --wts-from is a
# size-correct AMD-packed file. Never invents a fake AMD weight shuffle.
#
# Optional link path mirrors dump_dd_txn only for DD presence check; this tool
# is pure C++ (no libtransaction / XRT required to pack sizes).
#
# Usage:
#   bash tools/pack_mladf_ref.sh [OUT_DIR] [--wts-from PATH]
#
# Env:
#   XDNA2_PACK_WTS_FROM  same as --wts-from
#   DD_ROOT             only used to print DD install hint (optional)
#
# After success (lab):
#   export XDNA2_LAB_PACK=mladf_a16w4
#   export XDNA2_LAB_WTS_PACKED=$PWD/testdata/xdna2_cmd/mladf_ref/wts.bin
set -euo pipefail
export PATH="/usr/bin:${HOME}/.local/bin:${PATH}"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_BIN="${ROOT}/target/pack_mladf_ref"
OUT_DIR="${ROOT}/testdata/xdna2_cmd/mladf_ref"
WTS_FROM="${XDNA2_PACK_WTS_FROM:-}"
EXTRA_ARGS=()

# Parse: optional OUT_DIR positional, optional --wts-from
while [[ $# -gt 0 ]]; do
  case "$1" in
    -h|--help)
      echo "Usage: bash tools/pack_mladf_ref.sh [OUT_DIR] [--wts-from PATH]"
      exit 0
      ;;
    --wts-from)
      shift
      [[ $# -ge 1 ]] || { echo "ERROR: --wts-from needs PATH" >&2; exit 2; }
      WTS_FROM="$1"
      shift
      ;;
    *)
      if [[ "$1" == -* ]]; then
        echo "ERROR: unknown flag $1" >&2
        exit 2
      fi
      OUT_DIR="$1"
      shift
      ;;
  esac
done

mkdir -p "$(dirname "$OUT_BIN")" "$OUT_DIR"

echo "compiling pack_mladf_ref (pure C++17; no AMD link) …"
c++ -std=c++17 -O2 -Wall -Wextra \
  "${ROOT}/tools/pack_mladf_ref.cpp" \
  -o "$OUT_BIN"

if [[ -n "$WTS_FROM" ]]; then
  EXTRA_ARGS+=(--wts-from "$WTS_FROM")
fi

echo "running: $OUT_BIN '$OUT_DIR' ${EXTRA_ARGS[*]:-}"
"$OUT_BIN" "$OUT_DIR" "${EXTRA_ARGS[@]+"${EXTRA_ARGS[@]}"}"

# Fail-closed size gate (also done in C++; double-check for shell consumers).
check_size() {
  local f="$1" expect="$2"
  local got
  got="$(wc -c <"$f" | tr -d ' ')"
  if [[ "$got" != "$expect" ]]; then
    echo "ERROR: $f size $got != $expect" >&2
    exit 1
  fi
}
check_size "$OUT_DIR/ifm.bin" 2304
check_size "$OUT_DIR/wts.bin" 174080
check_size "$OUT_DIR/ofm_expected.bin" 512
[[ -f "$OUT_DIR/PROVENANCE.md" ]] || { echo "ERROR: missing PROVENANCE.md" >&2; exit 1; }

echo
echo "Sizes OK. Wire lab env (WTS may be zeros — not Pass weights):"
echo "  export XDNA2_LAB_OFM=1"
echo "  export XDNA2_LAB_PACK=mladf_a16w4"
echo "  export XDNA2_LAB_WTS_PACKED=$OUT_DIR/wts.bin"
echo "  # ofm_expected.bin is all zeros — refused as Pass golden by default"
echo "  # export XDNA2_LAB_OFM_GOLDEN=/path/to/nonzero_ofm.bin"
echo "  # still need known-good txn + L1/L3 gates for EXEC; packing alone ≠ ran_on_npu"
