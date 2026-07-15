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
//! - **Library-first**: Core API is the product; optional `dragon-npu` binary is a thin Phase 8 wrapper.
//! - **TDD**: Tests are written first. Use `cargo test` as the primary loop.
//! - **Zero panics in library code**: No `unwrap()`, `expect()`, `panic!()`,
//!   or `unreachable!()`. Every fallible operation returns `Result<T, DragonNpuError>`.
//! - **Explicit errors**: A single rich `DragonNpuError` enum via `thiserror`.
//! - **Incremental**: Real compiler passes, JSON-based model loading (`parse_model`),
//!   and observable graph mutations are implemented. Remaining work focuses on
//!   full ONNX/TorchScript loading and additional hardware backends.
//!
//! ## Current Status (XDNA2 primary; Phase 9 packaging)
//!
//! - AMD XDNA2 backend (feature `amd-xdna2`): direct path bring-up + product CLI/benches.
//!   **No** default silicon GEMM Pass / `ran_on_npu` without numerical verify.
//! - Secondary features `intel-vpu` / `qualcomm-hexagon`: honest stubs only (not auto-selected).
//! - Optional `xrt-bridge`: research refuse-stub only — never default or hot path.
//! - Real ONNX + Phase 7 typed `DragonNpuIR`; Phase 8 CLI (`dragon-npu`).
//!
//! See `PLAN.md`, `AGENTS.md`, `docs/SECONDARY_BACKENDS.md`, `docs/PACKAGING.md`.

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

/// Phase 7 typed IR (`DragonNpuIR`) alongside `ModelGraph` parallel Vecs.
///
/// Shape inference, constant fold, DCE, and heuristic XDNA2 cost model.
/// Dual representation — does not replace host_ref / fuse paths.
pub mod ir;
pub use ir::{
    AttrValue, CostEstimate, DType, DragonNpuIR, IrNode, IrOptStats, NodeAttrs, NodeId, NodeOp,
    TensorDesc, TensorId, estimate_xdna2_cost, node_attrs_from_onnx_attributes,
};

/// Colibrì / `npu_direct_bench` JSON schema version for timed-op and related objects.
///
/// Feature-free so default builds and feature-disabled example arms share one constant.
/// Bump only when removing/renaming fields or changing honesty semantics.
/// See `docs/COLIBRI_ADAPTER.md`.
pub const TIMED_OP_JSON_SCHEMA_VERSION: u32 = 1;

/// CLI argv helpers for the optional `dragon-npu` binary (Phase 8).
///
/// Pure parse/load helpers are unit-tested here so default `cargo test --lib`
/// never opens `/dev/accel` or enables EXEC_CMD.
pub mod cli;

/// Re-exports for convenient `use dragon_npu::prelude::*;`
pub mod prelude {
    pub use crate::CompiledModel;
    pub use crate::DragonNpu;
    pub use crate::DragonNpuError;
    pub use crate::DragonNpuIR;
    pub use crate::ModelCompiler;
    pub use crate::NpuStatus;
}
