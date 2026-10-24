#!/usr/bin/env bash
# One-shot offline Pass-unblock fingerprint pack (no live, no ALLOW, no invent Pass).
#
# Purpose: dump + fingerprint flexml STX xclbin PDI sections vs the known CVML
# research baseline (283840 B / sha16 a1e65d04add3422c) so a named offline reason
# can justify a future one-shot unlock. Does **not** mint unlock. Does **not**
# open L3. Does **not** set XDNA2_ALLOW_EXEC_CMD.
#
# S4: spent default (flexml unified-2x4x4 / sha16 0eeeb70c…) is **already
# measured** under CONSUMED unlock 2ae69036 — skip re-dump unless --force /
# XDNA2_FP_FORCE_REDUMP=1. Spent sha16 alone is **not** a new unlock reason.
# Ledger footer matches RESIDUAL.md (spent/unspent). invent_pass=0 always.
#
# S5: read-only inventory of *mladf*.xclbin / *gemm*.xclbin under common install
# roots (xclbin-only by design; missing roots OK).
# Report: .agents/reports/pdi-xclbin-inventory.md
#
# Usage:
#   bash tools/pass_unblock_fingerprint_once.sh              # same as --offline
#   bash tools/pass_unblock_fingerprint_once.sh --offline
#   bash tools/pass_unblock_fingerprint_once.sh --offline --force  # re-dump spent default
#   bash tools/pass_unblock_fingerprint_once.sh --print-env  # after offline: export preferred blob
#   bash tools/pass_unblock_fingerprint_once.sh --live       # refuse unless active C_UNLOCK
#   bash tools/pass_unblock_fingerprint_once.sh --inventory-only  # S5 only, no PDI dumps
#
# Outputs:
#   stdout
#   .agents/reports/pass-unblock-fingerprint-once.md  (primary; under workspace)
#   .agents/reports/pdi-xclbin-inventory.md            (S5 paths/sizes only)
#   exclusive mktemp report + exclusive private dump dir under /tmp
#   (never fixed predictable public basenames for PDI dumps)
#
# Exit: 0 offline finished (even if some dumps fail); 2 bad args; 3 --live refused
set -euo pipefail
export PATH="/usr/bin:${HOME}/.local/bin:${PATH}"
unset XDNA2_ALLOW_EXEC_CMD || true

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
BAG_DIR="$ROOT/testdata/xdna2_cmd/external_known_good"
PROV="$BAG_DIR/PROVENANCE.md"
# Never fixed /tmp path — symlink plant would overwrite victim. Set at write time.
REPORT_TMP=""
REPORT_MD="$ROOT/.agents/reports/pass-unblock-fingerprint-once.md"
INVENTORY_MD="$ROOT/.agents/reports/pdi-xclbin-inventory.md"
RESIDUAL="$ROOT/RESIDUAL.md"

# CVML research baseline from live stock CONFIG_CU shots (not product known-good).
BASE_BYTES=283840
BASE_SHA16="a1e65d04add3422c"

# Spent vs unspent ledger (must match RESIDUAL.md § Spent vs unspent research PDI).
# SSoT: RESIDUAL.md — all three flexml STX research PDIs are Spent (2026-08-11).
# Spent default (skip re-dump unless --force): unified-2x4x4 under 2ae69036.
SPENT_DEFAULT_TAG="flexml_unified-2x4x4"
SPENT_DEFAULT_BYTES=27200
SPENT_DEFAULT_SHA16="0eeeb70c84df056e"
SPENT_DEFAULT_UNLOCK="2ae69036"

# Spent flexml llm-2x4x4 under 8352882b (wait_cmd_einval + IO_PAGE_FAULT; not Pass).
SPENT_LLM_TAG="flexml_llm-2x4x4"
SPENT_LLM_BYTES=21664
SPENT_LLM_SHA16="ba0cd738fd870011"
SPENT_LLM_UNLOCK="8352882b"

# Spent flexml unified-4x4 under efd44f35 (wait_cmd_einval + IO_PAGE_FAULT; not Pass).
SPENT_U4_TAG="flexml_unified-4x4"
SPENT_U4_BYTES=13792
SPENT_U4_SHA16="7d172bf223a9b06b"
SPENT_U4_UNLOCK="efd44f35"

# CVML spent (prior unlocks; orphan IO_PAGE_FAULT class).
SPENT_CVML_BYTES=283840
SPENT_CVML_SHA16="a1e65d04add3422c"

# Legacy aliases (pre-spend variable names) — same Spent fingerprints; do not treat as unspent.
UNSPENT_LLM_TAG="$SPENT_LLM_TAG"
UNSPENT_LLM_BYTES="$SPENT_LLM_BYTES"
UNSPENT_LLM_SHA16="$SPENT_LLM_SHA16"
UNSPENT_U4_TAG="$SPENT_U4_TAG"
UNSPENT_U4_BYTES="$SPENT_U4_BYTES"
UNSPENT_U4_SHA16="$SPENT_U4_SHA16"

FLEXML_STX="${HOME}/Projects/ai/ryzen-ai-install/venv/lib/python3.12/site-packages/flexml/flexml_extras/data/ryzen-ai/stx"
FLEXML_UNIFIED_2X4X4="${FLEXML_STX}/unified-2x4x4.xclbin"
FLEXML_LLM_2X4X4="${FLEXML_STX}/llm-2x4x4.xclbin"
FLEXML_UNIFIED_4X4="${FLEXML_STX}/unified-4x4.xclbin"

# Prior CVML research dump candidates (same set as dump_xclbin_pdi before flexml reorder).
CVML_CANDIDATES=(
  "${HOME}/Projects/ai/RyzenAI-SW/Ryzen-AI-CVML-Library/linux/onnx/ryzen14/AMD_AIE2P_4x4_Overlay.xclbin"
  "${HOME}/Projects/ai/RyzenAI-SW/Ryzen-AI-CVML-Library/linux/onnx/ryzen14/AMD_AIE2P_4x4_Overlay_CFG0.xclbin"
  "${HOME}/Projects/ai/RyzenAI-SW/Ryzen-AI-CVML-Library/linux/onnx/ryzen14/AMD_AIE2P_8x4x1_Overlay.xclbin"
)

# S5: read-only search roots (missing OK; never fail the pack).
INVENTORY_ROOTS=(
  "${HOME}/Projects/ai/ryzen-ai-install"
  "${HOME}/Projects/ai/RyzenAI-SW"
)
# Bound walk; xclbin-only by design (txn bins are ctrltext, not PDI).
INVENTORY_MAXDEPTH=12

MODE="offline"
PRINT_ENV=0
FORCE_REDUMP=0
INVENTORY_ONLY=0

# Env override for force re-dump of spent default (research).
if [[ "${XDNA2_FP_FORCE_REDUMP:-}" == "1" || "${XDNA2_FP_FORCE_REDUMP:-}" == "true" ]]; then
  FORCE_REDUMP=1
fi

usage() {
  cat <<'EOF'
pass_unblock_fingerprint_once — offline PDI fingerprint + inventory (no live)

Usage:
  bash tools/pass_unblock_fingerprint_once.sh
  bash tools/pass_unblock_fingerprint_once.sh --offline
  bash tools/pass_unblock_fingerprint_once.sh --offline --force
  bash tools/pass_unblock_fingerprint_once.sh --print-env
  bash tools/pass_unblock_fingerprint_once.sh --inventory-only
  bash tools/pass_unblock_fingerprint_once.sh --live
  bash tools/pass_unblock_fingerprint_once.sh -h|--help

Flags:
  --offline           Default. Dump/fingerprint flexml+CVML PDI; invent_pass=0.
  --force             Re-dump spent default (unified-2x4x4 / 0eeeb70c…). Same as
  --force-redump      XDNA2_FP_FORCE_REDUMP=1.
  --print-env         After offline: print export lines for preferred blob if any.
  --inventory-only    S5 only: read-only *mladf*.xclbin / *gemm*.xclbin search.
  --live              Refuse unless active C_UNLOCK (never opens L3 / EXEC).
  -h, --help          This help.

Honesty:
  invent_pass=0 product_pass=0 ran_on_npu=0 Pass=No
  All three flexml STX research PDIs are Spent (RESIDUAL SSoT); skip re-dump
  unless --force / XDNA2_FP_FORCE_REDUMP=1.
  Spent DIFF alone is not a new unlock reason.
  Eligible unspent material ≠ unlock mint this turn.
  PDI dumps use exclusive private mktemp dir (never fixed /tmp basenames).
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --offline)
      MODE="offline"
      shift
      ;;
    --print-env)
      PRINT_ENV=1
      shift
      ;;
    --force|--force-redump)
      FORCE_REDUMP=1
      shift
      ;;
    --inventory-only)
      MODE="offline"
      INVENTORY_ONLY=1
      shift
      ;;
    --live)
      MODE="live"
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "ERROR: unknown arg: $1" >&2
      echo "allowlist: --offline --print-env --force --force-redump --inventory-only --live --help" >&2
      exit 2
      ;;
  esac
done

# --- live refuse (document only; do not run live without active unlock) ---
if [[ "$MODE" == "live" ]]; then
  echo "=== --live refused in this implement path ==="
  echo "invent_pass=0 not_ran_on_npu=1"
  active=""
  if [[ -f "$RESIDUAL" ]]; then
    # First C_UNLOCK=… line; active only when that line has no CONSUMED.
    active="$(grep -m1 -oE 'C_UNLOCK=[0-9a-fA-F]+' "$RESIDUAL" 2>/dev/null || true)"
    line="$(grep -m1 -E 'C_UNLOCK=[0-9a-fA-F]+' "$RESIDUAL" 2>/dev/null || true)"
    if [[ -n "$line" && "$line" == *CONSUMED* ]]; then
      active=""
    fi
  fi
  if [[ -z "$active" ]]; then
    echo "ERROR: no active (non-CONSUMED) C_UNLOCK in RESIDUAL.md."
    echo "Do not reopen CONSUMED unlocks (2ae69036, 97f38307, 11bc9016, 3a70fabf, b17b0119)."
    echo "Next: run --offline for fingerprint; parent mints unlock only after named reason."
    echo "HUMAN_NEEDED=0 for offline path; live needs new host-minted unlock + operator sandwich."
    exit 3
  fi
  echo "ERROR: --live is documented only. This script does not open L3 or run EXEC."
  echo "Active unlock seen: $active — operator still runs hitl_c / config-cu-run under sandwich."
  echo "Preferred offline prep: bash tools/pass_unblock_fingerprint_once.sh --offline"
  exit 3
fi

mkdir -p "$(dirname "$REPORT_MD")"
mkdir -p "$(dirname "$INVENTORY_MD")"
TMPDIR_CLAMP="${TMPDIR:-/tmp}"
if [[ "$TMPDIR_CLAMP" == "/" ]]; then
  TMPDIR_CLAMP="/tmp"
fi

# Exclusive private dump dir for PDI outs + xclbinutil logs (Issue 2/10/11).
# Created only when this run will dump PDI (not on --inventory-only).
# Never fixed public basenames under /tmp.
DUMP_DIR=""

# Collect lines for reports (stdout tee-style).
OUT_BUF=""
emit() {
  # shellcheck disable=SC2059
  printf '%s\n' "$*"
  OUT_BUF+="$*"$'\n'
}

# True if sha16 is a known spent research fingerprint (not a new unlock reason).
# Matches RESIDUAL.md: all three flexml STX + CVML 4x4 baseline are Spent.
is_spent_sha16() {
  local s="$1"
  case "$s" in
    "$SPENT_DEFAULT_SHA16"|"$SPENT_LLM_SHA16"|"$SPENT_U4_SHA16"|"$SPENT_CVML_SHA16")
      return 0
      ;;
  esac
  return 1
}

# True if tag is the spent default (skip re-dump unless force).
is_spent_default_tag() {
  [[ "$1" == "$SPENT_DEFAULT_TAG" ]]
}

# True if tag is any of the three spent flexml STX research PDIs (RESIDUAL SSoT).
is_spent_flexml_tag() {
  case "$1" in
    "$SPENT_DEFAULT_TAG"|"$SPENT_LLM_TAG"|"$SPENT_U4_TAG")
      return 0
      ;;
  esac
  return 1
}

# Unlock stamp for a spent flexml tag (for ledger notes).
spent_flexml_unlock_for_tag() {
  case "$1" in
    "$SPENT_DEFAULT_TAG") printf '%s' "$SPENT_DEFAULT_UNLOCK" ;;
    "$SPENT_LLM_TAG") printf '%s' "$SPENT_LLM_UNLOCK" ;;
    "$SPENT_U4_TAG") printf '%s' "$SPENT_U4_UNLOCK" ;;
    *) printf '%s' "?" ;;
  esac
}

# Ledger bytes/sha16 for spent flexml tags when skip re-dump.
spent_flexml_ledger_for_tag() {
  # Sets: _sf_bytes _sf_sha16 via nameref-style globals for bash 4.
  case "$1" in
    "$SPENT_DEFAULT_TAG")
      _sf_bytes="$SPENT_DEFAULT_BYTES"
      _sf_sha16="$SPENT_DEFAULT_SHA16"
      ;;
    "$SPENT_LLM_TAG")
      _sf_bytes="$SPENT_LLM_BYTES"
      _sf_sha16="$SPENT_LLM_SHA16"
      ;;
    "$SPENT_U4_TAG")
      _sf_bytes="$SPENT_U4_BYTES"
      _sf_sha16="$SPENT_U4_SHA16"
      ;;
    *)
      _sf_bytes="-"
      _sf_sha16="-"
      ;;
  esac
}

# Flexml research tags (preferred named offline material).
is_flexml_tag() {
  case "$1" in
    flexml_*) return 0 ;;
  esac
  return 1
}

print_spent_ledger() {
  emit "=== Spent vs unspent research PDI ledger (matches RESIDUAL.md) ==="
  emit "Research overlays only. Not product known-good. invent_pass=0."
  emit "Re-live of a spent fingerprint alone is not progress (needs new unlock AND named delta)."
  emit "All three flexml STX research PDIs are SPENT (2026-08-11). No flexml unspent candidate remains."
  emit ""
  emit "tag                              pdi_bytes  sha16             live"
  emit "flexml STX unified-2x4x4         ${SPENT_DEFAULT_BYTES}      ${SPENT_DEFAULT_SHA16}  SPENT under ${SPENT_DEFAULT_UNLOCK} (wait_cmd_einval; not Pass)"
  emit "flexml STX llm-2x4x4             ${SPENT_LLM_BYTES}      ${SPENT_LLM_SHA16}  SPENT under ${SPENT_LLM_UNLOCK} (wait_cmd_einval + IO_PAGE_FAULT; not Pass)"
  emit "flexml STX unified-4x4           ${SPENT_U4_BYTES}      ${SPENT_U4_SHA16}  SPENT under ${SPENT_U4_UNLOCK} (wait_cmd_einval + IO_PAGE_FAULT; not Pass)"
  emit "CVML AIE2P 4x4 Overlay           ${SPENT_CVML_BYTES}     ${SPENT_CVML_SHA16}  SPENT prior unlocks (orphan IO_PAGE_FAULT class)"
  emit ""
  emit "spent_default_tag=${SPENT_DEFAULT_TAG} spent_default_sha16=${SPENT_DEFAULT_SHA16}"
  emit "force_redump=${FORCE_REDUMP} (1 = re-dump spent default via --force or XDNA2_FP_FORCE_REDUMP=1)"
  emit "product_known_good=0 invent_pass=0 not_ran_on_npu=1"
  emit ""
}

# --- S5: read-only inventory of *mladf*.xclbin / *gemm*.xclbin under install roots ---
run_inventory() {
  local found=0
  local root
  local inv_body=""
  local sz
  inv_body+="# PDI / xclbin inventory (read-only; S5)"$'\n'
  inv_body+=$'\n'
  inv_body+="**Date:** $(date -Iseconds 2>/dev/null || date)"$'\n'
  inv_body+=$'\n'
  inv_body+="## Honesty"$'\n'
  inv_body+=$'\n'
  inv_body+="- invent_pass=0"$'\n'
  inv_body+="- not product known-good; empty result is OK"$'\n'
  inv_body+="- paths and sizes only; no proprietary blobs copied into git"$'\n'
  inv_body+="- missing install roots are not errors"$'\n'
  inv_body+="- **xclbin-only by design** (plan A4: \`*mladf*.xclbin\` / \`*gemm*.xclbin\`; txn bins are ctrltext, not PDI)"$'\n'
  inv_body+="- maxdepth=${INVENTORY_MAXDEPTH}"$'\n'
  inv_body+=$'\n'
  inv_body+="## Search roots"$'\n'
  inv_body+=$'\n'
  for root in "${INVENTORY_ROOTS[@]}"; do
    if [[ -d "$root" ]]; then
      inv_body+="- present: \`${root}\`"$'\n'
      emit "inventory_root=present path=${root}"
    else
      inv_body+="- missing (ok): \`${root}\`"$'\n'
      emit "inventory_root=missing path=${root}"
    fi
  done
  inv_body+=$'\n'
  inv_body+="## Matches (\`*mladf*.xclbin\` / \`*gemm*.xclbin\` only)"$'\n'
  inv_body+=$'\n'
  inv_body+="| path | bytes |"$'\n'
  inv_body+="|------|-------|"$'\n'

  emit "=== S5 read-only inventory (*mladf*.xclbin / *gemm*.xclbin; xclbin-only) ==="
  emit "inventory_maxdepth=${INVENTORY_MAXDEPTH} inventory_xclbin_only=1"
  for root in "${INVENTORY_ROOTS[@]}"; do
    [[ -d "$root" ]] || continue
    # Bounded depth; null-safe; ignore permission noise. Paths/sizes only.
    while IFS= read -r -d '' f; do
      sz="$(wc -c <"$f" 2>/dev/null | tr -d ' ' || echo 0)"
      emit "INVENTORY  bytes=${sz}  path=${f}"
      inv_body+="| \`${f}\` | ${sz} |"$'\n'
      found=$((found + 1))
    done < <(
      find "$root" -maxdepth "${INVENTORY_MAXDEPTH}" \
        \( -iname '*mladf*.xclbin' -o -iname '*gemm*.xclbin' \) \
        -type f -print0 2>/dev/null || true
    )
  done

  if [[ "$found" -eq 0 ]]; then
    emit "INVENTORY_COUNT=0 (honest empty; no invent known-good)"
    inv_body+=$'\n'
    inv_body+="**Count:** 0 — no matching xclbins under present roots (honest empty)."$'\n'
  else
    emit "INVENTORY_COUNT=${found}"
    inv_body+=$'\n'
    inv_body+="**Count:** ${found}"$'\n'
  fi
  inv_body+=$'\n'
  inv_body+="## Footer"$'\n'
  inv_body+=$'\n'
  inv_body+="invent_pass=0 product_known_good=0 not_ran_on_npu=1"$'\n'
  inv_body+="Note: S5 is **xclbin-only by design**. DynamicDispatch txn bins are ctrltext, not PDI."$'\n'

  printf '%s' "$inv_body" >"$INVENTORY_MD"
  emit "INVENTORY_MD=${INVENTORY_MD}"
  emit ""
}

emit "================================================================"
emit "pass_unblock_fingerprint_once — OFFLINE only"
emit "================================================================"
emit "Honesty banner:"
emit "  invent_pass=0"
emit "  not product Pass"
emit "  not_ran_on_npu=1"
emit "  do not reopen CONSUMED unlocks: 2ae69036 97f38307 11bc9016 3a70fabf b17b0119"
emit "  dump ≠ Pass ≠ C unlock ≠ matmul known-good PDI"
emit "  CVML baseline for compare: bytes=${BASE_BYTES} sha16=${BASE_SHA16}"
emit "  spent default: ${SPENT_DEFAULT_TAG} ${SPENT_DEFAULT_BYTES}/${SPENT_DEFAULT_SHA16} under ${SPENT_DEFAULT_UNLOCK}"
emit "  force_redump=${FORCE_REDUMP}"
emit "  DUMP_DIR=deferred (created only when dumping PDI; not on --inventory-only)"
emit ""

print_spent_ledger

# S5 inventory always (also --inventory-only).
run_inventory

if [[ "$INVENTORY_ONLY" -eq 1 ]]; then
  emit "=== --inventory-only (skip PDI dumps) ==="
  emit "invent_pass=0 not_ran_on_npu=1 product_known_good=0"
  emit "DUMP_DIR=none (inventory-only; no private dump dir created)"
  emit "INVENTORY_MD=${INVENTORY_MD}"
  REPORT_TMP="$(mktemp "${TMPDIR_CLAMP}/dragon_npu_pass_unblock_fingerprint.XXXXXX")"
  {
    printf '%s' "$OUT_BUF"
  } >"$REPORT_TMP"
  {
    echo "# Pass-unblock fingerprint once (inventory-only)"
    echo
    echo "**Date:** $(date -Iseconds 2>/dev/null || date)"
    echo
    echo "## Honesty"
    echo
    echo "- invent_pass=0 not product Pass not_ran_on_npu=1"
    echo "- PDI dumps skipped (--inventory-only)"
    echo "- DUMP_DIR not created (no PDI dumps this run)"
    echo "- S5 xclbin-only by design"
    echo
    echo "Inventory report: \`${INVENTORY_MD}\`"
  } >"$REPORT_MD"
  emit "REPORT_TMP=$REPORT_TMP"
  emit "REPORT_MD=$REPORT_MD"
  {
    printf '%s' "$OUT_BUF"
    printf 'REPORT_TMP=%s\n' "$REPORT_TMP"
    printf 'REPORT_MD=%s\n' "$REPORT_MD"
  } >"$REPORT_TMP"
  exit 0
fi

# Private dump dir only when this run will dump PDI (after inventory-only exit).
DUMP_DIR="$(mktemp -d "${TMPDIR_CLAMP}/dragon_npu_pdi.XXXXXX")"
chmod 700 "$DUMP_DIR"
emit "DUMP_DIR=${DUMP_DIR} (exclusive private; chmod 700)"
emit ""

# --- capture dir status (never invent bins) ---
emit "=== Capture dir (testdata/xdna2_cmd/external_known_good/) ==="
emit "dir=$BAG_DIR"
if [[ ! -d "$BAG_DIR" ]]; then
  emit "MISSING_DIR"
  BAG_BINS=0
else
  if [[ -f "$PROV" ]]; then
    emit "PROVENANCE=present"
  else
    emit "PROVENANCE=MISSING"
  fi
  BAG_BINS=0
  if [[ -d "$BAG_DIR" ]]; then
    while IFS= read -r f; do
      [[ -s "$f" ]] || continue
      b="$(basename "$f")"
      case "$b" in
        instr_post_patch.bin|instr_vendor_post_patch.bin|ctrl_post_patch.bin|\
        instr_raw_txn.bin|instr_vendor_txn_bin.bin|lab_instr.bin|lab_instr.elf)
          continue
          ;;
      esac
      BAG_BINS=$((BAG_BINS + 1))
      emit "BIN  $b  bytes=$(wc -c <"$f" | tr -d ' ')  sha256=$(sha256sum "$f" | awk '{print $1}')"
    done < <(find "$BAG_DIR" -maxdepth 1 -type f \( -name '*.bin' -o -name '*.ctrltext' \) 2>/dev/null || true)
  fi
  if [[ "$BAG_BINS" -eq 0 ]]; then
    emit "BINS=0  blocked_on_external_bytes=1 (expected offline; not inventing)"
  else
    emit "BINS=$BAG_BINS"
  fi
fi
emit ""

HAS_XCLBINUTIL=0
if command -v xclbinutil >/dev/null 2>&1; then
  HAS_XCLBINUTIL=1
  emit "xclbinutil=$(command -v xclbinutil)"
else
  emit "xclbinutil=MISSING"
fi
emit ""

# Table rows: tag|source|out|bytes|sha16|sha256|vs_baseline|note|spent
TABLE_ROWS=()
# ANY_DUMP_OK only on real RESULT=OK with existing out file (not pure skip).
ANY_DUMP_OK=0
ANY_DIFF=0
ANY_UNSPENT_DIFF=0
ANY_UNSPENT_FLEXML_DIFF=0
PREFERRED_BLOB=""
PREFERRED_TAG=""
PREFERRED_REASON=""
PREFERRED_SPENT=0
SKIPPED_SPENT_REDUMP=0
SPENT_DEFAULT_LEDGER_DRIFT=0

# Prefer unspent flexml DIFF over spent / CVML; never set preferred from skip.
consider_preferred() {
  local tag="$1" out="$2" vs="$3" spent="$4"
  # Refuse non-existent or placeholder outs.
  if [[ -z "$out" || "$out" == "(not dumped)" || ! -f "$out" ]]; then
    return 0
  fi
  if [[ "$vs" != "DIFF" && "$vs" != "SAME" ]]; then
    return 0
  fi
  # Prefer unspent over spent always when choosing preferred blob for future unlock.
  if [[ -z "$PREFERRED_BLOB" ]]; then
    PREFERRED_BLOB="$out"
    PREFERRED_TAG="$tag"
    PREFERRED_SPENT="$spent"
    if [[ "$spent" == "1" ]]; then
      PREFERRED_REASON="first successful dump ($vs) — SPENT (not a new unlock reason alone)"
    else
      PREFERRED_REASON="first successful dump ($vs) unspent candidate"
    fi
    return 0
  fi
  # Upgrade from spent preferred to unspent.
  if [[ "$PREFERRED_SPENT" == "1" && "$spent" == "0" && "$vs" == "DIFF" ]]; then
    PREFERRED_BLOB="$out"
    PREFERRED_TAG="$tag"
    PREFERRED_SPENT=0
    PREFERRED_REASON="unspent DIFF preferred over spent fingerprint"
    return 0
  fi
  # Do not upgrade spent preferred with another spent.
  if [[ "$spent" == "1" ]]; then
    return 0
  fi
  # Prefer flexml unspent DIFF over non-flexml unspent (e.g. CVML 8x4x1).
  if [[ "$vs" == "DIFF" && "$spent" == "0" ]] && is_flexml_tag "$tag" && ! is_flexml_tag "$PREFERRED_TAG"; then
    PREFERRED_BLOB="$out"
    PREFERRED_TAG="$tag"
    PREFERRED_SPENT=0
    PREFERRED_REASON="prefer unspent flexml DIFF over non-flexml unspent"
    return 0
  fi
  # Upgrade to DIFF when current preferred is SAME (unspent path).
  if [[ "$vs" == "DIFF" && "$PREFERRED_REASON" != *DIFF* ]]; then
    PREFERRED_BLOB="$out"
    PREFERRED_TAG="$tag"
    PREFERRED_SPENT=0
    PREFERRED_REASON="DIFF preferred over SAME (unspent)"
    return 0
  fi
  # Historical: among unspent flexml DIFF, prefer llm-2x4x4 over unified-4x4.
  # All three flexml STX are now Spent (RESIDUAL SSoT) — this branch is inert for them.
  if [[ "$vs" == "DIFF" && "$spent" == "0" && "$tag" == "$SPENT_LLM_TAG" && "$PREFERRED_TAG" != "$SPENT_LLM_TAG" ]]; then
    PREFERRED_BLOB="$out"
    PREFERRED_TAG="$tag"
    PREFERRED_SPENT=0
    PREFERRED_REASON="prefer flexml llm-2x4x4 when DIFF (legacy path; tag is Spent under RESIDUAL)"
  fi
}

dump_one() {
  local tag="$1"
  local xclbin="$2"
  # Exclusive path inside private DUMP_DIR (not fixed public /tmp basename).
  local safe_tag
  safe_tag="$(printf '%s' "$tag" | tr -c 'A-Za-z0-9._-' '_' | cut -c1-64)"
  local out="${DUMP_DIR}/${safe_tag}.bin"
  local log
  local vs="DUMP_FAIL"
  local bytes="-"
  local sha16="-"
  local sha256="-"
  local note=""
  local spent=0
  local su=""
  local su_force=""
  local ledger_sha=""

  emit "--- dump tag=$tag ---"
  emit "source=$xclbin"

  # S4: skip re-dump of spent flexml STX PDIs unless --force / env.
  # RESIDUAL SSoT: unified-2x4x4, llm-2x4x4, unified-4x4 all Spent.
  # Pure ledger row: no dump file, no ANY_DUMP_OK, no PREFERRED from skip.
  if is_spent_flexml_tag "$tag" && [[ "$FORCE_REDUMP" -ne 1 ]]; then
    SKIPPED_SPENT_REDUMP=1
    spent_flexml_ledger_for_tag "$tag"
    bytes="$_sf_bytes"
    sha16="$_sf_sha16"
    su="$(spent_flexml_unlock_for_tag "$tag")"
    sha256="(ledger; re-dump skipped — use --force to measure)"
    vs="DIFF"
    spent=1
    note="already_measured_spent under ${su}; skip re-dump (not a new unlock reason)"
    out="(not dumped)"
    # Do NOT set ANY_DUMP_OK — skip is not a real dump success (Issue 1).
    ANY_DIFF=1
    # Do NOT set ANY_UNSPENT_DIFF — spent alone is not a named offline unlock reason.
    # Do NOT consider_preferred — no live path (Issue 3).
    emit "out=(not dumped)"
    emit "source_status=ledger_only"
    emit "RESULT=SKIP_SPENT_REDUMP pdi_bytes=$bytes sha16=$sha16 vs_cvml=$vs"
    emit "NOTE: ${note}"
    emit "NOTE: spent fingerprint already live-measured; do not pretend this is a new unlock reason"
    TABLE_ROWS+=("${tag}|${xclbin}|${out}|${bytes}|${sha16}|${sha256}|${vs}|${note}|${spent}")
    return 0
  fi

  emit "out=$out"

  if [[ ! -f "$xclbin" ]]; then
    note="xclbin_missing"
    emit "RESULT=MISSING_XCLBIN"
    TABLE_ROWS+=("${tag}|${xclbin}|${out}|${bytes}|${sha16}|${sha256}|${vs}|${note}|0")
    return 0
  fi

  if [[ "$HAS_XCLBINUTIL" -ne 1 ]]; then
    note="xclbinutil_missing"
    emit "RESULT=NO_XCLBINUTIL"
    TABLE_ROWS+=("${tag}|${xclbin}|${out}|${bytes}|${sha16}|${sha256}|${vs}|${note}|0")
    return 0
  fi

  # Exclusive log under private dump dir (Issue 11).
  log="$(mktemp "${DUMP_DIR}/xclbinutil.XXXXXX.log")"
  # Refuse leftover same-tag out inside private dir.
  if [[ -e "$out" ]]; then
    rm -f "$out"
  fi

  if ! xclbinutil --force --dump-section "PDI:raw:${out}" --input "$xclbin" >"$log" 2>&1; then
    note="xclbinutil_dump_failed"
    emit "RESULT=DUMP_FAIL"
    emit "xclbinutil_log_tail:"
    tail -n 20 "$log" 2>/dev/null | while IFS= read -r ln; do emit "  $ln"; done || true
    emit "xclbinutil --info (best effort):"
    if xclbinutil --info --input "$xclbin" >"${log}.info" 2>&1; then
      # Keep short: sections + PDI mentions only.
      grep -E 'Section|PDI|UUID|Target|Platform' "${log}.info" 2>/dev/null | head -n 40 \
        | while IFS= read -r ln; do emit "  $ln"; done || true
      note="dump_fail_info_ok"
    else
      emit "  --info also failed"
      note="dump_fail_info_fail"
    fi
    rm -f "$log" "${log}.info" "$out" 2>/dev/null || true
    TABLE_ROWS+=("${tag}|${xclbin}|${out}|${bytes}|${sha16}|${sha256}|${vs}|${note}|0")
    return 0
  fi
  rm -f "$log"

  if [[ ! -f "$out" ]]; then
    note="out_missing_after_dump"
    emit "RESULT=DUMP_FAIL out missing after claimed success"
    TABLE_ROWS+=("${tag}|${xclbin}|${out}|${bytes}|${sha16}|${sha256}|${vs}|${note}|0")
    return 0
  fi

  # Proprietary research bytes: owner-only (Issue 2/10).
  chmod 0600 "$out" 2>/dev/null || true

  bytes="$(wc -c <"$out" | tr -d ' ')"
  if [[ "$bytes" -eq 0 ]]; then
    note="empty_pdi"
    vs="DUMP_FAIL"
    emit "RESULT=DUMP_FAIL empty PDI"
    rm -f "$out"
    TABLE_ROWS+=("${tag}|${xclbin}|${out}|0|-|-|${vs}|${note}|0")
    return 0
  fi

  sha256="$(sha256sum "$out" | awk '{print $1}')"
  sha16="${sha256:0:16}"
  # Real dump success only (Issue 1).
  ANY_DUMP_OK=1

  if is_spent_sha16 "$sha16"; then
    spent=1
  fi
  if is_spent_default_tag "$tag"; then
    spent=1
  fi

  if [[ "$bytes" == "$BASE_BYTES" && "$sha16" == "$BASE_SHA16" ]]; then
    vs="SAME"
    note="matches_cvml_baseline"
    if [[ "$spent" == "1" ]]; then
      note="matches_cvml_baseline; SPENT prior unlocks (not a new unlock reason)"
    fi
  else
    vs="DIFF"
    ANY_DIFF=1
    if [[ "$spent" == "1" ]]; then
      note="differs_from_cvml_baseline; already_measured_spent (not a new unlock reason alone)"
    else
      note="differs_from_cvml_baseline; unspent_candidate"
      ANY_UNSPENT_DIFF=1
      if is_flexml_tag "$tag"; then
        ANY_UNSPENT_FLEXML_DIFF=1
      fi
    fi
  fi

  # Force re-dump of spent flexml: loud honesty even when measured again.
  if is_spent_flexml_tag "$tag" && [[ "$FORCE_REDUMP" -eq 1 ]]; then
    spent=1
    su_force="$(spent_flexml_unlock_for_tag "$tag")"
    spent_flexml_ledger_for_tag "$tag"
    ledger_sha="$_sf_sha16"
    note="force_redump of spent flexml ${ledger_sha}; still already_measured under ${su_force}; not a new unlock reason alone"
    if [[ "$sha16" != "$ledger_sha" ]]; then
      if is_spent_default_tag "$tag"; then
        SPENT_DEFAULT_LEDGER_DRIFT=1
      fi
      note="force_redump sha16=${sha16} DIFFERS from ledger ${ledger_sha}; invent_pass=0 (research signal; not auto-mint unlock)"
      emit "SPENT_LEDGER_DRIFT=1 tag=${tag}"
    fi
  fi

  emit "RESULT=OK pdi_bytes=$bytes sha16=$sha16 vs_cvml=$vs spent=${spent}"
  emit "sha256=$sha256"
  if [[ "$spent" == "1" ]]; then
    emit "NOTE: already measured / spent — do not use as sole new unlock reason"
  fi
  TABLE_ROWS+=("${tag}|${xclbin}|${out}|${bytes}|${sha16}|${sha256}|${vs}|${note}|${spent}")
  consider_preferred "$tag" "$out" "$vs" "$spent"
}

emit "=== Fingerprint dumps (flexml STX + CVML baseline candidates) ==="
if [[ "$FORCE_REDUMP" -eq 1 ]]; then
  emit "force_redump=1 — will re-dump spent default ${SPENT_DEFAULT_TAG}"
else
  emit "force_redump=0 — spent default ${SPENT_DEFAULT_TAG} skip re-dump (ledger sha16=${SPENT_DEFAULT_SHA16})"
fi
emit "DUMP_DIR=${DUMP_DIR}"
emit ""

# Flexml first (pass-unblock target), then CVML for baseline table.
dump_one "flexml_unified-2x4x4" "$FLEXML_UNIFIED_2X4X4"
dump_one "flexml_llm-2x4x4" "$FLEXML_LLM_2X4X4"
dump_one "flexml_unified-4x4" "$FLEXML_UNIFIED_4X4"

i=0
for c in "${CVML_CANDIDATES[@]}"; do
  i=$((i + 1))
  base="$(basename "$c" .xclbin)"
  # short tag (printf avoids newline → trailing underscore from tr)
  tag="cvml_${base}"
  tag="$(printf '%s' "$tag" | tr -c 'A-Za-z0-9._-' '_' | cut -c1-80)"
  dump_one "$tag" "$c"
done

emit ""
emit "=== Fingerprint table ==="
emit "columns: tag | source | out | pdi_bytes | sha16 | sha256 | vs_cvml_baseline | note | spent"
emit "baseline: bytes=${BASE_BYTES} sha16=${BASE_SHA16} (research CVML; not product known-good)"
emit ""
for row in "${TABLE_ROWS[@]+"${TABLE_ROWS[@]}"}"; do
  IFS='|' read -r tag src out bytes sha16 sha256 vs note spent <<<"$row"
  emit "ROW tag=$tag"
  emit "  source=$src"
  emit "  out=$out"
  emit "  pdi_bytes=$bytes sha16=$sha16 vs=$vs spent=${spent:-?} note=$note"
  emit "  sha256=$sha256"
done
if [[ ${#TABLE_ROWS[@]} -eq 0 ]]; then
  emit "(no rows — unexpected)"
fi

# --- verdict ---
# NAMED_OFFLINE_REASON stickiness (Issue 8):
#   - 1 when preferred is unspent flexml DIFF, or force redump ledger drift
#   - ELIGIBLE_UNSPENT_DIFF tracks any unspent DIFF (incl. CVML 8x4x1) without
#     auto-minting unlock; eligible material exists ≠ mint this turn.
NAMED_OFFLINE_REASON=0
NAMED_REASON_TEXT="no preferred unspent flexml DIFF (spent DIFF alone is not a new unlock reason)"
ELIGIBLE_UNSPENT_DIFF=0
if [[ "$ANY_UNSPENT_DIFF" -eq 1 ]]; then
  ELIGIBLE_UNSPENT_DIFF=1
fi

if [[ "$SPENT_DEFAULT_LEDGER_DRIFT" -eq 1 ]]; then
  NAMED_OFFLINE_REASON=1
  NAMED_REASON_TEXT="SPENT_DEFAULT_LEDGER_DRIFT=1 force re-dump sha16 differs from ledger ${SPENT_DEFAULT_SHA16} (research only; invent_pass=0; not auto-mint unlock)"
elif [[ -n "${PREFERRED_TAG:-}" && "$PREFERRED_SPENT" == "0" ]] && is_flexml_tag "$PREFERRED_TAG" && [[ "$ANY_UNSPENT_FLEXML_DIFF" -eq 1 ]]; then
  NAMED_OFFLINE_REASON=1
  NAMED_REASON_TEXT="preferred unspent flexml DIFF tag=${PREFERRED_TAG} vs CVML baseline ${BASE_BYTES}/${BASE_SHA16} (eligible material; unlock NOT minted here)"
elif [[ "$ANY_UNSPENT_DIFF" -eq 1 ]]; then
  NAMED_OFFLINE_REASON=0
  NAMED_REASON_TEXT="ELIGIBLE_UNSPENT_DIFF=1 (non-preferred and/or non-flexml unspent DIFF present) — eligible material exists ≠ unlock mint this turn"
elif [[ "$ANY_DIFF" -eq 1 ]]; then
  NAMED_OFFLINE_REASON=0
  NAMED_REASON_TEXT="DIFF seen only for SPENT fingerprints (e.g. ${SPENT_DEFAULT_SHA16}) — already measured; not a new unlock reason"
fi

HUMAN_NEEDED=0
HUMAN_CMD="none — agent finished offline"
if [[ "$ANY_DUMP_OK" -eq 0 ]]; then
  if [[ "$HAS_XCLBINUTIL" -ne 1 ]]; then
    HUMAN_NEEDED=1
    HUMAN_CMD="Install XRT tools so xclbinutil is on PATH, then: bash tools/pass_unblock_fingerprint_once.sh --offline"
  else
    # All xclbins missing or dump failed with tool present — operator may need paths.
    # Pure skip of spent default without any other real dump also lands here (Issue 1).
    HUMAN_NEEDED=1
    HUMAN_CMD="Confirm flexml STX xclbins under ryzen-ai-install venv (or pass new paths), then: bash tools/pass_unblock_fingerprint_once.sh --offline"
  fi
fi
# Empty capture bag is OK for offline-only (HUMAN_NEEDED stays 0 if dumps worked).

emit ""
emit "=== Verdict ==="
emit "NAMED_OFFLINE_REASON=${NAMED_OFFLINE_REASON}"
emit "  reason: ${NAMED_REASON_TEXT}"
emit "ELIGIBLE_UNSPENT_DIFF=${ELIGIBLE_UNSPENT_DIFF}"
emit "ANY_DIFF=${ANY_DIFF} ANY_UNSPENT_DIFF=${ANY_UNSPENT_DIFF} ANY_UNSPENT_FLEXML_DIFF=${ANY_UNSPENT_FLEXML_DIFF}"
emit "SKIPPED_SPENT_REDUMP=${SKIPPED_SPENT_REDUMP} SPENT_DEFAULT_LEDGER_DRIFT=${SPENT_DEFAULT_LEDGER_DRIFT}"
emit "ANY_DUMP_OK=${ANY_DUMP_OK} (1 only if real dump file written)"
if [[ "$NAMED_OFFLINE_REASON" -eq 1 ]]; then
  emit "  unlock: NOT minted here — residual may note ready-for-unlock-mint; parent decides live"
else
  emit "  unlock: NOT justified by this run alone (spent reconfirm frozen; eligible ≠ mint)"
fi
emit "PREFERRED_BLOB=${PREFERRED_BLOB:-}"
emit "PREFERRED_TAG=${PREFERRED_TAG:-}"
emit "PREFERRED_REASON=${PREFERRED_REASON:-}"
emit "PREFERRED_SPENT=${PREFERRED_SPENT}"
emit "HUMAN_NEEDED=${HUMAN_NEEDED}"
emit "HUMAN_CMD=${HUMAN_CMD}"
emit "DUMP_DIR=${DUMP_DIR}"
emit "invent_pass=0 not_ran_on_npu=1 product_known_good=0"
emit "not_c_ready=1 dump_alone_not_pass=1 unlock_not_minted=1"
emit "capture_bins=${BAG_BINS}"
emit "INVENTORY_MD=${INVENTORY_MD}"
emit ""

# Ledger footer again (plan A5 acceptance: always print spent/unspent matching RESIDUAL).
print_spent_ledger

if [[ -n "${PREFERRED_BLOB:-}" && "$PREFERRED_SPENT" != "1" && -f "${PREFERRED_BLOB}" ]]; then
  emit "# research export (not product known-good; still needs new C_UNLOCK for live):"
  emit "export XDNA2_LAB_PDI_BLOB=${PREFERRED_BLOB}"
  emit "export XDNA2_LAB_CU_FUNC=0"
elif [[ -n "${PREFERRED_BLOB:-}" && "$PREFERRED_SPENT" == "1" ]]; then
  emit "# preferred is SPENT — export only for research reconfirm; not a new unlock reason:"
  emit "# export XDNA2_LAB_PDI_BLOB=${PREFERRED_BLOB}   # spent; needs NEW unlock + named delta"
  emit "# export XDNA2_LAB_CU_FUNC=0"
fi

if [[ "$PRINT_ENV" -eq 1 ]]; then
  emit ""
  emit "=== --print-env ==="
  if [[ -n "${PREFERRED_BLOB:-}" && -f "${PREFERRED_BLOB}" && "$PREFERRED_SPENT" != "1" ]]; then
    emit "export XDNA2_LAB_PDI_BLOB=${PREFERRED_BLOB}"
    emit "export XDNA2_LAB_CU_FUNC=0"
    emit "# fish:"
    emit "set -gx XDNA2_LAB_PDI_BLOB ${PREFERRED_BLOB}"
    emit "set -gx XDNA2_LAB_CU_FUNC 0"
  elif [[ -n "${PREFERRED_BLOB:-}" && "$PREFERRED_SPENT" == "1" ]]; then
    emit "# preferred spent — not exported as unlock justification"
    emit "# force re-dump: bash tools/pass_unblock_fingerprint_once.sh --offline --force"
  else
    emit "# no preferred unspent blob available"
  fi
fi

# Exclusive /tmp plain report (never fixed predictable name; never follow symlink).
# Primary durable report stays under .agents/reports/ (workspace).
if [[ -z "${TMPDIR_CLAMP:-}" ]]; then
  TMPDIR_CLAMP="${TMPDIR:-/tmp}"
  if [[ "$TMPDIR_CLAMP" == "/" ]]; then
    TMPDIR_CLAMP="/tmp"
  fi
fi
REPORT_TMP="$(mktemp "${TMPDIR_CLAMP}/dragon_npu_pass_unblock_fingerprint.XXXXXX")"
{
  printf '%s' "$OUT_BUF"
} >"$REPORT_TMP"

# Markdown report for agents (workspace path; not a /tmp fixed name)
{
  echo "# Pass-unblock fingerprint once (offline)"
  echo
  echo "**Date:** $(date -Iseconds 2>/dev/null || date)"
  echo
  echo "## Honesty"
  echo
  echo "- invent_pass=0"
  echo "- not product Pass; not_ran_on_npu=1"
  echo "- do not reopen CONSUMED unlocks: \`1f761bf4\`, \`efd44f35\`, \`8352882b\`, \`2ae69036\`, \`97f38307\`, \`11bc9016\`, \`3a70fabf\`, \`b17b0119\`"
  echo "- dump ≠ Pass ≠ C unlock ≠ matmul known-good PDI"
  echo "- CVML baseline compare: bytes=\`${BASE_BYTES}\` sha16=\`${BASE_SHA16}\`"
  echo "- spent default: \`${SPENT_DEFAULT_TAG}\` \`${SPENT_DEFAULT_BYTES}\` / \`${SPENT_DEFAULT_SHA16}\` under \`${SPENT_DEFAULT_UNLOCK}\`"
  echo "- all three flexml STX research PDIs **Spent** (RESIDUAL SSoT 2026-08-11)"
  echo "- force_redump=${FORCE_REDUMP}; SKIPPED_SPENT_REDUMP=${SKIPPED_SPENT_REDUMP}; SPENT_DEFAULT_LEDGER_DRIFT=${SPENT_DEFAULT_LEDGER_DRIFT}"
  echo "- ANY_DUMP_OK=${ANY_DUMP_OK} (real dump only; pure skip does not set)"
  echo "- DUMP_DIR=\`${DUMP_DIR}\` (exclusive private; dumps chmod 0600)"
  echo "- eligible unspent material ≠ unlock mint this turn"
  echo
  echo "## Spent vs unspent ledger"
  echo
  echo "| tag | pdi_bytes | sha16 | live |"
  echo "|-----|-----------|-------|------|"
  echo "| flexml STX \`unified-2x4x4\` | ${SPENT_DEFAULT_BYTES} | \`${SPENT_DEFAULT_SHA16}\` | **Spent** under \`${SPENT_DEFAULT_UNLOCK}\` |"
  echo "| flexml STX \`llm-2x4x4\` | ${SPENT_LLM_BYTES} | \`${SPENT_LLM_SHA16}\` | **Spent** under \`${SPENT_LLM_UNLOCK}\` |"
  echo "| flexml STX \`unified-4x4\` | ${SPENT_U4_BYTES} | \`${SPENT_U4_SHA16}\` | **Spent** under \`${SPENT_U4_UNLOCK}\` |"
  echo "| CVML AIE2P 4x4 Overlay | ${SPENT_CVML_BYTES} | \`${SPENT_CVML_SHA16}\` | **Spent** prior |"
  echo
  echo "## Capture dir"
  echo
  echo "- path: \`testdata/xdna2_cmd/external_known_good/\`"
  echo "- bins (non-oracle): **${BAG_BINS}**"
  if [[ "$BAG_BINS" -eq 0 ]]; then
    echo "- blocked_on_external_bytes=1 (expected; not invented)"
  fi
  echo
  echo "## Fingerprint table"
  echo
  echo "| tag | pdi_bytes | sha16 | vs CVML baseline | spent | out | note |"
  echo "|-----|-----------|-------|------------------|-------|-----|------|"
  for row in "${TABLE_ROWS[@]+"${TABLE_ROWS[@]}"}"; do
    IFS='|' read -r tag src out bytes sha16 sha256 vs note spent <<<"$row"
    echo "| \`${tag}\` | ${bytes} | \`${sha16}\` | **${vs}** | ${spent} | \`${out}\` | ${note} |"
  done
  echo
  echo "### Sources"
  echo
  for row in "${TABLE_ROWS[@]+"${TABLE_ROWS[@]}"}"; do
    IFS='|' read -r tag src out bytes sha16 sha256 vs note spent <<<"$row"
    echo "- **${tag}**: \`${src}\`"
    echo "  - sha256=\`${sha256}\`"
  done
  echo
  echo "## Inventory (S5)"
  echo
  echo "See \`.agents/reports/pdi-xclbin-inventory.md\` (paths/sizes only; empty OK; **xclbin-only by design**)."
  echo
  echo "## Verdict"
  echo
  echo "| field | value |"
  echo "|-------|-------|"
  echo "| NAMED_OFFLINE_REASON | **${NAMED_OFFLINE_REASON}** |"
  echo "| reason | ${NAMED_REASON_TEXT} |"
  echo "| ELIGIBLE_UNSPENT_DIFF | ${ELIGIBLE_UNSPENT_DIFF} |"
  echo "| ANY_UNSPENT_DIFF | ${ANY_UNSPENT_DIFF} |"
  echo "| ANY_UNSPENT_FLEXML_DIFF | ${ANY_UNSPENT_FLEXML_DIFF} |"
  echo "| SKIPPED_SPENT_REDUMP | ${SKIPPED_SPENT_REDUMP} |"
  echo "| SPENT_DEFAULT_LEDGER_DRIFT | ${SPENT_DEFAULT_LEDGER_DRIFT} |"
  echo "| ANY_DUMP_OK | ${ANY_DUMP_OK} |"
  echo "| PREFERRED_BLOB | \`${PREFERRED_BLOB:-}\` |"
  echo "| PREFERRED_TAG | \`${PREFERRED_TAG:-}\` |"
  echo "| PREFERRED_REASON | ${PREFERRED_REASON:-} |"
  echo "| PREFERRED_SPENT | ${PREFERRED_SPENT} |"
  echo "| HUMAN_NEEDED | **${HUMAN_NEEDED}** |"
  echo "| HUMAN_CMD | ${HUMAN_CMD} |"
  echo "| DUMP_DIR | \`${DUMP_DIR}\` |"
  echo "| invent_pass | 0 |"
  echo "| not_ran_on_npu | 1 |"
  echo
  if [[ "$NAMED_OFFLINE_REASON" -eq 1 ]]; then
    echo "**Named offline reason (for parent unlock decision, not minted here):** ${NAMED_REASON_TEXT}"
  else
    echo "No preferred unspent flexml DIFF that justifies a new unlock mint this turn. Spent default reconfirm is frozen (use \`--force\` only for research re-measure). ELIGIBLE_UNSPENT_DIFF=${ELIGIBLE_UNSPENT_DIFF} (eligible material exists ≠ mint). invent_pass=0."
  fi
  echo
  echo "## Live next (not run)"
  echo
  echo "\`--live\` is refused unless RESIDUAL has an active non-CONSUMED \`C_UNLOCK=\`. This slice does not open L3 or run EXEC. All three flexml STX research PDIs are **Spent** (do not re-live as progress). After parent mints unlock + operator sandwich: set \`XDNA2_LAB_PDI_BLOB\` to a **new** research blob (not spent flexml fingerprints) under DUMP_DIR and run stock CONFIG_CU once."
  echo
  echo "Plain-text dump (exclusive mktemp, not a fixed /tmp name): \`${REPORT_TMP}\`"
} >"$REPORT_MD"

emit ""
emit "REPORT_TMP=$REPORT_TMP"
emit "REPORT_MD=$REPORT_MD"
emit "invent_pass=0 product_pass=0 ran_on_npu=0 Pass=No"

# Append path lines into the exclusive mktemp file we own (not a fixed name).
{
  printf '%s' "$OUT_BUF"
  printf 'REPORT_TMP=%s\n' "$REPORT_TMP"
  printf 'REPORT_MD=%s\n' "$REPORT_MD"
  printf 'INVENTORY_MD=%s\n' "$INVENTORY_MD"
  printf 'DUMP_DIR=%s\n' "$DUMP_DIR"
} >"$REPORT_TMP"

exit 0
