#!/usr/bin/env bash
# Post-C2 / post-CONFIG_CU evidence pack (offline; no L3 / no ALLOW).
#
# Maximizes one paste for the agent after a live sandwich:
#   1) L3 allow state
#   2) last_apply path resolution + head tokens
#   3) journal scrape via tools/journal_npu_scrape.sh (IO_PAGE_FAULT / APP_LOAD / timeout / Get bo)
#   4) p5 interpret (classify + C3 + C4 + P6)
#
# Usage:
#   bash tools/collect_post_c2_evidence.sh
#   bash tools/collect_post_c2_evidence.sh /tmp/dragon_npu_last_apply_config_cu_123.json
#   env XDNA2_LAB_LAST_APPLY_JSON=/path/to.json bash tools/collect_post_c2_evidence.sh
#
# Exit: 0 soft complete; 2 path refuse / bad args.
# Honesty: invent_pass=0 not_c_ready=1 not_ran_on_npu=1 always.
set -euo pipefail
export PATH="/usr/bin${HOME:+:${HOME}/.local/bin}:${PATH:-}"
unset XDNA2_ALLOW_EXEC_CMD || true

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

IN=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    -h|--help)
      grep -E '^#([^!]|$)' "$0" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    -*)
      echo "ERROR: unknown $1 (allowlist: --help; optional last_apply path)" >&2
      exit 2
      ;;
    *)
      if [[ -n "$IN" ]]; then
        echo "ERROR: only one path argument allowed" >&2
        exit 2
      fi
      IN="$1"
      shift
      ;;
  esac
done

if [[ -z "$IN" && -n "${XDNA2_LAB_LAST_APPLY_JSON:-}" ]]; then
  IN="$XDNA2_LAB_LAST_APPLY_JSON"
fi

echo "=== collect_post_c2_evidence (offline; ≠ Pass ≠ ran_on_npu) ==="
echo "ROOT=$ROOT"
echo "NOTE: invent_pass=0 not_c_ready=1 not_ran_on_npu=1 product_pass_requires_live_decide=1"

ALLOW_PATH="/sys/module/amdxdna/parameters/exec_cmd_allow"
if [[ -r "$ALLOW_PATH" ]]; then
  allow="$(tr -d '[:space:]' <"$ALLOW_PATH" || true)"
  echo "exec_cmd_allow=$allow"
  case "$allow" in
    0|N|n) echo "L3_OK closed" ;;
    1|Y|y)
      echo "CRITICAL: L3 still open — restore before anything else:" >&2
      echo "  sudo sh -c 'echo 0 > /sys/module/amdxdna/parameters/exec_cmd_allow'" >&2
      ;;
    *) echo "WARN: unexpected exec_cmd_allow=$allow" >&2 ;;
  esac
else
  echo "exec_cmd_allow=missing (module param not readable)"
fi

# Resolve last_apply for display
RESOLVED=""
BASELINE_FIXTURE_ONLY=0
if [[ -n "$IN" ]]; then
  if [[ ! -f "$IN" ]]; then
    echo "ERROR: last_apply path not a file: $IN" >&2
    echo "PLAIN: Explicit pin missing. This matches an outer lab timeout kill" >&2
    echo "  (timeout(1) rc=124 → lab exit 4) when the process died before a" >&2
    echo "  progressive or final last_apply stamp. Incomplete evidence — not" >&2
    echo "  APP_LOAD classify, not product Pass, not ran_on_npu." >&2
    echo "EVIDENCE_PATH_REFUSE invent_pass=0 not_c_ready=1 not_ran_on_npu=1 incomplete_timeout_or_missing_pin=1" >&2
    exit 2
  fi
  RESOLVED="$IN"
elif [[ -f "$ROOT/testdata/xdna2_cmd/last_apply.json" ]]; then
  RESOLVED="$ROOT/testdata/xdna2_cmd/last_apply.json"
  BASELINE_FIXTURE_ONLY=1
  echo "============================================================"
  echo "LOUD: BASELINE FIXTURE ONLY — not this live shot's last_apply"
  echo "  path: $RESOLVED"
  echo "  This is the locked 2026-07-18 APP_LOAD fixture under testdata."
  echo "  Do NOT treat it as evidence for a timeout/CONFIG_CU shot."
  echo "  Prefer the explicit pin path from hitl_config_cu_once / LAST_APPLY."
  echo "  invent_pass=0 not_c_ready=1 not_ran_on_npu=1"
  echo "============================================================"
else
  # newest temp
  TMP_ROOT="${TMPDIR:-/tmp}"
  TMP_ROOT="${TMP_ROOT%/}"
  if [[ -z "$TMP_ROOT" || "$TMP_ROOT" == "/" ]]; then TMP_ROOT="/tmp"; fi
  # shellcheck disable=SC2012
  RESOLVED="$(ls -t "$TMP_ROOT"/dragon_npu_last_apply_*.json 2>/dev/null | head -n1 || true)"
fi

echo "last_apply=${RESOLVED:-(none)}"
if [[ "$BASELINE_FIXTURE_ONLY" -eq 1 ]]; then
  echo "last_apply_source=baseline_fixture_only"
fi
if [[ -n "$RESOLVED" && -f "$RESOLVED" ]]; then
  echo "last_apply_bytes=$(wc -c <"$RESOLVED" | tr -d ' ')"
  # Soft token peek (python-free)
  if command -v rg >/dev/null 2>&1; then
    echo "--- last_apply token peek ---"
    rg -o 'config_cu[=_][A-Za-z0-9]+|config_cu_failed|pdi_bytes=[0-9]+|ert_state=[0-9]+|aiebu_elf[=:][A-Za-z0-9]+|rela_count[=:][0-9]+|ran_on_npu[=:][A-Za-z0-9]+|lab_partial_[A-Za-z0-9_]+|"status"[[:space:]]*:[[:space:]]*"[^"]+"' \
      "$RESOLVED" 2>/dev/null | head -40 || true
  fi
elif [[ -z "$RESOLVED" ]]; then
  echo "PLAIN: No last_apply found (no explicit pin, no temp pin, no baseline)."
  echo "  Incomplete evidence — typical after outer timeout kill before stamp."
  echo "  invent_pass=0 not_c_ready=1 not_ran_on_npu=1"
fi

echo "--- journal (kernel; best-effort via journal_npu_scrape) ---"
# Soft read-only scrape: last 24h default, residual keywords, invent_pass=0.
# Standalone: bash tools/journal_npu_scrape.sh [--hours N|--since TS|--boot]
if [[ -f "$ROOT/tools/journal_npu_scrape.sh" ]]; then
  set +e
  # Prefer this-boot when collecting post-shot evidence (matches prior -b window).
  bash "$ROOT/tools/journal_npu_scrape.sh" --boot --lines 40
  set -e
else
  echo "JOURNAL_SCRAPE_SKIP invent_pass=0 reason=helper_missing"
  if command -v journalctl >/dev/null 2>&1; then
    set +e
    journalctl -k -b --no-pager 2>/dev/null \
      | rg -i 'IO_PAGE_FAULT|APP_LOAD|aie2_set_cmd_timeout|Get bo failed|Failed cmd|Status 0x|0x3000003|0x4000003|amdxdna|CONFIG|EINVAL' \
      | tail -n 40 || true
    set -e
  else
    echo "(journalctl not available)"
  fi
fi

echo "--- p5 interpret ---"
if [[ -n "$RESOLVED" ]]; then
  bash "$ROOT/tools/p5_post_c2_interpret.sh" "$RESOLVED" || true
else
  bash "$ROOT/tools/p5_post_c2_interpret.sh" || true
fi

echo "--- dual residual reminder ---"
echo "userspace ELF+rela OK ≠ FW accept (0x3000003 APP_LOAD_PDI_FAIL)."
echo "config_cu=1 with still APP_LOAD → PDI/association primary suspect (progress)."
echo "config_cu_failed / absent → attach/env miss (not yet measured CONFIG_CU)."
echo "always: invent_pass=0 not_c_ready=1 not_ran_on_npu=1"
echo "EVIDENCE_PACK_OK invent_pass=0 not_ran_on_npu=1"
exit 0
