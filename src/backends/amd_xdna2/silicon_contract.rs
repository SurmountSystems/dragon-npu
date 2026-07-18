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
    TimedOpResult, build_ert_start_npu_cmd_bo, build_ert_start_npu_cmd_bo_with_kernel_args,
};
use super::lab_io::{LabKernelArgLayout, pack_start_npu_kernel_args_with_ctrl_layout};
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
/// `ran_on_npu`. Encodes locked residual `0x3000003` / `APP_LOAD_PDI_FAIL` plus
/// START_NPU size dual-mode fields. Env layout/size trials are **CLOSED**.
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
        "app_load_residual code=0x3000003 name={} aiebu_elf={} rela_count={} \
         rela_mode={} layout={layout}; {size}; env_hitl_closed=1 \
         content_and_xrt3_closed=1 not_c_ready=1 not_ran_on_npu=1",
        aie2_fw_status_name(0x3_000_003),
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
/// optional top-level `rela_count`. **Not** a full-body hash and **not** new
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
    let status = status
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned);
    LastApplyEvidenceFingerprint {
        status,
        aiebu_elf,
        ert_error,
        ran_on_npu,
        detail_len: detail.map_or(0, str::len),
        rela_count,
    }
}

/// True when two coarse fingerprints match (same C3 evidence identity).
///
/// Equality of status / aiebu / ert / ran_on_npu / detail_len / rela_count only.
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
        || l.contains("app_load")
        || l.contains("failed cmd")
        || l.contains("status 0x3000003")
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

/// True when a research note/checklist string contains an exact whitespace token.
#[must_use]
pub fn residual_note_has_exact_token(note: &str, token: &str) -> bool {
    note.split_whitespace().any(|t| t == token)
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
}

/// Offline userspace-vs-FW checklist from last_apply signals (research only).
///
/// Unit-locks APP_LOAD **signal matrix** already present in detail
/// (`map_offset`, Dev BO, content/bo/start_npu sizes, `SubmitWaitNotOk`, OFM
/// zeros, missing journal hex). **Never** invents Pass / C-ready / `ran_on_npu`.
///
/// Print this string **as-is** (already starts with `app_load_checklist=1`).
#[must_use]
pub fn format_app_load_offline_checklist(
    class: SiliconResidualClass,
    sig: &LastApplyDetailSignals,
) -> String {
    let mut parts = vec![
        "app_load_checklist=1".to_string(),
        format!("residual={}", class.as_str()),
        "not_c_ready=1".to_string(),
        "not_ran_on_npu=1".to_string(),
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
    if sig.instr_bo_type.as_deref() == Some("Dev") {
        parts.push("instr_ty_dev=1".to_string());
    } else if let Some(ref t) = sig.instr_bo_type {
        parts.push(format!("instr_ty={t}"));
    } else {
        parts.push("instr_ty_unknown=1".to_string());
    }
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
    if let Some(ref mode) = sig.inst_size_mode {
        parts.push(format!("inst_size_mode={mode}"));
    }
    if let Some(ref mode) = sig.rela_mode {
        parts.push(format!("rela_mode={mode}"));
    }
    if let Some(ref lay) = sig.layout {
        parts.push(format!("layout={lay}"));
    }
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
    if class.is_locked_app_load_after_aiebu() {
        // Class label already in residual=; do not invent journal code narrative.
        parts.push("class_implies_app_load_after_aiebu=1".to_string());
        parts.push("env_hitl_closed=1".to_string());
    }
    parts.join(" ")
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
/// Includes APP status (`0x3000003`, `0x4000003`) and MSG_OP `0x106` (map_host).
#[must_use]
pub fn aie2_fw_status_name(code: u32) -> &'static str {
    match code {
        0x3_000_003 => "APP_LOAD_PDI_FAIL",
        0x4_000_003 => "AIE2_STATUS_INVALID_PARAM",
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

fn path_under_allowed_roots(path: &std::path::Path) -> bool {
    // Component-aware: refuse `/tmp_evil/...` when temp is `/tmp` (string prefix footgun).
    let temp = std::env::temp_dir();
    let lab = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/xdna2_cmd");
    // Prefer resolved form of roots when available (symlink temp_dir).
    let temp_root = std::fs::canonicalize(&temp).unwrap_or(temp);
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
    std::env::temp_dir().join(format!("dragon_npu_last_apply_{}.json", std::process::id()))
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
    if let Some(parent) = owned.parent() {
        if parent.is_dir()
            || parent.ends_with("xdna2_cmd")
            || parent.starts_with(std::env::temp_dir())
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
        let dir = std::env::temp_dir();
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
        }
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
        let temp = std::env::temp_dir();
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
        // Lexical: under temp_dir; resolved parent: outside → must refuse.
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
        let tmp = std::env::temp_dir().join("dragon_npu_la_test.json");
        assert!(last_apply_path_is_allowed(&tmp));
        assert!(!last_apply_path_is_allowed(Path::new("/etc/passwd")));
        assert!(!last_apply_path_is_allowed(Path::new("../../etc/passwd")));
        assert!(!last_apply_path_is_allowed(Path::new("/tmp/not_json.txt")));
        // String prefix footgun: /tmp_evil is NOT under /tmp as Path components.
        assert!(
            !path_under_allowed_roots(Path::new("/tmp_evil/last_apply.json")),
            "sibling prefix of temp_dir must be refused"
        );
        // When temp is /tmp, /tmp_evil must also fail last_apply_path_is_allowed.
        if std::env::temp_dir() == Path::new("/tmp") {
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
        let dir =
            std::env::temp_dir().join(format!("dragon_npu_last_apply_test_{}", std::process::id()));
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
}
