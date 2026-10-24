#!/usr/bin/env bash
# One entry point: path A (CONFIG_CU / PDI) + path B (external capture).
#
# You should not need to rediscover flags. Default = print both tracks.
#
#   bash tools/operator_app_load_kit.sh              # overview + status
#   bash tools/operator_app_load_kit.sh a-prep        # dump PDI + print env
#   bash tools/operator_app_load_kit.sh config-cu      # full CONFIG_CU prep + sandwich print
#   bash tools/operator_app_load_kit.sh config-cu-run  # stock: prep + live apply + collect (no sudo)
#   bash tools/operator_app_load_kit.sh config-cu-skip # A/B prep without CONFIG_CU (unset PDI)
#   bash tools/operator_app_load_kit.sh config-cu-run-skip  # stock live without CONFIG_CU
#   bash tools/operator_app_load_kit.sh b-status      # capture dir ready?
#   bash tools/operator_app_load_kit.sh b-steps       # capture commands only
#   bash tools/operator_app_load_kit.sh b-install PATH [NAME]
#   bash tools/operator_app_load_kit.sh b-compare
#   bash tools/operator_app_load_kit.sh offline-gates # soft offline suite
#   bash tools/operator_app_load_kit.sh post-evidence [JSON]  # after live sandwich
#   bash tools/operator_app_load_kit.sh pass-unblock […]  # one Pass-unblock entrypoint
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

Residual: still no product Pass. Latest live 2ae69036: flexml STX unified-2x4x4
        CONFIG_CU attach measured, status=wait_cmd_einval (not Pass). Prior CVML
        97f38307 / 11bc9016: wait_cmd_einval + orphan IO_PAGE_FAULT. Historical
        APP_LOAD 0x3000003 was a different pin.
Unlock: No active unlock. CONSUMED: 2ae69036 (flexml), 97f38307, 11bc9016,
        3a70fabf, b17b0119. Do not reopen.

PRIMARY — what unblocks Pass next (one entrypoint):
  bash tools/pass_unblock_once.sh
  bash tools/pass_unblock_once.sh --agent-offline
  bash tools/pass_unblock_once.sh --ingest /path/to/working_stack_ctrltext.bin
  bash tools/pass_unblock_once.sh --ingest-pdi /path/to/research.pdi   # /tmp mktemp; not known-good
  bash tools/pass_unblock_once.sh --live   # exit 3 unless active C_UNLOCK in RESIDUAL
  alias: bash tools/operator_app_load_kit.sh pass-unblock […]
  invent_pass=0 Pass=No; privilege not required on stock host for stock live.

Path A — CONFIG_CU + opaque research PDI (CVML/AIE2P dump ≠ product matmul known-good):
  bash tools/operator_app_load_kit.sh config-cu-run
       → stock host: prep + live EXEC + collect (agent-runnable; no sudo L3)
  bash tools/operator_app_load_kit.sh config-cu
       → prep + sandwich print only (no live EXEC)
  research_pdi_not_product_known_good=1 invent_pass=0 not_ran_on_npu=1
  Wrong overlay may still IO_PAGE_FAULT / APP_LOAD — still progress if config_cu= measured.
  Spent flexml (do not re-live alone): 0eeeb70c / ba0cd738 / 7d172bf2 (+ CVML a1e65d04).

Path A-skip — no CONFIG_CU (product Arm A / live-shot-effective; live needs NEW unlock):
  bash tools/operator_app_load_kit.sh config-cu-skip
       → unset XDNA2_LAB_PDI_BLOB + XDNA2_LAB_SKIP_CONFIG_CU=1; preflight; sandwich print
  bash tools/operator_app_load_kit.sh config-cu-run-skip
       → stock live without CONFIG_CU (active C_UNLOCK only)
  Env: unset XDNA2_LAB_PDI_BLOB; export XDNA2_LAB_SKIP_CONFIG_CU=1; unset XDNA2_ALLOW_EXEC_CMD
  Product Arm A defaults XDNA2_LAB_SAME_SHOT_RELA=1 (kit + hitl_c + lab_exec forward).
  same_shot_gate alone ≠ Pass ≠ unlock; stale disk UVAs forbidden for product live.
  Outer budget ≥120s on sandwich print. Expected classes: config_cu absent;
  wait_cmd_einval / wait_timeout / APP_LOAD possible.
  skip-CU prep ≠ Pass ≠ ran_on_npu. Full checklist: docs/CONSTRUCTION_MAP_APP_LOAD.md
  § Skip-CONFIG_CU offline prep; docs/HITL_C_APP_LOAD_RECIPE.md Step 2 skip + C6.
  Plan: .agents/plans/plan-live-shot-effective.md

B1 hypothesis (not proven RC): lab pack key
  mladfmatmulbias_prmptflat_mladf_2x4x4_flat_a16fw4acc16f_1_1152_256_128
  under flexml overlays that attached then failed post-attach — possible topology
  mismatch. invent_pass=0. Report: .agents/reports/impl-next-load-path-offline.md

Path B — external working-stack ctrltext capture (human, when stack works):
  Prefer: bash tools/pass_unblock_once.sh --ingest /path/to/ctrltext.bin
  1) bash tools/operator_app_load_kit.sh b-steps
  2) bash tools/operator_app_load_kit.sh b-install /path/to/ctrltext.bin
  3) Fill PROVENANCE table under testdata/xdna2_cmd/external_known_good/
  4) bash tools/operator_app_load_kit.sh b-compare
  Public bins under external_known_good/ are research only (≠ product known-good).

After live sandwich:
  bash tools/operator_app_load_kit.sh post-evidence [/path/to/last_apply.json]

Step 2 (live under NEW unlock only — see docs/HITL_C_APP_LOAD_RECIPE.md):
  Prefer skip-CONFIG_CU isolation or new material; never re-live spent flexml alone.
  Optional debug BO: when XDNA2_LAB_RESEARCH_DEBUG_BO=1, lab timed path
  CREATE 4096 B Dev (AMDXDNA_BO_DEV) + ASSIGN_DBG_BUF (REMOVE on teardown);
  invent_pass=0. (Shmem-4096 failed live 48da6e60 / Get bo failed.)
  After any live shot: post-evidence → p5_post_c2_interpret / c3+c4+p6.

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
  config-cu|config_cu|a-shot|cu-shot)
    bash "$TOOLS/hitl_config_cu_once.sh" prep "$@"
    exit $?
    ;;
  config-cu-skip|config_cu_skip|skip-pdi|prep-skip-pdi)
    bash "$TOOLS/hitl_config_cu_once.sh" prep-skip-pdi "$@"
    exit $?
    ;;
  config-cu-run|config_cu_run|run-stock|run_stock)
    bash "$TOOLS/hitl_config_cu_once.sh" run-stock "$@"
    exit $?
    ;;
  config-cu-run-skip|config_cu_run_skip|run-stock-skip-pdi)
    bash "$TOOLS/hitl_config_cu_once.sh" run-stock-skip-pdi "$@"
    exit $?
    ;;
  post-evidence|post|evidence)
    bash "$TOOLS/collect_post_c2_evidence.sh" "$@"
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
  pass-unblock|pass_unblock|pass-unblock-once|pass_unblock_once)
    bash "$TOOLS/pass_unblock_once.sh" "$@"
    exit $?
    ;;
  *)
    echo "ERROR: unknown command: $cmd" >&2
    echo "allowlist: overview a-prep config-cu config-cu-skip config-cu-run config-cu-run-skip post-evidence b-status b-steps b-install b-compare offline-gates pass-unblock" >&2
    exit 2
    ;;
esac
