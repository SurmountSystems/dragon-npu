//! Offline silicon contract helpers for the staged mladf lab path.
//!
//! Pure helpers only — **no** `/dev/accel`, **no** EXEC_CMD, **no** `ran_on_npu`.
//! Used by timed-path batch diagnostics, unit tests, and
//! `tools/silicon_contract_check.sh` (via cargo tests).
//!
//! Live residuals (2026-07-16):
//! - Raw 1688 B txn → FW `APP_LOAD_PDI_FAIL` (`0x3000003`) / `ert_state=Error`
//! - aiebu ELF + rela patch works in userspace; FW may still reject (`ert_state=Error`)
//! - Driver soft-invalid QoS when `gops=0` and `fps=0` ("Invalid QoS …")
//!
//! Note: `0x106` in [`aie2_fw_status_name`] is a **MSG_OP** (map_host), not APP status.

use super::direct::{
    DirectError, MAX_INSTRUCTION_BLOB_BYTES, TimedOpResult, build_ert_start_npu_cmd_bo,
    build_ert_start_npu_cmd_bo_with_kernel_args,
};
use super::lab_io::{LabKernelArgLayout, pack_start_npu_kernel_args_with_ctrl_layout};
use super::mladf_pack::MAX_MLADF_LAB_FILE_BYTES;
use super::txn::{
    InstructionBlobKind, LabRelaMode, classify_instruction_blob, format_dynsym_report,
    is_aiebu_elf_ctrltext, parse_elf32_dynsym, parse_elf32_rela_dyn,
    patch_aiebu_elf_ctrltext_with_bos_ex, validate_patched_ctrltext_classic,
};

/// Byte offset of `prop_count` in a START_NPU Cmd BO from
/// [`build_ert_start_npu_cmd_bo_with_kernel_args`] / `_ex`.
///
/// Layout: packed header **4** B + CU mask (4) + buffer u64 (8) + buffer_size (4)
/// → prop_count at **20**. (Not 28 — that is n_bos hi after kernel args begin.)
pub const START_NPU_PROP_COUNT_BYTE_OFF: usize = 20;

/// Read START_NPU `prop_count` from a packed Cmd BO (same offset as direct unit tests).
#[must_use]
pub fn prop_count_from_start_npu_cmd_bo(bo: &[u8]) -> Option<u32> {
    let off = START_NPU_PROP_COUNT_BYTE_OFF;
    bo.get(off..off + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

/// Default lab QoS gops when env unset (makes driver `is_valid_qos_dpm_params` happy).
pub const LAB_DEFAULT_QOS_GOPS: u32 = 100;
/// Default lab QoS fps when env unset.
pub const LAB_DEFAULT_QOS_FPS: u32 = 30;

/// What value goes into START_NPU `instruction_buffer_size` / `inst_size`.
///
/// Device BO is still zero-padded to page (`bo_alloc`, often 4096). Userspace
/// may advertise **content** size (ctrltext/txn_size, e.g. 1688) or full **bo**
/// size (page, e.g. 4096).
///
/// Env: `XDNA2_LAB_INSTR_SIZE_MODE=content|bo` (default **content**).
///
/// **Content resolve:** when both `content_len` and `bo_alloc` are non-zero,
/// [`resolve_start_npu_inst_size`] returns `min(content_len, bo_alloc)` so
/// START_NPU never advertises DMA longer than the written BO (see
/// [`instr_size_content_clamped`]). Zero content falls back to `bo_alloc`.
///
/// **Live residual (CLOSED env HITL):** both content=1688 and historical bo=4096
/// still yield FW `APP_LOAD_PDI_FAIL` (`0x3000003`) with aiebu ELF+rela×12 on
/// 2026-07-16. Size mode is locked offline for contract honesty only — **not**
/// a silicon Pass knob and **not** C-ready.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LabInstrSizeMode {
    /// START_NPU size = content length, **clamped to `bo_alloc`** when both set
    /// (default; live content trial CLOSED still APP_LOAD).
    #[default]
    Content,
    /// START_NPU size = BO allocation (XRT `bo.size()` page-aligned parity).
    Bo,
}

impl LabInstrSizeMode {
    /// Stable token for detail strings.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Content => "content",
            Self::Bo => "bo",
        }
    }
}

/// Fail-closed parse of `XDNA2_LAB_INSTR_SIZE_MODE`.
///
/// Unset / empty → [`LabInstrSizeMode::Content`]. Known: `content`, `bo`
/// (aliases `alloc`/`page` → bo). Unknown tokens → `Err` (no silent default).
///
/// # Errors
///
/// Unrecognized non-empty token.
pub fn try_lab_instr_size_mode_from_str(raw: Option<&str>) -> Result<LabInstrSizeMode, String> {
    match raw.map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(LabInstrSizeMode::Content),
        Some(s) if s.eq_ignore_ascii_case("content") => Ok(LabInstrSizeMode::Content),
        Some(s)
            if s.eq_ignore_ascii_case("bo")
                || s.eq_ignore_ascii_case("alloc")
                || s.eq_ignore_ascii_case("page") =>
        {
            Ok(LabInstrSizeMode::Bo)
        }
        Some(s) => Err(format!(
            "unknown XDNA2_LAB_INSTR_SIZE_MODE={s:?}; expected content|bo \
             (aliases alloc|page→bo)"
        )),
    }
}

/// Convenience: unset/empty → content; **unknown panics not used** — prefer try.
///
/// Deprecated for residual HITL env: use [`try_lab_instr_size_mode_from_str`].
/// Retained for pure defaults when input is known-good or None.
#[must_use]
pub fn lab_instr_size_mode_from_str(raw: Option<&str>) -> LabInstrSizeMode {
    try_lab_instr_size_mode_from_str(raw).unwrap_or(LabInstrSizeMode::Content)
}

/// Read process env `XDNA2_LAB_INSTR_SIZE_MODE` (fail-closed on unknown).
///
/// # Errors
///
/// Unknown token set in env.
pub fn lab_instr_size_mode_from_env() -> Result<LabInstrSizeMode, String> {
    try_lab_instr_size_mode_from_str(std::env::var("XDNA2_LAB_INSTR_SIZE_MODE").ok().as_deref())
}

/// Resolve START_NPU instruction size field (pure).
///
/// - `content` → `min(content_len, bo_alloc)` when both non-zero (never advertise
///   DMA longer than written BO bytes); zero content falls back to `bo_alloc`
/// - `bo` → `bo_alloc` (e.g. 4096)
///
/// Floor is 1 when both inputs are zero (refuse-class footgun for callers to detect).
#[must_use]
pub fn resolve_start_npu_inst_size(content_len: u32, bo_alloc: u32, mode: LabInstrSizeMode) -> u32 {
    match mode {
        LabInstrSizeMode::Bo => bo_alloc.max(1),
        LabInstrSizeMode::Content => {
            if content_len == 0 {
                bo_alloc.max(1)
            } else if bo_alloc == 0 {
                content_len.max(1)
            } else {
                content_len.min(bo_alloc)
            }
        }
    }
}

/// True when content mode would clamp `content_len` down to `bo_alloc`.
#[must_use]
pub fn instr_size_content_clamped(content_len: u32, bo_alloc: u32, mode: LabInstrSizeMode) -> bool {
    matches!(mode, LabInstrSizeMode::Content)
        && content_len > 0
        && bo_alloc > 0
        && content_len > bo_alloc
}

/// Format size dual-mode detail fragment (no secrets).
#[must_use]
pub fn format_instr_size_detail(
    mode: LabInstrSizeMode,
    content_len: u32,
    bo_alloc: u32,
    start_npu_size: u32,
) -> String {
    format!(
        "inst_size_mode={} content_size={content_len}B bo_size={bo_alloc}B \
         start_npu_size={start_npu_size}B",
        mode.as_str()
    )
}

/// Unit/API research helper for APP_LOAD residual classification strings.
///
/// **Not** auto-wired into live timed `detail` / `last_apply` paths — call sites
/// may adopt later. Pure string only; never claims Pass, C-ready, or
/// `ran_on_npu`. Encodes locked residual [`AIE2_FW_APP_LOAD_PDI_FAIL`] /
/// `APP_LOAD_PDI_FAIL` plus START_NPU size dual-mode fields. Env layout/size
/// trials are **CLOSED**.
#[must_use]
pub fn format_app_load_residual_diag(
    aiebu_elf: bool,
    rela_count: usize,
    size_mode: LabInstrSizeMode,
    content_len: u32,
    bo_alloc: u32,
    rela_mode: LabRelaMode,
    layout: &str,
) -> String {
    let start = resolve_start_npu_inst_size(content_len, bo_alloc, size_mode);
    let size = format_instr_size_detail(size_mode, content_len, bo_alloc, start);
    let layout = if layout.is_empty() { "na" } else { layout };
    format!(
        "app_load_residual code=0x{:x} name={} aiebu_elf={} rela_count={} \
         rela_mode={} layout={layout}; {size}; env_hitl_closed=1 \
         content_and_xrt3_closed=1 not_c_ready=1 not_ran_on_npu=1",
        AIE2_FW_APP_LOAD_PDI_FAIL,
        aie2_fw_status_name(AIE2_FW_APP_LOAD_PDI_FAIL),
        u8::from(aiebu_elf),
        rela_count,
        rela_mode.as_str(),
    )
}

/// Offline residual class from `last_apply.json` / timed `detail` signals.
///
/// Pure research enum — **never** means C-ready or product Pass. Use
/// [`classify_last_apply_residual`] / [`parse_last_apply_detail_signals`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SiliconResidualClass {
    /// aiebu ELF **and** rela evidence (`rela_count>0` or `aiebu_rela_patched`)
    /// **and** ERT Error → locked APP_LOAD residual class (journal often
    /// `0x3000003`; do not invent code without detail/journal).
    AppLoadPdiFailAfterAiebu,
    /// Raw / non-ELF blob + ERT Error — classic raw-bin APP_LOAD class (heuristic).
    AppLoadLikelyRawOrUnpatched,
    /// ERT Error without enough aiebu+rela classification (unknown blob, or
    /// aiebu without rela evidence).
    ErtErrorUnknownBlob,
    /// Timed path report without ERT Error marker but no Pass (e.g. OFM refuse).
    NotPassNoErtError,
    /// Insufficient signals to classify.
    Unclassified,
}

impl SiliconResidualClass {
    /// Stable token for docs / scripts (never `ran_on_npu` / C-ready).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AppLoadPdiFailAfterAiebu => "app_load_pdi_fail_after_aiebu",
            Self::AppLoadLikelyRawOrUnpatched => "app_load_likely_raw_or_unpatched",
            Self::ErtErrorUnknownBlob => "ert_error_unknown_blob",
            Self::NotPassNoErtError => "not_pass_no_ert_error",
            Self::Unclassified => "unclassified",
        }
    }

    /// True when this class is the locked ELF+rela APP_LOAD residual.
    #[must_use]
    pub const fn is_locked_app_load_after_aiebu(self) -> bool {
        matches!(self, Self::AppLoadPdiFailAfterAiebu)
    }
}

/// Parsed signals from a timed `detail` string (best-effort, pure).
///
/// Research bag of optional tokens — several independent presence flags are
/// expected (not a state machine).
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LastApplyDetailSignals {
    /// `ert_state=N` numeric when present (first `ert_state=` only).
    pub ert_state: Option<u32>,
    /// True when **first** `ert_state=` token is Error (name or numeric 5).
    pub ert_error: bool,
    /// `aiebu_elf=0|1` when present.
    pub aiebu_elf: Option<bool>,
    /// `rela_count=N` when present.
    pub rela_count: Option<usize>,
    /// True when `aiebu_rela_patched` token appears (userspace rela path ran).
    pub aiebu_rela_patched: bool,
    /// Bare `layout=` token (`count4` / `xrt3` / …) when found.
    pub layout: Option<String>,
    /// `inst_size_mode=content|bo` when found.
    pub inst_size_mode: Option<String>,
    /// Exact `ran_on_npu=true|false` token when found — **never invent true**.
    pub ran_on_npu: Option<bool>,
    /// `raw_nonzero_bytes=N` from ofm_stats when found.
    pub ofm_raw_nonzero_bytes: Option<u64>,
    /// Known FW status hex found in detail (`0x3000003` / `0x4000003`), if any.
    pub fw_status_code: Option<u32>,
    /// First `map_offset=` (fixture: instruction Dev BO; often `0xffffffffffffffff`).
    pub instr_map_offset: Option<u64>,
    /// First `ty=` buffer type token (fixture instr: `Dev`).
    pub instr_bo_type: Option<String>,
    /// First `xdna_addr=` after instr context (or first in detail).
    pub instr_xdna_addr: Option<u64>,
    /// `content_size=N` or first `content=N` size (bytes, `B` suffix optional).
    pub content_size_b: Option<u32>,
    /// First `bo_size=N` (instruction BO alloc; often page-padded 4096).
    pub bo_size_b: Option<u32>,
    /// `start_npu_size=N` START_NPU buffer_size field.
    pub start_npu_size_b: Option<u32>,
    /// True when detail contains `SubmitWaitNotOk` (OFM refuse after WAIT).
    pub submit_wait_not_ok: bool,
    /// `rela_mode=overwrite|add` when found.
    pub rela_mode: Option<String>,
    /// `config_cu=0|1` lab CONFIG_CU attach token when present (research; ≠ Pass).
    pub config_cu: Option<u8>,
    /// True when detail/status indicates CONFIG_CU attach failed (research only).
    pub config_cu_failed: bool,
    /// `pdi_bytes=N` when present (opaque PDI length after lab attach).
    pub pdi_bytes: Option<u32>,
    /// `cu_func=N` when present (0–255).
    pub cu_func: Option<u8>,
}

/// End of first whitespace/`;`/`,` delimited token.
#[must_use]
fn detail_token_end(s: &str) -> usize {
    s.find(|c: char| c.is_whitespace() || c == ';' || c == ',' || c == ']')
        .unwrap_or(s.len())
}

/// Parse `0x…` / decimal token as `u64` (first token only).
#[must_use]
fn parse_detail_u64_token(tok: &str) -> Option<u64> {
    let t = tok.trim();
    if t.is_empty() {
        return None;
    }
    if let Some(hex) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        // Allow full 16-nibble values (e.g. map_offset=0xffffffffffffffff).
        return u64::from_str_radix(hex, 16).ok();
    }
    t.parse::<u64>().ok()
}

/// Parse size token `1688` / `1688B` / `1688b` as `u32` bytes.
#[must_use]
fn parse_detail_size_bytes_token(tok: &str) -> Option<u32> {
    let t = tok.trim();
    let t = t
        .strip_suffix('B')
        .or_else(|| t.strip_suffix('b'))
        .unwrap_or(t)
        .trim();
    if t.is_empty() {
        return None;
    }
    t.parse::<u32>().ok()
}

/// First value after `key=` as a delimited token, or `None`.
#[must_use]
fn detail_first_token_after<'a>(detail: &'a str, key: &str) -> Option<&'a str> {
    let i = detail.find(key)?;
    let rest = &detail[i + key.len()..];
    let tok = &rest[..detail_token_end(rest)];
    let tok = tok.trim();
    if tok.is_empty() { None } else { Some(tok) }
}

/// True when instruction BO `map_offset` is the DRM "no host mmap" sentinel.
///
/// Live XDNA2 Dev-type instruction BOs commonly report
/// `map_offset=0xffffffffffffffff` (`u64::MAX`) — filled via DevHeap, not
/// GEM mmap. **Not** an APP_LOAD root cause by itself.
#[must_use]
pub fn instr_map_offset_is_no_mmap(offset: u64) -> bool {
    offset == u64::MAX
}

/// True when signals show userspace rela evidence.
#[must_use]
pub fn last_apply_has_rela_evidence(sig: &LastApplyDetailSignals) -> bool {
    sig.aiebu_rela_patched || sig.rela_count.is_some_and(|n| n > 0)
}

/// True when content/start_npu sizes agree (content-mode START_NPU honesty).
#[must_use]
pub fn last_apply_start_npu_matches_content(sig: &LastApplyDetailSignals) -> bool {
    match (sig.content_size_b, sig.start_npu_size_b) {
        (Some(c), Some(s)) => c == s && c > 0,
        _ => false,
    }
}

/// True when BO alloc is strictly larger than content (`bo_size > content_size`).
///
/// Research only — does **not** prove page-alignment; token is `bo_gt_content=1`.
#[must_use]
pub fn last_apply_bo_gt_content(sig: &LastApplyDetailSignals) -> bool {
    match (sig.content_size_b, sig.bo_size_b) {
        (Some(c), Some(b)) => b > c && c > 0,
        _ => false,
    }
}

/// Coarse last_apply fingerprint for C3 "no new C2 evidence" detection.
///
/// Compares `status` / `aiebu_elf` / `ert_error` / `ran_on_npu` / `detail` length /
/// optional top-level `rela_count` / **`config_cu_present`** (CONFIG_CU attach
/// signal so a post-P4 sandwich with `config_cu=` is not false C3_SKIP when
/// other fields coincidentally match). **Not** a full-body hash and **not** new
/// APP_LOAD research — only detects whether a snapshot matches another (or the
/// locked baseline shape). Never claims Pass / C-ready / `ran_on_npu`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LastApplyEvidenceFingerprint {
    /// Top-level or status string (trimmed); empty → `None`.
    pub status: Option<String>,
    /// Top-level `aiebu_elf` when known.
    pub aiebu_elf: Option<bool>,
    /// Top-level `ert_error` when known.
    pub ert_error: Option<bool>,
    /// Top-level `ran_on_npu` when known — never invent `true` for Pass.
    pub ran_on_npu: Option<bool>,
    /// Byte length of `detail` (0 when absent).
    pub detail_len: usize,
    /// Schema-2 top-level `rela_count` when provided (detail rela is not re-parsed).
    pub rela_count: Option<u32>,
    /// True when status/detail show a lab CONFIG_CU attempt (`config_cu=` /
    /// `config_cu_failed` / status `config_cu_failed`). Locked baseline = false.
    pub config_cu_present: bool,
}

/// True when status/detail carry a lab CONFIG_CU attach signal (research only).
///
/// Detects: top-level status `config_cu_failed`, detail `config_cu=…`, or
/// `config_cu_failed` token / "CONFIG_CU attach failed" phrase. Never invents
/// Pass / C-ready / `ran_on_npu`.
#[must_use]
pub fn last_apply_has_config_cu_signal(status: Option<&str>, detail: Option<&str>) -> bool {
    if status
        .map(str::trim)
        .is_some_and(|s| s.eq_ignore_ascii_case("config_cu_failed"))
    {
        return true;
    }
    let Some(d) = detail else {
        return false;
    };
    if d.contains("config_cu=") {
        return true;
    }
    // Whole-token `config_cu_failed` (not a longer identifier).
    if let Some(i) = d.find("config_cu_failed") {
        let before_ok = i == 0
            || d.as_bytes()
                .get(i - 1)
                .is_some_and(|b| !b.is_ascii_alphanumeric() && *b != b'_');
        let after = i + "config_cu_failed".len();
        let after_ok = d
            .as_bytes()
            .get(after)
            .is_none_or(|b| !b.is_ascii_alphanumeric() && *b != b'_');
        if before_ok && after_ok {
            return true;
        }
    }
    // Live fail detail phrase from timed path (no bare token).
    d.to_ascii_lowercase().contains("config_cu attach failed")
}

/// True when parsed detail signals show CONFIG_CU attach evidence.
///
/// Aligned with [`last_apply_has_config_cu_signal`] / C3 fingerprint `config_cu`
/// bit: `config_cu=` token and/or `config_cu_failed` only. Bare `pdi_bytes=` /
/// `cu_func=` alone do **not** set present (lab always emits `config_cu=` with
/// PDI size; checklist may still print those fields when parsed).
#[must_use]
pub fn last_apply_config_cu_present(sig: &LastApplyDetailSignals) -> bool {
    sig.config_cu.is_some() || sig.config_cu_failed
}

/// Merge top-level `status` into CONFIG_CU signals (e.g. `config_cu_failed`).
///
/// Pure; does not invent Pass. Call after [`parse_last_apply_detail_signals`].
pub fn merge_last_apply_status_config_cu(status: Option<&str>, sig: &mut LastApplyDetailSignals) {
    if status
        .map(str::trim)
        .is_some_and(|s| s.eq_ignore_ascii_case("config_cu_failed"))
    {
        sig.config_cu_failed = true;
    }
}

/// Build a coarse fingerprint from last_apply field slices (pure; no I/O).
#[must_use]
pub fn last_apply_evidence_fingerprint(
    status: Option<&str>,
    aiebu_elf: Option<bool>,
    ert_error: Option<bool>,
    ran_on_npu: Option<bool>,
    detail: Option<&str>,
    rela_count: Option<u32>,
) -> LastApplyEvidenceFingerprint {
    let status_owned = status
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned);
    LastApplyEvidenceFingerprint {
        status: status_owned,
        aiebu_elf,
        ert_error,
        ran_on_npu,
        detail_len: detail.map_or(0, str::len),
        rela_count,
        config_cu_present: last_apply_has_config_cu_signal(status, detail),
    }
}

/// True when two coarse fingerprints match (same C3 evidence identity).
///
/// Equality of status / aiebu / ert / ran_on_npu / detail_len / rela_count /
/// `config_cu_present` only.
#[must_use]
pub fn last_apply_is_same_fixture_fingerprint(
    a: &LastApplyEvidenceFingerprint,
    b: &LastApplyEvidenceFingerprint,
) -> bool {
    a == b
}

/// True when fields match the **locked** baseline APP_LOAD residual shape
/// (`ert_cmd_error` + aiebu + ert_error + `ran_on_npu=false` + class
/// [`SiliconResidualClass::AppLoadPdiFailAfterAiebu`]).
///
/// Used by C3 skip when only the repo fixture (or identical shape) is present —
/// does **not** expand the closed APP_LOAD signal matrix and never unlocks C.
#[must_use]
pub fn last_apply_matches_locked_app_load_baseline(
    status: Option<&str>,
    aiebu_elf: Option<bool>,
    ert_error: Option<bool>,
    ran_on_npu: Option<bool>,
    detail: Option<&str>,
    rela_count: Option<u32>,
) -> bool {
    if aiebu_elf != Some(true) || ert_error != Some(true) || ran_on_npu != Some(false) {
        return false;
    }
    let status_ok = status.is_some_and(|s| {
        let t = s.trim();
        t.eq_ignore_ascii_case("ert_cmd_error") || t.eq_ignore_ascii_case("ert_error")
    });
    if !status_ok {
        return false;
    }
    let rela_usize = rela_count.map(|n| n as usize);
    let class = classify_last_apply_residual_ex(status, detail, aiebu_elf, ert_error, rela_usize);
    class == SiliconResidualClass::AppLoadPdiFailAfterAiebu
}

/// True when a journal (or detail) line is a soft APP_LOAD / Failed-cmd hint.
///
/// Research-only filter for C3 `journal_hint=` print — **never** invents a FW
/// code into residual class without a detail field.
#[must_use]
pub fn journal_line_has_app_load_hint(line: &str) -> bool {
    let l = line.to_ascii_lowercase();
    l.contains("0x3000003")
        || l.contains("0x4000003")
        || l.contains("app_load")
        || l.contains("failed cmd")
        || l.contains("status 0x3000003")
        || l.contains("status 0x4000003")
}

/// Live C2 journal dual-status pair (research only; never invents product Pass).
///
/// Operator C2 (2026-07-18): `Failed cmd … 0x3000003` + nearby `Status 0x4000003`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct JournalFwStatusPair {
    /// From `Failed cmd … status 0x…` when present.
    pub failed_cmd_status: Option<u32>,
    /// From a standalone `Status 0x…` line (completion-chain) when present.
    pub chain_status: Option<u32>,
}

impl JournalFwStatusPair {
    /// True when the locked live pair is present (APP_LOAD fail + INVALID_PARAM chain).
    #[must_use]
    pub fn is_c2_app_load_pair(&self) -> bool {
        self.failed_cmd_status == Some(AIE2_FW_APP_LOAD_PDI_FAIL)
            && self.chain_status == Some(AIE2_FW_INVALID_PARAM)
    }

    /// Soft report tokens (never claims Pass / C-ready).
    #[must_use]
    pub fn format_soft_note(&self) -> String {
        let mut parts = vec![
            "journal_fw_pair".into(),
            "not_c_ready=1".into(),
            "not_ran_on_npu=1".into(),
            "invent_pass=0".into(),
        ];
        match self.failed_cmd_status {
            Some(c) => {
                parts.push(format!("failed_cmd=0x{c:x}"));
                parts.push(format!("failed_cmd_name={}", aie2_fw_status_name(c)));
            }
            None => parts.push("failed_cmd=absent".into()),
        }
        match self.chain_status {
            Some(c) => {
                parts.push(format!("chain_status=0x{c:x}"));
                parts.push(format!("chain_name={}", aie2_fw_status_name(c)));
            }
            None => parts.push("chain_status=absent".into()),
        }
        if self.is_c2_app_load_pair() {
            parts.push("c2_app_load_pair=1".into());
        }
        parts.join(" ")
    }
}

/// Parse journal (or multi-line) text for Failed-cmd + Status hex codes.
///
/// **Never** invents codes: only literal `0x…` on lines that match the pure
/// token helpers ([`parse_failed_cmd_status_token`] /
/// [`parse_chain_status_token`]). First match wins per field.
#[must_use]
pub fn parse_journal_fw_status_pair(text: &str) -> JournalFwStatusPair {
    let mut out = JournalFwStatusPair::default();
    for line in text.lines() {
        if out.failed_cmd_status.is_none() {
            if let Some(c) = parse_failed_cmd_status_token(line) {
                out.failed_cmd_status = Some(c);
            }
        }
        if out.chain_status.is_none() {
            if let Some(c) = parse_chain_status_token(line) {
                out.chain_status = Some(c);
            }
        }
    }
    out
}

/// Pure single-line parser: Failed-cmd status hex when the line is a failed-cmd
/// journal line.
///
/// Accepts `failed cmd` / `failed_cmd` (case-insensitive). Returns the first
/// `0x…` token on that line only. **Never** invents codes from unrelated text.
///
/// Stamp `2c4b96bf` unit surface (with multi-line pair parse).
#[must_use]
pub fn parse_failed_cmd_status_token(line: &str) -> Option<u32> {
    let l = line.trim();
    if l.is_empty() {
        return None;
    }
    let ll = l.to_ascii_lowercase();
    if !(ll.contains("failed cmd") || ll.contains("failed_cmd")) {
        return None;
    }
    first_hex_status_token(l)
}

/// Pure single-line parser: chain/completion `Status 0x…` when the line is **not**
/// a Failed-cmd line.
///
/// Requires a `status 0x` / `status 0X` substring and excludes failed-cmd lines so
/// the primary load-reject code is not double-counted as chain status.
///
/// Stamp `2c4b96bf` unit surface (with multi-line pair parse).
#[must_use]
pub fn parse_chain_status_token(line: &str) -> Option<u32> {
    let l = line.trim();
    if l.is_empty() {
        return None;
    }
    let ll = l.to_ascii_lowercase();
    if ll.contains("failed cmd") || ll.contains("failed_cmd") {
        return None;
    }
    if !ll.contains("status 0x") {
        return None;
    }
    first_hex_status_token(l)
}

/// First `0x` / `0X` hex u32-ish token on a line (truncates to low 32 bits if longer).
///
/// Public pure helper for journal research; **never** invents a code without a
/// literal hex token. Prefer [`parse_failed_cmd_status_token`] /
/// [`parse_chain_status_token`] for typed line roles.
#[must_use]
pub fn first_hex_status_token(line: &str) -> Option<u32> {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i + 2 < bytes.len() {
        if bytes[i] == b'0' && (bytes[i + 1] == b'x' || bytes[i + 1] == b'X') {
            let start = i + 2;
            let mut end = start;
            while end < bytes.len() && bytes[end].is_ascii_hexdigit() {
                end += 1;
            }
            if end > start {
                let hex = &line[start..end];
                // Accept up to 16 hex digits; use low 32 bits of the parse.
                if let Ok(v) = u64::from_str_radix(hex, 16) {
                    return u32::try_from(v & 0xffff_ffff).ok();
                }
            }
            i = end;
            continue;
        }
        i += 1;
    }
    None
}

/// Max hex words accepted from a fixture/list (research dumps are tiny).
pub const PARSE_U32_HEX_WORD_LIST_MAX: usize = 256;

/// Parse LE u32 word list from a research fixture (one hex word per line; `#` comments).
///
/// Refuses empty input and lists longer than [`PARSE_U32_HEX_WORD_LIST_MAX`].
///
/// # Errors
///
/// Invalid hex token, empty after strip, or too many words.
pub fn parse_u32_hex_word_list(text: &str) -> Result<Vec<u32>, String> {
    let mut out = Vec::new();
    for (lineno, raw) in text.lines().enumerate() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if out.len() >= PARSE_U32_HEX_WORD_LIST_MAX {
            return Err(format!(
                "too many hex words (max {PARSE_U32_HEX_WORD_LIST_MAX})"
            ));
        }
        let hex = line
            .strip_prefix("0x")
            .or_else(|| line.strip_prefix("0X"))
            .unwrap_or(line);
        let w = u32::from_str_radix(hex, 16)
            .map_err(|e| format!("line {}: bad hex {line:?}: {e}", lineno + 1))?;
        out.push(w);
    }
    if out.is_empty() {
        return Err("no hex words".into());
    }
    Ok(out)
}

/// Decode a pure START_NPU Cmd BO (header + payload) into fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartNpuCmdDecode {
    /// Unpacked header state nibble.
    pub state: u32,
    /// Unpacked header count (data dwords).
    pub count: u32,
    /// Unpacked header opcode.
    pub opcode: u32,
    /// CU mask dword.
    pub cu_mask: u32,
    /// Instruction buffer device address.
    pub inst_addr: u64,
    /// START_NPU size field.
    pub inst_size: u32,
    /// Property count field (shim I/O uses 0).
    pub prop_count: u32,
    /// Flexible-array dwords after prop_count.
    pub flexible_args: Vec<u32>,
}

/// Decode START_NPU Cmd BO built by [`build_ert_start_npu_cmd_bo_ex`] family.
///
/// # Errors
///
/// Truncated buffer or count mismatch.
pub fn decode_start_npu_cmd_bo(bytes: &[u8]) -> Result<StartNpuCmdDecode, String> {
    use super::direct::{ErtCmdOpcode, parse_amdxdna_cmd_header_from_bytes};
    let h = parse_amdxdna_cmd_header_from_bytes(bytes).map_err(|e| e.to_string())?;
    if h.opcode != ErtCmdOpcode::StartNpu as u32 {
        return Err(format!("opcode {} not START_NPU", h.opcode));
    }
    let need = 4usize.saturating_add((h.count as usize).saturating_mul(4));
    if bytes.len() < need {
        return Err(format!(
            "cmd bo short: have {} need {need} (count={})",
            bytes.len(),
            h.count
        ));
    }
    if h.count < 5 {
        return Err(format!("count {} < 5 (cu+buf+size+prop)", h.count));
    }
    let mut dwords = Vec::with_capacity(h.count as usize);
    for i in 0..h.count as usize {
        let o = 4 + i * 4;
        dwords.push(u32::from_le_bytes([
            bytes[o],
            bytes[o + 1],
            bytes[o + 2],
            bytes[o + 3],
        ]));
    }
    let cu_mask = dwords[0];
    let inst_addr = u64::from(dwords[1]) | (u64::from(dwords[2]) << 32);
    let inst_size = dwords[3];
    let prop_count = dwords[4];
    let flexible_args = dwords[5..].to_vec();
    Ok(StartNpuCmdDecode {
        state: h.state,
        count: h.count,
        opcode: h.opcode,
        cu_mask,
        inst_addr,
        inst_size,
        prop_count,
        flexible_args,
    })
}

/// Rebuild pure START_NPU for C2 live paddrs (xrt3, content size 1688, prop_count=0).
///
/// Research helper — does **not** submit.
///
/// # Errors
///
/// Builder refuse (should not happen for valid paddrs).
pub fn rebuild_c2_live_start_npu_cmd_bo(
    inst_addr: u64,
    inst_size: u32,
    ifm: u64,
    wts: u64,
    ofm: u64,
    cu_mask: u32,
) -> Result<Vec<u8>, String> {
    let kargs =
        pack_start_npu_kernel_args_with_ctrl_layout(ifm, wts, ofm, None, LabKernelArgLayout::Xrt3);
    build_ert_start_npu_cmd_bo_with_kernel_args(inst_addr, inst_size, &kargs, cu_mask)
        .map_err(|e| e.to_string())
}

/// Word-level diff summary (research).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct U32WordDiff {
    /// Length of left slice.
    pub left_len: usize,
    /// Length of right slice.
    pub right_len: usize,
    /// First index where words differ (or min len if prefix-equal but lens differ).
    pub first_mismatch: Option<usize>,
    /// Count of differing positions in shared prefix.
    pub mismatch_count: usize,
}

/// Diff two u32 word lists.
#[must_use]
pub fn diff_u32_word_lists(left: &[u32], right: &[u32]) -> U32WordDiff {
    let n = left.len().min(right.len());
    let mut mismatch_count = 0usize;
    let mut first = None;
    for i in 0..n {
        if left[i] != right[i] {
            mismatch_count += 1;
            if first.is_none() {
                first = Some(i);
            }
        }
    }
    if first.is_none() && left.len() != right.len() {
        first = Some(n);
    }
    U32WordDiff {
        left_len: left.len(),
        right_len: right.len(),
        first_mismatch: first,
        mismatch_count,
    }
}

/// Live C2 journal cmdbuf words (28 × u32) from operator dmesg 2026-07-18.
///
/// Embedded for offline tests without depending on gitignored path layout.
#[must_use]
pub fn c2_live_cmdbuf_words_embedded() -> &'static [u32] {
    &[
        0x0000_0002,
        0x0402_0000,
        0x0000_0000,
        0x0000_0000,
        0x0000_0000,
        0x0000_0000,
        0x0000_0000,
        0x0000_0698,
        0x0000_0000,
        0x0000_0000,
        0x0000_0000,
        0x0000_0000,
        0x0000_000f,
        0x0000_0003,
        0x0000_0000,
        0x0000_0000,
        0x0000_0000,
        0x0000_0000,
        0x3b83_5000,
        0x0000_7fda,
        0x3b66_b000,
        0x0000_7fda,
        0x3b83_4000,
        0x0000_7fda,
        0x0000_0000,
        0x0000_0000,
        0x0000_0000,
        0x0000_0000,
    ]
}

/// Soft research note comparing pure START_NPU rebuild to live C2 cmdbuf words.
///
/// Always includes `not_c_ready=1 not_ran_on_npu=1 invent_pass=0`.
#[must_use]
pub fn format_c2_cmdbuf_research_note(pure_bo: &[u8], live_words: &[u32]) -> String {
    let pure_words: Vec<u32> = pure_bo
        .chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect();
    let d = diff_u32_word_lists(&pure_words, live_words);
    let mut parts = vec![
        "c2_cmdbuf_research".into(),
        "not_c_ready=1".into(),
        "not_ran_on_npu=1".into(),
        "invent_pass=0".into(),
        format!("pure_words={}", pure_words.len()),
        format!("live_words={}", live_words.len()),
    ];
    if let Ok(dec) = decode_start_npu_cmd_bo(pure_bo) {
        parts.push(format!("pure_prop_count={}", dec.prop_count));
        parts.push(format!("pure_inst_size={}", dec.inst_size));
        parts.push(format!("pure_cu_mask={:#x}", dec.cu_mask));
        parts.push(format!("pure_count={}", dec.count));
    }
    // Live markers without full decode (dump framing may differ from pure BO).
    let live_has_size = live_words.contains(&0x698);
    let live_has_n_bos3 =
        live_words.windows(2).any(|w| w[0] == 0xf && w[1] == 3) || live_words.contains(&3);
    parts.push(format!("live_has_size_0x698={}", u8::from(live_has_size)));
    parts.push(format!(
        "live_word12_is_0xf={}",
        u8::from(live_words.get(12) == Some(&0xf))
    ));
    parts.push(format!(
        "live_has_n_bos3_token={}",
        u8::from(live_has_n_bos3)
    ));
    if let Some(i) = d.first_mismatch {
        parts.push(format!("first_mismatch_idx={i}"));
        parts.push(format!("mismatch_count={}", d.mismatch_count));
        parts.push("envelope_exact_match=0".into());
    } else {
        parts.push("envelope_exact_match=1".into());
    }
    // Hypothesis tokens (research, not Pass).
    if live_words.get(12) == Some(&0xf) {
        parts.push("hypothesis_live_prop_count_15_or_framing=1".into());
    }
    if pure_words.len() != live_words.len() {
        parts.push("hypothesis_live_dump_longer_or_padded=1".into());
    }
    parts.join(" ")
}

// ---------------------------------------------------------------------------
// Track F/G — live C2 cmdbuf semantic map + cmdlist msg constants (research)
// ---------------------------------------------------------------------------

/// Live C2 cmdlist slot message size field (`0x70` = 112 B) — matches 28-word cmdbuf.
///
/// Public kernel/driver research constant only; **not** a packing root-cause claim.
pub const C2_LIVE_CMDLIST_MSG_SIZE_BYTES: u32 = 0x70;

/// Expected live C2 cmdbuf dword count (112 B / 4).
pub const C2_LIVE_CMDBUF_WORD_COUNT: usize = 28;

/// Pure shim START_NPU dword count for xrt3 + content 1688 (header + 20 data).
pub const C2_PURE_START_NPU_WORD_COUNT: usize = 21;

/// Word index of instr buffer lo in the locked live C2 cmdbuf dump (research).
pub const C2_LIVE_INSTR_LO_WORD_IDX: usize = 1;
/// Word index of instruction size (`0x698`) in the locked live dump.
pub const C2_LIVE_INSTR_SIZE_WORD_IDX: usize = 7;
/// Word index of research marker `0xf` (≠ invent live `prop_count=15`).
pub const C2_LIVE_WORD12_MARKER_IDX: usize = 12;
/// Word index of n_bos-like token `3` immediately after word12 marker.
pub const C2_LIVE_N_BOS_WORD_IDX: usize = 13;
/// Word index of IFM paddr lo in the locked live dump.
pub const C2_LIVE_IFM_LO_WORD_IDX: usize = 18;

/// Live C2 cmdlist slot message words (6 × u32) from operator dmesg 2026-07-18.
///
/// Embedded for offline tests. Research only — never EXEC / never invent fill_slot body.
#[must_use]
pub fn c2_live_cmdlist_msg_words_embedded() -> &'static [u32] {
    &[
        0x0000_0000,
        0x0000_0000,
        0x0400_0000,
        0x0000_0000,
        0x0000_0070, // size = 0x70 = 112 B (cmdbuf length)
        0x0000_0001,
    ]
}

/// Semantic map of the live 28-word C2 cmdbuf dump (research, not Pass).
///
/// Field indices are **unit-locked** against `c2_live_cmdbuf_words_embedded` and
/// the operator fixture. Framing differs from pure START_NPU (21 words): this is
/// a **cmdlist-slot view**, not proof that pure `prop_count` should change.
///
/// **Ctrl paddr** (`c2_live_io_paddrs_locked` fifth value) is **not** in the
/// live dump (words 24–27 are zero on locked C2) — only instr + IFM/WTS/OFM.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct C2LiveCmdbufMap {
    /// Dword count of the live dump.
    pub live_word_count: usize,
    /// Length delta vs pure START_NPU rebuild (live − pure words). **Not**
    /// proven prefix layout — extra dwords may be suffix/interleave/cmdlist.
    pub suspected_framing_extra_words: usize,
    /// Instruction buffer device address when lo/hi pair present **and** valid.
    pub instr_addr: Option<u64>,
    /// START_NPU / instr size at locked index 7 only (`0x698` == 1688 content).
    pub instr_size: Option<u32>,
    /// Word12 research marker value when present (`0xf` on locked C2 dump).
    pub word12_marker: Option<u32>,
    /// n_bos-like token after word12 (`3` on locked C2 = IFM/WTS/OFM).
    pub n_bos_token: Option<u32>,
    /// IFM host/device paddr when present at locked indices **and** valid.
    pub ifm_paddr: Option<u64>,
    /// WTS paddr when present and valid.
    pub wts_paddr: Option<u64>,
    /// OFM paddr when present and valid.
    pub ofm_paddr: Option<u64>,
    /// True only when **all four** expect fields are `Some` + valid, live paddrs
    /// match them, and instr size is content 1688 (`0x698`). Partial expects
    /// never set this (fail-closed full logical match). Soft note also prints
    /// legacy alias `logical_paddrs_match_pure=` with the same 0/1 value.
    pub logical_paddrs_match_expect: bool,
    /// Envelope byte/word exact match pure vs live (`false` on locked C2).
    pub envelope_exact_match: bool,
    /// True when `pure_bo` was provided but `len % 4 != 0` (envelope forced false).
    pub pure_bo_unaligned: bool,
}

impl C2LiveCmdbufMap {
    /// Soft research note (always honesty tokens; never Pass / C-ready).
    #[must_use]
    pub fn format_soft_note(&self) -> String {
        let mut parts = vec![
            "c2_live_cmdbuf_map".into(),
            "not_c_ready=1".into(),
            "not_ran_on_npu=1".into(),
            "invent_pass=0".into(),
            format!(
                "envelope_exact_match={}",
                u8::from(self.envelope_exact_match)
            ),
            format!("live_words={}", self.live_word_count),
            format!(
                "suspected_framing_extra_words={}",
                self.suspected_framing_extra_words
            ),
            // Framing length delta alias (docs may still say "framing +N words").
            format!("framing_len_delta={}", self.suspected_framing_extra_words),
            format!(
                "logical_paddrs_match_expect={}",
                u8::from(self.logical_paddrs_match_expect)
            ),
            // Legacy alias — same bit as `logical_paddrs_match_expect`.
            format!(
                "logical_paddrs_match_pure={}",
                u8::from(self.logical_paddrs_match_expect)
            ),
        ];
        if self.pure_bo_unaligned {
            parts.push("pure_bo_unaligned=1".into());
        }
        if let Some(a) = self.instr_addr {
            parts.push(format!("instr_addr=0x{a:x}"));
        } else {
            parts.push("instr_addr=absent".into());
        }
        if let Some(sz) = self.instr_size {
            parts.push(format!("instr_size=0x{sz:x}"));
            parts.push(format!("instr_size_dec={sz}"));
        } else {
            parts.push("instr_size=absent".into());
        }
        if let Some(m) = self.word12_marker {
            parts.push(format!("word12_marker=0x{m:x}"));
            if m == 0xf {
                parts.push("word12_is_0xf=1".into());
                // Research-only marker name (not invent live prop_count=15).
                parts.push("word12_marker_0xf_research=1".into());
                parts.push("hypothesis_live_prop_count_15_or_framing=1".into());
                parts.push("do_not_invent_prop_count_15=1".into());
            }
        }
        if let Some(n) = self.n_bos_token {
            parts.push(format!("n_bos_token={n}"));
        }
        if let Some(p) = self.ifm_paddr {
            parts.push(format!("ifm=0x{p:x}"));
        }
        if let Some(p) = self.wts_paddr {
            parts.push(format!("wts=0x{p:x}"));
        }
        if let Some(p) = self.ofm_paddr {
            parts.push(format!("ofm=0x{p:x}"));
        }
        parts.join(" ")
    }
}

/// Read LE u64 at word index `lo` (lo + hi dwords) from a u32 word list.
#[must_use]
fn u64_at_words(words: &[u32], lo_idx: usize) -> Option<u64> {
    let lo = *words.get(lo_idx)?;
    let hi = *words.get(lo_idx + 1)?;
    Some(u64::from(lo) | (u64::from(hi) << 32))
}

/// Optional expected IO paddrs for [`decode_c2_live_cmdbuf_research`] logical match.
///
/// Full logical match (`logical_paddrs_match_expect`) requires **all four**
/// fields `Some` and valid device paddrs. Partial expects never claim full match.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct C2ExpectPaddrs {
    /// Instruction buffer device address.
    pub instr: Option<u64>,
    /// IFM buffer paddr.
    pub ifm: Option<u64>,
    /// WTS buffer paddr.
    pub wts: Option<u64>,
    /// OFM buffer paddr.
    pub ofm: Option<u64>,
}

impl C2ExpectPaddrs {
    /// All four known C2 live values (instr + IFM/WTS/OFM). Ctrl is **not** in
    /// the live cmdbuf dump (words 24–27 zero) — use `c2_live_io_paddrs_locked`.
    #[must_use]
    pub const fn from_c2_locked_io() -> Self {
        let (instr, ifm, wts, ofm, _ctrl) = c2_live_io_paddrs_locked();
        Self {
            instr: Some(instr),
            ifm: Some(ifm),
            wts: Some(wts),
            ofm: Some(ofm),
        }
    }

    /// True when any field is `Some` (does **not** imply full logical match).
    #[must_use]
    pub const fn any(self) -> bool {
        self.instr.is_some() || self.ifm.is_some() || self.wts.is_some() || self.ofm.is_some()
    }

    /// True when all four fields are `Some`.
    #[must_use]
    pub const fn is_complete(self) -> bool {
        self.instr.is_some() && self.ifm.is_some() && self.wts.is_some() && self.ofm.is_some()
    }

    /// True when complete and every present value is a valid device paddr.
    #[must_use]
    pub const fn is_complete_and_valid(self) -> bool {
        match (self.instr, self.ifm, self.wts, self.ofm) {
            (Some(i), Some(a), Some(b), Some(c)) => {
                is_valid_device_paddr(i)
                    && is_valid_device_paddr(a)
                    && is_valid_device_paddr(b)
                    && is_valid_device_paddr(c)
            }
            _ => false,
        }
    }
}

/// Exact paddr equality only when both sides are `Some` (no None="match anything").
#[must_use]
fn opt_paddr_eq(expect: Option<u64>, got: Option<u64>) -> bool {
    match (expect, got) {
        (Some(e), Some(g)) => e == g,
        _ => false,
    }
}

/// Content instr size locked on C2 dump word7 (`0x698` == 1688 decimal).
const C2_LIVE_INSTR_SIZE_CONTENT: u32 = 0x698; // == 1688 content bytes

/// Decode live C2 cmdbuf words into a semantic research map.
///
/// [`C2ExpectPaddrs`] gates `logical_paddrs_match_expect` only when **all four**
/// expects are present + valid and live paddrs (also validated) match + size
/// is content 1688. Never invents paddrs when absent from dump.
///
/// `pure_bo` when provided sets `envelope_exact_match` via word diff; unaligned
/// BO (`len % 4 != 0`) forces `envelope_exact_match=false` + `pure_bo_unaligned`.
#[must_use]
pub fn decode_c2_live_cmdbuf_research(
    live_words: &[u32],
    pure_bo: Option<&[u8]>,
    expect: C2ExpectPaddrs,
) -> C2LiveCmdbufMap {
    let mut m = C2LiveCmdbufMap {
        live_word_count: live_words.len(),
        suspected_framing_extra_words: live_words
            .len()
            .saturating_sub(C2_PURE_START_NPU_WORD_COUNT),
        ..C2LiveCmdbufMap::default()
    };

    // Locked indices from C2 fixture only (short dump → fields stay None).
    if live_words.len() >= C2_LIVE_CMDBUF_WORD_COUNT {
        m.instr_addr =
            u64_at_words(live_words, C2_LIVE_INSTR_LO_WORD_IDX).and_then(some_valid_device_paddr);
        if let Some(&sz) = live_words.get(C2_LIVE_INSTR_SIZE_WORD_IDX) {
            m.instr_size = Some(sz);
        }
        m.word12_marker = live_words.get(C2_LIVE_WORD12_MARKER_IDX).copied();
        m.n_bos_token = live_words.get(C2_LIVE_N_BOS_WORD_IDX).copied();
        m.ifm_paddr =
            u64_at_words(live_words, C2_LIVE_IFM_LO_WORD_IDX).and_then(some_valid_device_paddr);
        m.wts_paddr =
            u64_at_words(live_words, C2_LIVE_IFM_LO_WORD_IDX + 2).and_then(some_valid_device_paddr);
        m.ofm_paddr =
            u64_at_words(live_words, C2_LIVE_IFM_LO_WORD_IDX + 4).and_then(some_valid_device_paddr);
    }
    // No bare `contains(0x698)` short-dump size invent — size only at locked idx.

    // size_ok: content 1688 only (`0x698 == 1688`).
    let size_ok = m.instr_size == Some(C2_LIVE_INSTR_SIZE_CONTENT);
    m.logical_paddrs_match_expect = expect.is_complete_and_valid()
        && opt_paddr_eq(expect.instr, m.instr_addr)
        && opt_paddr_eq(expect.ifm, m.ifm_paddr)
        && opt_paddr_eq(expect.wts, m.wts_paddr)
        && opt_paddr_eq(expect.ofm, m.ofm_paddr)
        && size_ok;

    if let Some(bo) = pure_bo {
        if bo.len() % 4 != 0 {
            m.pure_bo_unaligned = true;
            m.envelope_exact_match = false;
            m.suspected_framing_extra_words = live_words.len().saturating_sub(bo.len() / 4);
        } else {
            let pure_words: Vec<u32> = bo
                .chunks_exact(4)
                .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                .collect();
            let d = diff_u32_word_lists(&pure_words, live_words);
            m.envelope_exact_match = d.first_mismatch.is_none() && d.left_len == d.right_len;
            m.suspected_framing_extra_words = live_words.len().saturating_sub(pure_words.len());
        }
    } else {
        m.envelope_exact_match = false;
    }
    m
}

/// Convenience: map locked embedded live dump vs pure rebuild for C2 paddrs.
///
/// Research helper — does **not** submit. Always soft; never Pass.
#[must_use]
pub fn decode_c2_live_cmdbuf_vs_pure_rebuild(
    pure_bo: &[u8],
    expect: C2ExpectPaddrs,
) -> C2LiveCmdbufMap {
    decode_c2_live_cmdbuf_research(c2_live_cmdbuf_words_embedded(), Some(pure_bo), expect)
}

/// Soft note for cmdlist msg fixture (size field + honesty).
///
/// `msg_size_matches_expected` compares word[4] to [`C2_LIVE_CMDLIST_MSG_SIZE_BYTES`]
/// (`0x70`). Legacy alias `msg_size_matches_cmdbuf=` prints the same bit (the
/// constant equals locked 28×4 live cmdbuf bytes — not a measured live slice).
#[must_use]
pub fn format_c2_cmdlist_msg_research_note(msg_words: &[u32]) -> String {
    let mut parts = vec![
        "c2_cmdlist_msg_research".into(),
        "not_c_ready=1".into(),
        "not_ran_on_npu=1".into(),
        "invent_pass=0".into(),
        format!("msg_words={}", msg_words.len()),
    ];
    // Size field at index 4 in locked fixture.
    if let Some(&sz) = msg_words.get(4) {
        let matches = sz == C2_LIVE_CMDLIST_MSG_SIZE_BYTES;
        parts.push(format!("msg_size_field=0x{sz:x}"));
        parts.push(format!("msg_size_matches_expected={}", u8::from(matches)));
        // Legacy alias (same bit as matches_expected).
        parts.push(format!("msg_size_matches_cmdbuf={}", u8::from(matches)));
        parts.push(format!("cmdbuf_bytes_from_msg={sz}"));
    } else {
        parts.push("msg_size_field=absent".into());
    }
    parts.push(format!(
        "expected_cmdlist_msg_size=0x{C2_LIVE_CMDLIST_MSG_SIZE_BYTES:x}"
    ));
    // Cross-check constant vs locked embed length (research lock).
    parts.push(format!(
        "cmdbuf_embed_bytes={}",
        c2_live_cmdbuf_words_embedded().len() * 4
    ));
    parts.join(" ")
}

// ---------------------------------------------------------------------------
// Track H — offline aiebu rela / ctrltext audit with known live paddrs
// ---------------------------------------------------------------------------

/// Offline rela+ctrltext audit report (userspace patch path only).
///
/// **Never** claims Pass / `ran_on_npu` / C-ready. Soft skip when ELF missing.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct C2RelaPaddrAudit {
    /// True when ELF bytes were provided and looked like aiebu.
    pub elf_present: bool,
    /// `parse_elf32_rela_dyn` count when parsed.
    pub rela_count: Option<usize>,
    /// Patch+classic walk succeeded (overwrite mode).
    pub patched_ctrltext_walk_ok: bool,
    /// Patched ctrltext length when walk ran.
    pub patched_ctrltext_len: Option<usize>,
    /// Patched ctrltext bytes when patch succeeded (even if classic walk failed).
    /// Enables single-patch profile (Track K) without a second rela write.
    pub patched_ctrltext: Option<Vec<u8>>,
    /// Detail / error token (never empty when not ok and elf present).
    pub detail: String,
}

impl C2RelaPaddrAudit {
    /// Soft research note.
    #[must_use]
    pub fn format_soft_note(&self) -> String {
        let mut parts = vec![
            "c2_rela_paddr_audit".into(),
            "not_c_ready=1".into(),
            "not_ran_on_npu=1".into(),
            "invent_pass=0".into(),
            format!("elf_present={}", u8::from(self.elf_present)),
            format!(
                "patched_ctrltext_walk_ok={}",
                u8::from(self.patched_ctrltext_walk_ok)
            ),
        ];
        if let Some(n) = self.rela_count {
            parts.push(format!("rela_count={n}"));
            if n > 0 {
                // Prefer checklist vocabulary; keep short alias for greppers.
                parts.push("userspace_elf_rela_ok=1".into());
                parts.push("userspace_rela_ok=1".into());
            }
        } else {
            parts.push("rela_count=absent".into());
        }
        if let Some(len) = self.patched_ctrltext_len {
            parts.push(format!("patched_ctrltext_len={len}"));
        }
        if self.patched_ctrltext_walk_ok {
            parts.push("userspace_patch_ok_fw_load_still_open=1".into());
        }
        if !self.detail.is_empty() {
            // Sanitize detail for soft-token integrity (no whitespace / `=` / newline).
            let safe: String = self
                .detail
                .chars()
                .map(|c| match c {
                    ' ' | '\t' | '\n' | '\r' | '=' | ';' | '[' | ']' => '_',
                    c if c.is_ascii_graphic() => c,
                    _ => '_',
                })
                .collect();
            parts.push(format!("detail={safe}"));
        }
        parts.join(" ")
    }
}

/// Patch staged aiebu ELF with known live IO paddrs and validate classic ctrltext.
///
/// Offline only. Empty `elf` → soft `elf_present=0` (CI without staged asset).
/// Invalid paddrs (`0` / `u64::MAX`) → refuse patch (`invalid_live_paddr`).
/// Does **not** invent paddrs; caller supplies C2-known values.
#[must_use]
pub fn audit_aiebu_rela_with_live_paddrs(
    elf: &[u8],
    ifm: u64,
    wts: u64,
    ofm: u64,
    ctrl: u64,
) -> C2RelaPaddrAudit {
    if elf.is_empty() {
        return C2RelaPaddrAudit {
            elf_present: false,
            detail: "elf_empty_skip".into(),
            ..C2RelaPaddrAudit::default()
        };
    }
    if !is_aiebu_elf_ctrltext(elf) {
        return C2RelaPaddrAudit {
            elf_present: true,
            detail: "not_aiebu_elf_ctrltext".into(),
            ..C2RelaPaddrAudit::default()
        };
    }
    // Refuse invalid device paddrs before patch (classic walk does not check addrs).
    if !is_valid_device_paddr(ifm)
        || !is_valid_device_paddr(wts)
        || !is_valid_device_paddr(ofm)
        || !is_valid_device_paddr(ctrl)
    {
        let rela_count = parse_elf32_rela_dyn(elf).ok().map(|r| r.len());
        return C2RelaPaddrAudit {
            elf_present: true,
            rela_count,
            patched_ctrltext_walk_ok: false,
            detail: "invalid_live_paddr".into(),
            ..C2RelaPaddrAudit::default()
        };
    }
    let rela_count = parse_elf32_rela_dyn(elf).ok().map(|r| r.len());
    let patched = match patch_aiebu_elf_ctrltext_with_bos_ex(
        elf,
        ifm,
        wts,
        ofm,
        ctrl,
        LabRelaMode::Overwrite,
    ) {
        Ok(p) => p,
        Err(e) => {
            return C2RelaPaddrAudit {
                elf_present: true,
                rela_count,
                patched_ctrltext_walk_ok: false,
                detail: format!("patch_failed:{e}"),
                ..C2RelaPaddrAudit::default()
            };
        }
    };
    let expect_staged = patched.len() == 1688;
    let plen = patched.len();
    match validate_patched_ctrltext_classic(&patched, expect_staged) {
        Ok(_) => C2RelaPaddrAudit {
            elf_present: true,
            rela_count,
            patched_ctrltext_walk_ok: true,
            patched_ctrltext_len: Some(plen),
            patched_ctrltext: Some(patched),
            detail: "classic_walk_ok".into(),
        },
        Err(e) => C2RelaPaddrAudit {
            elf_present: true,
            rela_count,
            patched_ctrltext_walk_ok: false,
            patched_ctrltext_len: Some(plen),
            patched_ctrltext: Some(patched),
            detail: format!("walk_failed:{e}"),
        },
    }
}

/// C2 live IO paddrs from operator last_apply 2026-07-18 (unit-locked research).
///
/// Order: `(instr_xdna, ifm, wts, ofm, ctrl)`.
#[must_use]
pub const fn c2_live_io_paddrs_locked() -> (u64, u64, u64, u64, u64) {
    (
        0x402_0000,
        0x7fda_3b83_5000,
        0x7fda_3b66_b000,
        0x7fda_3b83_4000,
        0x7fda_3b82_c000,
    )
}

// ---------------------------------------------------------------------------
// Track K — instr/PDI content residual (FW-visible load payload after rela)
// ---------------------------------------------------------------------------

/// Pure SHA-256 of `data` as lowercase hex (64 chars). No extra crate dep.
///
/// Research digests only — **not** a Pass token and **not** FW PDI decode.
/// Empty string when [`sha256_digest`] refuses (oversized input).
#[must_use]
pub fn sha256_hex_lower(data: &[u8]) -> String {
    let Some(d) = sha256_digest(data) else {
        return String::new();
    };
    let mut s = String::with_capacity(64);
    for b in d {
        use std::fmt::Write as _;
        let _ = write!(s, "{b:02x}");
    }
    s
}

/// Pure SHA-256 digest (32 bytes). Public for unit locks; prefer [`sha256_hex_lower`].
///
/// Returns `None` when `data` exceeds [`MAX_INSTRUCTION_BLOB_BYTES`] (64 MiB lab
/// ceiling) so callers cannot force multi-GiB clones at this boundary.
#[must_use]
#[allow(
    clippy::too_many_lines,
    clippy::many_single_char_names,
    clippy::needless_range_loop
)]
pub fn sha256_digest(data: &[u8]) -> Option<[u8; 32]> {
    // Minimal FIPS-180-4 SHA-256 (offline research digests; no ring/sha2 dep).
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
    if (data.len() as u64) > MAX_INSTRUCTION_BLOB_BYTES {
        return None;
    }
    let mut h: [u32; 8] = [
        0x6a09_e667,
        0xbb67_ae85,
        0x3c6e_f372,
        0xa54f_f53a,
        0x510e_527f,
        0x9b05_688c,
        0x1f83_d9ab,
        0x5be0_cd19,
    ];
    // wrapping_mul: length already capped at MAX_INSTRUCTION_BLOB_BYTES (<< 2^61).
    let bit_len = (data.len() as u64).wrapping_mul(8);
    let mut msg = data.to_vec();
    msg.push(0x80);
    while (msg.len() % 64) != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());
    debug_assert_eq!(msg.len() % 64, 0);
    for chunk in msg.chunks_exact(64) {
        let mut w = [0_u32; 64];
        for i in 0..16 {
            let j = i * 4;
            w[i] = u32::from_be_bytes([chunk[j], chunk[j + 1], chunk[j + 2], chunk[j + 3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];
        let mut f = h[5];
        let mut g = h[6];
        let mut hh = h[7];
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }
    let mut out = [0_u8; 32];
    for (i, v) in h.iter().enumerate() {
        out[i * 4..(i + 1) * 4].copy_from_slice(&v.to_be_bytes());
    }
    Some(out)
}

/// Classic v0.1 opcode histogram (research profile; not FW private decode).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TxnOpOpcodeProfile {
    /// Count of `XAIE_IO_WRITE` (0).
    pub write: u32,
    /// Count of `XAIE_IO_BLOCKWRITE` (1).
    pub block_write: u32,
    /// Count of `XAIE_IO_BLOCKSET` (2).
    pub block_set: u32,
    /// Count of `XAIE_IO_MASKWRITE` (3).
    pub mask_write: u32,
    /// Count of `XAIE_IO_MASKPOLL` (4).
    pub mask_poll: u32,
    /// Count of `XAIE_IO_NOOP` (5).
    pub noop: u32,
    /// Count of `XAIE_IO_PREEMPT` (6).
    pub preempt: u32,
    /// Count of `XAIE_IO_MASKPOLL_BUSY` (7).
    pub mask_poll_busy: u32,
    /// Count of opcodes **not** in classic `0..=7` (includes custom `>=128` and gaps 8..=127).
    pub custom_or_other: u32,
    /// Total ops counted.
    pub total: u32,
}

impl TxnOpOpcodeProfile {
    /// Build histogram from walked classic ops.
    #[must_use]
    pub fn from_ops(ops: &[super::txn::XaieTxnOp]) -> Self {
        let mut p = Self::default();
        for op in ops {
            p.total = p.total.saturating_add(1);
            match op.opcode {
                0 => p.write = p.write.saturating_add(1),
                1 => p.block_write = p.block_write.saturating_add(1),
                2 => p.block_set = p.block_set.saturating_add(1),
                3 => p.mask_write = p.mask_write.saturating_add(1),
                4 => p.mask_poll = p.mask_poll.saturating_add(1),
                5 => p.noop = p.noop.saturating_add(1),
                6 => p.preempt = p.preempt.saturating_add(1),
                7 => p.mask_poll_busy = p.mask_poll_busy.saturating_add(1),
                _ => p.custom_or_other = p.custom_or_other.saturating_add(1),
            }
        }
        p
    }

    /// Compact soft tokens (`op_write=N …`).
    #[must_use]
    pub fn format_soft_tokens(&self) -> String {
        format!(
            "op_total={} op_write={} op_block_write={} op_block_set={} op_mask_write={} op_mask_poll={} op_noop={} op_preempt={} op_mask_poll_busy={} op_custom_or_other={}",
            self.total,
            self.write,
            self.block_write,
            self.block_set,
            self.mask_write,
            self.mask_poll,
            self.noop,
            self.preempt,
            self.mask_poll_busy,
            self.custom_or_other
        )
    }
}

/// Profile of FW-visible instruction payload bytes (ctrltext / classic txn).
///
/// Built only from public classic walk + SHA-256 of payload bytes. **Never**
/// invents vendor post-patch or private PDI fields.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InstrPayloadProfile {
    /// Payload byte length (ctrltext or raw txn).
    pub payload_len: usize,
    /// Lowercase SHA-256 hex of payload bytes (empty payload → empty string).
    pub payload_sha256_hex: String,
    /// Blob kind token (`classic_txn_v01` / `txn_header_only` / `unknown_raw` / …).
    pub blob_kind: String,
    /// Classic walk succeeded.
    pub classic_walk_ok: bool,
    /// Header `num_ops` when classic or header-only.
    pub txn_num_ops: Option<u32>,
    /// Header `txn_size` when classic or header-only.
    pub txn_size: Option<u32>,
    /// Header `dev_gen` when present.
    pub txn_dev_gen: Option<u8>,
    /// Header cols when present.
    pub txn_num_cols: Option<u8>,
    /// Header rows when present.
    pub txn_num_rows: Option<u8>,
    /// Opcode histogram when classic walk OK.
    pub op_profile: Option<TxnOpOpcodeProfile>,
    /// Staged mladf classic profile (1688 B / 40 ops).
    pub staged_mladf_classic_profile: bool,
}

impl InstrPayloadProfile {
    /// Soft research note (always honesty tokens).
    #[must_use]
    pub fn format_soft_note(&self) -> String {
        let mut parts = vec![
            "instr_payload_profile".into(),
            "not_c_ready=1".into(),
            "not_ran_on_npu=1".into(),
            "invent_pass=0".into(),
            format!("payload_len={}", self.payload_len),
            format!("blob_kind={}", self.blob_kind),
            format!("classic_walk_ok={}", u8::from(self.classic_walk_ok)),
            format!(
                "staged_mladf_classic_profile={}",
                u8::from(self.staged_mladf_classic_profile)
            ),
        ];
        if self.payload_sha256_hex.is_empty() {
            parts.push("payload_sha256=absent".into());
        } else {
            parts.push(format!("payload_sha256={}", self.payload_sha256_hex));
        }
        if let Some(n) = self.txn_num_ops {
            parts.push(format!("txn_num_ops={n}"));
        }
        if let Some(sz) = self.txn_size {
            parts.push(format!("txn_size={sz}"));
        }
        if let Some(g) = self.txn_dev_gen {
            parts.push(format!("txn_dev_gen={g}"));
        }
        if let Some(c) = self.txn_num_cols {
            parts.push(format!("txn_num_cols={c}"));
        }
        if let Some(r) = self.txn_num_rows {
            parts.push(format!("txn_num_rows={r}"));
        }
        if let Some(ref op) = self.op_profile {
            parts.push(op.format_soft_tokens());
        }
        // Public in-tree headers do not expose PDI image decode for APP_LOAD.
        parts.push(format!("pdi_marker_public={PUBLIC_PDI_MARKER_TOKEN}"));
        parts.join(" ")
    }
}

/// Profile raw instruction payload bytes (already peeled ctrltext or classic txn).
///
/// Empty → zeroed profile with `blob_kind=empty`. Does **not** patch ELF.
#[must_use]
pub fn profile_instr_payload_bytes(payload: &[u8]) -> InstrPayloadProfile {
    if payload.is_empty() {
        return InstrPayloadProfile {
            blob_kind: "empty".into(),
            ..InstrPayloadProfile::default()
        };
    }
    if (payload.len() as u64) > MAX_INSTRUCTION_BLOB_BYTES {
        return InstrPayloadProfile {
            payload_len: payload.len(),
            blob_kind: "oversized_refused".into(),
            ..InstrPayloadProfile::default()
        };
    }
    let kind = classify_instruction_blob(payload);
    let mut p = InstrPayloadProfile {
        payload_len: payload.len(),
        payload_sha256_hex: sha256_hex_lower(payload),
        ..InstrPayloadProfile::default()
    };
    match kind {
        InstructionBlobKind::ClassicTxnV01 { header, ops } => {
            p.blob_kind = "classic_txn_v01".into();
            p.classic_walk_ok = true;
            p.txn_num_ops = Some(header.num_ops);
            p.txn_size = Some(header.txn_size);
            p.txn_dev_gen = Some(header.dev_gen);
            p.txn_num_cols = Some(header.num_cols);
            p.txn_num_rows = Some(header.num_rows);
            p.staged_mladf_classic_profile = header.is_staged_mladf_classic_profile()
                && payload.len() == super::txn::STAGED_MLADF_CLASSIC_INSTR_BYTES;
            p.op_profile = Some(TxnOpOpcodeProfile::from_ops(&ops));
        }
        InstructionBlobKind::TxnHeaderOnly { header, .. } => {
            p.blob_kind = "txn_header_only".into();
            p.classic_walk_ok = false;
            p.txn_num_ops = Some(header.num_ops);
            p.txn_size = Some(header.txn_size);
            p.txn_dev_gen = Some(header.dev_gen);
            p.txn_num_cols = Some(header.num_cols);
            p.txn_num_rows = Some(header.num_rows);
            // Mirror classic arm: header profile + payload length (header.validate
            // already ties txn_size to buffer len when classification reached here).
            p.staged_mladf_classic_profile = header.is_staged_mladf_classic_profile()
                && payload.len() == super::txn::STAGED_MLADF_CLASSIC_INSTR_BYTES;
        }
        InstructionBlobKind::Aie2ElfCtrltext { .. } => {
            // Caller should peel; still classify honestly if raw ELF slipped in.
            p.blob_kind = "aie2_elf_ctrltext_unpeeled".into();
            p.classic_walk_ok = false;
        }
        InstructionBlobKind::UnknownRaw { len } => {
            p.blob_kind = "unknown_raw".into();
            p.payload_len = len;
            p.classic_walk_ok = false;
        }
    }
    p
}

/// Full offline research report: what FW load path sees after rela×N + live paddrs.
///
/// Combines [`audit_aiebu_rela_with_live_paddrs`] with payload profile of the
/// **patched** ctrltext. Always soft; never Pass / C-ready / invent vendor.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InstrPdiLoadResearch {
    /// Underlying rela/patch audit.
    pub rela_audit: C2RelaPaddrAudit,
    /// Profile of patched ctrltext when patch+walk produced bytes.
    pub payload: Option<InstrPayloadProfile>,
    /// True when userspace ELF+rela+classic walk OK (FW load residual still open).
    pub userspace_ok_fw_load_open: bool,
    /// Stable detail token.
    pub detail: String,
}

impl InstrPdiLoadResearch {
    /// Soft research note (always honesty + `userspace_ok_fw_load_open=`).
    #[must_use]
    pub fn format_soft_note(&self) -> String {
        let mut parts = vec![
            "instr_pdi_load_research".into(),
            "not_c_ready=1".into(),
            "not_ran_on_npu=1".into(),
            "invent_pass=0".into(),
            format!(
                "userspace_ok_fw_load_open={}",
                u8::from(self.userspace_ok_fw_load_open)
            ),
        ];
        // Embed rela audit key bits without double-prefix spam.
        // Track H token `userspace_patch_ok_fw_load_still_open` = walk_ok alone
        // (stricter `userspace_ok_fw_load_open` also needs rela>0 + classic profile).
        parts.push(format!(
            "elf_present={}",
            u8::from(self.rela_audit.elf_present)
        ));
        parts.push(format!(
            "patched_ctrltext_walk_ok={}",
            u8::from(self.rela_audit.patched_ctrltext_walk_ok)
        ));
        if self.rela_audit.patched_ctrltext_walk_ok {
            parts.push("userspace_patch_ok_fw_load_still_open=1".into());
        } else {
            parts.push("userspace_patch_ok_fw_load_still_open=0".into());
        }
        if let Some(n) = self.rela_audit.rela_count {
            parts.push(format!("rela_count={n}"));
            if n > 0 {
                parts.push("userspace_elf_rela_ok=1".into());
            }
        }
        if let Some(len) = self.rela_audit.patched_ctrltext_len {
            parts.push(format!("patched_ctrltext_len={len}"));
        }
        if let Some(ref pay) = self.payload {
            parts.push(format!("payload_len={}", pay.payload_len));
            parts.push(format!("blob_kind={}", pay.blob_kind));
            parts.push(format!("classic_walk_ok={}", u8::from(pay.classic_walk_ok)));
            if !pay.payload_sha256_hex.is_empty() {
                parts.push(format!("payload_sha256={}", pay.payload_sha256_hex));
            }
            if let Some(n) = pay.txn_num_ops {
                parts.push(format!("txn_num_ops={n}"));
            }
            if let Some(sz) = pay.txn_size {
                parts.push(format!("txn_size={sz}"));
            }
            parts.push(format!(
                "staged_mladf_classic_profile={}",
                u8::from(pay.staged_mladf_classic_profile)
            ));
            if let Some(ref op) = pay.op_profile {
                parts.push(op.format_soft_tokens());
            }
        } else {
            parts.push("payload=absent".into());
        }
        parts.push(format!("pdi_marker_public={PUBLIC_PDI_MARKER_TOKEN}"));
        if !self.detail.is_empty() {
            let safe: String = self
                .detail
                .chars()
                .map(|c| match c {
                    ' ' | '\t' | '\n' | '\r' | '=' | ';' | '[' | ']' => '_',
                    c if c.is_ascii_graphic() => c,
                    _ => '_',
                })
                .collect();
            parts.push(format!("detail={safe}"));
        }
        parts.join(" ")
    }
}

/// Patch aiebu ELF with live IO paddrs and profile the FW-visible ctrltext payload.
///
/// Offline only. Single patch via [`audit_aiebu_rela_with_live_paddrs`] (Overwrite);
/// staged size is inferred inside the audit from patched length. Profiles the
/// **same** patched bytes (no second rela write). Empty ELF → soft skip.
/// Invalid paddrs refuse. Always soft; never Pass / C-ready.
#[must_use]
pub fn profile_instr_payload_after_rela(
    elf: &[u8],
    ifm: u64,
    wts: u64,
    ofm: u64,
    ctrl: u64,
) -> InstrPdiLoadResearch {
    let rela_audit = audit_aiebu_rela_with_live_paddrs(elf, ifm, wts, ofm, ctrl);
    let payload = rela_audit
        .patched_ctrltext
        .as_deref()
        .map(profile_instr_payload_bytes);
    let userspace_ok = rela_audit.patched_ctrltext_walk_ok
        && rela_audit.rela_count.is_some_and(|n| n > 0)
        && payload.as_ref().is_some_and(|p| p.classic_walk_ok);
    let detail = if elf.is_empty() {
        "elf_empty_skip".into()
    } else if userspace_ok {
        "userspace_ok_fw_load_open".into()
    } else if rela_audit.patched_ctrltext_walk_ok {
        "walk_ok_but_no_rela_or_profile".into()
    } else {
        rela_audit.detail.clone()
    };
    InstrPdiLoadResearch {
        userspace_ok_fw_load_open: userspace_ok,
        detail,
        rela_audit,
        payload,
    }
}

/// Convenience: profile with locked C2 live IO paddrs (instr BO not needed for rela).
///
/// Uses [`c2_live_io_paddrs_locked`] IFM/WTS/OFM/ctrl. Research only.
#[must_use]
pub fn profile_instr_payload_after_rela_c2_locked(elf: &[u8]) -> InstrPdiLoadResearch {
    let (_instr, ifm, wts, ofm, ctrl) = c2_live_io_paddrs_locked();
    profile_instr_payload_after_rela(elf, ifm, wts, ofm, ctrl)
}

// ---------------------------------------------------------------------------
// Construction path research — lab (instr-only START_NPU) vs public (CU/PDI)
// ---------------------------------------------------------------------------

/// Offline research: lab construction path vs working public stack (no Pass).
///
/// From [`docs/CONSTRUCTION_MAP_APP_LOAD.md`](../../../docs/CONSTRUCTION_MAP_APP_LOAD.md):
/// public KMQ path loads **CU/PDI via CONFIG_CU** then peels/patches aiebu
/// ctrltext for START_NPU. Pure-direct lab default does instruction/ctrltext
/// only; optional `XDNA2_LAB_PDI_BLOB` may attach CONFIG_CU (research, not
/// proven). Offline research matrix still locks default `lab_has_cu_pdi_config=
/// false` so we never invent a closed gap without live evidence. Hypothesis
/// for residual `0x3000003`, **not** proof / C-ready / Pass / `ran_on_npu`.
///
/// Field bag is intentional research flags (not a product state machine).
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // research flag bag; honesty-first
pub struct ConstructionPathResearch {
    /// True when `elf` peels classic-walkable ctrltext **and** has rela evidence.
    pub lab_has_instr_ctrltext: bool,
    /// Default pure-direct path: **false** (CONFIG_CU only when lab env set live).
    /// Offline research never invents true from machinery alone.
    pub lab_has_cu_pdi_config: bool,
    /// Research fact from open driver/shim: production KMQ path has a CU/PDI step.
    pub public_stack_requires_cu_pdi_step: bool,
    /// `public_stack_requires_cu_pdi_step && !lab_has_cu_pdi_config` (always true today).
    pub construction_gap_cu_pdi: bool,
    /// Peeled ctrltext length when aiebu ELF present (else `None`).
    pub lab_ctrltext_len: Option<usize>,
    /// `.rela.dyn` entry count when parseable (else `None`).
    pub lab_rela_count: Option<usize>,
    /// Stable research detail token (`elf_empty_skip`, `classic_ctrltext_ok`, …).
    pub detail: String,
}

impl Default for ConstructionPathResearch {
    fn default() -> Self {
        // Honesty-safe defaults: gap open (public requires, lab has no CU/PDI).
        Self {
            lab_has_instr_ctrltext: false,
            lab_has_cu_pdi_config: false,
            public_stack_requires_cu_pdi_step: true,
            construction_gap_cu_pdi: true,
            lab_ctrltext_len: None,
            lab_rela_count: None,
            detail: "default_gap_open".into(),
        }
    }
}

impl ConstructionPathResearch {
    /// Soft research note (always honesty trio; never invent Pass / C-ready).
    #[must_use]
    pub fn format_soft_note(&self) -> String {
        let mut parts = vec![
            "construction_path_research".into(),
            "not_c_ready=1".into(),
            "not_ran_on_npu=1".into(),
            "invent_pass=0".into(),
            format!(
                "construction_gap_cu_pdi={}",
                u8::from(self.construction_gap_cu_pdi)
            ),
            format!("lab_has_instr={}", u8::from(self.lab_has_instr_ctrltext)),
            // Always 0 for pure-direct lab — do not invent CONFIG_CU.
            format!(
                "lab_has_cu_pdi_config={}",
                u8::from(self.lab_has_cu_pdi_config)
            ),
            format!(
                "public_stack_cu_pdi_step={}",
                u8::from(self.public_stack_requires_cu_pdi_step)
            ),
        ];
        if let Some(n) = self.lab_ctrltext_len {
            parts.push(format!("lab_ctrltext_len={n}"));
        }
        if let Some(n) = self.lab_rela_count {
            parts.push(format!("lab_rela_count={n}"));
        }
        if !self.detail.is_empty() {
            parts.push(format!(
                "detail={}",
                sanitize_soft_note_field_value(&self.detail)
            ));
        }
        parts.join(" ")
    }
}

/// Research pure-direct lab construction path vs public CU/PDI + CONFIG_CU stack.
///
/// - Empty `elf` → soft skip-ish: no lab instr, gap still **true** (public requires
///   CU/PDI; lab never has it).
/// - Valid aiebu ELF with peelable classic ctrltext + rela → `lab_has_instr_ctrltext`.
/// - `lab_has_cu_pdi_config` is **false** offline (default path; do not invent
///   closed gap from optional env machinery alone).
/// - Always soft; never Pass / C-ready / `ran_on_npu`.
#[must_use]
pub fn research_construction_path_lab_vs_public(elf: &[u8]) -> ConstructionPathResearch {
    // Structural research facts from construction map / open driver+shim.
    // Optional XDNA2_LAB_PDI_BLOB can attach live; offline matrix stays open.
    let public_requires = true;
    let lab_has_cu = false;
    let gap = public_requires && !lab_has_cu;

    if elf.is_empty() {
        return ConstructionPathResearch {
            lab_has_instr_ctrltext: false,
            lab_has_cu_pdi_config: lab_has_cu,
            public_stack_requires_cu_pdi_step: public_requires,
            construction_gap_cu_pdi: gap,
            lab_ctrltext_len: None,
            lab_rela_count: None,
            detail: "elf_empty_skip".into(),
        };
    }

    if !is_aiebu_elf_ctrltext(elf) {
        // Non-aiebu / non-ELF: do not report `lab_rela_count=0` (that reads as
        // "ELF with empty rela table"). Leave `None` until aiebu ctrltext path.
        return ConstructionPathResearch {
            lab_has_instr_ctrltext: false,
            lab_has_cu_pdi_config: lab_has_cu,
            public_stack_requires_cu_pdi_step: public_requires,
            construction_gap_cu_pdi: gap,
            lab_ctrltext_len: None,
            lab_rela_count: None,
            detail: "not_aiebu_elf_ctrltext".into(),
        };
    }

    let rela_count = parse_elf32_rela_dyn(elf).ok().map(|r| r.len());
    // Belt-and-suspenders: after `is_aiebu_elf_ctrltext`, peel should Ok-slice;
    // keep branch if peel API later returns Err without panicking on OOB.
    let Ok(peeled) = super::txn::instruction_payload_bytes(elf) else {
        return ConstructionPathResearch {
            lab_has_instr_ctrltext: false,
            lab_has_cu_pdi_config: lab_has_cu,
            public_stack_requires_cu_pdi_step: public_requires,
            construction_gap_cu_pdi: gap,
            lab_ctrltext_len: None,
            lab_rela_count: rela_count,
            detail: "ctrltext_peel_failed".into(),
        };
    };
    let profile = profile_instr_payload_bytes(&peeled);
    let has_rela = rela_count.is_some_and(|n| n > 0);
    let lab_instr = profile.classic_walk_ok && has_rela;
    let detail = if lab_instr {
        "classic_ctrltext_ok".into()
    } else if profile.classic_walk_ok {
        "classic_walk_ok_no_rela".into()
    } else {
        "ctrltext_not_classic_walkable".into()
    };
    ConstructionPathResearch {
        lab_has_instr_ctrltext: lab_instr,
        lab_has_cu_pdi_config: lab_has_cu,
        public_stack_requires_cu_pdi_step: public_requires,
        construction_gap_cu_pdi: gap,
        lab_ctrltext_len: Some(peeled.len()),
        lab_rela_count: rela_count,
        detail,
    }
}

// ---------------------------------------------------------------------------
// CU/PDI necessity residual matrix (ctrltext-only vs CONFIG_CU identity)
// ---------------------------------------------------------------------------

/// Soft research: why FW `APP_LOAD_PDI_FAIL` may need CU/PDI identity, not
/// ctrltext alone. **Not** proof. **Not** Pass / C-ready / `ran_on_npu`.
///
/// Locks the residual matrix from construction map + locked journal class:
/// - Pure-direct lab **default** is ctrltext-only (optional env CONFIG_CU not
///   counted as closed gap offline).
/// - Production public stack has a CU/PDI step (construction gap open).
/// - Public FW status **name** `APP_LOAD_PDI_FAIL` (`0x3000003`) implies a
///   partition/PDI load step rejected something — status name only, not image
///   decode.
/// - External ctrltext **sha match** alone **never** closes the FW residual
///   offline (match ≠ Pass; lab may still lack CU/PDI).
///
/// Field bag is intentional research flags (not a product state machine).
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // research flag bag; honesty-first
pub struct CuPdiNecessityResearch {
    /// Default lab path is ctrltext-only (true for offline research matrix).
    pub lab_is_ctrltext_only: bool,
    /// Open driver/shim fact: production KMQ path has CU/PDI + CONFIG_CU.
    pub public_has_cu_pdi_step: bool,
    /// Construction gap open (`public && !lab_has_cu`).
    pub construction_gap_open: bool,
    /// Caller/journal says residual class or code is APP_LOAD_PDI_FAIL.
    pub fw_name_app_load_pdi_fail: bool,
    /// Research compare reported external ctrltext sha match (optional input).
    pub external_ctrltext_sha_match: bool,
    /// Always **false**: sha match alone does not close FW residual / invent Pass.
    pub ctrltext_match_alone_closes_fw_residual: bool,
    /// Open **hypothesis** (gap + FW name): CU/PDI identity may still be required.
    /// Not proven root cause; never unlocks C or claims Pass.
    pub cu_pdi_identity_may_be_required: bool,
    /// Stable research detail token.
    pub detail: String,
}

impl Default for CuPdiNecessityResearch {
    fn default() -> Self {
        // Honesty-safe: gap open, match never closes residual, no invent Pass.
        Self {
            lab_is_ctrltext_only: true,
            public_has_cu_pdi_step: true,
            construction_gap_open: true,
            fw_name_app_load_pdi_fail: false,
            external_ctrltext_sha_match: false,
            ctrltext_match_alone_closes_fw_residual: false,
            cu_pdi_identity_may_be_required: false,
            detail: "default_gap_open_no_fw_flag".into(),
        }
    }
}

impl CuPdiNecessityResearch {
    /// Soft research note (always honesty trio; never invent Pass / C-ready).
    #[must_use]
    pub fn format_soft_note(&self) -> String {
        let mut parts = vec![
            "cu_pdi_necessity_research".into(),
            "not_c_ready=1".into(),
            "not_ran_on_npu=1".into(),
            "invent_pass=0".into(),
            "product_known_good=0".into(),
            format!(
                "lab_is_ctrltext_only={}",
                u8::from(self.lab_is_ctrltext_only)
            ),
            format!(
                "public_has_cu_pdi_step={}",
                u8::from(self.public_has_cu_pdi_step)
            ),
            format!(
                "construction_gap_open={}",
                u8::from(self.construction_gap_open)
            ),
            format!(
                "fw_name_app_load_pdi_fail={}",
                u8::from(self.fw_name_app_load_pdi_fail)
            ),
            format!(
                "external_ctrltext_sha_match={}",
                u8::from(self.external_ctrltext_sha_match)
            ),
            // Always 0 in soft notes — ignore field mutation (same honesty pattern
            // as invent_pass=0 / not_c_ready=1). Constructors also lock the field.
            "ctrltext_match_alone_closes_fw_residual=0".into(),
            format!(
                "cu_pdi_identity_may_be_required={}",
                u8::from(self.cu_pdi_identity_may_be_required)
            ),
        ];
        if !self.detail.is_empty() {
            parts.push(format!(
                "detail={}",
                sanitize_soft_note_field_value(&self.detail)
            ));
        }
        parts.join(" ")
    }
}

/// Research CU/PDI necessity residual matrix from construction path + FW flag.
///
/// - `construction` from [`research_construction_path_lab_vs_public`].
/// - `fw_app_load_residual`: residual class / journal is APP_LOAD_PDI_FAIL
///   (caller-supplied; never invents journal codes).
/// - `external_ctrltext_sha_match`: optional soft compare result (research only).
/// - Always `ctrltext_match_alone_closes_fw_residual=false`.
/// - `cu_pdi_identity_may_be_required` only when gap open **and** FW residual.
/// - Soft only; never Pass / C-ready / second unlock.
#[must_use]
pub fn research_cu_pdi_necessity_for_app_load(
    construction: &ConstructionPathResearch,
    fw_app_load_residual: bool,
    external_ctrltext_sha_match: bool,
) -> CuPdiNecessityResearch {
    let lab_ctrltext_only = !construction.lab_has_cu_pdi_config;
    let public_step = construction.public_stack_requires_cu_pdi_step;
    let gap = construction.construction_gap_cu_pdi;
    // Open hypothesis only — not proof that missing CONFIG_CU causes 0x3000003.
    let may_need = gap && fw_app_load_residual;
    let detail = if external_ctrltext_sha_match && fw_app_load_residual {
        "sha_match_fw_residual_still_open"
    } else if fw_app_load_residual && gap {
        "gap_open_fw_app_load"
    } else if gap {
        "gap_open_no_fw_flag"
    } else {
        "no_gap"
    };
    CuPdiNecessityResearch {
        lab_is_ctrltext_only: lab_ctrltext_only,
        public_has_cu_pdi_step: public_step,
        construction_gap_open: gap,
        fw_name_app_load_pdi_fail: fw_app_load_residual,
        external_ctrltext_sha_match,
        // Hard lock: research match never invents residual close / Pass.
        ctrltext_match_alone_closes_fw_residual: false,
        cu_pdi_identity_may_be_required: may_need,
        detail: detail.into(),
    }
}

/// Convenience: CU/PDI necessity for locked APP_LOAD residual + construction gap.
///
/// Uses empty-ELF construction path (gap still open) + `fw_app_load_residual=true`.
/// Optional sha_match for honesty when research compare matches failing corpus.
#[must_use]
pub fn research_cu_pdi_necessity_c2_locked(
    external_ctrltext_sha_match: bool,
) -> CuPdiNecessityResearch {
    let construction = research_construction_path_lab_vs_public(&[]);
    research_cu_pdi_necessity_for_app_load(&construction, true, external_ctrltext_sha_match)
}

// ---------------------------------------------------------------------------
// APP_LOAD residual axis matrix (partition vs CU/PDI vs ctrltext-only)
// ---------------------------------------------------------------------------

/// Soft residual **axis** matrix for FW `APP_LOAD_PDI_FAIL` (`0x3000003`).
///
/// Relates partition status-name surface, CU/PDI construction gap, and lab
/// ctrltext-only path. **Not** proof of root cause. **Not** Pass / C-ready /
/// `ran_on_npu`.
///
/// Locks the offline interpretation matrix (stamp `ac20bc3f`):
/// - Public FW name maps via [`aie2_fw_status_name`] only (no PDI image decode).
/// - Partition surface remains [`PUBLIC_PARTITION_SURFACE_TOKEN`] (`status_name_only`).
/// - Lab is ctrltext-only; public stack has a CU/PDI step → construction gap open.
/// - Ctrltext sha match alone **never** closes residual (hard lock).
/// - Partition status name alone **never** proves CU/PDI root cause (hard lock).
///
/// Field bag is intentional research flags (not a product state machine).
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // research flag bag; honesty-first
pub struct AppLoadResidualAxisMatrix {
    /// Residual class and/or journal maps to APP_LOAD_PDI_FAIL name surface.
    pub axis_fw_status_name_app_load: bool,
    /// Partition load surface depth is status_name_only (always true from builders).
    pub axis_partition_surface_status_name_only: bool,
    /// Pure-direct lab never issues CONFIG_CU (ctrltext / START_NPU only).
    pub axis_lab_ctrltext_only: bool,
    /// Production public stack has CU/PDI + CONFIG_CU construction step.
    pub axis_public_cu_pdi_step: bool,
    /// Construction gap open (`public step && !lab CU`).
    pub axis_construction_gap_open: bool,
    /// Open hypothesis: CU/PDI identity may still be required (gap + FW residual).
    pub axis_cu_pdi_identity_hypothesis_open: bool,
    /// Optional research external ctrltext sha match (compare helper).
    pub axis_external_ctrltext_sha_match: bool,
    /// Always **false**: sha match alone does not close FW residual / invent Pass.
    pub ctrltext_match_alone_closes_fw_residual: bool,
    /// Always **false**: status-name surface alone does not prove root cause.
    pub partition_name_alone_proves_root_cause: bool,
    /// Stable research detail token.
    pub detail: String,
}

impl Default for AppLoadResidualAxisMatrix {
    fn default() -> Self {
        // Honesty-safe: gap open, never invent close/Pass, no FW flag assumed.
        Self {
            axis_fw_status_name_app_load: false,
            axis_partition_surface_status_name_only: true,
            axis_lab_ctrltext_only: true,
            axis_public_cu_pdi_step: true,
            axis_construction_gap_open: true,
            axis_cu_pdi_identity_hypothesis_open: false,
            axis_external_ctrltext_sha_match: false,
            ctrltext_match_alone_closes_fw_residual: false,
            partition_name_alone_proves_root_cause: false,
            detail: "default_gap_open_no_fw_flag".into(),
        }
    }
}

impl AppLoadResidualAxisMatrix {
    /// Soft research note (always honesty trio; never invent Pass / C-ready).
    #[must_use]
    pub fn format_soft_note(&self) -> String {
        let mut parts = vec![
            "app_load_residual_axis_matrix".into(),
            "not_c_ready=1".into(),
            "not_ran_on_npu=1".into(),
            "invent_pass=0".into(),
            "product_known_good=0".into(),
            format!(
                "axis_fw_status_name_app_load={}",
                u8::from(self.axis_fw_status_name_app_load)
            ),
            // Always status_name_only surface — force soft token (ignore bag mutation).
            "axis_partition_surface_status_name_only=1".into(),
            format!(
                "axis_lab_ctrltext_only={}",
                u8::from(self.axis_lab_ctrltext_only)
            ),
            format!(
                "axis_public_cu_pdi_step={}",
                u8::from(self.axis_public_cu_pdi_step)
            ),
            format!(
                "axis_construction_gap_open={}",
                u8::from(self.axis_construction_gap_open)
            ),
            format!(
                "axis_cu_pdi_identity_hypothesis_open={}",
                u8::from(self.axis_cu_pdi_identity_hypothesis_open)
            ),
            format!(
                "axis_external_ctrltext_sha_match={}",
                u8::from(self.axis_external_ctrltext_sha_match)
            ),
            // Hard-forced honesty locks (ignore field mutation in soft notes).
            "ctrltext_match_alone_closes_fw_residual=0".into(),
            "partition_name_alone_proves_root_cause=0".into(),
            format!(
                "partition_surface_public={}",
                PUBLIC_PARTITION_SURFACE_TOKEN
            ),
        ];
        // Emit FW code/name only when the FW axis is actually open — never invent
        // APP_LOAD_PDI_FAIL narrative from gap-alone / no-fw-flag paths.
        if self.axis_fw_status_name_app_load {
            parts.push(format!("fw_code=0x{AIE2_FW_APP_LOAD_PDI_FAIL:x}"));
            parts.push(format!(
                "fw_name={}",
                aie2_fw_status_name(AIE2_FW_APP_LOAD_PDI_FAIL)
            ));
        } else {
            parts.push("fw_code=absent".into());
        }
        if !self.detail.is_empty() {
            parts.push(format!(
                "detail={}",
                sanitize_soft_note_field_value(&self.detail)
            ));
        }
        parts.join(" ")
    }
}

/// Build residual axis matrix from partition research + CU/PDI necessity bags.
///
/// - Surface tokens always status_name_only / match-alone=0 / name-alone≠proof.
/// - `axis_fw_status_name_app_load` from partition class/pair **or** necessity FW flag.
/// - Soft only; never Pass / C-ready / second unlock.
#[must_use]
pub fn research_app_load_residual_axis_matrix(
    partition: &AppLoadPdiPartitionResearch,
    necessity: &CuPdiNecessityResearch,
) -> AppLoadResidualAxisMatrix {
    let fw_name = partition.class_app_load_after_aiebu
        || partition.c2_app_load_pair
        || partition.failed_cmd_status == Some(AIE2_FW_APP_LOAD_PDI_FAIL)
        || necessity.fw_name_app_load_pdi_fail;
    let detail = if necessity.external_ctrltext_sha_match && fw_name {
        "sha_match_fw_residual_axes_open"
    } else if fw_name && necessity.construction_gap_open {
        "gap_open_fw_app_load_axes"
    } else if necessity.construction_gap_open {
        "gap_open_no_fw_flag"
    } else {
        "no_gap"
    };
    AppLoadResidualAxisMatrix {
        axis_fw_status_name_app_load: fw_name,
        // Always status_name_only from research constructors — never invent decode.
        axis_partition_surface_status_name_only: true,
        axis_lab_ctrltext_only: necessity.lab_is_ctrltext_only,
        axis_public_cu_pdi_step: necessity.public_has_cu_pdi_step,
        axis_construction_gap_open: necessity.construction_gap_open,
        axis_cu_pdi_identity_hypothesis_open: necessity.cu_pdi_identity_may_be_required,
        axis_external_ctrltext_sha_match: necessity.external_ctrltext_sha_match,
        ctrltext_match_alone_closes_fw_residual: false,
        partition_name_alone_proves_root_cause: false,
        detail: detail.into(),
    }
}

/// Convenience: locked C2 APP_LOAD residual + construction gap axis matrix.
///
/// Uses Track L c2-locked partition (userspace content green) + CU/PDI necessity
/// with `fw_app_load_residual=true`. Optional sha_match for honesty when research
/// compare matches failing corpus — still never closes residual.
#[must_use]
pub fn research_app_load_residual_axis_matrix_c2_locked(
    external_ctrltext_sha_match: bool,
) -> AppLoadResidualAxisMatrix {
    let partition = research_app_load_pdi_partition_c2_locked(true);
    let necessity = research_cu_pdi_necessity_c2_locked(external_ctrltext_sha_match);
    research_app_load_residual_axis_matrix(&partition, &necessity)
}

// ---------------------------------------------------------------------------
// Xclbin extract research dry-run (public markers only; VE2 refuse)
// ---------------------------------------------------------------------------

/// Public AXLF magic (`xclbin2` + NUL). Research marker only — not a full decoder.
pub const XCLBIN2_MAGIC: &[u8; 8] = b"xclbin2\0";

/// Public section **name** markers from docs / `xclbinutil --info` text.
///
/// Scan is case-sensitive on these tokens as printed by XRT tools. This is **not**
/// a private PDI layout map and does **not** decode section bodies.
pub const XCLBIN_RESEARCH_SECTION_MARKERS: &[&str] = &[
    "AIE_PARTITION",
    "BITSTREAM",
    "MEM_TOPOLOGY",
    "IP_LAYOUT",
    "CONNECTIVITY",
    "BUILD_METADATA",
    "EMBEDDED_METADATA",
    "SYSTEM_METADATA",
    "GROUP_CONNECTIVITY",
    "GROUP_TOPOLOGY",
];

/// Soft xclbin extract **dry-run** (public markers only; VE2 refuse).
///
/// Validates magic + section-list research markers without treating VE2 /
/// NOT_AIE2 fixtures as AIE2 known-good and without inventing a PDI field map.
/// **Not** Pass / C-ready / `ran_on_npu`.
///
/// Stamp `ac20bc3f`: prepare offline extract path for when a working public
/// AIE2 xclbin exists; unit-lock refuse of VE2 edge `validate.xclbin`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // research flag bag; honesty-first
pub struct XclbinExtractResearchDryRun {
    /// First 8 bytes match [`XCLBIN2_MAGIC`].
    pub magic_xclbin2: bool,
    /// Count of public section name markers found in inspect/info text (capped).
    pub section_marker_count: u8,
    /// Text listed `AIE_PARTITION` (public name only).
    pub has_aie_partition_marker: bool,
    /// Platform claim / path marks VE2 or NOT_AIE2 known-good refuse.
    pub platform_not_aie2_known_good: bool,
    /// Research placement candidate for AIE2 external bag (never product Pass).
    /// Always **false** when VE2/NOT_AIE2; also false without working-stack claim.
    pub eligible_as_aie2_external_known_good: bool,
    /// Always **true**: no private PDI layout invent.
    pub do_not_invent_pdi_layout: bool,
    /// Always **false**: public image decode remains closed.
    pub public_pdi_image_decode: bool,
    /// Stable research detail token.
    pub detail: String,
}

impl Default for XclbinExtractResearchDryRun {
    fn default() -> Self {
        Self {
            magic_xclbin2: false,
            section_marker_count: 0,
            has_aie_partition_marker: false,
            platform_not_aie2_known_good: false,
            eligible_as_aie2_external_known_good: false,
            do_not_invent_pdi_layout: true,
            public_pdi_image_decode: false,
            detail: "default_no_input".into(),
        }
    }
}

impl XclbinExtractResearchDryRun {
    /// Soft research note (always honesty trio; never invent Pass / product known-good).
    #[must_use]
    pub fn format_soft_note(&self) -> String {
        let mut parts = vec![
            "xclbin_extract_research_dry_run".into(),
            "not_c_ready=1".into(),
            "not_ran_on_npu=1".into(),
            "invent_pass=0".into(),
            "product_known_good=0".into(),
            // Force honesty locks even if a hand-built bag mutates fields.
            "do_not_invent_pdi_layout=1".into(),
            "public_pdi_image_decode=0".into(),
            format!("magic_xclbin2={}", u8::from(self.magic_xclbin2)),
            format!("section_marker_count={}", self.section_marker_count),
            format!(
                "has_aie_partition_marker={}",
                u8::from(self.has_aie_partition_marker)
            ),
            format!(
                "platform_not_aie2_known_good={}",
                u8::from(self.platform_not_aie2_known_good)
            ),
            // Eligible is research bag candidate only — soft always product=0.
            format!(
                "eligible_as_aie2_external_known_good={}",
                u8::from(self.eligible_as_aie2_external_known_good)
            ),
        ];
        if !self.detail.is_empty() {
            parts.push(format!(
                "detail={}",
                sanitize_soft_note_field_value(&self.detail)
            ));
        }
        parts.join(" ")
    }
}

/// True when `bytes` begin with public AXLF magic [`XCLBIN2_MAGIC`].
#[must_use]
pub fn xclbin_magic_is_xclbin2(bytes: &[u8]) -> bool {
    bytes.len() >= XCLBIN2_MAGIC.len() && bytes[..XCLBIN2_MAGIC.len()] == *XCLBIN2_MAGIC
}

/// Scan inspect/info **text** for public section name markers (no binary decode).
///
/// Returns distinct markers from [`XCLBIN_RESEARCH_SECTION_MARKERS`] found as
/// substrings. Empty / whitespace text → empty list.
#[must_use]
pub fn scan_xclbin_section_markers_from_text(text: &str) -> Vec<&'static str> {
    let t = text.trim();
    if t.is_empty() {
        return Vec::new();
    }
    let mut found = Vec::new();
    for m in XCLBIN_RESEARCH_SECTION_MARKERS {
        if t.contains(m) {
            found.push(*m);
        }
    }
    found
}

/// True when platform claim / path / PROVENANCE marks **not** AIE2 known-good.
///
/// Matches (ascii-lower) whole-token or documented markers:
/// - `not_aie2_known_good` / `not_aie2_known_good=1`
/// - `ve2_edge` / `ve2_latency` / path-ish `ve2`
/// - `claim=not_known_good_for_aie2`
///
/// Case-insensitive. Does **not** invent AIE2 eligibility from absence alone.
#[must_use]
pub fn platform_claim_is_not_aie2_known_good(claim: &str) -> bool {
    let lower = claim.to_ascii_lowercase();
    if lower.trim().is_empty() {
        return false;
    }
    // Prefer exact-token honesty for product-ish keys; substrings for known markers.
    if lower.contains("not_aie2_known_good")
        || lower.contains("not_known_good_for_aie2")
        || lower.contains("ve2_edge")
        || lower.contains("ve2_latency")
        || lower.contains("shim_ve2")
    {
        return true;
    }
    // Whole-token `ve2` (avoid matching random substrings like "carve2").
    lower
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .any(|tok| tok == "ve2")
}

/// True when claim marks a **working public AIE2** research source (still not Pass).
///
/// Requires **both** a positive working/source-stack marker **and** an AIE2-ish
/// whole-token (`xrt` / `aie2` / `npu4` / `strix`). Bare single-token `xrt`
/// alone does **not** qualify (avoids over-flipping eligibility on thin claims).
/// Must **not** be NOT_AIE2/VE2 refuse. Research eligibility only — never Pass.
#[must_use]
pub fn platform_claim_is_working_public_aie2_research(claim: &str) -> bool {
    if platform_claim_is_not_aie2_known_good(claim) {
        return false;
    }
    let lower = claim.to_ascii_lowercase();
    let has_working = lower.contains("working public")
        || lower.contains("working_public")
        || lower.contains("public stack")
        || lower.contains("source stack")
        || lower.contains("ryzen ai")
        || lower.contains("dynamicdispatch");
    // Whole-token xrt / aie2 (not substring of larger opaque words alone).
    let has_aie2ish = lower
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .any(|tok| tok == "xrt" || tok == "aie2" || tok == "npu4" || tok == "strix");
    // Both required: refuse bare "xrt" / "aie2" without working/source-stack context.
    has_working && has_aie2ish
}

/// Research dry-run for xclbin extract markers (public text + magic only).
///
/// - `header_bytes`: optional leading bytes for magic check (may be empty).
/// - `section_or_info_text`: optional `xclbinutil --info` / sections paste.
/// - `platform_claim`: optional PROVENANCE / path / PLATFORM line (VE2 refuse).
/// - Never invents PDI layout. Never product known-good / Pass / C-ready.
/// - `eligible_as_aie2_external_known_good` only when magic + AIE_PARTITION marker
///   + working public AIE2 claim and **not** NOT_AIE2/VE2.
#[must_use]
pub fn research_xclbin_extract_dry_run(
    header_bytes: &[u8],
    section_or_info_text: Option<&str>,
    platform_claim: Option<&str>,
) -> XclbinExtractResearchDryRun {
    let magic = xclbin_magic_is_xclbin2(header_bytes);
    let markers = section_or_info_text
        .map(scan_xclbin_section_markers_from_text)
        .unwrap_or_default();
    // Public marker set is tiny; min(255) makes truncation impossible.
    #[allow(clippy::cast_possible_truncation)]
    let marker_count = markers.len().min(255) as u8;
    let has_aie = markers.contains(&"AIE_PARTITION");
    let claim = platform_claim.unwrap_or("").trim();
    let not_aie2 = !claim.is_empty() && platform_claim_is_not_aie2_known_good(claim);
    let working = !claim.is_empty() && platform_claim_is_working_public_aie2_research(claim);
    // Research bag candidate only — still product_known_good=0 in soft notes.
    let eligible = magic && has_aie && working && !not_aie2;
    let detail = if not_aie2 {
        "refuse_not_aie2_or_ve2"
    } else if header_bytes.is_empty() && section_or_info_text.is_none() && claim.is_empty() {
        "default_no_input"
    } else if !magic && !header_bytes.is_empty() {
        "magic_not_xclbin2"
    } else if eligible {
        "research_eligible_not_product_pass"
    } else if magic && has_aie {
        "markers_ok_awaiting_working_stack_claim"
    } else if magic {
        "magic_ok_markers_incomplete"
    } else {
        "awaiting_bytes_or_markers"
    };
    XclbinExtractResearchDryRun {
        magic_xclbin2: magic,
        section_marker_count: marker_count,
        has_aie_partition_marker: has_aie,
        platform_not_aie2_known_good: not_aie2,
        eligible_as_aie2_external_known_good: eligible,
        do_not_invent_pdi_layout: true,
        public_pdi_image_decode: false,
        detail: detail.into(),
    }
}

/// Locked refuse path for staged VE2 edge `validate.xclbin` research fixtures.
///
/// Uses VE2 PLATFORM claim + optional magic from provided header bytes.
/// Always `eligible_as_aie2_external_known_good=false` and
/// `platform_not_aie2_known_good=true`. Soft only; never invent Pass.
#[must_use]
pub fn research_xclbin_extract_ve2_fixture_refused(
    header_bytes: &[u8],
    section_or_info_text: Option<&str>,
) -> XclbinExtractResearchDryRun {
    research_xclbin_extract_dry_run(
        header_bytes,
        section_or_info_text,
        Some("PLATFORM=VE2_EDGE NOT_AIE2_KNOWN_GOOD=1 claim=not_known_good_for_aie2_17f0"),
    )
}

// ---------------------------------------------------------------------------
// Pure CU/PDI section identity (public xclbin section **names** only)
// ---------------------------------------------------------------------------

/// Public xclbin section **names** that form the CU/PDI identity surface.
///
/// From docs / `xclbinutil --info` only — **not** a PDI body layout map.
/// - `AIE_PARTITION` — partition table / load surface name
/// - `BITSTREAM` — public name for the bitstream/PDI payload section
///
/// Presence of these names is research identity for extract recipes. It is
/// **not** product known-good, Pass, or residual closed.
pub const XCLBIN_CU_PDI_IDENTITY_SECTION_MARKERS: &[&str] = &["AIE_PARTITION", "BITSTREAM"];

/// Public topology / connectivity section names (not CU/PDI identity alone).
pub const XCLBIN_TOPOLOGY_SECTION_MARKERS: &[&str] = &[
    "MEM_TOPOLOGY",
    "IP_LAYOUT",
    "CONNECTIVITY",
    "GROUP_CONNECTIVITY",
    "GROUP_TOPOLOGY",
];

/// Public metadata section names (build / embed / system — not CU/PDI identity).
pub const XCLBIN_METADATA_SECTION_MARKERS: &[&str] =
    &["BUILD_METADATA", "EMBEDDED_METADATA", "SYSTEM_METADATA"];

/// Kind of a public xclbin section **name** for CU/PDI identity research.
///
/// Classifies documented section tokens only. Does **not** decode section bodies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XclbinSectionKind {
    /// CU/PDI identity surface (`AIE_PARTITION` / `BITSTREAM`).
    CuPdiIdentity,
    /// Memory / IP / connectivity topology.
    Topology,
    /// Build / embedded / system metadata.
    Metadata,
    /// Not in the public research marker tables.
    Unknown,
}

/// Classify a public section **name** (exact match against research marker tables).
///
/// Case-sensitive to match `xclbinutil` print tokens. Unknown / empty →
/// [`XclbinSectionKind::Unknown`].
#[must_use]
pub fn classify_xclbin_section_kind(name: &str) -> XclbinSectionKind {
    let t = name.trim();
    if t.is_empty() {
        return XclbinSectionKind::Unknown;
    }
    if XCLBIN_CU_PDI_IDENTITY_SECTION_MARKERS.contains(&t) {
        return XclbinSectionKind::CuPdiIdentity;
    }
    if XCLBIN_TOPOLOGY_SECTION_MARKERS.contains(&t) {
        return XclbinSectionKind::Topology;
    }
    if XCLBIN_METADATA_SECTION_MARKERS.contains(&t) {
        return XclbinSectionKind::Metadata;
    }
    XclbinSectionKind::Unknown
}

/// Soft research: which public xclbin section **names** form CU/PDI identity.
///
/// Advances extract recipe readiness without inventing PDI body layout or Pass.
/// Stamp `7a5ab864`: pure section-name / kind unit locks only.
///
/// Honesty:
/// - Name surface only — `do_not_invent_pdi_layout=1` / `public_pdi_image_decode=0`
/// - Section identity alone **never** closes FW residual
/// - VE2 / NOT_AIE2 still refused as product known-good (caller dry-run)
/// - Always `invent_pass=0 not_c_ready=1 not_ran_on_npu=1 product_known_good=0`
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // research flag bag; honesty-first
pub struct CuPdiSectionIdentityResearch {
    /// Public `AIE_PARTITION` name marker present.
    pub has_aie_partition: bool,
    /// Public `BITSTREAM` name marker present.
    pub has_bitstream: bool,
    /// Count of CU/PDI identity markers found (capped).
    pub cu_pdi_identity_marker_count: u8,
    /// Count of topology markers found (capped).
    pub topology_marker_count: u8,
    /// Count of metadata markers found (capped).
    pub metadata_marker_count: u8,
    /// True when ≥1 CU/PDI identity name is present (name surface only).
    pub has_cu_pdi_identity_surface: bool,
    /// Always **false**: section names alone never close FW residual / invent Pass.
    pub section_identity_alone_closes_fw_residual: bool,
    /// Always **true**: no private PDI layout invent.
    pub do_not_invent_pdi_layout: bool,
    /// Always **false**: public image decode remains closed.
    pub public_pdi_image_decode: bool,
    /// Stable research detail token.
    pub detail: String,
}

impl Default for CuPdiSectionIdentityResearch {
    fn default() -> Self {
        Self {
            has_aie_partition: false,
            has_bitstream: false,
            cu_pdi_identity_marker_count: 0,
            topology_marker_count: 0,
            metadata_marker_count: 0,
            has_cu_pdi_identity_surface: false,
            section_identity_alone_closes_fw_residual: false,
            do_not_invent_pdi_layout: true,
            public_pdi_image_decode: false,
            detail: "default_no_markers".into(),
        }
    }
}

impl CuPdiSectionIdentityResearch {
    /// Soft research note (always honesty trio; never invent Pass / product known-good).
    #[must_use]
    pub fn format_soft_note(&self) -> String {
        let mut parts = vec![
            "cu_pdi_section_identity_research".into(),
            "not_c_ready=1".into(),
            "not_ran_on_npu=1".into(),
            "invent_pass=0".into(),
            "product_known_good=0".into(),
            // Force honesty locks even if a hand-built bag mutates fields.
            "do_not_invent_pdi_layout=1".into(),
            "public_pdi_image_decode=0".into(),
            "section_identity_alone_closes_fw_residual=0".into(),
            format!("has_aie_partition={}", u8::from(self.has_aie_partition)),
            format!("has_bitstream={}", u8::from(self.has_bitstream)),
            format!(
                "cu_pdi_identity_marker_count={}",
                self.cu_pdi_identity_marker_count
            ),
            format!("topology_marker_count={}", self.topology_marker_count),
            format!("metadata_marker_count={}", self.metadata_marker_count),
            format!(
                "has_cu_pdi_identity_surface={}",
                u8::from(self.has_cu_pdi_identity_surface)
            ),
        ];
        if !self.detail.is_empty() {
            parts.push(format!(
                "detail={}",
                sanitize_soft_note_field_value(&self.detail)
            ));
        }
        parts.join(" ")
    }
}

/// Research CU/PDI section identity from already-scanned public markers.
///
/// Markers should come from [`scan_xclbin_section_markers_from_text`] (or the
/// same public name table). Names are **trimmed** before kind classify and
/// identity flags (consistent with [`classify_xclbin_section_kind`]). Never
/// invents section bodies / PDI layout / Pass.
#[must_use]
pub fn research_cu_pdi_section_identity_from_markers(
    markers: &[&str],
) -> CuPdiSectionIdentityResearch {
    let mut has_aie = false;
    let mut has_bs = false;
    let mut cu_n = 0_u8;
    let mut topo_n = 0_u8;
    let mut meta_n = 0_u8;
    let mut saw_any_nonempty = false;
    for m in markers {
        let t = m.trim();
        if t.is_empty() {
            continue;
        }
        saw_any_nonempty = true;
        match classify_xclbin_section_kind(t) {
            XclbinSectionKind::CuPdiIdentity => {
                cu_n = cu_n.saturating_add(1);
                // Use trimmed token (same as classify) so padded markers stay consistent.
                if t == "AIE_PARTITION" {
                    has_aie = true;
                }
                if t == "BITSTREAM" {
                    has_bs = true;
                }
            }
            XclbinSectionKind::Topology => {
                topo_n = topo_n.saturating_add(1);
            }
            XclbinSectionKind::Metadata => {
                meta_n = meta_n.saturating_add(1);
            }
            XclbinSectionKind::Unknown => {}
        }
    }
    let has_surface = has_aie || has_bs;
    let detail = if !saw_any_nonempty {
        "default_no_markers"
    } else if has_aie && has_bs {
        "cu_pdi_identity_surface_aie_and_bitstream"
    } else if has_aie {
        "cu_pdi_identity_surface_aie_partition_only"
    } else if has_bs {
        "cu_pdi_identity_surface_bitstream_only"
    } else if topo_n > 0 || meta_n > 0 {
        "topology_or_metadata_only_no_cu_pdi_identity"
    } else {
        "markers_unclassified"
    };
    CuPdiSectionIdentityResearch {
        has_aie_partition: has_aie,
        has_bitstream: has_bs,
        cu_pdi_identity_marker_count: cu_n,
        topology_marker_count: topo_n,
        metadata_marker_count: meta_n,
        has_cu_pdi_identity_surface: has_surface,
        section_identity_alone_closes_fw_residual: false,
        do_not_invent_pdi_layout: true,
        public_pdi_image_decode: false,
        detail: detail.into(),
    }
}

/// Research CU/PDI section identity from `xclbinutil --info` / sections paste text.
///
/// Empty / whitespace text → default no-markers bag. Soft only; never Pass.
#[must_use]
pub fn research_cu_pdi_section_identity_from_text(text: &str) -> CuPdiSectionIdentityResearch {
    let markers = scan_xclbin_section_markers_from_text(text);
    research_cu_pdi_section_identity_from_markers(&markers)
}

// ---------------------------------------------------------------------------
// Pure BITSTREAM / AIE_PARTITION section-offset map (public axlf layout)
// ---------------------------------------------------------------------------

/// Public `axlf_section_kind` value for `BITSTREAM` (`xrt/detail/xclbin.h`).
pub const AXLF_SECTION_KIND_BITSTREAM: u32 = 0;

/// Public `axlf_section_kind` value for `AIE_PARTITION` (`xrt/detail/xclbin.h`).
pub const AXLF_SECTION_KIND_AIE_PARTITION: u32 = 32;

/// Size of public `struct axlf_section_header` (natural alignment; 40 bytes).
///
/// Layout (little-endian, 64-bit host natural align as shipped in XRT headers):
/// - `m_sectionKind` @ 0 (`u32`)
/// - `m_sectionName` @ 4 (`[u8; 16]`)
/// - pad 4 @ 20 (align `u64`)
/// - `m_sectionOffset` @ 24 (`u64`)
/// - `m_sectionSize` @ 32 (`u64`)
pub const AXLF_SECTION_HEADER_SIZE: usize = 40;

/// Size of public `struct axlf_header` (152 bytes including trailing pad).
pub const AXLF_HEADER_SIZE: usize = 152;

/// Absolute file offset of `axlf.m_header` (after magic / signature / keyBlock / uniqueId).
///
/// `8 + 4 + 28 + 256 + 8 = 304`.
pub const AXLF_M_HEADER_ABS_OFFSET: usize = 304;

/// Absolute file offset of `axlf.m_header.m_numSections` (`m_header` + 144).
pub const AXLF_NUM_SECTIONS_ABS_OFFSET: usize = AXLF_M_HEADER_ABS_OFFSET + 144;

/// Absolute file offset of the first `axlf_section_header` (`m_header` + 152).
pub const AXLF_FIRST_SECTION_HEADER_ABS_OFFSET: usize = AXLF_M_HEADER_ABS_OFFSET + AXLF_HEADER_SIZE;

/// Cap on section headers walked offline (research; full XRT max is larger).
pub const AXLF_MAX_SECTIONS_WALK: u32 = 256;

/// Parsed public `axlf_section_header` fields (offset/size only — no body decode).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AxlfSectionHeaderFields {
    /// `m_sectionKind` (public enum value).
    pub kind: u32,
    /// `m_sectionName` as UTF-8 lossy, cut at first NUL (not whitespace-trimmed).
    /// Trailing spaces without a NUL remain. Kind detection uses `m_sectionKind` only.
    pub name: String,
    /// `m_sectionOffset` — file offset of section **data** (not header).
    pub offset: u64,
    /// `m_sectionSize` — section data byte length.
    pub size: u64,
}

/// Soft research: file offsets of public BITSTREAM / AIE_PARTITION sections.
///
/// Stamp `a5787723`: pure public axlf section-header field map only.
/// Does **not** decode PDI/bitstream bodies. Does **not** invent product
/// known-good / Pass / residual closed.
///
/// Honesty:
/// - Offset map alone **never** closes FW residual
/// - Always `do_not_invent_pdi_layout=1` / `public_pdi_image_decode=0`
/// - Always `invent_pass=0 not_c_ready=1 not_ran_on_npu=1 product_known_good=0`
/// - VE2 / NOT_AIE2 fixtures may parse cleanly and still are **not** AIE2 known-good
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // research flag bag; honesty-first
pub struct CuPdiSectionOffsetMapResearch {
    /// First 8 bytes match [`XCLBIN2_MAGIC`].
    pub magic_xclbin2: bool,
    /// `m_numSections` when header long enough to read.
    pub num_sections: Option<u32>,
    /// How many section headers were successfully parsed (capped walk).
    pub section_headers_parsed: u32,
    /// File offset of first `BITSTREAM` section data when present.
    pub bitstream_offset: Option<u64>,
    /// Size of first `BITSTREAM` section data when present.
    pub bitstream_size: Option<u64>,
    /// File offset of first `AIE_PARTITION` section data when present.
    pub aie_partition_offset: Option<u64>,
    /// Size of first `AIE_PARTITION` section data when present.
    pub aie_partition_size: Option<u64>,
    /// True when a BITSTREAM section header was found.
    pub has_bitstream_section: bool,
    /// True when an AIE_PARTITION section header was found.
    pub has_aie_partition_section: bool,
    /// True when ≥1 of BITSTREAM / AIE_PARTITION section headers is present.
    pub has_cu_pdi_offset_surface: bool,
    /// Always **false**: offset map alone never closes FW residual / invent Pass.
    pub section_offset_map_alone_closes_fw_residual: bool,
    /// Always **true**: no private PDI layout invent.
    pub do_not_invent_pdi_layout: bool,
    /// Always **false**: public image decode remains closed.
    pub public_pdi_image_decode: bool,
    /// Stable research detail token.
    pub detail: String,
}

impl Default for CuPdiSectionOffsetMapResearch {
    fn default() -> Self {
        Self {
            magic_xclbin2: false,
            num_sections: None,
            section_headers_parsed: 0,
            bitstream_offset: None,
            bitstream_size: None,
            aie_partition_offset: None,
            aie_partition_size: None,
            has_bitstream_section: false,
            has_aie_partition_section: false,
            has_cu_pdi_offset_surface: false,
            section_offset_map_alone_closes_fw_residual: false,
            do_not_invent_pdi_layout: true,
            public_pdi_image_decode: false,
            detail: "default_empty_bytes".into(),
        }
    }
}

impl CuPdiSectionOffsetMapResearch {
    /// Soft research note (always honesty trio; never invent Pass / product known-good).
    #[must_use]
    pub fn format_soft_note(&self) -> String {
        let mut parts = vec![
            "cu_pdi_section_offset_map_research".into(),
            "not_c_ready=1".into(),
            "not_ran_on_npu=1".into(),
            "invent_pass=0".into(),
            "product_known_good=0".into(),
            // Force honesty locks even if a hand-built bag mutates fields.
            "do_not_invent_pdi_layout=1".into(),
            "public_pdi_image_decode=0".into(),
            "section_offset_map_alone_closes_fw_residual=0".into(),
            format!("magic_xclbin2={}", u8::from(self.magic_xclbin2)),
            format!("section_headers_parsed={}", self.section_headers_parsed),
            format!(
                "has_bitstream_section={}",
                u8::from(self.has_bitstream_section)
            ),
            format!(
                "has_aie_partition_section={}",
                u8::from(self.has_aie_partition_section)
            ),
            format!(
                "has_cu_pdi_offset_surface={}",
                u8::from(self.has_cu_pdi_offset_surface)
            ),
        ];
        match self.num_sections {
            Some(n) => parts.push(format!("num_sections={n}")),
            None => parts.push("num_sections=absent".into()),
        }
        match self.bitstream_offset {
            Some(o) => parts.push(format!("bitstream_offset={o}")),
            None => parts.push("bitstream_offset=absent".into()),
        }
        match self.bitstream_size {
            Some(s) => parts.push(format!("bitstream_size={s}")),
            None => parts.push("bitstream_size=absent".into()),
        }
        match self.aie_partition_offset {
            Some(o) => parts.push(format!("aie_partition_offset={o}")),
            None => parts.push("aie_partition_offset=absent".into()),
        }
        match self.aie_partition_size {
            Some(s) => parts.push(format!("aie_partition_size={s}")),
            None => parts.push("aie_partition_size=absent".into()),
        }
        if !self.detail.is_empty() {
            parts.push(format!(
                "detail={}",
                sanitize_soft_note_field_value(&self.detail)
            ));
        }
        parts.join(" ")
    }
}

/// Map public `axlf_section_kind` numeric values to research name tokens.
///
/// Covers the CU/PDI identity pair + the topology/metadata scan set. Unknown
/// kinds → `None` (still parseable as raw headers; no invent of private names).
#[must_use]
pub fn axlf_section_kind_public_name(kind: u32) -> Option<&'static str> {
    match kind {
        AXLF_SECTION_KIND_BITSTREAM => Some("BITSTREAM"),
        2 => Some("EMBEDDED_METADATA"),
        6 => Some("MEM_TOPOLOGY"),
        7 => Some("CONNECTIVITY"),
        8 => Some("IP_LAYOUT"),
        14 => Some("BUILD_METADATA"),
        22 => Some("SYSTEM_METADATA"),
        26 => Some("GROUP_TOPOLOGY"),
        27 => Some("GROUP_CONNECTIVITY"),
        AXLF_SECTION_KIND_AIE_PARTITION => Some("AIE_PARTITION"),
        _ => None,
    }
}

/// Parse one public `axlf_section_header` (40 bytes, natural align LE).
///
/// Returns `None` when `bytes` is shorter than [`AXLF_SECTION_HEADER_SIZE`].
/// Does **not** validate that offset/size lie inside a parent file.
#[must_use]
pub fn parse_axlf_section_header(bytes: &[u8]) -> Option<AxlfSectionHeaderFields> {
    if bytes.len() < AXLF_SECTION_HEADER_SIZE {
        return None;
    }
    let kind = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    // name @ 4..20 (16 bytes); trim at first NUL.
    let name_raw = &bytes[4..20];
    let name_end = name_raw
        .iter()
        .position(|&b| b == 0)
        .unwrap_or(name_raw.len());
    let name = String::from_utf8_lossy(&name_raw[..name_end]).into_owned();
    // offset @ 24, size @ 32 (4-byte pad at 20..24 for u64 align).
    let offset = u64::from_le_bytes([
        bytes[24], bytes[25], bytes[26], bytes[27], bytes[28], bytes[29], bytes[30], bytes[31],
    ]);
    let size = u64::from_le_bytes([
        bytes[32], bytes[33], bytes[34], bytes[35], bytes[36], bytes[37], bytes[38], bytes[39],
    ]);
    Some(AxlfSectionHeaderFields {
        kind,
        name,
        offset,
        size,
    })
}

/// Read `m_numSections` from a buffer that includes the axlf header prefix.
///
/// Requires `bytes.len() >= AXLF_NUM_SECTIONS_ABS_OFFSET + 4`.
#[must_use]
pub fn parse_axlf_num_sections(bytes: &[u8]) -> Option<u32> {
    let off = AXLF_NUM_SECTIONS_ABS_OFFSET;
    if bytes.len() < off + 4 {
        return None;
    }
    Some(u32::from_le_bytes([
        bytes[off],
        bytes[off + 1],
        bytes[off + 2],
        bytes[off + 3],
    ]))
}

/// Walk public section headers after a valid xclbin2 magic + header prefix.
///
/// Stops at `min(num_sections, AXLF_MAX_SECTIONS_WALK)` or when bytes run out.
/// Empty / non-magic → empty list. Soft research only.
#[must_use]
pub fn walk_axlf_section_headers(bytes: &[u8]) -> Vec<AxlfSectionHeaderFields> {
    if !xclbin_magic_is_xclbin2(bytes) {
        return Vec::new();
    }
    let Some(num) = parse_axlf_num_sections(bytes) else {
        return Vec::new();
    };
    let walk = num.min(AXLF_MAX_SECTIONS_WALK) as usize;
    let mut out = Vec::with_capacity(walk.min(16));
    let mut off = AXLF_FIRST_SECTION_HEADER_ABS_OFFSET;
    for _ in 0..walk {
        if off + AXLF_SECTION_HEADER_SIZE > bytes.len() {
            break;
        }
        if let Some(h) = parse_axlf_section_header(&bytes[off..off + AXLF_SECTION_HEADER_SIZE]) {
            out.push(h);
        }
        off += AXLF_SECTION_HEADER_SIZE;
    }
    out
}

/// Research BITSTREAM / AIE_PARTITION **file offsets** from public axlf bytes.
///
/// Pure offline TDD surface (stamp `a5787723`). Parses section **headers** only
/// — never decodes section bodies / invents PDI layout / product known-good.
///
/// - Empty bytes → `detail=default_empty_bytes`
/// - Non-xclbin2 magic → `detail=magic_not_xclbin2`
/// - Magic OK but truncated before num_sections → `detail=header_truncated`
/// - Headers walked but neither CU/PDI kind present → `detail=no_cu_pdi_section_headers`
/// - BITSTREAM and/or AIE_PARTITION found → offset/size filled; detail names surface
///
/// Soft only; always `section_offset_map_alone_closes_fw_residual=false`.
#[must_use]
pub fn research_xclbin_cu_pdi_section_offset_map(bytes: &[u8]) -> CuPdiSectionOffsetMapResearch {
    if bytes.is_empty() {
        return CuPdiSectionOffsetMapResearch::default();
    }
    if !xclbin_magic_is_xclbin2(bytes) {
        return CuPdiSectionOffsetMapResearch {
            magic_xclbin2: false,
            detail: "magic_not_xclbin2".into(),
            ..CuPdiSectionOffsetMapResearch::default()
        };
    }
    let Some(num) = parse_axlf_num_sections(bytes) else {
        return CuPdiSectionOffsetMapResearch {
            magic_xclbin2: true,
            detail: "header_truncated".into(),
            ..CuPdiSectionOffsetMapResearch::default()
        };
    };
    let headers = walk_axlf_section_headers(bytes);
    #[allow(clippy::cast_possible_truncation)]
    let parsed = headers.len().min(u32::MAX as usize) as u32;
    let mut bs_off = None;
    let mut bs_sz = None;
    let mut aie_off = None;
    let mut aie_sz = None;
    for h in &headers {
        if h.kind == AXLF_SECTION_KIND_BITSTREAM && bs_off.is_none() {
            bs_off = Some(h.offset);
            bs_sz = Some(h.size);
        }
        if h.kind == AXLF_SECTION_KIND_AIE_PARTITION && aie_off.is_none() {
            aie_off = Some(h.offset);
            aie_sz = Some(h.size);
        }
    }
    let has_bs = bs_off.is_some();
    let has_aie = aie_off.is_some();
    let has_surface = has_bs || has_aie;
    let detail = if headers.is_empty() && num > 0 {
        "section_headers_truncated"
    } else if has_bs && has_aie {
        "cu_pdi_offset_surface_bitstream_and_aie_partition"
    } else if has_bs {
        "cu_pdi_offset_surface_bitstream_only"
    } else if has_aie {
        "cu_pdi_offset_surface_aie_partition_only"
    } else if !headers.is_empty() {
        "no_cu_pdi_section_headers"
    } else {
        "zero_sections"
    };
    CuPdiSectionOffsetMapResearch {
        magic_xclbin2: true,
        num_sections: Some(num),
        section_headers_parsed: parsed,
        bitstream_offset: bs_off,
        bitstream_size: bs_sz,
        aie_partition_offset: aie_off,
        aie_partition_size: aie_sz,
        has_bitstream_section: has_bs,
        has_aie_partition_section: has_aie,
        has_cu_pdi_offset_surface: has_surface,
        section_offset_map_alone_closes_fw_residual: false,
        do_not_invent_pdi_layout: true,
        public_pdi_image_decode: false,
        detail: detail.into(),
    }
}

// ---------------------------------------------------------------------------
// Pure axlf section body window bounds (no body decode; stamp 2c4b96bf)
// ---------------------------------------------------------------------------

/// Overflow-safe: section body window `[offset, offset+size)` fits in `file_len`.
///
/// - Zero-size at `offset <= file_len` is **in-bounds** (empty window).
/// - `offset > file_len` is OOB even when `size == 0`.
/// - `offset + size` overflow (wrapping) is OOB.
///
/// Does **not** read or decode body bytes. Soft research only.
#[must_use]
pub fn axlf_section_body_window_in_bounds(file_len: u64, offset: u64, size: u64) -> bool {
    check_axlf_section_body_bounds(file_len, offset, size).in_bounds
}

/// One pure section body window bounds check (no body bytes returned).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AxlfSectionBodyBounds {
    /// Claimed section data file offset.
    pub offset: u64,
    /// Claimed section data size.
    pub size: u64,
    /// Parent buffer / file length used for the check.
    pub file_len: u64,
    /// True when the half-open window fits without overflow.
    pub in_bounds: bool,
    /// Stable research detail: `in_bounds` / `oob_offset_past_file` /
    /// `oob_end_past_file` / `oob_size_overflow`.
    pub detail: String,
}

/// Pure bounds check for one axlf section body window vs a file length.
///
/// Never returns body bytes. Never invents PDI layout / Pass.
#[must_use]
pub fn check_axlf_section_body_bounds(
    file_len: u64,
    offset: u64,
    size: u64,
) -> AxlfSectionBodyBounds {
    let (in_bounds, detail) = if offset > file_len {
        (false, "oob_offset_past_file")
    } else {
        match offset.checked_add(size) {
            None => (false, "oob_size_overflow"),
            Some(end) if end > file_len => (false, "oob_end_past_file"),
            Some(_) => (true, "in_bounds"),
        }
    };
    AxlfSectionBodyBounds {
        offset,
        size,
        file_len,
        in_bounds,
        detail: detail.into(),
    }
}

/// Soft research: BITSTREAM / AIE_PARTITION body windows vs buffer/file length.
///
/// Stamp `2c4b96bf`: uses the public section-offset map ([`CuPdiSectionOffsetMapResearch`]
/// / `a5787723`) and checks that claimed body windows fit in `file_len` —
/// **without** decoding PDI/bitstream bodies or inventing product known-good.
///
/// Honesty:
/// - Body-bounds alone **never** closes FW residual
/// - Always `do_not_invent_pdi_layout=1` / `public_pdi_image_decode=0`
/// - Always `invent_pass=0 not_c_ready=1 not_ran_on_npu=1 product_known_good=0`
/// - In-bounds windows on a research buffer **≠** working-stack known-good
/// - Header-only / truncated buffers correctly report OOB vs short `file_len`
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // research flag bag; honesty-first
pub struct CuPdiSectionBodyBoundsResearch {
    /// Parent buffer/file length used for bounds checks.
    pub file_len: u64,
    /// True when parent buffer has xclbin2 magic (from offset map path).
    pub magic_xclbin2: bool,
    /// BITSTREAM window in-bounds when section present; `None` if absent.
    pub bitstream_in_bounds: Option<bool>,
    /// AIE_PARTITION window in-bounds when section present; `None` if absent.
    pub aie_partition_in_bounds: Option<bool>,
    /// BITSTREAM offset when section header present.
    pub bitstream_offset: Option<u64>,
    /// BITSTREAM size when section header present.
    pub bitstream_size: Option<u64>,
    /// AIE_PARTITION offset when section header present.
    pub aie_partition_offset: Option<u64>,
    /// AIE_PARTITION size when section header present.
    pub aie_partition_size: Option<u64>,
    /// True when ≥1 of BITSTREAM / AIE_PARTITION section headers is present.
    pub has_cu_pdi_offset_surface: bool,
    /// True when any present CU/PDI window is OOB vs `file_len`.
    pub any_oob: bool,
    /// True when every present CU/PDI window is in-bounds (false if none present).
    pub all_present_in_bounds: bool,
    /// Always **false**: body bounds alone never closes FW residual / invent Pass.
    pub body_bounds_alone_closes_fw_residual: bool,
    /// Always **true**: no private PDI layout invent.
    pub do_not_invent_pdi_layout: bool,
    /// Always **false**: public image decode remains closed.
    pub public_pdi_image_decode: bool,
    /// Stable research detail token.
    pub detail: String,
}

impl Default for CuPdiSectionBodyBoundsResearch {
    fn default() -> Self {
        Self {
            file_len: 0,
            magic_xclbin2: false,
            bitstream_in_bounds: None,
            aie_partition_in_bounds: None,
            bitstream_offset: None,
            bitstream_size: None,
            aie_partition_offset: None,
            aie_partition_size: None,
            has_cu_pdi_offset_surface: false,
            any_oob: false,
            all_present_in_bounds: false,
            body_bounds_alone_closes_fw_residual: false,
            do_not_invent_pdi_layout: true,
            public_pdi_image_decode: false,
            detail: "default_empty_bytes".into(),
        }
    }
}

impl CuPdiSectionBodyBoundsResearch {
    /// Soft research note (always honesty trio; never invent Pass / product known-good).
    #[must_use]
    pub fn format_soft_note(&self) -> String {
        let mut parts = vec![
            "cu_pdi_section_body_bounds_research".into(),
            "not_c_ready=1".into(),
            "not_ran_on_npu=1".into(),
            "invent_pass=0".into(),
            "product_known_good=0".into(),
            // Force honesty locks even if a hand-built bag mutates fields.
            "do_not_invent_pdi_layout=1".into(),
            "public_pdi_image_decode=0".into(),
            "body_bounds_alone_closes_fw_residual=0".into(),
            format!("file_len={}", self.file_len),
            format!("magic_xclbin2={}", u8::from(self.magic_xclbin2)),
            format!(
                "has_cu_pdi_offset_surface={}",
                u8::from(self.has_cu_pdi_offset_surface)
            ),
            format!("any_oob={}", u8::from(self.any_oob)),
            format!(
                "all_present_in_bounds={}",
                u8::from(self.all_present_in_bounds)
            ),
        ];
        match self.bitstream_in_bounds {
            Some(true) => parts.push("bitstream_in_bounds=1".into()),
            Some(false) => parts.push("bitstream_in_bounds=0".into()),
            None => parts.push("bitstream_in_bounds=absent".into()),
        }
        match self.aie_partition_in_bounds {
            Some(true) => parts.push("aie_partition_in_bounds=1".into()),
            Some(false) => parts.push("aie_partition_in_bounds=0".into()),
            None => parts.push("aie_partition_in_bounds=absent".into()),
        }
        match self.bitstream_offset {
            Some(o) => parts.push(format!("bitstream_offset={o}")),
            None => parts.push("bitstream_offset=absent".into()),
        }
        match self.bitstream_size {
            Some(s) => parts.push(format!("bitstream_size={s}")),
            None => parts.push("bitstream_size=absent".into()),
        }
        match self.aie_partition_offset {
            Some(o) => parts.push(format!("aie_partition_offset={o}")),
            None => parts.push("aie_partition_offset=absent".into()),
        }
        match self.aie_partition_size {
            Some(s) => parts.push(format!("aie_partition_size={s}")),
            None => parts.push("aie_partition_size=absent".into()),
        }
        if !self.detail.is_empty() {
            parts.push(format!(
                "detail={}",
                sanitize_soft_note_field_value(&self.detail)
            ));
        }
        parts.join(" ")
    }
}

/// Research CU/PDI body bounds from an offset map + file length (no body decode).
///
/// Stamp `2c4b96bf`. Soft only; always `body_bounds_alone_closes_fw_residual=false`.
#[must_use]
pub fn research_cu_pdi_section_body_bounds_from_offset_map(
    map: &CuPdiSectionOffsetMapResearch,
    file_len: u64,
) -> CuPdiSectionBodyBoundsResearch {
    if !map.magic_xclbin2 && map.detail == "default_empty_bytes" && file_len == 0 {
        return CuPdiSectionBodyBoundsResearch::default();
    }
    if !map.magic_xclbin2 {
        return CuPdiSectionBodyBoundsResearch {
            file_len,
            magic_xclbin2: false,
            detail: if map.detail.is_empty() {
                "magic_not_xclbin2".into()
            } else {
                map.detail.clone()
            },
            ..CuPdiSectionBodyBoundsResearch::default()
        };
    }
    if !map.has_cu_pdi_offset_surface {
        let detail = if map.detail.is_empty() {
            "no_cu_pdi_section_headers".into()
        } else {
            map.detail.clone()
        };
        return CuPdiSectionBodyBoundsResearch {
            file_len,
            magic_xclbin2: true,
            bitstream_offset: map.bitstream_offset,
            bitstream_size: map.bitstream_size,
            aie_partition_offset: map.aie_partition_offset,
            aie_partition_size: map.aie_partition_size,
            has_cu_pdi_offset_surface: false,
            detail,
            ..CuPdiSectionBodyBoundsResearch::default()
        };
    }

    let bs_bounds = match (map.bitstream_offset, map.bitstream_size) {
        (Some(o), Some(s)) => Some(check_axlf_section_body_bounds(file_len, o, s)),
        _ => None,
    };
    let aie_bounds = match (map.aie_partition_offset, map.aie_partition_size) {
        (Some(o), Some(s)) => Some(check_axlf_section_body_bounds(file_len, o, s)),
        _ => None,
    };
    let bitstream_in_bounds = bs_bounds.as_ref().map(|b| b.in_bounds);
    let aie_partition_in_bounds = aie_bounds.as_ref().map(|b| b.in_bounds);
    let any_oob = bitstream_in_bounds == Some(false) || aie_partition_in_bounds == Some(false);
    let present =
        usize::from(bitstream_in_bounds.is_some()) + usize::from(aie_partition_in_bounds.is_some());
    let present_ok = usize::from(bitstream_in_bounds == Some(true))
        + usize::from(aie_partition_in_bounds == Some(true));
    let all_present_in_bounds = present > 0 && present_ok == present;

    let detail = if all_present_in_bounds && present == 2 {
        "cu_pdi_body_windows_in_bounds"
    } else if all_present_in_bounds {
        "cu_pdi_body_window_in_bounds_partial_surface"
    } else if any_oob && present_ok > 0 {
        "cu_pdi_body_windows_partial_oob"
    } else if any_oob {
        "cu_pdi_body_windows_oob"
    } else {
        "cu_pdi_body_bounds_unknown"
    };

    CuPdiSectionBodyBoundsResearch {
        file_len,
        magic_xclbin2: true,
        bitstream_in_bounds,
        aie_partition_in_bounds,
        bitstream_offset: map.bitstream_offset,
        bitstream_size: map.bitstream_size,
        aie_partition_offset: map.aie_partition_offset,
        aie_partition_size: map.aie_partition_size,
        has_cu_pdi_offset_surface: true,
        any_oob,
        all_present_in_bounds,
        body_bounds_alone_closes_fw_residual: false,
        do_not_invent_pdi_layout: true,
        public_pdi_image_decode: false,
        detail: detail.into(),
    }
}

/// Research BITSTREAM / AIE_PARTITION body windows from axlf bytes (headers + len).
///
/// Pure offline TDD surface (stamp `2c4b96bf`). Parses section **headers** via
/// [`research_xclbin_cu_pdi_section_offset_map`], then checks body windows against
/// `bytes.len()` — **never** slices or decodes section bodies / invents PDI /
/// product known-good / residual closed.
///
/// Soft only; always `body_bounds_alone_closes_fw_residual=false`.
#[must_use]
pub fn research_xclbin_cu_pdi_section_body_bounds(bytes: &[u8]) -> CuPdiSectionBodyBoundsResearch {
    #[allow(clippy::cast_possible_truncation)]
    let file_len = bytes.len() as u64;
    let map = research_xclbin_cu_pdi_section_offset_map(bytes);
    research_cu_pdi_section_body_bounds_from_offset_map(&map, file_len)
}

/// Corpus fingerprint of patched (and unpatched) ctrltext after rela apply.
///
/// Pure offline research for future byte-diff vs a known-good image. **Not**
/// Pass evidence. Sha of **patched** bytes depends on IO paddrs; prefer
/// structural locks (`patched_len`, `rela_count`, classic header) and
/// [`Self::unpatched_sha256_hex`] for paddr-stable identity.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PatchedCtrltextFingerprint {
    /// Length of patched ctrltext when patch+walk produced bytes.
    pub patched_len: Option<usize>,
    /// Classic v0.1 walk OK after rela overwrite.
    pub classic_walk_ok: bool,
    /// `.rela.dyn` entry count when parseable.
    pub rela_count: Option<usize>,
    /// Staged mladf classic profile (1688 B / gen4 / 8×6 class) when true.
    pub staged_mladf_classic_profile: bool,
    /// Classic header `num_ops` when walkable (staged mladf = 40).
    pub txn_num_ops: Option<u32>,
    /// Classic header `dev_gen` (u8; staged mladf = 4).
    pub txn_dev_gen: Option<u8>,
    /// Classic header cols (u8; staged 8×6).
    pub txn_num_cols: Option<u8>,
    /// Classic header rows (u8; staged 8×6).
    pub txn_num_rows: Option<u8>,
    /// SHA-256 of **patched** ctrltext (paddr-dependent). Empty if absent.
    pub patched_sha256_hex: String,
    /// SHA-256 of **unpatched** peeled ctrltext (stable across paddrs). Empty if absent.
    pub unpatched_sha256_hex: String,
    /// Soft detail (`elf_empty`, `invalid_live_paddr`, `classic_walk_ok`, …).
    pub detail: String,
}

impl PatchedCtrltextFingerprint {
    /// Soft research note (honesty trio; documents paddr dependence of patched sha).
    #[must_use]
    pub fn format_soft_note(&self) -> String {
        let mut parts = vec![
            "patched_ctrltext_fingerprint".into(),
            "not_c_ready=1".into(),
            "not_ran_on_npu=1".into(),
            "invent_pass=0".into(),
            format!("classic_walk_ok={}", u8::from(self.classic_walk_ok)),
            format!(
                "staged_mladf_classic_profile={}",
                u8::from(self.staged_mladf_classic_profile)
            ),
        ];
        if let Some(n) = self.patched_len {
            parts.push(format!("patched_len={n}"));
        }
        if let Some(n) = self.rela_count {
            parts.push(format!("rela_count={n}"));
        }
        if let Some(n) = self.txn_num_ops {
            parts.push(format!("txn_num_ops={n}"));
        }
        if let Some(g) = self.txn_dev_gen {
            parts.push(format!("txn_dev_gen={g}"));
        }
        if let Some(c) = self.txn_num_cols {
            parts.push(format!("txn_num_cols={c}"));
        }
        if let Some(r) = self.txn_num_rows {
            parts.push(format!("txn_num_rows={r}"));
        }
        // Only claim paddr dependence when a patched sha is present.
        if self.patched_sha256_hex.is_empty() {
            parts.push("patched_sha=absent".into());
        } else {
            parts.push("patched_sha_depends_on_paddrs=1".into());
            parts.push(format!("patched_sha256={}", self.patched_sha256_hex));
        }
        if !self.unpatched_sha256_hex.is_empty() {
            parts.push(format!("unpatched_sha256={}", self.unpatched_sha256_hex));
        }
        if !self.detail.is_empty() {
            parts.push(format!(
                "detail={}",
                sanitize_soft_note_field_value(&self.detail)
            ));
        }
        parts.join(" ")
    }
}

/// Fingerprint peeled/patched ctrltext after rela with supplied IO paddrs.
///
/// Returns `None` only when inputs are empty (soft-skip). Invalid paddrs /
/// non-ELF still return a fingerprint bag with honesty detail (no invent Pass).
/// Prefer structural fields + `unpatched_sha256_hex` when locking unit tests;
/// `patched_sha256_hex` is stable only for a **fixed** paddr set.
#[must_use]
pub fn fingerprint_patched_ctrltext_after_rela(
    elf: &[u8],
    ifm: u64,
    wts: u64,
    ofm: u64,
    ctrl: u64,
) -> Option<PatchedCtrltextFingerprint> {
    if elf.is_empty() {
        return None;
    }
    let unpatched_sha = if is_aiebu_elf_ctrltext(elf) {
        super::txn::instruction_payload_bytes(elf)
            .ok()
            .map(|p| sha256_hex_lower(&p))
            .unwrap_or_default()
    } else {
        String::new()
    };
    let research = profile_instr_payload_after_rela(elf, ifm, wts, ofm, ctrl);
    let pay = research.payload.as_ref();
    Some(PatchedCtrltextFingerprint {
        patched_len: research.rela_audit.patched_ctrltext_len,
        classic_walk_ok: research.rela_audit.patched_ctrltext_walk_ok
            && pay.is_some_and(|p| p.classic_walk_ok),
        rela_count: research.rela_audit.rela_count,
        staged_mladf_classic_profile: pay.is_some_and(|p| p.staged_mladf_classic_profile),
        txn_num_ops: pay.and_then(|p| p.txn_num_ops),
        txn_dev_gen: pay.and_then(|p| p.txn_dev_gen),
        txn_num_cols: pay.and_then(|p| p.txn_num_cols),
        txn_num_rows: pay.and_then(|p| p.txn_num_rows),
        patched_sha256_hex: pay
            .map(|p| p.payload_sha256_hex.clone())
            .unwrap_or_default(),
        unpatched_sha256_hex: unpatched_sha,
        detail: research.detail,
    })
}

/// Convenience: fingerprint with locked C2 live IO paddrs (research only).
#[must_use]
pub fn fingerprint_patched_ctrltext_after_rela_c2_locked(
    elf: &[u8],
) -> Option<PatchedCtrltextFingerprint> {
    let (_instr, ifm, wts, ofm, ctrl) = c2_live_io_paddrs_locked();
    fingerprint_patched_ctrltext_after_rela(elf, ifm, wts, ofm, ctrl)
}

// ---------------------------------------------------------------------------
// Construction residual: fingerprint export + external ctrltext compare (no Pass)
// ---------------------------------------------------------------------------

/// Soft-note / export token: no known-good **external** ctrltext offline yet.
///
/// Local pure-direct oracle (`gt_capture/instr_post_patch.bin`) is **not** this —
/// it is our own patched peel, not a working public-stack capture. Export APIs
/// keep this status until a real external known-good is supplied to the compare
/// helper. Never equals Pass / C-ready / `ran_on_npu`.
pub const KNOWN_GOOD_CTRLTEXT_ABSENT_TOKEN: &str = "absent_awaiting_capture";

/// Soft-note bag name for [`CtrltextFingerprintExport::format_soft_note`].
pub const CTRLTEXT_FINGERPRINT_EXPORT_NOTE: &str = "ctrltext_fingerprint_export";

/// Soft-note bag name for [`ExternalCtrltextCompareResearch::format_soft_note`].
pub const EXTERNAL_CTRLTEXT_COMPARE_NOTE: &str = "external_ctrltext_compare";

/// Exportable unpatched ctrltext identity for later external known-good compare.
///
/// Pure offline research bag. Locks structural fields + paddr-stable
/// [`Self::unpatched_sha256_hex`]. **Always** reports
/// [`KNOWN_GOOD_CTRLTEXT_ABSENT_TOKEN`] from constructors — this API never invents
/// known-good external bytes. Compare via
/// [`compare_external_ctrltext_to_lab_export`] when external bytes arrive.
///
/// Honesty: export identity **≠** Pass **≠** C-ready **≠** `ran_on_npu`.
/// Matching our failing silicon_lab corpus is research only (FW still
/// `0x3000003`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // research flag bag; honesty-first
pub struct CtrltextFingerprintExport {
    /// Peeled unpatched ctrltext length when aiebu present.
    pub ctrltext_len: Option<usize>,
    /// `.rela.dyn` entry count when parseable (None for non-aiebu).
    pub rela_count: Option<usize>,
    /// Classic v0.1 walk OK on **unpatched** peel.
    pub classic_walk_ok: bool,
    /// Staged mladf classic profile (1688 / gen4 / 8×6) on unpatched peel.
    pub staged_mladf_classic_profile: bool,
    /// Classic header `num_ops` when walkable.
    pub txn_num_ops: Option<u32>,
    /// Classic header `dev_gen` (u8).
    pub txn_dev_gen: Option<u8>,
    /// Classic header cols (u8).
    pub txn_num_cols: Option<u8>,
    /// Classic header rows (u8).
    pub txn_num_rows: Option<u8>,
    /// SHA-256 of **unpatched** peeled ctrltext (stable; empty if peel absent).
    pub unpatched_sha256_hex: String,
    /// Always **false** from [`export_ctrltext_fingerprint_research`] — do not invent.
    pub known_good_present: bool,
    /// [`KNOWN_GOOD_CTRLTEXT_ABSENT_TOKEN`] from production export path.
    pub known_good_status: &'static str,
    /// Research detail (`elf_empty`, `classic_unpatched_ok`, …).
    pub detail: String,
}

impl Default for CtrltextFingerprintExport {
    fn default() -> Self {
        Self {
            ctrltext_len: None,
            rela_count: None,
            classic_walk_ok: false,
            staged_mladf_classic_profile: false,
            txn_num_ops: None,
            txn_dev_gen: None,
            txn_num_cols: None,
            txn_num_rows: None,
            unpatched_sha256_hex: String::new(),
            known_good_present: false,
            known_good_status: KNOWN_GOOD_CTRLTEXT_ABSENT_TOKEN,
            detail: "default_known_good_absent".into(),
        }
    }
}

impl CtrltextFingerprintExport {
    /// True when export reports known-good external still absent (production path).
    #[must_use]
    pub fn is_known_good_absent(&self) -> bool {
        !self.known_good_present && self.known_good_status == KNOWN_GOOD_CTRLTEXT_ABSENT_TOKEN
    }

    /// Soft research note (always honesty trio; never invent Pass / C-ready).
    #[must_use]
    pub fn format_soft_note(&self) -> String {
        let mut parts = vec![
            CTRLTEXT_FINGERPRINT_EXPORT_NOTE.into(),
            "not_c_ready=1".into(),
            "not_ran_on_npu=1".into(),
            "invent_pass=0".into(),
            format!("classic_walk_ok={}", u8::from(self.classic_walk_ok)),
            format!(
                "staged_mladf_classic_profile={}",
                u8::from(self.staged_mladf_classic_profile)
            ),
            format!("known_good_present={}", u8::from(self.known_good_present)),
            format!(
                "known_good_status={}",
                sanitize_soft_note_field_value(self.known_good_status)
            ),
            format!(
                "known_good_absent={}",
                u8::from(self.is_known_good_absent())
            ),
        ];
        if let Some(n) = self.ctrltext_len {
            parts.push(format!("ctrltext_len={n}"));
        }
        if let Some(n) = self.rela_count {
            parts.push(format!("rela_count={n}"));
        }
        if let Some(n) = self.txn_num_ops {
            parts.push(format!("txn_num_ops={n}"));
        }
        if let Some(g) = self.txn_dev_gen {
            parts.push(format!("txn_dev_gen={g}"));
        }
        if let Some(c) = self.txn_num_cols {
            parts.push(format!("txn_num_cols={c}"));
        }
        if let Some(r) = self.txn_num_rows {
            parts.push(format!("txn_num_rows={r}"));
        }
        if self.unpatched_sha256_hex.is_empty() {
            parts.push("unpatched_sha=absent".into());
        } else {
            parts.push(format!("unpatched_sha256={}", self.unpatched_sha256_hex));
        }
        if !self.detail.is_empty() {
            parts.push(format!(
                "detail={}",
                sanitize_soft_note_field_value(&self.detail)
            ));
        }
        parts.join(" ")
    }

    /// Portable key=value export text for offline bags (when known-good arrives later).
    ///
    /// Same honesty tokens as [`Self::format_soft_note`]; one field per line for
    /// easy `diff` / archival. **Not** a Pass snapshot.
    ///
    /// **Contract:** soft-note field values are single tokens only (via
    /// [`sanitize_soft_note_field_value`] / hex / numeric). Splitting on space
    /// is safe under that contract; do not add unsanitized multi-word values.
    #[must_use]
    pub fn format_export_text(&self) -> String {
        // Reuse soft-note tokens; split on space into lines for bag files.
        // Safe only while values remain single tokens (see rustdoc contract).
        self.format_soft_note().replace(' ', "\n")
    }
}

/// Build a fingerprint export from an aiebu ELF (unpatched peel + structural).
///
/// - Empty / non-aiebu → honesty bag with `known_good_absent`; no invent sha.
/// - Valid aiebu → structural + unpatched sha; **still**
///   [`KNOWN_GOOD_CTRLTEXT_ABSENT_TOKEN`] (export is lab identity, not known-good).
/// - Does **not** apply rela (paddr-independent identity).
/// - Always soft; never Pass / C-ready / second unlock.
#[must_use]
pub fn export_ctrltext_fingerprint_research(elf: &[u8]) -> CtrltextFingerprintExport {
    if elf.is_empty() {
        return CtrltextFingerprintExport {
            detail: "elf_empty".into(),
            ..CtrltextFingerprintExport::default()
        };
    }
    if !is_aiebu_elf_ctrltext(elf) {
        return CtrltextFingerprintExport {
            detail: "not_aiebu_elf_ctrltext".into(),
            ..CtrltextFingerprintExport::default()
        };
    }
    let rela_count = parse_elf32_rela_dyn(elf).ok().map(|r| r.len());
    let Ok(peeled) = super::txn::instruction_payload_bytes(elf) else {
        return CtrltextFingerprintExport {
            rela_count,
            detail: "ctrltext_peel_failed".into(),
            ..CtrltextFingerprintExport::default()
        };
    };
    let profile = profile_instr_payload_bytes(&peeled);
    let detail = if profile.classic_walk_ok && profile.staged_mladf_classic_profile {
        "silicon_lab_structural_ok".into()
    } else if profile.classic_walk_ok {
        "classic_unpatched_ok".into()
    } else {
        "ctrltext_not_classic_walkable".into()
    };
    CtrltextFingerprintExport {
        ctrltext_len: Some(peeled.len()),
        rela_count,
        classic_walk_ok: profile.classic_walk_ok,
        staged_mladf_classic_profile: profile.staged_mladf_classic_profile,
        txn_num_ops: profile.txn_num_ops,
        txn_dev_gen: profile.txn_dev_gen,
        txn_num_cols: profile.txn_num_cols,
        txn_num_rows: profile.txn_num_rows,
        unpatched_sha256_hex: sha256_hex_lower(&peeled),
        known_good_present: false,
        known_good_status: KNOWN_GOOD_CTRLTEXT_ABSENT_TOKEN,
        detail,
    }
}

/// Offline compare of optional external ctrltext bytes vs a lab fingerprint export.
///
/// Soft research only:
/// - Empty / missing external → `known_good_absent=1` (awaiting capture).
/// - Present external → sha/len/structural diff vs lab unpatched peel.
/// - **Even** `sha_match=1` is **not** Pass (may match our failing corpus; FW
///   residual still open). Never invents C-ready / `ran_on_npu`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // research flag bag; honesty-first
pub struct ExternalCtrltextCompareResearch {
    /// Lab unpatched sha (echo from export).
    pub lab_unpatched_sha256_hex: String,
    /// Lab ctrltext length when known.
    pub lab_len: Option<usize>,
    /// True when non-empty external bytes were supplied.
    pub external_present: bool,
    /// SHA-256 of external bytes (empty when absent).
    pub external_sha256_hex: String,
    /// External byte length when present.
    pub external_len: Option<usize>,
    /// `external_len == lab_len` when both known.
    pub len_match: bool,
    /// External sha equals lab unpatched sha (both non-empty).
    pub sha_match: bool,
    /// Classic walk OK on external bytes.
    pub external_classic_walk_ok: bool,
    /// Echo of lab classic walk.
    pub lab_classic_walk_ok: bool,
    /// External matches staged mladf classic profile.
    pub external_staged_mladf_classic_profile: bool,
    /// Echo of lab staged profile bit.
    pub lab_staged_mladf_classic_profile: bool,
    /// len + classic + staged profile bits all agree (research; ≠ Pass).
    pub structural_match: bool,
    /// True when external missing/empty (awaiting capture). False when bytes
    /// were supplied for research compare — still **not** product known-good /
    /// Pass (see soft-note honesty trio + always `known_good_present=0` /
    /// `product_known_good=0` on [`Self::format_soft_note`]).
    pub known_good_absent: bool,
    /// Research detail token.
    pub detail: String,
}

impl Default for ExternalCtrltextCompareResearch {
    fn default() -> Self {
        Self {
            lab_unpatched_sha256_hex: String::new(),
            lab_len: None,
            external_present: false,
            external_sha256_hex: String::new(),
            external_len: None,
            len_match: false,
            sha_match: false,
            external_classic_walk_ok: false,
            lab_classic_walk_ok: false,
            external_staged_mladf_classic_profile: false,
            lab_staged_mladf_classic_profile: false,
            structural_match: false,
            known_good_absent: true,
            detail: "default_known_good_absent".into(),
        }
    }
}

impl ExternalCtrltextCompareResearch {
    /// Soft research note (always honesty trio; sha_match ≠ invent Pass).
    ///
    /// Always emits `known_good_present=0` and `product_known_good=0` so a
    /// log skim cannot confuse `known_good_absent=0` (research external bytes
    /// supplied) with product known-good capture.
    #[must_use]
    pub fn format_soft_note(&self) -> String {
        let mut parts = vec![
            EXTERNAL_CTRLTEXT_COMPARE_NOTE.into(),
            "not_c_ready=1".into(),
            "not_ran_on_npu=1".into(),
            "invent_pass=0".into(),
            // Explicit product-known-good denial even when external_present=1
            // (research bytes / lab self-peel are never product known-good).
            "known_good_present=0".into(),
            "product_known_good=0".into(),
            format!("external_present={}", u8::from(self.external_present)),
            format!("known_good_absent={}", u8::from(self.known_good_absent)),
            format!("len_match={}", u8::from(self.len_match)),
            format!("sha_match={}", u8::from(self.sha_match)),
            format!("structural_match={}", u8::from(self.structural_match)),
            format!(
                "external_classic_walk_ok={}",
                u8::from(self.external_classic_walk_ok)
            ),
            format!("lab_classic_walk_ok={}", u8::from(self.lab_classic_walk_ok)),
            format!(
                "external_staged_mladf_classic_profile={}",
                u8::from(self.external_staged_mladf_classic_profile)
            ),
            format!(
                "lab_staged_mladf_classic_profile={}",
                u8::from(self.lab_staged_mladf_classic_profile)
            ),
        ];
        if let Some(n) = self.lab_len {
            parts.push(format!("lab_len={n}"));
        }
        if let Some(n) = self.external_len {
            parts.push(format!("external_len={n}"));
        }
        if !self.lab_unpatched_sha256_hex.is_empty() {
            parts.push(format!(
                "lab_unpatched_sha256={}",
                self.lab_unpatched_sha256_hex
            ));
        }
        if !self.external_sha256_hex.is_empty() {
            parts.push(format!("external_sha256={}", self.external_sha256_hex));
        }
        if !self.detail.is_empty() {
            parts.push(format!(
                "detail={}",
                sanitize_soft_note_field_value(&self.detail)
            ));
        }
        parts.join(" ")
    }
}

/// Compare optional external ctrltext bytes to a lab fingerprint export.
///
/// - `external = None` or empty slice → known-good absent bag (awaiting capture).
/// - Non-empty → profile + sha/len vs lab; soft only.
/// - Never claims Pass even on full match.
#[must_use]
pub fn compare_external_ctrltext_to_lab_export(
    lab: &CtrltextFingerprintExport,
    external: Option<&[u8]>,
) -> ExternalCtrltextCompareResearch {
    let lab_sha = lab.unpatched_sha256_hex.clone();
    let lab_len = lab.ctrltext_len;
    let lab_classic = lab.classic_walk_ok;
    let lab_staged = lab.staged_mladf_classic_profile;

    let Some(ext) = external.filter(|b| !b.is_empty()) else {
        return ExternalCtrltextCompareResearch {
            lab_unpatched_sha256_hex: lab_sha,
            lab_len,
            external_present: false,
            external_sha256_hex: String::new(),
            external_len: None,
            len_match: false,
            sha_match: false,
            external_classic_walk_ok: false,
            lab_classic_walk_ok: lab_classic,
            external_staged_mladf_classic_profile: false,
            lab_staged_mladf_classic_profile: lab_staged,
            structural_match: false,
            known_good_absent: true,
            detail: "external_absent_awaiting_capture".into(),
        };
    };

    let ext_profile = profile_instr_payload_bytes(ext);
    let ext_sha = sha256_hex_lower(ext);
    let ext_len = ext.len();
    let len_match = lab_len == Some(ext_len);
    let sha_match = !lab_sha.is_empty() && lab_sha == ext_sha;
    // Positive structural claim requires meaningful classic structure on both
    // sides (not dual non-walkable same-length garbage reporting match=1).
    let structural_match = len_match
        && lab_classic
        && ext_profile.classic_walk_ok
        && ext_profile.staged_mladf_classic_profile == lab_staged
        && ext_profile.txn_num_ops == lab.txn_num_ops
        && ext_profile.txn_dev_gen == lab.txn_dev_gen
        && ext_profile.txn_num_cols == lab.txn_num_cols
        && ext_profile.txn_num_rows == lab.txn_num_rows;
    let detail = if sha_match && structural_match {
        // Match to lab unpatched peel — still not a known-good working-stack capture.
        "sha_and_structural_match_lab_unpatched".into()
    } else if sha_match {
        "sha_match_lab_unpatched".into()
    } else if structural_match {
        "structural_match_sha_diff".into()
    } else if len_match {
        "len_match_content_diff".into()
    } else {
        "external_differs_from_lab".into()
    };

    ExternalCtrltextCompareResearch {
        lab_unpatched_sha256_hex: lab_sha,
        lab_len,
        external_present: true,
        external_sha256_hex: ext_sha,
        external_len: Some(ext_len),
        len_match,
        sha_match,
        external_classic_walk_ok: ext_profile.classic_walk_ok,
        lab_classic_walk_ok: lab_classic,
        external_staged_mladf_classic_profile: ext_profile.staged_mladf_classic_profile,
        lab_staged_mladf_classic_profile: lab_staged,
        structural_match,
        // Bytes present for research compare — still never product known-good /
        // Pass (`invent_pass=0` soft note). Working public-stack capture is open.
        known_good_absent: false,
        detail,
    }
}

// ---------------------------------------------------------------------------
// External known-good bag confinement + load (feed compare when bytes arrive)
// ---------------------------------------------------------------------------

/// Subdir under `testdata/xdna2_cmd/` for operator-placed external known-good bags.
///
/// Local pure-direct oracles (`gt_capture/instr_post_patch.bin`, etc.) live
/// **outside** this tree and are **refused** as known-good. Bins under this
/// subdir stay gitignored; tracked `PROVENANCE.md` documents the capture.
pub const EXTERNAL_KNOWN_GOOD_SUBDIR: &str = "external_known_good";

/// Env: optional path to external ctrltext for soft classify compare.
///
/// Soft only. Path must pass [`external_known_good_load_path_is_allowed`].
/// Loading for research **≠** product known-good **≠** Pass **≠** C-ready.
pub const ENV_LAB_EXTERNAL_CTRLTEXT: &str = "XDNA2_LAB_EXTERNAL_CTRLTEXT";

/// Basenames that are local pure-direct / vendor post-patch oracles — never
/// treated as working public-stack known-good by default.
const REFUSED_KNOWN_GOOD_BASENAMES: &[&str] = &[
    "instr_post_patch.bin",
    "instr_vendor_post_patch.bin",
    "ctrl_post_patch.bin",
    "instr_raw_txn.bin",
    "instr_vendor_txn_bin.bin",
];

/// True when `path` basenames a local post-patch / pure-direct oracle that must
/// **not** be treated as external known-good.
///
/// Pure path check (basename only). Complements dedicated-subdir confinement.
#[must_use]
pub fn is_refused_local_post_patch_as_known_good(path: &std::path::Path) -> bool {
    path.file_name().is_some_and(|n| {
        let s = n.to_string_lossy();
        REFUSED_KNOWN_GOOD_BASENAMES
            .iter()
            .any(|b| s.eq_ignore_ascii_case(b))
    })
}

/// True when the final path component has an allowed external bag extension.
///
/// Allowed: `.bin`, `.ctrltext` (case-insensitive). Not `.json` / `.elf` —
/// known-good compare expects raw ctrltext bytes, not whole ELF/meta.
#[must_use]
pub fn external_ctrltext_extension_ok(path: &std::path::Path) -> bool {
    path.extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("bin") || ext.eq_ignore_ascii_case("ctrltext"))
}

/// Lab root for external known-good bags: `testdata/xdna2_cmd/external_known_good`.
#[must_use]
pub fn external_known_good_lab_root() -> std::path::PathBuf {
    let lab = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("testdata/xdna2_cmd")
        .join(EXTERNAL_KNOWN_GOOD_SUBDIR);
    std::fs::canonicalize(&lab).unwrap_or(lab)
}

/// True when `path` is under clamped temp root or the dedicated external bag root.
fn path_under_external_known_good_roots(path: &std::path::Path) -> bool {
    let temp_root = last_apply_temp_root();
    let bag_root = external_known_good_lab_root();
    path.starts_with(&temp_root) || path.starts_with(&bag_root)
}

/// Whether an external known-good **path** is allowed (roots + extension + refuse).
///
/// **Confinement** (mirrors [`last_apply_path_is_allowed`]):
/// - Under [`last_apply_temp_root`] **or** [`external_known_good_lab_root`]
/// - Extension `.bin` / `.ctrltext` only
/// - No `..` components
/// - Refuse local post-patch oracle basenames
/// - Resolve intermediate dir symlinks via `canonicalize(parent)` before root check
///
/// Does **not** require PROVENANCE (that is a load-time check for lab bag paths).
/// Does **not** open the file.
#[must_use]
pub fn external_known_good_path_is_allowed(path: &std::path::Path) -> bool {
    if path.as_os_str().is_empty() {
        return false;
    }
    if path
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return false;
    }
    if !external_ctrltext_extension_ok(path) {
        return false;
    }
    if is_refused_local_post_patch_as_known_good(path) {
        return false;
    }
    let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) else {
        return path_under_external_known_good_roots(path);
    };
    if let Ok(canon_parent) = std::fs::canonicalize(parent) {
        let candidate = canon_parent.join(path.file_name().unwrap_or_default());
        // Re-check refuse on resolved candidate basename (same as path usually).
        if is_refused_local_post_patch_as_known_good(&candidate) {
            return false;
        }
        path_under_external_known_good_roots(&candidate)
    } else if path.is_absolute() {
        // Absolute + missing parent: cannot prove confinement → refuse.
        false
    } else {
        path_under_external_known_good_roots(path)
    }
}

/// Soft-load path: path roots + refuse final-component symlink + re-check
/// resolved real path under allowed roots when the file exists.
///
/// Lab-bag paths (under [`external_known_good_lab_root`]) additionally require a
/// **research-ready** sibling/parent [`PROVENANCE.md`] via
/// [`external_known_good_provenance_research_ready`] (non-empty + research-only
/// marker + source/stack marker + no product Pass invent tokens) so local
/// pure-direct dumps cannot be renamed into the bag without honest notes.
/// Temp-dir files used by unit tests do **not** require PROVENANCE.
///
/// **Fail-closed:** if the path exists but `canonicalize` fails (permissions /
/// transient FS), refuse — cannot prove resolved roots or lab-bag PROVENANCE.
///
/// Residual TOCTOU after this check: [`load_external_ctrltext_for_compare`] opens
/// with Unix `O_NOFOLLOW` as defense-in-depth (same posture as soft last_apply
/// JSON in the classify example).
#[must_use]
pub fn external_known_good_load_path_is_allowed(path: &std::path::Path) -> bool {
    if !external_known_good_path_is_allowed(path) {
        return false;
    }
    if let Ok(meta) = std::fs::symlink_metadata(path) {
        if meta.file_type().is_symlink() {
            return false;
        }
    }
    if path.exists() {
        let Ok(canon) = std::fs::canonicalize(path) else {
            // Fail closed: cannot re-check roots / bag PROVENANCE.
            return false;
        };
        if !path_under_external_known_good_roots(&canon) {
            return false;
        }
        if is_refused_local_post_patch_as_known_good(&canon) {
            return false;
        }
        // Lab bag only: require research-ready PROVENANCE (fields + refuse invent).
        // Pure gate = [`external_known_good_lab_bag_provenance_allows_load`].
        let bag = external_known_good_lab_root();
        if canon.starts_with(&bag)
            && !external_known_good_lab_bag_provenance_allows_load(
                read_external_known_good_provenance_text(&canon).as_deref(),
            )
        {
            return false;
        }
    }
    true
}

/// True when a non-empty `PROVENANCE.md` exists next to `path` or in its parent
/// directory (lab bag honesty token). Whitespace-only counts as empty.
///
/// For **research-present** eligibility (field markers + refuse product Pass
/// claims), use [`external_known_good_provenance_research_ready`] /
/// [`validate_external_known_good_provenance_text`].
#[must_use]
pub fn external_known_good_provenance_present(path: &std::path::Path) -> bool {
    read_external_known_good_provenance_text(path).is_some()
}

/// Read first non-empty `PROVENANCE.md` for a bag path.
///
/// **Candidates (no grandparent walk outside the bag):**
/// 1. Immediate parent `PROVENANCE.md` (subdir sibling note).
/// 2. When `path` is under [`external_known_good_lab_root`], also the **bag root**
///    `external_known_good/PROVENANCE.md` (lab bag owns its research-ready note).
///
/// Does **not** accept `testdata/xdna2_cmd/PROVENANCE.md` (grandparent of the lab
/// bag) — that would weaken “lab bag requires its own research-ready PROVENANCE.”
/// Temp-dir research files use parent only (no grandparent).
///
/// Returns `None` when missing, not a file, empty, or whitespace-only.
#[must_use]
pub fn read_external_known_good_provenance_text(path: &std::path::Path) -> Option<String> {
    let mut candidates: Vec<std::path::PathBuf> = Vec::new();
    if let Some(parent) = path.parent() {
        candidates.push(parent.join("PROVENANCE.md"));
    }
    // Lab bag only: bag-root PROVENANCE when path is nested under the bag.
    // (Immediate parent already covers bag/foo.bin → bag/PROVENANCE.md.)
    let bag = external_known_good_lab_root();
    let under_bag = path.starts_with(&bag)
        || std::fs::canonicalize(path).is_ok_and(|c| c.starts_with(&bag))
        || path
            .parent()
            .and_then(|p| std::fs::canonicalize(p).ok())
            .is_some_and(|cp| cp.starts_with(&bag));
    if under_bag {
        let bag_prov = bag.join("PROVENANCE.md");
        if !candidates.iter().any(|c| c == &bag_prov) {
            candidates.push(bag_prov);
        }
    }
    for cand in candidates {
        if let Ok(meta) = std::fs::metadata(&cand) {
            if !meta.is_file() || meta.len() == 0 {
                continue;
            }
            if let Ok(text) = std::fs::read_to_string(&cand) {
                if !text.trim().is_empty() {
                    return Some(text);
                }
            }
        }
    }
    None
}

/// Fail-closed PROVENANCE field check for external known-good research bags.
///
/// Research only — passing this check **≠** product known-good **≠** Pass
/// **≠** C-ready **≠** second unlock.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // research flag bag; honesty-first
pub struct ExternalKnownGoodProvenanceCheck {
    /// Text was non-empty after trim.
    pub non_empty: bool,
    /// Honesty language: research-only / not product Pass / not C-ready.
    pub has_research_only_marker: bool,
    /// Source/stack section or public-stack language present.
    pub has_source_stack_marker: bool,
    /// No product Pass invent tokens (`invent_pass=1`, `ran_on_npu=1`, …).
    pub refuses_product_pass_claim: bool,
    /// All required markers pass → bag may be treated as research-present.
    pub research_present_eligible: bool,
    /// Stable detail token for soft notes / unit locks.
    pub detail: String,
}

impl Default for ExternalKnownGoodProvenanceCheck {
    fn default() -> Self {
        Self {
            non_empty: false,
            has_research_only_marker: false,
            has_source_stack_marker: false,
            refuses_product_pass_claim: true,
            research_present_eligible: false,
            detail: "empty".into(),
        }
    }
}

impl ExternalKnownGoodProvenanceCheck {
    /// Soft research note (always honesty trio; never invent Pass).
    #[must_use]
    pub fn format_soft_note(&self) -> String {
        let mut parts = vec![
            "external_known_good_provenance_check".into(),
            "not_c_ready=1".into(),
            "not_ran_on_npu=1".into(),
            "invent_pass=0".into(),
            "product_known_good=0".into(),
            format!("non_empty={}", u8::from(self.non_empty)),
            format!(
                "has_research_only_marker={}",
                u8::from(self.has_research_only_marker)
            ),
            format!(
                "has_source_stack_marker={}",
                u8::from(self.has_source_stack_marker)
            ),
            format!(
                "refuses_product_pass_claim={}",
                u8::from(self.refuses_product_pass_claim)
            ),
            format!(
                "research_present_eligible={}",
                u8::from(self.research_present_eligible)
            ),
        ];
        if !self.detail.is_empty() {
            parts.push(format!(
                "detail={}",
                sanitize_soft_note_field_value(&self.detail)
            ));
        }
        parts.join(" ")
    }
}

/// True when ascii-lowered `hay` contains `needle` as a whole alphanumeric token
/// (split on non-alphanumeric). Used for source-stack marker `xrt` so prose like
/// `texture` / `extract` does not satisfy the stack marker by accident.
#[must_use]
fn provenance_lower_has_alnum_token(hay_lower: &str, needle_lower: &str) -> bool {
    hay_lower
        .split(|c: char| !c.is_ascii_alphanumeric())
        .any(|t| t == needle_lower)
}

/// Validate PROVENANCE.md body before treating an external bag as research-present.
///
/// **Required (all):**
/// 1. Non-empty (after trim).
/// 2. Research-only honesty marker (case-insensitive):
///    prose `research only` / `not product pass` / `not c-ready` / `not c ready`,
///    **or** exact token `not_c_ready=1` (not bare substring `not_c_ready`, which
///    would also match honesty-inverse `not_c_ready=0`).
/// 3. Source/stack marker (case-insensitive contains one of:
///    `source stack`, `public stack`, `working public`, `dynamicdispatch`,
///    `ryzen ai`, or whole-token `xrt`).
/// 4. Refuse product Pass invent / honesty-inverse tokens (ascii-lower exact):
///    `invent_pass=1`, `invent_pass=true`, `ran_on_npu=1`, `ran_on_npu=true`,
///    `c_ready=1`, `product_known_good=1`, `not_c_ready=0`.
///
/// Empty / missing markers → `research_present_eligible=false` (fail-closed).
/// Passing eligibility **≠** product known-good **≠** Pass.
#[must_use]
pub fn validate_external_known_good_provenance_text(
    text: &str,
) -> ExternalKnownGoodProvenanceCheck {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return ExternalKnownGoodProvenanceCheck {
            non_empty: false,
            has_research_only_marker: false,
            has_source_stack_marker: false,
            refuses_product_pass_claim: true,
            research_present_eligible: false,
            detail: "empty".into(),
        };
    }
    // All marker / invent checks on ascii-lower so mixed-case claims cannot bypass.
    let lower = trimmed.to_ascii_lowercase();
    let research = lower.contains("research only")
        || lower.contains("not product pass")
        || lower.contains("not c-ready")
        || lower.contains("not c ready")
        // Exact honesty token only — not substring (avoids `not_c_ready=0`).
        || residual_note_has_exact_token(&lower, "not_c_ready=1");
    let source = lower.contains("source stack")
        || lower.contains("public stack")
        || lower.contains("working public")
        || lower.contains("dynamicdispatch")
        || lower.contains("ryzen ai")
        // Whole-token `xrt` only (not substring of unrelated words).
        || provenance_lower_has_alnum_token(&lower, "xrt");
    // Product invent / honesty-inverse claims as exact soft-note tokens on lower.
    // Not substring of `not_ran_on_npu=1` / `invent_pass=0`.
    let invent = residual_note_has_exact_token(&lower, "invent_pass=1")
        || residual_note_has_exact_token(&lower, "invent_pass=true")
        || residual_note_has_exact_token(&lower, "ran_on_npu=1")
        || residual_note_has_exact_token(&lower, "ran_on_npu=true")
        || residual_note_has_exact_token(&lower, "c_ready=1")
        || residual_note_has_exact_token(&lower, "product_known_good=1")
        || residual_note_has_exact_token(&lower, "not_c_ready=0");
    let refuses = !invent;
    let eligible = research && source && refuses;
    let detail = if invent {
        "product_pass_claim_refused"
    } else if !research {
        "missing_research_only_marker"
    } else if !source {
        "missing_source_stack_marker"
    } else {
        "research_present_eligible"
    };
    ExternalKnownGoodProvenanceCheck {
        non_empty: true,
        has_research_only_marker: research,
        has_source_stack_marker: source,
        refuses_product_pass_claim: refuses,
        research_present_eligible: eligible,
        detail: detail.into(),
    }
}

/// Pure lab-bag PROVENANCE load gate given already-read text.
///
/// `None` / invalid markers / invent tokens → **false** (fail-closed). Used by
/// [`external_known_good_provenance_research_ready`] and unit-locked so load
/// path refuse does not depend on mutating the tracked bag on disk.
///
/// Research only — eligible **≠** product known-good **≠** Pass **≠** C-ready.
#[must_use]
pub fn external_known_good_lab_bag_provenance_allows_load(text: Option<&str>) -> bool {
    text.is_some_and(|t| validate_external_known_good_provenance_text(t).research_present_eligible)
}

/// True when lab-bag PROVENANCE next to `path` passes field validation.
///
/// Fail-closed: missing/empty/invalid markers → false. Temp-dir test bags do
/// not need this (load path only requires it under the dedicated lab root).
#[must_use]
pub fn external_known_good_provenance_research_ready(path: &std::path::Path) -> bool {
    external_known_good_lab_bag_provenance_allows_load(
        read_external_known_good_provenance_text(path).as_deref(),
    )
}

/// Load external ctrltext bytes for [`compare_external_ctrltext_to_lab_export`].
///
/// **Confinement:** [`external_known_good_load_path_is_allowed`] must pass.
/// **Size cap:** `MAX_MLADF_LAB_FILE_BYTES` (same ceiling as [`read_lab_file_capped`]).
/// **Open:** Unix `O_NOFOLLOW` so a final-component symlink race after the load
/// path check cannot redirect the read (mirrors soft last_apply JSON classify).
/// Intermediate dir symlinks are already refused by parent canonicalize.
///
/// Research only: loaded bytes for compare **≠** product known-good (soft note
/// always `product_known_good=0`) **≠** Pass **≠** C-ready **≠** second unlock.
///
/// # Errors
///
/// Path refused / missing / empty / oversize / symlink / I/O.
pub fn load_external_ctrltext_for_compare(path: &std::path::Path) -> Result<Vec<u8>, DirectError> {
    if !external_known_good_load_path_is_allowed(path) {
        return Err(DirectError::InvalidArgument(format!(
            "external known-good path not allowed (must be under temp_dir or \
             testdata/xdna2_cmd/{EXTERNAL_KNOWN_GOOD_SUBDIR}, .bin/.ctrltext only, \
             no .., no symlink escape, not local post-patch oracle; lab bag needs \
             research-ready PROVENANCE.md: research-only + source/stack markers, \
             no invent_pass=1 / ran_on_npu=1): {}",
            path.display()
        )));
    }
    // Re-check final component is not a symlink immediately before open (TOCTOU
    // window still residual until O_NOFOLLOW open below).
    if let Ok(meta) = std::fs::symlink_metadata(path) {
        if meta.file_type().is_symlink() {
            return Err(DirectError::InvalidArgument(format!(
                "external known-good path is a symlink (refused): {}",
                path.display()
            )));
        }
        if !meta.is_file() {
            return Err(DirectError::InvalidArgument(format!(
                "external known-good missing or not a file: {}",
                path.display()
            )));
        }
        let len = meta.len();
        if len == 0 {
            return Err(DirectError::InvalidArgument(format!(
                "external known-good file is empty: {}",
                path.display()
            )));
        }
        if len > MAX_MLADF_LAB_FILE_BYTES {
            return Err(DirectError::InvalidArgument(format!(
                "external known-good file is {len} bytes > max {MAX_MLADF_LAB_FILE_BYTES}: {}",
                path.display()
            )));
        }
    } else {
        return Err(DirectError::InvalidArgument(format!(
            "external known-good missing or not a file: {}",
            path.display()
        )));
    }
    let bytes = read_external_ctrltext_nofollow_capped(path)?;
    if bytes.is_empty() {
        return Err(DirectError::InvalidArgument(format!(
            "external known-good file is empty: {}",
            path.display()
        )));
    }
    if (bytes.len() as u64) > MAX_MLADF_LAB_FILE_BYTES {
        return Err(DirectError::InvalidArgument(format!(
            "external known-good read size {} > max {MAX_MLADF_LAB_FILE_BYTES}: {}",
            bytes.len(),
            path.display()
        )));
    }
    Ok(bytes)
}

// ---------------------------------------------------------------------------
// Soft bag-drop checklist + e2e fixture path (stamp 8e898123)
// ---------------------------------------------------------------------------
//
// Highest-value offline delta while external known-good **bytes still absent**:
// unit-testable orchestration of body-bounds + journal tokens against the
// last_apply / C2 residual surface, plus fail-closed bag-drop completeness.
// Never invents Pass / residual closed / second C unlock.

/// Fail-closed operator bag-drop checklist for external known-good capture.
///
/// Stamp `8e898123`. Incomplete drops (no ctrltext bytes, PROVENANCE not
/// research-ready, refused local post-patch basenames, VE2/NOT_AIE2 claims)
/// never report complete. **Complete drop ≠ Pass / product known-good /
/// residual closed / C-ready / second unlock.**
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // research flag bag; honesty-first
pub struct ExternalKnownGoodBagDropCheck {
    /// Operator placed non-empty `.bin`/`.ctrltext` bytes for compare.
    pub ctrltext_bytes_present: bool,
    /// PROVENANCE text passes [`validate_external_known_good_provenance_text`].
    pub provenance_research_ready: bool,
    /// Proposed basename is a local pure-direct/post-patch oracle (refused).
    pub refused_local_post_patch_basename: bool,
    /// Platform claim marks VE2 / NOT_AIE2 (refused as AIE2 known-good).
    pub refused_ve2_not_aie2: bool,
    /// True only when bytes + research-ready PROVENANCE and no refuse flags.
    pub bag_drop_complete: bool,
    /// Always **false**: bag drop alone never closes FW residual / invent Pass.
    pub bag_drop_alone_closes_fw_residual: bool,
    /// Stable research detail token.
    pub detail: String,
}

impl Default for ExternalKnownGoodBagDropCheck {
    fn default() -> Self {
        Self {
            ctrltext_bytes_present: false,
            provenance_research_ready: false,
            refused_local_post_patch_basename: false,
            refused_ve2_not_aie2: false,
            bag_drop_complete: false,
            bag_drop_alone_closes_fw_residual: false,
            detail: "bag_drop_absent".into(),
        }
    }
}

impl ExternalKnownGoodBagDropCheck {
    /// Soft research note (always honesty trio hard-coded; never invent Pass).
    #[must_use]
    pub fn format_soft_note(&self) -> String {
        let mut parts = vec![
            "external_known_good_bag_drop_check".into(),
            // Force honesty locks even if a hand-built bag mutates fields.
            "not_c_ready=1".into(),
            "not_ran_on_npu=1".into(),
            "invent_pass=0".into(),
            "product_known_good=0".into(),
            "bag_drop_alone_closes_fw_residual=0".into(),
            format!(
                "ctrltext_bytes_present={}",
                u8::from(self.ctrltext_bytes_present)
            ),
            format!(
                "provenance_research_ready={}",
                u8::from(self.provenance_research_ready)
            ),
            format!(
                "refused_local_post_patch_basename={}",
                u8::from(self.refused_local_post_patch_basename)
            ),
            format!(
                "refused_ve2_not_aie2={}",
                u8::from(self.refused_ve2_not_aie2)
            ),
            format!("bag_drop_complete={}", u8::from(self.bag_drop_complete)),
        ];
        if !self.detail.is_empty() {
            parts.push(format!(
                "detail={}",
                sanitize_soft_note_field_value(&self.detail)
            ));
        }
        parts.join(" ")
    }
}

/// Pure fail-closed bag-drop completeness for external known-good capture.
///
/// Stamp `8e898123`. Inputs are pure (no FS). Use when the operator stages a
/// drop (bytes present?, PROVENANCE body, proposed basename, optional platform
/// claim). Lab bag disk load still uses [`external_known_good_load_path_is_allowed`].
///
/// **Refuse (incomplete):** missing bytes, PROVENANCE not research-ready,
/// refused post-patch basename, VE2/NOT_AIE2 platform claim.
///
/// Complete **≠** product known-good **≠** residual closed.
#[must_use]
pub fn check_external_known_good_bag_drop(
    ctrltext_bytes_present: bool,
    provenance_text: Option<&str>,
    proposed_basename: Option<&str>,
    platform_claim: Option<&str>,
) -> ExternalKnownGoodBagDropCheck {
    let provenance_research_ready =
        external_known_good_lab_bag_provenance_allows_load(provenance_text);
    let refused_local_post_patch_basename = proposed_basename.is_some_and(|b| {
        let p = std::path::Path::new(b);
        // Basename-only check: accept bare names or path-ish inputs.
        is_refused_local_post_patch_as_known_good(p)
            || p.file_name()
                .is_some_and(|n| is_refused_local_post_patch_as_known_good(std::path::Path::new(n)))
    });
    let refused_ve2_not_aie2 = platform_claim.is_some_and(platform_claim_is_not_aie2_known_good);

    let bag_drop_complete = ctrltext_bytes_present
        && provenance_research_ready
        && !refused_local_post_patch_basename
        && !refused_ve2_not_aie2;

    let detail = if bag_drop_complete {
        "bag_drop_complete_research_only"
    } else if refused_local_post_patch_basename {
        "bag_drop_refused_local_post_patch"
    } else if refused_ve2_not_aie2 {
        "bag_drop_refused_ve2_not_aie2"
    } else if !ctrltext_bytes_present && !provenance_research_ready {
        "bag_drop_absent_bytes_and_provenance"
    } else if !ctrltext_bytes_present {
        "bag_drop_absent_ctrltext_bytes"
    } else if !provenance_research_ready {
        "bag_drop_provenance_not_research_ready"
    } else {
        "bag_drop_incomplete"
    };

    ExternalKnownGoodBagDropCheck {
        ctrltext_bytes_present,
        provenance_research_ready,
        refused_local_post_patch_basename,
        refused_ve2_not_aie2,
        bag_drop_complete,
        bag_drop_alone_closes_fw_residual: false,
        detail: detail.into(),
    }
}

/// Soft end-to-end fixture research: residual class + journal tokens + body
/// bounds + bag drop against last_apply / C2 surfaces.
///
/// Stamp `8e898123`. Unit-testable orchestration only — never invents Pass,
/// never claims residual closed, never product known-good, never C-ready.
///
/// Hard locks (always in soft note, even if fields are mutated):
/// - `residual_not_closed=1`
/// - `body_bounds_alone_closes_fw_residual=0`
/// - `bag_drop_alone_closes_fw_residual=0`
/// - `ctrltext_match_alone_closes_fw_residual=0`
/// - honesty trio + `product_known_good=0`
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // research flag bag; honesty-first
pub struct AppLoadE2eFixtureResearch {
    /// Residual class token ([`SiliconResidualClass::as_str`]).
    pub residual_class: String,
    /// True when class is locked APP_LOAD after aiebu+rela.
    pub locked_app_load_after_aiebu: bool,
    /// Userspace ELF+rela green (from last_apply signals / class lock).
    pub userspace_elf_rela_ok: bool,
    /// Journal Failed-cmd status when present (never invented).
    pub journal_failed_cmd: Option<u32>,
    /// Journal chain Status when present (never invented).
    pub journal_chain_status: Option<u32>,
    /// True when pair matches locked C2 APP_LOAD dual codes.
    pub journal_c2_app_load_pair: bool,
    /// True when last_apply detail alone has no failed_cmd/chain tokens (userspace-only).
    pub last_apply_detail_journal_absent: bool,
    /// Body-bounds: any present CU/PDI window OOB (from optional axlf prefix).
    pub body_bounds_any_oob: bool,
    /// Body-bounds: ≥1 CU/PDI section header surface present.
    pub body_bounds_has_surface: bool,
    /// Always **false** (body bounds alone never closes residual).
    pub body_bounds_alone_closes_fw_residual: bool,
    /// Bag drop complete for research compare (still ≠ Pass).
    pub bag_drop_complete: bool,
    /// Always **true**: residual remains open offline without live Pass evidence.
    pub residual_not_closed: bool,
    /// Stable research detail token.
    pub detail: String,
}

impl Default for AppLoadE2eFixtureResearch {
    fn default() -> Self {
        Self {
            residual_class: SiliconResidualClass::Unclassified.as_str().into(),
            locked_app_load_after_aiebu: false,
            userspace_elf_rela_ok: false,
            journal_failed_cmd: None,
            journal_chain_status: None,
            journal_c2_app_load_pair: false,
            last_apply_detail_journal_absent: true,
            body_bounds_any_oob: false,
            body_bounds_has_surface: false,
            body_bounds_alone_closes_fw_residual: false,
            bag_drop_complete: false,
            residual_not_closed: true,
            detail: "e2e_default_unclassified".into(),
        }
    }
}

impl AppLoadE2eFixtureResearch {
    /// Soft research note (always honesty trio + residual_not_closed hard-coded).
    #[must_use]
    pub fn format_soft_note(&self) -> String {
        let mut parts = vec![
            "app_load_e2e_fixture_research".into(),
            // Force honesty locks even if a hand-built bag mutates fields.
            "not_c_ready=1".into(),
            "not_ran_on_npu=1".into(),
            "invent_pass=0".into(),
            "product_known_good=0".into(),
            "residual_not_closed=1".into(),
            "body_bounds_alone_closes_fw_residual=0".into(),
            "bag_drop_alone_closes_fw_residual=0".into(),
            "ctrltext_match_alone_closes_fw_residual=0".into(),
            format!(
                "residual_class={}",
                sanitize_soft_note_field_value(&self.residual_class)
            ),
            format!(
                "locked_app_load_after_aiebu={}",
                u8::from(self.locked_app_load_after_aiebu)
            ),
            format!(
                "userspace_elf_rela_ok={}",
                u8::from(self.userspace_elf_rela_ok)
            ),
            format!(
                "journal_c2_app_load_pair={}",
                u8::from(self.journal_c2_app_load_pair)
            ),
            format!(
                "last_apply_detail_journal_absent={}",
                u8::from(self.last_apply_detail_journal_absent)
            ),
            format!("body_bounds_any_oob={}", u8::from(self.body_bounds_any_oob)),
            format!(
                "body_bounds_has_surface={}",
                u8::from(self.body_bounds_has_surface)
            ),
            format!("bag_drop_complete={}", u8::from(self.bag_drop_complete)),
        ];
        match self.journal_failed_cmd {
            Some(c) => parts.push(format!("journal_failed_cmd=0x{c:x}")),
            None => parts.push("journal_failed_cmd=absent".into()),
        }
        match self.journal_chain_status {
            Some(c) => parts.push(format!("journal_chain_status=0x{c:x}")),
            None => parts.push("journal_chain_status=absent".into()),
        }
        if !self.detail.is_empty() {
            parts.push(format!(
                "detail={}",
                sanitize_soft_note_field_value(&self.detail)
            ));
        }
        parts.join(" ")
    }
}

/// Orchestrate soft e2e fixture research from pure inputs (stamp `8e898123`).
///
/// - `class` / `detail`: last_apply residual class + userspace detail
/// - `journal_text`: optional journal paste (C2 dual codes when present)
/// - `axlf_bytes`: optional axlf/xclbin header prefix for body-bounds surface
/// - `bag_drop`: fail-closed bag completeness ([`check_external_known_good_bag_drop`])
///
/// Always `residual_not_closed=true`. Never invents journal codes from detail
/// alone when tokens are absent. Never Pass.
#[must_use]
pub fn research_app_load_e2e_fixture_path(
    class: SiliconResidualClass,
    detail: Option<&str>,
    journal_text: Option<&str>,
    axlf_bytes: Option<&[u8]>,
    bag_drop: &ExternalKnownGoodBagDropCheck,
) -> AppLoadE2eFixtureResearch {
    let sig = detail.map(str::trim).filter(|s| !s.is_empty()).map_or_else(
        LastApplyDetailSignals::default,
        parse_last_apply_detail_signals,
    );
    let userspace_elf_rela_ok = class.is_locked_app_load_after_aiebu()
        || (sig.aiebu_elf == Some(true) && last_apply_has_rela_evidence(&sig));

    // Journal pair: only from explicit journal paste (never invent from detail).
    let pair = journal_text
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(parse_journal_fw_status_pair);
    let journal_failed_cmd = pair.as_ref().and_then(|p| p.failed_cmd_status);
    let journal_chain_status = pair.as_ref().and_then(|p| p.chain_status);
    let journal_c2_app_load_pair = pair
        .as_ref()
        .is_some_and(JournalFwStatusPair::is_c2_app_load_pair);

    // last_apply detail userspace surface: confirm no invented journal tokens.
    let detail_pair = detail
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(parse_journal_fw_status_pair);
    let last_apply_detail_journal_absent = detail_pair
        .as_ref()
        .is_none_or(|p| p.failed_cmd_status.is_none() && p.chain_status.is_none());

    let bounds = axlf_bytes.map(research_xclbin_cu_pdi_section_body_bounds);
    let body_bounds_any_oob = bounds.as_ref().is_some_and(|b| b.any_oob);
    let body_bounds_has_surface = bounds.as_ref().is_some_and(|b| b.has_cu_pdi_offset_surface);

    let detail_tok = if class.is_locked_app_load_after_aiebu() && journal_c2_app_load_pair {
        if bag_drop.bag_drop_complete {
            "e2e_app_load_c2_pair_bag_present_residual_open"
        } else {
            "e2e_app_load_c2_pair_bag_absent_residual_open"
        }
    } else if class.is_locked_app_load_after_aiebu() {
        if bag_drop.bag_drop_complete {
            "e2e_app_load_userspace_ok_bag_present_residual_open"
        } else {
            "e2e_app_load_userspace_ok_bag_absent_residual_open"
        }
    } else if bag_drop.bag_drop_complete {
        "e2e_class_open_bag_present_residual_open"
    } else {
        "e2e_class_open_bag_absent_residual_open"
    };

    AppLoadE2eFixtureResearch {
        residual_class: class.as_str().into(),
        locked_app_load_after_aiebu: class.is_locked_app_load_after_aiebu(),
        userspace_elf_rela_ok,
        journal_failed_cmd,
        journal_chain_status,
        journal_c2_app_load_pair,
        last_apply_detail_journal_absent,
        body_bounds_any_oob,
        body_bounds_has_surface,
        body_bounds_alone_closes_fw_residual: false,
        bag_drop_complete: bag_drop.bag_drop_complete,
        residual_not_closed: true,
        detail: detail_tok.into(),
    }
}

/// E2e fixture research with locked C2 journal + absent bag (bytes still missing).
///
/// Unit surface for the offline residual while external known-good is absent.
/// Always `residual_not_closed` / honesty trio / invent_pass=0.
#[must_use]
pub fn research_app_load_e2e_fixture_c2_locked_absent_bag() -> AppLoadE2eFixtureResearch {
    let bag = check_external_known_good_bag_drop(false, None, None, None);
    research_app_load_e2e_fixture_path(
        SiliconResidualClass::AppLoadPdiFailAfterAiebu,
        Some(
            "ert_state=5(Error) aiebu_elf=1 rela_count=12 aiebu_rela_patched \
             ctrltext=1688B ran_on_npu=false",
        ),
        Some(c2_locked_app_load_journal_text()),
        None,
        &bag,
    )
}

/// Research-only external compare status for the offline progress scorecard.
///
/// Never product Pass. Soft note always hard-codes invent_pass=0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExternalCompareStatus {
    /// No external bytes supplied / bag empty.
    #[default]
    Absent,
    /// Bytes present (bag or env) but compare not conclusive match/mismatch.
    Loaded,
    /// Research sha match vs lab export (≠ residual closed / ≠ Pass).
    Match,
    /// Bytes present and sha does not match lab export.
    Mismatch,
    /// Load/bag refused (local post_patch basename, VE2/NOT_AIE2, path policy).
    Refused,
}

impl ExternalCompareStatus {
    /// Stable soft-note token (`absent` / `loaded` / `match` / `mismatch` / `refused`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Absent => "absent",
            Self::Loaded => "loaded",
            Self::Match => "match",
            Self::Mismatch => "mismatch",
            Self::Refused => "refused",
        }
    }
}

/// Pure soft offline residual **progress scorecard** (stamp `9cdfe8d0`).
///
/// Composes existing pure surfaces without inventing Pass, residual closed,
/// product known-good, or journal codes from userspace detail alone:
/// residual class, bag-drop checklist, e2e residual_not_closed, body-bounds
/// alone_closes=0, journal token presence when journal paste supplied,
/// external compare status.
///
/// Hard locks (always in soft note, even if fields are mutated):
/// - honesty trio + `product_known_good=0`
/// - `residual_not_closed=1`
/// - `scorecard_alone_closes_fw_residual=0`
/// - `ctrltext_match_alone_closes_fw_residual=0`
/// - `body_bounds_alone_closes_fw_residual=0`
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // research flag bag; honesty-first
pub struct AppLoadOfflineProgressScorecard {
    /// Residual class token ([`SiliconResidualClass::as_str`]).
    pub residual_class: String,
    /// True when class is locked APP_LOAD after aiebu+rela.
    pub locked_app_load_after_aiebu: bool,
    /// Userspace ELF+rela green (from e2e / class lock).
    pub userspace_elf_rela_ok: bool,
    /// Bag drop complete for research compare (still ≠ Pass).
    pub bag_drop_complete: bool,
    /// Echo of e2e residual_not_closed (always true offline without Pass).
    pub e2e_residual_not_closed: bool,
    /// Always **false** (body bounds alone never closes residual).
    pub body_bounds_alone_closes_fw_residual: bool,
    /// True when journal paste yielded failed_cmd and/or chain_status.
    pub journal_tokens_present: bool,
    /// True when pair matches locked C2 APP_LOAD dual codes.
    pub journal_c2_app_load_pair: bool,
    /// External compare research status (never invent Pass).
    pub external_compare_status: ExternalCompareStatus,
    /// True when bag complete **and** compare status is `match` (research only).
    pub scorecard_research_surfaces_ready: bool,
    /// Always **true**: residual remains open offline without live Pass evidence.
    pub residual_not_closed: bool,
    /// Always **false**: scorecard alone never closes FW residual / invent Pass.
    pub scorecard_alone_closes_fw_residual: bool,
    /// Stable research detail token.
    pub detail: String,
}

impl Default for AppLoadOfflineProgressScorecard {
    fn default() -> Self {
        Self {
            residual_class: SiliconResidualClass::Unclassified.as_str().into(),
            locked_app_load_after_aiebu: false,
            userspace_elf_rela_ok: false,
            bag_drop_complete: false,
            e2e_residual_not_closed: true,
            body_bounds_alone_closes_fw_residual: false,
            journal_tokens_present: false,
            journal_c2_app_load_pair: false,
            external_compare_status: ExternalCompareStatus::Absent,
            scorecard_research_surfaces_ready: false,
            residual_not_closed: true,
            scorecard_alone_closes_fw_residual: false,
            detail: "scorecard_default_incomplete".into(),
        }
    }
}

impl AppLoadOfflineProgressScorecard {
    /// Soft research note (always honesty + residual_not_closed hard-coded).
    #[must_use]
    pub fn format_soft_note(&self) -> String {
        let mut parts = vec![
            "app_load_offline_progress_scorecard".into(),
            // Force honesty locks even if a hand-built bag mutates fields.
            "not_c_ready=1".into(),
            "not_ran_on_npu=1".into(),
            "invent_pass=0".into(),
            "product_known_good=0".into(),
            "residual_not_closed=1".into(),
            "scorecard_alone_closes_fw_residual=0".into(),
            "ctrltext_match_alone_closes_fw_residual=0".into(),
            "body_bounds_alone_closes_fw_residual=0".into(),
            format!(
                "residual_class={}",
                sanitize_soft_note_field_value(&self.residual_class)
            ),
            format!(
                "locked_app_load_after_aiebu={}",
                u8::from(self.locked_app_load_after_aiebu)
            ),
            format!(
                "userspace_elf_rela_ok={}",
                u8::from(self.userspace_elf_rela_ok)
            ),
            format!("bag_drop_complete={}", u8::from(self.bag_drop_complete)),
            format!(
                "e2e_residual_not_closed={}",
                u8::from(self.e2e_residual_not_closed)
            ),
            format!(
                "journal_tokens_present={}",
                u8::from(self.journal_tokens_present)
            ),
            format!(
                "journal_c2_app_load_pair={}",
                u8::from(self.journal_c2_app_load_pair)
            ),
            format!(
                "external_compare_status={}",
                self.external_compare_status.as_str()
            ),
            format!(
                "scorecard_research_surfaces_ready={}",
                u8::from(self.scorecard_research_surfaces_ready)
            ),
        ];
        if !self.detail.is_empty() {
            parts.push(format!(
                "detail={}",
                sanitize_soft_note_field_value(&self.detail)
            ));
        }
        parts.join(" ")
    }
}

/// Derive research-only external compare status (never Pass).
///
/// Priority: refuse flags → sha match → mismatch → loaded (bytes only) → absent.
#[must_use]
pub fn external_compare_status_from_surfaces(
    bag_drop: &ExternalKnownGoodBagDropCheck,
    compare: Option<&ExternalCtrltextCompareResearch>,
    external_load_refused: bool,
) -> ExternalCompareStatus {
    if external_load_refused
        || bag_drop.refused_local_post_patch_basename
        || bag_drop.refused_ve2_not_aie2
    {
        return ExternalCompareStatus::Refused;
    }
    if let Some(c) = compare {
        if c.external_present {
            return if c.sha_match {
                ExternalCompareStatus::Match
            } else {
                ExternalCompareStatus::Mismatch
            };
        }
    }
    if bag_drop.ctrltext_bytes_present {
        return ExternalCompareStatus::Loaded;
    }
    ExternalCompareStatus::Absent
}

/// Compose offline residual progress scorecard from pure surfaces (stamp `9cdfe8d0`).
///
/// - `class` / `bag_drop` / `e2e`: existing pure research surfaces
/// - `compare`: optional external ctrltext compare (never invent Pass on match)
/// - `external_load_refused`: path/policy refuse even when compare absent
///
/// Always `residual_not_closed=true` and `scorecard_alone_closes_fw_residual=false`.
/// Never invents journal codes (reads `e2e` journal fields only).
#[must_use]
pub fn research_app_load_offline_progress_scorecard(
    class: SiliconResidualClass,
    bag_drop: &ExternalKnownGoodBagDropCheck,
    e2e: &AppLoadE2eFixtureResearch,
    compare: Option<&ExternalCtrltextCompareResearch>,
    external_load_refused: bool,
) -> AppLoadOfflineProgressScorecard {
    let external_compare_status =
        external_compare_status_from_surfaces(bag_drop, compare, external_load_refused);
    let journal_tokens_present =
        e2e.journal_failed_cmd.is_some() || e2e.journal_chain_status.is_some();
    let scorecard_research_surfaces_ready =
        bag_drop.bag_drop_complete && external_compare_status == ExternalCompareStatus::Match;

    let detail = if class.is_locked_app_load_after_aiebu()
        && e2e.journal_c2_app_load_pair
        && !bag_drop.bag_drop_complete
        && external_compare_status == ExternalCompareStatus::Absent
    {
        "scorecard_c2_locked_bag_absent_residual_open"
    } else if external_compare_status == ExternalCompareStatus::Refused {
        "scorecard_external_refused_residual_open"
    } else if scorecard_research_surfaces_ready {
        // Match + bag complete still residual open (never close from scorecard).
        "scorecard_surfaces_ready_residual_still_open"
    } else if bag_drop.bag_drop_complete {
        // Distinguish compare-ran mismatch vs bytes-present-no-compare vs other.
        match external_compare_status {
            ExternalCompareStatus::Mismatch => "scorecard_bag_complete_mismatch_residual_open",
            ExternalCompareStatus::Loaded => {
                "scorecard_bag_complete_loaded_compare_pending_residual_open"
            }
            // Match handled above; Absent/Refused edges keep incomplete token.
            _ => "scorecard_bag_complete_compare_incomplete_residual_open",
        }
    } else if class.is_locked_app_load_after_aiebu() {
        "scorecard_app_load_userspace_ok_bag_incomplete_residual_open"
    } else {
        "scorecard_incomplete_residual_open"
    };

    AppLoadOfflineProgressScorecard {
        residual_class: class.as_str().into(),
        locked_app_load_after_aiebu: class.is_locked_app_load_after_aiebu(),
        userspace_elf_rela_ok: e2e.userspace_elf_rela_ok,
        bag_drop_complete: bag_drop.bag_drop_complete,
        e2e_residual_not_closed: e2e.residual_not_closed,
        body_bounds_alone_closes_fw_residual: false,
        journal_tokens_present,
        journal_c2_app_load_pair: e2e.journal_c2_app_load_pair,
        external_compare_status,
        scorecard_research_surfaces_ready,
        residual_not_closed: true,
        scorecard_alone_closes_fw_residual: false,
        detail: detail.into(),
    }
}

/// Scorecard with locked C2 journal + absent bag (bytes still missing offline).
///
/// Unit surface: incomplete scorecard, residual_not_closed, honesty trio.
#[must_use]
pub fn research_app_load_offline_progress_scorecard_c2_locked_absent_bag()
-> AppLoadOfflineProgressScorecard {
    let bag = check_external_known_good_bag_drop(false, None, None, None);
    let e2e = research_app_load_e2e_fixture_c2_locked_absent_bag();
    research_app_load_offline_progress_scorecard(
        SiliconResidualClass::AppLoadPdiFailAfterAiebu,
        &bag,
        &e2e,
        None,
        false,
    )
}

/// Operator extract-to-bag checklist text (pure; stamp `588c1c72` + prior steps).
///
/// Fail-closed refuse tokens unit-locked: local post_patch, VE2/NOT_AIE2,
/// invent_pass / product Pass / residual closed / second unlock.
/// Research-only — does not invent bytes or claim Pass.
/// Dry-run when bag absent reports explicit `blocked_on_external_bytes=1`
/// (see [`research_external_ctrltext_compare_dry_run`]).
/// Lab export fingerprint **auto-feed** into dry-run when ELF/export path is
/// available (see [`research_lab_export_fingerprint_feed`]).
/// Bag + feed + dry-run **composition scorecard** reports the same blocker in one
/// soft row (see [`research_app_load_bag_feed_dry_run_scorecard`]).
/// Extract → bag → scorecard **pipeline** folds public section markers into that
/// row (see [`research_app_load_extract_bag_scorecard_pipeline`]).
#[must_use]
pub fn format_external_known_good_operator_extract_checklist() -> String {
    [
        "external_known_good_operator_extract_checklist",
        "stamp=588c1c72",
        "step1=run_working_public_stack_xrt_or_dd_same_mladf_key_when_possible",
        "step2=capture_post_patch_ctrltext_or_aiebu_elf_after_peel_document_path",
        "step3=copy_bytes_to_testdata_xdna2_cmd_external_known_good_clear_basename",
        "step4=fill_PROVENANCE_capture_table_research_ready_markers",
        "step5=confirm_bag_drop_complete_via_soft_classify_or_unit_bag_drop",
        "step6=set_XDNA2_LAB_EXTERNAL_CTRLTEXT_confined_path_soft_compare",
        "step7=compare_external_ctrltext_to_lab_export_match_neq_pass",
        "step8=soft_external_ctrltext_compare_dry_run_blocked_on_external_bytes_when_absent",
        "step9=lab_export_fingerprint_auto_feed_into_dry_run_when_export_path_ready",
        "step10=soft_app_load_bag_feed_dry_run_scorecard_blocked_on_external_bytes_after_auto_feed",
        "step11=soft_app_load_extract_bag_scorecard_pipeline_markers_without_bytes_still_blocked",
        "refuse_local_post_patch=1",
        "refuse_instr_post_patch_bin=1",
        "refuse_instr_vendor_post_patch_bin=1",
        "refuse_ctrl_post_patch_bin=1",
        "refuse_instr_raw_txn_bin=1",
        "refuse_ve2_not_aie2=1",
        "refuse_validate_xclbin_as_aie2_known_good=1",
        "bag_drop_complete_neq_pass=1",
        "blocked_on_external_bytes_when_bag_absent=1",
        "lab_export_auto_feed=1",
        "extract_alone_closes_fw_residual=0",
        "pipeline_alone_closes_fw_residual=0",
        "ctrltext_match_alone_closes_fw_residual=0",
        "scorecard_alone_closes_fw_residual=0",
        "dry_run_alone_closes_fw_residual=0",
        "feed_alone_closes_fw_residual=0",
        "residual_not_closed=1",
        "invent_pass=0",
        "not_c_ready=1",
        "not_ran_on_npu=1",
        "product_known_good=0",
        "no_second_c_unlock=1",
        "c_unlock_b17b0119_consumed=1",
    ]
    .join(" ")
}

// ---------------------------------------------------------------------------
// External ctrltext compare dry-run (stamp `6033158a`)
// ---------------------------------------------------------------------------

/// Soft-note bag name for [`ExternalCtrltextCompareDryRun::format_soft_note`].
pub const EXTERNAL_CTRLTEXT_COMPARE_DRY_RUN_NOTE: &str = "external_ctrltext_compare_dry_run";

/// Explicit offline blocker token when external known-good bytes are still missing.
///
/// Emitted as `blocked_on_external_bytes=1` on the dry-run soft note when the bag
/// is absent (and not on a refuse path). Never invents match / Pass.
pub const BLOCKED_ON_EXTERNAL_BYTES_TOKEN: &str = "blocked_on_external_bytes";

/// Pure offline **compare dry-run** when external known-good may still be absent
/// (stamp `6033158a`).
///
/// Advances residual research beyond the progress scorecard alone by making
/// the bag-absent path **explicit** (`blocked_on_external_bytes=1`) instead of
/// a silent `known_good_absent` skim. Optional refuse paths (local post_patch
/// basename, VE2/NOT_AIE2 platform claim) are unit-locked. When both lab export
/// and external bytes are present, surfaces compare results but **match ≠ Pass**.
///
/// Hard locks (always in soft note, even if fields are mutated):
/// - honesty trio + `product_known_good=0`
/// - `residual_not_closed=1`
/// - `dry_run_alone_closes_fw_residual=0`
/// - `ctrltext_match_alone_closes_fw_residual=0`
/// - `scorecard_alone_closes_fw_residual=0`
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // research flag bag; honesty-first
pub struct ExternalCtrltextCompareDryRun {
    /// Lab fingerprint has non-empty unpatched sha (export ready for compare).
    pub lab_export_ready: bool,
    /// Non-empty external bytes supplied and **not** on a refuse path.
    pub external_present: bool,
    /// Explicit bag-absent blocker (primary when external missing, not refused).
    pub blocked_on_external_bytes: bool,
    /// Local pure-direct / post_patch basename refused as known-good.
    pub refused_local_post_patch: bool,
    /// VE2 / NOT_AIE2 platform claim refused as AIE2 known-good.
    pub refused_ve2_not_aie2: bool,
    /// Tag when VE2 / NOT_AIE2 fixtures drive the refuse path (`platform_not_aie2=1`).
    pub platform_not_aie2: bool,
    /// True when [`compare_external_ctrltext_to_lab_export`] ran with both sides.
    pub compare_ran: bool,
    /// Echo of compare `sha_match` when `compare_ran` (research only; ≠ Pass).
    pub sha_match: bool,
    /// Echo of compare `structural_match` when `compare_ran`.
    pub structural_match: bool,
    /// VE2 / NOT_AIE2 refuse companion: set when xclbin extract dry-run marks
    /// NOT_AIE2 **or** when claim-only VE2 refuses (echo of the same refuse matrix;
    /// not limited to “headers were passed”).
    pub xclbin_extract_refused_ve2: bool,
    /// Always **true**: residual remains open offline without live Pass evidence.
    pub residual_not_closed: bool,
    /// Always **false**: dry-run alone never closes FW residual / invent Pass.
    pub dry_run_alone_closes_fw_residual: bool,
    /// Stable research detail token.
    pub detail: String,
}

impl Default for ExternalCtrltextCompareDryRun {
    fn default() -> Self {
        Self {
            lab_export_ready: false,
            external_present: false,
            blocked_on_external_bytes: true,
            refused_local_post_patch: false,
            refused_ve2_not_aie2: false,
            platform_not_aie2: false,
            compare_ran: false,
            sha_match: false,
            structural_match: false,
            xclbin_extract_refused_ve2: false,
            residual_not_closed: true,
            dry_run_alone_closes_fw_residual: false,
            detail: "dry_run_default_blocked_on_external_bytes".into(),
        }
    }
}

impl ExternalCtrltextCompareDryRun {
    /// Soft research note (always honesty + residual_not_closed hard-coded).
    ///
    /// Honesty tokens are **hard-coded** (not from mutable struct fields) so a
    /// hand-built bag cannot invent Pass / residual closed / product known-good.
    #[must_use]
    pub fn format_soft_note(&self) -> String {
        let mut parts = vec![
            EXTERNAL_CTRLTEXT_COMPARE_DRY_RUN_NOTE.into(),
            // Force honesty locks even if a hand-built bag mutates fields.
            "not_c_ready=1".into(),
            "not_ran_on_npu=1".into(),
            "invent_pass=0".into(),
            "product_known_good=0".into(),
            "residual_not_closed=1".into(),
            "dry_run_alone_closes_fw_residual=0".into(),
            "ctrltext_match_alone_closes_fw_residual=0".into(),
            "scorecard_alone_closes_fw_residual=0".into(),
            format!("lab_export_ready={}", u8::from(self.lab_export_ready)),
            format!("external_present={}", u8::from(self.external_present)),
            format!(
                "{}={}",
                BLOCKED_ON_EXTERNAL_BYTES_TOKEN,
                u8::from(self.blocked_on_external_bytes)
            ),
            format!(
                "refused_local_post_patch={}",
                u8::from(self.refused_local_post_patch)
            ),
            format!(
                "refused_ve2_not_aie2={}",
                u8::from(self.refused_ve2_not_aie2)
            ),
            format!("platform_not_aie2={}", u8::from(self.platform_not_aie2)),
            format!("compare_ran={}", u8::from(self.compare_ran)),
            format!("sha_match={}", u8::from(self.sha_match)),
            format!("structural_match={}", u8::from(self.structural_match)),
            format!(
                "xclbin_extract_refused_ve2={}",
                u8::from(self.xclbin_extract_refused_ve2)
            ),
        ];
        if !self.detail.is_empty() {
            parts.push(format!(
                "detail={}",
                sanitize_soft_note_field_value(&self.detail)
            ));
        }
        parts.join(" ")
    }
}

/// True when a lab fingerprint export is ready for research compare (non-empty sha).
#[must_use]
pub fn lab_fingerprint_export_is_ready(lab: Option<&CtrltextFingerprintExport>) -> bool {
    lab.is_some_and(|e| !e.unpatched_sha256_hex.is_empty() && e.ctrltext_len.is_some())
}

/// Pure external ctrltext compare **dry-run** (stamp `6033158a`).
///
/// - `lab`: optional lab fingerprint export (from
///   [`export_ctrltext_fingerprint_research`]); when ready → `lab_export_ready=1`.
/// - `external`: optional external ctrltext bytes; empty/None →
///   **`blocked_on_external_bytes=1`** (explicit; not silent absent) unless refused.
/// - `proposed_basename`: optional basename for local post_patch refuse.
/// - `platform_claim`: optional path/PROVENANCE claim for VE2/NOT_AIE2 refuse.
/// - `xclbin_header` / `xclbin_section_text`: optional companion xclbin extract
///   dry-run (refuse VE2; never product known-good / never invent PDI layout).
/// - `external_load_refused`: soft/confinement load failed (non-empty path set but
///   bytes unusable); when true and external absent (not refuse-matrix), detail is
///   `dry_run_external_load_refused_blocked_on_bytes` (still `blocked_on_external_bytes=1`).
///
/// Compare runs **only** when lab export is ready **and** external non-empty (and
/// not refused). External present without lab-ready →
/// `dry_run_lab_export_not_ready_external_present` with `compare_ran=0` (no default
/// empty-sha compare). **Even** `sha_match=1` leaves `residual_not_closed=1` and
/// `dry_run_alone_closes_fw_residual=0`.
///
/// Never invents Pass / C-ready / product known-good / second unlock.
#[must_use]
pub fn research_external_ctrltext_compare_dry_run(
    lab: Option<&CtrltextFingerprintExport>,
    external: Option<&[u8]>,
    proposed_basename: Option<&str>,
    platform_claim: Option<&str>,
) -> ExternalCtrltextCompareDryRun {
    research_external_ctrltext_compare_dry_run_ex(
        lab,
        external,
        proposed_basename,
        platform_claim,
        None,
        None,
        false,
    )
}

/// Extended dry-run with optional confined xclbin extract refuse companion and
/// external-load-refused detail.
///
/// Same honesty locks as [`research_external_ctrltext_compare_dry_run`]. When
/// xclbin inputs are provided **or** claim is VE2/NOT_AIE2, composes
/// [`research_xclbin_extract_dry_run`] / VE2 refuse helper and surfaces
/// `xclbin_extract_refused_ve2` / `platform_not_aie2` (claim-only VE2 also sets
/// the xclbin refuse echo — not only when headers were passed).
/// Never grants product known-good / Pass from xclbin eligibility.
#[must_use]
#[allow(clippy::too_many_arguments, clippy::too_many_lines)] // research surface: lab + external + refuse + xclbin + load flag
pub fn research_external_ctrltext_compare_dry_run_ex(
    lab: Option<&CtrltextFingerprintExport>,
    external: Option<&[u8]>,
    proposed_basename: Option<&str>,
    platform_claim: Option<&str>,
    xclbin_header: Option<&[u8]>,
    xclbin_section_text: Option<&str>,
    external_load_refused: bool,
) -> ExternalCtrltextCompareDryRun {
    let lab_export_ready = lab_fingerprint_export_is_ready(lab);
    let claim = platform_claim.unwrap_or("").trim();
    let refused_ve2 = !claim.is_empty() && platform_claim_is_not_aie2_known_good(claim);
    let refused_post = proposed_basename
        .is_some_and(|b| is_refused_local_post_patch_as_known_good(std::path::Path::new(b)));

    // Optional xclbin companion (public magic + markers only; never invent PDI).
    // Single flag: claim-only VE2 also sets xclbin_extract_refused_ve2 (refuse echo).
    let xclbin_refused_ve2 =
        if xclbin_header.is_some() || xclbin_section_text.is_some() || refused_ve2 {
            let hdr = xclbin_header.unwrap_or(&[]);
            let x = if refused_ve2 && xclbin_header.is_none() && xclbin_section_text.is_none() {
                // Claim-only VE2: use locked refuse helper path.
                research_xclbin_extract_ve2_fixture_refused(&[], None)
            } else {
                research_xclbin_extract_dry_run(hdr, xclbin_section_text, platform_claim)
            };
            x.platform_not_aie2_known_good || refused_ve2
        } else {
            false
        };

    let platform_not_aie2 = refused_ve2 || xclbin_refused_ve2;

    // Refuse paths take priority over blocked_on_external_bytes.
    if refused_post || refused_ve2 || xclbin_refused_ve2 {
        let detail = if refused_post {
            "dry_run_refused_local_post_patch"
        } else {
            "dry_run_refused_ve2_not_aie2"
        };
        return ExternalCtrltextCompareDryRun {
            lab_export_ready,
            external_present: false,
            blocked_on_external_bytes: false,
            refused_local_post_patch: refused_post,
            refused_ve2_not_aie2: refused_ve2 || xclbin_refused_ve2,
            platform_not_aie2,
            compare_ran: false,
            sha_match: false,
            structural_match: false,
            xclbin_extract_refused_ve2: xclbin_refused_ve2,
            residual_not_closed: true,
            dry_run_alone_closes_fw_residual: false,
            detail: detail.into(),
        };
    }

    let Some(ext) = external.filter(|b| !b.is_empty()) else {
        let detail = if external_load_refused {
            // Env/path set but confinement/read failed (not post_patch/VE2 refuse).
            "dry_run_external_load_refused_blocked_on_bytes"
        } else if lab_export_ready {
            "dry_run_lab_export_ready_blocked_on_external_bytes"
        } else {
            "dry_run_blocked_on_external_bytes"
        };
        return ExternalCtrltextCompareDryRun {
            lab_export_ready,
            external_present: false,
            blocked_on_external_bytes: true,
            refused_local_post_patch: false,
            refused_ve2_not_aie2: false,
            platform_not_aie2: false,
            compare_ran: false,
            sha_match: false,
            structural_match: false,
            xclbin_extract_refused_ve2: false,
            residual_not_closed: true,
            dry_run_alone_closes_fw_residual: false,
            detail: detail.into(),
        };
    };

    // External present: compare only when lab export is ready (no empty-sha compare).
    if !lab_export_ready {
        return ExternalCtrltextCompareDryRun {
            lab_export_ready: false,
            external_present: true,
            blocked_on_external_bytes: false,
            refused_local_post_patch: false,
            refused_ve2_not_aie2: false,
            platform_not_aie2: false,
            compare_ran: false,
            sha_match: false,
            structural_match: false,
            xclbin_extract_refused_ve2: false,
            residual_not_closed: true,
            dry_run_alone_closes_fw_residual: false,
            detail: "dry_run_lab_export_not_ready_external_present".into(),
        };
    }

    // Both lab-ready and external available (match still ≠ Pass).
    // lab_export_ready already requires non-empty unpatched sha + ctrltext_len.
    let lab_export = lab.cloned().unwrap_or_default();
    let cmp = compare_external_ctrltext_to_lab_export(&lab_export, Some(ext));
    let detail = if cmp.sha_match && cmp.structural_match {
        "dry_run_sha_and_structural_match_residual_still_open"
    } else if cmp.sha_match {
        "dry_run_sha_match_residual_still_open"
    } else if cmp.structural_match {
        "dry_run_structural_match_sha_diff_residual_open"
    } else {
        "dry_run_compare_mismatch_residual_open"
    };
    ExternalCtrltextCompareDryRun {
        lab_export_ready: true,
        external_present: true,
        blocked_on_external_bytes: false,
        refused_local_post_patch: false,
        refused_ve2_not_aie2: false,
        platform_not_aie2: false,
        compare_ran: true,
        sha_match: cmp.sha_match,
        structural_match: cmp.structural_match,
        xclbin_extract_refused_ve2: false,
        residual_not_closed: true,
        dry_run_alone_closes_fw_residual: false,
        detail: detail.into(),
    }
}

/// Dry-run with locked C2 residual context + absent external bag (bytes missing).
///
/// Unit surface: `blocked_on_external_bytes=1`, honesty trio, residual_not_closed.
/// Does **not** invent lab export from disk (lab_export_ready=0 unless a real
/// export is passed via the full API).
#[must_use]
pub fn research_external_ctrltext_compare_dry_run_c2_locked_absent_bag()
-> ExternalCtrltextCompareDryRun {
    research_external_ctrltext_compare_dry_run(None, None, None, None)
}

// ---------------------------------------------------------------------------
// Lab export fingerprint auto-feed into compare dry-run (stamp `d2e287ab`)
// ---------------------------------------------------------------------------

/// Soft-note bag name for [`LabExportFingerprintFeed::format_soft_note`].
pub const LAB_EXPORT_FINGERPRINT_FEED_NOTE: &str = "lab_export_fingerprint_feed";

/// Pure **lab export fingerprint auto-feed** for compare dry-run (stamp `d2e287ab`).
///
/// Derives optional [`CtrltextFingerprintExport`] from export-path ELF bytes when
/// available (research surface already used by soft classify). **Never invents
/// external ctrltext** and **never invents unpatched sha** from last_apply
/// signals alone (content size / aiebu flags are incomplete without ELF bytes).
///
/// Auto-feed into [`research_external_ctrltext_compare_dry_run_from_lab_feed`] so
/// dry-run is not only hand-fed when optional args exist: lab-ready + bag absent
/// → `lab_export_ready=1` + `blocked_on_external_bytes=1`.
///
/// Hard locks (always in soft note): honesty trio + `product_known_good=0` +
/// `residual_not_closed=1` + `feed_alone_closes_fw_residual=0` +
/// `dry_run_alone_closes_fw_residual=0` + `ctrltext_match_alone_closes_fw_residual=0`
/// + `scorecard_alone_closes_fw_residual=0`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // research flag bag; honesty-first
pub struct LabExportFingerprintFeed {
    /// True when export has non-empty unpatched sha + ctrltext_len (ready for dry-run).
    pub lab_export_ready: bool,
    /// Always **true** from constructors — marks auto-feed research surface.
    pub lab_export_auto_feed: bool,
    /// Stable source token: `elf_bytes` / `last_apply_signals` / `absent` /
    /// `refused_local_post_patch` / `platform_not_aie2`.
    pub feed_source: String,
    /// Local pure-direct / post_patch basename refused as known-good for feed.
    pub refused_local_post_patch: bool,
    /// VE2 / NOT_AIE2 platform claim on feed path (`platform_not_aie2=1`).
    pub platform_not_aie2: bool,
    /// Built export when ELF bytes parseable (still known_good absent / not external).
    pub export: Option<CtrltextFingerprintExport>,
    /// Echo of last_apply `content_size` when signals path used (no invent sha).
    pub last_apply_content_size_b: Option<u32>,
    /// Echo of last_apply `aiebu_elf` when signals path used.
    pub last_apply_aiebu_elf: Option<bool>,
    /// Echo of last_apply `rela_count` when signals path used.
    pub last_apply_rela_count: Option<usize>,
    /// Always **true**: feed alone never closes FW residual.
    pub residual_not_closed: bool,
    /// Always **false**: feed alone never closes residual / invent Pass.
    pub feed_alone_closes_fw_residual: bool,
    /// Stable research detail token.
    pub detail: String,
}

impl Default for LabExportFingerprintFeed {
    fn default() -> Self {
        Self {
            lab_export_ready: false,
            lab_export_auto_feed: true,
            feed_source: "absent".into(),
            refused_local_post_patch: false,
            platform_not_aie2: false,
            export: None,
            last_apply_content_size_b: None,
            last_apply_aiebu_elf: None,
            last_apply_rela_count: None,
            residual_not_closed: true,
            feed_alone_closes_fw_residual: false,
            detail: "feed_default_absent".into(),
        }
    }
}

impl LabExportFingerprintFeed {
    /// Soft research note (honesty + residual_not_closed hard-coded).
    ///
    /// Honesty tokens are **hard-coded** (not from mutable struct fields).
    #[must_use]
    pub fn format_soft_note(&self) -> String {
        let mut parts = vec![
            LAB_EXPORT_FINGERPRINT_FEED_NOTE.into(),
            "not_c_ready=1".into(),
            "not_ran_on_npu=1".into(),
            "invent_pass=0".into(),
            "product_known_good=0".into(),
            "residual_not_closed=1".into(),
            "feed_alone_closes_fw_residual=0".into(),
            "dry_run_alone_closes_fw_residual=0".into(),
            "ctrltext_match_alone_closes_fw_residual=0".into(),
            "scorecard_alone_closes_fw_residual=0".into(),
            // Always report auto-feed=1 from production path (hard-coded lock).
            "lab_export_auto_feed=1".into(),
            format!("lab_export_ready={}", u8::from(self.lab_export_ready)),
            format!(
                "feed_source={}",
                sanitize_soft_note_field_value(&self.feed_source)
            ),
            format!(
                "refused_local_post_patch={}",
                u8::from(self.refused_local_post_patch)
            ),
            format!("platform_not_aie2={}", u8::from(self.platform_not_aie2)),
        ];
        if let Some(n) = self.last_apply_content_size_b {
            parts.push(format!("last_apply_content_size_b={n}"));
        }
        if let Some(a) = self.last_apply_aiebu_elf {
            parts.push(format!("last_apply_aiebu_elf={}", u8::from(a)));
        }
        if let Some(r) = self.last_apply_rela_count {
            parts.push(format!("last_apply_rela_count={r}"));
        }
        if !self.detail.is_empty() {
            parts.push(format!(
                "detail={}",
                sanitize_soft_note_field_value(&self.detail)
            ));
        }
        parts.join(" ")
    }

    /// Lab export borrow for dry-run when ready (None when incomplete/absent).
    ///
    /// Belt-and-suspenders: requires both `lab_export_ready` **and**
    /// [`lab_fingerprint_export_is_ready`] on the attached export (hand-mutated
    /// bags cannot invent a ready export with empty sha).
    #[must_use]
    pub fn ready_export(&self) -> Option<&CtrltextFingerprintExport> {
        if self.lab_export_ready && lab_fingerprint_export_is_ready(self.export.as_ref()) {
            self.export.as_ref()
        } else {
            None
        }
    }
}

/// Build lab export feed from optional ELF bytes only (export-path surface).
///
/// Empty / None → absent incomplete feed. Valid aiebu → ready when fingerprint
/// has unpatched sha. Never invents external bytes / product known-good.
#[must_use]
pub fn research_lab_export_fingerprint_feed_from_elf(
    elf: Option<&[u8]>,
) -> LabExportFingerprintFeed {
    research_lab_export_fingerprint_feed_from_elf_ex(elf, None, None)
}

/// ELF feed with optional local post_patch basename / VE2 platform refuse.
///
/// Refuse takes priority: post_patch basename or VE2 claim → refuse-only feed
/// (`export: None`, `lab_export_ready=0`). ELF bytes are **not** attached on
/// refuse paths (combined [`research_lab_export_fingerprint_feed`] also drops
/// ELF when refuse wins). Refuse flags still surface for dry-run merge.
#[must_use]
pub fn research_lab_export_fingerprint_feed_from_elf_ex(
    elf: Option<&[u8]>,
    proposed_basename: Option<&str>,
    platform_claim: Option<&str>,
) -> LabExportFingerprintFeed {
    let refused_post = proposed_basename
        .is_some_and(|b| is_refused_local_post_patch_as_known_good(std::path::Path::new(b)));
    let claim = platform_claim.unwrap_or("").trim();
    let refused_ve2 = !claim.is_empty() && platform_claim_is_not_aie2_known_good(claim);

    if refused_post {
        return LabExportFingerprintFeed {
            lab_export_ready: false,
            lab_export_auto_feed: true,
            feed_source: "refused_local_post_patch".into(),
            refused_local_post_patch: true,
            platform_not_aie2: refused_ve2,
            export: None,
            residual_not_closed: true,
            feed_alone_closes_fw_residual: false,
            detail: "feed_refused_local_post_patch".into(),
            ..LabExportFingerprintFeed::default()
        };
    }
    if refused_ve2 {
        return LabExportFingerprintFeed {
            lab_export_ready: false,
            lab_export_auto_feed: true,
            feed_source: "platform_not_aie2".into(),
            refused_local_post_patch: false,
            platform_not_aie2: true,
            export: None,
            residual_not_closed: true,
            feed_alone_closes_fw_residual: false,
            detail: "feed_refused_ve2_not_aie2".into(),
            ..LabExportFingerprintFeed::default()
        };
    }

    let Some(bytes) = elf.filter(|b| !b.is_empty()) else {
        return LabExportFingerprintFeed {
            lab_export_ready: false,
            lab_export_auto_feed: true,
            feed_source: "absent".into(),
            detail: "feed_elf_absent".into(),
            ..LabExportFingerprintFeed::default()
        };
    };

    let exp = export_ctrltext_fingerprint_research(bytes);
    let ready = lab_fingerprint_export_is_ready(Some(&exp));
    LabExportFingerprintFeed {
        lab_export_ready: ready,
        lab_export_auto_feed: true,
        feed_source: "elf_bytes".into(),
        refused_local_post_patch: false,
        platform_not_aie2: false,
        export: Some(exp),
        residual_not_closed: true,
        feed_alone_closes_fw_residual: false,
        detail: if ready {
            "feed_elf_export_ready".into()
        } else {
            "feed_elf_export_not_ready".into()
        },
        ..LabExportFingerprintFeed::default()
    }
}

/// Feed from last_apply detail **signals only** — never invents unpatched sha.
///
/// When aiebu + content size present → incomplete feed (`lab_export_ready=0`)
/// with explicit detail (need ELF bytes for ready export). When empty → absent.
#[must_use]
pub fn research_lab_export_fingerprint_feed_from_last_apply_signals(
    aiebu_elf: Option<bool>,
    content_size_b: Option<u32>,
    rela_count: Option<usize>,
) -> LabExportFingerprintFeed {
    // No ELF → never ready (do not invent sha from size alone).
    let has_any = aiebu_elf.is_some() || content_size_b.is_some() || rela_count.is_some();
    if !has_any {
        return LabExportFingerprintFeed {
            lab_export_ready: false,
            lab_export_auto_feed: true,
            feed_source: "absent".into(),
            detail: "feed_last_apply_signals_absent".into(),
            ..LabExportFingerprintFeed::default()
        };
    }
    let detail = if aiebu_elf == Some(true) && content_size_b.is_some() {
        "feed_last_apply_signals_incomplete_need_elf_bytes"
    } else if aiebu_elf == Some(true) {
        "feed_last_apply_aiebu_without_content_size"
    } else {
        "feed_last_apply_signals_incomplete"
    };
    LabExportFingerprintFeed {
        lab_export_ready: false,
        lab_export_auto_feed: true,
        feed_source: "last_apply_signals".into(),
        refused_local_post_patch: false,
        platform_not_aie2: false,
        export: None,
        last_apply_content_size_b: content_size_b,
        last_apply_aiebu_elf: aiebu_elf,
        last_apply_rela_count: rela_count,
        residual_not_closed: true,
        feed_alone_closes_fw_residual: false,
        detail: detail.into(),
    }
}

/// Feed from parsed last_apply detail signals (pure; no I/O).
#[must_use]
pub fn research_lab_export_fingerprint_feed_from_last_apply_detail(
    sig: &LastApplyDetailSignals,
) -> LabExportFingerprintFeed {
    research_lab_export_fingerprint_feed_from_last_apply_signals(
        sig.aiebu_elf,
        sig.content_size_b,
        sig.rela_count,
    )
}

/// Prefer ELF bytes when present; else fall back to last_apply detail signals.
///
/// Optional basename / platform refuse apply to the ELF path (post_patch / VE2).
/// On refuse, ELF is **dropped** (refuse-only feed; no ready export invent).
/// Never invents external ctrltext. Soft classify auto-feed uses this.
/// Combined absent (no ELF, no signals) uses detail `feed_absent_no_elf_no_signals`.
#[must_use]
pub fn research_lab_export_fingerprint_feed(
    elf: Option<&[u8]>,
    last_apply_detail: Option<&str>,
    proposed_basename: Option<&str>,
    platform_claim: Option<&str>,
) -> LabExportFingerprintFeed {
    // Refuse surfaces always win (no invent ready export from post_patch / VE2).
    let refused_post = proposed_basename
        .is_some_and(|b| is_refused_local_post_patch_as_known_good(std::path::Path::new(b)));
    let claim = platform_claim.unwrap_or("").trim();
    let refused_ve2 = !claim.is_empty() && platform_claim_is_not_aie2_known_good(claim);
    if refused_post || refused_ve2 {
        return research_lab_export_fingerprint_feed_from_elf_ex(
            None,
            proposed_basename,
            platform_claim,
        );
    }

    if elf.is_some_and(|b| !b.is_empty()) {
        return research_lab_export_fingerprint_feed_from_elf_ex(
            elf,
            proposed_basename,
            platform_claim,
        );
    }

    // No ELF: pure last_apply signals (incomplete without bytes).
    if let Some(d) = last_apply_detail.filter(|s| !s.trim().is_empty()) {
        let sig = parse_last_apply_detail_signals(d);
        return research_lab_export_fingerprint_feed_from_last_apply_detail(&sig);
    }

    // Combined absent: distinct from ELF-only or signals-only absent details.
    LabExportFingerprintFeed {
        lab_export_ready: false,
        lab_export_auto_feed: true,
        feed_source: "absent".into(),
        detail: "feed_absent_no_elf_no_signals".into(),
        ..LabExportFingerprintFeed::default()
    }
}

/// Auto-feed lab fingerprint into external compare dry-run (stamp `d2e287ab`).
///
/// When feed is lab-ready and external absent → `lab_export_ready=1` +
/// `blocked_on_external_bytes=1`. Feed refuse flags **force** refuse even when
/// the caller passes a non-refused basename/platform (cannot clobber feed refuse).
/// Match ≠ Pass. Never invents external bytes.
#[must_use]
pub fn research_external_ctrltext_compare_dry_run_from_lab_feed(
    feed: &LabExportFingerprintFeed,
    external: Option<&[u8]>,
    proposed_basename: Option<&str>,
    platform_claim: Option<&str>,
) -> ExternalCtrltextCompareDryRun {
    research_external_ctrltext_compare_dry_run_from_lab_feed_ex(
        feed,
        external,
        proposed_basename,
        platform_claim,
        None,
        None,
        false,
    )
}

/// Extended auto-feed dry-run (xclbin companion + external_load_refused).
///
/// Merges feed refuse into dry-run refuse matrix. Feed refuse flags **force**
/// refuse: non-refused caller basename/platform cannot drop post_patch/VE2.
/// Lab export from feed only when ready (no empty-sha invent).
#[must_use]
#[allow(clippy::too_many_arguments)] // research surface mirrors dry_run_ex
pub fn research_external_ctrltext_compare_dry_run_from_lab_feed_ex(
    feed: &LabExportFingerprintFeed,
    external: Option<&[u8]>,
    proposed_basename: Option<&str>,
    platform_claim: Option<&str>,
    xclbin_header: Option<&[u8]>,
    xclbin_section_text: Option<&str>,
    external_load_refused: bool,
) -> ExternalCtrltextCompareDryRun {
    // Force refuse from feed flags: caller non-refused overrides must not drop.
    let basename = if feed.refused_local_post_patch {
        let caller_refused = proposed_basename.filter(|b| {
            !b.is_empty() && is_refused_local_post_patch_as_known_good(std::path::Path::new(b))
        });
        Some(caller_refused.unwrap_or("instr_post_patch.bin"))
    } else {
        proposed_basename
    };
    let platform = if feed.platform_not_aie2 {
        let caller_ve2 = platform_claim
            .filter(|c| !c.trim().is_empty() && platform_claim_is_not_aie2_known_good(c));
        Some(caller_ve2.unwrap_or("PLATFORM=VE2_EDGE NOT_AIE2_KNOWN_GOOD=1"))
    } else {
        platform_claim
    };
    research_external_ctrltext_compare_dry_run_ex(
        feed.ready_export(),
        external,
        basename,
        platform,
        xclbin_header,
        xclbin_section_text,
        external_load_refused,
    )
}

// ---------------------------------------------------------------------------
// Bag-drop + lab feed + dry-run soft scorecard (stamp `2b01425e`)
// ---------------------------------------------------------------------------

/// Soft-note bag name for [`AppLoadBagFeedDryRunScorecard::format_soft_note`].
pub const APP_LOAD_BAG_FEED_DRY_RUN_SCORECARD_NOTE: &str = "app_load_bag_feed_dry_run_scorecard";

/// Pure soft **composition scorecard**: bag-drop + lab export feed + compare
/// dry-run after auto-feed (stamp `2b01425e`).
///
/// Beyond feed alone (`d2e287ab`): one research row that surfaces explicit
/// **`blocked_on_external_bytes=1`** when auto-feed is lab-ready and the external
/// bag is still absent — without inventing Pass / residual closed.
///
/// Composes existing pure surfaces only:
/// - [`ExternalKnownGoodBagDropCheck`] / [`check_external_known_good_bag_drop`]
/// - [`LabExportFingerprintFeed`] / feed ready flags
/// - [`research_external_ctrltext_compare_dry_run_from_lab_feed_ex`]
///
/// Hard locks (always in soft note, even if fields are mutated):
/// - honesty trio + `product_known_good=0`
/// - `residual_not_closed=1`
/// - `bag_drop_alone_closes_fw_residual=0`
/// - `feed_alone_closes_fw_residual=0`
/// - `dry_run_alone_closes_fw_residual=0`
/// - `scorecard_alone_closes_fw_residual=0`
/// - `ctrltext_match_alone_closes_fw_residual=0`
/// - `lab_export_auto_feed=1`
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // research flag bag; honesty-first
pub struct AppLoadBagFeedDryRunScorecard {
    /// Echo of bag-drop completeness (still ≠ Pass).
    pub bag_drop_complete: bool,
    /// Echo of bag-drop ctrltext_bytes_present.
    pub bag_ctrltext_bytes_present: bool,
    /// Lab fingerprint ready for dry-run (from feed / dry-run).
    pub lab_export_ready: bool,
    /// Always **true** from constructors — auto-feed research surface.
    pub lab_export_auto_feed: bool,
    /// Explicit bag-absent blocker after auto-feed (primary when external missing).
    pub blocked_on_external_bytes: bool,
    /// Non-empty external on dry-run path (not refuse).
    pub external_present: bool,
    /// Compare ran with both sides present.
    pub compare_ran: bool,
    /// Echo of dry-run sha_match (research only; ≠ Pass).
    pub sha_match: bool,
    /// Echo of dry-run structural_match.
    pub structural_match: bool,
    /// Local post_patch refuse (bag and/or feed and/or dry-run).
    pub refused_local_post_patch: bool,
    /// VE2 / NOT_AIE2 refuse (bag and/or feed and/or dry-run).
    pub refused_ve2_not_aie2: bool,
    /// Tag when VE2 / NOT_AIE2 fixtures drive refuse (`platform_not_aie2=1`).
    pub platform_not_aie2: bool,
    /// Always **true**: residual remains open offline without live Pass evidence.
    pub residual_not_closed: bool,
    /// Always **false**: bag-drop alone never closes residual.
    pub bag_drop_alone_closes_fw_residual: bool,
    /// Always **false**: feed alone never closes residual.
    pub feed_alone_closes_fw_residual: bool,
    /// Always **false**: dry-run alone never closes residual.
    pub dry_run_alone_closes_fw_residual: bool,
    /// Always **false**: this scorecard alone never closes residual / invent Pass.
    pub scorecard_alone_closes_fw_residual: bool,
    /// Stable research detail token (distinct outcomes; not one lumped label).
    pub detail: String,
}

impl Default for AppLoadBagFeedDryRunScorecard {
    fn default() -> Self {
        Self {
            bag_drop_complete: false,
            bag_ctrltext_bytes_present: false,
            lab_export_ready: false,
            lab_export_auto_feed: true,
            blocked_on_external_bytes: true,
            external_present: false,
            compare_ran: false,
            sha_match: false,
            structural_match: false,
            refused_local_post_patch: false,
            refused_ve2_not_aie2: false,
            platform_not_aie2: false,
            residual_not_closed: true,
            bag_drop_alone_closes_fw_residual: false,
            feed_alone_closes_fw_residual: false,
            dry_run_alone_closes_fw_residual: false,
            scorecard_alone_closes_fw_residual: false,
            detail: "scorecard_default_blocked_on_external_bytes".into(),
        }
    }
}

impl AppLoadBagFeedDryRunScorecard {
    /// Soft research note (honesty + residual_not_closed hard-coded).
    ///
    /// Honesty tokens are **hard-coded** (not from mutable struct fields).
    #[must_use]
    pub fn format_soft_note(&self) -> String {
        let mut parts = vec![
            APP_LOAD_BAG_FEED_DRY_RUN_SCORECARD_NOTE.into(),
            // Force honesty locks even if a hand-built bag mutates fields.
            "not_c_ready=1".into(),
            "not_ran_on_npu=1".into(),
            "invent_pass=0".into(),
            "product_known_good=0".into(),
            "residual_not_closed=1".into(),
            "bag_drop_alone_closes_fw_residual=0".into(),
            "feed_alone_closes_fw_residual=0".into(),
            "dry_run_alone_closes_fw_residual=0".into(),
            "scorecard_alone_closes_fw_residual=0".into(),
            "ctrltext_match_alone_closes_fw_residual=0".into(),
            // Always report auto-feed=1 from production path (hard-coded lock).
            "lab_export_auto_feed=1".into(),
            format!("bag_drop_complete={}", u8::from(self.bag_drop_complete)),
            format!(
                "bag_ctrltext_bytes_present={}",
                u8::from(self.bag_ctrltext_bytes_present)
            ),
            format!("lab_export_ready={}", u8::from(self.lab_export_ready)),
            format!(
                "{}={}",
                BLOCKED_ON_EXTERNAL_BYTES_TOKEN,
                u8::from(self.blocked_on_external_bytes)
            ),
            format!("external_present={}", u8::from(self.external_present)),
            format!("compare_ran={}", u8::from(self.compare_ran)),
            format!("sha_match={}", u8::from(self.sha_match)),
            format!("structural_match={}", u8::from(self.structural_match)),
            format!(
                "refused_local_post_patch={}",
                u8::from(self.refused_local_post_patch)
            ),
            format!(
                "refused_ve2_not_aie2={}",
                u8::from(self.refused_ve2_not_aie2)
            ),
            format!("platform_not_aie2={}", u8::from(self.platform_not_aie2)),
        ];
        if !self.detail.is_empty() {
            parts.push(format!(
                "detail={}",
                sanitize_soft_note_field_value(&self.detail)
            ));
        }
        parts.join(" ")
    }
}

/// Compose bag-drop + lab feed + dry-run scorecard (stamp `2b01425e`).
///
/// - `bag_drop`: fail-closed bag completeness
/// - `feed`: lab export fingerprint feed (auto-feed surface)
/// - `external`: optional external ctrltext (None/empty → blocked when not refused)
/// - `proposed_basename` / `platform_claim`: refuse matrix (post_patch / VE2)
///
/// Bag-drop and feed refuse flags **force** refuse even when the caller passes
/// a non-refused basename/platform (cannot clobber prior refuse). Match ≠ Pass.
/// Always `residual_not_closed=true` and all alone_closes locks false.
#[must_use]
pub fn research_app_load_bag_feed_dry_run_scorecard(
    bag_drop: &ExternalKnownGoodBagDropCheck,
    feed: &LabExportFingerprintFeed,
    external: Option<&[u8]>,
    proposed_basename: Option<&str>,
    platform_claim: Option<&str>,
) -> AppLoadBagFeedDryRunScorecard {
    research_app_load_bag_feed_dry_run_scorecard_ex(
        bag_drop,
        feed,
        external,
        proposed_basename,
        platform_claim,
        false,
    )
}

/// Extended composition scorecard (external_load_refused detail path).
///
/// Same honesty locks as [`research_app_load_bag_feed_dry_run_scorecard`].
/// When `external_load_refused` and bag absent (not refuse-matrix), detail is
/// `scorecard_external_load_refused_blocked_on_bytes` (still
/// `blocked_on_external_bytes=1`).
#[must_use]
pub fn research_app_load_bag_feed_dry_run_scorecard_ex(
    bag_drop: &ExternalKnownGoodBagDropCheck,
    feed: &LabExportFingerprintFeed,
    external: Option<&[u8]>,
    proposed_basename: Option<&str>,
    platform_claim: Option<&str>,
    external_load_refused: bool,
) -> AppLoadBagFeedDryRunScorecard {
    // Force refuse from bag_drop + feed: non-refused caller overrides must not drop.
    let basename = if bag_drop.refused_local_post_patch_basename || feed.refused_local_post_patch {
        let caller_refused = proposed_basename.filter(|b| {
            !b.is_empty() && is_refused_local_post_patch_as_known_good(std::path::Path::new(b))
        });
        Some(caller_refused.unwrap_or("instr_post_patch.bin"))
    } else {
        proposed_basename
    };
    let platform = if bag_drop.refused_ve2_not_aie2 || feed.platform_not_aie2 {
        let caller_ve2 = platform_claim
            .filter(|c| !c.trim().is_empty() && platform_claim_is_not_aie2_known_good(c));
        Some(caller_ve2.unwrap_or("PLATFORM=VE2_EDGE NOT_AIE2_KNOWN_GOOD=1"))
    } else {
        platform_claim
    };

    let dry = research_external_ctrltext_compare_dry_run_from_lab_feed_ex(
        feed,
        external,
        basename,
        platform,
        None,
        None,
        external_load_refused,
    );

    let refused_post = dry.refused_local_post_patch
        || bag_drop.refused_local_post_patch_basename
        || feed.refused_local_post_patch;
    let refused_ve2 =
        dry.refused_ve2_not_aie2 || bag_drop.refused_ve2_not_aie2 || feed.platform_not_aie2;
    let platform_not_aie2 =
        dry.platform_not_aie2 || feed.platform_not_aie2 || bag_drop.refused_ve2_not_aie2;

    // Distinct detail tokens — refuse first, then blocked, then compare outcomes.
    let detail = if refused_post {
        "scorecard_refused_local_post_patch_residual_open"
    } else if refused_ve2 {
        "scorecard_refused_ve2_not_aie2_residual_open"
    } else if dry.blocked_on_external_bytes {
        if external_load_refused {
            "scorecard_external_load_refused_blocked_on_bytes"
        } else if dry.lab_export_ready {
            "scorecard_lab_ready_blocked_on_external_bytes"
        } else {
            "scorecard_blocked_on_external_bytes_lab_not_ready"
        }
    } else if dry.compare_ran && dry.sha_match && dry.structural_match {
        "scorecard_sha_and_structural_match_residual_still_open"
    } else if dry.compare_ran && dry.sha_match {
        "scorecard_sha_match_residual_still_open"
    } else if dry.compare_ran && dry.structural_match {
        // Align with dry-run `dry_run_structural_match_sha_diff_residual_open`.
        "scorecard_structural_match_sha_diff_residual_open"
    } else if dry.compare_ran {
        "scorecard_mismatch_residual_open"
    } else if dry.external_present && !dry.lab_export_ready {
        "scorecard_lab_export_not_ready_external_present_residual_open"
    } else {
        "scorecard_bag_feed_dry_run_residual_open"
    };

    AppLoadBagFeedDryRunScorecard {
        bag_drop_complete: bag_drop.bag_drop_complete,
        bag_ctrltext_bytes_present: bag_drop.ctrltext_bytes_present,
        lab_export_ready: dry.lab_export_ready,
        lab_export_auto_feed: true,
        blocked_on_external_bytes: dry.blocked_on_external_bytes,
        external_present: dry.external_present,
        compare_ran: dry.compare_ran,
        sha_match: dry.sha_match,
        structural_match: dry.structural_match,
        refused_local_post_patch: refused_post,
        refused_ve2_not_aie2: refused_ve2,
        platform_not_aie2,
        residual_not_closed: true,
        bag_drop_alone_closes_fw_residual: false,
        feed_alone_closes_fw_residual: false,
        dry_run_alone_closes_fw_residual: false,
        scorecard_alone_closes_fw_residual: false,
        detail: detail.into(),
    }
}

/// Unit surface: locked C2 residual context + absent external bag (bytes missing).
///
/// Lab feed absent → not ready; dry-run / scorecard → `blocked_on_external_bytes=1`.
/// Honesty trio + residual_not_closed + all alone_closes=0.
#[must_use]
pub fn research_app_load_bag_feed_dry_run_scorecard_c2_locked_absent_bag()
-> AppLoadBagFeedDryRunScorecard {
    let bag = check_external_known_good_bag_drop(false, None, None, None);
    let feed = research_lab_export_fingerprint_feed(None, None, None, None);
    research_app_load_bag_feed_dry_run_scorecard(&bag, &feed, None, None, None)
}

// ---------------------------------------------------------------------------
// Extract → bag-drop → composition scorecard pipeline (stamp `588c1c72`)
// ---------------------------------------------------------------------------

/// Soft-note bag name for [`AppLoadExtractBagScorecardPipeline::format_soft_note`].
pub const APP_LOAD_EXTRACT_BAG_SCORECARD_PIPELINE_NOTE: &str =
    "app_load_extract_bag_scorecard_pipeline";

/// Pure **extract → bag-drop → composition scorecard** pipeline (stamp `588c1c72`).
///
/// Beyond bag+feed+dry-run alone (`2b01425e`): one research row that folds
/// [`research_xclbin_extract_dry_run`] (public section markers / magic) into
/// bag-drop + [`research_app_load_bag_feed_dry_run_scorecard_ex`].
///
/// When public section markers exist **without** ctrltext bytes, still reports
/// explicit **`blocked_on_external_bytes=1`** and residual open — markers alone
/// never invent Pass / bag-drop complete / product known-good.
///
/// Composes existing pure surfaces only:
/// - [`XclbinExtractResearchDryRun`] / [`research_xclbin_extract_dry_run`]
/// - [`ExternalKnownGoodBagDropCheck`] / [`check_external_known_good_bag_drop`]
/// - [`LabExportFingerprintFeed`]
/// - [`AppLoadBagFeedDryRunScorecard`] / bag-feed composition
///
/// Hard locks (always in soft note, even if fields are mutated):
/// - honesty trio + `product_known_good=0`
/// - `residual_not_closed=1`
/// - `extract_alone_closes_fw_residual=0`
/// - `bag_drop_alone_closes_fw_residual=0`
/// - `feed_alone_closes_fw_residual=0`
/// - `dry_run_alone_closes_fw_residual=0`
/// - `scorecard_alone_closes_fw_residual=0`
/// - `pipeline_alone_closes_fw_residual=0`
/// - `ctrltext_match_alone_closes_fw_residual=0`
/// - `lab_export_auto_feed=1`
/// - `do_not_invent_pdi_layout=1` / `public_pdi_image_decode=0`
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // research flag bag; honesty-first
pub struct AppLoadExtractBagScorecardPipeline {
    /// Echo of extract magic_xclbin2.
    pub extract_magic_xclbin2: bool,
    /// Echo of extract AIE_PARTITION marker presence.
    pub extract_has_aie_partition_marker: bool,
    /// Echo of extract section marker count (capped).
    pub extract_section_marker_count: u8,
    /// Research bag candidate from extract (never product Pass).
    pub extract_eligible_as_aie2_external_known_good: bool,
    /// Extract VE2 / NOT_AIE2 refuse.
    pub extract_platform_not_aie2_known_good: bool,
    /// Echo of bag-drop completeness (still ≠ Pass).
    pub bag_drop_complete: bool,
    /// Echo of bag-drop ctrltext_bytes_present.
    pub bag_ctrltext_bytes_present: bool,
    /// Lab fingerprint ready for dry-run (from composition scorecard).
    pub lab_export_ready: bool,
    /// Always **true** from constructors — auto-feed research surface.
    pub lab_export_auto_feed: bool,
    /// Explicit bag-absent blocker after auto-feed (primary when external missing).
    pub blocked_on_external_bytes: bool,
    /// Non-empty external on dry-run path (not refuse).
    pub external_present: bool,
    /// Compare ran with both sides present.
    pub compare_ran: bool,
    /// Echo of dry-run/scorecard sha_match (research only; ≠ Pass).
    pub sha_match: bool,
    /// Echo of dry-run/scorecard structural_match.
    pub structural_match: bool,
    /// Local post_patch refuse (extract path / bag / feed / scorecard).
    pub refused_local_post_patch: bool,
    /// VE2 / NOT_AIE2 refuse (extract and/or bag/feed/scorecard).
    pub refused_ve2_not_aie2: bool,
    /// Tag when VE2 / NOT_AIE2 fixtures drive refuse (`platform_not_aie2=1`).
    pub platform_not_aie2: bool,
    /// Always **true**: residual remains open offline without live Pass evidence.
    pub residual_not_closed: bool,
    /// Always **false**: extract markers alone never close residual.
    pub extract_alone_closes_fw_residual: bool,
    /// Always **false**: bag-drop alone never closes residual.
    pub bag_drop_alone_closes_fw_residual: bool,
    /// Always **false**: feed alone never closes residual.
    pub feed_alone_closes_fw_residual: bool,
    /// Always **false**: dry-run alone never closes residual.
    pub dry_run_alone_closes_fw_residual: bool,
    /// Always **false**: bag+feed scorecard alone never closes residual.
    pub scorecard_alone_closes_fw_residual: bool,
    /// Always **false**: this pipeline alone never closes residual / invent Pass.
    pub pipeline_alone_closes_fw_residual: bool,
    /// Stable research detail token (distinct outcomes; not one lumped label).
    pub detail: String,
}

impl Default for AppLoadExtractBagScorecardPipeline {
    fn default() -> Self {
        Self {
            extract_magic_xclbin2: false,
            extract_has_aie_partition_marker: false,
            extract_section_marker_count: 0,
            extract_eligible_as_aie2_external_known_good: false,
            extract_platform_not_aie2_known_good: false,
            bag_drop_complete: false,
            bag_ctrltext_bytes_present: false,
            lab_export_ready: false,
            lab_export_auto_feed: true,
            blocked_on_external_bytes: true,
            external_present: false,
            compare_ran: false,
            sha_match: false,
            structural_match: false,
            refused_local_post_patch: false,
            refused_ve2_not_aie2: false,
            platform_not_aie2: false,
            residual_not_closed: true,
            extract_alone_closes_fw_residual: false,
            bag_drop_alone_closes_fw_residual: false,
            feed_alone_closes_fw_residual: false,
            dry_run_alone_closes_fw_residual: false,
            scorecard_alone_closes_fw_residual: false,
            pipeline_alone_closes_fw_residual: false,
            detail: "pipeline_default_blocked_on_external_bytes".into(),
        }
    }
}

impl AppLoadExtractBagScorecardPipeline {
    /// Soft research note (honesty + residual_not_closed + alone_closes hard-coded).
    ///
    /// Honesty tokens are **hard-coded** (not from mutable struct fields).
    #[must_use]
    pub fn format_soft_note(&self) -> String {
        let mut parts = vec![
            APP_LOAD_EXTRACT_BAG_SCORECARD_PIPELINE_NOTE.into(),
            // Force honesty locks even if a hand-built bag mutates fields.
            "not_c_ready=1".into(),
            "not_ran_on_npu=1".into(),
            "invent_pass=0".into(),
            "product_known_good=0".into(),
            "residual_not_closed=1".into(),
            "extract_alone_closes_fw_residual=0".into(),
            "bag_drop_alone_closes_fw_residual=0".into(),
            "feed_alone_closes_fw_residual=0".into(),
            "dry_run_alone_closes_fw_residual=0".into(),
            "scorecard_alone_closes_fw_residual=0".into(),
            "pipeline_alone_closes_fw_residual=0".into(),
            "ctrltext_match_alone_closes_fw_residual=0".into(),
            "lab_export_auto_feed=1".into(),
            "do_not_invent_pdi_layout=1".into(),
            "public_pdi_image_decode=0".into(),
            format!(
                "extract_magic_xclbin2={}",
                u8::from(self.extract_magic_xclbin2)
            ),
            format!(
                "extract_has_aie_partition_marker={}",
                u8::from(self.extract_has_aie_partition_marker)
            ),
            format!(
                "extract_section_marker_count={}",
                self.extract_section_marker_count
            ),
            format!(
                "extract_eligible_as_aie2_external_known_good={}",
                u8::from(self.extract_eligible_as_aie2_external_known_good)
            ),
            format!(
                "extract_platform_not_aie2_known_good={}",
                u8::from(self.extract_platform_not_aie2_known_good)
            ),
            format!("bag_drop_complete={}", u8::from(self.bag_drop_complete)),
            format!(
                "bag_ctrltext_bytes_present={}",
                u8::from(self.bag_ctrltext_bytes_present)
            ),
            format!("lab_export_ready={}", u8::from(self.lab_export_ready)),
            format!(
                "{}={}",
                BLOCKED_ON_EXTERNAL_BYTES_TOKEN,
                u8::from(self.blocked_on_external_bytes)
            ),
            format!("external_present={}", u8::from(self.external_present)),
            format!("compare_ran={}", u8::from(self.compare_ran)),
            format!("sha_match={}", u8::from(self.sha_match)),
            format!("structural_match={}", u8::from(self.structural_match)),
            format!(
                "refused_local_post_patch={}",
                u8::from(self.refused_local_post_patch)
            ),
            format!(
                "refused_ve2_not_aie2={}",
                u8::from(self.refused_ve2_not_aie2)
            ),
            format!("platform_not_aie2={}", u8::from(self.platform_not_aie2)),
        ];
        if !self.detail.is_empty() {
            parts.push(format!(
                "detail={}",
                sanitize_soft_note_field_value(&self.detail)
            ));
        }
        parts.join(" ")
    }
}

/// Compose extract dry-run + bag-drop + bag-feed composition scorecard (stamp `588c1c72`).
///
/// - `extract`: public xclbin magic/markers research (never invents PDI / product known-good)
/// - `bag_drop`: fail-closed bag completeness
/// - `feed`: lab export fingerprint feed (auto-feed surface)
/// - `external`: optional external ctrltext (None/empty → blocked when not refused)
/// - `proposed_basename` / `platform_claim`: refuse matrix (post_patch / VE2)
///
/// Extract VE2 refuse, bag-drop refuse, and feed refuse **force** refuse even when
/// the caller passes a non-refused basename/platform (cannot clobber prior refuse).
/// Markers present without bytes → still `blocked_on_external_bytes=1`.
/// Match ≠ Pass. Always `residual_not_closed=true` and all alone_closes locks false.
#[must_use]
pub fn research_app_load_extract_bag_scorecard_pipeline(
    extract: &XclbinExtractResearchDryRun,
    bag_drop: &ExternalKnownGoodBagDropCheck,
    feed: &LabExportFingerprintFeed,
    external: Option<&[u8]>,
    proposed_basename: Option<&str>,
    platform_claim: Option<&str>,
) -> AppLoadExtractBagScorecardPipeline {
    research_app_load_extract_bag_scorecard_pipeline_ex(
        extract,
        bag_drop,
        feed,
        external,
        proposed_basename,
        platform_claim,
        false,
    )
}

/// Extended extract→bag→scorecard pipeline (external_load_refused detail path).
///
/// Same honesty locks as [`research_app_load_extract_bag_scorecard_pipeline`].
/// When `external_load_refused` and bag absent (not refuse-matrix), detail is
/// `pipeline_external_load_refused_blocked_on_bytes` (still
/// `blocked_on_external_bytes=1`).
#[must_use]
#[allow(clippy::too_many_arguments)] // research surface: extract + bag + feed + refuse + load flag
pub fn research_app_load_extract_bag_scorecard_pipeline_ex(
    extract: &XclbinExtractResearchDryRun,
    bag_drop: &ExternalKnownGoodBagDropCheck,
    feed: &LabExportFingerprintFeed,
    external: Option<&[u8]>,
    proposed_basename: Option<&str>,
    platform_claim: Option<&str>,
    external_load_refused: bool,
) -> AppLoadExtractBagScorecardPipeline {
    // Force VE2 refuse from extract into platform_claim so bag/feed/scorecard merge.
    let platform = if extract.platform_not_aie2_known_good
        || bag_drop.refused_ve2_not_aie2
        || feed.platform_not_aie2
    {
        let caller_ve2 = platform_claim
            .filter(|c| !c.trim().is_empty() && platform_claim_is_not_aie2_known_good(c));
        Some(caller_ve2.unwrap_or("PLATFORM=VE2_EDGE NOT_AIE2_KNOWN_GOOD=1"))
    } else {
        platform_claim
    };

    let scorecard = research_app_load_bag_feed_dry_run_scorecard_ex(
        bag_drop,
        feed,
        external,
        proposed_basename,
        platform,
        external_load_refused,
    );

    let refused_post = scorecard.refused_local_post_patch
        || bag_drop.refused_local_post_patch_basename
        || feed.refused_local_post_patch;
    let refused_ve2 = scorecard.refused_ve2_not_aie2
        || extract.platform_not_aie2_known_good
        || bag_drop.refused_ve2_not_aie2
        || feed.platform_not_aie2;
    let platform_not_aie2 = scorecard.platform_not_aie2
        || extract.platform_not_aie2_known_good
        || feed.platform_not_aie2
        || bag_drop.refused_ve2_not_aie2;

    // Distinct detail tokens — refuse first, then marker+blocked, then compare.
    let markers_present = extract.has_aie_partition_marker
        || extract.section_marker_count > 0
        || extract.magic_xclbin2;
    let detail = if refused_post {
        "pipeline_refused_local_post_patch_residual_open"
    } else if refused_ve2 {
        "pipeline_refused_ve2_not_aie2_residual_open"
    } else if scorecard.blocked_on_external_bytes {
        if external_load_refused {
            "pipeline_external_load_refused_blocked_on_bytes"
        } else if extract.eligible_as_aie2_external_known_good {
            // Research-eligible extract still needs real ctrltext bytes.
            "pipeline_extract_eligible_research_only_blocked_on_external_bytes"
        } else if extract.has_aie_partition_marker || extract.section_marker_count > 0 {
            "pipeline_extract_markers_present_blocked_on_external_bytes"
        } else if extract.magic_xclbin2 {
            "pipeline_extract_magic_only_blocked_on_external_bytes"
        } else if scorecard.lab_export_ready {
            "pipeline_lab_ready_blocked_on_external_bytes"
        } else {
            "pipeline_blocked_on_external_bytes_lab_not_ready"
        }
    } else if scorecard.compare_ran && scorecard.sha_match && scorecard.structural_match {
        "pipeline_sha_and_structural_match_residual_still_open"
    } else if scorecard.compare_ran && scorecard.sha_match {
        "pipeline_sha_match_residual_still_open"
    } else if scorecard.compare_ran && scorecard.structural_match {
        "pipeline_structural_match_sha_diff_residual_open"
    } else if scorecard.compare_ran {
        "pipeline_mismatch_residual_open"
    } else if scorecard.external_present && !scorecard.lab_export_ready {
        "pipeline_lab_export_not_ready_external_present_residual_open"
    } else if markers_present {
        "pipeline_extract_markers_present_residual_open"
    } else {
        "pipeline_extract_bag_scorecard_residual_open"
    };

    AppLoadExtractBagScorecardPipeline {
        extract_magic_xclbin2: extract.magic_xclbin2,
        extract_has_aie_partition_marker: extract.has_aie_partition_marker,
        extract_section_marker_count: extract.section_marker_count,
        extract_eligible_as_aie2_external_known_good: extract.eligible_as_aie2_external_known_good,
        extract_platform_not_aie2_known_good: extract.platform_not_aie2_known_good,
        bag_drop_complete: bag_drop.bag_drop_complete,
        bag_ctrltext_bytes_present: bag_drop.ctrltext_bytes_present,
        lab_export_ready: scorecard.lab_export_ready,
        lab_export_auto_feed: true,
        blocked_on_external_bytes: scorecard.blocked_on_external_bytes,
        external_present: scorecard.external_present,
        compare_ran: scorecard.compare_ran,
        sha_match: scorecard.sha_match,
        structural_match: scorecard.structural_match,
        refused_local_post_patch: refused_post,
        refused_ve2_not_aie2: refused_ve2,
        platform_not_aie2,
        residual_not_closed: true,
        extract_alone_closes_fw_residual: false,
        bag_drop_alone_closes_fw_residual: false,
        feed_alone_closes_fw_residual: false,
        dry_run_alone_closes_fw_residual: false,
        scorecard_alone_closes_fw_residual: false,
        pipeline_alone_closes_fw_residual: false,
        detail: detail.into(),
    }
}

/// Build pipeline from raw extract inputs + bag/feed surfaces (convenience).
///
/// Pure: builds [`research_xclbin_extract_dry_run`] then composes bag+feed scorecard.
/// Never invents product known-good / Pass / residual closed.
#[must_use]
#[allow(clippy::too_many_arguments)] // research convenience: header + text + claim + bag + feed
pub fn research_app_load_extract_bag_scorecard_pipeline_from_inputs(
    header_bytes: &[u8],
    section_or_info_text: Option<&str>,
    extract_platform_claim: Option<&str>,
    bag_drop: &ExternalKnownGoodBagDropCheck,
    feed: &LabExportFingerprintFeed,
    external: Option<&[u8]>,
    proposed_basename: Option<&str>,
    platform_claim: Option<&str>,
) -> AppLoadExtractBagScorecardPipeline {
    let extract =
        research_xclbin_extract_dry_run(header_bytes, section_or_info_text, extract_platform_claim);
    research_app_load_extract_bag_scorecard_pipeline(
        &extract,
        bag_drop,
        feed,
        external,
        proposed_basename,
        platform_claim,
    )
}

/// Unit surface: locked C2 residual + empty extract + absent external bag.
///
/// Extract has no markers; bag absent; lab feed not ready →
/// `blocked_on_external_bytes=1`. Honesty trio + residual_not_closed + all
/// alone_closes=0. Never invents Pass.
#[must_use]
pub fn research_app_load_extract_bag_scorecard_pipeline_c2_locked_absent_bag()
-> AppLoadExtractBagScorecardPipeline {
    let extract = research_xclbin_extract_dry_run(&[], None, None);
    let bag = check_external_known_good_bag_drop(false, None, None, None);
    let feed = research_lab_export_fingerprint_feed(None, None, None, None);
    research_app_load_extract_bag_scorecard_pipeline(&extract, &bag, &feed, None, None, None)
}

// ---------------------------------------------------------------------------
// Pure public-stack section marker fixture + confined marker path (stamp `93ccd077`)
// ---------------------------------------------------------------------------

/// Soft-note bag name for [`PublicStackSectionMarkerFixtureResearch::format_soft_note`].
pub const PUBLIC_STACK_SECTION_MARKER_FIXTURE_NOTE: &str = "public_stack_section_marker_fixture";

/// Soft-note bag name for confined markers path / stage research.
pub const RESEARCH_XCLBIN_SECTION_MARKERS_PATH_NOTE: &str = "research_xclbin_section_markers_path";

/// Env: optional **confined path** to research-only xclbin section marker text
/// (public names only; never product known-good / never PDI decode).
///
/// Allowed under temp root ∪ `testdata/xdna2_cmd/external_known_good/`;
/// extensions `.txt` / `.sections` / `.info`; refuse post_patch basenames;
/// O_NOFOLLOW open. Soft feeds extract half of the pipeline under PRINT.
pub const ENV_LAB_XCLBIN_SECTION_MARKERS_PATH: &str = "XDNA2_LAB_XCLBIN_SECTION_MARKERS_PATH";

/// Locked pure fixture: public xclbin section **names only** (no bodies, no PDI).
///
/// Drawn from [`XCLBIN_RESEARCH_SECTION_MARKERS`] / CU-PDI identity markers so
/// unit tests can drive
/// `pipeline_extract_markers_present_blocked_on_external_bytes` without inventing
/// Pass or product known-good. Format mimics a short `xclbinutil --info` paste.
pub const PUBLIC_STACK_XCLBIN_SECTION_MARKER_FIXTURE_TEXT: &str = "\
Section: AIE_PARTITION
Section: BITSTREAM
Section: MEM_TOPOLOGY
Section: IP_LAYOUT
Section: CONNECTIVITY
Section: BUILD_METADATA
";

/// Cap for research section-marker text (paste or file). Same order as soft paste.
pub const MAX_XCLBIN_SECTION_MARKERS_TEXT_BYTES: usize = 64 * 1024;

/// Pure **public-stack section marker fixture** research (stamp `93ccd077`).
///
/// Beyond extract→bag→scorecard pipeline alone (`588c1c72`): a locked pure
/// fixture of public section **names** that feeds the extract half of the
/// pipeline and **still** asserts `blocked_on_external_bytes=1` + residual open.
/// Never invents ctrltext/PDI bytes, Pass, or product known-good.
///
/// Hard locks (always in soft note, even if fields are mutated):
/// - honesty trio + `product_known_good=0`
/// - `residual_not_closed=1`
/// - `fixture_alone_closes_fw_residual=0`
/// - `markers_alone_closes_fw_residual=0`
/// - `pipeline_alone_closes_fw_residual=0`
/// - `ctrltext_match_alone_closes_fw_residual=0`
/// - `do_not_invent_pdi_layout=1` / `public_pdi_image_decode=0`
/// - `invents_ctrltext_bytes=0`
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // research flag bag; honesty-first
pub struct PublicStackSectionMarkerFixtureResearch {
    /// Distinct public section name markers found in the locked fixture text.
    pub section_marker_count: u8,
    /// Fixture lists `AIE_PARTITION`.
    pub has_aie_partition_marker: bool,
    /// Fixture lists `BITSTREAM`.
    pub has_bitstream_marker: bool,
    /// Always **false**: names only — never product known-good.
    pub product_known_good: bool,
    /// Always **false**: fixture does not invent ctrltext/PDI bytes.
    pub invents_ctrltext_bytes: bool,
    /// When fed into extract→bag→scorecard with absent bag, expected blocker.
    pub feeds_pipeline_markers_blocked_on_external_bytes: bool,
    /// Always **true**: residual remains open offline.
    pub residual_not_closed: bool,
    /// Always **false**: fixture alone never closes residual.
    pub fixture_alone_closes_fw_residual: bool,
    /// Always **false**: markers alone never close residual.
    pub markers_alone_closes_fw_residual: bool,
    /// Always **false**: pipeline alone never closes residual.
    pub pipeline_alone_closes_fw_residual: bool,
    /// Stable research detail token.
    pub detail: String,
}

impl Default for PublicStackSectionMarkerFixtureResearch {
    fn default() -> Self {
        Self {
            section_marker_count: 0,
            has_aie_partition_marker: false,
            has_bitstream_marker: false,
            product_known_good: false,
            invents_ctrltext_bytes: false,
            feeds_pipeline_markers_blocked_on_external_bytes: true,
            residual_not_closed: true,
            fixture_alone_closes_fw_residual: false,
            markers_alone_closes_fw_residual: false,
            pipeline_alone_closes_fw_residual: false,
            detail: "fixture_default_blocked_on_external_bytes".into(),
        }
    }
}

impl PublicStackSectionMarkerFixtureResearch {
    /// Soft research note (honesty + alone_closes hard-coded).
    ///
    /// Honesty tokens are **hard-coded** (not from mutable struct fields).
    #[must_use]
    pub fn format_soft_note(&self) -> String {
        let mut parts = vec![
            PUBLIC_STACK_SECTION_MARKER_FIXTURE_NOTE.into(),
            "not_c_ready=1".into(),
            "not_ran_on_npu=1".into(),
            "invent_pass=0".into(),
            "product_known_good=0".into(),
            "residual_not_closed=1".into(),
            "fixture_alone_closes_fw_residual=0".into(),
            "markers_alone_closes_fw_residual=0".into(),
            "pipeline_alone_closes_fw_residual=0".into(),
            "ctrltext_match_alone_closes_fw_residual=0".into(),
            "do_not_invent_pdi_layout=1".into(),
            "public_pdi_image_decode=0".into(),
            "invents_ctrltext_bytes=0".into(),
            format!("section_marker_count={}", self.section_marker_count),
            format!(
                "has_aie_partition_marker={}",
                u8::from(self.has_aie_partition_marker)
            ),
            format!(
                "has_bitstream_marker={}",
                u8::from(self.has_bitstream_marker)
            ),
            format!(
                "feeds_pipeline_markers_blocked_on_external_bytes={}",
                u8::from(self.feeds_pipeline_markers_blocked_on_external_bytes)
            ),
        ];
        if !self.detail.is_empty() {
            parts.push(format!(
                "detail={}",
                sanitize_soft_note_field_value(&self.detail)
            ));
        }
        parts.join(" ")
    }
}

/// Locked pure fixture text (public section names only).
#[must_use]
pub fn public_stack_xclbin_section_marker_fixture_text() -> &'static str {
    PUBLIC_STACK_XCLBIN_SECTION_MARKER_FIXTURE_TEXT
}

/// Research the locked public-stack section marker fixture (stamp `93ccd077`).
///
/// Always `product_known_good=0` / `invents_ctrltext_bytes=0` /
/// `fixture_alone_closes_fw_residual=0`. Never invents Pass.
#[must_use]
pub fn research_public_stack_section_marker_fixture() -> PublicStackSectionMarkerFixtureResearch {
    let text = public_stack_xclbin_section_marker_fixture_text();
    let markers = scan_xclbin_section_markers_from_text(text);
    #[allow(clippy::cast_possible_truncation)]
    let count = markers.len().min(255) as u8;
    let has_aie = markers.contains(&"AIE_PARTITION");
    let has_bs = markers.contains(&"BITSTREAM");
    PublicStackSectionMarkerFixtureResearch {
        section_marker_count: count,
        has_aie_partition_marker: has_aie,
        has_bitstream_marker: has_bs,
        product_known_good: false,
        invents_ctrltext_bytes: false,
        feeds_pipeline_markers_blocked_on_external_bytes: true,
        residual_not_closed: true,
        fixture_alone_closes_fw_residual: false,
        markers_alone_closes_fw_residual: false,
        pipeline_alone_closes_fw_residual: false,
        detail: "fixture_public_section_names_only_blocked_on_external_bytes".into(),
    }
}

/// Extract dry-run from the locked pure fixture (names only; no magic bytes).
///
/// No working-stack claim → not research-eligible product path; markers present
/// so pipeline detail becomes
/// `pipeline_extract_markers_present_blocked_on_external_bytes` when bag absent.
/// Never invents Pass / product known-good.
#[must_use]
pub fn research_public_stack_section_marker_fixture_extract() -> XclbinExtractResearchDryRun {
    research_xclbin_extract_dry_run(
        &[],
        Some(public_stack_xclbin_section_marker_fixture_text()),
        None,
    )
}

/// Unit surface: fixture markers + absent bag → pipeline still blocked.
///
/// Detail locked to
/// `pipeline_extract_markers_present_blocked_on_external_bytes`. Honesty +
/// residual_not_closed + all alone_closes=0. Never invents Pass.
#[must_use]
pub fn research_public_stack_marker_pipeline_c2_locked_blocked()
-> AppLoadExtractBagScorecardPipeline {
    let extract = research_public_stack_section_marker_fixture_extract();
    let bag = check_external_known_good_bag_drop(false, None, None, None);
    let feed = research_lab_export_fingerprint_feed(None, None, None, None);
    research_app_load_extract_bag_scorecard_pipeline(&extract, &bag, &feed, None, None, None)
}

/// True when the final path component has an allowed **section markers** extension.
///
/// Allowed: `.txt`, `.sections`, `.info` (case-insensitive). Not `.bin` /
/// `.ctrltext` (those are ctrltext bag bytes) and not binary xclbin bodies.
#[must_use]
pub fn xclbin_section_markers_extension_ok(path: &std::path::Path) -> bool {
    path.extension().is_some_and(|ext| {
        ext.eq_ignore_ascii_case("txt")
            || ext.eq_ignore_ascii_case("sections")
            || ext.eq_ignore_ascii_case("info")
    })
}

/// Whether a research section-markers **path** is allowed (roots + ext + refuse).
///
/// **Confinement** (mirrors external known-good roots):
/// - Under [`last_apply_temp_root`] **or** [`external_known_good_lab_root`]
/// - Extension `.txt` / `.sections` / `.info` only
/// - No `..` components
/// - Refuse local post-patch oracle basenames
/// - Resolve intermediate dir symlinks via `canonicalize(parent)` before root check
///
/// Does **not** open the file. Markers path **≠** product known-good.
#[must_use]
pub fn xclbin_section_markers_path_is_allowed(path: &std::path::Path) -> bool {
    if path.as_os_str().is_empty() {
        return false;
    }
    if path
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return false;
    }
    if !xclbin_section_markers_extension_ok(path) {
        return false;
    }
    if is_refused_local_post_patch_as_known_good(path) {
        return false;
    }
    let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) else {
        return path_under_external_known_good_roots(path);
    };
    if let Ok(canon_parent) = std::fs::canonicalize(parent) {
        let candidate = canon_parent.join(path.file_name().unwrap_or_default());
        if is_refused_local_post_patch_as_known_good(&candidate) {
            return false;
        }
        path_under_external_known_good_roots(&candidate)
    } else if path.is_absolute() {
        false
    } else {
        path_under_external_known_good_roots(path)
    }
}

/// Soft-load path for markers: path allow + refuse final-component symlink.
///
/// Does **not** require bag PROVENANCE (markers are research names only, not
/// product known-good ctrltext). Never invents Pass.
#[must_use]
pub fn xclbin_section_markers_load_path_is_allowed(path: &std::path::Path) -> bool {
    if !xclbin_section_markers_path_is_allowed(path) {
        return false;
    }
    if let Ok(meta) = std::fs::symlink_metadata(path) {
        if meta.file_type().is_symlink() {
            return false;
        }
    }
    if path.exists() {
        let Ok(canon) = std::fs::canonicalize(path) else {
            return false;
        };
        if !path_under_external_known_good_roots(&canon) {
            return false;
        }
        if is_refused_local_post_patch_as_known_good(&canon) {
            return false;
        }
    }
    true
}

/// Pure validation of research section-marker **text** (names only).
///
/// - Empty / whitespace → not ready
/// - Invent / honesty-inverse tokens refuse (`invent_pass=1`, `product_known_good=1`, …)
/// - At least one public marker from [`XCLBIN_RESEARCH_SECTION_MARKERS`] required
/// - Never product known-good / Pass
#[must_use]
pub fn validate_research_xclbin_section_markers_text(text: &str) -> bool {
    let t = text.trim();
    if t.is_empty() {
        return false;
    }
    let lower = t.to_ascii_lowercase();
    // Refuse invent / honesty-inverse claims embedded in marker pastes.
    if lower.contains("invent_pass=1")
        || lower.contains("invent_pass=true")
        || lower.contains("product_known_good=1")
        || lower.contains("product_known_good=true")
        || lower.contains("ran_on_npu=1")
        || lower.contains("ran_on_npu=true")
        || lower.contains("c_ready=1")
        || lower.contains("not_c_ready=0")
    {
        return false;
    }
    !scan_xclbin_section_markers_from_text(t).is_empty()
}

/// Pure **stage check** for research-only xclbin section marker files (stamp `93ccd077`).
///
/// Confined operator path (research only): proposed basename + markers text under
/// refuse rules. Never product known-good. Stage allowed ≠ Pass ≠ residual closed.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // research flag bag; honesty-first
pub struct ResearchXclbinSectionMarkersStageCheck {
    /// Basename present and not a refused post_patch oracle.
    pub basename_allowed: bool,
    /// Extension is `.txt` / `.sections` / `.info`.
    pub extension_ok: bool,
    /// Text validates as research public markers (no invent tokens).
    pub markers_text_ok: bool,
    /// Count of public markers when text ok (else 0).
    pub section_marker_count: u8,
    /// Local post_patch basename refuse.
    pub refused_local_post_patch: bool,
    /// VE2 / NOT_AIE2 platform claim refuse.
    pub refused_ve2_not_aie2: bool,
    /// Tag when VE2 / NOT_AIE2 (`platform_not_aie2=1`).
    pub platform_not_aie2: bool,
    /// Research stage path ok (all gates + not refuse). Still ≠ product known-good.
    pub stage_allowed: bool,
    /// Always **true**: residual remains open.
    pub residual_not_closed: bool,
    /// Always **false**: stage alone never closes residual.
    pub stage_alone_closes_fw_residual: bool,
    /// Stable research detail token.
    pub detail: String,
}

impl Default for ResearchXclbinSectionMarkersStageCheck {
    fn default() -> Self {
        Self {
            basename_allowed: false,
            extension_ok: false,
            markers_text_ok: false,
            section_marker_count: 0,
            refused_local_post_patch: false,
            refused_ve2_not_aie2: false,
            platform_not_aie2: false,
            stage_allowed: false,
            residual_not_closed: true,
            stage_alone_closes_fw_residual: false,
            detail: "stage_default_not_allowed".into(),
        }
    }
}

impl ResearchXclbinSectionMarkersStageCheck {
    /// Soft research note (honesty hard-coded).
    #[must_use]
    pub fn format_soft_note(&self) -> String {
        let mut parts = vec![
            RESEARCH_XCLBIN_SECTION_MARKERS_PATH_NOTE.into(),
            "not_c_ready=1".into(),
            "not_ran_on_npu=1".into(),
            "invent_pass=0".into(),
            "product_known_good=0".into(),
            "residual_not_closed=1".into(),
            "stage_alone_closes_fw_residual=0".into(),
            "markers_alone_closes_fw_residual=0".into(),
            "pipeline_alone_closes_fw_residual=0".into(),
            "do_not_invent_pdi_layout=1".into(),
            "public_pdi_image_decode=0".into(),
            format!("basename_allowed={}", u8::from(self.basename_allowed)),
            format!("extension_ok={}", u8::from(self.extension_ok)),
            format!("markers_text_ok={}", u8::from(self.markers_text_ok)),
            format!("section_marker_count={}", self.section_marker_count),
            format!(
                "refused_local_post_patch={}",
                u8::from(self.refused_local_post_patch)
            ),
            format!(
                "refused_ve2_not_aie2={}",
                u8::from(self.refused_ve2_not_aie2)
            ),
            format!("platform_not_aie2={}", u8::from(self.platform_not_aie2)),
            format!("stage_allowed={}", u8::from(self.stage_allowed)),
        ];
        if !self.detail.is_empty() {
            parts.push(format!(
                "detail={}",
                sanitize_soft_note_field_value(&self.detail)
            ));
        }
        parts.join(" ")
    }
}

/// Pure stage check for research-only section marker files (stamp `93ccd077`).
///
/// - `proposed_basename`: e.g. `public_stack_sections.txt` (not post_patch oracles)
/// - `markers_text`: public section names paste (or fixture text)
/// - `platform_claim`: optional VE2 refuse
///
/// Stage allowed ≠ product known-good ≠ Pass. Always residual_not_closed.
#[must_use]
pub fn check_research_xclbin_section_markers_stage(
    proposed_basename: Option<&str>,
    markers_text: Option<&str>,
    platform_claim: Option<&str>,
) -> ResearchXclbinSectionMarkersStageCheck {
    let claim = platform_claim.unwrap_or("").trim();
    let refused_ve2 = !claim.is_empty() && platform_claim_is_not_aie2_known_good(claim);
    let base = proposed_basename.map(str::trim).filter(|s| !s.is_empty());
    let path_for_ext = base.map(std::path::Path::new);
    let refused_post = path_for_ext.is_some_and(is_refused_local_post_patch_as_known_good);
    let extension_ok = path_for_ext.is_some_and(xclbin_section_markers_extension_ok);
    let basename_allowed = base.is_some() && !refused_post && extension_ok;
    let text = markers_text.map(str::trim).filter(|s| !s.is_empty());
    let markers_text_ok = text.is_some_and(validate_research_xclbin_section_markers_text);
    let markers = text
        .map(scan_xclbin_section_markers_from_text)
        .unwrap_or_default();
    #[allow(clippy::cast_possible_truncation)]
    let count = if markers_text_ok {
        markers.len().min(255) as u8
    } else {
        0
    };
    let stage_allowed = basename_allowed && markers_text_ok && !refused_ve2;
    let detail = if refused_post {
        "stage_refused_local_post_patch"
    } else if refused_ve2 {
        "stage_refused_ve2_not_aie2"
    } else if base.is_none() {
        "stage_basename_absent"
    } else if !extension_ok {
        "stage_extension_not_markers_text"
    } else if text.is_none() {
        "stage_markers_text_absent"
    } else if !markers_text_ok {
        "stage_markers_text_invalid_or_invent"
    } else if stage_allowed {
        "stage_research_only_allowed_not_product_known_good"
    } else {
        "stage_not_allowed"
    };
    ResearchXclbinSectionMarkersStageCheck {
        basename_allowed,
        extension_ok,
        markers_text_ok,
        section_marker_count: count,
        refused_local_post_patch: refused_post,
        refused_ve2_not_aie2: refused_ve2,
        platform_not_aie2: refused_ve2,
        stage_allowed,
        residual_not_closed: true,
        stage_alone_closes_fw_residual: false,
        detail: detail.into(),
    }
}

/// Load research section-marker text from a confined path (O_NOFOLLOW).
///
/// Cap [`MAX_XCLBIN_SECTION_MARKERS_TEXT_BYTES`]. Validates text after load.
/// Never product known-good / Pass.
///
/// # Errors
///
/// Path refused, I/O failure, oversize, or text fails
/// [`validate_research_xclbin_section_markers_text`].
pub fn load_xclbin_section_markers_text_for_research(
    path: &std::path::Path,
) -> Result<String, DirectError> {
    use std::io::Read;

    if !xclbin_section_markers_load_path_is_allowed(path) {
        return Err(DirectError::InvalidArgument(format!(
            "xclbin section markers path refused (confinement/ext/post_patch/symlink): {}",
            path.display()
        )));
    }
    let mut f = open_external_ctrltext_nofollow(path).map_err(|e| {
        DirectError::InvalidArgument(format!(
            "xclbin section markers open failed ({}): {e}",
            path.display()
        ))
    })?;
    let mut bytes = Vec::new();
    f.by_ref()
        .take(MAX_XCLBIN_SECTION_MARKERS_TEXT_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| {
            DirectError::InvalidArgument(format!(
                "xclbin section markers read failed ({}): {e}",
                path.display()
            ))
        })?;
    if bytes.len() > MAX_XCLBIN_SECTION_MARKERS_TEXT_BYTES {
        return Err(DirectError::InvalidArgument(format!(
            "xclbin section markers text oversized ({} > {})",
            bytes.len(),
            MAX_XCLBIN_SECTION_MARKERS_TEXT_BYTES
        )));
    }
    let text = String::from_utf8(bytes).map_err(|_| {
        DirectError::InvalidArgument(format!(
            "xclbin section markers not UTF-8: {}",
            path.display()
        ))
    })?;
    if !validate_research_xclbin_section_markers_text(&text) {
        return Err(DirectError::InvalidArgument(format!(
            "xclbin section markers text invalid or invent tokens: {}",
            path.display()
        )));
    }
    Ok(text)
}

/// Resolve optional [`ENV_LAB_XCLBIN_SECTION_MARKERS_PATH`] → confined text or `None`.
///
/// - Unset / empty → `Ok(None)`
/// - Set + refuse / I/O / invalid → `Err`
/// - Set + load OK → `Ok(Some(text))` for extract research only
///
/// # Errors
///
/// Path refused or load failure when env is non-empty.
pub fn load_xclbin_section_markers_text_from_env() -> Result<Option<String>, DirectError> {
    let Ok(raw) = std::env::var(ENV_LAB_XCLBIN_SECTION_MARKERS_PATH) else {
        return Ok(None);
    };
    let t = raw.trim();
    if t.is_empty() {
        return Ok(None);
    }
    let path = std::path::PathBuf::from(t);
    Ok(Some(load_xclbin_section_markers_text_for_research(&path)?))
}

/// Open external known-good bytes without following a final-component symlink.
///
/// Unix: `O_NOFOLLOW`. Non-Unix: residual TOCTOU after [`symlink_metadata`] refuse.
fn read_external_ctrltext_nofollow_capped(path: &std::path::Path) -> Result<Vec<u8>, DirectError> {
    use std::io::Read;
    let mut f = open_external_ctrltext_nofollow(path).map_err(|e| {
        DirectError::InvalidArgument(format!(
            "external known-good open failed ({}): {e}",
            path.display()
        ))
    })?;
    let mut bytes = Vec::new();
    f.by_ref()
        .take(MAX_MLADF_LAB_FILE_BYTES)
        .read_to_end(&mut bytes)
        .map_err(|e| {
            DirectError::InvalidArgument(format!(
                "external known-good read failed ({}): {e}",
                path.display()
            ))
        })?;
    Ok(bytes)
}

#[cfg(unix)]
fn open_external_ctrltext_nofollow(path: &std::path::Path) -> std::io::Result<std::fs::File> {
    use std::os::unix::fs::OpenOptionsExt;
    std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
}

#[cfg(not(unix))]
fn open_external_ctrltext_nofollow(path: &std::path::Path) -> std::io::Result<std::fs::File> {
    // Residual TOCTOU: path policy refuses symlink via symlink_metadata only.
    std::fs::File::open(path)
}

/// Resolve optional [`ENV_LAB_EXTERNAL_CTRLTEXT`] → confined load or `None`.
///
/// - Unset / empty → `Ok(None)` (honest absent path; compare uses no external).
/// - Set + confinement fail / I/O → `Err` (caller soft-WARNs; never invent bytes).
/// - Set + load OK → `Ok(Some(bytes))` for research compare only.
///
/// # Errors
///
/// Path refused or read failure when env is non-empty.
pub fn load_external_ctrltext_from_env() -> Result<Option<Vec<u8>>, DirectError> {
    let Ok(raw) = std::env::var(ENV_LAB_EXTERNAL_CTRLTEXT) else {
        return Ok(None);
    };
    let t = raw.trim();
    if t.is_empty() {
        return Ok(None);
    }
    let path = std::path::PathBuf::from(t);
    Ok(Some(load_external_ctrltext_for_compare(&path)?))
}

// ---------------------------------------------------------------------------
// Track L — public FW APP_LOAD / PDI / partition load surface (no private PDI)
// ---------------------------------------------------------------------------

/// Journal / mailbox code for Failed-cmd APP_LOAD PDI fail (`0x3000003`).
///
/// [`aie2_fw_status_name`] returns **`APP_LOAD_PDI_FAIL`** (public status label).
/// Does **not** decode a PDI image layout.
pub const AIE2_FW_APP_LOAD_PDI_FAIL: u32 = 0x3_000_003;

/// Journal / mailbox code: chain `Status 0x4000003`.
///
/// [`aie2_fw_status_name`] returns **`AIE2_STATUS_INVALID_PARAM`**.
pub const AIE2_FW_INVALID_PARAM: u32 = 0x4_000_003;

/// Public in-tree PDI image marker token — always absent (no decoder).
///
/// SSoT for Track K + Track L soft notes (`pdi_marker_public=`).
pub const PUBLIC_PDI_MARKER_TOKEN: &str = "marker_absent_public";

/// Public partition/PDI load surface depth available offline without private blobs.
///
/// `status_name_only` = FW status **names** + hex codes from journal/docs only.
pub const PUBLIC_PARTITION_SURFACE_TOKEN: &str = "status_name_only";

/// Cap for journal paste fed to [`research_app_load_pdi_partition_from_journal`].
///
/// Matches classify env `XDNA2_LAB_JOURNAL_TEXT` ceiling (256 KiB). Oversized
/// text is treated as **absent** (no invent codes) — same honesty as empty.
pub const MAX_APP_LOAD_JOURNAL_TEXT_BYTES: usize = 256 * 1024;

/// Offline research: what public headers/docs/journal say about FW APP_LOAD PDI /
/// partition load — **beyond** staged classic 1688/40 content profile (Track K).
///
/// Honesty:
/// - No private PDI binary layout decode.
/// - No invent vendor post-patch / prop_count=15 / Pass / C-ready.
/// - Userspace content green ≠ FW partition load OK ≠ C-ready.
///
/// **Construction:** prefer [`research_app_load_pdi_partition`] /
/// [`research_app_load_pdi_partition_from_journal`] /
/// [`research_app_load_pdi_partition_c2_locked`]. Those always set surface tokens
/// to [`PUBLIC_PARTITION_SURFACE_TOKEN`] / [`PUBLIC_PDI_MARKER_TOKEN`].
/// [`Default`] is honesty-safe (same surface tokens; all residual flags false).
/// Field `pub` is for soft-note / unit access — do not overwrite surface tokens
/// with empty strings in product paths.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // research flag bag; not a state machine
pub struct AppLoadPdiPartitionResearch {
    /// Failed-cmd status when journal/detail provided a literal hex.
    pub failed_cmd_status: Option<u32>,
    /// Chain/completion status when journal provided a literal hex.
    pub chain_status: Option<u32>,
    /// Locked live C2 dual pair (`0x3000003` + `0x4000003`).
    pub c2_app_load_pair: bool,
    /// Residual class is [`SiliconResidualClass::AppLoadPdiFailAfterAiebu`].
    pub class_app_load_after_aiebu: bool,
    /// Caller-supplied userspace content green (ELF+rela+classic walk).
    /// Research only — never invents Pass.
    pub userspace_content_ok: bool,
    /// True when userspace content is green **and** FW APP_LOAD residual still open
    /// (class and/or C2 journal pair). Dual residual honesty lock.
    ///
    /// Research/PRINT only — checklist Track L tokens are **class-gated** and do
    /// **not** open on journal pair alone (see RESIDUAL / CMD_PAYLOAD).
    pub fw_residual_open_after_userspace: bool,
    /// Always [`PUBLIC_PARTITION_SURFACE_TOKEN`] from constructors / Default.
    pub partition_surface_public: &'static str,
    /// Always [`PUBLIC_PDI_MARKER_TOKEN`] from constructors / Default.
    pub pdi_marker_public: &'static str,
}

impl Default for AppLoadPdiPartitionResearch {
    fn default() -> Self {
        Self {
            failed_cmd_status: None,
            chain_status: None,
            c2_app_load_pair: false,
            class_app_load_after_aiebu: false,
            userspace_content_ok: false,
            fw_residual_open_after_userspace: false,
            partition_surface_public: PUBLIC_PARTITION_SURFACE_TOKEN,
            pdi_marker_public: PUBLIC_PDI_MARKER_TOKEN,
        }
    }
}

impl AppLoadPdiPartitionResearch {
    /// Soft research note (always honesty trio + public surface tokens).
    ///
    /// Emits constructor surface values; prefer research_* builders so tokens
    /// stay locked to [`PUBLIC_PARTITION_SURFACE_TOKEN`] /
    /// [`PUBLIC_PDI_MARKER_TOKEN`].
    #[must_use]
    pub fn format_soft_note(&self) -> String {
        let mut parts = vec![
            "app_load_pdi_partition_research".into(),
            "not_c_ready=1".into(),
            "not_ran_on_npu=1".into(),
            "invent_pass=0".into(),
            "do_not_invent_pdi_layout=1".into(),
            "public_pdi_image_decode=0".into(),
            format!("partition_surface_public={}", self.partition_surface_public),
            format!("pdi_marker_public={}", self.pdi_marker_public),
            format!(
                "class_app_load_after_aiebu={}",
                u8::from(self.class_app_load_after_aiebu)
            ),
            format!(
                "userspace_content_ok={}",
                u8::from(self.userspace_content_ok)
            ),
            format!(
                "fw_residual_open_after_userspace={}",
                u8::from(self.fw_residual_open_after_userspace)
            ),
            format!("c2_app_load_pair={}", u8::from(self.c2_app_load_pair)),
        ];
        match self.failed_cmd_status {
            Some(c) => {
                parts.push(format!("failed_cmd=0x{c:x}"));
                parts.push(format!("failed_cmd_name={}", aie2_fw_status_name(c)));
            }
            None => parts.push("failed_cmd=absent".into()),
        }
        match self.chain_status {
            Some(c) => {
                parts.push(format!("chain_status=0x{c:x}"));
                parts.push(format!("chain_name={}", aie2_fw_status_name(c)));
            }
            None => parts.push("chain_status=absent".into()),
        }
        // Public name surface: APP_LOAD_PDI_FAIL implies partition/PDI load step
        // rejected the app — status **name** only, not an image field map.
        if self.failed_cmd_status == Some(AIE2_FW_APP_LOAD_PDI_FAIL)
            || self.class_app_load_after_aiebu
        {
            parts.push("public_fw_name_implies_pdi_load=1".into());
        } else {
            parts.push("public_fw_name_implies_pdi_load=0".into());
        }
        parts.join(" ")
    }
}

/// Build Track L research from residual class + optional journal pair + userspace flag.
///
/// - Never invents journal codes when `journal` is `None` or fields absent.
/// - `userspace_content_ok` is caller-supplied (e.g. Track K research bit) — not
///   inferred from residual class alone.
/// - `fw_residual_open_after_userspace` = `userspace_content_ok && (class || pair)`.
/// - Always sets surface tokens to [`PUBLIC_PARTITION_SURFACE_TOKEN`] /
///   [`PUBLIC_PDI_MARKER_TOKEN`].
/// - Always soft; never Pass / C-ready.
#[must_use]
pub fn research_app_load_pdi_partition(
    class: SiliconResidualClass,
    journal: Option<&JournalFwStatusPair>,
    userspace_content_ok: bool,
) -> AppLoadPdiPartitionResearch {
    let class_app = class.is_locked_app_load_after_aiebu();
    let (failed, chain, pair) = journal.map_or((None, None, false), |j| {
        (j.failed_cmd_status, j.chain_status, j.is_c2_app_load_pair())
    });
    let fw_open = userspace_content_ok && (class_app || pair);
    AppLoadPdiPartitionResearch {
        failed_cmd_status: failed,
        chain_status: chain,
        c2_app_load_pair: pair,
        class_app_load_after_aiebu: class_app,
        userspace_content_ok,
        fw_residual_open_after_userspace: fw_open,
        partition_surface_public: PUBLIC_PARTITION_SURFACE_TOKEN,
        pdi_marker_public: PUBLIC_PDI_MARKER_TOKEN,
    }
}

/// Parse optional journal/multi-line text then run [`research_app_load_pdi_partition`].
///
/// - Empty/whitespace journal → treated as absent (no invent).
/// - Text longer than [`MAX_APP_LOAD_JOURNAL_TEXT_BYTES`] → treated as absent
///   (`journal_text_oversized` path: no codes invented). Callers that need a
///   loud refuse (classify env) still cap at the env boundary.
#[must_use]
pub fn research_app_load_pdi_partition_from_journal(
    class: SiliconResidualClass,
    journal_text: Option<&str>,
    userspace_content_ok: bool,
) -> AppLoadPdiPartitionResearch {
    let pair = journal_text
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .filter(|s| s.len() <= MAX_APP_LOAD_JOURNAL_TEXT_BYTES)
        .map(parse_journal_fw_status_pair);
    research_app_load_pdi_partition(class, pair.as_ref(), userspace_content_ok)
}

/// Locked C2 journal paste used by unit tests (Failed-cmd + chain Status).
///
/// Research fixture text only — not a live journal read.
#[must_use]
pub fn c2_locked_app_load_journal_text() -> &'static str {
    "amdxdna: Failed cmd idx 0, status 0x3000003\n\
     amdxdna: Status 0x4000003\n"
}

/// Convenience: Track L research for locked APP_LOAD class + locked C2 journal pair.
///
/// `userspace_content_ok` defaults the dual-residual path (content green, FW open).
#[must_use]
pub fn research_app_load_pdi_partition_c2_locked(
    userspace_content_ok: bool,
) -> AppLoadPdiPartitionResearch {
    research_app_load_pdi_partition_from_journal(
        SiliconResidualClass::AppLoadPdiFailAfterAiebu,
        Some(c2_locked_app_load_journal_text()),
        userspace_content_ok,
    )
}

// ---------------------------------------------------------------------------
// Journal `0x3000003` partition-id field map (public surface only)
// ---------------------------------------------------------------------------

/// Public journal field name for the Failed-cmd status code.
pub const APP_LOAD_JOURNAL_FIELD_FAILED_CMD: &str = "failed_cmd_status";

/// Public journal field name for the chain/completion status code.
pub const APP_LOAD_JOURNAL_FIELD_CHAIN_STATUS: &str = "chain_status";

/// Public token: partition **id** is absent on the offline journal surface.
///
/// Journal paste yields status **codes/names** only. There is no public
/// `partition_id` / column-id field to map without private decode. Soft notes
/// always emit this token (never invent an id).
pub const APP_LOAD_PARTITION_ID_PUBLIC_TOKEN: &str = "absent_public";

/// Soft research: public field map for FW `APP_LOAD_PDI_FAIL` (`0x3000003`).
///
/// Unit-locks which journal fields we actually parse vs what remains absent:
/// - `failed_cmd_status` → primary load-reject code (name via [`aie2_fw_status_name`])
/// - `chain_status` → secondary chain code (often `0x4000003` INVALID_PARAM)
/// - `partition_id` → always [`APP_LOAD_PARTITION_ID_PUBLIC_TOKEN`] (`absent_public`)
///
/// Stamp `7a5ab864`. **Not** root-cause closed. **Not** Pass / C-ready / `ran_on_npu`.
/// Field map alone **never** closes FW residual.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // research flag bag; honesty-first
pub struct AppLoadPartitionIdFieldMap {
    /// Failed-cmd status when journal provided a literal hex.
    pub failed_cmd_status: Option<u32>,
    /// Chain/completion status when journal provided a literal hex.
    pub chain_status: Option<u32>,
    /// True when failed_cmd is [`AIE2_FW_APP_LOAD_PDI_FAIL`].
    pub failed_cmd_is_app_load: bool,
    /// True when chain is [`AIE2_FW_INVALID_PARAM`].
    pub chain_is_invalid_param: bool,
    /// Locked live C2 dual pair present.
    pub c2_app_load_pair: bool,
    /// Always [`APP_LOAD_PARTITION_ID_PUBLIC_TOKEN`] from constructors / Default.
    pub partition_id_public: &'static str,
    /// Always **true**: never invent a partition id from public paste.
    pub do_not_invent_partition_id: bool,
    /// Always **false**: field map alone does not close FW residual / invent Pass.
    pub field_map_alone_closes_fw_residual: bool,
    /// Always **false**: never invent root-cause closed.
    pub root_cause_closed: bool,
    /// Stable research detail token.
    pub detail: String,
}

impl Default for AppLoadPartitionIdFieldMap {
    fn default() -> Self {
        Self {
            failed_cmd_status: None,
            chain_status: None,
            failed_cmd_is_app_load: false,
            chain_is_invalid_param: false,
            c2_app_load_pair: false,
            partition_id_public: APP_LOAD_PARTITION_ID_PUBLIC_TOKEN,
            do_not_invent_partition_id: true,
            field_map_alone_closes_fw_residual: false,
            root_cause_closed: false,
            detail: "default_no_journal".into(),
        }
    }
}

impl AppLoadPartitionIdFieldMap {
    /// Soft research note (always honesty trio; never invent Pass / root-cause closed).
    #[must_use]
    pub fn format_soft_note(&self) -> String {
        let mut parts = vec![
            "app_load_partition_id_field_map".into(),
            "not_c_ready=1".into(),
            "not_ran_on_npu=1".into(),
            "invent_pass=0".into(),
            "product_known_good=0".into(),
            // Force honesty locks even if a hand-built bag mutates fields.
            format!("partition_id_public={APP_LOAD_PARTITION_ID_PUBLIC_TOKEN}"),
            "do_not_invent_partition_id=1".into(),
            "field_map_alone_closes_fw_residual=0".into(),
            "root_cause_closed=0".into(),
            format!(
                "partition_surface_public={}",
                PUBLIC_PARTITION_SURFACE_TOKEN
            ),
            format!(
                "failed_cmd_is_app_load={}",
                u8::from(self.failed_cmd_is_app_load)
            ),
            format!(
                "chain_is_invalid_param={}",
                u8::from(self.chain_is_invalid_param)
            ),
            format!("c2_app_load_pair={}", u8::from(self.c2_app_load_pair)),
            format!(
                "journal_field_failed_cmd={}",
                APP_LOAD_JOURNAL_FIELD_FAILED_CMD
            ),
            format!(
                "journal_field_chain_status={}",
                APP_LOAD_JOURNAL_FIELD_CHAIN_STATUS
            ),
        ];
        match self.failed_cmd_status {
            Some(c) => {
                parts.push(format!("failed_cmd=0x{c:x}"));
                parts.push(format!("failed_cmd_name={}", aie2_fw_status_name(c)));
            }
            None => parts.push("failed_cmd=absent".into()),
        }
        match self.chain_status {
            Some(c) => {
                parts.push(format!("chain_status=0x{c:x}"));
                parts.push(format!("chain_name={}", aie2_fw_status_name(c)));
            }
            None => parts.push("chain_status=absent".into()),
        }
        if !self.detail.is_empty() {
            parts.push(format!(
                "detail={}",
                sanitize_soft_note_field_value(&self.detail)
            ));
        }
        parts.join(" ")
    }
}

/// Build partition-id field map from an optional journal pair (no invent).
///
/// - `None` / empty pair → codes absent; partition_id still `absent_public`.
/// - Always `root_cause_closed=false` / `field_map_alone_closes_fw_residual=false`.
/// - Soft only; never Pass / C-ready / second unlock.
#[must_use]
pub fn research_app_load_partition_id_field_map(
    journal: Option<&JournalFwStatusPair>,
) -> AppLoadPartitionIdFieldMap {
    let (failed, chain) = journal.map_or((None, None), |j| (j.failed_cmd_status, j.chain_status));
    let failed_app = failed == Some(AIE2_FW_APP_LOAD_PDI_FAIL);
    let chain_inv = chain == Some(AIE2_FW_INVALID_PARAM);
    let pair = failed_app && chain_inv;
    let detail = if pair {
        "c2_pair_mapped_partition_id_absent"
    } else if failed_app {
        "failed_cmd_app_load_partition_id_absent"
    } else if failed.is_some() || chain.is_some() {
        "partial_codes_partition_id_absent"
    } else {
        "default_no_journal"
    };
    AppLoadPartitionIdFieldMap {
        failed_cmd_status: failed,
        chain_status: chain,
        failed_cmd_is_app_load: failed_app,
        chain_is_invalid_param: chain_inv,
        c2_app_load_pair: pair,
        partition_id_public: APP_LOAD_PARTITION_ID_PUBLIC_TOKEN,
        do_not_invent_partition_id: true,
        field_map_alone_closes_fw_residual: false,
        root_cause_closed: false,
        detail: detail.into(),
    }
}

/// Parse optional journal text then run [`research_app_load_partition_id_field_map`].
///
/// Oversized / empty text → absent codes (same cap as Track L). Soft only.
#[must_use]
pub fn research_app_load_partition_id_field_map_from_journal(
    journal_text: Option<&str>,
) -> AppLoadPartitionIdFieldMap {
    let pair = journal_text
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .filter(|s| s.len() <= MAX_APP_LOAD_JOURNAL_TEXT_BYTES)
        .map(parse_journal_fw_status_pair);
    research_app_load_partition_id_field_map(pair.as_ref())
}

/// Convenience: locked C2 journal paste → field map with partition_id absent.
#[must_use]
pub fn research_app_load_partition_id_field_map_c2_locked() -> AppLoadPartitionIdFieldMap {
    research_app_load_partition_id_field_map_from_journal(Some(c2_locked_app_load_journal_text()))
}

// ---------------------------------------------------------------------------
// Track M — public PDI/partition evidence scan (maintain / closed lock)
// ---------------------------------------------------------------------------

/// Public offline evidence depth for PDI/partition load beyond status names.
///
/// Always [`PUBLIC_PARTITION_SURFACE_TOKEN`] (`status_name_only`) until in-tree
/// public headers/docs expose decodeable PDI image fields or load ioctls beyond
/// named FW status codes. **Do not** invent a stronger depth.
pub const PUBLIC_PDI_EVIDENCE_DEPTH_TOKEN: &str = PUBLIC_PARTITION_SURFACE_TOKEN;

/// Count of **new** public PDI image / partition-load fields found beyond Track L.
///
/// Locked `0` from the **human offline research conclusion** (Track M maintain):
/// in-tree `docs/` / `kernel/patches/` / `kernel/dkms/` / RESIDUAL notes had no
/// decodeable PDI layout beyond status names. This constant encodes that audit —
/// it is **not** produced by a runtime filesystem walk.
///
/// Runtime pure path ([`scan_public_pdi_partition_evidence`]) only re-checks
/// [`aie2_fw_status_name`] + emits this locked bag. Bump only with real public
/// evidence — never invent layout fields. Does **not** claim external XRT
/// exhaustiveness.
pub const PUBLIC_PDI_NEW_FIELDS_CLOSED: u8 = 0;

/// Soft-note key for derived closed-scan bit (`scan_closed_no_new_public_fields=`).
///
/// Value is **derived** from fields in [`PublicPdiEvidenceScan::format_soft_note`]
/// — never hard-coded `=1` when the bag is non-closed.
pub const PUBLIC_PDI_SCAN_CLOSED_TOKEN_KEY: &str = "scan_closed_no_new_public_fields";

/// Offline scan conclusion: public PDI/partition evidence beyond Track L.
///
/// Honesty:
/// - No private PDI binary layout decode.
/// - No invent vendor post-patch / prop_count=15 / Pass / C-ready / second unlock.
/// - Closed scan (`new_public_pdi_fields=0`) **≠** FW load OK ≠ C-ready.
///
/// **Construction:** prefer [`scan_public_pdi_partition_evidence`] /
/// [`scan_public_pdi_partition_evidence_closed`]. [`Default`] **delegates** to
/// the same scan path (name-map re-check via [`aie2_fw_status_name`]).
///
/// Runtime pure API does **not** walk `docs/` or kernel trees — those were
/// human-audited offline; this type locks the closed conclusion + status names.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // research flag bag; not a state machine
pub struct PublicPdiEvidenceScan {
    /// Always [`PUBLIC_PDI_EVIDENCE_DEPTH_TOKEN`] / [`PUBLIC_PARTITION_SURFACE_TOKEN`].
    pub public_pdi_evidence_depth: &'static str,
    /// New public fields beyond Track L status-name surface (`0` when closed).
    pub new_public_pdi_fields: u8,
    /// Always true from constructors / Default (`do_not_invent_pdi_layout=1`).
    pub do_not_invent_pdi_layout: bool,
    /// Always false — public image decode remains closed (Track L + M).
    pub public_pdi_image_decode: bool,
    /// Echo of Track L surface token (same value as depth when closed).
    pub partition_surface_public: &'static str,
    /// Echo of Track L marker token ([`PUBLIC_PDI_MARKER_TOKEN`]).
    pub pdi_marker_public: &'static str,
    /// Named APP_LOAD code still maps via [`aie2_fw_status_name`] only.
    pub app_load_code_name_ok: bool,
    /// Named INVALID_PARAM code still maps via [`aie2_fw_status_name`] only.
    pub invalid_param_code_name_ok: bool,
}

impl Default for PublicPdiEvidenceScan {
    fn default() -> Self {
        // Same production path as scan — name_ok via aie2_fw_status_name.
        scan_public_pdi_partition_evidence()
    }
}

impl PublicPdiEvidenceScan {
    /// True when this bag is the closed public-evidence conclusion (Track M).
    ///
    /// Used to derive `scan_closed_no_new_public_fields=` so a hostile hand-built
    /// bag cannot emit closed=1 alongside non-zero new fields / decode.
    ///
    /// **Not** gated on `app_load_code_name_ok` / `invalid_param_code_name_ok` —
    /// those are **diagnostic-only** status-name labels (always printed in the
    /// soft note). Closed = no new public fields / no invent layout / no image
    /// decode.
    ///
    /// **Depth intentionally omitted** (narrower than Track N/O): M closed does
    /// **not** require `public_pdi_evidence_depth == status_name_only`. Track O
    /// maintain reconfirm and Track N external closed **do** gate on depth —
    /// O's depth check is authoritative for A–N maintain. Do not tighten M alone
    /// without an explicit parity decision.
    #[must_use]
    pub fn is_scan_closed_no_new_public_fields(&self) -> bool {
        self.new_public_pdi_fields == 0
            && !self.public_pdi_image_decode
            && self.do_not_invent_pdi_layout
    }

    /// Soft research note (always honesty trio; closed bit **derived** from fields).
    ///
    /// String field values are passed through [`sanitize_soft_note_field_value`]
    /// so hostile hand-built bags cannot inject extra whitespace tokens.
    #[must_use]
    pub fn format_soft_note(&self) -> String {
        let closed = u8::from(self.is_scan_closed_no_new_public_fields());
        let mut parts = vec![
            "public_pdi_evidence_scan".into(),
            "not_c_ready=1".into(),
            "not_ran_on_npu=1".into(),
            "invent_pass=0".into(),
            format!(
                "public_pdi_evidence_depth={}",
                sanitize_soft_note_field_value(self.public_pdi_evidence_depth)
            ),
            format!("new_public_pdi_fields={}", self.new_public_pdi_fields),
            format!(
                "do_not_invent_pdi_layout={}",
                u8::from(self.do_not_invent_pdi_layout)
            ),
            format!(
                "public_pdi_image_decode={}",
                u8::from(self.public_pdi_image_decode)
            ),
            format!(
                "partition_surface_public={}",
                sanitize_soft_note_field_value(self.partition_surface_public)
            ),
            format!(
                "pdi_marker_public={}",
                sanitize_soft_note_field_value(self.pdi_marker_public)
            ),
            format!(
                "app_load_code_name_ok={}",
                u8::from(self.app_load_code_name_ok)
            ),
            format!(
                "invalid_param_code_name_ok={}",
                u8::from(self.invalid_param_code_name_ok)
            ),
            // Derived from fields — not a hard-coded =1 (hostile bag → =0).
            format!("{PUBLIC_PDI_SCAN_CLOSED_TOKEN_KEY}={closed}"),
        ];
        // Anchor named codes without inventing a layout.
        parts.push(format!("locked_app_load=0x{AIE2_FW_APP_LOAD_PDI_FAIL:x}"));
        parts.push(format!("locked_invalid_param=0x{AIE2_FW_INVALID_PARAM:x}"));
        parts.join(" ")
    }
}

/// Pure public-evidence scan for PDI/partition load (Track M).
///
/// Returns the **closed** conclusion locked by the human offline research
/// stamp ([`PUBLIC_PDI_NEW_FIELDS_CLOSED`] / status-name depth): no new public
/// PDI fields beyond Track L. Runtime only re-checks locked FW codes via
/// [`aie2_fw_status_name`] (label only).
///
/// Does **not** read the filesystem, walk `docs/`/`kernel/`, open L3, or decode
/// private blobs. Always soft; never Pass / C-ready / second unlock.
#[must_use]
pub fn scan_public_pdi_partition_evidence() -> PublicPdiEvidenceScan {
    let app_ok = aie2_fw_status_name(AIE2_FW_APP_LOAD_PDI_FAIL) == "APP_LOAD_PDI_FAIL";
    let inv_ok = aie2_fw_status_name(AIE2_FW_INVALID_PARAM) == "AIE2_STATUS_INVALID_PARAM";
    PublicPdiEvidenceScan {
        public_pdi_evidence_depth: PUBLIC_PDI_EVIDENCE_DEPTH_TOKEN,
        new_public_pdi_fields: PUBLIC_PDI_NEW_FIELDS_CLOSED,
        do_not_invent_pdi_layout: true,
        public_pdi_image_decode: false,
        partition_surface_public: PUBLIC_PARTITION_SURFACE_TOKEN,
        pdi_marker_public: PUBLIC_PDI_MARKER_TOKEN,
        app_load_code_name_ok: app_ok,
        invalid_param_code_name_ok: inv_ok,
    }
}

/// Alias: explicit closed-scan constructor (same as [`scan_public_pdi_partition_evidence`]).
#[must_use]
pub fn scan_public_pdi_partition_evidence_closed() -> PublicPdiEvidenceScan {
    scan_public_pdi_partition_evidence()
}

// ---------------------------------------------------------------------------
// Track N — external public PDI evidence (beyond in-tree Track M; maintain)
// ---------------------------------------------------------------------------

/// Soft-note token value: external public kernel/XRT evidence beyond status names
/// remains closed (no new layout fields found offline this turn).
///
/// Encodes the **human offline research conclusion** for Track N — not a live
/// network/XRT tree walk. Bump only with real external public evidence.
pub const EXTERNAL_PUBLIC_PDI_EVIDENCE_TOKEN: &str = "closed_no_new";

/// Count of new external public kernel/XRT fields beyond Track M status-name depth.
///
/// Locked `0` — no external public layout/ioctl surface found beyond named FW
/// status codes already locked in Tracks L/M.
///
/// Does **not** claim exhaustive internet / full XRT source audit; only that
/// this maintain turn found **no new** external public evidence to encode.
pub const EXTERNAL_KERNEL_XRT_BEYOND_STATUS_NAME: u8 = 0;

/// Soft-note key for derived external-closed bit
/// (`external_scan_closed_no_new=`).
///
/// Value is **derived** from fields in
/// [`ExternalPublicPdiEvidenceScan::format_soft_note`] — never hard-coded `=1`
/// when the bag is non-closed.
pub const EXTERNAL_PUBLIC_PDI_SCAN_CLOSED_TOKEN_KEY: &str = "external_scan_closed_no_new";

/// Offline scan conclusion: external public PDI/partition evidence beyond Track M.
///
/// Honesty:
/// - No private PDI binary layout decode / no invent prop_count=15.
/// - No invent vendor post-patch / Pass / C-ready / second unlock.
/// - Closed external scan **≠** FW load OK ≠ C-ready ≠ second C unlock.
/// - Depth remains [`PUBLIC_PDI_EVIDENCE_DEPTH_TOKEN`] (`status_name_only`).
///
/// **Construction:** prefer [`scan_external_public_pdi_evidence`] /
/// [`scan_external_public_pdi_evidence_closed`]. [`Default`] **delegates** to
/// the same scan path (status-name re-check via [`aie2_fw_status_name`]).
///
/// Runtime pure API does **not** fetch external XRT trees or walk the network —
/// those were not present as new public evidence this turn; this type locks the
/// closed conclusion + status names.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // research flag bag; not a state machine
pub struct ExternalPublicPdiEvidenceScan {
    /// Always [`EXTERNAL_PUBLIC_PDI_EVIDENCE_TOKEN`] (`closed_no_new`) when closed.
    pub external_public_pdi_evidence: &'static str,
    /// New external kernel/XRT fields beyond status-name (`0` when closed).
    pub external_kernel_xrt_beyond_status_name: u8,
    /// Echo of Track M depth — still `status_name_only` (no stronger depth).
    pub public_pdi_evidence_depth: &'static str,
    /// Always true from constructors / Default (`do_not_invent_pdi_layout=1`).
    pub do_not_invent_pdi_layout: bool,
    /// Always false — public image decode remains closed (Tracks L/M/N).
    pub public_pdi_image_decode: bool,
    /// Named APP_LOAD code still maps via [`aie2_fw_status_name`] only.
    /// Diagnostic soft-note field — **not** part of
    /// [`Self::is_external_scan_closed_no_new`].
    pub app_load_code_name_ok: bool,
    /// Named INVALID_PARAM code still maps via [`aie2_fw_status_name`] only.
    /// Diagnostic soft-note field — **not** part of
    /// [`Self::is_external_scan_closed_no_new`].
    pub invalid_param_code_name_ok: bool,
}

impl Default for ExternalPublicPdiEvidenceScan {
    fn default() -> Self {
        // Same production path as scan — name_ok via aie2_fw_status_name.
        scan_external_public_pdi_evidence()
    }
}

impl ExternalPublicPdiEvidenceScan {
    /// True when this bag is the closed external public-evidence conclusion (Track N).
    ///
    /// Used to derive `external_scan_closed_no_new=` so a hostile hand-built bag
    /// cannot emit closed=1 alongside non-zero external fields / decode / wrong
    /// depth / non-`closed_no_new` token.
    ///
    /// **Not** gated on `app_load_code_name_ok` / `invalid_param_code_name_ok` —
    /// those are **diagnostic-only** status-name labels (always printed in the
    /// soft note). Closed = external count 0 + `closed_no_new` token +
    /// status-name depth + no invent layout + no image decode.
    ///
    /// **Depth vs Track M:** N (and O) **require**
    /// `public_pdi_evidence_depth == status_name_only`. Track M's
    /// [`PublicPdiEvidenceScan::is_scan_closed_no_new_public_fields`] is
    /// intentionally **narrower** (fields/decode/layout only — no depth gate).
    /// Shared with M: non-zero fields / decode / invent-layout clear closed;
    /// diagnostic `name_ok` flags do not. This is **not** full M closed parity
    /// on depth.
    #[must_use]
    pub fn is_external_scan_closed_no_new(&self) -> bool {
        self.external_kernel_xrt_beyond_status_name == 0
            && self.external_public_pdi_evidence == EXTERNAL_PUBLIC_PDI_EVIDENCE_TOKEN
            && !self.public_pdi_image_decode
            && self.do_not_invent_pdi_layout
            && self.public_pdi_evidence_depth == PUBLIC_PDI_EVIDENCE_DEPTH_TOKEN
    }

    /// Soft research note (always honesty trio; closed bit **derived** from fields).
    ///
    /// String field values are passed through [`sanitize_soft_note_field_value`]
    /// so hostile hand-built bags cannot inject extra whitespace tokens.
    #[must_use]
    pub fn format_soft_note(&self) -> String {
        let closed = u8::from(self.is_external_scan_closed_no_new());
        let mut parts = vec![
            "external_public_pdi_evidence_scan".into(),
            "not_c_ready=1".into(),
            "not_ran_on_npu=1".into(),
            "invent_pass=0".into(),
            format!(
                "external_public_pdi_evidence={}",
                sanitize_soft_note_field_value(self.external_public_pdi_evidence)
            ),
            format!(
                "external_kernel_xrt_beyond_status_name={}",
                self.external_kernel_xrt_beyond_status_name
            ),
            format!(
                "public_pdi_evidence_depth={}",
                sanitize_soft_note_field_value(self.public_pdi_evidence_depth)
            ),
            format!(
                "do_not_invent_pdi_layout={}",
                u8::from(self.do_not_invent_pdi_layout)
            ),
            format!(
                "public_pdi_image_decode={}",
                u8::from(self.public_pdi_image_decode)
            ),
            format!(
                "app_load_code_name_ok={}",
                u8::from(self.app_load_code_name_ok)
            ),
            format!(
                "invalid_param_code_name_ok={}",
                u8::from(self.invalid_param_code_name_ok)
            ),
            // Derived from fields — not a hard-coded =1 (hostile bag → =0).
            format!("{EXTERNAL_PUBLIC_PDI_SCAN_CLOSED_TOKEN_KEY}={closed}"),
        ];
        // Anchor named codes without inventing a layout.
        parts.push(format!("locked_app_load=0x{AIE2_FW_APP_LOAD_PDI_FAIL:x}"));
        parts.push(format!("locked_invalid_param=0x{AIE2_FW_INVALID_PARAM:x}"));
        parts.join(" ")
    }
}

/// Pure external public-evidence scan for PDI/partition load (Track N).
///
/// Returns the **closed** conclusion locked by the human offline research
/// stamp ([`EXTERNAL_KERNEL_XRT_BEYOND_STATUS_NAME`] / `closed_no_new`): no new
/// external public kernel/XRT fields beyond Track M status-name depth. Runtime
/// only re-checks locked FW codes via [`aie2_fw_status_name`] (label only).
///
/// Does **not** fetch external XRT trees, walk the network, open L3, or decode
/// private blobs. Always soft; never Pass / C-ready / second unlock.
#[must_use]
pub fn scan_external_public_pdi_evidence() -> ExternalPublicPdiEvidenceScan {
    let app_ok = aie2_fw_status_name(AIE2_FW_APP_LOAD_PDI_FAIL) == "APP_LOAD_PDI_FAIL";
    let inv_ok = aie2_fw_status_name(AIE2_FW_INVALID_PARAM) == "AIE2_STATUS_INVALID_PARAM";
    ExternalPublicPdiEvidenceScan {
        external_public_pdi_evidence: EXTERNAL_PUBLIC_PDI_EVIDENCE_TOKEN,
        external_kernel_xrt_beyond_status_name: EXTERNAL_KERNEL_XRT_BEYOND_STATUS_NAME,
        public_pdi_evidence_depth: PUBLIC_PDI_EVIDENCE_DEPTH_TOKEN,
        do_not_invent_pdi_layout: true,
        public_pdi_image_decode: false,
        app_load_code_name_ok: app_ok,
        invalid_param_code_name_ok: inv_ok,
    }
}

/// Alias: explicit closed-scan constructor (same as [`scan_external_public_pdi_evidence`]).
#[must_use]
pub fn scan_external_public_pdi_evidence_closed() -> ExternalPublicPdiEvidenceScan {
    scan_external_public_pdi_evidence()
}

// ---------------------------------------------------------------------------
// Track O — maintain reconfirm (A–N still closed; no new external evidence)
// ---------------------------------------------------------------------------

/// Soft-note token value: Tracks A–N public APP_LOAD evidence remains closed
/// under this maintain turn (no new in-tree or external public fields).
///
/// Encodes the **human offline maintain conclusion** for Track O — not a new
/// evidence track and not a live FS/network/XRT walk. Distinct from Track N's
/// [`EXTERNAL_PUBLIC_PDI_EVIDENCE_TOKEN`] (`closed_no_new`) so bags do not bleed.
pub const APP_LOAD_MAINTAIN_RECONFIRM_TOKEN: &str = "a_n_closed_no_new";

/// Soft-note token when composed Track M and/or N is **not** closed.
///
/// Production [`reconfirm_app_load_tracks_a_n_closed`] emits this instead of
/// [`APP_LOAD_MAINTAIN_RECONFIRM_TOKEN`] so the soft-note token matches openness
/// (not a stale `a_n_closed_no_new` when M/N open).
pub const APP_LOAD_MAINTAIN_RECONFIRM_OPEN_TOKEN: &str = "not_closed";

/// Soft-note key for derived maintain-reconfirm closed bit
/// (`maintain_reconfirm_a_n_closed=`).
///
/// Value is **derived** from fields in
/// [`AppLoadMaintainReconfirm::format_soft_note`] — never hard-coded `=1` when
/// the bag is non-closed.
pub const APP_LOAD_MAINTAIN_RECONFIRM_CLOSED_TOKEN_KEY: &str = "maintain_reconfirm_a_n_closed";

/// Offline maintain reconfirm: Tracks A–N public APP_LOAD evidence still closed.
///
/// Honesty:
/// - **Not** a new silicon-progress / evidence-depth track.
/// - No private PDI decode / no invent prop_count=15 / Pass / C-ready / second unlock.
/// - Composes Track M + N closed conclusions; does **not** claim new external
///   public fields or full internet/XRT exhaustiveness.
/// - Soft bag name `app_load_maintain_reconfirm=` is **distinct** from
///   `public_pdi_evidence_scan=` (M) and `external_public_pdi_evidence_scan=` (N).
///
/// **Construction:** prefer [`reconfirm_app_load_tracks_a_n_closed`] /
/// [`reconfirm_app_load_tracks_a_n_closed_maintain`]. [`Default`] **delegates**
/// to the same path (M+N scan composition + status-name re-check).
///
/// **Depth vs Track M:** O (and N) gate closed on
/// `public_pdi_evidence_depth == status_name_only`. Track M's
/// [`PublicPdiEvidenceScan::is_scan_closed_no_new_public_fields`] is intentionally
/// **narrower** (fields/decode/layout only) — O's depth check is authoritative
/// for maintain reconfirm.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // research flag bag; not a state machine
pub struct AppLoadMaintainReconfirm {
    /// [`APP_LOAD_MAINTAIN_RECONFIRM_TOKEN`] when M+N closed; else
    /// [`APP_LOAD_MAINTAIN_RECONFIRM_OPEN_TOKEN`] from production constructors.
    pub maintain_reconfirm: &'static str,
    /// Track M closed bit (from [`PublicPdiEvidenceScan::is_scan_closed_no_new_public_fields`]).
    pub in_tree_m_closed: bool,
    /// Track N closed bit (from [`ExternalPublicPdiEvidenceScan::is_external_scan_closed_no_new`]).
    pub external_n_closed: bool,
    /// Echo of composed M/N depth — `status_name_only` when both match closed depth.
    pub public_pdi_evidence_depth: &'static str,
    /// Echo of M∧N `do_not_invent_pdi_layout` (production closed → true).
    pub do_not_invent_pdi_layout: bool,
    /// Echo of M∨N `public_pdi_image_decode` (production closed → false).
    pub public_pdi_image_decode: bool,
    /// Echo of Track M new-fields count (`0` when closed).
    pub new_public_pdi_fields: u8,
    /// Echo of Track N external beyond-status-name count (`0` when closed).
    pub external_kernel_xrt_beyond_status_name: u8,
    /// Named APP_LOAD code still maps via [`aie2_fw_status_name`] only.
    /// Diagnostic soft-note field — **not** part of
    /// [`Self::is_maintain_reconfirm_a_n_closed`].
    pub app_load_code_name_ok: bool,
    /// Named INVALID_PARAM code still maps via [`aie2_fw_status_name`] only.
    /// Diagnostic soft-note field — **not** part of
    /// [`Self::is_maintain_reconfirm_a_n_closed`].
    pub invalid_param_code_name_ok: bool,
}

impl Default for AppLoadMaintainReconfirm {
    fn default() -> Self {
        reconfirm_app_load_tracks_a_n_closed()
    }
}

impl AppLoadMaintainReconfirm {
    /// True when this bag is the closed maintain-reconfirm conclusion (Track O).
    ///
    /// Used to derive `maintain_reconfirm_a_n_closed=` so a hostile hand-built
    /// bag cannot emit closed=1 alongside non-closed M/N / non-zero counts /
    /// decode / wrong depth / non-`a_n_closed_no_new` token.
    ///
    /// **Not** gated on `app_load_code_name_ok` / `invalid_param_code_name_ok` —
    /// those are **diagnostic-only** status-name labels (always printed in the
    /// soft note). Closed = M closed + N closed + maintain token + status-name
    /// depth + no invent layout + no image decode + zero new-field counts.
    #[must_use]
    pub fn is_maintain_reconfirm_a_n_closed(&self) -> bool {
        self.maintain_reconfirm == APP_LOAD_MAINTAIN_RECONFIRM_TOKEN
            && self.in_tree_m_closed
            && self.external_n_closed
            && self.new_public_pdi_fields == 0
            && self.external_kernel_xrt_beyond_status_name == 0
            && !self.public_pdi_image_decode
            && self.do_not_invent_pdi_layout
            && self.public_pdi_evidence_depth == PUBLIC_PDI_EVIDENCE_DEPTH_TOKEN
    }

    /// Soft research note (always honesty trio; closed bit **derived** from fields).
    ///
    /// String field values are passed through [`sanitize_soft_note_field_value`]
    /// so hostile hand-built bags cannot inject extra whitespace tokens.
    #[must_use]
    pub fn format_soft_note(&self) -> String {
        let closed = u8::from(self.is_maintain_reconfirm_a_n_closed());
        let mut parts = vec![
            "app_load_maintain_reconfirm".into(),
            "not_c_ready=1".into(),
            "not_ran_on_npu=1".into(),
            "invent_pass=0".into(),
            format!(
                "maintain_reconfirm={}",
                sanitize_soft_note_field_value(self.maintain_reconfirm)
            ),
            format!("in_tree_m_closed={}", u8::from(self.in_tree_m_closed)),
            format!("external_n_closed={}", u8::from(self.external_n_closed)),
            format!(
                "public_pdi_evidence_depth={}",
                sanitize_soft_note_field_value(self.public_pdi_evidence_depth)
            ),
            format!(
                "do_not_invent_pdi_layout={}",
                u8::from(self.do_not_invent_pdi_layout)
            ),
            format!(
                "public_pdi_image_decode={}",
                u8::from(self.public_pdi_image_decode)
            ),
            format!("new_public_pdi_fields={}", self.new_public_pdi_fields),
            format!(
                "external_kernel_xrt_beyond_status_name={}",
                self.external_kernel_xrt_beyond_status_name
            ),
            format!(
                "app_load_code_name_ok={}",
                u8::from(self.app_load_code_name_ok)
            ),
            format!(
                "invalid_param_code_name_ok={}",
                u8::from(self.invalid_param_code_name_ok)
            ),
            // Derived from fields — not a hard-coded =1 (hostile bag → =0).
            format!("{APP_LOAD_MAINTAIN_RECONFIRM_CLOSED_TOKEN_KEY}={closed}"),
        ];
        // Anchor named codes without inventing a layout.
        parts.push(format!("locked_app_load=0x{AIE2_FW_APP_LOAD_PDI_FAIL:x}"));
        parts.push(format!("locked_invalid_param=0x{AIE2_FW_INVALID_PARAM:x}"));
        parts.join(" ")
    }
}

/// Pure maintain reconfirm that Tracks A–N public APP_LOAD evidence is still closed.
///
/// Composes [`scan_public_pdi_partition_evidence`] (Track M) +
/// [`scan_external_public_pdi_evidence`] (Track N). Runtime only re-checks locked
/// FW codes via those scans' [`aie2_fw_status_name`] labels — **no** FS walk,
/// **no** network/XRT tree walk, **no** private PDI decode.
///
/// Field values (**depth / layout / decode / counts**) are **echoed** from M+N
/// (not hard-coded closed constants). Maintain token is
/// [`APP_LOAD_MAINTAIN_RECONFIRM_TOKEN`] only when both M and N closed bits hold;
/// otherwise [`APP_LOAD_MAINTAIN_RECONFIRM_OPEN_TOKEN`].
///
/// Always soft; never Pass / C-ready / second unlock. Does **not** invent new
/// external public evidence (this turn found none).
#[must_use]
pub fn reconfirm_app_load_tracks_a_n_closed() -> AppLoadMaintainReconfirm {
    let m = scan_public_pdi_partition_evidence();
    let n = scan_external_public_pdi_evidence();
    let m_closed = m.is_scan_closed_no_new_public_fields();
    let n_closed = n.is_external_scan_closed_no_new();
    // Echo depth: both match → that value; disagree → explicit mismatch (closed fails).
    let depth = if m.public_pdi_evidence_depth == n.public_pdi_evidence_depth {
        m.public_pdi_evidence_depth
    } else {
        "m_n_depth_mismatch"
    };
    AppLoadMaintainReconfirm {
        maintain_reconfirm: if m_closed && n_closed {
            APP_LOAD_MAINTAIN_RECONFIRM_TOKEN
        } else {
            APP_LOAD_MAINTAIN_RECONFIRM_OPEN_TOKEN
        },
        in_tree_m_closed: m_closed,
        external_n_closed: n_closed,
        public_pdi_evidence_depth: depth,
        do_not_invent_pdi_layout: m.do_not_invent_pdi_layout && n.do_not_invent_pdi_layout,
        public_pdi_image_decode: m.public_pdi_image_decode || n.public_pdi_image_decode,
        new_public_pdi_fields: m.new_public_pdi_fields,
        external_kernel_xrt_beyond_status_name: n.external_kernel_xrt_beyond_status_name,
        app_load_code_name_ok: m.app_load_code_name_ok && n.app_load_code_name_ok,
        invalid_param_code_name_ok: m.invalid_param_code_name_ok && n.invalid_param_code_name_ok,
    }
}

/// Alias: explicit maintain constructor (same as [`reconfirm_app_load_tracks_a_n_closed`]).
#[must_use]
pub fn reconfirm_app_load_tracks_a_n_closed_maintain() -> AppLoadMaintainReconfirm {
    reconfirm_app_load_tracks_a_n_closed()
}

/// Public-kernel cmdlist fill cross-check (Track G depth; no private fill body).
///
/// Maps locked 112 B live dump + cmdlist msg `0x70` to START_NPU fill_slot
/// expectations already unit-locked in Tracks F–G. **Does not** invent fill_slot
/// post-copy body beyond public size/logical field map.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[allow(clippy::struct_excessive_bools)] // research flag bag; not a state machine
pub struct C2CmdlistFillCrossCheck {
    /// Live cmdbuf byte length (words × 4).
    pub live_cmdbuf_bytes: usize,
    /// Cmdlist msg size field (`0x70` expected).
    pub cmdlist_msg_size_field: Option<u32>,
    /// True when msg size == live cmdbuf bytes == [`C2_LIVE_CMDLIST_MSG_SIZE_BYTES`].
    pub msg_size_matches_live_cmdbuf: bool,
    /// Pure START_NPU BO byte length when provided.
    pub pure_start_npu_bytes: Option<usize>,
    /// True when pure BO is 21 words (84 B) — locked xrt3 content path.
    pub pure_is_21w_start_npu: bool,
    /// Live dump carries content instr size `0x698`.
    pub live_instr_size_content_1688: bool,
    /// Logical paddrs match locked C2 expect (via research map).
    pub logical_paddrs_match_expect: bool,
    /// Envelope exact pure vs live (false on locked C2).
    pub envelope_exact_match: bool,
}

impl C2CmdlistFillCrossCheck {
    /// Soft research note.
    #[must_use]
    pub fn format_soft_note(&self) -> String {
        let mut parts = vec![
            "c2_cmdlist_fill_cross_check".into(),
            "not_c_ready=1".into(),
            "not_ran_on_npu=1".into(),
            "invent_pass=0".into(),
            format!("live_cmdbuf_bytes={}", self.live_cmdbuf_bytes),
            format!(
                "msg_size_matches_live_cmdbuf={}",
                u8::from(self.msg_size_matches_live_cmdbuf)
            ),
            format!(
                "pure_is_21w_start_npu={}",
                u8::from(self.pure_is_21w_start_npu)
            ),
            format!(
                "live_instr_size_content_1688={}",
                u8::from(self.live_instr_size_content_1688)
            ),
            format!(
                "logical_paddrs_match_expect={}",
                u8::from(self.logical_paddrs_match_expect)
            ),
            format!(
                "envelope_exact_match={}",
                u8::from(self.envelope_exact_match)
            ),
            format!("expected_cmdlist_msg_size=0x{C2_LIVE_CMDLIST_MSG_SIZE_BYTES:x}"),
            // Public fill_slot expectation: slot message size tracks cmdbuf dump.
            "fill_slot_public_expect=msg_size_eq_cmdbuf_dump".into(),
            "fill_slot_body_invent=0".into(),
            "do_not_invent_prop_count_15=1".into(),
        ];
        if let Some(sz) = self.cmdlist_msg_size_field {
            parts.push(format!("cmdlist_msg_size_field=0x{sz:x}"));
        } else {
            parts.push("cmdlist_msg_size_field=absent".into());
        }
        if let Some(b) = self.pure_start_npu_bytes {
            parts.push(format!("pure_start_npu_bytes={b}"));
        }
        parts.join(" ")
    }
}

/// Cross-check locked live cmdlist/cmdbuf vs pure START_NPU (public kernel docs only).
///
/// `pure_bo` optional — when present, drives envelope / 21w flags. Never EXEC.
#[must_use]
pub fn cross_check_c2_cmdlist_fill_public(
    live_words: &[u32],
    msg_words: &[u32],
    pure_bo: Option<&[u8]>,
) -> C2CmdlistFillCrossCheck {
    let live_bytes = live_words.len().saturating_mul(4);
    let msg_sz = msg_words.get(4).copied();
    let msg_match = msg_sz == Some(C2_LIVE_CMDLIST_MSG_SIZE_BYTES)
        && live_bytes == C2_LIVE_CMDLIST_MSG_SIZE_BYTES as usize
        && live_words.len() == C2_LIVE_CMDBUF_WORD_COUNT;
    let expect = C2ExpectPaddrs::from_c2_locked_io();
    let map = decode_c2_live_cmdbuf_research(live_words, pure_bo, expect);
    let pure_len = pure_bo.map(<[u8]>::len);
    // Length 84 alone is not enough — require decodable START_NPU with prop_count=0.
    let pure_21 = pure_bo.is_some_and(|b| {
        b.len() == C2_PURE_START_NPU_WORD_COUNT.saturating_mul(4)
            && decode_start_npu_cmd_bo(b).is_ok_and(|d| d.prop_count == 0)
    });
    C2CmdlistFillCrossCheck {
        live_cmdbuf_bytes: live_bytes,
        cmdlist_msg_size_field: msg_sz,
        msg_size_matches_live_cmdbuf: msg_match,
        pure_start_npu_bytes: pure_len,
        pure_is_21w_start_npu: pure_21,
        live_instr_size_content_1688: map.instr_size == Some(C2_LIVE_INSTR_SIZE_CONTENT),
        logical_paddrs_match_expect: map.logical_paddrs_match_expect,
        envelope_exact_match: map.envelope_exact_match,
    }
}

/// Locked C2 embed convenience for [`cross_check_c2_cmdlist_fill_public`].
#[must_use]
pub fn cross_check_c2_cmdlist_fill_locked(pure_bo: Option<&[u8]>) -> C2CmdlistFillCrossCheck {
    cross_check_c2_cmdlist_fill_public(
        c2_live_cmdbuf_words_embedded(),
        c2_live_cmdlist_msg_words_embedded(),
        pure_bo,
    )
}

/// Pure offline Pass-evidence fields from a `last_apply.json` snapshot (C4 audit).
///
/// **Not** product Pass: live `decide_ran_on_npu` / timed path still own `ran_on_npu`.
/// This struct never invents `json_ran_on_npu=true` from missing fields.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SiliconPassEvidence {
    /// Top-level JSON `ran_on_npu` — **must** be `Some(true)` for [`SiliconPassEval::PassCandidate`].
    pub json_ran_on_npu: Option<bool>,
    /// Top-level schema-2 `ert_error` when known.
    pub ert_error_field: Option<bool>,
    /// First `ert_state=` numeric from detail when present.
    pub ert_state: Option<u32>,
    /// Detail first-token `ert_error` (Error / state 5).
    pub detail_ert_error: bool,
    /// `raw_nonzero_bytes` from ofm_stats when known.
    pub ofm_raw_nonzero_bytes: Option<u64>,
    /// Top-level `real_ioctl` — **must** be `Some(true)` for PassCandidate.
    pub real_ioctl: Option<bool>,
    /// Top-level `status` string (e.g. `ert_cmd_error` / `ok_npu_verified`).
    pub status: Option<String>,
}

/// Offline audit result for C4 Pass readiness (snapshot only — never invent product Pass).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SiliconPassEval {
    /// Snapshot does not meet candidate Pass criteria (reason is a stable token).
    NotPass {
        /// Stable reason token for scripts (`json_ran_on_npu_false`, `ert_error`, …).
        reason: &'static str,
    },
    /// Snapshot *looks* like a Pass report offline — still requires live decide for product.
    PassCandidate {
        /// Space-separated research notes (`offline_audit_only=1` always present).
        notes: String,
    },
}

impl SiliconPassEval {
    /// True when eval is [`Self::PassCandidate`] (offline audit only).
    #[must_use]
    pub const fn is_pass_candidate(&self) -> bool {
        matches!(self, Self::PassCandidate { .. })
    }

    /// Stable script token: `pass_candidate` or `not_pass`.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::PassCandidate { .. } => "pass_candidate",
            Self::NotPass { .. } => "not_pass",
        }
    }
}

/// Build [`SiliconPassEvidence`] from last_apply field slices (pure; no I/O).
///
/// Merges detail signals when `detail` is non-empty. **Never** sets
/// `json_ran_on_npu` from detail alone — JSON top-level is authoritative for
/// the hard PassCandidate gate (detail `ran_on_npu=` is residual-only).
#[must_use]
pub fn silicon_pass_evidence_from_last_apply(
    json_ran_on_npu: Option<bool>,
    status: Option<&str>,
    detail: Option<&str>,
    ert_error_field: Option<bool>,
    real_ioctl: Option<bool>,
) -> SiliconPassEvidence {
    let sig = detail
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(parse_last_apply_detail_signals);
    SiliconPassEvidence {
        json_ran_on_npu,
        ert_error_field,
        ert_state: sig.as_ref().and_then(|s| s.ert_state),
        detail_ert_error: sig.as_ref().is_some_and(|s| s.ert_error),
        ofm_raw_nonzero_bytes: sig.as_ref().and_then(|s| s.ofm_raw_nonzero_bytes),
        real_ioctl,
        status: status
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned),
    }
}

/// Offline audit of a last_apply snapshot for **candidate** Pass reporting.
///
/// **Hard rule:** `json_ran_on_npu` must be `Some(true)` — missing or `false`
/// → [`SiliconPassEval::NotPass`] (never invent Pass).
///
/// Additional gates (all required for PassCandidate):
/// - `real_ioctl == Some(true)` (missing → `real_ioctl_missing`)
/// - not ERT error (field, detail first-token, or status `ert_cmd_error` / `ert_error`)
/// - completed: first `ert_state=4` **and/or** status **exact** `ok_npu_verified`
///   (case-insensitive). No `contains`/`starts_with` — blocks `not_completed`,
///   `ok_npu_fake`, `ok_npu_error`. Present non-exact status → NotPass.
/// - `ofm_raw_nonzero_bytes` **must** be `Some(n)` with `n > 0` (missing →
///   `ofm_nonzero_missing`; zero → `ofm_all_zero`)
///
/// **Product Pass** still requires the live `decide_ran_on_npu` path — this is
/// snapshot audit only (`offline_audit_only=1`). Always `not_c_ready=1` /
/// `invent_pass=0` in the note (even PassCandidate).
#[must_use]
pub fn evaluate_silicon_pass_evidence(ev: &SiliconPassEvidence) -> SiliconPassEval {
    match ev.json_ran_on_npu {
        None => {
            return SiliconPassEval::NotPass {
                reason: "json_ran_on_npu_missing",
            };
        }
        Some(false) => {
            return SiliconPassEval::NotPass {
                reason: "json_ran_on_npu_false",
            };
        }
        Some(true) => {}
    }

    match ev.real_ioctl {
        Some(true) => {}
        Some(false) => {
            return SiliconPassEval::NotPass {
                reason: "real_ioctl_false",
            };
        }
        None => {
            return SiliconPassEval::NotPass {
                reason: "real_ioctl_missing",
            };
        }
    }

    if silicon_pass_evidence_has_ert_error(ev) {
        return SiliconPassEval::NotPass {
            reason: "ert_error",
        };
    }

    // Present status that is not exact ok_npu_verified → refuse (ok_npu_fake etc.).
    if let Some(ref s) = ev.status {
        let t = s.trim();
        if !t.eq_ignore_ascii_case("ok_npu_verified") {
            return SiliconPassEval::NotPass {
                reason: "status_not_pass",
            };
        }
    }

    if !silicon_pass_evidence_ert_completed(ev) {
        return SiliconPassEval::NotPass {
            reason: "ert_not_completed",
        };
    }

    match ev.ofm_raw_nonzero_bytes {
        None => {
            return SiliconPassEval::NotPass {
                reason: "ofm_nonzero_missing",
            };
        }
        Some(0) => {
            return SiliconPassEval::NotPass {
                reason: "ofm_all_zero",
            };
        }
        Some(_) => {}
    }

    let mut notes = vec![
        "offline_audit_only=1".to_string(),
        "product_pass_requires_live_decide=1".to_string(),
        "not_c_ready=1".to_string(),
        "not_ran_on_npu=1".to_string(),
        "invent_pass=0".to_string(),
        "json_ran_on_npu=true".to_string(),
        "real_ioctl=1".to_string(),
    ];
    if let Some(st) = ev.ert_state {
        notes.push(format!("ert_state={st}"));
    }
    if let Some(nz) = ev.ofm_raw_nonzero_bytes {
        notes.push(format!("ofm_raw_nonzero={nz}"));
    }
    SiliconPassEval::PassCandidate {
        notes: notes.join(" "),
    }
}

/// True when evidence shows ERT error (field, detail, or error status token).
#[must_use]
pub fn silicon_pass_evidence_has_ert_error(ev: &SiliconPassEvidence) -> bool {
    if ev.ert_error_field == Some(true) || ev.detail_ert_error {
        return true;
    }
    if ev.ert_state == Some(5) {
        return true;
    }
    ev.status.as_deref().is_some_and(|s| {
        let t = s.trim();
        t.eq_ignore_ascii_case("ert_cmd_error") || t.eq_ignore_ascii_case("ert_error")
    })
}

/// True when evidence shows ERT Completed without inventing it.
///
/// **Exact tokens only:**
/// - first `ert_state=4` (Completed), **and/or**
/// - status **exact** `ok_npu_verified` (case-insensitive)
///
/// Does **not** use `contains("completed")` or `starts_with("ok_npu_")` — those
/// invent candidate Pass for `not_completed` / `ok_npu_fake` / `ok_npu_error`.
/// Caller may also refuse present non-exact status before this check.
#[must_use]
pub fn silicon_pass_evidence_ert_completed(ev: &SiliconPassEvidence) -> bool {
    if silicon_pass_evidence_has_ert_error(ev) {
        return false;
    }
    let status_verified = ev
        .status
        .as_deref()
        .is_some_and(|s| s.trim().eq_ignore_ascii_case("ok_npu_verified"));
    // Present non-exact status is not completed (even if ert_state=4 elsewhere).
    if ev.status.is_some() && !status_verified {
        return false;
    }
    status_verified || ev.ert_state == Some(4)
}

/// Research note for C4 offline pass check (always honest; never product Pass claim).
///
/// Even [`SiliconPassEval::PassCandidate`] is **offline audit only** — always
/// `not_c_ready=1` / `not_ran_on_npu=1` / `invent_pass=0` (snapshot ≠ product Pass).
#[must_use]
pub fn format_silicon_pass_eval_note(eval: &SiliconPassEval) -> String {
    match eval {
        SiliconPassEval::NotPass { reason } => {
            format!(
                "c4_eval=not_pass reason={reason} not_c_ready=1 not_ran_on_npu=1 \
                 offline_audit_only=1 invent_pass=0"
            )
        }
        SiliconPassEval::PassCandidate { notes } => {
            // notes already embed honesty tokens; still stamp snapshot_pass_candidate.
            format!("c4_eval=pass_candidate {notes} snapshot_pass_candidate=1")
        }
    }
}

/// SSoT **true product-Pass** criteria checklist (P6 offline readiness).
///
/// Always research-only: never invents product Pass / `ran_on_npu` from offline
/// green. Product Pass remains live `decide_ran_on_npu` / timed path only.
/// Offline C4 [`SiliconPassEval::PassCandidate`] is **not** product Pass
/// (`c4_pass_candidate_neq_product_pass=1`).
///
/// Always includes honesty:
/// `invent_pass=0 not_c_ready=1 product_pass_requires_live_decide=1 offline_audit_only=1`.
#[must_use]
pub fn format_true_pass_criteria_checklist() -> String {
    [
        "true_pass_criteria=1",
        "product_pass_requires_live_decide=1",
        "offline_audit_only=1",
        "invent_pass=0",
        "not_c_ready=1",
        "not_ran_on_npu=1",
        "product_pass=0",
        "require_ert_completed_or_ok_npu_verified=1",
        "require_ofm_nonzero_allclose_live=1",
        "require_real_ioctl=1",
        "require_json_ran_on_npu_from_product_path=1",
        "c4_pass_candidate_neq_product_pass=1",
        "userspace_elf_rela_ok_neq_fw_accept=1",
        "fw_accept_neq_product_pass=1",
        "locked_fixture_not_product_pass=1",
        "allclose_only_via_live_decide_ran_on_npu=1",
        "p6_criteria_ready_offline=1",
    ]
    .join(" ")
}

/// P6 status note from a C4 offline eval — **never** claims product Pass.
///
/// Reuses [`evaluate_silicon_pass_evidence`] results; does not re-gate Pass.
/// Even [`SiliconPassEval::PassCandidate`] stays
/// `product_pass=0 invent_pass=0 product_pass_requires_live_decide=1`.
#[must_use]
pub fn format_true_pass_p6_note(eval: &SiliconPassEval) -> String {
    match eval {
        SiliconPassEval::NotPass { reason } => {
            format!(
                "p6_status=not_pass reason={reason} product_pass=0 invent_pass=0 \
                 not_ran_on_npu=1 not_c_ready=1 product_pass_requires_live_decide=1 \
                 offline_audit_only=1 locked_fixture_or_snapshot_not_pass=1"
            )
        }
        SiliconPassEval::PassCandidate { .. } => {
            "p6_status=offline_candidate_only product_pass=0 snapshot_pass_candidate=1 \
             invent_pass=0 not_ran_on_npu=1 not_c_ready=1 \
             product_pass_requires_live_decide=1 offline_audit_only=1 \
             c4_pass_candidate_neq_product_pass=1"
                .to_string()
        }
    }
}

/// True when a research note/checklist string contains an exact whitespace token.
#[must_use]
pub fn residual_note_has_exact_token(note: &str, token: &str) -> bool {
    note.split_whitespace().any(|t| t == token)
}

/// Sanitize a soft-note **field value** before `key=value` emission.
///
/// Maps whitespace, `=`, and control characters to `_` so a hostile hand-built
/// bag cannot inject extra exact tokens (e.g. `invent_pass=1` / `ran_on_npu=1`)
/// via interpolated `&str` fields. Production constructors only use consts
/// (already safe); this hardens M/N/O research notes against bag mutation.
///
/// Does **not** alter hard-coded honesty trio tokens (`invent_pass=0` etc.).
#[must_use]
pub fn sanitize_soft_note_field_value(raw: &str) -> String {
    raw.chars()
        .map(|c| {
            if c.is_whitespace() || c == '=' || c.is_control() {
                '_'
            } else {
                c
            }
        })
        .collect()
}

/// True when env value is an explicit enable (`1` / `true`, case-insensitive).
///
/// Presence alone is **not** enough — `0` / `false` / empty must stay off.
/// Matches classify example `env_flag_truthy` (PRINT / research hard-opt-ins).
#[must_use]
pub fn env_flag_truthy_os(key: &str) -> bool {
    std::env::var(key).ok().is_some_and(|v| {
        let t = v.trim();
        t == "1" || t.eq_ignore_ascii_case("true")
    })
}

/// Prefer the `inst_blob`… region for instr BO tokens (until `bo_diag` / `ofm_stats`).
///
/// Falls back to full `detail` when `inst_blob` is absent.
#[must_use]
fn detail_instr_region(detail: &str) -> &str {
    let Some(start) = detail.find("inst_blob") else {
        return detail;
    };
    let rest = &detail[start..];
    let end = rest
        .find("bo_diag=")
        .or_else(|| rest.find("bo_diag=["))
        .or_else(|| rest.find("ofm_stats="))
        .or_else(|| rest.find("ofm_stats=["))
        .unwrap_or(rest.len());
    &rest[..end]
}

/// Best-effort pure parse of timed `detail` tokens used in `last_apply.json`.
///
/// Does **not** claim Pass; missing tokens stay `None` / false.
///
/// **`ert_state=`:** only the **first** occurrence; only the **first token**
/// after `=` is inspected (e.g. `5(Error)`). A later `(Error)` elsewhere in
/// the string must **not** set `ert_error`.
#[must_use]
pub fn parse_last_apply_detail_signals(detail: &str) -> LastApplyDetailSignals {
    let mut s = LastApplyDetailSignals::default();
    // ert_state=5(Error) or ert_state=4(Completed) — first token only
    if let Some(i) = detail.find("ert_state=") {
        let rest = &detail[i + "ert_state=".len()..];
        let tok = &rest[..detail_token_end(rest)];
        let num_end = tok.find(|c: char| !c.is_ascii_digit()).unwrap_or(tok.len());
        if num_end > 0 {
            if let Ok(n) = tok[..num_end].parse::<u32>() {
                s.ert_state = Some(n);
            }
        }
        // Parenthetical name on the same token only (not rest of detail).
        let mut name_error = false;
        if let Some(open) = tok[num_end..].find('(') {
            let after_open = &tok[num_end + open + 1..];
            if let Some(close) = after_open.find(')') {
                let name = after_open[..close].trim();
                name_error = name.eq_ignore_ascii_case("Error");
            }
        } else if tok.eq_ignore_ascii_case("Error") {
            name_error = true;
        }
        // ERT Error numeric state is 5 (`ErtCmdState::Error`).
        s.ert_error = name_error || s.ert_state == Some(5);
    }
    // aiebu_elf=1 / aiebu_elf=0 (first occurrence; first token, leading digit only)
    if let Some(i) = detail.find("aiebu_elf=") {
        let rest = &detail[i + "aiebu_elf=".len()..];
        let tok = &rest[..detail_token_end(rest)];
        s.aiebu_elf = match tok.as_bytes() {
            [b'1', rest @ ..] if rest.first().is_none_or(|b| !b.is_ascii_digit()) => Some(true),
            [b'0', rest @ ..] if rest.first().is_none_or(|b| !b.is_ascii_digit()) => Some(false),
            _ => None,
        };
    }
    if let Some(i) = detail.find("rela_count=") {
        let rest = &detail[i + "rela_count=".len()..];
        let tok = &rest[..detail_token_end(rest)];
        let num_end = tok.find(|c: char| !c.is_ascii_digit()).unwrap_or(tok.len());
        if num_end > 0 {
            if let Ok(n) = tok[..num_end].parse::<usize>() {
                s.rela_count = Some(n);
            }
        }
    }
    // Token-ish: require `aiebu_rela_patched` not as a longer identifier prefix.
    if let Some(i) = detail.find("aiebu_rela_patched") {
        let before_ok = i == 0
            || detail
                .as_bytes()
                .get(i - 1)
                .is_some_and(|b| !b.is_ascii_alphanumeric() && *b != b'_');
        let after = i + "aiebu_rela_patched".len();
        let after_ok = detail
            .as_bytes()
            .get(after)
            .is_none_or(|b| !b.is_ascii_alphanumeric() && *b != b'_');
        s.aiebu_rela_patched = before_ok && after_ok;
    }
    // Prefer `layout=xrt3` / `layout=count4` token (first layout=)
    if let Some(i) = detail.find("layout=") {
        let rest = &detail[i + "layout=".len()..];
        let end = detail_token_end(rest);
        let tok = rest[..end].trim();
        if !tok.is_empty() && tok != "na" {
            s.layout = Some(tok.to_string());
        }
    }
    if let Some(i) = detail.find("inst_size_mode=") {
        let rest = &detail[i + "inst_size_mode=".len()..];
        let end = detail_token_end(rest);
        let tok = rest[..end].trim();
        if !tok.is_empty() && tok != "na" {
            s.inst_size_mode = Some(tok.to_string());
        }
    }
    if let Some(i) = detail.find("ran_on_npu=") {
        let rest = &detail[i + "ran_on_npu=".len()..];
        let tok = &rest[..detail_token_end(rest)];
        // Exact token only — not starts_with("true") (would match trueish).
        s.ran_on_npu = match tok {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        };
    }
    if let Some(i) = detail.find("raw_nonzero_bytes=") {
        let rest = &detail[i + "raw_nonzero_bytes=".len()..];
        let tok = &rest[..detail_token_end(rest)];
        let num_end = tok.find(|c: char| !c.is_ascii_digit()).unwrap_or(tok.len());
        if num_end > 0 {
            if let Ok(n) = tok[..num_end].parse::<u64>() {
                s.ofm_raw_nonzero_bytes = Some(n);
            }
        }
    }
    // Known journal codes only when literally present (no invent).
    let dl = detail.to_ascii_lowercase();
    s.fw_status_code = if dl.contains("0x3000003") {
        Some(0x3_000_003)
    } else if dl.contains("0x4000003") {
        Some(0x4_000_003)
    } else {
        None
    };
    parse_last_apply_app_load_tokens(detail, &mut s);
    s
}

/// Fill APP_LOAD research fields on `s` from timed `detail` (offline only).
///
/// Instr BO tokens (`map_offset`, `ty`, `xdna_addr`, sizes) prefer the
/// [`detail_instr_region`] slice so a leading `bo_diag=` cannot steal first-match.
fn parse_last_apply_app_load_tokens(detail: &str, s: &mut LastApplyDetailSignals) {
    let instr = detail_instr_region(detail);
    // map_offset / ty / xdna_addr from instr region (fallback: full detail).
    let map_src = if detail_first_token_after(instr, "map_offset=").is_some() {
        instr
    } else {
        detail
    };
    if let Some(tok) = detail_first_token_after(map_src, "map_offset=") {
        s.instr_map_offset = parse_detail_u64_token(tok);
    }
    let ty_src = if detail_first_token_after(instr, "ty=").is_some() {
        instr
    } else {
        detail
    };
    if let Some(tok) = detail_first_token_after(ty_src, "ty=") {
        // Keep short buffer-type tokens only (Dev / Shmem / DevHeap / Cmd …).
        if tok.len() <= 16 && tok.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
            s.instr_bo_type = Some(tok.to_string());
        }
    }
    let xdna_src = if detail_first_token_after(instr, "xdna_addr=").is_some() {
        instr
    } else {
        detail
    };
    if let Some(tok) = detail_first_token_after(xdna_src, "xdna_addr=") {
        s.instr_xdna_addr = parse_detail_u64_token(tok);
    }
    // Sizes: prefer instr region, then full detail.
    if let Some(tok) = detail_first_token_after(instr, "content_size=")
        .or_else(|| detail_first_token_after(detail, "content_size="))
    {
        s.content_size_b = parse_detail_size_bytes_token(tok);
    } else if let Some(tok) = detail_first_token_after(instr, "content=") {
        s.content_size_b = parse_detail_size_bytes_token(tok);
    }
    if let Some(tok) = detail_first_token_after(instr, "bo_size=")
        .or_else(|| detail_first_token_after(detail, "bo_size="))
    {
        s.bo_size_b = parse_detail_size_bytes_token(tok);
    }
    if let Some(tok) = detail_first_token_after(instr, "start_npu_size=")
        .or_else(|| detail_first_token_after(detail, "start_npu_size="))
    {
        s.start_npu_size_b = parse_detail_size_bytes_token(tok);
    }
    // Substring match (detail may embed `Some(SubmitWaitNotOk)`); not a bare token.
    s.submit_wait_not_ok = detail.contains("SubmitWaitNotOk");
    if let Some(tok) = detail_first_token_after(detail, "rela_mode=") {
        if tok == "overwrite" || tok == "add" {
            s.rela_mode = Some(tok.to_string());
        }
    }
    // Lab CONFIG_CU attach tokens (research; attach alone ≠ Pass / ≠ residual class change).
    if let Some(tok) = detail_first_token_after(detail, "config_cu=") {
        s.config_cu = match tok.as_bytes() {
            [b'1', rest @ ..] if rest.first().is_none_or(|b| !b.is_ascii_digit()) => Some(1),
            [b'0', rest @ ..] if rest.first().is_none_or(|b| !b.is_ascii_digit()) => Some(0),
            _ => {
                // Accept small decimal cu flag (still research-only).
                let num_end = tok.find(|c: char| !c.is_ascii_digit()).unwrap_or(tok.len());
                if num_end > 0 {
                    tok[..num_end].parse::<u8>().ok()
                } else {
                    None
                }
            }
        };
    }
    if let Some(tok) = detail_first_token_after(detail, "pdi_bytes=") {
        s.pdi_bytes = parse_detail_size_bytes_token(tok);
    }
    if let Some(tok) = detail_first_token_after(detail, "cu_func=") {
        let num_end = tok.find(|c: char| !c.is_ascii_digit()).unwrap_or(tok.len());
        if num_end > 0 {
            if let Ok(n) = tok[..num_end].parse::<u8>() {
                s.cu_func = Some(n);
            }
        }
    }
    // Failed attach: bare token or live timed-path phrase.
    if let Some(i) = detail.find("config_cu_failed") {
        let before_ok = i == 0
            || detail
                .as_bytes()
                .get(i - 1)
                .is_some_and(|b| !b.is_ascii_alphanumeric() && *b != b'_');
        let after = i + "config_cu_failed".len();
        let after_ok = detail
            .as_bytes()
            .get(after)
            .is_none_or(|b| !b.is_ascii_alphanumeric() && *b != b'_');
        if before_ok && after_ok {
            s.config_cu_failed = true;
        }
    }
    if !s.config_cu_failed
        && detail
            .to_ascii_lowercase()
            .contains("config_cu attach failed")
    {
        s.config_cu_failed = true;
    }
}

/// Offline userspace-vs-FW checklist from last_apply signals (research only).
///
/// Unit-locks APP_LOAD **signal matrix** already present in detail
/// (`map_offset`, Dev BO, content/bo/start_npu sizes, `SubmitWaitNotOk`, OFM
/// zeros, missing journal hex). **Never** invents Pass / C-ready / `ran_on_npu`.
///
/// Print this string **as-is** (already starts with `app_load_checklist=1`).
///
/// For optional journal dual-status (`0x3000003` / `0x4000003`), use
/// [`format_app_load_offline_checklist_ex`].
#[must_use]
pub fn format_app_load_offline_checklist(
    class: SiliconResidualClass,
    sig: &LastApplyDetailSignals,
) -> String {
    format_app_load_offline_checklist_ex(class, sig, None)
}

/// Same as [`format_app_load_offline_checklist`] with optional journal / multi-line
/// text for dual FW status codes.
///
/// - When `journal_text` is `None` or empty/whitespace: always emits
///   `journal_text_absent=1` (plus `fw_code_absent_in_detail=1` if detail lacks hex).
/// - When present: parses via [`parse_journal_fw_status_pair`] and emits named
///   codes **only** when literals parse — never invents `0x3000003` / pair.
/// - Always ends honesty with `invent_pass=0` (research checklist ≠ product Pass).
#[must_use]
pub fn format_app_load_offline_checklist_ex(
    class: SiliconResidualClass,
    sig: &LastApplyDetailSignals,
    journal_text: Option<&str>,
) -> String {
    let mut parts = vec![
        "app_load_checklist=1".to_string(),
        format!("residual={}", class.as_str()),
        "not_c_ready=1".to_string(),
        "not_ran_on_npu=1".to_string(),
        "invent_pass=0".to_string(),
    ];
    // Userspace OK only with explicit aiebu=true + rela evidence (not aiebu unknown).
    if sig.aiebu_elf == Some(true) && last_apply_has_rela_evidence(sig) {
        parts.push("userspace_elf_rela_ok=1".to_string());
    } else {
        parts.push("userspace_elf_rela_ok=0".to_string());
        if sig.aiebu_elf.is_none() {
            parts.push("aiebu_unknown=1".to_string());
        }
    }
    append_checklist_instr_ty_token(&mut parts, sig.instr_bo_type.as_deref());
    match sig.instr_map_offset {
        Some(off) if instr_map_offset_is_no_mmap(off) => {
            parts.push("instr_map_offset_no_mmap=1".to_string());
            // "not APP_LOAD root" only when Dev ty confirms expected DevHeap fill path.
            if sig.instr_bo_type.as_deref() == Some("Dev") {
                parts.push("map_offset_not_app_load_root=1".to_string());
            } else {
                parts.push("map_offset_no_mmap_ty_not_dev=1".to_string());
            }
        }
        Some(off) => parts.push(format!("instr_map_offset=0x{off:x}")),
        None => parts.push("instr_map_offset_absent=1".to_string()),
    }
    if let Some(addr) = sig.instr_xdna_addr {
        if is_valid_device_paddr(addr) {
            parts.push(format!("instr_xdna_addr_ok=0x{addr:x}"));
        } else {
            parts.push(format!("instr_xdna_addr_invalid=0x{addr:x}"));
        }
    }
    if last_apply_start_npu_matches_content(sig) {
        parts.push("start_npu_matches_content=1".to_string());
    }
    if last_apply_bo_gt_content(sig) {
        parts.push("bo_gt_content=1".to_string());
    }
    // Print partial sizes independently (do not require all three).
    if let Some(c) = sig.content_size_b {
        parts.push(format!("content_size={c}B"));
    }
    if let Some(b) = sig.bo_size_b {
        parts.push(format!("bo_size={b}B"));
    }
    if let Some(st) = sig.start_npu_size_b {
        parts.push(format!("start_npu_size={st}B"));
    }
    append_checklist_allowlisted_mode(
        &mut parts,
        "inst_size_mode",
        sig.inst_size_mode.as_deref(),
        &["content", "bo"],
    );
    append_checklist_allowlisted_mode(
        &mut parts,
        "rela_mode",
        sig.rela_mode.as_deref(),
        &["overwrite", "add"],
    );
    append_checklist_allowlisted_mode(
        &mut parts,
        "layout",
        sig.layout.as_deref(),
        &["count4", "xrt3"],
    );
    append_checklist_config_cu_tokens(&mut parts, class, sig);
    // Submit / OFM layer (userspace saw FW reject, not ioctl hang)
    if sig.submit_wait_not_ok {
        parts.push("submit_wait_not_ok=1".to_string());
    }
    match sig.ofm_raw_nonzero_bytes {
        Some(0) => parts.push("ofm_all_zero=1".to_string()),
        Some(n) => parts.push(format!("ofm_raw_nonzero={n}")),
        None => {}
    }
    // FW layer: name only when hex code literally present (no invent APP_LOAD name).
    if let Some(code) = sig.fw_status_code {
        parts.push(format!("fw_code=0x{code:x}"));
        parts.push(format!("fw_name={}", aie2_fw_status_name(code)));
    } else {
        parts.push("fw_code_absent_in_detail=1".to_string());
        parts.push("fw_code=unknown_check_journal".to_string());
    }
    // Optional journal dual-status (Track I): never invent codes when absent.
    append_journal_dual_checklist_tokens(&mut parts, journal_text);
    if class.is_locked_app_load_after_aiebu() {
        // Class label already in residual=; do not invent journal code narrative.
        parts.push("class_implies_app_load_after_aiebu=1".to_string());
        parts.push("env_hitl_closed=1".to_string());
        // Track K: userspace ELF+rela green but FW APP_LOAD residual still open.
        // Only when checklist already marked userspace_elf_rela_ok (aiebu+rela).
        if sig.aiebu_elf == Some(true) && last_apply_has_rela_evidence(sig) {
            // Distinct from research `userspace_ok_fw_load_open` (which also needs
            // classic walk of patched ctrltext). Checklist has last_apply signals only.
            parts.push("checklist_userspace_elf_rela_app_load=1".to_string());
        }
        // Track L: public PDI/partition surface is status-name only (no image decode).
        // Distinct from research `partition_surface_public=` / `pdi_marker_public=`
        // on `AppLoadPdiPartitionResearch` (same values; checklist prefix avoids merge).
        parts.push(format!(
            "checklist_partition_surface_public={PUBLIC_PARTITION_SURFACE_TOKEN}"
        ));
        parts.push(format!(
            "checklist_pdi_marker_public={PUBLIC_PDI_MARKER_TOKEN}"
        ));
        parts.push("checklist_public_pdi_image_decode=0".to_string());
    }
    parts.join(" ")
}

/// Lab CONFIG_CU attach honesty tokens (P5; attach alone ≠ Pass / ≠ class invent).
fn append_checklist_config_cu_tokens(
    parts: &mut Vec<String>,
    class: SiliconResidualClass,
    sig: &LastApplyDetailSignals,
) {
    let cu_present = last_apply_config_cu_present(sig);
    parts.push(format!("config_cu_present={}", u8::from(cu_present)));
    parts.push(format!(
        "config_cu_failed={}",
        u8::from(sig.config_cu_failed)
    ));
    if let Some(v) = sig.config_cu {
        parts.push(format!("config_cu={v}"));
    }
    if let Some(n) = sig.pdi_bytes {
        parts.push(format!("pdi_bytes={n}"));
    }
    if let Some(f) = sig.cu_func {
        parts.push(format!("cu_func={f}"));
    }
    // Note only: CONFIG_CU + still locked APP_LOAD (wrong PDI / multi-CU / ctrltext).
    if cu_present && !sig.config_cu_failed && class.is_locked_app_load_after_aiebu() {
        parts.push("config_cu_attached_still_app_load=1".into());
    }
}

/// Allowlist instr BO type tokens (token integrity; unknown → `other`).
fn append_checklist_instr_ty_token(parts: &mut Vec<String>, ty: Option<&str>) {
    match ty {
        Some("Dev") => parts.push("instr_ty_dev=1".into()),
        Some(t @ ("Shmem" | "Cmd" | "ExecBuf" | "DevHeap")) => {
            parts.push(format!("instr_ty={t}"));
        }
        Some(_) => parts.push("instr_ty=other".into()),
        None => parts.push("instr_ty_unknown=1".into()),
    }
}

/// Emit `key=value` only for allowlisted values (else `key=other`); skip if None.
fn append_checklist_allowlisted_mode(
    parts: &mut Vec<String>,
    key: &str,
    value: Option<&str>,
    allow: &[&str],
) {
    let Some(v) = value else {
        return;
    };
    let out = if allow.contains(&v) { v } else { "other" };
    parts.push(format!("{key}={out}"));
}

/// Append journal dual-status tokens for the APP_LOAD checklist (never invents).
fn append_journal_dual_checklist_tokens(parts: &mut Vec<String>, journal_text: Option<&str>) {
    let Some(jt) = journal_text.map(str::trim).filter(|s| !s.is_empty()) else {
        parts.push("journal_text_absent=1".into());
        return;
    };
    let pair = parse_journal_fw_status_pair(jt);
    match pair.failed_cmd_status {
        Some(c) => {
            parts.push(format!("journal_failed_cmd=0x{c:x}"));
            parts.push(format!(
                "journal_failed_cmd_name={}",
                aie2_fw_status_name(c)
            ));
        }
        None => parts.push("journal_failed_cmd=absent".into()),
    }
    match pair.chain_status {
        Some(c) => {
            parts.push(format!("journal_chain_status=0x{c:x}"));
            parts.push(format!("journal_chain_name={}", aie2_fw_status_name(c)));
        }
        None => parts.push("journal_chain_status=absent".into()),
    }
    if pair.is_c2_app_load_pair() {
        parts.push("journal_c2_app_load_pair=1".into());
    } else {
        parts.push("journal_c2_app_load_pair=0".into());
    }
}

/// Classify residual from optional structured last_apply fields + detail text.
///
/// - Never invents `ran_on_npu` / Pass / C-ready.
/// - ERT Error from: `status` ∈ {`ert_cmd_error`,`ert_error`}, `ert_error_field`,
///   or first `ert_state=` token Error/5 in detail.
/// - [`SiliconResidualClass::AppLoadPdiFailAfterAiebu`]: aiebu **and**
///   rela evidence (`rela_count>0` or `aiebu_rela_patched`) **and** ert_error.
/// - aiebu + ert_error **without** rela evidence → [`SiliconResidualClass::ErtErrorUnknownBlob`].
/// - aiebu false + ert_error → raw/unpatched APP_LOAD class.
///
/// Detail `ran_on_npu=true` is **ignored for Pass** — residual labels only;
/// product Pass stays `decide_ran_on_npu` (not live-wired here).
///
/// For schema-2 snapshots that put `rela_count` only at the **JSON top level**
/// (not in `detail`), use [`classify_last_apply_residual_ex`].
#[must_use]
pub fn classify_last_apply_residual(
    status: Option<&str>,
    detail: Option<&str>,
    aiebu_elf_field: Option<bool>,
    ert_error_field: Option<bool>,
) -> SiliconResidualClass {
    classify_last_apply_residual_ex(status, detail, aiebu_elf_field, ert_error_field, None)
}

/// Same as [`classify_last_apply_residual`] with optional top-level `rela_count`
/// from schema-2 `last_apply.json` (feeds rela evidence when detail omits it).
///
/// `rela_count_field > 0` counts as rela evidence; `0` / `None` do not invent it.
#[must_use]
pub fn classify_last_apply_residual_ex(
    status: Option<&str>,
    detail: Option<&str>,
    aiebu_elf_field: Option<bool>,
    ert_error_field: Option<bool>,
    rela_count_field: Option<usize>,
) -> SiliconResidualClass {
    let detail_nonempty = detail.map(str::trim).filter(|s| !s.is_empty());
    let mut sig = detail_nonempty.map_or_else(LastApplyDetailSignals::default, |d| {
        parse_last_apply_detail_signals(d)
    });
    // Schema-2 top-level rela_count when detail lacks rela evidence.
    if !last_apply_has_rela_evidence(&sig) {
        if let Some(n) = rela_count_field.filter(|&n| n > 0) {
            sig.rela_count = Some(n);
        }
    }
    let status_ert = status.is_some_and(|s| {
        let t = s.trim();
        t.eq_ignore_ascii_case("ert_cmd_error") || t.eq_ignore_ascii_case("ert_error")
    });
    let ert_error = ert_error_field.unwrap_or(false) || status_ert || sig.ert_error;
    let aiebu = aiebu_elf_field.or(sig.aiebu_elf);

    if !ert_error {
        if detail_nonempty.is_some() || status.is_some_and(|s| !s.trim().is_empty()) {
            return SiliconResidualClass::NotPassNoErtError;
        }
        return SiliconResidualClass::Unclassified;
    }

    match aiebu {
        Some(true) if last_apply_has_rela_evidence(&sig) => {
            SiliconResidualClass::AppLoadPdiFailAfterAiebu
        }
        Some(false) => SiliconResidualClass::AppLoadLikelyRawOrUnpatched,
        // aiebu true without rela evidence, or aiebu unknown
        Some(true) | None => SiliconResidualClass::ErtErrorUnknownBlob,
    }
}

/// Format a short research note for a classified residual (never C-ready / Pass).
///
/// `sig` gates claims: `userspace_elf_rela_ok` only with aiebu=true **and** rela
/// evidence; FW `code=` / `name=` only when `fw_status_code` is present in signals.
#[must_use]
pub fn format_silicon_residual_class_note(
    class: SiliconResidualClass,
    sig: Option<&LastApplyDetailSignals>,
) -> String {
    let mut parts = vec![
        format!("residual={}", class.as_str()),
        "not_c_ready=1".to_string(),
        "not_ran_on_npu=1".to_string(),
    ];
    match class {
        SiliconResidualClass::AppLoadPdiFailAfterAiebu => {
            parts.push("env_hitl_closed=1".to_string());
            if sig.is_some_and(|s| s.aiebu_elf == Some(true) && last_apply_has_rela_evidence(s)) {
                parts.push("userspace_elf_rela_ok=1".to_string());
            }
            if let Some(code) = sig.and_then(|s| s.fw_status_code) {
                parts.push(format!("code=0x{code:x}"));
                parts.push(format!("name={}", aie2_fw_status_name(code)));
            } else {
                parts.push("fw_code=unknown_check_journal".to_string());
            }
        }
        SiliconResidualClass::AppLoadLikelyRawOrUnpatched => {
            parts.push("likely_raw_or_unpatched=1".to_string());
            parts.push("prefer_aiebu_elf=1".to_string());
            if let Some(code) = sig.and_then(|s| s.fw_status_code) {
                parts.push(format!("code=0x{code:x}"));
                parts.push(format!("name={}", aie2_fw_status_name(code)));
            } else {
                parts.push("fw_code=heuristic_check_journal".to_string());
            }
        }
        SiliconResidualClass::ErtErrorUnknownBlob => {
            parts.push("ert_error=1".to_string());
            let aiebu = sig.and_then(|s| s.aiebu_elf);
            match aiebu {
                Some(true) => {
                    parts.push("aiebu=1".to_string());
                    parts.push("rela_evidence=0".to_string());
                }
                Some(false) => parts.push("aiebu=0".to_string()),
                None => parts.push("aiebu_unknown=1".to_string()),
            }
            parts.push("check_instr_path_or_journal=1".to_string());
            if let Some(code) = sig.and_then(|s| s.fw_status_code) {
                parts.push(format!("code=0x{code:x}"));
                parts.push(format!("name={}", aie2_fw_status_name(code)));
            }
        }
        SiliconResidualClass::NotPassNoErtError => {
            parts.push("no_ert_error_signal=1".to_string());
        }
        SiliconResidualClass::Unclassified => {}
    }
    parts.join(" ")
}

/// Resolve lab CREATE_HWCTX QoS from optional env strings (pure; no `std::env` read).
///
/// - `force_zero` (`XDNA2_HWCTX_QOS_ZERO=1`) → `(0, 0)` only research path.
/// - Unset / empty / invalid parse → defaults (100, 30).
/// - Explicit `0` for gops or fps **without** `force_zero` is clamped to default
///   (avoids live "Invalid QoS gops 0" from accidental env).
#[must_use]
pub fn resolve_lab_qos_gops_fps(
    force_zero: bool,
    gops_env: Option<&str>,
    fps_env: Option<&str>,
) -> (u32, u32) {
    if force_zero {
        return (0, 0);
    }
    let parse = |s: Option<&str>, default: u32| -> u32 {
        let Some(raw) = s.map(str::trim).filter(|t| !t.is_empty()) else {
            return default;
        };
        let v = raw
            .strip_prefix("0x")
            .or_else(|| raw.strip_prefix("0X"))
            .map_or_else(
                || raw.parse::<u32>().ok(),
                |hex| u32::from_str_radix(hex, 16).ok(),
            );
        match v {
            Some(0) | None => default, // 0 without force_zero → clamp
            Some(n) => n,
        }
    };
    (
        parse(gops_env, LAB_DEFAULT_QOS_GOPS),
        parse(fps_env, LAB_DEFAULT_QOS_FPS),
    )
}

/// Read process env into [`resolve_lab_qos_gops_fps`].
#[must_use]
pub fn lab_qos_gops_fps_from_env() -> (u32, u32) {
    let force_zero = std::env::var("XDNA2_HWCTX_QOS_ZERO")
        .is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true"));
    resolve_lab_qos_gops_fps(
        force_zero,
        std::env::var("XDNA2_HWCTX_QOS_GOPS").ok().as_deref(),
        std::env::var("XDNA2_HWCTX_QOS_FPS").ok().as_deref(),
    )
}

/// Known AIE2 firmware / mailbox codes seen on this lab host (from journal).
///
/// Includes APP status ([`AIE2_FW_APP_LOAD_PDI_FAIL`], [`AIE2_FW_INVALID_PARAM`])
/// and MSG_OP `0x106` (map_host). Names are **public status labels only** — not
/// a PDI/partition binary layout decoder (Track L).
#[must_use]
pub fn aie2_fw_status_name(code: u32) -> &'static str {
    match code {
        AIE2_FW_APP_LOAD_PDI_FAIL => "APP_LOAD_PDI_FAIL",
        AIE2_FW_INVALID_PARAM => "AIE2_STATUS_INVALID_PARAM",
        0x106 => "MSG_OP_MAP_HOST_BUFFER",
        _ => "unknown_fw_status",
    }
}

/// Heuristic FW status hint when journal is unavailable (detail / last_apply only).
///
/// Never claims Pass. `None` aiebu = blob never classified (not “confirmed raw”).
#[must_use]
pub fn fw_status_hint_for_lab(ert_error: bool, aiebu_elf: Option<bool>) -> &'static str {
    if !ert_error {
        return "n/a";
    }
    match aiebu_elf {
        Some(false) => "likely_0x3000003_APP_LOAD_PDI_FAIL_if_raw_or_unpatched",
        Some(true) => "ert_Error_after_aiebu_rela_check_journal_Status_0x",
        None => "ert_Error_unknown_blob_kind_check_instr_path_or_journal",
    }
}

/// Snapshot of fields appended to timed `detail` (batch diagnostics).
#[derive(Debug, Clone, Copy)]
pub struct LabBatchDiag {
    /// `num_tiles` passed to CREATE_HWCTX.
    pub tiles: u32,
    /// QoS gops used.
    pub qos_gops: u32,
    /// QoS fps used.
    pub qos_fps: u32,
    /// `.rela.dyn` entry count when ELF patched; `None` if N/A.
    pub rela_count: Option<usize>,
    /// `Some(true)` aiebu ELF, `Some(false)` raw/other blob, `None` no blob.
    pub aiebu_elf: Option<bool>,
    /// True when ERT header state was Error after WAIT.
    pub ert_error: bool,
    /// Bare layout token: `count4` / `xrt3` / `na` (no `layout=` prefix).
    pub layout: &'static str,
    /// `content` / `bo` / `na` for residual START_NPU size mode.
    pub instr_size_mode: &'static str,
    /// `overwrite` / `add` / `na` for residual rela patch mode.
    pub rela_mode: &'static str,
}

/// Map aiebu flag to a stable `blob_kind=` detail token (plan A1.4).
#[must_use]
pub fn blob_kind_token(aiebu_elf: Option<bool>) -> &'static str {
    match aiebu_elf {
        Some(true) => "aiebu_elf",
        Some(false) => "raw_or_non_elf",
        None => "unknown",
    }
}

/// Format cheap batch diagnostics for TimedOpResult.detail (no secrets).
#[must_use]
pub fn format_lab_batch_diag(d: LabBatchDiag) -> String {
    let rela = d.rela_count.map_or_else(
        || "rela_count=na".to_string(),
        |n| format!("rela_count={n}"),
    );
    let elf = match d.aiebu_elf {
        Some(true) => "aiebu_elf=1",
        Some(false) => "aiebu_elf=0",
        None => "aiebu_elf=na",
    };
    let fw = fw_status_hint_for_lab(d.ert_error, d.aiebu_elf);
    let layout = if d.layout.is_empty() { "na" } else { d.layout };
    let ism = if d.instr_size_mode.is_empty() {
        "na"
    } else {
        d.instr_size_mode
    };
    let rm = if d.rela_mode.is_empty() {
        "na"
    } else {
        d.rela_mode
    };
    let blob_kind = blob_kind_token(d.aiebu_elf);
    format!(
        "tiles={}; qos_gops={} qos_fps={}; {rela}; {elf}; blob_kind={blob_kind}; \
         layout={layout}; inst_size_mode={ism}; rela_mode={rm}; fw_status_hint={fw}",
        d.tiles, d.qos_gops, d.qos_fps
    )
}

/// Stamp batch diagnostics onto a timed result and optionally write last_apply.
///
/// Safe on every exit path after QoS/tiles are known. I/O errors never fail the op.
pub fn finalize_lab_timed_result(
    result: &mut TimedOpResult,
    batch: LabBatchDiag,
    kernel_args_len: usize,
    write_last_apply: bool,
) {
    let batch_s = format_lab_batch_diag(batch);
    if result.detail.is_empty() {
        result.detail = batch_s;
    } else if !result.detail.contains("qos_gops=") {
        result.detail = format!("{}; {batch_s}", result.detail);
    }
    if write_last_apply {
        write_last_apply_best_effort(result, batch, batch.layout, kernel_args_len);
    }
}

/// One offline contract check row (pure; for dry-run / unit tests).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractCheck {
    /// Stable id (`blob_is_aiebu_elf`, `kernel_args_count4_len`, …).
    pub id: String,
    /// Whether the check passed.
    pub ok: bool,
    /// Short human detail.
    pub detail: String,
}

/// Classify instruction bytes for contract reports (raw classic vs aiebu ELF).
///
/// `ok` requires aiebu ELF with **non-empty** `.rela.dyn` (EXEC parity).
#[must_use]
pub fn contract_classify_blob(bytes: &[u8]) -> ContractCheck {
    let kind = classify_instruction_blob(bytes);
    let is_elf = is_aiebu_elf_ctrltext(bytes);
    let rela_n = if is_elf {
        parse_elf32_rela_dyn(bytes).map_or(0, |r| r.len())
    } else {
        0
    };
    let ok_for_exec = is_elf && rela_n > 0;
    let detail = match &kind {
        InstructionBlobKind::Aie2ElfCtrltext {
            ctrltext_len,
            ctrltext,
            ..
        } => format!(
            "aiebu_elf ctrltext={ctrltext_len}B rela_count={rela_n} inner={}",
            ctrltext.summary()
        ),
        InstructionBlobKind::ClassicTxnV01 { header, ops } => format!(
            "raw_classic_txn size={} ops={} cols={} rows={} (EXEC often APP_LOAD_PDI_FAIL)",
            bytes.len(),
            ops.len(),
            header.num_cols,
            header.num_rows
        ),
        other => other.summary(),
    };
    ContractCheck {
        id: "blob_kind".into(),
        ok: ok_for_exec,
        detail,
    }
}

/// Pure layout lock: count4 vs xrt3 both yield 15 u32 kernel args; n_bos differs.
/// Also builds START_NPU envelope and asserts `prop_count == 0`.
#[must_use]
pub fn contract_kernel_arg_layouts(ifm: u64, wts: u64, ofm: u64, ctrl: u64) -> Vec<ContractCheck> {
    let count4 = pack_start_npu_kernel_args_with_ctrl_layout(
        ifm,
        wts,
        ofm,
        Some(ctrl),
        LabKernelArgLayout::Count4Ctrl,
    );
    let xrt3 = pack_start_npu_kernel_args_with_ctrl_layout(
        ifm,
        wts,
        ofm,
        Some(ctrl),
        LabKernelArgLayout::Xrt3,
    );
    // prop_count at byte 20 (header 4 B + cu_mask + buffer u64 + size). See
    // START_NPU_PROP_COUNT_BYTE_OFF / prop_count_from_start_npu_cmd_bo.
    let prop_ok = build_ert_start_npu_cmd_bo_with_kernel_args(0x1000, 1688, &count4, 0x1)
        .ok()
        .and_then(|bo| prop_count_from_start_npu_cmd_bo(&bo))
        .is_some_and(|pc| pc == 0);
    // Regression lock: legacy builder puts prop_count=len (non-zero) at the same offset.
    let legacy_nonzero = build_ert_start_npu_cmd_bo(0x1000, 1688, &count4, 0x1)
        .ok()
        .and_then(|bo| prop_count_from_start_npu_cmd_bo(&bo))
        .is_some_and(|pc| pc != 0);
    vec![
        ContractCheck {
            id: "kernel_args_count4_len".into(),
            ok: count4.len() == 15 && count4.first() == Some(&4),
            detail: format!(
                "count4 len={} n_bos={}",
                count4.len(),
                count4.first().copied().unwrap_or(0)
            ),
        },
        ContractCheck {
            id: "kernel_args_xrt3_len".into(),
            ok: xrt3.len() == 15 && xrt3.first() == Some(&3),
            detail: format!(
                "xrt3 len={} n_bos={}",
                xrt3.len(),
                xrt3.first().copied().unwrap_or(0)
            ),
        },
        ContractCheck {
            id: "kernel_args_prop_count_policy".into(),
            ok: prop_ok && legacy_nonzero,
            detail: if prop_ok && legacy_nonzero {
                format!(
                    "prop_count=0 at byte {START_NPU_PROP_COUNT_BYTE_OFF} (shim); \
                     legacy builder non-zero at same offset (paired)"
                )
            } else {
                format!(
                    "prop_count check failed (ok={prop_ok} legacy_nz={legacy_nonzero}) \
                     offset={START_NPU_PROP_COUNT_BYTE_OFF}"
                )
            },
        },
        ContractCheck {
            id: "kernel_args_count4_ctrl_slot".into(),
            ok: count4.get(11).copied() == Some((ctrl & 0xffff_ffff) as u32),
            detail: format!(
                "count4 ctrl_lo={:#x} (expect {:#x})",
                count4.get(11).copied().unwrap_or(0),
                (ctrl & 0xffff_ffff) as u32
            ),
        },
        ContractCheck {
            id: "kernel_args_xrt3_no_ctrl_slot".into(),
            ok: xrt3.get(11).copied() == Some(0),
            detail: "xrt3 slot after OFM is trailing zero (ctrl pin-only)".into(),
        },
        ContractCheck {
            id: "kernel_args_hi_dword".into(),
            ok: {
                let hi = pack_start_npu_kernel_args_with_ctrl_layout(
                    0x1111_0000_2222_0000,
                    0xb,
                    0xc,
                    None,
                    LabKernelArgLayout::Count4Ctrl,
                );
                hi.get(5).copied() == Some(0x2222_0000) && hi.get(6).copied() == Some(0x1111_0000)
            },
            detail: "u64 paddr splits into lo/hi dwords".into(),
        },
    ]
}

/// Pure instr-size dual-mode lock (content vs bo).
#[must_use]
pub fn contract_instr_size_modes() -> Vec<ContractCheck> {
    let content = resolve_start_npu_inst_size(1688, 4096, LabInstrSizeMode::Content);
    let bo = resolve_start_npu_inst_size(1688, 4096, LabInstrSizeMode::Bo);
    let zero_content = resolve_start_npu_inst_size(0, 4096, LabInstrSizeMode::Content);
    let default_mode = lab_instr_size_mode_from_str(None);
    vec![
        ContractCheck {
            id: "instr_size_mode_content".into(),
            ok: content == 1688,
            detail: format!("content mode → {content} (expect 1688)"),
        },
        ContractCheck {
            id: "instr_size_mode_bo".into(),
            ok: bo == 4096,
            detail: format!("bo mode → {bo} (expect 4096)"),
        },
        ContractCheck {
            id: "instr_size_mode_zero_content_fallback".into(),
            ok: zero_content == 4096,
            detail: format!("content+0 content_len falls back to bo → {zero_content}"),
        },
        ContractCheck {
            id: "instr_size_mode_default".into(),
            ok: default_mode == LabInstrSizeMode::Content,
            detail: format!(
                "default mode={} (content preferred after bo=4096 APP_LOAD_PDI residual)",
                default_mode.as_str()
            ),
        },
    ]
}

/// Dynsym name/size report for aiebu ELF (informational).
///
/// **Policy:** `ok = true` when parse succeeds — including empty dynsym (no
/// `.dynsym` section). Emptiness is only in `detail`. Hard EXEC gate remains
/// `blob_kind` (aiebu + non-empty `.rela.dyn`), not dynsym names.
#[must_use]
pub fn contract_dynsym_report(elf: &[u8]) -> ContractCheck {
    match parse_elf32_dynsym(elf) {
        Ok(syms) => {
            let detail = format_dynsym_report(&syms);
            ContractCheck {
                id: "dynsym_report".into(),
                ok: true,
                detail,
            }
        }
        Err(e) => ContractCheck {
            id: "dynsym_report".into(),
            ok: false,
            detail: format!("dynsym parse failed: {e}"),
        },
    }
}

/// Patch + classic v0.1 walk for one [`LabRelaMode`].
#[must_use]
pub fn contract_post_patch_txn_mode(
    elf: &[u8],
    ifm: u64,
    wts: u64,
    ofm: u64,
    ctrl: u64,
    mode: LabRelaMode,
) -> ContractCheck {
    let id = match mode {
        LabRelaMode::Overwrite => "post_patch_txn_walk",
        LabRelaMode::Add => "post_patch_txn_walk_add",
    };
    let patched = match patch_aiebu_elf_ctrltext_with_bos_ex(elf, ifm, wts, ofm, ctrl, mode) {
        Ok(p) => p,
        Err(e) => {
            return ContractCheck {
                id: id.into(),
                ok: false,
                detail: format!("rela patch ({}) failed: {e}", mode.as_str()),
            };
        }
    };
    // Prefer staged expect when length is 1688 (mladf lab).
    let expect_staged = patched.len() == 1688;
    match validate_patched_ctrltext_classic(&patched, expect_staged) {
        Ok(InstructionBlobKind::ClassicTxnV01 { header, ops }) => ContractCheck {
            id: id.into(),
            ok: true,
            detail: format!(
                "ClassicTxnV01 txn_size={} num_ops={} ops_walked={} (rela {})",
                header.txn_size,
                header.num_ops,
                ops.len(),
                mode.as_str()
            ),
        },
        Ok(other) => ContractCheck {
            id: id.into(),
            ok: false,
            detail: format!("unexpected kind after validate: {}", other.summary()),
        },
        Err(e) => ContractCheck {
            id: id.into(),
            ok: false,
            detail: format!("post-patch walk refuse: {e}"),
        },
    }
}

/// Patch + classic v0.1 walk (default overwrite).
#[must_use]
pub fn contract_post_patch_txn(
    elf: &[u8],
    ifm: u64,
    wts: u64,
    ofm: u64,
    ctrl: u64,
) -> ContractCheck {
    contract_post_patch_txn_mode(elf, ifm, wts, ofm, ctrl, LabRelaMode::Overwrite)
}

/// Aggregate offline contract report (A3/A4 pure entry).
#[must_use]
pub fn contract_report(
    instr_bytes: Option<&[u8]>,
    ifm: u64,
    wts: u64,
    ofm: u64,
    ctrl: u64,
) -> Vec<ContractCheck> {
    let mut out = Vec::new();
    if let Some(b) = instr_bytes {
        out.push(contract_classify_blob(b));
        if is_aiebu_elf_ctrltext(b) {
            out.push(contract_dynsym_report(b));
            out.push(contract_post_patch_txn(b, ifm, wts, ofm, ctrl));
            // Residual trial: add-mode still must walk as ClassicTxnV01 offline.
            out.push(contract_post_patch_txn_mode(
                b,
                ifm,
                wts,
                ofm,
                ctrl,
                LabRelaMode::Add,
            ));
        }
    } else {
        out.push(ContractCheck {
            id: "blob_kind".into(),
            ok: false,
            detail: "no instruction bytes".into(),
        });
    }
    out.extend(contract_kernel_arg_layouts(ifm, wts, ofm, ctrl));
    out.extend(contract_instr_size_modes());
    out
}

/// Whether a last_apply path is allowed (under temp_dir or repo testdata/xdna2_cmd).
///
/// **Confinement:** resolves intermediate directory symlinks via
/// `canonicalize(parent)` before the root check. Absolute lexical
/// `starts_with(temp_dir)` alone is **not** trusted (mid-path symlink bypass).
///
/// Missing parent for an absolute path → **refuse** (fail closed). Relative
/// paths with a missing parent fall back to a lexical root check only (write
/// staging of not-yet-created names under known roots).
#[must_use]
pub fn last_apply_path_is_allowed(path: &std::path::Path) -> bool {
    if path.as_os_str().is_empty() {
        return false;
    }
    // Refuse obvious traversal tokens before canonicalize.
    if path
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return false;
    }
    if !path
        .file_name()
        .is_some_and(|n| n.to_string_lossy().ends_with(".json"))
    {
        return false;
    }
    let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) else {
        return path_under_allowed_roots(path);
    };
    // Resolve intermediate dir symlinks: /tmp/evil_mid → /outside must fail.
    if let Ok(canon_parent) = std::fs::canonicalize(parent) {
        let candidate = canon_parent.join(path.file_name().unwrap_or_default());
        path_under_allowed_roots(&candidate)
    } else {
        // Absolute + missing parent: cannot prove confinement → refuse.
        if path.is_absolute() {
            return false;
        }
        // Relative / not-yet-created parent: lexical only (write path).
        path_under_allowed_roots(path)
    }
}

/// Soft-load path: write roots + refuse final-component symlink + re-check
/// resolved real path under allowed roots when the file exists.
///
/// Residual TOCTOU: metadata then open; classify example uses `O_NOFOLLOW` on
/// Unix when available as defense-in-depth.
#[must_use]
pub fn last_apply_load_path_is_allowed(path: &std::path::Path) -> bool {
    if !last_apply_path_is_allowed(path) {
        return false;
    }
    if let Ok(meta) = std::fs::symlink_metadata(path) {
        if meta.file_type().is_symlink() {
            return false;
        }
    }
    // If file exists, resolve fully (intermediate + final) and re-check roots.
    if path.exists() {
        if let Ok(canon) = std::fs::canonicalize(path) {
            if !path_under_allowed_roots(&canon) {
                return false;
            }
        }
    }
    true
}

/// True when `p` is empty or filesystem root only (`/` / RootDir components).
fn path_is_filesystem_root(p: &std::path::Path) -> bool {
    p.as_os_str().is_empty()
        || p == std::path::Path::new("/")
        || p.components()
            .all(|c| matches!(c, std::path::Component::RootDir))
}

/// Temp root used for last_apply path confinement (Unix `TMPDIR` / `temp_dir`).
///
/// **Security:** `TMPDIR=/` makes `std::env::temp_dir() == "/"` and
/// `Path::starts_with("/")` true for **every** absolute path. Also: if
/// `TMPDIR` is a **symlink to `/`**, lexical path is not RootDir-only but
/// canonicalize resolves to `/` — same bypass. Clamp empty and filesystem-root
/// (lexical **and** after canonicalize) to `/tmp` (same policy as
/// `tools/c4_pass_check.sh` and classify/c3 shell helpers).
#[must_use]
pub fn last_apply_temp_root() -> std::path::PathBuf {
    let temp = std::env::temp_dir();
    let clamped = if path_is_filesystem_root(&temp) {
        std::path::PathBuf::from("/tmp")
    } else {
        temp
    };
    // Resolve symlinks (e.g. TMPDIR → symlink-to-/); re-clamp if still root.
    let mut root = std::fs::canonicalize(&clamped).unwrap_or(clamped);
    if path_is_filesystem_root(&root) {
        root = std::fs::canonicalize("/tmp").unwrap_or_else(|_| std::path::PathBuf::from("/tmp"));
    }
    root
}

fn path_under_allowed_roots(path: &std::path::Path) -> bool {
    // Component-aware: refuse `/tmp_evil/...` when temp is `/tmp` (string prefix footgun).
    // Never treat filesystem root as the temp allow-root (TMPDIR=/ clamp).
    let temp_root = last_apply_temp_root();
    let lab = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/xdna2_cmd");
    // Prefer resolved form of lab root when available.
    let lab_root = std::fs::canonicalize(&lab).unwrap_or(lab);
    let under = path.starts_with(&temp_root) || path.starts_with(&lab_root);
    under
        && path
            .file_name()
            .is_some_and(|n| n.to_string_lossy().ends_with(".json"))
}

/// Default path for last-apply snapshot (confined; operator-local).
#[must_use]
pub fn default_last_apply_path() -> std::path::PathBuf {
    if let Ok(p) = std::env::var("XDNA2_LAB_LAST_APPLY_JSON") {
        let t = p.trim();
        if !t.is_empty() {
            let pb = std::path::PathBuf::from(t);
            if last_apply_path_is_allowed(&pb) {
                return pb;
            }
            // Reject unsafe env path; fall through to safe default.
        }
    }
    let lab =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/xdna2_cmd/last_apply.json");
    if lab.parent().is_some_and(std::path::Path::is_dir) {
        return lab;
    }
    // Private per-user temp name (avoid fixed world-writable /tmp symlink plant).
    // Use clamped temp root so TMPDIR=/ does not plant under filesystem root.
    last_apply_temp_root().join(format!("dragon_npu_last_apply_{}.json", std::process::id()))
}

/// True when `a` is a usable device paddr (not 0 / not `u64::MAX`).
///
/// Mirrors [`super::direct::is_valid_xdna_device_addr`] for pure JSON/detail paths
/// without pulling live ioctl types into vendor parse.
#[must_use]
pub const fn is_valid_device_paddr(a: u64) -> bool {
    a != 0 && a != u64::MAX
}

/// Map a parsed numeric paddr to `Some` only when valid; `0` / `u64::MAX` → `None`.
#[must_use]
const fn some_valid_device_paddr(a: u64) -> Option<u64> {
    if is_valid_device_paddr(a) {
        Some(a)
    } else {
        None
    }
}

/// Parsed vendor / GT paddr JSON (schema v1) for fair pure-direct re-patch.
///
/// Missing, null, `0`, or `u64::MAX` paddrs stay / become `None`.
/// `paddrs_complete` is true only when ifm/wts/ofm are all **valid** device addrs
/// (ctrl optional; invalid ctrl is treated as absent → 0 in [`VendorPaddrs::as_io_tuple`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VendorPaddrs {
    /// Schema version when present (expect 1).
    pub schema_version: Option<u64>,
    /// `registry_get_instr_bo` / `txn_bin_fallback_unpatched` / …
    pub capture_mode: Option<String>,
    /// Source label (`capture_mladf_instr_vendor`, `last_apply_extract`, …).
    pub source: Option<String>,
    /// IFM buffer device/host paddr when known.
    pub ifm_paddr: Option<u64>,
    /// WTS buffer paddr when known.
    pub wts_paddr: Option<u64>,
    /// OFM buffer paddr when known.
    pub ofm_paddr: Option<u64>,
    /// CTRL packet BO paddr when known (optional for rela complete).
    pub ctrl_paddr: Option<u64>,
}

impl VendorPaddrs {
    /// True when ifm/wts/ofm are all known **and** valid (not 0 / `u64::MAX`).
    ///
    /// Ctrl may still be None or invalid; it does not affect completeness.
    #[must_use]
    pub fn paddrs_complete(&self) -> bool {
        matches!(
            (self.ifm_paddr, self.wts_paddr, self.ofm_paddr),
            (Some(i), Some(w), Some(o))
                if is_valid_device_paddr(i)
                    && is_valid_device_paddr(w)
                    && is_valid_device_paddr(o)
        )
    }

    /// Tuple for re-patch when ifm/wts/ofm are valid; ctrl defaults to 0 if
    /// absent or invalid (`0` / `u64::MAX`).
    #[must_use]
    pub fn as_io_tuple(&self) -> Option<(u64, u64, u64, u64)> {
        let i = self.ifm_paddr.filter(|&a| is_valid_device_paddr(a))?;
        let w = self.wts_paddr.filter(|&a| is_valid_device_paddr(a))?;
        let o = self.ofm_paddr.filter(|&a| is_valid_device_paddr(a))?;
        let c = self
            .ctrl_paddr
            .filter(|&a| is_valid_device_paddr(a))
            .unwrap_or(0);
        Some((i, w, o, c))
    }
}

/// Parse a single paddr JSON value: null → None; `"0x…"` / decimal string / u64.
///
/// After a successful numeric parse, **`0` and `u64::MAX` are treated as
/// unknown/invalid device addresses** and return `Ok(None)` (same policy as
/// [`is_valid_device_paddr`] / live `is_valid_xdna_device_addr`).
///
/// # Errors
///
/// Returns `Err` only for non-null values that cannot be parsed as u64
/// (including negatives).
pub fn parse_paddr_json_value(v: &serde_json::Value) -> Result<Option<u64>, String> {
    if v.is_null() {
        return Ok(None);
    }
    if let Some(s) = v.as_str() {
        let s = s.trim();
        if s.is_empty() || s.eq_ignore_ascii_case("null") {
            return Ok(None);
        }
        let hex = s
            .strip_prefix("0x")
            .or_else(|| s.strip_prefix("0X"))
            .unwrap_or(s);
        if let Ok(n) = u64::from_str_radix(hex, 16) {
            // Ambiguous pure-decimal strings without 0x: prefer decimal if all digits.
            if !s.starts_with("0x") && !s.starts_with("0X") && s.chars().all(|c| c.is_ascii_digit())
            {
                return s
                    .parse::<u64>()
                    .map(some_valid_device_paddr)
                    .map_err(|e| format!("paddr decimal parse: {e}"));
            }
            return Ok(some_valid_device_paddr(n));
        }
        return s
            .parse::<u64>()
            .map(some_valid_device_paddr)
            .map_err(|e| format!("paddr parse {s:?}: {e}"));
    }
    if let Some(n) = v.as_u64() {
        return Ok(some_valid_device_paddr(n));
    }
    if let Some(n) = v.as_i64() {
        if n < 0 {
            return Err(format!("paddr negative: {n}"));
        }
        return Ok(some_valid_device_paddr(n.cast_unsigned()));
    }
    Err(format!("paddr value not string/number/null: {v}"))
}

/// Parse `vendor_paddrs.json` (or compatible) from a JSON object text.
///
/// Accepts schema v1 fields; unknown extra fields ignored. Null paddrs OK.
///
/// # Errors
///
/// Invalid JSON or unparseable non-null paddr field.
pub fn parse_vendor_paddrs_json(text: &str) -> Result<VendorPaddrs, String> {
    let v: serde_json::Value =
        serde_json::from_str(text).map_err(|e| format!("vendor_paddrs JSON: {e}"))?;
    let obj = v
        .as_object()
        .ok_or_else(|| "vendor_paddrs root must be object".to_string())?;
    let schema_version = obj
        .get("schema_version")
        .and_then(serde_json::Value::as_u64);
    let capture_mode = obj
        .get("capture_mode")
        .and_then(|x| x.as_str())
        .map(str::to_string);
    let source = obj
        .get("source")
        .and_then(|x| x.as_str())
        .map(str::to_string);
    let get = |k: &str| -> Result<Option<u64>, String> {
        obj.get(k).map_or(Ok(None), |x| {
            parse_paddr_json_value(x).map_err(|e| format!("{k}: {e}"))
        })
    };
    Ok(VendorPaddrs {
        schema_version,
        capture_mode,
        source,
        ifm_paddr: get("ifm_paddr")?,
        wts_paddr: get("wts_paddr")?,
        ofm_paddr: get("ofm_paddr")?,
        ctrl_paddr: get("ctrl_paddr")?,
    })
}

/// Extract `io_paddrs ifm=… wts=… ofm=… ctrl=…` from a timed `detail` string.
///
/// Best-effort pure parse for last_apply → vendor_paddrs conversion.
/// Returns `None` when ifm/wts/ofm is missing, unparseable, `0`, or `u64::MAX`
/// (invalid device addresses — same policy as [`is_valid_device_paddr`]).
/// Ctrl may be 0 (optional pin).
#[must_use]
pub fn parse_io_paddrs_from_detail(detail: &str) -> Option<(u64, u64, u64, u64)> {
    // Match: io_paddrs ifm=0x… wts=0x… ofm=0x… ctrl=0x…
    let key = "io_paddrs ifm=";
    let start = detail.find(key)?;
    let rest = &detail[start + key.len()..];
    let mut parts = rest.split_whitespace();
    let ifm_s = parts.next()?;
    let wts_tok = parts.next()?; // "wts=0x…"
    let ofm_tok = parts.next()?;
    let ctrl_tok = parts.next()?;
    let parse_hex = |s: &str| -> Option<u64> {
        let s = s.trim().trim_end_matches([',', ';']);
        let s = s
            .strip_prefix("0x")
            .or_else(|| s.strip_prefix("0X"))
            .unwrap_or(s);
        u64::from_str_radix(s, 16).ok()
    };
    let ifm = parse_hex(ifm_s)?;
    let wts = parse_hex(wts_tok.strip_prefix("wts=")?)?;
    let ofm = parse_hex(ofm_tok.strip_prefix("ofm=")?)?;
    let ctrl = parse_hex(ctrl_tok.strip_prefix("ctrl=")?)?;
    if !is_valid_device_paddr(ifm) || !is_valid_device_paddr(wts) || !is_valid_device_paddr(ofm) {
        return None;
    }
    Some((ifm, wts, ofm, ctrl))
}

/// Build vendor_paddrs-compatible JSON value from last_apply detail paddrs.
#[must_use]
pub fn vendor_paddrs_json_from_io(
    ifm: u64,
    wts: u64,
    ofm: u64,
    ctrl: u64,
    source: &str,
) -> serde_json::Value {
    serde_json::json!({
        "schema_version": 1,
        "source": source,
        "capture_mode": "last_apply_detail",
        "ifm_paddr": format!("{ifm:#x}"),
        "wts_paddr": format!("{wts:#x}"),
        "ofm_paddr": format!("{ofm:#x}"),
        "ctrl_paddr": format!("{ctrl:#x}"),
        "paddrs_known": true,
        "note": "Extracted from last_apply detail io_paddrs; pure-direct re-patch only"
    })
}

/// Best-effort write of last apply snapshot. Never panics; ignores I/O errors.
pub fn write_last_apply_best_effort(
    result: &TimedOpResult,
    batch: LabBatchDiag,
    layout: &str,
    kernel_args_len: usize,
) {
    write_last_apply_to_path(result, batch, layout, kernel_args_len, None);
}

/// Same as [`write_last_apply_best_effort`] with explicit path (tests / tools).
///
/// Refuses symlink targets and paths outside temp_dir / testdata/xdna2_cmd.
/// Writes mode `0o600` when possible. Does not create arbitrary parent trees for env paths.
pub fn write_last_apply_to_path(
    result: &TimedOpResult,
    batch: LabBatchDiag,
    layout: &str,
    kernel_args_len: usize,
    path_override: Option<&std::path::Path>,
) {
    let owned = path_override.map_or_else(default_last_apply_path, std::path::Path::to_path_buf);
    if !last_apply_path_is_allowed(&owned) {
        return;
    }
    // Refuse if path is a symlink (TOCTOU residual accepted for lab tool).
    if let Ok(meta) = std::fs::symlink_metadata(&owned) {
        if meta.file_type().is_symlink() {
            return;
        }
    }
    let layout_bare = layout.strip_prefix("layout=").unwrap_or(layout);
    let body = serde_json::json!({
        "schema": 2,
        "honesty": "≠ ran_on_npu unless result.ran_on_npu true; offline snapshot only; operator-private",
        "op": result.op,
        "status": result.status,
        "ran_on_npu": result.ran_on_npu,
        "real_ioctl": result.real_ioctl,
        "m": result.m,
        "n": result.n,
        "k": result.k,
        "dtype": result.dtype,
        "tiles": batch.tiles,
        "qos_gops": batch.qos_gops,
        "qos_fps": batch.qos_fps,
        "rela_count": batch.rela_count,
        "aiebu_elf": batch.aiebu_elf,
        "ert_error": batch.ert_error,
        "fw_status_hint": fw_status_hint_for_lab(batch.ert_error, batch.aiebu_elf),
        "layout": layout_bare,
        "kernel_args_len": kernel_args_len,
        // Detail may include addresses — operator-private; mode 0600.
        "detail": result.detail,
        "total_s": result.total_s,
    });
    // Only create parent if it is the known lab or temp dir (not arbitrary depth).
    // Use clamped temp root so TMPDIR=/ does not allow create_dir_all everywhere.
    if let Some(parent) = owned.parent() {
        if parent.is_dir()
            || parent.ends_with("xdna2_cmd")
            || parent.starts_with(last_apply_temp_root())
        {
            let _ = std::fs::create_dir_all(parent);
        }
    }
    let Ok(s) = serde_json::to_string_pretty(&body) else {
        return;
    };
    // Write via temp file in same dir then rename when possible.
    let tmp = owned.with_extension("json.tmp");
    if std::fs::write(&tmp, s.as_bytes()).is_ok() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600));
        }
        if std::fs::rename(&tmp, &owned).is_err() {
            let _ = std::fs::write(&owned, s.as_bytes());
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&owned, std::fs::Permissions::from_mode(0o600));
            }
            let _ = std::fs::remove_file(&tmp);
        }
    }
}

/// Build a minimal ELF32 LE with `.ctrltext` and optional `.rela.dyn` for unit tests.
///
/// Not a full aiebu image — only enough for classify/patch refuse paths.
#[must_use]
#[allow(clippy::cast_possible_truncation)] // test fixture sizes are tiny (<4 KiB)
pub fn build_minimal_test_elf32(ctrltext: &[u8], relas: &[(u32, u32, i32)]) -> Vec<u8> {
    // ELF32 LE header (52 bytes) + 3 section headers (null, .ctrltext, .rela.dyn, .shstrtab)
    // Layout:
    //  [0,52) ehdr
    //  [52, 52+ctrltext) .ctrltext data
    //  rela table
    //  shstrtab
    //  section headers
    let mut out = vec![0_u8; 52];
    out[0..4].copy_from_slice(&[0x7f, b'E', b'L', b'F']);
    out[4] = 1; // ELFCLASS32
    out[5] = 1; // ELFDATA2LSB
    out[6] = 1; // EV_CURRENT
    out[16] = 1; // ET_REL
    out[18] = 3; // EM_386 placeholder
    out[20] = 1; // EV_CURRENT
    // e_shoff, e_shentsize=40, e_shnum, e_shstrndx filled later

    let ctrl_off = 52_u32;
    out.extend_from_slice(ctrltext);
    let rela_off = out.len() as u32;
    for &(offset, info, addend) in relas {
        out.extend_from_slice(&offset.to_le_bytes());
        out.extend_from_slice(&info.to_le_bytes());
        out.extend_from_slice(&addend.to_le_bytes());
    }
    let rela_size = (relas.len() * 12) as u32;
    let str_off = out.len() as u32;
    // shstrtab: \0 .ctrltext\0 .rela.dyn\0 .shstrtab\0
    let mut strtab = vec![0_u8];
    let name_ctrl = strtab.len() as u32;
    strtab.extend_from_slice(b".ctrltext\0");
    let name_rela = strtab.len() as u32;
    strtab.extend_from_slice(b".rela.dyn\0");
    let name_str = strtab.len() as u32;
    strtab.extend_from_slice(b".shstrtab\0");
    out.extend_from_slice(&strtab);
    let str_size = strtab.len() as u32;

    let sh_off = out.len() as u32;
    // null section
    out.extend_from_slice(&[0_u8; 40]);
    // .ctrltext SHT_PROGBITS=1
    write_shdr(&mut out, name_ctrl, 1, ctrl_off, ctrltext.len() as u32);
    // .rela.dyn SHT_RELA=4
    write_shdr(&mut out, name_rela, 4, rela_off, rela_size);
    // .shstrtab SHT_STRTAB=3
    write_shdr(&mut out, name_str, 3, str_off, str_size);

    // e_shoff @ 32, e_shentsize @ 46, e_shnum @ 48, e_shstrndx @ 50
    out[32..36].copy_from_slice(&sh_off.to_le_bytes());
    out[46..48].copy_from_slice(&40_u16.to_le_bytes());
    out[48..50].copy_from_slice(&4_u16.to_le_bytes());
    out[50..52].copy_from_slice(&3_u16.to_le_bytes()); // shstrndx = 3
    out
}

fn write_shdr(out: &mut Vec<u8>, name: u32, sh_type: u32, offset: u32, size: u32) {
    let mut h = [0_u8; 40];
    h[0..4].copy_from_slice(&name.to_le_bytes());
    h[4..8].copy_from_slice(&sh_type.to_le_bytes());
    h[16..20].copy_from_slice(&offset.to_le_bytes());
    h[20..24].copy_from_slice(&size.to_le_bytes());
    out.extend_from_slice(&h);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::amd_xdna2::txn::{
        build_strix_research_noop_txn, patch_aiebu_elf_ctrltext_with_bos,
    };
    use std::path::{Path, PathBuf};
    use std::sync::{Mutex, OnceLock};

    /// Serialize tests that mutate process `TMPDIR` (env is global).
    fn tmpdir_env_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|e| e.into_inner())
    }

    /// RAII restore of `TMPDIR` after test mutation (panic-safe).
    struct TmpdirEnvGuard {
        prev: Option<std::ffi::OsString>,
        _lock: std::sync::MutexGuard<'static, ()>,
    }

    impl TmpdirEnvGuard {
        #[allow(unsafe_code)] // test-only env isolation
        fn set(value: impl AsRef<std::ffi::OsStr>) -> Self {
            let _lock = tmpdir_env_lock();
            let prev = std::env::var_os("TMPDIR");
            // SAFETY: exclusive under tmpdir_env_lock; Drop restores.
            unsafe {
                std::env::set_var("TMPDIR", value);
            }
            Self { prev, _lock }
        }
    }

    impl Drop for TmpdirEnvGuard {
        #[allow(unsafe_code)] // test-only env isolation
        fn drop(&mut self) {
            unsafe {
                match &self.prev {
                    Some(v) => std::env::set_var("TMPDIR", v),
                    None => std::env::remove_var("TMPDIR"),
                }
            }
        }
    }

    #[test]
    fn resolve_lab_qos_defaults_and_clamp_zero() {
        assert_eq!(resolve_lab_qos_gops_fps(false, None, None), (100, 30));
        assert_eq!(
            resolve_lab_qos_gops_fps(false, Some(""), Some("")),
            (100, 30)
        );
        assert_eq!(
            resolve_lab_qos_gops_fps(false, Some("0"), Some("0")),
            (100, 30),
            "env 0 without QOS_ZERO clamps"
        );
        assert_eq!(resolve_lab_qos_gops_fps(true, Some("0"), Some("0")), (0, 0));
        assert_eq!(
            resolve_lab_qos_gops_fps(false, Some("50"), Some("15")),
            (50, 15)
        );
        assert_eq!(
            resolve_lab_qos_gops_fps(false, Some("0"), Some("15")),
            (100, 15),
            "gops=0 alone clamps gops"
        );
    }

    #[test]
    fn fw_status_names_known() {
        assert_eq!(aie2_fw_status_name(0x3_000_003), "APP_LOAD_PDI_FAIL");
        assert_eq!(
            aie2_fw_status_name(0x4_000_003),
            "AIE2_STATUS_INVALID_PARAM"
        );
        assert_eq!(aie2_fw_status_name(0x106), "MSG_OP_MAP_HOST_BUFFER");
        assert_eq!(aie2_fw_status_name(1), "unknown_fw_status");
    }

    #[test]
    fn fw_hint_raw_vs_elf() {
        assert_eq!(fw_status_hint_for_lab(false, Some(true)), "n/a");
        assert!(fw_status_hint_for_lab(true, Some(false)).contains("0x3000003"));
        assert!(fw_status_hint_for_lab(true, Some(true)).contains("aiebu_rela"));
        assert!(
            fw_status_hint_for_lab(true, None).contains("unknown_blob"),
            "None is not raw"
        );
    }

    #[test]
    fn batch_diag_format_variants() {
        let full = format_lab_batch_diag(LabBatchDiag {
            tiles: 32,
            qos_gops: LAB_DEFAULT_QOS_GOPS,
            qos_fps: LAB_DEFAULT_QOS_FPS,
            rela_count: Some(12),
            aiebu_elf: Some(true),
            ert_error: true,
            layout: "count4",
            instr_size_mode: "content",
            rela_mode: "overwrite",
        });
        assert!(full.contains("tiles=32"));
        assert!(full.contains("qos_gops=100"));
        assert!(full.contains("rela_count=12"));
        assert!(full.contains("layout=count4"));
        assert!(full.contains("inst_size_mode=content"));
        assert!(full.contains("rela_mode=overwrite"));
        let sparse = format_lab_batch_diag(LabBatchDiag {
            tiles: 4,
            qos_gops: 100,
            qos_fps: 30,
            rela_count: None,
            aiebu_elf: None,
            ert_error: false,
            layout: "na",
            instr_size_mode: "content",
            rela_mode: "overwrite",
        });
        assert!(sparse.contains("rela_count=na"));
        assert!(sparse.contains("aiebu_elf=na"));
        assert!(sparse.contains("fw_status_hint=n/a"));
        let raw = format_lab_batch_diag(LabBatchDiag {
            tiles: 4,
            qos_gops: 100,
            qos_fps: 30,
            rela_count: None,
            aiebu_elf: Some(false),
            ert_error: true,
            layout: "na",
            instr_size_mode: "content",
            rela_mode: "overwrite",
        });
        assert!(raw.contains("aiebu_elf=0"));
        assert!(raw.contains("0x3000003"));
    }

    #[test]
    fn contract_classify_raw_classic_txn() {
        let txn = build_strix_research_noop_txn().expect("noop");
        let c = contract_classify_blob(&txn);
        assert!(!c.ok, "raw txn must not be green for EXEC contract");
    }

    #[test]
    fn contract_classify_empty_and_garbage() {
        assert!(!contract_classify_blob(&[]).ok);
        assert!(!contract_classify_blob(&[0xde, 0xad, 0xbe, 0xef]).ok);
        let fake = b"\x7fELF\x01\x01";
        assert!(!contract_classify_blob(fake).ok);
    }

    #[test]
    fn prop_count_offset_is_20_not_28() {
        let kargs = pack_start_npu_kernel_args_with_ctrl_layout(
            0xa,
            0xb,
            0xc,
            Some(0xd),
            LabKernelArgLayout::Count4Ctrl,
        );
        let bo = build_ert_start_npu_cmd_bo_with_kernel_args(0x1000, 1688, &kargs, 0x1)
            .expect("envelope");
        assert_eq!(
            prop_count_from_start_npu_cmd_bo(&bo),
            Some(0),
            "kernel-args path prop_count"
        );
        // Offset 28 is n_bos hi (always 0 for n_bos=4) — must NOT be used as prop_count.
        let at_28 = u32::from_le_bytes(bo[28..32].try_into().unwrap());
        assert_eq!(
            at_28, 0,
            "n_bos hi at 28 is 0 (false green if used as prop_count)"
        );
        // First kernel dword (n_bos lo) at 24.
        assert_eq!(u32::from_le_bytes(bo[24..28].try_into().unwrap()), 4);
        // Legacy treats all as props → prop_count at 20 is non-zero.
        let legacy = build_ert_start_npu_cmd_bo(0x1000, 1688, &kargs, 0x1).expect("legacy");
        let leg_pc = prop_count_from_start_npu_cmd_bo(&legacy).expect("leg");
        assert_eq!(leg_pc, kargs.len() as u32);
        assert_ne!(leg_pc, 0);
        // Wrong-offset check would still see 0 on legacy (n_bos hi) — prove the trap.
        let legacy_at_28 = u32::from_le_bytes(legacy[28..32].try_into().unwrap());
        assert_eq!(
            legacy_at_28, 0,
            "legacy n_bos hi still 0 at 28 — wrong offset would false-green"
        );
    }

    #[test]
    fn contract_kernel_arg_layouts_lock() {
        let checks = contract_kernel_arg_layouts(0xa, 0xb, 0xc, 0xd);
        assert!(checks.iter().all(|c| c.ok), "{checks:?}");
        let prop = checks
            .iter()
            .find(|c| c.id == "kernel_args_prop_count_policy")
            .expect("prop");
        assert!(prop.ok && prop.detail.contains("prop_count=0"));
        assert!(prop.detail.contains("byte 20") || prop.detail.contains("20"));
    }

    #[test]
    fn contract_report_aggregates() {
        let txn = build_strix_research_noop_txn().expect("noop");
        let r = contract_report(Some(&txn), 1, 2, 3, 4);
        assert!(r.iter().any(|c| c.id == "blob_kind" && !c.ok));
        assert!(
            r.iter()
                .any(|c| c.id == "kernel_args_prop_count_policy" && c.ok)
        );
        assert!(r.iter().any(|c| c.id == "instr_size_mode_content" && c.ok));
        assert!(r.iter().any(|c| c.id == "instr_size_mode_bo" && c.ok));
        assert!(r.iter().any(|c| c.id == "instr_size_mode_default" && c.ok));
        // Raw txn has no dynsym / post-patch rows.
        assert!(!r.iter().any(|c| c.id == "dynsym_report"));
        assert!(!r.iter().any(|c| c.id == "post_patch_txn_walk"));
    }

    #[test]
    fn instr_size_mode_pure_resolution() {
        assert_eq!(
            resolve_start_npu_inst_size(1688, 4096, LabInstrSizeMode::Content),
            1688
        );
        assert_eq!(
            resolve_start_npu_inst_size(1688, 4096, LabInstrSizeMode::Bo),
            4096
        );
        assert_eq!(
            resolve_start_npu_inst_size(9999, 4096, LabInstrSizeMode::Bo),
            4096,
            "bo ignores content"
        );
        assert_eq!(
            resolve_start_npu_inst_size(5000, 4096, LabInstrSizeMode::Content),
            4096,
            "content clamps to bo"
        );
        assert!(instr_size_content_clamped(
            5000,
            4096,
            LabInstrSizeMode::Content
        ));
        assert!(!instr_size_content_clamped(
            1688,
            4096,
            LabInstrSizeMode::Content
        ));
        assert!(!instr_size_content_clamped(
            5000,
            4096,
            LabInstrSizeMode::Bo
        ));
        assert_eq!(
            resolve_start_npu_inst_size(0, 4096, LabInstrSizeMode::Content),
            4096
        );
        assert_eq!(
            resolve_start_npu_inst_size(0, 0, LabInstrSizeMode::Content),
            1
        );
        // bo mode floors at 1 when alloc is 0 (callers must refuse empty BO).
        assert_eq!(
            resolve_start_npu_inst_size(1688, 0, LabInstrSizeMode::Bo),
            1
        );
        assert_eq!(
            try_lab_instr_size_mode_from_str(Some("bo")).unwrap(),
            LabInstrSizeMode::Bo
        );
        assert_eq!(
            try_lab_instr_size_mode_from_str(Some("alloc")).unwrap(),
            LabInstrSizeMode::Bo
        );
        assert_eq!(
            try_lab_instr_size_mode_from_str(Some("page")).unwrap(),
            LabInstrSizeMode::Bo
        );
        assert_eq!(
            try_lab_instr_size_mode_from_str(Some("CONTENT")).unwrap(),
            LabInstrSizeMode::Content
        );
        assert_eq!(
            try_lab_instr_size_mode_from_str(None).unwrap(),
            LabInstrSizeMode::Content
        );
        assert!(try_lab_instr_size_mode_from_str(Some("contents")).is_err());
        assert!(try_lab_instr_size_mode_from_str(Some("box")).is_err());
        let d = format_instr_size_detail(LabInstrSizeMode::Content, 1688, 4096, 1688);
        assert!(d.contains("inst_size_mode=content"));
        assert!(d.contains("content_size=1688B"));
        assert!(d.contains("bo_size=4096B"));
        assert!(d.contains("start_npu_size=1688B"));
        let dbo = format_instr_size_detail(LabInstrSizeMode::Bo, 1688, 4096, 4096);
        assert!(dbo.contains("inst_size_mode=bo"));
        assert!(dbo.contains("start_npu_size=4096B"));
        assert!(contract_instr_size_modes().iter().all(|c| c.ok));
    }

    /// APP_LOAD residual diag is pure research string — never C-ready / ran_on_npu.
    #[test]
    fn format_app_load_residual_diag_locks_closed_env_hitl() {
        let s = format_app_load_residual_diag(
            true,
            12,
            LabInstrSizeMode::Content,
            1688,
            4096,
            LabRelaMode::Overwrite,
            "count4",
        );
        assert!(s.contains("code=0x3000003"));
        assert!(s.contains("name=APP_LOAD_PDI_FAIL"));
        assert!(s.contains("aiebu_elf=1"));
        assert!(s.contains("rela_count=12"));
        assert!(s.contains("rela_mode=overwrite"));
        assert!(s.contains("layout=count4"));
        assert!(s.contains("inst_size_mode=content"));
        assert!(s.contains("content_size=1688B"));
        assert!(s.contains("bo_size=4096B"));
        assert!(s.contains("start_npu_size=1688B"));
        assert!(s.contains("env_hitl_closed=1"));
        assert!(s.contains("content_and_xrt3_closed=1"));
        assert!(s.contains("not_c_ready=1"));
        assert!(s.contains("not_ran_on_npu=1"));
        // bo mode flips start_npu_size only (residual still APP_LOAD offline note).
        let s_bo = format_app_load_residual_diag(
            true,
            12,
            LabInstrSizeMode::Bo,
            1688,
            4096,
            LabRelaMode::Add,
            "xrt3",
        );
        assert!(s_bo.contains("inst_size_mode=bo"));
        assert!(s_bo.contains("start_npu_size=4096B"));
        assert!(s_bo.contains("rela_mode=add"));
        assert!(s_bo.contains("layout=xrt3"));
        assert!(s_bo.contains("not_c_ready=1"));
    }

    /// last_apply-style detail (2026-07-16 xrt3 trial) → locked APP_LOAD after aiebu.
    #[test]
    fn classify_last_apply_locked_app_load_after_aiebu_xrt3() {
        // Shape matches testdata/xdna2_cmd/last_apply.json (operator-private snapshot).
        let detail = "EXEC_CMD+WAIT seq=0 ert_state=5(Error) accepted but OFM verify refused \
            (Some(SubmitWaitNotOk)); inst_blob content=1688B bo_size=4096B ty=Dev; \
            inst_size_mode=content content_size=1688B bo_size=4096B start_npu_size=1688B \
            aiebu_elf=1 (rela pending) rela_count=12; aiebu_rela_patched ctrltext=1688B \
            rela_mode=overwrite; layout=xrt3 ctrl_bytes=29664; \
            ofm_stats=[raw_len=512 raw_nonzero_bytes=0]; ran_on_npu=false; \
            fw_status_hint=ert_Error_after_aiebu_rela_check_journal_Status_0x";
        let sig = parse_last_apply_detail_signals(detail);
        assert_eq!(sig.ert_state, Some(5));
        assert!(sig.ert_error);
        assert_eq!(sig.aiebu_elf, Some(true));
        assert_eq!(sig.rela_count, Some(12));
        assert!(sig.aiebu_rela_patched);
        assert_eq!(sig.layout.as_deref(), Some("xrt3"));
        assert_eq!(sig.inst_size_mode.as_deref(), Some("content"));
        assert_eq!(sig.ran_on_npu, Some(false));
        assert_eq!(sig.ofm_raw_nonzero_bytes, Some(0));
        assert!(sig.fw_status_code.is_none(), "no hex code in this detail");

        let class = classify_last_apply_residual(
            Some("ert_cmd_error"),
            Some(detail),
            Some(true),
            Some(true),
        );
        assert_eq!(class, SiliconResidualClass::AppLoadPdiFailAfterAiebu);
        assert!(class.is_locked_app_load_after_aiebu());
        let note = format_silicon_residual_class_note(class, Some(&sig));
        assert!(note.contains("userspace_elf_rela_ok=1"));
        assert!(note.contains("fw_code=unknown_check_journal"));
        assert!(!note.contains("code=0x3000003"));
        assert!(note.contains("not_c_ready=1"));
        assert!(note.contains("not_ran_on_npu=1"));
        assert!(!note.contains("ran_on_npu=true"));
        // With explicit journal code in detail → note may emit code=
        let with_code = format!("{detail} journal Status 0x3000003");
        let sig2 = parse_last_apply_detail_signals(&with_code);
        assert_eq!(sig2.fw_status_code, Some(0x3_000_003));
        let note2 = format_silicon_residual_class_note(
            SiliconResidualClass::AppLoadPdiFailAfterAiebu,
            Some(&sig2),
        );
        assert!(note2.contains("code=0x3000003"));
        assert!(note2.contains("name=APP_LOAD_PDI_FAIL"));
    }

    #[test]
    fn classify_last_apply_raw_blob_app_load_class() {
        let detail = "ert_state=5(Error) aiebu_elf=0 rela_count=0 ran_on_npu=false";
        let class = classify_last_apply_residual(Some("ert_cmd_error"), Some(detail), None, None);
        assert_eq!(class, SiliconResidualClass::AppLoadLikelyRawOrUnpatched);
        let sig = parse_last_apply_detail_signals(detail);
        let note = format_silicon_residual_class_note(class, Some(&sig));
        assert!(note.contains("prefer_aiebu_elf=1"));
        assert!(note.contains("not_c_ready=1"));
        assert!(note.contains("fw_code=heuristic_check_journal"));
    }

    #[test]
    fn classify_last_apply_never_pass_from_detail_true() {
        // Even if detail lies with ran_on_npu=true, residual class stays residual.
        let detail = "ert_state=5(Error) aiebu_elf=1 rela_count=12 ran_on_npu=true";
        let class = classify_last_apply_residual(
            Some("ert_cmd_error"),
            Some(detail),
            Some(true),
            Some(true),
        );
        assert_eq!(class, SiliconResidualClass::AppLoadPdiFailAfterAiebu);
        assert!(!class.as_str().contains("pass"));
        let note = format_silicon_residual_class_note(class, None);
        assert!(note.contains("not_ran_on_npu=1"));
        // Without sig, do not invent userspace_elf_rela_ok or fw code.
        assert!(!note.contains("userspace_elf_rela_ok=1"));
        assert!(note.contains("fw_code=unknown_check_journal"));
        // Without ERT error → not APP_LOAD invent
        let okish = classify_last_apply_residual(
            Some("ok_submit_wait_with_inst_blob"),
            Some("ert_state=4(Completed) aiebu_elf=1 rela_count=12 ran_on_npu=false"),
            Some(true),
            Some(false),
        );
        assert_eq!(okish, SiliconResidualClass::NotPassNoErtError);
        assert_eq!(
            classify_last_apply_residual(None, None, None, None),
            SiliconResidualClass::Unclassified
        );
    }

    #[test]
    fn classify_ert_state_first_token_only_completed_then_error_elsewhere() {
        // First ert_state=Completed; later "(Error)" must not flip ert_error.
        let detail = "ert_state=4(Completed) aiebu_elf=1 rela_count=12 \
            ofm_stats note (Error) elsewhere ran_on_npu=false";
        let sig = parse_last_apply_detail_signals(detail);
        assert_eq!(sig.ert_state, Some(4));
        assert!(!sig.ert_error, "must not scan past first ert_state token");
        let class = classify_last_apply_residual(None, Some(detail), Some(true), Some(false));
        assert_ne!(class, SiliconResidualClass::AppLoadPdiFailAfterAiebu);
        assert_eq!(class, SiliconResidualClass::NotPassNoErtError);
    }

    #[test]
    fn classify_empty_detail_and_aiebu_without_rela_unknown() {
        assert_eq!(
            classify_last_apply_residual(None, Some(""), None, None),
            SiliconResidualClass::Unclassified
        );
        assert_eq!(
            classify_last_apply_residual(None, Some("   "), None, None),
            SiliconResidualClass::Unclassified
        );
        // aiebu + ert_error but no rela_count / aiebu_rela_patched
        let detail = "ert_state=5(Error) aiebu_elf=1 rela_count=0 ran_on_npu=false";
        let sig = parse_last_apply_detail_signals(detail);
        assert_eq!(sig.aiebu_elf, Some(true));
        assert_eq!(sig.rela_count, Some(0));
        assert!(!sig.aiebu_rela_patched);
        assert!(!last_apply_has_rela_evidence(&sig));
        let class = classify_last_apply_residual(Some("ert_cmd_error"), Some(detail), None, None);
        assert_eq!(class, SiliconResidualClass::ErtErrorUnknownBlob);
        let note = format_silicon_residual_class_note(class, Some(&sig));
        assert!(note.contains("aiebu=1"));
        assert!(note.contains("rela_evidence=0"));
        // unknown aiebu + status ert
        let unk =
            classify_last_apply_residual(Some("ert_cmd_error"), Some("no_blob_flags"), None, None);
        assert_eq!(unk, SiliconResidualClass::ErtErrorUnknownBlob);
    }

    #[test]
    fn classify_top_level_rela_count_field_unlocks_locked_app_load() {
        // Schema-2: rela_count only on JSON top-level; detail has aiebu+ert, no rela tokens.
        let detail = "ert_state=5(Error) aiebu_elf=1 ran_on_npu=false";
        let sig = parse_last_apply_detail_signals(detail);
        assert!(!last_apply_has_rela_evidence(&sig));
        // Without top-level field → unknown blob (no invent rela).
        assert_eq!(
            classify_last_apply_residual(
                Some("ert_cmd_error"),
                Some(detail),
                Some(true),
                Some(true)
            ),
            SiliconResidualClass::ErtErrorUnknownBlob
        );
        // Top-level rela_count=12 → locked APP_LOAD residual (still not Pass / C-ready).
        let class = classify_last_apply_residual_ex(
            Some("ert_cmd_error"),
            Some(detail),
            Some(true),
            Some(true),
            Some(12),
        );
        assert_eq!(class, SiliconResidualClass::AppLoadPdiFailAfterAiebu);
        // Zero / None do not invent.
        assert_eq!(
            classify_last_apply_residual_ex(
                Some("ert_cmd_error"),
                Some(detail),
                Some(true),
                Some(true),
                Some(0),
            ),
            SiliconResidualClass::ErtErrorUnknownBlob
        );
        assert_eq!(
            classify_last_apply_residual_ex(
                Some("ert_cmd_error"),
                Some(detail),
                Some(true),
                Some(true),
                None,
            ),
            SiliconResidualClass::ErtErrorUnknownBlob
        );
    }

    #[test]
    fn last_apply_load_path_refuses_symlink_when_present() {
        let dir = last_apply_temp_root();
        let target = dir.join(format!("dragon_npu_la_target_{}.json", std::process::id()));
        let link = dir.join(format!("dragon_npu_la_link_{}.json", std::process::id()));
        let _ = std::fs::remove_file(&target);
        let _ = std::fs::remove_file(&link);
        std::fs::write(&target, b"{}").expect("write target");
        // Best-effort symlink; skip assertion if OS refuses.
        if std::os::unix::fs::symlink(&target, &link).is_ok() {
            assert!(
                last_apply_path_is_allowed(&link),
                "path roots may still allow"
            );
            assert!(
                !last_apply_load_path_is_allowed(&link),
                "load must refuse symlink"
            );
            let _ = std::fs::remove_file(&link);
        }
        assert!(last_apply_load_path_is_allowed(&target));
        assert!(!last_apply_load_path_is_allowed(std::path::Path::new(
            "/etc/passwd"
        )));
        let _ = std::fs::remove_file(&target);
    }

    #[test]
    fn classify_ran_on_npu_token_exact_not_prefix() {
        let loose = parse_last_apply_detail_signals("ran_on_npu=trueish aiebu_elf=1");
        assert_eq!(loose.ran_on_npu, None);
        let exact = parse_last_apply_detail_signals("ran_on_npu=false; other=1");
        assert_eq!(exact.ran_on_npu, Some(false));
    }

    /// C3: coarse fingerprint equality + locked baseline shape (no new research matrix).
    #[test]
    fn last_apply_fixture_fingerprint_and_locked_baseline() {
        let detail = "EXEC_CMD+WAIT seq=0 ert_state=5(Error) accepted but OFM verify refused \
            (Some(SubmitWaitNotOk)); inst_blob content=1688B bo_size=4096B ty=Dev \
            xdna_addr=0x4020000 map_offset=0xffffffffffffffff; \
            aiebu_elf=1 rela_count=12; aiebu_rela_patched ctrltext=1688B \
            rela_mode=overwrite; layout=xrt3; \
            ofm_stats=[raw_len=512 raw_nonzero_bytes=0]; ran_on_npu=false";
        let fp = last_apply_evidence_fingerprint(
            Some("ert_cmd_error"),
            Some(true),
            Some(true),
            Some(false),
            Some(detail),
            Some(12),
        );
        assert_eq!(fp.detail_len, detail.len());
        assert_eq!(fp.status.as_deref(), Some("ert_cmd_error"));
        assert_eq!(fp.aiebu_elf, Some(true));
        assert_eq!(fp.ert_error, Some(true));
        assert_eq!(fp.ran_on_npu, Some(false));
        assert_eq!(fp.rela_count, Some(12));
        assert!(
            !fp.config_cu_present,
            "locked baseline detail must not invent config_cu_present"
        );
        let fp2 = last_apply_evidence_fingerprint(
            Some("ert_cmd_error"),
            Some(true),
            Some(true),
            Some(false),
            Some(detail),
            Some(12),
        );
        assert!(last_apply_is_same_fixture_fingerprint(&fp, &fp2));
        // Different detail length (same status/aiebu/ert/ran/rela) → different
        // evidence identity. C3: FP differs ⇒ C3_EVIDENCE=1 even if class still
        // locked APP_LOAD (never force same_body_as_baseline when FP differs).
        let detail_extra = format!("{detail} extra_c3_token=1");
        assert_ne!(detail_extra.len(), detail.len());
        let fp_diff = last_apply_evidence_fingerprint(
            Some("ert_cmd_error"),
            Some(true),
            Some(true),
            Some(false),
            Some(detail_extra.as_str()),
            Some(12),
        );
        assert_eq!(fp_diff.status, fp.status);
        assert_eq!(fp_diff.aiebu_elf, fp.aiebu_elf);
        assert_eq!(fp_diff.ert_error, fp.ert_error);
        assert_eq!(fp_diff.ran_on_npu, fp.ran_on_npu);
        assert_eq!(fp_diff.rela_count, fp.rela_count);
        assert_ne!(fp_diff.detail_len, fp.detail_len);
        assert!(
            !last_apply_is_same_fixture_fingerprint(&fp, &fp_diff),
            "detail_len change alone must mark different C3 evidence fingerprint"
        );
        // P5: config_cu= on detail must not false C3_SKIP vs locked baseline
        // (length may also change — covered next with equal-len isolation).
        let detail_cu = format!(
            "{detail} config_cu=1 pdi_bytes=283840 cu_func=0 invent_pass=0 not_ran_on_npu=1"
        );
        let fp_cu = last_apply_evidence_fingerprint(
            Some("ert_cmd_error"),
            Some(true),
            Some(true),
            Some(false),
            Some(detail_cu.as_str()),
            Some(12),
        );
        assert!(fp_cu.config_cu_present);
        assert!(
            !last_apply_is_same_fixture_fingerprint(&fp, &fp_cu),
            "config_cu= detail must not false C3_SKIP vs locked baseline"
        );
        // Equal detail_len isolation: only config_cu_present bit differs
        // (regression: dropping the bit from FP equality must fail this).
        let pad11 = "zzzzzzzzzzz"; // same len as "config_cu=1"
        assert_eq!(pad11.len(), "config_cu=1".len());
        let detail_eq_base = format!("{detail} {pad11}");
        let detail_eq_cu = format!("{detail} config_cu=1");
        assert_eq!(
            detail_eq_base.len(),
            detail_eq_cu.len(),
            "isolation pair must share detail_len"
        );
        let fp_eq_base = last_apply_evidence_fingerprint(
            Some("ert_cmd_error"),
            Some(true),
            Some(true),
            Some(false),
            Some(detail_eq_base.as_str()),
            Some(12),
        );
        let fp_eq_cu = last_apply_evidence_fingerprint(
            Some("ert_cmd_error"),
            Some(true),
            Some(true),
            Some(false),
            Some(detail_eq_cu.as_str()),
            Some(12),
        );
        assert_eq!(fp_eq_base.detail_len, fp_eq_cu.detail_len);
        assert!(!fp_eq_base.config_cu_present);
        assert!(fp_eq_cu.config_cu_present);
        assert!(
            !last_apply_is_same_fixture_fingerprint(&fp_eq_base, &fp_eq_cu),
            "equal detail_len + config_cu=1 must still differ via config_cu_present bit"
        );
        // Status-only fail path still marks evidence.
        let fp_fail = last_apply_evidence_fingerprint(
            Some("config_cu_failed"),
            Some(true),
            Some(false),
            Some(false),
            Some("XDNA2_LAB_PDI_BLOB set but CONFIG_CU attach failed (invent_pass=0)"),
            None,
        );
        assert!(fp_fail.config_cu_present);
        assert!(
            !last_apply_is_same_fixture_fingerprint(&fp, &fp_fail),
            "config_cu_failed status must differ from locked baseline FP"
        );
        // Still locked APP_LOAD class on the longer detail (rela preserved).
        assert!(last_apply_matches_locked_app_load_baseline(
            Some("ert_cmd_error"),
            Some(true),
            Some(true),
            Some(false),
            Some(detail_extra.as_str()),
            Some(12),
        ));
        // Locked APP_LOAD baseline shape (class must still be AppLoad after aiebu).
        assert!(last_apply_matches_locked_app_load_baseline(
            Some("ert_cmd_error"),
            Some(true),
            Some(true),
            Some(false),
            Some(detail),
            Some(12),
        ));
        // ran_on_npu true must never count as locked baseline residual shape.
        assert!(!last_apply_matches_locked_app_load_baseline(
            Some("ert_cmd_error"),
            Some(true),
            Some(true),
            Some(true),
            Some(detail),
            Some(12),
        ));
        // Missing aiebu → not locked baseline.
        assert!(!last_apply_matches_locked_app_load_baseline(
            Some("ert_cmd_error"),
            Some(false),
            Some(true),
            Some(false),
            Some(detail),
            Some(12),
        ));
        // aiebu without rela evidence → ErtErrorUnknownBlob, not locked APP_LOAD baseline.
        let aiebu_no_rela = "EXEC_CMD+WAIT ert_state=5(Error) aiebu_elf=1 layout=xrt3 \
            ran_on_npu=false ofm_stats=[raw_nonzero_bytes=0]";
        assert_eq!(
            classify_last_apply_residual_ex(
                Some("ert_cmd_error"),
                Some(aiebu_no_rela),
                Some(true),
                Some(true),
                None,
            ),
            SiliconResidualClass::ErtErrorUnknownBlob
        );
        assert!(
            !last_apply_matches_locked_app_load_baseline(
                Some("ert_cmd_error"),
                Some(true),
                Some(true),
                Some(false),
                Some(aiebu_no_rela),
                None,
            ),
            "aiebu without rela must not match locked APP_LOAD baseline for C3_SKIP"
        );
        // rela_count=0 top-level does not invent rela evidence.
        assert!(!last_apply_matches_locked_app_load_baseline(
            Some("ert_cmd_error"),
            Some(true),
            Some(true),
            Some(false),
            Some(aiebu_no_rela),
            Some(0),
        ));
        // Journal soft hint filter (print-only; never invents residual class).
        assert!(journal_line_has_app_load_hint(
            "amdxdna: Failed cmd … Status 0x3000003"
        ));
        assert!(journal_line_has_app_load_hint("APP_LOAD_PDI_FAIL residual"));
        assert!(!journal_line_has_app_load_hint("amdxdna: CREATE_HWCTX ok"));
        // Fixture on disk (when present) is locked baseline — C3 skip target.
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/xdna2_cmd/last_apply.json");
        if path.is_file() {
            let text = std::fs::read_to_string(&path).expect("read last_apply");
            let v: serde_json::Value = serde_json::from_str(&text).expect("json");
            let detail = v.get("detail").and_then(|x| x.as_str());
            let status = v.get("status").and_then(|x| x.as_str());
            let aiebu = v.get("aiebu_elf").and_then(|x| x.as_bool());
            let ert_error = v.get("ert_error").and_then(|x| x.as_bool());
            let ran = v.get("ran_on_npu").and_then(|x| x.as_bool());
            let rela = v
                .get("rela_count")
                .and_then(serde_json::Value::as_u64)
                .and_then(|n| u32::try_from(n).ok());
            assert!(
                last_apply_matches_locked_app_load_baseline(
                    status, aiebu, ert_error, ran, detail, rela
                ),
                "repo fixture must match locked APP_LOAD baseline for C3_SKIP"
            );
            let fp_fix =
                last_apply_evidence_fingerprint(status, aiebu, ert_error, ran, detail, rela);
            assert!(last_apply_is_same_fixture_fingerprint(&fp_fix, &fp_fix));
            assert_eq!(fp_fix.ran_on_npu, Some(false));
            assert!(
                !fp_fix.config_cu_present,
                "repo fixture last_apply must have config_cu_present=false (pre-P4 sandwich)"
            );
            if let Some(d) = detail {
                let sig = parse_last_apply_detail_signals(d);
                assert!(!last_apply_config_cu_present(&sig));
                assert!(!sig.config_cu_failed);
                assert_eq!(sig.config_cu, None);
            }
        }
    }

    /// P5: parse CONFIG_CU detail tokens + checklist honesty (≠ Pass / ≠ class invent).
    #[test]
    fn parse_last_apply_config_cu_signals_and_checklist() {
        // No tokens → absent.
        let bare = parse_last_apply_detail_signals(
            "ert_state=5(Error) aiebu_elf=1 rela_count=12 aiebu_rela_patched ran_on_npu=false",
        );
        assert_eq!(bare.config_cu, None);
        assert!(!bare.config_cu_failed);
        assert_eq!(bare.pdi_bytes, None);
        assert_eq!(bare.cu_func, None);
        assert!(!last_apply_config_cu_present(&bare));
        assert!(!last_apply_has_config_cu_signal(
            Some("ert_cmd_error"),
            Some("ert_state=5(Error) aiebu_elf=1 rela_count=12")
        ));

        // Lab success token string (matches LabConfigCuAttach::detail_token shape).
        let ok_detail = "EXEC_CMD+WAIT seq=0 ert_state=5(Error) aiebu_elf=1 rela_count=12 \
            aiebu_rela_patched layout=xrt3; \
            config_cu=1 pdi_bytes=283840 cu_bo=7 cu_func=0 invent_pass=0 not_ran_on_npu=1; \
            ofm_stats=[raw_nonzero_bytes=0]; ran_on_npu=false";
        let ok = parse_last_apply_detail_signals(ok_detail);
        assert_eq!(ok.config_cu, Some(1));
        assert!(!ok.config_cu_failed);
        assert_eq!(ok.pdi_bytes, Some(283_840));
        assert_eq!(ok.cu_func, Some(0));
        assert!(last_apply_config_cu_present(&ok));
        assert!(last_apply_has_config_cu_signal(
            Some("ert_cmd_error"),
            Some(ok_detail)
        ));
        // Residual class still APP_LOAD when aiebu+rela+ert_error (CONFIG_CU alone ≠ new class).
        let class = classify_last_apply_residual(
            Some("ert_cmd_error"),
            Some(ok_detail),
            Some(true),
            Some(true),
        );
        assert_eq!(class, SiliconResidualClass::AppLoadPdiFailAfterAiebu);
        let checklist = format_app_load_offline_checklist(class, &ok);
        assert!(checklist.contains("config_cu_present=1"));
        assert!(checklist.contains("config_cu_failed=0"));
        assert!(checklist.contains("config_cu=1"));
        assert!(checklist.contains("pdi_bytes=283840"));
        assert!(checklist.contains("cu_func=0"));
        assert!(checklist.contains("config_cu_attached_still_app_load=1"));
        assert!(checklist.contains("not_c_ready=1"));
        assert!(checklist.contains("not_ran_on_npu=1"));
        assert!(checklist.contains("invent_pass=0"));

        // Fail path: status + phrase (no config_cu= token).
        let fail_detail = "XDNA2_LAB_PDI_BLOB set but CONFIG_CU attach failed (fail-closed; invent_pass=0 not_ran_on_npu=1): io";
        let mut fail = parse_last_apply_detail_signals(fail_detail);
        assert!(fail.config_cu_failed);
        assert_eq!(fail.config_cu, None);
        merge_last_apply_status_config_cu(Some("config_cu_failed"), &mut fail);
        assert!(fail.config_cu_failed);
        assert!(last_apply_config_cu_present(&fail));
        let checklist_fail =
            format_app_load_offline_checklist(SiliconResidualClass::NotPassNoErtError, &fail);
        assert!(checklist_fail.contains("config_cu_present=1"));
        assert!(checklist_fail.contains("config_cu_failed=1"));
        assert!(!checklist_fail.contains("config_cu_attached_still_app_load=1"));
        assert!(checklist_fail.contains("invent_pass=0"));

        // Align present vs signal: bare pdi_bytes= alone is not CONFIG_CU present.
        let bare_pdi = parse_last_apply_detail_signals("pdi_bytes=283840 cu_func=0 invent_pass=0");
        assert_eq!(bare_pdi.pdi_bytes, Some(283_840));
        assert_eq!(bare_pdi.cu_func, Some(0));
        assert_eq!(bare_pdi.config_cu, None);
        assert!(!bare_pdi.config_cu_failed);
        assert!(
            !last_apply_config_cu_present(&bare_pdi),
            "pdi_bytes alone must not set config_cu_present (align with fingerprint bit)"
        );
        assert!(!last_apply_has_config_cu_signal(
            Some("ert_cmd_error"),
            Some("pdi_bytes=283840 cu_func=0")
        ));
        // Longer identifier must not whole-token match config_cu_failed.
        assert!(!last_apply_has_config_cu_signal(
            None,
            Some("my_config_cu_failed_extra=1")
        ));
    }

    #[test]
    fn classify_last_apply_json_fixture_when_present() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/xdna2_cmd/last_apply.json");
        if !path.is_file() {
            eprintln!("skip: no last_apply.json (operator-private)");
            return;
        }
        let text = std::fs::read_to_string(&path).expect("read last_apply");
        let v: serde_json::Value = serde_json::from_str(&text).expect("json");
        let detail = v.get("detail").and_then(|x| x.as_str());
        let status = v.get("status").and_then(|x| x.as_str());
        let aiebu = v.get("aiebu_elf").and_then(|x| x.as_bool());
        let ert_error = v.get("ert_error").and_then(|x| x.as_bool());
        assert_eq!(v.get("ran_on_npu").and_then(|x| x.as_bool()), Some(false));
        let class = classify_last_apply_residual(status, detail, aiebu, ert_error);
        assert_eq!(
            class,
            SiliconResidualClass::AppLoadPdiFailAfterAiebu,
            "fixture must classify locked APP_LOAD (ELF+rela+ert_error)"
        );
        if let Some(d) = detail {
            let sig = parse_last_apply_detail_signals(d);
            assert!(sig.ert_error);
            assert_eq!(sig.aiebu_elf, Some(true));
            assert!(last_apply_has_rela_evidence(&sig));
            assert_eq!(sig.ran_on_npu, Some(false));
            let note = format_silicon_residual_class_note(class, Some(&sig));
            assert!(note.contains("userspace_elf_rela_ok=1"));
            assert!(note.contains("not_c_ready=1"));
            // NEW offline signals unit-locked on fixture body (aae86bb9).
            assert_eq!(sig.instr_map_offset, Some(u64::MAX));
            assert!(instr_map_offset_is_no_mmap(sig.instr_map_offset.unwrap()));
            assert_eq!(sig.instr_bo_type.as_deref(), Some("Dev"));
            assert_eq!(sig.instr_xdna_addr, Some(0x402_0000));
            assert_eq!(sig.content_size_b, Some(1688));
            assert_eq!(sig.bo_size_b, Some(4096));
            assert_eq!(sig.start_npu_size_b, Some(1688));
            assert!(last_apply_start_npu_matches_content(&sig));
            assert!(last_apply_bo_gt_content(&sig));
            assert!(sig.submit_wait_not_ok);
            assert_eq!(sig.rela_mode.as_deref(), Some("overwrite"));
            assert!(sig.fw_status_code.is_none());
            // Pre-CONFIG_CU sandwich fixture: no lab config_cu tokens.
            assert_eq!(sig.config_cu, None);
            assert!(!sig.config_cu_failed);
            assert!(!last_apply_config_cu_present(&sig));
            let checklist = format_app_load_offline_checklist(class, &sig);
            assert!(checklist.contains("config_cu_present=0"));
            assert!(checklist.contains("config_cu_failed=0"));
            assert!(!checklist.contains("config_cu_attached_still_app_load=1"));
            let ck = format_app_load_offline_checklist(class, &sig);
            assert!(residual_note_has_exact_token(
                &ck,
                "instr_map_offset_no_mmap=1"
            ));
            assert!(residual_note_has_exact_token(
                &ck,
                "map_offset_not_app_load_root=1"
            ));
            assert!(residual_note_has_exact_token(&ck, "instr_ty_dev=1"));
            assert!(residual_note_has_exact_token(
                &ck,
                "start_npu_matches_content=1"
            ));
            assert!(residual_note_has_exact_token(&ck, "bo_gt_content=1"));
            assert!(residual_note_has_exact_token(&ck, "submit_wait_not_ok=1"));
            assert!(residual_note_has_exact_token(&ck, "ofm_all_zero=1"));
            assert!(residual_note_has_exact_token(
                &ck,
                "fw_code_absent_in_detail=1"
            ));
            assert!(residual_note_has_exact_token(
                &ck,
                "class_implies_app_load_after_aiebu=1"
            ));
            assert!(!residual_note_has_exact_token(
                &ck,
                "fw_residual=APP_LOAD_PDI_FAIL"
            ));
            assert!(residual_note_has_exact_token(&ck, "not_c_ready=1"));
            assert!(residual_note_has_exact_token(&ck, "not_ran_on_npu=1"));
            assert!(!residual_note_has_exact_token(&ck, "ran_on_npu=true"));
            assert!(!residual_note_has_exact_token(&ck, "c_ready=1"));
        }
    }

    /// Offline APP_LOAD signals: map_offset / Dev / sizes / SubmitWaitNotOk.
    #[test]
    fn parse_last_apply_app_load_root_cause_signals() {
        let detail = "EXEC_CMD+WAIT seq=0 ert_state=5(Error) accepted but OFM verify refused \
            (Some(SubmitWaitNotOk)); inst_blob content=1688B bo_size=4096B ty=Dev \
            xdna_addr=0x4020000 map_offset=0xffffffffffffffff; \
            inst_size_mode=content content_size=1688B bo_size=4096B start_npu_size=1688B \
            aiebu_elf=1 rela_count=12; aiebu_rela_patched ctrltext=1688B \
            rela_mode=overwrite; layout=xrt3; \
            ofm_stats=[raw_len=512 raw_nonzero_bytes=0]; ran_on_npu=false; \
            bo_diag=[DevHeap: handle=1 ty=DevHeap map_offset=0x100000000 xdna_addr=0x4000000]";
        let sig = parse_last_apply_detail_signals(detail);
        assert_eq!(sig.instr_map_offset, Some(u64::MAX));
        assert!(instr_map_offset_is_no_mmap(u64::MAX));
        assert!(!instr_map_offset_is_no_mmap(0x1000_0000));
        assert_eq!(sig.instr_bo_type.as_deref(), Some("Dev"));
        assert_eq!(sig.instr_xdna_addr, Some(0x402_0000));
        assert_eq!(sig.content_size_b, Some(1688));
        assert_eq!(sig.bo_size_b, Some(4096));
        assert_eq!(sig.start_npu_size_b, Some(1688));
        assert!(sig.submit_wait_not_ok);
        assert_eq!(sig.rela_mode.as_deref(), Some("overwrite"));
        assert!(last_apply_start_npu_matches_content(&sig));
        assert!(last_apply_bo_gt_content(&sig));
        // First ty= is Dev (inst), not DevHeap later — instr_bo_type stays Dev.
        assert_ne!(sig.instr_bo_type.as_deref(), Some("DevHeap"));
        // First map_offset is instr MAX, not DevHeap 0x100000000.
        assert_ne!(sig.instr_map_offset, Some(0x1_0000_0000));

        let class = classify_last_apply_residual(
            Some("ert_cmd_error"),
            Some(detail),
            Some(true),
            Some(true),
        );
        assert_eq!(class, SiliconResidualClass::AppLoadPdiFailAfterAiebu);
        let ck = format_app_load_offline_checklist(class, &sig);
        assert!(residual_note_has_exact_token(
            &ck,
            "userspace_elf_rela_ok=1"
        ));
        assert!(residual_note_has_exact_token(
            &ck,
            "instr_map_offset_no_mmap=1"
        ));
        assert!(residual_note_has_exact_token(
            &ck,
            "map_offset_not_app_load_root=1"
        ));
        assert!(ck.contains("instr_xdna_addr_ok=0x4020000"));
        assert!(ck.contains("content_size=1688B"));
        assert!(ck.contains("bo_size=4096B"));
        assert!(ck.contains("start_npu_size=1688B"));
        assert!(residual_note_has_exact_token(
            &ck,
            "fw_code_absent_in_detail=1"
        ));
        assert!(residual_note_has_exact_token(
            &ck,
            "class_implies_app_load_after_aiebu=1"
        ));
        assert!(!residual_note_has_exact_token(
            &ck,
            "fw_residual=APP_LOAD_PDI_FAIL"
        ));
        assert!(residual_note_has_exact_token(&ck, "not_c_ready=1"));
        // Journal code present → checklist emits fw_code / fw_name, not invent without code.
        let with = format!("{detail} journal Status 0x3000003");
        let sig2 = parse_last_apply_detail_signals(&with);
        assert_eq!(sig2.fw_status_code, Some(0x3_000_003));
        let ck2 = format_app_load_offline_checklist(class, &sig2);
        assert!(residual_note_has_exact_token(&ck2, "fw_code=0x3000003"));
        assert!(residual_note_has_exact_token(
            &ck2,
            "fw_name=APP_LOAD_PDI_FAIL"
        ));
        assert!(!residual_note_has_exact_token(
            &ck2,
            "fw_code_absent_in_detail=1"
        ));
    }

    #[test]
    fn app_load_checklist_never_invents_pass() {
        let sig = LastApplyDetailSignals {
            aiebu_elf: Some(true),
            rela_count: Some(12),
            aiebu_rela_patched: true,
            ert_error: true,
            instr_map_offset: Some(u64::MAX),
            instr_bo_type: Some("Dev".into()),
            content_size_b: Some(1688),
            bo_size_b: Some(4096),
            start_npu_size_b: Some(1688),
            submit_wait_not_ok: true,
            ofm_raw_nonzero_bytes: Some(0),
            ran_on_npu: Some(false),
            ..Default::default()
        };
        let ck =
            format_app_load_offline_checklist(SiliconResidualClass::AppLoadPdiFailAfterAiebu, &sig);
        assert!(residual_note_has_exact_token(&ck, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&ck, "not_ran_on_npu=1"));
        // Substring-safe: `not_c_ready=1` must not count as `c_ready=1`.
        assert!(!residual_note_has_exact_token(&ck, "c_ready=1"));
        assert!(!residual_note_has_exact_token(&ck, "ran_on_npu=1"));
        assert!(!residual_note_has_exact_token(&ck, "ran_on_npu=true"));
        assert!(!ck.to_ascii_lowercase().contains("pass_ok"));
        // Vacuous trap: contains() would wrongly fire on not_* tokens.
        assert!(ck.contains("c_ready=1")); // substring only
        assert!(!residual_note_has_exact_token(&ck, "c_ready=1"));
    }

    /// bo_diag-first order must not steal instr map_offset / ty.
    #[test]
    fn parse_app_load_tokens_prefer_inst_blob_region() {
        let detail = "bo_diag=[DevHeap: handle=1 ty=DevHeap map_offset=0x100000000 \
            xdna_addr=0x4000000]; inst_blob content=1688B bo_size=4096B ty=Dev \
            xdna_addr=0x4020000 map_offset=0xffffffffffffffff content_size=1688B \
            start_npu_size=1688B; ofm_stats=[raw_nonzero_bytes=0]";
        let sig = parse_last_apply_detail_signals(detail);
        assert_eq!(sig.instr_map_offset, Some(u64::MAX));
        assert_eq!(sig.instr_bo_type.as_deref(), Some("Dev"));
        assert_eq!(sig.instr_xdna_addr, Some(0x402_0000));
        assert_eq!(sig.content_size_b, Some(1688));
        assert_eq!(sig.bo_size_b, Some(4096));
        assert_eq!(sig.start_npu_size_b, Some(1688));
    }

    /// Negative matrix for APP_LOAD helpers + checklist branches.
    #[test]
    fn app_load_helpers_negative_matrix() {
        let empty = parse_last_apply_detail_signals("ert_state=5(Error) aiebu_elf=1 rela_count=12");
        assert!(empty.instr_map_offset.is_none());
        assert!(!empty.submit_wait_not_ok);
        assert!(!last_apply_start_npu_matches_content(&empty));
        assert!(!last_apply_bo_gt_content(&empty));
        let ck_empty = format_app_load_offline_checklist(
            SiliconResidualClass::AppLoadPdiFailAfterAiebu,
            &empty,
        );
        assert!(residual_note_has_exact_token(
            &ck_empty,
            "instr_map_offset_absent=1"
        ));
        assert!(residual_note_has_exact_token(
            &ck_empty,
            "instr_ty_unknown=1"
        ));
        assert!(!residual_note_has_exact_token(
            &ck_empty,
            "map_offset_not_app_load_root=1"
        ));

        let zero_map = parse_last_apply_detail_signals(
            "inst_blob ty=Dev map_offset=0x0 xdna_addr=0x0 content_size=100B \
             bo_size=100B start_npu_size=200B aiebu_elf=1 rela_count=12",
        );
        assert_eq!(zero_map.instr_map_offset, Some(0));
        assert!(!instr_map_offset_is_no_mmap(0));
        assert!(!last_apply_start_npu_matches_content(&zero_map)); // 100 != 200
        assert!(!last_apply_bo_gt_content(&zero_map)); // equal sizes
        let ck_z = format_app_load_offline_checklist(
            SiliconResidualClass::AppLoadPdiFailAfterAiebu,
            &zero_map,
        );
        assert!(ck_z.contains("instr_map_offset=0x0"));
        assert!(ck_z.contains("instr_xdna_addr_invalid=0x0"));
        assert!(!residual_note_has_exact_token(
            &ck_z,
            "start_npu_matches_content=1"
        ));
        assert!(!residual_note_has_exact_token(&ck_z, "bo_gt_content=1"));

        // no_mmap without Dev → do not claim not_app_load_root
        let no_dev = LastApplyDetailSignals {
            aiebu_elf: Some(true),
            rela_count: Some(12),
            instr_map_offset: Some(u64::MAX),
            instr_bo_type: Some("Shmem".into()),
            ..Default::default()
        };
        let ck_nd = format_app_load_offline_checklist(
            SiliconResidualClass::AppLoadPdiFailAfterAiebu,
            &no_dev,
        );
        assert!(residual_note_has_exact_token(
            &ck_nd,
            "instr_map_offset_no_mmap=1"
        ));
        assert!(residual_note_has_exact_token(
            &ck_nd,
            "map_offset_no_mmap_ty_not_dev=1"
        ));
        assert!(!residual_note_has_exact_token(
            &ck_nd,
            "map_offset_not_app_load_root=1"
        ));

        // aiebu unknown + rela → userspace_elf_rela_ok=0
        let unk = LastApplyDetailSignals {
            aiebu_elf: None,
            rela_count: Some(12),
            ..Default::default()
        };
        let ck_u =
            format_app_load_offline_checklist(SiliconResidualClass::ErtErrorUnknownBlob, &unk);
        assert!(residual_note_has_exact_token(
            &ck_u,
            "userspace_elf_rela_ok=0"
        ));
        assert!(residual_note_has_exact_token(&ck_u, "aiebu_unknown=1"));

        // content_size=0 does not match start_npu
        let zero_c = LastApplyDetailSignals {
            content_size_b: Some(0),
            start_npu_size_b: Some(0),
            bo_size_b: Some(4096),
            ..Default::default()
        };
        assert!(!last_apply_start_npu_matches_content(&zero_c));
        assert!(!last_apply_bo_gt_content(&zero_c)); // c==0
    }

    #[test]
    fn last_apply_path_refuses_intermediate_dir_symlink() {
        // Clamped temp root (safe if another test briefly sets TMPDIR=/).
        let temp = last_apply_temp_root();
        let pid = std::process::id();
        let temp_root = std::fs::canonicalize(&temp).unwrap_or_else(|_| temp.clone());
        let lab = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/xdna2_cmd");
        let lab_root = std::fs::canonicalize(&lab).unwrap_or(lab);

        // Plant outside allowed roots: prefer /var/tmp, else $HOME/.cache.
        let mut real_outside = PathBuf::from(format!("/var/tmp/dragon_npu_la_out_{pid}"));
        if std::fs::create_dir_all(&real_outside).is_err() {
            let home = match std::env::var_os("HOME") {
                Some(h) => PathBuf::from(h).join(format!(".cache/dragon_npu_la_out_{pid}")),
                None => {
                    eprintln!("skip: no outside dir for mid-path symlink test");
                    return;
                }
            };
            if std::fs::create_dir_all(&home).is_err() {
                eprintln!("skip: cannot create outside dir");
                return;
            }
            real_outside = home;
        }
        let real_canon = match std::fs::canonicalize(&real_outside) {
            Ok(c) => c,
            Err(_) => {
                let _ = std::fs::remove_dir_all(&real_outside);
                return;
            }
        };
        if real_canon.starts_with(&temp_root) || real_canon.starts_with(&lab_root) {
            let _ = std::fs::remove_dir_all(&real_outside);
            eprintln!("skip: outside dir landed under allowed root");
            return;
        }
        let real_file = real_outside.join("secret.json");
        std::fs::write(&real_file, b"{}").expect("write outside file");
        let mid = temp.join(format!("dragon_npu_evil_mid_{pid}"));
        let _ = std::fs::remove_file(&mid);
        let _ = std::fs::remove_dir(&mid);
        if std::os::unix::fs::symlink(&real_outside, &mid).is_err() {
            let _ = std::fs::remove_file(&real_file);
            let _ = std::fs::remove_dir_all(&real_outside);
            eprintln!("skip: symlink refused");
            return;
        }
        let attack = mid.join("secret.json");
        // Lexical: under temp root; resolved parent: outside → must refuse.
        assert!(
            !last_apply_path_is_allowed(&attack),
            "mid-path dir symlink must refuse write path: {}",
            attack.display()
        );
        assert!(
            !last_apply_load_path_is_allowed(&attack),
            "mid-path dir symlink must refuse load path"
        );
        let _ = std::fs::remove_file(&mid);
        let _ = std::fs::remove_file(&real_file);
        let _ = std::fs::remove_dir_all(&real_outside);
    }

    #[test]
    fn contract_dynsym_empty_is_green_on_parse_ok() {
        let ctrl = vec![0_u8; 64];
        let elf = build_minimal_test_elf32(&ctrl, &[(0, (1 << 8) | 5, 0)]);
        let c = contract_dynsym_report(&elf);
        assert!(c.ok, "empty dynsym must not fail contract: {}", c.detail);
        assert!(c.detail.contains("dynsym empty") || c.detail.contains("dynsym_n=0"));
        let c2 = contract_dynsym_report(&[1, 2, 3]);
        assert!(c2.ok, "non-ELF empty parse ok");
    }

    #[test]
    fn contract_report_staged_elf_post_patch_and_dynsym() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("testdata/xdna2_cmd/silicon_lab/lab_instr.elf");
        if !path.is_file() {
            eprintln!("skip: no lab_instr.elf (staged asset)");
            return;
        }
        let elf = std::fs::read(&path).expect("elf");
        let r = contract_report(
            Some(&elf),
            0xa11f_0000_1000,
            0xa11f_0000_2000,
            0xa11f_0000_3000,
            0xa11f_0000_4000,
        );
        assert!(
            r.iter().any(|c| c.id == "blob_kind" && c.ok),
            "aiebu+rela should be green"
        );
        let dsym = r.iter().find(|c| c.id == "dynsym_report").expect("dynsym");
        assert!(dsym.ok, "{}", dsym.detail);
        assert!(dsym.detail.contains("dynsym_n=4"));
        assert!(dsym.detail.contains("\"6\""));
        let walk = r
            .iter()
            .find(|c| c.id == "post_patch_txn_walk")
            .expect("walk");
        assert!(walk.ok, "{}", walk.detail);
        assert!(walk.detail.contains("num_ops=40"));
        assert!(walk.detail.contains("txn_size=1688"));
        let walk_add = r
            .iter()
            .find(|c| c.id == "post_patch_txn_walk_add")
            .expect("walk add");
        assert!(walk_add.ok, "{}", walk_add.detail);
        assert!(r.iter().any(|c| c.id == "kernel_args_count4_len" && c.ok));
        assert!(r.iter().any(|c| c.id == "kernel_args_xrt3_len" && c.ok));
    }

    #[test]
    fn contract_classify_synthetic_elf_with_rela() {
        let ctrl = vec![0_u8; 64];
        // sym=1 (info = 1<<8 | type5), offset 0, addend 0
        let elf = build_minimal_test_elf32(&ctrl, &[(0, (1 << 8) | 5, 0)]);
        let c = contract_classify_blob(&elf);
        assert!(c.ok, "detail={}", c.detail);
        assert!(c.detail.contains("rela_count=1"));
        // Empty rela → not EXEC-green (even if .rela.dyn section exists with size 0).
        let empty = build_minimal_test_elf32(&ctrl, &[]);
        assert!(!contract_classify_blob(&empty).ok, "empty rela must be !ok");
    }

    #[test]
    fn patch_refuses_raw_txn_and_empty_rela() {
        let txn = build_strix_research_noop_txn().expect("noop");
        let e = patch_aiebu_elf_ctrltext_with_bos(&txn, 1, 2, 3, 4).expect_err("raw");
        assert!(e.to_string().contains("aiebu ELF") || e.to_string().contains("ctrltext"));

        let ctrl = vec![0_u8; 32];
        let elf_no_rela = build_minimal_test_elf32(&ctrl, &[]);
        // classify may still see .rela.dyn section with size 0
        let e2 = patch_aiebu_elf_ctrltext_with_bos(&elf_no_rela, 1, 2, 3, 4).expect_err("empty");
        assert!(
            e2.to_string().contains("rela") || e2.to_string().contains("ctrltext"),
            "{e2}"
        );

        // OOB reloc
        let elf_oob = build_minimal_test_elf32(&ctrl, &[(100, (1 << 8) | 5, 0)]);
        let e3 = patch_aiebu_elf_ctrltext_with_bos(&elf_oob, 1, 2, 3, 4).expect_err("oob");
        assert!(
            e3.to_string().contains("OOB")
                || e3.to_string().contains("oob")
                || e3.to_string().contains("offset"),
            "{e3}"
        );
    }

    #[test]
    fn patch_synthetic_elf_applies_u64() {
        let mut ctrl = vec![0_u8; 32];
        ctrl[0..8].copy_from_slice(&0_u64.to_le_bytes());
        let elf = build_minimal_test_elf32(&ctrl, &[(0, (1 << 8) | 5, 0x10)]);
        let patched =
            patch_aiebu_elf_ctrltext_with_bos(&elf, 0x1000, 0x2000, 0x3000, 0x4000).expect("patch");
        let v = u64::from_le_bytes(patched[0..8].try_into().unwrap());
        assert_eq!(v, 0x1000 + 0x10);
    }

    #[test]
    fn contract_classify_staged_elf_if_present() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("testdata/xdna2_cmd/silicon_lab/lab_instr.elf");
        if !path.is_file() {
            return;
        }
        let elf = std::fs::read(&path).expect("read");
        let c = contract_classify_blob(&elf);
        assert!(c.ok, "staged ELF should be contract-green: {}", c.detail);
        assert!(
            c.detail.contains("rela_count=12"),
            "expect 12 relas: {}",
            c.detail
        );
    }

    #[test]
    fn last_apply_path_allowed_confinement() {
        // Use clamped temp root (not raw temp_dir) so TMPDIR=/ or symlink→/ is safe.
        let tmp = last_apply_temp_root().join("dragon_npu_la_test.json");
        assert!(last_apply_path_is_allowed(&tmp));
        assert!(!last_apply_path_is_allowed(Path::new("/etc/passwd")));
        assert!(!last_apply_path_is_allowed(Path::new("../../etc/passwd")));
        assert!(!last_apply_path_is_allowed(Path::new("/tmp/not_json.txt")));
        // String prefix footgun: /tmp_evil is NOT under /tmp as Path components.
        assert!(
            !path_under_allowed_roots(Path::new("/tmp_evil/last_apply.json")),
            "sibling prefix of temp_dir must be refused"
        );
        // When clamped temp is /tmp (or ends with tmp), /tmp_evil must also fail.
        let tr = last_apply_temp_root();
        if tr == Path::new("/tmp") || tr.ends_with("tmp") {
            assert!(!last_apply_path_is_allowed(Path::new(
                "/tmp_evil/last_apply.json"
            )));
        }
    }

    #[test]
    fn batch_diag_includes_blob_kind() {
        let s = format_lab_batch_diag(LabBatchDiag {
            tiles: 4,
            qos_gops: 100,
            qos_fps: 30,
            rela_count: None,
            aiebu_elf: Some(true),
            ert_error: false,
            layout: "count4",
            instr_size_mode: "content",
            rela_mode: "overwrite",
        });
        assert!(s.contains("blob_kind=aiebu_elf"), "{s}");
        assert!(s.contains("aiebu_elf=1"), "{s}");
    }

    #[test]
    fn write_last_apply_best_effort_tmp() {
        let dir = last_apply_temp_root()
            .join(format!("dragon_npu_last_apply_test_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("last_apply.json");
        let r = TimedOpResult {
            op: "matmul".into(),
            m: 1,
            n: 256,
            k: 1152,
            size_bytes: None,
            dtype: "mladf_a16w4".into(),
            total_s: 0.01,
            host_prep_s: 0.0,
            hwctx_s: None,
            fill_s: None,
            submit_wait_s: None,
            device_s: None,
            host_ref_s: None,
            host_ref_ok: false,
            host_ref_detail: String::new(),
            ran_on_npu: false,
            ran_on_device_path: false,
            real_ioctl: false,
            status: "ert_cmd_error".into(),
            detail: "unit test".into(),
        };
        write_last_apply_to_path(
            &r,
            LabBatchDiag {
                tiles: 32,
                qos_gops: 100,
                qos_fps: 30,
                rela_count: Some(12),
                aiebu_elf: Some(true),
                ert_error: true,
                layout: "count4",
                instr_size_mode: "content",
                rela_mode: "overwrite",
            },
            "layout=count4",
            15,
            Some(&path),
        );
        let text = std::fs::read_to_string(&path).expect("written");
        assert!(text.contains("\"ran_on_npu\": false"));
        assert!(text.contains("\"qos_gops\": 100"));
        assert!(text.contains("\"layout\": \"count4\""));
        assert!(!text.contains("\"layout\": \"layout=count4\""));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn parse_paddr_json_null_and_hex() {
        assert_eq!(
            parse_paddr_json_value(&serde_json::Value::Null).unwrap(),
            None
        );
        assert_eq!(
            parse_paddr_json_value(&serde_json::json!("null")).unwrap(),
            None
        );
        assert_eq!(
            parse_paddr_json_value(&serde_json::json!("0x1000")).unwrap(),
            Some(0x1000)
        );
        assert_eq!(
            parse_paddr_json_value(&serde_json::json!("0XABC")).unwrap(),
            Some(0xABC)
        );
        assert_eq!(
            parse_paddr_json_value(&serde_json::json!(42u64)).unwrap(),
            Some(42)
        );
        // Pure decimal string → decimal, not hex
        assert_eq!(
            parse_paddr_json_value(&serde_json::json!("1000")).unwrap(),
            Some(1000)
        );
    }

    #[test]
    fn parse_paddr_json_value_refuses_zero_and_max() {
        assert_eq!(
            parse_paddr_json_value(&serde_json::json!("0x0")).unwrap(),
            None
        );
        assert_eq!(
            parse_paddr_json_value(&serde_json::json!(0u64)).unwrap(),
            None
        );
        assert_eq!(
            parse_paddr_json_value(&serde_json::json!("0")).unwrap(),
            None
        );
        assert_eq!(
            parse_paddr_json_value(&serde_json::json!(u64::MAX)).unwrap(),
            None
        );
        assert_eq!(
            parse_paddr_json_value(&serde_json::json!("0xffffffffffffffff")).unwrap(),
            None
        );
        assert_eq!(
            parse_paddr_json_value(&serde_json::json!("0xFFFFFFFFFFFFFFFF")).unwrap(),
            None
        );
        assert!(!is_valid_device_paddr(0));
        assert!(!is_valid_device_paddr(u64::MAX));
        assert!(is_valid_device_paddr(1));
    }

    #[test]
    fn parse_paddr_json_value_err_invalid() {
        assert!(parse_paddr_json_value(&serde_json::json!(-1i64)).is_err());
        assert!(parse_paddr_json_value(&serde_json::json!("not_a_paddr")).is_err());
        assert!(parse_paddr_json_value(&serde_json::json!([1, 2])).is_err());
    }

    #[test]
    fn parse_vendor_paddrs_schema_v1_nulls() {
        let j = r#"{
          "schema_version": 1,
          "source": "capture_mladf_instr_vendor",
          "capture_mode": "txn_bin_fallback_unpatched",
          "ifm_paddr": null,
          "wts_paddr": null,
          "ofm_paddr": null,
          "ctrl_paddr": null
        }"#;
        let p = parse_vendor_paddrs_json(j).expect("parse");
        assert_eq!(p.schema_version, Some(1));
        assert_eq!(
            p.capture_mode.as_deref(),
            Some("txn_bin_fallback_unpatched")
        );
        assert!(!p.paddrs_complete());
        assert!(p.as_io_tuple().is_none());
        assert!(p.ifm_paddr.is_none());
    }

    #[test]
    fn parse_vendor_paddrs_schema_zero_addrs_incomplete() {
        let j = r#"{
          "schema_version": 1,
          "source": "unit",
          "ifm_paddr": "0x0",
          "wts_paddr": "0x0",
          "ofm_paddr": "0x0",
          "ctrl_paddr": "0x0"
        }"#;
        let p = parse_vendor_paddrs_json(j).expect("parse");
        // parse_paddr_json_value maps 0 → None
        assert!(p.ifm_paddr.is_none());
        assert!(!p.paddrs_complete());
        assert!(p.as_io_tuple().is_none());

        // Belt-and-suspenders: raw Some(0) still incomplete
        let raw = VendorPaddrs {
            schema_version: Some(1),
            capture_mode: None,
            source: None,
            ifm_paddr: Some(0),
            wts_paddr: Some(1),
            ofm_paddr: Some(2),
            ctrl_paddr: Some(u64::MAX),
        };
        assert!(!raw.paddrs_complete());
        assert!(raw.as_io_tuple().is_none());
        let raw_max = VendorPaddrs {
            ifm_paddr: Some(u64::MAX),
            wts_paddr: Some(0x10),
            ofm_paddr: Some(0x20),
            ..raw.clone()
        };
        assert!(!raw_max.paddrs_complete());
        assert!(raw_max.as_io_tuple().is_none());
    }

    #[test]
    fn parse_vendor_paddrs_json_invalid_root() {
        assert!(parse_vendor_paddrs_json("not json").is_err());
        assert!(parse_vendor_paddrs_json("[1,2,3]").is_err());
        assert!(parse_vendor_paddrs_json(r#"{"ifm_paddr":-1}"#).is_err());
    }

    #[test]
    fn parse_vendor_paddrs_schema_v1_hex() {
        let j = r#"{
          "schema_version": 1,
          "source": "last_apply_extract",
          "capture_mode": "last_apply_detail",
          "ifm_paddr": "0x7f57b364c000",
          "wts_paddr": "0x7f57b3482000",
          "ofm_paddr": "0x7f57b364b000",
          "ctrl_paddr": "0x7f57b3643000"
        }"#;
        let p = parse_vendor_paddrs_json(j).expect("parse");
        assert!(p.paddrs_complete());
        let t = p.as_io_tuple().expect("tuple");
        assert_eq!(t.0, 0x7f57_b364_c000);
        assert_eq!(t.1, 0x7f57_b348_2000);
        assert_eq!(t.2, 0x7f57_b364_b000);
        assert_eq!(t.3, 0x7f57_b364_3000);
    }

    #[test]
    fn parse_io_paddrs_from_detail_live_shape() {
        let detail = "lab_ofm pack=mladf_a16w4; io_paddrs ifm=0x7f57b364c000 wts=0x7f57b3482000 \
                      ofm=0x7f57b364b000 ctrl=0x7f57b3643000; ofm_stats=...";
        let t = parse_io_paddrs_from_detail(detail).expect("paddrs");
        assert_eq!(t.0, 0x7f57_b364_c000);
        assert_eq!(t.1, 0x7f57_b348_2000);
        assert_eq!(t.2, 0x7f57_b364_b000);
        assert_eq!(t.3, 0x7f57_b364_3000);
        assert!(parse_io_paddrs_from_detail("no paddrs here").is_none());
    }

    #[test]
    fn parse_io_paddrs_from_detail_zeros_none() {
        let z = "io_paddrs ifm=0x0 wts=0x0 ofm=0x0 ctrl=0x0";
        assert!(parse_io_paddrs_from_detail(z).is_none());
        let max = "io_paddrs ifm=0xffffffffffffffff wts=0x1 ofm=0x2 ctrl=0x3";
        assert!(parse_io_paddrs_from_detail(max).is_none());
        // Valid ifm/wts/ofm with zero ctrl still accepted (ctrl optional)
        let ctrl0 = "io_paddrs ifm=0x10 wts=0x20 ofm=0x30 ctrl=0x0";
        assert_eq!(
            parse_io_paddrs_from_detail(ctrl0),
            Some((0x10, 0x20, 0x30, 0))
        );
    }

    #[test]
    fn vendor_paddrs_json_from_io_roundtrip() {
        let v = vendor_paddrs_json_from_io(0x11, 0x22, 0x33, 0x44, "unit");
        let s = serde_json::to_string(&v).unwrap();
        let p = parse_vendor_paddrs_json(&s).unwrap();
        assert_eq!(p.as_io_tuple(), Some((0x11, 0x22, 0x33, 0x44)));
        assert_eq!(p.source.as_deref(), Some("unit"));
    }

    /// C4: locked baseline fixture is NotPass (json_ran_on_npu=false + ERT Error).
    #[test]
    fn c4_pass_eval_fixture_last_apply_is_not_pass() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/xdna2_cmd/last_apply.json");
        if !path.is_file() {
            eprintln!("skip: no last_apply.json (operator-private)");
            return;
        }
        let text = std::fs::read_to_string(&path).expect("read last_apply");
        let v: serde_json::Value = serde_json::from_str(&text).expect("json");
        let detail = v.get("detail").and_then(|x| x.as_str());
        let status = v.get("status").and_then(|x| x.as_str());
        let ran = v.get("ran_on_npu").and_then(|x| x.as_bool());
        let ert_error = v.get("ert_error").and_then(|x| x.as_bool());
        let real_ioctl = v.get("real_ioctl").and_then(|x| x.as_bool());
        assert_eq!(ran, Some(false));
        let ev = silicon_pass_evidence_from_last_apply(ran, status, detail, ert_error, real_ioctl);
        let eval = evaluate_silicon_pass_evidence(&ev);
        assert!(
            !eval.is_pass_candidate(),
            "fixture must never be PassCandidate"
        );
        match eval {
            SiliconPassEval::NotPass { reason } => {
                assert_eq!(
                    reason, "json_ran_on_npu_false",
                    "hard rule: false ran_on_npu blocks first"
                );
            }
            SiliconPassEval::PassCandidate { notes } => {
                panic!("unexpected PassCandidate: {notes}");
            }
        }
        let note = format_silicon_pass_eval_note(&eval);
        assert!(residual_note_has_exact_token(&note, "not_ran_on_npu=1"));
        assert!(residual_note_has_exact_token(&note, "offline_audit_only=1"));
        assert!(residual_note_has_exact_token(&note, "invent_pass=0"));
        assert!(!note.contains("ran_on_npu=true"));
    }

    /// C4: synthetic all-true snapshot → PassCandidate (offline audit only).
    /// Requires ofm nonzero + real_ioctl=true + exact status/ert.
    #[test]
    fn c4_pass_eval_synthetic_all_true_is_pass_candidate() {
        let detail = "EXEC_CMD+WAIT seq=0 ert_state=4(Completed) aiebu_elf=1 rela_count=12; \
            ofm_stats=[raw_len=512 raw_nonzero_bytes=42]; ran_on_npu=true";
        let ev = silicon_pass_evidence_from_last_apply(
            Some(true),
            Some("ok_npu_verified"),
            Some(detail),
            Some(false),
            Some(true),
        );
        assert_eq!(ev.ert_state, Some(4));
        assert!(!ev.detail_ert_error);
        assert_eq!(ev.ofm_raw_nonzero_bytes, Some(42));
        assert_eq!(ev.real_ioctl, Some(true));
        let eval = evaluate_silicon_pass_evidence(&ev);
        assert!(eval.is_pass_candidate());
        assert_eq!(eval.as_str(), "pass_candidate");
        match eval {
            SiliconPassEval::PassCandidate { ref notes } => {
                assert!(notes.contains("offline_audit_only=1"));
                assert!(notes.contains("json_ran_on_npu=true"));
                assert!(notes.contains("real_ioctl=1"));
                assert!(notes.contains("ofm_raw_nonzero=42"));
                assert!(notes.contains("not_c_ready=1"));
                assert!(notes.contains("not_ran_on_npu=1"));
                assert!(notes.contains("invent_pass=0"));
            }
            SiliconPassEval::NotPass { reason } => panic!("expected candidate, got {reason}"),
        }
        let note = format_silicon_pass_eval_note(&eval);
        assert!(residual_note_has_exact_token(
            &note,
            "snapshot_pass_candidate=1"
        ));
        assert!(residual_note_has_exact_token(&note, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&note, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&note, "not_ran_on_npu=1"));
        assert!(residual_note_has_exact_token(
            &note,
            "product_pass_requires_live_decide=1"
        ));
    }

    /// C4 hard rule: json ran_on_npu=true alone with ERT error → NotPass.
    #[test]
    fn c4_pass_eval_ran_true_with_ert_error_is_not_pass() {
        let detail = "ert_state=5(Error) aiebu_elf=1 rela_count=12 \
            ofm_stats=[raw_nonzero_bytes=99] ran_on_npu=true";
        let ev = silicon_pass_evidence_from_last_apply(
            Some(true),
            Some("ert_cmd_error"),
            Some(detail),
            Some(true),
            Some(true),
        );
        let eval = evaluate_silicon_pass_evidence(&ev);
        assert!(!eval.is_pass_candidate());
        assert_eq!(
            eval,
            SiliconPassEval::NotPass {
                reason: "ert_error"
            }
        );
    }

    /// C4: missing json ran_on_npu never invents PassCandidate.
    #[test]
    fn c4_pass_eval_missing_json_ran_never_pass() {
        let detail = "ert_state=4(Completed) ofm_stats=[raw_nonzero_bytes=1]";
        let ev = silicon_pass_evidence_from_last_apply(
            None,
            Some("ok_npu_verified"),
            Some(detail),
            Some(false),
            Some(true),
        );
        assert_eq!(
            evaluate_silicon_pass_evidence(&ev),
            SiliconPassEval::NotPass {
                reason: "json_ran_on_npu_missing"
            }
        );
    }

    /// C4: exact status/ert only — no contains/starts_with invent.
    #[test]
    fn c4_pass_eval_status_exact_not_completed_ok_npu_fake() {
        let detail = "ert_state=4(Completed) ofm_stats=[raw_nonzero_bytes=9]";
        // not_completed must not match contains("completed")
        let mut ev = silicon_pass_evidence_from_last_apply(
            Some(true),
            Some("not_completed"),
            Some(detail),
            Some(false),
            Some(true),
        );
        assert_eq!(
            evaluate_silicon_pass_evidence(&ev),
            SiliconPassEval::NotPass {
                reason: "status_not_pass"
            }
        );
        // ok_npu_fake must not match starts_with("ok_npu_")
        ev = silicon_pass_evidence_from_last_apply(
            Some(true),
            Some("ok_npu_fake"),
            Some(detail),
            Some(false),
            Some(true),
        );
        assert_eq!(
            evaluate_silicon_pass_evidence(&ev),
            SiliconPassEval::NotPass {
                reason: "status_not_pass"
            }
        );
        // ok_npu_error must not invent completed
        ev = silicon_pass_evidence_from_last_apply(
            Some(true),
            Some("ok_npu_error"),
            Some(detail),
            Some(false),
            Some(true),
        );
        assert_eq!(
            evaluate_silicon_pass_evidence(&ev),
            SiliconPassEval::NotPass {
                reason: "status_not_pass"
            }
        );
        // No status + no ert_state=4 → ert_not_completed
        ev = silicon_pass_evidence_from_last_apply(
            Some(true),
            None,
            Some("ofm_stats=[raw_nonzero_bytes=9]"),
            Some(false),
            Some(true),
        );
        assert_eq!(
            evaluate_silicon_pass_evidence(&ev),
            SiliconPassEval::NotPass {
                reason: "ert_not_completed"
            }
        );
        // Exact ok_npu_verified alone (no ert_state) still completed
        ev = silicon_pass_evidence_from_last_apply(
            Some(true),
            Some("ok_npu_verified"),
            Some("ofm_stats=[raw_nonzero_bytes=9]"),
            Some(false),
            Some(true),
        );
        assert!(evaluate_silicon_pass_evidence(&ev).is_pass_candidate());
        // No status + ert_state=4 still completed
        ev = silicon_pass_evidence_from_last_apply(
            Some(true),
            None,
            Some("ert_state=4(Completed) ofm_stats=[raw_nonzero_bytes=9]"),
            Some(false),
            Some(true),
        );
        assert!(evaluate_silicon_pass_evidence(&ev).is_pass_candidate());
    }

    /// C4: real_ioctl required true; OFM nonzero required; missing reasons clear.
    #[test]
    fn c4_pass_eval_secondary_gates() {
        // real_ioctl false
        let mut ev = silicon_pass_evidence_from_last_apply(
            Some(true),
            Some("ok_npu_verified"),
            Some("ert_state=4(Completed) ofm_stats=[raw_nonzero_bytes=3]"),
            Some(false),
            Some(false),
        );
        assert_eq!(
            evaluate_silicon_pass_evidence(&ev),
            SiliconPassEval::NotPass {
                reason: "real_ioctl_false"
            }
        );
        // real_ioctl missing
        ev = silicon_pass_evidence_from_last_apply(
            Some(true),
            Some("ok_npu_verified"),
            Some("ert_state=4(Completed) ofm_stats=[raw_nonzero_bytes=7]"),
            Some(false),
            None,
        );
        assert_eq!(
            evaluate_silicon_pass_evidence(&ev),
            SiliconPassEval::NotPass {
                reason: "real_ioctl_missing"
            }
        );
        // ofm all zero
        ev = silicon_pass_evidence_from_last_apply(
            Some(true),
            Some("ok_npu_verified"),
            Some("ert_state=4(Completed) ofm_stats=[raw_nonzero_bytes=0]"),
            Some(false),
            Some(true),
        );
        assert_eq!(
            evaluate_silicon_pass_evidence(&ev),
            SiliconPassEval::NotPass {
                reason: "ofm_all_zero"
            }
        );
        // ofm missing
        ev = silicon_pass_evidence_from_last_apply(
            Some(true),
            Some("ok_npu_verified"),
            Some("ert_state=4(Completed)"),
            Some(false),
            Some(true),
        );
        assert_eq!(
            evaluate_silicon_pass_evidence(&ev),
            SiliconPassEval::NotPass {
                reason: "ofm_nonzero_missing"
            }
        );
        // no ert completed evidence (exact status only — envelope not enough)
        ev = silicon_pass_evidence_from_last_apply(
            Some(true),
            Some("ok_submit_wait_envelope"),
            Some("ofm_stats=[raw_nonzero_bytes=9]"),
            Some(false),
            Some(true),
        );
        assert_eq!(
            evaluate_silicon_pass_evidence(&ev),
            SiliconPassEval::NotPass {
                reason: "status_not_pass"
            }
        );
    }

    /// P6: SSoT true-Pass criteria checklist always honest + key requirement tokens.
    #[test]
    fn true_pass_criteria_checklist_always_honest() {
        let c = format_true_pass_criteria_checklist();
        for tok in [
            "true_pass_criteria=1",
            "invent_pass=0",
            "not_c_ready=1",
            "not_ran_on_npu=1",
            "product_pass=0",
            "product_pass_requires_live_decide=1",
            "offline_audit_only=1",
            "require_ert_completed_or_ok_npu_verified=1",
            "require_ofm_nonzero_allclose_live=1",
            "require_real_ioctl=1",
            "require_json_ran_on_npu_from_product_path=1",
            "c4_pass_candidate_neq_product_pass=1",
            "userspace_elf_rela_ok_neq_fw_accept=1",
            "fw_accept_neq_product_pass=1",
            "locked_fixture_not_product_pass=1",
            "allclose_only_via_live_decide_ran_on_npu=1",
            "p6_criteria_ready_offline=1",
        ] {
            assert!(
                residual_note_has_exact_token(&c, tok),
                "missing exact token {tok} in checklist: {c}"
            );
        }
        // Never invent product success tokens.
        assert!(!residual_note_has_exact_token(&c, "invent_pass=1"));
        assert!(!residual_note_has_exact_token(&c, "product_pass=1"));
        assert!(!residual_note_has_exact_token(&c, "ran_on_npu=1"));
        assert!(!c.contains("ran_on_npu=true"));
    }

    /// P6 note from NotPass (locked fixture shape) never product Pass.
    #[test]
    fn true_pass_p6_note_not_pass_is_honest() {
        let eval = SiliconPassEval::NotPass {
            reason: "json_ran_on_npu_false",
        };
        let note = format_true_pass_p6_note(&eval);
        assert!(residual_note_has_exact_token(&note, "p6_status=not_pass"));
        assert!(residual_note_has_exact_token(
            &note,
            "reason=json_ran_on_npu_false"
        ));
        assert!(residual_note_has_exact_token(&note, "product_pass=0"));
        assert!(residual_note_has_exact_token(&note, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&note, "not_ran_on_npu=1"));
        assert!(residual_note_has_exact_token(&note, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(
            &note,
            "product_pass_requires_live_decide=1"
        ));
        assert!(residual_note_has_exact_token(&note, "offline_audit_only=1"));
        assert!(!note.contains("product_pass=1"));
        assert!(!note.contains("ran_on_npu=true"));
    }

    /// P6 note from offline PassCandidate still requires live decide (≠ product Pass).
    #[test]
    fn true_pass_p6_note_candidate_still_not_product_pass() {
        let eval = SiliconPassEval::PassCandidate {
            notes: "offline_audit_only=1 product_pass_requires_live_decide=1 invent_pass=0"
                .to_string(),
        };
        let note = format_true_pass_p6_note(&eval);
        assert!(residual_note_has_exact_token(
            &note,
            "p6_status=offline_candidate_only"
        ));
        assert!(residual_note_has_exact_token(&note, "product_pass=0"));
        assert!(residual_note_has_exact_token(
            &note,
            "snapshot_pass_candidate=1"
        ));
        assert!(residual_note_has_exact_token(&note, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&note, "not_ran_on_npu=1"));
        assert!(residual_note_has_exact_token(
            &note,
            "product_pass_requires_live_decide=1"
        ));
        assert!(residual_note_has_exact_token(
            &note,
            "c4_pass_candidate_neq_product_pass=1"
        ));
        // Candidate path reuses C4 evaluator; checklist still distinct SSoT.
        let checklist = format_true_pass_criteria_checklist();
        assert!(residual_note_has_exact_token(
            &checklist,
            "c4_pass_candidate_neq_product_pass=1"
        ));
    }

    /// P6 note from real evaluate path on synthetic candidate stays product_pass=0.
    #[test]
    fn true_pass_p6_note_from_evaluate_candidate() {
        let detail = "EXEC_CMD+WAIT seq=0 ert_state=4(Completed) aiebu_elf=1 rela_count=12; \
            ofm_stats=[raw_len=512 raw_nonzero_bytes=42]; ran_on_npu=true";
        let ev = silicon_pass_evidence_from_last_apply(
            Some(true),
            Some("ok_npu_verified"),
            Some(detail),
            Some(false),
            Some(true),
        );
        let eval = evaluate_silicon_pass_evidence(&ev);
        assert!(eval.is_pass_candidate());
        let note = format_true_pass_p6_note(&eval);
        assert!(residual_note_has_exact_token(
            &note,
            "p6_status=offline_candidate_only"
        ));
        assert!(residual_note_has_exact_token(&note, "product_pass=0"));
        assert!(residual_note_has_exact_token(&note, "invent_pass=0"));
        // C4 note and P6 note agree: offline only.
        let c4 = format_silicon_pass_eval_note(&eval);
        assert!(residual_note_has_exact_token(&c4, "offline_audit_only=1"));
        assert!(residual_note_has_exact_token(&note, "offline_audit_only=1"));
    }

    #[test]
    fn journal_fw_status_pair_c2_live_shape() {
        let j = "\
amdxdna: Status 0x4000003\n\
amdxdna: Failed cmd idx 0, status 0x3000003\n\
amdxdna: wait finished\n";
        let p = parse_journal_fw_status_pair(j);
        assert_eq!(p.failed_cmd_status, Some(0x3_000_003));
        assert_eq!(p.chain_status, Some(0x4_000_003));
        assert!(p.is_c2_app_load_pair());
        let note = p.format_soft_note();
        assert!(note.contains("failed_cmd=0x3000003"));
        assert!(note.contains("failed_cmd_name=APP_LOAD_PDI_FAIL"));
        assert!(note.contains("chain_status=0x4000003"));
        assert!(note.contains("chain_name=AIE2_STATUS_INVALID_PARAM"));
        assert!(note.contains("c2_app_load_pair=1"));
        assert!(note.contains("not_c_ready=1"));
        assert!(note.contains("not_ran_on_npu=1"));
        assert!(residual_note_has_exact_token(&note, "invent_pass=0"));
        // no invent when absent
        let empty = parse_journal_fw_status_pair("hello\n");
        assert!(empty.failed_cmd_status.is_none());
        assert!(empty.chain_status.is_none());
        assert!(!empty.is_c2_app_load_pair());
        assert!(journal_line_has_app_load_hint(
            "Failed cmd idx 0, status 0x3000003"
        ));
        assert!(journal_line_has_app_load_hint("Status 0x4000003"));
        // One-sided journal never invents full pair.
        let only_fail = parse_journal_fw_status_pair("Failed cmd idx 0, status 0x3000003\n");
        assert_eq!(only_fail.failed_cmd_status, Some(0x3_000_003));
        assert!(only_fail.chain_status.is_none());
        assert!(!only_fail.is_c2_app_load_pair());
        let only_chain = parse_journal_fw_status_pair("Status 0x4000003\n");
        assert!(only_chain.failed_cmd_status.is_none());
        assert_eq!(only_chain.chain_status, Some(0x4_000_003));
        assert!(!only_chain.is_c2_app_load_pair());
    }

    #[test]
    fn c2_live_cmdbuf_vs_pure_start_npu_rebuild_research() {
        // Live paddrs from operator C2 last_apply (2026-07-18).
        let ifm = 0x7fda_3b83_5000;
        let wts = 0x7fda_3b66_b000;
        let ofm = 0x7fda_3b83_4000;
        let inst = 0x402_0000;
        let pure =
            rebuild_c2_live_start_npu_cmd_bo(inst, 1688, ifm, wts, ofm, 0x1).expect("rebuild");
        let dec = decode_start_npu_cmd_bo(&pure).expect("decode pure");
        assert_eq!(dec.prop_count, 0, "shim I/O uses prop_count=0");
        assert_eq!(dec.inst_size, 1688);
        assert_eq!(dec.inst_addr, inst);
        assert_eq!(dec.cu_mask, 0x1);
        assert_eq!(dec.flexible_args.len(), 15);
        assert_eq!(dec.flexible_args[0], 3, "xrt3 n_bos low");
        // IFM lo after marker (u64 3, u64 0, u32 0) = index 5
        assert_eq!(dec.flexible_args[5], (ifm & 0xffff_ffff) as u32);
        assert_eq!(dec.flexible_args[6], (ifm >> 32) as u32);

        let live = c2_live_cmdbuf_words_embedded();
        assert_eq!(live.len(), 28);
        assert!(live.contains(&0x698));
        assert_eq!(
            live[12], 0xf,
            "live dump has 0xf at word12 (research marker)"
        );
        assert_eq!(live[1], 0x0402_0000, "instr lo appears early in live dump");
        // Live dump is longer / different framing than pure 21-word BO.
        let pure_words: Vec<u32> = pure
            .chunks_exact(4)
            .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();
        assert_eq!(pure_words.len(), 21); // header + 20 data
        let d = diff_u32_word_lists(&pure_words, live);
        assert!(d.first_mismatch.is_some(), "live dump ≠ pure BO framing");
        assert_ne!(d.left_len, d.right_len);

        let note = format_c2_cmdbuf_research_note(&pure, live);
        assert!(note.contains("envelope_exact_match=0"));
        assert!(note.contains("pure_prop_count=0"));
        assert!(note.contains("live_word12_is_0xf=1"));
        assert!(note.contains("hypothesis_live_prop_count_15_or_framing=1"));
        assert!(note.contains("not_c_ready=1"));
        assert!(note.contains("invent_pass=0"));

        // Tracked fixture must parse to the same words (always required in-tree).
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("testdata/xdna2_cmd/gt_capture/c2_live_cmdbuf_words.txt");
        assert!(
            path.is_file(),
            "tracked fixture missing: {}",
            path.display()
        );
        let text = std::fs::read_to_string(&path).expect("read fixture");
        let from_file = parse_u32_hex_word_list(&text).expect("parse fixture");
        assert_eq!(from_file.as_slice(), live);
    }

    /// Track F — semantic map of live 28w dump vs pure xrt3 rebuild paddrs.
    #[test]
    fn decode_c2_live_cmdbuf_research_semantic_map() {
        let (inst, ifm, wts, ofm, _ctrl) = c2_live_io_paddrs_locked();
        let expect = C2ExpectPaddrs::from_c2_locked_io();
        assert!(expect.is_complete_and_valid());
        let pure =
            rebuild_c2_live_start_npu_cmd_bo(inst, 1688, ifm, wts, ofm, 0x1).expect("rebuild");
        let live = c2_live_cmdbuf_words_embedded();
        assert_eq!(live.len(), C2_LIVE_CMDBUF_WORD_COUNT);

        let map = decode_c2_live_cmdbuf_research(live, Some(&pure), expect);
        assert_eq!(map.live_word_count, 28);
        assert_eq!(map.suspected_framing_extra_words, 7); // 28 − 21
        assert_eq!(map.instr_addr, Some(inst));
        assert_eq!(map.instr_size, Some(0x698));
        assert_eq!(map.word12_marker, Some(0xf));
        assert_eq!(map.n_bos_token, Some(3));
        assert_eq!(map.ifm_paddr, Some(ifm));
        assert_eq!(map.wts_paddr, Some(wts));
        assert_eq!(map.ofm_paddr, Some(ofm));
        assert!(map.logical_paddrs_match_expect);
        assert!(!map.envelope_exact_match, "cmdlist framing ≠ pure BO");
        assert!(!map.pure_bo_unaligned);

        let note = map.format_soft_note();
        assert!(note.contains("c2_live_cmdbuf_map"));
        assert!(note.contains("envelope_exact_match=0"));
        assert!(residual_note_has_exact_token(
            &note,
            "logical_paddrs_match_expect=1"
        ));
        assert!(residual_note_has_exact_token(
            &note,
            "logical_paddrs_match_pure=1"
        ));
        assert!(note.contains("instr_size=0x698"));
        assert!(note.contains("word12_is_0xf=1"));
        assert!(note.contains("do_not_invent_prop_count_15=1"));
        assert!(note.contains("word12_marker_0xf_research=1"));
        assert!(residual_note_has_exact_token(&note, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&note, "not_ran_on_npu=1"));
        assert!(residual_note_has_exact_token(&note, "invent_pass=0"));
        assert!(!residual_note_has_exact_token(&note, "ran_on_npu=1"));
        assert!(note.contains(&format!("ifm=0x{ifm:x}")));
        assert!(note.contains("suspected_framing_extra_words=7"));

        // Convenience wrapper full equality.
        let map2 = decode_c2_live_cmdbuf_vs_pure_rebuild(&pure, expect);
        assert_eq!(map2, map);

        // Wrong expected IFM → logical match fails (no invent).
        let mut bad_expect = expect;
        bad_expect.ifm = Some(0xdead_beef);
        let bad = decode_c2_live_cmdbuf_research(live, Some(&pure), bad_expect);
        assert!(!bad.logical_paddrs_match_expect);

        // Partial expect (only ifm) must not claim full logical match even if ifm ok.
        let partial = C2ExpectPaddrs {
            ifm: Some(ifm),
            ..C2ExpectPaddrs::default()
        };
        let part = decode_c2_live_cmdbuf_research(live, Some(&pure), partial);
        assert!(!part.logical_paddrs_match_expect);
        assert!(!partial.is_complete());

        // Mismatch wts / ofm / instr alone.
        for field in ["instr", "wts", "ofm"] {
            let mut e = expect;
            match field {
                "instr" => e.instr = Some(0x1111_2222),
                "wts" => e.wts = Some(0xdead_beef_cafe),
                "ofm" => e.ofm = Some(0xfeed_face_0000),
                _ => unreachable!(),
            }
            let m = decode_c2_live_cmdbuf_research(live, Some(&pure), e);
            assert!(
                !m.logical_paddrs_match_expect,
                "mismatch {field} must clear full match"
            );
        }

        // Wrong instr size at locked index → size_ok false.
        let mut live_bad_sz: Vec<u32> = live.to_vec();
        live_bad_sz[C2_LIVE_INSTR_SIZE_WORD_IDX] = 0x1000;
        let bad_sz = decode_c2_live_cmdbuf_research(&live_bad_sz, Some(&pure), expect);
        assert!(!bad_sz.logical_paddrs_match_expect);
        assert_eq!(bad_sz.instr_size, Some(0x1000));

        // Invalid live paddr (zero) → field None + no full match even if expect=0.
        let mut live_z: Vec<u32> = live.to_vec();
        live_z[C2_LIVE_IFM_LO_WORD_IDX] = 0;
        live_z[C2_LIVE_IFM_LO_WORD_IDX + 1] = 0;
        let zero_got = decode_c2_live_cmdbuf_research(&live_z, Some(&pure), expect);
        assert!(zero_got.ifm_paddr.is_none());
        assert!(!zero_got.logical_paddrs_match_expect);
        let expect_zero = C2ExpectPaddrs {
            instr: Some(inst),
            ifm: Some(0),
            wts: Some(wts),
            ofm: Some(ofm),
        };
        assert!(!expect_zero.is_complete_and_valid());
        let zmatch = decode_c2_live_cmdbuf_research(&live_z, Some(&pure), expect_zero);
        assert!(!zmatch.logical_paddrs_match_expect);

        // Short dump: no locked fields; no size invent from bare contains.
        let short = &live[..20];
        let sh = decode_c2_live_cmdbuf_research(short, Some(&pure), expect);
        assert!(sh.instr_addr.is_none());
        assert!(sh.instr_size.is_none());
        assert!(sh.ifm_paddr.is_none());
        assert!(!sh.logical_paddrs_match_expect);
        assert!(!sh.envelope_exact_match);

        // Unaligned pure BO → pure_bo_unaligned + envelope false.
        let mut unaligned = pure.clone();
        unaligned.push(0xaa);
        let ua = decode_c2_live_cmdbuf_research(live, Some(&unaligned), expect);
        assert!(ua.pure_bo_unaligned);
        assert!(!ua.envelope_exact_match);

        // Envelope exact match when pure words == live words.
        let live_as_bo: Vec<u8> = live.iter().flat_map(|w| w.to_le_bytes()).collect();
        let env_ok = decode_c2_live_cmdbuf_research(live, Some(&live_as_bo), expect);
        assert!(env_ok.envelope_exact_match);
        assert_eq!(env_ok.suspected_framing_extra_words, 0);
        // Flip one pure word → envelope false; logical may still hold.
        let mut flipped = live_as_bo.clone();
        flipped[0] ^= 0xff;
        let env_bad = decode_c2_live_cmdbuf_research(live, Some(&flipped), expect);
        assert!(!env_bad.envelope_exact_match);
        assert!(env_bad.logical_paddrs_match_expect);

        // No expectations → vacuous logical match stays false.
        let bare = decode_c2_live_cmdbuf_research(live, None, C2ExpectPaddrs::default());
        assert!(!bare.logical_paddrs_match_expect);
        assert!(!bare.envelope_exact_match);
        assert_eq!(bare.instr_addr, Some(inst));
        assert_eq!(bare.instr_size, Some(0x698));
        assert_eq!(bare.ifm_paddr, Some(ifm));
        assert_eq!(bare.suspected_framing_extra_words, 7);
    }

    /// Track G — cmdlist msg size 0x70 locks to 112 B cmdbuf.
    #[test]
    fn c2_live_cmdlist_msg_size_0x70_matches_cmdbuf() {
        let msg = c2_live_cmdlist_msg_words_embedded();
        assert_eq!(msg.len(), 6);
        assert_eq!(msg[4], C2_LIVE_CMDLIST_MSG_SIZE_BYTES);
        assert_eq!(C2_LIVE_CMDLIST_MSG_SIZE_BYTES, 0x70);
        assert_eq!(
            C2_LIVE_CMDLIST_MSG_SIZE_BYTES as usize,
            C2_LIVE_CMDBUF_WORD_COUNT * 4
        );
        assert_eq!(
            C2_LIVE_CMDLIST_MSG_SIZE_BYTES as usize,
            c2_live_cmdbuf_words_embedded().len() * 4
        );
        let note = format_c2_cmdlist_msg_research_note(msg);
        assert!(note.contains("msg_size_field=0x70"));
        assert!(residual_note_has_exact_token(
            &note,
            "msg_size_matches_expected=1"
        ));
        assert!(residual_note_has_exact_token(
            &note,
            "msg_size_matches_cmdbuf=1"
        ));
        assert!(residual_note_has_exact_token(
            &note,
            "expected_cmdlist_msg_size=0x70"
        ));
        assert!(residual_note_has_exact_token(&note, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&note, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&note, "not_ran_on_npu=1"));

        // Wrong size at index 4.
        let mut bad_msg = msg.to_vec();
        bad_msg[4] = 0x40;
        let nbad = format_c2_cmdlist_msg_research_note(&bad_msg);
        assert!(residual_note_has_exact_token(
            &nbad,
            "msg_size_matches_expected=0"
        ));

        // Short / empty msg → size field absent.
        let nempty = format_c2_cmdlist_msg_research_note(&[]);
        assert!(nempty.contains("msg_size_field=absent"));

        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("testdata/xdna2_cmd/gt_capture/c2_live_cmdlist_msg_words.txt");
        assert!(
            path.is_file(),
            "tracked fixture missing: {}",
            path.display()
        );
        let text = std::fs::read_to_string(&path).expect("read");
        let from_file = parse_u32_hex_word_list(&text).expect("parse");
        assert_eq!(from_file.as_slice(), msg);
    }

    /// Track H — empty / non-aiebu / invalid paddr + staged or synthetic ELF.
    #[test]
    fn audit_aiebu_rela_with_c2_live_paddrs_offline() {
        let empty = audit_aiebu_rela_with_live_paddrs(&[], 1, 2, 3, 4);
        assert!(!empty.elf_present);
        assert!(!empty.patched_ctrltext_walk_ok);
        let en = empty.format_soft_note();
        assert!(residual_note_has_exact_token(&en, "elf_present=0"));
        assert!(residual_note_has_exact_token(&en, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&en, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&en, "not_ran_on_npu=1"));

        // Non-aiebu non-empty bytes.
        let garbage = audit_aiebu_rela_with_live_paddrs(b"not-elf", 1, 2, 3, 4);
        assert!(garbage.elf_present);
        assert!(!garbage.patched_ctrltext_walk_ok);
        assert_eq!(garbage.detail, "not_aiebu_elf_ctrltext");
        let gn = garbage.format_soft_note();
        assert!(gn.contains("not_aiebu_elf_ctrltext"));
        assert!(!gn.contains("userspace_patch_ok_fw_load_still_open=1"));
        assert!(residual_note_has_exact_token(&gn, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&gn, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&gn, "not_ran_on_npu=1"));

        let (_inst, ifm, wts, ofm, ctrl) = c2_live_io_paddrs_locked();

        // Synthetic minimal aiebu ELF — always CI-meaningful (patch+walk path).
        let ctrltext = build_strix_research_noop_txn().expect("noop txn");
        let relas = [(0_u32, (1_u32 << 8) | 5, 0_i32)];
        let synth = build_minimal_test_elf32(&ctrltext, &relas);
        assert!(is_aiebu_elf_ctrltext(&synth));
        // Zero paddr refuse (any of ifm/wts/ofm/ctrl).
        let inv = audit_aiebu_rela_with_live_paddrs(&synth, 0, wts, ofm, ctrl);
        assert!(inv.elf_present);
        assert!(!inv.patched_ctrltext_walk_ok);
        assert_eq!(inv.detail, "invalid_live_paddr");
        // u64::MAX is also invalid (same refuse path as zero).
        let inv_max = audit_aiebu_rela_with_live_paddrs(&synth, ifm, wts, ofm, u64::MAX);
        assert!(inv_max.elf_present);
        assert!(!inv_max.patched_ctrltext_walk_ok);
        assert_eq!(inv_max.detail, "invalid_live_paddr");
        // Valid paddrs → patch path runs (walk may fail on noop txn — still not skip).
        let a_syn = audit_aiebu_rela_with_live_paddrs(&synth, ifm, wts, ofm, ctrl);
        assert!(a_syn.elf_present);
        assert_ne!(a_syn.detail.as_str(), "invalid_live_paddr");
        // One crafted rela — must not soft-skip with rela_count=0 / empty detail.
        assert_eq!(a_syn.rela_count, Some(1));
        assert!(
            a_syn.detail.starts_with("classic_walk_ok")
                || a_syn.detail.starts_with("walk_failed")
                || a_syn.detail.starts_with("patch_failed"),
            "expected post-parse detail, got {:?}",
            a_syn.detail
        );
        let sn = a_syn.format_soft_note();
        assert!(residual_note_has_exact_token(&sn, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&sn, "not_ran_on_npu=1"));
        assert!(!residual_note_has_exact_token(&sn, "ran_on_npu=1"));
        assert!(residual_note_has_exact_token(
            &sn,
            "userspace_elf_rela_ok=1"
        ));
        assert!(residual_note_has_exact_token(&sn, "rela_count=1"));

        // Optional full staged mladf only (silicon_lab / gt_capture) — never the
        // 1072 B research stub (`testdata/xdna2_cmd/lab_instr.elf`, rela×1).
        // Align with Track K soft-skip: gate ×12/1688 before hard assert.
        let paths = [
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("testdata/xdna2_cmd/silicon_lab/lab_instr.elf"),
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("testdata/xdna2_cmd/gt_capture/instr_pre.elf"),
        ];
        let mut ran_staged = false;
        for p in &paths {
            if !p.is_file() {
                continue;
            }
            let Ok(elf) = std::fs::read(p) else {
                continue;
            };
            if !is_aiebu_elf_ctrltext(&elf) {
                continue;
            }
            let a = audit_aiebu_rela_with_live_paddrs(&elf, ifm, wts, ofm, ctrl);
            // Gate: only lock ×12/1688 when candidate is full staged mladf.
            if a.rela_count != Some(12) || a.patched_ctrltext_len != Some(1688) {
                eprintln!(
                    "OFFLINE_ASSET_SKIP: aiebu at {} not staged mladf (rela={:?} len={:?})",
                    p.display(),
                    a.rela_count,
                    a.patched_ctrltext_len
                );
                continue;
            }
            assert!(a.elf_present);
            assert!(
                a.patched_ctrltext_walk_ok,
                "classic walk should pass offline: {}",
                a.detail
            );
            let n = a.format_soft_note();
            assert!(residual_note_has_exact_token(&n, "userspace_elf_rela_ok=1"));
            assert!(residual_note_has_exact_token(&n, "userspace_rela_ok=1"));
            assert!(residual_note_has_exact_token(
                &n,
                "userspace_patch_ok_fw_load_still_open=1"
            ));
            assert!(residual_note_has_exact_token(&n, "not_ran_on_npu=1"));
            assert!(residual_note_has_exact_token(&n, "not_c_ready=1"));
            assert!(residual_note_has_exact_token(&n, "invent_pass=0"));
            assert!(!residual_note_has_exact_token(&n, "ran_on_npu=1"));
            ran_staged = true;
            break;
        }
        if !ran_staged {
            eprintln!(
                "OFFLINE_ASSET_SKIP: no full staged mladf aiebu ELF (synthetic path still locked)"
            );
        }
        // Require staged only when env is explicitly truthy (`1` / `true`), same
        // convention as `XDNA2_LAB_PRINT_C2_CMDBUF_RESEARCH` — presence of `0`/`false`
        // must NOT force the hard assert.
        if env_flag_truthy_os("XDNA2_REQUIRE_STAGED_ELF") {
            assert!(
                ran_staged,
                "XDNA2_REQUIRE_STAGED_ELF=1|true but no full staged mladf ELF found"
            );
        }
    }

    /// Track I — checklist optional journal dual codes; absent stays honest.
    #[test]
    fn app_load_checklist_journal_dual_optional() {
        let sig = LastApplyDetailSignals {
            aiebu_elf: Some(true),
            rela_count: Some(12),
            aiebu_rela_patched: true,
            ert_error: true,
            instr_bo_type: Some("Dev".into()),
            instr_map_offset: Some(u64::MAX),
            content_size_b: Some(1688),
            start_npu_size_b: Some(1688),
            bo_size_b: Some(4096),
            ofm_raw_nonzero_bytes: Some(0),
            submit_wait_not_ok: true,
            fw_status_code: None, // detail lacks hex — common last_apply shape
            ..LastApplyDetailSignals::default()
        };
        let class = SiliconResidualClass::AppLoadPdiFailAfterAiebu;

        // No journal → absent honesty (and fw_code_absent from detail).
        let ck = format_app_load_offline_checklist(class, &sig);
        assert!(residual_note_has_exact_token(
            &ck,
            "fw_code_absent_in_detail=1"
        ));
        assert!(residual_note_has_exact_token(&ck, "journal_text_absent=1"));
        assert!(!residual_note_has_exact_token(
            &ck,
            "journal_c2_app_load_pair=1"
        ));
        assert!(!ck.contains("journal_c2_app_load_pair=")); // omit pair when absent
        assert!(residual_note_has_exact_token(&ck, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&ck, "not_ran_on_npu=1"));
        assert!(residual_note_has_exact_token(&ck, "invent_pass=0"));
        assert!(!residual_note_has_exact_token(&ck, "ran_on_npu=1"));

        let j = "\
amdxdna: Status 0x4000003\n\
amdxdna: Failed cmd idx 0, status 0x3000003\n";
        let ck_j = format_app_load_offline_checklist_ex(class, &sig, Some(j));
        assert!(residual_note_has_exact_token(
            &ck_j,
            "journal_failed_cmd=0x3000003"
        ));
        assert!(ck_j.contains("journal_failed_cmd_name=APP_LOAD_PDI_FAIL"));
        assert!(residual_note_has_exact_token(
            &ck_j,
            "journal_chain_status=0x4000003"
        ));
        assert!(ck_j.contains("journal_chain_name=AIE2_STATUS_INVALID_PARAM"));
        assert!(residual_note_has_exact_token(
            &ck_j,
            "journal_c2_app_load_pair=1"
        ));
        assert!(residual_note_has_exact_token(
            &ck_j,
            "fw_code_absent_in_detail=1"
        )); // detail still lacks
        assert!(!residual_note_has_exact_token(
            &ck_j,
            "journal_text_absent=1"
        ));
        assert!(residual_note_has_exact_token(&ck_j, "not_ran_on_npu=1"));
        assert!(residual_note_has_exact_token(&ck_j, "invent_pass=0"));
        assert!(!residual_note_has_exact_token(&ck_j, "ran_on_npu=1"));
        // Empty journal string treated as absent.
        let ck_e = format_app_load_offline_checklist_ex(class, &sig, Some("  \n"));
        assert!(residual_note_has_exact_token(
            &ck_e,
            "journal_text_absent=1"
        ));
        // No invent pair from unrelated text.
        let ck_u = format_app_load_offline_checklist_ex(class, &sig, Some("hello world\n"));
        assert!(residual_note_has_exact_token(
            &ck_u,
            "journal_failed_cmd=absent"
        ));
        assert!(residual_note_has_exact_token(
            &ck_u,
            "journal_chain_status=absent"
        ));
        assert!(residual_note_has_exact_token(
            &ck_u,
            "journal_c2_app_load_pair=0"
        ));
        // One-sided: Failed cmd only → pair=0, chain absent.
        let ck_one = format_app_load_offline_checklist_ex(
            class,
            &sig,
            Some("Failed cmd idx 0, status 0x3000003\n"),
        );
        assert!(residual_note_has_exact_token(
            &ck_one,
            "journal_failed_cmd=0x3000003"
        ));
        assert!(residual_note_has_exact_token(
            &ck_one,
            "journal_chain_status=absent"
        ));
        assert!(residual_note_has_exact_token(
            &ck_one,
            "journal_c2_app_load_pair=0"
        ));
        // One-sided: chain Status only.
        let ck_ch = format_app_load_offline_checklist_ex(class, &sig, Some("Status 0x4000003\n"));
        assert!(residual_note_has_exact_token(
            &ck_ch,
            "journal_failed_cmd=absent"
        ));
        assert!(residual_note_has_exact_token(
            &ck_ch,
            "journal_chain_status=0x4000003"
        ));
        assert!(residual_note_has_exact_token(
            &ck_ch,
            "journal_c2_app_load_pair=0"
        ));
    }

    /// Track K — pure SHA-256 FIPS vectors (empty, abc, multi-block) + oversize refuse.
    #[test]
    fn sha256_hex_lower_fips_vectors_and_oversize() {
        assert_eq!(
            sha256_hex_lower(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex_lower(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        // Multi-block FIPS-180-2 vector (56 bytes → padding crosses block).
        let multi = b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq";
        assert_eq!(
            sha256_hex_lower(multi),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
        assert_eq!(sha256_digest(b"abc").map(|d| d.len()), Some(32));
        // Oversize refuse at public boundary (no multi-GiB clone).
        let over = vec![0_u8; (MAX_INSTRUCTION_BLOB_BYTES as usize).saturating_add(1)];
        assert!(sha256_digest(&over).is_none());
        assert!(sha256_hex_lower(&over).is_empty());
    }

    #[test]
    fn txn_op_opcode_profile_table() {
        use super::super::txn::XaieTxnOp;
        let empty = TxnOpOpcodeProfile::from_ops(&[]);
        assert_eq!(empty.total, 0);
        assert_eq!(
            empty.format_soft_tokens(),
            "op_total=0 op_write=0 op_block_write=0 op_block_set=0 op_mask_write=0 op_mask_poll=0 op_noop=0 op_preempt=0 op_mask_poll_busy=0 op_custom_or_other=0"
        );
        let mut ops = Vec::new();
        for op in 0_u8..=7 {
            ops.push(XaieTxnOp {
                opcode: op,
                byte_offset: 0,
                byte_len: 4,
            });
        }
        ops.push(XaieTxnOp {
            opcode: 128,
            byte_offset: 0,
            byte_len: 4,
        });
        let p = TxnOpOpcodeProfile::from_ops(&ops);
        assert_eq!(p.total, 9);
        assert_eq!(p.write, 1);
        assert_eq!(p.block_write, 1);
        assert_eq!(p.block_set, 1);
        assert_eq!(p.mask_write, 1);
        assert_eq!(p.mask_poll, 1);
        assert_eq!(p.noop, 1);
        assert_eq!(p.preempt, 1);
        assert_eq!(p.mask_poll_busy, 1);
        assert_eq!(p.custom_or_other, 1);
        assert_eq!(
            p.format_soft_tokens(),
            "op_total=9 op_write=1 op_block_write=1 op_block_set=1 op_mask_write=1 op_mask_poll=1 op_noop=1 op_preempt=1 op_mask_poll_busy=1 op_custom_or_other=1"
        );
        // Gap opcode 8 also custom_or_other.
        let gap = TxnOpOpcodeProfile::from_ops(&[XaieTxnOp {
            opcode: 8,
            byte_offset: 0,
            byte_len: 1,
        }]);
        assert_eq!(gap.custom_or_other, 1);
        assert_eq!(gap.total, 1);
    }

    #[test]
    fn profile_instr_payload_bytes_branches_and_honesty() {
        let empty = profile_instr_payload_bytes(&[]);
        assert_eq!(empty.blob_kind, "empty");
        assert!(!empty.classic_walk_ok);
        let en = empty.format_soft_note();
        assert!(residual_note_has_exact_token(&en, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&en, "not_ran_on_npu=1"));
        assert!(residual_note_has_exact_token(&en, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(
            &en,
            "pdi_marker_public=marker_absent_public"
        ));

        let noop = build_strix_research_noop_txn().expect("noop");
        let p = profile_instr_payload_bytes(&noop);
        assert_eq!(p.blob_kind, "classic_txn_v01");
        assert!(p.classic_walk_ok);
        assert!(p.payload_len > 0);
        assert_eq!(p.payload_sha256_hex.len(), 64);
        assert_eq!(p.payload_sha256_hex, sha256_hex_lower(&noop));
        let n = p.format_soft_note();
        assert!(residual_note_has_exact_token(&n, "not_ran_on_npu=1"));
        assert!(residual_note_has_exact_token(&n, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&n, "not_c_ready=1"));
        assert!(!residual_note_has_exact_token(&n, "ran_on_npu=1"));
        assert!(n.contains("op_total="));
        assert!(!p.staged_mladf_classic_profile);

        // unknown_raw short non-txn
        let raw = profile_instr_payload_bytes(&[0xde, 0xad, 0xbe, 0xef]);
        assert_eq!(raw.blob_kind, "unknown_raw");
        assert!(!raw.classic_walk_ok);
        let rn = raw.format_soft_note();
        assert!(residual_note_has_exact_token(&rn, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&rn, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&rn, "not_ran_on_npu=1"));

        // aie2_elf_ctrltext_unpeeled when full ELF slipped into profile
        let ctrltext = build_strix_research_noop_txn().expect("noop");
        let relas = [(0_u32, (1_u32 << 8) | 5, 0_i32)];
        let elf = build_minimal_test_elf32(&ctrltext, &relas);
        assert!(is_aiebu_elf_ctrltext(&elf));
        let unp = profile_instr_payload_bytes(&elf);
        assert_eq!(unp.blob_kind, "aie2_elf_ctrltext_unpeeled");
        assert!(!unp.classic_walk_ok);
        let un = unp.format_soft_note();
        assert!(residual_note_has_exact_token(&un, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&un, "not_c_ready=1"));

        // oversized_refused
        let over = vec![0_u8; (MAX_INSTRUCTION_BLOB_BYTES as usize).saturating_add(1)];
        let o = profile_instr_payload_bytes(&over);
        assert_eq!(o.blob_kind, "oversized_refused");
        assert!(o.payload_sha256_hex.is_empty());
        let on = o.format_soft_note();
        assert!(residual_note_has_exact_token(&on, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&on, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&on, "not_ran_on_npu=1"));

        // txn_header_only: buffer_len == txn_size but op walk fails (garbage body).
        // XAie layout: major/minor/dev_gen/rows/cols/mem @0..6; num_ops @8; txn_size @12.
        let mut hdr_only = vec![0_u8; 32];
        hdr_only[0] = 0; // major
        hdr_only[1] = 1; // minor (v0.1)
        hdr_only[2] = 4; // dev_gen
        hdr_only[3] = 8; // num_rows
        hdr_only[4] = 6; // num_cols
        hdr_only[5] = 0; // num_mem_tile_rows
        hdr_only[8..12].copy_from_slice(&1_u32.to_le_bytes()); // num_ops=1
        hdr_only[12..16].copy_from_slice(&32_u32.to_le_bytes()); // txn_size == len
        // body[16..] left zeros → walk fails → TxnHeaderOnly
        let ho = profile_instr_payload_bytes(&hdr_only);
        assert_eq!(ho.blob_kind, "txn_header_only", "detail payload={ho:?}");
        assert!(!ho.classic_walk_ok);
        assert!(!ho.staged_mladf_classic_profile);
        let hn = ho.format_soft_note();
        assert!(residual_note_has_exact_token(
            &hn,
            "blob_kind=txn_header_only"
        ));
        assert!(residual_note_has_exact_token(&hn, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&hn, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&hn, "not_ran_on_npu=1"));
    }

    #[test]
    fn profile_instr_payload_after_rela_ci_mandatory_userspace_ok() {
        // Empty / garbage / invalid paddr honesty.
        let empty = profile_instr_payload_after_rela(&[], 1, 2, 3, 4);
        assert!(!empty.userspace_ok_fw_load_open);
        assert!(empty.payload.is_none());
        let en = empty.format_soft_note();
        assert!(residual_note_has_exact_token(
            &en,
            "userspace_ok_fw_load_open=0"
        ));
        assert!(residual_note_has_exact_token(
            &en,
            "userspace_patch_ok_fw_load_still_open=0"
        ));
        assert!(residual_note_has_exact_token(&en, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&en, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&en, "not_ran_on_npu=1"));

        let garbage = profile_instr_payload_after_rela_c2_locked(b"not-elf");
        assert!(!garbage.userspace_ok_fw_load_open);
        assert_eq!(garbage.detail, "not_aiebu_elf_ctrltext");
        let gn = garbage.format_soft_note();
        assert!(residual_note_has_exact_token(&gn, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&gn, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&gn, "not_ran_on_npu=1"));
        assert!(residual_note_has_exact_token(
            &gn,
            "userspace_ok_fw_load_open=0"
        ));

        let (_i, _ifm, wts, ofm, ctrl) = c2_live_io_paddrs_locked();
        let ctrltext = build_strix_research_noop_txn().expect("noop");
        let relas = [(0_u32, (1_u32 << 8) | 5, 0_i32)];
        let synth = build_minimal_test_elf32(&ctrltext, &relas);
        let inv = profile_instr_payload_after_rela(&synth, 0, wts, ofm, ctrl);
        assert!(!inv.userspace_ok_fw_load_open);
        assert_eq!(inv.detail, "invalid_live_paddr");

        // CI-mandatory: synthetic ELF + 1 rela + walkable noop ctrltext → userspace_ok=1.
        let a_syn = profile_instr_payload_after_rela_c2_locked(&synth);
        assert!(a_syn.rela_audit.elf_present);
        assert!(
            a_syn.userspace_ok_fw_load_open,
            "synthetic noop+rela must lock userspace_ok without staged asset: detail={}",
            a_syn.detail
        );
        assert_eq!(a_syn.rela_audit.rela_count, Some(1));
        assert!(a_syn.rela_audit.patched_ctrltext_walk_ok);
        let pay = a_syn.payload.as_ref().expect("payload after single patch");
        assert!(pay.classic_walk_ok);
        assert_eq!(pay.blob_kind, "classic_txn_v01");
        assert_eq!(pay.payload_sha256_hex.len(), 64);
        // Exact soft tokens from fields (no weak contains OR).
        let sn = a_syn.format_soft_note();
        assert!(residual_note_has_exact_token(
            &sn,
            "userspace_ok_fw_load_open=1"
        ));
        // Track H alias = walk_ok alone (same 1 here because walk OK).
        assert!(residual_note_has_exact_token(
            &sn,
            "userspace_patch_ok_fw_load_still_open=1"
        ));
        assert!(residual_note_has_exact_token(&sn, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&sn, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&sn, "not_ran_on_npu=1"));
        assert!(residual_note_has_exact_token(
            &sn,
            &format!("payload_sha256={}", pay.payload_sha256_hex)
        ));
        assert!(!pay.staged_mladf_classic_profile);
        // Single-patch: rela_audit holds same bytes as profiled.
        assert_eq!(
            a_syn.rela_audit.patched_ctrltext.as_ref().map(|b| b.len()),
            Some(pay.payload_len)
        );

        // Optional full staged mladf (silicon_lab / gt_capture only — not 1072 B stub).
        let paths = [
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("testdata/xdna2_cmd/silicon_lab/lab_instr.elf"),
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("testdata/xdna2_cmd/gt_capture/instr_pre.elf"),
        ];
        let mut ran = false;
        for pth in &paths {
            if !pth.is_file() {
                continue;
            }
            let elf = match std::fs::read(pth) {
                Ok(e) => e,
                Err(_) => continue,
            };
            if !is_aiebu_elf_ctrltext(&elf) {
                continue;
            }
            let r = profile_instr_payload_after_rela_c2_locked(&elf);
            // Gate: only lock ×12/1688 when candidate is full staged mladf.
            if r.rela_audit.rela_count != Some(12)
                || r.rela_audit.patched_ctrltext_len != Some(1688)
            {
                eprintln!(
                    "OFFLINE_ASSET_SKIP: aiebu at {} not staged mladf (rela={:?} len={:?})",
                    pth.display(),
                    r.rela_audit.rela_count,
                    r.rela_audit.patched_ctrltext_len
                );
                continue;
            }
            assert!(r.userspace_ok_fw_load_open, "detail={}", r.detail);
            let pay = r.payload.as_ref().expect("payload");
            assert_eq!(pay.payload_len, 1688);
            assert!(pay.classic_walk_ok);
            assert!(pay.staged_mladf_classic_profile);
            assert_eq!(pay.txn_num_ops, Some(40));
            assert_eq!(pay.blob_kind, "classic_txn_v01");
            assert_eq!(pay.payload_sha256_hex.len(), 64);
            let op = pay.op_profile.as_ref().expect("ops");
            assert_eq!(op.total, 40);
            let n = r.format_soft_note();
            assert!(residual_note_has_exact_token(
                &n,
                "userspace_ok_fw_load_open=1"
            ));
            assert!(residual_note_has_exact_token(&n, "not_ran_on_npu=1"));
            assert!(residual_note_has_exact_token(&n, "invent_pass=0"));
            assert!(residual_note_has_exact_token(&n, "not_c_ready=1"));
            assert!(residual_note_has_exact_token(
                &n,
                "staged_mladf_classic_profile=1"
            ));
            assert!(residual_note_has_exact_token(
                &n,
                &format!("payload_sha256={}", pay.payload_sha256_hex)
            ));
            assert!(residual_note_has_exact_token(&n, "op_total=40"));
            let r2 = profile_instr_payload_after_rela_c2_locked(&elf);
            assert_eq!(
                r.payload.as_ref().unwrap().payload_sha256_hex,
                r2.payload.as_ref().unwrap().payload_sha256_hex
            );
            ran = true;
            break;
        }
        if !ran {
            eprintln!(
                "OFFLINE_ASSET_SKIP: no full staged mladf aiebu ELF (synthetic userspace_ok still locked)"
            );
        }
        if env_flag_truthy_os("XDNA2_REQUIRE_STAGED_ELF") {
            assert!(
                ran,
                "XDNA2_REQUIRE_STAGED_ELF requires full staged mladf ELF for Track K"
            );
        }
    }

    // -----------------------------------------------------------------------
    // Construction path research (lab instr-only vs public CU/PDI + CONFIG_CU)
    // -----------------------------------------------------------------------

    #[test]
    fn construction_path_empty_elf_honesty_and_gap() {
        let r = research_construction_path_lab_vs_public(&[]);
        assert!(!r.lab_has_instr_ctrltext);
        assert!(!r.lab_has_cu_pdi_config, "lab never CONFIG_CU");
        assert!(r.public_stack_requires_cu_pdi_step);
        assert!(
            r.construction_gap_cu_pdi,
            "gap stays true even when ELF empty (public requires, lab lacks)"
        );
        assert_eq!(r.detail, "elf_empty_skip");
        let n = r.format_soft_note();
        assert!(residual_note_has_exact_token(
            &n,
            "construction_path_research"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "construction_gap_cu_pdi=1"
        ));
        assert!(residual_note_has_exact_token(&n, "lab_has_instr=0"));
        assert!(residual_note_has_exact_token(&n, "lab_has_cu_pdi_config=0"));
        assert!(residual_note_has_exact_token(
            &n,
            "public_stack_cu_pdi_step=1"
        ));
        assert!(residual_note_has_exact_token(&n, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&n, "not_ran_on_npu=1"));
        assert!(residual_note_has_exact_token(&n, "invent_pass=0"));
        // Soft note must never claim product Pass (exact tokens only — substring
        // `ran_on_npu=1` would false-positive match honesty `not_ran_on_npu=1`).
        assert!(!n.contains("ran_on_npu=true"));
        assert!(!residual_note_has_exact_token(&n, "ran_on_npu=1"));
        assert!(!residual_note_has_exact_token(&n, "ran_on_npu=true"));
        assert!(!residual_note_has_exact_token(&n, "invent_pass=1"));
        assert!(!residual_note_has_exact_token(&n, "c_ready=1"));
    }

    #[test]
    fn construction_path_garbage_and_synthetic_noop() {
        let g = research_construction_path_lab_vs_public(b"not-elf");
        assert!(!g.lab_has_instr_ctrltext);
        assert!(g.construction_gap_cu_pdi);
        assert!(!g.lab_has_cu_pdi_config);
        assert_eq!(g.detail, "not_aiebu_elf_ctrltext");
        // Non-ELF must not claim rela_count=0 (reads as empty rela table).
        assert!(g.lab_rela_count.is_none());
        let gn = g.format_soft_note();
        assert!(residual_note_has_exact_token(&gn, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&gn, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&gn, "not_ran_on_npu=1"));
        assert!(!gn.contains("ran_on_npu=true"));
        assert!(!gn.contains("lab_rela_count="));

        // Synthetic aiebu + rela + walkable classic ctrltext → lab instr ok, gap still open.
        let ctrltext = build_strix_research_noop_txn().expect("noop");
        let relas = [(0_u32, (1_u32 << 8) | 5, 0_i32)];
        let synth = build_minimal_test_elf32(&ctrltext, &relas);
        let s = research_construction_path_lab_vs_public(&synth);
        assert!(
            s.lab_has_instr_ctrltext,
            "synthetic ELF must lock lab_has_instr: detail={}",
            s.detail
        );
        assert!(!s.lab_has_cu_pdi_config);
        assert!(s.public_stack_requires_cu_pdi_step);
        assert!(s.construction_gap_cu_pdi);
        assert_eq!(s.lab_rela_count, Some(1));
        assert!(s.lab_ctrltext_len.is_some_and(|n| n > 0));
        let sn = s.format_soft_note();
        assert!(residual_note_has_exact_token(&sn, "lab_has_instr=1"));
        assert!(residual_note_has_exact_token(
            &sn,
            "construction_gap_cu_pdi=1"
        ));
        assert!(residual_note_has_exact_token(
            &sn,
            "lab_has_cu_pdi_config=0"
        ));
        assert!(residual_note_has_exact_token(
            &sn,
            "public_stack_cu_pdi_step=1"
        ));
        assert!(residual_note_has_exact_token(&sn, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&sn, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&sn, "not_ran_on_npu=1"));
        assert!(!sn.contains("ran_on_npu=true"));
        assert!(!residual_note_has_exact_token(&sn, "invent_pass=1"));
    }

    #[test]
    fn construction_path_classic_walk_ok_no_rela() {
        // aiebu + classic-walkable ctrltext but empty rela table → not lab-instr-ready.
        let ctrltext = build_strix_research_noop_txn().expect("noop");
        let synth = build_minimal_test_elf32(&ctrltext, &[]);
        let r = research_construction_path_lab_vs_public(&synth);
        assert!(
            !r.lab_has_instr_ctrltext,
            "zero rela must not claim lab instr ready: detail={}",
            r.detail
        );
        assert_eq!(r.detail, "classic_walk_ok_no_rela");
        assert_eq!(r.lab_rela_count, Some(0));
        assert!(r.lab_ctrltext_len.is_some_and(|n| n > 0));
        assert!(!r.lab_has_cu_pdi_config);
        assert!(r.construction_gap_cu_pdi);
        assert!(r.public_stack_requires_cu_pdi_step);
        let n = r.format_soft_note();
        assert!(residual_note_has_exact_token(&n, "lab_has_instr=0"));
        assert!(residual_note_has_exact_token(
            &n,
            "construction_gap_cu_pdi=1"
        ));
        assert!(residual_note_has_exact_token(&n, "lab_has_cu_pdi_config=0"));
        assert!(residual_note_has_exact_token(&n, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&n, "not_ran_on_npu=1"));
        assert!(residual_note_has_exact_token(&n, "invent_pass=0"));
        assert!(!n.contains("ran_on_npu=true"));
        assert!(!residual_note_has_exact_token(&n, "invent_pass=1"));
    }

    // -----------------------------------------------------------------------
    // CU/PDI necessity residual matrix (ctrltext-only vs CONFIG_CU identity)
    // -----------------------------------------------------------------------

    #[test]
    fn cu_pdi_necessity_gap_open_fw_residual_honesty() {
        let construction = research_construction_path_lab_vs_public(&[]);
        let r = research_cu_pdi_necessity_for_app_load(&construction, true, false);
        assert!(r.lab_is_ctrltext_only);
        assert!(r.public_has_cu_pdi_step);
        assert!(r.construction_gap_open);
        assert!(r.fw_name_app_load_pdi_fail);
        assert!(!r.external_ctrltext_sha_match);
        assert!(
            !r.ctrltext_match_alone_closes_fw_residual,
            "match alone never closes residual"
        );
        assert!(
            r.cu_pdi_identity_may_be_required,
            "open hypothesis when gap+fw residual"
        );
        assert_eq!(r.detail, "gap_open_fw_app_load");
        let n = r.format_soft_note();
        assert!(residual_note_has_exact_token(
            &n,
            "cu_pdi_necessity_research"
        ));
        assert!(residual_note_has_exact_token(&n, "lab_is_ctrltext_only=1"));
        assert!(residual_note_has_exact_token(&n, "construction_gap_open=1"));
        assert!(residual_note_has_exact_token(
            &n,
            "fw_name_app_load_pdi_fail=1"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "cu_pdi_identity_may_be_required=1"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "ctrltext_match_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(&n, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&n, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&n, "not_ran_on_npu=1"));
        assert!(residual_note_has_exact_token(&n, "product_known_good=0"));
        assert!(!residual_note_has_exact_token(&n, "invent_pass=1"));
        assert!(!residual_note_has_exact_token(&n, "ran_on_npu=1"));
        assert!(!residual_note_has_exact_token(&n, "c_ready=1"));
    }

    #[test]
    fn cu_pdi_necessity_sha_match_still_open_residual() {
        // Even when research external ctrltext sha matches lab export, FW residual
        // + construction gap stay open — match ≠ Pass.
        let construction = research_construction_path_lab_vs_public(&[]);
        let r = research_cu_pdi_necessity_for_app_load(&construction, true, true);
        assert!(r.external_ctrltext_sha_match);
        assert!(!r.ctrltext_match_alone_closes_fw_residual);
        assert!(r.cu_pdi_identity_may_be_required);
        assert_eq!(r.detail, "sha_match_fw_residual_still_open");
        let n = r.format_soft_note();
        assert!(residual_note_has_exact_token(
            &n,
            "external_ctrltext_sha_match=1"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "ctrltext_match_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(&n, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&n, "product_known_good=0"));
        assert!(residual_note_has_exact_token(&n, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&n, "not_ran_on_npu=1"));
        // Soft note hard-forces match_alone=0 even if a hand-built bag mutates field.
        let mut mutated = r.clone();
        mutated.ctrltext_match_alone_closes_fw_residual = true;
        let mn = mutated.format_soft_note();
        assert!(
            residual_note_has_exact_token(&mn, "ctrltext_match_alone_closes_fw_residual=0"),
            "format_soft_note must force match_alone=0: {mn}"
        );
        assert!(!residual_note_has_exact_token(
            &mn,
            "ctrltext_match_alone_closes_fw_residual=1"
        ));

        // Locked convenience for C2 residual matrix.
        let locked = research_cu_pdi_necessity_c2_locked(true);
        assert!(locked.fw_name_app_load_pdi_fail);
        assert!(locked.cu_pdi_identity_may_be_required);
        assert!(!locked.ctrltext_match_alone_closes_fw_residual);
        let ln = locked.format_soft_note();
        assert!(residual_note_has_exact_token(&ln, "invent_pass=0"));
        assert!(!residual_note_has_exact_token(&ln, "ran_on_npu=1"));
    }

    #[test]
    fn cu_pdi_necessity_no_fw_flag_not_required_claim() {
        // Gap open but no FW residual flag → may_be_required stays false
        // (do not invent APP_LOAD from construction gap alone).
        let construction = research_construction_path_lab_vs_public(&[]);
        let r = research_cu_pdi_necessity_for_app_load(&construction, false, false);
        assert!(r.construction_gap_open);
        assert!(!r.fw_name_app_load_pdi_fail);
        assert!(!r.cu_pdi_identity_may_be_required);
        assert_eq!(r.detail, "gap_open_no_fw_flag");
        let n = r.format_soft_note();
        assert!(residual_note_has_exact_token(
            &n,
            "cu_pdi_identity_may_be_required=0"
        ));
        assert!(residual_note_has_exact_token(&n, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&n, "not_c_ready=1"));
    }

    // -----------------------------------------------------------------------
    // APP_LOAD residual axis matrix (partition vs CU/PDI vs ctrltext-only)
    // -----------------------------------------------------------------------

    #[test]
    fn app_load_residual_axis_matrix_c2_locked_honesty() {
        // Locked C2: FW 0x3000003 name + construction gap + ctrltext-only lab.
        let m = research_app_load_residual_axis_matrix_c2_locked(false);
        assert!(m.axis_fw_status_name_app_load);
        assert!(m.axis_partition_surface_status_name_only);
        assert!(m.axis_lab_ctrltext_only);
        assert!(m.axis_public_cu_pdi_step);
        assert!(m.axis_construction_gap_open);
        assert!(m.axis_cu_pdi_identity_hypothesis_open);
        assert!(!m.axis_external_ctrltext_sha_match);
        assert!(!m.ctrltext_match_alone_closes_fw_residual);
        assert!(!m.partition_name_alone_proves_root_cause);
        assert_eq!(m.detail, "gap_open_fw_app_load_axes");
        let n = m.format_soft_note();
        assert!(residual_note_has_exact_token(
            &n,
            "app_load_residual_axis_matrix"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "axis_fw_status_name_app_load=1"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "axis_partition_surface_status_name_only=1"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "axis_lab_ctrltext_only=1"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "axis_construction_gap_open=1"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "axis_cu_pdi_identity_hypothesis_open=1"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "ctrltext_match_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "partition_name_alone_proves_root_cause=0"
        ));
        // FW axis open → emit locked C2 code/name (not invent from gap alone).
        assert!(residual_note_has_exact_token(&n, "fw_code=0x3000003"));
        assert!(residual_note_has_exact_token(
            &n,
            "fw_name=APP_LOAD_PDI_FAIL"
        ));
        assert!(!residual_note_has_exact_token(&n, "fw_code=absent"));
        assert!(residual_note_has_exact_token(
            &n,
            "partition_surface_public=status_name_only"
        ));
        assert!(residual_note_has_exact_token(&n, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&n, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&n, "not_ran_on_npu=1"));
        assert!(residual_note_has_exact_token(&n, "product_known_good=0"));
        // Contradictory invent tokens must not appear.
        assert!(!residual_note_has_exact_token(&n, "invent_pass=1"));
        assert!(!residual_note_has_exact_token(&n, "ran_on_npu=1"));
        assert!(!residual_note_has_exact_token(&n, "c_ready=1"));
        assert!(!residual_note_has_exact_token(
            &n,
            "ctrltext_match_alone_closes_fw_residual=1"
        ));
        assert!(!residual_note_has_exact_token(
            &n,
            "partition_name_alone_proves_root_cause=1"
        ));
    }

    #[test]
    fn app_load_residual_axis_matrix_sha_match_still_open() {
        // Research external sha match never closes residual axes.
        let m = research_app_load_residual_axis_matrix_c2_locked(true);
        assert!(m.axis_external_ctrltext_sha_match);
        assert!(!m.ctrltext_match_alone_closes_fw_residual);
        assert!(!m.partition_name_alone_proves_root_cause);
        assert!(m.axis_cu_pdi_identity_hypothesis_open);
        assert_eq!(m.detail, "sha_match_fw_residual_axes_open");
        let n = m.format_soft_note();
        assert!(residual_note_has_exact_token(
            &n,
            "axis_external_ctrltext_sha_match=1"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "ctrltext_match_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(&n, "product_known_good=0"));
        // Soft note forces locks even if bag fields are mutated.
        let mut mutated = m.clone();
        mutated.ctrltext_match_alone_closes_fw_residual = true;
        mutated.partition_name_alone_proves_root_cause = true;
        mutated.axis_partition_surface_status_name_only = false;
        let mn = mutated.format_soft_note();
        assert!(residual_note_has_exact_token(
            &mn,
            "ctrltext_match_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(
            &mn,
            "partition_name_alone_proves_root_cause=0"
        ));
        // Surface token always forced =1 in soft notes (Issue 5).
        assert!(residual_note_has_exact_token(
            &mn,
            "axis_partition_surface_status_name_only=1"
        ));
        assert!(!residual_note_has_exact_token(
            &mn,
            "axis_partition_surface_status_name_only=0"
        ));
        assert!(!residual_note_has_exact_token(
            &mn,
            "ctrltext_match_alone_closes_fw_residual=1"
        ));
    }

    #[test]
    fn app_load_residual_axis_matrix_no_fw_flag_not_required() {
        // Gap open without FW residual → hypothesis closed (do not invent APP_LOAD).
        let partition = AppLoadPdiPartitionResearch::default();
        let construction = research_construction_path_lab_vs_public(&[]);
        let necessity = research_cu_pdi_necessity_for_app_load(&construction, false, false);
        let m = research_app_load_residual_axis_matrix(&partition, &necessity);
        assert!(!m.axis_fw_status_name_app_load);
        assert!(m.axis_construction_gap_open);
        assert!(!m.axis_cu_pdi_identity_hypothesis_open);
        assert_eq!(m.detail, "gap_open_no_fw_flag");
        let n = m.format_soft_note();
        assert!(residual_note_has_exact_token(
            &n,
            "axis_cu_pdi_identity_hypothesis_open=0"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "axis_fw_status_name_app_load=0"
        ));
        // Honesty: never invent FW residual-class narrative when axis is off.
        assert!(residual_note_has_exact_token(&n, "fw_code=absent"));
        assert!(!residual_note_has_exact_token(&n, "fw_code=0x3000003"));
        assert!(!residual_note_has_exact_token(
            &n,
            "fw_name=APP_LOAD_PDI_FAIL"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "axis_partition_surface_status_name_only=1"
        ));
        assert!(residual_note_has_exact_token(&n, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&n, "not_c_ready=1"));
    }

    // -----------------------------------------------------------------------
    // Xclbin extract research dry-run (public markers; VE2 refuse)
    // -----------------------------------------------------------------------

    #[test]
    fn xclbin_magic_and_section_markers_public_only() {
        assert!(xclbin_magic_is_xclbin2(XCLBIN2_MAGIC));
        assert!(xclbin_magic_is_xclbin2(b"xclbin2\0rest"));
        assert!(!xclbin_magic_is_xclbin2(b""));
        assert!(!xclbin_magic_is_xclbin2(b"xclbin1\0"));
        assert!(!xclbin_magic_is_xclbin2(b"ELF"));

        let info = "Sections: BITSTREAM, MEM_TOPOLOGY, IP_LAYOUT, AIE_PARTITION\n";
        let markers = scan_xclbin_section_markers_from_text(info);
        assert!(markers.contains(&"AIE_PARTITION"));
        assert!(markers.contains(&"BITSTREAM"));
        assert!(markers.contains(&"MEM_TOPOLOGY"));
        assert_eq!(scan_xclbin_section_markers_from_text("   ").len(), 0);
        assert_eq!(
            scan_xclbin_section_markers_from_text("no sections").len(),
            0
        );
    }

    #[test]
    fn xclbin_extract_ve2_fixture_refused_not_known_good() {
        // VE2 edge validate.xclbin is NOT_AIE2 — never AIE2 external known-good.
        let header = *XCLBIN2_MAGIC;
        let info = "\
xclbin Information
   Sections:               BITSTREAM, MEM_TOPOLOGY, IP_LAYOUT, CONNECTIVITY,
                           BUILD_METADATA, EMBEDDED_METADATA, SYSTEM_METADATA,
                           GROUP_CONNECTIVITY, GROUP_TOPOLOGY, AIE_PARTITION
   Platform VBNV:          xilinx_v1_ipu_0_0
";
        let r = research_xclbin_extract_ve2_fixture_refused(&header, Some(info));
        assert!(r.magic_xclbin2);
        assert!(r.has_aie_partition_marker);
        assert!(r.section_marker_count >= 5);
        assert!(r.platform_not_aie2_known_good);
        assert!(
            !r.eligible_as_aie2_external_known_good,
            "VE2 must never be AIE2 known-good"
        );
        assert!(r.do_not_invent_pdi_layout);
        assert!(!r.public_pdi_image_decode);
        assert_eq!(r.detail, "refuse_not_aie2_or_ve2");
        let n = r.format_soft_note();
        assert!(residual_note_has_exact_token(
            &n,
            "xclbin_extract_research_dry_run"
        ));
        assert!(residual_note_has_exact_token(&n, "magic_xclbin2=1"));
        assert!(residual_note_has_exact_token(
            &n,
            "platform_not_aie2_known_good=1"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "eligible_as_aie2_external_known_good=0"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "do_not_invent_pdi_layout=1"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "public_pdi_image_decode=0"
        ));
        assert!(residual_note_has_exact_token(&n, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&n, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&n, "not_ran_on_npu=1"));
        assert!(residual_note_has_exact_token(&n, "product_known_good=0"));
        assert!(!residual_note_has_exact_token(&n, "invent_pass=1"));
        assert!(!residual_note_has_exact_token(
            &n,
            "eligible_as_aie2_external_known_good=1"
        ));
        assert!(!residual_note_has_exact_token(&n, "product_known_good=1"));

        // Claim helpers: case-insensitive refuse; carve2 must not false-positive.
        assert!(platform_claim_is_not_aie2_known_good(
            "NOT_AIE2_KNOWN_GOOD=1"
        ));
        assert!(platform_claim_is_not_aie2_known_good("PLATFORM=VE2_EDGE"));
        assert!(platform_claim_is_not_aie2_known_good(
            "testdata/ve2_latency/x.xclbin"
        ));
        assert!(platform_claim_is_not_aie2_known_good("ve2"));
        assert!(!platform_claim_is_not_aie2_known_good("carve2_board"));
        assert!(!platform_claim_is_not_aie2_known_good(""));
        assert!(!platform_claim_is_working_public_aie2_research(
            "PLATFORM=VE2_EDGE xrt"
        ));
    }

    #[test]
    fn xclbin_extract_working_public_research_eligible_not_pass() {
        // Positive research path: magic + AIE_PARTITION + working public claim.
        // Soft still product_known_good=0 invent_pass=0 (eligible ≠ Pass).
        let header = *XCLBIN2_MAGIC;
        let info = "Sections: BITSTREAM, AIE_PARTITION, IP_LAYOUT\n";
        let r = research_xclbin_extract_dry_run(
            &header,
            Some(info),
            Some("Source stack: working public XRT / DynamicDispatch AIE2 npu4"),
        );
        assert!(r.magic_xclbin2);
        assert!(r.has_aie_partition_marker);
        assert!(!r.platform_not_aie2_known_good);
        assert!(r.eligible_as_aie2_external_known_good);
        assert_eq!(r.detail, "research_eligible_not_product_pass");
        let n = r.format_soft_note();
        assert!(residual_note_has_exact_token(
            &n,
            "eligible_as_aie2_external_known_good=1"
        ));
        assert!(residual_note_has_exact_token(&n, "product_known_good=0"));
        assert!(residual_note_has_exact_token(&n, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&n, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&n, "not_ran_on_npu=1"));
        // Mutated decode/invent fields must not leak into soft note.
        let mut mutated = r.clone();
        mutated.do_not_invent_pdi_layout = false;
        mutated.public_pdi_image_decode = true;
        let mn = mutated.format_soft_note();
        assert!(residual_note_has_exact_token(
            &mn,
            "do_not_invent_pdi_layout=1"
        ));
        assert!(residual_note_has_exact_token(
            &mn,
            "public_pdi_image_decode=0"
        ));
        assert!(!residual_note_has_exact_token(
            &mn,
            "public_pdi_image_decode=1"
        ));

        // Magic alone without working claim → not eligible.
        let await_claim = research_xclbin_extract_dry_run(&header, Some(info), None);
        assert!(!await_claim.eligible_as_aie2_external_known_good);
        assert_eq!(
            await_claim.detail,
            "markers_ok_awaiting_working_stack_claim"
        );

        // Bare single-token "xrt" / "aie2" without working/source-stack → not eligible.
        assert!(!platform_claim_is_working_public_aie2_research("xrt"));
        assert!(!platform_claim_is_working_public_aie2_research("aie2"));
        assert!(!platform_claim_is_working_public_aie2_research("npu4"));
        let bare_xrt = research_xclbin_extract_dry_run(&header, Some(info), Some("xrt"));
        assert!(
            !bare_xrt.eligible_as_aie2_external_known_good,
            "bare xrt must not over-flip eligibility"
        );
        assert_eq!(bare_xrt.detail, "markers_ok_awaiting_working_stack_claim");
        // Working marker without AIE2-ish whole-token also incomplete.
        assert!(!platform_claim_is_working_public_aie2_research(
            "working public only"
        ));
    }

    #[test]
    fn xclbin_extract_refuse_edges_magic_and_empty() {
        // Non-xclbin2 header → magic_not_xclbin2; never eligible.
        let bad = research_xclbin_extract_dry_run(
            b"ELF\0xxxx",
            Some("Sections: AIE_PARTITION, BITSTREAM\n"),
            Some("Source stack: working public XRT AIE2 npu4"),
        );
        assert!(!bad.magic_xclbin2);
        assert!(!bad.eligible_as_aie2_external_known_good);
        assert_eq!(bad.detail, "magic_not_xclbin2");
        let n = bad.format_soft_note();
        assert!(residual_note_has_exact_token(&n, "magic_xclbin2=0"));
        assert!(residual_note_has_exact_token(
            &n,
            "eligible_as_aie2_external_known_good=0"
        ));
        assert!(!residual_note_has_exact_token(
            &n,
            "eligible_as_aie2_external_known_good=1"
        ));
        assert!(residual_note_has_exact_token(&n, "product_known_good=0"));
        assert!(residual_note_has_exact_token(&n, "invent_pass=0"));

        // Empty inputs → default_no_input; all eligibility false.
        let empty = research_xclbin_extract_dry_run(&[], None, None);
        assert!(!empty.magic_xclbin2);
        assert_eq!(empty.section_marker_count, 0);
        assert!(!empty.has_aie_partition_marker);
        assert!(!empty.eligible_as_aie2_external_known_good);
        assert_eq!(empty.detail, "default_no_input");
        let en = empty.format_soft_note();
        assert!(residual_note_has_exact_token(&en, "magic_xclbin2=0"));
        assert!(residual_note_has_exact_token(
            &en,
            "eligible_as_aie2_external_known_good=0"
        ));
        assert!(!residual_note_has_exact_token(
            &en,
            "eligible_as_aie2_external_known_good=1"
        ));

        // Default bag soft note matches empty-input honesty.
        let def = XclbinExtractResearchDryRun::default();
        assert_eq!(def.detail, "default_no_input");
        assert!(!def.eligible_as_aie2_external_known_good);
    }

    // -----------------------------------------------------------------------
    // Pure CU/PDI section identity (public section names only; stamp 7a5ab864)
    // -----------------------------------------------------------------------

    #[test]
    fn cu_pdi_section_kind_classify_public_markers() {
        assert_eq!(
            classify_xclbin_section_kind("AIE_PARTITION"),
            XclbinSectionKind::CuPdiIdentity
        );
        assert_eq!(
            classify_xclbin_section_kind("BITSTREAM"),
            XclbinSectionKind::CuPdiIdentity
        );
        assert_eq!(
            classify_xclbin_section_kind("MEM_TOPOLOGY"),
            XclbinSectionKind::Topology
        );
        assert_eq!(
            classify_xclbin_section_kind("IP_LAYOUT"),
            XclbinSectionKind::Topology
        );
        assert_eq!(
            classify_xclbin_section_kind("CONNECTIVITY"),
            XclbinSectionKind::Topology
        );
        assert_eq!(
            classify_xclbin_section_kind("BUILD_METADATA"),
            XclbinSectionKind::Metadata
        );
        assert_eq!(
            classify_xclbin_section_kind("SYSTEM_METADATA"),
            XclbinSectionKind::Metadata
        );
        assert_eq!(classify_xclbin_section_kind(""), XclbinSectionKind::Unknown);
        assert_eq!(
            classify_xclbin_section_kind("NOT_A_SECTION"),
            XclbinSectionKind::Unknown
        );
        // Case-sensitive public tokens (xclbinutil style).
        assert_eq!(
            classify_xclbin_section_kind("aie_partition"),
            XclbinSectionKind::Unknown
        );
        // Identity table is exactly AIE_PARTITION + BITSTREAM.
        assert_eq!(XCLBIN_CU_PDI_IDENTITY_SECTION_MARKERS.len(), 2);
        assert!(XCLBIN_CU_PDI_IDENTITY_SECTION_MARKERS.contains(&"AIE_PARTITION"));
        assert!(XCLBIN_CU_PDI_IDENTITY_SECTION_MARKERS.contains(&"BITSTREAM"));
        // Trimmed padded names classify the same as exact table tokens.
        assert_eq!(
            classify_xclbin_section_kind(" AIE_PARTITION "),
            XclbinSectionKind::CuPdiIdentity
        );
        assert_eq!(
            classify_xclbin_section_kind("  BITSTREAM\t"),
            XclbinSectionKind::CuPdiIdentity
        );
    }

    #[test]
    fn cu_pdi_section_kind_tables_union_equals_research_scan() {
        // Kind tables partition the public research scan set (no drift).
        let mut union: Vec<&str> = Vec::new();
        union.extend_from_slice(XCLBIN_CU_PDI_IDENTITY_SECTION_MARKERS);
        union.extend_from_slice(XCLBIN_TOPOLOGY_SECTION_MARKERS);
        union.extend_from_slice(XCLBIN_METADATA_SECTION_MARKERS);
        union.sort_unstable();
        let mut scan: Vec<&str> = XCLBIN_RESEARCH_SECTION_MARKERS.to_vec();
        scan.sort_unstable();
        assert_eq!(
            union, scan,
            "union of kind tables must equal XCLBIN_RESEARCH_SECTION_MARKERS"
        );
        // Pairwise disjoint.
        for a in XCLBIN_CU_PDI_IDENTITY_SECTION_MARKERS {
            assert!(!XCLBIN_TOPOLOGY_SECTION_MARKERS.contains(a));
            assert!(!XCLBIN_METADATA_SECTION_MARKERS.contains(a));
        }
        for a in XCLBIN_TOPOLOGY_SECTION_MARKERS {
            assert!(!XCLBIN_METADATA_SECTION_MARKERS.contains(a));
            assert!(!XCLBIN_CU_PDI_IDENTITY_SECTION_MARKERS.contains(a));
        }
        for a in XCLBIN_METADATA_SECTION_MARKERS {
            assert!(!XCLBIN_TOPOLOGY_SECTION_MARKERS.contains(a));
            assert!(!XCLBIN_CU_PDI_IDENTITY_SECTION_MARKERS.contains(a));
        }
        // Every research marker has a non-Unknown kind.
        for m in XCLBIN_RESEARCH_SECTION_MARKERS {
            assert_ne!(
                classify_xclbin_section_kind(m),
                XclbinSectionKind::Unknown,
                "research marker {m} must classify"
            );
        }
    }

    #[test]
    fn cu_pdi_section_identity_from_info_text_not_pass() {
        let info = "\
Sections: BITSTREAM, MEM_TOPOLOGY, IP_LAYOUT, CONNECTIVITY,
          BUILD_METADATA, AIE_PARTITION
";
        let r = research_cu_pdi_section_identity_from_text(info);
        assert!(r.has_aie_partition);
        assert!(r.has_bitstream);
        assert!(r.has_cu_pdi_identity_surface);
        assert_eq!(r.cu_pdi_identity_marker_count, 2);
        assert!(r.topology_marker_count >= 2);
        assert!(r.metadata_marker_count >= 1);
        assert!(!r.section_identity_alone_closes_fw_residual);
        assert!(r.do_not_invent_pdi_layout);
        assert!(!r.public_pdi_image_decode);
        assert_eq!(r.detail, "cu_pdi_identity_surface_aie_and_bitstream");
        let n = r.format_soft_note();
        assert!(residual_note_has_exact_token(
            &n,
            "cu_pdi_section_identity_research"
        ));
        assert!(residual_note_has_exact_token(&n, "has_aie_partition=1"));
        assert!(residual_note_has_exact_token(&n, "has_bitstream=1"));
        assert!(residual_note_has_exact_token(
            &n,
            "has_cu_pdi_identity_surface=1"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "section_identity_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "do_not_invent_pdi_layout=1"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "public_pdi_image_decode=0"
        ));
        assert!(residual_note_has_exact_token(&n, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&n, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&n, "not_ran_on_npu=1"));
        assert!(residual_note_has_exact_token(&n, "product_known_good=0"));
        // Mutated invent/close fields must not leak into soft note.
        let mut mutated = r.clone();
        mutated.section_identity_alone_closes_fw_residual = true;
        mutated.do_not_invent_pdi_layout = false;
        mutated.public_pdi_image_decode = true;
        let mn = mutated.format_soft_note();
        assert!(residual_note_has_exact_token(
            &mn,
            "section_identity_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(
            &mn,
            "do_not_invent_pdi_layout=1"
        ));
        assert!(residual_note_has_exact_token(
            &mn,
            "public_pdi_image_decode=0"
        ));
        assert!(!residual_note_has_exact_token(
            &mn,
            "section_identity_alone_closes_fw_residual=1"
        ));
        assert!(!residual_note_has_exact_token(
            &mn,
            "public_pdi_image_decode=1"
        ));
        assert!(!residual_note_has_exact_token(&mn, "invent_pass=1"));
        assert!(!residual_note_has_exact_token(&mn, "product_known_good=1"));

        // Topology-only → no CU/PDI identity surface.
        let topo = research_cu_pdi_section_identity_from_text(
            "Sections: MEM_TOPOLOGY, IP_LAYOUT, CONNECTIVITY\n",
        );
        assert!(!topo.has_cu_pdi_identity_surface);
        assert!(!topo.has_aie_partition);
        assert!(!topo.has_bitstream);
        assert_eq!(topo.detail, "topology_or_metadata_only_no_cu_pdi_identity");
        let tn = topo.format_soft_note();
        assert!(residual_note_has_exact_token(
            &tn,
            "has_cu_pdi_identity_surface=0"
        ));
        assert!(residual_note_has_exact_token(
            &tn,
            "section_identity_alone_closes_fw_residual=0"
        ));

        // Empty / default honesty.
        let empty = research_cu_pdi_section_identity_from_text("   ");
        assert!(!empty.has_cu_pdi_identity_surface);
        assert_eq!(empty.detail, "default_no_markers");
        let def = CuPdiSectionIdentityResearch::default();
        assert_eq!(def.detail, "default_no_markers");
        assert!(!def.section_identity_alone_closes_fw_residual);

        // AIE_PARTITION alone still counts as identity surface (name only).
        let aie_only = research_cu_pdi_section_identity_from_markers(&["AIE_PARTITION"]);
        assert!(aie_only.has_cu_pdi_identity_surface);
        assert_eq!(
            aie_only.detail,
            "cu_pdi_identity_surface_aie_partition_only"
        );
        let bs_only = research_cu_pdi_section_identity_from_markers(&["BITSTREAM"]);
        assert!(bs_only.has_cu_pdi_identity_surface);
        assert_eq!(bs_only.detail, "cu_pdi_identity_surface_bitstream_only");

        // Padded markers: classify trims; identity flags must use the same trim
        // (public from_markers API consistency).
        let padded = research_cu_pdi_section_identity_from_markers(&[
            " AIE_PARTITION ",
            "\tBITSTREAM ",
            " MEM_TOPOLOGY ",
        ]);
        assert!(padded.has_aie_partition);
        assert!(padded.has_bitstream);
        assert!(padded.has_cu_pdi_identity_surface);
        assert_eq!(padded.cu_pdi_identity_marker_count, 2);
        assert_eq!(padded.topology_marker_count, 1);
        assert_eq!(padded.detail, "cu_pdi_identity_surface_aie_and_bitstream");
        let pn = padded.format_soft_note();
        assert!(residual_note_has_exact_token(&pn, "has_aie_partition=1"));
        assert!(residual_note_has_exact_token(&pn, "has_bitstream=1"));
        assert!(residual_note_has_exact_token(
            &pn,
            "has_cu_pdi_identity_surface=1"
        ));
        assert!(residual_note_has_exact_token(
            &pn,
            "section_identity_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(&pn, "product_known_good=0"));
    }

    // -----------------------------------------------------------------------
    // Pure BITSTREAM / AIE_PARTITION section-offset map (stamp a5787723)
    // -----------------------------------------------------------------------

    /// Build a minimal synthetic xclbin2 prefix with section headers only.
    ///
    /// Layout matches public `struct axlf` / `axlf_section_header` (LE, natural
    /// align). Section **bodies** are not included — research header walk only.
    fn synth_axlf_with_sections(sections: &[(u32, &str, u64, u64)]) -> Vec<u8> {
        #[allow(clippy::cast_possible_truncation)]
        let n = u32::try_from(sections.len()).unwrap_or(u32::MAX);
        let total =
            AXLF_FIRST_SECTION_HEADER_ABS_OFFSET + sections.len() * AXLF_SECTION_HEADER_SIZE;
        let mut buf = vec![0_u8; total];
        // magic
        buf[..8].copy_from_slice(XCLBIN2_MAGIC);
        // m_signature_length = -1
        buf[8..12].copy_from_slice(&(-1_i32).to_le_bytes());
        // m_numSections
        let ns_off = AXLF_NUM_SECTIONS_ABS_OFFSET;
        buf[ns_off..ns_off + 4].copy_from_slice(&n.to_le_bytes());
        // section headers
        let mut off = AXLF_FIRST_SECTION_HEADER_ABS_OFFSET;
        for &(kind, name, soff, ssize) in sections {
            buf[off..off + 4].copy_from_slice(&kind.to_le_bytes());
            let nb = name.as_bytes();
            let take = nb.len().min(16);
            buf[off + 4..off + 4 + take].copy_from_slice(&nb[..take]);
            // pad 4 at 20..24 already zero
            buf[off + 24..off + 32].copy_from_slice(&soff.to_le_bytes());
            buf[off + 32..off + 40].copy_from_slice(&ssize.to_le_bytes());
            off += AXLF_SECTION_HEADER_SIZE;
        }
        buf
    }

    #[test]
    fn xclbin_section_offset_map_layout_constants_public() {
        // Public header sizes / offsets stay stable for research walk.
        assert_eq!(AXLF_SECTION_HEADER_SIZE, 40);
        assert_eq!(AXLF_HEADER_SIZE, 152);
        assert_eq!(AXLF_M_HEADER_ABS_OFFSET, 304);
        assert_eq!(AXLF_NUM_SECTIONS_ABS_OFFSET, 448);
        assert_eq!(AXLF_FIRST_SECTION_HEADER_ABS_OFFSET, 456);
        assert_eq!(AXLF_SECTION_KIND_BITSTREAM, 0);
        assert_eq!(AXLF_SECTION_KIND_AIE_PARTITION, 32);
        assert_eq!(axlf_section_kind_public_name(0), Some("BITSTREAM"));
        assert_eq!(axlf_section_kind_public_name(32), Some("AIE_PARTITION"));
        assert_eq!(axlf_section_kind_public_name(6), Some("MEM_TOPOLOGY"));
        assert_eq!(axlf_section_kind_public_name(999), None);
    }

    #[test]
    fn xclbin_section_offset_map_parse_header_fields() {
        // Manual 40-byte header: kind=32, name=aie_partition, off=0x100, size=0x40
        let mut raw = [0_u8; 40];
        raw[0..4].copy_from_slice(&32_u32.to_le_bytes());
        raw[4..4 + 13].copy_from_slice(b"aie_partition");
        raw[24..32].copy_from_slice(&0x100_u64.to_le_bytes());
        raw[32..40].copy_from_slice(&0x40_u64.to_le_bytes());
        let h = parse_axlf_section_header(&raw).expect("40-byte header");
        assert_eq!(h.kind, AXLF_SECTION_KIND_AIE_PARTITION);
        assert_eq!(h.name, "aie_partition");
        assert_eq!(h.offset, 0x100);
        assert_eq!(h.size, 0x40);
        assert!(parse_axlf_section_header(&raw[..39]).is_none());
        assert!(parse_axlf_section_header(&[]).is_none());
    }

    #[test]
    fn xclbin_section_offset_map_synth_bitstream_and_aie_not_pass() {
        let bytes = synth_axlf_with_sections(&[
            (AXLF_SECTION_KIND_BITSTREAM, "behav", 0x400, 0x1000),
            (6, "", 0x1400, 88),
            (
                AXLF_SECTION_KIND_AIE_PARTITION,
                "aie_partition",
                0x2000,
                416,
            ),
        ]);
        assert!(xclbin_magic_is_xclbin2(&bytes));
        assert_eq!(parse_axlf_num_sections(&bytes), Some(3));
        let headers = walk_axlf_section_headers(&bytes);
        assert_eq!(headers.len(), 3);
        assert_eq!(headers[0].kind, AXLF_SECTION_KIND_BITSTREAM);
        assert_eq!(headers[0].offset, 0x400);
        assert_eq!(headers[0].size, 0x1000);
        assert_eq!(headers[2].kind, AXLF_SECTION_KIND_AIE_PARTITION);
        assert_eq!(headers[2].offset, 0x2000);
        assert_eq!(headers[2].size, 416);

        let r = research_xclbin_cu_pdi_section_offset_map(&bytes);
        assert!(r.magic_xclbin2);
        assert_eq!(r.num_sections, Some(3));
        assert_eq!(r.section_headers_parsed, 3);
        assert!(r.has_bitstream_section);
        assert!(r.has_aie_partition_section);
        assert!(r.has_cu_pdi_offset_surface);
        assert_eq!(r.bitstream_offset, Some(0x400));
        assert_eq!(r.bitstream_size, Some(0x1000));
        assert_eq!(r.aie_partition_offset, Some(0x2000));
        assert_eq!(r.aie_partition_size, Some(416));
        assert!(!r.section_offset_map_alone_closes_fw_residual);
        assert!(r.do_not_invent_pdi_layout);
        assert!(!r.public_pdi_image_decode);
        assert_eq!(
            r.detail,
            "cu_pdi_offset_surface_bitstream_and_aie_partition"
        );
        let n = r.format_soft_note();
        assert!(residual_note_has_exact_token(
            &n,
            "cu_pdi_section_offset_map_research"
        ));
        assert!(residual_note_has_exact_token(&n, "magic_xclbin2=1"));
        assert!(residual_note_has_exact_token(&n, "num_sections=3"));
        assert!(residual_note_has_exact_token(&n, "bitstream_offset=1024"));
        assert!(residual_note_has_exact_token(&n, "bitstream_size=4096"));
        assert!(residual_note_has_exact_token(
            &n,
            "aie_partition_offset=8192"
        ));
        assert!(residual_note_has_exact_token(&n, "aie_partition_size=416"));
        assert!(residual_note_has_exact_token(
            &n,
            "has_cu_pdi_offset_surface=1"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "section_offset_map_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "do_not_invent_pdi_layout=1"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "public_pdi_image_decode=0"
        ));
        assert!(residual_note_has_exact_token(&n, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&n, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&n, "not_ran_on_npu=1"));
        assert!(residual_note_has_exact_token(&n, "product_known_good=0"));
        // Mutated invent/close fields must not leak into soft note.
        let mut mutated = r.clone();
        mutated.section_offset_map_alone_closes_fw_residual = true;
        mutated.do_not_invent_pdi_layout = false;
        mutated.public_pdi_image_decode = true;
        let mn = mutated.format_soft_note();
        assert!(residual_note_has_exact_token(
            &mn,
            "section_offset_map_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(
            &mn,
            "do_not_invent_pdi_layout=1"
        ));
        assert!(residual_note_has_exact_token(
            &mn,
            "public_pdi_image_decode=0"
        ));
        assert!(!residual_note_has_exact_token(
            &mn,
            "section_offset_map_alone_closes_fw_residual=1"
        ));
        assert!(!residual_note_has_exact_token(&mn, "invent_pass=1"));
        assert!(!residual_note_has_exact_token(&mn, "product_known_good=1"));
    }

    #[test]
    fn xclbin_section_offset_map_empty_magic_truncated_not_pass() {
        let empty = research_xclbin_cu_pdi_section_offset_map(&[]);
        assert!(!empty.magic_xclbin2);
        assert_eq!(empty.detail, "default_empty_bytes");
        assert!(!empty.has_cu_pdi_offset_surface);
        let en = empty.format_soft_note();
        assert!(residual_note_has_exact_token(
            &en,
            "bitstream_offset=absent"
        ));
        assert!(residual_note_has_exact_token(
            &en,
            "aie_partition_offset=absent"
        ));
        assert!(residual_note_has_exact_token(
            &en,
            "section_offset_map_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(&en, "product_known_good=0"));

        let bad = research_xclbin_cu_pdi_section_offset_map(b"ELF\0notxclbin");
        assert!(!bad.magic_xclbin2);
        assert_eq!(bad.detail, "magic_not_xclbin2");
        assert!(!bad.has_cu_pdi_offset_surface);

        // Magic only — header truncated before num_sections.
        let mut magic_only = vec![0_u8; 32];
        magic_only[..8].copy_from_slice(XCLBIN2_MAGIC);
        let trunc = research_xclbin_cu_pdi_section_offset_map(&magic_only);
        assert!(trunc.magic_xclbin2);
        assert_eq!(trunc.detail, "header_truncated");
        assert!(trunc.num_sections.is_none());

        // Magic + num_sections but no room for section headers.
        let mut hdr_only = vec![0_u8; AXLF_FIRST_SECTION_HEADER_ABS_OFFSET];
        hdr_only[..8].copy_from_slice(XCLBIN2_MAGIC);
        let ns_off = AXLF_NUM_SECTIONS_ABS_OFFSET;
        hdr_only[ns_off..ns_off + 4].copy_from_slice(&2_u32.to_le_bytes());
        let no_sec = research_xclbin_cu_pdi_section_offset_map(&hdr_only);
        assert!(no_sec.magic_xclbin2);
        assert_eq!(no_sec.num_sections, Some(2));
        assert_eq!(no_sec.section_headers_parsed, 0);
        assert_eq!(no_sec.detail, "section_headers_truncated");

        // Topology-only sections → no CU/PDI offset surface.
        let topo = synth_axlf_with_sections(&[(6, "", 0x100, 8), (8, "", 0x200, 16)]);
        let t = research_xclbin_cu_pdi_section_offset_map(&topo);
        assert!(!t.has_cu_pdi_offset_surface);
        assert_eq!(t.detail, "no_cu_pdi_section_headers");
        assert!(residual_note_has_exact_token(
            &t.format_soft_note(),
            "has_cu_pdi_offset_surface=0"
        ));

        // BITSTREAM alone still counts as offset surface (header only).
        let bs = synth_axlf_with_sections(&[(AXLF_SECTION_KIND_BITSTREAM, "behav", 100, 50)]);
        let b = research_xclbin_cu_pdi_section_offset_map(&bs);
        assert!(b.has_bitstream_section);
        assert!(!b.has_aie_partition_section);
        assert!(b.has_cu_pdi_offset_surface);
        assert_eq!(b.detail, "cu_pdi_offset_surface_bitstream_only");

        let def = CuPdiSectionOffsetMapResearch::default();
        assert_eq!(def.detail, "default_empty_bytes");
        assert!(!def.section_offset_map_alone_closes_fw_residual);
    }

    #[test]
    fn xclbin_section_offset_map_aie_partition_only_not_pass() {
        // Unit-lock detail branch that synth both/bitstream-only paths skip.
        let bytes = synth_axlf_with_sections(&[(
            AXLF_SECTION_KIND_AIE_PARTITION,
            "aie_partition",
            0x3000,
            416,
        )]);
        let r = research_xclbin_cu_pdi_section_offset_map(&bytes);
        assert!(r.magic_xclbin2);
        assert_eq!(r.num_sections, Some(1));
        assert_eq!(r.section_headers_parsed, 1);
        assert!(!r.has_bitstream_section);
        assert!(r.has_aie_partition_section);
        assert!(r.has_cu_pdi_offset_surface);
        assert_eq!(r.bitstream_offset, None);
        assert_eq!(r.bitstream_size, None);
        assert_eq!(r.aie_partition_offset, Some(0x3000));
        assert_eq!(r.aie_partition_size, Some(416));
        assert_eq!(r.detail, "cu_pdi_offset_surface_aie_partition_only");
        assert!(!r.section_offset_map_alone_closes_fw_residual);
        assert!(r.do_not_invent_pdi_layout);
        assert!(!r.public_pdi_image_decode);
        let n = r.format_soft_note();
        assert!(residual_note_has_exact_token(&n, "has_bitstream_section=0"));
        assert!(residual_note_has_exact_token(
            &n,
            "has_aie_partition_section=1"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "has_cu_pdi_offset_surface=1"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "aie_partition_offset=12288"
        ));
        assert!(residual_note_has_exact_token(&n, "aie_partition_size=416"));
        assert!(residual_note_has_exact_token(&n, "bitstream_offset=absent"));
        assert!(residual_note_has_exact_token(
            &n,
            "section_offset_map_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(&n, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&n, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&n, "not_ran_on_npu=1"));
        assert!(residual_note_has_exact_token(&n, "product_known_good=0"));
        // Soft force-lock survives mutated close/invent fields.
        let mut mutated = r.clone();
        mutated.section_offset_map_alone_closes_fw_residual = true;
        mutated.do_not_invent_pdi_layout = false;
        let mn = mutated.format_soft_note();
        assert!(residual_note_has_exact_token(
            &mn,
            "section_offset_map_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(
            &mn,
            "do_not_invent_pdi_layout=1"
        ));
        assert!(!residual_note_has_exact_token(
            &mn,
            "section_offset_map_alone_closes_fw_residual=1"
        ));
    }

    #[test]
    fn xclbin_section_offset_map_ve2_fixture_when_present_not_aie2_known_good() {
        // VE2 validate.xclbin is a real public axlf — parse offsets for research
        // only. Always NOT product known-good / NOT AIE2 bag (platform refuse).
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("testdata/xdna2_cmd/ve2_latency/sources/ve2_runner_latency_validate.xclbin");
        if !path.is_file() {
            eprintln!("OFFLINE_ASSET_SKIP: no ve2 validate.xclbin");
            return;
        }
        // Read only the header + section table (not full 9 MiB body).
        let mut f = match std::fs::File::open(&path) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("OFFLINE_ASSET_SKIP: open failed: {e}");
                return;
            }
        };
        use std::io::Read;
        // 456 + 256*40 is well under a page*2; VE2 has 10 sections.
        let mut prefix =
            vec![0_u8; AXLF_FIRST_SECTION_HEADER_ABS_OFFSET + 16 * AXLF_SECTION_HEADER_SIZE];
        let n = match f.read(&mut prefix) {
            Ok(n) => n,
            Err(e) => {
                eprintln!("OFFLINE_ASSET_SKIP: read failed: {e}");
                return;
            }
        };
        prefix.truncate(n);
        let r = research_xclbin_cu_pdi_section_offset_map(&prefix);
        assert!(r.magic_xclbin2, "VE2 xclbin must be xclbin2");
        assert_eq!(r.num_sections, Some(10));
        assert_eq!(r.section_headers_parsed, 10);
        assert!(r.has_bitstream_section);
        assert!(r.has_aie_partition_section);
        assert!(r.has_cu_pdi_offset_surface);
        // Locked public offsets from this fixture (header walk only).
        assert_eq!(r.bitstream_offset, Some(856));
        assert_eq!(r.bitstream_size, Some(9_717_811));
        assert_eq!(r.aie_partition_offset, Some(9_731_640));
        assert_eq!(r.aie_partition_size, Some(416));
        assert!(!r.section_offset_map_alone_closes_fw_residual);
        assert!(r.do_not_invent_pdi_layout);
        assert!(!r.public_pdi_image_decode);
        let n = r.format_soft_note();
        assert!(residual_note_has_exact_token(&n, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&n, "product_known_good=0"));
        assert!(residual_note_has_exact_token(
            &n,
            "section_offset_map_alone_closes_fw_residual=0"
        ));
        // VE2 platform claim still refuses as AIE2 known-good (orthogonal).
        assert!(platform_claim_is_not_aie2_known_good(
            "PLATFORM=VE2_EDGE NOT_AIE2_KNOWN_GOOD=1"
        ));
    }

    // -----------------------------------------------------------------------
    // Pure axlf section body bounds + journal tokens (stamp 2c4b96bf)
    // -----------------------------------------------------------------------

    #[test]
    fn axlf_section_body_window_pure_bounds_overflow_and_oob() {
        // Empty file: only zero-size at offset 0 is in-bounds.
        assert!(axlf_section_body_window_in_bounds(0, 0, 0));
        assert!(!axlf_section_body_window_in_bounds(0, 0, 1));
        assert!(!axlf_section_body_window_in_bounds(0, 1, 0));

        let ok = check_axlf_section_body_bounds(100, 40, 60);
        assert!(ok.in_bounds);
        assert_eq!(ok.detail, "in_bounds");
        assert_eq!(ok.file_len, 100);
        assert_eq!(ok.offset, 40);
        assert_eq!(ok.size, 60);

        let past = check_axlf_section_body_bounds(100, 101, 0);
        assert!(!past.in_bounds);
        assert_eq!(past.detail, "oob_offset_past_file");

        let end_past = check_axlf_section_body_bounds(100, 50, 51);
        assert!(!end_past.in_bounds);
        assert_eq!(end_past.detail, "oob_end_past_file");

        // size overflow: offset near max + large size.
        let overflow = check_axlf_section_body_bounds(u64::MAX, u64::MAX - 1, 10);
        assert!(!overflow.in_bounds);
        assert_eq!(overflow.detail, "oob_size_overflow");

        // Exact fit end == file_len is in-bounds (half-open).
        assert!(axlf_section_body_window_in_bounds(100, 0, 100));
        assert!(axlf_section_body_window_in_bounds(100, 100, 0));
        assert!(!axlf_section_body_window_in_bounds(100, 100, 1));
    }

    #[test]
    fn xclbin_section_body_bounds_synth_in_bounds_and_oob_not_pass() {
        // Header-only synth: claimed bodies past buffer end → OOB (honest).
        let header_only = synth_axlf_with_sections(&[
            (AXLF_SECTION_KIND_BITSTREAM, "behav", 0x400, 0x1000),
            (
                AXLF_SECTION_KIND_AIE_PARTITION,
                "aie_partition",
                0x2000,
                416,
            ),
        ]);
        let oob = research_xclbin_cu_pdi_section_body_bounds(&header_only);
        assert!(oob.magic_xclbin2);
        assert!(oob.has_cu_pdi_offset_surface);
        assert_eq!(oob.bitstream_in_bounds, Some(false));
        assert_eq!(oob.aie_partition_in_bounds, Some(false));
        assert!(oob.any_oob);
        assert!(!oob.all_present_in_bounds);
        assert_eq!(oob.detail, "cu_pdi_body_windows_oob");
        assert!(!oob.body_bounds_alone_closes_fw_residual);
        assert!(oob.do_not_invent_pdi_layout);
        assert!(!oob.public_pdi_image_decode);
        let on = oob.format_soft_note();
        assert!(residual_note_has_exact_token(
            &on,
            "cu_pdi_section_body_bounds_research"
        ));
        assert!(residual_note_has_exact_token(&on, "any_oob=1"));
        assert!(residual_note_has_exact_token(
            &on,
            "all_present_in_bounds=0"
        ));
        assert!(residual_note_has_exact_token(&on, "bitstream_in_bounds=0"));
        assert!(residual_note_has_exact_token(
            &on,
            "aie_partition_in_bounds=0"
        ));
        assert!(residual_note_has_exact_token(
            &on,
            "body_bounds_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(&on, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&on, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&on, "not_ran_on_npu=1"));
        assert!(residual_note_has_exact_token(&on, "product_known_good=0"));
        assert!(residual_note_has_exact_token(
            &on,
            "do_not_invent_pdi_layout=1"
        ));
        assert!(residual_note_has_exact_token(
            &on,
            "public_pdi_image_decode=0"
        ));
        // Mutated invent/close fields must not leak into soft note.
        let mut mutated = oob.clone();
        mutated.body_bounds_alone_closes_fw_residual = true;
        mutated.do_not_invent_pdi_layout = false;
        mutated.public_pdi_image_decode = true;
        let mn = mutated.format_soft_note();
        assert!(residual_note_has_exact_token(
            &mn,
            "body_bounds_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(
            &mn,
            "do_not_invent_pdi_layout=1"
        ));
        assert!(residual_note_has_exact_token(
            &mn,
            "public_pdi_image_decode=0"
        ));
        assert!(!residual_note_has_exact_token(
            &mn,
            "body_bounds_alone_closes_fw_residual=1"
        ));
        assert!(!residual_note_has_exact_token(&mn, "invent_pass=1"));

        // Pad buffer so claimed windows fit — still research only, not Pass.
        let mut padded = header_only;
        let need = 0x2000_u64 + 416;
        #[allow(clippy::cast_possible_truncation)]
        let need_usize = need as usize;
        if padded.len() < need_usize {
            padded.resize(need_usize, 0);
        }
        let ok = research_xclbin_cu_pdi_section_body_bounds(&padded);
        assert!(ok.magic_xclbin2);
        assert_eq!(ok.bitstream_in_bounds, Some(true));
        assert_eq!(ok.aie_partition_in_bounds, Some(true));
        assert!(!ok.any_oob);
        assert!(ok.all_present_in_bounds);
        assert_eq!(ok.detail, "cu_pdi_body_windows_in_bounds");
        assert!(!ok.body_bounds_alone_closes_fw_residual);
        let ok_n = ok.format_soft_note();
        assert!(residual_note_has_exact_token(&ok_n, "any_oob=0"));
        assert!(residual_note_has_exact_token(
            &ok_n,
            "all_present_in_bounds=1"
        ));
        assert!(residual_note_has_exact_token(
            &ok_n,
            "bitstream_in_bounds=1"
        ));
        assert!(residual_note_has_exact_token(
            &ok_n,
            "aie_partition_in_bounds=1"
        ));
        assert!(residual_note_has_exact_token(&ok_n, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&ok_n, "product_known_good=0"));
        assert!(residual_note_has_exact_token(
            &ok_n,
            "body_bounds_alone_closes_fw_residual=0"
        ));
        // In-bounds ≠ product known-good / residual closed.
        assert!(!residual_note_has_exact_token(
            &ok_n,
            "product_known_good=1"
        ));
    }

    #[test]
    fn xclbin_section_body_bounds_partial_surface_and_empty_not_pass() {
        // BITSTREAM alone, padded in-bounds.
        let mut bs = synth_axlf_with_sections(&[(AXLF_SECTION_KIND_BITSTREAM, "behav", 100, 50)]);
        if bs.len() < 150 {
            bs.resize(150, 0);
        }
        let r = research_xclbin_cu_pdi_section_body_bounds(&bs);
        assert_eq!(r.bitstream_in_bounds, Some(true));
        assert!(r.aie_partition_in_bounds.is_none());
        assert!(r.all_present_in_bounds);
        assert!(!r.any_oob);
        assert_eq!(r.detail, "cu_pdi_body_window_in_bounds_partial_surface");
        let n = r.format_soft_note();
        assert!(residual_note_has_exact_token(
            &n,
            "aie_partition_in_bounds=absent"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "body_bounds_alone_closes_fw_residual=0"
        ));

        // Partial OOB: BITSTREAM fits, AIE_PARTITION does not.
        let mut mixed = synth_axlf_with_sections(&[
            (AXLF_SECTION_KIND_BITSTREAM, "behav", 100, 20),
            (
                AXLF_SECTION_KIND_AIE_PARTITION,
                "aie_partition",
                10_000,
                416,
            ),
        ]);
        if mixed.len() < 120 {
            mixed.resize(120, 0);
        }
        let m = research_xclbin_cu_pdi_section_body_bounds(&mixed);
        assert_eq!(m.bitstream_in_bounds, Some(true));
        assert_eq!(m.aie_partition_in_bounds, Some(false));
        assert!(m.any_oob);
        assert!(!m.all_present_in_bounds);
        assert_eq!(m.detail, "cu_pdi_body_windows_partial_oob");

        // Empty / non-magic / topology-only.
        let empty = research_xclbin_cu_pdi_section_body_bounds(&[]);
        assert_eq!(empty.detail, "default_empty_bytes");
        assert!(!empty.has_cu_pdi_offset_surface);
        assert!(residual_note_has_exact_token(
            &empty.format_soft_note(),
            "body_bounds_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(
            &empty.format_soft_note(),
            "product_known_good=0"
        ));

        let bad = research_xclbin_cu_pdi_section_body_bounds(b"ELF\0notxclbin");
        assert!(!bad.magic_xclbin2);
        assert_eq!(bad.detail, "magic_not_xclbin2");

        let topo = synth_axlf_with_sections(&[(6, "", 0x100, 8)]);
        let t = research_xclbin_cu_pdi_section_body_bounds(&topo);
        assert!(!t.has_cu_pdi_offset_surface);
        assert_eq!(t.detail, "no_cu_pdi_section_headers");
        assert!(t.bitstream_in_bounds.is_none());
        assert!(t.aie_partition_in_bounds.is_none());

        // from_offset_map with explicit file_len (header map + long file).
        let map = research_xclbin_cu_pdi_section_offset_map(&synth_axlf_with_sections(&[(
            AXLF_SECTION_KIND_AIE_PARTITION,
            "aie_partition",
            0x3000,
            416,
        )]));
        let long = research_cu_pdi_section_body_bounds_from_offset_map(&map, 0x3000 + 416);
        assert_eq!(long.aie_partition_in_bounds, Some(true));
        assert!(long.all_present_in_bounds);
        assert_eq!(long.detail, "cu_pdi_body_window_in_bounds_partial_surface");
        assert!(!long.body_bounds_alone_closes_fw_residual);

        let def = CuPdiSectionBodyBoundsResearch::default();
        assert_eq!(def.detail, "default_empty_bytes");
        assert!(!def.body_bounds_alone_closes_fw_residual);
        assert!(def.do_not_invent_pdi_layout);
    }

    #[test]
    fn xclbin_section_body_bounds_ve2_prefix_oob_not_aie2_known_good() {
        // VE2 header prefix is shorter than BITSTREAM body → OOB vs prefix len.
        // Full-file windows would fit; we never load full multi-MiB body here.
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("testdata/xdna2_cmd/ve2_latency/sources/ve2_runner_latency_validate.xclbin");
        if !path.is_file() {
            eprintln!("OFFLINE_ASSET_SKIP: no ve2 validate.xclbin");
            return;
        }
        let mut f = match std::fs::File::open(&path) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("OFFLINE_ASSET_SKIP: open failed: {e}");
                return;
            }
        };
        use std::io::Read;
        let mut prefix =
            vec![0_u8; AXLF_FIRST_SECTION_HEADER_ABS_OFFSET + 16 * AXLF_SECTION_HEADER_SIZE];
        let n = match f.read(&mut prefix) {
            Ok(n) => n,
            Err(e) => {
                eprintln!("OFFLINE_ASSET_SKIP: read failed: {e}");
                return;
            }
        };
        prefix.truncate(n);
        let r = research_xclbin_cu_pdi_section_body_bounds(&prefix);
        assert!(r.magic_xclbin2);
        assert!(r.has_cu_pdi_offset_surface);
        // Prefixed buffer cannot contain multi-MiB BITSTREAM → OOB vs prefix.
        assert_eq!(r.bitstream_in_bounds, Some(false));
        assert!(r.any_oob);
        assert!(!r.all_present_in_bounds);
        assert!(!r.body_bounds_alone_closes_fw_residual);
        let n = r.format_soft_note();
        assert!(residual_note_has_exact_token(&n, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&n, "product_known_good=0"));
        assert!(residual_note_has_exact_token(
            &n,
            "body_bounds_alone_closes_fw_residual=0"
        ));
        assert!(platform_claim_is_not_aie2_known_good(
            "PLATFORM=VE2_EDGE NOT_AIE2_KNOWN_GOOD=1"
        ));
    }

    #[test]
    fn journal_failed_cmd_chain_status_token_parsers_unit_locked() {
        // C2 locked journal lines (primary load-reject + chain INVALID_PARAM).
        let fail_line = "amdxdna: Failed cmd idx 0, status 0x3000003";
        let chain_line = "amdxdna: Status 0x4000003";
        assert_eq!(
            parse_failed_cmd_status_token(fail_line),
            Some(AIE2_FW_APP_LOAD_PDI_FAIL)
        );
        assert_eq!(parse_chain_status_token(fail_line), None, "not chain line");
        assert_eq!(
            parse_chain_status_token(chain_line),
            Some(AIE2_FW_INVALID_PARAM)
        );
        assert_eq!(
            parse_failed_cmd_status_token(chain_line),
            None,
            "chain line is not failed-cmd"
        );
        assert_eq!(
            first_hex_status_token(fail_line),
            Some(AIE2_FW_APP_LOAD_PDI_FAIL)
        );
        assert_eq!(
            first_hex_status_token(chain_line),
            Some(AIE2_FW_INVALID_PARAM)
        );

        // Underscore form + case.
        assert_eq!(
            parse_failed_cmd_status_token("FAILED_CMD status 0x3000003"),
            Some(0x3_000_003)
        );
        assert_eq!(
            parse_chain_status_token("  STATUS 0x4000003  "),
            Some(0x4_000_003)
        );

        // Negatives: no invent from unrelated text / missing hex / empty.
        assert!(parse_failed_cmd_status_token("").is_none());
        assert!(parse_chain_status_token("").is_none());
        assert!(parse_failed_cmd_status_token("status 0x3000003").is_none());
        assert!(parse_chain_status_token("failed cmd without hex").is_none());
        assert!(parse_failed_cmd_status_token("failed cmd without hex").is_none());
        assert!(parse_chain_status_token("no status token here").is_none());
        assert!(first_hex_status_token("no hex here").is_none());

        // Multi-line pair via pure helpers (same as parse_journal_fw_status_pair).
        let locked = c2_locked_app_load_journal_text();
        let pair = parse_journal_fw_status_pair(locked);
        assert_eq!(pair.failed_cmd_status, Some(AIE2_FW_APP_LOAD_PDI_FAIL));
        assert_eq!(pair.chain_status, Some(AIE2_FW_INVALID_PARAM));
        assert!(pair.is_c2_app_load_pair());
        let pn = pair.format_soft_note();
        assert!(residual_note_has_exact_token(&pn, "failed_cmd=0x3000003"));
        assert!(residual_note_has_exact_token(&pn, "chain_status=0x4000003"));
        assert!(residual_note_has_exact_token(&pn, "c2_app_load_pair=1"));
        assert!(residual_note_has_exact_token(&pn, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&pn, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&pn, "not_ran_on_npu=1"));
        // Soft pair note never invents residual-closed / Pass.
        assert!(!residual_note_has_exact_token(&pn, "invent_pass=1"));
        assert!(!residual_note_has_exact_token(&pn, "root_cause_closed=1"));

        // last_apply detail is userspace-only: no failed_cmd / chain tokens.
        let last_apply =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/xdna2_cmd/last_apply.json");
        if last_apply.is_file() {
            let text = std::fs::read_to_string(&last_apply).unwrap_or_default();
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                if let Some(detail) = v.get("detail").and_then(|d| d.as_str()) {
                    let p = parse_journal_fw_status_pair(detail);
                    assert!(
                        p.failed_cmd_status.is_none(),
                        "last_apply detail must not invent failed_cmd"
                    );
                    assert!(
                        p.chain_status.is_none(),
                        "last_apply detail must not invent chain_status"
                    );
                    assert!(!p.is_c2_app_load_pair());
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Soft e2e fixture path + bag-drop checklist (stamp 8e898123)
    // -----------------------------------------------------------------------

    #[test]
    fn external_known_good_bag_drop_check_fail_closed_incomplete() {
        // Absent bytes + no provenance → incomplete.
        let absent = check_external_known_good_bag_drop(false, None, None, None);
        assert!(!absent.bag_drop_complete);
        assert!(!absent.ctrltext_bytes_present);
        assert!(!absent.provenance_research_ready);
        assert!(!absent.bag_drop_alone_closes_fw_residual);
        assert_eq!(absent.detail, "bag_drop_absent_bytes_and_provenance");
        let n = absent.format_soft_note();
        assert!(residual_note_has_exact_token(
            &n,
            "external_known_good_bag_drop_check"
        ));
        assert!(residual_note_has_exact_token(&n, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&n, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&n, "not_ran_on_npu=1"));
        assert!(residual_note_has_exact_token(&n, "product_known_good=0"));
        assert!(residual_note_has_exact_token(
            &n,
            "bag_drop_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(&n, "bag_drop_complete=0"));

        // Bytes without research-ready PROVENANCE → incomplete.
        let no_prov = check_external_known_good_bag_drop(
            true,
            Some("empty notes without markers"),
            Some("working_stack_ctrltext.bin"),
            None,
        );
        assert!(!no_prov.bag_drop_complete);
        assert!(no_prov.ctrltext_bytes_present);
        assert!(!no_prov.provenance_research_ready);
        assert_eq!(no_prov.detail, "bag_drop_provenance_not_research_ready");

        // Refused local post-patch basename even with ready PROVENANCE.
        let ready = "Research only. Not product Pass. Not C-ready.\nSource stack: XRT public stack";
        let post = check_external_known_good_bag_drop(
            true,
            Some(ready),
            Some("instr_post_patch.bin"),
            None,
        );
        assert!(!post.bag_drop_complete);
        assert!(post.refused_local_post_patch_basename);
        assert_eq!(post.detail, "bag_drop_refused_local_post_patch");

        // VE2 / NOT_AIE2 refuse.
        let ve2 = check_external_known_good_bag_drop(
            true,
            Some(ready),
            Some("ve2_ctrltext.bin"),
            Some("PLATFORM=VE2_EDGE NOT_AIE2_KNOWN_GOOD=1"),
        );
        assert!(!ve2.bag_drop_complete);
        assert!(ve2.refused_ve2_not_aie2);
        assert_eq!(ve2.detail, "bag_drop_refused_ve2_not_aie2");

        // Complete research-only drop still invent_pass=0 / product_known_good=0.
        let ok = check_external_known_good_bag_drop(
            true,
            Some(ready),
            Some("working_stack_ctrltext.bin"),
            Some("Source stack: working public XRT AIE2"),
        );
        assert!(ok.bag_drop_complete);
        assert!(!ok.bag_drop_alone_closes_fw_residual);
        assert_eq!(ok.detail, "bag_drop_complete_research_only");
        let on = ok.format_soft_note();
        assert!(residual_note_has_exact_token(&on, "bag_drop_complete=1"));
        assert!(residual_note_has_exact_token(&on, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&on, "product_known_good=0"));
        assert!(!residual_note_has_exact_token(&on, "invent_pass=1"));
    }

    #[test]
    fn external_known_good_bag_drop_check_honesty_hard_coded_not_mutable() {
        // Mutating bag_drop_alone_closes / invent-ish fields must not flip soft note.
        let ready = "Research only. Not product Pass. Not C-ready.\nSource stack: XRT public stack";
        let mut incomplete = check_external_known_good_bag_drop(false, None, None, None);
        incomplete.bag_drop_alone_closes_fw_residual = true;
        incomplete.bag_drop_complete = true;
        incomplete.ctrltext_bytes_present = true;
        let n = incomplete.format_soft_note();
        assert!(residual_note_has_exact_token(
            &n,
            "bag_drop_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(&n, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&n, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&n, "not_ran_on_npu=1"));
        assert!(residual_note_has_exact_token(&n, "product_known_good=0"));
        // bag_drop_complete field is echoed from struct (not honesty lock).
        assert!(residual_note_has_exact_token(&n, "bag_drop_complete=1"));
        assert!(!residual_note_has_exact_token(
            &n,
            "bag_drop_alone_closes_fw_residual=1"
        ));
        assert!(!residual_note_has_exact_token(&n, "invent_pass=1"));

        let mut complete = check_external_known_good_bag_drop(
            true,
            Some(ready),
            Some("working_stack_ctrltext.bin"),
            Some("Source stack: working public XRT AIE2"),
        );
        assert!(complete.bag_drop_complete);
        complete.bag_drop_alone_closes_fw_residual = true;
        let cn = complete.format_soft_note();
        assert!(residual_note_has_exact_token(
            &cn,
            "bag_drop_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(&cn, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&cn, "product_known_good=0"));
        assert!(residual_note_has_exact_token(&cn, "bag_drop_complete=1"));
    }

    #[test]
    fn app_load_e2e_fixture_research_c2_locked_absent_bag() {
        let r = research_app_load_e2e_fixture_c2_locked_absent_bag();
        assert!(r.locked_app_load_after_aiebu);
        assert!(r.userspace_elf_rela_ok);
        assert_eq!(r.residual_class, "app_load_pdi_fail_after_aiebu");
        assert_eq!(r.journal_failed_cmd, Some(AIE2_FW_APP_LOAD_PDI_FAIL));
        assert_eq!(r.journal_chain_status, Some(AIE2_FW_INVALID_PARAM));
        assert!(r.journal_c2_app_load_pair);
        assert!(r.last_apply_detail_journal_absent);
        assert!(!r.bag_drop_complete);
        assert!(r.residual_not_closed);
        assert!(!r.body_bounds_alone_closes_fw_residual);
        assert_eq!(r.detail, "e2e_app_load_c2_pair_bag_absent_residual_open");

        let n = r.format_soft_note();
        assert!(residual_note_has_exact_token(
            &n,
            "app_load_e2e_fixture_research"
        ));
        assert!(residual_note_has_exact_token(&n, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&n, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&n, "not_ran_on_npu=1"));
        assert!(residual_note_has_exact_token(&n, "product_known_good=0"));
        assert!(residual_note_has_exact_token(&n, "residual_not_closed=1"));
        assert!(residual_note_has_exact_token(
            &n,
            "body_bounds_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "ctrltext_match_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(&n, "bag_drop_complete=0"));
        assert!(residual_note_has_exact_token(
            &n,
            "journal_failed_cmd=0x3000003"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "journal_chain_status=0x4000003"
        ));
        assert!(!residual_note_has_exact_token(&n, "invent_pass=1"));
        assert!(!residual_note_has_exact_token(&n, "residual_not_closed=0"));
    }

    #[test]
    fn app_load_e2e_fixture_research_last_apply_fixture_no_invent_journal() {
        let last_apply =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/xdna2_cmd/last_apply.json");
        if !last_apply.is_file() {
            eprintln!("OFFLINE_ASSET_SKIP: no last_apply.json");
            return;
        }
        let text = std::fs::read_to_string(&last_apply).unwrap_or_default();
        let v: serde_json::Value = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("OFFLINE_ASSET_SKIP: last_apply parse: {e}");
                return;
            }
        };
        let status = v.get("status").and_then(|s| s.as_str());
        let detail = v.get("detail").and_then(|d| d.as_str());
        let aiebu = v.get("aiebu_elf").and_then(|a| a.as_bool());
        let ert_err = v.get("ert_error").and_then(|e| e.as_bool());
        let rela = v
            .get("rela_count")
            .and_then(|r| r.as_u64())
            .map(|n| n as usize);
        let class = classify_last_apply_residual_ex(status, detail, aiebu, ert_err, rela);
        assert_eq!(class, SiliconResidualClass::AppLoadPdiFailAfterAiebu);

        // Lab bag PROVENANCE may be research-ready template without bytes.
        let bag_root = external_known_good_lab_root();
        let prov = read_external_known_good_provenance_text(&bag_root.join("any.bin"));
        let bag = check_external_known_good_bag_drop(
            false, // no .bin in bag offline
            prov.as_deref(),
            None,
            None,
        );
        assert!(!bag.bag_drop_complete, "bytes still absent offline");

        let r = research_app_load_e2e_fixture_path(
            class, detail, None, // no journal paste → no invent codes
            None, &bag,
        );
        assert!(r.locked_app_load_after_aiebu);
        assert!(r.userspace_elf_rela_ok);
        assert!(r.journal_failed_cmd.is_none());
        assert!(r.journal_chain_status.is_none());
        assert!(!r.journal_c2_app_load_pair);
        assert!(r.last_apply_detail_journal_absent);
        assert!(r.residual_not_closed);
        assert!(!r.bag_drop_complete);
        assert_eq!(
            r.detail,
            "e2e_app_load_userspace_ok_bag_absent_residual_open"
        );

        // Optional body-bounds surface from VE2 header prefix (OOB honest).
        let ve2 = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("testdata/xdna2_cmd/ve2_latency/sources/ve2_runner_latency_validate.xclbin");
        if ve2.is_file() {
            let mut f = match std::fs::File::open(&ve2) {
                Ok(f) => f,
                Err(_) => return,
            };
            use std::io::Read;
            let mut prefix =
                vec![0_u8; AXLF_FIRST_SECTION_HEADER_ABS_OFFSET + 16 * AXLF_SECTION_HEADER_SIZE];
            let n = f.read(&mut prefix).unwrap_or(0);
            prefix.truncate(n);
            let with_bounds = research_app_load_e2e_fixture_path(
                class,
                detail,
                Some(c2_locked_app_load_journal_text()),
                Some(&prefix),
                &bag,
            );
            assert!(with_bounds.body_bounds_has_surface);
            assert!(with_bounds.body_bounds_any_oob); // prefix OOB vs multi-MiB body
            assert!(with_bounds.journal_c2_app_load_pair);
            assert!(with_bounds.residual_not_closed);
            assert!(!with_bounds.body_bounds_alone_closes_fw_residual);
            let bn = with_bounds.format_soft_note();
            assert!(residual_note_has_exact_token(&bn, "body_bounds_any_oob=1"));
            assert!(residual_note_has_exact_token(&bn, "residual_not_closed=1"));
            assert!(residual_note_has_exact_token(&bn, "invent_pass=0"));
        }
    }

    #[test]
    fn app_load_e2e_fixture_research_honesty_hard_coded_not_mutable() {
        // Mutating residual_not_closed / invent-ish fields must not flip soft note.
        let mut r = research_app_load_e2e_fixture_c2_locked_absent_bag();
        r.residual_not_closed = false;
        r.bag_drop_complete = true;
        r.body_bounds_alone_closes_fw_residual = true;
        let n = r.format_soft_note();
        assert!(residual_note_has_exact_token(&n, "residual_not_closed=1"));
        assert!(residual_note_has_exact_token(
            &n,
            "body_bounds_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "ctrltext_match_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(&n, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&n, "not_c_ready=1"));
        // bag_drop_complete field is echoed from struct (not honesty lock);
        // residual locks stay hard-coded.
        assert!(residual_note_has_exact_token(&n, "bag_drop_complete=1"));
    }

    // -----------------------------------------------------------------------
    // Offline progress scorecard + operator extract checklist (stamp 9cdfe8d0)
    // -----------------------------------------------------------------------

    #[test]
    fn app_load_offline_progress_scorecard_c2_locked_absent_bag() {
        let s = research_app_load_offline_progress_scorecard_c2_locked_absent_bag();
        assert!(s.locked_app_load_after_aiebu);
        assert!(s.userspace_elf_rela_ok);
        assert_eq!(s.residual_class, "app_load_pdi_fail_after_aiebu");
        assert!(!s.bag_drop_complete);
        assert!(s.e2e_residual_not_closed);
        assert!(s.residual_not_closed);
        assert!(!s.scorecard_alone_closes_fw_residual);
        assert!(!s.body_bounds_alone_closes_fw_residual);
        assert!(s.journal_tokens_present);
        assert!(s.journal_c2_app_load_pair);
        assert_eq!(s.external_compare_status, ExternalCompareStatus::Absent);
        assert!(!s.scorecard_research_surfaces_ready);
        assert_eq!(s.detail, "scorecard_c2_locked_bag_absent_residual_open");

        let n = s.format_soft_note();
        assert!(residual_note_has_exact_token(
            &n,
            "app_load_offline_progress_scorecard"
        ));
        assert!(residual_note_has_exact_token(&n, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&n, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&n, "not_ran_on_npu=1"));
        assert!(residual_note_has_exact_token(&n, "product_known_good=0"));
        assert!(residual_note_has_exact_token(&n, "residual_not_closed=1"));
        assert!(residual_note_has_exact_token(
            &n,
            "scorecard_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "ctrltext_match_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "body_bounds_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(&n, "bag_drop_complete=0"));
        assert!(residual_note_has_exact_token(
            &n,
            "external_compare_status=absent"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "journal_tokens_present=1"
        ));
        assert!(!residual_note_has_exact_token(&n, "invent_pass=1"));
        assert!(!residual_note_has_exact_token(&n, "residual_not_closed=0"));
    }

    #[test]
    fn app_load_offline_progress_scorecard_no_invent_journal_from_detail_alone() {
        let bag = check_external_known_good_bag_drop(false, None, None, None);
        // last_apply-style userspace detail only — no journal paste on e2e.
        let e2e = research_app_load_e2e_fixture_path(
            SiliconResidualClass::AppLoadPdiFailAfterAiebu,
            Some(
                "ert_state=5(Error) aiebu_elf=1 rela_count=12 aiebu_rela_patched \
                 ctrltext=1688B ran_on_npu=false",
            ),
            None, // no journal → no invent codes
            None,
            &bag,
        );
        assert!(e2e.journal_failed_cmd.is_none());
        assert!(e2e.last_apply_detail_journal_absent);
        let s = research_app_load_offline_progress_scorecard(
            SiliconResidualClass::AppLoadPdiFailAfterAiebu,
            &bag,
            &e2e,
            None,
            false,
        );
        assert!(!s.journal_tokens_present);
        assert!(!s.journal_c2_app_load_pair);
        assert!(s.residual_not_closed);
        assert_eq!(s.external_compare_status, ExternalCompareStatus::Absent);
        assert_eq!(
            s.detail,
            "scorecard_app_load_userspace_ok_bag_incomplete_residual_open"
        );
        let n = s.format_soft_note();
        assert!(residual_note_has_exact_token(
            &n,
            "journal_tokens_present=0"
        ));
        assert!(residual_note_has_exact_token(&n, "residual_not_closed=1"));
        assert!(residual_note_has_exact_token(&n, "invent_pass=0"));
    }

    #[test]
    fn app_load_offline_progress_scorecard_compare_status_and_refuse() {
        let ready = "Research only. Not product Pass. Not C-ready.\nSource stack: XRT public stack";
        // Loaded: bytes present, bag incomplete (no provenance) → status loaded.
        let bag_loaded = check_external_known_good_bag_drop(
            true,
            Some("not research ready"),
            Some("working_stack_ctrltext.bin"),
            None,
        );
        assert!(!bag_loaded.bag_drop_complete);
        let e2e = research_app_load_e2e_fixture_c2_locked_absent_bag();
        let st_loaded = external_compare_status_from_surfaces(&bag_loaded, None, false);
        assert_eq!(st_loaded, ExternalCompareStatus::Loaded);
        // Scorecard for Loaded (bytes present, no compare, bag incomplete).
        let s_loaded = research_app_load_offline_progress_scorecard(
            SiliconResidualClass::AppLoadPdiFailAfterAiebu,
            &bag_loaded,
            &e2e,
            None,
            false,
        );
        assert_eq!(
            s_loaded.external_compare_status,
            ExternalCompareStatus::Loaded
        );
        assert!(!s_loaded.bag_drop_complete);
        assert!(s_loaded.residual_not_closed);
        assert!(!s_loaded.scorecard_alone_closes_fw_residual);
        assert_eq!(
            s_loaded.detail,
            "scorecard_app_load_userspace_ok_bag_incomplete_residual_open"
        );
        let ln = s_loaded.format_soft_note();
        assert!(residual_note_has_exact_token(
            &ln,
            "external_compare_status=loaded"
        ));
        assert!(residual_note_has_exact_token(&ln, "residual_not_closed=1"));
        assert!(residual_note_has_exact_token(&ln, "invent_pass=0"));
        assert!(residual_note_has_exact_token(
            &ln,
            "scorecard_alone_closes_fw_residual=0"
        ));

        // Loaded + bag_complete (provenance ready, no compare yet) → pending detail.
        let bag_complete_no_cmp = check_external_known_good_bag_drop(
            true,
            Some(ready),
            Some("working_stack_ctrltext.bin"),
            Some("Source stack: working public XRT AIE2"),
        );
        assert!(bag_complete_no_cmp.bag_drop_complete);
        let s_pending = research_app_load_offline_progress_scorecard(
            SiliconResidualClass::AppLoadPdiFailAfterAiebu,
            &bag_complete_no_cmp,
            &e2e,
            None,
            false,
        );
        assert_eq!(
            s_pending.external_compare_status,
            ExternalCompareStatus::Loaded
        );
        assert_eq!(
            s_pending.detail,
            "scorecard_bag_complete_loaded_compare_pending_residual_open"
        );
        assert!(s_pending.residual_not_closed);
        assert!(!s_pending.scorecard_alone_closes_fw_residual);
        let pn = s_pending.format_soft_note();
        assert!(residual_note_has_exact_token(
            &pn,
            "external_compare_status=loaded"
        ));
        assert!(residual_note_has_exact_token(&pn, "residual_not_closed=1"));
        assert!(residual_note_has_exact_token(&pn, "invent_pass=0"));

        // Refused: VE2 platform claim.
        let bag_ve2 = check_external_known_good_bag_drop(
            true,
            Some(ready),
            Some("ve2_ctrltext.bin"),
            Some("PLATFORM=VE2_EDGE NOT_AIE2_KNOWN_GOOD=1"),
        );
        assert!(bag_ve2.refused_ve2_not_aie2);
        let st_ref = external_compare_status_from_surfaces(&bag_ve2, None, false);
        assert_eq!(st_ref, ExternalCompareStatus::Refused);
        let s_ref = research_app_load_offline_progress_scorecard(
            SiliconResidualClass::AppLoadPdiFailAfterAiebu,
            &bag_ve2,
            &e2e,
            None,
            false,
        );
        assert_eq!(
            s_ref.external_compare_status,
            ExternalCompareStatus::Refused
        );
        assert_eq!(s_ref.detail, "scorecard_external_refused_residual_open");
        assert!(s_ref.residual_not_closed);

        // external_load_refused even without bag refuse flags.
        let bag_ok = check_external_known_good_bag_drop(false, None, None, None);
        assert_eq!(
            external_compare_status_from_surfaces(&bag_ok, None, true),
            ExternalCompareStatus::Refused
        );

        // Match: compare reports external_present + sha_match (research only).
        let mut cmp = ExternalCtrltextCompareResearch {
            external_present: true,
            known_good_absent: false,
            sha_match: true,
            detail: "unit_match".into(),
            ..Default::default()
        };
        let bag_complete = check_external_known_good_bag_drop(
            true,
            Some(ready),
            Some("working_stack_ctrltext.bin"),
            Some("Source stack: working public XRT AIE2"),
        );
        assert!(bag_complete.bag_drop_complete);
        assert_eq!(
            external_compare_status_from_surfaces(&bag_complete, Some(&cmp), false),
            ExternalCompareStatus::Match
        );
        let s_match = research_app_load_offline_progress_scorecard(
            SiliconResidualClass::AppLoadPdiFailAfterAiebu,
            &bag_complete,
            &e2e,
            Some(&cmp),
            false,
        );
        assert!(s_match.scorecard_research_surfaces_ready);
        assert!(s_match.residual_not_closed);
        assert!(!s_match.scorecard_alone_closes_fw_residual);
        assert_eq!(
            s_match.detail,
            "scorecard_surfaces_ready_residual_still_open"
        );
        let mn = s_match.format_soft_note();
        assert!(residual_note_has_exact_token(
            &mn,
            "external_compare_status=match"
        ));
        assert!(residual_note_has_exact_token(&mn, "residual_not_closed=1"));
        assert!(residual_note_has_exact_token(&mn, "invent_pass=0"));
        assert!(residual_note_has_exact_token(
            &mn,
            "scorecard_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(
            &mn,
            "ctrltext_match_alone_closes_fw_residual=0"
        ));

        // Mismatch: bag complete + compare ran with sha_match=false (mirror Match honesty).
        cmp.sha_match = false;
        assert_eq!(
            external_compare_status_from_surfaces(&bag_complete, Some(&cmp), false),
            ExternalCompareStatus::Mismatch
        );
        let s_mis = research_app_load_offline_progress_scorecard(
            SiliconResidualClass::AppLoadPdiFailAfterAiebu,
            &bag_complete,
            &e2e,
            Some(&cmp),
            false,
        );
        assert_eq!(
            s_mis.external_compare_status,
            ExternalCompareStatus::Mismatch
        );
        assert!(s_mis.bag_drop_complete);
        assert!(!s_mis.scorecard_research_surfaces_ready);
        assert!(s_mis.residual_not_closed);
        assert!(!s_mis.scorecard_alone_closes_fw_residual);
        assert_eq!(
            s_mis.detail,
            "scorecard_bag_complete_mismatch_residual_open"
        );
        let misn = s_mis.format_soft_note();
        assert!(residual_note_has_exact_token(
            &misn,
            "external_compare_status=mismatch"
        ));
        assert!(residual_note_has_exact_token(
            &misn,
            "residual_not_closed=1"
        ));
        assert!(residual_note_has_exact_token(&misn, "invent_pass=0"));
        assert!(residual_note_has_exact_token(
            &misn,
            "scorecard_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(
            &misn,
            "ctrltext_match_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(&misn, "bag_drop_complete=1"));
        assert!(!residual_note_has_exact_token(&misn, "invent_pass=1"));
        assert!(!residual_note_has_exact_token(
            &misn,
            "residual_not_closed=0"
        ));
    }

    #[test]
    fn app_load_offline_progress_scorecard_honesty_hard_coded_not_mutable() {
        let mut s = research_app_load_offline_progress_scorecard_c2_locked_absent_bag();
        s.residual_not_closed = false;
        s.scorecard_alone_closes_fw_residual = true;
        s.body_bounds_alone_closes_fw_residual = true;
        s.scorecard_research_surfaces_ready = true;
        s.bag_drop_complete = true;
        s.external_compare_status = ExternalCompareStatus::Match;
        let n = s.format_soft_note();
        assert!(residual_note_has_exact_token(&n, "residual_not_closed=1"));
        assert!(residual_note_has_exact_token(
            &n,
            "scorecard_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "ctrltext_match_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "body_bounds_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(&n, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&n, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&n, "not_ran_on_npu=1"));
        assert!(residual_note_has_exact_token(&n, "product_known_good=0"));
        // Mutable surface fields may echo; honesty locks stay hard-coded.
        assert!(residual_note_has_exact_token(&n, "bag_drop_complete=1"));
        assert!(residual_note_has_exact_token(
            &n,
            "external_compare_status=match"
        ));
        assert!(!residual_note_has_exact_token(
            &n,
            "scorecard_alone_closes_fw_residual=1"
        ));
        assert!(!residual_note_has_exact_token(&n, "invent_pass=1"));
        assert!(!residual_note_has_exact_token(&n, "residual_not_closed=0"));
    }

    #[test]
    fn external_known_good_operator_extract_checklist_refuse_tokens() {
        let t = format_external_known_good_operator_extract_checklist();
        assert!(t.contains("external_known_good_operator_extract_checklist"));
        assert!(t.contains("stamp=588c1c72"));
        assert!(t.contains("lab_export_auto_feed=1"));
        assert!(t.contains("feed_alone_closes_fw_residual=0"));
        assert!(t.contains("step9=lab_export_fingerprint_auto_feed"));
        assert!(t.contains("step10=soft_app_load_bag_feed_dry_run_scorecard"));
        assert!(t.contains("step11=soft_app_load_extract_bag_scorecard_pipeline"));
        assert!(t.contains("extract_alone_closes_fw_residual=0"));
        assert!(t.contains("pipeline_alone_closes_fw_residual=0"));
        // Key refuse tokens (unit-locked).
        assert!(t.contains("refuse_local_post_patch=1"));
        assert!(t.contains("refuse_instr_post_patch_bin=1"));
        assert!(t.contains("refuse_ve2_not_aie2=1"));
        assert!(t.contains("refuse_validate_xclbin_as_aie2_known_good=1"));
        // Honesty footer.
        assert!(t.contains("invent_pass=0"));
        assert!(t.contains("not_c_ready=1"));
        assert!(t.contains("not_ran_on_npu=1"));
        assert!(t.contains("product_known_good=0"));
        assert!(t.contains("residual_not_closed=1"));
        assert!(t.contains("scorecard_alone_closes_fw_residual=0"));
        assert!(t.contains("ctrltext_match_alone_closes_fw_residual=0"));
        assert!(t.contains("dry_run_alone_closes_fw_residual=0"));
        assert!(t.contains("blocked_on_external_bytes_when_bag_absent=1"));
        assert!(t.contains("no_second_c_unlock=1"));
        assert!(t.contains("c_unlock_b17b0119_consumed=1"));
        assert!(t.contains("bag_drop_complete_neq_pass=1"));
        // Steps present.
        assert!(t.contains("step3=copy_bytes_to_testdata_xdna2_cmd_external_known_good"));
        assert!(t.contains("step6=set_XDNA2_LAB_EXTERNAL_CTRLTEXT"));
        assert!(t.contains("step8=soft_external_ctrltext_compare_dry_run"));
    }

    // -----------------------------------------------------------------------
    // External ctrltext compare dry-run (stamp 6033158a)
    // -----------------------------------------------------------------------

    #[test]
    fn external_ctrltext_compare_dry_run_absent_blocked_on_external_bytes() {
        let d = research_external_ctrltext_compare_dry_run_c2_locked_absent_bag();
        assert!(!d.lab_export_ready);
        assert!(!d.external_present);
        assert!(d.blocked_on_external_bytes);
        assert!(!d.refused_local_post_patch);
        assert!(!d.refused_ve2_not_aie2);
        assert!(!d.compare_ran);
        assert!(!d.sha_match);
        assert!(d.residual_not_closed);
        assert!(!d.dry_run_alone_closes_fw_residual);
        assert_eq!(d.detail, "dry_run_blocked_on_external_bytes");

        let n = d.format_soft_note();
        assert!(residual_note_has_exact_token(
            &n,
            EXTERNAL_CTRLTEXT_COMPARE_DRY_RUN_NOTE
        ));
        let blocked_1 = format!("{BLOCKED_ON_EXTERNAL_BYTES_TOKEN}=1");
        let blocked_0 = format!("{BLOCKED_ON_EXTERNAL_BYTES_TOKEN}=0");
        assert!(residual_note_has_exact_token(&n, &blocked_1));
        assert!(residual_note_has_exact_token(&n, "lab_export_ready=0"));
        assert!(residual_note_has_exact_token(&n, "external_present=0"));
        assert!(residual_note_has_exact_token(&n, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&n, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&n, "not_ran_on_npu=1"));
        assert!(residual_note_has_exact_token(&n, "product_known_good=0"));
        assert!(residual_note_has_exact_token(&n, "residual_not_closed=1"));
        assert!(residual_note_has_exact_token(
            &n,
            "dry_run_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "ctrltext_match_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "scorecard_alone_closes_fw_residual=0"
        ));
        assert!(!residual_note_has_exact_token(&n, "invent_pass=1"));
        assert!(!residual_note_has_exact_token(&n, "residual_not_closed=0"));
        assert!(!residual_note_has_exact_token(&n, &blocked_0));
    }

    #[test]
    fn external_ctrltext_compare_dry_run_lab_ready_blocked_on_external() {
        let ctrltext = build_strix_research_noop_txn().expect("noop");
        let synth = build_minimal_test_elf32(&ctrltext, &[(0_u32, (1_u32 << 8) | 5, 0_i32)]);
        let exp = export_ctrltext_fingerprint_research(&synth);
        assert!(lab_fingerprint_export_is_ready(Some(&exp)));
        let d = research_external_ctrltext_compare_dry_run(Some(&exp), None, None, None);
        assert!(d.lab_export_ready);
        assert!(d.blocked_on_external_bytes);
        assert!(!d.external_present);
        assert!(!d.compare_ran);
        assert_eq!(
            d.detail,
            "dry_run_lab_export_ready_blocked_on_external_bytes"
        );
        let n = d.format_soft_note();
        assert!(residual_note_has_exact_token(&n, "lab_export_ready=1"));
        assert!(residual_note_has_exact_token(
            &n,
            "blocked_on_external_bytes=1"
        ));
        assert!(residual_note_has_exact_token(&n, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&n, "residual_not_closed=1"));
        assert!(residual_note_has_exact_token(
            &n,
            "dry_run_alone_closes_fw_residual=0"
        ));
        // Empty external slice same as None.
        let d_empty = research_external_ctrltext_compare_dry_run(Some(&exp), Some(&[]), None, None);
        assert!(d_empty.blocked_on_external_bytes);
        assert!(!d_empty.external_present);
    }

    #[test]
    fn external_ctrltext_compare_dry_run_refuse_local_post_patch() {
        let d = research_external_ctrltext_compare_dry_run(
            None,
            Some(b"not-real-known-good"),
            Some("instr_post_patch.bin"),
            None,
        );
        assert!(d.refused_local_post_patch);
        assert!(!d.blocked_on_external_bytes);
        assert!(!d.external_present);
        assert!(!d.compare_ran);
        assert!(!d.sha_match);
        assert_eq!(d.detail, "dry_run_refused_local_post_patch");
        let n = d.format_soft_note();
        assert!(residual_note_has_exact_token(
            &n,
            "refused_local_post_patch=1"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "blocked_on_external_bytes=0"
        ));
        assert!(residual_note_has_exact_token(&n, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&n, "product_known_good=0"));
        assert!(residual_note_has_exact_token(&n, "residual_not_closed=1"));
        // Vendor post_patch basename also refused.
        let d2 = research_external_ctrltext_compare_dry_run(
            None,
            None,
            Some("instr_vendor_post_patch.bin"),
            None,
        );
        assert!(d2.refused_local_post_patch);
        assert_eq!(d2.detail, "dry_run_refused_local_post_patch");
    }

    #[test]
    fn external_ctrltext_compare_dry_run_refuse_ve2_platform_not_aie2() {
        let d = research_external_ctrltext_compare_dry_run(
            None,
            Some(b"ve2-bytes"),
            Some("ve2_ctrltext.bin"),
            Some("PLATFORM=VE2_EDGE NOT_AIE2_KNOWN_GOOD=1"),
        );
        assert!(d.refused_ve2_not_aie2);
        assert!(d.platform_not_aie2);
        assert!(!d.blocked_on_external_bytes);
        assert!(!d.external_present);
        assert!(!d.compare_ran);
        assert_eq!(d.detail, "dry_run_refused_ve2_not_aie2");
        let n = d.format_soft_note();
        assert!(residual_note_has_exact_token(&n, "refused_ve2_not_aie2=1"));
        assert!(residual_note_has_exact_token(&n, "platform_not_aie2=1"));
        assert!(residual_note_has_exact_token(
            &n,
            "xclbin_extract_refused_ve2=1"
        ));
        assert!(residual_note_has_exact_token(&n, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&n, "product_known_good=0"));
        assert!(residual_note_has_exact_token(&n, "residual_not_closed=1"));

        // Companion xclbin header + VE2 claim (explicit path).
        let d_x = research_external_ctrltext_compare_dry_run_ex(
            None,
            None,
            None,
            Some("ve2_latency validate.xclbin NOT_AIE2_KNOWN_GOOD=1"),
            Some(XCLBIN2_MAGIC),
            Some("AIE_PARTITION BITSTREAM"),
            false,
        );
        assert!(d_x.refused_ve2_not_aie2);
        assert!(d_x.platform_not_aie2);
        assert!(d_x.xclbin_extract_refused_ve2);
        assert!(!d_x.blocked_on_external_bytes);
        let xn = d_x.format_soft_note();
        assert!(residual_note_has_exact_token(&xn, "platform_not_aie2=1"));
        assert!(residual_note_has_exact_token(&xn, "invent_pass=0"));
    }

    #[test]
    fn external_ctrltext_compare_dry_run_match_neq_pass() {
        let ctrltext = build_strix_research_noop_txn().expect("noop");
        let synth = build_minimal_test_elf32(&ctrltext, &[(0_u32, (1_u32 << 8) | 5, 0_i32)]);
        let exp = export_ctrltext_fingerprint_research(&synth);
        let d = research_external_ctrltext_compare_dry_run(Some(&exp), Some(&ctrltext), None, None);
        assert!(d.lab_export_ready);
        assert!(d.external_present);
        assert!(!d.blocked_on_external_bytes);
        assert!(d.compare_ran);
        assert!(d.sha_match);
        assert!(d.structural_match);
        assert!(d.residual_not_closed);
        assert!(!d.dry_run_alone_closes_fw_residual);
        assert_eq!(
            d.detail,
            "dry_run_sha_and_structural_match_residual_still_open"
        );
        let n = d.format_soft_note();
        assert!(residual_note_has_exact_token(&n, "compare_ran=1"));
        assert!(residual_note_has_exact_token(&n, "sha_match=1"));
        assert!(residual_note_has_exact_token(&n, "structural_match=1"));
        assert!(residual_note_has_exact_token(
            &n,
            "blocked_on_external_bytes=0"
        ));
        assert!(residual_note_has_exact_token(&n, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&n, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&n, "not_ran_on_npu=1"));
        assert!(residual_note_has_exact_token(&n, "product_known_good=0"));
        assert!(residual_note_has_exact_token(&n, "residual_not_closed=1"));
        assert!(residual_note_has_exact_token(
            &n,
            "dry_run_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "ctrltext_match_alone_closes_fw_residual=0"
        ));
        assert!(!residual_note_has_exact_token(&n, "invent_pass=1"));
        assert!(!residual_note_has_exact_token(&n, "residual_not_closed=0"));

        // Mismatch path still residual open.
        let mut flipped = ctrltext.clone();
        if let Some(b) = flipped.first_mut() {
            *b ^= 0xff;
        }
        let d_mis =
            research_external_ctrltext_compare_dry_run(Some(&exp), Some(&flipped), None, None);
        assert!(d_mis.compare_ran);
        assert!(!d_mis.sha_match);
        assert!(!d_mis.blocked_on_external_bytes);
        assert!(d_mis.residual_not_closed);
        // Flipped first byte → sha_match=0; structural may still match (len/walk).
        assert!(
            d_mis.detail == "dry_run_compare_mismatch_residual_open"
                || d_mis.detail == "dry_run_structural_match_sha_diff_residual_open",
            "unexpected mismatch detail={}",
            d_mis.detail
        );
        let mn = d_mis.format_soft_note();
        assert!(residual_note_has_exact_token(&mn, "sha_match=0"));
        assert!(residual_note_has_exact_token(&mn, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&mn, "residual_not_closed=1"));
        assert!(residual_note_has_exact_token(
            &mn,
            "dry_run_alone_closes_fw_residual=0"
        ));
    }

    #[test]
    fn external_ctrltext_compare_dry_run_honesty_hard_coded_not_mutable() {
        let mut d = research_external_ctrltext_compare_dry_run_c2_locked_absent_bag();
        d.residual_not_closed = false;
        d.dry_run_alone_closes_fw_residual = true;
        d.blocked_on_external_bytes = false;
        d.sha_match = true;
        d.compare_ran = true;
        d.external_present = true;
        d.lab_export_ready = true;
        let n = d.format_soft_note();
        // Honesty locks stay hard-coded even if fields mutated.
        assert!(residual_note_has_exact_token(&n, "residual_not_closed=1"));
        assert!(residual_note_has_exact_token(
            &n,
            "dry_run_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "ctrltext_match_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "scorecard_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(&n, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&n, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&n, "not_ran_on_npu=1"));
        assert!(residual_note_has_exact_token(&n, "product_known_good=0"));
        // Mutable surface fields may echo; honesty locks stay hard-coded.
        assert!(residual_note_has_exact_token(&n, "sha_match=1"));
        assert!(residual_note_has_exact_token(&n, "compare_ran=1"));
        let blocked_0 = format!("{BLOCKED_ON_EXTERNAL_BYTES_TOKEN}=0");
        assert!(residual_note_has_exact_token(&n, &blocked_0));
        assert!(!residual_note_has_exact_token(
            &n,
            "dry_run_alone_closes_fw_residual=1"
        ));
        assert!(!residual_note_has_exact_token(&n, "invent_pass=1"));
        assert!(!residual_note_has_exact_token(&n, "residual_not_closed=0"));
    }

    #[test]
    fn external_ctrltext_compare_dry_run_lab_not_ready_external_present() {
        // External bytes without lab-ready export → no empty-sha compare.
        let d = research_external_ctrltext_compare_dry_run(
            None,
            Some(b"external-bytes-present"),
            None,
            None,
        );
        assert!(!d.lab_export_ready);
        assert!(d.external_present);
        assert!(!d.blocked_on_external_bytes);
        assert!(!d.compare_ran);
        assert!(!d.sha_match);
        assert!(!d.structural_match);
        assert!(d.residual_not_closed);
        assert!(!d.dry_run_alone_closes_fw_residual);
        assert_eq!(d.detail, "dry_run_lab_export_not_ready_external_present");
        let n = d.format_soft_note();
        assert!(residual_note_has_exact_token(&n, "lab_export_ready=0"));
        assert!(residual_note_has_exact_token(&n, "external_present=1"));
        assert!(residual_note_has_exact_token(&n, "compare_ran=0"));
        assert!(residual_note_has_exact_token(
            &n,
            &format!("{BLOCKED_ON_EXTERNAL_BYTES_TOKEN}=0")
        ));
        assert!(residual_note_has_exact_token(&n, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&n, "product_known_good=0"));
        assert!(residual_note_has_exact_token(&n, "residual_not_closed=1"));
        assert!(residual_note_has_exact_token(
            &n,
            "dry_run_alone_closes_fw_residual=0"
        ));
        // Default lab export (empty sha) is not ready either.
        let empty_lab = CtrltextFingerprintExport::default();
        assert!(!lab_fingerprint_export_is_ready(Some(&empty_lab)));
        let d2 = research_external_ctrltext_compare_dry_run(
            Some(&empty_lab),
            Some(b"still-external"),
            None,
            None,
        );
        assert!(!d2.compare_ran);
        assert!(d2.external_present);
        assert_eq!(d2.detail, "dry_run_lab_export_not_ready_external_present");
    }

    #[test]
    fn external_ctrltext_compare_dry_run_load_refused_distinct_detail() {
        // Soft confinement/read refuse: still blocked_on_external_bytes, distinct detail.
        let d = research_external_ctrltext_compare_dry_run_ex(
            None,
            None,
            Some("allowed_name.ctrltext"),
            Some("/tmp/allowed_name.ctrltext"),
            None,
            None,
            true, // external_load_refused
        );
        assert!(d.blocked_on_external_bytes);
        assert!(!d.external_present);
        assert!(!d.refused_local_post_patch);
        assert!(!d.refused_ve2_not_aie2);
        assert!(!d.compare_ran);
        assert_eq!(d.detail, "dry_run_external_load_refused_blocked_on_bytes");
        let n = d.format_soft_note();
        assert!(residual_note_has_exact_token(
            &n,
            &format!("{BLOCKED_ON_EXTERNAL_BYTES_TOKEN}=1")
        ));
        assert!(residual_note_has_exact_token(&n, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&n, "residual_not_closed=1"));
        // post_patch refuse still wins over load_refused flag.
        let d_post = research_external_ctrltext_compare_dry_run_ex(
            None,
            None,
            Some("instr_post_patch.bin"),
            None,
            None,
            None,
            true,
        );
        assert!(d_post.refused_local_post_patch);
        assert!(!d_post.blocked_on_external_bytes);
        assert_eq!(d_post.detail, "dry_run_refused_local_post_patch");
        // lab-ready + load_refused still uses load-refused detail (bytes missing).
        let ctrltext = build_strix_research_noop_txn().expect("noop");
        let synth = build_minimal_test_elf32(&ctrltext, &[(0_u32, (1_u32 << 8) | 5, 0_i32)]);
        let exp = export_ctrltext_fingerprint_research(&synth);
        let d_lab = research_external_ctrltext_compare_dry_run_ex(
            Some(&exp),
            None,
            None,
            None,
            None,
            None,
            true,
        );
        assert!(d_lab.lab_export_ready);
        assert!(d_lab.blocked_on_external_bytes);
        assert_eq!(
            d_lab.detail,
            "dry_run_external_load_refused_blocked_on_bytes"
        );
    }

    // -----------------------------------------------------------------------
    // Lab export fingerprint auto-feed (stamp d2e287ab)
    // -----------------------------------------------------------------------

    #[test]
    fn lab_export_fingerprint_feed_from_elf_ready_blocked_on_external() {
        let ctrltext = build_strix_research_noop_txn().expect("noop");
        let synth = build_minimal_test_elf32(&ctrltext, &[(0_u32, (1_u32 << 8) | 5, 0_i32)]);
        let feed = research_lab_export_fingerprint_feed_from_elf(Some(&synth));
        assert!(feed.lab_export_ready);
        assert!(feed.lab_export_auto_feed);
        assert_eq!(feed.feed_source, "elf_bytes");
        assert!(!feed.refused_local_post_patch);
        assert!(!feed.platform_not_aie2);
        assert!(feed.residual_not_closed);
        assert!(!feed.feed_alone_closes_fw_residual);
        assert_eq!(feed.detail, "feed_elf_export_ready");
        assert!(feed.ready_export().is_some());

        let n = feed.format_soft_note();
        assert!(residual_note_has_exact_token(
            &n,
            LAB_EXPORT_FINGERPRINT_FEED_NOTE
        ));
        assert!(residual_note_has_exact_token(&n, "lab_export_ready=1"));
        assert!(residual_note_has_exact_token(&n, "lab_export_auto_feed=1"));
        assert!(residual_note_has_exact_token(&n, "feed_source=elf_bytes"));
        assert!(residual_note_has_exact_token(&n, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&n, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&n, "not_ran_on_npu=1"));
        assert!(residual_note_has_exact_token(&n, "product_known_good=0"));
        assert!(residual_note_has_exact_token(&n, "residual_not_closed=1"));
        assert!(residual_note_has_exact_token(
            &n,
            "feed_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "dry_run_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "ctrltext_match_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "scorecard_alone_closes_fw_residual=0"
        ));

        // Auto-feed dry-run: lab ready + bag absent → blocked_on_external_bytes.
        let dry = research_external_ctrltext_compare_dry_run_from_lab_feed(&feed, None, None, None);
        assert!(dry.lab_export_ready);
        assert!(dry.blocked_on_external_bytes);
        assert!(!dry.external_present);
        assert!(!dry.compare_ran);
        assert!(dry.residual_not_closed);
        assert_eq!(
            dry.detail,
            "dry_run_lab_export_ready_blocked_on_external_bytes"
        );
        let dn = dry.format_soft_note();
        assert!(residual_note_has_exact_token(&dn, "lab_export_ready=1"));
        assert!(residual_note_has_exact_token(
            &dn,
            "blocked_on_external_bytes=1"
        ));
        assert!(residual_note_has_exact_token(&dn, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&dn, "residual_not_closed=1"));
    }

    #[test]
    fn lab_export_fingerprint_feed_absent_both_sides() {
        let feed = research_lab_export_fingerprint_feed(None, None, None, None);
        assert!(!feed.lab_export_ready);
        assert_eq!(feed.feed_source, "absent");
        // Combined entry: distinct from ELF-only `feed_elf_absent`.
        assert_eq!(feed.detail, "feed_absent_no_elf_no_signals");
        assert!(feed.ready_export().is_none());
        let n = feed.format_soft_note();
        assert!(residual_note_has_exact_token(&n, "lab_export_ready=0"));
        assert!(residual_note_has_exact_token(&n, "lab_export_auto_feed=1"));
        assert!(residual_note_has_exact_token(&n, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&n, "residual_not_closed=1"));
        assert!(residual_note_has_exact_token(
            &n,
            "detail=feed_absent_no_elf_no_signals"
        ));

        let dry = research_external_ctrltext_compare_dry_run_from_lab_feed(&feed, None, None, None);
        assert!(!dry.lab_export_ready);
        assert!(dry.blocked_on_external_bytes);
        assert_eq!(dry.detail, "dry_run_blocked_on_external_bytes");
    }

    #[test]
    fn lab_export_fingerprint_feed_last_apply_signals_incomplete_no_invent_sha() {
        // aiebu + content size alone never invent unpatched sha / ready export.
        let feed = research_lab_export_fingerprint_feed_from_last_apply_signals(
            Some(true),
            Some(1688),
            Some(12),
        );
        assert!(!feed.lab_export_ready);
        assert_eq!(feed.feed_source, "last_apply_signals");
        assert_eq!(feed.last_apply_content_size_b, Some(1688));
        assert_eq!(feed.last_apply_aiebu_elf, Some(true));
        assert_eq!(feed.last_apply_rela_count, Some(12));
        assert!(feed.export.is_none());
        assert_eq!(
            feed.detail,
            "feed_last_apply_signals_incomplete_need_elf_bytes"
        );
        let n = feed.format_soft_note();
        assert!(residual_note_has_exact_token(
            &n,
            "feed_source=last_apply_signals"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "last_apply_content_size_b=1688"
        ));
        assert!(residual_note_has_exact_token(&n, "lab_export_ready=0"));
        assert!(residual_note_has_exact_token(&n, "invent_pass=0"));
        assert!(residual_note_has_exact_token(
            &n,
            "feed_alone_closes_fw_residual=0"
        ));

        // Prefer ELF when both last_apply detail and ELF present.
        let ctrltext = build_strix_research_noop_txn().expect("noop");
        let synth = build_minimal_test_elf32(&ctrltext, &[(0_u32, (1_u32 << 8) | 5, 0_i32)]);
        let detail =
            "aiebu_elf=1 rela_count=12 content_size=1688B start_npu_size=1688B bo_size=4096B";
        let combined = research_lab_export_fingerprint_feed(Some(&synth), Some(detail), None, None);
        assert!(combined.lab_export_ready);
        assert_eq!(combined.feed_source, "elf_bytes");

        // last_apply detail alone → incomplete signals feed.
        let signals_only = research_lab_export_fingerprint_feed(None, Some(detail), None, None);
        assert!(!signals_only.lab_export_ready);
        assert_eq!(signals_only.feed_source, "last_apply_signals");
        assert_eq!(signals_only.last_apply_content_size_b, Some(1688));
    }

    #[test]
    fn lab_export_fingerprint_feed_refuse_post_patch_and_ve2() {
        let feed =
            research_lab_export_fingerprint_feed(None, None, Some("instr_post_patch.bin"), None);
        assert!(feed.refused_local_post_patch);
        assert!(!feed.lab_export_ready);
        assert_eq!(feed.feed_source, "refused_local_post_patch");
        assert_eq!(feed.detail, "feed_refused_local_post_patch");
        let n = feed.format_soft_note();
        assert!(residual_note_has_exact_token(
            &n,
            "refused_local_post_patch=1"
        ));
        assert!(residual_note_has_exact_token(&n, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&n, "product_known_good=0"));

        let dry = research_external_ctrltext_compare_dry_run_from_lab_feed(
            &feed,
            Some(b"not-real"),
            None,
            None,
        );
        assert!(dry.refused_local_post_patch);
        assert!(!dry.blocked_on_external_bytes);
        assert_eq!(dry.detail, "dry_run_refused_local_post_patch");

        // Issue 1: feed refuse must not drop when caller passes non-refused basename.
        let dry_clobber = research_external_ctrltext_compare_dry_run_from_lab_feed(
            &feed,
            Some(b"not-real-external-bytes"),
            Some("external_good.bin"),
            None,
        );
        assert!(dry_clobber.refused_local_post_patch);
        assert!(!dry_clobber.compare_ran);
        assert!(!dry_clobber.blocked_on_external_bytes);
        assert_eq!(dry_clobber.detail, "dry_run_refused_local_post_patch");
        assert!(residual_note_has_exact_token(
            &dry_clobber.format_soft_note(),
            "refused_local_post_patch=1"
        ));

        let ve2 = research_lab_export_fingerprint_feed(
            None,
            None,
            None,
            Some("ve2_latency validate.xclbin NOT_AIE2_KNOWN_GOOD=1"),
        );
        assert!(ve2.platform_not_aie2);
        assert!(!ve2.lab_export_ready);
        assert_eq!(ve2.feed_source, "platform_not_aie2");
        let vn = ve2.format_soft_note();
        assert!(residual_note_has_exact_token(&vn, "platform_not_aie2=1"));
        assert!(residual_note_has_exact_token(&vn, "invent_pass=0"));

        let dry_ve2 =
            research_external_ctrltext_compare_dry_run_from_lab_feed(&ve2, None, None, None);
        assert!(dry_ve2.refused_ve2_not_aie2);
        assert!(dry_ve2.platform_not_aie2);
        assert!(!dry_ve2.blocked_on_external_bytes);
        assert_eq!(dry_ve2.detail, "dry_run_refused_ve2_not_aie2");

        // Issue 1: feed platform_not_aie2 must not drop when caller passes non-VE2 claim.
        let dry_ve2_clobber = research_external_ctrltext_compare_dry_run_from_lab_feed(
            &ve2,
            Some(b"not-real-external-bytes"),
            None,
            Some("PLATFORM=AIE2_STX research_only"),
        );
        assert!(dry_ve2_clobber.refused_ve2_not_aie2);
        assert!(dry_ve2_clobber.platform_not_aie2);
        assert!(!dry_ve2_clobber.compare_ran);
        assert!(!dry_ve2_clobber.blocked_on_external_bytes);
        assert_eq!(dry_ve2_clobber.detail, "dry_run_refused_ve2_not_aie2");
        assert!(residual_note_has_exact_token(
            &dry_ve2_clobber.format_soft_note(),
            "platform_not_aie2=1"
        ));
    }

    #[test]
    fn lab_export_fingerprint_feed_honesty_hard_coded_not_mutable() {
        let mut feed = research_lab_export_fingerprint_feed(None, None, None, None);
        feed.residual_not_closed = false;
        feed.feed_alone_closes_fw_residual = true;
        feed.lab_export_ready = true;
        feed.lab_export_auto_feed = false;
        let n = feed.format_soft_note();
        // Honesty locks stay hard-coded even if fields mutated.
        assert!(residual_note_has_exact_token(&n, "residual_not_closed=1"));
        assert!(residual_note_has_exact_token(
            &n,
            "feed_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "dry_run_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "ctrltext_match_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "scorecard_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(&n, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&n, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&n, "not_ran_on_npu=1"));
        assert!(residual_note_has_exact_token(&n, "product_known_good=0"));
        // lab_export_auto_feed always hard-coded 1 in soft note.
        assert!(residual_note_has_exact_token(&n, "lab_export_auto_feed=1"));
        // Mutable ready bit may echo.
        assert!(residual_note_has_exact_token(&n, "lab_export_ready=1"));
        assert!(!residual_note_has_exact_token(&n, "invent_pass=1"));
        assert!(!residual_note_has_exact_token(&n, "residual_not_closed=0"));
        assert!(!residual_note_has_exact_token(
            &n,
            "feed_alone_closes_fw_residual=1"
        ));
    }

    #[test]
    fn lab_export_fingerprint_feed_match_neq_pass_via_auto_feed() {
        let ctrltext = build_strix_research_noop_txn().expect("noop");
        let synth = build_minimal_test_elf32(&ctrltext, &[(0_u32, (1_u32 << 8) | 5, 0_i32)]);
        let feed = research_lab_export_fingerprint_feed_from_elf(Some(&synth));
        assert!(feed.lab_export_ready);
        let dry = research_external_ctrltext_compare_dry_run_from_lab_feed(
            &feed,
            Some(&ctrltext),
            None,
            None,
        );
        assert!(dry.lab_export_ready);
        assert!(dry.external_present);
        assert!(!dry.blocked_on_external_bytes);
        assert!(dry.compare_ran);
        assert!(dry.sha_match);
        assert!(dry.structural_match);
        assert!(dry.residual_not_closed);
        assert!(!dry.dry_run_alone_closes_fw_residual);
        assert_eq!(
            dry.detail,
            "dry_run_sha_and_structural_match_residual_still_open"
        );
        let n = dry.format_soft_note();
        assert!(residual_note_has_exact_token(&n, "sha_match=1"));
        assert!(residual_note_has_exact_token(&n, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&n, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&n, "product_known_good=0"));
        assert!(residual_note_has_exact_token(&n, "residual_not_closed=1"));
        assert!(residual_note_has_exact_token(
            &n,
            "dry_run_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "ctrltext_match_alone_closes_fw_residual=0"
        ));
        assert!(!residual_note_has_exact_token(&n, "invent_pass=1"));
    }

    #[test]
    fn lab_export_fingerprint_feed_empty_elf_not_ready() {
        let feed = research_lab_export_fingerprint_feed_from_elf(Some(&[]));
        assert!(!feed.lab_export_ready);
        assert_eq!(feed.feed_source, "absent");
        assert_eq!(feed.detail, "feed_elf_absent");
        let non_aiebu = research_lab_export_fingerprint_feed_from_elf(Some(b"not-elf"));
        assert!(!non_aiebu.lab_export_ready);
        assert_eq!(non_aiebu.feed_source, "elf_bytes");
        assert_eq!(non_aiebu.detail, "feed_elf_export_not_ready");
        assert!(non_aiebu.export.is_some());
        assert!(non_aiebu.ready_export().is_none());
    }

    #[test]
    fn lab_export_fingerprint_feed_ready_export_requires_export_ready() {
        // Hand-mutated bag: lab_export_ready=true with missing/empty export must not
        // invent a ready export borrow for dry-run auto-feed.
        let mut feed = research_lab_export_fingerprint_feed(None, None, None, None);
        feed.lab_export_ready = true;
        feed.export = None;
        assert!(feed.ready_export().is_none());
        let dry = research_external_ctrltext_compare_dry_run_from_lab_feed(&feed, None, None, None);
        assert!(!dry.lab_export_ready);
        assert!(dry.blocked_on_external_bytes);
    }

    // -----------------------------------------------------------------------
    // Bag + feed + dry-run composition scorecard (stamp 2b01425e)
    // -----------------------------------------------------------------------

    #[test]
    fn app_load_bag_feed_dry_run_scorecard_c2_locked_absent_blocked() {
        let s = research_app_load_bag_feed_dry_run_scorecard_c2_locked_absent_bag();
        assert!(!s.bag_drop_complete);
        assert!(!s.bag_ctrltext_bytes_present);
        assert!(!s.lab_export_ready);
        assert!(s.lab_export_auto_feed);
        assert!(s.blocked_on_external_bytes);
        assert!(!s.external_present);
        assert!(!s.compare_ran);
        assert!(!s.refused_local_post_patch);
        assert!(!s.refused_ve2_not_aie2);
        assert!(s.residual_not_closed);
        assert!(!s.scorecard_alone_closes_fw_residual);
        assert!(!s.feed_alone_closes_fw_residual);
        assert!(!s.dry_run_alone_closes_fw_residual);
        assert!(!s.bag_drop_alone_closes_fw_residual);
        assert_eq!(
            s.detail,
            "scorecard_blocked_on_external_bytes_lab_not_ready"
        );

        let n = s.format_soft_note();
        assert!(residual_note_has_exact_token(
            &n,
            APP_LOAD_BAG_FEED_DRY_RUN_SCORECARD_NOTE
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "blocked_on_external_bytes=1"
        ));
        assert!(residual_note_has_exact_token(&n, "lab_export_ready=0"));
        assert!(residual_note_has_exact_token(&n, "lab_export_auto_feed=1"));
        assert!(residual_note_has_exact_token(&n, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&n, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&n, "not_ran_on_npu=1"));
        assert!(residual_note_has_exact_token(&n, "product_known_good=0"));
        assert!(residual_note_has_exact_token(&n, "residual_not_closed=1"));
        assert!(residual_note_has_exact_token(
            &n,
            "scorecard_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "feed_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "dry_run_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "bag_drop_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "ctrltext_match_alone_closes_fw_residual=0"
        ));
        assert!(!residual_note_has_exact_token(&n, "invent_pass=1"));
        assert!(!residual_note_has_exact_token(&n, "residual_not_closed=0"));
    }

    #[test]
    fn app_load_bag_feed_dry_run_scorecard_lab_ready_blocked_on_external() {
        let ctrltext = build_strix_research_noop_txn().expect("noop");
        let synth = build_minimal_test_elf32(&ctrltext, &[(0_u32, (1_u32 << 8) | 5, 0_i32)]);
        let bag = check_external_known_good_bag_drop(false, None, None, None);
        let feed = research_lab_export_fingerprint_feed_from_elf(Some(&synth));
        assert!(feed.lab_export_ready);
        let s = research_app_load_bag_feed_dry_run_scorecard(&bag, &feed, None, None, None);
        assert!(s.lab_export_ready);
        assert!(s.blocked_on_external_bytes);
        assert!(!s.external_present);
        assert!(!s.compare_ran);
        assert!(s.residual_not_closed);
        assert_eq!(s.detail, "scorecard_lab_ready_blocked_on_external_bytes");
        let n = s.format_soft_note();
        assert!(residual_note_has_exact_token(&n, "lab_export_ready=1"));
        assert!(residual_note_has_exact_token(
            &n,
            "blocked_on_external_bytes=1"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "detail=scorecard_lab_ready_blocked_on_external_bytes"
        ));
        assert!(residual_note_has_exact_token(&n, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&n, "residual_not_closed=1"));
        assert!(residual_note_has_exact_token(
            &n,
            "scorecard_alone_closes_fw_residual=0"
        ));
    }

    #[test]
    fn app_load_bag_feed_dry_run_scorecard_refuse_post_patch_and_ve2() {
        let bag_post = check_external_known_good_bag_drop(
            true,
            Some("Research only. Not product Pass. Not C-ready.\nSource stack: XRT public stack"),
            Some("instr_post_patch.bin"),
            None,
        );
        assert!(bag_post.refused_local_post_patch_basename);
        let feed = research_lab_export_fingerprint_feed(None, None, None, None);
        let s_post = research_app_load_bag_feed_dry_run_scorecard(
            &bag_post,
            &feed,
            Some(b"not-real"),
            Some("external_good.bin"), // non-refused caller — must not clobber bag refuse
            None,
        );
        assert!(s_post.refused_local_post_patch);
        assert!(!s_post.blocked_on_external_bytes);
        assert!(!s_post.compare_ran);
        assert_eq!(
            s_post.detail,
            "scorecard_refused_local_post_patch_residual_open"
        );
        assert!(residual_note_has_exact_token(
            &s_post.format_soft_note(),
            "refused_local_post_patch=1"
        ));
        assert!(residual_note_has_exact_token(
            &s_post.format_soft_note(),
            "invent_pass=0"
        ));

        // Feed refuse path (no bag refuse) still forces refuse.
        let bag_ok = check_external_known_good_bag_drop(false, None, None, None);
        let feed_post =
            research_lab_export_fingerprint_feed(None, None, Some("instr_post_patch.bin"), None);
        let s_feed = research_app_load_bag_feed_dry_run_scorecard(
            &bag_ok,
            &feed_post,
            Some(b"not-real"),
            Some("external_good.bin"),
            None,
        );
        assert!(s_feed.refused_local_post_patch);
        assert!(!s_feed.blocked_on_external_bytes);
        assert_eq!(
            s_feed.detail,
            "scorecard_refused_local_post_patch_residual_open"
        );

        // VE2 refuse + platform_not_aie2.
        let bag_ve2 = check_external_known_good_bag_drop(
            true,
            Some("Research only. Not product Pass. Not C-ready.\nSource stack: XRT public stack"),
            Some("ve2_ctrltext.bin"),
            Some("PLATFORM=VE2_EDGE NOT_AIE2_KNOWN_GOOD=1"),
        );
        assert!(bag_ve2.refused_ve2_not_aie2);
        let s_ve2 = research_app_load_bag_feed_dry_run_scorecard(
            &bag_ve2,
            &feed,
            Some(b"not-real"),
            None,
            Some("PLATFORM=AIE2_STX research_only"), // non-VE2 caller — must not clobber
        );
        assert!(s_ve2.refused_ve2_not_aie2);
        assert!(s_ve2.platform_not_aie2);
        assert!(!s_ve2.blocked_on_external_bytes);
        assert!(!s_ve2.compare_ran);
        assert_eq!(s_ve2.detail, "scorecard_refused_ve2_not_aie2_residual_open");
        let vn = s_ve2.format_soft_note();
        assert!(residual_note_has_exact_token(&vn, "platform_not_aie2=1"));
        assert!(residual_note_has_exact_token(&vn, "refused_ve2_not_aie2=1"));
        assert!(residual_note_has_exact_token(&vn, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&vn, "residual_not_closed=1"));
    }

    #[test]
    fn app_load_bag_feed_dry_run_scorecard_match_neq_pass() {
        let ctrltext = build_strix_research_noop_txn().expect("noop");
        let synth = build_minimal_test_elf32(&ctrltext, &[(0_u32, (1_u32 << 8) | 5, 0_i32)]);
        let ready = "Research only. Not product Pass. Not C-ready.\nSource stack: XRT public stack";
        let bag = check_external_known_good_bag_drop(
            true,
            Some(ready),
            Some("working_stack_ctrltext.bin"),
            Some("Source stack: working public XRT AIE2"),
        );
        assert!(bag.bag_drop_complete);
        let feed = research_lab_export_fingerprint_feed_from_elf(Some(&synth));
        let s =
            research_app_load_bag_feed_dry_run_scorecard(&bag, &feed, Some(&ctrltext), None, None);
        assert!(s.bag_drop_complete);
        assert!(s.lab_export_ready);
        assert!(s.external_present);
        assert!(!s.blocked_on_external_bytes);
        assert!(s.compare_ran);
        assert!(s.sha_match);
        assert!(s.structural_match);
        assert!(s.residual_not_closed);
        assert!(!s.scorecard_alone_closes_fw_residual);
        assert_eq!(
            s.detail,
            "scorecard_sha_and_structural_match_residual_still_open"
        );
        let n = s.format_soft_note();
        assert!(residual_note_has_exact_token(&n, "sha_match=1"));
        assert!(residual_note_has_exact_token(&n, "compare_ran=1"));
        assert!(residual_note_has_exact_token(&n, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&n, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&n, "product_known_good=0"));
        assert!(residual_note_has_exact_token(&n, "residual_not_closed=1"));
        assert!(residual_note_has_exact_token(
            &n,
            "scorecard_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "ctrltext_match_alone_closes_fw_residual=0"
        ));
        assert!(!residual_note_has_exact_token(&n, "invent_pass=1"));
        assert!(!residual_note_has_exact_token(&n, "residual_not_closed=0"));
    }

    #[test]
    fn app_load_bag_feed_dry_run_scorecard_honesty_hard_coded_not_mutable() {
        let mut s = research_app_load_bag_feed_dry_run_scorecard_c2_locked_absent_bag();
        s.residual_not_closed = false;
        s.scorecard_alone_closes_fw_residual = true;
        s.feed_alone_closes_fw_residual = true;
        s.dry_run_alone_closes_fw_residual = true;
        s.bag_drop_alone_closes_fw_residual = true;
        s.lab_export_auto_feed = false;
        s.blocked_on_external_bytes = false;
        let n = s.format_soft_note();
        // Honesty locks stay hard-coded even if fields mutated.
        assert!(residual_note_has_exact_token(&n, "residual_not_closed=1"));
        assert!(residual_note_has_exact_token(
            &n,
            "scorecard_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "feed_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "dry_run_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "bag_drop_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "ctrltext_match_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(&n, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&n, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&n, "not_ran_on_npu=1"));
        assert!(residual_note_has_exact_token(&n, "product_known_good=0"));
        assert!(residual_note_has_exact_token(&n, "lab_export_auto_feed=1"));
        // Mutable blocked bit may echo false after mutation.
        assert!(residual_note_has_exact_token(
            &n,
            "blocked_on_external_bytes=0"
        ));
        assert!(!residual_note_has_exact_token(&n, "invent_pass=1"));
        assert!(!residual_note_has_exact_token(&n, "residual_not_closed=0"));
        assert!(!residual_note_has_exact_token(
            &n,
            "scorecard_alone_closes_fw_residual=1"
        ));
    }

    #[test]
    fn app_load_bag_feed_dry_run_scorecard_external_load_refused_detail() {
        let bag = check_external_known_good_bag_drop(false, None, None, None);
        let feed = research_lab_export_fingerprint_feed(None, None, None, None);
        let s = research_app_load_bag_feed_dry_run_scorecard_ex(
            &bag, &feed, None, None, None, true, // external_load_refused
        );
        assert!(s.blocked_on_external_bytes);
        assert_eq!(s.detail, "scorecard_external_load_refused_blocked_on_bytes");
        assert!(s.residual_not_closed);
        assert!(residual_note_has_exact_token(
            &s.format_soft_note(),
            "blocked_on_external_bytes=1"
        ));
        assert!(residual_note_has_exact_token(
            &s.format_soft_note(),
            "invent_pass=0"
        ));
    }

    #[test]
    fn app_load_bag_feed_dry_run_scorecard_compare_detail_tokens_and_lab_not_ready() {
        // Lab not ready + external present → distinct residual-open detail (no invent Pass).
        let bag_absent = check_external_known_good_bag_drop(false, None, None, None);
        let feed_absent = research_lab_export_fingerprint_feed(None, None, None, None);
        assert!(!feed_absent.lab_export_ready);
        let s_lab = research_app_load_bag_feed_dry_run_scorecard(
            &bag_absent,
            &feed_absent,
            Some(b"external-present-lab-not-ready"),
            None,
            None,
        );
        assert!(!s_lab.lab_export_ready);
        assert!(s_lab.external_present);
        assert!(!s_lab.blocked_on_external_bytes);
        assert!(!s_lab.compare_ran);
        assert!(s_lab.residual_not_closed);
        assert_eq!(
            s_lab.detail,
            "scorecard_lab_export_not_ready_external_present_residual_open"
        );
        let ln = s_lab.format_soft_note();
        assert!(residual_note_has_exact_token(&ln, "external_present=1"));
        assert!(residual_note_has_exact_token(&ln, "lab_export_ready=0"));
        assert!(residual_note_has_exact_token(&ln, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&ln, "residual_not_closed=1"));
        assert!(residual_note_has_exact_token(
            &ln,
            "scorecard_alone_closes_fw_residual=0"
        ));

        // Classic lab + flipped first byte → structural may hold, sha differs.
        let ctrltext = build_strix_research_noop_txn().expect("noop");
        let synth = build_minimal_test_elf32(&ctrltext, &[(0_u32, (1_u32 << 8) | 5, 0_i32)]);
        let feed = research_lab_export_fingerprint_feed_from_elf(Some(&synth));
        assert!(feed.lab_export_ready);
        let mut flipped = ctrltext.clone();
        if let Some(b) = flipped.first_mut() {
            *b ^= 0xff;
        }
        let s_flip = research_app_load_bag_feed_dry_run_scorecard(
            &bag_absent,
            &feed,
            Some(&flipped),
            None,
            None,
        );
        assert!(s_flip.compare_ran);
        assert!(!s_flip.sha_match);
        assert!(!s_flip.blocked_on_external_bytes);
        assert!(s_flip.residual_not_closed);
        if s_flip.structural_match {
            assert_eq!(
                s_flip.detail,
                "scorecard_structural_match_sha_diff_residual_open"
            );
            assert!(residual_note_has_exact_token(
                &s_flip.format_soft_note(),
                "structural_match=1"
            ));
        } else {
            assert_eq!(s_flip.detail, "scorecard_mismatch_residual_open");
        }
        assert!(residual_note_has_exact_token(
            &s_flip.format_soft_note(),
            "sha_match=0"
        ));
        assert!(residual_note_has_exact_token(
            &s_flip.format_soft_note(),
            "invent_pass=0"
        ));

        // Pure mismatch: classic lab vs unrelated garbage (neither sha nor structural).
        let garbage = b"not-classic-txn-garbage-payload-mismatch-path";
        let s_mis = research_app_load_bag_feed_dry_run_scorecard(
            &bag_absent,
            &feed,
            Some(garbage),
            None,
            None,
        );
        assert!(s_mis.compare_ran);
        assert!(!s_mis.sha_match);
        assert!(!s_mis.structural_match);
        assert_eq!(s_mis.detail, "scorecard_mismatch_residual_open");
        assert!(s_mis.residual_not_closed);
        let mn = s_mis.format_soft_note();
        assert!(residual_note_has_exact_token(&mn, "sha_match=0"));
        assert!(residual_note_has_exact_token(&mn, "structural_match=0"));
        assert!(residual_note_has_exact_token(&mn, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&mn, "residual_not_closed=1"));

        // sha_match without structural: hand-built ready export (non-classic lab).
        let same = b"fixed-garbage-bytes-for-sha-only-path";
        let mut exp = CtrltextFingerprintExport::default();
        exp.unpatched_sha256_hex = sha256_hex_lower(same);
        exp.ctrltext_len = Some(same.len());
        exp.classic_walk_ok = false;
        exp.known_good_status = KNOWN_GOOD_CTRLTEXT_ABSENT_TOKEN;
        let mut feed_sha = LabExportFingerprintFeed::default();
        feed_sha.lab_export_ready = true;
        feed_sha.export = Some(exp);
        feed_sha.feed_source = "unit_hand_export".into();
        assert!(feed_sha.ready_export().is_some());
        let s_sha = research_app_load_bag_feed_dry_run_scorecard(
            &bag_absent,
            &feed_sha,
            Some(same),
            None,
            None,
        );
        assert!(s_sha.compare_ran);
        assert!(s_sha.sha_match);
        assert!(!s_sha.structural_match);
        assert_eq!(s_sha.detail, "scorecard_sha_match_residual_still_open");
        assert!(s_sha.residual_not_closed);
        let sn = s_sha.format_soft_note();
        assert!(residual_note_has_exact_token(&sn, "sha_match=1"));
        assert!(residual_note_has_exact_token(&sn, "structural_match=0"));
        assert!(residual_note_has_exact_token(&sn, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&sn, "residual_not_closed=1"));
        assert!(residual_note_has_exact_token(
            &sn,
            "scorecard_alone_closes_fw_residual=0"
        ));

        // Default bag: blocked_on_external_bytes + honesty (not mutable invent Pass).
        let d = AppLoadBagFeedDryRunScorecard::default();
        assert!(d.blocked_on_external_bytes);
        assert!(d.residual_not_closed);
        assert!(!d.scorecard_alone_closes_fw_residual);
        assert_eq!(d.detail, "scorecard_default_blocked_on_external_bytes");
        let dn = d.format_soft_note();
        assert!(residual_note_has_exact_token(
            &dn,
            "blocked_on_external_bytes=1"
        ));
        assert!(residual_note_has_exact_token(&dn, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&dn, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&dn, "residual_not_closed=1"));
        assert!(residual_note_has_exact_token(
            &dn,
            "scorecard_alone_closes_fw_residual=0"
        ));
    }

    // -----------------------------------------------------------------------
    // Extract → bag-drop → composition scorecard pipeline (stamp 588c1c72)
    // -----------------------------------------------------------------------

    #[test]
    fn app_load_extract_bag_scorecard_pipeline_c2_locked_absent_blocked() {
        let p = research_app_load_extract_bag_scorecard_pipeline_c2_locked_absent_bag();
        assert!(!p.extract_magic_xclbin2);
        assert!(!p.extract_has_aie_partition_marker);
        assert_eq!(p.extract_section_marker_count, 0);
        assert!(!p.extract_eligible_as_aie2_external_known_good);
        assert!(!p.bag_drop_complete);
        assert!(!p.bag_ctrltext_bytes_present);
        assert!(!p.lab_export_ready);
        assert!(p.lab_export_auto_feed);
        assert!(p.blocked_on_external_bytes);
        assert!(!p.external_present);
        assert!(!p.compare_ran);
        assert!(!p.refused_local_post_patch);
        assert!(!p.refused_ve2_not_aie2);
        assert!(p.residual_not_closed);
        assert!(!p.pipeline_alone_closes_fw_residual);
        assert!(!p.extract_alone_closes_fw_residual);
        assert!(!p.scorecard_alone_closes_fw_residual);
        assert_eq!(p.detail, "pipeline_blocked_on_external_bytes_lab_not_ready");

        let n = p.format_soft_note();
        assert!(residual_note_has_exact_token(
            &n,
            APP_LOAD_EXTRACT_BAG_SCORECARD_PIPELINE_NOTE
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "blocked_on_external_bytes=1"
        ));
        assert!(residual_note_has_exact_token(&n, "lab_export_ready=0"));
        assert!(residual_note_has_exact_token(&n, "lab_export_auto_feed=1"));
        assert!(residual_note_has_exact_token(&n, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&n, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&n, "not_ran_on_npu=1"));
        assert!(residual_note_has_exact_token(&n, "product_known_good=0"));
        assert!(residual_note_has_exact_token(&n, "residual_not_closed=1"));
        assert!(residual_note_has_exact_token(
            &n,
            "pipeline_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "extract_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "scorecard_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "ctrltext_match_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "do_not_invent_pdi_layout=1"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "public_pdi_image_decode=0"
        ));
        assert!(!residual_note_has_exact_token(&n, "invent_pass=1"));
        assert!(!residual_note_has_exact_token(&n, "residual_not_closed=0"));
    }

    #[test]
    fn app_load_extract_bag_scorecard_pipeline_lab_ready_blocked_on_external() {
        // Common soft offline path: empty extract (no magic/markers), absent bag,
        // lab ELF export ready for fingerprint feed, external bag bytes still missing.
        // Locks distinct detail `pipeline_lab_ready_blocked_on_external_bytes` so a
        // future blocked-detail ladder reorder cannot silently remap this path.
        let extract = research_xclbin_extract_dry_run(&[], None, None);
        assert!(!extract.magic_xclbin2);
        assert!(!extract.has_aie_partition_marker);
        assert_eq!(extract.section_marker_count, 0);
        assert!(!extract.eligible_as_aie2_external_known_good);
        let bag = check_external_known_good_bag_drop(false, None, None, None);
        assert!(!bag.bag_drop_complete);
        let ctrltext = build_strix_research_noop_txn().expect("noop");
        let synth = build_minimal_test_elf32(&ctrltext, &[(0_u32, (1_u32 << 8) | 5, 0_i32)]);
        let feed = research_lab_export_fingerprint_feed_from_elf(Some(&synth));
        assert!(feed.lab_export_ready);
        let p = research_app_load_extract_bag_scorecard_pipeline(
            &extract, &bag, &feed, None, None, None,
        );
        assert!(p.lab_export_ready);
        assert!(p.lab_export_auto_feed);
        assert!(p.blocked_on_external_bytes);
        assert!(!p.external_present);
        assert!(!p.compare_ran);
        assert!(!p.extract_magic_xclbin2);
        assert!(!p.extract_has_aie_partition_marker);
        assert!(!p.extract_eligible_as_aie2_external_known_good);
        assert!(!p.bag_drop_complete);
        assert!(!p.refused_local_post_patch);
        assert!(!p.refused_ve2_not_aie2);
        assert!(p.residual_not_closed);
        assert!(!p.pipeline_alone_closes_fw_residual);
        assert!(!p.extract_alone_closes_fw_residual);
        assert!(!p.scorecard_alone_closes_fw_residual);
        assert_eq!(p.detail, "pipeline_lab_ready_blocked_on_external_bytes");

        let n = p.format_soft_note();
        assert!(residual_note_has_exact_token(
            &n,
            APP_LOAD_EXTRACT_BAG_SCORECARD_PIPELINE_NOTE
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "detail=pipeline_lab_ready_blocked_on_external_bytes"
        ));
        assert!(residual_note_has_exact_token(&n, "lab_export_ready=1"));
        assert!(residual_note_has_exact_token(
            &n,
            "blocked_on_external_bytes=1"
        ));
        assert!(residual_note_has_exact_token(&n, "lab_export_auto_feed=1"));
        assert!(residual_note_has_exact_token(&n, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&n, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&n, "not_ran_on_npu=1"));
        assert!(residual_note_has_exact_token(&n, "product_known_good=0"));
        assert!(residual_note_has_exact_token(&n, "residual_not_closed=1"));
        assert!(residual_note_has_exact_token(
            &n,
            "pipeline_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "extract_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "scorecard_alone_closes_fw_residual=0"
        ));
        assert!(!residual_note_has_exact_token(&n, "invent_pass=1"));
        assert!(!residual_note_has_exact_token(&n, "residual_not_closed=0"));
        // Distinct from lab-not-ready and marker-blocked tokens.
        assert!(!residual_note_has_exact_token(
            &n,
            "detail=pipeline_blocked_on_external_bytes_lab_not_ready"
        ));
        assert!(!residual_note_has_exact_token(
            &n,
            "detail=pipeline_extract_magic_only_blocked_on_external_bytes"
        ));
    }

    #[test]
    fn app_load_extract_bag_scorecard_pipeline_markers_present_still_blocked() {
        // Public section markers without ctrltext bytes → still blocked, residual open.
        let header = XCLBIN2_MAGIC.as_slice();
        let section_text = "Section: AIE_PARTITION\nSection: BITSTREAM\nSection: MEM_TOPOLOGY\n";
        let extract = research_xclbin_extract_dry_run(
            header,
            Some(section_text),
            Some("Source stack: working public XRT AIE2 DynamicDispatch"),
        );
        assert!(extract.magic_xclbin2);
        assert!(extract.has_aie_partition_marker);
        assert!(extract.eligible_as_aie2_external_known_good);
        let bag = check_external_known_good_bag_drop(false, None, None, None);
        let feed = research_lab_export_fingerprint_feed(None, None, None, None);
        let p = research_app_load_extract_bag_scorecard_pipeline(
            &extract, &bag, &feed, None, None, None,
        );
        assert!(p.extract_magic_xclbin2);
        assert!(p.extract_has_aie_partition_marker);
        assert!(p.extract_eligible_as_aie2_external_known_good);
        assert!(!p.bag_drop_complete);
        assert!(p.blocked_on_external_bytes);
        assert!(!p.compare_ran);
        assert!(p.residual_not_closed);
        assert!(!p.pipeline_alone_closes_fw_residual);
        assert!(!p.extract_alone_closes_fw_residual);
        assert_eq!(
            p.detail,
            "pipeline_extract_eligible_research_only_blocked_on_external_bytes"
        );
        let n = p.format_soft_note();
        assert!(residual_note_has_exact_token(
            &n,
            "blocked_on_external_bytes=1"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "extract_eligible_as_aie2_external_known_good=1"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "extract_has_aie_partition_marker=1"
        ));
        assert!(residual_note_has_exact_token(&n, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&n, "product_known_good=0"));
        assert!(residual_note_has_exact_token(&n, "residual_not_closed=1"));
        assert!(residual_note_has_exact_token(
            &n,
            "pipeline_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "extract_alone_closes_fw_residual=0"
        ));

        // Markers present, not research-eligible (no working-stack claim) — distinct token.
        let markers_only = research_xclbin_extract_dry_run(header, Some(section_text), None);
        assert!(!markers_only.eligible_as_aie2_external_known_good);
        assert!(markers_only.has_aie_partition_marker);
        let p2 = research_app_load_extract_bag_scorecard_pipeline(
            &markers_only,
            &bag,
            &feed,
            None,
            None,
            None,
        );
        assert!(p2.blocked_on_external_bytes);
        assert_eq!(
            p2.detail,
            "pipeline_extract_markers_present_blocked_on_external_bytes"
        );
        assert!(residual_note_has_exact_token(
            &p2.format_soft_note(),
            "detail=pipeline_extract_markers_present_blocked_on_external_bytes"
        ));

        // Magic only (no section markers) — distinct token.
        let magic_only = research_xclbin_extract_dry_run(header, None, None);
        assert!(magic_only.magic_xclbin2);
        assert!(!magic_only.has_aie_partition_marker);
        let p3 = research_app_load_extract_bag_scorecard_pipeline(
            &magic_only,
            &bag,
            &feed,
            None,
            None,
            None,
        );
        assert!(p3.blocked_on_external_bytes);
        assert_eq!(
            p3.detail,
            "pipeline_extract_magic_only_blocked_on_external_bytes"
        );
    }

    #[test]
    fn app_load_extract_bag_scorecard_pipeline_refuse_post_patch_and_ve2() {
        let extract_ok = research_xclbin_extract_dry_run(&[], None, None);
        let bag_post = check_external_known_good_bag_drop(
            true,
            Some("Research only. Not product Pass. Not C-ready.\nSource stack: XRT public stack"),
            Some("instr_post_patch.bin"),
            None,
        );
        assert!(bag_post.refused_local_post_patch_basename);
        let feed = research_lab_export_fingerprint_feed(None, None, None, None);
        let p_post = research_app_load_extract_bag_scorecard_pipeline(
            &extract_ok,
            &bag_post,
            &feed,
            Some(b"not-real"),
            Some("external_good.bin"), // non-refused caller — must not clobber bag refuse
            None,
        );
        assert!(p_post.refused_local_post_patch);
        assert!(!p_post.blocked_on_external_bytes);
        assert!(!p_post.compare_ran);
        assert_eq!(
            p_post.detail,
            "pipeline_refused_local_post_patch_residual_open"
        );
        assert!(residual_note_has_exact_token(
            &p_post.format_soft_note(),
            "refused_local_post_patch=1"
        ));
        assert!(residual_note_has_exact_token(
            &p_post.format_soft_note(),
            "invent_pass=0"
        ));

        // Extract VE2 refuse forces pipeline refuse even with non-VE2 caller platform.
        let extract_ve2 = research_xclbin_extract_ve2_fixture_refused(&[], None);
        assert!(extract_ve2.platform_not_aie2_known_good);
        let bag_ok = check_external_known_good_bag_drop(false, None, None, None);
        let p_ve2 = research_app_load_extract_bag_scorecard_pipeline(
            &extract_ve2,
            &bag_ok,
            &feed,
            Some(b"not-real"),
            None,
            Some("PLATFORM=AIE2_STX research_only"), // non-VE2 caller — must not clobber
        );
        assert!(p_ve2.refused_ve2_not_aie2);
        assert!(p_ve2.platform_not_aie2);
        assert!(p_ve2.extract_platform_not_aie2_known_good);
        assert!(!p_ve2.blocked_on_external_bytes);
        assert!(!p_ve2.compare_ran);
        assert_eq!(p_ve2.detail, "pipeline_refused_ve2_not_aie2_residual_open");
        let vn = p_ve2.format_soft_note();
        assert!(residual_note_has_exact_token(&vn, "platform_not_aie2=1"));
        assert!(residual_note_has_exact_token(&vn, "refused_ve2_not_aie2=1"));
        assert!(residual_note_has_exact_token(
            &vn,
            "extract_platform_not_aie2_known_good=1"
        ));
        assert!(residual_note_has_exact_token(&vn, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&vn, "residual_not_closed=1"));
    }

    #[test]
    fn app_load_extract_bag_scorecard_pipeline_match_neq_pass() {
        let ctrltext = build_strix_research_noop_txn().expect("noop");
        let synth = build_minimal_test_elf32(&ctrltext, &[(0_u32, (1_u32 << 8) | 5, 0_i32)]);
        let ready = "Research only. Not product Pass. Not C-ready.\nSource stack: XRT public stack";
        let bag = check_external_known_good_bag_drop(
            true,
            Some(ready),
            Some("working_stack_ctrltext.bin"),
            Some("Source stack: working public XRT AIE2"),
        );
        assert!(bag.bag_drop_complete);
        let feed = research_lab_export_fingerprint_feed_from_elf(Some(&synth));
        let header = XCLBIN2_MAGIC.as_slice();
        let section_text = "AIE_PARTITION BITSTREAM";
        let extract = research_xclbin_extract_dry_run(
            header,
            Some(section_text),
            Some("Source stack: working public XRT AIE2"),
        );
        let p = research_app_load_extract_bag_scorecard_pipeline(
            &extract,
            &bag,
            &feed,
            Some(&ctrltext),
            None,
            None,
        );
        assert!(p.bag_drop_complete);
        assert!(p.lab_export_ready);
        assert!(p.external_present);
        assert!(!p.blocked_on_external_bytes);
        assert!(p.compare_ran);
        assert!(p.sha_match);
        assert!(p.structural_match);
        assert!(p.residual_not_closed);
        assert!(!p.pipeline_alone_closes_fw_residual);
        assert!(!p.extract_alone_closes_fw_residual);
        assert_eq!(
            p.detail,
            "pipeline_sha_and_structural_match_residual_still_open"
        );
        let n = p.format_soft_note();
        assert!(residual_note_has_exact_token(&n, "sha_match=1"));
        assert!(residual_note_has_exact_token(&n, "compare_ran=1"));
        assert!(residual_note_has_exact_token(&n, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&n, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&n, "product_known_good=0"));
        assert!(residual_note_has_exact_token(&n, "residual_not_closed=1"));
        assert!(residual_note_has_exact_token(
            &n,
            "pipeline_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "ctrltext_match_alone_closes_fw_residual=0"
        ));
        assert!(!residual_note_has_exact_token(&n, "invent_pass=1"));
        assert!(!residual_note_has_exact_token(&n, "residual_not_closed=0"));
    }

    #[test]
    fn app_load_extract_bag_scorecard_pipeline_honesty_hard_coded_not_mutable() {
        let mut p = research_app_load_extract_bag_scorecard_pipeline_c2_locked_absent_bag();
        p.residual_not_closed = false;
        p.pipeline_alone_closes_fw_residual = true;
        p.extract_alone_closes_fw_residual = true;
        p.scorecard_alone_closes_fw_residual = true;
        p.feed_alone_closes_fw_residual = true;
        p.dry_run_alone_closes_fw_residual = true;
        p.bag_drop_alone_closes_fw_residual = true;
        p.lab_export_auto_feed = false;
        p.blocked_on_external_bytes = false;
        let n = p.format_soft_note();
        assert!(residual_note_has_exact_token(&n, "residual_not_closed=1"));
        assert!(residual_note_has_exact_token(
            &n,
            "pipeline_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "extract_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "scorecard_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "feed_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "dry_run_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "bag_drop_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "ctrltext_match_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(&n, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&n, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&n, "not_ran_on_npu=1"));
        assert!(residual_note_has_exact_token(&n, "product_known_good=0"));
        assert!(residual_note_has_exact_token(&n, "lab_export_auto_feed=1"));
        assert!(residual_note_has_exact_token(
            &n,
            "do_not_invent_pdi_layout=1"
        ));
        // Mutable blocked bit may echo false after mutation.
        assert!(residual_note_has_exact_token(
            &n,
            "blocked_on_external_bytes=0"
        ));
        assert!(!residual_note_has_exact_token(&n, "invent_pass=1"));
        assert!(!residual_note_has_exact_token(&n, "residual_not_closed=0"));
        assert!(!residual_note_has_exact_token(
            &n,
            "pipeline_alone_closes_fw_residual=1"
        ));
    }

    #[test]
    fn app_load_extract_bag_scorecard_pipeline_external_load_refused_and_from_inputs() {
        let extract = research_xclbin_extract_dry_run(&[], None, None);
        let bag = check_external_known_good_bag_drop(false, None, None, None);
        let feed = research_lab_export_fingerprint_feed(None, None, None, None);
        let p = research_app_load_extract_bag_scorecard_pipeline_ex(
            &extract, &bag, &feed, None, None, None, true, // external_load_refused
        );
        assert!(p.blocked_on_external_bytes);
        assert_eq!(p.detail, "pipeline_external_load_refused_blocked_on_bytes");
        assert!(p.residual_not_closed);
        assert!(residual_note_has_exact_token(
            &p.format_soft_note(),
            "blocked_on_external_bytes=1"
        ));
        assert!(residual_note_has_exact_token(
            &p.format_soft_note(),
            "invent_pass=0"
        ));

        // from_inputs convenience: markers → blocked without inventing Pass.
        let header = XCLBIN2_MAGIC.as_slice();
        let p2 = research_app_load_extract_bag_scorecard_pipeline_from_inputs(
            header,
            Some("AIE_PARTITION"),
            None,
            &bag,
            &feed,
            None,
            None,
            None,
        );
        assert!(p2.extract_magic_xclbin2);
        assert!(p2.extract_has_aie_partition_marker);
        assert!(p2.blocked_on_external_bytes);
        assert_eq!(
            p2.detail,
            "pipeline_extract_markers_present_blocked_on_external_bytes"
        );
        assert!(p2.residual_not_closed);
        assert!(!p2.pipeline_alone_closes_fw_residual);

        // Default bag honesty.
        let d = AppLoadExtractBagScorecardPipeline::default();
        assert!(d.blocked_on_external_bytes);
        assert!(d.residual_not_closed);
        assert!(!d.pipeline_alone_closes_fw_residual);
        assert_eq!(d.detail, "pipeline_default_blocked_on_external_bytes");
        let dn = d.format_soft_note();
        assert!(residual_note_has_exact_token(
            &dn,
            "blocked_on_external_bytes=1"
        ));
        assert!(residual_note_has_exact_token(&dn, "invent_pass=0"));
        assert!(residual_note_has_exact_token(
            &dn,
            "pipeline_alone_closes_fw_residual=0"
        ));
    }

    // -----------------------------------------------------------------------
    // Journal partition-id field map (0x3000003 surface; stamp 7a5ab864)
    // -----------------------------------------------------------------------

    #[test]
    fn app_load_partition_id_field_map_c2_locked_absent_id() {
        let r = research_app_load_partition_id_field_map_c2_locked();
        assert_eq!(r.failed_cmd_status, Some(AIE2_FW_APP_LOAD_PDI_FAIL));
        assert_eq!(r.chain_status, Some(AIE2_FW_INVALID_PARAM));
        assert!(r.failed_cmd_is_app_load);
        assert!(r.chain_is_invalid_param);
        assert!(r.c2_app_load_pair);
        assert_eq!(r.partition_id_public, APP_LOAD_PARTITION_ID_PUBLIC_TOKEN);
        assert_eq!(r.partition_id_public, "absent_public");
        assert!(r.do_not_invent_partition_id);
        assert!(!r.field_map_alone_closes_fw_residual);
        assert!(!r.root_cause_closed);
        assert_eq!(r.detail, "c2_pair_mapped_partition_id_absent");
        // Public field name constants stay stable for soft notes / docs.
        assert_eq!(APP_LOAD_JOURNAL_FIELD_FAILED_CMD, "failed_cmd_status");
        assert_eq!(APP_LOAD_JOURNAL_FIELD_CHAIN_STATUS, "chain_status");

        let n = r.format_soft_note();
        assert!(residual_note_has_exact_token(
            &n,
            "app_load_partition_id_field_map"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "partition_id_public=absent_public"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "do_not_invent_partition_id=1"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "field_map_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(&n, "root_cause_closed=0"));
        assert!(residual_note_has_exact_token(
            &n,
            "partition_surface_public=status_name_only"
        ));
        assert!(residual_note_has_exact_token(&n, "failed_cmd=0x3000003"));
        assert!(residual_note_has_exact_token(
            &n,
            "failed_cmd_name=APP_LOAD_PDI_FAIL"
        ));
        assert!(residual_note_has_exact_token(&n, "chain_status=0x4000003"));
        assert!(residual_note_has_exact_token(&n, "c2_app_load_pair=1"));
        assert!(residual_note_has_exact_token(
            &n,
            "journal_field_failed_cmd=failed_cmd_status"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "journal_field_chain_status=chain_status"
        ));
        assert!(residual_note_has_exact_token(&n, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&n, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&n, "not_ran_on_npu=1"));
        assert!(residual_note_has_exact_token(&n, "product_known_good=0"));
        // Mutated close/invent fields must not leak.
        let mut mutated = r.clone();
        mutated.field_map_alone_closes_fw_residual = true;
        mutated.root_cause_closed = true;
        mutated.do_not_invent_partition_id = false;
        mutated.partition_id_public = "invented_id_0";
        let mn = mutated.format_soft_note();
        assert!(residual_note_has_exact_token(
            &mn,
            "partition_id_public=absent_public"
        ));
        assert!(residual_note_has_exact_token(
            &mn,
            "do_not_invent_partition_id=1"
        ));
        assert!(residual_note_has_exact_token(
            &mn,
            "field_map_alone_closes_fw_residual=0"
        ));
        assert!(residual_note_has_exact_token(&mn, "root_cause_closed=0"));
        assert!(!residual_note_has_exact_token(&mn, "root_cause_closed=1"));
        assert!(!residual_note_has_exact_token(
            &mn,
            "field_map_alone_closes_fw_residual=1"
        ));
        assert!(!residual_note_has_exact_token(
            &mn,
            "partition_id_public=invented_id_0"
        ));
        assert!(!residual_note_has_exact_token(&mn, "invent_pass=1"));
    }

    #[test]
    fn app_load_partition_id_field_map_absent_and_partial() {
        let empty = research_app_load_partition_id_field_map(None);
        assert!(empty.failed_cmd_status.is_none());
        assert!(empty.chain_status.is_none());
        assert!(!empty.c2_app_load_pair);
        assert_eq!(empty.partition_id_public, "absent_public");
        assert_eq!(empty.detail, "default_no_journal");
        let en = empty.format_soft_note();
        assert!(residual_note_has_exact_token(&en, "failed_cmd=absent"));
        assert!(residual_note_has_exact_token(&en, "chain_status=absent"));
        assert!(residual_note_has_exact_token(
            &en,
            "partition_id_public=absent_public"
        ));
        assert!(residual_note_has_exact_token(&en, "root_cause_closed=0"));

        // Partial: failed_cmd only.
        let partial = research_app_load_partition_id_field_map_from_journal(Some(
            "amdxdna: Failed cmd idx 0, status 0x3000003\n",
        ));
        assert!(partial.failed_cmd_is_app_load);
        assert!(!partial.c2_app_load_pair);
        assert_eq!(partial.detail, "failed_cmd_app_load_partition_id_absent");
        assert_eq!(partial.partition_id_public, "absent_public");

        // Oversized journal → treat as absent (no invent).
        let big = "x".repeat(MAX_APP_LOAD_JOURNAL_TEXT_BYTES + 1);
        let over = research_app_load_partition_id_field_map_from_journal(Some(&big));
        assert!(over.failed_cmd_status.is_none());
        assert_eq!(over.detail, "default_no_journal");

        let def = AppLoadPartitionIdFieldMap::default();
        assert!(!def.root_cause_closed);
        assert!(!def.field_map_alone_closes_fw_residual);
        assert_eq!(def.partition_id_public, APP_LOAD_PARTITION_ID_PUBLIC_TOKEN);
    }

    #[test]
    fn construction_path_silicon_lab_elf_when_present() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("testdata/xdna2_cmd/silicon_lab/lab_instr.elf");
        if !path.is_file() {
            eprintln!("OFFLINE_ASSET_SKIP: no silicon_lab lab_instr.elf");
            return;
        }
        let elf = match std::fs::read(&path) {
            Ok(e) => e,
            Err(e) => {
                eprintln!("OFFLINE_ASSET_SKIP: read failed: {e}");
                return;
            }
        };
        let r = research_construction_path_lab_vs_public(&elf);
        assert!(
            r.lab_has_instr_ctrltext,
            "silicon_lab must peel classic+rela: detail={}",
            r.detail
        );
        assert!(!r.lab_has_cu_pdi_config, "never invent CONFIG_CU");
        assert!(r.public_stack_requires_cu_pdi_step);
        assert!(r.construction_gap_cu_pdi);
        assert_eq!(r.lab_ctrltext_len, Some(1688));
        assert_eq!(r.lab_rela_count, Some(12));
        assert_eq!(r.detail, "classic_ctrltext_ok");
        let n = r.format_soft_note();
        assert!(residual_note_has_exact_token(
            &n,
            "construction_path_research"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "construction_gap_cu_pdi=1"
        ));
        assert!(residual_note_has_exact_token(&n, "lab_has_instr=1"));
        assert!(residual_note_has_exact_token(&n, "lab_has_cu_pdi_config=0"));
        assert!(residual_note_has_exact_token(
            &n,
            "public_stack_cu_pdi_step=1"
        ));
        assert!(residual_note_has_exact_token(&n, "lab_ctrltext_len=1688"));
        assert!(residual_note_has_exact_token(&n, "lab_rela_count=12"));
        assert!(residual_note_has_exact_token(&n, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&n, "not_ran_on_npu=1"));
        assert!(residual_note_has_exact_token(&n, "invent_pass=0"));
        assert!(!n.contains("ran_on_npu=true"));
        assert!(!residual_note_has_exact_token(&n, "invent_pass=1"));
        assert!(!residual_note_has_exact_token(&n, "ran_on_npu=1"));
        // Construction research never opens C / invents Pass.
        assert!(!n.contains("c_unlock"));
    }

    #[test]
    fn fingerprint_empty_none() {
        assert!(fingerprint_patched_ctrltext_after_rela(&[], 1, 2, 3, 4).is_none());
        assert!(fingerprint_patched_ctrltext_after_rela_c2_locked(&[]).is_none());
        // Non-ELF still returns a bag (not None); soft note must not claim paddr sha.
        let g = fingerprint_patched_ctrltext_after_rela(b"not-elf", 1, 2, 3, 4)
            .expect("non-empty non-elf bag");
        assert!(g.patched_sha256_hex.is_empty());
        let gn = g.format_soft_note();
        assert!(residual_note_has_exact_token(&gn, "patched_sha=absent"));
        assert!(!residual_note_has_exact_token(
            &gn,
            "patched_sha_depends_on_paddrs=1"
        ));
        assert!(residual_note_has_exact_token(&gn, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&gn, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&gn, "not_ran_on_npu=1"));
    }

    #[test]
    fn fingerprint_synthetic_c2_locked() {
        // Synthetic aiebu + rela + walkable noop: C2-locked paddrs for userspace-ok shape.
        let ctrltext = build_strix_research_noop_txn().expect("noop");
        let relas = [(0_u32, (1_u32 << 8) | 5, 0_i32)];
        let synth = build_minimal_test_elf32(&ctrltext, &relas);
        let f1 =
            fingerprint_patched_ctrltext_after_rela_c2_locked(&synth).expect("synth fingerprint");
        assert!(
            f1.classic_walk_ok,
            "synthetic noop+rela must classic-walk: detail={}",
            f1.detail
        );
        assert_eq!(f1.rela_count, Some(1));
        assert!(!f1.patched_sha256_hex.is_empty());
        assert!(!f1.unpatched_sha256_hex.is_empty());
        let f2 = fingerprint_patched_ctrltext_after_rela_c2_locked(&synth)
            .expect("synth fingerprint re-run");
        assert_eq!(f1.patched_sha256_hex, f2.patched_sha256_hex);
        assert_eq!(f1.unpatched_sha256_hex, f2.unpatched_sha256_hex);
        let sn = f1.format_soft_note();
        assert!(residual_note_has_exact_token(
            &sn,
            "patched_ctrltext_fingerprint"
        ));
        assert!(residual_note_has_exact_token(&sn, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&sn, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&sn, "not_ran_on_npu=1"));
        assert!(residual_note_has_exact_token(
            &sn,
            "patched_sha_depends_on_paddrs=1"
        ));
        assert!(!sn.contains("ran_on_npu=true"));
        assert!(!residual_note_has_exact_token(&sn, "ran_on_npu=1"));
    }

    #[test]
    fn fingerprint_silicon_lab_structural_when_present() {
        // Optional full staged silicon_lab corpus (structural lock; unpatched sha stable).
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("testdata/xdna2_cmd/silicon_lab/lab_instr.elf");
        if !path.is_file() {
            eprintln!("OFFLINE_ASSET_SKIP: no silicon_lab for fingerprint");
            return;
        }
        let elf = match std::fs::read(&path) {
            Ok(e) => e,
            Err(e) => {
                eprintln!("OFFLINE_ASSET_SKIP: read failed: {e}");
                return;
            }
        };
        // Fixed synthetic valid paddrs (nonzero, not u64::MAX) — same style as txn tests.
        let ifm = 0x1111_0000_u64;
        let wts = 0x2222_0000_u64;
        let ofm = 0x3333_0000_u64;
        let ctrl = 0x4444_0000_u64;
        let fp = fingerprint_patched_ctrltext_after_rela(&elf, ifm, wts, ofm, ctrl)
            .expect("silicon_lab fingerprint");
        assert_eq!(fp.patched_len, Some(1688));
        assert!(fp.classic_walk_ok, "detail={}", fp.detail);
        assert_eq!(fp.rela_count, Some(12));
        assert!(fp.staged_mladf_classic_profile);
        assert_eq!(fp.txn_num_ops, Some(40));
        // gen4 / 8×6 classic header fields (staged mladf profile).
        assert_eq!(fp.txn_dev_gen, Some(4));
        assert_eq!(fp.txn_num_cols, Some(8));
        assert_eq!(fp.txn_num_rows, Some(6));
        assert_eq!(fp.patched_sha256_hex.len(), 64);
        assert_eq!(fp.unpatched_sha256_hex.len(), 64);
        // Stability under same paddrs.
        let fp2 =
            fingerprint_patched_ctrltext_after_rela(&elf, ifm, wts, ofm, ctrl).expect("re-run");
        assert_eq!(fp.patched_sha256_hex, fp2.patched_sha256_hex);
        assert_eq!(fp.unpatched_sha256_hex, fp2.unpatched_sha256_hex);
        // Unpatched sha is independent of paddrs (structural identity of corpus).
        let fp_other = fingerprint_patched_ctrltext_after_rela(
            &elf,
            0xaaaa_0000,
            0xbbbb_0000,
            0xcccc_0000,
            0xdddd_0000,
        )
        .expect("other paddrs");
        assert_eq!(
            fp.unpatched_sha256_hex, fp_other.unpatched_sha256_hex,
            "unpatched peel sha must not depend on IO paddrs"
        );
        assert_ne!(
            fp.patched_sha256_hex, fp_other.patched_sha256_hex,
            "patched sha must change with paddrs (documents dependence)"
        );
        let n = fp.format_soft_note();
        assert!(residual_note_has_exact_token(&n, "patched_len=1688"));
        assert!(residual_note_has_exact_token(&n, "rela_count=12"));
        assert!(residual_note_has_exact_token(&n, "txn_num_ops=40"));
        assert!(residual_note_has_exact_token(&n, "classic_walk_ok=1"));
        assert!(residual_note_has_exact_token(&n, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&n, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&n, "not_ran_on_npu=1"));
        assert!(!n.contains("ran_on_npu=true"));
        assert!(!residual_note_has_exact_token(&n, "ran_on_npu=1"));
    }

    // -----------------------------------------------------------------------
    // Fingerprint export + external ctrltext compare (known-good absent offline)
    // -----------------------------------------------------------------------

    #[test]
    fn fingerprint_export_empty_and_garbage_known_good_absent() {
        let e = export_ctrltext_fingerprint_research(&[]);
        assert!(e.is_known_good_absent());
        assert!(!e.known_good_present);
        assert_eq!(e.known_good_status, KNOWN_GOOD_CTRLTEXT_ABSENT_TOKEN);
        assert_eq!(e.detail, "elf_empty");
        assert!(e.unpatched_sha256_hex.is_empty());
        let n = e.format_soft_note();
        assert!(residual_note_has_exact_token(
            &n,
            CTRLTEXT_FINGERPRINT_EXPORT_NOTE
        ));
        assert!(residual_note_has_exact_token(&n, "known_good_absent=1"));
        assert!(residual_note_has_exact_token(&n, "known_good_present=0"));
        assert!(residual_note_has_exact_token(
            &n,
            "known_good_status=absent_awaiting_capture"
        ));
        assert!(residual_note_has_exact_token(&n, "unpatched_sha=absent"));
        assert!(residual_note_has_exact_token(&n, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&n, "not_ran_on_npu=1"));
        assert!(residual_note_has_exact_token(&n, "invent_pass=0"));
        assert!(!residual_note_has_exact_token(&n, "ran_on_npu=1"));
        assert!(!residual_note_has_exact_token(&n, "invent_pass=1"));

        let g = export_ctrltext_fingerprint_research(b"not-elf");
        assert!(g.is_known_good_absent());
        assert_eq!(g.detail, "not_aiebu_elf_ctrltext");
        assert!(g.rela_count.is_none());
        assert!(g.unpatched_sha256_hex.is_empty());
        let gn = g.format_soft_note();
        assert!(residual_note_has_exact_token(&gn, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&gn, "known_good_absent=1"));
        // Export text is soft-note with newlines (portable bag).
        let xt = e.format_export_text();
        assert!(xt.contains('\n'));
        assert!(xt.lines().any(|l| l == "not_c_ready=1"));
        assert!(xt.lines().any(|l| l == "invent_pass=0"));
        assert!(xt.lines().any(|l| l == "known_good_absent=1"));
    }

    #[test]
    fn fingerprint_export_synthetic_classic_and_compare_absent() {
        let ctrltext = build_strix_research_noop_txn().expect("noop");
        let relas = [(0_u32, (1_u32 << 8) | 5, 0_i32)];
        let synth = build_minimal_test_elf32(&ctrltext, &relas);
        let exp = export_ctrltext_fingerprint_research(&synth);
        assert!(exp.classic_walk_ok, "detail={}", exp.detail);
        assert_eq!(exp.rela_count, Some(1));
        assert_eq!(exp.ctrltext_len, Some(ctrltext.len()));
        assert_eq!(exp.unpatched_sha256_hex.len(), 64);
        assert!(exp.is_known_good_absent());
        assert_eq!(exp.detail, "classic_unpatched_ok");
        // Unpatched sha must match peel of ctrltext (not whole ELF).
        assert_eq!(exp.unpatched_sha256_hex, sha256_hex_lower(&ctrltext));
        let n = exp.format_soft_note();
        assert!(residual_note_has_exact_token(&n, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&n, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&n, "not_ran_on_npu=1"));
        assert!(residual_note_has_exact_token(&n, "known_good_absent=1"));
        assert!(!n.contains("ran_on_npu=true"));

        // Compare with no external → still awaiting capture.
        let c0 = compare_external_ctrltext_to_lab_export(&exp, None);
        assert!(c0.known_good_absent);
        assert!(!c0.external_present);
        assert!(!c0.sha_match);
        assert!(!c0.structural_match);
        assert_eq!(c0.detail, "external_absent_awaiting_capture");
        assert_eq!(c0.lab_unpatched_sha256_hex, exp.unpatched_sha256_hex);
        let cn = c0.format_soft_note();
        assert!(residual_note_has_exact_token(
            &cn,
            EXTERNAL_CTRLTEXT_COMPARE_NOTE
        ));
        assert!(residual_note_has_exact_token(&cn, "known_good_absent=1"));
        assert!(residual_note_has_exact_token(&cn, "known_good_present=0"));
        assert!(residual_note_has_exact_token(&cn, "product_known_good=0"));
        assert!(residual_note_has_exact_token(&cn, "external_present=0"));
        assert!(residual_note_has_exact_token(&cn, "sha_match=0"));
        assert!(residual_note_has_exact_token(&cn, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&cn, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&cn, "not_ran_on_npu=1"));
        assert!(!residual_note_has_exact_token(&cn, "ran_on_npu=1"));

        // Empty slice same as None.
        let c_empty = compare_external_ctrltext_to_lab_export(&exp, Some(&[]));
        assert!(c_empty.known_good_absent);
        assert!(!c_empty.external_present);
    }

    #[test]
    fn fingerprint_export_zero_rela_aiebu_known_good_absent() {
        // Zero-rela aiebu: peel + classic walk still OK; rela_count=Some(0);
        // known-good still absent (export never invents product capture).
        let ctrltext = build_strix_research_noop_txn().expect("noop");
        let synth = build_minimal_test_elf32(&ctrltext, &[]);
        let exp = export_ctrltext_fingerprint_research(&synth);
        assert!(exp.classic_walk_ok, "detail={}", exp.detail);
        assert_eq!(exp.rela_count, Some(0));
        assert_eq!(exp.ctrltext_len, Some(ctrltext.len()));
        assert_eq!(exp.unpatched_sha256_hex, sha256_hex_lower(&ctrltext));
        assert!(exp.is_known_good_absent());
        assert!(!exp.known_good_present);
        assert_eq!(exp.known_good_status, KNOWN_GOOD_CTRLTEXT_ABSENT_TOKEN);
        let n = exp.format_soft_note();
        assert!(residual_note_has_exact_token(&n, "rela_count=0"));
        assert!(residual_note_has_exact_token(&n, "known_good_absent=1"));
        assert!(residual_note_has_exact_token(&n, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&n, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&n, "not_ran_on_npu=1"));
        let c0 = compare_external_ctrltext_to_lab_export(&exp, None);
        assert!(c0.known_good_absent);
        assert!(!c0.external_present);
        assert!(residual_note_has_exact_token(
            &c0.format_soft_note(),
            "product_known_good=0"
        ));
    }

    #[test]
    fn fingerprint_export_compare_match_and_mismatch_honesty() {
        let ctrltext = build_strix_research_noop_txn().expect("noop");
        let relas = [(0_u32, (1_u32 << 8) | 5, 0_i32)];
        let synth = build_minimal_test_elf32(&ctrltext, &relas);
        let exp = export_ctrltext_fingerprint_research(&synth);

        // Exact unpatched peel match — research only; invent_pass still 0.
        let cm = compare_external_ctrltext_to_lab_export(&exp, Some(&ctrltext));
        assert!(cm.external_present);
        assert!(!cm.known_good_absent);
        assert!(cm.sha_match);
        assert!(cm.len_match);
        assert!(cm.structural_match);
        assert!(cm.external_classic_walk_ok);
        assert_eq!(cm.detail, "sha_and_structural_match_lab_unpatched");
        let mn = cm.format_soft_note();
        assert!(residual_note_has_exact_token(&mn, "sha_match=1"));
        assert!(residual_note_has_exact_token(&mn, "structural_match=1"));
        assert!(residual_note_has_exact_token(&mn, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&mn, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&mn, "not_ran_on_npu=1"));
        // Research external present ≠ product known-good.
        assert!(residual_note_has_exact_token(&mn, "known_good_present=0"));
        assert!(residual_note_has_exact_token(&mn, "product_known_good=0"));
        assert!(residual_note_has_exact_token(&mn, "known_good_absent=0"));
        // Match ≠ product Pass tokens.
        assert!(!residual_note_has_exact_token(&mn, "ran_on_npu=1"));
        assert!(!residual_note_has_exact_token(&mn, "invent_pass=1"));
        assert!(!residual_note_has_exact_token(&mn, "c_ready=1"));
        // Staged profile bits on soft note for silicon_lab debug parity.
        assert!(residual_note_has_exact_token(
            &mn,
            "lab_staged_mladf_classic_profile=0"
        ));
        assert!(residual_note_has_exact_token(
            &mn,
            "external_staged_mladf_classic_profile=0"
        ));

        // Different content same length class → sha_match=0.
        let mut flipped = ctrltext.clone();
        if let Some(b) = flipped.last_mut() {
            *b ^= 0x5a;
        }
        let cdiff = compare_external_ctrltext_to_lab_export(&exp, Some(&flipped));
        assert!(cdiff.external_present);
        assert!(!cdiff.sha_match);
        assert!(cdiff.len_match);
        assert_ne!(cdiff.external_sha256_hex, exp.unpatched_sha256_hex);
        let dn = cdiff.format_soft_note();
        assert!(residual_note_has_exact_token(&dn, "sha_match=0"));
        assert!(residual_note_has_exact_token(&dn, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&dn, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&dn, "product_known_good=0"));

        // Length mismatch.
        let short = &ctrltext[..ctrltext.len().saturating_sub(8).max(1)];
        let clen = compare_external_ctrltext_to_lab_export(&exp, Some(short));
        assert!(!clen.len_match);
        assert!(!clen.sha_match);
        assert!(!clen.structural_match);
        assert_eq!(clen.detail, "external_differs_from_lab");

        // Dual non-walkable same-length garbage must NOT claim structural_match.
        let garbage = vec![0xABu8; 64];
        let lab_garbage = CtrltextFingerprintExport {
            ctrltext_len: Some(64),
            classic_walk_ok: false,
            staged_mladf_classic_profile: false,
            unpatched_sha256_hex: sha256_hex_lower(&garbage),
            known_good_present: false,
            known_good_status: KNOWN_GOOD_CTRLTEXT_ABSENT_TOKEN,
            detail: "test_non_walkable".into(),
            ..CtrltextFingerprintExport::default()
        };
        let c_garb = compare_external_ctrltext_to_lab_export(&lab_garbage, Some(&garbage));
        assert!(c_garb.sha_match);
        assert!(c_garb.len_match);
        assert!(
            !c_garb.structural_match,
            "dual garbage must not structural_match"
        );
        assert_eq!(c_garb.detail, "sha_match_lab_unpatched");
        assert!(residual_note_has_exact_token(
            &c_garb.format_soft_note(),
            "structural_match=0"
        ));
        assert!(residual_note_has_exact_token(
            &c_garb.format_soft_note(),
            "product_known_good=0"
        ));
    }

    #[test]
    fn fingerprint_export_silicon_lab_structural_when_present() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("testdata/xdna2_cmd/silicon_lab/lab_instr.elf");
        if !path.is_file() {
            eprintln!("OFFLINE_ASSET_SKIP: no silicon_lab for fingerprint export");
            return;
        }
        let elf = match std::fs::read(&path) {
            Ok(e) => e,
            Err(e) => {
                eprintln!("OFFLINE_ASSET_SKIP: read failed: {e}");
                return;
            }
        };
        let exp = export_ctrltext_fingerprint_research(&elf);
        assert!(exp.is_known_good_absent(), "never invent known-good");
        assert_eq!(exp.ctrltext_len, Some(1688));
        assert_eq!(exp.rela_count, Some(12));
        assert!(exp.classic_walk_ok, "detail={}", exp.detail);
        assert!(exp.staged_mladf_classic_profile);
        assert_eq!(exp.txn_num_ops, Some(40));
        assert_eq!(exp.txn_dev_gen, Some(4));
        assert_eq!(exp.txn_num_cols, Some(8));
        assert_eq!(exp.txn_num_rows, Some(6));
        assert_eq!(exp.unpatched_sha256_hex.len(), 64);
        assert_eq!(exp.detail, "silicon_lab_structural_ok");
        // Export identity must match PatchedCtrltextFingerprint unpatched sha.
        let ifm = 0x1111_0000_u64;
        let wts = 0x2222_0000_u64;
        let ofm = 0x3333_0000_u64;
        let ctrl = 0x4444_0000_u64;
        let fp = fingerprint_patched_ctrltext_after_rela(&elf, ifm, wts, ofm, ctrl)
            .expect("fingerprint");
        assert_eq!(exp.unpatched_sha256_hex, fp.unpatched_sha256_hex);
        let n = exp.format_soft_note();
        assert!(residual_note_has_exact_token(&n, "ctrltext_len=1688"));
        assert!(residual_note_has_exact_token(&n, "rela_count=12"));
        assert!(residual_note_has_exact_token(&n, "txn_num_ops=40"));
        assert!(residual_note_has_exact_token(&n, "known_good_absent=1"));
        assert!(residual_note_has_exact_token(
            &n,
            "known_good_status=absent_awaiting_capture"
        ));
        assert!(residual_note_has_exact_token(&n, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&n, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&n, "not_ran_on_npu=1"));

        // Local pure-direct post-patch oracle is NOT external known-good: when
        // supplied as external, compare may sha-diff (patched vs unpatched).
        // Absent external remains the honest default offline.
        let c_abs = compare_external_ctrltext_to_lab_export(&exp, None);
        assert!(c_abs.known_good_absent);
        assert!(residual_note_has_exact_token(
            &c_abs.format_soft_note(),
            "known_good_absent=1"
        ));

        // Peel unpatched matches export (research self-check).
        let peeled = super::super::txn::instruction_payload_bytes(&elf).expect("peel");
        let c_peel = compare_external_ctrltext_to_lab_export(&exp, Some(&peeled));
        assert!(c_peel.sha_match);
        assert!(c_peel.structural_match);
        assert!(residual_note_has_exact_token(
            &c_peel.format_soft_note(),
            "invent_pass=0"
        ));
        // Self-match of failing corpus ≠ Pass.
        assert!(!residual_note_has_exact_token(
            &c_peel.format_soft_note(),
            "ran_on_npu=1"
        ));
    }

    // -----------------------------------------------------------------------
    // External known-good path confinement + load (feed compare when present)
    // -----------------------------------------------------------------------

    #[test]
    fn external_known_good_path_refuses_etc_passwd_and_traversal() {
        assert!(!external_known_good_path_is_allowed(Path::new(
            "/etc/passwd"
        )));
        assert!(!external_known_good_load_path_is_allowed(Path::new(
            "/etc/passwd"
        )));
        assert!(!external_known_good_path_is_allowed(Path::new(
            "../../etc/passwd"
        )));
        assert!(!external_known_good_path_is_allowed(Path::new(
            "/tmp/not_ctrltext.json"
        )));
        assert!(!external_known_good_path_is_allowed(Path::new(
            "/tmp/evil.elf"
        )));
        // String prefix footgun: /tmp_evil is not under /tmp as components.
        let evil = Path::new("/tmp_evil/known_good.bin");
        if last_apply_temp_root() == Path::new("/tmp") || last_apply_temp_root().ends_with("tmp") {
            assert!(
                !external_known_good_path_is_allowed(evil),
                "sibling prefix of temp must be refused"
            );
        }
    }

    #[test]
    fn external_known_good_refuses_local_post_patch_oracles() {
        assert!(is_refused_local_post_patch_as_known_good(Path::new(
            "instr_post_patch.bin"
        )));
        assert!(is_refused_local_post_patch_as_known_good(Path::new(
            "instr_vendor_post_patch.bin"
        )));
        assert!(is_refused_local_post_patch_as_known_good(Path::new(
            "CTRL_POST_PATCH.BIN"
        )));
        assert!(!is_refused_local_post_patch_as_known_good(Path::new(
            "working_stack_ctrltext.bin"
        )));

        // Even under temp root, refuse oracle basenames.
        let tmp_oracle = last_apply_temp_root().join("instr_post_patch.bin");
        assert!(
            !external_known_good_path_is_allowed(&tmp_oracle),
            "local pure-direct post_patch must never be known-good"
        );
        assert!(!external_known_good_load_path_is_allowed(&tmp_oracle));

        // gt_capture absolute under lab (outside external_known_good/) refused.
        let gt = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("testdata/xdna2_cmd/gt_capture/instr_post_patch.bin");
        assert!(
            !external_known_good_path_is_allowed(&gt),
            "gt_capture post_patch outside dedicated bag root"
        );
        let vendor = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("testdata/xdna2_cmd/gt_capture/instr_vendor_post_patch.bin");
        assert!(!external_known_good_path_is_allowed(&vendor));
    }

    #[test]
    fn external_known_good_allows_temp_and_load_compare_honesty() {
        let dir =
            last_apply_temp_root().join(format!("dragon_npu_ext_kg_test_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("research_ctrltext.bin");
        let _ = std::fs::remove_file(&path);

        // Synthetic classic ctrltext for round-trip load + compare.
        let ctrltext = build_strix_research_noop_txn().expect("noop");
        std::fs::write(&path, &ctrltext).expect("write temp ctrltext");

        assert!(
            external_known_good_path_is_allowed(&path),
            "temp under clamped root must be allowed"
        );
        assert!(
            external_known_good_load_path_is_allowed(&path),
            "temp load does not require PROVENANCE"
        );

        let loaded = load_external_ctrltext_for_compare(&path).expect("load temp");
        assert_eq!(loaded, ctrltext);

        let relas = [(0_u32, (1_u32 << 8) | 5, 0_i32)];
        let synth = build_minimal_test_elf32(&ctrltext, &relas);
        let exp = export_ctrltext_fingerprint_research(&synth);
        let cmp = compare_external_ctrltext_to_lab_export(&exp, Some(&loaded));
        assert!(cmp.external_present);
        assert!(cmp.sha_match);
        // Research load/match still never product known-good / Pass.
        let n = cmp.format_soft_note();
        assert!(residual_note_has_exact_token(&n, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&n, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&n, "not_ran_on_npu=1"));
        assert!(residual_note_has_exact_token(&n, "product_known_good=0"));
        assert!(residual_note_has_exact_token(&n, "known_good_present=0"));
        assert!(!residual_note_has_exact_token(&n, "ran_on_npu=1"));

        // Extension refuse under same temp dir.
        let bad_ext = dir.join("research_ctrltext.json");
        std::fs::write(&bad_ext, b"{}").expect("write");
        assert!(!external_known_good_path_is_allowed(&bad_ext));
        assert!(load_external_ctrltext_for_compare(&bad_ext).is_err());

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(&bad_ext);
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn external_known_good_load_refuses_symlink_when_present() {
        let dir =
            last_apply_temp_root().join(format!("dragon_npu_ext_kg_link_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let target = dir.join("target_ctrltext.bin");
        let link = dir.join("link_ctrltext.bin");
        let _ = std::fs::remove_file(&target);
        let _ = std::fs::remove_file(&link);
        std::fs::write(&target, b"\0\0\0\0research").expect("write target");
        if std::os::unix::fs::symlink(&target, &link).is_ok() {
            assert!(
                external_known_good_path_is_allowed(&link),
                "path roots may still allow lexical name"
            );
            assert!(
                !external_known_good_load_path_is_allowed(&link),
                "load must refuse final-component symlink"
            );
            assert!(load_external_ctrltext_for_compare(&link).is_err());
            let _ = std::fs::remove_file(&link);
        }
        // Mid-path symlink escape: dir/evil_mid → /etc, then /etc/passwd style.
        // Create mid-dir symlink pointing outside temp if OS allows.
        let mid_link = dir.join("evil_mid");
        let outside = Path::new("/etc");
        let _ = std::fs::remove_file(&mid_link);
        let _ = std::fs::remove_dir(&mid_link);
        if std::os::unix::fs::symlink(outside, &mid_link).is_ok() {
            // Lexical path under temp, but parent canonicalize → /etc.
            let attack = mid_link.join("passwd");
            // passwd has no .bin extension → already refuse; also use a .bin name.
            let attack_bin = mid_link.join("shadow.bin");
            assert!(
                !external_known_good_path_is_allowed(&attack_bin)
                    || !external_known_good_load_path_is_allowed(&attack_bin),
                "mid-path symlink must not escape confinement"
            );
            // Stronger: path_is_allowed must fail after parent canonicalize.
            assert!(
                !external_known_good_path_is_allowed(&attack_bin),
                "canonical parent outside roots must refuse"
            );
            let _ = attack; // silence unused if not asserted
            let _ = std::fs::remove_file(&mid_link);
        }
        assert!(external_known_good_load_path_is_allowed(&target));
        let _ = std::fs::remove_file(&target);
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn external_known_good_lab_bag_requires_provenance_when_present() {
        let bag = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("testdata/xdna2_cmd")
            .join(EXTERNAL_KNOWN_GOOD_SUBDIR);
        // Tracked PROVENANCE.md is required by this stamp — hard-assert (not soft-skip).
        let prov = bag.join("PROVENANCE.md");
        assert!(
            prov.is_file(),
            "tracked external_known_good/PROVENANCE.md must exist"
        );
        let prov_text = std::fs::read_to_string(&prov).expect("read PROVENANCE");
        assert!(
            !prov_text.trim().is_empty(),
            "tracked PROVENANCE.md must be non-empty"
        );
        assert!(
            external_known_good_provenance_present(&bag.join("any.bin")),
            "template PROVENANCE must be non-empty"
        );
        // Template must be research-ready (fields + refuse invent) — load path gate.
        assert!(
            external_known_good_provenance_research_ready(&bag.join("any.bin")),
            "tracked PROVENANCE template must pass field validation"
        );
        let chk = validate_external_known_good_provenance_text(&prov_text);
        assert!(chk.research_present_eligible, "detail={}", chk.detail);
        assert!(chk.has_research_only_marker);
        assert!(chk.has_source_stack_marker);
        assert!(chk.refuses_product_pass_claim);
        // Path under bag root with good extension is allowed (lexical).
        let candidate = bag.join("working_stack_ctrltext.bin");
        assert!(
            external_known_good_path_is_allowed(&candidate),
            "lab bag root + .bin allowed"
        );
        // Load of missing file still errors (not invent empty known-good).
        if !candidate.is_file() {
            assert!(load_external_ctrltext_for_compare(&candidate).is_err());
        }
        // Refuse oracle basename even if planted under bag.
        let planted = bag.join("instr_post_patch.bin");
        assert!(!external_known_good_path_is_allowed(&planted));
    }

    #[test]
    fn external_known_good_provenance_field_validation_fail_closed() {
        // Empty / whitespace.
        let empty = validate_external_known_good_provenance_text("");
        assert!(!empty.non_empty);
        assert!(!empty.research_present_eligible);
        assert_eq!(empty.detail, "empty");
        let ws = validate_external_known_good_provenance_text("  \n\t  ");
        assert!(!ws.research_present_eligible);

        // Research marker only — missing source/stack.
        let no_src = validate_external_known_good_provenance_text(
            "Research only. Not product Pass. Not C-ready.",
        );
        assert!(no_src.has_research_only_marker);
        assert!(!no_src.has_source_stack_marker);
        assert!(!no_src.research_present_eligible);
        assert_eq!(no_src.detail, "missing_source_stack_marker");

        // Source only — missing research-only honesty.
        let no_res = validate_external_known_good_provenance_text(
            "Source stack: XRT shim public stack capture note only.",
        );
        assert!(no_res.has_source_stack_marker);
        assert!(!no_res.has_research_only_marker);
        assert!(!no_res.research_present_eligible);
        assert_eq!(no_res.detail, "missing_research_only_marker");

        // Product invent tokens refused even with markers.
        let invent = validate_external_known_good_provenance_text(
            "Research only. Source stack: XRT. invent_pass=1 not_c_ready=1",
        );
        assert!(!invent.refuses_product_pass_claim);
        assert!(!invent.research_present_eligible);
        assert_eq!(invent.detail, "product_pass_claim_refused");
        let ron = validate_external_known_good_provenance_text(
            "Research only. Public stack. ran_on_npu=true",
        );
        assert!(!ron.research_present_eligible);
        // Issue 1: mixed-case invent claims must refuse (ascii-lower exact tokens).
        let invent_case = validate_external_known_good_provenance_text(
            "Research only. Source stack: XRT. INVENT_PASS=1 not_c_ready=1",
        );
        assert!(!invent_case.research_present_eligible);
        assert_eq!(invent_case.detail, "product_pass_claim_refused");
        let ron_case = validate_external_known_good_provenance_text(
            "Research only. Public stack. ran_on_npu=TRUE",
        );
        assert!(!ron_case.research_present_eligible);
        assert_eq!(ron_case.detail, "product_pass_claim_refused");
        // Issue 9: standalone c_ready=1 refuse.
        let c_ready = validate_external_known_good_provenance_text(
            "Research only. Source stack: DynamicDispatch. c_ready=1",
        );
        assert!(!c_ready.research_present_eligible);
        assert_eq!(c_ready.detail, "product_pass_claim_refused");
        let c_ready_case = validate_external_known_good_provenance_text(
            "Research only. Source stack: XRT. C_READY=1",
        );
        assert!(!c_ready_case.research_present_eligible);
        // Issue 7: product_known_good=1 / invent_pass=true refuse.
        let pkg = validate_external_known_good_provenance_text(
            "Research only. Public stack. product_known_good=1",
        );
        assert!(!pkg.research_present_eligible);
        assert_eq!(pkg.detail, "product_pass_claim_refused");
        let inv_true = validate_external_known_good_provenance_text(
            "Research only. Source stack: XRT. invent_pass=true",
        );
        assert!(!inv_true.research_present_eligible);
        // Issue 2: not_c_ready=0 must not count as research marker; refuse invent-style.
        let ncr0 = validate_external_known_good_provenance_text(
            "not_c_ready=0 invent_pass=0 Source stack: XRT",
        );
        assert!(
            !ncr0.has_research_only_marker,
            "substring not_c_ready must not match =0 as research marker"
        );
        assert!(!ncr0.research_present_eligible);
        assert_eq!(ncr0.detail, "product_pass_claim_refused");
        // Exact not_c_ready=1 + source is eligible (without invent).
        let ncr1 = validate_external_known_good_provenance_text(
            "not_c_ready=1 invent_pass=0 Source stack: XRT",
        );
        assert!(ncr1.has_research_only_marker);
        assert!(
            ncr1.research_present_eligible,
            "exact not_c_ready=1 + source must pass: detail={}",
            ncr1.detail
        );
        // Issue 8: bare "xrt" substring inside other words is not a source marker.
        let no_xrt_word = validate_external_known_good_provenance_text(
            "Research only. Not product Pass. capture texture extract notes.",
        );
        assert!(!no_xrt_word.has_source_stack_marker);
        assert!(!no_xrt_word.research_present_eligible);
        // Issue 6: pure lab-bag load gate locks refuse when text not research-eligible.
        assert!(!external_known_good_lab_bag_provenance_allows_load(None));
        assert!(!external_known_good_lab_bag_provenance_allows_load(Some(
            ""
        )));
        assert!(!external_known_good_lab_bag_provenance_allows_load(Some(
            "not_c_ready=0 invent_pass=0 Source stack: XRT"
        )));
        assert!(!external_known_good_lab_bag_provenance_allows_load(Some(
            "Research only. Source stack: XRT. invent_pass=1"
        )));
        assert!(external_known_good_lab_bag_provenance_allows_load(Some(
            "Research only. Not product Pass. Source stack: XRT.\n\
             invent_pass=0 not_c_ready=1 not_ran_on_npu=1 product_known_good=0"
        )));
        // not_ran_on_npu=1 must not false-positive refuse.
        let ok_not = validate_external_known_good_provenance_text(
            "Research only. Not product Pass. Source stack: XRT / DynamicDispatch.\n\
             not_ran_on_npu=1 invent_pass=0 not_c_ready=1",
        );
        assert!(
            ok_not.research_present_eligible,
            "honesty tokens must pass: detail={}",
            ok_not.detail
        );
        let n = ok_not.format_soft_note();
        assert!(residual_note_has_exact_token(
            &n,
            "research_present_eligible=1"
        ));
        assert!(residual_note_has_exact_token(&n, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&n, "product_known_good=0"));
        assert!(!residual_note_has_exact_token(&n, "invent_pass=1"));
    }

    #[test]
    #[allow(unsafe_code)] // test-only env isolation for external ctrltext
    fn external_known_good_env_absent_is_none() {
        // Ensure unset path: remove for this test process if present.
        let prev = std::env::var(ENV_LAB_EXTERNAL_CTRLTEXT).ok();
        // SAFETY: unit-test isolation only; restored below.
        unsafe {
            std::env::remove_var(ENV_LAB_EXTERNAL_CTRLTEXT);
        }
        let r = load_external_ctrltext_from_env().expect("unset ok");
        assert!(r.is_none(), "absent env → None (honest known-good absent)");
        // Empty / whitespace-only also honest None.
        unsafe {
            std::env::set_var(ENV_LAB_EXTERNAL_CTRLTEXT, "   \t  ");
        }
        let r_ws = load_external_ctrltext_from_env().expect("whitespace ok");
        assert!(r_ws.is_none(), "whitespace env → None");
        // SAFETY: restore prior env for other tests.
        unsafe {
            match prev {
                Some(v) => std::env::set_var(ENV_LAB_EXTERNAL_CTRLTEXT, v),
                None => std::env::remove_var(ENV_LAB_EXTERNAL_CTRLTEXT),
            }
        }
    }

    #[test]
    #[allow(unsafe_code)] // test-only env isolation for external ctrltext
    fn external_known_good_env_set_confined_and_refused() {
        let prev = std::env::var(ENV_LAB_EXTERNAL_CTRLTEXT).ok();
        let dir =
            last_apply_temp_root().join(format!("dragon_npu_ext_kg_env_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("env_ctrltext.bin");
        let _ = std::fs::remove_file(&path);
        let payload = b"\0\0\0\0ext_env_research_bytes";
        std::fs::write(&path, payload).expect("write temp");

        // (1) Confined temp path → Ok(Some(bytes)).
        unsafe {
            std::env::set_var(ENV_LAB_EXTERNAL_CTRLTEXT, path.as_os_str());
        }
        let loaded = load_external_ctrltext_from_env()
            .expect("confined env load")
            .expect("Some(bytes)");
        assert_eq!(loaded, payload);

        // (2) Outside roots → Err.
        unsafe {
            std::env::set_var(ENV_LAB_EXTERNAL_CTRLTEXT, "/etc/passwd");
        }
        assert!(
            load_external_ctrltext_from_env().is_err(),
            "/etc/passwd must refuse"
        );

        // (3) Refused oracle basename under temp → Err.
        let oracle = dir.join("instr_post_patch.bin");
        std::fs::write(&oracle, payload).expect("write oracle name");
        unsafe {
            std::env::set_var(ENV_LAB_EXTERNAL_CTRLTEXT, oracle.as_os_str());
        }
        assert!(
            load_external_ctrltext_from_env().is_err(),
            "post_patch basename must refuse even under temp"
        );

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(&oracle);
        let _ = std::fs::remove_dir(&dir);
        unsafe {
            match prev {
                Some(v) => std::env::set_var(ENV_LAB_EXTERNAL_CTRLTEXT, v),
                None => std::env::remove_var(ENV_LAB_EXTERNAL_CTRLTEXT),
            }
        }
    }

    #[test]
    fn instr_pdi_research_token_divergence_walk_vs_userspace_ok() {
        // Production path: empty-rela ELF refuses patch → walk=0 and userspace_ok=0
        // (cannot produce walk=1 without rela via public API). Lock soft-note
        // predicates via direct field construction instead.
        let walk_ok_no_rela = InstrPdiLoadResearch {
            rela_audit: C2RelaPaddrAudit {
                elf_present: true,
                rela_count: Some(0),
                patched_ctrltext_walk_ok: true,
                patched_ctrltext_len: Some(416),
                patched_ctrltext: None,
                detail: "walk_ok_forced_no_rela".into(),
            },
            payload: Some(InstrPayloadProfile {
                payload_len: 416,
                classic_walk_ok: true,
                blob_kind: "classic_txn_v01".into(),
                ..InstrPayloadProfile::default()
            }),
            // Stricter bit: walk alone is not enough without rela>0.
            userspace_ok_fw_load_open: false,
            detail: "walk_ok_but_no_rela_or_profile".into(),
        };
        let n = walk_ok_no_rela.format_soft_note();
        assert!(residual_note_has_exact_token(&n, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&n, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&n, "not_ran_on_npu=1"));
        assert!(residual_note_has_exact_token(
            &n,
            "userspace_ok_fw_load_open=0"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "userspace_patch_ok_fw_load_still_open=1"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "patched_ctrltext_walk_ok=1"
        ));
        assert!(residual_note_has_exact_token(&n, "rela_count=0"));

        // walk=1 + rela>0 but classic_walk_ok=false → userspace_ok still 0.
        let walk_ok_no_classic = InstrPdiLoadResearch {
            rela_audit: C2RelaPaddrAudit {
                elf_present: true,
                rela_count: Some(1),
                patched_ctrltext_walk_ok: true,
                patched_ctrltext_len: Some(64),
                patched_ctrltext: None,
                detail: "walk_ok_profile_not_classic".into(),
            },
            payload: Some(InstrPayloadProfile {
                payload_len: 64,
                classic_walk_ok: false,
                blob_kind: "unknown_raw".into(),
                ..InstrPayloadProfile::default()
            }),
            userspace_ok_fw_load_open: false,
            detail: "walk_ok_but_no_rela_or_profile".into(),
        };
        let n2 = walk_ok_no_classic.format_soft_note();
        assert!(residual_note_has_exact_token(
            &n2,
            "userspace_patch_ok_fw_load_still_open=1"
        ));
        assert!(residual_note_has_exact_token(
            &n2,
            "userspace_ok_fw_load_open=0"
        ));
        assert!(residual_note_has_exact_token(
            &n2,
            "userspace_elf_rela_ok=1"
        ));
        assert!(residual_note_has_exact_token(&n2, "invent_pass=0"));

        // Empty-rela production path: never invent userspace_ok=1.
        let ctrltext = build_strix_research_noop_txn().expect("noop");
        let elf_no_rela = build_minimal_test_elf32(&ctrltext, &[]);
        let r = profile_instr_payload_after_rela_c2_locked(&elf_no_rela);
        assert!(!r.userspace_ok_fw_load_open);
        let rn = r.format_soft_note();
        assert!(residual_note_has_exact_token(
            &rn,
            "userspace_ok_fw_load_open=0"
        ));
        assert!(residual_note_has_exact_token(&rn, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&rn, "not_c_ready=1"));
    }

    #[test]
    fn cross_check_c2_cmdlist_fill_public_locked_and_negatives() {
        let (inst, ifm, wts, ofm, _ctrl) = c2_live_io_paddrs_locked();
        let pure = rebuild_c2_live_start_npu_cmd_bo(inst, 1688, ifm, wts, ofm, 0x1).expect("pure");
        let x = cross_check_c2_cmdlist_fill_locked(Some(&pure));
        assert_eq!(x.live_cmdbuf_bytes, 112);
        assert_eq!(x.cmdlist_msg_size_field, Some(0x70));
        assert!(x.msg_size_matches_live_cmdbuf);
        assert!(x.pure_is_21w_start_npu);
        assert_eq!(x.pure_start_npu_bytes, Some(84));
        assert!(x.live_instr_size_content_1688);
        assert!(x.logical_paddrs_match_expect);
        assert!(!x.envelope_exact_match);
        let n = x.format_soft_note();
        assert!(residual_note_has_exact_token(&n, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&n, "not_ran_on_npu=1"));
        assert!(residual_note_has_exact_token(&n, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&n, "fill_slot_body_invent=0"));
        assert!(residual_note_has_exact_token(
            &n,
            "do_not_invent_prop_count_15=1"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "fill_slot_public_expect=msg_size_eq_cmdbuf_dump"
        ));

        // Bare (no pure) still maps msg size + honesty.
        let bare = cross_check_c2_cmdlist_fill_locked(None);
        assert!(bare.msg_size_matches_live_cmdbuf);
        assert!(!bare.pure_is_21w_start_npu);
        let bn = bare.format_soft_note();
        assert!(residual_note_has_exact_token(&bn, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&bn, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&bn, "not_ran_on_npu=1"));
        assert!(residual_note_has_exact_token(
            &bn,
            "do_not_invent_prop_count_15=1"
        ));

        // Negatives: empty live, short msg, wrong pure len (84 zeros ≠ START_NPU).
        let empty_live = cross_check_c2_cmdlist_fill_public(&[], &[], Some(&pure));
        assert!(!empty_live.msg_size_matches_live_cmdbuf);
        assert_eq!(empty_live.live_cmdbuf_bytes, 0);
        let en = empty_live.format_soft_note();
        assert!(residual_note_has_exact_token(&en, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&en, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(
            &en,
            "do_not_invent_prop_count_15=1"
        ));

        let short_msg = cross_check_c2_cmdlist_fill_public(
            c2_live_cmdbuf_words_embedded(),
            &[0, 0, 0], // no size at idx 4
            Some(&pure),
        );
        assert!(!short_msg.msg_size_matches_live_cmdbuf);
        assert_eq!(short_msg.cmdlist_msg_size_field, None);
        let sn = short_msg.format_soft_note();
        assert!(residual_note_has_exact_token(
            &sn,
            "cmdlist_msg_size_field=absent"
        ));
        assert!(residual_note_has_exact_token(&sn, "invent_pass=0"));

        let fake84 = vec![0_u8; 84];
        let bad_pure = cross_check_c2_cmdlist_fill_locked(Some(&fake84));
        assert!(!bad_pure.pure_is_21w_start_npu); // decode fails / prop_count not 0
        assert_eq!(bad_pure.pure_start_npu_bytes, Some(84));
        let bp = bad_pure.format_soft_note();
        assert!(residual_note_has_exact_token(
            &bp,
            "pure_is_21w_start_npu=0"
        ));
        assert!(residual_note_has_exact_token(&bp, "invent_pass=0"));
        assert!(residual_note_has_exact_token(
            &bp,
            "do_not_invent_prop_count_15=1"
        ));
    }

    #[test]
    fn app_load_checklist_userspace_elf_rela_app_load_token_split() {
        let sig_ok = LastApplyDetailSignals {
            aiebu_elf: Some(true),
            rela_count: Some(12),
            aiebu_rela_patched: true,
            ert_error: true,
            instr_bo_type: Some("Dev".into()),
            ofm_raw_nonzero_bytes: Some(0),
            ..LastApplyDetailSignals::default()
        };
        let class = SiliconResidualClass::AppLoadPdiFailAfterAiebu;
        let ck = format_app_load_offline_checklist(class, &sig_ok);
        // Checklist uses distinct token (not research userspace_ok_fw_load_open).
        assert!(residual_note_has_exact_token(
            &ck,
            "checklist_userspace_elf_rela_app_load=1"
        ));
        assert!(!residual_note_has_exact_token(
            &ck,
            "userspace_ok_fw_load_open=1"
        ));
        assert!(residual_note_has_exact_token(
            &ck,
            "userspace_elf_rela_ok=1"
        ));
        assert!(residual_note_has_exact_token(&ck, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&ck, "not_ran_on_npu=1"));
        assert!(residual_note_has_exact_token(&ck, "invent_pass=0"));
        // Track L checklist tokens on APP_LOAD class (public surface only).
        assert!(residual_note_has_exact_token(
            &ck,
            "checklist_partition_surface_public=status_name_only"
        ));
        assert!(residual_note_has_exact_token(
            &ck,
            "checklist_pdi_marker_public=marker_absent_public"
        ));
        assert!(residual_note_has_exact_token(
            &ck,
            "checklist_public_pdi_image_decode=0"
        ));
        // Research token names must not leak into checklist without prefix.
        assert!(!residual_note_has_exact_token(
            &ck,
            "partition_surface_public=status_name_only"
        ));
        assert!(!residual_note_has_exact_token(
            &ck,
            "pdi_marker_public=marker_absent_public"
        ));

        // (1) APP_LOAD class + aiebu + no rela → no checklist userspace token;
        //     Track L surface tokens still emit (class-gated, not rela-gated).
        let sig_no_rela = LastApplyDetailSignals {
            aiebu_elf: Some(true),
            rela_count: None,
            aiebu_rela_patched: false,
            ert_error: true,
            ..LastApplyDetailSignals::default()
        };
        // Class label is forced here for emission gate isolation (classify may
        // map aiebu-no-rela to ErtErrorUnknownBlob — still exercise APP_LOAD arm).
        let ck1 = format_app_load_offline_checklist(
            SiliconResidualClass::AppLoadPdiFailAfterAiebu,
            &sig_no_rela,
        );
        assert!(!residual_note_has_exact_token(
            &ck1,
            "checklist_userspace_elf_rela_app_load=1"
        ));
        assert!(!residual_note_has_exact_token(
            &ck1,
            "userspace_ok_fw_load_open=1"
        ));
        assert!(residual_note_has_exact_token(
            &ck1,
            "checklist_partition_surface_public=status_name_only"
        ));
        assert!(residual_note_has_exact_token(
            &ck1,
            "checklist_pdi_marker_public=marker_absent_public"
        ));
        assert!(residual_note_has_exact_token(
            &ck1,
            "checklist_public_pdi_image_decode=0"
        ));
        assert!(residual_note_has_exact_token(&ck1, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&ck1, "not_ran_on_npu=1"));
        assert!(residual_note_has_exact_token(&ck1, "invent_pass=0"));

        // (2) non-APP_LOAD + aiebu + rela×12 → no checklist userspace token,
        //     no Track L class-gated surface tokens.
        let ck2 =
            format_app_load_offline_checklist(SiliconResidualClass::ErtErrorUnknownBlob, &sig_ok);
        assert!(!residual_note_has_exact_token(
            &ck2,
            "checklist_userspace_elf_rela_app_load=1"
        ));
        assert!(!residual_note_has_exact_token(
            &ck2,
            "userspace_ok_fw_load_open=1"
        ));
        assert!(!residual_note_has_exact_token(
            &ck2,
            "checklist_partition_surface_public=status_name_only"
        ));
        assert!(!residual_note_has_exact_token(
            &ck2,
            "checklist_pdi_marker_public=marker_absent_public"
        ));
        assert!(!residual_note_has_exact_token(
            &ck2,
            "checklist_public_pdi_image_decode=0"
        ));
        assert!(residual_note_has_exact_token(&ck2, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&ck2, "not_ran_on_npu=1"));
        assert!(residual_note_has_exact_token(&ck2, "invent_pass=0"));

        // Locked APP_LOAD path: never invent decode=1 or prop_count=15.
        assert!(!residual_note_has_exact_token(
            &ck,
            "checklist_public_pdi_image_decode=1"
        ));
        assert!(!residual_note_has_exact_token(
            &ck,
            "public_pdi_image_decode=1"
        ));
        assert!(!residual_note_has_exact_token(&ck, "prop_count=15"));
        assert!(!residual_note_has_exact_token(
            &ck,
            "do_not_invent_prop_count_15=0"
        ));
    }

    #[test]
    fn app_load_pdi_partition_research_c2_locked_and_honesty() {
        // CI-mandatory: locked C2 pair + APP_LOAD class + userspace content green.
        let r = research_app_load_pdi_partition_c2_locked(true);
        assert_eq!(r.failed_cmd_status, Some(AIE2_FW_APP_LOAD_PDI_FAIL));
        assert_eq!(r.chain_status, Some(AIE2_FW_INVALID_PARAM));
        assert!(r.c2_app_load_pair);
        assert!(r.class_app_load_after_aiebu);
        assert!(r.userspace_content_ok);
        assert!(r.fw_residual_open_after_userspace);
        assert_eq!(r.partition_surface_public, PUBLIC_PARTITION_SURFACE_TOKEN);
        assert_eq!(r.pdi_marker_public, PUBLIC_PDI_MARKER_TOKEN);
        assert_eq!(r.partition_surface_public, "status_name_only");
        assert_eq!(r.pdi_marker_public, "marker_absent_public");

        let n = r.format_soft_note();
        assert!(residual_note_has_exact_token(
            &n,
            "app_load_pdi_partition_research"
        ));
        assert!(residual_note_has_exact_token(&n, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&n, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&n, "not_ran_on_npu=1"));
        assert!(residual_note_has_exact_token(
            &n,
            "do_not_invent_pdi_layout=1"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "public_pdi_image_decode=0"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "partition_surface_public=status_name_only"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "pdi_marker_public=marker_absent_public"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "fw_residual_open_after_userspace=1"
        ));
        assert!(residual_note_has_exact_token(&n, "userspace_content_ok=1"));
        assert!(residual_note_has_exact_token(&n, "c2_app_load_pair=1"));
        assert!(residual_note_has_exact_token(
            &n,
            "class_app_load_after_aiebu=1"
        ));
        assert!(residual_note_has_exact_token(&n, "failed_cmd=0x3000003"));
        assert!(residual_note_has_exact_token(
            &n,
            "failed_cmd_name=APP_LOAD_PDI_FAIL"
        ));
        assert!(residual_note_has_exact_token(&n, "chain_status=0x4000003"));
        assert!(residual_note_has_exact_token(
            &n,
            "chain_name=AIE2_STATUS_INVALID_PARAM"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "public_fw_name_implies_pdi_load=1"
        ));
        // Exact-token honesty: never substring invent_pass / ran_on_npu success.
        assert!(!residual_note_has_exact_token(&n, "invent_pass=1"));
        assert!(!residual_note_has_exact_token(&n, "ran_on_npu=1"));
        assert!(!residual_note_has_exact_token(&n, "ran_on_npu=true"));
        assert!(!residual_note_has_exact_token(&n, "c_ready=1"));
        assert!(!residual_note_has_exact_token(&n, "prop_count=15"));
        assert!(!residual_note_has_exact_token(
            &n,
            "public_pdi_image_decode=1"
        ));

        // Default is honesty-safe (surface tokens always PUBLIC_*; flags false).
        let d = AppLoadPdiPartitionResearch::default();
        assert_eq!(d.partition_surface_public, PUBLIC_PARTITION_SURFACE_TOKEN);
        assert_eq!(d.pdi_marker_public, PUBLIC_PDI_MARKER_TOKEN);
        assert!(!d.fw_residual_open_after_userspace);
        let dn = d.format_soft_note();
        assert!(residual_note_has_exact_token(
            &dn,
            "partition_surface_public=status_name_only"
        ));
        assert!(residual_note_has_exact_token(
            &dn,
            "pdi_marker_public=marker_absent_public"
        ));
        assert!(residual_note_has_exact_token(
            &dn,
            "fw_residual_open_after_userspace=0"
        ));
        assert!(residual_note_has_exact_token(&dn, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&dn, "not_c_ready=1"));
        // Never empty surface tokens from Default.
        assert!(!residual_note_has_exact_token(
            &dn,
            "partition_surface_public="
        ));
        assert!(!residual_note_has_exact_token(&dn, "pdi_marker_public="));

        // Named constants match aie2_fw_status_name.
        assert_eq!(
            aie2_fw_status_name(AIE2_FW_APP_LOAD_PDI_FAIL),
            "APP_LOAD_PDI_FAIL"
        );
        assert_eq!(
            aie2_fw_status_name(AIE2_FW_INVALID_PARAM),
            "AIE2_STATUS_INVALID_PARAM"
        );
        assert_eq!(AIE2_FW_APP_LOAD_PDI_FAIL, 0x3_000_003);
        assert_eq!(AIE2_FW_INVALID_PARAM, 0x4_000_003);

        // Locked journal fixture parses to C2 pair.
        let pair = parse_journal_fw_status_pair(c2_locked_app_load_journal_text());
        assert!(pair.is_c2_app_load_pair());
    }

    #[test]
    fn app_load_pdi_partition_dual_residual_truth_table() {
        // TFT: pair-only + userspace=1 + non-APP_LOAD class → fw open (locks || pair).
        let pair = parse_journal_fw_status_pair(c2_locked_app_load_journal_text());
        assert!(pair.is_c2_app_load_pair());
        let pair_us = research_app_load_pdi_partition(
            SiliconResidualClass::ErtErrorUnknownBlob,
            Some(&pair),
            true,
        );
        assert!(!pair_us.class_app_load_after_aiebu);
        assert!(pair_us.c2_app_load_pair);
        assert!(pair_us.userspace_content_ok);
        assert!(
            pair_us.fw_residual_open_after_userspace,
            "pair alone + userspace must open dual residual"
        );
        let pn = pair_us.format_soft_note();
        assert!(residual_note_has_exact_token(
            &pn,
            "fw_residual_open_after_userspace=1"
        ));
        assert!(residual_note_has_exact_token(&pn, "c2_app_load_pair=1"));
        assert!(residual_note_has_exact_token(
            &pn,
            "class_app_load_after_aiebu=0"
        ));
        assert!(residual_note_has_exact_token(&pn, "userspace_content_ok=1"));
        assert!(residual_note_has_exact_token(&pn, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&pn, "not_c_ready=1"));

        // userspace=1 + non-APP_LOAD + no pair → fw closed.
        let us_only =
            research_app_load_pdi_partition(SiliconResidualClass::ErtErrorUnknownBlob, None, true);
        assert!(!us_only.fw_residual_open_after_userspace);
        assert!(us_only.userspace_content_ok);
        let un = us_only.format_soft_note();
        assert!(residual_note_has_exact_token(
            &un,
            "fw_residual_open_after_userspace=0"
        ));
        assert!(residual_note_has_exact_token(&un, "userspace_content_ok=1"));
        assert!(residual_note_has_exact_token(&un, "c2_app_load_pair=0"));
        assert!(residual_note_has_exact_token(&un, "invent_pass=0"));
    }

    #[test]
    fn app_load_pdi_partition_research_negatives_no_invent() {
        // Split: userspace=false × (class-only | pair-only | neither) → fw=0.
        let class_only_no_us = research_app_load_pdi_partition(
            SiliconResidualClass::AppLoadPdiFailAfterAiebu,
            None,
            false,
        );
        assert!(class_only_no_us.class_app_load_after_aiebu);
        assert!(!class_only_no_us.c2_app_load_pair);
        assert!(!class_only_no_us.userspace_content_ok);
        assert!(!class_only_no_us.fw_residual_open_after_userspace);
        let cn = class_only_no_us.format_soft_note();
        assert!(residual_note_has_exact_token(
            &cn,
            "fw_residual_open_after_userspace=0"
        ));
        assert!(residual_note_has_exact_token(&cn, "userspace_content_ok=0"));
        assert!(residual_note_has_exact_token(
            &cn,
            "class_app_load_after_aiebu=1"
        ));
        assert!(residual_note_has_exact_token(&cn, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&cn, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(
            &cn,
            "partition_surface_public=status_name_only"
        ));

        let pair_only_no_us = research_app_load_pdi_partition_c2_locked(false);
        assert!(pair_only_no_us.c2_app_load_pair);
        assert!(!pair_only_no_us.userspace_content_ok);
        assert!(!pair_only_no_us.fw_residual_open_after_userspace);
        let pn = pair_only_no_us.format_soft_note();
        assert!(residual_note_has_exact_token(
            &pn,
            "fw_residual_open_after_userspace=0"
        ));
        assert!(residual_note_has_exact_token(&pn, "userspace_content_ok=0"));
        assert!(residual_note_has_exact_token(&pn, "c2_app_load_pair=1"));
        assert!(residual_note_has_exact_token(&pn, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&pn, "not_c_ready=1"));

        let neither =
            research_app_load_pdi_partition(SiliconResidualClass::ErtErrorUnknownBlob, None, false);
        assert!(!neither.class_app_load_after_aiebu);
        assert!(!neither.c2_app_load_pair);
        assert!(!neither.userspace_content_ok);
        assert!(!neither.fw_residual_open_after_userspace);
        let nn = neither.format_soft_note();
        assert!(residual_note_has_exact_token(
            &nn,
            "fw_residual_open_after_userspace=0"
        ));
        assert!(residual_note_has_exact_token(&nn, "failed_cmd=absent"));
        assert!(residual_note_has_exact_token(&nn, "chain_status=absent"));
        assert!(residual_note_has_exact_token(&nn, "c2_app_load_pair=0"));
        assert!(residual_note_has_exact_token(
            &nn,
            "public_fw_name_implies_pdi_load=0"
        ));
        assert!(residual_note_has_exact_token(&nn, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&nn, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&nn, "not_ran_on_npu=1"));
        assert!(residual_note_has_exact_token(
            &nn,
            "partition_surface_public=status_name_only"
        ));
        assert!(residual_note_has_exact_token(
            &nn,
            "pdi_marker_public=marker_absent_public"
        ));
        assert!(!residual_note_has_exact_token(&nn, "failed_cmd=0x3000003"));
        assert!(!residual_note_has_exact_token(
            &nn,
            "failed_cmd_name=APP_LOAD_PDI_FAIL"
        ));

        // Empty journal + APP_LOAD + userspace=1 → fw open (class arm); exact token.
        let empty = research_app_load_pdi_partition_from_journal(
            SiliconResidualClass::AppLoadPdiFailAfterAiebu,
            Some("   \n\t  "),
            true,
        );
        assert!(empty.failed_cmd_status.is_none());
        assert!(empty.class_app_load_after_aiebu);
        assert!(empty.fw_residual_open_after_userspace);
        assert!(!empty.c2_app_load_pair);
        let en = empty.format_soft_note();
        assert!(residual_note_has_exact_token(
            &en,
            "fw_residual_open_after_userspace=1"
        ));
        assert!(residual_note_has_exact_token(&en, "failed_cmd=absent"));
        assert!(residual_note_has_exact_token(
            &en,
            "public_fw_name_implies_pdi_load=1"
        ));
        assert!(residual_note_has_exact_token(&en, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&en, "not_c_ready=1"));

        // Partial journal (failed only) — no invent chain / no c2 pair; class+us open.
        let partial = research_app_load_pdi_partition_from_journal(
            SiliconResidualClass::AppLoadPdiFailAfterAiebu,
            Some("Failed cmd idx 0, status 0x3000003\n"),
            true,
        );
        assert_eq!(partial.failed_cmd_status, Some(AIE2_FW_APP_LOAD_PDI_FAIL));
        assert!(partial.chain_status.is_none());
        assert!(!partial.c2_app_load_pair);
        assert!(partial.fw_residual_open_after_userspace);
        let prn = partial.format_soft_note();
        assert!(residual_note_has_exact_token(
            &prn,
            "fw_residual_open_after_userspace=1"
        ));
        assert!(residual_note_has_exact_token(
            &prn,
            "failed_cmd_name=APP_LOAD_PDI_FAIL"
        ));
        assert!(residual_note_has_exact_token(&prn, "chain_status=absent"));
        assert!(residual_note_has_exact_token(&prn, "c2_app_load_pair=0"));
        assert!(residual_note_has_exact_token(&prn, "invent_pass=0"));

        // Oversized journal paste → treated as absent (no invent codes).
        let over = "x".repeat(MAX_APP_LOAD_JOURNAL_TEXT_BYTES.saturating_add(1));
        let oversized = research_app_load_pdi_partition_from_journal(
            SiliconResidualClass::ErtErrorUnknownBlob,
            Some(&over),
            false,
        );
        assert!(oversized.failed_cmd_status.is_none());
        assert!(!oversized.c2_app_load_pair);
        let on = oversized.format_soft_note();
        assert!(residual_note_has_exact_token(&on, "failed_cmd=absent"));
        assert!(residual_note_has_exact_token(
            &on,
            "fw_residual_open_after_userspace=0"
        ));
        assert!(residual_note_has_exact_token(&on, "invent_pass=0"));

        // Unknown FW code name — still no invent PDI layout.
        let unk = research_app_load_pdi_partition(
            SiliconResidualClass::ErtErrorUnknownBlob,
            Some(&JournalFwStatusPair {
                failed_cmd_status: Some(0xdead_beef),
                chain_status: None,
            }),
            false,
        );
        let un = unk.format_soft_note();
        assert!(residual_note_has_exact_token(
            &un,
            "failed_cmd_name=unknown_fw_status"
        ));
        assert!(residual_note_has_exact_token(
            &un,
            "public_fw_name_implies_pdi_load=0"
        ));
        assert!(residual_note_has_exact_token(
            &un,
            "public_pdi_image_decode=0"
        ));
        assert!(residual_note_has_exact_token(
            &un,
            "fw_residual_open_after_userspace=0"
        ));
        assert!(residual_note_has_exact_token(&un, "invent_pass=0"));
    }

    #[test]
    fn public_pdi_evidence_scan_track_m_closed_and_honesty() {
        // Track M: closed bag locks no new PDI fields beyond Track L.
        let s = scan_public_pdi_partition_evidence();
        assert_eq!(s.public_pdi_evidence_depth, PUBLIC_PDI_EVIDENCE_DEPTH_TOKEN);
        assert_eq!(s.public_pdi_evidence_depth, PUBLIC_PARTITION_SURFACE_TOKEN);
        assert_eq!(s.public_pdi_evidence_depth, "status_name_only");
        assert_eq!(s.new_public_pdi_fields, PUBLIC_PDI_NEW_FIELDS_CLOSED);
        assert_eq!(s.new_public_pdi_fields, 0);
        assert!(s.do_not_invent_pdi_layout);
        assert!(!s.public_pdi_image_decode);
        assert_eq!(s.partition_surface_public, PUBLIC_PARTITION_SURFACE_TOKEN);
        assert_eq!(s.pdi_marker_public, PUBLIC_PDI_MARKER_TOKEN);
        assert!(s.app_load_code_name_ok);
        assert!(s.invalid_param_code_name_ok);
        assert!(s.is_scan_closed_no_new_public_fields());

        // Closed alias is identical.
        let c = scan_public_pdi_partition_evidence_closed();
        assert_eq!(c, s);

        // Default == scan path (name_ok via aie2_fw_status_name; no hard-code drift).
        let d = PublicPdiEvidenceScan::default();
        assert_eq!(d, s);
        assert_eq!(d, scan_public_pdi_partition_evidence());
        let dn = d.format_soft_note();
        assert!(residual_note_has_exact_token(
            &dn,
            "scan_closed_no_new_public_fields=1"
        ));
        assert!(residual_note_has_exact_token(&dn, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&dn, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&dn, "not_ran_on_npu=1"));

        let n = s.format_soft_note();
        assert!(residual_note_has_exact_token(
            &n,
            "public_pdi_evidence_scan"
        ));
        assert!(residual_note_has_exact_token(&n, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&n, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&n, "not_ran_on_npu=1"));
        assert!(residual_note_has_exact_token(
            &n,
            "public_pdi_evidence_depth=status_name_only"
        ));
        assert!(residual_note_has_exact_token(&n, "new_public_pdi_fields=0"));
        assert!(residual_note_has_exact_token(
            &n,
            "do_not_invent_pdi_layout=1"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "public_pdi_image_decode=0"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "partition_surface_public=status_name_only"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "pdi_marker_public=marker_absent_public"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "scan_closed_no_new_public_fields=1"
        ));
        // Named key const must match emitted soft token prefix.
        assert!(n.contains(&format!("{PUBLIC_PDI_SCAN_CLOSED_TOKEN_KEY}=")));
        assert!(residual_note_has_exact_token(&n, "app_load_code_name_ok=1"));
        assert!(residual_note_has_exact_token(
            &n,
            "invalid_param_code_name_ok=1"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "locked_app_load=0x3000003"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "locked_invalid_param=0x4000003"
        ));
        // Exact-token honesty: never success / invent / layout decode.
        assert!(!residual_note_has_exact_token(&n, "invent_pass=1"));
        assert!(!residual_note_has_exact_token(&n, "ran_on_npu=1"));
        assert!(!residual_note_has_exact_token(&n, "ran_on_npu=true"));
        assert!(!residual_note_has_exact_token(&n, "c_ready=1"));
        assert!(!residual_note_has_exact_token(&n, "prop_count=15"));
        assert!(!residual_note_has_exact_token(
            &n,
            "public_pdi_image_decode=1"
        ));
        assert!(!residual_note_has_exact_token(
            &n,
            "new_public_pdi_fields=1"
        ));
        assert!(!residual_note_has_exact_token(
            &n,
            "public_pdi_evidence_depth="
        ));
        assert!(!residual_note_has_exact_token(
            &n,
            "partition_surface_public="
        ));
        assert!(!residual_note_has_exact_token(&n, "pdi_marker_public="));
        // Ban invented stronger-depth soft tokens (exact; not weak assert_ne on field).
        assert!(!residual_note_has_exact_token(
            &n,
            "public_pdi_evidence_depth=image_layout_public"
        ));
        assert!(!residual_note_has_exact_token(
            &n,
            "public_pdi_evidence_depth=ioctl_decode_public"
        ));

        // Locked codes still status-name only (Track L constants).
        assert_eq!(AIE2_FW_APP_LOAD_PDI_FAIL, 0x3_000_003);
        assert_eq!(AIE2_FW_INVALID_PARAM, 0x4_000_003);
        assert_eq!(
            aie2_fw_status_name(AIE2_FW_APP_LOAD_PDI_FAIL),
            "APP_LOAD_PDI_FAIL"
        );
        assert_eq!(
            aie2_fw_status_name(AIE2_FW_INVALID_PARAM),
            "AIE2_STATUS_INVALID_PARAM"
        );

        // Token split: Track M research note does not emit checklist_ prefixes
        // (checklist remains class-gated under format_app_load_offline_checklist*).
        assert!(!residual_note_has_exact_token(
            &n,
            "checklist_partition_surface_public=status_name_only"
        ));
        assert!(!residual_note_has_exact_token(
            &n,
            "checklist_pdi_marker_public=marker_absent_public"
        ));
        assert!(!residual_note_has_exact_token(
            &n,
            "checklist_public_pdi_image_decode=0"
        ));
        assert!(!residual_note_has_exact_token(
            &n,
            "checklist_userspace_elf_rela_app_load=1"
        ));
        assert!(!residual_note_has_exact_token(
            &n,
            "userspace_ok_fw_load_open=1"
        ));
        assert!(!residual_note_has_exact_token(
            &n,
            "userspace_patch_ok_fw_load_still_open=1"
        ));
        assert!(!residual_note_has_exact_token(
            &n,
            "fw_residual_open_after_userspace=1"
        ));

        // Hostile hand-built bag: closed bit must derive =0; honesty trio still holds.
        let mut hostile = s.clone();
        hostile.new_public_pdi_fields = 1;
        assert!(!hostile.is_scan_closed_no_new_public_fields());
        let hn = hostile.format_soft_note();
        assert!(residual_note_has_exact_token(
            &hn,
            "scan_closed_no_new_public_fields=0"
        ));
        assert!(residual_note_has_exact_token(
            &hn,
            "new_public_pdi_fields=1"
        ));
        assert!(residual_note_has_exact_token(&hn, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&hn, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&hn, "not_ran_on_npu=1"));
        assert!(!residual_note_has_exact_token(
            &hn,
            "scan_closed_no_new_public_fields=1"
        ));

        // Hostile decode / invent-layout flags also clear closed.
        let mut hostile_decode = s.clone();
        hostile_decode.public_pdi_image_decode = true;
        assert!(!hostile_decode.is_scan_closed_no_new_public_fields());
        let hdn = hostile_decode.format_soft_note();
        assert!(residual_note_has_exact_token(
            &hdn,
            "scan_closed_no_new_public_fields=0"
        ));
        assert!(residual_note_has_exact_token(
            &hdn,
            "public_pdi_image_decode=1"
        ));
        assert!(residual_note_has_exact_token(&hdn, "invent_pass=0"));

        let mut hostile_layout = s.clone();
        hostile_layout.do_not_invent_pdi_layout = false;
        assert!(!hostile_layout.is_scan_closed_no_new_public_fields());
        let hln = hostile_layout.format_soft_note();
        assert!(residual_note_has_exact_token(
            &hln,
            "scan_closed_no_new_public_fields=0"
        ));
        assert!(residual_note_has_exact_token(
            &hln,
            "do_not_invent_pdi_layout=0"
        ));
        assert!(residual_note_has_exact_token(&hln, "not_c_ready=1"));

        // Soft-note sanitizer on M path: space/= inject cannot mint invent_pass=1.
        let mut inject_m = s.clone();
        inject_m.public_pdi_evidence_depth = "status invent_pass=1";
        // M closed does not gate on depth — still closed bit may stay 1; honesty
        // trio + sanitizer must still hold (no injected invent_pass=1 token).
        let imn = inject_m.format_soft_note();
        assert!(!residual_note_has_exact_token(&imn, "invent_pass=1"));
        assert!(residual_note_has_exact_token(&imn, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&imn, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&imn, "not_ran_on_npu=1"));
        assert!(residual_note_has_exact_token(
            &imn,
            "public_pdi_evidence_depth=status_invent_pass_1"
        ));
    }

    #[test]
    fn external_public_pdi_evidence_scan_track_n_closed_and_honesty() {
        // Track N: external public evidence closed — no new kernel/XRT beyond status-name.
        let s = scan_external_public_pdi_evidence();
        assert_eq!(
            s.external_public_pdi_evidence,
            EXTERNAL_PUBLIC_PDI_EVIDENCE_TOKEN
        );
        assert_eq!(s.external_public_pdi_evidence, "closed_no_new");
        assert_eq!(
            s.external_kernel_xrt_beyond_status_name,
            EXTERNAL_KERNEL_XRT_BEYOND_STATUS_NAME
        );
        assert_eq!(s.external_kernel_xrt_beyond_status_name, 0);
        assert_eq!(s.public_pdi_evidence_depth, PUBLIC_PDI_EVIDENCE_DEPTH_TOKEN);
        assert_eq!(s.public_pdi_evidence_depth, "status_name_only");
        assert!(s.do_not_invent_pdi_layout);
        assert!(!s.public_pdi_image_decode);
        assert!(s.app_load_code_name_ok);
        assert!(s.invalid_param_code_name_ok);
        assert!(s.is_external_scan_closed_no_new());

        // Closed alias is identical.
        let c = scan_external_public_pdi_evidence_closed();
        assert_eq!(c, s);

        // Default == scan path (name_ok via aie2_fw_status_name; no hard-code drift).
        let d = ExternalPublicPdiEvidenceScan::default();
        assert_eq!(d, s);
        assert_eq!(d, scan_external_public_pdi_evidence());
        let dn = d.format_soft_note();
        assert!(residual_note_has_exact_token(
            &dn,
            "external_scan_closed_no_new=1"
        ));
        assert!(residual_note_has_exact_token(&dn, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&dn, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&dn, "not_ran_on_npu=1"));

        let n = s.format_soft_note();
        assert!(residual_note_has_exact_token(
            &n,
            "external_public_pdi_evidence_scan"
        ));
        assert!(residual_note_has_exact_token(&n, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&n, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&n, "not_ran_on_npu=1"));
        assert!(residual_note_has_exact_token(
            &n,
            "external_public_pdi_evidence=closed_no_new"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "external_kernel_xrt_beyond_status_name=0"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "public_pdi_evidence_depth=status_name_only"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "do_not_invent_pdi_layout=1"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "public_pdi_image_decode=0"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "external_scan_closed_no_new=1"
        ));
        // Named key const must match emitted soft token prefix.
        assert!(n.contains(&format!("{EXTERNAL_PUBLIC_PDI_SCAN_CLOSED_TOKEN_KEY}=")));
        assert!(residual_note_has_exact_token(&n, "app_load_code_name_ok=1"));
        assert!(residual_note_has_exact_token(
            &n,
            "invalid_param_code_name_ok=1"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "locked_app_load=0x3000003"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "locked_invalid_param=0x4000003"
        ));
        // Exact-token honesty: never success / invent / layout decode.
        assert!(!residual_note_has_exact_token(&n, "invent_pass=1"));
        assert!(!residual_note_has_exact_token(&n, "ran_on_npu=1"));
        assert!(!residual_note_has_exact_token(&n, "ran_on_npu=true"));
        assert!(!residual_note_has_exact_token(&n, "c_ready=1"));
        assert!(!residual_note_has_exact_token(&n, "prop_count=15"));
        assert!(!residual_note_has_exact_token(
            &n,
            "public_pdi_image_decode=1"
        ));
        assert!(!residual_note_has_exact_token(
            &n,
            "external_kernel_xrt_beyond_status_name=1"
        ));
        assert!(!residual_note_has_exact_token(
            &n,
            "external_public_pdi_evidence="
        ));
        assert!(!residual_note_has_exact_token(
            &n,
            "public_pdi_evidence_depth="
        ));
        // Ban invented stronger-depth soft tokens.
        assert!(!residual_note_has_exact_token(
            &n,
            "public_pdi_evidence_depth=image_layout_public"
        ));
        assert!(!residual_note_has_exact_token(
            &n,
            "public_pdi_evidence_depth=ioctl_decode_public"
        ));
        assert!(!residual_note_has_exact_token(
            &n,
            "external_public_pdi_evidence=layout_public"
        ));

        // Locked codes still status-name only (Track L/M constants).
        assert_eq!(AIE2_FW_APP_LOAD_PDI_FAIL, 0x3_000_003);
        assert_eq!(AIE2_FW_INVALID_PARAM, 0x4_000_003);
        assert_eq!(
            aie2_fw_status_name(AIE2_FW_APP_LOAD_PDI_FAIL),
            "APP_LOAD_PDI_FAIL"
        );
        assert_eq!(
            aie2_fw_status_name(AIE2_FW_INVALID_PARAM),
            "AIE2_STATUS_INVALID_PARAM"
        );

        // Token split: Track N research note does not emit checklist_ prefixes
        // or invent K userspace / dual-residual open / Track M closed bag name.
        assert!(!residual_note_has_exact_token(
            &n,
            "checklist_partition_surface_public=status_name_only"
        ));
        assert!(!residual_note_has_exact_token(
            &n,
            "checklist_pdi_marker_public=marker_absent_public"
        ));
        assert!(!residual_note_has_exact_token(
            &n,
            "checklist_public_pdi_image_decode=0"
        ));
        assert!(!residual_note_has_exact_token(
            &n,
            "checklist_userspace_elf_rela_app_load=1"
        ));
        assert!(!residual_note_has_exact_token(
            &n,
            "userspace_ok_fw_load_open=1"
        ));
        assert!(!residual_note_has_exact_token(
            &n,
            "userspace_patch_ok_fw_load_still_open=1"
        ));
        assert!(!residual_note_has_exact_token(
            &n,
            "fw_residual_open_after_userspace=1"
        ));
        // Distinct from Track M research bag name (no bleed).
        assert!(!residual_note_has_exact_token(
            &n,
            "public_pdi_evidence_scan"
        ));
        assert!(!residual_note_has_exact_token(
            &n,
            "scan_closed_no_new_public_fields=1"
        ));
        assert!(!residual_note_has_exact_token(
            &n,
            "new_public_pdi_fields=0"
        ));

        // Hostile hand-built bag: closed bit must derive =0; honesty trio still holds.
        let mut hostile = s.clone();
        hostile.external_kernel_xrt_beyond_status_name = 1;
        assert!(!hostile.is_external_scan_closed_no_new());
        let hn = hostile.format_soft_note();
        assert!(residual_note_has_exact_token(
            &hn,
            "external_scan_closed_no_new=0"
        ));
        assert!(residual_note_has_exact_token(
            &hn,
            "external_kernel_xrt_beyond_status_name=1"
        ));
        assert!(residual_note_has_exact_token(&hn, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&hn, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&hn, "not_ran_on_npu=1"));
        assert!(!residual_note_has_exact_token(
            &hn,
            "external_scan_closed_no_new=1"
        ));

        // Hostile decode / invent-layout / depth / token flags also clear closed.
        // Each secondary bag: full honesty trio + ban closed=1 (Issue 6).
        let assert_hostile_n = |note: &str, marker: &str| {
            assert!(
                residual_note_has_exact_token(note, "external_scan_closed_no_new=0"),
                "expected closed=0 for {marker}"
            );
            assert!(
                !residual_note_has_exact_token(note, "external_scan_closed_no_new=1"),
                "ban closed=1 for {marker}"
            );
            assert!(residual_note_has_exact_token(note, "invent_pass=0"));
            assert!(residual_note_has_exact_token(note, "not_c_ready=1"));
            assert!(residual_note_has_exact_token(note, "not_ran_on_npu=1"));
        };

        let mut hostile_decode = s.clone();
        hostile_decode.public_pdi_image_decode = true;
        assert!(!hostile_decode.is_external_scan_closed_no_new());
        let hdn = hostile_decode.format_soft_note();
        assert!(residual_note_has_exact_token(
            &hdn,
            "public_pdi_image_decode=1"
        ));
        assert_hostile_n(&hdn, "decode");

        let mut hostile_layout = s.clone();
        hostile_layout.do_not_invent_pdi_layout = false;
        assert!(!hostile_layout.is_external_scan_closed_no_new());
        let hln = hostile_layout.format_soft_note();
        assert!(residual_note_has_exact_token(
            &hln,
            "do_not_invent_pdi_layout=0"
        ));
        assert_hostile_n(&hln, "layout");

        let mut hostile_depth = s.clone();
        hostile_depth.public_pdi_evidence_depth = "image_layout_public";
        assert!(!hostile_depth.is_external_scan_closed_no_new());
        let hdep = hostile_depth.format_soft_note();
        assert!(residual_note_has_exact_token(
            &hdep,
            "public_pdi_evidence_depth=image_layout_public"
        ));
        assert_hostile_n(&hdep, "depth");

        let mut hostile_tok = s.clone();
        hostile_tok.external_public_pdi_evidence = "layout_public";
        assert!(!hostile_tok.is_external_scan_closed_no_new());
        let htk = hostile_tok.format_soft_note();
        assert!(residual_note_has_exact_token(
            &htk,
            "external_public_pdi_evidence=layout_public"
        ));
        assert_hostile_n(&htk, "token");

        // Issue 7: name_ok=false is diagnostic-only — closed bit stays 1 (Track M parity).
        let mut name_diag = s.clone();
        name_diag.app_load_code_name_ok = false;
        name_diag.invalid_param_code_name_ok = false;
        assert!(
            name_diag.is_external_scan_closed_no_new(),
            "name_ok flags must not clear external closed (diagnostic-only)"
        );
        let ndn = name_diag.format_soft_note();
        assert!(residual_note_has_exact_token(
            &ndn,
            "external_scan_closed_no_new=1"
        ));
        assert!(residual_note_has_exact_token(
            &ndn,
            "app_load_code_name_ok=0"
        ));
        assert!(residual_note_has_exact_token(
            &ndn,
            "invalid_param_code_name_ok=0"
        ));
        assert!(residual_note_has_exact_token(&ndn, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&ndn, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&ndn, "not_ran_on_npu=1"));

        // Soft-note sanitizer on N path: space/= inject cannot mint invent_pass=1.
        let mut inject_n = s.clone();
        inject_n.external_public_pdi_evidence = "x invent_pass=1 ran_on_npu=1";
        assert!(!inject_n.is_external_scan_closed_no_new());
        let inn = inject_n.format_soft_note();
        assert!(!residual_note_has_exact_token(&inn, "invent_pass=1"));
        assert!(!residual_note_has_exact_token(&inn, "ran_on_npu=1"));
        assert!(residual_note_has_exact_token(&inn, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&inn, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&inn, "not_ran_on_npu=1"));
        assert!(residual_note_has_exact_token(
            &inn,
            "external_public_pdi_evidence=x_invent_pass_1_ran_on_npu_1"
        ));
        assert!(residual_note_has_exact_token(
            &inn,
            "external_scan_closed_no_new=0"
        ));
    }

    #[test]
    fn app_load_maintain_reconfirm_track_o_closed_and_honesty() {
        // Track O: maintain reconfirm A–N still closed — no new external public evidence.
        let s = reconfirm_app_load_tracks_a_n_closed();
        assert_eq!(s.maintain_reconfirm, APP_LOAD_MAINTAIN_RECONFIRM_TOKEN);
        assert_eq!(s.maintain_reconfirm, "a_n_closed_no_new");
        assert_eq!(APP_LOAD_MAINTAIN_RECONFIRM_OPEN_TOKEN, "not_closed");
        assert_ne!(
            APP_LOAD_MAINTAIN_RECONFIRM_TOKEN,
            APP_LOAD_MAINTAIN_RECONFIRM_OPEN_TOKEN
        );
        assert!(s.in_tree_m_closed);
        assert!(s.external_n_closed);
        assert_eq!(s.public_pdi_evidence_depth, PUBLIC_PDI_EVIDENCE_DEPTH_TOKEN);
        assert_eq!(s.public_pdi_evidence_depth, "status_name_only");
        assert!(s.do_not_invent_pdi_layout);
        assert!(!s.public_pdi_image_decode);
        assert_eq!(s.new_public_pdi_fields, PUBLIC_PDI_NEW_FIELDS_CLOSED);
        assert_eq!(
            s.external_kernel_xrt_beyond_status_name,
            EXTERNAL_KERNEL_XRT_BEYOND_STATUS_NAME
        );
        assert_eq!(s.new_public_pdi_fields, 0);
        assert_eq!(s.external_kernel_xrt_beyond_status_name, 0);
        assert!(s.app_load_code_name_ok);
        assert!(s.invalid_param_code_name_ok);
        assert!(s.is_maintain_reconfirm_a_n_closed());

        // Maintain alias is identical.
        let c = reconfirm_app_load_tracks_a_n_closed_maintain();
        assert_eq!(c, s);

        // Default == reconfirm path (composes M+N; no hard-code drift).
        let d = AppLoadMaintainReconfirm::default();
        assert_eq!(d, s);
        assert_eq!(d, reconfirm_app_load_tracks_a_n_closed());
        let dn = d.format_soft_note();
        assert!(residual_note_has_exact_token(
            &dn,
            "maintain_reconfirm_a_n_closed=1"
        ));
        assert!(residual_note_has_exact_token(&dn, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&dn, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&dn, "not_ran_on_npu=1"));
        // Closed/Default path must not emit open token.
        assert!(!residual_note_has_exact_token(
            &dn,
            "maintain_reconfirm=not_closed"
        ));
        assert!(residual_note_has_exact_token(
            &dn,
            "maintain_reconfirm=a_n_closed_no_new"
        ));

        let n = s.format_soft_note();
        assert!(residual_note_has_exact_token(
            &n,
            "app_load_maintain_reconfirm"
        ));
        assert!(residual_note_has_exact_token(&n, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&n, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&n, "not_ran_on_npu=1"));
        assert!(residual_note_has_exact_token(
            &n,
            "maintain_reconfirm=a_n_closed_no_new"
        ));
        assert!(!residual_note_has_exact_token(
            &n,
            "maintain_reconfirm=not_closed"
        ));
        assert!(residual_note_has_exact_token(&n, "in_tree_m_closed=1"));
        assert!(residual_note_has_exact_token(&n, "external_n_closed=1"));
        assert!(residual_note_has_exact_token(
            &n,
            "public_pdi_evidence_depth=status_name_only"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "do_not_invent_pdi_layout=1"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "public_pdi_image_decode=0"
        ));
        assert!(residual_note_has_exact_token(&n, "new_public_pdi_fields=0"));
        assert!(residual_note_has_exact_token(
            &n,
            "external_kernel_xrt_beyond_status_name=0"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "maintain_reconfirm_a_n_closed=1"
        ));
        // Named key const must match emitted soft token prefix.
        assert!(n.contains(&format!("{APP_LOAD_MAINTAIN_RECONFIRM_CLOSED_TOKEN_KEY}=")));
        assert!(residual_note_has_exact_token(&n, "app_load_code_name_ok=1"));
        assert!(residual_note_has_exact_token(
            &n,
            "invalid_param_code_name_ok=1"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "locked_app_load=0x3000003"
        ));
        assert!(residual_note_has_exact_token(
            &n,
            "locked_invalid_param=0x4000003"
        ));
        // Exact-token honesty: never success / invent / layout decode.
        assert!(!residual_note_has_exact_token(&n, "invent_pass=1"));
        assert!(!residual_note_has_exact_token(&n, "ran_on_npu=1"));
        assert!(!residual_note_has_exact_token(&n, "ran_on_npu=true"));
        assert!(!residual_note_has_exact_token(&n, "c_ready=1"));
        assert!(!residual_note_has_exact_token(&n, "prop_count=15"));
        assert!(!residual_note_has_exact_token(
            &n,
            "public_pdi_image_decode=1"
        ));
        assert!(!residual_note_has_exact_token(
            &n,
            "external_kernel_xrt_beyond_status_name=1"
        ));
        assert!(!residual_note_has_exact_token(
            &n,
            "new_public_pdi_fields=1"
        ));
        assert!(!residual_note_has_exact_token(&n, "maintain_reconfirm="));
        assert!(!residual_note_has_exact_token(
            &n,
            "public_pdi_evidence_depth="
        ));
        // Ban invented stronger-depth / fake-progress soft tokens (N parity).
        assert!(!residual_note_has_exact_token(
            &n,
            "public_pdi_evidence_depth=image_layout_public"
        ));
        assert!(!residual_note_has_exact_token(
            &n,
            "public_pdi_evidence_depth=ioctl_decode_public"
        ));
        assert!(!residual_note_has_exact_token(
            &n,
            "public_pdi_evidence_depth=m_n_depth_mismatch"
        ));
        assert!(!residual_note_has_exact_token(
            &n,
            "maintain_reconfirm=new_evidence"
        ));
        // Distinct from Track N closed_no_new token (no bleed).
        assert!(!residual_note_has_exact_token(
            &n,
            "maintain_reconfirm=closed_no_new"
        ));
        assert!(!residual_note_has_exact_token(
            &n,
            "external_public_pdi_evidence=closed_no_new"
        ));

        // Locked codes still status-name only (Track L/M/N constants).
        assert_eq!(AIE2_FW_APP_LOAD_PDI_FAIL, 0x3_000_003);
        assert_eq!(AIE2_FW_INVALID_PARAM, 0x4_000_003);
        assert_eq!(
            aie2_fw_status_name(AIE2_FW_APP_LOAD_PDI_FAIL),
            "APP_LOAD_PDI_FAIL"
        );
        assert_eq!(
            aie2_fw_status_name(AIE2_FW_INVALID_PARAM),
            "AIE2_STATUS_INVALID_PARAM"
        );

        // Token split: Track O research note does not emit checklist_ prefixes
        // or invent K userspace / dual-residual open / Track M/N bag names.
        assert!(!residual_note_has_exact_token(
            &n,
            "checklist_partition_surface_public=status_name_only"
        ));
        assert!(!residual_note_has_exact_token(
            &n,
            "checklist_pdi_marker_public=marker_absent_public"
        ));
        assert!(!residual_note_has_exact_token(
            &n,
            "checklist_public_pdi_image_decode=0"
        ));
        assert!(!residual_note_has_exact_token(
            &n,
            "checklist_userspace_elf_rela_app_load=1"
        ));
        assert!(!residual_note_has_exact_token(
            &n,
            "userspace_ok_fw_load_open=1"
        ));
        assert!(!residual_note_has_exact_token(
            &n,
            "userspace_patch_ok_fw_load_still_open=1"
        ));
        assert!(!residual_note_has_exact_token(
            &n,
            "fw_residual_open_after_userspace=1"
        ));
        // Distinct from Track M/N research bag names (no bleed).
        assert!(!residual_note_has_exact_token(
            &n,
            "public_pdi_evidence_scan"
        ));
        assert!(!residual_note_has_exact_token(
            &n,
            "scan_closed_no_new_public_fields=1"
        ));
        assert!(!residual_note_has_exact_token(
            &n,
            "external_public_pdi_evidence_scan"
        ));
        assert!(!residual_note_has_exact_token(
            &n,
            "external_scan_closed_no_new=1"
        ));

        // Composed M+N scans are themselves closed (Track O depends on them).
        assert!(scan_public_pdi_partition_evidence().is_scan_closed_no_new_public_fields());
        assert!(scan_external_public_pdi_evidence().is_external_scan_closed_no_new());

        // Hostile hand-built bag: closed bit must derive =0; honesty trio still holds.
        let mut hostile = s.clone();
        hostile.external_n_closed = false;
        // Open path: production would set not_closed; hand-built open bit alone
        // also clears closed (token may still be a_n_closed_no_new until set).
        assert!(!hostile.is_maintain_reconfirm_a_n_closed());
        let hn = hostile.format_soft_note();
        assert!(residual_note_has_exact_token(
            &hn,
            "maintain_reconfirm_a_n_closed=0"
        ));
        assert!(residual_note_has_exact_token(&hn, "external_n_closed=0"));
        assert!(residual_note_has_exact_token(&hn, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&hn, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&hn, "not_ran_on_npu=1"));
        assert!(!residual_note_has_exact_token(
            &hn,
            "maintain_reconfirm_a_n_closed=1"
        ));

        let assert_hostile_o = |note: &str, marker: &str| {
            assert!(
                residual_note_has_exact_token(note, "maintain_reconfirm_a_n_closed=0"),
                "expected closed=0 for {marker}"
            );
            assert!(
                !residual_note_has_exact_token(note, "maintain_reconfirm_a_n_closed=1"),
                "ban closed=1 for {marker}"
            );
            assert!(residual_note_has_exact_token(note, "invent_pass=0"));
            assert!(residual_note_has_exact_token(note, "not_c_ready=1"));
            assert!(residual_note_has_exact_token(note, "not_ran_on_npu=1"));
        };

        // Explicit open token path (production emits when M/N not both closed).
        let mut open_tok = s.clone();
        open_tok.maintain_reconfirm = APP_LOAD_MAINTAIN_RECONFIRM_OPEN_TOKEN;
        open_tok.external_n_closed = false;
        assert_eq!(open_tok.maintain_reconfirm, "not_closed");
        assert!(!open_tok.is_maintain_reconfirm_a_n_closed());
        let otn = open_tok.format_soft_note();
        assert!(residual_note_has_exact_token(
            &otn,
            "maintain_reconfirm=not_closed"
        ));
        assert_hostile_o(&otn, "open_token");

        let mut hostile_m = s.clone();
        hostile_m.in_tree_m_closed = false;
        assert!(!hostile_m.is_maintain_reconfirm_a_n_closed());
        let hm = hostile_m.format_soft_note();
        assert!(residual_note_has_exact_token(&hm, "in_tree_m_closed=0"));
        assert_hostile_o(&hm, "m_closed");

        let mut hostile_fields = s.clone();
        hostile_fields.new_public_pdi_fields = 1;
        assert!(!hostile_fields.is_maintain_reconfirm_a_n_closed());
        let hf = hostile_fields.format_soft_note();
        assert!(residual_note_has_exact_token(
            &hf,
            "new_public_pdi_fields=1"
        ));
        assert_hostile_o(&hf, "new_fields");

        let mut hostile_ext = s.clone();
        hostile_ext.external_kernel_xrt_beyond_status_name = 1;
        assert!(!hostile_ext.is_maintain_reconfirm_a_n_closed());
        let he = hostile_ext.format_soft_note();
        assert!(residual_note_has_exact_token(
            &he,
            "external_kernel_xrt_beyond_status_name=1"
        ));
        assert_hostile_o(&he, "external_count");

        let mut hostile_decode = s.clone();
        hostile_decode.public_pdi_image_decode = true;
        assert!(!hostile_decode.is_maintain_reconfirm_a_n_closed());
        let hdn = hostile_decode.format_soft_note();
        assert!(residual_note_has_exact_token(
            &hdn,
            "public_pdi_image_decode=1"
        ));
        assert_hostile_o(&hdn, "decode");

        let mut hostile_layout = s.clone();
        hostile_layout.do_not_invent_pdi_layout = false;
        assert!(!hostile_layout.is_maintain_reconfirm_a_n_closed());
        let hln = hostile_layout.format_soft_note();
        assert!(residual_note_has_exact_token(
            &hln,
            "do_not_invent_pdi_layout=0"
        ));
        assert_hostile_o(&hln, "layout");

        let mut hostile_depth = s.clone();
        hostile_depth.public_pdi_evidence_depth = "image_layout_public";
        assert!(!hostile_depth.is_maintain_reconfirm_a_n_closed());
        let hdep = hostile_depth.format_soft_note();
        assert!(residual_note_has_exact_token(
            &hdep,
            "public_pdi_evidence_depth=image_layout_public"
        ));
        assert_hostile_o(&hdep, "depth");

        // Compose depth sentinel m_n_depth_mismatch → closed=0.
        let mut hostile_mm = s.clone();
        hostile_mm.public_pdi_evidence_depth = "m_n_depth_mismatch";
        assert!(!hostile_mm.is_maintain_reconfirm_a_n_closed());
        let hmm = hostile_mm.format_soft_note();
        assert!(residual_note_has_exact_token(
            &hmm,
            "public_pdi_evidence_depth=m_n_depth_mismatch"
        ));
        assert_hostile_o(&hmm, "m_n_depth_mismatch");

        let mut hostile_tok = s.clone();
        hostile_tok.maintain_reconfirm = "new_evidence";
        assert!(!hostile_tok.is_maintain_reconfirm_a_n_closed());
        let htk = hostile_tok.format_soft_note();
        assert!(residual_note_has_exact_token(
            &htk,
            "maintain_reconfirm=new_evidence"
        ));
        assert_hostile_o(&htk, "token");

        // Hostile: N's `closed_no_new` on maintain field must clear O closed
        // (token namespace split — O requires a_n_closed_no_new only).
        let mut hostile_n_tok = s.clone();
        hostile_n_tok.maintain_reconfirm = EXTERNAL_PUBLIC_PDI_EVIDENCE_TOKEN;
        assert_eq!(hostile_n_tok.maintain_reconfirm, "closed_no_new");
        assert!(!hostile_n_tok.is_maintain_reconfirm_a_n_closed());
        let hnt = hostile_n_tok.format_soft_note();
        assert!(residual_note_has_exact_token(
            &hnt,
            "maintain_reconfirm=closed_no_new"
        ));
        assert_hostile_o(&hnt, "n_token_on_maintain");

        // Hostile depth ioctl_decode_public (N parity ban).
        let mut hostile_ioctl = s.clone();
        hostile_ioctl.public_pdi_evidence_depth = "ioctl_decode_public";
        assert!(!hostile_ioctl.is_maintain_reconfirm_a_n_closed());
        let hio = hostile_ioctl.format_soft_note();
        assert!(residual_note_has_exact_token(
            &hio,
            "public_pdi_evidence_depth=ioctl_decode_public"
        ));
        assert_hostile_o(&hio, "ioctl_depth");

        // Soft-note sanitizer: space/=/control injection cannot mint invent_pass=1.
        let mut inject = s.clone();
        inject.maintain_reconfirm = "x invent_pass=1 ran_on_npu=1";
        assert!(!inject.is_maintain_reconfirm_a_n_closed());
        let inj = inject.format_soft_note();
        assert!(!residual_note_has_exact_token(&inj, "invent_pass=1"));
        assert!(!residual_note_has_exact_token(&inj, "ran_on_npu=1"));
        assert!(residual_note_has_exact_token(&inj, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&inj, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&inj, "not_ran_on_npu=1"));
        assert!(residual_note_has_exact_token(
            &inj,
            "maintain_reconfirm=x_invent_pass_1_ran_on_npu_1"
        ));
        assert_hostile_o(&inj, "space_inject");

        let mut inject_depth = s.clone();
        inject_depth.public_pdi_evidence_depth = "status invent_pass=1";
        assert!(!inject_depth.is_maintain_reconfirm_a_n_closed());
        let idn = inject_depth.format_soft_note();
        assert!(!residual_note_has_exact_token(&idn, "invent_pass=1"));
        assert!(residual_note_has_exact_token(&idn, "invent_pass=0"));
        assert!(residual_note_has_exact_token(
            &idn,
            "public_pdi_evidence_depth=status_invent_pass_1"
        ));
        assert_hostile_o(&idn, "depth_inject");

        // Shared sanitizer pure unit.
        assert_eq!(sanitize_soft_note_field_value("a b=c\td"), "a_b_c_d");
        assert_eq!(
            sanitize_soft_note_field_value(APP_LOAD_MAINTAIN_RECONFIRM_TOKEN),
            APP_LOAD_MAINTAIN_RECONFIRM_TOKEN
        );
        assert_eq!(
            sanitize_soft_note_field_value(APP_LOAD_MAINTAIN_RECONFIRM_OPEN_TOKEN),
            APP_LOAD_MAINTAIN_RECONFIRM_OPEN_TOKEN
        );

        // name_ok=false is diagnostic-only — closed bit stays 1 (M/N parity).
        let mut name_diag = s.clone();
        name_diag.app_load_code_name_ok = false;
        name_diag.invalid_param_code_name_ok = false;
        assert!(
            name_diag.is_maintain_reconfirm_a_n_closed(),
            "name_ok flags must not clear maintain closed (diagnostic-only)"
        );
        let ndn = name_diag.format_soft_note();
        assert!(residual_note_has_exact_token(
            &ndn,
            "maintain_reconfirm_a_n_closed=1"
        ));
        assert!(residual_note_has_exact_token(
            &ndn,
            "app_load_code_name_ok=0"
        ));
        assert!(residual_note_has_exact_token(
            &ndn,
            "invalid_param_code_name_ok=0"
        ));
        assert!(residual_note_has_exact_token(&ndn, "invent_pass=0"));
        assert!(residual_note_has_exact_token(&ndn, "not_c_ready=1"));
        assert!(residual_note_has_exact_token(&ndn, "not_ran_on_npu=1"));
    }

    #[test]
    #[allow(unsafe_code)] // test-only env isolation for TMPDIR=/ clamp
    fn last_apply_temp_root_clamps_fs_root_tmpdir() {
        // TMPDIR=/ must not make path_under_allowed_roots accept every absolute path.
        let _guard = TmpdirEnvGuard::set("/");
        let root = last_apply_temp_root();
        assert_ne!(
            root,
            Path::new("/"),
            "temp root must not be filesystem root"
        );
        // Clamped path is /tmp or its canonicalize (e.g. /private/tmp on some hosts).
        let as_str = root.to_string_lossy();
        assert!(
            as_str == "/tmp" || as_str.ends_with("/tmp") || root.ends_with("tmp"),
            "expected /tmp clamp, got {}",
            root.display()
        );
        // Outside home/etc must refuse even when TMPDIR was /.
        assert!(
            !path_under_allowed_roots(Path::new("/etc/x.json")),
            "/etc must refuse under TMPDIR=/"
        );
        if let Ok(home) = std::env::var("HOME") {
            let home_json = PathBuf::from(home).join("x.json");
            assert!(
                !path_under_allowed_roots(&home_json),
                "$HOME/x.json must refuse under TMPDIR=/"
            );
            assert!(!last_apply_path_is_allowed(&home_json));
        }
        // /tmp under clamp remains allow-root for .json names (lexical).
        assert!(path_under_allowed_roots(Path::new(
            "/tmp/dragon_npu_la_clamp_probe.json"
        )));
    }

    #[test]
    #[allow(unsafe_code)] // test-only env isolation for TMPDIR symlink→/
    fn last_apply_temp_root_clamps_symlink_to_fs_root() {
        // TMPDIR = symlink to / is not lexically RootDir; canonicalize → / must re-clamp.
        // Hard-fail if symlink cannot be created — Linux CI must cover this residual.
        let pid = std::process::id();
        let link = PathBuf::from(format!("/tmp/dragon_npu_tmpdir_rootlink_{pid}"));
        let _ = std::fs::remove_file(&link);
        let _ = std::fs::remove_dir(&link);
        std::os::unix::fs::symlink("/", &link).expect(
            "symlink to / under /tmp required for last_apply_temp_root clamp coverage (Linux CI)",
        );
        let _guard = TmpdirEnvGuard::set(&link);
        let root = last_apply_temp_root();
        assert_ne!(
            root,
            Path::new("/"),
            "symlink-to-/ TMPDIR must not resolve allow-root to /"
        );
        assert!(
            !path_is_filesystem_root(&root),
            "post-clamp root must not be fs root: {}",
            root.display()
        );
        // Parity with lexical TMPDIR=/ companion: clamped root is /tmp shape.
        let as_str = root.to_string_lossy();
        assert!(
            as_str == "/tmp" || as_str.ends_with("/tmp") || root.ends_with("tmp"),
            "expected /tmp clamp after symlink-to-/, got {}",
            root.display()
        );
        if let Ok(tmp_canon) = std::fs::canonicalize("/tmp") {
            assert_eq!(
                root, tmp_canon,
                "clamped root should equal canonicalize(/tmp)"
            );
        }
        assert!(
            !path_under_allowed_roots(Path::new("/etc/x.json")),
            "/etc must refuse under TMPDIR=symlink-to-/"
        );
        if let Ok(home) = std::env::var("HOME") {
            let home_json = PathBuf::from(home).join("x.json");
            assert!(
                !path_under_allowed_roots(&home_json),
                "$HOME/x.json must refuse under TMPDIR=symlink-to-/"
            );
            assert!(!last_apply_path_is_allowed(&home_json));
        }
        // Clamped /tmp still allows .json under /tmp.
        assert!(path_under_allowed_roots(Path::new(
            "/tmp/dragon_npu_la_symlink_clamp_probe.json"
        )));
        drop(_guard);
        let _ = std::fs::remove_file(&link);
    }
}
