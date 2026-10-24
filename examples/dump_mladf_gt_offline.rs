//! Offline ground-truth dump for staged mladf a16w4 assets (no device, no L3, no EXEC).
//!
//! Writes patched instruction/ctrl + layout JSON under `testdata/xdna2_cmd/gt_capture/`
//! using fixed synthetic paddrs so pure-direct parity tests are deterministic.
//!
//! ```bash
//! cargo run --features amd-xdna2 --example dump_mladf_gt_offline
//! # or: bash tools/dump_mladf_gt_offline.sh
//! ```
//!
//! Honesty: this is **our** patch oracle, not a live XRT BO dump. Milestone B vendor
//! capture still needed for OFM golden + optional live paddrs; this unblocks offline work.

#![allow(clippy::print_stdout, clippy::print_stderr, clippy::unwrap_used)]

#[cfg(feature = "amd-xdna2")]
use std::fmt::Write;
#[cfg(feature = "amd-xdna2")]
use std::fs;
#[cfg(feature = "amd-xdna2")]
use std::path::{Path, PathBuf};

#[cfg(feature = "amd-xdna2")]
use earth_dragon_npu::backends::amd_xdna2::{
    LAB_DEFAULT_QOS_FPS, LAB_DEFAULT_QOS_GOPS, LabInstrSizeMode, LabKernelArgLayout,
    STAGED_MLADF_CTRL_BYTES, STAGED_MLADF_IFM_BYTES, STAGED_MLADF_OFM_BYTES,
    STAGED_MLADF_WTS_BYTES, build_ert_start_npu_cmd_bo_with_kernel_args, format_dynsym_report,
    is_aiebu_elf_ctrltext, pack_start_npu_kernel_args_with_ctrl_layout, parse_elf32_dynsym,
    parse_elf32_rela_dyn, parse_vendor_paddrs_json, patch_aiebu_elf_ctrltext_with_bos,
    patch_ctrl_from_meta_io_paddrs, prop_count_from_start_npu_cmd_bo, resolve_start_npu_inst_size,
    validate_patched_ctrltext_classic,
};

#[cfg(feature = "amd-xdna2")]
/// Distinctive synthetic paddrs (not real device addrs) for offline parity.
const IFM_PADDR: u64 = 0x0000_A11F_0000_1000;
#[cfg(feature = "amd-xdna2")]
const WTS_PADDR: u64 = 0x0000_A11F_0000_2000;
#[cfg(feature = "amd-xdna2")]
const OFM_PADDR: u64 = 0x0000_A11F_0000_3000;
#[cfg(feature = "amd-xdna2")]
const CTRL_PADDR: u64 = 0x0000_A11F_0000_4000;

#[cfg(feature = "amd-xdna2")]
const STAGED_KEY: &str = "mladfmatmulbias_prmptflat_mladf_2x4x4_flat_a16fw4acc16f_1_1152_256_128";

fn main() {
    #[cfg(not(feature = "amd-xdna2"))]
    {
        eprintln!("dump_mladf_gt_offline requires --features amd-xdna2");
        std::process::exit(1);
    }
    #[cfg(feature = "amd-xdna2")]
    {
        let code = run();
        std::process::exit(code);
    }
}

#[cfg(feature = "amd-xdna2")]
#[allow(clippy::too_many_lines)]
fn run() -> i32 {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let silicon = root.join("testdata/xdna2_cmd/silicon_lab");
    let out = std::env::var("XDNA2_GT_OUT").map_or_else(
        |_| root.join("testdata/xdna2_cmd/gt_capture"),
        PathBuf::from,
    );

    let elf_path = silicon.join("lab_instr.elf");
    let bin_path = silicon.join("lab_instr.bin");
    let ctrl_path = silicon.join("lab_ctrl.bin");
    let meta_path = silicon.join("lab_ctrl_meta.json");

    if !elf_path.is_file() {
        eprintln!("ERROR: missing {}", elf_path.display());
        eprintln!("  stage: bash tools/stage_aiebu_lab_instr.sh");
        return 2;
    }
    if !ctrl_path.is_file() || !meta_path.is_file() {
        eprintln!(
            "ERROR: missing lab_ctrl.bin or lab_ctrl_meta.json under {}",
            silicon.display()
        );
        return 2;
    }

    let elf = match fs::read(&elf_path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("ERROR: read elf: {e}");
            return 2;
        }
    };
    if !is_aiebu_elf_ctrltext(&elf) {
        eprintln!(
            "ERROR: {} is not aiebu ELF with .ctrltext",
            elf_path.display()
        );
        return 2;
    }

    let relas = match parse_elf32_rela_dyn(&elf) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("ERROR: parse rela: {e}");
            return 2;
        }
    };
    if relas.is_empty() {
        eprintln!("ERROR: .rela.dyn empty — cannot dump post-patch instr");
        return 2;
    }

    // Optional: XDNA2_GT_PADDRS_JSON with ifm_paddr/wts_paddr/ofm_paddr/ctrl_paddr as
    // hex strings or numbers — re-patch pure-direct with vendor/live paddrs for fair diff.
    let (ifm_p, wts_p, ofm_p, ctrl_p, paddr_src) =
        load_gt_paddrs_or_synthetic().unwrap_or_else(|| {
            (
                IFM_PADDR,
                WTS_PADDR,
                OFM_PADDR,
                CTRL_PADDR,
                "synthetic".to_string(),
            )
        });
    eprintln!(
        "gt paddrs source={paddr_src} ifm={ifm_p:#x} wts={wts_p:#x} ofm={ofm_p:#x} ctrl={ctrl_p:#x}"
    );

    let patched_instr = match patch_aiebu_elf_ctrltext_with_bos(&elf, ifm_p, wts_p, ofm_p, ctrl_p) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("ERROR: rela patch: {e}");
            return 2;
        }
    };
    // Fail-closed: post-patch ctrltext must walk as classic v0.1 (staged 40 ops / 1688 B).
    if let Err(e) = validate_patched_ctrltext_classic(&patched_instr, true) {
        eprintln!("ERROR: post-patch txn walk refuse: {e}");
        return 2;
    }

    let dynsyms = match parse_elf32_dynsym(&elf) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("ERROR: dynsym parse: {e}");
            return 2;
        }
    };
    let dynsym_report = format_dynsym_report(&dynsyms);
    eprintln!("dynsym: {dynsym_report}");

    let mut ctrl = match fs::read(&ctrl_path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("ERROR: read ctrl: {e}");
            return 2;
        }
    };
    let meta = match fs::read_to_string(&meta_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("ERROR: read meta: {e}");
            return 2;
        }
    };
    // Honor the same paddrs as instr rela patch (XDNA2_GT_PADDRS_JSON or synthetic).
    if let Err(e) = patch_ctrl_from_meta_io_paddrs(&mut ctrl, &meta, ifm_p, wts_p, ofm_p) {
        eprintln!("ERROR: ctrl patch: {e}");
        return 2;
    }

    let args_count4 = pack_start_npu_kernel_args_with_ctrl_layout(
        ifm_p,
        wts_p,
        ofm_p,
        Some(ctrl_p),
        LabKernelArgLayout::Count4Ctrl,
    );
    let args_xrt3 = pack_start_npu_kernel_args_with_ctrl_layout(
        ifm_p,
        wts_p,
        ofm_p,
        Some(ctrl_p),
        LabKernelArgLayout::Xrt3,
    );

    if let Err(e) = fs::create_dir_all(&out) {
        eprintln!("ERROR: mkdir {}: {e}", out.display());
        return 2;
    }

    // Copy pre-patch sources for parity diffs.
    if let Err(e) = fs::copy(&elf_path, out.join("instr_pre.elf")) {
        eprintln!("ERROR: copy elf: {e}");
        return 2;
    }
    if bin_path.is_file() {
        let _ = fs::copy(&bin_path, out.join("instr_raw_txn.bin"));
    }
    if let Err(e) = fs::write(out.join("instr_post_patch.bin"), &patched_instr) {
        eprintln!("ERROR: write instr_post_patch: {e}");
        return 2;
    }
    if let Err(e) = fs::write(out.join("ctrl_post_patch.bin"), &ctrl) {
        eprintln!("ERROR: write ctrl_post_patch: {e}");
        return 2;
    }

    // rela table JSON
    let rela_json = {
        let mut v = Vec::new();
        for (i, r) in relas.iter().enumerate() {
            let site = r.offset as usize;
            let patched_u64 = if site + 8 <= patched_instr.len() {
                u64::from_le_bytes(patched_instr[site..site + 8].try_into().unwrap())
            } else {
                0
            };
            v.push(serde_json::json!({
                "i": i,
                "offset": r.offset,
                "sym": r.sym(),
                "rtype": r.rtype(),
                "addend": r.addend,
                "patched_u64": format!("{patched_u64:#x}"),
            }));
        }
        serde_json::Value::Array(v)
    };

    let kernel_args = serde_json::json!({
        "note": if paddr_src == "synthetic" {
            "Synthetic paddrs; layout only. Live apply uses real xdna_addr."
        } else {
            "Paddrs from XDNA2_GT_PADDRS_JSON for fair vendor/pure re-patch diff."
        },
        "paddr_source": paddr_src,
        "paddrs": {
            "ifm": format!("{ifm_p:#x}"),
            "wts": format!("{wts_p:#x}"),
            "ofm": format!("{ofm_p:#x}"),
            "ctrl": format!("{ctrl_p:#x}"),
        },
        "count4_u32": args_count4,
        "xrt3_u32": args_xrt3,
        "count4_n_u32": args_count4.len(),
        "xrt3_n_u32": args_xrt3.len(),
        "prop_count_expected": 0,
    });
    write_json(&out.join("kernel_args.json"), &kernel_args);

    // START_NPU Cmd BO envelope (count4 layout, prop_count=0) — offline would-submit layout.
    // Default residual trial: content size (1688), not BO page (4096).
    let content_size = u32::try_from(patched_instr.len()).unwrap_or(u32::MAX);
    let bo_alloc = content_size.max(4096);
    let instr_buf_size =
        resolve_start_npu_inst_size(content_size, bo_alloc, LabInstrSizeMode::Content);
    let start_npu = match build_ert_start_npu_cmd_bo_with_kernel_args(
        /* synthetic instr paddr (not a real Dev heap addr) */ 0x0000_A11F_0000_5000,
        instr_buf_size,
        &args_count4,
        0x1,
    ) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("ERROR: build START_NPU: {e}");
            return 2;
        }
    };
    let prop_c = prop_count_from_start_npu_cmd_bo(&start_npu);
    if prop_c != Some(0) {
        eprintln!("ERROR: start_npu_cmd prop_count={prop_c:?} expected Some(0)");
        return 2;
    }
    if let Err(e) = fs::write(out.join("start_npu_cmd.bin"), &start_npu) {
        eprintln!("ERROR: write start_npu_cmd.bin: {e}");
        return 2;
    }

    // Synthetic hwctx / BO table (offline placeholders — not live ioctl dump).
    let hwctx = serde_json::json!({
        "note": "Offline synthetic only — not CREATE_HWCTX output. Live tiles/qos from timed_matmul.",
        "offline": true,
        "tiles": 32,
        "qos_gops": LAB_DEFAULT_QOS_GOPS,
        "qos_fps": LAB_DEFAULT_QOS_FPS,
        "mem_size": 262_144,
        "umq_size": 16384,
        "log_size": 131_072,
        "key": STAGED_KEY,
    });
    write_json(&out.join("hwctx.json"), &hwctx);

    let bo_table = serde_json::json!({
        "note": "Offline size/type plan with synthetic paddrs — not live GET_BO_INFO.",
        "offline": true,
        "bos": [
            {
                "role": "instr",
                "type": "Dev",
                "size_bytes": patched_instr.len(),
                "paddr_synthetic": "0xa11f00005000",
                "content": "ctrltext post-rela (instr_post_patch.bin)",
            },
            {
                "role": "ifm",
                "type": "Shmem",
                "size_bytes": STAGED_MLADF_IFM_BYTES,
                "paddr_synthetic": format!("{IFM_PADDR:#x}"),
            },
            {
                "role": "wts",
                "type": "Shmem",
                "size_bytes": STAGED_MLADF_WTS_BYTES,
                "paddr_synthetic": format!("{WTS_PADDR:#x}"),
            },
            {
                "role": "ofm",
                "type": "Shmem",
                "size_bytes": STAGED_MLADF_OFM_BYTES,
                "paddr_synthetic": format!("{OFM_PADDR:#x}"),
            },
            {
                "role": "ctrl",
                "type": "Shmem",
                "size_bytes": STAGED_MLADF_CTRL_BYTES,
                "paddr_synthetic": format!("{CTRL_PADDR:#x}"),
                "content": "ctrl_post_patch.bin after meta read_add_write",
            },
            {
                "role": "cmd",
                "type": "Cmd",
                "size_bytes": start_npu.len(),
                "content": "start_npu_cmd.bin (ERT_START_NPU prop_count=0 count4)",
            },
        ],
    });
    write_json(&out.join("bo_table.json"), &bo_table);

    // Optional I/O copies from silicon_lab (if present; not required for oracle).
    for name in ["ifm.bin", "wts.bin", "ofm_golden.bin"] {
        let src = silicon.join(name);
        if src.is_file() {
            let _ = fs::copy(&src, out.join(name));
        }
    }

    let dynsym_json: Vec<serde_json::Value> = dynsyms
        .iter()
        .map(|s| {
            serde_json::json!({
                "index": s.index,
                "name": s.name,
                "value": s.value,
                "size": s.size,
                "info": s.info,
                "shndx": s.shndx,
            })
        })
        .collect();

    let contract = serde_json::json!({
        "schema_version": 3,
        "kind": "offline_patch_oracle",
        "key": STAGED_KEY,
        "elf_bytes": elf.len(),
        "ctrltext_post_patch_bytes": patched_instr.len(),
        "ctrl_post_patch_bytes": ctrl.len(),
        "start_npu_cmd_bytes": start_npu.len(),
        "start_npu_prop_count": prop_c,
        "start_npu_instr_size": instr_buf_size,
        "start_npu_instr_size_mode": "content",
        "instr_bo_alloc": bo_alloc,
        "rela_count": relas.len(),
        "is_aiebu_elf": true,
        "post_patch_txn_walk_ok": true,
        "post_patch_num_ops_expected": 40,
        "paddrs_synthetic": true,
        "dynsym_report": dynsym_report,
        "dynsym_note": "Staged aiebu names are opaque \"6\"; lab map is index 1→ifm 2→wts 3→ofm (do not invent name map)",
        "dynsym": dynsym_json,
        "instr_sha256": sha256_hex(&patched_instr),
        "ctrl_sha256": sha256_hex(&ctrl),
        "elf_sha256": sha256_hex(&elf),
        "start_npu_sha256": sha256_hex(&start_npu),
        "rela": rela_json,
    });
    write_json(&out.join("contract.json"), &contract);

    let provenance = format!(
        r"# PROVENANCE — offline GT capture (dump_mladf_gt_offline)

**Not live XRT BO dump. Not ran_on_npu.**

## What this is

Pure-direct **patch oracle** for staged `lab_instr.elf` + `lab_ctrl.bin` + meta:
- `.rela.dyn` applied with fixed synthetic paddrs
- ctrl meta size=6 `read_add_write` patches applied
- START_NPU envelope + synthetic hwctx/bo_table (layout only)

## Synthetic paddrs

| Role | Value |
|------|-------|
| IFM | `{IFM_PADDR:#x}` |
| WTS | `{WTS_PADDR:#x}` |
| OFM | `{OFM_PADDR:#x}` |
| CTRL | `{CTRL_PADDR:#x}` |

## Files

| File | Role |
|------|------|
| `instr_pre.elf` | staged aiebu ELF |
| `instr_post_patch.bin` | ctrltext after rela patch |
| `ctrl_post_patch.bin` | lab_ctrl after meta patch |
| `start_npu_cmd.bin` | ERT_START_NPU envelope (count4, prop_count=0) |
| `hwctx.json` | synthetic tiles=32 qos 100/30 |
| `bo_table.json` | instr Dev + IFM/WTS/OFM/ctrl Shmem sizes |
| `kernel_args.json` | count4 + xrt3 u32 packs |
| `contract.json` | checksums + rela table |
| `ifm.bin` / `wts.bin` / `ofm_golden.bin` | optional copies from silicon_lab |
| `MANIFEST.sha256` | file hashes |

## Key

`{STAGED_KEY}`

## Next

1. Offline contract (self-check only): `bash tools/silicon_contract_check.sh` — **CONTRACT_OK ≠ C-ready**
2. Offline parity: `bash tools/diff_vendor_instr_parity.sh` (0=unpatched body match CLASS A ≠ C-ready; 2=mismatch+CLASS_HINT; 3=miss)
3. Agent offline pure-direct / APP_LOAD residual if needed (ban env C applies)
4. Optional capture-instr (logs only; unpatched A already body-green): L3 → `capture-instr` → close → `diff`
5. Milestone C **only if agent says ready** — **not** unlocked by `diff rc=0` / CLASS A / registry / parity alone: `bash tools/hitl_c_pass_apply.sh`
6. Optional Milestone B OFM golden re-lock: `bash tools/hitl_b_gt_capture.sh --with-vendor`

**Honesty:** unpatched CLASS A / mere `diff rc=0` = body match only → **≠ C-ready**, **≠ `ran_on_npu`**.
"
    );
    if let Err(e) = fs::write(out.join("PROVENANCE.md"), provenance) {
        eprintln!("ERROR: write PROVENANCE: {e}");
        return 2;
    }

    // MANIFEST
    let mut manifest = String::new();
    for name in [
        "instr_pre.elf",
        "instr_post_patch.bin",
        "ctrl_post_patch.bin",
        "start_npu_cmd.bin",
        "hwctx.json",
        "bo_table.json",
        "kernel_args.json",
        "contract.json",
        "PROVENANCE.md",
        "ifm.bin",
        "wts.bin",
        "ofm_golden.bin",
        "instr_raw_txn.bin",
    ] {
        let p = out.join(name);
        if p.is_file() {
            let b = fs::read(&p).unwrap_or_default();
            let _ = writeln!(manifest, "{}  {}", sha256_hex(&b), name);
        }
    }
    if let Err(e) = fs::write(out.join("MANIFEST.sha256"), &manifest) {
        eprintln!("ERROR: write MANIFEST: {e}");
        return 2;
    }

    println!("dump_mladf_gt_offline: OK → {}", out.display());
    println!(
        "  rela_count={} ctrltext={}B ctrl={}B start_npu={}B",
        relas.len(),
        patched_instr.len(),
        ctrl.len(),
        start_npu.len()
    );
    println!("  instr_post_patch sha256={}", sha256_hex(&patched_instr));
    println!("  ctrl_post_patch  sha256={}", sha256_hex(&ctrl));
    println!("  start_npu_cmd    sha256={}", sha256_hex(&start_npu));
    println!();
    println!("Offline only — no L3 / no EXEC / ≠ ran_on_npu.");
    println!("Next (CONTRACT_OK ≠ C-ready; unpatched A / diff rc=0 ≠ C-ready):");
    println!(
        "  bash tools/diff_vendor_instr_parity.sh             # 0 body match CLASS A ≠ C-ready"
    );
    println!("  # optional capture-instr for logs only (not C unlock; research closed offline)");
    println!("  # C only if agent says ready (not parity green / registry alone):");
    println!("  # bash tools/hitl_c_pass_apply.sh --assume-l3-already-open");
    0
}

#[cfg(feature = "amd-xdna2")]
fn write_json(path: &Path, v: &serde_json::Value) {
    let s = serde_json::to_string_pretty(v).expect("json");
    fs::write(path, s).unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
}

/// Load paddrs from `XDNA2_GT_PADDRS_JSON` when all of ifm/wts/ofm present.
///
/// Uses shared [`parse_vendor_paddrs_json`] (schema v1 null/hex/u64).
#[cfg(feature = "amd-xdna2")]
fn load_gt_paddrs_or_synthetic() -> Option<(u64, u64, u64, u64, String)> {
    let path = std::env::var("XDNA2_GT_PADDRS_JSON").ok()?;
    let text = fs::read_to_string(&path).ok()?;
    let parsed = parse_vendor_paddrs_json(&text).ok()?;
    let (ifm, wts, ofm, ctrl) = parsed.as_io_tuple()?;
    Some((ifm, wts, ofm, ctrl, path))
}

/// SHA-256 via `sha256sum` (zero new crate deps). FNV fallback only if missing.
#[cfg(feature = "amd-xdna2")]
fn sha256_hex(bytes: &[u8]) -> String {
    let tmp = std::env::temp_dir().join(format!("dragon_npu_gt_{}.bin", std::process::id()));
    if fs::write(&tmp, bytes).is_err() {
        return "ERR".into();
    }
    let out = std::process::Command::new("sha256sum").arg(&tmp).output();
    let _ = fs::remove_file(&tmp);
    match out {
        Ok(o) if o.status.success() => {
            let s = String::from_utf8_lossy(&o.stdout);
            s.split_whitespace().next().unwrap_or("ERR").to_string()
        }
        _ => {
            let mut h: u64 = 0xcbf2_9ce4_8422_2325;
            for &b in bytes {
                h ^= u64::from(b);
                h = h.wrapping_mul(0x0100_0000_01b3);
            }
            format!("fnv1a64_{h:016x}")
        }
    }
}
