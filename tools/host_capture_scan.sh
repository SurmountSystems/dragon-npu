#!/usr/bin/env bash
# host_capture_scan.sh — W5-2 read-only host capture material scan
#
# Purpose: walk common host roots for capture-like binaries (ctrltext, insts,
# post_patch, xclbin, pdi, elf) and print a short scorecard: path, size,
# sha16, first 8 bytes hex, refuse class (multi-txn / research-duplicate /
# large / candidate). Does not copy, install, open L3, or invent Pass.
#
# Usage:
#   bash tools/host_capture_scan.sh
#   bash tools/host_capture_scan.sh --report
#   bash tools/host_capture_scan.sh --stdout-only
#   bash tools/host_capture_scan.sh -h|--help
#
# Always exit 0 (soft scan). Never: sudo, modprobe, EXEC_CMD, ALLOW=1,
# invent capture bytes, network install, live apply.
#
# Footer always:
#   invent_pass=0 not_ran_on_npu=1 not_c_ready=1 Pass=No load_accepted_claim=0
#
# Related:
#   bash tools/host_npu_stack_inventory.sh
#   bash tools/capture_from_host_candidate.sh [PATH]
#   bash tools/collect_load_accepted_candidate.sh PATH [NAME]
#   bash tools/pass_unblock_once.sh --ingest PATH [NAME]
set -euo pipefail
export PATH="/usr/bin:${HOME}/.local/bin:${PATH:-}"
unset XDNA2_ALLOW_EXEC_CMD || true

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
BAG_DIR="$ROOT/testdata/xdna2_cmd/external_known_good"
REPORT_MD="$ROOT/.agents/reports/wave5-host-capture-scan.md"
WRITE_REPORT=0
STDOUT_ONLY=0

# Bound walks so a huge tree cannot hang the agent.
MAXDEPTH=5
LIST_CAP=48
# Multi-txn wrong-body historical class (HITL false CLASS_HINT C era).
MULTI_TXN_BYTES=32208
# Soft "large refuse" for load-accepted ctrltext path (not a hard ban on scan).
LARGE_CTRLTEXT_SOFT=65536

# Research-only public mlir-aie npu2 peels already bagged (sha16 of ctrltext).
# Matching sha alone ≠ new load-accepted material.
KNOWN_RESEARCH_SHA16=(
  aaeb5b68cc298a45  # vector_scalar_add 300 B
  de8671245bb72436  # single_core matmul 660 B
  7ee4b267ce31cd70  # whole_array 2592 B
  18cb653c05a9d4c8  # cascade 4512 B
  34baaff69260033c  # single_core raw insts
  ae692499a79c74b4  # whole_array raw insts
  e1e4db951d44092d  # cascade raw insts
)

usage() {
  cat <<'EOF'
host_capture_scan - read-only host capture material scan (W5-2)

Usage:
  bash tools/host_capture_scan.sh
  bash tools/host_capture_scan.sh --report
  bash tools/host_capture_scan.sh --stdout-only
  bash tools/host_capture_scan.sh -h|--help

What it does (all soft / exit 0)
 - Walk candidate roots: /tmp, ~/.cache, ryzen-ai-install, mlir-aie-capture,
   RyzenAI-SW, XRT sample paths, FastFlowLM / Lemonade paths if present
 - For each matching binary: size, sha16, first 8 B hex, refuse class
 - Note multi-txn 32208, known research-duplicate sha16, large soft class
 - Optional durable report under .agents/reports/wave5-host-capture-scan.md

Honesty
  invent_pass=0 not_ran_on_npu=1 not_c_ready=1 Pass=No load_accepted_claim=0
  Scan ≠ load-accepted. Research-only public AOT sha match ≠ residual match.
  Multi-txn 32208 must not be claimed load-accepted.

Related
  tools/host_npu_stack_inventory.sh   (broader stack probe)
  tools/collect_load_accepted_candidate.sh  (fail-closed collect + optional ingest)
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --report)
      WRITE_REPORT=1
      shift
      ;;
    --stdout-only)
      STDOUT_ONLY=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "ERROR: unknown arg: $1 (allowlist: --report --stdout-only --help)" >&2
      exit 2
      ;;
  esac
done

# Default: write report when not --stdout-only (join-friendly).
if [[ "$STDOUT_ONLY" -eq 0 ]]; then
  WRITE_REPORT=1
fi

OUT_BUF=""
emit() {
  printf '%s\n' "$*"
  OUT_BUF+="$*"$'\n'
}

is_research_sha16() {
  local s="$1"
  local k
  for k in "${KNOWN_RESEARCH_SHA16[@]}"; do
    if [[ "$s" == "$k" ]]; then
      return 0
    fi
  done
  return 1
}

# Soft basename filter for capture-like material.
is_capture_like_basename() {
  local b bl
  b="$(basename "$1")"
  bl="$(printf '%s' "$b" | tr '[:upper:]' '[:lower:]')"
  case "$bl" in
    working_stack*.bin|*ctrltext*.bin|*.ctrltext|\
    instr_vendor_post_patch.bin|instr_post_patch.bin|ctrl_post_patch.bin|\
    *insts*.bin|*.pdi|*.xclbin|*.elf|\
    *transaction*.bin|*txn*.bin)
      return 0
      ;;
  esac
  return 1
}

# Classify a present file (stdout tokens; never invent load-accepted).
classify_file() {
  local f="$1"
  local bytes sha16 head8 refuse_class note
  bytes="$(wc -c <"$f" 2>/dev/null | tr -d ' ' || echo 0)"
  if [[ "${bytes:-0}" -lt 32 ]]; then
    return 0
  fi
  # sha16 of full file (portable).
  sha16="$(sha256sum "$f" 2>/dev/null | awk '{print substr($1,1,16)}' || echo unknown)"
  # First 8 bytes as hex (python3 if present; else od).
  head8="unknown"
  if command -v python3 >/dev/null 2>&1; then
    head8="$(python3 -c 'import sys; d=open(sys.argv[1],"rb").read(8); print(d.hex())' "$f" 2>/dev/null || echo unknown)"
  elif command -v od >/dev/null 2>&1; then
    head8="$(od -An -tx1 -N8 "$f" 2>/dev/null | tr -d ' \n' || echo unknown)"
  fi

  refuse_class="none"
  note="host_candidate_not_proven"
  if [[ "$bytes" -eq "$MULTI_TXN_BYTES" ]]; then
    refuse_class="multi_txn_32208"
    note="historical_multi_txn_wrong_body_not_load_accepted"
  elif is_research_sha16 "$sha16"; then
    refuse_class="research_duplicate_sha16"
    note="matches_known_public_mlir_aie_research_peel"
  elif [[ "$bytes" -ge "$LARGE_CTRLTEXT_SOFT" ]]; then
    # Soft note only — xclbin/pdi can be large and still research-useful.
    case "$(basename "$f" | tr '[:upper:]' '[:lower:]')" in
      *.xclbin|*.pdi|*.elf)
        refuse_class="large_container_soft"
        note="large_container_scan_only_not_ctrltext_claim"
        ;;
      *)
        refuse_class="large_soft"
        note="size_ge_${LARGE_CTRLTEXT_SOFT}_soft_not_auto_load_accepted"
        ;;
    esac
  fi

  # Never claim load-accepted from scan alone.
  printf 'HIT bytes=%s sha16=%s head8=%s refuse_class=%s load_accepted_claim=0 invent_pass=0 path=%s note=%s\n' \
    "$bytes" "$sha16" "$head8" "$refuse_class" "$f" "$note"
}

# Candidate roots (missing OK).
SCAN_ROOTS=(
  /tmp
  "${HOME}/.cache"
  "${HOME}/Projects/ai/ryzen-ai-install"
  "${HOME}/Projects/ai/mlir-aie-capture"
  "${HOME}/Projects/ai/RyzenAI-SW"
  "${HOME}/Projects/ai/RyzenAI"
  "${HOME}/ryzen-ai"
  "${HOME}/Projects/ai/FastFlowLM"
  "${HOME}/Projects/ai/fastflowlm"
  "${HOME}/Projects/ai/Lemonade"
  "${HOME}/Projects/ai/lemonade"
  "${HOME}/.local/share/FastFlowLM"
  "${HOME}/.local/share/lemonade"
  "/opt/xilinx/xrt"
  "/opt/xilinx/xrt/share"
  "/opt/xilinx"
  "/opt/amd/ryzen-ai"
  "/usr/share/xrt"
  "/usr/lib/xrt"
  # In-tree research / lab (listed; not invent load-accepted)
  "$ROOT/testdata/xdna2_cmd/gt_capture"
  "$ROOT/testdata/xdna2_cmd/external_known_good"
  "$ROOT/testdata/xdna2_cmd/silicon_lab"
)

HIT_N=0
HIT_MULTI=0
HIT_RESEARCH_DUP=0
HIT_LARGE=0
HIT_OTHER=0
ROOTS_PRESENT_N=0
LISTED_N=0

emit "================================================================"
emit "host_capture_scan - read-only (W5-2)"
emit "================================================================"
emit "ROOT=$ROOT"
emit "date=$(date -Iseconds 2>/dev/null || date)"
emit "Honesty: invent_pass=0 not_ran_on_npu=1 not_c_ready=1 Pass=No load_accepted_claim=0"
emit "No sudo / no EXEC_CMD / no ALLOW / no invent bytes / no live apply"
emit "Multi-txn ${MULTI_TXN_BYTES} and known research sha16 never claimed load-accepted"
emit ""

emit "=== candidate roots (present / missing) ==="
for r in "${SCAN_ROOTS[@]}"; do
  if [[ -d "$r" ]]; then
    ROOTS_PRESENT_N=$((ROOTS_PRESENT_N + 1))
    emit "  present: $r"
  else
    emit "  missing: $r"
  fi
done

emit ""
emit "=== capture-like files (bounded maxdepth=${MAXDEPTH} list_cap=${LIST_CAP}) ==="

# Collect hits into a temp list (path only), then classify.
TMPDIR_CLAMP="${TMPDIR:-/tmp}"
if [[ "$TMPDIR_CLAMP" == "/" ]]; then
  TMPDIR_CLAMP="/tmp"
fi
HIT_LIST="$(mktemp "${TMPDIR_CLAMP}/dragon_npu_host_capture_scan.XXXXXX")"
HIT_LIST_RAW="${HIT_LIST}.raw"
trap 'rm -f "$HIT_LIST" "$HIT_LIST_RAW"' EXIT
: >"$HIT_LIST_RAW"

for r in "${SCAN_ROOTS[@]}"; do
  [[ -d "$r" ]] || continue
  # Bound find; ignore permission noise.
  while IFS= read -r -d '' f; do
    [[ -f "$f" && -s "$f" ]] || continue
    if ! is_capture_like_basename "$f"; then
      continue
    fi
    printf '%s\0' "$f" >>"$HIT_LIST_RAW" 2>/dev/null || true
  done < <(
    find "$r" -maxdepth "$MAXDEPTH" -type f \( \
        -iname '*ctrltext*' -o -iname 'working_stack*.bin' \
        -o -iname 'instr_vendor_post_patch.bin' -o -iname 'instr_post_patch.bin' \
        -o -iname 'ctrl_post_patch.bin' -o -iname '*insts*.bin' \
        -o -iname '*.pdi' -o -iname '*.xclbin' -o -iname '*.elf' \
        -o -iname '*transaction*.bin' -o -iname '*txn*.bin' \
      \) -print0 2>/dev/null || true
  )
done

# Dedupe paths (sort -zu) then classify.
if [[ -s "$HIT_LIST_RAW" ]]; then
  if ! sort -zu "$HIT_LIST_RAW" -o "$HIT_LIST" 2>/dev/null; then
    cp -- "$HIT_LIST_RAW" "$HIT_LIST"
  fi
else
  : >"$HIT_LIST"
fi

while IFS= read -r -d '' f; do
  [[ -n "$f" && -f "$f" ]] || continue
  HIT_N=$((HIT_N + 1))
  if [[ "$LISTED_N" -ge "$LIST_CAP" ]]; then
    continue
  fi
  line="$(classify_file "$f")"
  [[ -n "$line" ]] || continue
  LISTED_N=$((LISTED_N + 1))
  emit "  $line"
  case "$line" in
    *refuse_class=multi_txn_32208*)
      HIT_MULTI=$((HIT_MULTI + 1))
      ;;
    *refuse_class=research_duplicate_sha16*)
      HIT_RESEARCH_DUP=$((HIT_RESEARCH_DUP + 1))
      ;;
    *refuse_class=large_*)
      HIT_LARGE=$((HIT_LARGE + 1))
      ;;
    *)
      HIT_OTHER=$((HIT_OTHER + 1))
      ;;
  esac
done <"$HIT_LIST"

if [[ "$HIT_N" -eq 0 ]]; then
  emit "  hits=0 (no capture-like files under present roots; soft ok)"
elif [[ "$HIT_N" -gt "$LIST_CAP" ]]; then
  emit "  ... listed first ${LIST_CAP} of ${HIT_N}"
fi

emit ""
emit "=== English summary ==="
emit "  roots present: ${ROOTS_PRESENT_N}  hits: ${HIT_N}  listed: ${LISTED_N}"
emit "  multi_txn_32208: ${HIT_MULTI}  research_duplicate_sha16: ${HIT_RESEARCH_DUP}  large_soft: ${HIT_LARGE}  other: ${HIT_OTHER}"
emit "  Operator next for true load-accepted: place completing-stack capture under"
emit "    testdata/xdna2_cmd/external_known_good/  with load-accepted PROVENANCE fields"
emit "  Collect helper (fail-closed): bash tools/collect_load_accepted_candidate.sh PATH [NAME]"
emit "  Stack inventory: bash tools/host_npu_stack_inventory.sh"

emit ""
emit "=== Footer (honest) ==="
emit "invent_pass=0"
emit "not_ran_on_npu=1"
emit "not_c_ready=1"
emit "Pass=No"
emit "load_accepted_claim=0"
emit "hits_n=${HIT_N}"
emit "multi_txn_n=${HIT_MULTI}"
emit "research_dup_n=${HIT_RESEARCH_DUP}"
emit "roots_present_n=${ROOTS_PRESENT_N}"
emit "entrypoint=tools/host_capture_scan.sh"

if [[ "$WRITE_REPORT" -eq 1 ]]; then
  mkdir -p "$(dirname "$REPORT_MD")"
  {
    echo "# wave5 host capture scan (W5-2)"
    echo
    echo "- **Date:** $(date -Iseconds 2>/dev/null || date)"
    echo "- **Pass:** **No**"
    echo "- **invent_pass:** 0"
    echo "- **not_ran_on_npu:** 1"
    echo "- **not_c_ready:** 1"
    echo "- **load_accepted_claim:** 0"
    echo
    echo "## Counts"
    echo
    echo "| metric | value |"
    echo "|--------|-------|"
    echo "| roots present | ${ROOTS_PRESENT_N} |"
    echo "| hits (capture-like) | ${HIT_N} |"
    echo "| listed (cap ${LIST_CAP}) | ${LISTED_N} |"
    echo "| multi_txn 32208 | ${HIT_MULTI} |"
    echo "| research-duplicate sha16 | ${HIT_RESEARCH_DUP} |"
    echo "| large soft | ${HIT_LARGE} |"
    echo "| other listed | ${HIT_OTHER} |"
    echo
    echo "## Honesty"
    echo
    echo "- invent_pass=0 · Pass=No · not_ran_on_npu=1 · not_c_ready=1"
    echo "- load_accepted_claim=0 always from this scan alone"
    echo "- Multi-txn 32208 and known public mlir-aie research sha16 never claimed load-accepted"
    echo "- Read-only; no sudo, EXEC_CMD, ALLOW, network install, live apply"
    echo
    echo "## Full stdout log"
    echo
    echo '```'
    printf '%s' "$OUT_BUF"
    echo '```'
  } >"$REPORT_MD"
  emit ""
  emit "report_md=${REPORT_MD}"
fi

exit 0
