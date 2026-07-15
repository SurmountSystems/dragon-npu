//! Qualcomm Hexagon (Snapdragon X / mobile) **honest stub**.
//!
//! Compiled only when the `qualcomm-hexagon` feature is enabled.
//!
//! # Honesty policy
//!
//! - **Not** a working Hexagon / QNN runtime. No remoteproc DSP or QNN SDK path.
//! - `status().available == false`, `simulated == true`.
//! - `compile_model` may run the host `ModelCompiler` for graph analysis only.
//! - `run` / `submit_and_run` always refuse with [`DragonNpuError::BackendError`].
//! - **Never** selected by [`crate::DragonNpu::new`] — opt-in via
//!   [`QualcommHexagonBackend::new`] only.
//!
//! Detection paths (future, weakest Linux laptop story): `remoteproc`, vendor
//! QNN — see `docs/SECONDARY_BACKENDS.md`.
//!
//! Primary hardware target remains AMD XDNA2 (`amd-xdna2`).

use std::sync::Arc;

use crate::ModelCompiler;
use crate::backends::Backend;
use crate::{CompiledModel, DragonNpuError, NpuStatus};

/// Clear backend id used in errors and docs.
pub const BACKEND_ID: &str = "qualcomm-hexagon";

/// Human-readable device label for status / diagnostics.
pub const DEVICE_NAME: &str = "Qualcomm Hexagon (stub — not implemented)";

/// Fixed refuse reason for run / submit (no silent simulation success).
const REFUSE_MSG: &str = "Qualcomm Hexagon backend is an honest stub: no QNN / remoteproc \
     hardware submission is implemented. Use SimulationBackend (default) or feature \
     `amd-xdna2` for XDNA2 bring-up. See docs/SECONDARY_BACKENDS.md.";

/// Qualcomm Hexagon backend stub (opt-in only).
///
/// Does **not** claim real Hexagon inference. Prefer simulation or XDNA2.
#[derive(Debug, Clone, Default)]
pub struct QualcommHexagonBackend;

impl QualcommHexagonBackend {
    /// Construct the opt-in stub. Not used by [`crate::DragonNpu::new`].
    pub fn new() -> Self {
        Self
    }

    fn refuse_exec() -> DragonNpuError {
        DragonNpuError::BackendError {
            backend: BACKEND_ID.to_string(),
            message: REFUSE_MSG.to_string(),
        }
    }
}

impl Backend for QualcommHexagonBackend {
    fn status(&self) -> Result<NpuStatus, DragonNpuError> {
        Ok(NpuStatus {
            vendor: "Qualcomm".to_string(),
            available: false,
            compute_units: 0,
            memory_mb: 0,
            peak_tops: 0.0,
            device_name: DEVICE_NAME.to_string(),
            driver_version: "not-implemented (stub; no QNN/remoteproc path)".to_string(),
            supported_dtypes: vec![],
            simulated: true,
        })
    }

    fn compile_model(
        &self,
        model_path: &str,
        optimization_level: u8,
        quantization: &str,
    ) -> Result<CompiledModel, DragonNpuError> {
        let compiler = ModelCompiler::new();
        compiler.compile(
            model_path,
            optimization_level,
            quantization,
            Arc::new(self.clone()),
        )
    }

    fn run(&self, _input: &[f32]) -> Result<Vec<f32>, DragonNpuError> {
        Err(Self::refuse_exec())
    }

    fn submit_and_run(
        &self,
        _model: &CompiledModel,
        _input: &[f32],
    ) -> Result<Vec<f32>, DragonNpuError> {
        Err(Self::refuse_exec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_is_unavailable_simulated_stub() {
        let b = QualcommHexagonBackend::new();
        let s = b.status().expect("status must succeed");
        assert!(!s.available);
        assert!(s.simulated);
        assert!(s.device_name.contains("stub"));
        assert!(s.driver_version.contains("not-implemented"));
        assert_eq!(s.vendor, "Qualcomm");
        assert!(s.supported_dtypes.is_empty());
        assert_eq!(s.peak_tops, 0.0);
    }

    #[test]
    fn run_refuses_without_fake_hardware_success() {
        let b = QualcommHexagonBackend::new();
        let err = b.run(&[1.0]).expect_err("run must refuse");
        let msg = err.to_string();
        assert!(
            msg.contains("qualcomm-hexagon") || msg.contains("stub"),
            "expected refuse, got: {msg}"
        );
    }

    #[test]
    fn submit_and_run_refuses() {
        let b = QualcommHexagonBackend::new();
        let model = b
            .compile_model("tests/fixtures/host_add_mul.json", 0, "fp32")
            .expect("compile is host analysis only");
        let err = b
            .submit_and_run(&model, &[0.0f32; 4])
            .expect_err("submit must refuse");
        assert!(
            err.to_string().contains("stub")
                || err.to_string().contains("Hexagon")
                || err.to_string().contains("not implemented")
        );
    }

    #[test]
    fn compile_succeeds_but_model_run_refuses() {
        let b = QualcommHexagonBackend::new();
        let model = b
            .compile_model("tests/fixtures/host_add_mul.json", 1, "int8")
            .expect("host compile");
        let err = model.run(&[0.0f32; 4]).expect_err("run must refuse");
        assert!(
            err.to_string().contains("stub") || err.to_string().contains("qualcomm-hexagon"),
            "got: {err}"
        );
    }

    #[test]
    fn dragon_npu_new_does_not_select_hexagon_stub() {
        let npu = crate::DragonNpu::new().expect("default new");
        let s = npu.status().expect("status");
        assert!(
            !s.device_name.contains("Qualcomm Hexagon (stub"),
            "DragonNpu::new must not auto-select Hexagon stub, got {}",
            s.device_name
        );
    }
}
