//! Soft offline residual class report from `last_apply.json` (no L3 / no EXEC).
//!
//! Loads a HITL snapshot (schema 2 fields: `status`, `detail`, `aiebu_elf`,
//! `ert_error`, optional top-level `rela_count`) and prints pure
//! [`SiliconResidualClass`] + research note.
//!
//! **Honesty:** always `not_c_ready=1` / `not_ran_on_npu=1` in the note.
//! Never invents Pass or C unlock. `CONTRACT_OK` ≠ C-ready.
//!
//! Path policy matches write confinement: temp_dir ∪ testdata/xdna2_cmd,
//! `.json` only, no `..`, no symlinks (`last_apply_load_path_is_allowed`).
//!
//! ```bash
//! cargo run --features amd-xdna2 --example classify_last_apply --quiet -- \
//!   testdata/xdna2_cmd/last_apply.json
//! # or: bash tools/classify_last_apply_residual.sh
//! ```

#![allow(clippy::print_stdout, clippy::print_stderr)]

#[cfg(feature = "amd-xdna2")]
use std::env;
#[cfg(feature = "amd-xdna2")]
use std::fs;
#[cfg(feature = "amd-xdna2")]
use std::io::Read;
#[cfg(feature = "amd-xdna2")]
use std::path::{Path, PathBuf};
#[cfg(feature = "amd-xdna2")]
use std::process::ExitCode;

#[cfg(feature = "amd-xdna2")]
use dragon_npu::backends::amd_xdna2::{
    AXLF_FIRST_SECTION_HEADER_ABS_OFFSET, AXLF_SECTION_HEADER_SIZE, C2ExpectPaddrs,
    ENV_LAB_EXTERNAL_CTRLTEXT, JournalFwStatusPair, MAX_MLADF_LAB_FILE_BYTES,
    c2_live_cmdlist_msg_words_embedded, c2_live_io_paddrs_locked,
    check_external_known_good_bag_drop, classify_last_apply_residual_ex,
    compare_external_ctrltext_to_lab_export, cross_check_c2_cmdlist_fill_locked,
    decode_c2_live_cmdbuf_vs_pure_rebuild, default_last_apply_path,
    export_ctrltext_fingerprint_research, format_app_load_offline_checklist,
    format_app_load_offline_checklist_ex, format_c2_cmdlist_msg_research_note,
    format_external_known_good_operator_extract_checklist, format_silicon_residual_class_note,
    last_apply_bo_gt_content, last_apply_has_rela_evidence, last_apply_load_path_is_allowed,
    last_apply_start_npu_matches_content, last_apply_temp_root, load_external_ctrltext_for_compare,
    load_external_ctrltext_from_env, parse_journal_fw_status_pair, parse_last_apply_detail_signals,
    profile_instr_payload_after_rela_c2_locked, read_external_known_good_provenance_text,
    read_lab_file_capped, rebuild_c2_live_start_npu_cmd_bo, reconfirm_app_load_tracks_a_n_closed,
    research_app_load_bag_feed_dry_run_scorecard_ex, research_app_load_e2e_fixture_path,
    research_app_load_extract_bag_scorecard_pipeline_ex,
    research_app_load_offline_progress_scorecard, research_app_load_partition_id_field_map,
    research_app_load_pdi_partition, research_app_load_residual_axis_matrix,
    research_construction_path_lab_vs_public, research_cu_pdi_necessity_for_app_load,
    research_cu_pdi_section_identity_from_text,
    research_external_ctrltext_compare_dry_run_from_lab_feed_ex,
    research_lab_export_fingerprint_feed, research_xclbin_cu_pdi_section_body_bounds,
    research_xclbin_cu_pdi_section_offset_map, research_xclbin_extract_dry_run,
    research_xclbin_extract_ve2_fixture_refused, scan_external_public_pdi_evidence,
    scan_public_pdi_partition_evidence, validate_external_known_good_provenance_text,
};

/// Soft VE2 header walk: enough section headers for typical research xclbins
/// (VE2 has 10). Smaller than pure walk cap `AXLF_MAX_SECTIONS_WALK` (256) so
/// the printer never loads multi-MiB section bodies — prefix only.
#[cfg(feature = "amd-xdna2")]
const SOFT_AXLF_SECTION_WALK: usize = 16;

/// Cap for optional `XDNA2_LAB_JOURNAL_TEXT` paste (research; not a file path).
const MAX_LAB_JOURNAL_TEXT_BYTES: usize = 256 * 1024;

/// True when env value is an explicit enable (`1` / `true`, case-insensitive).
fn env_flag_truthy(key: &str) -> bool {
    env::var(key).ok().is_some_and(|v| {
        let t = v.trim();
        t == "1" || t.eq_ignore_ascii_case("true")
    })
}

fn main() -> ExitCode {
    #[cfg(not(feature = "amd-xdna2"))]
    {
        eprintln!("classify_last_apply requires --features amd-xdna2");
        return ExitCode::from(2);
    }
    #[cfg(feature = "amd-xdna2")]
    {
        match run() {
            Ok(()) => ExitCode::SUCCESS,
            Err(code) => ExitCode::from(code),
        }
    }
}

#[cfg(feature = "amd-xdna2")]
fn run() -> Result<(), u8> {
    let mut args: Vec<String> = env::args().skip(1).collect();
    if args.iter().any(|a| a == "-h" || a == "--help") {
        print_help();
        return Ok(());
    }
    // Strip a single optional `--` (cargo passes args after --).
    if args.first().is_some_and(|a| a == "--") {
        args.remove(0);
    }
    let path = match args.as_slice() {
        [] => resolve_default_path(),
        [p] => PathBuf::from(p),
        _ => {
            eprintln!("ERROR: usage: classify_last_apply [last_apply.json]");
            print_help();
            return Err(2);
        }
    };

    if !last_apply_load_path_is_allowed(&path) {
        eprintln!(
            "ERROR: last_apply path not allowed (must be under temp_dir or \
             testdata/xdna2_cmd, .json only, no .., no symlink): {}",
            path.display()
        );
        return Err(2);
    }

    if !path.is_file() {
        eprintln!("ERROR: missing last_apply path: {}", path.display());
        eprintln!(
            "  Pass an explicit path or set XDNA2_LAB_LAST_APPLY_JSON / stage a HITL C snapshot."
        );
        return Err(2);
    }

    let text = read_capped(&path).map_err(|e| {
        eprintln!("ERROR: read {}: {e}", path.display());
        2_u8
    })?;
    let v: serde_json::Value = serde_json::from_str(&text).map_err(|e| {
        eprintln!("ERROR: json parse {}: {e}", path.display());
        2_u8
    })?;

    let detail = v.get("detail").and_then(serde_json::Value::as_str);
    let status = v.get("status").and_then(serde_json::Value::as_str);
    let aiebu = v.get("aiebu_elf").and_then(serde_json::Value::as_bool);
    let ert_error = v.get("ert_error").and_then(serde_json::Value::as_bool);
    let ran_field = v.get("ran_on_npu").and_then(serde_json::Value::as_bool);
    let rela_count_field = v
        .get("rela_count")
        .and_then(serde_json::Value::as_u64)
        .and_then(|n| usize::try_from(n).ok());

    let class = classify_last_apply_residual_ex(status, detail, aiebu, ert_error, rela_count_field);
    let sig = merge_sig(detail, rela_count_field);
    let note = format_silicon_residual_class_note(class, sig.as_ref());
    print_report(
        &path,
        class,
        status,
        aiebu,
        ert_error,
        rela_count_field,
        ran_field,
        detail,
        sig.as_ref(),
        &note,
    );
    Ok(())
}

/// Merge detail parse with optional schema-2 top-level `rela_count`.
#[cfg(feature = "amd-xdna2")]
fn merge_sig(
    detail: Option<&str>,
    rela_count_field: Option<usize>,
) -> Option<dragon_npu::backends::amd_xdna2::LastApplyDetailSignals> {
    let mut sig = detail.map(parse_last_apply_detail_signals);
    if let Some(ref mut s) = sig {
        if !last_apply_has_rela_evidence(s) {
            if let Some(n) = rela_count_field.filter(|&n| n > 0) {
                s.rela_count = Some(n);
            }
        }
    } else if let Some(n) = rela_count_field.filter(|&n| n > 0) {
        sig = Some(dragon_npu::backends::amd_xdna2::LastApplyDetailSignals {
            rela_count: Some(n),
            ..Default::default()
        });
    }
    sig
}

/// Resolve optional `XDNA2_LAB_JOURNAL_TEXT` paste (research; never a file path).
///
/// Oversized paste → refuse + `None` (checklist takes absent path). Cap 256 KiB.
#[cfg(feature = "amd-xdna2")]
fn resolve_lab_journal_text() -> Option<String> {
    let journal_env = env::var("XDNA2_LAB_JOURNAL_TEXT").ok();
    match journal_env
        .as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty())
    {
        Some(jt) if jt.len() > MAX_LAB_JOURNAL_TEXT_BYTES => {
            println!(
                "journal_text_refused=oversized max={MAX_LAB_JOURNAL_TEXT_BYTES} got={}",
                jt.len()
            );
            None
        }
        Some(jt) => Some(jt.to_string()),
        None => None,
    }
}

#[cfg(feature = "amd-xdna2")]
fn print_sig_detail(
    class: dragon_npu::backends::amd_xdna2::SiliconResidualClass,
    s: &dragon_npu::backends::amd_xdna2::LastApplyDetailSignals,
    journal: Option<&str>,
) {
    if let Some(st) = s.ert_state {
        println!("ert_state={st}");
    }
    println!("detail_ert_error={}", s.ert_error);
    println!("detail_aiebu_elf={:?}", s.aiebu_elf);
    println!("detail_rela_count={:?}", s.rela_count);
    println!("aiebu_rela_patched={}", s.aiebu_rela_patched);
    println!("rela_evidence={}", last_apply_has_rela_evidence(s));
    if let Some(ref lay) = s.layout {
        println!("layout={lay}");
    }
    if let Some(ref mode) = s.inst_size_mode {
        println!("inst_size_mode={mode}");
    }
    if let Some(ref mode) = s.rela_mode {
        println!("rela_mode={mode}");
    }
    if let Some(r) = s.ran_on_npu {
        println!("detail_ran_on_npu={r}");
    }
    if let Some(nz) = s.ofm_raw_nonzero_bytes {
        println!("ofm_raw_nonzero_bytes={nz}");
    }
    if let Some(code) = s.fw_status_code {
        println!("fw_status_code=0x{code:x}");
    }
    if let Some(off) = s.instr_map_offset {
        println!("instr_map_offset=0x{off:x}");
    }
    if let Some(ref ty) = s.instr_bo_type {
        println!("instr_bo_type={ty}");
    }
    if let Some(addr) = s.instr_xdna_addr {
        println!("instr_xdna_addr=0x{addr:x}");
    }
    if let Some(n) = s.content_size_b {
        println!("content_size_b={n}");
    }
    if let Some(n) = s.bo_size_b {
        println!("bo_size_b={n}");
    }
    if let Some(n) = s.start_npu_size_b {
        println!("start_npu_size_b={n}");
    }
    println!(
        "start_npu_matches_content={}",
        u8::from(last_apply_start_npu_matches_content(s))
    );
    println!("bo_gt_content={}", u8::from(last_apply_bo_gt_content(s)));
    println!("submit_wait_not_ok={}", u8::from(s.submit_wait_not_ok));
    // Optional journal dual (shared with Track L research print).
    if let Some(jt) = journal {
        println!(
            "{}",
            format_app_load_offline_checklist_ex(class, s, Some(jt))
        );
        let pair = parse_journal_fw_status_pair(jt);
        println!("journal_fw_pair_soft={}", pair.format_soft_note());
    } else {
        println!("{}", format_app_load_offline_checklist(class, s));
    }
}

/// Track F/G/K/L/M/N/O soft research when `XDNA2_LAB_PRINT_C2_CMDBUF_RESEARCH=1|true`.
///
/// Always soft (`invent_pass=0`); never opens devices / never Pass / never C-ready.
/// Track K lines: `c2_cmdlist_fill_cross_check=` (public fill map) and optional
/// `instr_pdi_load_research=` when a full staged mladf aiebu ELF is present
/// (silicon_lab / gt_capture only — never the 1072 B research stub).
/// Track L: `app_load_pdi_partition_research=` (public FW status-name surface only).
/// Track M: `public_pdi_evidence_scan=` (closed public-evidence lock; no FS walk;
/// pure status-name re-check + `new_public_pdi_fields=0`).
/// Track N: `external_public_pdi_evidence_scan=` (external kernel/XRT beyond
/// status-name closed_no_new; no network/XRT tree walk; depth still status_name_only).
/// Track O: `app_load_maintain_reconfirm=` (A–N still closed; compose M+N field
/// echoes; no FS/network/XRT walk; no new external evidence this maintain turn —
/// ≠ C-ready ≠ silicon progress).
///
/// `detail` is last_apply userspace detail (e2e journal codes still only from
/// `journal_text` — never invent from detail alone).
#[cfg(feature = "amd-xdna2")]
fn print_c2_cmdbuf_research_optional(
    class: dragon_npu::backends::amd_xdna2::SiliconResidualClass,
    journal_text: Option<&str>,
    detail: Option<&str>,
) {
    if !env_flag_truthy("XDNA2_LAB_PRINT_C2_CMDBUF_RESEARCH") {
        return;
    }
    let (inst, ifm, wts, ofm, _ctrl) = c2_live_io_paddrs_locked();
    let expect = C2ExpectPaddrs::from_c2_locked_io();
    let pure_bo = match rebuild_c2_live_start_npu_cmd_bo(inst, 1688, ifm, wts, ofm, 0x1) {
        Ok(pure) => {
            let map = decode_c2_live_cmdbuf_vs_pure_rebuild(&pure, expect);
            println!("c2_cmdbuf_map={}", map.format_soft_note());
            Some(pure)
        }
        Err(e) => {
            eprintln!("WARN: c2 rebuild failed: {e}");
            None
        }
    };
    println!(
        "c2_cmdlist_msg={}",
        format_c2_cmdlist_msg_research_note(c2_live_cmdlist_msg_words_embedded())
    );
    // Track K: public cmdlist fill cross-check (locked embed + pure START_NPU).
    let fill = cross_check_c2_cmdlist_fill_locked(pure_bo.as_deref());
    println!("c2_cmdlist_fill_cross_check={}", fill.format_soft_note());
    // Track K: optional instr/PDI payload profile after rela (full staged ELF only).
    let userspace_content_ok = print_instr_pdi_load_research_optional();
    // Track L: public PDI/partition surface (status names only; no private decode).
    let pair = journal_text.map(parse_journal_fw_status_pair);
    let l = research_app_load_pdi_partition(class, pair.as_ref(), userspace_content_ok);
    println!("app_load_pdi_partition_research={}", l.format_soft_note());
    // Track M: pure public-evidence scan (closed = no new fields beyond Track L).
    let m = scan_public_pdi_partition_evidence();
    println!("public_pdi_evidence_scan={}", m.format_soft_note());
    // Track N: external public evidence closed (no new kernel/XRT beyond status-name).
    let nscan = scan_external_public_pdi_evidence();
    println!(
        "external_public_pdi_evidence_scan={}",
        nscan.format_soft_note()
    );
    // Track O: maintain reconfirm A–N still closed (compose M+N; no new evidence).
    let o = reconfirm_app_load_tracks_a_n_closed();
    println!("app_load_maintain_reconfirm={}", o.format_soft_note());
    // Construction path (lab instr-only vs public CU/PDI): soft research only.
    print_construction_path_research_optional();
    // Fingerprint export identity + known-good absent compare (awaiting capture).
    // Also prints CU/PDI necessity residual matrix + residual axis matrix.
    // Reuse Track L userspace + journal (do not invent userspace green / drop pair).
    print_ctrltext_fingerprint_export_optional(class, journal_text, detail, userspace_content_ok);
}

/// Soft `construction_path_research=` from staged full mladf aiebu when present.
///
/// Dual-root parity with [`print_instr_pdi_load_research_optional`]: prefer
/// `silicon_lab/lab_instr.elf`, then `gt_capture/instr_pre.elf`. Full staged
/// only (rela×12 / ctrltext 1688) when on disk; otherwise empty ELF still
/// prints gap honesty (public requires CU/PDI; lab has none). Never Pass.
#[cfg(feature = "amd-xdna2")]
fn print_construction_path_research_optional() {
    use std::path::PathBuf;
    let roots = [
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("testdata/xdna2_cmd/silicon_lab/lab_instr.elf"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("testdata/xdna2_cmd/gt_capture/instr_pre.elf"),
    ];
    let mut elf = Vec::new();
    for pth in &roots {
        if !pth.is_file() {
            continue;
        }
        match read_lab_file_capped(pth.to_str().unwrap_or(""), "construction_path_research") {
            Ok(e) => {
                let r = research_construction_path_lab_vs_public(&e);
                // Prefer full staged (×12 / 1688); skip stub/rela×1 if both present.
                if r.lab_ctrltext_len == Some(1688) && r.lab_rela_count == Some(12) {
                    elf = e;
                    break;
                }
                if elf.is_empty() && r.lab_has_instr_ctrltext {
                    elf = e;
                }
            }
            Err(e) => {
                eprintln!("WARN: construction_path ELF refuse {}: {e}", pth.display());
            }
        }
    }
    let r = research_construction_path_lab_vs_public(&elf);
    println!("construction_path_research={}", r.format_soft_note());
}

/// Soft `ctrltext_fingerprint_export=` + `external_ctrltext_compare=`.
///
/// Exports silicon_lab (or gt_capture) unpatched identity. External known-good
/// is **absent** unless `XDNA2_LAB_EXTERNAL_CTRLTEXT` is set and passes
/// confinement (`temp_dir` ∪ `testdata/xdna2_cmd/external_known_good/`, not
/// local pure-direct `instr_post_patch.bin`). Never invents Pass; soft note
/// always `product_known_good=0` even on sha_match.
///
/// `userspace_content_ok` and optional journal must match Track L (do not invent
/// userspace green or drop journal). FW residual for necessity/axes opens from
/// locked class **or** C2 journal pair (same dual residual honesty as Track L).
#[cfg(feature = "amd-xdna2")]
fn print_ctrltext_fingerprint_export_optional(
    class: dragon_npu::backends::amd_xdna2::SiliconResidualClass,
    journal_text: Option<&str>,
    detail: Option<&str>,
    userspace_content_ok: bool,
) {
    use std::path::PathBuf;
    let roots = [
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("testdata/xdna2_cmd/silicon_lab/lab_instr.elf"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("testdata/xdna2_cmd/gt_capture/instr_pre.elf"),
    ];
    let mut elf = Vec::new();
    for pth in &roots {
        if !pth.is_file() {
            continue;
        }
        match read_lab_file_capped(pth.to_str().unwrap_or(""), "ctrltext_fingerprint_export") {
            Ok(e) => {
                let exp = export_ctrltext_fingerprint_research(&e);
                if exp.ctrltext_len == Some(1688) && exp.rela_count == Some(12) {
                    elf = e;
                    break;
                }
                if elf.is_empty() && exp.classic_walk_ok {
                    elf = e;
                }
            }
            Err(e) => {
                eprintln!("WARN: fingerprint export ELF refuse {}: {e}", pth.display());
            }
        }
    }
    let exp = export_ctrltext_fingerprint_research(&elf);
    println!("ctrltext_fingerprint_export={}", exp.format_soft_note());
    // Lab export fingerprint auto-feed (stamp d2e287ab): derive feed from staged
    // export-path ELF + optional last_apply signals (never invent external bytes;
    // signals alone stay incomplete without ELF). Env external path is NOT a lab
    // refuse source — it is applied only on dry-run for external bag refuse matrix.
    // Residual class is not invented here — feed/dry-run are research surfaces only.
    let elf_for_feed = if elf.is_empty() {
        None
    } else {
        Some(elf.as_slice())
    };
    let feed = research_lab_export_fingerprint_feed(elf_for_feed, detail, None, None);
    println!("lab_export_fingerprint_feed={}", feed.format_soft_note());
    // Optional confined external bag (env). Unset → honest absent awaiting capture.
    // Never treat gt_capture/instr_post_patch as known-good (loader refuses basenames).
    let mut external_load_refused = false;
    let external = match load_external_ctrltext_from_env() {
        Ok(v) => v,
        Err(e) => {
            external_load_refused = true;
            eprintln!(
                "WARN: {ENV_LAB_EXTERNAL_CTRLTEXT} refuse (confinement/read): {e}; \
                 comparing with absent external"
            );
            None
        }
    };
    if external.is_some() {
        println!(
            "external_ctrltext_source=env_confined invent_pass=0 not_c_ready=1 \
             not_ran_on_npu=1 product_known_good=0"
        );
    }
    let cmp = compare_external_ctrltext_to_lab_export(&exp, external.as_deref());
    println!("external_ctrltext_compare={}", cmp.format_soft_note());
    // Compare dry-run via auto-feed (stamp d2e287ab; prior dry-run 6033158a surface).
    // Residual class is not invented here — dry-run is research surface only.
    print_external_ctrltext_compare_dry_run_optional(
        &feed,
        external.as_deref(),
        external_load_refused,
    );
    // Track L parity: journal pair + same userspace_content_ok (no invent green).
    let journal_pair = journal_text.map(parse_journal_fw_status_pair);
    let partition_for_axes =
        research_app_load_pdi_partition(class, journal_pair.as_ref(), userspace_content_ok);
    // CU/PDI necessity: FW residual from class **or** C2 journal pair (not class alone).
    // Sha match alone never closes residual (product_known_good=0 invent_pass=0).
    let construction = research_construction_path_lab_vs_public(&elf);
    let fw_app_load = class.is_locked_app_load_after_aiebu()
        || journal_pair
            .as_ref()
            .is_some_and(JournalFwStatusPair::is_c2_app_load_pair);
    let necessity = research_cu_pdi_necessity_for_app_load(
        &construction,
        fw_app_load,
        cmp.sha_match && cmp.external_present,
    );
    println!("cu_pdi_necessity_research={}", necessity.format_soft_note());
    // Residual axis matrix: same partition bag as Track L + necessity.
    let axes = research_app_load_residual_axis_matrix(&partition_for_axes, &necessity);
    println!("app_load_residual_axis_matrix={}", axes.format_soft_note());
    // Xclbin extract dry-run: VE2 fixture refuse path (NOT_AIE2 ≠ AIE2 known-good).
    // Optional staged header bytes when present; never invent product known-good.
    print_xclbin_extract_research_dry_run_optional();
    // Pure CU/PDI section identity from public section names (info text when present).
    // Name surface only — section identity alone never closes residual / invent Pass.
    print_cu_pdi_section_identity_research_optional();
    // Pure BITSTREAM/AIE_PARTITION section-offset map (public axlf headers only).
    // Offset map alone never closes residual / invent Pass / product known-good.
    print_cu_pdi_section_offset_map_research_optional();
    // Pure body window bounds vs buffer len (headers only / prefix OOB honest).
    // Body bounds alone never closes residual / invent Pass / product known-good.
    print_cu_pdi_section_body_bounds_research_optional();
    // Journal partition-id field map: status codes mapped; partition_id always absent_public.
    let field_map = research_app_load_partition_id_field_map(journal_pair.as_ref());
    println!(
        "app_load_partition_id_field_map={}",
        field_map.format_soft_note()
    );
    // Lab bag PROVENANCE + bag-drop + e2e + offline progress scorecard
    // (stamps 8e898123 / 9cdfe8d0) + bag+feed+dry-run composition (`2b01425e`).
    print_app_load_e2e_and_bag_drop_optional(
        class,
        journal_text,
        detail,
        external.is_some(),
        Some(&cmp),
        external_load_refused,
        &feed,
        external.as_deref(),
    );
}

/// Soft compare dry-run via lab export auto-feed (stamp `d2e287ab`).
///
/// Bag absent → `blocked_on_external_bytes=1`. Lab feed ready → `lab_export_ready=1`.
/// Refuse local post_patch / VE2 via feed + env basename/path even when load fails.
/// Residual class is **not** invented here — research surface only. Match ≠ Pass.
#[cfg(feature = "amd-xdna2")]
fn print_external_ctrltext_compare_dry_run_optional(
    feed: &dragon_npu::backends::amd_xdna2::LabExportFingerprintFeed,
    external: Option<&[u8]>,
    external_load_refused: bool,
) {
    use std::path::PathBuf;
    let env_path_for_dry = env::var(ENV_LAB_EXTERNAL_CTRLTEXT)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .map(PathBuf::from);
    let dry_basename = env_path_for_dry.as_ref().and_then(|p| {
        p.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .filter(|s| !s.is_empty())
    });
    let dry_platform = env_path_for_dry
        .as_ref()
        .map(|p| p.display().to_string())
        .filter(|s| !s.is_empty());
    // On load refuse without usable bytes, still feed basename/path for refuse matrix.
    // external_load_refused → detail dry_run_external_load_refused_blocked_on_bytes
    // (distinct from truly unset bag) when not post_patch/VE2 refuse.
    let dry_external = if external_load_refused {
        None
    } else {
        external
    };
    let dry = research_external_ctrltext_compare_dry_run_from_lab_feed_ex(
        feed,
        dry_external,
        dry_basename.as_deref(),
        dry_platform.as_deref(),
        None,
        None,
        external_load_refused,
    );
    println!(
        "external_ctrltext_compare_dry_run={}",
        dry.format_soft_note()
    );
}

/// Soft PROVENANCE + bag-drop + e2e fixture + offline progress scorecard
/// (stamps `8e898123` / `9cdfe8d0`) + bag+feed+dry-run composition (`2b01425e`).
///
/// Always honesty trio + residual_not_closed. Incomplete bag (no bytes offline)
/// → bag_drop_complete=0. Never invents Pass / C-ready / product known-good.
/// Scorecard alone never closes residual. Composition scorecard reports explicit
/// `blocked_on_external_bytes=1` after auto-feed when bag absent.
///
/// Bag-drop refuse matrix is fed even when env load fails: basename / path from
/// `XDNA2_LAB_EXTERNAL_CTRLTEXT` (post_patch / VE2 refuse detail). When env is
/// unset, soft also probes the lab bag for non-empty allowed `.bin`/`.ctrltext`
/// (bag-resident completeness). E2e uses last_apply `detail` for userspace
/// surface; journal codes only from `journal_text`. VE2 body-bounds surface is
/// tagged `info_source=ve2_latency platform_not_aie2=1`.
/// Residual class is **not** invented here — research surfaces only.
#[cfg(feature = "amd-xdna2")]
#[allow(clippy::too_many_arguments, clippy::too_many_lines)] // research soft print: bag + feed + dry-run composition
fn print_app_load_e2e_and_bag_drop_optional(
    class: dragon_npu::backends::amd_xdna2::SiliconResidualClass,
    journal_text: Option<&str>,
    detail: Option<&str>,
    env_bytes_present: bool,
    external_compare: Option<&dragon_npu::backends::amd_xdna2::ExternalCtrltextCompareResearch>,
    external_load_refused: bool,
    feed: &dragon_npu::backends::amd_xdna2::LabExportFingerprintFeed,
    external: Option<&[u8]>,
) {
    use std::path::PathBuf;
    let bag_any = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("testdata/xdna2_cmd/external_known_good/any.bin");
    let bag_prov = read_external_known_good_provenance_text(&bag_any);
    if let Some(ref text) = bag_prov {
        let chk = validate_external_known_good_provenance_text(text);
        println!(
            "external_known_good_provenance_check={}",
            chk.format_soft_note()
        );
    } else {
        println!(
            "external_known_good_provenance_check=absent invent_pass=0 not_c_ready=1 \
             not_ran_on_npu=1 product_known_good=0"
        );
    }

    // Env path basename + path string as platform_claim even on load fail
    // (refuse matrix: post_patch / VE2 detail on bag-drop line).
    let env_path = env::var(ENV_LAB_EXTERNAL_CTRLTEXT)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .map(PathBuf::from);
    let env_basename = env_path.as_ref().and_then(|p| {
        p.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .filter(|s| !s.is_empty())
    });
    // Path string feeds platform_claim_is_not_aie2 (ve2 / NOT_AIE2 markers in path).
    let env_platform_claim = env_path
        .as_ref()
        .map(|p| p.display().to_string())
        .filter(|s| !s.is_empty());

    // Bag-resident non-empty allowed ctrltext when env load absent.
    let (bag_resident, bag_basename, bag_path) = soft_probe_bag_resident_ctrltext();
    let bytes_present = env_bytes_present || bag_resident;
    let proposed_basename = env_basename.as_deref().or(bag_basename.as_deref());
    let platform_claim = env_platform_claim.as_deref();

    let bag_drop = check_external_known_good_bag_drop(
        bytes_present,
        bag_prov.as_deref(),
        proposed_basename,
        platform_claim,
    );
    println!(
        "external_known_good_bag_drop_check={}",
        bag_drop.format_soft_note()
    );
    // last_apply detail for userspace surface (journal codes only from journal_text).
    // VE2 body-bounds surface tagged platform_not_aie2 on soft line.
    let (axlf_prefix, body_from_ve2) = soft_axlf_header_prefix_optional();
    let e2e = research_app_load_e2e_fixture_path(
        class,
        detail,
        journal_text,
        axlf_prefix.as_deref(),
        &bag_drop,
    );
    if body_from_ve2 {
        println!(
            "app_load_e2e_fixture_research={} info_source=ve2_latency platform_not_aie2=1",
            e2e.format_soft_note()
        );
    } else {
        println!("app_load_e2e_fixture_research={}", e2e.format_soft_note());
    }
    // Offline progress scorecard (stamp 9cdfe8d0): compose class + bag + e2e +
    // compare status. Always residual_not_closed / scorecard alone ≠ close.
    // Residual class is the classifier `class` arg (never invent FW class).
    let scorecard = research_app_load_offline_progress_scorecard(
        class,
        &bag_drop,
        &e2e,
        external_compare,
        external_load_refused,
    );
    println!(
        "app_load_offline_progress_scorecard={}",
        scorecard.format_soft_note()
    );
    // Bag + lab feed + dry-run composition scorecard (stamp 2b01425e):
    // after auto-feed, bag absent → blocked_on_external_bytes=1 explicitly.
    // Never invents residual class / Pass / residual closed.
    // Prefer env external; when unset, try confined load of bag-resident path so
    // bag_drop_complete and compare path are not dual-flag silent. Fail →
    // explicit bag_resident_bytes_not_fed_to_compare=1 (not invent Pass).
    let mut bag_resident_fed: Option<Vec<u8>> = None;
    let mut bag_resident_not_fed = false;
    if external.is_none() && !external_load_refused {
        if let Some(ref p) = bag_path {
            match load_external_ctrltext_for_compare(p) {
                Ok(bytes) if !bytes.is_empty() => {
                    bag_resident_fed = Some(bytes);
                    println!(
                        "external_ctrltext_source=bag_resident_confined invent_pass=0 \
                         not_c_ready=1 not_ran_on_npu=1 product_known_good=0"
                    );
                }
                Ok(_) | Err(_) => {
                    bag_resident_not_fed = bag_resident;
                }
            }
        } else if bag_resident {
            bag_resident_not_fed = true;
        }
    } else if bag_resident && external.is_none() {
        // Env load refused or otherwise absent while bag-resident flagged present.
        bag_resident_not_fed = true;
    }
    if bag_resident_not_fed {
        println!(
            "bag_resident_bytes_not_fed_to_compare=1 invent_pass=0 not_c_ready=1 \
             not_ran_on_npu=1 product_known_good=0 residual_not_closed=1"
        );
    }
    let dry_external = if external_load_refused {
        None
    } else {
        external.or(bag_resident_fed.as_deref())
    };
    let bag_feed_dry = research_app_load_bag_feed_dry_run_scorecard_ex(
        &bag_drop,
        feed,
        dry_external,
        proposed_basename,
        platform_claim,
        external_load_refused,
    );
    println!(
        "app_load_bag_feed_dry_run_scorecard={}",
        bag_feed_dry.format_soft_note()
    );
    // Extract → bag-drop → composition scorecard pipeline (stamp 588c1c72):
    // public section markers without ctrltext still blocked_on_external_bytes=1.
    // Soft extract: optional research paste of xclbinutil section names only
    // (never invent product known-good / PDI layout). VE2 platform claim → refuse.
    let extract = soft_xclbin_extract_for_pipeline(platform_claim);
    let pipeline = research_app_load_extract_bag_scorecard_pipeline_ex(
        &extract,
        &bag_drop,
        feed,
        dry_external,
        proposed_basename,
        platform_claim,
        external_load_refused,
    );
    println!(
        "app_load_extract_bag_scorecard_pipeline={}",
        pipeline.format_soft_note()
    );
    // Operator extract-to-bag dry-run checklist (pure text; refuse tokens locked).
    println!(
        "external_known_good_operator_extract_checklist={}",
        format_external_known_good_operator_extract_checklist()
    );
}

/// Soft extract input for pipeline: optional section-name paste or VE2 refuse.
///
/// Env `XDNA2_LAB_XCLBIN_SECTION_TEXT` = research paste of public section **names**
/// (e.g. from `xclbinutil --info`) — never product known-good, never PDI decode.
/// Cap 64 KiB. When unset: empty extract (no invent markers). VE2 platform claim
/// uses locked refuse helper (`platform_not_aie2=1`).
#[cfg(feature = "amd-xdna2")]
fn soft_xclbin_extract_for_pipeline(
    platform_claim: Option<&str>,
) -> dragon_npu::backends::amd_xdna2::XclbinExtractResearchDryRun {
    const MAX_SECTION_TEXT: usize = 64 * 1024;
    let claim = platform_claim.unwrap_or("").trim();
    if !claim.is_empty()
        && dragon_npu::backends::amd_xdna2::platform_claim_is_not_aie2_known_good(claim)
    {
        return research_xclbin_extract_ve2_fixture_refused(&[], None);
    }
    let section_text = env::var("XDNA2_LAB_XCLBIN_SECTION_TEXT")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let section_text = section_text.as_deref().and_then(|t| {
        if t.len() > MAX_SECTION_TEXT {
            eprintln!(
                "WARN: XDNA2_LAB_XCLBIN_SECTION_TEXT oversized ({} > {}); ignored",
                t.len(),
                MAX_SECTION_TEXT
            );
            None
        } else {
            Some(t)
        }
    });
    research_xclbin_extract_dry_run(&[], section_text, platform_claim)
}

/// Soft probe: non-empty allowed `.bin`/`.ctrltext` under lab bag root.
///
/// Respects refuse basenames + extension policy. Does **not** invent product
/// known-good — only sets `ctrltext_bytes_present` input for bag-drop checklist
/// and optional confined load path for composition dry-run.
/// Returns `(present, first_basename, first_path)`.
#[cfg(feature = "amd-xdna2")]
fn soft_probe_bag_resident_ctrltext() -> (bool, Option<String>, Option<PathBuf>) {
    use dragon_npu::backends::amd_xdna2::{
        external_ctrltext_extension_ok, external_known_good_lab_root,
        is_refused_local_post_patch_as_known_good,
    };
    let root = external_known_good_lab_root();
    let Ok(rd) = fs::read_dir(&root) else {
        return (false, None, None);
    };
    for ent in rd.flatten() {
        let path = ent.path();
        if !path.is_file() {
            continue;
        }
        if !external_ctrltext_extension_ok(&path) {
            continue;
        }
        if is_refused_local_post_patch_as_known_good(&path) {
            continue;
        }
        let Ok(meta) = fs::metadata(&path) else {
            continue;
        };
        if meta.len() == 0 {
            continue;
        }
        let basename = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .filter(|s| !s.is_empty());
        return (true, basename, Some(path));
    }
    (false, None, None)
}

/// Soft axlf header prefix for e2e body-bounds surface.
///
/// Returns `(prefix_bytes, from_ve2_fixture)`. VE2 when present (never full body).
/// Caller must tag soft e2e with `info_source=ve2_latency platform_not_aie2=1`
/// when `from_ve2` is true — VE2 ≠ AIE2 product known-good.
#[cfg(feature = "amd-xdna2")]
fn soft_axlf_header_prefix_optional() -> (Option<Vec<u8>>, bool) {
    use std::io::Read;
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("testdata/xdna2_cmd/ve2_latency/sources/ve2_runner_latency_validate.xclbin");
    if !path.is_file() {
        return (None, false);
    }
    let Ok(mut f) = std::fs::File::open(&path) else {
        return (None, false);
    };
    let mut prefix = vec![
        0_u8;
        AXLF_FIRST_SECTION_HEADER_ABS_OFFSET
            + SOFT_AXLF_SECTION_WALK * AXLF_SECTION_HEADER_SIZE
    ];
    let Ok(n) = f.read(&mut prefix) else {
        return (None, false);
    };
    prefix.truncate(n);
    if prefix.is_empty() {
        (None, false)
    } else {
        (Some(prefix), true)
    }
}

/// Soft `xclbin_extract_research_dry_run=` — VE2 fixture refuse + public markers.
///
/// Uses staged `ve2_latency` header prefix + info text when present. When the
/// xclbin file is missing, header stays **empty** (`magic_xclbin2=0`) — do not
/// inject synthetic magic and claim disk magic. VE2 claim still refuses as
/// known-good. Always `product_known_good=0 invent_pass=0` (VE2 ≠ AIE2 bag).
/// Header read is capped at 64 B (magic only — no full 9 MiB load).
#[cfg(feature = "amd-xdna2")]
fn print_xclbin_extract_research_dry_run_optional() {
    use std::io::Read;
    use std::path::PathBuf;
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("testdata/xdna2_cmd/ve2_latency");
    let xclbin = root
        .join("sources")
        .join("ve2_runner_latency_validate.xclbin");
    let info_path = root.join("inspect").join("xclbin.info.txt");
    let mut header = Vec::new();
    let mut header_from_disk = false;
    if xclbin.is_file() {
        match std::fs::File::open(&xclbin) {
            Ok(mut f) => {
                let mut buf = [0_u8; 64];
                match f.read(&mut buf) {
                    Ok(n) if n > 0 => {
                        header.extend_from_slice(&buf[..n]);
                        header_from_disk = true;
                    }
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("WARN: ve2 xclbin header read {}: {e}", xclbin.display());
                    }
                }
            }
            Err(e) => {
                eprintln!("WARN: ve2 xclbin open {}: {e}", xclbin.display());
            }
        }
    }
    // Empty header when missing: magic_xclbin2=0; VE2 claim still refuse path.
    // Do not inject XCLBIN2_MAGIC and imply a successful disk read.
    let info_text = if info_path.is_file() {
        std::fs::read_to_string(&info_path).ok()
    } else {
        None
    };
    let r = research_xclbin_extract_ve2_fixture_refused(&header, info_text.as_deref());
    if header_from_disk {
        println!("xclbin_extract_research_dry_run={}", r.format_soft_note());
    } else {
        // Honest soft: no disk magic; still refuse VE2 as known-good.
        println!(
            "xclbin_extract_research_dry_run={} header_source=absent_no_disk_read",
            r.format_soft_note()
        );
    }
}

/// Soft `cu_pdi_section_identity_research=` from public section name markers.
///
/// Prefers staged `ve2_latency/inspect/xclbin.info.txt` when present (name scan
/// only — VE2 still ≠ AIE2 product known-good). Always tags
/// `info_source=ve2_latency platform_not_aie2=1` when that fixture path is used
/// so operators do not read CU/PDI name surface as working-stack evidence.
/// Distinguishes missing file / read error / empty content. Always
/// `section_identity_alone_closes_fw_residual=0 invent_pass=0 product_known_good=0`.
#[cfg(feature = "amd-xdna2")]
fn print_cu_pdi_section_identity_research_optional() {
    use std::path::PathBuf;
    // Soft prefers the staged VE2 inspect fixture path (name scan only).
    // Always tag `platform_not_aie2=1` on this path — VE2 ≠ AIE2 product known-good
    // even when the info text lists BITSTREAM/AIE_PARTITION section names.
    let info_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("testdata/xdna2_cmd/ve2_latency/inspect/xclbin.info.txt");
    if !info_path.is_file() {
        let r = research_cu_pdi_section_identity_from_text("");
        println!(
            "cu_pdi_section_identity_research={} info_source=absent_no_disk_read \
             platform_not_aie2=1 invent_pass=0 not_c_ready=1 not_ran_on_npu=1 \
             product_known_good=0",
            r.format_soft_note()
        );
        return;
    }
    match std::fs::read_to_string(&info_path) {
        Ok(text) if text.trim().is_empty() => {
            let r = research_cu_pdi_section_identity_from_text("");
            println!(
                "cu_pdi_section_identity_research={} info_source=ve2_latency \
                 platform_not_aie2=1 content=empty invent_pass=0 not_c_ready=1 \
                 not_ran_on_npu=1 product_known_good=0",
                r.format_soft_note()
            );
        }
        Ok(text) => {
            let r = research_cu_pdi_section_identity_from_text(&text);
            // VE2 fixture may list BITSTREAM+AIE_PARTITION names — surface flags
            // can be 1, but this is NOT AIE2 known-good / working-stack evidence.
            println!(
                "cu_pdi_section_identity_research={} info_source=ve2_latency \
                 platform_not_aie2=1 invent_pass=0 not_c_ready=1 not_ran_on_npu=1 \
                 product_known_good=0",
                r.format_soft_note()
            );
        }
        Err(e) => {
            eprintln!("WARN: ve2 xclbin info read {}: {e}", info_path.display());
            let r = research_cu_pdi_section_identity_from_text("");
            println!(
                "cu_pdi_section_identity_research={} info_source=read_failed \
                 platform_not_aie2=1 invent_pass=0 not_c_ready=1 not_ran_on_npu=1 \
                 product_known_good=0",
                r.format_soft_note()
            );
        }
    }
}

/// Soft `cu_pdi_section_body_bounds_research=` from public axlf headers + buffer len.
///
/// Same VE2 header-prefix path as the offset map: multi-MiB BITSTREAM windows are
/// **OOB vs prefix** (honest; never load full body / never decode PDI). Always
/// `body_bounds_alone_closes_fw_residual=0 invent_pass=0 product_known_good=0`
/// and `platform_not_aie2=1` on the VE2 path.
#[cfg(feature = "amd-xdna2")]
fn print_cu_pdi_section_body_bounds_research_optional() {
    use std::io::Read;
    use std::path::PathBuf;
    let prefix_cap =
        AXLF_FIRST_SECTION_HEADER_ABS_OFFSET + SOFT_AXLF_SECTION_WALK * AXLF_SECTION_HEADER_SIZE;
    let xclbin = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("testdata/xdna2_cmd/ve2_latency/sources/ve2_runner_latency_validate.xclbin");
    if !xclbin.is_file() {
        let r = research_xclbin_cu_pdi_section_body_bounds(&[]);
        println!(
            "cu_pdi_section_body_bounds_research={} info_source=absent_no_disk_read \
             platform_not_aie2=1 invent_pass=0 not_c_ready=1 not_ran_on_npu=1 \
             product_known_good=0",
            r.format_soft_note()
        );
        return;
    }
    match std::fs::File::open(&xclbin) {
        Ok(mut f) => {
            let mut buf = vec![0_u8; prefix_cap];
            match f.read(&mut buf) {
                Ok(n) if n > 0 => {
                    buf.truncate(n);
                    let r = research_xclbin_cu_pdi_section_body_bounds(&buf);
                    // Prefix is shorter than BITSTREAM body → any_oob=1 expected.
                    // Still NOT AIE2 product known-good / residual closed.
                    println!(
                        "cu_pdi_section_body_bounds_research={} info_source=ve2_latency \
                         platform_not_aie2=1 invent_pass=0 not_c_ready=1 not_ran_on_npu=1 \
                         product_known_good=0",
                        r.format_soft_note()
                    );
                }
                Ok(_) => {
                    let r = research_xclbin_cu_pdi_section_body_bounds(&[]);
                    println!(
                        "cu_pdi_section_body_bounds_research={} info_source=ve2_latency \
                         platform_not_aie2=1 content=empty invent_pass=0 not_c_ready=1 \
                         not_ran_on_npu=1 product_known_good=0",
                        r.format_soft_note()
                    );
                }
                Err(e) => {
                    eprintln!("WARN: ve2 xclbin header read {}: {e}", xclbin.display());
                    let r = research_xclbin_cu_pdi_section_body_bounds(&[]);
                    println!(
                        "cu_pdi_section_body_bounds_research={} info_source=read_failed \
                         platform_not_aie2=1 invent_pass=0 not_c_ready=1 not_ran_on_npu=1 \
                         product_known_good=0",
                        r.format_soft_note()
                    );
                }
            }
        }
        Err(e) => {
            eprintln!("WARN: ve2 xclbin open {}: {e}", xclbin.display());
            let r = research_xclbin_cu_pdi_section_body_bounds(&[]);
            println!(
                "cu_pdi_section_body_bounds_research={} info_source=read_failed \
                 platform_not_aie2=1 invent_pass=0 not_c_ready=1 not_ran_on_npu=1 \
                 product_known_good=0",
                r.format_soft_note()
            );
        }
    }
}

/// Soft `cu_pdi_section_offset_map_research=` from public axlf section headers.
///
/// Prefers staged VE2 `validate.xclbin` **header prefix only** (not full body /
/// no PDI decode). When present, always tags `info_source=ve2_latency
/// platform_not_aie2=1` so operators do not read offset surface as AIE2
/// product known-good. Distinguishes missing file / read error. Always
/// `section_offset_map_alone_closes_fw_residual=0 invent_pass=0 product_known_good=0`.
///
/// Prefix size uses public layout constants + [`SOFT_AXLF_SECTION_WALK`] (16),
/// not the full pure walk cap (256), so soft stays header-only.
#[cfg(feature = "amd-xdna2")]
fn print_cu_pdi_section_offset_map_research_optional() {
    use std::io::Read;
    use std::path::PathBuf;
    // Header + section table only. Never load full multi-MiB section bodies.
    let prefix_cap =
        AXLF_FIRST_SECTION_HEADER_ABS_OFFSET + SOFT_AXLF_SECTION_WALK * AXLF_SECTION_HEADER_SIZE;
    let xclbin = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("testdata/xdna2_cmd/ve2_latency/sources/ve2_runner_latency_validate.xclbin");
    if !xclbin.is_file() {
        let r = research_xclbin_cu_pdi_section_offset_map(&[]);
        println!(
            "cu_pdi_section_offset_map_research={} info_source=absent_no_disk_read \
             platform_not_aie2=1 invent_pass=0 not_c_ready=1 not_ran_on_npu=1 \
             product_known_good=0",
            r.format_soft_note()
        );
        return;
    }
    match std::fs::File::open(&xclbin) {
        Ok(mut f) => {
            let mut buf = vec![0_u8; prefix_cap];
            match f.read(&mut buf) {
                Ok(n) if n > 0 => {
                    buf.truncate(n);
                    let r = research_xclbin_cu_pdi_section_offset_map(&buf);
                    // VE2 may show BITSTREAM+AIE_PARTITION offsets — still NOT
                    // AIE2 product known-good / working-stack evidence.
                    println!(
                        "cu_pdi_section_offset_map_research={} info_source=ve2_latency \
                         platform_not_aie2=1 invent_pass=0 not_c_ready=1 not_ran_on_npu=1 \
                         product_known_good=0",
                        r.format_soft_note()
                    );
                }
                Ok(_) => {
                    let r = research_xclbin_cu_pdi_section_offset_map(&[]);
                    println!(
                        "cu_pdi_section_offset_map_research={} info_source=ve2_latency \
                         platform_not_aie2=1 content=empty invent_pass=0 not_c_ready=1 \
                         not_ran_on_npu=1 product_known_good=0",
                        r.format_soft_note()
                    );
                }
                Err(e) => {
                    eprintln!("WARN: ve2 xclbin header read {}: {e}", xclbin.display());
                    let r = research_xclbin_cu_pdi_section_offset_map(&[]);
                    println!(
                        "cu_pdi_section_offset_map_research={} info_source=read_failed \
                         platform_not_aie2=1 invent_pass=0 not_c_ready=1 not_ran_on_npu=1 \
                         product_known_good=0",
                        r.format_soft_note()
                    );
                }
            }
        }
        Err(e) => {
            eprintln!("WARN: ve2 xclbin open {}: {e}", xclbin.display());
            let r = research_xclbin_cu_pdi_section_offset_map(&[]);
            println!(
                "cu_pdi_section_offset_map_research={} info_source=read_failed \
                 platform_not_aie2=1 invent_pass=0 not_c_ready=1 not_ran_on_npu=1 \
                 product_known_good=0",
                r.format_soft_note()
            );
        }
    }
}

/// Soft `instr_pdi_load_research=` when a full staged mladf aiebu is on disk.
///
/// Skips the 1072 B `lab_instr.elf` stub (rela×1). Missing asset → one SKIP line.
/// ELF read is size-capped via [`read_lab_file_capped`] (same 64 MiB lab ceiling).
///
/// Returns Track K `userspace_ok_fw_load_open` when a full staged profile ran;
/// `false` on soft-skip (Track L must not invent userspace content green).
#[cfg(feature = "amd-xdna2")]
fn print_instr_pdi_load_research_optional() -> bool {
    use std::path::PathBuf;
    let roots = [
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("testdata/xdna2_cmd/silicon_lab/lab_instr.elf"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("testdata/xdna2_cmd/gt_capture/instr_pre.elf"),
    ];
    for pth in &roots {
        if !pth.is_file() {
            continue;
        }
        // Cap before full read / profile (refuse sparse/huge files).
        let elf = match read_lab_file_capped(pth.to_str().unwrap_or(""), "instr_pdi_load_research")
        {
            Ok(e) => e,
            Err(e) => {
                eprintln!("WARN: instr_pdi ELF refuse {}: {e}", pth.display());
                continue;
            }
        };
        let r = profile_instr_payload_after_rela_c2_locked(&elf);
        // Full staged mladf only (×12 / 1688) — never hard-assert the research stub.
        if r.rela_audit.rela_count != Some(12) || r.rela_audit.patched_ctrltext_len != Some(1688) {
            continue;
        }
        println!("instr_pdi_load_research={}", r.format_soft_note());
        return r.userspace_ok_fw_load_open;
    }
    println!(
        "instr_pdi_load_research=OFFLINE_ASSET_SKIP not_c_ready=1 not_ran_on_npu=1 invent_pass=0"
    );
    false
}

#[cfg(feature = "amd-xdna2")]
#[allow(clippy::too_many_arguments)]
fn print_report(
    path: &Path,
    class: dragon_npu::backends::amd_xdna2::SiliconResidualClass,
    status: Option<&str>,
    aiebu: Option<bool>,
    ert_error: Option<bool>,
    rela_count_field: Option<usize>,
    ran_field: Option<bool>,
    detail: Option<&str>,
    sig: Option<&dragon_npu::backends::amd_xdna2::LastApplyDetailSignals>,
    note: &str,
) {
    println!("=== classify_last_apply (offline residual; soft report) ===");
    println!("path={}", path.display());
    println!("residual_class={}", class.as_str());
    println!(
        "locked_app_load_after_aiebu={}",
        class.is_locked_app_load_after_aiebu()
    );
    if let Some(s) = status {
        println!("status={s}");
    }
    if let Some(b) = aiebu {
        println!("aiebu_elf={b}");
    }
    if let Some(b) = ert_error {
        println!("ert_error_field={b}");
    }
    if let Some(n) = rela_count_field {
        println!("rela_count_field={n}");
    }
    // Shared journal paste for checklist dual + Track L research (never invent).
    let journal_owned = resolve_lab_journal_text();
    let journal = journal_owned.as_deref();
    if let Some(s) = sig {
        print_sig_detail(class, s, journal);
    }
    if let Some(r) = ran_field {
        println!("json_ran_on_npu={r}");
    }
    print_c2_cmdbuf_research_optional(class, journal, detail);
    println!("note={note}");
    println!("not_c_ready=1");
    println!("not_ran_on_npu=1");
    println!(
        "CLASSIFY_SOFT_OK residual={} (report only; ≠ C-ready ≠ ran_on_npu)",
        class.as_str()
    );
}

/// Cap soft classify reads (same lab file ceiling as WTS/OFM loaders).
///
/// Unix: open with `O_NOFOLLOW` so a final-component symlink race after
/// [`last_apply_load_path_is_allowed`] cannot redirect the read. Intermediate
/// dir symlinks are already refused by parent canonicalize in path policy.
#[cfg(feature = "amd-xdna2")]
fn read_capped(path: &Path) -> Result<String, String> {
    let meta = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if meta.file_type().is_symlink() {
        return Err("path is a symlink (refused)".into());
    }
    let len = meta.len();
    if len > MAX_MLADF_LAB_FILE_BYTES {
        return Err(format!(
            "file is {len} bytes > max {MAX_MLADF_LAB_FILE_BYTES}"
        ));
    }
    let mut f = open_nofollow(path).map_err(|e| e.to_string())?;
    let mut buf = String::new();
    // Cap at MAX even if sparse growth races metadata.
    f.by_ref()
        .take(MAX_MLADF_LAB_FILE_BYTES)
        .read_to_string(&mut buf)
        .map_err(|e| e.to_string())?;
    if (buf.len() as u64) > MAX_MLADF_LAB_FILE_BYTES {
        return Err(format!(
            "read size {} > max {MAX_MLADF_LAB_FILE_BYTES}",
            buf.len()
        ));
    }
    Ok(buf)
}

/// Open for read without following a final symlink (Unix `O_NOFOLLOW`).
#[cfg(all(feature = "amd-xdna2", unix))]
fn open_nofollow(path: &Path) -> std::io::Result<fs::File> {
    use std::os::unix::fs::OpenOptionsExt;
    fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
}

#[cfg(all(feature = "amd-xdna2", not(unix)))]
fn open_nofollow(path: &Path) -> std::io::Result<fs::File> {
    // Non-Unix: residual TOCTOU after symlink_metadata refuse in load path.
    fs::File::open(path)
}

/// Resolve default path: env (if allowed+file) → testdata fixture → newest
/// pid-suffixed snapshot under `temp_dir()` (never fixed world plant name).
#[cfg(feature = "amd-xdna2")]
fn resolve_default_path() -> PathBuf {
    for key in ["XDNA2_LAB_LAST_APPLY_JSON", "XDNA2_LAST_APPLY_JSON"] {
        if let Ok(p) = env::var(key) {
            let t = p.trim();
            if t.is_empty() {
                continue;
            }
            let pb = PathBuf::from(t);
            if !last_apply_load_path_is_allowed(&pb) {
                eprintln!(
                    "WARN: {key} path refused by load confinement: {}",
                    pb.display()
                );
                continue;
            }
            if pb.is_file() {
                return pb;
            }
            eprintln!("WARN: {key} set but not a file: {}", pb.display());
        }
    }
    // Prefer shared default (testdata when present; else private pid temp name).
    let def = default_last_apply_path();
    if last_apply_load_path_is_allowed(&def) && def.is_file() {
        return def;
    }
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/xdna2_cmd/last_apply.json");
    if last_apply_load_path_is_allowed(&repo) && repo.is_file() {
        return repo;
    }
    // Newest dragon_npu_last_apply_*.json under temp_dir() (writer pid pattern).
    if let Some(p) = newest_temp_last_apply() {
        return p;
    }
    // Fall back so caller prints missing error with a stable path.
    repo
}

#[cfg(feature = "amd-xdna2")]
fn newest_temp_last_apply() -> Option<PathBuf> {
    // Clamped temp root (never read_dir("/") when TMPDIR=/).
    let tmp = last_apply_temp_root();
    if tmp == Path::new("/") {
        // Defense-in-depth: clamp already maps root → /tmp; refuse root scan.
        return None;
    }
    let mut newest: Option<(PathBuf, std::time::SystemTime)> = None;
    let rd = fs::read_dir(&tmp).ok()?;
    for ent in rd.flatten() {
        let name = ent.file_name();
        let s = name.to_string_lossy();
        // Writer pattern: dragon_npu_last_apply_{pid}.json — refuse bare plant name
        // `dragon_npu_last_apply.json` without underscore-suffix.
        if !s.starts_with("dragon_npu_last_apply_") || !s.ends_with(".json") {
            continue;
        }
        let path = ent.path();
        if !last_apply_load_path_is_allowed(&path) || !path.is_file() {
            continue;
        }
        let mt = ent
            .metadata()
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        match &newest {
            None => newest = Some((path, mt)),
            Some((_, t)) if mt > *t => newest = Some((path, mt)),
            _ => {}
        }
    }
    newest.map(|(p, _)| p)
}

#[cfg(feature = "amd-xdna2")]
#[allow(clippy::too_many_lines)] // research soft-token help dump (PRINT surface catalog)
fn print_help() {
    eprintln!(
        "classify_last_apply — soft offline residual class from last_apply.json\n\
         \n\
         Usage:\n\
           cargo run --features amd-xdna2 --example classify_last_apply -- [path.json]\n\
           bash tools/classify_last_apply_residual.sh [path.json]\n\
         \n\
         Env (optional path; must pass load confinement):\n\
           XDNA2_LAB_LAST_APPLY_JSON / XDNA2_LAST_APPLY_JSON\n\
           bash:  env XDNA2_LAB_LAST_APPLY_JSON=/path/to.json bash tools/classify_last_apply_residual.sh\n\
           fish:  set -gx XDNA2_LAB_LAST_APPLY_JSON /path/to.json; bash tools/classify_last_apply_residual.sh\n\
         \n\
         Optional research envs (≠ C-ready / ≠ Pass / never open devices):\n\
           XDNA2_LAB_JOURNAL_TEXT=…   inline journal **paste text** (not a file path);\n\
                                     dual FW codes when literals parse; cap 256 KiB;\n\
                                     oversized → journal_text_refused=oversized\n\
           XDNA2_LAB_PRINT_C2_CMDBUF_RESEARCH=1|true\n\
                                     soft research lines (Tracks F/G/K/L/M/N/O + construction):\n\
                                       c2_cmdbuf_map= / c2_cmdlist_msg=\n\
                                       c2_cmdlist_fill_cross_check=\n\
                                       construction_path_research= (staged full aiebu pure path)\n\
                                       ctrltext_fingerprint_export= (unpatched identity;\n\
                                         known_good always absent_awaiting_capture offline)\n\
                                       external_ctrltext_compare= (default absent external;\n\
                                         never treats instr_post_patch as known-good;\n\
                                         sha_match ≠ Pass / product_known_good=0)\n\
                                       cu_pdi_necessity_research= (gap + FW APP_LOAD matrix;\n\
                                         match alone never closes residual)\n\
                                       app_load_residual_axis_matrix= (partition name vs\n\
                                         CU/PDI gap vs ctrltext-only; 0x3000003 axes;\n\
                                         match alone / name alone never close residual)\n\
                                       xclbin_extract_research_dry_run= (public markers +\n\
                                         VE2 NOT_AIE2 refuse; never AIE2 known-good invent)\n\
                                       cu_pdi_section_identity_research= (AIE_PARTITION/\n\
                                         BITSTREAM name surface; identity alone ≠ residual closed)\n\
                                       cu_pdi_section_offset_map_research= (BITSTREAM/\n\
                                         AIE_PARTITION file offsets from axlf headers;\n\
                                         VE2 prefix; map alone ≠ residual closed;\n\
                                         platform_not_aie2=1)\n\
                                       cu_pdi_section_body_bounds_research= (body window\n\
                                         vs buffer len from offset map; prefix OOB honest;\n\
                                         no body decode; bounds alone ≠ residual closed;\n\
                                         platform_not_aie2=1)\n\
                                       app_load_partition_id_field_map= (0x3000003 fields;\n\
                                         partition_id always absent_public; map alone ≠ close)\n\
                                       external_known_good_provenance_check= (field markers;\n\
                                         research-ready ≠ product known-good)\n\
                                       external_known_good_bag_drop_check= (fail-closed\n\
                                         completeness: bag-resident OR env bytes + PROVENANCE;\n\
                                         refuse post_patch/VE2 via env basename/path even on\n\
                                         load fail; complete ≠ Pass / residual closed)\n\
                                       app_load_e2e_fixture_research= (orchestrates residual\n\
                                         class + last_apply detail + journal tokens + body\n\
                                         bounds + bag drop; VE2 body surface tagged\n\
                                         info_source=ve2_latency platform_not_aie2=1;\n\
                                         always residual_not_closed=1 invent_pass=0)\n\
                                       app_load_offline_progress_scorecard= (composes class +\n\
                                         bag-drop + e2e residual_not_closed + journal token\n\
                                         presence + external_compare_status absent|loaded|\n\
                                         match|mismatch|refused; scorecard alone ≠ close;\n\
                                         always residual_not_closed=1 invent_pass=0)\n\
                                       app_load_bag_feed_dry_run_scorecard= (stamp 2b01425e;\n\
                                         composes bag-drop + lab feed + dry-run auto-feed;\n\
                                         bag absent after feed → blocked_on_external_bytes=1;\n\
                                         lab ready path; refuse post_patch/VE2 platform_not_aie2;\n\
                                         match ≠ Pass; scorecard alone ≠ close; invent_pass=0)\n\
                                       app_load_extract_bag_scorecard_pipeline= (stamp 588c1c72;\n\
                                         extract markers + bag-drop + composition scorecard;\n\
                                         markers without ctrltext still blocked_on_external_bytes=1;\n\
                                         extract alone ≠ close; pipeline alone ≠ close; invent_pass=0)\n\
                                       external_known_good_operator_extract_checklist= (pure\n\
                                         extract-to-bag steps; refuse post_patch/VE2;\n\
                                         honesty footer; no invent bytes)\n\
                                       lab_export_fingerprint_feed= (stamp d2e287ab;\n\
                                         auto-feed lab fingerprint from export ELF path +\n\
                                         last_apply signals; ready when ELF export has sha;\n\
                                         signals alone incomplete; refuse post_patch/VE2;\n\
                                         feed alone ≠ residual closed; invent_pass=0)\n\
                                       external_ctrltext_compare_dry_run= (stamp d2e287ab auto-feed;\n\
                                         bag absent → blocked_on_external_bytes=1; lab_export_ready\n\
                                         from feed; refuse post_patch/VE2; match ≠ Pass;\n\
                                         dry-run alone ≠ close; invent_pass=0 residual_not_closed=1)\n\
                                       instr_pdi_load_research= (full staged ELF or SKIP)\n\
                                       app_load_pdi_partition_research= (public status-name only;\n\
                                         full C2 dual pair needs XDNA2_LAB_JOURNAL_TEXT paste;\n\
                                         without journal: failed_cmd=absent / pair=0 still OK)\n\
                                       public_pdi_evidence_scan= (Track M closed lock; pure\n\
                                         status-name re-check; new_public_pdi_fields=0; no FS)\n\
                                       external_public_pdi_evidence_scan= (Track N closed_no_new;\n\
                                         external_kernel_xrt_beyond_status_name=0; no XRT walk)\n\
                                       app_load_maintain_reconfirm= (Track O A–N still closed;\n\
                                         compose M+N field echoes; no FS/network/XRT walk;\n\
                                         maintain_reconfirm=a_n_closed_no_new;\n\
                                         ≠ C-ready ≠ silicon progress / not new evidence)\n\
                                     always invent_pass=0; 0/false/empty does **not** enable\n\
           XDNA2_LAB_EXTERNAL_CTRLTEXT=/path/to.bin\n\
                                     optional external ctrltext for compare (with PRINT flag):\n\
                                     confined to temp_dir or\n\
                                     testdata/xdna2_cmd/external_known_good/; .bin/.ctrltext;\n\
                                     refuse local post_patch basenames; lab bag needs\n\
                                     PROVENANCE.md; O_NOFOLLOW open; load ≠ Pass /\n\
                                     ≠ product_known_good\n\
           XDNA2_LAB_XCLBIN_SECTION_TEXT=…\n\
                                     research paste of public xclbin section **names** only\n\
                                     (e.g. AIE_PARTITION BITSTREAM); cap 64 KiB; feeds extract\n\
                                     half of app_load_extract_bag_scorecard_pipeline= under PRINT;\n\
                                     never product known-good / never PDI body decode; markers\n\
                                     without bag bytes still blocked_on_external_bytes=1\n\
         \n\
         Exit: 0 soft report (CLASSIFY_SOFT_OK); 2 missing/bad/refused path or usage.\n\
         Never claims Pass / C-ready / ran_on_npu. CONTRACT_OK ≠ C-ready.\n\
         Token note: research userspace_ok_fw_load_open= ≠ checklist\n\
         checklist_userspace_elf_rela_app_load=; Track L research\n\
         partition_surface_public= ≠ checklist_partition_surface_public=;\n\
         checklist L tokens are **class-gated** (APP_LOAD only); dual residual\n\
         pair-OR is research/PRINT only. Both pair with invent_pass=0.\n\
         No private PDI decode."
    );
}
