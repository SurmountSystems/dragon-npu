#!/usr/bin/env bash
# Stage silicon-path lab assets (offline; no EXEC):
#   - instruction txn + ctrl.bin + ctrl_meta.json via dump_dd_txn
#   - ifm/wts/ofm size fixtures via pack_mladf_ref
#
# Real AMD-packed WTS:
#   --wts-from PATH   (must be 174080 non-zero for Pass readiness)
#   --export-wts      run tools/export_mladf_wts.sh then use that wts.bin
# Zero WTS is length-only plumbing — not product Pass.
#
# Usage:
#   bash tools/stage_silicon_lab_assets.sh [OUT_DIR] [--wts-from PATH | --export-wts]
set -euo pipefail
export PATH="/usr/bin:${HOME}/.local/bin:${PATH}"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="${ROOT}/testdata/xdna2_cmd/silicon_lab"
WTS_FROM="${XDNA2_PACK_WTS_FROM:-}"
EXPORT_WTS=0
KEY='mladfmatmulbias_prmptflat_mladf_2x4x4_flat_a16fw4acc16f_1_1152_256_128'

while [[ $# -gt 0 ]]; do
  case "$1" in
    --wts-from)
      shift
      if [[ $# -lt 1 || -z "${1:-}" || "$1" == -* ]]; then
        echo "ERROR: --wts-from needs a non-empty PATH (not another flag)" >&2
        exit 2
      fi
      WTS_FROM="$1"
      shift
      ;;
    --export-wts) EXPORT_WTS=1; shift ;;
    -h|--help)
      sed -n '1,22p' "$0" | sed 's/^# \?//'
      exit 0
      ;;
    *) OUT="$1"; shift ;;
  esac
done

if [[ "$EXPORT_WTS" -eq 1 ]]; then
  EXPORT_DIR="${ROOT}/testdata/xdna2_cmd/mladf_wts_export"
  bash "$ROOT/tools/export_mladf_wts.sh" "$EXPORT_DIR"
  WTS_FROM="$EXPORT_DIR/wts.bin"
fi

mkdir -p "$OUT"
DUMP="$OUT/dd_dump"
bash "$ROOT/tools/dump_dd_txn.sh" "$KEY" "$DUMP" 1

# Copy staged assets under stable names
cp -a "$DUMP/${KEY}.bin" "$OUT/lab_instr.bin"
cp -a "$DUMP/${KEY}_ctrl.bin" "$OUT/lab_ctrl.bin"
cp -a "$DUMP/${KEY}_ctrl_meta.json" "$OUT/lab_ctrl_meta.json"

# Fail-closed zero gate: cmp first N bytes vs /dev/zero.
# rc 0 = all zero; rc 1 = non-zero content; other = tool failure → refuse.
wts_bytes_all_zero() {
  local f="$1" n="$2" rc=0
  cmp -n "$n" -s -- "$f" /dev/zero || rc=$?
  if [[ $rc -eq 0 ]]; then
    return 0
  fi
  if [[ $rc -eq 1 ]]; then
    return 1
  fi
  echo "ERROR: zero-check failed for $f (cmp exit $rc)" >&2
  exit 2
}

PACK_ARGS=("$OUT")
if [[ -n "$WTS_FROM" ]]; then
  if [[ ! -f "$WTS_FROM" ]]; then
    echo "ERROR: --wts-from not a file: $WTS_FROM" >&2
    exit 2
  fi
  local_sz=$(wc -c <"$WTS_FROM" | tr -d ' ')
  if [[ "$local_sz" != "174080" ]]; then
    echo "ERROR: --wts-from size $local_sz != 174080" >&2
    exit 2
  fi
  if wts_bytes_all_zero "$WTS_FROM" 174080; then
    echo "ERROR: --wts-from is all zeros (not AMD-packed; refuse for Pass staging)" >&2
    exit 2
  fi
  PACK_ARGS+=(--wts-from "$WTS_FROM")
fi
bash "$ROOT/tools/pack_mladf_ref.sh" "${PACK_ARGS[@]}"

# Size gates (exit 2 matches validate_mladf_*.sh validation failures)
check() {
  local f="$1" e="$2" g
  g=$(wc -c <"$f" | tr -d ' ')
  if [[ "$g" != "$e" ]]; then
    echo "ERROR $f size $g != $e" >&2
    exit 2
  fi
}
check "$OUT/lab_instr.bin" 1688
check "$OUT/lab_ctrl.bin" 29664
check "$OUT/ifm.bin" 2304
check "$OUT/wts.bin" 174080
check "$OUT/ofm_expected.bin" 512

# Offline ctrl gate (same contract as tools/validate_mladf_ctrl.sh / Rust)
bash "$ROOT/tools/validate_mladf_ctrl.sh" "$OUT/lab_ctrl.bin"

WTS_ZERO=0
if wts_bytes_all_zero "$OUT/wts.bin" 174080; then
  WTS_ZERO=1
fi

cat > "$OUT/PROVENANCE_SILICON.md" << MD
# Silicon lab staging provenance

- **date_utc:** $(date -u +%Y-%m-%dT%H:%M:%SZ)
- **key:** \`${KEY}\`
- **lab_instr.bin:** 1688 B (classic v0.1 txn from DD dump — **vendor extract**, not hand-written)
- **lab_ctrl.bin:** 29664 B (runtime control packet from DD dump — **vendor extract**, not hand-written microcode)
- **lab_ctrl_meta.json:** sizes + \`control_packet_patch_locations\` (IFM/WTS/OFM paddrs; size=6 read_add_write residual for full XRT fidelity)
- **ifm.bin:** 2304 B bf16 ones (host pack)
- **wts.bin:** 174080 B — $(if [[ "$WTS_ZERO" -eq 1 ]]; then echo 'ALL ZEROS (not AMD-packed; not Pass weights)'; else echo "from ${WTS_FROM:-unknown} (operator-supplied)"; fi)
- **ofm_expected.bin:** 512 B zeros placeholder — **not** Pass golden
- **NOT silicon Pass** until non-zero AMD WTS + non-zero OFM golden + L1/L3 EXEC + allclose
- Ctrl BO attach / kernel-arg count 4 ≠ \`ran_on_npu\`

## Env recipe (dry-run first)

\`\`\`bash
export XDNA2_INSTRUCTION_BLOB=$OUT/lab_instr.bin
export XDNA2_LAB_OFM=1
export XDNA2_LAB_PACK=mladf_a16w4
export XDNA2_LAB_MLADF_KEY='${KEY}'
export XDNA2_LAB_MLADF_META=$OUT/lab_ctrl_meta.json
export XDNA2_LAB_WTS_PACKED=$OUT/wts.bin
export XDNA2_LAB_CTRL_PACKED=$OUT/lab_ctrl.bin
export XDNA2_LAB_REQUIRE_CTRL=1
export XDNA2_LAB_REQUIRE_NONZERO_WTS=1
export XDNA2_LAB_CTRL_AUTO_PATCH=1   # lab u64-LE paddr patch from meta (size=6 read_add_write residual)
# export XDNA2_LAB_OFM_GOLDEN=/path/to/nonzero_bf16_ofm.bin  # required for Pass
bash tools/silicon_lab_preflight.sh --print-env
# Pass-readiness fail-closed:
# bash tools/silicon_lab_preflight.sh --require-pass-assets --print-env
bash scripts/lab_exec_once.sh --silicon-mladf --blob "\$XDNA2_INSTRUCTION_BLOB"
# apply only with operator risk accept + temporary exec_cmd_allow=1
# bash scripts/lab_exec_once.sh --silicon-mladf --apply --i-accept-wedge-risk --blob "\$XDNA2_INSTRUCTION_BLOB"
\`\`\`
MD

echo
echo "Staged under $OUT"
echo "  lab_instr.bin  $(wc -c <"$OUT/lab_instr.bin" | tr -d ' ') B"
echo "  lab_ctrl.bin   $(wc -c <"$OUT/lab_ctrl.bin" | tr -d ' ') B"
echo "  wts.bin        $(wc -c <"$OUT/wts.bin" | tr -d ' ') B  zero=$WTS_ZERO"
echo "See $OUT/PROVENANCE_SILICON.md"
echo "Next: bash tools/silicon_lab_preflight.sh --print-env"
if [[ "$WTS_ZERO" -eq 1 ]]; then
  echo "NOTE: WTS is all zeros — obtain AMD-packed weights via --wts-from before Pass attempts."
fi
