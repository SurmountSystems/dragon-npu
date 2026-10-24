# Implement report: T3b IO_PAGE_FAULT vs BO map audit

**Date:** 2026-08-10
**Slice:** T3b only (pure residual science offline)
**Honesty:** invent_pass=0 · **no** product Pass · **no** `ran_on_npu` · **no** new unlock minted
**Unlock:** `C_UNLOCK=11bc9016` still **CONSUMED** (do not reopen)

---

## Plain English

The stock live shot under `11bc9016` got past CONFIG_CU and EXEC, then WAIT_CMD returned EINVAL immediately. Journal shows **IO_PAGE_FAULT** at **`0x2fff80007f40`** plus `aie2_set_cmd_timeout`.

**T3b** lands pure helpers that:

1. Parse last_apply `bo_diag=[...]` into labeled bases (`DevHeap`, `instr`, `ifm`, `wts`, `ofm`, `ctrl`) with `xdna_addr`.
2. Parse journal lines for AMD-Vi-style `address=0x…` on `IO_PAGE_FAULT` lines.
3. Optionally list overwrite rela patch targets from a staged multi-sym ELF + paddrs.
4. Classify each fault: `in_bo_span` / `near_bo_base` / `matches_rela_target` / **`orphan`**.
5. Soft note always carries `invent_pass=0 not_c_ready=1 not_ran_on_npu=1` and `audit_alone_not_pass=1`.

**Live-shape unit lock:** fault `0x2fff80007f40` vs DevHeap `0x4000000` / instr `0x4068000` / host UVAs ~`0x7f6b…` → **orphan** (not any BO base; not in a small near window of Dev/instr).

This does **not** create Pass. It only names the residual class for the next offline lever (PDI identity / capture / driver map), not “wait budget too short.”

---

## API names

| API | Role |
|-----|------|
| `BoDiagEntry` | Parsed bo_diag row (label, xdna_addr, vaddr, ty, handle) |
| `BoSpan` | Base + size span for classification |
| `FaultMapClass` | `InBoSpan` / `NearBoBase` / `MatchesRelaTarget` / `Orphan` |
| `FaultClassified` | One `(fault, class)` pair |
| `IoPageFaultMapAudit` | Composite result + `format_soft_note()` / `soft_residual_class_token()` |
| `parse_bo_diag_from_detail` | Detail string → `Vec<BoDiagEntry>` |
| `parse_journal_io_page_fault_addrs` | Journal text → fault `u64` list |
| `default_bo_diag_size_for_label` | Staged sizes for DevHeap/instr/ifm/wts/ofm/ctrl |
| `bo_spans_from_diag_entries` | Entries → spans (valid paddrs only) |
| `classify_fault_addr` | One fault vs spans + optional rela targets |
| `list_rela_patch_targets_overwrite` | ELF + ifm/wts/ofm/ctrl → `(sym, offset, value)` |
| `audit_io_page_fault_vs_lab_map` | Composite: detail + optional journal + optional ELF |
| `IO_PAGE_FAULT_MAP_AUDIT_CLASS` | Soft residual token `io_page_fault_vs_bo_map` |
| `BO_DIAG_DEFAULT_DEVHEAP_SIZE` | 64 MiB default DevHeap span |
| `BO_DIAG_DEFAULT_INSTR_SIZE` | 4096 B default instr span |
| `FAULT_NEAR_BO_BASE_WINDOW` | 4096 B near-base neighborhood |

**Module:** `src/backends/amd_xdna2/silicon_contract.rs` (re-exported from `amd_xdna2`).

**Soft wire:** `examples/classify_last_apply.rs` prints
`io_page_fault_map_audit=…` when a journal is present (soft only; never hard-fails gates).

**Optional micro skipped:** emit `io_paddrs ifm=…` on submit_wait Err path in timed_matmul (multi-file risk).

---

## Live-shape classification result

| Input | Value |
|-------|--------|
| DevHeap xdna_addr | `0x4000000` |
| instr xdna_addr | `0x4068000` |
| ifm/wts/ofm | host UVAs ~`0x7f6b0d…` |
| Journal fault | `0x2fff80007f40` |
| **Class** | **`orphan`** |

Tightest-span rule: instr at `0x4068000` sits inside the 64 MiB DevHeap span; when a fault lands in both, the **smallest** matching span wins so instr is not mis-labeled as DevHeap. The live fault is outside both, and outside the near window of either base.

---

## TDD

### Tests (filter)

| Test | Role |
|------|------|
| `parse_bo_diag_from_detail_live_shaped` | DevHeap/instr/ifm/wts/ofm xdna_addr parse |
| `parse_journal_io_page_fault_addrs_extracts_hex` | AMD-Vi `address=0x2fff80007f40` |
| `classify_fault_orphan_vs_in_span` | orphan / in_span / near / rela match priority |
| `list_rela_patch_targets_synthetic_multi_sym` | overwrite targets from synthetic ELF |
| `audit_io_page_fault_vs_lab_map_11bc9016_shaped_orphan` | composite + honesty trio |

### Red → green

**Named contract (red before product helpers):** live-shaped bo_diag + journal must classify fault `0x2fff80007f40` as **orphan**, never invent Pass / APP_LOAD from audit alone, and soft note must carry the honesty trio.

**Product helpers then green on the same asserts.**

```bash
cargo test --features amd-xdna2 --lib parse_bo_diag
cargo test --features amd-xdna2 --lib parse_journal_io_page_fault
cargo test --features amd-xdna2 --lib classify_fault
cargo test --features amd-xdna2 --lib list_rela_patch
cargo test --features amd-xdna2 --lib audit_io_page_fault
cargo clippy --features amd-xdna2 --lib -- -D warnings
cargo fmt -p dragon-npu
```

**Worker note:** implementer shell may be limited; parent / process mop should run the commands above and confirm exit 0. Logic walk matches every assert in the five tests (orphan lock + honesty tokens + rela list).

---

## Honesty / residual unlock

| Claim | Status |
|-------|--------|
| product Pass | **No** |
| `ran_on_npu` | **No** |
| invent_pass | **0** |
| `C_UNLOCK=11bc9016` | **CONSUMED** (do not reopen) |
| Map audit alone = Pass? | **No** (`audit_alone_not_pass=1`) |
| Next unlock | Only after **named** offline reason (PDI identity / capture / driver) + host-minted value |

---

## Next (not this slice)

- **T3c:** PDI identity for staged mladf matmul key, or real capture under `testdata/xdna2_cmd/external_known_good/`, or driver/MMU map research with new evidence.
- **T4:** new unlock + live re-shot only after enough offline reason.
- **Empty reconfirm mill:** still frozen.
