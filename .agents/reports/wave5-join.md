# Wave5 join — load-accepted capture automation closeout

**Date:** 2026-08-11
**Kind:** Join pin (docs/residual + agent capture attempt results). No unlock mint.
**Honesty:** invent_pass=**0** · Pass=**No** · active unlock=**none** · first
`C_UNLOCK=` = **`4008e6fd` CONSUMED** · residual_matched_claim=**0** for lab
classic 1688 · IRON ≠ product Pass · load_accepted_claim=**0**
**Amend:** matmul Path B closed same day — see § Matmul Path B +
`wave5-join-matmul-amend.md`.

---

## One-screen status

### What landed (W5 tools)

| Slice | Outcome |
|-------|---------|
| **W5 S0** | Plan mirror + residual pin; campaign start. Report: `impl-w5-s0-pin-mirror.md` |
| **W5-2** | `tools/host_capture_scan.sh` read-only host walk; refuse multi-txn / research-dup sha. |
| **W5-3** | PROVENANCE load-accepted template (`completed_on_this_fw`, stack, FW, claim rules). |
| **W5-4** | `tools/collect_load_accepted_candidate.sh` fail-closed dry-run/ingest; claim never invent. |
| **W5-5/6/7** | `docs/LOAD_ACCEPTED_CAPTURE.md` Paths B/C/A; playbook §3b; `tools/load_accepted_path_status.sh`. |
| **W5 agent** | Operator “lol no u do it” → agent ran host scan + Path B + Path C (not checklist-only). |
| **W5 matmul Path B** | single_core / whole_array / cascade all IRON **PASS!**; research_dup only. |

Scan on this host: **120** hits; multi_txn=1; research_dup=11; durable
`.agents/reports/wave5-host-capture-scan.md`. All dry-run collect rows
`load_accepted_claim=0`.

### Path B — IRON public AOT peels on FW 1.1.2.64 (agent)

| Peel | Ctrltext | sha16 | IRON NPU | completed_on_this_fw | load_accepted / residual_match |
|------|----------|-------|----------|----------------------|--------------------------------|
| `vector_scalar_add` | **300 B** | `aaeb5b68cc298a45` | **PASS!** | **yes** | **0** / **0** |
| single_core matmul | **660 B** | `de8671245bb72436` | **PASS!** | **yes** | **0** / **0** |
| whole_array matmul | **2592 B** | `7ee4b267ce31cd70` | **PASS!** | **yes** | **0** / **0** |
| cascade matmul | **4512 B** | `18cb653c05a9d4c8` | **PASS!** | **yes** | **0** / **0** |

| Item | Value |
|------|--------|
| Device / FW | RyzenAI-npu6 · NPU Firmware **1.1.2.64** · XRT 2.21.75 |
| Host fix | system **`pyxrt`** on `PYTHONPATH` (capture venv alone → “no NPU runtime”) |
| Collect class (all four) | **`research_duplicate_sha16`** — bag peels unchanged; re-tag **refused** |
| product Pass / `ran_on_npu` | **No** / false |
| invent_pass | **0** |
| Evidence (vector) | `.agents/reports/path-b-vector-scalar-add-npu2-pass-20260811.log`; `wave5-path-b-agent-attempt.md` |
| Evidence (matmul ×3) | `.agents/reports/wave5-path-b-matmul-attempt.md`; capture dirs under `mlir-aie-capture/completed_run_*_matmul_20260811_150*` |

**Meaning:** IRON + XRT + amdxdna stack is **proven working** on this host/FW for
all four public mlir-aie npu2 AOT programs (vector + three matmuls). That is
**not** product `decide_ran_on_npu`, not residual match for the lab classic
**1688** path, and not a product load-accepted re-tag of known research peels.

### Path C — `xrt-smi`

| Item | Value |
|------|--------|
| `xrt-smi` / examine | **OK** — device present, FW 1.1.2.64 |
| validate | **FAILED** — **no platform test archives** (`/opt/xilinx/xrt/test/` absent) |
| NPU sample complete | **No** (tests skipped; no peel) |
| Ingest | **none** |
| Report | `wave5-path-c-agent-attempt.md` |

### Path A — FastFlowLM / Lemonade

| Item | Value |
|------|--------|
| Host trees | **Absent** |
| Agent action | no network install (policy) |

### Matmul Path B (closed)

| Item | Value |
|------|--------|
| `wave5-path-b-matmul-attempt.md` | **present** — three IRON completed runs |
| single_core 64³ i16→i32 | **PASS!** · 660 B · `de8671245bb72436` · research_dup |
| whole_array 128³ 4-col | **PASS!** · 2592 B · `7ee4b267ce31cd70` · research_dup |
| cascade 128³ 4-col | **PASS!** · 4512 B · `18cb653c05a9d4c8` · research_dup |
| New ctrltext identity | **no** — same bag peels |
| load_accepted_claim | **0** (helper refuse re-tag 660/2592/4512) |
| residual_matched_claim (lab classic 1688) | **0** |
| Amend report | `wave5-join-matmul-amend.md` |

### Honesty / unlock

| Item | Value |
|------|--------|
| invent_pass | **0** |
| Pass | **No** |
| ran_on_npu (product) | **false** (not claimed) |
| load_accepted_claim | **0** |
| residual_matched_claim (lab classic) | **0** |
| completed_on_this_fw (public AOT 300/660/2592/4512) | **yes** (IRON on FW 1.1.2.64) |
| Active unlock | **None** |
| First `C_UNLOCK=` | **`4008e6fd` CONSUMED** (do not reopen) |
| Prior CONSUMED | `48da6e60`, `1f761bf4`, `efd44f35`, `8352882b`, `2ae69036`, `97f38307`, `11bc9016`, `3a70fabf`, `b17b0119` |
| L3 / ALLOW / dragon-npu live apply | **not opened** |

IRON PASS ≠ product Pass ≠ residual match ≠ unlock mint.

### Material wall (still primary)

- Product residual-matched / load-accepted identity for **lab classic 1688** /
  matmul-bias working stack is **still open**.
- Public mlir-aie peels (300 / 660 / 2592 / cascade 4512) remain **RESEARCH ONLY**;
  Path B completed IRON on the **same** shas does **not** re-tag them.
- Multi-txn 32208 still **refused**.
- Host scan found no new residual-matched ctrltext (CVML/phoenix xclbins = research classify only).
- **Do not** re-run public AOT as residual progress; next useful material is
  **non-public** DD/XRT post-patch ctrltext (or intercept peels from a
  completing stack that matches residual identity).

### Soft W5-8 — scorecard `completed_on_this_fw`

| Question | Answer |
|----------|--------|
| PROVENANCE / helpers | Already document **`completed_on_this_fw`** as operator/evidence field (yes only when true; Path B IRON has **`completed_on_this_fw=yes`** for all four public peels; product load_accepted still 0). |
| Pure scorecards (`MaterialExecReadinessScorecard`, residual-match inventory) | Score **bytes** (size class, walk, claim=0 always). They do **not** invent FW-complete load-accepted from structure alone. |
| Product code change this join? | **No.** Not clearly missing for offline TDD: field lives in PROVENANCE + collect helpers; hard-coding “yes” from IRON on research-dup sha would be dishonest. Optional later: soft research token that **reads** PROVENANCE only — out of scope unless named ask. |

### Live queue

| Slot | Status |
|------|--------|
| **L1** | Only if residual-matched / load-accepted material lands + **new** unlock |
| **L5 / L5b** | **Closed** (CONSUMED) |
| **L7** | **No** |
| **L8** | **Not** from wire alone (W4 already: pack+wire green ≠ L8 justify) |

### Freeze list

- No unlock mint from this join
- No invent Pass / invent residual match / invent load-accepted
- No re-tag public AOT 300/660/2592/4512 as product load-accepted
- No re-live under CONSUMED unlocks
- No sticky ALLOW / L3 from agent
- No empty reconfirm mill
- No multi-txn as known-good
- No re-run public IRON AOT as residual progress

### Who acts next

1. **Residual-useful path (preferred):** non-public DD/XRT **post-patch**
   ctrltext for the matmul-bias lab path, or intercept peels from a completing
   stack that **matches residual identity** (lab classic 1688 / working stack).
   **Not** re-run public mlir-aie AOT (300/660/2592/4512 already completed_on_this_fw).
2. **Material wall still open** for lab classic 1688 product path.
3. **L1** only after residual match + **new** host-minted unlock + named reason.
4. **L8** only if new offline reason beyond “wire landed” + new unlock.
5. **Idle** on empty reconfirm / public AOT re-run; Path C archive install is operator/human if wanted.

---

## Report index

| Report | Role |
|--------|------|
| `impl-w5-s0-pin-mirror.md` | S0 plan pin + mirror |
| `impl-w5-scan-extract.md` | W5-2/3/4 tools + PROVENANCE template |
| `impl-w5-path-docs.md` | W5-5/6/7 path docs + status helper |
| `wave5-host-scan-agent.md` | Agent host scan + Path B/C summary |
| `wave5-host-capture-scan.md` | Durable scan tool output |
| `wave5-path-b-agent-attempt.md` | Path B IRON vector PASS detail |
| `wave5-path-b-matmul-attempt.md` | Path B IRON matmul ×3 PASS detail |
| `wave5-path-c-agent-attempt.md` | Path C validate archives missing |
| `path-b-vector-scalar-add-npu2-pass-20260811.log` | Path B vector durable log |
| `wave5-join-matmul-amend.md` | This join’s matmul amend pin |
| `wave5-join.md` | this join |

## Explicit non-claims

- No product Pass · invent_pass=0 · no product `ran_on_npu`
- No unlock mint · no reopen `4008e6fd`
- No load_accepted_claim from IRON scalar-add alone
- No residual_matched_claim for lab classic 1688
- No Path C peel · no Path A install
- No L8 live · no dragon-npu live apply
