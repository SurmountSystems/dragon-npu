//! Qualcomm Hexagon (Snapdragon X) backend stub.
//!
//! Compiled only when the `qualcomm-hexagon` feature is enabled.

use crate::backends::Backend;
use crate::backends::simulation::{SimulationBackend, SimulationConfig};
use crate::{CompiledModel, DragonNpuError, NpuStatus};

/// Qualcomm Hexagon backend.
///
/// Uses simulation configured with realistic Hexagon parameters.
#[derive(Debug, Default)]
pub struct QualcommHexagonBackend {
    inner: SimulationBackend,
}

impl QualcommHexagonBackend {
    /// Create a new Qualcomm Hexagon backend with realistic hardware parameters.
    pub fn new() -> Self {
        let config = SimulationConfig {
            device_name: "Qualcomm Hexagon (Snapdragon X)".to_string(),
            peak_tops: 45.0,
            latency_factor: 0.65,
            ..Default::default()
        };
        Self {
            inner: SimulationBackend::with_config(config),
        }
    }
}

impl Backend for QualcommHexagonBackend {
    fn status(&self) -> Result<NpuStatus, DragonNpuError> {
        Ok(NpuStatus {
            vendor: "Qualcomm".to_string(),
            available: true,
            compute_units: 6,
            memory_mb: 6144,
            peak_tops: 45.0,
            device_name: "Qualcomm Hexagon (Snapdragon X)".to_string(),
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
