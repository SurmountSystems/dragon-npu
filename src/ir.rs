//! Phase 7 — Typed Earth Dragon NPU IR (alongside `ModelGraph` parallel Vecs).
//!
//! # Design
//!
//! `ModelGraph` remains the production mutation surface for fuse/layout/quantize/
//! memory and for `CompiledModel` field packing. `DragonNpuIR` is a **typed dual
//! representation** with explicit tensors, nodes, attrs, and shape slots.
//!
//! Conversion is intentional dual-path:
//! - [`DragonNpuIR::from_model_graph`] — build typed IR for shape infer / fold / DCE / cost
//! - [`DragonNpuIR::to_model_graph`] — lower back so host_ref / fuse / backends stay unchanged
//!
//! # Honesty
//!
//! - Shape inference never invents silicon tile sizes.
//! - [`estimate_xdna2_cost`] is a **heuristic host estimate**, not measured NPU time.
//! - No EXEC_CMD, no `/dev/accel`, no `ran_on_npu` claims.

// ONNX dims are i64; MAC / byte estimates promote to u64. Pedantic cast lints
// would drown legitimate shape math without improving safety here.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]

use std::collections::{HashMap, HashSet};

use crate::DragonNpuError;
use crate::compiler::{Initializer, ModelGraph};

/// Opaque tensor identifier within a single [`DragonNpuIR`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TensorId(pub u32);

/// Opaque node identifier within a single [`DragonNpuIR`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub u32);

/// Element type for tensors (aligned with ONNX TensorProto / weight path).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum DType {
    /// IEEE float32 (ONNX FLOAT=1).
    #[default]
    F32,
    /// float16 (ONNX FLOAT16=10).
    F16,
    /// bfloat16 (ONNX BFLOAT16=16).
    BF16,
    /// signed int8.
    I8,
    /// unsigned int8.
    U8,
    /// int32.
    I32,
    /// int64.
    I64,
    /// bool.
    Bool,
    /// Unknown / unparsed.
    Unknown,
}

impl DType {
    /// Parse a short dtype string from ModelGraph / JSON (`fp32`, `int8`, …).
    #[must_use]
    pub fn from_name(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "fp32" | "float" | "float32" | "f32" => Self::F32,
            "fp16" | "float16" | "f16" => Self::F16,
            "bf16" | "bfloat16" => Self::BF16,
            "int8" | "i8" => Self::I8,
            "uint8" | "u8" => Self::U8,
            "int32" | "i32" | "int" => Self::I32,
            "int64" | "i64" => Self::I64,
            "bool" => Self::Bool,
            _ => Self::Unknown,
        }
    }

    /// Map ONNX TensorProto `data_type` enum.
    #[must_use]
    pub fn from_onnx_data_type(t: i32) -> Self {
        match t {
            1 => Self::F32,
            2 => Self::U8,
            3 => Self::I8,
            6 => Self::I32,
            7 => Self::I64,
            9 => Self::Bool,
            10 => Self::F16,
            16 => Self::BF16,
            _ => Self::Unknown,
        }
    }

    /// Short name for ModelGraph / diagnostics.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::F32 => "fp32",
            Self::F16 => "fp16",
            Self::BF16 => "bf16",
            Self::I8 => "int8",
            Self::U8 => "uint8",
            Self::I32 => "int32",
            Self::I64 => "int64",
            Self::Bool => "bool",
            Self::Unknown => "unknown",
        }
    }
}

/// Tensor value description (shape dims use `-1` for unknown / symbolic).
#[derive(Debug, Clone)]
pub struct TensorDesc {
    /// Stable id within this IR instance.
    pub id: TensorId,
    /// Value name (ONNX value / initializer name).
    pub name: String,
    /// Shape; empty = unknown rank; `-1` = unknown dim.
    pub shape: Vec<i64>,
    /// Element type.
    pub dtype: DType,
    /// True when this tensor is (or is backed by) an initializer constant.
    pub is_constant: bool,
}

/// Core operators + fused names from Phase 6 + unknown passthrough.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeOp {
    /// Identity / passthrough.
    Identity,
    /// ReLU.
    Relu,
    /// Elementwise add.
    Add,
    /// Elementwise mul.
    Mul,
    /// Matrix multiply.
    MatMul,
    /// Gemm (optional bias, alpha/beta/trans).
    Gemm,
    /// Convolution.
    Conv,
    /// Reshape.
    Reshape,
    /// Transpose.
    Transpose,
    /// Constant / initializer producer (folding result or explicit).
    Constant,
    /// Phase 6 fused op stored as canonical lowercase name (e.g. `fused_matmul_relu`).
    Fused(String),
    /// Unrecognized op_type (preserved for roundtrip honesty).
    Unknown(String),
}

impl NodeOp {
    /// Parse an ONNX / IR op name (case-insensitive for known ops).
    #[must_use]
    pub fn parse(op: &str) -> Self {
        let n = op.trim().to_ascii_lowercase();
        match n.as_str() {
            "identity" => Self::Identity,
            "relu" => Self::Relu,
            "add" => Self::Add,
            "mul" => Self::Mul,
            "matmul" => Self::MatMul,
            "gemm" => Self::Gemm,
            "conv" | "conv2d" => Self::Conv,
            "reshape" => Self::Reshape,
            "transpose" => Self::Transpose,
            "constant" => Self::Constant,
            s if s.starts_with("fused_") => Self::Fused(s.to_string()),
            _ => Self::Unknown(op.to_string()),
        }
    }

    /// Canonical string for ModelGraph.ops (preserves unknown casing via stored string).
    #[must_use]
    pub fn as_op_string(&self) -> String {
        match self {
            Self::Identity => "Identity".to_string(),
            Self::Relu => "Relu".to_string(),
            Self::Add => "Add".to_string(),
            Self::Mul => "Mul".to_string(),
            Self::MatMul => "MatMul".to_string(),
            Self::Gemm => "Gemm".to_string(),
            Self::Conv => "Conv".to_string(),
            Self::Reshape => "Reshape".to_string(),
            Self::Transpose => "Transpose".to_string(),
            Self::Constant => "Constant".to_string(),
            Self::Fused(s) | Self::Unknown(s) => s.clone(),
        }
    }

    /// True when this op is a plausible NPU-candidate dense compute kernel.
    #[must_use]
    pub fn is_npu_candidate(&self) -> bool {
        match self {
            Self::MatMul | Self::Gemm | Self::Conv => true,
            Self::Fused(s) => {
                let s = s.as_str();
                s.contains("matmul") || s.contains("gemm") || s.contains("conv")
            }
            _ => false,
        }
    }
}

/// Attribute value for untyped extras.
#[derive(Debug, Clone, PartialEq)]
pub enum AttrValue {
    /// Scalar float.
    F(f32),
    /// Scalar int.
    I(i64),
    /// UTF-8 string.
    S(String),
    /// Float list.
    Floats(Vec<f32>),
    /// Int list.
    Ints(Vec<i64>),
}

/// Typed node attributes used by Gemm / Conv / Reshape / Transpose.
///
/// Unknown attributes land in [`NodeAttrs::extra`].
#[derive(Debug, Clone, Default, PartialEq)]
pub struct NodeAttrs {
    /// Gemm `alpha` (default 1.0 in ONNX when absent).
    pub alpha: Option<f32>,
    /// Gemm `beta` (default 1.0 in ONNX when absent).
    pub beta: Option<f32>,
    /// Gemm `transA` (0/1).
    pub trans_a: Option<i64>,
    /// Gemm `transB` (0/1).
    pub trans_b: Option<i64>,
    /// Conv `kernel_shape`.
    pub kernel_shape: Option<Vec<i64>>,
    /// Conv `pads`.
    pub pads: Option<Vec<i64>>,
    /// Conv `strides`.
    pub strides: Option<Vec<i64>>,
    /// Transpose `perm`.
    pub perm: Option<Vec<i64>>,
    /// Reshape target shape (when known as attribute rather than input).
    pub reshape_shape: Option<Vec<i64>>,
    /// Remaining named attributes.
    pub extra: HashMap<String, AttrValue>,
}

impl NodeAttrs {
    /// True when no typed fields and no extras are set.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.alpha.is_none()
            && self.beta.is_none()
            && self.trans_a.is_none()
            && self.trans_b.is_none()
            && self.kernel_shape.is_none()
            && self.pads.is_none()
            && self.strides.is_none()
            && self.perm.is_none()
            && self.reshape_shape.is_none()
            && self.extra.is_empty()
    }

    /// Parse a JSON object (`{"alpha":1.0,"transB":1,...}`) into attrs.
    #[must_use]
    #[allow(clippy::float_cmp)] // integral check via fract()==0 is intentional
    pub fn from_json_map(map: &serde_json::Map<String, serde_json::Value>) -> Self {
        let mut a = Self::default();
        for (k, v) in map {
            let key = k.as_str();
            match key {
                "alpha" => {
                    if let Some(f) = v.as_f64() {
                        a.alpha = Some(f as f32);
                    }
                }
                "beta" => {
                    if let Some(f) = v.as_f64() {
                        a.beta = Some(f as f32);
                    }
                }
                "transA" | "trans_a" => {
                    if let Some(i) = v.as_i64() {
                        a.trans_a = Some(i);
                    }
                }
                "transB" | "trans_b" => {
                    if let Some(i) = v.as_i64() {
                        a.trans_b = Some(i);
                    }
                }
                "kernel_shape" => {
                    a.kernel_shape = json_i64_array(v);
                }
                "pads" => {
                    a.pads = json_i64_array(v);
                }
                "strides" => {
                    a.strides = json_i64_array(v);
                }
                "perm" => {
                    a.perm = json_i64_array(v);
                }
                "shape" => {
                    a.reshape_shape = json_i64_array(v);
                }
                _ => {
                    if let Some(f) = v.as_f64() {
                        // Prefer int when integral.
                        if f.fract() == 0.0 && f.abs() < (i64::MAX as f64) {
                            a.extra.insert(key.to_string(), AttrValue::I(f as i64));
                        } else {
                            a.extra.insert(key.to_string(), AttrValue::F(f as f32));
                        }
                    } else if let Some(i) = v.as_i64() {
                        a.extra.insert(key.to_string(), AttrValue::I(i));
                    } else if let Some(s) = v.as_str() {
                        a.extra.insert(key.to_string(), AttrValue::S(s.to_string()));
                    } else if let Some(arr) = json_i64_array(v) {
                        a.extra.insert(key.to_string(), AttrValue::Ints(arr));
                    }
                }
            }
        }
        a
    }
}

fn json_i64_array(v: &serde_json::Value) -> Option<Vec<i64>> {
    v.as_array().map(|arr| {
        arr.iter()
            .filter_map(|x| x.as_i64().or_else(|| x.as_f64().map(|f| f as i64)))
            .collect()
    })
}

/// A single compute node in the typed IR.
#[derive(Debug, Clone)]
pub struct IrNode {
    /// Stable id.
    pub id: NodeId,
    /// Optional node name.
    pub name: Option<String>,
    /// Operator.
    pub op: NodeOp,
    /// Original op string from ModelGraph (preserves casing; Issue 9).
    pub op_raw: String,
    /// Input tensor ids (order matches ONNX).
    pub inputs: Vec<TensorId>,
    /// Output tensor ids.
    pub outputs: Vec<TensorId>,
    /// Attributes.
    pub attrs: NodeAttrs,
    /// Per-node layout (Issue 10 — preserved across IR roundtrip).
    pub layout: Option<String>,
}

/// Typed multi-op graph IR (Phase 7).
#[derive(Debug, Clone)]
pub struct DragonNpuIR {
    /// All tensors (activations + initializers).
    pub tensors: Vec<TensorDesc>,
    /// Compute nodes in topological / ModelGraph order.
    pub nodes: Vec<IrNode>,
    /// Graph-level input tensor ids (best-effort from unused producers).
    pub graph_inputs: Vec<TensorId>,
    /// Graph-level output tensor ids (explicit sinks or unconsumed heuristic).
    pub graph_outputs: Vec<TensorId>,
    /// Preserved graph input **names** for ModelGraph roundtrip.
    pub graph_input_names: Vec<String>,
    /// Preserved graph output **names** for ModelGraph roundtrip / DCE sinks.
    pub graph_output_names: Vec<String>,
    /// Weight / constant initializers (cloned from ModelGraph).
    pub initializers: Vec<Initializer>,
    /// Layout metadata from ModelGraph.
    pub layout: Option<String>,
    /// Quantization metadata from ModelGraph.
    pub quantization: Option<String>,
    /// Memory plan bytes estimate from ModelGraph.
    pub memory_plan: u64,
    /// Name → tensor id (private bookkeeping; rebuilt on conversions).
    name_to_tensor: HashMap<String, TensorId>,
}

impl DragonNpuIR {
    /// Build typed IR from the parallel-`Vec` [`ModelGraph`].
    ///
    /// # Errors
    ///
    /// Currently infallible for well-formed graphs; returns `Internal` only if
    /// structural invariants are violated (reserved for future checks).
    #[allow(
        clippy::too_many_lines,
        clippy::collapsible_if,
        clippy::assigning_clones
    )]
    pub fn from_model_graph(graph: &ModelGraph) -> Result<Self, DragonNpuError> {
        let n = graph.ops.len();
        let mut name_to_tensor: HashMap<String, TensorId> = HashMap::new();
        let mut tensors: Vec<TensorDesc> = Vec::new();

        let mut next_tid = 0u32;
        let mut ensure_tensor = |name: &str,
                                 name_to_tensor: &mut HashMap<String, TensorId>,
                                 tensors: &mut Vec<TensorDesc>,
                                 shape: Vec<i64>,
                                 dtype: DType,
                                 is_constant: bool|
         -> TensorId {
            if let Some(&id) = name_to_tensor.get(name) {
                // Upgrade unknown shape/dtype if we learn more.
                let t = &mut tensors[id.0 as usize];
                if t.shape.is_empty() && !shape.is_empty() {
                    t.shape = shape;
                }
                if t.dtype == DType::Unknown && dtype != DType::Unknown {
                    t.dtype = dtype;
                }
                if is_constant {
                    t.is_constant = true;
                }
                return id;
            }
            let id = TensorId(next_tid);
            next_tid = next_tid.saturating_add(1);
            name_to_tensor.insert(name.to_string(), id);
            tensors.push(TensorDesc {
                id,
                name: name.to_string(),
                shape,
                dtype,
                is_constant,
            });
            id
        };

        // Register named shapes/dtypes from value_info + graph I/O (Issue 5).
        let mut named_shape_map: HashMap<String, Vec<i64>> = HashMap::new();
        let mut named_dtype_map: HashMap<String, DType> = HashMap::new();
        for (name, sh) in &graph.named_shapes {
            if !name.is_empty() && !sh.is_empty() {
                named_shape_map.insert(name.clone(), sh.clone());
            }
        }
        for (name, dt) in &graph.named_dtypes {
            if !name.is_empty() {
                named_dtype_map.insert(name.clone(), DType::from_name(dt));
            }
        }

        // Register initializers first (constants).
        for init in &graph.initializers {
            let name = if init.name.is_empty() {
                format!("init_{}", tensors.len())
            } else {
                init.name.clone()
            };
            let _ = ensure_tensor(
                &name,
                &mut name_to_tensor,
                &mut tensors,
                init.shape.clone(),
                DType::from_onnx_data_type(init.data_type),
                true,
            );
        }

        // Seed graph input / output tensors with shapes when known.
        for name in graph
            .graph_input_names
            .iter()
            .chain(graph.graph_output_names.iter())
        {
            if name.is_empty() {
                continue;
            }
            let sh = named_shape_map.get(name).cloned().unwrap_or_default();
            let dt = named_dtype_map.get(name).copied().unwrap_or(DType::Unknown);
            let _ = ensure_tensor(name, &mut name_to_tensor, &mut tensors, sh, dt, false);
        }

        let mut nodes = Vec::with_capacity(n);
        for i in 0..n {
            let op_raw = graph.ops[i].clone();
            let op = NodeOp::parse(&op_raw);
            let in_names = graph.inputs.get(i).cloned().unwrap_or_default();
            let out_names = graph.outputs.get(i).cloned().unwrap_or_default();

            let out_shape = graph.shapes.get(i).cloned().flatten().unwrap_or_default();
            let out_dtype = graph
                .dtypes
                .get(i)
                .and_then(|d| d.as_ref())
                .map_or(DType::Unknown, |s| DType::from_name(s));

            let mut input_ids = Vec::new();
            for (j, name) in in_names.iter().enumerate() {
                // Issue 9: keep empty names empty when possible; only invent
                // synthetic names when the slot is truly missing.
                let tname = if name.is_empty() {
                    format!("__node{i}_in{j}")
                } else {
                    name.clone()
                };
                let seed_sh = named_shape_map.get(&tname).cloned().unwrap_or_default();
                let seed_dt = named_dtype_map
                    .get(&tname)
                    .copied()
                    .unwrap_or(DType::Unknown);
                let id = ensure_tensor(
                    &tname,
                    &mut name_to_tensor,
                    &mut tensors,
                    seed_sh,
                    seed_dt,
                    false,
                );
                input_ids.push(id);
            }

            let mut output_ids = Vec::new();
            if out_names.is_empty() {
                // Preserve empty outputs on lower when original had none: use a
                // stable synthetic only for IR wiring, strip on to_model_graph
                // via empty-marker prefix.
                let tname = format!("__anon_out_{i}");
                let id = ensure_tensor(
                    &tname,
                    &mut name_to_tensor,
                    &mut tensors,
                    out_shape.clone(),
                    out_dtype,
                    false,
                );
                output_ids.push(id);
            } else {
                for (j, name) in out_names.iter().enumerate() {
                    let tname = if name.is_empty() {
                        format!("__anon_out_{i}_{j}")
                    } else {
                        name.clone()
                    };
                    let mut shape = if j == 0 {
                        out_shape.clone()
                    } else {
                        Vec::new()
                    };
                    if shape.is_empty() {
                        if let Some(s) = named_shape_map.get(&tname) {
                            shape = s.clone();
                        }
                    }
                    let mut dtype = if j == 0 { out_dtype } else { DType::Unknown };
                    if dtype == DType::Unknown {
                        if let Some(d) = named_dtype_map.get(&tname) {
                            dtype = *d;
                        }
                    }
                    let id = ensure_tensor(
                        &tname,
                        &mut name_to_tensor,
                        &mut tensors,
                        shape,
                        dtype,
                        false,
                    );
                    output_ids.push(id);
                }
            }

            let attrs = graph.node_attrs.get(i).cloned().unwrap_or_default();
            let layout = graph.layouts.get(i).cloned().flatten();
            let name = graph.node_names.get(i).cloned().flatten();
            nodes.push(IrNode {
                id: NodeId(u32::try_from(i).unwrap_or(u32::MAX)),
                name,
                op,
                op_raw,
                inputs: input_ids,
                outputs: output_ids,
                attrs,
                layout,
            });
        }

        let mut used_as_input: HashSet<TensorId> = HashSet::new();
        for node in &nodes {
            for &t in &node.inputs {
                used_as_input.insert(t);
            }
        }
        let mut produced: HashSet<TensorId> = HashSet::new();
        for node in &nodes {
            for &t in &node.outputs {
                produced.insert(t);
            }
        }

        // Explicit graph outputs from ModelGraph when present (Issue 3).
        let mut graph_outputs = Vec::new();
        if !graph.graph_output_names.is_empty() {
            for name in &graph.graph_output_names {
                if let Some(&id) = name_to_tensor.get(name) {
                    if !graph_outputs.contains(&id) {
                        graph_outputs.push(id);
                    }
                }
            }
        }
        if graph_outputs.is_empty() {
            // Heuristic: unconsumed producers (safe for unconnected heuristic graphs).
            for node in &nodes {
                for &t in &node.outputs {
                    if !used_as_input.contains(&t) && !graph_outputs.contains(&t) {
                        graph_outputs.push(t);
                    }
                }
            }
        }
        if graph_outputs.is_empty()
            && let Some(last) = nodes.last()
        {
            graph_outputs.clone_from(&last.outputs);
        }

        // Graph inputs: explicit names first, else unused non-const producers.
        let mut graph_inputs = Vec::new();
        if graph.graph_input_names.is_empty() {
            for node in &nodes {
                for &t in &node.inputs {
                    if !produced.contains(&t) {
                        let is_const = tensors.get(t.0 as usize).is_some_and(|td| td.is_constant);
                        if !is_const && !graph_inputs.contains(&t) {
                            graph_inputs.push(t);
                        }
                    }
                }
            }
        } else {
            for name in &graph.graph_input_names {
                if let Some(&id) = name_to_tensor.get(name) {
                    if !graph_inputs.contains(&id) {
                        graph_inputs.push(id);
                    }
                }
            }
        }

        Ok(Self {
            tensors,
            nodes,
            graph_inputs,
            graph_outputs,
            graph_input_names: graph.graph_input_names.clone(),
            graph_output_names: graph.graph_output_names.clone(),
            initializers: graph.initializers.clone(),
            layout: graph.layout.clone(),
            quantization: graph.quantization.clone(),
            memory_plan: graph.memory_plan,
            name_to_tensor,
        })
    }

    /// Lower typed IR back to [`ModelGraph`] (parallel Vec form).
    #[must_use]
    #[allow(clippy::too_many_lines, clippy::similar_names)]
    pub fn to_model_graph(&self) -> ModelGraph {
        let n = self.nodes.len();
        let mut ops = Vec::with_capacity(n);
        let mut node_names = Vec::with_capacity(n);
        let mut shapes = Vec::with_capacity(n);
        let mut dtypes = Vec::with_capacity(n);
        let mut inputs = Vec::with_capacity(n);
        let mut outputs = Vec::with_capacity(n);
        let mut layouts = Vec::with_capacity(n);
        let mut node_attrs = Vec::with_capacity(n);
        let mut named_shapes = Vec::new();
        let mut named_dtypes = Vec::new();

        for t in &self.tensors {
            if t.name.starts_with("__") {
                continue;
            }
            if !t.shape.is_empty() {
                named_shapes.push((t.name.clone(), t.shape.clone()));
            }
            if t.dtype != DType::Unknown {
                named_dtypes.push((t.name.clone(), t.dtype.as_str().to_string()));
            }
        }

        for node in &self.nodes {
            // Issue 9: preserve original op string when fold/fuse did not change kind.
            let op_str = match &node.op {
                NodeOp::Constant => "Constant".to_string(),
                NodeOp::Fused(s) => s.clone(),
                _ if !node.op_raw.is_empty() => node.op_raw.clone(),
                _ => node.op.as_op_string(),
            };
            ops.push(op_str);
            node_names.push(node.name.clone());
            let (shape, dtype) = node
                .outputs
                .first()
                .and_then(|id| self.tensors.get(id.0 as usize))
                .map_or((None, None), |t| {
                    let sh = if t.shape.is_empty() {
                        None
                    } else {
                        Some(t.shape.clone())
                    };
                    let dt = if t.dtype == DType::Unknown {
                        None
                    } else {
                        Some(t.dtype.as_str().to_string())
                    };
                    (sh, dt)
                });
            shapes.push(shape);
            dtypes.push(dtype);
            inputs.push(
                node.inputs
                    .iter()
                    .filter_map(|id| {
                        self.tensors.get(id.0 as usize).map(|t| {
                            if t.name.starts_with("__anon_") {
                                String::new()
                            } else {
                                t.name.clone()
                            }
                        })
                    })
                    .collect(),
            );
            let out_names: Vec<String> = node
                .outputs
                .iter()
                .filter_map(|id| self.tensors.get(id.0 as usize).map(|t| t.name.clone()))
                .collect();
            if out_names.iter().all(|nm| nm.starts_with("__anon_")) {
                outputs.push(vec![]);
            } else {
                outputs.push(
                    out_names
                        .into_iter()
                        .map(|nm| {
                            if nm.starts_with("__anon_") {
                                String::new()
                            } else {
                                nm
                            }
                        })
                        .collect(),
                );
            }
            node_attrs.push(node.attrs.clone());
            layouts.push(node.layout.clone().or_else(|| self.layout.clone()));
        }

        ModelGraph {
            ops,
            memory_plan: self.memory_plan,
            layout: self.layout.clone(),
            quantization: self.quantization.clone(),
            node_names,
            shapes,
            dtypes,
            inputs,
            outputs,
            layouts,
            initializer_count: self.initializers.len(),
            initializers: self.initializers.clone(),
            node_attrs,
            graph_input_names: self.graph_input_names.clone(),
            graph_output_names: self.graph_output_names.clone(),
            named_shapes,
            named_dtypes,
        }
    }

    /// Infer output shapes for nodes when input ranks are known.
    ///
    /// Unknown dims stay as `-1`. Never invents silicon / tile geometry.
    ///
    /// # Errors
    ///
    /// Returns `Internal` only on index corruption (should not happen for IR
    /// built via [`from_model_graph`]).
    #[allow(clippy::collapsible_if)]
    pub fn infer_shapes(&mut self) -> Result<(), DragonNpuError> {
        // Multiple passes for forward chains.
        for _ in 0..self.nodes.len().saturating_add(1).max(1) {
            let mut changed = false;
            for ni in 0..self.nodes.len() {
                let (op, in_ids, out_ids, attrs) = {
                    let n = &self.nodes[ni];
                    (
                        n.op.clone(),
                        n.inputs.clone(),
                        n.outputs.clone(),
                        n.attrs.clone(),
                    )
                };
                let in_shapes: Vec<Vec<i64>> = in_ids
                    .iter()
                    .map(|id| {
                        self.tensors
                            .get(id.0 as usize)
                            .map_or_else(Vec::new, |t| t.shape.clone())
                    })
                    .collect();
                let in_dtypes: Vec<DType> = in_ids
                    .iter()
                    .map(|id| {
                        self.tensors
                            .get(id.0 as usize)
                            .map_or(DType::Unknown, |t| t.dtype)
                    })
                    .collect();

                let inferred = infer_op_shape(&op, &in_shapes, &attrs);
                let out_dtype = in_dtypes
                    .iter()
                    .copied()
                    .find(|d| *d != DType::Unknown)
                    .unwrap_or(DType::Unknown);

                if let Some(shape) = inferred {
                    if let Some(&oid) = out_ids.first() {
                        if let Some(t) = self.tensors.get_mut(oid.0 as usize) {
                            if t.shape != shape {
                                t.shape = shape;
                                changed = true;
                            }
                            if t.dtype == DType::Unknown && out_dtype != DType::Unknown {
                                t.dtype = out_dtype;
                                changed = true;
                            }
                        }
                    }
                }
            }
            if !changed {
                break;
            }
        }
        Ok(())
    }

    /// Constant-fold cheap pure ops when all inputs are constant and small.
    ///
    /// Folds: Identity chains, Relu(const), Add/Mul of two small const tensors.
    /// Result replaces the node with `Constant` (host allowlisted — loads initializer)
    /// and materializes the value under the output tensor name.
    ///
    /// Runs to a **fixed point** (Issue 2) with a safe iteration cap so consumer
    /// nodes that appear before producers still fold across passes.
    ///
    /// Returns the total number of nodes folded across all iterations.
    #[allow(clippy::too_many_lines, clippy::collapsible_if)]
    pub fn constant_fold(&mut self) -> Result<usize, DragonNpuError> {
        const MAX_FOLD_ELEMS: usize = 4096;
        const MAX_ITERS: usize = 64;
        let mut total_folded = 0usize;

        for _iter in 0..MAX_ITERS {
            let mut const_data: HashMap<TensorId, Vec<f32>> = HashMap::new();
            for init in &self.initializers {
                if init.name.is_empty() {
                    continue;
                }
                if let Some(&tid) = self.name_to_tensor.get(&init.name) {
                    let data = init.as_f32_data();
                    if !data.is_empty() && data.len() <= MAX_FOLD_ELEMS {
                        const_data.insert(tid, data);
                    }
                }
            }
            // Seed already-folded Constant outputs.
            for node in &self.nodes {
                if matches!(node.op, NodeOp::Constant) {
                    if let Some(&oid) = node.outputs.first() {
                        if let Some(t) = self.tensors.get(oid.0 as usize) {
                            if let Some(init) = self.initializers.iter().find(|i| i.name == t.name)
                            {
                                let data = init.as_f32_data();
                                if !data.is_empty() {
                                    const_data.insert(oid, data);
                                }
                            }
                        }
                    }
                }
            }

            let mut pass_folded = 0usize;
            for ni in 0..self.nodes.len() {
                if matches!(self.nodes[ni].op, NodeOp::Constant) {
                    continue; // already folded
                }
                let (op, in_ids, out_ids) = {
                    let n = &self.nodes[ni];
                    (n.op.clone(), n.inputs.clone(), n.outputs.clone())
                };

                let fold_result: Option<Vec<f32>> = match &op {
                    NodeOp::Identity => in_ids.first().and_then(|a| const_data.get(a).cloned()),
                    NodeOp::Relu => in_ids.first().and_then(|a| {
                        const_data
                            .get(a)
                            .map(|v| v.iter().map(|&x| if x > 0.0 { x } else { 0.0 }).collect())
                    }),
                    NodeOp::Add if in_ids.len() >= 2 => {
                        match (const_data.get(&in_ids[0]), const_data.get(&in_ids[1])) {
                            (Some(a), Some(b))
                                if a.len() == b.len() && a.len() <= MAX_FOLD_ELEMS =>
                            {
                                Some(a.iter().zip(b.iter()).map(|(x, y)| x + y).collect())
                            }
                            (Some(a), Some(b)) if b.len() == 1 && a.len() <= MAX_FOLD_ELEMS => {
                                Some(a.iter().map(|x| x + b[0]).collect())
                            }
                            (Some(a), Some(b)) if a.len() == 1 && b.len() <= MAX_FOLD_ELEMS => {
                                Some(b.iter().map(|x| a[0] + x).collect())
                            }
                            _ => None,
                        }
                    }
                    NodeOp::Mul if in_ids.len() >= 2 => {
                        match (const_data.get(&in_ids[0]), const_data.get(&in_ids[1])) {
                            (Some(a), Some(b))
                                if a.len() == b.len() && a.len() <= MAX_FOLD_ELEMS =>
                            {
                                Some(a.iter().zip(b.iter()).map(|(x, y)| x * y).collect())
                            }
                            (Some(a), Some(b)) if b.len() == 1 && a.len() <= MAX_FOLD_ELEMS => {
                                Some(a.iter().map(|x| x * b[0]).collect())
                            }
                            (Some(a), Some(b)) if a.len() == 1 && b.len() <= MAX_FOLD_ELEMS => {
                                Some(b.iter().map(|x| a[0] * x).collect())
                            }
                            _ => None,
                        }
                    }
                    _ => None,
                };

                if let Some(data) = fold_result {
                    if let Some(&oid) = out_ids.first() {
                        const_data.insert(oid, data.clone());
                        let out_name = if let Some(t) = self.tensors.get_mut(oid.0 as usize) {
                            t.is_constant = true;
                            if t.shape.is_empty() {
                                t.shape = vec![i64::try_from(data.len()).unwrap_or(-1)];
                            }
                            if t.dtype == DType::Unknown {
                                t.dtype = DType::F32;
                            }
                            // Prefer real names; for anon, invent a stable fold name.
                            if t.name.starts_with("__anon_") || t.name.starts_with("__node") {
                                let fold_name = format!("__folded_{}", oid.0);
                                t.name.clone_from(&fold_name);
                                self.name_to_tensor.insert(fold_name.clone(), oid);
                                fold_name
                            } else {
                                t.name.clone()
                            }
                        } else {
                            continue;
                        };

                        self.nodes[ni].op = NodeOp::Constant;
                        self.nodes[ni].op_raw = "Constant".to_string();
                        self.nodes[ni].inputs.clear();
                        pass_folded += 1;

                        // Upsert initializer for host_ref Constant load / RHS bind.
                        let fallback_shape = vec![i64::try_from(data.len()).unwrap_or(-1)];
                        if let Some(existing) =
                            self.initializers.iter_mut().find(|i| i.name == out_name)
                        {
                            existing.data_f32 = data;
                            existing.data_type = 1;
                            existing.shape = self
                                .tensors
                                .get(oid.0 as usize)
                                .map_or_else(Vec::new, |t| t.shape.clone());
                        } else {
                            let shape = self
                                .tensors
                                .get(oid.0 as usize)
                                .map_or_else(|| fallback_shape, |t| t.shape.clone());
                            self.initializers.push(Initializer {
                                name: out_name,
                                shape,
                                data_type: 1,
                                data_f32: data,
                                data_i8: vec![],
                                data_int32: vec![],
                                data_int64: vec![],
                                raw_data: vec![],
                            });
                        }
                    }
                }
            }

            total_folded += pass_folded;
            if pass_folded == 0 {
                break;
            }
        }
        Ok(total_folded)
    }

    /// Remove nodes whose outputs are not used by any live consumer or graph output.
    ///
    /// Live roots are [`Self::graph_outputs`]. When empty, all unconsumed node
    /// outputs are treated as sinks (safe for unconnected heuristic graphs).
    ///
    /// Returns the number of nodes removed.
    #[allow(clippy::collapsible_if)]
    pub fn dead_code_elim(&mut self) -> Result<usize, DragonNpuError> {
        if self.nodes.is_empty() {
            return Ok(0);
        }

        let mut used_as_input: HashSet<TensorId> = HashSet::new();
        for node in &self.nodes {
            for &t in &node.inputs {
                used_as_input.insert(t);
            }
        }

        let mut live_tensors: HashSet<TensorId> = HashSet::new();
        for &t in &self.graph_outputs {
            live_tensors.insert(t);
        }
        // If graph_outputs empty, every unconsumed producer is a sink.
        if live_tensors.is_empty() {
            for node in &self.nodes {
                for &t in &node.outputs {
                    if !used_as_input.contains(&t) {
                        live_tensors.insert(t);
                    }
                }
            }
        }
        if live_tensors.is_empty() {
            if let Some(last) = self.nodes.last() {
                for &t in &last.outputs {
                    live_tensors.insert(t);
                }
            }
        }

        // Backward mark.
        let mut changed = true;
        while changed {
            changed = false;
            for node in self.nodes.iter().rev() {
                let produces_live = node.outputs.iter().any(|o| live_tensors.contains(o));
                if produces_live {
                    for &inp in &node.inputs {
                        if live_tensors.insert(inp) {
                            changed = true;
                        }
                    }
                }
            }
        }

        let before = self.nodes.len();
        self.nodes
            .retain(|n| n.outputs.iter().any(|o| live_tensors.contains(o)));
        // Re-index node ids for stability (optional hygiene).
        for (i, node) in self.nodes.iter_mut().enumerate() {
            node.id = NodeId(u32::try_from(i).unwrap_or(u32::MAX));
        }
        Ok(before.saturating_sub(self.nodes.len()))
    }

    /// Run shape infer → constant fold (fixed-point) → DCE (optimization level ≥2).
    ///
    /// # Errors
    ///
    /// Propagates shape/fold/DCE errors.
    pub fn optimize_level2(&mut self) -> Result<IrOptStats, DragonNpuError> {
        self.infer_shapes()?;
        // constant_fold already iterates to fixed point (Issue 2).
        let folded = self.constant_fold()?;
        let eliminated = self.dead_code_elim()?;
        // Re-infer after fold/DCE; fold once more if DCE exposed pure const chains.
        self.infer_shapes()?;
        let folded2 = self.constant_fold()?;
        let eliminated2 = self.dead_code_elim()?;
        self.infer_shapes()?;
        Ok(IrOptStats {
            folded: folded.saturating_add(folded2),
            eliminated: eliminated.saturating_add(eliminated2),
        })
    }
}

/// Stats from IR optimize_level2.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IrOptStats {
    /// Nodes constant-folded.
    pub folded: usize,
    /// Nodes removed by DCE.
    pub eliminated: usize,
}

#[allow(clippy::collapsible_if)]
fn infer_op_shape(op: &NodeOp, inputs: &[Vec<i64>], attrs: &NodeAttrs) -> Option<Vec<i64>> {
    match op {
        NodeOp::Identity | NodeOp::Relu | NodeOp::Constant => {
            inputs.first().cloned().filter(|s| !s.is_empty())
        }
        NodeOp::Add | NodeOp::Mul => {
            let a = inputs.first()?;
            let b = inputs.get(1)?;
            if a.is_empty() && b.is_empty() {
                return None;
            }
            if a.is_empty() {
                return Some(b.clone());
            }
            if b.is_empty() {
                return Some(a.clone());
            }
            Some(broadcast_shape(a, b))
        }
        NodeOp::MatMul => matmul_out_shape(inputs),
        NodeOp::Gemm => gemm_out_shape(inputs, attrs),
        NodeOp::Fused(s) if s.contains("matmul") => matmul_out_shape(inputs),
        NodeOp::Fused(s) if s.contains("gemm") => gemm_out_shape(inputs, attrs),
        NodeOp::Fused(s) if s.contains("conv") => infer_conv_shape(inputs, attrs),
        NodeOp::Reshape => {
            if let Some(ref sh) = attrs.reshape_shape {
                return Some(sh.clone());
            }
            // Second input may be shape tensor — unknown without const eval.
            inputs.first().cloned().filter(|s| !s.is_empty())
        }
        NodeOp::Transpose => {
            let a = inputs.first()?;
            if a.is_empty() {
                return None;
            }
            if let Some(ref perm) = attrs.perm {
                if perm.len() == a.len() {
                    let mut out = vec![-1_i64; a.len()];
                    for (i, &p) in perm.iter().enumerate() {
                        if let Ok(pi) = usize::try_from(p) {
                            if pi < a.len() {
                                out[i] = a[pi];
                            }
                        }
                    }
                    return Some(out);
                }
            }
            // Default reverse.
            let mut out = a.clone();
            out.reverse();
            Some(out)
        }
        NodeOp::Conv => infer_conv_shape(inputs, attrs),
        NodeOp::Unknown(_) | NodeOp::Fused(_) => None,
    }
}

#[allow(clippy::collapsible_if)]
fn infer_conv_shape(inputs: &[Vec<i64>], attrs: &NodeAttrs) -> Option<Vec<i64>> {
    // Honest: NCHW [N,C,H,W] → [N, Cout, H', W'] with H'/W' often unknown.
    let a = inputs.first()?;
    if a.len() >= 4 {
        let mut out = a.clone();
        if let Some(w) = inputs.get(1) {
            if !w.is_empty() && w[0] > 0 {
                out[1] = w[0];
            }
        }
        if attrs.kernel_shape.is_some() {
            for d in out.iter_mut().skip(2) {
                *d = -1;
            }
        }
        return Some(out);
    }
    if a.is_empty() { None } else { Some(a.clone()) }
}

/// MatMul output shape (v1).
///
/// **Limitation (Issue 7):** batch / leading dims are taken from **A only**.
/// ONNX allows broadcasting B's leading dims against A; that is not implemented —
/// rank-mismatched batched MatMul may yield incomplete shapes (`-1` or A-prefix only).
fn matmul_out_shape(inputs: &[Vec<i64>]) -> Option<Vec<i64>> {
    let a = inputs.first()?;
    let b = inputs.get(1)?;
    if a.len() < 2 || b.len() < 2 {
        return None;
    }
    // A[..., M, K] @ B[..., K, N] → [..., M, N]  (batch prefix from A only — see doc).
    let m = a[a.len() - 2];
    let k_a = a[a.len() - 1];
    let k_b = b[b.len() - 2];
    let n = b[b.len() - 1];
    if k_a > 0 && k_b > 0 && k_a != k_b {
        // Inconsistent — still emit shape with M,N; honesty: leave as-is for host to reject.
    }
    let mut prefix = if a.len() > 2 {
        a[..a.len() - 2].to_vec()
    } else {
        vec![]
    };
    prefix.push(m);
    prefix.push(n);
    Some(prefix)
}

fn gemm_out_shape(inputs: &[Vec<i64>], attrs: &NodeAttrs) -> Option<Vec<i64>> {
    let a = inputs.first()?;
    let b = inputs.get(1)?;
    if a.len() < 2 || b.len() < 2 {
        return None;
    }
    let trans_a = attrs.trans_a.unwrap_or(0) != 0;
    let trans_b = attrs.trans_b.unwrap_or(0) != 0;
    let (m, _k_a) = if trans_a {
        (a[a.len() - 1], a[a.len() - 2])
    } else {
        (a[a.len() - 2], a[a.len() - 1])
    };
    let (_k_b, n) = if trans_b {
        (b[b.len() - 1], b[b.len() - 2])
    } else {
        (b[b.len() - 2], b[b.len() - 1])
    };
    Some(vec![m, n])
}

fn broadcast_shape(a: &[i64], b: &[i64]) -> Vec<i64> {
    let rank = a.len().max(b.len());
    let mut out = vec![-1_i64; rank];
    for i in 0..rank {
        let da = if i < rank - a.len() {
            1
        } else {
            a[i - (rank - a.len())]
        };
        let db = if i < rank - b.len() {
            1
        } else {
            b[i - (rank - b.len())]
        };
        out[i] = if da == db {
            da
        } else if da == 1 || da == -1 {
            db
        } else if db == 1 || db == -1 {
            da
        } else {
            // Conflict — keep -1 rather than invent.
            -1
        };
    }
    out
}

/// Heuristic XDNA2 cost estimate — **not** measured silicon time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CostEstimate {
    /// Elementwise / host-cheap op count.
    pub host_ops: u32,
    /// Dense compute candidates (MatMul/Gemm/Conv/fused dense).
    pub npu_candidate_ops: u32,
    /// Rough MAC count from known shapes (0 when unknown).
    pub rough_macs: u64,
    /// Suggested tile count used for notes (caller-provided heuristic).
    pub tiles: u32,
    /// `tops_max` used for notes.
    pub tops_max: u64,
    /// Human notes (honesty: not wall-clock).
    pub notes: Vec<String>,
}

/// Typical amdxdna DevHeap size used only for static budget notes (not a hard limit).
const DEVHEAP_BUDGET_BYTES: u64 = 64 * 1024 * 1024;

/// Estimate XDNA2-oriented cost for an IR graph.
///
/// # Honesty
///
/// This is a **static heuristic** for compile-time planning only. It does **not**
/// measure driver EXEC_CMD latency, does not claim `ran_on_npu`, and does not
/// equal silicon GEMM TOPS.
#[must_use]
pub fn estimate_xdna2_cost(ir: &DragonNpuIR, tiles: u32, tops_max: u64) -> CostEstimate {
    let mut host_ops = 0u32;
    let mut npu_candidate_ops = 0u32;
    let mut rough_macs = 0u64;

    for node in &ir.nodes {
        if node.op.is_npu_candidate() {
            npu_candidate_ops = npu_candidate_ops.saturating_add(1);
            rough_macs = rough_macs.saturating_add(estimate_node_macs(ir, node));
        } else {
            host_ops = host_ops.saturating_add(1);
        }
    }

    let mut notes = vec![
        "estimate_xdna2_cost is a host-side heuristic; not measured silicon time".to_string(),
        format!("tiles={tiles} (caller-supplied; multiples of 4 typical for AIE2)"),
        format!("tops_max={tops_max} (device resource query or default; not achieved TOPS)"),
    ];
    if npu_candidate_ops > 0 && rough_macs == 0 {
        notes.push(
            "npu_candidate ops present but rough_macs=0 (missing shapes); cannot score MACs"
                .to_string(),
        );
    }
    if tiles > 0 && tops_max > 0 && rough_macs > 0 {
        // Purely illustrative throughput bound — not a runtime prediction.
        notes.push(format!(
            "rough_macs={rough_macs}; if peak tops were free, lower-bound time is illustrative only"
        ));
    }
    // Static memory vs DevHeap (64 MiB typical) — honest order-of-magnitude only.
    let weight_bytes: u64 = ir
        .initializers
        .iter()
        .map(|init| {
            let elems = init.as_f32_data().len() as u64;
            elems.saturating_mul(4)
        })
        .sum();
    notes.push(format!(
        "static_weight_bytes≈{weight_bytes} vs typical DevHeap budget {DEVHEAP_BUDGET_BYTES} (heuristic only)"
    ));
    if weight_bytes > DEVHEAP_BUDGET_BYTES {
        notes.push(
            "weight payload may exceed default DevHeap; plan sharding or host path (not auto-applied)"
                .to_string(),
        );
    }

    CostEstimate {
        host_ops,
        npu_candidate_ops,
        rough_macs,
        tiles,
        tops_max,
        notes,
    }
}

#[allow(clippy::many_single_char_names)]
fn estimate_node_macs(ir: &DragonNpuIR, node: &IrNode) -> u64 {
    let shapes: Vec<Vec<i64>> = node
        .inputs
        .iter()
        .map(|id| {
            ir.tensors
                .get(id.0 as usize)
                .map_or_else(Vec::new, |t| t.shape.clone())
        })
        .collect();
    let is_matmul = matches!(node.op, NodeOp::MatMul)
        || matches!(&node.op, NodeOp::Fused(s) if s.contains("matmul"));
    let is_gemm = matches!(node.op, NodeOp::Gemm)
        || matches!(&node.op, NodeOp::Fused(s) if s.contains("gemm"));
    let is_conv = matches!(node.op, NodeOp::Conv)
        || matches!(&node.op, NodeOp::Fused(s) if s.contains("conv"));

    if is_matmul {
        if shapes.len() >= 2 && shapes[0].len() >= 2 && shapes[1].len() >= 2 {
            let a = &shapes[0];
            let b = &shapes[1];
            let m = a[a.len() - 2].max(0) as u64;
            let k = a[a.len() - 1].max(0) as u64;
            let n_dim = b[b.len() - 1].max(0) as u64;
            return m.saturating_mul(n_dim).saturating_mul(k);
        }
        return 0;
    }
    if is_gemm {
        if shapes.len() >= 2 && shapes[0].len() >= 2 && shapes[1].len() >= 2 {
            let a = &shapes[0];
            let b = &shapes[1];
            let trans_a = node.attrs.trans_a.unwrap_or(0) != 0;
            let trans_b = node.attrs.trans_b.unwrap_or(0) != 0;
            let (m, k) = if trans_a {
                (a[a.len() - 1].max(0) as u64, a[a.len() - 2].max(0) as u64)
            } else {
                (a[a.len() - 2].max(0) as u64, a[a.len() - 1].max(0) as u64)
            };
            let n = if trans_b {
                b[b.len() - 2].max(0) as u64
            } else {
                b[b.len() - 1].max(0) as u64
            };
            return m.saturating_mul(n).saturating_mul(k);
        }
        return 0;
    }
    if is_conv {
        // Rough: N * Cout * H * W * Cin * kH * kW when all known.
        if shapes.len() >= 2 && shapes[0].len() >= 4 && shapes[1].len() >= 4 {
            let x = &shapes[0];
            let w = &shapes[1];
            let n = x[0].max(0) as u64;
            let h = x[2].max(0) as u64;
            let ww = x[3].max(0) as u64;
            let cout = w[0].max(0) as u64;
            let cin = w[1].max(0) as u64;
            let kh = w[2].max(0) as u64;
            let kw = w[3].max(0) as u64;
            return n
                .saturating_mul(cout)
                .saturating_mul(h)
                .saturating_mul(ww)
                .saturating_mul(cin)
                .saturating_mul(kh)
                .saturating_mul(kw);
        }
        return 0;
    }
    0
}

/// Parse ONNX AttributeProto list into [`NodeAttrs`] (Gemm/Conv class).
///
/// Used by the ONNX loader path — pure, no I/O.
#[must_use]
pub fn node_attrs_from_onnx_attributes(attrs: &[tract_onnx::pb::AttributeProto]) -> NodeAttrs {
    let mut out = NodeAttrs::default();
    for a in attrs {
        let name = a.name.as_str();
        match name {
            "alpha" => out.alpha = Some(a.f),
            "beta" => out.beta = Some(a.f),
            "transA" => out.trans_a = Some(a.i),
            "transB" => out.trans_b = Some(a.i),
            "kernel_shape" => {
                if !a.ints.is_empty() {
                    out.kernel_shape = Some(a.ints.clone());
                }
            }
            "pads" => {
                if !a.ints.is_empty() {
                    out.pads = Some(a.ints.clone());
                }
            }
            "strides" => {
                if !a.ints.is_empty() {
                    out.strides = Some(a.ints.clone());
                }
            }
            "perm" => {
                if !a.ints.is_empty() {
                    out.perm = Some(a.ints.clone());
                }
            }
            "shape" => {
                if !a.ints.is_empty() {
                    out.reshape_shape = Some(a.ints.clone());
                }
            }
            _ => {
                // Issue 6: use AttributeProto type — do not drop zero-valued scalars.
                // ONNX AttributeType: FLOAT=1, INT=2, STRING=3, FLOATS=6, INTS=7.
                if !a.floats.is_empty() {
                    out.extra
                        .insert(name.to_string(), AttrValue::Floats(a.floats.clone()));
                } else if !a.ints.is_empty() {
                    out.extra
                        .insert(name.to_string(), AttrValue::Ints(a.ints.clone()));
                } else if !a.s.is_empty() || a.r#type == 3 {
                    out.extra.insert(
                        name.to_string(),
                        AttrValue::S(String::from_utf8_lossy(&a.s).into_owned()),
                    );
                } else if a.r#type == 1 {
                    out.extra.insert(name.to_string(), AttrValue::F(a.f));
                } else if a.r#type == 2 {
                    out.extra.insert(name.to_string(), AttrValue::I(a.i));
                } else {
                    // Type unset (0): still record a scalar so zeros are not dropped.
                    // Prefer INT (common for flags); use FLOAT when f is the non-zero field.
                    if a.f == 0.0 {
                        out.extra.insert(name.to_string(), AttrValue::I(a.i));
                    } else {
                        out.extra.insert(name.to_string(), AttrValue::F(a.f));
                    }
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::ModelGraph;

    fn simple_matmul_relu_graph() -> ModelGraph {
        ModelGraph {
            ops: vec!["MatMul".to_string(), "Relu".to_string()],
            memory_plan: 0,
            layout: None,
            quantization: None,
            node_names: vec![Some("mm".to_string()), Some("act".to_string())],
            shapes: vec![Some(vec![2, 4]), Some(vec![2, 4])],
            dtypes: vec![Some("fp32".to_string()), Some("fp32".to_string())],
            inputs: vec![
                vec!["x".to_string(), "W".to_string()],
                vec!["y".to_string()],
            ],
            outputs: vec![vec!["y".to_string()], vec!["z".to_string()]],
            layouts: vec![None, None],
            initializer_count: 1,
            initializers: vec![Initializer {
                name: "W".to_string(),
                shape: vec![3, 4],
                data_type: 1,
                data_f32: vec![1.0; 12],
                data_i8: vec![],
                data_int32: vec![],
                data_int64: vec![],
                raw_data: vec![],
            }],
            node_attrs: vec![NodeAttrs::default(), NodeAttrs::default()],
            graph_input_names: vec![],
            graph_output_names: vec![],
            named_shapes: vec![],
            named_dtypes: vec![],
        }
    }

    #[test]
    fn ir_roundtrip_preserves_ops_and_edges() {
        let g = simple_matmul_relu_graph();
        let ir = DragonNpuIR::from_model_graph(&g).expect("from");
        assert_eq!(ir.nodes.len(), 2);
        assert!(matches!(ir.nodes[0].op, NodeOp::MatMul));
        assert!(matches!(ir.nodes[1].op, NodeOp::Relu));
        let back = ir.to_model_graph();
        assert_eq!(back.ops.len(), 2);
        assert_eq!(back.ops[0], "MatMul");
        assert_eq!(back.ops[1], "Relu");
        assert_eq!(back.inputs[0], vec!["x".to_string(), "W".to_string()]);
        assert_eq!(back.outputs[1], vec!["z".to_string()]);
        assert_eq!(back.initializers.len(), 1);
        assert_eq!(back.initializers[0].name, "W");
    }

    #[test]
    fn shape_infer_matmul_add_relu() {
        let g = ModelGraph {
            ops: vec!["MatMul".to_string(), "Add".to_string(), "Relu".to_string()],
            memory_plan: 0,
            layout: None,
            quantization: None,
            node_names: vec![None, None, None],
            shapes: vec![None, None, None],
            dtypes: vec![None, None, None],
            inputs: vec![
                vec!["x".to_string(), "W".to_string()],
                vec!["mm".to_string(), "bias".to_string()],
                vec!["add".to_string()],
            ],
            outputs: vec![
                vec!["mm".to_string()],
                vec!["add".to_string()],
                vec!["out".to_string()],
            ],
            layouts: vec![None, None, None],
            initializer_count: 2,
            initializers: vec![
                Initializer {
                    name: "W".to_string(),
                    shape: vec![8, 16],
                    data_type: 1,
                    data_f32: vec![0.0; 128],
                    ..Default::default()
                },
                Initializer {
                    name: "bias".to_string(),
                    shape: vec![16],
                    data_type: 1,
                    data_f32: vec![0.0; 16],
                    ..Default::default()
                },
            ],
            node_attrs: vec![NodeAttrs::default(); 3],
            graph_input_names: vec![],
            graph_output_names: vec![],
            named_shapes: vec![],
            named_dtypes: vec![],
        };
        // Seed activation shape via a value name — x is graph input; set via tensor after build.
        let mut ir = DragonNpuIR::from_model_graph(&g).expect("from");
        // x unknown rank until we set it; set x to [4, 8]
        if let Some(&xid) = ir.name_to_tensor.get("x") {
            ir.tensors[xid.0 as usize].shape = vec![4, 8];
            ir.tensors[xid.0 as usize].dtype = DType::F32;
        }
        ir.infer_shapes().expect("infer");
        let mm = ir.name_to_tensor.get("mm").copied().expect("mm");
        assert_eq!(ir.tensors[mm.0 as usize].shape, vec![4, 16]);
        let add = ir.name_to_tensor.get("add").copied().expect("add");
        assert_eq!(ir.tensors[add.0 as usize].shape, vec![4, 16]);
        let out = ir.name_to_tensor.get("out").copied().expect("out");
        assert_eq!(ir.tensors[out.0 as usize].shape, vec![4, 16]);
    }

    #[test]
    fn constant_fold_relu_and_add() {
        let g = ModelGraph {
            ops: vec!["Relu".to_string(), "Add".to_string()],
            memory_plan: 0,
            layout: None,
            quantization: None,
            node_names: vec![None, None],
            shapes: vec![Some(vec![3]), Some(vec![3])],
            dtypes: vec![Some("fp32".to_string()), Some("fp32".to_string())],
            inputs: vec![
                vec!["c0".to_string()],
                vec!["r".to_string(), "c1".to_string()],
            ],
            outputs: vec![vec!["r".to_string()], vec!["out".to_string()]],
            layouts: vec![None, None],
            initializer_count: 2,
            initializers: vec![
                Initializer {
                    name: "c0".to_string(),
                    shape: vec![3],
                    data_type: 1,
                    data_f32: vec![-1.0, 2.0, -3.0],
                    ..Default::default()
                },
                Initializer {
                    name: "c1".to_string(),
                    shape: vec![3],
                    data_type: 1,
                    data_f32: vec![1.0, 1.0, 1.0],
                    ..Default::default()
                },
            ],
            node_attrs: vec![NodeAttrs::default(); 2],
            graph_input_names: vec![],
            graph_output_names: vec![],
            named_shapes: vec![],
            named_dtypes: vec![],
        };
        let mut ir = DragonNpuIR::from_model_graph(&g).expect("from");
        // Fixed-point fold should fold Relu then Add in one call (Issue 2).
        let n = ir.constant_fold().expect("fold");
        assert!(
            n >= 2,
            "expected Relu+Add fold in one fixed-point call, got {n}"
        );
        assert!(
            ir.nodes.iter().all(|n| matches!(n.op, NodeOp::Constant)),
            "expected both nodes Constant after fold"
        );
        let out_init = ir
            .initializers
            .iter()
            .find(|i| i.name == "out")
            .expect("out initializer");
        assert_eq!(out_init.data_f32, vec![1.0, 3.0, 1.0]);
    }

    #[test]
    fn constant_fold_order_independent_add_before_relu() {
        // Issue 2: consumer appears before producer in node list — fixed-point still folds.
        let g = ModelGraph {
            ops: vec!["Add".to_string(), "Relu".to_string()],
            memory_plan: 0,
            layout: None,
            quantization: None,
            node_names: vec![None, None],
            shapes: vec![Some(vec![2]), Some(vec![2])],
            dtypes: vec![None, None],
            // Add uses c0,c1; Relu uses add_out. Order in vec is Add then Relu (natural).
            // Reverse-data: put Relu first in node list but Relu depends on Add.
            inputs: vec![
                vec!["c0".to_string(), "c1".to_string()],
                vec!["s".to_string()],
            ],
            outputs: vec![vec!["s".to_string()], vec!["out".to_string()]],
            layouts: vec![None, None],
            initializer_count: 2,
            initializers: vec![
                Initializer {
                    name: "c0".to_string(),
                    shape: vec![2],
                    data_type: 1,
                    data_f32: vec![-2.0, 3.0],
                    ..Default::default()
                },
                Initializer {
                    name: "c1".to_string(),
                    shape: vec![2],
                    data_type: 1,
                    data_f32: vec![1.0, 1.0],
                    ..Default::default()
                },
            ],
            node_attrs: vec![NodeAttrs::default(); 2],
            graph_input_names: vec![],
            graph_output_names: vec!["out".to_string()],
            named_shapes: vec![],
            named_dtypes: vec![],
        };
        // Reverse op order: Relu then Add (Relu listed first, uses output of Add which is second).
        let mut g2 = g;
        g2.ops = vec!["Relu".to_string(), "Add".to_string()];
        g2.inputs = vec![
            vec!["s".to_string()],
            vec!["c0".to_string(), "c1".to_string()],
        ];
        g2.outputs = vec![vec!["out".to_string()], vec!["s".to_string()]];
        // This graph has a cycle in dataflow if Relu uses s and Add produces s —
        // better test: only Add of consts that appears *after* a Mul of consts in wrong order.
        // Use two independent const ops: Mul then Relu on Mul's input const (not Mul out).
        // Simpler Issue 2 case: Add(const,const) listed after Identity(const) where Identity
        // produces mid used by Add — Identity is second, Add is first:
        let g3 = ModelGraph {
            ops: vec!["Add".to_string(), "Identity".to_string()],
            memory_plan: 0,
            layout: None,
            quantization: None,
            node_names: vec![None, None],
            shapes: vec![Some(vec![2]), Some(vec![2])],
            dtypes: vec![None, None],
            inputs: vec![
                vec!["mid".to_string(), "c1".to_string()],
                vec!["c0".to_string()],
            ],
            outputs: vec![vec!["out".to_string()], vec!["mid".to_string()]],
            layouts: vec![None, None],
            initializer_count: 2,
            initializers: vec![
                Initializer {
                    name: "c0".to_string(),
                    shape: vec![2],
                    data_type: 1,
                    data_f32: vec![1.0, 2.0],
                    ..Default::default()
                },
                Initializer {
                    name: "c1".to_string(),
                    shape: vec![2],
                    data_type: 1,
                    data_f32: vec![3.0, 4.0],
                    ..Default::default()
                },
            ],
            node_attrs: vec![NodeAttrs::default(); 2],
            graph_input_names: vec![],
            graph_output_names: vec!["out".to_string()],
            named_shapes: vec![],
            named_dtypes: vec![],
        };
        let _ = g2;
        let mut ir = DragonNpuIR::from_model_graph(&g3).expect("from");
        let n = ir.constant_fold().expect("fold");
        assert!(
            n >= 2,
            "fixed-point must fold Identity then Add despite order, got {n}"
        );
        assert!(
            ir.nodes.iter().all(|nd| matches!(nd.op, NodeOp::Constant)),
            "ops={:?}",
            ir.nodes
                .iter()
                .map(|nd| nd.op.as_op_string())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn dce_uses_explicit_graph_outputs() {
        // Issue 3: with graph_output_names set, dead branch is removed without manual IR tweak.
        let g = ModelGraph {
            ops: vec![
                "Identity".to_string(),
                "Relu".to_string(),
                "Identity".to_string(),
            ],
            memory_plan: 0,
            layout: None,
            quantization: None,
            node_names: vec![None, None, None],
            shapes: vec![Some(vec![2]), Some(vec![2]), Some(vec![2])],
            dtypes: vec![None, None, None],
            inputs: vec![
                vec!["in".to_string()],
                vec!["a".to_string()],
                vec!["dead_in".to_string()],
            ],
            outputs: vec![
                vec!["a".to_string()],
                vec!["out".to_string()],
                vec!["dead".to_string()],
            ],
            layouts: vec![None, None, None],
            initializer_count: 0,
            initializers: vec![],
            node_attrs: vec![NodeAttrs::default(); 3],
            graph_input_names: vec!["in".to_string()],
            graph_output_names: vec!["out".to_string()],
            named_shapes: vec![],
            named_dtypes: vec![],
        };
        let mut ir = DragonNpuIR::from_model_graph(&g).expect("from");
        // Do not manually set graph_outputs — compile path relies on names.
        assert_eq!(ir.graph_output_names, vec!["out".to_string()]);
        let removed = ir.dead_code_elim().expect("dce");
        assert!(
            removed >= 1,
            "expected dead Identity removed, got {removed}"
        );
        assert_eq!(ir.nodes.len(), 2);
    }

    #[test]
    fn dce_removes_unused_identity() {
        let g = ModelGraph {
            ops: vec![
                "Identity".to_string(),
                "Relu".to_string(),
                "Identity".to_string(),
            ],
            memory_plan: 0,
            layout: None,
            quantization: None,
            node_names: vec![None, None, None],
            shapes: vec![Some(vec![2]), Some(vec![2]), Some(vec![2])],
            dtypes: vec![None, None, None],
            inputs: vec![
                vec!["in".to_string()],
                vec!["a".to_string()],
                vec!["dead_in".to_string()],
            ],
            outputs: vec![
                vec!["a".to_string()],
                vec!["out".to_string()],
                vec!["dead".to_string()],
            ],
            layouts: vec![None, None, None],
            initializer_count: 0,
            initializers: vec![],
            node_attrs: vec![NodeAttrs::default(); 3],
            graph_input_names: vec![],
            graph_output_names: vec![],
            named_shapes: vec![],
            named_dtypes: vec![],
        };
        let mut ir = DragonNpuIR::from_model_graph(&g).expect("from");
        // Live output is `out` from Relu (node 1).
        ir.graph_outputs = vec![ir.name_to_tensor["out"]];
        let removed = ir.dead_code_elim().expect("dce");
        assert!(
            removed >= 1,
            "expected dead Identity removed, got {removed}"
        );
        assert_eq!(ir.nodes.len(), 2);
        assert!(ir.nodes.iter().all(|n| {
            !matches!(
                (
                    &n.op,
                    n.outputs
                        .first()
                        .and_then(|id| ir.tensors.get(id.0 as usize).map(|t| t.name.as_str()))
                ),
                (NodeOp::Identity, Some("dead"))
            )
        }));
    }

    #[test]
    fn cost_model_marks_matmul_as_npu_candidate() {
        let g = simple_matmul_relu_graph();
        let mut ir = DragonNpuIR::from_model_graph(&g).expect("from");
        if let Some(&xid) = ir.name_to_tensor.get("x") {
            ir.tensors[xid.0 as usize].shape = vec![2, 3];
        }
        ir.infer_shapes().ok();
        let cost = estimate_xdna2_cost(&ir, 4, 50);
        assert_eq!(cost.npu_candidate_ops, 1);
        assert_eq!(cost.host_ops, 1);
        assert_eq!(cost.rough_macs, 2 * 4 * 3); // M=2,N=4,K=3
        assert!(cost.notes.iter().any(|n| n.contains("heuristic")));
        assert_ne!(cost.notes, [] as [String; 0]);
    }

    #[test]
    fn gemm_attrs_survive_roundtrip() {
        let attrs = NodeAttrs {
            alpha: Some(0.5),
            beta: Some(1.5),
            trans_b: Some(1),
            ..Default::default()
        };
        let g = ModelGraph {
            ops: vec!["Gemm".to_string()],
            memory_plan: 0,
            layout: None,
            quantization: None,
            node_names: vec![Some("g0".to_string())],
            shapes: vec![None],
            dtypes: vec![None],
            inputs: vec![vec!["a".to_string(), "b".to_string()]],
            outputs: vec![vec!["c".to_string()]],
            layouts: vec![None],
            initializer_count: 0,
            initializers: vec![],
            node_attrs: vec![attrs],
            graph_input_names: vec![],
            graph_output_names: vec![],
            named_shapes: vec![],
            named_dtypes: vec![],
        };
        let ir = DragonNpuIR::from_model_graph(&g).expect("from");
        assert_eq!(ir.nodes[0].attrs.alpha, Some(0.5));
        assert_eq!(ir.nodes[0].attrs.trans_b, Some(1));
        let back = ir.to_model_graph();
        assert_eq!(back.node_attrs[0].alpha, Some(0.5));
        assert_eq!(back.node_attrs[0].beta, Some(1.5));
        assert_eq!(back.node_attrs[0].trans_b, Some(1));
    }

    #[test]
    fn node_attrs_from_onnx_gemm_attributes() {
        use tract_onnx::pb::AttributeProto;
        let attrs = vec![
            AttributeProto {
                name: "alpha".to_string(),
                f: 2.0,
                ..Default::default()
            },
            AttributeProto {
                name: "transB".to_string(),
                i: 1,
                ..Default::default()
            },
        ];
        let na = node_attrs_from_onnx_attributes(&attrs);
        assert_eq!(na.alpha, Some(2.0));
        assert_eq!(na.trans_b, Some(1));
    }
}
