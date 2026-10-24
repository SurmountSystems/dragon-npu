# W6-1 residual identity distance — lab classic 1688 vs IRON public peels

**Date:** 2026-08-11
**Kind:** Offline residual identity distance map (docs/report only).
**Honesty:** invent_pass=**0** · residual_matched_claim=**0** · Pass=**No** ·
active unlock=**none** · first `C_UNLOCK=` = **`4008e6fd` CONSUMED** ·
IRON PASS ≠ product Pass ≠ product load-accepted · no unlock mint · no live apply.

---

## Goal

Refresh the offline distance map between:

1. **Lab residual path** — classic **1688 B** aiebu + rela×12 (silicon_lab / gt_capture).
2. **IRON Path B public peels** — mlir-aie npu2 AOT that completed on this FW
   (vector 300 / single_core 660 / whole_array 2592 / cascade 4512).

Sources measured on disk this turn:

| Source | Path |
|--------|------|
| Lab classic raw | `testdata/xdna2_cmd/lab_instr.bin` |
| Lab staged | `testdata/xdna2_cmd/silicon_lab/lab_instr.bin` + `lab_instr.elf` |
| GT capture | `testdata/xdna2_cmd/gt_capture/instr_raw_txn.bin` / `instr_post_patch.bin` / `instr_pre.elf` |
| Capture dir | `testdata/xdna2_cmd/external_known_good/` (peels + PROVENANCE) |
| Prior Path B | `.agents/reports/wave5-join.md`, `wave5-path-b-matmul-attempt.md`, `wave5-join-matmul-amend.md` |
| Structure tools | Route B in `silicon_contract.rs` (`CtrltextSizeClass`,
  `research_external_vs_lab_structural_distance`, `classify_material_residual_match`) |

---

## Distance table (ctrltext identity)

| Material | Size (B) | sha16 | Txn header (v / gen / grid / NumOps / txn_size) | Size-class tag (code) | Docs residual tag | Match lab classic 1688? | completed_on_this_fw | residual_matched_claim | product load_accepted |
|----------|----------|-------|--------------------------------------------------|------------------------|-------------------|-------------------------|----------------------|------------------------|------------------------|
| **Lab classic raw** `lab_instr.bin` / silicon_lab same / `gt_capture/instr_raw_txn.bin` | **1688** | **`3c5ffca750922d3b`** | ClassicTxn v0.1 · gen4 · **8×6** · **NumOps=40** · size=1688 · hdr match | **`mladf_classic_1688`** | **`mladf-classic-1688`** | **identity (oracle)** | n/a (lab oracle; not IRON AOT) | **0** (oracle ≠ product known-good) | **0** |
| **Lab post-rela** `gt_capture/instr_post_patch.bin` | **1688** | **`ccab3960f6dd789b`** | same header profile (40 ops / 8×6 / gen4); body **≠** unpatched | **`mladf_classic_1688`** | **`mladf-classic-1688`** | same **size-class + header profile**; **byte body DIFF** vs raw | n/a | **0** | **0** |
| **Lab aiebu ELF** `silicon_lab/lab_instr.elf` ≡ `gt_capture/instr_pre.elf` | **32264** | **`6ed93809133413fa`** | ELF `.ctrltext`=1688; **`.rela.dyn` ×12** | aiebu+rela path (not size-class on ELF) | lab residual EXEC path | N/A (container for 1688) | n/a | **0** | **0** |
| IRON **vector_scalar_add** (+ `working_stack_ctrltext.bin` alias) | **300** | **`aaeb5b68cc298a45`** | ClassicTxn v0.1 · gen4 · **1×6** · **NumOps=8** · size=300 | **`small_public_npu2`** | **`public-npu2-small`** | **NO** (len, sha, grid, ops) | **yes** (Path B IRON PASS FW **1.1.2.64**) | **0** | **0** (research_dup) |
| IRON **single_core matmul** 64³ i16→i32 | **660** | **`de8671245bb72436`** | ClassicTxn v0.1 · gen4 · **1×6** · **NumOps=17** · size=660 | **`small_public_npu2`** | **`public-npu2-small`** | **NO** | **yes** | **0** | **0** |
| IRON **whole_array matmul** 128³ 4-col | **2592** | **`7ee4b267ce31cd70`** | ClassicTxn v0.1 · gen4 · **8×6** · **NumOps=68** · size=2592 | **`other`** | **`public-npu2-other` / `other`** | **NO** (len, sha, ops; grid matches 8×6 only) | **yes** | **0** | **0** |
| IRON **cascade matmul** | **4512** | **`18cb653c05a9d4c8`** | ClassicTxn v0.1 · gen4 · **8×6** · **NumOps=116** · size=4512 | **`other`** | **`public-npu2-other` / `other`** | **NO** | **yes** | **0** | **0** |

**cmp (byte identity vs lab raw 1688):** all four IRON peels **MISMATCH**.
`working_stack_ctrltext.bin` **≡** vector 300 peel (research alias, not lab classic).

---

## Multi-axis distance (lab classic vs each IRON peel)

Axes from Route B + measured headers. Soft always:
`distance_alone_not_pass=1 invent_pass=0 not_c_ready=1 not_ran_on_npu=1 product_known_good=0`.

| Axis | Lab classic | vs 300 | vs 660 | vs 2592 | vs 4512 |
|------|-------------|--------|--------|---------|---------|
| **Length** | 1688 | 300 (−1388) | 660 (−1028) | 2592 (+904) | 4512 (+2824) |
| **sha16** | `3c5ffca750922d3b` | different | different | different | different |
| **Size class** | `mladf_classic_1688` | `small_public_npu2` | `small_public_npu2` | `other` | `other` |
| **size_class_match** | — | **false** | **false** | **false** | **false** |
| **Header v0.1 classic** | yes | yes | yes | yes | yes |
| **dev_gen** | 4 | 4 | 4 | 4 | 4 |
| **cols×rows** | 8×6 | **1×6** | **1×6** | 8×6 | 8×6 |
| **NumOps** | **40** | **8** | **17** | **68** | **116** |
| **txn_size == len** | yes | yes | yes | yes | yes |
| **Staged mladf classic profile** (ops=40 ∧ size=1688) | **yes** | no | no | no | no |
| **Rela count (aiebu `.rela.dyn`)** | **12** (lab ELF) | **2** | **5** | **20** | **36** |
| **aiebu ELF size** | 32264 | 992 | 1408 | 3520 | 5632 |
| **PDI size (research sibling)** | n/a (lab uses CONFIG_CU flexml/CVML PDIs historically) | 2528 | 5536 | 66864 | 39696 |
| **Byte identity to lab raw** | yes | no | no | no | no |
| **residual_matched_claim** | 0 | 0 | 0 | 0 | 0 |

**Closest structural (still not residual match):** whole_array **2592** and cascade **4512** share **gen4 + 8×6** with lab classic; still wrong size, wrong NumOps (68/116 vs 40), wrong sha, wrong rela×N.
**Farthest:** vector **300** and single_core **660** (1×6 grid, small-public size class).

---

## Lab residual path (oracle inventory)

| Artifact | Size | sha16 | Role |
|----------|------|-------|------|
| `testdata/xdna2_cmd/lab_instr.bin` | 1688 | `3c5ffca750922d3b` | Unpatched classic raw (≡ silicon_lab raw ≡ gt raw) |
| `testdata/xdna2_cmd/silicon_lab/lab_instr.bin` | 1688 | `3c5ffca750922d3b` | Same |
| `testdata/xdna2_cmd/silicon_lab/lab_instr.elf` | 32264 | `6ed93809133413fa` | aiebu ELF; `.ctrltext` 1688; **rela×12** |
| `testdata/xdna2_cmd/gt_capture/instr_pre.elf` | 32264 | `6ed93809133413fa` | ≡ silicon_lab ELF |
| `testdata/xdna2_cmd/gt_capture/instr_raw_txn.bin` | 1688 | `3c5ffca750922d3b` | ≡ lab raw |
| `testdata/xdna2_cmd/gt_capture/instr_post_patch.bin` | 1688 | `ccab3960f6dd789b` | After synthetic rela paddrs (body DIFF, same class) |

Lab residual product path = **START_NPU + aiebu ELF + rela×12** on classic 1688 body
(userspace green historically; FW residual separate). This is the identity
**residual_matched** material would need to approach, not IRON AOT peels.

---

## IRON Path B (completed_on_this_fw)

From W5 join + this turn re-hash (bags unchanged):

| Peel | Ctrltext | sha16 | IRON NPU | FW | completed_on_this_fw | load_accepted | residual_match lab |
|------|----------|-------|----------|-----|----------------------|---------------|--------------------|
| vector_scalar_add | 300 | `aaeb5b68cc298a45` | **PASS!** | 1.1.2.64 | **yes** | **0** | **0** |
| single_core matmul | 660 | `de8671245bb72436` | **PASS!** | 1.1.2.64 | **yes** | **0** | **0** |
| whole_array matmul | 2592 | `7ee4b267ce31cd70` | **PASS!** | 1.1.2.64 | **yes** | **0** | **0** |
| cascade matmul | 4512 | `18cb653c05a9d4c8` | **PASS!** | 1.1.2.64 | **yes** | **0** | **0** |

Device context (W5): RyzenAI-npu6 · XRT 2.21.75 · amdxdna present · system `pyxrt`
required on `PYTHONPATH`. Collect class for all four: **`research_duplicate_sha16`**
(helper refuse re-tag).

---

## Classifier / walk notes (offline cheap)

| Check | Result |
|-------|--------|
| `classify_ctrltext_size_class` | 1688 → `mladf_classic_1688`; 300/660 → `small_public_npu2` (max **1024**); 2592/4512 → `other` |
| `classify_material_residual_match` (len / mlir_aie tags) | always **`residual_matched_claim=0`**; public peels → research_only; 1688 → lab oracle not product_known_good |
| `cargo test --features amd-xdna2 --lib material_residual_match` | **ok** |
| `structural_distance_*` unit tests | **ok** (300 vs lab shape + self-match invent_pass=0) |
| Cascade unit soft-walk | header classic v0.1 gen4 8×6 ops=116 size=4512; **walk** currently reports `MASK* size field 28 != 32` → `txn_header_only` in live classifier (`optional_cascade_public_aot_classifies_if_present` **FAILED** this host). **Does not** change residual mismatch vs 1688. Report as header-OK / walk soft-fail offline; not a Pass claim either way. |

---

## Match / mismatch conclusions

1. **No residual identity** between lab classic **1688** (`3c5ffca750922d3b`, NumOps **40**, rela×**12**, 8×6) and any of the four IRON peels (300/660/2592/4512).
   **residual_matched_claim=0** for all public AOT rows.
2. **Shared format family only:** all are ClassicTxn **v0.1** + **dev_gen=4**. That is **not** residual match.
3. **Size-class wall:** lab = `mladf_classic_1688`; vector/single_core = `small_public_npu2`; whole_array/cascade = `other`.
   Code max for small-public = **1024** (covers 300 and 660).
4. **IRON completed_on_this_fw=yes** for all four peels on FW **1.1.2.64** proves the **host/XRT/amdxdna IRON path** works. It does **not** re-tag peels as product load-accepted or lab residual identity.
5. **working_stack_ctrltext.bin** is the **300 B vector** peel alias, **not** a working-stack matmul-bias capture.
6. **Post-rela lab body** (`ccab3960f6dd789b`) differs from unpatched raw but stays classic 1688 class — still the residual oracle family; IRON peels are outside it.
7. **Primary wall unchanged:** need residual-matched / load-accepted material for the **lab classic 1688 / DD matmul-bias** path (non-public post-patch or completing-stack intercept), **not** another public IRON AOT re-run.
8. **Honesty:** invent_pass=0 · Pass=No · no unlock mint · no live dragon-npu apply · IRON PASS ≠ `decide_ran_on_npu`.

---

## Freeze / next

| Do | Do not |
|----|--------|
| Hunt non-public DD/XRT post-patch ctrltext matching lab classic identity | Re-run public mlir-aie AOT as residual progress |
| Cite size-class / NumOps / grid / rela axes in unlock **text** if a future unlock is justified | Mint unlock from this distance map alone |
| Keep research peels as RESEARCH ONLY | Re-tag 300/660/2592/4512 as load-accepted |
| Optional offline: fix cascade MASK* walk strictness (classifier hygiene) | Invent residual_matched_claim=1 |

**Unlock:** none active. First `C_UNLOCK=` = **`4008e6fd` CONSUMED**. Do not mint.

---

## Offline test evidence (this turn)

```text
cargo test --features amd-xdna2 --lib material_residual_match  → ok
cargo test --features amd-xdna2 --lib structural_distance_300 → ok
cargo test --features amd-xdna2 --lib structural_distance_size_class_labels → ok
cargo test --features amd-xdna2 --lib structural_distance_self_match_zero_still_invent_pass_0 → ok
# optional_cascade_public_aot_classifies_if_present → FAILED (walk MASK* size);
#   header identity still 4512/ops=116/8×6 as above
```

No `XDNA2_ALLOW_EXEC_CMD`; no live IRON re-run; no product apply.

---

## Cross-links

- `.agents/reports/wave5-join.md`
- `.agents/reports/wave5-path-b-matmul-attempt.md`
- `.agents/reports/wave5-join-matmul-amend.md`
- `.agents/reports/impl-route-b-structure-tools.md`
- `testdata/xdna2_cmd/external_known_good/PROVENANCE.md`
- `docs/LOAD_ACCEPTED_CAPTURE.md`
- `RESIDUAL.md` top (primary wall: residual-matched / load-accepted material)
