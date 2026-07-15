//! Integration tests for the real AMD XDNA2 production execution path (device open + Real arm).
//!
//! These tests are the primary cargo-runnable way for hardware owners to validate
//! that the real (non-simulation) XDNA2 device layer works on their physical Ryzen
//! AI 300 / Strix Point / Krackan hardware with the amdxdna driver.
//!
//! ## Running on target hardware (the correct platform)
//! ```bash
//! cargo test --features amd-xdna2 --test xdna2_real_hardware -- --nocapture
//! ```
//!
//! The tests will:
//! - Detect whether real XDNA2 hardware (amdxdna + 1022:17f0 family) is present.
//! - If present: fully exercise `new_with_real_stub()`, the multi-index open logic
//!   (via the public diagnostics), `real_execution_active()`, `real_device_*()`,
//!   and confirm that `submit_and_run` / compile reach the RealXdna2Executor with
//!   honest "not yet wired" errors (the scaffolding is ready for actual command
//!   submission work).
//! - If NOT present (CI, other laptops, containers): skip cleanly with a printed
//!   message. The test binary still compiles and "passes" (no spurious failures).
//!
//! This satisfies the requirement for a cargo integration test that behaves
//! correctly on the correct platform and is a no-op otherwise.
//!
//! See docs/XDNA2_HARDWARE_TESTING.md for the full hardware validation guide
//! and docs/XDNA2_REAL_EXECUTION.md for the production architecture.

#[cfg(not(feature = "amd-xdna2"))]
mod feature_not_enabled {
    #[test]
    fn xdna2_real_hardware_tests_require_the_amd_xdna2_feature() {
        // Exists only so `cargo test` (default features) produces a valid test
        // binary instead of "no tests" warnings for this integration crate.
        // When the feature is enabled the real tests below are compiled instead.
    }
}

#[cfg(feature = "amd-xdna2")]
mod real_xdna2_on_target_platform {
    use dragon_npu::backends::amd_xdna2::AmdXdna2Backend;
    use dragon_npu::backends::{Backend, probe_accel_devices};

    /// Helper: returns true only when this machine has a live amdxdna XDNA2 device
    /// that our probe recognizes (the "correct platform").
    fn running_on_real_xdna2_hardware() -> bool {
        let devices = probe_accel_devices();
        devices.iter().any(|d| {
            d.driver.contains("amdxdna") && d.pci_vendor == "0x1022" && d.pci_device == "0x17f0"
        })
    }

    #[test]
    fn real_xdna2_integration_skips_gracefully_on_non_target_platform() {
        if running_on_real_xdna2_hardware() {
            // On the correct platform we want the *other* tests to run.
            // This one just documents the skip path for everyone else.
            eprintln!("On real XDNA2 hardware — the substantive tests below will execute.");
            return;
        }

        // Normal CI / dev laptop path: prove we detect "not target" and do not
        // claim hardware presence.
        let backend = AmdXdna2Backend::new();
        assert!(
            !backend.hardware_detected(),
            "probe must not falsely claim XDNA2 hardware on this platform"
        );
        assert!(
            !backend.real_execution_active(),
            "normal new() path must not activate Real arm without explicit stub"
        );

        // new_with_real_stub still constructs safely (no panic) even off-target.
        let real_stub = AmdXdna2Backend::new_with_real_stub();
        assert!(!real_stub.hardware_detected());
        // Diagnostics must be safe to call.
        let _ = real_stub.real_device_open();
        let _ = real_stub.real_device_path();

        eprintln!(
            "xdna2_real_hardware: skipped (no amdxdna 1022:17f0 device detected on this platform)"
        );
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn real_xdna2_device_open_and_diagnostics_exercised_on_target_platform() {
        if !running_on_real_xdna2_hardware() {
            eprintln!("Skipping real-device-open integration test: not the target XDNA2 platform.");
            return;
        }

        // === This only runs on the owner's physical hardware ===
        let backend = AmdXdna2Backend::new_with_real_stub();

        // Core contract for real production path
        assert!(
            backend.real_execution_active(),
            "new_with_real_stub() must activate the Real execution arm"
        );
        assert!(
            backend.hardware_detected(),
            "target platform must report hardware_detected() == true"
        );

        // The multi-index open helper (now extracted) has been exercised inside
        // RealXdna2Executor construction. Public diagnostics must be callable.
        let is_open = backend.real_device_open();
        let path = backend.real_device_path();

        // On a working amdxdna system at least one of the accel0..3 nodes should
        // have opened successfully for a fully "production ready" open layer.
        // We do not hard-assert true here because driver load order or udev rules
        // can affect it on first boot; the important thing is that the plumbing
        // reached the open attempt and reported consistently.
        eprintln!(
            "real_xdna2_device_open: is_open={} path={:?}",
            is_open,
            path.map(|p| p.display().to_string())
        );

        // Exercise the new richer device info (RealXdna2DeviceInfo) on real hardware.
        if let Some(info) = backend.real_device_info() {
            eprintln!(
                "real_xdna2_device_info: pci={}:{}/{} variant={:?} modalias_present={}",
                info.pci_vendor,
                info.pci_device,
                info.pci_revision,
                info.xdna2_variant,
                !info.modalias.is_empty()
            );
        }

        // New safe read-only sysfs diagnostics (very useful on real hardware).
        if let Some(det) = backend.real_device_detected() {
            eprintln!(
                "real_xdna2_device_detected: name={} driver={}",
                det.name, det.driver
            );
        }

        // Try a few commonly useful sysfs reads on real amdxdna devices.
        for attr in ["name", "device/uevent", "device/power/control"] {
            if let Some(val) = backend.real_device_read_sysfs(attr) {
                eprintln!("real_xdna2_sysfs[{attr}] = {val}");
            }
        }

        // === Lightweight direct driver queries (the new typed GET_INFO path) ===
        // These now perform real DRM_IOCTL_AMDXDNA_GET_INFO ioctls against the
        // amdxdna driver because direct_submission() on the Real device returns
        // RealDirectSubmission (wired in this cycle).
        //
        // We exercise both the high-level convenience methods on the backend
        // and the DirectSubmission trait surface directly.
        let _ = backend.get_direct_firmware_version().map(|v| {
            eprintln!(
                "direct_firmware: {}.{}.{}.{}",
                v.major, v.minor, v.patch, v.build
            );
        });
        let _ = backend.get_direct_power_mode().map(|m| {
            eprintln!("direct_power_mode: {}", m.power_mode);
        });
        let _ = backend.get_direct_resource_info().map(|r| {
            eprintln!(
                "direct_resource: tops_max={} clk_max={} tasks_max={}",
                r.npu_tops_max, r.npu_clk_max, r.npu_task_max
            );
        });

        // (Advanced users with a RealXdna2Device handle can also use the
        // DirectSubmission trait surface directly for more control.)

        // Full real hwctx lifecycle test using the direct path (bring-up friendly API).
        // We now exercise several (num_tiles, mem_size) combinations because the
        // latest UMQ matrix runs showed consistent EINVAL independent of UMQ size/ty.
        // Different tile/mem values in the CREATE_HWCTX req itself are the next
        // highest-signal axis (plus this gives the user a one-command way to
        // collect the matrix on their hardware).
        // Prefer multiples of AIE2 core.row_count (often 4).
        for (tiles, mem) in [(4, 256 * 1024), (4, 1024 * 1024), (8, 1024 * 1024)] {
            match backend.create_direct_hwctx_with_sizes(tiles, mem, 16 * 1024, 128 * 1024, None) {
                Ok(ctx) => {
                    eprintln!(
                        "real_hwctx_try: SUCCESS tiles={} mem={} handle={} umq_bo={} log_bo={}",
                        tiles, mem, ctx.handle, ctx.umq_bo, ctx.log_buf_bo
                    );
                    let _umq_info = backend.get_direct_bo_info(ctx.umq_bo);
                    let _log_info = backend.get_direct_bo_info(ctx.log_buf_bo);
                    // Best-effort cleanup (fd ignored by real impl).
                    let sub = backend.direct_submission();
                    let _ = sub.destroy_buffer(0, ctx.umq_bo);
                    let _ = sub.destroy_buffer(0, ctx.log_buf_bo);
                    let _ = sub.destroy_hwctx(0, ctx.handle);
                }
                Err(e) => {
                    eprintln!("real_hwctx_try: tiles={tiles} mem={mem} -> {e:?}");
                }
            }
        }

        // Timed matmul surface (honest status; may still be hwctx_blocked).
        let timed = backend.timed_matmul(1, 64, 64, "f32");
        eprintln!(
            "timed_matmul: status={} ran_on_npu={} real_ioctl={} total_s={:.6} detail={}",
            timed.status, timed.ran_on_npu, timed.real_ioctl, timed.total_s, timed.detail,
        );
        assert_eq!(timed.op, "matmul");
        assert!(!timed.status.is_empty());
        // Phase 3: never claim NPU GEMM Pass without numerical verify.
        assert!(
            !timed.ran_on_npu,
            "ran_on_npu must stay false until Phase 3.5 numerical verify; status={}",
            timed.status
        );

        // Safe silicon DMA path (no EXEC_CMD). When device is open, expect real ioctl path.
        let bo = backend.timed_bo_roundtrip(4096);
        eprintln!(
            "timed_bo_roundtrip: status={} ran_on_device_path={} real_ioctl={} size_bytes={:?} detail={}",
            bo.status, bo.ran_on_device_path, bo.real_ioctl, bo.size_bytes, bo.detail,
        );
        assert_eq!(bo.op, "bo_roundtrip");
        assert!(!bo.ran_on_npu);
        assert_eq!(bo.m, 0);
        assert_eq!(bo.size_bytes, Some(4096));
        if backend.real_device_open() {
            assert!(
                bo.real_ioctl,
                "open device must use RealDirectSubmission (not silent stub)"
            );
            // Prefer success; soft-skip only if driver wedge / create fails.
            if bo.status == "bo_roundtrip_ok" {
                assert!(
                    bo.ran_on_device_path,
                    "bo_roundtrip_ok on real open must set ran_on_device_path"
                );
            } else {
                assert!(
                    !bo.ran_on_device_path,
                    "failed bo_roundtrip must not claim device path"
                );
                eprintln!(
                    "timed_bo_roundtrip soft-skip (status={}); driver may be wedged",
                    bo.status
                );
            }
        } else {
            assert!(
                !bo.ran_on_device_path,
                "no open device must never set ran_on_device_path"
            );
        }

        // detected_npu_type must give useful info on real hardware
        let npu_type = backend.detected_npu_type();
        assert!(
            npu_type.is_some(),
            "real XDNA2 hardware must report a detected_npu_type"
        );
        let npu_str = npu_type.unwrap();
        assert!(
            npu_str.contains("XDNA2") || npu_str.contains("NPU"),
            "detected type should mention XDNA2 or NPU family: {npu_str}"
        );

        // status() must report simulated=false and XDNA2 details
        let status = backend
            .status()
            .expect("status must succeed on real hardware");
        assert_eq!(status.vendor, "AMD");
        assert!(
            !status.simulated,
            "real hardware path must report simulated=false"
        );
        assert!(status.device_name.contains("hardware detected"));
        assert!(status.supported_dtypes.contains(&"bf16".to_string()));
    }

    #[test]
    fn real_xdna2_submit_and_run_boundary_reached_on_target_platform() {
        if !running_on_real_xdna2_hardware() {
            eprintln!("Skipping submit_and_run boundary test: not the target platform.");
            return;
        }

        let backend = AmdXdna2Backend::new_with_real_stub();

        // Phase 4: compile succeeds on Real arm via ModelCompiler (host-side).
        let model = backend
            .compile_model("any_real_model.onnx", 2, "bf16")
            .expect("Real arm compile_model must succeed (host ModelCompiler)");
        assert!(!model.ops().is_empty());

        // Pin FailClosed so shell XDNA2_REAL_SUBMIT_MODE cannot flake this boundary test.
        let submit_err = backend
            .submit_and_run_with_mode(
                &model,
                &[0.0f32; 4],
                dragon_npu::backends::amd_xdna2::RealSubmitMode::FailClosed,
            )
            .expect_err("FailClosed Real submit must be RealSubmitNotReady");
        let msg = format!("{submit_err:?}");
        assert!(
            msg.contains("RealSubmitNotReady") || msg.contains("not ready"),
            "error must be actionable Phase 4 fail-closed: {msg}"
        );

        eprintln!(
            "real_xdna2_submit_and_run: Phase 4 compile OK + fail-closed submit exercised on hardware"
        );
    }

    #[test]
    fn real_xdna2_probe_and_backend_consistency_on_target() {
        if !running_on_real_xdna2_hardware() {
            eprintln!("Skipping consistency check: not target platform.");
            return;
        }

        let devices = probe_accel_devices();
        let xdna2_count = devices
            .iter()
            .filter(|d| {
                d.driver.contains("amdxdna") && d.pci_vendor == "0x1022" && d.pci_device == "0x17f0"
            })
            .count();
        assert!(
            xdna2_count >= 1,
            "probe must have found at least one XDNA2 device"
        );

        let backend = AmdXdna2Backend::new_with_real_stub();
        assert!(backend.detected_device().is_some());
        // The backend's internal detected_device should be consistent with what probe returned.
    }
}
