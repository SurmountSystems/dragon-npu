#!/usr/bin/env bash
# One entry point: path A (CONFIG_CU / PDI) + path B (external capture).
#
# You should not need to rediscover flags. Default = print both tracks.
#
#   bash tools/operator_app_load_kit.sh              # overview + status
#   bash tools/operator_app_load_kit.sh a-prep        # dump PDI + print env
#   bash tools/operator_app_load_kit.sh b-status      # capture dir ready?
#   bash tools/operator_app_load_kit.sh b-steps       # capture commands only
#   bash tools/operator_app_load_kit.sh b-install PATH [NAME]
#   bash tools/operator_app_load_kit.sh b-compare
#   bash tools/operator_app_load_kit.sh offline-gates # soft offline suite
#
# Never opens L3 / never sets ALLOW / never invents Pass.
set -euo pipefail
export PATH="/usr/bin:${HOME}/.local/bin:${PATH}"
unset XDNA2_ALLOW_EXEC_CMD || true

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
TOOLS="$ROOT/tools"

cmd="${1:-overview}"
shift || true

case "$cmd" in
  overview|""|-h|--help)
    cat <<'EOF'
=== APP_LOAD operator kit (A + B) ===

Residual: firmware load reject 0x3000003 after userspace ELF+rela OK.
Unlock C_UNLOCK=b17b0119 is CONSUMED — no live re-apply under that stamp.

Path A — CONFIG_CU + opaque PDI (named offline path change; agent-wired):
  1) bash tools/operator_app_load_kit.sh a-prep
       → dumps PDI from host AIE2P xclbin, prints XDNA2_LAB_PDI_BLOB=
  2) Agent/offline: lab timed path reads that env after CREATE_HWCTX and
     issues CONFIG_HWCTX(CONFIG_CU). Default off when env unset.
  3) Live try only after **new** host-minted C_UNLOCK + L3 sandwich
     (hitl_c2_once). Wrong overlay may still 0x3000003 — still progress.

Path B — external working-stack ctrltext capture (human, when stack works):
  1) bash tools/operator_app_load_kit.sh b-steps
  2) bash tools/operator_app_load_kit.sh b-install /path/to/ctrltext.bin
  3) Fill PROVENANCE table under testdata/xdna2_cmd/external_known_good/
  4) bash tools/operator_app_load_kit.sh b-compare

Honesty: invent_pass=0 not_c_ready=1 not_ran_on_npu=1 product_known_good=0
EOF
    echo
    bash "$TOOLS/operator_external_capture.sh" --status || true
    echo
    echo "PDI dump candidates:"
    bash "$TOOLS/dump_xclbin_pdi.sh" --list-default || true
    exit 0
    ;;
  a-prep|a|pdi)
    bash "$TOOLS/dump_xclbin_pdi.sh" --print-env "$@"
    exit $?
    ;;
  b-status)
    bash "$TOOLS/operator_external_capture.sh" --status
    exit $?
    ;;
  b-steps)
    bash "$TOOLS/operator_external_capture.sh" --print-steps
    exit $?
    ;;
  b-install)
    bash "$TOOLS/operator_external_capture.sh" --install "$@"
    exit $?
    ;;
  b-compare)
    bash "$TOOLS/operator_external_capture.sh" --compare
    exit $?
    ;;
  offline-gates|gates)
    bash "$TOOLS/run_offline_gates.sh" "$@"
    exit $?
    ;;
  *)
    echo "ERROR: unknown command: $cmd" >&2
    echo "allowlist: overview a-prep b-status b-steps b-install b-compare offline-gates" >&2
    exit 2
    ;;
esac
