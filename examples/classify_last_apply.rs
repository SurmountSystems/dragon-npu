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
    C2ExpectPaddrs, MAX_MLADF_LAB_FILE_BYTES, c2_live_cmdlist_msg_words_embedded,
    c2_live_io_paddrs_locked, classify_last_apply_residual_ex, cross_check_c2_cmdlist_fill_locked,
    decode_c2_live_cmdbuf_vs_pure_rebuild, default_last_apply_path,
    format_app_load_offline_checklist, format_app_load_offline_checklist_ex,
    format_c2_cmdlist_msg_research_note, format_silicon_residual_class_note,
    last_apply_bo_gt_content, last_apply_has_rela_evidence, last_apply_load_path_is_allowed,
    last_apply_start_npu_matches_content, last_apply_temp_root, parse_journal_fw_status_pair,
    parse_last_apply_detail_signals, profile_instr_payload_after_rela_c2_locked,
    read_lab_file_capped, rebuild_c2_live_start_npu_cmd_bo, reconfirm_app_load_tracks_a_n_closed,
    research_app_load_pdi_partition, scan_external_public_pdi_evidence,
    scan_public_pdi_partition_evidence,
};

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
#[cfg(feature = "amd-xdna2")]
fn print_c2_cmdbuf_research_optional(
    class: dragon_npu::backends::amd_xdna2::SiliconResidualClass,
    journal_text: Option<&str>,
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
    print_c2_cmdbuf_research_optional(class, journal);
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
                                     soft research lines (Tracks F/G/K/L/M/N/O):\n\
                                       c2_cmdbuf_map= / c2_cmdlist_msg=\n\
                                       c2_cmdlist_fill_cross_check=\n\
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
