#!/usr/bin/env bash
# Gather read-only host evidence for amd/xdna-driver issue/PR attachments.
# Does NOT open /dev/accel, insmod, rmmod, or post to GitHub.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="${ROOT}/docs/upstream/evidence"
mkdir -p "${OUT_DIR}"
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
OUT="${OUT_DIR}/HOST_SNAPSHOT_${STAMP}.txt"

{
  echo "=== amdxdna upstream evidence snapshot ==="
  echo "utc: ${STAMP}"
  echo "host: $(hostname 2>/dev/null || true)"
  echo "user: $(id -un 2>/dev/null || true)"
  echo

  echo "=== uname ==="
  uname -a
  echo

  echo "=== modinfo amdxdna ==="
  modinfo amdxdna 2>&1 || true
  echo

  echo "=== /sys/module/amdxdna (if loaded) ==="
  if [[ -d /sys/module/amdxdna ]]; then
    echo "refcnt: $(cat /sys/module/amdxdna/refcnt 2>/dev/null || echo '?')"
    echo "srcversion: $(cat /sys/module/amdxdna/srcversion 2>/dev/null || echo '?')"
    echo "taint: $(cat /sys/module/amdxdna/taint 2>/dev/null || echo '?')"
    echo "parameters:"
    for p in /sys/module/amdxdna/parameters/*; do
      [[ -e "$p" ]] || continue
      printf '  %s=%s\n' "$(basename "$p")" "$(cat "$p" 2>/dev/null || echo '?')"
    done
  else
    echo "(module not loaded)"
  fi
  echo

  echo "=== modprobe.d (amdxdna) ==="
  for f in /etc/modprobe.d/*amdxdna* /etc/modprobe.d/*xdna*; do
    [[ -f "$f" ]] || continue
    echo "---- $f ----"
    cat "$f"
  done
  echo

  echo "=== firmware amdnpu/17f0_10 ==="
  ls -la /usr/lib/firmware/amdnpu/17f0_10/ 2>&1 || ls -la /lib/firmware/amdnpu/17f0_10/ 2>&1 || true
  echo

  echo "=== PCI 0000:c6:00.1 (or first 1022:17f0) ==="
  for d in /sys/bus/pci/devices/*; do
    [[ -f "$d/vendor" && -f "$d/device" ]] || continue
    v=$(cat "$d/vendor" 2>/dev/null)
    dev=$(cat "$d/device" 2>/dev/null)
    if [[ "$v" == "0x1022" && "$dev" == "0x17f0" ]]; then
      echo "path: $d"
      echo "vendor/device: $v $dev"
      echo "driver: $(readlink -f "$d/driver" 2>/dev/null || echo none)"
      echo "enable: $(cat "$d/enable" 2>/dev/null || true)"
      cat "$d/uevent" 2>/dev/null || true
    fi
  done
  echo

  echo "=== lsmod (amdxdna / friends) ==="
  lsmod | rg 'amdxdna|amd_pmf|gpu_sched|amdgpu' || true
  echo

  echo "=== accel class (sysfs only) ==="
  ls -la /sys/class/accel/ 2>&1 || true
  ls -la /dev/accel/ 2>&1 || true
  echo

  echo "=== recent journal (amdxdna / oops, last 200 matching lines) ==="
  journalctl -k -n 500 --no-pager 2>/dev/null | rg -i 'amdxdna|aie2|fill_slot|Oops|NULL pointer|BUG:' | tail -n 200 || true
  echo

  echo "=== patch inventory (dragon-npu tree) ==="
  ls -la "${ROOT}/kernel/patches/"000*.patch 2>&1 || true
  echo "base SHA (from patches README):"
  rg -n '905717f5|Tree SHA' "${ROOT}/kernel/patches/README.md" 2>/dev/null | head -5 || true
  echo

  echo "=== done ==="
} | tee "${OUT}"

echo
echo "Wrote: ${OUT}"
echo "Attach to GitHub issue if useful (review for secrets first)."
echo "Optional next: bash scripts/diagnose_amdxdna.sh | tee ${OUT_DIR}/DIAGNOSE_${STAMP}.txt"
