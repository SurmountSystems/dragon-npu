# Wave5 host scan + agent Path B/C attempt

- **Date:** 2026-08-11
- **Operator ask:** agent owns capture attempt (“lol no u do it”)
- **Pass:** **No**
- **invent_pass:** 0
- **not_ran_on_npu:** 1 (product claim)
- **not_c_ready:** 1
- **load_accepted_claim:** 0
- **Unlock mint:** **none** (first `C_UNLOCK=` still `4008e6fd` CONSUMED)

## What ran

| Step | Result |
|------|--------|
| `bash tools/load_accepted_path_status.sh` | Path B trees ready; Path C `xrt-smi` present; Path A FastFlowLM/Lemonade **absent** |
| `bash tools/host_capture_scan.sh --report` | 8 roots present; **120** hits; multi_txn=1; research_dup=11; durable `.agents/reports/wave5-host-capture-scan.md` |
| Host find (extra) | CVML linux overlays + phoenix VOE xclbins; no NEW residual-matched ctrltext |
| Dry-run `collect_load_accepted_candidate.sh` | See below (all refuse load_accepted_claim) |
| Path B IRON NPU run | **PASS!** after system `pyxrt` on `PYTHONPATH` |
| Path C | `xrt-smi examine` OK (device present); full `validate` **not** run |

## Path B — IRON completed run (agent)

First attempt **failed** (`path_b_rc=1`):

```
no NPU runtime device is available to run the pre-built artifacts
```

Root cause: capture venv has **no** `pyxrt`. Host has
`/usr/lib/python3.14/site-packages/pyxrt.cpython-314-x86_64-linux-gnu.so`.
Device `/dev/accel/accel0` is world-writable + user in `render`.

Second attempt **succeeded** (`path_b_rc=0`):

```bash
export PYTHONPATH="/usr/lib/python3.14/site-packages:$VENV/.../mlir_aie/python"
# + PEANO / capture PATH as in docs/LOAD_ACCEPTED_CAPTURE.md
cd $MLIR_AIE/programming_examples/basic/vector_scalar_add
python3 vector_scalar_add.py --dev npu2 \
  --from-xclbin $CAP/aot_vector_scalar_add/vector_scalar_add.xclbin \
  --from-insts  $CAP/aot_vector_scalar_add/vector_scalar_add.insts.bin
# → PASS!
```

Durable log: `.agents/reports/path-b-vector-scalar-add-npu2-pass-20260811.log`

| Item | Value |
|------|-------|
| Device | RyzenAI-npu6 `0000:c6:00.1` |
| NPU FW | **1.1.2.64** |
| XRT | 2.21.75 |
| Ctrltext | **300 B** sha16 **`aaeb5b68cc298a45`** (identical to bag research peel) |
| iron_completed_on_this_fw | **yes** |
| product load_accepted_claim | **0** |
| residual_matched_claim | **0** |
| invent_pass | **0** |
| product Pass | **No** |

Journal scrape (1h window after run): **no** APP_LOAD / IO_PAGE_FAULT matches
(`journal_npu_scrape.sh --hours 1`). Soft only; ≠ Pass.

PROVENANCE updated under
`testdata/xdna2_cmd/external_known_good/PROVENANCE.md` § Path B agent attempt.
Product load-accepted table **not** filled as product `load_accepted=1`
(research-duplicate sha + no product decide).

## Host scan summary (no new residual-matched peel)

### Present roots

- `/tmp`, `~/.cache`, `ryzen-ai-install`, `mlir-aie-capture`, `RyzenAI-SW`
- dragon-npu `gt_capture`, `external_known_good`, `silicon_lab`

### Absent

- FastFlowLM / Lemonade trees
- `/opt/xilinx/xrt`, `/opt/amd/ryzen-ai`

### Interesting paths (not load-accepted)

| Path | Note |
|------|------|
| `mlir-aie-capture/aot_*` | Public AOT peels 300/660/2592/4512 — research / now IRON-completed for 300 only |
| `gt_capture/instr_post_patch.bin` 1688 | Local oracle basename **refused** |
| `gt_capture/instr_vendor_post_patch.bin` 32208 | multi_txn **refused** |
| `/tmp/instr_post_patch.bin` size=2 | junk + refused basename |
| CVML `…/linux/onnx/ryzen14/*.xclbin` | Large overlays (0.46–6.0 MiB); dry-run classify only; no completing-stack evidence for residual match |
| `ryzen-ai-install/…/phoenix/{1x4,4x4}.xclbin` | Phoenix class; not residual-matched ctrltext |

### Dry-run collect results

| Candidate | Outcome |
|-----------|---------|
| `gt_capture/instr_post_patch.bin` | **REFUSE** basename oracle |
| `/tmp/instr_post_patch.bin` | **REFUSE** basename + size 2 |
| cascade `ctrltext` 4512 | **REFUSE** research_duplicate_sha16 |
| multi-txn 32208 | **REFUSE** multi_txn |
| vector_scalar_add.xclbin | dry-run OK classify; completing_stack_evidence_ok=0; load_accepted_claim=0 |
| lab_instr.elf | **REFUSE** basename |
| vector_scalar_add.ctrltext (even after Path B PASS) | **REFUSE** research_duplicate_sha16 for load_accepted |
| CVML / phoenix xclbin | dry-run classify; load_accepted_claim=0 |

## Honesty (hard)

- invent_pass=0 · Pass=**No** · not_c_ready=1 · product not_ran_on_npu=1
- IRON PASS ≠ product `decide_ran_on_npu` ≠ residual match to lab classic 1688
- Same 300 B peel is **not** re-tagged product load-accepted by helpers
- **No** unlock minted; **no** ALLOW; **no** dragon-npu live apply
- Path A still absent (agent will not network-install)

## Who acts next

1. **Residual wall still material-shaped:** product residual-matched / load-accepted
   identity for lab classic 1688 / matmul stack is **not** closed by IRON vector
   scalar add 300 B (size-class public-npu2-small).
2. Optional next captures: larger IRON matmul **to completion** (single_core /
   whole_array / cascade) with same `pyxrt` PYTHONPATH fix; or working DD/XRT
   post-patch ctrltext that is **not** a research-duplicate peel.
3. Agent: no empty reconfirm mill; no unlock mint from this pin alone.
4. Live dragon-npu shot only under **new** host-minted unlock + named offline reason.

## Related artifacts

- `.agents/reports/wave5-host-capture-scan.md` (scan tool report)
- `.agents/reports/path-b-vector-scalar-add-npu2-pass-20260811.log`
- `docs/LOAD_ACCEPTED_CAPTURE.md`
- `testdata/xdna2_cmd/external_known_good/PROVENANCE.md` (Path B section)
