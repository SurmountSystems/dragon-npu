#!/usr/bin/env bash
# C3 post-C2 offline pipeline — soft classify + new-evidence detection.
#
# IMPL f76067f8: C3 readiness. Without a **new** C2 last_apply body, soft-skip
# with C3_SKIP — do **not** invent APP_LOAD research by re-parsing the closed
# baseline fixture matrix.
#
# What this does (offline only; NO L3 / NO EXEC / never sets ALLOW):
#   1) Resolve last_apply under load confinement (temp_dir ∪ testdata/xdna2_cmd)
#      order: explicit argv → env → repo baseline (if present) → newest temp
#      only when no baseline. Explicit/env never overridden by temp discovery.
#   2) Detect new evidence by **fingerprint equality** vs locked baseline file
#      (status|aiebu|ert|ran|detail_len|rela|config_cu). Path alone ≠ evidence.
#      config_cu bit: status config_cu_failed / detail config_cu= / attach-failed
#      phrase → so post-P4 CONFIG_CU sandwich is not false C3_SKIP.
#      same fingerprint → C3_SKIP same_body_as_baseline; FP differs → C3_EVIDENCE=1
#      even if residual class is still locked APP_LOAD (annotate
#      shape_still_locked_app_load=1). Never force skip when FP differs.
#   3) Soft classify via tools/classify_last_apply_residual.sh (reuse)
#   4) Optional best-effort journalctl lines → journal_hint= only (never invent
#      FW residual class without detail field)
#
# Always prints: not_c_ready=1 not_ran_on_npu=1
# Never unlocks C. Never opens L3. Unsets XDNA2_ALLOW_EXEC_CMD at entry.
#
# Usage:
#   bash tools/c3_post_c2_classify.sh
#   bash tools/c3_post_c2_classify.sh /path/to/last_apply.json
#   bash tools/c3_post_c2_classify.sh --require-new   # exit 2 if no new evidence
#   bash tools/c3_post_c2_classify.sh --skip-journal
#   bash tools/c3_post_c2_classify.sh --no-classify   # evidence detect only
#   # bash (portable env prefix):
#   env XDNA2_LAB_LAST_APPLY_JSON=/path/to.json bash tools/c3_post_c2_classify.sh
#   # fish:
#   set -gx XDNA2_LAB_LAST_APPLY_JSON /path/to.json; bash tools/c3_post_c2_classify.sh
#
# Manual smoke (same body different path → skip):
#   cp testdata/xdna2_cmd/last_apply.json /tmp/dragon_npu_last_apply_copy_test.json
#   bash tools/c3_post_c2_classify.sh /tmp/dragon_npu_last_apply_copy_test.json
#   # expect C3_SKIP reason=same_body_as_baseline
#   rm -f /tmp/dragon_npu_last_apply_copy_test.json
#
# Exit:
#   0  C3_SKIP (no new evidence) or C3_EVIDENCE soft path complete
#   2  --require-new and no new evidence; bad args; path refuse; realpath fail
#
# Honesty: C3 without new last_apply ≠ silicon progress. CLASSIFY_SOFT_OK ≠ C-ready.
set -euo pipefail
export PATH="/usr/bin${HOME:+:${HOME}/.local/bin}:${PATH:-}"
# Never inherit sticky userspace EXEC gate.
unset XDNA2_ALLOW_EXEC_CMD || true

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# TMPDIR=/ (or empty after strip) must not become "" and match every absolute path.
# Resolve symlink-to-/ via realpath when possible; fail-closed if realpath fails or
# path remains a symlink (never keep unresolved rootlink as allow-root).
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

# Match Rust MAX_MLADF_LAB_FILE_BYTES for last_apply reads.
MAX_LAST_APPLY_BYTES=$((64 * 1024 * 1024))

BASELINE="$ROOT/testdata/xdna2_cmd/last_apply.json"
REQUIRE_NEW=0
SKIP_JOURNAL=0
NO_CLASSIFY=0
IN=""
# Set when argv or env selected the path — never replaced by temp auto-discovery.
PATH_PINNED=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --require-new)
      REQUIRE_NEW=1
      shift
      ;;
    --skip-journal)
      SKIP_JOURNAL=1
      shift
      ;;
    --no-classify)
      NO_CLASSIFY=1
      shift
      ;;
    -h|--help)
      sed -n '1,55p' "$0" | grep -E '^#([^!]|$)' | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    -*)
      echo "ERROR: unknown $1 (allowlist: --require-new --skip-journal --no-classify --help)" >&2
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

# Fail-closed absolute path (must exist; no fallback to raw string).
abs_path() {
  local p="$1" abs
  if [[ "$p" = /* ]]; then
    abs="$(realpath -e "$p" 2>/dev/null)" || return 1
  else
    abs="$(realpath -e "$ROOT/$p" 2>/dev/null)" || return 1
  fi
  # Refuse if realpath still reports a symlink (defense-in-depth).
  if [[ -L "$abs" ]]; then
    return 1
  fi
  printf '%s\n' "$abs"
}

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
  # Symlink refuse (defense-in-depth; python also O_NOFOLLOW).
  if [[ -L "$p" ]]; then
    return 1
  fi
  local abs
  if [[ ! -e "$p" && "$p" != /* ]]; then
    if [[ ! -e "$ROOT/$p" ]]; then
      return 1
    fi
  fi
  if ! abs="$(abs_path "$p")"; then
    return 1
  fi
  case "$abs" in
    "$TMP_ROOT"/*|"$ROOT/testdata/xdna2_cmd"/*) return 0 ;;
    *) return 1 ;;
  esac
}

# Secure JSON helpers: O_NOFOLLOW open + size cap (never follow final symlink).
# Prints fingerprint: status|aiebu|ert|ran|detail_len|rela|config_cu
# Exit 0 ok; 1 parse/shape false; 2 I/O or refuse.
_python_read_json() {
  local mode="$1" f="$2"
  MAX_BYTES="$MAX_LAST_APPLY_BYTES" python3 - "$mode" "$f" <<'PY'
import json, os, sys

mode = sys.argv[1]
path = sys.argv[2]
max_bytes = int(os.environ.get("MAX_BYTES", str(64 * 1024 * 1024)))

def die(msg, code=2):
    print(msg, file=sys.stderr)
    sys.exit(code)

flags = os.O_RDONLY
# Unix: refuse final-component symlink (TOCTOU vs bare open).
if hasattr(os, "O_NOFOLLOW"):
    flags |= os.O_NOFOLLOW

try:
    st = os.lstat(path)
except OSError as e:
    die(f"lstat failed: {e}", 2)
if os.path.islink(path):
    die("refused: path is symlink", 2)
if not os.path.isfile(path):
    die("refused: not a regular file", 2)
if st.st_size > max_bytes:
    die(f"refused: file {st.st_size} bytes > max {max_bytes}", 2)

try:
    fd = os.open(path, flags)
except OSError as e:
    die(f"open failed (O_NOFOLLOW): {e}", 2)

try:
    data = os.read(fd, max_bytes + 1)
finally:
    os.close(fd)

if len(data) > max_bytes:
    die(f"refused: read size {len(data)} > max {max_bytes}", 2)

try:
    d = json.loads(data.decode("utf-8"))
except (UnicodeDecodeError, json.JSONDecodeError) as e:
    die(f"json parse failed: {e}", 2)
if not isinstance(d, dict):
    die("json root must be object", 2)

status = d.get("status")
if status is None:
    status_s = ""
else:
    status_s = str(status).strip()
aiebu = d.get("aiebu_elf")
ert = d.get("ert_error")
ran = d.get("ran_on_npu")
detail = d.get("detail") or ""
if not isinstance(detail, str):
    detail = str(detail)
rela = d.get("rela_count")

def b(v):
    if v is True:
        return "1"
    if v is False:
        return "0"
    return "?"

if mode == "fingerprint":
    rela_s = "" if rela is None else str(rela)
    # P5: config_cu bit — post-P4 sandwich must not false C3_SKIP vs baseline.
    # Align with Rust last_apply_has_config_cu_signal (whole-token config_cu_failed).
    def whole_token(d, tok):
        i = 0
        while True:
            j = d.find(tok, i)
            if j < 0:
                return False
            before_ok = j == 0 or (not d[j - 1].isalnum() and d[j - 1] != "_")
            after = j + len(tok)
            after_ok = after >= len(d) or (not d[after].isalnum() and d[after] != "_")
            if before_ok and after_ok:
                return True
            i = j + 1

    cu = "0"
    if status_s.lower() == "config_cu_failed":
        cu = "1"
    elif "config_cu=" in detail:
        cu = "1"
    elif whole_token(detail, "config_cu_failed"):
        cu = "1"
    elif "config_cu attach failed" in detail.lower():
        cu = "1"
    print("|".join([status_s, b(aiebu), b(ert), b(ran), str(len(detail)), rela_s, cu]))
    sys.exit(0)

if mode == "locked_shape":
    # Align with Rust last_apply_matches_locked_app_load_baseline coarse gate:
    # status ert_cmd_error|ert_error, aiebu=true, ert_error=true, ran=false,
    # rela evidence (top-level >0 or detail rela_count=/aiebu_rela_patched).
    # Full residual class still belongs to soft classify — this is skip identity only.
    ok_status = status_s.lower() in ("ert_cmd_error", "ert_error")
    aiebu_ok = aiebu is True
    ert_ok = ert is True
    ran_ok = ran is False
    rela_ok = False
    if isinstance(rela, int) and rela > 0:
        rela_ok = True
    elif isinstance(rela, float) and rela > 0 and rela == int(rela):
        rela_ok = True
    elif "aiebu_rela_patched" in detail:
        rela_ok = True
    elif "rela_count=" in detail:
        # Require a positive count token when present in detail only.
        import re
        m = re.search(r"rela_count=(\d+)", detail)
        if m and int(m.group(1)) > 0:
            rela_ok = True
    sys.exit(0 if (ok_status and aiebu_ok and ert_ok and ran_ok and rela_ok) else 1)

die(f"unknown mode {mode}", 2)
PY
}

fingerprint_from_json() {
  _python_read_json fingerprint "$1"
}

# Exit 0 = locked shape; 1 = not locked; 2 = I/O/parse refuse.
is_locked_app_load_shape() {
  _python_read_json locked_shape "$1"
}

find_newest_temp() {
  local newest="" newest_mtime=0 f mt
  shopt -s nullglob
  for f in "$TMP_ROOT"/dragon_npu_last_apply_*.json; do
    path_load_allowed "$f" || continue
    [[ -f "$f" ]] || continue
    [[ -L "$f" ]] && continue
    mt=$(stat -c %Y "$f" 2>/dev/null || echo 0)
    if [[ -z "$newest" || "$mt" -gt "$newest_mtime" ]]; then
      newest="$f"
      newest_mtime="$mt"
    fi
  done
  shopt -u nullglob
  if [[ -n "$newest" && -f "$newest" ]]; then
    printf '%s\n' "$newest"
    return 0
  fi
  return 1
}

# --- Resolve path: explicit → env → baseline → temp only if no baseline ---
if [[ -n "$IN" ]]; then
  if ! path_load_allowed "$IN"; then
    echo "ERROR: path refused by load confinement (temp_dir|testdata/xdna2_cmd, .json, no symlink, realpath): $IN" >&2
    exit 2
  fi
  if [[ ! -f "$IN" ]]; then
    echo "ERROR: path not a file: $IN" >&2
    exit 2
  fi
  PATH_PINNED=1
else
  for c in "${XDNA2_LAB_LAST_APPLY_JSON:-}" "${XDNA2_LAST_APPLY_JSON:-}"; do
    [[ -n "$c" ]] || continue
    if ! path_load_allowed "$c"; then
      echo "WARN: env path refused by load confinement: $c" >&2
      continue
    fi
    if [[ -f "$c" ]]; then
      IN="$c"
      PATH_PINNED=1
      break
    fi
  done
fi

if [[ -z "$IN" ]]; then
  if [[ -f "$BASELINE" ]]; then
    # Prefer stable repo fixture over ambient temps (skip stability).
    if path_load_allowed "$BASELINE"; then
      IN="$BASELINE"
    else
      echo "WARN: baseline present but load confinement refuse (soft)" >&2
    fi
  fi
fi

if [[ -z "$IN" ]]; then
  # Newest temp only when no baseline (and no explicit/env).
  if newest_temp="$(find_newest_temp)"; then
    IN="$newest_temp"
  fi
fi

echo "=== c3_post_c2_classify (offline; ≠ C-ready ≠ ran_on_npu) ==="
echo "ROOT=$ROOT"
echo "NOTE: XDNA2_ALLOW_EXEC_CMD unset; never open L3 / never unlock C."
echo "path_pinned=$PATH_PINNED (1=argv/env; never overridden by temp auto-discovery)"

if [[ -z "$IN" || ! -f "$IN" ]]; then
  echo "C3_SKIP no_last_apply not_c_ready=1 not_ran_on_npu=1"
  echo "NOTE: no last_apply.json (fixture or temp). Operator C2 still required for new evidence."
  if [[ "$REQUIRE_NEW" -eq 1 ]]; then
    echo "ERROR: --require-new but no last_apply evidence path" >&2
    exit 2
  fi
  exit 0
fi

if ! IN_ABS="$(abs_path "$IN")"; then
  echo "ERROR: realpath fail-closed for IN: $IN" >&2
  exit 2
fi

BASE_ABS=""
if [[ -f "$BASELINE" ]]; then
  if BASE_ABS="$(abs_path "$BASELINE" 2>/dev/null)"; then
    :
  else
    BASE_ABS=""
    echo "WARN: baseline realpath failed (soft; body compare limited)" >&2
  fi
fi

echo "IN: $IN"
echo "IN_ABS: $IN_ABS"

# --- Fingerprints (secure read); never invent evidence on parse fail ---
FP_IN=""
FP_IN_RC=0
FP_ERR="$(mktemp "${TMP_ROOT}/c3_fp_err.XXXXXX" 2>/dev/null || mktemp)"
set +e
FP_IN="$(fingerprint_from_json "$IN" 2>"$FP_ERR")"
FP_IN_RC=$?
set -e
if [[ "$FP_IN_RC" -ne 0 ]]; then
  err_snip="$(tr '\n' ' ' <"$FP_ERR" 2>/dev/null | cut -c1-200 || true)"
  rm -f "$FP_ERR" 2>/dev/null || true
  echo "WARN: fingerprint parse/I/O failed for IN (rc=$FP_IN_RC): ${err_snip:-unknown}" >&2
  # Baseline path: soft skip — do NOT invent C3_EVIDENCE.
  if [[ -n "$BASE_ABS" && "$IN_ABS" == "$BASE_ABS" ]]; then
    echo "C3_SKIP parse_error_baseline not_c_ready=1 not_ran_on_npu=1"
    echo "NOTE: parse failure on baseline must not invent evidence / re-open matrix."
    if [[ "$REQUIRE_NEW" -eq 1 ]]; then
      echo "ERROR: --require-new and no parseable new evidence" >&2
      exit 2
    fi
    exit 0
  fi
  # Explicit non-baseline unreadable → hard refuse (not false evidence).
  if [[ "$PATH_PINNED" -eq 1 ]]; then
    echo "ERROR: pinned path unreadable/unparseable: $IN" >&2
    exit 2
  fi
  echo "C3_SKIP parse_error not_c_ready=1 not_ran_on_npu=1"
  if [[ "$REQUIRE_NEW" -eq 1 ]]; then
    echo "ERROR: --require-new and no parseable new evidence" >&2
    exit 2
  fi
  exit 0
fi
rm -f "$FP_ERR" 2>/dev/null || true
echo "fingerprint=$FP_IN"

FP_BASE=""
if [[ -n "$BASE_ABS" && -f "$BASELINE" ]]; then
  set +e
  FP_BASE="$(fingerprint_from_json "$BASELINE" 2>/dev/null)"
  fp_base_rc=$?
  set -e
  if [[ "$fp_base_rc" -eq 0 && -n "$FP_BASE" ]]; then
    echo "baseline_fingerprint=$FP_BASE"
  else
    FP_BASE=""
    echo "WARN: baseline fingerprint unavailable (soft)" >&2
  fi
fi

# Locked shape on IN (exit 0/1/2).
LOCKED_RC=1
set +e
is_locked_app_load_shape "$IN"
LOCKED_RC=$?
set -e
if [[ "$LOCKED_RC" -eq 2 ]]; then
  # Should be rare after fingerprint ok; soft skip, no invent evidence.
  echo "C3_SKIP locked_shape_io_error not_c_ready=1 not_ran_on_npu=1"
  if [[ "$REQUIRE_NEW" -eq 1 ]]; then
    echo "ERROR: --require-new and locked_shape I/O error" >&2
    exit 2
  fi
  exit 0
fi

EVIDENCE=0
REASON=""
SHAPE_STILL_LOCKED=0

# Body-first via **fingerprint equality only** (Issue 7):
# - same_body_as_baseline / baseline skip **only** when FP_IN == FP_BASE
# - FP differs → always C3_EVIDENCE=1 (even if still locked APP_LOAD class);
#   annotate shape_still_locked_app_load=1 when locked shape matches
# - Never force C3_SKIP from locked-shape alone when FP differs
if [[ -n "$FP_BASE" && "$FP_IN" == "$FP_BASE" ]]; then
  EVIDENCE=0
  if [[ -n "$BASE_ABS" && "$IN_ABS" == "$BASE_ABS" ]]; then
    REASON="baseline_locked_app_load"
  else
    REASON="same_body_as_baseline"
  fi
elif [[ -n "$FP_BASE" ]]; then
  # Fingerprint differs from baseline file → new evidence (body changed).
  EVIDENCE=1
  if [[ -n "$BASE_ABS" && "$IN_ABS" == "$BASE_ABS" ]]; then
    REASON="baseline_body_changed"
  else
    REASON="body_differs_from_baseline"
  fi
  if [[ "$LOCKED_RC" -eq 0 ]]; then
    SHAPE_STILL_LOCKED=1
  fi
elif [[ -n "$BASE_ABS" && "$IN_ABS" == "$BASE_ABS" ]]; then
  # Baseline path but FP_BASE unavailable: skip only when locked shape (soft
  # stability); do not invent same_body without FP equality.
  if [[ "$LOCKED_RC" -eq 0 ]]; then
    EVIDENCE=0
    REASON="baseline_locked_app_load"
  else
    EVIDENCE=1
    REASON="baseline_shape_changed"
  fi
else
  # No baseline fingerprint to compare — any other shot is evidence.
  EVIDENCE=1
  REASON="no_baseline_new_shot"
  if [[ "$LOCKED_RC" -eq 0 ]]; then
    SHAPE_STILL_LOCKED=1
  fi
fi

# Honesty: never swap IN to a temp after resolve (PATH_PINNED or baseline pick).
# (Older buggy path re-selected newer_temp_exists — removed.)

echo "evidence_reason=$REASON"
if [[ "$SHAPE_STILL_LOCKED" -eq 1 ]]; then
  echo "shape_still_locked_app_load=1"
else
  echo "shape_still_locked_app_load=0"
fi

if [[ "$EVIDENCE" -eq 0 ]]; then
  echo "C3_SKIP no_new_c2_evidence reason=$REASON not_c_ready=1 not_ran_on_npu=1"
  echo "NOTE: Offline APP_LOAD signal matrix CLOSED for same fixture — not new research."
  echo "NOTE: C3_SKIP ≠ unlock consume. See RESIDUAL.md top Unlock row for active vs already-used stamp."
  echo "NOTE: Further live C after the active stamp is used needs a new agent unlock + offline delta."
  if [[ "$NO_CLASSIFY" -eq 0 ]]; then
    echo "--- soft classify (feedback only; ≠ C-ready) ---"
    set +e
    bash "$ROOT/tools/classify_last_apply_residual.sh" "$IN"
    cl_rc=$?
    set -e
    echo "classify_rc=$cl_rc (soft)"
  fi
  if [[ "$REQUIRE_NEW" -eq 1 ]]; then
    echo "ERROR: --require-new and no new C2 evidence (C3_SKIP)" >&2
    exit 2
  fi
  exit 0
fi

echo "C3_EVIDENCE=1 reason=$REASON not_c_ready=1 not_ran_on_npu=1"
if [[ "$SHAPE_STILL_LOCKED" -eq 1 ]]; then
  echo "NOTE: Fingerprint differs from baseline (new shot) but residual shape still locked APP_LOAD — still ≠ Pass / ≠ C-ready."
fi
echo "NOTE: New last_apply body (FP ≠ baseline) — full soft classify; still ≠ C-ready / ≠ Pass."

if [[ "$NO_CLASSIFY" -eq 0 ]]; then
  echo "--- soft classify (new evidence) ---"
  set +e
  bash "$ROOT/tools/classify_last_apply_residual.sh" "$IN"
  cl_rc=$?
  set -e
  echo "classify_rc=$cl_rc (soft; ≠ C-ready)"
fi

# Optional journal hints only (never invent residual class).
# Soft dual-pair parse mirrors pure JournalFwStatusPair / parse_journal_fw_status_pair
# (Failed cmd + Status). Keep token shape in sync with format_soft_note (incl invent_pass=0).
# Longer-term: call cargo classify example instead of this Python twin.
if [[ "$SKIP_JOURNAL" -eq 0 ]]; then
  if command -v journalctl >/dev/null 2>&1; then
    echo "--- journal hints (read-only; journal_hint= only) ---"
    set +e
    j_out="$(journalctl -k -n 200 --no-pager 2>/dev/null | grep -iE '0x3000003|0x4000003|APP_LOAD|Failed cmd|Status 0x' || true)"
    set -e
    if [[ -n "${j_out//[[:space:]]/}" ]]; then
      while IFS= read -r line; do
        [[ -n "$line" ]] || continue
        echo "journal_hint=${line}"
      done <<<"$j_out"
      # Soft dual-status pair (research only; never invents residual class / Pass).
      # Mirrors pure JournalFwStatusPair; stdin = journal lines (heredoc is -c script).
      if command -v python3 >/dev/null 2>&1; then
        set +e
        pair_note="$(printf '%s\n' "$j_out" | python3 -c '
import re, sys
text = sys.stdin.read()
failed = chain = None
for line in text.splitlines():
    ll = line.lower()
    if failed is None and ("failed cmd" in ll or "failed_cmd" in ll):
        m = re.search(r"0x([0-9a-fA-F]+)", line)
        if m:
            failed = int(m.group(1), 16) & 0xFFFFFFFF
    if chain is None and "status 0x" in ll and "failed cmd" not in ll and "failed_cmd" not in ll:
        m = re.search(r"0x([0-9a-fA-F]+)", line)
        if m:
            chain = int(m.group(1), 16) & 0xFFFFFFFF
parts = ["journal_fw_pair", "not_c_ready=1", "not_ran_on_npu=1", "invent_pass=0"]
if failed is None:
    parts.append("failed_cmd=absent")
else:
    parts.append("failed_cmd=0x%x" % failed)
    if failed == 0x3000003:
        parts.append("failed_cmd_name=APP_LOAD_PDI_FAIL")
if chain is None:
    parts.append("chain_status=absent")
else:
    parts.append("chain_status=0x%x" % chain)
    if chain == 0x4000003:
        parts.append("chain_name=AIE2_STATUS_INVALID_PARAM")
if failed == 0x3000003 and chain == 0x4000003:
    parts.append("c2_app_load_pair=1")
print(" ".join(parts))
')"
        set -e
        if [[ -n "${pair_note//[[:space:]]/}" ]]; then
          echo "journal_fw_pair_soft=${pair_note}"
        fi
      fi
    else
      echo "journal_hint=none (no matching lines in last 200 kernel msgs)"
    fi
  else
    echo "journal_hint=skipped (journalctl not available)"
  fi
else
  echo "journal_hint=skipped (--skip-journal)"
fi

echo "C3_DONE evidence=1 not_c_ready=1 not_ran_on_npu=1"
echo "NOTE: paste classify + last_apply + journal_hint for agent C3 residual;"
echo "  then bash tools/c4_pass_check.sh (offline PassCandidate audit only —"
echo "  C4_PASS_CANDIDATE ≠ invent product Pass; product ran_on_npu = live decide)."
exit 0
