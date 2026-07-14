//! # DragonNPU
//!
//! Production-quality Rust library for vendor-agnostic NPU (Neural Processing Unit)
//! acceleration on Linux.
//!
//! This is the Rust rewrite of the original Python `dragon_npu` package.
//! The Python implementation is preserved in `dragon_npu/` and serves as the
//! reference for API parity, behavior, and feature set (see `README.old.md`
//! and `PLAN.md`).
//!
//! ## Design Principles
//!
//! - **Library-first**: No binary until the core library is solid.
//! - **TDD**: Tests are written first. Use `cargo test` as the primary loop.
//! - **Zero panics in library code**: No `unwrap()`, `expect()`, `panic!()`,
//!   or `unreachable!()`. Every fallible operation returns `Result<T, DragonNpuError>`.
//! - **Explicit errors**: A single rich `DragonNpuError` enum via `thiserror`.
//! - **Incremental**: Real compiler passes, JSON-based model loading (`parse_model`),
//!   and observable graph mutations are implemented. Remaining work focuses on
//!   full ONNX/TorchScript loading and additional hardware backends.
//!
//! ## Current Status (autonomous execution, XDNA2 primary focus)
//!
//! - AMD XDNA2 backend (feature `amd-xdna2`): Rich probe (`AccelDevice` with npu_name + modalias),
//!   pure `parse_xdna2_variant_from_modalias` (NPU4/5/6, heavily TDD'd), stores/exposes
//!   `detected_device()` + `detected_npu_type()`, realistic params (50 TOPS, bf16), `simulated: bool`
//!   on `NpuStatus`, preferred in `DragonNpu::new()` under the feature. Best-effort simulation
//!   with honest fallback.
//! - Real ONNX loading (tract-onnx): op extraction + full value_info dtype + shape extraction
//!   (TensorShapeProto + Dimension oneof) now live in the protobuf path (onx-03 complete).
//! - Compiler: all optimization passes perform real `ModelGraph` mutations (including weight-aware
//!   MemoryPlanning + folding via as_f32_data). Initializer now has zero dead_code allows; small
//!   onx-16 micro-improvement (better content hashing) landed.
//! - All gates (fmt/clippy -D/test +/- features) clean after every change. Continuing autonomous
//!   work on remaining small non-HITL items (see todo list and PLAN.md).
//!
//! See `PLAN.md` for roadmap and `AGENTS.md` for process. HITL limited to user's real XDNA2 hardware testing.

/// Error types for the DragonNPU library.
///
/// See `src/error.rs` for the full `DragonNpuError` enum definition.
pub mod error;
pub use error::DragonNpuError;

/// Core NPU types (DragonNpu, CompiledModel, NpuStatus) and pluggable backends.
///
/// `DragonNpu::new()` prefers the AMD XDNA2 backend when the `amd-xdna2` feature
/// is enabled (real detection + realistic parameters or honest simulation fallback).
/// All execution goes through the `Backend` trait; `submit_and_run` is the primary
/// hook for real NPU hardware submission paths.
///
/// See `dragon_npu/dragon_npu_core.py` for the Python reference implementation.
pub mod core;
pub use core::{CompiledModel, DragonNpu, NpuStatus};

pub mod backends;
pub use backends::simulation::SimulationBackend;

/// Compiler module (real ONNX loading with multi-dtype weights + bf16 raw, optimization passes with observable mutations, numeric weight data use in scoring and memory planning).
///
/// Model parsing, optimization passes, and quantization.
/// See `dragon_npu_compiler.py` for the reference implementation.
pub mod compiler;
pub use compiler::ModelCompiler;

/// Re-exports for convenient `use dragon_npu::prelude::*;`
pub mod prelude {
    pub use crate::CompiledModel;
    pub use crate::DragonNpu;
    pub use crate::DragonNpuError;
    pub use crate::ModelCompiler;
    pub use crate::NpuStatus;
}
