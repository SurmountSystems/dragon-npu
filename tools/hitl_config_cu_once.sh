#!/usr/bin/env bash
# CONFIG_CU one-shot helper — prep offline; stock live EXEC agent-runnable.
#
# Never invents Pass. Never leaves sticky XDNA2_ALLOW_EXEC_CMD.
# Unlock SSoT: first C_UNLOCK=… line in RESIDUAL.md (active one-shot when that
# line has no CONSUMED). Soft NOTE only — never invent Pass. Recommend
# --timeout-s=120 for CONFIG_CU silicon (default lab outer 30s killed prior shot).
#
# Agent owns offline prep (a-prep, PDI dump, preflight). On **stock** amdxdna
# (no exec_cmd_allow sysfs) agent may also run live EXEC via run-stock (no sudo).
# Hardened L3 open/restore still needs sudo — refuse run-stock and print sandwich.
#
# What this solves:
#   - a-prep + env print in one place
#   - pins LAST_APPLY away from locked baseline (C3 can see new evidence)
#   - prints ready-to-paste fish/bash live shot with **real paths filled in**
#   - stock vs hardened sandwich (no fake sudo on stock)
#   - one-command stock live: prep → apply → collect (run-stock / run)
#   - preflight assets offline so human time is not wasted on missing ELF/PDI
#   - post-shot evidence collect after live shot
#
# Usage:
#   bash tools/hitl_config_cu_once.sh              # prep + sandwich print (default)
#   bash tools/hitl_config_cu_once.sh prep          # same
#   bash tools/hitl_config_cu_once.sh prep-skip-pdi # A/B: no CONFIG_CU (unset PDI blob)
#   bash tools/hitl_config_cu_once.sh print-only    # sandwich only (reuse env if set)
#   bash tools/hitl_config_cu_once.sh preflight     # offline checks only
#   bash tools/hitl_config_cu_once.sh run-stock     # prep + stock live apply + collect
#   bash tools/hitl_config_cu_once.sh run-stock-skip-pdi  # A/B live without CONFIG_CU
#   bash tools/hitl_config_cu_once.sh run           # alias of run-stock when stock
#   bash tools/hitl_config_cu_once.sh post [JSON]   # journal + p5 + p6 (no L3)
#
# Skip CONFIG_CU (A/B load-path isolation):
#   unset XDNA2_LAB_PDI_BLOB   # or empty path — lab never issues CONFIG_CU
#   export XDNA2_LAB_SKIP_CONFIG_CU=1   # loud note for logs/sandwich/last_apply name
#   kit aliases: config-cu-skip / config-cu-run-skip (operator_app_load_kit.sh)
#   Research CVML/AIE2P dump is NOT product matmul known-good PDI.
#   research_pdi_not_product_known_good=1 invent_pass=0 not_ran_on_npu=1
#   skip-CU prep ≠ Pass. Live skip still needs active C_UNLOCK (not CONSUMED).
#   Spent-sha process refuse (do not re-live alone as “new” progress):
#     flexml 0eeeb70c / ba0cd738 / 7d172bf2; CVML a1e65d04 (see CONSTRUCTION_MAP).
#   Expected residual classes on skip arm: config_cu absent; wait_cmd_einval /
#     wait_timeout / APP_LOAD / journal timeout or IO_PAGE_FAULT possible.
#   B1 hypothesis (not proven): lab classic pack key under flexml overlay may
#     mismatch topology — see docs/CONSTRUCTION_MAP_APP_LOAD.md § B1.
#   Debug BO: XDNA2_LAB_RESEARCH_DEBUG_BO=1 → lab timed CREATE+Assign 4096 Shmem
#   (research; invent_pass=0; size/type uncertain live; not Pass).
#   Same-run address patch (product Arm A / live-shot-effective):
#     export XDNA2_LAB_SAME_SHOT_RELA=1  # defaulted by prep / sandwich / run-stock
#     lab_exec_once forwards into cargo child; gate alone ≠ Pass ≠ unlock.
#     Stale disk UVAs forbidden for product live patch.
#
# Fish: run with bash. Export printed env into the same shell that runs apply.
set -euo pipefail
export PATH="/usr/bin${HOME:+:${HOME}/.local/bin}:${PATH:-}"
unset XDNA2_ALLOW_EXEC_CMD || true

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
TOOLS="$ROOT/tools"

cmd="${1:-prep}"
shift || true

TMP_ROOT="${TMPDIR:-/tmp}"
TMP_ROOT="${TMP_ROOT%/}"
if [[ -z "$TMP_ROOT" || "$TMP_ROOT" == "/" ]]; then
  TMP_ROOT="/tmp"
fi

read_allow() {
  local p="/sys/module/amdxdna/parameters/exec_cmd_allow"
  if [[ -r "$p" ]]; then
    tr -d '[:space:]' <"$p" || true
  else
    echo "missing"
  fi
}

first_unlock_line() {
  if [[ -f "$ROOT/RESIDUAL.md" ]]; then
    { grep -m1 -E 'C_UNLOCK=[A-Za-z0-9_]+' "$ROOT/RESIDUAL.md" || true; } | head -n1
  fi
}

unlock_note() {
  local line tok
  line="$(first_unlock_line)"
  tok="$( { grep -oE 'C_UNLOCK=[A-Za-z0-9_]+' <<<"${line}" || true; } | head -n1 | tr -d '`"*' )"
  if [[ -n "$tok" ]]; then
    if grep -qiE 'CONSUMED' <<<"$line"; then
      echo "NOTE: ${tok} appears CONSUMED on first line — check RESIDUAL Unlock row."
    else
      echo "NOTE: ${tok} one-shot (active per RESIDUAL first C_UNLOCK=); unlock ≠ Pass ≠ ran_on_npu"
    fi
  fi
}

# Exit 0 if first C_UNLOCK= line is present and not CONSUMED; else exit 2.
require_active_unlock() {
  local line tok
  line="$(first_unlock_line)"
  tok="$( { grep -oE 'C_UNLOCK=[A-Za-z0-9_]+' <<<"${line}" || true; } | head -n1 | tr -d '`"*' )"
  if [[ -z "$tok" ]]; then
    echo "ERROR: no C_UNLOCK= in RESIDUAL.md — refuse live run" >&2
    exit 2
  fi
  if grep -qiE 'CONSUMED' <<<"$line"; then
    echo "ERROR: ${tok} is CONSUMED on first Unlock line — refuse live run (mint new unlock first)" >&2
    exit 2
  fi
  echo "UNLOCK_ACTIVE=${tok} invent_pass=0 not_ran_on_npu=1 (unlock ≠ Pass)"
}

skip_config_cu_requested() {
  # Loud skip flag or empty/unset PDI path both mean no CONFIG_CU.
  if [[ "${XDNA2_LAB_SKIP_CONFIG_CU:-}" == "1" || "${XDNA2_LAB_SKIP_CONFIG_CU:-}" == "true" ]]; then
    return 0
  fi
  return 1
}

clear_pdi_for_skip() {
  unset XDNA2_LAB_PDI_BLOB || true
  export XDNA2_LAB_SKIP_CONFIG_CU=1
  echo "CONFIG_CU_SKIP=1 (XDNA2_LAB_PDI_BLOB unset; lab will not CONFIG_CU)"
  echo "NOTE: research_pdi_not_product_known_good=1 invent_pass=0 not_ran_on_npu=1"
  echo "NOTE: A/B vs research blob — skip path measures baseline without overlay PDI"
}

ensure_pdi() {
  if skip_config_cu_requested; then
    clear_pdi_for_skip
    return 0
  fi
  if [[ -n "${XDNA2_LAB_PDI_BLOB:-}" && -f "${XDNA2_LAB_PDI_BLOB}" ]]; then
    local sz
    sz="$(wc -c <"${XDNA2_LAB_PDI_BLOB}" | tr -d ' ')"
    if [[ "$sz" -gt 0 ]]; then
      echo "PDI_REUSE path=${XDNA2_LAB_PDI_BLOB} bytes=${sz}"
      echo "NOTE: research PDI only — research_pdi_not_product_known_good=1 invent_pass=0 not_ran_on_npu=1"
      echo "NOTE: attach measured ≠ Pass; wrong overlay may still IO_PAGE_FAULT / APP_LOAD"
      return 0
    fi
  fi
  echo "=== a-prep (research PDI; not known-good matmul) ==="
  echo "NOTE: default dump is CVML/AIE2P research overlay — research_pdi_not_product_known_good=1"
  # Capture dump_xclbin_pdi --print-env and parse export line.
  local out
  out="$(bash "$TOOLS/dump_xclbin_pdi.sh" --print-env 2>&1)" || {
    echo "$out" >&2
    echo "ERROR: a-prep / dump_xclbin_pdi failed" >&2
    exit 2
  }
  echo "$out"
  local pdi
  pdi="$( { grep -oE 'XDNA2_LAB_PDI_BLOB=[^[:space:]]+' <<<"$out" || true; } | head -n1 | cut -d= -f2- )"
  if [[ -z "$pdi" || ! -f "$pdi" ]]; then
    echo "ERROR: could not parse PDI path from a-prep output" >&2
    exit 2
  fi
  export XDNA2_LAB_PDI_BLOB="$pdi"
  export XDNA2_LAB_CU_FUNC="${XDNA2_LAB_CU_FUNC:-0}"
  echo "NOTE: attach measured ≠ Pass; wrong overlay may still IO_PAGE_FAULT / APP_LOAD"
}

pin_last_apply() {
  if [[ -n "${XDNA2_LAB_LAST_APPLY_JSON:-}" ]]; then
    echo "LAST_APPLY already set: $XDNA2_LAB_LAST_APPLY_JSON"
    return 0
  fi
  export XDNA2_LAB_LAST_APPLY_JSON="${TMP_ROOT}/dragon_npu_last_apply_config_cu_$$.json"
  echo "LAST_APPLY pin (new shot; baseline preserved): $XDNA2_LAB_LAST_APPLY_JSON"
}

# Product Arm A / live-shot-effective: default same-run address gate on so the
# sandwich is labeled and fail-closed for invalid in-process paddrs. Operator may
# force off with XDNA2_LAB_SAME_SHOT_RELA=0. Env alone ≠ Pass ≠ unlock.
ensure_same_shot_rela() {
  if [[ -z "${XDNA2_LAB_SAME_SHOT_RELA+x}" ]]; then
    export XDNA2_LAB_SAME_SHOT_RELA=1
  elif [[ -z "${XDNA2_LAB_SAME_SHOT_RELA}" ]]; then
    export XDNA2_LAB_SAME_SHOT_RELA=1
  fi
  case "${XDNA2_LAB_SAME_SHOT_RELA}" in
    1|true|TRUE|yes|YES|on|ON)
      echo "SAME_SHOT_RELA=1 (in-process BO paddrs only; refuse stale disk UVAs)"
      echo "NOTE: same_shot_gate alone ≠ Pass ≠ unlock ≠ ran_on_npu invent_pass=0"
      ;;
    0|false|FALSE|no|NO|off|OFF)
      echo "SAME_SHOT_RELA=${XDNA2_LAB_SAME_SHOT_RELA} (explicitly off; product Arm A prefers 1)"
      ;;
    *)
      echo "SAME_SHOT_RELA=${XDNA2_LAB_SAME_SHOT_RELA} (token table wants 1/true/yes/on)"
      ;;
  esac
}

preflight() {
  echo "=== config_cu preflight (offline; no L3) ==="
  unlock_note
  ensure_same_shot_rela
  echo "ROOT=$ROOT"
  local _allow_now
  _allow_now="$(read_allow)"
  if [[ "$_allow_now" == "missing" ]]; then
    echo "driver: exec_cmd_allow=missing (stock in-tree amdxdna — no L3; live shot needs no sudo)"
  else
    echo "driver: exec_cmd_allow=${_allow_now} (expect N/0 before hardened L3 sandwich)"
  fi
  local elf="$ROOT/testdata/xdna2_cmd/silicon_lab/lab_instr.elf"
  if [[ ! -f "$elf" ]]; then
    echo "ERROR: missing $elf — stage first: bash tools/silicon_lab_operator.sh stage" >&2
    exit 2
  fi
  local magic
  magic="$(od -An -tx1 -N4 "$elf" | tr -d ' \n')"
  if [[ "$magic" != "7f454c46" ]]; then
    echo "ERROR: lab_instr.elf not ELF magic ($magic)" >&2
    exit 2
  fi
  echo "ELF_OK $elf ($(wc -c <"$elf" | tr -d ' ') B)"
  if skip_config_cu_requested || [[ -z "${XDNA2_LAB_PDI_BLOB:-}" ]]; then
    if skip_config_cu_requested || [[ "${XDNA2_LAB_SKIP_CONFIG_CU:-}" == "1" ]]; then
      echo "PDI_SKIP config_cu_off=1 (XDNA2_LAB_PDI_BLOB unset/empty)"
      echo "NOTE: research_pdi_not_product_known_good=1 invent_pass=0 not_ran_on_npu=1"
      echo "NOTE: A/B baseline without research overlay PDI attach"
    else
      echo "ERROR: PDI not ready — run: bash tools/hitl_config_cu_once.sh prep" >&2
      echo "ERROR: or skip CONFIG_CU: bash tools/hitl_config_cu_once.sh prep-skip-pdi" >&2
      exit 2
    fi
  else
    if [[ ! -f "${XDNA2_LAB_PDI_BLOB}" ]]; then
      echo "ERROR: XDNA2_LAB_PDI_BLOB not a file: ${XDNA2_LAB_PDI_BLOB}" >&2
      exit 2
    fi
    local psz
    psz="$(wc -c <"${XDNA2_LAB_PDI_BLOB}" | tr -d ' ')"
    echo "PDI_OK path=${XDNA2_LAB_PDI_BLOB} bytes=${psz} cu_func=${XDNA2_LAB_CU_FUNC:-0}"
    echo "NOTE: research_pdi_not_product_known_good=1 invent_pass=0 not_ran_on_npu=1"
    echo "NOTE: attach measured ≠ Pass; wrong overlay may still IO_PAGE_FAULT / APP_LOAD"
  fi
  echo "LAST_APPLY=${XDNA2_LAB_LAST_APPLY_JSON:-(hitl_c will pin if unset)}"
  echo "SAME_SHOT_RELA=${XDNA2_LAB_SAME_SHOT_RELA:-(unset)} (lab_exec forwards; ≠ Pass)"
  echo "PREFLIGHT_OK invent_pass=0 not_ran_on_npu=1 (assets only; ≠ Pass)"
}

print_sandwich() {
  ensure_same_shot_rela
  local pdi="${XDNA2_LAB_PDI_BLOB:-}"
  local cu="${XDNA2_LAB_CU_FUNC:-0}"
  local la="${XDNA2_LAB_LAST_APPLY_JSON:-}"
  local ss="${XDNA2_LAB_SAME_SHOT_RELA:-1}"
  local allow
  allow="$(read_allow)"
  local skip=0
  if skip_config_cu_requested || [[ -z "$pdi" ]]; then
    if skip_config_cu_requested || [[ "${XDNA2_LAB_SKIP_CONFIG_CU:-}" == "1" ]]; then
      skip=1
    else
      echo "ERROR: XDNA2_LAB_PDI_BLOB unset — run prep first, or prep-skip-pdi for A/B" >&2
      exit 2
    fi
  fi
  if [[ -z "$la" ]]; then
    if [[ "$skip" -eq 1 ]]; then
      la="${TMP_ROOT}/dragon_npu_last_apply_skip_pdi_SHOT.json"
    else
      la="${TMP_ROOT}/dragon_npu_last_apply_config_cu_SHOT.json"
    fi
  fi

  if [[ "$allow" == "missing" ]]; then
    if [[ "$skip" -eq 1 ]]; then
      cat <<EOF

======== SKIP CONFIG_CU LIVE SHOT — STOCK (A/B baseline; no sudo / no L3) ========
Host: exec_cmd_allow sysfs **missing** (in-tree amdxdna). Driver has no L3 gate.
Experiment: **no** XDNA2_LAB_PDI_BLOB → lab never issues CONFIG_CU (compare vs research blob).
Product Arm A: XDNA2_LAB_SAME_SHOT_RELA=$ss (in-process paddrs only; refuse stale disk UVAs).
Unlock: first C_UNLOCK= in RESIDUAL.md (active one-shot when that line has no CONSUMED).
Unlock ≠ Pass ≠ ran_on_npu. Active unlock = first C_UNLOCK= line in RESIDUAL.md only when that line has no CONSUMED (read RESIDUAL top; do not reopen CONSUMED).
research_pdi_not_product_known_good=1 invent_pass=0 not_ran_on_npu=1
Outer budget: --timeout-s=120. WAIT stays finite. Wedge risk on stock is real.
same_shot_gate alone ≠ Pass ≠ unlock.

--- bash (one paste; no sudo) — under active RESIDUAL unlock only ---
cd $ROOT
export PATH="/usr/bin:\${HOME}/.local/bin:\${PATH}"
unset XDNA2_ALLOW_EXEC_CMD
unset XDNA2_LAB_PDI_BLOB
export XDNA2_LAB_SKIP_CONFIG_CU=1
export XDNA2_LAB_LAST_APPLY_JSON=$la
export XDNA2_LAB_BO_DIAG=\${XDNA2_LAB_BO_DIAG:-1}
export XDNA2_LAB_SAME_SHOT_RELA=$ss
bash tools/hitl_c_pass_apply.sh --assume-l3-already-open --timeout-s=120
bash tools/collect_post_c2_evidence.sh "\$XDNA2_LAB_LAST_APPLY_JSON"

--- fish (one paste; no sudo) — under active RESIDUAL unlock only ---
cd $ROOT
set -gx PATH /usr/bin \$HOME/.local/bin \$PATH
set -e XDNA2_ALLOW_EXEC_CMD
set -e XDNA2_LAB_PDI_BLOB
set -gx XDNA2_LAB_SKIP_CONFIG_CU 1
set -gx XDNA2_LAB_LAST_APPLY_JSON $la
set -gx XDNA2_LAB_BO_DIAG 1
set -gx XDNA2_LAB_SAME_SHOT_RELA $ss
bash tools/hitl_c_pass_apply.sh --assume-l3-already-open --timeout-s=120
bash tools/collect_post_c2_evidence.sh \$XDNA2_LAB_LAST_APPLY_JSON

Paste for agent: collect_post_c2_evidence output + hitl_c rc + last_apply path above.
Do not reopen CONSUMED unlocks (see first C_UNLOCK= in RESIDUAL.md).
======================================================================
EOF
      return 0
    fi
    cat <<EOF

======== CONFIG_CU LIVE SHOT — STOCK MODULE (no sudo / no L3) ========
Host: exec_cmd_allow sysfs **missing** (in-tree amdxdna). Driver has no L3 gate.
Prep (a-prep / PDI / preflight) is **agent-offline** — do not re-run for the human.
This paste is the **live EXEC** only (userspace ALLOW set by lab_exec for one process).
Unlock: first C_UNLOCK= in RESIDUAL.md (active one-shot when that line has no CONSUMED).
Unlock ≠ Pass ≠ ran_on_npu. Active unlock = first C_UNLOCK= line in RESIDUAL.md only when that line has no CONSUMED (read RESIDUAL top; do not reopen CONSUMED).
PDI research bytes only (CVML/AIE2P dump ≠ product matmul known-good).
research_pdi_not_product_known_good=1 invent_pass=0 not_ran_on_npu=1
Wrong overlay may still IO_PAGE_FAULT / APP_LOAD / 0x3000003; still measure config_cu=.
XDNA2_LAB_SAME_SHOT_RELA=$ss (in-process paddrs; gate alone ≠ Pass).
Outer budget: use --timeout-s=120 (or ≥120). Default 30s outer killed a prior CONFIG_CU shot.
WAIT stays finite (XDNA2_WAIT_TIMEOUT_MS ~5–15s). Progressive last_apply stamps mid-path.
Wedge risk on stock is real (no L2 null-guards).

--- bash (one paste; no sudo) — under active RESIDUAL unlock only ---
cd $ROOT
export PATH="/usr/bin:\${HOME}/.local/bin:\${PATH}"
unset XDNA2_ALLOW_EXEC_CMD
export XDNA2_LAB_PDI_BLOB=$pdi
export XDNA2_LAB_CU_FUNC=$cu
export XDNA2_LAB_LAST_APPLY_JSON=$la
export XDNA2_LAB_BO_DIAG=\${XDNA2_LAB_BO_DIAG:-1}
export XDNA2_LAB_SAME_SHOT_RELA=$ss
bash tools/hitl_c_pass_apply.sh --assume-l3-already-open --timeout-s=120
bash tools/collect_post_c2_evidence.sh "\$XDNA2_LAB_LAST_APPLY_JSON"

--- fish (one paste; no sudo) — under active RESIDUAL unlock only ---
cd $ROOT
set -gx PATH /usr/bin \$HOME/.local/bin \$PATH
set -e XDNA2_ALLOW_EXEC_CMD
set -gx XDNA2_LAB_PDI_BLOB $pdi
set -gx XDNA2_LAB_CU_FUNC $cu
set -gx XDNA2_LAB_LAST_APPLY_JSON $la
set -gx XDNA2_LAB_BO_DIAG 1
set -gx XDNA2_LAB_SAME_SHOT_RELA $ss
bash tools/hitl_c_pass_apply.sh --assume-l3-already-open --timeout-s=120
bash tools/collect_post_c2_evidence.sh \$XDNA2_LAB_LAST_APPLY_JSON

Paste for agent: collect_post_c2_evidence output + hitl_c rc + last_apply path above.
Do not reopen CONSUMED unlocks (see first C_UNLOCK= in RESIDUAL.md).
======================================================================
EOF
  else
    if [[ "$skip" -eq 1 ]]; then
      cat <<EOF

======== SKIP CONFIG_CU SANDWICH — HARDENED (A/B baseline) ========
Unlock: first C_UNLOCK= in RESIDUAL.md (active one-shot when that line has no CONSUMED).
Unlock ≠ Pass ≠ ran_on_npu. Active unlock = first C_UNLOCK= line in RESIDUAL.md only when that line has no CONSUMED (read RESIDUAL top; do not reopen CONSUMED).
Experiment: **no** XDNA2_LAB_PDI_BLOB → lab never issues CONFIG_CU.
Product Arm A: XDNA2_LAB_SAME_SHOT_RELA=$ss (in-process paddrs; gate alone ≠ Pass).
research_pdi_not_product_known_good=1 invent_pass=0 not_ran_on_npu=1
Outer budget: --timeout-s=120. WAIT stays finite. driver exec_cmd_allow=$allow.

--- bash (one paste) — under active RESIDUAL unlock only ---
cd $ROOT
export PATH="/usr/bin:\${HOME}/.local/bin:\${PATH}"
unset XDNA2_ALLOW_EXEC_CMD
unset XDNA2_LAB_PDI_BLOB
export XDNA2_LAB_SKIP_CONFIG_CU=1
export XDNA2_LAB_LAST_APPLY_JSON=$la
export XDNA2_LAB_BO_DIAG=\${XDNA2_LAB_BO_DIAG:-1}
export XDNA2_LAB_SAME_SHOT_RELA=$ss
sudo sh -c 'echo 1 > /sys/module/amdxdna/parameters/exec_cmd_allow'
bash tools/hitl_c_pass_apply.sh --assume-l3-already-open --timeout-s=120
sudo sh -c 'echo 0 > /sys/module/amdxdna/parameters/exec_cmd_allow'
cat /sys/module/amdxdna/parameters/exec_cmd_allow   # must be 0 or N
bash tools/collect_post_c2_evidence.sh "\$XDNA2_LAB_LAST_APPLY_JSON"

--- fish (one paste) — under active RESIDUAL unlock only ---
cd $ROOT
set -gx PATH /usr/bin \$HOME/.local/bin \$PATH
set -e XDNA2_ALLOW_EXEC_CMD
set -e XDNA2_LAB_PDI_BLOB
set -gx XDNA2_LAB_SKIP_CONFIG_CU 1
set -gx XDNA2_LAB_LAST_APPLY_JSON $la
set -gx XDNA2_LAB_BO_DIAG 1
set -gx XDNA2_LAB_SAME_SHOT_RELA $ss
sudo sh -c 'echo 1 > /sys/module/amdxdna/parameters/exec_cmd_allow'
bash tools/hitl_c_pass_apply.sh --assume-l3-already-open --timeout-s=120
sudo sh -c 'echo 0 > /sys/module/amdxdna/parameters/exec_cmd_allow'
cat /sys/module/amdxdna/parameters/exec_cmd_allow
bash tools/collect_post_c2_evidence.sh \$XDNA2_LAB_LAST_APPLY_JSON

If L3 left open after any failure:
  sudo sh -c 'echo 0 > /sys/module/amdxdna/parameters/exec_cmd_allow'

Paste for agent: collect_post_c2_evidence + hitl_c rc + last_apply.
Do not reopen CONSUMED unlocks (see first C_UNLOCK= in RESIDUAL.md).
======================================================================
EOF
      return 0
    fi
    cat <<EOF

======== CONFIG_CU SANDWICH — HARDENED MODULE (L3 open/restore) ========
Unlock: first C_UNLOCK= in RESIDUAL.md (active one-shot when that line has no CONSUMED).
Unlock ≠ Pass ≠ ran_on_npu. Active unlock = first C_UNLOCK= line in RESIDUAL.md only when that line has no CONSUMED (read RESIDUAL top; do not reopen CONSUMED).
PDI research bytes only (CVML/AIE2P ≠ product matmul known-good).
research_pdi_not_product_known_good=1 invent_pass=0 not_ran_on_npu=1
Wrong overlay may still IO_PAGE_FAULT / APP_LOAD / 0x3000003; still measure config_cu=.
XDNA2_LAB_SAME_SHOT_RELA=$ss (in-process paddrs; gate alone ≠ Pass).
Outer budget: use --timeout-s=120 (or ≥120). WAIT stays finite (~5–15s).
driver exec_cmd_allow=$allow (expect 0/N before open).

--- bash (one paste) — under active RESIDUAL unlock only ---
cd $ROOT
export PATH="/usr/bin:\${HOME}/.local/bin:\${PATH}"
unset XDNA2_ALLOW_EXEC_CMD
export XDNA2_LAB_PDI_BLOB=$pdi
export XDNA2_LAB_CU_FUNC=$cu
export XDNA2_LAB_LAST_APPLY_JSON=$la
export XDNA2_LAB_BO_DIAG=\${XDNA2_LAB_BO_DIAG:-1}
export XDNA2_LAB_SAME_SHOT_RELA=$ss
sudo sh -c 'echo 1 > /sys/module/amdxdna/parameters/exec_cmd_allow'
bash tools/hitl_c_pass_apply.sh --assume-l3-already-open --timeout-s=120
sudo sh -c 'echo 0 > /sys/module/amdxdna/parameters/exec_cmd_allow'
cat /sys/module/amdxdna/parameters/exec_cmd_allow   # must be 0 or N
bash tools/collect_post_c2_evidence.sh "\$XDNA2_LAB_LAST_APPLY_JSON"

--- fish (one paste) — under active RESIDUAL unlock only ---
cd $ROOT
set -gx PATH /usr/bin \$HOME/.local/bin \$PATH
set -e XDNA2_ALLOW_EXEC_CMD
set -gx XDNA2_LAB_PDI_BLOB $pdi
set -gx XDNA2_LAB_CU_FUNC $cu
set -gx XDNA2_LAB_LAST_APPLY_JSON $la
set -gx XDNA2_LAB_BO_DIAG 1
set -gx XDNA2_LAB_SAME_SHOT_RELA $ss
sudo sh -c 'echo 1 > /sys/module/amdxdna/parameters/exec_cmd_allow'
bash tools/hitl_c_pass_apply.sh --assume-l3-already-open --timeout-s=120
sudo sh -c 'echo 0 > /sys/module/amdxdna/parameters/exec_cmd_allow'
cat /sys/module/amdxdna/parameters/exec_cmd_allow
bash tools/collect_post_c2_evidence.sh \$XDNA2_LAB_LAST_APPLY_JSON

If L3 left open after any failure:
  sudo sh -c 'echo 0 > /sys/module/amdxdna/parameters/exec_cmd_allow'

Paste for agent: collect_post_c2_evidence output + hitl_c rc + last_apply path above.
Do not reopen CONSUMED unlocks (see first C_UNLOCK= in RESIDUAL.md).
======================================================================
EOF
  fi
}

run_post() {
  local json="${1:-${XDNA2_LAB_LAST_APPLY_JSON:-}}"
  bash "$TOOLS/collect_post_c2_evidence.sh" ${json:+"$json"}
}

# Stock-only live path: prep → hitl_c_pass_apply → collect. No sudo L3.
# Hardened hosts (exec_cmd_allow present) refuse — needs human sudo for L3.
run_stock_live() {
  local timeout_s="${XDNA2_HITL_OUTER_TIMEOUT_S:-120}"
  if ! [[ "$timeout_s" =~ ^[0-9]+$ ]] || [[ "$timeout_s" -lt 120 ]]; then
    timeout_s=120
  fi
  echo "=== hitl_config_cu_once run-stock (live EXEC; invent_pass=0) ==="
  require_active_unlock
  local allow
  allow="$(read_allow)"
  if [[ "$allow" != "missing" ]]; then
    echo "ERROR: exec_cmd_allow=${allow} (hardened module present)." >&2
    echo "ERROR: run-stock is stock-only (no sudo L3). Open L3 with sudo yourself, then:" >&2
    echo "ERROR:   bash tools/hitl_c_pass_apply.sh --assume-l3-already-open --timeout-s=${timeout_s}" >&2
    ensure_pdi
    pin_last_apply
    preflight
    print_sandwich
    exit 2
  fi
  echo "NOTE: stock amdxdna — no sudo L3; wedge risk real (no L2 null-guards)"
  echo "NOTE: outer timeout-s=${timeout_s}; unlock ≠ Pass ≠ ran_on_npu"
  ensure_pdi
  pin_last_apply
  preflight
  export XDNA2_LAB_BO_DIAG="${XDNA2_LAB_BO_DIAG:-1}"
  export XDNA2_LAB_CU_FUNC="${XDNA2_LAB_CU_FUNC:-0}"
  ensure_same_shot_rela
  echo "--- live apply (stock) ---"
  if skip_config_cu_requested || [[ -z "${XDNA2_LAB_PDI_BLOB:-}" ]]; then
    echo "PDI=(skip CONFIG_CU) XDNA2_LAB_SKIP_CONFIG_CU=${XDNA2_LAB_SKIP_CONFIG_CU:-}"
  else
    echo "PDI=${XDNA2_LAB_PDI_BLOB}"
  fi
  echo "LAST_APPLY=${XDNA2_LAB_LAST_APPLY_JSON}"
  local rc=0
  set +e
  bash "$TOOLS/hitl_c_pass_apply.sh" --assume-l3-already-open --timeout-s="${timeout_s}"
  rc=$?
  set -e
  unset XDNA2_ALLOW_EXEC_CMD || true
  echo "--- post-evidence (offline) ---"
  set +e
  run_post "${XDNA2_LAB_LAST_APPLY_JSON:-}"
  local post_rc=$?
  set -e
  unset XDNA2_ALLOW_EXEC_CMD || true
  echo "RUN_STOCK_DONE hitl_c_rc=${rc} post_rc=${post_rc} last_apply=${XDNA2_LAB_LAST_APPLY_JSON:-} invent_pass=0"
  if [[ -n "${XDNA2_LAB_LAST_APPLY_JSON:-}" && -f "${XDNA2_LAB_LAST_APPLY_JSON}" ]]; then
    echo "LAST_APPLY_EXISTS bytes=$(wc -c <"${XDNA2_LAB_LAST_APPLY_JSON}" | tr -d ' ')"
  else
    echo "LAST_APPLY_MISSING (timeout mid-ioctl may leave only progressive pin or none)"
  fi
  # Prefer apply rc for exit (post soft); still 0 only if both green rare Pass path.
  exit "$rc"
}

case "$cmd" in
  -h|--help)
    grep -E '^#([^!]|$)' "$0" | sed 's/^# \{0,1\}//'
    exit 0
    ;;
  prep|default|"")
    echo "=== hitl_config_cu_once prep (no L3 / no ALLOW / invent_pass=0) ==="
    unlock_note
    ensure_pdi
    pin_last_apply
    preflight
    print_sandwich
    echo "PREP_OK: for stock live, agent may run: bash tools/hitl_config_cu_once.sh run-stock"
    echo "PREP_OK: A/B without CONFIG_CU: bash tools/hitl_config_cu_once.sh prep-skip-pdi"
    exit 0
    ;;
  prep-skip-pdi|prep_skip_pdi|skip-pdi|no-pdi)
    echo "=== hitl_config_cu_once prep-skip-pdi (A/B; no CONFIG_CU; invent_pass=0) ==="
    unlock_note
    clear_pdi_for_skip
    pin_last_apply
    preflight
    print_sandwich
    echo "PREP_SKIP_OK: stock live without CONFIG_CU: bash tools/hitl_config_cu_once.sh run-stock-skip-pdi"
    exit 0
    ;;
  print-only|print)
    if [[ -z "${XDNA2_LAB_PDI_BLOB:-}" || ! -f "${XDNA2_LAB_PDI_BLOB}" ]]; then
      if skip_config_cu_requested; then
        clear_pdi_for_skip
      else
        ensure_pdi
      fi
    fi
    pin_last_apply
    print_sandwich
    exit 0
    ;;
  preflight)
    preflight
    exit 0
    ;;
  run-stock|run_stock|run)
    run_stock_live
    ;;
  run-stock-skip-pdi|run_stock_skip_pdi|run-skip-pdi)
    clear_pdi_for_skip
    run_stock_live
    ;;
  post|evidence)
    run_post "${1:-}"
    exit $?
    ;;
  *)
    echo "ERROR: unknown command: $cmd" >&2
    echo "allowlist: prep prep-skip-pdi print-only preflight run-stock run-stock-skip-pdi run post" >&2
    exit 2
    ;;
esac
