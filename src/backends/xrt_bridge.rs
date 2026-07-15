//! Optional **research-only** XRT bridge stub (`xrt-bridge` feature).
//!
//! # Purpose
//!
//! Offline research / capture comparison notes only — e.g. comparing public ERT
//! command layouts or documenting how XRT packages instruction BOs. This module
//! is **not** on the production hot path and must never become the default
//! execution route.
//!
//! # Hard rules
//!
//! - Feature is **OFF** by default and is **not** implied by `amd-xdna2`.
//! - No XRT library link, no hard runtime dependency on `libxrt`.
//! - Does **not** open EXEC_CMD, does **not** recommend enabling L1 gates
//!   (`xdna2-dangerous-exec-cmd` / `XDNA2_ALLOW_EXEC_CMD`).
//! - Never sets or implies `ran_on_npu` / silicon Pass.
//! - Production path remains pure lightweight direct `amdxdna` under `amd-xdna2`.
//!
//! See `docs/XDNA2_REAL_EXECUTION.md`, `docs/XDNA2_NEVER_AGAIN.md`, and
//! `docs/SECONDARY_BACKENDS.md`.

use crate::backends::Backend;
use crate::{CompiledModel, DragonNpuError, NpuStatus};

/// Feature / backend id for errors and docs.
pub const BACKEND_ID: &str = "xrt-bridge";

/// Status device label — explicitly research stub.
pub const DEVICE_NAME: &str = "XRT bridge (research stub — not a runtime)";

const REFUSE_MSG: &str = "xrt-bridge is research-only and does not call XRT or amdxdna \
     EXEC_CMD. Production path: feature `amd-xdna2` direct driver (no XRT). Do not enable \
     xdna2-dangerous-exec-cmd or XDNA2_ALLOW_EXEC_CMD for this stub. See docs/SECONDARY_BACKENDS.md \
     and docs/XDNA2_NEVER_AGAIN.md.";

/// Research-only XRT bridge handle (opt-in feature `xrt-bridge`).
///
/// All execution entry points refuse. No device open, no EXEC_CMD.
#[derive(Debug, Clone, Default)]
pub struct XrtBridge;

impl XrtBridge {
    /// Construct the research stub. Never selected by [`crate::DragonNpu::new`].
    pub fn new() -> Self {
        Self
    }

    /// Documented purpose string for tooling / CLI research notes.
    pub fn purpose() -> &'static str {
        "offline research / capture comparison only; never production hot path; never EXEC_CMD"
    }

    fn refuse() -> DragonNpuError {
        DragonNpuError::BackendError {
            backend: BACKEND_ID.to_string(),
            message: REFUSE_MSG.to_string(),
        }
    }

    /// Explicit research API: always refuse real XRT calls (placeholder for future
    /// offline capture helpers that must still not submit work).
    pub fn try_xrt_session(&self) -> Result<(), DragonNpuError> {
        Err(Self::refuse())
    }
}

impl Backend for XrtBridge {
    fn status(&self) -> Result<NpuStatus, DragonNpuError> {
        Ok(NpuStatus {
            vendor: "XRT-research".to_string(),
            available: false,
            compute_units: 0,
            memory_mb: 0,
            peak_tops: 0.0,
            device_name: DEVICE_NAME.to_string(),
            driver_version: "research-stub (no libxrt link)".to_string(),
            supported_dtypes: vec![],
            simulated: true,
        })
    }

    fn compile_model(
        &self,
        model_path: &str,
        _optimization_level: u8,
        _quantization: &str,
    ) -> Result<CompiledModel, DragonNpuError> {
        Err(DragonNpuError::BackendError {
            backend: BACKEND_ID.to_string(),
            message: format!(
                "xrt-bridge does not compile models (research stub). path={model_path}. \
                 Use SimulationBackend or amd-xdna2 ModelCompiler path."
            ),
        })
    }

    fn run(&self, _input: &[f32]) -> Result<Vec<f32>, DragonNpuError> {
        Err(Self::refuse())
    }

    fn submit_and_run(
        &self,
        _model: &CompiledModel,
        _input: &[f32],
    ) -> Result<Vec<f32>, DragonNpuError> {
        Err(Self::refuse())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::SimulationBackend;

    #[test]
    fn purpose_documents_research_only() {
        let p = XrtBridge::purpose();
        assert!(p.contains("research"));
        assert!(p.contains("never production hot path"));
        assert!(p.contains("never EXEC_CMD"));
    }

    #[test]
    fn status_unavailable_simulated() {
        let x = XrtBridge::new();
        let s = x.status().expect("status ok");
        assert!(!s.available);
        assert!(s.simulated);
        assert!(s.device_name.contains("research stub"));
        // Explicit no-link wording (not a tautology on "libxrt" alone).
        assert!(
            s.driver_version.contains("no libxrt"),
            "expected no-libxrt honesty, got {}",
            s.driver_version
        );
    }

    #[test]
    fn try_xrt_session_refuses() {
        let x = XrtBridge::new();
        let err = x.try_xrt_session().expect_err("must refuse");
        let msg = err.to_string();
        assert!(msg.contains("xrt-bridge"));
        assert!(
            msg.contains("Do not enable"),
            "must warn against opening L1 gates, got: {msg}"
        );
        assert!(msg.contains("research") || msg.contains("research-only"));
    }

    #[test]
    fn run_and_compile_refuse() {
        let x = XrtBridge::new();
        let run_err = x.run(&[0.0]).expect_err("run must refuse");
        assert!(run_err.to_string().contains("xrt-bridge"));
        let c_err = x
            .compile_model("any.onnx", 0, "fp32")
            .expect_err("compile refuse");
        assert!(
            c_err.to_string().contains("xrt-bridge")
                || c_err.to_string().contains("does not compile")
        );
    }

    #[test]
    fn submit_and_run_refuses_with_foreign_compiled_model() {
        // XrtBridge::compile_model always refuses, so build a model via Simulation
        // and prove the Backend::submit_and_run refuse path (honesty contract).
        let model = SimulationBackend::new()
            .compile_model("tests/fixtures/host_matmul_relu.json", 0, "fp32")
            .expect("sim compile for dummy CompiledModel arg");
        let x = XrtBridge::new();
        let err = x
            .submit_and_run(&model, &[0.0f32; 4])
            .expect_err("submit_and_run must refuse");
        let msg = err.to_string();
        assert!(
            msg.contains("xrt-bridge") || msg.contains("research"),
            "expected xrt refuse, got: {msg}"
        );
        assert!(
            msg.contains("Do not enable") || msg.contains("EXEC_CMD"),
            "refuse message should mention EXEC_CMD policy, got: {msg}"
        );
    }

    #[test]
    fn dragon_npu_new_does_not_select_xrt_bridge() {
        let npu = crate::DragonNpu::new().expect("new");
        let s = npu.status().expect("status");
        assert!(
            !s.device_name.contains("XRT bridge"),
            "must not auto-select xrt-bridge, got {}",
            s.device_name
        );
    }
}
