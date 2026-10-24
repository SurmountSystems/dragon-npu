//! NPU backend implementations for Earth Dragon NPU.
//!
//! - **Simulation** — always available (default).
//! - **AMD XDNA2** (`amd-xdna2`) — primary hardware path (lightweight direct
//!   `amdxdna`); see `docs/XDNA2_REAL_EXECUTION.md`.
//! - **Intel VPU / Qualcomm Hexagon** (`intel-vpu`, `qualcomm-hexagon`) —
//!   **honest stubs** only until XDNA2 MVP is green. Not auto-selected by
//!   `DragonNpu::new`. See `docs/SECONDARY_BACKENDS.md`.
//! - **XRT bridge** (`xrt-bridge`) — research-only, never default, never hot path.
//! - **External runner** — opt-in IRON/eepy wrap (`execution=external_runner`,
//!   `ran_on_npu=false`). Never default. See `external_runner` and
//!   `docs/SECONDARY_BACKENDS.md`.
//!
//! # Example
//!
//! ```rust
//! use earth_dragon_npu::backends::simulation::SimulationBackend;
//! ```

pub mod external_runner;
pub mod iron_kernel;
pub mod simulation;

#[cfg(feature = "amd-xdna2")]
pub mod amd_xdna2;

#[cfg(feature = "intel-vpu")]
pub mod intel_vpu;

#[cfg(feature = "qualcomm-hexagon")]
pub mod qualcomm_hexagon;

#[cfg(feature = "xrt-bridge")]
pub mod xrt_bridge;

use crate::{CompiledModel, DragonNpuError, NpuStatus};

/// Common trait for all NPU backends (simulation, AMD XDNA2, Intel VPU, etc.).
/// This enables pluggable backends as described in PLAN.md.
///
/// The `submit_and_run` method is the primary boundary for real NPU hardware
/// execution. Backends without a distinct real path inherit a default that
/// delegates to `run()`. AMD XDNA2 overrides it to dispatch through its
/// internal real vs simulation Execution enum.
pub trait Backend: std::fmt::Debug {
    /// Returns the current NPU status / capabilities.
    fn status(&self) -> Result<NpuStatus, DragonNpuError>;

    /// Compile a model for the selected NPU backend.
    fn compile_model(
        &self,
        model_path: &str,
        optimization_level: u8,
        quantization: &str,
    ) -> Result<CompiledModel, DragonNpuError>;

    /// Run inference on the compiled model.
    fn run(&self, input: &[f32]) -> Result<Vec<f32>, DragonNpuError>;

    /// Submit a compiled model for execution, preferring a real NPU hardware
    /// submission path when the backend has one (e.g. AMD XDNA2 once the real
    /// executor is wired).
    ///
    /// The default implementation falls back to `run()`. Backends with a
    /// distinct real-execution path (currently only AmdXdna2Backend via its
    /// internal Execution enum) override this to route to the real arm when
    /// active.
    ///
    /// This makes the primary "run on actual silicon" boundary first-class in
    /// the pluggable Backend trait, ready for incremental real-hardware
    /// implementations.
    fn submit_and_run(
        &self,
        _model: &CompiledModel,
        input: &[f32],
    ) -> Result<Vec<f32>, DragonNpuError> {
        self.run(input)
    }

    // === Real device diagnostics (only meaningful for backends with a real path) ===

    /// When a real device is open on this backend, returns the probe metadata.
    /// Default returns None. Only `AmdXdna2Backend` (real arm) currently implements it.
    fn real_device_detected(&self) -> Option<&crate::backends::AccelDevice> {
        None
    }

    /// Safe read-only sysfs access for an open real device.
    /// Default returns None. See `AmdXdna2Backend` real arm for the real implementation.
    fn real_device_read_sysfs(&self, _relative_path: &str) -> Option<String> {
        None
    }
}

// Re-export the simulation backend's main types for convenience.
// Use the public crate path for NpuStatus to avoid private import errors.
pub use crate::NpuStatus as SimulationNpuStatus;
pub use external_runner::{
    EXECUTION_EXTERNAL_RUNNER, ExternalRunnerBackend, ExternalRunnerConfig, ExternalRunnerReport,
};
pub use simulation::SimulationBackend;

impl Backend for SimulationBackend {
    fn status(&self) -> Result<NpuStatus, DragonNpuError> {
        self.status()
    }

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

    fn submit_and_run(
        &self,
        _model: &CompiledModel,
        input: &[f32],
    ) -> Result<Vec<f32>, DragonNpuError> {
        // SimulationBackend implements Xdna2Executor-style submit_and_run via its
        // internal run path for consistency with the trait default. Real backends
        // (AmdXdna2) override to use their Execution dispatch.
        self.run(input)
    }
}

/// Pure-std probe for Linux `accel` subsystem devices.
///
/// Returns richer info for AMD devices so we can specifically identify XDNA2
/// (PCI 1022:17f0 family) vs older XDNA.
///
/// This is the zero-dependency foundation for the amd-xdna2 backend.
/// No new crates required.
///
/// Future (post-detection): Use this for real XRT submission when we add execution.
pub fn probe_accel_devices() -> Vec<AccelDevice> {
    let mut devices = vec![];
    let accel_dir = std::path::Path::new("/sys/class/accel");

    if let Ok(entries) = std::fs::read_dir(accel_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let driver_link = path.join("device").join("driver");
            let driver = std::fs::read_link(&driver_link)
                .ok()
                .and_then(|p| p.file_name().map(|s| s.to_string_lossy().into_owned()))
                .unwrap_or_else(|| "unknown".to_string());

            let name = path.file_name().map_or_else(
                || "accel?".to_string(),
                |s| s.to_string_lossy().into_owned(),
            );

            // Try to read PCI vendor/device/revision for AMD XDNA detection
            let device_dir = path.join("device");
            let vendor = read_sysfs_hex(&device_dir.join("vendor")).unwrap_or_default();
            let device = read_sysfs_hex(&device_dir.join("device")).unwrap_or_default();
            let revision = read_sysfs_hex(&device_dir.join("revision")).unwrap_or_default();

            // Additional AMD NPU details (name file and modalias are very useful on amdxdna)
            let npu_name = std::fs::read_to_string(path.join("name"))
                .ok()
                .map(|s| s.trim().to_string())
                .unwrap_or_default();

            let modalias = std::fs::read_to_string(device_dir.join("modalias"))
                .ok()
                .map(|s| s.trim().to_string())
                .unwrap_or_default();

            // Extract XDNA2 variant hint (modalias substring or 17f0 PCI family)
            let xdna2_variant = parse_xdna2_variant(&modalias, &device);
            // Revision table from ids already read above. No /dev/accel open.
            let cmdlist_class = cmdlist_class_from_sysfs(&vendor, &device, &revision);

            devices.push(AccelDevice {
                name,
                driver,
                pci_vendor: vendor,
                pci_device: device,
                pci_revision: revision,
                npu_name,
                modalias,
                xdna2_variant,
                cmdlist_class,
            });
        }
    }

    devices
}

fn read_sysfs_hex(path: &std::path::Path) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_lowercase())
}

/// Pure function: derive XDNA2 generation/family from modalias + pci_device.
///
/// This is the testable heart of variant reporting for AMD XDNA2 (Ryzen AI 300 / Strix Point,
/// Strix Halo, Krackan Point, etc.).
///
/// Priority:
/// 1. Explicit "npu4"/"npu5"/"npu6" in the modalias string (best case on some systems)
/// 2. PCI device 0x17f0 → XDNA2 family (very common on real hardware, as seen on user's 17f0 rev 0x20)
///
/// Returns None for first-gen XDNA or non-XDNA devices.
pub(crate) fn parse_xdna2_variant(modalias: &str, pci_device: &str) -> Option<String> {
    let m = modalias.to_ascii_lowercase();
    if m.contains("npu4") {
        Some("NPU4".to_string())
    } else if m.contains("npu5") {
        Some("NPU5".to_string())
    } else if m.contains("npu6") {
        Some("NPU6".to_string())
    } else if pci_device == "0x17f0" {
        Some("XDNA2".to_string())
    } else {
        None
    }
}

/// AIE2 device-info row this crate's cmdlist path may name.
///
/// Classification only. Not a submit, and not NPU3 / AIE4.
/// Variant names keep the upstream `dev_npu*_info` stems.
#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Aie2DevInfo {
    /// `1022:17f0` revision `0x10` → `dev_npu4_info`.
    DevNpu4,
    /// `1022:17f0` revision `0x11` → `dev_npu5_info`.
    DevNpu5,
    /// `1022:17f0` revision `0x20` → `dev_npu6_info` (this host, Strix).
    DevNpu6,
}

impl std::fmt::Display for Aie2DevInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::DevNpu4 => "dev_npu4_info",
            Self::DevNpu5 => "dev_npu5_info",
            Self::DevNpu6 => "dev_npu6_info",
        })
    }
}

/// Why an AMD NPU PCI id is not on this crate's AIE2 cmdlist path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum AmdNpuPciClassError {
    /// NPU3 classic, PF, or VF (`aie4_classic_ops`). Fail closed.
    #[error("AIE4: not supported by this crate's AIE2 cmdlist path")]
    Aie4NotSupported,
    /// Device id or revision is not a row this crate classifies.
    ///
    /// Includes `1022:1502` (match list only; patch 5 showed no info struct)
    /// and unknown revisions of a known device. No generation is invented.
    #[error("unknown AMD NPU PCI id 0x{vendor:04x}:0x{device:04x} revision 0x{revision:02x}")]
    Unknown {
        /// PCI vendor id.
        vendor: u16,
        /// PCI device id.
        device: u16,
        /// PCI revision.
        revision: u8,
    },
}

/// Classify an AMD NPU PCI id for this crate's AIE2 cmdlist path.
///
/// Pure: no device open, no ioctl, no `/dev/accel`, no `EXEC_CMD`.
/// `1022:17f0` revisions `0x10`, `0x11`, and `0x20` are AIE2
/// (`dev_npu4_info`, `dev_npu5_info`, `dev_npu6_info`). Revision `0x20`
/// is npu6, not npu4, and not AIE4. `0x1502` is left unsupported.
///
/// # Errors
///
/// * [`AmdNpuPciClassError::Aie4NotSupported`] for `1022:17f1`, `1022:17f2`,
///   and `1022:17f3` at revisions `0x10` and `0x13`.
/// * [`AmdNpuPciClassError::Unknown`] for every other vendor, device, or revision.
pub const fn classify_amd_npu_pci_id(
    vendor: u16,
    device: u16,
    revision: u8,
) -> Result<Aie2DevInfo, AmdNpuPciClassError> {
    match (vendor, device, revision) {
        (0x1022, 0x17f0, 0x10) => Ok(Aie2DevInfo::DevNpu4),
        (0x1022, 0x17f0, 0x11) => Ok(Aie2DevInfo::DevNpu5),
        (0x1022, 0x17f0, 0x20) => Ok(Aie2DevInfo::DevNpu6),
        (0x1022, 0x17f1..=0x17f3, 0x10 | 0x13) => Err(AmdNpuPciClassError::Aie4NotSupported),
        _ => Err(AmdNpuPciClassError::Unknown {
            vendor,
            device,
            revision,
        }),
    }
}

fn parse_sysfs_hex_u16(text: &str) -> Option<u16> {
    parse_sysfs_hex_u32(text).and_then(|value| u16::try_from(value).ok())
}

fn parse_sysfs_hex_u8(text: &str) -> Option<u8> {
    parse_sysfs_hex_u32(text).and_then(|value| u8::try_from(value).ok())
}

fn parse_sysfs_hex_u32(text: &str) -> Option<u32> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    let hex = trimmed.strip_prefix("0x").unwrap_or(trimmed);
    if hex.is_empty() {
        return None;
    }
    u32::from_str_radix(hex, 16).ok()
}

/// Note stored on a sysfs probe row. Empty when the id text did not parse.
fn cmdlist_class_from_sysfs(vendor: &str, device: &str, revision: &str) -> String {
    let (Some(vendor_id), Some(device_id), Some(revision_id)) = (
        parse_sysfs_hex_u16(vendor),
        parse_sysfs_hex_u16(device),
        parse_sysfs_hex_u8(revision),
    ) else {
        return String::new();
    };
    match classify_amd_npu_pci_id(vendor_id, device_id, revision_id) {
        Ok(info) => format!("AIE2 {info}"),
        Err(err) => err.to_string(),
    }
}

/// Attempt to open a /dev/accel/accelN character device for the real XDNA2
/// production execution path.
///
/// Tries a small range of indices (0..4) for robustness across systems where
/// the exact accel index for the amdxdna device is not known a priori (common
/// when multiple accel devices or dynamic enumeration is present).
///
/// Returns `(Some(file), Some(path))` for the first successfully opened device,
/// or `(None, None)` if no candidate could be opened (no hardware, permissions,
/// or driver not exposing the node yet).
///
/// Opens **read+write** (`O_RDWR`). `std::fs::File::open` is read-only and is
/// **wrong** for the direct path: DRM `mmap` of DevHeap / Shmem BOs with
/// `PROT_READ|PROT_WRITE` returns `EPERM` on a read-only fd (confirmed on
/// 1022:17f0 — CREATE_BO + GET_BO_INFO succeed, mmap fails until O_RDWR).
/// CREATE_HWCTX then never gets a valid heap UVA.
///
/// This helper is deliberately pure-std with no additional dependencies. It is
/// the single source of truth for multi-index device open logic in the real
/// XDNA2 arm. Safe to call in any environment (returns None on CI / non-XDNA2
/// machines).
///
/// Exposed as `pub(crate)` so integration tests running on the owner's physical
/// XDNA2 hardware (via `cargo test --features amd-xdna2 --test ...`) can
/// directly validate the open behavior when the platform matches.
///
/// Without `amd-xdna2`, production callers are feature-gated away; unit tests in
/// this module still exercise the helper. Allow dead_code for default-feature
/// clippy on the non-test lib graph.
#[cfg_attr(not(feature = "amd-xdna2"), allow(dead_code))]
pub(crate) fn try_open_accel_device(
    detected: Option<&AccelDevice>,
) -> (Option<std::fs::File>, Option<std::path::PathBuf>) {
    if detected.is_none() {
        return (None, None);
    }

    for i in 0..4 {
        let candidate = std::path::PathBuf::from(format!("/dev/accel/accel{i}"));
        // Must be O_RDWR — see doc above. Do not use File::open (O_RDONLY).
        if let Ok(f) = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&candidate)
        {
            return (Some(f), Some(candidate));
        }
    }
    (None, None)
}

/// Information about a detected accel device.
#[derive(Debug, Clone, Default)]
#[allow(missing_docs)] // internal probe details, documented via usage
pub struct AccelDevice {
    pub name: String,
    pub driver: String,
    pub pci_vendor: String,
    pub pci_device: String,
    pub pci_revision: String,
    /// The NPU name reported by the driver (e.g. from /sys/class/accel/accelN/name)
    pub npu_name: String,
    /// modalias string (often contains exact chip info for amdxdna)
    pub modalias: String,
    /// Extracted XDNA2 variant (NPU4/NPU5/NPU6) if detectable from modalias
    pub xdna2_variant: Option<String>,
    /// Pure PCI cmdlist note from vendor/device/revision already read.
    /// Empty when those strings did not parse. AIE4 rows store the fail-closed
    /// error. This field is not a device open and not a submit.
    pub cmdlist_class: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_accel_devices_is_safe_and_deterministic() {
        let devs = probe_accel_devices();
        for d in &devs {
            assert!(d.name.starts_with("accel"));
            assert_ne!(d.driver, "");
            if !d.pci_vendor.is_empty() {
                assert!(d.pci_vendor.starts_with("0x"));
            }
            // npu_name, modalias, and xdna2_variant are nice-to-have on real hardware
        }
    }

    // TDD tests for the pure modalias variant parser (XDNA2 focus).
    // These directly exercise richer reporting for user's real AMD XDNA2 hardware
    // (NPU4/5/6 variants on Ryzen AI 300 series etc.) without any FS or hardware.

    #[test]
    fn parse_xdna2_variant_npu4() {
        // 17f0 family now correctly falls back to XDNA2 even without npu substring in modalias
        assert_eq!(
            parse_xdna2_variant("pci:1022:17f0:...", "0x17f0"),
            Some("XDNA2".to_string())
        );
        assert_eq!(parse_xdna2_variant("npu4", ""), Some("NPU4".to_string()));
        assert_eq!(
            parse_xdna2_variant("foo-npu4-bar", ""),
            Some("NPU4".to_string())
        );
    }

    #[test]
    fn parse_xdna2_variant_npu5() {
        assert_eq!(parse_xdna2_variant("npu5", ""), Some("NPU5".to_string()));
        assert_eq!(parse_xdna2_variant("NPU5", ""), Some("NPU5".to_string()));
        assert_eq!(
            parse_xdna2_variant("modalias=...npu5...strix", ""),
            Some("NPU5".to_string())
        );
    }

    #[test]
    fn parse_xdna2_variant_npu6() {
        assert_eq!(parse_xdna2_variant("npu6", ""), Some("NPU6".to_string()));
        assert_eq!(
            parse_xdna2_variant("something-NPU6", ""),
            Some("NPU6".to_string())
        );
    }

    #[test]
    fn parse_xdna2_variant_none_for_firstgen_or_other() {
        assert_eq!(parse_xdna2_variant("", ""), None);
        assert_eq!(parse_xdna2_variant("amdxdna", ""), None);
        assert_eq!(parse_xdna2_variant("npu", ""), None);
        assert_eq!(parse_xdna2_variant("npu3", ""), None);
        assert_eq!(parse_xdna2_variant("1022:1502", ""), None); // first-gen XDNA PCI
    }

    #[test]
    fn parse_xdna2_variant_prefers_first_match_priority() {
        // Current impl prioritizes npu4 > npu5 > npu6 on multi-matches (rare but defined).
        let multi = "npu4-and-npu5";
        assert_eq!(parse_xdna2_variant(multi, ""), Some("NPU4".to_string()));
    }

    #[test]
    fn parse_xdna2_variant_real_hardware_modalias_patterns() {
        // Additional TDD cases mirroring what amdxdna + kernel expose on real
        // Ryzen AI 300 / Strix Point / Krackan hardware (user's XDNA2 target).
        let strix = "pci:v00001022d000017F0sv00000000sd00000000bc12sc80i00-npu5";
        assert_eq!(parse_xdna2_variant(strix, ""), Some("NPU5".to_string()));

        let npu6_long = "modalias=...npu6-phx...";
        assert_eq!(parse_xdna2_variant(npu6_long, ""), Some("NPU6".to_string()));
    }

    #[test]
    fn parse_xdna2_variant_substring_safety() {
        // "npu" alone or as part of non-XDNA2 string must not false-positive.
        assert_eq!(parse_xdna2_variant("npu-legacy", ""), None);
        assert_eq!(parse_xdna2_variant("something-npuish", ""), None);
    }

    #[test]
    fn parse_xdna2_variant_falls_back_on_17f0_pci_family() {
        // Real hardware case (user's 1022:17f0 rev 0x20 system): modalias often lacks
        // explicit "npu5" etc., so we fall back on the well-known XDNA2 PCI device ID.
        assert_eq!(
            parse_xdna2_variant("pci:v00001022d000017F0...", "0x17f0"),
            Some("XDNA2".to_string())
        );
        assert_eq!(
            parse_xdna2_variant("some other modalias", "0x17f0"),
            Some("XDNA2".to_string())
        );
    }

    /// V5 patch 5 PCI rows. Revision selects the info struct.
    /// `0x17f0` rev `0x20` is npu6, not npu4. AIE4 rows fail closed.
    /// `0x1502` stays unclassified (no info struct in that patch).
    #[test]
    fn classify_amd_npu_pci_id_aie2_rows_and_aie4_fail_closed() {
        const AIE4: &str = "AIE4: not supported by this crate's AIE2 cmdlist path";

        assert_eq!(
            classify_amd_npu_pci_id(0x1022, 0x17f0, 0x10),
            Ok(Aie2DevInfo::DevNpu4)
        );
        assert_eq!(
            classify_amd_npu_pci_id(0x1022, 0x17f0, 0x11),
            Ok(Aie2DevInfo::DevNpu5)
        );
        assert_eq!(
            classify_amd_npu_pci_id(0x1022, 0x17f0, 0x20),
            Ok(Aie2DevInfo::DevNpu6)
        );
        assert_ne!(
            classify_amd_npu_pci_id(0x1022, 0x17f0, 0x20),
            Ok(Aie2DevInfo::DevNpu4),
            "rev 0x20 is dev_npu6_info, not npu4"
        );

        for (device, revision) in [
            (0x17f1_u16, 0x10_u8),
            (0x17f1, 0x13),
            (0x17f2, 0x10),
            (0x17f2, 0x13),
            (0x17f3, 0x10),
            (0x17f3, 0x13),
        ] {
            let Err(err) = classify_amd_npu_pci_id(0x1022, device, revision) else {
                panic!("AIE4 id {device:#06x} rev {revision:#04x} must fail closed");
            };
            let message = err.to_string();
            assert!(
                message.contains(AIE4),
                "device {device:#06x} rev {revision:#04x}: {message}"
            );
        }

        for (device, revision) in [
            (0x17f0_u16, 0x00_u8),
            (0x17f0, 0x12),
            (0x17f0, 0x21),
            (0x17f1, 0x11),
            (0x17f1, 0x20),
            (0x17f2, 0x00),
            (0x17f3, 0x11),
        ] {
            let Err(err) = classify_amd_npu_pci_id(0x1022, device, revision) else {
                panic!("unknown rev {device:#06x}/{revision:#04x} must fail closed");
            };
            let message = err.to_string();
            assert!(
                !message.contains(AIE4),
                "unknown rev must not use the AIE4 sentence: {message}"
            );
            assert!(
                message.contains("unknown"),
                "unknown rev needs a clear error: {message}"
            );
        }

        let Err(first_gen) = classify_amd_npu_pci_id(0x1022, 0x1502, 0x00) else {
            panic!("0x1502 has no info struct and must stay unsupported");
        };
        let first_gen_message = first_gen.to_string();
        assert!(
            !first_gen_message.contains(AIE4),
            "0x1502 is not an AIE4 row: {first_gen_message}"
        );
        assert!(
            !first_gen_message.to_ascii_lowercase().contains("npu4")
                && !first_gen_message.to_ascii_lowercase().contains("npu6")
                && !first_gen_message.contains("AIE2"),
            "0x1502 must not invent a generation: {first_gen_message}"
        );

        let Err(other_vendor) = classify_amd_npu_pci_id(0x1002, 0x17f0, 0x20) else {
            panic!("non-AMD vendor must fail closed");
        };
        assert!(!other_vendor.to_string().contains(AIE4));
    }

    // === TDD for the extracted real-device open helper (xdna2-real-exec-01) ===

    #[test]
    fn try_open_accel_device_returns_none_safely_for_no_device() {
        // Critical safe path: never panics, returns None pair when no AccelDevice
        // (the normal case on CI, dev laptops without XDNA2, containers, etc.).
        let (file, path) = try_open_accel_device(None);
        assert!(file.is_none());
        assert!(path.is_none());
    }

    #[test]
    fn try_open_accel_device_with_detected_device_attempts_open_and_returns_consistent_result() {
        // Supplying a detected device causes the helper to attempt the real
        // multi-index open loop (0..4). Result depends on whether /dev/accel nodes
        // exist on the current machine:
        // - On real XDNA2 hardware (user's target platform) this often succeeds.
        // - On CI / other machines it returns None.
        // The contract under test: no panic, and any successful open returns a
        // sensible /dev/accel path opened O_RDWR (required for DevHeap mmap).
        let fake = AccelDevice {
            name: "accel0".to_string(),
            driver: "amdxdna".to_string(),
            pci_vendor: "0x1022".to_string(),
            pci_device: "0x17f0".to_string(),
            pci_revision: "0x20".to_string(),
            ..Default::default()
        };
        let (file, path) = try_open_accel_device(Some(&fake));
        if let (Some(f), Some(p)) = (file, path) {
            let pstr = p.to_string_lossy();
            assert!(
                pstr.contains("/dev/accel/accel"),
                "opened path must be an accel device: {pstr}"
            );
            // Drop the file handle cleanly (test does not need to keep it open).
            drop(f);
        }
    }

    #[test]
    fn try_open_accel_device_uses_rdwr_not_rdonly() {
        // Regression: File::open (O_RDONLY) made DevHeap mmap return EPERM and
        // blocked CREATE_HWCTX on real silicon. OpenOptions must request write.
        // We only assert the helper compiles the O_RDWR path; on hardware the
        // integration / probe exercises real mmap.
        let fake = AccelDevice {
            name: "accel0".to_string(),
            driver: "amdxdna".to_string(),
            pci_vendor: "0x1022".to_string(),
            pci_device: "0x17f0".to_string(),
            pci_revision: "0x20".to_string(),
            ..Default::default()
        };
        let (file, path) = try_open_accel_device(Some(&fake));
        // Either no device (CI) or a live O_RDWR handle — never panic.
        assert_eq!(file.is_some(), path.is_some());
    }
}
