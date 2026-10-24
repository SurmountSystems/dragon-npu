# W7-1 — capture-instr attempt (classic 1688 prefer)

**Date:** 2026-08-11
**Host:** kernel `7.1.5-zen1-2-zen`, amdxdna srcversion `77910A99EDBD0B6C78C8053`
**Device:** `/dev/accel/accel0` present (render group)
**Unlock:** none active; first `C_UNLOCK=` = **`4008e6fd` CONSUMED** (do not mint this slice)
**Result class:** **succeeded** (classic 1688 primary re-landed) — **not** Pass / **not** load_accepted / **not** residual_matched product claim

---

## 1. Recipes discovered

| Path | Role |
|------|------|
| `docs/HITL_BC.md` | Milestone B offline; optional L3 capture-instr sandwich |
| `tools/hitl_b_gt_capture.sh` | Offline GT oracle (no sudo); optional `--with-vendor` |
| `tools/silicon_lab_operator.sh capture-instr` | Preferred one-liner → `capture_mladf_instr_vendor.sh --install` |
| `tools/capture_mladf_instr_vendor.sh` + `.cpp` | DD NPU execute + instr dump; **classic 1688 Transaction map preferred** over multi 32208 |
| `tools/diff_vendor_instr_parity.sh` | Offline body class; recovers classic sibling if post_patch is multi |
| `tools/collect_load_accepted_candidate.sh` | Fail-closed dry-run / ingest (refuse multi + oracle basenames) |
| `tools/operator_app_load_kit.sh` | CONFIG_CU / capture kit (not used for this instr path) |

**Classic prefer contract (in capture tool):** multi-txn `get_transaction_bin` → `instr_vendor_get_transaction_bin.bin` diagnostic only; Transaction map classic 1688 → primary `instr_vendor_post_patch.bin` + `instr_vendor_txn_bin.bin`.

---

## 2. Preflight

| Check | Result |
|-------|--------|
| `exec_cmd_allow` sysfs | **Param absent** (`/sys/module/amdxdna/parameters/exec_cmd_allow` missing). Stock **in-tree** amdxdna (`intree: Y`), not L2+L3 hardened OOT. Params: `force_iova`, `aie2_max_col`, `force_cmdlist` only. |
| Device | `/dev/accel/accel0` OK; module loaded |
| Staged lab assets | `lab_instr.elf` 32264, `lab_instr.bin` **1688**, ctrl/WTS/golden present |
| Prior gt_capture | Had multi **32208** as `instr_vendor_post_patch.bin` (wrong primary) + classic **1688** sibling `instr_vendor_txn_bin.bin` |
| DD_ROOT / libaiebu | Present under ryzen-ai venv + `/usr/lib/libaiebu.a` |
| `sudo -n true` | **Fail:** `sudo: a password is required` (rc=1) |
| `sudo -S` null stdin | Fingerprint timeout + `sudo: no password was provided` / `sudo: a password is required` (timeout rc=124) |

**Sudo needed?** Only if installing hardened L3 gate or writing root-only sysfs. On **stock** driver, capture-instr ran **without** sudo (no L3 param to open).

---

## 3. Commands run + exit codes

| # | Command | Exit | Notes |
|---|---------|------|-------|
| 1 | `bash tools/silicon_lab_operator.sh status` | **0** | Assets OK; `exec_cmd_allow=missing` |
| 2 | `bash tools/hitl_b_gt_capture.sh` | **0** | Offline GT only; vendor step skipped |
| 3 | `sudo -n true` | **1** | Exact: `sudo: a password is required` |
| 4 | `sudo -n sh -c 'echo 1 > …/exec_cmd_allow'` | **1** | Same password wall; path also nonexistent on stock |
| 5 | `bash tools/silicon_lab_operator.sh capture-instr` | **5 then fixed** | Capture binary **succeeded** (classic 1688 primary). Wrapper falsely elevated to 5 because `missing` ≠ closed. Fixed (see §6). |
| 6 | Inner `capture_mladf_instr_vendor.sh --install` | **0** | OFM non-zero; classic primary written |
| 7 | `bash tools/diff_vendor_instr_parity.sh` | **0** | CLASS_HINT A body match; unpatched ≠ C-ready |
| 8 | `bash tools/collect_load_accepted_candidate.sh --dry-run …/instr_vendor_post_patch.bin` | **2** | Fail-closed: `basename_refused=1` local oracle; `load_accepted_claim=0` `residual_matched_claim=0` |

### Capture tool evidence (classic prefer)

```
OFM non-zero OK (512 nonzero bytes)
dumped instr_vendor_get_transaction_bin.bin size=32208 … label=DIAG_multi_txn_NOT_classic_primary
ALWAYS dumped instr_vendor_txn_bin.bin size=1688 … label=UNPATCHED_txn_container_classic
FALLBACK → instr_vendor_post_patch.bin from txn_container size=1688 … capture_mode=txn_container_unpatched classic=1
registry: INSTR_BO_LIMIT / Transaction not found (no live run_ BO; expected research closed)
```

### L3 open / restore proof

- **Stock driver:** no `exec_cmd_allow` param → nothing to open, nothing sticky.
- Agent did **not** leave a hardened L3 gate open (gate does not exist on this boot).
- `XDNA2_ALLOW_EXEC_CMD` unset throughout.
- False wrapper exit 5 on first capture-instr run documented above; tool fix treats `missing` as closed.

---

## 4. New peels (post W7-1 capture)

| File | Bytes | sha16 | Role |
|------|------:|------|------|
| `gt_capture/instr_vendor_post_patch.bin` | **1688** | `3c5ffca750922d3b` | **Primary classic** UNPATCHED (`txn_container_unpatched`) |
| `gt_capture/instr_vendor_txn_bin.bin` | **1688** | `3c5ffca750922d3b` | Same classic map |
| `gt_capture/instr_vendor_get_transaction_bin.bin` | 32208 | `3de5f68d14ff1773` | Multi-txn **diag only** — refuse residual identity |
| `gt_capture/ofm_golden.bin` | 512 | `8e6b548203bfc086` | NPU OFM non-zero (vendor stack ran) |
| `gt_capture/vendor_paddrs.json` | — | — | `capture_mode=txn_container_unpatched`, `from_registry=false` |

**Identity honesty:**

- sha16 `3c5ffca750922d3b` **matches** `silicon_lab/lab_instr.bin` and `instr_raw_txn.bin` (same staged classic dump body).
- **≠** product load-accepted completing-stack material (no ERT Completed + OFM allclose via `decide_ran_on_npu` for dragon-npu apply path).
- Multi 32208 **refused** as residual match (diag path only).
- Collect dry-run: **refuse** oracle basename + no completing-stack evidence flags.

---

## 5. collect_load_accepted dry-run (fail-closed)

```
class=refused_basename
reason=local_oracle_or_wrong_platform:instr_vendor_post_patch.bin
blob_len=1688 sha16=3c5ffca750922d3b
load_accepted_claim=0 residual_matched_claim=0 invent_pass=0 Pass=No
collect_rc=2
```

Did **not** re-tag research/GT as load_accepted. Did **not** ingest into `external_known_good/`.

---

## 6. Tool fix (stock L3 false open)

**Bug:** `tools/silicon_lab_operator.sh` treated `exec_cmd_allow=missing` like still-open → exit **5** after successful capture.

**Fix:** case `missing` → POST_OK (stock driver; no L3 gate). Applied to capture-instr, capture-golden, and apply post-checks. Aligns with `capture_mladf_instr_vendor.sh` (unreadable ALLOW keeps base rc).

`bash -n tools/silicon_lab_operator.sh` OK.

---

## 7. Human required?

**Not proven human-only wall for capture-instr itself** on this host:

- Stock amdxdna allowed vendor DD execute without sudo.
- Classic 1688 primary re-landed by agent.

**Human still required for:**

1. **Hardened L2+L3 module** install (if product path needs `exec_cmd_allow` gate) — sudo password / operator install.
2. **True load-accepted / residual-matched completing-stack** material with live ERT Completed + OFM allclose evidence (not GT/unpatched classic).
3. **New unlock mint + live L1** (W7-5 owns live; this slice **no mint**).

Sudo password wall is real for root sysfs / hardened install (`sudo: a password is required`) but was **not** the blocker for classic 1688 capture on stock.

---

## 8. Honesty footer

```
invent_pass=0
Pass=No
not_ran_on_npu=1          # capture tool OFM ≠ dragon-npu decide_ran_on_npu
not_c_ready=1
load_accepted_claim=0
residual_matched_claim=0
product_known_good=0
unlock=none
C_UNLOCK=4008e6fd CONSUMED  # do not reopen / do not mint this slice
sticky_ALLOW=0
public_mlir_aie_aot_rerun=not_done (frozen; not residual progress)
multi_txn_32208_as_residual_identity=refused
classic_1688_primary=landed (unpatched txn_container; body CLASS A offline)
```

---

## 9. Outcome one-liner

**Succeeded:** agent capture-instr re-landed **classic 1688** as primary (sha16 `3c5ffca750922d3b`); multi 32208 diag-only; collect refuse load_accepted; unlock none; invent_pass=0.
**No new residual-matched / load-accepted product material** — unpatched classic body class only.
