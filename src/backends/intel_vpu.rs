//! Intel VPU (Meteor Lake / Arrow Lake) backend stub.
//!
//! Compiled only when the `intel-vpu` feature is enabled.

use crate::backends::Backend;
use crate::backends::simulation::{SimulationBackend, SimulationConfig};
use crate::{CompiledModel, DragonNpuError, NpuStatus};

/// Intel VPU backend.
///
/// Uses simulation configured with realistic Intel VPU parameters.
#[derive(Debug, Default)]
pub struct IntelVpuBackend {
    inner: SimulationBackend,
}

impl IntelVpuBackend {
    /// Create a new Intel VPU backend with realistic hardware parameters.
    pub fn new() -> Self {
        let config = SimulationConfig {
            device_name: "Intel VPU (Meteor Lake)".to_string(),
            peak_tops: 40.0, // Approximate for current Intel NPUs
            latency_factor: 0.7,
            ..Default::default()
        };
        Self {
            inner: SimulationBackend::with_config(config),
        }
    }
}

impl Backend for IntelVpuBackend {
    fn status(&self) -> Result<NpuStatus, DragonNpuError> {
        Ok(NpuStatus {
            vendor: "Intel".to_string(),
            available: true,
            compute_units: 8,
            memory_mb: 4096,
            peak_tops: 40.0,
            device_name: "Intel VPU (Meteor Lake)".to_string(),
            driver_version: "stub-0.1".to_string(),
            supported_dtypes: vec!["fp32".to_string(), "fp16".to_string(), "int8".to_string()],
            simulated: true,
        })
    }

    fn compile_model(
        &self,
        model_path: &str,
        optimization_level: u8,
        quantization: &str,
    ) -> Result<CompiledModel, DragonNpuError> {
        self.inner
            .compile_model(model_path, optimization_level, quantization)
    }

    fn run(&self, input: &[f32]) -> Result<Vec<f32>, DragonNpuError> {
        self.inner.run(input)
    }
}
