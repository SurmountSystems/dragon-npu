#!/usr/bin/env bash
# Host NPU stack inventory (read-only; W4a T1 + T8 who-acts).
#
# Purpose: one offline probe of what is on this machine for NPU bring-up and
# capture hunting - device nodes, tools, install roots, sample bins/docs, soft
# journal keywords, and a plain-English "who acts next" line.
#
# Usage:
#   bash tools/host_npu_stack_inventory.sh
#   bash tools/host_npu_stack_inventory.sh --json-summary
#   bash tools/host_npu_stack_inventory.sh -h|--help
#
# Always exit 0 (soft inventory). Never: sudo, modprobe, EXEC_CMD, ALLOW=1,
# invent capture bytes, curl|sh, pip install, download, open L3.
#
# Footer always:
#   invent_pass=0 not_ran_on_npu=1 not_c_ready=1 Pass=No
#
# Related:
#   bash tools/pass_unblock_once.sh --status
#   bash tools/pass_unblock_fingerprint_once.sh --inventory-only
#   bash tools/host_capture_scan.sh              # W5-2 capture-like file scan
#   bash tools/collect_load_accepted_candidate.sh PATH  # W5-4 fail-closed collect
set -euo pipefail
export PATH="/usr/bin:${HOME}/.local/bin:${PATH:-}"
unset XDNA2_ALLOW_EXEC_CMD || true

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
BAG_DIR="$ROOT/testdata/xdna2_cmd/external_known_good"
REPORT_MD="$ROOT/.agents/reports/wave4-host-inventory.md"
JSON_SUMMARY=0
JOURNAL_HOURS="${XDNA2_HOST_INV_JOURNAL_HOURS:-6}"
# Bound walks so a huge install tree cannot hang the agent.
MAXDEPTH_INSTALL=8
MAXDEPTH_SAMPLES=6
# Cap listed sample paths (still count more if found).
SAMPLE_LIST_CAP=24
CANDIDATE_LIST_CAP=16

usage() {
  cat <<'EOF'
host_npu_stack_inventory - read-only host NPU stack probe (W4a T1 + T8)

Usage:
  bash tools/host_npu_stack_inventory.sh
  bash tools/host_npu_stack_inventory.sh --json-summary
  bash tools/host_npu_stack_inventory.sh -h|--help

What it does (all soft / exit 0)
 - List /dev/accel* and /sys/class/accel (no open for EXEC)
 - Firmware / module params if readable
 - which: xrt-smi, xclbinutil, xbutil, aiebu tools, etc.
 - Probe known install roots (ryzen-ai-install, RyzenAI, XRT samples)
 - Sample binaries / docs on disk (paths+sizes only; no copy into git)
 - Soft journal keywords last N hours if journalctl is user-readable
 - Completing-stack capture candidates (host paths that look capturable)
 - Plain English "who acts next" (T8)

Honesty
  invent_pass=0 not_ran_on_npu=1 not_c_ready=1 Pass=No
  Inventory ≠ Pass. Candidate path ≠ residual match ≠ load-accepted.
  Research-only public AOT already under the capture dir is not a new candidate.

Env
  XDNA2_HOST_INV_JOURNAL_HOURS  journal lookback hours (default 6)
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --json-summary)
      JSON_SUMMARY=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "ERROR: unknown arg: $1 (allowlist: --json-summary --help)" >&2
      exit 2
      ;;
  esac
done

# --- counters / collected lines (stdout + report) ---
OUT_BUF=""
emit() {
  printf '%s\n' "$*"
  OUT_BUF+="$*"$'\n'
}

ACCEL_DEV_N=0
ACCEL_SYS_N=0
TOOLS_PRESENT_N=0
INSTALL_ROOTS_PRESENT_N=0
SAMPLE_BINS_N=0
CANDIDATE_N=0
JOURNAL_HITS=0
JOURNAL_OK=0
FW_NOTE="absent_or_unreadable"
EXEC_CMD_ALLOW_NOTE="missing"
MODULE_AMDXDNA="absent"

# Candidate paths (newline-separated; paths only).
CANDIDATE_PATHS=""
SAMPLE_PATHS=""

# Known install / sample roots (missing OK).
INSTALL_ROOTS=(
  "${HOME}/Projects/ai/ryzen-ai-install"
  "${HOME}/Projects/ai/RyzenAI-SW"
  "${HOME}/Projects/ai/RyzenAI"
  "${HOME}/ryzen-ai"
  "/opt/xilinx/xrt"
  "/opt/xilinx"
  "/opt/amd/ryzen-ai"
  "/usr/lib/xrt"
  "/usr/share/xrt"
)

# Extra roots that often hold capture-like material (read-only).
SAMPLE_ROOTS=(
  "${HOME}/Projects/ai/ryzen-ai-install"
  "${HOME}/Projects/ai/RyzenAI-SW"
  "${HOME}/Projects/ai/RyzenAI"
  "${HOME}/xrt"
  "/opt/xilinx/xrt/share"
  "/usr/share/xrt"
)

# Tools we care about (command names).
TOOL_NAMES=(
  xrt-smi
  xclbinutil
  xbutil
  xrt-smi
  aiebu-asm
  aiebu-dump
  aiecompiler
  xchesscc
  vitis
  xrt
)

# Completing-stack candidate basenames / globs (host capture material).
# Not research-only public mlir-aie already in bag. Not inventing bytes.
# Agent may attempt capture/ingest from these when present outside bag.
is_candidate_basename() {
  local b bl
  b="$(basename "$1")"
  bl="$(printf '%s' "$b" | tr '[:upper:]' '[:lower:]')"
  case "$bl" in
    # Explicit working-stack / vendor capture names
    working_stack_ctrltext.bin|working_stack*.bin|instr_vendor_post_patch.bin|\
    instr_post_patch.bin|ctrl_post_patch.bin|vendor_paddrs*.json)
      return 0
      ;;
    # Generic ctrltext dumps that may be load-path material (size checked later)
    *ctrltext*.bin|*_ctrltext.bin|*.ctrltext)
      return 0
      ;;
  esac
  return 1
}

# Refuse local oracle / wrong-platform basenames as "completing" candidates.
refuse_as_completing() {
  local b bl
  b="$(basename "$1")"
  bl="$(printf '%s' "$b" | tr '[:upper:]' '[:lower:]')"
  case "$bl" in
    lab_instr.bin|lab_instr.elf|validate.xclbin|instr_raw_txn.bin|\
    instr_vendor_txn_bin.bin)
      return 0
      ;;
  esac
  case "$bl" in
    *ve2*|*not_aie2*)
      return 0
      ;;
  esac
  return 1
}

# Path is under the in-tree capture dir (already bagged research ≠ new host candidate).
path_under_bag() {
  local p="$1"
  case "$p" in
    "$BAG_DIR"/*|"$BAG_DIR")
      return 0
      ;;
  esac
  return 1
}

# Path is under this repo (in-tree research material already tracked).
path_under_repo() {
  local p="$1"
  case "$p" in
    "$ROOT"/*|"$ROOT")
      return 0
      ;;
  esac
  return 1
}

add_candidate() {
  local f="$1"
  local sz
  [[ -f "$f" && -s "$f" ]] || return 0
  if refuse_as_completing "$f"; then
    return 0
  fi
  if path_under_bag "$f"; then
    return 0
  fi
  # Prefer host/out-of-tree completing material; in-tree lab/gt is research.
  if path_under_repo "$f"; then
    case "$(basename "$f")" in
      instr_vendor_post_patch.bin|working_stack_ctrltext.bin)
        ;;
      *)
        return 0
        ;;
    esac
  fi
  if ! is_candidate_basename "$f"; then
    return 0
  fi
  sz="$(wc -c <"$f" 2>/dev/null | tr -d ' ' || echo 0)"
  # Tiny stubs (< 64 B) are not useful captures.
  if [[ "${sz:-0}" -lt 64 ]]; then
    return 0
  fi
  CANDIDATE_N=$((CANDIDATE_N + 1))
  if [[ "$CANDIDATE_N" -le "$CANDIDATE_LIST_CAP" ]]; then
    CANDIDATE_PATHS+="${f}"$'\t'"${sz}"$'\n'
  fi
}

emit "================================================================"
emit "host_npu_stack_inventory - read-only (W4a T1 + T8)"
emit "================================================================"
emit "ROOT=$ROOT"
emit "date=$(date -Iseconds 2>/dev/null || date)"
emit "Honesty: invent_pass=0 not_ran_on_npu=1 not_c_ready=1 Pass=No"
emit "No sudo / no modprobe / no EXEC_CMD / no ALLOW / no invent bytes"
emit ""

# ---------------------------------------------------------------------------
# Device nodes + sysfs
# ---------------------------------------------------------------------------
emit "=== /dev/accel* ==="
if compgen -G '/dev/accel*' >/dev/null 2>&1; then
  while IFS= read -r d; do
    [[ -e "$d" ]] || continue
    ACCEL_DEV_N=$((ACCEL_DEV_N + 1))
    if [[ -r "$d" ]]; then
      emit "  present readable: $d"
    else
      emit "  present (not readable by user): $d"
    fi
  done < <(compgen -G '/dev/accel*' | sort || true)
else
  emit "  none"
fi

emit ""
emit "=== /sys/class/accel ==="
if [[ -d /sys/class/accel ]]; then
  while IFS= read -r -d '' s; do
    ACCEL_SYS_N=$((ACCEL_SYS_N + 1))
    emit "  $(basename "$s") -> $(readlink -f "$s" 2>/dev/null || echo "$s")"
  done < <(find /sys/class/accel -mindepth 1 -maxdepth 1 -print0 2>/dev/null || true)
  if [[ "$ACCEL_SYS_N" -eq 0 ]]; then
    emit "  dir present, no children"
  fi
else
  emit "  missing"
fi

emit ""
emit "=== amdxdna module / firmware (soft) ==="
if [[ -d /sys/module/amdxdna ]]; then
  MODULE_AMDXDNA="present"
  emit "  module=present"
  if [[ -r /sys/module/amdxdna/parameters/exec_cmd_allow ]]; then
    EXEC_CMD_ALLOW_NOTE="$(tr -d '[:space:]' </sys/module/amdxdna/parameters/exec_cmd_allow 2>/dev/null || echo unreadable)"
    emit "  exec_cmd_allow=$EXEC_CMD_ALLOW_NOTE"
  else
    EXEC_CMD_ALLOW_NOTE="missing"
    emit "  exec_cmd_allow=missing (stock path; no L3 knob)"
  fi
  if [[ -r /sys/module/amdxdna/version ]]; then
    emit "  module_version=$(tr -d '[:space:]' </sys/module/amdxdna/version 2>/dev/null || true)"
  fi
else
  MODULE_AMDXDNA="absent"
  emit "  module=absent (or not loaded)"
fi

# Firmware blobs often live under /lib/firmware
FW_CANDIDATES=(
  /lib/firmware/amdnpu
  /lib/firmware/amdxdna
  /lib/firmware/updates/amdnpu
  /lib/firmware/updates/amdxdna
)
fw_found=0
for fw in "${FW_CANDIDATES[@]}"; do
  if [[ -e "$fw" ]]; then
    fw_found=1
    if [[ -d "$fw" ]]; then
      emit "  firmware_dir=$fw"
      _fw_n=0
      while IFS= read -r -d '' ff; do
        _fw_n=$((_fw_n + 1))
        [[ "$_fw_n" -le 12 ]] || break
        sz="$(wc -c <"$ff" 2>/dev/null | tr -d ' ' || echo 0)"
        emit "    bytes=${sz}  $(basename "$ff")"
      done < <(find "$fw" -maxdepth 3 -type f -print0 2>/dev/null || true)
    else
      sz="$(wc -c <"$fw" 2>/dev/null | tr -d ' ' || echo 0)"
      emit "  firmware_file bytes=${sz} path=$fw"
    fi
  fi
done
if [[ "$fw_found" -eq 1 ]]; then
  FW_NOTE="present"
else
  FW_NOTE="absent_or_unreadable"
  emit "  firmware=absent_or_unreadable under common /lib/firmware paths"
fi

# ---------------------------------------------------------------------------
# Tools on PATH
# ---------------------------------------------------------------------------
emit ""
emit "=== tools on PATH (which) ==="
# Dedupe tool names (xrt-smi listed once conceptually)
declare -A SEEN_TOOL=()
for t in "${TOOL_NAMES[@]}"; do
  [[ -n "${SEEN_TOOL[$t]:-}" ]] && continue
  SEEN_TOOL[$t]=1
  if command -v "$t" >/dev/null 2>&1; then
    TOOLS_PRESENT_N=$((TOOLS_PRESENT_N + 1))
    p="$(command -v "$t" 2>/dev/null || true)"
    emit "  present: $t -> $p"
  else
    emit "  missing: $t"
  fi
done

# ---------------------------------------------------------------------------
# Install roots
# ---------------------------------------------------------------------------
emit ""
emit "=== known install / sample roots ==="
for root in "${INSTALL_ROOTS[@]}"; do
  if [[ -d "$root" ]]; then
    INSTALL_ROOTS_PRESENT_N=$((INSTALL_ROOTS_PRESENT_N + 1))
    emit "  present: $root"
  else
    emit "  missing: $root"
  fi
done

# ---------------------------------------------------------------------------
# Sample binaries / docs on disk (paths+sizes only)
# ---------------------------------------------------------------------------
emit ""
emit "=== sample binaries / docs on disk (bounded; no copy) ==="
# Patterns of interest: xclbin, pdi, ctrltext, transaction samples, README
sample_find() {
  local root="$1"
  [[ -d "$root" ]] || return 0
  # Prefer shallow useful names; bound depth + count.
  find "$root" -maxdepth "$MAXDEPTH_SAMPLES" -type f \( \
      -iname '*.xclbin' -o -iname '*.pdi' -o -iname '*ctrltext*' \
      -o -iname 'instr_vendor_post_patch.bin' -o -iname 'working_stack*.bin' \
      -o -iname '*transaction*.bin' -o -iname 'README*' -o -iname 'PROVENANCE*' \
    \) -print0 2>/dev/null || true
}

while IFS= read -r -d '' f; do
  SAMPLE_BINS_N=$((SAMPLE_BINS_N + 1))
  sz="$(wc -c <"$f" 2>/dev/null | tr -d ' ' || echo 0)"
  if [[ "$SAMPLE_BINS_N" -le "$SAMPLE_LIST_CAP" ]]; then
    SAMPLE_PATHS+="${f}"$'\t'"${sz}"$'\n'
    emit "  SAMPLE bytes=${sz}  path=${f}"
  fi
  # Also score completing-stack candidates while walking.
  add_candidate "$f"
done < <(
  for root in "${SAMPLE_ROOTS[@]}"; do
    sample_find "$root"
  done
)

if [[ "$SAMPLE_BINS_N" -eq 0 ]]; then
  emit "  none found under common sample roots (ok soft)"
elif [[ "$SAMPLE_BINS_N" -gt "$SAMPLE_LIST_CAP" ]]; then
  emit "  ... listed first ${SAMPLE_LIST_CAP} of ${SAMPLE_BINS_N}"
fi

# Explicit extra candidate hunt (gt_capture in-tree + common host capture names)
emit ""
emit "=== completing-stack capture candidates (host / out-of-bag) ==="
# In-tree gt_capture vendor post_patch is the classic HITL capture target.
GT_CAPTURE="$ROOT/testdata/xdna2_cmd/gt_capture"
if [[ -d "$GT_CAPTURE" ]]; then
  while IFS= read -r -d '' f; do
    add_candidate "$f"
  done < <(find "$GT_CAPTURE" -maxdepth 2 -type f -print0 2>/dev/null || true)
fi
# Host temp last_apply pins are evidence, not capture bins - skip.
# Walk install roots deeper for explicit post_patch / working_stack names only.
for root in "${INSTALL_ROOTS[@]}"; do
  [[ -d "$root" ]] || continue
  while IFS= read -r -d '' f; do
    add_candidate "$f"
  done < <(
    find "$root" -maxdepth "$MAXDEPTH_INSTALL" -type f \( \
        -iname 'instr_vendor_post_patch.bin' \
        -o -iname 'working_stack*.bin' \
        -o -iname 'working_stack_ctrltext.bin' \
        -o -iname '*ctrltext*.bin' \
      \) -print0 2>/dev/null || true
  )
done

if [[ "$CANDIDATE_N" -eq 0 ]]; then
  emit "  candidates=0 (none suitable for agent capture attempt)"
else
  emit "  candidates=${CANDIDATE_N} (paths below; not residual-matched; invent_pass=0)"
  while IFS=$'\t' read -r cp csz; do
    [[ -n "${cp:-}" ]] || continue
    emit "  CANDIDATE bytes=${csz}  path=${cp}"
  done <<<"$CANDIDATE_PATHS"
  if [[ "$CANDIDATE_N" -gt "$CANDIDATE_LIST_CAP" ]]; then
    emit "  ... listed first ${CANDIDATE_LIST_CAP} of ${CANDIDATE_N}"
  fi
fi

# Bag status (in-tree research material already present)
emit ""
emit "=== in-tree capture dir (testdata/xdna2_cmd/external_known_good/) ==="
bag_bins=0
if [[ -d "$BAG_DIR" ]]; then
  while IFS= read -r f; do
    [[ -s "$f" ]] || continue
    bag_bins=$((bag_bins + 1))
  done < <(find "$BAG_DIR" -maxdepth 1 -type f \( -name '*.bin' -o -name '*.ctrltext' -o -name '*.pdi' -o -name '*.xclbin' -o -name '*.elf' \) 2>/dev/null || true)
  emit "  dir=present files=${bag_bins}"
  if [[ -f "$BAG_DIR/PROVENANCE.md" ]]; then
    emit "  PROVENANCE=present"
  else
    emit "  PROVENANCE=MISSING"
  fi
  emit "  note: research-only public AOT / cascade already here ≠ load-accepted residual match ≠ Pass"
else
  emit "  dir=MISSING"
fi

# ---------------------------------------------------------------------------
# Soft journal keywords
# ---------------------------------------------------------------------------
emit ""
emit "=== soft journal keywords (last ${JOURNAL_HOURS}h; user-readable only) ==="
if command -v journalctl >/dev/null 2>&1; then
  set +e
  jout="$(
    journalctl -k --since "${JOURNAL_HOURS} hours ago" --no-pager 2>/dev/null \
      | rg -i 'IO_PAGE_FAULT|APP_LOAD|wait_cmd|aie2_set_cmd_timeout|amdxdna|0x3000003|Failed cmd' \
      | tail -n 20
  )"
  set -e
  # journalctl may fail if no permission; treat as soft skip
  if [[ -n "${jout//[$'\t\n\r ']/}" ]]; then
    JOURNAL_OK=1
    JOURNAL_HITS="$(printf '%s\n' "$jout" | grep -c . || true)"
    emit "  journal_ok=1 hits≈${JOURNAL_HITS} (tail 20 lines)"
    while IFS= read -r line; do
      [[ -n "$line" ]] || continue
      emit "  J: $line"
    done <<<"$jout"
  else
    JOURNAL_OK=0
    emit "  journal_ok=0 (no matching lines or not readable - soft skip)"
  fi
  unset jout || true
else
  emit "  journalctl=missing"
fi

# ---------------------------------------------------------------------------
# English summary + T8 who acts next
# ---------------------------------------------------------------------------
emit ""
emit "=== English summary ==="
emit "  accel devices: ${ACCEL_DEV_N}  sysfs accel: ${ACCEL_SYS_N}  amdxdna: ${MODULE_AMDXDNA}"
emit "  tools present: ${TOOLS_PRESENT_N}  install roots present: ${INSTALL_ROOTS_PRESENT_N}"
emit "  sample files listed/count: min(${SAMPLE_BINS_N},${SAMPLE_LIST_CAP})/${SAMPLE_BINS_N}"
emit "  firmware: ${FW_NOTE}  exec_cmd_allow: ${EXEC_CMD_ALLOW_NOTE}"
emit "  in-tree capture dir files: ${bag_bins}  completing-stack candidates: ${CANDIDATE_N}"
if [[ "$CANDIDATE_N" -eq 0 ]]; then
  emit "  Capture candidates: none on host for agent capture attempt."
else
  emit "  Capture candidates: ${CANDIDATE_N} host path(s) may hold capturable material (not proven load-accepted)."
fi

emit ""
emit "=== Who acts next (T8) ==="
WHO_ACTS=""
if [[ "$CANDIDATE_N" -eq 0 ]]; then
  WHO_ACTS="operator: drop load-accepted capture under testdata/xdna2_cmd/external_known_good/"
  emit "  ${WHO_ACTS}"
  emit "  (Agent cannot invent load-accepted bytes. Research-only public AOT already in the capture dir is not residual match.)"
else
  # First candidate path for the agent line
  first_cand=""
  while IFS=$'\t' read -r cp _csz; do
    [[ -n "${cp:-}" ]] || continue
    first_cand="$cp"
    break
  done <<<"$CANDIDATE_PATHS"
  if [[ -n "$first_cand" ]]; then
    WHO_ACTS="agent: attempt capture from ${first_cand}"
  else
    WHO_ACTS="agent: attempt capture from listed CANDIDATE paths (${CANDIDATE_N})"
  fi
  emit "  ${WHO_ACTS}"
  emit "  Also: operator may still place a true load-accepted working-stack capture under the capture dir."
  if [[ "$CANDIDATE_N" -gt 1 ]]; then
    emit "  Additional candidates: $((CANDIDATE_N - 1)) more (see CANDIDATE lines above)."
  fi
fi
emit "  Live apply still needs active C_UNLOCK + named offline reason. Unlock ≠ Pass."
emit "  Capture scan (W5-2): bash tools/host_capture_scan.sh"
emit "  Collect helper (W5-4): bash tools/collect_load_accepted_candidate.sh PATH [NAME]"

emit ""
emit "=== Footer (honest) ==="
emit "invent_pass=0"
emit "not_ran_on_npu=1"
emit "not_c_ready=1"
emit "Pass=No"
emit "accel_dev_n=${ACCEL_DEV_N}"
emit "accel_sys_n=${ACCEL_SYS_N}"
emit "tools_present_n=${TOOLS_PRESENT_N}"
emit "install_roots_present_n=${INSTALL_ROOTS_PRESENT_N}"
emit "sample_bins_n=${SAMPLE_BINS_N}"
emit "candidate_n=${CANDIDATE_N}"
emit "bag_files_n=${bag_bins}"
emit "journal_ok=${JOURNAL_OK}"
emit "who_acts=${WHO_ACTS}"
emit "entrypoint=tools/host_npu_stack_inventory.sh"

if [[ "$JSON_SUMMARY" -eq 1 ]]; then
  # Minimal JSON (no external jq required).
  json_escape() {
    printf '%s' "$1" | sed 's/\\/\\\\/g; s/"/\\"/g'
  }
  emit ""
  emit "=== json-summary ==="
  printf '{'
  printf '"invent_pass":0,"not_ran_on_npu":1,"not_c_ready":1,"Pass":"No",'
  printf '"accel_dev_n":%s,"accel_sys_n":%s,"tools_present_n":%s,' \
    "$ACCEL_DEV_N" "$ACCEL_SYS_N" "$TOOLS_PRESENT_N"
  printf '"install_roots_present_n":%s,"sample_bins_n":%s,"candidate_n":%s,' \
    "$INSTALL_ROOTS_PRESENT_N" "$SAMPLE_BINS_N" "$CANDIDATE_N"
  printf '"bag_files_n":%s,"journal_ok":%s,' "$bag_bins" "$JOURNAL_OK"
  printf '"who_acts":"%s"' "$(json_escape "$WHO_ACTS")"
  printf '}\n'
  OUT_BUF+="$(printf '{"invent_pass":0,"candidate_n":%s,"who_acts":"%s"}\n' \
    "$CANDIDATE_N" "$(json_escape "$WHO_ACTS")")"
fi

# Write workspace report (primary durable artifact for this slice).
mkdir -p "$(dirname "$REPORT_MD")"
{
  echo "# wave4 host NPU stack inventory (W4a T1 + T8)"
  echo
  echo "- **Date:** $(date -Iseconds 2>/dev/null || date)"
  echo "- **Pass:** **No**"
  echo "- **invent_pass:** 0"
  echo "- **not_ran_on_npu:** 1"
  echo "- **not_c_ready:** 1"
  echo
  echo "## Counts"
  echo
  echo "| metric | value |"
  echo "|--------|-------|"
  echo "| accel devices (/dev/accel*) | ${ACCEL_DEV_N} |"
  echo "| sysfs accel | ${ACCEL_SYS_N} |"
  echo "| amdxdna module | ${MODULE_AMDXDNA} |"
  echo "| tools present | ${TOOLS_PRESENT_N} |"
  echo "| install roots present | ${INSTALL_ROOTS_PRESENT_N} |"
  echo "| sample files found | ${SAMPLE_BINS_N} |"
  echo "| completing-stack candidates | ${CANDIDATE_N} |"
  echo "| in-tree capture dir files | ${bag_bins} |"
  echo "| firmware | ${FW_NOTE} |"
  echo "| exec_cmd_allow | ${EXEC_CMD_ALLOW_NOTE} |"
  echo "| journal soft ok | ${JOURNAL_OK} |"
  echo
  echo "## Who acts next (T8)"
  echo
  echo "\`${WHO_ACTS}\`"
  echo
  if [[ "$CANDIDATE_N" -eq 0 ]]; then
    echo "No completing-stack candidates on host. Operator places load-accepted capture under \`testdata/xdna2_cmd/external_known_good/\`. Agent does not invent bytes."
  else
    echo "Agent may attempt capture from host candidate path(s). Candidate ≠ residual match ≠ Pass. Live still needs active unlock + named reason."
    echo
    echo "### Candidate paths"
    echo
    echo "| bytes | path |"
    echo "|-------|------|"
    while IFS=$'\t' read -r cp csz; do
      [[ -n "${cp:-}" ]] || continue
      echo "| ${csz} | \`${cp}\` |"
    done <<<"$CANDIDATE_PATHS"
  fi
  echo
  echo "## Honesty"
  echo
  echo "- invent_pass=0"
  echo "- Pass=No"
  echo "- not_ran_on_npu=1"
  echo "- not_c_ready=1"
  echo "- Read-only host probes only (no sudo, modprobe, EXEC_CMD, ALLOW)."
  echo "- In-tree research AOT under the capture dir is not a new completing-stack candidate."
  echo
  echo "## Full stdout log"
  echo
  echo '```'
  printf '%s' "$OUT_BUF"
  echo '```'
} >"$REPORT_MD"

emit ""
emit "report_md=${REPORT_MD}"

exit 0
