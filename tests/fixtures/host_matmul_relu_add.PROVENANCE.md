# host_matmul_relu_add.json

Tiny **host-only** linear demo for S2: MatMul (2x2 identity) + Relu + Add (bias `[0.5, 1.5]`).

Hand-written JSON. Not an ONNX export. Not NPU microcode. Not residual-matched material.

Golden: input `[-1.0, 2.0]` → `[0.5, 3.5]` via `evaluate_host_graph`.

`ran_on_npu` stays false. This is the CPU fallback while the external runner is wired.

See unit test `fixture_host_matmul_relu_add_known_output`.
