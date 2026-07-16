#!/usr/bin/env bash
# Offline validate an operator-supplied or exported mladf WTS pack (no device).
#
# Checks (fail-closed):
#   - file exists
#   - exact size 174080 (staged a16w4 key contract; override with EXPECT_WTS_BYTES)
#   - not all zeros
#
# Does **not** verify AMD shuffle byte-correctness beyond non-zero + length.
# Does **not** claim silicon Pass.
#
# Usage:
#   bash tools/validate_mladf_wts.sh PATH/to/wts.bin
#   EXPECT_WTS_BYTES=174080 bash tools/validate_mladf_wts.sh PATH
#
# Exit 0 = Pass-candidate length+nonzero gate (same as Rust
# validate_mladf_wts_packed_for_pass). Exit 2 = validation failed.
set -euo pipefail
export PATH="/usr/bin:${HOME}/.local/bin:${PATH}"

EXPECT="${EXPECT_WTS_BYTES:-174080}"

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  sed -n '1,25p' "$0" | sed 's/^# \?//'
  exit 0
fi
if [[ $# -lt 1 ]]; then
  sed -n '1,25p' "$0" | sed 's/^# \?//' >&2
  echo "ERROR: missing PATH (usage: bash tools/validate_mladf_wts.sh PATH)" >&2
  exit 2
fi

WTS="$1"
fail() { echo "ERROR: $*" >&2; exit 2; }

[[ -f "$WTS" ]] || fail "not a file: $WTS"
got="$(wc -c <"$WTS" | tr -d ' ')"
[[ "$got" == "$EXPECT" ]] || fail "WTS size $got != expected $EXPECT ($WTS)"

if [[ "$got" -eq 0 ]]; then
  fail "WTS is empty"
fi

# All-zero refuse (matches Rust bytes_are_all_zero + validate_mladf_wts_packed_for_pass).
# cmp -n vs /dev/zero: 0=equal (all zero), 1=differ (ok), other=tool fail → refuse.
cmp_rc=0
cmp -n "$EXPECT" -s -- "$WTS" /dev/zero || cmp_rc=$?
if [[ $cmp_rc -eq 0 ]]; then
  fail "WTS is all zeros — not AMD-packed (use tools/export_mladf_wts.sh or capture)"
fi
if [[ $cmp_rc -ne 1 ]]; then
  fail "zero-check failed for $WTS (cmp exit $cmp_rc)"
fi

nz="$(tr -d '\0' <"$WTS" | wc -c | tr -d ' ')"
# tr -d '\0' removes zero bytes; remaining count is nonzero count (may be slightly off for binary; ok for report)
# Better: python one-liner for accurate nz if available
if command -v python3 >/dev/null 2>&1; then
  nz="$(python3 -c "import sys; b=open(sys.argv[1],'rb').read(); print(sum(1 for x in b if x))" "$WTS")"
fi

echo "validate_mladf_wts: OK"
echo "  path:     $WTS"
echo "  size:     $got"
echo "  nonzero:  ${nz:-unknown}"
echo "  note:     length+nonzero only — not a silicon Pass / not shuffle audit"
echo "  next:     export XDNA2_LAB_WTS_PACKED=$WTS"
echo "            export XDNA2_LAB_REQUIRE_NONZERO_WTS=1"
