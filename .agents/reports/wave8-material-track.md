# W8 Track Material — true post-patch classic 1688 hunt

**Date:** 2026-08-11
**Host:** stock in-tree amdxdna (`/dev/accel/accel0` present); `exec_cmd_allow` **param absent**
**Unlock:** none active; first `C_UNLOCK=` = **`4008e6fd` CONSUMED** (do **not** mint this slice)
**Result:** **no new true post-patch classic 1688** · invent_pass=0 · Pass=**No** · residual_matched_claim=0 · load_accepted_claim=0

---

## Goal

Agent-max attempt to land **true post-patch classic 1688** (sha16 **≠** lab oracle `3c5ffca750922d3b`) or prove hard blocks with exact errors.

Tracks: **M1** post-patch hunt · **M4** INSTR_BO_LIMIT / private `run_` · **A5** DD `instruction_cache` after fit_MKN/execute.

---

## M1 — True post-patch hunt

### What ran

| # | Command | Exit | Notes |
|---|---------|-----:|-------|
| 1 | `bash tools/capture_mladf_instr_vendor.sh /tmp/w8_material_capture_*` | **0** | Full rebuild + execute; OFM non-zero; classic prefer |
| 2 | Offline `dump_dd_txn.sh` staged key family | **0** | Unpatched Transaction map only (no NPU) |
| 3 | `bash tools/diff_vendor_instr_parity.sh` | **0** | CLASS_HINT **A** unpatched body class |
| 4 | `collect_load_accepted_candidate.sh --dry-run` on W8 1688 | **2** | basename refuse + no completing-stack evidence |
| 5 | same on multi 32208 | **2** | `multi_txn_refused=1` |
| 6 | `host_capture_scan.sh` | **0** | hits present; no residual-matched ingest |

### Capture modes exercised (same tool paths as W7)

1. **`get_transaction_bin` inputs={IFM,WTS}** (fit_MKN ≥2) → **multi 32208** diagnostic only
   - size=32208 · sha16 `3de5f68d14ff1773` · num_ops=500 · label `DIAG_multi_txn_NOT_classic_primary`
   - multi embeds classic1688=**0** · not ELF · refuse residual identity
2. **Transaction map** staged primary → classic **1688** unpatched
   - `capture_mode=txn_container_unpatched`
   - primary `instr_vendor_post_patch.bin` + `instr_vendor_txn_bin.bin`
3. **`registry get_instr_bo`** over 9 primary keys → **all fail** (see M4/A5 exact errors)
4. **Fallback** unpatched classic as post_patch filename (honest CAPTURE_MODE_UNPATCHED)

No alternate env knobs in the capture tool beyond `XDNA2_CAPTURE_ALLOW_L3_OPEN` (L3 research only; not used). No second capture binary path that peels live-patched `run_` BO.

### Sha table (W8 re-run)

| Blob | Bytes | sha16 | Role |
|------|------:|-------|------|
| W8 `instr_vendor_post_patch.bin` | **1688** | **`3c5ffca750922d3b`** | Unpatched classic primary (= oracle) |
| W8 `instr_vendor_txn_bin.bin` | 1688 | `3c5ffca750922d3b` | Same map |
| W8 multi `get_transaction_bin` | 32208 | `3de5f68d14ff1773` | Diag only — **refused** residual |
| Lab `instr_raw_txn.bin` / `lab_instr.bin` | 1688 | `3c5ffca750922d3b` | Oracle (same body) |
| Lab pure-direct `instr_post_patch.bin` | 1688 | `ccab3960f6dd789b` | Local rela patch w/ synthetic paddrs — **not** vendor live post-patch |
| Offline dump staged classic | 1688 | `3c5ffca750922d3b` | libtransaction map (unpatched) |
| Offline GM siblings (map) | **1840** | `abf03a4ac4258003` | Unpatched GM-size class — **not** classic 1688, **not** post-patch |

**New classic 1688 post-patch sha:** **none**
`same_as_oracle=True` for every classic 1688 path agent could dump.

### Collect honesty

```
1688 peel: basename_refused / completing_stack_evidence_ok=0
           residual_matched_claim=0 load_accepted_claim=0 invent_pass=0 Pass=No
32208 peel: multi_txn_refused=1 residual_matched_claim=0 load_accepted_claim=0
```

Did **not** install into `testdata/xdna2_cmd/external_known_good/`. Did **not** re-tag research/GT as load-accepted.

---

## M4 — INSTR_BO_LIMIT / private `run_` BO

### Code + header dig (DD 1.7.1 on host)

| Fact | Evidence |
|------|----------|
| `get_instr_bo(key)` | `instruction_registry::get_instr_bo` → `instruction_cache::get/put` loads **Transaction map string → host BO**, not live execute image |
| Cache API | `utils/instruction_cache.hpp`: `put(key)` / `put(key, txn_string)` / `get` / `present`; default `capacity_=0` until ctor with `hw_context`+kernel |
| Limit message | Static lib object `instruction_cache.cpp.o` string: `" is greater than the INSTR_BO_LIMIT"` |
| Align research | Capture tool constants: align `0x8000` (32 KiB), default capacity research note `0x3c00000` (60 MiB); classic 1688 **and** multi 32208 both **fit** default capacity arithmetic |
| Live throw still INSTR_BO_LIMIT | Exact error for staged + GM keys after successful execute (below) — **not** explained by “1688 > 60MiB”; points at **registry/cache instance or getInstrBufSize path**, not pure-direct content size |
| `mladfmatmulbias` public BOs | `get_input` / `get_output` / `get_const` only (+ ctrl helpers). **No** public live instr BO after execute |
| Private `run_` | Header field `xrt::run run_;` under private — used for kernel launch; **not** exposed for dump |
| Static registry entry | `OpInterface::get_instr_reg()` is **static** (`op_interface.hpp`); capture tool uses that after execute. Context owns a separate `instr_reg_` on `xrt_context` |
| Capture JSON flag | `live_instr_bo_public_api: false` in `vendor_paddrs.json` |

### Exact registry errors (W8 capture log)

```
Instruction buffer size for transaction
  mladfmatmulbias_prmptflat_mladf_2x4x4_flat_a16fw4acc16f_1_1152_256_128
  is greater than the INSTR_BO_LIMIT

(same INSTR_BO_LIMIT for _1_4096_gm / _1_16384_gm / _1_32768_gm)

Transaction not found:
  mladfmatmulbias_prmptflat_mladf_2x4x4_flat_a16w4acc16f_1_1152_256_128
  mladfmatmulbias_prmptflat_1_1152_256_128
  mladfmatmulbias_1_1152_256_128
  mladf_1_1152_256_128
  mladfmatmulbias_mladf_2x4x4_flat_a16fw4acc16f_1_1152_256_128
```

**Conclusion (M4):** Public path after execute **cannot** dump a proven live-patched instr BO. Registry mode = Transaction/cache host image only, and on this host it **throws INSTR_BO_LIMIT** for the staged family keys that *do* exist in the Transaction map (offline dump works at 1688/1840). Private `run_` remains inaccessible without DD/source fork. **True post-patch public API: closed.**

No product Pass invent; no new residual match claim from lab pure-direct `ccab3960…` rela patch.

---

## A5 — DynamicDispatch `instruction_cache` after fit_MKN / execute

### What was attempted

1. **Stock execute path (agent-legal, no sudo):**
   `capture_mladf_instr_vendor` with `load_xrt=true`, IFM+WTS for `get_transaction_bin` / fit_MKN, then registry `get_instr_bo` after OFM non-zero.
   - **OFM non-zero OK** (512 nonzero) — NPU ran under stock driver
   - **fit_MKN path OK** (multi 32208 returned; classic map preferred)
   - **Cache put via `get_instr_bo`:** INSTR_BO_LIMIT for staged+GM keys (exact strings above)

2. **Temporary L3 sandwich (hardened gate):**
   ```
   sudo -n sh -c 'echo 1 > /sys/module/amdxdna/parameters/exec_cmd_allow'
   → exit 1: sudo: a password is required

   echo | sudo -S …  (null passphrase, timeout 3s)
   → exit 124: sudo: no password was provided / a password is required
   ```
   - Path also **nonexistent** on stock (`exec_cmd_allow` param missing).
   - Agent did **not** hang. Did **not** set `XDNA2_ALLOW_EXEC_CMD`. Sticky ALLOW=0.
   - L3 open/restore **N/A** on this boot for capture (stock already ran OFM).

### Useful evidence from A5

INSTR_BO_LIMIT is a **live, repeated** registry failure **after** successful execute + fit_MKN, not a missing OFM / L3-only failure on stock. Hardened L3 password wall is separate (product lab / L2+L3 install), not the reason classic unpatched re-land works.

---

## Proven walls (exact)

| Wall | Exact error / fact | Agent-legal? |
|------|--------------------|--------------|
| No true post-patch classic 1688 | Every classic 1688 dump sha16 = oracle `3c5ffca750922d3b` | Tried; blocked |
| Registry post-patch | `… is greater than the INSTR_BO_LIMIT` (staged + GM keys) | Tried; blocked |
| Alternate registry keys | `Transaction not found: <key>` | Tried; blocked |
| Live instr BO API | `mladfmatmulbias`: private `run_`; public IO/const only; `live_instr_bo_public_api=false` | Code dig closed |
| Multi as residual | collect `multi_txn_refused=1` | Fail-closed OK |
| Unpatched as load-accepted | collect refuse (basename / no completing-stack evidence) | Fail-closed OK |
| Hardened L3 sysfs | `sudo: a password is required` (rc=1); param absent on stock | Human-only for root |
| Unlock mint | Join owns; this slice **none** | Honored |

---

## Honesty footer

```
invent_pass=0
Pass=No
not_ran_on_npu=1          # capture OFM ≠ dragon-npu decide_ran_on_npu
not_c_ready=1
load_accepted_claim=0
residual_matched_claim=0
product_known_good=0
new_true_post_patch_classic_1688_sha=none
oracle_sha16=3c5ffca750922d3b
w8_classic_primary_sha16=3c5ffca750922d3b   # same as oracle
capture_mode=txn_container_unpatched
registry_mode=INSTR_BO_LIMIT / Transaction not found
multi_txn_32208_as_residual_identity=refused
sticky_ALLOW=0
unlock=none
C_UNLOCK=4008e6fd CONSUMED   # do not reopen / do not mint this slice
public_mlir_aie_aot_rerun=not_done (frozen)
```

---

## Next named step

1. **Join track:** residual honesty pin + unlock policy only if **new residual-shaped bytes** appear (not unpatched re-land, not multi 32208, not public AOT re-run).
2. **Material still highest gap:** true completing-stack / load-accepted peel (ERT Completed + OFM allclose via product `decide_ran_on_npu`), **not** another classic-unpatched capture.
3. **Optional research-only (not Pass):** DD fork / private-field dump of live instr BO after execute, or third-party stack intercept — human/source wall, not empty reconfirm.
4. **Human-only if product needs L2+L3:** sudo for hardened module + `exec_cmd_allow` (exact: `sudo: a password is required` on this host).
5. **Freeze:** empty reconfirm of unpatched `3c5ffca750922d3b`; reopen CONSUMED unlocks; invent residual match / load_accepted / Pass.

---

## One-liner

**W8 Material:** re-ran capture-instr + map dump + registry; classic 1688 still oracle `3c5ffca750922d3b` (unpatched); registry **INSTR_BO_LIMIT** / private `run_` block true post-patch; multi 32208 refused; invent_pass=0; Pass=No; next = residual-shaped / load-accepted material or join pin, not empty re-capture.
