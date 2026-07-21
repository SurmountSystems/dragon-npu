#!/usr/bin/env bash
# Polished operator kit for external known-good capture (path B) — no time waste.
#
# What this is for:
#   Firmware still rejects our lab load (0x3000003) after ELF+rela OK.
#   Soft compare tools need a **real** post-patch ctrltext from a stack that
#   actually completes. Put that under:
#     testdata/xdna2_cmd/external_known_good/
#   with PROVENANCE filled. This script **never invents bytes**.
#
# Modes (pick one; default is checklist + validate):
#   bash tools/operator_external_capture.sh              # checklist + bag status
#   bash tools/operator_external_capture.sh --status     # is the capture dir ready?
#   bash tools/operator_external_capture.sh --print-steps  # exact commands only
#   bash tools/operator_external_capture.sh --validate PATH.bin
#       # check one file + PROVENANCE; refuse local-oracle basenames
#   bash tools/operator_external_capture.sh --install PATH.bin [BASENAME]
#       # copy into capture dir (safe basename), print next compare commands
#   bash tools/operator_external_capture.sh --compare
#       # soft classify last_apply with external path if bag has a .bin
#
# Never: opens L3, sets XDNA2_ALLOW_EXEC_CMD, claims Pass, second C_UNLOCK.
#
# Exit: 0 ok / ready / soft compare ran
#       1 blocked (no capture bytes yet) — expected until you drop a file
#       2 bad args / refused basename / missing inputs
set -euo pipefail
export PATH="/usr/bin:${HOME}/.local/bin:${PATH}"
unset XDNA2_ALLOW_EXEC_CMD || true

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

BAG_DIR="$ROOT/testdata/xdna2_cmd/external_known_good"
PROV="$BAG_DIR/PROVENANCE.md"
LAST_APPLY="${XDNA2_LAB_LAST_APPLY_JSON:-$ROOT/testdata/xdna2_cmd/last_apply.json}"

MODE="default"
INSTALL_SRC=""
INSTALL_NAME=""
VALIDATE_PATH=""

# Basenames that loaders refuse as "external known-good" (local oracles).
refuse_basename() {
  local b
  b="$(basename "$1")"
  case "$b" in
    instr_post_patch.bin|instr_vendor_post_patch.bin|ctrl_post_patch.bin|\
    instr_raw_txn.bin|instr_vendor_txn_bin.bin|lab_instr.bin|lab_instr.elf)
      return 0
      ;;
  esac
  return 1
}

bag_bins() {
  # Non-empty .bin / .ctrltext under bag dir (not following into weird trees).
  find "$BAG_DIR" -maxdepth 1 -type f \( -name '*.bin' -o -name '*.ctrltext' \) 2>/dev/null \
    | while read -r f; do
        [[ -s "$f" ]] || continue
        if refuse_basename "$f"; then
          continue
        fi
        echo "$f"
      done
}

print_checklist() {
  cat <<'EOF'
=== External capture checklist (path B) — do once when a public stack works ===

Goal: one non-empty post-patch ctrltext file under
  testdata/xdna2_cmd/external_known_good/
plus PROVENANCE capture table filled. Research only — not Pass.

Steps:
  1) Baseline: exec_cmd_allow must be 0/N when idle
       cat /sys/module/amdxdna/parameters/exec_cmd_allow
  2) Open execute-allow ONLY for a known-working public path (Ryzen AI / XRT /
     DynamicDispatch / eepy VitisAI that actually hits the NPU). Restore to 0
     immediately after.
  3) Capture the instruction image that went to the instr BO after host
     peel/patch (ctrltext), or aiebu ELF and document the peel.
  4) Install into the capture dir with a clear name:
       bash tools/operator_external_capture.sh --install /path/to/captured.bin working_stack_ctrltext.bin
  5) Edit PROVENANCE capture table (date, source stack, sha256, CONFIG_CU?).
  6) Soft compare:
       bash tools/operator_external_capture.sh --compare
  7) Paste output to the agent. Match ≠ residual closed ≠ Pass.

Refuse as external known-good (local oracles):
  instr_post_patch.bin, instr_vendor_post_patch.bin, ctrl_post_patch.bin,
  instr_raw_txn.bin, instr_vendor_txn_bin.bin, lab_instr.bin/.elf

Related path A (CONFIG_CU machinery, offline):
  bash tools/dump_xclbin_pdi.sh --print-env
  # then lab apply with XDNA2_LAB_PDI_BLOB set (needs new C_UNLOCK + L3 sandwich)

Unlock C_UNLOCK=b17b0119 is CONSUMED. Further live apply needs a new unlock
after a real path delta — do not reopen under the old stamp.
EOF
}

print_steps_compact() {
  cat <<EOF
# --- status ---
bash tools/operator_external_capture.sh --status

# --- after you have a working-stack ctrltext file ---
bash tools/operator_external_capture.sh --install /path/to/captured.bin working_stack_ctrltext.bin
# edit: $PROV  (capture table)

# --- soft compare (no L3 / no ALLOW) ---
bash tools/operator_external_capture.sh --compare

# --- path A prep (CONFIG_CU opaque PDI) ---
bash tools/dump_xclbin_pdi.sh --print-env
EOF
}

status_bag() {
  echo "=== Capture dir status ==="
  echo "dir=$BAG_DIR"
  if [[ ! -d "$BAG_DIR" ]]; then
    echo "MISSING_DIR"
    return 1
  fi
  if [[ -f "$PROV" ]]; then
    echo "PROVENANCE=present"
  else
    echo "PROVENANCE=MISSING"
  fi
  local n=0
  local first=""
  while IFS= read -r f; do
    n=$((n + 1))
    [[ -z "$first" ]] && first="$f"
    local b sha
    b=$(basename "$f")
    sha=$(sha256sum "$f" | awk '{print $1}')
    echo "BIN  $b  bytes=$(wc -c <"$f" | tr -d ' ')  sha256=$sha"
  done < <(bag_bins)
  if [[ "$n" -eq 0 ]]; then
    echo "BINS=0  blocked_on_external_bytes=1"
    echo "NEXT: place a real capture (see --print-steps) or use path A dump_xclbin_pdi."
    echo "Honesty: invent_pass=0 not_c_ready=1 not_ran_on_npu=1 product_known_good=0"
    return 1
  fi
  echo "BINS=$n  first=$first"
  echo "Honesty: invent_pass=0 not_c_ready=1 not_ran_on_npu=1 product_known_good=0"
  echo "bag_drop_alone_closes_fw_residual=0"
  return 0
}

validate_one() {
  local p="$1"
  if [[ ! -f "$p" ]]; then
    echo "ERROR: file missing: $p" >&2
    return 2
  fi
  if [[ ! -s "$p" ]]; then
    echo "ERROR: empty file: $p" >&2
    return 2
  fi
  if refuse_basename "$p"; then
    echo "ERROR: refused basename (local oracle): $(basename "$p")" >&2
    echo "  pick a name like working_stack_ctrltext.bin" >&2
    return 2
  fi
  local b
  b=$(basename "$p")
  if [[ "$b" == *ve2* || "$b" == *NOT_AIE2* ]]; then
    echo "ERROR: VE2 / NOT_AIE2 name refused as AIE2 known-good" >&2
    return 2
  fi
  echo "VALIDATE_OK path=$p bytes=$(wc -c <"$p" | tr -d ' ') sha256=$(sha256sum "$p" | awk '{print $1}')"
  echo "NOTE: research-only; product_known_good=0 invent_pass=0"
  return 0
}

install_one() {
  local src="$1"
  local name="${2:-working_stack_ctrltext.bin}"
  validate_one "$src" || return $?
  if refuse_basename "$name"; then
    echo "ERROR: install basename refused: $name" >&2
    return 2
  fi
  mkdir -p "$BAG_DIR"
  local dest="$BAG_DIR/$name"
  if [[ -e "$dest" ]]; then
    echo "ERROR: refuse clobber $dest (remove or pick another basename)" >&2
    return 2
  fi
  cp -n -- "$src" "$dest"
  chmod 0644 "$dest" || true
  echo "INSTALLED $dest"
  echo "bytes=$(wc -c <"$dest" | tr -d ' ') sha256=$(sha256sum "$dest" | awk '{print $1}')"
  echo
  echo "NEXT:"
  echo "  1) Fill capture table in $PROV"
  echo "  2) bash tools/operator_external_capture.sh --compare"
  echo "  3) Paste compare output for the agent"
  return 0
}

run_compare() {
  local bins first
  first=""
  while IFS= read -r f; do
    first="$f"
    break
  done < <(bag_bins)
  if [[ -z "$first" ]]; then
    echo "COMPARE_BLOCKED blocked_on_external_bytes=1"
    echo "No .bin/.ctrltext in $BAG_DIR yet."
    echo "Honesty: invent_pass=0 not_c_ready=1 not_ran_on_npu=1"
    return 1
  fi
  if [[ ! -f "$LAST_APPLY" ]]; then
    echo "ERROR: last_apply missing: $LAST_APPLY" >&2
    echo "  set XDNA2_LAB_LAST_APPLY_JSON or ensure testdata fixture exists" >&2
    return 2
  fi
  echo "=== Soft external compare ==="
  echo "external=$first"
  echo "last_apply=$LAST_APPLY"
  export XDNA2_LAB_PRINT_C2_CMDBUF_RESEARCH=1
  export XDNA2_LAB_EXTERNAL_CTRLTEXT="$first"
  # Prefer shell classify if present; else cargo example.
  if [[ -x "$ROOT/tools/classify_last_apply_residual.sh" ]] || [[ -f "$ROOT/tools/classify_last_apply_residual.sh" ]]; then
    bash "$ROOT/tools/classify_last_apply_residual.sh" "$LAST_APPLY" || true
  else
    cargo run --features amd-xdna2 --example classify_last_apply --quiet -- \
      "$LAST_APPLY" || true
  fi
  echo
  echo "Read: external_ctrltext_compare= … product_known_good=0 invent_pass=0"
  echo "Match ≠ residual closed ≠ Pass ≠ second C_UNLOCK"
  unset XDNA2_LAB_EXTERNAL_CTRLTEXT || true
  return 0
}

# --- argv ---
while [[ $# -gt 0 ]]; do
  case "$1" in
    --status) MODE="status"; shift ;;
    --print-steps) MODE="steps"; shift ;;
    --validate)
      MODE="validate"
      VALIDATE_PATH="${2:-}"
      shift 2
      ;;
    --install)
      MODE="install"
      INSTALL_SRC="${2:-}"
      INSTALL_NAME="${3:-working_stack_ctrltext.bin}"
      if [[ $# -ge 3 ]]; then shift 3; else shift 2; fi
      ;;
    --compare) MODE="compare"; shift ;;
    -h|--help)
      grep -E '^#([^!]|$)' "$0" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      echo "ERROR: unknown arg: $1" >&2
      exit 2
      ;;
  esac
done

case "$MODE" in
  default)
    print_checklist
    echo
    status_bag || true
    exit 0
    ;;
  status)
    if status_bag; then exit 0; else exit 1; fi
    ;;
  steps)
    print_steps_compact
    exit 0
    ;;
  validate)
    if [[ -z "$VALIDATE_PATH" ]]; then
      echo "ERROR: --validate needs PATH" >&2
      exit 2
    fi
    validate_one "$VALIDATE_PATH"
    exit $?
    ;;
  install)
    if [[ -z "$INSTALL_SRC" ]]; then
      echo "ERROR: --install needs SOURCE path" >&2
      exit 2
    fi
    install_one "$INSTALL_SRC" "$INSTALL_NAME"
    exit $?
    ;;
  compare)
    run_compare
    exit $?
    ;;
  *)
    echo "ERROR: internal mode=$MODE" >&2
    exit 2
    ;;
esac
