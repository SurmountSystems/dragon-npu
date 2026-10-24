#!/usr/bin/env bash
# capture_from_host_candidate.sh — W4b T7 checklist (fail-closed)
#
# Optional path from host inventory candidate → external_known_good only when
# evidence proves a true completing stack. Multi-txn / wrong-body historical
# bins MUST refuse residual_matched_claim and MUST NOT invent PROVENANCE.
#
# Honesty: invent_pass=0 · Pass=No · not_ran_on_npu=1 · not_c_ready=1
# Never: ALLOW=1, EXEC_CMD, unlock mint, live apply, invent load-accepted.
#
# Exit:
#   0  dry report only (default) — never claims residual match
#   2  refused: multi-txn / size≠1688 / missing evidence / unsafe path
#   3  usage / missing inventory
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

CANDIDATE="${1:-}"
FORCE_PRINT="${FORCE_PRINT:-0}"
# Named job (default: classic 1688 identity). IRON vector-add is a different job:
#   XDNA2_CAPTURE_JOB=iron_npu2_vector_add
# That job may kind-match a public 300-byte peel only. It must never claim classic 1688.
CAPTURE_JOB="${XDNA2_CAPTURE_JOB:-mladf_classic_1688}"
# Completing path only if operator sets BOTH (never from multi-txn alone):
#   XDNA2_CAPTURE_COMPLETING_STACK_EVIDENCE=1
#   XDNA2_CAPTURE_ALLOW_RESIDUAL_MATCHED_CLAIM=1  (still never set residual_matched_claim in PROVENANCE invent)
# Without both: always refuse residual_matched_claim=1 and refuse copy as load-accepted.
# XDNA2_CAPTURE_JOB=iron_npu2_vector_add still refuses 1688 residual_matched_claim.

usage() {
  cat <<'EOF'
Usage: bash tools/capture_from_host_candidate.sh [CANDIDATE_PATH]

Fail-closed checklist for host inventory candidates.
Default: print score axes + refuse residual_matched_claim (exit 2 for multi-txn).

Job identity (env XDNA2_CAPTURE_JOB):
  mladf_classic_1688      default frozen lab identity
  iron_npu2_vector_add    IRON vector-add collect (not 1688 load-accepted)

Does NOT:
  - invent PROVENANCE claiming completed-on-FW
  - open ALLOW / EXEC_CMD / unlock
  - mark residual_matched / load-accepted from multi-txn alone
  - retag a public 300-byte peel as classic 1688 load-accepted

Related:
  bash tools/host_capture_scan.sh
  bash tools/collect_load_accepted_candidate.sh PATH [NAME]
  testdata/xdna2_cmd/external_known_good/PROVENANCE.md (load-accepted fields)

See: .agents/reports/wave4-discovery.md
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

if [[ -z "$CANDIDATE" ]]; then
  # Default: inventory-named multi-txn historical candidate
  CANDIDATE="$ROOT/testdata/xdna2_cmd/gt_capture/instr_vendor_post_patch.bin"
fi

echo "================================================================"
echo "capture_from_host_candidate - fail-closed (W4b T7)"
echo "================================================================"
echo "ROOT=$ROOT"
echo "candidate=$CANDIDATE"
echo "job=$CAPTURE_JOB"
echo "Honesty: invent_pass=0 not_ran_on_npu=1 not_c_ready=1 Pass=No residual_matched_claim=0"
echo "No ALLOW / no EXEC_CMD / no unlock mint / no invent load-accepted"

if [[ ! -f "$CANDIDATE" ]]; then
  echo "REFUSE: candidate file missing"
  echo "residual_matched_claim=0 invent_pass=0"
  exit 3
fi

bytes=$(wc -c <"$CANDIDATE" | tr -d ' ')
echo "blob_len=$bytes"

# Historical wrong-body multi-txn (HITL false CLASS_HINT C era): 32208 B.
# Lab classic residual identity is 1688 B. Public AOT research sizes differ.
multi_txn_refused=0
if [[ "$bytes" -eq 32208 ]]; then
  multi_txn_refused=1
  echo "class=historical_multi_txn_wrong_body"
  echo "reason=size_32208_is_multi_class_not_lab_classic_1688"
fi

if [[ "$bytes" -ne 1688 ]]; then
  echo "distance_hint_to_1688=$(( bytes > 1688 ? bytes - 1688 : 1688 - bytes ))"
  echo "note=not_lab_classic_1688_size"
fi

# Completing-stack evidence gate (operator explicit only).
# Classic 1688 only. IRON job never uses this as a 1688 residual match.
evidence_ok=0
# One allow-list with classify/refuse: underscore or hyphen, case-insensitive.
capture_job_norm="$(printf '%s' "$CAPTURE_JOB" | tr '[:upper:]-' '[:lower:]_')"
iron_job=0
if [[ "$capture_job_norm" == "iron_npu2_vector_add" ]]; then
  iron_job=1
fi

if [[ "$iron_job" -eq 0 \
   && "${XDNA2_CAPTURE_COMPLETING_STACK_EVIDENCE:-0}" == "1" \
   && "${XDNA2_CAPTURE_ALLOW_RESIDUAL_MATCHED_CLAIM:-0}" == "1" \
   && "$multi_txn_refused" -eq 0 \
   && "$bytes" -eq 1688 ]]; then
  evidence_ok=1
fi

iron_kind_match=0
classic_1688_claim_refused=0
if [[ "$iron_job" -eq 1 ]]; then
  classic_1688_claim_refused=1
  if [[ "$bytes" -eq 300 ]]; then
    iron_kind_match=1
    echo "size_class=public-npu2-small"
    echo "note=iron_npu2_vector_add_kind_match_not_1688_load_accepted"
  fi
fi

echo "--- decision ---"
echo "job=$CAPTURE_JOB"
echo "iron_job=$iron_job"
echo "iron_kind_match=$iron_kind_match"
echo "classic_1688_claim_refused=$classic_1688_claim_refused"
echo "multi_txn_refused=$multi_txn_refused"
echo "completing_stack_evidence_ok=$evidence_ok"
echo "residual_matched_claim=0"
echo "load_accepted=0"
echo "product_known_good=0"
echo "invent_pass=0"
echo "Pass=No"
echo "not_ran_on_npu=1"
echo "not_c_ready=1"

if [[ "$iron_job" -eq 1 ]]; then
  echo "REFUSE: job=iron_npu2_vector_add must never set residual_matched_claim against classic 1688."
  echo "Public 300 B peel may be the right kind for this IRON job after a completed run."
  echo "It is not 1688 load-accepted. invent_pass=0 product_pass=0 ran_on_npu=0"
  exit 2
fi

if [[ "$multi_txn_refused" -eq 1 ]]; then
  echo "REFUSE: multi-txn candidate alone must not be ingested as residual_matched / load-accepted."
  echo "Operator material ask still open: place a true load-accepted working-stack capture under"
  echo "  testdata/xdna2_cmd/external_known_good/"
  echo "Public mlir-aie AOT under that dir remains RESEARCH ONLY (not residual identity)."
  exit 2
fi

if [[ "$evidence_ok" -eq 0 ]]; then
  echo "REFUSE: no safe completing capture path without operator evidence."
  echo "Set XDNA2_CAPTURE_COMPLETING_STACK_EVIDENCE=1 and"
  echo "XDNA2_CAPTURE_ALLOW_RESIDUAL_MATCHED_CLAIM=1 only when live ERT Completed +"
  echo "OFM allclose evidence exists (decide_ran_on_npu). Multi-txn still refused above."
  echo "blocked_on_host_stack=0 (host has tools; missing is residual-matched material)"
  exit 2
fi

# Even with evidence flags, this script does not invent PROVENANCE or copy as
# load-accepted without a human-reviewed path. Print only.
echo "NOTE: evidence flags present but auto-ingest is not implemented (fail-closed)."
echo "Manual: operator places bytes + honest PROVENANCE under external_known_good/."
echo "residual_matched_claim=0 invent_pass=0 Pass=No"
exit 2
