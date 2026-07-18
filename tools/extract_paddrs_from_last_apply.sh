#!/usr/bin/env bash
# Best-effort: parse io_paddrs from last_apply.json detail → vendor_paddrs-compatible JSON.
#
# Offline only. No L3 / no EXEC / ≠ ran_on_npu.
#
# Usage:
#   bash tools/extract_paddrs_from_last_apply.sh
#   bash tools/extract_paddrs_from_last_apply.sh /path/to/last_apply.json
#   bash tools/extract_paddrs_from_last_apply.sh --out /tmp/my_paddrs.json
#
# Default OUT: testdata/xdna2_cmd/gt_capture/vendor_paddrs_from_last_apply.json
# (does NOT overwrite vendor_paddrs.json from capture_mladf_instr_vendor)
#
# Exit: 0 wrote JSON; 2 usage/missing; 3 no io_paddrs / zero ifm|wts|ofm
set -euo pipefail
export PATH="/usr/bin:${HOME}/.local/bin:${PATH}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

IN=""
OUT="$ROOT/testdata/xdna2_cmd/gt_capture/vendor_paddrs_from_last_apply.json"
FORCE=0
DEFAULT_CANDIDATES=(
  "$ROOT/testdata/xdna2_cmd/last_apply.json"
  "/tmp/dragon_npu_last_apply.json"
)

while [[ $# -gt 0 ]]; do
  case "$1" in
    --out)
      OUT="${2:-}"
      [[ -n "$OUT" ]] || { echo "ERROR: --out needs path" >&2; exit 2; }
      shift 2
      ;;
    --force)
      FORCE=1
      shift
      ;;
    -h|--help)
      # Only comment header lines 1–20: filter ^# (skip shebang #!)
      sed -n '1,20p' "$0" | grep -E '^#([^!]|$)' | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    -*)
      echo "ERROR: unknown $1" >&2
      exit 2
      ;;
    *)
      IN="$1"
      shift
      ;;
  esac
done

if [[ -z "$IN" ]]; then
  for c in "${DEFAULT_CANDIDATES[@]}"; do
    if [[ -f "$c" ]]; then
      IN="$c"
      break
    fi
  done
  # Newest /tmp/dragon_npu_last_apply_*.json without ls|head under pipefail
  if [[ -z "$IN" ]]; then
    newest=""
    newest_mtime=0
    shopt -s nullglob
    for f in /tmp/dragon_npu_last_apply_*.json; do
      [[ -f "$f" ]] || continue
      mt=$(stat -c %Y "$f" 2>/dev/null || echo 0)
      if [[ -z "$newest" || "$mt" -gt "$newest_mtime" ]]; then
        newest="$f"
        newest_mtime="$mt"
      fi
    done
    shopt -u nullglob
    if [[ -n "$newest" && -f "$newest" ]]; then
      IN="$newest"
    fi
  fi
fi

[[ -n "$IN" && -f "$IN" ]] || {
  echo "ERROR: no last_apply.json found (pass path or run HITL C once)" >&2
  exit 2
}

# Refuse clobbering vendor capture paddrs unless --force
out_base="$(basename -- "$OUT")"
if [[ "$out_base" == "vendor_paddrs.json" && -f "$OUT" && "$FORCE" -eq 0 ]]; then
  if grep -q 'capture_mladf_instr_vendor' "$OUT" 2>/dev/null; then
    echo "ERROR: --out $OUT looks like vendor capture (source capture_mladf_instr_vendor)." >&2
    echo "  Default is vendor_paddrs_from_last_apply.json; use --force to overwrite." >&2
    exit 2
  fi
fi

echo "IN:  $IN"
echo "OUT: $OUT"

python3 - "$IN" "$OUT" <<'PY'
import json, re, sys
from pathlib import Path

src, out = Path(sys.argv[1]), Path(sys.argv[2])
try:
    d = json.loads(src.read_text())
except Exception as e:
    print(f"ERROR: JSON parse: {e}", file=sys.stderr)
    sys.exit(2)

detail = d.get("detail") or ""
if not isinstance(detail, str):
    detail = str(detail)

# io_paddrs ifm=0x… wts=0x… ofm=0x… ctrl=0x…
m = re.search(
    r"io_paddrs\s+ifm=(0x[0-9a-fA-F]+)\s+wts=(0x[0-9a-fA-F]+)\s+ofm=(0x[0-9a-fA-F]+)\s+ctrl=(0x[0-9a-fA-F]+)",
    detail,
)
if not m:
    print("ERROR: no io_paddrs in detail", file=sys.stderr)
    print("  detail snippet:", detail[:200], file=sys.stderr)
    sys.exit(3)

ifm_s, wts_s, ofm_s, ctrl_s = m.groups()
try:
    ifm_i = int(ifm_s, 16)
    wts_i = int(wts_s, 16)
    ofm_i = int(ofm_s, 16)
    ctrl_i = int(ctrl_s, 16)
except ValueError as e:
    print(f"ERROR: bad hex paddr: {e}", file=sys.stderr)
    sys.exit(3)

# ifm/wts/ofm must be valid device paddrs (≠0 and ≠u64::MAX); ctrl may be 0 on some layouts
U64_MAX = (1 << 64) - 1


def valid_device_paddr(a: int) -> bool:
    return a != 0 and a != U64_MAX


paddrs_known = (
    valid_device_paddr(ifm_i)
    and valid_device_paddr(wts_i)
    and valid_device_paddr(ofm_i)
)
if not paddrs_known:
    print(
        f"ERROR: invalid paddr among ifm/wts/ofm (0 or 0xffffffffffffffff) "
        f"(ifm={ifm_s} wts={wts_s} ofm={ofm_s}) — refuse",
        file=sys.stderr,
    )
    sys.exit(3)

body = {
    "schema_version": 1,
    "source": "extract_paddrs_from_last_apply",
    "capture_mode": "last_apply_detail",
    "last_apply_path": str(src),
    "ifm_paddr": ifm_s,
    "wts_paddr": wts_s,
    "ofm_paddr": ofm_s,
    "ctrl_paddr": ctrl_s,
    "paddrs_known": True,
    "status": d.get("status"),
    "layout": d.get("layout"),
    "ran_on_npu": d.get("ran_on_npu", False),
    "note": "Best-effort pure-direct re-patch paddrs from last_apply detail; not vendor capture",
}
out.parent.mkdir(parents=True, exist_ok=True)
out.write_text(json.dumps(body, indent=2) + "\n")
print(f"OK: wrote {out}")
print(f"  ifm={ifm_s} wts={wts_s} ofm={ofm_s} ctrl={ctrl_s}")
print("Next (does NOT auto re-patch — export then dump):")
print(f"  export XDNA2_GT_PADDRS_JSON={out}")
print("  bash tools/dump_mladf_gt_offline.sh")
print("  bash tools/diff_vendor_instr_parity.sh")
sys.exit(0)
PY
