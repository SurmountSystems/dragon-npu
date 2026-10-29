//! Offline MLIR_AIE ioctl prep. Does not open a device and does not ioctl.

#[cfg(feature = "amd-xdna2")]
use std::env;
#[cfg(feature = "amd-xdna2")]
use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    refuse()
}

#[cfg(not(feature = "amd-xdna2"))]
fn refuse() -> ExitCode {
    eprintln!("amd-xdna2 is off. Pass is still No. The device was not opened.");
    ExitCode::FAILURE
}

#[cfg(feature = "amd-xdna2")]
fn refuse() -> ExitCode {
    let invocation = match Invocation::from_args() {
        Ok(invocation) => invocation,
        Err(detail) => {
            eprintln!("{detail}");
            return ExitCode::from(2);
        }
    };
    let host_opcode =
        earth_dragon_npu::backends::amd_xdna2::mlir_aie_ioctl::WHOLE_ARRAY_HOST_OPCODE;
    let arg_order =
        earth_dragon_npu::backends::amd_xdna2::mlir_aie_ioctl::WHOLE_ARRAY_HOST_ARG_ORDER;
    let exec_word = earth_dragon_npu::backends::amd_xdna2::mlir_aie_ioctl::PACKED_EXEC_CMD_REQUEST;
    eprintln!(
        "host opcode {host_opcode} cited at test.cpp:229; not ERT_START_NPU or ERT_START_DPU. argument order {}",
        arg_order.join(", ")
    );
    let submit =
        earth_dragon_npu::backends::amd_xdna2::mlir_aie_ioctl::submit_mlir_aie_whole_array_ioctl(
            Path::new(&invocation.xclbin),
            Path::new(&invocation.insts),
        );
    let exec_ask = earth_dragon_npu::backends::amd_xdna2::mlir_aie_ioctl::mlir_aie_ioctl_ask(
        earth_dragon_npu::backends::amd_xdna2::mlir_aie_ioctl::MlirAieIoctlAsk::PackedExecCmdNumber,
    );
    let wait_ask = earth_dragon_npu::backends::amd_xdna2::mlir_aie_ioctl::mlir_aie_ioctl_ask(
        earth_dragon_npu::backends::amd_xdna2::mlir_aie_ioctl::MlirAieIoctlAsk::PackedWaitCmdNumber,
    );
    let failing_request = cited_failing_request(&submit);
    let line = format_post_submit_refusal(
        failing_request,
        exec_word,
        &invocation.reason,
        &invocation.xclbin,
        &invocation.xclbin_sha256,
        &invocation.insts,
        &invocation.insts_sha256,
        &submit,
        &exec_ask,
        &wait_ask,
    );
    eprintln!("{line}");
    ExitCode::FAILURE
}

/// Request number from a cited ioctl failure. Any other refusal is request 0.
#[cfg(feature = "amd-xdna2")]
fn cited_failing_request(
    submit: &Result<(), earth_dragon_npu::backends::amd_xdna2::mlir_aie_ioctl::MlirAieIoctlError>,
) -> u64 {
    match submit {
        Err(earth_dragon_npu::backends::amd_xdna2::mlir_aie_ioctl::MlirAieIoctlError::CitedIoctlFailed {
            request,
            ..
        }) => *request,
        _ => 0,
    }
}

/// Refusal line printed after submit returns. Does not open a device and does not ioctl.
#[cfg(feature = "amd-xdna2")]
fn format_post_submit_refusal(
    failing_request: u64,
    exec_word: u64,
    reason: &str,
    xclbin: &str,
    xclbin_sha256: &str,
    insts: &str,
    insts_sha256: &str,
    submit: &Result<(), earth_dragon_npu::backends::amd_xdna2::mlir_aie_ioctl::MlirAieIoctlError>,
    exec_ask: &Result<
        Vec<u8>,
        earth_dragon_npu::backends::amd_xdna2::mlir_aie_ioctl::MlirAieIoctlError,
    >,
    wait_ask: &Result<
        Vec<u8>,
        earth_dragon_npu::backends::amd_xdna2::mlir_aie_ioctl::MlirAieIoctlError,
    >,
) -> String {
    let device_clause = if failing_request == 0 {
        "The device was not opened. "
    } else {
        ""
    };
    format!(
        "Pass is still No. {device_clause}EXEC_CMD packed request 0x{exec_word:x} is stored and was not issued. reason {reason} xclbin {xclbin} sha256 {xclbin_sha256} insts {insts} sha256 {insts_sha256} submit {submit:?} exec {exec_ask:?} wait {wait_ask:?}"
    )
}

#[cfg(feature = "amd-xdna2")]
struct Invocation {
    reason: String,
    xclbin: String,
    xclbin_sha256: String,
    insts: String,
    insts_sha256: String,
}

#[cfg(feature = "amd-xdna2")]
impl Invocation {
    fn from_args() -> Result<Self, String> {
        let args: Vec<String> = env::args().skip(1).collect();
        Ok(Self {
            reason: flag_value(&args, "--reason")?,
            xclbin: flag_value(&args, "--xclbin")?,
            xclbin_sha256: flag_value(&args, "--xclbin-sha256")?,
            insts: flag_value(&args, "--insts")?,
            insts_sha256: flag_value(&args, "--insts-sha256")?,
        })
    }
}

#[cfg(feature = "amd-xdna2")]
fn flag_value(args: &[String], flag: &str) -> Result<String, String> {
    let position = args
        .iter()
        .position(|arg| arg == flag)
        .ok_or_else(|| format!("missing {flag}"))?;
    args.get(position.saturating_add(1))
        .cloned()
        .filter(|value| !value.starts_with("--"))
        .ok_or_else(|| format!("missing value for {flag}"))
}

#[cfg(all(test, feature = "amd-xdna2"))]
mod tests {
    use super::format_post_submit_refusal;

    #[test]
    fn nonzero_failing_request_omits_device_was_not_opened() {
        let exec_word =
            earth_dragon_npu::backends::amd_xdna2::mlir_aie_ioctl::PACKED_EXEC_CMD_REQUEST;
        let exec_ask = earth_dragon_npu::backends::amd_xdna2::mlir_aie_ioctl::mlir_aie_ioctl_ask(
            earth_dragon_npu::backends::amd_xdna2::mlir_aie_ioctl::MlirAieIoctlAsk::PackedExecCmdNumber,
        );
        let wait_ask = earth_dragon_npu::backends::amd_xdna2::mlir_aie_ioctl::mlir_aie_ioctl_ask(
            earth_dragon_npu::backends::amd_xdna2::mlir_aie_ioctl::MlirAieIoctlAsk::PackedWaitCmdNumber,
        );
        for request in [0xc038_6440_u64, 0x1_u64] {
            let submit = Err(earth_dragon_npu::backends::amd_xdna2::mlir_aie_ioctl::MlirAieIoctlError::CitedIoctlFailed {
                request,
                errno: 11,
            });
            let line = format_post_submit_refusal(
                request,
                exec_word,
                "cited ioctl failed",
                "testdata/xdna2_cmd/whole_array_128_i16/final.xclbin",
                "sha",
                "testdata/xdna2_cmd/whole_array_128_i16/insts.bin",
                "isha",
                &submit,
                &exec_ask,
                &wait_ask,
            );
            assert!(
                !line.contains("The device was not opened."),
                "non-zero failing request {request:#x} still printed the sentence: {line}"
            );
        }
        let request_zero = Err(
            earth_dragon_npu::backends::amd_xdna2::mlir_aie_ioctl::MlirAieIoctlError::CitedIoctlFailed {
                request: 0,
                errno: 11,
            },
        );
        let zero_line = format_post_submit_refusal(
            0,
            exec_word,
            "cited ioctl failed",
            "xclbin",
            "sha",
            "insts",
            "isha",
            &request_zero,
            &exec_ask,
            &wait_ask,
        );
        assert!(
            zero_line.contains("The device was not opened."),
            "request 0 must still emit the sentence: {zero_line}"
        );
    }
}
