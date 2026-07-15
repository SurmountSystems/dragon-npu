//! `dragon-npu` — thin CLI over the DragonNPU library.
//!
//! Host/sim `compile` + `run` work without hardware. Optional `probe` needs
//! `--features amd-xdna2`. This binary **never** enables EXEC_CMD or advertises
//! unsafe enable flags.
//!
//! ```bash
//! cargo run --bin dragon-npu -- status
//! cargo run --bin dragon-npu -- compile tests/fixtures/host_matmul_relu.json -O 2
//! cargo run --bin dragon-npu -- run tests/fixtures/host_matmul_relu.json --input-len 4
//! cargo run --features amd-xdna2 --bin dragon-npu -- probe
//! ```
//!
//! Colibrì timed JSON remains `examples/npu_direct_bench` (see `docs/COLIBRI_ADAPTER.md`).

use dragon_npu::cli::{self, Command, EXIT_ERROR, EXIT_OK, EXIT_USAGE, help_text, load_input_f32};
use dragon_npu::{DragonNpu, NpuStatus};

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
        Command::Status { json } => cmd_status(json),
        Command::Compile {
            model,
            opt,
            quant,
            json,
        } => cmd_compile(&model, opt, &quant, json),
        Command::Run {
            model,
            opt,
            quant,
            input,
            input_len,
            json,
        } => cmd_run(&model, opt, &quant, input.as_deref(), input_len, json),
        Command::Probe { json } => cmd_probe(json),
    };
    std::process::exit(code);
}

fn cmd_status(json: bool) -> i32 {
    let npu = match DragonNpu::new() {
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

fn cmd_compile(model: &str, opt: u8, quant: &str, json: bool) -> i32 {
    let npu = match DragonNpu::new() {
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
        println!("  compile_host_only=true; ran_on_npu=false");
    }
    EXIT_OK
}

fn cmd_run(
    model: &str,
    opt: u8,
    quant: &str,
    input_path: Option<&str>,
    input_len: usize,
    json: bool,
) -> i32 {
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
    let compiled = match npu.compile_model(model, opt, quant) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("run failed (compile): {e}");
            return EXIT_ERROR;
        }
    };
    let input = match load_input_f32(input_path, input_len) {
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
    if json {
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
        println!("  ran_on_npu=false (honest: no silicon GEMM claim from this CLI)");
        let preview: Vec<f32> = output.iter().take(8).copied().collect();
        println!("  output_preview={preview:?}");
    }
    EXIT_OK
}

fn cmd_probe(json: bool) -> i32 {
    #[cfg(not(feature = "amd-xdna2"))]
    {
        let msg = "probe requires --features amd-xdna2 (rebuild: cargo run --features amd-xdna2 --bin dragon-npu -- probe)";
        if json {
            let v = serde_json::json!({
                "schema_version": dragon_npu::TIMED_OP_JSON_SCHEMA_VERSION,
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
        use dragon_npu::backends::amd_xdna2::{
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
                "schema_version": dragon_npu::TIMED_OP_JSON_SCHEMA_VERSION,
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
            println!("dragon-npu probe (amd-xdna2, no EXEC_CMD)");
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
    println!("DragonNPU status");
    println!("  vendor:        {}", st.vendor);
    println!("  device_name:   {}", st.device_name);
    println!("  available:     {}", st.available);
    println!("  simulated:     {}", st.simulated);
    println!("  compute_units: {}", st.compute_units);
    println!("  memory_mb:     {}", st.memory_mb);
    println!("  peak_tops:     {}", st.peak_tops);
    println!("  driver:        {}", st.driver_version);
    println!("  dtypes:        {:?}", st.supported_dtypes);
    println!("  ran_on_npu:    false");
    println!("  note: simulated= reflects detection/backend labeling only.");
    println!("        CLI compile/run use host ModelCompiler + Backend::run");
    println!("        (AmdXdna2Backend::new keeps Simulation arm even when hardware is detected).");
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
        "ran_on_npu": false,
        "cli_run_execution": "host_or_simulation_run",
        "note": "simulated is detection metadata; CLI run uses host/sim Backend::run, not silicon GEMM",
    });
    v.to_string()
}
