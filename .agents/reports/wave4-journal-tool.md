# W4a T4 — Journal scrape / evidence helper polish

**Date:** 2026-08-11
**Scope:** offline read-only tools only
**Honesty:** invent_pass=0 not_c_ready=1 not_ran_on_npu=1; scrape alone ≠ Pass

## Done

### New: `tools/journal_npu_scrape.sh`

Read-only kernel journal helper (user session; **no sudo**; no L3; no ALLOW; no dyndbg).

| Flag / env | Behavior |
|------------|----------|
| (default) | last **24** hours (`XDNA2_JOURNAL_HOURS` override) |
| `--hours N` | last N hours |
| `--since TIMESTAMP` | journalctl `--since` |
| `--boot` | this boot only (`-b`) |
| `--lines N` | max printed match lines (default 60; `XDNA2_JOURNAL_LINES`) |
| exit | **always 0 soft** (including SKIP) |

**Keywords (case-insensitive):**
`IO_PAGE_FAULT`, `APP_LOAD`, `aie2_set_cmd_timeout`, `Get bo failed`, `Response status`, `wait_cmd`, `EINVAL`, `Failed cmd`, `Status 0x`, `0x3000003`, `0x4000003`, `amdxdna`, `CONFIG`, `aie2_error_backtrack`, `aie2_hwctx_cfg_debug_bo`

**Soft SKIP reasons (never hard-fail):**

- `journalctl_not_available`
- `journal_permission_denied` / `journal_unreadable_or_empty`
- `journalctl_empty_or_failed`
- `helper_missing` (callers only)
- `bad_args` (still exit 0)

**Output tokens:**

- `journal_hint=…` per matching line (printed = last N of total)
- `match_total=` / `match_printed=`
- `soft_tokens io_page_fault=… app_load_hint=… aie2_set_cmd_timeout=… get_bo_failed=… einval_hint=… response_status=…`
  (computed on **full** match set, not only the printed tail)
- Footer: `JOURNAL_SCRAPE_OK invent_pass=0 not_c_ready=1 not_ran_on_npu=1 journal_scrape_alone_not_pass=1`

### Extended: `tools/collect_post_c2_evidence.sh`

Journal section now calls:

```bash
bash tools/journal_npu_scrape.sh --boot --lines 40
```

Fallback inline `rg` if helper missing (still soft). Header comment points at the new helper.

### Soft wire: `tools/pass_unblock_once.sh --agent-offline`

After bag status, soft-runs:

```bash
bash tools/journal_npu_scrape.sh --hours 24 --lines 30
```

Never hard-fails agent-offline. No cargo/Rust wire (pure `classify_journal_soft_signals` / `parse_journal_*` remain for paste text offline; shell scrape is the operator-facing journal surface).

## Gates

```text
bash -n tools/journal_npu_scrape.sh     → 0
bash -n tools/collect_post_c2_evidence.sh → 0
bash -n tools/pass_unblock_once.sh      → 0
```

**Live smoke (this host, user session readable):**

```text
bash tools/journal_npu_scrape.sh --hours 24 --lines 20
  → JOURNAL_SCRAPE_OK; soft_tokens io_page_fault=1 aie2_set_cmd_timeout=1 get_bo_failed=1
bash tools/journal_npu_scrape.sh --boot --lines 3
  → match_total=144 match_printed=3; soft_tokens still io_page_fault=1 (full set)
```

Matches prior spent flexml / debug-BO residual fingerprints in journal (IO_PAGE_FAULT ~`0x8fff…`, `aie2_set_cmd_timeout`, `Get bo 2 failed`). **Research only; not Pass.**

## Not done / frozen

- No live EXEC / ALLOW / unlock mint
- No invent Pass / no `ran_on_npu`
- No kernel dyndbg flip
- No new C_UNLOCK
- Pure Rust journal parsers not re-invoked from cargo here (paste path already exists)

## How to use

```bash
# Post-shot evidence pack (includes scrape --boot)
bash tools/collect_post_c2_evidence.sh /tmp/dragon_npu_last_apply_….json

# Standalone window
bash tools/journal_npu_scrape.sh --hours 48
bash tools/journal_npu_scrape.sh --since "2026-08-11 10:00:00" --lines 100
bash tools/journal_npu_scrape.sh --boot

# Layer A offline wrap (soft journal section)
bash tools/pass_unblock_once.sh --agent-offline
```

## Residual honesty

Scrape evidence improves residual notes after live shots. It does **not** close Pass residual, does **not** match load-accepted material, and does **not** justify a new unlock alone.
