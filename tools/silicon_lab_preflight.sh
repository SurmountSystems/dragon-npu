#!/usr/bin/env bash
# Offline silicon-path Pass-readiness preflight (no device, no L3, no EXEC).
#
# Checks staged mladf a16w4 assets under SILICON_LAB_DIR (default
# testdata/xdna2_cmd/silicon_lab) and/or env paths:
#   - instruction blob (lab_instr.elf preferred, or lab_instr.bin / XDNA2_INSTRUCTION_BLOB)
#   - WTS 174080 non-zero (validate_mladf_wts) — prefers first Pass-candidate
#   - CTRL 29664 non-zero (validate_mladf_ctrl)
#   - OFM golden 512/1024 non-zero when path set / --require-pass-assets
#
# Soft mode (default, no --require-pass-assets):
#   Always exit 0 after printing READINESS + DRY_RUN_READY=0|1.
#   DRY_RUN_READY=1 only when instr+ctrl OK (WTS/golden may still warn).
#   Soft exit 0 means "plan/report printed" — NOT Pass readiness / ≠ ran_on_npu.
#
# --require-pass-assets:
#   Exit 2 if instr / WTS nz / CTRL nz / golden nz incomplete.
#   Ignores XDNA2_LAB_ALLOW_ZERO_GOLDEN / ALLOW_ZERO_GOLDEN (cannot Pass-ready zeros).
#
# Does **not** open /dev/accel, modprobe, set ALLOW, or leave exec_cmd_allow=1.
#
# Usage:
#   bash tools/silicon_lab_preflight.sh
#   bash tools/silicon_lab_preflight.sh --require-pass-assets
#   bash tools/silicon_lab_preflight.sh --print-env
#   SILICON_LAB_DIR=/path bash tools/silicon_lab_preflight.sh
#
# Exit:
#   0  soft report complete (see DRY_RUN_READY) OR require-pass all green
#   2  bad args / Pass assets missing under --require-pass-assets
set -euo pipefail
export PATH="/usr/bin:${HOME}/.local/bin:${PATH}"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIR="${SILICON_LAB_DIR:-$ROOT/testdata/xdna2_cmd/silicon_lab}"
REQUIRE_PASS=0
PRINT_ENV=0
KEY='mladfmatmulbias_prmptflat_mladf_2x4x4_flat_a16fw4acc16f_1_1152_256_128'
EXPECT_INSTR="${EXPECT_INSTR_BYTES:-1688}"
EXPECT_WTS="${EXPECT_WTS_BYTES:-174080}"
EXPECT_CTRL="${EXPECT_CTRL_BYTES:-29664}"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --require-pass-assets) REQUIRE_PASS=1; shift ;;
    --print-env) PRINT_ENV=1; shift ;;
    --dir)
      [[ $# -ge 2 ]] || { echo "ERROR: --dir needs PATH" >&2; exit 2; }
      DIR="$2"
      shift 2
      ;;
    -h|--help)
      sed -n '1,40p' "$0" | sed 's/^# \?//'
      exit 0
      ;;
    *)
      echo "ERROR: unknown arg: $1 (try --help)" >&2
      exit 2
      ;;
  esac
done

fail() { echo "ERROR: $*" >&2; exit 2; }
warn() { echo "WARN: $*" >&2; }
ok() { echo "OK: $*"; }

# Unique temp dir per run (concurrent preflight safe).
TMPDIR_RUN="$(mktemp -d "${TMPDIR:-/tmp}/silicon_preflight.XXXXXX")"
cleanup_tmp() { rm -rf "$TMPDIR_RUN"; }
trap cleanup_tmp EXIT

bytes_all_zero() {
  local f="$1" n="$2" rc=0
  cmp -n "$n" -s -- "$f" /dev/zero || rc=$?
  if [[ $rc -eq 0 ]]; then
    return 0
  fi
  if [[ $rc -eq 1 ]]; then
    return 1
  fi
  fail "zero-check failed for $f (cmp exit $rc)"
}

resolve_path() {
  # Prefer env, then staged name under DIR.
  local env_val="${1:-}"
  local staged_name="$2"
  if [[ -n "$env_val" && -f "$env_val" ]]; then
    echo "$env_val"
    return 0
  fi
  if [[ -f "$DIR/$staged_name" ]]; then
    echo "$DIR/$staged_name"
    return 0
  fi
  echo ""
}

# First path that validates as Pass-candidate WTS (174080 non-zero).
pick_pass_candidate_wts() {
  local c out rc
  for c in "$@"; do
    [[ -n "$c" && -f "$c" ]] || continue
    out="$TMPDIR_RUN/wts_try.out"
    set +e
    bash "$ROOT/tools/validate_mladf_wts.sh" "$c" >"$out" 2>&1
    rc=$?
    set -e
    if [[ $rc -eq 0 ]]; then
      echo "$c"
      return 0
    fi
  done
  echo ""
  return 1
}

# Prefer aiebu ELF (rela-patchable) over raw DD txn for EXEC.
if [[ -n "${XDNA2_INSTRUCTION_BLOB:-}" && -f "${XDNA2_INSTRUCTION_BLOB}" ]]; then
  INSTR="$XDNA2_INSTRUCTION_BLOB"
  # Auto-upgrade raw .bin → sibling .elf when present (same as lab_exec_once).
  if [[ "$INSTR" == *.bin || "$INSTR" == *.BIN ]]; then
    elf_sib="${INSTR%.bin}.elf"
    elf_sib="${elf_sib%.BIN}.elf"
    if [[ -f "$elf_sib" ]]; then
      echo "NOTE: preflight upgrading raw txn → aiebu ELF: $elf_sib" >&2
      INSTR="$elf_sib"
    fi
  fi
elif [[ -f "$DIR/lab_instr.elf" ]]; then
  INSTR="$DIR/lab_instr.elf"
else
  INSTR="$(resolve_path "" lab_instr.bin)"
fi
# WTS precedence: explicit env → first Pass-candidate among staged / export.
EXPORT_WTS="$ROOT/testdata/xdna2_cmd/mladf_wts_export/wts.bin"
STAGED_WTS=""
[[ -f "$DIR/wts.bin" ]] && STAGED_WTS="$DIR/wts.bin"
if [[ -n "${XDNA2_LAB_WTS_PACKED:-}" && -f "${XDNA2_LAB_WTS_PACKED}" ]]; then
  WTS="$XDNA2_LAB_WTS_PACKED"
else
  WTS="$(pick_pass_candidate_wts "$STAGED_WTS" "$EXPORT_WTS" || true)"
  if [[ -z "$WTS" ]]; then
    # Fall back to first existing file for soft report (may be zero).
    if [[ -n "$STAGED_WTS" ]]; then
      WTS="$STAGED_WTS"
    elif [[ -f "$EXPORT_WTS" ]]; then
      WTS="$EXPORT_WTS"
    else
      WTS=""
    fi
  fi
fi
CTRL="$(resolve_path "${XDNA2_LAB_CTRL_PACKED:-}" lab_ctrl.bin)"
META="$(resolve_path "${XDNA2_LAB_MLADF_META:-}" lab_ctrl_meta.json)"
GOLDEN="$(resolve_path "${XDNA2_LAB_OFM_GOLDEN:-}" ofm_golden.bin)"
# pack placeholder is ofm_expected.bin (zeros) — never treat as Pass golden
EXPECTED_PLACEHOLDER="$DIR/ofm_expected.bin"

echo "silicon_lab_preflight: dir=$DIR require_pass_assets=$REQUIRE_PASS"
echo "  (offline only — no L3 / no EXEC / ≠ ran_on_npu)"
if [[ "$REQUIRE_PASS" -eq 1 ]]; then
  # Forbid plumbing zero-golden ALLOW from greening Pass readiness.
  unset XDNA2_LAB_ALLOW_ZERO_GOLDEN ALLOW_ZERO_GOLDEN || true
  echo "  (require-pass: ALLOW_ZERO_GOLDEN ignored)"
fi

INSTR_OK=0
WTS_OK=0
CTRL_OK=0
GOLDEN_OK=0
META_OK=0

# --- instruction ---
if [[ -z "$INSTR" || ! -f "$INSTR" ]]; then
  if [[ "$REQUIRE_PASS" -eq 1 ]]; then
    fail "instruction blob missing (stage lab_instr.elf via tools/stage_aiebu_lab_instr.sh)"
  fi
  warn "instruction blob missing"
else
  isz="$(wc -c <"$INSTR" | tr -d ' ')"
  if [[ "$isz" -eq 0 ]]; then
    if [[ "$REQUIRE_PASS" -eq 1 ]]; then
      fail "instruction blob empty: $INSTR"
    fi
    warn "instruction blob empty: $INSTR"
  else
    is_elf=0
    if [[ "$isz" -ge 4 ]]; then
      magic="$(od -An -tx1 -N4 "$INSTR" | tr -d ' \n')"
      # 7f 45 4c 46 = ELF
      if [[ "$magic" == "7f454c46" ]]; then
        is_elf=1
      fi
    fi
    if [[ "$is_elf" -eq 1 ]]; then
      ok "instr path=$INSTR size=$isz (aiebu ELF — preferred for EXEC/rela patch)"
      INSTR_OK=1
    elif [[ "$REQUIRE_PASS" -eq 1 ]]; then
      # Pass gate: raw classic txn is known-bad EXEC (APP_LOAD_PDI_FAIL).
      sib=""
      if [[ "$INSTR" == *.bin ]]; then
        sib="${INSTR%.bin}.elf"
      fi
      if [[ -n "$sib" && -f "$sib" ]]; then
        fail "instruction is raw non-ELF ($INSTR size=$isz) but sibling ELF exists ($sib) — use ELF for Pass"
      fi
      fail "instruction is not aiebu ELF magic (path=$INSTR size=$isz) — stage lab_instr.elf for --require-pass-assets"
    elif [[ -n "$EXPECT_INSTR" && "$isz" != "$EXPECT_INSTR" ]]; then
      warn "instruction size $isz != staged expect $EXPECT_INSTR (may still be valid txn)"
      ok "instr path=$INSTR size=$isz"
      INSTR_OK=1
    else
      if [[ "$INSTR" == *.bin ]]; then
        warn "raw lab_instr.bin (no ELF) — EXEC often APP_LOAD_PDI_FAIL; stage lab_instr.elf"
      fi
      ok "instr path=$INSTR size=$isz"
      INSTR_OK=1
    fi
  fi
fi

# --- WTS ---
if [[ -z "$WTS" || ! -f "$WTS" ]]; then
  if [[ "$REQUIRE_PASS" -eq 1 ]]; then
    fail "WTS missing (export_mladf_wts / --wts-from / XDNA2_LAB_WTS_PACKED)"
  fi
  warn "WTS missing"
else
  set +e
  bash "$ROOT/tools/validate_mladf_wts.sh" "$WTS" >"$TMPDIR_RUN/wts.out" 2>&1
  wts_rc=$?
  set -e
  if [[ $wts_rc -eq 0 ]]; then
    ok "WTS $(grep -E 'path:|size:|nonzero:' "$TMPDIR_RUN/wts.out" | tr '\n' ' ')"
    WTS_OK=1
  else
    cat "$TMPDIR_RUN/wts.out" >&2 || true
    if [[ "$REQUIRE_PASS" -eq 1 ]]; then
      fail "WTS failed validate_mladf_wts (need 174080 non-zero)"
    fi
    warn "WTS not Pass-candidate (zero or wrong size) — packing-only OK without --require-pass-assets"
  fi
fi

# --- CTRL ---
if [[ -z "$CTRL" || ! -f "$CTRL" ]]; then
  if [[ "$REQUIRE_PASS" -eq 1 ]]; then
    fail "CTRL missing (stage lab_ctrl.bin / XDNA2_LAB_CTRL_PACKED)"
  fi
  warn "CTRL missing"
else
  set +e
  bash "$ROOT/tools/validate_mladf_ctrl.sh" "$CTRL" >"$TMPDIR_RUN/ctrl.out" 2>&1
  ctrl_rc=$?
  set -e
  if [[ $ctrl_rc -eq 0 ]]; then
    ok "CTRL $(grep -E 'path:|size:|nonzero:' "$TMPDIR_RUN/ctrl.out" | tr '\n' ' ')"
    CTRL_OK=1
  else
    cat "$TMPDIR_RUN/ctrl.out" >&2 || true
    if [[ "$REQUIRE_PASS" -eq 1 ]]; then
      fail "CTRL failed validate_mladf_ctrl"
    fi
    warn "CTRL not Pass-candidate"
  fi
fi

# --- META (soft) ---
if [[ -n "$META" && -f "$META" ]]; then
  ok "meta path=$META"
  META_OK=1
else
  warn "lab_ctrl_meta.json missing (AUTO_PATCH needs XDNA2_LAB_MLADF_META)"
fi

# --- OFM golden ---
if [[ -n "$GOLDEN" && -f "$GOLDEN" ]]; then
  # Refuse mistaking placeholder path when same as ofm_expected and zeros
  if [[ -f "$EXPECTED_PLACEHOLDER" ]]; then
    g_abs="$(readlink -f "$GOLDEN" 2>/dev/null || echo "$GOLDEN")"
    p_abs="$(readlink -f "$EXPECTED_PLACEHOLDER" 2>/dev/null || echo "$EXPECTED_PLACEHOLDER")"
    if [[ "$g_abs" == "$p_abs" ]]; then
      if bytes_all_zero "$GOLDEN" "$(wc -c <"$GOLDEN" | tr -d ' ')"; then
        if [[ "$REQUIRE_PASS" -eq 1 ]]; then
          fail "ofm_expected.bin is zero placeholder — not Pass golden (capture non-zero bf16 512 B)"
        fi
        warn "ofm_expected.bin is zero placeholder — not Pass golden"
        GOLDEN=""
      fi
    fi
  fi
fi

if [[ -n "${GOLDEN:-}" && -f "$GOLDEN" ]]; then
  set +e
  # Under require-pass ALLOW already unset; soft may still see exit 3 from ALLOW.
  bash "$ROOT/tools/validate_mladf_ofm_golden.sh" "$GOLDEN" >"$TMPDIR_RUN/ofm.out" 2>&1
  g_rc=$?
  set -e
  if [[ $g_rc -eq 0 ]]; then
    ok "OFM golden $(grep -E 'path:|size:|nonzero:' "$TMPDIR_RUN/ofm.out" | tr '\n' ' ')"
    GOLDEN_OK=1
  elif [[ $g_rc -eq 3 ]]; then
    cat "$TMPDIR_RUN/ofm.out" >&2 || true
    if [[ "$REQUIRE_PASS" -eq 1 ]]; then
      fail "OFM golden is zero (ALLOW plumbing only) — refuse --require-pass-assets"
    fi
    warn "OFM golden all-zero under ALLOW — not Pass-candidate (validator exit 3)"
  else
    cat "$TMPDIR_RUN/ofm.out" >&2 || true
    if [[ "$REQUIRE_PASS" -eq 1 ]]; then
      fail "OFM golden failed validate_mladf_ofm_golden"
    fi
    warn "OFM golden not Pass-candidate"
  fi
else
  if [[ "$REQUIRE_PASS" -eq 1 ]]; then
    fail "OFM golden missing (set XDNA2_LAB_OFM_GOLDEN to non-zero bf16 512 B or f32 1024 B; pack ofm_expected.bin is zeros)"
  fi
  warn "OFM golden not set — Pass require: export XDNA2_LAB_OFM_GOLDEN=…/nonzero.bin"
fi

DRY_RUN_READY=0
if [[ "$INSTR_OK" -eq 1 && "$CTRL_OK" -eq 1 ]]; then
  DRY_RUN_READY=1
fi

echo
echo "READINESS: instr=$INSTR_OK wts=$WTS_OK ctrl=$CTRL_OK golden=$GOLDEN_OK meta=$META_OK"
echo "DRY_RUN_READY=$DRY_RUN_READY  # 1=instr+ctrl present (soft plan ok); 0=incomplete stage"
echo "  note: soft exit 0 always prints plan — use --require-pass-assets for Pass asset gate"
echo "  note: DRY_RUN_READY ≠ ran_on_npu; golden/WTS may still be missing"

if [[ "$REQUIRE_PASS" -eq 1 ]]; then
  if [[ "$INSTR_OK" -ne 1 || "$WTS_OK" -ne 1 || "$CTRL_OK" -ne 1 || "$GOLDEN_OK" -ne 1 ]]; then
    fail "Pass assets incomplete under --require-pass-assets"
  fi
  ok "Pass assets offline-ready (still need L1+L3 EXEC+OFM allclose for ran_on_npu)"
else
  if [[ "$INSTR_OK" -ne 1 || "$CTRL_OK" -ne 1 ]]; then
    warn "minimal stage incomplete (instr+ctrl); re-run tools/stage_silicon_lab_assets.sh"
  fi
  ok "soft preflight complete (exit 0 = report only; check DRY_RUN_READY / --require-pass-assets)"
fi

if [[ "$PRINT_ENV" -eq 1 ]]; then
  # Absolute paths for copy-paste
  abs() { readlink -f "$1" 2>/dev/null || realpath "$1" 2>/dev/null || echo "$1"; }
  cat <<EOF

# --- env recipe (export in your shell; dry-run first) ---
export XDNA2_INSTRUCTION_BLOB=$( [[ -n "$INSTR" ]] && abs "$INSTR" || echo "$DIR/lab_instr.elf" )
export XDNA2_LAB_OFM=1
export XDNA2_LAB_PACK=mladf_a16w4
export XDNA2_LAB_MLADF_KEY='$KEY'
export XDNA2_LAB_MLADF_META=$( [[ -n "$META" ]] && abs "$META" || echo "$DIR/lab_ctrl_meta.json" )
export XDNA2_LAB_WTS_PACKED=$( [[ -n "$WTS" ]] && abs "$WTS" || echo "$DIR/wts.bin" )
export XDNA2_LAB_CTRL_PACKED=$( [[ -n "$CTRL" ]] && abs "$CTRL" || echo "$DIR/lab_ctrl.bin" )
export XDNA2_LAB_REQUIRE_NONZERO_WTS=1
export XDNA2_LAB_REQUIRE_CTRL=1
export XDNA2_LAB_CTRL_AUTO_PATCH=1
$( if [[ -n "${GOLDEN:-}" && -f "${GOLDEN:-}" && "$GOLDEN_OK" -eq 1 ]]; then echo "export XDNA2_LAB_OFM_GOLDEN=$(abs "$GOLDEN")"; else echo "# export XDNA2_LAB_OFM_GOLDEN=/path/to/nonzero_bf16_ofm.bin  # required for Pass"; fi )

# Dry-run lab window (no L3 open):
bash scripts/lab_exec_once.sh --silicon-mladf --blob "\$XDNA2_INSTRUCTION_BLOB"
# Pass asset gate:
# bash tools/silicon_lab_preflight.sh --require-pass-assets --print-env

# Apply only after dry-run OK + operator risk accept (HITL):
# bash scripts/lab_exec_once.sh --silicon-mladf --apply --i-accept-wedge-risk --blob "\$XDNA2_INSTRUCTION_BLOB"
# Honesty: apply exit 4 if !ran_on_npu is common; restore traps still clear gates.
# Soft dry-run exit 0 / DRY_RUN_READY ≠ ran_on_npu. Never leave exec_cmd_allow=1.
EOF
fi

exit 0
