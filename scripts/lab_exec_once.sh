#!/usr/bin/env bash
# lab_exec_once.sh — one-shot, wedge-aware EXEC_CMD lab window for XDNA2.
#
# Safety guarantees:
#   - Default: dry-run; no EXEC, no sudo, no ALLOW, no modprobe
#   - --apply requires --i-accept-wedge-risk AND --blob PATH
#   - Refuses diagnose WEDGED_* and refcnt > --refcnt-max (unless allow-suspicious)
#   - Never opens /dev/accel during preflight
#   - Never stores sudo password; never uses askpass files
#   - trap EXIT restores exec_cmd_allow→0 path + unsets XDNA2_ALLOW_EXEC_CMD
#   - Outer timeout(1) + finite XDNA2_WAIT_TIMEOUT_MS; no WAIT infinite
#   - Single cargo/example invocation; no retry loop
#   - Does not write permanent modprobe.d exec_cmd_allow=1
#
# Honesty: success of this script ≠ product ran_on_npu=true. Without a
# known-good instruction blob + OFM numerical verify, lab apply is research
# plumbing only. See docs/XDNA2_EXEC_CMD_LAB.md and docs/XDNA2_NEVER_AGAIN.md.
#
# Exit codes:
#   0  Dry-run plan OK, or apply: cargo/example exit 0 AND restore verified
#      (npu_direct_bench exits 0 only when ran_on_npu=true — rare without known-good txn)
#   1  Preflight refuse (WEDGED, high refcnt, missing module on apply, …)
#   2  Bad args / missing required flags / invalid numeric options
#   3  Root/sudo needed but password required / --print-sudo-only
#   4  Live process non-zero or timed out (gates still restored).
#      Includes npu_direct_bench exit 2 (blocked_npu_compute / !ran_on_npu) —
#      EXEC may have run; product Pass still false. Restore must succeed first.
#   5  Restore failed (CRITICAL — print manual restore block)
#
# Apply always exports XDNA2_LAB_OFM=1 so timed path attempts OFM decide wire.
#
# Usage:
#   bash scripts/lab_exec_once.sh
#   bash scripts/lab_exec_once.sh --help
#   bash scripts/lab_exec_once.sh --blob /path/to.bin   # dry-run with blob preview
#   bash scripts/lab_exec_once.sh --apply --i-accept-wedge-risk --blob /path/to.bin
#
# Prefer: PATH=/usr/bin:$PATH (Garuda / wild PATH hygiene).
#
# Residual: SIGKILL / power-loss cannot run traps — operator must still verify
# exec_cmd_allow=0 after a hard kill (see RESIDUAL.md).

set -euo pipefail

# Refuse `source scripts/lab_exec_once.sh` (would pollute parent env / miss traps).
if [[ "${BASH_SOURCE[0]}" != "${0}" ]]; then
  echo "ERROR: do not source lab_exec_once.sh; run: bash scripts/lab_exec_once.sh …" >&2
  return 2 2>/dev/null || exit 2
fi

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MODE="dry-run"
BLOB=""
ACCEPT_RISK=0
SKIP_VERIFY=0
ALLOW_SUSPICIOUS=0
ALLOW_STOCK_KERNEL=0
ALLOW_RESEARCH_BLOB=0
REFCNT_MAX=5
TIMEOUT_S="${LAB_EXEC_TIMEOUT_S:-30}"
WAIT_MS="${XDNA2_WAIT_TIMEOUT_MS:-5000}"
DIAG_TIMEOUT_S="${LAB_EXEC_DIAG_TIMEOUT_S:-30}"
EXAMPLE="npu_direct_bench"
# Default example args as array (no unquoted expansion).
CARGO_EXTRA=(-- --m 1 --n 1 --k 1 --dtype f32)
PRINT_SUDO_ONLY=0
NO_BUILD=0
ASSUME_L3_OPEN=0
MAX_BLOB_BYTES=$((64 * 1024 * 1024)) # mirror MAX_INSTRUCTION_BLOB_BYTES

# L3_WE_ENABLED=1 only when this script intentionally opened L3 (or assumed ownership).
L3_WE_ENABLED=0
RESTORE_OK=0
# RESTORE_RC: 0 = restore clean; 5 = restore failed (prefer over signal codes on trap).
RESTORE_RC=0
EXEC_RC=0

die() {
  local code="$1"
  shift
  echo "ERROR: $*" >&2
  exit "$code"
}

usage() {
  cat <<'EOF'
Usage: bash scripts/lab_exec_once.sh [options]

Default (no --apply): DRY-RUN / plan only
  - Preflight diagnose (+ optional verify)
  - Validate --blob if provided (size/sha preview)
  - Print exact enable / cargo / restore commands
  - Never: modprobe, sudo, EXEC_CMD, set ALLOW, open /dev/accel for work

--apply requires ALL of:
  --i-accept-wedge-risk    Explicit operator risk acceptance (wedge/oops)
  --blob PATH              Non-empty instruction/txn blob file

Other options:
  --blob PATH              Dry-run preview or apply required path
  --skip-verify            Skip verify_amdxdna_hardened_host.sh (requires --allow-stock-kernel)
  --allow-suspicious-refcnt  Apply may continue if refcnt > max (still refuse WEDGED_*)
  --allow-stock-kernel     DANGEROUS: allow apply when hardened verify fails (stock amdxdna).
                           Required as an explicit CLI flag — LAB_EXEC_ALLOW_STOCK=1 alone is NOT enough.
                           Also required with --skip-verify (skipping verify = stock risk).
  --allow-research-blob    Allow --blob whose sibling PROVENANCE.md marks NOT_AIE2_KNOWN_GOOD=1
                           (VE2 / research staging). Default: refuse on apply (exit 2).
  --refcnt-max N           Default 5
  --timeout-s N            Outer timeout around live cargo (default 30)
  --wait-timeout-ms N      XDNA2_WAIT_TIMEOUT_MS (default 5000)
  --example NAME           Default: npu_direct_bench (charset [A-Za-z0-9_-]+)
  --cargo-args "…"         Args after -- for the example (quoted carefully)
  --print-sudo-only        Never invoke sudo; print human commands (exit 3 if root needed)
  --assume-l3-already-open On apply: require sysfs exec_cmd_allow already 1/Y
  --no-build               Skip cargo build
  -h | --help

Exit codes:
  0 dry-run OK, or apply example exit 0 + restore OK
    (npu_direct_bench exits 0 only if ran_on_npu — usually need known-good blob)
  1 preflight refuse
  2 bad args / invalid numbers (timeout-s/wait-timeout-ms must be >=1)
  3 sudo needed
  4 process non-zero/timeout (incl. bench exit 2 = !ran_on_npu) + restore OK
  5 restore failed

Honesty: apply exit 0 is rare without known-good txn; exit 4 with restore OK is
common and does NOT mean gates left open. Need blob + OFM verify for Pass.
EOF
}

require_positive_int() {
  local name="$1"
  local val="$2"
  if ! [[ "$val" =~ ^[1-9][0-9]*$ ]]; then
    die 2 "$name must be a positive integer >= 1 (got '${val}')"
  fi
}

require_nonneg_int() {
  local name="$1"
  local val="$2"
  if ! [[ "$val" =~ ^[0-9]+$ ]]; then
    die 2 "$name must be a non-negative integer (got '${val}')"
  fi
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --apply) MODE="apply"; shift ;;
    --i-accept-wedge-risk) ACCEPT_RISK=1; shift ;;
    --blob)
      [[ $# -ge 2 ]] || die 2 "--blob requires PATH"
      BLOB="$2"
      shift 2
      ;;
    --skip-verify) SKIP_VERIFY=1; shift ;;
    --allow-suspicious-refcnt) ALLOW_SUSPICIOUS=1; shift ;;
    --allow-stock-kernel) ALLOW_STOCK_KERNEL=1; shift ;;
    --allow-research-blob) ALLOW_RESEARCH_BLOB=1; shift ;;
    --refcnt-max)
      [[ $# -ge 2 ]] || die 2 "--refcnt-max requires N"
      REFCNT_MAX="$2"
      shift 2
      ;;
    --timeout-s)
      [[ $# -ge 2 ]] || die 2 "--timeout-s requires N"
      TIMEOUT_S="$2"
      shift 2
      ;;
    --wait-timeout-ms)
      [[ $# -ge 2 ]] || die 2 "--wait-timeout-ms requires N"
      WAIT_MS="$2"
      shift 2
      ;;
    --example)
      [[ $# -ge 2 ]] || die 2 "--example requires NAME"
      EXAMPLE="$2"
      if ! [[ "$EXAMPLE" =~ ^[A-Za-z0-9_-]+$ ]]; then
        die 2 "--example must match [A-Za-z0-9_-]+ (got '${EXAMPLE}')"
      fi
      shift 2
      ;;
    --cargo-args)
      [[ $# -ge 2 ]] || die 2 "--cargo-args requires string"
      # Refuse shell metacharacters; split into array without eval.
      if [[ "$2" =~ [\;\|\&\`\$\(\)\<\>] ]]; then
        die 2 "--cargo-args refuses shell metacharacters (;|&\`\$()<>)"
      fi
      # shellcheck disable=SC2206
      CARGO_EXTRA=($2)
      shift 2
      ;;
    --print-sudo-only) PRINT_SUDO_ONLY=1; shift ;;
    --assume-l3-already-open) ASSUME_L3_OPEN=1; shift ;;
    --no-build) NO_BUILD=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) die 2 "unknown argument: $1 (try --help)" ;;
  esac
done

# Fail closed: timeout 0 disables outer timeout(1); wait 0 is infinite WAIT (refused by lib).
require_positive_int "--timeout-s / LAB_EXEC_TIMEOUT_S" "$TIMEOUT_S"
require_positive_int "--wait-timeout-ms / XDNA2_WAIT_TIMEOUT_MS" "$WAIT_MS"
require_positive_int "LAB_EXEC_DIAG_TIMEOUT_S" "$DIAG_TIMEOUT_S"
require_nonneg_int "--refcnt-max" "$REFCNT_MAX"
if ! [[ "$EXAMPLE" =~ ^[A-Za-z0-9_-]+$ ]]; then
  die 2 "--example must match [A-Za-z0-9_-]+ (got '${EXAMPLE}')"
fi
# Loud warning if env-only stock override is set without the CLI flag.
if [[ "${LAB_EXEC_ALLOW_STOCK:-0}" == "1" && "$ALLOW_STOCK_KERNEL" -ne 1 ]]; then
  echo "WARN: LAB_EXEC_ALLOW_STOCK=1 is IGNORED without --allow-stock-kernel (explicit CLI required)" >&2
fi
# Skipping hardened verify is a stock-kernel risk class — require explicit stock allow.
if [[ "$SKIP_VERIFY" -eq 1 && "$ALLOW_STOCK_KERNEL" -ne 1 ]]; then
  die 2 "--skip-verify requires --allow-stock-kernel (skipping hardened verify is stock/NULL risk)"
fi

# Refuse blobs staged as research-only (VE2 / NOT_AIE2_KNOWN_GOOD) unless opt-in.
# Checks: same dir PROVENANCE.md, parent PROVENANCE.md, blob.PROVENANCE.md.
blob_has_not_aie2_provenance() {
  local blob="$1"
  local dir parent base candidates c
  dir=$(dirname -- "$blob")
  parent=$(dirname -- "$dir")
  base=$(basename -- "$blob")
  candidates=(
    "$dir/PROVENANCE.md"
    "$parent/PROVENANCE.md"
    "$dir/${base}.PROVENANCE.md"
    "$blob.PROVENANCE.md"
  )
  for c in "${candidates[@]}"; do
    if [[ -f "$c" ]] && grep -qE 'NOT_AIE2_KNOWN_GOOD[[:space:]]*=[[:space:]]*1' "$c" 2>/dev/null; then
      echo "$c"
      return 0
    fi
  done
  return 1
}

require_apply_flags() {
  [[ "$MODE" == "apply" ]] || return 0
  [[ "$ACCEPT_RISK" -eq 1 ]] || die 2 "--apply requires --i-accept-wedge-risk (wedge/oops risk)"
  [[ -n "$BLOB" ]] || die 2 "--apply requires --blob PATH (see scripts/capture_instruction_blob_checklist.md)"
  [[ -f "$BLOB" ]] || die 2 "blob is not a regular file: $BLOB"
  [[ -s "$BLOB" ]] || die 2 "blob is empty: $BLOB"
  local sz
  sz="$(wc -c <"$BLOB" | tr -d ' ')"
  if [[ "$sz" -gt "$MAX_BLOB_BYTES" ]]; then
    die 2 "blob size $sz exceeds MAX_INSTRUCTION_BLOB_BYTES ($MAX_BLOB_BYTES)"
  fi
  local prov
  if prov=$(blob_has_not_aie2_provenance "$BLOB"); then
    if [[ "$ALLOW_RESEARCH_BLOB" -ne 1 ]]; then
      die 2 "blob has research PROVENANCE NOT_AIE2_KNOWN_GOOD=1 ($prov); \
refuse without --allow-research-blob (VE2/staging is not Strix Pass)"
    fi
    echo "WARN: --allow-research-blob: using NOT_AIE2_KNOWN_GOOD blob ($prov) — not product Pass" >&2
  fi
  if ! command -v timeout >/dev/null 2>&1; then
    die 2 "timeout(1) required for --apply (coreutils)"
  fi
}

check_blob_preview() {
  if [[ -z "$BLOB" ]]; then
    echo "BLOB: (missing — required for --apply; see capture_instruction_blob_checklist.md)"
    return 0
  fi
  if [[ ! -f "$BLOB" ]]; then
    echo "BLOB: path not a file: $BLOB"
    return 0
  fi
  local abs sz sha head
  abs="$(readlink -f "$BLOB" 2>/dev/null || realpath "$BLOB" 2>/dev/null || echo "$BLOB")"
  sz="$(wc -c <"$BLOB" | tr -d ' ')"
  if command -v sha256sum >/dev/null 2>&1; then
    sha="$(sha256sum "$BLOB" | awk '{print $1}')"
  else
    sha="(sha256sum not available)"
  fi
  if command -v xxd >/dev/null 2>&1; then
    head="$(xxd -l 32 -p "$BLOB" | tr -d '\n')"
  else
    head="$(od -An -tx1 -N 32 "$BLOB" | tr -s ' ' | tr -d '\n')"
  fi
  echo "BLOB: path=$abs size=$sz sha256=$sha head32=$head"
  if [[ "$sz" -eq 0 ]]; then
    echo "BLOB_WARN: empty file"
  fi
  local prov
  if prov=$(blob_has_not_aie2_provenance "$BLOB"); then
    if [[ "$ALLOW_RESEARCH_BLOB" -eq 1 ]]; then
      echo "BLOB_WARN: research NOT_AIE2_KNOWN_GOOD ($prov) allowed via --allow-research-blob"
    else
      echo "BLOB_WARN: research NOT_AIE2_KNOWN_GOOD ($prov) — apply will refuse without --allow-research-blob"
    fi
  fi
  if [[ "$sz" -gt "$MAX_BLOB_BYTES" ]]; then
    echo "BLOB_WARN: oversize > $MAX_BLOB_BYTES"
    if [[ "$MODE" == "apply" ]]; then
      die 2 "blob oversize"
    fi
  fi
  case "$abs" in
    */tests/fixtures/*) echo "BLOB_WARN: under tests/fixtures — prefer testdata/xdna2_cmd/ or /tmp" ;;
  esac
}

run_diagnose() {
  local out
  if ! command -v timeout >/dev/null 2>&1; then
    out="$(bash "$ROOT/scripts/diagnose_amdxdna.sh" 2>&1 || true)"
  else
    out="$(timeout "${DIAG_TIMEOUT_S}s" bash "$ROOT/scripts/diagnose_amdxdna.sh" 2>&1 || true)"
  fi
  DIAG_OUT="$out"
  DIAG_VERDICT="$(printf '%s\n' "$out" | awk -F': ' '/^verdict:/{print $2; exit}' | tr -d '[:space:]')"
  REFCNT="$(printf '%s\n' "$out" | awk -F': ' '/^refcnt:/{print $2; exit}' | tr -d '[:space:]')"
  if [[ -z "$DIAG_VERDICT" ]]; then
    DIAG_VERDICT="PREFLIGHT_UNKNOWN"
  fi
  if [[ -z "$REFCNT" ]]; then
    REFCNT="unknown"
  fi
}

preflight_refuse() {
  echo "PREFLIGHT: verdict=$DIAG_VERDICT refcnt=$REFCNT"
  case "$DIAG_VERDICT" in
    WEDGED_OPEN|WEDGED_OTHER|WEDGED_*)
      die 1 "diagnose verdict=$DIAG_VERDICT — refuse lab EXEC (see NEVER_AGAIN / recovery docs)"
      ;;
    PREFLIGHT_UNKNOWN)
      if [[ "$MODE" == "apply" ]]; then
        die 1 "diagnose produced no verdict (timeout/fail-closed)"
      fi
      echo "PREFLIGHT_WARN: no verdict (dry-run continues)"
      ;;
  esac
  if [[ "$REFCNT" =~ ^[0-9]+$ ]] && [[ "$REFCNT" -gt "$REFCNT_MAX" ]]; then
    if [[ "$MODE" == "apply" && "$ALLOW_SUSPICIOUS" -eq 0 ]]; then
      die 1 "refcnt=$REFCNT > max $REFCNT_MAX (use --allow-suspicious-refcnt only if intentional)"
    fi
    echo "PREFLIGHT_WARN: high refcnt=$REFCNT (max $REFCNT_MAX)"
  fi
  if [[ "$MODE" == "apply" ]]; then
    if ! lsmod | awk '{print $1}' | grep -qx amdxdna; then
      die 1 "amdxdna module not loaded — cannot EXEC on apply"
    fi
  fi
}

run_verify() {
  [[ "$SKIP_VERIFY" -eq 1 ]] && return 0
  local rc=0
  if [[ -x "$ROOT/scripts/verify_amdxdna_hardened_host.sh" ]]; then
    set +e
    bash "$ROOT/scripts/verify_amdxdna_hardened_host.sh" --quiet
    rc=$?
    set -e
  else
    echo "VERIFY_WARN: verify script missing"
    return 0
  fi
  if [[ $rc -ne 0 ]]; then
    if [[ "$MODE" == "apply" ]]; then
      if [[ "$ALLOW_STOCK_KERNEL" -eq 1 ]]; then
        echo "VERIFY_WARN: hardened verify failed (rc=$rc); continuing ONLY because --allow-stock-kernel (DANGEROUS stock amdxdna NULL risk)" >&2
      else
        die 1 "verify_amdxdna_hardened_host failed (rc=$rc). Refuse apply. \
To override intentionally: pass --allow-stock-kernel (LAB_EXEC_ALLOW_STOCK=1 alone is NOT enough)."
      fi
    else
      echo "VERIFY_WARN: hardened verify failed (rc=$rc); dry-run continues"
    fi
  else
    echo "VERIFY: hardened host OK"
  fi
}

print_human_enable() {
  cat <<EOF
======== HUMAN ROOT STEPS (password required; script will not store it) ========
# Enable L3 for this window only:
sudo sh -c 'echo 1 > /sys/module/amdxdna/parameters/exec_cmd_allow'
cat /sys/module/amdxdna/parameters/exec_cmd_allow   # expect Y or 1

# Then re-run apply with the same flags, optionally:
#   --assume-l3-already-open

# After lab (MANDATORY):
sudo sh -c 'echo 0 > /sys/module/amdxdna/parameters/exec_cmd_allow'
unset XDNA2_ALLOW_EXEC_CMD
bash scripts/diagnose_amdxdna.sh
================================================================================
EOF
}

print_human_restore() {
  cat <<'EOF'
======== CRITICAL: restore exec_cmd_allow manually ========
sudo sh -c 'echo 0 > /sys/module/amdxdna/parameters/exec_cmd_allow'
unset XDNA2_ALLOW_EXEC_CMD
cat /sys/module/amdxdna/parameters/exec_cmd_allow   # expect N or 0
bash scripts/diagnose_amdxdna.sh
===========================================================
EOF
}

sudo_can_n() {
  command -v sudo >/dev/null 2>&1 || return 1
  sudo -n true 2>/dev/null
}

read_exec_cmd_allow() {
  local p="/sys/module/amdxdna/parameters/exec_cmd_allow"
  if [[ -r "$p" ]]; then
    tr -d '[:space:]' <"$p" || true
  else
    echo "missing"
  fi
}

enable_l3() {
  local cur
  cur="$(read_exec_cmd_allow)"
  if [[ "$ASSUME_L3_OPEN" -eq 1 ]]; then
    case "$cur" in
      1|Y|y)
        echo "L3: assume-open confirmed exec_cmd_allow=$cur (script owns restore→0)"
        L3_WE_ENABLED=1
        return 0
        ;;
      *) die 1 "--assume-l3-already-open but exec_cmd_allow=$cur (not enabled)" ;;
    esac
  fi
  if [[ "$PRINT_SUDO_ONLY" -eq 1 ]]; then
    print_human_enable
    exit 3
  fi
  if [[ "$(id -u)" -eq 0 ]]; then
    echo 1 > /sys/module/amdxdna/parameters/exec_cmd_allow
    L3_WE_ENABLED=1
    echo "L3: enabled as root (exec_cmd_allow=$(read_exec_cmd_allow))"
    return 0
  fi
  if sudo_can_n; then
    timeout 15s sudo -n sh -c 'echo 1 > /sys/module/amdxdna/parameters/exec_cmd_allow' \
      || die 3 "sudo -n failed writing exec_cmd_allow=1"
    L3_WE_ENABLED=1
    echo "L3: enabled via sudo -n (exec_cmd_allow=$(read_exec_cmd_allow))"
    return 0
  fi
  print_human_enable
  exit 3
}

restore_l3() {
  # Always scrub userspace lab env (process-local).
  unset XDNA2_ALLOW_EXEC_CMD || true
  unset XDNA2_WAIT_ALLOW_INFINITE || true
  unset XDNA2_LAB_OFM || true

  local ok=1
  local val
  val="$(read_exec_cmd_allow)"

  # Only write sysfs when **this script** opened L3 (or assumed ownership).
  # Never clear a foreign operator session if we never enabled.
  if [[ "$L3_WE_ENABLED" -eq 1 ]]; then
    if [[ "$(id -u)" -eq 0 ]]; then
      echo 0 > /sys/module/amdxdna/parameters/exec_cmd_allow 2>/dev/null || ok=0
    elif [[ "$PRINT_SUDO_ONLY" -eq 0 ]] && sudo_can_n; then
      timeout 15s sudo -n sh -c 'echo 0 > /sys/module/amdxdna/parameters/exec_cmd_allow' 2>/dev/null || ok=0
    else
      ok=0
    fi
    val="$(read_exec_cmd_allow)"
    case "$val" in
      0|N|n) ;;
      *)
        ok=0
        ;;
    esac
  fi

  if [[ -n "${XDNA2_ALLOW_EXEC_CMD+x}" ]]; then
    ok=0
  fi

  if [[ "$ok" -eq 1 ]]; then
    RESTORE_OK=1
    RESTORE_RC=0
  else
    RESTORE_OK=0
    RESTORE_RC=5
  fi

  echo "RESTORE: l3_we_enabled=$L3_WE_ENABLED exec_cmd_allow=$val \
ALLOW_unset=$([[ -z "${XDNA2_ALLOW_EXEC_CMD+x}" ]] && echo yes || echo no) \
ok=$RESTORE_OK rc=$RESTORE_RC"
  if [[ "$ok" -eq 0 ]]; then
    print_human_restore
  fi
}

cleanup_lab_window() {
  restore_l3
  trap - EXIT INT TERM
}

# Prefer exit 5 (restore failed) over signal exit codes when cleanup cannot restore.
on_signal_exit() {
  local sig_rc="$1"
  cleanup_lab_window
  if [[ "$RESTORE_RC" -eq 5 ]]; then
    exit 5
  fi
  exit "$sig_rc"
}

print_plan() {
  local abs_blob="(none)"
  if [[ -n "$BLOB" ]]; then
    abs_blob="$(readlink -f "$BLOB" 2>/dev/null || echo "$BLOB")"
  fi
  cat <<EOF
MODE: dry-run
PREFLIGHT: verdict=$DIAG_VERDICT refcnt=$REFCNT
BLOB_PATH: $abs_blob
WOULD enable (apply only; not executed now):
  1. cargo build --features amd-xdna2,xdna2-dangerous-exec-cmd --example $EXAMPLE
  2. [sudo] temporarily exec_cmd_allow=1 (sysfs preferred; never leave permanent)
  3. export XDNA2_ALLOW_EXEC_CMD=1
  4. export XDNA2_INSTRUCTION_BLOB=$abs_blob
  5. export XDNA2_WAIT_TIMEOUT_MS=$WAIT_MS
  6. export XDNA2_LAB_OFM=1   # OFM decide wire (still needs known-good txn)
  7. timeout ${TIMEOUT_S}s cargo run --features amd-xdna2,xdna2-dangerous-exec-cmd --example $EXAMPLE ${CARGO_EXTRA[*]}
  8. restore exec_cmd_allow=0 only if this script enabled L3; unset ALLOW / LAB_OFM
THEN: re-run diagnose; expect exec_cmd_allow N/0 after apply we owned

Honesty:
  - npu_direct_bench exits 0 only if ran_on_npu; else exit 2 → this script exit 4 (if restore OK).
  - cargo build/run non-zero (incl. 101) → exit 4 after restore attempt.
  - exit 4 with restore OK is NOT product Pass and does NOT leave gates open.
  - Need known-good blob + OFM numerical match for true Pass.
  - SIGKILL cannot run traps — verify exec_cmd_allow manually after hard kill.

No state changed.
EOF
}

main() {
  require_apply_flags
  run_diagnose
  preflight_refuse
  run_verify
  check_blob_preview

  if [[ "$MODE" == "dry-run" ]]; then
    print_plan
    exit 0
  fi

  # Apply path
  if [[ "$NO_BUILD" -eq 0 ]]; then
    echo "BUILD: cargo build --features amd-xdna2,xdna2-dangerous-exec-cmd --example $EXAMPLE"
    set +e
    (cd "$ROOT" && cargo build --features amd-xdna2,xdna2-dangerous-exec-cmd --example "$EXAMPLE")
    build_rc=$?
    set -e
    if [[ "$build_rc" -ne 0 ]]; then
      echo "ERROR: cargo build failed rc=$build_rc (mapped to exit 4; no L3 opened yet)" >&2
      exit 4
    fi
  fi

  trap cleanup_lab_window EXIT
  trap 'on_signal_exit 130' INT
  trap 'on_signal_exit 143' TERM

  enable_l3

  local abs_blob
  abs_blob="$(readlink -f "$BLOB" 2>/dev/null || realpath "$BLOB" 2>/dev/null || echo "$BLOB")"
  export XDNA2_ALLOW_EXEC_CMD=1
  export XDNA2_INSTRUCTION_BLOB="$abs_blob"
  export XDNA2_WAIT_TIMEOUT_MS="$WAIT_MS"
  # Always enable lab OFM wire on apply so timed path can decide (still need good txn).
  export XDNA2_LAB_OFM=1
  unset XDNA2_WAIT_ALLOW_INFINITE || true

  echo "EXEC: single-shot under timeout ${TIMEOUT_S}s (example=$EXAMPLE LAB_OFM=1)"
  set +e
  (cd "$ROOT" && timeout "${TIMEOUT_S}s" env \
    XDNA2_ALLOW_EXEC_CMD=1 \
    XDNA2_INSTRUCTION_BLOB="$abs_blob" \
    XDNA2_WAIT_TIMEOUT_MS="$WAIT_MS" \
    XDNA2_LAB_OFM=1 \
    cargo run --features amd-xdna2,xdna2-dangerous-exec-cmd --example "$EXAMPLE" \
    "${CARGO_EXTRA[@]}")
  EXEC_RC=$?
  set -e

  # Scrub lab env before restore check (parent shell is this process only).
  unset XDNA2_LAB_OFM || true

  cleanup_lab_window
  if [[ "$RESTORE_OK" -ne 1 || "$RESTORE_RC" -eq 5 ]]; then
    exit 5
  fi
  if [[ "$EXEC_RC" -ne 0 ]]; then
    # Map all process failures (timeout 124, cargo 101, bench 2, …) to exit 4.
    echo "EXEC: process rc=$EXEC_RC (timeout/cargo/bench non-zero) — gates restored; script exit 4"
    if [[ "$EXEC_RC" -eq 2 ]]; then
      echo "NOTE: npu_direct_bench exits 2 when ran_on_npu=false (blocked_npu_compute)."
      echo "      That is expected without known-good txn+OFM match — not product Pass."
    fi
    if [[ "$EXEC_RC" -eq 101 ]]; then
      echo "NOTE: cargo often exits 101 on compile/run errors — mapped to lab exit 4."
    fi
    exit 4
  fi
  echo "EXEC: process exit 0 + restore OK (npu_direct_bench implies ran_on_npu=true if default example)"
  echo "POST: record dmesg/status; prefer scripts/diagnose_amdxdna.sh"
  exit 0
}

main
