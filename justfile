# Dragon-npu operator recipes.
# Named IRON: `just npu-iron` is the run+verify wrap (vector add).
# `just npu-iron-export` is export-only (writes artifacts; no child PASS! expected).
# Also: npu-eltwise, npu-relu, npu-softmax, npu-gelu, npu-silu, npu-mul,
# npu-swiglu, npu-scale-shift. That is not product Pass. Child PASS! is not ran_on_npu.

set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

# List recipes (default).
default:
    @just --list

# Install this checkout's dragon-npu onto ~/.cargo/bin.
install:
    cargo install --path . --force --locked

# Print the mlir-aie tree and IRON example dir this host would use.
npu-iron-paths kernel="vector_add":
    #!/usr/bin/env bash
    set -euo pipefail
    source "{{justfile_directory()}}/tools/npu_iron_env.sh"
    npu_iron_export_env "{{kernel}}"
    npu_iron_ensure_bin
    npu_iron_print_paths

# Shared IRON wrap. Kernel is a catalog name. Not product Pass.
_npu-iron-named kernel:
    #!/usr/bin/env bash
    set -euo pipefail
    root="{{justfile_directory()}}"
    cd "$root"
    source "$root/tools/npu_iron_env.sh"
    npu_iron_export_env "{{kernel}}"
    npu_iron_ensure_bin
    npu_iron_print_paths
    echo "Honesty: IRON child PASS is not product ran_on_npu. invent_pass=0."
    echo "NPU_IRON_PYTHON=$NPU_IRON_PYTHON"
    echo "NPU_IRON_KERNEL={{kernel}}"
    exec "$NPU_IRON_BIN" run --iron "{{kernel}}"

# IRON vector_scalar_add run+verify wrap (catalog name vector_add). Not product Pass.
npu-iron:
    just _npu-iron-named vector_add

# Export-only collect for vector_add. Writes this-run artifacts. No child PASS! expected.
npu-iron-export:
    #!/usr/bin/env bash
    set -euo pipefail
    root="{{justfile_directory()}}"
    cd "$root"
    source "$root/tools/npu_iron_env.sh"
    npu_iron_export_env "vector_add"
    npu_iron_ensure_bin
    npu_iron_print_paths
    echo "Honesty: export-only. No child PASS! expected. IRON child PASS is not product ran_on_npu. invent_pass=0."
    echo "NPU_IRON_PYTHON=$NPU_IRON_PYTHON"
    echo "NPU_IRON_KERNEL=vector_add"
    exec "$NPU_IRON_BIN" run --iron vector_add --iron-export

# IRON eltwise add. Not product Pass.
npu-eltwise:
    just _npu-iron-named eltwise

# IRON ReLU (eltwise_unary --op relu). Not product Pass.
npu-relu:
    just _npu-iron-named relu

# IRON softmax. Not product Pass.
npu-softmax:
    just _npu-iron-named softmax

# IRON GELU (eltwise_unary --op gelu). Not product Pass.
npu-gelu:
    just _npu-iron-named gelu

# IRON SiLU (eltwise_unary --op silu). Not product Pass.
npu-silu:
    just _npu-iron-named silu

# IRON eltwise mul. Not product Pass.
npu-mul:
    just _npu-iron-named eltwise_mul

# IRON SwiGLU. Not product Pass.
npu-swiglu:
    just _npu-iron-named swiglu

# IRON scale-and-shift. Not product Pass.
npu-scale-shift:
    just _npu-iron-named scale_shift
