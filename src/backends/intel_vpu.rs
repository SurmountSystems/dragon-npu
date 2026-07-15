//! Intel VPU (Meteor Lake / Arrow Lake / Lunar Lake) **honest stub**.
//!
//! Compiled only when the `intel-vpu` feature is enabled.
//!
//! # Honesty policy
//!
//! - **Not** a working Intel NPU runtime. No Level Zero / OpenVINO / ivpu
//!   submission is implemented.
//! - `status().available == false`, `simulated == true`.
//! - `compile_model` may run the host `ModelCompiler` for graph analysis only;
//!   the resulting `CompiledModel` is still bound to this stub.
//! - `run` / `submit_and_run` always refuse with [`DragonNpuError::BackendError`].
//! - **Never** selected by [`crate::DragonNpu::new`] — opt-in via
//!   [`IntelVpuBackend::new`] only.
//!
//! Detection paths (future work, not wired): Linux `intel_vpu` / `ivpu` under
//! `/sys/class/accel` or accel DRM nodes — see `docs/SECONDARY_BACKENDS.md`.
//!
//! Primary hardware target remains AMD XDNA2 (`amd-xdna2`).

use std::sync::Arc;

use crate::ModelCompiler;
use crate::backends::Backend;
use crate::{CompiledModel, DragonNpuError, NpuStatus};

/// Clear backend id used in errors and docs.
pub const BACKEND_ID: &str = "intel-vpu";

/// Human-readable device label for status / diagnostics.
pub const DEVICE_NAME: &str = "Intel VPU (stub — not implemented)";

/// Fixed refuse reason for run / submit (no silent simulation success).
const REFUSE_MSG: &str = "Intel VPU backend is an honest stub: no hardware or Level Zero \
     submission is implemented. Use SimulationBackend (default) or feature `amd-xdna2` for \
     XDNA2 bring-up. See docs/SECONDARY_BACKENDS.md.";

/// Intel VPU backend stub (opt-in only).
///
/// Does **not** claim real VPU inference. Prefer simulation or XDNA2 for work.
#[derive(Debug, Clone, Default)]
pub struct IntelVpuBackend;

impl IntelVpuBackend {
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

impl Backend for IntelVpuBackend {
    fn status(&self) -> Result<NpuStatus, DragonNpuError> {
        Ok(NpuStatus {
            vendor: "Intel".to_string(),
            available: false,
            compute_units: 0,
            memory_mb: 0,
            peak_tops: 0.0,
            device_name: DEVICE_NAME.to_string(),
            driver_version: "not-implemented (stub; no ivpu/Level Zero path)".to_string(),
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
        // Host-side graph analysis only; run still refuses on this backend.
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
        let b = IntelVpuBackend::new();
        let s = b.status().expect("status must succeed");
        assert!(!s.available, "stub must not report hardware available");
        assert!(s.simulated, "stub must set simulated=true");
        assert!(s.device_name.contains("stub"));
        assert!(s.driver_version.contains("not-implemented"));
        assert_eq!(s.vendor, "Intel");
        assert!(s.supported_dtypes.is_empty());
        assert_eq!(s.peak_tops, 0.0);
    }

    #[test]
    fn run_refuses_without_fake_hardware_success() {
        let b = IntelVpuBackend::new();
        let err = b.run(&[1.0, 2.0]).expect_err("run must refuse");
        let msg = err.to_string();
        assert!(
            msg.contains("intel-vpu") || msg.contains("stub"),
            "expected backend refuse, got: {msg}"
        );
        assert!(
            !msg.to_lowercase().contains("ran_on_npu"),
            "must not claim ran_on_npu"
        );
    }

    #[test]
    fn submit_and_run_refuses() {
        let b = IntelVpuBackend::new();
        // Minimal compiled model is not needed: submit refuses before using model fields.
        // Compile a tiny fixture graph then refuse execute.
        let model = b
            .compile_model("tests/fixtures/host_matmul_relu.json", 0, "fp32")
            .expect("compile is host analysis only");
        let err = b
            .submit_and_run(&model, &[0.0f32; 4])
            .expect_err("submit must refuse");
        let msg = err.to_string();
        assert!(msg.contains("stub") || msg.contains("not implemented") || msg.contains("Intel"));
    }

    #[test]
    fn compile_succeeds_but_model_run_refuses() {
        let b = IntelVpuBackend::new();
        let model = b
            .compile_model("tests/fixtures/host_matmul_relu.json", 1, "fp16")
            .expect("host compile for graph analysis");
        assert!(model.node_count() > 0 || model.initial_node_count() > 0);
        let err = model
            .run(&[0.0f32; 8])
            .expect_err("CompiledModel::run must refuse");
        assert!(
            err.to_string().contains("stub") || err.to_string().contains("intel-vpu"),
            "got: {err}"
        );
    }

    #[test]
    fn dragon_npu_new_does_not_select_intel_stub() {
        // Even with intel-vpu feature, default handle stays simulation (or xdna2 if that
        // feature is also on — never this stub).
        let npu = crate::DragonNpu::new().expect("default new");
        let s = npu.status().expect("status");
        assert!(
            !s.device_name.contains("Intel VPU (stub"),
            "DragonNpu::new must not auto-select Intel stub, got {}",
            s.device_name
        );
    }
}
