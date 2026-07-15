# Instruction source provenance (offline extract)

**NOT_AIE2_KNOWN_GOOD=1**

| Field | Value |
|-------|--------|
| PLATFORM | VE2_EDGE |
| CLAIM | not_known_good_for_aie2_17f0 |
| date_utc | 2026-07-15T10:49:34Z |
| elf_source | /home/hunter/Projects/ai/dragon-npu/kernel/dkms/staging/amdxdna-hardened-0.1.0/src/shim_ve2/Runner/latency/nop.elf |
| xclbin_source | /home/hunter/Projects/ai/dragon-npu/kernel/dkms/staging/amdxdna-hardened-0.1.0/src/shim_ve2/Runner/latency/validate.xclbin |
| recipe_source | /home/hunter/Projects/ai/dragon-npu/kernel/dkms/staging/amdxdna-hardened-0.1.0/src/shim_ve2/Runner/latency/recipe_latency.json |
| out | /home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/ve2_latency |
| force_lab_instr | 0 |
| allow_ve2_as_lab_instr | 0 |
| lab_instr_written | 0 |
| stage_note | Staged by dragon-npu L4 prepare_from_clone.sh date: 2026-07-13T23:57:26-06:00 clone: /tmp/xdna-l4-rerev clone_HEAD: 905717f5cd470580054a1635b9eea24b84dfd67f expected_base_SHA: 905717f5cd470580054a1635b9eea24b84dfd67f  |

elf_sha256=ee4ee73f80cb34105b8aca4a0f95e0657f5d0c4ae1157567493df5bb8c940115
xclbin_sha256=700a29d80bbeb6f3143c42ea1832e4d1972a7746f03a29ebe20273f607e00e82
recipe_sha256=1eb40e05cb2b2ccd32625b1f6879c41ab2ffb41b13a618e9d65c1979d42c9540


## Honesty

- VE2 Runner `nop.elf` / `validate.xclbin` are **VE2 edge** fixtures (`ops = ve2_ops`),
  **not** Strix PCI `1022:17f0` / npu4 AIE2 known-good instruction BO content.
- Feeding raw ELF bytes into `XDNA2_INSTRUCTION_BLOB` only exercises envelope plumbing.
- Product `ran_on_npu` requires partner/npu4 patched ctrltext + OFM numerical verify.
- Binaries under this tree are **gitignored**; `PROVENANCE.md` / `*.sha256` / `*.txt` may be tracked.
- Script never sets `XDNA2_ALLOW_EXEC_CMD` or runs EXEC_CMD.
- `lab_exec_once` refuses blobs with sibling PROVENANCE NOT_AIE2_KNOWN_GOOD unless
  `--allow-research-blob`.

## Recipe (if VE2)

See staged `sources/ve2_runner_latency_recipe_latency.json` (kernel DPU instance `k1`,
IFM-sized 20-byte args) — not the dragon-npu matmul OFM contract.
