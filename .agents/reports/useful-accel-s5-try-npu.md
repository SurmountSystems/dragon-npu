# S5 agent-try: use the NPU this week (not product Pass)

**Date:** 2026-08-12
**Plan step:** useful-accel S5 (operator expanded from human-only to agent-try)
**Honesty:** invent_pass=0 · not product Pass · not product `ran_on_npu` · unlock untouched
**Write scope:** this report + logs under `.agents/reports/` + `/tmp/grok-1000/grok-impl-summary-useful-accel-s5.md`
**Did not edit:** `src/`, `RESIDUAL.md`, `AGENTS.md`, CLI, `silicon_contract`, last_apply, capture dir, sibling trees

IRON `PASS!` is **not** product `ran_on_npu`.
eepy CPU smoke is **not** an NPU session.

---

## Device (read-only)

| Item | Value |
|------|--------|
| Host | Lenovo 21QLCTO1WW, AMD Ryzen AI 7 PRO 350 |
| Device | `/dev/accel/accel0` `crw-rw-rw-` root:render; user `hunter` is in `render` |
| xrt-smi | XRT 2.21.75; `amdxdna` 7.1.8-zen1-3-zen; FW **1.1.2.64** |
| Name | RyzenAI-npu6 `[0000:c6:00.1]`, 8 columns |
| Driver | stock in-tree (`exec_cmd_allow` sysfs **absent**) |
| Unlock | not read as live permission; **no** `C_UNLOCK` mint; **no** last_apply; **no** `XDNA2_ALLOW_EXEC_CMD` |

---

## Bottom line

| Branch | Result | Did the NPU run? |
|--------|--------|------------------|
| **1. IRON mlir-aie ML (beyond the four public AOT peels)** | Ran three already-working IRON npu2 examples. MobileNet/ResNet not prebuilt. | **Yes (IRON).** `PASS!` + exit 0 on eltwise add, relu, softmax. That is IRON host-runtime verify, **not** product Pass. |
| **2. eepy-min VitisAI** | Session created; `quicktest.py` and a `vaip_config.json` retry both exit 0. | **Not proven.** EP name is present. Logs show default CPU provider, empty VAIP cache, **no** AIE hardware contexts after the session. Treat as CPU fallback / EP registered without offload. |
| FastFlowLM / Lemonade | Not installed (no network). | n/a |
| Capture-dir peels | Not retagged. Capture dir not touched. | n/a |

---

## 1. IRON mlir-aie

### What I looked at

- `/home/hunter/Projects/ai/mlir-aie/programming_examples/ml/mobilenet/` (README + Makefile; **no** `build/`, **no** `.xclbin`)
- `/home/hunter/Projects/ai/mlir-aie/programming_examples/ml/resnet/layers_conv2_x/` (Makefile JIT; `test.py` imports **torch**)
- `/home/hunter/Projects/ai/mlir-aie/programming_examples/ml/bottleneck/` (`bottleneck.py` is compile-only unless `--xclbin-path` + `--insts-path`; `test.py` needs torch)
- `/home/hunter/Projects/ai/mlir-aie-capture/` (present). Only the **four already-landed** public AOT peels + `completed_run_*` from Wave 5. **Not** MobileNet/ResNet.

Skipped a world rebuild / MobileNet compile. That is a full-network `@iron.jit` xclbin (`build/final_mobilenet.xclbin`) and would exceed the 10–15 minute cap.

### Env (same recipe that already ran the four peels)

```bash
CAP=/home/hunter/Projects/ai/mlir-aie-capture
VENV=$CAP/venv
export PATH="$CAP/tools:$VENV/lib/python3.14/site-packages/mlir_aie/bin:$VENV/bin:/usr/bin"
export PEANO_INSTALL_DIR="$VENV/lib/python3.14/site-packages/llvm-aie"
export PYTHONPATH="/usr/lib/python3.14/site-packages:$VENV/lib/python3.14/site-packages/mlir_aie/python"
```

Python: `/home/hunter/Projects/ai/mlir-aie-capture/venv/bin/python3`. System `pyxrt` via `/usr/lib/python3.14/site-packages`.

### Commands and exit codes

| Command | Exit | Notes |
|---------|------|--------|
| `xrt-smi examine` | 0 | FW 1.1.2.64, RyzenAI-npu6 |
| `python3 .../ml/eltwise/eltwise.py --dev npu2 --op add` | **0** | First shot ~7s (JIT). Log did not capture the `PASS!` line (stdout timing). |
| same, second shot | **0** | Printed **`PASS!`** (~0s cached) |
| `python3 .../ml/eltwise_unary/eltwise_unary.py --dev npu2 --op relu` | **0** | **`PASS!`** |
| `python3 .../ml/softmax/softmax.py --dev npu2` | **0** | **`PASS!`** (~8s first, later rerun ~0s) |
| `python3 .../ml/bottleneck/bottleneck.py --dev npu2` | **1** | `run_design_cli: no run_and_verify callback was provided — this design only supports the compile-only path (pass --xclbin-path + --insts-path) or --emit-mlir.` |
| `python3 -c 'import torch'` (capture venv) | **1** | `ModuleNotFoundError: No module named 'torch'` |
| `/usr/bin/python3 -c 'import torch'` | **1** | same |
| `python3 .../resnet/layers_conv2_x/test.py --help` | **1** | import torch fails immediately |
| `ls .../mobilenet/build` | **2** | no build dir |

These IRON ML examples are **not** the four public AOT peels (vector 300 / single_core 660 / whole_array 2592 / cascade 4512). Eltwise is bf16 add on two AIE cores; relu and softmax are other ML kernels already in-tree.

### xrt-smi during IRON

Polled `xrt-smi examine --report aie-partitions` while a cached softmax ran. Softmax printed `PASS!` and exited before the first useful poll. Every sample said **No hardware contexts running on device**. That does **not** disprove IRON PASS. The job is too short for this poll. NPU evidence here is IRON `PASS!`, not a live partition listing.

Full transcript: `.agents/reports/useful-accel-s5-iron.log`

### MobileNet later (commands only; not run)

Compile is large (full MobileNet V3 on npu2). Do **not** `pip install`. `test_mobilenet.py` imports torch, so **run** is blocked until torch exists. **Compile** of the IRON design does **not** import torch.

`makefile-common` defaults `devicename` to `npu` unless `NPU2=1`. On this Strix box you must force **npu2**.

```bash
CAP=/home/hunter/Projects/ai/mlir-aie-capture
VENV=$CAP/venv
export PATH="$CAP/tools:$VENV/lib/python3.14/site-packages/mlir_aie/bin:$VENV/bin:/usr/bin"
export PEANO_INSTALL_DIR="$VENV/lib/python3.14/site-packages/llvm-aie"
export PYTHONPATH="/usr/lib/python3.14/site-packages:$VENV/lib/python3.14/site-packages/mlir_aie/python"

cd /home/hunter/Projects/ai/mlir-aie/programming_examples/ml/mobilenet
# long compile (tens of minutes to hours). skipped this slice.
make -f Makefile build/final_mobilenet.xclbin devicename=npu2
# equivalent:
#   mkdir -p build
#   python3 -m mobilenet.aie2_mobilenet_iron \
#     --xclbin-path build/final_mobilenet.xclbin \
#     --insts-path build/insts.bin -d npu2

# run still needs torch (do not network-install):
# make -f Makefile run_py devicename=npu2
```

ResNet conv2_x later:

```bash
# same IRON env as above
cd /home/hunter/Projects/ai/mlir-aie/programming_examples/ml/resnet/layers_conv2_x
# make run_py   # blocked: test.py imports torch
```

Bottleneck later (compile only, then a torch test harness):

```bash
# same IRON env
python3 /home/hunter/Projects/ai/mlir-aie/programming_examples/ml/bottleneck/bottleneck.py \
  --dev npu2 --xclbin-path /tmp/bn.xclbin --insts-path /tmp/bn.insts.bin
```

Password is **not** required for these (user is in `render`; `/dev/accel/accel0` is world-writable on this host).

---

## 2. eepy-min VitisAI

### Docs / activate

Read `docs/RYZEN_AI_EEPY_MIN.md` and `/home/hunter/Projects/ai/ryzen-ai-install/activate-eepy.sh`.
Activate prefers `venv-eepy-min`. Smoke script is **CPU session only**.

On-disk ONNX: `/home/hunter/Projects/ai/ryzen-ai-install/venv-eepy-min/quicktest/test_model.onnx` (~23.7 MB).
Existing runner: `venv-eepy-min/quicktest/quicktest.py` (VitisAI-only providers, random `1x3x32x32`).

`vaip_config.json` exists (746984 B):

`/home/hunter/Projects/ai/ryzen-ai-install/venv-eepy-min/lib/python3.12/site-packages/voe-4.0-linux_x86_64/vaip_config.json`

Did **not** write new scripts under dragon-npu. Used activate + existing `smoke_eepy_min.sh` + existing `quicktest.py` + one in-process `python -c` to pass `config_file` (quicktest hardcodes `provider_options = [{}]`).

### Commands and exit codes

```bash
source /home/hunter/Projects/ai/ryzen-ai-install/activate-eepy.sh
bash /home/hunter/Projects/ai/ryzen-ai-install/smoke_eepy_min.sh
# exit 0  SMOKE_OK  (CPU session; docs say this is not NPU EXEC)

export RYZEN_AI_INSTALLATION_PATH=/home/hunter/Projects/ai/ryzen-ai-install/venv-eepy-min
python /home/hunter/Projects/ai/ryzen-ai-install/venv-eepy-min/quicktest/quicktest.py
# exit 0  "Test Finished"
```

ORT **1.23.3.dev20260320**.
Available providers: `['VitisAIExecutionProvider', 'CPUExecutionProvider']`.
EP so: `.../onnxruntime/capi/libonnxruntime_vitisai_ep.so` (~42 MB).

After the VitisAI-only session:

- `session.get_providers()` = `['VitisAIExecutionProvider', 'CPUExecutionProvider']`
- `get_provider_options()` = both `{}`
- run_ok shape `(1, 10)`, sample `0.0`
- ORT log: **Adding default CPU execution provider.** No VAIP compile / xclbin lines.
- `xrt-smi examine --report aie-partitions`: **No hardware contexts running on device**
- Estimated power ~0.002 W

Retry with `config_file` + `cacheDir=/tmp/grok-s5-eepy-cache` + `cacheKey=s5_quicktest`:

| Item | Value |
|------|--------|
| exit | **0** |
| session_s | 4.789 |
| run_s | 0.003 |
| sample | -0.75 |
| cache_entries | **0** |
| reported VitisAI provider_options | still `{}` (options not adopted / not reported) |
| aie-partitions after | **No hardware contexts** |
| Estimated power after | 0.023 W (idle noise, not proof of NPU) |

No VAIP / xclbin compile logs on either try. Empty cache dir means the EP did not write a compile cache.

Full transcript: `.agents/reports/useful-accel-s5-eepy.log`

### Honest eepy NPU call

**Unknown / not proven. Treat as CPU fallback.**
Reasons: session initializes in ~5 s (graph opts only), default CPU EP log line, empty `cacheDir`, no AIE contexts, provider options not visible on the session. VitisAI is **registered**. That is not the same as an NPU run.

eepy-min may be missing a working VAIP compile/runtime path (toolchain paths, plugins, or EP actually compiling). This slice did not network-install anything to chase that.

Stock in-tree `amdxdna` has no `exec_cmd_allow` gate. A missing L3 allow is **not** the explanation on this host.

---

## 3–5. Frozen

- Did **not** network-install FastFlowLM or Lemonade.
- Did **not** retag capture-dir peels as load-accepted.
- Did **not** write under `testdata/xdna2_cmd/external_known_good/`. No new residual-matched classic 1688.

---

## Still blocked

| Want | Why blocked |
|------|-------------|
| MobileNet V3 IRON e2e | No `build/final_mobilenet.xclbin`. Full compile is too long for this slice. `make run_py` also needs **torch**. |
| ResNet conv2_x IRON run | `test.py` needs **torch**. No network pip. |
| Bottleneck IRON **run** | Default CLI is compile-only. Test harness needs torch. Compile-to-xclbin was not started (can be large). |
| eepy VitisAI **on NPU** | EP listed; offload not proven. `vaip_config.json` retry did not create cache or AIE contexts. |
| Product Pass / `ran_on_npu` | Not this slice. Unlock not minted. |

---

## Honesty stamp

```
invent_pass=0
product_pass=0
ran_on_npu=0
unlock=untouched
C_UNLOCK=not_minted
last_apply=not_touched
XDNA2_ALLOW_EXEC_CMD=unset
```

Who acts next: optional long MobileNet compile (commands above) if someone wants a bigger IRON ML net; or a deeper eepy VAIP/toolchain debug. Agent will not empty-reconfirm this. IRON already used the NPU this week via eltwise / relu / softmax.
