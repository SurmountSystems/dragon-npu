//! Thin JSON CLI surface for colibrì tools (plan step 5, option A).
//!
//! ```bash
//! cargo run --features amd-xdna2 --example npu_direct_bench -- --m 1 --n 256 --k 1024
//! cargo run --features amd-xdna2 --example npu_direct_bench -- --shape q_a
//! cargo run --features amd-xdna2 --example npu_direct_bench -- --checklist
//! ```
//!
//! Always prints one JSON object to stdout (machine-readable). Human notes go
//! to stderr. Never claims NPU Pass when CREATE_HWCTX / payload still block.

#[cfg(feature = "amd-xdna2")]
use dragon_npu::backends::amd_xdna2::AmdXdna2Backend;

fn main() {
    #[cfg(not(feature = "amd-xdna2"))]
    {
        eprintln!("npu_direct_bench requires --features amd-xdna2");
        let v = serde_json::json!({
            "status": "feature_disabled",
            "detail": "rebuild with --features amd-xdna2",
            "ran_on_npu": false,
        });
        println!("{v}");
        std::process::exit(1);
    }

    #[cfg(feature = "amd-xdna2")]
    {
        run_amd();
    }
}

#[cfg(feature = "amd-xdna2")]
fn print_checklist() {
    let backend = AmdXdna2Backend::new_with_real_stub();
    let open = backend.real_device_open();
    let fw_ok = backend.get_direct_firmware_version().is_ok();
    let timed = backend.timed_matmul(1, 1, 1, "f32");
    let v = serde_json::json!({
        "capabilities": AmdXdna2Backend::direct_capability_ids(),
        "host": {
            "hardware_detected": backend.hardware_detected(),
            "real_device_open": open,
            "get_info_ok": fw_ok,
            "create_hwctx_status": timed.status,
            "timed_gemm_ran_on_npu": timed.ran_on_npu,
        },
        "notes": {
            "open": if open { "pass" } else { "fail_or_no_device" },
            "get_info": if fw_ok { "pass" } else { "fail_or_no_device" },
            "create_bo": "pass_when_open",
            "sync_bo": "pass_when_open",
            "create_hwctx": timed.status,
            "destroy_hwctx": "wired",
            "submit": if dragon_npu::backends::amd_xdna2::exec_cmd_is_enabled() {
                "enabled_dangerous_scaffold"
            } else {
                "permanently_gated"
            },
            "exec_cmd_enabled": dragon_npu::backends::amd_xdna2::exec_cmd_is_enabled(),
            "exec_cmd_block_reason": dragon_npu::backends::amd_xdna2::exec_cmd_block_reason(),
            "timed_gemm": if timed.ran_on_npu { "pass" } else { "blocked" },
        },
        "detail": timed.detail,
    });
    println!("{v}");
}

#[cfg(feature = "amd-xdna2")]
fn parse_shape_args(args: &[String]) -> (u32, u32, u32, String) {
    let (mut rows, mut cols, mut inner) = (1u32, 256u32, 1024u32);
    let mut dtype = "f32".to_string();

    let mut idx = 0;
    while idx < args.len() {
        match args[idx].as_str() {
            "--m" if idx + 1 < args.len() => {
                rows = args[idx + 1].parse().unwrap_or(rows);
                idx += 2;
            }
            "--n" if idx + 1 < args.len() => {
                cols = args[idx + 1].parse().unwrap_or(cols);
                idx += 2;
            }
            "--k" if idx + 1 < args.len() => {
                inner = args[idx + 1].parse().unwrap_or(inner);
                idx += 2;
            }
            "--dtype" if idx + 1 < args.len() => {
                dtype.clone_from(&args[idx + 1]);
                idx += 2;
            }
            "--shape" if idx + 1 < args.len() => {
                match args[idx + 1].as_str() {
                    "q_a" | "default" => {
                        rows = 1;
                        cols = 256;
                        inner = 1024;
                    }
                    "tiny" => {
                        rows = 1;
                        cols = 64;
                        inner = 64;
                    }
                    other => eprintln!("unknown shape {other}, using current m/n/k"),
                }
                idx += 2;
            }
            other => {
                eprintln!("ignoring arg: {other}");
                idx += 1;
            }
        }
    }
    (rows, cols, inner, dtype)
}

#[cfg(feature = "amd-xdna2")]
fn run_amd() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.iter().any(|a| a == "--help" || a == "-h") {
        eprintln!(
            "Usage: npu_direct_bench [--m M] [--n N] [--k K] [--dtype D] [--shape NAME] [--checklist]\n\
             Shapes: q_a (1x256x1024), tiny (1x64x64), default (1x256x1024)\n\
             Output: single JSON object on stdout"
        );
        std::process::exit(0);
    }

    if args.iter().any(|a| a == "--checklist") {
        print_checklist();
        return;
    }

    let (rows, cols, inner, dtype) = parse_shape_args(&args);

    let backend = AmdXdna2Backend::new_with_real_stub();
    eprintln!(
        "npu_direct_bench: hardware_detected={} real_open={} shape={}x{}x{} dtype={}",
        backend.hardware_detected(),
        backend.real_device_open(),
        rows,
        cols,
        inner,
        dtype
    );

    let result = backend.timed_matmul(rows, cols, inner, &dtype);
    let mut json_out = result.to_json_value();
    if let Some(obj) = json_out.as_object_mut() {
        obj.insert(
            "hardware_detected".to_string(),
            serde_json::json!(backend.hardware_detected()),
        );
        obj.insert(
            "real_device_open".to_string(),
            serde_json::json!(backend.real_device_open()),
        );
        obj.insert(
            "capabilities".to_string(),
            serde_json::json!(AmdXdna2Backend::direct_capability_ids()),
        );
    }
    println!("{json_out}");

    // Non-zero exit when NPU compute did not complete (tools can still parse JSON).
    if !result.ran_on_npu {
        std::process::exit(2);
    }
}
