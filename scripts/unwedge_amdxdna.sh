#!/usr/bin/env bash
# Recover a wedged amdxdna (stuck amdxdna_drm_open / high module refcnt)
# without a full system reboot, if the kernel will allow it.
#
# Usage:
#   sudo bash scripts/unwedge_amdxdna.sh
#
# Safe defaults: never opens /dev/accel. Stops if a step hangs (timeouts).
# Exit codes: 0 recovered, 1 failed (reboot still required), 2 bad usage

set -u

PCI_ID="${AMDXDNA_PCI_ID:-0000:c6:00.1}"
TIMEOUT_S="${AMDXDNA_UNWEDGE_TIMEOUT:-12}"

red() { printf '\033[31m%s\033[0m\n' "$*"; }
grn() { printf '\033[32m%s\033[0m\n' "$*"; }
ylw() { printf '\033[33m%s\033[0m\n' "$*"; }

need_root() {
  if [[ ${EUID:-$(id -u)} -ne 0 ]]; then
    red "Run as root: sudo bash $0"
    exit 2
  fi
}

run_to() {
  local label=$1
  shift
  ylw "==> $label"
  if timeout "$TIMEOUT_S" "$@"; then
    grn "    ok: $label"
    return 0
  else
    local ec=$?
    red "    FAIL/TIMEOUT (exit $ec): $label"
    return "$ec"
  fi
}

show_state() {
  echo "--- state ---"
  lsmod | grep -E 'amdxdna|amd_pmf|gpu_sched' || true
  if [[ -r /sys/module/amdxdna/refcnt ]]; then
    echo "refcnt=$(cat /sys/module/amdxdna/refcnt)"
  else
    echo "refcnt=(module not loaded)"
  fi
  echo "D-state amdxdna waiters:"
  ps -eo pid,stat,wchan:28,cmd | awk '$2 ~ /D/ && /amdxdna/' || echo "  (none)"
  if [[ -e /sys/bus/pci/devices/$PCI_ID ]]; then
    echo "pci $PCI_ID driver: $(readlink -f /sys/bus/pci/devices/$PCI_ID/driver 2>/dev/null || echo unbound)"
  else
    echo "pci $PCI_ID: not present"
  fi
  echo "-------------"
}

kill_userspace_waiters() {
  ylw "==> kill -9 D-state waiters (usually no-ops, try anyway)"
  local pids
  pids=$(ps -eo pid,stat,wchan,cmd | awk '$2 ~ /D/ && /amdxdna/ {print $1}')
  if [[ -z "${pids:-}" ]]; then
    echo "    none"
    return 0
  fi
  # shellcheck disable=SC2086
  kill -9 $pids 2>/dev/null || true
  sleep 0.5
}

pci_unbind() {
  local path=/sys/bus/pci/drivers/amdxdna/unbind
  if [[ ! -e /sys/bus/pci/devices/$PCI_ID/driver ]]; then
    ylw "==> already unbound from driver"
    return 0
  fi
  if [[ ! -w $path ]]; then
    red "cannot write $path"
    return 1
  fi
  run_to "pci unbind $PCI_ID" bash -c "echo $PCI_ID > $path"
}

pci_remove() {
  local path=/sys/bus/pci/devices/$PCI_ID/remove
  if [[ ! -e /sys/bus/pci/devices/$PCI_ID ]]; then
    ylw "==> PCI device already gone"
    return 0
  fi
  run_to "pci remove $PCI_ID" bash -c "echo 1 > $path"
}

pci_rescan() {
  run_to "pci rescan" bash -c "echo 1 > /sys/bus/pci/rescan"
}

try_rmmod() {
  # Drop soft soft dep first when possible (amdxdna uses amd_pmf)
  run_to "modprobe -r amd_pmf" modprobe -r amd_pmf || true
  if run_to "modprobe -r amdxdna" modprobe -r amdxdna; then
    return 0
  fi
  # Force only if still present and CONFIG_MODULE_FORCE_UNLOAD
  if lsmod | grep -q '^amdxdna'; then
    ylw "==> trying rmmod -f amdxdna (risky)"
    if run_to "rmmod -f amdxdna" rmmod -f amdxdna; then
      return 0
    fi
  fi
  return 1
}

reload() {
  run_to "modprobe amdxdna" modprobe amdxdna || return 1
  # device may need bind after remove/rescan
  if [[ -e /sys/bus/pci/devices/$PCI_ID ]] && [[ ! -e /sys/bus/pci/devices/$PCI_ID/driver ]]; then
    if [[ -w /sys/bus/pci/drivers/amdxdna/bind ]]; then
      run_to "pci bind $PCI_ID" bash -c "echo $PCI_ID > /sys/bus/pci/drivers/amdxdna/bind" || true
    fi
  fi
  return 0
}

smoke_open() {
  # Optional; only after reload. Short timeout. Do not use for recovery itself.
  if [[ ! -e /dev/accel/accel0 ]]; then
    ylw "no /dev/accel/accel0 yet"
    return 1
  fi
  run_to "timed open /dev/accel/accel0" \
    bash -c 'exec 3<>/dev/accel/accel0 && echo OPEN_OK >&2'
}

main() {
  need_root
  echo "PCI_ID=$PCI_ID TIMEOUT_S=${TIMEOUT_S}s"
  show_state

  kill_userspace_waiters
  show_state

  # Prefer remove over unbind when open() is stuck: remove often tears down
  # the DRM minor harder. Timeouts prevent infinite hang.
  if ! pci_unbind; then
    ylw "unbind failed/hung — trying PCI remove"
  fi
  show_state

  if lsmod | grep -q '^amdxdna'; then
    pci_remove || true
    show_state
  fi

  if try_rmmod; then
    grn "module unloaded"
  else
    red "module still loaded after unbind/remove/rmmod"
    show_state
    red "Non-reboot recovery FAILED. Reboot is required."
    exit 1
  fi

  pci_rescan || true
  if reload; then
    grn "module reloaded"
  else
    red "reload failed"
    show_state
    exit 1
  fi

  show_state
  if smoke_open; then
    grn "RECOVERED: open() works again"
    exit 0
  fi

  # Module reloaded but open still broken → still need reboot
  if ps -eo stat,wchan,cmd | awk '$1 ~ /D/ && /amdxdna/ {found=1} END {exit !found}'; then
    red "RELOAD happened but open still wedges (D-state). Reboot required."
    exit 1
  fi
  ylw "Module cycle done; open smoke inconclusive. Check manually with short timeout."
  exit 0
}

main "$@"
