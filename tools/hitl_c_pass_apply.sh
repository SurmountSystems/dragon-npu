#!/usr/bin/env bash
# Milestone C — Pure-direct Pass apply (YOU run this when agent says ready).
#
# Forces aiebu ELF (not raw 1688 B txn), silicon-mladf assets, L1+L3 window,
# single-shot npu_direct_bench, always restores exec_cmd_allow=0.
#
# Policy: run only when agent explicitly says ready. See RESIDUAL.md.
# SSoT unlock: first C_UNLOCK=… line in RESIDUAL.md (soft NOTE only — never
# hard-blocks apply, even when CONSUMED). Soft-parse matches hitl_c2_once.sh:
#   Active: first match without CONSUMED on that line → one-shot (CONFIG_CU +
#           XDNA2_LAB_PDI_BLOB sandwich; prefer --timeout-s=120).
#   Consumed: CONSUMED on first line / same stamp → historical; further C needs
#             a new agent unlock + offline delta after this one-shot is used.
# Historical: 3a70fabf / b17b0119 CONSUMED — do not reopen under those stamps.
# Standing policy: CONSUMED is messaging honesty only (soft NOTE); it does NOT
# refuse --assume-l3-already-open. Operator owns L3; hitl_c2_once prints
# executable sandwich when active, HISTORICAL record-only when consumed.
# Unlock ≠ Pass ≠ ran_on_npu ≠ permanent open season. CONFIG_CU attach alone ≠ Pass.
# L3 = amdxdna exec_cmd_allow (driver). Userspace ALLOW = XDNA2_ALLOW_EXEC_CMD
# (Cargo double-gate). Leave neither open; unlock is not permanent either.
# SSoT: vendor classic body often EXISTS (unpatched CLASS A offline).
# Never unlocks C alone: CONTRACT_OK, unpatched CLASS A / mere diff rc=0,
# registry_get_instr_bo (Transaction/cache, not live run_ BO), vendor miss
# soft-skip, residual_class / CLASSIFY_SOFT_OK / app_load_checklist,
# run_offline_gates / OFFLINE_GATES_OK. Soft-WARNs if vendor bin missing;
# does NOT hard-block. Missing C_UNLOCK stamp also does NOT hard-block.
# Missing silicon_contract_check.sh fails closed unless --force-apply.
#
# Fish-safe: run with bash.
#
# Usage (when RESIDUAL first C_UNLOCK= is active):
#   # CONFIG_CU pre-step (optional env; default off):
#   # bash tools/operator_app_load_kit.sh a-prep
#   # export XDNA2_LAB_PDI_BLOB=/path/from/a-prep
#   # export XDNA2_LAB_CU_FUNC=0
#   # Recommended: open L3 yourself, then:
#   sudo sh -c 'echo 1 > /sys/module/amdxdna/parameters/exec_cmd_allow'
#   bash tools/hitl_c_pass_apply.sh --assume-l3-already-open
#   sudo sh -c 'echo 0 > /sys/module/amdxdna/parameters/exec_cmd_allow'
#
#   # Or let lab_exec try to open L3 (active unlock only; may print human sudo steps):
#   bash tools/hitl_c_pass_apply.sh
#
#   # Research only — apply even if offline contract red / script missing:
#   bash tools/hitl_c_pass_apply.sh --force-apply-despite-offline-red --assume-l3-already-open
#
#   # Faster re-apply when operator already ran contract green this session:
#   XDNA2_HITL_SKIP_CONTRACT=1 bash tools/hitl_c_pass_apply.sh --assume-l3-already-open
#   # or: bash tools/hitl_c_pass_apply.sh --skip-offline-contract --assume-l3-already-open
#   # Still refuses non-ELF magic; does NOT skip ELF / preflight asset gates.
#
# Exit: same as lab_exec_once (0 rare Pass; 4 = !ran_on_npu common; 5 restore fail / L3 open).
#       2 = offline contract red/missing script (default) or missing ELF / unknown args.
set -euo pipefail
export PATH="/usr/bin:${HOME}/.local/bin:${PATH}"
# Never inherit sticky userspace EXEC gate into lab/cargo children.
# lab_exec_once is the sole intentional setter during the L3 apply window.
unset XDNA2_ALLOW_EXEC_CMD || true
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

ASSUME=()
BO_DIAG=1
FORCE_APPLY=0
NO_BUILD=0
SKIP_CONTRACT=0
TIMEOUT_S=""
# Env opt-in for skip (re-apply UX); flag also works. Truthy: 1/true/yes/on (case-insens).
_skip_env="${XDNA2_HITL_SKIP_CONTRACT:-}"
case "${_skip_env,,}" in
  1|true|yes|on) SKIP_CONTRACT=1 ;;
esac
SKIP_CONTRACT_FROM_ENV=0
[[ "$SKIP_CONTRACT" -eq 1 ]] && SKIP_CONTRACT_FROM_ENV=1
CONTRACT_STATUS="not_run"  # green|red_forced|skipped|not_run|missing_forced
CONTRACT_RC=""
for a in "$@"; do
  case "$a" in
    --assume-l3-already-open|--assume-l3-open)
      ASSUME+=(--assume-l3-already-open)
      ;;
    --no-bo-diag) BO_DIAG=0 ;;
    --no-build) NO_BUILD=1 ;;
    --force-apply-despite-offline-red|--force-apply) FORCE_APPLY=1 ;;
    --skip-offline-contract|--skip-contract) SKIP_CONTRACT=1 ;;
    --timeout-s=*)
      TIMEOUT_S="${a#--timeout-s=}"
      ;;
    --timeout-s)
      echo "ERROR: use --timeout-s=N (joined form)" >&2
      exit 2
      ;;
    -h|--help)
      # Comment-only help (never dump executable body past header).
      grep -E '^#([^!]|$)' "$0" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    --blob|--cargo-args|--allow-stock-kernel|--allow-research-blob|--allow-suspicious-refcnt)
      echo "ERROR: hitl_c refuses $a (ELF force + allowlist only; no blob override)" >&2
      exit 2
      ;;
    --blob=*|--cargo-args=*|--allow-*)
      echo "ERROR: hitl_c refuses $a (use lab_exec_once only for research flags)" >&2
      exit 2
      ;;
    *)
      echo "ERROR: unknown arg: $a (allowlist: --assume-l3-already-open --no-bo-diag --no-build --timeout-s=N --force-apply-despite-offline-red --skip-offline-contract)" >&2
      exit 2
      ;;
  esac
done

DIR="$ROOT/testdata/xdna2_cmd/silicon_lab"
ELF="$DIR/lab_instr.elf"
BIN="$DIR/lab_instr.bin"

echo "=== Milestone C: pure-direct Pass apply ==="

# Soft unlock stamp from RESIDUAL SSoT (NOTE only — never hard-blocks apply).
# Tolerate markdown: **C_UNLOCK=…**, `C_UNLOCK=…`, indent, mid-line.
# Detect CONSUMED: first C_UNLOCK= line (or any line with that stamp) near CONSUMED
# must not print as active "one-shot" after operator C2.
_c_unlock_tok=""
_c_unlock_consumed=0
if [[ -f "$ROOT/RESIDUAL.md" ]]; then
  _c_unlock_line="$(
    { grep -m1 -E 'C_UNLOCK=[A-Za-z0-9_]+' "$ROOT/RESIDUAL.md" || true; } | head -n1
  )"
  _c_unlock_tok="$(
    { grep -oE 'C_UNLOCK=[A-Za-z0-9_]+' <<<"${_c_unlock_line}" || true; } \
      | head -n1 | tr -d '`"*' | tr -d '[:space:]'
  )"
  if [[ -n "${_c_unlock_tok}" ]]; then
    _c_unlock_stamp="${_c_unlock_tok#C_UNLOCK=}"
    if grep -qiE 'CONSUMED' <<<"${_c_unlock_line}"; then
      _c_unlock_consumed=1
    elif grep -E "C_UNLOCK=${_c_unlock_stamp}" "$ROOT/RESIDUAL.md" \
      | grep -qiE 'CONSUMED'; then
      _c_unlock_consumed=1
    elif grep -qiE "${_c_unlock_stamp}[^A-Za-z0-9_].*CONSUMED|CONSUMED[^A-Za-z0-9_].*${_c_unlock_stamp}" \
      "$ROOT/RESIDUAL.md"; then
      _c_unlock_consumed=1
    fi
  fi
fi
if [[ -n "${_c_unlock_tok}" && "${_c_unlock_consumed}" -eq 1 ]]; then
  echo "NOTE: ${_c_unlock_tok} is **CONSUMED** (historical one-shot; not active)." >&2
  echo "NOTE: further C needs a **new** agent unlock + offline delta; restore L3; ≠ Pass / ≠ ran_on_npu." >&2
  echo "NOTE: unlock ≠ Pass ≠ permanent open season. L3=exec_cmd_allow; userspace ALLOW=XDNA2_ALLOW_EXEC_CMD (leave both closed after)." >&2
elif [[ -n "${_c_unlock_tok}" ]]; then
  echo "NOTE: ${_c_unlock_tok} one-shot (active); restore L3 (exec_cmd_allow=0); ≠ ran_on_npu until Pass" >&2
  echo "NOTE: unlock ≠ Pass ≠ permanent open season. L3=exec_cmd_allow; userspace ALLOW=XDNA2_ALLOW_EXEC_CMD (leave both closed after)." >&2
else
  echo "NOTE: no C_UNLOCK= stamp in RESIDUAL.md (soft; not a hard refuse)." >&2
  echo "NOTE: prefer agent unlock stamp before live apply; research force still allowed." >&2
fi

# Always emit unlock-ban (classic EXISTS SSoT; not gated on vendor miss).
echo "NOTE: unlock-ban — false unlocks alone never equal Pass:" >&2
echo "NOTE:   CONTRACT_OK / OFFLINE_GATES_OK / run_offline_gates green" >&2
echo "NOTE:   unpatched CLASS A / mere diff rc=0 (body match only)" >&2
echo "NOTE:   registry BO alone (Transaction/cache, not live run_ BO)" >&2
echo "NOTE:   residual_class / CLASSIFY_SOFT_OK / app_load_checklist" >&2
echo "NOTE: See RESIDUAL.md / docs/HITL_C_APP_LOAD_RECIPE.md." >&2

# Soft policy WARN: vendor miss (do not hard-block). Classic body may still EXISTS via sibling.
VENDOR_INSTR="$ROOT/testdata/xdna2_cmd/gt_capture/instr_vendor_post_patch.bin"
if [[ ! -f "$VENDOR_INSTR" ]]; then
  echo "WARN: vendor bin MISSING: $VENDOR_INSTR" >&2
  echo "WARN: CONTRACT_OK with miss ≠ C-ready. Optional capture if body missing:" >&2
  echo "WARN:   sudo sh -c 'echo 1 > /sys/module/amdxdna/parameters/exec_cmd_allow'" >&2
  echo "WARN:   bash tools/silicon_lab_operator.sh capture-instr" >&2
  echo "WARN:   sudo sh -c 'echo 0 > /sys/module/amdxdna/parameters/exec_cmd_allow'" >&2
  echo "WARN:   bash tools/diff_vendor_instr_parity.sh" >&2
  echo "WARN:     rc=0 unpatched CLASS A = body match only → still ≠ C-ready" >&2
  echo "WARN:     rc=2 mismatch + CLASS_HINT → agent offline fix first (not C-ready)" >&2
  echo "WARN:     rc=3 miss (this case) → capture optional; not C-ready" >&2
  echo "WARN: continuing (soft docs policy; not a hard refuse)." >&2
fi

if [[ ! -f "$ELF" ]]; then
  if [[ -f "$BIN" ]]; then
    echo "NOTE: staging aiebu ELF (raw .bin is not valid EXEC payload) …"
    bash tools/stage_aiebu_lab_instr.sh
  else
    echo "ERROR: missing $ELF — stage assets first" >&2
    echo "  bash tools/silicon_lab_operator.sh stage" >&2
    exit 2
  fi
fi
[[ -f "$ELF" ]] || { echo "ERROR: still no lab_instr.elf" >&2; exit 2; }

echo "BLOB: $ELF ($(wc -c <"$ELF" | tr -d ' ') B) — must be ELF, not raw txn"
head32="$(od -An -tx1 -N4 "$ELF" | tr -d ' \n')"
if [[ "$head32" != "7f454c46" ]]; then
  echo "ERROR: $ELF is not ELF magic (got $head32) — refuse raw txn EXEC" >&2
  exit 2
fi

# Offline gate: hard-fail unless --force-apply-despite-offline-red (A9).
# Skip: operator claim only — NOT proof of prior green (sticky env is loud-warned).
# Missing contract script: fail closed (exit 2) unless force-apply (research).
if [[ "$SKIP_CONTRACT" -eq 1 ]]; then
  CONTRACT_STATUS="skipped"
  echo "--- offline contract check SKIPPED (--skip-offline-contract / XDNA2_HITL_SKIP_CONTRACT) ---"
  echo "WARN: skip ≠ proof of prior CONTRACT_OK this session (sticky env can disable A9)." >&2
  echo "NOTE: ELF magic still enforced; preflight Pass assets still run. Prefer re-run contract after asset restage."
  if [[ "$SKIP_CONTRACT_FROM_ENV" -eq 1 ]]; then
    echo "WARN: XDNA2_HITL_SKIP_CONTRACT is set in environment (sticky). Unset when done re-applying." >&2
  fi
elif [[ -f "$ROOT/tools/silicon_contract_check.sh" ]]; then
  echo "--- offline contract check ---"
  set +e
  bash "$ROOT/tools/silicon_contract_check.sh" --require-pass-assets
  cc=$?
  set -e
  CONTRACT_RC="$cc"
  if [[ "$cc" -ne 0 ]]; then
    if [[ "$FORCE_APPLY" -eq 1 ]]; then
      CONTRACT_STATUS="red_forced"
      echo "WARN: silicon_contract_check rc=$cc — continuing (--force-apply-despite-offline-red)" >&2
    else
      echo "ERROR: offline contract red (rc=$cc). Fix offline or pass --force-apply-despite-offline-red" >&2
      exit 2
    fi
  else
    CONTRACT_STATUS="green"
  fi
else
  if [[ "$FORCE_APPLY" -eq 1 ]]; then
    CONTRACT_STATUS="missing_forced"
    echo "WARN: tools/silicon_contract_check.sh missing — continuing (--force-apply-despite-offline-red)" >&2
  else
    echo "ERROR: tools/silicon_contract_check.sh missing (fail closed). Restore script or pass --force-apply-despite-offline-red" >&2
    exit 2
  fi
fi

echo "--- preflight Pass assets ---"
bash tools/silicon_lab_preflight.sh --require-pass-assets

export XDNA2_LAB_BO_DIAG="${XDNA2_LAB_BO_DIAG:-$BO_DIAG}"

ALLOW_PATH="/sys/module/amdxdna/parameters/exec_cmd_allow"
read_allow() {
  if [[ -r "$ALLOW_PATH" ]]; then
    tr -d '[:space:]' <"$ALLOW_PATH" || true
  else
    echo "missing"
  fi
}
allow_now="$(read_allow)"
echo "driver: exec_cmd_allow=$allow_now"

# --assume-l3-already-open means YOU already opened L3 (hardened module).
# Stock in-tree amdxdna has no exec_cmd_allow sysfs → treat as stock path, proceed.
# If N/0 on hardened, fail early (not a silicon Pass failure).
if [[ ${#ASSUME[@]} -gt 0 ]]; then
  case "$allow_now" in
    1|Y|y) ;;
    missing)
      echo "NOTE: stock amdxdna (exec_cmd_allow missing) — no L3 open/restore; userspace ALLOW only" >&2
      echo "NOTE: wedge risk on stock is real (no L2 null-guards); --i-accept-wedge-risk still required via lab_exec" >&2
      ;;
    *)
      echo "" >&2
      echo "ERROR: --assume-l3-already-open but exec_cmd_allow=$allow_now (L3 closed)." >&2
      case "$CONTRACT_STATUS" in
        green)
          echo "This is NOT an NPU/FW failure — offline contract was green; you only skipped sudo open." >&2
          ;;
        skipped)
          echo "This is NOT an NPU/FW failure — offline contract was SKIPPED; you only skipped sudo open." >&2
          echo "(skip ≠ proof of prior green; ELF+preflight still ran)" >&2
          ;;
        red_forced)
          echo "This is NOT an NPU/FW failure — offline contract was RED but --force-apply; you only skipped sudo open." >&2
          ;;
        *)
          echo "This is NOT an NPU/FW failure — you only skipped sudo open (contract status=$CONTRACT_STATUS)." >&2
          ;;
      esac
      echo "" >&2
      # CONSUMED: do not nudge operator to re-open L3 under the old stamp
      # (parity with hitl_c2_once). Soft-NOTE only — still exit 1 (L3 closed);
      # never hard-block apply solely because stamp is CONSUMED when L3 is open.
      if [[ "${_c_unlock_consumed}" -eq 1 ]]; then
        echo "NOTE: ${_c_unlock_tok:-C_UNLOCK} is **CONSUMED** — do **not** open L3 under this stamp." >&2
        echo "NOTE: further live C needs a **NEW** agent unlock + offline delta first." >&2
        echo "NOTE: CONSUMED is soft-NOTE only (never hard-blocks apply if L3 already open)." >&2
        echo "NOTE: L3 closed (exec_cmd_allow=$allow_now) — correct while stamp is CONSUMED." >&2
        echo "Offline next (no L3; ≠ C-ready / ≠ Pass):" >&2
        echo "  bash tools/c3_post_c2_classify.sh" >&2
        echo "  bash tools/c4_pass_check.sh" >&2
        echo "  bash tools/run_offline_gates.sh" >&2
        echo "If L3 were left open after a prior failure, restore only:" >&2
        echo "  sudo sh -c 'echo 0 > /sys/module/amdxdna/parameters/exec_cmd_allow'" >&2
      else
        echo "Fish (recommended — open L3, apply, restore on **hardened** module):" >&2
        echo "  sudo sh -c 'echo 1 > /sys/module/amdxdna/parameters/exec_cmd_allow'" >&2
        echo "  bash tools/hitl_c_pass_apply.sh --assume-l3-already-open" >&2
        echo "  sudo sh -c 'echo 0 > /sys/module/amdxdna/parameters/exec_cmd_allow'" >&2
        echo "" >&2
        echo "Stock module (no exec_cmd_allow): omit sudo; just run apply (lab_exec detects missing)." >&2
        echo "  bash tools/hitl_c_pass_apply.sh --assume-l3-already-open" >&2
        echo "Or omit --assume-l3-already-open and let lab_exec try sudo -n / print human steps:" >&2
        echo "  bash tools/hitl_c_pass_apply.sh" >&2
      fi
      exit 1
      ;;
  esac
fi

EXTRA=()
[[ "$NO_BUILD" -eq 1 ]] && EXTRA+=(--no-build)
[[ -n "$TIMEOUT_S" ]] && EXTRA+=(--timeout-s "$TIMEOUT_S")

# Residual trial env banner (paste-friendly).
# CONFIG_CU / PDI preflight (loud; soft refuse only when --require-pdi set via env).
# Pin last_apply away from locked baseline when PDI set so C3 can detect new evidence.
_pdi="${XDNA2_LAB_PDI_BLOB:-}"
_pdi_trim="${_pdi#"${_pdi%%[![:space:]]*}"}"
_pdi_trim="${_pdi_trim%"${_pdi_trim##*[![:space:]]}"}"
if [[ -n "${_pdi_trim}" ]]; then
  if [[ ! -f "${_pdi_trim}" ]]; then
    echo "ERROR: XDNA2_LAB_PDI_BLOB set but not a file: ${_pdi_trim}" >&2
    echo "  re-run: bash tools/operator_app_load_kit.sh a-prep" >&2
    exit 2
  fi
  _pdi_bytes="$(wc -c <"${_pdi_trim}" | tr -d ' ')"
  if [[ "${_pdi_bytes}" -eq 0 ]]; then
    echo "ERROR: XDNA2_LAB_PDI_BLOB is empty (0 B): ${_pdi_trim}" >&2
    exit 2
  fi
  export XDNA2_LAB_PDI_BLOB="${_pdi_trim}"
  echo "CONFIG_CU preflight: PDI_OK path=${_pdi_trim} bytes=${_pdi_bytes} cu_func=${XDNA2_LAB_CU_FUNC:-0}"
  echo "  honesty: attach alone ≠ Pass ≠ ran_on_npu; research overlay may still 0x3000003"
  # Prefer a shot-specific last_apply under /tmp so locked fixture stays C3 baseline.
  if [[ -z "${XDNA2_LAB_LAST_APPLY_JSON:-}" ]]; then
    _la_tmp="${TMPDIR:-/tmp}"
    _la_tmp="${_la_tmp%/}"
    if [[ -z "${_la_tmp}" || "${_la_tmp}" == "/" ]]; then
      _la_tmp="/tmp"
    fi
    export XDNA2_LAB_LAST_APPLY_JSON="${_la_tmp}/dragon_npu_last_apply_config_cu_$$.json"
    echo "CONFIG_CU preflight: pinned LAST_APPLY=${XDNA2_LAB_LAST_APPLY_JSON}"
    echo "  (locked baseline testdata/xdna2_cmd/last_apply.json preserved for C3 compare)"
  else
    echo "CONFIG_CU preflight: LAST_APPLY already set=${XDNA2_LAB_LAST_APPLY_JSON}"
  fi
else
  echo "WARN: XDNA2_LAB_PDI_BLOB unset — this apply will NOT issue CONFIG_CU" >&2
  echo "WARN: for active-unlock CONFIG_CU measurement, run a-prep + export first:" >&2
  echo "WARN:   bash tools/hitl_config_cu_once.sh prep   # recommended one-shot prep" >&2
  echo "WARN:   # or: bash tools/operator_app_load_kit.sh a-prep && set -gx XDNA2_LAB_PDI_BLOB …" >&2
  if [[ "${XDNA2_HITL_REQUIRE_PDI:-}" == "1" ]]; then
    echo "ERROR: XDNA2_HITL_REQUIRE_PDI=1 but PDI unset (fail closed)" >&2
    exit 2
  fi
fi

# Product Arm A / live-shot-effective: default same-run address gate when unset.
# Operator may force off with XDNA2_LAB_SAME_SHOT_RELA=0. Env alone ≠ Pass ≠ unlock.
# lab_exec_once must forward this into the cargo child (explicit RUN_ENV).
if [[ -z "${XDNA2_LAB_SAME_SHOT_RELA+x}" || -z "${XDNA2_LAB_SAME_SHOT_RELA}" ]]; then
  export XDNA2_LAB_SAME_SHOT_RELA=1
fi
case "${XDNA2_LAB_SAME_SHOT_RELA}" in
  1|true|TRUE|yes|YES|on|ON)
    echo "SAME_SHOT_RELA: XDNA2_LAB_SAME_SHOT_RELA=${XDNA2_LAB_SAME_SHOT_RELA} (in-process BO paddrs only; refuse stale disk UVAs)"
    echo "  honesty: same_shot_gate alone ≠ Pass ≠ unlock ≠ ran_on_npu invent_pass=0"
    ;;
  *)
    echo "SAME_SHOT_RELA: XDNA2_LAB_SAME_SHOT_RELA=${XDNA2_LAB_SAME_SHOT_RELA} (product Arm A prefers 1)"
    ;;
esac

echo "RESIDUAL_ENV: INSTR_SIZE_MODE=${XDNA2_LAB_INSTR_SIZE_MODE:-content(default)} \
RELA_MODE=${XDNA2_LAB_RELA_MODE:-overwrite(default)} \
KERNEL_ARG_LAYOUT=${XDNA2_LAB_KERNEL_ARG_LAYOUT:-count4(default)} \
PDI_BLOB=${XDNA2_LAB_PDI_BLOB:-(unset)} \
CU_FUNC=${XDNA2_LAB_CU_FUNC:-0(default)} \
LAST_APPLY=${XDNA2_LAB_LAST_APPLY_JSON:-(default fixture path)} \
SAME_SHOT_RELA=${XDNA2_LAB_SAME_SHOT_RELA:-(unset)} \
contract_status=$CONTRACT_STATUS"

echo "--- apply (wedge risk accepted by flag; ELF forced last) ---"
STOCK_EXTRA=()
if [[ "$allow_now" == "missing" ]]; then
  STOCK_EXTRA+=(--allow-stock-kernel)
  echo "NOTE: passing --allow-stock-kernel (stock amdxdna; no hardened L2+L3)"
fi
set +e
bash scripts/lab_exec_once.sh --silicon-mladf --require-pass-assets \
  --apply --i-accept-wedge-risk \
  "${ASSUME[@]}" \
  "${STOCK_EXTRA[@]}" \
  "${EXTRA[@]}" \
  --blob "$ELF"
rc=$?
set -e

allow="?"
if [[ -r /sys/module/amdxdna/parameters/exec_cmd_allow ]]; then
  allow="$(tr -d '[:space:]' </sys/module/amdxdna/parameters/exec_cmd_allow || true)"
else
  allow="missing"
fi
echo ""
echo "POST: lab_exec rc=$rc  exec_cmd_allow=$allow"
case "$allow" in
  0|N|n) echo "POST_OK: L3 closed" ;;
  missing) echo "POST_OK: stock module (no L3 param to restore)" ;;
  *)
    echo "CRITICAL: restore L3:" >&2
    echo "  sudo sh -c 'echo 0 > /sys/module/amdxdna/parameters/exec_cmd_allow'" >&2
    # Always restore-failed class when L3 still open (never mask prior rc with
    # non-5 exits such as 1/2/3). Prior code only elevated 0/4.
    rc=5
    ;;
esac

echo ""
echo "What to paste for the agent (if !ran_on_npu):"
echo "  - status=… line / TimedOpResult JSON status field"
echo "  - detail: config_cu= / config_cu_failed / pdi_bytes= / cu_func= (CONFIG_CU tokens)"
echo "  - detail: aiebu_elf= rela_count= qos_gops= tiles= ert_state= ofm_stats= fw_status_hint="
echo "  - residual: inst_size_mode= content_size= bo_size= start_npu_size= rela_mode= layout="
echo "  - RESIDUAL_ENV banner line from this run"
if [[ -n "${XDNA2_LAB_LAST_APPLY_JSON:-}" ]]; then
  echo "  - last_apply: ${XDNA2_LAB_LAST_APPLY_JSON}"
  if [[ -f "${XDNA2_LAB_LAST_APPLY_JSON}" ]]; then
    echo "  - last_apply EXISTS ($(wc -c <"${XDNA2_LAB_LAST_APPLY_JSON}" | tr -d ' ') B)"
  else
    echo "  - last_apply MISSING at pin (timed path may not have written — still paste status/detail)"
  fi
else
  echo "  - last_apply: testdata/xdna2_cmd/last_apply.json or \$XDNA2_LAB_LAST_APPLY_JSON"
fi
echo "  - journal: journalctl -k -b --no-pager | rg -i 'Failed cmd|Status 0x|APP_LOAD|amdxdna' | tail -30"
echo "  - PDI provenance: path + bytes + sha256 + xclbin source (from a-prep)"
echo "  - exec_cmd_allow after restore (must be 0/N, or missing on stock)"
echo ""
echo "Post-shot offline (no L3; paste outputs):"
if [[ -n "${XDNA2_LAB_LAST_APPLY_JSON:-}" && -f "${XDNA2_LAB_LAST_APPLY_JSON}" ]]; then
  echo "  bash tools/collect_post_c2_evidence.sh ${XDNA2_LAB_LAST_APPLY_JSON}"
else
  echo "  bash tools/collect_post_c2_evidence.sh"
fi
echo ""
echo "Success signal: ran_on_npu=true (bench exit 0). Common: exit 4 until Pass."
echo "Mark the active unlock used in RESIDUAL only after a real sandwich (not print-only)."
exit "$rc"
