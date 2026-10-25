//! START_NPU raw ioctl path for the recorded whole-array i16 pair.
//!
//! Does not register the xclbin. Does not launch kernel MLIR_AIE.
//! The double gate stays closed here: this process does not set
//! `XDNA2_ALLOW_EXEC_CMD` and does not open `/dev/accel`.

#[cfg(feature = "amd-xdna2")]
fn main() {
    use std::path::Path;
    use std::process::ExitCode;

    use earth_dragon_npu::backends::amd_xdna2::whole_array_i16::{
        WholeArrayError, check_insts_file, check_xclbin_pair_identity, sha256_hex,
        submit_whole_array,
    };

    eprintln!(
        "START_NPU raw ioctl path. Does not register the xclbin. Does not launch kernel MLIR_AIE."
    );

    let mut args = std::env::args().skip(1);
    let Some(xclbin_path) = args.next() else {
        eprintln!("usage: whole_array_i16_submit <xclbin> <insts>");
        std::process::exit(2);
    };
    let Some(insts_path) = args.next() else {
        eprintln!("usage: whole_array_i16_submit <xclbin> <insts>");
        std::process::exit(2);
    };

    let xclbin_bytes = match std::fs::read(&xclbin_path) {
        Ok(bytes) => bytes,
        Err(err) => {
            eprintln!("xclbin read failed: {err}");
            std::process::exit(1);
        }
    };
    let sha = sha256_hex(&xclbin_bytes);
    let Ok(size) = u64::try_from(xclbin_bytes.len()) else {
        eprintln!("xclbin size does not fit u64");
        std::process::exit(1);
    };
    if let Err(err) = check_xclbin_pair_identity(&sha, size) {
        eprintln!("{err}");
        std::process::exit(1);
    }
    if let Err(err) = check_insts_file(Path::new(&insts_path)) {
        eprintln!("{err}");
        std::process::exit(1);
    }

    match submit_whole_array(&sha, size, Path::new(&insts_path), None) {
        Err(WholeArrayError::DoubleGateClosed {
            had_instruction_blob,
            submit_wait_ok,
            real_ioctl_path,
        }) => {
            eprintln!(
                "double gate is closed; had_instruction_blob={had_instruction_blob} submit_wait_ok={submit_wait_ok} real_ioctl_path={real_ioctl_path}"
            );
            eprintln!("Pass is still No. invent_pass=0. Device was not opened.");
            let _ = ExitCode::from(1u8);
            std::process::exit(1);
        }
        Ok(_) => {
            eprintln!("refusing to treat a submit result as product Pass");
            std::process::exit(1);
        }
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    }
}

#[cfg(not(feature = "amd-xdna2"))]
fn main() {
    eprintln!("enable --features amd-xdna2");
    eprintln!(
        "START_NPU raw ioctl path. Does not register the xclbin. Does not launch kernel MLIR_AIE."
    );
    std::process::exit(2);
}
