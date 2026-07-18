#!/usr/bin/env bash
# Offline gate suite (self-runnable table from RESIDUAL.md).
#
# Hard steps (exit non-zero on fail):
#   fmt, clippy (lib+classify example), pure_direct, silicon_contract,
#   classic_1688 (hard; refuse 0 matched tests),
#   bash -n: hitl_c_pass_apply, hitl_c2_once, c3_post_c2_classify,
#            c4_pass_check, run_offline_gates, classify_last_apply_residual,
#            silicon_contract_check, test_diff_vendor_parity_offline,
#            diff_vendor_instr_parity
#
# Soft steps (default: report only; never hard-fail the suite):
#   classify_last_apply_residual (always soft unless --require-assets + fixture skip)
#   c4_pass_check (soft; C4_SKIP on baseline fixture — never invents Pass)
#   Asset-dependent scripts when lab_instr.elf present:
#     diff_vendor_instr_parity, test_diff_vendor_parity_offline,
#     silicon_contract_check — soft skip when assets missing (CI clean checkout)
#
# --require-assets:
#   Hard-fail if lab ELF missing, soft asset steps skip, or soft asset/classify
#   steps return non-zero. Soft_ok metrics still distinguish soft_warn.
#
# Honesty (permanent):
#   Green here ≠ C-ready ≠ ran_on_npu.
#   residual_class / CLASSIFY_SOFT_OK / app_load_checklist / CONTRACT_OK /
#   CLASS A / unpatched diff rc=0 / OFFLINE_GATES_OK never unlock HITL C.
#   Never sets XDNA2_ALLOW_EXEC_CMD. No L3 / no EXEC / no apply.
#
# Usage:
#   bash tools/run_offline_gates.sh
#   bash tools/run_offline_gates.sh --require-assets   # hard on missing/failing asset steps
#   bash tools/run_offline_gates.sh --help
set -euo pipefail
# HOME may be unset under set -u (CI/containers); never require it for PATH.
export PATH="/usr/bin${HOME:+:${HOME}/.local/bin}:${PATH:-}"
# Refuse accidental EXEC gate in this suite.
unset XDNA2_ALLOW_EXEC_CMD || true

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

REQUIRE_ASSETS=0
for a in "$@"; do
  case "$a" in
    --require-assets) REQUIRE_ASSETS=1 ;;
    -h|--help)
      sed -n '1,35p' "$0" | sed 's/^# \?//'
      exit 0
      ;;
    *)
      echo "ERROR: unknown arg: $a (allowlist: --require-assets --help)" >&2
      exit 2
      ;;
  esac
done

HARD_FAIL=0
SOFT_SKIP=0
SOFT_OK=0
SOFT_WARN=0
HARD_OK=0

ok_hard() { echo "HARD_OK: $*"; HARD_OK=$((HARD_OK + 1)); }
fail_hard() {
  echo "HARD_FAIL: $*" >&2
  HARD_FAIL=$((HARD_FAIL + 1))
}
ok_soft() { echo "SOFT_OK: $*"; SOFT_OK=$((SOFT_OK + 1)); }
warn_soft() {
  echo "SOFT_WARN: $*" >&2
  SOFT_WARN=$((SOFT_WARN + 1))
}
skip_soft() { echo "SOFT_SKIP: $*"; SOFT_SKIP=$((SOFT_SKIP + 1)); }

run_hard() {
  local label="$1"
  shift
  echo "--- hard: $label ---"
  set +e
  "$@"
  local rc=$?
  set -e
  if [[ "$rc" -eq 0 ]]; then
    ok_hard "$label"
  else
    fail_hard "$label (rc=$rc)"
  fi
}

# Soft: only SOFT_OK on rc=0; non-zero → SOFT_WARN (never count as ok).
# Under --require-assets, non-zero soft steps also trip HARD_FAIL.
run_soft() {
  local label="$1"
  shift
  echo "--- soft: $label ---"
  set +e
  "$@"
  local rc=$?
  set -e
  if [[ "$rc" -eq 0 ]]; then
    ok_soft "$label"
  else
    warn_soft "$label rc=$rc (non-fatal unless --require-assets; still ≠ C-ready)"
    if [[ "$REQUIRE_ASSETS" -eq 1 ]]; then
      fail_hard "require-assets soft step failed: $label (rc=$rc)"
    fi
  fi
}

# Hard cargo test filter: refuse vacuous green (0 tests matched).
# Capture cargo output with set +e (never abort on compile fail).
# Parse without bare grep under pipefail (grep no-match would abort pipeline).
run_hard_cargo_filter() {
  local label="$1"
  local filter="$2"
  echo "--- hard: $label (filter=$filter) ---"
  local out=""
  local rc=0
  local n=""
  local snippet=""
  set +e
  out="$(cargo test --features amd-xdna2 --lib "$filter" 2>&1)"
  rc=$?
  set -e
  # Always show cargo log so operators see compile errors.
  printf '%s\n' "$out"
  # Safe parse: grep exit 1 must not abort (pipefail + set -e).
  n="$(printf '%s\n' "$out" | { grep -E '^running [0-9]+ tests?$' || true; } | tail -1 | awk '{print $2}')"
  if [[ -z "${n}" ]]; then
    n="$(printf '%s\n' "$out" | { grep -E '^test result:' || true; } | tail -1 | sed -n 's/.* \([0-9][0-9]*\) passed.*/\1/p')"
  fi
  # Last ~12 lines for fail_hard message (compile-fail / missing summary).
  snippet="$(printf '%s\n' "$out" | tail -12 | tr '\n' ' ' | sed 's/  */ /g' | cut -c1-240)"
  if [[ -z "${n}" ]]; then
    fail_hard "$label (no 'running N tests' / test-result parse; cargo_rc=$rc; refuse vacuous green; tail: ${snippet:-<empty>})"
    return 0
  fi
  if [[ "$n" -eq 0 ]]; then
    fail_hard "$label (0 tests matched filter '$filter'; cargo_rc=$rc; refuse vacuous green)"
    return 0
  fi
  if [[ "$rc" -ne 0 ]]; then
    fail_hard "$label (rc=$rc; matched=$n; tail: ${snippet:-<empty>})"
    return 0
  fi
  ok_hard "$label (matched=$n)"
}

ELF="$ROOT/testdata/xdna2_cmd/silicon_lab/lab_instr.elf"
LAST_APPLY="$ROOT/testdata/xdna2_cmd/last_apply.json"

echo "=== run_offline_gates (offline only; ≠ C-ready ≠ ran_on_npu) ==="
echo "ROOT=$ROOT REQUIRE_ASSETS=$REQUIRE_ASSETS"
echo "BAN: residual_class / CLASSIFY_SOFT_OK / checklist / CONTRACT_OK / CLASS A / OFFLINE_GATES_OK ≠ unlock C"

# --- hard unit / style ---
run_hard "cargo fmt --check" cargo fmt -- --check
run_hard "clippy amd-xdna2 lib+classify+c4" \
  cargo clippy --features amd-xdna2 --lib --example classify_last_apply --example c4_pass_check -- -D warnings
run_hard_cargo_filter "pure_direct" "pure_direct"
run_hard_cargo_filter "silicon_contract" "silicon_contract"
run_hard_cargo_filter "classic_1688" "classic_1688"

run_hard "bash -n hitl_c_pass_apply" bash -n tools/hitl_c_pass_apply.sh
run_hard "bash -n hitl_c2_once" bash -n tools/hitl_c2_once.sh
run_hard "bash -n c3_post_c2_classify" bash -n tools/c3_post_c2_classify.sh
run_hard "bash -n c4_pass_check" bash -n tools/c4_pass_check.sh
run_hard "bash -n run_offline_gates" bash -n tools/run_offline_gates.sh
run_hard "bash -n classify_last_apply_residual" bash -n tools/classify_last_apply_residual.sh
run_hard "bash -n silicon_contract_check" bash -n tools/silicon_contract_check.sh
run_hard "bash -n test_diff_vendor_parity_offline" bash -n tools/test_diff_vendor_parity_offline.sh
run_hard "bash -n diff_vendor_instr_parity" bash -n tools/diff_vendor_instr_parity.sh

# --- soft classify + C4 pass audit (fixture may be gitignored) ---
if [[ -f "$LAST_APPLY" ]] || [[ -n "${XDNA2_LAB_LAST_APPLY_JSON:-}" ]]; then
  run_soft "classify_last_apply_residual" bash tools/classify_last_apply_residual.sh
  run_soft "c4_pass_check" bash tools/c4_pass_check.sh
else
  skip_soft "classify_last_apply_residual (no last_apply fixture; gitignored OK)"
  skip_soft "c4_pass_check (no last_apply fixture; gitignored OK)"
  if [[ "$REQUIRE_ASSETS" -eq 1 ]]; then
    fail_hard "require-assets: classify skipped (no last_apply fixture / XDNA2_LAB_LAST_APPLY_JSON)"
  fi
fi

# --- asset-dependent soft / require ---
if [[ -f "$ELF" ]]; then
  run_soft "diff_vendor_instr_parity" bash tools/diff_vendor_instr_parity.sh
  run_soft "test_diff_vendor_parity_offline" bash tools/test_diff_vendor_parity_offline.sh
  run_soft "silicon_contract_check" bash tools/silicon_contract_check.sh
else
  msg="no $ELF (gitignored lab assets; CI clean checkout OK)"
  skip_soft "asset scripts: $msg"
  if [[ "$REQUIRE_ASSETS" -eq 1 ]]; then
    fail_hard "require-assets: $msg"
  fi
fi

echo
echo "=== run_offline_gates summary ==="
echo "hard_ok=$HARD_OK hard_fail=$HARD_FAIL soft_ok=$SOFT_OK soft_warn=$SOFT_WARN soft_skip=$SOFT_SKIP"
echo "not_c_ready=1 not_ran_on_npu=1"
echo "BAN: do not treat green offline gates as HITL C unlock / Pass"
# Soft stamp NOTE only (never flips not_c_ready; never opens L3 / ALLOW).
if [[ -f "$ROOT/RESIDUAL.md" ]]; then
  _og_c_unlock="$(
    { grep -m1 -oE 'C_UNLOCK=[A-Za-z0-9_]+' "$ROOT/RESIDUAL.md" || true; } \
      | head -n1 | tr -d '`"*' | tr -d '[:space:]'
  )"
  if [[ -n "${_og_c_unlock}" ]]; then
    echo "NOTE: ${_og_c_unlock} one-shot present in RESIDUAL (soft; still not_c_ready=1 not_ran_on_npu=1)"
    echo "NOTE: unlock ≠ Pass; operator C2 only; restore L3=exec_cmd_allow=0 after apply"
  fi
fi
if [[ "$HARD_FAIL" -ne 0 ]]; then
  echo "OFFLINE_GATES_FAIL: $HARD_FAIL hard step(s) failed" >&2
  exit 1
fi
echo "OFFLINE_GATES_OK (still ≠ C-ready ≠ ran_on_npu)"
exit 0
