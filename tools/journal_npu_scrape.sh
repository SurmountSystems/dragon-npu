#!/usr/bin/env bash
# Read-only NPU journal scrape (user session; no sudo; no L3 / no ALLOW).
#
# Soft evidence helper for post-live residual notes. Never invents Pass,
# never opens dyndbg, never claims ran_on_npu.
#
# Usage:
#   bash tools/journal_npu_scrape.sh
#   bash tools/journal_npu_scrape.sh --hours 48
#   bash tools/journal_npu_scrape.sh --since "2026-08-11 12:00:00"
#   bash tools/journal_npu_scrape.sh --lines 80
#   bash tools/journal_npu_scrape.sh --boot          # this boot only (-b)
#   bash tools/journal_npu_scrape.sh --help
#
# Env (optional overrides):
#   XDNA2_JOURNAL_HOURS=N     default window when no --hours / --since (default 24)
#   XDNA2_JOURNAL_LINES=N     max matching lines to print (default 60)
#
# Exit: always 0 soft (including SKIP when journal unreadable / journalctl missing).
# Honesty: invent_pass=0 not_c_ready=1 not_ran_on_npu=1 always.
set -euo pipefail
export PATH="/usr/bin${HOME:+:${HOME}/.local/bin}:${PATH:-}"
unset XDNA2_ALLOW_EXEC_CMD || true

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

HOURS=""
SINCE=""
LINES=""
BOOT_ONLY=0

# Keywords: residual-class hits from live shots + classic APP_LOAD / wait paths.
# Extended: IO_PAGE_FAULT, APP_LOAD, aie2_set_cmd_timeout, Get bo failed,
# Response status, wait_cmd, EINVAL, Failed cmd, amdxdna, CONFIG, 0x3000003.
KEYWORD_RE='IO_PAGE_FAULT|APP_LOAD|aie2_set_cmd_timeout|Get bo failed|Response status|wait_cmd|EINVAL|Failed cmd|Status 0x|0x3000003|0x4000003|amdxdna|CONFIG|aie2_error_backtrack|aie2_hwctx_cfg_debug_bo'

usage() {
  grep -E '^#([^!]|$)' "$0" | sed 's/^# \{0,1\}//'
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h|--help)
      usage
      exit 0
      ;;
    --hours)
      if [[ $# -lt 2 || -z "${2:-}" ]]; then
        echo "ERROR: --hours needs N" >&2
        exit 0
      fi
      HOURS="$2"
      shift 2
      ;;
    --since)
      if [[ $# -lt 2 || -z "${2:-}" ]]; then
        echo "ERROR: --since needs TIMESTAMP" >&2
        exit 0
      fi
      SINCE="$2"
      shift 2
      ;;
    --lines)
      if [[ $# -lt 2 || -z "${2:-}" ]]; then
        echo "ERROR: --lines needs N" >&2
        exit 0
      fi
      LINES="$2"
      shift 2
      ;;
    --boot)
      BOOT_ONLY=1
      shift
      ;;
    -*)
      echo "ERROR: unknown $1 (allowlist: --hours --since --lines --boot --help)" >&2
      # Soft: never fail the evidence pack hard on bad args from agents.
      echo "JOURNAL_SCRAPE_SKIP invent_pass=0 reason=bad_args"
      exit 0
      ;;
    *)
      echo "ERROR: unexpected positional: $1 (use --since TIMESTAMP)" >&2
      echo "JOURNAL_SCRAPE_SKIP invent_pass=0 reason=bad_args"
      exit 0
      ;;
  esac
done

# Defaults
if [[ -z "$HOURS" ]]; then
  HOURS="${XDNA2_JOURNAL_HOURS:-24}"
fi
if [[ -z "$LINES" ]]; then
  LINES="${XDNA2_JOURNAL_LINES:-60}"
fi

# Soft numeric checks (non-digits → default)
if ! [[ "$HOURS" =~ ^[0-9]+$ ]] || [[ "$HOURS" -eq 0 ]]; then
  HOURS=24
fi
if ! [[ "$LINES" =~ ^[0-9]+$ ]] || [[ "$LINES" -eq 0 ]]; then
  LINES=60
fi

echo "=== journal_npu_scrape (read-only; ≠ Pass ≠ ran_on_npu) ==="
echo "ROOT=$ROOT"
echo "NOTE: invent_pass=0 not_c_ready=1 not_ran_on_npu=1 journal_scrape_alone_not_pass=1"
echo "keywords=$KEYWORD_RE"

if ! command -v journalctl >/dev/null 2>&1; then
  echo "JOURNAL_SCRAPE_SKIP invent_pass=0 reason=journalctl_not_available"
  echo "PLAIN: journalctl missing from PATH — cannot scrape kernel journal."
  exit 0
fi

# Build journalctl args: user session, no sudo. Prefer -k (kernel) without root
# when readable; fall back soft if unreadable.
JC_ARGS=(--no-pager -o short-iso)
if [[ "$BOOT_ONLY" -eq 1 ]]; then
  JC_ARGS+=(-b)
  echo "window=this_boot (-b)"
elif [[ -n "$SINCE" ]]; then
  JC_ARGS+=(--since "$SINCE")
  echo "window=since:$SINCE"
else
  JC_ARGS+=(--since "${HOURS} hours ago")
  echo "window=last_${HOURS}_hours"
fi
echo "max_match_lines=$LINES"

# Probe readability first (stderr may say permission denied).
set +e
probe_err="$(journalctl -k -n 1 --no-pager 2>&1 >/dev/null)"
probe_rc=$?
set -e

if [[ "$probe_rc" -ne 0 ]]; then
  # Common: user not in systemd-journal / adm; or empty journal.
  reason="journal_unreadable_or_empty"
  if printf '%s' "$probe_err" | grep -qiE 'Permission denied|No journal|Failed to|Access denied'; then
    reason="journal_permission_denied"
  fi
  echo "JOURNAL_SCRAPE_SKIP invent_pass=0 reason=$reason"
  echo "PLAIN: cannot read kernel journal as this user (no sudo). Soft skip."
  if [[ -n "${probe_err//[[:space:]]/}" ]]; then
    echo "journalctl_stderr_hint=$(printf '%s' "$probe_err" | head -c 200 | tr '\n' ' ')"
  fi
  echo "HINT: operator may re-run under a journal-readable session, or paste"
  echo "  matching lines; pure classify_journal_soft_signals / parse_journal_* "
  echo "  remain available offline from paste text."
  echo "invent_pass=0 not_c_ready=1 not_ran_on_npu=1"
  exit 0
fi

# Scrape kernel journal for residual keywords.
set +e
raw="$(journalctl -k "${JC_ARGS[@]}" 2>/dev/null)"
jc_rc=$?
set -e

if [[ "$jc_rc" -ne 0 || -z "${raw//[[:space:]]/}" ]]; then
  echo "JOURNAL_SCRAPE_SKIP invent_pass=0 reason=journalctl_empty_or_failed jc_rc=$jc_rc"
  echo "invent_pass=0 not_c_ready=1 not_ran_on_npu=1"
  exit 0
fi

set +e
if command -v rg >/dev/null 2>&1; then
  matches_all="$(printf '%s\n' "$raw" | rg -i -- "$KEYWORD_RE" || true)"
else
  matches_all="$(printf '%s\n' "$raw" | grep -iE -- "$KEYWORD_RE" || true)"
fi
set -e

match_total=0
if [[ -n "${matches_all//[[:space:]]/}" ]]; then
  match_total="$(printf '%s\n' "$matches_all" | grep -c . || true)"
fi
# Print only the last N matches (most recent residual noise).
matches="$(printf '%s\n' "$matches_all" | tail -n "$LINES")"
match_n=0
if [[ -n "${matches//[[:space:]]/}" ]]; then
  match_n="$(printf '%s\n' "$matches" | grep -c . || true)"
fi

echo "match_total=$match_total match_printed=$match_n"
if [[ "$match_total" -eq 0 ]]; then
  echo "journal_hint=none (no matching lines in window)"
else
  echo "--- journal matches (read-only; journal_hint= only; last $LINES of $match_total) ---"
  while IFS= read -r line; do
    [[ -n "$line" ]] || continue
    echo "journal_hint=${line}"
  done <<<"$matches"

  # Soft tokens over FULL match set (not only printed tail).
  # Research only; never invent residual class / Pass.
  lower="$(printf '%s\n' "$matches_all" | tr '[:upper:]' '[:lower:]')"
  has_io=0 has_app=0 has_to=0 has_dbg=0 has_einval=0 has_resp=0
  printf '%s' "$lower" | grep -q 'io_page_fault\|io page fault' && has_io=1 || true
  printf '%s' "$lower" | grep -q 'app_load\|0x3000003' && has_app=1 || true
  printf '%s' "$lower" | grep -q 'aie2_set_cmd_timeout' && has_to=1 || true
  printf '%s' "$lower" | grep -q 'get bo failed\|aie2_hwctx_cfg_debug_bo' && has_dbg=1 || true
  printf '%s' "$lower" | grep -q 'einval\|invalid argument' && has_einval=1 || true
  printf '%s' "$lower" | grep -q 'response status\|status 0x' && has_resp=1 || true
  echo "soft_tokens io_page_fault=$has_io app_load_hint=$has_app aie2_set_cmd_timeout=$has_to get_bo_failed=$has_dbg einval_hint=$has_einval response_status=$has_resp"
  echo "NOTE: soft tokens ≠ residual root; axes_alone_not_pass=1 invent_pass=0"
fi

echo "JOURNAL_SCRAPE_OK invent_pass=0 not_c_ready=1 not_ran_on_npu=1 journal_scrape_alone_not_pass=1"
exit 0
