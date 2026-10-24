//! Named IRON kernel catalog (`--iron` / `just npu-relu`).
//!
//! Points at existing mlir-aie examples. Does **not** claim product Pass.
//! Child `PASS!` is not `ran_on_npu`. Soft always `invent_pass=0`.

use std::path::{Path, PathBuf};

/// One named IRON example: relative cwd under mlir-aie plus python argv.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IronKernelSpec {
    /// Catalog name (`vector_add`, `eltwise`, `relu`, `softmax`, wave-2 unaries).
    pub name: &'static str,
    /// Directory under the mlir-aie root (no leading slash).
    pub cwd_suffix: &'static str,
    /// Child argv (script name first). Never `sh -c`.
    pub argv: &'static [&'static str],
}

/// Known catalog names, stable order for help / error text.
pub const IRON_KERNEL_NAMES: &[&str] = &[
    "vector_add",
    "eltwise",
    "relu",
    "softmax",
    "gelu",
    "silu",
    "eltwise_mul",
    "swiglu",
    "scale_shift",
];

/// Second Pass identity (docs + match helpers). Not classic 1688. Not product Pass.
pub const IRON_NPU2_VECTOR_ADD_JOB: &str = "iron_npu2_vector_add";

/// Classic lab matmul job name. Frozen, separate from [`IRON_NPU2_VECTOR_ADD_JOB`].
pub const MLADF_CLASSIC_1688_JOB: &str = "mladf_classic_1688";

/// Look up a named IRON kernel. Unknown names fail closed.
///
/// # Errors
///
/// Unknown or empty name.
pub fn iron_kernel_spec(name: &str) -> Result<IronKernelSpec, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(format!(
            "unknown --iron name (empty). Known: {}",
            IRON_KERNEL_NAMES.join(" ")
        ));
    }
    let spec = match trimmed {
        "vector_add" => IronKernelSpec {
            name: "vector_add",
            cwd_suffix: "programming_examples/basic/vector_scalar_add",
            argv: &["vector_scalar_add.py", "--dev", "npu2"],
        },
        "eltwise" => IronKernelSpec {
            name: "eltwise",
            cwd_suffix: "programming_examples/ml/eltwise",
            argv: &["eltwise.py", "--dev", "npu2"],
        },
        "relu" => IronKernelSpec {
            name: "relu",
            cwd_suffix: "programming_examples/ml/eltwise_unary",
            argv: &["eltwise_unary.py", "--dev", "npu2", "--op", "relu"],
        },
        "softmax" => IronKernelSpec {
            name: "softmax",
            cwd_suffix: "programming_examples/ml/softmax",
            argv: &["softmax.py", "--dev", "npu2"],
        },
        "gelu" => IronKernelSpec {
            name: "gelu",
            cwd_suffix: "programming_examples/ml/eltwise_unary",
            argv: &["eltwise_unary.py", "--dev", "npu2", "--op", "gelu"],
        },
        "silu" => IronKernelSpec {
            name: "silu",
            cwd_suffix: "programming_examples/ml/eltwise_unary",
            argv: &["eltwise_unary.py", "--dev", "npu2", "--op", "silu"],
        },
        "eltwise_mul" => IronKernelSpec {
            name: "eltwise_mul",
            cwd_suffix: "programming_examples/ml/eltwise",
            argv: &["eltwise.py", "--dev", "npu2", "--op", "mul"],
        },
        "swiglu" => IronKernelSpec {
            name: "swiglu",
            cwd_suffix: "programming_examples/ml/swiglu",
            argv: &["swiglu.py", "--dev", "npu2"],
        },
        "scale_shift" => IronKernelSpec {
            name: "scale_shift",
            cwd_suffix: "programming_examples/ml/scale_shift",
            argv: &["scale_shift.py", "--dev", "npu2"],
        },
        other => {
            return Err(format!(
                "unknown --iron name '{other}'. Known: {}",
                IRON_KERNEL_NAMES.join(" ")
            ));
        }
    };
    Ok(spec)
}

/// Parse a child vector from stdout and/or in-memory `.npy` bytes.
///
/// Prefer a valid npy payload when present. Otherwise take the first JSON
/// array of numbers in stdout. Missing or unparseable numbers return an
/// empty vec. Never invents OFM. Not product Pass.
#[must_use]
pub fn parse_child_f32_vector(stdout: &str, npy: Option<&[u8]>) -> Vec<f32> {
    if let Some(bytes) = npy
        && let Some(vals) = parse_npy_f32(bytes)
        && !vals.is_empty()
    {
        return vals;
    }
    parse_stdout_f32_json_array(stdout).unwrap_or_default()
}

/// First `[…]` JSON array of finite numbers in `stdout`, if any.
///
/// Skip arrays labeled as a shape (`shape [256]` / `shape: [256]`). Those are
/// not this-run OFM.
fn parse_stdout_f32_json_array(stdout: &str) -> Option<Vec<f32>> {
    let bytes = stdout.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'['
            && !array_preceded_by_shape_label(stdout, i)
            && let Some((vals, end)) = try_parse_json_number_array(&stdout[i..])
        {
            let _ = end;
            if !vals.is_empty() {
                return Some(vals);
            }
        }
        i += 1;
    }
    None
}

/// True when `[` is the incidental shape print (`shape [256]`, `shape: [256]`).
fn array_preceded_by_shape_label(stdout: &str, bracket_idx: usize) -> bool {
    let Some(before) = stdout.get(..bracket_idx) else {
        return false;
    };
    let before = before.trim_end();
    let before = before.strip_suffix(':').map_or(before, str::trim_end);
    before
        .rsplit(|c: char| c.is_ascii_whitespace())
        .next()
        .is_some_and(|w| w.eq_ignore_ascii_case("shape"))
}

fn try_parse_json_number_array(from_bracket: &str) -> Option<(Vec<f32>, usize)> {
    let close = from_bracket.find(']')?;
    let inner = from_bracket.get(1..close)?;
    let mut vals = Vec::new();
    let trimmed = inner.trim();
    if trimmed.is_empty() {
        return Some((vals, close + 1));
    }
    for part in trimmed.split(',') {
        let t = part.trim();
        if t.is_empty() {
            return None;
        }
        let n: f32 = t.parse().ok()?;
        if !n.is_finite() {
            return None;
        }
        vals.push(n);
    }
    Some((vals, close + 1))
}

/// Minimal little-endian NumPy `.npy` v1/v2 reader for 1-D (or flat) `<f4`.
fn parse_npy_f32(bytes: &[u8]) -> Option<Vec<f32>> {
    if bytes.len() < 10 || &bytes[0..6] != b"\x93NUMPY" {
        return None;
    }
    let major = bytes[6];
    let header_len = match major {
        1 => {
            if bytes.len() < 10 {
                return None;
            }
            u16::from_le_bytes([bytes[8], bytes[9]]) as usize
        }
        2 => {
            if bytes.len() < 12 {
                return None;
            }
            u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize
        }
        _ => return None,
    };
    let header_start: usize = if major == 1 { 10 } else { 12 };
    let data_start = header_start.checked_add(header_len)?;
    if data_start > bytes.len() {
        return None;
    }
    let header = std::str::from_utf8(&bytes[header_start..data_start]).ok()?;
    if !header.contains("'<f4'") && !header.contains("\"<f4\"") {
        return None;
    }
    if header.contains("fortran_order': True") || header.contains("fortran_order\": True") {
        return None;
    }
    let data = &bytes[data_start..];
    if !data.len().is_multiple_of(4) {
        return None;
    }
    let mut out = Vec::with_capacity(data.len() / 4);
    for chunk in data.as_chunks::<4>().0 {
        let n = f32::from_le_bytes(*chunk);
        if !n.is_finite() {
            return None;
        }
        out.push(n);
    }
    Some(out)
}

/// Resolve mlir-aie the same way as `tools/npu_iron_env.sh` (no `$MLIR_AIE`
/// required). Used by CLI `--iron`.
///
/// # Errors
///
/// No candidate tree contains the vector_scalar_add example script.
pub fn resolve_mlir_aie_root() -> Result<PathBuf, String> {
    resolve_mlir_aie_root_from(mlir_aie_candidates())
}

/// Testable resolve: first candidate that contains the catalog marker script.
///
/// # Errors
///
/// No candidate matches.
pub fn resolve_mlir_aie_root_from(candidates: Vec<PathBuf>) -> Result<PathBuf, String> {
    let marker = "programming_examples/basic/vector_scalar_add/vector_scalar_add.py";
    for cand in candidates {
        if cand.as_os_str().is_empty() {
            continue;
        }
        if cand.join(marker).is_file() {
            return Ok(cand);
        }
    }
    Err(
        "could not find mlir-aie (looked for programming_examples/basic/vector_scalar_add). \
         Set MLIR_AIE or clone mlir-aie next to this repo."
            .to_string(),
    )
}

fn mlir_aie_candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(p) = std::env::var("MLIR_AIE") {
        let t = p.trim();
        if !t.is_empty() {
            out.push(PathBuf::from(t));
        }
    }
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR")
        && let Some(parent) = Path::new(&manifest).parent()
    {
        out.push(parent.join("mlir-aie"));
    }
    if let Ok(home) = std::env::var("HOME") {
        let home = PathBuf::from(home);
        out.push(home.join("Projects/ai/mlir-aie"));
        out.push(home.join("mlir-aie"));
        out.push(home.join("src/mlir-aie"));
    }
    out
}

/// Absolute python for `--iron` (env first, then `/usr/bin/python3`).
///
/// # Errors
///
/// No usable absolute interpreter.
pub fn resolve_iron_python() -> Result<PathBuf, String> {
    for key in ["NPU_IRON_PYTHON", "PYTHON"] {
        if let Ok(p) = std::env::var(key) {
            let t = p.trim();
            if t.is_empty() {
                continue;
            }
            let path = PathBuf::from(t);
            if path.is_absolute() && path.is_file() {
                return Ok(path);
            }
        }
    }
    let fallback = PathBuf::from("/usr/bin/python3");
    if fallback.is_file() {
        return Ok(fallback);
    }
    Err(
        "no IRON python (set NPU_IRON_PYTHON or PYTHON to an absolute interpreter, \
         or install /usr/bin/python3)"
            .to_string(),
    )
}

/// Absolute example cwd + argv for a named kernel under `mlir_root`.
///
/// # Errors
///
/// Unknown name, or example directory missing.
pub fn iron_kernel_cwd_and_argv(
    name: &str,
    mlir_root: &Path,
) -> Result<(PathBuf, Vec<String>), String> {
    let spec = iron_kernel_spec(name)?;
    let cwd = mlir_root.join(spec.cwd_suffix);
    if !cwd.is_dir() {
        return Err(format!(
            "IRON example dir is not a directory: {}",
            cwd.display()
        ));
    }
    let argv = spec.argv.iter().map(|s| (*s).to_string()).collect();
    Ok((cwd, argv))
}

/// True when the sibling example already documents `--aot-dir` (vector add).
#[must_use]
pub fn iron_kernel_accepts_aot_dir(name: &str) -> bool {
    name.trim() == "vector_add"
}

/// Create a this-run `--aot-dir` only for explicit export of `vector_add`.
///
/// The default wrap is run+verify (no `--aot-dir`). Sibling
/// `vector_scalar_add.py --aot-dir` is export-only (no child `PASS!`).
#[must_use]
pub fn iron_kernel_should_create_this_run_aot_dir(name: &str, export_requested: bool) -> bool {
    export_requested && iron_kernel_accepts_aot_dir(name)
}

/// Catalog cwd + argv, plus `--aot-dir` when this kernel accepts it.
///
/// # Errors
///
/// Unknown name, or example directory missing.
pub fn iron_kernel_cwd_and_argv_with_aot(
    name: &str,
    mlir_root: &Path,
    this_run_aot_dir: Option<&Path>,
) -> Result<(PathBuf, Vec<String>), String> {
    let (cwd, mut argv) = iron_kernel_cwd_and_argv(name, mlir_root)?;
    if let Some(dir) = this_run_aot_dir
        && iron_kernel_accepts_aot_dir(name)
    {
        argv.push("--aot-dir".into());
        argv.push(dir.to_string_lossy().into_owned());
    }
    Ok((cwd, argv))
}

/// Create an exclusive this-run directory under the process temp dir.
///
/// # Errors
///
/// Temp create failed.
pub fn create_this_run_aot_dir(kernel: &str) -> Result<PathBuf, String> {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let safe: String = kernel
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    let dir = std::env::temp_dir().join(format!(
        "dragon_npu_iron_aot_{safe}_{}_{stamp}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).map_err(|e| format!("this-run aot dir: {e}"))?;
    Ok(dir)
}

/// Known artifact suffixes written by `vector_scalar_add.py --aot-dir`.
pub const THIS_RUN_AOT_ARTIFACT_NAMES: [&str; 9] = [
    "vector_scalar_add.xclbin",
    "vector_scalar_add.insts.bin",
    "vector_scalar_add.pdi",
    "vector_scalar_add.insts.elf",
    "output.npy",
    "out.npy",
    "ofm.npy",
    "result.npy",
    "c_out.npy",
];

/// List present this-run AOT artifacts (non-empty files only). Missing dir → empty.
#[must_use]
pub fn list_this_run_aot_artifacts(aot_dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for name in THIS_RUN_AOT_ARTIFACT_NAMES {
        let p = aot_dir.join(name);
        if p.is_file() {
            match std::fs::metadata(&p) {
                Ok(meta) if meta.len() > 0 => out.push(p),
                _ => {}
            }
        }
    }
    out
}

/// Best-effort remove of an exclusive this-run aot directory after collect.
pub fn remove_this_run_aot_dir_best_effort(aot_dir: &Path) {
    let _ = std::fs::remove_dir_all(aot_dir);
}

/// Load a known npy from an exclusive this-run `--aot-dir`.
///
/// Unlike [`load_this_run_npy_bytes`], leftover example-cwd files are not
/// in this directory, so a basename mention is not required.
#[must_use]
pub fn load_this_run_npy_from_aot_dir(aot_dir: &Path) -> Option<Vec<u8>> {
    for name in KNOWN_OUTPUT_NPY_NAMES {
        let p = aot_dir.join(name);
        if let Ok(bytes) = std::fs::read(&p)
            && !bytes.is_empty()
        {
            return Some(bytes);
        }
    }
    None
}

/// Collect PROVENANCE line for an IRON job. Never a 1688 residual match.
///
/// Soft always `invent_pass=0 not_ran_on_npu=1 residual_matched_claim=0`.
#[must_use]
pub fn format_iron_job_collect_note(
    job: &str,
    artifact_count: usize,
    residual_matched_claim_1688: bool,
) -> String {
    let _ = residual_matched_claim_1688;
    format!(
        "job={} artifact_count={artifact_count} residual_matched_claim=0 \
         classic_1688_claim_refused=1 invent_pass=0 not_ran_on_npu=1 \
         not_c_ready=1 product_known_good=0 collect_neq_product_pass=1",
        if job.trim().is_empty() {
            IRON_NPU2_VECTOR_ADD_JOB
        } else {
            job.trim()
        }
    )
}

/// One allow-list for the IRON npu2 vector-add job (classify / refuse / capture).
///
/// Accepts underscore or hyphen, case-insensitive. Empty / whitespace is not a job.
#[must_use]
pub fn is_iron_npu2_vector_add_job_name(job: &str) -> bool {
    let t = job.trim();
    if t.is_empty() {
        return false;
    }
    let norm: String = t
        .chars()
        .map(|c| {
            if c == '-' {
                '_'
            } else {
                c.to_ascii_lowercase()
            }
        })
        .collect();
    norm == IRON_NPU2_VECTOR_ADD_JOB
}

/// True when this job must never set `residual_matched_claim` against classic 1688.
#[must_use]
pub fn refuse_classic_1688_residual_claim_for_job(job: &str) -> bool {
    is_iron_npu2_vector_add_job_name(job)
}

/// Known `.npy` basenames next to an IRON example (optional numbers).
pub const KNOWN_OUTPUT_NPY_NAMES: [&str; 5] = [
    "output.npy",
    "out.npy",
    "ofm.npy",
    "result.npy",
    "c_out.npy",
];

/// Load the first known npy beside `cwd` (bytes only). Missing is `None`.
///
/// This is a file-exists helper. Product parse uses
/// [`load_this_run_npy_bytes`] so leftover files are not this-run OFM.
#[must_use]
pub fn load_known_npy_bytes(cwd: &Path) -> Option<Vec<u8>> {
    for name in KNOWN_OUTPUT_NPY_NAMES {
        let p = cwd.join(name);
        if let Ok(bytes) = std::fs::read(&p)
            && !bytes.is_empty()
        {
            return Some(bytes);
        }
    }
    None
}

/// Load a known npy beside `cwd` only when this child mentioned its basename.
///
/// Leftover `output.npy` with stdout `PASS!` only is **not** this-run OFM.
#[must_use]
pub fn load_this_run_npy_bytes(cwd: &Path, stdout: &str, stderr: &str) -> Option<Vec<u8>> {
    for name in KNOWN_OUTPUT_NPY_NAMES {
        if !child_mentions_npy_basename(stdout, stderr, name) {
            continue;
        }
        let p = cwd.join(name);
        if let Ok(bytes) = std::fs::read(&p)
            && !bytes.is_empty()
        {
            return Some(bytes);
        }
    }
    None
}

fn child_mentions_npy_basename(stdout: &str, stderr: &str, name: &str) -> bool {
    stdout.contains(name) || stderr.contains(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iron_kernel_vector_add_cwd_suffix_and_argv() {
        let spec = iron_kernel_spec("vector_add").expect("vector_add is a catalog name");
        assert_eq!(spec.name, "vector_add");
        assert_eq!(
            spec.cwd_suffix,
            "programming_examples/basic/vector_scalar_add"
        );
        assert_eq!(spec.argv, ["vector_scalar_add.py", "--dev", "npu2"]);
    }

    #[test]
    fn iron_kernel_eltwise_cwd_suffix_and_argv() {
        let spec = iron_kernel_spec("eltwise").expect("eltwise is a catalog name");
        assert_eq!(spec.cwd_suffix, "programming_examples/ml/eltwise");
        assert_eq!(spec.argv, ["eltwise.py", "--dev", "npu2"]);
    }

    #[test]
    fn iron_kernel_relu_cwd_suffix_and_argv() {
        let spec = iron_kernel_spec("relu").expect("relu is a catalog name");
        assert_eq!(spec.cwd_suffix, "programming_examples/ml/eltwise_unary");
        assert_eq!(
            spec.argv,
            ["eltwise_unary.py", "--dev", "npu2", "--op", "relu"]
        );
    }

    #[test]
    fn iron_kernel_softmax_cwd_suffix_and_argv() {
        let spec = iron_kernel_spec("softmax").expect("softmax is a catalog name");
        assert_eq!(spec.cwd_suffix, "programming_examples/ml/softmax");
        assert_eq!(spec.argv, ["softmax.py", "--dev", "npu2"]);
    }

    #[test]
    fn iron_kernel_gelu_same_unary_script_op_gelu() {
        let spec = iron_kernel_spec("gelu").expect("gelu is a catalog name");
        assert_eq!(spec.name, "gelu");
        assert_eq!(spec.cwd_suffix, "programming_examples/ml/eltwise_unary");
        assert_eq!(
            spec.argv,
            ["eltwise_unary.py", "--dev", "npu2", "--op", "gelu"]
        );
    }

    #[test]
    fn iron_kernel_silu_same_unary_script_op_silu() {
        let spec = iron_kernel_spec("silu").expect("silu is a catalog name");
        assert_eq!(spec.cwd_suffix, "programming_examples/ml/eltwise_unary");
        assert_eq!(
            spec.argv,
            ["eltwise_unary.py", "--dev", "npu2", "--op", "silu"]
        );
    }

    #[test]
    fn iron_kernel_eltwise_mul_same_eltwise_script_op_mul() {
        let spec = iron_kernel_spec("eltwise_mul").expect("eltwise_mul is a catalog name");
        assert_eq!(spec.cwd_suffix, "programming_examples/ml/eltwise");
        assert_eq!(spec.argv, ["eltwise.py", "--dev", "npu2", "--op", "mul"]);
    }

    #[test]
    fn iron_kernel_swiglu_cwd_suffix_and_argv() {
        let spec = iron_kernel_spec("swiglu").expect("swiglu is a catalog name");
        assert_eq!(spec.cwd_suffix, "programming_examples/ml/swiglu");
        assert_eq!(spec.argv, ["swiglu.py", "--dev", "npu2"]);
    }

    #[test]
    fn iron_kernel_scale_shift_cwd_suffix_and_argv() {
        let spec = iron_kernel_spec("scale_shift").expect("scale_shift is a catalog name");
        assert_eq!(spec.cwd_suffix, "programming_examples/ml/scale_shift");
        assert_eq!(spec.argv, ["scale_shift.py", "--dev", "npu2"]);
    }

    #[test]
    fn iron_kernel_names_lists_wave1_and_wave2() {
        for name in [
            "vector_add",
            "eltwise",
            "relu",
            "softmax",
            "gelu",
            "silu",
            "eltwise_mul",
            "swiglu",
            "scale_shift",
        ] {
            assert!(
                IRON_KERNEL_NAMES.contains(&name),
                "catalog must list {name}: {IRON_KERNEL_NAMES:?}"
            );
        }
    }

    #[test]
    fn iron_kernel_unknown_name_fails_closed() {
        let err = iron_kernel_spec("not-a-kernel").expect_err("unknown name must refuse");
        assert!(
            err.contains("unknown") && err.contains("not-a-kernel"),
            "expected unknown-name refuse, got: {err}"
        );
        assert!(
            err.contains("vector_add") && err.contains("relu"),
            "error should list known names, got: {err}"
        );
    }

    #[test]
    fn iron_kernel_empty_name_fails_closed() {
        let err = iron_kernel_spec("  ").expect_err("empty name must refuse");
        assert!(err.contains("unknown"), "got: {err}");
    }

    #[test]
    fn iron_parse_stdout_json_array_to_f32() {
        let v = parse_child_f32_vector("verify: PASS!\nresult: [1.0, 2.5, -3]\n", None);
        assert_eq!(
            v,
            vec![1.0, 2.5, -3.0],
            "fixture stdout JSON array must become Vec<f32>"
        );
    }

    #[test]
    fn iron_parse_missing_numbers_is_empty_not_error() {
        let v = parse_child_f32_vector("PASS!\nGFLOPS 3.2\n", None);
        assert!(
            v.is_empty(),
            "PASS! / GFLOPS must not invent a tensor: {v:?}"
        );
    }

    #[test]
    fn iron_parse_tiny_npy_to_f32() {
        let bytes = tiny_npy_f32(&[0.5, 1.25, -2.0]);
        let v = parse_child_f32_vector("PASS!", Some(&bytes));
        assert_eq!(v, vec![0.5, 1.25, -2.0]);
    }

    #[test]
    fn iron_parse_invalid_npy_without_stdout_numbers_is_empty() {
        let v = parse_child_f32_vector("no numbers here", Some(b"not npy"));
        assert!(v.is_empty(), "bad npy must not invent OFM: {v:?}");
    }

    #[test]
    fn iron_parse_must_not_treat_shape_bracket_as_ofm() {
        let v = parse_child_f32_vector("shape [256]\nPASS!\n", None);
        assert!(
            v.is_empty(),
            "incidental shape [256] must not become OFM: {v:?}"
        );
        let colon = parse_child_f32_vector("shape: [256]\nPASS!\n", None);
        assert!(
            colon.is_empty(),
            "shape: [256] must not become OFM: {colon:?}"
        );
        let after = parse_child_f32_vector("shape [256]\nresult: [1.0, 2.0]\n", None);
        assert_eq!(
            after,
            vec![1.0, 2.0],
            "a later labeled result array is still OFM"
        );
    }

    #[test]
    fn iron_parse_npy_wins_over_stdout_json() {
        let bytes = tiny_npy_f32(&[0.25, 0.5]);
        let v = parse_child_f32_vector("result: [9.0]\nPASS!\n", Some(&bytes));
        assert_eq!(v, vec![0.25, 0.5], "valid npy must win over stdout JSON");
    }

    #[test]
    fn load_known_npy_bytes_stale_file_is_not_this_run_ofm() {
        let tmp = std::env::temp_dir().join(format!(
            "dragon_npu_stale_npy_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos())
        ));
        std::fs::create_dir_all(&tmp).expect("temp cwd");
        let npy = tiny_npy_f32(&[99.0, 98.0]);
        std::fs::write(tmp.join("output.npy"), npy).expect("write leftover npy");
        let bytes = load_this_run_npy_bytes(&tmp, "PASS!\n", "");
        let v = parse_child_f32_vector("PASS!\n", bytes.as_deref());
        let _ = std::fs::remove_dir_all(&tmp);
        assert!(
            v.is_empty(),
            "leftover output.npy with no this-run mention must not be OFM: {v:?}"
        );
    }

    #[test]
    fn load_this_run_npy_bytes_requires_basename_mention() {
        let tmp = std::env::temp_dir().join(format!(
            "dragon_npu_this_run_npy_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(2, |d| d.as_nanos())
        ));
        std::fs::create_dir_all(&tmp).expect("temp cwd");
        let npy = tiny_npy_f32(&[0.25, 0.5]);
        std::fs::write(tmp.join("output.npy"), npy).expect("write npy");
        let mentioned = load_this_run_npy_bytes(&tmp, "wrote output.npy\nPASS!\n", "");
        let silent = load_this_run_npy_bytes(&tmp, "PASS!\n", "");
        let _ = std::fs::remove_dir_all(&tmp);
        assert!(mentioned.is_some(), "mention must allow this-run npy");
        assert!(silent.is_none(), "no mention must refuse leftover npy");
    }

    #[test]
    fn resolve_mlir_aie_root_from_first_marker_wins() {
        let tmp = std::env::temp_dir().join(format!(
            "dragon_npu_mlir_resolve_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(1, |d| d.as_nanos())
        ));
        let first = tmp.join("first");
        let second = tmp.join("second");
        let marker = "programming_examples/basic/vector_scalar_add/vector_scalar_add.py";
        std::fs::create_dir_all(first.join(marker).parent().expect("parent")).expect("first");
        std::fs::create_dir_all(second.join(marker).parent().expect("parent")).expect("second");
        std::fs::write(first.join(marker), b"# marker\n").expect("first marker");
        std::fs::write(second.join(marker), b"# marker\n").expect("second marker");
        let empty = tmp.join("empty");
        std::fs::create_dir_all(&empty).expect("empty");
        let got = resolve_mlir_aie_root_from(vec![empty, first.clone(), second]).expect("marker");
        let _ = std::fs::remove_dir_all(&tmp);
        assert_eq!(got, first);
    }

    #[test]
    fn iron_kernel_cwd_and_argv_missing_dir_fails_closed() {
        let tmp =
            std::env::temp_dir().join(format!("dragon_npu_missing_iron_{}", std::process::id()));
        std::fs::create_dir_all(&tmp).expect("tmp");
        let err = iron_kernel_cwd_and_argv("relu", &tmp).expect_err("missing example dir");
        let _ = std::fs::remove_dir_all(&tmp);
        assert!(
            err.contains("not a directory"),
            "missing example must fail closed, got: {err}"
        );
    }

    #[test]
    fn iron_kernel_vector_add_accepts_aot_dir_and_appends_argv() {
        assert!(
            iron_kernel_accepts_aot_dir("vector_add"),
            "vector_add child documents --aot-dir"
        );
        assert!(
            !iron_kernel_accepts_aot_dir("relu"),
            "relu does not document --aot-dir"
        );
        let tmp = std::env::temp_dir().join(format!(
            "dragon_npu_aot_argv_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(3, |d| d.as_nanos())
        ));
        let example = tmp.join("programming_examples/basic/vector_scalar_add");
        std::fs::create_dir_all(&example).expect("example dir");
        let aot = tmp.join("this_run");
        std::fs::create_dir_all(&aot).expect("aot dir");
        let (_cwd, argv) =
            iron_kernel_cwd_and_argv_with_aot("vector_add", &tmp, Some(&aot)).expect("argv");
        let relu_argv = iron_kernel_cwd_and_argv_with_aot("relu", &tmp, Some(&aot))
            .expect_err("relu dir missing");
        let _ = relu_argv;
        assert!(
            argv.windows(2)
                .any(|w| w[0] == "--aot-dir" && w[1] == aot.to_string_lossy()),
            "vector_add argv must pass --aot-dir this-run: {argv:?}"
        );
        let relu_example = tmp.join("programming_examples/ml/eltwise_unary");
        std::fs::create_dir_all(&relu_example).expect("relu example");
        let (_cwd, relu_argv) =
            iron_kernel_cwd_and_argv_with_aot("relu", &tmp, Some(&aot)).expect("relu argv");
        let _ = std::fs::remove_dir_all(&tmp);
        assert!(
            !relu_argv.iter().any(|a| a == "--aot-dir"),
            "relu must not get --aot-dir: {relu_argv:?}"
        );
    }

    #[test]
    fn iron_kernel_vector_add_default_wrap_has_no_aot_dir() {
        let spec = iron_kernel_spec("vector_add").expect("vector_add");
        assert_eq!(spec.argv, ["vector_scalar_add.py", "--dev", "npu2"]);
        assert!(
            !spec.argv.contains(&"--aot-dir"),
            "default product argv is the run wrap, not export-only"
        );
        let tmp = std::env::temp_dir().join(format!(
            "dragon_npu_default_wrap_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(5, |d| d.as_nanos())
        ));
        let example = tmp.join("programming_examples/basic/vector_scalar_add");
        std::fs::create_dir_all(&example).expect("example dir");
        let (_cwd, argv) =
            iron_kernel_cwd_and_argv_with_aot("vector_add", &tmp, None).expect("default wrap");
        let _ = std::fs::remove_dir_all(&tmp);
        let expected: Vec<String> =
            vec!["vector_scalar_add.py".into(), "--dev".into(), "npu2".into()];
        assert_eq!(argv, expected);
        assert!(
            !argv.iter().any(|a| a == "--aot-dir"),
            "None aot dir must not append --aot-dir: {argv:?}"
        );
        assert!(
            !iron_kernel_should_create_this_run_aot_dir("vector_add", false),
            "default wrap must not create a this-run aot dir"
        );
        assert!(
            iron_kernel_should_create_this_run_aot_dir("vector_add", true),
            "explicit export may create a this-run aot dir"
        );
        assert!(
            !iron_kernel_should_create_this_run_aot_dir("gelu", true),
            "only vector_add accepts --aot-dir"
        );
    }

    #[test]
    fn only_vector_add_accepts_aot_dir_across_catalog() {
        let tmp = std::env::temp_dir().join(format!(
            "dragon_npu_catalog_aot_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(6, |d| d.as_nanos())
        ));
        for name in IRON_KERNEL_NAMES {
            assert_eq!(
                iron_kernel_accepts_aot_dir(name),
                *name == "vector_add",
                "{name}"
            );
            let spec = iron_kernel_spec(name).expect("catalog name");
            let example = tmp.join(spec.cwd_suffix);
            std::fs::create_dir_all(&example).expect("example");
            let aot = tmp.join("aot");
            std::fs::create_dir_all(&aot).expect("aot");
            let (_cwd, argv) =
                iron_kernel_cwd_and_argv_with_aot(name, &tmp, Some(aot.as_path())).expect("argv");
            let has_aot = argv.iter().any(|a| a == "--aot-dir");
            assert_eq!(has_aot, *name == "vector_add", "{name} argv={argv:?}");
        }
        let gelu_aot = tmp.join("gelu-aot");
        std::fs::create_dir_all(&gelu_aot).expect("gelu aot");
        let (_cwd, gelu_argv) =
            iron_kernel_cwd_and_argv_with_aot("gelu", &tmp, Some(gelu_aot.as_path()))
                .expect("gelu+aot");
        let _ = std::fs::remove_dir_all(&tmp);
        assert!(
            !gelu_argv.iter().any(|a| a == "--aot-dir"),
            "gelu + Some(aot) must not append --aot-dir: {gelu_argv:?}"
        );
    }

    #[test]
    fn missing_aot_dir_and_empty_npy_are_none_and_empty_listing() {
        let missing = std::env::temp_dir().join(format!(
            "dragon_npu_aot_missing_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(7, |d| d.as_nanos())
        ));
        let _ = std::fs::remove_dir_all(&missing);
        assert!(
            load_this_run_npy_from_aot_dir(&missing).is_none(),
            "missing aot dir must load None"
        );
        assert!(
            list_this_run_aot_artifacts(&missing).is_empty(),
            "missing aot dir must list empty"
        );

        let tmp = std::env::temp_dir().join(format!(
            "dragon_npu_aot_empty_npy_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(8, |d| d.as_nanos())
        ));
        std::fs::create_dir_all(&tmp).expect("aot");
        std::fs::write(tmp.join("output.npy"), []).expect("empty npy");
        let empty_bytes = load_this_run_npy_from_aot_dir(&tmp);
        let empty_arts = list_this_run_aot_artifacts(&tmp);
        assert!(empty_bytes.is_none(), "empty npy must load None");
        assert!(
            empty_arts.is_empty(),
            "empty npy must not appear in this-run listing: {empty_arts:?}"
        );
        std::fs::write(tmp.join("out.npy"), tiny_npy_f32(&[3.0, 4.0])).expect("valid out.npy");
        let loaded = load_this_run_npy_from_aot_dir(&tmp);
        let listed = list_this_run_aot_artifacts(&tmp);
        let parsed = parse_child_f32_vector("PASS!\n", loaded.as_deref());
        let _ = std::fs::remove_dir_all(&tmp);
        assert_eq!(
            parsed,
            vec![3.0, 4.0],
            "valid out.npy after empty output.npy"
        );
        assert!(
            listed
                .iter()
                .any(|p| p.file_name().is_some_and(|n| n == "out.npy")),
            "listing must include non-empty out.npy: {listed:?}"
        );
    }

    #[test]
    fn remove_this_run_aot_dir_best_effort_deletes_dir() {
        let dir = create_this_run_aot_dir("vector_add").expect("create");
        assert!(dir.is_dir());
        remove_this_run_aot_dir_best_effort(&dir);
        assert!(!dir.exists());
    }

    #[test]
    fn iron_npu2_vector_add_job_aliases_share_one_allow_list() {
        for alias in [
            IRON_NPU2_VECTOR_ADD_JOB,
            "IRON_NPU2_VECTOR_ADD",
            "iron-npu2-vector-add",
            "Iron-Npu2-Vector-Add",
        ] {
            assert!(
                is_iron_npu2_vector_add_job_name(alias),
                "allow-list miss: {alias}"
            );
            assert!(
                refuse_classic_1688_residual_claim_for_job(alias),
                "refuse miss: {alias}"
            );
        }
        assert!(!is_iron_npu2_vector_add_job_name(""));
        assert!(!is_iron_npu2_vector_add_job_name("   "));
        assert!(!refuse_classic_1688_residual_claim_for_job(""));
        assert!(!refuse_classic_1688_residual_claim_for_job("   "));
        assert!(!is_iron_npu2_vector_add_job_name(MLADF_CLASSIC_1688_JOB));
        assert!(!refuse_classic_1688_residual_claim_for_job(
            MLADF_CLASSIC_1688_JOB
        ));
    }

    #[test]
    fn load_this_run_npy_from_aot_dir_does_not_need_basename_mention() {
        let tmp = std::env::temp_dir().join(format!(
            "dragon_npu_aot_npy_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(4, |d| d.as_nanos())
        ));
        std::fs::create_dir_all(&tmp).expect("aot");
        let npy = tiny_npy_f32(&[1.5, 2.5]);
        std::fs::write(tmp.join("output.npy"), npy).expect("write npy");
        let bytes = load_this_run_npy_from_aot_dir(&tmp);
        let v = parse_child_f32_vector("PASS!\n", bytes.as_deref());
        let arts = list_this_run_aot_artifacts(&tmp);
        let _ = std::fs::remove_dir_all(&tmp);
        assert_eq!(
            v,
            vec![1.5, 2.5],
            "this-run aot npy is exclusive, not leftover cwd"
        );
        assert!(
            arts.iter()
                .any(|p| p.file_name().is_some_and(|n| n == "output.npy")),
            "aot listing must include output.npy: {arts:?}"
        );
    }

    #[test]
    fn iron_job_collect_note_refuses_classic_1688_claim() {
        let note = format_iron_job_collect_note("iron_npu2_vector_add", 3, false);
        assert!(
            note.contains("job=iron_npu2_vector_add"),
            "collect note must name the IRON job: {note}"
        );
        assert!(
            note.contains("residual_matched_claim=0"),
            "must never claim 1688 residual match: {note}"
        );
        assert!(
            note.contains("invent_pass=0"),
            "honesty invent_pass=0: {note}"
        );
        assert!(
            note.contains("not_ran_on_npu=1"),
            "honesty not_ran_on_npu=1: {note}"
        );
        assert!(
            refuse_classic_1688_residual_claim_for_job("iron_npu2_vector_add"),
            "IRON job must refuse 1688 residual_matched_claim"
        );
        assert!(
            !refuse_classic_1688_residual_claim_for_job("mladf_classic_1688"),
            "classic job is the 1688 identity (still claim=0 offline)"
        );
        let claimed = format_iron_job_collect_note("iron_npu2_vector_add", 1, true);
        assert_collect_note_honesty(&claimed);
        let empty = format_iron_job_collect_note("   ", 0, true);
        assert_collect_note_honesty(&empty);
    }

    #[test]
    fn iron_job_collect_note_true_claim_arg_still_emits_residual_matched_claim_0() {
        let claimed = format_iron_job_collect_note("iron_npu2_vector_add", 0, true);
        assert_collect_note_honesty(&claimed);
        let empty = format_iron_job_collect_note("  ", 0, true);
        assert_collect_note_honesty(&empty);
    }

    fn collect_note_has_exact_token(note: &str, token: &str) -> bool {
        note.split_whitespace().any(|t| t == token)
    }

    fn assert_collect_note_honesty(note: &str) {
        for tok in [
            "residual_matched_claim=0",
            "invent_pass=0",
            "not_ran_on_npu=1",
            "not_c_ready=1",
            "classic_1688_claim_refused=1",
            "product_known_good=0",
            "collect_neq_product_pass=1",
        ] {
            assert!(
                collect_note_has_exact_token(note, tok),
                "missing exact token {tok}: {note}"
            );
        }
        assert!(
            !collect_note_has_exact_token(note, "invent_pass=1"),
            "must not emit invent_pass=1: {note}"
        );
        assert!(
            !collect_note_has_exact_token(note, "ran_on_npu=1"),
            "must not emit ran_on_npu=1: {note}"
        );
        assert!(
            !collect_note_has_exact_token(note, "residual_matched_claim=1"),
            "must not emit residual_matched_claim=1: {note}"
        );
    }

    fn tiny_npy_f32(vals: &[f32]) -> Vec<u8> {
        let header = format!(
            "{{'descr': '<f4', 'fortran_order': False, 'shape': ({},), }}",
            vals.len()
        );
        let mut header_bytes = header.into_bytes();
        header_bytes.push(b'\n');
        // npy v1: magic + version + u16 header_len + header + data
        let mut out = b"\x93NUMPY\x01\x00".to_vec();
        let hlen = u16::try_from(header_bytes.len()).expect("tiny header");
        out.extend_from_slice(&hlen.to_le_bytes());
        out.extend_from_slice(&header_bytes);
        for v in vals {
            out.extend_from_slice(&v.to_le_bytes());
        }
        out
    }
}
