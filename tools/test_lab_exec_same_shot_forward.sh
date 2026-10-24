#!/usr/bin/env bash
# Offline smoke: lab_exec_once must forward XDNA2_LAB_SAME_SHOT_RELA into RUN_ENV
# (cargo child). Grep-only — never executes lab apply / ALLOW / L3.
# Exit 0 when the forward line is present; exit 2 when missing.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LAB="${ROOT}/scripts/lab_exec_once.sh"
if [[ ! -f "$LAB" ]]; then
  echo "ERROR: missing $LAB" >&2
  exit 2
fi
# Exact forward shape (must not rely on inherit alone).
if ! grep -qE 'RUN_ENV\+=\("XDNA2_LAB_SAME_SHOT_RELA=\$XDNA2_LAB_SAME_SHOT_RELA"\)' "$LAB"; then
  echo "FAIL: lab_exec_once.sh missing RUN_ENV forward for XDNA2_LAB_SAME_SHOT_RELA" >&2
  exit 2
fi
if ! grep -q 'XDNA2_LAB_SAME_SHOT_RELA' "$LAB"; then
  echo "FAIL: lab_exec_once.sh has no XDNA2_LAB_SAME_SHOT_RELA reference" >&2
  exit 2
fi
echo "OK: lab_exec_once RUN_ENV forwards XDNA2_LAB_SAME_SHOT_RELA (grep-only; no execute)"
exit 0
