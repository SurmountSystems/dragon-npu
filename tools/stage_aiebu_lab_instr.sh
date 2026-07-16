#!/usr/bin/env bash
# Build aiebu ELF from staged raw txn (+ optional ctrl) for pure-direct EXEC.
#
# XRT does: ml_txn.bin + pm_ctrlpkt → aiebu → ELF → peel .ctrltext + apply .rela.dyn
# with buffer paddrs (patch_ctrl_code). Submitting raw DD txn causes FW
# APP_LOAD_PDI_FAIL (0x3000003) on silicon.
#
# Usage (fish-friendly — run with bash):
#   bash tools/stage_aiebu_lab_instr.sh
#   # writes testdata/xdna2_cmd/silicon_lab/lab_instr.elf
#   # then set:
#   #   export XDNA2_INSTRUCTION_BLOB=$PWD/testdata/xdna2_cmd/silicon_lab/lab_instr.elf
#
set -euo pipefail
export PATH="/usr/bin:${HOME}/.local/bin:${PATH}"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIR="${1:-$ROOT/testdata/xdna2_cmd/silicon_lab}"
TXN="${DIR}/lab_instr.bin"
CTRL="${DIR}/lab_ctrl.bin"
OUT="${DIR}/lab_instr.elf"

AIEBU="${AIEBU_ASM:-}"
if [[ -z "$AIEBU" ]]; then
  for c in \
    "$HOME/Projects/ai/ryzen-ai-install/venv/bin/aiebu-asm" \
    "$HOME/Projects/ai/ryzen-ai-install/venv/lib/python3.12/site-packages/bin/aiebu-asm" \
    /usr/bin/aiebu-asm
  do
    if [[ -x "$c" ]]; then AIEBU="$c"; break; fi
  done
fi

[[ -x "${AIEBU:-}" ]] || {
  echo "ERROR: aiebu-asm not found. Set AIEBU_ASM=/path/to/aiebu-asm" >&2
  exit 1
}
[[ -f "$TXN" ]] || { echo "ERROR: missing $TXN" >&2; exit 1; }

echo "aiebu-asm: $AIEBU"
echo "txn:       $TXN ($(wc -c <"$TXN") B)"
if [[ -f "$CTRL" ]]; then
  echo "ctrl:      $CTRL ($(wc -c <"$CTRL") B)"
  "$AIEBU" -t aie2txn -c "$TXN" -p "$CTRL" -o "$OUT"
else
  echo "ctrl:      (none — txn only)"
  "$AIEBU" -t aie2txn -c "$TXN" -o "$OUT"
fi

echo "out:       $OUT ($(wc -c <"$OUT") B)"
if command -v readelf >/dev/null 2>&1; then
  readelf -S "$OUT" 2>/dev/null | grep -E 'ctrltext|ctrldata|rela' || true
fi
echo
echo "Use with pure-direct:"
echo "  export XDNA2_INSTRUCTION_BLOB=$OUT"
echo "  # lab_exec_once --silicon-mladf will keep packing env; blob path above"
echo "Honesty: ELF alone ≠ ran_on_npu; Rust applies .rela.dyn at EXEC with live paddrs."
