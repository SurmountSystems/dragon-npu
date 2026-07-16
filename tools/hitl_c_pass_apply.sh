#!/usr/bin/env bash
# Milestone C — Pure-direct Pass apply (YOU run this when agent says ready).
#
# Forces aiebu ELF (not raw 1688 B txn), silicon-mladf assets, L1+L3 window,
# single-shot npu_direct_bench, always restores exec_cmd_allow=0.
#
# Fish-safe: run with bash.
#
# Usage:
#   # Recommended: open L3 yourself, then:
#   sudo sh -c 'echo 1 > /sys/module/amdxdna/parameters/exec_cmd_allow'
#   bash tools/hitl_c_pass_apply.sh --assume-l3-already-open
#   sudo sh -c 'echo 0 > /sys/module/amdxdna/parameters/exec_cmd_allow'
#
#   # Or let lab_exec try to open L3 (may print human sudo steps):
#   bash tools/hitl_c_pass_apply.sh
#
# Exit: same as lab_exec_once (0 rare Pass; 4 = !ran_on_npu common; 5 restore fail).
set -euo pipefail
export PATH="/usr/bin:${HOME}/.local/bin:${PATH}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

ASSUME=()
BO_DIAG=1
for a in "$@"; do
  case "$a" in
    --assume-l3-already-open|--assume-l3-open)
      ASSUME+=(--assume-l3-already-open)
      ;;
    --no-bo-diag) BO_DIAG=0 ;;
    -h|--help)
      sed -n '1,35p' "$0" | sed 's/^# \?//'
      exit 0
      ;;
    *)
      # forward unknown to lab_exec
      ASSUME+=("$a")
      ;;
  esac
done

DIR="$ROOT/testdata/xdna2_cmd/silicon_lab"
ELF="$DIR/lab_instr.elf"
BIN="$DIR/lab_instr.bin"

echo "=== Milestone C: pure-direct Pass apply ==="

if [[ ! -f "$ELF" ]]; then
  if [[ -f "$BIN" ]]; then
    echo "NOTE: staging aiebu ELF (raw .bin is not valid EXEC payload) …"
    bash tools/stage_aiebu_lab_instr.sh
  else
    echo "ERROR: missing $ELF — stage assets first" >&2
    echo "  bash tools/silicon_lab_operator.sh stage" >&2
    exit 2
  fi
fi
[[ -f "$ELF" ]] || { echo "ERROR: still no lab_instr.elf" >&2; exit 2; }

echo "BLOB: $ELF ($(wc -c <"$ELF" | tr -d ' ') B) — must be ELF, not raw txn"
head32="$(od -An -tx1 -N4 "$ELF" | tr -d ' \n')"
if [[ "$head32" != "7f454c46" ]]; then
  echo "ERROR: $ELF is not ELF magic (got $head32) — refuse raw txn EXEC" >&2
  exit 2
fi

# Offline gate (soft if GT missing)
if [[ -x "$ROOT/tools/silicon_contract_check.sh" ]] || [[ -f "$ROOT/tools/silicon_contract_check.sh" ]]; then
  echo "--- offline contract check ---"
  set +e
  bash "$ROOT/tools/silicon_contract_check.sh"
  cc=$?
  set -e
  if [[ "$cc" -ne 0 ]]; then
    echo "WARN: silicon_contract_check rc=$cc — continuing apply (HITL may still teach)" >&2
  fi
fi

echo "--- preflight Pass assets ---"
bash tools/silicon_lab_preflight.sh --require-pass-assets

export XDNA2_LAB_BO_DIAG="${XDNA2_LAB_BO_DIAG:-$BO_DIAG}"
# Clear layout override so default count4 is used unless you set it.
# (fish: set -e XDNA2_LAB_KERNEL_ARG_LAYOUT before invoking)

echo "--- apply (wedge risk accepted by flag) ---"
set +e
bash scripts/lab_exec_once.sh --silicon-mladf --require-pass-assets \
  --apply --i-accept-wedge-risk \
  --blob "$ELF" \
  "${ASSUME[@]}"
rc=$?
set -e

allow="?"
[[ -r /sys/module/amdxdna/parameters/exec_cmd_allow ]] && \
  allow="$(tr -d '[:space:]' </sys/module/amdxdna/parameters/exec_cmd_allow || true)"
echo ""
echo "POST: lab_exec rc=$rc  exec_cmd_allow=$allow"
case "$allow" in
  0|N|n) echo "POST_OK: L3 closed" ;;
  *)
    echo "CRITICAL: restore L3:" >&2
    echo "  sudo sh -c 'echo 0 > /sys/module/amdxdna/parameters/exec_cmd_allow'" >&2
    ;;
esac

echo ""
echo "What to paste for the agent (if !ran_on_npu):"
echo "  - status=… line / TimedOpResult JSON status field"
echo "  - detail snippets: aiebu_elf=  aiebu_rela_patched=  ert_state=  ofm_stats="
echo "  - journal: journalctl -k -b --no-pager | rg -i 'Failed cmd|Status 0x|APP_LOAD|amdxdna' | tail -20"
echo ""
echo "Success signal: ran_on_npu=true (bench exit 0). Common: exit 4 until Pass."
exit "$rc"
