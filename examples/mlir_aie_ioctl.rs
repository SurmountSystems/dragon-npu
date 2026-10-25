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
    eprintln!(
        "Pass is still No. The device was not opened. EXEC_CMD packed request 0x{exec_word:x} is stored and was not issued. reason {} xclbin {} sha256 {} insts {} sha256 {} submit {submit:?} exec {exec_ask:?} wait {wait_ask:?}",
        invocation.reason,
        invocation.xclbin,
        invocation.xclbin_sha256,
        invocation.insts,
        invocation.insts_sha256
    );
    ExitCode::FAILURE
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
