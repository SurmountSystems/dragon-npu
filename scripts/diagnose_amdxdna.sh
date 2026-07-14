#!/usr/bin/env bash
# Read-only diagnosis of amdxdna / NPU accel state (safe while wedged).
#
# Companion to unwedge_amdxdna.sh (recovery — that one needs root and can change
# kernel state). THIS script is intentionally passive.
#
# Safety guarantees:
#   - Never opens /dev/accel/* (open can hang forever if driver is wedged)
#   - Never unbinds/removes PCI, never modprobe -r / rmmod / kill
#   - Only reads sysfs/proc/lsmod/journal (and optional garuda-inxi)
#   - External tools wrapped in timeout so a stuck utility cannot freeze the run
#   - Safe as non-root; --with-root only adds read-only stacks/journal depth
#
# Usage:
#   bash scripts/diagnose_amdxdna.sh
#   bash scripts/diagnose_amdxdna.sh --out /tmp/amdxdna-diag.txt
#   bash scripts/diagnose_amdxdna.sh --inxi          # include garuda-inxi / inxi
#   sudo bash scripts/diagnose_amdxdna.sh --with-root --out /tmp/amdxdna-diag.txt
#
# Clipboard (all report text is on stdout):
#   fish:  bash scripts/diagnose_amdxdna.sh | pbcopy
#          (use &| only if you also want stderr; this script keeps noise on stdout)
#   bash:  bash scripts/diagnose_amdxdna.sh | pbcopy   # or xclip -selection clipboard
#
# Exit: 0 always for a completed report; 2 on bad args.

set -u

OUT=""
WITH_ROOT=0
DO_INXI=0
PCI_ID="${AMDXDNA_PCI_ID:-}"
TIMEOUT_S="${AMDXDNA_DIAG_TIMEOUT:-8}"

usage() {
  cat <<'EOF'
Read-only amdxdna / NPU diagnosis (does not open /dev/accel, no module unload).

  bash scripts/diagnose_amdxdna.sh [options]

Options:
  --out PATH     Also write full report to PATH (stdout still gets the report)
  --with-root    Read /proc/PID/stack + fuller journal (use with sudo)
  --inxi         Run garuda-inxi or inxi (timeout-capped; system inventory)
  --pci ID       PCI slot (default: from accel0 sysfs, else 0000:c6:00.1)
  -h, --help     This help

Fish shell + clipboard:
  bash scripts/diagnose_amdxdna.sh | pbcopy
  # |  = stdout only  (preferred; report is all on stdout)
  # &| = stdout+stderr (only if something leaks to stderr)

Safe to re-run any time. Prefer this over unwedge_amdxdna.sh for first diagnosis.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --out)
      OUT=${2:-}
      [[ -n "$OUT" ]] || { echo "diagnose: --out needs a path" >&2; exit 2; }
      shift 2
      ;;
    --with-root)
      WITH_ROOT=1
      shift
      ;;
    --inxi)
      DO_INXI=1
      shift
      ;;
    --pci)
      PCI_ID=${2:-}
      shift 2
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      echo "diagnose: unknown arg: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

# --- helpers -----------------------------------------------------------------

ts() { date -Is 2>/dev/null || date; }

section() {
  printf '\n======== %s ========\n' "$*"
}

# Run command with timeout; print stdout; never fail the script.
# Usage: try_cmd "label" cmd args...
try_cmd() {
  local label=$1
  shift
  echo "\$ $*"
  if command -v timeout >/dev/null 2>&1; then
    timeout "$TIMEOUT_S" "$@" 2>&1 || echo "(exit $?; timed out or failed after ${TIMEOUT_S}s: $label)"
  else
    "$@" 2>&1 || echo "(exit $?; failed: $label)"
  fi
}

read_file() {
  local f=$1
  if [[ -r $f ]]; then
    cat "$f" 2>/dev/null
  else
    echo "(unreadable: $f)"
  fi
}

# Resolve PCI from sysfs (readlink only — no device open)
if [[ -z "$PCI_ID" ]]; then
  if [[ -e /sys/class/accel/accel0/device ]]; then
    PCI_ID=$(basename "$(readlink -f /sys/class/accel/accel0/device 2>/dev/null || true)")
  fi
fi
PCI_ID=${PCI_ID:-0000:c6:00.1}

IS_ROOT=0
[[ $(id -u) -eq 0 ]] && IS_ROOT=1
[[ $WITH_ROOT -eq 1 && $IS_ROOT -eq 0 ]] && WITH_ROOT=0 # honor flag only if actually root

# --- collect signals for summary (before printing) ---------------------------

REFCNT="n/a"
MODULE_LOADED=0
[[ -d /sys/module/amdxdna ]] && MODULE_LOADED=1
[[ $MODULE_LOADED -eq 1 && -r /sys/module/amdxdna/refcnt ]] && REFCNT=$(cat /sys/module/amdxdna/refcnt 2>/dev/null || echo "?")

# Note: wchan is a separate column; cmd may be "npu_direct_bench" without "amdxdna".
# Match wchan field OR whole line so we catch amdxdna_drm_open waiters.
D_WAITERS=$(ps -eo pid,stat,wchan:40,cmd 2>/dev/null \
  | awk 'NR>1 && $2 ~ /D/ && ($3 ~ /amdxdna/ || $0 ~ /amdxdna/) {c++} END {print c+0}')
D_OPEN=$(ps -eo pid,stat,wchan:40,cmd 2>/dev/null \
  | awk 'NR>1 && $2 ~ /D/ && $3 ~ /amdxdna_drm_open/ {c++} END {print c+0}')
Z_NPU=$(ps -eo stat,cmd 2>/dev/null | awk 'NR>1 && $1 ~ /Z/ && /dragon|npu|xdna/ {c++} END {print c+0}')

ACCEL_NODE=0
[[ -e /dev/accel/accel0 ]] && ACCEL_NODE=1
PCI_PRESENT=0
[[ -d /sys/bus/pci/devices/$PCI_ID ]] && PCI_PRESENT=1
DRIVER_BOUND="none"
if [[ $PCI_PRESENT -eq 1 ]]; then
  if [[ -e /sys/bus/pci/devices/$PCI_ID/driver ]]; then
    DRIVER_BOUND=$(basename "$(readlink -f /sys/bus/pci/devices/$PCI_ID/driver 2>/dev/null || echo unknown)")
  else
    DRIVER_BOUND="unbound"
  fi
fi

# Best-effort oops sniff (may be empty without journal perms)
OOPS_SNIP=""
if command -v journalctl >/dev/null 2>&1; then
  OOPS_SNIP=$(timeout 5 journalctl -k -n 300 --no-pager 2>/dev/null \
    | grep -iE 'BUG:|NULL pointer|aie2_cmdlist|amdxdna.*Oops|drm_mm_takedown' \
    | tail -3 | tr '\n' ' | ' || true)
fi

# Verdict
VERDICT="UNKNOWN"
REASON=""
if [[ $MODULE_LOADED -eq 0 ]]; then
  VERDICT="MODULE_NOT_LOADED"
  REASON="amdxdna not in kernel; NPU driver absent"
elif [[ $D_OPEN -gt 0 ]]; then
  VERDICT="WEDGED_OPEN"
  REASON="$D_OPEN process(es) stuck in amdxdna_drm_open (D-state); module refcnt=$REFCNT — do not open /dev/accel; reboot or careful recovery"
elif [[ $D_WAITERS -gt 0 ]]; then
  VERDICT="WEDGED_OTHER"
  REASON="$D_WAITERS D-state task(s) waiting in amdxdna paths; refcnt=$REFCNT"
elif [[ "$REFCNT" =~ ^[0-9]+$ ]] && [[ $REFCNT -gt 5 ]]; then
  VERDICT="SUSPICIOUS_REFCNT"
  REASON="refcnt=$REFCNT with no D-state waiters listed (stale clients or unlisted holders)"
elif [[ $PCI_PRESENT -eq 1 && $DRIVER_BOUND != amdxdna && $DRIVER_BOUND != none ]]; then
  VERDICT="DRIVER_MISMATCH"
  REASON="PCI $PCI_ID bound to $DRIVER_BOUND, not amdxdna"
elif [[ $MODULE_LOADED -eq 1 && $ACCEL_NODE -eq 1 && $D_WAITERS -eq 0 ]]; then
  VERDICT="LIKELY_OK_METADATA"
  REASON="module loaded, accel node exists, no D-state amdxdna waiters (metadata only — did not open device)"
else
  VERDICT="PARTIAL"
  REASON="see sections below"
fi

# --- report (all on stdout for | pbcopy) -------------------------------------

report() {
  echo "amdxdna READ-ONLY diagnosis"
  echo "safety: no /dev/accel open, no PCI unbind/remove, no rmmod/kill"
  echo "time: $(ts)"
  echo "host: $(hostname 2>/dev/null || echo unknown)"
  echo "user: $(id -un) uid=$(id -u) groups=$(id -Gn 2>/dev/null || true)"
  echo "kernel: $(uname -srm 2>/dev/null || uname -r)"
  echo "pci_id: $PCI_ID"
  echo "timeout_s: $TIMEOUT_S"
  echo "with_root: $WITH_ROOT (effective; need real root for stacks)"
  echo "inxi: $DO_INXI"

  section "EXECUTIVE SUMMARY"
  echo "verdict: $VERDICT"
  echo "reason:  $REASON"
  echo "module_loaded: $MODULE_LOADED"
  echo "refcnt: $REFCNT"
  echo "d_state_amdxdna_waiters: $D_WAITERS"
  echo "d_state_amdxdna_drm_open: $D_OPEN"
  echo "zombie_npu_related: $Z_NPU"
  echo "dev_accel_accel0_exists: $ACCEL_NODE"
  echo "pci_present: $PCI_PRESENT"
  echo "pci_driver: $DRIVER_BOUND"
  if [[ -n "$OOPS_SNIP" ]]; then
    echo "recent_oops_snips: $OOPS_SNIP"
  else
    echo "recent_oops_snips: (none found in last journal slice, or no access)"
  fi
  echo
  echo "next_steps:"
  case "$VERDICT" in
    WEDGED_OPEN | WEDGED_OTHER)
      echo "  1. Do NOT run npu_direct_bench / open /dev/accel"
      echo "  2. Prefer reboot to clear D-state + refcnt"
      echo "  3. Optional: scripts/unwedge_amdxdna.sh (root, riskier than reboot)"
      echo "  4. After recovery: checklist WITHOUT xdna2-dangerous-exec-cmd / XDNA2_ALLOW_EXEC_CMD"
      ;;
    LIKELY_OK_METADATA)
      echo "  1. Safe smoke: timeout 10 cargo run --features amd-xdna2 --example npu_direct_bench -- --checklist"
      echo "  2. Never enable EXEC_CMD (feature xdna2-dangerous-exec-cmd + XDNA2_ALLOW_EXEC_CMD=1) without a known-good payload"
      ;;
    MODULE_NOT_LOADED)
      echo "  1. sudo modprobe amdxdna"
      echo "  2. re-run this diagnose script"
      ;;
    *)
      echo "  1. Read sections below"
      echo "  2. Prefer reboot if open hangs; avoid force rmmod"
      ;;
  esac

  section "lsmod (amdxdna / friends)"
  try_cmd "lsmod" lsmod | grep -E 'amdxdna|amd_pmf|gpu_sched|amdgpu|amd_sfh' || echo "(no matching modules)"

  section "module sysfs"
  if [[ $MODULE_LOADED -eq 1 ]]; then
    echo "loaded: yes"
    echo -n "refcnt: "
    read_file /sys/module/amdxdna/refcnt
    echo -n "srcversion: "
    read_file /sys/module/amdxdna/srcversion
    echo "taint: $(cat /sys/module/amdxdna/taint 2>/dev/null || echo n/a)"
    echo "holders (other modules depending on amdxdna — often empty):"
    ls -la /sys/module/amdxdna/holders 2>/dev/null || true
    echo "parameters:"
    ls /sys/module/amdxdna/parameters 2>/dev/null | head -40 || true
  else
    echo "loaded: no"
  fi

  section "accel class (sysfs only)"
  ls -la /sys/class/accel/ 2>/dev/null || echo "(no /sys/class/accel)"
  if [[ -e /sys/class/accel/accel0 ]]; then
    echo "accel0 -> $(readlink -f /sys/class/accel/accel0 2>/dev/null || true)"
    echo "device -> $(readlink -f /sys/class/accel/accel0/device 2>/dev/null || true)"
    for f in name uevent power/runtime_status power/control; do
      p=/sys/class/accel/accel0/$f
      if [[ -r $p ]]; then
        printf '%s: %s\n' "$f" "$(tr '\n' ' ' <"$p" 2>/dev/null)"
      fi
    done
  fi

  section "device node metadata (NO open of /dev/accel)"
  ls -la /dev/accel/ 2>/dev/null || echo "(no /dev/accel)"
  if [[ $ACCEL_NODE -eq 1 ]]; then
    stat /dev/accel/accel0 2>/dev/null || true
    echo "note: skipped fuser/lsof on accel0 — they can hang on a wedged char device"
  fi

  section "PCI device $PCI_ID"
  dev=/sys/bus/pci/devices/$PCI_ID
  if [[ -d $dev ]]; then
    echo "present: yes"
    for f in vendor device class revision subsystem_vendor subsystem_device enable current_link_speed current_link_width; do
      if [[ -r $dev/$f ]]; then
        printf '%s: %s' "$f" "$(cat "$dev/$f" 2>/dev/null)"
        # ensure newline
        echo
      fi
    done
    echo "driver: $(readlink -f $dev/driver 2>/dev/null || echo unbound)"
    echo "iommu_group: $(readlink -f $dev/iommu_group 2>/dev/null || echo n/a)"
    [[ -r $dev/uevent ]] && {
      echo "uevent:"
      cat "$dev/uevent"
    }
    [[ -r $dev/modalias ]] && printf 'modalias: %s\n' "$(cat "$dev/modalias")"
  else
    echo "present: no (wrong id or device removed from bus)"
  fi

  section "PCI driver amdxdna bound devices"
  if [[ -d /sys/bus/pci/drivers/amdxdna ]]; then
    ls -la /sys/bus/pci/drivers/amdxdna/ 2>/dev/null | head -30
  else
    echo "(no /sys/bus/pci/drivers/amdxdna)"
  fi

  section "D-state waiters (kernel stuck paths)"
  echo "filter: stat=D and (wchan ~ amdxdna* OR line contains amdxdna)"
  ps -eo user,pid,ppid,stat,wchan:40,etime,cmd 2>/dev/null \
    | awk 'NR==1 || ($4 ~ /D/ && ($5 ~ /amdxdna/ || $0 ~ /amdxdna/))' || true

  section "related userspace / zombies"
  echo "dragon_npu / npu_direct / xdna /accel cmds:"
  ps -eo user,pid,stat,etime,cmd 2>/dev/null \
    | awk 'NR==1 || /dragon_npu|npu_direct|xdna2|amdxdna|\/dev\/accel/' || true
  echo
  echo "zombies (do not hold module refs by themselves):"
  ps -eo user,pid,stat,etime,cmd 2>/dev/null | awk 'NR==1 || ($3 ~ /Z/ && /dragon|npu|xdna/)' || true

  section "kernel worker threads (names)"
  ps -eo pid,stat,wchan:32,cmd 2>/dev/null | awk '/\[.*xdna|amdxdna|amdxdna_js/ {print}' || echo "(none)"

  section "kernel log (amdxdna / aie2 / oops)"
  if command -v journalctl >/dev/null 2>&1; then
    try_cmd "journalctl -k" journalctl -k -n 250 --no-pager \
      | grep -iE 'amdxdna|aie2|xdna|accel0|Oops|BUG:|NULL pointer|drm_mm_takedown|Call Trace|WARNING:.*drm' \
      | tail -100 || echo "(no matches)"
  elif [[ -r /var/log/kern.log ]]; then
    grep -iE 'amdxdna|aie2|xdna|Oops' /var/log/kern.log 2>/dev/null | tail -50 || true
  else
    echo "(no journalctl / kern.log)"
  fi

  if [[ $WITH_ROOT -eq 1 || $IS_ROOT -eq 1 ]]; then
    section "kernel stacks of D-state amdxdna waiters"
    pids=$(ps -eo pid,stat,wchan,cmd 2>/dev/null | awk '$2 ~ /D/ && /amdxdna/ {print $1}')
    if [[ -z "${pids:-}" ]]; then
      echo "(none)"
    else
      for p in $pids; do
        echo "--- pid $p ---"
        tr '\0' ' ' <"/proc/$p/cmdline" 2>/dev/null
        echo
        cat "/proc/$p/stack" 2>/dev/null || echo "(no stack)"
        echo
      done
    fi
  else
    section "kernel stacks"
    echo "skipped — re-run with: sudo bash $0 --with-root"
  fi

  if [[ $DO_INXI -eq 1 ]]; then
    section "system inventory (garuda-inxi / inxi)"
    if command -v garuda-inxi >/dev/null 2>&1; then
      # -Gxxxz: graphics-focused; -Fxxxz full is huge. Cap time.
      # Flags are read-only inventory.
      try_cmd "garuda-inxi" garuda-inxi -Gxxxz --no-host 2>/dev/null \
        || try_cmd "garuda-inxi fallback" garuda-inxi -G --no-host
    elif command -v inxi >/dev/null 2>&1; then
      try_cmd "inxi" inxi -Gxxxz --no-host
    else
      echo "(garuda-inxi / inxi not found)"
    fi
    echo "cpu/mem quick:"
    try_cmd "lscpu" lscpu | head -20 || true
    try_cmd "free" free -h || true
  else
    section "system inventory"
    echo "skipped (pass --inxi to run garuda-inxi/inxi; timeout ${TIMEOUT_S}s)"
  fi

  section "interpretation"
  cat <<'EOF'
lsmod column "Used by":
  amdxdna  SIZE  N          → N references (opens/ioctls), not "N modules"
  amd_pmf  … 1 amdxdna      → amdxdna depends on amd_pmf
  gpu_sched … 2 amdxdna,amdgpu → NPU + GPU share scheduler helper

D-state + wchan amdxdna_drm_open:
  Stuck in kernel open(); kill -9 usually useless; each open raised refcnt.

Do not "test" with: cat /dev/accel/accel0  or  untimed probe  while WEDGED_*.

Clipboard (fish):  bash scripts/diagnose_amdxdna.sh | pbcopy
EOF

  section "done"
  echo "finished: $(ts)"
  echo "verdict: $VERDICT"
}

# Capture full report once so --out and stdout stay identical; clean for pipes.
if [[ -n "$OUT" ]]; then
  report | tee "$OUT"
  # tee already printed to stdout; tiny note only if interactive-ish
else
  report
fi

exit 0
