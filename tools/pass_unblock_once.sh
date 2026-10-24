#!/usr/bin/env bash
# One entry point: what unblocks product Pass next (Layer A autonomy).
#
# Full agent offline + status + capture ingest + stock live refuse-if-no-unlock.
# Not limited to fingerprint alone. Prefer this over multi-tool scavenger hunts.
#
# Usage:
#   bash tools/pass_unblock_once.sh                 # same as --status
#   bash tools/pass_unblock_once.sh --status
#   bash tools/pass_unblock_once.sh --agent-offline  # fingerprint pack + bag + residual
#   bash tools/pass_unblock_once.sh --ingest PATH [NAME]
#   bash tools/pass_unblock_once.sh --ingest-pdi PATH
#   bash tools/pass_unblock_once.sh --live           # refuse unless active C_UNLOCK
#   bash tools/pass_unblock_once.sh -h|--help
#
# Always: invent_pass=0, product_pass=0, ran_on_npu=0, Pass=No unless true live decide.
# Never opens L3 / never sets XDNA2_ALLOW_EXEC_CMD / never invents capture bytes.
# Privilege script is NOT this file — stock host needs no sudo for config-cu-run
# (exec_cmd_allow sysfs missing). Do not invent pass_unblock_privileged_once.sh
# until a real Layer A wall is proven.
#
# Exit:
#   0  status / agent-offline finished (soft; see fingerprint_rc=) / ingest ok / live ran
#   2  bad args / refused basename / path traversal / missing path
#   3  --live refused (no active unlock or no live path wired without unlock)
set -euo pipefail
export PATH="/usr/bin:${HOME}/.local/bin:${PATH}"
unset XDNA2_ALLOW_EXEC_CMD || true

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
TOOLS="$ROOT/tools"
BAG_DIR="$ROOT/testdata/xdna2_cmd/external_known_good"
PROV="$BAG_DIR/PROVENANCE.md"
RESIDUAL="$ROOT/RESIDUAL.md"
REPORT_MD="$ROOT/.agents/reports/pass-unblock-once.md"
# Never a fixed predictable /tmp path (symlink plant → overwrite victim).
# Optional /tmp copy is created with mktemp only at write time.
REPORT_TMP=""
FINGERPRINT_SH="$TOOLS/pass_unblock_fingerprint_once.sh"
CAPTURE_SH="$TOOLS/operator_external_capture.sh"
KIT_SH="$TOOLS/operator_app_load_kit.sh"
HOST_INV_SH="$TOOLS/host_npu_stack_inventory.sh"

MODE="status"
INGEST_PATH=""
INGEST_NAME=""
INGEST_PDI_PATH=""
FP_RC="" # set on agent-offline for footer

usage() {
  cat <<'EOF'
pass_unblock_once — one entry for Pass-unblock Layer A work

What this is for
  Product Pass still needs live ERT Completed + non-zero OFM allclose + real ioctl.
  This script is the single command for agent autonomy inside the workspace:
  honest status, offline fingerprint, capture ingest, optional PDI research dump,
  and stock live only when RESIDUAL has an active (non-CONSUMED) C_UNLOCK.

Commands
  --status            Default. Pass=No, unlock state, bag empty?, stock no-sudo note.
  --agent-offline     Offline fingerprint pack (spent ledger + skip spent re-dump
                      unless --force on fingerprint or XDNA2_FP_FORCE_REDUMP=1;
                      *mladf*.xclbin/*gemm*.xclbin inventory) + bag status +
                      residual pointer + report.
  --ingest PATH [NAME]
                      Install working-stack ctrltext into
                      testdata/xdna2_cmd/external_known_good/ (same refuse rules as
                      operator_external_capture --install). NAME = single path
                      component only. Exit 2 on refuse/missing/traversal.
  --ingest-pdi PATH   Fingerprint/copy research PDI dump to a unique mktemp path
                      under /tmp + report. Read-only source outside the tree is OK.
                      Not product known-good. Loud sha16; full sha256 only if
                      XDNA2_LAB_PRINT_FULL_SHA=1.
  --live              Stock config-cu-run only if active C_UNLOCK in RESIDUAL.md.
                      No unlock → exit 3. Never invents unlock. Never opens L3.
  -h, --help          This help.

Honesty (always)
  invent_pass=0 product_pass=0 ran_on_npu=0 Pass=No
  HUMAN_NEEDED=1     when capture bins=0 (human holds working-stack bytes);
                     agent offline fingerprint still runs regardless.
  PRIVILEGE_NEEDED=0|1  from probe: missing exec_cmd_allow → 0 (stock no-sudo);
                     present+0/N or unreadable → 1. Still no invented privilege script.

Not this script
  Privilege / sudo L3 sandwich (only if proven wall → separate privileged script).
  Re-running flexml live under CONSUMED unlock.
  Inventing capture bytes or minting unlock.

Related
  bash tools/pass_unblock_fingerprint_once.sh --offline
  bash tools/host_npu_stack_inventory.sh   # W4a host stack + who-acts (T8)
  bash tools/operator_app_load_kit.sh pass-unblock …
EOF
}

# --- helpers (mirror operator_external_capture refuse set for early refuse) ---
refuse_basename() {
  local b bl
  b="$(basename "$1")"
  case "$b" in
    instr_post_patch.bin|instr_vendor_post_patch.bin|ctrl_post_patch.bin|\
    instr_raw_txn.bin|instr_vendor_txn_bin.bin|lab_instr.bin|lab_instr.elf|\
    validate.xclbin)
      return 0
      ;;
  esac
  bl="$(printf '%s' "$b" | tr '[:upper:]' '[:lower:]')"
  case "$bl" in
    *ve2*|*not_aie2*)
      return 0
      ;;
  esac
  return 1
}

# Single-component install NAME (same rules as capture safe_install_name).
safe_install_name() {
  local name="$1"
  if [[ -z "$name" ]]; then
    return 1
  fi
  if [[ "$name" == /* || "$name" == *'/'* || "$name" == *'\'* ]]; then
    return 1
  fi
  if [[ "$name" == "." || "$name" == ".." || "$name" == *..* ]]; then
    return 1
  fi
  if [[ ! "$name" =~ ^[A-Za-z0-9][A-Za-z0-9._-]*$ ]]; then
    return 1
  fi
  return 0
}

count_bag_bins() {
  local n=0
  if [[ ! -d "$BAG_DIR" ]]; then
    echo 0
    return 0
  fi
  while IFS= read -r f; do
    [[ -s "$f" ]] || continue
    if refuse_basename "$f"; then
      continue
    fi
    n=$((n + 1))
  done < <(find "$BAG_DIR" -maxdepth 1 -type f \( -name '*.bin' -o -name '*.ctrltext' \) 2>/dev/null || true)
  echo "$n"
}

# First C_UNLOCK=… in RESIDUAL; active only when that line has no CONSUMED.
parse_active_unlock() {
  local active="" line
  if [[ -f "$RESIDUAL" ]]; then
    active="$(grep -m1 -oE 'C_UNLOCK=[0-9a-fA-F]+' "$RESIDUAL" 2>/dev/null || true)"
    line="$(grep -m1 -E 'C_UNLOCK=[0-9a-fA-F]+' "$RESIDUAL" 2>/dev/null || true)"
    if [[ -n "$line" && "$line" == *CONSUMED* ]]; then
      active=""
    fi
  fi
  printf '%s' "$active"
}

# Probe privilege need (honesty only; never invents a privilege script).
# 0 = stock path (missing knob) or allow already non-zero when readable
# 1 = hardened knob present and 0/N (or unreadable)
probe_privilege_needed() {
  local p="/sys/module/amdxdna/parameters/exec_cmd_allow"
  if [[ ! -e "$p" ]]; then
    # Stock / non-hardened: prior lives ran config-cu-run without sudo L3.
    echo 0
    return 0
  fi
  local v
  v="$(cat "$p" 2>/dev/null || true)"
  if [[ -z "$v" ]]; then
    echo 1
    return 0
  fi
  # Trim whitespace; treat 0 / N / n / false as blocked until flip.
  v="$(printf '%s' "$v" | tr -d '[:space:]')"
  case "$v" in
    0|N|n|false|FALSE)
      echo 1
      return 0
      ;;
  esac
  echo 0
}

# stock_no_sudo=1 when probe says privilege not needed for stock path.
stock_no_sudo_from_priv() {
  local priv="$1"
  if [[ "$priv" == "0" ]]; then
    echo 1
  else
    echo 0
  fi
}

print_footer() {
  local human="${1:-0}"
  local priv="${2:-0}"
  local bag_n="${3:-0}"
  local unlock_state="${4:-none}"
  local stock_ns
  stock_ns="$(stock_no_sudo_from_priv "$priv")"
  cat <<EOF

=== Footer (honest) ===
invent_pass=0
product_pass=0
ran_on_npu=0
Pass=No
C_UNLOCK_active=${unlock_state}
capture_bins=${bag_n}
capture_bytes_present=$([[ "$bag_n" -gt 0 ]] && echo 1 || echo 0)
driver_mmu=parked
stock_no_sudo_for_config_cu_run=${stock_ns}
flexml_live_2ae69036=CONSUMED_not_Pass
HUMAN_NEEDED=${human}
PRIVILEGE_NEEDED=${priv}
fingerprint_rc=${FP_RC:-n/a}
privilege_script=skipped_unless_proven_wall
outside_tree=read-only
entrypoint=tools/pass_unblock_once.sh
EOF
}

exit_refuse() {
  # Usage: exit_refuse RC "message lines..."
  local rc="$1"
  shift
  local line
  for line in "$@"; do
    echo "$line" >&2
  done
  print_footer "$HUMAN" "$PRIV" "$BAG_N" "$UNLOCK_STATE"
  exit "$rc"
}

# --- parse args ---
while [[ $# -gt 0 ]]; do
  case "$1" in
    --status)
      MODE="status"
      shift
      ;;
    --agent-offline|--offline)
      MODE="agent-offline"
      shift
      ;;
    --ingest)
      MODE="ingest"
      shift
      if [[ $# -lt 1 ]]; then
        # early: BAG_N/PRIV not yet set — set defaults for footer
        BAG_N=0
        HUMAN=1
        PRIV="$(probe_privilege_needed)"
        UNLOCK_STATE="none"
        exit_refuse 2 "ERROR: --ingest needs SOURCE path"
      fi
      INGEST_PATH="$1"
      shift
      if [[ $# -gt 0 && "$1" != -* ]]; then
        INGEST_NAME="$1"
        shift
      fi
      ;;
    --ingest-pdi)
      MODE="ingest-pdi"
      shift
      if [[ $# -lt 1 ]]; then
        BAG_N=0
        HUMAN=1
        PRIV="$(probe_privilege_needed)"
        UNLOCK_STATE="none"
        exit_refuse 2 "ERROR: --ingest-pdi needs SOURCE path"
      fi
      INGEST_PDI_PATH="$1"
      shift
      ;;
    --live)
      MODE="live"
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "ERROR: unknown arg: $1" >&2
      echo "allowlist: --status --agent-offline --ingest --ingest-pdi --live --help" >&2
      exit 2
      ;;
  esac
done

mkdir -p "$(dirname "$REPORT_MD")"
BAG_N="$(count_bag_bins)"
ACTIVE_UNLOCK="$(parse_active_unlock)"
UNLOCK_STATE="none"
if [[ -n "$ACTIVE_UNLOCK" ]]; then
  UNLOCK_STATE="$ACTIVE_UNLOCK"
else
  UNLOCK_STATE="none (first C_UNLOCK= is CONSUMED or absent)"
fi
PRIV="$(probe_privilege_needed)"

# HUMAN_NEEDED=1 when capture bins=0 (human holds working-stack bytes).
# Agent offline fingerprint still always available.
HUMAN=0
if [[ "$BAG_N" -eq 0 ]]; then
  HUMAN=1
fi

# ---------------------------------------------------------------------------
# --status
# ---------------------------------------------------------------------------
if [[ "$MODE" == "status" ]]; then
  echo "================================================================"
  echo "pass_unblock_once — STATUS"
  echo "================================================================"
  echo "Product Pass: No"
  echo "invent_pass=0  product_pass=0  ran_on_npu=0"
  echo ""
  echo "Unlock: $UNLOCK_STATE"
  echo "  CONSUMED (do not reopen): 2ae69036 (flexml), 97f38307, 11bc9016, 3a70fabf, b17b0119"
  echo "  Flexml live 2ae69036: attach measured, still wait_cmd_einval — not Pass; spent."
  echo ""
  echo "Capture dir: $BAG_DIR"
  if [[ -f "$PROV" ]]; then
    echo "  PROVENANCE=present"
  else
    echo "  PROVENANCE=MISSING"
  fi
  echo "  bins=$BAG_N  blocked_on_external_bytes=$([[ "$BAG_N" -eq 0 ]] && echo 1 || echo 0)"
  if [[ "$BAG_N" -eq 0 ]]; then
    echo "  NEXT capture: bash tools/pass_unblock_once.sh --ingest /path/to/working_stack_ctrltext.bin"
  fi
  echo ""
  echo "What unblocks Pass next (honest):"
  echo "  1) Working-stack ctrltext under testdata/xdna2_cmd/external_known_good/  (bag empty → human or --ingest)"
  echo "  2) True matmul-matched PDI/ctrltext beyond spent flexml  (agent offline discovery + new unlock if new FP)"
  echo "  3) Driver/MMU orphan class  (parked until new evidence)"
  echo ""
  echo "Stock host privilege:"
  if [[ ! -e /sys/module/amdxdna/parameters/exec_cmd_allow ]]; then
    echo "  exec_cmd_allow=missing → stock config-cu-run needs NO sudo L3 (proven prior lives)."
  else
    echo "  exec_cmd_allow=present → PRIVILEGE_NEEDED=$PRIV (1 if value 0/N or unreadable)."
  fi
  echo "  privilege_script=not invented (no Layer A wall on stock path without proof)."
  echo ""
  echo "Agent next (Layer A):"
  echo "  bash tools/pass_unblock_once.sh --agent-offline"
  if [[ -n "$ACTIVE_UNLOCK" ]]; then
    echo "  bash tools/pass_unblock_once.sh --live   # active unlock present"
  else
    echo "  --live refused until new host-minted unlock + named offline reason"
  fi
  echo "  Freeze empty reconfirm mill (AGENTS.md §6–9)."
  echo ""
  echo "Who acts next (T8 soft; full probe: bash tools/host_npu_stack_inventory.sh):"
  # Keep --status cheap: do not walk install trees here. Default operator line
  # unless a prior report has a who_acts line.
  _who=""
  if [[ -f "$ROOT/.agents/reports/wave4-host-inventory.md" ]]; then
    _who="$(grep -m1 -E '^who_acts=|^`?(operator|agent):' "$ROOT/.agents/reports/wave4-host-inventory.md" 2>/dev/null | head -n1 || true)"
  fi
  if [[ -n "${_who:-}" ]]; then
    echo "  ${_who#who_acts=}"
  else
    echo "  operator: drop load-accepted capture under testdata/xdna2_cmd/external_known_good/"
    echo "  (run inventory for host candidates: bash tools/host_npu_stack_inventory.sh)"
  fi
  unset _who || true
  print_footer "$HUMAN" "$PRIV" "$BAG_N" "$UNLOCK_STATE"
  exit 0
fi

# ---------------------------------------------------------------------------
# --agent-offline
# ---------------------------------------------------------------------------
if [[ "$MODE" == "agent-offline" ]]; then
  echo "================================================================"
  echo "pass_unblock_once — AGENT OFFLINE"
  echo "================================================================"
  echo "Wraps fingerprint offline + bag status. invent_pass=0 Pass=No"
  echo ""

  FP_RC=0
  if [[ -x "$FINGERPRINT_SH" || -f "$FINGERPRINT_SH" ]]; then
    set +e
    # S4/S5 live in fingerprint_once: spent ledger footer, skip spent re-dump
    # unless --force / XDNA2_FP_FORCE_REDUMP=1, and read-only *mladf*.xclbin /
    # *gemm*.xclbin inventory (xclbin-only).
    bash "$FINGERPRINT_SH" --offline
    FP_RC=$?
    set -e
  else
    exit_refuse 2 "ERROR: missing $FINGERPRINT_SH"
  fi

  echo ""
  echo "=== Bag status (after fingerprint) ==="
  BAG_N="$(count_bag_bins)"
  if [[ -f "$CAPTURE_SH" ]]; then
    set +e
    bash "$CAPTURE_SH" --status || true
    set -e
  else
    echo "bins=$BAG_N dir=$BAG_DIR"
  fi

  # Soft journal scrape (read-only; invent_pass=0). Never hard-fail agent-offline.
  echo ""
  echo "=== Journal scrape (soft; ≠ Pass) ==="
  if [[ -f "$TOOLS/journal_npu_scrape.sh" ]]; then
    set +e
    bash "$TOOLS/journal_npu_scrape.sh" --hours 24 --lines 30
    set -e
  else
    echo "JOURNAL_SCRAPE_SKIP invent_pass=0 reason=helper_missing"
  fi

  # W4a T1/T8: host NPU stack inventory + who-acts (read-only; soft).
  echo ""
  echo "=== Host NPU stack inventory + who-acts (W4a T1/T8 soft) ==="
  HOST_INV_RC=0
  if [[ -f "$HOST_INV_SH" ]]; then
    set +e
    bash "$HOST_INV_SH"
    HOST_INV_RC=$?
    set -e
  else
    echo "HOST_INV_SKIP invent_pass=0 reason=helper_missing path=$HOST_INV_SH"
    echo "who_acts=operator: drop load-accepted capture under testdata/xdna2_cmd/external_known_good/"
    HOST_INV_RC=2
  fi

  ACTIVE_UNLOCK="$(parse_active_unlock)"
  UNLOCK_STATE="none (first C_UNLOCK= is CONSUMED or absent)"
  if [[ -n "$ACTIVE_UNLOCK" ]]; then
    UNLOCK_STATE="$ACTIVE_UNLOCK"
  fi
  HUMAN=0
  if [[ "$BAG_N" -eq 0 ]]; then
    HUMAN=1
  fi
  PRIV="$(probe_privilege_needed)"

  {
    echo "# pass_unblock_once — agent-offline"
    echo
    echo "- Date: $(date -Iseconds 2>/dev/null || date)"
    echo "- Pass: **No** invent_pass=0 ran_on_npu=0"
    echo "- Unlock: \`$UNLOCK_STATE\`"
    echo "- Capture bins: $BAG_N under \`testdata/xdna2_cmd/external_known_good/\`"
    echo "- Fingerprint script rc: $FP_RC"
    echo "- Fingerprint report: \`.agents/reports/pass-unblock-fingerprint-once.md\`"
    echo "- PDI inventory (S5): \`.agents/reports/pdi-xclbin-inventory.md\`"
    echo "- Host NPU inventory (W4a): \`.agents/reports/wave4-host-inventory.md\` (rc=${HOST_INV_RC:-n/a})"
    echo "- Residual: \`RESIDUAL.md\` top (no active unlock; flexml unified-2x4x4 spent)"
    echo "- Spent default re-dump: skipped unless fingerprint \`--force\` or \`XDNA2_FP_FORCE_REDUMP=1\` (S4)"
    echo "- PRIVILEGE_NEEDED=$PRIV"
    echo "- HUMAN_NEEDED=$HUMAN (1 when bag empty — human holds working-stack bytes; agent offline still runs)"
    echo
    echo "Next: ingest when path known; all three flexml STX PDIs are Spent (RESIDUAL SSoT) — next live needs NEW unlock + named reason (not re-live spent); no --live without active unlock; no invent Pass."
  } >"$REPORT_MD"
  # Optional /tmp mirror: exclusive mktemp only (never fixed name; never follow symlink).
  TMPDIR_CLAMP="${TMPDIR:-/tmp}"
  if [[ "$TMPDIR_CLAMP" == "/" ]]; then
    TMPDIR_CLAMP="/tmp"
  fi
  REPORT_TMP="$(mktemp "${TMPDIR_CLAMP}/dragon_npu_pass_unblock_once.XXXXXX")"
  cp -- "$REPORT_MD" "$REPORT_TMP"
  echo ""
  echo "report_md=$REPORT_MD"
  echo "report_tmp=$REPORT_TMP"
  print_footer "$HUMAN" "$PRIV" "$BAG_N" "$UNLOCK_STATE"
  # Soft exit 0 so agent-offline always finishes the wrap; fingerprint_rc= in footer.
  exit 0
fi

# ---------------------------------------------------------------------------
# --ingest
# ---------------------------------------------------------------------------
if [[ "$MODE" == "ingest" ]]; then
  if [[ ! -f "$CAPTURE_SH" ]]; then
    exit_refuse 2 "ERROR: missing $CAPTURE_SH"
  fi
  if [[ -z "$INGEST_PATH" ]]; then
    exit_refuse 2 "ERROR: --ingest needs SOURCE path"
  fi
  if [[ ! -e "$INGEST_PATH" ]]; then
    exit_refuse 2 "ERROR: path missing: $INGEST_PATH"
  fi
  if refuse_basename "$INGEST_PATH"; then
    exit_refuse 2 \
      "ERROR: refused basename (local oracle / wrong platform): $(basename "$INGEST_PATH")" \
      "  pick a name like working_stack_ctrltext.bin (not validate.xclbin / ve2 / post_patch)"
  fi
  if [[ -n "$INGEST_NAME" ]]; then
    if ! safe_install_name "$INGEST_NAME"; then
      exit_refuse 2 \
        "ERROR: install NAME must be a single safe path component (no / \\ ..): $INGEST_NAME"
    fi
    if refuse_basename "$INGEST_NAME"; then
      exit_refuse 2 "ERROR: install basename refused: $INGEST_NAME"
    fi
  fi
  echo "=== pass_unblock_once — ingest (confined to capture dir) ==="
  set +e
  if [[ -n "$INGEST_NAME" ]]; then
    bash "$CAPTURE_SH" --install "$INGEST_PATH" "$INGEST_NAME"
    IRC=$?
  else
    bash "$CAPTURE_SH" --install "$INGEST_PATH"
    IRC=$?
  fi
  set -e
  BAG_N="$(count_bag_bins)"
  HUMAN=0
  [[ "$BAG_N" -eq 0 ]] && HUMAN=1
  # C4 pure classify lives in silicon_contract (iron ingest); soft honesty only.
  echo "c4_iron_artifact_ingest note: pure classify_iron_artifact_ingest / format_c4_iron_ingest_note (offline)"
  echo "  invent_pass=0 not_c_ready=1 not_ran_on_npu=1 product_known_good=0 iron_ingest_alone_not_pass=1"
  if [[ "$BAG_N" -eq 0 ]]; then
    echo "  blocked_on_empty=1 (capture dir still has no allowed bins)"
  else
    echo "  blocked_on_empty=0 bins=$BAG_N (research present ≠ product known-good / Pass)"
  fi
  echo "c5_pdi_geometry_vs_spent_flexml: compare_pdi_geometry_to_spent_flexml (spent rematch ≠ unlock alone)"
  echo "  spent_rematch_not_unlock_alone=1 invent_pass=0 geometry_alone_not_pass=1"
  print_footer "$HUMAN" "$PRIV" "$BAG_N" "$UNLOCK_STATE"
  exit "$IRC"
fi

# ---------------------------------------------------------------------------
# --ingest-pdi
# ---------------------------------------------------------------------------
if [[ "$MODE" == "ingest-pdi" ]]; then
  src="$INGEST_PDI_PATH"
  if [[ -z "$src" || ! -f "$src" ]]; then
    exit_refuse 2 "ERROR: --ingest-pdi needs an existing file: ${src:-"(empty)"}"
  fi
  if [[ ! -s "$src" ]]; then
    exit_refuse 2 "ERROR: empty PDI file: $src"
  fi
  if refuse_basename "$src"; then
    exit_refuse 2 \
      "ERROR: refused PDI source basename (local oracle / wrong platform): $(basename "$src")"
  fi
  TMPDIR_CLAMP="${TMPDIR:-/tmp}"
  if [[ "$TMPDIR_CLAMP" == "/" ]]; then
    TMPDIR_CLAMP="/tmp"
  fi
  # Exclusive unique path — never predictable basename-derived names (symlink plant).
  out="$(mktemp "${TMPDIR_CLAMP}/dragon_npu_pdi_ingest.XXXXXX")"
  echo "=== pass_unblock_once — ingest-pdi (research dump; not product known-good) ==="
  echo "source=$src"
  echo "out=$out"
  # Copy into the exclusive mktemp file (overwrite that inode only; no -f on foreign path).
  cp -- "$src" "$out"
  bytes="$(wc -c <"$out" | tr -d ' ')"
  sha256="$(sha256sum "$out" | awk '{print $1}')"
  sha16="${sha256:0:16}"
  echo "pdi_bytes=$bytes sha16=$sha16"
  if [[ "${XDNA2_LAB_PRINT_FULL_SHA:-0}" == "1" ]]; then
    echo "sha256=$sha256"
  else
    echo "sha256=omitted (set XDNA2_LAB_PRINT_FULL_SHA=1 for full hash)"
  fi
  echo "research_pdi_not_product_known_good=1 invent_pass=0 not_ran_on_npu=1"
  {
    echo "# pass_unblock_once — ingest-pdi"
    echo
    echo "- source: \`$src\`"
    echo "- out: \`$out\` (mktemp exclusive)"
    echo "- pdi_bytes: $bytes"
    echo "- sha16: \`$sha16\`"
    if [[ "${XDNA2_LAB_PRINT_FULL_SHA:-0}" == "1" ]]; then
      echo "- sha256: \`$sha256\`"
    else
      echo "- sha256: omitted (XDNA2_LAB_PRINT_FULL_SHA!=1)"
    fi
    echo "- research_pdi_not_product_known_good=1 invent_pass=0 Pass=No"
    echo "- Not product known-good. Dump ≠ Pass ≠ unlock."
  } >"$REPORT_MD"
  echo "report_md=$REPORT_MD"
  print_footer "$HUMAN" "$PRIV" "$BAG_N" "$UNLOCK_STATE"
  exit 0
fi

# ---------------------------------------------------------------------------
# --live
# ---------------------------------------------------------------------------
if [[ "$MODE" == "live" ]]; then
  echo "=== pass_unblock_once — live ==="
  ACTIVE_UNLOCK="$(parse_active_unlock)"
  if [[ -z "$ACTIVE_UNLOCK" ]]; then
    echo "ERROR: no active (non-CONSUMED) C_UNLOCK in RESIDUAL.md."
    echo "Do not reopen CONSUMED unlocks: 2ae69036 97f38307 11bc9016 3a70fabf b17b0119."
    echo "Flexml live under 2ae69036 already spent (wait_cmd_einval; not Pass)."
    echo "Next: --agent-offline for fingerprint; new unlock only after named offline reason."
    echo "Stock host still needs no sudo for config-cu-run once unlock is active."
    print_footer "$HUMAN" "$PRIV" "$BAG_N" "none"
    exit 3
  fi
  echo "Active unlock: $ACTIVE_UNLOCK"
  echo "Stock path: kit config-cu-run (no sudo L3; never ALLOW)."
  if [[ ! -f "$KIT_SH" ]]; then
    exit_refuse 2 "ERROR: missing $KIT_SH"
  fi
  set +e
  bash "$KIT_SH" config-cu-run
  LRC=$?
  set -e
  BAG_N="$(count_bag_bins)"
  print_footer "$HUMAN" "$PRIV" "$BAG_N" "$ACTIVE_UNLOCK"
  exit "$LRC"
fi

echo "ERROR: internal mode fallthrough: $MODE" >&2
exit 2
