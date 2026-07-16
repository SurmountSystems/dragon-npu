#!/usr/bin/env bash
# Offline validate an operator-supplied / staged mladf control packet (no device).
#
# Checks (fail-closed):
#   - file exists
#   - exact size 29664 (staged a16w4 key contract; override with EXPECT_CTRL_BYTES)
#   - not empty
#   - not all zeros
#
# Does **not** verify control-packet semantics, paddr patch correctness, or silicon Pass.
# Does **not** apply meta patch locations (Rust: patch_ctrl_from_meta_io_paddrs).
#
# Usage:
#   bash tools/validate_mladf_ctrl.sh PATH/to/lab_ctrl.bin
#   EXPECT_CTRL_BYTES=29664 bash tools/validate_mladf_ctrl.sh PATH
#
# Exit 0 = Pass-candidate length+nonzero gate (same as Rust
# validate_mladf_ctrl_packed_for_pass). Exit 2 = validation failed.
set -euo pipefail
export PATH="/usr/bin:${HOME}/.local/bin:${PATH}"

EXPECT="${EXPECT_CTRL_BYTES:-29664}"

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  sed -n '1,25p' "$0" | sed 's/^# \?//'
  exit 0
fi
if [[ $# -lt 1 ]]; then
  sed -n '1,25p' "$0" | sed 's/^# \?//' >&2
  echo "ERROR: missing PATH (usage: bash tools/validate_mladf_ctrl.sh PATH)" >&2
  exit 2
fi

CTRL="$1"
fail() { echo "ERROR: $*" >&2; exit 2; }

[[ -f "$CTRL" ]] || fail "not a file: $CTRL"
got="$(wc -c <"$CTRL" | tr -d ' ')"
[[ "$got" == "$EXPECT" ]] || fail "CTRL size $got != expected $EXPECT ($CTRL)"

if [[ "$got" -eq 0 ]]; then
  fail "CTRL is empty"
fi

# All-zero refuse (matches Rust bytes_are_all_zero + validate_mladf_ctrl_packed_for_pass).
# cmp -n vs /dev/zero: 0=equal (all zero), 1=differ (ok), other=tool fail → refuse.
cmp_rc=0
cmp -n "$EXPECT" -s -- "$CTRL" /dev/zero || cmp_rc=$?
if [[ $cmp_rc -eq 0 ]]; then
  fail "CTRL is all zeros — refuse Pass readiness (stage via tools/stage_silicon_lab_assets.sh)"
fi
if [[ $cmp_rc -ne 1 ]]; then
  fail "zero-check failed for $CTRL (cmp exit $cmp_rc)"
fi

nz="unknown"
if command -v python3 >/dev/null 2>&1; then
  nz="$(python3 -c "import sys; b=open(sys.argv[1],'rb').read(); print(sum(1 for x in b if x))" "$CTRL")"
fi

echo "validate_mladf_ctrl: OK"
echo "  path:     $CTRL"
echo "  size:     $got"
echo "  nonzero:  ${nz}"
echo "  note:     length+nonzero only — not silicon Pass / not patch audit"
echo "  next:     export XDNA2_LAB_CTRL_PACKED=$CTRL"
echo "            export XDNA2_LAB_REQUIRE_CTRL=1"
echo "  residual: meta size=6 read_add_write paddr apply is lab u64-LE overwrite only"
