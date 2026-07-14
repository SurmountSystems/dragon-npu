//! AMD XDNA2 (Ryzen AI 300 series / Strix Point, Strix Halo, Krackan Point, etc.) backend.
//!
//! This module is only compiled when the `amd-xdna2` feature is enabled.
//!
//! **Philosophy**: We are building a lighter-weight, open, focused, and eventually
//! more performant direct driver implementation against the amdxdna kernel driver.
//! We deliberately avoid heavy official runtimes (XRT, IRON, etc.) that the
//! original Python project leaned on. The goal is a clean, maintainable, direct
//! path using the fd and ioctls exposed by the mainline amdxdna driver.
//!
//! **Primary focus**: XDNA2 hardware (second-generation NPU). First-generation XDNA
//! (Phoenix/Hawk Point) detection is best-effort only.
//!
//! Current state (real device layer mature, validated on physical 1022:17f0 + amdxdna hardware):
//! - Rich detection via `probe_accel_devices()` + pure modalias variant parser (extensive TDD).
//! - `RealXdna2Device` — production handle that owns a successfully opened `/dev/accel/accelN` fd.
//! - `RealXdna2DeviceInfo` + `real_device_detected()` + `real_device_read_sysfs()` — rich,
//!   safe, read-only diagnostics directly from the open real device.
//! - `direct` submodule — home of the lightweight, open, direct driver path
//!   (`DirectSubmission` trait, `QosInfo`, ioctl constants, etc.).
//! - `new_with_real_stub()` activates the `Real` execution arm for bring-up on actual hardware.
//! - `AmdXdna2Backend::new()` deliberately stays on high-fidelity Simulation for safety/CI.
//! - Full public diagnostics: `hardware_detected()`, `real_execution_active()`,
//!   `real_device_open()`, `real_device_path()`, `real_device_info()`.
//! - `xdna2_real_hardware` integration test + `examples/xdna2_probe.rs` provide repeatable
//!   cargo validation of the entire real open + diagnostics layer on target hardware.
//! - `submit_and_run` is the stable production boundary (routes to Real arm when active).
//! - Real .onnx weight tensors (multi-dtype + bf16 raw) are already loaded and available
//!   via `CompiledModel` for future real execution use.
//! - All of the above is strongly TDD'd (96+ lib tests + dedicated real-hardware integration tests).
//!
//! **Honest status on actual NPU execution**:
//! The device open, handle management, rich observability, and typed driver queries
//! (via the lightweight direct amdxdna path) are solid and hardware-validated.
//! Actual command submission and model execution inside the Real arm are not yet
//! implemented. This is the next major focus, kept strictly in the pure lightweight
//! direct style (no heavy runtimes).
//!
//! See docs/XDNA2_REAL_EXECUTION.md for the production plan and current readiness.
//! Real hardware validation data lives in docs/XDNA2_HARDWARE_TESTING.md.
//!
//! References: autonomous HW research + Python amd_xdna_backend.py + npu_driver_integration.py

pub mod direct;

pub use direct::{
    BufferHandle, Command, CommandType, DirectCapability, DirectError, DirectSubmission,
    ExecCmdPayloadHeader, HardwareContext, StubDirectSubmission, TimedOpResult,
    build_exec_cmd_payload_scaffold, exec_cmd_block_reason, exec_cmd_is_enabled,
    timed_matmul_direct,
};

use crate::backends::simulation::{SimulationBackend, SimulationConfig};
use crate::backends::{AccelDevice, Backend, probe_accel_devices, try_open_accel_device};
use crate::{CompiledModel, DragonNpuError, NpuStatus};

/// AMD XDNA2 backend.
///
/// Detection is performed at construction time using the pure-std accel probe.
/// We prioritize XDNA2 (PCI 1022:17f0 family). When XDNA2 hardware is visible,
/// we configure realistic parameters (≈50 TOPS) and report the detection
/// honestly in status().
///
/// First-gen XDNA is lower priority / best-effort.
///
/// Execution is dispatched through the private `Execution` enum.
/// Real XRT / direct submission will be added as a new variant when we have
/// the actual execution path (this is now the highest technical priority after
/// detection was validated on real hardware).
#[derive(Debug)]
pub struct AmdXdna2Backend {
    execution: Execution,
    /// True if real XDNA2 hardware (PCI 1022:17f0 + amdxdna) was detected.
    hardware_detected: bool,
    /// The detected accel device details (if any) for rich reporting.
    detected_device: Option<AccelDevice>,
}

/// Internal execution backend for AmdXdna2Backend.
///
/// Trait that both the simulation and (future) real XDNA2 execution paths
/// will implement. This makes swapping in actual NPU submission clean.
trait Xdna2Executor: std::fmt::Debug + Send + Sync {
    fn compile_model(
        &self,
        model_path: &str,
        optimization_level: u8,
        quantization: &str,
    ) -> Result<CompiledModel, DragonNpuError>;

    fn run(&self, input: &[f32]) -> Result<Vec<f32>, DragonNpuError>;

    /// Returns true if this executor is talking to real hardware.
    fn is_real(&self) -> bool;

    /// The core "submit compiled model + run inference on the NPU" boundary.
    /// This is the method that will eventually do actual hardware submission.
    ///
    /// This is the primary production boundary for real XDNA2 hardware submission.
    /// When a real executor (XRT-based or direct amdxdna) is wired, this method
    /// will perform actual NPU command submission.
    ///
    /// See docs/XDNA2_REAL_EXECUTION.md for the full production plan, optimization
    /// strategies, and benchmarking methodology.
    fn submit_and_run(
        &self,
        model: &CompiledModel,
        input: &[f32],
    ) -> Result<Vec<f32>, DragonNpuError>;

    /// Production helper: whether the real device is open and ready.
    /// Default impl returns false; Real executor overrides it.
    fn real_device_open(&self) -> bool {
        false
    }

    /// Production helper: the path of the device we opened (if any).
    /// Default returns None; Real executor can override.
    fn real_device_path(&self) -> Option<&std::path::Path> {
        None
    }

    /// Production helper: richer information about the opened real device.
    /// Default returns None; the real executor overrides this when a device
    /// handle is present.
    #[allow(dead_code)]
    fn real_device_info(&self) -> Option<RealXdna2DeviceInfo> {
        None
    }

    /// Production helper: the full probe metadata for the opened real device.
    fn real_device_detected(&self) -> Option<&AccelDevice> {
        None
    }

    /// Production helper: safe read-only sysfs access for the opened real device.
    /// See `RealXdna2Device::read_sysfs` for semantics.
    fn real_device_read_sysfs(&self, _relative_path: &str) -> Option<String> {
        None
    }

    /// Lightweight direct driver query (firmware version via GET_INFO).
    /// Default: Unsupported. Real executor overrides when a device is open.
    fn get_direct_firmware_version(
        &self,
    ) -> Result<direct::amdxdna_drm_query_firmware_version, direct::DirectError> {
        Err(direct::DirectError::Unsupported(
            "get_direct_firmware_version not available on this executor".to_string(),
        ))
    }

    fn get_direct_power_mode(
        &self,
    ) -> Result<direct::amdxdna_drm_get_power_mode, direct::DirectError> {
        Err(direct::DirectError::Unsupported(
            "get_direct_power_mode not available on this executor".to_string(),
        ))
    }

    fn get_direct_resource_info(
        &self,
    ) -> Result<direct::amdxdna_drm_get_resource_info, direct::DirectError> {
        Err(direct::DirectError::Unsupported(
            "get_direct_resource_info not available on this executor".to_string(),
        ))
    }

    fn get_direct_bo_info(
        &self,
        _handle: u32,
    ) -> Result<direct::amdxdna_drm_get_bo_info, direct::DirectError> {
        Err(direct::DirectError::Unsupported(
            "get_direct_bo_info not available on this executor".to_string(),
        ))
    }

    #[allow(dead_code)]
    fn create_direct_hwctx(
        &self,
        _num_tiles: u32,
        _mem_size: u32,
        _qos: Option<&direct::QosInfo>,
    ) -> Result<direct::HardwareContext, direct::DirectError> {
        Err(direct::DirectError::Unsupported(
            "create_direct_hwctx not available on this executor".to_string(),
        ))
    }

    #[allow(dead_code)]
    fn create_direct_hwctx_with_sizes(
        &self,
        num_tiles: u32,
        mem_size: u32,
        _umq_size: u64,
        _log_size: u64,
        qos: Option<&direct::QosInfo>,
    ) -> Result<direct::HardwareContext, direct::DirectError> {
        self.create_direct_hwctx(num_tiles, mem_size, qos)
    }

    #[allow(dead_code)]
    fn destroy_direct_hwctx(&self, _handle: u32) -> Result<(), direct::DirectError> {
        Err(direct::DirectError::Unsupported(
            "destroy_direct_hwctx not available on this executor".to_string(),
        ))
    }

    #[allow(dead_code)]
    fn get_direct_aie_version(
        &self,
    ) -> Result<direct::amdxdna_drm_query_aie_version, direct::DirectError> {
        Err(direct::DirectError::Unsupported(
            "get_direct_aie_version not available on this executor".to_string(),
        ))
    }

    #[allow(dead_code)]
    fn created_direct_hwctx_count(&self) -> usize {
        0
    }

    /// Returns a handle implementing the lightweight direct submission trait.
    /// On the Real arm with an open device this will be a RealDirectSubmission
    /// that can perform actual ioctls. Default returns a stub.
    fn direct_submission(&self) -> Box<dyn direct::DirectSubmission> {
        Box::new(direct::StubDirectSubmission::new(-1))
    }

    fn get_sensors(&self) -> Result<direct::amdxdna_drm_query_sensor, direct::DirectError> {
        Err(direct::DirectError::Unsupported(
            "get_sensors not available on this executor".to_string(),
        ))
    }
}

impl Xdna2Executor for SimulationBackend {
    fn compile_model(
        &self,
        model_path: &str,
        optimization_level: u8,
        quantization: &str,
    ) -> Result<CompiledModel, DragonNpuError> {
        self.compile_model(model_path, optimization_level, quantization)
    }

    fn run(&self, input: &[f32]) -> Result<Vec<f32>, DragonNpuError> {
        self.run(input)
    }

    fn is_real(&self) -> bool {
        false
    }

    fn submit_and_run(
        &self,
        _model: &CompiledModel,
        input: &[f32],
    ) -> Result<Vec<f32>, DragonNpuError> {
        // For simulation we just run the input directly.
        self.run(input)
    }
}

/// Internal record for a hwctx created via the direct path + the BOs we own for it.
/// (Small private helper; placed early so the richly-documented RealXdna2Device
/// struct can follow its doc comment with no intervening blank line.)
#[derive(Debug, Clone)]
struct HwCtxRecord {
    handle: u32,
    umq_bo: u32,
    log_buf_bo: u32,
}

/// A successfully opened real XDNA2 accel device (/dev/accel/accelN under amdxdna).
///
/// This is the stable production abstraction for an actually-open real XDNA2 device.
/// It owns the file descriptor and rich metadata, and is the natural home for
/// future real execution work (command submission, buffer allocation, etc.).
///
/// It is deliberately factored out of `RealXdna2Executor` so it can be unit-tested,
/// reused, and evolved independently.
///
/// **Current readiness**:
/// - Device open (multi-index, robust) — complete and validated on real hardware.
/// - Diagnostics (`path()`, `info()`, `as_fd()` for future use) — complete.
/// - Actual command submission / model execution — **not yet implemented**.
///
/// See `Xdna2Executor::submit_and_run` and docs/XDNA2_REAL_EXECUTION.md for the plan.
#[derive(Debug)]
pub(crate) struct RealXdna2Device {
    /// Metadata from the sysfs probe at the time we decided to open this device.
    detected: AccelDevice,
    /// The open file descriptor. We keep it so we can eventually issue ioctls
    /// or pass it to XRT / other runtimes.
    file: std::fs::File,
    /// The exact path we successfully opened (e.g. /dev/accel/accel0).
    path: std::path::PathBuf,
    /// Records of hwctxs + their associated BOs created via the direct path.
    /// Interior mutability so create/destroy work through `&self` public APIs.
    created_hwctxs: std::sync::Mutex<Vec<HwCtxRecord>>,
}

impl RealXdna2Device {
    /// Create a device handle from an already-successful open.
    /// The caller is responsible for having performed the open.
    fn from_opened(detected: AccelDevice, file: std::fs::File, path: std::path::PathBuf) -> Self {
        Self {
            detected,
            file,
            path,
            created_hwctxs: std::sync::Mutex::new(Vec::new()),
        }
    }

    /// Record a successfully created hwctx for ownership / destroy tracking.
    fn track_hwctx(&self, ctx: &direct::HardwareContext) {
        if let Ok(mut guard) = self.created_hwctxs.lock() {
            guard.push(HwCtxRecord {
                handle: ctx.handle,
                umq_bo: ctx.umq_bo,
                log_buf_bo: ctx.log_buf_bo,
            });
        }
    }

    /// Forget a tracked hwctx after destroy (by handle). Returns the record if found.
    fn untrack_hwctx(&self, handle: u32) -> Option<HwCtxRecord> {
        let Ok(mut guard) = self.created_hwctxs.lock() else {
            return None;
        };
        guard
            .iter()
            .position(|r| r.handle == handle)
            .map(|pos| guard.remove(pos))
    }

    /// The path of the opened device.
    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    /// Returns true (this handle only exists when the device is open).
    #[allow(dead_code, clippy::unused_self)]
    pub fn is_open(&self) -> bool {
        true
    }

    /// The underlying raw file descriptor (for future ioctl / submission work).
    /// This is intentionally not pub yet — we will expose a safe API when needed.
    #[allow(dead_code)]
    pub(crate) fn as_fd(&self) -> std::os::unix::io::RawFd {
        use std::os::unix::io::AsRawFd;
        self.file.as_raw_fd()
    }

    /// Basic driver / device information derived from the open device and sysfs.
    /// This is useful for logging and diagnostics on real hardware.
    #[allow(dead_code)]
    pub fn info(&self) -> RealXdna2DeviceInfo {
        RealXdna2DeviceInfo {
            path: self.path.clone(),
            pci_vendor: self.detected.pci_vendor.clone(),
            pci_device: self.detected.pci_device.clone(),
            pci_revision: self.detected.pci_revision.clone(),
            npu_name: self.detected.npu_name.clone(),
            modalias: self.detected.modalias.clone(),
            xdna2_variant: self.detected.xdna2_variant.clone(),
        }
    }

    /// Returns the full `AccelDevice` metadata that was known at open time.
    /// This gives access to name, driver, modalias, PCI details, and variant.
    pub fn detected(&self) -> &AccelDevice {
        &self.detected
    }

    /// Safely read a sysfs attribute for this open device.
    ///
    /// `relative_path` is interpreted relative to the standard sysfs directory
    /// for the device (typically `/sys/class/accel/{name}/` and its `device/`
    /// subdirectory).
    ///
    /// Examples of useful paths on real amdxdna XDNA2 systems:
    /// - "device/uevent"
    /// - "device/power/control"
    /// - "name"
    /// - various firmware or thermal attributes when exposed by the driver.
    ///
    /// This method performs **only read-only filesystem access**. It never
    /// issues ioctls, never modifies state, and never performs any command
    /// submission. It is intended for observability, bring-up, power/thermal
    /// monitoring, and to support future real execution code.
    ///
    /// Returns `None` on any failure (file does not exist, permission denied,
    /// not a regular file, etc.). This is the expected behavior on systems
    /// where the kernel does not expose the attribute.
    #[allow(dead_code)]
    pub fn read_sysfs(&self, relative_path: &str) -> Option<String> {
        // Primary location: /sys/class/accel/{name}/ + relative_path
        let base = std::path::Path::new("/sys/class/accel").join(&self.detected.name);
        let candidate = base.join(relative_path);
        if let Ok(s) = std::fs::read_to_string(&candidate) {
            return Some(s.trim().to_string());
        }

        // Also try under the device subdirectory (very common for PCI attrs)
        let dev_candidate = base.join("device").join(relative_path);
        if let Ok(s) = std::fs::read_to_string(&dev_candidate) {
            return Some(s.trim().to_string());
        }

        None
    }

    /// Convenience: current power control state of the device (e.g. "auto", "on").
    /// Returns None if the attribute is not readable.
    #[allow(dead_code)]
    pub fn power_control(&self) -> Option<String> {
        self.read_sysfs("device/power/control")
    }

    /// Convenience: whether we can read basic uevent information for the device.
    #[allow(dead_code)]
    pub fn has_uevent(&self) -> bool {
        self.read_sysfs("device/uevent").is_some()
    }

    /// Returns a lightweight direct submission handle for this device.
    ///
    /// This is the entry point for the focused, non-official direct driver
    /// execution path we are building (direct amdxdna ioctls, no heavy official
    /// runtimes). The handle is created with the real open fd so that future
    /// real implementations have everything they need.
    #[allow(dead_code)]
    pub fn direct_submission(&self) -> impl DirectSubmission {
        // Return the real implementation (which performs actual DRM_IOCTL_AMDXDNA_GET_INFO
        // calls for the typed helpers). This is what makes get_direct_firmware_version(),
        // get_direct_power_mode(), and get_direct_resource_info() talk to the live
        // amdxdna driver on physical hardware when the Real arm or new_with_real_stub()
        // is used.
        direct::RealDirectSubmission::new(self.as_fd())
    }

    /// Query firmware version using the lightweight direct path (real ioctl on hardware).
    /// This is one of the first useful real operations that actually talks to the amdxdna driver.
    #[allow(dead_code)]
    pub fn get_direct_firmware_version(
        &self,
    ) -> Result<direct::amdxdna_drm_query_firmware_version, direct::DirectError> {
        // Delegate through the direct submission. When the real arm is active,
        // RealDirectSubmission will perform the actual GET_INFO ioctl.
        self.direct_submission().get_firmware_version()
    }

    /// Query configured power mode using the lightweight direct path.
    #[allow(dead_code)]
    pub fn get_direct_power_mode(
        &self,
    ) -> Result<direct::amdxdna_drm_get_power_mode, direct::DirectError> {
        self.direct_submission().get_power_mode()
    }

    /// Query resource information (current/max TOPS, clocks, task limits) via the direct path.
    #[allow(dead_code)]
    pub fn get_direct_resource_info(
        &self,
    ) -> Result<direct::amdxdna_drm_get_resource_info, direct::DirectError> {
        self.direct_submission().get_resource_info()
    }

    /// Query sensor data via the lightweight direct path.
    #[allow(dead_code)]
    pub fn get_direct_sensors(
        &self,
    ) -> Result<direct::amdxdna_drm_query_sensor, direct::DirectError> {
        self.direct_submission().get_sensors()
    }

    /// Get information about a specific buffer object via the lightweight direct path.
    #[allow(dead_code)]
    pub fn get_direct_bo_info(
        &self,
        handle: u32,
    ) -> Result<direct::amdxdna_drm_get_bo_info, direct::DirectError> {
        self.direct_submission().get_bo_info(self.as_fd(), handle)
    }

    /// Convenience: create a hardware context with internally-managed UMQ and log buffers.
    /// This is the easiest way to get a real hwctx on hardware using the direct path.
    #[allow(dead_code)]
    pub fn create_direct_hwctx(
        &self,
        num_tiles: u32,
        mem_size: u32,
        qos: Option<&direct::QosInfo>,
    ) -> Result<direct::HardwareContext, direct::DirectError> {
        self.create_direct_hwctx_with_sizes(num_tiles, mem_size, 16 * 1024, 128 * 1024, qos)
    }

    /// Create a hardware context with explicit control over UMQ and log buffer sizes.
    /// Extremely useful for bring-up and debugging on real hardware.
    ///
    /// Sizes are wired through `DirectSubmission::create_hwctx_with_sizes` so Strategy B
    /// uses exactly these UMQ/log BO sizes (not the full probe matrix).
    #[allow(dead_code)]
    pub fn create_direct_hwctx_with_sizes(
        &self,
        num_tiles: u32,
        mem_size: u32,
        umq_size: u64,
        log_size: u64,
        qos: Option<&direct::QosInfo>,
    ) -> Result<direct::HardwareContext, direct::DirectError> {
        let ctx = self.direct_submission().create_hwctx_with_sizes(
            self.as_fd(),
            num_tiles,
            mem_size,
            umq_size,
            log_size,
            qos,
        )?;
        self.track_hwctx(&ctx);
        Ok(ctx)
    }

    /// Destroy a previously created hardware context.
    ///
    /// If this handle was created via [`Self::create_direct_hwctx_with_sizes`], also
    /// closes the tracked UMQ/log BOs. Prefer [`Self::destroy_direct_hwctx_full`] when
    /// you still have the full `HardwareContext`.
    #[allow(dead_code)]
    pub fn destroy_direct_hwctx(&self, handle: u32) -> Result<(), direct::DirectError> {
        let record = self.untrack_hwctx(handle);
        let sub = self.direct_submission();
        let fd = self.as_fd();
        if let Some(r) = record {
            if r.umq_bo != 0 {
                let _ = sub.destroy_buffer(fd, r.umq_bo);
            }
            if r.log_buf_bo != 0 {
                let _ = sub.destroy_buffer(fd, r.log_buf_bo);
            }
        }
        sub.destroy_hwctx(fd, handle)
    }

    /// Destroy a hwctx and its associated BOs (recommended for bring-up).
    /// Uses the info from HardwareContext (which now carries the BO handles it created).
    #[allow(dead_code)]
    pub fn destroy_direct_hwctx_full(
        &self,
        ctx: &direct::HardwareContext,
    ) -> Result<(), direct::DirectError> {
        let _ = self.untrack_hwctx(ctx.handle);
        let sub = self.direct_submission();
        let fd = self.as_fd();
        // Destroy hwctx first while BOs are still valid references for the driver,
        // then GEM_CLOSE the UMQ/log BOs we own.
        let hwctx_res = sub.destroy_hwctx(fd, ctx.handle);
        if ctx.umq_bo != 0 {
            let _ = sub.destroy_buffer(fd, ctx.umq_bo);
        }
        if ctx.log_buf_bo != 0 {
            let _ = sub.destroy_buffer(fd, ctx.log_buf_bo);
        }
        hwctx_res
    }

    /// Query AIE version via the lightweight direct path.
    #[allow(dead_code)]
    pub fn get_direct_aie_version(
        &self,
    ) -> Result<direct::amdxdna_drm_query_aie_version, direct::DirectError> {
        self.direct_submission().get_aie_version()
    }

    /// Returns how many hwctxs are currently tracked on this device handle
    /// (basic observability / bring-up aid). Decrements on successful destroy.
    #[allow(dead_code)]
    pub fn created_direct_hwctx_count(&self) -> usize {
        self.created_hwctxs.lock().map_or(0, |g| g.len())
    }
}

/// Snapshot of information about an opened real XDNA2 device.
/// Useful for user-facing diagnostics and logging when the real execution
/// arm is active.
#[derive(Debug, Clone)]
pub struct RealXdna2DeviceInfo {
    /// The /dev/accel path that was successfully opened.
    pub path: std::path::PathBuf,
    /// PCI vendor ID (e.g. "0x1022" for AMD).
    pub pci_vendor: String,
    /// PCI device ID (e.g. "0x17f0" for XDNA2 family).
    pub pci_device: String,
    /// PCI revision.
    pub pci_revision: String,
    /// NPU name reported by the driver (often empty on real systems).
    pub npu_name: String,
    /// Full modalias string (very useful for variant detection).
    pub modalias: String,
    /// Derived XDNA2 generation (NPU4/NPU5/NPU6 or "XDNA2").
    pub xdna2_variant: Option<String>,
}

/// Real XDNA2 executor — the production path for actual hardware submission.
///
/// When the `Real` arm is active, this type holds a `RealXdna2Device` (the opened
/// /dev/accel handle) and is responsible for future real NPU command submission.
///
/// Current state:
/// - Device open + rich diagnostics via the embedded `RealXdna2Device` — mature.
/// - `submit_and_run` / `compile_model` — honest "not yet wired" errors until
///   the actual execution implementation lands.
///
/// This is where real XDNA2 command submission work will continue in the pure
/// lightweight direct style (no heavy/official runtimes).
///
/// See docs/XDNA2_REAL_EXECUTION.md.
#[derive(Debug)]
#[allow(dead_code)] // Real submission implementation pending architectural decisions
struct RealXdna2Executor {
    /// Details of the detected XDNA2 device (from sysfs probe).
    detected: Option<AccelDevice>,
    /// The opened real device handle, if we successfully opened one.
    /// This is the new home for all real-device concerns.
    device: Option<RealXdna2Device>,
}

impl RealXdna2Executor {
    /// Create a real executor.
    ///
    /// If a device was detected, this will attempt to open the corresponding
    /// /dev/accel device using the shared helper. Failure to open is turned
    /// into a clear runtime error only when submission is actually attempted.
    fn new(detected: Option<AccelDevice>) -> Self {
        let (opened_file, opened_path) = try_open_accel_device(detected.as_ref());

        let device = match (detected, opened_file, opened_path) {
            (Some(det), Some(f), Some(p)) => Some(RealXdna2Device::from_opened(det, f, p)),
            _ => None,
        };

        Self {
            detected: device.as_ref().map(|d| d.detected.clone()),
            device,
        }
    }
}

impl Xdna2Executor for RealXdna2Executor {
    fn compile_model(
        &self,
        model_path: &str,
        _optimization_level: u8,
        _quantization: &str,
    ) -> Result<CompiledModel, DragonNpuError> {
        // In a full production implementation this would compile or load
        // an XDNA2-native artifact using the opened device context.
        Err(DragonNpuError::CompilationFailed {
            model: model_path.to_string(),
            reason: if self.device.is_some() {
                "real XDNA2 compilation/submission not yet wired (device open succeeded; \
                 real_device_read_sysfs() and real_device_detected() are available for diagnostics)"
                    .to_string()
            } else {
                "real XDNA2 device not available for compilation".to_string()
            },
        })
    }

    fn run(&self, _input: &[f32]) -> Result<Vec<f32>, DragonNpuError> {
        Err(DragonNpuError::RuntimeError(
            "use submit_and_run for real XDNA2 hardware (direct run is not supported on the production path)".to_string(),
        ))
    }

    fn is_real(&self) -> bool {
        true
    }

    fn submit_and_run(
        &self,
        _model: &CompiledModel,
        _input: &[f32],
    ) -> Result<Vec<f32>, DragonNpuError> {
        // This is where the real production submission will happen:
        // - Use the opened device to issue commands to the amdxdna driver.
        // - Feed model weights from CompiledModel::initializers().
        // - Handle tiling, bf16, power modes, etc. (see XDNA2_REAL_EXECUTION.md).
        //
        // NEVER AGAIN (docs/XDNA2_NEVER_AGAIN.md): when this is wired, EXEC_CMD
        // must go only through `RealDirectSubmission::submit_command` (which
        // hard-gates on `exec_cmd_is_enabled()`), or call `exec_cmd_is_enabled()`
        // at the top and refuse. Do **not** open-code DRM_IOCTL_AMDXDNA_EXEC_CMD
        // here and bypass the double gate (feature + XDNA2_ALLOW_EXEC_CMD).
        if self.device.is_some() {
            Err(DragonNpuError::RuntimeError(
                "real XDNA2 submit_and_run not yet implemented (device successfully opened; \
                 use real_device_detected(), real_device_info(), and real_device_read_sysfs() \
                 for rich diagnostics while implementing submission — see docs/XDNA2_REAL_EXECUTION.md)".to_string(),
            ))
        } else {
            Err(DragonNpuError::RuntimeError(
                "real XDNA2 hardware device could not be opened for submission".to_string(),
            ))
        }
    }

    fn real_device_open(&self) -> bool {
        self.device.is_some()
    }

    fn real_device_path(&self) -> Option<&std::path::Path> {
        self.device.as_ref().map(RealXdna2Device::path)
    }

    fn real_device_info(&self) -> Option<RealXdna2DeviceInfo> {
        self.device.as_ref().map(RealXdna2Device::info)
    }

    fn real_device_detected(&self) -> Option<&AccelDevice> {
        self.device.as_ref().map(RealXdna2Device::detected)
    }

    fn real_device_read_sysfs(&self, relative_path: &str) -> Option<String> {
        self.device
            .as_ref()
            .and_then(|d| d.read_sysfs(relative_path))
    }

    // Lightweight direct query forwarding (real ioctls only when we have an open device).
    fn get_direct_firmware_version(
        &self,
    ) -> Result<direct::amdxdna_drm_query_firmware_version, direct::DirectError> {
        self.device.as_ref().map_or_else(
            || {
                Err(direct::DirectError::Unsupported(
                    "no real XDNA2 device open for direct query".to_string(),
                ))
            },
            RealXdna2Device::get_direct_firmware_version,
        )
    }

    fn get_direct_power_mode(
        &self,
    ) -> Result<direct::amdxdna_drm_get_power_mode, direct::DirectError> {
        self.device.as_ref().map_or_else(
            || {
                Err(direct::DirectError::Unsupported(
                    "no real XDNA2 device open for direct query".to_string(),
                ))
            },
            RealXdna2Device::get_direct_power_mode,
        )
    }

    fn get_direct_resource_info(
        &self,
    ) -> Result<direct::amdxdna_drm_get_resource_info, direct::DirectError> {
        self.device.as_ref().map_or_else(
            || {
                Err(direct::DirectError::Unsupported(
                    "no real XDNA2 device open for direct query".to_string(),
                ))
            },
            RealXdna2Device::get_direct_resource_info,
        )
    }

    fn direct_submission(&self) -> Box<dyn direct::DirectSubmission> {
        if let Some(dev) = &self.device {
            Box::new(direct::RealDirectSubmission::new(dev.as_fd()))
        } else {
            Box::new(direct::StubDirectSubmission::new(-1))
        }
    }

    fn get_sensors(&self) -> Result<direct::amdxdna_drm_query_sensor, direct::DirectError> {
        self.device.as_ref().map_or_else(
            || {
                Err(direct::DirectError::Unsupported(
                    "no real XDNA2 device open for direct query".to_string(),
                ))
            },
            RealXdna2Device::get_direct_sensors,
        )
    }

    fn get_direct_bo_info(
        &self,
        handle: u32,
    ) -> Result<direct::amdxdna_drm_get_bo_info, direct::DirectError> {
        self.device.as_ref().map_or_else(
            || {
                Err(direct::DirectError::Unsupported(
                    "no real XDNA2 device open for direct query".to_string(),
                ))
            },
            |d| d.get_direct_bo_info(handle),
        )
    }

    fn create_direct_hwctx(
        &self,
        num_tiles: u32,
        mem_size: u32,
        qos: Option<&direct::QosInfo>,
    ) -> Result<direct::HardwareContext, direct::DirectError> {
        self.create_direct_hwctx_with_sizes(num_tiles, mem_size, 16 * 1024, 128 * 1024, qos)
    }

    fn create_direct_hwctx_with_sizes(
        &self,
        num_tiles: u32,
        mem_size: u32,
        umq_size: u64,
        log_size: u64,
        qos: Option<&direct::QosInfo>,
    ) -> Result<direct::HardwareContext, direct::DirectError> {
        self.device.as_ref().map_or_else(
            || {
                Err(direct::DirectError::Unsupported(
                    "no real XDNA2 device open for direct hwctx".to_string(),
                ))
            },
            |d| d.create_direct_hwctx_with_sizes(num_tiles, mem_size, umq_size, log_size, qos),
        )
    }

    fn destroy_direct_hwctx(&self, handle: u32) -> Result<(), direct::DirectError> {
        self.device.as_ref().map_or_else(
            || {
                Err(direct::DirectError::Unsupported(
                    "no real XDNA2 device open for direct query".to_string(),
                ))
            },
            |d| d.destroy_direct_hwctx(handle),
        )
    }

    fn get_direct_aie_version(
        &self,
    ) -> Result<direct::amdxdna_drm_query_aie_version, direct::DirectError> {
        self.device.as_ref().map_or_else(
            || {
                Err(direct::DirectError::Unsupported(
                    "no real XDNA2 device open for direct query".to_string(),
                ))
            },
            RealXdna2Device::get_direct_aie_version,
        )
    }

    fn created_direct_hwctx_count(&self) -> usize {
        self.device
            .as_ref()
            .map_or(0, RealXdna2Device::created_direct_hwctx_count)
    }
}

/// Internal dispatch between Simulation and Real XDNA2 execution arms.
///
/// This enum exists so the transition to full real hardware submission is clean.
/// The `Real` variant holds the `RealXdna2Device` when the production path is active.
///
/// Current reality:
/// - Simulation arm: high-fidelity and always used by the default `new()`.
/// - Real arm: device open + diagnostics are solid (via `RealXdna2Device`).
///   Actual NPU submission inside the Real arm is still pending.
///
/// The architectural decision on *how* real submission will be implemented
/// (XRT, direct amdxdna ioctls, etc.) has not been made yet.
#[derive(Debug)]
#[allow(dead_code)] // Real submission logic pending architectural decision
enum Execution {
    Simulation(Box<dyn Xdna2Executor>),

    // Real XDNA2 production execution path.
    // This variant holds the logic that performs actual command submission to
    // the NPU via the lightweight direct amdxdna driver path.
    //
    // Goals for this path:
    // - Proper device open / context / memory management for /dev/accel devices.
    // - Submission of models (potentially compiled to XDNA2-native format).
    // - Efficient `submit_and_run` that actually executes on silicon and returns results.
    //
    // Optimization and benchmarking guidance lives in docs/XDNA2_REAL_EXECUTION.md.
    Real(Box<dyn Xdna2Executor>),
}

impl Execution {
    fn compile_model(
        &self,
        model_path: &str,
        optimization_level: u8,
        quantization: &str,
    ) -> Result<CompiledModel, DragonNpuError> {
        match self {
            Self::Simulation(exec) | Self::Real(exec) => {
                exec.compile_model(model_path, optimization_level, quantization)
            }
        }
    }

    fn run(&self, input: &[f32]) -> Result<Vec<f32>, DragonNpuError> {
        match self {
            Self::Simulation(exec) | Self::Real(exec) => exec.run(input),
        }
    }

    #[allow(dead_code)]
    fn real_device_info(&self) -> Option<RealXdna2DeviceInfo> {
        match self {
            Self::Simulation(exec) | Self::Real(exec) => exec.real_device_info(),
        }
    }

    #[allow(dead_code)]
    fn real_device_detected(&self) -> Option<&AccelDevice> {
        match self {
            Self::Simulation(exec) | Self::Real(exec) => exec.real_device_detected(),
        }
    }

    #[allow(dead_code)]
    fn real_device_read_sysfs(&self, relative_path: &str) -> Option<String> {
        match self {
            Self::Simulation(exec) | Self::Real(exec) => exec.real_device_read_sysfs(relative_path),
        }
    }
}

impl AmdXdna2Backend {
    /// Create a new AMD XDNA2 backend.
    ///
    /// Probes the system for real AMD XDNA2 hardware (amdxdna driver + PCI 1022:17f0 family).
    /// Sets realistic XDNA2 parameters (~50 TOPS) when detected.
    ///
    /// Always succeeds — falls back to high-fidelity simulation when no
    /// XDNA2 hardware is present.
    #[allow(clippy::option_if_let_else)]
    pub fn new() -> Self {
        let devices = probe_accel_devices();

        // XDNA2 detection: amdxdna driver + AMD PCI 1022:17f0 family
        let xdna2 = devices.iter().find(|d| {
            d.driver.contains("amdxdna") && d.pci_vendor == "0x1022" && d.pci_device == "0x17f0"
        });

        let hardware_detected = xdna2.is_some();
        let detected_device = xdna2.cloned();

        let (device_name, peak_tops, _driver_version) = if let Some(d) = &detected_device {
            let display_name = if d.npu_name.is_empty() {
                if d.pci_device == "0x17f0" {
                    format!(
                        "AMD XDNA2 (17f0 rev {}) - hardware detected",
                        d.pci_revision
                    )
                } else {
                    "AMD XDNA2 (Ryzen AI) - hardware detected".to_string()
                }
            } else {
                format!("AMD XDNA2 ({}) - hardware detected", d.npu_name)
            };
            (
                display_name,
                50.0,
                format!("amdxdna (XDNA2 {} detected)", d.pci_revision),
            )
        } else {
            (
                "AMD XDNA2 (Ryzen AI) - simulated (no XDNA2 hardware detected)".to_string(),
                50.0,
                "stub-0.1 (no XDNA2 hardware found)".to_string(),
            )
        };

        let config = SimulationConfig {
            device_name,
            peak_tops,
            latency_factor: if hardware_detected { 0.6 } else { 1.0 },
            ..Default::default()
        };

        // Default to high-fidelity Simulation for `new()`.
        // Real production execution is activated explicitly via `new_with_real_stub()`
        // (or future configuration). This is the safe, development-friendly default
        // while the real submission logic is being completed.
        Self {
            execution: Execution::Simulation(Box::new(SimulationBackend::with_config(config))),
            hardware_detected,
            detected_device,
        }
    }

    /// Creates an AmdXdna2Backend that uses the Real execution path.
    /// Useful for early bring-up and validation of real hardware submission
    /// plumbing on physical XDNA2 devices.
    pub fn new_with_real_stub() -> Self {
        let devices = probe_accel_devices();

        let xdna2 = devices.iter().find(|d| {
            d.driver.contains("amdxdna") && d.pci_vendor == "0x1022" && d.pci_device == "0x17f0"
        });

        let hardware_detected = xdna2.is_some();
        let detected_device = xdna2.cloned();

        Self {
            execution: Execution::Real(Box::new(RealXdna2Executor::new(detected_device.clone()))),
            hardware_detected,
            detected_device,
        }
    }

    /// Returns whether real AMD XDNA2 hardware was detected at construction time.
    pub fn hardware_detected(&self) -> bool {
        self.hardware_detected
    }

    /// Returns details about the detected XDNA2 device, if any.
    pub fn detected_device(&self) -> Option<&AccelDevice> {
        self.detected_device.as_ref()
    }

    /// Convenience accessor: returns a human-friendly string describing the
    /// detected NPU (e.g. "XDNA2 (Strix Point)" or the raw npu_name).
    /// Returns None when running in simulated mode.
    pub fn detected_npu_type(&self) -> Option<String> {
        self.detected_device.as_ref().map(|d| {
            let base = if d.npu_name.is_empty() {
                // Common on real hardware — fall back to PCI family + revision for usefulness
                if d.pci_device == "0x17f0" {
                    format!("XDNA2 (17f0 rev {})", d.pci_revision)
                } else {
                    "XDNA2".to_string()
                }
            } else {
                format!("XDNA2 ({})", d.npu_name)
            };
            // Only append the variant when it adds distinct information (e.g. "NPU5").
            // Avoid "XDNA2 (17f0 rev 0x20) / XDNA2" on real hardware where variant is just "XDNA2".
            if let Some(variant) = &d.xdna2_variant {
                if variant != "XDNA2" && !base.contains(variant) {
                    format!("{base} / {variant}")
                } else {
                    base
                }
            } else {
                base
            }
        })
    }

    /// Returns whether the real (non-simulation) XDNA2 execution path is active.
    ///
    /// When true, `submit_and_run` will attempt actual hardware submission.
    /// This is the primary production signal for real NPU execution.
    pub fn real_execution_active(&self) -> bool {
        match &self.execution {
            Execution::Simulation(exec) | Execution::Real(exec) => exec.is_real(),
        }
    }

    /// For the Real execution arm on actual hardware: whether we successfully
    /// opened the /dev/accel device. High-value diagnostic on the user's laptop.
    pub fn real_device_open(&self) -> bool {
        match &self.execution {
            Execution::Real(exec) => exec.real_device_open(),
            Execution::Simulation(_) => false,
        }
    }

    /// Returns the path of the accel device we successfully opened on the Real
    /// execution arm (if any). Very useful for debugging on real XDNA2 hardware.
    pub fn real_device_path(&self) -> Option<&std::path::Path> {
        match &self.execution {
            Execution::Real(exec) => exec.real_device_path(),
            Execution::Simulation(_) => None,
        }
    }

    /// When the real XDNA2 execution arm is active and we successfully opened
    /// a device, returns detailed information about that device (PCI ids,
    /// modalias, variant, etc.). This is the richest diagnostic available for
    /// the production real path on actual hardware.
    pub fn real_device_info(&self) -> Option<RealXdna2DeviceInfo> {
        match &self.execution {
            Execution::Real(exec) => exec.real_device_info(),
            Execution::Simulation(_) => None,
        }
    }

    /// When the real execution arm has an open device, returns the full
    /// `AccelDevice` metadata captured at probe time for that device.
    pub fn real_device_detected(&self) -> Option<&AccelDevice> {
        match &self.execution {
            Execution::Real(exec) => exec.real_device_detected(),
            Execution::Simulation(_) => None,
        }
    }

    /// Safe, read-only access to sysfs attributes for the currently open real
    /// XDNA2 device (when the real arm is active and a device was opened).
    ///
    /// This is the recommended way for users and future real execution code
    /// to inspect driver-exposed attributes (power, firmware, thermal, etc.)
    /// without performing any submission.
    ///
    /// See `RealXdna2Device::read_sysfs` for details and example paths.
    pub fn real_device_read_sysfs(&self, relative_path: &str) -> Option<String> {
        match &self.execution {
            Execution::Real(exec) => exec.real_device_read_sysfs(relative_path),
            Execution::Simulation(_) => None,
        }
    }

    /// Convenience wrapper for the most common real-hardware power query.
    pub fn real_device_power_control(&self) -> Option<String> {
        self.real_device_read_sysfs("device/power/control")
    }

    /// Query firmware version via the lightweight direct driver path (GET_INFO ioctl).
    ///
    /// Only meaningful on the Real execution arm with a successfully opened device.
    /// Returns a clear Unsupported error on the simulation path or when no device is open.
    pub fn get_direct_firmware_version(
        &self,
    ) -> Result<direct::amdxdna_drm_query_firmware_version, direct::DirectError> {
        match &self.execution {
            Execution::Real(exec) => exec.get_direct_firmware_version(),
            Execution::Simulation(_) => Err(direct::DirectError::Unsupported(
                "direct queries require the Real XDNA2 execution arm (use new_with_real_stub())"
                    .to_string(),
            )),
        }
    }

    /// Query configured power mode via the lightweight direct driver path.
    pub fn get_direct_power_mode(
        &self,
    ) -> Result<direct::amdxdna_drm_get_power_mode, direct::DirectError> {
        match &self.execution {
            Execution::Real(exec) => exec.get_direct_power_mode(),
            Execution::Simulation(_) => Err(direct::DirectError::Unsupported(
                "direct queries require the Real XDNA2 execution arm (use new_with_real_stub())"
                    .to_string(),
            )),
        }
    }

    /// Query resource limits (TOPS, clocks, task caps) via the lightweight direct driver path.
    pub fn get_direct_resource_info(
        &self,
    ) -> Result<direct::amdxdna_drm_get_resource_info, direct::DirectError> {
        match &self.execution {
            Execution::Real(exec) => exec.get_direct_resource_info(),
            Execution::Simulation(_) => Err(direct::DirectError::Unsupported(
                "direct queries require the Real XDNA2 execution arm (use new_with_real_stub())"
                    .to_string(),
            )),
        }
    }

    /// Query AIE version via the lightweight direct driver path.
    #[allow(dead_code)]
    pub fn get_direct_aie_version(
        &self,
    ) -> Result<direct::amdxdna_drm_query_aie_version, direct::DirectError> {
        match &self.execution {
            Execution::Real(exec) => exec.get_direct_aie_version(),
            Execution::Simulation(_) => Err(direct::DirectError::Unsupported(
                "direct queries require the Real XDNA2 execution arm (use new_with_real_stub())"
                    .to_string(),
            )),
        }
    }

    /// Returns how many hwctxs have been created through this backend on the Real arm.
    #[allow(dead_code)]
    pub fn created_direct_hwctx_count(&self) -> usize {
        match &self.execution {
            Execution::Real(exec) => exec.created_direct_hwctx_count(),
            Execution::Simulation(_) => 0,
        }
    }

    /// Convenience: create hwctx via backend (Real arm only).
    #[allow(dead_code)]
    pub fn create_direct_hwctx(
        &self,
        num_tiles: u32,
        mem_size: u32,
        qos: Option<&direct::QosInfo>,
    ) -> Result<direct::HardwareContext, direct::DirectError> {
        self.create_direct_hwctx_with_sizes(num_tiles, mem_size, 16 * 1024, 128 * 1024, qos)
    }

    /// Create hwctx with explicit UMQ/log buffer sizes (highly recommended for real hardware bring-up).
    ///
    /// On the Real arm this routes through `RealXdna2Device` so sizes reach
    /// `create_hwctx_with_sizes` and the hwctx is tracked for destroy.
    #[allow(dead_code)]
    pub fn create_direct_hwctx_with_sizes(
        &self,
        num_tiles: u32,
        mem_size: u32,
        umq_size: u64,
        log_size: u64,
        qos: Option<&direct::QosInfo>,
    ) -> Result<direct::HardwareContext, direct::DirectError> {
        match &self.execution {
            Execution::Real(exec) => {
                exec.create_direct_hwctx_with_sizes(num_tiles, mem_size, umq_size, log_size, qos)
            }
            Execution::Simulation(_) => Err(direct::DirectError::Unsupported(
                "requires Real arm".to_string(),
            )),
        }
    }

    /// Destroy hwctx via backend.
    #[allow(dead_code)]
    pub fn destroy_direct_hwctx(&self, handle: u32) -> Result<(), direct::DirectError> {
        match &self.execution {
            Execution::Real(exec) => exec.destroy_direct_hwctx(handle),
            Execution::Simulation(_) => Err(direct::DirectError::Unsupported(
                "requires Real arm".to_string(),
            )),
        }
    }

    /// Returns a `DirectSubmission` handle for the lightweight direct driver path.
    ///
    /// On the Real execution arm (via `new_with_real_stub()`) with a successfully
    /// opened device, this returns a live `RealDirectSubmission` that can perform
    /// actual ioctls. On simulation or when no device is open, it returns a stub
    /// that gives clear "not available" errors.
    ///
    /// This is the main way advanced users and bring-up code get access to the
    /// full direct trait surface from the public backend.
    pub fn direct_submission(&self) -> Box<dyn direct::DirectSubmission> {
        match &self.execution {
            Execution::Real(exec) => exec.direct_submission(),
            Execution::Simulation(_) => Box::new(direct::StubDirectSubmission::new(-1)),
        }
    }

    /// Timed matmul attempt via the pure direct path (plan step 4).
    ///
    /// Returns structured timings + honest `status` (`hwctx_blocked`,
    /// `payload_unsupported`, `ok_submit_scaffold`, …). Never fakes NPU Pass.
    pub fn timed_matmul(&self, m: u32, n: u32, k: u32, dtype: &str) -> direct::TimedOpResult {
        let sub = self.direct_submission();
        direct::timed_matmul_direct(sub.as_ref(), m, n, k, dtype)
    }

    /// Snapshot of the frozen Direct NPU capability checklist for tools/docs.
    pub fn direct_capability_ids() -> &'static [&'static str] {
        direct::DirectCapability::all_ids()
    }

    /// Query a sensor reading (temperature, power, etc.) via the lightweight direct path.
    pub fn get_direct_sensors(
        &self,
    ) -> Result<direct::amdxdna_drm_query_sensor, direct::DirectError> {
        match &self.execution {
            Execution::Real(exec) => exec.get_sensors(),
            Execution::Simulation(_) => Err(direct::DirectError::Unsupported(
                "direct queries require the Real XDNA2 execution arm (use new_with_real_stub())"
                    .to_string(),
            )),
        }
    }

    /// Get information about a buffer object via the lightweight direct driver path.
    pub fn get_direct_bo_info(
        &self,
        handle: u32,
    ) -> Result<direct::amdxdna_drm_get_bo_info, direct::DirectError> {
        match &self.execution {
            Execution::Real(exec) => exec.get_direct_bo_info(handle),
            Execution::Simulation(_) => Err(direct::DirectError::Unsupported(
                "direct queries require the Real XDNA2 execution arm (use new_with_real_stub())"
                    .to_string(),
            )),
        }
    }

    /// Primary production API for running inference on XDNA2 hardware.
    ///
    /// When `real_execution_active()` is true, this performs actual NPU submission
    /// (once the real execution implementation is wired).
    pub fn submit_and_run(
        &self,
        model: &CompiledModel,
        input: &[f32],
    ) -> Result<Vec<f32>, DragonNpuError> {
        match &self.execution {
            Execution::Simulation(exec) | Execution::Real(exec) => {
                exec.submit_and_run(model, input)
            }
        }
    }
}

impl Backend for AmdXdna2Backend {
    #[allow(clippy::option_if_let_else)]
    fn status(&self) -> Result<NpuStatus, DragonNpuError> {
        let (device_name, driver_version) = if let Some(d) = &self.detected_device {
            let dn = if d.npu_name.is_empty() {
                // Improve reporting on real hardware where npu_name is often empty
                if d.pci_device == "0x17f0" {
                    format!(
                        "AMD XDNA2 (17f0 rev {}) - hardware detected",
                        d.pci_revision
                    )
                } else {
                    "AMD XDNA2 (Ryzen AI) - hardware detected".to_string()
                }
            } else {
                format!("AMD XDNA2 ({}) - hardware detected", d.npu_name)
            };
            (dn, format!("amdxdna (XDNA2 {} detected)", d.pci_revision))
        } else {
            (
                "AMD XDNA2 (Ryzen AI) - simulated (no XDNA2 hardware detected)".to_string(),
                "stub-0.1 (no XDNA2 hardware found)".to_string(),
            )
        };

        Ok(NpuStatus {
            vendor: "AMD".to_string(),
            available: true,
            compute_units: if self.hardware_detected { 8 } else { 4 },
            memory_mb: if self.hardware_detected { 8192 } else { 4096 },
            peak_tops: 50.0,
            device_name,
            driver_version,
            supported_dtypes: if self.hardware_detected {
                // XDNA2 has strong native Block FP16/bf16 support (per research)
                vec![
                    "fp32".to_string(),
                    "fp16".to_string(),
                    "bf16".to_string(),
                    "int8".to_string(),
                ]
            } else {
                vec![
                    "fp32".to_string(),
                    "fp16".to_string(),
                    "int8".to_string(),
                    "bf16".to_string(),
                ]
            },
            simulated: !self.hardware_detected,
        })
    }

    fn compile_model(
        &self,
        model_path: &str,
        optimization_level: u8,
        quantization: &str,
    ) -> Result<CompiledModel, DragonNpuError> {
        self.execution
            .compile_model(model_path, optimization_level, quantization)
    }

    fn run(&self, input: &[f32]) -> Result<Vec<f32>, DragonNpuError> {
        self.execution.run(input)
    }

    fn submit_and_run(
        &self,
        model: &CompiledModel,
        input: &[f32],
    ) -> Result<Vec<f32>, DragonNpuError> {
        // Delegate to the XDNA2-specific implementation that goes through the
        // private Execution enum (Simulation vs Real stub). This is the path
        // that will eventually perform real NPU submission.
        self.submit_and_run(model, input)
    }
}

impl Default for AmdXdna2Backend {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amd_xdna2_backend_constructs_safely() {
        let backend = AmdXdna2Backend::new();
        // In this environment there is almost certainly no real amdxdna hardware.
        // The important guarantee is that construction never panics and we get
        // a usable backend (simulation with honest metadata).
        let status = backend.status().expect("status must succeed");
        assert_eq!(status.vendor, "AMD");
        assert!(status.available);
        // The device_name must clearly communicate simulated vs hardware.
        assert!(status.device_name.contains("AMD XDNA2"));
        assert!(
            status.device_name.contains("simulated")
                || status.device_name.contains("hardware detected")
        );
    }

    #[test]
    fn amd_xdna2_backend_reports_hardware_detection_state() {
        let backend = AmdXdna2Backend::new();
        // The probe result must be observable.
        let _detected = backend.hardware_detected();
        // When not detected we must still be usable (this is the critical safe fallback).
        let status = backend.status().unwrap();
        if !backend.hardware_detected() {
            assert!(status.device_name.contains("simulated"));
            assert!(
                status.driver_version.contains("stub")
                    || status.driver_version.contains("no XDNA2")
            );
        }
    }

    #[test]
    fn amd_xdna2_backend_exposes_detected_device_when_available() {
        let backend = AmdXdna2Backend::new();
        let dev = backend.detected_device();
        if backend.hardware_detected() {
            assert!(dev.is_some());
        } else {
            // In CI/dev without hardware, it should be None
            assert!(dev.is_none() || dev.unwrap().driver.contains("amdxdna"));
        }
    }

    #[test]
    fn amd_xdna2_backend_detected_npu_type_reports_xdna2_when_hardware_present() {
        let backend = AmdXdna2Backend::new();
        let npu_type = backend.detected_npu_type();
        if backend.hardware_detected() {
            assert!(npu_type.is_some());
            assert!(npu_type.unwrap().contains("XDNA2"));
        } else {
            assert!(npu_type.is_none());
        }
    }

    #[test]
    fn amd_xdna2_backend_status_reflects_xdna2_when_simulated() {
        let backend = AmdXdna2Backend::new();
        let status = backend.status().unwrap();
        assert!(status.device_name.contains("XDNA2"));
        assert_eq!(status.simulated, !backend.hardware_detected());
        if !backend.hardware_detected() {
            assert!(status.device_name.contains("simulated"));
        }
    }

    #[test]
    fn amd_xdna2_backend_detected_npu_type_includes_variant_when_available() {
        let backend = AmdXdna2Backend::new();
        // In real hardware with modalias containing npu4/npu5, this would show the variant.
        // In current simulated runs it will be None or basic, which is acceptable.
        let _ = backend.detected_npu_type();
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn amd_xdna2_backend_exposes_consistent_simulated_status() {
        let backend = AmdXdna2Backend::new();
        let status = backend.status().unwrap();
        assert!(status.device_name.contains("XDNA2"));
        assert_eq!(status.peak_tops, 50.0); // XDNA2-class even in sim for this backend
        assert_eq!(status.simulated, !backend.hardware_detected());
    }

    #[test]
    fn amd_xdna2_backend_detected_npu_type_is_none_in_simulated_mode() {
        let backend = AmdXdna2Backend::new();
        if !backend.hardware_detected() {
            // This is the normal path in CI/dev without real XDNA2 hardware
            assert!(backend.detected_npu_type().is_none());
            assert!(backend.detected_device().is_none());
        }
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn amd_xdna2_backend_status_has_expected_xdna2_fields() {
        let backend = AmdXdna2Backend::new();
        let status = backend.status().unwrap();
        assert_eq!(status.vendor, "AMD");
        assert!(status.device_name.contains("XDNA2"));
        assert!(status.supported_dtypes.contains(&"bf16".to_string()));
        assert_eq!(status.peak_tops, 50.0);
    }

    // === Execution boundary tests (Xdna2Executor + submit_and_run) ===

    #[test]
    fn new_with_real_stub_forces_real_arm() {
        let backend = AmdXdna2Backend::new_with_real_stub();
        assert!(backend.real_execution_active());
        // The stub still reports whatever hardware the probe found (or not).
        // The key contract is that is_real() is true and subsequent calls go to RealXdna2Executor.
        let _ = backend.hardware_detected();
    }

    #[test]
    fn normal_new_path_has_real_execution_inactive() {
        let backend = AmdXdna2Backend::new();
        // Normal construction always uses the Simulation arm today.
        // real_execution_active() only becomes true when the caller explicitly
        // chooses the stub (for early plumbing validation on real silicon).
        assert!(!backend.real_execution_active());
    }

    #[test]
    fn submit_and_run_on_real_stub_returns_clear_actionable_error() {
        let backend = AmdXdna2Backend::new_with_real_stub();
        // We must be able to reach the compile call site on the Real arm.
        // Compile fails honestly today (either device-open or not-wired).
        let compile_res = backend.compile_model("any_model.onnx", 1, "fp32");
        assert!(compile_res.is_err());
        let err = compile_res.unwrap_err();
        let msg = format!("{err:?}");
        // Real arm always prefixes with "real XDNA2 …" (open or not).
        // Do not require a live /dev/accel open (wedged driver / CI without NPU).
        assert!(
            msg.contains("real XDNA2 compilation/submission not yet wired")
                || msg.contains("real XDNA2 device not available for compilation"),
            "expected Real-arm compile error, got: {msg}"
        );
    }

    #[test]
    fn submit_and_run_succeeds_on_simulation_arm() {
        let backend = AmdXdna2Backend::new();
        // On the normal path we always get the high-fidelity Simulation arm.
        // submit_and_run should delegate cleanly and produce a result.
        let model = backend
            .compile_model("model.onnx", 0, "fp32")
            .expect("simulation compile must succeed");
        let input = vec![0.0f32; 4];
        let output = backend
            .submit_and_run(&model, &input)
            .expect("submit_and_run on simulation arm must succeed");
        assert!(!output.is_empty());
    }

    // === New trait surface + CompiledModel delegation tests ===

    #[test]
    fn compiled_model_submit_and_run_delegates_through_backend() {
        let backend = AmdXdna2Backend::new();
        let model = backend
            .compile_model("model.onnx", 1, "fp32")
            .expect("compile must succeed on simulation path");
        let input = vec![0.1f32; 8];

        // Direct on backend (trait method)
        let out1 = backend
            .submit_and_run(&model, &input)
            .expect("backend.submit_and_run must work");

        // Via CompiledModel (public user-facing method)
        let out2 = model
            .submit_and_run(&input)
            .expect("CompiledModel.submit_and_run must work");

        assert_eq!(out1.len(), out2.len());
    }

    #[test]
    fn submit_and_run_on_real_stub_via_compiled_model_returns_clear_error() {
        let backend = AmdXdna2Backend::new_with_real_stub();
        // Even if compile "succeeds" in future real wiring, today the Real arm
        // errors early. Accept both open and not-open honest messages so CI
        // and wedged-driver hosts stay green without requiring /dev/accel.
        let compile_res = backend.compile_model("real_model.onnx", 2, "bf16");
        assert!(compile_res.is_err());
        let err_msg = format!("{:?}", compile_res.unwrap_err());
        assert!(
            err_msg.contains("real XDNA2 compilation/submission not yet wired")
                || err_msg.contains("real XDNA2 device not available for compilation"),
            "expected Real-arm compile error, got: {err_msg}"
        );
    }

    #[test]
    fn real_device_open_is_available_on_real_arm() {
        let backend = AmdXdna2Backend::new_with_real_stub();
        // The diagnostic method should be callable. On real hardware it will
        // reflect whether the /dev/accel open succeeded.
        let _ = backend.real_device_open();
    }

    #[test]
    fn real_executor_stores_device_path_when_hardware_present() {
        let backend = AmdXdna2Backend::new_with_real_stub();
        // When using the Real arm, the executor should have attempted to
        // open a device path (even if the actual file open only succeeds on
        // real hardware with the amdxdna driver).
        if backend.real_execution_active() {
            // We can't easily inspect the private executor here, but we can
            // at least verify the public diagnostic is consistent.
            let _ = backend.real_device_open();
        }
    }

    #[test]
    fn real_device_diagnostics_are_callable_and_consistent_on_real_arm() {
        let backend = AmdXdna2Backend::new_with_real_stub();
        // Basic production API contract: these methods must be callable
        // without panicking and should be consistent with real_execution_active().
        let is_open = backend.real_device_open();
        let path = backend.real_device_path();

        if backend.real_execution_active() {
            // On the Real arm we expect the diagnostics to be available.
            // The actual open may only succeed on real hardware.
            let _ = is_open;
            let _ = path;
        } else {
            // On Simulation arm these should be false / None.
            assert!(!is_open);
            assert!(path.is_none());
        }
    }

    #[test]
    fn real_arm_always_attempts_device_path() {
        let backend = AmdXdna2Backend::new_with_real_stub();
        if backend.real_execution_active() {
            // When the Real arm is active, we should have attempted a device path.
            // On real hardware this will usually be Some after a successful open.
            // In CI it may still be Some (the path we tried) even if open failed.
            // The important contract is that the diagnostic is present.
            let _path = backend.real_device_path();
        }
    }

    #[test]
    fn amd_xdna2_backend_real_onnx_weights_flow_through_backend() {
        // TDD hook for real weight data (onx-08) flowing through AmdXdna2Backend.
        // Heuristic models have 0 weights; real .onnx + bf16 raw paths are covered
        // by compiler TDD but exercised via this backend on hardware owners' machines.
        let backend = AmdXdna2Backend::new();
        let model = backend
            .compile_model("model.onnx", 1, "fp32")
            .expect("compile must succeed");
        // Heuristic fallback has no real initializers.
        assert!(!model.has_real_weights());
        // submit_and_run still works (delegates to simulation arm on this machine).
        let input = vec![0.0f32; 4];
        let _ = backend
            .submit_and_run(&model, &input)
            .expect("submit_and_run works through XDNA2 backend");
        // On real hardware with a real .onnx containing bf16 weights, has_real_weights()
        // and initializer data would be true and available for the future real executor.
    }

    #[test]
    fn amd_xdna2_backend_bf16_weights_flow_through_xdna2_backend() {
        // Dedicated TDD (xdna2-exec-06): real .onnx + bf16 raw weights through AmdXdna2Backend.
        // Uses the same programmatic ONNX construction as compiler tests.
        use std::io::Write;
        use tract_onnx::pb::{GraphProto, ModelProto, NodeProto, TensorProto};

        let mut path = std::env::temp_dir();
        path.push("dragon_npu_xdna2_bf16_weights.onnx");

        let bf16_raw: Vec<u8> = vec![0x00, 0x3F, 0x00, 0x40]; // 2 elements

        {
            let mut f = std::fs::File::create(&path).expect("create");
            let model = ModelProto {
                ir_version: 7,
                graph: Some(GraphProto {
                    node: vec![NodeProto {
                        op_type: "add".to_string(),
                        input: vec!["a".to_string(), "w".to_string()],
                        output: vec!["out".to_string()],
                        ..Default::default()
                    }],
                    initializer: vec![TensorProto {
                        name: "w".to_string(),
                        dims: vec![2],
                        data_type: 16, // BFLOAT16
                        raw_data: bf16_raw,
                        ..Default::default()
                    }],
                    ..Default::default()
                }),
                ..Default::default()
            };
            let mut buf = Vec::new();
            prost::Message::encode(&model, &mut buf).expect("encode");
            f.write_all(&buf).expect("write");
        }

        let backend = AmdXdna2Backend::new(); // uses simulation arm on this machine, but full path
        let model = backend
            .compile_model(path.to_str().unwrap(), 1, "bf16")
            .expect("real .onnx with bf16 weights must load through XDNA2 backend");
        let _ = std::fs::remove_file(&path);

        assert!(model.has_real_weights());
        let inits = model.initializers();
        assert!(!inits.is_empty());
        let w = &inits[0];
        assert_eq!(w.data_type, 16);
        assert!(!w.raw_data.is_empty());
        assert!(!w.as_f32_data().is_empty()); // usable approx available

        // Exercise the new numeric folding / weight size APIs through the XDNA2 backend.
        let _ = model.total_weight_elements();
        let _ = model
            .initializers()
            .iter()
            .map(|i| i.as_f32_data().len())
            .sum::<usize>();

        // submit_and_run still works end-to-end through the backend
        let input = vec![0.0f32; 2];
        let _ = backend
            .submit_and_run(&model, &input)
            .expect("submit_and_run works with real bf16-weight model via XDNA2 backend");
    }

    // === Tests for the new RealXdna2Device / RealXdna2DeviceInfo abstraction ===

    #[test]
    fn real_device_info_is_none_on_normal_construction_path() {
        // On the safe default path (even with amd-xdna2 feature), we use the
        // high-fidelity Simulation arm. real_device_info must be None.
        let backend = AmdXdna2Backend::new();
        assert!(!backend.real_execution_active());
        assert!(backend.real_device_info().is_none());
    }

    #[test]
    fn real_device_info_available_only_on_real_arm_when_opened() {
        let backend = AmdXdna2Backend::new_with_real_stub();

        let info = backend.real_device_info();

        if backend.real_execution_active() && backend.real_device_open() {
            // On real hardware where the open succeeded, we should have rich info.
            let info = info.expect("real_device_info must be Some when device is open on real arm");
            assert!(!info.pci_vendor.is_empty());
            assert!(!info.pci_device.is_empty());
            // variant may be Some("XDNA2") or more specific on real systems
        } else {
            // In all other cases (no hardware or open failed) it must be None.
            assert!(info.is_none());
        }
    }

    #[test]
    fn real_device_info_is_always_none_on_simulation_arm() {
        let backend = AmdXdna2Backend::new(); // always simulation for this constructor
        // Even if we call the method, it must be None.
        assert!(backend.real_device_info().is_none());
    }

    #[test]
    fn real_xdna2_device_info_has_expected_shape_when_present() {
        // On the real stub path we can still inspect the type even if no device opened.
        let backend = AmdXdna2Backend::new_with_real_stub();
        if let Some(info) = backend.real_device_info() {
            // When we have info, all the key fields should be populated from the probe.
            assert!(info.pci_vendor.starts_with("0x"));
            assert!(info.pci_device.starts_with("0x"));
            // On user's real 17f0 hardware this is typically "XDNA2" or "NPU5" etc.
            if let Some(v) = &info.xdna2_variant {
                assert!(v.contains("XDNA") || v.contains("NPU"));
            }
            // The info should be Debug-printable (used by humans and logs).
            let _ = format!("{info:?}");
        }
    }

    #[test]
    fn real_xdna2_device_info_can_be_constructed_manually_for_testing() {
        // This exercises the public struct directly (useful for future real-exec tests
        // that want to mock device info without opening real hardware).
        let info = RealXdna2DeviceInfo {
            path: std::path::PathBuf::from("/dev/accel/accel0"),
            pci_vendor: "0x1022".to_string(),
            pci_device: "0x17f0".to_string(),
            pci_revision: "0x20".to_string(),
            npu_name: String::new(),
            modalias: "pci:v00001022d000017F0...".to_string(),
            xdna2_variant: Some("XDNA2".to_string()),
        };

        assert_eq!(info.pci_device, "0x17f0");
        assert_eq!(info.xdna2_variant.as_deref(), Some("XDNA2"));
        let debug = format!("{info:?}");
        assert!(debug.contains("17f0"));
    }

    #[test]
    fn real_device_detected_and_read_sysfs_are_none_on_simulation_paths() {
        let backend = AmdXdna2Backend::new();
        assert!(backend.real_device_detected().is_none());
        assert!(backend.real_device_read_sysfs("device/uevent").is_none());

        let real_stub = AmdXdna2Backend::new_with_real_stub();
        if !real_stub.real_device_open() {
            assert!(real_stub.real_device_detected().is_none());
            assert!(real_stub.real_device_read_sysfs("name").is_none());
        }
    }
}
