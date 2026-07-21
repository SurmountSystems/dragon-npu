#!/usr/bin/env bash
# Dump opaque PDI section from a public xclbin (offline research).
#
# Purpose (path A machinery): produce a real PDI blob for
#   XDNA2_LAB_PDI_BLOB=/path/to/pdi.bin
# so pure-direct can issue CONFIG_HWCTX(CONFIG_CU) after CREATE_HWCTX.
#
# Honesty:
#   - Output is **opaque** research bytes (no PDI field decode).
#   - CVML / AIE2P overlay PDI ≠ mladf matmul known-good corpus.
#   - Dump ≠ Pass ≠ ran_on_npu ≠ C unlock.
#
# Usage:
#   bash tools/dump_xclbin_pdi.sh
#   bash tools/dump_xclbin_pdi.sh --xclbin /path/to.xclbin
#   bash tools/dump_xclbin_pdi.sh --out /tmp/research_pdi.bin
#   bash tools/dump_xclbin_pdi.sh --print-env   # print export lines after dump
#   bash tools/dump_xclbin_pdi.sh --list-default  # show candidate xclbins only
#
# Exit: 0 ok, 2 bad args / missing tool / missing input, 3 dump failed
set -euo pipefail
export PATH="/usr/bin:${HOME}/.local/bin:${PATH}"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

DEFAULT_XCLBIN_CANDIDATES=(
  "${HOME}/Projects/ai/RyzenAI-SW/Ryzen-AI-CVML-Library/linux/onnx/ryzen14/AMD_AIE2P_4x4_Overlay.xclbin"
  "${HOME}/Projects/ai/RyzenAI-SW/Ryzen-AI-CVML-Library/linux/onnx/ryzen14/AMD_AIE2P_4x4_Overlay_CFG0.xclbin"
  "${HOME}/Projects/ai/RyzenAI-SW/Ryzen-AI-CVML-Library/linux/onnx/ryzen14/AMD_AIE2P_8x4x1_Overlay.xclbin"
)

XCLBIN=""
OUT=""
PRINT_ENV=0
LIST_ONLY=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --xclbin)
      XCLBIN="${2:-}"
      shift 2
      ;;
    --out)
      OUT="${2:-}"
      shift 2
      ;;
    --print-env)
      PRINT_ENV=1
      shift
      ;;
    --list-default|--list)
      LIST_ONLY=1
      shift
      ;;
    -h|--help)
      grep -E '^#([^!]|$)' "$0" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      echo "ERROR: unknown arg: $1" >&2
      echo "allowlist: --xclbin PATH --out PATH --print-env --list-default" >&2
      exit 2
      ;;
  esac
done

if [[ "$LIST_ONLY" -eq 1 ]]; then
  echo "=== Default xclbin candidates (exist?) ==="
  for c in "${DEFAULT_XCLBIN_CANDIDATES[@]}"; do
    if [[ -f "$c" ]]; then
      echo "EXISTS  $c  ($(wc -c <"$c") bytes)"
    else
      echo "MISSING $c"
    fi
  done
  exit 0
fi

if [[ -z "$XCLBIN" ]]; then
  for c in "${DEFAULT_XCLBIN_CANDIDATES[@]}"; do
    if [[ -f "$c" ]]; then
      XCLBIN="$c"
      break
    fi
  done
fi

if [[ -z "$XCLBIN" || ! -f "$XCLBIN" ]]; then
  echo "ERROR: no xclbin found. Pass --xclbin PATH or install RyzenAI-SW overlay." >&2
  echo "  try: bash tools/dump_xclbin_pdi.sh --list-default" >&2
  exit 2
fi

if ! command -v xclbinutil >/dev/null 2>&1; then
  echo "ERROR: xclbinutil not on PATH (need XRT tools)." >&2
  exit 2
fi

if [[ -z "$OUT" ]]; then
  OUT_DIR="${TMPDIR:-/tmp}"
  # Refuse TMPDIR=/ only (same clamp as other lab tools).
  if [[ "$OUT_DIR" == "/" ]]; then
    OUT_DIR="/tmp"
  fi
  OUT="${OUT_DIR}/dragon_npu_research_pdi_$$.bin"
fi

if [[ -e "$OUT" ]]; then
  echo "ERROR: refuse clobber existing OUT=$OUT (delete or pick --out)." >&2
  exit 2
fi

echo "=== dump_xclbin_pdi (research only; not known-good matmul) ==="
echo "xclbin=$XCLBIN"
echo "out=$OUT"

if ! xclbinutil --force --dump-section "PDI:raw:${OUT}" --input "$XCLBIN" >/tmp/dragon_npu_xclbinutil_pdi_$$.log 2>&1; then
  echo "ERROR: xclbinutil dump failed:" >&2
  cat /tmp/dragon_npu_xclbinutil_pdi_$$.log >&2 || true
  rm -f /tmp/dragon_npu_xclbinutil_pdi_$$.log
  exit 3
fi
rm -f /tmp/dragon_npu_xclbinutil_pdi_$$.log

if [[ ! -f "$OUT" ]]; then
  echo "ERROR: dump claimed success but OUT missing: $OUT" >&2
  exit 3
fi

BYTES=$(wc -c <"$OUT" | tr -d ' ')
if [[ "$BYTES" -eq 0 ]]; then
  echo "ERROR: empty PDI dump" >&2
  exit 3
fi

SHA=$(sha256sum "$OUT" | awk '{print $1}')
echo "PDI_OK bytes=$BYTES sha256=$SHA"
echo "NOTE: opaque research blob only — invent_pass=0 not_ran_on_npu=1 product_known_good=0"

if [[ "$PRINT_ENV" -eq 1 ]]; then
  echo
  echo "# --- paste for lab CONFIG_CU path (still needs EXEC double-gate + L3 for live) ---"
  echo "export XDNA2_LAB_PDI_BLOB=$OUT"
  echo "export XDNA2_LAB_CU_FUNC=0   # optional; default 0"
  echo "# fish:"
  echo "set -gx XDNA2_LAB_PDI_BLOB $OUT"
  echo "set -gx XDNA2_LAB_CU_FUNC 0"
fi

exit 0
