#!/usr/bin/env bash
# C4 offline Pass-evidence audit — soft skip until a real Pass snapshot exists.
#
# IMPL 3a14f38d: C4 readiness. Without a PassCandidate last_apply (json
# ran_on_npu=true + exact ok_npu_verified and/or ert_state=4 + not ert_error +
# ofm_raw_nonzero>0 + real_ioctl=true), soft-skip with C4_SKIP — do **not**
# invent Pass from offline green / C3_SKIP / CLASSIFY_SOFT_OK.
#
# What this does (offline only; NO L3 / NO EXEC / never sets ALLOW):
#   1) Resolve last_apply under load confinement (temp_dir ∪ testdata/xdna2_cmd)
#   2) cargo run --example c4_pass_check → pure evaluate_silicon_pass_evidence
#   3) Default baseline fixture → C4_SKIP no_pass_evidence exit 0
#   4) --require-pass → exit 2 if not PassCandidate
#
# Always honest: not_c_ready=1 invent_pass=0 even on PassCandidate (offline audit
# ≠ product Pass).
#
# Usage:
#   bash tools/c4_pass_check.sh
#   bash tools/c4_pass_check.sh /path/to/last_apply.json
#   bash tools/c4_pass_check.sh --require-pass
#   # bash (portable env prefix):
#   env XDNA2_LAB_LAST_APPLY_JSON=/path/to.json bash tools/c4_pass_check.sh
#   # fish:
#   set -gx XDNA2_LAB_LAST_APPLY_JSON /path/to.json; bash tools/c4_pass_check.sh
#
# Exit:
#   0  C4_SKIP (no pass evidence) or C4_PASS_CANDIDATE soft path
#   2  --require-pass and not PassCandidate; bad args; path refuse / missing explicit path
#
# Honesty: C4 without real Pass snapshot ≠ silicon progress.
# Soft exit 0 only when no fixture was resolvable (no argv path). Explicit path
# refuse/missing always exit 2.
set -euo pipefail
export PATH="/usr/bin${HOME:+:${HOME}/.local/bin}:${PATH:-}"
# Never inherit sticky userspace EXEC gate.
unset XDNA2_ALLOW_EXEC_CMD || true

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# TMPDIR=/ (or empty after strip) must not become "" and match every absolute path.
TMP_ROOT="${TMPDIR:-/tmp}"
TMP_ROOT="${TMP_ROOT%/}"
if [[ -z "$TMP_ROOT" || "$TMP_ROOT" == "/" ]]; then
  TMP_ROOT="/tmp"
fi

REQUIRE_PASS=0
IN=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --require-pass)
      REQUIRE_PASS=1
      shift
      ;;
    -h|--help)
      sed -n '1,40p' "$0" | grep -E '^#([^!]|$)' | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    -*)
      echo "ERROR: unknown $1 (allowlist: --require-pass --help)" >&2
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
  case "$p" in
    *'/..'|*'../'*|'..'|*/../*) return 1 ;;
  esac
  case "$p" in
    *.json) ;;
    *) return 1 ;;
  esac
  if [[ -L "$p" ]]; then
    return 1
  fi
  local abs
  if [[ "$p" = /* ]]; then
    abs="$p"
  else
    abs="$ROOT/$p"
  fi
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

set +e
resolve_in
resolve_rc=$?
set -e
if [[ "$resolve_rc" -ne 0 ]]; then
  # Explicit path refuse/missing → always exit 2 (never soft-skip path errors).
  if [[ "$resolve_rc" -eq 2 ]]; then
    echo "ERROR: c4_pass_check: path missing/refused ($IN)" >&2
    echo "C4_REFUSE path not_c_ready=1 not_ran_on_npu=1 invent_pass=0"
    exit 2
  fi
  # Soft only: no resolvable fixture when no argv path was given.
  if [[ "$REQUIRE_PASS" -eq 1 ]]; then
    echo "ERROR: no last_apply.json (pass path or run HITL C once)" >&2
    exit 2
  fi
  echo "NOTE: c4_pass_check: no last_apply.json — soft skip"
  echo "C4_SKIP no_fixture not_c_ready=1 not_ran_on_npu=1 invent_pass=0"
  exit 0
fi

echo "=== c4_pass_check (offline; ≠ invent Pass ≠ ran_on_npu claim) ==="
echo "IN: $IN"

EX_ARGS=("$IN")
if [[ "$REQUIRE_PASS" -eq 1 ]]; then
  EX_ARGS=("--require-pass" "$IN")
fi

set +e
cargo run --features amd-xdna2 --example c4_pass_check --quiet -- "${EX_ARGS[@]}"
ex_rc=$?
set -e

if [[ "$ex_rc" -eq 0 ]]; then
  exit 0
fi

if [[ "$REQUIRE_PASS" -eq 1 ]]; then
  echo "ERROR: c4_pass_check example rc=$ex_rc (--require-pass)" >&2
  exit 2
fi
echo "WARN: c4_pass_check example rc=$ex_rc (soft; not hard-fail)" >&2
echo "C4_WARN example_rc=$ex_rc not_c_ready=1 not_ran_on_npu=1 invent_pass=0"
exit 0
