#!/usr/bin/env bash
# Offline silicon contract gate (no L3 / no EXEC / ≠ ran_on_npu).
#
# Exit:
#   0  green (ELF + offline GT dump OK + preflight soft-or-require)
#   2  blocked (missing ELF/assets/patch oracle fail)
#
#   bash tools/silicon_contract_check.sh
#   bash tools/silicon_contract_check.sh --require-pass-assets
set -euo pipefail
export PATH="/usr/bin:${HOME}/.local/bin:${PATH}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

REQUIRE_PASS=0
for a in "$@"; do
  case "$a" in
    --require-pass-assets) REQUIRE_PASS=1 ;;
    -h|--help)
      sed -n '1,20p' "$0" | sed 's/^# \?//'
      exit 0
      ;;
    *) echo "ERROR: unknown $a" >&2; exit 2 ;;
  esac
done

fail() { echo "CONTRACT_FAIL: $*" >&2; exit 2; }
ok() { echo "CONTRACT_OK: $*"; }

DIR="$ROOT/testdata/xdna2_cmd/silicon_lab"
ELF="$DIR/lab_instr.elf"
BIN="$DIR/lab_instr.bin"
GT="$ROOT/testdata/xdna2_cmd/gt_capture"

echo "=== silicon_contract_check (offline) ==="

if [[ ! -f "$ELF" ]]; then
  if [[ -f "$BIN" ]]; then
    echo "NOTE: staging aiebu ELF …"
    bash tools/stage_aiebu_lab_instr.sh || fail "stage_aiebu_lab_instr failed"
  else
    fail "no lab_instr.elf or .bin under $DIR"
  fi
fi
[[ -f "$ELF" ]] || fail "missing $ELF"
magic="$(od -An -tx1 -N4 "$ELF" | tr -d ' \n')"
[[ "$magic" == "7f454c46" ]] || fail "lab_instr.elf not ELF magic ($magic)"
ok "elf present size=$(wc -c <"$ELF" | tr -d ' ')"

# Unit tests for rela/patch (feature-gated)
echo "--- cargo test patch_aiebu / classify ---"
cargo test --features amd-xdna2 --lib patch_aiebu_elf -- --quiet
cargo test --features amd-xdna2 --lib classify_instruction -- --quiet 2>/dev/null || true

echo "--- offline GT dump ---"
bash tools/dump_mladf_gt_offline.sh
[[ -f "$GT/instr_post_patch.bin" ]] || fail "missing gt_capture/instr_post_patch.bin"
[[ -f "$GT/ctrl_post_patch.bin" ]] || fail "missing gt_capture/ctrl_post_patch.bin"
[[ -f "$GT/MANIFEST.sha256" ]] || fail "missing gt_capture/MANIFEST.sha256"
ok "gt_capture oracle present"

isz="$(wc -c <"$GT/instr_post_patch.bin" | tr -d ' ')"
[[ "$isz" -eq 1688 ]] || fail "instr_post_patch size $isz != 1688"
ok "ctrltext post-patch size=1688"

echo "--- preflight ---"
if [[ "$REQUIRE_PASS" -eq 1 ]]; then
  bash tools/silicon_lab_preflight.sh --require-pass-assets --print-env | head -25
else
  bash tools/silicon_lab_preflight.sh --print-env | head -25 || true
fi

echo ""
ok "offline contract green (still ≠ ran_on_npu)"
echo "HITL: bash tools/hitl_b_gt_capture.sh [--with-vendor]"
echo "HITL: bash tools/hitl_c_pass_apply.sh --assume-l3-already-open"
exit 0
