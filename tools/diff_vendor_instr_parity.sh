#!/usr/bin/env bash
# Diff vendor post-patch instr vs pure-direct patch (same paddrs when available).
#
# Inputs (defaults under testdata/xdna2_cmd/gt_capture):
#   instr_vendor_post_patch.bin   — from capture_mladf_instr_vendor
#   instr_vendor_txn_bin.bin      — classic unpatched 1688 sibling (preferred
#                                   when post_patch is oversized multi-txn)
#   vendor_paddrs.json            — optional paddrs + capture_mode
#   lab_instr.elf                 — staged aiebu ELF
#
# Exit:
#   0  exact match (first min(len, ctrltext) bytes) OR --self pure oracle OK
#   2  mismatch (prints rela-site u64 table + CLASS_HINT A–E)
#   3  missing inputs (vendor / ELF / pure oracle)
#
# CLASS_HINT (honesty):
#   A  exact match first N (unpatched classic vs pure unpatched → body parity)
#   B  all rela DIFF after both post-patched with same paddrs (symbol/paddr)
#   C  true non-rela content mismatch on same length class (not multi-txn prefix)
#   D  unpatched capture mode / vendor matches raw txn / map 1688
#   E  length class mismatch
#
# Unpatched capture (capture_mode contains unpatched / txn_bin_fallback /
# txn_container): do NOT re-patch pure with vendor paddrs (false rela DIFF).
# Prefer sibling classic 1688 when post_patch is multi-txn wrong body (32k).
#
# No L3 / no EXEC / ≠ ran_on_npu.
#
# Env:
#   XDNA2_GT_OUT              gt_capture dir
#   XDNA2_REQUIRE_VENDOR_PARITY=1  (used by silicon_contract_check to hard-fail)
set -euo pipefail
export PATH="/usr/bin:${HOME}/.local/bin:${PATH}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

GT="${XDNA2_GT_OUT:-$ROOT/testdata/xdna2_cmd/gt_capture}"
ELF="$ROOT/testdata/xdna2_cmd/silicon_lab/lab_instr.elf"
VENDOR="$GT/instr_vendor_post_patch.bin"
TXN_SIB="$GT/instr_vendor_txn_bin.bin"
PADDRS="$GT/vendor_paddrs.json"
SELF=0

for a in "$@"; do
  case "$a" in
    --self) SELF=1 ;;
    -h|--help)
      sed -n '1,40p' "$0" | sed -n 's/^# \{0,1\}//p'
      exit 0
      ;;
  esac
done

fail() { echo "PARITY_FAIL: $*" >&2; exit 2; }
miss() { echo "PARITY_MISS: $*" >&2; exit 3; }
ok() { echo "PARITY_OK: $*"; }

[[ -f "$ELF" ]] || miss "missing $ELF — stage aiebu ELF (bash tools/stage_aiebu_lab_instr.sh)"

# Baseline pure-direct oracle: always synthetic paddrs (strip ambient GT_PADDRS)
env -u XDNA2_GT_PADDRS_JSON XDNA2_GT_OUT="$GT" bash tools/dump_mladf_gt_offline.sh >/dev/null
PURE_SYN="$GT/instr_post_patch.bin"
[[ -f "$PURE_SYN" ]] || miss "missing pure oracle $PURE_SYN"

if [[ "$SELF" -eq 1 ]]; then
  # Self: pure oracle length/sha sanity only (no tautological cmp, no vendor)
  h1=$(sha256sum "$PURE_SYN" | awk '{print $1}')
  psz=$(wc -c <"$PURE_SYN" | tr -d ' ')
  if [[ "$psz" -ne 1688 ]]; then
    fail "self pure size $psz != 1688 (CLASS_HINT E length)"
  fi
  ok "self pure sha256=$h1 size=$psz CLASS_HINT=self_ok"
  exit 0
fi

[[ -f "$VENDOR" ]] || miss "missing $VENDOR — run: bash tools/capture_mladf_instr_vendor.sh (or silicon_lab_operator.sh capture-instr)"

# Detect unpatched capture mode from vendor_paddrs.json (if present).
UNPATCHED_MODE=0
CAPTURE_MODE=""
if [[ -f "$PADDRS" ]]; then
  CAPTURE_MODE="$(python3 - "$PADDRS" <<'PY'
import json, sys
try:
    d = json.load(open(sys.argv[1]))
except Exception:
    sys.exit(0)
print(d.get("capture_mode") or "")
PY
)"
  case "$CAPTURE_MODE" in
    *unpatched*|*txn_bin_fallback*|*txn_container*)
      UNPATCHED_MODE=1
      echo "NOTE: capture_mode='$CAPTURE_MODE' → UNPATCHED path (no pure re-patch with paddrs)"
      ;;
  esac
fi

# Pure re-patch with vendor paddrs ONLY for true registry post-patch captures.
# Unpatched modes must compare against pure *unpatched* raw txn (instr_raw_txn)
# so rela sites are not false-DIFF'd (synthetic paddrs rewrite → false B/C).
PURE_CMP="$PURE_SYN"
RAW_TXN="$GT/instr_raw_txn.bin"
if [[ "$UNPATCHED_MODE" -eq 1 ]]; then
  if [[ -f "$RAW_TXN" ]]; then
    rsz=$(wc -c <"$RAW_TXN" | tr -d ' ')
    if [[ "$rsz" -eq 1688 ]]; then
      PURE_CMP="$RAW_TXN"
      echo "NOTE: unpatched capture — comparing pure unpatched raw ($RAW_TXN) vs vendor (body class)"
    else
      echo "NOTE: unpatched capture — instr_raw_txn size=$rsz (want 1688); fall back to pure synthetic" >&2
      PURE_CMP="$PURE_SYN"
    fi
  else
    echo "NOTE: unpatched capture — no instr_raw_txn.bin; fall back to pure synthetic (rela may false-DIFF)" >&2
    PURE_CMP="$PURE_SYN"
  fi
elif [[ -f "$PADDRS" ]]; then
  if python3 - "$PADDRS" <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
# Explicit paddrs_known=false → incomplete
if d.get("paddrs_known") is False:
    sys.exit(1)
need = ["ifm_paddr", "wts_paddr", "ofm_paddr"]

U64_MAX = (1 << 64) - 1


def valid_device_paddr(n: int) -> bool:
    # Align with Rust is_valid_device_paddr: refuse 0 and u64::MAX
    return n != 0 and n != U64_MAX


def ok_v(x):
    if x is None or x == "null" or x == "":
        return False
    if isinstance(x, (int, float)):
        return valid_device_paddr(int(x))
    s = str(x).strip().lower()
    if s in ("0", "0x0", "0x00", "null", "none", "0xffffffffffffffff"):
        return False
    try:
        n = int(s, 16) if s.startswith("0x") else int(s, 0)
        return valid_device_paddr(n)
    except ValueError:
        return False

sys.exit(0 if all(ok_v(d.get(k)) for k in need) else 1)
PY
  then
    echo "NOTE: re-patch pure-direct with vendor_paddrs.json …"
    if XDNA2_GT_PADDRS_JSON="$PADDRS" XDNA2_GT_OUT="$GT" \
      cargo run --features amd-xdna2 --example dump_mladf_gt_offline --quiet 2>/dev/null; then
      PURE_CMP="$GT/instr_post_patch.bin"
      echo "NOTE: pure oracle re-patched with vendor paddrs → $PURE_CMP"
    else
      echo "WARN: XDNA2_GT_PADDRS_JSON re-patch failed — falling back to synthetic pure" >&2
      # Restore synthetic pure after failed re-patch attempt
      env -u XDNA2_GT_PADDRS_JSON XDNA2_GT_OUT="$GT" bash tools/dump_mladf_gt_offline.sh >/dev/null
      PURE_CMP="$PURE_SYN"
    fi
  else
    echo "NOTE: vendor_paddrs null/incomplete/zero or paddrs_known=false — comparing pure synthetic paddrs vs vendor (algorithm/layout only)"
  fi
else
  echo "NOTE: no vendor_paddrs.json — synthetic pure paddrs vs vendor"
fi

python3 - "$VENDOR" "$PURE_CMP" "$ELF" "$GT" "$UNPATCHED_MODE" "$CAPTURE_MODE" <<'PY'
import sys, struct
from pathlib import Path

vendor_path = Path(sys.argv[1])
pure_path = Path(sys.argv[2])
elf_path = Path(sys.argv[3])
gt = Path(sys.argv[4])
unpatched_mode = sys.argv[5] == "1"
capture_mode = sys.argv[6] if len(sys.argv) > 6 else ""

vendor_orig = vendor_path.read_bytes()
pure = pure_path.read_bytes()
elf = elf_path.read_bytes()

vendor = vendor_orig
vendor_src = vendor_path.name
recovered_via_sibling = False

# Peel .ctrltext length if vendor is longer (page pad)
def ctrltext_len(e: bytes) -> int:
    if len(e) < 52 or e[0:4] != b"\x7fELF":
        return 1688
    e_shoff = struct.unpack_from("<I", e, 32)[0]
    e_shentsize = struct.unpack_from("<H", e, 46)[0]
    e_shnum = struct.unpack_from("<H", e, 48)[0]
    e_shstrndx = struct.unpack_from("<H", e, 50)[0]
    str_hdr = e_shoff + e_shentsize * e_shstrndx
    str_off = struct.unpack_from("<I", e, str_hdr + 16)[0]
    str_size = struct.unpack_from("<I", e, str_hdr + 20)[0]
    strtab = e[str_off : str_off + str_size]
    for i in range(e_shnum):
        h = e_shoff + e_shentsize * i
        name_off = struct.unpack_from("<I", e, h)[0]
        sh_type = struct.unpack_from("<I", e, h + 4)[0]
        if sh_type != 1:
            continue
        end = strtab.find(b"\0", name_off)
        name = strtab[name_off:end].decode()
        if name == ".ctrltext":
            return struct.unpack_from("<I", e, h + 20)[0]
    return 1688

ct_len = ctrltext_len(elf)

# Recover classic 1688 when post_patch is oversized multi-txn wrong body.
# NEVER replace true registry post-patch with unpatched sibling (would false-A).
# Recover only when:
#   (a) unpatched capture_mode (txn_*_unpatched / contains "unpatched"), OR
#   (b) clearly multi-txn wrong body AND capture_mode is not registry_get_instr_bo
# Sibling must be classic 1688 non-JSON.
cm = (capture_mode or "").strip()
is_registry_mode = cm == "registry_get_instr_bo" or cm.startswith("registry_")
txn_sib = gt / "instr_vendor_txn_bin.bin"
if is_registry_mode:
    print(
        "NOTE: capture_mode is registry post-patch — "
        "not recovering unpatched sibling (keep vendor post_patch)"
    )
elif txn_sib.is_file():
    sib = txn_sib.read_bytes()
    if len(sib) == 1688 and not (
        sib[0:1] == b"{" or (len(sib) > 0 and sib[0] in (ord("{"), ord("[")))
    ):
        need_recover = False
        multi_wrong_body = (
            len(vendor_orig) > 4096
            and len(vendor_orig) >= 1688
            and vendor_orig[:1688] != sib
        )
        if unpatched_mode:
            # Unpatched fallback may have stored multi-txn as post_patch (HITL).
            if len(vendor_orig) > 4096:
                need_recover = True
            elif len(vendor_orig) not in (1688, 4096):
                need_recover = True
            elif (
                len(vendor_orig) > 1688
                and vendor_orig[:1688] != sib
            ):
                need_recover = True
        elif multi_wrong_body:
            # Legacy/missing mode label: multi-txn wrong body only, never registry.
            need_recover = True
            print(
                "NOTE: multi-txn wrong body without unpatched label — "
                "recover classic sibling (capture_mode not registry)"
            )
        if need_recover:
            vendor = sib
            vendor_src = txn_sib.name
            recovered_via_sibling = True
            print(
                f"NOTE: recovered classic vendor via sibling {txn_sib.name} "
                f"(post_patch was {len(vendor_orig)} B multi-txn/wrong body; "
                f"discard multi-txn prefix — avoid false content class)"
            )

# Prefer compare first ct_len of both (vendor may be page-padded to 4096)
n = min(len(vendor), len(pure), ct_len if ct_len else min(len(vendor), len(pure)))
# Always also report first-1688 if different from n
n1688 = min(len(vendor), len(pure), 1688)
v = vendor[:n]
p = pure[:n]

print(
    f"compare_len={n} vendor_len={len(vendor)} pure_len={len(pure)} "
    f"ctrltext_expect={ct_len} vendor_src={vendor_src} "
    f"unpatched_mode={int(unpatched_mode)} capture_mode={capture_mode or 'n/a'}"
)
if recovered_via_sibling:
    print(f"NOTE: post_patch_orig_len={len(vendor_orig)} recovered_via_sibling=1")
if len(vendor) > n:
    print(f"NOTE: vendor page-padded or longer — comparing first {n} bytes only")

# Parse rela from elf for site table
def parse_rela(e: bytes):
    if len(e) < 52 or e[0:4] != b"\x7fELF":
        return []
    e_shoff = struct.unpack_from("<I", e, 32)[0]
    e_shentsize = struct.unpack_from("<H", e, 46)[0]
    e_shnum = struct.unpack_from("<H", e, 48)[0]
    e_shstrndx = struct.unpack_from("<H", e, 50)[0]
    str_hdr = e_shoff + e_shentsize * e_shstrndx
    str_off = struct.unpack_from("<I", e, str_hdr + 16)[0]
    str_size = struct.unpack_from("<I", e, str_hdr + 20)[0]
    strtab = e[str_off : str_off + str_size]
    for i in range(e_shnum):
        h = e_shoff + e_shentsize * i
        name_off = struct.unpack_from("<I", e, h)[0]
        sh_type = struct.unpack_from("<I", e, h + 4)[0]
        if sh_type != 4:  # SHT_RELA
            continue
        end = strtab.find(b"\0", name_off)
        name = strtab[name_off:end].decode()
        if name != ".rela.dyn":
            continue
        off = struct.unpack_from("<I", e, h + 16)[0]
        size = struct.unpack_from("<I", e, h + 20)[0]
        out = []
        for o in range(off, off + size, 12):
            roff, info, addend = struct.unpack_from("<IIi", e, o)
            out.append((roff, info >> 8, info & 0xFF, addend))
        return out
    return []

relas = parse_rela(elf)
print(f"rela_count={len(relas)}")
mismatches = 0
rela_ok = 0
rela_diff = 0
rela_oob = 0
for roff, sym, rty, add in relas:
    if roff + 8 > n:
        print(f"  OOB off={roff:#x}")
        mismatches += 1
        rela_oob += 1
        continue
    pv = struct.unpack_from("<Q", p, roff)[0]
    vv = struct.unpack_from("<Q", v, roff)[0]
    mark = "OK" if pv == vv else "DIFF"
    if pv != vv:
        mismatches += 1
        rela_diff += 1
    else:
        rela_ok += 1
    print(f"  {mark} off={roff:#x} sym={sym} addend={add} pure={pv:#x} vendor={vv:#x}")

# Full byte compare on prefix
prefix_eq = v == p
first_diff = None
if not prefix_eq:
    for i, (a, b) in enumerate(zip(v, p)):
        if a != b:
            first_diff = i
            print(f"PARITY_FAIL: first_byte_diff off={i:#x} pure={a:#x} vendor={b:#x}")
            break

# Extra: first 1688 always reported when vendor longer
if n1688 != n or len(vendor) > 1688:
    v18 = vendor[:n1688]
    p18 = pure[:n1688]
    print(f"first_1688_match={v18 == p18} (vendor may be page-padded)")

print(f"rela_site_mismatches={mismatches} rela_ok={rela_ok} rela_diff={rela_diff} rela_oob={rela_oob}")

# Detect vendor ≈ unpatched raw / map 1688
txn_candidates = [
    gt / "instr_vendor_txn_bin.bin",
    gt / "instr_raw_txn.bin",
]
prefix_n = min(1688, n, len(vendor))
vendor_prefix = vendor[:prefix_n]
matches_raw = False
matched_raw_name = None
for raw_path in txn_candidates:
    if not raw_path.is_file():
        continue
    raw = raw_path.read_bytes()
    raw_prefix = raw[:prefix_n]
    if len(raw_prefix) == prefix_n and vendor_prefix == raw_prefix:
        matches_raw = True
        matched_raw_name = raw_path.name
        break

# CLASS_HINT A–E
# A: exact match
# B: all rela sites DIFF, non-rela mostly OK → symbol map / paddr binding
#    (only meaningful when both sides post-patched with same paddrs)
# C: true non-rela content mismatch on same length class
# D: unpatched capture / vendor ≈ raw txn / map 1688
# E: length class mismatch
hint = "E"
if prefix_eq:
    # Exact body match. Unpatched classic match is still A for body class;
    # also annotate D when unpatched mode (operator honesty).
    hint = "A"
elif len(vendor) < 1688 or (len(pure) != 1688 and len(pure) < 1688):
    hint = "E"
elif unpatched_mode or matches_raw or recovered_via_sibling:
    # Unpatched path or recovered classic sibling: not false C from 32k prefix.
    if matches_raw or recovered_via_sibling:
        hint = "D"
    else:
        # unpatched mode but body still mismatches pure unpatched → D (not C)
        hint = "D"
elif rela_diff == len(relas) and len(relas) > 0 and rela_ok == 0:
    # Check non-rela bytes roughly equal?
    non_rela_diff = 0
    covered = set()
    for roff, _, _, _ in relas:
        for b in range(roff, min(roff + 8, n)):
            covered.add(b)
    for i in range(n):
        if i in covered:
            continue
        if v[i] != p[i]:
            non_rela_diff += 1
    if non_rela_diff < max(8, n // 50):
        hint = "B"
    else:
        hint = "C"
elif rela_ok == len(relas) and len(relas) > 0 and not prefix_eq:
    hint = "C"
elif len(vendor) in (1688, 4096) and not prefix_eq and rela_diff > 0:
    # mixed rela + content
    if rela_ok == 0 and rela_diff > 0:
        hint = "B"
    else:
        hint = "C"
else:
    hint = "C"

# Strengthen CLASS_HINT D annotations
if matches_raw:
    print(f"CLASS_HINT D strong: vendor first {prefix_n} == {matched_raw_name}")
if recovered_via_sibling:
    print("CLASS_HINT D: recovered classic 1688 sibling (multi-txn post_patch discarded)")
if unpatched_mode and hint in ("A", "D"):
    print(
        f"NOTE: unpatched capture_mode — body class {hint}; "
        "NOT true post-patch; NOT C-ready"
    )

# If exact match under unpatched, prefer reporting A with D note already printed
if prefix_eq and unpatched_mode:
    # Keep A (exact body parity unpatched). D note already printed above.
    pass
elif (not prefix_eq) and (unpatched_mode or matches_raw or recovered_via_sibling):
    hint = "D"

labels = {
    "A": "exact match first N bytes",
    "B": "all rela DIFF / symbol map or paddrs (re-patch with live paddrs)",
    "C": "true non-rela content mismatch on same length class (not multi-txn prefix)",
    "D": "vendor ≈ unpatched raw txn / map 1688 / unpatched capture mode",
    "E": "length class mismatch (expect ctrltext 1688; vendor page-pad OK if prefix matches)",
}
print(f"CLASS_HINT {hint}: {labels[hint]}")

if prefix_eq:
    print("PARITY_OK: exact match first", n, "bytes")
    sys.exit(0)

sys.exit(2)
PY
