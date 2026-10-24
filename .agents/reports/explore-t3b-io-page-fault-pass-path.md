# Explore report: T3b offline delta after WAIT EINVAL + IO_PAGE_FAULT

**Date:** 2026-08-10
**Mode:** Read-only codebase exploration (report only; no product edits).
**Context:** T3 classifier honesty landed (`wait_cmd_einval`). Unlock `C_UNLOCK=11bc9016` **CONSUMED**.
**Honesty:** invent_pass=0 · **no** product Pass · **no** `ran_on_npu` · unlock ≠ Pass · classifier honesty ≠ Pass

---

## Plain English summary

The stock live shot under `11bc9016` got past CONFIG_CU (PDI 283840 B), aiebu ELF + rela×12, and EXEC_CMD, then **WAIT_CMD returned EINVAL immediately** (`submit_wait_s≈0`). Journal shows **IO_PAGE_FAULT** at addresses like `0x2fff80007f40` plus **aie2_set_cmd_timeout** (DPU PC `0xffffffff`, TXN OP ID `0xd`).

T3 fixed the **status label**. Residual silicon is still **MMU / bad reloc / wrong map / wrong overlay**, not “wait budget too short.”

**Highest-value agent offline delta (T3b):** pure unit-testable **IO_PAGE_FAULT address vs last_apply BO map + rela patch targets** audit. That proves the next root-cause class (orphan fault vs in-BO span vs matches a patched reloc) without inventing Pass and without a new unlock.

---

## Investigation answers (1–5)

### 1. How rela paddrs are chosen and written into ctrltext

| Piece | Fact | Path |
|-------|------|------|
| I/O bases | Shmem CREATE_BO → `pin_bo_for_device_access` → `xdna_addr` (PASID UVA for Shmem; Dev heap addr when no mmap) | `lab_io.rs` `create_io_bos` ~1191–1288; `direct.rs` `pin_bo_for_device_access` ~2507–2525 |
| Validity gate | Only refuses `0` and `u64::MAX` | `is_valid_xdna_device_addr` (`direct.rs` ~757–759) |
| Live wire | `patch_aiebu_elf_ctrltext_with_bos_ex(elf, ifm, wts, ofm, ctrl, rela_mode)` then write peeled ctrltext into **Dev** instr BO | `direct.rs` ~5196–5207 |
| Sym map (lab) | `bases = [0, ifm, wts, ofm, ctrl]`; **sym 1→IFM, 2→WTS, 3→OFM, 4→CTRL** | `txn.rs` ~1040–1101 |
| Staged corpus | 12 relas: **8×sym1 IFM, 2×sym2 WTS, 2×sym3 OFM**; **no sym4** (CTRL is meta/packet path, not rela) | tests `txn.rs` ~1162–1163; `docs/XDNA2_XRT_CONTRACT_MLADF.md` |
| Default mode | **overwrite** `base + addend` u64 LE (XRT-style). Trial `add` RMW is residual-only | `LabRelaMode` |
| Dynsym | Names all opaque `"6"`; **do not invent name→role map** | `parse_elf32_dynsym` docs |
| Post-patch gate | Classic walk 1688 B / 40 ops before EXEC | `validate_patched_ctrltext_classic` |

**Off-by-one / wrong-sym risks (code facts, not proven live root cause):**

| Risk | Code behavior | Live likelihood |
|------|----------------|-----------------|
| Sym index OOB | `bases.get(sym).unwrap_or(0)` → patch **addend only** (base 0) | Staged has only 1–3; unit-locked |
| Sym0 | base 0 | Not in staged 12 |
| Swap IFM/WTS/OFM | Index map is hard-coded order; wrong only if XRT uses different index roles | Historical C2 used same map + host UVAs; still APP_LOAD then, not page-fault class |
| Missing CTRL rela | Staged intentional (no sym4); CTRL via `patch_ctrl_from_meta_io_paddrs` size=6 | Separate from 12 rela sites |
| Host-looking UVAs “wrong” | **Not automatic bug** — pin path is Shmem PASID; C2 lock used `0x7fda…` UVAs | Same class as 11bc9016 IFM/WTS/OFM ~`0x7f6b…` |

**Fault address class:** journal `0x2fff80007f40` is **neither** DevHeap/instr low range (`0x40xxxxx`) **nor** high host PASID UVAs (`0x7f…`). That strongly suggests **orphan / unmapped / tile-internal / corrupted target**, not “IFM base equals fault.”

Userspace rela algorithm itself is already TDD-locked (overwrite, multi-sym, negative addend, staged 12). Re-deriving the patch formula alone is **low value** without comparing **fault** to **targets**.

### 2. CONFIG_CU PDI source: known-wrong for this matmul key?

| Fact | Detail |
|------|--------|
| Dump tool | `tools/dump_xclbin_pdi.sh` |
| Default xclbins | RyzenAI **CVML AIE2P overlay** paths (`AMD_AIE2P_4x4_Overlay*.xclbin`), **not** mladf matmul xclbin |
| Script honesty | Header: “CVML / AIE2P overlay PDI ≠ mladf matmul known-good corpus”; dump ≠ Pass |
| Live measured | `config_cu=1`, `pdi_bytes=283840`, `cu_func=0` under 11bc9016 |
| Residual implication | CONFIG_CU **machinery works**; overlay may still be **wrong partition image** for staged `mladfmatmulbias_…_1_1152_256_128` |

**Offline:** more docs that “PDI might be wrong” do **not** move residual. Proving right PDI needs a real matmul-matching xclbin/PDI capture (human / external), not another agent reconfirm of 283840 B smoke dump.

Treat wrong-PDI as **open hypothesis** consistent with page-fault / FW timeout **after** attach, but **not** the single T3b code deliverable (no matmul PDI bytes in tree).

### 3. Can EXEC_CMD succeed while WAIT fails EINVAL after FW faulted?

**Yes — by construction.**

```text
submit_and_wait_command:
  seq = submit_command(...)   // EXEC_CMD
  wait_command(..., seq, timeout_ms)  // WAIT_CMD
```

(`direct.rs` ~3781–3783)

Live shape:

| Signal | Meaning |
|--------|---------|
| `submit_wait_s≈0` | Wait ioctl returned **immediately** (not 5s ETIMEDOUT) |
| `total_s≈63` | Setup (hwctx + CONFIG_CU 283840 write + BO fill) dominated wall time |
| T3 status | `wait_cmd_einval` (after fix); was false `wait_timeout` |
| Journal IO_PAGE_FAULT + aie2_set_cmd_timeout | FW/MMU aborted job; WAIT can then EINVAL on dead/aborted seq |

**Ordering residual class:** EXEC was **accepted enough** to schedule work and fault; WAIT is a **secondary** userspace symptom. Bumping `XDNA2_WAIT_TIMEOUT_MS` will **not** Pass this pin.

Historical contrast: 2026-07-18 finished **WAIT OK + ERT Error** (APP_LOAD). This pin is **WAIT EINVAL + journal IO fault** — different class; do not invent APP_LOAD from 11bc9016 alone.

### 4. Public XRT/DD BO map path in-repo

| Source | What it shows |
|--------|----------------|
| `docs/CONSTRUCTION_MAP_APP_LOAD.md` | Public order: xclbin PDI → CONFIG_CU → aiebu peel/patch → START_NPU; lab optional CONFIG_CU |
| `docs/XDNA2_XRT_CONTRACT_MLADF.md` | Shim `elf_io` / `exec_buf.h` contract: Dev instr, Shmem I/O, rela 1/2/3, prop_count=0 |
| Local trees (cited, not vendored) | `~/Projects/kernel/xdna-driver/.../shim/kmq/hwctx.cpp`, `test/shim_test/io.cpp` |
| In-repo offline | `audit_aiebu_rela_with_live_paddrs` + `c2_live_io_paddrs_locked()` (2026-07-18 paddrs) |

**No** checked-in working-stack post-patch ctrltext under `testdata/xdna2_cmd/external_known_good/` (PROVENANCE-only). So agent cannot byte-diff “known good map” offline; can only audit **our** last_apply + journal.

### 5. Pure offline intermediate: last_apply bo_diag + journal fault compare

**Gap today:**

| Exists | Missing |
|--------|---------|
| `collect_lab_bo_diag` / `format_bo_info_diag` on **submit_wait Err** (always) | Pure **parser** of `bo_diag=[...]` into structured bases |
| `parse_io_paddrs_from_detail` (success-path `io_paddrs ifm=…`) | Err path often has **bo_diag only**, not always `io_paddrs` token |
| `classify_journal_soft_signals` (`io_page_fault=bool`) | **No** fault **address** extract; no compare to BO spans / rela targets |
| `audit_aiebu_rela_with_live_paddrs` (walk OK) | Does not list targets or match journal faults |
| Journal soft class `incomplete_timeout_or_journal` | Coarse; does not discriminate orphan vs in-span |

**Err-path detail shape** (`direct.rs` ~5816–5833):

```text
… submit_and_wait failed ({st}): {e}. {ofm_note}; bo_diag=[{bo_diag}]
```

`bo_diag` entries look like:

```text
DevHeap: handle=… ty=DevHeap map_offset=… xdna_addr=0x4000000 vaddr=… valid_addr=…
instr: … xdna_addr=0x4068000 …
ifm/wts/ofm/ctrl: … ty=Shmem xdna_addr=0x7f6b… …
```

Live fault `0x2fff80007f40` is the right offline probe against those bases.

---

## Recommended single T3b delta

### Title

**T3b — Offline IO_PAGE_FAULT vs BO map + rela patch-target audit (pure TDD)**

### Goal

Given (a) last_apply `detail` with `bo_diag` / optional `io_paddrs`, (b) optional journal paste with `IO_PAGE_FAULT` hex addresses, and (c) optional staged aiebu ELF + paddrs: classify each fault address as:

| Class | Meaning |
|-------|---------|
| `in_bo_span` | Within known base + size for a labeled BO (ifm/wts/ofm/ctrl/instr/devheap) |
| `near_bo_base` | Within small page neighborhood of a base (size unknown) |
| `matches_rela_target` | Equals some `base[sym]+addend` after overwrite patch list |
| `orphan` | None of the above (11bc9016-shaped expectation for `0x2fff80007f40`) |

Always soft honesty: `invent_pass=0 not_c_ready=1 not_ran_on_npu=1 product_pass_requires_live_decide=1`.
**Never** invent APP_LOAD or Pass from this audit alone.

### Why this is the single highest-value agent offline move

| Candidate | Moves residual understanding? | Pass by itself? | Agent offline? |
|-----------|------------------------------|-----------------|----------------|
| **Fault vs map + rela targets (this T3b)** | **Yes — names next root-cause class** | No | Pure TDD |
| Re-spin rela overwrite algorithm | Low (already unit-locked) | No | Redundant |
| Bump WAIT / outer timeout | **No** (`submit_wait_s≈0`) | No | Harmful distraction |
| Docs that “PDI may be wrong” | Already stated in dump script | No | Empty prose |
| Switch all I/O to Dev | Speculative HITL risk | Unproven | Needs new unlock |
| Capture compare without bytes | Still blocked | No | Frozen empty loop |
| T3 classifier (done) | Honesty only | No | Landed |

If audit locks **orphan + no rela target match**, next lever is **FW/PDI/MMU/tile map** (or START_NPU field encoding), not “fix sym index 1/2/3.”
If audit locks **matches_rela_target** for a specific sym, next lever is **that buffer’s pin/map/size**.
Either outcome is more valuable than another empty gate reconfirm.

### Files to touch (implementer)

| File | Change |
|------|--------|
| `src/backends/amd_xdna2/silicon_contract.rs` (preferred home; pure residual) | New types + pure parsers/classifiers (below) |
| Optional re-export | `src/backends/amd_xdna2.rs` public surface for classify example |
| Optional soft wire | `examples/classify_last_apply.rs` print audit when journal/env detail present — **never** hard-fail gates into Pass |
| Optional product micro (same slice or immediate follow) | On submit_wait **Err**, also append `io_paddrs ifm=…` from `IoBoSet` (parity with Ok path) so future pins always carry both tokens — **does not create Pass** |

**Do not** need: new unlock, L3, ALLOW=1, capture invent, PDI decode.

### API sketch (names may be refined; contracts fixed)

```rust
// Parse GET_BO_INFO-style lines from bo_diag=[...]
struct BoDiagEntry { label: String, xdna_addr: u64, vaddr: Option<u64>, ty: Option<String> }
fn parse_bo_diag_from_detail(detail: &str) -> Vec<BoDiagEntry>;

// Journal: extract hex fault addresses near IO_PAGE_FAULT lines
fn parse_journal_io_page_fault_addrs(text: &str) -> Vec<u64>;

// Staged size defaults when size not in detail (mladf contract)
// IFM 2304, WTS 174080, OFM 512, CTRL 29664, instr content 1688 (page 4096 optional)
struct BoSpan { label: &'static str, base: u64, size: u64 }

// Classification
enum FaultMapClass { InBoSpan { label }, NearBoBase { label }, MatchesRelaTarget { sym, offset }, Orphan }
fn classify_fault_addr(fault: u64, spans: &[BoSpan], rela_targets: &[(u32, u32, u64)]) -> FaultMapClass;

// List overwrite targets: (sym, ctrltext_offset, value = base+addend)
fn list_rela_patch_targets(elf: &[u8], ifm: u64, wts: u64, ofm: u64, ctrl: u64)
  -> Result<Vec<(u32 /*sym*/, u32 /*off*/, u64 /*value*/)>, …>;

// Composite research report
struct IoPageFaultMapAudit { /* per-fault classes + honesty tokens */ }
fn audit_io_page_fault_vs_lab_map(
  detail: &str,
  journal: Option<&str>,
  elf: Option<&[u8]>,  // empty → skip rela target match
) -> IoPageFaultMapAudit;
// format_soft_note always invent_pass=0 not_c_ready=1 not_ran_on_npu=1
```

Reuse: `parse_io_paddrs_from_detail`, `is_valid_device_paddr` / `is_valid_xdna_device_addr`, `parse_elf32_rela_dyn`, `patch` math (or call `list` without mutating), `classify_journal_soft_signals` (boolean), staged size constants from `mladf_pack` / contract.

### TDD red tests (suggested)

1. **`parse_bo_diag_from_detail_live_shaped`**
   Input: `bo_diag=[DevHeap: … xdna_addr=0x4000000 …; instr: … xdna_addr=0x4068000 …; ifm: … xdna_addr=0x7f6b0d000000 …]`
   Assert: three labels + exact hex bases.

2. **`parse_journal_io_page_fault_addrs_extracts_hex`**
   Input with `IO_PAGE_FAULT` / `io_page_fault` and `0x2fff80007f40`
   Assert: vec contains that u64; empty text → empty vec.

3. **`classify_fault_orphan_vs_in_span`**
   - fault `0x2fff80007f40` + bases DevHeap/instr/host UVAs + staged sizes → **`Orphan`**
   - fault `0x7f6b0d000010` with ifm base `0x7f6b0d000000` size 2304 → **`InBoSpan{ifm}`**
   - fault `0x4068000` with instr base same size 1688/4096 → **`InBoSpan{instr}`** or near

4. **`list_rela_patch_targets_synthetic_multi_sym`** (asset-free ELF via `build_minimal_test_elf32`)
   Assert values match overwrite formula for sym 1–4.

5. **`audit_io_page_fault_vs_lab_map_11bc9016_shaped_orphan`**
   Composite fixture: detail bo_diag + journal fault `0x2fff80007f40` + synthetic or staged ELF if present
   Assert: class orphan (or not matches_rela for synthetic paddrs that cannot produce 0x2fff…); soft note honesty trio; **never** PassCandidate / locked APP_LOAD class.

6. **(optional product)** **`submit_wait_err_detail_includes_io_paddrs`** — pure format helper or stub path so Err detail contains `io_paddrs ifm=` when IoBoSet present (if that micro lands in same slice).

**Red order:** write tests first; observe missing symbols / fail; land pure helpers; green same filters. No live EXEC in tests.

**Commands (implementer):**

```bash
cargo test --features amd-xdna2 --lib io_page_fault
# and/or:
cargo test --features amd-xdna2 --lib parse_bo_diag
cargo test --features amd-xdna2 --lib classify_fault
cargo clippy -p dragon-npu --features amd-xdna2 --lib -- -D warnings
cargo fmt -p dragon-npu
```

### Acceptance (offline)

- Live-shaped bo_diag + journal fault parses without inventing paddrs.
- `0x2fff80007f40` vs 11bc9016-shaped bases classifies **orphan** (unit lock).
- Soft honesty trio always on.
- **No** Pass / **no** `ran_on_npu` / **no** new unlock / **no** ALLOW=1 / **no** L3 / **no** invent capture bytes.

---

## What will NOT get Pass by itself

| Item | Why not Pass |
|------|----------------|
| This T3b audit green offline | Research classification only; product Pass still needs live ERT Completed + OFM allclose + `decide_ran_on_npu` |
| T3 `wait_cmd_einval` | Honesty; residual still MMU/FW |
| CONFIG_CU attach (`config_cu=1`) | Measured machinery; overlay may be wrong |
| aiebu+rela×12 userspace walk | Already green historically with APP_LOAD; now green with page-fault class |
| CVML 283840 B PDI dump | Explicitly not matmul known-good |
| Raising WAIT to 120s | Live wait was immediate EINVAL |
| Empty external known-good dir reconfirm | Frozen; still no bytes |
| Second unlock without offline delta | Process ban; T3b **is** the named delta that can justify a later T4 unlock **only if** human chooses — unlock still ≠ Pass |

---

## Explicit non-goals

- Do **not** invent product Pass or `ran_on_npu`.
- Do **not** reopen `C_UNLOCK=11bc9016` / `3a70fabf` / `b17b0119`.
- Do **not** invent capture under `testdata/xdna2_cmd/external_known_good/`.
- Do **not** claim IO_PAGE_FAULT fixed by parsing alone.
- Do **not** re-open count4 vs xrt3 env matrix as the main deliverable.
- Do **not** empty-reconfirm offline gates without this named code delta.
- Do **not** “fix” rela by swapping Shmem→Dev without evidence from the audit.

---

## Confidence

| Claim | Confidence |
|-------|------------|
| T3 false `wait_timeout` is fixed offline; residual is wait EINVAL + journal IO fault class | **High** (code + live reports) |
| EXEC accepted then WAIT failed after FW fault (ordering) | **High** (call order + `submit_wait_s≈0`) |
| Fault `0x2fff…` is not equal to reported DevHeap/instr/host BO bases | **High** from operator address classes; unit lock will make it permanent |
| Host UVAs in rela are not automatically wrong | **High** (pin path + C2 historical) |
| PDI is research overlay, not proven matmul-matched | **High** (dump tool + docs) |
| Orphan fault ⇒ next lever is PDI/MMU/FW map not rela index 1/2/3 | **Medium** (best hypothesis; T3b is designed to **confirm or refute**) |
| Wrong-PDI is sole root cause of 0x2fff fault | **Low–medium** (plausible; not proven offline) |
| T3b alone yields Pass | **None** — by design research only |

**Overall confidence that T3b is the right single offline next step:** **high** for residual science and unlock justification hygiene; **zero** that it equals silicon Pass.

---

## Paths

| Artifact | Path |
|----------|------|
| This report | `.agents/reports/explore-t3b-io-page-fault-pass-path.md` |
| Prior explore (T3) | `.agents/reports/explore-t3-wait-io-page-fault.md` |
| T3 implement | `.agents/reports/impl-t3-wait-cmd-einval-classify.md` |
| Live pin | `.agents/reports/live-shot-11bc9016.md` |
| Residual SSoT | `RESIDUAL.md` top plain status |
| Rela patch | `src/backends/amd_xdna2/txn.rs` |
| Timed path / bo_diag Err | `src/backends/amd_xdna2/direct.rs` |
| Soft journal (bool only) | `src/backends/amd_xdna2/silicon_contract.rs` |
| PDI dump | `tools/dump_xclbin_pdi.sh` |

---

## Who acts next

| Who | Next |
|-----|------|
| **Agent implementer** | Land T3b pure audit + unit tests (this report). Offline only. |
| **Human** | Optional: matmul-matched xclbin/PDI or external known-good ctrltext when available. No stock sandwich under consumed unlock. |
| **Later live (T4)** | Only after T3b lands + **new** host-minted unlock + real offline reason from audit outcome. Still invent_pass=0 until OFM allclose. |
