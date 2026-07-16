# Residual — silicon path (honest status)

**Date:** 2026-07-16  
**Honesty:** No silicon `ran_on_npu` Pass claimed in this session. Live EXEC is operator HITL.

## Strategy (stop apply ping-pong)

| Mode | Who | Command |
|------|-----|---------|
| **Offline** | agent | contract tests, GT oracle, research |
| **B GT** | you once | `bash tools/hitl_b_gt_capture.sh` (+ optional `--with-vendor`) |
| **C Pass** | you once | `bash tools/hitl_c_pass_apply.sh --assume-l3-already-open` |

See `docs/HITL_BC.md`. **Do not** serial “try flag X on live silicon” as default progress.

---

## Done (offline / pure-direct)

### Assets & lab plumbing (A/B/C)

| Item | Status |
|------|--------|
| AMD-packed WTS export 174080 non-zero | done (`export_mladf_wts`) |
| CTRL 29664 + count-4 pin + auto-patch residual | done |
| OFM golden capture / validate / stage | done |
| Preflight + `lab_exec_once --silicon-mladf` recipe | done |
| Zero golden / zero WTS fail-closed | done |
| Pass flags pure (`decide_ran_on_npu`, `silicon_ran_on_npu_flags_ok`) | done |

### Phase D — EXEC unblock (userspace)

| Item | Status |
|------|--------|
| **D1** refuse `xdna_addr` 0 / `u64::MAX` | done |
| **D2** instruction BO = `BufferType::Dev` + heap fill | **live OK** — `ty=Dev xdna_addr=0x4020000` |
| **D2b** SHMEM PASID pin (persistent mmap for IFM/WTS/OFM/ctrl) | **live OK** — all IFM/WTS/OFM/ctrl `valid_addr=true` |
| **D3 diagnostics** `bo_diag` on fail + `XDNA2_LAB_BO_DIAG` | done |
| **D3c DevHeap UVA 64 MiB align** | **code landed** — live map_host `addr 0x7f3515400000` not 64 MiB-aligned → FW `0x4000003`; XRT-style `mmap_dev_heap_drm_aligned` |
| **D3 map_host / exec_eio** | **live clear** after D3c — `EXEC_CMD+WAIT seq=0 accepted` |
| **D3d ctrl size=6 read_add_write** | **code live**; HITL re-apply still **zero OFM** → not sole residual |
| **D3e ERT state after WAIT** | **code landed** — `ert_state=Completed\|Error` + `io_paddrs` in detail; decide needs Completed when readable |
| **D4** honest `exec_eio` | done |
| **empty EXEC refuse** | done |
| **assume-l3 verify** | done (`--allow-open-exec-cmd`) |

### Phase E readiness (offline)

| Item | Status |
|------|--------|
| OFM decide wire when LAB_OFM=1 | done |
| Dump-on-mismatch `ofm_stats` (nonzero, first bf16/f32) | done |
| Product Pass after allclose | **blocked on submit OK + HITL** |

### Operator UX / efficiency (pure)

| Item | Status |
|------|--------|
| `silicon_lab_operator.sh apply` (risk wrap + restore note) | done |
| Dry-run plan: silicon-mladf shape note (not m=n=k=1 compute) | done |
| Status: `exec_cmd_allow` + readiness one-liner | done |
| **G1** `suggest_hwctx_tiles` in timed path (env override wins) | done |
| **G2** multi-shot BO reuse | **residual** (below) |

---

## Highest-value next (HITL — two scripts only)

1. **Milestone B** (unblocks agent offline parity):
   ```fish
   bash tools/hitl_b_gt_capture.sh
   # optional vendor golden: bash tools/hitl_b_gt_capture.sh --with-vendor
   ```
2. **Milestone C** (Pass attempt — only after agent says offline green; **ELF not .bin**):
   ```fish
   sudo sh -c 'echo 1 > /sys/module/amdxdna/parameters/exec_cmd_allow'
   bash tools/hitl_c_pass_apply.sh --assume-l3-already-open
   sudo sh -c 'echo 0 > /sys/module/amdxdna/parameters/exec_cmd_allow'
   ```
3. Expect `aiebu_elf=1` + `aiebu_rela_patched` in detail; then non-zero OFM / allclose → `ran_on_npu`.

---

## Residual product work (not blockers for first Pass)

| Item | Notes |
|------|--------|
| **G2 BO reuse** | Each `timed_matmul` still CREATE/fill/destroy instr + I/O BOs. Multi-shot reuse needs a small state machine (keep hwctx + WTS/ctrl/instr; re-fill IFM/OFM only). Document only until Pass holds. |
| Ctrl meta size=6 full XRT patch | **D3d landed** (`read_add_write`); re-verify on HITL if OFM still wrong |
| F2 `npu_exec` product wire | After fixture Pass green; still double-gated |
| Multi-key / larger M | After first Pass |

---

## Autonomous validation (no sudo / no ALLOW)

```bash
export PATH=/usr/bin:$PATH
cargo fmt -- --check
cargo clippy --features amd-xdna2 --lib -- -D warnings
cargo test --features amd-xdna2 --lib
bash -n tools/silicon_lab_operator.sh scripts/lab_exec_once.sh
bash tools/silicon_lab_preflight.sh --print-env || true
bash tools/silicon_lab_operator.sh status || true
bash scripts/lab_exec_once.sh --silicon-mladf  # dry-run only
```

**Never** set `XDNA2_ALLOW_EXEC_CMD=1` in default tests. Dry-run green ≠ `ran_on_npu`.

---

## Env recipe (dry-run first)

```bash
bash tools/export_mladf_wts.sh
bash tools/stage_silicon_lab_assets.sh --wts-from testdata/xdna2_cmd/mladf_wts_export/wts.bin
# non-zero golden already installed under silicon_lab/ or:
# export XDNA2_LAB_OFM_GOLDEN=…/ofm_golden.bin
bash tools/silicon_lab_preflight.sh --require-pass-assets --print-env
bash scripts/lab_exec_once.sh --silicon-mladf --require-pass-assets
# apply: bash tools/silicon_lab_operator.sh apply
```

See `docs/SILICON_PATH_LAB.md`.
