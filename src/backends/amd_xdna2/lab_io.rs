//! Phase 3.5 lab OFM / IFM / WTS helpers (pure packing + optional BO plan).
//!
//! **No default EXEC.** Pure functions need no device. BO lifecycle helpers use
//! [`super::direct::DirectSubmission`] and never claim silicon GEMM Pass alone.
//!
//! **Kernel-arg layout (PARTIAL_ELF START_NPU):** XRT shim_test sets
//! `instruction_prop_count = 0` and packs IFM/WTS/OFM as **kernel args** in the
//! flexible `prop_args[]` array (not as properties). See
//! `docs/XDNA2_TXN_RESEARCH.md` and research on `elf_io_test_bo_set::init_cmd`.
//!
//! **Env:** `XDNA2_LAB_OFM=1` enables optional OFM create/bind/readback in
//! [`super::direct::timed_matmul_direct`]. Default off → hard-false `ran_on_npu`.
//!
//! **Pack mode:** `XDNA2_LAB_PACK=f32_ones` (default) or `mladf_a16w4` for AMD
//! mladf buffer sizes (bf16 IFM / packed WTS). See `docs/AMD_MLADF_MATMUL_PACK.md`.

use super::direct::{
    BufferHandle, BufferType, DirectError, DirectSubmission, HOST_REF_MAX_ELEMENTS,
    SYNC_BO_FROM_DEVICE, is_valid_xdna_device_addr,
};
use super::mladf_pack::{
    LabPackMode, MladfBufferPlan, build_mladf_lab_payloads, load_wts_packed_from_env,
    mladf_buffer_plan_from_env, read_lab_file_capped, unpack_ofm_bf16_to_f32,
    validate_wts_packed_len_nonzero, wts_bytes_are_all_zero,
};
use super::numerical::{
    NumericalVerifyCriteria, NumericalVerifyInput, RanOnNpuBlockReason, decide_ran_on_npu_reason,
    host_matmul_f32_ones_vec, max_abs_diff_f32,
};

// ---------------------------------------------------------------------------
// Pure f32 LE encode / decode
// ---------------------------------------------------------------------------

/// Decode little-endian f32 values from a byte slice.
///
/// Empty input → empty vector. Length must be a multiple of 4.
///
/// # Errors
///
/// [`DirectError::InvalidArgument`] when `bytes.len() % 4 != 0`.
pub fn f32_slice_from_le_bytes(bytes: &[u8]) -> Result<Vec<f32>, DirectError> {
    if !bytes.len().is_multiple_of(4) {
        return Err(DirectError::InvalidArgument(format!(
            "f32_slice_from_le_bytes: length {} is not a multiple of 4",
            bytes.len()
        )));
    }
    let n = bytes.len() / 4;
    let mut out = Vec::with_capacity(n);
    for chunk in bytes.as_chunks::<4>().0 {
        out.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
    }
    Ok(out)
}

/// Encode f32 values as little-endian bytes.
#[must_use]
pub fn f32_slice_to_le_bytes(vals: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(vals.len().saturating_mul(4));
    for v in vals {
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

// ---------------------------------------------------------------------------
// Lab matmul buffer sizes (row-major f32 A[m,k], B[k,n], C[m,n])
// ---------------------------------------------------------------------------

/// Bytes for row-major f32 IFM `A[m,k]` (input feature map / activation).
///
/// Layout: element `(i,j)` at offset `(i*k + j) * 4`.
///
/// # Errors
///
/// Zero dims, unsupported dtype, overflow, or exceeds [`HOST_REF_MAX_ELEMENTS`].
pub fn lab_matmul_ifm_nbytes(m: u32, k: u32, dtype: &str) -> Result<u64, DirectError> {
    lab_f32_matrix_nbytes(m, k, dtype, "IFM")
}

/// Bytes for row-major f32 WTS `B[k,n]` (weights / parameters).
///
/// Layout: element `(i,j)` at offset `(i*n + j) * 4`.
///
/// # Errors
///
/// See [`lab_matmul_ifm_nbytes`].
pub fn lab_matmul_wts_nbytes(k: u32, n: u32, dtype: &str) -> Result<u64, DirectError> {
    lab_f32_matrix_nbytes(k, n, dtype, "WTS")
}

/// Bytes for row-major f32 OFM `C[m,n]` (output feature map).
///
/// Layout: element `(i,j)` at offset `(i*n + j) * 4`.
///
/// # Errors
///
/// See [`lab_matmul_ifm_nbytes`].
pub fn lab_matmul_ofm_nbytes(m: u32, n: u32, dtype: &str) -> Result<u64, DirectError> {
    lab_f32_matrix_nbytes(m, n, dtype, "OFM")
}

fn lab_f32_matrix_nbytes(
    rows: u32,
    cols: u32,
    dtype: &str,
    role: &str,
) -> Result<u64, DirectError> {
    if rows == 0 || cols == 0 {
        return Err(DirectError::InvalidArgument(format!(
            "lab_matmul_{role}_nbytes requires positive dims (rows={rows} cols={cols})"
        )));
    }
    let dt = dtype.trim().to_ascii_lowercase();
    if dt != "f32" && dt != "float32" {
        return Err(DirectError::InvalidArgument(format!(
            "lab_matmul_{role}_nbytes supports f32 only (got {dtype:?})"
        )));
    }
    let elems = u64::from(rows)
        .checked_mul(u64::from(cols))
        .ok_or_else(|| {
            DirectError::InvalidArgument(format!(
                "lab_matmul_{role}_nbytes element product overflow ({rows}x{cols})"
            ))
        })?;
    if elems > HOST_REF_MAX_ELEMENTS {
        return Err(DirectError::InvalidArgument(format!(
            "lab_matmul_{role}_nbytes refuses {elems} elems > max {HOST_REF_MAX_ELEMENTS}"
        )));
    }
    elems.checked_mul(4).ok_or_else(|| {
        DirectError::InvalidArgument(format!(
            "lab_matmul_{role}_nbytes byte size overflow ({elems}*4)"
        ))
    })
}

/// Host-side all-ones f32 matrix as LE bytes (`rows * cols` ones).
///
/// Matches the all-ones IFM/WTS contract used by
/// [`host_matmul_f32_ones_vec`] (`C` elements = `k` when both are ones).
///
/// # Errors
///
/// Zero dims or product overflow / host-ref cap.
pub fn host_ones_f32_matrix_bytes(rows: u32, cols: u32) -> Result<Vec<u8>, DirectError> {
    let nbytes = lab_f32_matrix_nbytes(rows, cols, "f32", "ones")?;
    let n = usize::try_from(nbytes / 4).map_err(|_| {
        DirectError::InvalidArgument("host_ones_f32_matrix_bytes length exceeds usize".into())
    })?;
    Ok(f32_slice_to_le_bytes(&vec![1.0_f32; n]))
}

// ---------------------------------------------------------------------------
// Kernel-arg packing (prop_count must stay 0 on the Cmd BO envelope)
// ---------------------------------------------------------------------------

/// Split a device address into LE low/high dwords for START_NPU regmap slots.
#[must_use]
pub fn u64_to_prop_dwords(addr: u64) -> [u32; 2] {
    [(addr & 0xffff_ffff) as u32, (addr >> 32) as u32]
}

/// Pack PARTIAL_ELF START_NPU **kernel args** (shim_test `elf_io_test_bo_set::init_cmd`).
///
/// Layout (dwords, little-endian pairs for u64):
/// ```text
/// u64  num_data_bos   // 3 without ctrl, 4 with ctrl
/// u64  0
/// u32  0
/// u64  IFM.paddr
/// u64  WTS.paddr
/// u64  OFM.paddr
/// u64  CTRL.paddr or 0
/// u64  0
/// ```
///
/// **Critical:** these dwords go in `prop_args[]` with **`prop_count = 0`**
/// (they are kernel args, not properties). Use
/// [`super::direct::build_ert_start_npu_cmd_bo_with_kernel_args`].
///
/// This layout is the XRT partial-elf convention for npu4; it is **not** proven
/// for every model. Wrong paddrs / blob still fail closed via numerical decide.
#[must_use]
pub fn pack_start_npu_kernel_args(ifm_paddr: u64, wts_paddr: u64, ofm_paddr: u64) -> Vec<u32> {
    pack_start_npu_kernel_args_with_ctrl(ifm_paddr, wts_paddr, ofm_paddr, None)
}

/// Kernel-arg buffer count / layout for START_NPU PARTIAL_ELF.
///
/// Live 2026-07-16: `ert_state=Error` with count-4; XRT shim_test uses **3**
/// (IFM/WTS/OFM + two trailing zeros). Count-4 with ctrl as 4th paddr is the
/// mladf meta `xrt_id` hypothesis — still unproven on silicon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LabKernelArgLayout {
    /// `n_bos=4`, slots IFM/WTS/OFM/CTRL, one trailing u64 zero (historical lab).
    #[default]
    Count4Ctrl,
    /// XRT `elf_io_test` classic: `n_bos=3`, IFM/WTS/OFM, two trailing u64 zeros.
    /// Control packet is still patched + pinned via `arg_bo_handles` when present.
    Xrt3,
}

impl LabKernelArgLayout {
    /// Stable token for detail / batch diag.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Count4Ctrl => "count4",
            Self::Xrt3 => "xrt3",
        }
    }
}

/// Fail-closed parse of `XDNA2_LAB_KERNEL_ARG_LAYOUT`.
///
/// Unset / empty → [`LabKernelArgLayout::Count4Ctrl`]. Known: `count4`/`4`,
/// `xrt3`/`3`/`classic`. Unknown → `Err` (typo must not burn HITL trial).
///
/// # Errors
///
/// Unrecognized non-empty token.
pub fn try_lab_kernel_arg_layout_from_str(raw: Option<&str>) -> Result<LabKernelArgLayout, String> {
    raw.map(str::trim)
        .filter(|s| !s.is_empty())
        .map_or(Ok(LabKernelArgLayout::Count4Ctrl), |s| {
            match s.to_ascii_lowercase().as_str() {
                "count4" | "4" => Ok(LabKernelArgLayout::Count4Ctrl),
                "xrt3" | "3" | "classic" => Ok(LabKernelArgLayout::Xrt3),
                other => Err(format!(
                    "unknown XDNA2_LAB_KERNEL_ARG_LAYOUT={other:?}; expected count4|xrt3 \
                     (aliases 4→count4; 3|classic→xrt3)"
                )),
            }
        })
}

/// Convenience for known-good / None; unknown falls back to count4 (prefer try).
#[must_use]
pub fn lab_kernel_arg_layout_from_str(raw: Option<&str>) -> LabKernelArgLayout {
    try_lab_kernel_arg_layout_from_str(raw).unwrap_or(LabKernelArgLayout::Count4Ctrl)
}

/// `XDNA2_LAB_KERNEL_ARG_LAYOUT=xrt3|count4` (default **count4**; fail-closed).
///
/// # Errors
///
/// Unknown token set in env.
pub fn lab_kernel_arg_layout_from_env() -> Result<LabKernelArgLayout, String> {
    try_lab_kernel_arg_layout_from_str(std::env::var("XDNA2_LAB_KERNEL_ARG_LAYOUT").ok().as_deref())
}

/// Like [`pack_start_npu_kernel_args`] with optional **control packet** paddr.
///
/// Layout from env when valid; unknown env is treated as default count4 here
/// (timed path refuses invalid env **before** EXEC via
/// [`lab_kernel_arg_layout_from_env`]). Prefer
/// [`pack_start_npu_kernel_args_with_ctrl_layout`] when layout is known.
#[must_use]
pub fn pack_start_npu_kernel_args_with_ctrl(
    ifm_paddr: u64,
    wts_paddr: u64,
    ofm_paddr: u64,
    ctrl_paddr: Option<u64>,
) -> Vec<u32> {
    let layout = lab_kernel_arg_layout_from_env().unwrap_or(LabKernelArgLayout::Count4Ctrl);
    pack_start_npu_kernel_args_with_ctrl_layout(ifm_paddr, wts_paddr, ofm_paddr, ctrl_paddr, layout)
}

/// Explicit layout variant of [`pack_start_npu_kernel_args_with_ctrl`].
#[must_use]
pub fn pack_start_npu_kernel_args_with_ctrl_layout(
    ifm_paddr: u64,
    wts_paddr: u64,
    ofm_paddr: u64,
    ctrl_paddr: Option<u64>,
    layout: LabKernelArgLayout,
) -> Vec<u32> {
    let mut out = Vec::with_capacity(15);
    match layout {
        LabKernelArgLayout::Xrt3 => {
            // XRT elf_io_test_bo_set::init_cmd (START_NPU PARTIAL_ELF):
            //   u64 3, u64 0, u32 0, ifm, wts, ofm, u64 0, u64 0
            out.extend_from_slice(&u64_to_prop_dwords(3));
            out.extend_from_slice(&u64_to_prop_dwords(0));
            out.push(0);
            out.extend_from_slice(&u64_to_prop_dwords(ifm_paddr));
            out.extend_from_slice(&u64_to_prop_dwords(wts_paddr));
            out.extend_from_slice(&u64_to_prop_dwords(ofm_paddr));
            out.extend_from_slice(&u64_to_prop_dwords(0));
            out.extend_from_slice(&u64_to_prop_dwords(0));
            let _ = ctrl_paddr; // pin-only; not in classic exec-buf args
        }
        LabKernelArgLayout::Count4Ctrl => {
            let n_bos: u64 = if ctrl_paddr.is_some() { 4 } else { 3 };
            out.extend_from_slice(&u64_to_prop_dwords(n_bos));
            out.extend_from_slice(&u64_to_prop_dwords(0));
            out.push(0);
            out.extend_from_slice(&u64_to_prop_dwords(ifm_paddr));
            out.extend_from_slice(&u64_to_prop_dwords(wts_paddr));
            out.extend_from_slice(&u64_to_prop_dwords(ofm_paddr));
            out.extend_from_slice(&u64_to_prop_dwords(ctrl_paddr.unwrap_or(0)));
            out.extend_from_slice(&u64_to_prop_dwords(0));
        }
    }
    out
}

/// No-I/O kernel args (shim `elf_init_no_arg_cmd` style): marker + zeros.
#[must_use]
pub fn pack_start_npu_kernel_args_no_io() -> Vec<u32> {
    pack_start_npu_kernel_args(0, 0, 0)
}

/// True if every byte is zero (length-only zero WTS / zero golden detection).
#[must_use]
pub fn bytes_are_all_zero(bytes: &[u8]) -> bool {
    wts_bytes_are_all_zero(bytes)
}

/// Refuse all-zero WTS for a live mladf EXEC attempt (length-only pack is not Pass).
///
/// Precedence (fail-closed for Pass readiness):
/// - Non-zero payload → always Ok.
/// - `XDNA2_LAB_REQUIRE_NONZERO_WTS=1` → always refuse zeros (**wins over ALLOW**).
/// - Else `XDNA2_LAB_ALLOW_ZERO_WTS=1` → allow zeros (plumbing only).
/// - Else refuse zeros (soft path for fill without REQUIRE still ignores this Err
///   unless REQUIRE is set — see [`create_and_fill_mladf_io_bos_ex`]).
///
/// Prefer a real pack from `tools/export_mladf_wts.sh` + `XDNA2_LAB_WTS_PACKED`.
///
/// # Errors
///
/// All-zero WTS when REQUIRE is set, or when neither REQUIRE nor ALLOW allows it.
pub fn check_mladf_wts_not_all_zero_for_exec(wts: &[u8]) -> Result<(), DirectError> {
    if !bytes_are_all_zero(wts) {
        return Ok(());
    }
    // Pass readiness always wins over plumbing ALLOW.
    if lab_require_non_zero_wts_env() {
        return Err(DirectError::InvalidArgument(
            "mladf WTS is all zeros — XDNA2_LAB_REQUIRE_NONZERO_WTS=1 refuses zero packs \
             (ALLOW_ZERO_WTS does not override REQUIRE; set XDNA2_LAB_WTS_PACKED from \
             tools/export_mladf_wts.sh)"
                .into(),
        ));
    }
    if lab_ofm_env_value_enabled(std::env::var("XDNA2_LAB_ALLOW_ZERO_WTS").ok().as_deref()) {
        return Ok(());
    }
    Err(DirectError::InvalidArgument(
        "mladf WTS is all zeros — not AMD-packed weights; refuse EXEC Pass path \
         (set XDNA2_LAB_WTS_PACKED from tools/export_mladf_wts.sh, or \
         XDNA2_LAB_ALLOW_ZERO_WTS=1 for plumbing only when REQUIRE_NONZERO_WTS is unset)"
            .into(),
    ))
}

/// Pure gate: packed WTS is Pass-candidate only if length matches and not all zeros.
///
/// Does **not** verify AMD shuffle correctness (needs vendor `export_const_params` /
/// `tools/export_mladf_wts.sh` or capture). Length + non-zero is the offline gate
/// before EXEC. Shell twin: `tools/validate_mladf_wts.sh`.
///
/// # Errors
///
/// Wrong length or all-zero payload.
pub fn validate_mladf_wts_packed_for_pass(
    wts: &[u8],
    expected_len: u64,
) -> Result<(), DirectError> {
    validate_wts_packed_len_nonzero(wts, expected_len)
}

/// Pure gate: control packet is Pass-candidate only if length matches and not empty/all-zero.
///
/// Staged key expects [`crate::backends::amd_xdna2::mladf_pack::STAGED_MLADF_CTRL_BYTES`]
/// (29664). Does **not** verify control-packet semantics, paddr patch correctness, or
/// silicon Pass. Shell twin: `tools/validate_mladf_ctrl.sh`.
///
/// # Errors
///
/// Wrong length, empty, or all-zero payload.
pub fn validate_mladf_ctrl_packed_for_pass(
    ctrl: &[u8],
    expected_len: u64,
) -> Result<(), DirectError> {
    let got = ctrl.len() as u64;
    if got != expected_len {
        return Err(DirectError::InvalidArgument(format!(
            "mladf CTRL len {got} != expected {expected_len} (staged a16w4 is 29664)"
        )));
    }
    if ctrl.is_empty() {
        return Err(DirectError::InvalidArgument("mladf CTRL is empty".into()));
    }
    if bytes_are_all_zero(ctrl) {
        return Err(DirectError::InvalidArgument(
            "mladf CTRL is all zeros — refuse Pass readiness (stage via \
             tools/stage_silicon_lab_assets.sh / dump_dd_txn sibling *_ctrl.bin)"
                .into(),
        ));
    }
    Ok(())
}

/// Pure gate: OFM golden file bytes are Pass-candidate for mladf a16w4.
///
/// Accepts **bf16 LE** (`m*n*2`) or **f32 LE** (`m*n*4`). Staged key is
/// `m=1,n=256` → **512** bf16 or **1024** f32. Refuses wrong size, empty, and
/// all-zero decoded values (unless `allow_zero` — lab escape hatch only).
///
/// Does **not** claim silicon Pass. Shell twin: `tools/validate_mladf_ofm_golden.sh`.
///
/// # Errors
///
/// Parse/size failure or all-zero golden without allow.
pub fn validate_mladf_ofm_golden_packed_for_pass(
    golden_bytes: &[u8],
    m: u32,
    n: u32,
    allow_zero: bool,
) -> Result<(), DirectError> {
    let golden = parse_ofm_golden_bytes(golden_bytes, m, n)?;
    check_mladf_ofm_golden_not_all_zero(LabPackMode::MladfA16W4, &golden, allow_zero)?;
    Ok(())
}

/// Pure offline readiness checklist for silicon-path Pass **assets** (no device).
///
/// All four must be true before an operator attempts gated EXEC for
/// `ran_on_npu`. Does **not** open L3, does **not** EXEC, does **not** set
/// `ran_on_npu`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // honesty checklist fields, not a state machine
pub struct SiliconPassAssetReadiness {
    /// Instruction / txn blob present and non-empty (size checked by caller).
    pub instr_nonempty: bool,
    /// WTS length+nonzero gate ([`validate_mladf_wts_packed_for_pass`]).
    pub wts_pass_candidate: bool,
    /// CTRL length+nonzero gate ([`validate_mladf_ctrl_packed_for_pass`]).
    pub ctrl_pass_candidate: bool,
    /// OFM golden non-zero + size ([`validate_mladf_ofm_golden_packed_for_pass`]).
    pub ofm_golden_pass_candidate: bool,
}

impl SiliconPassAssetReadiness {
    /// True when every offline Pass asset gate is green.
    #[must_use]
    pub const fn all_ready(self) -> bool {
        self.instr_nonempty
            && self.wts_pass_candidate
            && self.ctrl_pass_candidate
            && self.ofm_golden_pass_candidate
    }
}

/// Aggregate pure asset gates (caller supplies already-validated bools).
///
/// Prefer validating each blob with the `validate_mladf_*_packed_for_pass`
/// helpers, then pass the `Ok` outcomes here for a single readiness object.
#[must_use]
#[allow(clippy::fn_params_excessive_bools)] // honesty gates mirror offline checklist
pub const fn silicon_pass_asset_readiness(
    instr_nonempty: bool,
    wts_pass_candidate: bool,
    ctrl_pass_candidate: bool,
    ofm_golden_pass_candidate: bool,
) -> SiliconPassAssetReadiness {
    SiliconPassAssetReadiness {
        instr_nonempty,
        wts_pass_candidate,
        ctrl_pass_candidate,
        ofm_golden_pass_candidate,
    }
}

// ---------------------------------------------------------------------------
// Lab env + decide from OFM bytes
// ---------------------------------------------------------------------------

/// Parse `XDNA2_LAB_OFM` style values: `1` / `true` (case-insensitive) → true.
///
/// Empty / `None` / `0` / other → **false**.
#[must_use]
pub fn lab_ofm_env_value_enabled(raw: Option<&str>) -> bool {
    raw.is_some_and(|v| {
        let t = v.trim();
        t == "1" || t.eq_ignore_ascii_case("true")
    })
}

/// True only when `XDNA2_LAB_OFM` is `1` or `true` (case-insensitive).
///
/// Default / unset / `0` / other → **false** (product path: no OFM, hard-false
/// `ran_on_npu`).
#[must_use]
pub fn lab_ofm_env_enabled() -> bool {
    lab_ofm_env_value_enabled(std::env::var("XDNA2_LAB_OFM").ok().as_deref())
}

/// True when `XDNA2_LAB_WEIGHT=1` / `true` (model WTS bind; requires LAB_OFM).
///
/// Default off. When set with LAB_OFM, timed path loads `XDNA2_LAB_WEIGHT_MODEL`
/// and fails closed if model/select/dims fail. Still never claims Pass without
/// known-good txn + OFM match.
#[must_use]
pub fn lab_weight_env_enabled() -> bool {
    lab_ofm_env_value_enabled(std::env::var("XDNA2_LAB_WEIGHT").ok().as_deref())
}

/// Host golden for ones IFM `A[m,k]` × WTS `B[k,n]` (row-major f32).
///
/// `wts_f32` must be `k * n` elements in row-major `[K,N]`.
///
/// # Errors
///
/// Zero dims, length mismatch, overflow, or host-ref cap.
pub fn host_matmul_f32_ones_ifm_with_wts(
    m: u32,
    n: u32,
    k: u32,
    wts_f32: &[f32],
) -> Result<Vec<f32>, DirectError> {
    if m == 0 || n == 0 || k == 0 {
        return Err(DirectError::InvalidArgument(
            "host_matmul_f32_ones_ifm_with_wts requires positive m,n,k".into(),
        ));
    }
    let need = (k as usize)
        .checked_mul(n as usize)
        .ok_or_else(|| DirectError::InvalidArgument("k*n overflow".into()))?;
    if wts_f32.len() != need {
        return Err(DirectError::InvalidArgument(format!(
            "host_matmul_f32_ones_ifm_with_wts: wts len {} != k*n={}",
            wts_f32.len(),
            need
        )));
    }
    let out_elems = (m as usize)
        .checked_mul(n as usize)
        .ok_or_else(|| DirectError::InvalidArgument("m*n overflow".into()))?;
    if out_elems as u64 > HOST_REF_MAX_ELEMENTS {
        return Err(DirectError::InvalidArgument(format!(
            "host_matmul_f32_ones_ifm_with_wts: m*n={out_elems} > max {HOST_REF_MAX_ELEMENTS}"
        )));
    }
    // A = ones → C[i,j] = sum_t B[t,j]
    let mut col_sums = vec![0.0_f32; n as usize];
    for t in 0..k as usize {
        for j in 0..n as usize {
            col_sums[j] += wts_f32[t * n as usize + j];
        }
    }
    let mut out = Vec::with_capacity(out_elems);
    for _ in 0..m {
        out.extend_from_slice(&col_sums);
    }
    Ok(out)
}

/// Inputs for pure lab decide from raw OFM bytes (ones-matmul golden).
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)] // honesty flags are intentional
pub struct LabOfmVerifyPlan {
    /// Matmul M.
    pub m: u32,
    /// Matmul N.
    pub n: u32,
    /// Matmul K (inner; golden elements = k).
    pub k: u32,
    /// Non-empty instruction blob was bound.
    pub used_inst_blob: bool,
    /// Gated EXEC + WAIT succeeded.
    pub submit_wait_ok: bool,
    /// Real ioctl path (not stub).
    pub real_ioctl: bool,
    /// Device OFM readback bytes (LE f32 expected for ones path).
    pub ofm_bytes: Vec<u8>,
    /// Allclose tolerances.
    pub criteria: NumericalVerifyCriteria,
}

/// Decode OFM bytes, build ones golden, run [`decide_ran_on_npu`].
///
/// **f32 ones path only.** For `XDNA2_LAB_PACK=mladf_a16w4`, use
/// [`lab_decide_from_ofm_bytes_pack_mode`] — f32 ones×ones golden is **invalid**
/// for the a16w4 kernel.
///
/// Returns `(ran_on_npu, optional block reason)`. Decode errors surface as
/// [`DirectError`]; policy refuses stay `Ok((false, Some(reason)))`.
///
/// # Errors
///
/// Golden build failure or non-multiple-of-4 OFM length.
pub fn lab_decide_from_ofm_bytes(
    plan: &LabOfmVerifyPlan,
) -> Result<(bool, Option<RanOnNpuBlockReason>), DirectError> {
    let golden = host_matmul_f32_ones_vec(plan.m, plan.n, plan.k)?;
    lab_decide_from_ofm_bytes_with_golden(plan, &golden)
}

/// Decode OFM bytes and compare against an explicit host golden (model WTS path).
///
/// Prefer this when WTS is **not** all-ones. Ones lab still uses
/// [`lab_decide_from_ofm_bytes`]. Never claims Pass without real ioctl + blob +
/// submit_wait + allclose (same policy as ones path).
///
/// Decodes OFM as **f32 LE** (ones / model-weight path). For bf16 OFM use
/// [`lab_decide_from_ofm_bytes_pack_mode`].
///
/// # Errors
///
/// - Golden length ≠ `m * n` (strict plan match)
/// - Non-multiple-of-4 OFM length (decode)
pub fn lab_decide_from_ofm_bytes_with_golden(
    plan: &LabOfmVerifyPlan,
    golden: &[f32],
) -> Result<(bool, Option<RanOnNpuBlockReason>), DirectError> {
    lab_decide_candidate_f32_with_golden(plan, &f32_slice_from_le_bytes(&plan.ofm_bytes)?, golden)
}

/// Compare an already-decoded f32 OFM candidate against golden (shared policy).
///
/// # Errors
///
/// Golden length ≠ `m * n`.
pub fn lab_decide_candidate_f32_with_golden(
    plan: &LabOfmVerifyPlan,
    candidate: &[f32],
    golden: &[f32],
) -> Result<(bool, Option<RanOnNpuBlockReason>), DirectError> {
    let need = u64::from(plan.m)
        .checked_mul(u64::from(plan.n))
        .ok_or_else(|| DirectError::InvalidArgument("lab_decide golden: m*n overflow".into()))?;
    if golden.len() as u64 != need {
        return Err(DirectError::InvalidArgument(format!(
            "lab_decide_from_ofm_bytes_with_golden: golden len {} != m*n={} (m={} n={})",
            golden.len(),
            need,
            plan.m,
            plan.n
        )));
    }
    // Candidate length mismatches are policy refuses (not Err) via
    // decide_ran_on_npu_reason — keep historical lab_decide_empty_ofm /
    // lab_decide_len_mismatch Ok((false, …)) behavior.
    let input = NumericalVerifyInput {
        host_golden: golden,
        device_or_candidate: candidate,
        had_instruction_blob: plan.used_inst_blob,
        submit_wait_ok: plan.submit_wait_ok,
        real_ioctl_path: plan.real_ioctl,
        criteria: plan.criteria,
    };
    match decide_ran_on_npu_reason(&input) {
        Ok(()) => Ok((true, None)),
        Err(r) => Ok((false, Some(r))),
    }
}

/// Whether a decide refuse reason should surface as `ofm_verify_failed` (content).
#[must_use]
pub fn ran_on_npu_reason_is_ofm_verify_fail(reason: Option<RanOnNpuBlockReason>) -> bool {
    matches!(
        reason,
        Some(RanOnNpuBlockReason::AllcloseFailed | RanOnNpuBlockReason::AllZeroGolden)
    )
}

/// Pure OFM dump-on-mismatch stats for timed-path `detail` (no secrets; truncated).
///
/// Used when decide refuses after submit_wait OK. Never claims Pass / `ran_on_npu`.
///
/// Reports: raw len, nonzero count (decoded f32 when possible), first few decoded
/// values, first few bf16 LE u16 when even-length raw, optional max_abs vs golden.
#[must_use]
pub fn format_ofm_mismatch_stats(
    ofm_bytes: &[u8],
    pack_mode: LabPackMode,
    golden: Option<&[f32]>,
) -> String {
    format_ofm_mismatch_stats_preview(ofm_bytes, pack_mode, golden, 8)
}

/// Same as [`format_ofm_mismatch_stats`] with explicit preview length (unit tests).
#[must_use]
pub fn format_ofm_mismatch_stats_preview(
    ofm_bytes: &[u8],
    pack_mode: LabPackMode,
    golden: Option<&[f32]>,
    preview_n: usize,
) -> String {
    let mut parts: Vec<String> = Vec::new();
    parts.push(format!("raw_len={}", ofm_bytes.len()));
    let nonzero_raw = ofm_bytes.iter().filter(|&&b| b != 0).count();
    parts.push(format!("raw_nonzero_bytes={nonzero_raw}"));

    // First + last few bf16 LE pairs as u16 (mladf OFM is 512 B bf16).
    if ofm_bytes.len() >= 2 && ofm_bytes.len().is_multiple_of(2) {
        let n_pairs = ofm_bytes.len() / 2;
        let n = preview_n.min(n_pairs);
        let mut first: Vec<String> = Vec::with_capacity(n);
        for i in 0..n {
            let off = i * 2;
            let u = u16::from_le_bytes([ofm_bytes[off], ofm_bytes[off + 1]]);
            first.push(format!("0x{u:04x}"));
        }
        parts.push(format!("first_bf16_le=[{}]", first.join(",")));
        if n_pairs > n {
            let mut last: Vec<String> = Vec::with_capacity(n);
            let start = n_pairs - n;
            for i in start..n_pairs {
                let off = i * 2;
                let u = u16::from_le_bytes([ofm_bytes[off], ofm_bytes[off + 1]]);
                last.push(format!("0x{u:04x}"));
            }
            parts.push(format!("last_bf16_le=[{}]", last.join(",")));
        } else if n_pairs > 0 {
            // Single window: still report last element for E1.2 symmetry.
            let off = (n_pairs - 1) * 2;
            let u = u16::from_le_bytes([ofm_bytes[off], ofm_bytes[off + 1]]);
            parts.push(format!("last_bf16_le=[0x{u:04x}]"));
        }
    }

    match lab_decode_ofm_bytes(pack_mode, ofm_bytes) {
        Ok(cand) => {
            let nz = cand.iter().filter(|&&x| x != 0.0).count();
            parts.push(format!("decoded_len={} nonzero_f32={nz}", cand.len()));
            let n = preview_n.min(cand.len());
            if n > 0 {
                let prev: Vec<String> = cand
                    .iter()
                    .take(n)
                    .map(|x| {
                        if x.is_finite() {
                            format!("{x:.6}")
                        } else {
                            format!("{x}")
                        }
                    })
                    .collect();
                parts.push(format!("first_f32=[{}]", prev.join(",")));
                if cand.len() > n {
                    let last: Vec<String> = cand
                        .iter()
                        .rev()
                        .take(n)
                        .collect::<Vec<_>>()
                        .into_iter()
                        .rev()
                        .map(|x| {
                            if x.is_finite() {
                                format!("{x:.6}")
                            } else {
                                format!("{x}")
                            }
                        })
                        .collect();
                    parts.push(format!("last_f32=[{}]", last.join(",")));
                }
            }
            if let Some(g) = golden {
                parts.push(format!("golden_len={}", g.len()));
                if g.len() == cand.len() {
                    match max_abs_diff_f32(&cand, g) {
                        Ok(d) => parts.push(format!("max_abs_diff={d:.6e}")),
                        Err(e) => parts.push(format!("max_abs_diff_err={e}")),
                    }
                    let gnz = g.iter().filter(|&&x| x != 0.0).count();
                    parts.push(format!("golden_nonzero_f32={gnz}"));
                } else {
                    parts.push("golden_len_mismatch".into());
                }
            }
        }
        Err(e) => parts.push(format!("decode_err={e}")),
    }
    parts.join(" ")
}

/// Parse OFM golden bytes as f32 LE (`m*n*4`) or bf16 LE (`m*n*2`).
///
/// Fail-closed on any other length.
///
/// # Errors
///
/// Empty buffer or size/dtype mismatch.
pub fn parse_ofm_golden_bytes(bytes: &[u8], m: u32, n: u32) -> Result<Vec<f32>, DirectError> {
    if bytes.is_empty() {
        return Err(DirectError::InvalidArgument(
            "OFM golden bytes are empty".into(),
        ));
    }
    let need_elems = u64::from(m)
        .checked_mul(u64::from(n))
        .ok_or_else(|| DirectError::InvalidArgument("OFM golden: m*n overflow".into()))?;
    let f32_len = need_elems
        .checked_mul(4)
        .ok_or_else(|| DirectError::InvalidArgument("OFM golden: f32 size overflow".into()))?;
    let bf16_len = need_elems
        .checked_mul(2)
        .ok_or_else(|| DirectError::InvalidArgument("OFM golden: bf16 size overflow".into()))?;
    if bytes.len() as u64 == f32_len {
        return f32_slice_from_le_bytes(bytes);
    }
    if bytes.len() as u64 == bf16_len {
        return unpack_ofm_bf16_to_f32(bytes);
    }
    Err(DirectError::InvalidArgument(format!(
        "OFM golden len {} is neither f32 LE (m*n*4={f32_len}) nor bf16 LE \
         (m*n*2={bf16_len}) for m={m} n={n}",
        bytes.len()
    )))
}

/// True when `XDNA2_LAB_ALLOW_ZERO_GOLDEN=1` / `true` (**offline resolve only**).
///
/// Default **off**: all-zero OFM golden is refused at resolve for mladf so
/// pack_mladf_ref's zero `ofm_expected.bin` cannot even load as a golden file.
///
/// **Does not** allow product [`decide_ran_on_npu`] / `ran_on_npu=true` — decide
/// always refuses all-zero golden ([`RanOnNpuBlockReason::AllZeroGolden`]).
#[must_use]
pub fn lab_allow_zero_ofm_golden_env() -> bool {
    lab_ofm_env_value_enabled(std::env::var("XDNA2_LAB_ALLOW_ZERO_GOLDEN").ok().as_deref())
}

/// True when every element is exactly ±0.0 (empty → true).
#[must_use]
pub fn ofm_golden_is_all_zero(golden: &[f32]) -> bool {
    super::numerical::f32_slice_is_all_zero(golden)
}

/// Refuse all-zero golden for [`LabPackMode::MladfA16W4`] unless `allow_zero`.
///
/// Offline resolve/load gate only. Product Pass still refuses zero golden via
/// [`decide_ran_on_npu_reason`] even when `allow_zero` is true.
///
/// # Errors
///
/// All-zero golden without allow flag.
pub fn check_mladf_ofm_golden_not_all_zero(
    mode: LabPackMode,
    golden: &[f32],
    allow_zero: bool,
) -> Result<(), DirectError> {
    if matches!(mode, LabPackMode::MladfA16W4) && !allow_zero && ofm_golden_is_all_zero(golden) {
        return Err(DirectError::InvalidArgument(
            "XDNA2_LAB_PACK=mladf_a16w4: all-zero OFM golden refused (would false-Pass against \
             host-zeroed OFM). Provide a non-zero golden. XDNA2_LAB_ALLOW_ZERO_GOLDEN=1 only \
             bypasses this *resolve* gate for plumbing (product decide still refuses zero golden). \
             See docs/AMD_MLADF_MATMUL_PACK.md"
                .into(),
        ));
    }
    Ok(())
}

/// Load optional packed OFM golden from `XDNA2_LAB_OFM_GOLDEN`.
///
/// Accepts:
/// - **f32 LE** file length `m*n*4`
/// - **bf16 LE** file length `m*n*2` (decoded to f32 for allclose)
///
/// Fail-closed on any other length or empty path when the env var is set.
/// File size is capped via [`read_lab_file_capped`].
///
/// # Errors
///
/// I/O, empty path, oversize, or size/dtype mismatch.
pub fn load_ofm_golden_from_env(m: u32, n: u32) -> Result<Option<Vec<f32>>, DirectError> {
    let Ok(path) = std::env::var("XDNA2_LAB_OFM_GOLDEN") else {
        return Ok(None);
    };
    let path = path.trim();
    if path.is_empty() {
        return Err(DirectError::InvalidArgument(
            "XDNA2_LAB_OFM_GOLDEN is set but empty".into(),
        ));
    }
    let bytes = read_lab_file_capped(path, "XDNA2_LAB_OFM_GOLDEN")?;
    Ok(Some(parse_ofm_golden_bytes(&bytes, m, n).map_err(|e| {
        DirectError::InvalidArgument(format!("{e} (path={path})"))
    })?))
}

/// Resolve host golden for pack-mode OFM decide (pure; no env I/O).
///
/// - `explicit_golden` wins when present.
/// - else `file_golden` (from `XDNA2_LAB_OFM_GOLDEN` load).
/// - else f32 ones for [`LabPackMode::F32Ones`].
/// - else fail-closed for [`LabPackMode::MladfA16W4`] (ones golden invalid).
/// - For mladf: also refuses **all-zero** golden unless `allow_zero_golden`.
///
/// # Errors
///
/// Missing golden for mladf, all-zero golden, or ones golden build failure.
pub fn resolve_pack_mode_ofm_golden(
    mode: LabPackMode,
    m: u32,
    n: u32,
    k: u32,
    explicit_golden: Option<&[f32]>,
    file_golden: Option<&[f32]>,
    allow_zero_golden: bool,
) -> Result<Vec<f32>, DirectError> {
    let golden = if let Some(g) = explicit_golden {
        g.to_vec()
    } else if let Some(g) = file_golden {
        g.to_vec()
    } else {
        match mode {
            LabPackMode::F32Ones => host_matmul_f32_ones_vec(m, n, k)?,
            LabPackMode::MladfA16W4 => {
                return Err(DirectError::InvalidArgument(
                    "XDNA2_LAB_PACK=mladf_a16w4: f32 ones×ones golden is invalid for a16w4; \
                     set XDNA2_LAB_OFM_GOLDEN to a bf16 (m*n*2) or f32 (m*n*4) golden file \
                     (see docs/AMD_MLADF_MATMUL_PACK.md)"
                        .into(),
                ));
            }
        }
    };
    let need = u64::from(m)
        .checked_mul(u64::from(n))
        .ok_or_else(|| DirectError::InvalidArgument("resolve OFM golden: m*n overflow".into()))?;
    if golden.len() as u64 != need {
        return Err(DirectError::InvalidArgument(format!(
            "resolve_pack_mode_ofm_golden: golden len {} != m*n={} (m={m} n={n})",
            golden.len(),
            need
        )));
    }
    check_mladf_ofm_golden_not_all_zero(mode, &golden, allow_zero_golden)?;
    Ok(golden)
}

/// Pack-mode-aware OFM decide for timed lab path.
///
/// - [`LabPackMode::F32Ones`]: f32 LE OFM + ones golden (or `explicit_golden`).
/// - [`LabPackMode::MladfA16W4`]: bf16 LE OFM; **refuses** f32 ones×ones default
///   and **all-zero** golden (unless `XDNA2_LAB_ALLOW_ZERO_GOLDEN=1`).
///   Requires `explicit_golden` or `XDNA2_LAB_OFM_GOLDEN` (f32 or bf16 file).
///
/// # Errors
///
/// Decode/size/golden resolve failures (fail-closed).
pub fn lab_decide_from_ofm_bytes_pack_mode(
    mode: LabPackMode,
    plan: &LabOfmVerifyPlan,
    explicit_golden: Option<&[f32]>,
) -> Result<(bool, Option<RanOnNpuBlockReason>), DirectError> {
    let candidate = lab_decode_ofm_bytes(mode, &plan.ofm_bytes)?;
    let file_g = if explicit_golden.is_some() {
        None
    } else {
        load_ofm_golden_from_env(plan.m, plan.n)?
    };
    let golden = resolve_pack_mode_ofm_golden(
        mode,
        plan.m,
        plan.n,
        plan.k,
        explicit_golden,
        file_g.as_deref(),
        lab_allow_zero_ofm_golden_env(),
    )?;
    lab_decide_candidate_f32_with_golden(plan, &candidate, &golden)
}

/// Convenience: bool-only wrapper (false on any policy refuse; decode errors Err).
pub fn lab_decide_from_ofm_bytes_bool(plan: &LabOfmVerifyPlan) -> Result<bool, DirectError> {
    Ok(lab_decide_from_ofm_bytes(plan)?.0)
}

// ---------------------------------------------------------------------------
// IoBoSet plan + lifecycle (stub-testable; Real uses ioctls)
// ---------------------------------------------------------------------------

/// Lab IFM/WTS/OFM buffer object set (handles + device paddrs + sizes).
///
/// Does **not** imply EXEC succeeded or OFM is valid compute output.
///
/// `*_bytes` are **logical** payload sizes (matmul plan). `*_alloc_bytes` are
/// CREATE_BO sizes (`max(logical, 4096)` page-ish). Zeroing uses alloc size so
/// page tails are not left stale.
#[derive(Debug, Clone)]
pub struct IoBoSet {
    /// IFM BO handle.
    pub ifm: BufferHandle,
    /// WTS BO handle.
    pub wts: BufferHandle,
    /// OFM BO handle.
    pub ofm: BufferHandle,
    /// Optional control-packet BO (mladf `runtime_control_packet`).
    pub ctrl: Option<BufferHandle>,
    /// Device-visible IFM address (`xdna_addr`).
    pub ifm_addr: u64,
    /// Device-visible WTS address.
    pub wts_addr: u64,
    /// Device-visible OFM address.
    pub ofm_addr: u64,
    /// Device-visible CTRL address (0 if none).
    pub ctrl_addr: u64,
    /// Logical IFM payload bytes (fill exact).
    pub ifm_bytes: u64,
    /// Logical WTS payload bytes (fill exact).
    pub wts_bytes: u64,
    /// Logical OFM payload bytes (readback exact).
    pub ofm_bytes: u64,
    /// Logical CTRL payload bytes (0 if none).
    pub ctrl_bytes: u64,
    /// Allocated IFM BO size (≥ `ifm_bytes`, typically page-rounded).
    pub ifm_alloc_bytes: u64,
    /// Allocated WTS BO size.
    pub wts_alloc_bytes: u64,
    /// Allocated OFM BO size (full zero before EXEC).
    pub ofm_alloc_bytes: u64,
    /// Allocated CTRL BO size (0 if none).
    pub ctrl_alloc_bytes: u64,
}

impl IoBoSet {
    /// Handles for EXEC_CMD `args[]` pin list (IFM, WTS, OFM, optional CTRL).
    #[must_use]
    pub fn arg_bo_handles(&self) -> Vec<u32> {
        let mut v = vec![self.ifm.0, self.wts.0, self.ofm.0];
        if let Some(c) = self.ctrl {
            v.push(c.0);
        }
        v
    }

    /// START_NPU kernel-arg dwords for this set (includes ctrl paddr when present).
    ///
    /// Uses env layout when valid; default count4 if unset. Timed path must
    /// fail-closed-parse env before calling this for residual HITL.
    #[must_use]
    pub fn pack_kernel_args(&self) -> Vec<u32> {
        self.pack_kernel_args_layout(
            lab_kernel_arg_layout_from_env().unwrap_or(LabKernelArgLayout::Count4Ctrl),
        )
    }

    /// Explicit layout variant of [`Self::pack_kernel_args`].
    #[must_use]
    pub fn pack_kernel_args_layout(&self, layout: LabKernelArgLayout) -> Vec<u32> {
        pack_start_npu_kernel_args_with_ctrl_layout(
            self.ifm_addr,
            self.wts_addr,
            self.ofm_addr,
            self.ctrl.map(|_| self.ctrl_addr),
            layout,
        )
    }
}

/// Build EXEC_CMD `args[]` pin list: optional IFM/WTS/OFM then optional instruction BO.
///
/// Pure (no device). Matches XRT-style pin of data BOs + instruction when present.
/// Instruction is always appended last when `inst_handle` is `Some`.
#[must_use]
pub fn build_lab_arg_bo_handles(
    io_handles: Option<(u32, u32, u32)>,
    inst_handle: Option<u32>,
) -> Vec<u32> {
    build_lab_arg_bo_handles_ex(io_handles, None, inst_handle)
}

/// Like [`build_lab_arg_bo_handles`] with optional control-packet handle before instruction.
#[must_use]
pub fn build_lab_arg_bo_handles_ex(
    io_handles: Option<(u32, u32, u32)>,
    ctrl_handle: Option<u32>,
    inst_handle: Option<u32>,
) -> Vec<u32> {
    let mut out = Vec::with_capacity(5);
    if let Some((ifm, wts, ofm)) = io_handles {
        out.push(ifm);
        out.push(wts);
        out.push(ofm);
    }
    if let Some(c) = ctrl_handle {
        out.push(c);
    }
    if let Some(h) = inst_handle {
        out.push(h);
    }
    out
}

/// Pure status classifier for timed_matmul lab OFM outcomes (no device I/O).
///
/// Callers that have a more specific submit failure string may override the
/// generic `"submit_wait_failed"` result (e.g. `"ert_cmd_error"` when ERT state
/// is Error after WAIT).
///
/// # Rules
/// - `lab_ofm && !ofm_setup_ok` → `lab_ofm_setup_failed` (fail closed before EXEC)
/// - `!submit_wait_ok` → `submit_wait_failed` (override with classified errno path)
/// - lab + submit + !readback → `ofm_readback_failed`
/// - lab + submit + readback + decide decode/policy **Err** → `ofm_decide_error`
/// - lab + submit + decide true → `ok_npu_verified`
/// - lab + submit + readback + allclose failed → `ofm_verify_failed`
/// - else envelope / inst-blob submit statuses (ran_on_npu still false)
///
/// Callers should set `ofm_allclose_failed` for [`RanOnNpuBlockReason::AllcloseFailed`]
/// after a successful OFM readback so product status is not a false envelope OK.
/// Set `ofm_decide_error` when `lab_decide_*` returns `Err` (e.g. golden len / decode).
#[must_use]
#[allow(clippy::fn_params_excessive_bools, clippy::too_many_arguments)] // honesty flags
pub fn classify_lab_matmul_status(
    lab_ofm_enabled: bool,
    ofm_setup_ok: bool,
    submit_wait_ok: bool,
    used_inst_blob: bool,
    ofm_readback_ok: bool,
    decide_ran: bool,
    ofm_allclose_failed: bool,
    ofm_decide_error: bool,
) -> &'static str {
    if lab_ofm_enabled && !ofm_setup_ok {
        return "lab_ofm_setup_failed";
    }
    if !submit_wait_ok {
        return "submit_wait_failed";
    }
    if lab_ofm_enabled {
        if !ofm_readback_ok {
            return "ofm_readback_failed";
        }
        if ofm_decide_error {
            return "ofm_decide_error";
        }
        if decide_ran {
            return "ok_npu_verified";
        }
        // Had OFM bytes + numerical/content refuse → verify failed (not envelope OK).
        if ofm_allclose_failed {
            return "ofm_verify_failed";
        }
        // Other decide refuses (missing blob / stub / submit flags inside decide)
        // keep submit envelope status so operators see gate/path honesty.
    }
    if used_inst_blob {
        "ok_submit_wait_with_inst_blob"
    } else {
        "ok_submit_wait_envelope"
    }
}

/// Pure size plan for ones-matmul lab I/O BOs (no device).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LabIoSizePlan {
    /// IFM bytes.
    pub ifm_bytes: u64,
    /// WTS bytes.
    pub wts_bytes: u64,
    /// OFM bytes.
    pub ofm_bytes: u64,
}

/// Compute IFM/WTS/OFM byte sizes for f32 ones matmul `m×k @ k×n`.
///
/// # Errors
///
/// Dim / dtype / overflow errors from size helpers.
pub fn lab_io_size_plan(m: u32, n: u32, k: u32, dtype: &str) -> Result<LabIoSizePlan, DirectError> {
    Ok(LabIoSizePlan {
        ifm_bytes: lab_matmul_ifm_nbytes(m, k, dtype)?,
        wts_bytes: lab_matmul_wts_nbytes(k, n, dtype)?,
        ofm_bytes: lab_matmul_ofm_nbytes(m, n, dtype)?,
    })
}

/// CREATE_BO(Shmem)×3 + **PASID pin** + device addrs; refuse invalid `xdna_addr`.
///
/// Live lab: temporary mmap+munmap left SHMEM `xdna_addr=u64::MAX` (invalid).
/// We pin (persistent mmap) so PASID UVA is registered before EXEC.
///
/// Does **not** submit. Does **not** claim compute. Caller must
/// [`destroy_io_bos`].
///
/// # Errors
///
/// Create / pin / info failures or invalid device address.
pub fn create_io_bos(
    submission: &dyn DirectSubmission,
    ifm_bytes: u64,
    wts_bytes: u64,
    ofm_bytes: u64,
) -> Result<IoBoSet, DirectError> {
    if ifm_bytes == 0 || wts_bytes == 0 || ofm_bytes == 0 {
        return Err(DirectError::InvalidArgument(
            "create_io_bos requires non-zero IFM/WTS/OFM sizes".into(),
        ));
    }
    // Round up to page-ish minimum used elsewhere for CREATE_BO.
    let alloc_sz = |nbytes: u64| nbytes.max(4096);
    let ifm_alloc = alloc_sz(ifm_bytes);
    let wts_alloc = alloc_sz(wts_bytes);
    let ofm_alloc = alloc_sz(ofm_bytes);
    let ifm = submission.create_buffer(0, ifm_alloc, BufferType::Shmem)?;
    let wts = match submission.create_buffer(0, wts_alloc, BufferType::Shmem) {
        Ok(h) => h,
        Err(e) => {
            let _ = submission.destroy_buffer(0, ifm.0);
            return Err(e);
        }
    };
    let ofm = match submission.create_buffer(0, ofm_alloc, BufferType::Shmem) {
        Ok(h) => h,
        Err(e) => {
            let _ = submission.destroy_buffer(0, wts.0);
            let _ = submission.destroy_buffer(0, ifm.0);
            return Err(e);
        }
    };

    let cleanup = |submission: &dyn DirectSubmission, a: u32, b: u32, c: u32| {
        let _ = submission.destroy_buffer(0, c);
        let _ = submission.destroy_buffer(0, b);
        let _ = submission.destroy_buffer(0, a);
    };

    // Pin before reading device addrs (PASID UVA registered on mmap).
    let ifm_addr = match submission.pin_bo_for_device_access(0, ifm.0, ifm_alloc) {
        Ok(a) if is_valid_xdna_device_addr(a) => a,
        Ok(a) => {
            cleanup(submission, ifm.0, wts.0, ofm.0);
            return Err(DirectError::InvalidArgument(format!(
                "create_io_bos: IFM xdna_addr={a:#x} invalid after pin"
            )));
        }
        Err(e) => {
            cleanup(submission, ifm.0, wts.0, ofm.0);
            return Err(e);
        }
    };
    let wts_addr = match submission.pin_bo_for_device_access(0, wts.0, wts_alloc) {
        Ok(a) if is_valid_xdna_device_addr(a) => a,
        Ok(a) => {
            cleanup(submission, ifm.0, wts.0, ofm.0);
            return Err(DirectError::InvalidArgument(format!(
                "create_io_bos: WTS xdna_addr={a:#x} invalid after pin"
            )));
        }
        Err(e) => {
            cleanup(submission, ifm.0, wts.0, ofm.0);
            return Err(e);
        }
    };
    let ofm_addr = match submission.pin_bo_for_device_access(0, ofm.0, ofm_alloc) {
        Ok(a) if is_valid_xdna_device_addr(a) => a,
        Ok(a) => {
            cleanup(submission, ifm.0, wts.0, ofm.0);
            return Err(DirectError::InvalidArgument(format!(
                "create_io_bos: OFM xdna_addr={a:#x} invalid after pin"
            )));
        }
        Err(e) => {
            cleanup(submission, ifm.0, wts.0, ofm.0);
            return Err(e);
        }
    };

    Ok(IoBoSet {
        ifm,
        wts,
        ofm,
        ctrl: None,
        ifm_addr,
        wts_addr,
        ofm_addr,
        ctrl_addr: 0,
        ifm_bytes,
        wts_bytes,
        ofm_bytes,
        ctrl_bytes: 0,
        ifm_alloc_bytes: ifm_alloc,
        wts_alloc_bytes: wts_alloc,
        ofm_alloc_bytes: ofm_alloc,
        ctrl_alloc_bytes: 0,
    })
}

/// Best-effort destroy of IFM/WTS/OFM(/CTRL) handles.
pub fn destroy_io_bos(submission: &dyn DirectSubmission, ios: &IoBoSet) {
    if let Some(c) = ios.ctrl {
        let _ = submission.destroy_buffer(0, c.0);
    }
    let _ = submission.destroy_buffer(0, ios.ofm.0);
    let _ = submission.destroy_buffer(0, ios.wts.0);
    let _ = submission.destroy_buffer(0, ios.ifm.0);
}

/// Host-fill IFM and WTS (`write_bo_bytes` → Real SYNC TO).
///
/// Host payload length must equal the logical plan sizes on [`IoBoSet`]
/// (`ifm_bytes` / `wts_bytes`) — neither undersize nor oversize.
///
/// # Errors
///
/// Empty host buffers, length mismatch vs plan, or write failures.
pub fn fill_ifm_wts(
    submission: &dyn DirectSubmission,
    ios: &IoBoSet,
    ifm: &[u8],
    wts: &[u8],
) -> Result<(), DirectError> {
    if ifm.is_empty() || wts.is_empty() {
        return Err(DirectError::InvalidArgument(
            "fill_ifm_wts refuses empty IFM or WTS host bytes".into(),
        ));
    }
    if ifm.len() as u64 != ios.ifm_bytes {
        return Err(DirectError::InvalidArgument(format!(
            "fill_ifm_wts IFM host {} != plan {} (exact length required)",
            ifm.len(),
            ios.ifm_bytes
        )));
    }
    if wts.len() as u64 != ios.wts_bytes {
        return Err(DirectError::InvalidArgument(format!(
            "fill_ifm_wts WTS host {} != plan {} (exact length required)",
            wts.len(),
            ios.wts_bytes
        )));
    }
    submission.write_bo_bytes(0, ios.ifm.0, 0, ifm)?;
    submission.write_bo_bytes(0, ios.wts.0, 0, wts)?;
    // Zero page-rounded tails so stale alloc bytes never leak into device view.
    zero_bo_tail(submission, ios.ifm.0, ios.ifm_bytes, ios.ifm_alloc_bytes)?;
    zero_bo_tail(submission, ios.wts.0, ios.wts_bytes, ios.wts_alloc_bytes)?;
    Ok(())
}

/// Write zeros from `logical`..`alloc` on a BO (no-op when alloc ≤ logical).
fn zero_bo_tail(
    submission: &dyn DirectSubmission,
    handle: u32,
    logical: u64,
    alloc: u64,
) -> Result<(), DirectError> {
    if alloc <= logical {
        return Ok(());
    }
    let tail = alloc - logical;
    let len = usize::try_from(tail)
        .map_err(|_| DirectError::InvalidArgument("zero_bo_tail: tail exceeds usize".into()))?;
    let zeros = vec![0u8; len];
    submission.write_bo_bytes(0, handle, logical, &zeros)
}

/// Explicit host-zero of the **full allocated** OFM BO (`write_bo_bytes` → SYNC TO on Real).
///
/// CREATE alone is not trusted to present zeros; zeros the page-rounded
/// `ofm_alloc_bytes` so tails are clean before EXEC.
///
/// # Errors
///
/// Zero alloc size or write failures.
pub fn zero_ofm(submission: &dyn DirectSubmission, ios: &IoBoSet) -> Result<(), DirectError> {
    let nbytes = ios.ofm_alloc_bytes.max(ios.ofm_bytes);
    if nbytes == 0 {
        return Err(DirectError::InvalidArgument("zero_ofm: ofm size=0".into()));
    }
    let len = usize::try_from(nbytes)
        .map_err(|_| DirectError::InvalidArgument("zero_ofm: size exceeds usize".into()))?;
    let zeros = vec![0u8; len];
    submission.write_bo_bytes(0, ios.ofm.0, 0, &zeros)
}

/// After WAIT: SYNC FROM OFM + read bytes (no second FROM).
///
/// # Errors
///
/// Sync/read failures or empty plan size.
pub fn readback_ofm(
    submission: &dyn DirectSubmission,
    ios: &IoBoSet,
) -> Result<Vec<u8>, DirectError> {
    if ios.ofm_bytes == 0 {
        return Err(DirectError::InvalidArgument(
            "readback_ofm: ofm_bytes=0".into(),
        ));
    }
    submission.sync_buffer(0, ios.ofm, SYNC_BO_FROM_DEVICE, 0, ios.ofm_bytes.max(4096))?;
    let len = usize::try_from(ios.ofm_bytes).map_err(|_| {
        DirectError::InvalidArgument("readback_ofm: ofm_bytes exceeds usize".into())
    })?;
    submission.read_bo_bytes_no_sync(0, ios.ofm.0, 0, len)
}

/// Create + fill ones IFM/WTS for matmul dims; **explicit host-zero OFM**.
///
/// # Errors
///
/// Size / create / fill failures.
pub fn create_and_fill_ones_io_bos(
    submission: &dyn DirectSubmission,
    m: u32,
    n: u32,
    k: u32,
    dtype: &str,
) -> Result<IoBoSet, DirectError> {
    let plan = lab_io_size_plan(m, n, k, dtype)?;
    let ifm = host_ones_f32_matrix_bytes(m, k)?;
    let wts = host_ones_f32_matrix_bytes(k, n)?;
    let ios = create_io_bos(submission, plan.ifm_bytes, plan.wts_bytes, plan.ofm_bytes)?;
    if let Err(e) = fill_ifm_wts(submission, &ios, &ifm, &wts) {
        destroy_io_bos(submission, &ios);
        return Err(e);
    }
    if let Err(e) = zero_ofm(submission, &ios) {
        destroy_io_bos(submission, &ios);
        return Err(e);
    }
    Ok(ios)
}

/// Create IFM/WTS/OFM BOs and fill them for AMD `mladf_a16w4` pack mode.
///
/// Sizes come from [`MladfBufferPlan`] (ctrl_meta / staged key). IFM is bf16 ones
/// (or packed from `ifm_f32`). WTS is `wts_packed` or zeros of the correct length
/// (not a fake shuffle — load AMD-packed WTS via `XDNA2_LAB_WTS_PACKED` for real
/// weights). OFM is zeroed.
///
/// **No EXEC.** Does not claim Pass.
///
/// # Errors
///
/// Plan/payload size mismatch or BO create/fill failure.
pub fn create_and_fill_mladf_io_bos(
    submission: &dyn DirectSubmission,
    plan: &MladfBufferPlan,
    ifm_f32: Option<&[f32]>,
    wts_packed: Option<&[u8]>,
) -> Result<IoBoSet, DirectError> {
    create_and_fill_mladf_io_bos_ex(submission, plan, ifm_f32, wts_packed, None)
}

/// Like [`create_and_fill_mladf_io_bos`] with optional control-packet bytes.
///
/// When `ctrl_packed` is `Some`, creates a fourth BO of `plan.ctrl_bytes` (or
/// payload length if plan.ctrl_bytes==0), fills it, and sets kernel-arg count=4.
///
/// When `XDNA2_LAB_REQUIRE_CTRL=1` and plan has `ctrl_bytes`:
/// - missing `ctrl_packed` → refuse
/// - present but empty / all-zero / wrong length → refuse
///   ([`validate_mladf_ctrl_packed_for_pass`])
///
/// When `XDNA2_LAB_CTRL_AUTO_PATCH=1` and `XDNA2_LAB_MLADF_META` is set with
/// non-empty `control_packet_patch_locations`, device IFM/WTS/OFM paddrs are
/// written into a mutable ctrl copy (**u64 LE overwrite** residual — not full
/// XRT size=6 `read_add_write`) before attach. Default auto-patch **off**.
///
/// # Errors
///
/// Size mismatch, missing/empty/all-zero required ctrl when env requires it,
/// meta patch OOB, or BO failures.
pub fn create_and_fill_mladf_io_bos_ex(
    submission: &dyn DirectSubmission,
    plan: &MladfBufferPlan,
    ifm_f32: Option<&[f32]>,
    wts_packed: Option<&[u8]>,
    ctrl_packed: Option<&[u8]>,
) -> Result<IoBoSet, DirectError> {
    let (ifm, wts, _ofm_host) = build_mladf_lab_payloads(plan, ifm_f32, wts_packed)?;
    // Fail-closed for EXEC readiness when XDNA2_LAB_REQUIRE_NONZERO_WTS=1
    // (REQUIRE wins over ALLOW_ZERO_WTS; see check_mladf_wts_not_all_zero_for_exec).
    // Without REQUIRE, zero WTS is still allowed for BO plumbing (Err ignored).
    if let Err(e) = check_mladf_wts_not_all_zero_for_exec(&wts)
        && lab_require_non_zero_wts_env()
    {
        return Err(e);
    }
    let mut ios = create_io_bos(submission, plan.ifm_bytes, plan.wts_bytes, plan.ofm_bytes)?;
    if let Err(e) = fill_ifm_wts(submission, &ios, &ifm, &wts) {
        destroy_io_bos(submission, &ios);
        return Err(e);
    }
    if let Err(e) = zero_ofm(submission, &ios) {
        destroy_io_bos(submission, &ios);
        return Err(e);
    }
    if let Some(ctrl) = ctrl_packed {
        let expect = if plan.ctrl_bytes > 0 {
            plan.ctrl_bytes
        } else {
            ctrl.len() as u64
        };
        if ctrl.len() as u64 != expect {
            destroy_io_bos(submission, &ios);
            return Err(DirectError::InvalidArgument(format!(
                "mladf CTRL packed {} != plan/expected {expect}",
                ctrl.len()
            )));
        }
        // Pass-readiness: REQUIRE_CTRL refuses empty/all-zero (parity with WTS REQUIRE).
        if lab_require_ctrl_env()
            && let Err(e) = validate_mladf_ctrl_packed_for_pass(ctrl, expect)
        {
            destroy_io_bos(submission, &ios);
            return Err(e);
        }
        // Optional lab auto-patch: IFM/WTS/OFM base paddrs via meta read_add_write.
        let mut patched_owned: Option<Vec<u8>> = None;
        if lab_ctrl_auto_patch_env() {
            match load_mladf_meta_json_text_from_env() {
                Ok(Some(meta)) => {
                    let mut buf = ctrl.to_vec();
                    if let Err(e) = patch_ctrl_from_meta_io_paddrs(
                        &mut buf,
                        &meta,
                        ios.ifm_addr,
                        ios.wts_addr,
                        ios.ofm_addr,
                    ) {
                        destroy_io_bos(submission, &ios);
                        return Err(e);
                    }
                    patched_owned = Some(buf);
                }
                Ok(None) => {}
                Err(e) => {
                    destroy_io_bos(submission, &ios);
                    return Err(e);
                }
            }
        }
        let to_attach = patched_owned.as_deref().unwrap_or(ctrl);
        if let Err(e) = attach_ctrl_bo(submission, &mut ios, to_attach) {
            destroy_io_bos(submission, &ios);
            return Err(e);
        }
    } else if plan.ctrl_bytes > 0 && lab_require_ctrl_env() {
        destroy_io_bos(submission, &ios);
        return Err(DirectError::InvalidArgument(format!(
            "mladf plan ctrl_bytes={} but no ctrl payload — set XDNA2_LAB_CTRL_PACKED \
             or XDNA2_LAB_REQUIRE_CTRL=0 for IFM/WTS/OFM-only plumbing",
            plan.ctrl_bytes
        )));
    }
    Ok(ios)
}

/// `XDNA2_LAB_REQUIRE_CTRL=1` forces refuse when plan has ctrl_bytes but ctrl is
/// missing, empty, or all-zero (see [`validate_mladf_ctrl_packed_for_pass`]).
///
/// Default **false** (unset) so unit tests and 3-BO plumbing work without a ctrl file.
/// Operators set `=1` / `true` for live EXEC dry-runs (same truth table as `LAB_OFM`).
#[must_use]
pub fn lab_require_ctrl_env() -> bool {
    lab_ofm_env_value_enabled(std::env::var("XDNA2_LAB_REQUIRE_CTRL").ok().as_deref())
}

/// `XDNA2_LAB_CTRL_AUTO_PATCH=1` applies meta IFM/WTS/OFM paddrs into the ctrl
/// buffer before attach (**size=6 `read_add_write`**: existing tile offset + base).
///
/// Default **false**. Requires `XDNA2_LAB_MLADF_META` with patch locations.
#[must_use]
pub fn lab_ctrl_auto_patch_env() -> bool {
    lab_ofm_env_value_enabled(std::env::var("XDNA2_LAB_CTRL_AUTO_PATCH").ok().as_deref())
}

/// Load optional `XDNA2_LAB_MLADF_META` text for ctrl paddr patch (capped read).
///
/// # Errors
///
/// I/O or non-UTF-8 when the env var is set.
pub fn load_mladf_meta_json_text_from_env() -> Result<Option<String>, DirectError> {
    let Ok(meta_path) = std::env::var("XDNA2_LAB_MLADF_META") else {
        return Ok(None);
    };
    let path = meta_path.trim();
    if path.is_empty() {
        return Err(DirectError::InvalidArgument(
            "XDNA2_LAB_MLADF_META is set but empty".into(),
        ));
    }
    let bytes = read_lab_file_capped(path, "XDNA2_LAB_MLADF_META")?;
    let text = String::from_utf8(bytes).map_err(|e| {
        DirectError::InvalidArgument(format!("XDNA2_LAB_MLADF_META is not UTF-8 ({path}): {e}"))
    })?;
    Ok(Some(text))
}

/// `XDNA2_LAB_REQUIRE_NONZERO_WTS=1` forces refuse of all-zero WTS at fill time.
#[must_use]
pub fn lab_require_non_zero_wts_env() -> bool {
    lab_ofm_env_value_enabled(
        std::env::var("XDNA2_LAB_REQUIRE_NONZERO_WTS")
            .ok()
            .as_deref(),
    )
}

/// Load control packet from `XDNA2_LAB_CTRL_PACKED` (capped read).
///
/// # Errors
///
/// I/O or empty when set.
pub fn load_ctrl_packed_from_env(
    expected_len: Option<u64>,
) -> Result<Option<Vec<u8>>, DirectError> {
    let Ok(path) = std::env::var("XDNA2_LAB_CTRL_PACKED") else {
        return Ok(None);
    };
    let path = path.trim();
    if path.is_empty() {
        return Err(DirectError::InvalidArgument(
            "XDNA2_LAB_CTRL_PACKED is set but empty".into(),
        ));
    }
    let bytes = crate::backends::amd_xdna2::mladf_pack::read_lab_file_capped(
        path,
        "XDNA2_LAB_CTRL_PACKED",
    )?;
    if bytes.is_empty() {
        return Err(DirectError::InvalidArgument(format!(
            "XDNA2_LAB_CTRL_PACKED file is empty: {path}"
        )));
    }
    if let Some(exp) = expected_len
        && bytes.len() as u64 != exp
    {
        return Err(DirectError::InvalidArgument(format!(
            "XDNA2_LAB_CTRL_PACKED len {} != expected {exp}",
            bytes.len()
        )));
    }
    Ok(Some(bytes))
}

/// Attach a control-packet BO to an existing [`IoBoSet`] (fills from `ctrl` bytes).
///
/// # Errors
///
/// Create/write/info failures or zero xdna_addr.
pub fn attach_ctrl_bo(
    submission: &dyn DirectSubmission,
    ios: &mut IoBoSet,
    ctrl: &[u8],
) -> Result<(), DirectError> {
    if ctrl.is_empty() {
        return Err(DirectError::InvalidArgument(
            "attach_ctrl_bo: empty ctrl payload".into(),
        ));
    }
    let ctrl_bytes = ctrl.len() as u64;
    let alloc = ctrl_bytes.max(4096);
    let h = submission.create_buffer(0, alloc, BufferType::Shmem)?;
    if let Err(e) = submission.write_bo_bytes(0, h.0, 0, ctrl) {
        let _ = submission.destroy_buffer(0, h.0);
        return Err(e);
    }
    // zero tail past logical ctrl payload
    if alloc > ctrl_bytes {
        let tail_len = usize::try_from(alloc - ctrl_bytes).map_err(|_| {
            DirectError::InvalidArgument("attach_ctrl_bo: tail length does not fit usize".into())
        })?;
        let tail = vec![0_u8; tail_len];
        if let Err(e) = submission.write_bo_bytes(0, h.0, ctrl_bytes, &tail) {
            let _ = submission.destroy_buffer(0, h.0);
            return Err(e);
        }
    }
    let ctrl_addr = match submission.pin_bo_for_device_access(0, h.0, alloc) {
        Ok(a) if is_valid_xdna_device_addr(a) => a,
        Ok(a) => {
            let _ = submission.destroy_buffer(0, h.0);
            return Err(DirectError::InvalidArgument(format!(
                "attach_ctrl_bo: CTRL xdna_addr={a:#x} invalid after pin"
            )));
        }
        Err(e) => {
            let _ = submission.destroy_buffer(0, h.0);
            return Err(e);
        }
    };
    ios.ctrl = Some(h);
    ios.ctrl_addr = ctrl_addr;
    ios.ctrl_bytes = ctrl_bytes;
    ios.ctrl_alloc_bytes = alloc;
    Ok(())
}

/// Pure **full u64 LE overwrite** at each offset (plumbing / tests).
///
/// Prefer [`patch_ctrl_from_meta_io_paddrs`] for live mladf — staged meta uses
/// `size: 6` + `operation: read_add_write` (see [`apply_ctrl_patch_location`]).
/// Fail-closed if `offset+8 > len`.
///
/// # Errors
///
/// Out of range offset.
pub fn patch_ctrl_packet_u64_le(
    ctrl: &mut [u8],
    patches: &[(usize, u64)],
) -> Result<(), DirectError> {
    for &(off, paddr) in patches {
        if off.checked_add(8).is_none_or(|end| end > ctrl.len()) {
            return Err(DirectError::InvalidArgument(format!(
                "ctrl patch offset {off}+8 out of range (ctrl len {})",
                ctrl.len()
            )));
        }
        ctrl[off..off + 8].copy_from_slice(&paddr.to_le_bytes());
    }
    Ok(())
}

/// Apply one meta patch site: AMD `read_add_write` size=6 or u64 overwrite.
///
/// Staged `lab_ctrl_meta.json` lists `size: 6`, `operation: "read_add_write"`.
/// Firmware fields already hold **tile sub-offsets** (live dump: OFM `0x100`,
/// WTS `0x15400`). Correct apply is:
/// 1. Read existing `size` LE bytes as u64 (zero-extend).
/// 2. `sum = existing.wrapping_add(base_paddr)` (fail if sum does not fit in `size`).
/// 3. Write **only** `size` LE bytes back — do **not** clobber neighbors.
///
/// Full u64 overwrite (old lab residual) **wipes** those offsets → wrong DMA →
/// zero OFM after EXEC+WAIT (live 2026-07-16 after D3c map_host Pass).
///
/// # Errors
///
/// OOB, unsupported size, or sum not fitting in the field width.
pub fn apply_ctrl_patch_location(
    ctrl: &mut [u8],
    offset: usize,
    size: u32,
    operation: &str,
    base_paddr: u64,
) -> Result<(), DirectError> {
    let size_n = usize::try_from(size).map_err(|_| {
        DirectError::InvalidArgument(format!("ctrl patch size {size} does not fit usize"))
    })?;
    if size_n == 0 || size_n > 8 {
        return Err(DirectError::InvalidArgument(format!(
            "ctrl patch size {size} unsupported (need 1..=8)"
        )));
    }
    let end = offset.checked_add(size_n).ok_or_else(|| {
        DirectError::InvalidArgument(format!(
            "ctrl patch offset {offset}+{size_n} overflow (ctrl len {})",
            ctrl.len()
        ))
    })?;
    if end > ctrl.len() {
        return Err(DirectError::InvalidArgument(format!(
            "ctrl patch offset {offset}+{size_n} out of range (ctrl len {})",
            ctrl.len()
        )));
    }

    let op = operation.trim().to_ascii_lowercase();
    // AMD staged: always read_add_write. Empty op + size 6 also treated as add
    // (fail-closed toward the live meta convention).
    let do_add = op.is_empty()
        || op == "read_add_write"
        || op.contains("add")
        || (size_n == 6 && op != "write" && op != "overwrite");

    let mut existing_bytes = [0_u8; 8];
    existing_bytes[..size_n].copy_from_slice(&ctrl[offset..end]);
    let existing = u64::from_le_bytes(existing_bytes);

    let value = if do_add {
        existing.checked_add(base_paddr).ok_or_else(|| {
            DirectError::InvalidArgument(format!(
                "ctrl patch read_add_write wrap: existing={existing:#x} + base={base_paddr:#x}"
            ))
        })?
    } else {
        base_paddr
    };

    let max_for_size = if size_n == 8 {
        u64::MAX
    } else {
        (1_u64 << (size_n * 8)) - 1
    };
    if value > max_for_size {
        return Err(DirectError::InvalidArgument(format!(
            "ctrl patch value {value:#x} does not fit size={size_n} (max {max_for_size:#x}); \
             existing={existing:#x} base={base_paddr:#x} op={operation:?}"
        )));
    }

    let out = value.to_le_bytes();
    ctrl[offset..end].copy_from_slice(&out[..size_n]);
    Ok(())
}

/// Apply meta-derived IFM/WTS/OFM paddr patches into a mutable ctrl buffer (pure).
///
/// Parses `control_packet_patch_locations`, maps roles → paddrs, then
/// [`apply_ctrl_patch_location`] per site (**size=6 `read_add_write`** by default).
/// Empty patch list → no-op Ok (still not Pass).
///
/// # Errors
///
/// Meta parse failure, OOB, or field-width overflow.
pub fn patch_ctrl_from_meta_io_paddrs(
    ctrl: &mut [u8],
    ctrl_meta_json: &str,
    ifm_paddr: u64,
    wts_paddr: u64,
    ofm_paddr: u64,
) -> Result<(), DirectError> {
    use crate::backends::amd_xdna2::mladf_pack::{
        CtrlPatchBufferRole, parse_mladf_ctrl_patch_locations,
    };
    let entries = parse_mladf_ctrl_patch_locations(ctrl_meta_json)?;
    for e in entries {
        let paddr = match e.role {
            CtrlPatchBufferRole::Ifm => ifm_paddr,
            CtrlPatchBufferRole::Wts => wts_paddr,
            CtrlPatchBufferRole::Ofm => ofm_paddr,
            CtrlPatchBufferRole::Ctrl | CtrlPatchBufferRole::Other => continue,
        };
        for loc in &e.locations {
            apply_ctrl_patch_location(ctrl, loc.offset, loc.size, &loc.operation, paddr)?;
        }
    }
    Ok(())
}

/// Resolve pack mode + create/fill I/O BOs for lab.
///
/// - [`LabPackMode::F32Ones`]: existing ones f32 path (`m,n,k` + `dtype` used).
/// - [`LabPackMode::MladfA16W4`]: uses env key/meta + optional packed WTS.
///   Timed `m,n,k` are ignored for BO sizes unless `require_dims_match`.
///   **`dtype` is ignored for fill** (always bf16 IFM / packed WTS); non-empty
///   dtype must still be `f32` / `float32` / `bf16` / `a16` / empty for honesty
///   (refuse silent wrong-type claims).
///
/// # Errors
///
/// Pack-mode resolve, dim mismatch, dtype refuse, or BO failures.
pub fn create_and_fill_lab_io_bos_for_pack_mode(
    submission: &dyn DirectSubmission,
    mode: LabPackMode,
    m: u32,
    n: u32,
    k: u32,
    dtype: &str,
    require_dims_match: bool,
) -> Result<(IoBoSet, Option<MladfBufferPlan>), DirectError> {
    match mode {
        LabPackMode::F32Ones => {
            let ios = create_and_fill_ones_io_bos(submission, m, n, k, dtype)?;
            Ok((ios, None))
        }
        LabPackMode::MladfA16W4 => {
            check_mladf_pack_dtype(dtype)?;
            let plan = mladf_buffer_plan_from_env()?;
            if require_dims_match && (m != plan.shape.m || n != plan.shape.n || k != plan.shape.k) {
                return Err(DirectError::InvalidArgument(format!(
                    "XDNA2_LAB_PACK=mladf_a16w4: timed dims m,n,k=({m},{n},{k}) != \
                     kernel shape M,N,K=({},{},{}) from key {:?}",
                    plan.shape.m, plan.shape.n, plan.shape.k, plan.key
                )));
            }
            let wts = load_wts_packed_from_env(Some(plan.wts_bytes))?;
            // When Pass readiness is required, refuse missing packs early (before BO
            // alloc). Zero file still fails via check_mladf_wts_not_all_zero_for_exec
            // unless XDNA2_LAB_ALLOW_ZERO_WTS=1 (plumbing only).
            if lab_require_non_zero_wts_env() && wts.is_none() {
                return Err(DirectError::InvalidArgument(format!(
                    "XDNA2_LAB_REQUIRE_NONZERO_WTS=1 but XDNA2_LAB_WTS_PACKED unset \
                     (export via tools/export_mladf_wts.sh; expect {} B non-zero)",
                    plan.wts_bytes
                )));
            }
            if let Some(bytes) = wts.as_deref()
                && lab_require_non_zero_wts_env()
            {
                // Length already enforced by load_wts_packed_from_env.
                check_mladf_wts_not_all_zero_for_exec(bytes)?;
            }
            let ctrl = load_ctrl_packed_from_env(if plan.ctrl_bytes > 0 {
                Some(plan.ctrl_bytes)
            } else {
                None
            })?;
            // REQUIRE_CTRL early refuse (same precedence style as REQUIRE_NONZERO_WTS).
            if lab_require_ctrl_env() && plan.ctrl_bytes > 0 && ctrl.is_none() {
                return Err(DirectError::InvalidArgument(format!(
                    "XDNA2_LAB_REQUIRE_CTRL=1 but XDNA2_LAB_CTRL_PACKED unset \
                     (stage via tools/stage_silicon_lab_assets.sh; expect {} B)",
                    plan.ctrl_bytes
                )));
            }
            if let Some(bytes) = ctrl.as_deref()
                && lab_require_ctrl_env()
            {
                // Length already enforced by load when plan.ctrl_bytes > 0;
                // Pass readiness also refuses empty/all-zero.
                let expect = if plan.ctrl_bytes > 0 {
                    plan.ctrl_bytes
                } else {
                    bytes.len() as u64
                };
                validate_mladf_ctrl_packed_for_pass(bytes, expect)?;
            }
            let ios = create_and_fill_mladf_io_bos_ex(
                submission,
                &plan,
                None,
                wts.as_deref(),
                ctrl.as_deref(),
            )?;
            Ok((ios, Some(plan)))
        }
    }
}

/// Acceptable timed `dtype` labels for mladf pack fill (dtype does not change pack).
///
/// # Errors
///
/// Unsupported dtype string.
pub fn check_mladf_pack_dtype(dtype: &str) -> Result<(), DirectError> {
    let t = dtype.trim();
    if t.is_empty()
        || t.eq_ignore_ascii_case("f32")
        || t.eq_ignore_ascii_case("float32")
        || t.eq_ignore_ascii_case("bf16")
        || t.eq_ignore_ascii_case("bfloat16")
        || t.eq_ignore_ascii_case("a16")
        || t.eq_ignore_ascii_case("a16w4")
    {
        return Ok(());
    }
    Err(DirectError::InvalidArgument(format!(
        "XDNA2_LAB_PACK=mladf_a16w4: dtype {dtype:?} not accepted (use f32|bf16|a16; \
         fill always uses bf16 IFM + packed WTS sizes — dtype is documentation only)"
    )))
}

/// Decode OFM for lab: f32 LE (ones path) or bf16 LE (mladf path).
///
/// # Errors
///
/// Decode failure.
pub fn lab_decode_ofm_bytes(mode: LabPackMode, ofm_bytes: &[u8]) -> Result<Vec<f32>, DirectError> {
    match mode {
        LabPackMode::F32Ones => f32_slice_from_le_bytes(ofm_bytes),
        LabPackMode::MladfA16W4 => unpack_ofm_bf16_to_f32(ofm_bytes),
    }
}

// ---------------------------------------------------------------------------
// Weight EXEC bind (live IFM/WTS/OFM for model WTS — not throwaway upload)
// ---------------------------------------------------------------------------

/// Host payload + dims for creating a live lab I/O bind (model WTS path).
///
/// Does **not** submit EXEC. Caller must [`destroy_weight_exec_bind`].
#[derive(Debug, Clone)]
pub struct WeightExecFillRequest {
    /// Matmul M (rows of IFM / OFM).
    pub m: u32,
    /// Matmul N (cols of WTS / OFM).
    pub n: u32,
    /// Matmul K (cols of IFM / rows of WTS).
    pub k: u32,
    /// Lab dtype (`f32` / `float32` only for MVP).
    pub dtype: String,
    /// IFM host bytes (must equal [`lab_matmul_ifm_nbytes`]).
    pub ifm_bytes: Vec<u8>,
    /// WTS host bytes (must equal [`lab_matmul_wts_nbytes`]; model weight f32 LE).
    pub wts_bytes: Vec<u8>,
    /// Diagnostic name for IFM (e.g. `"ifm"` or activation tensor name).
    pub ifm_name: String,
    /// Diagnostic name for WTS (initializer name).
    pub wts_name: String,
    /// Diagnostic name for OFM.
    pub ofm_name: String,
}

/// Live Shmem BO bind for lab EXEC (IFM / WTS / OFM). **Not** a Pass claim.
///
/// Unlike [`super::host_graph::upload_weights_to_shmem_bos`], handles remain
/// live until [`destroy_weight_exec_bind`]. Pin via [`IoBoSet::arg_bo_handles`]
/// / [`build_lab_arg_bo_handles`].
///
/// **Not RAII:** `Drop` does **not** free BOs (no submission handle stored). Always
/// call [`destroy_weight_exec_bind`] (or [`destroy_io_bos`] on `.io`) on every exit
/// path. Moving `.io` out of the bind transfers ownership to the caller.
#[derive(Debug, Clone)]
pub struct WeightExecBind {
    /// Underlying IFM/WTS/OFM handles + paddrs.
    pub io: IoBoSet,
    /// IFM role name (diagnostics only).
    pub ifm_name: String,
    /// WTS initializer / role name.
    pub wts_name: String,
    /// OFM role name.
    pub ofm_name: String,
    /// Logical matmul M.
    pub m: u32,
    /// Logical matmul N.
    pub n: u32,
    /// Logical matmul K.
    pub k: u32,
    /// Host payload bytes written to IFM (may be &lt; page-rounded BO size).
    pub ifm_payload_nbytes: u64,
    /// Host payload bytes written to WTS.
    pub wts_payload_nbytes: u64,
    /// Logical OFM payload bytes (host-zeroed to full plan size).
    pub ofm_payload_nbytes: u64,
}

impl WeightExecBind {
    /// EXEC_CMD pin list: IFM, WTS, OFM (same order as [`IoBoSet::arg_bo_handles`]).
    #[must_use]
    pub fn arg_bo_handles(&self) -> Vec<u32> {
        self.io.arg_bo_handles()
    }
}

/// Create IFM/WTS/OFM, fill IFM+WTS from host bytes, **explicit host-zero OFM**.
///
/// Reuses [`create_io_bos`] / [`fill_ifm_wts`] / [`zero_ofm`]. Never EXEC.
///
/// # Errors
///
/// Size plan mismatch vs host payload lengths, create/fill/zero failures.
pub fn create_and_fill_weight_io_bos(
    submission: &dyn DirectSubmission,
    req: &WeightExecFillRequest,
) -> Result<WeightExecBind, DirectError> {
    let plan = lab_io_size_plan(req.m, req.n, req.k, &req.dtype)?;
    if req.ifm_bytes.len() as u64 != plan.ifm_bytes {
        return Err(DirectError::InvalidArgument(format!(
            "create_and_fill_weight_io_bos: IFM host {} != plan {} (m={} k={})",
            req.ifm_bytes.len(),
            plan.ifm_bytes,
            req.m,
            req.k
        )));
    }
    if req.wts_bytes.len() as u64 != plan.wts_bytes {
        return Err(DirectError::InvalidArgument(format!(
            "create_and_fill_weight_io_bos: WTS host {} != plan {} (k={} n={})",
            req.wts_bytes.len(),
            plan.wts_bytes,
            req.k,
            req.n
        )));
    }
    let ios = create_io_bos(submission, plan.ifm_bytes, plan.wts_bytes, plan.ofm_bytes)?;
    if let Err(e) = fill_ifm_wts(submission, &ios, &req.ifm_bytes, &req.wts_bytes) {
        destroy_io_bos(submission, &ios);
        return Err(e);
    }
    if let Err(e) = zero_ofm(submission, &ios) {
        destroy_io_bos(submission, &ios);
        return Err(e);
    }
    Ok(WeightExecBind {
        io: ios,
        ifm_name: req.ifm_name.clone(),
        wts_name: req.wts_name.clone(),
        ofm_name: req.ofm_name.clone(),
        m: req.m,
        n: req.n,
        k: req.k,
        ifm_payload_nbytes: plan.ifm_bytes,
        wts_payload_nbytes: plan.wts_bytes,
        ofm_payload_nbytes: plan.ofm_bytes,
    })
}

/// Best-effort destroy of a [`WeightExecBind`] (delegates to [`destroy_io_bos`]).
pub fn destroy_weight_exec_bind(submission: &dyn DirectSubmission, bind: &WeightExecBind) {
    destroy_io_bos(submission, &bind.io);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::amd_xdna2::direct::StubDirectSubmission;

    #[test]
    fn f32_le_roundtrip() {
        let v = [1.0_f32, -0.0, 3.5, f32::NAN, f32::INFINITY];
        let bytes = f32_slice_to_le_bytes(&v);
        assert_eq!(bytes.len(), 20);
        let back = f32_slice_from_le_bytes(&bytes).expect("decode");
        assert_eq!(back.len(), 5);
        assert!((back[0] - 1.0).abs() < f32::EPSILON);
        assert_eq!(back[1].to_bits(), (-0.0_f32).to_bits());
        assert!((back[2] - 3.5).abs() < f32::EPSILON);
        assert!(back[3].is_nan());
        assert!(back[4].is_infinite());
    }

    #[test]
    fn f32_from_le_empty_ok() {
        let v = f32_slice_from_le_bytes(&[]).expect("empty");
        assert_eq!(v, [] as [f32; 0]);
        assert_eq!(f32_slice_to_le_bytes(&[]), [] as [u8; 0]);
    }

    #[test]
    fn f32_from_le_odd_len_errs() {
        let err = f32_slice_from_le_bytes(&[0, 1, 2]).expect_err("odd");
        assert!(err.to_string().contains("multiple of 4"), "err={err}");
    }

    #[test]
    fn lab_nbytes_1x1x1_f32() {
        assert_eq!(lab_matmul_ifm_nbytes(1, 1, "f32").unwrap(), 4);
        assert_eq!(lab_matmul_wts_nbytes(1, 1, "f32").unwrap(), 4);
        assert_eq!(lab_matmul_ofm_nbytes(1, 1, "f32").unwrap(), 4);
        let p = lab_io_size_plan(2, 3, 4, "f32").unwrap();
        assert_eq!(p.ifm_bytes, 2 * 4 * 4);
        assert_eq!(p.wts_bytes, 4 * 3 * 4);
        assert_eq!(p.ofm_bytes, 2 * 3 * 4);
    }

    #[test]
    fn lab_nbytes_accepts_float32_alias() {
        assert_eq!(lab_matmul_ifm_nbytes(2, 2, "float32").unwrap(), 16);
        assert_eq!(lab_matmul_wts_nbytes(2, 2, "FLOAT32").unwrap(), 16);
        assert_eq!(lab_matmul_ofm_nbytes(2, 2, "Float32").unwrap(), 16);
        let p = lab_io_size_plan(1, 1, 1, "float32").unwrap();
        assert_eq!(p.ofm_bytes, 4);
    }

    #[test]
    fn lab_nbytes_refuses_zero_and_bad_dtype() {
        assert!(lab_matmul_ofm_nbytes(0, 1, "f32").is_err());
        assert!(lab_matmul_ifm_nbytes(1, 0, "f32").is_err());
        assert!(lab_matmul_wts_nbytes(1, 1, "bf16").is_err());
    }

    #[test]
    fn lab_nbytes_refuses_over_cap() {
        // 2001*2000 > HOST_REF_MAX_ELEMENTS
        let err = lab_matmul_ofm_nbytes(2001, 2000, "f32").expect_err("cap");
        assert!(
            err.to_string().contains("refuses") || err.to_string().contains("max"),
            "err={err}"
        );
    }

    #[test]
    fn host_ones_matrix_bytes_len() {
        let b = host_ones_f32_matrix_bytes(2, 3).expect("ones");
        assert_eq!(b.len(), 24);
        let v = f32_slice_from_le_bytes(&b).unwrap();
        assert!(v.iter().all(|&x| (x - 1.0).abs() < 1e-6));
    }

    #[test]
    fn u64_to_prop_dwords_split() {
        assert_eq!(
            u64_to_prop_dwords(0x1111_2222_3333_4444),
            [0x3333_4444, 0x1111_2222]
        );
        assert_eq!(u64_to_prop_dwords(0), [0, 0]);
    }

    #[test]
    fn lab_kernel_arg_layout_from_str_truth_table() {
        assert_eq!(
            try_lab_kernel_arg_layout_from_str(None).unwrap(),
            LabKernelArgLayout::Count4Ctrl
        );
        assert_eq!(
            try_lab_kernel_arg_layout_from_str(Some("")).unwrap(),
            LabKernelArgLayout::Count4Ctrl
        );
        assert_eq!(
            try_lab_kernel_arg_layout_from_str(Some("count4")).unwrap(),
            LabKernelArgLayout::Count4Ctrl
        );
        assert_eq!(
            try_lab_kernel_arg_layout_from_str(Some("xrt3")).unwrap(),
            LabKernelArgLayout::Xrt3
        );
        assert_eq!(
            try_lab_kernel_arg_layout_from_str(Some("3")).unwrap(),
            LabKernelArgLayout::Xrt3
        );
        assert_eq!(
            try_lab_kernel_arg_layout_from_str(Some("CLASSIC")).unwrap(),
            LabKernelArgLayout::Xrt3
        );
        // Fail-closed: typos must not silently become count4.
        assert!(try_lab_kernel_arg_layout_from_str(Some("nope")).is_err());
        assert!(try_lab_kernel_arg_layout_from_str(Some("xrt")).is_err());
        assert!(try_lab_kernel_arg_layout_from_str(Some("xrt-3")).is_err());
        // Legacy convenience still defaults unknown for pure helpers.
        assert_eq!(
            lab_kernel_arg_layout_from_str(Some("nope")),
            LabKernelArgLayout::Count4Ctrl
        );
    }

    #[test]
    fn pack_kernel_args_partial_elf_layout() {
        let args = pack_start_npu_kernel_args_with_ctrl_layout(
            0xa,
            0xb,
            0xc,
            None,
            LabKernelArgLayout::Count4Ctrl,
        );
        // 2+2+1+2+2+2+2+2 = 15 dwords
        assert_eq!(args.len(), 15);
        assert_eq!(args[0], 3);
        let with_ctrl = pack_start_npu_kernel_args_with_ctrl_layout(
            0xa,
            0xb,
            0xc,
            Some(0xd),
            LabKernelArgLayout::Count4Ctrl,
        );
        assert_eq!(with_ctrl.len(), 15);
        assert_eq!(with_ctrl[0], 4); // num_data_bos
        assert_eq!(with_ctrl[11], 0xd); // ctrl lo (slot after ofm)
        // XRT classic 3-arg: n_bos=3, two trailing zeros (ctrl not in exec-buf).
        let xrt3 = pack_start_npu_kernel_args_with_ctrl_layout(
            0xa,
            0xb,
            0xc,
            Some(0xd),
            LabKernelArgLayout::Xrt3,
        );
        assert_eq!(xrt3.len(), 15);
        assert_eq!(xrt3[0], 3);
        assert_eq!(xrt3[5], 0xa);
        assert_eq!(xrt3[7], 0xb);
        assert_eq!(xrt3[9], 0xc);
        assert_eq!(xrt3[11], 0); // not ctrl
        assert_eq!(xrt3[13], 0);
        assert_eq!(args[1], 0);
        assert_eq!(args[2], 0);
        assert_eq!(args[3], 0);
        assert_eq!(args[4], 0);
        assert_eq!(args[5], 0xa); // ifm lo
        assert_eq!(args[6], 0);
        assert_eq!(args[7], 0xb);
        assert_eq!(args[8], 0);
        assert_eq!(args[9], 0xc);
        assert_eq!(args[10], 0);
        assert_eq!(args[11], 0);
        assert_eq!(args[12], 0);
        assert_eq!(args[13], 0);
        assert_eq!(args[14], 0);
        let no_io = pack_start_npu_kernel_args_no_io();
        assert_eq!(no_io.len(), 15);
        assert_eq!(no_io[5], 0);
    }

    #[test]
    fn lab_ofm_env_value_truth_table() {
        assert!(!lab_ofm_env_value_enabled(None));
        assert!(!lab_ofm_env_value_enabled(Some("")));
        assert!(!lab_ofm_env_value_enabled(Some("0")));
        assert!(!lab_ofm_env_value_enabled(Some("false")));
        assert!(!lab_ofm_env_value_enabled(Some("no")));
        assert!(lab_ofm_env_value_enabled(Some("1")));
        assert!(lab_ofm_env_value_enabled(Some("true")));
        assert!(lab_ofm_env_value_enabled(Some("TRUE")));
        assert!(lab_ofm_env_value_enabled(Some(" 1 ")));
        // Live env must match pure parser (no mutation).
        assert_eq!(
            lab_ofm_env_enabled(),
            lab_ofm_env_value_enabled(std::env::var("XDNA2_LAB_OFM").ok().as_deref())
        );
    }

    #[test]
    fn lab_decide_matching_ofm_all_flags() {
        let golden = host_matmul_f32_ones_vec(1, 2, 3).unwrap();
        let ofm = f32_slice_to_le_bytes(&golden);
        let plan = LabOfmVerifyPlan {
            m: 1,
            n: 2,
            k: 3,
            used_inst_blob: true,
            submit_wait_ok: true,
            real_ioctl: true,
            ofm_bytes: ofm,
            criteria: NumericalVerifyCriteria::default(),
        };
        let (ok, reason) = lab_decide_from_ofm_bytes(&plan).unwrap();
        assert!(ok, "reason={reason:?}");
        assert!(reason.is_none());
    }

    #[test]
    fn lab_decide_empty_ofm() {
        let plan = LabOfmVerifyPlan {
            m: 1,
            n: 1,
            k: 1,
            used_inst_blob: true,
            submit_wait_ok: true,
            real_ioctl: true,
            ofm_bytes: vec![],
            criteria: NumericalVerifyCriteria::default(),
        };
        let (ok, reason) = lab_decide_from_ofm_bytes(&plan).unwrap();
        assert!(!ok);
        assert_eq!(reason, Some(RanOnNpuBlockReason::EmptyBuffers));
    }

    #[test]
    fn lab_decide_zeros_vs_ones_allclose_fail() {
        let ofm = f32_slice_to_le_bytes(&[0.0_f32]);
        let plan = LabOfmVerifyPlan {
            m: 1,
            n: 1,
            k: 2,
            used_inst_blob: true,
            submit_wait_ok: true,
            real_ioctl: true,
            ofm_bytes: ofm,
            criteria: NumericalVerifyCriteria::default(),
        };
        let (ok, reason) = lab_decide_from_ofm_bytes(&plan).unwrap();
        assert!(!ok);
        assert_eq!(reason, Some(RanOnNpuBlockReason::AllcloseFailed));
    }

    #[test]
    fn lab_decide_missing_blob() {
        let golden = host_matmul_f32_ones_vec(1, 1, 1).unwrap();
        let plan = LabOfmVerifyPlan {
            m: 1,
            n: 1,
            k: 1,
            used_inst_blob: false,
            submit_wait_ok: true,
            real_ioctl: true,
            ofm_bytes: f32_slice_to_le_bytes(&golden),
            criteria: NumericalVerifyCriteria::default(),
        };
        let (ok, reason) = lab_decide_from_ofm_bytes(&plan).unwrap();
        assert!(!ok);
        assert_eq!(reason, Some(RanOnNpuBlockReason::MissingInstructionBlob));
    }

    #[test]
    fn lab_decide_not_real_ioctl() {
        let golden = host_matmul_f32_ones_vec(1, 1, 1).unwrap();
        let plan = LabOfmVerifyPlan {
            m: 1,
            n: 1,
            k: 1,
            used_inst_blob: true,
            submit_wait_ok: true,
            real_ioctl: false,
            ofm_bytes: f32_slice_to_le_bytes(&golden),
            criteria: NumericalVerifyCriteria::default(),
        };
        let (ok, reason) = lab_decide_from_ofm_bytes(&plan).unwrap();
        assert!(!ok);
        assert_eq!(reason, Some(RanOnNpuBlockReason::NotRealIoctlPath));
    }

    #[test]
    fn lab_decide_submit_not_ok() {
        let golden = host_matmul_f32_ones_vec(1, 1, 1).unwrap();
        let plan = LabOfmVerifyPlan {
            m: 1,
            n: 1,
            k: 1,
            used_inst_blob: true,
            submit_wait_ok: false,
            real_ioctl: true,
            ofm_bytes: f32_slice_to_le_bytes(&golden),
            criteria: NumericalVerifyCriteria::default(),
        };
        let (ok, reason) = lab_decide_from_ofm_bytes(&plan).unwrap();
        assert!(!ok);
        assert_eq!(reason, Some(RanOnNpuBlockReason::SubmitWaitNotOk));
    }

    #[test]
    fn lab_decide_len_mismatch() {
        // OFM has 2 f32 but golden is 1×1
        let plan = LabOfmVerifyPlan {
            m: 1,
            n: 1,
            k: 1,
            used_inst_blob: true,
            submit_wait_ok: true,
            real_ioctl: true,
            ofm_bytes: f32_slice_to_le_bytes(&[1.0, 2.0]),
            criteria: NumericalVerifyCriteria::default(),
        };
        let (ok, reason) = lab_decide_from_ofm_bytes(&plan).unwrap();
        assert!(!ok);
        assert_eq!(reason, Some(RanOnNpuBlockReason::AllcloseFailed));
    }

    #[test]
    fn lab_decide_nan_ofm() {
        let plan = LabOfmVerifyPlan {
            m: 1,
            n: 1,
            k: 1,
            used_inst_blob: true,
            submit_wait_ok: true,
            real_ioctl: true,
            ofm_bytes: f32_slice_to_le_bytes(&[f32::NAN]),
            criteria: NumericalVerifyCriteria::default(),
        };
        let (ok, reason) = lab_decide_from_ofm_bytes(&plan).unwrap();
        assert!(!ok);
        assert_eq!(reason, Some(RanOnNpuBlockReason::AllcloseFailed));
    }

    #[test]
    fn stub_create_fill_readback_io_bos() {
        let stub = StubDirectSubmission::new(-1);
        let ios = create_and_fill_ones_io_bos(&stub, 1, 1, 2, "f32").expect("create");
        assert_ne!(ios.ifm_addr, 0);
        assert_ne!(ios.wts_addr, 0);
        assert_ne!(ios.ofm_addr, 0);
        assert_eq!(ios.arg_bo_handles().len(), 3);
        // Explicit host-zero OFM before use (create_and_fill always zeros)
        let ofm = readback_ofm(&stub, &ios).expect("read");
        assert_eq!(ofm.len(), 4);
        assert_eq!(ofm, [0, 0, 0, 0]);
        // Synthetic: write golden to OFM store and decide (stub still !real)
        let golden = host_matmul_f32_ones_vec(1, 1, 2).unwrap();
        let gbytes = f32_slice_to_le_bytes(&golden);
        stub.write_bo_bytes(0, ios.ofm.0, 0, &gbytes).unwrap();
        let ofm2 = readback_ofm(&stub, &ios).unwrap();
        let plan = LabOfmVerifyPlan {
            m: 1,
            n: 1,
            k: 2,
            used_inst_blob: true,
            submit_wait_ok: true,
            real_ioctl: false, // stub
            ofm_bytes: ofm2,
            criteria: NumericalVerifyCriteria::default(),
        };
        let (ok, reason) = lab_decide_from_ofm_bytes(&plan).unwrap();
        assert!(!ok);
        assert_eq!(reason, Some(RanOnNpuBlockReason::NotRealIoctlPath));
        destroy_io_bos(&stub, &ios);
    }

    #[test]
    fn zero_ofm_overwrites_dirty_ofm() {
        let stub = StubDirectSubmission::new(-1);
        let ios = create_io_bos(&stub, 4, 4, 8).expect("create");
        stub.write_bo_bytes(0, ios.ofm.0, 0, &[0xff; 8]).unwrap();
        zero_ofm(&stub, &ios).expect("zero");
        let ofm = readback_ofm(&stub, &ios).unwrap();
        assert_eq!(ofm, vec![0u8; 8]);
        destroy_io_bos(&stub, &ios);
    }

    #[test]
    fn create_io_bos_refuses_zero_sizes() {
        let stub = StubDirectSubmission::new(-1);
        assert!(create_io_bos(&stub, 0, 4, 4).is_err());
        assert!(create_io_bos(&stub, 4, 0, 4).is_err());
        assert!(create_io_bos(&stub, 4, 4, 0).is_err());
    }

    #[test]
    fn fill_ifm_wts_refuses_empty() {
        let stub = StubDirectSubmission::new(-1);
        let ios = create_io_bos(&stub, 4, 4, 4).expect("create");
        assert!(fill_ifm_wts(&stub, &ios, &[], &[0; 4]).is_err());
        assert!(fill_ifm_wts(&stub, &ios, &[0; 4], &[]).is_err());
        destroy_io_bos(&stub, &ios);
    }

    #[test]
    fn fill_ifm_wts_refuses_oversize_and_undersize_host() {
        let stub = StubDirectSubmission::new(-1);
        let ios = create_io_bos(&stub, 4, 4, 4).expect("create");
        let big = [0u8; 8];
        let err = fill_ifm_wts(&stub, &ios, &big, &[0; 4]).expect_err("ifm oversize");
        assert!(err.to_string().contains("IFM"), "err={err}");
        let err2 = fill_ifm_wts(&stub, &ios, &[0; 4], &big).expect_err("wts oversize");
        assert!(err2.to_string().contains("WTS"), "err={err2}");
        // Undersize also refused (exact length).
        let err3 = fill_ifm_wts(&stub, &ios, &[0; 2], &[0; 4]).expect_err("ifm under");
        assert!(err3.to_string().contains("IFM"), "err={err3}");
        let err4 = fill_ifm_wts(&stub, &ios, &[0; 4], &[0; 2]).expect_err("wts under");
        assert!(err4.to_string().contains("WTS"), "err={err4}");
        destroy_io_bos(&stub, &ios);
    }

    #[test]
    fn lab_decide_with_golden_refuses_wrong_len() {
        let plan = LabOfmVerifyPlan {
            m: 1,
            n: 2,
            k: 1,
            used_inst_blob: true,
            submit_wait_ok: true,
            real_ioctl: true,
            ofm_bytes: f32_slice_to_le_bytes(&[0.0, 0.0]),
            criteria: NumericalVerifyCriteria::default(),
        };
        let err = lab_decide_from_ofm_bytes_with_golden(&plan, &[1.0]).expect_err("golden len");
        assert!(err.to_string().contains("m*n"), "err={err}");
    }

    #[test]
    fn lab_decide_odd_ofm_bytes_errs() {
        let plan = LabOfmVerifyPlan {
            m: 1,
            n: 1,
            k: 1,
            used_inst_blob: true,
            submit_wait_ok: true,
            real_ioctl: true,
            ofm_bytes: vec![0, 1, 2], // not multiple of 4
            criteria: NumericalVerifyCriteria::default(),
        };
        let err = lab_decide_from_ofm_bytes(&plan).expect_err("odd ofm");
        assert!(err.to_string().contains("multiple of 4"), "err={err}");
    }

    #[test]
    fn lab_decide_bool_wrapper() {
        let golden = host_matmul_f32_ones_vec(1, 1, 1).unwrap();
        let plan = LabOfmVerifyPlan {
            m: 1,
            n: 1,
            k: 1,
            used_inst_blob: true,
            submit_wait_ok: true,
            real_ioctl: true,
            ofm_bytes: f32_slice_to_le_bytes(&golden),
            criteria: NumericalVerifyCriteria::default(),
        };
        assert!(lab_decide_from_ofm_bytes_bool(&plan).unwrap());
        let bad = LabOfmVerifyPlan {
            ofm_bytes: f32_slice_to_le_bytes(&[0.0]),
            ..plan
        };
        assert!(!lab_decide_from_ofm_bytes_bool(&bad).unwrap());
    }

    #[test]
    fn lab_decide_with_custom_golden_pass_fail() {
        // Custom golden [3.0] vs matching OFM → allclose ok when all flags true.
        let golden = vec![3.0_f32];
        let plan = LabOfmVerifyPlan {
            m: 1,
            n: 1,
            k: 2,
            used_inst_blob: true,
            submit_wait_ok: true,
            real_ioctl: true,
            ofm_bytes: f32_slice_to_le_bytes(&golden),
            criteria: NumericalVerifyCriteria::default(),
        };
        let (ok, reason) = lab_decide_from_ofm_bytes_with_golden(&plan, &golden).unwrap();
        assert!(ok, "reason={reason:?}");
        // All-zero golden is product-refused before allclose (not vacuous Pass).
        let (bad, r2) = lab_decide_from_ofm_bytes_with_golden(&plan, &[0.0_f32]).unwrap();
        assert!(!bad);
        assert_eq!(r2, Some(RanOnNpuBlockReason::AllZeroGolden));
        // Non-zero golden mismatch still AllcloseFailed.
        let (bad2, r3) = lab_decide_from_ofm_bytes_with_golden(&plan, &[9.0_f32]).unwrap();
        assert!(!bad2);
        assert_eq!(r3, Some(RanOnNpuBlockReason::AllcloseFailed));
    }

    #[test]
    fn create_and_fill_weight_io_bos_stub_fills_wts_not_ones() {
        let stub = StubDirectSubmission::new(-1);
        // k=2,n=1 → WTS = [2.0, 3.0] (not ones)
        let wts = f32_slice_to_le_bytes(&[2.0, 3.0]);
        let ifm = f32_slice_to_le_bytes(&[1.0, 1.0]); // m=1,k=2 ones
        let req = WeightExecFillRequest {
            m: 1,
            n: 1,
            k: 2,
            dtype: "f32".into(),
            ifm_bytes: ifm,
            wts_bytes: wts.clone(),
            ifm_name: "ifm".into(),
            wts_name: "W".into(),
            ofm_name: "ofm".into(),
        };
        let bind = create_and_fill_weight_io_bos(&stub, &req).expect("create weight bind");
        assert_eq!(bind.wts_name, "W");
        assert_eq!(bind.m, 1);
        assert_eq!(bind.n, 1);
        assert_eq!(bind.k, 2);
        assert_eq!(bind.wts_payload_nbytes, 8);
        // WTS store holds model bytes, not ones
        let wts_rb = stub
            .read_bo_bytes_no_sync(0, bind.io.wts.0, 0, 8)
            .expect("read wts");
        assert_eq!(wts_rb, wts);
        let ones = host_ones_f32_matrix_bytes(2, 1).unwrap();
        assert_ne!(wts_rb, ones);
        // OFM zeroed
        let ofm = readback_ofm(&stub, &bind.io).unwrap();
        assert_eq!(ofm, vec![0u8; 4]);
        // Pin order IFM,WTS,OFM
        assert_eq!(
            bind.arg_bo_handles(),
            vec![bind.io.ifm.0, bind.io.wts.0, bind.io.ofm.0]
        );
        assert_eq!(
            build_lab_arg_bo_handles(
                Some((bind.io.ifm.0, bind.io.wts.0, bind.io.ofm.0)),
                Some(99)
            ),
            vec![bind.io.ifm.0, bind.io.wts.0, bind.io.ofm.0, 99]
        );
        destroy_weight_exec_bind(&stub, &bind);
        // Idempotent best-effort second destroy
        destroy_weight_exec_bind(&stub, &bind);
    }

    #[test]
    fn create_and_fill_weight_io_bos_refuses_size_mismatch() {
        let stub = StubDirectSubmission::new(-1);
        let req = WeightExecFillRequest {
            m: 1,
            n: 1,
            k: 2,
            dtype: "f32".into(),
            ifm_bytes: vec![0u8; 4], // wrong: need 8
            wts_bytes: f32_slice_to_le_bytes(&[1.0, 1.0]),
            ifm_name: "ifm".into(),
            wts_name: "W".into(),
            ofm_name: "ofm".into(),
        };
        let err = create_and_fill_weight_io_bos(&stub, &req).expect_err("ifm mismatch");
        assert!(err.to_string().contains("IFM"), "err={err}");
        let req2 = WeightExecFillRequest {
            ifm_bytes: f32_slice_to_le_bytes(&[1.0, 1.0]),
            wts_bytes: vec![0u8; 4], // wrong: need 8
            ..req
        };
        let err2 = create_and_fill_weight_io_bos(&stub, &req2).expect_err("wts mismatch");
        assert!(err2.to_string().contains("WTS"), "err={err2}");
    }

    #[test]
    fn pack_kernel_args_uses_weight_paddrs() {
        let args = pack_start_npu_kernel_args(0x1111, 0x2222, 0x3333);
        assert_eq!(&args[5..7], &u64_to_prop_dwords(0x1111));
        assert_eq!(&args[7..9], &u64_to_prop_dwords(0x2222));
        assert_eq!(&args[9..11], &u64_to_prop_dwords(0x3333));
    }

    #[test]
    fn host_matmul_ones_ifm_with_wts_golden() {
        // A=ones 1x2, B=[[2,3],[4,5]] → C=[6,8]
        let wts = [2.0_f32, 3.0, 4.0, 5.0];
        let g = host_matmul_f32_ones_ifm_with_wts(1, 2, 2, &wts).unwrap();
        assert_eq!(g, vec![6.0, 8.0]);
        let g2 = host_matmul_f32_ones_ifm_with_wts(2, 2, 2, &wts).unwrap();
        assert_eq!(g2, vec![6.0, 8.0, 6.0, 8.0]);
        assert!(host_matmul_f32_ones_ifm_with_wts(1, 2, 2, &[1.0]).is_err());
    }

    #[test]
    fn lab_decide_model_wts_golden_matches_helper() {
        let wts = [2.0_f32, 0.0, 0.0, 3.0]; // k=2,n=2
        let golden = host_matmul_f32_ones_ifm_with_wts(1, 2, 2, &wts).unwrap();
        // ones IFM → C = [2, 3]
        assert_eq!(golden, vec![2.0, 3.0]);
        let plan = LabOfmVerifyPlan {
            m: 1,
            n: 2,
            k: 2,
            used_inst_blob: true,
            submit_wait_ok: true,
            real_ioctl: true,
            ofm_bytes: f32_slice_to_le_bytes(&golden),
            criteria: NumericalVerifyCriteria::default(),
        };
        let (ok, _) = lab_decide_from_ofm_bytes_with_golden(&plan, &golden).unwrap();
        assert!(ok);
        // ones golden would fail for same OFM
        let (ones_ok, reason) = lab_decide_from_ofm_bytes(&plan).unwrap();
        assert!(!ones_ok);
        assert_eq!(reason, Some(RanOnNpuBlockReason::AllcloseFailed));
    }

    #[test]
    fn build_lab_arg_bo_handles_inst_only_and_io_plus_inst() {
        assert_eq!(build_lab_arg_bo_handles(None, None), Vec::<u32>::new());
        assert_eq!(build_lab_arg_bo_handles(None, Some(99)), vec![99]);
        assert_eq!(
            build_lab_arg_bo_handles(Some((1, 2, 3)), None),
            vec![1, 2, 3]
        );
        assert_eq!(
            build_lab_arg_bo_handles(Some((1, 2, 3)), Some(99)),
            vec![1, 2, 3, 99]
        );
        // CTRL between I/O and instruction
        assert_eq!(
            build_lab_arg_bo_handles_ex(Some((1, 2, 3)), Some(4), Some(99)),
            vec![1, 2, 3, 4, 99]
        );
        assert_eq!(
            build_lab_arg_bo_handles_ex(Some((1, 2, 3)), Some(4), None),
            vec![1, 2, 3, 4]
        );
    }

    #[test]
    fn pack_kernel_args_with_ctrl_and_io_set() {
        let args = pack_start_npu_kernel_args_with_ctrl(0x10, 0x20, 0x30, Some(0x40));
        assert_eq!(args[0], 4); // num_data_bos
        assert_eq!(args[5], 0x10); // ifm lo
        assert_eq!(args[7], 0x20); // wts lo
        assert_eq!(args[9], 0x30); // ofm lo
        assert_eq!(args[11], 0x40); // ctrl lo
        // Stub IoBoSet-shaped pack via public helper
        let stub = StubDirectSubmission::new(-1);
        let mut ios = create_io_bos(&stub, 16, 16, 16).expect("create");
        ios.ifm_addr = 0x10;
        ios.wts_addr = 0x20;
        ios.ofm_addr = 0x30;
        assert_eq!(ios.pack_kernel_args()[0], 3);
        // Attach real ctrl bytes (16 B) — alloc padded to 4096
        attach_ctrl_bo(&stub, &mut ios, &[1u8; 16]).expect("attach");
        assert!(ios.ctrl.is_some());
        assert_eq!(ios.ctrl_bytes, 16);
        assert_eq!(ios.pack_kernel_args()[0], 4);
        assert_eq!(ios.arg_bo_handles().len(), 4);
    }

    #[test]
    fn patch_ctrl_packet_u64_le_roundtrip() {
        let mut buf = vec![0u8; 32];
        patch_ctrl_packet_u64_le(&mut buf, &[(0, 0x0102_0304_0506_0708), (8, 0xAABB)]).unwrap();
        assert_eq!(&buf[0..8], &0x0102_0304_0506_0708_u64.to_le_bytes());
        assert_eq!(&buf[8..16], &0xAABB_u64.to_le_bytes());
        let err = patch_ctrl_packet_u64_le(&mut buf, &[(28, 1)]).expect_err("oob");
        assert!(err.to_string().contains("out of range"));
    }

    #[test]
    fn apply_ctrl_patch_size6_read_add_preserves_tile_offset() {
        // Live staged lab_ctrl: OFM sites hold 0x100, WTS sites hold 0x15400.
        // read_add_write must ADD base, write only 6 bytes (not wipe offset / neighbors).
        let mut buf = vec![0xEE_u8; 16];
        // existing 6-byte LE 0x100 at offset 0; neighbors 0xEE at [6..].
        buf[0..6].copy_from_slice(&0x100_u64.to_le_bytes()[..6]);
        apply_ctrl_patch_location(&mut buf, 0, 6, "read_add_write", 0x7fc3_b8d8_b000).unwrap();
        let mut got = [0_u8; 8];
        got[..6].copy_from_slice(&buf[0..6]);
        let val = u64::from_le_bytes(got);
        assert_eq!(val, 0x7fc3_b8d8_b000 + 0x100);
        assert_eq!(buf[6], 0xEE, "must not clobber byte after size=6 field");
        assert_eq!(buf[7], 0xEE);

        // WTS-style 0x15400 + base
        let mut wts = vec![0_u8; 8];
        wts[0..6].copy_from_slice(&0x1_5400_u64.to_le_bytes()[..6]);
        apply_ctrl_patch_location(&mut wts, 0, 6, "read_add_write", 0x1000).unwrap();
        let mut g2 = [0_u8; 8];
        g2[..6].copy_from_slice(&wts[0..6]);
        assert_eq!(u64::from_le_bytes(g2), 0x1000 + 0x1_5400);

        // Overflow past 48 bits refuses.
        let mut big = vec![0_u8; 8];
        let u48max = (1_u64 << 48) - 1;
        big[0..6].copy_from_slice(&u48max.to_le_bytes()[..6]);
        let err = apply_ctrl_patch_location(&mut big, 0, 6, "read_add_write", 1).expect_err("ovf");
        assert!(
            err.to_string().contains("does not fit") || err.to_string().contains("wrap"),
            "{err}"
        );
    }

    #[test]
    #[allow(unsafe_code)]
    fn bytes_are_all_zero_and_wts_exec_check() {
        assert!(!bytes_are_all_zero(&[]));
        assert!(bytes_are_all_zero(&[0, 0, 0]));
        assert!(!bytes_are_all_zero(&[0, 1, 0]));
        check_mladf_wts_not_all_zero_for_exec(&[1, 0, 0]).unwrap();
        let _guard = lab_wts_env_test_lock();
        // Without ALLOW_ZERO_WTS, all-zero fails.
        let prev_allow = std::env::var_os("XDNA2_LAB_ALLOW_ZERO_WTS");
        // SAFETY: held under lab_wts_env_test_lock.
        unsafe {
            std::env::remove_var("XDNA2_LAB_ALLOW_ZERO_WTS");
        }
        let prev_req = std::env::var_os("XDNA2_LAB_REQUIRE_NONZERO_WTS");
        unsafe {
            std::env::remove_var("XDNA2_LAB_REQUIRE_NONZERO_WTS");
        }
        let err = check_mladf_wts_not_all_zero_for_exec(&[0, 0, 0]).expect_err("zero");
        assert!(err.to_string().contains("all zeros"));
        unsafe {
            std::env::set_var("XDNA2_LAB_ALLOW_ZERO_WTS", "1");
        }
        check_mladf_wts_not_all_zero_for_exec(&[0, 0, 0]).expect("allow without REQUIRE");
        // REQUIRE wins over ALLOW
        unsafe {
            std::env::set_var("XDNA2_LAB_REQUIRE_NONZERO_WTS", "1");
        }
        let err_req = check_mladf_wts_not_all_zero_for_exec(&[0, 0, 0]).expect_err("REQUIRE wins");
        assert!(
            err_req.to_string().contains("REQUIRE") || err_req.to_string().contains("all zeros"),
            "err={err_req}"
        );
        unsafe {
            match prev_allow {
                Some(v) => std::env::set_var("XDNA2_LAB_ALLOW_ZERO_WTS", v),
                None => std::env::remove_var("XDNA2_LAB_ALLOW_ZERO_WTS"),
            }
            match prev_req {
                Some(v) => std::env::set_var("XDNA2_LAB_REQUIRE_NONZERO_WTS", v),
                None => std::env::remove_var("XDNA2_LAB_REQUIRE_NONZERO_WTS"),
            }
        }
        // Pure pass-candidate gate (no env)
        validate_mladf_wts_packed_for_pass(&[1, 2, 3], 3).unwrap();
        assert!(
            validate_mladf_wts_packed_for_pass(&[0, 0, 0], 3)
                .unwrap_err()
                .to_string()
                .contains("all zeros")
        );
        assert!(
            validate_mladf_wts_packed_for_pass(&[1], 3)
                .unwrap_err()
                .to_string()
                .contains("len")
        );
        // Staged contract size
        let mut staged =
            vec![
                0_u8;
                usize::try_from(crate::backends::amd_xdna2::mladf_pack::STAGED_MLADF_WTS_BYTES)
                    .expect("value fits in usize")
            ];
        staged[0] = 1;
        validate_mladf_wts_packed_for_pass(
            &staged,
            crate::backends::amd_xdna2::mladf_pack::STAGED_MLADF_WTS_BYTES,
        )
        .unwrap();
        let zeros =
            vec![
                0_u8;
                usize::try_from(crate::backends::amd_xdna2::mladf_pack::STAGED_MLADF_WTS_BYTES)
                    .expect("value fits in usize")
            ];
        assert!(
            validate_mladf_wts_packed_for_pass(
                &zeros,
                crate::backends::amd_xdna2::mladf_pack::STAGED_MLADF_WTS_BYTES,
            )
            .unwrap_err()
            .to_string()
            .contains("all zeros")
        );
    }

    /// Serialize lab env mutations (REQUIRE_*/ALLOW_*/PACKED paths) across parallel tests.
    fn lab_env_test_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
        LOCK.get_or_init(|| std::sync::Mutex::new(()))
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Alias kept for call sites that only touch WTS env.
    fn lab_wts_env_test_lock() -> std::sync::MutexGuard<'static, ()> {
        lab_env_test_lock()
    }

    #[test]
    #[allow(unsafe_code)]
    fn require_nonzero_wts_env_refuses_zero_fill() {
        let _guard = lab_env_test_lock();
        let stub = StubDirectSubmission::new(-1);
        let plan = crate::backends::amd_xdna2::mladf_pack::mladf_buffer_plan_from_key(
            crate::backends::amd_xdna2::mladf_pack::STAGED_MLADF_KEY,
            None,
        )
        .expect("plan");
        let prev_req = std::env::var_os("XDNA2_LAB_REQUIRE_NONZERO_WTS");
        let prev_allow = std::env::var_os("XDNA2_LAB_ALLOW_ZERO_WTS");
        let prev_ctrl_req = std::env::var_os("XDNA2_LAB_REQUIRE_CTRL");
        // SAFETY: held under lab_env_test_lock.
        unsafe {
            std::env::set_var("XDNA2_LAB_REQUIRE_NONZERO_WTS", "1");
            std::env::remove_var("XDNA2_LAB_ALLOW_ZERO_WTS");
            // WTS-only test: do not trip ctrl REQUIRE from concurrent env.
            std::env::remove_var("XDNA2_LAB_REQUIRE_CTRL");
        }
        let err = create_and_fill_mladf_io_bos_ex(&stub, &plan, None, None, None)
            .expect_err("zero WTS with REQUIRE_NONZERO");
        assert!(
            err.to_string().contains("all zeros") || err.to_string().contains("zeros"),
            "err={err}"
        );
        // REQUIRE wins over ALLOW: still refuse zeros when both set.
        unsafe {
            std::env::set_var("XDNA2_LAB_ALLOW_ZERO_WTS", "1");
        }
        let err2 = create_and_fill_mladf_io_bos_ex(&stub, &plan, None, None, None)
            .expect_err("REQUIRE wins over ALLOW");
        assert!(
            err2.to_string().contains("REQUIRE") || err2.to_string().contains("all zeros"),
            "err={err2}"
        );
        // ALLOW alone (REQUIRE unset) → plumbing zero WTS allowed (ctrl optional).
        unsafe {
            std::env::remove_var("XDNA2_LAB_REQUIRE_NONZERO_WTS");
            std::env::set_var("XDNA2_LAB_ALLOW_ZERO_WTS", "1");
            std::env::remove_var("XDNA2_LAB_REQUIRE_CTRL");
        }
        let ios = create_and_fill_mladf_io_bos_ex(&stub, &plan, None, None, None)
            .expect("allow zero plumbing without REQUIRE");
        destroy_io_bos(&stub, &ios);
        unsafe {
            match prev_req {
                Some(v) => std::env::set_var("XDNA2_LAB_REQUIRE_NONZERO_WTS", v),
                None => std::env::remove_var("XDNA2_LAB_REQUIRE_NONZERO_WTS"),
            }
            match prev_allow {
                Some(v) => std::env::set_var("XDNA2_LAB_ALLOW_ZERO_WTS", v),
                None => std::env::remove_var("XDNA2_LAB_ALLOW_ZERO_WTS"),
            }
            match prev_ctrl_req {
                Some(v) => std::env::set_var("XDNA2_LAB_REQUIRE_CTRL", v),
                None => std::env::remove_var("XDNA2_LAB_REQUIRE_CTRL"),
            }
        }
    }

    #[test]
    #[allow(unsafe_code)]
    fn require_nonzero_wts_pack_mode_refuses_unset_wts() {
        let _guard = lab_wts_env_test_lock();
        let stub = StubDirectSubmission::new(-1);
        let prev_req = std::env::var_os("XDNA2_LAB_REQUIRE_NONZERO_WTS");
        let prev_pack = std::env::var_os("XDNA2_LAB_PACK");
        let prev_wts = std::env::var_os("XDNA2_LAB_WTS_PACKED");
        let prev_key = std::env::var_os("XDNA2_LAB_MLADF_KEY");
        let prev_meta = std::env::var_os("XDNA2_LAB_MLADF_META");
        // SAFETY: held under lab_wts_env_test_lock.
        unsafe {
            std::env::set_var("XDNA2_LAB_REQUIRE_NONZERO_WTS", "1");
            std::env::remove_var("XDNA2_LAB_WTS_PACKED");
            std::env::remove_var("XDNA2_LAB_MLADF_META");
            std::env::set_var(
                "XDNA2_LAB_MLADF_KEY",
                crate::backends::amd_xdna2::mladf_pack::STAGED_MLADF_KEY,
            );
        }
        let err = create_and_fill_lab_io_bos_for_pack_mode(
            &stub,
            LabPackMode::MladfA16W4,
            1,
            256,
            1152,
            "bf16",
            false,
        )
        .expect_err("unset WTS with REQUIRE");
        assert!(
            err.to_string().contains("XDNA2_LAB_WTS_PACKED")
                || err.to_string().contains("REQUIRE_NONZERO"),
            "err={err}"
        );
        unsafe {
            match prev_req {
                Some(v) => std::env::set_var("XDNA2_LAB_REQUIRE_NONZERO_WTS", v),
                None => std::env::remove_var("XDNA2_LAB_REQUIRE_NONZERO_WTS"),
            }
            match prev_pack {
                Some(v) => std::env::set_var("XDNA2_LAB_PACK", v),
                None => std::env::remove_var("XDNA2_LAB_PACK"),
            }
            match prev_wts {
                Some(v) => std::env::set_var("XDNA2_LAB_WTS_PACKED", v),
                None => std::env::remove_var("XDNA2_LAB_WTS_PACKED"),
            }
            match prev_key {
                Some(v) => std::env::set_var("XDNA2_LAB_MLADF_KEY", v),
                None => std::env::remove_var("XDNA2_LAB_MLADF_KEY"),
            }
            match prev_meta {
                Some(v) => std::env::set_var("XDNA2_LAB_MLADF_META", v),
                None => std::env::remove_var("XDNA2_LAB_MLADF_META"),
            }
        }
    }

    #[test]
    fn attach_ctrl_bo_refuses_empty() {
        let stub = StubDirectSubmission::new(-1);
        let mut ios = create_io_bos(&stub, 16, 16, 16).expect("create");
        let err = attach_ctrl_bo(&stub, &mut ios, &[]).expect_err("empty");
        assert!(err.to_string().contains("empty"));
        assert!(ios.ctrl.is_none());
    }

    #[test]
    fn validate_mladf_ctrl_packed_for_pass_gate() {
        validate_mladf_ctrl_packed_for_pass(&[1, 2, 3], 3).unwrap();
        assert!(
            validate_mladf_ctrl_packed_for_pass(&[0, 0, 0], 3)
                .unwrap_err()
                .to_string()
                .contains("all zeros")
        );
        assert!(
            validate_mladf_ctrl_packed_for_pass(&[1], 3)
                .unwrap_err()
                .to_string()
                .contains("len")
        );
        assert!(
            validate_mladf_ctrl_packed_for_pass(&[], 0)
                .unwrap_err()
                .to_string()
                .contains("empty")
        );
        let mut staged =
            vec![
                0_u8;
                usize::try_from(crate::backends::amd_xdna2::mladf_pack::STAGED_MLADF_CTRL_BYTES)
                    .expect("value fits in usize")
            ];
        staged[100] = 0xCD;
        validate_mladf_ctrl_packed_for_pass(
            &staged,
            crate::backends::amd_xdna2::mladf_pack::STAGED_MLADF_CTRL_BYTES,
        )
        .unwrap();
        let zeros =
            vec![
                0_u8;
                usize::try_from(crate::backends::amd_xdna2::mladf_pack::STAGED_MLADF_CTRL_BYTES)
                    .expect("value fits in usize")
            ];
        assert!(
            validate_mladf_ctrl_packed_for_pass(
                &zeros,
                crate::backends::amd_xdna2::mladf_pack::STAGED_MLADF_CTRL_BYTES,
            )
            .unwrap_err()
            .to_string()
            .contains("all zeros")
        );
    }

    #[test]
    #[allow(unsafe_code)]
    fn require_ctrl_refuses_missing_ctrl_payload() {
        let _guard = lab_env_test_lock();
        let stub = StubDirectSubmission::new(-1);
        let plan = crate::backends::amd_xdna2::mladf_pack::mladf_buffer_plan_from_key(
            crate::backends::amd_xdna2::mladf_pack::STAGED_MLADF_KEY,
            None,
        )
        .expect("plan");
        assert!(plan.ctrl_bytes > 0);
        let prev_req = std::env::var_os("XDNA2_LAB_REQUIRE_CTRL");
        let prev_wts_req = std::env::var_os("XDNA2_LAB_REQUIRE_NONZERO_WTS");
        let prev_allow_wts = std::env::var_os("XDNA2_LAB_ALLOW_ZERO_WTS");
        // SAFETY: held under lab_env_test_lock.
        unsafe {
            std::env::set_var("XDNA2_LAB_REQUIRE_CTRL", "1");
            // Avoid WTS zero gate masking ctrl miss.
            std::env::remove_var("XDNA2_LAB_REQUIRE_NONZERO_WTS");
            std::env::set_var("XDNA2_LAB_ALLOW_ZERO_WTS", "1");
        }
        let err = create_and_fill_mladf_io_bos_ex(&stub, &plan, None, None, None)
            .expect_err("missing ctrl with REQUIRE_CTRL");
        assert!(
            err.to_string().contains("ctrl") || err.to_string().contains("CTRL"),
            "err={err}"
        );
        // With matching non-zero ctrl payload, attach succeeds (no Pass claim).
        let mut ctrl = vec![0_u8; usize::try_from(plan.ctrl_bytes).expect("value fits in usize")];
        ctrl[0] = 1;
        let ios = create_and_fill_mladf_io_bos_ex(&stub, &plan, None, None, Some(&ctrl))
            .expect("ctrl attach");
        assert!(ios.ctrl.is_some());
        assert_eq!(ios.ctrl_bytes, plan.ctrl_bytes);
        assert_eq!(ios.pack_kernel_args()[0], 4);
        assert_eq!(ios.arg_bo_handles().len(), 4);
        // Pin list: IFM WTS OFM CTRL (+ optional inst)
        let pins = build_lab_arg_bo_handles_ex(
            Some((ios.ifm.0, ios.wts.0, ios.ofm.0)),
            ios.ctrl.map(|c| c.0),
            Some(99),
        );
        assert_eq!(pins.len(), 5);
        assert_eq!(pins[3], ios.ctrl.unwrap().0);
        assert_eq!(pins[4], 99);
        destroy_io_bos(&stub, &ios);
        // Wrong size refuses.
        let bad = vec![1_u8; 16];
        let err_sz = create_and_fill_mladf_io_bos_ex(&stub, &plan, None, None, Some(&bad))
            .expect_err("size");
        assert!(
            err_sz.to_string().contains("CTRL") || err_sz.to_string().contains("ctrl"),
            "err={err_sz}"
        );
        // REQUIRE_CTRL + all-zero correct length refuses (fill API Pass gate).
        let zeros = vec![0_u8; usize::try_from(plan.ctrl_bytes).expect("value fits in usize")];
        let err_zeros = create_and_fill_mladf_io_bos_ex(&stub, &plan, None, None, Some(&zeros))
            .expect_err("all-zero ctrl");
        assert!(
            err_zeros.to_string().contains("all zeros"),
            "err={err_zeros}"
        );
        unsafe {
            match prev_req {
                Some(v) => std::env::set_var("XDNA2_LAB_REQUIRE_CTRL", v),
                None => std::env::remove_var("XDNA2_LAB_REQUIRE_CTRL"),
            }
            match prev_wts_req {
                Some(v) => std::env::set_var("XDNA2_LAB_REQUIRE_NONZERO_WTS", v),
                None => std::env::remove_var("XDNA2_LAB_REQUIRE_NONZERO_WTS"),
            }
            match prev_allow_wts {
                Some(v) => std::env::set_var("XDNA2_LAB_ALLOW_ZERO_WTS", v),
                None => std::env::remove_var("XDNA2_LAB_ALLOW_ZERO_WTS"),
            }
        }
    }

    #[test]
    #[allow(unsafe_code)]
    #[allow(clippy::too_many_lines)]
    fn ctrl_auto_patch_writes_io_paddrs_into_attached_bo() {
        let _guard = lab_env_test_lock();
        let stub = StubDirectSubmission::new(-1);
        // Tiny plan: IFM/WTS/OFM sizes match ones pack for M=1,K=8,N=4 (16/16/8).
        let plan = MladfBufferPlan {
            shape: crate::backends::amd_xdna2::mladf_pack::MladfMatmulShape {
                m: 1,
                k: 8,
                n: 4,
                group_size: 128,
            },
            ifm_bytes: 16,
            wts_bytes: 16,
            ofm_bytes: 8,
            ctrl_bytes: 40,
            key: "lab_auto_patch_test".into(),
        };
        let meta = r#"{
          "external_buffers": {
            "buffer0": {
              "name": "coalesed_inputs",
              "size_in_bytes": 16,
              "coalesed_buffers": [{
                "name": "compute_graph.ifm_ddr",
                "control_packet_patch_locations": [
                  {"offset": 0, "operation": "read_add_write", "size": 6}
                ]
              }]
            },
            "buffer1": {
              "name": "coalesed_weights",
              "size_in_bytes": 16,
              "coalesed_buffers": [{
                "name": "compute_graph.wts_ddr",
                "control_packet_patch_locations": [
                  {"offset": 8, "operation": "read_add_write", "size": 6}
                ]
              }]
            },
            "buffer2": {
              "name": "coalesed_outputs",
              "size_in_bytes": 8,
              "coalesed_buffers": [{
                "name": "compute_graph.ofm_ddr",
                "control_packet_patch_locations": [
                  {"offset": 16, "operation": "read_add_write", "size": 6}
                ]
              }]
            }
          }
        }"#;
        let meta_path = std::env::temp_dir().join(format!(
            "dragon_npu_ctrl_auto_patch_meta_{}.json",
            std::process::id()
        ));
        std::fs::write(&meta_path, meta).expect("write meta");
        let prev_auto = std::env::var_os("XDNA2_LAB_CTRL_AUTO_PATCH");
        let prev_meta = std::env::var_os("XDNA2_LAB_MLADF_META");
        let prev_req = std::env::var_os("XDNA2_LAB_REQUIRE_CTRL");
        let prev_wts_req = std::env::var_os("XDNA2_LAB_REQUIRE_NONZERO_WTS");
        let prev_allow_wts = std::env::var_os("XDNA2_LAB_ALLOW_ZERO_WTS");
        // SAFETY: held under lab_env_test_lock.
        unsafe {
            std::env::set_var("XDNA2_LAB_CTRL_AUTO_PATCH", "1");
            std::env::set_var("XDNA2_LAB_MLADF_META", &meta_path);
            std::env::remove_var("XDNA2_LAB_REQUIRE_CTRL");
            std::env::remove_var("XDNA2_LAB_REQUIRE_NONZERO_WTS");
            std::env::set_var("XDNA2_LAB_ALLOW_ZERO_WTS", "1");
        }
        // Patch fields at 0/8/16 start zero (like staged IFM); mark non-zero elsewhere
        // so REQUIRE_CTRL plumbing still sees non-all-zero payload.
        let mut ctrl = vec![0_u8; 40];
        ctrl[32] = 1;
        // Seed OFM tile offset 0x100 at offset 16 (size-6), as staged dump does.
        ctrl[16..22].copy_from_slice(&0x100_u64.to_le_bytes()[..6]);
        let ios = create_and_fill_mladf_io_bos_ex(&stub, &plan, None, None, Some(&ctrl))
            .expect("auto-patch attach");
        let ctrl_h = ios.ctrl.expect("ctrl bo").0;
        let written = stub
            .read_bo_bytes_no_sync(0, ctrl_h, 0, 40)
            .expect("read ctrl bo");
        // size=6: IFM/WTS = base only (existing 0); OFM = base + 0x100
        assert_eq!(&written[0..6], &ios.ifm_addr.to_le_bytes()[..6]);
        assert_eq!(&written[8..14], &ios.wts_addr.to_le_bytes()[..6]);
        let mut ofm_le = [0_u8; 8];
        ofm_le[..6].copy_from_slice(&written[16..22]);
        assert_eq!(u64::from_le_bytes(ofm_le), ios.ofm_addr + 0x100);
        assert_eq!(written[32], 1, "bytes outside patch sites preserved");
        // Without AUTO_PATCH, dump bytes keep original (no paddr overwrite).
        destroy_io_bos(&stub, &ios);
        unsafe {
            std::env::remove_var("XDNA2_LAB_CTRL_AUTO_PATCH");
        }
        let ios2 = create_and_fill_mladf_io_bos_ex(&stub, &plan, None, None, Some(&ctrl))
            .expect("no auto-patch");
        let h2 = ios2.ctrl.expect("ctrl").0;
        let raw = stub.read_bo_bytes_no_sync(0, h2, 0, 40).expect("read raw");
        assert_eq!(raw[0], 0);
        assert_eq!(raw[32], 1);
        assert_eq!(&raw[16..22], &0x100_u64.to_le_bytes()[..6]);
        destroy_io_bos(&stub, &ios2);
        let _ = std::fs::remove_file(&meta_path);
        unsafe {
            match prev_auto {
                Some(v) => std::env::set_var("XDNA2_LAB_CTRL_AUTO_PATCH", v),
                None => std::env::remove_var("XDNA2_LAB_CTRL_AUTO_PATCH"),
            }
            match prev_meta {
                Some(v) => std::env::set_var("XDNA2_LAB_MLADF_META", v),
                None => std::env::remove_var("XDNA2_LAB_MLADF_META"),
            }
            match prev_req {
                Some(v) => std::env::set_var("XDNA2_LAB_REQUIRE_CTRL", v),
                None => std::env::remove_var("XDNA2_LAB_REQUIRE_CTRL"),
            }
            match prev_wts_req {
                Some(v) => std::env::set_var("XDNA2_LAB_REQUIRE_NONZERO_WTS", v),
                None => std::env::remove_var("XDNA2_LAB_REQUIRE_NONZERO_WTS"),
            }
            match prev_allow_wts {
                Some(v) => std::env::set_var("XDNA2_LAB_ALLOW_ZERO_WTS", v),
                None => std::env::remove_var("XDNA2_LAB_ALLOW_ZERO_WTS"),
            }
        }
    }

    #[test]
    #[allow(unsafe_code)]
    fn require_ctrl_pack_mode_refuses_unset_ctrl() {
        let _guard = lab_env_test_lock();
        let stub = StubDirectSubmission::new(-1);
        let prev_req = std::env::var_os("XDNA2_LAB_REQUIRE_CTRL");
        let prev_pack = std::env::var_os("XDNA2_LAB_PACK");
        let prev_ctrl = std::env::var_os("XDNA2_LAB_CTRL_PACKED");
        let prev_key = std::env::var_os("XDNA2_LAB_MLADF_KEY");
        let prev_meta = std::env::var_os("XDNA2_LAB_MLADF_META");
        let prev_wts = std::env::var_os("XDNA2_LAB_WTS_PACKED");
        let prev_wts_req = std::env::var_os("XDNA2_LAB_REQUIRE_NONZERO_WTS");
        let prev_allow_wts = std::env::var_os("XDNA2_LAB_ALLOW_ZERO_WTS");
        // SAFETY: held under lab_env_test_lock.
        unsafe {
            std::env::set_var("XDNA2_LAB_REQUIRE_CTRL", "1");
            std::env::set_var("XDNA2_LAB_PACK", "mladf_a16w4");
            std::env::set_var(
                "XDNA2_LAB_MLADF_KEY",
                crate::backends::amd_xdna2::mladf_pack::STAGED_MLADF_KEY,
            );
            std::env::remove_var("XDNA2_LAB_CTRL_PACKED");
            std::env::remove_var("XDNA2_LAB_MLADF_META");
            std::env::remove_var("XDNA2_LAB_WTS_PACKED");
            std::env::remove_var("XDNA2_LAB_REQUIRE_NONZERO_WTS");
            std::env::set_var("XDNA2_LAB_ALLOW_ZERO_WTS", "1");
        }
        let err = create_and_fill_lab_io_bos_for_pack_mode(
            &stub,
            LabPackMode::MladfA16W4,
            1,
            256,
            1152,
            "bf16",
            false,
        )
        .expect_err("REQUIRE_CTRL unset path");
        assert!(
            err.to_string().contains("REQUIRE_CTRL")
                || err.to_string().contains("CTRL_PACKED")
                || err.to_string().contains("ctrl"),
            "err={err}"
        );
        unsafe {
            match prev_req {
                Some(v) => std::env::set_var("XDNA2_LAB_REQUIRE_CTRL", v),
                None => std::env::remove_var("XDNA2_LAB_REQUIRE_CTRL"),
            }
            match prev_pack {
                Some(v) => std::env::set_var("XDNA2_LAB_PACK", v),
                None => std::env::remove_var("XDNA2_LAB_PACK"),
            }
            match prev_ctrl {
                Some(v) => std::env::set_var("XDNA2_LAB_CTRL_PACKED", v),
                None => std::env::remove_var("XDNA2_LAB_CTRL_PACKED"),
            }
            match prev_key {
                Some(v) => std::env::set_var("XDNA2_LAB_MLADF_KEY", v),
                None => std::env::remove_var("XDNA2_LAB_MLADF_KEY"),
            }
            match prev_meta {
                Some(v) => std::env::set_var("XDNA2_LAB_MLADF_META", v),
                None => std::env::remove_var("XDNA2_LAB_MLADF_META"),
            }
            match prev_wts {
                Some(v) => std::env::set_var("XDNA2_LAB_WTS_PACKED", v),
                None => std::env::remove_var("XDNA2_LAB_WTS_PACKED"),
            }
            match prev_wts_req {
                Some(v) => std::env::set_var("XDNA2_LAB_REQUIRE_NONZERO_WTS", v),
                None => std::env::remove_var("XDNA2_LAB_REQUIRE_NONZERO_WTS"),
            }
            match prev_allow_wts {
                Some(v) => std::env::set_var("XDNA2_LAB_ALLOW_ZERO_WTS", v),
                None => std::env::remove_var("XDNA2_LAB_ALLOW_ZERO_WTS"),
            }
        }
    }

    #[test]
    fn patch_ctrl_from_meta_io_paddrs_applies_offsets() {
        let json = r#"{
          "external_buffers": {
            "buffer0": {
              "name": "coalesed_inputs",
              "size_in_bytes": 2304,
              "coalesed_buffers": [{
                "name": "compute_graph.ifm_ddr",
                "control_packet_patch_locations": [
                  {"offset": 0, "operation": "read_add_write", "size": 6},
                  {"offset": 16, "operation": "read_add_write", "size": 6}
                ]
              }]
            },
            "buffer1": {
              "name": "coalesed_weights",
              "size_in_bytes": 174080,
              "coalesed_buffers": [{
                "name": "compute_graph.wts_ddr",
                "control_packet_patch_locations": [
                  {"offset": 8, "operation": "read_add_write", "size": 6}
                ]
              }]
            },
            "buffer2": {
              "name": "coalesed_outputs",
              "size_in_bytes": 512,
              "coalesed_buffers": [{
                "name": "compute_graph.ofm_ddr",
                "control_packet_patch_locations": [
                  {"offset": 24, "operation": "read_add_write", "size": 6}
                ]
              }]
            }
          }
        }"#;
        let mut buf = vec![0u8; 40];
        // Seed tile offsets like staged dump (OFM 0x100 at offset 24).
        buf[24..30].copy_from_slice(&0x100_u64.to_le_bytes()[..6]);
        patch_ctrl_from_meta_io_paddrs(&mut buf, json, 0xAAA, 0xBBB, 0xCCC).unwrap();
        // size=6 write: only low 6 bytes; high 2 stay 0 from zeroed buffer.
        assert_eq!(&buf[0..6], &0xAAA_u64.to_le_bytes()[..6]);
        assert_eq!(&buf[6..8], &[0, 0]);
        assert_eq!(&buf[8..14], &0xBBB_u64.to_le_bytes()[..6]);
        assert_eq!(&buf[16..22], &0xAAA_u64.to_le_bytes()[..6]);
        // OFM: existing 0x100 + 0xCCC
        let mut ofm = [0_u8; 8];
        ofm[..6].copy_from_slice(&buf[24..30]);
        assert_eq!(u64::from_le_bytes(ofm), 0xCCC + 0x100);
        // OOB offset from meta fails closed
        let bad = r#"{
          "external_buffers": {
            "b0": {
              "name": "coalesed_inputs",
              "coalesed_buffers": [{
                "name": "ifm",
                "control_packet_patch_locations": [{"offset": 100, "size": 6}]
              }]
            }
          }
        }"#;
        let mut small = vec![0u8; 32];
        let err = patch_ctrl_from_meta_io_paddrs(&mut small, bad, 1, 2, 3).expect_err("oob");
        assert!(err.to_string().contains("out of range"));
    }

    #[test]
    fn classify_lab_matmul_status_setup_failed_and_verified() {
        // Args: lab_ofm, setup_ok, submit_ok, inst, readback, decide, allclose_fail, decide_err
        assert_eq!(
            classify_lab_matmul_status(true, false, false, false, false, false, false, false),
            "lab_ofm_setup_failed"
        );
        assert_eq!(
            classify_lab_matmul_status(true, true, true, true, true, true, false, false),
            "ok_npu_verified"
        );
        assert_eq!(
            classify_lab_matmul_status(true, true, true, true, false, false, false, false),
            "ofm_readback_failed"
        );
        assert_eq!(
            classify_lab_matmul_status(true, true, true, true, true, false, false, true),
            "ofm_decide_error"
        );
        assert_eq!(
            classify_lab_matmul_status(true, true, true, true, true, false, true, false),
            "ofm_verify_failed"
        );
        assert_eq!(
            classify_lab_matmul_status(false, true, true, true, false, false, false, false),
            "ok_submit_wait_with_inst_blob"
        );
        assert_eq!(
            classify_lab_matmul_status(false, true, true, false, false, false, false, false),
            "ok_submit_wait_envelope"
        );
        assert_eq!(
            classify_lab_matmul_status(false, true, false, true, false, false, false, false),
            "submit_wait_failed"
        );
        // Envelope / blob submit never ok_npu_verified without decide_ran.
        assert_ne!(
            classify_lab_matmul_status(true, true, true, true, true, false, false, false),
            "ok_npu_verified"
        );
        assert_ne!(
            classify_lab_matmul_status(false, true, true, true, false, false, false, false),
            "ok_npu_verified"
        );
    }

    #[test]
    fn validate_mladf_ofm_golden_packed_for_pass_gate() {
        use crate::backends::amd_xdna2::mladf_pack::f32_to_bf16_bits;
        // bf16 1×2 non-zero
        let mut nz = Vec::new();
        nz.extend_from_slice(&f32_to_bf16_bits(1.0).to_le_bytes());
        nz.extend_from_slice(&f32_to_bf16_bits(2.0).to_le_bytes());
        validate_mladf_ofm_golden_packed_for_pass(&nz, 1, 2, false).expect("nonzero bf16");
        // all-zero bf16 refused
        let z = [0_u8; 4];
        let err = validate_mladf_ofm_golden_packed_for_pass(&z, 1, 2, false).expect_err("zero");
        assert!(
            err.to_string().contains("all-zero") || err.to_string().contains("false-Pass"),
            "msg={err}"
        );
        // escape hatch
        validate_mladf_ofm_golden_packed_for_pass(&z, 1, 2, true).expect("allow zero");
        // wrong size
        let bad = validate_mladf_ofm_golden_packed_for_pass(&[1, 2, 3], 1, 2, false)
            .expect_err("bad len");
        assert!(
            bad.to_string().contains("neither") || bad.to_string().contains("bf16"),
            "msg={bad}"
        );
        // empty
        let empty = validate_mladf_ofm_golden_packed_for_pass(&[], 1, 1, false).expect_err("empty");
        assert!(empty.to_string().contains("empty"));
    }

    #[test]
    fn silicon_pass_asset_readiness_all_or_nothing() {
        let ready = silicon_pass_asset_readiness(true, true, true, true);
        assert!(ready.all_ready());
        assert!(!silicon_pass_asset_readiness(false, true, true, true).all_ready());
        assert!(!silicon_pass_asset_readiness(true, false, true, true).all_ready());
        assert!(!silicon_pass_asset_readiness(true, true, false, true).all_ready());
        assert!(!silicon_pass_asset_readiness(true, true, true, false).all_ready());
    }

    #[test]
    fn zero_ofm_covers_full_alloc_not_just_logical() {
        let stub = StubDirectSubmission::new(-1);
        // logical 4 bytes → alloc 4096
        let ios = create_io_bos(&stub, 4, 4, 4).expect("create");
        assert_eq!(ios.ofm_bytes, 4);
        assert_eq!(ios.ofm_alloc_bytes, 4096);
        // Dirty full alloc
        let dirty = vec![0xffu8; 4096];
        stub.write_bo_bytes(0, ios.ofm.0, 0, &dirty).unwrap();
        zero_ofm(&stub, &ios).expect("zero full");
        let full = stub
            .read_bo_bytes_no_sync(0, ios.ofm.0, 0, 4096)
            .expect("read full");
        assert!(full.iter().all(|&b| b == 0), "page tail must be zeroed");
        destroy_io_bos(&stub, &ios);
    }

    #[test]
    fn fill_ifm_wts_zeros_alloc_tail() {
        let stub = StubDirectSubmission::new(-1);
        let ios = create_io_bos(&stub, 4, 4, 4).expect("create");
        // Dirty IFM alloc
        stub.write_bo_bytes(0, ios.ifm.0, 0, &[0xaau8; 4096])
            .unwrap();
        fill_ifm_wts(&stub, &ios, &[1, 2, 3, 4], &[5, 6, 7, 8]).expect("fill");
        let ifm_full = stub
            .read_bo_bytes_no_sync(0, ios.ifm.0, 0, 4096)
            .expect("read");
        assert_eq!(&ifm_full[..4], &[1, 2, 3, 4]);
        assert!(ifm_full[4..].iter().all(|&b| b == 0), "IFM tail zeroed");
        destroy_io_bos(&stub, &ios);
    }

    #[test]
    fn create_and_fill_mladf_staged_sizes() {
        use crate::backends::amd_xdna2::mladf_pack::staged_mladf_buffer_plan;
        let stub = StubDirectSubmission::new(-1);
        let plan = staged_mladf_buffer_plan();
        let ios = create_and_fill_mladf_io_bos(&stub, &plan, None, None).expect("mladf fill");
        assert_eq!(ios.ifm_bytes, 2304);
        assert_eq!(ios.wts_bytes, 174_080);
        assert_eq!(ios.ofm_bytes, 512);
        // IFM first element bf16 1.0
        let ifm = stub
            .read_bo_bytes_no_sync(0, ios.ifm.0, 0, 2)
            .expect("ifm peek");
        assert_eq!(ifm, 0x3f80_u16.to_le_bytes());
        // WTS zero-filled when no packed file
        let wts_head = stub
            .read_bo_bytes_no_sync(0, ios.wts.0, 0, 16)
            .expect("wts peek");
        assert!(wts_head.iter().all(|&b| b == 0));
        destroy_io_bos(&stub, &ios);
    }

    #[test]
    fn lab_decode_ofm_mladf_bf16() {
        let bits = crate::backends::amd_xdna2::mladf_pack::f32_to_bf16_bits(1.0).to_le_bytes();
        let v = lab_decode_ofm_bytes(LabPackMode::MladfA16W4, &bits).expect("decode");
        assert_eq!(v.len(), 1);
        assert!((v[0] - 1.0).abs() < 0.05);
    }

    #[test]
    fn pack_mode_dispatch_f32_ones_unchanged() {
        let stub = StubDirectSubmission::new(-1);
        let (ios, plan) = create_and_fill_lab_io_bos_for_pack_mode(
            &stub,
            LabPackMode::F32Ones,
            1,
            1,
            2,
            "f32",
            true,
        )
        .expect("f32 ones");
        assert!(plan.is_none());
        assert_eq!(ios.ifm_bytes, 8); // 1*2*4
        destroy_io_bos(&stub, &ios);
    }

    #[test]
    #[allow(unsafe_code)]
    fn pack_mode_dispatch_mladf_a16w4_staged_sizes() {
        // Isolate from REQUIRE_*/ALLOW_* set by concurrent env tests.
        let _guard = lab_env_test_lock();
        let prev_req = std::env::var_os("XDNA2_LAB_REQUIRE_NONZERO_WTS");
        let prev_allow = std::env::var_os("XDNA2_LAB_ALLOW_ZERO_WTS");
        let prev_wts = std::env::var_os("XDNA2_LAB_WTS_PACKED");
        let prev_ctrl_req = std::env::var_os("XDNA2_LAB_REQUIRE_CTRL");
        let prev_ctrl = std::env::var_os("XDNA2_LAB_CTRL_PACKED");
        // SAFETY: held under lab_env_test_lock.
        unsafe {
            std::env::remove_var("XDNA2_LAB_REQUIRE_NONZERO_WTS");
            std::env::remove_var("XDNA2_LAB_ALLOW_ZERO_WTS");
            std::env::remove_var("XDNA2_LAB_WTS_PACKED");
            std::env::remove_var("XDNA2_LAB_REQUIRE_CTRL");
            std::env::remove_var("XDNA2_LAB_CTRL_PACKED");
        }
        let stub = StubDirectSubmission::new(-1);
        let result = create_and_fill_lab_io_bos_for_pack_mode(
            &stub,
            LabPackMode::MladfA16W4,
            1,
            1,
            1, // probe dims ignored
            "f32",
            false,
        );
        unsafe {
            match prev_req {
                Some(v) => std::env::set_var("XDNA2_LAB_REQUIRE_NONZERO_WTS", v),
                None => std::env::remove_var("XDNA2_LAB_REQUIRE_NONZERO_WTS"),
            }
            match prev_allow {
                Some(v) => std::env::set_var("XDNA2_LAB_ALLOW_ZERO_WTS", v),
                None => std::env::remove_var("XDNA2_LAB_ALLOW_ZERO_WTS"),
            }
            match prev_wts {
                Some(v) => std::env::set_var("XDNA2_LAB_WTS_PACKED", v),
                None => std::env::remove_var("XDNA2_LAB_WTS_PACKED"),
            }
            match prev_ctrl_req {
                Some(v) => std::env::set_var("XDNA2_LAB_REQUIRE_CTRL", v),
                None => std::env::remove_var("XDNA2_LAB_REQUIRE_CTRL"),
            }
            match prev_ctrl {
                Some(v) => std::env::set_var("XDNA2_LAB_CTRL_PACKED", v),
                None => std::env::remove_var("XDNA2_LAB_CTRL_PACKED"),
            }
        }
        let (ios, plan) = result.expect("mladf pack");
        let p = plan.expect("mladf plan");
        assert_eq!(ios.ifm_bytes, 2304);
        assert_eq!(ios.wts_bytes, 174_080);
        assert_eq!(ios.ofm_bytes, 512);
        assert_eq!(p.shape.m, 1);
        assert_eq!(p.shape.k, 1152);
        assert_eq!(p.shape.n, 256);
        destroy_io_bos(&stub, &ios);
    }

    #[test]
    #[allow(unsafe_code)] // test-only env isolation for ambient silicon-lab vars
    fn pack_mode_mladf_require_dims_match_refuses() {
        let _guard = lab_env_test_lock();
        // Isolate ambient silicon-lab env so plan comes from staged key constants,
        // not operator META/WTS that change the error path before timed dims check.
        let prev_key = std::env::var("XDNA2_LAB_MLADF_KEY").ok();
        let prev_meta = std::env::var("XDNA2_LAB_MLADF_META").ok();
        let prev_wts = std::env::var("XDNA2_LAB_WTS_PACKED").ok();
        let prev_req = std::env::var("XDNA2_LAB_REQUIRE_NONZERO_WTS").ok();
        let prev_ctrl_req = std::env::var("XDNA2_LAB_REQUIRE_CTRL").ok();
        // SAFETY: test-only env isolation; held under lab_env_test_lock.
        unsafe {
            std::env::remove_var("XDNA2_LAB_MLADF_KEY");
            std::env::remove_var("XDNA2_LAB_MLADF_META");
            std::env::remove_var("XDNA2_LAB_WTS_PACKED");
            std::env::remove_var("XDNA2_LAB_REQUIRE_NONZERO_WTS");
            std::env::remove_var("XDNA2_LAB_REQUIRE_CTRL");
        }
        let stub = StubDirectSubmission::new(-1);
        let err = create_and_fill_lab_io_bos_for_pack_mode(
            &stub,
            LabPackMode::MladfA16W4,
            1,
            1,
            1,
            "bf16",
            true,
        )
        .expect_err("dims");
        let msg = err.to_string();
        unsafe {
            match prev_key {
                Some(v) => std::env::set_var("XDNA2_LAB_MLADF_KEY", v),
                None => std::env::remove_var("XDNA2_LAB_MLADF_KEY"),
            }
            match prev_meta {
                Some(v) => std::env::set_var("XDNA2_LAB_MLADF_META", v),
                None => std::env::remove_var("XDNA2_LAB_MLADF_META"),
            }
            match prev_wts {
                Some(v) => std::env::set_var("XDNA2_LAB_WTS_PACKED", v),
                None => std::env::remove_var("XDNA2_LAB_WTS_PACKED"),
            }
            match prev_req {
                Some(v) => std::env::set_var("XDNA2_LAB_REQUIRE_NONZERO_WTS", v),
                None => std::env::remove_var("XDNA2_LAB_REQUIRE_NONZERO_WTS"),
            }
            match prev_ctrl_req {
                Some(v) => std::env::set_var("XDNA2_LAB_REQUIRE_CTRL", v),
                None => std::env::remove_var("XDNA2_LAB_REQUIRE_CTRL"),
            }
        }
        assert!(
            msg.contains("timed dims") || msg.contains("dim"),
            "unexpected err={msg}"
        );
    }

    #[test]
    fn pack_mode_mladf_refuses_bad_dtype() {
        let stub = StubDirectSubmission::new(-1);
        let err = create_and_fill_lab_io_bos_for_pack_mode(
            &stub,
            LabPackMode::MladfA16W4,
            1,
            256,
            1152,
            "int8",
            false,
        )
        .expect_err("dtype");
        assert!(err.to_string().contains("dtype"));
    }

    #[test]
    fn pack_mode_mladf_ref_sizes_if_present() {
        // Optional ambient fixture from tools/pack_mladf_ref.sh — skip if missing.
        use crate::backends::amd_xdna2::mladf_pack::{
            STAGED_MLADF_IFM_BYTES, STAGED_MLADF_OFM_BYTES, STAGED_MLADF_WTS_BYTES,
        };
        let base = concat!(env!("CARGO_MANIFEST_DIR"), "/testdata/xdna2_cmd/mladf_ref");
        let wts = format!("{base}/wts.bin");
        let ifm = format!("{base}/ifm.bin");
        let ofm = format!("{base}/ofm_expected.bin");
        if std::fs::metadata(&wts).is_err() {
            return;
        }
        assert_eq!(
            std::fs::metadata(&wts).unwrap().len(),
            STAGED_MLADF_WTS_BYTES
        );
        assert_eq!(
            std::fs::metadata(&ifm).unwrap().len(),
            STAGED_MLADF_IFM_BYTES
        );
        assert_eq!(
            std::fs::metadata(&ofm).unwrap().len(),
            STAGED_MLADF_OFM_BYTES
        );
        // IFM first element is bf16 1.0 (0x3f80 LE).
        let head = std::fs::read(&ifm).expect("read ifm");
        assert!(head.len() >= 2);
        assert_eq!(&head[0..2], &0x3f80_u16.to_le_bytes());
    }

    #[test]
    fn mladf_pack_mode_refuses_ones_golden_without_file() {
        let err =
            resolve_pack_mode_ofm_golden(LabPackMode::MladfA16W4, 1, 2, 1152, None, None, false)
                .expect_err("must refuse ones golden for a16w4");
        let msg = err.to_string();
        assert!(
            msg.contains("invalid for a16w4") || msg.contains("XDNA2_LAB_OFM_GOLDEN"),
            "msg={msg}"
        );
    }

    #[test]
    fn mladf_refuses_all_zero_ofm_golden() {
        let zeros = vec![0.0_f32; 2];
        let err = resolve_pack_mode_ofm_golden(
            LabPackMode::MladfA16W4,
            1,
            2,
            1152,
            Some(&zeros),
            None,
            false,
        )
        .expect_err("zero golden");
        assert!(
            err.to_string().contains("all-zero") || err.to_string().contains("false-Pass"),
            "msg={err}"
        );
        // Escape hatch
        let ok = resolve_pack_mode_ofm_golden(
            LabPackMode::MladfA16W4,
            1,
            2,
            1152,
            Some(&zeros),
            None,
            true,
        )
        .expect("allow zero");
        assert!(ofm_golden_is_all_zero(&ok));
    }

    #[test]
    fn format_ofm_mismatch_stats_bf16_preview() {
        use crate::backends::amd_xdna2::mladf_pack::f32_to_bf16_bits;
        let mut ofm = Vec::new();
        ofm.extend_from_slice(&f32_to_bf16_bits(1.0).to_le_bytes());
        ofm.extend_from_slice(&f32_to_bf16_bits(2.0).to_le_bytes());
        ofm.extend_from_slice(&0u16.to_le_bytes());
        ofm.extend_from_slice(&0u16.to_le_bytes());
        let s = format_ofm_mismatch_stats_preview(
            &ofm,
            LabPackMode::MladfA16W4,
            Some(&[1.0_f32, 0.0, 0.0, 0.0]),
            4,
        );
        assert!(s.contains("raw_len=8"), "{s}");
        assert!(s.contains("raw_nonzero_bytes="), "{s}");
        assert!(s.contains("first_bf16_le=["), "{s}");
        assert!(s.contains("last_bf16_le=["), "{s}");
        assert!(s.contains("decoded_len=4"), "{s}");
        assert!(s.contains("nonzero_f32=2"), "{s}");
        assert!(s.contains("first_f32=["), "{s}");
        assert!(s.contains("max_abs_diff="), "{s}");
        assert!(s.contains("golden_nonzero_f32=1"), "{s}");
        // Odd length: must fail decode (not vacuous raw_len only).
        let odd = format_ofm_mismatch_stats(&[0xab], LabPackMode::MladfA16W4, None);
        assert!(odd.contains("raw_len=1"), "{odd}");
        assert!(
            odd.contains("decode_err="),
            "odd bf16 must surface decode_err: {odd}"
        );
        // No golden → no max_abs; with golden of wrong len → mismatch marker
        let no_g = format_ofm_mismatch_stats_preview(&ofm, LabPackMode::MladfA16W4, None, 2);
        assert!(!no_g.contains("max_abs_diff="), "{no_g}");
        let bad_g =
            format_ofm_mismatch_stats_preview(&ofm, LabPackMode::MladfA16W4, Some(&[1.0]), 2);
        assert!(bad_g.contains("golden_len_mismatch"), "{bad_g}");
    }

    #[test]
    fn mladf_pack_mode_decide_with_bf16_golden_slice() {
        use crate::backends::amd_xdna2::mladf_pack::f32_to_bf16_bits;
        let mut ofm = Vec::new();
        ofm.extend_from_slice(&f32_to_bf16_bits(1.0).to_le_bytes());
        ofm.extend_from_slice(&f32_to_bf16_bits(1.0).to_le_bytes());
        let plan = LabOfmVerifyPlan {
            m: 1,
            n: 2,
            k: 3,
            used_inst_blob: true,
            submit_wait_ok: true,
            real_ioctl: true,
            ofm_bytes: ofm,
            criteria: NumericalVerifyCriteria::default(),
        };
        let golden = vec![1.0_f32, 1.0];
        let (ok, reason) =
            lab_decide_from_ofm_bytes_pack_mode(LabPackMode::MladfA16W4, &plan, Some(&golden))
                .expect("decide");
        assert!(ok, "reason={reason:?}");
    }

    #[test]
    #[allow(unsafe_code)]
    fn mladf_pack_mode_decide_refuses_zero_golden_slice() {
        // Isolate ALLOW — parallel tests may leave it set.
        let prev = std::env::var_os("XDNA2_LAB_ALLOW_ZERO_GOLDEN");
        // SAFETY: unit-test env isolation.
        unsafe {
            std::env::remove_var("XDNA2_LAB_ALLOW_ZERO_GOLDEN");
        }
        let ofm = vec![0_u8; 4]; // 2 bf16 zeros
        let plan = LabOfmVerifyPlan {
            m: 1,
            n: 2,
            k: 3,
            used_inst_blob: true,
            submit_wait_ok: true,
            real_ioctl: true,
            ofm_bytes: ofm,
            criteria: NumericalVerifyCriteria::default(),
        };
        let golden = vec![0.0_f32, 0.0];
        // Default: resolve refuses zero golden (Err) unless ALLOW env set.
        let result =
            lab_decide_from_ofm_bytes_pack_mode(LabPackMode::MladfA16W4, &plan, Some(&golden));
        unsafe {
            match prev {
                Some(v) => std::env::set_var("XDNA2_LAB_ALLOW_ZERO_GOLDEN", v),
                None => std::env::remove_var("XDNA2_LAB_ALLOW_ZERO_GOLDEN"),
            }
        }
        // Either Err at resolve, or Ok(false) with AllZeroGolden if ALLOW ambient races.
        match result {
            Err(e) => {
                assert!(
                    e.to_string().contains("all-zero") || e.to_string().contains("false-Pass"),
                    "err={e}"
                );
            }
            Ok((false, Some(RanOnNpuBlockReason::AllZeroGolden))) => {}
            other => panic!("expected zero golden refuse, got {other:?}"),
        }
    }

    #[test]
    #[allow(unsafe_code)]
    fn mladf_allow_zero_golden_still_refuses_product_decide() {
        // ALLOW bypasses resolve only; product decide must still refuse AllZeroGolden.
        let prev = std::env::var_os("XDNA2_LAB_ALLOW_ZERO_GOLDEN");
        // SAFETY: test-only env isolation.
        unsafe {
            std::env::set_var("XDNA2_LAB_ALLOW_ZERO_GOLDEN", "1");
        }
        let ofm = vec![0_u8; 4];
        let plan = LabOfmVerifyPlan {
            m: 1,
            n: 2,
            k: 3,
            used_inst_blob: true,
            submit_wait_ok: true,
            real_ioctl: true,
            ofm_bytes: ofm,
            criteria: NumericalVerifyCriteria::default(),
        };
        let golden = vec![0.0_f32, 0.0];
        let (ok, reason) =
            lab_decide_from_ofm_bytes_pack_mode(LabPackMode::MladfA16W4, &plan, Some(&golden))
                .expect("resolve allowed under ALLOW");
        assert!(!ok, "ALLOW must never product Pass on zero golden");
        assert_eq!(reason, Some(RanOnNpuBlockReason::AllZeroGolden));
        assert!(ran_on_npu_reason_is_ofm_verify_fail(reason));
        unsafe {
            match prev {
                Some(v) => std::env::set_var("XDNA2_LAB_ALLOW_ZERO_GOLDEN", v),
                None => std::env::remove_var("XDNA2_LAB_ALLOW_ZERO_GOLDEN"),
            }
        }
    }

    #[test]
    fn parse_ofm_golden_bytes_size_fail_closed() {
        let err = parse_ofm_golden_bytes(&[0_u8; 3], 1, 2).expect_err("bad len");
        let msg = err.to_string();
        assert!(
            msg.contains("neither f32") || msg.contains("bf16"),
            "msg={msg}"
        );
        // bf16 1×2 = 4 bytes → two f32 after unpack
        let bf = parse_ofm_golden_bytes(&[0, 0, 0, 0], 1, 2).expect("bf16 zeros");
        assert_eq!(bf.len(), 2);
        // f32 1×2 = 8 bytes
        let f = parse_ofm_golden_bytes(&[0_u8; 8], 1, 2).expect("f32 zeros");
        assert_eq!(f.len(), 2);
    }

    #[test]
    fn check_mladf_pack_dtype_accepts_aliases() {
        assert!(check_mladf_pack_dtype("f32").is_ok());
        assert!(check_mladf_pack_dtype("bf16").is_ok());
        assert!(check_mladf_pack_dtype("a16").is_ok());
        assert!(check_mladf_pack_dtype("int8").is_err());
    }

    #[test]
    fn resolve_golden_len_mismatch_fail_closed() {
        let err = resolve_pack_mode_ofm_golden(
            LabPackMode::MladfA16W4,
            1,
            2,
            3,
            Some(&[1.0]), // need 2 elems
            None,
            true,
        )
        .expect_err("len");
        assert!(err.to_string().contains("golden len"));
    }

    #[test]
    fn resolve_golden_explicit_beats_file() {
        let file = vec![9.0_f32, 9.0];
        let explicit = vec![1.0_f32, 2.0];
        let g = resolve_pack_mode_ofm_golden(
            LabPackMode::MladfA16W4,
            1,
            2,
            3,
            Some(&explicit),
            Some(&file),
            true,
        )
        .expect("explicit");
        assert_eq!(g, explicit);
    }

    #[test]
    fn parse_ofm_golden_empty_fails() {
        let err = parse_ofm_golden_bytes(&[], 1, 1).expect_err("empty");
        assert!(err.to_string().contains("empty"));
    }

    #[test]
    fn lab_decode_ofm_odd_bf16_fails() {
        let err = lab_decode_ofm_bytes(LabPackMode::MladfA16W4, &[0_u8; 3]).expect_err("odd");
        assert!(err.to_string().contains("multiple of 2") || err.to_string().contains("bf16"));
    }

    #[test]
    fn lab_decide_pack_mode_mladf_without_golden_errs() {
        let err = resolve_pack_mode_ofm_golden(LabPackMode::MladfA16W4, 1, 2, 3, None, None, false)
            .expect_err("no golden");
        assert!(
            err.to_string().contains("invalid for a16w4")
                || err.to_string().contains("XDNA2_LAB_OFM_GOLDEN")
        );
    }

    #[test]
    #[allow(unsafe_code)]
    fn load_ofm_golden_from_env_temp_file() {
        let dir = std::env::temp_dir().join(format!("dragon_npu_ofm_env_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("g.bin");
        // bf16 two ones for m=1 n=2
        let mut bytes = Vec::new();
        bytes.extend_from_slice(
            &crate::backends::amd_xdna2::mladf_pack::f32_to_bf16_bits(1.0).to_le_bytes(),
        );
        bytes.extend_from_slice(
            &crate::backends::amd_xdna2::mladf_pack::f32_to_bf16_bits(2.0).to_le_bytes(),
        );
        std::fs::write(&path, &bytes).unwrap();
        let prev = std::env::var_os("XDNA2_LAB_OFM_GOLDEN");
        // SAFETY: test-only golden path env.
        unsafe {
            std::env::set_var("XDNA2_LAB_OFM_GOLDEN", path.to_str().unwrap());
        }
        let g = load_ofm_golden_from_env(1, 2).expect("load").expect("some");
        assert_eq!(g.len(), 2);
        assert!((g[0] - 1.0).abs() < 0.05);
        // bad size
        std::fs::write(&path, [0_u8; 3]).unwrap();
        let err = load_ofm_golden_from_env(1, 2).expect_err("bad len");
        assert!(err.to_string().contains("neither") || err.to_string().contains("bf16"));
        unsafe {
            match prev {
                Some(v) => std::env::set_var("XDNA2_LAB_OFM_GOLDEN", v),
                None => std::env::remove_var("XDNA2_LAB_OFM_GOLDEN"),
            }
        }
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
    }
}
