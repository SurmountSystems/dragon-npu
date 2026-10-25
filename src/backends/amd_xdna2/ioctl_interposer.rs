//! Ioctl interposer record format.
//!
//! Offline only. This module does not open `/dev/accel`, does not issue an
//! ioctl, and does not set `XDNA2_ALLOW_EXEC_CMD`. A green replay of a record
//! is not product Pass and is not `ran_on_npu`.
//!
//! A record stores the ioctl request word, the `_IOC_SIZE` struct bytes, the
//! command-buffer bytes, and the instruction-buffer bytes. A pointer-only
//! field is representable so a replay can reject it. The dump path never
//! writes a pointer in place of those bytes.
//!
//! The numeric request `0xc03064ca` is not named here. The installed
//! `amdxdna_accel.h` does not contain that token.

use std::convert::TryFrom;

/// Magic at the start of an interposer record (`DNPI`).
const RECORD_MAGIC: &[u8; 4] = b"DNPI";
/// Record version this decoder accepts.
const RECORD_VERSION: u32 = 1;
/// The field was omitted.
const TAG_MISSING: u8 = 0;
/// The capture stored an address and did not copy the bytes.
const TAG_POINTER_ONLY: u8 = 1;
/// The capture stored the bytes.
const TAG_BYTES: u8 = 2;

/// Linux `_IOC_SIZE` bit shift (`_IOC_SIZESHIFT` in `asm-generic/ioctl.h`).
const IOC_SIZE_SHIFT: u32 = 16;
/// Linux `_IOC_SIZE` mask, 14 bits.
const IOC_SIZE_MASK: u64 = (1 << 14) - 1;

/// Name of the ioctl request struct field. Not a pointer.
pub const FIELD_REQUEST_STRUCT: &str = "request_struct";
/// Name of the command-buffer field. Not a pointer.
pub const FIELD_COMMAND_BUFFER: &str = "command_buffer";
/// Name of the instruction-buffer field. Not a pointer.
pub const FIELD_INSTRUCTION_BUFFER: &str = "instruction_buffer";
/// Name of a followed buffer in a text preload dump. Not a pointer.
const FIELD_FOLLOWED: &str = "followed";

/// One captured field. `Bytes` is the copied contents. The other two forms
/// are refused by the decoder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapturedField {
    /// The capture omitted this field.
    Missing,
    /// The capture stored only this address.
    PointerOnly {
        /// Userspace address. Not the bytes at that address.
        address: u64,
    },
    /// Copied bytes.
    Bytes(Vec<u8>),
}

/// One ioctl capture before decode. Fields may still be pointers or missing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IoctlInterposerRecord {
    /// Numeric request word. Not a struct name.
    pub request: u64,
    /// `_IOC_SIZE` buffer, or a pointer, or missing.
    pub request_struct: CapturedField,
    /// Command-buffer contents, or a pointer, or missing.
    pub command_buffer: CapturedField,
    /// Instruction-buffer contents, or a pointer, or missing.
    pub instruction_buffer: CapturedField,
}

/// Bytes the decoder accepted. Every buffer is owned contents, not an address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedIoctlDump {
    /// Numeric request word from the record.
    pub request: u64,
    /// `_IOC_SIZE` struct bytes.
    pub request_struct: Vec<u8>,
    /// Command-buffer bytes.
    pub command_buffer: Vec<u8>,
    /// Instruction-buffer bytes.
    pub instruction_buffer: Vec<u8>,
}

/// Why a record cannot be replayed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum IoctlInterposerError {
    /// The byte string ended before a required width.
    #[error("ioctl interposer record is truncated")]
    Truncated,
    /// The leading four bytes were not `DNPI`.
    #[error("ioctl interposer record has a bad magic")]
    BadMagic,
    /// The version word is not the one this decoder reads.
    #[error("ioctl interposer record version {version} is not supported")]
    BadVersion {
        /// Version word from the record.
        version: u32,
    },
    /// A field tag was not missing, pointer-only, or bytes.
    #[error("ioctl interposer field tag {tag} is not a known tag")]
    BadTag {
        /// Tag byte.
        tag: u8,
    },
    /// Bytes followed the three fields.
    #[error("ioctl interposer record has trailing bytes")]
    TrailingBytes,
    /// A required field stored an address and not the bytes.
    #[error("ioctl interposer field {field} is only a pointer to {address:#x}")]
    PointerOnly {
        /// Which field.
        field: &'static str,
        /// Address that was stored instead of bytes.
        address: u64,
    },
    /// A required field was omitted or had no bytes.
    #[error("ioctl interposer field {field} is missing")]
    Missing {
        /// Which field.
        field: &'static str,
    },
    /// Struct bytes were present but their length was not `_IOC_SIZE`.
    #[error("ioctl request {request:#x} struct is {actual} bytes; _IOC_SIZE is {expected}")]
    StructSizeMismatch {
        /// Numeric request word.
        request: u64,
        /// `_IOC_SIZE` of that word.
        expected: usize,
        /// Length that was stored.
        actual: usize,
    },
}

/// Byte count in the Linux `_IOC_SIZE` bits of `request`.
///
/// This is bits 16..30. It is not a struct name. Zero means the request word
/// has no struct buffer.
#[must_use]
pub fn ioctl_request_struct_size(request: u64) -> usize {
    let masked = (request >> IOC_SIZE_SHIFT) & IOC_SIZE_MASK;
    usize::try_from(masked).unwrap_or(0)
}

/// Write a record, including a pointer-only field, so a replay can refuse it.
///
/// This does not open a device. Pointer-only is a captured shape, not a dump
/// of the bytes.
pub fn encode_ioctl_interposer_record(
    record: &IoctlInterposerRecord,
) -> Result<Vec<u8>, IoctlInterposerError> {
    let mut out = Vec::new();
    out.extend_from_slice(RECORD_MAGIC);
    push_u32(&mut out, RECORD_VERSION);
    push_u64(&mut out, record.request);
    push_field(&mut out, &record.request_struct)?;
    push_field(&mut out, &record.command_buffer)?;
    push_field(&mut out, &record.instruction_buffer)?;
    Ok(out)
}

/// Copy struct, command-buffer, and instruction-buffer bytes into a record.
///
/// Refuses an empty buffer. Does not store a pointer. Does not open a device.
pub fn dump_ioctl_capture(
    request: u64,
    request_struct: &[u8],
    command_buffer: &[u8],
    instruction_buffer: &[u8],
) -> Result<Vec<u8>, IoctlInterposerError> {
    if request_struct.is_empty() {
        return Err(IoctlInterposerError::Missing {
            field: FIELD_REQUEST_STRUCT,
        });
    }
    if command_buffer.is_empty() {
        return Err(IoctlInterposerError::Missing {
            field: FIELD_COMMAND_BUFFER,
        });
    }
    if instruction_buffer.is_empty() {
        return Err(IoctlInterposerError::Missing {
            field: FIELD_INSTRUCTION_BUFFER,
        });
    }
    let expected = ioctl_request_struct_size(request);
    if request_struct.len() != expected {
        return Err(IoctlInterposerError::StructSizeMismatch {
            request,
            expected,
            actual: request_struct.len(),
        });
    }
    encode_ioctl_interposer_record(&IoctlInterposerRecord {
        request,
        request_struct: CapturedField::Bytes(request_struct.to_vec()),
        command_buffer: CapturedField::Bytes(command_buffer.to_vec()),
        instruction_buffer: CapturedField::Bytes(instruction_buffer.to_vec()),
    })
}

/// Decode a record. The replay test calls this function.
///
/// Does not open a device. A pointer or a missing field is an error.
/// Struct bytes must be `_IOC_SIZE` long.
pub fn decode_ioctl_interposer_record(
    bytes: &[u8],
) -> Result<DecodedIoctlDump, IoctlInterposerError> {
    let mut rest = bytes;
    let magic = take(&mut rest, RECORD_MAGIC.len())?;
    if magic != RECORD_MAGIC.as_slice() {
        return Err(IoctlInterposerError::BadMagic);
    }
    let version = take_u32(&mut rest)?;
    if version != RECORD_VERSION {
        return Err(IoctlInterposerError::BadVersion { version });
    }
    let request = take_u64(&mut rest)?;
    let request_struct = require_bytes(FIELD_REQUEST_STRUCT, take_field(&mut rest)?)?;
    let command_buffer = require_bytes(FIELD_COMMAND_BUFFER, take_field(&mut rest)?)?;
    let instruction_buffer = require_bytes(FIELD_INSTRUCTION_BUFFER, take_field(&mut rest)?)?;
    if !rest.is_empty() {
        return Err(IoctlInterposerError::TrailingBytes);
    }
    let expected = ioctl_request_struct_size(request);
    if request_struct.len() != expected {
        return Err(IoctlInterposerError::StructSizeMismatch {
            request,
            expected,
            actual: request_struct.len(),
        });
    }
    Ok(DecodedIoctlDump {
        request,
        request_struct,
        command_buffer,
        instruction_buffer,
    })
}

/// One ioctl line from a text preload dump.
///
/// Offline only. `followed` holds copied `follow` hex blocks. A pointer with
/// no bytes is an error, not a stored address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextIoctlDumpRecord {
    /// Numeric request word from the dump line.
    pub request: u64,
    /// `_IOC_SIZE` struct bytes from the `arg` hex block.
    pub request_struct: Vec<u8>,
    /// Copied `follow` buffers that belong to this ioctl line.
    pub followed: Vec<CapturedField>,
}

/// Decode a text preload dump into one record per `ioctl` line.
///
/// Does not open a device and does not hard-code request numbers. An empty
/// buffer is missing. A follow line that stores only an address is
/// pointer-only. Struct bytes must be `_IOC_SIZE` long.
pub fn decode_text_ioctl_dump(
    bytes: &[u8],
) -> Result<Vec<TextIoctlDumpRecord>, IoctlInterposerError> {
    if bytes.is_empty() {
        return Err(IoctlInterposerError::Missing {
            field: FIELD_REQUEST_STRUCT,
        });
    }
    let text = std::str::from_utf8(bytes).map_err(|_err| IoctlInterposerError::BadMagic)?;
    if !text.starts_with("preload ") {
        return Err(IoctlInterposerError::BadMagic);
    }
    let lines: Vec<&str> = text.lines().collect();
    let mut records = Vec::new();
    let mut current: Option<TextIoctlDumpRecord> = None;
    let mut index = 0_usize;
    while index < lines.len() {
        let line = lines[index];
        index += 1;
        if line.is_empty() || line.starts_with("preload ") {
            continue;
        }
        if let Some(rest) = line.strip_prefix("ioctl ") {
            push_text_record(&mut records, current.take())?;
            current = Some(begin_text_ioctl(rest)?);
        } else if let Some(rest) = line.strip_prefix("arg ") {
            let record = current.as_mut().ok_or(IoctlInterposerError::Truncated)?;
            fill_text_request_struct(record, rest, &lines, &mut index)?;
        } else if let Some(rest) = line.strip_prefix("follow ") {
            let record = current.as_mut().ok_or(IoctlInterposerError::Truncated)?;
            record
                .followed
                .push(take_text_follow(rest, &lines, &mut index)?);
        } else if let Some(rest) = line.strip_prefix("mmap ") {
            skip_text_mmap_hex(rest, &lines, &mut index)?;
        } else {
            return Err(IoctlInterposerError::Truncated);
        }
    }
    push_text_record(&mut records, current)?;
    if records.is_empty() {
        return Err(IoctlInterposerError::Missing {
            field: FIELD_REQUEST_STRUCT,
        });
    }
    Ok(records)
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_field(out: &mut Vec<u8>, field: &CapturedField) -> Result<(), IoctlInterposerError> {
    match field {
        CapturedField::Missing => out.push(TAG_MISSING),
        CapturedField::PointerOnly { address } => {
            out.push(TAG_POINTER_ONLY);
            push_u64(out, *address);
        }
        CapturedField::Bytes(bytes) => {
            let len = u32::try_from(bytes.len()).map_err(|_err| IoctlInterposerError::Truncated)?;
            out.push(TAG_BYTES);
            push_u32(out, len);
            out.extend_from_slice(bytes);
        }
    }
    Ok(())
}

fn take<'a>(rest: &mut &'a [u8], len: usize) -> Result<&'a [u8], IoctlInterposerError> {
    if rest.len() < len {
        return Err(IoctlInterposerError::Truncated);
    }
    let (head, tail) = rest.split_at(len);
    *rest = tail;
    Ok(head)
}

fn take_u32(rest: &mut &[u8]) -> Result<u32, IoctlInterposerError> {
    let bytes = take(rest, 4)?;
    let array = <[u8; 4]>::try_from(bytes).map_err(|_err| IoctlInterposerError::Truncated)?;
    Ok(u32::from_le_bytes(array))
}

fn take_u64(rest: &mut &[u8]) -> Result<u64, IoctlInterposerError> {
    let bytes = take(rest, 8)?;
    let array = <[u8; 8]>::try_from(bytes).map_err(|_err| IoctlInterposerError::Truncated)?;
    Ok(u64::from_le_bytes(array))
}

fn take_field(rest: &mut &[u8]) -> Result<CapturedField, IoctlInterposerError> {
    let tag = take(rest, 1)?[0];
    match tag {
        TAG_MISSING => Ok(CapturedField::Missing),
        TAG_POINTER_ONLY => Ok(CapturedField::PointerOnly {
            address: take_u64(rest)?,
        }),
        TAG_BYTES => {
            let len = take_u32(rest)?;
            let len_usize = usize::try_from(len).map_err(|_err| IoctlInterposerError::Truncated)?;
            Ok(CapturedField::Bytes(take(rest, len_usize)?.to_vec()))
        }
        tag => Err(IoctlInterposerError::BadTag { tag }),
    }
}

fn require_bytes(
    field: &'static str,
    captured: CapturedField,
) -> Result<Vec<u8>, IoctlInterposerError> {
    match captured {
        CapturedField::PointerOnly { address } => {
            Err(IoctlInterposerError::PointerOnly { field, address })
        }
        CapturedField::Missing => Err(IoctlInterposerError::Missing { field }),
        CapturedField::Bytes(bytes) if bytes.is_empty() => {
            Err(IoctlInterposerError::Missing { field })
        }
        CapturedField::Bytes(bytes) => Ok(bytes),
    }
}

fn begin_text_ioctl(line: &str) -> Result<TextIoctlDumpRecord, IoctlInterposerError> {
    let request = parse_hex_token(line, "request=")?;
    let size = parse_dec_token(line, "size=")?;
    let expected = ioctl_request_struct_size(request);
    if size == 0 || expected == 0 {
        return Err(IoctlInterposerError::Missing {
            field: FIELD_REQUEST_STRUCT,
        });
    }
    if size != expected {
        return Err(IoctlInterposerError::StructSizeMismatch {
            request,
            expected,
            actual: size,
        });
    }
    Ok(TextIoctlDumpRecord {
        request,
        request_struct: Vec::new(),
        followed: Vec::new(),
    })
}

fn push_text_record(
    records: &mut Vec<TextIoctlDumpRecord>,
    record: Option<TextIoctlDumpRecord>,
) -> Result<(), IoctlInterposerError> {
    let Some(record) = record else {
        return Ok(());
    };
    let expected = ioctl_request_struct_size(record.request);
    if record.request_struct.is_empty() {
        return Err(IoctlInterposerError::Missing {
            field: FIELD_REQUEST_STRUCT,
        });
    }
    if record.request_struct.len() != expected {
        return Err(IoctlInterposerError::StructSizeMismatch {
            request: record.request,
            expected,
            actual: record.request_struct.len(),
        });
    }
    records.push(record);
    Ok(())
}

fn fill_text_request_struct(
    record: &mut TextIoctlDumpRecord,
    line: &str,
    lines: &[&str],
    index: &mut usize,
) -> Result<(), IoctlInterposerError> {
    if line.contains("unreadable") {
        return Err(IoctlInterposerError::Missing {
            field: FIELD_REQUEST_STRUCT,
        });
    }
    let len = parse_dec_token(line, "len=")?;
    let expected = ioctl_request_struct_size(record.request);
    if len == 0 {
        return Err(IoctlInterposerError::Missing {
            field: FIELD_REQUEST_STRUCT,
        });
    }
    if len != expected {
        return Err(IoctlInterposerError::StructSizeMismatch {
            request: record.request,
            expected,
            actual: len,
        });
    }
    record.request_struct = read_text_hex_bytes(lines, index, len, FIELD_REQUEST_STRUCT, None)?;
    Ok(())
}

fn take_text_follow(
    line: &str,
    lines: &[&str],
    index: &mut usize,
) -> Result<CapturedField, IoctlInterposerError> {
    let len = parse_dec_token(line, "len=")?;
    let address = parse_hex_token(line, "ptr=")?;
    if len == 0 {
        return Err(IoctlInterposerError::Missing {
            field: FIELD_FOLLOWED,
        });
    }
    let buf = read_text_hex_bytes(lines, index, len, FIELD_FOLLOWED, Some(address))?;
    Ok(CapturedField::Bytes(buf))
}

fn skip_text_mmap_hex(
    line: &str,
    lines: &[&str],
    index: &mut usize,
) -> Result<(), IoctlInterposerError> {
    if line
        .split_whitespace()
        .any(|token| token.starts_with("prot="))
    {
        return Ok(());
    }
    let len = parse_dec_token(line, "len=")?;
    if len == 0 {
        return Err(IoctlInterposerError::Missing {
            field: FIELD_FOLLOWED,
        });
    }
    let _skipped = read_text_hex_bytes(lines, index, len, FIELD_FOLLOWED, None)?;
    Ok(())
}

fn read_text_hex_bytes(
    lines: &[&str],
    index: &mut usize,
    expected: usize,
    field: &'static str,
    pointer: Option<u64>,
) -> Result<Vec<u8>, IoctlInterposerError> {
    let mut buf = Vec::with_capacity(expected);
    while buf.len() < expected && *index < lines.len() && is_hex_dump_line(lines[*index]) {
        let (offset, chunk) = parse_hex_dump_line(lines[*index])?;
        if offset != buf.len() || buf.len() + chunk.len() > expected {
            return Err(IoctlInterposerError::Truncated);
        }
        buf.extend(chunk);
        *index += 1;
    }
    if buf.len() == expected {
        return Ok(buf);
    }
    if buf.is_empty() {
        return match pointer {
            Some(address) => Err(IoctlInterposerError::PointerOnly { field, address }),
            None => Err(IoctlInterposerError::Missing { field }),
        };
    }
    Err(IoctlInterposerError::Truncated)
}

fn parse_hex_token(line: &str, key: &str) -> Result<u64, IoctlInterposerError> {
    let text = text_token(line, key)?;
    let digits = text.strip_prefix("0x").unwrap_or(text);
    u64::from_str_radix(digits, 16).map_err(|_err| IoctlInterposerError::Truncated)
}

fn parse_dec_token(line: &str, key: &str) -> Result<usize, IoctlInterposerError> {
    text_token(line, key)?
        .parse::<usize>()
        .map_err(|_err| IoctlInterposerError::Truncated)
}

fn text_token<'a>(line: &'a str, key: &str) -> Result<&'a str, IoctlInterposerError> {
    line.split_whitespace()
        .find_map(|token| token.strip_prefix(key))
        .ok_or(IoctlInterposerError::Truncated)
}

fn is_hex_dump_line(line: &str) -> bool {
    let Some((offset, _)) = line.split_once(':') else {
        return false;
    };
    offset.len() == 4 && offset.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn parse_hex_dump_line(line: &str) -> Result<(usize, Vec<u8>), IoctlInterposerError> {
    let (offset_text, rest) = line
        .split_once(':')
        .ok_or(IoctlInterposerError::Truncated)?;
    let offset =
        usize::from_str_radix(offset_text, 16).map_err(|_err| IoctlInterposerError::Truncated)?;
    let mut chunk = Vec::new();
    for token in rest.split_whitespace() {
        if token.len() != 2 {
            return Err(IoctlInterposerError::Truncated);
        }
        let byte = u8::from_str_radix(token, 16).map_err(|_err| IoctlInterposerError::Truncated)?;
        chunk.push(byte);
    }
    Ok((offset, chunk))
}

#[cfg(test)]
mod tests {
    use std::fmt::Write;

    use super::{
        CapturedField, FIELD_COMMAND_BUFFER, FIELD_INSTRUCTION_BUFFER, FIELD_REQUEST_STRUCT,
        IoctlInterposerError, IoctlInterposerRecord, decode_ioctl_interposer_record,
        decode_text_ioctl_dump, dump_ioctl_capture, encode_ioctl_interposer_record,
        ioctl_request_struct_size,
    };

    #[test]
    fn replay_rejects_pointer_only_and_requires_buffer_bytes() {
        // Strace of the whole-array run stored this address for request
        // 0xc0386446 and did not store the struct bytes. The header names
        // that request DRM_IOCTL_AMDXDNA_EXEC_CMD. This test does not open
        // /dev/accel and does not issue the ioctl.
        let request = 0xc038_6446_u64;
        let struct_pointer = 0x7ffe_0e17_5bd0_u64;
        let pointer_only = IoctlInterposerRecord {
            request,
            request_struct: CapturedField::PointerOnly {
                address: struct_pointer,
            },
            command_buffer: CapturedField::PointerOnly {
                address: struct_pointer,
            },
            instruction_buffer: CapturedField::PointerOnly {
                address: struct_pointer,
            },
        };
        let encoded = encode_ioctl_interposer_record(&pointer_only).expect("encode pointer-only");
        assert_eq!(
            decode_ioctl_interposer_record(&encoded),
            Err(IoctlInterposerError::PointerOnly {
                field: FIELD_REQUEST_STRUCT,
                address: struct_pointer,
            }),
            "pointer-only record must be rejected"
        );

        let struct_len = ioctl_request_struct_size(request);
        let request_struct = vec![0x5a_u8; struct_len];
        let command_buffer = vec![0x10_u8, 0x20, 0x30, 0x40];
        let instruction_buffer = vec![0x01_u8, 0x02, 0x03];
        let command_pointer = IoctlInterposerRecord {
            request,
            request_struct: CapturedField::Bytes(request_struct.clone()),
            command_buffer: CapturedField::PointerOnly {
                address: struct_pointer,
            },
            instruction_buffer: CapturedField::Bytes(instruction_buffer.clone()),
        };
        let encoded_command_pointer =
            encode_ioctl_interposer_record(&command_pointer).expect("encode command pointer");
        assert_eq!(
            decode_ioctl_interposer_record(&encoded_command_pointer),
            Err(IoctlInterposerError::PointerOnly {
                field: FIELD_COMMAND_BUFFER,
                address: struct_pointer,
            }),
            "a command buffer that is only a pointer must be rejected"
        );

        let missing_command = IoctlInterposerRecord {
            request,
            request_struct: CapturedField::Bytes(request_struct.clone()),
            command_buffer: CapturedField::Missing,
            instruction_buffer: CapturedField::Bytes(instruction_buffer.clone()),
        };
        let encoded_missing =
            encode_ioctl_interposer_record(&missing_command).expect("encode missing command");
        assert_eq!(
            decode_ioctl_interposer_record(&encoded_missing),
            Err(IoctlInterposerError::Missing {
                field: FIELD_COMMAND_BUFFER,
            }),
            "a missing command buffer must be rejected"
        );

        let empty_instruction = dump_ioctl_capture(request, &request_struct, &command_buffer, &[]);
        assert_eq!(
            empty_instruction,
            Err(IoctlInterposerError::Missing {
                field: FIELD_INSTRUCTION_BUFFER,
            }),
            "dump must refuse an empty instruction buffer"
        );

        let dumped = dump_ioctl_capture(
            request,
            &request_struct,
            &command_buffer,
            &instruction_buffer,
        )
        .expect("dump bytes");
        let decoded = decode_ioctl_interposer_record(&dumped).expect("decode dump");
        assert_eq!(decoded.request, request);
        assert_eq!(decoded.request_struct, request_struct);
        assert_eq!(decoded.command_buffer, command_buffer);
        assert_eq!(decoded.instruction_buffer, instruction_buffer);
    }

    fn replay_token<'a>(line: &'a str, key: &str) -> &'a str {
        line.split_whitespace()
            .find_map(|token| token.strip_prefix(key))
            .unwrap_or_else(|| panic!("dump line missing {key}"))
    }

    fn replay_hex_u64(text: &str) -> u64 {
        let digits = text.strip_prefix("0x").unwrap_or(text);
        u64::from_str_radix(digits, 16).unwrap_or_else(|err| panic!("hex {text}: {err}"))
    }

    fn replay_dec_usize(text: &str) -> usize {
        text.parse::<usize>()
            .unwrap_or_else(|err| panic!("integer {text}: {err}"))
    }

    fn replay_hex_line(offset: usize, bytes: &[u8]) -> String {
        let mut line = format!("{offset:04x}:");
        for byte in bytes {
            let _ = write!(line, " {byte:02x}");
        }
        line
    }

    fn replay_hex_block(bytes: &[u8]) -> String {
        let mut block = String::new();
        for (index, chunk) in bytes.chunks(16).enumerate() {
            if index > 0 {
                block.push('\n');
            }
            block.push_str(&replay_hex_line(index * 16, chunk));
        }
        block
    }

    #[test]
    fn replay_loads_text_ioctl_dump_bytes() {
        // Offline replay of a text preload dump. Does not open /dev/accel.
        // Quiet when DRAGON_IOCTL_DUMP is unset so the rest of the suite passes.
        let Ok(path) = std::env::var("DRAGON_IOCTL_DUMP") else {
            return;
        };
        let bytes = std::fs::read(&path).unwrap_or_else(|err| panic!("read {path}: {err}"));
        assert!(
            bytes.starts_with(b"preload "),
            "dump file was not opened as a text preload capture"
        );
        let text = std::str::from_utf8(&bytes).expect("dump is utf-8 text");
        let mut expected_requests = Vec::new();
        let mut expected_sizes = Vec::new();
        let mut expected_follow_lens = Vec::new();
        for line in text.lines() {
            if let Some(rest) = line.strip_prefix("ioctl ") {
                expected_requests.push(replay_hex_u64(replay_token(rest, "request=")));
                expected_sizes.push(replay_dec_usize(replay_token(rest, "size=")));
            } else if let Some(rest) = line.strip_prefix("follow ") {
                expected_follow_lens.push(replay_dec_usize(replay_token(rest, "len=")));
            }
        }
        assert!(!expected_requests.is_empty(), "dump has no ioctl lines");
        let records = decode_text_ioctl_dump(&bytes).expect("decode text ioctl dump");
        assert_eq!(
            records.len(),
            expected_requests.len(),
            "one decoded record per ioctl line"
        );
        let mut followed_index = 0_usize;
        for (record, (request, size)) in records
            .iter()
            .zip(expected_requests.iter().zip(expected_sizes.iter()))
        {
            assert_eq!(
                record.request, *request,
                "request word must come from the dump"
            );
            assert_eq!(
                record.request_struct.len(),
                *size,
                "arg byte count must match the dump size"
            );
            assert_eq!(
                record.request_struct.len(),
                ioctl_request_struct_size(record.request),
                "struct bytes must be _IOC_SIZE long"
            );
            assert!(
                !record.request_struct.is_empty(),
                "empty request struct is missing"
            );
            assert!(
                text.contains(replay_hex_block(&record.request_struct).as_str()),
                "decoded arg hex must occur in the dump text"
            );
            for field in &record.followed {
                let follow_len = expected_follow_lens[followed_index];
                followed_index += 1;
                match field {
                    CapturedField::Bytes(buf) => {
                        assert!(follow_len > 0, "empty buffer is missing, not bytes");
                        assert_eq!(buf.len(), follow_len, "followed buffer length");
                        assert!(!buf.is_empty(), "empty buffer is missing");
                        assert!(
                            text.contains(replay_hex_line(0x40, &buf[0x40..0x50]).as_str()),
                            "followed buffer bytes must come from the dump hex"
                        );
                        let last = if follow_len.is_multiple_of(16) {
                            follow_len - 16
                        } else {
                            follow_len - (follow_len % 16)
                        };
                        let last_end = follow_len;
                        assert!(
                            text.contains(replay_hex_line(last, &buf[last..last_end]).as_str()),
                            "followed buffer tail must come from the dump hex"
                        );
                    }
                    CapturedField::PointerOnly { address } => {
                        panic!("follow bytes were captured; got pointer-only {address:#x}");
                    }
                    CapturedField::Missing => {
                        panic!("follow len {follow_len} was not missing in the dump");
                    }
                }
            }
        }
        assert_eq!(
            followed_index,
            expected_follow_lens.len(),
            "one followed buffer per follow line"
        );
    }
}
