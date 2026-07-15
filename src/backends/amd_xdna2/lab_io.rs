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

use super::direct::{
    BufferHandle, BufferType, DirectError, DirectSubmission, HOST_REF_MAX_ELEMENTS,
    SYNC_BO_FROM_DEVICE,
};
use super::numerical::{
    NumericalVerifyCriteria, NumericalVerifyInput, RanOnNpuBlockReason, decide_ran_on_npu_reason,
    host_matmul_f32_ones_vec,
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
    for chunk in bytes.chunks_exact(4) {
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
/// u64  3
/// u64  0
/// u32  0
/// u64  IFM.paddr
/// u64  WTS.paddr
/// u64  OFM.paddr
/// u64  0
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
    let mut out = Vec::with_capacity(15);
    // marker u64 = 3
    out.extend_from_slice(&u64_to_prop_dwords(3));
    // u64 0
    out.extend_from_slice(&u64_to_prop_dwords(0));
    // u32 0
    out.push(0);
    out.extend_from_slice(&u64_to_prop_dwords(ifm_paddr));
    out.extend_from_slice(&u64_to_prop_dwords(wts_paddr));
    out.extend_from_slice(&u64_to_prop_dwords(ofm_paddr));
    out.extend_from_slice(&u64_to_prop_dwords(0));
    out.extend_from_slice(&u64_to_prop_dwords(0));
    out
}

/// No-I/O kernel args (shim `elf_init_no_arg_cmd` style): marker + zeros.
#[must_use]
pub fn pack_start_npu_kernel_args_no_io() -> Vec<u32> {
    pack_start_npu_kernel_args(0, 0, 0)
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
/// # Errors
///
/// - Golden length ≠ `m * n` (strict plan match)
/// - Non-multiple-of-4 OFM length (decode)
pub fn lab_decide_from_ofm_bytes_with_golden(
    plan: &LabOfmVerifyPlan,
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
    let candidate = f32_slice_from_le_bytes(&plan.ofm_bytes)?;
    let input = NumericalVerifyInput {
        host_golden: golden,
        device_or_candidate: &candidate,
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
    /// Device-visible IFM address (`xdna_addr`).
    pub ifm_addr: u64,
    /// Device-visible WTS address.
    pub wts_addr: u64,
    /// Device-visible OFM address.
    pub ofm_addr: u64,
    /// Logical IFM payload bytes (fill exact).
    pub ifm_bytes: u64,
    /// Logical WTS payload bytes (fill exact).
    pub wts_bytes: u64,
    /// Logical OFM payload bytes (readback exact).
    pub ofm_bytes: u64,
    /// Allocated IFM BO size (≥ `ifm_bytes`, typically page-rounded).
    pub ifm_alloc_bytes: u64,
    /// Allocated WTS BO size.
    pub wts_alloc_bytes: u64,
    /// Allocated OFM BO size (full zero before EXEC).
    pub ofm_alloc_bytes: u64,
}

impl IoBoSet {
    /// Handles for EXEC_CMD `args[]` pin list (IFM, WTS, OFM).
    #[must_use]
    pub fn arg_bo_handles(&self) -> Vec<u32> {
        vec![self.ifm.0, self.wts.0, self.ofm.0]
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
    let mut out = Vec::with_capacity(4);
    if let Some((ifm, wts, ofm)) = io_handles {
        out.push(ifm);
        out.push(wts);
        out.push(ofm);
    }
    if let Some(h) = inst_handle {
        out.push(h);
    }
    out
}

/// Pure status classifier for timed_matmul lab OFM outcomes (no device I/O).
///
/// Callers that have a more specific submit failure string may override the
/// generic `"submit_wait_failed"` result.
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

/// CREATE_BO(Shmem)×3 + `get_bo_info`; refuse zero `xdna_addr`.
///
/// Does **not** submit. Does **not** claim compute. Caller must
/// [`destroy_io_bos`].
///
/// # Errors
///
/// Create / info failures or zero device address.
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

    let ifm_info = match submission.get_bo_info(0, ifm.0) {
        Ok(i) if i.xdna_addr != 0 => i,
        Ok(_) => {
            cleanup(submission, ifm.0, wts.0, ofm.0);
            return Err(DirectError::InvalidArgument(
                "create_io_bos: IFM xdna_addr=0".into(),
            ));
        }
        Err(e) => {
            cleanup(submission, ifm.0, wts.0, ofm.0);
            return Err(e);
        }
    };
    let wts_info = match submission.get_bo_info(0, wts.0) {
        Ok(i) if i.xdna_addr != 0 => i,
        Ok(_) => {
            cleanup(submission, ifm.0, wts.0, ofm.0);
            return Err(DirectError::InvalidArgument(
                "create_io_bos: WTS xdna_addr=0".into(),
            ));
        }
        Err(e) => {
            cleanup(submission, ifm.0, wts.0, ofm.0);
            return Err(e);
        }
    };
    let ofm_info = match submission.get_bo_info(0, ofm.0) {
        Ok(i) if i.xdna_addr != 0 => i,
        Ok(_) => {
            cleanup(submission, ifm.0, wts.0, ofm.0);
            return Err(DirectError::InvalidArgument(
                "create_io_bos: OFM xdna_addr=0".into(),
            ));
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
        ifm_addr: ifm_info.xdna_addr,
        wts_addr: wts_info.xdna_addr,
        ofm_addr: ofm_info.xdna_addr,
        ifm_bytes,
        wts_bytes,
        ofm_bytes,
        ifm_alloc_bytes: ifm_alloc,
        wts_alloc_bytes: wts_alloc,
        ofm_alloc_bytes: ofm_alloc,
    })
}

/// Best-effort destroy of IFM/WTS/OFM handles.
pub fn destroy_io_bos(submission: &dyn DirectSubmission, ios: &IoBoSet) {
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
        assert!(v.is_empty());
        assert!(f32_slice_to_le_bytes(&[]).is_empty());
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
    fn pack_kernel_args_partial_elf_layout() {
        let args = pack_start_npu_kernel_args(0xa, 0xb, 0xc);
        // 2+2+1+2+2+2+2+2 = 15 dwords
        assert_eq!(args.len(), 15);
        assert_eq!(args[0], 3);
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
        let (bad, r2) = lab_decide_from_ofm_bytes_with_golden(&plan, &[0.0_f32]).unwrap();
        assert!(!bad);
        assert_eq!(r2, Some(RanOnNpuBlockReason::AllcloseFailed));
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
}
