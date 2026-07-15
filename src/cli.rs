//! Thin argv helpers for the `dragon-npu` binary (library-testable).
//!
//! Parsing is pure and free of hardware side effects so unit tests never open
//! `/dev/accel` or enable EXEC_CMD. The binary (`src/bin/dragon_npu.rs`) owns
//! process exit codes and I/O.
//!
//! ## Safety
//!
//! This CLI surface does **not** enable `XDNA2_ALLOW_EXEC_CMD` or the
//! `xdna2-dangerous-exec-cmd` feature. See `docs/XDNA2_NEVER_AGAIN.md`.

/// Process exit: success.
pub const EXIT_OK: i32 = 0;
/// Process exit: runtime / library error.
pub const EXIT_ERROR: i32 = 1;
/// Process exit: usage / bad argv.
pub const EXIT_USAGE: i32 = 2;

/// Maximum f32 elements for CLI zero-fill or loaded input (caps OOM via `--input-len`).
///
/// 16 Mi elements ≈ 64 MiB of f32 — enough for small/medium demos, not multi-GB tensors.
pub const MAX_INPUT_ELEMENTS: usize = 16_777_216;

/// Parsed top-level subcommand for `dragon-npu`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Print NPU / backend status.
    Status {
        /// Emit a single JSON object instead of human text.
        json: bool,
    },
    /// Compile a model path (ONNX / JSON / heuristic); no inference.
    Compile {
        /// Model file path.
        model: String,
        /// Optimization level 0–3.
        opt: u8,
        /// Quantization string (`fp32`, `fp16`, `int8`, …).
        quant: String,
        /// JSON summary output.
        json: bool,
    },
    /// Compile + run with zeros or file input (host/sim path for this CLI).
    Run {
        /// Model file path.
        model: String,
        /// Optimization level 0–3.
        opt: u8,
        /// Quantization string.
        quant: String,
        /// Optional path to input (JSON float list or raw little-endian f32).
        input: Option<String>,
        /// Length of zero input when `--input` is omitted.
        input_len: usize,
        /// JSON summary output.
        json: bool,
    },
    /// Probe-ish diagnostics (requires `amd-xdna2` at runtime; never EXEC_CMD).
    Probe {
        /// JSON summary.
        json: bool,
    },
    /// Print help text.
    Help,
}

/// Default optimization level for compile/run when `-O` is omitted.
pub const DEFAULT_OPT_LEVEL: u8 = 2;
/// Default quantization label when `-q` is omitted.
pub const DEFAULT_QUANT: &str = "fp32";
/// Default zero-vector length for `run` without `--input`.
pub const DEFAULT_INPUT_LEN: usize = 16;

/// Parse argv **after** the program name (same shape as `std::env::args().skip(1)`).
///
/// # Errors
///
/// Returns a human-readable usage string when the command line is invalid.
pub fn parse_args(args: &[String]) -> Result<Command, String> {
    if args.is_empty() {
        return Ok(Command::Help);
    }
    let cmd = args[0].as_str();
    match cmd {
        "help" | "-h" | "--help" => Ok(Command::Help),
        "status" => {
            let json = has_flag(&args[1..], "--json");
            refuse_unknown_flags(&args[1..], &["--json"])?;
            Ok(Command::Status { json })
        }
        "compile" => {
            let rest = &args[1..];
            let model = first_positional(rest).ok_or_else(|| {
                "usage: dragon-npu compile <model> [-O|--opt N] [-q|--quantization QUANT] [--json]"
                    .to_string()
            })?;
            let opt = parse_opt_flag(rest, DEFAULT_OPT_LEVEL)?;
            let quant = parse_string_flag(rest, "-q", DEFAULT_QUANT.to_string())?;
            let quant = parse_string_flag(rest, "--quantization", quant)?;
            let json = has_flag(rest, "--json");
            refuse_unknown_flags(rest, &["-O", "--opt", "-q", "--quantization", "--json"])?;
            Ok(Command::Compile {
                model,
                opt,
                quant,
                json,
            })
        }
        "run" => {
            let rest = &args[1..];
            let model = first_positional(rest).ok_or_else(|| {
                "usage: dragon-npu run <model> [-O|--opt N] [-q|--quantization QUANT] \
                 [--input|-i PATH] [--input-len N] [--json]"
                    .to_string()
            })?;
            let opt = parse_opt_flag(rest, DEFAULT_OPT_LEVEL)?;
            let quant = parse_string_flag(rest, "-q", DEFAULT_QUANT.to_string())?;
            let quant = parse_string_flag(rest, "--quantization", quant)?;
            let input = match parse_optional_string_flag(rest, "--input")? {
                Some(v) => Some(v),
                None => parse_optional_string_flag(rest, "-i")?,
            };
            let input_len = parse_usize_flag(rest, "--input-len", DEFAULT_INPUT_LEN)?;
            if input_len > MAX_INPUT_ELEMENTS {
                return Err(format!(
                    "--input-len {input_len} exceeds max {MAX_INPUT_ELEMENTS} elements"
                ));
            }
            let json = has_flag(rest, "--json");
            refuse_unknown_flags(
                rest,
                &[
                    "-O",
                    "--opt",
                    "-q",
                    "--quantization",
                    "--input",
                    "-i",
                    "--input-len",
                    "--json",
                ],
            )?;
            Ok(Command::Run {
                model,
                opt,
                quant,
                input,
                input_len,
                json,
            })
        }
        "probe" => {
            let json = has_flag(&args[1..], "--json");
            refuse_unknown_flags(&args[1..], &["--json"])?;
            Ok(Command::Probe { json })
        }
        other => Err(format!(
            "unknown command '{other}'. Try: status | compile | run | probe | help"
        )),
    }
}

/// Human-readable help for the binary (printed to stderr by the CLI).
#[must_use]
pub fn help_text() -> String {
    format!(
        "\
dragon-npu — DragonNPU library CLI (host/sim run; optional amd-xdna2 detect/probe)

USAGE:
  dragon-npu status [--json]
  dragon-npu compile <model> [-O|--opt 0..3] [-q|--quantization QUANT] [--json]
  dragon-npu run <model> [-O|--opt 0..3] [-q|--quantization QUANT]
                         [--input|-i PATH] [--input-len N] [--json]
  dragon-npu probe [--json]          # requires --features amd-xdna2
  dragon-npu help

NOTES:
  - compile/run use DragonNpu::new() + ModelCompiler / Backend::run (host path).
  - With --features amd-xdna2, status may report hardware detection (simulated=false)
    while CLI run() still uses the Simulation execution arm (AmdXdna2Backend::new).
    Silicon GEMM / ran_on_npu is never claimed from this CLI.
  - --input-len max is {MAX_INPUT_ELEMENTS} elements (~64 MiB f32 zeros).
  - This CLI never enables EXEC_CMD (no XDNA2_ALLOW_EXEC_CMD; no dangerous flags).
  - Colibrì timed JSON: cargo run --features amd-xdna2 --example npu_direct_bench
  - Safety: docs/XDNA2_NEVER_AGAIN.md, docs/XDNA2_HARDENED_INSTALL.md

EXIT CODES:
  {EXIT_OK} success
  {EXIT_ERROR} error
  {EXIT_USAGE} usage
"
    )
}

fn has_flag(args: &[String], flag: &str) -> bool {
    args.iter().any(|a| a == flag)
}

/// True if `token` looks like a CLI flag (not a bare `-` path, not `--`).
///
/// Values for value-taking flags must not match this, so `-q --json` is usage.
fn looks_like_flag(token: &str) -> bool {
    token.starts_with('-') && token != "-"
}

fn first_positional(args: &[String]) -> Option<String> {
    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        if a.starts_with('-') {
            // flags that take a value
            if matches!(
                a,
                "-O" | "--opt" | "-q" | "--quantization" | "--input" | "-i" | "--input-len"
            ) {
                i += 2;
                continue;
            }
            // bare flags
            i += 1;
            continue;
        }
        return Some(a.to_string());
    }
    None
}

fn parse_opt_flag(args: &[String], default: u8) -> Result<u8, String> {
    if let Some(v) = flag_value(args, "-O")? {
        return parse_opt_level(&v);
    }
    if let Some(v) = flag_value(args, "--opt")? {
        return parse_opt_level(&v);
    }
    Ok(default)
}

fn parse_opt_level(s: &str) -> Result<u8, String> {
    let n: u8 = s
        .parse()
        .map_err(|_| format!("invalid optimization level '{s}' (expected 0..3)"))?;
    if n > 3 {
        return Err(format!("optimization level {n} out of range (0..3)"));
    }
    Ok(n)
}

fn parse_string_flag(args: &[String], flag: &str, default: String) -> Result<String, String> {
    Ok(flag_value(args, flag)?.unwrap_or(default))
}

fn parse_optional_string_flag(args: &[String], flag: &str) -> Result<Option<String>, String> {
    flag_value(args, flag)
}

fn parse_usize_flag(args: &[String], flag: &str, default: usize) -> Result<usize, String> {
    flag_value(args, flag)?.map_or(Ok(default), |v| {
        v.parse()
            .map_err(|_| format!("invalid {flag} value '{v}' (expected usize)"))
    })
}

/// Value for `flag`, or `None` if absent.
///
/// # Errors
///
/// Flag present with missing value, or next token that looks like another flag.
fn flag_value(args: &[String], flag: &str) -> Result<Option<String>, String> {
    let mut i = 0;
    while i < args.len() {
        if args[i] == flag {
            return match args.get(i + 1) {
                None => Err(format!("flag {flag} requires a value")),
                Some(v) if looks_like_flag(v) => {
                    Err(format!("flag {flag} requires a value (got '{v}')"))
                }
                Some(v) => Ok(Some(v.clone())),
            };
        }
        i += 1;
    }
    Ok(None)
}

/// Refuse unknown `--foo` / `-x` tokens so we never advertise unimplemented flags.
fn refuse_unknown_flags(args: &[String], known: &[&str]) -> Result<(), String> {
    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        if a.starts_with('-') {
            if !known.contains(&a) {
                return Err(format!("unknown flag '{a}'. Known: {}", known.join(" ")));
            }
            // value-taking flags consume next token
            if matches!(
                a,
                "-O" | "--opt" | "-q" | "--quantization" | "--input" | "-i" | "--input-len"
            ) {
                if i + 1 >= args.len() {
                    return Err(format!("flag {a} requires a value"));
                }
                let v = args[i + 1].as_str();
                if looks_like_flag(v) {
                    return Err(format!("flag {a} requires a value (got '{v}')"));
                }
                i += 2;
                continue;
            }
            i += 1;
            continue;
        }
        // positional — skip
        i += 1;
    }
    Ok(())
}

fn check_input_len(len: usize, context: &str) -> Result<(), String> {
    if len > MAX_INPUT_ELEMENTS {
        return Err(format!(
            "{context}: {len} elements exceeds max {MAX_INPUT_ELEMENTS}"
        ));
    }
    Ok(())
}

/// Max raw little-endian f32 file size before we refuse a full read for the raw path.
///
/// JSON may still exceed this in text form and is capped by element count after parse.
#[must_use]
pub const fn max_raw_input_bytes() -> usize {
    MAX_INPUT_ELEMENTS.saturating_mul(4)
}

/// True when path extension is `.json` or the first non-whitespace byte is `[`.
fn input_looks_like_json(path: &str, head: &[u8]) -> bool {
    if std::path::Path::new(path)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
    {
        return true;
    }
    head.iter()
        .find(|b| !b.is_ascii_whitespace())
        .is_some_and(|b| *b == b'[')
}

/// Load a flat f32 input buffer from path or generate zeros.
///
/// Supported path formats:
/// - `.json` / content starting with `[`: JSON array of numbers
/// - otherwise: little-endian raw f32 bytes (length must be multiple of 4)
///
/// Zero-fill and loaded lengths are capped at [`MAX_INPUT_ELEMENTS`].
/// Raw (non-JSON) files larger than [`max_raw_input_bytes`] are refused **before**
/// a full `fs::read` (metadata + short peek). JSON still uses the element cap after parse.
///
/// # Errors
///
/// I/O, JSON parse, non-multiple-of-4 raw length, or size over cap.
pub fn load_input_f32(path: Option<&str>, zero_len: usize) -> Result<Vec<f32>, String> {
    let Some(path) = path else {
        check_input_len(zero_len, "--input-len / zero fill")?;
        return Ok(vec![0.0_f32; zero_len]);
    };
    let meta = std::fs::metadata(path).map_err(|e| format!("read input '{path}': {e}"))?;
    let file_len = meta.len();
    let max_raw = max_raw_input_bytes() as u64;

    // Oversize raw refuse before full read; JSON may still be large text.
    let mut prefer_json_hint = std::path::Path::new(path)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("json"));
    if file_len > max_raw && !prefer_json_hint {
        use std::io::Read;
        let mut f = std::fs::File::open(path).map_err(|e| format!("read input '{path}': {e}"))?;
        let mut head = [0_u8; 64];
        let n = f
            .read(&mut head)
            .map_err(|e| format!("read input '{path}': {e}"))?;
        if !input_looks_like_json(path, &head[..n]) {
            return Err(format!(
                "raw f32 input '{path}' is {file_len} bytes > max {max_raw} \
                 ({MAX_INPUT_ELEMENTS} elements × 4); refuse full read"
            ));
        }
        prefer_json_hint = true;
    }

    let bytes = std::fs::read(path).map_err(|e| format!("read input '{path}': {e}"))?;
    // Prefer JSON when extension or leading whitespace + '['
    let as_text = std::str::from_utf8(&bytes).ok();
    let prefer_json = prefer_json_hint || as_text.is_some_and(|s| s.trim_start().starts_with('['));
    if prefer_json {
        let text = as_text.ok_or_else(|| format!("input '{path}' is not valid UTF-8 JSON"))?;
        let vals: Vec<f32> =
            serde_json::from_str(text).map_err(|e| format!("parse input JSON '{path}': {e}"))?;
        check_input_len(vals.len(), &format!("input '{path}'"))?;
        return Ok(vals);
    }
    // Defense: if we got here with oversize raw, refuse (e.g. race grow after peek).
    if (bytes.len() as u64) > max_raw {
        return Err(format!(
            "raw f32 input '{path}' is {} bytes > max {max_raw}",
            bytes.len()
        ));
    }
    if bytes.len() % 4 != 0 {
        return Err(format!(
            "raw f32 input '{path}' length {} is not a multiple of 4",
            bytes.len()
        ));
    }
    let n = bytes.len() / 4;
    check_input_len(n, &format!("raw f32 input '{path}'"))?;
    let mut out = Vec::with_capacity(n);
    for chunk in bytes.chunks_exact(4) {
        let arr: [u8; 4] = [chunk[0], chunk[1], chunk[2], chunk[3]];
        out.push(f32::from_le_bytes(arr));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn s(args: &[&str]) -> Vec<String> {
        args.iter().map(|a| (*a).to_string()).collect()
    }

    #[test]
    fn parse_empty_is_help() {
        assert_eq!(parse_args(&[]).unwrap(), Command::Help);
    }

    #[test]
    fn parse_status_json() {
        assert_eq!(
            parse_args(&s(&["status", "--json"])).unwrap(),
            Command::Status { json: true }
        );
    }

    #[test]
    fn parse_compile_flags() {
        let c = parse_args(&s(&["compile", "m.json", "-O", "3", "-q", "int8"])).unwrap();
        match c {
            Command::Compile {
                model,
                opt,
                quant,
                json,
            } => {
                assert_eq!(model, "m.json");
                assert_eq!(opt, 3);
                assert_eq!(quant, "int8");
                assert!(!json);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn parse_run_input_len() {
        let c = parse_args(&s(&["run", "m.json", "--input-len", "64", "--json"])).unwrap();
        match c {
            Command::Run {
                input,
                input_len,
                json,
                ..
            } => {
                assert!(input.is_none());
                assert_eq!(input_len, 64);
                assert!(json);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn parse_unknown_flag_errors() {
        let err = parse_args(&s(&["status", "--allow-exec-cmd"])).unwrap_err();
        assert!(err.contains("unknown flag"), "{err}");
    }

    #[test]
    fn parse_unknown_command() {
        let err = parse_args(&s(&["bench-json"])).unwrap_err();
        assert!(err.contains("unknown command"), "{err}");
    }

    #[test]
    fn parse_opt_out_of_range() {
        let err = parse_args(&s(&["compile", "m.json", "-O", "9"])).unwrap_err();
        assert!(err.contains("out of range"), "{err}");
    }

    #[test]
    fn parse_q_followed_by_flag_is_usage() {
        let err = parse_args(&s(&["compile", "m.json", "-q", "--json"])).unwrap_err();
        assert!(
            err.contains("requires a value") && err.contains("--json"),
            "{err}"
        );
    }

    #[test]
    fn parse_input_followed_by_flag_is_usage() {
        let err = parse_args(&s(&["run", "m.json", "--input", "--json"])).unwrap_err();
        assert!(
            err.contains("requires a value") && err.contains("--json"),
            "{err}"
        );
    }

    #[test]
    fn parse_i_followed_by_input_len_is_usage() {
        let err = parse_args(&s(&["run", "m.json", "-i", "--input-len", "4"])).unwrap_err();
        assert!(
            err.contains("requires a value") && err.contains("--input-len"),
            "{err}"
        );
    }

    #[test]
    fn parse_o_followed_by_flag_is_usage() {
        let err = parse_args(&s(&["compile", "m.json", "-O", "--json"])).unwrap_err();
        assert!(
            err.contains("requires a value") && err.contains("--json"),
            "{err}"
        );
    }

    #[test]
    fn parse_input_len_over_max_is_usage() {
        let too_big = (MAX_INPUT_ELEMENTS + 1).to_string();
        let err = parse_args(&s(&["run", "m.json", "--input-len", &too_big])).unwrap_err();
        assert!(err.contains("exceeds max"), "{err}");
    }

    #[test]
    fn load_input_zeros() {
        let v = load_input_f32(None, 4).unwrap();
        assert_eq!(v, vec![0.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn load_input_zeros_over_max_errors() {
        let err = load_input_f32(None, MAX_INPUT_ELEMENTS + 1).unwrap_err();
        assert!(err.contains("exceeds max"), "{err}");
    }

    #[test]
    fn load_input_json_array() {
        let dir = std::env::temp_dir();
        let path = dir.join("dragon_npu_cli_test_input.json");
        {
            let mut f = std::fs::File::create(&path).unwrap();
            write!(f, "[1.0, 2.5, -3.0]").unwrap();
        }
        let v = load_input_f32(Some(path.to_str().unwrap()), 0).unwrap();
        assert_eq!(v, vec![1.0, 2.5, -3.0]);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn load_input_raw_le_f32() {
        let dir = std::env::temp_dir();
        let path = dir.join("dragon_npu_cli_test_input.bin");
        {
            let mut f = std::fs::File::create(&path).unwrap();
            f.write_all(&1.0_f32.to_le_bytes()).unwrap();
            f.write_all(&2.0_f32.to_le_bytes()).unwrap();
        }
        let v = load_input_f32(Some(path.to_str().unwrap()), 0).unwrap();
        assert_eq!(v, vec![1.0, 2.0]);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn load_input_raw_bad_len_errors() {
        let dir = std::env::temp_dir();
        let path = dir.join("dragon_npu_cli_test_input_bad.bin");
        {
            let mut f = std::fs::File::create(&path).unwrap();
            f.write_all(&[1, 2, 3]).unwrap();
        }
        let err = load_input_f32(Some(path.to_str().unwrap()), 0).unwrap_err();
        assert!(err.contains("not a multiple of 4"), "{err}");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn load_input_raw_oversize_refuses_before_full_read() {
        assert_eq!(max_raw_input_bytes(), MAX_INPUT_ELEMENTS.saturating_mul(4));
        let dir = std::env::temp_dir();
        let path = dir.join("dragon_npu_cli_test_input_oversize_sparse.bin");
        {
            // Sparse file: metadata length > max_raw without allocating multi-GB.
            let f = std::fs::File::create(&path).unwrap();
            let over = (max_raw_input_bytes() as u64).saturating_add(4);
            f.set_len(over).unwrap();
        }
        let err = load_input_f32(Some(path.to_str().unwrap()), 0).unwrap_err();
        let _ = std::fs::remove_file(&path);
        assert!(
            err.contains("refuse full read") || err.contains("> max"),
            "err={err}"
        );
    }

    #[test]
    fn load_input_missing_file_errors() {
        let err = load_input_f32(Some("/no/such/dragon_npu_input_xyz.bin"), 0).unwrap_err();
        assert!(err.contains("read input"), "{err}");
    }

    #[test]
    fn help_mentions_safety_and_no_exec() {
        let h = help_text();
        assert!(h.contains("EXEC_CMD"));
        assert!(h.contains("NEVER_AGAIN"));
        assert!(h.contains("npu_direct_bench"));
        assert!(h.contains("--opt"));
        assert!(h.contains("--quantization"));
        assert!(h.contains("-i"));
        assert!(h.contains("Simulation execution arm") || h.contains("host path"));
    }

    #[test]
    fn schema_version_matches_crate_root() {
        // Default-feature: feature-disabled JSON emitters must share this constant.
        assert_eq!(crate::TIMED_OP_JSON_SCHEMA_VERSION, 1);
    }
}
