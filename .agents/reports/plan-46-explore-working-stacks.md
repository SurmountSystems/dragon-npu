# Working stacks on this Ryzen AI / XDNA2 host (independent of product Pass)

**Date:** 2026-08-12
**Kind:** Read-only host + docs inventory. No product edits. No unlock. No invent Pass.
**Honesty:** invent_pass=0 · product Pass=**No** · product `ran_on_npu`=**false** · IRON/eepy ≠ dragon-npu decide.

This answers: what can already do **useful NPU compute or inference** on this machine **without** waiting for dragon-npu matmul Pass.

---

## One-screen table

| Stack | Present on host? | Proven NPU run? | Useful inference? | Honesty (≠ product `ran_on_npu`) |
|-------|------------------|-----------------|-------------------|----------------------------------|
| **IRON / mlir-aie npu2** (XRT + `pyxrt`) | **Yes** — `/home/hunter/Projects/ai/mlir-aie` + `/home/hunter/Projects/ai/mlir-aie-capture` | **Yes** (2026-08-11, FW **1.1.2.64**) — vector add + 3 public matmuls, IRON `PASS!` | **NPU compute yes** (tiny vector + 64³/128³ i16 matmul). **App inference not yet** (MobileNet/ResNet trees exist, not run in W5). | IRON `PASS!` ≠ product Pass ≠ residual match to lab classic 1688. Peels stay RESEARCH ONLY. |
| **eepy-min / ORT VitisAI EP** | **Yes** — `~/Projects/ai/ryzen-ai-install/` (`venv-eepy-min/` + full `venv/`) | **Providers listed only.** Smoke is **CPU session** (`SMOKE_OK` design). **No logged VitisAI NPU session** in this repo. | **Not proven.** Closest vendor path to real ONNX/CNN/LLM if EP session works on stock driver. | Smoke ≠ NPU EXEC. EP install ≠ Pass. Do not claim `ran_on_npu`. |
| **XRT / `xrt-smi` / `/dev/accel`** | **Yes** — `/dev/accel/accel0`, `/usr/bin/xrt-smi`, `/usr/bin/xclbinutil`. `/opt/xilinx` **no**. | Examine **OK** (RyzenAI-npu6, FW 1.1.2.64). Validate **FAILED** (no `/opt/xilinx/xrt/test/` archives). | Health/device only. No sample gemm/throughput job. | Validate fail ≠ product residual. `xrt-smi` ≠ Pass. |
| **Public mlir-aie AOT bins** under `testdata/xdna2_cmd/external_known_good/` | **Yes** (300/660/2592/4512 + siblings) | Those **same** bytes ran via IRON (above). In-tree files are **peels**, not a product EXEC path. | Research material / re-run via IRON examples. Product lab will not Pass on them. | RESEARCH ONLY. Helpers refuse load-accepted re-tag. |
| **dragon-npu CLI** `compile` / `run` | **Yes** (this repo) | **No NPU.** `run` is host/sim. `npu_exec` is library-only, fail-closed / 1×1×1 probe, never model graph. | Host graph / sim only. | CLI always `ran_on_npu=false`. `host_ref` is CPU. |
| **`xrt-bridge` feature** | Code stub only (`Cargo.toml` `xrt-bridge = []`, off by default) | **No.** Refuses sessions. No `libxrt` link. | None. | Research stub ≠ runtime. |
| **FastFlowLM** | **No** (`~/Projects/ai/FastFlowLM` missing) | No | No | W5 Path A absent. Agent must not network-install. |
| **Lemonade** | **No** | No | No | Same as FastFlowLM. |
| **Standalone `amd/IRON` clone** | **No** (`~/Projects/ai/IRON` missing) | N/A | IRON **API lives inside mlir-aie** (`python/iron/`, `import aie.iron`). | Clone-of-amd/IRON is not required. |
| **RyzenAI-SW examples + CVML** | **Yes** — `~/Projects/ai/RyzenAI-SW` (CNN, ASR, LLM, linux CVML `.so`) | **Not proven** in W5/W6 reports. | Possible next (ResNet / hello_world / CVML) **if** wired to eepy VitisAI EP. | Examples on disk ≠ NPU run. Phoenix VOE xclbins are not residual match. |
| **Intel VPU / Hexagon** | Product stubs only | No | No | Honest refuse. |

---

## Verified path checks (yes / no)

| Path | Exists? |
|------|---------|
| `~/Projects/ai/ryzen-ai-install/` | **Yes** (full `venv/`, `venv-eepy-min/`, `smoke_eepy_min.sh`, share tarball) |
| `/opt/xilinx` | **No** |
| `/opt/amd/ryzen-ai` | **No** |
| FastFlowLM / lemonade trees | **No** |
| IRON/mlir-aie from W5 | **Yes** — `/home/hunter/Projects/ai/mlir-aie` and `/home/hunter/Projects/ai/mlir-aie-capture` |
| `~/Projects/ai/IRON` (amd/IRON sibling) | **No** |
| `/dev/accel/accel0` | **Yes** |
| System `pyxrt` | **Yes** — `/usr/lib/python3.14/site-packages/pyxrt.cpython-314-x86_64-linux-gnu.so` |
| `exec_cmd_allow` module param | **No** (stock `amdxdna` params are `aie2_max_col`, `force_cmdlist`, `force_iova` only) |

---

## 1. eepy-min / VitisAI EP

- Docs: `docs/RYZEN_AI_EEPY_MIN.md`. Tree: `~/Projects/ai/ryzen-ai-install/`.
- Min venv is installed (`onnxruntime`, `onnxruntime_vitisai`, `voe`, `flexmlrt`, `quicktest/test_model.onnx`).
- Smoke (`smoke_eepy_min.sh`) asserts **VitisAI + CPU providers** and runs a **CPU** `InferenceSession` only. Comment in script: no EXEC_CMD / no live NPU job required.
- Doc warning: hardened L3 `exec_cmd_allow=0` **may** `-EPERM` EP inference. **This host is stock** (no `exec_cmd_allow` knob). IRON already submitted work via XRT without a dragon-npu L3 sandwich.
- **Still true:** nobody in W5/W6 recorded a VitisAI EP **NPU** session. Do not invent that eepy already accelerates models.

## 2. W5 / W6 IRON Path B (what actually ran)

- IRON here means **mlir-aie Python `aie.iron` + system `pyxrt` + XRT 2.21.75**, not a separate amd/IRON checkout.
- Capture venv alone failed (`no NPU runtime`). Fix: put `/usr/lib/python3.14/site-packages` on `PYTHONPATH`.
- Proven on FW **1.1.2.64**, device RyzenAI-npu6 `0000:c6:00.1`, no dragon-npu ALLOW/L3:

| Example | Result | Size / sha16 | Evidence |
|---------|--------|--------------|----------|
| `vector_scalar_add` AOT + JIT | `PASS!` | 300 B `aaeb5b68cc298a45` | `.agents/reports/path-b-vector-scalar-add-npu2-pass-20260811.log`; `mlir-aie-capture/completed_run_vector_scalar_add_20260811_150247/` |
| `single_core` 64³ i16→i32 | `PASS!` ~3.2 GFLOPS | 660 B `de8671245bb72436` | `wave5-path-b-matmul-attempt.md` |
| `whole_array` 128³ 4-col | `PASS!` ~14 GFLOPS | 2592 B `7ee4b267ce31cd70` | same |
| `cascade` 128³ 4-col | `PASS!` | 4512 B `18cb653c05a9d4c8` | same |

- Still runnable in principle: same env, same trees, same firmware class. Re-run is **not** product progress and **not** a new Pass claim.
- W6 froze public AOT re-run as residual work. Re-run is only for **using** the NPU, not for residual identity.

## 3. mlir-aie peels vs execute

- In-tree `testdata/xdna2_cmd/external_known_good/` + `docs/CAPTURE_MATERIAL_MLIR_AIE.md`: **research peels** for classify/ingest.
- They **can** execute **through IRON/XRT** (already did). They **cannot** be treated as product load-accepted or lab classic 1688.
- Larger IRON ML examples exist and were **not** W5-run: `programming_examples/ml/mobilenet` (e2e MobileNet V3 on npu2), `resnet/`, `conv2d/`, `magika/`.

## 4. XRT packaging vs product bridge

- Distro XRT is present (`xrt-smi`, `xclbinutil`). Official `/opt/xilinx` tree is **absent**, so `xrt-smi validate` has no platform tests (`wave5-path-c-agent-attempt.md`).
- `docs/XDNA2_PACKAGING.md`: product user should not need sudo per inference; L3 is lab-only on **hardened** module.
- `docs/SECONDARY_BACKENDS.md` + `src/backends/xrt_bridge.rs`: feature `xrt-bridge` is a **refuse stub**. Production path is `amd-xdna2` direct, still not Pass.

## 5. Product CLI honesty

- `src/cli.rs` / `src/bin/dragon_npu.rs`: `compile` is host compile. `run` sets `execution=host_or_simulation_run` and **always** `ran_on_npu=false`.
- Even with `amd-xdna2` and hardware detected, `AmdXdna2Backend::new()` keeps the **Simulation** arm for `Backend::run`.
- Library `XDNA2_REAL_SUBMIT_MODE=host_ref` is CPU allowlist. `npu_exec` is a gated 1×1×1 probe that still returns `RealSubmitNotReady`. CLI never enables EXEC_CMD.

## 6. Host inventory / Path A

- `tools/host_npu_stack_inventory.sh` (W4) and `tools/load_accepted_path_status.sh` (W5): accel present; FastFlowLM/Lemonade **absent**; `/opt/xilinx` **absent**; ryzen-ai-install + RyzenAI-SW **present**.
- W4 `exec_cmd_allow=missing` matches current sysfs (stock module).

---

## Fastest path to **useful** NPU work this week

**Already working today (use this first):** IRON public examples.

```text
export VENV=/home/hunter/Projects/ai/mlir-aie-capture/venv
export CAP=/home/hunter/Projects/ai/mlir-aie-capture
export MLIR_AIE=/home/hunter/Projects/ai/mlir-aie
export PATH=$CAP/tools:$VENV/lib/python3.14/site-packages/mlir_aie/bin:$VENV/bin:/usr/bin
export PEANO_INSTALL_DIR=$VENV/lib/python3.14/site-packages/llvm-aie
export PYTHONPATH=/usr/lib/python3.14/site-packages:$VENV/lib/python3.14/site-packages/mlir_aie/python
cd $MLIR_AIE/programming_examples/basic/vector_scalar_add
python3 vector_scalar_add.py --dev npu2
```

That is real NPU compute (verify + timing). It is **not** LLM chat and **not** product Pass.

**Closest step-up to “inference” (not proven yet):** same env, `programming_examples/ml/mobilenet` (`make run_py` / `test_mobilenet.py`) or `resnet/`. Same XRT path that already passed vector/matmul.

**Closest step-up to vendor ONNX apps (not proven yet):** `source ~/Projects/ai/ryzen-ai-install/activate-eepy.sh` then a **VitisAI** `InferenceSession` on `venv-eepy-min/quicktest/test_model.onnx` or a RyzenAI-SW hello_world/ResNet. Stock driver has no L3 knob, so the old “EP blocked by exec_cmd_allow=0” warning is **not** the current host fact. Still needs a measured NPU session.

**Not this week without operator install:** FastFlowLM / Lemonade (LLM serving). Product dragon-npu Pass.

---

## What still needs the operator

- **Sudo L3 / `exec_cmd_allow`:** not required for IRON on this **stock** module. Only needed if a **hardened** L3 build is loaded later, or for dragon-npu dangerous EXEC lab.
- **AMD EULA wheels:** already on disk under `ryzen-ai-install`. No new 11 GiB dump required for eepy-min.
- **Missing trees:** FastFlowLM, Lemonade, `/opt/xilinx` validate archives, standalone amd/IRON. Operator clone/install only; agents do not download.
- **Unproven next runs:** eepy VitisAI NPU session; IRON MobileNet/ResNet; RyzenAI-SW CNN/CVML. Operator or a named implement slice, not assumed green.
- **Product Pass:** still needs residual-matched / load-accepted material + new unlock. IRON/eepy do not close that.

---

## Bullets (evidence, 16)

1. This machine already ran **four** public mlir-aie npu2 programs to IRON `PASS!` on FW 1.1.2.64.
2. Those runs used **XRT + system pyxrt**, not dragon-npu `decide_ran_on_npu`.
3. Product Pass remains **No**. invent_pass stays 0.
4. Capture-dir 300/660/2592/4512 peels are the **same** research identities; helpers refuse load-accepted re-tag.
5. IRON is **inside mlir-aie**; `~/Projects/ai/IRON` is absent and not needed.
6. mlir-aie-capture venv + Peano + aiebu tools are present; pyxrt must come from **system** site-packages.
7. eepy-min is **installed** and can list VitisAI EP; smoke is **CPU-only**.
8. Full `ryzen-ai-install/venv/` is also present (flexml, aiecompiler, xclbinutil in that venv).
9. `/dev/accel/accel0` is present; user/render path was enough for IRON.
10. Stock `amdxdna` has **no** `exec_cmd_allow` sysfs knob. L3 is not what blocked IRON.
11. `xrt-smi examine` works; `validate` cannot (no `/opt/xilinx/xrt/test/`).
12. FastFlowLM and Lemonade are **absent**. No LLM NPU stack is installed.
13. dragon-npu `compile`/`run` is **host/sim**. `npu_exec` is not a useful inference API.
14. `xrt-bridge` does not talk to XRT. Do not enable it for inference.
15. RyzenAI-SW + linux CVML libs are on disk but **not** proven NPU-run here.
16. Fastest useful work this week: **re-use IRON**; then try MobileNet or eepy VitisAI. Do not wait on product matmul Pass for that.

---

## Footer

```
invent_pass=0
not_c_ready=1
not_ran_on_npu=1
Pass=No
iron_completed_on_this_fw=yes   # public 300/660/2592/4512 only
eepy_npu_session_proven=no
unlock=none
```

Related: `docs/RYZEN_AI_EEPY_MIN.md`, `docs/CAPTURE_MATERIAL_MLIR_AIE.md`, `docs/LOAD_ACCEPTED_CAPTURE.md`, `.agents/reports/wave5-join.md`, `.agents/reports/wave5-path-b-matmul-attempt.md`, `.agents/reports/wave4-host-inventory.md`.
