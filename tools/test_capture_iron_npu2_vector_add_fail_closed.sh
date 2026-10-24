#!/usr/bin/env bash
# Fail-closed smoke: capture_from_host_candidate.sh with
# XDNA2_CAPTURE_JOB=iron_npu2_vector_add on temp 300 B and temp 1688 B.
# Never copies into testdata/xdna2_cmd/external_known_good/.
# Exit 0 when both refuse with honesty tokens; exit 2 on contract miss.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CAP="${ROOT}/tools/capture_from_host_candidate.sh"
if [[ ! -f "$CAP" ]]; then
  echo "ERROR: missing $CAP" >&2
  exit 2
fi

TMP="$(mktemp -d "${TMPDIR:-/tmp}/dragon-npu-iron-capture-XXXXXX")"
cleanup() { rm -rf "$TMP"; }
trap cleanup EXIT

head -c 300 /dev/zero >"$TMP/peel300.bin"
head -c 1688 /dev/zero >"$TMP/classic1688.bin"
# 660 B must not kind-match the IRON vector-add job.
head -c 660 /dev/zero >"$TMP/peel660.bin"

need_tokens() {
  local out="$1"
  local label="$2"
  for tok in \
    "residual_matched_claim=0" \
    "invent_pass=0" \
    "not_ran_on_npu=1" \
    "not_c_ready=1" \
    "classic_1688_claim_refused=1"; do
    if ! grep -qF -- "$tok" <<<"$out"; then
      echo "FAIL ($label): missing honesty token $tok" >&2
      echo "$out" >&2
      exit 2
    fi
  done
}

set +e
OUT300="$(XDNA2_CAPTURE_JOB=iron_npu2_vector_add bash "$CAP" "$TMP/peel300.bin" 2>&1)"
RC300=$?
OUT1688="$(XDNA2_CAPTURE_JOB=iron_npu2_vector_add bash "$CAP" "$TMP/classic1688.bin" 2>&1)"
RC1688=$?
OUT660="$(XDNA2_CAPTURE_JOB=iron_npu2_vector_add bash "$CAP" "$TMP/peel660.bin" 2>&1)"
RC660=$?
set -e

if [[ "$RC300" -ne 2 ]]; then
  echo "FAIL: 300 B iron job must exit 2, got $RC300" >&2
  echo "$OUT300" >&2
  exit 2
fi
if [[ "$RC1688" -ne 2 ]]; then
  echo "FAIL: 1688 B iron job must exit 2, got $RC1688" >&2
  echo "$OUT1688" >&2
  exit 2
fi
if [[ "$RC660" -ne 2 ]]; then
  echo "FAIL: 660 B iron job must exit 2, got $RC660" >&2
  echo "$OUT660" >&2
  exit 2
fi

need_tokens "$OUT300" "300"
need_tokens "$OUT1688" "1688"
need_tokens "$OUT660" "660"

if ! grep -qF "iron_kind_match=1" <<<"$OUT300"; then
  echo "FAIL: 300 B must kind-match the IRON job (iron_kind_match=1)" >&2
  echo "$OUT300" >&2
  exit 2
fi
if grep -qF "iron_kind_match=1" <<<"$OUT1688"; then
  echo "FAIL: 1688 B must not kind-match the IRON job" >&2
  echo "$OUT1688" >&2
  exit 2
fi
if grep -qF "iron_kind_match=1" <<<"$OUT660"; then
  echo "FAIL: 660 B must not kind-match the IRON job (300 B only)" >&2
  echo "$OUT660" >&2
  exit 2
fi
if grep -qE 'residual_matched_claim=1|copied_to_external_known_good|load_accepted=1' <<<"$OUT1688"; then
  echo "FAIL: 1688 under iron job must not become classic residual match" >&2
  echo "$OUT1688" >&2
  exit 2
fi

# Hyphen alias must also refuse (one allow-list).
set +e
OUT_HYPHEN="$(XDNA2_CAPTURE_JOB=iron-npu2-vector-add bash "$CAP" "$TMP/peel300.bin" 2>&1)"
RC_HYPHEN=$?
set -e
if [[ "$RC_HYPHEN" -ne 2 ]]; then
  echo "FAIL: hyphen job name must be accepted as IRON job and exit 2, got $RC_HYPHEN" >&2
  echo "$OUT_HYPHEN" >&2
  exit 2
fi
need_tokens "$OUT_HYPHEN" "hyphen"

echo "OK: iron_npu2_vector_add capture fail-closed (300 kind-match, 660/1688 no match, no copy)"
exit 0
