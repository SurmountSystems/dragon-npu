//! Pure numerical verify helpers for Phase 3.5 product `ran_on_npu` policy.
//!
//! **No hardware.** No `/dev/accel`. No EXEC_CMD. These functions only compare
//! host golden vs candidate buffers and encode the honesty rules for when
//! [`crate::backends::amd_xdna2::TimedOpResult::ran_on_npu`] may become true.
//!
//! **Feature-gated:** compiled only with `amd-xdna2` (parent module). Default
//! `cargo test --lib` does not run this suite; CI runs both feature sets.
//!
//! **Timed-path wire:** [`super::direct::timed_matmul_direct`] calls
//! [`super::lab_io::lab_decide_from_ofm_bytes`] **only** when `XDNA2_LAB_OFM=1`
//! and OFM readback succeeds. Default (env unset) still hard-false `ran_on_npu`.
//! Call [`decide_ran_on_npu`] only after a real **non-empty** device output buffer
//! is available. `submit_wait_ok` must reflect a real L1-gated EXEC+WAIT success
//! (never host_ref / stub envelope alone).
//!
//! See `docs/XDNA2_TXN_RESEARCH.md` and `docs/XDNA2_DIRECT_CAPABILITIES.md`.

use super::direct::{DirectError, HOST_REF_MAX_ELEMENTS};

/// Default absolute tolerance for f32 allclose (GEMM microbench scale).
pub const DEFAULT_VERIFY_ATOL: f32 = 1e-4;
/// Default relative tolerance for f32 allclose.
pub const DEFAULT_VERIFY_RTOL: f32 = 1e-3;

/// Tolerances for elementwise f32 compare.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NumericalVerifyCriteria {
    /// Relative tolerance: `|a-b| <= atol + rtol * |expected|`.
    pub rtol: f32,
    /// Absolute tolerance floor.
    pub atol: f32,
}

impl Default for NumericalVerifyCriteria {
    fn default() -> Self {
        Self {
            rtol: DEFAULT_VERIFY_RTOL,
            atol: DEFAULT_VERIFY_ATOL,
        }
    }
}

/// Inputs for product [`decide_ran_on_npu`] policy (pure; no device I/O).
#[derive(Debug, Clone, Copy)]
pub struct NumericalVerifyInput<'a> {
    /// Host golden output (e.g. all-ones matmul or graph golden). **Must be non-empty for Pass.**
    pub host_golden: &'a [f32],
    /// Device readback or candidate buffer to compare. **Must be non-empty for Pass.**
    pub device_or_candidate: &'a [f32],
    /// True only when a non-empty instruction/txn blob was used for START_NPU.
    pub had_instruction_blob: bool,
    /// True only when gated EXEC_CMD + WAIT completed without error.
    pub submit_wait_ok: bool,
    /// True only on [`super::direct::RealDirectSubmission`] ioctl path (never stub).
    pub real_ioctl_path: bool,
    /// Elementwise tolerances.
    pub criteria: NumericalVerifyCriteria,
}

/// Why product [`decide_ran_on_npu`] refused Pass (diagnostic; pure).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RanOnNpuBlockReason {
    /// Empty golden and/or candidate (empty OFM must never Pass).
    EmptyBuffers,
    /// EXEC_CMD + WAIT did not succeed.
    SubmitWaitNotOk,
    /// No instruction/txn blob on the job.
    MissingInstructionBlob,
    /// Not real ioctl path (stub / simulation).
    NotRealIoctlPath,
    /// Length mismatch, non-finite, or tolerance failure.
    AllcloseFailed,
}

/// Maximum absolute elementwise difference `|a[i]-b[i]|`.
///
/// Non-finite values on either side fail closed (same honesty as allclose).
///
/// # Errors
///
/// [`DirectError::InvalidArgument`] when lengths differ or any element is non-finite.
pub fn max_abs_diff_f32(a: &[f32], b: &[f32]) -> Result<f32, DirectError> {
    if a.len() != b.len() {
        return Err(DirectError::InvalidArgument(format!(
            "max_abs_diff_f32 length mismatch: got_len={} expected_len={}",
            a.len(),
            b.len()
        )));
    }
    if a.is_empty() {
        return Ok(0.0);
    }
    let mut max_d = 0.0_f32;
    for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        if !x.is_finite() || !y.is_finite() {
            return Err(DirectError::InvalidArgument(format!(
                "max_abs_diff_f32 non-finite at index {i}: a={x} b={y}"
            )));
        }
        let d = (x - y).abs();
        if d > max_d {
            max_d = d;
        }
    }
    Ok(max_d)
}

/// Elementwise allclose: for each `i`, `|a[i]-b[i]| <= atol + rtol * |b[i]|`
/// with `b` treated as the expected / golden side.
///
/// - NaN or non-finite values on either side fail closed.
/// - Empty equal-length slices return `Ok(())` (numpy-like); **product policy**
///   [`decide_ran_on_npu`] separately refuses empty buffers.
///
/// # Errors
///
/// Length mismatch, bad tolerances, non-finite values, or tolerance violation.
pub fn verify_f32_allclose(a: &[f32], b: &[f32], rtol: f32, atol: f32) -> Result<(), DirectError> {
    if a.len() != b.len() {
        return Err(DirectError::InvalidArgument(format!(
            "verify_f32_allclose length mismatch: got_len={} golden_len={}",
            a.len(),
            b.len()
        )));
    }
    if !(rtol.is_finite() && atol.is_finite()) || rtol < 0.0 || atol < 0.0 {
        return Err(DirectError::InvalidArgument(format!(
            "verify_f32_allclose requires finite non-negative rtol/atol (rtol={rtol}, atol={atol})"
        )));
    }
    for (i, (got, exp)) in a.iter().zip(b.iter()).enumerate() {
        if !got.is_finite() || !exp.is_finite() {
            return Err(DirectError::InvalidArgument(format!(
                "verify_f32_allclose non-finite at index {i}: got={got} expected={exp}"
            )));
        }
        let tol = rtol.mul_add(exp.abs(), atol);
        let diff = (got - exp).abs();
        if diff > tol {
            return Err(DirectError::InvalidArgument(format!(
                "verify_f32_allclose mismatch at index {i}: |{got}-{exp}|={diff} > tol={tol} \
                 (atol={atol} rtol={rtol})"
            )));
        }
    }
    Ok(())
}

/// Pure host f32 all-ones matmul `C = ones(m,k) @ ones(k,n)` → every element `= k as f32`.
///
/// Same contract as [`super::direct::host_matmul_f32_ones`] but returns the full
/// output vector for elementwise verify (never NPU). Dimensions are `u32` to match
/// timed/API shapes; uses shared [`HOST_REF_MAX_ELEMENTS`].
///
/// # Errors
///
/// Zero dimensions or product overflow / exceeds host-ref element cap.
#[allow(clippy::cast_precision_loss)]
pub fn host_matmul_f32_ones_vec(m: u32, n: u32, k: u32) -> Result<Vec<f32>, DirectError> {
    if m == 0 || n == 0 || k == 0 {
        return Err(DirectError::InvalidArgument(
            "host_matmul_f32_ones_vec requires m,n,k > 0".to_string(),
        ));
    }
    let m_u = u64::from(m);
    let n_u = u64::from(n);
    let k_u = u64::from(k);
    let mul_elems = m_u
        .checked_mul(n_u)
        .and_then(|x| x.checked_mul(k_u))
        .ok_or_else(|| {
            DirectError::InvalidArgument(
                "host_matmul_f32_ones_vec element product overflow".to_string(),
            )
        })?;
    if mul_elems > HOST_REF_MAX_ELEMENTS {
        return Err(DirectError::InvalidArgument(format!(
            "host_matmul_f32_ones_vec refuses m*n*k={mul_elems} > max {HOST_REF_MAX_ELEMENTS}"
        )));
    }
    let out_elems = m_u.saturating_mul(n_u);
    let out_len = usize::try_from(out_elems).map_err(|_| {
        DirectError::InvalidArgument(
            "host_matmul_f32_ones_vec output length exceeds usize".to_string(),
        )
    })?;
    let k_f = k as f32;
    // All-ones A,B → every C element equals k (exact for modest k in f32).
    Ok(vec![k_f; out_len])
}

/// Product policy with structured block reason (pure; no device I/O).
///
/// Returns `Ok(())` only when **all** honesty criteria pass, including
/// **non-empty** golden and candidate buffers.
///
/// L1/L3 gates are **not** encoded here — caller must set `submit_wait_ok` only
/// after a real gated EXEC+WAIT success.
pub fn decide_ran_on_npu_reason(
    input: &NumericalVerifyInput<'_>,
) -> Result<(), RanOnNpuBlockReason> {
    if input.host_golden.is_empty() || input.device_or_candidate.is_empty() {
        return Err(RanOnNpuBlockReason::EmptyBuffers);
    }
    if !input.submit_wait_ok {
        return Err(RanOnNpuBlockReason::SubmitWaitNotOk);
    }
    if !input.had_instruction_blob {
        return Err(RanOnNpuBlockReason::MissingInstructionBlob);
    }
    if !input.real_ioctl_path {
        return Err(RanOnNpuBlockReason::NotRealIoctlPath);
    }
    verify_f32_allclose(
        input.device_or_candidate,
        input.host_golden,
        input.criteria.rtol,
        input.criteria.atol,
    )
    .map_err(|_| RanOnNpuBlockReason::AllcloseFailed)
}

/// Product policy for `ran_on_npu`: true only when **all** honesty criteria pass.
///
/// Returns **false** if any of:
/// - empty `host_golden` or `device_or_candidate` (empty OFM must never Pass)
/// - `!submit_wait_ok`
/// - `!had_instruction_blob`
/// - `!real_ioctl_path`
/// - allclose fails (including length mismatch / non-finite / tol)
///
/// Does **not** open devices or submit work. See [`decide_ran_on_npu_reason`] for
/// which criterion blocked.
#[must_use]
pub fn decide_ran_on_npu(input: &NumericalVerifyInput<'_>) -> bool {
    decide_ran_on_npu_reason(input).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_abs_diff_length_mismatch_errs() {
        let err = max_abs_diff_f32(&[1.0], &[1.0, 2.0]).expect_err("len");
        assert!(err.to_string().contains("length mismatch"), "err={err}");
        assert!(err.to_string().contains("got_len=1"), "err={err}");
    }

    #[test]
    fn max_abs_diff_empty_is_zero() {
        assert!((max_abs_diff_f32(&[], &[]).expect("empty") - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn max_abs_diff_finds_peak() {
        let a = [0.0, 1.0, 3.5];
        let b = [0.0, 1.0, 1.0];
        let d = max_abs_diff_f32(&a, &b).expect("diff");
        assert!((d - 2.5).abs() < 1e-6);
    }

    #[test]
    fn max_abs_diff_nan_fail_closed() {
        let err = max_abs_diff_f32(&[f32::NAN], &[0.0]).expect_err("nan got");
        assert!(err.to_string().contains("non-finite"), "err={err}");
        let err2 = max_abs_diff_f32(&[0.0], &[f32::NAN]).expect_err("nan exp");
        assert!(err2.to_string().contains("non-finite"), "err={err2}");
    }

    #[test]
    fn max_abs_diff_inf_fail_closed() {
        let err = max_abs_diff_f32(&[f32::INFINITY], &[f32::INFINITY]).expect_err("inf-inf");
        assert!(err.to_string().contains("non-finite"), "err={err}");
        let err2 = max_abs_diff_f32(&[f32::NEG_INFINITY], &[0.0]).expect_err("neginf");
        assert!(err2.to_string().contains("non-finite"), "err={err2}");
    }

    #[test]
    fn verify_allclose_exact_ok() {
        let v = [1.0_f32, 2.0, 3.0];
        verify_f32_allclose(&v, &v, 0.0, 0.0).expect("exact");
    }

    #[test]
    fn verify_allclose_empty_ok_numpy_like() {
        // Pure allclose: empty equal lengths succeed; product decide refuses empty.
        verify_f32_allclose(&[], &[], 0.0, 0.0).expect("empty allclose");
    }

    #[test]
    fn verify_allclose_length_mismatch_errs() {
        let err = verify_f32_allclose(&[1.0], &[1.0, 2.0], 0.0, 0.0).expect_err("len");
        assert!(err.to_string().contains("length mismatch"), "err={err}");
        let err2 = verify_f32_allclose(&[1.0, 2.0], &[1.0], 0.0, 0.0).expect_err("len rev");
        assert!(err2.to_string().contains("got_len=2"), "err={err2}");
    }

    #[test]
    fn verify_allclose_rtol_only_boundary() {
        // exp=1000, rtol=1e-3, atol=0 → tol=1.0; pass at 1000.5, fail at 1001.5
        let golden = [1000.0_f32];
        verify_f32_allclose(&[1000.5], &golden, 1e-3, 0.0).expect("within rtol");
        // equality at tol must pass (diff > tol, not >=)
        verify_f32_allclose(&[1001.0], &golden, 1e-3, 0.0).expect("boundary eq");
        let err = verify_f32_allclose(&[1001.5], &golden, 1e-3, 0.0).expect_err("outside rtol");
        assert!(err.to_string().contains("mismatch"), "err={err}");
    }

    #[test]
    fn verify_allclose_atol_only_boundary() {
        // exp=0, rtol=0, atol=1e-4; pass 1e-5, fail 1e-3; boundary 1e-4 passes
        let golden = [0.0_f32];
        verify_f32_allclose(&[1e-5], &golden, 0.0, 1e-4).expect("within atol");
        verify_f32_allclose(&[1e-4], &golden, 0.0, 1e-4).expect("boundary eq");
        let err = verify_f32_allclose(&[1e-3], &golden, 0.0, 1e-4).expect_err("outside atol");
        assert!(err.to_string().contains("mismatch"), "err={err}");
    }

    #[test]
    fn verify_allclose_within_tol_ok() {
        let golden = [1.0_f32, 2.0];
        let got = [1.0 + 1e-5, 2.0 - 1e-5];
        verify_f32_allclose(&got, &golden, 1e-3, 1e-4).expect("within");
    }

    #[test]
    fn verify_allclose_outside_tol_errs() {
        let golden = [1.0_f32];
        let got = [2.0_f32];
        let err = verify_f32_allclose(&got, &golden, 1e-6, 1e-6).expect_err("far");
        assert!(err.to_string().contains("mismatch"), "err={err}");
        assert!(err.to_string().contains("index 0"), "err={err}");
    }

    #[test]
    fn verify_allclose_rejects_bad_rtol_atol() {
        let v = [1.0_f32];
        assert!(verify_f32_allclose(&v, &v, -1.0, 0.0).is_err());
        assert!(verify_f32_allclose(&v, &v, 0.0, -1.0).is_err());
        assert!(verify_f32_allclose(&v, &v, f32::NAN, 0.0).is_err());
        assert!(verify_f32_allclose(&v, &v, 0.0, f32::INFINITY).is_err());
    }

    #[test]
    fn verify_allclose_nan_fail_closed() {
        let golden = [1.0_f32];
        let got = [f32::NAN];
        let err = verify_f32_allclose(&got, &golden, 0.0, 1.0).expect_err("nan got");
        assert!(err.to_string().contains("non-finite"), "err={err}");
    }

    #[test]
    fn verify_allclose_nan_golden_and_inf_fail_closed() {
        let err = verify_f32_allclose(&[1.0], &[f32::NAN], 0.0, 1.0).expect_err("nan golden");
        assert!(err.to_string().contains("non-finite"), "err={err}");
        let err2 = verify_f32_allclose(&[f32::INFINITY], &[1.0], 0.0, 1.0).expect_err("inf got");
        assert!(err2.to_string().contains("non-finite"), "err={err2}");
        let err3 =
            verify_f32_allclose(&[1.0], &[f32::NEG_INFINITY], 0.0, 1.0).expect_err("neginf golden");
        assert!(err3.to_string().contains("non-finite"), "err={err3}");
    }

    #[test]
    fn host_matmul_ones_vec_all_k() {
        let c = host_matmul_f32_ones_vec(2, 3, 4).expect("vec");
        assert_eq!(c.len(), 6);
        assert!(c.iter().all(|&x| (x - 4.0).abs() < 1e-5));
    }

    #[test]
    fn host_matmul_ones_vec_refuses_zero() {
        assert!(host_matmul_f32_ones_vec(0, 1, 1).is_err());
        assert!(host_matmul_f32_ones_vec(1, 0, 1).is_err());
        assert!(host_matmul_f32_ones_vec(1, 1, 0).is_err());
    }

    #[test]
    fn host_matmul_ones_vec_refuses_over_max_elems() {
        // Product just above HOST_REF_MAX_ELEMENTS without huge allocation.
        // 2001 * 2000 * 1 = 4_002_000 > 4_000_000
        let err = host_matmul_f32_ones_vec(2001, 2000, 1).expect_err("over max");
        assert!(
            err.to_string().contains("refuses") || err.to_string().contains("max"),
            "err={err}"
        );
    }

    #[test]
    fn host_matmul_ones_vec_shares_cap_with_host_ref() {
        assert_eq!(HOST_REF_MAX_ELEMENTS, 4_000_000);
    }

    #[test]
    fn decide_ran_on_npu_true_only_when_all_criteria() {
        let golden = host_matmul_f32_ones_vec(1, 1, 2).expect("g");
        let device = golden.clone();
        let ok = NumericalVerifyInput {
            host_golden: &golden,
            device_or_candidate: &device,
            had_instruction_blob: true,
            submit_wait_ok: true,
            real_ioctl_path: true,
            criteria: NumericalVerifyCriteria::default(),
        };
        assert!(decide_ran_on_npu(&ok));
        assert_eq!(decide_ran_on_npu_reason(&ok), Ok(()));
    }

    #[test]
    fn decide_ran_on_npu_false_on_empty_buffers() {
        let empty: &[f32] = &[];
        let input = NumericalVerifyInput {
            host_golden: empty,
            device_or_candidate: empty,
            had_instruction_blob: true,
            submit_wait_ok: true,
            real_ioctl_path: true,
            criteria: NumericalVerifyCriteria::default(),
        };
        assert!(!decide_ran_on_npu(&input));
        assert_eq!(
            decide_ran_on_npu_reason(&input),
            Err(RanOnNpuBlockReason::EmptyBuffers)
        );
        // One-sided empty also fails (empty candidate).
        let non_empty = [1.0_f32];
        let input2 = NumericalVerifyInput {
            host_golden: &non_empty,
            device_or_candidate: empty,
            had_instruction_blob: true,
            submit_wait_ok: true,
            real_ioctl_path: true,
            criteria: NumericalVerifyCriteria::default(),
        };
        assert!(!decide_ran_on_npu(&input2));
        assert_eq!(
            decide_ran_on_npu_reason(&input2),
            Err(RanOnNpuBlockReason::EmptyBuffers)
        );
        // Empty golden + non-empty candidate (classic OFM mis-bind).
        let input3 = NumericalVerifyInput {
            host_golden: empty,
            device_or_candidate: &non_empty,
            had_instruction_blob: true,
            submit_wait_ok: true,
            real_ioctl_path: true,
            criteria: NumericalVerifyCriteria::default(),
        };
        assert!(!decide_ran_on_npu(&input3));
        assert_eq!(
            decide_ran_on_npu_reason(&input3),
            Err(RanOnNpuBlockReason::EmptyBuffers)
        );
    }

    #[test]
    fn decide_ran_on_npu_false_without_submit() {
        let golden = [2.0_f32];
        let device = [2.0_f32];
        let input = NumericalVerifyInput {
            host_golden: &golden,
            device_or_candidate: &device,
            had_instruction_blob: true,
            submit_wait_ok: false,
            real_ioctl_path: true,
            criteria: NumericalVerifyCriteria::default(),
        };
        assert!(!decide_ran_on_npu(&input));
        assert_eq!(
            decide_ran_on_npu_reason(&input),
            Err(RanOnNpuBlockReason::SubmitWaitNotOk)
        );
    }

    #[test]
    fn decide_ran_on_npu_false_without_blob() {
        let golden = [2.0_f32];
        let device = [2.0_f32];
        let input = NumericalVerifyInput {
            host_golden: &golden,
            device_or_candidate: &device,
            had_instruction_blob: false,
            submit_wait_ok: true,
            real_ioctl_path: true,
            criteria: NumericalVerifyCriteria::default(),
        };
        assert!(!decide_ran_on_npu(&input));
        assert_eq!(
            decide_ran_on_npu_reason(&input),
            Err(RanOnNpuBlockReason::MissingInstructionBlob)
        );
    }

    #[test]
    fn decide_ran_on_npu_false_without_real_ioctl() {
        let golden = [2.0_f32];
        let device = [2.0_f32];
        let input = NumericalVerifyInput {
            host_golden: &golden,
            device_or_candidate: &device,
            had_instruction_blob: true,
            submit_wait_ok: true,
            real_ioctl_path: false,
            criteria: NumericalVerifyCriteria::default(),
        };
        assert!(!decide_ran_on_npu(&input));
        assert_eq!(
            decide_ran_on_npu_reason(&input),
            Err(RanOnNpuBlockReason::NotRealIoctlPath)
        );
    }

    #[test]
    fn decide_ran_on_npu_false_when_allclose_fails() {
        let golden = [2.0_f32];
        let device = [99.0_f32];
        let input = NumericalVerifyInput {
            host_golden: &golden,
            device_or_candidate: &device,
            had_instruction_blob: true,
            submit_wait_ok: true,
            real_ioctl_path: true,
            criteria: NumericalVerifyCriteria::default(),
        };
        assert!(!decide_ran_on_npu(&input));
        assert_eq!(
            decide_ran_on_npu_reason(&input),
            Err(RanOnNpuBlockReason::AllcloseFailed)
        );
    }

    #[test]
    fn decide_ran_on_npu_false_on_length_mismatch() {
        let golden = [1.0_f32, 2.0];
        let device = [1.0_f32];
        let input = NumericalVerifyInput {
            host_golden: &golden,
            device_or_candidate: &device,
            had_instruction_blob: true,
            submit_wait_ok: true,
            real_ioctl_path: true,
            criteria: NumericalVerifyCriteria::default(),
        };
        assert!(!decide_ran_on_npu(&input));
        assert_eq!(
            decide_ran_on_npu_reason(&input),
            Err(RanOnNpuBlockReason::AllcloseFailed)
        );
    }

    #[test]
    fn decide_ran_on_npu_false_on_nan_device() {
        let golden = [1.0_f32];
        let device = [f32::NAN];
        let input = NumericalVerifyInput {
            host_golden: &golden,
            device_or_candidate: &device,
            had_instruction_blob: true,
            submit_wait_ok: true,
            real_ioctl_path: true,
            criteria: NumericalVerifyCriteria::default(),
        };
        assert!(!decide_ran_on_npu(&input));
        assert_eq!(
            decide_ran_on_npu_reason(&input),
            Err(RanOnNpuBlockReason::AllcloseFailed)
        );
    }

    #[test]
    fn decide_ran_on_npu_false_on_inf_device() {
        let golden = [1.0_f32];
        let device = [f32::INFINITY];
        let input = NumericalVerifyInput {
            host_golden: &golden,
            device_or_candidate: &device,
            had_instruction_blob: true,
            submit_wait_ok: true,
            real_ioctl_path: true,
            criteria: NumericalVerifyCriteria::default(),
        };
        assert!(!decide_ran_on_npu(&input));
        assert_eq!(
            decide_ran_on_npu_reason(&input),
            Err(RanOnNpuBlockReason::AllcloseFailed)
        );
        let device_neg = [f32::NEG_INFINITY];
        let input2 = NumericalVerifyInput {
            device_or_candidate: &device_neg,
            ..input
        };
        assert!(!decide_ran_on_npu(&input2));
        assert_eq!(
            decide_ran_on_npu_reason(&input2),
            Err(RanOnNpuBlockReason::AllcloseFailed)
        );
    }

    #[test]
    fn decide_ran_on_npu_respects_criteria_field() {
        let golden = [1.0_f32];
        let device = [1.1_f32]; // residual 0.1
        let tight = NumericalVerifyInput {
            host_golden: &golden,
            device_or_candidate: &device,
            had_instruction_blob: true,
            submit_wait_ok: true,
            real_ioctl_path: true,
            criteria: NumericalVerifyCriteria {
                rtol: 0.0,
                atol: 1e-6,
            },
        };
        assert!(!decide_ran_on_npu(&tight));
        let loose = NumericalVerifyInput {
            criteria: NumericalVerifyCriteria {
                rtol: 0.0,
                atol: 0.2,
            },
            ..tight
        };
        assert!(decide_ran_on_npu(&loose));
    }

    #[test]
    fn default_criteria_constants_reject_known_outlier() {
        // Default atol=1e-4 rtol=1e-3: |2-1|=1 >> tol for exp=1
        let golden = [1.0_f32];
        let device = [2.0_f32];
        let input = NumericalVerifyInput {
            host_golden: &golden,
            device_or_candidate: &device,
            had_instruction_blob: true,
            submit_wait_ok: true,
            real_ioctl_path: true,
            criteria: NumericalVerifyCriteria::default(),
        };
        assert!(!decide_ran_on_npu(&input));
        assert!((DEFAULT_VERIFY_ATOL - 1e-4).abs() < 1e-12);
        assert!((DEFAULT_VERIFY_RTOL - 1e-3).abs() < 1e-12);
    }
}
