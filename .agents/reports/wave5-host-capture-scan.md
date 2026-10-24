# wave5 host capture scan (W5-2)

- **Date:** 2026-08-11T15:41:55-06:00
- **Pass:** **No**
- **invent_pass:** 0
- **not_ran_on_npu:** 1
- **not_c_ready:** 1
- **load_accepted_claim:** 0

## Counts

| metric | value |
|--------|-------|
| roots present | 8 |
| hits (capture-like) | 147 |
| listed (cap 48) | 48 |
| multi_txn 32208 | 1 |
| research-duplicate sha16 | 11 |
| large soft | 2 |
| other listed | 34 |

## Honesty

- invent_pass=0 · Pass=No · not_ran_on_npu=1 · not_c_ready=1
- load_accepted_claim=0 always from this scan alone
- Multi-txn 32208 and known public mlir-aie research sha16 never claimed load-accepted
- Read-only; no sudo, EXEC_CMD, ALLOW, network install, live apply

## Full stdout log

```
================================================================
host_capture_scan - read-only (W5-2)
================================================================
ROOT=/home/hunter/Projects/ai/dragon-npu
date=2026-08-11T15:41:52-06:00
Honesty: invent_pass=0 not_ran_on_npu=1 not_c_ready=1 Pass=No load_accepted_claim=0
No sudo / no EXEC_CMD / no ALLOW / no invent bytes / no live apply
Multi-txn 32208 and known research sha16 never claimed load-accepted

=== candidate roots (present / missing) ===
  present: /tmp
  present: /home/hunter/.cache
  present: /home/hunter/Projects/ai/ryzen-ai-install
  present: /home/hunter/Projects/ai/mlir-aie-capture
  present: /home/hunter/Projects/ai/RyzenAI-SW
  missing: /home/hunter/Projects/ai/RyzenAI
  missing: /home/hunter/ryzen-ai
  missing: /home/hunter/Projects/ai/FastFlowLM
  missing: /home/hunter/Projects/ai/fastflowlm
  missing: /home/hunter/Projects/ai/Lemonade
  missing: /home/hunter/Projects/ai/lemonade
  missing: /home/hunter/.local/share/FastFlowLM
  missing: /home/hunter/.local/share/lemonade
  missing: /opt/xilinx/xrt
  missing: /opt/xilinx/xrt/share
  missing: /opt/xilinx
  missing: /opt/amd/ryzen-ai
  missing: /usr/share/xrt
  missing: /usr/lib/xrt
  present: /home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/gt_capture
  present: /home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/external_known_good
  present: /home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/silicon_lab

=== capture-like files (bounded maxdepth=5 list_cap=48) ===
  HIT bytes=3308 sha16=0f56a114b434bbf7 head8=7f454c4601010100 refuse_class=none load_accepted_claim=0 invent_pass=0 path=/home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/external_known_good/mlir_aie_cascade_matmul_npu2_core.elf note=host_candidate_not_proven
  HIT bytes=4512 sha16=18cb653c05a9d4c8 head8=0001040608010000 refuse_class=research_duplicate_sha16 load_accepted_claim=0 invent_pass=0 path=/home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/external_known_good/mlir_aie_cascade_matmul_npu2.ctrltext.bin note=matches_known_public_mlir_aie_research_peel
  HIT bytes=4512 sha16=e1e4db951d44092d head8=0001040608010000 refuse_class=research_duplicate_sha16 load_accepted_claim=0 invent_pass=0 path=/home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/external_known_good/mlir_aie_cascade_matmul_npu2.insts.bin note=matches_known_public_mlir_aie_research_peel
  HIT bytes=5632 sha16=486bc26f5f5de1be head8=7f454c4601010145 refuse_class=none load_accepted_claim=0 invent_pass=0 path=/home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/external_known_good/mlir_aie_cascade_matmul_npu2.insts.elf note=host_candidate_not_proven
  HIT bytes=39696 sha16=e2a2b177dc3f4a5e head8=dd00000044332211 refuse_class=none load_accepted_claim=0 invent_pass=0 path=/home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/external_known_good/mlir_aie_cascade_matmul_npu2.pdi note=host_candidate_not_proven
  HIT bytes=46153 sha16=478214ec08d481f7 head8=78636c62696e3200 refuse_class=none load_accepted_claim=0 invent_pass=0 path=/home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/external_known_good/mlir_aie_cascade_matmul_npu2.xclbin note=host_candidate_not_proven
  HIT bytes=4632 sha16=3c17f06f9b690348 head8=7f454c4601010100 refuse_class=none load_accepted_claim=0 invent_pass=0 path=/home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/external_known_good/mlir_aie_single_core_matmul_npu2_core.elf note=host_candidate_not_proven
  HIT bytes=660 sha16=de8671245bb72436 head8=0001040601010000 refuse_class=research_duplicate_sha16 load_accepted_claim=0 invent_pass=0 path=/home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/external_known_good/mlir_aie_single_core_matmul_npu2.ctrltext.bin note=matches_known_public_mlir_aie_research_peel
  HIT bytes=660 sha16=34baaff69260033c head8=0001040601010000 refuse_class=research_duplicate_sha16 load_accepted_claim=0 invent_pass=0 path=/home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/external_known_good/mlir_aie_single_core_matmul_npu2.insts.bin note=matches_known_public_mlir_aie_research_peel
  HIT bytes=1408 sha16=6fbbf33de3543be7 head8=7f454c4601010145 refuse_class=none load_accepted_claim=0 invent_pass=0 path=/home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/external_known_good/mlir_aie_single_core_matmul_npu2.insts.elf note=host_candidate_not_proven
  HIT bytes=5536 sha16=cd3d9fa6577d1e37 head8=dd00000044332211 refuse_class=none load_accepted_claim=0 invent_pass=0 path=/home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/external_known_good/mlir_aie_single_core_matmul_npu2.pdi note=host_candidate_not_proven
  HIT bytes=11993 sha16=8a96c4cb5e01656c head8=78636c62696e3200 refuse_class=none load_accepted_claim=0 invent_pass=0 path=/home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/external_known_good/mlir_aie_single_core_matmul_npu2.xclbin note=host_candidate_not_proven
  HIT bytes=2468 sha16=ffd58932f0f38bca head8=7f454c4601010100 refuse_class=none load_accepted_claim=0 invent_pass=0 path=/home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/external_known_good/mlir_aie_vector_scalar_add_npu2_core.elf note=host_candidate_not_proven
  HIT bytes=300 sha16=aaeb5b68cc298a45 head8=0001040601010000 refuse_class=research_duplicate_sha16 load_accepted_claim=0 invent_pass=0 path=/home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/external_known_good/mlir_aie_vector_scalar_add_npu2.ctrltext.bin note=matches_known_public_mlir_aie_research_peel
  HIT bytes=300 sha16=aaeb5b68cc298a45 head8=0001040601010000 refuse_class=research_duplicate_sha16 load_accepted_claim=0 invent_pass=0 path=/home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/external_known_good/mlir_aie_vector_scalar_add_npu2.insts.bin note=matches_known_public_mlir_aie_research_peel
  HIT bytes=992 sha16=790d8b3e6d25bda8 head8=7f454c4601010145 refuse_class=none load_accepted_claim=0 invent_pass=0 path=/home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/external_known_good/mlir_aie_vector_scalar_add_npu2.insts.elf note=host_candidate_not_proven
  HIT bytes=2528 sha16=fe438059f5d0e4eb head8=dd00000044332211 refuse_class=none load_accepted_claim=0 invent_pass=0 path=/home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/external_known_good/mlir_aie_vector_scalar_add_npu2.pdi note=host_candidate_not_proven
  HIT bytes=8983 sha16=849d0ba7276ada71 head8=78636c62696e3200 refuse_class=none load_accepted_claim=0 invent_pass=0 path=/home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/external_known_good/mlir_aie_vector_scalar_add_npu2.xclbin note=host_candidate_not_proven
  HIT bytes=4456 sha16=2d47c4d6017cc6a6 head8=7f454c4601010100 refuse_class=none load_accepted_claim=0 invent_pass=0 path=/home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/external_known_good/mlir_aie_whole_array_matmul_npu2_core.elf note=host_candidate_not_proven
  HIT bytes=2592 sha16=7ee4b267ce31cd70 head8=0001040608010000 refuse_class=research_duplicate_sha16 load_accepted_claim=0 invent_pass=0 path=/home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/external_known_good/mlir_aie_whole_array_matmul_npu2.ctrltext.bin note=matches_known_public_mlir_aie_research_peel
  HIT bytes=2592 sha16=ae692499a79c74b4 head8=0001040608010000 refuse_class=research_duplicate_sha16 load_accepted_claim=0 invent_pass=0 path=/home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/external_known_good/mlir_aie_whole_array_matmul_npu2.insts.bin note=matches_known_public_mlir_aie_research_peel
  HIT bytes=3520 sha16=0984b40244c33f68 head8=7f454c4601010145 refuse_class=none load_accepted_claim=0 invent_pass=0 path=/home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/external_known_good/mlir_aie_whole_array_matmul_npu2.insts.elf note=host_candidate_not_proven
  HIT bytes=66864 sha16=e721b80335e6d8ed head8=dd00000044332211 refuse_class=large_container_soft load_accepted_claim=0 invent_pass=0 path=/home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/external_known_good/mlir_aie_whole_array_matmul_npu2.pdi note=large_container_scan_only_not_ctrltext_claim
  HIT bytes=73327 sha16=44ab78cf9eeb315e head8=78636c62696e3200 refuse_class=large_container_soft load_accepted_claim=0 invent_pass=0 path=/home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/external_known_good/mlir_aie_whole_array_matmul_npu2.xclbin note=large_container_scan_only_not_ctrltext_claim
  HIT bytes=300 sha16=aaeb5b68cc298a45 head8=0001040601010000 refuse_class=research_duplicate_sha16 load_accepted_claim=0 invent_pass=0 path=/home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/external_known_good/working_stack_ctrltext.bin note=matches_known_public_mlir_aie_research_peel
  HIT bytes=29664 sha16=18d4b7c0c46417c5 head8=0800000080483000 refuse_class=none load_accepted_claim=0 invent_pass=0 path=/home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/gt_capture/ctrl_post_patch.bin note=host_candidate_not_proven
  HIT bytes=1688 sha16=ccab3960f6dd789b head8=0001040608010000 refuse_class=none load_accepted_claim=0 invent_pass=0 path=/home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/gt_capture/instr_post_patch.bin note=host_candidate_not_proven
  HIT bytes=32264 sha16=6ed93809133413fa head8=7f454c4601010145 refuse_class=none load_accepted_claim=0 invent_pass=0 path=/home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/gt_capture/instr_pre.elf note=host_candidate_not_proven
  HIT bytes=1688 sha16=3c5ffca750922d3b head8=0001040608010000 refuse_class=none load_accepted_claim=0 invent_pass=0 path=/home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/gt_capture/instr_raw_txn.bin note=host_candidate_not_proven
  HIT bytes=32208 sha16=3de5f68d14ff1773 head8=0001040608010000 refuse_class=multi_txn_32208 load_accepted_claim=0 invent_pass=0 path=/home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/gt_capture/instr_vendor_get_transaction_bin.bin note=historical_multi_txn_wrong_body_not_load_accepted
  HIT bytes=1688 sha16=3c5ffca750922d3b head8=0001040608010000 refuse_class=none load_accepted_claim=0 invent_pass=0 path=/home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/gt_capture/instr_vendor_post_patch.bin note=host_candidate_not_proven
  HIT bytes=1688 sha16=3c5ffca750922d3b head8=0001040608010000 refuse_class=none load_accepted_claim=0 invent_pass=0 path=/home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/gt_capture/instr_vendor_txn_bin.bin note=host_candidate_not_proven
  HIT bytes=32264 sha16=6ed93809133413fa head8=7f454c4601010145 refuse_class=none load_accepted_claim=0 invent_pass=0 path=/home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/silicon_lab/lab_instr.elf note=host_candidate_not_proven
  HIT bytes=3308 sha16=0f56a114b434bbf7 head8=7f454c4601010100 refuse_class=none load_accepted_claim=0 invent_pass=0 path=/home/hunter/Projects/ai/mlir-aie-capture/aot_cascade_matmul_npu2/cascade_matmul_core.elf note=host_candidate_not_proven
  HIT bytes=4512 sha16=18cb653c05a9d4c8 head8=0001040608010000 refuse_class=research_duplicate_sha16 load_accepted_claim=0 invent_pass=0 path=/home/hunter/Projects/ai/mlir-aie-capture/aot_cascade_matmul_npu2/cascade_matmul.ctrltext.bin note=matches_known_public_mlir_aie_research_peel
  HIT bytes=4512 sha16=e1e4db951d44092d head8=0001040608010000 refuse_class=research_duplicate_sha16 load_accepted_claim=0 invent_pass=0 path=/home/hunter/Projects/ai/mlir-aie-capture/aot_cascade_matmul_npu2/cascade_matmul.insts.bin note=matches_known_public_mlir_aie_research_peel
  HIT bytes=5632 sha16=486bc26f5f5de1be head8=7f454c4601010145 refuse_class=none load_accepted_claim=0 invent_pass=0 path=/home/hunter/Projects/ai/mlir-aie-capture/aot_cascade_matmul_npu2/cascade_matmul.insts.elf note=host_candidate_not_proven
  HIT bytes=39696 sha16=e2a2b177dc3f4a5e head8=dd00000044332211 refuse_class=none load_accepted_claim=0 invent_pass=0 path=/home/hunter/Projects/ai/mlir-aie-capture/aot_cascade_matmul_npu2/cascade_matmul.pdi note=host_candidate_not_proven
  HIT bytes=3308 sha16=0f56a114b434bbf7 head8=7f454c4601010100 refuse_class=none load_accepted_claim=0 invent_pass=0 path=/home/hunter/Projects/ai/mlir-aie-capture/aot_cascade_matmul_npu2/cascade_matmul.prj/elfs_main_core_0_2/elfs_main_core_0_2.elf note=host_candidate_not_proven
  HIT bytes=3044 sha16=bfbd68b5f4d805f6 head8=7f454c4601010100 refuse_class=none load_accepted_claim=0 invent_pass=0 path=/home/hunter/Projects/ai/mlir-aie-capture/aot_cascade_matmul_npu2/cascade_matmul.prj/elfs_main_core_0_3/elfs_main_core_0_3.elf note=host_candidate_not_proven
  HIT bytes=3012 sha16=16d28785982fd333 head8=7f454c4601010100 refuse_class=none load_accepted_claim=0 invent_pass=0 path=/home/hunter/Projects/ai/mlir-aie-capture/aot_cascade_matmul_npu2/cascade_matmul.prj/elfs_main_core_0_4/elfs_main_core_0_4.elf note=host_candidate_not_proven
  HIT bytes=2800 sha16=416788fb4528fb8f head8=7f454c4601010100 refuse_class=none load_accepted_claim=0 invent_pass=0 path=/home/hunter/Projects/ai/mlir-aie-capture/aot_cascade_matmul_npu2/cascade_matmul.prj/elfs_main_core_0_5/elfs_main_core_0_5.elf note=host_candidate_not_proven
  HIT bytes=3556 sha16=ef73d9e1ddeb43ed head8=7f454c4601010100 refuse_class=none load_accepted_claim=0 invent_pass=0 path=/home/hunter/Projects/ai/mlir-aie-capture/aot_cascade_matmul_npu2/cascade_matmul.prj/elfs_main_core_1_2/elfs_main_core_1_2.elf note=host_candidate_not_proven
  HIT bytes=3260 sha16=34a83fabdeacf939 head8=7f454c4601010100 refuse_class=none load_accepted_claim=0 invent_pass=0 path=/home/hunter/Projects/ai/mlir-aie-capture/aot_cascade_matmul_npu2/cascade_matmul.prj/elfs_main_core_1_3/elfs_main_core_1_3.elf note=host_candidate_not_proven
  HIT bytes=3228 sha16=c06548f069993532 head8=7f454c4601010100 refuse_class=none load_accepted_claim=0 invent_pass=0 path=/home/hunter/Projects/ai/mlir-aie-capture/aot_cascade_matmul_npu2/cascade_matmul.prj/elfs_main_core_1_4/elfs_main_core_1_4.elf note=host_candidate_not_proven
  HIT bytes=3016 sha16=3e2cc929bfbaada9 head8=7f454c4601010100 refuse_class=none load_accepted_claim=0 invent_pass=0 path=/home/hunter/Projects/ai/mlir-aie-capture/aot_cascade_matmul_npu2/cascade_matmul.prj/elfs_main_core_1_5/elfs_main_core_1_5.elf note=host_candidate_not_proven
  HIT bytes=3556 sha16=a3e7899aa106eb64 head8=7f454c4601010100 refuse_class=none load_accepted_claim=0 invent_pass=0 path=/home/hunter/Projects/ai/mlir-aie-capture/aot_cascade_matmul_npu2/cascade_matmul.prj/elfs_main_core_2_2/elfs_main_core_2_2.elf note=host_candidate_not_proven
  HIT bytes=3260 sha16=d4a2e1efeeac167d head8=7f454c4601010100 refuse_class=none load_accepted_claim=0 invent_pass=0 path=/home/hunter/Projects/ai/mlir-aie-capture/aot_cascade_matmul_npu2/cascade_matmul.prj/elfs_main_core_2_3/elfs_main_core_2_3.elf note=host_candidate_not_proven
  ... listed first 48 of 147

=== English summary ===
  roots present: 8  hits: 147  listed: 48
  multi_txn_32208: 1  research_duplicate_sha16: 11  large_soft: 2  other: 34
  Operator next for true load-accepted: place completing-stack capture under
    testdata/xdna2_cmd/external_known_good/  with load-accepted PROVENANCE fields
  Collect helper (fail-closed): bash tools/collect_load_accepted_candidate.sh PATH [NAME]
  Stack inventory: bash tools/host_npu_stack_inventory.sh

=== Footer (honest) ===
invent_pass=0
not_ran_on_npu=1
not_c_ready=1
Pass=No
load_accepted_claim=0
hits_n=147
multi_txn_n=1
research_dup_n=11
roots_present_n=8
entrypoint=tools/host_capture_scan.sh
```
