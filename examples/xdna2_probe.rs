//! Primary bring-up and submission-scaffolding harness for the real AMD XDNA2
//! lightweight direct execution path.
//!
//! ```bash
//! cargo run --features amd-xdna2 --example xdna2_probe
//! ```
//!
//! On real 1022:17f0 + amdxdna hardware this is one of the highest-signal
//! commands you can run:
//! - Exercises all the rich GET_INFO diagnostics (firmware, power, resource/tops/clocks,
//!   sensors, AIE version, etc.).
//! - Runs the full UMQ/log BO + CREATE_HWCTX candidate matrix via
//!   `create_direct_hwctx_with_sizes` (override via XDNA2_HWCTX_* env vars).
//! - **Submission scaffolding**: CREATE_BO(Cmd) diagnostics always run. EXEC_CMD
//!   is permanently double-gated (`xdna2-dangerous-exec-cmd` feature +
//!   `XDNA2_ALLOW_EXEC_CMD=1`). Only when both are enabled will a successful hwctx
//!   be retained for a real-hwctx submit attempt (dummy/scaffold payload — still
//!   dangerous; see docs/XDNA2_NEVER_AGAIN.md). Legacy `XDNA2_TRY_SUBMIT` is ignored.
//!
//! Rich per-attempt errors contain every diagnostic we have (get_bo_info, sync results,
//! power/aie/resource state, destroy results, exact ioctl codes, etc.).
//!
//! This is the tool for rapid iteration on the pure direct path toward the first
//! workable real submission of CompiledModel weights.
//!
//! See docs/XDNA2_REAL_EXECUTION.md and docs/XDNA2_HARDWARE_TESTING.md for more.

#[cfg(feature = "amd-xdna2")]
use dragon_npu::backends::Backend;
#[cfg(feature = "amd-xdna2")]
use dragon_npu::backends::amd_xdna2::AmdXdna2Backend;
#[cfg(feature = "amd-xdna2")]
use dragon_npu::backends::amd_xdna2::direct::{BufferType, Command, CommandType, HardwareContext};

fn main() {
    #[cfg(not(feature = "amd-xdna2"))]
    {
        eprintln!("This example requires the `amd-xdna2` feature.");
        eprintln!("Run: cargo run --features amd-xdna2 --example xdna2_probe");
        std::process::exit(1);
    }

    #[cfg(feature = "amd-xdna2")]
    {
        println!("=== Normal path: AmdXdna2Backend::new() (enhanced sim with real params) ===");
        let normal = AmdXdna2Backend::new();
        print_backend_state("normal", &normal);

        println!(
            "\n=== Real execution path: new_with_real_stub() (exercises production Real arm) ==="
        );
        let real_stub = AmdXdna2Backend::new_with_real_stub();
        print_backend_state("real_stub", &real_stub);

        // Phase 4: Real arm compile uses ModelCompiler; default submit is fail-closed
        // (XDNA2_REAL_SUBMIT_MODE=host_ref for host outputs; npu_exec needs EXEC gates).
        println!("\n=== submit_and_run on Real arm (Phase 4: compile OK, default fail-closed) ===");
        match real_stub.compile_model("model.onnx", 1, "fp32") {
            Ok(model) => {
                println!("real_stub.compile_ok=true");
                println!("real_stub.ops_count={}", model.ops().len());
                let plan = AmdXdna2Backend::xdna2_compile_plan(&model);
                println!("real_stub.host_ref_supported={}", plan.host_ref_supported);
                println!("real_stub.plan_notes={}", plan.notes);
                // Real .onnx files now populate model.initializers() with actual
                // weight tensor data (float32 + int8/int32 + bf16 raw_data) when present.
                if model.has_real_weights() {
                    println!("real_stub.model_has_real_weights=true");
                    println!("real_stub.initializer_count={}", model.initializer_count());
                }
                let input = vec![0.0f32; 10];
                match real_stub.submit_and_run(&model, &input) {
                    Ok(_) => println!(
                        "submit_and_run_ok=true (only expected with XDNA2_REAL_SUBMIT_MODE=host_ref + allowlisted ops; not NPU)"
                    ),
                    Err(e) => println!("submit_and_run_error={e:?}"),
                }
            }
            Err(e) => println!("compile_for_submit_and_run_error={e:?}"),
        }
    }
}

#[cfg(feature = "amd-xdna2")]
#[allow(clippy::too_many_lines)]
fn print_backend_state(label: &str, backend: &AmdXdna2Backend) {
    println!("{label}.hardware_detected={}", backend.hardware_detected());
    println!(
        "{label}.detected_npu_type={:?}",
        backend.detected_npu_type()
    );
    println!(
        "{label}.real_execution_active={}",
        backend.real_execution_active()
    );

    if let Some(dev) = backend.detected_device() {
        println!("{label}.device.name={}", dev.name);
        println!("{label}.device.driver={}", dev.driver);
        println!(
            "{label}.device.pci={}:{} rev={}",
            dev.pci_vendor, dev.pci_device, dev.pci_revision
        );
        let npu_display = if dev.npu_name.is_empty() {
            "(empty — common on some systems)".to_string()
        } else {
            dev.npu_name.clone()
        };
        println!("{label}.device.npu_name={npu_display}");
        println!("{label}.device.modalias={}", dev.modalias);
        println!("{label}.device.variant={:?}", dev.xdna2_variant);
    }

    if let Ok(status) = backend.status() {
        println!("{label}.status.simulated={}", status.simulated);
        println!("{label}.status.device_name={}", status.device_name);
        println!("{label}.status.driver_version={}", status.driver_version);
        println!("{label}.status.peak_tops={}", status.peak_tops);
        println!("{label}.status.compute_units={}", status.compute_units);
        println!("{label}.status.memory_mb={}", status.memory_mb);
        println!("{label}.status.dtypes={:?}", status.supported_dtypes);
    }

    // New production real-device diagnostics (only meaningful on the Real arm
    // when running on actual XDNA2 hardware with amdxdna).
    println!("{label}.real_device_open={}", backend.real_device_open());
    if let Some(p) = backend.real_device_path() {
        println!("{label}.real_device_path={}", p.display());
    }
    if let Some(info) = backend.real_device_info() {
        println!(
            "{label}.real_device_info.pci={}:{}/{}",
            info.pci_vendor, info.pci_device, info.pci_revision
        );
        println!(
            "{label}.real_device_info.npu_name={}",
            if info.npu_name.is_empty() {
                "(empty)"
            } else {
                &info.npu_name
            }
        );
        println!("{label}.real_device_info.variant={:?}", info.xdna2_variant);
        println!(
            "{label}.real_device_info.modalias_present={}",
            !info.modalias.is_empty()
        );
    }

    // New safe sysfs reads (very useful on real amdxdna hardware)
    if let Some(det) = backend.real_device_detected() {
        println!("{label}.real_device_detected.name={}", det.name);
    }
    for attr in ["device/uevent", "device/power/control"] {
        if let Some(val) = backend.real_device_read_sysfs(attr) {
            let short = if val.len() > 60 {
                format!("{}...", &val[..57])
            } else {
                val
            };
            println!("{label}.real_device_sysfs[{attr}]={short}");
        }
    }

    // Lightweight direct driver queries (now perform real GET_INFO ioctls on the Real arm).
    // These are the first typed driver interactions in the pure direct path.
    if let Ok(fw) = backend.get_direct_firmware_version() {
        println!(
            "{label}.direct_firmware={}.{}.{}.{}",
            fw.major, fw.minor, fw.patch, fw.build
        );
    } else {
        println!("{label}.direct_firmware=unsupported_or_error");
    }
    if let Ok(pwr) = backend.get_direct_power_mode() {
        println!("{label}.direct_power_mode={}", pwr.power_mode);
    } else {
        println!("{label}.direct_power_mode=unsupported_or_error");
    }
    if let Ok(res) = backend.get_direct_resource_info() {
        println!(
            "{label}.direct_resource.tops_max={} clk_max={} task_max={}",
            res.npu_tops_max, res.npu_clk_max, res.npu_task_max
        );
    } else {
        println!("{label}.direct_resource=unsupported_or_error");
    }

    if let Ok(s) = backend.get_direct_sensors() {
        // label is a C string; print first part safely
        let label = s
            .label
            .iter()
            .take_while(|&&b| b != 0)
            .map(|&b| b as char)
            .collect::<String>();
        println!("{label}.direct_sensor.label={label} input={}", s.input);
    } else {
        println!("{label}.direct_sensor=unsupported_or_error");
    }

    // Exercise the new BO info path (will typically error until we create real buffers,
    // but proves the wiring and ioctl surface).
    let _ = backend.get_direct_bo_info(0).map(|info| {
        println!(
            "{label}.direct_bo_info.handle=0 map_offset={} xdna_addr={}",
            info.map_offset, info.xdna_addr
        );
    });

    // Determine early whether the user wants us to retain a successful hwctx
    // for an immediate real-hwctx submit_command attempt. Needed before the grid.
    let try_real_submit = dragon_npu::backends::amd_xdna2::direct::exec_cmd_is_enabled();

    // Demonstrate real hwctx create attempts with a small bring-up grid.
    // This is now one of the most useful things to run on real XDNA2 hardware.
    // The rich errors from the deep candidate loop will be printed for each variant.
    //
    // You can override the grid via environment variables for easy experimentation
    // without recompiling:
    //   XDNA2_HWCTX_TILES=1,2
    //   XDNA2_HWCTX_MEM=262144,1048576
    //   XDNA2_HWCTX_UMQ=4096,16384
    //   XDNA2_HWCTX_LOG=65536,131072
    //   XDNA2_BO_FLAGS=0,0x10,0x20   (forces the CREATE_BO flags matrix; see logs for what was tried)
    //   XDNA2_HWCTX_MAX_OPC=...      (overrides max_opc in the CREATE_HWCTX request)
    //   XDNA2_HWCTX_UMQ_DOORBELL=... (overrides umq_doorbell in the CREATE_HWCTX request)
    //   XDNA2_HWCTX_EXT_FLAGS=...    (overrides ext_flags in the CREATE_HWCTX request)
    // num_tiles should be a multiple of AIE2 core.row_count (often 4):
    // aie2_ctx_init sets orig_num_col = num_tiles / row_count.
    // Prefer 4/8 over 1/2 so column math is non-zero on real silicon.
    let default_variants = [
        (4, 256 * 1024, 4 * 1024, 64 * 1024),
        (4, 1024 * 1024, 16 * 1024, 128 * 1024),
        (8, 1024 * 1024, 16 * 1024, 128 * 1024),
    ];
    let hwctx_variants =
        parse_hwctx_variants_from_env().unwrap_or_else(|| default_variants.to_vec());
    // Track one successful hwctx (if any) for real-hwctx submit only when
    // exec_cmd_is_enabled() (feature + XDNA2_ALLOW_EXEC_CMD=1). Always cleaned up.
    let mut successful_ctx_for_submit: Option<HardwareContext> = None;
    let mut hwctx_successes: usize = 0;
    let mut hwctx_failures: usize = 0;

    for (i, (tiles, mem, umq_sz, log_sz)) in hwctx_variants.iter().enumerate() {
        match backend.create_direct_hwctx_with_sizes(*tiles, *mem, *umq_sz, *log_sz, None) {
            Ok(ctx) => {
                hwctx_successes += 1;
                println!(
                    "{label}.direct_hwctx[{i}].SUCCESS tiles={tiles} mem={mem} umq={umq_sz} log={log_sz} handle={}",
                    ctx.handle
                );
                if successful_ctx_for_submit.is_none() && try_real_submit {
                    // Keep the first success alive for the submission exercise below.
                    successful_ctx_for_submit = Some(ctx);
                } else {
                    let _ = backend.destroy_direct_hwctx(ctx.handle);
                }
            }
            Err(e) => {
                hwctx_failures += 1;
                let err_str = format!("{e:?}");
                let display = if err_str.len() > 600 {
                    format!(
                        "{}... [truncated, {} total chars; full details in library error]",
                        &err_str[..600],
                        err_str.len()
                    )
                } else {
                    err_str
                };
                println!(
                    "{label}.direct_hwctx[{i}].error tiles={tiles} mem={mem} umq={umq_sz} log={log_sz} -> {display}"
                );
            }
        }
    }

    // One more typed query: AIE version.
    let _ = backend.get_direct_aie_version().map(|v| {
        println!("{label}.direct_aie_version={}.{}", v.major, v.minor);
    });

    // Observability: how many hwctxs has this backend created so far via the direct path.
    println!(
        "{label}.direct_hwctx_count={}",
        backend.created_direct_hwctx_count()
    );

    // =====================================================================
    // Submission scaffolding exercise (the bridge to real model execution)
    // =====================================================================
    // EXEC_CMD is permanently gated (docs/XDNA2_NEVER_AGAIN.md). Default builds
    // never issue the ioctl — not even with a fake hwctx.
    println!("\n=== Submission path (EXEC_CMD permanently gated) ===");
    println!(
        "{label}.exec_cmd_enabled={} reason={}",
        dragon_npu::backends::amd_xdna2::direct::exec_cmd_is_enabled(),
        dragon_npu::backends::amd_xdna2::direct::exec_cmd_block_reason()
    );

    let sub = backend.direct_submission();
    if !dragon_npu::backends::amd_xdna2::direct::exec_cmd_is_enabled() {
        println!(
            "{label}.submit_skipped=1 (CREATE_BO(Cmd) still ok for diagnostics; no EXEC_CMD ioctl)"
        );
        if let Ok(h) = sub.create_buffer(0, 4096, BufferType::Cmd) {
            println!(
                "{label}.cmd_bo_only.handle={} (destroyed without submit)",
                h.0
            );
            let _ = sub.destroy_buffer(0, h.0);
        }
        // Drop any real ctx without submitting.
        if let Some(real_ctx) = successful_ctx_for_submit.take() {
            println!(
                "{label}.real_hwctx_held_but_no_submit.handle={}",
                real_ctx.handle
            );
            let _ = backend.destroy_direct_hwctx(real_ctx.handle);
        }
    } else if let Some(real_ctx) = successful_ctx_for_submit.take() {
        // Only reached with feature xdna2-dangerous-exec-cmd + XDNA2_ALLOW_EXEC_CMD=1.
        println!(
            "{label}.submit_real_hwctx.handle={} (DANGEROUS path explicitly enabled)",
            real_ctx.handle
        );

        let sub2 = backend.direct_submission();
        // Prefer submit+wait (finite timeout) over bare EXEC_CMD when gates are open.
        // See docs/XDNA2_EXEC_CMD_LAB.md — single-shot only; never loop; never empty Cmd BO.
        let wait_ms = std::env::var("XDNA2_WAIT_TIMEOUT_MS")
            .ok()
            .and_then(|s| s.parse::<u32>().ok())
            .filter(|&v| v > 0)
            .unwrap_or(dragon_npu::backends::amd_xdna2::DEFAULT_WAIT_CMD_TIMEOUT_MS);

        // Phase 1 START_NPU envelope + host fill is mandatory before EXEC_CMD.
        // inst_buffer=0 is structural research only (not GEMM). Refuse rather than
        // empty-submit (2026-07-13 wedge class). No no-cmd-bo fallback.
        match dragon_npu::backends::amd_xdna2::build_ert_start_npu_cmd_bo(0, 0, &[], 0x1) {
            Ok(payload) => {
                let cmd_bo_size = payload.len() as u64;
                match sub2.create_buffer(0, cmd_bo_size, BufferType::Cmd) {
                    Ok(cmd_handle) => {
                        println!(
                            "{label}.submit_real_hwctx.cmd_bo.created handle={} size={} \
                             (START_NPU envelope)",
                            cmd_handle.0, cmd_bo_size
                        );
                        match dragon_npu::backends::amd_xdna2::fill_cmd_bo_with_payload(
                            sub2.as_ref(),
                            0,
                            cmd_handle.0,
                            &payload,
                        ) {
                            Ok(()) => {
                                println!(
                                    "{label}.submit_real_hwctx.cmd_bo.host_filled bytes={}",
                                    payload.len()
                                );
                                let real_cmd = Command {
                                    ty: CommandType::SubmitExecBuf,
                                    hwctx_handle: real_ctx.handle,
                                    payload,
                                    cmd_bo_handle: Some(cmd_handle.0),
                                    // Envelope-only probe path: no IFM/WTS/OFM pin list.
                                    arg_bo_handles: Vec::new(),
                                };
                                match dragon_npu::backends::amd_xdna2::submit_and_wait_command(
                                    sub2.as_ref(),
                                    0,
                                    &real_cmd,
                                    wait_ms,
                                ) {
                                    Ok(r) => {
                                        println!(
                                            "{label}.submit_real_hwctx.success seq={} wait_ms={} \
                                             cmd_bo={} (EXEC_CMD+WAIT accept only — not ran_on_npu)",
                                            r.seq, r.timeout_ms, cmd_handle.0
                                        );
                                    }
                                    Err(e) => {
                                        println!(
                                            "{label}.submit_real_hwctx.error cmd_bo={} \
                                             wait_ms={wait_ms} -> {e:?}",
                                            cmd_handle.0
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                println!(
                                    "{label}.submit_real_hwctx.cmd_bo_fill_failed handle={} -> {e:?} \
                                     (refusing EXEC_CMD; empty Cmd BO is the wedge class)",
                                    cmd_handle.0
                                );
                            }
                        }
                        let _ = sub2.destroy_buffer(0, cmd_handle.0);
                    }
                    Err(e) => {
                        println!(
                            "{label}.submit_real_hwctx.cmd_bo.creation_failed -> {e:?} \
                             (refusing EXEC_CMD without host-filled Cmd BO; no empty fallback)"
                        );
                    }
                }
            }
            Err(e) => {
                println!(
                    "{label}.submit_real_hwctx.payload_build_failed -> {e:?} \
                     (refusing EXEC_CMD)"
                );
            }
        }

        // Clean up the hwctx itself (BO lifetime best-effort in bring-up).
        let _ = backend.destroy_direct_hwctx(real_ctx.handle);
    } else if try_real_submit {
        println!(
            "{label}.submit_real_hwctx=no_successful_ctx_from_grid (no real EXEC_CMD attempted this run)"
        );
    }

    // Final cleanup of any retained ctx (belt-and-suspenders).
    if let Some(ctx) = successful_ctx_for_submit.take() {
        let _ = backend.destroy_direct_hwctx(ctx.handle);
    }

    // =====================================================================
    // Bring-up summary & recommended next experiments (for the user on real hardware)
    // =====================================================================
    println!("\n=== Bring-up summary & recommended next experiments ===");
    println!(
        "{label}.bringup_status=CREATE_HWCTX grid: {hwctx_successes} ok, {hwctx_failures} fail (of {} variants)",
        hwctx_variants.len()
    );
    if hwctx_successes > 0 {
        println!(
            "{label}.bringup_status=CREATE_HWCTX works on this host (Real arm + current amdxdna/firmware)."
        );
        println!(
            "{label}.bringup_status=Open/GET_INFO/create+destroy hwctx path is usable for payload research."
        );
    } else {
        println!(
            "{label}.bringup_status=CREATE_HWCTX still failing all variants — check driver (stock vs hardened), DevHeap, qos_p, tiles×4."
        );
        println!(
            "{label}.bringup_status=Note: Simulation/normal backend path cannot create real hwctx (needs Real arm)."
        );
    }
    println!(
        "{label}.bringup_status=EXEC_CMD permanently gated (feature xdna2-dangerous-exec-cmd + XDNA2_ALLOW_EXEC_CMD=1); lab kernel may also need exec_cmd_allow=1."
    );
    println!(
        "{label}.bringup_status=Never claim ran_on_npu / microbench Pass from scaffold EXEC_CMD (docs/XDNA2_NEVER_AGAIN.md)."
    );

    if hwctx_successes > 0 {
        println!(
            "{label}.recommended_next=Research real Cmd BO / txn payload (public ERT/cmdlist only; no default submit)."
        );
        println!(
            "{label}.recommended_next=Keep L1+L3 gates closed until a known-good payload exists."
        );
        println!(
            "{label}.recommended_next=Optional: wire timed_matmul host-side path to report hwctx_ok timings more richly."
        );
    } else {
        println!(
            "{label}.recommended_next=Use new_with_real_stub / Real arm; verify L2+L3 or stock amdxdna + firmware alias."
        );
        println!(
            "{label}.recommended_next=Diagnose: bash scripts/diagnose_amdxdna.sh (never open /dev/accel while WEDGED_*)."
        );
    }
    println!(
        "{label}.recommended_next=Hardened install / never-again: docs/XDNA2_NEVER_AGAIN.md + docs/XDNA2_HARDENED_INSTALL.md"
    );
}

/// Parse a small hwctx experiment grid from environment variables.
/// Format: comma-separated lists for tiles, mem (bytes), umq, log.
/// Example: XDNA2_HWCTX_TILES=1,2 XDNA2_HWCTX_MEM=262144,1048576 ...
#[cfg(feature = "amd-xdna2")]
fn parse_hwctx_variants_from_env() -> Option<Vec<(u32, u32, u64, u64)>> {
    use std::env;

    let tiles: Vec<u32> = env::var("XDNA2_HWCTX_TILES")
        .ok()?
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();

    let mems: Vec<u32> = env::var("XDNA2_HWCTX_MEM")
        .ok()?
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();

    let umqs: Vec<u64> = env::var("XDNA2_HWCTX_UMQ")
        .ok()?
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();

    let logs: Vec<u64> = env::var("XDNA2_HWCTX_LOG")
        .ok()?
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();

    if tiles.is_empty() || mems.is_empty() || umqs.is_empty() || logs.is_empty() {
        return None;
    }

    // Build a small cartesian product (kept small on purpose for bring-up runs)
    let mut out = Vec::new();
    for &t in &tiles {
        for &m in &mems {
            for &u in &umqs {
                for &l in &logs {
                    out.push((t, m, u, l));
                    if out.len() > 12 {
                        // safety cap
                        return Some(out);
                    }
                }
            }
        }
    }
    Some(out)
}
