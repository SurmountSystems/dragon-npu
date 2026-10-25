//! Whole-array i16 matmul inputs, CPU golden, and the raw START_NPU submit gate.
//!
//! The xclbin sha256 and size are a recorded pair identity. This module does not
//! register that file and does not launch kernel MLIR_AIE.

use std::path::Path;

use thiserror::Error;

/// Row, column, and shared dimension of the recorded whole-array problem.
pub const MATRIX_DIM: usize = 128;
/// Elements in one i16 matrix (`128 * 128`).
pub const MATRIX_ELEMS: usize = MATRIX_DIM * MATRIX_DIM;
/// Bytes in one i16 input matrix.
pub const INPUT_BYTES: usize = MATRIX_ELEMS * std::mem::size_of::<i16>();
/// Bytes in the i32 output matrix.
pub const OUTPUT_BYTES: usize = MATRIX_ELEMS * std::mem::size_of::<i32>();
/// Recorded instruction-blob length. Checked before any ioctl.
pub const INSTS_LEN: usize = 2592;

/// Xclbin sha256 this path refuses. Not loaded.
pub const REFUSED_XCLBIN_SHA256: &str =
    "286fe2324ef02cad008c99718cf648a6f0286aa5b6170dba62856ca1b90665c1";
/// Xclbin sha256 accepted only together with [`ACCEPTED_XCLBIN_BYTES`].
pub const ACCEPTED_XCLBIN_SHA256: &str =
    "0cadf32ec0472e3f55e9102a6175d005b1a95a8ea428907086e15ad44d825dad";
/// Size in bytes of the accepted xclbin pair identity.
pub const ACCEPTED_XCLBIN_BYTES: u64 = 73327;
/// Sha256 of the 2592-byte instruction file.
pub const INSTS_SHA256: &str = "ae692499a79c74b46175cb4841f235edb0059175144941522f792105b758dec3";

/// Relative path of the on-disk instruction file. Tests may use a temp file instead.
pub const INSTS_REL_PATH: &str = "testdata/xdna2_cmd/whole_array_128_i16/insts.bin";

/// Failure of the offline checks or of the gated raw ioctl path.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum WholeArrayError {
    /// Sha256 is the refused xclbin, whatever the size is.
    #[error("refused xclbin sha256 {sha256}")]
    RefusedXclbin {
        /// Hex digest that was refused.
        sha256: String,
    },
    /// Digest and size are not the one accepted recorded pair.
    #[error("xclbin pair identity mismatch sha256={sha256} size={size}")]
    XclbinPairMismatch {
        /// Hex digest that was presented.
        sha256: String,
        /// Byte length that was presented.
        size: u64,
    },
    /// Instruction path could not be read.
    #[error("insts file is missing at {path}")]
    InstsMissing {
        /// Path that was not read.
        path: String,
    },
    /// Instruction bytes are present but not 2592 bytes long.
    #[error("insts length {got} is not {INSTS_LEN}")]
    InstsLength {
        /// Length that was read.
        got: usize,
    },
    /// Instruction bytes are 2592 long but not the recorded digest.
    #[error("insts sha256 {got} is not the recorded digest")]
    InstsSha256 {
        /// Hex digest that was computed.
        got: String,
    },
    /// `A` or `B` was not `128 * 128` i16 elements.
    #[error("i16 matrices must each be {MATRIX_ELEMS} elements")]
    MatrixShape,
    /// Offline pack has no instruction-BO device address. Flags stay false.
    #[error("instruction device address is absent")]
    InstructionDeviceAddressAbsent {
        /// Not set: no blob was submitted.
        had_instruction_blob: bool,
        /// Not set: submit-and-wait did not complete.
        submit_wait_ok: bool,
        /// Not set: this error is not a real ioctl.
        real_ioctl_path: bool,
    },
    /// Feature `xdna2-dangerous-exec-cmd` is default off, and `XDNA2_ALLOW_EXEC_CMD` is unset.
    #[error(
        "the double gate is closed (feature xdna2-dangerous-exec-cmd is default off, and XDNA2_ALLOW_EXEC_CMD is unset)"
    )]
    DoubleGateClosed {
        /// Not set: the device was not submitted.
        had_instruction_blob: bool,
        /// Not set: submit-and-wait was not called.
        submit_wait_ok: bool,
        /// Not set: `/dev/accel` was not opened.
        real_ioctl_path: bool,
    },
    /// Gate was open but the raw ioctl path returned an error. Flags stay as recorded.
    #[error("START_NPU submit-and-wait failed: {detail}")]
    SubmitWaitFailed {
        /// Error text from the existing submit path.
        detail: String,
        /// True only if the command carried the instruction blob into the ioctl.
        had_instruction_blob: bool,
        /// True only if submit-and-wait completed.
        submit_wait_ok: bool,
        /// True only if the ioctl path was the real one.
        real_ioctl_path: bool,
    },
}

/// Host golden and device buffer in the shape [`crate::backends::amd_xdna2::numerical::decide_ran_on_npu`] takes.
#[derive(Debug, Clone)]
pub struct DecideBufferShape {
    /// Non-empty host golden, as f32.
    pub host_golden: Vec<f32>,
    /// Device bytes copied into the same element count. Not invented.
    pub device_buffer: Vec<f32>,
    /// True only after a real gated submit-and-wait completed.
    pub submit_wait_ok: bool,
    /// True only when an instruction blob was part of that submit.
    pub had_instruction_blob: bool,
    /// True only on the real ioctl path.
    pub real_ioctl_path: bool,
}

/// Claim flags that stay false on every offline refusal.
#[must_use]
pub const fn offline_refusal_flags() -> (bool, bool, bool) {
    (false, false, false)
}

fn address_absent() -> WholeArrayError {
    let (had_instruction_blob, submit_wait_ok, real_ioctl_path) = offline_refusal_flags();
    WholeArrayError::InstructionDeviceAddressAbsent {
        had_instruction_blob,
        submit_wait_ok,
        real_ioctl_path,
    }
}

fn gate_closed() -> WholeArrayError {
    let (had_instruction_blob, submit_wait_ok, real_ioctl_path) = offline_refusal_flags();
    WholeArrayError::DoubleGateClosed {
        had_instruction_blob,
        submit_wait_ok,
        real_ioctl_path,
    }
}

/// Deterministic i16 value for matrix `A` at `index`. Not time-based.
#[must_use]
pub fn input_a_i16(index: usize) -> i16 {
    let mixed = index.wrapping_mul(17).wrapping_add(3) % 11;
    i16::try_from(mixed).unwrap_or(0) - 5
}

/// Deterministic i16 value for matrix `B` at `index`. Not time-based.
#[must_use]
pub fn input_b_i16(index: usize) -> i16 {
    let mixed = index.wrapping_mul(13).wrapping_add(7) % 9;
    i16::try_from(mixed).unwrap_or(0) - 4
}

/// Matrix `A`, row-major, `M` by `K`. 32768 bytes.
#[must_use]
pub fn matrix_a() -> Vec<i16> {
    (0..MATRIX_ELEMS).map(input_a_i16).collect()
}

/// Matrix `B`, row-major, `K` by `N`. 32768 bytes.
#[must_use]
pub fn matrix_b() -> Vec<i16> {
    (0..MATRIX_ELEMS).map(input_b_i16).collect()
}

/// i32 dot of one row of `A` and one column of `B`. CPU only.
#[must_use]
pub fn dot_row_col(a: &[i16], b: &[i16], row: usize, col: usize) -> i32 {
    let mut acc = 0i32;
    for k in 0..MATRIX_DIM {
        let a_val = i32::from(a[row * MATRIX_DIM + k]);
        let b_val = i32::from(b[k * MATRIX_DIM + col]);
        acc += a_val * b_val;
    }
    acc
}

/// CPU golden. Each element is the i32 dot of one i16 row of `A` and one i16 column of `B`.
///
/// # Errors
///
/// Returns [`WholeArrayError::MatrixShape`] when either input is not 16384 elements.
pub fn cpu_golden_i32(a: &[i16], b: &[i16]) -> Result<Vec<i32>, WholeArrayError> {
    if a.len() != MATRIX_ELEMS || b.len() != MATRIX_ELEMS {
        return Err(WholeArrayError::MatrixShape);
    }
    let mut out = vec![0i32; MATRIX_ELEMS];
    for row in 0..MATRIX_DIM {
        for col in 0..MATRIX_DIM {
            out[row * MATRIX_DIM + col] = dot_row_col(a, b, row, col);
        }
    }
    Ok(out)
}

/// Accept only the recorded sha256 and size. Refuse the other recorded sha256.
///
/// This does not open the xclbin and does not register it.
///
/// # Errors
///
/// Returns a refused or mismatch error when the pair is not the accepted one.
pub fn check_xclbin_pair_identity(sha256_hex: &str, size: u64) -> Result<(), WholeArrayError> {
    let sha = sha256_hex.trim().to_ascii_lowercase();
    if sha == REFUSED_XCLBIN_SHA256 {
        return Err(WholeArrayError::RefusedXclbin { sha256: sha });
    }
    if sha == ACCEPTED_XCLBIN_SHA256 && size == ACCEPTED_XCLBIN_BYTES {
        return Ok(());
    }
    Err(WholeArrayError::XclbinPairMismatch { sha256: sha, size })
}

/// Read and check the instruction file before any ioctl.
///
/// # Errors
///
/// Missing path, length other than 2592, or a different sha256.
pub fn check_insts_file(path: &Path) -> Result<Vec<u8>, WholeArrayError> {
    let bytes = std::fs::read(path).map_err(|_| WholeArrayError::InstsMissing {
        path: path.display().to_string(),
    })?;
    if bytes.len() != INSTS_LEN {
        return Err(WholeArrayError::InstsLength { got: bytes.len() });
    }
    let got = sha256_hex(&bytes);
    if got != INSTS_SHA256 {
        return Err(WholeArrayError::InstsSha256 { got });
    }
    Ok(bytes)
}

/// Pack START_NPU command bytes without opening a device.
///
/// `inst_dev_addr` must already be a real instruction-BO address. Offline callers
/// pass [`None`]. This function does not invent an address.
///
/// # Errors
///
/// [`WholeArrayError::InstructionDeviceAddressAbsent`] when the address is missing.
/// The three claim flags on that error are false.
pub fn build_start_npu_command(
    inst_dev_addr: Option<u64>,
    inst_size: u32,
) -> Result<Vec<u8>, WholeArrayError> {
    let Some(addr) = inst_dev_addr else {
        return Err(address_absent());
    };
    pack_start_npu(0, addr, inst_size)
}

fn pack_failed(detail: String) -> WholeArrayError {
    let (had_instruction_blob, submit_wait_ok, real_ioctl_path) = offline_refusal_flags();
    WholeArrayError::SubmitWaitFailed {
        detail,
        had_instruction_blob,
        submit_wait_ok,
        real_ioctl_path,
    }
}

fn pack_start_npu(
    cu_index: u16,
    instr_addr: u64,
    instr_size: u32,
) -> Result<Vec<u8>, WholeArrayError> {
    let Some(cu_mask0) = 1u32.checked_shl(u32::from(cu_index)) else {
        return Err(pack_failed(
            "cu index does not fit in the first CU mask".to_string(),
        ));
    };
    super::direct::build_ert_start_npu_cmd_bo(instr_addr, instr_size, &[], cu_mask0)
        .map_err(|err| pack_failed(err.to_string()))
}

fn double_gate_is_open() -> bool {
    super::direct::exec_cmd_is_enabled()
}

/// Submit entry. Calls [`super::direct::exec_cmd_is_enabled`] before any device open.
///
/// When the double gate is closed this returns [`WholeArrayError::DoubleGateClosed`]
/// and does not open `/dev/accel`, does not set `real_ioctl_path`, and does not set
/// `submit_wait_ok`.
///
/// # Errors
///
/// Gate closed, instruction check failure, missing device address, or submit-and-wait failure.
pub fn submit_whole_array(
    xclbin_sha256: &str,
    xclbin_size: u64,
    insts_path: &Path,
    inst_dev_addr: Option<u64>,
) -> Result<DecideBufferShape, WholeArrayError> {
    if !double_gate_is_open() {
        return Err(gate_closed());
    }
    check_xclbin_pair_identity(xclbin_sha256, xclbin_size)?;
    let insts = check_insts_file(insts_path)?;
    let host = cpu_golden_i32(&matrix_a(), &matrix_b())?;
    submit_after_gate_open(&insts, inst_dev_addr, &host)
}

fn submit_after_gate_open(
    insts: &[u8],
    inst_dev_addr: Option<u64>,
    host_golden_i32: &[i32],
) -> Result<DecideBufferShape, WholeArrayError> {
    let inst_size = u32::try_from(insts.len())
        .map_err(|_| WholeArrayError::InstsLength { got: insts.len() })?;
    let cmd = build_start_npu_command(inst_dev_addr, inst_size)?;
    submit_and_wait_copy(&cmd, host_golden_i32)
}

fn submit_and_wait_copy(
    cmd: &[u8],
    host_golden_i32: &[i32],
) -> Result<DecideBufferShape, WholeArrayError> {
    let copied = invoke_submit_and_wait(cmd)?;
    Ok(DecideBufferShape {
        host_golden: i32_to_f32(host_golden_i32),
        device_buffer: i32_to_f32(&copied.device_i32),
        submit_wait_ok: copied.submit_wait_ok,
        had_instruction_blob: copied.had_instruction_blob,
        real_ioctl_path: copied.real_ioctl_path,
    })
}

struct SubmitCopy {
    device_i32: Vec<i32>,
    submit_wait_ok: bool,
    had_instruction_blob: bool,
    real_ioctl_path: bool,
}

fn invoke_submit_and_wait(cmd: &[u8]) -> Result<SubmitCopy, WholeArrayError> {
    // Packing START_NPU bytes is not an ioctl. Flags stay false until a real
    // gated `RealDirectSubmission` submit-and-wait runs. This function does
    // not open `/dev/accel` and does not register an xclbin.
    let (had_instruction_blob, submit_wait_ok, real_ioctl_path) = offline_refusal_flags();
    Err(WholeArrayError::SubmitWaitFailed {
        detail: format!(
            "START_NPU command is {} bytes; device was not opened; no EXEC_CMD ioctl",
            cmd.len()
        ),
        had_instruction_blob,
        submit_wait_ok,
        real_ioctl_path,
    })
}

fn i32_to_f32(values: &[i32]) -> Vec<f32> {
    values.iter().map(|value| i32_bits_to_f32(*value)).collect()
}

fn i32_bits_to_f32(value: i32) -> f32 {
    // Golden dots for this matrix fit in the f32 mantissa (max product is far below 2^24).
    #[allow(clippy::cast_precision_loss)]
    {
        value as f32
    }
}

/// Lower-hex SHA-256 of `bytes`. Does not open a device and does not register a file.
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    sha256(bytes).iter().fold(String::new(), |mut hex, byte| {
        write!(hex, "{byte:02x}").unwrap_or(());
        hex
    })
}

#[allow(
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    clippy::many_single_char_names,
    clippy::similar_names
)]
fn sha256(message: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a_2f98,
        0x7137_4491,
        0xb5c0_fbcf,
        0xe9b5_dba5,
        0x3956_c25b,
        0x59f1_11f1,
        0x923f_82a4,
        0xab1c_5ed5,
        0xd807_aa98,
        0x1283_5b01,
        0x2431_85be,
        0x550c_7dc3,
        0x72be_5d74,
        0x80de_b1fe,
        0x9bdc_06a7,
        0xc19b_f174,
        0xe49b_69c1,
        0xefbe_4786,
        0x0fc1_9dc6,
        0x240c_a1cc,
        0x2de9_2c6f,
        0x4a74_84aa,
        0x5cb0_a9dc,
        0x76f9_88da,
        0x983e_5152,
        0xa831_c66d,
        0xb003_27c8,
        0xbf59_7fc7,
        0xc6e0_0bf3,
        0xd5a7_9147,
        0x06ca_6351,
        0x1429_2967,
        0x27b7_0a85,
        0x2e1b_2138,
        0x4d2c_6dfc,
        0x5338_0d13,
        0x650a_7354,
        0x766a_0abb,
        0x81c2_c92e,
        0x9272_2c85,
        0xa2bf_e8a1,
        0xa81a_664b,
        0xc24b_8b70,
        0xc76c_51a3,
        0xd192_e819,
        0xd699_0624,
        0xf40e_3585,
        0x106a_a070,
        0x19a4_c116,
        0x1e37_6c08,
        0x2748_774c,
        0x34b0_bcb5,
        0x391c_0cb3,
        0x4ed8_aa4a,
        0x5b9c_ca4f,
        0x682e_6ff3,
        0x748f_82ee,
        0x78a5_636f,
        0x84c8_7814,
        0x8cc7_0208,
        0x90be_fffa,
        0xa450_6ceb,
        0xbef9_a3f7,
        0xc671_78f2,
    ];
    let mut state = [
        0x6a09_e667_u32,
        0xbb67_ae85,
        0x3c6e_f372,
        0xa54f_f53a,
        0x510e_527f,
        0x9b05_688c,
        0x1f83_d9ab,
        0x5be0_cd19,
    ];
    let bit_len = u64::try_from(message.len()).unwrap_or(0).saturating_mul(8);
    let mut data = Vec::with_capacity(message.len() + 72);
    data.extend_from_slice(message);
    data.push(0x80);
    while data.len() % 64 != 56 {
        data.push(0);
    }
    data.extend_from_slice(&bit_len.to_be_bytes());
    for chunk in data.as_chunks::<64>().0 {
        let mut schedule = [0u32; 64];
        for (index, word) in schedule.iter_mut().take(16).enumerate() {
            let start = index * 4;
            let mut raw = [0u8; 4];
            raw.copy_from_slice(&chunk[start..start + 4]);
            *word = u32::from_be_bytes(raw);
        }
        for index in 16..64 {
            let s0 = schedule[index - 15].rotate_right(7)
                ^ schedule[index - 15].rotate_right(18)
                ^ (schedule[index - 15] >> 3);
            let s1 = schedule[index - 2].rotate_right(17)
                ^ schedule[index - 2].rotate_right(19)
                ^ (schedule[index - 2] >> 10);
            schedule[index] = schedule[index - 16]
                .wrapping_add(s0)
                .wrapping_add(schedule[index - 7])
                .wrapping_add(s1);
        }
        let mut working = state;
        for index in 0..64 {
            let s1 = working[4].rotate_right(6)
                ^ working[4].rotate_right(11)
                ^ working[4].rotate_right(25);
            let choose = (working[4] & working[5]) ^ ((!working[4]) & working[6]);
            let temp1 = working[7]
                .wrapping_add(s1)
                .wrapping_add(choose)
                .wrapping_add(K[index])
                .wrapping_add(schedule[index]);
            let s0 = working[0].rotate_right(2)
                ^ working[0].rotate_right(13)
                ^ working[0].rotate_right(22);
            let majority =
                (working[0] & working[1]) ^ (working[0] & working[2]) ^ (working[1] & working[2]);
            let temp2 = s0.wrapping_add(majority);
            working[7] = working[6];
            working[6] = working[5];
            working[5] = working[4];
            working[4] = working[3].wrapping_add(temp1);
            working[3] = working[2];
            working[2] = working[1];
            working[1] = working[0];
            working[0] = temp1.wrapping_add(temp2);
        }
        for (slot, addend) in state.iter_mut().zip(working) {
            *slot = slot.wrapping_add(addend);
        }
    }
    let mut out = [0u8; 32];
    for (index, word) in state.iter().enumerate() {
        out[index * 4..index * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matrices_are_deterministic_i16_and_sized() {
        let a = matrix_a();
        let b = matrix_b();
        assert_eq!(a, matrix_a());
        assert_eq!(b, matrix_b());
        assert_eq!(a.len() * std::mem::size_of::<i16>(), 32768);
        assert_eq!(b.len() * std::mem::size_of::<i16>(), 32768);
        assert_eq!(INPUT_BYTES, 32768);
        assert_ne!(a, b);
    }

    #[test]
    fn cpu_golden_is_i32_dot_and_65536_bytes() {
        let a = matrix_a();
        let b = matrix_b();
        let golden = cpu_golden_i32(&a, &b).expect("shape");
        assert_eq!(golden.len() * std::mem::size_of::<i32>(), 65536);
        assert_eq!(OUTPUT_BYTES, 65536);
        assert_eq!(golden, cpu_golden_i32(&a, &b).expect("shape"));
        let mut manual = 0i32;
        for k in 0..MATRIX_DIM {
            manual += i32::from(a[k]) * i32::from(b[k * MATRIX_DIM]);
        }
        assert_eq!(golden[0], manual);
        assert_eq!(golden[0], dot_row_col(&a, &b, 0, 0));
        assert!(golden.iter().any(|value| *value != 0));
    }

    #[test]
    fn xclbin_pair_refuses_one_digest_and_accepts_only_the_recorded_pair() {
        let refused = check_xclbin_pair_identity(REFUSED_XCLBIN_SHA256, ACCEPTED_XCLBIN_BYTES);
        assert!(matches!(
            refused,
            Err(WholeArrayError::RefusedXclbin { .. })
        ));
        let wrong_size = check_xclbin_pair_identity(ACCEPTED_XCLBIN_SHA256, 73326);
        assert!(matches!(
            wrong_size,
            Err(WholeArrayError::XclbinPairMismatch { .. })
        ));
        assert!(check_xclbin_pair_identity(ACCEPTED_XCLBIN_SHA256, ACCEPTED_XCLBIN_BYTES).is_ok());
    }

    #[test]
    fn insts_checks_reject_missing_wrong_length_and_wrong_sha_before_ioctl() {
        let missing = Path::new("/tmp/dragon-npu-whole-array-no-such-insts.bin");
        drop(std::fs::remove_file(missing));
        assert!(matches!(
            check_insts_file(missing),
            Err(WholeArrayError::InstsMissing { .. })
        ));
        let short_path = std::env::temp_dir().join(format!(
            "dragon-npu-whole-array-short-{}.bin",
            std::process::id()
        ));
        assert!(std::fs::write(&short_path, [1u8, 2, 3]).is_ok());
        assert!(matches!(
            check_insts_file(&short_path),
            Err(WholeArrayError::InstsLength { got: 3 })
        ));
        drop(std::fs::remove_file(&short_path));
        let wrong = vec![0u8; INSTS_LEN];
        let wrong_path = std::env::temp_dir().join(format!(
            "dragon-npu-whole-array-wrong-{}.bin",
            std::process::id()
        ));
        assert!(std::fs::write(&wrong_path, &wrong).is_ok());
        assert!(matches!(
            check_insts_file(&wrong_path),
            Err(WholeArrayError::InstsSha256 { .. })
        ));
        drop(std::fs::remove_file(&wrong_path));
    }

    #[test]
    fn on_disk_insts_match_recorded_sha_when_present() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(INSTS_REL_PATH);
        let bytes = check_insts_file(&path).expect("recorded insts");
        assert_eq!(bytes.len(), INSTS_LEN);
        assert_eq!(sha256_hex(&bytes), INSTS_SHA256);
    }

    #[test]
    fn sha256_matches_known_vectors() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn missing_instruction_address_does_not_claim_ioctl_flags() {
        let Ok(inst_size) = u32::try_from(INSTS_LEN) else {
            panic!("recorded insts length fits in u32");
        };
        let err = build_start_npu_command(None, inst_size).expect_err("no address");
        match err {
            WholeArrayError::InstructionDeviceAddressAbsent {
                had_instruction_blob,
                submit_wait_ok,
                real_ioctl_path,
            } => {
                assert!(!had_instruction_blob);
                assert!(!submit_wait_ok);
                assert!(!real_ioctl_path);
            }
            other => panic!("unexpected {other:?}"),
        }
        assert!(
            err.to_string()
                .contains("instruction device address is absent")
        );
    }

    #[test]
    fn submit_stops_when_double_gate_is_closed() {
        let err = submit_whole_array(
            ACCEPTED_XCLBIN_SHA256,
            ACCEPTED_XCLBIN_BYTES,
            Path::new("/tmp/dragon-npu-whole-array-gate-closed-insts.bin"),
            None,
        )
        .expect_err("gate");
        match err {
            WholeArrayError::DoubleGateClosed {
                had_instruction_blob,
                submit_wait_ok,
                real_ioctl_path,
            } => {
                assert!(!had_instruction_blob);
                assert!(!submit_wait_ok);
                assert!(!real_ioctl_path);
            }
            other => panic!("unexpected {other:?}"),
        }
        let text = err.to_string();
        assert!(text.contains("double gate is closed"));
        assert!(text.contains("xdna2-dangerous-exec-cmd"));
        assert!(text.contains("XDNA2_ALLOW_EXEC_CMD"));
    }

    #[test]
    fn decide_ran_on_npu_is_false_for_buffers_this_test_created() {
        let golden = cpu_golden_i32(&matrix_a(), &matrix_b()).expect("golden");
        let host_golden = i32_to_f32(&golden);
        let device_buffer = vec![0f32; host_golden.len()];
        assert_ne!(host_golden.len(), 0);
        assert_ne!(device_buffer.len(), 0);
        // These fixture flags are not a silicon claim. Device bytes are zeros from this test.
        let false_flags = crate::backends::amd_xdna2::numerical::NumericalVerifyInput {
            host_golden: &host_golden,
            device_or_candidate: &device_buffer,
            had_instruction_blob: false,
            submit_wait_ok: false,
            real_ioctl_path: false,
            criteria: crate::backends::amd_xdna2::numerical::NumericalVerifyCriteria::default(),
        };
        let ran = crate::backends::amd_xdna2::numerical::decide_ran_on_npu(&false_flags);
        assert!(!ran);
        // Zeros still fail the match when the three claim flags are set on this fixture only.
        let true_flags = crate::backends::amd_xdna2::numerical::NumericalVerifyInput {
            host_golden: &host_golden,
            device_or_candidate: &device_buffer,
            had_instruction_blob: true,
            submit_wait_ok: true,
            real_ioctl_path: true,
            criteria: crate::backends::amd_xdna2::numerical::NumericalVerifyCriteria::default(),
        };
        let ran_flags = crate::backends::amd_xdna2::numerical::decide_ran_on_npu(&true_flags);
        assert!(!ran_flags);
    }
}
