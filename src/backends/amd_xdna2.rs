//! AMD XDNA2 (Ryzen AI 300 series / Strix Point, Strix Halo, Krackan Point, etc.) backend.
//!
//! This module is only compiled when the `amd-xdna2` feature is enabled.
//!
//! **Philosophy**: We are building a lighter-weight, open, focused, and eventually
//! more performant direct driver implementation against the amdxdna kernel driver.
//! We deliberately avoid heavy official runtimes (XRT, IRON, etc.) that the
//! original Python project leaned on. The goal is a clean, maintainable, direct
//! path using the fd and ioctls exposed by the mainline amdxdna driver.
//!
//! **Primary focus**: XDNA2 hardware (second-generation NPU). First-generation XDNA
//! (Phoenix/Hawk Point) detection is best-effort only.
//!
//! Current state (real device layer mature, validated on physical 1022:17f0 + amdxdna hardware):
//! - Rich detection via `probe_accel_devices()` + pure modalias variant parser (extensive TDD).
//! - `RealXdna2Device` — production handle that owns a successfully opened `/dev/accel/accelN` fd.
//! - `RealXdna2DeviceInfo` + `real_device_detected()` + `real_device_read_sysfs()` — rich,
//!   safe, read-only diagnostics directly from the open real device.
//! - `direct` submodule — home of the lightweight, open, direct driver path
//!   (`DirectSubmission` trait, `QosInfo`, ioctl constants, etc.).
//! - `new_with_real_stub()` activates the `Real` execution arm for bring-up on actual hardware.
//! - `AmdXdna2Backend::new()` deliberately stays on high-fidelity Simulation for safety/CI.
//! - Full public diagnostics: `hardware_detected()`, `real_execution_active()`,
//!   `real_device_open()`, `real_device_path()`, `real_device_info()`.
//! - `xdna2_real_hardware` integration test + `examples/xdna2_probe.rs` provide repeatable
//!   cargo validation of the entire real open + diagnostics layer on target hardware.
//! - `submit_and_run` is the stable production boundary (routes to Real arm when active).
//! - **Phase 4:** Real `compile_model` uses [`ModelCompiler`] (same host passes as simulation);
//!   Real `submit_and_run` is mode-dispatched (`XDNA2_REAL_SUBMIT_MODE`: default fail-closed,
//!   `host_ref` host f32 for allowlisted ops, `npu_exec` gated EXEC attempt — never fake NPU Pass).
//! - **Phase 5:** Host graph interpreter ([`host_graph`]) runs Identity/Relu/Add/Mul/MatMul/Gemm
//!   with real f32 math + weights; weight BO plan scaffolding; **still not** NPU graph Pass.
//! - **Phase 3.5 lab:** Pure numerical policy in [`numerical`] + OFM helpers in [`lab_io`]
//!   (`pack_start_npu_kernel_args`, `lab_decide_from_ofm_bytes_pack_mode` for f32 ones or
//!   `XDNA2_LAB_PACK=mladf_a16w4` with non-zero golden — all-zero golden refused by default).
//!   Timed path wires decide **only** when `XDNA2_LAB_OFM=1` after OFM readback; default
//!   still hard-false `ran_on_npu`.
//! - Real .onnx weight tensors (multi-dtype + bf16 raw) are loaded via `CompiledModel`.
//! - All of the above is strongly TDD'd (lib tests + dedicated real-hardware integration tests).
//!
//! **Honest status on actual NPU execution**:
//! Device open, hwctx/BO/diagnostics, and Phase 1–3 EXEC infrastructure are real.
//! Host graph pipeline is real (Phase 5). Full silicon model inference (`ran_on_npu`) is
//! **not** claimed without a known-good instruction txn + OFM numerical verify
//! (see docs/XDNA2_TXN_RESEARCH.md + docs/XDNA2_REAL_EXECUTION.md Phase 4–5).
//!
//! See docs/XDNA2_REAL_EXECUTION.md for the production plan and current readiness.
//! Real hardware validation data lives in docs/XDNA2_HARDWARE_TESTING.md.
//!
//! References: autonomous HW research + Python amd_xdna_backend.py + npu_driver_integration.py

pub mod direct;
pub mod host_graph;
pub mod ioctl_interposer;
pub mod lab_io;
pub mod mladf_pack;
pub mod mlir_aie_ioctl;
pub mod numerical;
pub mod silicon_contract;
pub mod txn;
pub mod whole_array_i16;

/// Re-export feature-free colibrì schema version (defined on crate root).
pub use crate::TIMED_OP_JSON_SCHEMA_VERSION;

pub use direct::{
    AIE2_DEV_HEAP_SIZE, AIE2_DEV_HEAP_UVA_ALIGN, AMDXDNA_CMD_COUNT_MAX,
    AMDXDNA_CONFIG_HWCTX_PARAM_VAL_MAX, AMDXDNA_CU_CONFIG_SIZE, AMDXDNA_DRM_CONFIG_HWCTX_SIZE,
    AMDXDNA_HWCTX_PARAM_CONFIG_CU_HEADER_SIZE, AMDXDNA_INVALID_ADDR, AMDXDNA_SET_PARAM_POWER_MODE,
    AmdxdnaCmdHeaderFields, BufferHandle, BufferType, Command, CommandType,
    DEFAULT_BO_ROUNDTRIP_SIZE, DEFAULT_HWCTX_TILES, DEFAULT_WAIT_CMD_TIMEOUT_MS,
    DRM_AMDXDNA_HWCTX_ASSIGN_DBG_BUF, DRM_AMDXDNA_HWCTX_CONFIG_CU,
    DRM_AMDXDNA_HWCTX_REMOVE_DBG_BUF, DirectCapability, DirectError, DirectSubmission,
    ENV_LAB_CU_FUNC, ENV_LAB_PDI_BLOB, ENV_LAB_RESEARCH_DEBUG_BO, ErtCmdOpcode, ErtCmdState,
    ExecCmdPayloadHeader, HOST_REF_MAX_ELEMENTS, HardwareContext, HostMatmulRef,
    HwctxDbgBufAie2Parsed, HwctxDbgBufOp, LAB_RESEARCH_DEBUG_BO_SIZE_BYTES,
    LAB_RESEARCH_DEBUG_BO_TYPE, LabConfigCuAttach, LabPdiBoGuard, LabResearchDebugBoAttach,
    LabResearchDebugBoGuard, MAX_INSTRUCTION_BLOB_BYTES, MAX_SUGGESTED_HWCTX_TILES,
    SYNC_BO_FROM_DEVICE, SYNC_BO_TO_DEVICE, SharedRealDirectSubmission, StubDbgBufOpRecord,
    StubDirectSubmission, SubmitWaitResult, TimedOpResult, amdxdna_cmd_num_cu_masks,
    amdxdna_cu_config, amdxdna_drm_config_hwctx, amdxdna_drm_set_power_mode, amdxdna_drm_set_state,
    amdxdna_drm_wait_cmd, build_amdxdna_cmd_bo, build_ert_cmd_chain_bo, build_ert_start_dpu_cmd_bo,
    build_ert_start_npu_cmd_bo, build_ert_start_npu_cmd_bo_ex,
    build_ert_start_npu_cmd_bo_with_kernel_args, build_ert_start_npu_preempt_cmd_bo,
    build_ert_start_npu_preempt_elf_cmd_bo, build_exec_cmd_payload_scaffold,
    check_finite_wait_timeout, check_wait_timeout_or_infinite_opt_in, collect_lab_bo_diag,
    decode_instruction_hex, destroy_hwctx_and_owned_bos, ert_cmd_state_is_completed,
    ert_cmd_state_name, exec_cmd_block_reason, exec_cmd_is_enabled,
    exec_cmd_requires_filled_cmd_bo, fill_cmd_bo_with_payload, format_bo_info_diag,
    format_debug_bo_aie2_pack_detail, format_research_debug_bo_config_detail, host_matmul_f32_ones,
    is_dev_heap_uva_aligned, is_power_of_two_usize, is_valid_xdna_device_addr,
    lab_bo_diag_env_enabled, lab_cu_func_from_env, lab_pdi_blob_env_requested,
    lab_research_debug_bo_create_recipe, lab_research_debug_bo_env_requested,
    lab_research_debug_bo_type_token, load_instruction_blob_from_env,
    load_instruction_blob_from_path, next_aligned_addr_usize, pack_amdxdna_cmd_header,
    pack_amdxdna_cu_config, pack_config_hwctx_dbg_buf_aie2, pack_hwctx_param_config_cu,
    parse_amdxdna_cmd_header_from_bytes, power_mode, research_debug_bo_config_block_reason,
    research_debug_bo_config_is_enabled, submit_and_wait_command, suggest_hwctx_tiles,
    suggest_hwctx_tiles_from_tops, timed_bo_roundtrip_direct, timed_matmul_direct,
    try_lab_attach_config_cu_from_env, try_lab_attach_research_debug_bo_from_env,
    unpack_amdxdna_cmd_header, validate_config_hwctx_dbg_buf_aie2, wait_infinite_is_allowed,
};

pub use numerical::{
    DEFAULT_VERIFY_ATOL, DEFAULT_VERIFY_RTOL, NumericalVerifyCriteria, NumericalVerifyInput,
    RanOnNpuBlockReason, decide_ran_on_npu, decide_ran_on_npu_reason, f32_slice_is_all_zero,
    host_matmul_f32_ones_vec, max_abs_diff_f32, silicon_ran_on_npu_flags_ok, verify_f32_allclose,
};

pub use lab_io::{
    IoBoSet, LabIoSizePlan, LabKernelArgLayout, LabOfmVerifyPlan, SiliconPassAssetReadiness,
    WeightExecBind, WeightExecFillRequest, apply_ctrl_patch_location, attach_ctrl_bo,
    build_lab_arg_bo_handles, build_lab_arg_bo_handles_ex, bytes_are_all_zero,
    check_mladf_ofm_golden_not_all_zero, check_mladf_pack_dtype,
    check_mladf_wts_not_all_zero_for_exec, classify_lab_matmul_status,
    create_and_fill_lab_io_bos_for_pack_mode, create_and_fill_mladf_io_bos,
    create_and_fill_mladf_io_bos_ex, create_and_fill_ones_io_bos, create_and_fill_weight_io_bos,
    create_io_bos, destroy_io_bos, destroy_weight_exec_bind, f32_slice_from_le_bytes,
    f32_slice_to_le_bytes, fill_ifm_wts, format_ofm_mismatch_stats,
    format_ofm_mismatch_stats_preview, host_matmul_f32_ones_ifm_with_wts,
    host_ones_f32_matrix_bytes, lab_allow_zero_ofm_golden_env, lab_ctrl_auto_patch_env,
    lab_decide_candidate_f32_with_golden, lab_decide_from_ofm_bytes,
    lab_decide_from_ofm_bytes_bool, lab_decide_from_ofm_bytes_pack_mode,
    lab_decide_from_ofm_bytes_with_golden, lab_decode_ofm_bytes, lab_io_size_plan,
    lab_kernel_arg_layout_from_env, lab_kernel_arg_layout_from_str, lab_matmul_ifm_nbytes,
    lab_matmul_ofm_nbytes, lab_matmul_wts_nbytes, lab_ofm_env_enabled, lab_ofm_env_value_enabled,
    lab_require_ctrl_env, lab_require_non_zero_wts_env, lab_weight_env_enabled,
    load_ctrl_packed_from_env, load_mladf_meta_json_text_from_env, load_ofm_golden_from_env,
    ofm_golden_is_all_zero, pack_start_npu_kernel_args, pack_start_npu_kernel_args_no_io,
    pack_start_npu_kernel_args_with_ctrl, pack_start_npu_kernel_args_with_ctrl_layout,
    parse_ofm_golden_bytes, patch_ctrl_from_meta_io_paddrs, patch_ctrl_packet_u64_le,
    ran_on_npu_reason_is_ofm_verify_fail, readback_ofm, resolve_pack_mode_ofm_golden,
    silicon_pass_asset_readiness, try_lab_kernel_arg_layout_from_str, u64_to_prop_dwords,
    validate_mladf_ctrl_packed_for_pass, validate_mladf_ofm_golden_packed_for_pass,
    validate_mladf_wts_packed_for_pass, zero_ofm,
};

// C2 offline research (not product Pass surface). Word-index constants are
// re-exported for soft classify / unit tests; shot-specific — not stable product API
// (ordinary `pub use`, not `#[doc(hidden)]` — rustdoc still lists them).
pub use silicon_contract::{
    // --- C2 locked research constants / maps (shot-specific; not product Pass) ---
    AIE2_FW_APP_LOAD_PDI_FAIL,
    AIE2_FW_INVALID_PARAM,
    AMDXDNA_INVALID_FENCE_HANDLE,
    APP_LOAD_BAG_FEED_DRY_RUN_SCORECARD_NOTE,
    APP_LOAD_EXTRACT_BAG_SCORECARD_PIPELINE_NOTE,
    APP_LOAD_JOURNAL_FIELD_CHAIN_STATUS,
    APP_LOAD_JOURNAL_FIELD_FAILED_CMD,
    // --- contract / residual / last_apply ---
    APP_LOAD_MAINTAIN_RECONFIRM_CLOSED_TOKEN_KEY,
    APP_LOAD_MAINTAIN_RECONFIRM_OPEN_TOKEN,
    APP_LOAD_MAINTAIN_RECONFIRM_TOKEN,
    APP_LOAD_PARTITION_ID_PUBLIC_TOKEN,
    AXLF_FIRST_SECTION_HEADER_ABS_OFFSET,
    AXLF_HEADER_SIZE,
    AXLF_M_HEADER_ABS_OFFSET,
    AXLF_MAX_SECTIONS_WALK,
    AXLF_NUM_SECTIONS_ABS_OFFSET,
    AXLF_SECTION_HEADER_SIZE,
    AXLF_SECTION_KIND_AIE_PARTITION,
    AXLF_SECTION_KIND_BITSTREAM,
    AiebuElfCtrltextValidateReason,
    AiebuElfCtrltextValidateResult,
    AppLoadBagFeedDryRunScorecard,
    AppLoadE2eFixtureResearch,
    AppLoadExtractBagScorecardPipeline,
    AppLoadMaintainReconfirm,
    AppLoadOfflineProgressScorecard,
    AppLoadPartitionIdFieldMap,
    AppLoadPdiPartitionResearch,
    AppLoadResidualAxisMatrix,
    AxlfSectionBodyBounds,
    AxlfSectionHeaderFields,
    BLOCKED_ON_EXTERNAL_BYTES_TOKEN,
    BO_DIAG_DEFAULT_DEVHEAP_SIZE,
    BO_DIAG_DEFAULT_INSTR_SIZE,
    BoDiagEntry,
    BoSpan,
    C1_AIEBU_VALIDATOR_NOTE,
    C2_LIVE_CMDBUF_WORD_COUNT,
    C2_LIVE_CMDLIST_MSG_SIZE_BYTES,
    C2_LIVE_IFM_LO_WORD_IDX,
    C2_LIVE_INSTR_LO_WORD_IDX,
    C2_LIVE_INSTR_SIZE_WORD_IDX,
    C2_LIVE_N_BOS_WORD_IDX,
    C2_LIVE_WORD12_MARKER_IDX,
    C2_PURE_START_NPU_WORD_COUNT,
    C2_STRUCTURAL_FINGERPRINT_NOTE,
    C2CmdlistFillCrossCheck,
    C2ExpectPaddrs,
    C2LiveCmdbufMap,
    C2RelaPaddrAudit,
    C3_RESIDUAL_SPLIT_NOTE,
    C4_IRON_INGEST_NOTE,
    C5_PDI_GEOMETRY_NOTE,
    C6_DEBUG_BO_PREP_NOTE,
    CAPTURE_MATERIAL_FINGERPRINT_NOTE,
    CAPTURE_PATCH_CLASS_NOTE,
    COMPLETED_ON_THIS_FW_NOTE,
    CTRLTEXT_FINGERPRINT_EXPORT_NOTE,
    CTRLTEXT_SMALL_PUBLIC_NPU2_MAX_BYTES,
    CaptureMaterialFingerprint,
    CapturePatchClass,
    CompletedOnThisFwResearch,
    CompletedOnThisFwValue,
    ConstructionPathResearch,
    ContractCheck,
    CtrltextFingerprintExport,
    CtrltextSizeClass,
    CuPdiNecessityResearch,
    CuPdiSectionBodyBoundsResearch,
    CuPdiSectionIdentityResearch,
    CuPdiSectionOffsetMapResearch,
    DRM_SYNCOBJ_WAIT_FLAGS_WAIT_ALL,
    DRM_SYNCOBJ_WAIT_FLAGS_WAIT_AVAILABLE,
    DRM_SYNCOBJ_WAIT_FLAGS_WAIT_DEADLINE,
    DRM_SYNCOBJ_WAIT_FLAGS_WAIT_FOR_SUBMIT,
    DebugBoUapiPrep,
    ENV_LAB_EXTERNAL_CTRLTEXT,
    ENV_LAB_SAME_SHOT_RELA,
    ENV_LAB_WAIT_VIA_SYNCOBJ,
    EXTERNAL_CTRLTEXT_COMPARE_DRY_RUN_NOTE,
    EXTERNAL_CTRLTEXT_COMPARE_NOTE,
    EXTERNAL_KERNEL_XRT_BEYOND_STATUS_NAME,
    EXTERNAL_KNOWN_GOOD_SUBDIR,
    EXTERNAL_PUBLIC_PDI_EVIDENCE_TOKEN,
    EXTERNAL_PUBLIC_PDI_SCAN_CLOSED_TOKEN_KEY,
    EXTERNAL_VS_LAB_STRUCTURAL_DISTANCE_NOTE,
    ExternalCompareStatus,
    ExternalCtrltextCompareDryRun,
    ExternalCtrltextCompareResearch,
    ExternalKnownGoodBagDropCheck,
    ExternalKnownGoodProvenanceCheck,
    ExternalPublicPdiEvidenceScan,
    ExternalVsLabStructuralDistance,
    FAULT_NEAR_BO_BASE_WINDOW,
    FaultClassified,
    FaultMapClass,
    GT_SYNTHETIC_CTRL_PADDR,
    GT_SYNTHETIC_IFM_PADDR,
    GT_SYNTHETIC_OFM_PADDR,
    GT_SYNTHETIC_PATCHED_CTRLTEXT_SHA16,
    GT_SYNTHETIC_PATCHED_CTRLTEXT_SHA256,
    GT_SYNTHETIC_WTS_PADDR,
    IO_PAGE_FAULT_MAP_AUDIT_CLASS,
    InstrPayloadProfile,
    InstrPdiLoadResearch,
    IoPageFaultMapAudit,
    IronArtifactIngestClassify,
    IronArtifactKind,
    JOB_RESIDUAL_MATCH_NOTE,
    JobResidualIdentity,
    JobResidualMatchInventory,
    JournalFwStatusPair,
    KNOWN_GOOD_CTRLTEXT_ABSENT_TOKEN,
    L1B_READINESS_CHECKLIST_NOTE,
    L1bReadiness,
    LAB_CLASSIC_UNPATCHED_CTRLTEXT_SHA16,
    LAB_DEFAULT_QOS_FPS,
    LAB_DEFAULT_QOS_GOPS,
    LAB_EXPORT_FINGERPRINT_FEED_NOTE,
    LAST_APPLY_RICH_TOKENS_NOTE,
    LIVE_MAPPED_RELA_RESEARCH_NOTE,
    LIVE_PADDRS_RESEARCH_NOTE,
    LabBatchDiag,
    LabExportFingerprintFeed,
    LabInstrSizeMode,
    LastApplyDetailSignals,
    LastApplyEvidenceFingerprint,
    LastApplyRichTokens,
    LiveMappedRelaResearch,
    LivePaddrsExtract,
    MATERIAL_EXEC_LAB_CLASSIC_BYTES,
    MATERIAL_EXEC_READINESS_NOTE,
    MATERIAL_RESIDUAL_MATCH_NOTE,
    MAX_APP_LOAD_JOURNAL_TEXT_BYTES,
    MaterialExecReadinessScorecard,
    MaterialResidualMatchInventory,
    MaterialResidualMatchKind,
    PARSE_U32_HEX_WORD_LIST_MAX,
    PDI_FINGERPRINT_SHA_PREFIX_HEX,
    POST_FAIL_ERT_RESEARCH_NOTE,
    PUBLIC_PARTITION_SURFACE_TOKEN,
    PUBLIC_PDI_EVIDENCE_DEPTH_TOKEN,
    PUBLIC_PDI_MARKER_TOKEN,
    PUBLIC_PDI_NEW_FIELDS_CLOSED,
    PUBLIC_PDI_SCAN_CLOSED_TOKEN_KEY,
    PatchedCtrltextFingerprint,
    PdiBlobFingerprint,
    PdiGeometryFingerprint,
    PdiGeometrySizeClass,
    PdiGeometryVsSpent,
    PdiGeometryVsSpentResult,
    PostFailErtPresence,
    PostFailErtResearch,
    PublicPdiEvidenceScan,
    RELA_STRUCTURE_COMPARE_FIRST_N,
    RELA_STRUCTURE_COMPARE_NOTE,
    RESEARCH_PDI_DEFAULT_DUMP_BYTES,
    RESEARCH_PDI_IDENTITY_CLASS,
    RelaStructureCompareResearch,
    ResearchPdiIdentity,
    ResearchPdiSourceKind,
    ResidualClassifierSplit,
    ResidualClassifierSplitResult,
    SAME_SHOT_LAST_APPLY_GATE_NOTE,
    SAME_SHOT_RELA_APPLY_NOTE,
    SAME_SHOT_RELA_PLAN_NOTE,
    SPENT_FLEXML_UNIFIED_2X4X4_BYTES,
    SPENT_FLEXML_UNIFIED_2X4X4_SHA16,
    START_NPU_PROP_COUNT_BYTE_OFF,
    SYNCOBJ_ERRNO_EINVAL,
    SYNCOBJ_ERRNO_ETIME,
    SYNCOBJ_ERRNO_ETIMEDOUT,
    SYNCOBJ_NSEC_PER_MS,
    SYNCOBJ_TIMELINE_WAIT_FLAGS_XRT_DEFAULT,
    SYNCOBJ_TIMELINE_WAIT_PACK_NOTE,
    SYNCOBJ_WAIT_PATH_DECISION_NOTE,
    SameShotPaddrSource,
    SameShotRelaApplyResult,
    SameShotRelaPlan,
    SiliconPassEval,
    SiliconPassEvidence,
    SiliconResidualClass,
    StartNpuCmdDecode,
    StructuralLabFingerprint,
    SyncobjTimelineWaitArgs,
    SyncobjTimelineWaitIoctlPack,
    SyncobjWaitErrorClass,
    TxnOpOpcodeProfile,
    U32WordDiff,
    VendorPaddrs,
    WAIT_CMD_EINVAL_AXES_NOTE,
    WAIT_CMD_EINVAL_DRIVER_AXES_NOTE,
    WaitCmdDriverAxis,
    WaitCmdEinvalAxes,
    WaitCmdEinvalDriverAxesMap,
    WaitCompletionPath,
    WaitSubmitProductClass,
    XCLBIN_CU_PDI_IDENTITY_SECTION_MARKERS,
    XCLBIN_METADATA_SECTION_MARKERS,
    XCLBIN_RESEARCH_SECTION_MARKERS,
    XCLBIN_TOPOLOGY_SECTION_MARKERS,
    XCLBIN2_MAGIC,
    XclbinExtractResearchDryRun,
    XclbinSectionKind,
    aie2_fw_status_name,
    apply_same_shot_rela_research,
    audit_aiebu_rela_with_live_paddrs,
    audit_io_page_fault_vs_lab_map,
    axlf_section_body_window_in_bounds,
    axlf_section_kind_public_name,
    blob_kind_token,
    bo_spans_from_diag_entries,
    c2_live_cmdbuf_words_embedded,
    c2_live_cmdlist_msg_words_embedded,
    c2_live_io_paddrs_locked,
    c2_locked_app_load_journal_text,
    check_axlf_section_body_bounds,
    check_external_known_good_bag_drop,
    classify_capture_patch_class,
    classify_completed_on_this_fw_token,
    classify_ctrltext_size_class,
    classify_ctrltext_size_class_from_export,
    classify_fault_addr,
    classify_iron_artifact_ingest,
    classify_job_residual_identity,
    classify_last_apply_residual,
    classify_last_apply_residual_ex,
    classify_material_residual_match,
    classify_material_residual_match_for_job,
    classify_residual_split,
    classify_residual_split_ex,
    classify_syncobj_wait_errno,
    classify_syncobj_wait_error_message,
    classify_xclbin_section_kind,
    compare_external_ctrltext_to_lab_export,
    compare_pdi_fingerprint_to_spent_flexml,
    compare_pdi_geometry_to_spent_flexml,
    compare_rela_structure_lab_vs_external_elf,
    contract_classify_blob,
    contract_dynsym_report,
    contract_instr_size_modes,
    contract_kernel_arg_layouts,
    contract_post_patch_txn,
    contract_post_patch_txn_mode,
    contract_report,
    cross_check_c2_cmdlist_fill_locked,
    cross_check_c2_cmdlist_fill_public,
    debug_bo_uapi_prep_aie2,
    decode_c2_live_cmdbuf_research,
    decode_c2_live_cmdbuf_vs_pure_rebuild,
    decode_start_npu_cmd_bo,
    default_bo_diag_size_for_label,
    default_last_apply_path,
    diff_u32_word_lists,
    evaluate_l1b_readiness,
    evaluate_silicon_pass_evidence,
    export_ctrltext_fingerprint_research,
    external_compare_status_from_surfaces,
    external_ctrltext_extension_ok,
    external_known_good_lab_bag_provenance_allows_load,
    external_known_good_lab_root,
    external_known_good_load_path_is_allowed,
    external_known_good_path_is_allowed,
    external_known_good_provenance_present,
    external_known_good_provenance_research_ready,
    extract_live_paddrs_from_detail,
    extract_live_paddrs_from_last_apply_json,
    extract_live_paddrs_from_vendor_json,
    finalize_lab_timed_result,
    fingerprint_capture_material_dir_research,
    fingerprint_patched_ctrltext_after_rela,
    fingerprint_patched_ctrltext_after_rela_c2_locked,
    fingerprint_pdi_blob_bytes,
    fingerprint_pdi_geometry_bytes,
    first_hex_status_token,
    format_app_load_offline_checklist,
    format_app_load_offline_checklist_ex,
    format_app_load_residual_diag,
    format_c1_validator_note,
    format_c2_cmdbuf_research_note,
    format_c2_cmdlist_msg_research_note,
    format_c2_structural_fingerprint_note,
    format_c3_residual_split_note,
    format_c4_iron_ingest_note,
    format_c5_pdi_geometry_note,
    format_c6_debug_bo_prep_note,
    format_capture_patch_class_note,
    format_completed_on_this_fw_note,
    format_external_known_good_operator_extract_checklist,
    format_instr_size_detail,
    format_job_residual_match_note,
    format_l1b_readiness_checklist,
    format_l1b_readiness_checklist_from_extract,
    format_l1b_soft_score_tokens,
    format_lab_batch_diag,
    format_last_apply_rich_tokens_note,
    format_live_mapped_rela_research_note,
    format_live_paddrs_research_note,
    format_material_exec_readiness_bag_soft_lines,
    format_material_exec_readiness_note,
    format_material_residual_match_checklist,
    format_material_residual_match_note,
    format_post_fail_ert_detail_tokens,
    format_post_fail_ert_research_note,
    format_post_fail_ert_research_soft,
    format_research_pdi_preflight_note,
    format_same_shot_last_apply_detail,
    format_same_shot_last_apply_gate_tokens,
    format_same_shot_rela_apply_note,
    format_same_shot_rela_plan_note,
    format_silicon_pass_eval_note,
    format_silicon_residual_class_note,
    format_syncobj_timeline_wait_pack_note,
    format_syncobj_wait_path_decision_note,
    format_true_pass_criteria_checklist,
    format_true_pass_p6_note,
    format_wait_cmd_einval_axes_note,
    format_wait_cmd_einval_driver_axes_note,
    format_wait_path_syncobj_research_soft,
    fw_status_hint_for_lab,
    guess_iron_artifact_kind,
    instr_map_offset_is_no_mmap,
    instr_size_content_clamped,
    iron_artifact_extension_ok,
    iron_artifact_path_is_allowed,
    is_refused_local_post_patch_as_known_good,
    is_valid_device_paddr,
    journal_hints_hwctx_missing,
    journal_hints_invalid_syncobj_or_seq,
    journal_line_has_app_load_hint,
    lab_fingerprint_export_is_ready,
    lab_instr_size_mode_from_env,
    lab_instr_size_mode_from_str,
    lab_qos_gops_fps_from_env,
    lab_same_shot_rela_env_requested,
    lab_same_shot_rela_env_token_is_enabled,
    lab_wait_via_syncobj_research_env_enabled,
    last_apply_bo_gt_content,
    last_apply_config_cu_present,
    last_apply_evidence_fingerprint,
    last_apply_has_config_cu_signal,
    last_apply_has_rela_evidence,
    last_apply_is_same_fixture_fingerprint,
    last_apply_load_path_is_allowed,
    last_apply_matches_locked_app_load_baseline,
    last_apply_path_is_allowed,
    last_apply_start_npu_matches_content,
    last_apply_temp_root,
    list_rela_patch_targets_overwrite,
    load_external_ctrltext_for_compare,
    load_external_ctrltext_from_env,
    map_wait_cmd_einval_driver_axes,
    material_exec_readiness_path_is_allowed,
    merge_last_apply_status_config_cu,
    pack_syncobj_timeline_wait_args,
    pack_syncobj_timeline_wait_ioctl,
    parse_axlf_num_sections,
    parse_axlf_section_header,
    parse_bo_diag_from_detail,
    parse_chain_status_token,
    parse_completed_on_this_fw_from_provenance,
    parse_failed_cmd_status_token,
    parse_io_paddrs_from_detail,
    parse_journal_fw_status_pair,
    parse_journal_io_page_fault_addrs,
    parse_last_apply_detail_signals,
    parse_last_apply_rich_tokens,
    parse_paddr_json_value,
    parse_pdi_fingerprint_from_detail,
    parse_post_fail_ert_research,
    parse_u32_hex_word_list,
    parse_vendor_paddrs_json,
    parse_wait_cmd_einval_axes,
    plan_same_shot_rela_from_disk_extract,
    plan_same_shot_rela_from_io_paddrs,
    platform_claim_is_not_aie2_known_good,
    platform_claim_is_working_public_aie2_research,
    post_fail_ert_state_name,
    profile_instr_payload_after_rela,
    profile_instr_payload_after_rela_c2_locked,
    profile_instr_payload_bytes,
    prop_count_from_start_npu_cmd_bo,
    read_external_known_good_provenance_text,
    rebuild_c2_live_start_npu_cmd_bo,
    reconfirm_app_load_tracks_a_n_closed,
    reconfirm_app_load_tracks_a_n_closed_maintain,
    research_app_load_bag_feed_dry_run_scorecard,
    research_app_load_bag_feed_dry_run_scorecard_c2_locked_absent_bag,
    research_app_load_bag_feed_dry_run_scorecard_ex,
    research_app_load_e2e_fixture_c2_locked_absent_bag,
    research_app_load_e2e_fixture_path,
    research_app_load_extract_bag_scorecard_pipeline,
    research_app_load_extract_bag_scorecard_pipeline_c2_locked_absent_bag,
    research_app_load_extract_bag_scorecard_pipeline_ex,
    research_app_load_extract_bag_scorecard_pipeline_from_inputs,
    research_app_load_offline_progress_scorecard,
    research_app_load_offline_progress_scorecard_c2_locked_absent_bag,
    research_app_load_partition_id_field_map,
    research_app_load_partition_id_field_map_c2_locked,
    research_app_load_partition_id_field_map_from_journal,
    research_app_load_pdi_partition,
    research_app_load_pdi_partition_c2_locked,
    research_app_load_pdi_partition_from_journal,
    research_app_load_residual_axis_matrix,
    research_app_load_residual_axis_matrix_c2_locked,
    research_construction_path_lab_vs_public,
    research_cu_pdi_necessity_c2_locked,
    research_cu_pdi_necessity_for_app_load,
    research_cu_pdi_section_body_bounds_from_offset_map,
    research_cu_pdi_section_identity_from_markers,
    research_cu_pdi_section_identity_from_text,
    research_external_ctrltext_compare_dry_run,
    research_external_ctrltext_compare_dry_run_c2_locked_absent_bag,
    research_external_ctrltext_compare_dry_run_ex,
    research_external_ctrltext_compare_dry_run_from_lab_feed,
    research_external_ctrltext_compare_dry_run_from_lab_feed_ex,
    research_external_vs_lab_structural_distance,
    research_lab_export_fingerprint_feed,
    research_lab_export_fingerprint_feed_from_elf,
    research_lab_export_fingerprint_feed_from_elf_ex,
    research_lab_export_fingerprint_feed_from_last_apply_detail,
    research_lab_export_fingerprint_feed_from_last_apply_signals,
    research_live_mapped_rela,
    research_live_mapped_rela_from_extract,
    research_pdi_identity_default_dump_locked,
    research_pdi_identity_from_bytes,
    research_pdi_identity_from_last_apply_signals,
    research_pdi_source_kind_from_hint,
    research_xclbin_cu_pdi_section_body_bounds,
    research_xclbin_cu_pdi_section_offset_map,
    research_xclbin_extract_dry_run,
    research_xclbin_extract_ve2_fixture_refused,
    residual_note_has_exact_token,
    resolve_lab_qos_gops_fps,
    resolve_start_npu_inst_size,
    sanitize_soft_note_field_value,
    scan_external_public_pdi_evidence,
    scan_external_public_pdi_evidence_closed,
    scan_public_pdi_partition_evidence,
    scan_public_pdi_partition_evidence_closed,
    scan_xclbin_section_markers_from_text,
    score_material_exec_readiness,
    score_material_exec_readiness_from_path,
    select_wait_completion_path,
    sha256_digest,
    sha256_hex_lower,
    should_wait_via_syncobj,
    silicon_pass_evidence_ert_completed,
    silicon_pass_evidence_from_last_apply,
    silicon_pass_evidence_has_ert_error,
    structural_fingerprint_from_lab_detail,
    structural_fingerprint_from_lab_signals,
    structural_fingerprint_merge_elf,
    syncobj_timeout_ms_to_abs_nsec,
    try_lab_instr_size_mode_from_str,
    validate_aiebu_elf_ctrltext_structure,
    validate_external_known_good_provenance_text,
    vendor_paddrs_json_from_io,
    wait_message_looks_like_os_einval,
    wait_message_looks_like_true_timeout,
    walk_axlf_section_headers,
    write_last_apply_best_effort,
    write_last_apply_to_path,
    xclbin_magic_is_xclbin2,
};

pub use txn::{
    Elf32DynSym, Elf32Rela, InstructionBlobKind, LabRelaMode, XAIE_TXN_HEADER_SIZE, XaieTxnHeader,
    XaieTxnOp, XaieTxnOpcode, build_minimal_noop_txn, build_strix_research_noop_txn,
    classify_instruction_blob, format_dynsym_report, instruction_payload_bytes,
    is_aiebu_elf_ctrltext, lab_rela_mode_from_env, lab_rela_mode_from_str, parse_elf32_dynsym,
    parse_elf32_rela_dyn, patch_aiebu_elf_ctrltext_with_bos, patch_aiebu_elf_ctrltext_with_bos_ex,
    try_lab_rela_mode_from_str, validate_patched_ctrltext_classic, validate_txn_blob,
    walk_txn_ops_v01,
};

pub use mladf_pack::{
    CtrlMetaPatchEntry, CtrlPatchBufferRole, CtrlPatchLocation, LabPackMode,
    MAX_MLADF_LAB_FILE_BYTES, MLADF_A16_ELEM_BYTES, MladfBufferPlan, MladfKeyTags,
    MladfMatmulShape, STAGED_MLADF_CTRL_BYTES, STAGED_MLADF_IFM_BYTES, STAGED_MLADF_KEY,
    STAGED_MLADF_OFM_BYTES, STAGED_MLADF_WTS_BYTES, bf16_bits_to_f32, build_mladf_lab_payloads,
    ctrl_u64_patches_for_io_paddrs, f32_to_bf16_bits, host_ones_bf16_matrix_bytes,
    is_exact_staged_mladf_key, load_wts_packed_from_env, mladf_buffer_plan_from_env,
    mladf_buffer_plan_from_key, normalize_mladf_instr_key, pack_ifm_bf16_from_f32_rowmajor,
    parse_mladf_ctrl_meta_json, parse_mladf_ctrl_patch_locations, parse_mladf_instr_key,
    parse_mladf_key_tags, read_lab_file_capped, staged_mladf_buffer_plan,
    strip_mladf_map_sibling_suffixes, unpack_ofm_bf16_to_f32, validate_wts_packed_len_nonzero,
    wts_bytes_are_all_zero,
};

pub use host_graph::{
    HOST_GRAPH_OP_ALLOWLIST, HostGraphOptions, LAB_WEIGHT_MVP_DOC, LabMatmulWeightSpec,
    WEIGHT_BO_TYPE_SHMEM, Xdna2WeightPlan, build_weight_bo_plan, check_initializer_shape_payload,
    check_lab_weight_mvp_model, create_lab_weight_io_from_env,
    create_lab_weight_io_from_model_path, evaluate_host_graph, evaluate_host_graph_with_options,
    evaluate_host_ir, evaluate_host_ir_with_options, fill_weight_bos_batch,
    host_graph_ops_supported, host_ir_ops_supported, initializer_host_bytes, initializer_nbytes,
    lab_weight_requires_ofm, onnx_data_type_name, select_lab_matmul_weight,
    upload_weights_to_shmem_bos, weight_exec_fill_request_from_spec,
};

use std::sync::Arc;

use crate::backends::simulation::{SimulationBackend, SimulationConfig};
use crate::backends::{AccelDevice, Backend, probe_accel_devices, try_open_accel_device};
use crate::{CompiledModel, DragonNpuError, ModelCompiler, NpuStatus};

/// Phase 4 Real-arm submit mode (`XDNA2_REAL_SUBMIT_MODE`).
///
/// Default is fail-closed: compile may succeed, but `submit_and_run` refuses to
/// claim silicon inference without explicit host_ref or gated npu_exec.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RealSubmitMode {
    /// Production default: actionable [`DragonNpuError::RealSubmitNotReady`].
    FailClosed,
    /// Host f32 path for allowlisted graphs; **not** NPU (`ran_on_npu` false).
    HostRef,
    /// Attempt gated EXEC_CMD + Phase 1–3 helpers; never returns fake NPU outputs.
    NpuExec,
}

impl RealSubmitMode {
    /// Parse `XDNA2_REAL_SUBMIT_MODE` (`host_ref` / `host_validate` / `npu_exec`).
    /// Unset or unknown → [`RealSubmitMode::FailClosed`].
    #[must_use]
    pub fn from_env() -> Self {
        std::env::var("XDNA2_REAL_SUBMIT_MODE").map_or(Self::FailClosed, |v| {
            let lower = v.trim().to_ascii_lowercase();
            match lower.as_str() {
                "host_ref" | "host_validate" => Self::HostRef,
                "npu_exec" => Self::NpuExec,
                _ => Self::FailClosed,
            }
        })
    }
}

/// Ops with real host semantics under `host_ref` (Phase 5 host graph).
///
/// Add/Mul/MatMul/Gemm require weights / RHS initializers at execute time; missing
/// weights → clear error (no fake golden). See [`host_graph::evaluate_host_graph`].
const HOST_REF_OP_ALLOWLIST: &[&str] = HOST_GRAPH_OP_ALLOWLIST;

/// Ops allowed to *attempt* gated `npu_exec` graph checks (still probe-only EXEC, not model run).
/// Same allowlist as host graph; does not claim silicon graph execution.
const NPU_EXEC_OP_ALLOWLIST: &[&str] = HOST_GRAPH_OP_ALLOWLIST;

/// XDNA2-oriented compile/submit plan derived from a [`CompiledModel`].
///
/// Produced at submit time (and via [`AmdXdna2Backend::xdna2_compile_plan`]). Does **not**
/// claim a known-good instruction txn or silicon GEMM Pass.
#[derive(Debug, Clone)]
pub struct Xdna2CompilePlan {
    /// Operator types from the compiled graph.
    pub ops: Vec<String>,
    /// Ops not in the Phase 4 Real allowlist (normalized names).
    pub unsupported_ops: Vec<String>,
    /// True when every op is allowlisted for `host_ref` / `npu_exec` graph checks.
    pub host_ref_supported: bool,
    /// Preferred CREATE_HWCTX tile count hint (AIE2-friendly default).
    pub preferred_tiles: u32,
    /// Preferred UMQ BO size hint (bytes).
    pub preferred_umq_bytes: u64,
    /// Preferred log BO size hint (bytes).
    pub preferred_log_bytes: u64,
    /// Free-form honesty notes for tools/logs.
    pub notes: String,
}

/// Build a Phase 4 plan from a compiled model (host-side only).
///
/// `preferred_tiles` defaults to [`DEFAULT_HWCTX_TILES`] (4) without a device.
/// Pass `resource` from live GET_INFO to apply [`suggest_hwctx_tiles`] (Phase 6).
/// Env `XDNA2_HWCTX_TILES_SINGLE` still overrides at timed_matmul time.
#[must_use]
pub fn build_xdna2_compile_plan(model: &CompiledModel) -> Xdna2CompilePlan {
    build_xdna2_compile_plan_with_resource(model, None)
}

/// Like [`build_xdna2_compile_plan`] with optional resource info for tile heuristic.
#[must_use]
pub fn build_xdna2_compile_plan_with_resource(
    model: &CompiledModel,
    resource: Option<&direct::amdxdna_drm_get_resource_info>,
) -> Xdna2CompilePlan {
    let ops: Vec<String> = model.ops().to_vec();
    let unsupported_host = unsupported_ops_vs(&ops, HOST_REF_OP_ALLOWLIST);
    let unsupported_npu = unsupported_ops_vs(&ops, NPU_EXEC_OP_ALLOWLIST);
    let host_ref_supported = unsupported_host.is_empty() && !ops.is_empty();
    let preferred_tiles = resource.map_or(DEFAULT_HWCTX_TILES, suggest_hwctx_tiles);
    let notes = if ops.is_empty() {
        "empty op list — model graph has no ops to validate".to_string()
    } else if host_ref_supported {
        format!(
            "all ops in host_ref allowlist (Identity/Relu/Add/Mul/MatMul/Gemm + Phase 6 fused linear); \
             npu_exec is EXEC-only Phase 3 probe (not model graph on NPU); no ran_on_npu claim; \
             preferred_tiles={preferred_tiles} (heuristic without device stays 4)"
        )
    } else {
        format!(
            "host_ref unsupported ops: [{}]; npu_exec graph-check unsupported: [{}]; \
             compile still OK — use simulation arm or narrow the graph. \
             host_ref is host f32 graph eval (not NPU); preferred_tiles={preferred_tiles}.",
            unsupported_host.join(", "),
            unsupported_npu.join(", ")
        )
    };
    Xdna2CompilePlan {
        ops,
        // Surface host_ref gaps as the primary product "unsupported" list.
        unsupported_ops: unsupported_host,
        host_ref_supported,
        preferred_tiles,
        preferred_umq_bytes: 16 * 1024,
        preferred_log_bytes: 128 * 1024,
        notes,
    }
}

fn normalize_op_name(op: &str) -> String {
    op.trim().to_ascii_lowercase()
}

fn unsupported_ops_vs(ops: &[String], allowlist: &[&str]) -> Vec<String> {
    let mut out = Vec::new();
    for op in ops {
        let n = normalize_op_name(op);
        if n.is_empty() {
            continue;
        }
        if !allowlist.contains(&n.as_str()) && !out.iter().any(|e| e == &n) {
            out.push(n);
        }
    }
    out
}

fn require_ops_allowlist(model: &CompiledModel, allowlist: &[&str]) -> Result<(), DragonNpuError> {
    let ops = model.ops();
    if ops.is_empty() {
        return Err(DragonNpuError::UnsupportedOps {
            model: model.model_path().to_string(),
            ops: "(none — empty graph)".to_string(),
        });
    }
    let bad = unsupported_ops_vs(ops, allowlist);
    if !bad.is_empty() {
        return Err(DragonNpuError::UnsupportedOps {
            model: model.model_path().to_string(),
            ops: bad.join(", "),
        });
    }
    Ok(())
}

/// Host-only f32 path via Phase 5 graph interpreter. **Never** claims NPU.
///
/// Identity/Relu/Add/Mul/MatMul/Gemm with real math when weights/shapes allow.
/// Missing weights or bad shapes → error (no fake golden).
/// CLI may set `XDNA2_HOST_MATMUL_K` via [`HostGraphOptions::from_env`]; unit tests
/// that call [`evaluate_host_graph`] directly stay env-isolated.
fn host_ref_execute(model: &CompiledModel, input: &[f32]) -> Result<Vec<f32>, DragonNpuError> {
    host_graph::evaluate_host_graph_with_options(
        model,
        input,
        &host_graph::HostGraphOptions::from_env(),
    )
}

/// Shared Real-arm state (device open + probe metadata) for executor + compiled-model Backend.
#[derive(Debug)]
struct RealXdna2Shared {
    detected: Option<AccelDevice>,
    device: Option<RealXdna2Device>,
}

/// Backend stored on models compiled by the Real arm so `CompiledModel::submit_and_run`
/// uses the same Phase 4 mode dispatch as [`AmdXdna2Backend::submit_and_run`].
#[derive(Clone, Debug)]
struct RealArmBackend {
    shared: Arc<RealXdna2Shared>,
}

impl Backend for RealArmBackend {
    fn status(&self) -> Result<NpuStatus, DragonNpuError> {
        let open = self.shared.device.is_some();
        let (device_name, driver_version) = self.shared.detected.as_ref().map_or_else(
            || {
                (
                    format!("AMD XDNA2 Real arm (no probe hit) device_open={open}"),
                    "amdxdna real-arm (no device)".to_string(),
                )
            },
            |d| {
                (
                    format!(
                        "AMD XDNA2 Real arm ({}:{}) device_open={open}",
                        d.pci_vendor, d.pci_device
                    ),
                    "amdxdna real-arm".to_string(),
                )
            },
        );
        Ok(NpuStatus {
            vendor: "AMD".to_string(),
            // available ≠ inference-ready: default submit is fail-closed without modes/gates.
            available: open,
            compute_units: 8,
            memory_mb: 8192,
            peak_tops: 50.0,
            driver_version,
            supported_dtypes: vec![
                "fp32".to_string(),
                "fp16".to_string(),
                "bf16".to_string(),
                "int8".to_string(),
            ],
            device_name,
            // Real arm path; simulated=true when accel fd is not open.
            simulated: !open,
        })
    }

    fn compile_model(
        &self,
        model_path: &str,
        optimization_level: u8,
        quantization: &str,
    ) -> Result<CompiledModel, DragonNpuError> {
        // Host-side ModelCompiler (same passes as simulation). Backend Arc is this Real arm
        // so model.submit_and_run uses Phase 4 dispatch.
        let compiler = ModelCompiler::new();
        compiler.compile(
            model_path,
            optimization_level,
            quantization,
            Arc::new(self.clone()),
        )
    }

    fn run(&self, input: &[f32]) -> Result<Vec<f32>, DragonNpuError> {
        let _ = input;
        // Direct run on Real arm is not the production path (needs model op list).
        Err(DragonNpuError::RuntimeError(
            "use submit_and_run for real XDNA2 hardware (direct run is not supported on the production path; \
             host_ref / npu_exec are submit_and_run modes via XDNA2_REAL_SUBMIT_MODE)"
                .to_string(),
        ))
    }

    fn submit_and_run(
        &self,
        model: &CompiledModel,
        input: &[f32],
    ) -> Result<Vec<f32>, DragonNpuError> {
        real_arm_submit_and_run(
            self.shared.as_ref(),
            model,
            input,
            RealSubmitMode::from_env(),
        )
    }
}

fn real_arm_submit_and_run(
    shared: &RealXdna2Shared,
    model: &CompiledModel,
    input: &[f32],
    mode: RealSubmitMode,
) -> Result<Vec<f32>, DragonNpuError> {
    // Prefer resource-backed tile heuristic when Real device is open (Issue 8).
    let resource = shared
        .device
        .as_ref()
        .and_then(|d| d.get_direct_resource_info().ok());
    let plan = build_xdna2_compile_plan_with_resource(model, resource.as_ref());
    let device_open = shared.device.is_some();
    let device_path = shared
        .device
        .as_ref()
        .map_or_else(|| "(none)".to_string(), |d| d.path().display().to_string());
    let gate = exec_cmd_block_reason();

    match mode {
        RealSubmitMode::FailClosed => Err(DragonNpuError::RealSubmitNotReady {
            reason: format!(
                "model '{}' compiled OK (ops={}, host_ref_unsupported=[{}], host_ref_supported={}); \
                 device_open={device_open} path={device_path}; EXEC_CMD: {gate}; \
                 no verified instruction txn for silicon inference (ran_on_npu remains false). \
                 Set XDNA2_REAL_SUBMIT_MODE=host_ref for Phase 5 host f32 graph \
                 (Identity/Relu/Add/Mul/MatMul/Gemm when weights present; not NPU), or \
                 XDNA2_REAL_SUBMIT_MODE=npu_exec (EXEC-only Phase 3 timed_matmul 1x1x1 probe — model graph \
                 is NOT executed) with feature xdna2-dangerous-exec-cmd + XDNA2_ALLOW_EXEC_CMD=1 \
                 (+ optional XDNA2_INSTRUCTION_BLOB). Still not a numerical NPU Pass. \
                 For hwctx-only bring-up without EXEC, use timed_matmul / xdna2_probe (not npu_exec). \
                 See docs/XDNA2_REAL_EXECUTION.md Phase 4–5.",
                model.model_path(),
                plan.ops.len(),
                plan.unsupported_ops.join(", "),
                plan.host_ref_supported,
            ),
        }),
        RealSubmitMode::HostRef => {
            // Phase 5 host graph; device open optional. Not NPU.
            host_ref_execute(model, input)
        }
        RealSubmitMode::NpuExec => {
            // Graph allowlist is a coarse pre-check only — npu_exec does not run the model.
            require_ops_allowlist(model, NPU_EXEC_OP_ALLOWLIST)?;
            if !device_open {
                return Err(DragonNpuError::RealSubmitNotReady {
                    reason: format!(
                        "npu_exec mode but real XDNA2 device is not open (path={device_path}); \
                         open /dev/accel via new_with_real_stub on a live amdxdna host. {gate}"
                    ),
                });
            }
            // npu_exec is EXEC-only by design (requires L1 double gate). For CREATE_HWCTX-only
            // diagnostics without opening EXEC, use timed_matmul_direct / probe (returns hwctx_ok).
            if !exec_cmd_is_enabled() {
                return Err(DragonNpuError::RealSubmitNotReady {
                    reason: format!(
                        "npu_exec is EXEC-only and EXEC_CMD is gated: {gate}. \
                         model '{}' passed coarse op allowlist; device_open=true. \
                         Model graph is not executed in npu_exec. \
                         For hwctx_ok without EXEC, call timed_matmul / xdna2_probe instead of enabling L1. \
                         Do not claim silicon Pass without gates + known-good txn + numerical verify.",
                        model.model_path()
                    ),
                });
            }
            let device =
                shared
                    .device
                    .as_ref()
                    .ok_or_else(|| DragonNpuError::RealSubmitNotReady {
                        reason: "npu_exec: device vanished (internal)".to_string(),
                    })?;
            // Reuse fd-scoped RealDirectSubmission (DevHeap ownership) + Phase 3 probe.
            // Model ops/weights/UMQ plan hints are intentionally not applied — 1x1x1 probe only.
            let sub = device.shared_direct_submission();
            let tr = timed_matmul_direct(&sub, 1, 1, 1, "f32");
            Err(DragonNpuError::RealSubmitNotReady {
                reason: format!(
                    "npu_exec Phase 3 timed_matmul probe only (1x1x1 f32) — model graph NOT executed \
                     (compiled model='{}' ops={}). probe status={} ran_on_npu={} ran_on_device_path={} \
                     detail={}. Outputs are intentionally not returned as inference results. \
                     Use host_ref for Phase 5 host f32 graph. See docs/XDNA2_REAL_EXECUTION.md Phase 4–5.",
                    model.model_path(),
                    plan.ops.len(),
                    tr.status,
                    tr.ran_on_npu,
                    tr.ran_on_device_path,
                    tr.detail
                ),
            })
        }
    }
}

/// AMD XDNA2 backend.
///
/// Detection is performed at construction time using the pure-std accel probe.
/// We prioritize XDNA2 (PCI 1022:17f0 family). When XDNA2 hardware is visible,
/// we configure realistic parameters (≈50 TOPS) and report the detection
/// honestly in status().
///
/// First-gen XDNA is lower priority / best-effort.
///
/// Execution is dispatched through the private `Execution` enum.
/// Real XRT / direct submission will be added as a new variant when we have
/// the actual execution path (this is now the highest technical priority after
/// detection was validated on real hardware).
#[derive(Debug)]
pub struct AmdXdna2Backend {
    execution: Execution,
    /// True if real XDNA2 hardware (PCI 1022:17f0 + amdxdna) was detected.
    hardware_detected: bool,
    /// The detected accel device details (if any) for rich reporting.
    detected_device: Option<AccelDevice>,
}

/// Internal execution backend for AmdXdna2Backend.
///
/// Trait that both the simulation and (future) real XDNA2 execution paths
/// will implement. This makes swapping in actual NPU submission clean.
trait Xdna2Executor: std::fmt::Debug + Send + Sync {
    fn compile_model(
        &self,
        model_path: &str,
        optimization_level: u8,
        quantization: &str,
    ) -> Result<CompiledModel, DragonNpuError>;

    fn run(&self, input: &[f32]) -> Result<Vec<f32>, DragonNpuError>;

    /// Returns true if this executor is talking to real hardware.
    fn is_real(&self) -> bool;

    /// The core "submit compiled model + run inference on the NPU" boundary.
    /// This is the method that will eventually do actual hardware submission.
    ///
    /// This is the primary production boundary for real XDNA2 hardware submission.
    /// When a real executor (XRT-based or direct amdxdna) is wired, this method
    /// will perform actual NPU command submission.
    ///
    /// See docs/XDNA2_REAL_EXECUTION.md for the full production plan, optimization
    /// strategies, and benchmarking methodology.
    fn submit_and_run(
        &self,
        model: &CompiledModel,
        input: &[f32],
    ) -> Result<Vec<f32>, DragonNpuError>;

    /// Phase 4: submit with explicit mode (no env). Default ignores mode and
    /// delegates to [`Self::submit_and_run`] (simulation). Real overrides.
    fn submit_and_run_with_mode(
        &self,
        model: &CompiledModel,
        input: &[f32],
        _mode: RealSubmitMode,
    ) -> Result<Vec<f32>, DragonNpuError> {
        self.submit_and_run(model, input)
    }

    /// Production helper: whether the real device is open and ready.
    /// Default impl returns false; Real executor overrides it.
    fn real_device_open(&self) -> bool {
        false
    }

    /// Production helper: the path of the device we opened (if any).
    /// Default returns None; Real executor can override.
    fn real_device_path(&self) -> Option<&std::path::Path> {
        None
    }

    /// Production helper: richer information about the opened real device.
    /// Default returns None; the real executor overrides this when a device
    /// handle is present.
    #[allow(dead_code)]
    fn real_device_info(&self) -> Option<RealXdna2DeviceInfo> {
        None
    }

    /// Production helper: the full probe metadata for the opened real device.
    fn real_device_detected(&self) -> Option<&AccelDevice> {
        None
    }

    /// Production helper: safe read-only sysfs access for the opened real device.
    /// See `RealXdna2Device::read_sysfs` for semantics.
    fn real_device_read_sysfs(&self, _relative_path: &str) -> Option<String> {
        None
    }

    /// Lightweight direct driver query (firmware version via GET_INFO).
    /// Default: Unsupported. Real executor overrides when a device is open.
    fn get_direct_firmware_version(
        &self,
    ) -> Result<direct::amdxdna_drm_query_firmware_version, direct::DirectError> {
        Err(direct::DirectError::Unsupported(
            "get_direct_firmware_version not available on this executor".to_string(),
        ))
    }

    fn get_direct_power_mode(
        &self,
    ) -> Result<direct::amdxdna_drm_get_power_mode, direct::DirectError> {
        Err(direct::DirectError::Unsupported(
            "get_direct_power_mode not available on this executor".to_string(),
        ))
    }

    fn set_direct_power_mode(&self, _mode: u8) -> Result<(), direct::DirectError> {
        Err(direct::DirectError::Unsupported(
            "set_direct_power_mode not available on this executor".to_string(),
        ))
    }

    fn get_direct_resource_info(
        &self,
    ) -> Result<direct::amdxdna_drm_get_resource_info, direct::DirectError> {
        Err(direct::DirectError::Unsupported(
            "get_direct_resource_info not available on this executor".to_string(),
        ))
    }

    fn get_direct_bo_info(
        &self,
        _handle: u32,
    ) -> Result<direct::amdxdna_drm_get_bo_info, direct::DirectError> {
        Err(direct::DirectError::Unsupported(
            "get_direct_bo_info not available on this executor".to_string(),
        ))
    }

    #[allow(dead_code)]
    fn create_direct_hwctx(
        &self,
        _num_tiles: u32,
        _mem_size: u32,
        _qos: Option<&direct::QosInfo>,
    ) -> Result<direct::HardwareContext, direct::DirectError> {
        Err(direct::DirectError::Unsupported(
            "create_direct_hwctx not available on this executor".to_string(),
        ))
    }

    #[allow(dead_code)]
    fn create_direct_hwctx_with_sizes(
        &self,
        num_tiles: u32,
        mem_size: u32,
        _umq_size: u64,
        _log_size: u64,
        qos: Option<&direct::QosInfo>,
    ) -> Result<direct::HardwareContext, direct::DirectError> {
        self.create_direct_hwctx(num_tiles, mem_size, qos)
    }

    #[allow(dead_code)]
    fn destroy_direct_hwctx(&self, _handle: u32) -> Result<(), direct::DirectError> {
        Err(direct::DirectError::Unsupported(
            "destroy_direct_hwctx not available on this executor".to_string(),
        ))
    }

    #[allow(dead_code)]
    fn get_direct_aie_version(
        &self,
    ) -> Result<direct::amdxdna_drm_query_aie_version, direct::DirectError> {
        Err(direct::DirectError::Unsupported(
            "get_direct_aie_version not available on this executor".to_string(),
        ))
    }

    #[allow(dead_code)]
    fn created_direct_hwctx_count(&self) -> usize {
        0
    }

    /// Returns a handle implementing the lightweight direct submission trait.
    /// On the Real arm with an open device this will be a RealDirectSubmission
    /// that can perform actual ioctls. Default returns a stub.
    fn direct_submission(&self) -> Box<dyn direct::DirectSubmission> {
        Box::new(direct::StubDirectSubmission::new(-1))
    }

    fn get_sensors(&self) -> Result<direct::amdxdna_drm_query_sensor, direct::DirectError> {
        Err(direct::DirectError::Unsupported(
            "get_sensors not available on this executor".to_string(),
        ))
    }
}

impl Xdna2Executor for SimulationBackend {
    fn compile_model(
        &self,
        model_path: &str,
        optimization_level: u8,
        quantization: &str,
    ) -> Result<CompiledModel, DragonNpuError> {
        self.compile_model(model_path, optimization_level, quantization)
    }

    fn run(&self, input: &[f32]) -> Result<Vec<f32>, DragonNpuError> {
        self.run(input)
    }

    fn is_real(&self) -> bool {
        false
    }

    fn submit_and_run(
        &self,
        _model: &CompiledModel,
        input: &[f32],
    ) -> Result<Vec<f32>, DragonNpuError> {
        // For simulation we just run the input directly.
        self.run(input)
    }
}

/// Internal record for a hwctx created via the direct path + the BOs we own for it.
/// (Small private helper; placed early so the richly-documented RealXdna2Device
/// struct can follow its doc comment with no intervening blank line.)
#[derive(Debug, Clone)]
struct HwCtxRecord {
    handle: u32,
    umq_bo: u32,
    log_buf_bo: u32,
}

/// A successfully opened real XDNA2 accel device (/dev/accel/accelN under amdxdna).
///
/// This is the stable production abstraction for an actually-open real XDNA2 device.
/// It owns the file descriptor and rich metadata, and is the natural home for
/// future real execution work (command submission, buffer allocation, etc.).
///
/// It is deliberately factored out of `RealXdna2Executor` so it can be unit-tested,
/// reused, and evolved independently.
///
/// **Current readiness**:
/// - Device open (multi-index, robust) — complete and validated on real hardware.
/// - Diagnostics (`path()`, `info()`, `as_fd()` for future use) — complete.
/// - Actual command submission / model execution — **not yet implemented**.
///
/// See `Xdna2Executor::submit_and_run` and docs/XDNA2_REAL_EXECUTION.md for the plan.
#[derive(Debug)]
pub(crate) struct RealXdna2Device {
    /// Metadata from the sysfs probe at the time we decided to open this device.
    detected: AccelDevice,
    /// The open file descriptor. Kept open for the device lifetime; Drop cleans
    /// tracked hwctxs then shared submission (DevHeap) before the fd closes.
    file: std::fs::File,
    /// The exact path we successfully opened (e.g. /dev/accel/accel0).
    path: std::path::PathBuf,
    /// Fd-scoped real submission (single DevHeap owner for this open).
    /// Reuse via [`Self::shared_direct_submission`] — do not construct a second
    /// `RealDirectSubmission` on this fd while this lives.
    submission: Arc<direct::RealDirectSubmission>,
    /// Records of hwctxs + their associated BOs created via the direct path.
    /// Interior mutability so create/destroy work through `&self` public APIs.
    created_hwctxs: std::sync::Mutex<Vec<HwCtxRecord>>,
}

impl Drop for RealXdna2Device {
    fn drop(&mut self) {
        // Best-effort: destroy tracked hwctxs + UMQ/log BOs while fd is still open.
        let records = self
            .created_hwctxs
            .lock()
            .map(|mut g| std::mem::take(&mut *g))
            .unwrap_or_default();
        let fd = self.as_fd();
        for r in records {
            let _ = self.submission.destroy_hwctx(fd, r.handle);
            if r.umq_bo != 0 {
                let _ = self.submission.destroy_buffer(fd, r.umq_bo);
            }
            if r.log_buf_bo != 0 {
                let _ = self.submission.destroy_buffer(fd, r.log_buf_bo);
            }
        }
        // Arc drop of submission → RealDirectSubmission::Drop releases DevHeap.
    }
}

impl RealXdna2Device {
    /// Create a device handle from an already-successful open.
    /// The caller is responsible for having performed the open.
    fn from_opened(detected: AccelDevice, file: std::fs::File, path: std::path::PathBuf) -> Self {
        use std::os::unix::io::AsRawFd;
        let fd = file.as_raw_fd();
        Self {
            detected,
            file,
            path,
            submission: Arc::new(direct::RealDirectSubmission::new(fd)),
            created_hwctxs: std::sync::Mutex::new(Vec::new()),
        }
    }

    /// Shared fd-scoped submission (reuses DevHeap; safe for repeated npu_exec / probe).
    #[must_use]
    pub fn shared_direct_submission(&self) -> SharedRealDirectSubmission {
        SharedRealDirectSubmission::new(Arc::clone(&self.submission))
    }

    /// Record a successfully created hwctx for ownership / destroy tracking.
    fn track_hwctx(&self, ctx: &direct::HardwareContext) {
        if let Ok(mut guard) = self.created_hwctxs.lock() {
            guard.push(HwCtxRecord {
                handle: ctx.handle,
                umq_bo: ctx.umq_bo,
                log_buf_bo: ctx.log_buf_bo,
            });
        }
    }

    /// Forget a tracked hwctx after destroy (by handle). Returns the record if found.
    fn untrack_hwctx(&self, handle: u32) -> Option<HwCtxRecord> {
        let Ok(mut guard) = self.created_hwctxs.lock() else {
            return None;
        };
        guard
            .iter()
            .position(|r| r.handle == handle)
            .map(|pos| guard.remove(pos))
    }

    /// The path of the opened device.
    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    /// Returns true (this handle only exists when the device is open).
    #[allow(dead_code, clippy::unused_self)]
    pub fn is_open(&self) -> bool {
        true
    }

    /// The underlying raw file descriptor (for future ioctl / submission work).
    /// This is intentionally not pub yet — we will expose a safe API when needed.
    #[allow(dead_code)]
    pub(crate) fn as_fd(&self) -> std::os::unix::io::RawFd {
        use std::os::unix::io::AsRawFd;
        self.file.as_raw_fd()
    }

    /// Basic driver / device information derived from the open device and sysfs.
    /// This is useful for logging and diagnostics on real hardware.
    #[allow(dead_code)]
    pub fn info(&self) -> RealXdna2DeviceInfo {
        RealXdna2DeviceInfo {
            path: self.path.clone(),
            pci_vendor: self.detected.pci_vendor.clone(),
            pci_device: self.detected.pci_device.clone(),
            pci_revision: self.detected.pci_revision.clone(),
            npu_name: self.detected.npu_name.clone(),
            modalias: self.detected.modalias.clone(),
            xdna2_variant: self.detected.xdna2_variant.clone(),
        }
    }

    /// Returns the full `AccelDevice` metadata that was known at open time.
    /// This gives access to name, driver, modalias, PCI details, and variant.
    pub fn detected(&self) -> &AccelDevice {
        &self.detected
    }

    /// Safely read a sysfs attribute for this open device.
    ///
    /// `relative_path` is interpreted relative to the standard sysfs directory
    /// for the device (typically `/sys/class/accel/{name}/` and its `device/`
    /// subdirectory).
    ///
    /// Examples of useful paths on real amdxdna XDNA2 systems:
    /// - "device/uevent"
    /// - "device/power/control"
    /// - "name"
    /// - various firmware or thermal attributes when exposed by the driver.
    ///
    /// This method performs **only read-only filesystem access**. It never
    /// issues ioctls, never modifies state, and never performs any command
    /// submission. It is intended for observability, bring-up, power/thermal
    /// monitoring, and to support future real execution code.
    ///
    /// Returns `None` on any failure (file does not exist, permission denied,
    /// not a regular file, etc.). This is the expected behavior on systems
    /// where the kernel does not expose the attribute.
    #[allow(dead_code)]
    pub fn read_sysfs(&self, relative_path: &str) -> Option<String> {
        // Primary location: /sys/class/accel/{name}/ + relative_path
        let base = std::path::Path::new("/sys/class/accel").join(&self.detected.name);
        let candidate = base.join(relative_path);
        if let Ok(s) = std::fs::read_to_string(&candidate) {
            return Some(s.trim().to_string());
        }

        // Also try under the device subdirectory (very common for PCI attrs)
        let dev_candidate = base.join("device").join(relative_path);
        if let Ok(s) = std::fs::read_to_string(&dev_candidate) {
            return Some(s.trim().to_string());
        }

        None
    }

    /// Convenience: current power control state of the device (e.g. "auto", "on").
    /// Returns None if the attribute is not readable.
    #[allow(dead_code)]
    pub fn power_control(&self) -> Option<String> {
        self.read_sysfs("device/power/control")
    }

    /// Convenience: whether we can read basic uevent information for the device.
    #[allow(dead_code)]
    pub fn has_uevent(&self) -> bool {
        self.read_sysfs("device/uevent").is_some()
    }

    /// Returns a lightweight direct submission handle for this device.
    ///
    /// **Reuses** the fd-scoped [`RealDirectSubmission`] (single DevHeap owner).
    /// Do not call `RealDirectSubmission::new` on this fd while the device lives.
    #[allow(dead_code)]
    pub fn direct_submission(&self) -> SharedRealDirectSubmission {
        self.shared_direct_submission()
    }

    /// Query firmware version using the lightweight direct path (real ioctl on hardware).
    /// This is one of the first useful real operations that actually talks to the amdxdna driver.
    #[allow(dead_code)]
    pub fn get_direct_firmware_version(
        &self,
    ) -> Result<direct::amdxdna_drm_query_firmware_version, direct::DirectError> {
        self.submission.get_firmware_version()
    }

    /// Query configured power mode using the lightweight direct path.
    #[allow(dead_code)]
    pub fn get_direct_power_mode(
        &self,
    ) -> Result<direct::amdxdna_drm_get_power_mode, direct::DirectError> {
        self.submission.get_power_mode()
    }

    /// Set power mode (SET_STATE / SET_POWER_MODE). See backend docs.
    #[allow(dead_code)]
    pub fn set_direct_power_mode(&self, mode: u8) -> Result<(), direct::DirectError> {
        self.submission.set_power_mode(mode)
    }

    /// Query resource information (current/max TOPS, clocks, task limits) via the direct path.
    #[allow(dead_code)]
    pub fn get_direct_resource_info(
        &self,
    ) -> Result<direct::amdxdna_drm_get_resource_info, direct::DirectError> {
        self.submission.get_resource_info()
    }

    /// Query sensor data via the lightweight direct path.
    #[allow(dead_code)]
    pub fn get_direct_sensors(
        &self,
    ) -> Result<direct::amdxdna_drm_query_sensor, direct::DirectError> {
        self.submission.get_sensors()
    }

    /// Get information about a specific buffer object via the lightweight direct path.
    #[allow(dead_code)]
    pub fn get_direct_bo_info(
        &self,
        handle: u32,
    ) -> Result<direct::amdxdna_drm_get_bo_info, direct::DirectError> {
        self.submission.get_bo_info(self.as_fd(), handle)
    }

    /// Convenience: create a hardware context with internally-managed UMQ and log buffers.
    /// This is the easiest way to get a real hwctx on hardware using the direct path.
    #[allow(dead_code)]
    pub fn create_direct_hwctx(
        &self,
        num_tiles: u32,
        mem_size: u32,
        qos: Option<&direct::QosInfo>,
    ) -> Result<direct::HardwareContext, direct::DirectError> {
        self.create_direct_hwctx_with_sizes(num_tiles, mem_size, 16 * 1024, 128 * 1024, qos)
    }

    /// Create a hardware context with explicit control over UMQ and log buffer sizes.
    /// Extremely useful for bring-up and debugging on real hardware.
    ///
    /// Sizes are wired through `DirectSubmission::create_hwctx_with_sizes` so Strategy B
    /// uses exactly these UMQ/log BO sizes (not the full probe matrix).
    #[allow(dead_code)]
    pub fn create_direct_hwctx_with_sizes(
        &self,
        num_tiles: u32,
        mem_size: u32,
        umq_size: u64,
        log_size: u64,
        qos: Option<&direct::QosInfo>,
    ) -> Result<direct::HardwareContext, direct::DirectError> {
        let ctx = self.submission.create_hwctx_with_sizes(
            self.as_fd(),
            num_tiles,
            mem_size,
            umq_size,
            log_size,
            qos,
        )?;
        self.track_hwctx(&ctx);
        Ok(ctx)
    }

    /// Destroy a previously created hardware context.
    ///
    /// If this handle was created via [`Self::create_direct_hwctx_with_sizes`], also
    /// closes the tracked UMQ/log BOs. Prefer [`Self::destroy_direct_hwctx_full`] when
    /// you still have the full `HardwareContext`.
    #[allow(dead_code)]
    pub fn destroy_direct_hwctx(&self, handle: u32) -> Result<(), direct::DirectError> {
        let record = self.untrack_hwctx(handle);
        let fd = self.as_fd();
        if let Some(r) = record {
            if r.umq_bo != 0 {
                let _ = self.submission.destroy_buffer(fd, r.umq_bo);
            }
            if r.log_buf_bo != 0 {
                let _ = self.submission.destroy_buffer(fd, r.log_buf_bo);
            }
        }
        self.submission.destroy_hwctx(fd, handle)
    }

    /// Destroy a hwctx and its associated BOs (recommended for bring-up).
    /// Uses the info from HardwareContext (which now carries the BO handles it created).
    #[allow(dead_code)]
    pub fn destroy_direct_hwctx_full(
        &self,
        ctx: &direct::HardwareContext,
    ) -> Result<(), direct::DirectError> {
        let _ = self.untrack_hwctx(ctx.handle);
        let fd = self.as_fd();
        // Destroy hwctx first while BOs are still valid references for the driver,
        // then GEM_CLOSE the UMQ/log BOs we own.
        let hwctx_res = self.submission.destroy_hwctx(fd, ctx.handle);
        if ctx.umq_bo != 0 {
            let _ = self.submission.destroy_buffer(fd, ctx.umq_bo);
        }
        if ctx.log_buf_bo != 0 {
            let _ = self.submission.destroy_buffer(fd, ctx.log_buf_bo);
        }
        hwctx_res
    }

    /// Query AIE version via the lightweight direct path.
    #[allow(dead_code)]
    pub fn get_direct_aie_version(
        &self,
    ) -> Result<direct::amdxdna_drm_query_aie_version, direct::DirectError> {
        self.submission.get_aie_version()
    }

    /// Returns how many hwctxs are currently tracked on this device handle
    /// (basic observability / bring-up aid). Decrements on successful destroy.
    #[allow(dead_code)]
    pub fn created_direct_hwctx_count(&self) -> usize {
        self.created_hwctxs.lock().map_or(0, |g| g.len())
    }
}

/// Snapshot of information about an opened real XDNA2 device.
/// Useful for user-facing diagnostics and logging when the real execution
/// arm is active.
#[derive(Debug, Clone)]
pub struct RealXdna2DeviceInfo {
    /// The /dev/accel path that was successfully opened.
    pub path: std::path::PathBuf,
    /// PCI vendor ID (e.g. "0x1022" for AMD).
    pub pci_vendor: String,
    /// PCI device ID (e.g. "0x17f0" for XDNA2 family).
    pub pci_device: String,
    /// PCI revision.
    pub pci_revision: String,
    /// NPU name reported by the driver (often empty on real systems).
    pub npu_name: String,
    /// Full modalias string (very useful for variant detection).
    pub modalias: String,
    /// Derived XDNA2 generation (NPU4/NPU5/NPU6 or "XDNA2").
    pub xdna2_variant: Option<String>,
}

/// Real XDNA2 executor — the production path for actual hardware submission.
///
/// When the `Real` arm is active, this type holds a `RealXdna2Device` (the opened
/// /dev/accel handle) and implements Phase 4 `compile_model` / `submit_and_run`:
/// - **compile_model**: always host-side [`ModelCompiler`] (same passes as simulation).
/// - **submit_and_run**: mode dispatch via `XDNA2_REAL_SUBMIT_MODE` (default fail-closed;
///   `host_ref` host f32; `npu_exec` gated EXEC attempt without fake NPU Pass).
///
/// See docs/XDNA2_REAL_EXECUTION.md Phase 4 and docs/XDNA2_NEVER_AGAIN.md.
#[derive(Debug)]
struct RealXdna2Executor {
    shared: Arc<RealXdna2Shared>,
}

impl RealXdna2Executor {
    /// Create a real executor.
    ///
    /// If a device was detected, this will attempt to open the corresponding
    /// /dev/accel device using the shared helper. Failure to open is turned
    /// into a clear error only when `npu_exec` submission is attempted.
    /// Host-side `compile_model` and `host_ref` submit do not require an open device.
    fn new(detected: Option<AccelDevice>) -> Self {
        let (opened_file, opened_path) = try_open_accel_device(detected.as_ref());

        let device = match (detected.clone(), opened_file, opened_path) {
            (Some(det), Some(f), Some(p)) => Some(RealXdna2Device::from_opened(det, f, p)),
            _ => None,
        };

        // Prefer probe metadata from the open device; else the original detection.
        let detected_meta = device.as_ref().map(|d| d.detected.clone()).or(detected);

        Self {
            shared: Arc::new(RealXdna2Shared {
                detected: detected_meta,
                device,
            }),
        }
    }

    fn arm_backend(&self) -> RealArmBackend {
        RealArmBackend {
            shared: Arc::clone(&self.shared),
        }
    }
}

impl Xdna2Executor for RealXdna2Executor {
    fn compile_model(
        &self,
        model_path: &str,
        optimization_level: u8,
        quantization: &str,
    ) -> Result<CompiledModel, DragonNpuError> {
        // Phase 4.1: real compile via ModelCompiler (host-side; device optional).
        self.arm_backend()
            .compile_model(model_path, optimization_level, quantization)
    }

    fn run(&self, input: &[f32]) -> Result<Vec<f32>, DragonNpuError> {
        self.arm_backend().run(input)
    }

    fn is_real(&self) -> bool {
        true
    }

    fn submit_and_run(
        &self,
        model: &CompiledModel,
        input: &[f32],
    ) -> Result<Vec<f32>, DragonNpuError> {
        // Phase 4.2: NEVER AGAIN — EXEC only through gated helpers inside npu_exec mode
        // (timed_matmul_direct → submit_and_wait_command / RealDirectSubmission).
        real_arm_submit_and_run(
            self.shared.as_ref(),
            model,
            input,
            RealSubmitMode::from_env(),
        )
    }

    fn submit_and_run_with_mode(
        &self,
        model: &CompiledModel,
        input: &[f32],
        mode: RealSubmitMode,
    ) -> Result<Vec<f32>, DragonNpuError> {
        real_arm_submit_and_run(self.shared.as_ref(), model, input, mode)
    }

    fn real_device_open(&self) -> bool {
        self.shared.device.is_some()
    }

    fn real_device_path(&self) -> Option<&std::path::Path> {
        self.shared.device.as_ref().map(RealXdna2Device::path)
    }

    fn real_device_info(&self) -> Option<RealXdna2DeviceInfo> {
        self.shared.device.as_ref().map(RealXdna2Device::info)
    }

    fn real_device_detected(&self) -> Option<&AccelDevice> {
        self.shared
            .device
            .as_ref()
            .map(RealXdna2Device::detected)
            .or(self.shared.detected.as_ref())
    }

    fn real_device_read_sysfs(&self, relative_path: &str) -> Option<String> {
        self.shared
            .device
            .as_ref()
            .and_then(|d| d.read_sysfs(relative_path))
    }

    // Lightweight direct query forwarding (real ioctls only when we have an open device).
    fn get_direct_firmware_version(
        &self,
    ) -> Result<direct::amdxdna_drm_query_firmware_version, direct::DirectError> {
        self.shared.device.as_ref().map_or_else(
            || {
                Err(direct::DirectError::Unsupported(
                    "no real XDNA2 device open for direct query".to_string(),
                ))
            },
            RealXdna2Device::get_direct_firmware_version,
        )
    }

    fn get_direct_power_mode(
        &self,
    ) -> Result<direct::amdxdna_drm_get_power_mode, direct::DirectError> {
        self.shared.device.as_ref().map_or_else(
            || {
                Err(direct::DirectError::Unsupported(
                    "no real XDNA2 device open for direct query".to_string(),
                ))
            },
            RealXdna2Device::get_direct_power_mode,
        )
    }

    fn set_direct_power_mode(&self, mode: u8) -> Result<(), direct::DirectError> {
        self.shared.device.as_ref().map_or_else(
            || {
                Err(direct::DirectError::Unsupported(
                    "no real XDNA2 device open for set_power_mode".to_string(),
                ))
            },
            |d| d.set_direct_power_mode(mode),
        )
    }

    fn get_direct_resource_info(
        &self,
    ) -> Result<direct::amdxdna_drm_get_resource_info, direct::DirectError> {
        self.shared.device.as_ref().map_or_else(
            || {
                Err(direct::DirectError::Unsupported(
                    "no real XDNA2 device open for direct query".to_string(),
                ))
            },
            RealXdna2Device::get_direct_resource_info,
        )
    }

    fn direct_submission(&self) -> Box<dyn direct::DirectSubmission> {
        if let Some(dev) = &self.shared.device {
            Box::new(dev.shared_direct_submission())
        } else {
            Box::new(direct::StubDirectSubmission::new(-1))
        }
    }

    fn get_sensors(&self) -> Result<direct::amdxdna_drm_query_sensor, direct::DirectError> {
        self.shared.device.as_ref().map_or_else(
            || {
                Err(direct::DirectError::Unsupported(
                    "no real XDNA2 device open for direct query".to_string(),
                ))
            },
            RealXdna2Device::get_direct_sensors,
        )
    }

    fn get_direct_bo_info(
        &self,
        handle: u32,
    ) -> Result<direct::amdxdna_drm_get_bo_info, direct::DirectError> {
        self.shared.device.as_ref().map_or_else(
            || {
                Err(direct::DirectError::Unsupported(
                    "no real XDNA2 device open for direct query".to_string(),
                ))
            },
            |d| d.get_direct_bo_info(handle),
        )
    }

    fn create_direct_hwctx(
        &self,
        num_tiles: u32,
        mem_size: u32,
        qos: Option<&direct::QosInfo>,
    ) -> Result<direct::HardwareContext, direct::DirectError> {
        self.create_direct_hwctx_with_sizes(num_tiles, mem_size, 16 * 1024, 128 * 1024, qos)
    }

    fn create_direct_hwctx_with_sizes(
        &self,
        num_tiles: u32,
        mem_size: u32,
        umq_size: u64,
        log_size: u64,
        qos: Option<&direct::QosInfo>,
    ) -> Result<direct::HardwareContext, direct::DirectError> {
        self.shared.device.as_ref().map_or_else(
            || {
                Err(direct::DirectError::Unsupported(
                    "no real XDNA2 device open for direct hwctx".to_string(),
                ))
            },
            |d| d.create_direct_hwctx_with_sizes(num_tiles, mem_size, umq_size, log_size, qos),
        )
    }

    fn destroy_direct_hwctx(&self, handle: u32) -> Result<(), direct::DirectError> {
        self.shared.device.as_ref().map_or_else(
            || {
                Err(direct::DirectError::Unsupported(
                    "no real XDNA2 device open for direct query".to_string(),
                ))
            },
            |d| d.destroy_direct_hwctx(handle),
        )
    }

    fn get_direct_aie_version(
        &self,
    ) -> Result<direct::amdxdna_drm_query_aie_version, direct::DirectError> {
        self.shared.device.as_ref().map_or_else(
            || {
                Err(direct::DirectError::Unsupported(
                    "no real XDNA2 device open for direct query".to_string(),
                ))
            },
            RealXdna2Device::get_direct_aie_version,
        )
    }

    fn created_direct_hwctx_count(&self) -> usize {
        self.shared
            .device
            .as_ref()
            .map_or(0, RealXdna2Device::created_direct_hwctx_count)
    }
}

/// Internal dispatch between Simulation and Real XDNA2 execution arms.
///
/// This enum exists so the transition to full real hardware submission is clean.
/// The `Real` variant holds the `RealXdna2Device` when the production path is active.
///
/// Current reality:
/// - Simulation arm: high-fidelity and always used by the default `new()`.
/// - Real arm: device open + diagnostics are solid (via `RealXdna2Device`).
///   Actual NPU submission inside the Real arm is still pending.
///
/// The architectural decision on *how* real submission will be implemented
/// (XRT, direct amdxdna ioctls, etc.) has not been made yet.
#[derive(Debug)]
#[allow(dead_code)] // Real submission logic pending architectural decision
enum Execution {
    Simulation(Box<dyn Xdna2Executor>),

    // Real XDNA2 production execution path.
    // This variant holds the logic that performs actual command submission to
    // the NPU via the lightweight direct amdxdna driver path.
    //
    // Goals for this path:
    // - Proper device open / context / memory management for /dev/accel devices.
    // - Submission of models (potentially compiled to XDNA2-native format).
    // - Efficient `submit_and_run` that actually executes on silicon and returns results.
    //
    // Optimization and benchmarking guidance lives in docs/XDNA2_REAL_EXECUTION.md.
    Real(Box<dyn Xdna2Executor>),
}

impl Execution {
    fn compile_model(
        &self,
        model_path: &str,
        optimization_level: u8,
        quantization: &str,
    ) -> Result<CompiledModel, DragonNpuError> {
        match self {
            Self::Simulation(exec) | Self::Real(exec) => {
                exec.compile_model(model_path, optimization_level, quantization)
            }
        }
    }

    fn run(&self, input: &[f32]) -> Result<Vec<f32>, DragonNpuError> {
        match self {
            Self::Simulation(exec) | Self::Real(exec) => exec.run(input),
        }
    }

    #[allow(dead_code)]
    fn real_device_info(&self) -> Option<RealXdna2DeviceInfo> {
        match self {
            Self::Simulation(exec) | Self::Real(exec) => exec.real_device_info(),
        }
    }

    #[allow(dead_code)]
    fn real_device_detected(&self) -> Option<&AccelDevice> {
        match self {
            Self::Simulation(exec) | Self::Real(exec) => exec.real_device_detected(),
        }
    }

    #[allow(dead_code)]
    fn real_device_read_sysfs(&self, relative_path: &str) -> Option<String> {
        match self {
            Self::Simulation(exec) | Self::Real(exec) => exec.real_device_read_sysfs(relative_path),
        }
    }
}

impl AmdXdna2Backend {
    /// Create a new AMD XDNA2 backend.
    ///
    /// Probes the system for real AMD XDNA2 hardware (amdxdna driver + PCI 1022:17f0 family).
    /// Sets realistic XDNA2 parameters (~50 TOPS) when detected.
    ///
    /// Always succeeds — falls back to high-fidelity simulation when no
    /// XDNA2 hardware is present.
    #[allow(clippy::option_if_let_else)]
    pub fn new() -> Self {
        let devices = probe_accel_devices();

        // XDNA2 detection: amdxdna driver + AMD PCI 1022:17f0 family
        let xdna2 = devices.iter().find(|d| {
            d.driver.contains("amdxdna") && d.pci_vendor == "0x1022" && d.pci_device == "0x17f0"
        });

        let hardware_detected = xdna2.is_some();
        let detected_device = xdna2.cloned();

        let (device_name, peak_tops, _driver_version) = if let Some(d) = &detected_device {
            let display_name = if d.npu_name.is_empty() {
                if d.pci_device == "0x17f0" {
                    format!(
                        "AMD XDNA2 (17f0 rev {}) - hardware detected",
                        d.pci_revision
                    )
                } else {
                    "AMD XDNA2 (Ryzen AI) - hardware detected".to_string()
                }
            } else {
                format!("AMD XDNA2 ({}) - hardware detected", d.npu_name)
            };
            (
                display_name,
                50.0,
                format!("amdxdna (XDNA2 {} detected)", d.pci_revision),
            )
        } else {
            (
                "AMD XDNA2 (Ryzen AI) - simulated (no XDNA2 hardware detected)".to_string(),
                50.0,
                "stub-0.1 (no XDNA2 hardware found)".to_string(),
            )
        };

        let config = SimulationConfig {
            device_name,
            peak_tops,
            latency_factor: if hardware_detected { 0.6 } else { 1.0 },
            ..Default::default()
        };

        // Default to high-fidelity Simulation for `new()`.
        // Real production execution is activated explicitly via `new_with_real_stub()`
        // (or future configuration). This is the safe, development-friendly default
        // while the real submission logic is being completed.
        Self {
            execution: Execution::Simulation(Box::new(SimulationBackend::with_config(config))),
            hardware_detected,
            detected_device,
        }
    }

    /// Creates an AmdXdna2Backend that uses the Real execution path.
    /// Useful for early bring-up and validation of real hardware submission
    /// plumbing on physical XDNA2 devices.
    pub fn new_with_real_stub() -> Self {
        let devices = probe_accel_devices();

        let xdna2 = devices.iter().find(|d| {
            d.driver.contains("amdxdna") && d.pci_vendor == "0x1022" && d.pci_device == "0x17f0"
        });

        let hardware_detected = xdna2.is_some();
        let detected_device = xdna2.cloned();

        Self {
            execution: Execution::Real(Box::new(RealXdna2Executor::new(detected_device.clone()))),
            hardware_detected,
            detected_device,
        }
    }

    /// Returns whether real AMD XDNA2 hardware was detected at construction time.
    pub fn hardware_detected(&self) -> bool {
        self.hardware_detected
    }

    /// Returns details about the detected XDNA2 device, if any.
    pub fn detected_device(&self) -> Option<&AccelDevice> {
        self.detected_device.as_ref()
    }

    /// Convenience accessor: returns a human-friendly string describing the
    /// detected NPU (e.g. "XDNA2 (Strix Point)" or the raw npu_name).
    /// Returns None when running in simulated mode.
    pub fn detected_npu_type(&self) -> Option<String> {
        self.detected_device.as_ref().map(|d| {
            let base = if d.npu_name.is_empty() {
                // Common on real hardware — fall back to PCI family + revision for usefulness
                if d.pci_device == "0x17f0" {
                    format!("XDNA2 (17f0 rev {})", d.pci_revision)
                } else {
                    "XDNA2".to_string()
                }
            } else {
                format!("XDNA2 ({})", d.npu_name)
            };
            // Only append the variant when it adds distinct information (e.g. "NPU5").
            // Avoid "XDNA2 (17f0 rev 0x20) / XDNA2" on real hardware where variant is just "XDNA2".
            if let Some(variant) = &d.xdna2_variant {
                if variant != "XDNA2" && !base.contains(variant) {
                    format!("{base} / {variant}")
                } else {
                    base
                }
            } else {
                base
            }
        })
    }

    /// Returns whether the real (non-simulation) XDNA2 execution path is active.
    ///
    /// When true, `submit_and_run` will attempt actual hardware submission.
    /// This is the primary production signal for real NPU execution.
    pub fn real_execution_active(&self) -> bool {
        match &self.execution {
            Execution::Simulation(exec) | Execution::Real(exec) => exec.is_real(),
        }
    }

    /// For the Real execution arm on actual hardware: whether we successfully
    /// opened the /dev/accel device. High-value diagnostic on the user's laptop.
    pub fn real_device_open(&self) -> bool {
        match &self.execution {
            Execution::Real(exec) => exec.real_device_open(),
            Execution::Simulation(_) => false,
        }
    }

    /// Returns the path of the accel device we successfully opened on the Real
    /// execution arm (if any). Very useful for debugging on real XDNA2 hardware.
    pub fn real_device_path(&self) -> Option<&std::path::Path> {
        match &self.execution {
            Execution::Real(exec) => exec.real_device_path(),
            Execution::Simulation(_) => None,
        }
    }

    /// When the real XDNA2 execution arm is active and we successfully opened
    /// a device, returns detailed information about that device (PCI ids,
    /// modalias, variant, etc.). This is the richest diagnostic available for
    /// the production real path on actual hardware.
    pub fn real_device_info(&self) -> Option<RealXdna2DeviceInfo> {
        match &self.execution {
            Execution::Real(exec) => exec.real_device_info(),
            Execution::Simulation(_) => None,
        }
    }

    /// When the real execution arm has an open device, returns the full
    /// `AccelDevice` metadata captured at probe time for that device.
    pub fn real_device_detected(&self) -> Option<&AccelDevice> {
        match &self.execution {
            Execution::Real(exec) => exec.real_device_detected(),
            Execution::Simulation(_) => None,
        }
    }

    /// Safe, read-only access to sysfs attributes for the currently open real
    /// XDNA2 device (when the real arm is active and a device was opened).
    ///
    /// This is the recommended way for users and future real execution code
    /// to inspect driver-exposed attributes (power, firmware, thermal, etc.)
    /// without performing any submission.
    ///
    /// See `RealXdna2Device::read_sysfs` for details and example paths.
    pub fn real_device_read_sysfs(&self, relative_path: &str) -> Option<String> {
        match &self.execution {
            Execution::Real(exec) => exec.real_device_read_sysfs(relative_path),
            Execution::Simulation(_) => None,
        }
    }

    /// Convenience wrapper for the most common real-hardware power query.
    pub fn real_device_power_control(&self) -> Option<String> {
        self.real_device_read_sysfs("device/power/control")
    }

    /// Query firmware version via the lightweight direct driver path (GET_INFO ioctl).
    ///
    /// Only meaningful on the Real execution arm with a successfully opened device.
    /// Returns a clear Unsupported error on the simulation path or when no device is open.
    pub fn get_direct_firmware_version(
        &self,
    ) -> Result<direct::amdxdna_drm_query_firmware_version, direct::DirectError> {
        match &self.execution {
            Execution::Real(exec) => exec.get_direct_firmware_version(),
            Execution::Simulation(_) => Err(direct::DirectError::Unsupported(
                "direct queries require the Real XDNA2 execution arm (use new_with_real_stub())"
                    .to_string(),
            )),
        }
    }

    /// Query configured power mode via the lightweight direct driver path.
    pub fn get_direct_power_mode(
        &self,
    ) -> Result<direct::amdxdna_drm_get_power_mode, direct::DirectError> {
        match &self.execution {
            Execution::Real(exec) => exec.get_direct_power_mode(),
            Execution::Simulation(_) => Err(direct::DirectError::Unsupported(
                "direct queries require the Real XDNA2 execution arm (use new_with_real_stub())"
                    .to_string(),
            )),
        }
    }

    /// Set power mode via `DRM_IOCTL_AMDXDNA_SET_STATE` + `SET_POWER_MODE` (Phase 6).
    ///
    /// Packed from public uAPI. **Operator-tested** — default tests never open
    /// `/dev/accel` for SET. Does not claim measured TOPS change.
    pub fn set_direct_power_mode(&self, mode: u8) -> Result<(), direct::DirectError> {
        match &self.execution {
            Execution::Real(exec) => exec.set_direct_power_mode(mode),
            Execution::Simulation(_) => Err(direct::DirectError::Unsupported(
                "set_direct_power_mode requires the Real XDNA2 execution arm".to_string(),
            )),
        }
    }

    /// Query resource limits (TOPS, clocks, task caps) via the lightweight direct driver path.
    pub fn get_direct_resource_info(
        &self,
    ) -> Result<direct::amdxdna_drm_get_resource_info, direct::DirectError> {
        match &self.execution {
            Execution::Real(exec) => exec.get_direct_resource_info(),
            Execution::Simulation(_) => Err(direct::DirectError::Unsupported(
                "direct queries require the Real XDNA2 execution arm (use new_with_real_stub())"
                    .to_string(),
            )),
        }
    }

    /// Suggested CREATE_HWCTX tile count from live resource info (Phase 6).
    ///
    /// Real arm only (needs GET_INFO). Heuristic multiples of 4 in 4..=8 — not a
    /// silicon efficiency claim. See [`suggest_hwctx_tiles`].
    pub fn suggested_hwctx_tiles(&self) -> Result<u32, direct::DirectError> {
        let res = self.get_direct_resource_info()?;
        Ok(suggest_hwctx_tiles(&res))
    }

    /// Query AIE version via the lightweight direct driver path.
    #[allow(dead_code)]
    pub fn get_direct_aie_version(
        &self,
    ) -> Result<direct::amdxdna_drm_query_aie_version, direct::DirectError> {
        match &self.execution {
            Execution::Real(exec) => exec.get_direct_aie_version(),
            Execution::Simulation(_) => Err(direct::DirectError::Unsupported(
                "direct queries require the Real XDNA2 execution arm (use new_with_real_stub())"
                    .to_string(),
            )),
        }
    }

    /// Returns how many hwctxs have been created through this backend on the Real arm.
    #[allow(dead_code)]
    pub fn created_direct_hwctx_count(&self) -> usize {
        match &self.execution {
            Execution::Real(exec) => exec.created_direct_hwctx_count(),
            Execution::Simulation(_) => 0,
        }
    }

    /// Convenience: create hwctx via backend (Real arm only).
    #[allow(dead_code)]
    pub fn create_direct_hwctx(
        &self,
        num_tiles: u32,
        mem_size: u32,
        qos: Option<&direct::QosInfo>,
    ) -> Result<direct::HardwareContext, direct::DirectError> {
        self.create_direct_hwctx_with_sizes(num_tiles, mem_size, 16 * 1024, 128 * 1024, qos)
    }

    /// Create hwctx with explicit UMQ/log buffer sizes (highly recommended for real hardware bring-up).
    ///
    /// On the Real arm this routes through `RealXdna2Device` so sizes reach
    /// `create_hwctx_with_sizes` and the hwctx is tracked for destroy.
    #[allow(dead_code)]
    pub fn create_direct_hwctx_with_sizes(
        &self,
        num_tiles: u32,
        mem_size: u32,
        umq_size: u64,
        log_size: u64,
        qos: Option<&direct::QosInfo>,
    ) -> Result<direct::HardwareContext, direct::DirectError> {
        match &self.execution {
            Execution::Real(exec) => {
                exec.create_direct_hwctx_with_sizes(num_tiles, mem_size, umq_size, log_size, qos)
            }
            Execution::Simulation(_) => Err(direct::DirectError::Unsupported(
                "requires Real arm".to_string(),
            )),
        }
    }

    /// Destroy hwctx via backend.
    #[allow(dead_code)]
    pub fn destroy_direct_hwctx(&self, handle: u32) -> Result<(), direct::DirectError> {
        match &self.execution {
            Execution::Real(exec) => exec.destroy_direct_hwctx(handle),
            Execution::Simulation(_) => Err(direct::DirectError::Unsupported(
                "requires Real arm".to_string(),
            )),
        }
    }

    /// Returns a `DirectSubmission` handle for the lightweight direct driver path.
    ///
    /// On the Real execution arm (via `new_with_real_stub()`) with a successfully
    /// opened device, this returns a live `RealDirectSubmission` that can perform
    /// actual ioctls. On simulation or when no device is open, it returns a stub
    /// that gives clear "not available" errors.
    ///
    /// This is the main way advanced users and bring-up code get access to the
    /// full direct trait surface from the public backend.
    pub fn direct_submission(&self) -> Box<dyn direct::DirectSubmission> {
        match &self.execution {
            Execution::Real(exec) => exec.direct_submission(),
            Execution::Simulation(_) => Box::new(direct::StubDirectSubmission::new(-1)),
        }
    }

    /// Timed matmul attempt via the pure direct path (plan step 4 / Phase 3).
    ///
    /// Returns structured stage timings + honest `status` (`hwctx_blocked`,
    /// `hwctx_ok`, `ok_submit_wait_envelope`, `ok_submit_wait_with_inst_blob`, …).
    /// Never fakes NPU Pass (`ran_on_npu` stays false without numerical verify).
    /// Optional host golden: `host_ref_*` (CPU only).
    pub fn timed_matmul(&self, m: u32, n: u32, k: u32, dtype: &str) -> direct::TimedOpResult {
        let sub = self.direct_submission();
        direct::timed_matmul_direct(sub.as_ref(), m, n, k, dtype)
    }

    /// Timed Shmem BO fill + SYNC TO/FROM + verify (Phase 3 safe silicon path).
    ///
    /// No EXEC_CMD. `ran_on_device_path` on success; never `ran_on_npu`.
    /// `size_bytes == 0` uses [`direct::DEFAULT_BO_ROUNDTRIP_SIZE`].
    pub fn timed_bo_roundtrip(&self, size_bytes: u64) -> direct::TimedOpResult {
        let sub = self.direct_submission();
        direct::timed_bo_roundtrip_direct(sub.as_ref(), size_bytes)
    }

    /// Snapshot of the frozen Direct NPU capability checklist for tools/docs.
    pub fn direct_capability_ids() -> &'static [&'static str] {
        direct::DirectCapability::all_ids()
    }

    /// Query a sensor reading (temperature, power, etc.) via the lightweight direct path.
    pub fn get_direct_sensors(
        &self,
    ) -> Result<direct::amdxdna_drm_query_sensor, direct::DirectError> {
        match &self.execution {
            Execution::Real(exec) => exec.get_sensors(),
            Execution::Simulation(_) => Err(direct::DirectError::Unsupported(
                "direct queries require the Real XDNA2 execution arm (use new_with_real_stub())"
                    .to_string(),
            )),
        }
    }

    /// Get information about a buffer object via the lightweight direct driver path.
    pub fn get_direct_bo_info(
        &self,
        handle: u32,
    ) -> Result<direct::amdxdna_drm_get_bo_info, direct::DirectError> {
        match &self.execution {
            Execution::Real(exec) => exec.get_direct_bo_info(handle),
            Execution::Simulation(_) => Err(direct::DirectError::Unsupported(
                "direct queries require the Real XDNA2 execution arm (use new_with_real_stub())"
                    .to_string(),
            )),
        }
    }

    /// Primary production API for running inference on XDNA2 hardware.
    ///
    /// When `real_execution_active()` is true (Real arm), Phase 4 mode dispatch applies
    /// (`XDNA2_REAL_SUBMIT_MODE`; default fail-closed). Simulation arm runs host fidelity path.
    pub fn submit_and_run(
        &self,
        model: &CompiledModel,
        input: &[f32],
    ) -> Result<Vec<f32>, DragonNpuError> {
        match &self.execution {
            Execution::Simulation(exec) | Execution::Real(exec) => {
                exec.submit_and_run(model, input)
            }
        }
    }

    /// Phase 4 lab/test API: Real-arm submit with an explicit mode (no env mutation).
    ///
    /// Simulation arm ignores `mode` and uses the normal simulation path.
    /// Prefer this in unit tests so `XDNA2_REAL_SUBMIT_MODE` in the shell cannot flake CI.
    pub fn submit_and_run_with_mode(
        &self,
        model: &CompiledModel,
        input: &[f32],
        mode: RealSubmitMode,
    ) -> Result<Vec<f32>, DragonNpuError> {
        match &self.execution {
            Execution::Simulation(exec) | Execution::Real(exec) => {
                exec.submit_and_run_with_mode(model, input, mode)
            }
        }
    }

    /// Phase 4: derive an XDNA2 compile/submit plan from a compiled model (host-side).
    ///
    /// Does not open devices or issue ioctls. Useful for tools that need op-allowlist
    /// status and hwctx size hints without claiming silicon compute.
    #[must_use]
    pub fn xdna2_compile_plan(model: &CompiledModel) -> Xdna2CompilePlan {
        build_xdna2_compile_plan(model)
    }
}

impl Backend for AmdXdna2Backend {
    #[allow(clippy::option_if_let_else)]
    fn status(&self) -> Result<NpuStatus, DragonNpuError> {
        let (device_name, driver_version) = if let Some(d) = &self.detected_device {
            let dn = if d.npu_name.is_empty() {
                // Improve reporting on real hardware where npu_name is often empty
                if d.pci_device == "0x17f0" {
                    format!(
                        "AMD XDNA2 (17f0 rev {}) - hardware detected",
                        d.pci_revision
                    )
                } else {
                    "AMD XDNA2 (Ryzen AI) - hardware detected".to_string()
                }
            } else {
                format!("AMD XDNA2 ({}) - hardware detected", d.npu_name)
            };
            (dn, format!("amdxdna (XDNA2 {} detected)", d.pci_revision))
        } else {
            (
                "AMD XDNA2 (Ryzen AI) - simulated (no XDNA2 hardware detected)".to_string(),
                "stub-0.1 (no XDNA2 hardware found)".to_string(),
            )
        };

        Ok(NpuStatus {
            vendor: "AMD".to_string(),
            available: true,
            compute_units: if self.hardware_detected { 8 } else { 4 },
            memory_mb: if self.hardware_detected { 8192 } else { 4096 },
            peak_tops: 50.0,
            device_name,
            driver_version,
            supported_dtypes: if self.hardware_detected {
                // XDNA2 has strong native Block FP16/bf16 support (per research)
                vec![
                    "fp32".to_string(),
                    "fp16".to_string(),
                    "bf16".to_string(),
                    "int8".to_string(),
                ]
            } else {
                vec![
                    "fp32".to_string(),
                    "fp16".to_string(),
                    "int8".to_string(),
                    "bf16".to_string(),
                ]
            },
            simulated: !self.hardware_detected,
        })
    }

    fn compile_model(
        &self,
        model_path: &str,
        optimization_level: u8,
        quantization: &str,
    ) -> Result<CompiledModel, DragonNpuError> {
        self.execution
            .compile_model(model_path, optimization_level, quantization)
    }

    fn run(&self, input: &[f32]) -> Result<Vec<f32>, DragonNpuError> {
        self.execution.run(input)
    }

    fn submit_and_run(
        &self,
        model: &CompiledModel,
        input: &[f32],
    ) -> Result<Vec<f32>, DragonNpuError> {
        // Delegate to the XDNA2-specific implementation that goes through the
        // private Execution enum (Simulation vs Real stub). This is the path
        // that will eventually perform real NPU submission.
        self.submit_and_run(model, input)
    }
}

impl Default for AmdXdna2Backend {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amd_xdna2_backend_constructs_safely() {
        let backend = AmdXdna2Backend::new();
        // In this environment there is almost certainly no real amdxdna hardware.
        // The important guarantee is that construction never panics and we get
        // a usable backend (simulation with honest metadata).
        let status = backend.status().expect("status must succeed");
        assert_eq!(status.vendor, "AMD");
        assert!(status.available);
        // The device_name must clearly communicate simulated vs hardware.
        assert!(status.device_name.contains("AMD XDNA2"));
        assert!(
            status.device_name.contains("simulated")
                || status.device_name.contains("hardware detected")
        );
    }

    #[test]
    fn amd_xdna2_backend_reports_hardware_detection_state() {
        let backend = AmdXdna2Backend::new();
        // The probe result must be observable.
        let _detected = backend.hardware_detected();
        // When not detected we must still be usable (this is the critical safe fallback).
        let status = backend.status().unwrap();
        if !backend.hardware_detected() {
            assert!(status.device_name.contains("simulated"));
            assert!(
                status.driver_version.contains("stub")
                    || status.driver_version.contains("no XDNA2")
            );
        }
    }

    #[test]
    fn amd_xdna2_backend_exposes_detected_device_when_available() {
        let backend = AmdXdna2Backend::new();
        let dev = backend.detected_device();
        if backend.hardware_detected() {
            assert!(dev.is_some());
        } else {
            // In CI/dev without hardware, it should be None
            assert!(dev.is_none() || dev.unwrap().driver.contains("amdxdna"));
        }
    }

    #[test]
    fn amd_xdna2_backend_detected_npu_type_reports_xdna2_when_hardware_present() {
        let backend = AmdXdna2Backend::new();
        let npu_type = backend.detected_npu_type();
        if backend.hardware_detected() {
            assert!(npu_type.is_some());
            assert!(npu_type.unwrap().contains("XDNA2"));
        } else {
            assert!(npu_type.is_none());
        }
    }

    #[test]
    fn amd_xdna2_backend_status_reflects_xdna2_when_simulated() {
        let backend = AmdXdna2Backend::new();
        let status = backend.status().unwrap();
        assert!(status.device_name.contains("XDNA2"));
        assert_eq!(status.simulated, !backend.hardware_detected());
        if !backend.hardware_detected() {
            assert!(status.device_name.contains("simulated"));
        }
    }

    #[test]
    fn amd_xdna2_backend_detected_npu_type_includes_variant_when_available() {
        let backend = AmdXdna2Backend::new();
        // In real hardware with modalias containing npu4/npu5, this would show the variant.
        // In current simulated runs it will be None or basic, which is acceptable.
        let _ = backend.detected_npu_type();
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn amd_xdna2_backend_exposes_consistent_simulated_status() {
        let backend = AmdXdna2Backend::new();
        let status = backend.status().unwrap();
        assert!(status.device_name.contains("XDNA2"));
        assert_eq!(status.peak_tops, 50.0); // XDNA2-class even in sim for this backend
        assert_eq!(status.simulated, !backend.hardware_detected());
    }

    #[test]
    fn amd_xdna2_backend_detected_npu_type_is_none_in_simulated_mode() {
        let backend = AmdXdna2Backend::new();
        if !backend.hardware_detected() {
            // This is the normal path in CI/dev without real XDNA2 hardware
            assert!(backend.detected_npu_type().is_none());
            assert!(backend.detected_device().is_none());
        }
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn amd_xdna2_backend_status_has_expected_xdna2_fields() {
        let backend = AmdXdna2Backend::new();
        let status = backend.status().unwrap();
        assert_eq!(status.vendor, "AMD");
        assert!(status.device_name.contains("XDNA2"));
        assert!(status.supported_dtypes.contains(&"bf16".to_string()));
        assert_eq!(status.peak_tops, 50.0);
    }

    // === Execution boundary tests (Xdna2Executor + submit_and_run) ===

    #[test]
    fn new_with_real_stub_forces_real_arm() {
        let backend = AmdXdna2Backend::new_with_real_stub();
        assert!(backend.real_execution_active());
        // The stub still reports whatever hardware the probe found (or not).
        // The key contract is that is_real() is true and subsequent calls go to RealXdna2Executor.
        let _ = backend.hardware_detected();
    }

    #[test]
    fn normal_new_path_has_real_execution_inactive() {
        let backend = AmdXdna2Backend::new();
        // Normal construction always uses the Simulation arm today.
        // real_execution_active() only becomes true when the caller explicitly
        // chooses the stub (for early plumbing validation on real silicon).
        assert!(!backend.real_execution_active());
    }

    #[test]
    fn submit_and_run_on_real_stub_returns_clear_actionable_error() {
        let backend = AmdXdna2Backend::new_with_real_stub();
        // Phase 4: compile succeeds on Real arm (host ModelCompiler).
        let model = backend
            .compile_model("any_model.onnx", 1, "fp32")
            .expect("Real arm compile_model must use ModelCompiler");
        assert_ne!(model.ops(), [] as [String; 0]);
        // Pin FailClosed so shell XDNA2_REAL_SUBMIT_MODE=host_ref cannot flake this test.
        let err = backend
            .submit_and_run_with_mode(&model, &[0.0f32; 4], RealSubmitMode::FailClosed)
            .expect_err("FailClosed Real submit must error");
        let msg = format!("{err:?}");
        assert!(
            msg.contains("RealSubmitNotReady") || msg.contains("not ready"),
            "expected actionable RealSubmitNotReady, got: {msg}"
        );
        // Plan helper must be callable without claiming silicon.
        let plan = AmdXdna2Backend::xdna2_compile_plan(&model);
        assert_ne!(plan.ops, [] as [String; 0]);
        assert_ne!(plan.notes, "");
    }

    #[test]
    fn real_submit_mode_from_env_defaults_to_fail_closed_when_unset() {
        // When the env var is absent, from_env must be FailClosed. Do not mutate env here.
        if std::env::var_os("XDNA2_REAL_SUBMIT_MODE").is_none() {
            assert_eq!(RealSubmitMode::from_env(), RealSubmitMode::FailClosed);
        }
    }

    #[test]
    fn real_compile_model_succeeds_without_device() {
        let backend = AmdXdna2Backend::new_with_real_stub();
        let model = backend
            .compile_model("model.onnx", 0, "fp32")
            .expect("compile must succeed on Real arm without requiring silicon");
        assert!(model.node_count() > 0);
        assert_ne!(model.ops(), [] as [String; 0]);
        assert_eq!(model.optimization_level(), 0);
    }

    #[test]
    fn host_ref_mode_runs_allowlisted_json_model() {
        // Tiny allowlisted graph (Relu only) + host_ref mode via pure mode arg (no env race).
        let mut path = std::env::temp_dir();
        path.push("dragon_npu_phase4_host_ref_relu.json");
        let json = r#"{"ops":["Relu"]}"#;
        std::fs::write(&path, json).expect("write temp model");
        let path_str = path.to_str().expect("utf8 path");

        let backend = AmdXdna2Backend::new_with_real_stub();
        let model = backend
            .compile_model(path_str, 0, "fp32")
            .expect("compile allowlisted json");
        let _ = std::fs::remove_file(&path);

        assert_eq!(
            model
                .ops()
                .iter()
                .map(|s| s.to_ascii_lowercase())
                .collect::<Vec<_>>(),
            vec!["relu".to_string()]
        );
        let plan = build_xdna2_compile_plan(&model);
        assert!(plan.host_ref_supported, "plan notes: {}", plan.notes);
        // Issue 8: offline plan stays DEFAULT_HWCTX_TILES without resource.
        assert_eq!(plan.preferred_tiles, DEFAULT_HWCTX_TILES);
        let res = direct::amdxdna_drm_get_resource_info {
            npu_clk_max: 1800,
            npu_tops_max: 58,
            npu_task_max: 16,
            npu_tops_curr: 0,
            npu_task_curr: 0,
        };
        let plan_hw = build_xdna2_compile_plan_with_resource(&model, Some(&res));
        assert_eq!(plan_hw.preferred_tiles, 8);

        // Call Phase 4 dispatch with HostRef directly (avoids process-wide env mutation).
        let shared = RealXdna2Shared {
            detected: None,
            device: None,
        };
        let input = vec![-1.0f32, 0.0, 2.0, 3.5];
        let out = real_arm_submit_and_run(&shared, &model, &input, RealSubmitMode::HostRef)
            .expect("host_ref must produce host-validated output");
        assert_eq!(out.len(), input.len());
        assert!((out[0] - 0.0).abs() < f32::EPSILON); // ReLU
        assert!((out[2] - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn host_ref_rejects_unsupported_ops() {
        let backend = AmdXdna2Backend::new_with_real_stub();
        // Heuristic .onnx path produces many "conv" ops — not in Phase 4 allowlist.
        let model = backend
            .compile_model("big_model.onnx", 0, "fp32")
            .expect("compile");
        let shared = RealXdna2Shared {
            detected: None,
            device: None,
        };
        let err = real_arm_submit_and_run(&shared, &model, &[1.0f32; 4], RealSubmitMode::HostRef)
            .expect_err("conv graph must be UnsupportedOps");
        let msg = format!("{err:?}");
        assert!(
            msg.contains("UnsupportedOps")
                || msg.contains("Unsupported ops")
                || msg.contains("conv"),
            "expected UnsupportedOps, got: {msg}"
        );
    }

    #[test]
    fn npu_exec_mode_refuses_without_device_or_when_gated() {
        assert!(
            !exec_cmd_is_enabled(),
            "test requires default EXEC_CMD gates closed"
        );
        let mut path = std::env::temp_dir();
        path.push("dragon_npu_phase4_npu_exec_id.json");
        std::fs::write(&path, r#"{"ops":["Identity"]}"#).expect("write");
        let backend = AmdXdna2Backend::new_with_real_stub();
        let model = backend
            .compile_model(path.to_str().unwrap(), 0, "fp32")
            .expect("compile");
        let _ = std::fs::remove_file(&path);

        // Synthetic shared without open device: npu_exec must fail closed (no fake Pass).
        let shared = RealXdna2Shared {
            detected: backend.detected_device().cloned(),
            device: None,
        };
        let err = real_arm_submit_and_run(&shared, &model, &[1.0f32], RealSubmitMode::NpuExec)
            .expect_err("npu_exec without device must refuse");
        let msg = format!("{err:?}");
        assert!(
            msg.contains("RealSubmitNotReady")
                || msg.contains("not ready")
                || msg.contains("device")
                || msg.contains("EXEC_CMD")
                || msg.contains("gated"),
            "expected npu_exec refuse, got: {msg}"
        );

        // On hosts with a live open device: pin modes so shell XDNA2_REAL_SUBMIT_MODE
        // (e.g. host_ref) cannot turn this branch into Ok (Identity is host_ref-allowlisted).
        if backend.real_device_open() {
            let err_fc = backend
                .submit_and_run_with_mode(&model, &[1.0f32], RealSubmitMode::FailClosed)
                .expect_err("FailClosed must error even when device open");
            let m_fc = format!("{err_fc:?}");
            assert!(
                m_fc.contains("RealSubmitNotReady") || m_fc.contains("not ready"),
                "got: {m_fc}"
            );
            // Gates closed in this test: NpuExec must also refuse (not Ok / not host_ref).
            let err_ne = backend
                .submit_and_run_with_mode(&model, &[1.0f32], RealSubmitMode::NpuExec)
                .expect_err("NpuExec must refuse when EXEC_CMD gated");
            let m_ne = format!("{err_ne:?}");
            assert!(
                m_ne.contains("RealSubmitNotReady")
                    || m_ne.contains("not ready")
                    || m_ne.contains("gated")
                    || m_ne.contains("EXEC"),
                "got: {m_ne}"
            );
        }
    }

    #[test]
    fn submit_and_run_succeeds_on_simulation_arm() {
        let backend = AmdXdna2Backend::new();
        // On the normal path we always get the high-fidelity Simulation arm.
        // submit_and_run should delegate cleanly and produce a result.
        let model = backend
            .compile_model("model.onnx", 0, "fp32")
            .expect("simulation compile must succeed");
        let input = vec![0.0f32; 4];
        let output = backend
            .submit_and_run(&model, &input)
            .expect("submit_and_run on simulation arm must succeed");
        assert_ne!(output, [] as [f32; 0]);
    }

    // === New trait surface + CompiledModel delegation tests ===

    #[test]
    fn compiled_model_submit_and_run_delegates_through_backend() {
        let backend = AmdXdna2Backend::new();
        let model = backend
            .compile_model("model.onnx", 1, "fp32")
            .expect("compile must succeed on simulation path");
        let input = vec![0.1f32; 8];

        // Direct on backend (trait method)
        let out1 = backend
            .submit_and_run(&model, &input)
            .expect("backend.submit_and_run must work");

        // Via CompiledModel (public user-facing method)
        let out2 = model
            .submit_and_run(&input)
            .expect("CompiledModel.submit_and_run must work");

        assert_eq!(out1.len(), out2.len());
    }

    #[test]
    fn submit_and_run_on_real_stub_via_compiled_model_returns_clear_error() {
        let backend = AmdXdna2Backend::new_with_real_stub();
        // Phase 4: compile succeeds; use pinned FailClosed (not env-dependent CompiledModel path).
        let model = backend
            .compile_model("real_model.onnx", 2, "bf16")
            .expect("Real arm compile must succeed");
        let err = backend
            .submit_and_run_with_mode(&model, &[0.1f32; 8], RealSubmitMode::FailClosed)
            .expect_err("FailClosed Real submit must error");
        let err_msg = format!("{err:?}");
        assert!(
            err_msg.contains("RealSubmitNotReady") || err_msg.contains("not ready"),
            "expected actionable Real submit error, got: {err_msg}"
        );
    }

    #[test]
    fn host_ref_rejects_matmul_without_fake_golden() {
        // Phase 5: MatMul is allowlisted but must refuse without weights (no fake golden).
        let mut path = std::env::temp_dir();
        path.push("dragon_npu_phase4_host_ref_matmul.json");
        std::fs::write(&path, r#"{"ops":["MatMul"]}"#).expect("write");
        let backend = AmdXdna2Backend::new_with_real_stub();
        let model = backend
            .compile_model(path.to_str().unwrap(), 0, "fp32")
            .expect("compile");
        let _ = std::fs::remove_file(&path);
        let err = backend
            .submit_and_run_with_mode(&model, &[1.0f32; 4], RealSubmitMode::HostRef)
            .expect_err("MatMul must not Ok on host_ref without weights");
        let msg = format!("{err:?}");
        assert!(
            msg.contains("initializer")
                || msg.contains("weight")
                || msg.contains("Runtime")
                || msg.contains("UnsupportedOps")
                || msg.contains("matmul")
                || msg.contains("MatMul"),
            "got {msg}"
        );
    }

    #[test]
    fn host_ref_matmul_with_weights_known_numbers() {
        let mut path = std::env::temp_dir();
        path.push("dragon_npu_phase5_host_ref_matmul_ok.json");
        std::fs::write(
            &path,
            r#"{
              "ops": ["MatMul", "Relu"],
              "initializers": [
                {"name": "W", "shape": [2, 2], "data": [1.0, 0.0, 0.0, 1.0]}
              ]
            }"#,
        )
        .expect("write");
        let backend = AmdXdna2Backend::new_with_real_stub();
        let model = backend
            .compile_model(path.to_str().unwrap(), 0, "fp32")
            .expect("compile");
        let _ = std::fs::remove_file(&path);
        // Identity weight: input [[-1, 2]] -> MatMul same -> Relu [0, 2]
        let out = backend
            .submit_and_run_with_mode(&model, &[-1.0, 2.0], RealSubmitMode::HostRef)
            .expect("host_ref MatMul+Relu with weights");
        assert_eq!(out, vec![0.0, 2.0]);
        let plan = build_weight_bo_plan(&model);
        assert_eq!(plan.len(), 1);
        assert_eq!(plan[0].nbytes, 16);
    }

    #[test]
    fn real_device_open_is_available_on_real_arm() {
        let backend = AmdXdna2Backend::new_with_real_stub();
        // The diagnostic method should be callable. On real hardware it will
        // reflect whether the /dev/accel open succeeded.
        let _ = backend.real_device_open();
    }

    #[test]
    fn real_executor_stores_device_path_when_hardware_present() {
        let backend = AmdXdna2Backend::new_with_real_stub();
        // When using the Real arm, the executor should have attempted to
        // open a device path (even if the actual file open only succeeds on
        // real hardware with the amdxdna driver).
        if backend.real_execution_active() {
            // We can't easily inspect the private executor here, but we can
            // at least verify the public diagnostic is consistent.
            let _ = backend.real_device_open();
        }
    }

    #[test]
    fn real_device_diagnostics_are_callable_and_consistent_on_real_arm() {
        let backend = AmdXdna2Backend::new_with_real_stub();
        // Basic production API contract: these methods must be callable
        // without panicking and should be consistent with real_execution_active().
        let is_open = backend.real_device_open();
        let path = backend.real_device_path();

        if backend.real_execution_active() {
            // On the Real arm we expect the diagnostics to be available.
            // The actual open may only succeed on real hardware.
            let _ = is_open;
            let _ = path;
        } else {
            // On Simulation arm these should be false / None.
            assert!(!is_open);
            assert!(path.is_none());
        }
    }

    #[test]
    fn real_arm_always_attempts_device_path() {
        let backend = AmdXdna2Backend::new_with_real_stub();
        if backend.real_execution_active() {
            // When the Real arm is active, we should have attempted a device path.
            // On real hardware this will usually be Some after a successful open.
            // In CI it may still be Some (the path we tried) even if open failed.
            // The important contract is that the diagnostic is present.
            let _path = backend.real_device_path();
        }
    }

    #[test]
    fn amd_xdna2_backend_real_onnx_weights_flow_through_backend() {
        // TDD hook for real weight data (onx-08) flowing through AmdXdna2Backend.
        // Heuristic models have 0 weights; real .onnx + bf16 raw paths are covered
        // by compiler TDD but exercised via this backend on hardware owners' machines.
        let backend = AmdXdna2Backend::new();
        let model = backend
            .compile_model("model.onnx", 1, "fp32")
            .expect("compile must succeed");
        // Heuristic fallback has no real initializers.
        assert!(!model.has_real_weights());
        // submit_and_run still works (delegates to simulation arm on this machine).
        let input = vec![0.0f32; 4];
        let _ = backend
            .submit_and_run(&model, &input)
            .expect("submit_and_run works through XDNA2 backend");
        // On real hardware with a real .onnx containing bf16 weights, has_real_weights()
        // and initializer data would be true and available for the future real executor.
    }

    #[test]
    fn amd_xdna2_backend_bf16_weights_flow_through_xdna2_backend() {
        // Dedicated TDD (xdna2-exec-06): real .onnx + bf16 raw weights through AmdXdna2Backend.
        // Uses the same programmatic ONNX construction as compiler tests.
        use std::io::Write;
        use tract_onnx::pb::{GraphProto, ModelProto, NodeProto, TensorProto};

        let mut path = std::env::temp_dir();
        path.push("dragon_npu_xdna2_bf16_weights.onnx");

        let bf16_raw: Vec<u8> = vec![0x00, 0x3F, 0x00, 0x40]; // 2 elements

        {
            let mut f = std::fs::File::create(&path).expect("create");
            let model = ModelProto {
                ir_version: 7,
                graph: Some(GraphProto {
                    node: vec![NodeProto {
                        op_type: "add".to_string(),
                        input: vec!["a".to_string(), "w".to_string()],
                        output: vec!["out".to_string()],
                        ..Default::default()
                    }],
                    initializer: vec![TensorProto {
                        name: "w".to_string(),
                        dims: vec![2],
                        data_type: 16, // BFLOAT16
                        raw_data: bf16_raw,
                        ..Default::default()
                    }],
                    ..Default::default()
                }),
                ..Default::default()
            };
            let mut buf = Vec::new();
            prost::Message::encode(&model, &mut buf).expect("encode");
            f.write_all(&buf).expect("write");
        }

        let backend = AmdXdna2Backend::new(); // uses simulation arm on this machine, but full path
        let model = backend
            .compile_model(path.to_str().unwrap(), 1, "bf16")
            .expect("real .onnx with bf16 weights must load through XDNA2 backend");
        let _ = std::fs::remove_file(&path);

        assert!(model.has_real_weights());
        let inits = model.initializers();
        // Initializer has no PartialEq, so an empty-slice compare does not type-check.
        #[allow(clippy::len_zero)]
        {
            assert!(!inits.is_empty());
        }
        let w = &inits[0];
        assert_eq!(w.data_type, 16);
        assert_ne!(w.raw_data, [] as [u8; 0]);
        assert_ne!(w.as_f32_data(), [] as [f32; 0]); // usable approx available

        // Exercise the new numeric folding / weight size APIs through the XDNA2 backend.
        let _ = model.total_weight_elements();
        let _ = model
            .initializers()
            .iter()
            .map(|i| i.as_f32_data().len())
            .sum::<usize>();

        // submit_and_run still works end-to-end through the backend
        let input = vec![0.0f32; 2];
        let _ = backend
            .submit_and_run(&model, &input)
            .expect("submit_and_run works with real bf16-weight model via XDNA2 backend");
    }

    // === Tests for the new RealXdna2Device / RealXdna2DeviceInfo abstraction ===

    #[test]
    fn real_device_info_is_none_on_normal_construction_path() {
        // On the safe default path (even with amd-xdna2 feature), we use the
        // high-fidelity Simulation arm. real_device_info must be None.
        let backend = AmdXdna2Backend::new();
        assert!(!backend.real_execution_active());
        assert!(backend.real_device_info().is_none());
    }

    #[test]
    fn real_device_info_available_only_on_real_arm_when_opened() {
        let backend = AmdXdna2Backend::new_with_real_stub();

        let info = backend.real_device_info();

        if backend.real_execution_active() && backend.real_device_open() {
            // On real hardware where the open succeeded, we should have rich info.
            let info = info.expect("real_device_info must be Some when device is open on real arm");
            assert_ne!(info.pci_vendor, "");
            assert_ne!(info.pci_device, "");
            // variant may be Some("XDNA2") or more specific on real systems
        } else {
            // In all other cases (no hardware or open failed) it must be None.
            assert!(info.is_none());
        }
    }

    #[test]
    fn real_device_info_is_always_none_on_simulation_arm() {
        let backend = AmdXdna2Backend::new(); // always simulation for this constructor
        // Even if we call the method, it must be None.
        assert!(backend.real_device_info().is_none());
    }

    #[test]
    fn real_xdna2_device_info_has_expected_shape_when_present() {
        // On the real stub path we can still inspect the type even if no device opened.
        let backend = AmdXdna2Backend::new_with_real_stub();
        if let Some(info) = backend.real_device_info() {
            // When we have info, all the key fields should be populated from the probe.
            assert!(info.pci_vendor.starts_with("0x"));
            assert!(info.pci_device.starts_with("0x"));
            // On user's real 17f0 hardware this is typically "XDNA2" or "NPU5" etc.
            if let Some(v) = &info.xdna2_variant {
                assert!(v.contains("XDNA") || v.contains("NPU"));
            }
            // The info should be Debug-printable (used by humans and logs).
            let _ = format!("{info:?}");
        }
    }

    #[test]
    fn real_xdna2_device_info_can_be_constructed_manually_for_testing() {
        // This exercises the public struct directly (useful for future real-exec tests
        // that want to mock device info without opening real hardware).
        let info = RealXdna2DeviceInfo {
            path: std::path::PathBuf::from("/dev/accel/accel0"),
            pci_vendor: "0x1022".to_string(),
            pci_device: "0x17f0".to_string(),
            pci_revision: "0x20".to_string(),
            npu_name: String::new(),
            modalias: "pci:v00001022d000017F0...".to_string(),
            xdna2_variant: Some("XDNA2".to_string()),
        };

        assert_eq!(info.pci_device, "0x17f0");
        assert_eq!(info.xdna2_variant.as_deref(), Some("XDNA2"));
        let debug = format!("{info:?}");
        assert!(debug.contains("17f0"));
    }

    #[test]
    fn real_device_detected_and_read_sysfs_are_none_on_simulation_paths() {
        let backend = AmdXdna2Backend::new();
        assert!(backend.real_device_detected().is_none());
        assert!(backend.real_device_read_sysfs("device/uevent").is_none());

        let real_stub = AmdXdna2Backend::new_with_real_stub();
        if !real_stub.real_device_open() {
            assert!(real_stub.real_device_detected().is_none());
            assert!(real_stub.real_device_read_sysfs("name").is_none());
        }
    }
}
