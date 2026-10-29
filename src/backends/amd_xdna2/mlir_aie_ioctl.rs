//! ioctl-native MLIR_AIE command prep.
//!
//! Offline only. This module does not open `/dev/accel` and does not issue
//! ioctls. The host opcode is the integer 3 from `test.cpp` line 229. It is
//! not named `ERT_START_NPU` or `ERT_START_DPU`. The packed EXEC_CMD request
//! word is stored and is not issued. Pass stays no. invent_pass stays 0.

use std::path::{Path, PathBuf};

/// Shared-memory buffer type from `/usr/include/drm/amdxdna_accel.h` line 163.
/// Not the instruction CREATE_BO choice. Types 3, 4, and 9 are not stored.
pub const AMDXDNA_BO_SHMEM: u32 = 1;
/// Share buffer type from the same header, line 164.
/// Same numeric code as [`AMDXDNA_BO_SHMEM`].
pub const AMDXDNA_BO_SHARE: u32 = 1;
/// Device-heap buffer type from the same header, line 165.
/// Not the instruction CREATE_BO choice.
pub const AMDXDNA_BO_DEV_HEAP: u32 = 2;

/// Host opcode integer from `test.cpp` line 229.
///
/// Re-read quote: `unsigned int opcode = 3;`
/// That line does not name `ERT_START_NPU`, `ERT_START_DPU`, or `ERT_EXIT`.
/// `ert.h` line 825 is `ERT_START_DPU = 18`. Line 827 is `ERT_START_NPU = 20`.
/// Line 810 is `ERT_EXIT = 3`. Those enumerator values are not stored. This
/// constant is only the integer 3. The byte width of host `unsigned int` is
/// not on the cited line.
pub const WHOLE_ARRAY_HOST_OPCODE: u32 = 3;

/// Packed EXEC_CMD request word. Stored. Not issued. Not a device open.
///
/// Compiler stdout quoted from
/// `/home/hunter/.agents/reports/dragon-npu-mlir-aie-macro-eval.md`
/// (not recomputed). Installed and local both printed
/// `DRM_IOCTL_AMDXDNA_EXEC_CMD decimal=3224921158 hex=0xc0386446`.
/// `sizeof struct amdxdna_drm_exec_cmd decimal=56 hex=0x38` was also
/// printed. That sizeof is the type-check width inside the macro. It is
/// not stored, and this constant is not a hand-summed size.
///
/// Installed `/usr/include/drm/amdxdna_accel.h` lines 742-744:
/// `DRM_IOCTL_AMDXDNA_EXEC_CMD` is `DRM_IOWR` of
/// `struct amdxdna_drm_exec_cmd`. Local
/// `/home/hunter/Projects/kernel/xdna-driver/src/include/uapi/drm_local/amdxdna_accel.h`
/// lines 910-912: the same `DRM_IOWR` and the same struct.
/// The integer equals `src/backends/amd_xdna2/direct.rs` line 566
/// (`pub const DRM_IOCTL_AMDXDNA_EXEC_CMD: libc::c_ulong = 0xc038_6446`).
/// That constant sits in a private module and is not visible here, so this
/// module stores the same integer. The uapi index 6 is not this word.
pub const PACKED_EXEC_CMD_REQUEST: u64 = 0xc038_6446;

/// Packed WAIT_CMD request word. Stored. Not issued. Not a device open.
///
/// `/usr/lib/modules/7.2.11-zen1-1-zen/build/include/uapi/drm/amdxdna_accel.h`
/// defines `DRM_IOCTL_AMDXDNA_WAIT_CMD` via `DRM_IOW`. The expanded word is
/// `0x40106449`. It is not `DRM_IOWR` `0xc0106449`.
pub const PACKED_WAIT_CMD_REQUEST: u64 = 0x4010_6449;

/// Argument names in host order. Not addresses and not a command buffer.
///
/// Re-read `test.cpp` line 232: `(opcode, instr, ninstr, A, B, C, [trace])`.
/// Line 234: `run.set_arg(0, opcode);`
/// Line 235: `run.set_arg(1, bo_instr);`
/// Line 236: `run.set_arg(2, instr_v.size());`
/// Line 237: `run.set_arg(3, bo_a);`
/// Line 238: `run.set_arg(4, bo_b);`
/// Line 239: `run.set_arg(5, bo_out);`
/// Those lines name the order. They do not print numeric address words.
pub const WHOLE_ARRAY_HOST_ARG_ORDER: [&str; 6] = ["opcode", "instr", "ninstr", "A", "B", "C"];

/// Member names and C types of `amdxdna_cmd_start_npu`, in source order.
///
/// `src/driver/amdxdna/amdxdna_ctx.h` lines 47-52 and
/// `drivers/accel/amdxdna/amdxdna_ctx.h` lines 43-48:
/// `u64 buffer`, `u32 buffer_size`, `u32 prop_count`, `u32 prop_args[]`.
/// Neither header has a numeric `offsetof`. This is not a `repr(C)` struct
/// and it does not state a byte offset.
pub const AMDXDNA_CMD_START_NPU_MEMBER_ORDER: &[(&str, &str)] = &[
    ("buffer", "u64"),
    ("buffer_size", "u32"),
    ("prop_count", "u32"),
    ("prop_args", "u32[]"),
];

const IOCTL_COMMAND_REL: &str = "testdata/xdna2_cmd/whole_array_128_i16/mlir-aie-ioctl-command.txt";
/// Recorded external argv. Capital `-M` `-K` `-N` are the shape. Lowercase `-k` is the kernel name.
const RECORDED_MLIR_AIE_ARGV: &str =
    "/home/hunter/.agents/reports/dragon-npu-whole-array-live-run.md";

/// Caller-supplied CONFIG_CU fields. This prep does not invent a packet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CallerConfigCu {
    /// Buffer object the caller named.
    pub cu_bo: u32,
    /// Function index the caller named.
    pub cu_func: u32,
    /// Compute-unit count the caller named.
    pub num_cus: u32,
}

/// Host matrices for the recorded whole-array shape. Device bytes are not a readback.
#[derive(Debug, Clone, PartialEq)]
pub struct WholeArrayHostBuffers {
    /// Row count from opened `-M`.
    pub m: u32,
    /// Reduction length from opened `-K`.
    pub k: u32,
    /// Column count from opened `-N`.
    pub n: u32,
    /// Row-major i16 activation matrix, length `m * k`.
    pub a_i16: Vec<i16>,
    /// Row-major i16 weight matrix, length `k * n`.
    pub b_i16: Vec<i16>,
    /// Row-major i32 product, length `m * n`.
    pub golden_i32: Vec<i32>,
    /// Host golden as exact f32 values of [`Self::golden_i32`].
    pub host_golden: Vec<f32>,
    /// Zeros. Not bytes read from a device.
    pub device_or_candidate: Vec<f32>,
    /// Always false. This path does not read the device.
    pub device_bytes_are_readback: bool,
}

/// Offline question. Unproven packed numbers return [`MlirAieIoctlError`] and do not ioctl.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MlirAieIoctlAsk {
    /// ERT NPU prefix bytes. Layout is uncited, so the ask returns an error.
    ErtNpuPrefix {
        /// Device address of the instruction buffer.
        instruction_buffer: u64,
        /// Instruction buffer size in bytes.
        instruction_buffer_size: u32,
        /// Stored as its own word. Does not grow the prefix.
        instruction_prop_count: u32,
        /// One cited cu_mask word.
        cu_mask: u32,
    },
    /// Packed EXEC_CMD request number. Cited. Not issued.
    PackedExecCmdNumber,
    /// Packed WAIT_CMD request number. Cited as DRM_IOW `0x40106449`. Not issued.
    PackedWaitCmdNumber,
    /// AIE_PARTITION bytes from this xclbin. Unproven.
    XclbinAiePartition,
    /// Host opcode integer is cited. This ask does not return command bytes.
    WholeArrayHostOpcode,
    /// `amdxdna_cmd_start_npu` numeric byte offsets. Uncited.
    AmdxdnaCmdStartNpuOffsets,
    /// Argument-address words after the instruction record. Names are separate.
    ArgumentAddressWords,
    /// Tile count read from this xclbin. Unproven.
    NumTilesFromThisXclbin,
    /// CREATE_BO type code for the instruction buffer. Unproven.
    InstructionCreateBoType,
}

/// Recorded userspace mmap of the device-heap GEM object. Not an ioctl.
///
/// `amdxdna_init_dev_bo` returns `-EAGAIN` while the heap user address is
/// still invalid. The mmap sits after heap CREATE_BO and GET_BO_INFO and
/// before CREATE_HWCTX. Size and alignment are 64 MiB (`0x4000000`).
/// This record does not open `/dev/accel` and does not call `mmap`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceHeapMmapRecord {
    /// Heap CREATE_BO size. 64 MiB (`0x4000000`). Not a second heap size.
    pub size: u64,
    /// Heap CREATE_BO type. [`AMDXDNA_BO_DEV_HEAP`] is 2.
    pub bo_type: u32,
    /// Userspace VA alignment. 64 MiB (`0x4000000`), same integer as the size.
    pub align: u64,
}

/// One cited amdxdna request the whole-array submit would send. Not issued.
///
/// A device-heap mmap row is not a request. Its `request` is 0 and
/// [`CitedIoctlStep::device_heap_mmap`] is set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CitedIoctlStep {
    /// Packed request word from the header cite and the dump line.
    /// Zero on the recorded device-heap mmap, which is not an ioctl.
    pub request: u64,
    /// Argument size in bytes from that same cite.
    /// Zero on the recorded device-heap mmap.
    pub size: u32,
    /// Byte offset of `num_tiles` inside `amdxdna_drm_create_hwctx`.
    /// Set only for CREATE_HWCTX. The cite puts the field at `0x24`.
    pub num_tiles_offset: Option<u32>,
    /// `num_tiles` value at that offset. The cite is `0x20`.
    /// Set only for CREATE_HWCTX.
    pub num_tiles: Option<u32>,
    /// Set only on the recorded device-heap mmap. Ioctl rows leave this empty.
    pub device_heap_mmap: Option<DeviceHeapMmapRecord>,
}

/// Send plan for the recorded whole-array pair. The device stays closed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CitedSubmitPlan {
    /// Traced before-phase requests for the recorded pair, in order.
    /// [`PACKED_WAIT_CMD_REQUEST`] is not a row.
    pub steps: Vec<CitedIoctlStep>,
    /// Sha256 of the xclbin bytes that were read. Empty when the pair was not identified.
    pub xclbin_sha256: String,
    /// Byte length of those xclbin bytes. Zero when the pair was not identified.
    pub xclbin_size: u64,
    /// Sha256 of the instruction bytes that were read. Empty when the pair was not identified.
    pub insts_sha256: String,
    /// Byte length of those instruction bytes. Zero when the pair was not identified.
    pub insts_size: u64,
    /// Always false. This path does not open `/dev/accel`.
    pub device_opened: bool,
    /// Always false. This path does not exec XRT or `whole_array.exe`.
    pub xrt_executed: bool,
}

/// Typed refusal. Uncited packed request numbers are not invented.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum MlirAieIoctlError {
    #[error("cited ioctl {request:#x} failed errno {errno}")]
    CitedIoctlFailed { request: u64, errno: i32 },
    /// The packed ioctl number for this slot is not a cited constant.
    #[error("unproven packed ioctl {name} at slot {slot}")]
    UnprovenPackedIoctl {
        /// Ioctl name that would have been packed.
        name: &'static str,
        /// Sequence slot. Not a packed request number.
        slot: u32,
    },
    /// xclbin AIE_PARTITION is uncited. Packed request `0xc03064ca` is uncited
    /// and is not named. It is not a wait request word.
    #[error(
        "unproven xclbin AIE_PARTITION is uncited; packed request 0xc03064ca is uncited and is not named"
    )]
    UnprovenAiePartition {
        /// Cited send plan when the recorded pair was identified.
        /// Empty when this refusal did not identify that pair.
        plan: CitedSubmitPlan,
    },
    /// The host opcode integer is cited. Its stored byte width is not.
    #[error("host opcode integer is 3 at test.cpp:229; the word byte width is not cited")]
    HostOpcodeWordWidthUncited,
    /// Prefix bytes would store an uncited layout. The host opcode is not that prefix.
    #[error(
        "ERT NPU prefix is not filled; host opcode is integer 3 at test.cpp:229 and is not named ERT_START_NPU; byte layout is uncited"
    )]
    ErtNpuPrefixUncited,
    /// `amdxdna_cmd_start_npu` offsets are not cited.
    #[error("unproven amdxdna_cmd_start_npu offsets")]
    UnprovenAmdxdnaCmdStartNpuOffsets,
    /// Argument-address words after the instruction record are not cited.
    #[error("unproven argument-address words")]
    UnprovenArgumentAddressWords,
    /// `num_tiles` was not read from this xclbin.
    #[error("unproven num_tiles from this xclbin")]
    UnprovenNumTilesFromXclbin,
    /// Instruction CREATE_BO type code is not cited.
    #[error("unproven instruction CREATE_BO type")]
    UnprovenInstructionCreateBoType,
    /// A directory name is not an opened file, so it cannot supply the element count.
    #[error("element count is not in an opened file")]
    ElementCountNotInOpenedFile,
    /// The opened text has no capital `-M`, `-K`, and `-N` integers.
    #[error("shape was not stated in opened flags")]
    ShapeNotStated {
        /// Text that was searched.
        text: String,
    },
    /// The i32 is not an exact f32 value.
    #[error("i32 {value} is not an exact f32")]
    InexactI32ToF32 {
        /// Value that could not be converted.
        value: i32,
    },
    /// A command file that this prep must open could not be read.
    #[error("could not open {path}: {detail}")]
    CommandUnreadable {
        /// Path that was opened.
        path: String,
        /// Filesystem error text.
        detail: String,
    },
    /// Matrix dimensions or the product do not fit.
    #[error("host buffer shape: {detail}")]
    HostBufferShape {
        /// Which bound failed.
        detail: &'static str,
    },
}

/// Copy the three caller fields. Does not build a CONFIG_CU ioctl.
#[must_use]
pub const fn config_cu_from_caller(cu_bo: u32, cu_func: u32, num_cus: u32) -> CallerConfigCu {
    CallerConfigCu {
        cu_bo,
        cu_func,
        num_cus,
    }
}

/// Directory names are not opened files. The element count has to come from flags.
pub fn element_count_from_artifact_directory_name(
    _directory_name: &str,
) -> Result<usize, MlirAieIoctlError> {
    Err(MlirAieIoctlError::ElementCountNotInOpenedFile)
}

/// Read capital `-M`, `-K`, and `-N` only.
///
/// Lowercase `-k` is the kernel name (`MLIR_AIE`), not the K dimension.
pub fn parse_whole_array_mkn(command: &str) -> Result<(u32, u32, u32), MlirAieIoctlError> {
    let tokens: Vec<&str> = command.split_whitespace().collect();
    let mut m: Option<u32> = None;
    let mut k: Option<u32> = None;
    let mut n: Option<u32> = None;
    for (flag, value) in tokens.iter().zip(tokens.iter().skip(1)) {
        let Ok(parsed) = value.parse::<u32>() else {
            continue;
        };
        match *flag {
            "-M" | "--M" => m = Some(parsed),
            "-K" | "--K" => k = Some(parsed),
            "-N" | "--N" => n = Some(parsed),
            _ => {}
        }
    }
    match (m, k, n) {
        (Some(m), Some(k), Some(n)) => Ok((m, k, n)),
        _ => Err(MlirAieIoctlError::ShapeNotStated {
            text: command.to_owned(),
        }),
    }
}

/// f32 bits of `value` when the conversion is exact.
pub fn i32_as_f32_bits(value: i32) -> Result<u32, MlirAieIoctlError> {
    let exact = f64::from(value);
    // f64 holds every i32. Reject the value when the f32 narrowing is not exact.
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        reason = "roundtrip below rejects inexact f32"
    )]
    let narrow = exact as f32;
    #[allow(
        clippy::float_cmp,
        reason = "exact i32 roundtrip reject, not an approximate compare"
    )]
    if f64::from(narrow) != exact {
        return Err(MlirAieIoctlError::InexactI32ToF32 { value });
    }
    Ok(narrow.to_bits())
}

/// Answer one offline question. Unproven asks return an error and do not ioctl.
pub fn mlir_aie_ioctl_ask(ask: MlirAieIoctlAsk) -> Result<Vec<u8>, MlirAieIoctlError> {
    match ask {
        MlirAieIoctlAsk::ErtNpuPrefix { .. } => Err(MlirAieIoctlError::ErtNpuPrefixUncited),
        // The cited word is `PACKED_EXEC_CMD_REQUEST`. These bytes are that
        // integer in little-endian order. They are not the struct sizeof and
        // they are not an ioctl argument buffer. This arm does not open a
        // device. The uapi index 6 is not this word.
        MlirAieIoctlAsk::PackedExecCmdNumber => Ok(PACKED_EXEC_CMD_REQUEST.to_le_bytes().to_vec()),
        // Cited word is `PACKED_WAIT_CMD_REQUEST` (`DRM_IOW` `0x40106449`).
        // These bytes are that integer in little-endian order. They are not
        // an ioctl argument buffer and this arm does not open a device.
        // `0xc0106449` is not stored.
        MlirAieIoctlAsk::PackedWaitCmdNumber => Ok(PACKED_WAIT_CMD_REQUEST.to_le_bytes().to_vec()),
        MlirAieIoctlAsk::XclbinAiePartition => Err(MlirAieIoctlError::UnprovenAiePartition {
            plan: empty_cited_plan(),
        }),
        MlirAieIoctlAsk::WholeArrayHostOpcode => {
            // Integer 3 is `WHOLE_ARRAY_HOST_OPCODE`. Do not emit command bytes.
            Err(MlirAieIoctlError::HostOpcodeWordWidthUncited)
        }
        MlirAieIoctlAsk::AmdxdnaCmdStartNpuOffsets => {
            Err(MlirAieIoctlError::UnprovenAmdxdnaCmdStartNpuOffsets)
        }
        MlirAieIoctlAsk::ArgumentAddressWords => {
            Err(MlirAieIoctlError::UnprovenArgumentAddressWords)
        }
        MlirAieIoctlAsk::NumTilesFromThisXclbin => {
            Err(MlirAieIoctlError::UnprovenNumTilesFromXclbin)
        }
        MlirAieIoctlAsk::InstructionCreateBoType => {
            Err(MlirAieIoctlError::UnprovenInstructionCreateBoType)
        }
    }
}

/// Stop before any device open, ioctl, or buffer-object create.
///
/// When both paths are the recorded whole-array pair, the error carries a
/// send plan of the twenty-eight traced request words and argument widths.
/// CREATE_HWCTX stores `num_tiles` `0x20` at struct offset `0x24`. The cite
/// also puts `max_opc` `0x800` at offset `0x20`. That value is not a separate
/// field on the step. The packed EXEC_CMD request word is
/// [`PACKED_EXEC_CMD_REQUEST`]. It is not issued. The packed WAIT_CMD request
/// word is [`PACKED_WAIT_CMD_REQUEST`]. It is stored and is not in the plan.
/// `0xc03064ca` is a plan row and is not named. It is not `0x40106449` and
/// not `0xc0106449`. AIE_PARTITION is still uncited, so this returns before
/// any device open. The xclbin is hashed for pair identity and is not parsed.
pub fn submit_mlir_aie_whole_array_ioctl(
    xclbin: &Path,
    insts: &Path,
) -> Result<(), MlirAieIoctlError> {
    #[cfg(feature = "xdna2-dangerous-exec-cmd")]
    if super::direct::exec_cmd_is_enabled() {
        return send_cited_whole_array_once(xclbin, insts);
    }
    Err(MlirAieIoctlError::UnprovenAiePartition {
        plan: cited_plan_for_whole_array_pair(xclbin, insts),
    })
}

/// Accepted whole-array pair only. Any other bytes return the cited plan and do not open.
#[cfg(feature = "xdna2-dangerous-exec-cmd")]
fn send_cited_whole_array_once(xclbin: &Path, insts: &Path) -> Result<(), MlirAieIoctlError> {
    if !whole_array_pair_is_accepted(xclbin, insts) {
        return Err(MlirAieIoctlError::UnprovenAiePartition {
            plan: cited_plan_for_whole_array_pair(xclbin, insts),
        });
    }
    let submitted =
        super::direct::send_cited_whole_array_device_once(xclbin, insts).map_err(|err| {
            MlirAieIoctlError::CitedIoctlFailed {
                request: err.request,
                errno: err.errno,
            }
        })?;
    let host = host_whole_array_i16_buffers()?;
    let output_matches =
        whole_array_output_matches_host(&submitted.output_i32_le, &host.golden_i32);
    let host_golden = i32_cells_as_f32(&host.golden_i32)?;
    let device_f32 = i32_le_bytes_as_f32(&submitted.output_i32_le)?;
    if host_golden.is_empty() || device_f32.is_empty() {
        return Err(MlirAieIoctlError::UnprovenAiePartition {
            plan: cited_plan_for_whole_array_pair(xclbin, insts),
        });
    }
    // This arm ran only after the accepted instruction file and an Ok device
    // return. The fixture records that. `apply_whole_array_verify_flags` must
    // forward those three bits. It must not invent a pass by itself.
    let verify = apply_whole_array_verify_flags(super::numerical::NumericalVerifyInput {
        host_golden: &host_golden,
        device_or_candidate: &device_f32,
        had_instruction_blob: true,
        submit_wait_ok: true,
        real_ioctl_path: true,
        criteria: super::numerical::NumericalVerifyCriteria::default(),
    });
    let ran = super::numerical::decide_ran_on_npu(&verify);
    if !output_matches || !ran {
        return Err(MlirAieIoctlError::UnprovenAiePartition {
            plan: cited_plan_for_whole_array_pair(xclbin, insts),
        });
    }
    Ok(())
}

#[cfg(feature = "xdna2-dangerous-exec-cmd")]
fn i32_cells_as_f32(cells: &[i32]) -> Result<Vec<f32>, MlirAieIoctlError> {
    let mut out = Vec::with_capacity(cells.len());
    for cell in cells {
        out.push(f32::from_bits(i32_as_f32_bits(*cell)?));
    }
    if out.is_empty() {
        return Err(MlirAieIoctlError::HostBufferShape {
            detail: "host golden i32 cells are empty",
        });
    }
    Ok(out)
}

#[cfg(feature = "xdna2-dangerous-exec-cmd")]
fn i32_le_bytes_as_f32(bytes: &[u8]) -> Result<Vec<f32>, MlirAieIoctlError> {
    if bytes.is_empty() || bytes.len() % 4 != 0 {
        return Err(MlirAieIoctlError::HostBufferShape {
            detail: "device i32 bytes are empty or not a multiple of 4",
        });
    }
    let mut cells = Vec::with_capacity(bytes.len() / 4);
    for chunk in bytes.chunks_exact(4) {
        cells.push(i32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
    }
    i32_cells_as_f32(&cells)
}

#[cfg(feature = "xdna2-dangerous-exec-cmd")]
fn whole_array_pair_is_accepted(xclbin: &Path, insts: &Path) -> bool {
    const XCLBIN_SHA: &str = "0cadf32ec0472e3f55e9102a6175d005b1a95a8ea428907086e15ad44d825dad";
    const INSTS_SHA: &str = "ae692499a79c74b46175cb4841f235edb0059175144941522f792105b758dec3";
    file_sha_len_matches(xclbin, XCLBIN_SHA, 73_327)
        && file_sha_len_matches(insts, INSTS_SHA, 2_592)
}

#[cfg(feature = "xdna2-dangerous-exec-cmd")]
fn file_sha_len_matches(path: &Path, expect_hex: &str, expect_len: usize) -> bool {
    let Ok(bytes) = std::fs::read(path) else {
        return false;
    };
    if bytes.len() != expect_len {
        return false;
    }
    let digest = <sha2::Sha256 as sha2::Digest>::digest(&bytes);
    let mut hex_out = String::with_capacity(64);
    for byte in digest {
        hex_out.push_str(&format!("{byte:02x}"));
    }
    hex_out == expect_hex
}

fn whole_array_output_matches_host(output_i32_le: &[u8], golden_i32: &[i32]) -> bool {
    let Some(need) = golden_i32.len().checked_mul(4) else {
        return false;
    };
    if golden_i32.is_empty() || output_i32_le.len() != need {
        return false;
    }
    output_i32_le
        .chunks_exact(4)
        .zip(golden_i32.iter())
        .all(|(chunk, cell)| {
            let word = i32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            word == *cell
        })
}

fn apply_whole_array_verify_flags<'a>(
    input: super::numerical::NumericalVerifyInput<'a>,
) -> super::numerical::NumericalVerifyInput<'a> {
    super::numerical::NumericalVerifyInput {
        host_golden: input.host_golden,
        device_or_candidate: input.device_or_candidate,
        had_instruction_blob: input.had_instruction_blob,
        submit_wait_ok: input.submit_wait_ok,
        real_ioctl_path: input.real_ioctl_path,
        criteria: input.criteria,
    }
}

fn empty_cited_plan() -> CitedSubmitPlan {
    CitedSubmitPlan {
        steps: Vec::new(),
        xclbin_sha256: String::new(),
        xclbin_size: 0,
        insts_sha256: String::new(),
        insts_size: 0,
        device_opened: false,
        xrt_executed: false,
    }
}

/// Cited device-heap CREATE_BO size and the recorded mmap length. 64 MiB.
/// The whole-array heap row is `0x4000000`, type 2. Not a second size.
const DEVICE_HEAP_MMAP_BYTES: u64 = 0x0400_0000;

fn device_heap_uva_align_bytes() -> u64 {
    DEVICE_HEAP_MMAP_BYTES
}

fn recorded_device_heap_mmap(record: DeviceHeapMmapRecord) -> bool {
    // `tests` is nested, so `super` there is this module, not `amd_xdna2`.
    // `super::direct` does not name the sibling. The crate path does.
    // `0x0400_0000` is the same 64 MiB as `AIE2_DEV_HEAP_SIZE` (`0x4000000`).
    record.size == 0x0400_0000
        && record.size == DEVICE_HEAP_MMAP_BYTES
        && record.size == crate::backends::amd_xdna2::direct::AIE2_DEV_HEAP_SIZE
        && record.bo_type == AMDXDNA_BO_DEV_HEAP
        && record.align == 0x0400_0000
        && record.align == device_heap_uva_align_bytes()
        && record.align == crate::backends::amd_xdna2::direct::AIE2_DEV_HEAP_SIZE
        && record.align == record.size
}

fn device_heap_mmap_step() -> CitedIoctlStep {
    // Same 64 MiB (`0x4000000`) as the heap CREATE_BO row. Not a second
    // size. request 0 means this row is not an ioctl. Type 2.
    CitedIoctlStep {
        request: 0,
        size: 0,
        num_tiles_offset: None,
        num_tiles: None,
        device_heap_mmap: Some(DeviceHeapMmapRecord {
            size: DEVICE_HEAP_MMAP_BYTES,
            bo_type: AMDXDNA_BO_DEV_HEAP,
            align: device_heap_uva_align_bytes(),
        }),
    }
}

fn cited_whole_array_steps() -> Vec<CitedIoctlStep> {
    // Argument `size` is the ioctl width, not a buffer-object byte length.
    // CREATE_HWCTX row: num_tiles 0x20 at offset 0x24. max_opc 0x800 at
    // offset 0x20 is the cite and is not stored on the step. No start_col.
    // No num_col. Pointer words are not in this table.
    // Buffer sizes and types, in CREATE_BO order: 0x4000000 type 2,
    // 0x10530 type 3, 0xa20 type 3, 0x8000 type 1, 0x8000 type 1,
    // 0x10000 type 1, 0x1000 type 4.
    // The device-heap mmap is inserted after the first GET_BO_INFO (the
    // heap one) and before CREATE_HWCTX. It is not one of these 28 requests.
    // GEM_CLOSE handles, in order: 6, 5, 4, 3, 7, 2, then 1 after
    // SYNCOBJ_DESTROY handle 1 and DESTROY_HWCTX context 1.
    // PACKED_WAIT_CMD_REQUEST is not a row. 0xc03064ca is not named.
    const ROWS: [(u64, u32, Option<u32>, Option<u32>); 28] = [
        (0xc020_6443, 32, None, None),
        (0xc030_6444, 48, None, None),
        (0xc010_6447, 16, None, None),
        (0xc038_6440, 56, Some(0x24), Some(0x20)),
        (0xc020_6443, 32, None, None),
        (0xc030_6444, 48, None, None),
        (0xc018_6442, 24, None, None),
        (0xc020_6443, 32, None, None),
        (0xc030_6444, 48, None, None),
        (0xc020_6443, 32, None, None),
        (0xc030_6444, 48, None, None),
        (0xc020_6443, 32, None, None),
        (0xc030_6444, 48, None, None),
        (0xc020_6443, 32, None, None),
        (0xc030_6444, 48, None, None),
        (0xc020_6443, 32, None, None),
        (0xc030_6444, 48, None, None),
        (PACKED_EXEC_CMD_REQUEST, 56, None, None),
        (0xc030_64ca, 48, None, None),
        (0x4008_6409, 8, None, None),
        (0x4008_6409, 8, None, None),
        (0x4008_6409, 8, None, None),
        (0x4008_6409, 8, None, None),
        (0x4008_6409, 8, None, None),
        (0x4008_6409, 8, None, None),
        (0xc008_64c0, 8, None, None),
        (0xc008_6441, 8, None, None),
        (0x4008_6409, 8, None, None),
    ];
    let mut steps: Vec<CitedIoctlStep> = ROWS
        .into_iter()
        .map(
            |(request, size, num_tiles_offset, num_tiles)| CitedIoctlStep {
                request,
                size,
                num_tiles_offset,
                num_tiles,
                device_heap_mmap: None,
            },
        )
        .collect();
    // Heap CREATE_BO is the first 0xc0206443. Its GET_BO_INFO is the first
    // 0xc0306444. The mmap has to follow that pair and precede CREATE_HWCTX.
    let heap_get_bo = steps.iter().position(|step| step.request == 0xc030_6444);
    let create_hwctx = steps.iter().position(|step| step.request == 0xc038_6440);
    if let (Some(get_bo), Some(hwctx)) = (heap_get_bo, create_hwctx) {
        if get_bo < hwctx {
            steps.insert(get_bo.saturating_add(1), device_heap_mmap_step());
        }
    }
    steps
}

/// `XDNA2_STOP_AFTER_CREATE_HWCTX=1` asks the replay to stop at CREATE_HWCTX.
pub(crate) fn stop_after_create_hwctx_asked() -> bool {
    std::env::var("XDNA2_STOP_AFTER_CREATE_HWCTX")
        .ok()
        .as_deref()
        == Some("1")
}

/// Steps `replay_printed_cited_requests` walks.
///
/// Unset `XDNA2_STOP_AFTER_CREATE_HWCTX` keeps the full list, including
/// EXEC_CMD `0xc0386446`. `=1` selects the prefix that ends at the first
/// CREATE_HWCTX `0xc0386440`.
pub(crate) fn issued_replay_printed_cited_requests() -> Vec<CitedIoctlStep> {
    replay_request_list(stop_after_create_hwctx_asked())
}

/// Pure replay list. `true` ends at the first CREATE_HWCTX `0xc0386440`
/// and excludes EXEC_CMD `0xc0386446` and timeline wait `0xc03064ca`.
/// `false` is the full shipped list. Does not open `/dev/accel`.
pub(crate) fn replay_request_list(stop_after_create_hwctx: bool) -> Vec<CitedIoctlStep> {
    let mut steps = issued_replay_full_printed_cited_requests();
    if !stop_after_create_hwctx {
        return steps;
    }
    let Some(at) = steps.iter().position(|step| step.request == 0xc038_6440) else {
        return steps;
    };
    steps.truncate(at.saturating_add(1));
    steps
}

/// Full shipped replay list. Not the helper list.
///
/// The 28 cited requests stay in order. The device-heap mmap is inserted
/// after the first GET_BO_INFO and before CREATE_HWCTX. It is not an ioctl.
/// Removing that insert makes
/// `shipped_whole_array_steps_lack_device_heap_mmap_before_create_hwctx` fail.
fn issued_replay_full_printed_cited_requests() -> Vec<CitedIoctlStep> {
    // Same 28 request words as the live replay. Width comes from the request.
    // CREATE_HWCTX keeps num_tiles 0x20 at offset 0x24. max_opc is not stored.
    const REQUESTS: [u64; 28] = [
        0xc020_6443,
        0xc030_6444,
        0xc010_6447,
        0xc038_6440,
        0xc020_6443,
        0xc030_6444,
        0xc018_6442,
        0xc020_6443,
        0xc030_6444,
        0xc020_6443,
        0xc030_6444,
        0xc020_6443,
        0xc030_6444,
        0xc020_6443,
        0xc030_6444,
        0xc020_6443,
        0xc030_6444,
        0xc038_6446,
        0x4010_6449,
        0x4008_6409,
        0x4008_6409,
        0x4008_6409,
        0x4008_6409,
        0x4008_6409,
        0x4008_6409,
        0xc008_64c0,
        0xc008_6441,
        0x4008_6409,
    ];
    let mut steps: Vec<CitedIoctlStep> = REQUESTS
        .into_iter()
        .map(|request| {
            let size = u32::try_from((request >> 16) & 0x3fff).unwrap_or(0);
            let (num_tiles_offset, num_tiles) = if request == 0xc038_6440 {
                (Some(0x24_u32), Some(0x20_u32))
            } else {
                (None, None)
            };
            CitedIoctlStep {
                request,
                size,
                num_tiles_offset,
                num_tiles,
                device_heap_mmap: None,
            }
        })
        .collect();
    let heap_get_bo = steps.iter().position(|step| step.request == 0xc030_6444);
    let create_hwctx = steps.iter().position(|step| step.request == 0xc038_6440);
    if let (Some(get_bo), Some(hwctx)) = (heap_get_bo, create_hwctx) {
        if get_bo < hwctx {
            steps.insert(get_bo.saturating_add(1), device_heap_mmap_step());
        }
    }
    steps
}

fn cited_plan_for_whole_array_pair(xclbin: &Path, insts: &Path) -> CitedSubmitPlan {
    let Ok(xclbin_bytes) = std::fs::read(xclbin) else {
        return empty_cited_plan();
    };
    let Ok(insts_bytes) = std::fs::read(insts) else {
        return empty_cited_plan();
    };
    let xclbin_sha256 = super::whole_array_i16::sha256_hex(&xclbin_bytes);
    let insts_sha256 = super::whole_array_i16::sha256_hex(&insts_bytes);
    if xclbin_sha256 == super::whole_array_i16::REFUSED_XCLBIN_SHA256 {
        return empty_cited_plan();
    }
    let Ok(xclbin_size) = u64::try_from(xclbin_bytes.len()) else {
        return empty_cited_plan();
    };
    let Ok(insts_size) = u64::try_from(insts_bytes.len()) else {
        return empty_cited_plan();
    };
    let pair_ok = xclbin_sha256 == super::whole_array_i16::ACCEPTED_XCLBIN_SHA256
        && xclbin_size == super::whole_array_i16::ACCEPTED_XCLBIN_BYTES
        && insts_sha256 == super::whole_array_i16::INSTS_SHA256
        && insts_bytes.len() == super::whole_array_i16::INSTS_LEN;
    if !pair_ok {
        return empty_cited_plan();
    }
    CitedSubmitPlan {
        steps: cited_whole_array_steps(),
        xclbin_sha256,
        xclbin_size,
        insts_sha256,
        insts_size,
        device_opened: false,
        xrt_executed: false,
    }
}

/// Open the unexecuted ioctl command, then the recorded argv if that file has no shape.
///
/// Element count is never taken from the directory name `whole_array_128_i16`.
pub fn host_whole_array_i16_buffers() -> Result<WholeArrayHostBuffers, MlirAieIoctlError> {
    let ioctl_command = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(IOCTL_COMMAND_REL);
    if let Some(dims) = stated_mkn_in_file(&ioctl_command)? {
        return buffers_for_dims(dims);
    }
    if let Some(dims) = stated_mkn_in_file(Path::new(RECORDED_MLIR_AIE_ARGV))? {
        return buffers_for_dims(dims);
    }
    Err(MlirAieIoctlError::ShapeNotStated {
        text: ioctl_command.display().to_string(),
    })
}

fn stated_mkn_in_file(path: &Path) -> Result<Option<(u32, u32, u32)>, MlirAieIoctlError> {
    let text =
        std::fs::read_to_string(path).map_err(|err| MlirAieIoctlError::CommandUnreadable {
            path: path.display().to_string(),
            detail: err.to_string(),
        })?;
    for line in text.lines() {
        if let Ok(dims) = parse_whole_array_mkn(line) {
            return Ok(Some(dims));
        }
    }
    Ok(None)
}

fn buffers_for_dims(dims: (u32, u32, u32)) -> Result<WholeArrayHostBuffers, MlirAieIoctlError> {
    let (m, k, n) = dims;
    let m_len = dim_usize(m)?;
    let k_len = dim_usize(k)?;
    let n_len = dim_usize(n)?;
    let a_len = product_len(m_len, k_len)?;
    let b_len = product_len(k_len, n_len)?;
    let a_i16 = vec![1_i16; a_len];
    let b_i16 = vec![1_i16; b_len];
    let golden_i32 = row_major_i16_matmul(m_len, k_len, n_len, &a_i16, &b_i16)?;
    let mut host_golden = Vec::with_capacity(golden_i32.len());
    for cell in &golden_i32 {
        host_golden.push(f32::from_bits(i32_as_f32_bits(*cell)?));
    }
    let device_or_candidate = vec![0.0_f32; golden_i32.len()];
    Ok(WholeArrayHostBuffers {
        m,
        k,
        n,
        a_i16,
        b_i16,
        golden_i32,
        host_golden,
        device_or_candidate,
        device_bytes_are_readback: false,
    })
}

fn dim_usize(dim: u32) -> Result<usize, MlirAieIoctlError> {
    usize::try_from(dim).map_err(|_err| MlirAieIoctlError::HostBufferShape {
        detail: "dimension does not fit usize",
    })
}

fn product_len(left: usize, right: usize) -> Result<usize, MlirAieIoctlError> {
    left.checked_mul(right)
        .ok_or(MlirAieIoctlError::HostBufferShape {
            detail: "matrix length overflow",
        })
}

fn row_major_i16_matmul(
    rows: usize,
    inner: usize,
    cols: usize,
    left_matrix: &[i16],
    right_matrix: &[i16],
) -> Result<Vec<i32>, MlirAieIoctlError> {
    let mut golden = vec![0_i32; product_len(rows, cols)?];
    for row in 0..rows {
        for col in 0..cols {
            let mut acc = 0_i32;
            for along in 0..inner {
                let a_ix = index_of(row, inner, along)?;
                let b_ix = index_of(along, cols, col)?;
                let left = i32::from(*left_matrix.get(a_ix).ok_or(
                    MlirAieIoctlError::HostBufferShape {
                        detail: "activation index",
                    },
                )?);
                let right = i32::from(*right_matrix.get(b_ix).ok_or(
                    MlirAieIoctlError::HostBufferShape {
                        detail: "weight index",
                    },
                )?);
                let prod = left
                    .checked_mul(right)
                    .ok_or(MlirAieIoctlError::HostBufferShape {
                        detail: "product overflow",
                    })?;
                acc = acc
                    .checked_add(prod)
                    .ok_or(MlirAieIoctlError::HostBufferShape {
                        detail: "accumulator overflow",
                    })?;
            }
            let out = index_of(row, cols, col)?;
            *golden
                .get_mut(out)
                .ok_or(MlirAieIoctlError::HostBufferShape {
                    detail: "golden index",
                })? = acc;
        }
    }
    Ok(golden)
}

struct CreateHwctxTryRecord {
    index: u32,
    errno: i32,
}

const CREATE_HWCTX_RETRY_WAIT: std::time::Duration = std::time::Duration::from_millis(200);

fn retry_create_hwctx<Issue, Sleep>(
    argument: &[u8],
    mut issue: Issue,
    mut on_wait: Sleep,
) -> Vec<CreateHwctxTryRecord>
where
    Issue: FnMut(u32, &[u8]) -> Result<(), i32>,
    Sleep: FnMut(std::time::Duration),
{
    let mut records = Vec::new();
    let mut tries_already_issued = 0u32;
    loop {
        let index = tries_already_issued.saturating_add(1);
        let errno = match issue(index, argument) {
            Ok(()) => 0,
            Err(errno) => errno,
        };
        tries_already_issued = tries_already_issued.saturating_add(1);
        records.push(CreateHwctxTryRecord { index, errno });
        if errno == 0 {
            break;
        }
        match super::direct::create_hwctx_eagain_backoff(0xc038_6440, errno, tries_already_issued) {
            Some(wait) => on_wait(wait),
            None => break,
        }
    }
    records
}

fn index_of(major: usize, stride: usize, minor: usize) -> Result<usize, MlirAieIoctlError> {
    product_len(major, stride)?
        .checked_add(minor)
        .ok_or(MlirAieIoctlError::HostBufferShape {
            detail: "index overflow",
        })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::super::numerical::{
        NumericalVerifyCriteria, NumericalVerifyInput, decide_ran_on_npu,
    };
    use super::{
        AMDXDNA_BO_DEV_HEAP, AMDXDNA_BO_SHARE, AMDXDNA_BO_SHMEM,
        AMDXDNA_CMD_START_NPU_MEMBER_ORDER, MlirAieIoctlAsk, MlirAieIoctlError,
        PACKED_EXEC_CMD_REQUEST, PACKED_WAIT_CMD_REQUEST, WHOLE_ARRAY_HOST_ARG_ORDER,
        WHOLE_ARRAY_HOST_OPCODE, apply_whole_array_verify_flags, config_cu_from_caller,
        element_count_from_artifact_directory_name, host_whole_array_i16_buffers, i32_as_f32_bits,
        mlir_aie_ioctl_ask, parse_whole_array_mkn, submit_mlir_aie_whole_array_ioctl,
        whole_array_output_matches_host,
    };
    use std::path::Path;

    #[test]
    fn ert_npu_prefix_is_refused_and_unproven_asks_fail() {
        // Do not build prefix bytes. The host opcode is integer 3, not
        // ERT_START_NPU. Numeric layout stays a typed error.
        let refused = mlir_aie_ioctl_ask(MlirAieIoctlAsk::ErtNpuPrefix {
            instruction_buffer: 0,
            instruction_buffer_size: 0,
            instruction_prop_count: 0,
            cu_mask: 0,
        });
        assert!(matches!(
            refused,
            Err(MlirAieIoctlError::ErtNpuPrefixUncited)
        ));

        // Cited word. Not a device open and not an issued ioctl.
        assert_eq!(PACKED_EXEC_CMD_REQUEST, 0xc038_6446);
        let exec_word = mlir_aie_ioctl_ask(MlirAieIoctlAsk::PackedExecCmdNumber).expect("exec");
        assert_eq!(
            exec_word.as_slice(),
            PACKED_EXEC_CMD_REQUEST.to_le_bytes().as_slice()
        );
        assert_ne!(exec_word.len(), 56);
        assert_eq!(PACKED_WAIT_CMD_REQUEST, 0x4010_6449);
        assert_ne!(PACKED_WAIT_CMD_REQUEST, 0xc010_6449);
        let wait_word = mlir_aie_ioctl_ask(MlirAieIoctlAsk::PackedWaitCmdNumber).expect("wait");
        assert_eq!(
            wait_word.as_slice(),
            PACKED_WAIT_CMD_REQUEST.to_le_bytes().as_slice()
        );
        assert!(matches!(
            mlir_aie_ioctl_ask(MlirAieIoctlAsk::XclbinAiePartition).expect_err("part"),
            MlirAieIoctlError::UnprovenAiePartition { .. }
        ));
        assert!(matches!(
            mlir_aie_ioctl_ask(MlirAieIoctlAsk::WholeArrayHostOpcode).expect_err("opcode"),
            MlirAieIoctlError::HostOpcodeWordWidthUncited
        ));
        assert_eq!(WHOLE_ARRAY_HOST_OPCODE, 3);
        assert!(matches!(
            mlir_aie_ioctl_ask(MlirAieIoctlAsk::AmdxdnaCmdStartNpuOffsets).expect_err("offsets"),
            MlirAieIoctlError::UnprovenAmdxdnaCmdStartNpuOffsets
        ));
        assert!(matches!(
            mlir_aie_ioctl_ask(MlirAieIoctlAsk::ArgumentAddressWords).expect_err("args"),
            MlirAieIoctlError::UnprovenArgumentAddressWords
        ));
        assert!(matches!(
            mlir_aie_ioctl_ask(MlirAieIoctlAsk::NumTilesFromThisXclbin).expect_err("tiles"),
            MlirAieIoctlError::UnprovenNumTilesFromXclbin
        ));
        assert!(matches!(
            mlir_aie_ioctl_ask(MlirAieIoctlAsk::InstructionCreateBoType).expect_err("bo"),
            MlirAieIoctlError::UnprovenInstructionCreateBoType
        ));
    }

    #[test]
    fn submit_stops_at_first_unproven_packed_ioctl() {
        let err = submit_mlir_aie_whole_array_ioctl(
            Path::new("/home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/whole_array_128_i16/final.xclbin"),
            Path::new("/home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/whole_array_128_i16/insts.bin"),
        )
        .expect_err("no ioctl");
        // EXEC and WAIT words are stored and were not issued. AIE_PARTITION
        // is still uncited, so submit returns before any device open.
        // This test does not set XDNA2_ALLOW_EXEC_CMD and does not open /dev/accel.
        assert_eq!(PACKED_EXEC_CMD_REQUEST, 0xc038_6446);
        assert_eq!(PACKED_WAIT_CMD_REQUEST, 0x4010_6449);
        assert!(matches!(
            err,
            MlirAieIoctlError::UnprovenAiePartition { .. }
        ));
    }

    #[test]
    fn submit_refuses_uncited_packed_request_and_uncited_aie_partition() {
        let err = submit_mlir_aie_whole_array_ioctl(
            Path::new("/home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/whole_array_128_i16/final.xclbin"),
            Path::new("/home/hunter/Projects/ai/dragon-npu/testdata/xdna2_cmd/whole_array_128_i16/insts.bin"),
        )
        .expect_err("no ioctl");
        let text = err.to_string();
        assert!(
            text.contains("0xc03064ca is uncited"),
            "missing uncited packed request: {text}"
        );
        assert!(
            text.contains("AIE_PARTITION is uncited"),
            "missing uncited AIE partition: {text}"
        );
        assert!(
            !text.contains("0x40106449") && !text.contains("0xc0106449"),
            "substituted a wait request word: {text}"
        );
        assert!(
            !text.contains("DRM_IOCTL"),
            "invented an ioctl name: {text}"
        );
        assert!(matches!(
            err,
            MlirAieIoctlError::UnprovenAiePartition { .. }
        ));
        // After the refusal returns. The asserts above are the pass condition.
        println!("device_opened=no");
    }

    #[test]
    pub(crate) fn submit_plans_only_cited_requests_for_whole_array_pair_then_refuses_before_open() {
        // Flush each line. A redirected --nocapture log is block-buffered,
        // and appending to /proc/self/fd/1 overwrites those lines.
        let emit_trace = |line: &str| {
            println!("{line}");
            let _ = std::io::Write::flush(&mut std::io::stdout());
        };
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let xclbin = root.join("testdata/xdna2_cmd/whole_array_128_i16/final.xclbin");
        let insts = root.join("testdata/xdna2_cmd/whole_array_128_i16/insts.bin");
        let xclbin_bytes = std::fs::read(&xclbin).expect("xclbin");
        let insts_bytes = std::fs::read(&insts).expect("insts");
        let xclbin_sha = super::super::whole_array_i16::sha256_hex(&xclbin_bytes);
        let insts_sha = super::super::whole_array_i16::sha256_hex(&insts_bytes);
        assert_eq!(
            xclbin_sha,
            "0cadf32ec0472e3f55e9102a6175d005b1a95a8ea428907086e15ad44d825dad"
        );
        assert_eq!(xclbin_bytes.len(), 73327);
        assert_eq!(
            insts_sha,
            "ae692499a79c74b46175cb4841f235edb0059175144941522f792105b758dec3"
        );
        assert_eq!(insts_bytes.len(), 2592);
        assert!(
            !xclbin_sha.starts_with("286fe232"),
            "refused xclbin was used"
        );

        let err = submit_mlir_aie_whole_array_ioctl(&xclbin, &insts).expect_err("no ioctl");
        let text = err.to_string();
        assert!(
            text.contains("0xc03064ca is uncited"),
            "missing uncited packed request: {text}"
        );
        assert!(
            text.contains("AIE_PARTITION is uncited"),
            "missing uncited AIE partition: {text}"
        );
        assert!(
            !text.contains("0x40106449") && !text.contains("0xc0106449"),
            "substituted a wait request word: {text}"
        );
        assert!(
            !text.contains("DRM_IOCTL"),
            "invented an ioctl name: {text}"
        );
        let MlirAieIoctlError::UnprovenAiePartition { plan } = err else {
            panic!("expected UnprovenAiePartition");
        };
        assert!(!plan.device_opened, "device was opened");
        assert!(!plan.xrt_executed, "XRT was executed");
        emit_trace("device_opened=no");
        assert_eq!(plan.xclbin_sha256, xclbin_sha);
        assert_eq!(plan.xclbin_size, 73327);
        assert_eq!(plan.insts_sha256, insts_sha);
        assert_eq!(plan.insts_size, 2592);
        assert_ne!(
            plan.xclbin_sha256,
            "286fe2324ef02cad008c99718cf648a6f0286aa5b6170dba62856ca1b90665c1"
        );

        // Traced before-phase order. Argument `size` is the ioctl width.
        // CREATE_BO buffer sizes and types are the cited integers, not a new
        // enumerator. Pointer words are not in this table. No start_col.
        // No num_col. PACKED_WAIT_CMD_REQUEST is not a row.
        let expected: [(u64, u32, Option<u32>, Option<u32>); 28] = [
            // REQ 1 CREATE_BO size 0x4000000 type 2
            (0xc020_6443, 32, None, None),
            // REQ 2 GET_BO_INFO
            (0xc030_6444, 48, None, None),
            // REQ 3 GET_INFO param 1
            (0xc010_6447, 16, None, None),
            // REQ 4 CREATE_HWCTX max_opc 0x800 at offset 0x20, num_tiles 0x20 at offset 0x24
            (0xc038_6440, 56, Some(0x24), Some(0x20)),
            // REQ 5 CREATE_BO size 0x10530 type 3
            (0xc020_6443, 32, None, None),
            // REQ 6 GET_BO_INFO
            (0xc030_6444, 48, None, None),
            // REQ 7 CONFIG_HWCTX leading u32 1, pointer not replayed, then u32 0x10
            (0xc018_6442, 24, None, None),
            // REQ 8 CREATE_BO size 0xa20 type 3
            (0xc020_6443, 32, None, None),
            // REQ 9 GET_BO_INFO
            (0xc030_6444, 48, None, None),
            // REQ 10 CREATE_BO size 0x8000 type 1
            (0xc020_6443, 32, None, None),
            // REQ 11 GET_BO_INFO
            (0xc030_6444, 48, None, None),
            // REQ 12 CREATE_BO size 0x8000 type 1
            (0xc020_6443, 32, None, None),
            // REQ 13 GET_BO_INFO
            (0xc030_6444, 48, None, None),
            // REQ 14 CREATE_BO size 0x10000 type 1
            (0xc020_6443, 32, None, None),
            // REQ 15 GET_BO_INFO
            (0xc030_6444, 48, None, None),
            // REQ 16 CREATE_BO size 0x1000 type 4
            (0xc020_6443, 32, None, None),
            // REQ 17 GET_BO_INFO
            (0xc030_6444, 48, None, None),
            // REQ 18 EXEC_CMD
            (0xc038_6446, 56, None, None),
            // REQ 19 syncobj timeline wait, argument width 48. Not 0x40106449. Not 0xc0106449.
            (0xc030_64ca, 48, None, None),
            // REQ 20 GEM_CLOSE handle 6
            (0x4008_6409, 8, None, None),
            // REQ 21 GEM_CLOSE handle 5
            (0x4008_6409, 8, None, None),
            // REQ 22 GEM_CLOSE handle 4
            (0x4008_6409, 8, None, None),
            // REQ 23 GEM_CLOSE handle 3
            (0x4008_6409, 8, None, None),
            // REQ 24 GEM_CLOSE handle 7
            (0x4008_6409, 8, None, None),
            // REQ 25 GEM_CLOSE handle 2
            (0x4008_6409, 8, None, None),
            // REQ 26 SYNCOBJ_DESTROY handle 1
            (0xc008_64c0, 8, None, None),
            // REQ 27 DESTROY_HWCTX context 1
            (0xc008_6441, 8, None, None),
            // REQ 28 GEM_CLOSE handle 1
            (0x4008_6409, 8, None, None),
        ];
        for (index, (request, _size, _num_tiles_offset, _num_tiles)) in expected.iter().enumerate()
        {
            emit_trace(&format!(
                "expected_request index={} request={request:#x}",
                index + 1
            ));
        }
        let hwctx_at = plan
            .steps
            .iter()
            .position(|step| step.request == 0xc038_6440)
            .expect("CREATE_HWCTX 0xc0386440");
        let heap_mmap_before = plan.steps[..hwctx_at]
            .iter()
            .filter(|step| step.device_heap_mmap.is_some())
            .count();
        assert_eq!(
            heap_mmap_before, 1,
            "device-heap mmap before CREATE_HWCTX (0xc0386440)"
        );
        let actual: Vec<(u64, u32, Option<u32>, Option<u32>)> = plan
            .steps
            .iter()
            .filter(|step| step.device_heap_mmap.is_none())
            .map(|step| {
                (
                    step.request,
                    step.size,
                    step.num_tiles_offset,
                    step.num_tiles,
                )
            })
            .collect();
        assert_eq!(
            actual.as_slice(),
            expected.as_slice(),
            "planned live requests are still the seven-step list"
        );
        assert!(
            plan.steps
                .iter()
                .all(|step| step.request != 0x4010_6449 && step.request != 0xc010_6449),
            "plan substituted a wait-cmd request"
        );
        assert_eq!(PACKED_EXEC_CMD_REQUEST, 0xc038_6446);
        assert_eq!(PACKED_WAIT_CMD_REQUEST, 0x4010_6449);
        assert!(
            plan.steps
                .iter()
                .all(|step| step.request != PACKED_WAIT_CMD_REQUEST)
        );
    }

    #[test]
    fn shipped_whole_array_steps_lack_device_heap_mmap_before_create_hwctx() {
        fn step_is_device_heap_mmap(step: &super::CitedIoctlStep) -> bool {
            // An ioctl row is not the mmap. The recorded step has request 0,
            // ioctl width 0, no CREATE_HWCTX tile words, and the 64 MiB heap
            // record (type 2). A list that only has the 28 requests fails.
            let super::CitedIoctlStep {
                request,
                size,
                num_tiles_offset,
                num_tiles,
                device_heap_mmap,
            } = *step;
            if request != 0 || size != 0 || num_tiles_offset.is_some() || num_tiles.is_some() {
                return false;
            }
            let Some(record) = device_heap_mmap else {
                return false;
            };
            super::recorded_device_heap_mmap(record)
        }

        let steps = super::issued_replay_printed_cited_requests();
        let create_bo_at = steps
            .iter()
            .position(|step| step.request == 0xc020_6443)
            .expect("shipped steps include CREATE_BO 0xc0206443");
        let get_bo_at = steps
            .iter()
            .position(|step| step.request == 0xc030_6444)
            .expect("shipped steps include GET_BO_INFO 0xc0306444");
        let create_hwctx_at = steps
            .iter()
            .position(|step| step.request == 0xc038_6440)
            .expect("shipped steps include CREATE_HWCTX 0xc0386440");
        let heap_mmap_at = steps.iter().position(step_is_device_heap_mmap);
        let heap_mmap_before = heap_mmap_at.is_some_and(|mmap_at| {
            create_bo_at < get_bo_at && get_bo_at < mmap_at && mmap_at < create_hwctx_at
        });
        assert!(
            heap_mmap_before,
            "no device-heap mmap before CREATE_HWCTX (0xc0386440)"
        );
    }

    #[test]
    #[allow(unsafe_code)]
    fn replay_request_list_stops_after_create_hwctx_when_asked() {
        // Pure list. This test does not open /dev/accel and does not issue ioctls.
        const STOP_ENV: &str = "XDNA2_STOP_AFTER_CREATE_HWCTX";
        struct StopEnvRestore(Option<std::ffi::OsString>);
        impl Drop for StopEnvRestore {
            fn drop(&mut self) {
                // SAFETY: test-only env restore. No device open.
                unsafe {
                    match self.0.take() {
                        Some(value) => std::env::set_var(STOP_ENV, value),
                        None => std::env::remove_var(STOP_ENV),
                    }
                }
            }
        }
        let restore = StopEnvRestore(std::env::var_os(STOP_ENV));
        // SAFETY: test-only env. No device open.
        unsafe {
            std::env::remove_var(STOP_ENV);
        }
        let full = super::issued_replay_printed_cited_requests();
        let full_requests: Vec<u64> = full.iter().map(|step| step.request).collect();
        assert!(
            full_requests.contains(&0xc038_6446),
            "unset env must keep EXEC_CMD 0xc0386446"
        );
        let before_hwctx = full
            .iter()
            .position(|step| step.request == 0xc038_6440)
            .expect("full list includes CREATE_HWCTX 0xc0386440");
        let requests_before: Vec<u64> = full_requests[..before_hwctx].to_vec();
        assert!(
            !requests_before.is_empty(),
            "requests before CREATE_HWCTX must stay"
        );
        // SAFETY: test-only env. No device open.
        unsafe {
            std::env::set_var(STOP_ENV, "1");
        }
        let stopped = super::issued_replay_printed_cited_requests();
        let stopped_requests: Vec<u64> = stopped.iter().map(|step| step.request).collect();
        assert!(
            stopped_requests.contains(&0xc038_6440),
            "stop was asked but CREATE_HWCTX 0xc0386440 is missing"
        );
        assert!(
            !stopped_requests.contains(&0xc038_6446),
            "stop was asked but EXEC_CMD 0xc0386446 is still in the shipped list"
        );
        assert!(
            !stopped_requests.contains(&0xc030_64ca),
            "stop was asked but timeline wait 0xc03064ca is still in the shipped list"
        );
        let stopped_hwctx = stopped
            .iter()
            .position(|step| step.request == 0xc038_6440)
            .expect("stopped list includes CREATE_HWCTX 0xc0386440");
        assert_eq!(
            &stopped_requests[..stopped_hwctx],
            requests_before.as_slice(),
            "requests before CREATE_HWCTX must stay"
        );
        assert_eq!(
            stopped_requests.len(),
            stopped_hwctx.saturating_add(1),
            "stop was asked but the list does not end at CREATE_HWCTX 0xc0386440"
        );
        drop(restore);
    }

    #[test]
    fn exec_wait_issue_2_successor_of_c0386446_is_40106449_and_not_forbidden() {
        // Shipped replay list and the shipped forbid check. No device open.
        let steps = super::issued_replay_full_printed_cited_requests();
        let exec_at = steps
            .iter()
            .position(|step| step.request == 0xc038_6446)
            .expect("shipped list includes EXEC_CMD 0xc0386446");
        let successor = steps
            .get(exec_at.saturating_add(1))
            .map(|step| step.request);
        assert_eq!(
            successor,
            Some(0x4010_6449),
            "successor of 0xc0386446 must be 0x40106449"
        );
        assert!(
            !super::super::direct::cited_request_is_forbidden(0x4010_6449),
            "0x40106449 is forbidden"
        );
        assert!(
            super::super::direct::cited_request_is_forbidden(0xc010_6449),
            "0xc0106449 must stay forbidden"
        );
        assert!(
            super::super::direct::cited_request_is_forbidden(0xc018_6445),
            "0xc0186445 must stay forbidden"
        );
        assert_eq!(
            steps
                .iter()
                .filter(|step| step.request == 0x4010_6449)
                .count(),
            1,
            "0x40106449 must replace one slot, not be inserted beside it"
        );
    }

    #[test]
    fn cited_bo_types_have_no_instruction_enumerator() {
        // amdxdna_accel.h lines 163-165. Types 3, 4, and 9 are not stored.
        // The instruction CREATE_BO choice stays the typed error.
        assert_eq!(AMDXDNA_BO_SHMEM, 1);
        assert_eq!(AMDXDNA_BO_SHARE, 1);
        assert_eq!(AMDXDNA_BO_DEV_HEAP, 2);
        assert!(matches!(
            mlir_aie_ioctl_ask(MlirAieIoctlAsk::InstructionCreateBoType),
            Err(MlirAieIoctlError::UnprovenInstructionCreateBoType)
        ));
        // Caller-chosen copies. Not a cited packet and not a BO type.
        let cu = config_cu_from_caller(7, 1, 1);
        assert_eq!(cu.cu_bo, 7);
        assert_eq!(cu.cu_func, 1);
        assert_eq!(cu.num_cus, 1);
    }

    #[test]
    fn host_i16_shape_comes_from_opened_flags_not_the_directory_name() {
        let err = element_count_from_artifact_directory_name("whole_array_128_i16")
            .expect_err("directory name");
        assert!(matches!(
            err,
            MlirAieIoctlError::ElementCountNotInOpenedFile
        ));
        let missing = parse_whole_array_mkn("-k MLIR_AIE").expect_err("no flags");
        assert!(matches!(missing, MlirAieIoctlError::ShapeNotStated { .. }));

        let bufs = host_whole_array_i16_buffers().expect("opened submit command");
        assert_eq!((bufs.m, bufs.k, bufs.n), (128, 128, 128));
        let side = 128_usize * 128_usize;
        assert_eq!(bufs.a_i16.len(), side);
        assert_eq!(bufs.b_i16.len(), side);
        assert_eq!(bufs.golden_i32.len(), side);
        assert_eq!(bufs.host_golden.len(), side);
        assert_eq!(bufs.device_or_candidate.len(), side);
        assert!(!bufs.device_bytes_are_readback);
        assert!(bufs.host_golden.iter().any(|v| v.to_bits() != 0));
        assert!(bufs.device_or_candidate.iter().all(|v| v.to_bits() == 0));
        assert_eq!(i32_as_f32_bits(3).expect("exact"), 3.0_f32.to_bits());
        let g0 = bufs.golden_i32.first().copied().expect("golden cell");
        assert_eq!(
            bufs.host_golden[0].to_bits(),
            i32_as_f32_bits(g0).expect("golden cell exact")
        );
    }

    #[test]
    fn whole_array_host_opcode_is_three_not_an_ert_enumerator() {
        // test.cpp:229 `unsigned int opcode = 3`. That line does not name
        // ERT_START_NPU or ERT_START_DPU. ert.h:810 ERT_EXIT is also the
        // integer 3; this constant is not that name. 18 is ERT_START_DPU and
        // 20 is ERT_START_NPU. They are not the host opcode and are not stored.
        assert_eq!(WHOLE_ARRAY_HOST_OPCODE, 3);
        let bytes = mlir_aie_ioctl_ask(MlirAieIoctlAsk::WholeArrayHostOpcode);
        assert!(matches!(
            bytes,
            Err(MlirAieIoctlError::HostOpcodeWordWidthUncited)
        ));
    }

    #[test]
    fn whole_array_arg_order_is_names_only() {
        // test.cpp:232 and set_arg lines 234-239. Names, not address words.
        assert_eq!(
            WHOLE_ARRAY_HOST_ARG_ORDER,
            ["opcode", "instr", "ninstr", "A", "B", "C"]
        );
        assert!(matches!(
            mlir_aie_ioctl_ask(MlirAieIoctlAsk::ArgumentAddressWords),
            Err(MlirAieIoctlError::UnprovenArgumentAddressWords)
        ));
    }

    #[test]
    fn start_npu_members_are_names_and_c_types_without_offsets() {
        // amdxdna_ctx.h:47-52 and drivers/accel/amdxdna/amdxdna_ctx.h:43-48.
        // No numeric byte offset is asserted.
        assert_eq!(
            AMDXDNA_CMD_START_NPU_MEMBER_ORDER,
            [
                ("buffer", "u64"),
                ("buffer_size", "u32"),
                ("prop_count", "u32"),
                ("prop_args", "u32[]"),
            ]
        );
        assert!(matches!(
            mlir_aie_ioctl_ask(MlirAieIoctlAsk::AmdxdnaCmdStartNpuOffsets),
            Err(MlirAieIoctlError::UnprovenAmdxdnaCmdStartNpuOffsets)
        ));
    }

    fn i32_cells_to_le(cells: &[i32]) -> Vec<u8> {
        let mut out = Vec::with_capacity(cells.len().saturating_mul(4));
        for cell in cells {
            out.extend_from_slice(&cell.to_le_bytes());
        }
        out
    }

    fn golden_cells_as_f32(cells: &[i32]) -> Vec<f32> {
        cells
            .iter()
            .map(|cell| f32::from_bits(i32_as_f32_bits(*cell).expect("exact i32")))
            .collect()
    }

    #[test]
    fn closed_whole_array_compare_uses_buffers_and_verify_flags() {
        let golden = [4_i32, 9, 16];
        let bytes = i32_cells_to_le(&golden);
        let host_f32 = golden_cells_as_f32(&golden);
        let device_f32 = golden_cells_as_f32(&golden);
        let compare_ok = whole_array_output_matches_host(&bytes, &golden);
        let wired = apply_whole_array_verify_flags(NumericalVerifyInput {
            host_golden: &host_f32,
            device_or_candidate: &device_f32,
            had_instruction_blob: true,
            submit_wait_ok: true,
            real_ioctl_path: true,
            criteria: NumericalVerifyCriteria::default(),
        });
        let ran = decide_ran_on_npu(&wired);
        assert!(
            compare_ok
                && wired.had_instruction_blob
                && wired.submit_wait_ok
                && wired.real_ioctl_path
                && ran,
            "compare_ok={compare_ok} blob={} submit={} ioctl={} ran={ran}; compare discards bytes and the three flags are false",
            wired.had_instruction_blob,
            wired.submit_wait_ok,
            wired.real_ioctl_path,
        );

        let short = i32_cells_to_le(&[4_i32]);
        assert!(
            !whole_array_output_matches_host(&short, &golden),
            "length mismatch must fail"
        );
        let zeros = [0.0_f32, 0.0];
        let zero_wired = apply_whole_array_verify_flags(NumericalVerifyInput {
            host_golden: &zeros,
            device_or_candidate: &zeros,
            had_instruction_blob: true,
            submit_wait_ok: true,
            real_ioctl_path: true,
            criteria: NumericalVerifyCriteria::default(),
        });
        assert!(
            zero_wired.had_instruction_blob
                && zero_wired.submit_wait_ok
                && zero_wired.real_ioctl_path,
            "all-zero fixture still records the three flags"
        );
        assert!(!decide_ran_on_npu(&zero_wired), "all-zero golden must fail");
        let missing = apply_whole_array_verify_flags(NumericalVerifyInput {
            host_golden: &host_f32,
            device_or_candidate: &device_f32,
            had_instruction_blob: false,
            submit_wait_ok: true,
            real_ioctl_path: true,
            criteria: NumericalVerifyCriteria::default(),
        });
        assert!(!missing.had_instruction_blob, "missing blob stays false");
        assert!(!decide_ran_on_npu(&missing), "missing blob must fail");
        let no_submit = apply_whole_array_verify_flags(NumericalVerifyInput {
            host_golden: &host_f32,
            device_or_candidate: &device_f32,
            had_instruction_blob: true,
            submit_wait_ok: false,
            real_ioctl_path: true,
            criteria: NumericalVerifyCriteria::default(),
        });
        assert!(!no_submit.submit_wait_ok, "failed submit stays false");
        assert!(!decide_ran_on_npu(&no_submit), "failed submit must fail");
        let not_real = apply_whole_array_verify_flags(NumericalVerifyInput {
            host_golden: &host_f32,
            device_or_candidate: &device_f32,
            had_instruction_blob: true,
            submit_wait_ok: true,
            real_ioctl_path: false,
            criteria: NumericalVerifyCriteria::default(),
        });
        assert!(!not_real.real_ioctl_path, "non-ioctl path stays false");
        assert!(!decide_ran_on_npu(&not_real), "non-ioctl path must fail");
    }

    #[test]
    fn create_hwctx_retry_calls_shipped_function_and_does_not_open_accel() {
        use std::cell::RefCell;
        use std::time::Duration;

        fn accel_fd_open_in_this_process() -> bool {
            let Ok(entries) = std::fs::read_dir("/proc/self/fd") else {
                return false;
            };
            for entry in entries.flatten() {
                let Ok(target) = std::fs::read_link(entry.path()) else {
                    continue;
                };
                if target.to_string_lossy().starts_with("/dev/accel") {
                    return true;
                }
            }
            false
        }

        let argument = {
            let mut bytes = [0u8; 56];
            bytes[0] = 0x7e;
            bytes[55] = 0x7e;
            bytes
        };
        let run = |script: &[i32]| {
            let remaining = RefCell::new(script.to_vec());
            let seen = RefCell::new(Vec::<Vec<u8>>::new());
            let sleeps = RefCell::new(Vec::<Duration>::new());
            let records = super::retry_create_hwctx(
                &argument,
                |_index, bytes| {
                    seen.borrow_mut().push(bytes.to_vec());
                    let mut left = remaining.borrow_mut();
                    let next = left.remove(0);
                    if next == 0 { Ok(()) } else { Err(next) }
                },
                |wait| sleeps.borrow_mut().push(wait),
            );
            (records, seen.into_inner(), sleeps.into_inner())
        };

        let (eagain, seen, sleeps) = run(&[11, 11, 11, 11, 11, 22]);
        assert_eq!(eagain.len(), 5, "at most five CREATE_HWCTX tries");
        assert!(eagain.iter().all(|row| row.errno == 11));
        assert_eq!(seen.len(), 5);
        assert!(seen.iter().all(|bytes| bytes.as_slice() == argument));
        assert_eq!(sleeps.len(), 4, "sleep only between EAGAIN tries");
        assert!(
            sleeps
                .iter()
                .all(|wait| *wait == super::CREATE_HWCTX_RETRY_WAIT)
        );
        assert_eq!(super::CREATE_HWCTX_RETRY_WAIT, Duration::from_millis(200));

        let (other, seen_other, sleeps_other) = run(&[13]);
        assert_eq!(other.len(), 1);
        assert_eq!(other[0].errno, 13);
        assert_eq!(seen_other.len(), 1);
        assert!(
            sleeps_other.is_empty(),
            "a non-11 errno stops with no sleep"
        );

        let (early, seen_early, sleeps_early) = run(&[11, 0, 11]);
        assert_eq!(early.len(), 2);
        assert_eq!(early[0].errno, 11);
        assert_eq!(early[1].errno, 0);
        assert_eq!(seen_early.len(), 2);
        assert!(seen_early.iter().all(|bytes| bytes.as_slice() == argument));
        assert_eq!(sleeps_early.len(), 1);

        let (ok, seen_ok, sleeps_ok) = run(&[0]);
        assert_eq!(ok.len(), 1);
        assert_eq!(ok[0].errno, 0);
        assert_eq!(ok[0].index, 1);
        assert_eq!(seen_ok.len(), 1);
        assert!(sleeps_ok.is_empty(), "success stops");
        assert!(
            !accel_fd_open_in_this_process(),
            "this unit test must not open /dev/accel"
        );
    }
}
