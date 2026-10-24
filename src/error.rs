/// Error types for Earth Dragon NPU.
///
/// All public APIs return `Result<T, DragonNpuError>`.
/// Never use `Option` to represent errors.
use thiserror::Error;

/// The primary error type for all Earth Dragon NPU operations.
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

    /// Real XDNA2 `submit_and_run` is intentionally not ready for a silicon Pass claim.
    ///
    /// Used when the Real arm compiled the model successfully but cannot (or must not)
    /// claim NPU inference: EXEC_CMD gated, missing instruction txn, fail-closed default
    /// mode, or EXEC envelope completed without numerical verify (`ran_on_npu` still false).
    /// See `docs/XDNA2_REAL_EXECUTION.md` Phase 4 and `docs/XDNA2_NEVER_AGAIN.md`.
    #[error("Real XDNA2 submit not ready: {reason}")]
    RealSubmitNotReady {
        /// Actionable reason (gates, modes, device open, blob, honesty notes).
        reason: String,
    },

    /// Graph contains operator types not supported for the selected real-submit mode.
    #[error("Unsupported ops for real XDNA2 submit on model '{model}': {ops}")]
    UnsupportedOps {
        /// Model path or name.
        model: String,
        /// Comma-separated unsupported op type names (ONNX op_type / IR ops).
        ops: String,
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
