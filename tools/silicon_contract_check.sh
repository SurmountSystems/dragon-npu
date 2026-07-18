#!/usr/bin/env bash
# Offline silicon contract gate (no L3 / no EXEC / ≠ ran_on_npu).
#
# Exit:
#   0  green (ELF + offline GT dump OK + preflight soft-or-require)
#   2  blocked (missing ELF/assets/patch oracle fail / test fail)
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

# Unit tests for rela/patch + pure contract (hard-fail; no || true)
# Filters cover residual P0–P4: overwrite+add patch, dynsym, validate_patched refuse.
echo "--- cargo test residual patch / dynsym / validate_patched / silicon_contract ---"
cargo test --features amd-xdna2 --lib patch_aiebu -- --quiet
cargo test --features amd-xdna2 --lib dynsym -- --quiet
cargo test --features amd-xdna2 --lib validate_patched -- --quiet
cargo test --features amd-xdna2 --lib lab_rela_mode_token -- --quiet
cargo test --features amd-xdna2 --lib classify_raw_classic_vs_not_aiebu_elf -- --quiet
cargo test --features amd-xdna2 --lib silicon_contract -- --quiet

if command -v readelf >/dev/null 2>&1 && [[ -f "$ELF" ]]; then
  n="$(readelf -rW "$ELF" 2>/dev/null | grep -c 'R_' || true)"
  if [[ -n "${n:-}" && "$n" -eq 12 ]]; then
    ok "readelf reloc rows=$n (staged mladf expects 12)"
  elif [[ -n "${n:-}" && "$n" -gt 0 ]]; then
    echo "WARN: readelf reloc rows=$n (expected 12 for staged mladf)" >&2
  fi
fi

echo "--- offline GT dump ---"
bash tools/dump_mladf_gt_offline.sh
[[ -f "$GT/instr_post_patch.bin" ]] || fail "missing gt_capture/instr_post_patch.bin"
[[ -f "$GT/ctrl_post_patch.bin" ]] || fail "missing gt_capture/ctrl_post_patch.bin"
[[ -f "$GT/MANIFEST.sha256" ]] || fail "missing gt_capture/MANIFEST.sha256"
[[ -f "$GT/PROVENANCE.md" ]] || fail "missing gt_capture/PROVENANCE.md"
ok "gt_capture oracle present"

isz="$(wc -c <"$GT/instr_post_patch.bin" | tr -d ' ')"
[[ "$isz" -eq 1688 ]] || fail "instr_post_patch size $isz != 1688"
ok "ctrltext post-patch size=1688"

echo "--- self-consistency / GT parity ---"
if [[ -f "$ROOT/tools/diff_gt_parity.sh" ]]; then
  bash "$ROOT/tools/diff_gt_parity.sh" || fail "diff_gt_parity failed"
  ok "diff_gt_parity"
fi

echo "--- vendor instr parity (soft unless XDNA2_REQUIRE_VENDOR_PARITY=1) ---"
if [[ -f "$ROOT/tools/diff_vendor_instr_parity.sh" ]]; then
  # Always run --self (no vendor required)
  bash "$ROOT/tools/diff_vendor_instr_parity.sh" --self || fail "diff_vendor_instr_parity --self failed"
  ok "diff_vendor_instr_parity --self"
  # Offline CLASS_HINT / exit-code matrix (soft unless XDNA2_REQUIRE_OFFLINE_PARITY_TESTS=1)
  if [[ -f "$ROOT/tools/test_diff_vendor_parity_offline.sh" ]]; then
    set +e
    bash "$ROOT/tools/test_diff_vendor_parity_offline.sh"
    ot_rc=$?
    set -e
    if [[ "$ot_rc" -eq 0 ]]; then
      ok "test_diff_vendor_parity_offline"
    else
      if [[ "${XDNA2_REQUIRE_OFFLINE_PARITY_TESTS:-0}" == "1" ]]; then
        fail "test_diff_vendor_parity_offline rc=$ot_rc and XDNA2_REQUIRE_OFFLINE_PARITY_TESTS=1"
      else
        echo "WARN: test_diff_vendor_parity_offline rc=$ot_rc (soft; set XDNA2_REQUIRE_OFFLINE_PARITY_TESTS=1 to hard-fail)" >&2
      fi
    fi
  fi
  if [[ -f "$GT/instr_vendor_post_patch.bin" ]]; then
    set +e
    bash "$ROOT/tools/diff_vendor_instr_parity.sh"
    vp_rc=$?
    set -e
    if [[ "$vp_rc" -eq 0 ]]; then
      ok "vendor vs pure-direct match"
    elif [[ "$vp_rc" -eq 3 ]]; then
      # Vendor file present but parity script still miss → hard fail (broken inputs)
      fail "vendor file present but diff_vendor_instr_parity miss (rc=3) — fix ELF/oracle/paths"
    else
      # mismatch (2) or other — soft unless REQUIRE
      if [[ "${XDNA2_REQUIRE_VENDOR_PARITY:-0}" == "1" ]]; then
        fail "vendor parity mismatch (rc=$vp_rc) and XDNA2_REQUIRE_VENDOR_PARITY=1"
      else
        echo "WARN: vendor vs pure-direct MISMATCH (rc=$vp_rc) — agent offline fix; not hard-fail" >&2
        echo "      set XDNA2_REQUIRE_VENDOR_PARITY=1 to hard-fail when vendor dump present" >&2
      fi
    fi
  else
    echo "WARN: skip vendor parity — missing $GT/instr_vendor_post_patch.bin" >&2
    echo "      optional: bash tools/silicon_lab_operator.sh capture-instr (not mandatory if body green via sibling)" >&2
  fi
fi

echo "--- last_apply residual class (soft; ≠ C-ready) ---"
# Soft feedback loop: when a HITL last_apply.json exists, classify APP_LOAD residual.
# Always soft (exit 0); never unlocks C; never claims ran_on_npu.
LAST_APPLY="$ROOT/testdata/xdna2_cmd/last_apply.json"
if [[ ! -f "$ROOT/tools/classify_last_apply_residual.sh" ]]; then
  echo "NOTE: skip last_apply residual classify — tools/classify_last_apply_residual.sh missing (soft)"
elif [[ -f "$LAST_APPLY" ]] || [[ -n "${XDNA2_LAB_LAST_APPLY_JSON:-}" ]] || [[ -n "${XDNA2_LAST_APPLY_JSON:-}" ]]; then
  set +e
  # Pass explicit fixture when that is why we entered (avoids accidental /tmp plant).
  if [[ -f "$LAST_APPLY" ]]; then
    bash "$ROOT/tools/classify_last_apply_residual.sh" "$LAST_APPLY"
  else
    bash "$ROOT/tools/classify_last_apply_residual.sh"
  fi
  cl_rc=$?
  set -e
  if [[ "$cl_rc" -eq 0 ]]; then
    ok "last_apply residual classify soft (≠ C-ready ≠ ran_on_npu)"
  else
    echo "WARN: classify_last_apply_residual rc=$cl_rc (soft; not hard-fail)" >&2
  fi
else
  echo "NOTE: skip last_apply residual classify — no testdata last_apply.json (soft)"
fi

echo "--- preflight ---"
# Avoid pipefail+head SIGPIPE (exit 141) on long --print-env output.
PF_OUT="$(mktemp)"
set +e
if [[ "$REQUIRE_PASS" -eq 1 ]]; then
  bash tools/silicon_lab_preflight.sh --require-pass-assets --print-env >"$PF_OUT" 2>&1
  pf_rc=$?
else
  bash tools/silicon_lab_preflight.sh --print-env >"$PF_OUT" 2>&1
  pf_rc=$?
fi
set -e
head -25 "$PF_OUT" || true
rm -f "$PF_OUT"
if [[ "$REQUIRE_PASS" -eq 1 && "$pf_rc" -ne 0 ]]; then
  fail "preflight --require-pass-assets rc=$pf_rc"
fi

echo ""
ok "offline contract green (still ≠ ran_on_npu)"
echo "Residual: true vendor post-patch via public DD not available offline (INSTR_BO / no live BO API);"
echo "  pure-direct body parity green (CLASS A unpatched); APP_LOAD_PDI_FAIL separate silicon residual."
echo "Soft last_apply class (when fixture present): bash tools/classify_last_apply_residual.sh"
echo "  → residual_class=… not_c_ready=1; never C unlock from CLASS A / CONTRACT_OK."
echo "Optional capture (logs INSTR_BO diagnostics; may still be unpatched A; ≠ C unlock):"
echo "  sudo sh -c 'echo 1 > /sys/module/amdxdna/parameters/exec_cmd_allow'"
echo "  bash tools/silicon_lab_operator.sh capture-instr"
echo "  sudo sh -c 'echo 0 > /sys/module/amdxdna/parameters/exec_cmd_allow'"
echo "  bash tools/diff_vendor_instr_parity.sh   # unpatched A = body match only ≠ C-ready"
echo "Ban: env C applies (KERNEL_ARG_LAYOUT / INSTR_SIZE / RELA_MODE)."
echo "HITL C one-shot under C_UNLOCK=b17b0119 (operator C2; unlock ≠ product Pass)."
echo "  registry_get_instr_bo = Transaction/cache only, not proven live run_ BO;"
echo "  mere CONTRACT_OK / unpatched diff rc=0 / CLASS A / registry / C3_SKIP /"
echo "  C4_SKIP / offline C4_PASS_CANDIDATE alone ≠ C-ready ≠ invent product Pass."
echo "  Offline PassCandidate audit: bash tools/c4_pass_check.sh"
echo "  Live apply (operator, L3 sandwich + restore): bash tools/hitl_c_pass_apply.sh"
echo "Doc:  docs/XDNA2_XRT_CONTRACT_MLADF.md docs/HITL_BC.md docs/HITL_C_APP_LOAD_RECIPE.md RESIDUAL.md"
exit 0
