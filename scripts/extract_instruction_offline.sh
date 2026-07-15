#!/usr/bin/env bash
# extract_instruction_offline.sh — offline inventory / optional VE2 staging copy.
#
# Safety:
#   - Never opens /dev/accel/*
#   - Never insmod / rmmod / modprobe
#   - Never sets XDNA2_ALLOW_EXEC_CMD or enables EXEC_CMD
#   - Never runs lab_exec_once --apply
#   - Never claims AIE2/17f0 known-good for VE2 Runner fixtures
#   - Never auto-wires lab_instr.bin without explicit --force-lab-instr
#   - Never copies VE2 ELF to lab_instr.bin without --allow-ve2-as-lab-instr
#
# Exit codes:
#   0  Inventory / stage report OK (smoke; not silicon Pass)
#   2  Bad args (e.g. --force-lab-instr without --apply-copy/--stage-ve2-nop)
#   4  Refuse unsafe claim / missing source / VE2 as lab_instr without allow flag
#
# Usage:
#   bash scripts/extract_instruction_offline.sh
#   bash scripts/extract_instruction_offline.sh --stage-ve2-nop
#   bash scripts/extract_instruction_offline.sh --stage-ve2-nop --out testdata/xdna2_cmd/ve2_latency
#   bash scripts/extract_instruction_offline.sh --elf PATH --xclbin PATH --apply-copy --out DIR
#
# Prefer: PATH=/usr/bin:$PATH

set -euo pipefail

if [[ "${BASH_SOURCE[0]}" != "${0}" ]]; then
  echo "ERROR: do not source extract_instruction_offline.sh; run: bash …" >&2
  return 2 2>/dev/null || exit 2
fi

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MODE="inventory"
OUT=""
ELF=""
XCLBIN=""
RECIPE=""
STAGE_VE2=0
APPLY_COPY=0
FORCE_LAB_INSTR=0
ALLOW_VE2_AS_LAB_INSTR=0
CLAIM_AIE2=0
I_UNDERSTAND_MISMATCH=0

DEFAULT_VE2_DIR="$ROOT/kernel/dkms/staging/amdxdna-hardened-0.1.0/src/shim_ve2/Runner/latency"
DEFAULT_OUT_VE2="$ROOT/testdata/xdna2_cmd/ve2_latency"
DEFAULT_OUT_NPU4="$ROOT/testdata/xdna2_cmd/npu4_candidate"
DEFAULT_OUT_UNKNOWN="$ROOT/testdata/xdna2_cmd/staging"

die() {
  local code="$1"
  shift
  echo "ERROR: $*" >&2
  exit "$code"
}

usage() {
  cat <<'EOF'
extract_instruction_offline.sh — offline inventory + optional VE2 staging copy

Default: inventory/report only (exit 0). Never enables EXEC / ALLOW.

Options:
  --stage-ve2-nop     Stage VE2 Runner latency nop.elf + validate.xclbin + recipe
                      under out dir with PROVENANCE.md (NOT_AIE2_KNOWN_GOOD=1)
  --apply-copy        Copy --elf/--xclbin into --out (required with --force-lab-instr)
  --elf PATH          Source ELF
  --xclbin PATH       Source xclbin
  --out DIR           Output directory (default by platform:
                        VE2_EDGE → testdata/xdna2_cmd/ve2_latency
                        NPU4_CANDIDATE → testdata/xdna2_cmd/npu4_candidate
                        else → testdata/xdna2_cmd/staging)
  --force-lab-instr   Write lab_instr.bin from ELF (requires --apply-copy or
                      --stage-ve2-nop). For VE2 also requires --allow-ve2-as-lab-instr.
                      Still NOT_AIE2_KNOWN_GOOD; envelope study only.
  --allow-ve2-as-lab-instr
                      Explicit opt-in to write lab_instr.bin from a VE2_EDGE ELF.
                      PROVENANCE still warns; never product Pass.
  --claim-aie2-known-good
                      Always refused by this offline script (exit 4). Silicon OFM
                      verify is required before any AIE2 known-good claim. VE2 paths
                      are refused even with --i-understand-platform-mismatch.
  --i-understand-platform-mismatch
                      Documented for npu4 candidate paths; does not make claim succeed
                      (script still exits 4 — no offline known-good recording).
  -h, --help          This help

Honesty strings always printed: PLATFORM=… CLAIM=not_known_good_for_aie2_17f0
Never prints ran_on_npu=true.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h|--help)
      usage
      exit 0
      ;;
    --stage-ve2-nop)
      STAGE_VE2=1
      APPLY_COPY=1
      MODE="stage"
      shift
      ;;
    --apply-copy)
      APPLY_COPY=1
      MODE="stage"
      shift
      ;;
    --elf)
      [[ $# -ge 2 ]] || die 2 "--elf requires PATH"
      ELF="$2"
      shift 2
      ;;
    --xclbin)
      [[ $# -ge 2 ]] || die 2 "--xclbin requires PATH"
      XCLBIN="$2"
      shift 2
      ;;
    --out)
      [[ $# -ge 2 ]] || die 2 "--out requires DIR"
      OUT="$2"
      shift 2
      ;;
    --force-lab-instr)
      FORCE_LAB_INSTR=1
      shift
      ;;
    --allow-ve2-as-lab-instr)
      ALLOW_VE2_AS_LAB_INSTR=1
      shift
      ;;
    --claim-aie2-known-good)
      CLAIM_AIE2=1
      shift
      ;;
    --i-understand-platform-mismatch)
      I_UNDERSTAND_MISMATCH=1
      shift
      ;;
    *)
      die 2 "unknown argument: $1 (try --help)"
      ;;
  esac
done

have() { command -v "$1" >/dev/null 2>&1; }

section() { echo; echo "--- $1 ---"; }

# --force-lab-instr is a no-op without stage/apply-copy → refuse early (exit 2)
if [[ "$FORCE_LAB_INSTR" -eq 1 && "$APPLY_COPY" -eq 0 ]]; then
  die 2 "--force-lab-instr requires --apply-copy or --stage-ve2-nop (refusing silent no-op)"
fi

echo "=== dragon-npu extract_instruction_offline (offline only) ==="
echo "NOTE: exit 0 is smoke/report only — not silicon Pass / not ran_on_npu"
echo "root=$ROOT"
echo "date_utc=$(date -u +%Y-%m-%dT%H:%M:%SZ 2>/dev/null || date)"
echo "CLAIM=not_known_good_for_aie2_17f0"
echo "ran_on_npu=false  # script never claims Pass"

section "tools (optional inspect)"
for t in xclbinutil readelf sha256sum stat timeout; do
  if have "$t"; then
    echo "tool $t: present ($(command -v "$t"))"
  else
    echo "tool $t: absent"
  fi
done

section "default VE2 Runner latency paths (research only)"
VE2_ELF="$DEFAULT_VE2_DIR/nop.elf"
VE2_XCLBIN="$DEFAULT_VE2_DIR/validate.xclbin"
VE2_RECIPE="$DEFAULT_VE2_DIR/recipe_latency.json"
for p in "$VE2_ELF" "$VE2_XCLBIN" "$VE2_RECIPE"; do
  if [[ -e "$p" ]]; then
    if have stat; then
      echo "path $p: exists size=$(stat -c '%s' "$p" 2>/dev/null || echo '?')"
    else
      echo "path $p: exists"
    fi
  else
    echo "path $p: missing"
  fi
done

section "inventory_txn_sources pointer"
echo "For full host inventory (still exit 0 report-only):"
echo "  bash scripts/inventory_txn_sources.sh"
echo "Operator capture checklist:"
echo "  scripts/capture_instruction_blob_checklist.md"
echo "Honesty: VE2 Runner nop.elf is PLATFORM=VE2_EDGE — not Strix npu4/17f0 known-good."

# Resolve sources for stage
if [[ "$STAGE_VE2" -eq 1 ]]; then
  [[ -z "$ELF" ]] && ELF="$VE2_ELF"
  [[ -z "$XCLBIN" ]] && XCLBIN="$VE2_XCLBIN"
  [[ -z "$RECIPE" ]] && RECIPE="$VE2_RECIPE"
fi

detect_platform() {
  local path="$1"
  # Normalize for case / trailing slash noise
  local p
  p=$(printf '%s' "$path" | tr '[:upper:]' '[:lower:]')
  case "$p" in
    *shim_ve2*|*/ve2/*|*ve2_runner*|*runner/latency*)
      echo "VE2_EDGE"
      ;;
    *npu4*|*17f0*|*strix*|*aie2*)
      echo "NPU4_CANDIDATE"
      ;;
    *)
      echo "UNKNOWN"
      ;;
  esac
}

PLATFORM="UNKNOWN"
if [[ -n "$ELF" ]]; then
  PLATFORM=$(detect_platform "$ELF")
elif [[ -n "$XCLBIN" ]]; then
  PLATFORM=$(detect_platform "$XCLBIN")
elif [[ "$STAGE_VE2" -eq 1 ]]; then
  PLATFORM="VE2_EDGE"
fi
# Stage VE2 always tags platform even if paths override oddly
if [[ "$STAGE_VE2" -eq 1 ]]; then
  PLATFORM="VE2_EDGE"
fi

# Default --out by platform (only when caller did not pass --out)
if [[ -z "$OUT" ]]; then
  case "$PLATFORM" in
    VE2_EDGE) OUT="$DEFAULT_OUT_VE2" ;;
    NPU4_CANDIDATE) OUT="$DEFAULT_OUT_NPU4" ;;
    *) OUT="$DEFAULT_OUT_UNKNOWN" ;;
  esac
fi

section "resolved"
echo "mode=$MODE"
echo "PLATFORM=$PLATFORM"
echo "elf=${ELF:-}"
echo "xclbin=${XCLBIN:-}"
echo "recipe=${RECIPE:-}"
echo "out=$OUT"
echo "apply_copy=$APPLY_COPY"
echo "force_lab_instr=$FORCE_LAB_INSTR"
echo "allow_ve2_as_lab_instr=$ALLOW_VE2_AS_LAB_INSTR"

if [[ "$CLAIM_AIE2" -eq 1 ]]; then
  if [[ "$PLATFORM" == "VE2_EDGE" ]]; then
    die 4 "REFUSE --claim-aie2-known-good for VE2_EDGE / shim_ve2 sources (platform mismatch)"
  fi
  if [[ "$I_UNDERSTAND_MISMATCH" -ne 1 ]]; then
    die 4 "REFUSE --claim-aie2-known-good without --i-understand-platform-mismatch"
  fi
  if [[ "$PLATFORM" != "NPU4_CANDIDATE" ]]; then
    die 4 "REFUSE --claim-aie2-known-good: path not under npu4/17f0 candidate tree"
  fi
  # Even with npu4 path + mismatch flag: offline script never records known-good.
  die 4 "REFUSE: this offline script never records AIE2 known-good (need silicon OFM verify)"
fi

if [[ "$APPLY_COPY" -eq 0 ]]; then
  section "next steps"
  echo "COPY_OK=skipped (inventory only)"
  echo "INSPECT_OK=skipped"
  echo "PLATFORM=$PLATFORM"
  echo "CLAIM=not_known_good_for_aie2_17f0"
  echo "To stage VE2 research copy (gitignored binaries, trackable PROVENANCE.md):"
  echo "  bash scripts/extract_instruction_offline.sh --stage-ve2-nop"
  echo "Obtain npu4 PARTIAL_ELF / partner ml_txn→patched ctrltext for 17f0 Pass path."
  exit 0
fi

# Apply-copy path
[[ -n "$ELF" || -n "$XCLBIN" ]] || die 2 "--apply-copy/--stage-ve2-nop needs --elf and/or --xclbin (or default VE2 paths)"

if [[ -n "$ELF" && ! -f "$ELF" ]]; then
  die 4 "missing ELF: $ELF"
fi
if [[ -n "$XCLBIN" && ! -f "$XCLBIN" ]]; then
  die 4 "missing xclbin: $XCLBIN"
fi

# Refuse unsafe force-lab-instr *before* staging so OUT is not left half-written.
if [[ "$FORCE_LAB_INSTR" -eq 1 ]]; then
  if [[ -z "$ELF" || ! -f "$ELF" ]]; then
    die 2 "--force-lab-instr requires a readable --elf (or --stage-ve2-nop default ELF)"
  fi
  if [[ "$PLATFORM" == "VE2_EDGE" && "$ALLOW_VE2_AS_LAB_INSTR" -ne 1 ]]; then
    die 4 "REFUSE writing lab_instr.bin from VE2_EDGE without --allow-ve2-as-lab-instr \
(research-only; not Strix 17f0 known-good)"
  fi
fi

mkdir -p "$OUT/sources" "$OUT/inspect"

# Staged basenames (used for copy + sources.sha256 so hashes match on-disk names)
STAGED_ELF_NAME=""
STAGED_XCLBIN_NAME=""
STAGED_RECIPE_NAME=""
if [[ "$PLATFORM" == "VE2_EDGE" ]]; then
  [[ -n "$ELF" ]] && STAGED_ELF_NAME="ve2_runner_latency_nop.elf"
  [[ -n "$XCLBIN" ]] && STAGED_XCLBIN_NAME="ve2_runner_latency_validate.xclbin"
  [[ -n "$RECIPE" ]] && STAGED_RECIPE_NAME="ve2_runner_latency_recipe_latency.json"
else
  [[ -n "$ELF" ]] && STAGED_ELF_NAME="$(basename "$ELF")"
  [[ -n "$XCLBIN" ]] && STAGED_XCLBIN_NAME="$(basename "$XCLBIN")"
  [[ -n "$RECIPE" && -f "$RECIPE" ]] && STAGED_RECIPE_NAME="$(basename "$RECIPE")"
fi

# Copy with platform-encoded names for VE2
if [[ -n "$ELF" && -n "$STAGED_ELF_NAME" ]]; then
  cp -f "$ELF" "$OUT/sources/$STAGED_ELF_NAME"
  echo "copied ELF → $OUT/sources/$STAGED_ELF_NAME"
fi
if [[ -n "$XCLBIN" && -n "$STAGED_XCLBIN_NAME" ]]; then
  cp -f "$XCLBIN" "$OUT/sources/$STAGED_XCLBIN_NAME"
  echo "copied xclbin → $OUT/sources/$STAGED_XCLBIN_NAME"
fi
if [[ -n "$RECIPE" && -f "$RECIPE" && -n "$STAGED_RECIPE_NAME" ]]; then
  cp -f "$RECIPE" "$OUT/sources/$STAGED_RECIPE_NAME"
  echo "copied recipe → $OUT/sources/$STAGED_RECIPE_NAME"
elif [[ -n "$RECIPE" && -n "$STAGED_RECIPE_NAME" ]]; then
  echo "recipe: missing (skip): $RECIPE"
fi

# Optional inspect (best-effort)
INSPECT_OK=0
if [[ -n "$ELF" ]] && have readelf; then
  if have timeout; then
    timeout 30s readelf -hW "$ELF" >"$OUT/inspect/elf.header.txt" 2>&1 || true
    timeout 30s readelf -SW "$ELF" >"$OUT/inspect/elf.sections.txt" 2>&1 || true
  else
    readelf -hW "$ELF" >"$OUT/inspect/elf.header.txt" 2>&1 || true
    readelf -SW "$ELF" >"$OUT/inspect/elf.sections.txt" 2>&1 || true
  fi
  INSPECT_OK=1
fi
if [[ -n "$XCLBIN" ]] && have xclbinutil; then
  if have timeout; then
    timeout 30s xclbinutil --info --input "$XCLBIN" >"$OUT/inspect/xclbin.info.txt" 2>&1 || true
    timeout 30s xclbinutil --list-sections --input "$XCLBIN" >"$OUT/inspect/xclbin.sections.txt" 2>&1 || true
  else
    xclbinutil --info --input "$XCLBIN" >"$OUT/inspect/xclbin.info.txt" 2>&1 || true
    xclbinutil --list-sections --input "$XCLBIN" >"$OUT/inspect/xclbin.sections.txt" 2>&1 || true
  fi
  INSPECT_OK=1
fi

# Hashes — rewrite each stage; paths relative to OUT so `sha256sum -c sources.sha256` works from OUT
HASH_LINES=""
: >"$OUT/sources.sha256"
if have sha256sum; then
  if [[ -n "$STAGED_ELF_NAME" && -f "$OUT/sources/$STAGED_ELF_NAME" ]]; then
    h=$(sha256sum "$OUT/sources/$STAGED_ELF_NAME" | awk '{print $1}')
    HASH_LINES+="elf_sha256=$h"$'\n'
    echo "$h  sources/$STAGED_ELF_NAME" >>"$OUT/sources.sha256"
  fi
  if [[ -n "$STAGED_XCLBIN_NAME" && -f "$OUT/sources/$STAGED_XCLBIN_NAME" ]]; then
    h=$(sha256sum "$OUT/sources/$STAGED_XCLBIN_NAME" | awk '{print $1}')
    HASH_LINES+="xclbin_sha256=$h"$'\n'
    echo "$h  sources/$STAGED_XCLBIN_NAME" >>"$OUT/sources.sha256"
  fi
  if [[ -n "$STAGED_RECIPE_NAME" && -f "$OUT/sources/$STAGED_RECIPE_NAME" ]]; then
    h=$(sha256sum "$OUT/sources/$STAGED_RECIPE_NAME" | awk '{print $1}')
    HASH_LINES+="recipe_sha256=$h"$'\n'
    echo "$h  sources/$STAGED_RECIPE_NAME" >>"$OUT/sources.sha256"
  fi
fi

# Force lab_instr (unsafe paths already refused before staging)
LAB_INSTR_WRITTEN=0
if [[ "$FORCE_LAB_INSTR" -eq 1 ]]; then
  cp -f "$ELF" "$OUT/lab_instr.bin"
  LAB_INSTR_WRITTEN=1
  echo "WARN: wrote $OUT/lab_instr.bin from raw ELF (envelope study only)"
  echo "WARN: NOT_AIE2_KNOWN_GOOD=1 — do not claim ran_on_npu from this alone"
  if [[ "$PLATFORM" == "VE2_EDGE" ]]; then
    echo "WARN: VE2 ELF is not Strix 17f0 AIE2 known-good txn/ctrltext"
    echo "WARN: --allow-ve2-as-lab-instr accepted; PROVENANCE still marks NOT_AIE2_KNOWN_GOOD"
  fi
else
  echo "lab_instr.bin: not written (use --force-lab-instr only for raw-ELF study; never auto)"
fi

# PROVENANCE.md
STAGE_SHA=""
if [[ -f "$ROOT/kernel/dkms/staging/amdxdna-hardened-0.1.0/README.dragon-npu-stage" ]]; then
  STAGE_SHA=$(head -5 "$ROOT/kernel/dkms/staging/amdxdna-hardened-0.1.0/README.dragon-npu-stage" 2>/dev/null | tr '\n' ' ' || true)
fi

cat >"$OUT/PROVENANCE.md" <<EOF
# Instruction source provenance (offline extract)

**NOT_AIE2_KNOWN_GOOD=1**

| Field | Value |
|-------|--------|
| PLATFORM | $PLATFORM |
| CLAIM | not_known_good_for_aie2_17f0 |
| date_utc | $(date -u +%Y-%m-%dT%H:%M:%SZ 2>/dev/null || date) |
| elf_source | ${ELF:-} |
| xclbin_source | ${XCLBIN:-} |
| recipe_source | ${RECIPE:-} |
| out | $OUT |
| force_lab_instr | $FORCE_LAB_INSTR |
| allow_ve2_as_lab_instr | $ALLOW_VE2_AS_LAB_INSTR |
| lab_instr_written | $LAB_INSTR_WRITTEN |
| stage_note | $STAGE_SHA |

$HASH_LINES

## Honesty

- VE2 Runner \`nop.elf\` / \`validate.xclbin\` are **VE2 edge** fixtures (\`ops = ve2_ops\`),
  **not** Strix PCI \`1022:17f0\` / npu4 AIE2 known-good instruction BO content.
- Feeding raw ELF bytes into \`XDNA2_INSTRUCTION_BLOB\` only exercises envelope plumbing.
- Product \`ran_on_npu\` requires partner/npu4 patched ctrltext + OFM numerical verify.
- Binaries under this tree are **gitignored**; \`PROVENANCE.md\` / \`*.sha256\` / \`*.txt\` may be tracked.
- Script never sets \`XDNA2_ALLOW_EXEC_CMD\` or runs EXEC_CMD.
- \`lab_exec_once\` refuses blobs with sibling PROVENANCE NOT_AIE2_KNOWN_GOOD unless
  \`--allow-research-blob\`.

## Recipe (if VE2)

See staged \`sources/ve2_runner_latency_recipe_latency.json\` (kernel DPU instance \`k1\`,
IFM-sized 20-byte args) — not the dragon-npu matmul OFM contract.
EOF

section "result"
echo "COPY_OK"
if [[ "$INSPECT_OK" -eq 1 ]]; then
  echo "INSPECT_OK"
else
  echo "INSPECT_OK=partial_or_skipped (tools missing or no sources)"
fi
echo "PLATFORM=$PLATFORM"
echo "CLAIM=not_known_good_for_aie2_17f0"
echo "PROVENANCE=$OUT/PROVENANCE.md"
echo "lab_instr_written=$LAB_INSTR_WRITTEN"
echo "ran_on_npu=false"
echo
echo "Next: obtain npu4 local_shim_test_data or partner ml_txn→patched ctrltext;"
echo "  never treat VE2 staging as silicon Pass. See docs/XDNA2_TXN_RESEARCH.md"

exit 0
