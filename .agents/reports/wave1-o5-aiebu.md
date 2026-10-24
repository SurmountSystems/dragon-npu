# Wave W1 O5 — aiebu offline rebuild / wrap findings

**Date:** 2026-08-11
**Scope:** Offline only. Explore whether aiebu can rebuild/wrap lab classic ELF or
public ctrltexts differently (rela count, peel). No live apply. No unlock mint.
**Honesty:** invent_pass=0 · product_pass=0 · ran_on_npu=0 · Pass=No · aiebu wrap
≠ residual identity ≠ C-ready.

Related: `tools/stage_aiebu_lab_instr.sh`, `scripts/materialize_txn_offline.sh`,
`docs/CONSTRUCTION_MAP_APP_LOAD.md`, `.agents/reports/wave0-structural-diff.md`,
`testdata/xdna2_cmd/external_known_good/PROVENANCE.md`.

---

## Tools present (host)

| Tool | Path | Role |
|------|------|------|
| `aiebu-asm` | `/home/hunter/Projects/ai/ryzen-ai-install/venv/bin/aiebu-asm` (also symlink `mlir-aie-capture/tools/aiebu-asm`) | txn → aiebu ELF |
| `aiebu-dump` | `mlir-aie-capture/tools/aiebu-dump` | dump/report aiebu objects |
| Lab stage helper | `tools/stage_aiebu_lab_instr.sh` | `-t aie2txn -c lab_instr.bin [-p lab_ctrl.bin] -o lab_instr.elf` |
| Research materialize | `scripts/materialize_txn_offline.sh --from-aiebu` | vendor DMA copy helper only (NOT GEMM) |

**Do not invent vendor post-patch.** aiebu does not create a working-stack EXEC peel.

---

## What aiebu actually does (closed offline)

XRT-style path (construction map):

1. Raw classic txn bytes (`ml_txn` / `lab_instr.bin`)
2. Optional control packet blob (`lab_ctrl.bin`)
3. `aiebu-asm -t aie2txn -c <txn> [-p <ctrl>] -o <elf>`
4. ELF carries `.ctrltext` + `.ctrldata` (if ctrl) + `.rela.dyn` + `.dynsym`
5. Host peels `.ctrltext`, applies `.rela.dyn` with live BO paddrs (Rust:
   `patch_aiebu_elf_ctrltext_with_bos_ex`)

**aiebu cannot:** invent residual-matched post-EXEC ctrltext; change FW accept;
turn public IRON AOT into lab classic 1688 identity; mint Pass.

---

## Lab classic corpus (silicon_lab) — measured wrap

| Artifact | Size | Notes |
|----------|-----:|-------|
| `testdata/xdna2_cmd/silicon_lab/lab_instr.bin` | 1688 | Classic v0.1 txn (mladf staged) |
| `testdata/xdna2_cmd/silicon_lab/lab_ctrl.bin` | 29664 | Control packet BO body |
| `testdata/xdna2_cmd/silicon_lab/lab_instr.elf` | **32264** | aiebu wrap with ctrl |

Measured sections (construction map / prior gates):

| Section | Size / count |
|---------|----------------|
| `.ctrltext` | **1688 B** |
| `.ctrldata` | **29664 B** |
| `.rela.dyn` | **12** entries (sym 1→IFM×8, 2→WTS×2, 3→OFM×2) |
| dynsym names | opaque all `"6"` |

Stage command (already landed; re-run is reconfirm only):

```bash
bash tools/stage_aiebu_lab_instr.sh
# aiebu-asm -t aie2txn -c lab_instr.bin -p lab_ctrl.bin -o lab_instr.elf
```

**Txn-only wrap (no ctrl)** is also supported by the stage script when
`lab_ctrl.bin` is absent; silicon corpus uses **txn + ctrl**.

Userspace rela path is **green offline**. Live residual after attach is
`wait_cmd_einval` (post-flexml) / historical APP_LOAD — **not** “forgot aiebu.”

---

## Research sample at testdata root (do not confuse)

| Artifact | Size | Notes |
|----------|-----:|-------|
| `testdata/xdna2_cmd/lab_instr.elf` | 1072 | Tiny research sample (not silicon corpus) |
| `testdata/xdna2_cmd/aiebu_report.txt` | — | Report for that sample: ctrltext **416 B**, rela **1×12 B**, gen4, cols/rows **4×6**, NumOps 13 |

This is **not** lab classic 1688 and **not** a product known-good peel.

---

## Public IRON AOT wraps (already landed)

Post-build wrap used for all public sets:

```bash
aiebu-asm -t aie2txn -c <insts.bin> -o <insts.elf>
llvm-objcopy -O binary --only-section=.ctrltext <insts.elf> <ctrltext.bin>
```

| Stream | raw insts | peeled ctrltext | insts.elf | rela_count | vs lab 12 |
|--------|----------:|----------------:|----------:|-----------:|-----------|
| vector_scalar_add | 300 | **300** (byte-identical) | 992 | **1** | mismatch |
| single_core matmul | 660 | **660** (**not** identical to raw) | 1408 | **5** | mismatch |
| whole_array matmul | 2592 | **2592** (**not** identical to raw) | 3520 | **20** | mismatch |
| lab classic | 1688 | 1688 | 32264 (+ctrl) | **12** | self |

**Normalization finding:** aiebu may rewrite one header field (single_core /
whole_array peels differ from raw `insts.bin` sha). Peel of `.ctrltext` is the
canonical research stream for soft compare. Normalization **≠** residual match.

**Rela finding:** rela count is **program-shaped** (IFM/WTS/OFM patch sites for
mladf ×12; IRON public examples 1/5/20). Re-wrapping the same raw txn does not
invent 12 relas for a 5-rela program. You cannot “force” lab rela structure onto
public AOT without a different txn body.

---

## Experiments considered (honest offline)

| Experiment | Result | Pass impact |
|------------|--------|-------------|
| Re-stage lab classic with aiebu + ctrl | Already on disk (`silicon_lab/lab_instr.elf`); userspace green | None alone |
| Re-wrap public `insts.bin` → ELF → peel | Already done for 300/660/2592; rela 1/5/20 | None alone |
| Wrap public ctrltext **as if** it were raw txn again | Expected: same size class; may double-wrap/normalize; **cannot** produce 1688 or rela×12 | No |
| Wrap lab classic **without** ctrl | Smaller ELF, missing `.ctrldata`; not residual close | No |
| aiebu on spent flexml PDI | PDI is CONFIG_CU overlay, **not** aie2txn ctrltext | Wrong object type; refuse as progress |
| Invent vendor post-patch via aiebu | **Forbidden** | N/A |

---

## What would still be new for O5 (not done = blocked)

1. **Working-stack EXEC peel** after ERT Completed + useful OFM on **this**
   firmware (post-patch ctrltext from a completing XRT/DD path). aiebu alone
   cannot produce that.
2. **Different raw txn body** that is residual-matched (not more re-wrap of the
   same 300/660/2592/1688).
3. Live under **new** unlock + named reason — out of O5 scope.

---

## Soft classification (always)

```
invent_pass=0
not_c_ready=1
not_ran_on_npu=1
aiebu_wrap_alone_not_pass=1
rela_structure_alone_not_pass=1
peel_alone_not_pass=1
product_known_good=0
```

---

## Conclusion

aiebu offline wrap/peel for lab classic (rela×12 + ctrl) and public AOT
(rela 1/5/20) is **already understood and staged**. Re-running aiebu on present
bytes is empty reconfirm, not new residual evidence. Residual remains EXEC /
WAIT EINVAL after attach (and historical APP_LOAD), with public material still
**residual-mismatched** to lab classic 1688 and to spent flexml PDI class.
)
