//! Thin argv helpers for the `earth-dragon-npu` binary (library-testable).
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

/// Parsed top-level subcommand for `earth-dragon-npu`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Print NPU / backend status.
    Status {
        /// Emit a single JSON object instead of human text.
        json: bool,
        /// Optional absolute external-runner program (`--external-runner`).
        external_runner: Option<String>,
        /// Optional child working directory.
        external_cwd: Option<String>,
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
        /// Optional absolute external-runner program (status of wrap only).
        external_runner: Option<String>,
        /// Optional child working directory.
        external_cwd: Option<String>,
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
        /// Optional absolute external-runner program (opt-in wrap).
        external_runner: Option<String>,
        /// Extra argv for the child (`--external-arg`, repeatable).
        external_args: Vec<String>,
        /// Optional absolute working directory for the child.
        external_cwd: Option<String>,
        /// Named IRON catalog kernel (`--iron NAME`). Mutually exclusive with
        /// `--host-graph` and raw `--external-runner`.
        iron_kernel: Option<String>,
        /// Opt-in export-only collect (`--iron-export`). Creates a this-run
        /// `--aot-dir` for `vector_add` only. Not the default run wrap.
        /// Child `PASS!` is not expected on this path.
        iron_export: bool,
        /// Host linear graph (`--host-graph`). Host math only, not NPU.
        host_graph: bool,
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
/// Parse placeholder when `--iron` has no positional model. `--iron` does **not**
/// compile this path (see [`run_iron_without_host_model`]).
pub const DEFAULT_IRON_MODEL: &str = "tests/fixtures/host_matmul_relu.json";
/// Default model when `--host-graph` is used without a positional path.
pub const DEFAULT_HOST_GRAPH_MODEL: &str = "tests/fixtures/host_matmul_relu_add.json";
/// Default host-graph activation when `--input` is omitted (`[-1, 2]`).
pub const DEFAULT_HOST_GRAPH_INPUT: [f32; 2] = [-1.0, 2.0];
/// Hard-coded host-graph honesty line. Not product Pass.
pub const HOST_GRAPH_HONESTY_TOKENS: &str = "execution=host ran_on_npu=false invent_pass=0";

/// CLI flag wins; else env `DRAGON_NPU_EXTERNAL_RUNNER` (opt-in; default off).
#[must_use]
pub fn resolve_external_runner_program(flag: Option<&str>) -> Option<String> {
    if let Some(p) = flag {
        let t = p.trim();
        if !t.is_empty() {
            return Some(t.to_string());
        }
    }
    std::env::var(crate::backends::external_runner::ENV_EXTERNAL_RUNNER)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// CLI flag wins; else env `DRAGON_NPU_EXTERNAL_CWD`.
#[must_use]
pub fn resolve_external_cwd(flag: Option<&str>) -> Option<String> {
    if let Some(p) = flag {
        let t = p.trim();
        if !t.is_empty() {
            return Some(t.to_string());
        }
    }
    std::env::var(crate::backends::external_runner::ENV_EXTERNAL_CWD)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

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
        "status" => parse_status_args(&args[1..]),
        "compile" => parse_compile_args(&args[1..]),
        "run" => parse_run_args(&args[1..]),
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

fn parse_status_args(rest: &[String]) -> Result<Command, String> {
    let json = has_flag(rest, "--json");
    let external_runner = parse_optional_string_flag(rest, "--external-runner")?;
    let external_cwd = parse_optional_string_flag(rest, "--external-cwd")?;
    refuse_unknown_flags(rest, &["--json", "--external-runner", "--external-cwd"])?;
    Ok(Command::Status {
        json,
        external_runner,
        external_cwd,
    })
}

fn parse_compile_args(rest: &[String]) -> Result<Command, String> {
    let model = first_positional(rest).ok_or_else(|| {
        "usage: earth-dragon-npu compile <model> [-O|--opt N] [-q|--quantization QUANT] [--json] \
         [--external-runner PATH] [--external-cwd DIR]"
            .to_string()
    })?;
    let opt = parse_opt_flag(rest, DEFAULT_OPT_LEVEL)?;
    let quant = parse_string_flag(rest, "-q", DEFAULT_QUANT.to_string())?;
    let quant = parse_string_flag(rest, "--quantization", quant)?;
    let json = has_flag(rest, "--json");
    let external_runner = parse_optional_string_flag(rest, "--external-runner")?;
    let external_cwd = parse_optional_string_flag(rest, "--external-cwd")?;
    refuse_unknown_flags(
        rest,
        &[
            "-O",
            "--opt",
            "-q",
            "--quantization",
            "--json",
            "--external-runner",
            "--external-cwd",
        ],
    )?;
    Ok(Command::Compile {
        model,
        opt,
        quant,
        json,
        external_runner,
        external_cwd,
    })
}

fn parse_run_args(rest: &[String]) -> Result<Command, String> {
    let iron_kernel = parse_optional_string_flag(rest, "--iron")?;
    if let Some(name) = iron_kernel.as_deref() {
        crate::backends::iron_kernel::iron_kernel_spec(name)?;
    }
    let host_graph = has_flag(rest, "--host-graph");
    if iron_kernel.is_some() && host_graph {
        return Err(
            "do not mix --iron and --host-graph (named IRON wrap vs host linear)".to_string(),
        );
    }
    let model = match first_positional(rest) {
        Some(m) => m,
        None if host_graph => DEFAULT_HOST_GRAPH_MODEL.to_string(),
        None if iron_kernel.is_some() => DEFAULT_IRON_MODEL.to_string(),
        None => {
            return Err(
                "usage: earth-dragon-npu run <model> [-O|--opt N] [-q|--quantization QUANT] \
                 [--input|-i PATH] [--input-len N] [--json] \
                 [--iron NAME] [--host-graph] \
                 [--external-runner PATH] [--external-arg ARG]... [--external-cwd DIR]"
                    .to_string(),
            );
        }
    };
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
    let iron_export = has_flag(rest, "--iron-export");
    let external_runner = parse_optional_string_flag(rest, "--external-runner")?;
    let external_args = collect_repeatable_flag(rest, "--external-arg")?;
    let external_cwd = parse_optional_string_flag(rest, "--external-cwd")?;
    if iron_export && iron_kernel.is_none() {
        return Err(
            "--iron-export requires --iron NAME (export-only collect; no child PASS! expected)"
                .to_string(),
        );
    }
    if iron_kernel.is_some() && external_runner.is_some() {
        return Err(
            "do not mix --iron and --external-runner (use --iron NAME or raw runner, not both)"
                .to_string(),
        );
    }
    if host_graph && external_runner.is_some() {
        return Err("do not mix --host-graph and --external-runner".to_string());
    }
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
            "--iron",
            "--iron-export",
            "--host-graph",
            "--external-runner",
            "--external-arg",
            "--external-cwd",
        ],
    )?;
    Ok(Command::Run {
        model,
        opt,
        quant,
        input,
        input_len,
        json,
        external_runner,
        external_args,
        external_cwd,
        iron_kernel,
        iron_export,
        host_graph,
    })
}

/// Human-readable help for the binary (printed to stderr by the CLI).
#[must_use]
pub fn help_text() -> String {
    format!(
        "\
earth-dragon-npu — Earth Dragon NPU library CLI (host/sim run; optional amd-xdna2 detect/probe)

USAGE:
  just npu-relu                      # named IRON ReLU (also npu-iron / npu-gelu / npu-silu / npu-mul / npu-swiglu / npu-scale-shift)
  just npu-iron                      # run+verify wrap for vector_add (no --aot-dir)
  just npu-iron-export               # export-only collect (writes artifacts; no child PASS! expected)
  earth-dragon-npu run --iron relu         # catalog: vector_add eltwise relu softmax gelu silu eltwise_mul swiglu scale_shift
  earth-dragon-npu run --host-graph        # host linear on tests/fixtures/host_matmul_relu_add.json
  earth-dragon-npu status [--json] [--external-runner PATH] [--external-cwd DIR]
  earth-dragon-npu compile <model> [-O|--opt 0..3] [-q|--quantization QUANT] [--json]
                             [--external-runner PATH] [--external-cwd DIR]
  earth-dragon-npu run <model> [-O|--opt 0..3] [-q|--quantization QUANT]
                         [--input|-i PATH] [--input-len N] [--json]
                         [--iron NAME] [--iron-export] [--host-graph]
                         [--external-runner PATH] [--external-arg ARG]...
                         [--external-cwd DIR]
  earth-dragon-npu probe [--json]          # requires --features amd-xdna2
  earth-dragon-npu help

NOTES:
  - compile/run use DragonNpu::new() + ModelCompiler / Backend::run (host path)
    unless --iron, --external-runner, or env DRAGON_NPU_EXTERNAL_RUNNER is set.
  - --iron NAME is the named IRON catalog (vector_add, eltwise, relu, softmax,
    gelu, silu, eltwise_mul, swiglu, scale_shift). Resolves mlir-aie without
    $MLIR_AIE. Prefer `just npu-relu` / `just npu-iron` (and siblings).
    Does not compile a dummy host model. Child PASS! is not product ran_on_npu.
    invent_pass=0. Default vector_add is the run+verify wrap (no --aot-dir).
    --iron-export / `just npu-iron-export` is export-only: writes this-run
    artifacts then returns. No child PASS! expected on the export path.
  - --host-graph runs evaluate_host_graph on the fixture (host math, not NPU).
    Default model is tests/fixtures/host_matmul_relu_add.json; default input [-1, 2].
    Requires --features amd-xdna2. execution=host ran_on_npu=false invent_pass=0.
  - --external-runner PATH is the raw opt-in wrap. Absolute program only.
    Repeat --external-arg for argv. Never sh -c. JSON/text print
    execution=external_runner and ran_on_npu=false even if the child prints PASS!.
  - Do not mix --iron, --host-graph, and --external-runner.
  - With --features amd-xdna2, status may report hardware detection (simulated=false)
    while default CLI run() still uses the Simulation execution arm (AmdXdna2Backend::new).
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
                "-O" | "--opt"
                    | "-q"
                    | "--quantization"
                    | "--input"
                    | "-i"
                    | "--input-len"
                    | "--iron"
                    | "--external-runner"
                    | "--external-arg"
                    | "--external-cwd"
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

/// All values for a repeatable flag (e.g. `--external-arg`).
///
/// The next token is taken literally, including child flags such as `--dev`.
fn collect_repeatable_flag(args: &[String], flag: &str) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if args[i] == flag {
            match args.get(i + 1) {
                None => return Err(format!("flag {flag} requires a value")),
                Some(v) => out.push(v.clone()),
            }
            i += 2;
            continue;
        }
        i += 1;
    }
    Ok(out)
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
                "-O" | "--opt"
                    | "-q"
                    | "--quantization"
                    | "--input"
                    | "-i"
                    | "--input-len"
                    | "--iron"
                    | "--external-runner"
                    | "--external-arg"
                    | "--external-cwd"
            ) {
                if i + 1 >= args.len() {
                    return Err(format!("flag {a} requires a value"));
                }
                let v = args[i + 1].as_str();
                // Child argv may be `--dev`; only `--external-arg` accepts a flag-like value.
                if looks_like_flag(v) && a != "--external-arg" {
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
    for chunk in bytes.as_chunks::<4>().0 {
        out.push(f32::from_le_bytes(*chunk));
    }
    Ok(out)
}

/// Hard-coded host-graph honesty tokens. Not product Pass.
#[must_use]
pub fn format_host_graph_honesty_tokens() -> &'static str {
    HOST_GRAPH_HONESTY_TOKENS
}

/// Resolve a CLI model path: absolute file, else cwd, else crate root.
///
/// Used so `run --host-graph` finds `tests/fixtures/...` when cwd is not
/// the checkout. `--iron` does not compile a host model.
///
/// # Errors
///
/// File missing under cwd and crate root.
pub fn resolve_cli_model_path_from(
    model: &str,
    cwd: &std::path::Path,
    crate_root: &std::path::Path,
) -> Result<String, String> {
    let p = std::path::Path::new(model);
    if p.is_absolute() {
        if p.is_file() {
            return Ok(model.to_string());
        }
        return Err(format!("model not found: {model}"));
    }
    let from_cwd = cwd.join(p);
    if from_cwd.is_file() {
        return Ok(from_cwd.to_string_lossy().into_owned());
    }
    let from_root = crate_root.join(p);
    if from_root.is_file() {
        return Ok(from_root.to_string_lossy().into_owned());
    }
    Err(format!("model '{model}' not found under cwd or crate root"))
}

/// Resolve `model` from process cwd, then this crate's manifest dir.
///
/// # Errors
///
/// File missing, or cwd unreadable.
pub fn resolve_cli_model_path(model: &str) -> Result<String, String> {
    let cwd = std::env::current_dir().map_err(|e| format!("current_dir: {e}"))?;
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    resolve_cli_model_path_from(model, &cwd, crate_root)
}

/// `--iron` child without compiling a dummy host model. Not product Pass.
#[derive(Debug, Clone)]
pub struct IronChildOutcome {
    /// [`EXIT_OK`] when the child exit is 0.
    pub exit: i32,
    /// Always [`crate::backends::external_runner::RAN_ON_NPU`] (`false`).
    pub ran_on_npu: bool,
    /// Always [`crate::backends::external_runner::INVENT_PASS`] (`0`).
    pub invent_pass: u8,
    /// Informational: child stdout mentioned `PASS!`.
    pub child_said_pass: bool,
    /// Parsed numbers (empty when the child printed none). Never invented.
    pub parsed: Vec<f32>,
    /// Always false: this path never calls `compile_model`.
    pub compiled_host_model: bool,
    /// Child report when spawn succeeded.
    pub report: Option<crate::backends::external_runner::ExternalRunnerReport>,
    /// Spawn / runner error text when [`Self::report`] is `None`.
    pub error: Option<String>,
}

/// Spawn the IRON wrap. Does **not** compile `tests/fixtures/host_matmul_relu.json`.
///
/// Honesty fields are constants. Child `PASS!` is not `ran_on_npu`.
#[must_use]
pub fn run_iron_without_host_model(
    backend: &crate::backends::external_runner::ExternalRunnerBackend,
) -> IronChildOutcome {
    use crate::backends::external_runner::{INVENT_PASS, RAN_ON_NPU};
    match backend.run_external() {
        Ok(report) => {
            let parsed = backend.parsed_output(&report);
            let ok = report.child_exit_ok();
            IronChildOutcome {
                exit: if ok { EXIT_OK } else { EXIT_ERROR },
                ran_on_npu: RAN_ON_NPU,
                invent_pass: INVENT_PASS,
                child_said_pass: report.child_said_pass,
                parsed,
                compiled_host_model: false,
                report: Some(report),
                error: None,
            }
        }
        Err(e) => IronChildOutcome {
            exit: EXIT_ERROR,
            ran_on_npu: RAN_ON_NPU,
            invent_pass: INVENT_PASS,
            child_said_pass: false,
            parsed: Vec::new(),
            compiled_host_model: false,
            report: None,
            error: Some(e.to_string()),
        },
    }
}

/// Host linear: compile `model_path` then [`crate::backends::amd_xdna2::evaluate_host_graph`].
///
/// Not product Pass. Label execution as host. Requires `amd-xdna2`.
/// Resolves a cwd-relative default fixture from the crate root.
///
/// # Errors
///
/// Feature missing, compile fail, or host-graph refuse.
#[cfg(feature = "amd-xdna2")]
pub fn evaluate_cli_host_graph(model_path: &str, input: &[f32]) -> Result<Vec<f32>, String> {
    let resolved = resolve_cli_model_path(model_path)?;
    let npu = crate::DragonNpu::new().map_err(|e| e.to_string())?;
    let compiled = npu
        .compile_model(&resolved, DEFAULT_OPT_LEVEL, DEFAULT_QUANT)
        .map_err(|e| e.to_string())?;
    crate::backends::amd_xdna2::evaluate_host_graph(&compiled, input).map_err(|e| e.to_string())
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
            Command::Status {
                json: true,
                external_runner: None,
                external_cwd: None,
            }
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
                external_runner,
                external_cwd,
            } => {
                assert_eq!(model, "m.json");
                assert_eq!(opt, 3);
                assert_eq!(quant, "int8");
                assert!(!json);
                assert!(external_runner.is_none());
                assert!(external_cwd.is_none());
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
        assert!(h.contains("--external-runner"));
        assert!(h.contains("just npu-iron"));
        assert!(h.contains("execution=external_runner"));
        assert!(h.contains("ran_on_npu=false"));
    }

    #[test]
    fn parse_run_external_runner_args() {
        let c = parse_args(&s(&[
            "run",
            "m.json",
            "--external-runner",
            "/usr/bin/python3",
            "--external-arg",
            "vector_scalar_add.py",
            "--external-arg",
            "--dev",
            "--external-arg",
            "npu2",
            "--external-cwd",
            "/tmp",
            "--json",
        ]))
        .unwrap();
        match c {
            Command::Run {
                external_runner,
                external_args,
                external_cwd,
                json,
                ..
            } => {
                assert_eq!(external_runner.as_deref(), Some("/usr/bin/python3"));
                assert_eq!(
                    external_args,
                    vec![
                        "vector_scalar_add.py".to_string(),
                        "--dev".to_string(),
                        "npu2".to_string()
                    ]
                );
                assert_eq!(external_cwd.as_deref(), Some("/tmp"));
                assert!(json);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn parse_external_runner_missing_value_is_usage() {
        let err = parse_args(&s(&["run", "m.json", "--external-runner"])).unwrap_err();
        assert!(err.contains("requires a value"), "{err}");
    }

    #[test]
    fn schema_version_matches_crate_root() {
        // Default-feature: feature-disabled JSON emitters must share this constant.
        assert_eq!(crate::TIMED_OP_JSON_SCHEMA_VERSION, 1);
    }

    #[test]
    fn parse_run_iron_relu_and_siblings() {
        for (name, suffix, argv_head) in [
            (
                "vector_add",
                "programming_examples/basic/vector_scalar_add",
                "vector_scalar_add.py",
            ),
            ("eltwise", "programming_examples/ml/eltwise", "eltwise.py"),
            (
                "relu",
                "programming_examples/ml/eltwise_unary",
                "eltwise_unary.py",
            ),
            ("softmax", "programming_examples/ml/softmax", "softmax.py"),
            (
                "gelu",
                "programming_examples/ml/eltwise_unary",
                "eltwise_unary.py",
            ),
            (
                "silu",
                "programming_examples/ml/eltwise_unary",
                "eltwise_unary.py",
            ),
            (
                "eltwise_mul",
                "programming_examples/ml/eltwise",
                "eltwise.py",
            ),
            ("swiglu", "programming_examples/ml/swiglu", "swiglu.py"),
            (
                "scale_shift",
                "programming_examples/ml/scale_shift",
                "scale_shift.py",
            ),
        ] {
            let c = parse_args(&s(&["run", "m.json", "--iron", name]))
                .unwrap_or_else(|e| panic!("--iron {name}: {e}"));
            match c {
                Command::Run {
                    iron_kernel,
                    host_graph,
                    external_runner,
                    ..
                } => {
                    assert_eq!(iron_kernel.as_deref(), Some(name));
                    assert!(!host_graph);
                    assert!(external_runner.is_none());
                    let spec =
                        crate::backends::iron_kernel::iron_kernel_spec(name).expect("catalog name");
                    assert_eq!(spec.cwd_suffix, suffix);
                    assert_eq!(spec.argv[0], argv_head);
                }
                other => panic!("unexpected {other:?}"),
            }
        }
    }

    #[test]
    fn parse_run_iron_export_is_opt_in_not_default() {
        let default = parse_args(&s(&["run", "--iron", "vector_add"])).expect("default wrap");
        match default {
            Command::Run {
                iron_kernel,
                iron_export,
                ..
            } => {
                assert_eq!(iron_kernel.as_deref(), Some("vector_add"));
                assert!(!iron_export, "default --iron is the run wrap, not export");
            }
            other => panic!("unexpected {other:?}"),
        }
        let export = parse_args(&s(&["run", "--iron", "vector_add", "--iron-export"]))
            .expect("opt-in export");
        match export {
            Command::Run {
                iron_kernel,
                iron_export,
                ..
            } => {
                assert_eq!(iron_kernel.as_deref(), Some("vector_add"));
                assert!(iron_export, "--iron-export must set the export flag");
            }
            other => panic!("unexpected {other:?}"),
        }
        let err = parse_args(&s(&["run", "m.json", "--iron-export"]))
            .expect_err("--iron-export without --iron must refuse");
        assert!(
            err.contains("--iron-export") && err.contains("--iron"),
            "export without --iron must mention both, got: {err}"
        );
    }

    #[test]
    fn parse_run_iron_unknown_fails_closed() {
        let err = parse_args(&s(&["run", "m.json", "--iron", "not-a-kernel"]))
            .expect_err("unknown --iron name must refuse");
        assert!(
            err.contains("unknown") && err.contains("not-a-kernel"),
            "expected unknown-name refuse, got: {err}"
        );
        assert!(
            err.contains("vector_add") && err.contains("relu"),
            "error should list known names, got: {err}"
        );
    }

    #[test]
    fn parse_run_host_graph_flag() {
        let c = parse_args(&s(&[
            "run",
            "tests/fixtures/host_matmul_relu_add.json",
            "--host-graph",
        ]))
        .expect("--host-graph");
        match c {
            Command::Run {
                host_graph,
                iron_kernel,
                model,
                ..
            } => {
                assert!(host_graph);
                assert!(iron_kernel.is_none());
                assert_eq!(model, "tests/fixtures/host_matmul_relu_add.json");
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn parse_run_iron_and_host_graph_are_exclusive() {
        let err = parse_args(&s(&["run", "m.json", "--iron", "relu", "--host-graph"]))
            .expect_err("must refuse mixing --iron and --host-graph");
        assert!(
            err.contains("--iron") && err.contains("--host-graph"),
            "got: {err}"
        );
    }

    #[test]
    fn parse_run_iron_and_external_runner_are_exclusive() {
        let err = parse_args(&s(&[
            "run",
            "m.json",
            "--iron",
            "relu",
            "--external-runner",
            "/usr/bin/true",
        ]))
        .expect_err("must refuse mixing --iron and --external-runner");
        assert!(
            err.contains("--iron") && err.contains("--external-runner"),
            "got: {err}"
        );
    }

    #[test]
    fn parse_run_host_graph_and_external_runner_are_exclusive() {
        let err = parse_args(&s(&[
            "run",
            "m.json",
            "--host-graph",
            "--external-runner",
            "/usr/bin/true",
        ]))
        .expect_err("must refuse mixing --host-graph and --external-runner");
        assert!(
            err.contains("--host-graph") && err.contains("--external-runner"),
            "got: {err}"
        );
    }

    #[test]
    fn parse_run_iron_without_model_uses_default() {
        let c = parse_args(&s(&["run", "--iron", "relu"])).expect("run --iron relu");
        match c {
            Command::Run {
                model,
                iron_kernel,
                host_graph,
                ..
            } => {
                assert_eq!(iron_kernel.as_deref(), Some("relu"));
                assert!(!host_graph);
                assert_eq!(model, DEFAULT_IRON_MODEL);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn parse_run_host_graph_without_model_uses_default() {
        let c = parse_args(&s(&["run", "--host-graph"])).expect("run --host-graph");
        match c {
            Command::Run {
                model,
                host_graph,
                iron_kernel,
                ..
            } => {
                assert!(host_graph);
                assert!(iron_kernel.is_none());
                assert_eq!(model, DEFAULT_HOST_GRAPH_MODEL);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn parse_run_bare_is_usage() {
        let err = parse_args(&s(&["run"])).expect_err("bare run must be usage");
        assert!(
            err.contains("usage"),
            "bare run must mention usage, got: {err}"
        );
    }

    #[test]
    fn help_lists_iron_names_and_host_graph() {
        let h = help_text();
        assert!(h.contains("--iron"), "help must list --iron");
        assert!(h.contains("vector_add"), "help must list vector_add");
        assert!(h.contains("eltwise"), "help must list eltwise");
        assert!(h.contains("relu"), "help must list relu");
        assert!(h.contains("softmax"), "help must list softmax");
        assert!(h.contains("gelu"), "help must list gelu");
        assert!(h.contains("silu"), "help must list silu");
        assert!(h.contains("eltwise_mul"), "help must list eltwise_mul");
        assert!(h.contains("swiglu"), "help must list swiglu");
        assert!(h.contains("scale_shift"), "help must list scale_shift");
        assert!(h.contains("--host-graph"), "help must list --host-graph");
        assert!(
            h.contains("just npu-relu"),
            "help must lead with just npu-relu"
        );
        assert!(h.contains("npu-swiglu"), "help lead must list npu-swiglu");
        assert!(
            h.contains("npu-scale-shift"),
            "help lead must list npu-scale-shift"
        );
        assert!(
            h.contains("--iron-export") || h.contains("npu-iron-export"),
            "help must name the opt-in export path"
        );
        assert!(
            h.contains("invent_pass=0"),
            "help must hard-code invent_pass=0"
        );
        assert!(
            h.contains("ran_on_npu=false"),
            "help must hard-code ran_on_npu=false"
        );
        assert!(
            h.contains("execution=host"),
            "help must label host-graph execution=host"
        );
    }

    #[test]
    fn iron_run_skips_missing_dummy_model() {
        use crate::backends::external_runner::{
            ChildOutput, CommandRunner, ExternalRunnerBackend, ExternalRunnerConfig, INVENT_PASS,
            RAN_ON_NPU, SpawnRequest,
        };
        use std::sync::{Arc, Mutex};

        #[derive(Debug)]
        struct Scripted {
            last: Mutex<Option<SpawnRequest>>,
        }
        impl CommandRunner for Scripted {
            fn run(&self, request: &SpawnRequest) -> Result<ChildOutput, crate::DragonNpuError> {
                *self.last.lock().expect("lock") = Some(request.clone());
                Ok(ChildOutput {
                    status_code: Some(0),
                    stdout: "PASS!\n".into(),
                    stderr: String::new(),
                })
            }
        }

        let tmp = std::env::temp_dir().join(format!(
            "dragon_npu_iron_skip_compile_{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&tmp).expect("temp cwd");
        let dummy = tmp.join(DEFAULT_IRON_MODEL);
        assert!(
            !dummy.is_file(),
            "temp cwd must not contain the dummy fixture"
        );
        let npu = crate::DragonNpu::new().expect("npu");
        npu.compile_model(
            dummy.to_str().expect("utf8"),
            DEFAULT_OPT_LEVEL,
            DEFAULT_QUANT,
        )
        .expect_err("missing dummy must fail compile");

        let program = if std::path::Path::new("/usr/bin/true").is_file() {
            "/usr/bin/true"
        } else {
            "/bin/true"
        };
        let cfg =
            ExternalRunnerConfig::new(program, vec![], Some(tmp.to_string_lossy().into_owned()))
                .expect("temp cwd is a directory");
        let backend = ExternalRunnerBackend::with_runner(
            cfg,
            Arc::new(Scripted {
                last: Mutex::new(None),
            }),
        );
        let out = run_iron_without_host_model(&backend);
        let _ = std::fs::remove_dir_all(&tmp);
        assert_eq!(out.exit, EXIT_OK);
        assert!(!out.ran_on_npu);
        assert_eq!(out.ran_on_npu, RAN_ON_NPU);
        assert_eq!(out.invent_pass, INVENT_PASS);
        assert_eq!(out.invent_pass, 0);
        assert!(out.child_said_pass);
        assert!(!out.compiled_host_model);
        assert_eq!(out.parsed, [] as [f32; 0]);
        assert!(out.report.is_some());
        assert!(out.error.is_none());
    }

    #[cfg(feature = "amd-xdna2")]
    #[test]
    fn host_graph_fixture_returns_documented_pair() {
        let path = format!(
            "{}/tests/fixtures/host_matmul_relu_add.json",
            env!("CARGO_MANIFEST_DIR")
        );
        let out =
            evaluate_cli_host_graph(&path, &DEFAULT_HOST_GRAPH_INPUT).expect("host graph fixture");
        assert_eq!(
            out,
            vec![0.5, 3.5],
            "documented host linear: [-1, 2] -> [0.5, 3.5]"
        );
        assert_eq!(
            format_host_graph_honesty_tokens(),
            "execution=host ran_on_npu=false invent_pass=0"
        );
    }

    #[cfg(feature = "amd-xdna2")]
    #[test]
    fn host_graph_default_model_resolves_from_crate_root() {
        let tmp =
            std::env::temp_dir().join(format!("dragon_npu_host_graph_cwd_{}", std::process::id()));
        std::fs::create_dir_all(&tmp).expect("temp cwd");
        assert!(
            !tmp.join(DEFAULT_HOST_GRAPH_MODEL).is_file(),
            "temp cwd must not contain the fixture"
        );
        let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let resolved = resolve_cli_model_path_from(DEFAULT_HOST_GRAPH_MODEL, &tmp, crate_root)
            .expect("crate-root fallback");
        let _ = std::fs::remove_dir_all(&tmp);
        let out = evaluate_cli_host_graph(&resolved, &DEFAULT_HOST_GRAPH_INPUT)
            .expect("host graph after crate-root resolve");
        assert_eq!(
            out,
            vec![0.5, 3.5],
            "no-positional --host-graph must still return [0.5, 3.5] from crate root"
        );
    }
}
