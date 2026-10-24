# Plan: Route A matmul material + Route B structure tools (parallel)

## Context

Empty capture is **solved**. The bag has real npu2 public material from mlir-aie `vector_scalar_add` (300 B ctrltext, sha16 `aaeb5b68cc298a45`, siblings xclbin/pdi/elf). Soft-compare correctly reports mismatch vs lab classic **1688 B**. Residual tools can ingest/classify; `HUMAN_NEEDED=0` for the empty-bag case.

**Remaining gap is narrower:** material is real but **not residual-matched** to the flexml / matmul class that already reached CONFIG_CU attach. A 300 B vector-add control stream is a different animal from 14–27 KiB flexml-style images (spent `0eeeb70c…` 27200 B; unspent `ba0cd738…` 21664 B / `7d172bf2…` 13792 B). Soft-compare mostly collapses that to “size mismatch / external_differs_from_lab” without multi-axis structural distance.

### Goal

Run **both** complementary offline routes in **parallel hierarchical implement loops**, invent_pass=0:

| Route | What | Outcome |
|-------|------|---------|
| **A** | Better public matmul-scale npu2 AOT material (same toolchain that worked) | Second capture set under `external_known_good/` + PROVENANCE + ingest |
| **B** | Stronger pure offline structural classifiers | Residual can cite shape/topology/opcode distance, not only 300 vs 1688 |

### Constraints (hard)

- invent_pass=0; **no** Pass / **no** `ran_on_npu` from offline work
- **Do not** mint `C_UNLOCK` in this plan (unlock only *after* A and/or B land, as a **separate** intentional step with a named reason)
- **Do not** re-live spent flexml fingerprint
- **Do not** treat current 300 B vector example alone as unlock justification
- Keep vector_scalar_add material (prove playbook works; do not delete)
- Agent-legal offline only (no L3 / ALLOW / live apply in A or B)
- Parent (L1) coordinates; L2 implementers own work; L3 specialists only if L2 needs them (max depth L3)
- Clone/build **outside** dragon-npu tree; only bins + PROVENANCE land in-repo

### Non-goals

- Product silicon Pass
- Live stock shot / new unlock mint inside this plan’s Steps
- Full whole_array / cascade matmul at large M/K/N on first try
- Driver/MMU work (still parked)

### Assumptions

- Reuse existing mlir-aie capture env: `/home/hunter/Projects/ai/mlir-aie-capture/venv` + examples at `/home/hunter/Projects/ai/mlir-aie`
- Prefer small M/K/N for first matmul AOT (default 512³ is heavy; start tiny)
- Route A and Route B do not share product file write races if A only touches capture dir + docs/PROVENANCE and B only touches `silicon_contract.rs` + re-exports + tests; residual pin serializes after both

Explore reports (read-only):

- `.agents/reports/plan-explore-route-a-matmul-material.md`
- `.agents/reports/plan-explore-route-b-structure-tools.md`

---

## Approach

### Parallel orchestration (hierarchical)

```
L1 parent (HITL coordinator)
├── L2-A  Route A implementer (material playbook)
│   └── L3 only if needed (e.g. matmul build debug)
├── L2-B  Route B implementer (pure TDD structure tools)
│   └── L3 only if needed (narrow pure helper)
└── L2-R  Residual pin + status closeout (after A and B reports land)
```

1. **Spawn L2-A and L2-B in parallel** (disjoint write scopes).
2. **Wait for both** healthy reports on disk.
3. **Spawn L2-R** to pin residual honesty, run `--status` / soft compare, write join report.
4. **Stop.** Unlock is **not** auto-minted; handoff notes when a named unlock reason is now legitimate.

### Route A — matmul public material (preferred next material)

Reuse proven AOT pattern from `vector_scalar_add`:

1. Env: same venv + Peano + symlink-only `aiebu-asm` (not full ryzen-ai bin).
2. Build first: `programming_examples/basic/matrix_multiplication/single_core/`
   - Prefer **small** dims (lit has small-M configs; e.g. M/K/N=64 or 128, tiles 32 if divisible).
   - Compile-only: `--dev npu2` + `--xclbin-path` / `--insts-path` (+ pdi/elf if exposed).
3. Fallback if single_core fails: `matrix_vector/` or `getting_started/03_matrix_multiplication_single_core/`.
4. Peel `.ctrltext` from aiebu ELF when available.
5. Land as **second set** (do not overwrite vector files):
   - `mlir_aie_single_core_matmul_npu2.{ctrltext,insts,insts.elf,xclbin,pdi,_core.elf}`
   - Optionally promote a **second** primary only if residual policy wants it; keep `working_stack_ctrltext.bin` as vector unless operator wants matmul as soft-compare primary (default: **keep** vector primary; add matmul siblings; soft tools can point `XDNA2_LAB_EXTERNAL_CTRLTEXT` at matmul peel for A/B compare).
6. Append PROVENANCE entry (example path, dims, commit, sizes, sha16/sha256, invent_pass=0 honesty).
7. Ingest matmul ctrltext (and optional `--ingest-pdi` research).
8. Report sizes vs lab 1688 vs unspent flexml PDI classes.

### Route B — offline structure tools (pure TDD)

Build on existing APIs in `silicon_contract.rs` (do not reinvent compare):

| Priority | Delta | Why |
|----------|-------|-----|
| **B2 first** | `research_external_vs_lab_structural_distance` (+ size-class labels) | Makes 300 vs 1688 **actionable** for residual pin |
| **B1** | Wire `TxnOpOpcodeProfile` / header fields into external-vs-lab compare soft note | Histogram already exists on `profile_instr_payload_bytes` |
| **B3** | `fingerprint_capture_material_dir_research` | Multi-file bag (ctrltext+xclbin+pdi+elf) one research fingerprint |
| **B4 optional** | `compare_rela_structure_lab_vs_external_elf` | When both sides are aiebu ELF |

Honesty locks on every soft note: `invent_pass=0 not_c_ready=1 not_ran_on_npu=1 product_known_good=0`. Match/distance ≠ Pass.

Wire soft print into `classify_last_apply` / existing PRINT flag only if low friction; otherwise pure API + unit tests + residual pin citing format note strings is enough for this slice.

### After A+B

Residual top pin: bag has vector **and** matmul public sets (if A succeeded); structural distance API can cite axes; still **no** Pass; unlock **none** unless a **separate** approve mints one with reason:

- matmul-scale public material present, and/or
- unspent flexml topology (`ba0cd738…` / `7d172bf2…`) + capture no longer empty + residual class still post-attach `wait_cmd_einval`.

**Not X:** unlock mint in same PR as A/B; re-live spent flexml; empty reconfirm mill; claim Pass from AOT.

---

## Critical files

| Path | Why |
|------|-----|
| `testdata/xdna2_cmd/external_known_good/` | Land matmul bins; keep vector set |
| `testdata/xdna2_cmd/external_known_good/PROVENANCE.md` | Second entry + hashes |
| `docs/CAPTURE_MATERIAL_MLIR_AIE.md` | Note matmul as preferred next example after vector |
| `src/backends/amd_xdna2/silicon_contract.rs` | Route B pure helpers + TDD |
| `src/backends/amd_xdna2.rs` | Re-exports |
| `examples/classify_last_apply.rs` | Optional soft-print distance note |
| `tools/pass_unblock_once.sh` | Ingest/status (A); optional status tokens if cheap |
| `tools/operator_external_capture.sh` | Soft compare path |
| `RESIDUAL.md` / `AGENTS.md` | Honesty pin after A+B (L2-R only) |
| Outside: `/home/hunter/Projects/ai/mlir-aie` + `mlir-aie-capture/` | Build only (read-only for product tree scope) |

---

## Reuse

| Symbol / module | Path | How |
|-----------------|------|-----|
| Proven AOT playbook | `docs/CAPTURE_MATERIAL_MLIR_AIE.md`, prior report `impl-mlir-aie-material-playbook.md` | Same env/flags for matmul |
| `compare_external_ctrltext_to_lab_export` | `silicon_contract.rs` | Extend soft note or sibling distance research |
| `profile_instr_payload_bytes` / `TxnOpOpcodeProfile` | same | Feed B1/B2 axes |
| `fingerprint_pdi_geometry_bytes` / C5 | same | Size class vs flexml for matmul PDI if present |
| `validate_aiebu_elf_ctrltext_structure` (C1) | same | Validate matmul ELF offline |
| `pass_unblock_once.sh --ingest` | tools | One PATH per ingest |
| Flexml unspent ledger | RESIDUAL / fingerprint reports | Cite in distance/residual, not re-live |

---

## Steps (implement after approve)

Fib leaves size 1|2 only. Parent seeds these as `impl:*` on handoff.

### Phase 0 — orchestration (L1)

0. **Mirror plan** — copy this plan to `.agents/plans/plan-route-a-b-material-structure.md` size=1
1. **Spawn L2-A + L2-B parallel** — wait both size=1

### Route A leaves (L2-A owns)

2. **A0 env probe** — confirm venv, Peano, aiebu-asm symlink, mlir-aie tree size=1
3. **A1 build single_core matmul npu2 AOT** — small M/K/N first; collect artifacts size=2
4. **A2 land + PROVENANCE** — second set names; append table; honesty size=1
5. **A3 ingest + status** — `--ingest` matmul peel; optional `--ingest-pdi`; `--status` size=1
6. **A4 report** — `.agents/reports/impl-route-a-matmul-material.md` size=1

### Route B leaves (L2-B owns; parallel with A)

7. **B0 red tests distance** — fail on missing multi-axis external vs lab distance size=2
8. **B1 implement B2 API** — `research_external_vs_lab_structural_distance` + size_class + soft note honesty size=2
9. **B2 opcode/header soft** — wire histogram + per-side header tokens into compare path (B1) size=1
10. **B3 capture-dir fingerprint** — multi-file bag research fingerprint (B3) size=1
11. **B4 optional rela** — only if 7–10 green and low cost size=1
12. **B5 gates** — fmt + clippy -D amd-xdna2 + targeted cargo tests size=1
13. **B6 report** — `.agents/reports/impl-route-b-structure-tools.md` size=1

### Join (after A and B)

14. **R residual pin** — top table: dual material sets; structure tools landed; Pass=No; unlock=none size=1
15. **R status closeout** — soft compare vector + matmul peels; cite distance tokens; join report size=1

### Explicitly out of Steps (do not implement without new plan)

- Mint `C_UNLOCK`
- Live `config-cu-run` / L3 sandwich
- Re-submit spent flexml PDI

---

## Orchestration notes for implementers

**L2-A prompt essentials:** invent_pass=0; keep vector files; outside-tree build; small dims; report even if build fails with exact error; no unlock.

**L2-B prompt essentials:** red→green TDD; always honesty tokens; re-export new symbols; do not rewrite tests to fake green; no live apply.

**L2-R:** read A and B reports only; pin residual; do not re-run full matmul build.

**Same-file race:** A must not edit `silicon_contract.rs`; B must not rewrite capture PROVENANCE except via residual pin if needed (A owns PROVENANCE append).

---

## Risks

| Risk | Mitigation |
|------|------------|
| Matmul AOT too heavy / aiecc OOM / long compile | Start tiny M/K/N; fallback matrix_vector / getting_started; report wall honestly |
| single_core CLI flags differ from vector_scalar_add | Read `add_compile_args` / Makefile; mirror proven path |
| Foreign PATH breaks MLIR imports | Symlink only `aiebu-asm` |
| Route B scope creep (full CU decode) | Cap at distance + hist + dir fingerprint; B4 optional |
| Residual claims Pass | L2-R honesty checklist; invent_pass=0 gates in tests |
| Soft compare picks wrong bag file | Keep vector as first `.bin`; matmul under distinct names; env override for matmul peel |

---

## Verification

### Route A

```bash
ls -la testdata/xdna2_cmd/external_known_good/mlir_aie_single_core_matmul_npu2.*
# PROVENANCE has matmul entry with dims + sha
bash tools/pass_unblock_once.sh --status   # capture_bins >= previous; invent_pass=0
# Soft compare pointing at matmul ctrltext (env) should not invent Pass
```

### Route B (TDD)

```bash
# Red observed then green on same filters (names may match implementer report):
cargo test --features amd-xdna2 --lib structural_distance
cargo test --features amd-xdna2 --lib external_ctrltext_compare
cargo test --features amd-xdna2 --lib capture_material_dir
cargo fmt -p dragon-npu   # or package name as in tree
cargo clippy --features amd-xdna2 --lib -- -D warnings
```

Named contracts (examples; implementer may refine):

- Present 300 B vs lab 1688 → distance note with `size_class_mismatch=1`, op hist present, `invent_pass=0`
- Self-match synthetic classic → axes zero distance still invent_pass=0
- Empty bag → blocked token, not Pass

### Honesty footer (both)

```
invent_pass=0
product_pass=0
ran_on_npu=0
Pass=No
C_UNLOCK_active=none
```

---

## Open questions

None blocking. Defaults if operator silent:

- Soft-compare **primary** remains `working_stack_ctrltext.bin` (vector); matmul is a **second named set**.
- Unlock **not** minted in this plan.
- B4 rela compare is optional after B2/B1/B3 green.

---

## Critical Files for Implementation

- `src/backends/amd_xdna2/silicon_contract.rs` — Route B pure research APIs + TDD
- `src/backends/amd_xdna2.rs` — re-exports
- `testdata/xdna2_cmd/external_known_good/` + `PROVENANCE.md` — Route A land
- `docs/CAPTURE_MATERIAL_MLIR_AIE.md` — matmul next-example note
- `tools/pass_unblock_once.sh` — ingest/status
- `RESIDUAL.md` — post A+B honesty pin only
- Outside tree: `/home/hunter/Projects/ai/mlir-aie`, `mlir-aie-capture/` — AOT build only
