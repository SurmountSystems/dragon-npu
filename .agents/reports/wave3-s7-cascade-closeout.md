# Wave W3 S7 — cascade AOT closeout

**Date:** 2026-08-11
**Scope:** Offline only. No live NPU. No unlock mint. No ALLOW=1.
**Honesty:** invent_pass=0 · residual_matched_claim=0 · product_pass=0 · ran_on_npu=0 · Pass=**No**

Related: `.agents/reports/wave3-cascade-aot-attempt.md`,
`testdata/xdna2_cmd/external_known_good/PROVENANCE.md` (fourth set),
`RESIDUAL.md` top, `AGENTS.md` Latest Turn.

---

## 1. Landed?

| Question | Answer |
|----------|--------|
| **Landed** | **Yes** |
| Capture path | `testdata/xdna2_cmd/external_known_good/` |
| Primary ctrltext | `mlir_aie_cascade_matmul_npu2.ctrltext.bin` |
| Size / sha16 | **4512 B** / `18cb653c05a9d4c8` |
| Siblings | insts.bin, insts.elf, pdi, xclbin, core.elf |
| Primary bag | `working_stack_ctrltext.bin` **unchanged** (still 300 B vector) |
| Soft compare (prior) | sha_match=0 structural_match=0 residual_not_closed=1 invent_pass=0 |

Build + ingest evidence: `.agents/reports/wave3-cascade-aot-attempt.md`.

---

## 2. Offline classify (txn / material helpers)

### 2.1 Helpers used

| Helper | Module | Use |
|--------|--------|-----|
| `classify_instruction_blob` | `txn.rs` | Kind of ctrltext / ELF bytes |
| `validate_txn_blob` | `txn.rs` | looks_like_txn refuse path |
| `is_staged_mladf_classic_instr_profile` | `txn.rs` | Lab 1688 profile check |
| `classify_ctrltext_size_class` | `silicon_contract.rs` | Length → size class |
| `classify_material_residual_match` | `silicon_contract.rs` | Residual-match inventory honesty |

Unit soft-skip (present-file): `optional_cascade_public_aot_classifies_if_present`
in `src/backends/amd_xdna2/txn.rs` (asserts ClassicTxnV01 + aiebu ELF peel).

### 2.2 Build-time aiebu header (from S7 attempt)

Post-process `aiebu-asm -t aie2txn` on cascade insts reported:

- Header **v0.1** (classic walk path)
- **gen4**, cols/rows **8×6**
- TransactionSize **4512**
- NumOps **116**
- UID `9d1e9296…` (asm-side)

`.ctrltext` peel size **4512** matches TransactionSize.

### 2.3 Classification verdict

| Artifact | Expected `InstructionBlobKind` | Notes |
|----------|--------------------------------|-------|
| `…ctrltext.bin` (4512) | **`ClassicTxnV01`** | Classic **START_NPU / XAie txn stream** (v0.1 full op walk). **Not** lab classic 1688/40 profile. |
| `…insts.bin` (4512) | Classic txn stream (same size class) | Distinct sha from ctrltext peel (`e1e4db95…` vs `18cb653c…`). |
| `…insts.elf` (5632) | **`Aie2ElfCtrltext`** → inner ClassicTxnV01 | aiebu ELF with `.ctrltext` len 4512. |
| Lab classic oracle | ClassicTxnV01 1688 / 40 ops | **Mismatch** size and body. |
| Spent flexml PDI class | PDI sizes 27200 / 21664 / 13792 | Cascade PDI **39696** ≠ spent flexml; rematch alone ≠ unlock. |

**Size class (length-only):** `CtrltextSizeClass::Other`
(4512 > `CTRLTEXT_SMALL_PUBLIC_NPU2_MAX_BYTES` 1024, and ≠ 1688).

**Material residual inventory (pure):**

| Input | kind | size_class | residual_matched_claim |
|-------|------|------------|------------------------|
| len=4512, no tag | `unknown` | `other` | **0** |
| len=4512, tag `mlir_aie` / `public_npu2` | `public_npu2_small_research_only` (kind token) | `other` | **0** |

Always: invent_pass=0 · research_only=1 · unlock_justification_alone=0 ·
product_known_good=0 · inventory_alone_not_pass=1 ·
public_aot_neq_residual_identity=1.

### 2.4 What this is / is not

| Is | Is not |
|----|--------|
| Valid classic **v0.1 START_NPU-style txn** body (format-compatible with lab START_NPU envelope) | Residual-matched / load-accepted material |
| aiebu-peelable ELF path | Lab classic 1688 identity |
| New public research size/sha | Pass / `ran_on_npu` / C-ready |
| Optional offline structure study | Unlock justification alone |

---

## 3. L7 live justified? **No**

Proposed L7 shape (from campaign language): skip-CONFIG_CU + public cascade
bytes as instruction BO under a **new** unlock.

| Criterion | Hold? | Why |
|-----------|-------|-----|
| Named offline product fix | **No** | Cascade is research AOT, not a userspace bugfix. |
| residual_matched / load-accepted | **No** | residual_matched_claim=0; no live ERT+OFM path. |
| Lab-compatible **identity** (not mere format) | **No** | Classic walk yes; body class is public IRON cascade matmul (116 ops / 4512), **not** residual oracle 1688 or working-stack peel. |
| Would teach more than re-prove residual | **Unlikely** | Skip-CU already closed under `1f761bf4` as `wait_cmd_einval` without overlay; wrong-body public AOT as instruction is expected to hit the same EXEC residual class (or fail earlier), not invent Pass. |
| Unlock criteria all hold | **No** | Unlock needs a new host-minted value **and** a real offline reason beyond “more public AOT.” |

**Decision: L7 live not justified. Prefer no unlock.**
Public AOT research live is optional risk only; do **not** mint `C_UNLOCK` for cascade alone.

---

## 4. Who acts next

1. **Highest (Pass lever):** working-stack **residual-matched / load-accepted**
   program material (live ERT Completed + non-zero OFM allclose via
   `decide_ran_on_npu`). Prefer honest EXEC peel into the capture dir with
   PROVENANCE.
2. **Cascade role:** optional research material only (structure / size-class /
   opcode axes). **Not** the next live shot reason.
3. **Live queue:** L1 blocked; L4 not open from unspent list alone; L5/L5b
   closed; **L7 no**; L8 needs live syncobj wire under a **new** unlock only
   after more offline if justified.
4. **Frozen:** re-live CONSUMED unlocks; spent flexml; invent Pass; sticky
   ALLOW; unlock mint from cascade / public AOT alone.

---

## 5. Explicit honesty

| Flag | Value |
|------|-------|
| invent_pass | **0** |
| Pass | **No** |
| ran_on_npu | **false** / not claimed |
| residual match | **0** (not invented) |
| Unlock minted this task | **No** |
| Live apply | **No** |
| First `C_UNLOCK=` | `4008e6fd` **CONSUMED** (do not reopen) |

---

## 6. Docs / code touched this closeout

| Path | Change |
|------|--------|
| `RESIDUAL.md` | Plain status + size-class tags + ranked next + material rule: cascade landed, L7 no, invent_pass=0 |
| `AGENTS.md` | Latest Turn W3 S7 cascade closeout pin |
| `src/backends/amd_xdna2/txn.rs` | Soft-skip unit `optional_cascade_public_aot_classifies_if_present` |
| This report | Closeout |

Re-verify offline (operator/CI):

```bash
cargo test --features amd-xdna2 --lib optional_cascade_public_aot_classifies_if_present -- --nocapture
```

---

## 7. Verdict

| Item | Result |
|------|--------|
| Landed | **Yes** |
| Offline classify | Classic **START_NPU/txn** + **aiebu ELF peel** (not residual match) |
| L7 justified | **No** |
| Unlock mint | **No** |
| invent_pass | **0** |
| Who acts next | Residual-matched / load-accepted material (highest); cascade optional research only |

**report:** `/home/hunter/Projects/ai/dragon-npu/.agents/reports/wave3-s7-cascade-closeout.md`
