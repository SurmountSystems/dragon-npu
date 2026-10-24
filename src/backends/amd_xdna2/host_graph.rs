//! Phase 5 — Host graph interpreter + weight → BO planning.
//! Phase 6 — Host efficiency: dtype-aware MatMul (int8/bf16 views), fused MatMul/Gemm+Relu.
//!
//! Executes allowlisted ONNX-style ops on the **host** using `f32` activations and
//! [`CompiledModel`] weights/ops. Results are correct host math only —
//! **never** claim `ran_on_npu` / silicon Pass / silicon GEMM tops.
//!
//! # Supported ops (host_ref)
//!
//! | Op | Semantics |
//! |----|-----------|
//! | `Identity` | Copy / no-op on the activation tensor |
//! | `Relu` | Elementwise `max(0, x)` |
//! | `Add` | Activation `+` RHS (initializer); elementwise same-len or scalar |
//! | `Mul` | Activation `*` RHS (initializer); elementwise same-len or scalar |
//! | `MatMul` / `Gemm` | `A[M,K] @ B[K,N] → C[M,N]` (row-major). See assumptions below |
//!
//! # Phase 6 host efficiency
//!
//! - MatMul/Gemm convert each weight tensor to an f32 view **once** per op
//!   ([`weight_f32_view`]); int8/`data_i8` multiplies without a temporary full
//!   clone when possible. bf16/fp16 use raw LE decode once via `as_f32_data`.
//! - Consecutive `MatMul`/`Gemm` + `Relu` fuse: Relu is applied on write into
//!   the output buffer (no second pass / no extra intermediate for Relu).
//! - Still **host-only** — not NPU throughput.
//!
//! # Phase 5 IR assumptions (pragmatic)
//!
//! The internal IR is still parallel `Vec`s (ops / per-node inputs / initializers).
//! For multi-input ops we resolve RHS tensors as follows:
//!
//! 1. If the node has `inputs` entries that match initializer **names**, use those
//!    (typical ONNX: MatMul `["act", "W"]`, Add `["act", "bias"]`, Gemm bias as third name).
//! 2. Else consume the next unused initializer in declaration order for **required**
//!    tensors only (weight / Add-Mul RHS). Linear-chain JSON with flat `ops` is sequential.
//! 3. Missing weight data → clear error (no fake MatMul golden).
//!
//! ## MatMul / Gemm shapes
//!
//! - Weight `B` must be rank-2 with shape `[K, N]` (ONNX common for linear layers
//!   when `A` is `[M, K]`), and `as_f32_data().len() == K * N`.
//! - Activation length must be `M * K` for some `M >= 1` (`len % K == 0`).
//! - If weight shape is missing/invalid, pass [`HostGraphOptions::matmul_k`] (explicit).
//!   Optional env via [`HostGraphOptions::from_env`] for CLI only — library default does
//!   **not** read ambient env (test isolation).
//! - **Gemm** bias: **named third input only** (must match an initializer). Never
//!   greedily steals the next sequential initializer (would steal the next layer's weight).
//!
//! # Weight BO plan
//!
//! [`build_weight_bo_plan`] sizes Shmem BO suggestions from initializers (payload bytes).
//! [`upload_weights_to_shmem_bos`] is optional bring-up plumbing (create + fill +
//! destroy) and **does not** run inference. On success it returns how many weights
//! were uploaded; handles are **not** live after return.

use std::collections::HashSet;
use std::os::unix::io::RawFd;

use crate::compiler::Initializer;
use crate::{CompiledModel, DragonNpuError};

use super::direct::{BufferType, DirectSubmission};

/// Suggested BO type name for weight upload scaffolding (`AMDXDNA_BO_SHMEM`).
pub const WEIGHT_BO_TYPE_SHMEM: &str = "Shmem";

/// Cap host graph element products (M·N·K work) to avoid runaway tests / OOM.
const HOST_GRAPH_MAX_ELEMS: u64 = 4_000_000;

/// Options for host graph evaluation (explicit; no ambient env in default path).
#[derive(Debug, Clone, Default)]
pub struct HostGraphOptions {
    /// Optional MatMul/Gemm `K` when weight shape is not rank-2.
    /// Prefer this over process env in library/tests.
    pub matmul_k: Option<usize>,
}

impl HostGraphOptions {
    /// CLI helper: read optional `XDNA2_HOST_MATMUL_K`. Not used by default
    /// [`evaluate_host_graph`] (keeps unit tests isolated from shell env).
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            matmul_k: parse_host_matmul_k_env(),
        }
    }
}

/// Weight → device BO plan entry (bring-up only; not an inference claim).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Xdna2WeightPlan {
    /// Initializer / tensor name from the model.
    pub name: String,
    /// Human-readable dtype (`fp32`, `fp16`, `bf16`, `int8`, …).
    pub dtype: String,
    /// Tensor shape as stored on the initializer.
    pub shape: Vec<i64>,
    /// Host-side byte length suggested for a BO fill (matches [`initializer_host_bytes`]).
    pub nbytes: u64,
    /// Suggested amdxdna BO type (`Shmem` for host-visible upload).
    pub suggested_bo_type: String,
}

/// Map ONNX TensorProto `data_type` enum to a short dtype string.
#[must_use]
pub fn onnx_data_type_name(data_type: i32) -> &'static str {
    match data_type {
        1 => "fp32",
        2 => "uint8",
        3 => "int8",
        6 => "int32",
        7 => "int64",
        10 => "fp16",
        16 => "bf16",
        _ => "unknown",
    }
}

/// Bytes required to store one element of an ONNX dtype (best-effort).
#[must_use]
pub fn onnx_dtype_elem_nbytes(data_type: i32) -> u64 {
    match data_type {
        2 | 3 => 1,   // u8 / i8
        7 => 8,       // int64
        10 | 16 => 2, // fp16 / bf16
        // FLOAT(1), INT32(6), and unknown → 4
        _ => 4,
    }
}

/// Product of non-negative shape dims, or `None` if any dim is negative / overflow.
#[must_use]
pub fn shape_element_count(shape: &[i64]) -> Option<u64> {
    if shape.is_empty() {
        return Some(0);
    }
    shape.iter().try_fold(1_u64, |acc, &d| {
        if d < 0 {
            None
        } else {
            acc.checked_mul(d.cast_unsigned())
        }
    })
}

/// Host byte length for an initializer (raw/typed payload preferred; else shape estimate).
///
/// When both shape and payload are present, nbytes follows the **payload** that
/// [`initializer_host_bytes`] will write (plan ↔ upload alignment). Callers that
/// need shape/payload consistency should use [`check_initializer_shape_payload`].
#[must_use]
pub fn initializer_nbytes(init: &Initializer) -> u64 {
    let host = initializer_host_bytes(init);
    if !host.is_empty() {
        return host.len() as u64;
    }
    let elems = shape_element_count(&init.shape).unwrap_or(0);
    elems.saturating_mul(onnx_dtype_elem_nbytes(init.data_type))
}

/// Refuse when non-empty shape product and non-empty payload disagree on element count.
///
/// # Errors
///
/// [`DragonNpuError::RuntimeError`] on mismatch.
pub fn check_initializer_shape_payload(init: &Initializer) -> Result<(), DragonNpuError> {
    let Some(shape_elems) = shape_element_count(&init.shape) else {
        return Err(DragonNpuError::RuntimeError(format!(
            "initializer '{}': invalid (negative/overflow) shape {:?}",
            init.name, init.shape
        )));
    };
    if shape_elems == 0 || init.shape.is_empty() {
        return Ok(());
    }
    let payload_elems = payload_element_count(init);
    if payload_elems == 0 {
        return Ok(());
    }
    if u64::try_from(payload_elems).unwrap_or(u64::MAX) != shape_elems {
        return Err(DragonNpuError::RuntimeError(format!(
            "initializer '{}': shape product {shape_elems} != payload elements {payload_elems} \
             (shape={:?}); refuse inconsistent plan/execute",
            init.name, init.shape
        )));
    }
    Ok(())
}

fn payload_element_count(init: &Initializer) -> usize {
    if !init.data_f32.is_empty() {
        return init.data_f32.len();
    }
    if !init.data_i8.is_empty() {
        return init.data_i8.len();
    }
    if !init.data_int32.is_empty() {
        return init.data_int32.len();
    }
    if !init.data_int64.is_empty() {
        return init.data_int64.len();
    }
    if !init.raw_data.is_empty() {
        let es = usize::try_from(onnx_dtype_elem_nbytes(init.data_type)).unwrap_or(1);
        if es == 0 {
            return 0;
        }
        return init.raw_data.len() / es;
    }
    0
}

/// Build a weight BO plan from [`CompiledModel::initializers`] (no hardware, no ioctls).
///
/// `nbytes` matches the bytes that upload will write. Shape/payload mismatches are
/// not fatal here (plan still reports payload size); use [`check_initializer_shape_payload`]
/// or JSON load validation for hard refuse.
#[must_use]
pub fn build_weight_bo_plan(model: &CompiledModel) -> Vec<Xdna2WeightPlan> {
    model
        .initializers()
        .iter()
        .map(|init| Xdna2WeightPlan {
            name: if init.name.is_empty() {
                "(anonymous)".to_string()
            } else {
                init.name.clone()
            },
            dtype: onnx_data_type_name(init.data_type).to_string(),
            shape: init.shape.clone(),
            nbytes: initializer_nbytes(init),
            suggested_bo_type: WEIGHT_BO_TYPE_SHMEM.to_string(),
        })
        .collect()
}

/// Upload each weight as a temporary Shmem BO (create → fill → destroy).
///
/// Bring-up plumbing only. **Does not** bind BOs to a command, run EXEC_CMD, or
/// claim inference.
///
/// On **success**: every created BO was destroyed; returns the number of weights
/// uploaded (handles are **not** live — do not GEM_CLOSE again).
///
/// On **error**: still attempts to destroy every created handle; destroy failures
/// are included in the error message so leaks are not silent.
///
/// # Errors
///
/// Create/write/destroy failures via [`DragonNpuError::BackendError`] / `RuntimeError`.
pub fn upload_weights_to_shmem_bos(
    submission: &dyn DirectSubmission,
    fd: RawFd,
    model: &CompiledModel,
) -> Result<usize, DragonNpuError> {
    let plan = build_weight_bo_plan(model);
    let mut created: Vec<u32> = Vec::new();
    let mut first_err: Option<DragonNpuError> = None;
    let mut uploaded = 0usize;

    for (init, entry) in model.initializers().iter().zip(plan.iter()) {
        if entry.nbytes == 0 {
            continue;
        }
        let size = entry.nbytes.max(1);
        match submission.create_buffer(fd, size, BufferType::Shmem) {
            Ok(bh) => {
                let handle = bh.0;
                created.push(handle);
                let bytes = initializer_host_bytes(init);
                if bytes.is_empty() {
                    first_err.get_or_insert_with(|| {
                        DragonNpuError::RuntimeError(format!(
                            "weight '{}' has nbytes={} but no host bytes to fill BO",
                            entry.name, entry.nbytes
                        ))
                    });
                    continue;
                }
                match submission.write_bo_bytes(fd, handle, 0, &bytes) {
                    Ok(()) => {
                        uploaded = uploaded.saturating_add(1);
                    }
                    Err(e) => {
                        first_err.get_or_insert_with(|| DragonNpuError::BackendError {
                            backend: "amd-xdna2".to_string(),
                            message: format!("write_bo_bytes weight '{}': {e}", entry.name),
                        });
                    }
                }
            }
            Err(e) => {
                first_err.get_or_insert_with(|| DragonNpuError::BackendError {
                    backend: "amd-xdna2".to_string(),
                    message: format!("create_buffer weight '{}': {e}", entry.name),
                });
            }
        }
    }

    // Always free BOs we created (no silent resource leak).
    let mut destroy_errs: Vec<String> = Vec::new();
    for h in &created {
        if let Err(e) = submission.destroy_buffer(fd, *h) {
            destroy_errs.push(format!("destroy_buffer handle={h}: {e}"));
        }
    }

    if let Some(err) = first_err {
        if destroy_errs.is_empty() {
            return Err(err);
        }
        return Err(DragonNpuError::BackendError {
            backend: "amd-xdna2".to_string(),
            message: format!("{err}; cleanup after error: {}", destroy_errs.join("; ")),
        });
    }
    if !destroy_errs.is_empty() {
        return Err(DragonNpuError::BackendError {
            backend: "amd-xdna2".to_string(),
            message: format!(
                "weight upload fill OK but cleanup failed (possible BO leak): {}",
                destroy_errs.join("; ")
            ),
        });
    }
    // Success: no live handles returned (all destroyed).
    Ok(uploaded)
}

/// Host bytes that would be written into a Shmem BO for this initializer.
#[must_use]
pub fn initializer_host_bytes(init: &Initializer) -> Vec<u8> {
    if !init.raw_data.is_empty() {
        return init.raw_data.clone();
    }
    if !init.data_f32.is_empty() {
        let mut out = Vec::with_capacity(init.data_f32.len() * 4);
        for v in &init.data_f32 {
            out.extend_from_slice(&v.to_le_bytes());
        }
        return out;
    }
    if !init.data_i8.is_empty() {
        return init.data_i8.iter().map(|v| (*v).cast_unsigned()).collect();
    }
    if !init.data_int32.is_empty() {
        let mut out = Vec::with_capacity(init.data_int32.len() * 4);
        for v in &init.data_int32 {
            out.extend_from_slice(&v.to_le_bytes());
        }
        return out;
    }
    if !init.data_int64.is_empty() {
        let mut out = Vec::with_capacity(init.data_int64.len() * 8);
        for v in &init.data_int64 {
            out.extend_from_slice(&v.to_le_bytes());
        }
        return out;
    }
    // Last resort: f32 view (bf16/fp16 approx already applied in as_f32_data).
    let f = init.as_f32_data();
    let mut out = Vec::with_capacity(f.len() * 4);
    for v in f {
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

fn normalize_op(op: &str) -> String {
    op.trim().to_ascii_lowercase()
}

/// Ops with real host implementations under Phase 5–6 `host_ref`.
///
/// Includes compiler Phase 6 linear fused names so `optimization_level >= 1`
/// graphs remain evaluable (Issue 2). `fused_conv_*` is **not** host-evaluable.
pub const HOST_GRAPH_OP_ALLOWLIST: &[&str] = &[
    "identity",
    "relu",
    "add",
    "mul",
    "matmul",
    "gemm",
    // Phase 7 IR constant_fold result (loads initializer as activation).
    "constant",
    // Compiler fuse_consecutive output (lowercase); host maps to MatMul/Gemm+Add+Relu.
    "fused_matmul_relu",
    "fused_gemm_relu",
    "fused_matmul_add",
    "fused_gemm_add",
    "fused_matmul_add_relu",
    "fused_gemm_add_relu",
];

/// True when every non-empty op is in [`HOST_GRAPH_OP_ALLOWLIST`].
#[must_use]
pub fn host_graph_ops_supported(ops: &[String]) -> bool {
    if ops.is_empty() {
        return false;
    }
    ops.iter().all(|op| {
        let n = normalize_op(op);
        n.is_empty() || HOST_GRAPH_OP_ALLOWLIST.contains(&n.as_str())
    })
}

/// Execute the compiled graph on the host (`f32`). **Not NPU.**
///
/// Uses [`HostGraphOptions::default`] (no ambient env). For CLI env overrides use
/// [`evaluate_host_graph_with_options`] with [`HostGraphOptions::from_env`].
///
/// # Errors
///
/// - Empty input
/// - Unsupported ops
/// - Missing / shape-mismatched weights for Add/Mul/MatMul/Gemm
pub fn evaluate_host_graph(
    model: &CompiledModel,
    input: &[f32],
) -> Result<Vec<f32>, DragonNpuError> {
    evaluate_host_graph_with_options(model, input, &HostGraphOptions::default())
}

/// Like [`evaluate_host_graph`] with explicit options (e.g. MatMul `K` override).
///
/// # Errors
///
/// Same as [`evaluate_host_graph`].
#[allow(clippy::too_many_lines)] // Phase 6 fused linear op arms + look-ahead Relu
pub fn evaluate_host_graph_with_options(
    model: &CompiledModel,
    input: &[f32],
    opts: &HostGraphOptions,
) -> Result<Vec<f32>, DragonNpuError> {
    if input.is_empty() {
        return Err(DragonNpuError::RuntimeError(
            "empty input tensor (host_graph)".to_string(),
        ));
    }
    let graph_ops = model.ops();
    if graph_ops.is_empty() {
        return Err(DragonNpuError::UnsupportedOps {
            model: model.model_path().to_string(),
            ops: "(none — empty graph)".to_string(),
        });
    }
    if !host_graph_ops_supported(graph_ops) {
        let bad: Vec<String> = graph_ops
            .iter()
            .map(|o| normalize_op(o))
            .filter(|n| !n.is_empty() && !HOST_GRAPH_OP_ALLOWLIST.contains(&n.as_str()))
            .fold(Vec::new(), |mut acc, n| {
                if !acc.contains(&n) {
                    acc.push(n);
                }
                acc
            });
        return Err(DragonNpuError::UnsupportedOps {
            model: model.model_path().to_string(),
            ops: bad.join(", "),
        });
    }

    let mut act = input.to_vec();
    let mut used_inits: HashSet<String> = HashSet::new();
    let mut next_init = 0usize;
    let mut i = 0usize;
    while i < graph_ops.len() {
        let n = normalize_op(&graph_ops[i]);
        match n.as_str() {
            "identity" => {
                i += 1;
            }
            "constant" => {
                // Phase 7: IR fold materializes output as initializer; replace act.
                act = resolve_constant_activation(model, i, &mut used_inits)?;
                i += 1;
            }
            "relu" => {
                for x in &mut act {
                    *x = x.max(0.0);
                }
                i += 1;
            }
            "add" => {
                let rhs = resolve_binary_rhs(model, i, &mut next_init, &mut used_inits)?;
                apply_binary_elem(&mut act, &rhs, BinaryOp::Add, model.model_path())?;
                i += 1;
            }
            "mul" => {
                let rhs = resolve_binary_rhs(model, i, &mut next_init, &mut used_inits)?;
                apply_binary_elem(&mut act, &rhs, BinaryOp::Mul, model.model_path())?;
                i += 1;
            }
            // Plain + look-ahead Relu, and compiler fused linear names (Issue 2).
            "matmul"
            | "gemm"
            | "fused_matmul_relu"
            | "fused_gemm_relu"
            | "fused_matmul_add"
            | "fused_gemm_add"
            | "fused_matmul_add_relu"
            | "fused_gemm_add_relu" => {
                let is_gemm = n == "gemm"
                    || n == "fused_gemm_relu"
                    || n == "fused_gemm_add"
                    || n == "fused_gemm_add_relu";
                let compiler_fused_relu = n.ends_with("_relu");
                let compiler_fused_add = n.contains("_add");
                // Runtime look-ahead only for plain matmul/gemm (not already-fused names).
                let look_ahead_relu = !n.starts_with("fused_")
                    && graph_ops
                        .get(i + 1)
                        .is_some_and(|next| normalize_op(next) == "relu");
                let apply_relu = compiler_fused_relu || look_ahead_relu;

                let w = resolve_matmul_weight(model, i, &mut next_init, &mut used_inits)?;
                let gemm_attrs = if is_gemm {
                    model.node_attrs().get(i).cloned().unwrap_or_default()
                } else {
                    crate::ir::NodeAttrs::default()
                };
                // MatMul (+ fused_matmul_* without gemm): optional Relu on write when no
                // intervening bias/add. Gemm / fused_*_add need bias/add before Relu.
                let relu_on_write = apply_relu && !is_gemm && !compiler_fused_add;
                act = matmul_host(
                    &act,
                    w,
                    model.model_path(),
                    opts.matmul_k,
                    relu_on_write,
                    if is_gemm { Some(&gemm_attrs) } else { None },
                )?;

                if is_gemm {
                    // Named third-input bias only (never sequential steal).
                    // ONNX Gemm: C' = alpha*A*B + beta*C
                    let beta = gemm_attrs.beta.unwrap_or(1.0);
                    if let Some(bias) = resolve_named_bias_only(model, i, &mut used_inits)? {
                        let mut scaled = bias;
                        if (beta - 1.0).abs() > f32::EPSILON {
                            for x in &mut scaled {
                                *x *= beta;
                            }
                        }
                        apply_bias(&mut act, &scaled, model.model_path())?;
                    } else if (beta - 1.0).abs() > f32::EPSILON && beta.abs() > f32::EPSILON {
                        // No bias tensor but beta≠1: nothing to scale (C absent → 0 in ONNX).
                    }
                }
                if compiler_fused_add {
                    // Extra Add RHS after weight (merged fuse inputs / sequential unused).
                    let rhs = resolve_binary_rhs(model, i, &mut next_init, &mut used_inits)?;
                    apply_binary_elem(&mut act, &rhs, BinaryOp::Add, model.model_path())?;
                }
                if apply_relu && !relu_on_write {
                    for x in &mut act {
                        *x = x.max(0.0);
                    }
                }
                i += if look_ahead_relu { 2 } else { 1 };
            }
            other => {
                return Err(DragonNpuError::UnsupportedOps {
                    model: model.model_path().to_string(),
                    ops: other.to_string(),
                });
            }
        }
    }
    Ok(act)
}

#[derive(Clone, Copy)]
enum BinaryOp {
    Add,
    Mul,
}

fn node_input_names(model: &CompiledModel, node_idx: usize) -> &[String] {
    model.inputs().get(node_idx).map_or(&[], Vec::as_slice)
}

fn find_init_by_name<'a>(model: &'a CompiledModel, name: &str) -> Option<&'a Initializer> {
    if name.is_empty() {
        return None;
    }
    model.initializers().iter().find(|i| i.name == name)
}

fn take_next_unused_init<'a>(
    model: &'a CompiledModel,
    next_init: &mut usize,
    used: &mut HashSet<String>,
) -> Result<&'a Initializer, DragonNpuError> {
    let inits = model.initializers();
    while *next_init < inits.len() {
        let init = &inits[*next_init];
        *next_init += 1;
        let key = if init.name.is_empty() {
            format!("#{}", *next_init - 1)
        } else {
            init.name.clone()
        };
        if used.insert(key) {
            return Ok(init);
        }
    }
    Err(DragonNpuError::RuntimeError(format!(
        "host_graph: no unused initializer remaining for model '{}' (need weight/RHS tensor; \
         refuse fake golden when weights missing)",
        model.model_path()
    )))
}

fn resolve_binary_rhs(
    model: &CompiledModel,
    node_idx: usize,
    next_init: &mut usize,
    used: &mut HashSet<String>,
) -> Result<Vec<f32>, DragonNpuError> {
    // Prefer a named initializer among node inputs (skip first = activation).
    // Skip names already in `used` (Phase 6 fused_matmul_add merges ["x","W"]+["y","bias"]
    // → ["x","W","y","bias"]; after MatMul consumes W, Add must not re-pick W as RHS).
    let names = node_input_names(model, node_idx);
    for name in names.iter().skip(1) {
        if name.is_empty() || used.contains(name.as_str()) {
            continue;
        }
        if let Some(init) = find_init_by_name(model, name) {
            used.insert(init.name.clone());
            let data = init.as_f32_data();
            if data.is_empty() {
                return Err(DragonNpuError::RuntimeError(format!(
                    "host_graph Add/Mul: initializer '{name}' has no f32 data"
                )));
            }
            return Ok(data);
        }
    }
    let init = take_next_unused_init(model, next_init, used)?;
    let data = init.as_f32_data();
    if data.is_empty() {
        return Err(DragonNpuError::RuntimeError(format!(
            "host_graph Add/Mul: initializer '{}' has no f32 data",
            init.name
        )));
    }
    Ok(data)
}

fn resolve_matmul_weight<'a>(
    model: &'a CompiledModel,
    node_idx: usize,
    next_init: &mut usize,
    used: &mut HashSet<String>,
) -> Result<&'a Initializer, DragonNpuError> {
    let names = node_input_names(model, node_idx);
    // Second input is typically the weight matrix.
    if let Some(name) = names.get(1)
        && let Some(init) = find_init_by_name(model, name)
    {
        used.insert(init.name.clone());
        return Ok(init);
    }
    // Any named input that is an initializer (except first).
    for name in names.iter().skip(1) {
        if let Some(init) = find_init_by_name(model, name) {
            used.insert(init.name.clone());
            return Ok(init);
        }
    }
    take_next_unused_init(model, next_init, used)
}

/// Gemm bias: **named third input only**. Never consumes sequential unused
/// initializers (would steal the next MatMul/Gemm weight on multi-op graphs).
fn resolve_named_bias_only(
    model: &CompiledModel,
    node_idx: usize,
    used: &mut HashSet<String>,
) -> Result<Option<Vec<f32>>, DragonNpuError> {
    let names = node_input_names(model, node_idx);
    let Some(name) = names.get(2) else {
        return Ok(None);
    };
    if name.is_empty() {
        return Ok(None);
    }
    let Some(init) = find_init_by_name(model, name) else {
        // Third name present but not an initializer (e.g. activation alias) → no bias.
        return Ok(None);
    };
    used.insert(init.name.clone());
    let d = init.as_f32_data();
    if d.is_empty() {
        return Err(DragonNpuError::RuntimeError(format!(
            "host_graph Gemm: named bias '{name}' has no f32 data"
        )));
    }
    Ok(Some(d))
}

fn apply_binary_elem(
    act: &mut [f32],
    rhs: &[f32],
    op: BinaryOp,
    model: &str,
) -> Result<(), DragonNpuError> {
    if rhs.len() == 1 {
        let s = rhs[0];
        match op {
            BinaryOp::Add => {
                for x in act.iter_mut() {
                    *x += s;
                }
            }
            BinaryOp::Mul => {
                for x in act.iter_mut() {
                    *x *= s;
                }
            }
        }
        return Ok(());
    }
    if rhs.len() != act.len() {
        return Err(DragonNpuError::RuntimeError(format!(
            "host_graph binary op length mismatch on model '{model}': act={} rhs={} \
             (need equal lengths or scalar rhs)",
            act.len(),
            rhs.len()
        )));
    }
    match op {
        BinaryOp::Add => {
            for (x, r) in act.iter_mut().zip(rhs.iter()) {
                *x += *r;
            }
        }
        BinaryOp::Mul => {
            for (x, r) in act.iter_mut().zip(rhs.iter()) {
                *x *= *r;
            }
        }
    }
    Ok(())
}

fn apply_bias(act: &mut [f32], bias: &[f32], model: &str) -> Result<(), DragonNpuError> {
    if bias.len() == act.len() {
        for (x, b) in act.iter_mut().zip(bias.iter()) {
            *x += *b;
        }
        return Ok(());
    }
    // Per-column bias: N columns, M rows row-major.
    if !bias.is_empty() && act.len().is_multiple_of(bias.len()) {
        let n = bias.len();
        for (i, x) in act.iter_mut().enumerate() {
            *x += bias[i % n];
        }
        return Ok(());
    }
    Err(DragonNpuError::RuntimeError(format!(
        "host_graph Gemm bias shape mismatch on model '{model}': act={} bias={}",
        act.len(),
        bias.len()
    )))
}

fn parse_host_matmul_k_env() -> Option<usize> {
    std::env::var("XDNA2_HOST_MATMUL_K").ok().and_then(|s| {
        let t = s.trim();
        if t.is_empty() {
            return None;
        }
        t.parse::<usize>().ok().filter(|&k| k > 0)
    })
}

/// Phase 6: prefer zero-copy / once-converted weight views for host MatMul.
///
/// Priority:
/// 1. `data_f32` slice (no clone)
/// 2. `data_i8` (int8/uint8) — multiply with `f32::from` in the kernel (no full temp vec)
/// 3. else convert once via [`Initializer::as_f32_data`] (bf16/fp16/int32/…) and cache in
///    the returned owned buffer for this op only
#[derive(Debug)]
enum WeightF32View<'a> {
    /// Borrowed float32 payload.
    F32(&'a [f32]),
    /// INT8 payload; signed promote to f32 at multiply time.
    I8(&'a [i8]),
    /// UINT8 payload stored as `i8` bytes; **unsigned** promote (`i8 as u8` → f32).
    U8(&'a [i8]),
    /// Owned conversion (bf16/fp16/etc.) — one allocation per MatMul/Gemm.
    Owned(Vec<f32>),
}

impl WeightF32View<'_> {
    fn len(&self) -> usize {
        match self {
            Self::F32(s) => s.len(),
            Self::I8(s) | Self::U8(s) => s.len(),
            Self::Owned(v) => v.len(),
        }
    }

    #[inline]
    fn get_f32(&self, idx: usize) -> f32 {
        match self {
            Self::F32(s) => s[idx],
            Self::I8(s) => f32::from(s[idx]),
            // UINT8: byte value 200 must be 200.0, not -56.0 (Issue 3).
            Self::U8(s) => f32::from(s[idx].cast_unsigned()),
            Self::Owned(v) => v[idx],
        }
    }
}

/// Build a weight view for host MatMul (convert at most once).
fn weight_f32_view(weight: &Initializer) -> WeightF32View<'_> {
    if !weight.data_f32.is_empty() {
        return WeightF32View::F32(&weight.data_f32);
    }
    // Prefer int8/uint8 path when typed i8 storage is present.
    if !weight.data_i8.is_empty() {
        match weight.data_type {
            2 => return WeightF32View::U8(&weight.data_i8), // UINT8
            3 | 0 => return WeightF32View::I8(&weight.data_i8), // INT8 / default i8
            _ => {}
        }
    }
    WeightF32View::Owned(weight.as_f32_data())
}

/// Load Constant op activation from initializer (Phase 7 fold / explicit Constant).
///
/// Prefer named match: first **output** name, then first **input** name, else refuse
/// (no sequential steal of unrelated weights).
fn resolve_constant_activation(
    model: &CompiledModel,
    node_idx: usize,
    used: &mut HashSet<String>,
) -> Result<Vec<f32>, DragonNpuError> {
    let outs = model.outputs().get(node_idx).map_or(&[][..], Vec::as_slice);
    let ins = node_input_names(model, node_idx);
    let candidates = outs.iter().chain(ins.iter());
    for name in candidates {
        if name.is_empty() {
            continue;
        }
        if let Some(init) = find_init_by_name(model, name) {
            let data = init.as_f32_data();
            if data.is_empty() {
                return Err(DragonNpuError::RuntimeError(format!(
                    "host_graph Constant: initializer '{name}' has no f32 data model='{}'",
                    model.model_path()
                )));
            }
            used.insert(name.clone());
            return Ok(data);
        }
    }
    Err(DragonNpuError::RuntimeError(format!(
        "host_graph Constant: no matching initializer for node {node_idx} \
         (outputs={outs:?} inputs={ins:?}) model='{}'",
        model.model_path()
    )))
}

/// `A[M,K] @ B[K,N]` row-major → `C[M,N]`.
///
/// When `apply_relu` is true, writes `max(0, acc)` into the output (Phase 6 fused MatMul+Relu).
/// Optional `gemm_attrs` applies ONNX Gemm `alpha` / `transA` / `transB` (Issue 4).
fn matmul_host(
    act: &[f32],
    weight: &Initializer,
    model: &str,
    matmul_k: Option<usize>,
    apply_relu: bool,
    gemm_attrs: Option<&crate::ir::NodeAttrs>,
) -> Result<Vec<f32>, DragonNpuError> {
    let wview = weight_f32_view(weight);
    if wview.len() == 0 {
        return Err(DragonNpuError::RuntimeError(format!(
            "host_graph MatMul: weight '{}' has no f32 data (refuse fake golden) model='{model}'",
            weight.name
        )));
    }

    let trans_a = gemm_attrs.and_then(|a| a.trans_a).unwrap_or(0) != 0;
    let trans_b = gemm_attrs.and_then(|a| a.trans_b).unwrap_or(0) != 0;
    let alpha = gemm_attrs.and_then(|a| a.alpha).unwrap_or(1.0);

    let b_len = wview.len();
    let (inner, out_cols) = if trans_b {
        // B stored as [N, K] when transB=1 (ONNX): treat shape [N,K] → K=shape[1], N=shape[0].
        resolve_weight_kn_transposed(weight, b_len, model, matmul_k)?
    } else {
        resolve_weight_kn(weight, b_len, model, matmul_k)?
    };
    if inner == 0 || out_cols == 0 {
        return Err(DragonNpuError::RuntimeError(format!(
            "host_graph MatMul: invalid K={inner} N={out_cols} model='{model}'"
        )));
    }
    if b_len != inner.saturating_mul(out_cols) {
        return Err(DragonNpuError::RuntimeError(format!(
            "host_graph MatMul: weight elems {b_len} != K*N={inner}*{out_cols} model='{model}'"
        )));
    }

    // transA: act is [K,M] instead of [M,K] → M = len/K still, but indexing swaps.
    let act_inner = if trans_a {
        // A is [K, M] with K=inner → len must be K*M.
        if !act.len().is_multiple_of(inner) {
            return Err(DragonNpuError::RuntimeError(format!(
                "host_graph Gemm transA: activation len {} not divisible by K={inner} model='{model}'",
                act.len()
            )));
        }
        inner
    } else if !act.len().is_multiple_of(inner) {
        return Err(DragonNpuError::RuntimeError(format!(
            "host_graph MatMul: activation len {} not divisible by K={} \
             (expected M*K row-major) model='{model}'",
            act.len(),
            inner
        )));
    } else {
        inner
    };
    let rows = act.len() / act_inner;
    let work = (rows as u64)
        .saturating_mul(out_cols as u64)
        .saturating_mul(inner as u64);
    if work > HOST_GRAPH_MAX_ELEMS {
        return Err(DragonNpuError::RuntimeError(format!(
            "host_graph MatMul: M*N*K={work} exceeds max {HOST_GRAPH_MAX_ELEMS}"
        )));
    }

    let mut out = vec![0.0_f32; rows.saturating_mul(out_cols)];
    for row in 0..rows {
        for col in 0..out_cols {
            let mut acc = 0.0_f32;
            for kk in 0..inner {
                let av = if trans_a {
                    // A[K,M] row-major: index (kk, row)
                    act[kk * rows + row]
                } else {
                    act[row * inner + kk]
                };
                let bv = if trans_b {
                    // B[N,K] row-major: index (col, kk)
                    wview.get_f32(col * inner + kk)
                } else {
                    wview.get_f32(kk * out_cols + col)
                };
                acc = f32::mul_add(av, bv, acc);
            }
            acc *= alpha;
            out[row * out_cols + col] = if apply_relu { acc.max(0.0) } else { acc };
        }
    }
    Ok(out)
}

/// Like [`resolve_weight_kn`] but for `transB=1`: weight shape `[N, K]` (ONNX).
fn resolve_weight_kn_transposed(
    weight: &Initializer,
    b_len: usize,
    model: &str,
    matmul_k: Option<usize>,
) -> Result<(usize, usize), DragonNpuError> {
    if weight.shape.len() >= 2 {
        let n = weight.shape[weight.shape.len() - 2];
        let k = weight.shape[weight.shape.len() - 1];
        if n > 0 && k > 0 {
            let nu = usize::try_from(n).unwrap_or(0);
            let ku = usize::try_from(k).unwrap_or(0);
            if nu.saturating_mul(ku) == b_len {
                return Ok((ku, nu)); // (K, N)
            }
        }
    }
    if let Some(k) = matmul_k.filter(|&kk| kk > 0 && b_len.is_multiple_of(kk)) {
        return Ok((k, b_len / k));
    }
    Err(DragonNpuError::RuntimeError(format!(
        "host_graph Gemm transB: cannot resolve K,N for weight '{}' shape={:?} elems={b_len} model='{model}'",
        weight.name, weight.shape
    )))
}

// ---------------------------------------------------------------------------
// Lab MatMul weight selection (pure — bridges host_graph → weight EXEC bind)
// ---------------------------------------------------------------------------

/// Pure selection of the first MatMul/Gemm weight for PARTIAL_ELF lab WTS bind.
///
/// **Not** multi-weight EXEC. Bias tensors are not selected as WTS.
/// Lab path expects f32 LE WTS matching [`super::lab_io::lab_matmul_wts_nbytes`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabMatmulWeightSpec {
    /// Initializer name selected as WTS.
    pub weight_name: String,
    /// Weight rows (K) for row-major `[K,N]`.
    pub k: u32,
    /// Weight cols (N).
    pub n: u32,
    /// Host WTS bytes as **f32 LE** (promoted from int8/bf16/fp16 when needed).
    pub host_wts_bytes: Vec<u8>,
}

fn is_matmul_like_op(op: &str) -> bool {
    matches!(
        normalize_op(op).as_str(),
        "matmul"
            | "gemm"
            | "fused_matmul_relu"
            | "fused_gemm_relu"
            | "fused_matmul_add"
            | "fused_gemm_add"
            | "fused_matmul_add_relu"
            | "fused_gemm_add_relu"
    )
}

/// Select first MatMul/Gemm weight + `(K,N)` for ones-compatible lab EXEC bind.
///
/// Uses the same named-second-input / sequential rules as host MatMul. Emits
/// f32 LE `host_wts_bytes` so nbytes match [`super::lab_io::lab_matmul_wts_nbytes`].
/// Honors Gemm `transB` (logical `[K,N]` layout after transpose of stored `[N,K]`).
///
/// # Errors
///
/// No MatMul-like op, missing weight, unresolvable shape, or empty f32 payload.
#[allow(clippy::too_many_lines)] // Gemm transB + sequential init walk + LE pack
pub fn select_lab_matmul_weight(
    model: &CompiledModel,
    opts: &HostGraphOptions,
) -> Result<LabMatmulWeightSpec, DragonNpuError> {
    let graph_ops = model.ops();
    let mut next_init = 0usize;
    let mut used: HashSet<String> = HashSet::new();
    let mut node_idx = None;
    for (i, op) in graph_ops.iter().enumerate() {
        if is_matmul_like_op(op) {
            node_idx = Some(i);
            break;
        }
        // Advance sequential cursor for prior ops that consume inits (best-effort).
        let n = normalize_op(op);
        if matches!(n.as_str(), "add" | "mul") {
            let _ = resolve_binary_rhs(model, i, &mut next_init, &mut used);
        } else if n == "constant" {
            let _ = resolve_constant_activation(model, i, &mut used);
        }
    }
    let i = node_idx.ok_or_else(|| {
        DragonNpuError::RuntimeError(format!(
            "select_lab_matmul_weight: no MatMul/Gemm op in model '{}'",
            model.model_path()
        ))
    })?;
    let weight = resolve_matmul_weight(model, i, &mut next_init, &mut used)?;
    let f32_data = weight.as_f32_data();
    if f32_data.is_empty() {
        return Err(DragonNpuError::RuntimeError(format!(
            "select_lab_matmul_weight: weight '{}' has no f32 data model='{}'",
            weight.name,
            model.model_path()
        )));
    }
    // Honor Gemm (and fused_gemm_*) transB like host matmul_host.
    let op_n = normalize_op(&graph_ops[i]);
    let is_gemm = op_n == "gemm"
        || op_n == "fused_gemm_relu"
        || op_n == "fused_gemm_add"
        || op_n == "fused_gemm_add_relu";
    let trans_b = if is_gemm {
        model
            .node_attrs()
            .get(i)
            .and_then(|a| a.trans_b)
            .unwrap_or(0)
            != 0
    } else {
        false
    };
    let (k_usz, n_usz) = if trans_b {
        resolve_weight_kn_transposed(weight, f32_data.len(), model.model_path(), opts.matmul_k)?
    } else {
        resolve_weight_kn(weight, f32_data.len(), model.model_path(), opts.matmul_k)?
    };
    let k = u32::try_from(k_usz).map_err(|_| {
        DragonNpuError::RuntimeError(format!(
            "select_lab_matmul_weight: K={k_usz} does not fit u32"
        ))
    })?;
    let n = u32::try_from(n_usz).map_err(|_| {
        DragonNpuError::RuntimeError(format!(
            "select_lab_matmul_weight: N={n_usz} does not fit u32"
        ))
    })?;
    if k == 0 || n == 0 {
        return Err(DragonNpuError::RuntimeError(format!(
            "select_lab_matmul_weight: invalid K={k} N={n} model='{}'",
            model.model_path()
        )));
    }
    // f32 LE for lab WTS in **logical** row-major [K,N] order (transB weights are
    // re-laid out so PARTIAL_ELF WTS nbytes match lab_matmul_wts_nbytes(k,n)).
    let host_wts_bytes = if trans_b {
        // Stored as [N,K] row-major → emit [K,N] for lab bind.
        let mut out = vec![0u8; f32_data.len().saturating_mul(4)];
        for row_n in 0..n_usz {
            for col_k in 0..k_usz {
                let src = f32_data[row_n * k_usz + col_k];
                let dst_idx = (col_k * n_usz + row_n).saturating_mul(4);
                out[dst_idx..dst_idx + 4].copy_from_slice(&src.to_le_bytes());
            }
        }
        out
    } else {
        let mut out = Vec::with_capacity(f32_data.len().saturating_mul(4));
        for v in &f32_data {
            out.extend_from_slice(&v.to_le_bytes());
        }
        out
    };
    let expected = u64::from(k)
        .checked_mul(u64::from(n))
        .and_then(|e| e.checked_mul(4))
        .ok_or_else(|| {
            DragonNpuError::RuntimeError("select_lab_matmul_weight: nbytes overflow".into())
        })?;
    if host_wts_bytes.len() as u64 != expected {
        return Err(DragonNpuError::RuntimeError(format!(
            "select_lab_matmul_weight: f32 LE len {} != K*N*4={expected} model='{}'",
            host_wts_bytes.len(),
            model.model_path()
        )));
    }
    Ok(LabMatmulWeightSpec {
        weight_name: if weight.name.is_empty() {
            "W".to_string()
        } else {
            weight.name.clone()
        },
        k,
        n,
        host_wts_bytes,
    })
}

/// Pure preflight: model-WTS lab requires OFM path (`XDNA2_LAB_OFM=1`).
///
/// # Errors
///
/// [`DirectError::InvalidArgument`] when weight env is on without OFM.
pub fn lab_weight_requires_ofm(
    lab_ofm: bool,
    lab_weight: bool,
) -> Result<(), super::direct::DirectError> {
    if lab_weight && !lab_ofm {
        return Err(super::direct::DirectError::InvalidArgument(
            "XDNA2_LAB_WEIGHT=1 requires XDNA2_LAB_OFM=1; refuse fail-closed".into(),
        ));
    }
    Ok(())
}

/// MVP limits for `XDNA2_LAB_WEIGHT` timed path (intentional; not full host_graph).
///
/// - **f32 LE WTS only** (promoted from int8/bf16/fp16 via `as_f32_data`)
/// - **ones IFM** golden: `ones(m,k) @ W[k,n]` — not model activations
/// - **no Gemm alpha/beta ≠ 1**, no named bias, no fused linear ops
/// - first MatMul/Gemm only; device txn must match this contract for OFM Pass
///
/// See also [`check_lab_weight_mvp_model`].
pub const LAB_WEIGHT_MVP_DOC: &str = "\
LAB_WEIGHT MVP: f32 LE WTS, ones IFM golden, plain MatMul/Gemm only \
(no fused_*, no Gemm alpha/beta≠1, no bias BO)";

/// Refuse models that exceed LAB_WEIGHT MVP (fused / non-default Gemm attrs).
///
/// # Errors
///
/// Fused ops or Gemm with alpha/beta ≠ 1 or named third-input bias.
pub fn check_lab_weight_mvp_model(model: &CompiledModel) -> Result<(), DragonNpuError> {
    let ops = model.ops();
    for (i, op) in ops.iter().enumerate() {
        let n = normalize_op(op);
        if n.starts_with("fused_") {
            return Err(DragonNpuError::RuntimeError(format!(
                "XDNA2_LAB_WEIGHT MVP refuses fused op '{op}' ({LAB_WEIGHT_MVP_DOC})"
            )));
        }
        if n == "gemm" {
            let attrs = model.node_attrs().get(i).cloned().unwrap_or_default();
            let alpha = attrs.alpha.unwrap_or(1.0);
            let beta = attrs.beta.unwrap_or(1.0);
            if (alpha - 1.0).abs() > f32::EPSILON || (beta - 1.0).abs() > f32::EPSILON {
                return Err(DragonNpuError::RuntimeError(format!(
                    "XDNA2_LAB_WEIGHT MVP refuses Gemm alpha={alpha}/beta={beta} \
                     (need defaults 1.0); {LAB_WEIGHT_MVP_DOC}"
                )));
            }
            // Named third-input bias not bound into PARTIAL_ELF WTS slot.
            if let Some(bias_name) = model.inputs().get(i).and_then(|ins| ins.get(2))
                && !bias_name.is_empty()
                && model
                    .initializers()
                    .iter()
                    .any(|init| init.name == *bias_name)
            {
                return Err(DragonNpuError::RuntimeError(format!(
                    "XDNA2_LAB_WEIGHT MVP refuses Gemm named bias '{bias_name}' \
                     (no bias BO in kernel args); {LAB_WEIGHT_MVP_DOC}"
                )));
            }
        }
    }
    Ok(())
}

/// Create live lab I/O BOs from a model path for timed matmul dims.
///
/// Requires model MatMul weight `(K,N)` to match timed `k,n`. IFM is ones;
/// WTS is model f32 LE; OFM host-zeroed (full alloc). Returns bind + host golden
/// for ones×W. See [`LAB_WEIGHT_MVP_DOC`].
///
/// Caller must [`super::lab_io::destroy_weight_exec_bind`]. **Not** RAII. Never EXEC.
///
/// # Errors
///
/// Compile/select/MVP refuse, dim mismatch, create/fill failure.
pub fn create_lab_weight_io_from_model_path(
    submission: &dyn DirectSubmission,
    model_path: &str,
    m: u32,
    n: u32,
    k: u32,
    dtype: &str,
) -> Result<(super::lab_io::WeightExecBind, Vec<f32>), DragonNpuError> {
    if model_path.trim().is_empty() {
        return Err(DragonNpuError::RuntimeError(
            "create_lab_weight_io_from_model_path: empty model path".into(),
        ));
    }
    // Simulation backend shell — host_graph only needs ops/initializers from compile.
    let backend: std::sync::Arc<dyn crate::backends::Backend> =
        std::sync::Arc::new(crate::backends::simulation::SimulationBackend::default());
    let model = crate::ModelCompiler::new().compile(model_path, 0, "fp32", backend)?;
    check_lab_weight_mvp_model(&model)?;
    let spec = select_lab_matmul_weight(&model, &HostGraphOptions::default())?;
    if spec.k != k || spec.n != n {
        return Err(DragonNpuError::RuntimeError(format!(
            "XDNA2_LAB_WEIGHT model weight K,N=({},{}) != timed k,n=({k},{n}); refuse fail-closed",
            spec.k, spec.n
        )));
    }
    let ifm = super::lab_io::host_ones_f32_matrix_bytes(m, k)
        .map_err(|e| DragonNpuError::RuntimeError(format!("lab weight IFM ones: {e}")))?;
    let req = weight_exec_fill_request_from_spec(&spec, m, ifm, dtype)?;
    let bind = super::lab_io::create_and_fill_weight_io_bos(submission, &req).map_err(|e| {
        DragonNpuError::BackendError {
            backend: "amd-xdna2".into(),
            message: format!("create_and_fill_weight_io_bos: {e}"),
        }
    })?;
    let wts_f32 = super::lab_io::f32_slice_from_le_bytes(&spec.host_wts_bytes)
        .map_err(|e| DragonNpuError::RuntimeError(format!("decode WTS f32: {e}")))?;
    let golden = super::lab_io::host_matmul_f32_ones_ifm_with_wts(m, n, k, &wts_f32)
        .map_err(|e| DragonNpuError::RuntimeError(format!("model WTS golden: {e}")))?;
    Ok((bind, golden))
}

/// Create live lab I/O BOs from `XDNA2_LAB_WEIGHT_MODEL` for timed matmul dims.
///
/// Thin env wrapper around [`create_lab_weight_io_from_model_path`].
///
/// # Errors
///
/// Missing env path or errors from path-based create.
pub fn create_lab_weight_io_from_env(
    submission: &dyn DirectSubmission,
    m: u32,
    n: u32,
    k: u32,
    dtype: &str,
) -> Result<(super::lab_io::WeightExecBind, Vec<f32>), DragonNpuError> {
    let path = std::env::var("XDNA2_LAB_WEIGHT_MODEL").map_err(|_| {
        DragonNpuError::RuntimeError(
            "XDNA2_LAB_WEIGHT=1 requires XDNA2_LAB_WEIGHT_MODEL=/path/to/model.json \
             (or .onnx); refuse fail-closed"
                .into(),
        )
    })?;
    create_lab_weight_io_from_model_path(submission, path.trim(), m, n, k, dtype)
}

/// Build a [`super::lab_io::WeightExecFillRequest`] from a selected weight + IFM bytes.
///
/// IFM must be `m * k * 4` f32 LE bytes. Does not touch hardware.
///
/// # Errors
///
/// Size mismatch or unsupported dtype.
pub fn weight_exec_fill_request_from_spec(
    spec: &LabMatmulWeightSpec,
    m: u32,
    ifm_bytes: Vec<u8>,
    dtype: &str,
) -> Result<super::lab_io::WeightExecFillRequest, DragonNpuError> {
    let ifm_need = super::lab_io::lab_matmul_ifm_nbytes(m, spec.k, dtype).map_err(|e| {
        DragonNpuError::RuntimeError(format!("weight_exec_fill_request IFM size: {e}"))
    })?;
    let wts_need = super::lab_io::lab_matmul_wts_nbytes(spec.k, spec.n, dtype).map_err(|e| {
        DragonNpuError::RuntimeError(format!("weight_exec_fill_request WTS size: {e}"))
    })?;
    if ifm_bytes.len() as u64 != ifm_need {
        return Err(DragonNpuError::RuntimeError(format!(
            "weight_exec_fill_request: IFM host {} != plan {ifm_need} (m={m} k={})",
            ifm_bytes.len(),
            spec.k
        )));
    }
    if spec.host_wts_bytes.len() as u64 != wts_need {
        return Err(DragonNpuError::RuntimeError(format!(
            "weight_exec_fill_request: WTS host {} != plan {wts_need} (k={} n={})",
            spec.host_wts_bytes.len(),
            spec.k,
            spec.n
        )));
    }
    Ok(super::lab_io::WeightExecFillRequest {
        m,
        n: spec.n,
        k: spec.k,
        dtype: dtype.to_string(),
        ifm_bytes,
        wts_bytes: spec.host_wts_bytes.clone(),
        ifm_name: "ifm".to_string(),
        wts_name: spec.weight_name.clone(),
        ofm_name: "ofm".to_string(),
    })
}

// ---------------------------------------------------------------------------
// IR-native host_ref (lower DragonNpuIR → shell CompiledModel → host_graph)
// ---------------------------------------------------------------------------

/// Pack a host-eval shell [`CompiledModel`] from a [`crate::compiler::ModelGraph`].
///
/// Backend is unused by host math (Simulation stub). **Not NPU.**
pub(crate) fn compiled_model_for_host_eval(
    graph: crate::compiler::ModelGraph,
    model_path: impl Into<String>,
) -> CompiledModel {
    use std::sync::Arc;
    let backend: Arc<dyn crate::backends::Backend> =
        Arc::new(crate::backends::simulation::SimulationBackend::default());
    let n = graph.ops.len();
    let init_count = graph.initializers.len();
    CompiledModel::new(
        model_path.into(),
        0,
        "fp32".to_string(),
        backend,
        vec![],
        0,
        0,
        n,
        graph.memory_plan,
        graph.layout,
        graph.quantization,
        n,
        graph.node_names,
        graph.shapes,
        graph.dtypes,
        graph.inputs,
        graph.outputs,
        graph.layouts,
        init_count,
        graph.initializers,
        graph.ops,
        graph.node_attrs,
    )
}

/// Host-eval a typed [`crate::DragonNpuIR`] graph. **Not NPU.**
///
/// Lowers via [`DragonNpuIR::to_model_graph`] then reuses
/// [`evaluate_host_graph`]. Does **not** claim `ran_on_npu`.
///
/// # Errors
///
/// Same as [`evaluate_host_graph`] (unsupported ops, missing weights, empty input).
pub fn evaluate_host_ir(
    ir: &crate::DragonNpuIR,
    input: &[f32],
) -> Result<Vec<f32>, DragonNpuError> {
    evaluate_host_ir_with_options(ir, input, &HostGraphOptions::default())
}

/// Like [`evaluate_host_ir`] with explicit [`HostGraphOptions`].
///
/// # Errors
///
/// Same as [`evaluate_host_graph_with_options`].
pub fn evaluate_host_ir_with_options(
    ir: &crate::DragonNpuIR,
    input: &[f32],
    opts: &HostGraphOptions,
) -> Result<Vec<f32>, DragonNpuError> {
    let graph = ir.to_model_graph();
    let model = compiled_model_for_host_eval(graph, "<host_ir>");
    evaluate_host_graph_with_options(&model, input, opts)
}

/// True when every IR node op is host-evaluable (allowlist after lower).
#[must_use]
pub fn host_ir_ops_supported(ir: &crate::DragonNpuIR) -> bool {
    let graph = ir.to_model_graph();
    host_graph_ops_supported(&graph.ops)
}

/// Upload-session batch helper: fill many pre-created BOs via one submission object.
///
/// Reuses a single [`DirectSubmission`] (Real: one fd / DevHeap session). Each
/// entry is `(handle, host_bytes)`. Empty payloads are refused. Does **not**
/// create or destroy BOs (caller owns handles — unlike [`upload_weights_to_shmem_bos`]).
///
/// # Partial fill on error
///
/// On the first write failure, returns immediately. **Earlier successful fills
/// remain written** in those BOs; later entries are untouched. There is no
/// automatic rollback — callers that need all-or-nothing must destroy/recreate
/// or re-fill after a failed batch. Error messages include how many fills
/// completed before the failure.
///
/// # Errors
///
/// Empty payload or write failure as [`DragonNpuError::RuntimeError`] /
/// [`DragonNpuError::BackendError`].
pub fn fill_weight_bos_batch(
    submission: &dyn DirectSubmission,
    fd: RawFd,
    fills: &[(u32, &[u8])],
) -> Result<(), DragonNpuError> {
    let mut completed = 0usize;
    for (handle, data) in fills {
        if data.is_empty() {
            return Err(DragonNpuError::RuntimeError(format!(
                "fill_weight_bos_batch: handle={handle} has empty payload (refuse empty BO fill); \
                 completed_fills={completed} (prior BOs may retain written data)"
            )));
        }
        submission
            .write_bo_bytes(fd, *handle, 0, data)
            .map_err(|e| DragonNpuError::BackendError {
                backend: "amd-xdna2".to_string(),
                message: format!(
                    "fill_weight_bos_batch write handle={handle}: {e}; \
                     completed_fills={completed} (prior BOs retain written data; no auto-rollback)"
                ),
            })?;
        completed = completed.saturating_add(1);
    }
    Ok(())
}

fn resolve_weight_kn(
    weight: &Initializer,
    b_len: usize,
    model: &str,
    matmul_k: Option<usize>,
) -> Result<(usize, usize), DragonNpuError> {
    if weight.shape.len() == 2 {
        let k = dim_usize(weight.shape[0], "K", model)?;
        let n = dim_usize(weight.shape[1], "N", model)?;
        return Ok((k, n));
    }
    if weight.shape.len() == 1 {
        // Vector weight: treat as [K, 1] with K = shape[0].
        let d = dim_usize(weight.shape[0], "dim0", model)?;
        if d == b_len {
            return Ok((d, 1));
        }
    }
    // Explicit option only (library path never reads ambient env here).
    if let Some(k) = matmul_k {
        if b_len.is_multiple_of(k) {
            return Ok((k, b_len / k));
        }
        return Err(DragonNpuError::RuntimeError(format!(
            "host_graph MatMul: matmul_k={k} does not divide weight len {b_len} model='{model}'"
        )));
    }
    Err(DragonNpuError::RuntimeError(format!(
        "host_graph MatMul: need rank-2 weight shape [K,N] (got {:?}, elems={b_len}); \
         set shape on initializer or HostGraphOptions.matmul_k model='{model}'",
        weight.shape
    )))
}

fn dim_usize(d: i64, name: &str, model: &str) -> Result<usize, DragonNpuError> {
    if d <= 0 {
        return Err(DragonNpuError::RuntimeError(format!(
            "host_graph MatMul: non-positive {name}={d} model='{model}'"
        )));
    }
    usize::try_from(d).map_err(|_| {
        DragonNpuError::RuntimeError(format!(
            "host_graph MatMul: {name}={d} does not fit usize model='{model}'"
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::Backend;
    use crate::backends::amd_xdna2::AmdXdna2Backend;
    use crate::backends::amd_xdna2::direct::{
        BufferHandle, BufferType, Command, DirectError, DirectSubmission, HardwareContext, QosInfo,
        StubDirectSubmission,
    };
    use std::os::unix::io::RawFd;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    fn write_temp_json(name: &str, body: &str) -> String {
        let mut path = std::env::temp_dir();
        path.push(name);
        std::fs::write(&path, body).expect("write temp json");
        path.to_str().expect("utf8").to_string()
    }

    fn fixture_path(name: &str) -> String {
        format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn weight_plan_sizes_from_json_initializers() {
        let path = write_temp_json(
            "dragon_npu_phase5_plan.json",
            r#"{
              "ops": ["MatMul"],
              "initializers": [
                {"name": "W", "shape": [2, 3], "data": [1.0, 0.0, 0.0, 0.0, 1.0, 0.0]}
              ]
            }"#,
        );
        let backend = AmdXdna2Backend::new_with_real_stub();
        let model = backend
            .compile_model(&path, 0, "fp32")
            .expect("compile plan fixture");
        let _ = std::fs::remove_file(&path);

        let plan = build_weight_bo_plan(&model);
        assert_eq!(plan.len(), 1);
        assert_eq!(plan[0].name, "W");
        assert_eq!(plan[0].shape, vec![2, 3]);
        assert_eq!(plan[0].nbytes, 6 * 4);
        assert_eq!(plan[0].suggested_bo_type, WEIGHT_BO_TYPE_SHMEM);
        assert_eq!(plan[0].dtype, "fp32");
        // nbytes matches host bytes written on upload.
        assert_eq!(
            plan[0].nbytes,
            initializer_host_bytes(&model.initializers()[0]).len() as u64
        );
    }

    #[test]
    fn host_matmul_known_numbers() {
        // A = [[1, 2], [3, 4]] (M=2,K=2), B = [[1, 0, 1], [0, 1, 0]] (K=2,N=3)
        // C = [[1, 2, 1], [3, 4, 3]]
        let path = write_temp_json(
            "dragon_npu_phase5_matmul.json",
            r#"{
              "ops": ["MatMul"],
              "initializers": [
                {"name": "W", "shape": [2, 3], "data": [1.0, 0.0, 1.0, 0.0, 1.0, 0.0]}
              ]
            }"#,
        );
        let backend = AmdXdna2Backend::new_with_real_stub();
        let model = backend.compile_model(&path, 0, "fp32").expect("compile");
        let _ = std::fs::remove_file(&path);

        let input = vec![1.0, 2.0, 3.0, 4.0];
        let out = evaluate_host_graph(&model, &input).expect("host matmul");
        assert_eq!(out, vec![1.0, 2.0, 1.0, 3.0, 4.0, 3.0]);
    }

    #[test]
    fn fixture_host_matmul_relu_known_output() {
        let path = fixture_path("host_matmul_relu.json");
        let backend = AmdXdna2Backend::new_with_real_stub();
        let model = backend
            .compile_model(&path, 0, "fp32")
            .expect("compile checked-in fixture");
        // Identity W 2x2: [-1, 2] → MatMul same → Relu → [0, 2]
        let out = evaluate_host_graph(&model, &[-1.0, 2.0]).expect("fixture matmul+relu");
        assert_eq!(out, vec![0.0, 2.0]);
    }

    #[test]
    fn fixture_host_add_mul_known_output() {
        let path = fixture_path("host_add_mul.json");
        let backend = AmdXdna2Backend::new_with_real_stub();
        let model = backend
            .compile_model(&path, 0, "fp32")
            .expect("compile checked-in fixture");
        // [1,2,3] + [1,1,1] → [2,3,4]; *2 → [4,6,8]
        let out = evaluate_host_graph(&model, &[1.0, 2.0, 3.0]).expect("fixture add+mul");
        assert_eq!(out, vec![4.0, 6.0, 8.0]);
    }

    #[test]
    fn fixture_host_matmul_relu_add_known_output() {
        let path = fixture_path("host_matmul_relu_add.json");
        let backend = AmdXdna2Backend::new_with_real_stub();
        let model = backend
            .compile_model(&path, 0, "fp32")
            .expect("compile checked-in fixture");
        // Identity 2x2: [-1, 2] → MatMul same → Relu → [0, 2] → +[0.5, 1.5] → [0.5, 3.5]
        let out = evaluate_host_graph(&model, &[-1.0, 2.0]).expect("host linear demo fixture");
        assert_eq!(out, vec![0.5, 3.5]);
    }

    #[test]
    fn host_relu_add_mul_chain() {
        let path = write_temp_json(
            "dragon_npu_phase5_chain.json",
            r#"{
              "ops": ["Relu", "Add", "Mul"],
              "initializers": [
                {"name": "b", "shape": [2], "data": [1.0, 2.0]},
                {"name": "s", "shape": [1], "data": [0.5]}
              ]
            }"#,
        );
        let backend = AmdXdna2Backend::new_with_real_stub();
        let model = backend.compile_model(&path, 0, "fp32").expect("compile");
        let _ = std::fs::remove_file(&path);

        // Relu([-1, 4]) -> [0, 4]; +[1,2] -> [1, 6]; *0.5 -> [0.5, 3.0]
        let out = evaluate_host_graph(&model, &[-1.0, 4.0]).expect("chain");
        assert_eq!(out, vec![0.5, 3.0]);
    }

    #[test]
    fn matmul_without_weights_refuses() {
        let path = write_temp_json(
            "dragon_npu_phase5_matmul_noweight.json",
            r#"{"ops":["MatMul"]}"#,
        );
        let backend = AmdXdna2Backend::new_with_real_stub();
        let model = backend.compile_model(&path, 0, "fp32").expect("compile");
        let _ = std::fs::remove_file(&path);
        let err = evaluate_host_graph(&model, &[1.0, 2.0]).expect_err("no fake golden");
        let msg = format!("{err:?}");
        assert!(
            msg.contains("initializer") || msg.contains("weight") || msg.contains("Runtime"),
            "got {msg}"
        );
    }

    #[test]
    fn gemm_weight_only_no_sequential_bias_steal() {
        // Two weights + Gemm then MatMul: Gemm must NOT steal W2 as bias.
        let path = write_temp_json(
            "dragon_npu_phase5_gemm_no_steal.json",
            r#"{
              "ops": ["Gemm", "MatMul"],
              "initializers": [
                {"name": "W1", "shape": [2, 2], "data": [1.0, 0.0, 0.0, 1.0]},
                {"name": "W2", "shape": [2, 2], "data": [2.0, 0.0, 0.0, 2.0]}
              ]
            }"#,
        );
        let backend = AmdXdna2Backend::new_with_real_stub();
        let model = backend.compile_model(&path, 0, "fp32").expect("compile");
        let _ = std::fs::remove_file(&path);
        // Gemm(I) then MatMul(2I): [1, 0] → [1, 0] → [2, 0]
        let out = evaluate_host_graph(&model, &[1.0, 0.0]).expect("gemm+matmul no steal");
        assert_eq!(out, vec![2.0, 0.0]);
    }

    #[test]
    fn gemm_named_bias_applies() {
        let path = write_temp_json(
            "dragon_npu_phase5_gemm_named_bias.json",
            r#"{
              "nodes": [
                {"op": "Gemm", "inputs": ["x", "W", "b"]}
              ],
              "initializers": [
                {"name": "W", "shape": [2, 2], "data": [1.0, 0.0, 0.0, 1.0]},
                {"name": "b", "shape": [2], "data": [0.5, -0.25]}
              ]
            }"#,
        );
        let backend = AmdXdna2Backend::new_with_real_stub();
        let model = backend.compile_model(&path, 0, "fp32").expect("compile");
        let _ = std::fs::remove_file(&path);
        // I @ [1,2] + bias = [1.5, 1.75]
        let out = evaluate_host_graph(&model, &[1.0, 2.0]).expect("named gemm bias");
        assert_eq!(out, vec![1.5, 1.75]);
    }

    #[test]
    fn gemm_weight_only_extra_init_not_applied_as_bias() {
        // Residual initializer after weight-only Gemm must not be applied as bias.
        let path = write_temp_json(
            "dragon_npu_phase5_gemm_residual.json",
            r#"{
              "ops": ["Gemm"],
              "initializers": [
                {"name": "W", "shape": [2, 2], "data": [1.0, 0.0, 0.0, 1.0]},
                {"name": "orphan", "shape": [2], "data": [100.0, 100.0]}
              ]
            }"#,
        );
        let backend = AmdXdna2Backend::new_with_real_stub();
        let model = backend.compile_model(&path, 0, "fp32").expect("compile");
        let _ = std::fs::remove_file(&path);
        let out = evaluate_host_graph(&model, &[1.0, 2.0]).expect("no silent bias");
        assert_eq!(out, vec![1.0, 2.0]);
    }

    #[test]
    fn named_weight_binding_preferred_over_declaration_order() {
        // Initializers declared B then A; node inputs name "W" which is second.
        let path = write_temp_json(
            "dragon_npu_phase5_named_w.json",
            r#"{
              "nodes": [
                {"op": "MatMul", "inputs": ["x", "W"]}
              ],
              "initializers": [
                {"name": "other", "shape": [2, 2], "data": [9.0, 0.0, 0.0, 9.0]},
                {"name": "W", "shape": [2, 2], "data": [1.0, 0.0, 0.0, 1.0]}
              ]
            }"#,
        );
        let backend = AmdXdna2Backend::new_with_real_stub();
        let model = backend.compile_model(&path, 0, "fp32").expect("compile");
        let _ = std::fs::remove_file(&path);
        assert_eq!(model.inputs()[0], vec!["x".to_string(), "W".to_string()]);
        let out = evaluate_host_graph(&model, &[3.0, 4.0]).expect("named W");
        assert_eq!(out, vec![3.0, 4.0]); // identity W, not 9I
    }

    #[test]
    fn matmul_act_len_not_divisible_by_k_refuses() {
        let path = write_temp_json(
            "dragon_npu_phase5_bad_act.json",
            r#"{
              "ops": ["MatMul"],
              "initializers": [
                {"name": "W", "shape": [2, 2], "data": [1.0, 0.0, 0.0, 1.0]}
              ]
            }"#,
        );
        let backend = AmdXdna2Backend::new_with_real_stub();
        let model = backend.compile_model(&path, 0, "fp32").expect("compile");
        let _ = std::fs::remove_file(&path);
        let err = evaluate_host_graph(&model, &[1.0, 2.0, 3.0]).expect_err("len 3 % K=2");
        let msg = format!("{err:?}");
        assert!(msg.contains("divisible") || msg.contains("K="), "got {msg}");
    }

    #[test]
    fn binary_length_mismatch_refuses() {
        let path = write_temp_json(
            "dragon_npu_phase5_add_mismatch.json",
            r#"{
              "ops": ["Add"],
              "initializers": [
                {"name": "b", "shape": [2], "data": [1.0, 2.0]}
              ]
            }"#,
        );
        let backend = AmdXdna2Backend::new_with_real_stub();
        let model = backend.compile_model(&path, 0, "fp32").expect("compile");
        let _ = std::fs::remove_file(&path);
        let err = evaluate_host_graph(&model, &[1.0, 2.0, 3.0]).expect_err("len mismatch");
        let msg = format!("{err:?}");
        assert!(
            msg.contains("length mismatch") || msg.contains("act="),
            "got {msg}"
        );
    }

    #[test]
    fn matmul_k_option_without_env() {
        // Flat weight data, no rank-2 shape — explicit options only (no env).
        let path = write_temp_json(
            "dragon_npu_phase5_k_opt.json",
            r#"{
              "ops": ["MatMul"],
              "initializers": [
                {"name": "W", "shape": [], "data": [1.0, 0.0, 0.0, 1.0]}
              ]
            }"#,
        );
        let backend = AmdXdna2Backend::new_with_real_stub();
        let model = backend.compile_model(&path, 0, "fp32").expect("compile");
        let _ = std::fs::remove_file(&path);
        let err = evaluate_host_graph(&model, &[1.0, 2.0]).expect_err("no shape no k");
        assert!(
            format!("{err:?}").contains("rank-2") || format!("{err:?}").contains("matmul_k"),
            "{err:?}"
        );
        let out = evaluate_host_graph_with_options(
            &model,
            &[1.0, 2.0],
            &HostGraphOptions { matmul_k: Some(2) },
        )
        .expect("explicit K");
        assert_eq!(out, vec![1.0, 2.0]);
    }

    #[test]
    fn shape_payload_mismatch_check() {
        let init = Initializer {
            name: "bad".to_string(),
            shape: vec![2, 2],
            data_type: 1,
            data_f32: vec![1.0, 2.0, 3.0], // 3 elems vs 4
            data_i8: vec![],
            data_int32: vec![],
            data_int64: vec![],
            raw_data: vec![],
        };
        let err = check_initializer_shape_payload(&init).expect_err("mismatch");
        assert!(format!("{err}").contains("shape product"));
        // plan nbytes still follows payload (3*4)
        assert_eq!(initializer_nbytes(&init), 12);
    }

    #[test]
    fn stub_weight_upload_create_fill_destroy() {
        let path = write_temp_json(
            "dragon_npu_phase5_upload.json",
            r#"{
              "ops": ["Identity"],
              "initializers": [
                {"name": "W", "shape": [2], "data": [1.25, -0.5]}
              ]
            }"#,
        );
        let backend = AmdXdna2Backend::new_with_real_stub();
        let model = backend.compile_model(&path, 0, "fp32").expect("compile");
        let _ = std::fs::remove_file(&path);

        let stub = StubDirectSubmission::new(-1);
        let n = upload_weights_to_shmem_bos(&stub, -1, &model).expect("upload");
        // Success: count only; no live handles.
        assert_eq!(n, 1);
        assert_ne!(stub.destroyed_bo_handles(), [] as [u32; 0]);
    }

    /// Stub that fails `write_bo_bytes` after creating buffers (cleanup test).
    #[derive(Debug)]
    struct FailWriteSubmission {
        inner: StubDirectSubmission,
        fail_write: AtomicBool,
        created: Mutex<Vec<u32>>,
    }

    impl FailWriteSubmission {
        fn new() -> Self {
            Self {
                inner: StubDirectSubmission::new(-1),
                fail_write: AtomicBool::new(true),
                created: Mutex::new(Vec::new()),
            }
        }
    }

    impl DirectSubmission for FailWriteSubmission {
        fn create_hwctx(
            &self,
            fd: RawFd,
            num_tiles: u32,
            mem_size: u32,
            qos: Option<&QosInfo>,
        ) -> Result<HardwareContext, DirectError> {
            self.inner.create_hwctx(fd, num_tiles, mem_size, qos)
        }
        fn destroy_hwctx(&self, fd: RawFd, handle: u32) -> Result<(), DirectError> {
            self.inner.destroy_hwctx(fd, handle)
        }
        fn create_buffer(
            &self,
            fd: RawFd,
            size: u64,
            ty: crate::backends::amd_xdna2::direct::BufferType,
        ) -> Result<BufferHandle, DirectError> {
            let h = self.inner.create_buffer(fd, size, ty)?;
            if let Ok(mut v) = self.created.lock() {
                v.push(h.0);
            }
            Ok(h)
        }
        fn destroy_buffer(&self, fd: RawFd, handle: u32) -> Result<(), DirectError> {
            self.inner.destroy_buffer(fd, handle)
        }
        fn get_bo_info(
            &self,
            fd: RawFd,
            handle: u32,
        ) -> Result<crate::backends::amd_xdna2::direct::amdxdna_drm_get_bo_info, DirectError>
        {
            self.inner.get_bo_info(fd, handle)
        }
        fn submit_command(&self, fd: RawFd, cmd: &Command) -> Result<u64, DirectError> {
            self.inner.submit_command(fd, cmd)
        }
        fn wait_command(
            &self,
            fd: RawFd,
            hwctx: u32,
            seq: u64,
            timeout_ms: u32,
        ) -> Result<(), DirectError> {
            self.inner.wait_command(fd, hwctx, seq, timeout_ms)
        }
        fn wait_command_with_syncobj(
            &self,
            fd: RawFd,
            hwctx: u32,
            seq: u64,
            timeout_ms: u32,
            syncobj_handle: u32,
        ) -> Result<(), DirectError> {
            self.inner
                .wait_command_with_syncobj(fd, hwctx, seq, timeout_ms, syncobj_handle)
        }
        fn sync_buffer(
            &self,
            fd: RawFd,
            handle: BufferHandle,
            direction: u32,
            offset: u64,
            size: u64,
        ) -> Result<(), DirectError> {
            self.inner.sync_buffer(fd, handle, direction, offset, size)
        }
        fn get_info(&self, fd: RawFd, param: u32, buffer: &mut [u8]) -> Result<(), DirectError> {
            self.inner.get_info(fd, param, buffer)
        }
        fn write_bo_bytes(
            &self,
            fd: RawFd,
            handle: u32,
            offset: u64,
            data: &[u8],
        ) -> Result<(), DirectError> {
            if self.fail_write.load(Ordering::Relaxed) {
                return Err(DirectError::Unsupported(
                    "test: forced write_bo_bytes failure".to_string(),
                ));
            }
            self.inner.write_bo_bytes(fd, handle, offset, data)
        }
    }

    #[test]
    fn upload_write_failure_still_destroys_created_bos() {
        let path = write_temp_json(
            "dragon_npu_phase5_upload_fail.json",
            r#"{
              "ops": ["Identity"],
              "initializers": [
                {"name": "W", "shape": [2], "data": [1.0, 2.0]},
                {"name": "W2", "shape": [1], "data": [3.0]}
              ]
            }"#,
        );
        let backend = AmdXdna2Backend::new_with_real_stub();
        let model = backend.compile_model(&path, 0, "fp32").expect("compile");
        let _ = std::fs::remove_file(&path);

        let stub = FailWriteSubmission::new();
        let err = upload_weights_to_shmem_bos(&stub, -1, &model).expect_err("write fail");
        let msg = format!("{err:?}");
        assert!(
            msg.contains("write_bo_bytes") || msg.contains("forced"),
            "got {msg}"
        );
        let created = stub.created.lock().map(|g| g.clone()).unwrap_or_default();
        assert!(!created.is_empty(), "should have created at least one BO");
        let destroyed = stub.inner.destroyed_bo_handles();
        for h in &created {
            assert!(
                destroyed.contains(h),
                "handle {h} not destroyed after write fail; destroyed={destroyed:?}"
            );
        }
    }

    #[test]
    fn host_matmul_relu_fused_matches_separate() {
        // Fused MatMul+Relu must match MatMul then Relu numerically.
        let path = write_temp_json(
            "dragon_npu_phase6_mm_relu.json",
            r#"{
              "ops": ["MatMul", "Relu"],
              "initializers": [
                {"name": "W", "shape": [2, 2], "data": [1.0, 0.0, 0.0, 1.0]}
              ]
            }"#,
        );
        let backend = AmdXdna2Backend::new_with_real_stub();
        let model = backend.compile_model(&path, 0, "fp32").expect("compile");
        let _ = std::fs::remove_file(&path);

        let out = evaluate_host_graph(&model, &[-2.0, 3.0]).expect("fused path");
        assert_eq!(out, vec![0.0, 3.0]);
    }

    #[test]
    fn host_matmul_int8_weight_view() {
        // int8 weights via data_i8: identity 2×2, exercise once-view + fused relu write.
        let w = Initializer {
            name: "W".to_string(),
            shape: vec![2, 2],
            data_type: 3, // INT8
            data_f32: vec![],
            data_i8: vec![1, 0, 0, 1],
            data_int32: vec![],
            data_int64: vec![],
            raw_data: vec![],
        };
        let view = weight_f32_view(&w);
        assert_eq!(view.len(), 4);
        assert!((view.get_f32(0) - 1.0).abs() < f32::EPSILON);
        let out = matmul_host(&[2.0, -1.0], &w, "int8_mm", None, false, None).expect("int8 mm");
        assert_eq!(out, vec![2.0, -1.0]);
        let out_relu =
            matmul_host(&[2.0, -1.0], &w, "int8_mm", None, true, None).expect("int8 mm relu");
        assert_eq!(out_relu, vec![2.0, 0.0]);
    }

    #[test]
    fn fill_weight_bos_batch_stub() {
        let stub = StubDirectSubmission::new(-1);
        let bo1 = stub
            .create_buffer(-1, 8, BufferType::Shmem)
            .expect("create1");
        let bo2 = stub
            .create_buffer(-1, 4, BufferType::Shmem)
            .expect("create2");
        let a = [1u8, 2, 3, 4, 5, 6, 7, 8];
        let b = [9u8, 10, 11, 12];
        fill_weight_bos_batch(&stub, -1, &[(bo1.0, &a[..]), (bo2.0, &b[..])]).expect("batch fill");
        let r1 = stub.read_bo_bytes_no_sync(-1, bo1.0, 0, 8).expect("read1");
        let r2 = stub.read_bo_bytes_no_sync(-1, bo2.0, 0, 4).expect("read2");
        assert_eq!(r1, a);
        assert_eq!(r2, b);
        let _ = stub.destroy_buffer(-1, bo1.0);
        let _ = stub.destroy_buffer(-1, bo2.0);
    }

    #[test]
    fn fill_weight_bos_batch_refuses_empty() {
        let stub = StubDirectSubmission::new(-1);
        let bo = stub
            .create_buffer(-1, 4, BufferType::Shmem)
            .expect("create");
        let err = fill_weight_bos_batch(&stub, -1, &[(bo.0, &[][..])]).expect_err("empty");
        assert!(format!("{err}").contains("empty"));
        let _ = stub.destroy_buffer(-1, bo.0);
    }

    #[test]
    #[allow(clippy::too_many_lines)] // FailSecondWrite stub trait impl is verbose by design
    fn fill_weight_bos_batch_partial_on_second_write_fail() {
        // Issue 4: first BO stays filled when later write fails (no auto-rollback).
        #[derive(Debug)]
        struct FailSecondWrite {
            inner: StubDirectSubmission,
            writes: AtomicUsize,
        }
        impl DirectSubmission for FailSecondWrite {
            fn create_hwctx(
                &self,
                fd: RawFd,
                num_tiles: u32,
                mem_size: u32,
                qos: Option<&QosInfo>,
            ) -> Result<HardwareContext, DirectError> {
                self.inner.create_hwctx(fd, num_tiles, mem_size, qos)
            }
            fn destroy_hwctx(&self, fd: RawFd, handle: u32) -> Result<(), DirectError> {
                self.inner.destroy_hwctx(fd, handle)
            }
            fn create_buffer(
                &self,
                fd: RawFd,
                size: u64,
                ty: BufferType,
            ) -> Result<BufferHandle, DirectError> {
                self.inner.create_buffer(fd, size, ty)
            }
            fn destroy_buffer(&self, fd: RawFd, handle: u32) -> Result<(), DirectError> {
                self.inner.destroy_buffer(fd, handle)
            }
            fn get_bo_info(
                &self,
                fd: RawFd,
                handle: u32,
            ) -> Result<crate::backends::amd_xdna2::direct::amdxdna_drm_get_bo_info, DirectError>
            {
                self.inner.get_bo_info(fd, handle)
            }
            fn submit_command(&self, fd: RawFd, cmd: &Command) -> Result<u64, DirectError> {
                self.inner.submit_command(fd, cmd)
            }
            fn wait_command(
                &self,
                fd: RawFd,
                hwctx: u32,
                seq: u64,
                timeout_ms: u32,
            ) -> Result<(), DirectError> {
                self.inner.wait_command(fd, hwctx, seq, timeout_ms)
            }
            fn wait_command_with_syncobj(
                &self,
                fd: RawFd,
                hwctx: u32,
                seq: u64,
                timeout_ms: u32,
                syncobj_handle: u32,
            ) -> Result<(), DirectError> {
                self.inner
                    .wait_command_with_syncobj(fd, hwctx, seq, timeout_ms, syncobj_handle)
            }
            fn sync_buffer(
                &self,
                fd: RawFd,
                handle: BufferHandle,
                direction: u32,
                offset: u64,
                size: u64,
            ) -> Result<(), DirectError> {
                self.inner.sync_buffer(fd, handle, direction, offset, size)
            }
            fn get_info(
                &self,
                fd: RawFd,
                param: u32,
                buffer: &mut [u8],
            ) -> Result<(), DirectError> {
                self.inner.get_info(fd, param, buffer)
            }
            fn write_bo_bytes(
                &self,
                fd: RawFd,
                handle: u32,
                offset: u64,
                data: &[u8],
            ) -> Result<(), DirectError> {
                let n = self.writes.fetch_add(1, Ordering::SeqCst);
                if n >= 1 {
                    return Err(DirectError::InvalidArgument(
                        "test: second write forced fail".to_string(),
                    ));
                }
                self.inner.write_bo_bytes(fd, handle, offset, data)
            }
            fn read_bo_bytes_no_sync(
                &self,
                fd: RawFd,
                handle: u32,
                offset: u64,
                len: usize,
            ) -> Result<Vec<u8>, DirectError> {
                self.inner.read_bo_bytes_no_sync(fd, handle, offset, len)
            }
        }
        let sub = FailSecondWrite {
            inner: StubDirectSubmission::new(-1),
            writes: AtomicUsize::new(0),
        };
        let bo1 = sub.create_buffer(-1, 4, BufferType::Shmem).expect("bo1");
        let bo2 = sub.create_buffer(-1, 4, BufferType::Shmem).expect("bo2");
        let a = [1u8, 2, 3, 4];
        let b = [5u8, 6, 7, 8];
        let err = fill_weight_bos_batch(&sub, -1, &[(bo1.0, &a[..]), (bo2.0, &b[..])])
            .expect_err("second fail");
        let msg = format!("{err}");
        assert!(msg.contains("completed_fills=1"), "{msg}");
        let r1 = sub
            .read_bo_bytes_no_sync(-1, bo1.0, 0, 4)
            .expect("first still written");
        assert_eq!(r1, a);
        let _ = sub.destroy_buffer(-1, bo1.0);
        let _ = sub.destroy_buffer(-1, bo2.0);
    }

    #[test]
    fn host_uint8_weight_promotes_unsigned() {
        // Issue 3: data_type=2 UINT8 byte 200 → 200.0 not -56.0
        let w = Initializer {
            name: "W".to_string(),
            shape: vec![1, 1],
            data_type: 2, // UINT8
            data_f32: vec![],
            data_i8: vec![200u8.cast_signed()], // raw store as i8
            data_int32: vec![],
            data_int64: vec![],
            raw_data: vec![],
        };
        assert!((w.as_f32_data()[0] - 200.0).abs() < f32::EPSILON);
        let view = weight_f32_view(&w);
        assert!((view.get_f32(0) - 200.0).abs() < f32::EPSILON);
        let out = matmul_host(&[2.0], &w, "u8_mm", None, false, None).expect("uint8 mm");
        assert_eq!(out, vec![400.0]);
    }

    #[test]
    fn host_gemm_named_bias_then_relu_order() {
        // Issue 6: matmul → bias → relu (not relu before bias).
        // Identity W, act=[1], bias=-2 → product 1, +bias=-1, relu → 0.
        // If relu-before-bias: relu(1)+(-2)= -1.
        let path = write_temp_json(
            "dragon_npu_phase6_gemm_bias_relu.json",
            r#"{
              "nodes": [
                {"op": "Gemm", "inputs": ["x", "W", "b"], "outputs": ["y"]},
                {"op": "Relu", "inputs": ["y"], "outputs": ["z"]}
              ],
              "initializers": [
                {"name": "W", "shape": [1, 1], "data": [1.0]},
                {"name": "b", "shape": [1], "data": [-2.0]}
              ]
            }"#,
        );
        let backend = AmdXdna2Backend::new_with_real_stub();
        let model = backend.compile_model(&path, 0, "fp32").expect("compile");
        let _ = std::fs::remove_file(&path);
        let out = evaluate_host_graph(&model, &[1.0]).expect("gemm+bias+relu");
        assert_eq!(out, vec![0.0], "expected matmul→bias→relu, got {out:?}");
    }

    #[test]
    fn host_ref_after_level1_fused_matmul_relu() {
        // Issue 2: optimization_level>=1 rewrites to fused_matmul_relu; host_ref still works.
        let path = write_temp_json(
            "dragon_npu_phase6_opt_fuse_mm_relu.json",
            r#"{
              "ops": ["MatMul", "Relu"],
              "initializers": [
                {"name": "W", "shape": [2, 2], "data": [1.0, 0.0, 0.0, 1.0]}
              ]
            }"#,
        );
        let backend = AmdXdna2Backend::new_with_real_stub();
        let model = backend.compile_model(&path, 1, "fp32").expect("compile L1");
        let _ = std::fs::remove_file(&path);
        // Fuse may produce fused_matmul_relu
        let has_fused = model
            .ops()
            .iter()
            .any(|o| normalize_op(o).contains("fused"));
        assert!(
            has_fused || model.ops().iter().any(|o| normalize_op(o) == "matmul"),
            "ops={:?}",
            model.ops()
        );
        assert!(
            host_graph_ops_supported(model.ops()),
            "fused names must remain host-evaluable; ops={:?}",
            model.ops()
        );
        let out = evaluate_host_graph(&model, &[-2.0, 3.0]).expect("host after fuse");
        assert_eq!(out, vec![0.0, 3.0]);
    }

    #[test]
    fn host_ref_after_level2_ir_then_fused_matmul_relu() {
        // Issue 8: opt≥2 runs IR optimize then fuse; MatMul+Relu remains host_ref-safe.
        let path = write_temp_json(
            "dragon_npu_phase7_opt2_mm_relu.json",
            r#"{
              "ops": ["MatMul", "Relu"],
              "initializers": [
                {"name": "W", "shape": [2, 2], "data": [1.0, 0.0, 0.0, 1.0]}
              ]
            }"#,
        );
        let backend = AmdXdna2Backend::new_with_real_stub();
        let model = backend.compile_model(&path, 2, "fp32").expect("compile L2");
        let _ = std::fs::remove_file(&path);
        assert!(
            host_graph_ops_supported(model.ops()),
            "level2 IR+fuse ops must stay host-allowlisted; ops={:?}",
            model.ops()
        );
        let out = evaluate_host_graph(&model, &[-2.0, 3.0]).expect("host after L2");
        assert_eq!(out, vec![0.0, 3.0]);
    }

    #[test]
    fn host_ref_level2_foldable_const_graph_golden() {
        // Issue 1 + 12: full host_ref path for opt≥2 constant fold (lives under amd-xdna2).
        // Relu([-1,2,-3]) → [0,2,0]; +[1,1,1] → [1,3,1] as Constant initializer.
        let path = write_temp_json(
            "dragon_npu_phase7_level2_fold_host.json",
            r#"{
              "nodes": [
                {"op": "Relu", "inputs": ["c0"], "outputs": ["r"]},
                {"op": "Add", "inputs": ["r", "c1"], "outputs": ["out"]}
              ],
              "initializers": [
                {"name": "c0", "shape": [3], "data": [-1.0, 2.0, -3.0]},
                {"name": "c1", "shape": [3], "data": [1.0, 1.0, 1.0]}
              ],
              "graph_outputs": ["out"]
            }"#,
        );
        let backend = AmdXdna2Backend::new_with_real_stub();
        let model = backend.compile_model(&path, 2, "fp32").expect("compile L2");
        let _ = std::fs::remove_file(&path);
        assert!(
            host_graph_ops_supported(model.ops()),
            "ops after level2 fold must stay host-allowlisted, got {:?}",
            model.ops()
        );
        assert!(
            model.ops().iter().all(|o| normalize_op(o) == "constant"),
            "expected only Constant ops, got {:?}",
            model.ops()
        );
        // Pure const graph: Constant ops load folded values; host_ref replaces act.
        let out = evaluate_host_graph(&model, &[0.0, 0.0, 0.0]).expect("host fold golden");
        assert_eq!(out.len(), 3);
        assert!((out[0] - 1.0).abs() < 1e-5, "got {out:?}");
        assert!((out[1] - 3.0).abs() < 1e-5, "got {out:?}");
        assert!((out[2] - 1.0).abs() < 1e-5, "got {out:?}");
    }

    #[test]
    fn host_ref_gemm_alpha_and_trans_b() {
        // Issue 4: Gemm attrs must affect host math (alpha=2, transB=1).
        // W as [N,K]=[[1,2],[3,4]] row-major; transB ⇒ A @ B^T with B^T=[[1,3],[2,4]].
        // act [1,0] @ B^T = [1,3]; alpha=2 → [2,6].
        let path = write_temp_json(
            "dragon_npu_phase7_gemm_attrs.json",
            r#"{
              "nodes": [
                {"op": "Gemm", "inputs": ["x", "W"], "outputs": ["y"],
                 "attrs": {"alpha": 2.0, "transB": 1}}
              ],
              "initializers": [
                {"name": "W", "shape": [2, 2], "data": [1.0, 2.0, 3.0, 4.0]}
              ]
            }"#,
        );
        let backend = AmdXdna2Backend::new_with_real_stub();
        let model = backend.compile_model(&path, 0, "fp32").expect("compile");
        let _ = std::fs::remove_file(&path);
        assert_eq!(model.node_attrs()[0].alpha, Some(2.0));
        assert_eq!(model.node_attrs()[0].trans_b, Some(1));
        let out = evaluate_host_graph(&model, &[1.0, 0.0]).expect("gemm attrs");
        assert_eq!(out.len(), 2);
        assert!((out[0] - 2.0).abs() < 1e-5, "got {out:?}");
        assert!((out[1] - 6.0).abs() < 1e-5, "got {out:?}");
    }

    #[test]
    fn host_ref_level1_fused_matmul_add_named_bias_golden() {
        // Re-review: fused MatMul+Add merges inputs ["x","W"]+["y","bias"].
        // resolve_binary_rhs must skip already-used W and pick bias (not A@W+W).
        // Identity W, bias=[10,20], act=[-2,3] → [-2,3]+[10,20] = [8,23].
        let path = write_temp_json(
            "dragon_npu_phase6_opt_fuse_mm_add.json",
            r#"{
              "nodes": [
                {"op": "MatMul", "inputs": ["x", "W"], "outputs": ["y"]},
                {"op": "Add", "inputs": ["y", "bias"], "outputs": ["z"]}
              ],
              "initializers": [
                {"name": "W", "shape": [2, 2], "data": [1.0, 0.0, 0.0, 1.0]},
                {"name": "bias", "shape": [2], "data": [10.0, 20.0]}
              ]
            }"#,
        );
        let backend = AmdXdna2Backend::new_with_real_stub();
        let model = backend.compile_model(&path, 1, "fp32").expect("compile L1");
        let _ = std::fs::remove_file(&path);
        assert!(
            model
                .ops()
                .iter()
                .any(|o| normalize_op(o) == "fused_matmul_add"),
            "expected fused_matmul_add after L1 fuse, ops={:?}",
            model.ops()
        );
        // Merged inputs should still list W before bias (reproduce the bug condition).
        let ins = model.inputs().first().cloned().unwrap_or_default();
        assert!(
            ins.iter().any(|n| n == "W") && ins.iter().any(|n| n == "bias"),
            "merged inputs should contain W and bias, got {ins:?}"
        );
        let out = evaluate_host_graph(&model, &[-2.0, 3.0]).expect("fused matmul+add");
        assert_eq!(
            out,
            vec![8.0, 23.0],
            "A@W+bias golden; wrong if RHS re-picked W (got {out:?})"
        );
    }

    #[test]
    fn select_lab_matmul_weight_named_second_input() {
        let path = write_temp_json(
            "dragon_npu_lab_wts_named.json",
            r#"{
              "nodes": [
                {"op": "MatMul", "inputs": ["x", "W"], "outputs": ["y"]}
              ],
              "initializers": [
                {"name": "W", "shape": [4, 8], "data": [
                  1,0,0,0,0,0,0,0, 0,1,0,0,0,0,0,0,
                  0,0,1,0,0,0,0,0, 0,0,0,1,0,0,0,0
                ]}
              ]
            }"#,
        );
        let backend = AmdXdna2Backend::new_with_real_stub();
        let model = backend.compile_model(&path, 0, "fp32").expect("compile");
        let _ = std::fs::remove_file(&path);
        let spec = select_lab_matmul_weight(&model, &HostGraphOptions::default()).expect("select");
        assert_eq!(spec.weight_name, "W");
        assert_eq!(spec.k, 4);
        assert_eq!(spec.n, 8);
        assert_eq!(spec.host_wts_bytes.len(), 4 * 8 * 4);
        let wts_plan = super::super::lab_io::lab_matmul_wts_nbytes(spec.k, spec.n, "f32").unwrap();
        assert_eq!(spec.host_wts_bytes.len() as u64, wts_plan);
    }

    #[test]
    fn select_lab_matmul_weight_sequential_fallback() {
        let path = write_temp_json(
            "dragon_npu_lab_wts_seq.json",
            r#"{
              "ops": ["MatMul"],
              "initializers": [
                {"name": "W", "shape": [2, 3], "data": [1,0,0, 0,1,0]}
              ]
            }"#,
        );
        let backend = AmdXdna2Backend::new_with_real_stub();
        let model = backend.compile_model(&path, 0, "fp32").expect("compile");
        let _ = std::fs::remove_file(&path);
        let spec = select_lab_matmul_weight(&model, &HostGraphOptions::default()).expect("select");
        assert_eq!(spec.k, 2);
        assert_eq!(spec.n, 3);
        assert_eq!(spec.host_wts_bytes.len(), 24);
    }

    #[test]
    fn select_lab_matmul_weight_matmul_k_option() {
        let path = write_temp_json(
            "dragon_npu_lab_wts_kopt.json",
            r#"{
              "ops": ["MatMul"],
              "initializers": [
                {"name": "W", "data": [1.0, 0.0, 0.0, 1.0]}
              ]
            }"#,
        );
        let backend = AmdXdna2Backend::new_with_real_stub();
        let model = backend.compile_model(&path, 0, "fp32").expect("compile");
        let _ = std::fs::remove_file(&path);
        let err = select_lab_matmul_weight(&model, &HostGraphOptions::default()).expect_err("no k");
        assert!(
            format!("{err}").contains("rank-2") || format!("{err}").contains("matmul_k"),
            "{err}"
        );
        let opts = HostGraphOptions { matmul_k: Some(2) };
        let spec = select_lab_matmul_weight(&model, &opts).expect("with k");
        assert_eq!(spec.k, 2);
        assert_eq!(spec.n, 2);
    }

    #[test]
    fn select_lab_matmul_weight_refuses_missing() {
        let path = write_temp_json("dragon_npu_lab_wts_none.json", r#"{ "ops": ["Relu"] }"#);
        let backend = AmdXdna2Backend::new_with_real_stub();
        let model = backend.compile_model(&path, 0, "fp32").expect("compile");
        let _ = std::fs::remove_file(&path);
        let err =
            select_lab_matmul_weight(&model, &HostGraphOptions::default()).expect_err("no mm");
        assert!(format!("{err}").contains("no MatMul"), "{err}");
    }

    #[test]
    fn weight_exec_fill_request_from_spec_and_stub_lifecycle() {
        use super::super::lab_io::{
            create_and_fill_weight_io_bos, destroy_weight_exec_bind, host_ones_f32_matrix_bytes,
            readback_ofm,
        };
        let path = write_temp_json(
            "dragon_npu_lab_wts_bind.json",
            r#"{
              "ops": ["MatMul"],
              "initializers": [
                {"name": "W", "shape": [2, 2], "data": [2.0, 0.0, 0.0, 3.0]}
              ]
            }"#,
        );
        let backend = AmdXdna2Backend::new_with_real_stub();
        let model = backend.compile_model(&path, 0, "fp32").expect("compile");
        let _ = std::fs::remove_file(&path);
        let spec = select_lab_matmul_weight(&model, &HostGraphOptions::default()).expect("select");
        let m = 1u32;
        let ifm = host_ones_f32_matrix_bytes(m, spec.k).unwrap();
        let req = weight_exec_fill_request_from_spec(&spec, m, ifm, "f32").expect("req");
        assert_eq!(req.wts_name, "W");
        let stub = StubDirectSubmission::new(-1);
        let bind = create_and_fill_weight_io_bos(&stub, &req).expect("bind");
        assert_eq!(bind.k, 2);
        assert_eq!(bind.n, 2);
        let ofm = readback_ofm(&stub, &bind.io).unwrap();
        assert_eq!(ofm, vec![0u8; 8]);
        destroy_weight_exec_bind(&stub, &bind);
    }

    #[test]
    fn evaluate_host_ir_matmul_relu_fixture_parity() {
        let path = fixture_path("host_matmul_relu.json");
        let backend = AmdXdna2Backend::new_with_real_stub();
        let model = backend
            .compile_model(&path, 0, "fp32")
            .expect("compile fixture");
        let out_cm = evaluate_host_graph(&model, &[-1.0, 2.0]).expect("host graph");
        assert_eq!(out_cm, vec![0.0, 2.0]);

        // Recover ModelGraph fields via compile path: load JSON → IR roundtrip.
        // Build IR from a temporary ModelGraph by re-compiling is hard; use
        // from_model_graph after packing shell from evaluate path is not available.
        // Instead: load via ModelCompiler internals by reusing ops/inits from CompiledModel.
        let graph = crate::compiler::ModelGraph {
            ops: model.ops().to_vec(),
            memory_plan: 0,
            layout: None,
            quantization: None,
            node_names: model.node_names().to_vec(),
            shapes: model.shapes().to_vec(),
            dtypes: model.dtypes().to_vec(),
            inputs: model.inputs().to_vec(),
            outputs: model.outputs().to_vec(),
            layouts: model.layouts().to_vec(),
            initializer_count: model.initializers().len(),
            initializers: model.initializers().to_vec(),
            node_attrs: model.node_attrs().to_vec(),
            graph_input_names: vec![],
            graph_output_names: vec![],
            named_shapes: vec![],
            named_dtypes: vec![],
        };
        let ir = crate::DragonNpuIR::from_model_graph(&graph).expect("to ir");
        let out_ir = evaluate_host_ir(&ir, &[-1.0, 2.0]).expect("host ir");
        assert_eq!(out_ir, out_cm);
        assert_eq!(out_ir, vec![0.0, 2.0]);
        // Roundtrip IR → graph → IR still evals
        let g2 = ir.to_model_graph();
        let ir2 = crate::DragonNpuIR::from_model_graph(&g2).expect("roundtrip");
        let out2 = evaluate_host_ir(&ir2, &[-1.0, 2.0]).expect("roundtrip eval");
        assert_eq!(out2, vec![0.0, 2.0]);
        assert!(host_ir_ops_supported(&ir));
    }

    #[test]
    fn evaluate_host_ir_refuses_conv() {
        let graph = crate::compiler::ModelGraph {
            ops: vec!["Conv".to_string()],
            memory_plan: 0,
            layout: None,
            quantization: None,
            node_names: vec![None],
            shapes: vec![None],
            dtypes: vec![None],
            inputs: vec![vec!["x".into()]],
            outputs: vec![vec!["y".into()]],
            layouts: vec![None],
            initializer_count: 0,
            initializers: vec![],
            node_attrs: vec![crate::ir::NodeAttrs::default()],
            graph_input_names: vec![],
            graph_output_names: vec![],
            named_shapes: vec![],
            named_dtypes: vec![],
        };
        let ir = crate::DragonNpuIR::from_model_graph(&graph).expect("ir");
        assert!(!host_ir_ops_supported(&ir));
        let err = evaluate_host_ir(&ir, &[1.0, 2.0]).expect_err("conv");
        match err {
            DragonNpuError::UnsupportedOps { ops, .. } => {
                assert!(ops.contains("conv") || ops.contains("Conv"), "ops={ops}");
            }
            other => panic!("expected UnsupportedOps, got {other}"),
        }
    }

    #[test]
    fn evaluate_host_ir_empty_input() {
        let graph = crate::compiler::ModelGraph {
            ops: vec!["Relu".to_string()],
            memory_plan: 0,
            layout: None,
            quantization: None,
            node_names: vec![None],
            shapes: vec![None],
            dtypes: vec![None],
            inputs: vec![vec![]],
            outputs: vec![vec![]],
            layouts: vec![None],
            initializer_count: 0,
            initializers: vec![],
            node_attrs: vec![crate::ir::NodeAttrs::default()],
            graph_input_names: vec![],
            graph_output_names: vec![],
            named_shapes: vec![],
            named_dtypes: vec![],
        };
        let ir = crate::DragonNpuIR::from_model_graph(&graph).expect("ir");
        let err = evaluate_host_ir(&ir, &[]).expect_err("empty");
        assert!(format!("{err}").contains("empty"), "{err}");
    }

    #[test]
    fn select_lab_matmul_weight_honors_gemm_trans_b() {
        // Weight stored [N,K]=[2,3] with transB=1 → logical K=3,N=2
        let path = write_temp_json(
            "dragon_npu_lab_wts_transb.json",
            r#"{
              "nodes": [
                {"op": "Gemm", "inputs": ["x", "W"], "outputs": ["y"],
                 "attrs": {"transB": 1}}
              ],
              "initializers": [
                {"name": "W", "shape": [2, 3], "data": [
                  1.0, 2.0, 3.0,
                  4.0, 5.0, 6.0
                ]}
              ]
            }"#,
        );
        let backend = AmdXdna2Backend::new_with_real_stub();
        let model = backend.compile_model(&path, 0, "fp32").expect("compile");
        let _ = std::fs::remove_file(&path);
        assert_eq!(model.node_attrs()[0].trans_b, Some(1));
        let spec = select_lab_matmul_weight(&model, &HostGraphOptions::default()).expect("select");
        assert_eq!(spec.k, 3, "transB K from shape[1]");
        assert_eq!(spec.n, 2, "transB N from shape[0]");
        assert_eq!(spec.host_wts_bytes.len(), 3 * 2 * 4);
        // Logical [K,N] layout: col0 of stored rows → first column of [K,N]
        let w = super::super::lab_io::f32_slice_from_le_bytes(&spec.host_wts_bytes).unwrap();
        // Stored row-major [N,K]: row0=[1,2,3], row1=[4,5,6]
        // Logical B[K,N]: B[0,0]=1,B[0,1]=4, B[1,0]=2,B[1,1]=5, B[2,0]=3,B[2,1]=6
        assert_eq!(w, vec![1.0, 4.0, 2.0, 5.0, 3.0, 6.0]);
    }

    #[test]
    fn evaluate_host_ir_with_options_matmul_k() {
        let path = write_temp_json(
            "dragon_npu_host_ir_kopt.json",
            r#"{
              "ops": ["MatMul"],
              "initializers": [
                {"name": "W", "data": [1.0, 0.0, 0.0, 1.0]}
              ]
            }"#,
        );
        let backend = AmdXdna2Backend::new_with_real_stub();
        let model = backend.compile_model(&path, 0, "fp32").expect("compile");
        let _ = std::fs::remove_file(&path);
        let graph = crate::compiler::ModelGraph {
            ops: model.ops().to_vec(),
            memory_plan: 0,
            layout: None,
            quantization: None,
            node_names: model.node_names().to_vec(),
            shapes: model.shapes().to_vec(),
            dtypes: model.dtypes().to_vec(),
            inputs: model.inputs().to_vec(),
            outputs: model.outputs().to_vec(),
            layouts: model.layouts().to_vec(),
            initializer_count: model.initializers().len(),
            initializers: model.initializers().to_vec(),
            node_attrs: model.node_attrs().to_vec(),
            graph_input_names: vec![],
            graph_output_names: vec![],
            named_shapes: vec![],
            named_dtypes: vec![],
        };
        let ir = crate::DragonNpuIR::from_model_graph(&graph).expect("ir");
        let opts = HostGraphOptions { matmul_k: Some(2) };
        let out = evaluate_host_ir_with_options(&ir, &[1.0, 2.0], &opts).expect("ir opts");
        let out_cm = evaluate_host_graph_with_options(&model, &[1.0, 2.0], &opts).expect("cm");
        assert_eq!(out, out_cm);
        assert_eq!(out, vec![1.0, 2.0]);
    }

    #[test]
    fn lab_weight_requires_ofm_pure() {
        assert!(lab_weight_requires_ofm(true, true).is_ok());
        assert!(lab_weight_requires_ofm(true, false).is_ok());
        assert!(lab_weight_requires_ofm(false, false).is_ok());
        let err = lab_weight_requires_ofm(false, true).expect_err("weight without ofm");
        assert!(
            err.to_string().contains("LAB_OFM") || err.to_string().contains("LAB_WEIGHT"),
            "{err}"
        );
    }

    #[test]
    fn create_lab_weight_io_from_model_path_stub() {
        let path = write_temp_json(
            "dragon_npu_lab_weight_from_path.json",
            r#"{
              "ops": ["MatMul"],
              "initializers": [
                {"name": "W", "shape": [2, 2], "data": [2.0, 0.0, 0.0, 3.0]}
              ]
            }"#,
        );
        let stub = StubDirectSubmission::new(-1);
        let (bind, golden) =
            create_lab_weight_io_from_model_path(&stub, &path, 1, 2, 2, "f32").expect("bind");
        let _ = std::fs::remove_file(&path);
        assert_eq!(bind.k, 2);
        assert_eq!(bind.n, 2);
        assert_eq!(bind.wts_name, "W");
        // ones IFM × diag(2,3) → [2, 3]
        assert_eq!(golden, vec![2.0, 3.0]);
        // Full OFM alloc zeroed
        assert!(bind.io.ofm_alloc_bytes >= 4096);
        let ofm = super::super::lab_io::readback_ofm(&stub, &bind.io).unwrap();
        assert_eq!(ofm, vec![0u8; 8]);
        destroy_weight_exec_bind_via_lab(&stub, &bind);
    }

    fn destroy_weight_exec_bind_via_lab(
        stub: &StubDirectSubmission,
        bind: &super::super::lab_io::WeightExecBind,
    ) {
        super::super::lab_io::destroy_weight_exec_bind(stub, bind);
    }

    #[test]
    fn create_lab_weight_io_refuses_dim_mismatch() {
        let path = write_temp_json(
            "dragon_npu_lab_weight_dim_mismatch.json",
            r#"{
              "ops": ["MatMul"],
              "initializers": [
                {"name": "W", "shape": [2, 2], "data": [1.0, 0.0, 0.0, 1.0]}
              ]
            }"#,
        );
        let stub = StubDirectSubmission::new(-1);
        let err = create_lab_weight_io_from_model_path(&stub, &path, 1, 3, 2, "f32")
            .expect_err("n mismatch");
        let _ = std::fs::remove_file(&path);
        assert!(
            format!("{err}").contains("K,N") || format!("{err}").contains("k,n"),
            "{err}"
        );
    }

    #[test]
    fn check_lab_weight_mvp_refuses_fused_and_gemm_alpha() {
        let path_f = write_temp_json(
            "dragon_npu_lab_weight_mvp_fused.json",
            r#"{ "ops": ["fused_matmul_relu"],
              "initializers": [{"name":"W","shape":[1,1],"data":[1.0]}] }"#,
        );
        let backend = AmdXdna2Backend::new_with_real_stub();
        let model_f = backend.compile_model(&path_f, 0, "fp32").expect("compile");
        let _ = std::fs::remove_file(&path_f);
        let err = check_lab_weight_mvp_model(&model_f).expect_err("fused");
        assert!(format!("{err}").contains("fused"), "{err}");

        let path_g = write_temp_json(
            "dragon_npu_lab_weight_mvp_alpha.json",
            r#"{
              "nodes": [
                {"op": "Gemm", "inputs": ["x","W"], "outputs": ["y"],
                 "attrs": {"alpha": 2.0}}
              ],
              "initializers": [{"name":"W","shape":[1,1],"data":[1.0]}]
            }"#,
        );
        let model_g = backend.compile_model(&path_g, 0, "fp32").expect("compile");
        let _ = std::fs::remove_file(&path_g);
        let err2 = check_lab_weight_mvp_model(&model_g).expect_err("alpha");
        assert!(
            format!("{err2}").contains("alpha") || format!("{err2}").contains("MVP"),
            "{err2}"
        );
    }

    #[test]
    fn select_lab_matmul_weight_fused_matmul_relu() {
        let path = write_temp_json(
            "dragon_npu_lab_wts_fused.json",
            r#"{
              "ops": ["fused_matmul_relu"],
              "initializers": [
                {"name": "W", "shape": [2, 2], "data": [1.0, 0.0, 0.0, 1.0]}
              ]
            }"#,
        );
        let backend = AmdXdna2Backend::new_with_real_stub();
        let model = backend.compile_model(&path, 0, "fp32").expect("compile");
        let _ = std::fs::remove_file(&path);
        let spec = select_lab_matmul_weight(&model, &HostGraphOptions::default()).expect("select");
        assert_eq!(spec.k, 2);
        assert_eq!(spec.n, 2);
        assert_eq!(spec.weight_name, "W");
    }

    #[test]
    fn evaluate_host_ir_refuses_missing_weights() {
        let graph = crate::compiler::ModelGraph {
            ops: vec!["MatMul".to_string()],
            memory_plan: 0,
            layout: None,
            quantization: None,
            node_names: vec![None],
            shapes: vec![None],
            dtypes: vec![None],
            inputs: vec![vec!["x".into()]],
            outputs: vec![vec!["y".into()]],
            layouts: vec![None],
            initializer_count: 0,
            initializers: vec![],
            node_attrs: vec![crate::ir::NodeAttrs::default()],
            graph_input_names: vec![],
            graph_output_names: vec![],
            named_shapes: vec![],
            named_dtypes: vec![],
        };
        let ir = crate::DragonNpuIR::from_model_graph(&graph).expect("ir");
        let err = evaluate_host_ir(&ir, &[1.0, 2.0]).expect_err("no weights");
        let msg = format!("{err}");
        assert!(
            msg.contains("initializer") || msg.contains("weight") || msg.contains("refuse"),
            "{msg}"
        );
    }

    #[test]
    fn evaluate_host_ir_add_mul_fixture_parity() {
        let path = fixture_path("host_add_mul.json");
        let backend = AmdXdna2Backend::new_with_real_stub();
        let model = backend.compile_model(&path, 0, "fp32").expect("compile");
        let out_cm = evaluate_host_graph(&model, &[1.0, 2.0, 3.0]).expect("graph");
        let graph = crate::compiler::ModelGraph {
            ops: model.ops().to_vec(),
            memory_plan: 0,
            layout: None,
            quantization: None,
            node_names: model.node_names().to_vec(),
            shapes: model.shapes().to_vec(),
            dtypes: model.dtypes().to_vec(),
            inputs: model.inputs().to_vec(),
            outputs: model.outputs().to_vec(),
            layouts: model.layouts().to_vec(),
            initializer_count: model.initializers().len(),
            initializers: model.initializers().to_vec(),
            node_attrs: model.node_attrs().to_vec(),
            graph_input_names: vec![],
            graph_output_names: vec![],
            named_shapes: vec![],
            named_dtypes: vec![],
        };
        let ir = crate::DragonNpuIR::from_model_graph(&graph).expect("ir");
        let out_ir = evaluate_host_ir(&ir, &[1.0, 2.0, 3.0]).expect("ir");
        assert_eq!(out_ir, out_cm);
        assert_eq!(out_ir, vec![4.0, 6.0, 8.0]);
    }
}
