#!/usr/bin/env bash
# One-stop silicon-lab helper for the human operator (offline by default).
#
# Does NOT open L3, set XDNA2_ALLOW_EXEC_CMD, or claim ran_on_npu (except
# `apply`, which still requires explicit --i-accept-wedge-risk via lab_exec).
#
# Usage (from repo root):
#   bash tools/silicon_lab_operator.sh status
#   bash tools/silicon_lab_operator.sh stage          # offline assets
#   bash tools/silicon_lab_operator.sh preflight      # require-pass (needs golden)
#   bash tools/silicon_lab_operator.sh dry-run
#   bash tools/silicon_lab_operator.sh check          # offline contract gate
#   bash tools/silicon_lab_operator.sh gt-offline     # dump patch oracle (no L3)
#   bash tools/silicon_lab_operator.sh apply          # HITL Pass: prefer lab_instr.elf
#   bash tools/silicon_lab_operator.sh capture-golden # vendor OFM (needs L3)
#   bash tools/silicon_lab_operator.sh capture-instr  # vendor post-patch instr (needs L3 once)
#   bash tools/silicon_lab_operator.sh install-golden /path/to/ofm.bin
#   bash tools/silicon_lab_operator.sh extract-paddrs # last_apply → vendor_paddrs JSON
#   bash tools/hitl_b_gt_capture.sh [--with-vendor]   # Milestone B (preferred)
#   bash tools/hitl_c_pass_apply.sh                   # Milestone C only if agent says ready
#     (C only if agent ready; unpatched CLASS A / diff rc=0 / registry alone ≠ C-ready)
#
# Exit: 0 ok; 2 usage / fail-closed missing assets; 3 golden zero+ALLOW style
#       apply: same exit codes as scripts/lab_exec_once.sh (4 = !ran_on_npu common)
set -euo pipefail
export PATH="/usr/bin:${HOME}/.local/bin:${PATH}"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIR="${SILICON_LAB_DIR:-$ROOT/testdata/xdna2_cmd/silicon_lab}"
WTS_EXPORT="$ROOT/testdata/xdna2_cmd/mladf_wts_export/wts.bin"
CMD="${1:-}"
shift || true

die() { echo "ERROR: $*" >&2; exit 2; }
note() { echo "$*"; }

usage() {
  sed -n '1,25p' "$0" | sed 's/^# \?//'
  exit 2
}

read_exec_cmd_allow() {
  local p="/sys/module/amdxdna/parameters/exec_cmd_allow"
  if [[ -r "$p" ]]; then
    tr -d '[:space:]' <"$p" || true
  else
    echo "missing"
  fi
}

cmd_status() {
  note "=== silicon lab status (offline) ==="
  note "dir: $DIR"
  for f in lab_instr.elf lab_instr.bin lab_ctrl.bin lab_ctrl_meta.json wts.bin ofm_golden.bin ifm.bin; do
    if [[ -f "$DIR/$f" ]]; then
      local n
      n=$(wc -c <"$DIR/$f" | tr -d ' ')
      note "  OK  $f  ($n B)"
    else
      note "  --  $f  (missing)"
    fi
  done
  if [[ -d "$ROOT/testdata/xdna2_cmd/gt_capture" && -f "$ROOT/testdata/xdna2_cmd/gt_capture/MANIFEST.sha256" ]]; then
    note "  OK  gt_capture/ (offline patch oracle)"
  else
    note "  --  gt_capture/ missing — bash tools/hitl_b_gt_capture.sh (offline, no sudo)"
  fi
  if [[ -f "$WTS_EXPORT" ]]; then
    note "  OK  export wts: $WTS_EXPORT ($(wc -c <"$WTS_EXPORT" | tr -d ' ') B)"
  else
    note "  --  export wts missing (run stage or export_mladf_wts.sh)"
  fi
  # Driver L3 + real readiness (preflight size/nonzero — not existence-only).
  local allow pre pf_rc=0
  allow="$(read_exec_cmd_allow)"
  note ""
  note "driver: exec_cmd_allow=$allow  (expect N/0 outside lab apply; restore after apply)"
  set +e
  bash "$ROOT/tools/silicon_lab_preflight.sh" --dir "$DIR" --require-pass-assets >/dev/null 2>&1
  pf_rc=$?
  set -e
  if [[ "$pf_rc" -eq 0 ]]; then
    pre="Pass-candidate ready (preflight --require-pass-assets OK: instr+WTS nz+CTRL nz+golden nz)"
  elif [[ ! -f "$DIR/lab_instr.elf" && ! -f "$DIR/lab_instr.bin" ]]; then
    pre="need stage (instr/ctrl/WTS) — preflight require-pass failed (rc=$pf_rc)"
  elif [[ ! -f "$DIR/ofm_golden.bin" ]]; then
    pre="need non-zero OFM golden — capture-golden or install-golden (preflight rc=$pf_rc)"
  else
    pre="files present but preflight --require-pass-assets FAILED (rc=$pf_rc: zero/size/missing/raw) — run preflight"
  fi
  note "readiness: $pre"
  note ""
  note "Exit codes: 0 ok; 2 missing/refused; 3 zero-golden ALLOW; apply 4=!ran_on_npu; 5 L3 restore fail"
  note "Next:"
  if [[ ! -f "$DIR/lab_instr.elf" && ! -f "$DIR/lab_instr.bin" ]]; then
    note "  1) bash tools/silicon_lab_operator.sh stage"
  elif [[ ! -f "$DIR/ofm_golden.bin" ]]; then
    note "  1) stage OK — capture OFM golden (Option A, vendor NPU):"
    note "     # if hardened L3 closed, temporarily:"
    note "     #   echo 1 | sudo tee /sys/module/amdxdna/parameters/exec_cmd_allow"
    note "     bash tools/silicon_lab_operator.sh capture-golden"
    note "     #   echo 0 | sudo tee /sys/module/amdxdna/parameters/exec_cmd_allow"
  elif [[ "$pf_rc" -ne 0 ]]; then
    note "  1) bash tools/silicon_lab_operator.sh preflight   # fix zero/size assets"
  else
    note "  1) bash tools/silicon_lab_operator.sh check          # offline contract (≠ C-ready)"
    if [[ ! -f "$ROOT/testdata/xdna2_cmd/gt_capture/instr_vendor_post_patch.bin" ]]; then
      note "  2) optional capture if classic body missing (one L3 window — ban env C):"
      note "     sudo sh -c 'echo 1 > /sys/module/amdxdna/parameters/exec_cmd_allow'"
      note "     bash tools/silicon_lab_operator.sh capture-instr"
      note "     sudo sh -c 'echo 0 > /sys/module/amdxdna/parameters/exec_cmd_allow'"
      note "     bash tools/diff_vendor_instr_parity.sh"
    else
      note "  2) vendor instr present — bash tools/diff_vendor_instr_parity.sh"
      note "     (unpatched CLASS A / mere rc=0 = body match only ≠ C-ready)"
      note "  3) offline: maintain pure-direct tests/contract (TDD landed); APP_LOAD is silicon residual"
      note "  4) hitl_c_pass_apply ONLY if agent says ready"
      note "     (not unpatched A / CONTRACT_OK / registry cache / parity green;"
      note "      INSTR_BO true post-patch public path closed — registry ≠ live run_ BO)"
    fi
  fi
}

cmd_apply() {
  note "=== HITL apply (explicit wedge risk; no password storage) ==="
  note "Prefer: bash tools/hitl_c_pass_apply.sh  (forces aiebu ELF + offline gate)"
  note "Delegates to hitl_c (A9: refuses unless offline contract green unless --force-apply)."
  note "After apply: verify exec_cmd_allow is N/0."
  note "Optional BO diag: export XDNA2_LAB_BO_DIAG=1"
  note ""
  # Only forward allowlisted flags (no arbitrary --blob / cargo-args).
  local fwd=()
  local a
  for a in "$@"; do
    case "$a" in
      --assume-l3-already-open|--assume-l3-open|--no-bo-diag|--no-build|--force-apply-despite-offline-red|--force-apply)
        fwd+=("$a")
        ;;
      --timeout-s=*)
        fwd+=("$a")
        ;;
      -h|--help)
        bash "$ROOT/tools/hitl_c_pass_apply.sh" --help
        return 2
        ;;
      *)
        die "operator apply refuses unknown arg: $a (use hitl_c allowlist)"
        ;;
    esac
  done
  local rc=0
  set +e
  bash "$ROOT/tools/hitl_c_pass_apply.sh" "${fwd[@]}"
  rc=$?
  set -e
  note ""
  local allow_post
  allow_post="$(read_exec_cmd_allow)"
  note "POST: exec_cmd_allow=$allow_post  (expect N or 0)"
  note "      bash scripts/diagnose_amdxdna.sh"
  case "$allow_post" in
    0|N|n)
      note "POST_OK: L3 closed"
      ;;
    *)
      note "CRITICAL: exec_cmd_allow still open ($allow_post) — restore manually:" >&2
      note "  sudo sh -c 'echo 0 > /sys/module/amdxdna/parameters/exec_cmd_allow'" >&2
      # Always restore-failed class when L3 still open (never mask prior rc).
      # hitl_c does the same; defense-in-depth if wrapper is the outer exit.
      rc=5
      ;;
  esac
  return "$rc"
}

cmd_check() {
  bash "$ROOT/tools/silicon_contract_check.sh" "$@"
}

cmd_gt_offline() {
  bash "$ROOT/tools/dump_mladf_gt_offline.sh"
}

cmd_capture_golden() {
  note "=== Priority 1 Option A: vendor OFM capture (instrumented) ==="
  note "Uses tools/capture_mladf_ofm_golden.sh (DD load_xrt=true + execute)."
  note "Same lab-synthetic tensors as export_mladf_wts; IFM=bf16 ones."
  note "This helper does NOT own L3 restore — operator must close exec_cmd_allow after."
  if [[ ! -f "$DIR/lab_instr.bin" ]]; then
    note "WARN: silicon_lab not staged yet — capturing golden anyway; stage before preflight."
  fi
  local rc=0
  set +e
  # Always install into silicon_lab when capture succeeds.
  bash "$ROOT/tools/capture_mladf_ofm_golden.sh" --install
  rc=$?
  set -e
  note ""
  local allow_post
  allow_post="$(read_exec_cmd_allow)"
  note "POST capture-golden: exec_cmd_allow=$allow_post"
  case "$allow_post" in
    0|N|n)
      note "L3 already closed (good)."
      ;;
    *)
      note "CRITICAL: exec_cmd_allow=$allow_post still open — restore NOW:" >&2
      note "  sudo sh -c 'echo 0 > /sys/module/amdxdna/parameters/exec_cmd_allow'" >&2
      # Match capture-instr: L3 open → rc=5 for ANY base rc (success or fail),
      # unless research override XDNA2_CAPTURE_ALLOW_L3_OPEN=1.
      if [[ "${XDNA2_CAPTURE_ALLOW_L3_OPEN:-0}" == "1" ]]; then
        note "WARN: XDNA2_CAPTURE_ALLOW_L3_OPEN=1 — not elevating to exit 5 (base rc=$rc)" >&2
      else
        note "CRITICAL: L3 still open after capture-golden → exit 5 (base was $rc)" >&2
        rc=5
      fi
      ;;
  esac
  return "$rc"
}

cmd_capture_instr() {
  note "=== Optional vendor instruction capture (one L3 window) ==="
  note "Optional logs only; residual highest-value = pure-direct / APP_LOAD"
  note "pure-direct / APP_LOAD residual (no more KERNEL_ARG / INSTR_SIZE env C applies)."
  note "Uses tools/capture_mladf_instr_vendor.sh — vendor DD NPU execute + instr dump."
  note "This helper does NOT own L3 restore — operator must close exec_cmd_allow after."
  note ""
  note "One-liner (you run if needed):"
  note "  sudo sh -c 'echo 1 > /sys/module/amdxdna/parameters/exec_cmd_allow'"
  note "  bash tools/silicon_lab_operator.sh capture-instr"
  note "  sudo sh -c 'echo 0 > /sys/module/amdxdna/parameters/exec_cmd_allow'"
  note "  bash tools/diff_vendor_instr_parity.sh   # unpatched A ≠ C-ready"
  note ""
  local rc=0
  set +e
  bash "$ROOT/tools/capture_mladf_instr_vendor.sh" --install
  rc=$?
  set -e
  note ""
  local allow_post
  allow_post="$(read_exec_cmd_allow)"
  note "POST capture-instr: exec_cmd_allow=$allow_post"
  case "$allow_post" in
    0|N|n)
      note "L3 already closed (good)."
      ;;
    *)
      note "CRITICAL: exec_cmd_allow=$allow_post still open — restore NOW:" >&2
      note "  sudo sh -c 'echo 0 > /sys/module/amdxdna/parameters/exec_cmd_allow'" >&2
      # Issue 6: elevate L3 open → 5 for ANY base rc (success or fail),
      # unless research override (matches capture_mladf_instr_vendor.sh).
      if [[ "${XDNA2_CAPTURE_ALLOW_L3_OPEN:-0}" == "1" ]]; then
        note "WARN: XDNA2_CAPTURE_ALLOW_L3_OPEN=1 — not elevating to exit 5 (base rc=$rc)" >&2
      else
        note "CRITICAL: L3 still open after capture-instr → exit 5 (base was $rc)" >&2
        rc=5
      fi
      ;;
  esac
  note ""
  note "Offline next (no sudo):"
  note "  bash tools/diff_vendor_instr_parity.sh"
  note "Ban: env C applies; unpatched CLASS A / mere diff rc=0 ≠ C-ready"
  note "Honesty: CAPTURE_MODE_UNPATCHED ≠ live run_ BO; registry=Transaction/cache only; C only if agent ready"
  note "C only if agent says ready (not unlocked by body match alone)"
  return "$rc"
}

cmd_extract_paddrs() {
  note "=== extract io_paddrs from last_apply.json → vendor_paddrs JSON ==="
  bash "$ROOT/tools/extract_paddrs_from_last_apply.sh" "$@"
}

cmd_stage() {
  note "=== stage silicon assets (Priority 2) ==="
  note "Uses existing export WTS if present, else --export-wts (needs DD+XRT)."
  if [[ -f "$WTS_EXPORT" ]]; then
    # Prefer already-exported non-zero WTS (fast).
    bash "$ROOT/tools/stage_silicon_lab_assets.sh" "$DIR" --wts-from "$WTS_EXPORT"
  else
    bash "$ROOT/tools/stage_silicon_lab_assets.sh" "$DIR" --export-wts
  fi
  note ""
  note "Staged under $DIR"
  note "OFM golden still missing unless you install one:"
  note "  bash tools/silicon_lab_operator.sh golden-help"
  note "Soft preflight (warns on missing golden):"
  note "  bash tools/silicon_lab_preflight.sh --print-env"
}

cmd_preflight() {
  local extra=()
  if [[ "${1:-}" == "--soft" ]]; then
    shift || true
  else
    extra+=(--require-pass-assets)
  fi
  bash "$ROOT/tools/silicon_lab_preflight.sh" --dir "$DIR" --print-env "${extra[@]}"
}

cmd_dry_run() {
  # Soft dry-run always; require-pass if assets complete.
  if [[ -f "$DIR/ofm_golden.bin" && -f "$DIR/lab_instr.bin" ]]; then
    bash "$ROOT/scripts/lab_exec_once.sh" --silicon-mladf --require-pass-assets
  else
    note "WARN: missing golden or instr — dry-run without --require-pass-assets"
    bash "$ROOT/scripts/lab_exec_once.sh" --silicon-mladf
  fi
}

cmd_install_golden() {
  local src="${1:-}"
  [[ -n "$src" ]] || die "install-golden needs PATH to ofm golden file"
  [[ -f "$src" ]] || die "not a file: $src"
  bash "$ROOT/tools/validate_mladf_ofm_golden.sh" "$src"
  mkdir -p "$DIR"
  cp -a "$src" "$DIR/ofm_golden.bin"
  # Provenance sidecar
  {
    echo "# OFM golden install"
    echo "- date_utc: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo "- source: $src"
    echo "- staged: $DIR/ofm_golden.bin"
    echo "- size: $(wc -c <"$DIR/ofm_golden.bin" | tr -d ' ')"
    echo "- sha256: $(sha256sum "$DIR/ofm_golden.bin" | awk '{print $1}')"
    echo "- note: Pass golden only if from trusted capture of same key+WTS; not ofm_expected zeros"
  } >"$DIR/ofm_golden.PROVENANCE.md"
  note "Installed + validated:"
  note "  $DIR/ofm_golden.bin"
  note "  $DIR/ofm_golden.PROVENANCE.md"
  note ""
  note "export XDNA2_LAB_OFM_GOLDEN=$DIR/ofm_golden.bin"
  note "Then: bash tools/silicon_lab_operator.sh preflight"
}

cmd_golden_help() {
  cat <<'EOF'
=== Priority 1: OFM golden (512 B bf16, non-zero) ===

Instrumented Option A (what you asked for — just run a script):

  # 1) stage already done on your machine
  # 2) temporary L3 if hardened (capture uses vendor XRT EXEC):
  echo 1 | sudo tee /sys/module/amdxdna/parameters/exec_cmd_allow
  bash tools/silicon_lab_operator.sh capture-golden
  echo 0 | sudo tee /sys/module/amdxdna/parameters/exec_cmd_allow

That builds tools/capture_mladf_ofm_golden (DD load_xrt=true), runs NPU execute
with the same synthetic WTS tensors as export_mladf_wts + bf16 ones IFM, writes
ofm_golden.bin, validates, and installs into silicon_lab/.

Then:
  bash tools/silicon_lab_operator.sh preflight

----------------------------------------------------------
Manual install (if you already have a 512 B capture file)
----------------------------------------------------------
  bash tools/silicon_lab_operator.sh install-golden /path/to/ofm.bin

----------------------------------------------------------
Do NOT use as Pass golden
----------------------------------------------------------
  ofm_expected.bin (zeros), all-zero 512 B, f32 ones×ones for a16w4

See docs/SILICON_PATH_LAB.md Phase C.
EOF
}

case "$CMD" in
  status|"") cmd_status ;;
  stage) cmd_stage ;;
  preflight) cmd_preflight "${1:-}" ;;
  dry-run) cmd_dry_run ;;
  check) cmd_check "$@" ;;
  gt-offline) cmd_gt_offline ;;
  apply) cmd_apply "$@" ;;
  capture-golden) cmd_capture_golden ;;
  capture-instr) cmd_capture_instr ;;
  extract-paddrs) cmd_extract_paddrs "$@" ;;
  install-golden) cmd_install_golden "${1:-}" ;;
  golden-help) cmd_golden_help ;;
  -h|--help) usage ;;
  *) die "unknown command: $CMD (try status|stage|check|gt-offline|capture-golden|capture-instr|extract-paddrs|preflight|dry-run|apply|install-golden|golden-help)" ;;
esac
