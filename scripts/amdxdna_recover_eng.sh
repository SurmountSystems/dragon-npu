#!/usr/bin/env bash
# Engineering recovery for wedged amdxdna — NOT a polite reboot wrapper.
#
# Goal: clear D-state amdxdna_drm_open waiters + drop module refs without
# rebooting the whole machine, using PCI secondary bus reset + optional
# rescue kmod. Prefer this over rmmod -f.
#
# Requires: root (sudo). Does NOT open /dev/accel from the happy path.
#
# Usage:
#   sudo bash scripts/amdxdna_recover_eng.sh diagnose
#   sudo bash scripts/amdxdna_recover_eng.sh recover
#   sudo bash scripts/amdxdna_recover_eng.sh verify
#
set -u

ROOT=$(cd "$(dirname "$0")/.." && pwd)
RESCUE_DIR="$ROOT/kernel/amdxdna_rescue"
NPU_PCI="${AMDXDNA_PCI_ID:-0000:c6:00.1}"
BRIDGE_PCI="${AMDXDNA_BRIDGE_PCI:-0000:00:08.2}"
TIMEOUT_S="${AMDXDNA_RECOVER_TIMEOUT:-15}"

red() { printf '\033[31m%s\033[0m\n' "$*"; }
grn() { printf '\033[32m%s\033[0m\n' "$*"; }
ylw() { printf '\033[33m%s\033[0m\n' "$*"; }

need_root() {
  if [[ ${EUID:-$(id -u)} -ne 0 ]]; then
    red "Need root: sudo bash $0 $*"
    exit 2
  fi
}

run_to() {
  local label=$1
  shift
  ylw "==> $label"
  if timeout "$TIMEOUT_S" "$@"; then
    grn "    ok"
    return 0
  fi
  red "    FAIL/TIMEOUT: $label"
  return 1
}

diag_userspace() {
  echo "--- userspace snapshot ---"
  lsmod | grep -E 'amdxdna|amd_pmf|gpu_sched' || true
  echo "refcnt=$(cat /sys/module/amdxdna/refcnt 2>/dev/null || echo n/a)"
  echo "D-state amdxdna waiters:"
  ps -eo pid,stat,wchan:40,etime,cmd | awk '$2 ~ /D/ && ($3 ~ /amdxdna/ || /amdxdna/)'
  echo "PCI NPU driver: $(readlink -f /sys/bus/pci/devices/$NPU_PCI/driver 2>/dev/null || echo none)"
  echo "bridge reset node: $(ls -la /sys/bus/pci/devices/$BRIDGE_PCI/reset 2>&1)"
  echo "npu reset node: $(ls -la /sys/bus/pci/devices/$NPU_PCI/reset 2>&1 || true)"
}

build_rescue() {
  ylw "==> build rescue kmod"
  make -C "$RESCUE_DIR" clean all
}

kmod_action() {
  local act=$1
  local ko="$RESCUE_DIR/amdxdna_rescue.ko"
  [[ -f $ko ]] || build_rescue
  # unload previous if present
  rmmod amdxdna_rescue 2>/dev/null || true
  ylw "==> insmod action=$act"
  if ! insmod "$ko" action="$act"; then
    red "insmod failed (see dmesg); module may not build on this kernel"
    dmesg | tail -30
    return 1
  fi
  sleep 0.5
  dmesg | tail -60
  rmmod amdxdna_rescue 2>/dev/null || true
}

# Secondary bus reset via sysfs (no kmod required) — primary lever on this HW.
sysfs_bridge_reset() {
  local path=/sys/bus/pci/devices/$BRIDGE_PCI/reset
  if [[ ! -e $path ]]; then
    red "no $path — cannot sysfs bridge reset"
    return 1
  fi
  run_to "echo 1 > $path (GPP bridge secondary reset)" \
    bash -c "echo 1 > '$path'"
}

sysfs_remove_npu() {
  local path=/sys/bus/pci/devices/$NPU_PCI/remove
  if [[ ! -e /sys/bus/pci/devices/$NPU_PCI ]]; then
    ylw "NPU already absent from bus"
    return 0
  fi
  run_to "remove NPU $NPU_PCI" bash -c "echo 1 > '$path'"
}

sysfs_rescan() {
  run_to "pci rescan" bash -c "echo 1 > /sys/bus/pci/rescan"
}

try_unload_load() {
  modprobe -r amd_pmf 2>/dev/null || true
  if run_to "modprobe -r amdxdna" modprobe -r amdxdna; then
    grn "amdxdna unloaded"
  else
    red "still cannot unload amdxdna (refcnt=$(cat /sys/module/amdxdna/refcnt 2>/dev/null))"
    return 1
  fi
  run_to "modprobe amdxdna" modprobe amdxdna
}

# Timed open — only after we believe recovery worked
verify_open() {
  if [[ ! -e /dev/accel/accel0 ]]; then
    red "no /dev/accel/accel0"
    return 1
  fi
  run_to "timed open /dev/accel/accel0" \
    bash -c 'exec 3<>/dev/accel/accel0 && echo OPEN_OK'
}

cmd=${1:-diagnose}

need_root "$cmd"
case "$cmd" in
  diagnose)
    diag_userspace
    if [[ -d /lib/modules/$(uname -r)/build ]]; then
      build_rescue && kmod_action diagnose || ylw "kmod diagnose skipped/failed"
    else
      ylw "no kernel build tree; sysfs-only diagnose"
    fi
    ;;
  recover)
    diag_userspace
    echo
    ylw "=== PHASE 1: diagnose kmod ==="
    build_rescue && kmod_action diagnose || true
    echo
    ylw "=== PHASE 2: GPP bridge secondary bus reset ($BRIDGE_PCI) ==="
    ylw "This resets the bus that holds NPU 17f0 + dummy 1116 (not the big GPU)."
    sysfs_bridge_reset || true
    sleep 1
    diag_userspace
    echo
    ylw "=== PHASE 3: kmod bus reset + wake (if still needed) ==="
    if ps -eo stat,wchan | awk '$1 ~ /D/ && /amdxdna/ {found=1} END {exit !found}'; then
      kmod_action full || kmod_action reset_bus || true
      sleep 1
    fi
    echo
    ylw "=== PHASE 4: remove NPU + rescan + reload module ==="
    sysfs_remove_npu || true
    sleep 1
    # If module still busy, try unload anyway after device gone
    try_unload_load || true
    sysfs_rescan || true
    sleep 1
    modprobe amdxdna 2>/dev/null || true
    echo
    ylw "=== PHASE 5: verify ==="
    diag_userspace
    if verify_open; then
      grn "RECOVERED without full system reboot"
      exit 0
    fi
    red "Recovery incomplete — D-state or open still broken."
    red "Remaining option: reboot (or continue forking driver with open-timeout)."
    exit 1
    ;;
  verify)
    diag_userspace
    verify_open
    ;;
  *)
    echo "Usage: $0 diagnose|recover|verify"
    exit 2
    ;;
esac
