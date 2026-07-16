#!/usr/bin/env bash
# Offline GT dump (no L3 / no EXEC / ≠ ran_on_npu).
#
#   bash tools/dump_mladf_gt_offline.sh
#   XDNA2_GT_OUT=/tmp/gt bash tools/dump_mladf_gt_offline.sh
#
# Prefers staged lab_instr.elf; stages via aiebu if missing.
set -euo pipefail
export PATH="/usr/bin:${HOME}/.local/bin:${PATH}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

ELF="$ROOT/testdata/xdna2_cmd/silicon_lab/lab_instr.elf"
BIN="$ROOT/testdata/xdna2_cmd/silicon_lab/lab_instr.bin"
if [[ ! -f "$ELF" && -f "$BIN" ]]; then
  echo "NOTE: staging aiebu ELF from lab_instr.bin …"
  bash "$ROOT/tools/stage_aiebu_lab_instr.sh"
fi
if [[ ! -f "$ELF" ]]; then
  echo "ERROR: no lab_instr.elf — stage silicon assets + aiebu first" >&2
  echo "  bash tools/silicon_lab_operator.sh stage" >&2
  echo "  bash tools/stage_aiebu_lab_instr.sh" >&2
  exit 2
fi

exec cargo run --features amd-xdna2 --example dump_mladf_gt_offline --quiet
