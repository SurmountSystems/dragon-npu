#!/usr/bin/env bash
# Dual-track amdxdna fill_slot check (zen stock vs mainline static / later boot).
# Does NOT open /dev/accel unless you pass --run-repro (DANGEROUS).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPRO="${REPRO:-/tmp/repro_amdxdna_cmdlist_oops}"
ML="${LINUX_MAINLINE:-$HOME/Projects/kernel/linux-mainline}"
RUN_REPRO=0

for a in "$@"; do
  case "$a" in
    --run-repro) RUN_REPRO=1 ;;
    -h|--help)
      echo "Usage: $0 [--run-repro]"
      echo "  Default: print module state + mainline static check only."
      echo "  --run-repro: after switching to stock zen in-tree, run the wedge-risk repro."
      exit 0
      ;;
  esac
done

echo "======== TRACK A: running zen host ========"
echo "kernel: $(uname -r)"
echo
echo "--- modinfo amdxdna ---"
modinfo amdxdna 2>&1 | grep -E 'filename:|srcversion:|vermagic:|intree:|exec_cmd' || true
echo
if [[ -f /lib/modules/$(uname -r)/kernel/drivers/accel/amdxdna/amdxdna.ko.zst ]] ||
   [[ -f /lib/modules/$(uname -r)/kernel/drivers/accel/amdxdna/amdxdna.ko ]]; then
  echo "stock in-tree .ko: present under kernel/drivers/accel/amdxdna/"
else
  echo "stock in-tree .ko: NOT FOUND (unexpected on linux-zen)"
fi
echo
echo "updates/ lab module:"
ls -la /lib/modules/$(uname -r)/updates/amdxdna-hardened/ 2>/dev/null || echo "  (none)"
echo
echo "diagnose (metadata only):"
bash "$ROOT/scripts/diagnose_amdxdna.sh" 2>&1 | grep -E 'verdict:|refcnt:|module_loaded:|pci_driver:|d_state_amdxdna' || true

echo
echo "======== TRACK B: mainline source (static) ========"
if [[ -f "$ML/drivers/accel/amdxdna/aie2_message.c" ]]; then
  echo "tree: $ML"
  echo "HEAD: $(git -C "$ML" rev-parse --short HEAD 2>/dev/null || echo '?')"
  echo "fill_slot xdna source line(s):"
  grep -n 'to_xdna_dev\|cmd_abo->client' "$ML/drivers/accel/amdxdna/aie2_message.c" | head -20
  echo
  if grep -q 'to_xdna_dev(to_gobj(cmd_abo)->dev)' "$ML/drivers/accel/amdxdna/aie2_message.c"; then
    echo "RESULT (static): mainline uses to_xdna_dev(...->dev), not cmd_abo->client->xdna"
  else
    echo "RESULT (static): could not find to_xdna_dev pattern — recheck tree"
  fi
  if grep -q 'cmd_len < sizeof(\*sn)' "$ML/drivers/accel/amdxdna/aie2_message.c"; then
    echo "RESULT (static): short START_NPU payload length check present (-EINVAL path)"
  fi
else
  echo "Mainline sparse tree missing at $ML"
  echo "Clone with:"
  echo "  git clone --depth=1 --filter=blob:none --sparse \\"
  echo "    https://github.com/torvalds/linux.git $ML"
  echo "  cd $ML && git sparse-checkout set drivers/accel/amdxdna"
fi

echo
echo "======== TRACK A: switch to stock zen in-tree (manual root) ========"
cat <<'EOF'
# Only when diagnose is clean (refcnt 0, not WEDGED):

  sudo modprobe -r amdxdna
  sudo modprobe amdxdna
  modinfo amdxdna | grep -E 'filename|srcversion|intree|exec_cmd'
  # expect filename under .../kernel/drivers/accel/... and NO exec_cmd_allow

# Restore lab module later:
  sudo modprobe -r amdxdna
  sudo insmod /lib/modules/$(uname -r)/updates/amdxdna-hardened/amdxdna.ko
  # or reboot
EOF

if [[ "$RUN_REPRO" -eq 1 ]]; then
  echo
  echo "======== RUN REPRO (DANGER) ========"
  if [[ ! -x "$REPRO" ]]; then
    gcc -O2 -Wall -Wextra -o "$REPRO" "$ROOT/examples/repro_amdxdna_cmdlist_oops.c"
  fi
  echo "Running: $REPRO --i-know-this-can-wedge"
  echo "Ensure modinfo shows STOCK in-tree before this."
  "$REPRO" --i-know-this-can-wedge || true
  echo
  echo "journal (last 3 min):"
  journalctl -k --since '3 min ago' --no-pager 2>/dev/null | grep -Ei 'fill_slot|Oops|NULL pointer|amdxdna' | tail -40 || true
else
  echo
  echo "Repro not run (pass --run-repro after switching to stock in-tree)."
fi

echo
echo "======== TRACK B: boot true mainline (later) ========"
cat <<EOF
# Option 1 — Arch vanilla linux package alongside zen (pick in boot menu):
#   sudo pacman -S linux linux-headers
#   reboot → select "linux" not "linux-zen"
#   then: modinfo amdxdna; run repro; compare journal

# Option 2 — full torvalds build (hours):
#   see docs/upstream/MAINLINE_AMDXDNA_CHECK.md § C1
#   CONFIG_DRM_ACCEL=y, CONFIG_DRM_ACCEL_AMDXDNA=m
EOF
