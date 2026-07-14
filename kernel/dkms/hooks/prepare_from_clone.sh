#!/usr/bin/env bash
# Stage a patched xdna-driver clone for DKMS template use (L4).
#
# Usage:
#   bash kernel/dkms/hooks/prepare_from_clone.sh /path/to/patched-xdna-driver
#   bash kernel/dkms/hooks/prepare_from_clone.sh /path/to/patched-xdna-driver /path/to/stage
#   bash kernel/dkms/hooks/prepare_from_clone.sh CLONE --force-stage /abs/stage
#   bash kernel/dkms/hooks/prepare_from_clone.sh CLONE --allow-unpatched
#   bash kernel/dkms/hooks/prepare_from_clone.sh CLONE --replace-stage   # rsync --delete
#
# Default stage: kernel/dkms/staging/amdxdna-hardened-0.1.0
#
# Prerequisites:
#   - Clone already checked out at the documented base SHA and patched with
#     0001 then 0002 (use scripts/install_amdxdna_hardened.sh --apply).
#   - Matching kernel headers for uname -r.
#
# Does NOT: insmod, rmmod, open /dev/accel, reboot, or run dkms as root.

set -euo pipefail

ROOT=$(cd "$(dirname "$0")/../../.." && pwd)
DKMS_DIR="${ROOT}/kernel/dkms"
PKG_NAME="amdxdna-hardened"
PKG_VER="0.1.0"
DEFAULT_STAGE="${DKMS_DIR}/staging/${PKG_NAME}-${PKG_VER}"
EXPECTED_SHA="${AMDXDNA_DRIVER_SHA:-905717f5cd470580054a1635b9eea24b84dfd67f}"

# L2-unique marker from 0001 (not present in unpatched upstream fill_slot)
L2_MARKER='Reject invalid args early; never deref NULL on the cmdlist path'
# L3 markers from 0002
L3_MARKER_RE='exec_cmd_allow|dev_unhealthy'

CLONE=""
STAGE=""
ALLOW_UNPATCHED=0
FORCE_STAGE=0
REPLACE_STAGE=0

usage() {
  cat <<EOF
prepare_from_clone.sh — stage patched xdna-driver sources for DKMS template

  $0 CLONE_PATH [STAGE_PATH]
  $0 CLONE_PATH [--force-stage STAGE] [--allow-unpatched] [--replace-stage]

CLONE_PATH     Path to amd/xdna-driver tree with L2+L3 already applied
STAGE_PATH     Stage dir (default: $DEFAULT_STAGE)
               Must stay under kernel/dkms/staging/ unless --force-stage
--force-stage  Allow STAGE outside default prefix (still refuses /, empty, etc.)
--allow-unpatched  Do not fail if L2/L3 markers missing (WARN only)
--replace-stage    Use rsync --delete (default: no --delete)

Then (root, clean host only):
  sudo rsync -a "\$STAGE_PATH/" /usr/src/${PKG_NAME}-${PKG_VER}/
  sudo cp ${DKMS_DIR}/dkms.conf /usr/src/${PKG_NAME}-${PKG_VER}/dkms.conf
  # Edit MAKE[] paths in dkms.conf if your tree layout differs; AUTOINSTALL=no by default
  sudo dkms add -m ${PKG_NAME} -v ${PKG_VER}
  sudo dkms build -m ${PKG_NAME} -v ${PKG_VER} -k "\$(uname -r)"
  sudo dkms install -m ${PKG_NAME} -v ${PKG_VER} -k "\$(uname -r)"
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h | --help)
      usage
      exit 0
      ;;
    --allow-unpatched)
      ALLOW_UNPATCHED=1
      shift
      ;;
    --replace-stage)
      REPLACE_STAGE=1
      shift
      ;;
    --force-stage)
      FORCE_STAGE=1
      STAGE=${2:-}
      [[ -n "$STAGE" ]] || { echo "prepare: --force-stage needs a path" >&2; exit 2; }
      shift 2
      ;;
    --*)
      echo "prepare: unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
    *)
      if [[ -z "$CLONE" ]]; then
        CLONE=$1
      elif [[ -z "$STAGE" ]]; then
        STAGE=$1
      else
        echo "prepare: unexpected arg: $1" >&2
        exit 2
      fi
      shift
      ;;
  esac
done

if [[ -z "$CLONE" ]]; then
  usage
  exit 0
fi

STAGE=${STAGE:-$DEFAULT_STAGE}

# Resolve absolute paths
CLONE=$(cd "$CLONE" 2>/dev/null && pwd -P) || {
  echo "prepare: not a directory: $CLONE" >&2
  exit 2
}
mkdir -p "$(dirname "$STAGE")" 2>/dev/null || true
# For STAGE that does not exist yet, resolve parent then append basename
if [[ -d "$STAGE" ]]; then
  STAGE=$(cd "$STAGE" && pwd -P)
else
  parent=$(cd "$(dirname "$STAGE")" 2>/dev/null && pwd -P) || {
    echo "prepare: cannot resolve stage parent: $STAGE" >&2
    exit 2
  }
  STAGE="${parent}/$(basename "$STAGE")"
fi

# Destination safety (Issue 4)
if [[ -z "$STAGE" || "$STAGE" == "/" ]]; then
  echo "prepare: refusing empty or root STAGE" >&2
  exit 1
fi
case "$STAGE" in
  /bin | /bin/* | /usr | /usr/* | /etc | /etc/* | /home | /lib | /lib/* | /boot | /boot/*)
    if [[ $FORCE_STAGE -eq 0 ]]; then
      echo "prepare: refusing STAGE under system prefix without --force-stage: $STAGE" >&2
      exit 1
    fi
    ;;
esac
# Must contain package name fragment
if [[ "$STAGE" != *"$PKG_NAME"* ]]; then
  echo "prepare: STAGE must contain package name fragment '$PKG_NAME': $STAGE" >&2
  exit 1
fi
# Default: only allow under kernel/dkms/staging/
mkdir -p "${DKMS_DIR}/staging"
staging_prefix=$(cd "${DKMS_DIR}/staging" && pwd -P)
if [[ $FORCE_STAGE -eq 0 ]]; then
  case "$STAGE" in
    "$staging_prefix" | "$staging_prefix"/*) ;;
    *)
      echo "prepare: STAGE must be under $staging_prefix (or pass --force-stage)" >&2
      echo "  got: $STAGE" >&2
      exit 1
      ;;
  esac
fi

if [[ ! -d "$CLONE/src/driver/amdxdna" ]]; then
  echo "prepare: expected src/driver/amdxdna under clone (xdna-driver layout)" >&2
  exit 1
fi

msg_c="$CLONE/src/driver/amdxdna/aie2_message.c"
msg_drm_c="$CLONE/src/driver/amdxdna/amdxdna_drm.c"
msg_drm_h="$CLONE/src/driver/amdxdna/amdxdna_drm.h"

# L2/L3 markers — fail unless --allow-unpatched (Issue 6)
l2_ok=0
l3_ok=0
if [[ -f "$msg_c" ]] && grep -qF "$L2_MARKER" "$msg_c" 2>/dev/null; then
  l2_ok=1
fi
if grep -qE "$L3_MARKER_RE" "$msg_drm_c" "$msg_drm_h" 2>/dev/null; then
  l3_ok=1
fi

if [[ $l2_ok -eq 0 ]]; then
  echo "prepare: L2 marker missing in aie2_message.c (null-guards not applied?)" >&2
  echo "  expected substring: $L2_MARKER" >&2
  if [[ $ALLOW_UNPATCHED -eq 0 ]]; then
    echo "  apply 0001 first, or pass --allow-unpatched" >&2
    exit 1
  fi
  echo "  WARN: continuing due to --allow-unpatched" >&2
fi
if [[ $l3_ok -eq 0 ]]; then
  echo "prepare: L3 markers (exec_cmd_allow / dev_unhealthy) not found" >&2
  if [[ $ALLOW_UNPATCHED -eq 0 ]]; then
    echo "  apply 0002 first: scripts/install_amdxdna_hardened.sh --apply" >&2
    echo "  or pass --allow-unpatched" >&2
    exit 1
  fi
  echo "  WARN: continuing due to --allow-unpatched" >&2
fi

mkdir -p "$STAGE"
echo "prepare: rsync $CLONE/ → $STAGE/"
rsync_opts=(-a
  --exclude '.git'
  --exclude 'build'
  --exclude '*.o'
  --exclude '*.ko'
  --exclude '.*.cmd')
if [[ $REPLACE_STAGE -eq 1 ]]; then
  echo "prepare: --replace-stage: using rsync --delete"
  rsync_opts+=(--delete)
fi
rsync "${rsync_opts[@]}" "$CLONE/" "$STAGE/"

# Optional: generate config_kernel.h if tools exist (non-fatal)
if [[ -x "$STAGE/tools/configure_kernel.sh" ]]; then
  echo "prepare: found tools/configure_kernel.sh — run it in the stage tree if build needs config_kernel.h"
  echo "  (cd \"$STAGE\" && ./tools/configure_kernel.sh)   # may need root/headers"
elif [[ -f "$STAGE/tools/configure_kernel.sh" ]]; then
  echo "prepare: tools/configure_kernel.sh present (not executable bit); review before use"
fi

cp -f "${DKMS_DIR}/dkms.conf" "${STAGE}/dkms.conf"

clone_head="n/a"
if [[ -d "$CLONE/.git" ]]; then
  clone_head=$(git -C "$CLONE" rev-parse HEAD 2>/dev/null || echo "n/a")
fi

cat >"${STAGE}/README.dragon-npu-stage" <<EOF
Staged by dragon-npu L4 prepare_from_clone.sh
date: $(date -Is 2>/dev/null || date)
clone: $CLONE
clone_HEAD: $clone_head
expected_base_SHA: $EXPECTED_SHA
package: ${PKG_NAME}-${PKG_VER}
patches: L2 0001 + L3 0002 (l2_ok=$l2_ok l3_ok=$l3_ok)
host_kernel: $(uname -r)
next: see kernel/dkms/README.md
EOF

echo "prepare: staged at $STAGE"
echo "prepare: next — see kernel/dkms/README.md (dkms add/build/install as root after clean boot)"
exit 0
