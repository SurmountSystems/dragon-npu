#!/usr/bin/env bash
# Offline tests for tools/diff_vendor_instr_parity.sh (CLASS_HINT / exit codes).
#
# No L3 / no EXEC / ≠ ran_on_npu. No live vendor capture required.
#
# Cases:
#   A  --self → exit 0
#   B  synthetic one-byte mismatch vs pure oracle → exit 2 + CLASS_HINT
#   C  missing vendor → exit 3
#   D  oversized multi-txn post_patch + sibling classic 1688 unpatched
#      → recover sibling; CLASS_HINT A or D with body match (not false C)
#
# Optional soft:
#   last_apply.json with io_paddrs → extract_paddrs smoke to temp OUT
#
# Exit: 0 only if A/B/C/D pass (soft extract never hard-fails).
set -euo pipefail
export PATH="/usr/bin:${HOME}/.local/bin:${PATH}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# Ambient paddrs JSON must not skew pure oracle during offline cases.
env -u XDNA2_GT_PADDRS_JSON true 2>/dev/null || true
unset XDNA2_GT_PADDRS_JSON || true

DIFF="$ROOT/tools/diff_vendor_instr_parity.sh"
ELF="$ROOT/testdata/xdna2_cmd/silicon_lab/lab_instr.elf"
LAST="$ROOT/testdata/xdna2_cmd/last_apply.json"
PASS=0
FAIL=0
EXPECTED_CASES=4

fail_case() {
  echo "OFFLINE_PARITY_FAIL: $*" >&2
  FAIL=$((FAIL + 1))
}
ok_case() {
  echo "OFFLINE_PARITY_OK: $*"
  PASS=$((PASS + 1))
}

[[ -f "$DIFF" ]] || {
  echo "ERROR: missing $DIFF" >&2
  exit 2
}
[[ -f "$ELF" ]] || {
  echo "ERROR: missing $ELF — stage aiebu ELF first" >&2
  echo "  bash tools/stage_aiebu_lab_instr.sh" >&2
  exit 2
}
[[ -x "$DIFF" || -f "$DIFF" ]] || true

# Ensure dump_mladf_gt_offline can run (cargo + ELF).
if ! bash tools/dump_mladf_gt_offline.sh >/dev/null 2>&1; then
  echo "ERROR: dump_mladf_gt_offline failed (need cargo --features amd-xdna2 + staged ELF)" >&2
  exit 2
fi

echo "=== test_diff_vendor_parity_offline ==="

# --- Case A: --self → 0 ---
set +e
A_OUT="$(env -u XDNA2_GT_PADDRS_JSON bash "$DIFF" --self 2>&1)"
A_RC=$?
set -e
if [[ "$A_RC" -eq 0 ]]; then
  ok_case "A --self rc=0"
else
  fail_case "A --self expected rc=0 got $A_RC"
  echo "$A_OUT" | tail -20 >&2 || true
fi

# --- Case B: synthetic mismatch → 2 + CLASS_HINT ---
TMPB="$(mktemp -d "${TMPDIR:-/tmp}/parity_b.XXXXXX")"
cleanup_b() { rm -rf "$TMPB"; }
trap cleanup_b EXIT

set +e
env -u XDNA2_GT_PADDRS_JSON XDNA2_GT_OUT="$TMPB" bash tools/dump_mladf_gt_offline.sh >/dev/null 2>&1
B_DUMP_RC=$?
set -e
if [[ "$B_DUMP_RC" -ne 0 || ! -f "$TMPB/instr_post_patch.bin" ]]; then
  fail_case "B dump pure oracle into $TMPB failed rc=$B_DUMP_RC"
else
  # Vendor = pure with one byte flipped (guarantees mismatch when both 1688).
  cp -f "$TMPB/instr_post_patch.bin" "$TMPB/instr_vendor_post_patch.bin"
  python3 - "$TMPB/instr_vendor_post_patch.bin" <<'PY'
import sys
from pathlib import Path
p = Path(sys.argv[1])
b = bytearray(p.read_bytes())
if not b:
    raise SystemExit("empty vendor")
# Flip last byte of ctrltext prefix (stable, non-empty).
i = min(len(b), 1688) - 1
b[i] = (b[i] ^ 0xFF) & 0xFF
p.write_bytes(bytes(b))
PY
  set +e
  B_OUT="$(env -u XDNA2_GT_PADDRS_JSON XDNA2_GT_OUT="$TMPB" bash "$DIFF" 2>&1)"
  B_RC=$?
  set -e
  if [[ "$B_RC" -ne 2 ]]; then
    fail_case "B synthetic mismatch expected rc=2 got $B_RC"
    echo "$B_OUT" | tail -30 >&2 || true
  elif ! grep -q 'CLASS_HINT' <<<"$B_OUT"; then
    fail_case "B expected CLASS_HINT in output"
    echo "$B_OUT" | tail -30 >&2 || true
  else
    hint="$(grep -oE 'CLASS_HINT [A-E]' <<<"$B_OUT" | head -1 || true)"
    ok_case "B synthetic mismatch rc=2 ${hint:-CLASS_HINT present}"
  fi
fi

# --- Case C: missing vendor → 3 ---
TMPC="$(mktemp -d "${TMPDIR:-/tmp}/parity_c.XXXXXX")"
cleanup_c() { rm -rf "$TMPB" "$TMPC"; }
trap cleanup_c EXIT

# Fresh GT dir with pure oracle only (no instr_vendor_post_patch.bin).
set +e
env -u XDNA2_GT_PADDRS_JSON XDNA2_GT_OUT="$TMPC" bash tools/dump_mladf_gt_offline.sh >/dev/null 2>&1
C_DUMP_RC=$?
set -e
rm -f "$TMPC/instr_vendor_post_patch.bin"
if [[ "$C_DUMP_RC" -ne 0 ]]; then
  fail_case "C dump pure oracle into $TMPC failed rc=$C_DUMP_RC"
else
  set +e
  C_OUT="$(env -u XDNA2_GT_PADDRS_JSON XDNA2_GT_OUT="$TMPC" bash "$DIFF" 2>&1)"
  C_RC=$?
  set -e
  if [[ "$C_RC" -eq 3 ]]; then
    ok_case "C missing vendor rc=3"
  else
    fail_case "C missing vendor expected rc=3 got $C_RC"
    echo "$C_OUT" | tail -20 >&2 || true
  fi
fi

# --- Case D: oversized multi-txn post_patch + classic sibling → A or D (not C) ---
TMPD="$(mktemp -d "${TMPDIR:-/tmp}/parity_d.XXXXXX")"
cleanup_d() { rm -rf "$TMPB" "$TMPC" "$TMPD"; }
trap cleanup_d EXIT

set +e
env -u XDNA2_GT_PADDRS_JSON XDNA2_GT_OUT="$TMPD" bash tools/dump_mladf_gt_offline.sh >/dev/null 2>&1
D_DUMP_RC=$?
set -e
if [[ "$D_DUMP_RC" -ne 0 || ! -f "$TMPD/instr_raw_txn.bin" ]]; then
  fail_case "D dump pure oracle into $TMPD failed rc=$D_DUMP_RC"
else
  # Sibling classic 1688 = pure unpatched raw (body match).
  cp -f "$TMPD/instr_raw_txn.bin" "$TMPD/instr_vendor_txn_bin.bin"
  # post_patch = multi-txn-ish wrong body (32k): classic header but different
  # size field + garbage so first-1688 ≠ classic (HITL false CLASS_HINT C).
  python3 - "$TMPD" <<'PY'
from pathlib import Path
import struct
gt = Path(__import__("sys").argv[1])
classic = (gt / "instr_raw_txn.bin").read_bytes()
assert len(classic) == 1688
# Multi-txn prefix: same magic, different num_ops / txn_size (HITL-like)
multi = bytearray(classic)
# XAie_TxnHeader-ish: keep maj/min/dev, force larger txn_size at typical offset
# Bytes 12:16 are txn_size on classic dump (0x00000698=1688); set to 32208.
struct.pack_into("<I", multi, 12, 32208)
# Flip a non-rela mid body byte so prefix clearly ≠ classic
multi[8] = (multi[8] ^ 0x55) & 0xFF
# Pad to 32208 multi-txn length
multi.extend(b"\x00" * (32208 - len(multi)))
(gt / "instr_vendor_post_patch.bin").write_bytes(bytes(multi))
# Unpatched capture_mode + known paddrs (must not re-patch pure for body class)
(gt / "vendor_paddrs.json").write_text(
    """{
  "schema_version": 1,
  "source": "test_diff_vendor_parity_offline_case_d",
  "capture_mode": "txn_bin_fallback_unpatched",
  "instr_bytes": 32208,
  "txn_bin_bytes": 1688,
  "classic_unpatched": true,
  "from_registry": false,
  "txn_bin_fallback_likely_unpatched": true,
  "ifm_paddr": "0x7ff1ff8c2000",
  "wts_paddr": "0x7ff1ff8c3000",
  "ofm_paddr": "0x7ff1ff8c1000",
  "ctrl_paddr": null,
  "paddrs_known": true,
  "note": "case D synthetic oversized multi-txn + classic sibling"
}
"""
)
PY
  set +e
  D_OUT="$(env -u XDNA2_GT_PADDRS_JSON XDNA2_GT_OUT="$TMPD" bash "$DIFF" 2>&1)"
  D_RC=$?
  set -e
  # Expect body match via sibling recover → rc=0 CLASS_HINT A (or D), not C.
  # Match only the verdict line "CLASS_HINT X:" (notes may say "not CLASS_HINT C").
  D_HINT_LINE="$(grep -E '^CLASS_HINT [A-E]:' <<<"$D_OUT" | tail -1 || true)"
  if [[ "$D_RC" -ne 0 ]]; then
    fail_case "D oversized+sibling expected rc=0 (body match) got $D_RC"
    echo "$D_OUT" | tail -40 >&2 || true
  elif [[ "$D_HINT_LINE" == CLASS_HINT\ C:* ]]; then
    fail_case "D must not report false CLASS_HINT C on multi-txn prefix"
    echo "$D_OUT" | tail -40 >&2 || true
  elif ! grep -qE '^CLASS_HINT [AD]:' <<<"$D_HINT_LINE"; then
    fail_case "D expected CLASS_HINT A or D got: ${D_HINT_LINE:-none}"
    echo "$D_OUT" | tail -40 >&2 || true
  elif ! grep -qE 'recovered classic|vendor_src=instr_vendor_txn_bin|PARITY_OK' <<<"$D_OUT"; then
    # Soft: still OK if A/D without recover phrase, but prefer recover path
    if grep -q 'PARITY_OK' <<<"$D_OUT"; then
      ok_case "D oversized+sibling rc=0 CLASS_HINT A/D (body match)"
    else
      fail_case "D expected recover/PARITY_OK path"
      echo "$D_OUT" | tail -40 >&2 || true
    fi
  else
    hint="$(grep -oE 'CLASS_HINT [A-E]' <<<"$D_HINT_LINE" | head -1 || true)"
    ok_case "D oversized+sibling rc=0 ${hint:-A/D} (not false C)"
  fi
fi

# --- Optional soft: extract-paddrs from last_apply if io_paddrs present ---
if [[ -f "$LAST" ]] && [[ -f "$ROOT/tools/extract_paddrs_from_last_apply.sh" ]]; then
  if grep -q 'io_paddrs' "$LAST" 2>/dev/null; then
    TMPE="$(mktemp -d "${TMPDIR:-/tmp}/parity_e.XXXXXX")"
    cleanup_e() { rm -rf "$TMPB" "$TMPC" "$TMPD" "$TMPE"; }
    trap cleanup_e EXIT
    set +e
    env -u XDNA2_GT_PADDRS_JSON bash "$ROOT/tools/extract_paddrs_from_last_apply.sh" \
      --out "$TMPE/vendor_paddrs.json" "$LAST" >/dev/null 2>&1
    E_RC=$?
    set -e
    if [[ "$E_RC" -eq 0 && -f "$TMPE/vendor_paddrs.json" ]]; then
      echo "OFFLINE_PARITY_SOFT: extract-paddrs smoke ok → $TMPE/vendor_paddrs.json"
    else
      echo "WARN: extract-paddrs soft smoke rc=$E_RC (non-fatal)" >&2
    fi
  else
    echo "NOTE: skip extract-paddrs — no io_paddrs in last_apply.json"
  fi
else
  echo "NOTE: skip extract-paddrs — last_apply or extract script missing"
fi

# Cleanup temps (trap also runs on exit)
rm -rf "$TMPB" "$TMPC" "$TMPD" "${TMPE:-}"
trap - EXIT

echo ""
echo "offline parity cases: pass=$PASS fail=$FAIL"
if [[ "$FAIL" -ne 0 ]]; then
  echo "OFFLINE_PARITY_FAIL: $FAIL case(s) failed" >&2
  exit 1
fi
if [[ "$PASS" -lt "$EXPECTED_CASES" ]]; then
  echo "OFFLINE_PARITY_FAIL: expected $EXPECTED_CASES cases, pass=$PASS" >&2
  exit 1
fi
echo "OFFLINE_PARITY_OK: all cases A/B/C/D green (still ≠ ran_on_npu)"
exit 0
