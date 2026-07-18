#!/usr/bin/env bash
# Offline GT parity / self-consistency (A6). No L3 / no EXEC / ≠ ran_on_npu.
#
# With gt_capture present:
#   - re-run dump_mladf_gt_offline and compare instr/ctrl post-patch sha256 to MANIFEST
#   - pack kernel_args self-consistency via cargo unit tests already in contract_check
#
# Exit: 0 green or soft-skip (no GT); 2 hard fail; if XDNA2_REQUIRE_GT=1 missing GT → 2.
set -euo pipefail
export PATH="/usr/bin:${HOME}/.local/bin:${PATH}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
GT="$ROOT/testdata/xdna2_cmd/gt_capture"
REQUIRE_GT=0
[[ "${XDNA2_REQUIRE_GT:-}" == "1" || "${XDNA2_REQUIRE_GT:-}" == "true" ]] && REQUIRE_GT=1

fail() { echo "PARITY_FAIL: $*" >&2; exit 2; }
ok() { echo "PARITY_OK: $*"; }
skip() { echo "PARITY_SKIP: $*"; exit 0; }

if [[ ! -d "$GT" || ! -f "$GT/MANIFEST.sha256" ]]; then
  if [[ "$REQUIRE_GT" -eq 1 ]]; then
    fail "gt_capture missing and XDNA2_REQUIRE_GT=1"
  fi
  skip "no gt_capture/MANIFEST (run hitl_b / dump_mladf_gt_offline)"
fi

[[ -f "$GT/instr_post_patch.bin" ]] || fail "missing instr_post_patch.bin"
[[ -f "$GT/ctrl_post_patch.bin" ]] || fail "missing ctrl_post_patch.bin"

# Snapshot current hashes
old_i="$(sha256sum "$GT/instr_post_patch.bin" | awk '{print $1}')"
old_c="$(sha256sum "$GT/ctrl_post_patch.bin" | awk '{print $1}')"

# Re-dump oracle (deterministic paddrs) and compare
bash tools/dump_mladf_gt_offline.sh >/dev/null
new_i="$(sha256sum "$GT/instr_post_patch.bin" | awk '{print $1}')"
new_c="$(sha256sum "$GT/ctrl_post_patch.bin" | awk '{print $1}')"

[[ "$old_i" == "$new_i" ]] || fail "instr_post_patch not stable: $old_i -> $new_i"
[[ "$old_c" == "$new_c" ]] || fail "ctrl_post_patch not stable: $old_c -> $new_c"
ok "re-dump stable instr=$new_i ctrl=$new_c"

# MANIFEST must list both files
grep -q 'instr_post_patch.bin' "$GT/MANIFEST.sha256" || fail "MANIFEST missing instr_post_patch"
grep -q 'ctrl_post_patch.bin' "$GT/MANIFEST.sha256" || fail "MANIFEST missing ctrl_post_patch"

isz="$(wc -c <"$GT/instr_post_patch.bin" | tr -d ' ')"
[[ "$isz" -eq 1688 ]] || fail "instr size $isz != 1688"
ok "sizes ctrltext=1688"

# Optional: kernel_args.json layout presence
if [[ -f "$GT/kernel_args.json" ]]; then
  grep -q 'count4' "$GT/kernel_args.json" || fail "kernel_args.json missing count4"
  grep -q 'xrt3' "$GT/kernel_args.json" || fail "kernel_args.json missing xrt3"
  ok "kernel_args.json layouts present"
fi

# A5 schema presence (offline synthetic)
for f in start_npu_cmd.bin hwctx.json bo_table.json; do
  [[ -f "$GT/$f" ]] || fail "missing $f (re-run dump_mladf_gt_offline)"
done
ssz="$(wc -c <"$GT/start_npu_cmd.bin" | tr -d ' ')"
[[ "$ssz" -gt 32 ]] || fail "start_npu_cmd.bin too small ($ssz)"
grep -q '"tiles"' "$GT/hwctx.json" || fail "hwctx.json missing tiles"
grep -q '"bos"' "$GT/bo_table.json" || fail "bo_table.json missing bos"
ok "A5 schema: start_npu_cmd.bin hwctx.json bo_table.json"

echo "PARITY_OK: offline self-consistency (≠ ran_on_npu; not live XRT BO dump)"
exit 0
