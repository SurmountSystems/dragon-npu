# Live shot report: `C_UNLOCK=3a70fabf` timeout (no last_apply)

**Date context:** 2026-08-09
**Honesty:** `invent_pass=0` · **no** product Pass · **no** `ran_on_npu` · timeout ≠ clean APP_LOAD residual class

---

## Plain English: what happened

The operator ran one CONFIG_CU live shot under active unlock `3a70fabf` on **stock** amdxdna (`exec_cmd_allow=missing`, no L3). Offline contract was green; prep/build worked. Live EXEC was `lab_exec_once` → `timeout 30s cargo run … npu_direct_bench` with silicon-mladf + `XDNA2_LAB_PDI_BLOB`.

The cargo child did **not** finish. Coreutils `timeout` killed it after **30s** (`process rc=124`). Lab script mapped that to **exit 4** and restored cleanly (`l3_we_enabled=0`, ALLOW unset). The pinned last_apply path

`/tmp/dragon_npu_last_apply_config_cu_477280.json`

was **never created**. Post tools (`collect_post_c2_evidence`) correctly refused a missing path.

This is **not** the same class as a finished apply that returns `ert_state=Error` + journal `0x3000003` APP_LOAD. A completed residual would leave a last_apply JSON with status/detail tokens. Here the process died **mid-flight**, so there is **no** structured residual snapshot from this shot.

**Pass claims:** none. `invent_pass=0`.

---

## Code path facts (file:line)

### Outer timeout and exit 4

| Fact | Location |
|------|----------|
| Default outer timeout **30s** (`LAB_EXEC_TIMEOUT_S` / `--timeout-s`) | `scripts/lab_exec_once.sh:64` |
| Default WAIT **5000 ms** (`XDNA2_WAIT_TIMEOUT_MS`) | `scripts/lab_exec_once.sh:65` |
| Apply: separate **cargo build** first (not inside the 30s window) | `scripts/lab_exec_once.sh:828–837` |
| Apply: `timeout ${TIMEOUT_S}s env … cargo run … --example npu_direct_bench` | `scripts/lab_exec_once.sh:900–905` |
| Forwards `XDNA2_LAB_PDI_BLOB`, `XDNA2_LAB_CU_FUNC`, `XDNA2_LAB_LAST_APPLY_JSON` into child | `scripts/lab_exec_once.sh:873–878` |
| Non-zero process rc (incl. **124**) → script **exit 4** after restore | `scripts/lab_exec_once.sh:916–926` |

### Where last_apply is written (only on normal return)

| Fact | Location |
|------|----------|
| Bench calls `timed_matmul` | `examples/npu_direct_bench.rs:262` (checklist also `:52`) |
| Stamp + optional last_apply via `finalize_lab_timed_result` | `src/backends/amd_xdna2/direct.rs:4562–4596` |
| Write when `LAB_OFM` or env pin set: `write_la_default` | `direct.rs:4597` |
| With EXEC enabled, `write_la = write_la_default \|\| try_submit` | `direct.rs:4672–4677` |
| `finalize_lab_timed_result` only writes if `write_last_apply` | `silicon_contract.rs:9586–9600` |
| Actual JSON write (best-effort, never panics) | `silicon_contract.rs:10264–10339` |
| Path from `XDNA2_LAB_LAST_APPLY_JSON` if allowed | `silicon_contract.rs:10022–10040` |
| Early failures that **do** stamp if they return: `hwctx_blocked`, `config_cu_failed`, … | `direct.rs:4604–4665` etc. |
| Success-path stamp after destroy hwctx | `direct.rs:5791+` |

**Critical:** last_apply is written only when `timed_matmul_direct` reaches a `stamp(…)` / return. **SIGTERM/SIGKILL from outer `timeout` does not run stamp.** No hang-path mid-ioctl snapshot exists today.

### CONFIG_CU on the timed path (wired)

| Fact | Location |
|------|----------|
| Env `XDNA2_LAB_PDI_BLOB` | `direct.rs:235–239` (`ENV_LAB_PDI_BLOB`) |
| After CREATE_HWCTX: attach if env set | `direct.rs:4632–4669` |
| Attach = read PDI → CREATE_BO(Dev) → write → `CONFIG_HWCTX(CONFIG_CU)` | `direct.rs:3122–3172` |
| Ioctl `config_hwctx_cu` | `direct.rs:1987–2016` |
| Success token: `config_cu=1 pdi_bytes=… invent_pass=0 not_ran_on_npu=1` | `direct.rs:3070–3076` |
| Fail-closed status `config_cu_failed` + stamp | `direct.rs:4642–4665` |

**Would `config_cu=` appear only after success?**
Yes for the **success** token (`config_cu=1 …`). Fail path uses status `config_cu_failed` and a fail detail string. **Neither** appears in a file if the process is killed **during** CREATE_HWCTX, Dev BO write, or the CONFIG_CU ioctl before return.

### What can burn ~30s (or hang until outer kill)

Order on silicon apply (EXEC + LAB_OFM on):

1. **Device already open** by backend construction (before timed path).
2. **`create_hwctx_with_sizes`** (`direct.rs:4599`) — historical wedge class: D-state / open lock hangs can sit past 30s with **no** stamp.
3. **`try_lab_attach_config_cu_from_env`** — Dev BO create/write + CONFIG_HWCTX ioctl (`direct.rs:3150–3167`). Synchronous; hang = no last_apply.
4. Lab OFM / instr BO fill, DevHeap map, rela patch (many early exits **do** stamp if they return).
5. **`submit_and_wait_command`** (`direct.rs:5496`) with default **WAIT 5s** (`DEFAULT_WAIT_CMD_TIMEOUT_MS=5000`, `direct.rs:3641`). A **normal** WAIT timeout returns an error status and should still stamp last_apply. Outer **30s** kill implies hang **outside** a healthy finite WAIT (or multi-step setup longer than remaining budget), not “waited 5s and classified cleanly.”

Also: `cargo run` still has process start cost; build is already done separately, so most of the 30s is likely **in the binary**, not compile.

---

## Why last_apply was missing

1. Prep pinned path (example pattern):
   `XDNA2_LAB_LAST_APPLY_JSON=/tmp/dragon_npu_last_apply_config_cu_<pid>.json`
   (`tools/hitl_config_cu_once.sh:96–102`, stock sandwich `print_sandwich` around `:168–173`).
2. That path is only a **pin** (export). The file is created when Rust `write_last_apply_to_path` runs after a completed timed result.
3. Outer `timeout` killed the process at 30s **before** any stamp return.
4. `/tmp` check this investigation: no `dragon_npu_last_apply_*.json` found for the shot pin; related PDI artifacts like `/tmp/dragon_npu_research_pdi_*.bin` can exist without a last_apply.
   collect refuses missing explicit path (`tools/collect_post_c2_evidence.sh:72–77`).

---

## Unlock: mark `C_UNLOCK=3a70fabf` CONSUMED?

**Yes.**

- One-shot was for a real live CONFIG_CU sandwich / stock EXEC with PDI + userspace ALLOW.
- Lab opened the apply path (BUILD ok, EXEC attempted under timeout, RESTORE ok).
- Unlock ≠ Pass. Consuming unlock means **do not** re-open under the same stamp; a further live C needs a **new** host-minted unlock plus a named offline reason.
- Prior historical consumable `b17b0119` stays historical only.

---

## Recommended next (no invent Pass)

### Human (evidence)

1. **Journal / diagnose** for hang class after the shot: CREATE_HWCTX stall, CONFIG_CU, map_host / DevHeap, D-state waiters, refcnt. Stock wedge risk is real without L2 null-guards.
2. **Optional retry with longer outer budget** (reasonable):
   `LAB_EXEC_TIMEOUT_S=120` or `--timeout-s 120`
   Keep `XDNA2_WAIT_TIMEOUT_MS=5000` (or 10–15s) unless WAIT itself is the suspect. Outer 30s is tight for first stock CONFIG_CU + full silicon-mladf fill. Longer outer timeout is **retry hygiene**, not Pass.
3. Keep `XDNA2_LAB_BO_DIAG=1` and the same last_apply pin path; re-run collect only if the JSON **exists**.
4. If driver looks wedged (refcnt / D-state open hang): recovery / reload before another EXEC (operator-owned).

### Offline agent deltas (optional product quality)

1. **Progress last_apply / partial stamp** on long stages (hwctx ok, config_cu ok/fail, pre-EXEC) so a kill still leaves evidence. Today: hang = zero JSON.
2. Document that outer timeout kill ⇒ collect path refuse is **expected**, not collect bug.
3. Do **not** invent `config_cu=1` or APP_LOAD class from journal alone without a completed snapshot.
4. Soft gates remain: `not_c_ready=1 not_ran_on_npu=1 invent_pass=0 product_pass_requires_live_decide=1`.

### Explicitly not next

- No product Pass / no `ran_on_npu` claim from rc=124.
- No empty reconfirm implement mill.
- No second use of unlock `3a70fabf`.

---

## Status tokens (this shot)

| Token | Value |
|-------|--------|
| Unlock | **CONSUMED** after real live attempt (`3a70fabf`) |
| last_apply | **missing** (timeout before write) |
| residual class | **unknown / incomplete** (not locked APP_LOAD from this shot) |
| product Pass | **no** |
| `ran_on_npu` | **no** |
| `invent_pass` | **0** |
