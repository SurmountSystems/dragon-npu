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
# Optional research envs (forwarded to the example; ≠ C-ready / ≠ Pass):
#   XDNA2_LAB_JOURNAL_TEXT=…   inline journal **paste text** (not a path); dual FW
#                              codes only when hex literals parse; 256 KiB cap
#   XDNA2_LAB_PRINT_C2_CMDBUF_RESEARCH=1|true
#                              soft research lines (Tracks F/G/K/L/M/N/O + later
#                              offline bags; SSoT = example --help / construction map):
#                                c2_cmdbuf_map= / c2_cmdlist_msg=
#                                c2_cmdlist_fill_cross_check=
#                                instr_pdi_load_research= (full staged ELF or SKIP)
#                                app_load_pdi_partition_research= (public status-name;
#                                  full C2 pair needs XDNA2_LAB_JOURNAL_TEXT)
#                                public_pdi_evidence_scan= (Track M closed lock)
#                                external_public_pdi_evidence_scan= (Track N closed_no_new)
#                                app_load_maintain_reconfirm= (Track O A–N still closed;
#                                  compose M+N; no FS/network/XRT walk;
#                                  ≠ C-ready ≠ silicon progress / not new evidence)
#                                app_load_offline_progress_scorecard= (stamp 9cdfe8d0;
#                                  class + bag + e2e + compare status; alone ≠ close)
#                                app_load_bag_feed_dry_run_scorecard= (stamp 2b01425e;
#                                  bag-drop + lab feed + dry-run auto-feed composition;
#                                  bag absent after feed → blocked_on_external_bytes=1;
#                                  match ≠ Pass; scorecard alone ≠ residual closed)
#                                app_load_extract_bag_scorecard_pipeline= (stamp 588c1c72;
#                                  extract markers + bag-drop + composition scorecard;
#                                  markers without ctrltext still blocked_on_external_bytes=1;
#                                  extract alone ≠ close; pipeline alone ≠ residual closed)
#                                external_known_good_operator_extract_checklist=
#                                  (extract-to-bag steps + refuse tokens; no invent)
#                                lab_export_fingerprint_feed= (stamp d2e287ab;
#                                  auto-feed lab fingerprint from export ELF path +
#                                  last_apply signals; ready when ELF has unpatched sha;
#                                  signals alone incomplete; feed alone ≠ residual closed)
#                                external_ctrltext_compare_dry_run= (stamp d2e287ab auto-feed;
#                                  bag absent → blocked_on_external_bytes=1; lab_export_ready
#                                  from feed; refuse post_patch/VE2; match ≠ Pass;
#                                  dry-run alone ≠ close)
#                              always invent_pass=0; must be exact 1/true (0/false/empty=off)
#   XDNA2_LAB_XCLBIN_SECTION_TEXT=…  research paste of public xclbin section **names** only
#                                  (cap 64 KiB); feeds extract half of
#                                  app_load_extract_bag_scorecard_pipeline= under PRINT;
#                                  never product known-good / never PDI decode; markers
#                                  without bag bytes still blocked_on_external_bytes=1
#
# Soft default (for silicon_contract_check):
#   - missing fixture → CLASSIFY_SKIP + exit 0
#   - cargo/example fail → WARN + exit 0
#   - successful report → CLASSIFY_SOFT_OK + exit 0
# Never claims C-ready / ran_on_npu. CONTRACT_OK ≠ C-ready.
# Never sets XDNA2_ALLOW_EXEC_CMD (unset at entry for hygiene).
#
# Optional:
#   --require   exit 2 if no last_apply or example fails (hard offline research)
set -euo pipefail
export PATH="/usr/bin:${HOME}/.local/bin:${PATH}"
# Soft classify is offline-only — never inherit a dangerous EXEC allow from the
# ambient shell (matches c3 / run_offline_gates hygiene).
unset XDNA2_ALLOW_EXEC_CMD || true
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# Match Rust std::env::temp_dir() on Unix (TMPDIR or /tmp).
# TMPDIR=/ (or empty after strip) must not become "" and match every absolute path.
# Resolve symlink-to-/ via realpath when possible; fail-closed if realpath fails or
# path remains a symlink (never keep unresolved rootlink as allow-root).
# Soft smoke: bash tools/smoke_tmpdir_root_clamp.sh (also soft in run_offline_gates).
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

REQUIRE=0
IN=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --require)
      REQUIRE=1
      shift
      ;;
    -h|--help)
      # Include PRINT research token list (scorecard / extract / dry-run past line 30).
      sed -n '1,60p' "$0" | grep -E '^#([^!]|$)' | sed 's/^# \{0,1\}//'
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
