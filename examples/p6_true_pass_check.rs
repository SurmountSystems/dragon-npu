//! Soft offline P6 true-Pass criteria check (no L3 / no EXEC).
//!
//! Prints SSoT [`format_true_pass_criteria_checklist`] and evaluates the same
//! snapshot path as C4 via [`evaluate_silicon_pass_evidence`] +
//! [`format_true_pass_p6_note`]. **Never invents product Pass.**
//!
//! Locked APP_LOAD fixture → `P6_NOT_PASS` / `P6_SKIP` (soft exit 0).
//! Offline C4 PassCandidate → still `product_pass=0` + live-decide required.
//!
//! ```bash
//! cargo run --features amd-xdna2 --example p6_true_pass_check --quiet -- \
//!   testdata/xdna2_cmd/last_apply.json
//! # or: bash tools/p6_true_pass_check.sh
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
use std::process::ExitCode;

#[cfg(feature = "amd-xdna2")]
use earth_dragon_npu::backends::amd_xdna2::{
    MAX_MLADF_LAB_FILE_BYTES, SiliconPassEval, SiliconPassEvidence, default_last_apply_path,
    evaluate_silicon_pass_evidence, format_silicon_pass_eval_note,
    format_true_pass_criteria_checklist, format_true_pass_p6_note, last_apply_load_path_is_allowed,
    silicon_pass_evidence_from_last_apply,
};

fn main() -> ExitCode {
    #[cfg(not(feature = "amd-xdna2"))]
    {
        eprintln!("p6_true_pass_check requires --features amd-xdna2");
        ExitCode::from(2)
    }
    #[cfg(feature = "amd-xdna2")]
    {
        ExitCode::from(run())
    }
}

#[cfg(feature = "amd-xdna2")]
fn run() -> u8 {
    let mut args: Vec<String> = env::args().skip(1).collect();
    if args.iter().any(|a| a == "-h" || a == "--help") {
        print_help();
        return 0;
    }
    if args.first().is_some_and(|a| a == "--") {
        args.remove(0);
    }

    let Ok((require_pass, path_arg)) = parse_args(&args) else {
        return 2;
    };

    // Always print SSoT criteria first (offline readiness; ≠ product Pass).
    let criteria = format_true_pass_criteria_checklist();
    println!("=== p6_true_pass_check (offline; ≠ invent product Pass) ===");
    println!("criteria={criteria}");

    let path = path_arg.map_or_else(resolve_default_path, PathBuf::from);
    if !last_apply_load_path_is_allowed(&path) {
        eprintln!(
            "ERROR: last_apply path not allowed (must be under temp_dir or \
             testdata/xdna2_cmd, .json only, no .., no symlink): {}",
            path.display()
        );
        println!(
            "P6_REFUSE path invent_pass=0 not_c_ready=1 not_ran_on_npu=1 \
             product_pass_requires_live_decide=1"
        );
        return 2;
    }

    if !path.is_file() {
        eprintln!("ERROR: missing last_apply path: {}", path.display());
        eprintln!(
            "  Pass an explicit path or set XDNA2_LAB_LAST_APPLY_JSON / stage a HITL C snapshot."
        );
        if require_pass {
            return 2;
        }
        println!(
            "P6_SKIP no_last_apply invent_pass=0 not_c_ready=1 not_ran_on_npu=1 \
             product_pass_requires_live_decide=1 offline_audit_only=1 product_pass=0"
        );
        return 0;
    }

    audit_path(&path, require_pass)
}

#[cfg(feature = "amd-xdna2")]
fn parse_args(args: &[String]) -> Result<(bool, Option<String>), ()> {
    let mut require_pass = false;
    let mut path_arg: Option<String> = None;
    for a in args {
        match a.as_str() {
            "--require-pass" => require_pass = true,
            s if s.starts_with('-') => {
                eprintln!("ERROR: unknown flag {s} (allowlist: --require-pass --help)");
                print_help();
                return Err(());
            }
            s => {
                if path_arg.is_some() {
                    eprintln!("ERROR: only one path argument allowed");
                    print_help();
                    return Err(());
                }
                path_arg = Some(s.to_owned());
            }
        }
    }
    Ok((require_pass, path_arg))
}

#[cfg(feature = "amd-xdna2")]
fn audit_path(path: &Path, require_pass: bool) -> u8 {
    let text = match read_capped(path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("ERROR: read {}: {e}", path.display());
            return 2;
        }
    };
    let v: serde_json::Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("ERROR: json parse {}: {e}", path.display());
            return 2;
        }
    };

    let detail = v.get("detail").and_then(serde_json::Value::as_str);
    let status = v.get("status").and_then(serde_json::Value::as_str);
    let ran = v.get("ran_on_npu").and_then(serde_json::Value::as_bool);
    let ert_error = v.get("ert_error").and_then(serde_json::Value::as_bool);
    let real_ioctl = v.get("real_ioctl").and_then(serde_json::Value::as_bool);

    // Reuse C4 evaluator — do not fork a second Pass gate.
    let ev = silicon_pass_evidence_from_last_apply(ran, status, detail, ert_error, real_ioctl);
    let eval = evaluate_silicon_pass_evidence(&ev);
    let c4_note = format_silicon_pass_eval_note(&eval);
    let p6_note = format_true_pass_p6_note(&eval);

    print_report(
        path, &eval, ran, status, real_ioctl, &ev, &c4_note, &p6_note,
    );

    if eval.is_pass_candidate() {
        // Offline candidate still ≠ product Pass (live decide owns product flag).
        println!(
            "P6_OFFLINE_CANDIDATE product_pass=0 invent_pass=0 not_ran_on_npu=1 \
             not_c_ready=1 product_pass_requires_live_decide=1 offline_audit_only=1 \
             c4_pass_candidate_neq_product_pass=1"
        );
        // --require-pass means offline PassCandidate present; still never product Pass.
        return 0;
    }

    let reason = match &eval {
        SiliconPassEval::NotPass { reason } => *reason,
        SiliconPassEval::PassCandidate { .. } => "unreachable",
    };
    println!(
        "P6_NOT_PASS no_product_pass reason={reason} invent_pass=0 not_c_ready=1 \
         not_ran_on_npu=1 product_pass=0 product_pass_requires_live_decide=1 \
         offline_audit_only=1"
    );
    // Soft skip token for operators (same soft exit 0 as C4_SKIP on fixture).
    println!(
        "P6_SKIP locked_or_not_candidate reason={reason} invent_pass=0 not_ran_on_npu=1 \
         product_pass_requires_live_decide=1"
    );

    if require_pass {
        eprintln!(
            "ERROR: --require-pass and not offline PassCandidate (reason={reason}); \
             never claims product Pass"
        );
        return 2;
    }
    0
}

#[cfg(feature = "amd-xdna2")]
#[allow(clippy::too_many_arguments)]
fn print_report(
    path: &Path,
    eval: &SiliconPassEval,
    ran: Option<bool>,
    status: Option<&str>,
    real_ioctl: Option<bool>,
    ev: &SiliconPassEvidence,
    c4_note: &str,
    p6_note: &str,
) {
    println!("path={}", path.display());
    println!("c4_eval={}", eval.as_str());
    if let Some(r) = ran {
        println!("json_ran_on_npu={r}");
    } else {
        println!("json_ran_on_npu=missing");
    }
    if let Some(s) = status {
        println!("status={s}");
    }
    if let Some(b) = real_ioctl {
        println!("real_ioctl={b}");
    }
    if let Some(st) = ev.ert_state {
        println!("ert_state={st}");
    }
    if let Some(nz) = ev.ofm_raw_nonzero_bytes {
        println!("ofm_raw_nonzero_bytes={nz}");
    }
    println!("c4_note={c4_note}");
    println!("p6_note={p6_note}");
    println!(
        "honesty: product Pass = live decide_ran_on_npu only \
         (ERT Completed + non-zero OFM allclose + real_ioctl); \
         offline C4/P6 ≠ invent ran_on_npu"
    );
}

/// Cap soft reads (same lab file ceiling as classify / c4).
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
    fs::File::open(path)
}

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
    let def = default_last_apply_path();
    if last_apply_load_path_is_allowed(&def) && def.is_file() {
        return def;
    }
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/xdna2_cmd/last_apply.json");
    if last_apply_load_path_is_allowed(&repo) && repo.is_file() {
        return repo;
    }
    if let Some(p) = newest_temp_last_apply() {
        return p;
    }
    repo
}

#[cfg(feature = "amd-xdna2")]
fn newest_temp_last_apply() -> Option<PathBuf> {
    let tmp = env::temp_dir();
    let mut newest: Option<(PathBuf, std::time::SystemTime)> = None;
    let rd = fs::read_dir(&tmp).ok()?;
    for ent in rd.flatten() {
        let name = ent.file_name();
        let s = name.to_string_lossy();
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
        "p6_true_pass_check — offline true product-Pass criteria (P6 readiness)\n\
         \n\
         Usage:\n\
           cargo run --features amd-xdna2 --example p6_true_pass_check -- [path.json]\n\
           cargo run --features amd-xdna2 --example p6_true_pass_check -- --require-pass [path.json]\n\
           bash tools/p6_true_pass_check.sh\n\
           bash tools/p6_true_pass_check.sh --require-pass\n\
         \n\
         Env (optional path; must pass load confinement):\n\
           XDNA2_LAB_LAST_APPLY_JSON / XDNA2_LAST_APPLY_JSON\n\
         \n\
         Exit:\n\
           0  P6_SKIP / P6_NOT_PASS (fixture) or P6_OFFLINE_CANDIDATE (still ≠ product Pass)\n\
           2  --require-pass and not offline PassCandidate; bad/refused path; usage\n\
         \n\
         Honesty: offline P6/C4 ≠ invent product Pass / ≠ ran_on_npu claim.\n\
         Product Pass = live decide_ran_on_npu only. Never sets ALLOW. No L3 / no EXEC."
    );
}
