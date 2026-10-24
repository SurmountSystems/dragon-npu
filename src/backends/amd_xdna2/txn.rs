//! Pure XAie transaction (ml_txn / ctrltext) header + op walk.
//!
//! # What this is
//!
//! Firmware instruction BOs for AIE2 START_NPU carry a **transaction opcode
//! stream** (not the ERT Cmd BO envelope). Public MIT layout is
//! `XAie_TxnHeader` + ops (`xaie_txn.h` from aie-rt / vendor includes).
//!
//! # What this is not
//!
//! - **Not** a silicon GEMM Pass. A well-formed header does not mean the ops
//!   implement MatMul or write OFM correctly.
//! - **Not** a rehost of AMD binaries. Tests use **synthetic** bytes only.
//! - v1.0 “optimized” txn streams (preempt save/restore in aiebu-asm-lib) share
//!   the same header but a different op packing — we validate the **header**
//!   only for those; full walk is implemented for classic **v0.1**.
//!
//! # Cross-links
//!
//! `docs/XDNA2_TXN_RESEARCH.md` · `scripts/materialize_txn_offline.sh`

use crate::backends::amd_xdna2::direct::DirectError;

/// Byte size of [`XaieTxnHeader`] on the wire (little-endian).
pub const XAIE_TXN_HEADER_SIZE: usize = 16;

/// Staged lab classic **content** size for `mladf … 1_1152_256_128` (ctrltext / map).
///
/// Live HITL: Transaction-map body == pure raw exactly (CLASS A unpatched).
pub const STAGED_MLADF_CLASSIC_INSTR_BYTES: usize = 1688;

/// `num_ops` for staged classic 1688 body (`0x28` LE in header).
pub const STAGED_MLADF_CLASSIC_NUM_OPS: u32 = 40;

/// HITL `get_transaction_bin` oversized body for the same shape (wrong body class).
///
/// Classic magic / v0.1 header layout, but **different** `num_ops`/`txn_size`.
/// Offline: `find(classic1688 in multi) == -1` — not a container of classic slices;
/// not ELF (`0x7fELF`). Do not treat prefix-as-C.
pub const HITL_MLADF_GET_TRANSACTION_BIN_BYTES: usize = 32_208;

/// `num_ops` observed on HITL multi-body dump (`0x1f4`).
pub const HITL_MLADF_GET_TRANSACTION_BIN_NUM_OPS: u32 = 500;

/// Classic (v0.1) WRITE op size including trailing size field.
const OP_WRITE_V01_SIZE: usize = 24;
/// Classic MASKWRITE / MASKPOLL / MASKPOLL_BUSY **compact** size (mlir-aie / public AOT).
///
/// Wire layout through the size field: OpHdr(4)+pad(4)+RegOff(8)+Value(4)+Mask(4)+Size(4)
/// = 28. Public mlir-aie npu2 peels (vector / single_core / whole_array / cascade) emit
/// `size=28` with no trailing pad.
const OP_MASK_V01_SIZE_COMPACT: usize = 28;
/// Classic MASKWRITE / MASKPOLL / MASKPOLL_BUSY **padded** size (DynamicDispatch / aiebu lab).
///
/// Same fields as compact through size@+24, plus 4 trailing pad bytes → `size=32`.
/// Lab classic 1688 and DD matmul txn dumps use this encoding.
const OP_MASK_V01_SIZE_PADDED: usize = 32;
/// NOOP / PREEMPT fixed size.
const OP_SHORT_SIZE: usize = 4;

/// True when `size` is a known classic v0.1 MASK* total (compact 28 or padded 32).
#[must_use]
const fn is_known_mask_v01_size(size: u32) -> bool {
    size as usize == OP_MASK_V01_SIZE_COMPACT || size as usize == OP_MASK_V01_SIZE_PADDED
}

/// `XAie_TxnOpcode` values we understand for walking (v0.1).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XaieTxnOpcode {
    /// `XAIE_IO_WRITE` — 32-bit register write.
    Write = 0,
    /// `XAIE_IO_BLOCKWRITE` — block of register data.
    BlockWrite = 1,
    /// `XAIE_IO_BLOCKSET` — block set pattern.
    BlockSet = 2,
    /// `XAIE_IO_MASKWRITE`.
    MaskWrite = 3,
    /// `XAIE_IO_MASKPOLL`.
    MaskPoll = 4,
    /// `XAIE_IO_NOOP`.
    Noop = 5,
    /// `XAIE_IO_PREEMPT`.
    Preempt = 6,
    /// `XAIE_IO_MASKPOLL_BUSY`.
    MaskPollBusy = 7,
    /// Custom ops start at `1 << 7` (TCT, DDR_PATCH, …).
    CustomBegin = 128,
}

impl XaieTxnOpcode {
    /// Map a raw opcode byte to a known variant (custom range → [`Self::CustomBegin`]).
    #[must_use]
    pub fn from_u8(v: u8) -> Option<Self> {
        Some(match v {
            0 => Self::Write,
            1 => Self::BlockWrite,
            2 => Self::BlockSet,
            3 => Self::MaskWrite,
            4 => Self::MaskPoll,
            5 => Self::Noop,
            6 => Self::Preempt,
            7 => Self::MaskPollBusy,
            128..=255 => Self::CustomBegin,
            _ => return None,
        })
    }
}

/// Parsed `XAie_TxnHeader` (16 bytes LE).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct XaieTxnHeader {
    /// Format major (classic export uses `0`).
    pub major: u8,
    /// Format minor (`1` = classic walk supported).
    pub minor: u8,
    /// Device generation (AIE2P Strix family often `4`).
    pub dev_gen: u8,
    /// Array rows.
    pub num_rows: u8,
    /// Array columns.
    pub num_cols: u8,
    /// Memory-tile rows.
    pub num_mem_tile_rows: u8,
    /// Number of ops following the header.
    pub num_ops: u32,
    /// Total txn size in bytes including this header.
    pub txn_size: u32,
}

impl XaieTxnHeader {
    /// Parse header from the first 16 bytes of a buffer.
    ///
    /// # Errors
    ///
    /// Buffer shorter than 16 bytes.
    pub fn parse(bytes: &[u8]) -> Result<Self, DirectError> {
        if bytes.len() < XAIE_TXN_HEADER_SIZE {
            return Err(DirectError::InvalidArgument(format!(
                "txn buffer too short for XAie_TxnHeader: {} < {XAIE_TXN_HEADER_SIZE}",
                bytes.len()
            )));
        }
        Ok(Self {
            major: bytes[0],
            minor: bytes[1],
            dev_gen: bytes[2],
            num_rows: bytes[3],
            num_cols: bytes[4],
            num_mem_tile_rows: bytes[5],
            // bytes[6..8] padding
            num_ops: u32::from_le_bytes(bytes[8..12].try_into().unwrap_or([0; 4])),
            txn_size: u32::from_le_bytes(bytes[12..16].try_into().unwrap_or([0; 4])),
        })
    }

    /// Encode header to 16 LE bytes.
    #[must_use]
    pub fn to_bytes(self) -> [u8; XAIE_TXN_HEADER_SIZE] {
        let mut out = [0_u8; XAIE_TXN_HEADER_SIZE];
        out[0] = self.major;
        out[1] = self.minor;
        out[2] = self.dev_gen;
        out[3] = self.num_rows;
        out[4] = self.num_cols;
        out[5] = self.num_mem_tile_rows;
        out[8..12].copy_from_slice(&self.num_ops.to_le_bytes());
        out[12..16].copy_from_slice(&self.txn_size.to_le_bytes());
        out
    }

    /// Header claims classic export format (full op walk supported).
    #[must_use]
    pub const fn is_classic_v01(self) -> bool {
        self.major == 0 && self.minor == 1
    }

    /// Staged lab classic profile: `num_ops == 40` and `txn_size == 1688`.
    ///
    /// Used to refuse silent wrong-body multi dumps (500 ops / 32208 B) as
    /// “same class” as pure raw for CLASS_HINT C.
    #[must_use]
    pub const fn is_staged_mladf_classic_profile(self) -> bool {
        self.num_ops == STAGED_MLADF_CLASSIC_NUM_OPS
            && self.txn_size as usize == STAGED_MLADF_CLASSIC_INSTR_BYTES
    }

    /// HITL `get_transaction_bin` wrong-body profile (500 ops / 32208 B).
    #[must_use]
    pub const fn is_hitl_mladf_multi_txn_profile(self) -> bool {
        self.num_ops == HITL_MLADF_GET_TRANSACTION_BIN_NUM_OPS
            && self.txn_size as usize == HITL_MLADF_GET_TRANSACTION_BIN_BYTES
    }

    /// Structural header checks (no op walk).
    ///
    /// - `txn_size == buffer_len`
    /// - `txn_size >= 16`
    /// - dimensions non-zero when `num_ops > 0`
    ///
    /// # Errors
    ///
    /// Inconsistent header vs buffer length / empty dimensions.
    pub fn validate_against_buffer(self, buffer_len: usize) -> Result<(), DirectError> {
        if self.txn_size as usize != buffer_len {
            return Err(DirectError::InvalidArgument(format!(
                "XAie_TxnHeader.TxnSize {} != buffer length {buffer_len}",
                self.txn_size
            )));
        }
        if (self.txn_size as usize) < XAIE_TXN_HEADER_SIZE {
            return Err(DirectError::InvalidArgument(format!(
                "XAie_TxnHeader.TxnSize {} < header size {XAIE_TXN_HEADER_SIZE}",
                self.txn_size
            )));
        }
        if self.num_ops > 0 && (self.num_cols == 0 || self.num_rows == 0) {
            return Err(DirectError::InvalidArgument(
                "XAie_TxnHeader has ops but NumCols/NumRows is zero".to_string(),
            ));
        }
        Ok(())
    }
}

/// One walked op (v0.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct XaieTxnOp {
    /// Raw `XAie_TxnOpcode` byte.
    pub opcode: u8,
    /// Byte offset of this op within the full buffer.
    pub byte_offset: usize,
    /// Byte length of this op (including payload for BLOCK/CUSTOM).
    pub byte_len: usize,
}

/// Result of classifying instruction-blob bytes (pure; no I/O).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstructionBlobKind {
    /// Classic v0.1 stream: header OK + full op walk OK.
    ClassicTxnV01 {
        /// Parsed header.
        header: XaieTxnHeader,
        /// Walked ops (`len == header.num_ops`).
        ops: Vec<XaieTxnOp>,
    },
    /// Header integrity OK; op packing not fully walked (e.g. v1.0 optimized).
    TxnHeaderOnly {
        /// Parsed header (size matches buffer).
        header: XaieTxnHeader,
        /// Why the classic walk failed.
        walk_error: String,
    },
    /// ELF32 with a `.ctrltext` PROGBITS section (aiebu output shape).
    Aie2ElfCtrltext {
        /// File offset of `.ctrltext`.
        ctrltext_off: usize,
        /// Byte length of `.ctrltext`.
        ctrltext_len: usize,
        /// Classification of the extracted `.ctrltext` bytes.
        ctrltext: Box<Self>,
    },
    /// Non-empty raw bytes that are not a recognizable XAie header / AIE ELF.
    UnknownRaw {
        /// Input length in bytes.
        len: usize,
    },
}

impl InstructionBlobKind {
    /// True when bytes look like a size-consistent XAie txn (classic or header-only).
    #[must_use]
    pub const fn looks_like_txn(&self) -> bool {
        matches!(
            self,
            Self::ClassicTxnV01 { .. } | Self::TxnHeaderOnly { .. } | Self::Aie2ElfCtrltext { .. }
        )
    }

    /// One-line diagnostic for logs / CLI (never claims silicon Pass).
    #[must_use]
    pub fn summary(&self) -> String {
        match self {
            Self::ClassicTxnV01 { header, ops } => format!(
                "classic_txn_v{}.{} gen{} {}x{} M{} ops={} size={} walked={}",
                header.major,
                header.minor,
                header.dev_gen,
                header.num_cols,
                header.num_rows,
                header.num_mem_tile_rows,
                header.num_ops,
                header.txn_size,
                ops.len()
            ),
            Self::TxnHeaderOnly { header, walk_error } => format!(
                "txn_header_only v{}.{} gen{} {}x{} M{} ops={} size={} walk_err={walk_error}",
                header.major,
                header.minor,
                header.dev_gen,
                header.num_cols,
                header.num_rows,
                header.num_mem_tile_rows,
                header.num_ops,
                header.txn_size
            ),
            Self::Aie2ElfCtrltext {
                ctrltext_off,
                ctrltext_len,
                ctrltext,
            } => format!(
                "aie2_elf_ctrltext off={ctrltext_off} len={ctrltext_len} inner={}",
                ctrltext.summary()
            ),
            Self::UnknownRaw { len } => format!("unknown_raw len={len}"),
        }
    }
}

/// True when `bytes` match staged classic **size + header profile** (1688 / 40 ops).
///
/// Does not require a full op walk; structural only. Not a vendor post-patch claim.
#[must_use]
pub fn is_staged_mladf_classic_instr_profile(bytes: &[u8]) -> bool {
    if bytes.len() != STAGED_MLADF_CLASSIC_INSTR_BYTES {
        return false;
    }
    XaieTxnHeader::parse(bytes).is_ok_and(|h| {
        h.is_staged_mladf_classic_profile() && h.validate_against_buffer(bytes.len()).is_ok()
    })
}

/// True when `bytes` match HITL oversized `get_transaction_bin` wrong-body profile.
///
/// Same classic header **layout** (maj/min) as staged classic, but different body
/// class — never treat as CLASS_HINT C vs pure raw.
#[must_use]
pub fn is_hitl_mladf_multi_txn_wrong_body_profile(bytes: &[u8]) -> bool {
    if bytes.len() != HITL_MLADF_GET_TRANSACTION_BIN_BYTES {
        return false;
    }
    XaieTxnHeader::parse(bytes).is_ok_and(|h| {
        h.is_hitl_mladf_multi_txn_profile() && h.validate_against_buffer(bytes.len()).is_ok()
    })
}

/// Whether `needle` appears as a contiguous byte slice anywhere in `haystack`.
///
/// Offline research (HITL multi 32208 vs classic 1688): `needle` is **not**
/// found as a full-body contiguous match (`find == -1`). That means multi is
/// not a container of the classic stream — do not invent a peel of multi→classic.
/// (Shared maj/min header layout alone is not an embed.)
#[must_use]
pub fn txn_bytes_embed_slice(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() || haystack.len() < needle.len() {
        return false;
    }
    haystack.windows(needle.len()).any(|w| w == needle)
}

/// Classify instruction BO candidate bytes (pure).
#[must_use]
pub fn classify_instruction_blob(bytes: &[u8]) -> InstructionBlobKind {
    if bytes.is_empty() {
        return InstructionBlobKind::UnknownRaw { len: 0 };
    }
    if let Some(kind) = try_classify_aie2_elf(bytes) {
        return kind;
    }
    classify_txn_stream(bytes)
}

fn classify_txn_stream(bytes: &[u8]) -> InstructionBlobKind {
    let Ok(header) = XaieTxnHeader::parse(bytes) else {
        return InstructionBlobKind::UnknownRaw { len: bytes.len() };
    };
    if header.validate_against_buffer(bytes.len()).is_err() {
        return InstructionBlobKind::UnknownRaw { len: bytes.len() };
    }
    match walk_txn_ops_v01(bytes, header) {
        Ok(ops) => {
            let walked = u32::try_from(ops.len()).unwrap_or(u32::MAX);
            if walked == header.num_ops {
                InstructionBlobKind::ClassicTxnV01 { header, ops }
            } else {
                InstructionBlobKind::TxnHeaderOnly {
                    header,
                    walk_error: format!(
                        "walked {} ops but header NumOps={}",
                        ops.len(),
                        header.num_ops
                    ),
                }
            }
        }
        Err(e) => InstructionBlobKind::TxnHeaderOnly {
            header,
            walk_error: e.to_string(),
        },
    }
}

/// Walk classic v0.1 ops; requires header already validated for length.
///
/// # Errors
///
/// Truncation, unknown opcode, size field mismatch, or trailing garbage.
pub fn walk_txn_ops_v01(
    bytes: &[u8],
    header: XaieTxnHeader,
) -> Result<Vec<XaieTxnOp>, DirectError> {
    let mut off = XAIE_TXN_HEADER_SIZE;
    let mut ops = Vec::with_capacity(header.num_ops as usize);
    for i in 0..header.num_ops {
        if off >= bytes.len() {
            return Err(DirectError::InvalidArgument(format!(
                "txn truncated at op {i} offset {off}"
            )));
        }
        let opcode = bytes[off];
        let start = off;
        let len = op_len_v01(bytes, off)?;
        off = off.saturating_add(len);
        if off > bytes.len() {
            return Err(DirectError::InvalidArgument(format!(
                "txn op {i} opcode={opcode} extends past buffer"
            )));
        }
        ops.push(XaieTxnOp {
            opcode,
            byte_offset: start,
            byte_len: len,
        });
    }
    if off != bytes.len() {
        return Err(DirectError::InvalidArgument(format!(
            "txn trailing bytes after ops: end_off={off} len={}",
            bytes.len()
        )));
    }
    Ok(ops)
}

fn op_len_v01(bytes: &[u8], off: usize) -> Result<usize, DirectError> {
    let opcode = bytes[off];
    match opcode {
        0 => {
            // WRITE: OpHdr(4)+pad(4)+RegOff(8)+Value(4)+Size(4)
            if off + OP_WRITE_V01_SIZE > bytes.len() {
                return Err(DirectError::InvalidArgument(
                    "WRITE op truncated".to_string(),
                ));
            }
            let size = u32::from_le_bytes(bytes[off + 20..off + 24].try_into().unwrap_or([0; 4]));
            if size as usize != OP_WRITE_V01_SIZE {
                return Err(DirectError::InvalidArgument(format!(
                    "WRITE size field {size} != {OP_WRITE_V01_SIZE}"
                )));
            }
            Ok(OP_WRITE_V01_SIZE)
        }
        3 | 4 | 7 => {
            // MASKWRITE / MASKPOLL / MASKPOLL_BUSY:
            //   compact 28 (mlir-aie public AOT) or padded 32 (lab DD/aiebu).
            // Size field is always LE u32 at +24; total op length = size.
            // Need at least compact length to read the size field.
            if off + OP_MASK_V01_SIZE_COMPACT > bytes.len() {
                return Err(DirectError::InvalidArgument(
                    "MASK* op truncated".to_string(),
                ));
            }
            let size = u32::from_le_bytes(bytes[off + 24..off + 28].try_into().unwrap_or([0; 4]));
            if !is_known_mask_v01_size(size) {
                return Err(DirectError::InvalidArgument(format!(
                    "MASK* size field {size} not in {{{OP_MASK_V01_SIZE_COMPACT},{OP_MASK_V01_SIZE_PADDED}}}"
                )));
            }
            let len = size as usize;
            if off + len > bytes.len() {
                return Err(DirectError::InvalidArgument(
                    "MASK* op truncated past size field".to_string(),
                ));
            }
            Ok(len)
        }
        1 | 2 => {
            // BLOCKWRITE / BLOCKSET: OpHdr(4)+pad(4)+RegOff u32 + Size u32 + payload
            if off + 16 > bytes.len() {
                return Err(DirectError::InvalidArgument(
                    "BLOCK* op header truncated".to_string(),
                ));
            }
            let size = u32::from_le_bytes(bytes[off + 12..off + 16].try_into().unwrap_or([0; 4]));
            if (size as usize) < 16 {
                return Err(DirectError::InvalidArgument(format!(
                    "BLOCK* size {size} < 16"
                )));
            }
            if off + size as usize > bytes.len() {
                return Err(DirectError::InvalidArgument(format!(
                    "BLOCK* size {size} past buffer"
                )));
            }
            Ok(size as usize)
        }
        5 | 6 => Ok(OP_SHORT_SIZE),
        128..=255 => {
            // CustomOpHdr: 4-byte OpHdr + u32 Size (total)
            if off + 8 > bytes.len() {
                return Err(DirectError::InvalidArgument(
                    "CUSTOM op header truncated".to_string(),
                ));
            }
            let size = u32::from_le_bytes(bytes[off + 4..off + 8].try_into().unwrap_or([0; 4]));
            if (size as usize) < 8 {
                return Err(DirectError::InvalidArgument(format!(
                    "CUSTOM size {size} < 8"
                )));
            }
            if off + size as usize > bytes.len() {
                return Err(DirectError::InvalidArgument(format!(
                    "CUSTOM size {size} past buffer"
                )));
            }
            Ok(size as usize)
        }
        _ => Err(DirectError::InvalidArgument(format!(
            "unsupported txn opcode {opcode} at offset {off} (v0.1 walk)"
        ))),
    }
}

/// Build a minimal classic v0.1 txn of `num_noops` NOOP ops (research only).
///
/// Grid defaults match AIE2P-style 4×6 + 1 mem-tile row (Strix family). **Not**
/// a known-good GEMM. Safe for unit tests and offline envelope dry-runs.
///
/// # Errors
///
/// `num_noops == 0` or size would overflow `u32`.
pub fn build_minimal_noop_txn(
    num_noops: u32,
    num_cols: u8,
    num_rows: u8,
    num_mem_tile_rows: u8,
    dev_gen: u8,
) -> Result<Vec<u8>, DirectError> {
    if num_noops == 0 {
        return Err(DirectError::InvalidArgument(
            "build_minimal_noop_txn requires num_noops >= 1".to_string(),
        ));
    }
    if num_cols == 0 || num_rows == 0 {
        return Err(DirectError::InvalidArgument(
            "build_minimal_noop_txn requires non-zero cols/rows".to_string(),
        ));
    }
    let body = usize::try_from(num_noops)
        .unwrap_or(usize::MAX)
        .saturating_mul(OP_SHORT_SIZE);
    let total = XAIE_TXN_HEADER_SIZE.saturating_add(body);
    let txn_size = u32::try_from(total)
        .map_err(|_| DirectError::InvalidArgument("txn size exceeds u32".to_string()))?;
    let header = XaieTxnHeader {
        major: 0,
        minor: 1,
        dev_gen,
        num_rows,
        num_cols,
        num_mem_tile_rows,
        num_ops: num_noops,
        txn_size,
    };
    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(&header.to_bytes());
    for _ in 0..num_noops {
        out.push(XaieTxnOpcode::Noop as u8);
        out.extend_from_slice(&[0, 0, 0]); // pad
    }
    debug_assert_eq!(out.len(), total);
    Ok(out)
}

/// Default Strix-oriented research NOOP txn (4 cols × 6 rows, gen 4, 4 NOOPs).
///
/// # Errors
///
/// Propagates from [`build_minimal_noop_txn`].
pub fn build_strix_research_noop_txn() -> Result<Vec<u8>, DirectError> {
    build_minimal_noop_txn(4, 4, 6, 1, 4)
}

/// Validate that `bytes` are a size-consistent XAie txn (classic walk preferred).
///
/// # Errors
///
/// Empty, unknown raw, or header inconsistency. Header-only (v1.0) is **Ok**
/// with the kind — callers that need full walk should match on
/// [`InstructionBlobKind::ClassicTxnV01`].
pub fn validate_txn_blob(bytes: &[u8]) -> Result<InstructionBlobKind, DirectError> {
    if bytes.is_empty() {
        return Err(DirectError::InvalidArgument(
            "txn blob is empty".to_string(),
        ));
    }
    let kind = classify_instruction_blob(bytes);
    if kind.looks_like_txn() {
        Ok(kind)
    } else {
        Err(DirectError::InvalidArgument(format!(
            "bytes are not a recognizable XAie txn/ELF ctrltext ({})",
            kind.summary()
        )))
    }
}

/// If `bytes` is ELF32 little-endian with a `.ctrltext` section, classify it.
fn try_classify_aie2_elf(bytes: &[u8]) -> Option<InstructionBlobKind> {
    // ELF32 magic + class 1 + data 1 (LE)
    if bytes.len() < 52 {
        return None;
    }
    if bytes[0..4] != [0x7f, b'E', b'L', b'F'] {
        return None;
    }
    if bytes[4] != 1 || bytes[5] != 1 {
        return None; // not ELF32 LE
    }
    let e_shoff = u32::from_le_bytes(bytes[32..36].try_into().ok()?) as usize;
    let e_shentsize = u16::from_le_bytes(bytes[46..48].try_into().ok()?) as usize;
    let e_shnum = u16::from_le_bytes(bytes[48..50].try_into().ok()?) as usize;
    let e_shstrndx = u16::from_le_bytes(bytes[50..52].try_into().ok()?) as usize;
    if e_shentsize < 40 || e_shnum == 0 || e_shstrndx >= e_shnum {
        return None;
    }
    let sh_table_end = e_shoff.checked_add(e_shentsize.checked_mul(e_shnum)?)?;
    if sh_table_end > bytes.len() {
        return None;
    }
    // Section header: sh_name u32, sh_type u32, sh_flags u32, sh_addr u32,
    // sh_offset u32, sh_size u32, ...
    let str_hdr_off = e_shoff + e_shentsize * e_shstrndx;
    let str_off =
        u32::from_le_bytes(bytes[str_hdr_off + 16..str_hdr_off + 20].try_into().ok()?) as usize;
    let str_size =
        u32::from_le_bytes(bytes[str_hdr_off + 20..str_hdr_off + 24].try_into().ok()?) as usize;
    if str_off.checked_add(str_size)? > bytes.len() {
        return None;
    }
    let strtab = &bytes[str_off..str_off + str_size];

    for i in 0..e_shnum {
        let h = e_shoff + e_shentsize * i;
        let name_off = u32::from_le_bytes(bytes[h..h + 4].try_into().ok()?) as usize;
        let sh_type = u32::from_le_bytes(bytes[h + 4..h + 8].try_into().ok()?);
        if sh_type != 1 {
            continue; // SHT_PROGBITS
        }
        let name = read_c_string(strtab, name_off)?;
        if name != ".ctrltext" {
            continue;
        }
        let sec_off = u32::from_le_bytes(bytes[h + 16..h + 20].try_into().ok()?) as usize;
        let sec_size = u32::from_le_bytes(bytes[h + 20..h + 24].try_into().ok()?) as usize;
        if sec_size == 0 || sec_off.checked_add(sec_size)? > bytes.len() {
            return None;
        }
        let ctrl = &bytes[sec_off..sec_off + sec_size];
        let inner = classify_txn_stream(ctrl);
        return Some(InstructionBlobKind::Aie2ElfCtrltext {
            ctrltext_off: sec_off,
            ctrltext_len: sec_size,
            ctrltext: Box::new(inner),
        });
    }
    None
}

fn read_c_string(tab: &[u8], off: usize) -> Option<&str> {
    if off >= tab.len() {
        return None;
    }
    let end = tab[off..].iter().position(|&c| c == 0)? + off;
    std::str::from_utf8(&tab[off..end]).ok()
}

/// Extract `.ctrltext` bytes from an aiebu-style ELF32, or return raw bytes if
/// already a txn stream.
///
/// # Errors
///
/// Empty input. Unknown ELF without `.ctrltext` still returns the original
/// bytes as `Cow`-like owned copy via `Ok` only when non-empty — prefer
/// [`classify_instruction_blob`] for diagnostics.
pub fn instruction_payload_bytes(bytes: &[u8]) -> Result<Vec<u8>, DirectError> {
    if bytes.is_empty() {
        return Err(DirectError::InvalidArgument(
            "instruction payload is empty".to_string(),
        ));
    }
    match classify_instruction_blob(bytes) {
        InstructionBlobKind::Aie2ElfCtrltext {
            ctrltext_off,
            ctrltext_len,
            ..
        } => Ok(bytes[ctrltext_off..ctrltext_off + ctrltext_len].to_vec()),
        _ => Ok(bytes.to_vec()),
    }
}

/// True when `bytes` look like ELF32 LE with a `.ctrltext` section (aiebu output).
#[must_use]
pub fn is_aiebu_elf_ctrltext(bytes: &[u8]) -> bool {
    matches!(
        classify_instruction_blob(bytes),
        InstructionBlobKind::Aie2ElfCtrltext { .. }
    )
}

/// One ELF32 `.rela.dyn` entry (12 bytes): `r_offset`, `r_info`, `r_addend`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Elf32Rela {
    /// Offset into the section being relocated (here: `.ctrltext`).
    pub offset: u32,
    /// `(sym << 8) | type` for ELF32.
    pub info: u32,
    /// Addend added to the symbol value.
    pub addend: i32,
}

impl Elf32Rela {
    /// Symbol index from `r_info`.
    #[must_use]
    pub fn sym(&self) -> u32 {
        self.info >> 8
    }
    /// Relocation type from `r_info`.
    #[must_use]
    pub fn rtype(&self) -> u32 {
        self.info & 0xff
    }
}

/// How to apply aiebu `.rela.dyn` into `.ctrltext` reloc holes.
///
/// Live staged ELF holes hold small placeholders (`0xb1` / `0x1e` / `0x159`);
/// default is **overwrite** with `base+addend` (XRT-style absolute store).
/// `Add` is a residual trial: RMW `placeholder + base + addend`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LabRelaMode {
    /// Store `base.wrapping_add(addend)` (default).
    #[default]
    Overwrite,
    /// Store `existing.wrapping_add(base).wrapping_add(addend)`.
    Add,
}

impl LabRelaMode {
    /// Stable token for detail / contract JSON.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Overwrite => "overwrite",
            Self::Add => "add",
        }
    }
}

/// Fail-closed parse of `XDNA2_LAB_RELA_MODE`.
///
/// Unset / empty → [`LabRelaMode::Overwrite`]. Known: `overwrite`, `add`
/// (alias `rmw` → add). Unknown → `Err`.
///
/// # Errors
///
/// Unrecognized non-empty token.
pub fn try_lab_rela_mode_from_str(raw: Option<&str>) -> Result<LabRelaMode, String> {
    match raw.map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(LabRelaMode::Overwrite),
        Some(s) if s.eq_ignore_ascii_case("overwrite") => Ok(LabRelaMode::Overwrite),
        Some(s) if s.eq_ignore_ascii_case("add") || s.eq_ignore_ascii_case("rmw") => {
            Ok(LabRelaMode::Add)
        }
        Some(s) => Err(format!(
            "unknown XDNA2_LAB_RELA_MODE={s:?}; expected overwrite|add (alias rmw→add)"
        )),
    }
}

/// Convenience for known-good / None; unknown falls back to overwrite (prefer try).
#[must_use]
pub fn lab_rela_mode_from_str(raw: Option<&str>) -> LabRelaMode {
    try_lab_rela_mode_from_str(raw).unwrap_or(LabRelaMode::Overwrite)
}

/// Read process env `XDNA2_LAB_RELA_MODE` (fail-closed on unknown).
///
/// # Errors
///
/// Unknown token set in env.
pub fn lab_rela_mode_from_env() -> Result<LabRelaMode, String> {
    try_lab_rela_mode_from_str(std::env::var("XDNA2_LAB_RELA_MODE").ok().as_deref())
}

/// One ELF32 dynamic symbol (`.dynsym` + name from linked `.dynstr`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Elf32DynSym {
    /// Symbol table index (0 = undefined).
    pub index: u32,
    /// Name from `.dynstr` (empty for index 0 / missing).
    pub name: String,
    /// `st_value`.
    pub value: u32,
    /// `st_size` (aiebu often encodes buffer role sizes; names may be opaque).
    pub size: u32,
    /// `st_info` (bind/type).
    pub info: u8,
    /// `st_other`.
    pub other: u8,
    /// `st_shndx`.
    pub shndx: u16,
}

/// Parse ELF32 LE `.dynsym` entries with names from the linked string table.
///
/// Staged aiebu `lab_instr.elf` has 4 symbols; non-null names are all **`"6"`**
/// (opaque aiebu encoding — do **not** invent a name→role map without evidence).
/// Lab patch map remains **index** `1→ifm`, `2→wts`, `3→ofm`, `4→ctrl`.
///
/// # Errors
///
/// Truncated section table / dynsym / dynstr.
#[allow(clippy::missing_panics_doc)] // try_into after length guards
pub fn parse_elf32_dynsym(elf: &[u8]) -> Result<Vec<Elf32DynSym>, DirectError> {
    if elf.len() < 52 || elf[0..4] != [0x7f, b'E', b'L', b'F'] || elf[4] != 1 || elf[5] != 1 {
        return Ok(Vec::new());
    }
    let e_shoff = u32::from_le_bytes(elf[32..36].try_into().unwrap()) as usize;
    let e_shentsize = u16::from_le_bytes(elf[46..48].try_into().unwrap()) as usize;
    let e_shnum = u16::from_le_bytes(elf[48..50].try_into().unwrap()) as usize;
    let e_shstrndx = u16::from_le_bytes(elf[50..52].try_into().unwrap()) as usize;
    if e_shentsize < 40 || e_shnum == 0 || e_shstrndx >= e_shnum {
        return Ok(Vec::new());
    }
    let sh_end = e_shoff
        .checked_add(e_shentsize.saturating_mul(e_shnum))
        .ok_or_else(|| DirectError::InvalidArgument("elf sh_table overflow".into()))?;
    if sh_end > elf.len() {
        return Err(DirectError::InvalidArgument(
            "elf section table out of range".into(),
        ));
    }
    let str_hdr = e_shoff + e_shentsize * e_shstrndx;
    let str_off = u32::from_le_bytes(elf[str_hdr + 16..str_hdr + 20].try_into().unwrap()) as usize;
    let str_size = u32::from_le_bytes(elf[str_hdr + 20..str_hdr + 24].try_into().unwrap()) as usize;
    if str_off.saturating_add(str_size) > elf.len() {
        return Err(DirectError::InvalidArgument("elf shstrtab OOB".into()));
    }
    let shstrtab = &elf[str_off..str_off + str_size];

    let mut dynsym_i: Option<usize> = None;
    for i in 0..e_shnum {
        let h = e_shoff + e_shentsize * i;
        let name_off = u32::from_le_bytes(elf[h..h + 4].try_into().unwrap()) as usize;
        let sh_type = u32::from_le_bytes(elf[h + 4..h + 8].try_into().unwrap());
        // SHT_DYNSYM = 11
        if sh_type != 11 {
            continue;
        }
        let name = read_c_string(shstrtab, name_off).unwrap_or("");
        if name == ".dynsym" {
            dynsym_i = Some(i);
            break;
        }
    }
    let Some(di) = dynsym_i else {
        return Ok(Vec::new());
    };
    let h = e_shoff + e_shentsize * di;
    let sec_off = u32::from_le_bytes(elf[h + 16..h + 20].try_into().unwrap()) as usize;
    let sec_size = u32::from_le_bytes(elf[h + 20..h + 24].try_into().unwrap()) as usize;
    let sh_link = u32::from_le_bytes(elf[h + 24..h + 28].try_into().unwrap()) as usize;
    let mut entsize = u32::from_le_bytes(elf[h + 36..h + 40].try_into().unwrap()) as usize;
    if entsize == 0 {
        entsize = 16; // ELF32 Sym
    }
    if entsize < 16
        || !sec_size.is_multiple_of(entsize)
        || sec_off.saturating_add(sec_size) > elf.len()
    {
        return Err(DirectError::InvalidArgument(
            "elf .dynsym size/offset invalid".into(),
        ));
    }
    if sh_link >= e_shnum {
        return Err(DirectError::InvalidArgument(
            "elf .dynsym sh_link OOB".into(),
        ));
    }
    let dh = e_shoff + e_shentsize * sh_link;
    let d_off = u32::from_le_bytes(elf[dh + 16..dh + 20].try_into().unwrap()) as usize;
    let d_size = u32::from_le_bytes(elf[dh + 20..dh + 24].try_into().unwrap()) as usize;
    if d_off.saturating_add(d_size) > elf.len() {
        return Err(DirectError::InvalidArgument("elf dynstr OOB".into()));
    }
    let dynstr = &elf[d_off..d_off + d_size];
    let n = sec_size / entsize;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let o = sec_off + i * entsize;
        let st_name = u32::from_le_bytes(elf[o..o + 4].try_into().unwrap()) as usize;
        let st_value = u32::from_le_bytes(elf[o + 4..o + 8].try_into().unwrap());
        let st_sz = u32::from_le_bytes(elf[o + 8..o + 12].try_into().unwrap());
        let st_info = elf[o + 12];
        let st_other = elf[o + 13];
        let st_shndx = u16::from_le_bytes(elf[o + 14..o + 16].try_into().unwrap());
        let name = read_c_string(dynstr, st_name).unwrap_or("").to_string();
        out.push(Elf32DynSym {
            index: u32::try_from(i).unwrap_or(u32::MAX),
            name,
            value: st_value,
            size: st_sz,
            info: st_info,
            other: st_other,
            shndx: st_shndx,
        });
    }
    Ok(out)
}

/// Short dynsym summary for contract / GT dumps (no invented name→role map).
#[must_use]
pub fn format_dynsym_report(syms: &[Elf32DynSym]) -> String {
    if syms.is_empty() {
        return "dynsym empty".into();
    }
    let names: Vec<&str> = syms.iter().map(|s| s.name.as_str()).collect();
    let sizes: Vec<String> = syms.iter().map(|s| s.size.to_string()).collect();
    let all_six = syms
        .iter()
        .skip(1)
        .all(|s| s.name == "6" || s.name.is_empty());
    let note = if all_six && syms.len() > 1 {
        " names_all_\"6\"_opaque_do_not_invent_map"
    } else {
        ""
    };
    format!(
        "dynsym_n={} names=[{}] sizes=[{}]{note}",
        syms.len(),
        names
            .iter()
            .map(|n| format!("\"{n}\""))
            .collect::<Vec<_>>()
            .join(","),
        sizes.join(",")
    )
}

/// Parse ELF32 little-endian `.rela.dyn` entries (empty if missing).
///
/// # Errors
///
/// Truncated section table / rela table.
///
/// # Panics
///
/// Does not panic: fixed-width LE reads use `try_into` only after length checks.
#[allow(clippy::missing_panics_doc)] // try_into after `elf.len() < 52` guard
pub fn parse_elf32_rela_dyn(elf: &[u8]) -> Result<Vec<Elf32Rela>, DirectError> {
    if elf.len() < 52 || elf[0..4] != [0x7f, b'E', b'L', b'F'] || elf[4] != 1 || elf[5] != 1 {
        return Ok(Vec::new());
    }
    let e_shoff = u32::from_le_bytes(elf[32..36].try_into().unwrap()) as usize;
    let e_shentsize = u16::from_le_bytes(elf[46..48].try_into().unwrap()) as usize;
    let e_shnum = u16::from_le_bytes(elf[48..50].try_into().unwrap()) as usize;
    let e_shstrndx = u16::from_le_bytes(elf[50..52].try_into().unwrap()) as usize;
    if e_shentsize < 40 || e_shnum == 0 || e_shstrndx >= e_shnum {
        return Ok(Vec::new());
    }
    let sh_end = e_shoff
        .checked_add(e_shentsize.saturating_mul(e_shnum))
        .ok_or_else(|| DirectError::InvalidArgument("elf sh_table overflow".into()))?;
    if sh_end > elf.len() {
        return Err(DirectError::InvalidArgument(
            "elf section table out of range".into(),
        ));
    }
    let str_hdr = e_shoff + e_shentsize * e_shstrndx;
    let str_off = u32::from_le_bytes(elf[str_hdr + 16..str_hdr + 20].try_into().unwrap()) as usize;
    let str_size = u32::from_le_bytes(elf[str_hdr + 20..str_hdr + 24].try_into().unwrap()) as usize;
    if str_off.saturating_add(str_size) > elf.len() {
        return Err(DirectError::InvalidArgument("elf shstrtab OOB".into()));
    }
    let strtab = &elf[str_off..str_off + str_size];
    for i in 0..e_shnum {
        let h = e_shoff + e_shentsize * i;
        let name_off = u32::from_le_bytes(elf[h..h + 4].try_into().unwrap()) as usize;
        let sh_type = u32::from_le_bytes(elf[h + 4..h + 8].try_into().unwrap());
        // SHT_RELA = 4
        if sh_type != 4 {
            continue;
        }
        let name = read_c_string(strtab, name_off).unwrap_or("");
        if name != ".rela.dyn" {
            continue;
        }
        let sec_off = u32::from_le_bytes(elf[h + 16..h + 20].try_into().unwrap()) as usize;
        let sec_size = u32::from_le_bytes(elf[h + 20..h + 24].try_into().unwrap()) as usize;
        if sec_off.saturating_add(sec_size) > elf.len() || !sec_size.is_multiple_of(12) {
            return Err(DirectError::InvalidArgument(
                "elf .rela.dyn size/offset invalid".into(),
            ));
        }
        let mut out = Vec::with_capacity(sec_size / 12);
        let mut o = sec_off;
        while o + 12 <= sec_off + sec_size {
            let offset = u32::from_le_bytes(elf[o..o + 4].try_into().unwrap());
            let info = u32::from_le_bytes(elf[o + 4..o + 8].try_into().unwrap());
            let addend = i32::from_le_bytes(elf[o + 8..o + 12].try_into().unwrap());
            out.push(Elf32Rela {
                offset,
                info,
                addend,
            });
            o += 12;
        }
        return Ok(out);
    }
    Ok(Vec::new())
}

/// Peel `.ctrltext` and apply aiebu `.rela.dyn` with device/buffer paddrs.
///
/// Default mode is [`LabRelaMode::Overwrite`]. See
/// [`patch_aiebu_elf_ctrltext_with_bos_ex`] for `Add` residual trial.
///
/// # Errors
///
/// Not an aiebu ELF, OOB reloc, or empty ctrltext.
pub fn patch_aiebu_elf_ctrltext_with_bos(
    elf: &[u8],
    ifm_paddr: u64,
    wts_paddr: u64,
    ofm_paddr: u64,
    ctrl_paddr: u64,
) -> Result<Vec<u8>, DirectError> {
    patch_aiebu_elf_ctrltext_with_bos_ex(
        elf,
        ifm_paddr,
        wts_paddr,
        ofm_paddr,
        ctrl_paddr,
        LabRelaMode::Overwrite,
    )
}

/// Peel `.ctrltext` and apply aiebu `.rela.dyn` with explicit [`LabRelaMode`].
///
/// XRT/shim (`txn_file2elf` + `patch_ctrl_code`) does this before EXEC. Submitting
/// **raw** DD txn (or unpatched ctrltext) yields FW `APP_LOAD_PDI_FAIL` (0x3000003)
/// on live silicon (2026-07-16).
///
/// **Rela symbol index** bases (not dynsym name/size roles): `1→ifm`, `2→wts`,
/// `3→ofm`, `4→ctrl` when present. Staged mladf uses sym **1–3 only** (no sym4
/// ctrl reloc — ctrl is meta/packet path). Dynsym names may all be opaque `"6"`.
/// Reloc type is treated as **u64 LE** at `ctrltext[offset]`:
/// - **overwrite** (default): `base + addend`
/// - **add**: `existing_placeholder + base + addend` (holes often `0xb1`/`0x1e`/`0x159`)
///
/// # Errors
///
/// Not an aiebu ELF, OOB reloc, or empty ctrltext.
pub fn patch_aiebu_elf_ctrltext_with_bos_ex(
    elf: &[u8],
    ifm_paddr: u64,
    wts_paddr: u64,
    ofm_paddr: u64,
    ctrl_paddr: u64,
    mode: LabRelaMode,
) -> Result<Vec<u8>, DirectError> {
    let (off, len) = match classify_instruction_blob(elf) {
        InstructionBlobKind::Aie2ElfCtrltext {
            ctrltext_off,
            ctrltext_len,
            ..
        } => (ctrltext_off, ctrltext_len),
        other => {
            return Err(DirectError::InvalidArgument(format!(
                "patch_aiebu_elf_ctrltext: need aiebu ELF with .ctrltext, got {}",
                other.summary()
            )));
        }
    };
    let mut ctrltext = elf[off..off + len].to_vec();
    let relas = parse_elf32_rela_dyn(elf)?;
    if relas.is_empty() {
        return Err(DirectError::InvalidArgument(
            "patch_aiebu_elf_ctrltext: ELF has no .rela.dyn entries to apply".into(),
        ));
    }
    let bases = [0_u64, ifm_paddr, wts_paddr, ofm_paddr, ctrl_paddr];
    for r in &relas {
        let sym = r.sym() as usize;
        let base = bases.get(sym).copied().unwrap_or(0);
        // ELF addend is signed; store as wrapping u64 (XRT-style address math).
        #[allow(clippy::cast_sign_loss)]
        let reloc = base.wrapping_add(r.addend as u64);
        let o = r.offset as usize;
        if o.checked_add(8).is_none_or(|end| end > ctrltext.len()) {
            return Err(DirectError::InvalidArgument(format!(
                "patch_aiebu_elf_ctrltext: reloc offset {o}+8 OOB (ctrltext {})",
                ctrltext.len()
            )));
        }
        let value = match mode {
            LabRelaMode::Overwrite => reloc,
            LabRelaMode::Add => {
                let existing = u64::from_le_bytes(ctrltext[o..o + 8].try_into().unwrap_or([0; 8]));
                existing.wrapping_add(reloc)
            }
        };
        // Type 5 observed on aiebu aie2txn output — always write u64 LE.
        ctrltext[o..o + 8].copy_from_slice(&value.to_le_bytes());
    }
    Ok(ctrltext)
}

/// After rela patch: require classic v0.1 walk (fail-closed offline).
///
/// Staged mladf ctrltext: `txn_size=1688`, `num_ops=40`.
///
/// # Errors
///
/// Walk/header fail, or (when `expect_staged`) size/ops mismatch.
pub fn validate_patched_ctrltext_classic(
    patched: &[u8],
    expect_staged: bool,
) -> Result<InstructionBlobKind, DirectError> {
    let kind = validate_txn_blob(patched)?;
    match &kind {
        InstructionBlobKind::ClassicTxnV01 { header, ops } => {
            if expect_staged {
                if header.txn_size != 1688 || patched.len() != 1688 {
                    return Err(DirectError::InvalidArgument(format!(
                        "patched ctrltext size {} txn_size={} expected 1688",
                        patched.len(),
                        header.txn_size
                    )));
                }
                if header.num_ops != 40 || ops.len() != 40 {
                    return Err(DirectError::InvalidArgument(format!(
                        "patched ctrltext num_ops={} ops.len()={} expected 40",
                        header.num_ops,
                        ops.len()
                    )));
                }
            }
            Ok(kind)
        }
        other => Err(DirectError::InvalidArgument(format!(
            "patched ctrltext is not ClassicTxnV01 walkable: {}",
            other.summary()
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn patch_aiebu_elf_applies_rela_u64() {
        // Prefer live staged ELF if present (gitignored); else skip-shaped synthetic.
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("testdata/xdna2_cmd/silicon_lab/lab_instr.elf");
        if !path.is_file() {
            eprintln!("skip: no lab_instr.elf (staged asset)");
            return;
        }
        let elf = std::fs::read(&path).expect("read elf");
        assert!(is_aiebu_elf_ctrltext(&elf));
        let relas = parse_elf32_rela_dyn(&elf).expect("rela");
        assert!(!relas.is_empty(), "expected .rela.dyn entries");
        // Staged mladf ELF: 12 relas (8×sym1 IFM, 2×sym2 WTS, 2×sym3 OFM).
        assert_eq!(relas.len(), 12, "staged ELF rela count");
        let ifm = 0x1111_0000_u64;
        let wts = 0x2222_0000_u64;
        let ofm = 0x3333_0000_u64;
        let ctrl = 0x4444_0000_u64;
        let patched = patch_aiebu_elf_ctrltext_with_bos(&elf, ifm, wts, ofm, ctrl).expect("patch");
        assert_eq!(patched.len(), 0x698);
        let bases = [0_u64, ifm, wts, ofm, ctrl];
        for r in &relas {
            let base = bases.get(r.sym() as usize).copied().unwrap_or(0);
            #[allow(clippy::cast_sign_loss)]
            let expect = base.wrapping_add(r.addend as u64);
            let o = r.offset as usize;
            let v = u64::from_le_bytes(patched[o..o + 8].try_into().unwrap());
            assert_eq!(
                v,
                expect,
                "rela off={o:#x} sym={} addend={}",
                r.sym(),
                r.addend
            );
        }
        // First reloc at 0x2c sym1 + 0 → ifm
        let v = u64::from_le_bytes(patched[0x2c..0x2c + 8].try_into().unwrap());
        assert_eq!(v, ifm);
        // Post-patch must remain classic v0.1 walkable (staged 40 ops / 1688 B).
        validate_patched_ctrltext_classic(&patched, true).expect("post-patch walk");
    }

    #[test]
    fn patch_aiebu_rela_mode_add_includes_placeholder() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("testdata/xdna2_cmd/silicon_lab/lab_instr.elf");
        if !path.is_file() {
            eprintln!("skip: no lab_instr.elf (staged asset)");
            return;
        }
        let elf = std::fs::read(&path).expect("read elf");
        let relas = parse_elf32_rela_dyn(&elf).expect("rela");
        let ifm = 0x1111_0000_u64;
        let wts = 0x2222_0000_u64;
        let ofm = 0x3333_0000_u64;
        let ctrl = 0x4444_0000_u64;
        // Capture first hole before patch (staged: 0xb1 at off 0x2c).
        let InstructionBlobKind::Aie2ElfCtrltext {
            ctrltext_off: coff,
            ctrltext_len: clen,
            ..
        } = classify_instruction_blob(&elf)
        else {
            panic!("need elf")
        };
        let pre = &elf[coff..coff + clen];
        let hole0 = u64::from_le_bytes(pre[0x2c..0x2c + 8].try_into().unwrap());
        assert_eq!(hole0, 0xb1, "staged IFM hole placeholder");

        let patched =
            patch_aiebu_elf_ctrltext_with_bos_ex(&elf, ifm, wts, ofm, ctrl, LabRelaMode::Add)
                .expect("add mode");
        let bases = [0_u64, ifm, wts, ofm, ctrl];
        for r in &relas {
            let base = bases.get(r.sym() as usize).copied().unwrap_or(0);
            let o = r.offset as usize;
            let existing = u64::from_le_bytes(pre[o..o + 8].try_into().unwrap());
            #[allow(clippy::cast_sign_loss)]
            let expect = existing.wrapping_add(base.wrapping_add(r.addend as u64));
            let v = u64::from_le_bytes(patched[o..o + 8].try_into().unwrap());
            assert_eq!(v, expect, "add mode off={o:#x}");
        }
        // First site: 0xb1 + ifm + 0
        let v = u64::from_le_bytes(patched[0x2c..0x2c + 8].try_into().unwrap());
        assert_eq!(v, ifm.wrapping_add(0xb1));
        assert_eq!(
            try_lab_rela_mode_from_str(Some("add")).unwrap(),
            LabRelaMode::Add
        );
        assert_eq!(
            try_lab_rela_mode_from_str(None).unwrap(),
            LabRelaMode::Overwrite
        );
        // Add mode patched stream must still walk as staged classic.
        validate_patched_ctrltext_classic(&patched, true).expect("add post-patch walk");
    }

    #[test]
    fn patch_synthetic_elf_add_mode_rmw_placeholder() {
        // Asset-free: hole 0xb1 at offset 0, sym1, addend 7.
        // Pure-direct only — not vendor post-patch / not silicon Pass / not C-ready.
        let mut ctrl = vec![0_u8; 64];
        ctrl[0..8].copy_from_slice(&0xb1_u64.to_le_bytes());
        let elf = crate::backends::amd_xdna2::silicon_contract::build_minimal_test_elf32(
            &ctrl,
            &[(0, (1 << 8) | 5, 7)],
        );
        let ifm = 0x1000_u64;
        let patched = patch_aiebu_elf_ctrltext_with_bos_ex(&elf, ifm, 0, 0, 0, LabRelaMode::Add)
            .expect("add");
        let v = u64::from_le_bytes(patched[0..8].try_into().unwrap());
        assert_eq!(v, 0xb1_u64.wrapping_add(ifm).wrapping_add(7));
        let over = patch_aiebu_elf_ctrltext_with_bos_ex(&elf, ifm, 0, 0, 0, LabRelaMode::Overwrite)
            .expect("over");
        let vo = u64::from_le_bytes(over[0..8].try_into().unwrap());
        assert_eq!(vo, ifm.wrapping_add(7));
    }

    /// Pure-direct multi-symbol synthetic ELF: sym 1–4 map ifm/wts/ofm/ctrl.
    ///
    /// Labels pure-direct patch algorithm only — **not** vendor post-patch,
    /// **not** silicon Pass, **not** C-ready / `ran_on_npu`.
    #[test]
    fn patch_aiebu_synthetic_multi_sym_overwrite_and_add_pure_direct() {
        let mut ctrl = vec![0_u8; 64];
        // Staged-like placeholders at 0 / 16 / 32 / 48.
        ctrl[0..8].copy_from_slice(&0xb1_u64.to_le_bytes());
        ctrl[16..24].copy_from_slice(&0x1e_u64.to_le_bytes());
        ctrl[32..40].copy_from_slice(&0x159_u64.to_le_bytes());
        ctrl[48..56].copy_from_slice(&0x42_u64.to_le_bytes());
        // info = (sym << 8) | type5
        let relas = [
            (0_u32, (1 << 8) | 5, 0_i32),
            (16, (2 << 8) | 5, 10),
            (32, (3 << 8) | 5, 20),
            (48, (4 << 8) | 5, 30),
        ];
        let elf =
            crate::backends::amd_xdna2::silicon_contract::build_minimal_test_elf32(&ctrl, &relas);
        let ifm = 0x1111_0000_u64;
        let wts = 0x2222_0000_u64;
        let ofm = 0x3333_0000_u64;
        let ctrl_p = 0x4444_0000_u64;

        let over = patch_aiebu_elf_ctrltext_with_bos_ex(
            &elf,
            ifm,
            wts,
            ofm,
            ctrl_p,
            LabRelaMode::Overwrite,
        )
        .expect("overwrite multi-sym");
        let read = |b: &[u8], o: usize| u64::from_le_bytes(b[o..o + 8].try_into().unwrap());
        assert_eq!(read(&over, 0), ifm.wrapping_add(0));
        assert_eq!(read(&over, 16), wts.wrapping_add(10));
        assert_eq!(read(&over, 32), ofm.wrapping_add(20));
        assert_eq!(read(&over, 48), ctrl_p.wrapping_add(30));
        // Default wrapper = overwrite.
        let def = patch_aiebu_elf_ctrltext_with_bos(&elf, ifm, wts, ofm, ctrl_p).expect("default");
        assert_eq!(def, over);

        let add =
            patch_aiebu_elf_ctrltext_with_bos_ex(&elf, ifm, wts, ofm, ctrl_p, LabRelaMode::Add)
                .expect("add multi-sym");
        assert_eq!(read(&add, 0), 0xb1_u64.wrapping_add(ifm));
        assert_eq!(read(&add, 16), 0x1e_u64.wrapping_add(wts).wrapping_add(10));
        assert_eq!(read(&add, 32), 0x159_u64.wrapping_add(ofm).wrapping_add(20));
        assert_eq!(
            read(&add, 48),
            0x42_u64.wrapping_add(ctrl_p).wrapping_add(30)
        );
    }

    /// Pure-direct: negative ELF addend wraps as u64 (XRT-style address math).
    /// Not vendor post-patch / not silicon Pass.
    #[test]
    fn patch_aiebu_synthetic_negative_addend_wrapping_pure_direct() {
        let mut ctrl = vec![0_u8; 16];
        ctrl[0..8].copy_from_slice(&0_u64.to_le_bytes());
        let elf = crate::backends::amd_xdna2::silicon_contract::build_minimal_test_elf32(
            &ctrl,
            &[(0, (1 << 8) | 5, -8)],
        );
        let ifm = 100_u64;
        let over = patch_aiebu_elf_ctrltext_with_bos_ex(&elf, ifm, 0, 0, 0, LabRelaMode::Overwrite)
            .expect("neg addend overwrite");
        #[allow(clippy::cast_sign_loss)]
        let expect = ifm.wrapping_add((-8_i32) as u64);
        let v = u64::from_le_bytes(over[0..8].try_into().unwrap());
        assert_eq!(v, expect);
        // RMW: placeholder 7 + base + wrapped addend.
        ctrl[0..8].copy_from_slice(&7_u64.to_le_bytes());
        let elf2 = crate::backends::amd_xdna2::silicon_contract::build_minimal_test_elf32(
            &ctrl,
            &[(0, (1 << 8) | 5, -8)],
        );
        let add = patch_aiebu_elf_ctrltext_with_bos_ex(&elf2, ifm, 0, 0, 0, LabRelaMode::Add)
            .expect("neg addend add");
        let va = u64::from_le_bytes(add[0..8].try_into().unwrap());
        assert_eq!(va, 7_u64.wrapping_add(expect));
    }

    /// Pure-direct golden: staged ELF (if present) → synthetic paddrs → classic walk.
    ///
    /// Also locks unpatched CLASS A body parity when vendor fixtures exist.
    /// **Not** vendor true post-patch / **not** C-ready / **not** `ran_on_npu`.
    #[test]
    fn pure_direct_patch_staged_elf_walks_classic_not_vendor_post_patch() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/xdna2_cmd");
        let elf_path = root.join("silicon_lab/lab_instr.elf");
        let lab_bin = root.join("silicon_lab/lab_instr.bin");
        let vendor_txn = root.join("gt_capture/instr_vendor_txn_bin.bin");
        let gt_raw = root.join("gt_capture/instr_raw_txn.bin");

        // CLASS A body match (unpatched classic): lab raw == vendor txn == gt raw.
        if lab_bin.is_file() && vendor_txn.is_file() {
            let a = std::fs::read(&lab_bin).expect("lab bin");
            let b = std::fs::read(&vendor_txn).expect("vendor txn");
            assert_eq!(
                a, b,
                "CLASS A unpatched body: lab_instr.bin == instr_vendor_txn_bin (≠ C-ready)"
            );
            assert_eq!(a.len(), 1688);
        }
        if lab_bin.is_file() && gt_raw.is_file() {
            let a = std::fs::read(&lab_bin).expect("lab bin");
            let c = std::fs::read(&gt_raw).expect("gt raw");
            assert_eq!(
                a, c,
                "pure raw GT == lab classic body (unpatched parity; not C-ready)"
            );
        }

        if !elf_path.is_file() {
            eprintln!("skip: no lab_instr.elf (staged asset) for pure-direct golden");
            return;
        }
        let elf = std::fs::read(&elf_path).expect("elf");
        assert!(is_aiebu_elf_ctrltext(&elf));
        let relas = parse_elf32_rela_dyn(&elf).expect("rela");
        assert_eq!(relas.len(), 12, "staged mladf rela×12");

        // Unpatched peel must already walk as staged classic (holes don't break op stream).
        let (coff, clen) = match classify_instruction_blob(&elf) {
            InstructionBlobKind::Aie2ElfCtrltext {
                ctrltext_off,
                ctrltext_len,
                ..
            } => (ctrltext_off, ctrltext_len),
            other => panic!("need aiebu ELF, got {}", other.summary()),
        };
        let unpatched = &elf[coff..coff + clen];
        validate_patched_ctrltext_classic(unpatched, true).expect("unpatched peel walks classic");

        // Note honesty: DD classic raw may differ from ELF ctrltext holes (addend-in-high
        // dword vs pure placeholder). Pure-direct patch is ELF-relative, not raw-bin rewrite.
        if lab_bin.is_file() {
            let raw = std::fs::read(&lab_bin).expect("raw");
            if raw.as_slice() != unpatched {
                // Expected on this host: 22 hole-related bytes differ — not a test failure.
                assert_eq!(raw.len(), unpatched.len());
            }
        }

        let ifm = 0xa11f_0000_1000_u64;
        let wts = 0xa11f_0000_2000_u64;
        let ofm = 0xa11f_0000_3000_u64;
        let ctrl = 0xa11f_0000_4000_u64;
        let patched = patch_aiebu_elf_ctrltext_with_bos(&elf, ifm, wts, ofm, ctrl)
            .expect("pure-direct patch");
        assert_eq!(patched.len(), 1688);
        validate_patched_ctrltext_classic(&patched, true)
            .expect("pure-direct overwrite walks classic 1688/40");
        let bases = [0_u64, ifm, wts, ofm, ctrl];
        for r in &relas {
            let base = bases.get(r.sym() as usize).copied().unwrap_or(0);
            #[allow(clippy::cast_sign_loss)]
            let expect = base.wrapping_add(r.addend as u64);
            let o = r.offset as usize;
            let v = u64::from_le_bytes(patched[o..o + 8].try_into().unwrap());
            assert_eq!(v, expect, "pure-direct overwrite site off={o:#x}");
        }

        let patched_add =
            patch_aiebu_elf_ctrltext_with_bos_ex(&elf, ifm, wts, ofm, ctrl, LabRelaMode::Add)
                .expect("pure-direct add");
        validate_patched_ctrltext_classic(&patched_add, true)
            .expect("pure-direct add still walks classic");
        // Add ≠ overwrite at first IFM hole (placeholder 0xb1).
        let v_over = u64::from_le_bytes(patched[0x2c..0x2c + 8].try_into().unwrap());
        let v_add = u64::from_le_bytes(patched_add[0x2c..0x2c + 8].try_into().unwrap());
        assert_eq!(v_over, ifm);
        assert_eq!(v_add, ifm.wrapping_add(0xb1));
        assert_ne!(v_over, v_add, "add mode must RMW placeholder");
    }

    #[test]
    fn lab_rela_mode_token_table_fail_closed() {
        assert_eq!(
            try_lab_rela_mode_from_str(None).unwrap(),
            LabRelaMode::Overwrite
        );
        assert_eq!(
            try_lab_rela_mode_from_str(Some("overwrite")).unwrap(),
            LabRelaMode::Overwrite
        );
        assert_eq!(
            try_lab_rela_mode_from_str(Some("ADD")).unwrap(),
            LabRelaMode::Add
        );
        assert_eq!(
            try_lab_rela_mode_from_str(Some("rmw")).unwrap(),
            LabRelaMode::Add
        );
        assert!(try_lab_rela_mode_from_str(Some("bogus")).is_err());
        assert!(try_lab_rela_mode_from_str(Some("rmw_add")).is_err());
    }

    #[test]
    fn validate_patched_ctrltext_classic_refuse_and_relax() {
        let noop = build_strix_research_noop_txn().expect("noop");
        // Relax path: walkable classic, not staged 1688/40.
        validate_patched_ctrltext_classic(&noop, false).expect("relax ok");
        let err = validate_patched_ctrltext_classic(&noop, true).expect_err("staged size");
        assert!(
            err.to_string().contains("1688") || err.to_string().contains("expected"),
            "{err}"
        );
        // Garbage
        let g = validate_patched_ctrltext_classic(&[0xde, 0xad], false).expect_err("garbage");
        assert_ne!(g.to_string(), "");
        // Truncate classic mid-stream
        let mut short = noop;
        short.truncate(20);
        assert!(validate_patched_ctrltext_classic(&short, false).is_err());
    }

    #[test]
    fn parse_dynsym_staged_names_are_opaque_six() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("testdata/xdna2_cmd/silicon_lab/lab_instr.elf");
        if !path.is_file() {
            eprintln!("skip: no lab_instr.elf (staged asset)");
            return;
        }
        let elf = std::fs::read(&path).expect("read elf");
        let syms = parse_elf32_dynsym(&elf).expect("dynsym");
        assert_eq!(syms.len(), 4, "null + 3 buffer symbols");
        assert_eq!(syms[0].name, "");
        assert_eq!(syms[1].name, "6");
        assert_eq!(syms[2].name, "6");
        assert_eq!(syms[3].name, "6");
        assert_eq!(syms[1].size, 708);
        assert_eq!(syms[2].size, 120);
        assert_eq!(syms[3].size, 1380);
        let report = format_dynsym_report(&syms);
        assert!(report.contains("names_all_\"6\""));
        assert!(report.contains("dynsym_n=4"));
        assert!(report.contains("708") && report.contains("120") && report.contains("1380"));
    }

    #[test]
    fn format_dynsym_report_empty_and_opaque() {
        assert_eq!(format_dynsym_report(&[]), "dynsym empty");
        let syms = vec![
            Elf32DynSym {
                index: 0,
                name: String::new(),
                value: 0,
                size: 0,
                info: 0,
                other: 0,
                shndx: 0,
            },
            Elf32DynSym {
                index: 1,
                name: "6".into(),
                value: 0,
                size: 10,
                info: 0x11,
                other: 0,
                shndx: 3,
            },
        ];
        let r = format_dynsym_report(&syms);
        assert!(r.contains("dynsym_n=2"));
        assert!(r.contains("names_all_\"6\""));
    }

    #[test]
    fn parse_dynsym_non_elf_returns_empty_ok() {
        let empty = parse_elf32_dynsym(&[]).expect("empty");
        assert_eq!(empty, [] as [Elf32DynSym; 0]);
        let garbage = parse_elf32_dynsym(&[0xde, 0xad, 0xbe, 0xef]).expect("garbage");
        assert_eq!(garbage, [] as [Elf32DynSym; 0]);
    }

    #[test]
    fn header_roundtrip() {
        let h = XaieTxnHeader {
            major: 0,
            minor: 1,
            dev_gen: 4,
            num_rows: 6,
            num_cols: 4,
            num_mem_tile_rows: 1,
            num_ops: 3,
            txn_size: 28,
        };
        let b = h.to_bytes();
        let p = XaieTxnHeader::parse(&b).expect("parse");
        assert_eq!(p, h);
        assert!(p.is_classic_v01());
    }

    #[test]
    fn minimal_noop_txn_classifies_classic() {
        let txn = build_strix_research_noop_txn().expect("build");
        assert_eq!(txn.len(), 16 + 4 * 4);
        let kind = classify_instruction_blob(&txn);
        match kind {
            InstructionBlobKind::ClassicTxnV01 { header, ops } => {
                assert_eq!(header.num_ops, 4);
                assert_eq!(ops.len(), 4);
                assert!(ops.iter().all(|o| o.opcode == XaieTxnOpcode::Noop as u8));
                assert_eq!(header.num_cols, 4);
                assert_eq!(header.dev_gen, 4);
            }
            other => panic!("expected classic: {}", other.summary()),
        }
        validate_txn_blob(&txn).expect("validate");
    }

    #[test]
    fn synthetic_write_mask_block_custom_walk() {
        // Hand-built classic stream: 1 WRITE + 1 MASKWRITE + 1 BLOCKWRITE + 1 CUSTOM + 1 MASKPOLL
        // MASK* uses lab/DD padded size=32.
        let mut body = Vec::new();
        // WRITE 24
        let mut write_pkt = vec![0_u8; 24];
        write_pkt[0] = 0;
        write_pkt[1] = 1; // col
        // reg @ +8 = 0x1b0000
        write_pkt[8..16].copy_from_slice(&0x001b_0000_u64.to_le_bytes());
        write_pkt[16..20].copy_from_slice(&0x8000_0007_u32.to_le_bytes());
        write_pkt[20..24].copy_from_slice(&24_u32.to_le_bytes());
        body.extend_from_slice(&write_pkt);
        // MASKWRITE 32 (padded lab encoding)
        let mut mask_pkt = vec![0_u8; 32];
        mask_pkt[0] = 3;
        mask_pkt[8..16].copy_from_slice(&0x1f000_u64.to_le_bytes());
        mask_pkt[16..20].copy_from_slice(&0x400_u32.to_le_bytes());
        mask_pkt[20..24].copy_from_slice(&0xc00_u32.to_le_bytes());
        mask_pkt[24..28].copy_from_slice(&32_u32.to_le_bytes());
        body.extend_from_slice(&mask_pkt);
        // BLOCKWRITE total 24: hdr16 + 8 payload
        let mut block_pkt = vec![0_u8; 24];
        block_pkt[0] = 1;
        block_pkt[8..12].copy_from_slice(&0x1d000_u32.to_le_bytes());
        block_pkt[12..16].copy_from_slice(&24_u32.to_le_bytes());
        body.extend_from_slice(&block_pkt);
        // CUSTOM DDR_PATCH-like total 16
        let mut custom_pkt = vec![0_u8; 16];
        custom_pkt[0] = 129;
        custom_pkt[4..8].copy_from_slice(&16_u32.to_le_bytes());
        body.extend_from_slice(&custom_pkt);
        // MASKPOLL 32 (padded)
        let mut poll_pkt = vec![0_u8; 32];
        poll_pkt[0] = 4;
        poll_pkt[8..16].copy_from_slice(&0x1d224_u64.to_le_bytes());
        poll_pkt[24..28].copy_from_slice(&32_u32.to_le_bytes());
        body.extend_from_slice(&poll_pkt);

        let total = 16 + body.len();
        let header = XaieTxnHeader {
            major: 0,
            minor: 1,
            dev_gen: 4,
            num_rows: 6,
            num_cols: 4,
            num_mem_tile_rows: 1,
            num_ops: 5,
            txn_size: u32::try_from(total).expect("value fits in u32"),
        };
        let mut bytes = header.to_bytes().to_vec();
        bytes.extend_from_slice(&body);
        let kind = classify_instruction_blob(&bytes);
        match kind {
            InstructionBlobKind::ClassicTxnV01 { ops, .. } => {
                assert_eq!(ops.len(), 5);
                assert_eq!(ops[0].opcode, 0);
                assert_eq!(ops[1].opcode, 3);
                assert_eq!(ops[1].byte_len, 32);
                assert_eq!(ops[2].opcode, 1);
                assert_eq!(ops[3].opcode, 129);
                assert_eq!(ops[4].opcode, 4);
                assert_eq!(ops[4].byte_len, 32);
            }
            other => panic!("{}", other.summary()),
        }
    }

    /// mlir-aie public AOT MASK* uses compact size=28 (no 4-byte pad after size).
    ///
    /// Walk must accept both 28 and 32; invent_pass=0; not residual match to lab 1688.
    #[test]
    fn synthetic_mask_compact_28_walk() {
        let mut body = Vec::new();
        // WRITE 24
        let mut w = vec![0_u8; 24];
        w[0] = 0;
        w[20..24].copy_from_slice(&24_u32.to_le_bytes());
        body.extend_from_slice(&w);
        // MASKWRITE compact 28 (public mlir-aie layout)
        let mut m = vec![0_u8; 28];
        m[0] = 3;
        m[8..16].copy_from_slice(&0x1d200_u64.to_le_bytes());
        m[16..20].copy_from_slice(&0xf_u32.to_le_bytes());
        m[20..24].copy_from_slice(&0x1f_u32.to_le_bytes());
        m[24..28].copy_from_slice(&28_u32.to_le_bytes());
        body.extend_from_slice(&m);
        // MASKPOLL compact 28
        let mut p = vec![0_u8; 28];
        p[0] = 4;
        p[8..16].copy_from_slice(&0x1d224_u64.to_le_bytes());
        p[24..28].copy_from_slice(&28_u32.to_le_bytes());
        body.extend_from_slice(&p);

        let total = 16 + body.len();
        let header = XaieTxnHeader {
            major: 0,
            minor: 1,
            dev_gen: 4,
            num_rows: 6,
            num_cols: 8,
            num_mem_tile_rows: 1,
            num_ops: 3,
            txn_size: u32::try_from(total).expect("value fits in u32"),
        };
        let mut bytes = header.to_bytes().to_vec();
        bytes.extend_from_slice(&body);
        let kind = classify_instruction_blob(&bytes);
        match kind {
            InstructionBlobKind::ClassicTxnV01 { ops, header: h } => {
                assert_eq!(ops.len(), 3);
                assert_eq!(h.num_ops, 3);
                assert_eq!(ops[0].opcode, 0);
                assert_eq!(ops[0].byte_len, 24);
                assert_eq!(ops[1].opcode, 3);
                assert_eq!(ops[1].byte_len, 28);
                assert_eq!(ops[2].opcode, 4);
                assert_eq!(ops[2].byte_len, 28);
            }
            other => panic!("expected ClassicTxnV01 compact MASK*: {}", other.summary()),
        }
        validate_txn_blob(&bytes).expect("compact MASK* stream validates");
    }

    /// Unknown MASK* size (not 28/32) still fails closed as TxnHeaderOnly.
    #[test]
    fn mask_unknown_size_is_header_only() {
        let mut body = Vec::new();
        let mut m = vec![0_u8; 28];
        m[0] = 3;
        m[24..28].copy_from_slice(&20_u32.to_le_bytes()); // too small / unknown
        body.extend_from_slice(&m);
        let total = 16 + body.len();
        let header = XaieTxnHeader {
            major: 0,
            minor: 1,
            dev_gen: 4,
            num_rows: 6,
            num_cols: 4,
            num_mem_tile_rows: 1,
            num_ops: 1,
            txn_size: u32::try_from(total).expect("value fits in u32"),
        };
        let mut bytes = header.to_bytes().to_vec();
        bytes.extend_from_slice(&body);
        match classify_instruction_blob(&bytes) {
            InstructionBlobKind::TxnHeaderOnly { walk_error, .. } => {
                assert!(
                    walk_error.contains("MASK*") && walk_error.contains("20"),
                    "walk_error={walk_error}"
                );
            }
            other => panic!("expected TxnHeaderOnly, got {}", other.summary()),
        }
    }

    #[test]
    fn garbage_is_unknown_raw() {
        let kind = classify_instruction_blob(&[0xde, 0xad, 0xbe, 0xef]);
        assert!(matches!(kind, InstructionBlobKind::UnknownRaw { len: 4 }));
        assert!(validate_txn_blob(&[1, 2, 3]).is_err());
    }

    #[test]
    fn classify_raw_classic_vs_not_aiebu_elf() {
        let txn = build_strix_research_noop_txn().expect("noop");
        assert!(matches!(
            classify_instruction_blob(&txn),
            InstructionBlobKind::ClassicTxnV01 { .. }
        ));
        assert!(!is_aiebu_elf_ctrltext(&txn));
        // Truncated ELF magic alone is not ctrltext-bearing aiebu.
        let fake_elf = b"\x7fELF\x01\x01";
        assert!(!is_aiebu_elf_ctrltext(fake_elf));
    }

    /// Synthetic staged classic header profile vs multi wrong-body (HITL research).
    ///
    /// Labels only our pure classifier — not vendor post-patch / not C-ready.
    #[test]
    fn classic_1688_vs_multi_32208_header_profiles() {
        // Staged classic: maj/min/dev/rows/cols/mem + num_ops=40 txn_size=1688
        let mut classic = vec![0_u8; STAGED_MLADF_CLASSIC_INSTR_BYTES];
        classic[0] = 0;
        classic[1] = 1;
        classic[2] = 4;
        classic[3] = 6;
        classic[4] = 8;
        classic[5] = 1;
        classic[8..12].copy_from_slice(&STAGED_MLADF_CLASSIC_NUM_OPS.to_le_bytes());
        classic[12..16].copy_from_slice(
            &u32::try_from(STAGED_MLADF_CLASSIC_INSTR_BYTES)
                .expect("value fits in u32")
                .to_le_bytes(),
        );
        // Non-zero body so not all-zero (header already non-zero).
        classic[16] = 0x83;

        assert!(is_staged_mladf_classic_instr_profile(&classic));
        assert!(!is_hitl_mladf_multi_txn_wrong_body_profile(&classic));
        let ch = XaieTxnHeader::parse(&classic).expect("classic hdr");
        assert!(ch.is_staged_mladf_classic_profile());
        assert!(!ch.is_hitl_mladf_multi_txn_profile());

        // Multi wrong-body: same maj/min layout, 500 ops / 32208 B (HITL dump).
        let mut multi = vec![0_u8; HITL_MLADF_GET_TRANSACTION_BIN_BYTES];
        multi[0] = 0;
        multi[1] = 1;
        multi[2] = 4;
        multi[3] = 6;
        multi[4] = 8;
        multi[5] = 1;
        multi[8..12].copy_from_slice(&HITL_MLADF_GET_TRANSACTION_BIN_NUM_OPS.to_le_bytes());
        multi[12..16].copy_from_slice(
            &u32::try_from(HITL_MLADF_GET_TRANSACTION_BIN_BYTES)
                .expect("value fits in u32")
                .to_le_bytes(),
        );
        multi[16] = 0xaa;

        assert!(is_hitl_mladf_multi_txn_wrong_body_profile(&multi));
        assert!(!is_staged_mladf_classic_instr_profile(&multi));
        // Prefix of multi (first 1688) is NOT staged classic profile — refuse silent
        // CLASS_HINT C from multi-txn head (txn_size field still 32208).
        let multi_prefix = &multi[..STAGED_MLADF_CLASSIC_INSTR_BYTES];
        assert!(
            !is_staged_mladf_classic_instr_profile(multi_prefix),
            "multi[..1688] must not look like staged classic (txn_size≠1688)"
        );
        let mh = XaieTxnHeader::parse(multi_prefix).expect("multi prefix hdr");
        assert_eq!(mh.num_ops, HITL_MLADF_GET_TRANSACTION_BIN_NUM_OPS);
        assert_eq!(mh.txn_size as usize, HITL_MLADF_GET_TRANSACTION_BIN_BYTES);
        assert!(!mh.is_staged_mladf_classic_profile());
        // Not a container of classic 1688 — peel is impossible (research negative).
        assert!(!txn_bytes_embed_slice(&multi, &classic));
        // Not ELF — aiebu peel N/A.
        assert!(!is_aiebu_elf_ctrltext(&multi));
        // Both use classic maj/min layout (0/1) — distinction is size/opcount only.
        assert_eq!(mh.major, 0);
        assert_eq!(mh.minor, 1);
        assert_ne!(
            STAGED_MLADF_CLASSIC_NUM_OPS, HITL_MLADF_GET_TRANSACTION_BIN_NUM_OPS,
            "wrong-body must differ in opcount so CLASS_HINT C cannot fire on prefix"
        );
    }

    #[test]
    fn live_gt_capture_classic_vs_stale_multi_if_present() {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/xdna2_cmd/gt_capture");
        let classic_path = base.join("instr_raw_txn.bin");
        if classic_path.is_file() {
            let classic = std::fs::read(&classic_path).expect("raw");
            assert_eq!(classic.len(), STAGED_MLADF_CLASSIC_INSTR_BYTES);
            assert!(
                is_staged_mladf_classic_instr_profile(&classic),
                "gt classic should match staged profile"
            );
        }
        // Stale multi post_patch may still be on disk from older HITL; classify only.
        let multi_path = base.join("instr_vendor_post_patch.bin");
        if multi_path.is_file() {
            let multi = std::fs::read(&multi_path).expect("vendor pp");
            if multi.len() == HITL_MLADF_GET_TRANSACTION_BIN_BYTES {
                assert!(is_hitl_mladf_multi_txn_wrong_body_profile(&multi));
                if classic_path.is_file() {
                    let classic = std::fs::read(&classic_path).expect("raw2");
                    assert!(
                        !txn_bytes_embed_slice(&multi, &classic),
                        "multi must not embed classic 1688 (peel impossible)"
                    );
                }
                assert!(!is_aiebu_elf_ctrltext(&multi));
            }
        }
    }

    #[test]
    fn classify_staged_lab_raw_bin_if_present() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("testdata/xdna2_cmd/silicon_lab/lab_instr.bin");
        if !path.is_file() {
            return;
        }
        let bytes = std::fs::read(&path).expect("read bin");
        assert!(!is_aiebu_elf_ctrltext(&bytes));
        let kind = classify_instruction_blob(&bytes);
        assert!(
            kind.looks_like_txn(),
            "staged lab_instr.bin should be classic txn: {}",
            kind.summary()
        );
    }

    #[test]
    fn header_size_mismatch_unknown() {
        let mut txn = build_minimal_noop_txn(1, 4, 6, 1, 4).expect("build");
        // corrupt TxnSize
        txn[12..16].copy_from_slice(&999_u32.to_le_bytes());
        let kind = classify_instruction_blob(&txn);
        assert!(matches!(kind, InstructionBlobKind::UnknownRaw { .. }));
    }

    #[test]
    fn instruction_payload_passthrough_raw() {
        let txn = build_strix_research_noop_txn().expect("build");
        let out = instruction_payload_bytes(&txn).expect("payload");
        assert_eq!(out, txn);
    }

    #[test]
    fn looks_like_txn_true_for_classic() {
        let txn = build_strix_research_noop_txn().expect("build");
        assert!(classify_instruction_blob(&txn).looks_like_txn());
    }

    #[test]
    fn optional_lab_matmul_blob_classifies_if_present() {
        // Gitignored lab extract from tools/dump_dd_txn — skip when absent (CI).
        let candidates = [
            "testdata/xdna2_cmd/lab_instr.bin",
            "testdata/xdna2_cmd/dd_matmul_txn/mladfmatmulbias_prmptflat_mladf_2x4x4_flat_a16fw4acc16f_1_1152_256_128.bin",
        ];
        let path = candidates.iter().map(Path::new).find(|p| p.is_file());
        let Some(path) = path else {
            return;
        };
        let bytes = std::fs::read(path).expect("read lab blob");
        if bytes.len() < 64 {
            return; // synthetic NOOP or empty research stub
        }
        let kind = classify_instruction_blob(&bytes);
        assert!(
            kind.looks_like_txn(),
            "lab matmul blob should look like XAie txn: {}",
            kind.summary()
        );
        if let InstructionBlobKind::ClassicTxnV01 { header, ops } = &kind {
            assert!(header.dev_gen >= 3, "dev_gen={}", header.dev_gen);
            assert_ne!(ops.as_slice(), []);
        }
    }

    /// Offline research: public mlir-aie cascade matmul npu2 AOT (when landed).
    ///
    /// Soft-skip when capture dir empty. Classifies as classic v0.1 START_NPU
    /// txn stream (not residual match / not Pass). invent_pass=0 surface only.
    #[test]
    fn optional_cascade_public_aot_classifies_if_present() {
        let root =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/xdna2_cmd/external_known_good");
        let ctrl_path = root.join("mlir_aie_cascade_matmul_npu2.ctrltext.bin");
        let insts_path = root.join("mlir_aie_cascade_matmul_npu2.insts.bin");
        let elf_path = root.join("mlir_aie_cascade_matmul_npu2.insts.elf");
        if !ctrl_path.is_file() {
            return;
        }
        let ctrl = std::fs::read(&ctrl_path).expect("read cascade ctrltext");
        assert_eq!(ctrl.len(), 4512, "cascade ctrltext size");
        // Distinct from insts.bin sha (aiebu may re-layout) but same size class.
        assert_eq!(std::fs::metadata(&insts_path).map_or(0, |m| m.len()), 4512);
        let kind = classify_instruction_blob(&ctrl);
        eprintln!("cascade_ctrltext classify: {}", kind.summary());
        match &kind {
            InstructionBlobKind::ClassicTxnV01 { header, ops } => {
                assert!(header.is_classic_v01(), "major/minor classic");
                assert_eq!(header.txn_size as usize, 4512);
                assert_eq!(header.num_ops as usize, ops.len());
                assert_eq!(header.num_ops, 116);
                assert_eq!(header.dev_gen, 4);
                // aiebu-asm report: cols/rows 8×6 (array+shim geometry)
                assert_eq!(header.num_cols, 8);
                assert_eq!(header.num_rows, 6);
            }
            other => panic!(
                "cascade ctrltext expected ClassicTxnV01, got {}",
                other.summary()
            ),
        }
        validate_txn_blob(&ctrl).expect("cascade ctrltext validates as txn");
        // Not lab classic 1688 profile.
        assert!(!is_staged_mladf_classic_instr_profile(&ctrl));

        if elf_path.is_file() {
            let elf = std::fs::read(&elf_path).expect("read cascade elf");
            let ekind = classify_instruction_blob(&elf);
            eprintln!("cascade_insts_elf classify: {}", ekind.summary());
            match ekind {
                InstructionBlobKind::Aie2ElfCtrltext {
                    ctrltext_len,
                    ctrltext,
                    ..
                } => {
                    assert_eq!(ctrltext_len, 4512);
                    assert!(
                        matches!(*ctrltext, InstructionBlobKind::ClassicTxnV01 { .. }),
                        "inner ctrltext classic: {}",
                        ctrltext.summary()
                    );
                }
                other => panic!("cascade elf expected Aie2ElfCtrltext: {}", other.summary()),
            }
        }
    }
}
