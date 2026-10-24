//! `earth-dragon-npu` — thin CLI over the Earth Dragon NPU library.
//!
//! Host/sim `compile` + `run` work without hardware. Optional `probe` needs
//! `--features amd-xdna2`. This binary **never** enables EXEC_CMD or advertises
//! unsafe enable flags.
//!
//! ```bash
//! cargo run --bin earth-dragon-npu -- status
//! cargo run --bin earth-dragon-npu -- compile tests/fixtures/host_matmul_relu.json -O 2
//! cargo run --bin earth-dragon-npu -- run tests/fixtures/host_matmul_relu.json --input-len 4
//! cargo run --features amd-xdna2 --bin earth-dragon-npu -- probe
//! ```
//!
//! Colibrì timed JSON remains `examples/npu_direct_bench` (see `docs/COLIBRI_ADAPTER.md`).

use std::sync::Arc;

use earth_dragon_npu::backends::external_runner::{
    EXECUTION_EXTERNAL_RUNNER, ExternalRunnerBackend, ExternalRunnerConfig, ExternalRunnerReport,
    INVENT_PASS, PRODUCT_KNOWN_GOOD, RAN_ON_NPU,
};

/// Opt-in wrap flags shared by status / compile / run.
#[derive(Clone, Copy)]
struct ExternalCliOpts<'a> {
    runner_flag: Option<&'a str>,
    extra_args: &'a [String],
    cwd_flag: Option<&'a str>,
}

/// `run` dispatch: model, input, and named catalog flags.
#[derive(Clone, Copy)]
struct RunCli<'a> {
    model: &'a str,
    opt: u8,
    quant: &'a str,
    input_path: Option<&'a str>,
    input_len: usize,
    json: bool,
    ext: ExternalCliOpts<'a>,
    iron_kernel: Option<&'a str>,
    iron_export: bool,
    host_graph: bool,
}
use earth_dragon_npu::backends::iron_kernel::{
    IRON_NPU2_VECTOR_ADD_JOB, create_this_run_aot_dir, format_iron_job_collect_note,
    iron_kernel_cwd_and_argv_with_aot, iron_kernel_should_create_this_run_aot_dir,
    list_this_run_aot_artifacts, load_this_run_npy_from_aot_dir, parse_child_f32_vector,
    resolve_iron_python, resolve_mlir_aie_root,
};
#[cfg(feature = "amd-xdna2")]
use earth_dragon_npu::cli::DEFAULT_HOST_GRAPH_INPUT;
use earth_dragon_npu::cli::{
    self, Command, EXIT_ERROR, EXIT_OK, EXIT_USAGE, help_text, load_input_f32,
    resolve_external_cwd, resolve_external_runner_program,
};
use earth_dragon_npu::{DragonNpu, NpuStatus};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = match cli::parse_args(&args) {
        Ok(c) => c,
        Err(msg) => {
            eprintln!("error: {msg}");
            eprintln!("{}", help_text());
            std::process::exit(EXIT_USAGE);
        }
    };

    let code = match cmd {
        Command::Help => {
            eprint!("{}", help_text());
            EXIT_OK
        }
        Command::Status {
            json,
            external_runner,
            external_cwd,
        } => cmd_status(json, external_runner.as_deref(), external_cwd.as_deref()),
        Command::Compile {
            model,
            opt,
            quant,
            json,
            external_runner,
            external_cwd,
        } => cmd_compile(
            &model,
            opt,
            &quant,
            json,
            external_runner.as_deref(),
            external_cwd.as_deref(),
        ),
        Command::Run {
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
        } => cmd_run(RunCli {
            model: &model,
            opt,
            quant: &quant,
            input_path: input.as_deref(),
            input_len,
            json,
            ext: ExternalCliOpts {
                runner_flag: external_runner.as_deref(),
                extra_args: &external_args,
                cwd_flag: external_cwd.as_deref(),
            },
            iron_kernel: iron_kernel.as_deref(),
            iron_export,
            host_graph,
        }),
        Command::Probe { json } => cmd_probe(json),
    };
    std::process::exit(code);
}

fn open_npu(
    runner_flag: Option<&str>,
    extra_args: &[String],
    cwd_flag: Option<&str>,
) -> Result<DragonNpu, String> {
    try_external_backend(runner_flag, extra_args, cwd_flag)?.map_or_else(
        || DragonNpu::new().map_err(|e| e.to_string()),
        |backend| Ok(DragonNpu::with_backend(Arc::new(backend))),
    )
}

fn try_external_backend(
    runner_flag: Option<&str>,
    extra_args: &[String],
    cwd_flag: Option<&str>,
) -> Result<Option<ExternalRunnerBackend>, String> {
    let Some(program) = resolve_external_runner_program(runner_flag) else {
        return Ok(None);
    };
    let cwd = resolve_external_cwd(cwd_flag);
    let cfg =
        ExternalRunnerConfig::new(program, extra_args.to_vec(), cwd).map_err(|e| e.to_string())?;
    Ok(Some(ExternalRunnerBackend::new(cfg)))
}

fn cmd_status(json: bool, runner_flag: Option<&str>, cwd_flag: Option<&str>) -> i32 {
    let npu = match open_npu(runner_flag, &[], cwd_flag) {
        Ok(n) => n,
        Err(e) => {
            eprintln!("status failed: {e}");
            return EXIT_ERROR;
        }
    };
    let st = match npu.status() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("status failed: {e}");
            return EXIT_ERROR;
        }
    };
    if json {
        println!("{}", status_json(&st));
    } else {
        print_status_human(&st);
    }
    EXIT_OK
}

fn cmd_compile(
    model: &str,
    opt: u8,
    quant: &str,
    json: bool,
    runner_flag: Option<&str>,
    cwd_flag: Option<&str>,
) -> i32 {
    let npu = match open_npu(runner_flag, &[], cwd_flag) {
        Ok(n) => n,
        Err(e) => {
            eprintln!("compile failed: {e}");
            return EXIT_ERROR;
        }
    };
    let compiled = match npu.compile_model(model, opt, quant) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("compile failed: {e}");
            return EXIT_ERROR;
        }
    };
    let passes: Vec<String> = compiled
        .applied_passes()
        .iter()
        .map(|p| format!("{p:?}"))
        .collect();
    // Applied quant from graph when present; else the CLI request string.
    let quant_applied = compiled
        .chosen_quantization()
        .map_or_else(|| quant.to_string(), ToOwned::to_owned);
    if json {
        let v = serde_json::json!({
            "model_path": compiled.model_path(),
            "optimization_level": compiled.optimization_level(),
            "quantization_requested": quant,
            "quantization": quant_applied,
            "chosen_quantization": compiled.chosen_quantization(),
            "node_count": compiled.node_count(),
            "initial_node_count": compiled.initial_node_count(),
            "optimization_score": compiled.optimization_score(),
            "fused_operations_count": compiled.fused_operations_count(),
            "memory_saved_mb": compiled.memory_saved_mb(),
            "chosen_layout": compiled.chosen_layout(),
            "applied_passes": passes,
            "ops": compiled.ops(),
            "compile_host_only": true,
            "status_simulated": status_simulated_hint(&npu),
            "ran_on_npu": false,
            "invent_pass": 0,
            "note": "compile is always host-side ModelCompiler; no NPU execution",
        });
        println!("{v}");
    } else {
        println!("compiled: {}", compiled.model_path());
        println!("  optimization_level: {}", compiled.optimization_level());
        println!(
            "  nodes: {} (initial {})",
            compiled.node_count(),
            compiled.initial_node_count()
        );
        println!("  score: {}", compiled.optimization_score());
        println!("  fused: {}", compiled.fused_operations_count());
        println!("  passes: {passes:?}");
        println!("  layout: {:?}", compiled.chosen_layout());
        println!(
            "  quantization: {quant_applied} (requested={quant}, chosen={:?})",
            compiled.chosen_quantization()
        );
        println!("  ops: {:?}", compiled.ops());
        println!("  compile_host_only=true; ran_on_npu=false invent_pass=0");
    }
    EXIT_OK
}

fn cmd_run(cli: RunCli<'_>) -> i32 {
    if cli.host_graph {
        return cmd_run_host_graph(cli.model, cli.input_path, cli.input_len, cli.json);
    }
    if let Some(name) = cli.iron_kernel {
        return cmd_run_iron(cli.json, name, cli.iron_export);
    }
    let backend =
        match try_external_backend(cli.ext.runner_flag, cli.ext.extra_args, cli.ext.cwd_flag) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("run failed: {e}");
                return EXIT_ERROR;
            }
        };
    if let Some(backend) = backend {
        return cmd_run_external(cli.model, cli.opt, cli.quant, cli.json, &backend);
    }

    // Independent of status.simulated: DragonNpu::new() with amd-xdna2 uses
    // AmdXdna2Backend::new() Simulation arm for Backend::run even when detected.
    let execution = "host_or_simulation_run";
    let npu = match DragonNpu::new() {
        Ok(n) => n,
        Err(e) => {
            eprintln!("run failed: {e}");
            return EXIT_ERROR;
        }
    };
    // Detection/status label — not the same as "this run executed on silicon".
    let status_simulated = status_simulated_hint(&npu);
    let compiled = match npu.compile_model(cli.model, cli.opt, cli.quant) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("run failed (compile): {e}");
            return EXIT_ERROR;
        }
    };
    let input = match load_input_f32(cli.input_path, cli.input_len) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("run failed (input): {e}");
            return EXIT_ERROR;
        }
    };
    // CLI always uses CompiledModel::run (host/sim path). Not submit_and_run / Real arm.
    let output = match compiled.run(&input) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("run failed: {e}");
            return EXIT_ERROR;
        }
    };
    let time_ms = compiled.last_inference_time_ms();
    if cli.json {
        let preview: Vec<f32> = output.iter().take(8).copied().collect();
        let v = serde_json::json!({
            "model_path": compiled.model_path(),
            "input_len": input.len(),
            "output_len": output.len(),
            "output_preview": preview,
            "inference_time_ms": time_ms,
            "execution": execution,
            "status_simulated": status_simulated,
            "ran_on_npu": false,
            "invent_pass": 0,
            "note": "CLI run uses CompiledModel::run (host/sim). status_simulated is detection metadata only; AmdXdna2Backend::new keeps Simulation arm for run even when hardware is detected. Not silicon GEMM.",
        });
        println!("{v}");
    } else {
        println!("run ok: model={}", compiled.model_path());
        println!("  input_len={} output_len={}", input.len(), output.len());
        if let Some(t) = time_ms {
            println!("  inference_time_ms={t:.4}");
        }
        println!("  execution={execution}");
        println!("  status.simulated={status_simulated} (detection only; run() still host/sim)");
        println!("  ran_on_npu=false invent_pass=0 (honest: no silicon GEMM claim from this CLI)");
        let preview: Vec<f32> = output.iter().take(8).copied().collect();
        println!("  output_preview={preview:?}");
    }
    EXIT_OK
}

fn cmd_run_iron(json: bool, name: &str, iron_export: bool) -> i32 {
    let mlir = match resolve_mlir_aie_root() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("run failed: {e}");
            return EXIT_ERROR;
        }
    };
    let this_run_aot = if iron_kernel_should_create_this_run_aot_dir(name, iron_export) {
        match create_this_run_aot_dir(name) {
            Ok(p) => Some(p),
            Err(e) => {
                eprintln!("run failed: {e}");
                return EXIT_ERROR;
            }
        }
    } else {
        None
    };
    let (cwd, argv) = match iron_kernel_cwd_and_argv_with_aot(name, &mlir, this_run_aot.as_deref())
    {
        Ok(v) => v,
        Err(e) => {
            eprintln!("run failed: {e}");
            return EXIT_ERROR;
        }
    };
    let python = match resolve_iron_python() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("run failed: {e}");
            return EXIT_ERROR;
        }
    };
    let cwd_abs = cwd.canonicalize().unwrap_or(cwd);
    let py_abs = python.canonicalize().unwrap_or(python);
    let cfg = match ExternalRunnerConfig::new(
        py_abs.to_string_lossy().into_owned(),
        argv,
        Some(cwd_abs.to_string_lossy().into_owned()),
    ) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("run failed: {e}");
            return EXIT_ERROR;
        }
    };
    let backend = ExternalRunnerBackend::new(cfg);
    let mut outcome = earth_dragon_npu::cli::run_iron_without_host_model(&backend);
    debug_assert!(!outcome.compiled_host_model);
    debug_assert!(!outcome.ran_on_npu);
    debug_assert_eq!(outcome.invent_pass, INVENT_PASS);
    let (aot_artifact_count, aot_collect_note) =
        apply_this_run_aot_to_outcome(&mut outcome, this_run_aot.as_deref());
    print_iron_child_outcome(
        json,
        name,
        &outcome,
        this_run_aot.as_deref(),
        aot_artifact_count,
        aot_collect_note.as_deref(),
    );
    outcome.exit
}

fn apply_this_run_aot_to_outcome(
    outcome: &mut earth_dragon_npu::cli::IronChildOutcome,
    this_run_aot: Option<&std::path::Path>,
) -> (usize, Option<String>) {
    let Some(aot) = this_run_aot else {
        return (0, None);
    };
    if let Some(bytes) = load_this_run_npy_from_aot_dir(aot) {
        let stdout = outcome.report.as_ref().map_or("", |r| r.stdout.as_str());
        let from_aot = parse_child_f32_vector(stdout, Some(&bytes));
        if !from_aot.is_empty() {
            outcome.parsed = from_aot;
        }
    }
    let n = list_this_run_aot_artifacts(aot).len();
    (
        n,
        Some(format_iron_job_collect_note(
            IRON_NPU2_VECTOR_ADD_JOB,
            n,
            false,
        )),
    )
}

fn print_iron_child_outcome(
    json: bool,
    name: &str,
    outcome: &earth_dragon_npu::cli::IronChildOutcome,
    this_run_aot: Option<&std::path::Path>,
    aot_artifact_count: usize,
    aot_collect_note: Option<&str>,
) {
    if let Some(report) = &outcome.report {
        debug_assert_eq!(report.ran_on_npu, RAN_ON_NPU);
        debug_assert_eq!(report.invent_pass, INVENT_PASS);
        if json {
            let mut v = report.to_json();
            if let Some(obj) = v.as_object_mut() {
                obj.insert(
                    "iron_kernel".into(),
                    serde_json::Value::String(name.to_string()),
                );
                obj.insert("compiled_host_model".into(), serde_json::Value::Bool(false));
                obj.insert(
                    "execution".into(),
                    serde_json::Value::String(EXECUTION_EXTERNAL_RUNNER.to_string()),
                );
                obj.insert("ran_on_npu".into(), serde_json::Value::Bool(RAN_ON_NPU));
                obj.insert(
                    "invent_pass".into(),
                    serde_json::Value::from(u64::from(INVENT_PASS)),
                );
                obj.insert("output".into(), serde_json::json!(outcome.parsed));
                obj.insert("output_len".into(), serde_json::json!(outcome.parsed.len()));
                if let Some(aot) = this_run_aot {
                    obj.insert(
                        "this_run_aot".into(),
                        serde_json::Value::String(aot.display().to_string()),
                    );
                    obj.insert("job".into(), serde_json::json!(IRON_NPU2_VECTOR_ADD_JOB));
                    obj.insert(
                        "aot_artifact_count".into(),
                        serde_json::json!(aot_artifact_count),
                    );
                    if let Some(note) = aot_collect_note {
                        obj.insert("iron_job_collect".into(), serde_json::json!(note));
                    }
                }
            }
            println!("{v}");
        } else if report.child_exit_ok() {
            println!("run ok: iron={name}");
            print_external_run_human_tokens(report);
            println!("  compiled_host_model=false");
            println!("  output_len={}", outcome.parsed.len());
            if !outcome.parsed.is_empty() {
                let preview: Vec<f32> = outcome.parsed.iter().take(8).copied().collect();
                println!("  output_preview={preview:?}");
            }
            if let Some(aot) = this_run_aot {
                println!("  this_run_aot={}", aot.display());
                if let Some(note) = aot_collect_note {
                    println!("  {note}");
                }
            }
        } else {
            println!(
                "run failed: external runner child exited {:?} (not a silent host Pass); \
                 execution=external_runner ran_on_npu=false invent_pass=0",
                report.child_exit_code
            );
            print_external_run_human_tokens(report);
        }
    } else if json {
        let v = serde_json::json!({
            "iron_kernel": name,
            "compiled_host_model": false,
            "execution": EXECUTION_EXTERNAL_RUNNER,
            "ran_on_npu": RAN_ON_NPU,
            "invent_pass": INVENT_PASS,
            "error": outcome.error,
        });
        println!("{v}");
    } else {
        eprintln!(
            "run failed: {} (not a silent host Pass); \
             execution=external_runner ran_on_npu=false invent_pass=0",
            outcome.error.as_deref().unwrap_or("external runner")
        );
    }
}

fn cmd_run_host_graph(model: &str, input_path: Option<&str>, input_len: usize, json: bool) -> i32 {
    #[cfg(not(feature = "amd-xdna2"))]
    {
        let _ = (model, input_path, input_len);
        let msg = "run --host-graph requires --features amd-xdna2 \
                   (rebuild: cargo run --features amd-xdna2 --bin earth-dragon-npu -- run --host-graph)";
        if json {
            let v = serde_json::json!({
                "execution": "host",
                "ran_on_npu": false,
                "invent_pass": 0,
                "error": msg,
            });
            println!("{v}");
        } else {
            eprintln!("{msg}");
        }
        EXIT_ERROR
    }

    #[cfg(feature = "amd-xdna2")]
    {
        let input = if input_path.is_some() {
            match load_input_f32(input_path, input_len) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("run failed (input): {e}");
                    return EXIT_ERROR;
                }
            }
        } else {
            DEFAULT_HOST_GRAPH_INPUT.to_vec()
        };
        let output = match earth_dragon_npu::cli::evaluate_cli_host_graph(model, &input) {
            Ok(o) => o,
            Err(e) => {
                eprintln!("run failed (host-graph): {e}");
                return EXIT_ERROR;
            }
        };
        if json {
            let v = serde_json::json!({
                "model_path": model,
                "input_len": input.len(),
                "output_len": output.len(),
                "output": output,
                "execution": "host",
                "ran_on_npu": false,
                "invent_pass": 0,
                "note": "CLI --host-graph is host evaluate_host_graph only. Not silicon. invent_pass=0.",
                "honesty": earth_dragon_npu::cli::format_host_graph_honesty_tokens(),
            });
            println!("{v}");
        } else {
            println!("run ok: model={model}");
            println!("  input_len={} output_len={}", input.len(), output.len());
            println!(
                "  {}",
                earth_dragon_npu::cli::format_host_graph_honesty_tokens()
            );
            println!("  output={output:?}");
        }
        EXIT_OK
    }
}

fn cmd_run_external(
    model: &str,
    opt: u8,
    quant: &str,
    json: bool,
    backend: &ExternalRunnerBackend,
) -> i32 {
    if !backend.runner_present() {
        eprintln!(
            "run failed: external runner binary is unavailable (not a silent host Pass); \
             execution=external_runner ran_on_npu=false invent_pass=0"
        );
        return EXIT_ERROR;
    }
    let npu = DragonNpu::with_backend(Arc::new(backend.clone()));
    let compiled = match npu.compile_model(model, opt, quant) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("run failed (compile): {e}");
            return EXIT_ERROR;
        }
    };
    let report = match backend.run_external() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("run failed: {e}");
            return EXIT_ERROR;
        }
    };
    debug_assert_eq!(report.ran_on_npu, RAN_ON_NPU);
    debug_assert_eq!(report.invent_pass, INVENT_PASS);
    let parsed = backend.parsed_output(&report);
    if json {
        let mut v = report.to_json();
        if let Some(obj) = v.as_object_mut() {
            obj.insert(
                "model_path".into(),
                serde_json::Value::String(compiled.model_path().to_string()),
            );
            obj.insert("compile_host_only".into(), serde_json::Value::Bool(true));
            // Honesty tokens are constants (not mutable report fields).
            obj.insert(
                "execution".into(),
                serde_json::Value::String(EXECUTION_EXTERNAL_RUNNER.to_string()),
            );
            obj.insert("ran_on_npu".into(), serde_json::Value::Bool(RAN_ON_NPU));
            obj.insert(
                "invent_pass".into(),
                serde_json::Value::from(u64::from(INVENT_PASS)),
            );
            obj.insert("output".into(), serde_json::json!(parsed));
            obj.insert("output_len".into(), serde_json::json!(parsed.len()));
        }
        println!("{v}");
    } else if report.child_exit_ok() {
        println!("run ok: model={}", compiled.model_path());
        print_external_run_human_tokens(&report);
        println!("  output_len={}", parsed.len());
        if !parsed.is_empty() {
            let preview: Vec<f32> = parsed.iter().take(8).copied().collect();
            println!("  output_preview={preview:?}");
        }
    } else {
        println!(
            "run failed: external runner child exited {:?} (not a silent host Pass); \
             execution=external_runner ran_on_npu=false invent_pass=0",
            report.child_exit_code
        );
        print_external_run_human_tokens(&report);
    }
    if report.child_exit_ok() {
        EXIT_OK
    } else {
        EXIT_ERROR
    }
}

fn print_external_run_human_tokens(report: &ExternalRunnerReport) {
    println!("  execution={EXECUTION_EXTERNAL_RUNNER}");
    println!("  ran_on_npu={RAN_ON_NPU}");
    println!("  invent_pass={INVENT_PASS}");
    println!("  product_known_good={PRODUCT_KNOWN_GOOD}");
    println!("  child_exit_code={:?}", report.child_exit_code);
    println!("  child_said_pass={}", report.child_said_pass);
    println!("  runner_path={}", report.runner_path);
    let preview: String = report.stdout.chars().take(240).collect();
    println!("  stdout_preview={preview:?}");
    let err_preview: String = report.stderr.chars().take(400).collect();
    println!("  stderr_preview={err_preview:?}");
    println!("  note={}", report.note);
}

fn cmd_probe(json: bool) -> i32 {
    #[cfg(not(feature = "amd-xdna2"))]
    {
        let msg = "probe requires --features amd-xdna2 (rebuild: cargo run --features amd-xdna2 --bin earth-dragon-npu -- probe)";
        if json {
            let v = serde_json::json!({
                "schema_version": earth_dragon_npu::TIMED_OP_JSON_SCHEMA_VERSION,
                "status": "feature_disabled",
                "detail": msg,
                "ran_on_npu": false,
                "exec_cmd_enabled": false,
            });
            println!("{v}");
        } else {
            eprintln!("{msg}");
            eprintln!(
                "For full probe, prefer: cargo run --features amd-xdna2 --example xdna2_probe"
            );
        }
        EXIT_ERROR
    }

    #[cfg(feature = "amd-xdna2")]
    {
        use earth_dragon_npu::backends::amd_xdna2::{
            AmdXdna2Backend, exec_cmd_block_reason, exec_cmd_is_enabled,
        };

        let backend = AmdXdna2Backend::new_with_real_stub();
        let open = backend.real_device_open();
        let hw = backend.hardware_detected();
        let fw = backend.get_direct_firmware_version().ok();
        let power = backend.get_direct_power_mode().ok();
        let resource = backend.get_direct_resource_info().ok();
        let exec_on = exec_cmd_is_enabled();
        let block = exec_cmd_block_reason();

        // Safe diagnostics only — no EXEC_CMD, no CREATE_HWCTX stress matrix.
        if json {
            let v = serde_json::json!({
                "schema_version": earth_dragon_npu::TIMED_OP_JSON_SCHEMA_VERSION,
                "hardware_detected": hw,
                "real_device_open": open,
                "firmware": fw.as_ref().map(|f| format!("{}.{}.{}.{}", f.major, f.minor, f.patch, f.build)),
                "power_mode": power.map(|p| p.power_mode),
                "resource": resource.as_ref().map(|r| serde_json::json!({
                    "npu_tops_max": r.npu_tops_max,
                    "npu_clk_max": r.npu_clk_max,
                    "npu_task_max": r.npu_task_max,
                })),
                "exec_cmd_enabled": exec_on,
                "exec_cmd_block_reason": block,
                "ran_on_npu": false,
                "note": "CLI probe is diagnostics-only; full matrix: example xdna2_probe; timed JSON: npu_direct_bench",
            });
            println!("{v}");
        } else {
            println!("earth-dragon-npu probe (amd-xdna2, no EXEC_CMD)");
            println!("  hardware_detected={hw}");
            println!("  real_device_open={open}");
            if let Some(f) = fw {
                println!("  firmware={}.{}.{}.{}", f.major, f.minor, f.patch, f.build);
            } else {
                println!("  firmware=(unavailable)");
            }
            if let Some(p) = power {
                println!("  power_mode={}", p.power_mode);
            }
            if let Some(r) = resource {
                println!("  tops_max={} clk_max={}", r.npu_tops_max, r.npu_clk_max);
            }
            println!("  exec_cmd_enabled={exec_on}");
            println!("  exec_cmd_block_reason={block}");
            println!("  ran_on_npu=false");
            println!("  tip: cargo run --features amd-xdna2 --example xdna2_probe");
            println!(
                "  tip: cargo run --features amd-xdna2 --example npu_direct_bench -- --checklist"
            );
        }
        EXIT_OK
    }
}

fn status_simulated_hint(npu: &DragonNpu) -> bool {
    npu.status().map_or(true, |s| s.simulated)
}

fn print_status_human(st: &NpuStatus) {
    println!("Earth Dragon NPU status");
    println!("  vendor:        {}", st.vendor);
    println!("  device_name:   {}", st.device_name);
    println!("  available:     {}", st.available);
    println!("  simulated:     {}", st.simulated);
    println!("  compute_units: {}", st.compute_units);
    println!("  memory_mb:     {}", st.memory_mb);
    println!("  peak_tops:     {}", st.peak_tops);
    println!("  driver:        {}", st.driver_version);
    println!("  dtypes:        {:?}", st.supported_dtypes);
    let execution = if st.vendor == "external-runner" {
        EXECUTION_EXTERNAL_RUNNER
    } else {
        "host_or_simulation_run"
    };
    println!("  execution:     {execution}");
    println!("  ran_on_npu:    {RAN_ON_NPU}");
    println!("  invent_pass:   {INVENT_PASS}");
    println!("  note: simulated= reflects detection/backend labeling only.");
    println!("        Default compile/run use host ModelCompiler + Backend::run");
    println!("        (AmdXdna2Backend::new keeps Simulation arm even when hardware is detected).");
    println!("        --external-runner prints execution=external_runner; still ran_on_npu=false.");
}

fn status_json(st: &NpuStatus) -> String {
    let v = serde_json::json!({
        "vendor": st.vendor,
        "device_name": st.device_name,
        "available": st.available,
        "simulated": st.simulated,
        "compute_units": st.compute_units,
        "memory_mb": st.memory_mb,
        "peak_tops": st.peak_tops,
        "driver_version": st.driver_version,
        "supported_dtypes": st.supported_dtypes,
        "execution": if st.vendor == "external-runner" {
            EXECUTION_EXTERNAL_RUNNER
        } else {
            "host_or_simulation_run"
        },
        "ran_on_npu": RAN_ON_NPU,
        "invent_pass": INVENT_PASS,
        "note": "simulated is detection metadata; default CLI run uses host/sim Backend::run, not silicon GEMM. external-runner wrap still ran_on_npu=false",
    });
    v.to_string()
}
