#!/usr/bin/env bash
# Phase 0 — read-only verification that this host still has the hardened lab
# amdxdna story (L2+L3 OOT under updates/, exec_cmd_allow off, firmware alias,
# modprobe.d options-only, diagnose not WEDGED_*).
#
# Safety guarantees (same class as diagnose_amdxdna.sh):
#   - Never opens /dev/accel/* (open can hang forever if wedged)
#   - Never insmod / rmmod / unbind / reboot
#   - Only reads sysfs, modinfo, modprobe.d, firmware links, and may invoke
#     diagnose_amdxdna.sh (also read-only)
#
# Exit codes:
#   0  looks hardened + healthy metadata (or MODULE_NOT_LOADED with hardened .ko present)
#   1  mismatch, wedge, or stock module where hardened expected
#   2  bad args
#
# Usage:
#   bash scripts/verify_amdxdna_hardened_host.sh
#   bash scripts/verify_amdxdna_hardened_host.sh --quiet
#   bash scripts/verify_amdxdna_hardened_host.sh --skip-diagnose
#   bash scripts/verify_amdxdna_hardened_host.sh --post-reboot   # same checks; labels output for reboot-proof checklist
#
# After kernel upgrade / reboot, re-run this script before any NPU bring-up.
# See docs/XDNA2_HARDENED_INSTALL.md § post-install / post-reboot / kernel upgrade.

set -euo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
DIAG="${ROOT}/scripts/diagnose_amdxdna.sh"

QUIET=0
SKIP_DIAG=0
POST_REBOOT=0
# Optional: expected substring in modinfo filename AND on-disk module path
# (default: updates/amdxdna-hardened under /lib/modules/$(uname -r)/)
EXPECT_PATH_SUB="${AMDXDNA_EXPECT_PATH_SUB:-updates/amdxdna-hardened}"
# Lab-known srcversion for 7.1.3-zen1-2-zen OOT L2+L3 (advisory if kernel differs)
LAB_SRCVERSION="${AMDXDNA_LAB_SRCVERSION:-E36F27FC09FB5CA0C0C3E87}"
# Outer timeout around diagnose_amdxdna.sh (seconds)
DIAG_TIMEOUT_S="${AMDXDNA_VERIFY_DIAG_TIMEOUT:-30}"
FW_DIR_CANDIDATES=(
  /lib/firmware/amdnpu/17f0_10
  /usr/lib/firmware/amdnpu/17f0_10
)

usage() {
  cat <<'EOF'
verify_amdxdna_hardened_host.sh — read-only hardened host check (Phase 0)

  bash scripts/verify_amdxdna_hardened_host.sh [options]

Options:
  --quiet            Reduce log noise. PASS: one summary line. FAIL: one FAIL
                     line plus concrete reasons on stderr (and next-steps).
  --skip-diagnose    Do not call diagnose_amdxdna.sh (still checks modinfo/sysfs)
  --post-reboot      Same checks as default; labels output for the reboot-proof
                     checklist (does not prove a reboot occurred; wrong-family
                     firmware aliases always hard-fail regardless of this flag)
  -h, --help         This help

Env:
  AMDXDNA_EXPECT_PATH_SUB       modinfo filename + on-disk path must contain this
                                (default: updates/amdxdna-hardened)
  AMDXDNA_LAB_SRCVERSION        expected srcversion when matching lab build (advisory)
  AMDXDNA_VERIFY_DIAG_TIMEOUT   seconds for outer diagnose wrap (default 30)
  AMDXDNA_PCI_ID                passed through to diagnose if used

Exit: 0 = hardened metadata OK; 1 = fail/wedge/mismatch; 2 = bad args

Does NOT open /dev/accel, load modules, or change system state.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --quiet) QUIET=1; shift ;;
    --skip-diagnose) SKIP_DIAG=1; shift ;;
    --post-reboot) POST_REBOOT=1; shift ;;
    -h | --help) usage; exit 0 ;;
    *)
      echo "verify: unknown arg: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

pass=0
fail=0
warn=0
FAIL_REASONS=()
WARN_REASONS=()

log() {
  [[ $QUIET -eq 1 ]] && return 0
  printf '%s\n' "$*"
}

# Always emit to stderr (used for quiet-mode fail reasons / diagnose hints).
err() {
  printf '%s\n' "$*" >&2
}

ok() {
  pass=$((pass + 1))
  log "  [OK]   $*"
}

bad() {
  fail=$((fail + 1))
  FAIL_REASONS+=("$*")
  log "  [FAIL] $*"
}

soft() {
  warn=$((warn + 1))
  WARN_REASONS+=("$*")
  log "  [WARN] $*"
}

# Active (non-comment) lines of a conf file for policy matching.
active_conf_lines() {
  local f=$1
  # Drop full-line and trailing comments; keep non-empty remainder.
  sed 's/#.*//' "$f" 2>/dev/null | sed '/^[[:space:]]*$/d' || true
}

KVER=$(uname -r 2>/dev/null || echo unknown)

log "amdxdna hardened host verification (read-only)"
log "safety: no /dev/accel open, no modprobe/rmmod/insmod"
log "time: $(date -Is 2>/dev/null || date)"
log "host: $(hostname 2>/dev/null || echo unknown)"
log "kernel: $KVER"
if [[ $POST_REBOOT -eq 1 ]]; then
  log "mode: POST-REBOOT proof run"
fi
log ""

# --- 1) Module identity via modinfo (works even if not loaded) ---------------
log "== module identity (modinfo) =="
MI_OUT=$(modinfo amdxdna 2>/dev/null || true)
if [[ -z "$MI_OUT" ]]; then
  bad "modinfo amdxdna failed (module not installed for this kernel?)"
  FILENAME=""
  SRCVERSION=""
  HAS_EXEC_PARM=0
else
  FILENAME=$(printf '%s\n' "$MI_OUT" | awk -F: '/^filename:/{sub(/^[[:space:]]+/,"",$2); print $2; exit}')
  SRCVERSION=$(printf '%s\n' "$MI_OUT" | awk -F: '/^srcversion:/{sub(/^[[:space:]]+/,"",$2); print $2; exit}')
  if printf '%s\n' "$MI_OUT" | grep -q '^parm:[[:space:]]*exec_cmd_allow:'; then
    HAS_EXEC_PARM=1
  else
    HAS_EXEC_PARM=0
  fi

  if [[ -n "$FILENAME" && "$FILENAME" == *"$EXPECT_PATH_SUB"* ]]; then
    ok "filename under hardened path: $FILENAME"
  elif [[ -n "$FILENAME" ]]; then
    bad "filename not under '$EXPECT_PATH_SUB': $FILENAME (stock/in-tree?)"
  else
    bad "modinfo produced no filename"
  fi

  if [[ $HAS_EXEC_PARM -eq 1 ]]; then
    ok "parm exec_cmd_allow present (L3 signature)"
  else
    bad "parm exec_cmd_allow missing — not an L3-hardened module"
  fi

  if [[ -n "$SRCVERSION" ]]; then
    if [[ "$SRCVERSION" == "$LAB_SRCVERSION" ]]; then
      ok "srcversion matches lab build: $SRCVERSION"
    else
      soft "srcversion=$SRCVERSION (lab reference was $LAB_SRCVERSION; OK if you rebuilt for a new kernel)"
    fi
  else
    soft "srcversion unavailable"
  fi
fi

# On-disk module: same EXPECT_PATH_SUB under /lib/modules/$KVER/; accept .ko / .ko.zst
# (aligned with modinfo filename expectation — override via AMDXDNA_EXPECT_PATH_SUB).
log ""
log "== on-disk module =="
DISK_FOUND=""
for cand in \
  "/lib/modules/${KVER}/${EXPECT_PATH_SUB}/amdxdna.ko" \
  "/lib/modules/${KVER}/${EXPECT_PATH_SUB}/amdxdna.ko.zst"; do
  if [[ -f "$cand" ]]; then
    DISK_FOUND=$cand
    break
  fi
done
if [[ -n "$DISK_FOUND" ]]; then
  ok "on-disk: $DISK_FOUND"
else
  bad "missing on-disk module under /lib/modules/${KVER}/${EXPECT_PATH_SUB}/amdxdna.ko[.zst] (rebuild after kernel upgrade — see HARDENED_INSTALL)"
fi

# --- 2) Runtime params if loaded ---------------------------------------------
log ""
log "== runtime (if loaded) =="
MODULE_LOADED=0
if [[ -d /sys/module/amdxdna ]]; then
  MODULE_LOADED=1
  REFCNT=$(cat /sys/module/amdxdna/refcnt 2>/dev/null || echo '?')
  LIVE_SRC=$(cat /sys/module/amdxdna/srcversion 2>/dev/null || echo '?')
  EXEC_VAL=$(cat /sys/module/amdxdna/parameters/exec_cmd_allow 2>/dev/null || echo 'MISSING')
  ok "module loaded refcnt=$REFCNT srcversion=$LIVE_SRC"
  case "$EXEC_VAL" in
    N | 0 | n)
      ok "exec_cmd_allow=$EXEC_VAL (lab default refuse EXEC_CMD)"
      ;;
    Y | 1 | y)
      bad "exec_cmd_allow=$EXEC_VAL — EXEC_CMD open in kernel; lab expects 0/N"
      ;;
    MISSING)
      bad "exec_cmd_allow sysfs param missing while module loaded"
      ;;
    *)
      soft "exec_cmd_allow=$EXEC_VAL (unexpected token; expect N/0 or Y/1)"
      ;;
  esac
else
  soft "module not loaded — identity checks still apply; load only after diagnose is clean"
fi

# --- 3) modprobe.d — options only, no softdep gpu_sched ----------------------
log ""
log "== modprobe.d =="
CONF_HITS=0
SOFTDEP_BAD=0
EXEC_OPT=0
shopt -s nullglob
# Deduplicate paths (both *amdxdna* and *xdna* may match the same file)
declare -A SEEN_CONF=()
for f in /etc/modprobe.d/*amdxdna* /etc/modprobe.d/*xdna*; do
  [[ -f "$f" ]] || continue
  [[ -n ${SEEN_CONF[$f]+x} ]] && continue
  SEEN_CONF[$f]=1
  CONF_HITS=$((CONF_HITS + 1))
  ACTIVE=$(active_conf_lines "$f")

  # Softdep: active lines only (commented anti-pattern docs must not false-fail)
  if printf '%s\n' "$ACTIVE" | grep -qiE '^[[:space:]]*softdep[[:space:]].*(gpu_sched|amdxdna)'; then
    SOFTDEP_BAD=1
    bad "$f contains active softdep involving amdxdna/gpu_sched (breaks modprobe -r while amdgpu holds gpu_sched)"
  fi

  # Explicit exec_cmd_allow enable → fail-closed (even if module not loaded yet)
  if printf '%s\n' "$ACTIVE" | grep -qE 'options[[:space:]]+amdxdna[[:space:]].*exec_cmd_allow[[:space:]]*=[[:space:]]*[1Yy]'; then
    bad "$f sets options amdxdna exec_cmd_allow=1 (lab expects 0; would open EXEC_CMD after modprobe)"
  elif printf '%s\n' "$ACTIVE" | grep -qE 'options[[:space:]]+amdxdna[[:space:]].*exec_cmd_allow[[:space:]]*=[[:space:]]*0'; then
    EXEC_OPT=1
    ok "$f sets options amdxdna exec_cmd_allow=0"
  elif printf '%s\n' "$ACTIVE" | grep -qE 'options[[:space:]]+amdxdna[[:space:]].*exec_cmd_allow[[:space:]]*='; then
    bad "$f sets options amdxdna exec_cmd_allow to a non-zero value (lab expects 0)"
  else
    soft "$f present but no clear 'options amdxdna exec_cmd_allow=0' line"
  fi
done
shopt -u nullglob

if [[ $CONF_HITS -eq 0 ]]; then
  soft "no /etc/modprobe.d/*amdxdna* conf — lab recommends options-only exec_cmd_allow=0 (no softdep)"
elif [[ $SOFTDEP_BAD -eq 0 && $EXEC_OPT -eq 1 ]]; then
  ok "modprobe.d: options-only lab default, no gpu_sched softdep"
fi

# --- 4) Firmware alias npu.dev.sbin → npu_7 ----------------------------------
log ""
log "== firmware alias (OOT requests npu.dev.sbin) =="
FW_OK=0
FW_FAMILY_OK=0
declare -A SEEN_FW_DIR=()
declare -A SEEN_FW_ALIAS=()
for d in "${FW_DIR_CANDIDATES[@]}"; do
  [[ -d "$d" ]] || continue
  # Deduplicate /lib vs /usr/lib (same inode on Arch/Garuda)
  real_d=$(readlink -f "$d" 2>/dev/null || echo "$d")
  [[ -n ${SEEN_FW_DIR[$real_d]+x} ]] && continue
  SEEN_FW_DIR[$real_d]=1

  # Prefer .zst names as on Arch linux-firmware
  for name in npu.dev.sbin.zst npu.dev.sbin; do
    p="$d/$name"
    if [[ -L "$p" || -e "$p" ]]; then
      real_p=$(readlink -f "$p" 2>/dev/null || echo "$p")
      # Dedupe alias entries that resolve to the same file via both dir candidates
      alias_key="${name}:${real_p}"
      [[ -n ${SEEN_FW_ALIAS[$alias_key]+x} ]] && continue
      SEEN_FW_ALIAS[$alias_key]=1

      if [[ -L "$p" ]]; then
        tgt=$(readlink -f "$p" 2>/dev/null || readlink "$p" 2>/dev/null || echo '?')
        link=$(readlink "$p" 2>/dev/null || echo '?')
        # Family match: link target name OR resolved path (covers npu_7 → 1.1.2.64 chain)
        family_hit=0
        case "$link" in
          *npu_7* | *1.1.2.64*) family_hit=1 ;;
        esac
        case "$tgt" in
          *npu_7* | *1.1.2.64*) family_hit=1 ;;
        esac
        if [[ $family_hit -eq 1 ]]; then
          ok "alias $p -> $link (resolved $tgt)"
          FW_OK=1
          FW_FAMILY_OK=1
        else
          # Wrong family: always hard-fail (lab refuse; independent of --post-reboot)
          bad "alias $p -> $link (resolved $tgt) not npu_7 / 1.1.2.64 family"
          FW_OK=1
        fi
      else
        soft "$p exists but is not a symlink (OOT still may load)"
        FW_OK=1
      fi
    fi
  done
  if [[ -e "$d/npu_7.sbin.zst" || -e "$d/npu_7.sbin" ]]; then
    ok "stock npu_7 present under $real_d"
  fi
done
if [[ $FW_OK -eq 0 ]]; then
  bad "npu.dev.sbin(.zst) alias not found under amdnpu/17f0_10 — OOT firmware load may fail"
elif [[ $FW_FAMILY_OK -eq 0 && $POST_REBOOT -eq 1 ]]; then
  # Label-only extra: non-symlink alias soft-path already warned above; wrong-family already bad().
  soft "post-reboot label: no verified npu_7 / 1.1.2.64 family alias (see FAIL/WARN lines if any)"
fi

# --- 5) Diagnose summary (optional) ------------------------------------------
DIAG_VERDICT="SKIPPED"
if [[ $SKIP_DIAG -eq 1 ]]; then
  soft "diagnose skipped (--skip-diagnose)"
elif [[ ! -x "$DIAG" && ! -f "$DIAG" ]]; then
  soft "diagnose script missing: $DIAG"
else
  log ""
  log "== diagnose_amdxdna.sh (read-only) =="
  DIAG_RC=0
  DIAG_OUT=""
  DIAG_ERR=""
  DIAG_TMP=$(mktemp 2>/dev/null || echo "")
  DIAG_ERR_TMP=$(mktemp 2>/dev/null || echo "")
  if [[ -n "$DIAG_TMP" && -n "$DIAG_ERR_TMP" ]]; then
    if command -v timeout >/dev/null 2>&1; then
      set +e
      timeout "${DIAG_TIMEOUT_S}s" bash "$DIAG" >"$DIAG_TMP" 2>"$DIAG_ERR_TMP"
      DIAG_RC=$?
      set -e
    else
      set +e
      bash "$DIAG" >"$DIAG_TMP" 2>"$DIAG_ERR_TMP"
      DIAG_RC=$?
      set -e
    fi
    DIAG_OUT=$(cat "$DIAG_TMP" 2>/dev/null || true)
    DIAG_ERR=$(cat "$DIAG_ERR_TMP" 2>/dev/null || true)
    rm -f "$DIAG_TMP" "$DIAG_ERR_TMP"
  else
    # Fallback without mktemp
    if command -v timeout >/dev/null 2>&1; then
      set +e
      DIAG_OUT=$(timeout "${DIAG_TIMEOUT_S}s" bash "$DIAG" 2>/dev/null)
      DIAG_RC=$?
      set -e
    else
      set +e
      DIAG_OUT=$(bash "$DIAG" 2>/dev/null)
      DIAG_RC=$?
      set -e
    fi
  fi

  if [[ $DIAG_RC -eq 124 ]]; then
    soft "diagnose timed out after ${DIAG_TIMEOUT_S}s — re-run: bash scripts/diagnose_amdxdna.sh"
    DIAG_VERDICT="TIMEOUT"
  else
    DIAG_VERDICT=$(printf '%s\n' "$DIAG_OUT" | awk -F: '/^verdict:/{sub(/^[[:space:]]+/,"",$2); print $2; exit}')
    REASON=$(printf '%s\n' "$DIAG_OUT" | awk -F: '/^reason:/{sub(/^[[:space:]]+/,"",$2); print $2; exit}')
    log "  diagnose verdict: ${DIAG_VERDICT:-unknown}"
    log "  diagnose reason:  ${REASON:-}"
    if [[ -z "${DIAG_VERDICT:-}" ]]; then
      soft "diagnose produced no verdict (rc=$DIAG_RC) — read full: bash scripts/diagnose_amdxdna.sh"
      if [[ -n "$DIAG_ERR" ]]; then
        # Short hint only; do not dump full stderr
        err "verify: diagnose stderr hint: $(printf '%s' "$DIAG_ERR" | head -c 200 | tr '\n' ' ')"
      fi
    fi
    case "$DIAG_VERDICT" in
      LIKELY_OK_METADATA)
        ok "diagnose LIKELY_OK_METADATA"
        ;;
      MODULE_NOT_LOADED)
        soft "diagnose MODULE_NOT_LOADED — hardened .ko may still be installable"
        ;;
      WEDGED_OPEN | WEDGED_OTHER)
        bad "diagnose $DIAG_VERDICT — do not open /dev/accel; reboot/recover first"
        ;;
      SUSPICIOUS_REFCNT)
        soft "diagnose SUSPICIOUS_REFCNT — prefer clean boot before experiments"
        ;;
      TIMEOUT)
        : # already soft above
        ;;
      "")
        : # already soft above
        ;;
      *)
        soft "diagnose verdict=${DIAG_VERDICT:-empty} — read full: bash scripts/diagnose_amdxdna.sh"
        ;;
    esac
  fi
fi

# --- Summary -----------------------------------------------------------------
log ""
log "== summary =="
log "checks_ok=$pass warnings=$warn failures=$fail diagnose=$DIAG_VERDICT"
if [[ $fail -gt 0 ]]; then
  log "RESULT: FAIL — host does not match hardened lab expectations"
  for r in "${FAIL_REASONS[@]}"; do
    log "  - $r"
  done
  if [[ $QUIET -eq 1 ]]; then
    # Quiet success is one line; quiet FAIL always prints concrete reasons on stderr.
    err "FAIL fail=$fail warn=$warn diag=$DIAG_VERDICT"
    for r in "${FAIL_REASONS[@]}"; do
      err "  - $r"
    done
  fi
  cat <<'EOF'

Next steps:
  1. If WEDGED_*: reboot (or recovery docs); do not thrash /dev/accel
  2. If stock module: reinstall from docs/XDNA2_HARDENED_INSTALL.md (kernel upgrade section)
  3. Re-run: bash scripts/verify_amdxdna_hardened_host.sh
  4. Safe smoke only when diagnose not WEDGED_*:
       timeout 15 cargo run --features amd-xdna2 --example npu_direct_bench -- --checklist
EOF
  exit 1
fi

if [[ $POST_REBOOT -eq 1 ]]; then
  log "RESULT: PASS (post-reboot metadata) — mark NEVER_AGAIN reboot proof only if this matches after real reboot"
else
  log "RESULT: PASS — hardened identity + lab gates look good (metadata only)"
fi
for r in "${WARN_REASONS[@]}"; do
  log "  warn: $r"
done
[[ $QUIET -eq 1 ]] && printf 'PASS fail=0 warn=%s diag=%s\n' "$warn" "$DIAG_VERDICT"

log ""
log "Optional live smoke (opens /dev/accel — only if not WEDGED_*):"
log "  timeout 15 cargo run --features amd-xdna2 --example npu_direct_bench -- --checklist"
log "Do NOT enable xdna2-dangerous-exec-cmd / XDNA2_ALLOW_EXEC_CMD / exec_cmd_allow=1 without known-good payload."
exit 0
