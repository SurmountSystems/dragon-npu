# Plan explore: driver / MMU / orphan IO_PAGE_FAULT residual path

**Date:** 2026-08-10
**Mode:** Read-only codebase exploration (report only; **no** product edits).
**Repo:** `/home/hunter/Projects/ai/dragon-npu`
**Honesty:** invent_pass=0 · **no** product Pass · **no** `ran_on_npu` · all unlocks for this class **CONSUMED**

---

## Plain English summary

Two stock live CONFIG_CU shots (`11bc9016`, then research-blob leg under `97f38307`) reached measured CONFIG_CU (PDI 283840 B), aiebu ELF + rela×12, and EXEC, then:

1. Product status **`wait_cmd_einval`** (WAIT_CMD **EINVAL** immediately; not a true wait timeout).
2. Journal **IO_PAGE_FAULT** near **`0x2fff8000…`** (e.g. `0x2fff80007f40`, `0x2fff80007f00`, …).
3. Journal **`aie2_set_cmd_timeout`** (DPU PC `0xffffffff`, TXN OP ID `0xd`).

Offline **T3b** already proved the fault is an **orphan** versus every labeled lab BO base (DevHeap `0x4000000`, instr `0x4068000`, IFM/WTS/OFM/CTRL host PASID UVAs ~`0x7f…`). Userspace rela algorithm and BO map diagnostics are **not** the open “wait budget” bug; that honesty work is **landed**.

**Ranking (this explore):** pure further driver/MMU product implement **without new evidence is low value now**. Park that path until **matmul-matched PDI**, **real capture bytes**, or a **named** journal/driver delta with a new unlock. Highest remaining levers are external/human material, not another offline reconfirm of orphan.

---

## Locked evidence (do not re-litigate)

| Pin | Unlock | Status | CONFIG_CU | Journal class |
|-----|--------|--------|-----------|---------------|
| 2026-08-10 stock | `11bc9016` **CONSUMED** | was false `wait_timeout` → T3 → `wait_cmd_einval` | `config_cu=1`, `pdi_bytes=283840` | IO_PAGE_FAULT `0x2fff80007f40` + `aie2_set_cmd_timeout` |
| 2026-08-10 research blob | `97f38307` **CONSUMED** | `wait_cmd_einval` | same size; PDI sha16 `a1e65d04add3422c` | same class ~`0x2fff80007f00…` |
| 2026-07-18 | `b17b0119` **CONSUMED** | WAIT ok + ERT Error | (no research CONFIG_CU of this shape) | **APP_LOAD** `0x3000003` (different class) |
| 2026-08-09 | `3a70fabf` **CONSUMED** | outer 30s kill | incomplete | timeout soft class |

| Claim | Fact |
|-------|------|
| Map audit alone = Pass? | **No** (`audit_alone_not_pass=1`) |
| Research CVML/AIE2P PDI = product matmul known-good? | **No** (T3c) |
| Bump `XDNA2_WAIT_TIMEOUT_MS`? | **No** (`submit_wait_s≈0`) |
| Reopen CONSUMED unlocks? | **No** |
| Empty reconfirm mill? | **Frozen** |

Reports:
`.agents/reports/live-shot-97f38307.md`,
`.agents/reports/live-shot-11bc9016.md`,
`.agents/reports/impl-t3b-io-page-fault-bo-map.md`,
`.agents/reports/impl-t3c-pdi-identity.md`,
`.agents/reports/impl-t3-wait-cmd-einval-classify.md`,
`.agents/reports/explore-t3b-io-page-fault-pass-path.md`.

---

## 1. What already landed (T3b audit surface)

**Module:** `src/backends/amd_xdna2/silicon_contract.rs` (re-exported from `amd_xdna2`).

| API | Role |
|-----|------|
| `parse_bo_diag_from_detail` | last_apply `bo_diag=[label: … xdna_addr=…; …]` → `BoDiagEntry` |
| `bo_spans_from_diag_entries` / `default_bo_diag_size_for_label` | Spans: DevHeap 64 MiB, instr 4096, IFM/WTS/OFM/CTRL staged sizes |
| `parse_journal_io_page_fault_addrs` | AMD-Vi `address=0x…` on `IO_PAGE_FAULT` lines |
| `classify_fault_addr` | Priority: exact rela target → tightest in-span → near base (4 KiB window) → **orphan** |
| `list_rela_patch_targets_overwrite` | ELF + ifm/wts/ofm/ctrl → `(sym, offset, value=base+addend)` |
| `audit_io_page_fault_vs_lab_map` | Composite + soft note always honesty trio |
| `IO_PAGE_FAULT_MAP_AUDIT_CLASS` | Soft token `io_page_fault_vs_bo_map` |
| Soft journal tax | `classify_journal_soft_signals` (`io_page_fault`, `aie2_set_cmd_timeout`, outer kill) |

**Soft wire:** `examples/classify_last_apply.rs` prints `io_page_fault_map_audit=…` when journal present (never hard-fails into Pass).

**Unit lock:** fault `0x2fff80007f40` vs DevHeap/instr/host UVAs → **`FaultMapClass::Orphan`**.

**What this proves:** residual is **not** “fault equals IFM base” or “wait ran out.” Next levers are **wrong overlay / tile map / unmapped DMA target / START_NPU field encoding / FW partition**, not re-deriving overwrite rela 1/2/3.

---

## 2. Rela patch + BO paddr paths (userspace facts)

| Piece | Behavior | Path |
|-------|----------|------|
| I/O BO type | **Shmem** + pin for PASID UVA | `lab_io` create path; `pin_bo_for_device_access` |
| Instr BO | **Dev** (heap carve); not Shmem | Phase D2 contract |
| DevHeap | 64 MiB, **UVA 64 MiB-aligned** for map_host | `mmap_dev_heap_drm_aligned`; D3c closed map_host `0x4000003` class |
| PASID pin | Keep mmap after write so HMM/PASID UVA stays registered | `PinnedShmemMaps` in `direct.rs` (~949–981) |
| Rela sym map | `bases=[0, ifm, wts, ofm, ctrl]`; staged 12: 8×IFM, 2×WTS, 2×OFM; **no CTRL rela** | `txn.rs` / `list_rela_patch_targets_overwrite` |
| Mode | Default **overwrite** `base+addend` u64 LE | `LabRelaMode` |
| Validity gate | Refuse only `0` and `u64::MAX` | `is_valid_xdna_device_addr` |
| Live wire | Patch ELF ctrltext → write into Dev instr BO → START_NPU | `direct.rs` timed silicon path |
| Err detail | Always appends `bo_diag=[…]` on submit_wait Err | `collect_lab_bo_diag` |
| Ok detail | Often also `io_paddrs ifm=…` | Success path ~5640 |
| Optional gap | Err path may lack `io_paddrs ifm=…` token (bo_diag still present) | T3b “optional micro” still open |

**Important:** Host-looking IFM/WTS/OFM UVAs (`0x7f…`) are **expected** for Shmem PASID, not automatic root cause. Historical C2 (2026-07-18) used the same class of UVAs and hit **APP_LOAD**, not orphan page-fault. Orphan `0x2fff…` is a **different residual class**.

---

## 3. In-repo notes on IO_PAGE_FAULT / PASID / map_host / `0x2fff`

| Topic | Where | Status vs this residual |
|-------|--------|-------------------------|
| map_host / MSG_OP `0x106` / unaligned DevHeap | `docs/SILICON_PATH_LAB.md` Phase D; `docs/XDNA2_XRT_CONTRACT_MLADF.md` | **Closed offline** (D3c). Live pins show DevHeap `0x4000000` class, not this failure. |
| SHMEM PASID pin | `direct.rs` `PinnedShmemMaps`; SILICON_PATH_LAB D2b | Landed mitigation for unmap→invalid UVA. |
| IO_PAGE_FAULT soft tax | `silicon_contract` `JournalSoftTaxonomy` | Bool presence only (pre-T3b). |
| Orphan vs map | T3b + live reports | **Locked orphan** for `0x2fff8000…`. |
| Meaning of `0x2fff…` | **No** in-repo decode of tile/IOMMU space for this range | Open hypothesis only: unmapped / tile-internal / corrupted target / wrong overlay DMA. |
| TXN OP `0xd` + DPU PC `0xffffffff` | Live journals; soft tax notes | Correlated FW abort; **no** pure product decode of OP `0xd` as Pass lever. |
| Construction path (CONFIG_CU then START_NPU) | `docs/CONSTRUCTION_MAP_APP_LOAD.md` | Lab now can CONFIG_CU; attach measured ≠ Pass. |
| Kernel L2/L3 patches | `kernel/patches/0001…`, `0002…` | Null-guard + open fail-fast + **exec_cmd_allow**. **Not** MMU orphan fix. |
| DKMS stage tree | `kernel/dkms/staging/.../amdxdna_iommu.c` etc. | Upstream-shaped stage for hardened install; **not** a product fix for `0x2fff` orphan. |
| Host | Stock `exec_cmd_allow=missing` still allows stock live path used for T4 | Hardened L3 gate is separate lab story. |

**There is no checked-in “fix orphan IO_PAGE_FAULT” driver patch and no known-good post-patch ctrltext under `testdata/xdna2_cmd/external_known_good/`.**

---

## 4. Agent-doable offline still remaining (non-empty only)

Only deltas that **name new residual science** or **wire existing evidence**. Do **not** re-run gates solely to reconfirm orphan.

| # | Candidate | Moves residual? | Needs live/unlock? | Rank |
|---|-----------|-----------------|--------------------|------|
| A | **Matmul-matched PDI discovery** (path/recipe to real mladf xclbin PDI; fingerprint vs research `a1e65d…`) | Yes (wrong-overlay hypothesis) | Offline if bytes exist on host; live only after new unlock | **High** if bytes/source found |
| B | **Real capture** under `testdata/xdna2_cmd/external_known_good/` + soft compare (helpers already exist) | Yes (ctrltext/map known-good) | Human place bytes; agent compare offline | **High** when bytes appear |
| C | Err-path also emit `io_paddrs ifm=…` on submit_wait Err (parity with Ok) | Small evidence hygiene | No unlock | **Low–medium** (nice, not root cause) |
| D | Pure research: public docs / staged driver comments for what TXN OP `0xd` / DPU PC `ffffffff` mean | Marginal | Offline read | **Low** unless it names a userspace field fix |
| E | Skip-PDI A/B live (no CONFIG_CU) | Discriminates attach-vs-map | **New** unlock + named reason | **Medium** live experiment only |
| F | Re-spin rela overwrite / wait timeout / empty gates | **No** | No | **Forbidden** empty mill |
| G | Invent Pass / invent capture / reopen CONSUMED unlock | **No** | — | **Banned** |
| H | Product “driver MMU fix” without new evidence | Speculative | Would need host kernel HITL | **Park** |

**Already done offline (do not redo as “progress”):** T3 wait_cmd_einval, T3b orphan map, T3c research PDI identity + skip tooling, T4 live research-blob reproduction.

---

## 5. What needs new journal / hardened driver / external capture

| Need | Why | Who |
|------|-----|-----|
| **Matmul-matched PDI** (or proof no partition needed for this key) | Research CVML/AIE2P overlay attached cleanly then orphan fault; wrong image remains leading hypothesis | Human dump from working stack / product xclbin; agent fingerprints offline |
| **Working-stack capture** (ctrltext / instr / BO map after success) | Orphan address cannot be matched to a public good map without bytes | Human under `testdata/xdna2_cmd/external_known_good/` |
| **Richer journal on next live** | Domain, flags, device id, multiple fault addrs, order vs CONFIG_CU, PASID, dmesg window tied to last_apply | Operator collect (`collect_post_c2_evidence` / journal paste) under **new** unlock |
| **Skip-PDI A/B live** | Isolates “CONFIG_CU wrong overlay” vs “START_NPU/map alone” | New unlock only |
| **Hardened driver + dyndbg** | If residual is kernel HMM/PASID/IOMMU registration gap (not wrong image) | Host/kernel: load hardened module, enable relevant dyndbg, re-shot |
| **Public XRT/DD success path on same host** | Proves device+driver can map correctly for matmul key | Operator Ryzen AI / XRT run; agent does not invent |

Without at least one of the **first three**, another pure product implement that only reclassifies the same orphan is empty reconfirm.

---

## 6. Driver / MMU work: in-repo product vs host/kernel

| Layer | In this repo? | Agent product default? |
|-------|---------------|------------------------|
| Userspace BO map + rela + map audit | **Yes** (`silicon_contract`, `direct`, lab tools) | **Done** for orphan classification |
| L2 cmdlist null-guards, L3 open fail-fast / `exec_cmd_allow` | **Yes** as patches under `kernel/patches/` | Maintenance / install path only; **not** orphan residual fix |
| Full amdxdna DKMS stage + `amdxdna_iommu.c` | Staged under `kernel/dkms/staging/` | **Host install / rebuild** (sudo, reboot risk); outside default offline product |
| Rescue kmod / PCI recover | `kernel/amdxdna_rescue/` | Operator recovery tooling, not Pass path |
| Fix IOMMU orphan `0x2fff…` at source | **No** known patch | **Host/kernel outside product** unless a **named** upstream commit is identified and staged as a **new** L2-style patch with dry-run evidence |

**Rule of thumb:**
- **Agent product offline** = pure residual science, capture compare, PDI identity, honesty classifiers.
- **Host/kernel** = module rebuild, dyndbg, IOMMU policy, hardened install, live under new unlock.
- Do **not** treat inventing a speculative MMU patch as the next `/implement` without a named public root cause and offline dry-run against a pinned driver SHA.

---

## Honest ranking: high value now or park?

### Verdict

| Path | Rank now | Why |
|------|----------|-----|
| **Capture bytes + soft compare** | **Highest** when material exists | Closes construction/map residual with evidence |
| **Matmul-matched PDI source** | **Highest** open hypothesis after orphan lock | CONFIG_CU machinery works; overlay likely wrong for staged matmul key |
| **Skip-PDI live A/B** | **Medium**, gated | Needs **new** host-minted unlock + experiment note |
| **Err-path `io_paddrs` micro** | **Low–medium** | Hygiene only |
| **Pure driver/MMU product implement (orphan)** | **Park** | Map class already named; no agent-local fix without external evidence or host kernel work |
| **Empty reconfirm / reopen CONSUMED unlock** | **Stop** | Rules §6–9 / RESIDUAL hard stop |

**Bottom line:** After T3 / T3b / T3c / T4, the orphan IO_PAGE_FAULT class is **understood enough offline**. Further **agent** value is **not** another map-audit layer. Value is **new material** (capture or matmul PDI) or a **deliberate live A/B** under a **new** unlock. Driver/MMU deeper work is **host/kernel-side** until a named public fix appears.

### Suggested next (for coordinator / operator, not auto-implement)

1. **Human:** place real working-stack capture under `testdata/xdna2_cmd/external_known_good/` **or** produce matmul-matched PDI (not CVML research dump).
2. **Agent (only then):** offline fingerprint/compare; optional Err-path `io_paddrs` micro if still useful.
3. **Live again:** only with **new** unlock + named offline reason (e.g. skip-PDI A/B, or matched PDI stock shot).
4. **If no new material:** pin blocker once (T6), **stop** empty implement. Do not mint unlock to reconfirm orphan.

---

## Residual honesty checklist (unchanged)

- invent_pass = **0**
- product Pass = **No**
- `ran_on_npu` = **No**
- `C_UNLOCK=97f38307` / `11bc9016` / `3a70fabf` / `b17b0119` = **CONSUMED**
- Map audit ≠ Pass
- Research PDI identity ≠ Pass
- Research blob live shot ≠ Pass

---

## Key absolute paths

| Role | Path |
|------|------|
| Audit + tests | `/home/hunter/Projects/ai/dragon-npu/src/backends/amd_xdna2/silicon_contract.rs` |
| PASID pin / wait classify / bo_diag | `/home/hunter/Projects/ai/dragon-npu/src/backends/amd_xdna2/direct.rs` |
| Soft classify example | `/home/hunter/Projects/ai/dragon-npu/examples/classify_last_apply.rs` |
| SSoT residual | `/home/hunter/Projects/ai/dragon-npu/RESIDUAL.md` |
| Capture dir (bytes still absent) | `/home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/external_known_good/` |
| Kernel patches (not MMU orphan fix) | `/home/hunter/Projects/ai/dragon-npu/kernel/patches/` |
| This report | `/home/hunter/Projects/ai/dragon-npu/.agents/reports/plan-explore-mmu-orphan-unblock.md` |
