#!/usr/bin/env bash
# Soft smoke: TMPDIR=/ and TMPDIR=symlink-to-/ clamp to /tmp (classify/c3/c4 policy).
#
# Mirrors the fail-closed block in tools/classify_last_apply_residual.sh (and c3/c4):
#   strip → lexical empty|/ clamp → realpath -m (keep only if non-empty) →
#   empty|/|-L clamp → /tmp
#
# Cases:
#   1) TMPDIR=/ → /tmp
#   2) TMPDIR=symlink-to-/ (realpath ok) → /tmp
#   3) same symlink with realpath stub that fails → still /tmp via -L check
#
# Exit: 0 always for soft gate use (prints SMOKE_OK or SMOKE_WARN).
# Never sets ALLOW / never opens L3 / never unlocks C.
#
# Usage:
#   bash tools/smoke_tmpdir_root_clamp.sh
#   # wired soft in tools/run_offline_gates.sh (not hard-fail)
set -euo pipefail
export PATH="/usr/bin${HOME:+:${HOME}/.local/bin}:${PATH:-}"
unset XDNA2_ALLOW_EXEC_CMD || true

# Same clamp policy as classify/c3/c4 (keep in sync).
clamp_tmp_root() {
  local TMP_ROOT="${1:-/tmp}"
  TMP_ROOT="${TMP_ROOT%/}"
  if [[ -z "$TMP_ROOT" || "$TMP_ROOT" == "/" ]]; then
    TMP_ROOT="/tmp"
  fi
  if [[ -d "$TMP_ROOT" ]]; then
    local resolved
    resolved="$(realpath -m "$TMP_ROOT" 2>/dev/null || true)"
    if [[ -n "$resolved" ]]; then
      TMP_ROOT="$resolved"
    fi
  fi
  if [[ -z "$TMP_ROOT" || "$TMP_ROOT" == "/" || -L "$TMP_ROOT" ]]; then
    TMP_ROOT="/tmp"
  fi
  printf '%s' "$TMP_ROOT"
}

warn=0
pass=0

# --- case 1: TMPDIR=/ ---
out="$(clamp_tmp_root "/")"
if [[ "$out" == "/tmp" ]]; then
  pass=$((pass + 1))
  echo "OK: TMPDIR=/ → $out"
else
  warn=1
  echo "WARN: TMPDIR=/ expected /tmp got '$out'" >&2
fi

# --- case 2: symlink to / ---
LINK="/tmp/dragon_npu_smoke_rootlink_$$"
rm -f "$LINK" 2>/dev/null || true
if ln -sfn / "$LINK" 2>/dev/null; then
  out="$(clamp_tmp_root "$LINK")"
  if [[ "$out" == "/tmp" ]]; then
    pass=$((pass + 1))
    echo "OK: TMPDIR=symlink-to-/ → $out"
  else
    warn=1
    echo "WARN: symlink-to-/ expected /tmp got '$out'" >&2
  fi

  # --- case 3: realpath fail-closed (PATH stub) still clamps via -L ---
  STUB_DIR="/tmp/dragon_npu_smoke_rpstub_$$"
  mkdir -p "$STUB_DIR"
  cat >"$STUB_DIR/realpath" <<'STUB'
#!/usr/bin/env bash
exit 1
STUB
  chmod +x "$STUB_DIR/realpath"
  out="$(
    PATH="$STUB_DIR:/usr/bin:/bin" bash -c '
      clamp_tmp_root() {
        local TMP_ROOT="${1:-/tmp}"
        TMP_ROOT="${TMP_ROOT%/}"
        if [[ -z "$TMP_ROOT" || "$TMP_ROOT" == "/" ]]; then
          TMP_ROOT="/tmp"
        fi
        if [[ -d "$TMP_ROOT" ]]; then
          local resolved
          resolved="$(realpath -m "$TMP_ROOT" 2>/dev/null || true)"
          if [[ -n "$resolved" ]]; then
            TMP_ROOT="$resolved"
          fi
        fi
        if [[ -z "$TMP_ROOT" || "$TMP_ROOT" == "/" || -L "$TMP_ROOT" ]]; then
          TMP_ROOT="/tmp"
        fi
        printf "%s" "$TMP_ROOT"
      }
      clamp_tmp_root "'"$LINK"'"
    '
  )"
  rm -rf "$STUB_DIR"
  if [[ "$out" == "/tmp" ]]; then
    pass=$((pass + 1))
    echo "OK: realpath-fail + symlink-to-/ → $out (-L fail-closed)"
  else
    warn=1
    echo "WARN: realpath-fail symlink expected /tmp got '$out'" >&2
  fi
  rm -f "$LINK"
else
  warn=1
  echo "WARN: cannot create symlink to / under /tmp (skip cases 2–3)" >&2
fi

echo "smoke_tmpdir_root_clamp pass=$pass warn=$warn not_c_ready=1 not_ran_on_npu=1"
if [[ "$warn" -eq 0 && "$pass" -ge 1 ]]; then
  echo "SMOKE_OK: tmpdir root clamp"
else
  echo "SMOKE_WARN: tmpdir root clamp (soft; not hard gate)"
fi
# Soft: always exit 0 so run_offline_gates does not hard-fail on warn.
exit 0
