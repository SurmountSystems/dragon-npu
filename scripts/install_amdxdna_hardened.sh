#!/usr/bin/env bash
# L4 — reproducible, wedge-aware install path for L2+L3 hardened amdxdna.
#
# Default mode is dry-run / check only:
#   - Verifies L2+L3 patches exist
#   - Optional patch --dry-run against a clone (or clones at documented SHA)
#   - Wedge preflight (read-only sysfs/ps — never opens /dev/accel)
#   - Prints exact next steps for a human after clean boot
#
# --apply only after preflight passes: clone/checkout SHA, apply 0001+0002,
# print build/install commands. Does NOT insmod, rmmod -f, or force-load over
# a wedge.
#
# Exit codes:
#   0  dry-run / apply source prep OK
#   1  wedge / preflight refuse / apply failure
#   2  bad args / missing artifacts
#
# Usage:
#   bash scripts/install_amdxdna_hardened.sh
#   bash scripts/install_amdxdna_hardened.sh --check
#   bash scripts/install_amdxdna_hardened.sh --dry-run-patches
#   bash scripts/install_amdxdna_hardened.sh --apply --workdir /tmp/xdna-driver-hardened
#   bash scripts/install_amdxdna_hardened.sh --skip-wedge-check   # CI offline dry-run
#   bash scripts/install_amdxdna_hardened.sh --help
#
# Env:
#   AMDXDNA_DRIVER_SHA   override base SHA (default from patches README)
#   AMDXDNA_CLONE_URL    override git URL (requires --trust-clone-url)
#   AMDXDNA_WORKDIR      default workdir for clone
#   AMDXDNA_PCI_ID       PCI slot for preflight sysfs checks (default 0000:c6:00.1)
#   AMDXDNA_REFCNT_WARN  refcnt threshold for SUSPICIOUS (default 5)

set -euo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
PATCHDIR="${ROOT}/kernel/patches"
PATCH_L2="${PATCHDIR}/0001-amdxdna-cmdlist-null-guards.patch"
PATCH_L3="${PATCHDIR}/0002-amdxdna-open-failfast-and-exec-gate.patch"
DKMS_DIR="${ROOT}/kernel/dkms"
PLAYBOOK="${ROOT}/docs/XDNA2_HARDENED_INSTALL.md"
NEVER_AGAIN="${ROOT}/docs/XDNA2_NEVER_AGAIN.md"

# Documented base from kernel/patches/README.md
DEFAULT_SHA="905717f5cd470580054a1635b9eea24b84dfd67f"
DEFAULT_URL="https://github.com/amd/xdna-driver.git"

SHA="${AMDXDNA_DRIVER_SHA:-$DEFAULT_SHA}"
CLONE_URL="${AMDXDNA_CLONE_URL:-$DEFAULT_URL}"
WORKDIR="${AMDXDNA_WORKDIR:-${TMPDIR:-/tmp}/xdna-driver-hardened-${SHA:0:12}}"
PCI_ID="${AMDXDNA_PCI_ID:-0000:c6:00.1}"
REFCNT_WARN="${AMDXDNA_REFCNT_WARN:-5}"

MODE="check" # check | apply
DRY_RUN_PATCHES=0
SKIP_WEDGE=0
SKIP_CLONE=0
CLONE_PATH=""
ALLOW_SUSPICIOUS=0 # allows --apply over SUSPICIOUS_REFCNT only; never force-loads module
KEEP_DIRTY=0
FORCE_CLONE_PATH=0
TRUST_CLONE_URL=0
OFFLINE_SOURCE_ONLY=0
PCI_PREFLIGHT_WARN=0 # set when DRIVER_MISMATCH / PARTIAL_PCI (source apply still OK)

# All status helpers go to stderr so command substitutions (e.g. resolve_tree)
# only capture the intended path/token on stdout.
red() { printf '\033[31m%s\033[0m\n' "$*" >&2; }
grn() { printf '\033[32m%s\033[0m\n' "$*" >&2; }
ylw() { printf '\033[33m%s\033[0m\n' "$*" >&2; }
info() { printf '%s\n' "$*" >&2; }

usage() {
  cat <<'EOF'
install_amdxdna_hardened.sh — L4 ops packaging for L2+L3 hardened amdxdna

Default (no flags): wedge preflight + verify patches exist + print next steps.
Does NOT open /dev/accel, insmod, rmmod -f, or reboot.

Modes / options:
  (default) / --check     Preflight + artifact check + human next steps
  --dry-run-patches       Also clone (if needed) and patch --dry-run L2 then L3
  --apply                 After preflight: clone @ SHA, apply 0001+0002, print
                          build/DKMS commands (no module load)
  --workdir PATH          Clone / work tree (default: $TMPDIR/xdna-driver-hardened-*)
  --clone-path PATH       Use existing xdna-driver tree (must be clean at SHA)
  --force-clone-path      Allow --clone-path when HEAD ≠ documented SHA (warn)
  --keep-dirty            Do not git reset --hard / clean workdir (unsafe)
  --skip-wedge-check      Skip host wedge preflight (CI / offline only)
  --offline-source-only   Required with --skip-wedge-check --apply; suppresses
                          "load now" language (source prep only)
  --allow-suspicious      With --apply: continue if SUSPICIOUS_REFCNT only
                          (not WEDGED_* / PREFLIGHT_UNKNOWN; never force-loads).
                          DRIVER_MISMATCH / PARTIAL_PCI already allow source --apply.
  --trust-clone-url       Allow AMDXDNA_CLONE_URL / non-default clone URL
  -h, --help              This help

Exit codes:
  0  success (check/dry-run/apply prep OK)
  1  refuse (wedged host, preflight, patch fail, missing tree)
  2  usage / missing in-repo artifacts

Install preflight verdicts (subset of diagnose):
  OK | MODULE_NOT_LOADED | WEDGED_OPEN | WEDGED_OTHER | SUSPICIOUS_REFCNT
  | PREFLIGHT_UNKNOWN | DRIVER_MISMATCH | PARTIAL_PCI
  --apply refuses WEDGED_* / PREFLIGHT_UNKNOWN / SUSPICIOUS_REFCNT (unless
  --allow-suspicious). DRIVER_MISMATCH / PARTIAL_PCI warn but allow source prep.
  Full diagnose: bash scripts/diagnose_amdxdna.sh

See also:
  docs/XDNA2_HARDENED_INSTALL.md
  kernel/patches/README.md
  kernel/dkms/README.md
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --check)
      MODE="check"
      shift
      ;;
    --dry-run-patches)
      DRY_RUN_PATCHES=1
      shift
      ;;
    --apply)
      MODE="apply"
      DRY_RUN_PATCHES=1
      shift
      ;;
    --workdir)
      WORKDIR=${2:-}
      [[ -n "$WORKDIR" ]] || { red "install: --workdir needs a path"; exit 2; }
      shift 2
      ;;
    --clone-path)
      CLONE_PATH=${2:-}
      [[ -n "$CLONE_PATH" ]] || { red "install: --clone-path needs a path"; exit 2; }
      SKIP_CLONE=1
      shift 2
      ;;
    --force-clone-path)
      FORCE_CLONE_PATH=1
      shift
      ;;
    --keep-dirty)
      KEEP_DIRTY=1
      shift
      ;;
    --skip-wedge-check)
      SKIP_WEDGE=1
      shift
      ;;
    --offline-source-only)
      OFFLINE_SOURCE_ONLY=1
      shift
      ;;
    --allow-suspicious)
      ALLOW_SUSPICIOUS=1
      shift
      ;;
    --trust-clone-url)
      TRUST_CLONE_URL=1
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      red "install: unknown arg: $1"
      usage >&2
      exit 2
      ;;
  esac
done

# --skip-wedge-check --apply requires explicit offline-source-only
if [[ $SKIP_WEDGE -eq 1 && $MODE == apply && $OFFLINE_SOURCE_ONLY -eq 0 ]]; then
  red "install: --skip-wedge-check with --apply requires --offline-source-only"
  red "  (source prep only; host health not verified — do not load module until diagnose is clean)"
  exit 2
fi

# Clone URL allowlist (Issue 12)
if [[ "$CLONE_URL" != "$DEFAULT_URL" && $TRUST_CLONE_URL -eq 0 ]]; then
  red "install: non-default clone URL requires --trust-clone-url"
  red "  default: $DEFAULT_URL"
  red "  got:     $CLONE_URL"
  red "  verify the remote before building/loading any module as root"
  exit 2
fi
if [[ $TRUST_CLONE_URL -eq 1 && "$CLONE_URL" != "$DEFAULT_URL" ]]; then
  ylw "WARNING: using non-default clone URL (supply-chain risk if untrusted):"
  ylw "  $CLONE_URL"
  ylw "  checkout still pins SHA $SHA — verify that object is what you intend"
fi

# --- artifact check ----------------------------------------------------------

check_artifacts() {
  local ok=1
  info "======== L2+L3 artifacts ========"
  for f in "$PATCH_L2" "$PATCH_L3"; do
    if [[ -f "$f" ]]; then
      grn "  OK  $f"
    else
      red "  MISSING  $f"
      ok=0
    fi
  done
  if [[ -f "$PATCHDIR/README.md" ]]; then
    grn "  OK  $PATCHDIR/README.md"
  else
    ylw "  WARN missing patches README"
  fi
  if [[ -d "$DKMS_DIR" ]]; then
    grn "  OK  $DKMS_DIR (DKMS scaffolding)"
  else
    ylw "  WARN no kernel/dkms yet"
  fi
  if [[ -f "$PLAYBOOK" ]]; then
    grn "  OK  $PLAYBOOK"
  else
    ylw "  WARN playbook not found (expected docs/XDNA2_HARDENED_INSTALL.md)"
  fi
  info "  base SHA: $SHA"
  info "  clone URL: $CLONE_URL"
  info "  pci_id: $PCI_ID (sysfs preflight)"
  [[ $ok -eq 1 ]] || return 1
  return 0
}

# --- wedge preflight (read-only; never open /dev/accel) -----------------------

# Sets globals: PREFLIGHT_VERDICT PREFLIGHT_REASON REFCNT D_OPEN D_WAITERS
# D_CMD_PATH_HINT PS_OK PCI_PRESENT DRIVER_BOUND
preflight_wedge() {
  REFCNT="n/a"
  local module_loaded=0
  D_OPEN=0
  D_WAITERS=0
  D_CMD_PATH_HINT=0
  PS_OK=1
  PCI_PRESENT=0
  DRIVER_BOUND="none"
  local refcnt_unreadable=0
  local ps_out=""

  if [[ -d /sys/module/amdxdna ]]; then
    module_loaded=1
    if [[ -r /sys/module/amdxdna/refcnt ]]; then
      REFCNT=$(cat /sys/module/amdxdna/refcnt 2>/dev/null || echo "?")
      if [[ ! "$REFCNT" =~ ^[0-9]+$ ]]; then
        refcnt_unreadable=1
        REFCNT="?"
      fi
    else
      refcnt_unreadable=1
      REFCNT="?"
    fi
  fi

  # Read-only PCI / bind (same class as diagnose; no device open)
  if [[ -d "/sys/bus/pci/devices/$PCI_ID" ]]; then
    PCI_PRESENT=1
    if [[ -e "/sys/bus/pci/devices/$PCI_ID/driver" ]]; then
      DRIVER_BOUND=$(basename "$(readlink -f "/sys/bus/pci/devices/$PCI_ID/driver" 2>/dev/null || echo unknown)")
    else
      DRIVER_BOUND="unbound"
    fi
  fi

  # Capture ps once; fail closed if ps is unusable when we need it
  if ! ps_out=$(ps -eo pid,stat,wchan:40,cmd 2>/dev/null); then
    PS_OK=0
    ps_out=""
  fi
  if [[ $PS_OK -eq 1 && -z "$ps_out" ]]; then
    PS_OK=0
  fi

  if [[ $PS_OK -eq 1 ]]; then
    # WEDGED_OPEN: wchan is amdxdna_drm_open
    D_OPEN=$(printf '%s\n' "$ps_out" \
      | awk 'NR>1 && $2 ~ /D/ && $3 ~ /amdxdna_drm_open/ {c++} END {print c+0}')
    # WEDGED_OTHER: wchan contains amdxdna (not cmd-path false positive)
    D_WAITERS=$(printf '%s\n' "$ps_out" \
      | awk 'NR>1 && $2 ~ /D/ && $3 ~ /amdxdna/ {c++} END {print c+0}')
    # Secondary: D-state with amdxdna only in cmd/path (build tree stall) → SUSPICIOUS hint
    D_CMD_PATH_HINT=$(printf '%s\n' "$ps_out" \
      | awk 'NR>1 && $2 ~ /D/ && $3 !~ /amdxdna/ && $0 ~ /amdxdna/ {c++} END {print c+0}')
  fi

  PREFLIGHT_VERDICT="OK"
  PREFLIGHT_REASON="no D-state amdxdna waiters"

  if [[ $PS_OK -eq 0 && $module_loaded -eq 1 ]]; then
    PREFLIGHT_VERDICT="PREFLIGHT_UNKNOWN"
    PREFLIGHT_REASON="ps failed or empty while amdxdna loaded — cannot rule out D-state; refuse --apply"
  elif [[ $module_loaded -eq 1 && $refcnt_unreadable -eq 1 ]]; then
    PREFLIGHT_VERDICT="PREFLIGHT_UNKNOWN"
    PREFLIGHT_REASON="amdxdna loaded but refcnt unreadable — refuse --apply"
  elif [[ $module_loaded -eq 0 ]]; then
    PREFLIGHT_VERDICT="MODULE_NOT_LOADED"
    PREFLIGHT_REASON="amdxdna not loaded (install after clean boot is fine)"
  elif [[ "$D_OPEN" -gt 0 ]]; then
    PREFLIGHT_VERDICT="WEDGED_OPEN"
    PREFLIGHT_REASON="$D_OPEN process(es) in amdxdna_drm_open (D); refcnt=$REFCNT — refuse install/load"
  elif [[ "$D_WAITERS" -gt 0 ]]; then
    PREFLIGHT_VERDICT="WEDGED_OTHER"
    PREFLIGHT_REASON="$D_WAITERS D-state task(s) with wchan ~ amdxdna; refcnt=$REFCNT — refuse install/load"
  elif [[ "$REFCNT" =~ ^[0-9]+$ ]] && [[ $REFCNT -gt $REFCNT_WARN ]]; then
    PREFLIGHT_VERDICT="SUSPICIOUS_REFCNT"
    PREFLIGHT_REASON="refcnt=$REFCNT > $REFCNT_WARN with no wchan-amdxdna D waiters"
  elif [[ "$D_CMD_PATH_HINT" -gt 0 ]]; then
    PREFLIGHT_VERDICT="SUSPICIOUS_REFCNT"
    PREFLIGHT_REASON="$D_CMD_PATH_HINT D-state task(s) with amdxdna only in cmd/path (not wchan) — not hard wedge; prefer re-check"
  elif [[ $PCI_PRESENT -eq 1 && $DRIVER_BOUND != amdxdna && $DRIVER_BOUND != unbound && $DRIVER_BOUND != none ]]; then
    PREFLIGHT_VERDICT="DRIVER_MISMATCH"
    PREFLIGHT_REASON="PCI $PCI_ID bound to $DRIVER_BOUND, not amdxdna"
  elif [[ $PCI_PRESENT -eq 0 && $module_loaded -eq 1 ]]; then
    PREFLIGHT_VERDICT="PARTIAL_PCI"
    PREFLIGHT_REASON="module loaded but PCI $PCI_ID not present (wrong id or removed)"
  else
    PREFLIGHT_VERDICT="OK"
    PREFLIGHT_REASON="module_loaded=$module_loaded refcnt=$REFCNT d_open=$D_OPEN d_wchan=$D_WAITERS pci=$PCI_PRESENT driver=$DRIVER_BOUND"
  fi

  info "======== wedge preflight (read-only) ========"
  info "  verdict: $PREFLIGHT_VERDICT"
  info "  reason:  $PREFLIGHT_REASON"
  info "  refcnt:  $REFCNT"
  info "  d_open:  $D_OPEN  d_wchan_amdxdna: $D_WAITERS  d_cmd_path_hint: $D_CMD_PATH_HINT"
  info "  ps_ok:   $PS_OK  pci: $PCI_PRESENT ($PCI_ID) driver: $DRIVER_BOUND"
  info "  note:    never opened /dev/accel; residual: non-amdxdna wchan hangs not fully covered"
  info "  full:    bash scripts/diagnose_amdxdna.sh"

  case "$PREFLIGHT_VERDICT" in
    WEDGED_OPEN | WEDGED_OTHER | PREFLIGHT_UNKNOWN)
      return 1
      ;;
    SUSPICIOUS_REFCNT)
      if [[ $ALLOW_SUSPICIOUS -eq 1 ]]; then
        ylw "  allowing SUSPICIOUS_REFCNT due to --allow-suspicious (still no force load)"
        return 0
      fi
      # check / dry-run: warn and continue so humans see next steps
      # --apply: refuse unless --allow-suspicious
      if [[ $MODE == apply ]]; then
        return 1
      fi
      ylw "  SUSPICIOUS_REFCNT: prefer clean boot / diagnose before --apply"
      return 0
      ;;
    DRIVER_MISMATCH | PARTIAL_PCI)
      # Source prep does not load the module; PCI/bind weirdness must not block
      # --apply. Always warn loudly; suppress “load now” language in build print
      # when these verdicts fire (see print_build_commands / PCI_PREFLIGHT_WARN).
      PCI_PREFLIGHT_WARN=1
      ylw "  $PREFLIGHT_VERDICT: source prep still allowed; do NOT modprobe until PCI/bind is sane"
      ylw "  reason: $PREFLIGHT_REASON"
      return 0
      ;;
    *)
      return 0
      ;;
  esac
}

print_wedged_next_steps() {
  red "======== INSTALL REFUSED: host wedged ========"
  cat <<EOF >&2

Verdict: $PREFLIGHT_VERDICT
Reason:  $PREFLIGHT_REASON

Do NOT:
  - open /dev/accel in a loop
  - rmmod -f / force-load a replacement over this wedge
  - insmod a "fixed" .ko while D-state waiters hold the old module

Do:
  1. Prefer reboot to clear D-state + refcnt
     (optional engineering path: scripts/amdxdna_recover_eng.sh — root, may still fail)
  2. After clean boot:
       bash scripts/diagnose_amdxdna.sh
       # want LIKELY_OK_METADATA or MODULE_NOT_LOADED, no WEDGED_*
  3. Then install hardened driver:
       bash scripts/install_amdxdna_hardened.sh --apply --workdir /tmp/xdna-driver-hardened
  4. Playbook: docs/XDNA2_HARDENED_INSTALL.md
  5. Policy: docs/XDNA2_NEVER_AGAIN.md

EOF
}

print_suspicious_next_steps() {
  # Only used for SUSPICIOUS_REFCNT refuse (DRIVER_MISMATCH / PARTIAL_PCI no longer refuse).
  ylw "======== INSTALL REFUSED: SUSPICIOUS_REFCNT (not a hard D-state wedge) ========"
  cat <<EOF >&2

Verdict: $PREFLIGHT_VERDICT
Reason:  $PREFLIGHT_REASON

This is **not** WEDGED_OPEN / WEDGED_OTHER. Do not reboot solely on this message
unless diagnose also shows a hard wedge.

Do:
  1. bash scripts/diagnose_amdxdna.sh
  2. Drop stale userspace clients if refcnt is high without D-state
  3. Prefer clean boot if refcnt stays elevated or open hangs
  4. Source-only --apply when refcnt is merely suspicious (no D-state):
       bash scripts/install_amdxdna_hardened.sh --apply --allow-suspicious \\
         --workdir /tmp/xdna-driver-hardened
     (--allow-suspicious applies only to SUSPICIOUS_REFCNT; still never force-loads)
  5. Note: DRIVER_MISMATCH / PARTIAL_PCI no longer block --apply (source-only);
     they warn and continue. Use diagnose before any modprobe.
  6. Playbook: docs/XDNA2_HARDENED_INSTALL.md

EOF
}

print_unknown_next_steps() {
  red "======== INSTALL REFUSED: preflight unknown (cannot prove host is safe) ========"
  cat <<EOF >&2

Verdict: $PREFLIGHT_VERDICT
Reason:  $PREFLIGHT_REASON

ps and/or refcnt could not be trusted. Bias: refuse --apply rather than claim OK.

Do:
  1. bash scripts/diagnose_amdxdna.sh
  2. Fix environment (ps, /sys permissions) or reboot if already suspect
  3. Offline source prep only (no host health claim):
       bash scripts/install_amdxdna_hardened.sh --apply --skip-wedge-check \\
         --offline-source-only --workdir /tmp/xdna-driver-hardened
  4. Do **not** modprobe until diagnose is clean on a known-good boot

EOF
}

print_skip_wedge_banner() {
  ylw "======== wedge preflight SKIPPED (--skip-wedge-check) ========"
  ylw "  HOST HEALTH WAS NOT VERIFIED."
  ylw "  Do NOT modprobe / insmod / modprobe -r until:"
  ylw "    bash scripts/diagnose_amdxdna.sh   # not WEDGED_*"
  if [[ $OFFLINE_SOURCE_ONLY -eq 1 ]]; then
    ylw "  mode: offline source-only (load instructions suppressed)"
  fi
}

print_human_next_steps() {
  info "======== next steps (human) ========"
  cat <<EOF >&2

After a **clean boot** (no WEDGED_* from diagnose — full script, not only this preflight):

  # 1) Confirm host is healthy (read-only)
  bash scripts/diagnose_amdxdna.sh

  # 2) Prepare patched source tree (this script)
  bash scripts/install_amdxdna_hardened.sh --apply --workdir /tmp/xdna-driver-hardened

  # 3) Build / install — prefer upstream xdna-driver CMake or DKMS docs.
  #    Template hooks: kernel/dkms/README.md
  #    Example (after --apply left a tree at WORKDIR):
  #      cd "\$WORKDIR"
  #      # follow amd/xdna-driver README / tools for this release
  #      # or: bash kernel/dkms/hooks/prepare_from_clone.sh "\$WORKDIR"

  # 4) Load with lab defaults (L3) — ONLY if diagnose is still clean:
  #      sudo modprobe amdxdna            # exec_cmd_allow=0 default after L3
  #      # or: sudo modprobe amdxdna exec_cmd_allow=0
  #    Do NOT set exec_cmd_allow=1 until a known-good payload + L1 gates.

  # 5) Verify without dangerous EXEC_CMD:
  #      bash scripts/diagnose_amdxdna.sh
  #      timeout 15 cargo run --features amd-xdna2 --example npu_direct_bench -- --checklist
  #      modinfo amdxdna | grep -E 'filename|exec_cmd|vermagic' || true

  # 6) Mark NEVER_AGAIN host-install checkbox only after step 5 is clean.

Playbook: ${PLAYBOOK}
Patches:  ${PATCHDIR}/README.md
DKMS:     ${DKMS_DIR}/README.md
Recovery: ${ROOT}/docs/XDNA2_RECOVERY_ENGINEERING.md
Policy:   ${NEVER_AGAIN}

EOF
}

print_build_commands() {
  local tree=$1
  local suppress_load=0
  if [[ $SKIP_WEDGE -eq 1 || $OFFLINE_SOURCE_ONLY -eq 1 || $PCI_PREFLIGHT_WARN -eq 1 ]]; then
    suppress_load=1
  fi

  info "======== build / install commands (print only — not executed) ========"
  cat <<EOF >&2

Patched tree ready at:
  $tree

Stack applied: 0001 (L2 null-guards) then 0002 (L3 open fail-fast + exec_cmd_allow).
Base SHA: $SHA

Preferred paths (do not invent a broken Makefile; use upstream build system):

  A) Upstream xdna-driver CMake / package flow
     cd "$tree"
     # Read the tree's README / docs for the release you checked out.
     # Typical pattern (names evolve — verify in-tree):
     #   cmake -S . -B build   # or src/driver packaging scripts
     #   cmake --build build
     # Install .ko via the project's install target or DKMS package.

  B) DKMS from this repo's template (hooks use your clone path)
     # From dragon-npu root, after patches applied to \$tree:
     bash kernel/dkms/hooks/prepare_from_clone.sh "$tree"
     # Then follow kernel/dkms/README.md (dkms add/build/install).
     # Needs: matching kernel headers (uname -r), e.g. on Arch/Garuda zen:
     #   pacman -S linux-zen-headers   # or your running kernel's -headers

  C) Last-resort bare Kbuild (often incomplete without config_kernel.h)
     cd "$tree/src/driver/amdxdna"
     # Prefer tools/configure_kernel.sh or CMake first.
     # make -C /lib/modules/\$(uname -r)/build M=\$PWD modules

EOF

  if [[ $suppress_load -eq 1 ]]; then
    cat <<EOF >&2
*** MODULE LOAD STEPS SUPPRESSED ***
  reason: skip-wedge/offline and/or PCI preflight warn (DRIVER_MISMATCH / PARTIAL_PCI)
Re-run diagnose on a clean host before any modprobe:
  bash scripts/diagnose_amdxdna.sh
Do NOT follow load instructions until that verdict is not WEDGED_* and PCI/bind is sane.

EOF
  else
    cat <<EOF >&2
After a successful module install, on a **clean** host only
(re-check diagnose immediately before load — TOCTOU residual):

  # unload stock only if unused and not wedged
  sudo modprobe -r amdxdna    # never use rmmod -f by default
  sudo modprobe amdxdna       # lab: exec_cmd_allow=0 (L3 default)
  # optional permanent lab default:
  #   echo 'options amdxdna exec_cmd_allow=0' | sudo tee /etc/modprobe.d/amdxdna-hardened.conf

  bash scripts/diagnose_amdxdna.sh
  timeout 15 cargo run --features amd-xdna2 --example npu_direct_bench -- --checklist

EOF
  fi

  cat <<EOF >&2
Rollback: restore distro/in-tree module, remove DKMS package, reboot if needed.
See docs/XDNA2_HARDENED_INSTALL.md § Rollback.

EOF
}

# --- clone / patch -----------------------------------------------------------

validate_clone_path() {
  local path=$1
  if [[ ! -d "$path" ]]; then
    red "install: --clone-path not a directory: $path"
    return 1
  fi
  if [[ ! -d "$path/src/driver/amdxdna" ]]; then
    red "install: --clone-path missing src/driver/amdxdna: $path"
    return 1
  fi

  if [[ -d "$path/.git" ]]; then
    local head
    head=$(git -C "$path" rev-parse HEAD 2>/dev/null || echo "")
    if [[ -z "$head" ]]; then
      red "install: --clone-path has .git but rev-parse failed"
      return 1
    fi
    if [[ "$head" != "$SHA" ]]; then
      if [[ $FORCE_CLONE_PATH -eq 0 ]]; then
        red "install: --clone-path HEAD=$head ≠ documented SHA $SHA"
        red "  use --force-clone-path to override (then dry-run both patches carefully)"
        return 1
      fi
      ylw "install: --force-clone-path: HEAD=$head (expected $SHA)"
    fi
    local dirty
    dirty=$(git -C "$path" status --porcelain 2>/dev/null || echo "dirty-unknown")
    if [[ -n "$dirty" && $KEEP_DIRTY -eq 0 ]]; then
      red "install: --clone-path is dirty (git status non-empty). Refuse."
      red "  reset/clean the tree, re-run install --apply, or pass --keep-dirty (unsafe)"
      return 1
    fi
    if [[ -n "$dirty" && $KEEP_DIRTY -eq 1 ]]; then
      ylw "install: --keep-dirty: proceeding with dirty clone-path (unsafe)"
    fi
  else
    if [[ $FORCE_CLONE_PATH -eq 0 ]]; then
      red "install: --clone-path is not a git tree; pass --force-clone-path to acknowledge"
      red "  (still requires successful patch --dry-run of 0001+0002)"
      return 1
    fi
    ylw "install: non-git --clone-path with --force-clone-path; relying on dry-run only"
  fi
  return 0
}

reset_tree_to_sha() {
  local tree=$1
  if [[ $KEEP_DIRTY -eq 1 ]]; then
    ylw "install: --keep-dirty: skipping git reset --hard / clean in $tree"
    return 0
  fi
  # All git chatter → stderr so TREE=$(resolve_tree) stays a single path line.
  (
    cd "$tree"
    git checkout --detach "$SHA" >/dev/null 2>&1 || {
      git fetch --depth=1 origin "$SHA" >/dev/null 2>&1 \
        || git fetch origin "$SHA" >/dev/null 2>&1 \
        || true
      if ! git checkout --detach "$SHA" >/dev/null 2>&1; then
        git fetch origin "+refs/heads/*:refs/remotes/origin/*" >/dev/null 2>&1 \
          || git fetch --unshallow >/dev/null 2>&1 \
          || true
        git checkout --detach "$SHA" >/dev/null 2>&1
      fi
    }
    git reset --hard "$SHA" >/dev/null 2>&1
    git clean -fdx >/dev/null 2>&1
    # Verify we landed on the intended object
    local head
    head=$(git rev-parse HEAD 2>/dev/null)
    [[ "$head" == "$SHA" ]]
  ) || {
    red "install: failed to reset $tree to pristine $SHA"
    return 1
  }
  info "  tree reset to pristine $SHA (git reset --hard + clean -fdx)"
}

resolve_tree() {
  if [[ $SKIP_CLONE -eq 1 ]]; then
    if ! validate_clone_path "$CLONE_PATH"; then
      return 1
    fi
    printf '%s\n' "$CLONE_PATH"
    return 0
  fi

  if [[ -d "$WORKDIR/.git" ]]; then
    info "reusing workdir: $WORKDIR"
  else
    info "cloning $CLONE_URL → $WORKDIR"
    mkdir -p "$(dirname "$WORKDIR")"
    # Redirect clone progress to stderr so $(resolve_tree) stays a pure path
    if ! git clone --filter=blob:none "$CLONE_URL" "$WORKDIR" >&2; then
      red "install: git clone failed"
      return 1
    fi
  fi

  if ! reset_tree_to_sha "$WORKDIR"; then
    return 1
  fi

  # Sole stdout line for callers that capture TREE=$(resolve_tree)
  printf '%s\n' "$WORKDIR"
}

# Apply or confirm one patch on a clean tree: forward OK, or reverse OK (= already).
# Mixed half-applied trees fail (forward and reverse both fail).
patch_one() {
  local mode=$1 # dry|apply
  local patchfile=$2
  local label=$3
  if [[ $mode == dry ]]; then
    if patch --dry-run -p1 <"$patchfile" >/dev/null 2>&1; then
      grn "  APPLY_OK dry-run: $label"
      return 0
    fi
    if patch --dry-run -R -p1 <"$patchfile" >/dev/null 2>&1; then
      ylw "  already applied (reverse dry-run OK): $label"
      return 0
    fi
    red "  dry-run FAILED (not clean base, not fully applied): $label"
    return 1
  fi
  # apply
  if patch --dry-run -p1 <"$patchfile" >/dev/null 2>&1; then
    patch -p1 <"$patchfile"
    grn "  applied $label"
    return 0
  fi
  if patch --dry-run -R -p1 <"$patchfile" >/dev/null 2>&1; then
    ylw "  already applied — skip $label"
    return 0
  fi
  red "  apply FAILED (not clean base, not fully applied): $label"
  return 1
}

dry_run_patches() {
  local tree=$1
  info "======== patch --dry-run (L2 then L3) ========"
  info "  tree: $tree"
  (
    cd "$tree"
    patch_one dry "$PATCH_L2" "0001-amdxdna-cmdlist-null-guards.patch" || return 1
    patch_one dry "$PATCH_L3" "0002-amdxdna-open-failfast-and-exec-gate.patch" || return 1
  )
}

apply_patches() {
  local tree=$1
  info "======== apply patches (L2 then L3) ========"
  # Workdir path is reset to pristine first; --clone-path must be clean (or --keep-dirty).
  (
    cd "$tree"
    patch_one apply "$PATCH_L2" "0001" || return 1
    patch_one apply "$PATCH_L3" "0002" || return 1
  )
}

# --- main --------------------------------------------------------------------

info "amdxdna L4 hardened install helper"
info "root: $ROOT"
info "mode: $MODE  dry_run_patches=$DRY_RUN_PATCHES  skip_wedge=$SKIP_WEDGE  offline=$OFFLINE_SOURCE_ONLY"
info "time: $(date -Is 2>/dev/null || date)"
info "kernel: $(uname -srm 2>/dev/null || uname -r)"
info

if ! check_artifacts; then
  red "install: missing required patches under kernel/patches/"
  exit 2
fi
info

if [[ $SKIP_WEDGE -eq 0 ]]; then
  if ! preflight_wedge; then
    case "$PREFLIGHT_VERDICT" in
      WEDGED_OPEN | WEDGED_OTHER)
        print_wedged_next_steps
        ;;
      PREFLIGHT_UNKNOWN)
        print_unknown_next_steps
        ;;
      *)
        print_suspicious_next_steps
        ;;
    esac
    exit 1
  fi
else
  print_skip_wedge_banner
fi
info

if [[ $DRY_RUN_PATCHES -eq 1 || $MODE == apply ]]; then
  if ! command -v git >/dev/null 2>&1; then
    red "install: git required for clone/checkout"
    exit 1
  fi
  if ! command -v patch >/dev/null 2>&1; then
    red "install: patch(1) required"
    exit 1
  fi

  TREE=$(resolve_tree) || exit 1
  info "  tree: $TREE"
  info

  if ! dry_run_patches "$TREE"; then
    red "install: patch dry-run failed (rebase may be needed if upstream moved)"
    exit 1
  fi
  info

  if [[ $MODE == apply ]]; then
    if ! apply_patches "$TREE"; then
      red "install: apply failed"
      exit 1
    fi
    print_build_commands "$TREE"
    grn "Source prep complete. Module was NOT loaded (by design)."
    info "Mark host-install in NEVER_AGAIN only after build + modprobe + diagnose on a clean boot."
  else
    if [[ $KEEP_DIRTY -eq 1 || $SKIP_CLONE -eq 1 ]]; then
      grn "Dry-run patches OK (tree not reset to pristine; no patches applied this run)."
      if [[ $KEEP_DIRTY -eq 1 ]]; then
        ylw "  note: --keep-dirty — tree may already contain prior edits/patches"
      fi
      if [[ $SKIP_CLONE -eq 1 ]]; then
        ylw "  note: --clone-path — used existing tree (not a fresh reset workdir)"
      fi
    else
      grn "Dry-run patches OK. Tree left at pristine base (no patches applied)."
    fi
    print_human_next_steps
  fi
else
  print_human_next_steps
fi

exit 0
