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
use std::fs;
#[cfg(feature = "amd-xdna2")]
use std::path::{Path, PathBuf};

#[cfg(feature = "amd-xdna2")]
use dragon_npu::backends::amd_xdna2::{
    is_aiebu_elf_ctrltext, pack_start_npu_kernel_args_with_ctrl_layout, parse_elf32_rela_dyn,
    patch_aiebu_elf_ctrltext_with_bos, patch_ctrl_from_meta_io_paddrs, LabKernelArgLayout,
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
const STAGED_KEY: &str =
    "mladfmatmulbias_prmptflat_mladf_2x4x4_flat_a16fw4acc16f_1_1152_256_128";

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
fn run() -> i32 {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let silicon = root.join("testdata/xdna2_cmd/silicon_lab");
    let out = std::env::var("XDNA2_GT_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| root.join("testdata/xdna2_cmd/gt_capture"));

    let elf_path = silicon.join("lab_instr.elf");
    let bin_path = silicon.join("lab_instr.bin");
    let ctrl_path = silicon.join("lab_ctrl.bin");
    let meta_path = silicon.join("lab_ctrl_meta.json");

    if !elf_path.is_file() {
        eprintln!("ERROR: missing {elf_path:?}");
        eprintln!("  stage: bash tools/stage_aiebu_lab_instr.sh");
        return 2;
    }
    if !ctrl_path.is_file() || !meta_path.is_file() {
        eprintln!("ERROR: missing lab_ctrl.bin or lab_ctrl_meta.json under {silicon:?}");
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
        eprintln!("ERROR: {elf_path:?} is not aiebu ELF with .ctrltext");
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

    let patched_instr = match patch_aiebu_elf_ctrltext_with_bos(
        &elf,
        IFM_PADDR,
        WTS_PADDR,
        OFM_PADDR,
        CTRL_PADDR,
    ) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("ERROR: rela patch: {e}");
            return 2;
        }
    };

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
    if let Err(e) =
        patch_ctrl_from_meta_io_paddrs(&mut ctrl, &meta, IFM_PADDR, WTS_PADDR, OFM_PADDR)
    {
        eprintln!("ERROR: ctrl patch: {e}");
        return 2;
    }

    let args_count4 = pack_start_npu_kernel_args_with_ctrl_layout(
        IFM_PADDR,
        WTS_PADDR,
        OFM_PADDR,
        Some(CTRL_PADDR),
        LabKernelArgLayout::Count4Ctrl,
    );
    let args_xrt3 = pack_start_npu_kernel_args_with_ctrl_layout(
        IFM_PADDR,
        WTS_PADDR,
        OFM_PADDR,
        Some(CTRL_PADDR),
        LabKernelArgLayout::Xrt3,
    );

    if let Err(e) = fs::create_dir_all(&out) {
        eprintln!("ERROR: mkdir {out:?}: {e}");
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
        "note": "Synthetic paddrs; layout only. Live apply uses real xdna_addr.",
        "paddrs": {
            "ifm": format!("{IFM_PADDR:#x}"),
            "wts": format!("{WTS_PADDR:#x}"),
            "ofm": format!("{OFM_PADDR:#x}"),
            "ctrl": format!("{CTRL_PADDR:#x}"),
        },
        "count4_u32": args_count4,
        "xrt3_u32": args_xrt3,
        "count4_n_u32": args_count4.len(),
        "xrt3_n_u32": args_xrt3.len(),
        "prop_count_expected": 0,
    });
    write_json(&out.join("kernel_args.json"), &kernel_args);

    let contract = serde_json::json!({
        "schema_version": 1,
        "kind": "offline_patch_oracle",
        "key": STAGED_KEY,
        "elf_bytes": elf.len(),
        "ctrltext_post_patch_bytes": patched_instr.len(),
        "ctrl_post_patch_bytes": ctrl.len(),
        "rela_count": relas.len(),
        "is_aiebu_elf": true,
        "paddrs_synthetic": true,
        "instr_sha256": sha256_hex(&patched_instr),
        "ctrl_sha256": sha256_hex(&ctrl),
        "elf_sha256": sha256_hex(&elf),
        "rela": rela_json,
    });
    write_json(&out.join("contract.json"), &contract);

    let provenance = format!(
        r#"# PROVENANCE — offline GT capture (dump_mladf_gt_offline)

**Not live XRT BO dump. Not ran_on_npu.**

## What this is

Pure-direct **patch oracle** for staged `lab_instr.elf` + `lab_ctrl.bin` + meta:
- `.rela.dyn` applied with fixed synthetic paddrs
- ctrl meta size=6 `read_add_write` patches applied

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
| `kernel_args.json` | count4 + xrt3 u32 packs |
| `contract.json` | checksums + rela table |
| `MANIFEST.sha256` | file hashes |

## Key

`{STAGED_KEY}`

## Next

1. Milestone B (optional vendor golden re-lock): `bash tools/hitl_b_gt_capture.sh`
2. Offline parity: `bash tools/silicon_contract_check.sh`
3. Milestone C Pass apply: `bash tools/hitl_c_pass_apply.sh`
"#
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
        "kernel_args.json",
        "contract.json",
        "PROVENANCE.md",
    ] {
        let p = out.join(name);
        if p.is_file() {
            let b = fs::read(&p).unwrap_or_default();
            manifest.push_str(&format!("{}  {}\n", sha256_hex(&b), name));
        }
    }
    if let Err(e) = fs::write(out.join("MANIFEST.sha256"), &manifest) {
        eprintln!("ERROR: write MANIFEST: {e}");
        return 2;
    }

    println!("dump_mladf_gt_offline: OK → {}", out.display());
    println!("  rela_count={} ctrltext={}B ctrl={}B", relas.len(), patched_instr.len(), ctrl.len());
    println!("  instr_post_patch sha256={}", sha256_hex(&patched_instr));
    println!("  ctrl_post_patch  sha256={}", sha256_hex(&ctrl));
    println!();
    println!("Offline only — no L3 / no EXEC / ≠ ran_on_npu.");
    println!("HITL next (optional vendor golden + Pass):");
    println!("  bash tools/hitl_b_gt_capture.sh   # Milestone B");
    println!("  bash tools/hitl_c_pass_apply.sh   # Milestone C");
    0
}

#[cfg(feature = "amd-xdna2")]
fn write_json(path: &Path, v: &serde_json::Value) {
    let s = serde_json::to_string_pretty(v).expect("json");
    fs::write(path, s).unwrap_or_else(|e| panic!("write {path:?}: {e}"));
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
