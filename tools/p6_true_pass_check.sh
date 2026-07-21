#!/usr/bin/env bash
# P6 offline true-Pass criteria check — soft skip until live product Pass.
#
# IMPL 4520851d: P6 readiness. Codifies exact product Pass requirements offline
# (ERT Completed / ok_npu_verified, non-zero OFM allclose via live decide,
# real_ioctl, json_ran_on_npu only when product path set it). Cites C4 audit;
# never invents product Pass from offline green / C4_PASS_CANDIDATE.
#
# What this does (offline only; NO L3 / NO EXEC / never sets ALLOW):
#   1) Resolve last_apply under load confinement (temp_dir ∪ testdata/xdna2_cmd)
#   2) Soft C4 audit (tools/c4_pass_check.sh) for PassCandidate snapshot gate
#   3) cargo run --example p6_true_pass_check → pure criteria + P6 status note
#   4) Default locked fixture → P6_NOT_PASS / P6_SKIP exit 0
#   5) --require-pass → exit 2 if not offline PassCandidate (still ≠ product Pass)
#
# Always honest: invent_pass=0 not_ran_on_npu=1 product_pass_requires_live_decide=1
# even on offline PassCandidate.
#
# Usage:
#   bash tools/p6_true_pass_check.sh
#   bash tools/p6_true_pass_check.sh /path/to/last_apply.json
#   bash tools/p6_true_pass_check.sh --require-pass
#   # bash (portable env prefix):
#   env XDNA2_LAB_LAST_APPLY_JSON=/path/to.json bash tools/p6_true_pass_check.sh
#   # fish:
#   set -gx XDNA2_LAB_LAST_APPLY_JSON /path/to.json; bash tools/p6_true_pass_check.sh
#
# Exit:
#   0  P6_SKIP / P6_NOT_PASS (no product Pass) or P6_OFFLINE_CANDIDATE soft path
#   2  --require-pass and not offline PassCandidate; bad args; path refuse / missing
#
# Honesty: P6 criteria ready offline ≠ silicon Pass ≠ ran_on_npu.
# Soft exit 0 only when no fixture was resolvable (no argv path). Explicit path
# refuse/missing always exit 2.
set -euo pipefail
export PATH="/usr/bin${HOME:+:${HOME}/.local/bin}:${PATH:-}"
# Never inherit sticky userspace EXEC gate.
unset XDNA2_ALLOW_EXEC_CMD || true

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# TMPDIR=/ (or empty after strip) must not become "" and match every absolute path.
# Soft smoke: bash tools/smoke_tmpdir_root_clamp.sh
TMP_ROOT="${TMPDIR:-/tmp}"
TMP_ROOT="${TMP_ROOT%/}"
if [[ -z "$TMP_ROOT" || "$TMP_ROOT" == "/" ]]; then
  TMP_ROOT="/tmp"
fi
if [[ -d "$TMP_ROOT" ]]; then
  resolved="$(realpath -m "$TMP_ROOT" 2>/dev/null || true)"
  if [[ -n "$resolved" ]]; then
    TMP_ROOT="$resolved"
  fi
fi
if [[ -z "$TMP_ROOT" || "$TMP_ROOT" == "/" || -L "$TMP_ROOT" ]]; then
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
      sed -n '1,45p' "$0" | grep -E '^#([^!]|$)' | sed 's/^# \{0,1\}//'
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
    echo "ERROR: p6_true_pass_check: path missing/refused ($IN)" >&2
    echo "P6_REFUSE path invent_pass=0 not_c_ready=1 not_ran_on_npu=1 product_pass_requires_live_decide=1"
    exit 2
  fi
  # Soft only: no resolvable fixture when no argv path was given.
  if [[ "$REQUIRE_PASS" -eq 1 ]]; then
    echo "ERROR: no last_apply.json (pass path or run HITL C once)" >&2
    exit 2
  fi
  echo "NOTE: p6_true_pass_check: no last_apply.json — soft skip"
  echo "P6_SKIP no_fixture invent_pass=0 not_c_ready=1 not_ran_on_npu=1 product_pass_requires_live_decide=1 offline_audit_only=1 product_pass=0"
  exit 0
fi

echo "=== p6_true_pass_check (offline; ≠ invent Pass ≠ ran_on_npu claim) ==="
echo "IN: $IN"
echo "NOTE: XDNA2_ALLOW_EXEC_CMD unset; never open L3; product Pass = live decide only"
echo

# Cite C4 first (same snapshot gate; never invent product Pass).
echo "--- (1/2) C4 offline PassCandidate audit ---"
C4_ARGS=("$IN")
if [[ "$REQUIRE_PASS" -eq 1 ]]; then
  C4_ARGS=("--require-pass" "$IN")
fi
set +e
bash tools/c4_pass_check.sh "${C4_ARGS[@]}"
c4_rc=$?
set -e
echo "c4_rc=$c4_rc"
echo

# P6 criteria + status (reuses evaluate_silicon_pass_evidence inside example).
echo "--- (2/2) P6 true-Pass criteria + status ---"
EX_ARGS=("$IN")
if [[ "$REQUIRE_PASS" -eq 1 ]]; then
  EX_ARGS=("--require-pass" "$IN")
fi

set +e
cargo run --features amd-xdna2 --example p6_true_pass_check --quiet -- "${EX_ARGS[@]}"
ex_rc=$?
set -e

if [[ "$ex_rc" -eq 0 && "$c4_rc" -eq 0 ]]; then
  echo
  echo "P6_CHECK_OK invent_pass=0 not_ran_on_npu=1 product_pass_requires_live_decide=1 offline_audit_only=1 product_pass=0"
  exit 0
fi

# Path / require-pass refuses: prefer exit 2 when either child hard-refused.
if [[ "$ex_rc" -eq 2 || "$c4_rc" -eq 2 ]]; then
  echo "ERROR: p6_true_pass_check refuse c4_rc=$c4_rc p6_example_rc=$ex_rc" >&2
  echo "P6_REFUSE invent_pass=0 not_c_ready=1 not_ran_on_npu=1 product_pass_requires_live_decide=1"
  exit 2
fi

if [[ "$REQUIRE_PASS" -eq 1 ]]; then
  echo "ERROR: p6_true_pass_check require-pass failed c4_rc=$c4_rc p6_example_rc=$ex_rc" >&2
  exit 2
fi

echo "WARN: p6_true_pass_check soft c4_rc=$c4_rc p6_example_rc=$ex_rc" >&2
echo "P6_WARN invent_pass=0 not_c_ready=1 not_ran_on_npu=1 product_pass_requires_live_decide=1"
exit 0
