#!/usr/bin/env bash
# Read-only inventory of instruction/txn capture sources on this host.
#
# Safety:
#   - Never opens /dev/accel/*
#   - Never insmod / rmmod / modprobe
#   - Never sets XDNA2_ALLOW_EXEC_CMD or enables EXEC_CMD
#   - Never runs dragon-npu with dangerous features
#   - Limited-depth find with per-search timeouts
#
# Exit: ALWAYS 0 (report only / smoke report).
#   - NOT a Pass/fail gate for silicon or CI product criteria
#   - Missing tools, empty finds, or noisy /tmp hits are OK
#   - Parse sections; do not treat missing tools as failure
#   - Real gates: cargo fmt / clippy / test (see RESIDUAL.md)

set -u

ROOT=$(cd "$(dirname "$0")/.." && pwd)
echo "=== dragon-npu txn source inventory (read-only SMOKE REPORT) ==="
echo "NOTE: exit always 0 — report only; not a Pass gate for ran_on_npu / silicon"
echo "root=$ROOT"
echo "date_utc=$(date -u +%Y-%m-%dT%H:%M:%SZ 2>/dev/null || date)"
echo

section() { echo; echo "--- $1 ---"; }

have() { command -v "$1" >/dev/null 2>&1; }

# Per-find timeout (seconds). Without timeout(1), deep finds are skipped.
FIND_TIMEOUT_S="${INVENTORY_FIND_TIMEOUT_S:-5}"

section "XRT / plugin tools"
for t in xrt-smi xclbinutil xrt-runner xrt-replay xrt-tracer xbutil; do
  if have "$t"; then
    p=$(command -v "$t")
    echo "tool $t: present ($p)"
  else
    echo "tool $t: absent"
  fi
done

if [[ -f /usr/include/xrt/detail/version.h ]]; then
  echo "xrt_version_h: present"
  grep -E 'xrt_build_version|XRT_DRIVER_VERSION|xrt_build_version_hash' \
    /usr/include/xrt/detail/version.h 2>/dev/null | head -10 || true
else
  echo "xrt_version_h: missing (/usr/include/xrt/detail/version.h)"
fi

if ls /usr/lib/libxrt_core*.so* >/dev/null 2>&1; then
  echo "libxrt_core: present"
  ls -1 /usr/lib/libxrt_core*.so* 2>/dev/null | head -5
else
  echo "libxrt_core: not found under /usr/lib"
fi

if ls /usr/lib/libxrt_driver_xdna.so* >/dev/null 2>&1; then
  echo "xrt_plugin_amdxdna: present"
  ls -1 /usr/lib/libxrt_driver_xdna.so* 2>/dev/null | head -5
else
  echo "xrt_plugin_amdxdna: not found under /usr/lib"
fi

if [[ -d /opt/xilinx ]]; then
  echo "opt_xilinx: present"
else
  echo "opt_xilinx: missing (distro layout under /usr is OK)"
fi

section "amdxdna module / exec_cmd_allow (sysfs read only)"
if [[ -d /sys/module/amdxdna ]]; then
  echo "amdxdna: loaded"
  if [[ -r /sys/module/amdxdna/parameters/exec_cmd_allow ]]; then
    echo "exec_cmd_allow=$(cat /sys/module/amdxdna/parameters/exec_cmd_allow 2>/dev/null || echo '?')"
  else
    echo "exec_cmd_allow: parameter not readable"
  fi
  if have modinfo; then
    modinfo amdxdna 2>/dev/null | grep -E '^(filename|version|srcversion|parm):' | head -20 || true
  fi
else
  echo "amdxdna: not loaded"
fi

section "device nodes (existence only; no open)"
for p in /dev/accel/accel0 /sys/class/accel/accel0; do
  if [[ -e "$p" ]]; then
    echo "path $p: exists"
  else
    echo "path $p: missing"
  fi
done
if [[ -r /sys/class/accel/accel0/device/modalias ]]; then
  echo "modalias=$(cat /sys/class/accel/accel0/device/modalias 2>/dev/null || true)"
fi

section "search common paths (depth-limited, timeout)"
# Prefer timeout(1). Without it, skip deep finds (avoid unbounded wall time).
run_find() {
  local label=$1
  shift
  echo "search: $label"
  if have timeout; then
    timeout "${FIND_TIMEOUT_S}s" find "$@" 2>/dev/null | head -40 || true
  else
    echo "  (skip deep find: timeout(1) missing; set PATH or install coreutils)"
  fi
}

SEARCH_ROOTS=(
  "$HOME/Projects/kernel/xdna-driver"
  "$HOME/Projects/kernel/xdna-driver-hardened"
  "$HOME/Projects/ai/RyzenAI-SW"
  "$ROOT/testdata"
  "$ROOT/kernel/dkms/staging"
)

for root in "${SEARCH_ROOTS[@]}"; do
  if [[ -d "$root" ]]; then
    run_find "ml_txn.bin under $root" "$root" -maxdepth 6 -type f -name 'ml_txn.bin'
    run_find "*.xclbin under $root" "$root" -maxdepth 5 -type f -name '*.xclbin'
    run_find "nop/vadd/gemm elf under $root" "$root" -maxdepth 7 -type f \( -name 'nop.elf' -o -name 'vadd.elf' -o -name 'gemm.elf' \)
  else
    echo "search root missing: $root"
  fi
done

# /tmp is best-effort noise OK — opt-in to avoid unrelated world-readable paths.
if [[ "${INVENTORY_SEARCH_TMP:-0}" == "1" ]]; then
  section "/tmp search (INVENTORY_SEARCH_TMP=1; best-effort noise OK)"
  tmp_root="${TMPDIR:-/tmp}"
  if [[ -d "$tmp_root" ]]; then
    run_find "ml_txn / lab dumps under $tmp_root" "$tmp_root" -maxdepth 3 -type f \( -name 'ml_txn.bin' -o -name 'lab_instr.bin' -o -name '*known_good*' -o -name 'nop.elf' \)
  fi
else
  echo "tmp search: skipped (set INVENTORY_SEARCH_TMP=1 to enable; best-effort noise OK)"
fi

if [[ -d "$HOME/Projects/kernel/xdna-driver/test" ]]; then
  if [[ -d "$HOME/Projects/kernel/xdna-driver/test/local_shim_test_data" ]]; then
    echo "local_shim_test_data: PRESENT"
    run_find "local_shim_test_data files" \
      "$HOME/Projects/kernel/xdna-driver/test/local_shim_test_data" -maxdepth 4 -type f
  else
    echo "local_shim_test_data: MISSING (primary known-good ELF path unavailable)"
  fi
fi

section "dragon-npu env hooks (names only; values not required)"
cat <<'EOF'
XDNA2_INSTRUCTION_BLOB   # path to raw instruction/txn bytes (gitignored lab file OK)
XDNA2_INSTRUCTION_HEX    # hex string (optional 0x); ignored if BLOB set
XDNA2_ALLOW_EXEC_CMD     # must stay unset for default tests (L1 runtime)
XDNA2_WAIT_TIMEOUT_MS    # finite WAIT default path
XDNA2_HOST_REF           # optional host matmul golden attach
XDNA2_REAL_SUBMIT_MODE   # host_ref | host_validate | npu_exec (default fail-closed)
# Cargo feature xdna2-dangerous-exec-cmd is OFF by default (not pulled by amd-xdna2)
# Load APIs: load_instruction_blob_from_env / _from_path / decode_instruction_hex
# Verify policy: decide_ran_on_npu / verify_f32_allclose; timed wire via XDNA2_LAB_OFM=1 only
# Docs: docs/XDNA2_TXN_RESEARCH.md  scripts/capture_instruction_blob_checklist.md
# Staging: testdata/xdna2_cmd/ (binaries gitignored)
EOF

section "repo staging"
if [[ -d "$ROOT/testdata/xdna2_cmd" ]]; then
  echo "testdata/xdna2_cmd: present"
  ls -la "$ROOT/testdata/xdna2_cmd" 2>/dev/null || true
else
  echo "testdata/xdna2_cmd: missing"
fi

echo
echo "=== end inventory (exit 0 always — smoke report, not a Pass gate) ==="
exit 0
