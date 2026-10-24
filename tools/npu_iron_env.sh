# Shared path + env setup for `just npu-iron` / `just npu-relu` / `--iron`.
# Sourced, not executed. Does not claim product Pass.
# Known kernels live in Rust (`iron_kernel.rs`): vector_add eltwise relu softmax
# gelu silu eltwise_mul swiglu scale_shift.

npu_iron_resolve_mlir() {
  local example="programming_examples/basic/vector_scalar_add/vector_scalar_add.py"
  local cand
  if [[ -n "${MLIR_AIE:-}" && -f "${MLIR_AIE}/${example}" ]]; then
    printf '%s\n' "$(readlink -f "$MLIR_AIE")"
    return 0
  fi
  for cand in \
    "${MLIR_AIE:-}" \
    "$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)/mlir-aie" \
    "${HOME}/Projects/ai/mlir-aie" \
    "${HOME}/mlir-aie" \
    "${HOME}/src/mlir-aie"
  do
    [[ -n "$cand" ]] || continue
    if [[ -f "${cand}/${example}" ]]; then
      printf '%s\n' "$(readlink -f "$cand")"
      return 0
    fi
  done
  echo "npu-iron: could not find mlir-aie with ${example}" >&2
  echo "Set MLIR_AIE to that tree, or clone mlir-aie next to this repo." >&2
  return 1
}

npu_iron_resolve_capture() {
  local cand
  for cand in \
    "${MLIR_AIE_CAPTURE:-}" \
    "${HOME}/Projects/ai/mlir-aie-capture" \
    "${HOME}/mlir-aie-capture"
  do
    [[ -n "$cand" ]] || continue
    if [[ -d "$cand/venv" ]]; then
      printf '%s\n' "$(readlink -f "$cand")"
      return 0
    fi
  done
  return 1
}

npu_iron_first_existing_dir() {
  local d
  for d in "$@"; do
    if [[ -n "$d" && -d "$d" ]]; then
      printf '%s\n' "$d"
      return 0
    fi
  done
  return 1
}

# Suffixes live only in Rust (`src/backends/iron_kernel.rs` / `--iron NAME`).
# This script resolves mlir-aie + Python so `just npu-relu` can set PYTHONPATH.

npu_iron_export_env() {
  local kernel="${1:-vector_add}"
  local mlir cap venv pyver site mlir_py peano tools_bin aie_bin usr_site
  mlir="$(npu_iron_resolve_mlir)"
  export MLIR_AIE="$mlir"
  export NPU_IRON_KERNEL="$kernel"
  unset NPU_IRON_EXAMPLE

  cap=""
  if cap="$(npu_iron_resolve_capture)"; then
    venv="${cap}/venv"
    pyver=""
    for d in "${venv}"/lib/python3.*; do
      if [[ -d "$d" ]]; then
        pyver="$(basename "$d")"
        break
      fi
    done
    site="${venv}/lib/${pyver:-python3.14}/site-packages"
    mlir_py="${site}/mlir_aie/python"
    peano="${site}/llvm-aie"
    tools_bin="${cap}/tools"
    aie_bin="${site}/mlir_aie/bin"
    [[ -d "$mlir_py" ]] || mlir_py=""
    [[ -d "$peano" ]] || peano=""
    [[ -d "$tools_bin" ]] || tools_bin=""
    [[ -d "$aie_bin" ]] || aie_bin=""
    [[ -d "$site" ]] || site=""
    export NPU_IRON_CAPTURE="$cap"
    if [[ -n "$peano" ]]; then
      export PEANO_INSTALL_DIR="$peano"
    fi
    export PATH="${tools_bin:+${tools_bin}:}${aie_bin:+${aie_bin}:}${venv}/bin:${PATH}"
    if [[ -n "${PYTHON:-}" && -x "${PYTHON}" ]]; then
      export NPU_IRON_PYTHON="$PYTHON"
    elif [[ -x "${venv}/bin/python3" ]]; then
      export NPU_IRON_PYTHON="${venv}/bin/python3"
    fi
  else
    mlir_py=""
    site=""
    export NPU_IRON_CAPTURE=""
    echo "npu-iron: mlir-aie-capture venv not found; trying system Python only" >&2
  fi

  if [[ -z "${NPU_IRON_PYTHON:-}" ]]; then
    if [[ -n "${PYTHON:-}" && -x "${PYTHON}" ]]; then
      export NPU_IRON_PYTHON="$PYTHON"
    else
      export NPU_IRON_PYTHON="/usr/bin/python3"
    fi
  fi

  usr_site="$(npu_iron_first_existing_dir \
    /usr/lib/python3.14/site-packages \
    /usr/lib/python3/site-packages || true)"

  # usr first so system pyxrt wins; venv site-packages supplies ml_dtypes/numpy;
  # mlir_aie/python is required for `import aie`.
  local pp=""
  for add in "$usr_site" "$site" "$mlir_py"; do
    [[ -n "$add" ]] || continue
    if [[ -n "$pp" ]]; then
      pp="${pp}:${add}"
    else
      pp="$add"
    fi
  done
  if [[ -n "$pp" ]]; then
    if [[ -n "${PYTHONPATH:-}" ]]; then
      export PYTHONPATH="${pp}:${PYTHONPATH}"
    else
      export PYTHONPATH="$pp"
    fi
  fi
}

npu_iron_ensure_bin() {
  local root
  root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
  if [[ -n "${DRAGON_NPU_BIN:-}" && -x "${DRAGON_NPU_BIN}" ]]; then
    export NPU_IRON_BIN="$DRAGON_NPU_BIN"
    return 0
  fi
  if [[ -x "${root}/target/release/dragon-npu" ]]; then
    export NPU_IRON_BIN="${root}/target/release/dragon-npu"
    return 0
  fi
  if command -v dragon-npu >/dev/null 2>&1; then
    export NPU_IRON_BIN="$(command -v dragon-npu)"
    return 0
  fi
  echo "npu-iron: dragon-npu not on PATH; cargo install --path . --force" >&2
  (cd "$root" && cargo install --path . --force --locked)
  export NPU_IRON_BIN="$(command -v dragon-npu)"
}

npu_iron_print_paths() {
  echo "MLIR_AIE=${MLIR_AIE:-}"
  echo "NPU_IRON_KERNEL=${NPU_IRON_KERNEL:-}"
  echo "NPU_IRON_EXAMPLE=rust_catalog_via_--iron"
  echo "NPU_IRON_CAPTURE=${NPU_IRON_CAPTURE:-}"
  echo "NPU_IRON_BIN=${NPU_IRON_BIN:-}"
  echo "PEANO_INSTALL_DIR=${PEANO_INSTALL_DIR:-}"
  echo "NPU_IRON_PYTHON=${NPU_IRON_PYTHON:-}"
  echo "PYTHON=${PYTHON:-}"
  echo "PYTHONPATH=${PYTHONPATH:-}"
}
