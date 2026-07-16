#!/usr/bin/env bash
# Build + run dump_dd_txn against local Ryzen AI DynamicDispatch (lab only).
# Dumps prebuilt matmul txn binaries AMD already ships in libtransaction —
# we are not inventing NPU microcode; we extract what the vendor stack uses.
#
# Also dumps sibling map keys when present:
#   <key>_ctrl_meta  → written as .json if content is JSON text
#   <key>_ctrl       → binary control packet (.bin)
#
# Usage:
#   bash tools/dump_dd_txn.sh [PREFIX_OR_EXACT_KEY] [OUT_DIR] [MAX_PRIMARY]
#
# WP0 staged key example (txn + ctrl_meta sizes IFM/WTS/OFM):
#   bash tools/dump_dd_txn.sh \
#     'mladfmatmulbias_prmptflat_mladf_2x4x4_flat_a16fw4acc16f_1_1152_256_128' \
#     /tmp/wp0_dump 5
#   # then: jq '..|objects|select(has("size_in_bytes"))|.name,.size_in_bytes' \
#   #   /tmp/wp0_dump/*_ctrl_meta.json
#
# AMD binaries are gitignored (testdata/xdna2_cmd/dd_matmul_txn/). Do not commit.
set -euo pipefail
export PATH="/usr/bin:${HOME}/.local/bin:${PATH}"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DD_ROOT="${DD_ROOT:-$HOME/Projects/ai/ryzen-ai-install/venv/lib/python3.12/site-packages/ryzenai_dynamic_dispatch}"
INC="${DD_ROOT}/include/ryzenai/dynamic_dispatch"
LIB="${DD_ROOT}/lib"
SO="${DD_ROOT}/_DynamicDispatch.cpython-312-x86_64-linux-gnu.so"
OUT_BIN="${ROOT}/target/dump_dd_txn"
PREFIX="${1:-mladfmatmul}"
OUT_DIR="${2:-$HOME/Projects/ai/dragon-npu/testdata/xdna2_cmd/dd_matmul_txn}"
MAX="${3:-15}"

if [[ ! -f "$SO" ]]; then
  echo "ERROR: DynamicDispatch .so not found at $SO" >&2
  echo "  Install Ryzen AI / DD into venv first." >&2
  exit 1
fi
if [[ ! -f "$INC/txn/txn_container.hpp" ]]; then
  echo "ERROR: missing headers under $INC" >&2
  exit 1
fi

mkdir -p "$(dirname "$OUT_BIN")" "$OUT_DIR"
echo "compiling dump_dd_txn (libtransaction.a + zlib; ~300 MiB link) …"
# libtransaction.a holds zlib-compressed matmul programs; -lz required to inflate.
c++ -std=c++17 -O2 \
  -I"$INC" \
  -I"${DD_ROOT}/include" \
  "${ROOT}/tools/dump_dd_txn.cpp" \
  -o "$OUT_BIN" \
  -Wl,--start-group "${LIB}/libtransaction.a" -Wl,--end-group \
  -lz -lpthread -ldl

echo "running: $OUT_BIN '$PREFIX' '$OUT_DIR' $MAX"
"$OUT_BIN" "$PREFIX" "$OUT_DIR" "$MAX"
echo
echo "Next (still not a Pass until OFM matches):"
echo "  # pick a dumped .bin, classify with Rust:"
echo "  export XDNA2_INSTRUCTION_BLOB=<path-to.bin>"
echo "  export XDNA2_INSTRUCTION_REQUIRE_TXN=1"
echo "  # optional sizes from sibling meta:"
echo "  # export XDNA2_LAB_MLADF_META=<path-to.*_ctrl_meta.json>"
echo "  # live EXEC still needs temporary driver unlock (sudo) — see lab_exec_once.sh"
