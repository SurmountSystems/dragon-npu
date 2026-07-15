# Operator checklist: capture a known-good XDNA2 instruction blob

**Audience:** human lab operator only.  
**Not** an auto-enable EXEC script. **Not** a CI step.  
**Safety:** default dragon-npu stays EXEC-closed. Prefer offline capture (Pathway A/B). Pathway C uses **XRT** EXEC (kernel `exec_cmd_allow`), not dragon-npu L1 — still restore gates.

Full research context: [`docs/XDNA2_TXN_RESEARCH.md`](../docs/XDNA2_TXN_RESEARCH.md) · Lab EXEC: [`docs/XDNA2_EXEC_CMD_LAB.md`](../docs/XDNA2_EXEC_CMD_LAB.md) · Policy: [`docs/XDNA2_NEVER_AGAIN.md`](../docs/XDNA2_NEVER_AGAIN.md)

---

## Preflight (always)

```bash
export PATH=/usr/bin:$PATH
unset XDNA2_ALLOW_EXEC_CMD
bash scripts/inventory_txn_sources.sh | tee /tmp/txn-inventory.txt
bash scripts/diagnose_amdxdna.sh
# Optional lab host:
# bash scripts/verify_amdxdna_hardened_host.sh
cat /sys/module/amdxdna/parameters/exec_cmd_allow 2>/dev/null  # expect N / 0
```

Do **not** open `/dev/accel` in a loop if diagnose reports WEDGED_*.

---

## Pathway A — offline (preferred; no EXEC)

0. Optional smoke inventory (always exit 0; not Pass):

```bash
bash scripts/extract_instruction_offline.sh
# Optional research-only VE2 stage (NOT_AIE2_KNOWN_GOOD — not Strix Pass):
# bash scripts/extract_instruction_offline.sh --stage-ve2-nop
```

**VE2 `shim_ve2/Runner/latency/nop.elf` is research-only** — do not treat as npu4/17f0
known-good instruction BO content.

1. Confirm tools: `xclbinutil`, XRT headers, optional partner ELF tree.
2. Obtain a **license-cleared** ELF or xclbin for **this** platform (Strix `1022:17f0` / npu4 preferred).  
   - Partner: `local_shim_test_data/npu*/{nop,vadd,gemm}/*.elf`  
   - **Missing today** on many lab hosts — inventory will say so.
3. Inspect only (example; verify flags with `--help` first):

```bash
xclbinutil --help | less
# Example pattern (confirm section names on your tool version):
# xclbinutil --info --input /path/to.xclbin | tee /tmp/xclbin-info.txt
# xclbinutil --dump-section <SECTION>:RAW:/tmp/section.bin --input /path/to.xclbin
```

4. If you have an open C++ sketch linking `libxrt_coreutil`, materialize ctrltext offline into a file (operator-implemented; not shipped as auto EXEC).
5. Record provenance:

```bash
sha256sum /tmp/known_good_instr.bin | tee /tmp/known_good_instr.sha256
# Write a side-car: tool version, source ELF/xclbin hash, date, license note
```

6. Stage under **gitignored** path (do not force-add binaries):

```bash
mkdir -p testdata/xdna2_cmd
cp /tmp/known_good_instr.bin testdata/xdna2_cmd/lab_instr.bin
# README stays in git; binary stays ignored
```

### License rules

| Source | Commit raw binary to dragon-npu? |
|--------|----------------------------------|
| Apache XRT tools / self-generated envelope | Synthetic envelope OK; real txn only with provenance |
| `xrt-plugin-amdxdna` proprietary | **Do not reverse-engineer**; dumps may be lab-only |
| RyzenAI Windows xclbin overlays | Prefer **not** without redistribution grant |
| Firmware `amdnpu/*.sbin` | **Never** as instruction fixture |

---

## Pathway B — feed dragon-npu **without** claiming Pass

```bash
unset XDNA2_ALLOW_EXEC_CMD
export XDNA2_INSTRUCTION_BLOB="$PWD/testdata/xdna2_cmd/lab_instr.bin"

# 1) Validate *your* lab file (do not rely only on cargo unit tests — those use temp paths)
test -s "$XDNA2_INSTRUCTION_BLOB"
wc -c < "$XDNA2_INSTRUCTION_BLOB"
sha256sum "$XDNA2_INSTRUCTION_BLOB"
# Optional hex peek (first 32 bytes):
xxd -l 32 "$XDNA2_INSTRUCTION_BLOB" || od -An -tx1 -N 32 "$XDNA2_INSTRUCTION_BLOB"

# 2) Regression only (temp fixtures / unset-env skips — NOT proof of lab file load):
cargo test --features amd-xdna2 --lib load_instruction_blob -- --nocapture

# Cap: load_instruction_blob_from_path refuses files > MAX_INSTRUCTION_BLOB_BYTES (64 MiB)
# Prefer one-shot wrapper (dry-run first):
#   bash scripts/lab_exec_once.sh --blob /path/to.bin
#   bash scripts/lab_exec_once.sh --apply --i-accept-wedge-risk --blob /path/to.bin
# Apply exports XDNA2_LAB_OFM=1; npu_direct_bench exits 0 only if ran_on_npu.
```

**Never stage under `tests/fixtures/`** — use `testdata/xdna2_cmd/` (gitignored binaries).

Statuses like `ok_submit_wait_with_inst_blob` ≠ product Pass. With `XDNA2_LAB_OFM=1`, timed path can flip `ran_on_npu` only after OFM readback + `decide_ran_on_npu` (needs known-good txn writing matching OFM).

---

## Pathway C — optional live XRT capture (restore gates)

Only on an intentional lab window:

1. Temporary kernel allow (example):

```bash
# Operator decision + sudo:
# sudo modprobe amdxdna exec_cmd_allow=1
# or write sysfs if your policy allows
cat /sys/module/amdxdna/parameters/exec_cmd_allow   # expect Y/1 during window
```

2. Prefer rebuilding plugin/shim with BO dump enabled (`m_dump_arg_bos` is false in stock).  
3. Run **XRT** shim_test / xrt_test with known ELF — **not** dragon-npu L1.  
4. Copy instruction BO dump to `/tmp/known_good_instr.bin`; `sha256sum`.  
5. **Restore immediately (mandatory exit criterion before closing the lab window):**

```bash
# sudo modprobe amdxdna exec_cmd_allow=0
unset XDNA2_ALLOW_EXEC_CMD
echo "exec_cmd_allow=$(cat /sys/module/amdxdna/parameters/exec_cmd_allow 2>/dev/null || echo missing)"
# Must print N or 0:
test "$(cat /sys/module/amdxdna/parameters/exec_cmd_allow 2>/dev/null)" = "N" \
  -o "$(cat /sys/module/amdxdna/parameters/exec_cmd_allow 2>/dev/null)" = "0"
# Must print nothing (ALLOW unset):
env | grep -E '^XDNA2_ALLOW_EXEC_CMD=' || echo "XDNA2_ALLOW_EXEC_CMD unset OK"
bash scripts/diagnose_amdxdna.sh
```

6. Then Pathway B for dragon-npu load-only tests.

---

## After a blob exists: single-shot dragon-npu EXEC (still not Pass without verify)

Follow [`docs/XDNA2_EXEC_CMD_LAB.md`](../docs/XDNA2_EXEC_CMD_LAB.md) triple gate:

1. diagnose clean  
2. build with `amd-xdna2,xdna2-dangerous-exec-cmd`  
3. temporary `exec_cmd_allow=1`  
4. `XDNA2_ALLOW_EXEC_CMD=1` + **one** `timeout` process  
5. host-filled Cmd BO required; refuse empty scaffold  
6. restore allow=0; unset ALLOW  

**Product `ran_on_npu=true` still requires** known-good txn + LAB_OFM OFM vs host golden after wait. Envelope/inst-blob submit alone is insufficient.

---

## Explicit refusals

- Do not loop empty/scaffold EXEC_CMD.  
- Do not leave `exec_cmd_allow=1` or `XDNA2_ALLOW_EXEC_CMD=1` after the session.  
- Do not commit proprietary dumps to default CI fixtures.  
- Do not claim silicon GEMM Pass from inventory or capture alone.
