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

/// Classic (v0.1) WRITE op size including trailing size field.
const OP_WRITE_V01_SIZE: usize = 24;
/// Classic MASKWRITE / MASKPOLL / MASKPOLL_BUSY size.
const OP_MASK_V01_SIZE: usize = 32;
/// NOOP / PREEMPT fixed size.
const OP_SHORT_SIZE: usize = 4;

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
            // MASKWRITE / MASKPOLL / MASKPOLL_BUSY: 32 bytes (size at +24)
            if off + OP_MASK_V01_SIZE > bytes.len() {
                return Err(DirectError::InvalidArgument(
                    "MASK* op truncated".to_string(),
                ));
            }
            let size = u32::from_le_bytes(bytes[off + 24..off + 28].try_into().unwrap_or([0; 4]));
            if size as usize != OP_MASK_V01_SIZE {
                return Err(DirectError::InvalidArgument(format!(
                    "MASK* size field {size} != {OP_MASK_V01_SIZE}"
                )));
            }
            Ok(OP_MASK_V01_SIZE)
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

/// Parse ELF32 little-endian `.rela.dyn` entries (empty if missing).
///
/// # Errors
///
/// Truncated section table / rela table.
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
/// XRT/shim (`txn_file2elf` + `patch_ctrl_code`) does this before EXEC. Submitting
/// **raw** DD txn (or unpatched ctrltext) yields FW `APP_LOAD_PDI_FAIL` (0x3000003)
/// on live silicon (2026-07-16).
///
/// Symbol index mapping (aiebu dynsym 1..n): `1→ifm`, `2→wts`, `3→ofm`, `4→ctrl`
/// (extra symbols ignored). Reloc type is treated as **u64 LE** store of
/// `base.wrapping_add(addend as u64)` at `ctrltext[offset]`.
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
        let value = base.wrapping_add(r.addend as u64);
        let o = r.offset as usize;
        if o.checked_add(8).is_none_or(|end| end > ctrltext.len()) {
            return Err(DirectError::InvalidArgument(format!(
                "patch_aiebu_elf_ctrltext: reloc offset {o}+8 OOB (ctrltext {})",
                ctrltext.len()
            )));
        }
        // Type 5 observed on aiebu aie2txn output — always write u64 LE.
        ctrltext[o..o + 8].copy_from_slice(&value.to_le_bytes());
    }
    Ok(ctrltext)
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
        let patched =
            patch_aiebu_elf_ctrltext_with_bos(&elf, ifm, wts, ofm, ctrl).expect("patch");
        assert_eq!(patched.len(), 0x698);
        let bases = [0_u64, ifm, wts, ofm, ctrl];
        for r in &relas {
            let base = bases.get(r.sym() as usize).copied().unwrap_or(0);
            let expect = base.wrapping_add(r.addend as u64);
            let o = r.offset as usize;
            let v = u64::from_le_bytes(patched[o..o + 8].try_into().unwrap());
            assert_eq!(v, expect, "rela off={o:#x} sym={} addend={}", r.sym(), r.addend);
        }
        // First reloc at 0x2c sym1 + 0 → ifm
        let v = u64::from_le_bytes(patched[0x2c..0x2c + 8].try_into().unwrap());
        assert_eq!(v, ifm);
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
        let mut body = Vec::new();
        // WRITE 24
        let mut w = vec![0_u8; 24];
        w[0] = 0;
        w[1] = 1; // col
        // reg @ +8 = 0x1b0000
        w[8..16].copy_from_slice(&0x1b0000_u64.to_le_bytes());
        w[16..20].copy_from_slice(&0x8000_0007_u32.to_le_bytes());
        w[20..24].copy_from_slice(&24_u32.to_le_bytes());
        body.extend_from_slice(&w);
        // MASKWRITE 32
        let mut m = vec![0_u8; 32];
        m[0] = 3;
        m[8..16].copy_from_slice(&0x1f000_u64.to_le_bytes());
        m[16..20].copy_from_slice(&0x400_u32.to_le_bytes());
        m[20..24].copy_from_slice(&0xc00_u32.to_le_bytes());
        m[24..28].copy_from_slice(&32_u32.to_le_bytes());
        body.extend_from_slice(&m);
        // BLOCKWRITE total 24: hdr16 + 8 payload
        let mut b = vec![0_u8; 24];
        b[0] = 1;
        b[8..12].copy_from_slice(&0x1d000_u32.to_le_bytes());
        b[12..16].copy_from_slice(&24_u32.to_le_bytes());
        body.extend_from_slice(&b);
        // CUSTOM DDR_PATCH-like total 16
        let mut c = vec![0_u8; 16];
        c[0] = 129;
        c[4..8].copy_from_slice(&16_u32.to_le_bytes());
        body.extend_from_slice(&c);
        // MASKPOLL 32
        let mut p = vec![0_u8; 32];
        p[0] = 4;
        p[8..16].copy_from_slice(&0x1d224_u64.to_le_bytes());
        p[24..28].copy_from_slice(&32_u32.to_le_bytes());
        body.extend_from_slice(&p);

        let total = 16 + body.len();
        let header = XaieTxnHeader {
            major: 0,
            minor: 1,
            dev_gen: 4,
            num_rows: 6,
            num_cols: 4,
            num_mem_tile_rows: 1,
            num_ops: 5,
            txn_size: total as u32,
        };
        let mut bytes = header.to_bytes().to_vec();
        bytes.extend_from_slice(&body);
        let kind = classify_instruction_blob(&bytes);
        match kind {
            InstructionBlobKind::ClassicTxnV01 { ops, .. } => {
                assert_eq!(ops.len(), 5);
                assert_eq!(ops[0].opcode, 0);
                assert_eq!(ops[1].opcode, 3);
                assert_eq!(ops[2].opcode, 1);
                assert_eq!(ops[3].opcode, 129);
                assert_eq!(ops[4].opcode, 4);
            }
            other => panic!("{}", other.summary()),
        }
    }

    #[test]
    fn garbage_is_unknown_raw() {
        let kind = classify_instruction_blob(&[0xde, 0xad, 0xbe, 0xef]);
        assert!(matches!(kind, InstructionBlobKind::UnknownRaw { len: 4 }));
        assert!(validate_txn_blob(&[1, 2, 3]).is_err());
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
            assert!(!ops.is_empty());
        }
    }
}
