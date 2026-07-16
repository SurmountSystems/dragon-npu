#!/usr/bin/env bash
# Offline materialize of research XAie txn / aiebu ELF — NO EXEC_CMD, no Pass claim.
#
# Prefer pure Rust synthetic NOOP for CI. Optionally wrap vendor copy_* bins (if
# present under a local Ryzen AI install) via aiebu-asm — those are lab-only and
# stay gitignored.
#
# Usage:
#   bash scripts/materialize_txn_offline.sh              # synthetic NOOP → testdata
#   bash scripts/materialize_txn_offline.sh --from-aiebu  # if aiebu + copy bin exist
#   bash scripts/materialize_txn_offline.sh --help
set -euo pipefail
export PATH="/usr/bin:${HOME}/.local/bin:${PATH}"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="${ROOT}/testdata/xdna2_cmd"
MODE="synthetic"
FORCE=0

usage() {
  sed -n '1,20p' "$0" | sed 's/^# \?//'
  exit 0
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --from-aiebu) MODE=aiebu; shift ;;
    --force) FORCE=1; shift ;;
    -h|--help) usage ;;
    *) echo "unknown arg: $1" >&2; exit 2 ;;
  esac
done

mkdir -p "$OUT_DIR"
PROV="${OUT_DIR}/PROVENANCE_txn.md"
BLOB="${OUT_DIR}/lab_instr.bin"
ELF="${OUT_DIR}/lab_instr.elf"

if [[ -f "$BLOB" && "$FORCE" -ne 1 ]]; then
  echo "exists: $BLOB (pass --force to overwrite)"
  ls -la "$BLOB" || true
  exit 0
fi

write_prov() {
  local kind="$1"
  local note="$2"
  cat > "$PROV" << MD
# Instruction blob provenance (lab only)

- **date_utc:** $(date -u +%Y-%m-%dT%H:%M:%SZ)
- **kind:** ${kind}
- **NOT_AIE2_KNOWN_GOOD_GEMM:** 1
- **note:** ${note}

Do **not** claim \`ran_on_npu\` / silicon GEMM Pass from this file alone.
See docs/XDNA2_TXN_RESEARCH.md and docs/XDNA2_NEVER_AGAIN.md.
MD
}

if [[ "$MODE" == "synthetic" ]]; then
  # Pure Rust builder — no vendor bits, CI-safe.
  (
    cd "$ROOT"
    export PATH="/usr/bin:${HOME}/.local/bin:${PATH}"
    cargo run --quiet --features amd-xdna2 --example materialize_research_txn -- "$BLOB"
  )
  write_prov "synthetic_classic_v01_noop" \
    "Built by materialize_txn_offline.sh (NOOP-only). Valid header/op walk; not GEMM."
  echo "OK synthetic: $BLOB ($(wc -c < "$BLOB") bytes)"
  echo "  PROVENANCE: $PROV"
  exit 0
fi

# --- aiebu path (lab machine with Ryzen AI full/min install) ---
AIEBU_CANDIDATES=(
  "${HOME}/Projects/ai/ryzen-ai-install/venv/bin/aiebu-asm"
  "${HOME}/Projects/ai/ryzen-ai-install/venv-eepy-min/bin/aiebu-asm"
  "$(command -v aiebu-asm || true)"
)
BIN_CANDIDATES=(
  "${HOME}/Projects/ai/ryzen-ai-install/venv/lib/python3.12/site-packages/bin/aiebu-asm-lib/aie2/copy_DDR_to_Mem_Tile.bin"
)

AIEBU=""
for c in "${AIEBU_CANDIDATES[@]}"; do
  [[ -n "$c" && -x "$c" ]] && AIEBU="$c" && break
done
SRC=""
for c in "${BIN_CANDIDATES[@]}"; do
  [[ -f "$c" ]] && SRC="$c" && break
done

if [[ -z "$AIEBU" || -z "$SRC" ]]; then
  echo "ERROR: --from-aiebu needs aiebu-asm + copy_DDR_to_Mem_Tile.bin on this host" >&2
  echo "  falling back to synthetic" >&2
  exec bash "$0" --force
fi

cp -a "$SRC" "$BLOB"
"$AIEBU" -t aie2txn -c "$SRC" -o "$ELF" -r > "${OUT_DIR}/aiebu_report.txt" 2>&1 || {
  echo "WARN: aiebu-asm failed; kept raw txn only" >&2
}
write_prov "aiebu_copy_DDR_to_Mem_Tile" \
  "Raw v0.1 txn from aiebu-asm-lib (DMA copy helper, not GEMM). Lab-only; AMD license terms apply; gitignored binary."
sha256sum "$BLOB" | tee "${OUT_DIR}/lab_instr.sha256"
echo "OK aiebu materialize: $BLOB"
echo "  (research DMA copy txn — still NOT_AIE2_KNOWN_GOOD_GEMM)"
