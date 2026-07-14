/// Error types for DragonNPU.
///
/// All public APIs return `Result<T, DragonNpuError>`.
/// Never use `Option` to represent errors.
use thiserror::Error;

/// The primary error type for all DragonNPU operations.
///
/// Variants are descriptive and include context (model names, reasons, etc.)
/// so that callers can produce good user-facing messages or logs.
#[derive(Error, Debug)]
pub enum DragonNpuError {
    /// NPU hardware or driver is not available on this system.
    #[error("NPU not available: {0}")]
    NpuNotAvailable(String),

    /// Model compilation failed.
    #[error("Compilation failed for model '{model}': {reason}")]
    CompilationFailed {
        /// Name or path of the model that failed to compile.
        model: String,
        /// Detailed reason for the failure.
        reason: String,
    },

    /// Runtime / inference error.
    #[error("Runtime error: {0}")]
    RuntimeError(String),

    /// Error from a specific backend (AMD XDNA, simulation, etc.).
    #[error("Backend '{backend}' error: {message}")]
    BackendError {
        /// Name of the backend that produced the error.
        backend: String,
        /// Backend-specific error message.
        message: String,
    },

    /// I/O or filesystem error (device files, model loading, etc.).
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Model parsing / deserialization error (JSON, .txt, future ONNX/TorchScript).
    #[error("Failed to parse model '{model}': {reason}")]
    ModelParseError {
        /// Path or name of the model that could not be parsed.
        model: String,
        /// Detailed parse failure reason.
        reason: String,
    },

    /// Generic internal error (used only during early development).
    #[error("Internal error: {0}")]
    Internal(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_variants_are_descriptive() {
        let err = DragonNpuError::NpuNotAvailable("no driver".to_string());
        assert!(err.to_string().contains("NPU not available"));
    }
}
