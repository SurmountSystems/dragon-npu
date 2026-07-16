#!/usr/bin/env bash
# One-stop silicon-lab helper for the human operator (offline by default).
#
# Does NOT open L3, set XDNA2_ALLOW_EXEC_CMD, or claim ran_on_npu.
#
# Usage (from repo root):
#   bash tools/silicon_lab_operator.sh status
#   bash tools/silicon_lab_operator.sh stage          # Priority 2
#   bash tools/silicon_lab_operator.sh preflight      # require-pass (needs golden)
#   bash tools/silicon_lab_operator.sh dry-run
#   bash tools/silicon_lab_operator.sh capture-golden   # Priority 1 Option A (vendor NPU)
#   bash tools/silicon_lab_operator.sh install-golden /path/to/ofm.bin
#   bash tools/silicon_lab_operator.sh golden-help
#
# Exit: 0 ok; 2 usage / fail-closed missing assets; 3 golden zero+ALLOW style
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
  sed -n '1,22p' "$0" | sed 's/^# \?//'
  exit 2
}

cmd_status() {
  note "=== silicon lab status (offline) ==="
  note "dir: $DIR"
  for f in lab_instr.bin lab_ctrl.bin lab_ctrl_meta.json wts.bin ofm_golden.bin ifm.bin; do
    if [[ -f "$DIR/$f" ]]; then
      local n
      n=$(wc -c <"$DIR/$f" | tr -d ' ')
      note "  OK  $f  ($n B)"
    else
      note "  --  $f  (missing)"
    fi
  done
  if [[ -f "$WTS_EXPORT" ]]; then
    note "  OK  export wts: $WTS_EXPORT ($(wc -c <"$WTS_EXPORT" | tr -d ' ') B)"
  else
    note "  --  export wts missing (run stage or export_mladf_wts.sh)"
  fi
  note ""
  note "Next:"
  if [[ ! -f "$DIR/lab_instr.bin" ]]; then
    note "  1) bash tools/silicon_lab_operator.sh stage"
  elif [[ ! -f "$DIR/ofm_golden.bin" ]]; then
    note "  1) stage OK — capture OFM golden (Option A, vendor NPU):"
    note "     # if hardened L3 closed, temporarily:"
    note "     #   echo 1 | sudo tee /sys/module/amdxdna/parameters/exec_cmd_allow"
    note "     bash tools/silicon_lab_operator.sh capture-golden"
    note "     #   echo 0 | sudo tee /sys/module/amdxdna/parameters/exec_cmd_allow"
  else
    note "  1) bash tools/silicon_lab_operator.sh preflight"
    note "  2) bash tools/silicon_lab_operator.sh dry-run"
  fi
}

cmd_capture_golden() {
  note "=== Priority 1 Option A: vendor OFM capture (instrumented) ==="
  note "Uses tools/capture_mladf_ofm_golden.sh (DD load_xrt=true + execute)."
  note "Same lab-synthetic tensors as export_mladf_wts; IFM=bf16 ones."
  if [[ ! -f "$DIR/lab_instr.bin" ]]; then
    note "WARN: silicon_lab not staged yet — capturing golden anyway; stage before preflight."
  fi
  # Always install into silicon_lab when capture succeeds.
  bash "$ROOT/tools/capture_mladf_ofm_golden.sh" --install
  note ""
  note "If capture needed L3 open, restore exec_cmd_allow=0 now."
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
  capture-golden) cmd_capture_golden ;;
  install-golden) cmd_install_golden "${1:-}" ;;
  golden-help) cmd_golden_help ;;
  -h|--help) usage ;;
  *) die "unknown command: $CMD (try status|stage|capture-golden|preflight|dry-run|install-golden|golden-help)" ;;
esac
