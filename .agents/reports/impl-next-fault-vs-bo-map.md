# Implement report: multi-shot IO_PAGE_FAULT vs lab BO map (Phase 1A)

**Date:** 2026-08-11
**Slice:** offline multi-shot map audit only
**Honesty:** `invent_pass=0` · **no** product Pass · **no** `ran_on_npu` · **no** unlock mint · **no** live apply
**Unlock state:** none active (`8352882b`, `efd44f35`, `2ae69036`, priors **CONSUMED**)

**Map audit ≠ Pass ≠ unlock.**

---

## Plain English

Both dual flexml stock CONFIG_CU shots that logged **IO_PAGE_FAULT** (llm-2x4x4 under `8352882b`, unified-4x4 under `efd44f35`) have **lab BO maps that look healthy and nearly identical on the device side**, and **every logged fault address is an orphan** relative to that map.

What that means in practice:

1. Userspace did create DevHeap, instruction BO, IFM/WTS/OFM/CTRL with valid handles and `valid_addr=true`.
2. Device-side bases stay at **DevHeap `0x4000000`** and **instr `0x4028000`** (same on both flexml shots).
3. Journal faults sit in **high bands** (`0x4fff8000…` for L, `0x8fff8000…` for U) that do **not** overlap DevHeap `[0x4000000, 0x8000000)`, instr’s 4 KiB page, or the host UVAs for IFM/WTS/OFM/CTRL.
4. Distance is not a near-miss: faults are tens of trillions of bytes away from every lab base under the T3b classifier (`orphan`, not `near_bo_base`, not `in_bo_span`).
5. Prior CVML fault `0x2fff80007f40` is the same class (**orphan**). Spent unified-2x4x4 `2ae69036` had **no** IO_PAGE_FAULT in window (same post-attach `wait_cmd_einval` residual class without the IOMMU signal).

**Product map/bind bug from this audit?** **No.** There is no named, fixable lab bug of the form “we forgot to pin BO X” or “instr paddr is wrong by a page.” The BO map is self-consistent; the faults do not land on lab paddrs. Residual points at **firmware / CONFIG_CU PDI / IOMMU address space** (wrong overlay or FW walk into unmapped bands that **shift with topology**), not at a missing lab BO span in `bo_diag`.

Existing pure helpers (`parse_bo_diag_from_detail`, `parse_journal_io_page_fault_addrs`, `classify_fault_addr`, `audit_io_page_fault_vs_lab_map`) already cover this. **No product code change** in this slice (multi-shot is report tables only).

---

## Inputs

| Shot | Unlock | PDI | last_apply | Live report | Journal window |
|------|--------|-----|------------|-------------|----------------|
| **L** | `8352882b` CONSUMED | flexml **llm-2x4x4** 21664 B sha16 `ba0cd738fd870011` | `/tmp/dragon_npu_last_apply_config_cu_2328515.json` | `.agents/reports/live-shot-8352882b.md` | ~10:43–10:44; log `/tmp/dragon_npu_live_shot_8352882b.log` |
| **U** | `efd44f35` CONSUMED | flexml **unified-4x4** 13792 B sha16 `7d172bf223a9b06b` | `/tmp/dragon_npu_last_apply_config_cu_2618482.json` | `.agents/reports/live-shot-efd44f35.md` | ~10:48–10:49; log `/tmp/dragon_npu_live_shot_efd44f35.log` |
| Prior A | `2ae69036` CONSUMED | flexml **unified-2x4x4** 27200 B sha16 `0eeeb70c84df056e` | (spent) | prior closeout | **no** IO_PAGE_FAULT |
| Prior CVML | `97f38307` / T3b `11bc9016` | CVML ~283840 B | T3b fixture / prior | T3b report | band ~`0x2fff8000…` |

Both L and U: `status=wait_cmd_einval`, `real_ioctl=true`, `ran_on_npu=false`, `config_cu=1`, classic aiebu ELF + rela×12, pack key still lab classic `mladfmatmulbias_prmptflat_mladf_2x4x4_flat_a16fw4acc16f_1_1152_256_128`.

---

## 1. Lab BO map (from last_apply `bo_diag`)

Classifier uses T3b defaults: DevHeap span **64 MiB**, instr **4096 B**, IFM **2304**, WTS **174080**, OFM **512**, CTRL **29664**; near window **4096 B**. Host Shmem `xdna_addr` equals host UVA on these shots.

### Shot L (`8352882b` / last_apply `…2328515`)

| Label | ty | handle | xdna_addr | vaddr | staged size used for class |
|-------|-----|--------|-----------|-------|----------------------------|
| DevHeap | DevHeap | 1 | `0x4000000` | `0x7f30ac000000` | 64 MiB → `[0x4000000, 0x8000000)` |
| instr | Dev | 3 | `0x4028000` | `0x7f30ac028000` | 4096 B (inside DevHeap span; tightest-span wins for in-span hits) |
| ifm | Shmem | 4 | `0x7f30b2261000` | same | 2304 |
| wts | Shmem | 5 | `0x7f30b2236000` | same | 174080 |
| ofm | Shmem | 6 | `0x7f30b2235000` | same | 512 |
| ctrl | Shmem | 7 | `0x7f30b222d000` | same | 29664 |

Also: instr blob content 1688 B, `bo_size=4096`, `map_offset=0xffffffffffffffff` (no-mmap Dev expected), `valid_addr=true` on all rows.

### Shot U (`efd44f35` / last_apply `…2618482`)

| Label | ty | handle | xdna_addr | vaddr | staged size used for class |
|-------|-----|--------|-----------|-------|----------------------------|
| DevHeap | DevHeap | 1 | `0x4000000` | `0x7f4db4000000` | 64 MiB → `[0x4000000, 0x8000000)` |
| instr | Dev | 3 | `0x4028000` | `0x7f4db4028000` | 4096 B |
| ifm | Shmem | 4 | `0x7f4db9a3a000` | same | 2304 |
| wts | Shmem | 5 | `0x7f4db97aa000` | same | 174080 |
| ofm | Shmem | 6 | `0x7f4db9a39000` | same | 512 |
| ctrl | Shmem | 7 | `0x7f4db9a31000` | same | 29664 |

**Map delta L vs U:** device paddrs **identical** (DevHeap/instr). Host UVAs differ (ASLR / new process). PDI size/sha differ; I/O sizes and classic pack key match.

---

## 2. Journal fault addresses (unique, shot window)

Source: AMD-Vi `IO_PAGE_FAULT` lines with `address=0x…` in live logs (preferred over report ellipsis).

### Shot L window (~10:43) — band **`0x4fff8000…`**

| # | address |
|---|---------|
| 1 | `0x4fff80007f00` |
| 2 | `0x4fff80008000` |
| 3 | `0x4fff80009000` |
| 4 | `0x4fff8000a000` |
| 5 | `0x4fff8000b000` |
| 6 | `0x4fff8000c000` |
| 7 | `0x4fff8000c800` |
| 8 | `0x4fff8000d000` |
| 9 | `0x4fff8000e000` |
| 10 | `0x4fff8000f000` |

**10 unique.** Cluster mid `0x8000`, low pages ~`0x7f00`…`0xf000`. Log notes suppressed callbacks (more traffic than 10 lines).

### Shot U window (~10:48) — band **`0x8fff8000…`**

| # | address |
|---|---------|
| 1 | `0x8fff80007f40` |
| 2 | `0x8fff80008000` |
| 3 | `0x8fff80008a40` |
| 4 | `0x8fff80009000` |
| 5 | `0x8fff80009400` |
| 6 | `0x8fff80009800` |
| 7 | `0x8fff80009c00` |
| 8 | `0x8fff8000a000` |

**8 unique** (log line for `0x8fff80009000` may repeat). Same mid pattern, **different high nibble**.

**Note:** U log also still contains **L-window residue** at 10:43 (`0x4fff8000d000`…`f000`). Classification below uses **shot windows only**, not cross-shot pollution.

### CVML / T3b lock (cite)

| Source | Sample fault | Class (T3b) |
|--------|--------------|-------------|
| `11bc9016` / CVML-shaped | `0x2fff80007f40` | **orphan** |
| Band | ~`0x2fff8000…` | orphan vs DevHeap `0x4000000` / instr ~`0x4068000` fixture |

### `2ae69036` (unified-2x4x4)

| Item | Value |
|------|--------|
| IO_PAGE_FAULT in shot window | **none** |
| status | `wait_cmd_einval` (still no Pass) |
| Implication | same post-attach WAIT EINVAL class can appear **with or without** IOMMU faults; fault band is **not** required for residual, but when present it is topology-linked |

---

## 3. Classification (T3b rules)

Priority: exact rela target → tightest in-span → near base within 4096 B → **orphan**.

Rela list not required for orphan proof: host IFM/WTS/OFM/CTRL paddrs are ~`0x7f3…` / `0x7f4…`, far from `0x*fff8000…`. Even with overwrite rela, patched values stay in those host/device bases + addend, not in the `0x2/4/8fff8000…` bands.

| Shot | Fault band | n faults classed | in_span | near_base | rela_match | **orphan** |
|------|------------|------------------|---------|-----------|------------|------------|
| L `8352882b` | `0x4fff8000…` | 10 | 0 | 0 | 0 | **10** |
| U `efd44f35` | `0x8fff8000…` | 8 | 0 | 0 | 0 | **8** |
| CVML cite | `0x2fff8000…` | 1+ | 0 | 0 | 0 | **all** |
| `2ae69036` | (none) | 0 | — | — | — | n/a |

Every individual L/U fault → **`FaultMapClass::Orphan`**.

---

## 4. Multi-shot summary table

| Unlock | Topology / PDI | pdi_bytes | sha16 | status | IO_PAGE_FAULT band | vs lab map | TXN / timeout notes |
|--------|----------------|-----------|-------|--------|--------------------|------------|---------------------|
| `2ae69036` | unified-2x4x4 | 27200 | `0eeeb70c84df056e` | wait_cmd_einval | **none** | n/a | backtrack + cmd timeout path |
| **`8352882b` (L)** | **llm-2x4x4** | **21664** | **`ba0cd738fd870011`** | wait_cmd_einval | **`0x4fff8000…`** | **all orphan** | DPU/TXN `0xffffffff`; backtrack yes |
| **`efd44f35` (U)** | **unified-4x4** | **13792** | **`7d172bf223a9b06b`** | wait_cmd_einval | **`0x8fff8000…`** | **all orphan** | TXN OP ID **`0xd`**; timeout yes; no backtrack in filtered window |
| `97f38307` / T3b | CVML research | 283840 | `a1e65d04…` | wait_cmd_einval | **`0x2fff8000…`** | **orphan** (T3b lock) | IO_PAGE_FAULT + timeout |

### Band pattern (research observation only)

| Name | Example fault | bits[47:32] | bits[31:16] | low page |
|------|---------------|-------------|-------------|----------|
| CVML | `0x2fff80007f40` | `0x2fff` | `0x8000` | `0x7f40` |
| L | `0x4fff80007f00` | `0x4fff` | `0x8000` | `0x7f00` |
| U | `0x8fff80007f40` | `0x8fff` | `0x8000` | `0x7f40` |
| Lab DevHeap | `0x4000000` | `0x0000` | `0x0400` | `0x0000` |
| Lab instr (L/U) | `0x4028000` | `0x0000` | `0x0402` | `0x8000` |

Shared **mid `0x8000`** + low pages in a small walk; **high nibble tracks CONFIG_CU PDI topology**, not lab DevHeap/instr bases. That is evidence of **overlay-dependent device address traffic outside the lab BO map**, not of a lab BO base off-by-page.

---

## 5. Product map/bind bug? (named fixable delta)

| Question | Answer |
|----------|--------|
| Is any fault `in_bo_span` of DevHeap/instr/ifm/wts/ofm/ctrl? | **No** |
| Is any fault `near_bo_base` (within 4 KiB of a lab base)? | **No** |
| Is DevHeap/instr missing or zero/`u64::MAX` on these last_apply? | **No** — bases valid and consistent |
| Does instr `map_offset=u64::MAX` explain IO_PAGE_FAULT as “forgot to mmap Dev instr”? | **No for these faults** — no-mmap on Dev instr is expected lab/XRT-style; faults are not at `0x4028000`±4 KiB |
| Fixable product code delta from this audit alone? | **No** — do not invent a lab remapping “fix” that maps lab BOs into `0x*fff8000…` without firmware/PDI evidence |
| Primary classification | **orphan** (all multi-shot faults with journals) |

**Not claimed:** root cause of FW timeout / WAIT EINVAL is “only” IO_PAGE_FAULT. `2ae69036` shows wait_cmd_einval **without** IO_PAGE_FAULT. Fault bands are a **strong secondary signal** that CONFIG_CU research PDIs drive different illegal address walks.

---

## 6. Code / TDD this slice

| Item | Action |
|------|--------|
| New product API | **None** (existing T3b surface sufficient) |
| Multi-shot compare unit | **Not added** — report tables only; no unlock/Pass token |
| Soft honesty | still `invent_pass=0 not_c_ready=1 not_ran_on_npu=1 audit_alone_not_pass=1` |
| RESIDUAL.md | **not edited** (Phase 2 join) |

Re-run lock if desired (parent / mop):

```bash
cargo test --features amd-xdna2 --lib audit_io_page_fault
cargo test --features amd-xdna2 --lib classify_fault
cargo test --features amd-xdna2 --lib parse_journal_io_page_fault
```

Optional future (not this slice): unit that freezes L-band `0x4fff80007f00` + U-band `0x8fff80007f40` against live-shaped L/U `bo_diag` as dual orphan — only if product wants a permanent multi-shot regression fixture.

---

## 7. Honesty / next

| Claim | Status |
|-------|--------|
| product Pass | **No** |
| `ran_on_npu` | **No** |
| invent_pass | **0** |
| Map audit alone = Pass / C-ready / unlock | **No** |
| Reopen CONSUMED unlocks | **Forbidden** |
| Re-live spent PDI fingerprints | **Forbidden** without new unlock + named delta |

**Agent-legal next (offline):** further structure/PDI classify; optional dual-orphan fixture; unlock **text** only if a human mints a new unlock with a **new** offline reason.
**Not legal from this pin alone:** mint unlock, live apply, invent Pass, claim map fix shipped.

---

## Artifacts

- This report: `.agents/reports/impl-next-fault-vs-bo-map.md`
- last_apply L: `/tmp/dragon_npu_last_apply_config_cu_2328515.json`
- last_apply U: `/tmp/dragon_npu_last_apply_config_cu_2618482.json`
- live logs: `/tmp/dragon_npu_live_shot_8352882b.log`, `/tmp/dragon_npu_live_shot_efd44f35.log`
- live reports: `.agents/reports/live-shot-8352882b.md`, `.agents/reports/live-shot-efd44f35.md`
- prior T3b: `.agents/reports/impl-t3b-io-page-fault-bo-map.md`
- helpers: `src/backends/amd_xdna2/silicon_contract.rs` (`parse_bo_diag_from_detail`, `parse_journal_io_page_fault_addrs`, `classify_fault_addr`, `audit_io_page_fault_vs_lab_map`)
