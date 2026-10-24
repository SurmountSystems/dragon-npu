#!/usr/bin/env bash
# load_accepted_path_status.sh — read-only Path A/B/C availability (W5)
#
# Prints which load-accepted capture avenues look available on this host.
# Never: download, install, ALLOW=1, EXEC_CMD, unlock mint, invent bins, Pass.
#
# Usage:
#   bash tools/load_accepted_path_status.sh
#   bash tools/load_accepted_path_status.sh -h|--help
#
# Exit: always 0 (soft status). Footer always invent_pass=0.
#
# Paths (operator playbook):
#   B = IRON / mlir-aie completed-run avenue
#   C = XRT / xrt-smi
#   A = FastFlowLM / Lemonade (typically absent)
#
# See: docs/LOAD_ACCEPTED_CAPTURE.md
set -euo pipefail
export PATH="/usr/bin${HOME:+:${HOME}/.local/bin}:${PATH:-}"
unset XDNA2_ALLOW_EXEC_CMD || true

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
BAG_DIR="$ROOT/testdata/xdna2_cmd/external_known_good"

usage() {
  cat <<'EOF'
load_accepted_path_status - read-only Path A/B/C availability (W5)

Usage:
  bash tools/load_accepted_path_status.sh
  bash tools/load_accepted_path_status.sh -h|--help

Honesty
  invent_pass=0 not_ran_on_npu=1 not_c_ready=1 Pass=No
  Availability ≠ load-accepted. AOT peels ≠ completed-on-FW.
  xrt-smi present ≠ Pass. FastFlowLM absent is expected until operator installs.

Docs
  docs/LOAD_ACCEPTED_CAPTURE.md
  docs/CAPTURE_MATERIAL_MLIR_AIE.md
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi
if [[ $# -gt 0 ]]; then
  echo "ERROR: unknown arg: $1 (allowlist: --help)" >&2
  echo "invent_pass=0"
  exit 0
fi

echo "================================================================"
echo "load_accepted_path_status - read-only (W5 Paths A/B/C)"
echo "================================================================"
echo "ROOT=$ROOT"
echo "date=$(date -Iseconds 2>/dev/null || date)"
echo "Honesty: invent_pass=0 not_ran_on_npu=1 not_c_ready=1 Pass=No"
echo "availability_not_load_accepted=1 aot_not_load_accepted=1 validate_not_pass=1"
echo ""

# --- Path B: mlir-aie / IRON ---
echo "=== Path B — IRON / mlir-aie (completed NPU run preferred) ==="
MLIR_CANDIDATES=(
  "${HOME}/Projects/ai/mlir-aie"
  "${HOME}/mlir-aie"
  "${HOME}/src/mlir-aie"
)
CAPTURE_CANDIDATES=(
  "${HOME}/Projects/ai/mlir-aie-capture"
  "${HOME}/mlir-aie-capture"
)
PATH_B_MLIR=0
PATH_B_CAP=0
PATH_B_VENV=0
PATH_B_AIEBU=0
PATH_B_EXAMPLE=0
PATH_B_AOT_DIR=0

for d in "${MLIR_CANDIDATES[@]}"; do
  if [[ -d "$d" ]]; then
    PATH_B_MLIR=1
    echo "  mlir-aie_tree=present path=$d"
    if [[ -f "$d/programming_examples/basic/vector_scalar_add/vector_scalar_add.py" ]]; then
      PATH_B_EXAMPLE=1
      echo "  smallest_example=present vector_scalar_add.py"
    else
      echo "  smallest_example=absent (no vector_scalar_add.py under tree)"
    fi
    break
  fi
done
if [[ "$PATH_B_MLIR" -eq 0 ]]; then
  echo "  mlir-aie_tree=absent"
fi

for d in "${CAPTURE_CANDIDATES[@]}"; do
  if [[ -d "$d" ]]; then
    PATH_B_CAP=1
    echo "  mlir-aie-capture=present path=$d"
    if [[ -d "$d/venv" ]]; then
      PATH_B_VENV=1
      echo "  capture_venv=present path=$d/venv"
    else
      echo "  capture_venv=absent"
    fi
    if [[ -x "$d/tools/aiebu-asm" || -L "$d/tools/aiebu-asm" ]]; then
      PATH_B_AIEBU=1
      echo "  aiebu-asm=present path=$d/tools/aiebu-asm"
    else
      echo "  aiebu-asm=absent_or_not_executable"
    fi
    # Prior AOT research dirs (not load-accepted by themselves)
    aot_n=0
    for aot in "$d"/aot_*; do
      if [[ -d "$aot" ]]; then
        aot_n=$((aot_n + 1))
      fi
    done
    if [[ "$aot_n" -gt 0 ]]; then
      PATH_B_AOT_DIR=1
      echo "  prior_aot_dirs=$aot_n (RESEARCH ONLY unless completed_on_this_fw proven)"
    else
      echo "  prior_aot_dirs=0"
    fi
    break
  fi
done
if [[ "$PATH_B_CAP" -eq 0 ]]; then
  echo "  mlir-aie-capture=absent"
fi

if [[ "$PATH_B_MLIR" -eq 1 && "$PATH_B_EXAMPLE" -eq 1 ]]; then
  echo "  path_b_status=trees_ready_for_operator_npu_run"
  echo "  path_b_note=AOT_research_peels_alone_not_load_accepted"
else
  echo "  path_b_status=blocked_or_partial (need mlir-aie tree + example)"
fi
echo "  next=see docs/LOAD_ACCEPTED_CAPTURE.md Path B; run vector_scalar_add to completion on NPU"
echo ""

# --- Path C: XRT ---
echo "=== Path C — XRT / xrt-smi (health ≠ Pass) ==="
PATH_C_XRT_SMI=0
PATH_C_XCLBINUTIL=0
if command -v xrt-smi >/dev/null 2>&1; then
  PATH_C_XRT_SMI=1
  xs="$(command -v xrt-smi)"
  echo "  xrt-smi=present path=$xs"
  # Version only (soft); never claim Pass from this.
  set +e
  ver="$(xrt-smi --version 2>/dev/null | head -n 3 | tr '\n' ' ')"
  set -e
  if [[ -n "${ver//[[:space:]]/}" ]]; then
    echo "  xrt-smi_version_hint=$(printf '%s' "$ver" | head -c 160)"
  fi
else
  echo "  xrt-smi=absent"
fi
if command -v xclbinutil >/dev/null 2>&1; then
  PATH_C_XCLBINUTIL=1
  echo "  xclbinutil=present path=$(command -v xclbinutil)"
else
  echo "  xclbinutil=absent"
fi
if [[ "$PATH_C_XRT_SMI" -eq 1 ]]; then
  echo "  path_c_status=xrt-smi_present"
  echo "  path_c_note=health_validate_not_product_pass=1 peel_only_if_job_truly_completes=1"
  echo "  next=operator may run xrt-smi validate; then bash tools/journal_npu_scrape.sh"
else
  echo "  path_c_status=xrt-smi_absent"
fi
echo ""

# --- Path A: FastFlowLM / Lemonade ---
echo "=== Path A — FastFlowLM / Lemonade (operator install only) ==="
PATH_A_FLM=0
PATH_A_LEM=0
FLM_CANDIDATES=(
  "${HOME}/Projects/ai/FastFlowLM"
  "${HOME}/FastFlowLM"
  "${HOME}/Projects/ai/fastflowlm"
)
LEM_CANDIDATES=(
  "${HOME}/Projects/ai/lemonade"
  "${HOME}/Projects/ai/lemonade-sdk"
  "${HOME}/lemonade"
)
for d in "${FLM_CANDIDATES[@]}"; do
  if [[ -d "$d" ]]; then
    PATH_A_FLM=1
    echo "  FastFlowLM=present path=$d"
    break
  fi
done
if [[ "$PATH_A_FLM" -eq 0 ]]; then
  echo "  FastFlowLM=absent"
fi
for d in "${LEM_CANDIDATES[@]}"; do
  if [[ -d "$d" ]]; then
    PATH_A_LEM=1
    echo "  Lemonade=present path=$d"
    break
  fi
done
if [[ "$PATH_A_LEM" -eq 0 ]]; then
  echo "  Lemonade=absent"
fi
if [[ "$PATH_A_FLM" -eq 0 && "$PATH_A_LEM" -eq 0 ]]; then
  echo "  path_a_status=absent"
  echo "  path_a_note=agent_will_not_download operator_install_only"
  echo "  links=https://github.com/ROCm/FastFlowLM https://github.com/lemonade-sdk/lemonade"
else
  echo "  path_a_status=partial_or_present"
  echo "  path_a_note=scan /tmp ~/.cache clone dirs for BO peels only after completing run"
fi
echo ""

# --- Capture dir honesty ---
echo "=== Capture directory (research peels ≠ load-accepted) ==="
if [[ -d "$BAG_DIR" ]]; then
  # Count non-md files (bins) softly
  bin_n=0
  while IFS= read -r -d '' f; do
    bin_n=$((bin_n + 1))
  done < <(find "$BAG_DIR" -maxdepth 1 -type f ! -name '*.md' -print0 2>/dev/null || true)
  echo "  bag_dir=$BAG_DIR"
  echo "  non_md_files=$bin_n"
  echo "  note=public_aot_300_660_2592_4512_research_only_do_not_retag_as_load_accepted"
else
  echo "  bag_dir=missing path=$BAG_DIR"
fi
echo ""

# --- Who acts next ---
echo "=== Who acts next (plain English) ==="
if [[ "$PATH_B_MLIR" -eq 1 && "$PATH_B_EXAMPLE" -eq 1 ]]; then
  echo "  primary=operator: run smallest npu2 example to completion (Path B), then ingest + PROVENANCE completed_on_this_fw=yes"
elif [[ "$PATH_C_XRT_SMI" -eq 1 ]]; then
  echo "  primary=operator: Path C xrt-smi validate / completing sample if any, journal scrape, peel only if complete"
else
  echo "  primary=operator: install or enable a stack that finishes on this FW (Path A install or mlir-aie Path B setup)"
fi
echo "  agent=docs/status only; no invent bins; no live apply; no re-tag public AOT as load-accepted"
echo ""

echo "=== Summary tokens ==="
echo "path_b_mlir=$PATH_B_MLIR path_b_example=$PATH_B_EXAMPLE path_b_venv=$PATH_B_VENV path_b_aiebu=$PATH_B_AIEBU path_b_aot_dirs=$PATH_B_AOT_DIR"
echo "path_c_xrt_smi=$PATH_C_XRT_SMI path_c_xclbinutil=$PATH_C_XCLBINUTIL"
echo "path_a_fastflowlm=$PATH_A_FLM path_a_lemonade=$PATH_A_LEM"
echo "invent_pass=0 not_c_ready=1 not_ran_on_npu=1 Pass=No"
echo "aot_not_load_accepted=1 validate_not_pass=1 availability_not_load_accepted=1"
echo "doc=docs/LOAD_ACCEPTED_CAPTURE.md"
exit 0
