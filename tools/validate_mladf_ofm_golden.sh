#!/usr/bin/env bash
# Offline validate an OFM golden for mladf a16w4 Pass readiness (no device).
#
# Checks (fail-closed):
#   - file exists and non-empty
#   - size is bf16 m*n*2 OR f32 m*n*4 (default staged m=1 n=256 → 512 or 1024)
#   - not all zeros (decoded ±0.0 when python3 available; else byte-identical to /dev/zero)
#
# Escape hatch (plumbing only — **not** Pass candidate):
#   XDNA2_LAB_ALLOW_ZERO_GOLDEN=1  (preferred; matches Rust)
#   ALLOW_ZERO_GOLDEN=1            (legacy alias)
#   With ALLOW: exit **3** (WARN), never exit 0 — so Pass preflight cannot treat as OK.
#
# Does **not** claim silicon Pass / ran_on_npu.
# Product decide always refuses all-zero golden even if ALLOW is set.
#
# Usage:
#   bash tools/validate_mladf_ofm_golden.sh PATH/to/ofm_golden.bin
#   GOLDEN_M=1 GOLDEN_N=256 bash tools/validate_mladf_ofm_golden.sh PATH
#
# Exit:
#   0 = Pass-candidate size+nonzero gate (same spirit as Rust
#       validate_mladf_ofm_golden_packed_for_pass without allow_zero)
#   2 = validation failed (missing/wrong size/zero without ALLOW)
#   3 = all-zero allowed for plumbing only (ALLOW set; not Pass-ready)
set -euo pipefail
export PATH="/usr/bin:${HOME}/.local/bin:${PATH}"

M="${GOLDEN_M:-1}"
N="${GOLDEN_N:-256}"
# Prefer Rust env name; accept legacy shell alias.
ALLOW_ZERO="${XDNA2_LAB_ALLOW_ZERO_GOLDEN:-${ALLOW_ZERO_GOLDEN:-0}}"

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  sed -n '1,32p' "$0" | sed 's/^# \?//'
  exit 0
fi
if [[ $# -lt 1 ]]; then
  sed -n '1,32p' "$0" | sed 's/^# \?//' >&2
  echo "ERROR: missing PATH (usage: bash tools/validate_mladf_ofm_golden.sh PATH)" >&2
  exit 2
fi

GOLDEN="$1"
fail() { echo "ERROR: $*" >&2; exit 2; }

[[ -f "$GOLDEN" ]] || fail "not a file: $GOLDEN"
got="$(wc -c <"$GOLDEN" | tr -d ' ')"
[[ "$got" -gt 0 ]] || fail "OFM golden is empty: $GOLDEN"

if ! [[ "$M" =~ ^[1-9][0-9]*$ && "$N" =~ ^[1-9][0-9]*$ ]]; then
  fail "GOLDEN_M/GOLDEN_N must be positive integers (got M=$M N=$N)"
fi

# shellcheck disable=SC2003
BF16_LEN=$((M * N * 2))
F32_LEN=$((M * N * 4))
if [[ "$got" != "$BF16_LEN" && "$got" != "$F32_LEN" ]]; then
  fail "OFM golden size $got is neither bf16 m*n*2=$BF16_LEN nor f32 m*n*4=$F32_LEN (m=$M n=$N)"
fi

dtype="bf16"
elem_bytes=2
if [[ "$got" == "$F32_LEN" ]]; then
  dtype="f32"
  elem_bytes=4
fi

# Decoded ±0.0 gate (matches Rust ofm_golden_is_all_zero / f32_slice_is_all_zero).
# Falls back to cmp vs /dev/zero when python3 unavailable (bytes-only; see note).
is_all_zero_decoded=0
if command -v python3 >/dev/null 2>&1; then
  if python3 - "$GOLDEN" "$elem_bytes" <<'PY'
import struct, sys
path, eb = sys.argv[1], int(sys.argv[2])
b = open(path, "rb").read()
if len(b) % eb != 0:
    sys.exit(2)
if eb == 4:
    n = len(b) // 4
    vals = struct.unpack(f"<{n}f", b)
    # f32: exact ±0.0
    allz = all(v == 0.0 for v in vals)
elif eb == 2:
    # bf16 LE → promote high bits into f32, compare == 0.0 (covers ±0 bf16 0x0000/0x8000)
    allz = True
    for i in range(0, len(b), 2):
        bits = int.from_bytes(b[i:i+2], "little")
        f32_bits = bits << 16
        v = struct.unpack("<f", f32_bits.to_bytes(4, "little"))[0]
        if v != 0.0:
            allz = False
            break
else:
    sys.exit(2)
sys.exit(0 if allz else 1)
PY
  then
    is_all_zero_decoded=1
  fi
else
  cmp_rc=0
  cmp -n "$got" -s -- "$GOLDEN" /dev/zero || cmp_rc=$?
  if [[ $cmp_rc -eq 0 ]]; then
    is_all_zero_decoded=1
  elif [[ $cmp_rc -ne 1 ]]; then
    fail "zero-check failed for $GOLDEN (cmp exit $cmp_rc)"
  fi
fi

if [[ "$is_all_zero_decoded" -eq 1 ]]; then
  if [[ "$ALLOW_ZERO" == "1" || "$ALLOW_ZERO" == "true" || "$ALLOW_ZERO" == "TRUE" ]]; then
    echo "WARN: OFM golden is all zeros (±0 decoded) — XDNA2_LAB_ALLOW_ZERO_GOLDEN/ALLOW_ZERO_GOLDEN set" >&2
    echo "WARN: plumbing only — NOT Pass-candidate (exit 3; product decide still refuses)" >&2
    echo "validate_mladf_ofm_golden: PLUMBING_ZERO_ONLY"
    echo "  path:     $GOLDEN"
    echo "  size:     $got ($dtype LE for m=$M n=$N)"
    echo "  nonzero:  0"
    echo "  note:     exit 3 — cannot combine with Pass readiness / --require-pass-assets"
    exit 3
  fi
  fail "OFM golden is all zeros (±0) — refuse Pass readiness (pack ofm_expected.bin is not a Pass golden; capture or host-ref after real WTS). XDNA2_LAB_ALLOW_ZERO_GOLDEN=1 only for plumbing (exits 3, not Pass-ready)"
fi

nz="unknown"
if command -v python3 >/dev/null 2>&1; then
  nz="$(python3 -c "import sys; b=open(sys.argv[1],'rb').read(); print(sum(1 for x in b if x))" "$GOLDEN")"
fi

echo "validate_mladf_ofm_golden: OK"
echo "  path:     $GOLDEN"
echo "  size:     $got ($dtype LE for m=$M n=$N)"
echo "  nonzero:  ${nz}"
echo "  note:     length+nonzero only — not silicon Pass / not WTS-matched golden audit"
echo "  next:     export XDNA2_LAB_OFM_GOLDEN=$GOLDEN"
echo "            export XDNA2_LAB_OFM=1"
echo "            # f32 ones host_ref is invalid for a16w4 — use captured/non-zero golden"
