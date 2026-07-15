//! Thin JSON CLI surface for colibrì tools (plan step 5, option A; Phase 3 metrics).
//!
//! ```bash
//! cargo run --features amd-xdna2 --example npu_direct_bench -- --m 1 --n 256 --k 1024
//! cargo run --features amd-xdna2 --example npu_direct_bench -- --shape q_a
//! cargo run --features amd-xdna2 --example npu_direct_bench -- --checklist
//! cargo run --features amd-xdna2 --example npu_direct_bench -- --bo-roundtrip
//! cargo run --features amd-xdna2 --example npu_direct_bench -- --bo-roundtrip --bo-size 8192
//! ```
//!
//! Always prints one JSON object to stdout (machine-readable). Human notes go
//! to stderr. Never claims NPU GEMM Pass when `ran_on_npu` is false.
//! Prefer `--bo-roundtrip` for honest timed silicon DMA without EXEC_CMD.
//!
//! **Exit 0 for `--bo-roundtrip` only when `real_device_open && ran_on_device_path`.**
//! Stub in-memory roundtrips never count as device path Pass.

#[cfg(feature = "amd-xdna2")]
use dragon_npu::backends::amd_xdna2::AmdXdna2Backend;

fn main() {
    #[cfg(not(feature = "amd-xdna2"))]
    {
        eprintln!("npu_direct_bench requires --features amd-xdna2");
        let v = serde_json::json!({
            "schema_version": dragon_npu::TIMED_OP_JSON_SCHEMA_VERSION,
            "status": "feature_disabled",
            "detail": "rebuild with --features amd-xdna2",
            "ran_on_npu": false,
            "ran_on_device_path": false,
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
fn bo_device_path_pass(open: bool, ran_on_device_path: bool, real_ioctl: bool) -> bool {
    open && ran_on_device_path && real_ioctl
}

#[cfg(feature = "amd-xdna2")]
fn print_checklist() {
    let backend = AmdXdna2Backend::new_with_real_stub();
    let open = backend.real_device_open();
    let fw_ok = backend.get_direct_firmware_version().is_ok();
    let timed = backend.timed_matmul(1, 1, 1, "f32");
    let bo = backend.timed_bo_roundtrip(4096);
    let timed_gemm_note = if timed.ran_on_npu {
        "pass"
    } else {
        "blocked_npu_compute"
    };
    // Mirror GEMM honesty: device path Pass only with open + real ioctl + verify.
    let bo_note = if bo_device_path_pass(open, bo.ran_on_device_path, bo.real_ioctl) {
        "pass_device_path"
    } else if !open || !bo.real_ioctl {
        "stub_or_no_device"
    } else {
        bo.status.as_str()
    };
    let v = serde_json::json!({
        "schema_version": dragon_npu::TIMED_OP_JSON_SCHEMA_VERSION,
        "capabilities": AmdXdna2Backend::direct_capability_ids(),
        "host": {
            "hardware_detected": backend.hardware_detected(),
            "real_device_open": open,
            "get_info_ok": fw_ok,
            "create_hwctx_status": timed.status,
            "timed_gemm_ran_on_npu": timed.ran_on_npu,
            "timed_gemm_status": timed.status,
            "bo_roundtrip_status": bo.status,
            "bo_roundtrip_ran_on_device_path": bo.ran_on_device_path,
            "bo_roundtrip_real_ioctl": bo.real_ioctl,
            "bo_roundtrip_total_s": bo.total_s,
            "timed_gemm_hwctx_s": timed.hwctx_s,
            "timed_gemm_host_ref_ok": timed.host_ref_ok,
        },
        "notes": {
            "open": if open { "pass" } else { "fail_or_no_device" },
            "get_info": if fw_ok { "pass" } else { "fail_or_no_device" },
            "create_bo": if open { "pass_when_open" } else { "stub_or_no_device" },
            "sync_bo": if open { "pass_when_open" } else { "stub_or_no_device" },
            "create_hwctx": if open {
                timed.status.as_str()
            } else {
                "stub_or_no_device"
            },
            "destroy_hwctx": "wired",
            "submit": if dragon_npu::backends::amd_xdna2::exec_cmd_is_enabled() {
                "enabled_dangerous_scaffold"
            } else {
                "permanently_gated"
            },
            "exec_cmd_enabled": dragon_npu::backends::amd_xdna2::exec_cmd_is_enabled(),
            "exec_cmd_block_reason": dragon_npu::backends::amd_xdna2::exec_cmd_block_reason(),
            // Never "pass" for GEMM unless ran_on_npu (Phase 3 honesty).
            "timed_gemm": timed_gemm_note,
            "bo_roundtrip": bo_note,
        },
        "detail": timed.detail,
        "bo_roundtrip_detail": bo.detail,
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
            "--bo-size" if idx + 1 < args.len() => {
                idx += 2;
            }
            "--bo-roundtrip" | "--checklist" | "--help" | "-h" => {
                idx += 1;
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
fn parse_bo_size(args: &[String]) -> u64 {
    let mut idx = 0;
    while idx < args.len() {
        if args[idx] == "--bo-size" && idx + 1 < args.len() {
            return args[idx + 1].parse().unwrap_or(4096);
        }
        idx += 1;
    }
    4096
}

#[cfg(feature = "amd-xdna2")]
#[allow(clippy::too_many_lines)] // argv dispatch + JSON assembly for colibrì
fn run_amd() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.iter().any(|a| a == "--help" || a == "-h") {
        eprintln!(
            "Usage: npu_direct_bench [--m M] [--n N] [--k K] [--dtype D] [--shape NAME]\n\
             \t[--checklist] [--bo-roundtrip] [--bo-size BYTES]\n\
             Shapes: q_a (1x256x1024), tiny (1x64x64), default (1x256x1024)\n\
             --bo-roundtrip: timed CREATE_BO+SYNC verify (safe; no EXEC_CMD)\n\
             Output: single JSON object on stdout\n\
             Exit 0 GEMM only when ran_on_npu; --bo-roundtrip only when\n\
             real_device_open && real_ioctl && ran_on_device_path (never stub)"
        );
        std::process::exit(0);
    }

    if args.iter().any(|a| a == "--checklist") {
        print_checklist();
        return;
    }

    let bo_only = args.iter().any(|a| a == "--bo-roundtrip");
    let backend = AmdXdna2Backend::new_with_real_stub();

    if bo_only {
        let size = parse_bo_size(&args);
        let open = backend.real_device_open();
        eprintln!(
            "npu_direct_bench bo_roundtrip: hardware_detected={} real_open={} size={}",
            backend.hardware_detected(),
            open,
            size
        );
        let result = backend.timed_bo_roundtrip(size);
        let device_pass = bo_device_path_pass(open, result.ran_on_device_path, result.real_ioctl);
        let mut json_out = result.to_json_value();
        if let Some(obj) = json_out.as_object_mut() {
            obj.insert(
                "hardware_detected".to_string(),
                serde_json::json!(backend.hardware_detected()),
            );
            obj.insert("real_device_open".to_string(), serde_json::json!(open));
            obj.insert(
                "capabilities".to_string(),
                serde_json::json!(AmdXdna2Backend::direct_capability_ids()),
            );
            obj.insert("timed_gemm".to_string(), serde_json::json!("not_requested"));
            obj.insert(
                "bo_roundtrip_product_pass".to_string(),
                serde_json::json!(device_pass),
            );
            if !device_pass {
                obj.insert(
                    "bo_roundtrip_note".to_string(),
                    serde_json::json!(if !open || !result.real_ioctl {
                        "stub_or_no_device"
                    } else {
                        result.status.as_str()
                    }),
                );
            }
        }
        println!("{json_out}");
        if device_pass {
            std::process::exit(0);
        }
        std::process::exit(2);
    }

    let (rows, cols, inner, dtype) = parse_shape_args(&args);

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
        // Honest: GEMM NPU Pass only when ran_on_npu; envelope submit ≠ Pass.
        obj.insert(
            "timed_gemm".to_string(),
            serde_json::json!(if result.ran_on_npu {
                "pass"
            } else {
                "blocked_npu_compute"
            }),
        );
    }
    println!("{json_out}");

    // Non-zero exit when NPU compute did not complete (tools can still parse JSON).
    if !result.ran_on_npu {
        std::process::exit(2);
    }
}
