#!/usr/bin/env bash
# Soft offline residual class report from last_apply.json (no L3 / no EXEC).
#
# Uses pure unit-tested classifier via examples/classify_last_apply:
#   classify_last_apply_residual_ex / parse_last_apply_detail_signals /
#   format_silicon_residual_class_note
#
# Usage:
#   bash tools/classify_last_apply_residual.sh
#   bash tools/classify_last_apply_residual.sh /path/to/last_apply.json
#   # bash (portable env prefix):
#   env XDNA2_LAB_LAST_APPLY_JSON=/path/to.json bash tools/classify_last_apply_residual.sh
#   # fish:
#   set -gx XDNA2_LAB_LAST_APPLY_JSON /path/to.json; bash tools/classify_last_apply_residual.sh
#
# Soft default (for silicon_contract_check):
#   - missing fixture → CLASSIFY_SKIP + exit 0
#   - cargo/example fail → WARN + exit 0
#   - successful report → CLASSIFY_SOFT_OK + exit 0
# Never claims C-ready / ran_on_npu. CONTRACT_OK ≠ C-ready.
#
# Optional:
#   --require   exit 2 if no last_apply or example fails (hard offline research)
set -euo pipefail
export PATH="/usr/bin:${HOME}/.local/bin:${PATH}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# Match Rust std::env::temp_dir() on Unix (TMPDIR or /tmp).
TMP_ROOT="${TMPDIR:-/tmp}"
# Strip trailing slash for clean joins.
TMP_ROOT="${TMP_ROOT%/}"

REQUIRE=0
IN=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --require)
      REQUIRE=1
      shift
      ;;
    -h|--help)
      sed -n '1,30p' "$0" | grep -E '^#([^!]|$)' | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    -*)
      echo "ERROR: unknown $1" >&2
      exit 2
      ;;
    *)
      if [[ -n "$IN" ]]; then
        echo "ERROR: only one path argument allowed (got '$IN' and '$1')" >&2
        exit 2
      fi
      IN="$1"
      shift
      ;;
  esac
done

# Load confinement: under TMP_ROOT or repo testdata/xdna2_cmd, .json only, no .., not symlink.
path_load_allowed() {
  local p="$1"
  [[ -n "$p" ]] || return 1
  # Refuse parent-dir components (string scan; matches Rust ParentDir refuse).
  case "$p" in
    *'/..'|*'../'*|'..'|*/../*) return 1 ;;
  esac
  case "$p" in
    *.json) ;;
    *) return 1 ;;
  esac
  # Symlink refuse (defense-in-depth; example also refuses).
  if [[ -L "$p" ]]; then
    return 1
  fi
  local abs
  if [[ "$p" = /* ]]; then
    abs="$p"
  else
    abs="$ROOT/$p"
  fi
  # Normalize via realpath when file exists; else string-prefix under known roots.
  if [[ -e "$abs" ]]; then
    abs="$(realpath -m "$abs" 2>/dev/null || echo "$abs")"
  fi
  case "$abs" in
    "$TMP_ROOT"/*|"$ROOT/testdata/xdna2_cmd"/*) return 0 ;;
    *) return 1 ;;
  esac
}

DEFAULT_CANDIDATES=(
  "${XDNA2_LAB_LAST_APPLY_JSON:-}"
  "${XDNA2_LAST_APPLY_JSON:-}"
  "$ROOT/testdata/xdna2_cmd/last_apply.json"
)

resolve_in() {
  if [[ -n "$IN" ]]; then
    if ! path_load_allowed "$IN"; then
      echo "ERROR: path refused by load confinement (temp_dir|testdata/xdna2_cmd, .json, no symlink): $IN" >&2
      return 2
    fi
    if [[ -f "$IN" ]]; then
      return 0
    fi
    echo "ERROR: path not a file: $IN" >&2
    return 2
  fi
  for c in "${DEFAULT_CANDIDATES[@]}"; do
    [[ -n "$c" ]] || continue
    if ! path_load_allowed "$c"; then
      echo "WARN: candidate refused by load confinement: $c" >&2
      continue
    fi
    if [[ -f "$c" ]]; then
      IN="$c"
      return 0
    fi
  done
  # Newest pid-suffixed snapshot under TMPDIR (writer: dragon_npu_last_apply_{pid}.json).
  # Do NOT accept fixed plant name dragon_npu_last_apply.json.
  local newest="" newest_mtime=0 f mt
  shopt -s nullglob
  for f in "$TMP_ROOT"/dragon_npu_last_apply_*.json; do
    path_load_allowed "$f" || continue
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
    return 0
  fi
  return 1
}

# Capture resolve status before any `!` invert (bash $? after `!` is 0/1 only).
set +e
resolve_in
resolve_rc=$?
set -e
if [[ "$resolve_rc" -ne 0 ]]; then
  if [[ "$resolve_rc" -eq 2 ]]; then
    # Explicit path given but missing/refused — soft unless --require
    if [[ "$REQUIRE" -eq 1 ]]; then
      exit 2
    fi
    echo "WARN: classify_last_apply_residual: path missing/refused ($IN) — soft skip" >&2
    echo "CLASSIFY_SKIP missing_path not_c_ready=1 not_ran_on_npu=1"
    exit 0
  fi
  if [[ "$REQUIRE" -eq 1 ]]; then
    echo "ERROR: no last_apply.json (pass path or run HITL C once)" >&2
    exit 2
  fi
  echo "NOTE: classify_last_apply_residual: no last_apply.json — soft skip"
  echo "CLASSIFY_SKIP no_fixture not_c_ready=1 not_ran_on_npu=1"
  exit 0
fi

echo "=== classify_last_apply_residual (soft; ≠ C-ready) ==="
echo "IN: $IN"

set +e
cargo run --features amd-xdna2 --example classify_last_apply --quiet -- "$IN"
ex_rc=$?
set -e

if [[ "$ex_rc" -eq 0 ]]; then
  echo "CLASSIFY_SOFT_OK (report only; not_c_ready=1 not_ran_on_npu=1; CONTRACT_OK≠C-ready)"
  exit 0
fi

if [[ "$REQUIRE" -eq 1 ]]; then
  echo "ERROR: classify_last_apply example rc=$ex_rc" >&2
  exit 2
fi
echo "WARN: classify_last_apply example rc=$ex_rc (soft; not hard-fail)" >&2
echo "CLASSIFY_WARN example_rc=$ex_rc not_c_ready=1 not_ran_on_npu=1"
exit 0
