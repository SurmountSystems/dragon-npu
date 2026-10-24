#!/usr/bin/env bash
# collect_load_accepted_candidate.sh — W5-4 fail-closed extract/collect helper
#
# Wraps capture_from_host_candidate classification + optional pass_unblock
# ingest. Never invents Pass, never claims load-accepted for multi-txn 32208
# or known research-duplicate public mlir-aie peels.
#
# Usage:
#   bash tools/collect_load_accepted_candidate.sh PATH [NAME]
#   bash tools/collect_load_accepted_candidate.sh --dry-run PATH [NAME]
#   bash tools/collect_load_accepted_candidate.sh --ingest PATH [NAME]
#   bash tools/collect_load_accepted_candidate.sh -h|--help
#
# Modes:
#   default / --dry-run  classify only (print axes; never copy)
#   --ingest             copy via pass_unblock_once --ingest only when classify
#                        does not refuse (multi-txn / research-duplicate /
#                        empty / refuse basename). Still never claims
#                        load_accepted=1 / residual_matched / Pass.
#
# Exit:
#   0  dry-run classify ok (or ingest completed as research material only)
#   2  refused (multi-txn / research-duplicate / empty / basename / missing)
#   3  usage
#
# Honesty footer always:
#   invent_pass=0 not_ran_on_npu=1 not_c_ready=1 Pass=No
#   load_accepted_claim=0 residual_matched_claim=0
#
# Related:
#   tools/capture_from_host_candidate.sh
#   tools/host_capture_scan.sh
#   tools/pass_unblock_once.sh --ingest
#   testdata/xdna2_cmd/external_known_good/PROVENANCE.md (load-accepted fields)
set -euo pipefail
export PATH="/usr/bin:${HOME}/.local/bin:${PATH:-}"
unset XDNA2_ALLOW_EXEC_CMD || true

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
BAG_DIR="$ROOT/testdata/xdna2_cmd/external_known_good"
CAPTURE_HOST_SH="$ROOT/tools/capture_from_host_candidate.sh"
PASS_UNBLOCK_SH="$ROOT/tools/pass_unblock_once.sh"
OPERATOR_CAPTURE_SH="$ROOT/tools/operator_external_capture.sh"

MULTI_TXN_BYTES=32208
MODE="dry-run" # dry-run | ingest
SRC=""
NAME=""

# Research-only public mlir-aie npu2 peels (ctrltext / insts sha16).
KNOWN_RESEARCH_SHA16=(
  aaeb5b68cc298a45
  de8671245bb72436
  7ee4b267ce31cd70
  18cb653c05a9d4c8
  34baaff69260033c
  ae692499a79c74b4
  e1e4db951d44092d
)

usage() {
  cat <<'EOF'
collect_load_accepted_candidate - fail-closed host candidate collect (W5-4)

Usage:
  bash tools/collect_load_accepted_candidate.sh PATH [NAME]
  bash tools/collect_load_accepted_candidate.sh --dry-run PATH [NAME]
  bash tools/collect_load_accepted_candidate.sh --ingest PATH [NAME]
  bash tools/collect_load_accepted_candidate.sh -h|--help

PATH   existing host capture file (ctrltext / insts / related)
NAME   optional single-component install basename for --ingest

Default is dry-run (classify only). --ingest calls pass_unblock_once only when
classify does not refuse multi-txn, research-duplicate sha16, empty, or bad
basename. Ingest still stores as research material under the capture dir —
it does NOT set load_accepted=1. Operator must fill PROVENANCE load-accepted
fields only when live ERT Completed + OFM allclose evidence exists.

Refuse as load-accepted (and as --ingest for residual match claim):
  - size 32208 (historical multi-txn wrong body)
  - sha16 matching known public mlir-aie research peels (300/660/2592/4512)
  - local oracle basenames (instr_post_patch, ve2, …) via pass_unblock refuse

Never: ALLOW=1, EXEC_CMD, unlock mint, invent Pass, invent capture bytes.
EOF
}

is_research_sha16() {
  local s="$1" k
  for k in "${KNOWN_RESEARCH_SHA16[@]}"; do
    [[ "$s" == "$k" ]] && return 0
  done
  return 1
}

refuse_basename() {
  local b bl
  b="$(basename "$1")"
  case "$b" in
    instr_post_patch.bin|instr_vendor_post_patch.bin|ctrl_post_patch.bin|\
    instr_raw_txn.bin|instr_vendor_txn_bin.bin|lab_instr.bin|lab_instr.elf|\
    validate.xclbin)
      return 0
      ;;
  esac
  bl="$(printf '%s' "$b" | tr '[:upper:]' '[:lower:]')"
  case "$bl" in
    *ve2*|*not_aie2*)
      return 0
      ;;
  esac
  return 1
}

safe_install_name() {
  local name="$1"
  [[ -n "$name" ]] || return 1
  if [[ "$name" == /* || "$name" == *'/'* || "$name" == *'\'* ]]; then
    return 1
  fi
  if [[ "$name" == "." || "$name" == ".." || "$name" == *..* ]]; then
    return 1
  fi
  if [[ ! "$name" =~ ^[A-Za-z0-9][A-Za-z0-9._-]*$ ]]; then
    return 1
  fi
  return 0
}

print_honesty() {
  echo "invent_pass=0"
  echo "not_ran_on_npu=1"
  echo "not_c_ready=1"
  echo "Pass=No"
  echo "load_accepted_claim=0"
  echo "residual_matched_claim=0"
  echo "product_known_good=0"
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h|--help)
      usage
      exit 0
      ;;
    --dry-run)
      MODE="dry-run"
      shift
      ;;
    --ingest)
      MODE="ingest"
      shift
      ;;
    --*)
      echo "ERROR: unknown flag: $1" >&2
      usage >&2
      exit 3
      ;;
    *)
      if [[ -z "$SRC" ]]; then
        SRC="$1"
      elif [[ -z "$NAME" ]]; then
        NAME="$1"
      else
        echo "ERROR: unexpected arg: $1" >&2
        exit 3
      fi
      shift
      ;;
  esac
done

if [[ -z "$SRC" ]]; then
  echo "ERROR: PATH required" >&2
  usage >&2
  exit 3
fi

echo "================================================================"
echo "collect_load_accepted_candidate - fail-closed (W5-4)"
echo "================================================================"
echo "ROOT=$ROOT"
echo "mode=$MODE"
echo "candidate=$SRC"
[[ -n "$NAME" ]] && echo "install_name=$NAME"
echo "Honesty: invent_pass=0 not_ran_on_npu=1 not_c_ready=1 Pass=No"
echo "No ALLOW / no EXEC_CMD / no unlock mint / no invent load-accepted"

if [[ ! -f "$SRC" ]]; then
  echo "REFUSE: candidate file missing"
  print_honesty
  exit 2
fi
if [[ ! -s "$SRC" ]]; then
  echo "REFUSE: empty file"
  print_honesty
  exit 2
fi

bytes=$(wc -c <"$SRC" | tr -d ' ')
sha256="$(sha256sum "$SRC" | awk '{print $1}')"
sha16="${sha256:0:16}"
echo "blob_len=$bytes"
echo "sha16=$sha16"
if [[ "${XDNA2_LAB_PRINT_FULL_SHA:-0}" == "1" ]]; then
  echo "sha256=$sha256"
else
  echo "sha256=omitted (set XDNA2_LAB_PRINT_FULL_SHA=1 for full hash)"
fi

multi_txn_refused=0
research_dup_refused=0
basename_refused=0
name_refused=0

if [[ "$bytes" -eq "$MULTI_TXN_BYTES" ]]; then
  multi_txn_refused=1
  echo "class=historical_multi_txn_wrong_body"
  echo "reason=size_32208_is_multi_class_not_lab_classic_1688"
fi

if is_research_sha16 "$sha16"; then
  research_dup_refused=1
  echo "class=research_duplicate_sha16"
  echo "reason=sha16_matches_known_public_mlir_aie_research_peel"
  echo "note=do_not_re_tag_300_660_2592_4512_as_load_accepted"
fi

if refuse_basename "$SRC"; then
  basename_refused=1
  echo "class=refused_basename"
  echo "reason=local_oracle_or_wrong_platform:$(basename "$SRC")"
fi

if [[ -n "$NAME" ]]; then
  if ! safe_install_name "$NAME"; then
    name_refused=1
    echo "class=unsafe_install_name"
  elif refuse_basename "$NAME"; then
    name_refused=1
    echo "class=refused_install_basename"
  fi
fi

# Soft size distance to lab classic 1688 (informational only).
if [[ "$bytes" -ne 1688 ]]; then
  if [[ "$bytes" -gt 1688 ]]; then
    echo "distance_hint_to_1688=$((bytes - 1688))"
  else
    echo "distance_hint_to_1688=$((1688 - bytes))"
  fi
  echo "note=not_lab_classic_1688_size"
fi

# Optional host candidate checklist (always residual_matched_claim=0).
if [[ -f "$CAPTURE_HOST_SH" ]]; then
  echo "--- capture_from_host_candidate (nested; soft) ---"
  set +e
  bash "$CAPTURE_HOST_SH" "$SRC"
  host_rc=$?
  set -e
  echo "capture_from_host_candidate_rc=$host_rc"
fi

echo "--- decision ---"
echo "multi_txn_refused=$multi_txn_refused"
echo "research_duplicate_refused=$research_dup_refused"
echo "basename_refused=$basename_refused"
echo "install_name_refused=$name_refused"
print_honesty

if [[ "$multi_txn_refused" -eq 1 ]]; then
  echo "REFUSE: multi-txn candidate must not be collected as load-accepted."
  echo "Operator material ask still open: true completing-stack capture under"
  echo "  testdata/xdna2_cmd/external_known_good/"
  exit 2
fi

if [[ "$research_dup_refused" -eq 1 ]]; then
  echo "REFUSE: research-duplicate sha16 (public mlir-aie AOT already known)."
  echo "Do not re-tag existing 300/660/2592/4512 peels as load-accepted."
  echo "Research-only ingest may still use: bash tools/pass_unblock_once.sh --ingest PATH"
  echo "  (that path is research material only; invent_pass=0; not residual match)."
  exit 2
fi

if [[ "$basename_refused" -eq 1 || "$name_refused" -eq 1 ]]; then
  echo "REFUSE: basename / install NAME not allowed for capture-dir install."
  exit 2
fi

# Completing-stack evidence flags (operator explicit). Still never auto-claim.
evidence_ok=0
if [[ "${XDNA2_CAPTURE_COMPLETING_STACK_EVIDENCE:-0}" == "1" \
   && "${XDNA2_CAPTURE_ALLOW_RESIDUAL_MATCHED_CLAIM:-0}" == "1" ]]; then
  evidence_ok=1
fi
echo "completing_stack_evidence_ok=$evidence_ok"
echo "note=evidence flags never invent load_accepted_claim in this helper"

if [[ "$MODE" == "dry-run" ]]; then
  echo "DRY_RUN: classify only; no copy; no Pass."
  if [[ "$evidence_ok" -eq 1 ]]; then
    echo "NOTE: evidence flags present but load_accepted_claim remains 0 here."
    echo "Operator fills PROVENANCE load-accepted table after live ERT Completed +"
    echo "OFM allclose (decide_ran_on_npu). Helper never sets invent_pass=1."
  fi
  echo "Next (research ingest only, still not load-accepted):"
  if [[ -n "$NAME" ]]; then
    echo "  bash tools/collect_load_accepted_candidate.sh --ingest $SRC $NAME"
  else
    echo "  bash tools/collect_load_accepted_candidate.sh --ingest $SRC"
  fi
  echo "Or: bash tools/pass_unblock_once.sh --ingest $SRC ${NAME:-}"
  echo "PROVENANCE template: testdata/xdna2_cmd/external_known_good/PROVENANCE.md"
  exit 0
fi

# --ingest: copy via pass_unblock / operator_external_capture (research only).
if [[ ! -f "$PASS_UNBLOCK_SH" && ! -f "$OPERATOR_CAPTURE_SH" ]]; then
  echo "REFUSE: missing pass_unblock_once and operator_external_capture"
  print_honesty
  exit 2
fi

echo "=== ingest (research material only; load_accepted_claim=0) ==="
set +e
if [[ -f "$PASS_UNBLOCK_SH" ]]; then
  if [[ -n "$NAME" ]]; then
    bash "$PASS_UNBLOCK_SH" --ingest "$SRC" "$NAME"
  else
    bash "$PASS_UNBLOCK_SH" --ingest "$SRC"
  fi
  irc=$?
else
  if [[ -n "$NAME" ]]; then
    bash "$OPERATOR_CAPTURE_SH" --install "$SRC" "$NAME"
  else
    bash "$OPERATOR_CAPTURE_SH" --install "$SRC"
  fi
  irc=$?
fi
set -e

echo "ingest_rc=$irc"
echo "NOTE: bytes under $BAG_DIR are research material until PROVENANCE"
echo "  load-accepted fields are filled from real completing evidence."
echo "  Ingest ≠ load-accepted ≠ residual match ≠ Pass ≠ ran_on_npu."
print_honesty
echo "Fill PROVENANCE load-accepted table only when completed_on_this_fw=yes"
echo "  with firmware version + stack + extract method documented."
exit "$irc"
